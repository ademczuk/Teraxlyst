# M0 follow-up punch list

Tracks rebrand work that did NOT land in the initial fork commit. Items are deliberate deferrals - low risk to leave, high risk to change blindly.

Status as of 2026-05-15 (after 0.1.0-pre.3):

- Initial fork landed with package/binary names rebranded.
- Self-review pass fixed: LICENSE attribution, CODEOWNERS, governance docs, CI workflows disabled, terax-ai screenshots removed.
- Brand sweep pass landed:
  - Frontend: 30 user-visible string replacements in 14 src/ files. `pnpm build` clean.
  - Backend: 6 safe internal references migrated (PTY thread names, backpressure marker, tmp filename).
  - Icons: Teraxlyst icon generated and applied to 14 PNG sizes plus .ico and .icns.
  - New CI workflow + issue/PR templates written.
  - See `planning/M0_FRONTEND_SWEEP.md` and `planning/M0_BACKEND_SWEEP.md` for detail.
- Remaining work is `pnpm tauri dev` verification (needs Rust installed locally) plus the deferred M0.1 punch list below. Data-migration items in particular block v0.2, not v0.1.

## Critical (block M2 or M3)

None. The app should compile and run with the current state once Rust is installed.

## Frontend rebrand (cosmetic, ~63 references across 30 src/ files)

Run `rg -l '\bTerax\b|\bterax\b' src/` for the full list. Bulk replace candidates:

- Component class names and CSS selectors with `terax-*` prefix
- Toast/notification copy mentioning "Terax"
- AboutSection.tsx and About dialogs
- Default theme name strings

Risk: some references are in localStorage keys (e.g. `terax-ui-theme-shadow`). Renaming those without a migration script wipes user theme/preference state on update. For now, keep them; write a one-shot migration helper in M0.1 that reads `terax-*` keys, writes `teraxlyst-*` keys, deletes the old keys, runs once on first launch of v0.2+.

## Backend internal references (compile-safe, runtime-cosmetic)

In `src-tauri/src/`:

- `lib.rs:19` Tauri event name `"terax:settings-tab"` - paired with a renderer listener; change both or neither
- `modules/fs/file.rs:96` temp filename `.{name}.terax.tmp` - runtime-only, cosmetic
- `modules/pty/shell_init.rs:172,211,298,312,332` cache directory `~/.cache/terax/shell-integration` - changing requires a migration script for existing users; safer to keep until v0.2
- `modules/pty/mod.rs:134` and `modules/pty/session.rs:110,153,177` thread names `terax-pty-*` - debug-only, leave
- `modules/pty/session.rs:23` backpressure marker text - user-visible in terminal output during overflow; rebrand candidate
- `modules/shell/session.rs:118,133` shell variable `__terax_rc` - exit code capture, internal
- `modules/pty/scripts/*.{zsh,bash,fish}` shell integration scripts use `_terax_` function prefix - need a consistent rename or none; all-or-nothing
- `modules/pty/scripts/*.{zsh,bash,fish}` env var `TERAX_USER_ZDOTDIR` - if changed, users' shell init breaks until they restart

## Asset rebrand

- `terax-icon.png` (not copied into the fork; intentionally absent)
- `src-tauri/icons/*.png` and `*.ico` and `*.icns` - still terax-ai's icons; need replacement assets
- `public/` static assets - check for branded screenshots/SVGs

## CI workflow

Files moved to `.github/workflows-pending/` to disable execution against missing secrets. Rewrite plan:

- `ci.yml` is mostly clean (frontend type-check + cargo check). Move back to `.github/workflows/` once we have verified the rebrand compiles locally.
- `release.yml` references Apple signing secrets (`APPLE_API_KEY_PATH`, `APPLE_CERTIFICATE`, `TAURI_SIGNING_PRIVATE_KEY`, etc.) that do not exist in this repo. Rewrite as part of M6 release prep with a Teraxlyst-owned signing keypair.
- Issue and PR templates similarly parked at `.github/templates-pending/`. Rewrite with Teraxlyst-specific labels and a planning-tag option.

## Updater

- Removed from `tauri.conf.json` in the initial fork commit.
- M6 work: generate a minisign keypair, store the private key offline, embed the public key in `tauri.conf.json`, configure the endpoint to `https://github.com/ademczuk/Teraxlyst/releases/latest/download/latest.json`.

## Documentation

- `CONTRIBUTING.md` - inherited from terax-ai, still references terax-ai's processes
- `CODE_OF_CONDUCT.md` - inherited, may reference terax-ai community
- `SECURITY.md` - inherited, references terax-ai's security disclosure process
- `docs/` directory - audit for any terax-ai-specific docs

All three are usable as-is for now. Will rewrite in M6 before 0.1.0 release.

## What stays terax-named forever

- `TERAX_AI_ORIGIN.md` - upstream marketing doc, preserved as historical artifact
- This punch list itself (M0_TODO.md) - intentionally references the old name
