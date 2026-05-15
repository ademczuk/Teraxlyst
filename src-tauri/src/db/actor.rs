// DbActor: single-writer SQLite access via mpsc + oneshot.
//
// Concurrency model:
// - Exactly one rusqlite Connection lives inside the actor task. All writes
//   (and v1 reads) go through it. The actor task runs on a dedicated OS
//   thread via tokio::task::spawn_blocking so its blocking sqlite calls
//   do not stall the tokio runtime.
// - Callers hold a DbHandle (cheaply cloneable mpsc::Sender). They build
//   a DbRequest, attach a oneshot::Sender for the reply, and await it.
// - v1 ships a single connection. A reader pool is a v1.1 optimization
//   per the milestone scope; the architecture doc calls it out but we
//   keep v1 minimal.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use tokio::sync::{mpsc, oneshot};

use super::error::DbError;
use super::migrations;
use super::types::{DiffProposal, Session, SessionEvent, Tracker, TrackerItem, Workspace};

const CHANNEL_CAPACITY: usize = 256;

type Reply<T> = oneshot::Sender<Result<T, DbError>>;

pub enum DbRequest {
    CreateWorkspace {
        path: String,
        name: String,
        reply: Reply<Workspace>,
    },
    ListWorkspaces {
        reply: Reply<Vec<Workspace>>,
    },
    CreateSession {
        workspace_id: i64,
        provider: String,
        title: Option<String>,
        reply: Reply<Session>,
    },
    AppendEvent {
        session_id: i64,
        kind: String,
        payload: serde_json::Value,
        reply: Reply<SessionEvent>,
    },
    ListEvents {
        session_id: i64,
        since_seq: i64,
        reply: Reply<Vec<SessionEvent>>,
    },
    CreateDiffProposal {
        session_id: i64,
        file_path: String,
        base_hash: Option<String>,
        patch: String,
        reply: Reply<DiffProposal>,
    },
    ResolveDiffProposal {
        id: i64,
        action: String,
        reply: Reply<DiffProposal>,
    },
    ListPendingProposals {
        session_id: Option<i64>,
        reply: Reply<Vec<DiffProposal>>,
    },
}

#[derive(Clone)]
pub struct DbHandle {
    tx: mpsc::Sender<DbRequest>,
}

impl DbHandle {
    pub async fn create_workspace(
        &self,
        path: String,
        name: String,
    ) -> Result<Workspace, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::CreateWorkspace { path, name, reply }).await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn list_workspaces(&self) -> Result<Vec<Workspace>, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::ListWorkspaces { reply }).await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn create_session(
        &self,
        workspace_id: i64,
        provider: String,
        title: Option<String>,
    ) -> Result<Session, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::CreateSession {
            workspace_id,
            provider,
            title,
            reply,
        })
        .await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn append_event(
        &self,
        session_id: i64,
        kind: String,
        payload: serde_json::Value,
    ) -> Result<SessionEvent, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::AppendEvent {
            session_id,
            kind,
            payload,
            reply,
        })
        .await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn list_events(
        &self,
        session_id: i64,
        since_seq: i64,
    ) -> Result<Vec<SessionEvent>, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::ListEvents {
            session_id,
            since_seq,
            reply,
        })
        .await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn create_diff_proposal(
        &self,
        session_id: i64,
        file_path: String,
        base_hash: Option<String>,
        patch: String,
    ) -> Result<DiffProposal, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::CreateDiffProposal {
            session_id,
            file_path,
            base_hash,
            patch,
            reply,
        })
        .await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn resolve_diff_proposal(
        &self,
        id: i64,
        action: String,
    ) -> Result<DiffProposal, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::ResolveDiffProposal { id, action, reply }).await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn list_pending_proposals(
        &self,
        session_id: Option<i64>,
    ) -> Result<Vec<DiffProposal>, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::ListPendingProposals { session_id, reply }).await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    async fn send(&self, req: DbRequest) -> Result<(), DbError> {
        self.tx.send(req).await.map_err(|_| DbError::ActorClosed)
    }
}

// Open a connection at the given filesystem path, run migrations, then spawn
// the actor on a dedicated blocking thread. Returns a handle for command
// dispatch.
pub fn spawn_at_path(db_path: &Path) -> Result<DbHandle, DbError> {
    let mut conn = Connection::open(db_path)?;
    migrations::run_migrations(&mut conn)?;
    Ok(spawn_with_connection(conn))
}

