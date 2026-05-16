// Codex provider adapter (OpenAI Codex CLI).
//
// Spawns `codex exec --json --skip-git-repo-check <prompt>`. The
// `--json` flag emits newline-delimited JSON events. The
// `--skip-git-repo-check` flag prevents the CLI from refusing to run
// outside a git working tree (we set cwd to $HOME by default).
//
// Verified event shapes (codex CLI early 2026):
//   {"type":"thread.started","thread_id":"..."}
//   {"type":"turn.started"}
//   {"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Hi."}}
//   {"type":"turn.completed","usage":{"input_tokens":...,"output_tokens":...}}
//
// Auth: OAuth via `codex login` only. Teraxlyst does not set any
// API-key environment variable and does not document an API-key
// fallback - the user authenticates the CLI once and Teraxlyst
// spawns reuse that token.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

use serde_json::Value;
use shared_child::SharedChild;

use super::error::ManagerError;
use super::types::TranscriptEvent;

const CODEX_BIN: &str = "codex";

pub fn spawn_codex(
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
            let mut c = Command::new(CODEX_BIN);
            c.arg("exec")
                .arg("--json")
                .arg("--skip-git-repo-check")
                .arg(prompt);
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

pub fn parse_line(line: &str) -> Option<TranscriptEvent> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() {
        return None;
    }
    // Non-JSON noise (e.g. "Reading additional input from stdin...",
    // "ERROR: The process N not found.") slip in. Surface them as
    // SystemNotice so the operator can see them but they don't
    // pollute the assistant transcript.
    if !trimmed.starts_with('{') {
        return Some(TranscriptEvent::SystemNotice {
            text: trimmed.to_string(),
        });
    }
    let value: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => {
            return Some(TranscriptEvent::SystemNotice {
                text: trimmed.to_string(),
            })
        }
    };
    let ty = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match ty {
        "thread.started" => {
            let tid = value
                .get("thread_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            Some(TranscriptEvent::SystemNotice {
                text: format!("codex thread started: {tid}"),
            })
        }
        "turn.started" => None,
        "item.completed" => {
            let item = value.get("item")?;
            let item_ty = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match item_ty {
                "agent_message" => {
                    let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    if text.is_empty() {
                        None
                    } else {
                        Some(TranscriptEvent::AssistantText {
                            text: text.to_string(),
                        })
                    }
                }
                "tool_call" | "function_call" => {
                    let name = item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let args = item.get("arguments").cloned().unwrap_or(Value::Null);
                    Some(TranscriptEvent::ToolCall { name, args })
                }
                _ => None,
            }
        }
        "turn.completed" => Some(TranscriptEvent::Completed),
        "error" => {
            let msg = value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or(trimmed);
            Some(TranscriptEvent::Error {
                text: msg.to_string(),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_thread_started_becomes_system_notice() {
        let line = r#"{"type":"thread.started","thread_id":"abc"}"#;
        match parse_line(line) {
            Some(TranscriptEvent::SystemNotice { text }) => {
                assert!(text.contains("codex thread started"));
                assert!(text.contains("abc"));
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn parse_turn_started_skipped() {
        assert!(parse_line(r#"{"type":"turn.started"}"#).is_none());
    }

    #[test]
    fn parse_agent_message_item_becomes_assistant_text() {
        let line = r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Hi."}}"#;
        match parse_line(line) {
            Some(TranscriptEvent::AssistantText { text }) => assert_eq!(text, "Hi."),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn parse_turn_completed_becomes_completed() {
        let line = r#"{"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":1}}"#;
        match parse_line(line) {
            Some(TranscriptEvent::Completed) => {}
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn parse_non_json_noise_becomes_system_notice() {
        match parse_line("Reading additional input from stdin...") {
            Some(TranscriptEvent::SystemNotice { text }) => {
                assert!(text.contains("Reading additional"));
            }
            other => panic!("unexpected: {:?}", other),
        }
    }
}
