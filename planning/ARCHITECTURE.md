# Teraxlyst Architecture

Status: draft, planning phase. No code yet.

This document captures the load-bearing technical decisions for Teraxlyst, a desktop app that fuses terax-ai's Tauri shell with a focused subset of nimbalyst's featureset. It is informed by:

- Direct source reading of both reference repos.
- A Pantheon council deliberation on the stack decision.
- A google-deep-research synthesis on Tauri 2 + AI-agent workspace patterns (2025-2026 ecosystem).
- Anthropic SDK and MCP protocol documentation.

If you change a decision here, update the rationale and date it.

## 1. Stack

**Verdict: Tauri 2 + Rust backend + React 19 frontend. Do not pivot to Electron.**

Rationale:

- The product identity is desktop-native orchestration (PTY supervision, process trees, filesystem, keychain, MCP hosting). Rust is the right language for that core, and terax-ai already provides a working shell around it.
- Replacing the shell with Electron would discard terax-ai's solved-edge-cases (Windows Job Objects for process groups, keyring abstraction, ripgrep-equivalent file search via ignore + grep-regex).
- Solo-dev part-time budget: replatforming costs more than it saves.

What we lose vs Electron: easier reuse of nimbalyst's existing TypeScript modules, and a larger ecosystem of mature web libraries.

What we gain vs Electron: smaller binary, lower idle RAM, cleaner separation of concerns, native process model that matches the agent-supervisor product role.

Frontend rendering: React 19 + Tailwind, matching terax-ai's existing setup. Editor surface for code uses Monaco (MIT). Rich text / markdown can use Lexical (MIT) if we want feature parity with nimbalyst, otherwise tiptap (MIT) or markdown-it.

## 2. Persistence

**Verdict: native SQLite via rusqlite, WAL mode, single-writer actor pattern. No PGLite in webview.**

Why not PGLite:

- PGLite is a Postgres-in-WASM database that lives in the renderer. nimbalyst uses it because Electron's renderer is a familiar place to run WASM.
- In Tauri, the renderer is also a webview, but our truth layer is Rust. Pushing the DB into the webview means the renderer becomes the source of truth and Rust becomes a proxy. That inverts the architecture and creates synchronization complexity.
- PGLite's PID locking is a workaround for browser-multi-tab contention. In a single-instance desktop app with all DB access flowing through Rust, OS-level SQLite locking + WAL is sufficient.

Why rusqlite over sqlx:

- Solo project, local-only DB, no remote backend planned. Compile-time query checking is nice but not load-bearing.
- sync rusqlite calls wrapped in `tokio::task::spawn_blocking` are fine. No async DB pool needed for this workload.
- Lower complexity, faster to learn, fewer macros, simpler error types.

Switch to sqlx if: we add a remote sync backend, or transcript volume grows beyond what a single writer can drain.

Schema (initial sketch, finalized in M1):

