// Diff approval module. See planning/ARCHITECTURE.md section 6 and
// planning/M5_IMPLEMENTATION.md.
//
// M5 scope: bridge between the M1 diff_proposals table and the renderer's
// Monaco diff viewer. The agent (M3 propose_diff tool) writes a proposal
// row carrying both the unified patch and the proposed new_content. The
// renderer subscribes to teraxlyst:mcp-diff channel events, opens the
// Monaco diff editor, and the user clicks Approve or Reject. Approve
// routes through diff_apply_and_resolve which writes new_content to disk
// before flipping the proposal status to "approved".

pub mod commands;

#[cfg(test)]
mod tests;
