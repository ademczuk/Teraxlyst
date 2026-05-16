// MCP host. The 3 tools land in the runtime tool registry via the
// `#[tool_router]` macro on `TeraxlystToolset` in tools.rs. The macro
// generates a `Self::tool_router()` dispatcher that is stored in the
// `tool_router` field at construction time and read by
// `server::spawn_in_process`, which keeps every Args struct + tool
// function reachable for clippy without a module-wide allow.
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

pub use server::spawn_in_process;
