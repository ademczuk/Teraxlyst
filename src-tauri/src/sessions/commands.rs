// Tauri command wrappers around SessionManager. Mirrors the db::commands
// pattern: take State<'_, ...>, map error to String at the IPC boundary.

use tauri::{AppHandle, State};

use crate::db::actor::DbHandle;
use crate::db::types::Session;

use super::manager::SessionManager;
use super::types::Provider;

#[tauri::command]
pub async fn session_create(
    db: State<'_, DbHandle>,
    mgr: State<'_, SessionManager>,
    app: AppHandle,
    workspace_id: i64,
    provider: Provider,
    prompt: String,
) -> Result<Session, String> {
    mgr.create(&db, workspace_id, provider, prompt, app)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn session_kill(
    mgr: State<'_, SessionManager>,
    session_id: i64,
) -> Result<(), String> {
    mgr.kill(session_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn session_list_running(mgr: State<'_, SessionManager>) -> Result<Vec<i64>, String> {
    Ok(mgr.list_running().await)
}
