# Cargo.lock Flow

## Why we commit Cargo.lock

Teraxlyst ships a binary (the Tauri desktop app), not a library. Per the
Cargo book, binary projects should commit `Cargo.lock` so every build,
on every machine and in CI, resolves to the exact same dependency
graph. This is what makes a build reproducible and what lets us catch
"works on my machine" supply-chain drift early.

The lockfile lives at `src-tauri/Cargo.lock` and is tracked in git.

## How to refresh it

The user's workstation does not have a local Rust toolchain, so the
lockfile is regenerated in CI rather than locally.

1. Go to the Actions tab on GitHub.
2. Select the "Refresh Cargo.lock" workflow.
3. Click "Run workflow" on the main branch.

The workflow runs `cargo generate-lockfile` inside `src-tauri/`, then
commits the refreshed `Cargo.lock` straight back to main with the
message `chore(deps): refresh src-tauri/Cargo.lock via workflow_dispatch`.
If the lockfile is already current it logs a skip and exits cleanly.

Trigger it any time a `Cargo.toml` change adds, removes, or pins a
dependency.

## CI uses --locked

The main CI workflow (`ci.yml`) runs `cargo check`, `cargo clippy`,
and `cargo test` with `--locked`. If a PR changes `Cargo.toml` but
the lockfile is stale, CI will fail with a lockfile-out-of-date error.
Fix: merge or trigger the refresh workflow, then re-run CI.
