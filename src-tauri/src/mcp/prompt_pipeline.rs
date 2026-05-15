// Correlation-ID pending-prompt machinery for prompt_for_user_input.
//
// Flow:
//   1. tool fires -> generate ulid, register oneshot in `pending`, emit a
//      `teraxlyst:mcp-prompt` event with {id, prompt_text, field}.
//   2. renderer renders the form, user submits, renderer invokes the
//      `mcp_prompt_response` Tauri command with {id, value}.
//   3. command looks up oneshot by id, sends value, tool returns.
//
// Tests inject a `MockEmitter` instead of the real Tauri AppHandle so we
// don't have to spin up a webview.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::{oneshot, Mutex};
use ulid::Ulid;

use super::error::McpError;
use super::types::{PendingPromptSummary, PromptField, PromptRequest, EVENT_PROMPT};

// Trait so tests can replace the AppHandle emit with a recording mock.
pub trait PromptEmitter: Send + Sync + 'static {
    fn emit_prompt(&self, payload: &PromptRequest) -> Result<(), McpError>;
}

// Real Tauri implementation. Wraps an AppHandle generic over Runtime.
pub struct TauriPromptEmitter<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriPromptEmitter<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> PromptEmitter for TauriPromptEmitter<R> {
    fn emit_prompt(&self, payload: &PromptRequest) -> Result<(), McpError> {
        self.app
            .emit(EVENT_PROMPT, payload)
            .map_err(|e| McpError::Emit(e.to_string()))
    }
}

// State for outstanding prompts. Each entry maps a correlation ID to the
// oneshot sender that the tool is awaiting on, plus a copy of the original
// PromptRequest so the renderer can rehydrate on reload.
struct Entry {
    request: PromptRequest,
    sender: oneshot::Sender<Value>,
}

#[derive(Default)]
struct Inner {
    entries: HashMap<String, Entry>,
}

#[derive(Clone)]
pub struct PendingPrompts {
    inner: Arc<Mutex<Inner>>,
    emitter: Arc<dyn PromptEmitter>,
}

impl PendingPrompts {
    pub fn new(emitter: Arc<dyn PromptEmitter>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            emitter,
        }
    }

    // Fire a prompt: register a pending entry, emit the event, return the
    // receiver that the caller awaits on.
    pub async fn fire(
        &self,
        prompt_text: String,
        field: PromptField,
    ) -> Result<oneshot::Receiver<Value>, McpError> {
        let id = Ulid::new().to_string();
        let (tx, rx) = oneshot::channel();
        let request = PromptRequest {
            id: id.clone(),
            prompt_text,
            field,
        };

        // Register first, emit second. If emit fails we still need to remove
        // the entry so the channel doesn't leak.
        {
            let mut guard = self.inner.lock().await;
            guard.entries.insert(
                id.clone(),
                Entry {
                    request: request.clone(),
                    sender: tx,
                },
            );
        }

        if let Err(e) = self.emitter.emit_prompt(&request) {
            // Roll back.
            let mut guard = self.inner.lock().await;
            guard.entries.remove(&id);
            return Err(e);
        }

        Ok(rx)
    }

    // Renderer-driven completion. Returns true if the id was found and the
    // value delivered, false if the id was unknown (stale reply).
    pub async fn resolve(&self, id: &str, value: Value) -> bool {
        let entry = {
            let mut guard = self.inner.lock().await;
            guard.entries.remove(id)
        };
        match entry {
            Some(e) => e.sender.send(value).is_ok(),
            None => false,
        }
    }

    // For renderer reload: list everything still pending so the UI can
    // re-render the dialog.
    pub async fn list(&self) -> Vec<PendingPromptSummary> {
        let guard = self.inner.lock().await;
        guard
            .entries
            .values()
            .map(|e| PendingPromptSummary {
                id: e.request.id.clone(),
                prompt_text: e.request.prompt_text.clone(),
                field: e.request.field.clone(),
            })
            .collect()
    }
}
