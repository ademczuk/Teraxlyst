# M0 Frontend Brand Sweep: Terax -> Teraxlyst

Scope: `src/` only. No changes to `src-tauri/`, `planning/`, top-level docs, or anything else.

## Totals

- Before: 63 occurrences across 30 files
- After: 33 occurrences across 20 files
- Changed: 30 occurrences across 14 files

## Files Modified

| File | Kind of change |
|---|---|
| `src/settings/sections/AboutSection.tsx` | Repo URL `crynta/terax-ai` -> `ademczuk/Teraxlyst`; default app name `"Terax"` -> `"Teraxlyst"`; bundle ID `app.crynta.terax` -> `com.ademczuk.teraxlyst` (now matches `src-tauri/tauri.conf.json`); displayed repo label `crynta/terax-ai` -> `ademczuk/Teraxlyst` |
| `src/settings/sections/AgentsSection.tsx` | Textarea placeholder: "Terax's core system prompt" -> "Teraxlyst's core system prompt" |
| `src/settings/sections/GeneralSection.tsx` | Description string: "Open Terax automatically" -> "Open Teraxlyst automatically" |
| `src/settings/sections/ModelsSection.tsx` | Description: "used only by Terax." -> "used only by Teraxlyst." |
| `src/modules/ai/config.ts` | `SYSTEM_PROMPT` and `SYSTEM_PROMPT_LITE` opening line "You are Terax" -> "You are Teraxlyst" (only the brand token; surrounding upstream prose left intact) |
| `src/modules/updater/UpdaterDialog.tsx` | DialogTitle "Terax v..." (x2) -> "Teraxlyst v..."; DialogDescription "Restart Terax to finish installing." -> "Restart Teraxlyst..." |
| `src/modules/updater/useUpdater.ts` | `GITHUB_LATEST_RELEASE` API endpoint repointed from `crynta/terax-ai` to `ademczuk/Teraxlyst` |
| `src/modules/shortcuts/ShortcutsDialog.tsx` | "Quick reference for Terax controls." -> "Quick reference for Teraxlyst controls." |
| `src/modules/ai/components/AiChat.tsx` | Empty-state title "Ask Terax anything" -> "Ask Teraxlyst anything" |
| `src/modules/ai/components/AiInputBar.tsx` | Input placeholder "Ask Terax anything" -> "Ask Teraxlyst anything" |
| `src/modules/ai/components/AiMiniWindow.tsx` | `<img alt="Terax">` -> `alt="Teraxlyst"`; heading "Ask Terax anything" -> "Ask Teraxlyst anything"; body "Terax sees the active terminal" -> "Teraxlyst sees the active terminal" (also replaced an em-dash on that same line with a plain hyphen, per CLAUDE.md typography rule) |
| `src/modules/ai/components/SelectionAskAi.tsx` | Button label "Ask Terax" -> "Ask Teraxlyst" |
| `src/modules/ai/lib/agent.ts` | OpenRouter `X-Title` header `"Terax"` -> `"Teraxlyst"`. `HTTP-Referer` left at `https://terax.ai` (see Risks) |
| `src/modules/terminal/lib/dormantRing.ts` | Terminal overflow notice `[terax: dropped output...]` -> `[teraxlyst: dropped output...]` |
| `src/modules/terminal/lib/rendererPool.ts` | All `console.warn` log tags `[terax]` -> `[teraxlyst]` and `[terax-webgl]` -> `[teraxlyst-webgl]`; DOM attributes `data-terax-recycler` and `data-terax-slot` -> `data-teraxlyst-*` (private to this file, not referenced anywhere else in the repo) |
| `src/modules/terminal/lib/useTerminalSession.ts` | `console.error("[terax] ...")` tags -> `[teraxlyst]` |

## Files NOT Modified (and why)

### localStorage / store-path / keychain keys (data migration concern, deferred to v0.2)

| File | Identifier | Reason |
|---|---|---|
| `src/modules/theme/ThemeProvider.tsx:34` | `terax-ui-theme-shadow` localStorage key | Touching this strands existing users' fast-path theme cache. Migration helper needed. |
| `src/modules/updater/useUpdater.ts:7` | `terax:updater:last-check` localStorage key | Same migration concern. |
| `src/modules/ai/config.ts:1` | `KEYRING_SERVICE = "terax-ai"` | OS keychain service name. Renaming orphans user API keys in macOS Keychain / Windows Credential Manager / Linux secret-service. |
| `src/modules/settings/store.ts:66` | `STORE_PATH = "terax-settings.json"` | tauri-plugin-store on-disk filename. Renaming hides existing user prefs. |
| `src/modules/ai/lib/agents.ts:82` | `STORE_PATH = "terax-ai-agents.json"` | Same. |
| `src/modules/ai/lib/sessions.ts:11` | `STORE_PATH = "terax-ai-sessions.json"` | Same. |
| `src/modules/ai/lib/snippets.ts:12` | `STORE_PATH = "terax-ai-snippets.json"` | Same. |
| `src/modules/ai/lib/todos.ts:12` | `STORE_PATH = "terax-ai-todos.json"` | Same. |

