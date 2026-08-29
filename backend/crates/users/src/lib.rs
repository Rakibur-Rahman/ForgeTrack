use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct User { pub id: Uuid, pub email: String, pub name: String }