```sql
-- workspaces map to a directory on disk + git remote
CREATE TABLE workspaces (
    id         INTEGER PRIMARY KEY,
    path       TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- AI sessions are agent runs; one workspace -> many sessions
CREATE TABLE sessions (
    id            INTEGER PRIMARY KEY,
    workspace_id  INTEGER NOT NULL REFERENCES workspaces(id),
    parent_id     INTEGER REFERENCES sessions(id),
    provider      TEXT NOT NULL,     -- 'claude-code', 'codex', 'opencode'
    title         TEXT,
    status        TEXT NOT NULL,     -- 'running', 'paused', 'completed', 'failed'
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

-- Append-only event log per session; raw payloads kept verbatim
CREATE TABLE session_events (
    id         INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    seq        INTEGER NOT NULL,
    kind       TEXT NOT NULL,        -- 'user', 'assistant', 'tool_call', 'tool_result', 'diff'
    payload    TEXT NOT NULL,        -- JSON
    created_at INTEGER NOT NULL,
    UNIQUE (session_id, seq)
);

-- Trackers are YAML-defined entity types (Plans, Decisions, Bugs, Tasks, ...)
CREATE TABLE trackers (
    id            INTEGER PRIMARY KEY,
    workspace_id  INTEGER NOT NULL REFERENCES workspaces(id),
    name          TEXT NOT NULL,
    yaml_source   TEXT NOT NULL,
    schema_json   TEXT NOT NULL,
    enabled       INTEGER NOT NULL DEFAULT 1,
    UNIQUE (workspace_id, name)
);

CREATE TABLE tracker_items (
    id          INTEGER PRIMARY KEY,
    tracker_id  INTEGER NOT NULL REFERENCES trackers(id),
    public_id   TEXT NOT NULL,       -- 'BUG-014' style, configurable
    status      TEXT,
    fields_json TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    UNIQUE (tracker_id, public_id)
);

-- Diff proposals from agents, awaiting approval
CREATE TABLE diff_proposals (
    id          INTEGER PRIMARY KEY,
    session_id  INTEGER NOT NULL REFERENCES sessions(id),
    file_path   TEXT NOT NULL,
    base_hash   TEXT,                -- git blob hash of the pre-change file
    patch       TEXT NOT NULL,       -- unified diff
    status      TEXT NOT NULL,       -- 'pending', 'approved', 'rejected'
    created_at  INTEGER NOT NULL,
    resolved_at INTEGER
);

CREATE INDEX idx_session_events_session_seq ON session_events(session_id, seq);
CREATE INDEX idx_tracker_items_tracker_status ON tracker_items(tracker_id, status);
CREATE INDEX idx_diff_proposals_session_status ON diff_proposals(session_id, status);
```

Locking model:

- App is single-instance (Tauri single-instance plugin).
- All DB access flows through a `DbActor` task with an `mpsc::Sender<DbRequest>` handle held by Tauri command handlers.
- WAL mode enabled on connection open.
- Two connection roles: one writer connection (held by the actor), N reader connections (pooled, read-only).

## 3. Process model

```
+-----------------------------------+      +-------------------+
| Tauri renderer (React 19)         |<-->  | Rust backend      |
| - kanban view                     | IPC  | - DbActor         |
| - Monaco diff editor              | (CMD | - SessionManager  |
| - tracker forms                   |  +   | - McpHost         |
| - transcript stream consumer      |  CH) | - PtyPool         |
| - MCP PromptForUserInput widget   |      | - DiffEngine      |
+-----------------------------------+      +---------+---------+
                                                     |
                                     +---------------+---------------+
                                     |               |               |
                              +------+-----+  +------+------+  +-----+------+
                              | AI agents  |  | MCP servers |  | terminal   |
                              | child proc |  | child proc  |  | (PTY)      |
                              +------------+  +-------------+  +------------+
```

Tauri commands: synchronous request/response. Used for DB reads, config get/set, tracker CRUD, file ops.

Tauri channel events: streaming. Used for transcript chunks, file watcher events, MCP tool-call notifications. Token streaming is debounced in Rust at 50ms intervals to prevent UI freezes (deep_research finding).

## 4. AI session manager

Each session is a Rust `tokio::task` orchestrating one agent subprocess:

```rust
struct Session {
    id: i64,
    provider: Provider,         // ClaudeCode, Codex, Opencode
    child: SharedChild,         // shared_child crate for cross-platform kill
    transcript_tx: mpsc::Sender<TranscriptEvent>,
    tool_call_tx: mpsc::Sender<ToolCall>,
}
```

The `SessionManager` owns the registry of running sessions, exposes:

- `create(workspace_id, provider, prompt)`
- `pause(session_id)`
- `resume(session_id)`
- `kill(session_id)`
- `subscribe(session_id)` - returns a channel of `TranscriptEvent`s

Transcript events are append-only writes to `session_events` AND a debounced emit to the renderer via Tauri channel. The renderer never reads transcripts from the backend's in-memory buffer; it queries the DB and subscribes to new events.

Provider integration: per-provider adapter modules (`providers/claude_code.rs`, `providers/codex.rs`) translate raw stdout/stderr into a canonical `TranscriptEvent` enum. nimbalyst's two-tier system (raw provider payload + canonical event) is the right pattern here.

## 5. MCP host

