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

// On Windows, `bash -c` is not universally available and
// `tauri::test::mock_app` fails to load even with WebView2Loader.dll
// next to the test binary. All integration tests here are cfg(unix);
// the imports they need are gated the same way to avoid dead-code
// warnings on the Windows test build.
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use tempfile::tempdir;

#[cfg(unix)]
use crate::db::actor::{spawn_at_path, DbHandle};

#[cfg(unix)]
use super::manager::SessionManager;
#[cfg(unix)]
use super::types::Provider;

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

    // The child exits almost immediately. Poll the registry until it clears.
    // The watcher task removes the entry after the child reaper returns.
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

    // After the registry clears, give the writer task a beat to drain any
    // queued events + the Completed marker. The flusher's 50ms tick + the
    // watcher's enqueue is not synchronized with the registry prune, so a
    // short additional poll is needed.
    let mut events = Vec::new();
    let mut drained = Duration::from_millis(0);
    while drained < Duration::from_secs(2) {
        events = db
            .list_events(session.id, 0)
            .await
            .expect("list_events");
        if events.iter().any(|e| e.kind == "completed") {
            break;
        }
        tokio::time::sleep(step).await;
        drained += step;
    }

    // M2.1: with the watcher pushing Completed through the flusher
    // channel instead of writing directly, the marker is deterministic.
    // We require the prompt (UserMessage) plus at least one stdout line
    // (AssistantText) and the Completed marker as the LAST event.
    assert!(
        events.len() >= 3,
        "expected >=3 events (prompt + >=1 echo line + completed), got {}: {:?}",
        events.len(),
        events.iter().map(|e| &e.kind).collect::<Vec<_>>()
    );
    let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"user"), "expected a user event: {:?}", kinds);
    assert!(
        kinds.contains(&"assistant"),
        "expected an assistant event: {:?}",
        kinds
    );
    assert_eq!(
        kinds.last().copied(),
        Some("completed"),
        "Completed marker must be the last event: {:?}",
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
// on PATH (and running it would burn credits). Enable locally with:
//   cargo test --lib --locked -- --ignored real_claude_code
//
// cfg(unix)-gated: `tauri::test::mock_app` on Windows fails to load even
// with WebView2Loader.dll copied to target/debug/deps - the failure is a
// STATUS_ENTRYPOINT_NOT_FOUND deeper in the wry/webview2-com chain. The
// JSON parser is verified by 11 unit tests against captured Windows
// stream-json samples, so this integration test is a nice-to-have rather
// than a gating concern.
#[cfg(unix)]
#[ignore]
#[tokio::test]
async fn real_claude_code_smoke() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test.sqlite");
    let db: DbHandle = spawn_at_path(&db_path).expect("spawn db actor");

    let workspace_dir = dir.path().to_string_lossy().to_string();
    let ws = db
        .create_workspace(workspace_dir, "claude-smoke".to_string())
        .await
        .expect("create_workspace");

    let mgr = SessionManager::new();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();

    // Use the same flag set the real provider uses (verified against CLI
    // 2.1.118). The test seam takes the program + args directly because the
    // default code path resolves `claude` from PATH and the test workspace
    // may not have it - on Windows the binary is typically under
    // C:\Users\<user>\.local\bin\claude.exe. We let the seam resolve via
    // PATH by passing just "claude" as the program name.
    let session = mgr
        .create_with_program(
            &db,
            ws.id,
            Provider::ClaudeCode,
            "say hi".to_string(),
            app_handle,
            "claude".to_string(),
            vec![
                "--print".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--verbose".to_string(),
                "respond with exactly: hi".to_string(),
            ],
        )
        .await
        .expect("create session");

    // Real `claude` takes ~5-15s depending on cache warmth + model latency.
    // Poll up to 60s for the registry to clear.
    let mut waited = Duration::from_millis(0);
    let step = Duration::from_millis(200);
    let cap = Duration::from_secs(60);
    while !mgr.list_running().await.is_empty() && waited < cap {
        tokio::time::sleep(step).await;
        waited += step;
    }
    assert!(
        mgr.list_running().await.is_empty(),
        "registry should be empty after claude exit (waited {:?})",
        waited
    );

    // Drain any post-exit events.
    let mut events = Vec::new();
    let mut drained = Duration::from_millis(0);
    while drained < Duration::from_secs(3) {
        events = db.list_events(session.id, 0).await.expect("list_events");
        if events.iter().any(|e| e.kind == "completed") {
            break;
        }
        tokio::time::sleep(step).await;
        drained += step;
    }

    let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
    eprintln!("real_claude_code_smoke event kinds: {:?}", kinds);
    assert!(
        kinds.contains(&"user"),
        "expected the prompt as a user event: {:?}",
        kinds
    );
    assert!(
        kinds.contains(&"assistant"),
        "expected an assistant event from real CLI output: {:?}",
        kinds
    );
    assert!(
        kinds.contains(&"completed"),
        "expected a completed marker (from JSON result event): {:?}",
        kinds
    );
}
