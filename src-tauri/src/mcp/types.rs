// Schema types for PromptForUserInput field variants and other MCP-layer
// payloads. The five field variants match the architecture doc section 5:
// multi-select, single-select, reorder, edit-text, confirm.
//
// We use a `#[serde(tag = "kind", rename_all = "kebab-case")]` tagged enum so
// the renderer sees stable JSON discriminators.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PromptField {
    MultiSelect {
        label: String,
        options: Vec<String>,
        min: Option<usize>,
        max: Option<usize>,
    },
    SingleSelect {
        label: String,
        options: Vec<String>,
    },
    Reorder {
        label: String,
        items: Vec<String>,
    },
    EditText {
        label: String,
        default: Option<String>,
        placeholder: Option<String>,
    },
    Confirm {
        label: String,
    },
}

// What we send to the renderer when a prompt fires. Correlation ID lets the
// renderer's reply land on the right oneshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRequest {
    pub id: String,
    pub prompt_text: String,
    pub field: PromptField,
}

// What we send to the renderer when a diff is proposed. proposal_id is the
// row id in diff_proposals; id is the correlation ID for the oneshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffRequest {
    pub id: String,
    pub proposal_id: i64,
    pub session_id: i64,
    pub file_path: String,
    pub patch: String,
}

// Resolution returned to the agent when the user clicks Approve/Reject.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResolution {
    pub proposal_id: i64,
    pub status: DiffStatus,
}

// Renderer-facing summary of an outstanding prompt (used by
// mcp_list_pending_prompts so the renderer can rehydrate after a reload).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPromptSummary {
    pub id: String,
    pub prompt_text: String,
    pub field: PromptField,
}

// Event names emitted on the Tauri channel. Kept in one place to avoid
// drift between Rust and TypeScript.
pub const EVENT_PROMPT: &str = "teraxlyst:mcp-prompt";
pub const EVENT_DIFF: &str = "teraxlyst:mcp-diff";
