// SessionList: minimal M2.0 UI for the multi-session manager. Lists the
// currently-running sessions and shows a flat stream of the latest 50
// transcript events across all of them.
//
// First-run behavior: if no workspace exists, auto-create one at the
// user's home dir so the "Start session" button works immediately.
// Nimbalyst does this; the explicit "create a workspace first" gate
// was a usability bug in the M2.0 scaffold.

import { invoke } from "@tauri-apps/api/core";
import { homeDir } from "@tauri-apps/api/path";
import { useCallback, useEffect, useMemo, useState } from "react";

import { summarizeEvent, type TranscriptEvent } from "./types";
import { useSessions } from "./useSessionManager";

type WorkspaceLite = { id: number; name: string; path: string };

const MAX_TRANSCRIPT_ROWS = 50;

// Pulls structured fields out of the parser's rate-limit text. The
// Rust side formats these as
//   "[RATE_LIMIT] status=exceeded limitType=five_hour resetsAtUnix=1778922000 usage=100%"
function parseRateLimitText(text: string): {
  isWarning: boolean;
  limitType: string;
  resetsAtUnix: number | null;
  usagePercent: number | null;
} {
  const isWarning = text.startsWith("[RATE_LIMIT_WARNING]");
  const ltMatch = text.match(/limitType=(\S+)/);
  const rsMatch = text.match(/resetsAtUnix=(\d+)/);
  const usageMatch = text.match(/usage=(\d+)%/);
  return {
    isWarning,
    limitType: ltMatch?.[1] ?? "unknown",
    resetsAtUnix: rsMatch ? Number(rsMatch[1]) : null,
    usagePercent: usageMatch ? Number(usageMatch[1]) : null,
  };
}

function formatResetCountdown(resetsAtUnix: number | null, nowMs: number): string {
  if (!resetsAtUnix) return "reset time unknown";
  const deltaSec = Math.max(0, resetsAtUnix - Math.floor(nowMs / 1000));
  if (deltaSec === 0) return "should be reset now (refresh)";
  const h = Math.floor(deltaSec / 3600);
  const m = Math.floor((deltaSec % 3600) / 60);
  if (h > 0) return `resets in ${h}h ${m}m`;
  return `resets in ${m}m`;
}

