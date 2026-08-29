# Phase 1 — Foundation: Auth, Projects, Issues, Users

**Target:** `v0.1`

## Goal

Deliver the core user flow—signup, project creation, and issue creation/editing—backed by PostgreSQL in a Rust modular monolith.

## Data model

- `users` (`id`, `email`, `password_hash`, `name`, `created_at`)
- `organizations`, `organization_members` (roles: `admin`, `manager`, `member`)
- `projects`, `project_members` (roles: `maintainer`, `developer`, `reporter`)
- `issues` (`id`, `project_id`, `title`, `description`, `status`, `priority`, `assignee_id`, `reporter_id`, `created_at`, `updated_at`)
- `labels`, `issue_labels` (many-to-many)

## API surface

- `POST /auth/signup`, `POST /auth/login`, `POST /auth/refresh`
- `GET/POST /organizations`, `GET/POST /organizations/:id/members`
- `GET/POST /projects`, `GET/PATCH/DELETE /projects/:id`
- `GET/POST /projects/:id/issues`, `GET/PATCH/DELETE /issues/:id`
- `POST /issues/:id/labels`

## Build order

Build natively first. Use a local PostgreSQL installation or a single Postgres container, run the backend with `cargo run`, and run the frontend with `next dev`. Do not introduce Docker Compose until steps 1–6 work.

1. Start PostgreSQL and create the database. Add SQLx migrations for the Phase 1 tables, a connection pool, and `tracing` instrumentation. Keep `DATABASE_URL` in a local `.env`.
2. Create a Cargo workspace with `api`, `auth`, `users`, `projects`, and `issues` crates. Wire Axum and Tower logging, CORS, and error middleware.
3. Add Argon2 password hashing, JWT issuance and verification, and an authentication middleware/extractor.
4. Add an `authorize(user, resource, action)` function and use it in every handler for org- and project-level RBAC.
5. Implement validated CRUD handlers for organizations, projects, issues, and labels using `serde` and `validator`.
6. Build a minimal **Next.js App Router** UI: login/signup, project list, issue list/detail, and create/edit issue form. Use Server Components for initial data, TanStack Query for client-side server state, and Zustand for auth/session state.
7. Containerize after the native flow works. Add a multi-stage backend Dockerfile, plus a frontend Dockerfile with a hot-reload `next dev` target and a production target using Next.js `standalone` output.
8. Add `docker-compose.yml` for Postgres (named volume), backend, and frontend. Run migrations automatically on `docker compose up`, and include `.env.example` plus `docker-compose.override.yml` for local secrets and ports.

## Testing bar

- Unit: password-hash round trip; valid, expired, and tampered JWTs; every org/project role combination in `authorize()`.
- Integration with real PostgreSQL: signup → login → authenticated happy path; unauthorized handler access; project-role-aware issue CRUD.
- After containerization, a fresh clone followed by `docker compose up` starts the complete working stack without manual steps.

## Definition of done

A user can register, create an organization and project, invite a teammate, and both users can create, edit, and close issues according to their roles. The final stack runs with a single `docker compose up`.
