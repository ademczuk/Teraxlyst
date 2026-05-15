// Integration test for M1 exit criterion. Validates:
// - Migrations apply cleanly to a fresh in-memory DB.
// - 1 workspace + 10 sessions + 100 events per session (1000 events total)
//   write successfully under concurrent task pressure.
// - No SQLITE_BUSY errors propagate (the actor serializes all writes; this
//   test would catch a regression where someone added direct connection
//   access from outside the actor).
// - Per-session seqs are dense 1..=100 with no gaps and no duplicates.
// - Diff proposal create + resolve round-trip works.

use super::actor::{spawn_in_memory, DbHandle};
use std::collections::HashSet;

#[tokio::test]
async fn concurrent_event_writes_serialize_correctly() {
    let handle: DbHandle = spawn_in_memory().expect("actor should spawn");

    let ws = handle
        .create_workspace("/tmp/test-workspace".to_string(), "test".to_string())
        .await
        .expect("create_workspace");

    let mut session_ids = Vec::with_capacity(10);
    for i in 0..10 {
        let s = handle
            .create_session(
                ws.id,
                "claude-code".to_string(),
                Some(format!("session-{}", i)),
            )
            .await
            .expect("create_session");
        session_ids.push(s.id);
    }

    // Spawn 10 concurrent writer tasks (one per session, 100 events each).
    // The actor serializes them; this verifies the channel + transaction
    // layer holds up under contention.
    let mut joins = Vec::with_capacity(10);
    for sid in session_ids.iter().copied() {
        let h = handle.clone();
        joins.push(tokio::spawn(async move {
            for i in 0..100 {
                let payload = serde_json::json!({ "i": i, "session": sid });
                h.append_event(sid, "user".to_string(), payload)
                    .await
                    .expect("append_event must not fail");
            }
        }));
    }
    for j in joins {
        j.await.expect("writer task panicked");
    }

    // Verify each session has exactly 100 events, seqs are 1..=100, no gaps.
    for sid in session_ids.iter().copied() {
        let events = handle.list_events(sid, 0).await.expect("list_events");
        assert_eq!(
            events.len(),
            100,
            "session {} should have 100 events, got {}",
            sid,
            events.len()
        );
        let seqs: Vec<i64> = events.iter().map(|e| e.seq).collect();
        let expected: Vec<i64> = (1..=100).collect();
        assert_eq!(seqs, expected, "session {} has wrong seq sequence", sid);
        let unique: HashSet<i64> = seqs.iter().copied().collect();
        assert_eq!(unique.len(), 100, "session {} has duplicate seqs", sid);
    }
}

#[tokio::test]
async fn workspace_list_round_trip() {
    let handle = spawn_in_memory().expect("actor should spawn");
    handle
        .create_workspace("/a".to_string(), "alpha".to_string())
        .await
        .unwrap();
    handle
        .create_workspace("/b".to_string(), "bravo".to_string())
        .await
        .unwrap();
    let list = handle.list_workspaces().await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].name, "alpha");
    assert_eq!(list[1].name, "bravo");
}

#[tokio::test]
async fn diff_proposal_create_resolve() {
    let handle = spawn_in_memory().expect("actor should spawn");
    let ws = handle
        .create_workspace("/p".to_string(), "p".to_string())
        .await
        .unwrap();
    let s = handle
        .create_session(ws.id, "codex".to_string(), None)
        .await
        .unwrap();
    let prop = handle
        .create_diff_proposal(
            s.id,
            "src/lib.rs".to_string(),
            Some("abc123".to_string()),
            "@@ -1,1 +1,1 @@\n-old\n+new\n".to_string(),
            // M5: legacy-shape test keeps new_content None to verify the
            // create path still accepts a row without a stashed payload.
            None,
        )
        .await
        .unwrap();
    assert_eq!(prop.status, "pending");

    let pending = handle.list_pending_proposals(Some(s.id)).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, prop.id);

    let resolved = handle
        .resolve_diff_proposal(prop.id, "approve".to_string())
        .await
        .unwrap();
    assert_eq!(resolved.status, "approved");
    assert!(resolved.resolved_at.is_some());

    let pending_after = handle.list_pending_proposals(Some(s.id)).await.unwrap();
    assert!(pending_after.is_empty());

    // Resolving a non-pending proposal must return NotFound.
    let err = handle
        .resolve_diff_proposal(prop.id, "approve".to_string())
        .await;
    assert!(err.is_err(), "double-resolve should fail");
}

#[tokio::test]
async fn migrations_idempotent_on_reopen() {
    use super::migrations::run_migrations;
    use rusqlite::Connection;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let path = dir.path().join("test.sqlite");

    {
        let mut c = Connection::open(&path).unwrap();
        let v = run_migrations(&mut c).unwrap();
        assert_eq!(v, 1);
    }
    {
        let mut c = Connection::open(&path).unwrap();
        let v = run_migrations(&mut c).unwrap();
        assert_eq!(v, 1, "second run must report current version, not re-apply");
    }
}
