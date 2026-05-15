// diff_apply_and_resolve: the M5 Tauri command that bridges the renderer's
// Approve/Reject decision back into the filesystem.
//
// Approve path:
//   1. Fetch the proposal row via DbHandle.
//   2. If new_content is None (legacy pre-v2 row), error out cleanly. The
//      renderer can fall back to telling the user to apply the patch
//      manually; we do not attempt unified-diff parsing in v1.
//   3. Atomic write: stage into a sibling .teraxlyst.tmp file and rename
//      over the target. Matches modules/fs/file.rs::fs_write_file
//      semantics so crash-mid-write leaves either the old or the new
//      file, never a partial.
//   4. Flip the proposal status to "approved" via the existing
//      resolve_diff_proposal path.
//
// Reject path:
//   1. No filesystem write.
//   2. Flip the proposal status to "rejected".
//
// The order matters on Approve: we write the file FIRST, then flip the
// status. If the write fails we leave the proposal pending so the user
// can retry. If the status flip fails after a successful write, the
// renderer sees a stale-pending row that nonetheless reflects what's on
// disk; a refresh will pick up the resolved status once eventual
// consistency catches up. Both error paths are exposed as Result::Err.

use std::io::Write as _;
use std::path::Path;

use serde::Serialize;
use tauri::State;

use crate::db::actor::DbHandle;
use crate::db::types::DiffProposal;

#[derive(Debug, Serialize)]
pub struct DiffApplyOutcome {
    pub proposal: DiffProposal,
    // True if we wrote new_content to disk. False on Reject or on a
    // legacy proposal where the write was skipped.
    pub wrote_file: bool,
}

#[tauri::command]
pub async fn diff_apply_and_resolve(
    handle: State<'_, DbHandle>,
    id: i64,
    action: String,
) -> Result<DiffApplyOutcome, String> {
    let proposal = handle
        .get_diff_proposal(id)
        .await
        .map_err(|e| e.to_string())?;

    if proposal.status != "pending" {
        return Err(format!(
            "diff_proposal {} is not pending (status={})",
            id, proposal.status
        ));
    }

    let action_lc = action.to_ascii_lowercase();
    let wrote_file = match action_lc.as_str() {
        "approve" => {
            let new_content = match proposal.new_content.as_deref() {
                Some(c) => c,
                None => {
                    return Err(format!(
                        "diff_proposal {} has no new_content (legacy pre-v2 row); \
                         apply the patch manually or recreate via propose_diff",
                        id
                    ));
                }
            };
            write_atomic(Path::new(&proposal.file_path), new_content)
                .map_err(|e| format!("write {}: {}", proposal.file_path, e))?;
            true
        }
        "reject" => false,
        other => {
            return Err(format!("unknown diff_apply action: {}", other));
        }
    };

    // Status flip after the side-effect succeeds. resolve_diff_proposal
    // returns the row with status/resolved_at set.
    let resolved = handle
        .resolve_diff_proposal(id, action_lc)
        .await
        .map_err(|e| e.to_string())?;

    Ok(DiffApplyOutcome {
        proposal: resolved,
        wrote_file,
    })
}

// Atomic write: stage to a sibling temp file, fsync, then rename. Mirrors
// modules/fs/file.rs::fs_write_file but lives here so the diff module
// doesn't need to depend on the workspace-env resolver (paths from
// proposals are already absolute).
fn write_atomic(target: &Path, content: &str) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| "path has no parent".to_string())?;
    let file_name = target
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "path has no file name".to_string())?;
    let tmp = parent.join(format!(".{}.teraxlyst.tmp", file_name));

    {
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })?;
    Ok(())
}
