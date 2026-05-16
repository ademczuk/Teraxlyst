# Changelog

All notable changes to Teraxlyst. Format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [SemVer](https://semver.org/) (pre-`1.0`, minor bumps may include breaking changes).

## [unreleased] - 2026-05-16

Post-rc3 audit pass. Will land in rc4 on the next tag.

### Fixed (security: 21 prod advisories -> 0)

- `package.json`: moved `shadcn` from `dependencies` to
  `devDependencies`. shadcn is a build-time CLI that scaffolds UI
  components into `src/components/ui/*.tsx`; nothing it ships is
  loaded at runtime. Misclassification was inherited from upstream
  terax-ai. Cleared 8 advisories along the
  `shadcn>@modelcontextprotocol/sdk>{ajv>fast-uri,hono,
  express-rate-limit>ip-address}` chain, including both `high`
  fast-uri items (path traversal + host confusion via
  percent-encoded URIs).
- `package.json` pnpm.overrides: pinned `dompurify` to `^3.4.0`.
  monaco-editor still bundles a pre-3.4 dompurify transitively;
  the override forces the patched version across the dep graph.
  Cleared 8 moderate dompurify advisories (XSS via mutation,
  ADD_ATTR predicate skip, USE_PROFILES prototype pollution,
  SAFE_FOR_TEMPLATES bypass, FORBID_TAGS bypass, etc). dompurify
  is reachable from the M5 diff viewer (Monaco renders proposed
  diffs).
- `pnpm up @monaco-editor/react@latest streamdown@latest` to pick
  up the latest published versions matching the override.

### Fixed (build hygiene)

- `.github/workflows/release.yml`: SHA256SUMS files now list
  user-facing installers only (`.deb` / `.rpm` / `.AppImage` /
  `.dmg` / `.app.tar.gz` / `.exe`). rc3 manifests on the macOS
  side contained tauri-bundler internal scaffolding
  (`share/create-dmg/support/template.applescript`,
  `dmg/bundle_dmg.sh`, the unpacked `.app/` tree) that users have
  no way to verify against the artifacts they download.
- 6 dead-code warnings cleared on the Windows test build by gating
  the cfg(unix)-only imports + the create_with_program test seam
  to mirror their consumers. `cargo test --lib --locked` now
  emits zero warnings (was 6).

### Verified

- 21 prod advisories -> 0 via `pnpm audit --prod`.
- 9/9 frontend vitest tests pass.
- `pnpm exec tsc --noEmit`: clean.
- `pnpm build`: clean, 8.84s, full dist output.
- 38/38 Rust tests pass with `cargo test --lib --locked`.
- `cargo clippy --all-targets --locked -- -D warnings`: clean.

## [0.1.0-rc3] - 2026-05-16

First multi-platform release candidate. M2.2 (real Claude CLI parser),
Windows boot, and the no-paid-cert release pipeline all landed in one
session. Draft release with 18 assets across macOS aarch64 + x86_64,
Linux x64, Windows x64. Smoke-tested locally on Windows; Sigstore
build provenance attestation verified end-to-end.

### Added (M2.2 - real Claude CLI parser)

- `src-tauri/src/sessions/provider_claude_code.rs`: rewrote
  `parse_line` against captured Claude CLI 2.1.118 `stream-json`
  output. Three-state `classify_json_event` cleanly distinguishes
  recognized-and-emit, recognized-and-skip (hook lifecycle), and
  parse-failed-fall-through-to-text. Envelope coverage:
  - `system + subtype=hook_started|hook_response`: skipped (CLI plumbing)
  - `system + subtype=init`: `SystemNotice` with model + cwd
  - `assistant + content[].type=text`: `AssistantText`
  - `assistant + content[].type=tool_use`: `ToolCall { name, args }`
  - `user + content[].type=tool_result`: `ToolResult { name, payload }`
  - `rate_limit_event`: `SystemNotice`
  - `result`: `Completed`
  - `error`: `Error`
  - unknown type: falls through to text
- `spawn_claude_code` default invocation now uses
  `claude --print --output-format stream-json --verbose <prompt>`.
- 11 new unit tests against captured real-CLI samples (total parser
  tests: 17, all green on Windows). Full suite: 38 pass, 0 fail.

### Added (M0.x - Windows boot fixes)

- `tauri.conf.json`: added `plugins.updater` config block with the
  embedded minisign public key + GitHub releases endpoint. Previously
  the missing config block panicked the app during plugin init.
- `db/actor.rs`: switched `spawn_with_connection` from
  `tokio::task::spawn_blocking` to `std::thread::Builder::new().spawn()`.
  Tauri's setup hook runs before the runtime is up, so the tokio call
  panicked with "no reactor running". The actor body is fully
  synchronous (rusqlite is blocking and `mpsc::Receiver::blocking_recv`
  does not require a tokio context), so a plain OS thread works in
  both Tauri-managed and `#[tokio::test]` paths.
- End-to-end verified: `pnpm tauri dev` on Windows now boots; vite
  serves, teraxlyst.exe runs, MCP toolset registers 3 tools, PTY
  spawns pwsh.exe, WebView2 renders, app survives a relaunch.

### Added (M6.1 - no-paid-cert release pipeline)

- `.github/workflows/release.yml`: multi-platform matrix on `v*` tag
  push (macOS aarch64+x86_64, ubuntu-22.04, windows-latest). Per-job
  SHA256SUMS upload + GitHub Actions OIDC build provenance attestation
  via `actions/attest-build-provenance@v2`. Sigstore-backed chain of
  custody from commit to binary, no CA involved.
- Tauri minisign updater keypair generated locally; public key
  embedded in `tauri.conf.json`. Private key + (empty) password
  stored as repo secrets `TAURI_SIGNING_PRIVATE_KEY` and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Auto-update path
  cryptographically guarded without paying a CA.
- `bundle.targets` switched from `"all"` to an explicit array
  excluding MSI (the MSI bundler rejects string pre-release tags like
  `rc3`; NSIS handles full semver). Windows ships NSIS `.exe`
  installer only.
- `planning/SIGNING_PLAN.md` rewritten: $0/year path documented as
  the active plan, the $179/year paid-cert path parked with secrets
  still named for an easy flip later.
- `docs/RELEASE.md`: operator-facing rc -> publish -> hotfix ->
  rollback procedure. Lists the one-time minisign setup steps.
- `packaging/scoop/teraxlyst.json` + `packaging/homebrew/teraxlyst.rb`:
  community-driven distribution templates (Scoop, Homebrew Cask).
  Both use the unsigned release artifacts directly; the Cask formula
  strips the macOS quarantine attribute on install. Documented in
  `packaging/README.md`.
- `INSTALL.md`: added `gh attestation verify --owner ademczuk <file>`
  invocation so users can prove any release binary came from this
  repo's CI run.

### Fixed (release pipeline iteration)

- rc1 -> rc2: `@tauri-apps/api` bumped from 2.10.1 to ^2.11.0 to
  match the `tauri` crate at 2.11.1. `tauri-action`'s `pnpm tauri
  build` strict-checks the npm/crate match; local `tauri dev` only
  warned, so the mismatch didn't surface until CI.
- rc2 -> rc3: dropped MSI from Windows targets after the bundler
  rejected the `rc2` pre-release identifier with "must be numeric-only
  and cannot be greater than 65535 for msi target". Knock-on updates
  to the Scoop manifest, packaging README, release.yml attestation
  step, and SIGNING_PLAN asset table.

### Verified

- All 38 Rust unit tests pass on Windows (up from 27 in pre.9).
- 9 frontend vitest tests pass.
- rc3 release workflow: 4/4 matrix jobs green
  (https://github.com/ademczuk/Teraxlyst/actions/runs/25957841865).
- Draft release `Teraxlyst v0.1.0-rc3` populated with 18 assets.
- Windows NSIS installer smoke-tested locally: silent install via
  `/S` flag, exit 0, app installed at `%LOCALAPPDATA%\Teraxlyst\`
  with ProductVersion `0.1.0-rc3`. Installed exe launched, allocated
  41 MB RSS, survived 8s sample.
- SHA256 manifest verified: NSIS bundle hash matches
  `SHA256SUMS-windows.txt`.
- Sigstore attestation verified end-to-end via
  `gh attestation verify --owner ademczuk`. Chain proves:
  sourceRepositoryURI = github.com/ademczuk/Teraxlyst,
  sourceRepositoryRef = refs/tags/v0.1.0-rc3,
  buildSignerURI = .github/workflows/release.yml@refs/tags/v0.1.0-rc3,
  runnerEnvironment = github-hosted,
  predicateType = https://slsa.dev/provenance/v1.

### Known gaps before promoting to v0.1.0

- **macOS bundles**: not smoke-tested. No macOS hardware in the
  operator environment.
- **Linux bundle**: structurally verified in WSL Ubuntu via
  `dpkg -I` and `dpkg --contents`. Package metadata clean
  (version 0.1.0-rc3, amd64, Depends on libwebkit2gtk-4.1-0 +
  libgtk-3-0). ELF binary correctly built (x86-64, stripped,
  dynamically linked, glibc 3.2.0+). Non-bundled libs resolve
  cleanly via ldd; only the two declared-as-deps libs
  (libwebkit2gtk-4.1, libsoup-3.0, libjavascriptcoregtk-4.1)
  are unresolved on the bare WSL image, which is the expected
  state until `apt-get install ./teraxlyst.deb` pulls them.
  Live GUI launch in WSL not attempted (needs sudo to install
  the webkit2gtk-4.1 chain).
- **Windows bundle**: smoke-tested end-to-end (see Verified
  section above). NSIS installer + binary + WebView2 render +
  uninstaller all green.
- `real_claude_code_smoke` integration test still `cfg(unix)`-gated
  on Windows. Probed: WebView2Loader.dll copy into
  target/debug/deps/ is necessary but not sufficient; the
  STATUS_ENTRYPOINT_NOT_FOUND failure lives deeper in the
  wry/webview2-com chain. The JSON parser is verified by 11
  unit tests against captured Windows stream-json samples, so
  this integration test is a nice-to-have rather than a gating
  concern.
- `planning/M_DOT_ONE_WIREUP.md` step 7 updated to reflect M2.2
  shipped, but the rest of the M*.1 doc still references
  M2.2-ready scaffolding rather than M2.2 SHIPPED.

### Cost summary

| Item | This session | Annual |
|---|---|---|
| Minisign keypair generation | $0 | $0 |
| GitHub Actions CI minutes | within free tier | $0 |
| Apple Developer Program | skipped | $0 |
| Windows code-signing cert | skipped | $0 |
| Sigstore provenance | built into Actions | $0 |
| **Total** | **$0** | **$0** |

## [0.1.0-pre.9] - 2026-05-16

Polish pass. Cargo.lock now tracked + CI runs --locked. Example
tracker YAMLs shipped. README badges added.

### Added

- `examples/trackers/`: bugs.yaml, tasks.yaml, decisions.yaml,
  plans.yaml + a README explaining the field type cheatsheet, roles,
  ID formats, and MCP-tool usage. Users can copy any of these to
  their workspace at `.teraxlyst/trackers/` and the M4 tracker UI
  picks them up.
- `.github/workflows/refresh-cargo-lock.yml`: workflow_dispatch-only
  job that runs `cargo generate-lockfile` in src-tauri/ and commits
  the refreshed Cargo.lock back to main via
  stefanzweifel/git-auto-commit-action@v5.
- `planning/CARGO_LOCK_FLOW.md`: why we commit Cargo.lock, how to
  refresh, why CI now fails on stale lockfile.

### Changed

- `.github/workflows/ci.yml`: --locked added to cargo check, clippy,
  and test. Lockfile drift is now a CI failure.
- `README.md`: CI badge, Apache-2.0 license badge, version badge,
  Tauri 2 + Rust badges. Icon shown at the top. Example trackers
  section added.

### Verified

- Refresh workflow ran successfully (manual workflow_dispatch).
- Cargo.lock regenerated with M1+ deps (rusqlite, tokio, thiserror,
  ulid, rmcp, similar, serde_yaml_ng, schemars).
- CI with --locked + -D warnings passes: 28 tests, 0 failed, 1
  ignored. Rust check 5m45s, frontend 40s.

### Operator notes

- Cargo.lock now drives deterministic builds across machines. Any
  PR that edits Cargo.toml without refreshing the lock will fail
  CI; manual fix is to trigger refresh-cargo-lock.yml from the
  Actions UI.
- The auto-commit-action push from the refresh workflow does NOT
  trigger downstream CI (GitHub's loop-prevention default). After
  a lockfile refresh, push an empty commit or any other change to
  retrigger CI on the new lockfile.

## [0.1.0-pre.8] - 2026-05-16

clippy `-D warnings` re-enabled. M3.2 rmcp tool router macro wired.
M4.2 + M5.2 sidebar tab layout shipped. 28 tests pass.

### Added (M3.2 - rmcp tool router)

- Second `impl TeraxlystToolset` block in `src-tauri/src/mcp/tools.rs`
  carries `#[tool_router]` with three `#[tool]` async adapter methods
  (`prompt_for_user_input`, `propose_diff`, `read_workspace_file`).
  Each takes `Parameters<ArgsStruct>`, delegates to the existing
  do_* method, and wraps the result into
  `CallToolResult::success(...)`. Errors map through a helper that
  picks the right `ErrorData` constructor per `McpError` variant.
- `TeraxlystToolset` gained a `tool_router: ToolRouter<TeraxlystToolset>`
  field populated in `new()` via `Self::tool_router()`. The router is
  read at startup by `server::spawn_in_process` (tool-count log) which
  makes every Args struct + adapter reachable for clippy.
- `schemars::JsonSchema` derives added to `PromptForUserInputArgs`,
  `ProposeDiffArgs`, `ReadWorkspaceFileArgs`, and the nested
  `PromptField`.
- Removed module-wide `#![allow(dead_code)]` from `mcp/mod.rs`,
  `diff/mod.rs`, `trackers/mod.rs`.
- Targeted `#[allow(dead_code)]` retained on items genuinely reserved
  for near-term work (DbError::Io, McpError::Serde, McpHandle field
  bundle, DbHandle::list_trackers, trackers::mcp_wrappers).

### Added (M4.2 + M5.2 - sidebar tab layout)

- `src/app/SidebarTabs.tsx`: 40px vertical icon strip with four tabs
  (Files / Sessions / Trackers / Diffs). Active tab gets a primary
  left border + muted background. Local `useState<TabId>` defaulting
  to "files".
- `src/app/panels/FilesPanel.tsx`: wraps existing FileExplorer (every
  prop forwarded).
- `src/app/panels/SessionsPanel.tsx`: wraps SessionList.
- `src/app/panels/TrackersPanel.tsx`: lists workspace trackers from
  `tracker_load_workspace`, drills into TrackerTable on click.
- `src/app/panels/DiffsPanel.tsx`: wraps DiffInbox.
- Removed `sessionsOpen` and `diffsOpen` state from App.tsx, along
  with both floating-toggle JSX blocks. Standalone FileExplorer
  mount replaced with `<SidebarTabs files=...>`.
- Sidebar widened to 265px default (170-500 range) to accommodate
  tracker tables and diff lists.
- `PromptForUserInputDialog` stays at top level (it's a modal).

### Fixed

- `provider_claude_code.rs:152` - dropped redundant `trim_start()`
  before `split_whitespace()` (clippy lint).
- `trackers/commands.rs:79` - removed explicit `.into_iter()` from
  `.zip()` (clippy lint).

### CI

- `cargo clippy --all-targets -- -D warnings` re-enabled and green.
- 28 tests pass (up from previous 28; no test count change but the
  M3.2 wireup didn't break any).

### Detail

- `planning/M3_DOT_2_WIREUP.md`
- `planning/M_DOT_TWO_LAYOUT.md`

## [0.1.0-pre.7] - 2026-05-16

M*.1 wireup pass, vision-QA-revised icon, and M2.1 race fix. CI green
with 28 tests passing (up from 25).

### Added (M3.1)

- `mcp::spawn_in_process(...)` invoked in `lib.rs` setup() closure.
- Three MCP commands registered in `tauri::generate_handler!`:
  `mcp_prompt_response`, `mcp_diff_resolve`, `mcp_list_pending_prompts`.

### Added (M4.1)

- Five tracker commands registered in `tauri::generate_handler!`:
  `tracker_load_workspace`, `tracker_create_item`, `tracker_update_item`,
  `tracker_list_items`, `tracker_query`.

### Added (M5.1)

- `diff_apply_and_resolve` Tauri command registered.
- `DiffInbox` imported and mounted in `App.tsx` behind a toggle
  button + 520x60vh overlay panel (mirrors the Sessions overlay
  pattern). A richer layout-routed panel is M5.2.

### Fixed (M2.1)

- Watcher race in `sessions/manager.rs`. The watcher now: (1) awaits
  `child.wait()`, (2) awaits the reader's `JoinHandle` so every
  stdout line is enqueued first, (3) sends `TranscriptEvent::Completed`
  through the shared `mpsc::Sender` clone, (4) drops the sender so
  the flusher's `recv()` returns None on the next poll.
- Result: `Completed` is deterministically the last DB row and the
  last event in the renderer batch.
- `session_lifecycle_writes_events_to_db` reverted to the strict
  assertion `kinds.last() == Some("completed")`.

### Improved (M2.2-ready)

- `provider_claude_code::parse_line` now distinguishes:
  - `[tool_call] <name>` / `tool_use: <name>` -> `ToolCall`
  - `[error]` / `error:` / `Error:` -> `Error`
  - substring `permission_request` / `awaiting_approval` -> `SystemNotice`
  - everything else -> `AssistantText` (unchanged)
- Three new unit tests cover the new branches.
- Marker shapes are best-guess until real Claude Code stdout
  samples are captured in M2.2.

### Changed (icon, vision-QA pass)

- Vision QA via local Qwen3.6-27B-Q5_K_M-instruct on :7870 reviewed
  the v1 icon: verdict REGEN. Problems: unbalanced composition (heavy
  left box vs thin right lines), mustard yellow too dated, terminal
  prompt `>_` poorly kerned, fails at 32x32.
- Regenerated v2 with the vision model's suggested prompt: centered
  radial composition, electric yellow glow, deep midnight blue,
  designed for 32x32 legibility.
- Vision QA pass 2 verdict: SHIP v2 (strictly better than v1).
- v2 source replaces v1 source PNG. All 15 sized PNGs + `icon.ico`
  (6 sizes) + `icon.icns` + `public/logo.png` regenerated from v2.

### Tests

- `cargo test --lib` on Linux CI: **28 passed, 0 failed, 1 ignored**.
- Frontend `pnpm exec tsc --noEmit` + `pnpm build`: clean.

### Known v0.1.0 punch list (M3.2 / M5.2 territory)

- clippy `-D warnings` remains off. The M3/M5 module internals
  (PendingPrompts.fire, ProposeDiffArgs, compute_unified_diff) are
  reached at runtime once a tool is called, but clippy's static
  analysis flags them as dead in the current wireup. M3.2 routes
  rmcp tool calls through the macro-generated dispatcher, which
  reads every Args struct and makes clippy happy.
- DiffInbox is mounted but not routed into the main layout.
- Real Claude Code stdout parsing still needs M2.2 validation
  against actual CLI output samples.
- Cargo.lock still gitignored (inherited from upstream); CI runs
  without `--locked`.

## [0.1.0-pre.6] - 2026-05-16

M2 milestone landed and validated in CI. M3+M4+M5 modules landed as
scaffolds (sub-agents were rate-limited mid-implementation; lib.rs
wireup deferred to M3.1/M4.1/M5.1). M6 docs added. CI now runs the
full test suite: 25 tests pass.

### Added (M2 - multi-session manager)

- `src-tauri/src/sessions/` (1300+ LOC): SessionManager with three-task
  lifecycle (stdout reader, 50ms-debounced flusher, child watcher),
  Claude Code subprocess adapter (scaffolded, real stdout-format
  parsing is M2.1 work), three Tauri commands, integration tests for
  lifecycle and kill semantics.
- `src/modules/sessions/`: `useSessions()` hook + minimal `SessionList`
  component wired into the App layout.
- TranscriptEvent canonical enum that the manager writes to the DB and
  emits to the renderer via `teraxlyst:session-events` channel.

### Added (M3 - MCP host scaffold)

- `src-tauri/src/mcp/`: in-process MCP server via rmcp 1.7 with three
  tools (`prompt_for_user_input`, `propose_diff`,
  `read_workspace_file`). Correlation-ID + oneshot pipelines for
  prompts and diff resolutions. ~1100 LOC. Module is gated behind
  `#![allow(dead_code)]` until M3.1 wires `spawn_in_process` into the
  lib.rs setup closure.
- `src/modules/mcp/`: 5 PromptForUserInput field components
  (multi-select, single-select, reorder, edit-text, confirm) +
  PromptForUserInputDialog + useMcpPrompts hook.

### Added (M4 - tracker scaffold)

- `src-tauri/src/trackers/`: YAML schema loader + validator + ID
  generation (sequential/ulid/uuid) + 4 Tauri commands (load_workspace,
  create_item, update_item, list_items, query_items) + MCP wrappers.
  ~900 LOC. Module is gated behind `#![allow(dead_code)]` until M4.1
  wires the commands into lib.rs.
- DbActor extended with 5 new request variants: UpsertTracker,
  ListTrackers, CreateTrackerItem, UpdateTrackerItem, ListTrackerItems.
  Schema already contained the tables (M1).
- `serde_yaml_ng = "0.10"` dep added (maintained fork of dtolnay's
  serde_yaml, which was deprecated in 2024).

### Added (M5 - diff approval scaffold)

- `src-tauri/src/diff/`: `diff_apply_and_resolve` Tauri command that
  writes new_content to disk on approve before flipping the proposal
  status. Module is gated behind `#![allow(dead_code)]` until M5.1
  wires it into lib.rs.
- Schema migration v2 (`diff_proposals.new_content TEXT`). DbActor
  CreateDiffProposal extended to accept and store new_content.
- `src/modules/diff/`: DiffInbox + DiffViewer (Monaco-backed) +
  useDiffProposals hook. DiffInbox import in App.tsx commented out
  until M5.1 wires the layout panel.
- `@monaco-editor/react = ^4.6.0` dep added.

### Added (M6 - release prep docs)

- `planning/SIGNING_PLAN.md`: full walkthrough for Apple Developer
  Program enrollment + Windows code-signing cert + Tauri minisign
  keypair generation. ~$179/year cost summary.
- `INSTALL.md`: build-from-source instructions for all three
  platforms with unsigned-binary escape hatches and a
  troubleshooting section.

### Changed (CI)

- pnpm install switched to --no-frozen-lockfile in 0.1.0-pre.4 because
  M5 added @monaco-editor/react without a lockfile refresh. The
  refresh + restoring --frozen-lockfile is M0.5 work.
- cargo clippy temporarily ran without -D warnings to land the
  M3+M4+M5 scaffolds. Their unreachable surface generates legitimate
  dead-code warnings until the lib.rs wireup happens. -D warnings
  re-enable is M0.7 work.

### Verified

- `pnpm exec tsc --noEmit`: clean.
- `pnpm build`: clean.
- `cargo check --all-targets`: clean.
- `cargo clippy --all-targets`: clean (without -D warnings).
- `cargo test --lib`: 25 passed, 0 failed, 1 ignored.

### Honest scaffold gaps

The three sub-agents writing M3, M4, M5 hit API rate limits before
completing the final lib.rs wireup. The code is on disk and compiles
cleanly, but the modules are unreachable from the running app until
the wireup happens. M3.1, M4.1, M5.1 are the follow-up tasks; each
is small (just registering commands + spawning the host task in
setup()). Tracked in `planning/M0_TODO.md`.

## [0.1.0-pre.5] - 2026-05-15

M1 milestone: native SQLite persistence layer landed and validated in CI.

### Added

- `src-tauri/src/db/` (8 files, 1065 LOC): rusqlite DbActor + 6-table schema + versioned migrations + 8 Tauri commands + 4 integration tests. Single-writer actor pattern (one `Connection` lives in a `tokio::task::spawn_blocking` thread, all requests serialized through an `mpsc::Receiver<DbRequest>`). WAL mode, `foreign_keys=ON`, `synchronous=NORMAL`. See `planning/M1_IMPLEMENTATION.md`.
- New Tauri commands: `db_create_workspace`, `db_list_workspaces`, `db_create_session`, `db_append_event`, `db_list_events`, `db_create_diff_proposal`, `db_resolve_diff_proposal`, `db_list_pending_proposals`.
- Schema: `workspaces`, `sessions`, `session_events`, `trackers`, `tracker_items`, `diff_proposals` + 3 indexes. Tracker tables exist but CRUD handlers are M4 scope.
- `Cargo.toml` deps: `rusqlite 0.31` (bundled + serde_json), `tokio 1` (rt-multi-thread + macros + sync + fs), `thiserror 1`, `ulid 1`. Dev-deps: `tempfile 3`, `tokio` test-util.
- `lib.rs` setup closure: resolves `<app_data_dir>/teraxlyst.sqlite`, creates the dir if missing, runs migrations, spawns the DbActor, stores the `DbHandle` in `app.manage()`.

### CI

- Added `cargo test --lib` to the Rust job. The 1000-event concurrent integration test, the migration idempotency test, the diff proposal round-trip test, and the workspace list test all execute on every push. 4 tests, 4 passed in 0.10s. Total Rust job time: 2m37s.

### Verified

- `cargo check --all-targets`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --lib`: 4/4 passed.
- Frontend `pnpm exec tsc --noEmit` + `pnpm build`: clean.
- AI-marker scan on M1 deliverables: zero hits.

### Deliberate v1.1 cuts (per `planning/M1_IMPLEMENTATION.md`)

- Single writer connection, no reader pool. Trivially additive.
- No FTS5 on `session_events.payload`. Will land with search UI post-M2.
- No batched event insert. M2 SessionManager will add `AppendEventsBatch` to address the write-amplification risk flagged in ROADMAP M1.
- No tracker CRUD. Schema includes the tables; handlers land in M4.
- `ulid` dep present but unused. Reserved for stable external session IDs once exposed over MCP.

## [0.1.0-pre.4] - 2026-05-15

M0 closeout pass. The deferred-by-design lockstep renames are now done. Two parallel sub-agents covered the persistence layer and the shell-integration scripts; the remaining items were handled inline.

### Added

- `src/lib/migration.ts`: one-time, idempotent localStorage key migration helper. Copies legacy `terax-*` and `terax:*` keys to the new `teraxlyst-*` / `teraxlyst:*` keys on first launch, then removes the originals. Try/catch-wrapped so a permission failure doesn't kill app startup. Wired into `src/main.tsx` before the React root mounts. No-op on second run.

### Changed

- **Frontend persistence** (sub-agent, 12 files): renamed 2 localStorage keys, 5 tauri-plugin-store filenames (`terax-settings.json` -> `teraxlyst-settings.json`, plus the four `terax-ai-*.json` files), 1 OS keychain service name (`terax-ai` -> `teraxlyst`), 4 custom event names (`teraxlyst://prefs-changed` and the three other change events), 3 CSS identifiers (`teraxlyst-collapsible-content` class + `teraxlyst-collapsible-down/up` keyframes) across globals.css + 2 consumers, and the `<teraxlyst-command>` wire-format token + `TERAXLYST_CMD_RE` regex paired across producer and consumer. Detail: `planning/M0_FRONTEND_PERSISTENCE_RENAME.md`.
- **Backend shell integration** (sub-agent, 9 files): renamed all script-internal function prefixes (`_teraxlyst_precmd`, `_teraxlyst_urlencode`, `_teraxlyst_preexec`, etc.) across bashrc.bash, zshrc.zsh, zshenv.zsh, zlogin.zsh, zprofile.zsh, init.fish, profile.ps1; renamed all env vars (`TERAXLYST_USER_ZDOTDIR`, `TERAXLYST_TERMINAL`, `__TERAXLYST_HOOKS_LOADED`, `__TERAXLYST_PS1_INJECTED`); renamed cache path to `~/.cache/teraxlyst/shell-integration`; renamed CWD sentinel `__TERAXLYST_CWD__`; renamed per-command wrapper var `__teraxlyst_rc`. Lockstep verified: every emit from Rust pairs with the corresponding script-side read. Detail: `planning/M0_BACKEND_SCRIPTS_RENAME.md`.
- **Tauri events crossing the Rust/TS boundary** (inline): `teraxlyst:settings-tab` and `teraxlyst:ai-attach-file` paired in `src-tauri/src/lib.rs` + `src/settings/SettingsApp.tsx` and `src/app/App.tsx` + `src/modules/ai/lib/composer.tsx`.
- **Domain references** (inline): `WEBSITE` constant in `AboutSection.tsx` and OpenRouter `HTTP-Referer` in `agent.ts` now point at `https://github.com/ademczuk/Teraxlyst`. This is a placeholder until a dedicated domain is registered.
- **CI workflow extended**: added `rust` job to `.github/workflows/ci.yml` that runs `cargo check` + `cargo clippy -- -D warnings` against `src-tauri/` on ubuntu-22.04 with webkit2gtk + gtk system deps. Caches cargo registry and target/ via swatinem/rust-cache. This validates the backend compiles on at least one platform (since local Windows MSVC isn't available).

### Verified

- `pnpm exec tsc --noEmit`: clean after all renames.
- `pnpm build`: clean, 13.54s.
- AI-marker scan on all 22 authored docs + new migration helper: zero hits.
- Cross-file lockstep correctness verified by both sub-agents via post-edit grep audits.

### Migration story

- **localStorage and event listeners**: automatic on first launch via the new migration helper.
- **tauri-plugin-store JSON files**: NOT migrated. Files live in the app data dir; old files are silently orphaned. Acceptable because there are no v0.1 users yet. Dev-mode users with pre-rename state can copy files manually if they want them.
- **OS keychain entries**: NOT migrated. macOS Keychain / Windows Credential Manager / Linux secret-service migrations are cross-platform painful. Users will need to re-enter API keys after upgrade. Documented in the persistence-rename report.
- **Shell-integration cache `~/.cache/terax/`**: orphan directory left on disk after upgrade. Inline comment in `shell_init.rs` notes users can `rm -rf` it manually.

### Out of scope (per user direction)

- Linux distro install command rewrite (`terax-bin` AUR, `Terax_*.deb`, `Terax-*.rpm`) in `UpdaterDialog.tsx`.
- Mobile icon assets (`src-tauri/icons/android/`, `src-tauri/icons/ios/`).

### Still deferred to M6 (release prep, not M0)

- Auto-updater minisign keypair generation and config.
- macOS notarization and Windows code signing.
- Cross-platform release pipeline (currently parked at `.github/workflows-pending/release.yml`).

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
