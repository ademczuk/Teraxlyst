// Correlation-ID pending-diff machinery for propose_diff.
//
// Flow:
//   1. tool fires:
//        - compute unified diff via `similar`
//        - write a diff_proposals row (status='pending') via DbHandle
//        - emit `teraxlyst:mcp-diff` event with {id, proposal_id, ...}
//        - register oneshot in `pending`, await
//   2. renderer M5 UI clicks Approve/Reject and invokes mcp_diff_resolve
//      with {id, action}. The command:
//        - calls DbHandle::resolve_diff_proposal (writes status row)
//        - on approve: writes new_content to disk
//        - looks up oneshot by id, sends DiffResolution
//   3. tool returns the resolution to the agent.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use similar::TextDiff;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::{oneshot, Mutex};
use ulid::Ulid;

use crate::db::actor::DbHandle;
use crate::db::types::DiffProposal;

use super::error::McpError;
use super::types::{DiffRequest, DiffResolution, DiffStatus, EVENT_DIFF};

// Compute a unified diff in the standard "unified-3" format.
pub fn compute_unified_diff(old: &str, new: &str, file_path: &str) -> String {
    TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(3)
        .header(&format!("a/{}", file_path), &format!("b/{}", file_path))
        .to_string()
}

pub trait DiffEmitter: Send + Sync + 'static {
    fn emit_diff(&self, payload: &DiffRequest) -> Result<(), McpError>;
}

pub struct TauriDiffEmitter<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriDiffEmitter<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> DiffEmitter for TauriDiffEmitter<R> {
    fn emit_diff(&self, payload: &DiffRequest) -> Result<(), McpError> {
        self.app
            .emit(EVENT_DIFF, payload)
            .map_err(|e| McpError::Emit(e.to_string()))
    }
}

// Per-pending-diff state. We also stash the proposed new_content so the
// approval path can write it to disk without re-shipping it through the IPC.
struct Entry {
    proposal_id: i64,
    file_path: PathBuf,
    new_content: String,
    sender: oneshot::Sender<DiffResolution>,
}

#[derive(Default)]
struct Inner {
    entries: HashMap<String, Entry>,
}

#[derive(Clone)]
pub struct PendingDiffs {
    inner: Arc<Mutex<Inner>>,
    emitter: Arc<dyn DiffEmitter>,
    db: DbHandle,
}

impl PendingDiffs {
    pub fn new(emitter: Arc<dyn DiffEmitter>, db: DbHandle) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            emitter,
            db,
        }
    }

    // The tool entry point. Reads the pre-image off disk (empty string if the
    // file doesn't exist yet), computes a unified diff, writes a proposal row
    // via the M1 DbHandle, emits the event, and returns a receiver to await.
    pub async fn fire(
        &self,
        session_id: i64,
        file_path: PathBuf,
        new_content: String,
    ) -> Result<(DiffProposal, oneshot::Receiver<DiffResolution>), McpError> {
        let old_content = match tokio::fs::read_to_string(&file_path).await {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(McpError::Io(e)),
        };

        let path_str = file_path.to_string_lossy().to_string();
        let patch = compute_unified_diff(&old_content, &new_content, &path_str);

        // M5: stash new_content alongside the patch so the renderer's
        // diff_apply_and_resolve command can write the file on approve
        // without re-parsing the unified diff.
        let proposal = self
            .db
            .create_diff_proposal(
                session_id,
                path_str.clone(),
                None,
                patch.clone(),
                Some(new_content.clone()),
            )
            .await?;

        let id = Ulid::new().to_string();
        let (tx, rx) = oneshot::channel();
        let request = DiffRequest {
            id: id.clone(),
            proposal_id: proposal.id,
            session_id,
            file_path: path_str,
            patch,
        };

        {
            let mut guard = self.inner.lock().await;
            guard.entries.insert(
                id.clone(),
                Entry {
                    proposal_id: proposal.id,
                    file_path: file_path.clone(),
                    new_content,
                    sender: tx,
                },
            );
        }

        if let Err(e) = self.emitter.emit_diff(&request) {
            let mut guard = self.inner.lock().await;
            guard.entries.remove(&id);
            return Err(e);
        }

        Ok((proposal, rx))
    }

    // Renderer-driven resolution. action is "approve" or "reject". On
    // approve we write new_content to disk; on reject we just update the
    // proposal row. Either way we send the resolution down the oneshot.
    pub async fn resolve(&self, id: &str, action: &str) -> Result<(), McpError> {
        let entry = {
            let mut guard = self.inner.lock().await;
            guard
                .entries
                .remove(id)
                .ok_or_else(|| McpError::NotFound(format!("pending diff {}", id)))?
        };

        // Update the DB row first so the audit trail is correct even if the
        // file-write fails.
        let proposal = self
            .db
            .resolve_diff_proposal(entry.proposal_id, action.to_string())
            .await?;

        let status = match proposal.status.as_str() {
            "approved" => DiffStatus::Approved,
            "rejected" => DiffStatus::Rejected,
            other => return Err(McpError::Invalid(format!("unexpected status {}", other))),
        };

        if matches!(status, DiffStatus::Approved) {
            // Make sure the parent dir exists. The agent might be proposing a
            // brand-new file in a nested directory.
            if let Some(parent) = entry.file_path.parent() {
                if !parent.as_os_str().is_empty() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }
            tokio::fs::write(&entry.file_path, entry.new_content.as_bytes()).await?;
        }

        let resolution = DiffResolution {
            proposal_id: entry.proposal_id,
            status,
        };
        let _ = entry.sender.send(resolution);
        Ok(())
    }
}
