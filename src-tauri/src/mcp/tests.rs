// MCP host integration tests.
//
// We mock the emitters so we don't have to spin up a real Tauri webview.
// The two pipelines (prompts, diffs) are exercised end-to-end:
//   1. tool call -> emit observed by mock -> renderer command resolves it
//   2. propose_diff approval round trip writes to disk

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::json;
use tempfile::tempdir;

use crate::db::actor::{spawn_in_memory, DbHandle};
use crate::mcp::diff_pipeline::{DiffEmitter, PendingDiffs};
use crate::mcp::error::McpError;
use crate::mcp::prompt_pipeline::{PendingPrompts, PromptEmitter};
use crate::mcp::tools::TeraxlystToolset;
use crate::mcp::types::{DiffRequest, DiffStatus, PromptField, PromptRequest};

#[derive(Default)]
struct MockPromptEmitter {
    seen: Mutex<Vec<PromptRequest>>,
}

impl PromptEmitter for MockPromptEmitter {
    fn emit_prompt(&self, payload: &PromptRequest) -> Result<(), McpError> {
        let mut g = self.seen.lock().expect("lock");
        g.push(payload.clone());
        Ok(())
    }
}

#[derive(Default)]
struct MockDiffEmitter {
    seen: Mutex<Vec<DiffRequest>>,
}

impl DiffEmitter for MockDiffEmitter {
    fn emit_diff(&self, payload: &DiffRequest) -> Result<(), McpError> {
        let mut g = self.seen.lock().expect("lock");
        g.push(payload.clone());
        Ok(())
    }
}

fn build_toolset(db: DbHandle) -> (TeraxlystToolset, Arc<MockPromptEmitter>, Arc<MockDiffEmitter>) {
    let pe = Arc::new(MockPromptEmitter::default());
    let de = Arc::new(MockDiffEmitter::default());
    let prompts = PendingPrompts::new(pe.clone());
    let diffs = PendingDiffs::new(de.clone(), db);
    (TeraxlystToolset::new(prompts, diffs), pe, de)
}

#[tokio::test]
async fn prompt_pipeline_round_trip() {
    let db = spawn_in_memory().expect("spawn db");
    let (toolset, pe, _) = build_toolset(db);

    // Spawn the tool call. It will park on the oneshot until we deliver a
    // renderer reply.
    let toolset_clone = toolset.clone();
    let join = tokio::spawn(async move {
        toolset_clone
            .do_prompt_for_user_input(
                "Pick a fruit".into(),
                PromptField::SingleSelect {
                    label: "Fruit".into(),
                    options: vec!["apple".into(), "pear".into()],
                },
            )
            .await
    });

    // Give the spawn a tick to register the pending entry + emit.
    let request = wait_for(|| {
        pe.seen.lock().ok().and_then(|g| g.first().cloned())
    })
    .await
    .expect("emit observed");

    // Simulate the renderer reply.
    assert!(toolset.prompts.resolve(&request.id, json!("apple")).await);

    let value = join.await.expect("join").expect("tool ok");
    assert_eq!(value, json!("apple"));
}

#[tokio::test]
async fn propose_diff_approved_writes_file() {
    let db = spawn_in_memory().expect("spawn db");
    let (toolset, _, de) = build_toolset(db.clone());

    // Need a workspace + session to satisfy diff_proposals FK.
    let ws = db
        .create_workspace("ws".into(), "test".into())
        .await
        .expect("ws");
    let session = db
        .create_session(ws.id, "claude-code".into(), None)
        .await
        .expect("session");

    let dir = tempdir().expect("tempdir");
    let target: PathBuf = dir.path().join("hello.txt");
    let target_str = target.to_string_lossy().to_string();

    let toolset_clone = toolset.clone();
    let join = tokio::spawn(async move {
        toolset_clone
            .do_propose_diff(session.id, target_str.clone(), "hi from agent\n".into())
            .await
    });

    let request = wait_for(|| de.seen.lock().ok().and_then(|g| g.first().cloned()))
        .await
        .expect("emit observed");

    // Simulate user clicking Approve.
    toolset
        .diffs
        .resolve(&request.id, "approve")
        .await
        .expect("resolve");

    let res = join.await.expect("join").expect("tool ok");
    assert!(matches!(res.status, DiffStatus::Approved));

    // File should now exist with the expected bytes.
    let read = tokio::fs::read_to_string(&target).await.expect("read");
    assert_eq!(read, "hi from agent\n");
}

#[tokio::test]
async fn propose_diff_rejected_leaves_file_alone() {
    let db = spawn_in_memory().expect("spawn db");
    let (toolset, _, de) = build_toolset(db.clone());

    let ws = db
        .create_workspace("ws".into(), "test".into())
        .await
        .expect("ws");
    let session = db
        .create_session(ws.id, "claude-code".into(), None)
        .await
        .expect("session");

    let dir = tempdir().expect("tempdir");
    let target: PathBuf = dir.path().join("nope.txt");
    let target_str = target.to_string_lossy().to_string();

    let toolset_clone = toolset.clone();
    let join = tokio::spawn(async move {
        toolset_clone
            .do_propose_diff(session.id, target_str, "would have written\n".into())
            .await
    });

    let request = wait_for(|| de.seen.lock().ok().and_then(|g| g.first().cloned()))
        .await
        .expect("emit observed");

    toolset
        .diffs
        .resolve(&request.id, "reject")
        .await
        .expect("resolve");

    let res = join.await.expect("join").expect("tool ok");
    assert!(matches!(res.status, DiffStatus::Rejected));
    assert!(!target.exists());
}

#[tokio::test]
async fn read_workspace_file_rejects_parent_traversal() {
    let db = spawn_in_memory().expect("spawn db");
    let (toolset, _, _) = build_toolset(db);

    let dir = tempdir().expect("tempdir");
    let workspace = dir.path().to_string_lossy().to_string();

    let err = toolset
        .do_read_workspace_file(&workspace, "../secret.txt")
        .await
        .expect_err("must reject ../");
    assert!(matches!(err, McpError::PathEscape(_)));
}

#[tokio::test]
async fn read_workspace_file_reads_inside_workspace() {
    let db = spawn_in_memory().expect("spawn db");
    let (toolset, _, _) = build_toolset(db);

    let dir = tempdir().expect("tempdir");
    let inner = dir.path().join("notes.md");
    tokio::fs::write(&inner, "# hi\n").await.expect("write");

    let workspace = dir.path().to_string_lossy().to_string();
    let body = toolset
        .do_read_workspace_file(&workspace, "notes.md")
        .await
        .expect("ok");
    assert_eq!(body, "# hi\n");
}

// Small helper: poll the closure every 5ms up to ~500ms until it returns
// Some. Used for "wait until the spawned tool task fires its emit."
async fn wait_for<T, F>(mut f: F) -> Option<T>
where
    F: FnMut() -> Option<T>,
{
    for _ in 0..100 {
        if let Some(v) = f() {
            return Some(v);
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    None
}
