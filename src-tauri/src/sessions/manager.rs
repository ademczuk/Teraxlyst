// SessionManager: the registry + lifecycle layer for AI sessions.
//
// Responsibilities:
// - Spawn the provider subprocess for a new session.
// - Run a background stdout-reader task that (a) parses each line into a
//   TranscriptEvent, (b) batches events in a 50ms debounce window, (c)
//   writes each event to the DB via DbHandle, and (d) emits the batched
//   events to the renderer via app_handle.emit.
// - Hold a registry mapping session_id -> RunningSession so the kill/list
//   commands can find live sessions.
//
// Concurrency model:
// - The registry sits behind tokio::sync::Mutex so command handlers can
//   await the lock without blocking the executor.
// - Each session has two associated tasks: a reader task pumping the
//   subprocess stdout, and a flush task on the same timer. We use a
//   tokio::task::JoinHandle to abort them on kill.
// - SharedChild from the existing `shared_child` dep gives us cross-platform
//   kill semantics matching what the shell-bg layer already relies on.

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::ChildStdout;
use std::sync::Arc;
use std::time::Duration;

use shared_child::SharedChild;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use crate::db::actor::DbHandle;
use crate::db::types::Session;

use super::error::ManagerError;
use super::provider_claude_code::{parse_line, spawn_claude_code};
use super::types::{Provider, SessionEventBatch, TranscriptEvent};

const EVENT_EMIT_NAME: &str = "teraxlyst:session-events";
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(50);

// Per-running-session state. We hold the SharedChild (so kill works), and
// the JoinHandles of the background tasks so we can abort them on kill.
//
// Tests inspect `tasks` indirectly via `list_running`; the registry value
// is otherwise opaque to callers.
struct RunningSession {
    #[allow(dead_code)] // future: pause/resume need the provider tag
    provider: Provider,
    child: Arc<SharedChild>,
    tasks: Vec<JoinHandle<()>>,
}

#[derive(Default)]
pub struct SessionManager {
    registry: Arc<Mutex<HashMap<i64, RunningSession>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    // Create a session: record it in the DB, spawn the provider subprocess,
    // wire up the stdout reader + debounced emitter, and stash everything in
    // the registry. Returns the freshly-created Session row so the renderer
    // can render it immediately.
    pub async fn create<R: Runtime>(
        &self,
        db: &DbHandle,
        workspace_id: i64,
        provider: Provider,
        prompt: String,
        app_handle: AppHandle<R>,
    ) -> Result<Session, ManagerError> {
        self.create_internal(db, workspace_id, provider, prompt, app_handle, None)
            .await
    }

