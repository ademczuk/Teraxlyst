// SessionManager error type. Serialized as its Display string at the IPC
// boundary, mirroring DbError so Tauri commands can return it directly.

use serde::{Serialize, Serializer};

use crate::db::error::DbError;

#[derive(Debug, thiserror::Error)]
pub enum ManagerError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("db: {0}")]
    Db(#[from] DbError),
    #[error("session not found: {0}")]
    NotFound(i64),
    #[error("session already running: {0}")]
    AlreadyRunning(i64),
    #[error("failed to spawn provider: {0}")]
    SpawnFailed(String),
    // Reserved for v2.1 channel-drop diagnostics. The variant exists so the
    // public error surface is stable when we wire it up.
    #[allow(dead_code)]
    #[error("channel: {0}")]
    Channel(String),
}

impl Serialize for ManagerError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
