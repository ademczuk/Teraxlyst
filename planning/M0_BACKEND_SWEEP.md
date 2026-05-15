# M0 Backend Sweep: terax -> teraxlyst

Scope: `src-tauri/src/` only. Frontend (`src/`), docs, and `planning/` untouched. No git commit performed.

## Changed (5 edits across 4 files)

1. `src-tauri/src/modules/pty/session.rs:23` - backpressure marker bytes. `[terax: dropped output due to backpressure]` -> `[teraxlyst: dropped output due to backpressure]`. User-visible terminal output on overflow.
2. `src-tauri/src/modules/pty/session.rs:110` - thread name `terax-pty-reader` -> `teraxlyst-pty-reader`.
3. `src-tauri/src/modules/pty/session.rs:153` - thread name `terax-pty-flusher` -> `teraxlyst-pty-flusher`.
4. `src-tauri/src/modules/pty/session.rs:177` - thread name `terax-pty-waiter` -> `teraxlyst-pty-waiter`.
5. `src-tauri/src/modules/pty/mod.rs:134` - thread name format `terax-pty-drop-{id}` -> `teraxlyst-pty-drop-{id}`.
6. `src-tauri/src/modules/fs/file.rs:96` - tmp filename suffix `.terax.tmp` -> `.teraxlyst.tmp`. Runtime cosmetic; affects atomic-write tmp filenames during execution only.

Note: item count is 6 edits across 4 files (pty/session.rs got 4 edits, the other three files got 1 each).

## Deferred (with reason)

### Paired with frontend (M0.1 coordinated)
- `src-tauri/src/lib.rs:19` - Tauri event `terax:settings-tab`. Paired with a renderer-side `listen("terax:settings-tab", ...)` somewhere in `src/`. Must migrate as a paired change. Another agent handles the frontend.

### Paired with shell scripts (M0.1 all-or-nothing sweep)
- `src-tauri/src/modules/pty/scripts/bashrc.bash` (13 hits): `_terax_urlencode`, `_terax_precmd`, `__TERAX_HOOKS_LOADED`, `__TERAX_PS1_INJECTED`.
- `src-tauri/src/modules/pty/scripts/zshrc.zsh` (16 hits): `_terax_urlencode`, `_terax_precmd`, `_terax_preexec`, `_terax_ret`, `_terax_user_zdotdir`, `__TERAX_HOOKS_LOADED`.
- `src-tauri/src/modules/pty/scripts/init.fish` (13 hits): `__terax_urlencode_path`, `__terax_restore_status`, `__terax_user_prompt`, `__terax_preexec`, `__TERAX_HOOKS_LOADED`.
- `src-tauri/src/modules/pty/scripts/profile.ps1` (8 hits): `__terax_user_prompt`, `__terax_urlencode`, `$global:__TERAX_HOOKS_LOADED`.
- `src-tauri/src/modules/pty/scripts/zshenv.zsh` (4 hits), `zlogin.zsh` (4 hits), `zprofile.zsh` (4 hits): `_terax_user_zdotdir`, `TERAX_USER_ZDOTDIR`.
- `src-tauri/src/modules/pty/shell_init.rs` lines around 53, 117, 289 - `TERAX_TERMINAL` and `TERAX_USER_ZDOTDIR` env-var emits. These env vars are read by the scripts above. Must rename in lockstep with all script files.

### Cache path migration (needs migration story)
- `src-tauri/src/modules/pty/shell_init.rs:172, 211, 298, 312, 332` - `~/.cache/terax/shell-integration` paths, `.__terax_tmp__` rename-staging filenames. Changing orphans existing users' caches. Defer until a migration step is designed (copy or symlink old -> new on first run, or accept the orphan).

### Internal shell sentinel (paired with above)
- `src-tauri/src/modules/shell/session.rs:39` - `CWD_SENTINEL = "__TERAX_CWD__"`. Internal sentinel only; safe to rename in isolation but no functional benefit until external surface lands.
- `src-tauri/src/modules/shell/session.rs:118, 133` - `__terax_rc` variable in command-wrapper templates. Internal-only to the wrapped invocation; defer with the rest of the shell sweep.

### Comments referencing old binary name
- `src-tauri/src/modules/pty/scripts/zshrc.zsh:6` - comment "we shadow $? into `_terax_ret`". Stale after rename but harmless; defer until script sweep.

### Already done (entry point only)
- `src-tauri/src/main.rs:5` - calls `teraxlyst_lib::run()` (already migrated from `terax_lib`). No action.

## Verification

Pre-edit grep: ~87 hits across 14 files.
Post-edit grep: 81 hits across 14 files. All 6 safe-to-change items now read `teraxlyst*`. The remaining 81 are deferred per categories above.

Re-run command: `grep -rin 'terax' src-tauri/src/`
