# Feature Map: nimbalyst -> Teraxlyst

Mapping each nimbalyst feature to a Teraxlyst implementation plan. Used to scope work, attribute prior art, and decide what's in vs out for v1.

Legend:
- **Where** = which layer owns the feature (Rust / TS / Both)
- **Complexity** = 1 (trivial) to 5 (major subsystem)
- **v1** = ships in 0.1.0 (Y / N / Maybe)

## Tier 1: in v1 scope (per user direction)

| Feature | Where | Data model | IPC | Complexity | v1 |
|---------|-------|------------|-----|------------|-----|
| Custom tracker system (YAML-defined) | Both | trackers, tracker_items | tauri-cmd | 4 | Y |
| Tracker kanban board | TS | reads tracker_items | tauri-cmd + channel | 3 | Y |
| Inline `#tracker[ID]` markdown refs | TS | reads tracker_items | tauri-cmd | 2 | Y |
| Tracker MCP tools (create/update/list/query) | Rust | tracker_items | mcp-stdio | 3 | Y |
| Multi-session AI agent manager | Rust | sessions, session_events | tauri-cmd + channel | 5 | Y |
| Live transcript streaming kanban | Both | session_events | channel (debounced 50ms) | 4 | Y |
| Session search and resume | Both | session_events FTS | tauri-cmd | 2 | Y |
| Visual red/green diff approval UI | Both | diff_proposals | tauri-cmd + channel | 4 | Y |
| Monaco diff editor integration | TS | reads diff_proposals.patch | tauri-cmd | 2 | Y |
| MCP server hosting (in-process) | Rust | n/a | mcp-stdio | 4 | Y |
| External MCP server registration | Rust | mcp.json config | child-proc | 2 | Y |
| PromptForUserInput widget | Both | n/a | channel + tauri-cmd | 3 | Y |
| Per-provider transcript parsers | Rust | session_events | n/a | 3 | Y |
| Provider: Claude Code | Rust | sessions | subprocess | 3 | Y |
| Provider: Codex | Rust | sessions | subprocess | 3 | Y |

## Tier 2: v1.1-v1.3 candidates

| Feature | Where | Data model | IPC | Complexity | v1 |
|---------|-------|------------|-----|------------|-----|
| Multi-project rail (Discord-style) | TS | workspaces | tauri-cmd | 3 | Maybe |
| Lexical rich text editor for notes | TS | new table `documents` | tauri-cmd | 4 | N |
| Excalidraw whiteboard editor | TS | filesystem-backed | tauri-cmd | 3 | N |
| Mermaid diagram editor | TS | filesystem-backed | tauri-cmd | 2 | N |
| CSV editor | TS | filesystem-backed | tauri-cmd | 2 | N |
| Data model editor (.datamodel) | TS | filesystem-backed | tauri-cmd | 3 | N |
| Mockup editor (.mockup.html) | TS | filesystem-backed | tauri-cmd | 3 | N |
| Provider: Opencode (alpha) | Rust | sessions | subprocess | 3 | N |
| Provider: GitHub Copilot CLI | Rust | sessions | subprocess | 4 | N |
| PDF export from markdown | TS | n/a | tauri-cmd | 2 | N |
| LaTeX math rendering | TS | n/a | n/a | 1 | Maybe |
| AI-assisted commit message generation | Both | n/a | mcp-tool | 2 | Maybe |
| Git state management panel | Both | n/a | tauri-cmd | 3 | Maybe |
| Workspace sync modes (local / shared / hybrid) | Both | new fields on trackers | tauri-cmd | 3 | N |
| Extension SDK | Both | new tables | tauri-cmd + plugin-api | 5 | N |

## Tier 3: deferred (months/years out)

| Feature | Where | Data model | IPC | Complexity | v1 |
|---------|-------|------------|-----|------------|-----|
| Realtime collab (Yjs-equivalent) | Both | CRDT overlay tables | websocket | 5 | N |
| Voice mode (OpenAI Realtime API) | TS | session_events.kind='voice' | webrtc | 4 | N |
| Mobile companion app (iOS) | new | local HTTP API on Tauri side | http | 5 | N |
| Mobile companion app (Android) | new | local HTTP API on Tauri side | http | 5 | N |
| PostHog analytics integration | Both | n/a | http | 1 | N |
| Marketplace for extensions | Both | new tables | http | 4 | N |
| Embedded Ghostty terminal | Rust | n/a | n/a | 3 | N (reuse terax-ai PTY) |

## Source mapping

Each Tier 1 feature is informed by specific nimbalyst patterns. Where the design is non-obvious, we cite the upstream source.

### Custom tracker system
- nimbalyst doc: UserDocs/creating-custom-trackers.md
- Pattern reused: YAML schema, field type enum, roles concept, ID format options, MCP tool surface
- Adapted: in nimbalyst, trackers can be synced via collab; in Teraxlyst v1, local-only

### Multi-session manager
- nimbalyst pattern: two-tier transcript (raw provider payload + canonical event)
- Pattern reused: append-only event log per session, per-provider parser as pure function, kanban view of running sessions
- Adapted: session_events table replaces nimbalyst's PGLite event store, debounced channel emit replaces direct WebSocket push

### Visual diff approval
- nimbalyst components: DiffPreview, TextDiffViewer, MonacoDiffViewer
- Pattern reused: per-file approve/reject cards, Monaco diff editor for visualization
- Adapted: diffs computed in Rust (similar crate), persisted as unified diff strings in SQLite, not in renderer state

### MCP host + PromptForUserInput
- nimbalyst doc: README + release notes for v0.60.1
- Pattern reused: structured input widget tool (multi-select, single-select, reorder, edit-text, confirm field types), MCP server hosted by the app itself
- Adapted: tool dispatch lives in Rust, UI rendering lives in TS, oneshot resolution via correlation IDs

## What we are deliberately NOT copying

- **PGLite-in-WASM database.** We use native SQLite via rusqlite. See ARCHITECTURE.md §2.
- **Jotai atoms for app state.** terax-ai uses a different React state approach; we keep theirs to reduce churn.
- **Cloudflare Durable Objects + Yjs CRDT.** AGPL-3.0; deferred and will be replaced with Automerge/Loro when added.
- **Ghostty terminal embed.** terax-ai already has a PTY layer via portable-pty. Reusing it.
- **Lexical for transcripts and notes.** Overkill for chat transcripts; we render markdown with a simple parser. Lexical may come in v1.2 for the notes editor.
- **Mobile apps.** Different repo, different effort, design for hookability via local HTTP API later.

## Anti-features (do not implement)

Things nimbalyst has been bitten by, that we explicitly avoid:

1. **Silent env-var API key reads.** nimbalyst removed a bug where it read `ANTHROPIC_API_KEY` from env and billed the user's personal account. We never read API keys from env vars; keychain only.
2. **localStorage in renderer.** All persistent state goes through the Rust DB. Renderer is stateless across reloads.
3. **Dynamic MCP server imports.** nimbalyst hit `__ELECTRON_LOG__` crashes with dynamic imports. Our MCP host is static and Rust-native.
4. **Direct file access to the DB from non-MCP code.** All DB reads/writes go through the actor.

## Attribution plan

In repo:
- `NOTICE.md` lists terax-ai (Apache-2.0) and nimbalyst (license per their LICENSE file) with links and a short description of what was reused.
- Each pattern-reused source file gets a short header comment: `// Pattern adapted from nimbalyst: <link>` where applicable.
- The README's "Inspired by" section names both projects with links.
