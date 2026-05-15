# Teraxlyst

A desktop workspace for AI coding agents. Tauri 2 + Rust + React.

Status: **early planning.** No working code yet. See [ARCHITECTURE.md](./ARCHITECTURE.md) and [ROADMAP.md](./ROADMAP.md) for the plan.

## What it is

Teraxlyst takes the Rust-and-Tauri shell from [terax-ai](https://github.com/crynta/terax-ai) (terminal emulator + AI chat + file explorer) and rebuilds a focused subset of [nimbalyst](https://github.com/nimbalyst/nimbalyst)'s workspace features on top of it:

- Multi-AI-session manager with live transcript streaming
- Visual red/green diff approval for agent file changes
- YAML-defined custom tracker system (Plans, Decisions, Bugs, Tasks, Ideas, Features, Automations)
- In-process MCP server with structured PromptForUserInput widgets

The goal is one binary, native systems integration, and a clean Rust core, instead of an Electron stack.

## What it is NOT

- A clone of either upstream. terax-ai is a terminal emulator; nimbalyst is an Electron monorepo with mobile apps and a Cloudflare collab backend. Teraxlyst picks specific features from each and rebuilds them in a single Tauri app.
- A v1 release of anything. The repo currently holds planning docs only.
- Mobile-companion-enabled. iOS and Android are out of scope until at least v2.
- Realtime-collaborative. Single-user in v1. Collab will come later with an MIT-licensed CRDT (Automerge, Diamond Types, or Loro), not Yjs.

## Why fork terax-ai

terax-ai already solved the gnarly desktop-native edges: PTY supervision with portable-pty, cross-platform process cleanup via shared_child + Windows Job Objects, OS keychain abstraction, ripgrep-style file search with ignore + grep-regex, streaming HTTP proxy for AI providers with SSRF defenses. Replatforming any of that costs more than it saves.

## Why borrow from nimbalyst

nimbalyst pioneered patterns we want:

- Two-tier append-only transcript architecture (raw provider payload + canonical events)
- YAML-defined trackers with role-based field semantics and MCP-tool integration
- Visual per-file diff approval as an interaction model for agent changes
- MCP PromptForUserInput as a way for agents to request structured UI input

We rebuild those patterns on top of a native Rust + SQLite stack, not their Electron + PGLite stack.

## Stack

| Layer | Choice |
|-------|--------|
| Shell | Tauri 2 |
| Backend | Rust (rusqlite, tokio, similar, reqwest, portable-pty, keyring) |
| Frontend | React 19 + Tailwind |
| Code editor | Monaco (MIT) |
| Markdown | tiptap or markdown-it |
| Database | SQLite via rusqlite, WAL mode, single-writer actor pattern |
| MCP transport | stdio JSON-RPC, official spec |
| Diff engine | Rust `similar` crate, Monaco diff editor for rendering |

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full rationale.

## Repository status

This `planning/` directory contains:

- [`ARCHITECTURE.md`](./ARCHITECTURE.md) - load-bearing technical decisions with rationale
- [`ROADMAP.md`](./ROADMAP.md) - phased milestones M0-M6, ~16 weeks of part-time work (realistic: 6-8 months)
- [`FEATURE_MAP.md`](./FEATURE_MAP.md) - nimbalyst feature -> Teraxlyst implementation mapping
- [`CRITIQUE.md`](./CRITIQUE.md) - self-review with what's still uncertain
- This README

When implementation starts (M0), code lives under `src-tauri/` (Rust) and `src/` (React) following terax-ai's existing layout.

## License

When code is added: Apache-2.0, inherited from terax-ai. A `NOTICE.md` will list terax-ai as the upstream fork source and nimbalyst as a design inspiration, both with full attribution.

## Attribution

- terax-ai by [crynta](https://github.com/crynta/terax-ai), Apache-2.0, latest tag 0.6.5 (2026-05-15). Forking source.
- nimbalyst by [Nimbalyst Inc.](https://github.com/nimbalyst/nimbalyst), MIT (2024-2026). Design-pattern inspiration only.

Nothing in this repo is taken verbatim from nimbalyst. Patterns are not copyrightable; the credit is given out of professional courtesy. terax-ai code lands in M0 under the Apache-2.0 inheritance, with all original copyright headers preserved and a `NOTICE.md` capturing the fork lineage.

## Not yet

- No binaries. No releases. No CI. No `cargo install`.
- The plan may change as M0/M1 reveal what doesn't survive contact with the code.
