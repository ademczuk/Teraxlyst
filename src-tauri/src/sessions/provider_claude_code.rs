// Claude Code provider adapter.
//
// This is a SCAFFOLD. The structure compiles and works end-to-end against a
// stub command (the tests use `bash -c "echo line1; echo line2; ..."`). The
// real `claude` / `claude-code` invocation will need to be verified once we
// test against an actual user install; see TODO below.
//
// Design boundary: this module knows how to spawn the subprocess and parse a
// single line of stdout into Option<TranscriptEvent>. It does NOT touch the
// DB, the registry, the Tauri AppHandle, or any debouncing. The manager owns
// all of that.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

use shared_child::SharedChild;

use super::error::ManagerError;
use super::types::TranscriptEvent;

// TODO(M2.1): Verify the exact CLI invocation. As of writing the assumption
// is `claude` (with `--print` for one-shot output) installed on the user's
// PATH. Alternatives observed in the wild include `claude-code` and the
// Anthropic-bundled launcher. The two failure modes we want to surface
// cleanly are (a) binary not on PATH, and (b) version with different flags.
// In M2.1 we'll probe `claude --version` on first spawn and fall back to a
// best-effort parser if the format differs.
const CLAUDE_BIN: &str = "claude";

// Spawn the Claude Code CLI as a subprocess, piping stdout/stderr so the
// manager can read line-by-line. `workspace_path` is passed as the cwd of the
// child process; the CLI is expected to operate inside that directory.
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
            // TODO(M2.1): confirm flags. `--print` is the documented one-shot
            // mode at the time of writing; if it streams properly we keep it,
            // otherwise we drop to interactive mode and feed the prompt on
            // stdin.
            c.arg("--print").arg(prompt);
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
// M2.1 heuristic (still needs validation against real Claude Code output
// samples - see M2.2 TODO): inspect known marker prefixes / substrings and
// route the line to the closest TranscriptEvent variant. Anything that
// doesn't match a marker becomes AssistantText, matching the M2.0 default.
//
// Marker rules (case-insensitive on the leading token where noted):
//   - "[tool_call]" or "tool_use:" prefix    -> ToolCall (name = first word
//     after the marker; args left empty until M2.2 introduces JSON envelopes)
//   - "[error]" / "error:" / "Error:" prefix -> Error
//   - line contains "permission_request" or "awaiting_approval" anywhere
//     -> SystemNotice (preserving the original text)
//   - empty (after trim)                     -> None (don't write blanks)
//   - anything else                          -> AssistantText
//
// TODO(M2.2): cross-check against captured Claude Code stdout samples;
// promote the JSON-envelope branches once we know the actual format. The
// permission-prompt path will round-trip through the MCP prompt pipeline
// (M3) once that bridge lands.
pub fn parse_line(line: &str) -> Option<TranscriptEvent> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() {
        return None;
    }

    // Tool-call markers. Synthesize the name from the first whitespace-
    // separated token after the prefix, falling back to "unknown".
    if let Some(rest) = strip_prefix_ci(trimmed, "[tool_call]") {
        return Some(TranscriptEvent::ToolCall {
            name: first_word(rest).unwrap_or("unknown").to_string(),
            args: serde_json::Value::Null,
        });
    }
    if let Some(rest) = strip_prefix_ci(trimmed, "tool_use:") {
        return Some(TranscriptEvent::ToolCall {
            name: first_word(rest).unwrap_or("unknown").to_string(),
            args: serde_json::Value::Null,
        });
    }

    // Error markers. Case-sensitive on "Error:" because the lowercase form
    // is matched by the "error:" branch; both routes land here.
    if starts_with_ci(trimmed, "[error]")
        || starts_with_ci(trimmed, "error:")
    {
        return Some(TranscriptEvent::Error {
            text: trimmed.to_string(),
        });
    }

    // Permission / approval prompts. Substring match because real samples
    // wrap these in JSON envelopes whose exact key path we haven't pinned
    // down yet. The whole line is preserved so the renderer can pretty-print.
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
    text.trim_start().split_whitespace().next()
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
        let line = "{\"type\":\"permission_request\",\"tool\":\"shell\"}";
        match parse_line(line) {
            Some(TranscriptEvent::SystemNotice { text }) => {
                assert!(text.contains("permission_request"));
            }
            other => panic!("unexpected event: {:?}", other),
        }
    }
}
