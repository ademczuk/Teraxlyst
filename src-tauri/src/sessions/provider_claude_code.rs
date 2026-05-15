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
// M2.0 stub: every non-empty line becomes AssistantText. The empty case
// returns None so we don't write blanks to the DB.
//
// TODO(M2.1): real parsing. The Claude Code CLI emits a mix of plain text,
// tool-call markers, permission prompts, and exit status. Once we have real
// output samples we'll detect:
//   - lines starting with "tool_call:" or JSON envelopes  -> ToolCall
//   - lines starting with "tool_result:"                   -> ToolResult
//   - permission-prompt envelopes                          -> SystemNotice + a
//     follow-up MCP prompt_for_user_input round-trip (M3)
//   - non-zero exit lines / "Error:" prefix                -> Error
pub fn parse_line(line: &str) -> Option<TranscriptEvent> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() {
        return None;
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
        assert!(parse_line("\r\n").is_none());
    }

    #[test]
    fn parse_text_line_returns_assistant_text() {
        match parse_line("hello world\n") {
            Some(TranscriptEvent::AssistantText { text }) => assert_eq!(text, "hello world"),
            other => panic!("unexpected event: {:?}", other),
        }
    }
}
