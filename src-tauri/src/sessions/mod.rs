// AI session manager. See planning/ARCHITECTURE.md sections 4 + 11.
//
// M2.0 scope: one provider (Claude Code subprocess) + a simple list view.
// Kanban + Codex adapter are M2.1.

pub mod commands;
pub mod error;
pub mod manager;
pub mod provider_claude_code;
pub mod types;

#[cfg(test)]
mod tests;

pub use manager::SessionManager;
