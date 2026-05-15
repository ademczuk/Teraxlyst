// Tauri commands that the renderer invokes to drive the MCP pipelines.
//
//   - mcp_prompt_response: resolve a pending prompt with the user's form data
//   - mcp_diff_resolve:    approve or reject a pending diff
//   - mcp_list_pending_prompts: rehydrate the dialog after a renderer reload

use serde_json::Value;
use tauri::State;

use super::server::McpHandle;
use super::types::PendingPromptSummary;

#[tauri::command]
pub async fn mcp_prompt_response(
    handle: State<'_, McpHandle>,
    id: String,
    value: Value,
) -> Result<bool, String> {
    Ok(handle.prompts.resolve(&id, value).await)
}

#[tauri::command]
pub async fn mcp_diff_resolve(
    handle: State<'_, McpHandle>,
    id: String,
    action: String,
) -> Result<(), String> {
    handle
        .diffs
        .resolve(&id, &action)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mcp_list_pending_prompts(
    handle: State<'_, McpHandle>,
) -> Result<Vec<PendingPromptSummary>, String> {
    Ok(handle.prompts.list().await)
}
