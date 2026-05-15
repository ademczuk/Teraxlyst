# Teraxlyst Roadmap

Status: draft, planning phase.

Estimates assume part-time solo work, ~10-15 hours/week. They are intentionally loose. Each milestone has a clear exit criterion and a primary risk callout. Milestones can ship out of order if a feature unblocks another.

## M0 - Fork and rebrand (1 week)

Goal: a working terax-ai build under the Teraxlyst name, with attribution to upstream.

Tasks:

1. Clone https://github.com/crynta/terax-ai at tag **v0.6.5** (released 2026-05-15) into a separate working tree. Do not track main; we'll merge selectively.
2. Strip terax branding from `tauri.conf.json`, `package.json`, app titles, icons.
3. Preserve the original LICENSE (Apache-2.0) and copyright headers. Add a `NOTICE.md` recording the fork lineage from terax-ai 0.6.5 and design-inspiration credit to nimbalyst.
4. Replace icons and visual identity.
5. Verify `pnpm tauri dev` runs on Windows + macOS + Linux.
6. Set up CI: GitHub Actions on `ubuntu-latest`, `macos-14`, `windows-latest`. Cache cargo + node_modules. Build artifact upload only on tagged releases.
7. The repo at `github.com/ademczuk/Teraxlyst` is created during this milestone with planning docs + the M0 commit pushed together.

Exit criterion: `pnpm tauri build` produces a signed (or at least named) Teraxlyst binary on at least one platform, with terax-ai branding fully removed.

Risk: license attribution. Apache-2.0 requires preserving the original NOTICE file and copyright headers. We add a `NOTICE.md` listing terax-ai as the upstream and ensure all original copyright lines remain intact in source files.

## M1 - Native persistence layer (2 weeks)

Goal: replace any browser-side state with a Rust-managed SQLite database.

Tasks:

1. Add `rusqlite` with the `bundled` and `serde_json` features.
2. Implement `DbActor`: a `tokio::task` owning a single rusqlite connection in WAL mode, receiving `DbRequest` messages on an mpsc channel.
3. Add a reader pool (4-8 read-only connections) for concurrent reads.
4. Write the initial schema (workspaces, sessions, session_events, trackers, tracker_items, diff_proposals) from ARCHITECTURE.md §2.
5. Implement `sqlx::migrate!`-equivalent: use `refinery` or hand-rolled migrations embedded as `include_str!` strings, run at startup before the webview opens.
6. Wire Tauri commands: `db_create_workspace`, `db_list_workspaces`, `db_create_session`, `db_append_event`, `db_list_events`.
7. Integration test: spawn the actor, write 1000 events across 10 sessions concurrently, verify no `SQLITE_BUSY` and correct seq ordering.

Exit criterion: the renderer can create a workspace, create a session, write 100 transcript events, list them back, and quit cleanly with the DB file intact.

Risk: write-amplification under heavy transcript load. If a fast model streams 500 tokens/sec and we write one row per token, we'll saturate the writer. Mitigation: batch transcript inserts in 50ms windows on the Rust side before writing.

## M2 - Multi-session manager + transcript streaming (3 weeks)

Goal: parallel AI agent sessions with live transcript streaming to the UI.

Tasks:

1. Implement `SessionManager`: registry of running sessions, `create / pause / kill / subscribe` methods.
2. Implement a `ClaudeCode` provider adapter that spawns `claude-code` as a subprocess, parses its stdout into a canonical `TranscriptEvent` enum, writes to DB and emits to renderer.
3. Implement debounced channel emit: collect events in a 50ms window, emit a single batch.
4. Implement `Codex` provider adapter (parity with Claude Code adapter).
5. Renderer: session list view, transcript view, "create session" form.
6. Renderer: live kanban view showing all running sessions, hover a card to see the transcript streaming.
7. Renderer: session resume - select a past session, see its full history reconstructed from `session_events`.
8. Integration test: launch two sessions simultaneously, verify both transcripts stream without interleaving, both DB-persist correctly, killing one doesn't affect the other.

