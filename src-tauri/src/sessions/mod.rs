// AI session manager. See planning/ARCHITECTURE.md sections 4 + 11.
//
// Four providers are wired, all OAuth-only (no API keys anywhere):
//   - claude-code (primary; OAuth via Claude Code CLI)
//   - codex      (plastic-pipes fallback; OpenAI OAuth via Codex CLI)
//   - gemini     (plastic-pipes fallback; Google OAuth via Gemini CLI)
//   - kimi       (plastic-pipes fallback; Moonshot OAuth via Kimi CLI)
// Adapters live alongside this file; the SessionManager picks one per
// session based on the Provider enum the renderer passes through.

pub mod commands;
pub mod error;
pub mod manager;
pub mod provider_claude_code;
pub mod provider_codex;
pub mod provider_gemini;
pub mod provider_kimi;
pub mod types;

#[cfg(test)]
mod tests;

pub use manager::SessionManager;
