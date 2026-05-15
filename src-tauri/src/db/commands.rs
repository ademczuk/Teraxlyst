// Tauri command wrappers around DbHandle. Each command takes a State<'_,
// DbHandle> and maps DbError to String at the IPC boundary (Tauri requires
// the error type be Serialize; we serialize the Display string).

use serde_json::Value;
use tauri::State;

use super::actor::DbHandle;
use super::types::{DiffProposal, Session, SessionEvent, Workspace};

#[tauri::command]
pub async fn db_create_workspace(
    handle: State<'_, DbHandle>,
    path: String,
    name: String,
) -> Result<Workspace, String> {
    handle
        .create_workspace(path, name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_list_workspaces(
    handle: State<'_, DbHandle>,
) -> Result<Vec<Workspace>, String> {
    handle.list_workspaces().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_create_session(
    handle: State<'_, DbHandle>,
    workspace_id: i64,
    provider: String,
    title: Option<String>,
) -> Result<Session, String> {
    handle
        .create_session(workspace_id, provider, title)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_append_event(
    handle: State<'_, DbHandle>,
    session_id: i64,
    kind: String,
    payload: Value,
) -> Result<SessionEvent, String> {
    handle
        .append_event(session_id, kind, payload)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_list_events(
    handle: State<'_, DbHandle>,
    session_id: i64,
    since_seq: Option<i64>,
) -> Result<Vec<SessionEvent>, String> {
    handle
        .list_events(session_id, since_seq.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_create_diff_proposal(
    handle: State<'_, DbHandle>,
    session_id: i64,
    file_path: String,
    base_hash: Option<String>,
    patch: String,
    new_content: Option<String>,
) -> Result<DiffProposal, String> {
    handle
        .create_diff_proposal(session_id, file_path, base_hash, patch, new_content)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_resolve_diff_proposal(
    handle: State<'_, DbHandle>,
    id: i64,
    action: String,
) -> Result<DiffProposal, String> {
    handle
        .resolve_diff_proposal(id, action)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn db_list_pending_proposals(
    handle: State<'_, DbHandle>,
    session_id: Option<i64>,
) -> Result<Vec<DiffProposal>, String> {
    handle
        .list_pending_proposals(session_id)
        .await
        .map_err(|e| e.to_string())
}