// In-memory variant used by tests. The schema must be created on the same
// connection that the actor takes ownership of, because each in-memory
// SQLite handle has its own database.
pub fn spawn_in_memory() -> Result<DbHandle, DbError> {
    let mut conn = Connection::open_in_memory()?;
    migrations::run_migrations(&mut conn)?;
    Ok(spawn_with_connection(conn))
}

fn spawn_with_connection(conn: Connection) -> DbHandle {
    let (tx, rx) = mpsc::channel::<DbRequest>(CHANNEL_CAPACITY);
    // The actor body is fully synchronous (rusqlite is blocking). Park it on
    // a dedicated blocking thread so the tokio runtime stays responsive.
    tokio::task::spawn_blocking(move || {
        run_actor(conn, rx);
    });
    DbHandle { tx }
}

fn run_actor(mut conn: Connection, mut rx: mpsc::Receiver<DbRequest>) {
    // Pragmas were set during migrations on this same connection; no need
    // to re-apply.
    while let Some(req) = rx.blocking_recv() {
        match req {
            DbRequest::CreateWorkspace { path, name, reply } => {
                let _ = reply.send(handle_create_workspace(&conn, &path, &name));
            }
            DbRequest::ListWorkspaces { reply } => {
                let _ = reply.send(handle_list_workspaces(&conn));
            }
            DbRequest::CreateSession {
                workspace_id,
                provider,
                title,
                reply,
            } => {
                let _ = reply.send(handle_create_session(
                    &conn,
                    workspace_id,
                    &provider,
                    title.as_deref(),
                ));
            }
            DbRequest::AppendEvent {
                session_id,
                kind,
                payload,
                reply,
            } => {
                let _ = reply.send(handle_append_event(
                    &mut conn, session_id, &kind, &payload,
                ));
            }
            DbRequest::ListEvents {
                session_id,
                since_seq,
                reply,
            } => {
                let _ = reply.send(handle_list_events(&conn, session_id, since_seq));
            }
            DbRequest::CreateDiffProposal {
                session_id,
                file_path,
                base_hash,
                patch,
                reply,
            } => {
                let _ = reply.send(handle_create_diff_proposal(
                    &conn,
                    session_id,
                    &file_path,
                    base_hash.as_deref(),
                    &patch,
                ));
            }
            DbRequest::ResolveDiffProposal { id, action, reply } => {
                let _ = reply.send(handle_resolve_diff_proposal(&conn, id, &action));
            }
            DbRequest::ListPendingProposals { session_id, reply } => {
                let _ = reply.send(handle_list_pending_proposals(&conn, session_id));
            }
        }
    }
}

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn handle_create_workspace(
    conn: &Connection,
    path: &str,
    name: &str,
) -> Result<Workspace, DbError> {
    let now = now_millis();
    conn.execute(
        "INSERT INTO workspaces (path, name, created_at) VALUES (?1, ?2, ?3)",
        params![path, name, now],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Workspace {
        id,
        path: path.to_string(),
        name: name.to_string(),
        created_at: now,
    })
}

fn handle_list_workspaces(conn: &Connection) -> Result<Vec<Workspace>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, path, name, created_at FROM workspaces ORDER BY created_at ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                path: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn handle_create_session(
    conn: &Connection,
    workspace_id: i64,
    provider: &str,
    title: Option<&str>,
) -> Result<Session, DbError> {
    let now = now_millis();
    let status = "running";
    conn.execute(
        "INSERT INTO sessions (workspace_id, parent_id, provider, title, status, created_at, updated_at) \
         VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?5)",
        params![workspace_id, provider, title, status, now],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Session {
        id,
        workspace_id,
        parent_id: None,
        provider: provider.to_string(),
        title: title.map(|s| s.to_string()),
        status: status.to_string(),
        created_at: now,
        updated_at: now,
    })
}

fn handle_append_event(
    conn: &mut Connection,
    session_id: i64,
    kind: &str,
    payload: &serde_json::Value,
) -> Result<SessionEvent, DbError> {
    // Compute next seq atomically inside a transaction so concurrent
    // AppendEvent calls (which are serialized through the actor anyway,
    // but a future reader pool would not be) cannot duplicate seq values.
    let tx = conn.transaction()?;
    let next_seq: i64 = tx.query_row(
        "SELECT COALESCE(MAX(seq), 0) + 1 FROM session_events WHERE session_id = ?1",
        params![session_id],
        |row| row.get(0),
    )?;
    let now = now_millis();
    let payload_str = serde_json::to_string(payload)?;
    tx.execute(
        "INSERT INTO session_events (session_id, seq, kind, payload, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![session_id, next_seq, kind, payload_str, now],
    )?;
    let id = tx.last_insert_rowid();
    // Bump the session's updated_at so listings reflect recent activity.
    tx.execute(
        "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
        params![now, session_id],
    )?;
    tx.commit()?;
    Ok(SessionEvent {
        id,
        session_id,
        seq: next_seq,
        kind: kind.to_string(),
        payload: payload.clone(),
        created_at: now,
    })
}

