// Error type for the MCP host layer.

use serde::{Serialize, Serializer};

use crate::db::error::DbError;

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("db: {0}")]
    Db(#[from] DbError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    // Reserved for tool argument parsing (M3.2 rmcp wire-format wiring);
    // current paths use typed handlers so serde_json failures are caught
    // upstream.
    #[allow(dead_code)]
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("emit failed: {0}")]
    Emit(String),
    #[error("pending channel closed")]
    PendingClosed,
    #[error("path outside workspace: {0}")]
    PathEscape(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
}

impl Serialize for McpError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