export function SessionList(){
  const { state, createSession, killSession, refreshRunning } = useSessions();

  const [workspaces, setWorkspaces] = useState<WorkspaceLite[]>([]);
  const [prompt, setPrompt] = useState<string>("");
  const [creating, setCreating] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  // Which CLI to spawn. claude-code is the default; codex / gemini /
  // kimi are the "plastic pipes" fallbacks for when Claude OAuth is
  // throttled. All four are OAuth-authenticated CLIs; no API keys.
  // The user authenticates each CLI once (`claude login`, `codex
  // login`, `gemini auth login`, `kimi login`); Teraxlyst spawns
  // reuse those tokens.
  const [provider, setProvider] = useState<
    "claude-code" | "codex" | "gemini" | "kimi"
  >("claude-code");
  // Tick once per 30s so the rate-limit countdown banner refreshes
  // without re-rendering the whole transcript on every frame.
  const [now, setNow] = useState<number>(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 30000);
    return () => clearInterval(id);
  }, []);

  // The two-column transcript layout needs a wide sidebar. When the
  // user navigates straight to Sessions (vs clicking a wide tab from
  // a narrow one), SidebarTabs.onActiveChange does NOT fire because
  // the tab id didn't change. Emit a window event the App listens
  // for so it can resize the sidebar panel on mount.
  useEffect(() => {
    window.dispatchEvent(
      new CustomEvent("teraxlyst:request-wide-sidebar", {
        detail: { source: "sessions" },
      }),
    );
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function loadOrSeed() {
      try {
        const list = await invoke<WorkspaceLite[]>("db_list_workspaces");
        if (cancelled) return;
        if (list.length > 0) {
          setWorkspaces(list);
          return;
        }
        // First run: auto-create a default workspace at $HOME so the
        // user can immediately start a session without first navigating
        // a workspace picker that does not exist yet.
        const home = await homeDir();
        const created = await invoke<WorkspaceLite>("db_create_workspace", {
          path: home.replace(/[/\\]$/, ""),
          name: "Home",
        });
        if (cancelled) return;
        setWorkspaces([created]);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    }
    void loadOrSeed();
    return () => {
      cancelled = true;
    };
  }, []);

  const activeWorkspace = workspaces[0] ?? null;

  const handleCreate = useCallback(async () => {
    if (!activeWorkspace) {
      setError("no workspace available; create one first");
      return;
    }
    if (!prompt.trim()) {
      setError("prompt is empty");
      return;
    }
    setCreating(true);
    setError(null);
    try {
      await createSession({
        workspaceId: activeWorkspace.id,
        provider,
        prompt,
      });
      setPrompt("");
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }, [activeWorkspace, prompt, provider, createSession]);

  const handleKill = useCallback(
    async (id: number) => {
      try {
        await killSession(id);
      } catch (e) {
        setError(String(e));
      }
    },
    [killSession],
  );

  // Flatten transcripts across all sessions into a chronological tail.
  const recentEvents = useMemo(() => {
    const flat: Array<{ sid: number; idx: number; event: TranscriptEvent }> = [];
    for (const [sidStr, list] of Object.entries(state.transcripts)) {
      const sid = Number(sidStr);
      list.forEach((event, idx) => flat.push({ sid, idx, event }));
    }
    // We don't have a true global ordering (no timestamp on TranscriptEvent),
    // so this is best-effort: per-session events are in order, then we take
    // the last MAX_TRANSCRIPT_ROWS overall. Good enough for the M2.0 UI.
    return flat.slice(-MAX_TRANSCRIPT_ROWS);
  }, [state.transcripts]);

  // Surface the most recent [RATE_LIMIT] / [RATE_LIMIT_WARNING] notice as
  // a banner. The Rust parser tags rate_limit_event SystemNotices with
  // these bracket prefixes when status != "allowed" so we can string-
  // match cheaply here.
  const rateLimitBanner = useMemo(() => {
    for (let i = recentEvents.length - 1; i >= 0; i--) {
      const ev = recentEvents[i]?.event;
      if (ev && ev.type === "system_notice" && typeof ev.text === "string") {
        if (
          ev.text.startsWith("[RATE_LIMIT]") ||
          ev.text.startsWith("[RATE_LIMIT_WARNING]")
        ) {
          return parseRateLimitText(ev.text);
        }
      }
    }
    return null;
  }, [recentEvents]);

  // Two-column layout: prompt + controls + running list on the left
  // (~340px), full-height transcript on the right (remainder). The
  // sidebar auto-widens to 900px when Sessions is the active tab
  // (see SidebarTabs.WIDE_TABS + App.tsx sidebar resize plumbing) so
  // this layout has real room to breathe.
  return (
    <div
      className="flex h-full min-h-0 flex-col gap-2 p-3 text-sm"
      data-testid="session-list"
    >
      <div className="flex items-center gap-2">
        <h2 className="text-base font-semibold">Sessions</h2>
        <button
          type="button"
          className="rounded border border-border px-2 py-0.5 text-xs"
          onClick={() => void refreshRunning()}
        >
          Refresh
        </button>
        <span className="ml-auto text-xs text-muted-foreground">
          {activeWorkspace ? (
            <>
              workspace: <span className="text-foreground">{activeWorkspace.name}</span>
            </>
          ) : (
            <span className="text-destructive">no workspace</span>
          )}
        </span>
      </div>

      {error ? (
        <div className="rounded border border-destructive bg-destructive/10 px-2 py-1 text-xs text-destructive">
          {error}
        </div>
      ) : null}

      {rateLimitBanner ? (
        <div
          data-testid="rate-limit-banner"
          className={
            rateLimitBanner.isWarning
              ? "rounded border border-yellow-500/60 bg-yellow-500/10 px-3 py-2 text-xs text-yellow-200"
              : "rounded border border-red-500/60 bg-red-500/10 px-3 py-2 text-xs text-red-200"
          }
        >
          <div className="font-semibold">
            {rateLimitBanner.isWarning
              ? "Claude rate limit warning"
              : "Claude rate limit hit"}
          </div>
          <div className="opacity-80">
            limit type: {rateLimitBanner.limitType}
            {rateLimitBanner.usagePercent != null
              ? ` (${rateLimitBanner.usagePercent}% used)`
              : ""}
            {" - "}
            {formatResetCountdown(rateLimitBanner.resetsAtUnix, now)}
          </div>
          <div className="mt-1 opacity-70">
            Pick Codex / Gemini / Kimi from the provider dropdown to
            keep working while the Claude window resets. All four use
            OAuth tokens from their respective {"`<cli> login`"}{" "}
            subcommand; no API keys.
          </div>
        </div>
      ) : null}

      <div className="flex min-h-0 flex-1 gap-3">
        {/* Left column: controls + running list. Narrow (220px) so the
            transcript on the right gets the bulk of the horizontal
            space even when the sidebar hasn't auto-widened. */}
        <div className="flex w-[220px] shrink-0 flex-col gap-3 overflow-y-auto">
          <div className="flex flex-col gap-1">
            <label
              className="text-xs text-muted-foreground"
              htmlFor="session-prompt"
            >
              New session prompt
            </label>
            <textarea
              id="session-prompt"
              className="min-h-[90px] resize-none rounded border border-border bg-background p-2"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="What should the agent do?"
            />
            <div className="flex flex-col gap-1.5">
              <label
                className="text-xs text-muted-foreground"
                htmlFor="session-provider"
              >
                Provider (OAuth)
              </label>
              <select
                id="session-provider"
                value={provider}
                onChange={(e) =>
                  setProvider(e.target.value as typeof provider)
                }
                className="rounded border border-border bg-background px-2 py-1 text-xs"
                data-testid="session-provider"
              >
                <option value="claude-code">Claude (claude CLI)</option>
                <option value="codex">Codex (OpenAI CLI)</option>
                <option value="gemini">Gemini (Google CLI)</option>
                <option value="kimi">Kimi (Moonshot CLI)</option>
              </select>
              <button
                type="button"
                disabled={creating || !activeWorkspace}
                className="rounded border border-border bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground disabled:opacity-50"
                onClick={() => void handleCreate()}
              >
                {creating ? "Starting..." : "Start session"}
              </button>
            </div>
          </div>

          <div className="flex flex-col gap-1">
            <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Running ({state.runningIds.length})
            </h3>
            {state.runningIds.length === 0 ? (
              <div className="text-xs italic text-muted-foreground">
                no running sessions
              </div>
            ) : (
              <ul className="flex flex-col gap-1">
                {state.runningIds.map((id) => (
                  <li
                    key={id}
                    className="flex items-center justify-between rounded border border-border px-2 py-1"
                  >
                    <span className="font-mono text-xs">session #{id}</span>
                    <button
                      type="button"
                      className="rounded border border-border px-2 py-0.5 text-xs"
                      onClick={() => void handleKill(id)}
                    >
                      Kill
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        {/* Right column: full-height transcript */}
        <div className="flex min-w-0 flex-1 flex-col gap-1 overflow-hidden">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            Transcript ({recentEvents.length} events, last {MAX_TRANSCRIPT_ROWS})
          </h3>
          <div
            className="min-h-0 flex-1 overflow-auto rounded border border-border bg-background p-3 font-mono text-xs"
            data-testid="transcript-pane"
          >
            {recentEvents.length === 0 ? (
              <div className="italic text-muted-foreground">
                no events yet - start a session to see live transcript
              </div>
            ) : (
              recentEvents.map(({ sid, idx, event }) => (
                <div
                  key={`${sid}-${idx}`}
                  className="mb-1 whitespace-pre-wrap break-words"
                >
                  <span className="text-muted-foreground">#{sid}</span>{" "}
                  <span>{summarizeEvent(event)}</span>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