fn handle_list_events(
    conn: &Connection,
    session_id: i64,
    since_seq: i64,
) -> Result<Vec<SessionEvent>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, seq, kind, payload, created_at \
         FROM session_events \
         WHERE session_id = ?1 AND seq > ?2 \
         ORDER BY seq ASC",
    )?;
    let rows = stmt
        .query_map(params![session_id, since_seq], |row| {
            let payload_str: String = row.get(4)?;
            let payload = serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
            Ok(SessionEvent {
                id: row.get(0)?,
                session_id: row.get(1)?,
                seq: row.get(2)?,
                kind: row.get(3)?,
                payload,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn handle_create_diff_proposal(
    conn: &Connection,
    session_id: i64,
    file_path: &str,
    base_hash: Option<&str>,
    patch: &str,
) -> Result<DiffProposal, DbError> {
    let now = now_millis();
    let status = "pending";
    conn.execute(
        "INSERT INTO diff_proposals (session_id, file_path, base_hash, patch, status, created_at, resolved_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
        params![session_id, file_path, base_hash, patch, status, now],
    )?;
    let id = conn.last_insert_rowid();
    Ok(DiffProposal {
        id,
        session_id,
        file_path: file_path.to_string(),
        base_hash: base_hash.map(|s| s.to_string()),
        patch: patch.to_string(),
        status: status.to_string(),
        created_at: now,
        resolved_at: None,
    })
}

fn handle_resolve_diff_proposal(
    conn: &Connection,
    id: i64,
    action: &str,
) -> Result<DiffProposal, DbError> {
    let new_status = match action {
        "approve" => "approved",
        "reject" => "rejected",
        other => {
            return Err(DbError::Invalid(format!(
                "unknown resolve action: {}",
                other
            )));
        }
    };
    let now = now_millis();
    let updated = conn.execute(
        "UPDATE diff_proposals SET status = ?1, resolved_at = ?2 \
         WHERE id = ?3 AND status = 'pending'",
        params![new_status, now, id],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound(format!(
            "no pending diff_proposal with id {}",
            id
        )));
    }
    let row = conn
        .query_row(
            "SELECT id, session_id, file_path, base_hash, patch, status, created_at, resolved_at \
             FROM diff_proposals WHERE id = ?1",
            params![id],
            |row| {
                Ok(DiffProposal {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    file_path: row.get(2)?,
                    base_hash: row.get(3)?,
                    patch: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    resolved_at: row.get(7)?,
                })
            },
        )
        .optional()?;
    row.ok_or_else(|| DbError::NotFound(format!("diff_proposal {} disappeared", id)))
}

fn handle_list_pending_proposals(
    conn: &Connection,
    session_id: Option<i64>,
) -> Result<Vec<DiffProposal>, DbError> {
    let query = match session_id {
        Some(_) => {
            "SELECT id, session_id, file_path, base_hash, patch, status, created_at, resolved_at \
             FROM diff_proposals \
             WHERE status = 'pending' AND session_id = ?1 \
             ORDER BY created_at ASC"
        }
        None => {
            "SELECT id, session_id, file_path, base_hash, patch, status, created_at, resolved_at \
             FROM diff_proposals \
             WHERE status = 'pending' \
             ORDER BY created_at ASC"
        }
    };
    let mut stmt = conn.prepare(query)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(DiffProposal {
            id: row.get(0)?,
            session_id: row.get(1)?,
            file_path: row.get(2)?,
            base_hash: row.get(3)?,
            patch: row.get(4)?,
            status: row.get(5)?,
            created_at: row.get(6)?,
            resolved_at: row.get(7)?,
        })
    };
    let rows = if let Some(sid) = session_id {
        stmt.query_map(params![sid], map_row)?
            .collect::<Result<Vec<_>, _>>()?
    } else {
        stmt.query_map([], map_row)?.collect::<Result<Vec<_>, _>>()?
    };
    Ok(rows)
}

// Silence dead-code warnings for types we ship but don't yet use in v1.
// Trackers + TrackerItems are part of the schema (M4) but aren't surfaced
// via DbRequest variants in M1; keeping them in types.rs documents the
// shape we'll consume in M4.
#[allow(dead_code)]
fn _ensure_unused_types_compile(_: Tracker, _: TrackerItem) {}
