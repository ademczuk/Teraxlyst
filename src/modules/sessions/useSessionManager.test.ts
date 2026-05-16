// Unit tests for applyTranscriptBatch, the pure reducer that backs the
// useSessions transcript ring buffer.
//
// The hook itself depends on Tauri's invoke/listen runtime so we don't try
// to test it here. Refactoring the reducer to a pure function is what makes
// these tests possible without spinning up a Tauri shell.

import { describe, expect, it } from "vitest";

import {
  applyTranscriptBatch,
  RING_CAP,
  type SessionsState,
} from "./useSessionManager";
import type { SessionEventBatch, TranscriptEvent } from "./types";

function emptyState(runningIds: number[] = []): SessionsState {
  return { runningIds, transcripts: {} };
}

function userMessage(text: string): TranscriptEvent {
  return { type: "user_message", text };
}

function assistantText(text: string): TranscriptEvent {
  return { type: "assistant_text", text };
}

describe("applyTranscriptBatch", () => {
  it("appends a single batch onto an empty session", () => {
    const before = emptyState([1]);
    const batch: SessionEventBatch = {
      session_id: 1,
      events: [userMessage("hi"), assistantText("hello")],
    };

    const after = applyTranscriptBatch(before, batch);

    expect(after.transcripts[1]).toHaveLength(2);
    expect(after.transcripts[1][0]).toEqual(userMessage("hi"));
    expect(after.transcripts[1][1]).toEqual(assistantText("hello"));
    expect(after.runningIds).toEqual([1]);
  });

  it("caps the transcript at RING_CAP and keeps the most recent events", () => {
    // Seed with RING_CAP - 5 events, then push 10 more in one batch.
    const seeded: TranscriptEvent[] = Array.from(
      { length: RING_CAP - 5 },
      (_, i) => assistantText(`old-${i}`),
    );
    const before: SessionsState = {
      runningIds: [42],
      transcripts: { 42: seeded },
    };
    const incoming: TranscriptEvent[] = Array.from(
      { length: 10 },
      (_, i) => assistantText(`new-${i}`),
    );
    const batch: SessionEventBatch = { session_id: 42, events: incoming };

    const after = applyTranscriptBatch(before, batch);

    expect(after.transcripts[42]).toHaveLength(RING_CAP);
    // The oldest 5 events from the seed were trimmed; the first remaining
    // entry should be the 6th seed event.
    expect(after.transcripts[42][0]).toEqual(assistantText("old-5"));
    // And the tail should be the last event we just pushed.
    expect(after.transcripts[42][RING_CAP - 1]).toEqual(
      assistantText("new-9"),
    );
  });

  it("keeps transcripts isolated across sessions", () => {
    const before: SessionsState = {
      runningIds: [1, 2],
      transcripts: { 1: [userMessage("from-one")] },
    };
    const batch: SessionEventBatch = {
      session_id: 2,
      events: [userMessage("from-two")],
    };

    const after = applyTranscriptBatch(before, batch);

    expect(after.transcripts[1]).toEqual([userMessage("from-one")]);
    expect(after.transcripts[2]).toEqual([userMessage("from-two")]);
  });

  it("drops the session id from runningIds when a completed event arrives", () => {
    const before: SessionsState = {
      runningIds: [1, 2, 3],
      transcripts: { 2: [assistantText("done soon")] },
    };
    const batch: SessionEventBatch = {
      session_id: 2,
      events: [assistantText("done"), { type: "completed" }],
    };

    const after = applyTranscriptBatch(before, batch);

    expect(after.runningIds).toEqual([1, 3]);
    expect(after.transcripts[2]).toHaveLength(3);
  });
});
