// Data model structs mirroring the schema in db/schema.sql.
// IDs are i64 (matches SQLite INTEGER). Timestamps are millis-since-epoch i64.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub workspace_id: i64,
    pub parent_id: Option<i64>,
    pub provider: String,
    pub title: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub id: i64,
    pub session_id: i64,
    pub seq: i64,
    pub kind: String,
    pub payload: serde_json::Value,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tracker {
    pub id: i64,
    pub workspace_id: i64,
    pub name: String,
    pub yaml_source: String,
    pub schema_json: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerItem {
    pub id: i64,
    pub tracker_id: i64,
    pub public_id: String,
    pub status: Option<String>,
    pub fields_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffProposal {
    pub id: i64,
    pub session_id: i64,
    pub file_path: String,
    pub base_hash: Option<String>,
    pub patch: String,
    pub status: String,
    pub created_at: i64,
    pub resolved_at: Option<i64>,
    // M5: the final file content the agent proposes. Populated by M3's
    // propose_diff tool. Legacy rows (pre-v2 migration) carry None and
    // cannot be applied via diff_apply_and_resolve.
    #[serde(default)]
    pub new_content: Option<String>,
}
