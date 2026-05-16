// Gemini provider adapter (Google Gemini CLI).
//
// Spawns `gemini --prompt <prompt> --allowed-mcp-server-names none`.
// The `--allowed-mcp-server-names none` flag is critical: without it
// the CLI auto-loads every MCP server the user has configured for
// `gemini` interactive mode, which adds 15-20 KB of system-prompt
// overhead and dramatically slows the first response. We want a
// lean one-shot.
//
// Gemini emits plain text on stdout (no streaming envelope). Lines
// like "Ripgrep is not available. Falling back to GrepTool." are
// transient CLI warnings that we treat as SystemNotice; the actual
// response lines become AssistantText.
//
// Auth: OAuth via `gemini auth login` only. Teraxlyst does not set
// any API-key environment variable and does not document an API-key
// fallback - the user authenticates the CLI once and Teraxlyst
// spawns reuse that token.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

use shared_child::SharedChild;

use super::error::ManagerError;
use super::types::TranscriptEvent;

const GEMINI_BIN: &str = "gemini";

pub fn spawn_gemini(
    workspace_path: &Path,
    prompt: &str,
    program_override: Option<(&str, &[&str])>,
) -> Result<Arc<SharedChild>, ManagerError> {
    let mut cmd = match program_override {
        Some((program, args)) => {
            let mut c = Command::new(program);
            c.args(args);
            c
        }
        None => {
            let mut c = Command::new(GEMINI_BIN);
            c.arg("--prompt")
                .arg(prompt)
                .arg("--allowed-mcp-server-names")
                .arg("none");
            c
        }
    };

    cmd.current_dir(workspace_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = SharedChild::spawn(&mut cmd).map_err(|e| ManagerError::SpawnFailed(e.to_string()))?;
    Ok(Arc::new(child))
}

// Known CLI-noise prefixes we route to SystemNotice instead of
// folding into the assistant transcript. List intentionally
// conservative: anything we don't explicitly recognize stays
// AssistantText so we don't silently drop a real response.
const NOISE_PREFIXES: &[&str] = &[
    "Ripgrep is not available",
    "Falling back to",
    "Loaded cached credentials",
    "Auth refresh successful",
];

pub fn parse_line(line: &str) -> Option<TranscriptEvent> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() {
        return None;
    }
    for prefix in NOISE_PREFIXES {
        if trimmed.starts_with(prefix) {
            return Some(TranscriptEvent::SystemNotice {
                text: trimmed.to_string(),
            });
        }
    }
    Some(TranscriptEvent::AssistantText {
        text: trimmed.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_line_returns_none() {
        assert!(parse_line("").is_none());
        assert!(parse_line("\n").is_none());
    }

    #[test]
    fn parse_response_line_becomes_assistant_text() {
        match parse_line("Hi there! How can I help you today?\n") {
            Some(TranscriptEvent::AssistantText { text }) => {
                assert_eq!(text, "Hi there! How can I help you today?")
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn parse_ripgrep_noise_becomes_system_notice() {
        match parse_line("Ripgrep is not available. Falling back to GrepTool.") {
            Some(TranscriptEvent::SystemNotice { text }) => {
                assert!(text.contains("Ripgrep"));
            }
            other => panic!("unexpected: {:?}", other),
        }
    }
}
