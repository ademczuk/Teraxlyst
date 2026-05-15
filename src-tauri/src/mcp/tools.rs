// The three M3 tool implementations.
//
// Architecture choice: the load-bearing tool LOGIC lives in plain async
// methods on `TeraxlystToolset` (do_prompt_for_user_input, do_propose_diff,
// do_read_workspace_file). The rmcp-macro-decorated wrappers that expose
// the tools over the MCP wire protocol live in `tools_rmcp.rs` behind the
// `mcp-stdio` cargo feature so a churn in the rmcp macro surface (which we
// can't verify without `cargo check` access) doesn't break the M3 core.
//
// Why this split:
//   - The renderer drives these tools via the Tauri commands today; the
//     wire transport is not on the critical path for the M3 exit criterion.
//   - rmcp's `#[tool_router]` and `#[tool]` macros are stable in the 1.x
//     line but their exact attribute syntax shifts between minor versions.
//     Isolating macro usage means a future bump won't ripple into business
//     logic or tests.
//   - Tests exercise the do_* methods directly, which is what we actually
//     want to cover.
//
// Tool surface (logical, also matches the wire surface when enabled):
//   1. prompt_for_user_input(prompt_text, field) -> JSON value
//   2. propose_diff(session_id, file_path, new_content) -> {status}
//   3. read_workspace_file(workspace_path, relative_path) -> string
//
// TODO (M4 wiring): four tracker tools are implemented in
// crate::trackers::mcp_wrappers and exposed as Tauri commands. To expose
// them via MCP, extend TeraxlystToolset with a `db: DbHandle` field
// (constructed in `server::spawn_in_process` from the same handle
// installed in Tauri state), then add do_tracker_* methods + adapter
// wrappers in tools_rmcp.rs.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::diff_pipeline::PendingDiffs;
use super::error::McpError;
use super::prompt_pipeline::PendingPrompts;
use super::types::{DiffStatus, PromptField};

// Tool input shapes. These are serde-only and re-used by the rmcp adapter
// when that feature is on.

#[derive(Debug, Deserialize, Serialize)]
pub struct PromptForUserInputArgs {
    pub prompt_text: String,
    pub field: PromptField,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProposeDiffArgs {
    pub session_id: i64,
    pub file_path: String,
    pub new_content: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReadWorkspaceFileArgs {
    pub workspace_path: String,
    pub relative_path: String,
}

#[derive(Debug, Serialize)]
pub struct ProposeDiffResult {
    pub proposal_id: i64,
    pub status: DiffStatus,
}

// The toolset struct: held by Tauri state so the prompt/diff resolve
// commands can dispatch, and held by the rmcp adapter (when the feature is
// on) for wire-protocol exposure.
#[derive(Clone)]
pub struct TeraxlystToolset {
    pub prompts: PendingPrompts,
    pub diffs: PendingDiffs,
}

impl TeraxlystToolset {
    pub fn new(prompts: PendingPrompts, diffs: PendingDiffs) -> Self {
        Self { prompts, diffs }
    }

    // -------------------------------------------------------------------
    // Tool implementations (used by both the rmcp adapter and tests).
    // -------------------------------------------------------------------

    pub async fn do_prompt_for_user_input(
        &self,
        prompt_text: String,
        field: PromptField,
    ) -> Result<Value, McpError> {
        let rx = self.prompts.fire(prompt_text, field).await?;
        rx.await.map_err(|_| McpError::PendingClosed)
    }

    pub async fn do_propose_diff(
        &self,
        session_id: i64,
        file_path: String,
        new_content: String,
    ) -> Result<ProposeDiffResult, McpError> {
        let path = PathBuf::from(file_path);
        let (proposal, rx) = self.diffs.fire(session_id, path, new_content).await?;
        let resolution = rx.await.map_err(|_| McpError::PendingClosed)?;
        Ok(ProposeDiffResult {
            proposal_id: proposal.id,
            status: resolution.status,
        })
    }

    pub async fn do_read_workspace_file(
        &self,
        workspace_path: &str,
        relative_path: &str,
    ) -> Result<String, McpError> {
        // Belt-and-suspenders sandbox check:
        //   1. Lexical pre-check rejects obvious `..` and absolute paths.
        //      Produces a clear error even when the file doesn't exist yet.
        //   2. Canonicalize both workspace and target, then verify the
        //      target starts_with the workspace. This catches symlinks
        //      that escape.
        reject_parent_traversal(relative_path)?;

        let workspace = PathBuf::from(workspace_path);
        let workspace_canon = tokio::fs::canonicalize(&workspace).await.map_err(|e| {
            McpError::Invalid(format!(
                "workspace path not accessible: {} ({})",
                workspace.display(),
                e
            ))
        })?;
        let joined = workspace_canon.join(relative_path);

        let target_canon = tokio::fs::canonicalize(&joined).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                McpError::NotFound(format!("{}", joined.display()))
            } else {
                McpError::Io(e)
            }
        })?;

        if !target_canon.starts_with(&workspace_canon) {
            return Err(McpError::PathEscape(target_canon.display().to_string()));
        }

        // Cap the read at 10 MiB to match the existing fs::file convention.
        let bytes = tokio::fs::read(&target_canon).await?;
        if bytes.len() > 10 * 1024 * 1024 {
            return Err(McpError::Invalid(format!(
                "file too large: {} bytes",
                bytes.len()
            )));
        }
        String::from_utf8(bytes).map_err(|_| McpError::Invalid("file is not UTF-8".into()))
    }
}

fn reject_parent_traversal(rel: &str) -> Result<(), McpError> {
    let p: &Path = Path::new(rel);
    for c in p.components() {
        match c {
            Component::ParentDir => {
                return Err(McpError::PathEscape(rel.to_string()));
            }
            Component::Prefix(_) | Component::RootDir => {
                // Absolute paths are not allowed in the "relative" arg.
                return Err(McpError::PathEscape(rel.to_string()));
            }
            _ => {}
        }
    }
    Ok(())
}

// Re-export Arc<TeraxlystToolset> alias so server.rs has a tidy name.
pub type SharedToolset = Arc<TeraxlystToolset>;
