use auth::{hash_password, issue_typed_token, verify_password, verify_token, TokenKind};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, net::SocketAddr, sync::Arc};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use validator::Validate;

#[derive(Clone)]
struct AppState {
    db: PgPool,
    jwt_secret: Arc<str>,
}
#[derive(Debug)]
struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({"error":self.1}))).into_response()
    }
}
type ApiResult<T> = Result<Json<T>, ApiError>;
fn bad(v: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, v.into())
}
fn forbidden() -> ApiError {
    ApiError(StatusCode::FORBIDDEN, "forbidden".into())
}
fn missing(v: &str) -> ApiError {
    ApiError(StatusCode::NOT_FOUND, format!("{v} not found"))
}
fn internal(e: impl std::fmt::Display) -> ApiError {
    tracing::error!(error=%e,"request failed");
    ApiError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal server error".into(),
    )
}
async fn validate<T: Validate>(v: &T) -> Result<(), ApiError> {
    v.validate().map_err(|e| bad(e.to_string()))
}
fn valid_color(v: &str) -> bool {
    v.len() == 7 && v.starts_with('#') && v.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}

#[derive(Deserialize, Validate)]
struct Credentials {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8, max = 128))]
    password: String,
}
#[derive(Deserialize, Validate)]
struct Signup {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8, max = 128))]
    password: String,
    #[validate(length(min = 1, max = 100))]
    name: String,
}
#[derive(Deserialize)]
struct RefreshInput {
    refresh_token: String,
}
#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: &'static str,
    expires_in: u32,
}
#[derive(Deserialize, Validate)]
struct OrganizationInput {
    #[validate(length(min = 1, max = 100))]
    name: String,
    #[validate(length(min = 2, max = 80))]
    slug: String,
}
#[derive(Deserialize, Validate)]
struct MemberInput {
    user_id: Option<Uuid>,
    #[validate(email)]
    email: Option<String>,
    role: String,
}
#[derive(Deserialize, Validate)]
struct ProjectInput {
    organization_id: Uuid,
    #[validate(length(min = 1, max = 100))]
    name: String,
    #[validate(length(min = 2, max = 20))]
    key: String,
    #[validate(length(max = 5000))]
    description: Option<String>,
}
#[derive(Deserialize, Validate)]
struct ProjectPatch {
    #[validate(length(min = 1, max = 100))]
    name: Option<String>,
    #[validate(length(max = 5000))]
    description: Option<String>,
}
#[derive(Deserialize, Validate)]
struct IssueInput {
    #[validate(length(min = 1, max = 250))]
    title: String,
    #[validate(length(max = 20000))]
    description: String,
    status: Option<String>,
    priority: Option<String>,
    assignee_id: Option<Uuid>,
}
#[derive(Deserialize, Validate)]
struct IssuePatch {
    #[validate(length(min = 1, max = 250))]
    title: Option<String>,
    #[validate(length(max = 20000))]
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    assignee_id: Option<Uuid>,
    clear_assignee: Option<bool>,
}
#[derive(Deserialize, Validate)]
struct LabelInput {
    #[validate(length(min = 1, max = 50))]
    name: String,
    color: String,
}
#[derive(Deserialize)]
struct LabelAssignment {
    label_id: Uuid,
}

#[derive(Serialize, sqlx::FromRow)]
struct Organization {
    id: Uuid,
    name: String,
    slug: String,
}
#[derive(Serialize, sqlx::FromRow)]
struct Project {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    key: String,
    description: Option<String>,
}
#[derive(Serialize, sqlx::FromRow)]
struct Issue {
    id: Uuid,
    project_id: Uuid,
    title: String,
    description: String,
    status: String,
    priority: String,
    assignee_id: Option<Uuid>,
    reporter_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Serialize, sqlx::FromRow)]
