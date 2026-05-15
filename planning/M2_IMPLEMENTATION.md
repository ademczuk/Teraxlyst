# M2 Implementation: Multi-session manager + transcript streaming

Date: 2026-05-15
Milestone: M2.0 (reduced scope vs the original ROADMAP M2)

## Files created

Rust under `src-tauri/src/sessions/`:

- `mod.rs` - re-exports `SessionManager`, `Provider`, `TranscriptEvent`, `ManagerError`. Gates `tests` on `#[cfg(test)]`.
- `types.rs` - `TranscriptEvent` enum (UserMessage, AssistantText, ToolCall, ToolResult, SystemNotice, Error, Completed), `Provider` enum (ClaudeCode only), `SessionEventBatch`. `db_kind()` + `db_payload()` map to the M1 schema.
- `error.rs` - `ManagerError` via thiserror with Serialize-as-Display mirroring DbError.
- `provider_claude_code.rs` - `spawn_claude_code` builds `claude --print <prompt>` (or a test override), returns `Arc<SharedChild>`. `parse_line` stub: every non-empty line becomes `AssistantText`.
- `manager.rs` - `SessionManager` owns `Arc<Mutex<HashMap<i64, RunningSession>>>`. Generic over `R: Runtime` so test (MockRuntime) and prod (Wry) share one path. Three tasks per session: stdout reader, flusher (50ms debounce), child watcher.
- `commands.rs` - three Tauri commands: `session_create`, `session_kill`, `session_list_running`. Non-generic `AppHandle` so `generate_handler!` is happy.
- `tests.rs` - two `#[cfg(unix)]` integration tests + one `#[ignore]` placeholder.

TypeScript under `src/modules/sessions/`:

- `types.ts` - mirrors the Rust types. `TranscriptEvent` is a discriminated union keyed on `type`.
- `useSessionManager.ts` - `useSessions()` hook. Calls `session_list_running` on mount, listens on `teraxlyst:session-events`, keeps a per-session ring buffer (cap 200), exposes `createSession` / `killSession` / `refreshRunning`.
- `SessionList.tsx` - the minimal UI: prompt textarea, Start button, running list with Kill buttons, flat tail of the last 50 events.
- `index.ts` - re-exports.

Wiring:

- `src-tauri/Cargo.toml` - added `time` to main tokio features, added `time` + `test-util` to dev tokio features, added `tauri = { features = ["test"] }` as a dev-dep so `tauri::test::mock_app` is available.
- `src-tauri/src/lib.rs` - `mod sessions;`, `app.manage(sessions::SessionManager::new())`, three commands registered.
- `src/app/App.tsx` - imported `SessionList`, added a floating toggle button + a 360x60vh overlay panel. Non-invasive: no changes to tab/panel layout.

## Design decisions

**One generic seam, one concrete edge.** Manager methods take `AppHandle<R: Runtime>` so the same code path runs in tests and production. Tauri's `generate_handler!` is fussy about generic commands, so the three commands take concrete `AppHandle` and delegate.

**Three tasks per session.** Reader (spawn_blocking, sync stdout), flusher (async, 50ms tokio interval), watcher (async, awaits child exit via inner spawn_blocking). Kill aborts all three. Natural completion cascades: child exits, stdout closes, reader returns, mpsc Sender drops, flusher recv yields None, flusher exits. The watcher then writes Completed and prunes.

**Debounce per ROADMAP M1 risk callout.** 50ms tokio interval ticks while events accumulate. Each tick (or channel-close) drains the buffer, writes events one-at-a-time to the DB (M1 only exposes single-event writes), and emits one batched `SessionEventBatch` to the renderer.

**Test seam, not test runtime.** Rather than fake the subprocess, the manager exposes `create_with_program(..., program, args)` behind `#[cfg(test)]`. The test spawns `bash -c "echo line1; echo line2; echo line3"` and exercises the same pipeline that would otherwise spawn `claude`.

## What is scaffold vs production

Production-ready: the full lifecycle (spawn, read, parse, debounce, DB write, renderer emit, exit, prune). Two integration tests exercise it. Kill is cross-platform via `shared_child`. The TS hook maintains per-session transcripts and drops session ids on `completed`.

Scaffold (marked TODO in source):

- The `claude --print <prompt>` invocation is best-guess. Real flags need verification against the user's installed CLI. TODO near `CLAUDE_BIN`.
- `parse_line` treats every non-empty line as `AssistantText`. Tool-call, permission-prompt, and error detection are M2.1 work once we have real samples.
- `SessionList.tsx` is minimal: single-workspace picker (just `workspaces[0]`), flat event log, no per-session detail. Kanban is M2.1.

## Known v1.1 cuts

1. **Kanban view.** ROADMAP M2 task 6. M2.0 ships a flat list; kanban is M2.1.
2. **Codex provider adapter.** ROADMAP M2 task 4. The `Provider` enum is structured to accept it; M2.1 adds the second adapter.
3. **Session resume.** ROADMAP M2 task 7. The DB has the full transcript; the UI does not yet reconstruct it on session click.
4. **Workspace picker.** Uses `workspaces[0]`. M3+ adds a real picker.
5. **Batched DB writes.** Flusher writes one event at a time. M2.1 batches via a new DbRequest variant.
6. **Real-bin smoke test.** `real_claude_code_smoke` is `#[ignore]`d because CI has no `claude`. M2.1 enables it on a runner with the CLI.

## Risk callouts

- **Provider parser is fragile.** Plain-text fallback works for the common case but a real Claude Code session emits structured tool calls and permission prompts. Rework in M2.1.
- **Windows test skip.** The integration test is `cfg(unix)` because `bash` is not universally available on Windows. Linux CI runs it; Windows CI skips cleanly.
