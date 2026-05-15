# M0 Backend Scripts Rename: terax -> teraxlyst

Lockstep rename pass covering shell integration scripts plus the Rust callers
that emit env vars and read cache paths. All identifiers renamed in sync.

## Files changed

| File | New-name hits | Notes |
|------|---------------|-------|
| `src-tauri/src/modules/pty/scripts/bashrc.bash` | 13 | Full rewrite. `__TERAX_HOOKS_LOADED`, `__TERAX_PS1_INJECTED`, `_terax_urlencode`, `_terax_precmd`, `_terax_ret` all renamed; PROMPT_COMMAND case-match string also bumped. |
| `src-tauri/src/modules/pty/scripts/zshrc.zsh` | 16 | Full rewrite. Includes `_teraxlyst_preexec`, `_teraxlyst_user_zdotdir`, `TERAXLYST_USER_ZDOTDIR` plus the `add-zsh-hook precmd/preexec` references and the comment about shadowing $? into `_teraxlyst_ret`. |
| `src-tauri/src/modules/pty/scripts/zshenv.zsh` | 4 | Full rewrite. Replaced the upstream comment's curly-prompt-arrow reference with ASCII "prompt arrow". |
| `src-tauri/src/modules/pty/scripts/zlogin.zsh` | 4 | Full rewrite. |
| `src-tauri/src/modules/pty/scripts/zprofile.zsh` | 4 | Full rewrite. |
| `src-tauri/src/modules/pty/scripts/init.fish` | 13 | Full rewrite. `__terax_urlencode_path`, `__terax_restore_status`, `__terax_user_prompt`, `__terax_preexec`, and `__terax_status` local all renamed. |
| `src-tauri/src/modules/pty/scripts/profile.ps1` | 8 | Full rewrite. `$global:__TERAX_HOOKS_LOADED`, `__terax_user_prompt`, `__terax_urlencode` all renamed; the `& __terax_user_prompt` call site and `Test-Path Function:` checks updated. |
| `src-tauri/src/modules/pty/shell_init.rs` | 8 | Surgical edits: 2x `TERAXLYST_TERMINAL` env (unix + WSL paths), 1x `TERAXLYST_USER_ZDOTDIR`, 2x `.cache/teraxlyst/...` PathBuf joins (unix + windows modules) plus a third format-string at the WSL-bash helper, 2x `.__teraxlyst_tmp__` (unix + windows write_if_changed), plus the two single-line migration-note comments above the renamed cache roots. |
| `src-tauri/src/modules/shell/session.rs` | 3 | `CWD_SENTINEL` const literal -> `__TERAXLYST_CWD__`; both Posix and PowerShell sentinel-wrapper format strings switched `__terax_rc` -> `__teraxlyst_rc`. |

## Final grep results

Inside scope `src-tauri/src/modules/pty/` and `src-tauri/src/modules/shell/`:

- `grep -rE 'TERAX_[A-Z]|__TERAX_|_terax_[a-z]|__terax_|terax-shell-integration|\.cache/terax/|\.__terax_tmp__'` -> 2 hits, BOTH the intentional pre-0.1.0 migration-note comments at `shell_init.rs:172` (unix) and `shell_init.rs:313` (windows).
- Case-insensitive `grep -i 'terax|TERAX'` -> all remaining hits are either:
  - `teraxlyst`/`TERAXLYST` (renamed correctly), or
  - the existing `teraxlyst-pty-*` thread names in `pty/session.rs` and `pty/mod.rs` (already renamed in the prior pass; out of this rename's scope), or
  - the two migration-note comments listed above.

No functional reference to old names remains.

## Cross-file coordination verified

Every layer is now in sync:

1. **Env vars** Rust emits -> what scripts read:
   - `TERAXLYST_TERMINAL` set in `shell_init.rs:53` (unix) and `:290` (WSL bash). Not read by scripts (consumer-side flag for child processes).
   - `TERAXLYST_USER_ZDOTDIR` set in `shell_init.rs:117`, read in `zshenv.zsh:7`, `zprofile.zsh:5`, `zshrc.zsh:9`, `zlogin.zsh:9`.
2. **Cache paths**: written by `shell_init.rs` integration_root helpers (both unix and windows modules), staging tmp `.__teraxlyst_tmp__` matches both `write_if_changed` callsites.
3. **CWD sentinel** `__TERAXLYST_CWD__` constant in `shell/session.rs:39` matches the literal interpolated into both wrappers via `{CWD_SENTINEL}` (so the wrapper-emitted string and the stdout-parser look for the same token).
4. **Wrapper rc-var** `__teraxlyst_rc` is internal to each wrapped command's shell, used only within the format string; no script-side reference needed.

## Tricky discoveries

1. **shell_init.rs has the cache path repeated three times.** The unix module's `integration_root` and the windows module's `integration_root` each have their own copy, plus a third format-string in `prepare_wsl_bash_rcfile` (`"{}/.cache/teraxlyst/shell-integration/bash"`). All three updated; missing the WSL one would have stranded WSL users with a path that the windows-side never creates.
2. **`__teraxlyst_rc` is two completely separate things.** In `bashrc.bash` and `zshrc.zsh` the wrappers do NOT define an `_terax_rc`; that name lives only in `shell/session.rs` wrap functions. The shell scripts use `_teraxlyst_ret` (precmd's exit-code shadow). No collision: `__teraxlyst_rc` only appears inside the per-command wrapper string, never inside the integration scripts.
3. **PowerShell variable scope.** `$global:__TERAXLYST_HOOKS_LOADED` keeps the `$global:` prefix; renaming only the identifier preserves the global-scope assignment semantics.
4. **PROMPT_COMMAND case-match in bash.** The case pattern `*":_terax_precmd:"*` must match the literal name used in the assignment one line below. Both edited together via the full file rewrite.
5. **zshenv comment had a non-ASCII curly arrow** in the original (`robbyrussell's curly-arrow`). Rewrote it as `robbyrussell's prompt arrow` to keep the file pure ASCII per the rule against typographic markers.
6. **Migration-note placement.** Added the single-line `// Pre-0.1.0 dev artifacts under ~/.cache/terax/ are stranded; safe to rm -rf that dir.` comment directly above EACH of the two `integration_root` cache-path lines (unix module and windows module), so anyone tracing the cache directory sees the note regardless of platform.

## Out of scope (not touched)

- `src/`, `planning/`, `src-tauri/src/lib.rs`, top-level docs.
- `pty/session.rs` and `pty/mod.rs` thread names already use `teraxlyst-pty-*` from prior pass.
- The em-dash characters at `shell_init.rs:125` and `shell/session.rs:15,37` are inside pre-existing comments that this rename pass did not modify. Flagging for a future ASCII-cleanup pass.
