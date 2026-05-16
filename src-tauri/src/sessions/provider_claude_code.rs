// Claude Code provider adapter.
//
// This module knows how to spawn the `claude` subprocess and parse a single
// line of its stdout into Option<TranscriptEvent>. It does NOT touch the DB,
// the registry, the Tauri AppHandle, or any debouncing. The manager owns all
// of that.
//
// M2.2 (verified against real Claude CLI 2.1.118 on Windows):
// the `--output-format stream-json --verbose` mode emits newline-delimited
// JSON events. Each event has a top-level `type` field discriminating it:
//   - system  + subtype=hook_started|hook_response : hook lifecycle (skipped)
//   - system  + subtype=init                       : session metadata
//   - assistant : a message with content[] where each entry is either
//                 { type: "text", text: "..." }
//                 or { type: "tool_use", name: "...", input: { ... } }
//   - user      : a tool_result wrapper, content[] entries are
//                 { type: "tool_result", tool_use_id: "...", content: ... }
//   - rate_limit_event : surfaced as SystemNotice
//   - result          : terminal event, mapped to Completed
//
// Plain text lines (when `--output-format` is omitted, i.e. classic --print
// mode) still flow through the legacy marker heuristic so existing scripts
// and the test stub keep working. JSON parsing is tried FIRST so that any
// well-formed JSON wins over the heuristic.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

use serde_json::Value;
use shared_child::SharedChild;

use super::error::ManagerError;
use super::types::TranscriptEvent;

const CLAUDE_BIN: &str = "claude";

// Spawn the Claude Code CLI as a subprocess, piping stdout/stderr so the
// manager can read line-by-line. `workspace_path` is the cwd of the child.
//
// When `program_override` is Some, that command + args is spawned instead.
// This is the seam the tests use to inject a deterministic shell command.
pub fn spawn_claude_code(
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
            let mut c = Command::new(CLAUDE_BIN);
            // Verified against Claude CLI 2.1.118 on Windows: this flag set
            // produces newline-delimited JSON envelopes that parse_line knows
            // how to decode. --verbose is required by the CLI when stream-json
            // is selected (otherwise the CLI exits with a usage error).
            c.arg("--print")
                .arg("--output-format")
                .arg("stream-json")
                .arg("--verbose")
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

// Parse a single stdout line into an event.
//
// The function returns at most one TranscriptEvent per line. assistant
// messages with multiple content entries (e.g. text + tool_use in the same
// turn) collapse to the FIRST non-empty entry; the caller is responsible
// for any multi-event fan-out it wants. For 0.1 the manager treats each
// line as atomic and writes one row per line, which keeps the actor simple
// and matches what M2 was already doing.
//
// Routing order:
//   1. If the line parses as JSON with a known "type", route via JSON path.
//   2. Otherwise fall back to the legacy marker heuristic (markers and
//      plain text). This keeps the test stub (bash -c "echo line1; ...")
//      and any operator using bare --print working.
pub fn parse_line(line: &str) -> Option<TranscriptEvent> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() {
        return None;
    }

    // JSON path. Only attempted when the line looks like it could be JSON;
    // this avoids paying serde_json for every plain-text line. The JSON
    // branch returns three distinct signals:
    //   - Some(Some(event)) : recognized event, emit it
    //   - Some(None)        : recognized but intentionally suppressed
    //                         (hook lifecycle, empty assistant content) -
    //                         the manager should skip this line entirely
    //   - None              : JSON parse failed or type was unknown - fall
    //                         through to the text heuristic so we don't
    //                         silently drop a line we couldn't classify
    if trimmed.starts_with('{') {
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            if let Some(decision) = classify_json_event(&value, trimmed) {
                return decision;
            }
        }
    }

    // Marker / plain-text fallback.
    parse_text_line(trimmed)
}

