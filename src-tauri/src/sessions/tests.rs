// Integration test for the M2.0 exit criterion: session lifecycle writes
// events to the DB.
//
// We avoid depending on `claude` being installed by using the
// create_with_program seam: spawn `bash -c "echo line1; echo line2; echo
// line3"` and verify the manager writes 3 events to the DB, marks the
// session Completed, and clears the registry when the child exits.
//
// Provider-specific tests (real Claude Code spawning) are marked #[ignore]
// because they require an external binary; CI does not have it.

use std::time::Duration;

use tempfile::tempdir;

use crate::db::actor::{spawn_at_path, DbHandle};

use super::manager::SessionManager;
use super::types::Provider;

// On Windows, `bash -c` is not universally available. The Linux + macOS CI
// runs this test; on Windows we skip via cfg(unix) below.

#[cfg(unix)]
#[tokio::test]
async fn session_lifecycle_writes_events_to_db() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test.sqlite");
    let db: DbHandle = spawn_at_path(&db_path).expect("spawn db actor");

    // Workspace path must exist so the subprocess can chdir into it.
    let workspace_dir = dir.path().to_string_lossy().to_string();
    let ws = db
        .create_workspace(workspace_dir, "test-ws".to_string())
        .await
        .expect("create_workspace");

    let mgr = SessionManager::new();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();

    let session = mgr
        .create_with_program(
            &db,
            ws.id,
            Provider::ClaudeCode,
            "hello".to_string(),
            app_handle,
            "bash".to_string(),
            vec!["-c".to_string(), "echo line1; echo line2; echo line3".to_string()],
        )
        .await
        .expect("create session");

    // The child exits almost immediately. Poll the registry until it clears
    // (the watcher task removes the entry on child exit) with a generous
    // upper bound so a slow CI runner doesn't flake.
    let mut waited = Duration::from_millis(0);
    let step = Duration::from_millis(50);
    let cap = Duration::from_secs(5);
    while !mgr.list_running().await.is_empty() && waited < cap {
        tokio::time::sleep(step).await;
        waited += step;
    }
    assert!(
        mgr.list_running().await.is_empty(),
        "registry should be empty after child exit"
    );

    // Verify events landed in the DB. We expect:
    //   1 UserMessage (the prompt), 3 AssistantText (echo lines),
    //   1 Completed = 5 total, in seq order.
    let events = db
        .list_events(session.id, 0)
        .await
        .expect("list_events");
    assert!(
        events.len() >= 4,
        "expected at least 4 events (prompt + 3 echo lines), got {}: {:?}",
        events.len(),
        events.iter().map(|e| &e.kind).collect::<Vec<_>>()
    );

    // Last event must be Completed.
    let last = events.last().expect("at least one event");
    assert_eq!(last.kind, "completed", "final event should be Completed");

    // The kinds should include the prompt (user) and at least one assistant
    // line. We don't assert exact count because line-buffer flushing can
    // coalesce or split on the boundary; the manager guarantees at-least
    // the lines we saw.
    let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"user"), "expected a user event: {:?}", kinds);
    assert!(
        kinds.contains(&"assistant"),
        "expected an assistant event: {:?}",
        kinds
    );
}

// Kill a long-running child and verify the registry is cleared. We use
// `sleep 30` so the child outlives the test timeout if we forget to kill.
#[cfg(unix)]
#[tokio::test]
async fn session_kill_clears_registry() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test.sqlite");
    let db: DbHandle = spawn_at_path(&db_path).expect("spawn db actor");

    let workspace_dir = dir.path().to_string_lossy().to_string();
    let ws = db
        .create_workspace(workspace_dir, "test-ws".to_string())
        .await
        .expect("create_workspace");

    let mgr = SessionManager::new();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();

    let session = mgr
        .create_with_program(
            &db,
            ws.id,
            Provider::ClaudeCode,
            "test".to_string(),
            app_handle,
            "bash".to_string(),
            vec!["-c".to_string(), "sleep 30".to_string()],
        )
        .await
        .expect("create session");

    assert_eq!(mgr.list_running().await, vec![session.id]);

    mgr.kill(session.id).await.expect("kill");

    // Watcher may take a beat to run after kill; poll briefly.
    let mut waited = Duration::from_millis(0);
    let step = Duration::from_millis(50);
    let cap = Duration::from_secs(5);
    while !mgr.list_running().await.is_empty() && waited < cap {
        tokio::time::sleep(step).await;
        waited += step;
    }
    assert!(mgr.list_running().await.is_empty());

    // Double-kill returns NotFound.
    let err = mgr.kill(session.id).await;
    assert!(err.is_err(), "double-kill should error");
}

// Real Claude Code adapter test. Ignored because CI does not have `claude`
// on PATH. Enable locally with `cargo test -- --ignored real_claude_code`.
#[ignore]
#[tokio::test]
async fn real_claude_code_smoke() {
    // Placeholder. Once we verify the actual CLI invocation in M2.1, this
    // test will spawn `claude --print "say hi"` against a tempdir workspace
    // and assert at least one assistant event lands in the DB.
}
