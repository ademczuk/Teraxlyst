# Changelog

All notable changes to Teraxlyst. Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [SemVer](https://semver.org/) (pre-`1.0`, minor bumps may include breaking changes).

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
