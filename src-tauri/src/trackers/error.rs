// TrackerError: errors from YAML parsing, validation, ID generation, and
// DB synchronization. Mirrors DbError serialization: the IPC boundary
// turns it into a Display string so Tauri commands can return it without
// losing context. Serialize impl is hand-rolled because anyhow-style
// thiserror enums don't get serde::Serialize automatically.

use serde::{Serialize, Serializer};

use crate::db::error::DbError;

#[derive(Debug, thiserror::Error)]
pub enum TrackerError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("db: {0}")]
    Db(#[from] DbError),

    #[error("schema: {0}")]
    Schema(String),

    #[error("validation: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid id format: {0}")]
    InvalidIdFormat(String),
}

impl Serialize for TrackerError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