### Tauri event names (paired with backend, owned by another agent)

| File | Event |
|---|---|
| `src/app/App.tsx:382` | `terax:ai-attach-file` (dispatched) |
| `src/settings/SettingsApp.tsx:69` | `terax:settings-tab` (listened from Rust side) |
| `src/modules/ai/lib/composer.tsx:107,108` | `terax:ai-attach-file` (listener pair with App.tsx) |
| `src/modules/settings/store.ts:137` | `terax://prefs-changed` emit |
| `src/modules/settings/store.ts:375` | `terax://ai-keys-changed` emit |
| `src/modules/ai/store/agentsStore.ts:12` | `terax://ai-agents-changed` |
| `src/modules/ai/store/snippetsStore.ts:10` | `terax://ai-snippets-changed` |

### Exported / cross-file CSS class and serialization marker

| File | Identifier | Reason |
|---|---|---|
| `src/styles/globals.css` (7 occurrences) | `terax-collapsible-content` class + `terax-collapsible-down/up` keyframes | Referenced from `tool.tsx` and `AiChat.tsx`. Cross-file rename, deferred. |
| `src/components/ai-elements/tool.tsx:203` | Uses `terax-collapsible-content` | Cross-file reference to globals.css. |
| `src/modules/ai/components/AiChat.tsx:524` | Uses `terax-collapsible-content` | Same. |
| `src/modules/ai/lib/slashCommands.ts:49,52` | `TERAX_CMD_RE` regex + `<terax-command name="...">` marker | Exported regex paired with serialization marker in `composer.tsx`. Wire-format string. |
| `src/modules/ai/lib/composer.tsx:238` | Emits `<terax-command ...>` marker | Paired with slashCommands.ts regex. |

### Infrastructure strings left for follow-up

| File | String | Reason |
|---|---|---|
| `src/settings/sections/AboutSection.tsx:12,117` | `https://terax.app` website URL + display label | No replacement domain known for the fork. Flag for owner decision. |
| `src/modules/ai/lib/agent.ts:135` | `HTTP-Referer: "https://terax.ai"` | OpenRouter attribution. Should be the fork's domain once it has one. Updated only the `X-Title` for now. |
| `src/modules/updater/UpdaterDialog.tsx:20,22,24` | `terax-bin` AUR package, `Terax_${v}_amd64.deb`, `Terax-${v}-1.x86_64.rpm` | These are literal published distro package filenames. They need to track whatever the fork's release pipeline actually produces. Changing them now without aligned `.deb`/`.rpm`/`PKGBUILD` artifacts would just give users broken install commands. |

## Risks and Surprises Flagged

1. **`https://terax.app` left in two places.** Will display the upstream's website to users until the fork picks its own domain (or replaces with the GitHub releases page).
2. **OpenRouter `HTTP-Referer` still points to upstream.** Low risk, but means OpenRouter analytics will attribute requests to upstream's app entry rather than the fork. Change once the fork has its own domain.
3. **Linux package install commands in `UpdaterDialog.tsx` are stale.** They reference `terax-bin` (AUR), `Terax_${v}_amd64.deb`, and `Terax-${v}-1.x86_64.rpm` which won't exist in the fork's release pipeline until packaging is set up. Users on Linux who hit the "manual update available" path will get install commands that fail. Recommend either rewriting these to point at the fork's eventual release artifacts or short-circuiting the Linux manual-update path to just "Download from GitHub Releases" until packaging exists.
4. **`useUpdater.ts` `GITHUB_LATEST_RELEASE` was repointed to `ademczuk/Teraxlyst`.** This means the auto-updater will start polling the fork's GitHub releases API. If there are no releases yet, the updater will report "no update available" (404 -> empty), which is the right behavior. Verify the repo is public so the API is reachable without auth.
5. **Pre-existing em-dashes in upstream `SYSTEM_PROMPT` (config.ts).** Both the long and lite system prompts contain em-dashes (U+2014) inherited from upstream. Per CLAUDE.md AI-marker rules these would be flagged, but they're inside a string literal we ship as-is to LLM providers - not user-facing text. Out of scope for this sweep; flag for a future pass if you want to scrub AI markers from the system prompt.
6. **Pre-existing typographic AI markers in `AboutSection.tsx`** (one em-dash in a retry-label string, ellipsis chars in status labels). Pre-existing, out of scope for the brand sweep.
7. **No hardcoded API keys, no obvious bugs found** during the read passes.

## Data Persistence Migration Note

A migration helper in v0.2 will need to handle, on first launch:
- Copy `terax-settings.json` -> `teraxlyst-settings.json`, same for the four `terax-ai-*.json` files
- Read `terax-ui-theme-shadow` and `terax:updater:last-check` from localStorage, write under new keys, delete old
- For the OS keychain, prompt user to re-enter API keys (you can't programmatically migrate keyring entries on all three OSes without security surface) OR copy via best-effort on each platform
- The Tauri event names can be renamed at the same time the Rust side does, in lockstep
