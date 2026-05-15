// MCP host. See planning/ARCHITECTURE.md sections 5 + 11.
//
// M3 scope (intentionally narrow):
// - One in-process MCP server with THREE tools:
//     * prompt_for_user_input  - emit UI form to renderer, await response
//     * propose_diff           - compute diff, write proposal row, await user
//     * read_workspace_file    - sandboxed read inside a workspace dir
// - PromptForUserInput pipeline (correlation-ID + oneshot) wired to a Tauri
//   channel event.
// - Diff resolution pipeline (correlation-ID + oneshot) backed by the M1
//   diff_proposals table.
//
// Tracker tools are M4. External MCP server hosting is M3.1.

pub mod commands;
pub mod diff_pipeline;
pub mod error;
pub mod prompt_pipeline;
pub mod server;
pub mod tools;
pub mod types;

#[cfg(test)]
mod tests;

pub use diff_pipeline::PendingDiffs;
pub use prompt_pipeline::PendingPrompts;
pub use server::{spawn_in_process, McpHandle, McpToolset};
