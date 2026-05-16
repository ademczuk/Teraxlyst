# Contributing to Teraxlyst

Thanks for the interest. M0-M6 scaffolds have landed and CI is green on Linux. The project is past planning-only; real PRs are welcome.

## Where to start

| If you want to | Then |
|---|---|
| Report a bug | Open an issue. Use the bug template. |
| Suggest a feature | Open an issue tagged `planning`. Check `planning/ROADMAP.md` first. |
| Improve the docs | PR straight to main is fine for typos; planning doc rewrites should reference an issue. |
| Add a tracker example | PR to `examples/trackers/`. Keep YAMLs ASCII-only. |
| Fix a bug | Comment on the issue, then PR. |
| Build a real feature | Open a planning issue first to align on scope, then PR. |

## Prerequisites

- Node 22+, pnpm 10 (https://pnpm.io/installation).
- Rust stable via rustup (https://rustup.rs/).
- Platform build tools per `INSTALL.md`.
- A real machine. Teraxlyst is a desktop app; everything visual needs you to run it locally.

## Local dev loop

```bash
git clone https://github.com/ademczuk/Teraxlyst
cd Teraxlyst
git config core.hooksPath .githooks    # one-time, enables pre-commit
pnpm install
pnpm tauri dev                          # opens the app in dev mode
```

In another shell, while the app runs:

```bash
# Frontend type-check
pnpm exec tsc --noEmit

# Rust check (clippy + tests)
cd src-tauri
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --lib --locked
```

CI runs exactly those commands. If they pass locally, they will pass in CI (modulo the rare platform-specific issue).

## PR workflow

1. Branch off `main`. Branch name suggestion: `<area>/<short-desc>` e.g. `m4/inline-tracker-refs`.
2. Write tests. The Rust integration test pattern is in `src-tauri/src/db/tests.rs` and `src-tauri/src/sessions/tests.rs`. Frontend tests are not yet wired (vitest setup is M0.x work).
3. Make the change.
4. Run the four commands above locally. Get them all green.
5. Push the branch, open a PR. The PR template lists the verification checklist.
6. CI runs frontend type-check + build + Rust check + clippy + tests + the M2 + M3 integration tests. All must be green.
7. Squash-merge when approved. Use the conventional-commit format from the PR template for the squash commit message.

## Style

- Rust: `cargo fmt` and `cargo clippy --all-targets -- -D warnings`. The CI clippy step rejects warnings.
- TypeScript: the existing prettier + eslint config (inherited from terax-ai). `pnpm exec tsc --noEmit` must be clean.
- Markdown and YAML: ASCII only. The `.githooks/pre-commit` hook blocks em-dashes, curly quotes, ellipsis, arrows, and a handful of AI vocabulary words. Bypass with `--no-verify` only for verbatim upstream content.

## Architecture conventions

- DB writes go through `DbHandle`. Never construct a `rusqlite::Connection` outside `db::actor`.
- New Tauri commands map errors to `String` at the IPC boundary. Use the existing `DbError`/`ManagerError`/`McpError`/`TrackerError` shape with `thiserror` + a custom `Serialize` impl.
- Frontend state: prefer Jotai/Zustand stores already in `src/modules/`, not new globals.
- Don't add a new top-level dep without an issue discussing the trade-off.
- No `unwrap()` in non-test code. Use `?` and proper error propagation.

## Tracker definitions

The four `examples/trackers/*.yaml` files are reference schemas. If you change the schema validator in `src-tauri/src/trackers/schema.rs`, update the examples to demonstrate any new field type or role.

## Releases

Not yet. The path to a tagged `v0.1.0` is in `planning/SIGNING_PLAN.md`. Until those external prerequisites are met, the project ships as source-only.

## Filing security issues

See `SECURITY.md`. Until 0.1.0 a regular issue is fine; mark it confidential if it carries exploit detail.

## Questions

Open an issue. The maintainer is @ademczuk.
