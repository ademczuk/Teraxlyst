// Canonical session-manager types.
//
// TranscriptEvent is the provider-agnostic event the SessionManager writes to
// the DB and emits to the renderer. Each provider adapter is responsible for
// parsing its raw stdout into this enum.
//
// In M2.0 the Claude Code adapter emits AssistantText for every non-empty
// stdout line. Tool-call / tool-result detection is a M2.1 task once we have
// actual Claude Code output samples to reverse-engineer.

use serde::{Deserialize, Serialize};

// Note: we use struct-form variants for the text-payload variants because
// serde's internally-tagged enum representation does not support newtype
// variants with primitive payloads. Wrapping the text in { text: String }
// gives us a clean discriminated-union shape on the TS side.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TranscriptEvent {
    UserMessage {
        text: String,
    },
    AssistantText {
        text: String,
    },
    ToolCall {
        name: String,
        args: serde_json::Value,
    },
    ToolResult {
        name: String,
        payload: serde_json::Value,
    },
    SystemNotice {
        text: String,
    },
    Error {
        text: String,
    },
    Completed,
}

impl TranscriptEvent {
    // The "kind" string we use as the session_events.kind column. Matches the
    // ARCHITECTURE.md schema's set: user, assistant, tool_call, tool_result,
    // plus two M2-extension kinds (system_notice, error, completed) that the
    // schema accepts because the column is a free-form TEXT.
    pub fn db_kind(&self) -> &'static str {
        match self {
            TranscriptEvent::UserMessage { .. } => "user",
            TranscriptEvent::AssistantText { .. } => "assistant",
            TranscriptEvent::ToolCall { .. } => "tool_call",
            TranscriptEvent::ToolResult { .. } => "tool_result",
            TranscriptEvent::SystemNotice { .. } => "system_notice",
            TranscriptEvent::Error { .. } => "error",
            TranscriptEvent::Completed => "completed",
        }
    }

    // JSON payload that lands in session_events.payload. We re-serialize the
    // event itself so the row contains the full discriminated union and a
    // future v1.1 reader can reconstruct typed events without a side-table.
    pub fn db_payload(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

// Provider tag. v1 ships ClaudeCode only; Codex + Opencode are placeholders.
// Kept as an enum (vs a string) so adding a provider is a compile-time
// exhaustiveness signal in the manager + parser.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    ClaudeCode,
    // M2.1+:
    // Codex,
    // Opencode,
}

impl Provider {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Provider::ClaudeCode => "claude-code",
        }
    }
}

// Debounced batch the manager pushes to the renderer via Tauri emit. Renderer
// listens on "teraxlyst:session-events" and dispatches per session_id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEventBatch {
    pub session_id: i64,
    pub events: Vec<TranscriptEvent>,
}