struct Label {
    id: Uuid,
    project_id: Uuid,
    name: String,
    color: String,
}
#[derive(Serialize, sqlx::FromRow)]
struct Membership {
    user_id: Uuid,
    name: String,
    email: String,
    role: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&env::var("DATABASE_URL").expect("DATABASE_URL is required"))
        .await?;
    if env::args().any(|a| a == "migrate") {
        sqlx::migrate!("../../migrations").run(&db).await?;
        return Ok(());
    }
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET is required");
    let app = routes(AppState {
        db,
        jwt_secret: secret.into(),
    });
    let address: SocketAddr = env::var("API_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3001".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
fn routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { StatusCode::NO_CONTENT }))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route(
            "/organizations",
            get(list_organizations).post(create_organization),
        )
        .route(
            "/organizations/:id/members",
            get(list_org_members).post(add_org_member),
        )
        .route("/projects", get(list_projects).post(create_project))
        .route(
            "/projects/:id",
            get(get_project)
                .patch(update_project)
                .delete(delete_project),
        )
        .route(
            "/projects/:id/members",
            get(list_project_members).post(add_project_member),
        )
        .route("/projects/:id/issues", get(list_issues).post(create_issue))
        .route("/projects/:id/labels", get(list_labels).post(create_label))
        .route(
            "/issues/:id",
            get(get_issue).patch(update_issue).delete(delete_issue),
        )
        .route(
            "/issues/:id/labels",
            get(list_issue_labels).post(assign_label),
        )
        .route("/issues/:issue/labels/:label", delete(unassign_label))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
        )
}

fn pair(user: Uuid, secret: &str) -> Result<TokenResponse, ApiError> {
    Ok(TokenResponse {
        access_token: issue_typed_token(user, secret, 1, TokenKind::Access).map_err(internal)?,
        refresh_token: issue_typed_token(user, secret, 720, TokenKind::Refresh)
            .map_err(internal)?,
        token_type: "Bearer",
        expires_in: 3600,
    })
}
async fn current_user(s: &AppState, h: &HeaderMap) -> Result<Uuid, ApiError> {
    let token = h
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError(
            StatusCode::UNAUTHORIZED,
            "missing bearer token".into(),
        ))?;
    let c = verify_token(token, &s.jwt_secret)
        .map_err(|_| ApiError(StatusCode::UNAUTHORIZED, "invalid token".into()))?;
    if c.kind != TokenKind::Access {
        return Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "access token required".into(),
        ));
    }
    Ok(c.sub)
}
async fn signup(State(s): State<AppState>, Json(v): Json<Signup>) -> ApiResult<TokenResponse> {
    validate(&v).await?;
    let hash = hash_password(&v.password).map_err(internal)?;
    let id = sqlx::query_scalar(
        "INSERT INTO users(email,password_hash,name) VALUES($1,$2,$3) RETURNING id",
    )
    .bind(v.email.to_lowercase())
    .bind(hash)
    .bind(v.name)
    .fetch_one(&s.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            bad("email already registered")
        } else {
            internal(e)
        }
    })?;
    Ok(Json(pair(id, &s.jwt_secret)?))
}
async fn login(State(s): State<AppState>, Json(v): Json<Credentials>) -> ApiResult<TokenResponse> {
    validate(&v).await?;
    let u: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id,password_hash FROM users WHERE email=$1")
            .bind(v.email.to_lowercase())
            .fetch_optional(&s.db)
            .await
            .map_err(internal)?;
    let (id, hash) = u.ok_or(ApiError(
        StatusCode::UNAUTHORIZED,
        "invalid credentials".into(),
    ))?;
    verify_password(&v.password, &hash)
        .map_err(|_| ApiError(StatusCode::UNAUTHORIZED, "invalid credentials".into()))?;
    Ok(Json(pair(id, &s.jwt_secret)?))
}
async fn refresh(
    State(s): State<AppState>,
    Json(v): Json<RefreshInput>,
) -> ApiResult<TokenResponse> {
    let c = verify_token(&v.refresh_token, &s.jwt_secret)
        .map_err(|_| ApiError(StatusCode::UNAUTHORIZED, "invalid refresh token".into()))?;
    if c.kind != TokenKind::Refresh {
        return Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "refresh token required".into(),
        ));
    }
    Ok(Json(pair(c.sub, &s.jwt_secret)?))
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Read,
    Write,
    Manage,
}
fn role_allows(r: Option<&str>, a: Action) -> bool {
    matches!(
        (r, a),
        (Some("maintainer"), _)
            | (Some("developer"), Action::Read | Action::Write)
            | (Some("reporter"), Action::Read | Action::Write)
    )
}
async fn org_role(db: &PgPool, o: Uuid, u: Uuid) -> Result<Option<String>, ApiError> {
    sqlx::query_scalar(
        "SELECT role::text FROM organization_members WHERE organization_id=$1 AND user_id=$2",
    )
    .bind(o)
    .bind(u)
    .fetch_optional(db)
    .await
    .map_err(internal)
}
async fn authorize(s: &AppState, u: Uuid, p: Uuid, a: Action) -> Result<(), ApiError> {
    let admin:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects p JOIN organization_members om ON om.organization_id=p.organization_id WHERE p.id=$1 AND om.user_id=$2 AND om.role='admin')").bind(p).bind(u).fetch_one(&s.db).await.map_err(internal)?;
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role::text FROM project_members WHERE project_id=$1 AND user_id=$2",
    )
    .bind(p)
    .bind(u)
    .fetch_optional(&s.db)
    .await
    .map_err(internal)?;
    if admin || role_allows(role.as_deref(), a) {
        Ok(())
    } else {
        Err(forbidden())
    }
}
async fn resolve_member(db: &PgPool, v: &MemberInput) -> Result<Uuid, ApiError> {
    if let Some(id) = v.user_id {
        return Ok(id);
    }
    let email = v
        .email
        .as_ref()
        .ok_or_else(|| bad("email or user_id is required"))?;
    sqlx::query_scalar("SELECT id FROM users WHERE email=$1")
        .bind(email.to_lowercase())
        .fetch_optional(db)
        .await
        .map_err(internal)?
        .ok_or_else(|| missing("user"))
}
async fn ensure_assignable(db: &PgPool, p: Uuid, u: Option<Uuid>) -> Result<(), ApiError> {
    let Some(u) = u else { return Ok(()) };
    let ok:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects p LEFT JOIN project_members pm ON pm.project_id=p.id AND pm.user_id=$2 LEFT JOIN organization_members om ON om.organization_id=p.organization_id AND om.user_id=$2 WHERE p.id=$1 AND(pm.user_id IS NOT NULL OR om.user_id IS NOT NULL))").bind(p).bind(u).fetch_one(db).await.map_err(internal)?;
    if ok {
        Ok(())
    } else {
        Err(bad("assignee must belong to the project or organization"))
    }
}

