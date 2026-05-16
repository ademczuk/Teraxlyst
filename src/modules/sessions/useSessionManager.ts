// useSessions(): single hook that surfaces the running-session list and the
// streaming transcript events to the React side.
//
// On mount we (a) ask the backend for the current list of running session
// ids, and (b) subscribe to the "teraxlyst:session-events" channel. On
// unmount we tear down the subscription. The hook exposes createSession +
// killSession as imperative wrappers around the Tauri commands.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

import type {
  Provider,
  SessionEventBatch,
  SessionRow,
  TranscriptEvent,
} from "./types";

const EVENT_CHANNEL = "teraxlyst:session-events";

// Max events kept in memory per session. The UI only renders the last 50
// per the M2.0 spec; we keep a small margin so a fast flush doesn't drop
// the in-flight ones before render.
export const RING_CAP = 200;

export type SessionsState = {
  runningIds: number[];
  // session_id -> last N events (capped at RING_CAP)
  transcripts: Record<number, TranscriptEvent[]>;
};

// Pure reducer extracted so it can be unit-tested without spinning up the
// Tauri event subscription. Applies one event batch, enforces the ring-buffer
// cap, and drops the session_id from runningIds on a completed event.
export function applyTranscriptBatch(
  prev: SessionsState,
  batch: SessionEventBatch,
): SessionsState {
  const existing = prev.transcripts[batch.session_id] ?? [];
  const merged = existing.concat(batch.events);
  const trimmed =
    merged.length > RING_CAP ? merged.slice(-RING_CAP) : merged;
  const nextTranscripts = {
    ...prev.transcripts,
    [batch.session_id]: trimmed,
  };
  const completed = batch.events.some((e) => e.type === "completed");
  const runningIds = completed
    ? prev.runningIds.filter((id) => id !== batch.session_id)
    : prev.runningIds;
  return {
    runningIds,
    transcripts: nextTranscripts,
  };
}

export type UseSessionsApi = {
  state: SessionsState;
  createSession: (args: {
    workspaceId: number;
    provider: Provider;
    prompt: string;
  }) => Promise<SessionRow>;
  killSession: (sessionId: number) => Promise<void>;
  refreshRunning: () => Promise<void>;
};

export function useSessions(): UseSessionsApi {
  const [state, setState] = useState<SessionsState>({
    runningIds: [],
    transcripts: {},
  });

  // Keep the unlisten fn in a ref so the effect cleanup grabs the latest.
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const refreshRunning = useCallback(async () => {
    const ids = await invoke<number[]>("session_list_running");
    setState((prev) => ({ ...prev, runningIds: ids }));
  }, []);

  useEffect(() => {
    let cancelled = false;

    void (async () => {
      try {
        const ids = await invoke<number[]>("session_list_running");
        if (cancelled) return;
        setState((prev) => ({ ...prev, runningIds: ids }));
      } catch {
        // Backend may not be ready yet on first paint; harmless.
      }

      const unlisten = await listen<SessionEventBatch>(
        EVENT_CHANNEL,
        (envelope) => {
          const batch = envelope.payload;
          setState((prev) => applyTranscriptBatch(prev, batch));
        },
      );

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
  }, []);

  const createSession = useCallback(
    async (args: {
      workspaceId: number;
      provider: Provider;
      prompt: string;
    }): Promise<SessionRow> => {
      const session = await invoke<SessionRow>("session_create", {
        workspaceId: args.workspaceId,
        provider: args.provider,
        prompt: args.prompt,
      });
      // Optimistically include the new id; the backend will also emit the
      // initial UserMessage event which sets up the transcript entry.
      setState((prev) => ({
        ...prev,
        runningIds: prev.runningIds.includes(session.id)
          ? prev.runningIds
          : [...prev.runningIds, session.id],
      }));
      return session;
    },
    [],
  );

  const killSession = useCallback(async (sessionId: number) => {
    await invoke<void>("session_kill", { sessionId });
    setState((prev) => ({
      ...prev,
      runningIds: prev.runningIds.filter((id) => id !== sessionId),
    }));
  }, []);

  return { state, createSession, killSession, refreshRunning };
}
