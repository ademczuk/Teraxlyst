# Contributing

Teraxlyst is in early planning. No working code yet beyond the M0 fork from terax-ai. The most useful contributions right now are feedback on the plan in `planning/`.

## Feedback on the plan

- Read `planning/ARCHITECTURE.md`, `planning/ROADMAP.md`, and `planning/CRITIQUE.md`.
- File an issue tagged `planning` with what you would change or what's missing.
- Architecture decisions are not locked in; the docs note where I'm uncertain.

## Code contributions

Hold for now. M0 still has the punch list in `planning/M0_TODO.md` to finish, M1 (rusqlite + DB actor) has not started, and the toolchain assumptions in `planning/ARCHITECTURE.md §2` and `§5` have not been validated against a running build yet.

Once M0 lands cleanly and there is a working `pnpm tauri dev` on at least one platform, this file will be rewritten with a normal contributor workflow (issue first, branch off main, PR with tests).

## Local development

```bash
git clone https://github.com/ademczuk/Teraxlyst
cd Teraxlyst
pnpm install
pnpm tauri dev
```

This is inherited from terax-ai v0.6.5 and has not been re-verified after the rebrand. If it does not work, please file an issue.

## Code style

When code starts landing: Rust uses `cargo fmt` and `cargo clippy --all-targets -- -D warnings`. TypeScript uses the project's existing prettier + eslint config inherited from terax-ai (also not re-verified after the rebrand).
