# Changelog

All notable changes to Teraxlyst. Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [SemVer](https://semver.org/) (pre-`1.0`, minor bumps may include breaking changes).

## [0.1.0-pre.3] - 2026-05-15

M0 follow-up pass. Three parallel sub-agents covered the file-level rebrand surface.

### Changed

- **Frontend (14 files, 30 replacements).** User-visible "Terax" strings throughout the React app: dialog titles, placeholders, system prompts, console log tags, terminal hibernation overflow text, OpenRouter `X-Title` header, repo URLs, updater API endpoint, AboutSection bundle ID + display fields. Detail in `planning/M0_FRONTEND_SWEEP.md`. Build verified: `pnpm exec tsc --noEmit` clean, `pnpm build` succeeds in 10.6s.
- **Backend (4 files, 6 replacements).** Safe Rust internal references: PTY thread names (`teraxlyst-pty-reader/flusher/waiter/drop-{id}`), backpressure marker bytes printed to terminal on overflow, and the atomic-write tmp filename suffix in `modules/fs/file.rs`. Detail in `planning/M0_BACKEND_SWEEP.md`.
- **Icon set.** Generated a Teraxlyst icon via Gemini Nano Banana Pro (1024x1024 source preserved as `src-tauri/icons/teraxlyst-source.png`). Resized into the 14 PNG sizes Tauri expects (32x32, 64x64, 128x128, 128x128@2x, icon.png, and 9 Square*Logo / StoreLogo files for Windows Store builds). Generated `icon.ico` (6 resolutions) and `icon.icns` via Pillow. `public/logo.png` also replaced. Android and iOS subdirectories left for M0.1 since mobile is out of v1 scope.
- **CI workflow.** New `.github/workflows/ci.yml`: minimal frontend type-check + build on push and pull_request to main. No Rust, no signing, no cross-platform builds yet.
- **Issue and PR templates.** Rewrote `bug_report.yml`, `feature_request.yml`, `config.yml`, and `PULL_REQUEST_TEMPLATE.md` for Teraxlyst. Replaces the parked terax-ai versions.

### Verified

- `pnpm install --frozen-lockfile`: succeeded in 35s with current `pnpm-lock.yaml`.
- `pnpm exec tsc --noEmit`: clean.
- `pnpm build`: clean, 10.6s, full bundle output.
- AI-marker scan on all 19 authored docs and template files: zero hits.

### Deferred to M0.1 (deliberate, with reasons in the sweep reports)

- **33 remaining `terax` occurrences in `src/`** across 20 files: localStorage keys (`terax-ui-theme-shadow`), tauri-plugin-store filenames (`terax-settings.json`, `terax-ai-{agents,sessions,snippets,todos}.json`), OS keychain service name (`KEYRING_SERVICE = "terax-ai"`), 7 Tauri event names paired with backend, CSS class `terax-collapsible-*`, and the `<terax-command>` wire format. All have either data-migration or paired-rename concerns. v0.2 needs a one-shot migration helper.
- **81 remaining `terax` occurrences in `src-tauri/src/`**: shell-integration scripts and their env-var emit paths (`TERAX_USER_ZDOTDIR`, `TERAX_TERMINAL`, `_terax_precmd`, etc.), shell cache path `~/.cache/terax/shell-integration`, and the Tauri event names paired with frontend. All all-or-nothing changes that need lockstep coordination.
- **Linux package install commands in `UpdaterDialog.tsx`** still reference `terax-bin` AUR, `Terax_*.deb`, `Terax-*.rpm`. Users on Linux who hit the manual-update path will see broken install commands until packaging exists.
- **`https://terax.app` website URL** in `AboutSection.tsx` and OpenRouter `HTTP-Referer` in `agent.ts` still point upstream. Need a fork domain decision.
- **Mobile icon assets** in `src-tauri/icons/android/` and `src-tauri/icons/ios/` untouched (mobile out of v1 scope).
- **macOS notarization, Windows code signing, .icns refinement via `iconutil`**: M6.

## [0.1.0-pre.2] - 2026-05-15

Hardening pass after self-review.

### Fixed

- **License compliance.** `LICENSE` now preserves the original "Copyright 2026 Crynta" line per Apache-2.0 §4(c) and adds "Copyright 2026 ademczuk" for Teraxlyst modifications. The prior commit stripped the upstream attribution, which would have been a license violation if released.
- `CODEOWNERS` no longer routes review requests to @crynta; set to @ademczuk.

### Changed

- Inherited GitHub Actions workflows moved from `.github/workflows/` to `.github/workflows-pending/` to prevent them firing against secrets and signing keys that do not exist in this repo. Will be rewritten and re-enabled in M0.1 or M6.
- Inherited issue and PR templates moved to `.github/templates-pending/` for the same reason. Will be rewritten before re-enabling.
- `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` replaced with minimal placeholders. Original terax-ai versions referenced a `security@terax.app` address we do not own and contained 22 em-dashes plus dozens of "Terax" references. Cleaner to rewrite than to scrub.
- Removed inherited terax-ai UI screenshots from `docs/`. They depicted terax-ai's interface, not Teraxlyst's.

### Updated

- `README.md` status table: M0 corrected from "not started" to "partial (file-level fork done, build not yet verified)".

## [0.1.0-pre] - 2026-05-15

Initial fork from terax-ai v0.6.5 (released 2026-05-15, https://github.com/crynta/terax-ai/releases/tag/v0.6.5).

### Added

- Planning docs under `planning/`: ARCHITECTURE, ROADMAP, FEATURE_MAP, CRITIQUE, README.
- NOTICE.md recording fork lineage and design inspiration.

### Changed

- Rebranded from Terax to Teraxlyst.
  - `package.json`: name `terax` -> `teraxlyst`; version reset to `0.1.0-pre`.
  - `src-tauri/Cargo.toml`: package name `terax` -> `teraxlyst`; lib name `terax_lib` -> `teraxlyst_lib`; description, authors, repository updated.
  - `src-tauri/tauri.conf.json`: productName, identifier (`com.ademczuk.teraxlyst`), window titles, descriptions.
  - `index.html` and `settings.html` titles.
  - `src-tauri/src/main.rs` lib reference.
- Removed terax-ai's auto-updater config (pubkey and endpoint). Will be reintroduced with a Teraxlyst-owned keypair in M6.

### Preserved (upstream attribution intact)

- All terax-ai source code under `src/` and `src-tauri/src/` is unchanged from v0.6.5 except for the `teraxlyst_lib::run()` call site in `main.rs`. Original copyright headers preserved per Apache-2.0.
- terax-ai's marketing doc preserved as `TERAX_AI_ORIGIN.md`.

### Known follow-up work in M0

See `planning/ROADMAP.md` and the new `planning/M0_TODO.md` for the punch list. High-level:
- 63 remaining `Terax`/`terax` references across 30 frontend source files.
- Tauri event names use a `terax:` namespace - decide whether to migrate or keep.
- localStorage keys use a `terax-` prefix - migration story TBD.
- Icons still use terax-ai's set; replacement pending.
- CI workflows under `.github/` reference terax-ai paths.

### Upstream history

Versions 0.0.2 through 0.6.5 are part of terax-ai's history. See the upstream changelog at https://github.com/crynta/terax-ai/blob/v0.6.5/CHANGELOG.md.
