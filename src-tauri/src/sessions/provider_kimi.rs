// Kimi provider adapter (Moonshot AI Kimi CLI).
//
// Spawns `kimi --prompt <prompt> --output-format stream-json --print`.
// PYTHONIOENCODING=utf-8 is set on the child env to work around a
// Windows charmap bug in the upstream CLI (it tries to print emoji
// like \U0001f44b on stdout and crashes when the active codepage is
// not UTF-8).
//
// Auth: OAuth via `kimi login` only. No API key paths are exercised
// or documented; the user authenticates the CLI once and Teraxlyst
// spawns reuse that token.
//
// Stream-json shape (Kimi, distinct from Claude's envelope):
//   {"role":"assistant","content":[
//     {"type":"think","think":"...","encrypted":null},
//     {"type":"text","text":"Hey!"}
//   ]}
// content[].type = "think" -> internal reasoning, skipped from
// transcript. content[].type = "text" -> AssistantText.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

use serde_json::Value;
use shared_child::SharedChild;

use super::error::ManagerError;
use super::types::TranscriptEvent;

const KIMI_BIN: &str = "kimi";

pub fn spawn_kimi(
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
            let mut c = Command::new(KIMI_BIN);
            c.env("PYTHONIOENCODING", "utf-8")
                .arg("--prompt")
                .arg(prompt)
                .arg("--output-format")
                .arg("stream-json")
                .arg("--print");
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
    if !trimmed.starts_with('{') {
        // "To resume this session: kimi -r ..." and similar terminal
        // hints land here. Surface as a system notice so the operator
        // can still see the resume id.
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
    let role = value.get("role").and_then(|v| v.as_str()).unwrap_or("");
    if role != "assistant" {
        return None;
    }
    let content = value.get("content")?.as_array()?;
    for entry in content {
        let ty = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match ty {
            "text" => {
                let text = entry.get("text").and_then(|v| v.as_str()).unwrap_or("");
                if !text.is_empty() {
                    return Some(TranscriptEvent::AssistantText {
                        text: text.to_string(),
                    });
                }
            }
            // "think" entries are reasoning traces; skip rather than
            // dump them into the transcript.
            "think" => continue,
            _ => continue,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_assistant_text_extracts_text() {
        let line = r#"{"role":"assistant","content":[{"type":"think","think":"reasoning","encrypted":null},{"type":"text","text":"Hey!"}]}"#;
        match parse_line(line) {
            Some(TranscriptEvent::AssistantText { text }) => assert_eq!(text, "Hey!"),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn parse_think_only_returns_none() {
        let line = r#"{"role":"assistant","content":[{"type":"think","think":"reasoning"}]}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn parse_resume_hint_becomes_system_notice() {
        match parse_line("To resume this session: kimi -r abc-123") {
            Some(TranscriptEvent::SystemNotice { text }) => {
                assert!(text.contains("resume"));
            }
            other => panic!("unexpected: {:?}", other),
        }
    }
}
