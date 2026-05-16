// Mirror of src-tauri/src/sessions/types.rs.
//
// Keep these definitions in sync with the Rust side. The Rust enum uses a
// tagged representation via #[serde(tag = "type", rename_all = "snake_case")]
// so TypeScript sees a discriminated union keyed on `type`.

export type TranscriptEvent =
  | { type: "user_message"; text: string }
  | { type: "assistant_text"; text: string }
  | { type: "tool_call"; name: string; args: unknown }
  | { type: "tool_result"; name: string; payload: unknown }
  | { type: "system_notice"; text: string }
  | { type: "error"; text: string }
  | { type: "completed" };

// Provider tag. Matches Rust's #[serde(rename_all = "kebab-case")] on
// the Provider enum. All four are OAuth-only; no API key path exists.
// claude-code is the primary; codex / gemini / kimi are the "plastic
// pipes" fallbacks for when Claude OAuth is throttled.
export type Provider = "claude-code" | "codex" | "gemini" | "kimi";

// Batched-event payload emitted from Rust on the "teraxlyst:session-events"
// channel.
export type SessionEventBatch = {
  session_id: number;
  events: TranscriptEvent[];
};

// DB Session row, matches src-tauri/src/db/types.rs Session.
export type SessionRow = {
  id: number;
  workspace_id: number;
  parent_id: number | null;
  provider: string;
  title: string | null;
  status: string;
  created_at: number;
  updated_at: number;
};

// Pretty-print a TranscriptEvent for the transcript list.
export function summarizeEvent(event: TranscriptEvent): string {
  switch (event.type) {
    case "user_message":
      return `user: ${event.text}`;
    case "assistant_text":
      return event.text;
    case "tool_call":
      return `tool_call: ${event.name}`;
    case "tool_result":
      return `tool_result: ${event.name}`;
    case "system_notice":
      return `[system] ${event.text}`;
    case "error":
      return `[error] ${event.text}`;
    case "completed":
      return "[session completed]";
  }
}