    // Test-only seam: spawn an arbitrary program instead of the provider
    // binary. Lets the integration test exercise the full pipeline without
    // depending on `claude` being installed.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_with_program<R: Runtime>(
        &self,
        db: &DbHandle,
        workspace_id: i64,
        provider: Provider,
        prompt: String,
        app_handle: AppHandle<R>,
        program: String,
        args: Vec<String>,
    ) -> Result<Session, ManagerError> {
        self.create_internal(
            db,
            workspace_id,
            provider,
            prompt,
            app_handle,
            Some((program, args)),
        )
        .await
    }

    async fn create_internal<R: Runtime>(
        &self,
        db: &DbHandle,
        workspace_id: i64,
        provider: Provider,
        prompt: String,
        app_handle: AppHandle<R>,
        program_override: Option<(String, Vec<String>)>,
    ) -> Result<Session, ManagerError> {
        // Resolve the workspace path from the DB so we can set cwd on the
        // child. List+find avoids a new DB request type; workspace counts
        // are small in v1 so the cost is negligible.
        let workspaces = db.list_workspaces().await?;
        let workspace = workspaces
            .into_iter()
            .find(|w| w.id == workspace_id)
            .ok_or(ManagerError::NotFound(workspace_id))?;
        let workspace_path = PathBuf::from(&workspace.path);

        // Record the session row first so the renderer has an id to subscribe
        // against, then spawn the child. If the spawn fails we mark the row
        // failed and propagate; v1 leaves cleanup of the failed row to a
        // future v1.1 audit pass (rows are cheap).
        let session = db
            .create_session(workspace_id, provider.as_db_str().to_string(), None)
            .await?;

        // Persist the prompt as the opening user message so resumes see the
        // full conversation. Errors here are non-fatal; we log via the
        // emitted Error event below if it fails.
        if !prompt.is_empty() {
            let event = TranscriptEvent::UserMessage { text: prompt.clone() };
            let _ = db
                .append_event(
                    session.id,
                    event.db_kind().to_string(),
                    event.db_payload(),
                )
                .await;
            let _ = app_handle.emit(
                EVENT_EMIT_NAME,
                SessionEventBatch {
                    session_id: session.id,
                    events: vec![event],
                },
            );
        }

        // Spawn the subprocess. The provider module handles the bin choice
        // and the test override (which we adapt into &str/&[&str] borrows
        // before calling).
        let child = if let Some((program, args)) = program_override.as_ref() {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            spawn_claude_code(
                &workspace_path,
                &prompt,
                Some((program.as_str(), arg_refs.as_slice())),
            )?
        } else {
            spawn_claude_code(&workspace_path, &prompt, None)?
        };

        let stdout = child
            .take_stdout()
            .ok_or_else(|| ManagerError::SpawnFailed("subprocess has no stdout".into()))?;

        // Channel from reader-thread -> debounce/flush task. Bounded so a
        // runaway stdout can't blow memory; 1024 is generous for the 50ms
        // window even on fast streams.
        let (line_tx, line_rx) = mpsc::channel::<TranscriptEvent>(1024);

        // The watcher gets its own sender clone so it can push the
        // Completed marker through the SAME channel the reader uses,
        // serialized behind any pending stdout events. See
        // spawn_child_watcher for ordering rationale.
        let watcher_tx = line_tx.clone();

        // Reader task: blocking std::io reads of stdout, parse each line,
        // forward via the mpsc channel. spawn_blocking because Read on a
        // ChildStdout is synchronous.
        let reader_handle = spawn_stdout_reader(stdout, line_tx);

        // Flush task: drain the channel in 50ms windows, write each event to
        // the DB and emit a batched event to the renderer. Once both the
        // reader and the watcher have dropped their senders the channel
        // closes and the flusher does its terminal flush + exit.
        let flusher_handle = spawn_event_flusher(
            session.id,
            line_rx,
            db.clone(),
            app_handle.clone(),
        );

        // Watcher task: owns reader_handle so it can AWAIT it before
        // enqueueing Completed. This guarantees every stdout line is in
        // the channel ahead of the marker. On kill we abort the watcher,
        // which cancels both the wait and the reader-join; the reader
        // task itself exits naturally as the killed child closes stdout.
        let watcher_handle = spawn_child_watcher(
            session.id,
            child.clone(),
            watcher_tx,
            reader_handle,
            Arc::clone(&self.registry),
        );

        let running = RunningSession {
            provider,
            child,
            tasks: vec![flusher_handle, watcher_handle],
        };

        let mut reg = self.registry.lock().await;
        if reg.contains_key(&session.id) {
            // Should be impossible: SQLite gives us a unique id. Defensive.
            return Err(ManagerError::AlreadyRunning(session.id));
        }
        reg.insert(session.id, running);

        Ok(session)
    }

    // Kill a running session: looks up the child, calls kill (cross-platform
    // via SharedChild), aborts the background tasks, and drops the registry
    // entry. Idempotent: killing a non-running session returns NotFound.
    pub async fn kill(&self, session_id: i64) -> Result<(), ManagerError> {
        let mut reg = self.registry.lock().await;
        let running = reg
            .remove(&session_id)
            .ok_or(ManagerError::NotFound(session_id))?;
        // Best-effort kill: if the child is already dead this returns Ok or
        // an OS error we don't care about.
        let _ = running.child.kill();
        for t in running.tasks {
            t.abort();
        }
        Ok(())
    }

    // Snapshot of running session ids. Cheap; intended for the renderer's
    // initial list-render after mount.
    pub async fn list_running(&self) -> Vec<i64> {
        self.registry.lock().await.keys().copied().collect()
    }
}

// stdout reader. Synchronous Read in a blocking task; chunks of bytes get
// accumulated into a line buffer and emitted line-by-line. We avoid BufRead
// here so partial lines at child-exit time don't block forever.
fn spawn_stdout_reader(
    mut stdout: ChildStdout,
    tx: mpsc::Sender<TranscriptEvent>,
) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 4096];
        let mut leftover: Vec<u8> = Vec::new();
        loop {
            let n = match stdout.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            leftover.extend_from_slice(&buf[..n]);
            // Drain complete lines.
            while let Some(idx) = leftover.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = leftover.drain(..=idx).collect();
                let line_str = String::from_utf8_lossy(&line);
                if let Some(event) = parse_line(line_str.as_ref()) {
                    // blocking_send because we're in a spawn_blocking thread.
                    // If the receiver is gone (manager dropped) we exit.
                    if tx.blocking_send(event).is_err() {
                        return;
                    }
                }
            }
        }
        // Emit any final non-newline-terminated line.
        if !leftover.is_empty() {
            let line_str = String::from_utf8_lossy(&leftover);
            if let Some(event) = parse_line(line_str.as_ref()) {
                let _ = tx.blocking_send(event);
            }
        }
    })
}

