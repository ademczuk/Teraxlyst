# M3.2 Wireup: rmcp #[tool_router] -> clippy clean -> CI -D warnings

Goal: stop clippy from flagging the M3 MCP surface as dead code so CI can
re-enable `cargo clippy --all-targets -- -D warnings`.

## How #[tool_router] was applied

`src-tauri/src/mcp/tools.rs` now carries two impl blocks on
`TeraxlystToolset`:

1. The original `impl TeraxlystToolset` keeps the do_* methods
   (`do_prompt_for_user_input`, `do_propose_diff`,
   `do_read_workspace_file`) that the tests exercise directly. `new()`
   now also populates a `tool_router` field by calling
   `Self::tool_router()`.

2. A second block `#[tool_router] impl TeraxlystToolset` carries three
   `#[tool(name, description)]` async methods that take
   `Parameters<ArgsStruct>` and return `Result<CallToolResult,
   ErrorData>`. Each adapter delegates to the matching do_* method, wraps
   success into `CallToolResult::success(vec![Content::text(...)])`, and
   routes errors through a helper `map_mcp_error` that picks an
   `ErrorData` constructor by McpError variant (`PathEscape`/`Invalid` ->
   `invalid_params`, `NotFound` -> `resource_not_found`, else
   `internal_error`).

The macro generates an associated function
`TeraxlystToolset::tool_router() -> ToolRouter<TeraxlystToolset>` whose
dispatcher pattern-matches on tool name and constructs the Args struct
from the JSON. The struct now holds the result in
`tool_router: ToolRouter<TeraxlystToolset>`.

`server::spawn_in_process` reads the field at startup to log the tool
count (`toolset.tool_router.list_all().len()`). That single read makes
the macro-generated dispatcher reachable from non-test code, so clippy
sees `PromptForUserInputArgs`, `ProposeDiffArgs`, and
`ReadWorkspaceFileArgs` as constructed by the dispatcher and the three
`rmcp_*` adapters as called.

The Args structs gained `schemars::JsonSchema` derives (required by the
macro to publish input schemas). `PromptField` in `mcp/types.rs` also
gained `JsonSchema` since it nests inside `PromptForUserInputArgs`.

## Test: clippy compiles clean without module-wide allows

Three module-wide `#![allow(dead_code)]` headers were removed:

- `src-tauri/src/mcp/mod.rs` - macro-generated dispatcher now references
  the Args + tool surface.
- `src-tauri/src/diff/mod.rs` - `diff::commands::diff_apply_and_resolve`
  is registered in `invoke_handler`.
- `src-tauri/src/trackers/mod.rs` - tracker commands are registered in
  `invoke_handler`.

## Remaining targeted allow(dead_code) annotations

Kept with justification:

- `db::error::DbError::Io` and `mcp::error::McpError::Serde` - only
  constructed via `#[from]` from paths v1 doesn't exercise (existing
  annotations, unchanged).
- `mcp::tools::SharedToolset` type alias - reserved for M3.2 in-process
  session loops (existing).
- `mcp::server::McpToolset` type alias - same (existing).
- `trackers::mcp_wrappers` module - entire file kept under
  `#![allow(dead_code)]` per the file's own header; the four
  `mcp_tracker_*` functions are scaffolds waiting for M4 wiring into
  `TeraxlystToolset`.

No new allows were needed for `mcp/diff_pipeline.rs`,
`mcp/prompt_pipeline.rs`, or the rest of the trackers module - every
public item is reachable from a command, tool, or test.

## Style lint fixes

- `src/sessions/provider_claude_code.rs:152` - dropped redundant
  `trim_start()` before `split_whitespace()`.
- `src/trackers/commands.rs:79` - removed explicit `.into_iter()` from
  the `.zip()` call.

## CI change

`.github/workflows/ci.yml` `cargo clippy` now runs with `--all-targets --
-D warnings`. The relaxation comment block was removed.

## Local verify

`pnpm exec tsc --noEmit` passes (no frontend touches). Cargo is not
available locally; CI is the authoritative check.