Exit criterion: user can launch 3+ Claude Code sessions in parallel, watch transcripts stream live in a kanban view, kill any session cleanly, and reopen the app to see all history reconstructed.

Risk: process cleanup on Windows. Tauri exit must kill all spawned agent processes. terax-ai already solves this with `shared_child` + Windows Job Objects. Reuse that machinery.

## M3 - MCP host + PromptForUserInput (3 weeks)

Goal: in-process MCP server exposing Teraxlyst tools to agents, including structured UI prompts.

Tasks:

1. Add the `rmcp` crate (v1.7+). Use its server-mode + stdio transport for our in-process host; use its client-mode for external MCP servers we connect to.
2. Implement `McpHost` as a thin wrapper around `rmcp`'s tool router. Each tool is a Rust function annotated with `#[tool]`.
3. Implement built-in tools: `tracker_create`, `tracker_update`, `tracker_list`, `tracker_query`, `propose_diff`, `read_workspace_file`, `prompt_for_user_input`.
4. Implement the prompt UI pipeline: `prompt_for_user_input` emits a Tauri channel event with `{id, field_schema}`, the renderer renders a form, the user submits, the renderer invokes a command, the McpHost resolves the pending oneshot.
5. Renderer: a generic form component for the five field types (multi-select, single-select, reorder, edit-text, confirm).
6. External MCP server registration via config (`<workspace>/.teraxlyst/mcp.json`). Spawn child processes with stdio transport.
7. Integration test: agent calls `prompt_for_user_input` with a multi-select schema, the renderer shows the form, the user submits, the agent receives the response and continues.

Exit criterion: a Claude Code session running inside Teraxlyst can call `prompt_for_user_input`, display a form to the user, and use the user's response to continue the conversation.

Risk: MCP spec churn. The protocol is young; the spec has had breaking changes. Pin to a specific `rmcp` minor version, document in `Cargo.toml`, and re-test against newer versions before upgrading. Less acute than originally feared because `rmcp` 1.x is stable and we don't implement the wire protocol ourselves.

## M4 - Tracker system (3 weeks)

Goal: YAML-defined trackers with kanban view and MCP integration.

Tasks:

1. YAML parser: load `<workspace>/.teraxlyst/trackers/*.yaml`, validate with serde + custom rules (role uniqueness, valid field types).
2. Schema-to-DB sync: insert/update `trackers` rows on workspace open. File watcher reloads on YAML change.
3. ID format generation: ulid, uuid, sequential with prefix (e.g. `BUG-014`). Configurable per tracker.
4. CRUD Tauri commands: `tracker_item_create`, `tracker_item_update`, `tracker_item_delete`, `tracker_item_list_by_status`.
5. MCP tools: typed wrappers around the commands, with schema validation against the loaded tracker definition.
6. Renderer: kanban board view, grouped by `workflowStatus` role.
7. Renderer: table view with sortable columns.
8. Renderer: inline `#tracker[BUG-014]` reference parser (markdown extension), rendered as a pill showing title + status.
9. Renderer: per-item profile view with all fields editable.
10. Integration test: create a Bugs tracker via YAML, agent calls `tracker_create({tracker: 'Bugs', title: 'CI broken', severity: 'high'})`, the kanban view updates live.

Exit criterion: a user can define a custom tracker via YAML, see it as a kanban board, an agent can create/update items via MCP, and inline references in markdown render correctly.

Risk: YAML schema versioning. If a user edits a tracker YAML after items exist, fields may become invalid. Mitigation: store the schema_json version per item insert, render legacy items with a "schema migration needed" indicator, never reject reads.

## M5 - Visual diff approval UI (2 weeks)

Goal: agents propose file changes via MCP; users approve or reject per file.

Tasks:

1. Add the `similar` crate. Implement `compute_unified_diff(old, new) -> String`.
2. MCP tool `propose_diff({path, new_content})`: reads pre-image, computes diff, writes `diff_proposals` row, awaits resolution.
3. Resolution: Tauri command `diff_resolve({id, action: 'approve' | 'reject'})`. On approve, write new_content to disk + git stage if in a repo.
4. Renderer: diff inbox showing pending proposals.
5. Renderer: Monaco diff editor for each proposal, with Approve / Reject buttons.
6. Renderer: keyboard shortcuts for bulk approve/reject.
7. Edge cases: file deleted between propose and resolve, file modified between propose and resolve (3-way merge fallback), binary files (reject with clear error).

Exit criterion: an agent proposes a multi-file edit, the user reviews each file in Monaco, approves a subset, the approved files write to disk and the agent receives a per-file result.

Risk: large diffs causing UI freeze. The deep_research guidance is to compute diffs in Rust and parse in a Web Worker. v1 sends JSON-encoded diff strings via Tauri command; if Monaco struggles with 10K+ line diffs, switch to MessagePack + Worker in v1.1.

## M6 - Polish + release (2 weeks)

Goal: ship a 0.1.0 release.

Tasks:

1. Auto-updater wiring via `tauri-plugin-updater` (terax-ai already has this).
2. Crash reporting (Sentry or in-app log dump).
3. Onboarding flow: first-run wizard that creates a workspace, configures a provider, runs a test session.
4. Docs: README, ARCHITECTURE, CONTRIBUTING, a quickstart guide.
5. Tests: integration test suite running on CI.
6. Binary release artifacts for Windows + macOS + Linux. macOS signing is a known PITA; defer notarization to a later patch release if it blocks.
7. Public announcement.

Exit criterion: 0.1.0 binaries downloadable, install on all three platforms, run a session end-to-end without errors.

## Cumulative timeline

| Milestone | Estimate | Cumulative |
|-----------|----------|------------|
| M0 fork + rebrand | 1 week | 1 |
| M1 persistence | 2 weeks | 3 |
| M2 sessions + streaming | 3 weeks | 6 |
| M3 MCP host | 3 weeks | 9 |
| M4 trackers | 3 weeks | 12 |
| M5 diff approval | 2 weeks | 14 |
| M6 release | 2 weeks | 16 |

16 weeks of part-time work = ~4 months wall-clock. Slips are likely; the realistic "first usable release" window is 5-6 months from M0 start.

## What's NOT in v1

These are explicit cuts. They are valuable but not load-bearing for the first release.

- Realtime collaboration (AGPL Yjs avoidance; defer until Automerge/Loro path is designed).
- Voice mode (OpenAI Realtime API; design later).
- Mobile companion app (iOS + Android; separate effort).
- Extension SDK (third-party plugins; design hooks during M3-M5 but ship registry in v1.1).
- WYSIWYG editors beyond Monaco diff and a markdown renderer (Excalidraw, Mermaid, CSV, data-model). Add in v1.1-v1.3.
- Embedded terminal (terax-ai already has PTY; reuse it as-is, no Ghostty port).
- Cloudflare-backed collab server (would carry AGPL).

## Risk register

| Risk | Severity | Likelihood | Mitigation |
|------|----------|------------|------------|
| AGPL contamination via Yjs port | High | Low (deferred) | Skip collab in v1; pick MIT CRDT later |
| Process cleanup on Windows | Medium | Medium | terax-ai's shared_child + Job Objects pattern |
| MCP spec churn breaks integrations | Medium | Medium | Pin to a spec version, gate upgrades behind a test suite |
| Token-stream IPC flood freezes UI | Medium | High if naive | Debounce in Rust at 50ms |
| Large diff renders block UI | Medium | Medium | MessagePack + Web Worker if Monaco struggles |
| nimbalyst pattern reuse without attribution | Medium | High if careless | Attribute in NOTICE.md, FEATURE_MAP.md, README |
| Solo dev burnout / slips | High | Medium | Keep scope tight, ship M0+M1 early as a "kitchen sink replacement for terax-ai" so the project has shipped value even if M2-M6 stall |