Use `rmcp` v1.7+ (the official Anthropic Rust MCP SDK, ~4.7M downloads on crates.io as of 2026-05). Stable 1.x, stdio + SSE transport, declarative `#[tool]` and `#[tool_router]` macros for tool registration. This removes the largest M3 risk: we are not implementing the MCP wire protocol ourselves.

Two-mode strategy:

- **Mode 1 (default):** Teraxlyst hosts a built-in MCP server in-process via `rmcp` server mode. Tools: `tracker_create`, `tracker_update`, `tracker_list`, `tracker_query`, `prompt_for_user_input`, `propose_diff`, `read_workspace_file`.
- **Mode 2:** External MCP servers can be registered via config. Teraxlyst is an `rmcp` client of each, transports stdio for local servers and SSE for remote ones. Child-process lifecycle managed via `shared_child` so a Tauri exit kills all children.

A subtle directional point: when a managed AI session runs inside Teraxlyst, Teraxlyst is the **server** in that relationship. When Teraxlyst connects to an external tool MCP server, Teraxlyst is the **client**. Both roles run concurrently in the same Rust process.

The MCP host runs as part of the Rust backend. Tools that need user UI (`prompt_for_user_input`) emit a Tauri channel event with a request payload + correlation ID, the renderer renders the widget, the user responds, the renderer sends a Tauri command with the response + correlation ID, the host resolves the pending future.

```
Agent -> MCP tool call -> McpHost.dispatch -> [tool needs UI?]
                                                    |
                                                    yes -> emit('mcp-prompt', {id, schema})
                                                            renderer renders form
                                                            renderer invokes('mcp-prompt-response', {id, values})
                                                            McpHost resolves pending oneshot
                                                    |
                                                    no -> direct DB / fs / git op
```

PromptForUserInput field types: multi-select, single-select, reorder, edit-text, confirm. Schema defined in Rust, validated with serde. Renderer uses a registry-based form component.

## 6. Diff approval UI

Diffs are computed in Rust using the `similar` crate. The unified-diff string is stored in `diff_proposals.patch`. The renderer uses Monaco's diff editor (MIT) for visualization.

Approval flow:

1. Agent runs `propose_diff` MCP tool with `{path, new_content}`.
2. Rust loads pre-image from disk (or git HEAD), computes unified diff, writes a `diff_proposals` row with status='pending'.
3. Renderer subscribes to `diff_proposals` changes (channel event on insert). Renders a card with Monaco diff view.
4. User clicks Approve or Reject. Renderer invokes `diff_resolve` command.
5. On Approve: Rust writes new_content to disk via the existing file ops layer.
6. On Reject: Rust updates the row to status='rejected'.
7. Agent receives the resolution as the MCP tool result.

Performance: for large diffs (>10K lines), the deep_research advice was to serialize via MessagePack and parse in a Web Worker. Defer this until we hit the bottleneck; v1 uses JSON.

## 7. Tracker system

YAML files live in `<workspace>/.teraxlyst/trackers/*.yaml`. Schema mirrors nimbalyst's:

```yaml
name: Bugs
id_format: { type: sequential, prefix: BUG, pad: 3 }
fields:
  - { name: title,    type: string,  roles: [title] }
  - { name: severity, type: select,  options: [low, medium, high, critical] }
  - { name: status,   type: select,  roles: [workflowStatus], options: [open, in_progress, fixed, wontfix] }
  - { name: reporter, type: user,    roles: [reporter] }
  - { name: created,  type: datetime, auto: created_at }
```

Field types: string, text, number, select, multiselect, date, datetime, boolean, user, reference, array, object.

Roles (semantic markers): title, workflowStatus, priority, assignee, reporter, tags, startDate, dueDate, progress.

The Rust parser loads each YAML on workspace open, validates with serde + custom checks, writes the parsed schema to the `trackers` table. File watcher reloads on YAML change.

Tracker items render in two views in the renderer: a kanban board (grouped by workflowStatus) and a table.

MCP tools `tracker_create`, `tracker_update`, `tracker_list` are typed wrappers around DB writes, with the schema validated against the loaded tracker definition.

## 8. Realtime collab - deferred

nimbalyst's CollabV3 is AGPL-3.0 (Yjs + Cloudflare Durable Objects). Teraxlyst v1 ships single-user only.