async fn list_organizations(
    State(s): State<AppState>,
    h: HeaderMap,
) -> ApiResult<Vec<Organization>> {
    let u = current_user(&s, &h).await?;
    Ok(Json(sqlx::query_as("SELECT o.id,o.name,o.slug FROM organizations o JOIN organization_members m ON m.organization_id=o.id WHERE m.user_id=$1 ORDER BY o.name").bind(u).fetch_all(&s.db).await.map_err(internal)?))
}
async fn create_organization(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(v): Json<OrganizationInput>,
) -> ApiResult<Organization> {
    let u = current_user(&s, &h).await?;
    validate(&v).await?;
    let mut tx = s.db.begin().await.map_err(internal)?;
    let o: Organization =
        sqlx::query_as("INSERT INTO organizations(name,slug)VALUES($1,$2)RETURNING id,name,slug")
            .bind(v.name)
            .bind(v.slug.to_lowercase())
            .fetch_one(&mut *tx)
            .await
            .map_err(internal)?;
    sqlx::query(
        "INSERT INTO organization_members(organization_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(o.id)
    .bind(u)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(Json(o))
}
async fn list_org_members(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<Membership>> {
    let u = current_user(&s, &h).await?;
    if org_role(&s.db, id, u).await?.is_none() {
        return Err(forbidden());
    }
    Ok(Json(sqlx::query_as("SELECT u.id user_id,u.name,u.email,m.role::text role FROM organization_members m JOIN users u ON u.id=m.user_id WHERE m.organization_id=$1 ORDER BY u.name").bind(id).fetch_all(&s.db).await.map_err(internal)?))
}
async fn add_org_member(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
    Json(v): Json<MemberInput>,
) -> ApiResult<Membership> {
    let u = current_user(&s, &h).await?;
    validate(&v).await?;
    if !matches!(
        org_role(&s.db, id, u).await?.as_deref(),
        Some("admin") | Some("manager")
    ) {
        return Err(forbidden());
    }
    if !matches!(v.role.as_str(), "admin" | "manager" | "member") {
        return Err(bad("invalid organization role"));
    }
    let m = resolve_member(&s.db, &v).await?;
    Ok(Json(sqlx::query_as("WITH m AS(INSERT INTO organization_members(organization_id,user_id,role)VALUES($1,$2,$3::organization_role)ON CONFLICT(organization_id,user_id)DO UPDATE SET role=EXCLUDED.role RETURNING user_id,role)SELECT u.id user_id,u.name,u.email,m.role::text role FROM m JOIN users u ON u.id=m.user_id").bind(id).bind(m).bind(v.role).fetch_one(&s.db).await.map_err(internal)?))
}

async fn list_projects(State(s): State<AppState>, h: HeaderMap) -> ApiResult<Vec<Project>> {
    let u = current_user(&s, &h).await?;
    Ok(Json(sqlx::query_as("SELECT DISTINCT p.id,p.organization_id,p.name,p.key,p.description FROM projects p LEFT JOIN project_members pm ON pm.project_id=p.id LEFT JOIN organization_members om ON om.organization_id=p.organization_id WHERE pm.user_id=$1 OR om.user_id=$1 ORDER BY p.name").bind(u).fetch_all(&s.db).await.map_err(internal)?))
}
async fn create_project(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(v): Json<ProjectInput>,
) -> ApiResult<Project> {
    let u = current_user(&s, &h).await?;
    validate(&v).await?;
    if !matches!(
        org_role(&s.db, v.organization_id, u).await?.as_deref(),
        Some("admin") | Some("manager")
    ) {
        return Err(forbidden());
    }
    let mut tx = s.db.begin().await.map_err(internal)?;
    let p:Project=sqlx::query_as("INSERT INTO projects(organization_id,name,key,description)VALUES($1,$2,$3,$4)RETURNING id,organization_id,name,key,description").bind(v.organization_id).bind(v.name).bind(v.key.to_uppercase()).bind(v.description).fetch_one(&mut*tx).await.map_err(internal)?;
    sqlx::query("INSERT INTO project_members(project_id,user_id,role)VALUES($1,$2,'maintainer')")
        .bind(p.id)
        .bind(u)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(Json(p))
}
async fn get_project(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Project> {
    let u = current_user(&s, &h).await?;
    authorize(&s, u, id, Action::Read).await?;
    Ok(Json(
        sqlx::query_as("SELECT id,organization_id,name,key,description FROM projects WHERE id=$1")
            .bind(id)
            .fetch_optional(&s.db)
            .await
            .map_err(internal)?
            .ok_or_else(|| missing("project"))?,
    ))
}
async fn update_project(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
    Json(v): Json<ProjectPatch>,
) -> ApiResult<Project> {
    let u = current_user(&s, &h).await?;
    validate(&v).await?;
    authorize(&s, u, id, Action::Manage).await?;
    Ok(Json(sqlx::query_as("UPDATE projects SET name=COALESCE($2,name),description=COALESCE($3,description),updated_at=now()WHERE id=$1 RETURNING id,organization_id,name,key,description").bind(id).bind(v.name).bind(v.description).fetch_optional(&s.db).await.map_err(internal)?.ok_or_else(||missing("project"))?))
}
async fn delete_project(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let u = current_user(&s, &h).await?;
    authorize(&s, u, id, Action::Manage).await?;
    if sqlx::query("DELETE FROM projects WHERE id=$1")
        .bind(id)
        .execute(&s.db)
        .await
        .map_err(internal)?
        .rows_affected()
        == 0
    {
        Err(missing("project"))
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}
async fn list_project_members(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Vec<Membership>> {
    let u = current_user(&s, &h).await?;
    authorize(&s, u, id, Action::Read).await?;
    Ok(Json(sqlx::query_as("SELECT u.id user_id,u.name,u.email,pm.role::text role FROM project_members pm JOIN users u ON u.id=pm.user_id WHERE pm.project_id=$1 ORDER BY u.name").bind(id).fetch_all(&s.db).await.map_err(internal)?))
}
async fn add_project_member(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
    Json(v): Json<MemberInput>,
) -> ApiResult<Membership> {
    let u = current_user(&s, &h).await?;
    validate(&v).await?;
    authorize(&s, u, id, Action::Manage).await?;
    if !matches!(v.role.as_str(), "maintainer" | "developer" | "reporter") {
        return Err(bad("invalid project role"));
    }
    let m = resolve_member(&s.db, &v).await?;
    let in_org:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects p JOIN organization_members om ON om.organization_id=p.organization_id WHERE p.id=$1 AND om.user_id=$2)").bind(id).bind(m).fetch_one(&s.db).await.map_err(internal)?;
    if !in_org {
        return Err(bad("add the user to the organization first"));
    }
    Ok(Json(sqlx::query_as("WITH m AS(INSERT INTO project_members(project_id,user_id,role)VALUES($1,$2,$3::project_role)ON CONFLICT(project_id,user_id)DO UPDATE SET role=EXCLUDED.role RETURNING user_id,role)SELECT u.id user_id,u.name,u.email,m.role::text role FROM m JOIN users u ON u.id=m.user_id").bind(id).bind(m).bind(v.role).fetch_one(&s.db).await.map_err(internal)?))
}

fn valid_issue(s: Option<&str>, p: Option<&str>) -> bool {
    s.is_none_or(|v| matches!(v, "open" | "in_progress" | "closed"))
        && p.is_none_or(|v| matches!(v, "low" | "medium" | "high" | "urgent"))
}
async fn list_issues(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(p): Path<Uuid>,
) -> ApiResult<Vec<Issue>> {
    let u = current_user(&s, &h).await?;
    authorize(&s, u, p, Action::Read).await?;
    Ok(Json(sqlx::query_as("SELECT id,project_id,title,description,status::text status,priority::text priority,assignee_id,reporter_id,created_at,updated_at FROM issues WHERE project_id=$1 ORDER BY updated_at DESC").bind(p).fetch_all(&s.db).await.map_err(internal)?))
}
async fn create_issue(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(p): Path<Uuid>,
    Json(v): Json<IssueInput>,
) -> ApiResult<Issue> {
    let u = current_user(&s, &h).await?;
    validate(&v).await?;
    authorize(&s, u, p, Action::Write).await?;
    if !valid_issue(v.status.as_deref(), v.priority.as_deref()) {
        return Err(bad("invalid issue status or priority"));
    }
    ensure_assignable(&s.db, p, v.assignee_id).await?;
    Ok(Json(sqlx::query_as("INSERT INTO issues(project_id,title,description,status,priority,assignee_id,reporter_id)VALUES($1,$2,$3,$4::issue_status,$5::issue_priority,$6,$7)RETURNING id,project_id,title,description,status::text status,priority::text priority,assignee_id,reporter_id,created_at,updated_at").bind(p).bind(v.title).bind(v.description).bind(v.status.unwrap_or_else(||"open".into())).bind(v.priority.unwrap_or_else(||"medium".into())).bind(v.assignee_id).bind(u).fetch_one(&s.db).await.map_err(internal)?))
}
async fn get_issue(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Issue> {
    let u = current_user(&s, &h).await?;
    let i:Issue=sqlx::query_as("SELECT id,project_id,title,description,status::text status,priority::text priority,assignee_id,reporter_id,created_at,updated_at FROM issues WHERE id=$1").bind(id).fetch_optional(&s.db).await.map_err(internal)?.ok_or_else(||missing("issue"))?;
    authorize(&s, u, i.project_id, Action::Read).await?;
    Ok(Json(i))
}
async fn update_issue(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
    Json(v): Json<IssuePatch>,
) -> ApiResult<Issue> {
    let u = current_user(&s, &h).await?;
    validate(&v).await?;
    let p: Uuid = sqlx::query_scalar("SELECT project_id FROM issues WHERE id=$1")
        .bind(id)
        .fetch_optional(&s.db)
        .await
        .map_err(internal)?
        .ok_or_else(|| missing("issue"))?;
    authorize(&s, u, p, Action::Write).await?;
    if !valid_issue(v.status.as_deref(), v.priority.as_deref()) {
        return Err(bad("invalid issue status or priority"));
    }
    ensure_assignable(&s.db, p, v.assignee_id).await?;
    Ok(Json(sqlx::query_as("UPDATE issues SET title=COALESCE($2,title),description=COALESCE($3,description),status=COALESCE($4::issue_status,status),priority=COALESCE($5::issue_priority,priority),assignee_id=CASE WHEN $7 THEN NULL ELSE COALESCE($6,assignee_id)END,updated_at=now()WHERE id=$1 RETURNING id,project_id,title,description,status::text status,priority::text priority,assignee_id,reporter_id,created_at,updated_at").bind(id).bind(v.title).bind(v.description).bind(v.status).bind(v.priority).bind(v.assignee_id).bind(v.clear_assignee.unwrap_or(false)).fetch_one(&s.db).await.map_err(internal)?))
}
async fn delete_issue(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let u = current_user(&s, &h).await?;
    let p: Uuid = sqlx::query_scalar("SELECT project_id FROM issues WHERE id=$1")
        .bind(id)
        .fetch_optional(&s.db)
        .await
        .map_err(internal)?
        .ok_or_else(|| missing("issue"))?;
    authorize(&s, u, p, Action::Write).await?;
    sqlx::query("DELETE FROM issues WHERE id=$1")
        .bind(id)
        .execute(&s.db)
        .await
        .map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn list_labels(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(p): Path<Uuid>,
) -> ApiResult<Vec<Label>> {
    let u = current_user(&s, &h).await?;
    authorize(&s, u, p, Action::Read).await?;
    Ok(Json(
        sqlx::query_as(
            "SELECT id,project_id,name,color FROM labels WHERE project_id=$1 ORDER BY name",
        )
        .bind(p)
        .fetch_all(&s.db)
        .await
        .map_err(internal)?,
    ))
}
async fn create_label(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(p): Path<Uuid>,
    Json(v): Json<LabelInput>,
) -> ApiResult<Label> {
    let u = current_user(&s, &h).await?;
    validate(&v).await?;
    if !valid_color(&v.color) {
        return Err(bad("color must be a #RRGGBB value"));
    }
    authorize(&s, u, p, Action::Write).await?;
    Ok(Json(sqlx::query_as("INSERT INTO labels(project_id,name,color)VALUES($1,$2,$3)RETURNING id,project_id,name,color").bind(p).bind(v.name).bind(v.color).fetch_one(&s.db).await.map_err(internal)?))
}
async fn list_issue_labels(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(i): Path<Uuid>,
) -> ApiResult<Vec<Label>> {
    let u = current_user(&s, &h).await?;
    let p: Uuid = sqlx::query_scalar("SELECT project_id FROM issues WHERE id=$1")
        .bind(i)
        .fetch_optional(&s.db)
        .await
        .map_err(internal)?
        .ok_or_else(|| missing("issue"))?;
    authorize(&s, u, p, Action::Read).await?;
    Ok(Json(sqlx::query_as("SELECT l.id,l.project_id,l.name,l.color FROM labels l JOIN issue_labels il ON il.label_id=l.id WHERE il.issue_id=$1 ORDER BY l.name").bind(i).fetch_all(&s.db).await.map_err(internal)?))
}
async fn assign_label(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(i): Path<Uuid>,
    Json(v): Json<LabelAssignment>,
) -> Result<StatusCode, ApiError> {
    let u = current_user(&s, &h).await?;
    let p: Uuid = sqlx::query_scalar("SELECT project_id FROM issues WHERE id=$1")
        .bind(i)
        .fetch_optional(&s.db)
        .await
        .map_err(internal)?
        .ok_or_else(|| missing("issue"))?;
    authorize(&s, u, p, Action::Write).await?;
    let ok: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM labels WHERE id=$1 AND project_id=$2)")
            .bind(v.label_id)
            .bind(p)
            .fetch_one(&s.db)
            .await
            .map_err(internal)?;
    if !ok {
        return Err(bad("label is not part of this project"));
    }
    sqlx::query("INSERT INTO issue_labels VALUES($1,$2)ON CONFLICT DO NOTHING")
        .bind(i)
        .bind(v.label_id)
        .execute(&s.db)
        .await
        .map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn unassign_label(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((i, l)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let u = current_user(&s, &h).await?;
    let p: Uuid = sqlx::query_scalar("SELECT project_id FROM issues WHERE id=$1")
        .bind(i)
        .fetch_optional(&s.db)
        .await
        .map_err(internal)?
        .ok_or_else(|| missing("issue"))?;
    authorize(&s, u, p, Action::Write).await?;
    sqlx::query("DELETE FROM issue_labels WHERE issue_id=$1 AND label_id=$2")
        .bind(i)
        .bind(l)
        .execute(&s.db)
        .await
        .map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn role_matrix() {
        for (a, m, d, r) in [
            (Action::Read, true, true, true),
            (Action::Write, true, true, true),
            (Action::Manage, true, false, false),
        ] {
            assert_eq!(role_allows(Some("maintainer"), a), m);
            assert_eq!(role_allows(Some("developer"), a), d);
            assert_eq!(role_allows(Some("reporter"), a), r);
            assert!(!role_allows(None, a))
        }
    }
    #[test]
    fn validates_values() {
        assert!(valid_issue(Some("closed"), Some("urgent")));
        assert!(!valid_issue(Some("done"), None));
        assert!(valid_color("#12abEF"));
        assert!(!valid_color("blue"));
    }
}
