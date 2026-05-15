# M0 Frontend Persistence Rename

Migration of persistence and IPC-layer string identifiers from `terax-*` /
`terax:*` / `terax://*` to `teraxlyst-*` / `teraxlyst:*` / `teraxlyst://*`,
plus a one-time localStorage migration helper.

## Files modified

### localStorage keys
- `src/modules/theme/ThemeProvider.tsx` - `FAST_PATH_KEY` value to `teraxlyst-ui-theme-shadow`.
- `src/modules/updater/useUpdater.ts` - `LAST_CHECK_KEY` value to `teraxlyst:updater:last-check`.

### tauri-plugin-store filenames
- `src/modules/settings/store.ts` - `STORE_PATH` to `teraxlyst-settings.json`.
- `src/modules/ai/lib/agents.ts` - `STORE_PATH` to `teraxlyst-ai-agents.json`.
- `src/modules/ai/lib/sessions.ts` - `STORE_PATH` to `teraxlyst-ai-sessions.json`.
- `src/modules/ai/lib/snippets.ts` - `STORE_PATH` to `teraxlyst-ai-snippets.json`.
- `src/modules/ai/lib/todos.ts` - `STORE_PATH` to `teraxlyst-ai-todos.json`.

### OS keychain service name
- `src/modules/ai/config.ts` - `KEYRING_SERVICE` to `teraxlyst`.
- `src-tauri/src/modules/secrets.rs` - no change needed. The Rust commands
  (`secrets_get/set/delete/get_all`) accept the service name from the
  frontend each call; there is no hardcoded constant or default to rename.

### Custom event names (terax://* to teraxlyst://*)
All four are stored in `const` event-name variables and used through the
Tauri event API (`emit`/`listen` from `@tauri-apps/api/event`). Producer and
consumer are paired through the constant, so a single edit per file keeps
the pair in lockstep:
- `src/modules/settings/store.ts` - `PREFS_CHANGED_EVENT`, `KEYS_CHANGED_EVENT`.
- `src/modules/ai/store/agentsStore.ts` - `CHANGED_EVENT`.
- `src/modules/ai/store/snippetsStore.ts` - `CHANGED_EVENT`.

Note: the spec described these as `window.dispatchEvent` / `window.addEventListener`
DOM events. They actually use Tauri's `@tauri-apps/api/event` emit/listen.
Renamed regardless since they were called out by name.

### CSS class and keyframe names
- `src/styles/globals.css` - keyframes `teraxlyst-collapsible-down`,
  `teraxlyst-collapsible-up`; class `.teraxlyst-collapsible-content`; both
  `animation:` references updated to the renamed keyframes.
- `src/components/ai-elements/tool.tsx:203` - class name in `cn()`.
- `src/modules/ai/components/AiChat.tsx:524` - class name on `CollapsibleContent`.

### Wire-format command token
- `src/modules/ai/lib/slashCommands.ts` - regex renamed to `TERAXLYST_CMD_RE`,
  matching `<teraxlyst-command name="..."/>`; emit format string updated.
- `src/modules/ai/components/AiChat.tsx` - import and match-call updated to
  `TERAXLYST_CMD_RE`.
- `src/modules/ai/lib/composer.tsx:238` - emit format updated to
  `<teraxlyst-command name="..."/>`.

### New: migration helper + wiring
- `src/lib/migration.ts` - new module exporting `runOneTimeMigration()`.
- `src/main.tsx` - calls `runOneTimeMigration()` synchronously before
  `ReactDOM.createRoot(...).render(<App />)`, so it runs before any
  component-mount path that reads localStorage (notably the
  `useState`-initialized fast-path read in `ThemeProvider`).

## Migration helper design

`runOneTimeMigration()` is synchronous, idempotent, and self-contained:

- Holds an explicit `KEY_PAIRS` table of `[oldKey, newKey]` tuples. Currently
  two pairs: the theme shadow key and the updater last-check key. Explicit
  table rather than prefix-derived so an accidental future rename can't
  silently reshape migration semantics.
- For each pair: read `oldKey`; if `null`, skip. Otherwise, if `newKey` is
  not already set, copy the value and log
  `[teraxlyst] migration: ran key=<oldKey>`. Always remove `oldKey` at the
  end of each pass (the new key, if present, is authoritative since post-rename
  writes go there).
- Whole function wrapped in `try/catch`; logs a warn on failure but cannot
  cascade into a white-screen boot.
- Idempotent: second call finds no old keys, logs nothing, returns fast.

Out of scope (by comment): the tauri-plugin-store JSON files on disk. The
`STORE_PATH` rename is enough because `LazyStore` recreates the file at first
read. Pre-v0.1 data sitting in old `terax-*.json` files is silently orphaned;
acceptable given zero current users.

## Final grep

`grep -rn '\bTerax\b\|\bterax\b' src/` returns 10 lines. All are intentionally
remaining:

| Lines | Intent |
|-|-|
| `src/lib/migration.ts:1,5,20,21` and `src/main.tsx:18` | Migration helper comments and the legacy-key strings in the migration table. Required by the helper's job. |
| `src/app/App.tsx:382`, `src/modules/ai/lib/composer.tsx:107,108` | Tauri event `terax:ai-attach-file` (single colon) - explicitly out of scope; handled by another agent. |
| `src/modules/updater/UpdaterDialog.tsx:20,22,24` | Linux package install commands (`yay -S terax-bin`, `apt install Terax_...deb`, `dnf install Terax-...rpm`) - user instructed to skip. |

Additional case-insensitive grep also surfaces `TERAX.md` references in
`agent.ts`, `composer.tsx`, `slashCommands.ts`, `transport.ts` (project
memory filename convention), and one `terax:settings-tab` site in
`SettingsApp.tsx` which appears already renamed to `teraxlyst:settings-tab`
by an earlier pass. Both outside my scope.

## Risks

1. **Existing dev installs lose tauri-plugin-store data on upgrade.** Settings,
   AI sessions, agents, snippets, todos all start fresh because the JSON
   filenames changed. Acceptable for a zero-user project. If anyone has been
   testing locally, advise them to either restore from the old files manually
   or accept a clean slate.
2. **Existing dev installs lose OS keychain secrets.** The `KEYRING_SERVICE`
   rename from `terax-ai` to `teraxlyst` means stored API keys under the old
   service name become orphaned. Re-entering API keys in Settings re-creates
   them under the new service. No automated migration because we'd need to
   know all account names a priori - the cost of a one-time re-paste is lower
   than the code path complexity to enumerate and migrate keychain entries
   across three OS backends.
3. **Tauri event-bus listeners across windows that already loaded with the
   old event names will not see new emits.** Only matters mid-session during
   a hot reload; cold restart picks up the renamed constants everywhere. No
   action needed.
4. **Wire-format `<teraxlyst-command>` token is a producer+consumer pair in
   the same commit.** No risk of skew because both regex and emit format
   were renamed together in slashCommands.ts, AiChat.tsx, and composer.tsx.
5. **CSS rename is also paired** - both `@keyframes` definitions, the
   `animation:` references, and all `className` consumers were updated in
   the same pass. No orphaned animation references possible.
