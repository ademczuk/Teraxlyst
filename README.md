# Teraxlyst

[![CI](https://github.com/ademczuk/Teraxlyst/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ademczuk/Teraxlyst/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
![Version](https://img.shields.io/badge/version-0.1.0--rc4-orange)
![Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24C8DB)
![Rust](https://img.shields.io/badge/Rust-stable-orange)

A Rust + Tauri 2 reference architecture for AI-coding-agent desktop tools. Ships a $0/year release pipeline with Sigstore provenance, an in-process MCP host, a visual diff-approval pipeline, and a YAML tracker system.

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Teraxlyst icon" width="128" height="128">
</p>

## Honest positioning (read this first)

Teraxlyst is **not a finished product** and **not a replacement for [nimbalyst](https://github.com/nimbalyst/nimbalyst)**. After a 4-model audit (Pantheon, Hermes-Agentic, Titan-Agentic, Google Deep Research), the consensus on "is this fork worthwhile as a product" was **no** - nimbalyst ships rich-text editing, Excalidraw, Mermaid, an iOS companion, and real-time collab that Teraxlyst does not, and the moving-target gap is months of solo work.

What Teraxlyst **does** ship as a reference for other Tauri 2 projects:

- A `$0/year` release pipeline: matrix build (macOS aarch64+x86_64, ubuntu-22.04, windows-latest), Sigstore OIDC build provenance, SHA256SUMS-per-platform, minisign updater signing. No paid CA certs.
- A `rusqlite` single-writer actor pattern for SQLite persistence in a Tauri app.
- A `tokio::sync::mpsc` + `oneshot` correlation-id MCP host built on the official `rmcp 1.7` SDK, with structured `PromptForUserInput` widgets.
- A `claude --print --output-format stream-json --verbose` parser verified against captured Claude CLI 2.1.118 output, with three-state envelope classification (recognized-and-emit / recognized-and-skip / fall-through-to-text).
- A pre-commit hook + CI guard that scans for AI typographic markers (em-dashes, curly quotes, banned phrases) on every push.
- 21 -> 0 npm prod CVEs and 0 Rust CVEs as of `v0.1.0-rc4`, with `pnpm audit --prod` + `cargo audit` as CI guards.

If you want a polished tool to use today: use nimbalyst. If you want a reference codebase showing how to ship a Rust Tauri app with $0 in cert fees and verifiable supply-chain provenance: read this one.

## What's in this repository

- [`planning/README.md`](./planning/README.md) - what Teraxlyst is, what it isn't
- [`planning/ARCHITECTURE.md`](./planning/ARCHITECTURE.md) - stack decisions, data model, MCP host design
- [`planning/ROADMAP.md`](./planning/ROADMAP.md) - phased milestones M0-M6
- [`planning/SIGNING_PLAN.md`](./planning/SIGNING_PLAN.md) - the $0-cert release path, with paid-cert fallback documented
- [`docs/RELEASE.md`](./docs/RELEASE.md) - operator-facing release procedure
- [`packaging/README.md`](./packaging/README.md) - Scoop + Homebrew manifests for community distribution

## Status by milestone (as of v0.1.0-rc4, 2026-05-16)

| Milestone | Status | What that means |
|-----------|--------|-----------------|
| M0 fork + rebrand | shipped | code lives in this repo with full upstream attribution |
| M1 persistence | shipped | DbActor + 6 tables + 8 commands + integration tests, all green |
| M2 multi-session manager | shipped | SessionManager + real Claude CLI stream-json parser + 17 parser tests |
| M3 MCP host | shipped | rmcp 1.7 tool_router with 3 tools + PromptForUserInputDialog mounted |
| M4 trackers | shipped | YAML loader + 5 tracker commands + TrackersPanel in sidebar |
| M5 diff approval | shipped | DiffInbox + Monaco diff viewer + apply-and-resolve |
| M6 release pipeline | shipped | unsigned releases with Sigstore provenance, matrix build green on all 4 platforms |
| User-facing v0.1.0 stable | not shipped | rc4 in draft. macOS + Linux live-install smoke tests pending. No onboarding wizard. |

What's verified:
- 38 Rust tests + 9 frontend tests pass on Windows
- `cargo clippy --all-targets --locked -- -D warnings` clean
- `cargo audit` clean (0 CVEs across 636 deps)
- `pnpm audit --prod` clean (0 advisories)
- Windows: rc4 NSIS installer smoke-tested end-to-end (install + launch + uninstall)
- Sigstore attestation verified on all 4 platforms' bundles against rc4 commit
- Pre-commit hook + CI guards prevent future regression

What's not verified:
- macOS aarch64 + x86_64 live install (no macOS hardware in operator environment)
- Linux live GUI launch (WSL has no passwordless sudo; structural validation only)
- Onboarding flow (no first-run wizard)
- Real-world session against Claude CLI (parser tests pin envelope shape but integration test is cfg(unix) due to a Windows webview2-com-sys quirk)

## High-level design

Teraxlyst takes the Rust + Tauri shell from [terax-ai](https://github.com/crynta/terax-ai) (Apache-2.0, fork base v0.6.5) and rebuilds a focused subset of [nimbalyst](https://github.com/nimbalyst/nimbalyst)'s (MIT) workspace features on top of it:

- Multi-AI-session manager with live transcript streaming
- Visual red/green diff approval for agent file changes
- YAML-defined custom tracker system (Plans, Decisions, Bugs, Tasks, Ideas, Features, Automations)
- In-process MCP server (via official `rmcp` SDK) with structured `PromptForUserInput` widgets

Single binary, native systems integration, single-user in v1.

What is **not** ported and is not on the roadmap:
- Lexical WYSIWYG editor (use Monaco / CodeMirror instead)
- Excalidraw canvas
- Mermaid live editor
- iOS / Android companion apps
- Real-time collab server

If you need any of these, use nimbalyst.

## Why fork instead of contribute upstream

Mostly to test "can a `$0/year` release pipeline for a Rust Tauri app reach production posture?" The answer turned out to be yes (rc4 shipped with Sigstore provenance + minisign updater + verified cross-platform builds for under one session of focused work). The novel MCP-host scaffold and YAML-tracker work could in principle land as nimbalyst PRs - that path is recommended for anyone who wants the features in a polished tool today.

## Licensing

- This repository: **Apache-2.0** (preserves terax-ai upstream attribution, see [`NOTICE.md`](./NOTICE.md)).
- terax-ai: Apache-2.0, full attribution preserved.
- nimbalyst: MIT, design-pattern inspiration only, no code reuse.

## Example trackers

Pre-built tracker YAMLs ship under [`examples/trackers/`](./examples/trackers/). Copy any of them to your workspace at `.teraxlyst/trackers/` to enable:

- `bugs.yaml` - bug reports with severity, status, tags
- `tasks.yaml` - generic tasks with priority and dependencies
- `decisions.yaml` - ADR-style architecture decisions
- `plans.yaml` - multi-phase initiatives with progress and milestones

See [`examples/trackers/README.md`](./examples/trackers/README.md) for the schema cheatsheet.

## Install

Pre-built binaries are in draft at [the v0.1.0-rc4 release](https://github.com/ademczuk/Teraxlyst/releases). Verify before installing:

```bash
# Verify SHA256
sha256sum -c SHA256SUMS-ubuntu-22.04.txt

# Verify the binary came from this repo's CI
gh attestation verify --owner ademczuk teraxlyst_0.1.0-rc4_amd64.deb
```

See [`INSTALL.md`](./INSTALL.md) for per-platform unsigned-binary bypass instructions (SmartScreen on Windows, Gatekeeper on macOS).

## Contributing

Issues and PRs welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md) for the contributor workflow. Feedback on the plan and the positioning goes in issues tagged `planning`.