// Returns Some(decision) when the JSON has a recognized "type" the parser
// knows about. Some(None) means "skip this line, it's a known no-op".
// Returns None when the type field is missing or unknown so the caller can
// fall through to text. This three-state signal is needed because hook
// lifecycle events are valid JSON we want to drop, not text we want to keep.
fn classify_json_event(value: &Value, raw: &str) -> Option<Option<TranscriptEvent>> {
    let ty = value.get("type").and_then(|v| v.as_str())?;

    match ty {
        "system" => {
            let subtype = value.get("subtype").and_then(|v| v.as_str()).unwrap_or("");
            match subtype {
                // Hook lifecycle events are internal CLI plumbing. The
                // operator doesn't need to see them in the session feed.
                "hook_started" | "hook_response" => Some(None),
                "init" => {
                    let model = value
                        .get("model")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let cwd = value
                        .get("cwd")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    Some(Some(TranscriptEvent::SystemNotice {
                        text: format!("session init: model={} cwd={}", model, cwd),
                    }))
                }
                _ => Some(Some(TranscriptEvent::SystemNotice {
                    text: format!("system: {}", subtype),
                })),
            }
        }
        "assistant" => {
            let content = value.pointer("/message/content").and_then(|v| v.as_array());
            if let Some(content) = content {
                for entry in content {
                    let entry_ty = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match entry_ty {
                        "text" => {
                            let text = entry.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            if !text.is_empty() {
                                return Some(Some(TranscriptEvent::AssistantText {
                                    text: text.to_string(),
                                }));
                            }
                        }
                        "tool_use" => {
                            let name = entry
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let args = entry.get("input").cloned().unwrap_or(Value::Null);
                            return Some(Some(TranscriptEvent::ToolCall { name, args }));
                        }
                        _ => continue,
                    }
                }
            }
            // Recognized envelope shape with no usable content: skip rather
            // than fall through to text. Emitting the raw JSON as assistant
            // text would pollute the feed with envelope noise.
            Some(None)
        }
        "user" => {
            let content = value.pointer("/message/content").and_then(|v| v.as_array());
            if let Some(content) = content {
                for entry in content {
                    if entry.get("type").and_then(|v| v.as_str()) == Some("tool_result") {
                        let name = entry
                            .get("tool_use_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let payload = entry.get("content").cloned().unwrap_or(Value::Null);
                        return Some(Some(TranscriptEvent::ToolResult { name, payload }));
                    }
                }
            }
            Some(None)
        }
        "rate_limit_event" => {
            let info = value.get("rate_limit_info").cloned().unwrap_or(Value::Null);
            Some(Some(TranscriptEvent::SystemNotice {
                text: format!("rate_limit: {}", info),
            }))
        }
        "result" => Some(Some(TranscriptEvent::Completed)),
        "error" => {
            let text = value
                .get("message")
                .or_else(|| value.get("error"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| raw.to_string());
            Some(Some(TranscriptEvent::Error { text }))
        }
        // Unknown type. Return None so the caller falls through to the text
        // heuristic, which will treat the raw line as assistant text.
        _ => None,
    }
}

// Legacy marker heuristic. Kept for `--print` (no stream-json) compatibility
// and for the unit-test stub that pipes bash echoes through the manager.
fn parse_text_line(trimmed: &str) -> Option<TranscriptEvent> {
    if let Some(rest) = strip_prefix_ci(trimmed, "[tool_call]") {
        return Some(TranscriptEvent::ToolCall {
            name: first_word(rest).unwrap_or("unknown").to_string(),
            args: Value::Null,
        });
    }
    if let Some(rest) = strip_prefix_ci(trimmed, "tool_use:") {
        return Some(TranscriptEvent::ToolCall {
            name: first_word(rest).unwrap_or("unknown").to_string(),
            args: Value::Null,
        });
    }

    if starts_with_ci(trimmed, "[error]") || starts_with_ci(trimmed, "error:") {
        return Some(TranscriptEvent::Error {
            text: trimmed.to_string(),
        });
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("permission_request") || lower.contains("awaiting_approval") {
        return Some(TranscriptEvent::SystemNotice {
            text: trimmed.to_string(),
        });
    }

    Some(TranscriptEvent::AssistantText {
        text: trimmed.to_string(),
    })
}

// Case-insensitive strip_prefix that returns the suffix on a match.
fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() < prefix.len() {
        return None;
    }
    let head = &text[..prefix.len()];
    if head.eq_ignore_ascii_case(prefix) {
        Some(&text[prefix.len()..])
    } else {
        None
    }
}

fn starts_with_ci(text: &str, prefix: &str) -> bool {
    strip_prefix_ci(text, prefix).is_some()
}

fn first_word(text: &str) -> Option<&str> {
    text.split_whitespace().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_line_returns_none() {
        assert!(parse_line("").is_none());
        assert!(parse_line("\n").is_none());
        assert!(parse_line("\r\n").is_none());
    }

    #[test]
    fn parse_text_line_returns_assistant_text() {
        match parse_line("hello world\n") {
            Some(TranscriptEvent::AssistantText { text }) => assert_eq!(text, "hello world"),
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_tool_call_marker_returns_tool_call() {
        match parse_line("[tool_call] bash --version\n") {
            Some(TranscriptEvent::ToolCall { name, .. }) => assert_eq!(name, "bash"),
            other => panic!("unexpected event: {:?}", other),
        }
        match parse_line("tool_use: read_file path/to/x\n") {
            Some(TranscriptEvent::ToolCall { name, .. }) => assert_eq!(name, "read_file"),
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_error_marker_returns_error() {
        for line in ["[error] boom", "error: boom", "Error: boom"] {
            match parse_line(line) {
                Some(TranscriptEvent::Error { .. }) => {}
                other => panic!("unexpected event for {}: {:?}", line, other),
            }
        }
    }

    #[test]
    fn parse_permission_request_returns_system_notice() {
        // Plain-text marker path. JSON path is covered by the
        // hook_lifecycle / system_init tests below.
        let line = "awaiting_approval: tool=shell";
        match parse_line(line) {
            Some(TranscriptEvent::SystemNotice { text }) => {
                assert!(text.contains("awaiting_approval"));
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    // M2.2: JSON envelope parsing against captured Claude CLI 2.1.118
    // samples. These samples were taken on Windows from
    // `claude --print --output-format stream-json --verbose "..."`.

    #[test]
    fn parse_assistant_text_json_extracts_content() {
        let line = r#"{"type":"assistant","message":{"model":"claude-opus-4-7","id":"msg_x","type":"message","role":"assistant","content":[{"type":"text","text":"hi"}],"stop_reason":null,"usage":{"input_tokens":6,"output_tokens":6}},"parent_tool_use_id":null,"session_id":"abc","uuid":"u1"}"#;
        match parse_line(line) {
            Some(TranscriptEvent::AssistantText { text }) => assert_eq!(text, "hi"),
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_assistant_tool_use_json_extracts_name_and_input() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_1","name":"Read","input":{"file_path":"/tmp/x"}}]},"session_id":"abc","uuid":"u2"}"#;
        match parse_line(line) {
            Some(TranscriptEvent::ToolCall { name, args }) => {
                assert_eq!(name, "Read");
                assert_eq!(
                    args.get("file_path").and_then(|v| v.as_str()),
                    Some("/tmp/x")
                );
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_user_tool_result_json_extracts_tool_use_id_and_content() {
        let line = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"file contents"}]},"session_id":"abc","uuid":"u3"}"#;
        match parse_line(line) {
            Some(TranscriptEvent::ToolResult { name, payload }) => {
                assert_eq!(name, "toolu_1");
                assert_eq!(payload.as_str(), Some("file contents"));
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_hook_started_returns_none() {
        let line = r#"{"type":"system","subtype":"hook_started","hook_id":"x","hook_name":"SessionStart:startup","hook_event":"SessionStart","uuid":"u","session_id":"s"}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn parse_hook_response_returns_none() {
        let line = r#"{"type":"system","subtype":"hook_response","hook_id":"x","hook_name":"SessionStart:startup","hook_event":"SessionStart","output":"","stdout":"","stderr":"","exit_code":0,"outcome":"success","uuid":"u","session_id":"s"}"#;
        assert!(parse_line(line).is_none());
    }

    #[test]
    fn parse_system_init_extracts_model_and_cwd() {
        let line = r#"{"type":"system","subtype":"init","cwd":"C:\\tmp","session_id":"s","tools":["Bash"],"mcp_servers":[],"model":"claude-opus-4-7[1m]","permissionMode":"default","slash_commands":[],"apiKeySource":"none","claude_code_version":"2.1.118","output_style":"default","agents":[],"skills":[],"plugins":[],"uuid":"u"}"#;
        match parse_line(line) {
            Some(TranscriptEvent::SystemNotice { text }) => {
                assert!(text.contains("model="));
                assert!(text.contains("cwd="));
                assert!(text.contains("claude-opus-4-7"));
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_rate_limit_event_becomes_system_notice() {
        let line = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1778922000,"rateLimitType":"five_hour"},"uuid":"u","session_id":"s"}"#;
        match parse_line(line) {
            Some(TranscriptEvent::SystemNotice { text }) => {
                assert!(text.starts_with("rate_limit:"));
                assert!(text.contains("allowed"));
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_result_event_returns_completed() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":1000,"num_turns":1,"result":"hi","stop_reason":"end_turn","session_id":"s","total_cost_usd":0.0,"uuid":"u"}"#;
        match parse_line(line) {
            Some(TranscriptEvent::Completed) => {}
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_error_event_extracts_message() {
        let line = r#"{"type":"error","message":"rate limited"}"#;
        match parse_line(line) {
            Some(TranscriptEvent::Error { text }) => assert_eq!(text, "rate limited"),
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_unknown_json_type_falls_through_to_text() {
        // Unknown type with valid JSON. The fallback should treat the raw
        // line as assistant text rather than swallowing it silently.
        let line = r#"{"type":"unknown_future_type","payload":"x"}"#;
        match parse_line(line) {
            Some(TranscriptEvent::AssistantText { text }) => {
                assert!(text.contains("unknown_future_type"));
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_malformed_json_falls_through_to_text() {
        // Looks like JSON but isn't well-formed. Should land in the text path.
        let line = r#"{"type":"oops,no_closing_brace"#;
        match parse_line(line) {
            Some(TranscriptEvent::AssistantText { text }) => {
                assert!(text.starts_with('{'));
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn parse_assistant_with_empty_content_returns_none() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[]},"session_id":"s","uuid":"u"}"#;
        // No usable entry in a recognized assistant envelope: the parser
        // skips the row so the operator's feed doesn't get raw JSON.
        assert!(parse_line(line).is_none());
    }
}
