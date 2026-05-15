// Mirrors the Rust types in src-tauri/src/mcp/types.rs. Kept in one place so
// the renderer side has stable shapes.

export type PromptField =
  | {
      kind: "multi-select";
      label: string;
      options: string[];
      min?: number | null;
      max?: number | null;
    }
  | { kind: "single-select"; label: string; options: string[] }
  | { kind: "reorder"; label: string; items: string[] }
  | {
      kind: "edit-text";
      label: string;
      default?: string | null;
      placeholder?: string | null;
    }
  | { kind: "confirm"; label: string };

export interface PromptRequest {
  id: string;
  prompt_text: string;
  field: PromptField;
}

export interface DiffRequest {
  id: string;
  proposal_id: number;
  session_id: number;
  file_path: string;
  patch: string;
}

export interface PendingPromptSummary {
  id: string;
  prompt_text: string;
  field: PromptField;
}

export const EVENT_PROMPT = "teraxlyst:mcp-prompt";
export const EVENT_DIFF = "teraxlyst:mcp-diff";
