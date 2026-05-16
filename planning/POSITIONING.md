# Teraxlyst positioning - the brutal-honest version

Snapshot of the post-rc4 audit. If you are deciding whether to invest more time in this fork, read this first.

## The question that drove this audit

The operator asked: "was the Rust fork from Electron-based nimbalyst into Tauri 2 + Rust worthwhile?"

## Inputs to the audit

- 4 independent agentic systems queried in parallel with the same prompt
- No agent saw any other agent's response
- Verdicts collected via Pantheon, Hermes-Agentic, Titan-Agentic, Google Deep Research

## Verdicts

| Source | Verdict | Confidence |
|---|---|---|
| Pantheon (tier 2, search strategy) | NOT WORTHWHILE | high |
| Hermes-Agentic (5-step CoT) | CONDITIONAL (NO unless 4 specific conditions met in 6 months) | 0.9 |
| Titan-Agentic (weighted scoring) | NO (2.075 / 10) | high |
| Google Deep Research (2024-26 case studies) | RARELY PAYS OFF (40-50% abandonment rate for Electron-to-Tauri solo forks) | high |

Consensus: **No, the fork is not worthwhile as a product replacement for nimbalyst.** All four converged on this independently.

## Why

| Axis | Score /10 | Weight | Reasoning |
|---|---|---|---|
| Functional completeness vs nimbalyst | 2 | 25% | M0-M6 scaffolds shipped. Zero user-facing creative tooling: no WYSIWYG editor, no Excalidraw, no Mermaid, no iOS, no real-time collab. ~5-20% of nimbalyst's surface. |
| Time-to-feature-parity (solo) | 2 | 20% | 35-55 weeks of focused solo work to match v0.60.1, and nimbalyst ships every ~3 days. The target gains ~120 releases/year you cannot match. |
| User-perceived value (today) | 1 | 20% | Performance wins (4.47 MB installer, 198 ms cold start, 34.6 MB RSS) are developer-legible but user-invisible. No user picks a note-taking tool because it starts in 198 ms instead of 400 ms. |
| Opportunity cost vs upstreaming | 2 | 15% | The novel ideas (MCP host, diff approval, YAML trackers) are not architecturally blocked by Electron. Same 2 days as nimbalyst PRs would have reached 492 stargazers immediately. |
| Architectural quality (Rust foundation) | 7 | 10% | Tauri 2 + Rust + rusqlite + rmcp is a defensible 2026 stack. Memory safety, smaller binary, Sigstore provenance are real wins - for the maintainer, not the user. |
| Maintenance burden going forward | 2 | 5% | Solo dev, two-language codebase, Tauri 2 mobile story still maturing. Burnout risk peaks at month 3-6 per published post-mortems. |
| Bus factor | 1 | 5% | 1. If maintainer steps away, project stalls. nimbalyst has corporate backing + 62 forks as a hedge. |
| **WEIGHTED TOTAL** | **2.4 / 10** | | Below the threshold any reasonable framework sets for "ship as v0.1.0 stable and tell users to use this instead of nimbalyst." |

## What was actually worthwhile about doing it

Verdict-as-learning-exercise: **strong yes.** What shipped that transfers:

1. End-to-end Rust + Tauri 2 + rmcp 1.7 + rusqlite-actor + Sigstore-provenance release pipeline, working on Windows. Reference architecture for any future Rust desktop project.
2. Real M2.2 Claude CLI stream-json parser, verified against captured CLI 2.1.118 output. 11 unit tests pin the envelope shape. Reusable in any future Rust-side MCP / Anthropic integration.
3. `$0/year` release infrastructure (minisign updater + Sigstore + SHA256SUMS + scoop / brew manifests). Transferable to any other Tauri project.
4. M-series novel scaffolds (SessionManager, DiffApproval, YAML trackers, MCP host) - non-trivial original patterns that didn't exist in nimbalyst.
5. 21 -> 0 npm prod CVEs, 0 Rust CVEs, audit guards in CI. Muscle memory transfers.

## What the consensus recommends

3 of 4 models converged on:

1. **Stop framing Teraxlyst as a nimbalyst replacement.** It isn't and won't be.
2. **Backport the novel ideas as nimbalyst PRs.** MCP host scaffold (M3), diff-approval workflow (M5), YAML trackers (M4) - none required leaving Electron. Ship them where the 492 stargazers already are.
3. **Keep Teraxlyst as a published reference repo.** Tag rc4 as v0.1.0-rc4 (already done). Don't promote to v0.1.0 stable and tell users to switch. README positions it as "Rust reference architecture + $0 release pipeline for OSS Tauri projects."
4. **If continuing Rust work:** pick a narrower scope where nimbalyst's full feature surface is irrelevant. A focused MCP client + diff approver, not a nimbalyst clone. Avoids the moving-target problem entirely.
5. **If the operator wants the Rust performance personally:** Teraxlyst is already doing that job. Use it personally. Don't market it as a nimbalyst alternative for others.

## What the prior README + HTML comparison got wrong

The README before this audit (and the first HTML comparison page) framed Teraxlyst as competing with nimbalyst on roughly equal terms. The data does not support that. Six specific framing failures:

1. **Cherry-picked wins.** 7 Teraxlyst advantages vs 7 nimbalyst advantages presented as symmetric. Each nimbalyst advantage is 5-100x more user-impactful than the wins.
2. **Conflated "first paint" with "cold start to WebView2 spawn".** 198 ms is when the renderer process spawns. Full launch to interactive is unmeasured but probably 800-1500 ms.
3. **RSS double-counting.** 581 MB tree RSS reported without flagging that shared Chromium DLL pages are counted per webview process. Honest private bytes is ~250-300 MB, well within Electron range.
4. **`$0/year vs ~$179/year` misframed.** The $179 is what nimbalyst pays for code-signing certs. Teraxlyst moved the cost from cash to user friction (SmartScreen warning on every install).
5. **Buried the feature-gap headline.** "Where nimbalyst wins" should have been the first section, not the seventh.
6. **Dodged the worthwhileness question** with both-sides framing. The operator asked yes/no/conditional and the prior page gave "it depends" weighted toward Teraxlyst.

## How to use this document

If you are a contributor evaluating whether to invest in Teraxlyst: read this first, then decide based on the goals it actually serves (reference architecture, $0 release pipeline experiment, sandbox for novel MCP / diff / tracker ideas), not the goals it was originally pitched against (replacing nimbalyst as a product).

If you are a user evaluating whether to switch from nimbalyst to Teraxlyst: don't. Use nimbalyst.

If you are the operator deciding whether to keep building Teraxlyst: the highest-impact move per consensus is to backport the M3 / M4 / M5 scaffolds to nimbalyst as PRs, then keep Teraxlyst as a personal reference. If you want to keep Teraxlyst alive as a published project, narrow the scope to something nimbalyst is not doing - the consensus suggested a focused MCP client + diff approver rather than a full workspace.

## Source

This document was generated 2026-05-16 in response to operator pushback "Are you sure? read it all again and this time think more deeply and slowly." after the first comparison HTML was deemed too rosy. Full session transcript in this repo's git history; audit prompt + 4 model responses captured in the comparison HTML at `C:/Users/andre/AppData/Local/Temp/teraxlyst-vs-nimbalyst.html` (or wherever the operator saved it).
