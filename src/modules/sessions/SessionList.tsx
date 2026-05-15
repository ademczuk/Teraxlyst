// SessionList: minimal M2.0 UI for the multi-session manager. Lists the
// currently-running sessions and shows a flat stream of the latest 50
// transcript events across all of them. Kanban view is M2.1.

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useState } from "react";

import { summarizeEvent, type TranscriptEvent } from "./types";
import { useSessions } from "./useSessionManager";

type WorkspaceLite = { id: number; name: string; path: string };

const MAX_TRANSCRIPT_ROWS = 50;

export function SessionList(): JSX.Element {
  const { state, createSession, killSession, refreshRunning } = useSessions();

  // We need at least one workspace to create sessions against. v1 just
  // picks the first one; a real workspace picker is M3+.
  const [workspaces, setWorkspaces] = useState<WorkspaceLite[]>([]);
  const [prompt, setPrompt] = useState<string>("");
  const [creating, setCreating] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void invoke<WorkspaceLite[]>("db_list_workspaces")
      .then((list) => setWorkspaces(list))
      .catch((e) => setError(String(e)));
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
        provider: "claude-code",
        prompt,
      });
      setPrompt("");
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }, [activeWorkspace, prompt, createSession]);

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

  return (
    <div className="flex h-full flex-col gap-3 p-3 text-sm">
      <div className="flex items-center gap-2">
        <h2 className="text-base font-semibold">Sessions</h2>
        <button
          type="button"
          className="rounded border border-border px-2 py-0.5 text-xs"
          onClick={() => void refreshRunning()}
        >
          Refresh
        </button>
      </div>

      {error ? (
        <div className="rounded border border-destructive bg-destructive/10 px-2 py-1 text-xs text-destructive">
          {error}
        </div>
      ) : null}

      <div className="flex flex-col gap-1">
        <label className="text-xs text-muted-foreground" htmlFor="session-prompt">
          New session prompt{" "}
          {activeWorkspace ? (
            <span className="opacity-70">(workspace: {activeWorkspace.name})</span>
          ) : (
            <span className="text-destructive">(no workspace)</span>
          )}
        </label>
        <textarea
          id="session-prompt"
          className="min-h-[60px] rounded border border-border bg-background p-2"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          placeholder="What should the agent do?"
        />
        <button
          type="button"
          disabled={creating || !activeWorkspace}
          className="self-start rounded border border-border bg-primary px-3 py-1 text-xs text-primary-foreground disabled:opacity-50"
          onClick={() => void handleCreate()}
        >
          {creating ? "Starting..." : "Start session"}
        </button>
      </div>

      <div className="flex flex-col gap-2">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Running ({state.runningIds.length})
        </h3>
        {state.runningIds.length === 0 ? (
          <div className="text-xs italic text-muted-foreground">no running sessions</div>
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

      <div className="flex min-h-0 flex-1 flex-col gap-1 overflow-hidden">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Latest transcript events ({recentEvents.length})
        </h3>
        <div className="min-h-0 flex-1 overflow-auto rounded border border-border bg-background p-2 font-mono text-xs">
          {recentEvents.length === 0 ? (
            <div className="italic text-muted-foreground">no events yet</div>
          ) : (
            recentEvents.map(({ sid, idx, event }) => (
              <div key={`${sid}-${idx}`} className="whitespace-pre-wrap">
                <span className="opacity-60">#{sid}</span>{" "}
                <span>{summarizeEvent(event)}</span>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
