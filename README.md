# Teraxlyst

A desktop workspace for AI coding agents. Tauri 2 + Rust + React.

**Status: planning only.** No working code yet. The first code lands in milestone M0 (see roadmap).

## What's in this repository right now

Only planning documents. They live in [`planning/`](./planning/):

- [`planning/README.md`](./planning/README.md) - what Teraxlyst is, what it isn't
- [`planning/ARCHITECTURE.md`](./planning/ARCHITECTURE.md) - stack decisions, data model, MCP host design
- [`planning/ROADMAP.md`](./planning/ROADMAP.md) - phased milestones M0-M6
- [`planning/FEATURE_MAP.md`](./planning/FEATURE_MAP.md) - nimbalyst feature -> Teraxlyst implementation table
- [`planning/CRITIQUE.md`](./planning/CRITIQUE.md) - self-review of the plan, open questions

## High-level design

Teraxlyst takes the Rust-and-Tauri shell from [terax-ai](https://github.com/crynta/terax-ai) (Apache-2.0, fork base v0.6.5) and rebuilds a focused subset of [nimbalyst](https://github.com/nimbalyst/nimbalyst)'s (MIT) workspace features on top of it:

- Multi-AI-session manager with live transcript streaming
- Visual red/green diff approval for agent file changes
- YAML-defined custom tracker system (Plans, Decisions, Bugs, Tasks, Ideas, Features, Automations)
- In-process MCP server (via official `rmcp` SDK) with structured PromptForUserInput widgets

Single binary, native systems integration, single-user in v1.

## Why not just use one of the upstream projects

- **terax-ai** is a terminal emulator with AI chat. Strong shell, light on workspace features.
- **nimbalyst** is an Electron + PGLite + Cloudflare stack with rich features and a collab server. Heavy, AGPL-licensed collab dependency.

Teraxlyst targets users who want nimbalyst's session-management and tracker workflow on top of a lighter, fully-permissive Rust stack.

## Licensing

- This repository: **Apache-2.0** (inherited from terax-ai upstream when the M0 fork lands).
- terax-ai: Apache-2.0, full attribution in [`NOTICE.md`](./NOTICE.md).
- nimbalyst: MIT, design-pattern inspiration only, no code reuse.

## Status by milestone

| Milestone | Status |
|-----------|--------|
| Planning docs | done |
| M0 fork + rebrand | done |
| M1 persistence | done (DbActor + 6 tables + 8 commands + 4 tests, all green in CI) |
| M2 multi-session manager | done (SessionManager + Claude Code subprocess + streaming + simple list view + 2 tests) |
| M3 MCP host | scaffold (rmcp 1.7 + 3 tools + PromptForUserInput pipeline; lib.rs wireup pending M3.1) |
| M4 trackers | scaffold (YAML schema + validation + 4 MCP tools + DbActor variants; lib.rs wireup pending M4.1) |
| M5 diff approval | scaffold (Monaco diff editor + diff_proposals v2 migration; lib.rs wireup pending M5.1) |
| M6 0.1.0 release | docs (INSTALL.md + SIGNING_PLAN.md). Signing certs + keypair generation needed before tagged release |

See [`planning/ROADMAP.md`](./planning/ROADMAP.md) for details.

## Contributing

Issues and PRs are welcome once M0 has landed. Right now there's nothing to build against.

If you have feedback on the plan itself, open an issue tagged `planning`.
