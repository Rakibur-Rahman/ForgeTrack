use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRole { Maintainer, Developer, Reporter }

#[derive(Debug, Clone, Serialize)]
pub struct Project { pub id: Uuid, pub organization_id: Uuid, pub name: String, pub key: String, pub description: Option<String> }
