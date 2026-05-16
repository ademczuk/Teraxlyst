// Error type for the db layer. Serialized as its Display string so Tauri
// commands can return it via .map_err(|e| e.to_string()) without losing info.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    // Reserved for path/file errors when the DB takes on filesystem
    // probing (planning/ROADMAP.md M1.1). Stable surface, no current path
    // hits it under v1 sqlite-only writes.
    #[allow(dead_code)]
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("actor channel closed")]
    ActorClosed,
}

impl Serialize for DbError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