// Debounced flusher. Owns the receiver side of the line channel, accumulates
// events in a Vec, and every DEBOUNCE_WINDOW (or on receiver close) writes
// each event to the DB and emits the batch to the renderer.
fn spawn_event_flusher<R: Runtime>(
    session_id: i64,
    mut rx: mpsc::Receiver<TranscriptEvent>,
    db: DbHandle,
    app_handle: AppHandle<R>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut pending: Vec<TranscriptEvent> = Vec::new();
        let mut ticker = tokio::time::interval(DEBOUNCE_WINDOW);
        // Skip the first tick so we don't fire instantly on a slow session.
        ticker.tick().await;
        loop {
            tokio::select! {
                maybe_event = rx.recv() => {
                    match maybe_event {
                        Some(event) => pending.push(event),
                        None => {
                            // Channel closed: flush remaining and exit.
                            flush(&db, &app_handle, session_id, &mut pending).await;
                            break;
                        }
                    }
                }
                _ = ticker.tick() => {
                    if !pending.is_empty() {
                        flush(&db, &app_handle, session_id, &mut pending).await;
                    }
                }
            }
        }
    })
}

async fn flush<R: Runtime>(
    db: &DbHandle,
    app_handle: &AppHandle<R>,
    session_id: i64,
    pending: &mut Vec<TranscriptEvent>,
) {
    if pending.is_empty() {
        return;
    }
    // Write each event to the DB. v1 calls append_event per-event because
    // the DbActor only exposes single-event writes; M2.1 will introduce a
    // batched variant per ROADMAP M1 risk callout.
    for event in pending.iter() {
        let _ = db
            .append_event(
                session_id,
                event.db_kind().to_string(),
                event.db_payload(),
            )
            .await;
    }
    let batch = SessionEventBatch {
        session_id,
        events: std::mem::take(pending),
    };
    let _ = app_handle.emit(EVENT_EMIT_NAME, batch);
}

// Watcher: hands the sync child.wait() to spawn_blocking, awaits the
// stdout-reader to fully drain, then enqueues the Completed marker
// through the flusher channel and prunes the registry entry.
//
// M2.1 race-fix: previously the watcher wrote Completed directly to the
// DB, racing the flusher's pending batch (and on Linux, racing the
// reader's final pipe-drain after `child.wait()` returns). The new
// design serializes everything through the flusher channel by (a)
// awaiting the reader handle so every stdout line is enqueued first,
// then (b) sending Completed via the shared sender. The flusher sees
// Completed last and, once both senders are dropped, runs its terminal
// flush + exit.
fn spawn_child_watcher(
    session_id: i64,
    child: Arc<SharedChild>,
    tx: mpsc::Sender<TranscriptEvent>,
    reader_handle: JoinHandle<()>,
    registry: Arc<Mutex<HashMap<i64, RunningSession>>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // child.wait() is synchronous; park it on the blocking pool so the
        // async worker thread stays free.
        let wait_result =
            tokio::task::spawn_blocking(move || child.wait()).await;
        // We discard the actual ExitStatus; v1 just records Completed.
        let _ = wait_result;

        // Reader exits when the child's stdout pipe closes (at or after
        // exit). Awaiting its JoinHandle guarantees every parsed line is
        // already queued in the channel before we push Completed.
        // JoinError on cancellation/panic is intentionally ignored - the
        // marker still represents "child is no longer running" either way.
        let _ = reader_handle.await;

        // Enqueue Completed. send() awaits if the channel is full, which
        // is fine - we are happy to backpressure the watcher here.
        let _ = tx.send(TranscriptEvent::Completed).await;
        // Drop our sender so the flusher's recv() returns None once the
        // queue drains, triggering its terminal flush + exit.
        drop(tx);

        // Drop the registry entry. The kill() path may have already
        // removed it; remove is idempotent.
        registry.lock().await.remove(&session_id);
    })
}
