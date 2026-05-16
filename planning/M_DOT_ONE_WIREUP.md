# M3.1 / M4.1 / M5.1 / M2.1 wireup notes

Closes the four "scaffold-only" callouts from `0.1.0-pre.6`. Lib.rs now
mounts every M3/M4/M5 surface, the watcher race in M2 is fixed, the
Claude Code parser knows more than "every line is text", and CI runs
`cargo clippy -- -D warnings` again.

## Step 1: M3.1 wireup

`mcp::spawn_in_process(...)` was already invoked in the setup closure
and the three Tauri commands were already registered. Removed
`#![allow(dead_code)]` from `mcp/mod.rs`. Two type aliases reserved
for the M3.2 stdio bridge (`McpToolset`, `SharedToolset`) get targeted
`#[allow(dead_code)]` with one-line reasons.

## Step 2: M4.1 wireup

Registered all five tracker commands: `tracker_load_workspace`,
`tracker_create_item`, `tracker_update_item`, `tracker_list_items`,
`tracker_query`. Removed `#![allow(dead_code)]` from
`trackers/mod.rs`. `trackers::mcp_wrappers` keeps a file-local
`#![allow(dead_code)]` (M3.2 will wire its four functions into
`TeraxlystToolset` once the toolset grows a `DbHandle` field).

## Step 3: M5.1 wireup

`diff_apply_and_resolve` was already in the handler list. Removed
`#![allow(dead_code)]` from `diff/mod.rs`. Frontend: replaced the
commented-out `DiffInbox` import in `src/app/App.tsx` with a live
import and a toggle-button + overlay panel mirroring the Sessions
overlay. Button sits left of "Sessions"; panel is 520x60vh. M5.2 will
route it through the real layout.

## Step 4: M2.1 watcher race fix

Plan B from the spec, applied with a small extension. The watcher now:

1. Owns a clone of `line_tx` (the flusher's mpsc sender) AND owns the
   reader task's `JoinHandle`.
2. Awaits `child.wait()` on the blocking pool.
3. Awaits `reader_handle` so every parsed stdout line is enqueued
   first.
4. Sends `TranscriptEvent::Completed` through the cloned `line_tx`.
5. Drops the sender so the flusher's `recv()` returns None on the next
   poll and runs its terminal flush + exit.

The `RunningSession.tasks` vec no longer holds the reader handle; on
`kill()` the watcher abort cancels both its child-wait and its
reader-join, the reader sees stdout EOF naturally as the killed child
closes the pipe, and the flusher abort tears down the channel. Net
result: in the happy path Completed is deterministically the last DB
row and the last event in the renderer's batch.

Test update: `session_lifecycle_writes_events_to_db` now asserts
`kinds.last() == Some("completed")` instead of the old best-effort
"contains user + assistant" relaxation.

## Step 5: clippy -D warnings

`.github/workflows/ci.yml` reverted to `cargo clippy --all-targets --
-D warnings`. Dropped the M3+M4+M5 scaffold comment. Targeted
`#[allow(dead_code)]` markers added where the surface is intentionally
reserved for a near-term milestone:

- `db::error::DbError::Io` (planning M1.1 filesystem probing)
- `mcp::error::McpError::Serde` (M3.2 rmcp wire-format tool argument
  parsing)
- `mcp::server::McpToolset`, `mcp::tools::SharedToolset` (M3.2 stdio
  bridge / in-process AI session loops)
- `trackers::mcp_wrappers` file-level (M3.2 toolset wiring)

Every other previously-allowed module is now subject to real
dead-code checks.

## Step 6: local verification

- `pnpm exec tsc --noEmit`: clean.
- `pnpm build`: clean, 12.8s.
- Cargo not installed on this Windows box; CI will validate
  `cargo check`, `cargo clippy -- -D warnings`, and `cargo test --lib`.

## Step 7: Claude Code parser improvement (M2.2-ready)

Replaced the "every non-empty line is AssistantText" stub with a small
heuristic in `provider_claude_code::parse_line`:

- `[tool_call] <name> ...` or `tool_use: <name> ...` -> `ToolCall`
  with the parsed name (args left null until M2.2 introduces JSON
  envelopes).
- `[error]`, `error:`, `Error:` prefixes -> `Error`.
- substring match on `permission_request` / `awaiting_approval`
  anywhere in the line -> `SystemNotice` carrying the original text.
- everything else -> `AssistantText` (M2.0 behavior).

Added three unit tests covering the new branches. Documented inline as
M2.2-ready: the marker shapes still need to be confirmed against real
Claude Code stdout captures.

## Surprise fixes

None major. Dead-code purge surfaced two unused type aliases
(`McpToolset`, `SharedToolset`), two error variants only reachable
via `#[from]` from not-yet-exercised paths (`DbError::Io`,
`McpError::Serde`), and the `trackers::mcp_wrappers` module. All
resolved with targeted allows so the M3.2 plan stays implementable.
