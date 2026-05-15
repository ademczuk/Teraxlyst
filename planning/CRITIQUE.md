# Self-critique of the Teraxlyst plan

Recorded 2026-05-15. After writing the first pass of the planning docs, I re-read everything looking for assumptions I'd made too quickly. This file captures what changed and what still bothers me.

## What I changed

| Issue | Resolution |
|-------|------------|
| README claimed nimbalyst was "per their LICENSE" without naming the license | Verified: nimbalyst is MIT (Nimbalyst Inc., 2024-2026). README and NOTICE now say MIT explicitly. |
| ROADMAP M0 said "clone https://github.com/crynta/terax-ai" without specifying a tag | Pinned to tag **v0.6.5** released 2026-05-15. Forking off main would put us on a moving target. |
| ARCHITECTURE §5 assumed we'd implement the MCP wire protocol in Rust ourselves | Verified: `rmcp` v1.7.0 is the official Anthropic Rust MCP SDK with 4.7M downloads, stable 1.x, stdio + SSE, declarative tool macros. M3 risk drops from "must implement protocol" to "must use a stable library." |
| MCP host docs muddled the server-vs-client role direction | Made the dual role explicit: Teraxlyst is a server to managed sessions and a client to external tool servers, both running concurrently. |
| No mention of provider integration model | Added §11 with three options (subprocess / native SDK / hybrid) and a v1 decision (subprocess). |
| No mention of how testing actually happens | Added §12 with Rust unit, Rust integration, E2E via tauri-driver + Playwright, MCP conformance via `rmcp` client. |
| No telemetry policy | Added §13: no telemetry by default, opt-in only if added later. |
| Code signing handwaved | Added §14: notarization punted to post-0.1.0, SmartScreen warnings documented in install steps. |

## What still bothers me

These are things I noticed but did not fix in this pass. They are open in the sense that the answer probably emerges from doing M0-M2 rather than more planning.

### The timeline is probably wrong

16 weeks part-time for someone who may not have built a Tauri app before is aggressive. Real risks I didn't budget:

- Tauri-WebDriver setup is its own miniature project; a week minimum if you've never done it.
- Cross-platform CI for Tauri (Linux GTK deps, macOS Xcode + signing toolchain, Windows MSVC) eats time disproportionately.
- The first time you wire up `rmcp` server + `rmcp` client in the same process and hit a deadlock between them, you lose two days.

A realistic ship date is closer to **6-8 months** wall-clock, not 4. The roadmap's "5-6 months realistic" caveat is too optimistic.

### The subprocess provider model has a stability hazard

I picked subprocess mode for v1 because it avoids re-implementing agent loops. But:

- `claude-code` and `codex` CLIs have no stable programmatic interface contract. Their stdout format can change between minor versions.
- Detecting tool use, permission prompts, and structured outputs from raw stdout is messy.
- nimbalyst maintains per-provider parsers and ships updates frequently for this reason.

If this becomes painful in M2, the fallback is to use the Anthropic and OpenAI APIs directly via terax-ai's existing HTTP proxy and write a minimal agent loop in Rust. That's a bigger pivot, but recoverable. Worth prototyping a basic Anthropic API agent loop in parallel during M2 as insurance.

### Forking terax-ai inherits 74 open issues + 59 open PRs

terax-ai is actively developed and has a real bug backlog. Forking off 0.6.5 freezes that backlog into our codebase. We'll either:

- Cherry-pick upstream fixes manually (high maintenance cost).
- Track main as a remote and merge periodically (conflict cost grows with our divergence).
- Diverge fully and own all bugs (probable outcome).

Plan: track upstream as a `git remote add upstream` and do a quarterly review of their commits to see if any are worth picking. Don't try to stay in lockstep.

### nimbalyst is moving faster than we will

nimbalyst at v0.60.1, 34 releases, 4,459 commits. They are funded enough to ship at speed. Teraxlyst v0.1.0 will be feature-incomplete vs nimbalyst at launch. The story has to be: different stack (Rust + Tauri), different scope (no mobile / no collab / no rich editors), open-source-first.

If Teraxlyst's pitch is "nimbalyst but in Rust," we lose. The pitch has to be: "a terminal-first AI session manager for power users who want a single Rust binary and don't need the collab + mobile + editor surface."

### The "what we are NOT copying" list might still be too long

The four v1 features (trackers, multi-session manager, diff approval, MCP host) is a tight scope. But each one is complexity-4-or-5. Doing all four well in 16 weeks part-time is a lot. A more honest v1 might be:

- M1 persistence ✓
- M2 multi-session manager (cut the kanban view, ship a simple list)
- M3 MCP host + PromptForUserInput
- Skip trackers in v1, add in v1.1
- Skip diff approval in v1, add in v1.1

That ships a usable "terminal + multi-session AI manager + MCP host" in ~10 weeks. Trackers and diff approval are then v1.1 and v1.2.

Counter-argument: trackers are nimbalyst's most differentiated feature, and shipping without them makes Teraxlyst feel like a less-polished terax-ai. So maybe keep trackers and cut diff approval first.

I do not have a confident answer here. Will revisit at end of M1.

### MCP spec versioning risk wasn't fully addressed

`rmcp` 1.7 is stable, but the underlying MCP spec is still evolving. If Anthropic ships a 2.0 spec that breaks the wire format, we ride out whatever transition `rmcp` provides. The risk is acceptable but not zero.

### Testing strategy is a paper plan

Saying "tauri-driver + Playwright" is easy. Actually wiring it on three OSes and having it run in CI without flakes is multiple days of work. Not budgeted explicitly. Real impact: M5 (diff approval) and M6 (release) will eat M0+M1's "buffer time" if I don't seed the E2E test harness early.

## What I'd do differently next time

- Verify external library availability (`rmcp`) in the research phase, not after the plan is written.
- Get the upstream license check on nimbalyst on day one.
- Budget testing infrastructure as its own milestone, not as line items inside other milestones.
- Resist the temptation to write the schema before the actual session-event flow is prototyped.
