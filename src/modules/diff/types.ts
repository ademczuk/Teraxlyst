// Shapes returned by the Rust backend for M5 diff approval.
//
// DiffProposal mirrors src-tauri/src/db/types.rs::DiffProposal.
// DiffApplyOutcome mirrors src-tauri/src/diff/commands.rs::DiffApplyOutcome.
//
// new_content is Option<String> on the Rust side; serde JSON-ifies it as
// either a string or null.

export type DiffProposal = {
  id: number;
  session_id: number;
  file_path: string;
  base_hash: string | null;
  patch: string;
  status: "pending" | "approved" | "rejected" | string;
  created_at: number;
  resolved_at: number | null;
  new_content: string | null;
};

export type DiffApplyOutcome = {
  proposal: DiffProposal;
  wrote_file: boolean;
};

// The M3 propose_diff tool emits this on teraxlyst:mcp-diff. It carries
// a correlation id (the M3 ulid, not the DB row id) plus the proposal
// id we use to look up new_content via db_list_pending_proposals.
export type DiffRequestEvent = {
  id: string;
  proposal_id: number;
  session_id: number;
  file_path: string;
  patch: string;
};

export type DiffAction = "approve" | "reject";
