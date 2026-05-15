# Security policy

Teraxlyst is in early planning. There are no releases yet and no users to put at risk.

## Reporting

Until 0.1.0 ships, file a regular GitHub issue at https://github.com/ademczuk/Teraxlyst/issues. Do not include working exploit code in public issues; describe the impact and the rough mechanism, and the issue can be moved to a private advisory if needed.

A proper coordinated disclosure policy with a dedicated reporting channel will land before the 0.1.0 release as part of milestone M6 (see `planning/ROADMAP.md`).

## Upstream

Teraxlyst is forked from terax-ai v0.6.5. Vulnerabilities in unmodified upstream code may also affect terax-ai. Please report those to both projects:

- terax-ai: https://github.com/crynta/terax-ai/blob/main/SECURITY.md
- Teraxlyst: the issue tracker above

## In scope (when releases exist)

- The Rust backend in `src-tauri/` (PTY, FS, IPC, plugins, MCP host)
- The frontend in `src/` wherever untrusted input lands (terminal output, file content, AI tool results, credentials)
- Release artifacts on GitHub
- The auto-updater (once reinstated in M6)

## Not in scope

- Upstream dependency bugs (report those upstream)
- Anything requiring an already-compromised host or a local attacker with shell access
- Pre-0.1.0 development builds
