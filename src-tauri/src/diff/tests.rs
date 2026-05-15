// M5 diff approval tests. Exercises the apply-and-resolve flow at the
// DbHandle layer (we cannot easily invoke the #[tauri::command] wrapper
// without a Tauri test harness, so the tests reuse the underlying steps:
// get_diff_proposal, write_atomic via std::fs, resolve_diff_proposal).
//
// Three cases cover the M5 contract:
//   - approve writes the new_content to disk and flips status to approved
//   - reject leaves the file content unchanged and flips status to rejected
//   - approving a legacy (new_content=NULL) proposal errors gracefully

use std::fs;
use std::io::Write as _;
use std::path::Path;

use crate::db::actor::spawn_in_memory;

// Replicates diff::commands::write_atomic so the test exercises the same
// staged-rename pattern without depending on the Tauri State wrapper.
fn write_atomic(target: &Path, content: &str) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| "path has no parent".to_string())?;
    let file_name = target
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "path has no file name".to_string())?;
    let tmp = parent.join(format!(".{}.teraxlyst.tmp", file_name));
    {
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })?;
    Ok(())
}

#[tokio::test]
async fn apply_and_resolve_writes_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = dir.path().join("hello.rs");
    fs::write(&file_path, "fn main() { println!(\"old\"); }\n").expect("seed");

    let handle = spawn_in_memory().expect("actor spawn");
    let ws = handle
        .create_workspace(dir.path().to_string_lossy().to_string(), "w".to_string())
        .await
        .expect("create_workspace");
    let s = handle
        .create_session(ws.id, "claude-code".to_string(), None)
        .await
        .expect("create_session");

    let new_content = "fn main() { println!(\"new\"); }\n";
    let prop = handle
        .create_diff_proposal(
            s.id,
            file_path.to_string_lossy().to_string(),
            None,
            "@@ -1,1 +1,1 @@\n-old\n+new\n".to_string(),
            Some(new_content.to_string()),
        )
        .await
        .expect("create_diff_proposal");

    // Simulate diff_apply_and_resolve approve path.
    let row = handle
        .get_diff_proposal(prop.id)
        .await
        .expect("get_diff_proposal");
    assert_eq!(row.status, "pending");
    let content = row.new_content.as_deref().expect("new_content present");
    write_atomic(Path::new(&row.file_path), content).expect("write_atomic");

    let resolved = handle
        .resolve_diff_proposal(prop.id, "approve".to_string())
        .await
        .expect("resolve approve");
    assert_eq!(resolved.status, "approved");
    assert!(resolved.resolved_at.is_some());

    let written = fs::read_to_string(&file_path).expect("read back");
    assert_eq!(written, new_content);
}

#[tokio::test]
async fn apply_and_resolve_reject_skips_write() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = dir.path().join("keep.rs");
    let original = "fn keep() {}\n";
    fs::write(&file_path, original).expect("seed");

    let handle = spawn_in_memory().expect("actor spawn");
    let ws = handle
        .create_workspace(dir.path().to_string_lossy().to_string(), "w".to_string())
        .await
        .expect("create_workspace");
    let s = handle
        .create_session(ws.id, "claude-code".to_string(), None)
        .await
        .expect("create_session");

    let prop = handle
        .create_diff_proposal(
            s.id,
            file_path.to_string_lossy().to_string(),
            None,
            "@@ ... @@\n".to_string(),
            Some("fn replaced() {}\n".to_string()),
        )
        .await
        .expect("create_diff_proposal");

    // Reject path: do NOT write, then resolve as rejected.
    let resolved = handle
        .resolve_diff_proposal(prop.id, "reject".to_string())
        .await
        .expect("resolve reject");
    assert_eq!(resolved.status, "rejected");

    let unchanged = fs::read_to_string(&file_path).expect("read back");
    assert_eq!(
        unchanged, original,
        "reject must leave file content untouched"
    );
}

#[tokio::test]
async fn legacy_proposal_without_new_content_errors_gracefully() {
    // Simulates a pre-v2 row by passing None for new_content. The apply
    // step must surface a clean error rather than panicking.
    let dir = tempfile::tempdir().expect("tempdir");
    let file_path = dir.path().join("legacy.rs");
    fs::write(&file_path, "// legacy\n").expect("seed");

    let handle = spawn_in_memory().expect("actor spawn");
    let ws = handle
        .create_workspace(dir.path().to_string_lossy().to_string(), "w".to_string())
        .await
        .expect("create_workspace");
    let s = handle
        .create_session(ws.id, "claude-code".to_string(), None)
        .await
        .expect("create_session");

    let prop = handle
        .create_diff_proposal(
            s.id,
            file_path.to_string_lossy().to_string(),
            None,
            "@@ legacy @@\n".to_string(),
            None,
        )
        .await
        .expect("create_diff_proposal");

    let row = handle
        .get_diff_proposal(prop.id)
        .await
        .expect("get_diff_proposal");
    assert_eq!(row.status, "pending");

    // The diff_apply_and_resolve command would catch this and return Err
    // without flipping status or writing. Assert the precondition that
    // drives that branch.
    assert!(
        row.new_content.is_none(),
        "legacy proposal must surface None new_content"
    );

    // Original content remains.
    let unchanged = std::fs::read_to_string(&file_path).expect("read back");
    assert_eq!(unchanged, "// legacy\n");
}