When we add collab in a later phase, the path is:

1. Use Automerge (MIT), Diamond Types (MIT), or Loro (MIT). Yjs is off the table.
2. Self-host a thin WebSocket relay in Rust. Don't embed any AGPL component in our process or our DB schemas.
3. Keep DB writes the source of truth; the CRDT layer is an overlay on top of `session_events` and `tracker_items`.

The schema is already collab-friendly: append-only `session_events`, version-tolerant `tracker_items.fields_json`.

## 9. Security boundaries

- API keys live in the OS keyring (terax-ai's existing `secrets.rs`).
- Never read API keys from env vars. nimbalyst had a documented bug where it silently used `ANTHROPIC_API_KEY` from env and billed the user's personal account. We will not repeat this.
- MCP servers are child processes with their own permissions. No special privilege granted.
- Diff proposals must be approved by the user before file writes. Agent has no direct file-write capability.
- SSRF block on cloud metadata IPs in the AI HTTP proxy (inherited from terax-ai's `net.rs`).

## 10. Open architectural questions

These are not blockers but are worth flagging:

1. **Voice mode.** nimbalyst uses OpenAI Realtime API with a dual-agent setup. Skipping for v1. If we add it, it lives in the renderer (WebRTC) with a Rust audio passthrough only if needed for system audio routing.
2. **Mobile companion.** nimbalyst has iOS + Android. Out of scope for v1; if we add it, it'll be a separate repo speaking to a local Tauri-hosted HTTP API.
3. **Plugin/extension system.** nimbalyst has an extension SDK. v1 ships without; we'll design for it from M3 onward by keeping editor surface and MCP tools registry-based.

## 11. Provider integration model

How a "managed session" works at the runtime level is an open design choice:

- **(a) Subprocess mode.** Spawn `claude-code`, `codex`, `opencode` as child processes; parse their stdout into canonical events. Pros: nothing to implement on the agent loop. Cons: brittle if CLI output format changes, must reverse-engineer each provider's stdout.
- **(b) Native SDK mode.** Use the Anthropic API and OpenAI API directly from Rust, run our own agent loop. Pros: stable, fully controlled. Cons: we re-implement an agent loop and tool-use translation per provider.
- **(c) Hybrid.** Subprocess mode for users who already have the CLIs installed; native mode for users without.

v1 plan: subprocess mode only. Mode (b) is a v1.x addition once we know what tool-use translation costs.

## 12. Testing strategy

- **Rust unit tests:** standard `cargo test` for parsers, schema validation, diff computation, DB actor.
- **Rust integration tests:** spawn the full backend in-process, exercise Tauri commands directly (no webview). Use `tempfile` for ephemeral DBs.
- **End-to-end UI tests:** Tauri's WebDriver support (`tauri-driver`) + Playwright or WebdriverIO. Reserve for M5 onward; not in M1 budget.
- **MCP conformance tests:** use `rmcp`'s own client to drive our server, verify tool round-trips against the spec.
- **CI matrix:** GitHub Actions on `ubuntu-latest`, `macos-14`, `windows-latest`. Cache `~/.cargo` and `node_modules`.

## 13. Telemetry and privacy

- **Default: no telemetry.** No PostHog, no Sentry without explicit opt-in.
- Crash reports stored locally as a log dump the user can attach to bug reports manually.
- If we add opt-in telemetry later, it ships disabled-by-default with a first-run banner.
- API keys never logged.

## 14. Code signing and updates

- **Auto-updater:** `tauri-plugin-updater`, inherited from terax-ai.
- **macOS notarization:** punted to a post-0.1.0 patch release. The 0.1.0 release ships an unnotarized macOS binary with documented Gatekeeper bypass instructions.
- **Windows signing:** also punted. SmartScreen warnings expected, documented in install instructions.
- Signing certificates are a real money + identity cost we'll budget for after v0.2.

## References

- terax-ai source: https://github.com/crynta/terax-ai
- nimbalyst source: https://github.com/nimbalyst/nimbalyst
- Pantheon council ID: 41329411-583d-4720-8235-bd2fa9e4f6d6 (recorded 2026-05-15)
- google-deep-research synthesis: in-session output dated 2026-05-15
