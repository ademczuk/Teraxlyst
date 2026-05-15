// DiffInbox: list of pending diff_proposals on the left, Monaco diff
// viewer on the right. The first pending proposal is selected on mount.

import { useCallback, useEffect, useMemo, useState } from "react";

import { DiffViewer } from "./DiffViewer";
import type { DiffAction } from "./types";
import { useDiffProposals } from "./useDiffProposals";

function shortName(path: string): string {
  const slash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return slash >= 0 ? path.slice(slash + 1) : path;
}

export function DiffInbox() {
  const { proposals, loading, error, refresh, resolve } = useDiffProposals();
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [opMessage, setOpMessage] = useState<string | null>(null);

  // Auto-select the oldest pending proposal when the list changes and the
  // current selection is no longer pending.
  useEffect(() => {
    if (proposals.length === 0) {
      setSelectedId(null);
      return;
    }
    if (selectedId === null || !proposals.some((p) => p.id === selectedId)) {
      setSelectedId(proposals[0].id);
    }
  }, [proposals, selectedId]);

  const selected = useMemo(
    () => proposals.find((p) => p.id === selectedId) ?? null,
    [proposals, selectedId],
  );

  const handleResolve = useCallback(
    async (action: DiffAction) => {
      if (!selected) return;
      try {
        const outcome = await resolve(selected.id, action);
        setOpMessage(
          action === "approve"
            ? outcome.wrote_file
              ? `wrote ${selected.file_path}`
              : `approved (no file write)`
            : `rejected ${selected.file_path}`,
        );
      } catch (e) {
        setOpMessage(`error: ${String(e)}`);
      }
    },
    [selected, resolve],
  );

  return (
    <div className="flex h-full min-h-0 flex-col text-sm">
      <div className="flex items-center justify-between gap-2 border-b border-border px-2 py-1">
        <h2 className="text-xs font-semibold uppercase tracking-wide">
          Diff Inbox ({proposals.length})
        </h2>
        <button
          type="button"
          onClick={() => void refresh()}
          className="rounded border border-border px-2 py-0.5 text-xs"
        >
          Refresh
        </button>
      </div>

      {error ? (
        <div className="border-b border-destructive bg-destructive/10 px-2 py-1 text-xs text-destructive">
          {error}
        </div>
      ) : null}
      {opMessage ? (
        <div className="border-b border-border bg-muted/40 px-2 py-1 text-[10px] text-muted-foreground">
          {opMessage}
        </div>
      ) : null}

      <div className="flex min-h-0 flex-1">
        <div className="w-[180px] shrink-0 overflow-auto border-r border-border">
          {loading && proposals.length === 0 ? (
            <div className="p-2 text-xs italic text-muted-foreground">
              loading...
            </div>
          ) : proposals.length === 0 ? (
            <div className="p-2 text-xs italic text-muted-foreground">
              no pending proposals
            </div>
          ) : (
            <ul>
              {proposals.map((p) => {
                const isActive = p.id === selectedId;
                return (
                  <li key={p.id}>
                    <button
                      type="button"
                      onClick={() => setSelectedId(p.id)}
                      className={
                        "block w-full truncate px-2 py-1 text-left text-xs " +
                        (isActive
                          ? "bg-primary/15 font-medium"
                          : "hover:bg-muted/40")
                      }
                      title={p.file_path}
                    >
                      <span className="block truncate font-mono">
                        {shortName(p.file_path)}
                      </span>
                      <span className="block text-[10px] text-muted-foreground">
                        #{p.id} session {p.session_id}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>

        <div className="min-w-0 flex-1">
          {selected ? (
            <DiffViewer
              key={selected.id}
              proposal={selected}
              onResolved={handleResolve}
            />
          ) : (
            <div className="p-2 text-xs italic text-muted-foreground">
              select a proposal to review
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
