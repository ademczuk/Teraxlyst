// useDiffProposals: subscribes to pending diff_proposals.
//
// Sources of truth:
//   1. On mount, we fetch the current set of pending proposals via
//      db_list_pending_proposals. This rehydrates the inbox after a
//      renderer reload.
//   2. We then listen on teraxlyst:mcp-diff for new proposals streaming
//      in from M3's propose_diff tool. The event payload carries the
//      proposal_id; we refetch the row to pick up new_content rather
//      than carrying it in the event (keeps the IPC payload small).
//
// The hook returns the proposal list, a manual refresh, and the
// imperative resolve helper that wraps diff_apply_and_resolve.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

import type {
  DiffAction,
  DiffApplyOutcome,
  DiffProposal,
  DiffRequestEvent,
} from "./types";

const EVENT_CHANNEL = "teraxlyst:mcp-diff";

export type UseDiffProposalsApi = {
  proposals: DiffProposal[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  resolve: (id: number, action: DiffAction) => Promise<DiffApplyOutcome>;
};

export function useDiffProposals(): UseDiffProposalsApi {
  const [proposals, setProposals] = useState<DiffProposal[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const list = await invoke<DiffProposal[]>("db_list_pending_proposals", {
        sessionId: null,
      });
      setProposals(list);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    void (async () => {
      try {
        const list = await invoke<DiffProposal[]>("db_list_pending_proposals", {
          sessionId: null,
        });
        if (cancelled) return;
        setProposals(list);
        setLoading(false);
      } catch (e) {
        if (cancelled) return;
        setError(String(e));
        setLoading(false);
      }

      const unlisten = await listen<DiffRequestEvent>(EVENT_CHANNEL, () => {
        // M3 emitted a new proposal. We could merge the event payload
        // into state directly but it lacks new_content; refetching keeps
        // the inbox honest.
        void refresh();
      });

      if (cancelled) {
        unlisten();
      } else {
        unlistenRef.current = unlisten;
      }
    })();

    return () => {
      cancelled = true;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, [refresh]);

  const resolve = useCallback(
    async (id: number, action: DiffAction): Promise<DiffApplyOutcome> => {
      const outcome = await invoke<DiffApplyOutcome>("diff_apply_and_resolve", {
        id,
        action,
      });
      // Optimistically drop the resolved row from local state. The next
      // refresh will reconfirm against the DB.
      setProposals((prev) => prev.filter((p) => p.id !== id));
      return outcome;
    },
    [],
  );

  return { proposals, loading, error, refresh, resolve };
}
