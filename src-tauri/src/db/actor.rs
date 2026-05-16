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
        new_content: Option<String>,
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
    GetDiffProposal {
        id: i64,
        reply: Reply<DiffProposal>,
    },
    // M4: tracker CRUD. Additive only; existing variants are unchanged.
    UpsertTracker {
        workspace_id: i64,
        name: String,
        yaml_source: String,
        schema_json: String,
        reply: Reply<Tracker>,
    },
    // Used by tests + future M4.2 tracker UI workspace switcher.
    #[allow(dead_code)]
    ListTrackers {
        workspace_id: i64,
        reply: Reply<Vec<Tracker>>,
    },
    GetTrackerByName {
        workspace_id: i64,
        name: String,
        reply: Reply<Option<Tracker>>,
    },
    CreateTrackerItem {
        tracker_id: i64,
        public_id: String,
        status: Option<String>,
        fields_json: String,
        reply: Reply<TrackerItem>,
    },
    UpdateTrackerItem {
        id: i64,
        status: Option<String>,
        fields_json: String,
        reply: Reply<TrackerItem>,
    },
    ListTrackerItems {
        tracker_id: i64,
        status_filter: Option<String>,
        reply: Reply<Vec<TrackerItem>>,
    },
    ListTrackerItemIds {
        tracker_id: i64,
        reply: Reply<Vec<String>>,
    },
    GetTrackerById {
        tracker_id: i64,
        reply: Reply<Option<Tracker>>,
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
        new_content: Option<String>,
    ) -> Result<DiffProposal, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::CreateDiffProposal {
            session_id,
            file_path,
            base_hash,
            patch,
            new_content,
            reply,
        })
        .await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn get_diff_proposal(&self, id: i64) -> Result<DiffProposal, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::GetDiffProposal { id, reply }).await?;
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

    // ---- M4: tracker CRUD ----

    pub async fn upsert_tracker(
        &self,
        workspace_id: i64,
        name: String,
        yaml_source: String,
        schema_json: String,
    ) -> Result<Tracker, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::UpsertTracker {
            workspace_id,
            name,
            yaml_source,
            schema_json,
            reply,
        })
        .await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    // Used by tests + future M4.2 tracker UI workspace switcher.
    #[allow(dead_code)]
    pub async fn list_trackers(&self, workspace_id: i64) -> Result<Vec<Tracker>, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::ListTrackers { workspace_id, reply }).await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn get_tracker_by_name(
        &self,
        workspace_id: i64,
        name: String,
    ) -> Result<Option<Tracker>, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::GetTrackerByName {
            workspace_id,
            name,
            reply,
        })
        .await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn create_tracker_item(
        &self,
        tracker_id: i64,
        public_id: String,
        status: Option<String>,
        fields_json: String,
    ) -> Result<TrackerItem, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::CreateTrackerItem {
            tracker_id,
            public_id,
            status,
            fields_json,
            reply,
        })
        .await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn update_tracker_item(
        &self,
        id: i64,
        status: Option<String>,
        fields_json: String,
    ) -> Result<TrackerItem, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::UpdateTrackerItem {
            id,
            status,
            fields_json,
            reply,
        })
        .await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn list_tracker_items(
        &self,
        tracker_id: i64,
        status_filter: Option<String>,
    ) -> Result<Vec<TrackerItem>, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::ListTrackerItems {
            tracker_id,
            status_filter,
            reply,
        })
        .await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn list_tracker_item_ids(
        &self,
        tracker_id: i64,
    ) -> Result<Vec<String>, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::ListTrackerItemIds { tracker_id, reply }).await?;
        rx.await.map_err(|_| DbError::ActorClosed)?
    }

    pub async fn get_tracker_by_id(
        &self,
        tracker_id: i64,
    ) -> Result<Option<Tracker>, DbError> {
        let (reply, rx) = oneshot::channel();
        self.send(DbRequest::GetTrackerById { tracker_id, reply }).await?;
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
#[cfg(test)]
pub fn spawn_in_memory() -> Result<DbHandle, DbError> {
    let mut conn = Connection::open_in_memory()?;
    migrations::run_migrations(&mut conn)?;
    Ok(spawn_with_connection(conn))
}

fn spawn_with_connection(conn: Connection) -> DbHandle {
    let (tx, rx) = mpsc::channel::<DbRequest>(CHANNEL_CAPACITY);
    // The actor body is fully synchronous (rusqlite is blocking) and
    // `mpsc::Receiver::blocking_recv` does not require a tokio runtime
    // context. Use a plain OS thread so the spawn works during Tauri's
    // setup hook (which runs before the runtime is up) as well as inside
    // tokio tests.
    std::thread::Builder::new()
        .name("teraxlyst-db-actor".to_string())
        .spawn(move || {
            run_actor(conn, rx);
        })
        .expect("spawn db actor thread");
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
                new_content,
                reply,
            } => {
                let _ = reply.send(handle_create_diff_proposal(
                    &conn,
                    session_id,
                    &file_path,
                    base_hash.as_deref(),
                    &patch,
                    new_content.as_deref(),
                ));
            }
            DbRequest::ResolveDiffProposal { id, action, reply } => {
                let _ = reply.send(handle_resolve_diff_proposal(&conn, id, &action));
            }
            DbRequest::ListPendingProposals { session_id, reply } => {
                let _ = reply.send(handle_list_pending_proposals(&conn, session_id));
            }
            DbRequest::GetDiffProposal { id, reply } => {
                let _ = reply.send(handle_get_diff_proposal(&conn, id));
            }
            // ---- M4 ----
            DbRequest::UpsertTracker {
                workspace_id,
                name,
                yaml_source,
                schema_json,
                reply,
            } => {
                let _ = reply.send(handle_upsert_tracker(
                    &conn,
                    workspace_id,
                    &name,
                    &yaml_source,
                    &schema_json,
                ));
            }
            DbRequest::ListTrackers {
                workspace_id,
                reply,
            } => {
                let _ = reply.send(handle_list_trackers(&conn, workspace_id));
            }
            DbRequest::GetTrackerByName {
                workspace_id,
                name,
                reply,
            } => {
                let _ = reply.send(handle_get_tracker_by_name(&conn, workspace_id, &name));
            }
            DbRequest::CreateTrackerItem {
                tracker_id,
                public_id,
                status,
                fields_json,
                reply,
            } => {
                let _ = reply.send(handle_create_tracker_item(
                    &conn,
                    tracker_id,
                    &public_id,
                    status.as_deref(),
                    &fields_json,
                ));
            }
            DbRequest::UpdateTrackerItem {
                id,
                status,
                fields_json,
                reply,
            } => {
                let _ = reply.send(handle_update_tracker_item(
                    &conn,
                    id,
                    status.as_deref(),
                    &fields_json,
                ));
            }
            DbRequest::ListTrackerItems {
                tracker_id,
                status_filter,
                reply,
            } => {
                let _ = reply.send(handle_list_tracker_items(
                    &conn,
                    tracker_id,
                    status_filter.as_deref(),
                ));
            }
            DbRequest::ListTrackerItemIds { tracker_id, reply } => {
                let _ = reply.send(handle_list_tracker_item_ids(&conn, tracker_id));
            }
            DbRequest::GetTrackerById { tracker_id, reply } => {
                let _ = reply.send(handle_get_tracker_by_id(&conn, tracker_id));
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
    new_content: Option<&str>,
) -> Result<DiffProposal, DbError> {
    let now = now_millis();
    let status = "pending";
    conn.execute(
        "INSERT INTO diff_proposals (session_id, file_path, base_hash, patch, status, created_at, resolved_at, new_content) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
        params![session_id, file_path, base_hash, patch, status, now, new_content],
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
        new_content: new_content.map(|s| s.to_string()),
    })
}

fn handle_get_diff_proposal(conn: &Connection, id: i64) -> Result<DiffProposal, DbError> {
    let row = conn
        .query_row(
            "SELECT id, session_id, file_path, base_hash, patch, status, created_at, resolved_at, new_content \
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
                    new_content: row.get(8)?,
                })
            },
        )
        .optional()?;
    row.ok_or_else(|| DbError::NotFound(format!("diff_proposal {} not found", id)))
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
            "SELECT id, session_id, file_path, base_hash, patch, status, created_at, resolved_at, new_content \
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
                    new_content: row.get(8)?,
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
            "SELECT id, session_id, file_path, base_hash, patch, status, created_at, resolved_at, new_content \
             FROM diff_proposals \
             WHERE status = 'pending' AND session_id = ?1 \
             ORDER BY created_at ASC"
        }
        None => {
            "SELECT id, session_id, file_path, base_hash, patch, status, created_at, resolved_at, new_content \
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
            new_content: row.get(8)?,
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

// ---- M4 tracker handlers ----

fn handle_upsert_tracker(
    conn: &Connection,
    workspace_id: i64,
    name: &str,
    yaml_source: &str,
    schema_json: &str,
) -> Result<Tracker, DbError> {
    // INSERT ... ON CONFLICT(workspace_id, name) updates the yaml_source +
    // schema_json in place. enabled stays at its prior value to preserve
    // a user's disable toggle across reloads.
    conn.execute(
        "INSERT INTO trackers (workspace_id, name, yaml_source, schema_json, enabled) \
         VALUES (?1, ?2, ?3, ?4, 1) \
         ON CONFLICT(workspace_id, name) DO UPDATE SET \
             yaml_source = excluded.yaml_source, \
             schema_json = excluded.schema_json",
        params![workspace_id, name, yaml_source, schema_json],
    )?;
    let row = conn
        .query_row(
            "SELECT id, workspace_id, name, yaml_source, schema_json, enabled \
             FROM trackers WHERE workspace_id = ?1 AND name = ?2",
            params![workspace_id, name],
            |row| {
                let enabled_int: i64 = row.get(5)?;
                Ok(Tracker {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    name: row.get(2)?,
                    yaml_source: row.get(3)?,
                    schema_json: row.get(4)?,
                    enabled: enabled_int != 0,
                })
            },
        )
        .optional()?;
    row.ok_or_else(|| {
        DbError::NotFound(format!(
            "tracker (workspace_id={}, name={}) upsert disappeared",
            workspace_id, name
        ))
    })
}

fn handle_list_trackers(conn: &Connection, workspace_id: i64) -> Result<Vec<Tracker>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, workspace_id, name, yaml_source, schema_json, enabled \
         FROM trackers WHERE workspace_id = ?1 ORDER BY name ASC",
    )?;
    let rows = stmt
        .query_map(params![workspace_id], |row| {
            let enabled_int: i64 = row.get(5)?;
            Ok(Tracker {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                yaml_source: row.get(3)?,
                schema_json: row.get(4)?,
                enabled: enabled_int != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn handle_get_tracker_by_name(
    conn: &Connection,
    workspace_id: i64,
    name: &str,
) -> Result<Option<Tracker>, DbError> {
    let row = conn
        .query_row(
            "SELECT id, workspace_id, name, yaml_source, schema_json, enabled \
             FROM trackers WHERE workspace_id = ?1 AND name = ?2",
            params![workspace_id, name],
            |row| {
                let enabled_int: i64 = row.get(5)?;
                Ok(Tracker {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    name: row.get(2)?,
                    yaml_source: row.get(3)?,
                    schema_json: row.get(4)?,
                    enabled: enabled_int != 0,
                })
            },
        )
        .optional()?;
    Ok(row)
}

fn handle_create_tracker_item(
    conn: &Connection,
    tracker_id: i64,
    public_id: &str,
    status: Option<&str>,
    fields_json: &str,
) -> Result<TrackerItem, DbError> {
    let now = now_millis();
    conn.execute(
        "INSERT INTO tracker_items (tracker_id, public_id, status, fields_json, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![tracker_id, public_id, status, fields_json, now],
    )?;
    let id = conn.last_insert_rowid();
    Ok(TrackerItem {
        id,
        tracker_id,
        public_id: public_id.to_string(),
        status: status.map(|s| s.to_string()),
        fields_json: fields_json.to_string(),
        created_at: now,
        updated_at: now,
    })
}

fn handle_update_tracker_item(
    conn: &Connection,
    id: i64,
    status: Option<&str>,
    fields_json: &str,
) -> Result<TrackerItem, DbError> {
    let now = now_millis();
    let updated = conn.execute(
        "UPDATE tracker_items SET status = ?1, fields_json = ?2, updated_at = ?3 \
         WHERE id = ?4",
        params![status, fields_json, now, id],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound(format!(
            "tracker_item id {} not found",
            id
        )));
    }
    let row = conn
        .query_row(
            "SELECT id, tracker_id, public_id, status, fields_json, created_at, updated_at \
             FROM tracker_items WHERE id = ?1",
            params![id],
            |row| {
                Ok(TrackerItem {
                    id: row.get(0)?,
                    tracker_id: row.get(1)?,
                    public_id: row.get(2)?,
                    status: row.get(3)?,
                    fields_json: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()?;
    row.ok_or_else(|| DbError::NotFound(format!("tracker_item {} disappeared", id)))
}

fn handle_list_tracker_items(
    conn: &Connection,
    tracker_id: i64,
    status_filter: Option<&str>,
) -> Result<Vec<TrackerItem>, DbError> {
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(TrackerItem {
            id: row.get(0)?,
            tracker_id: row.get(1)?,
            public_id: row.get(2)?,
            status: row.get(3)?,
            fields_json: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    };
    let rows: Vec<TrackerItem> = match status_filter {
        Some(filter) => {
            let mut stmt = conn.prepare(
                "SELECT id, tracker_id, public_id, status, fields_json, created_at, updated_at \
                 FROM tracker_items \
                 WHERE tracker_id = ?1 AND status = ?2 \
                 ORDER BY created_at ASC",
            )?;
            let iter = stmt.query_map(params![tracker_id, filter], map_row)?;
            let out: Result<Vec<_>, _> = iter.collect();
            out?
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, tracker_id, public_id, status, fields_json, created_at, updated_at \
                 FROM tracker_items \
                 WHERE tracker_id = ?1 \
                 ORDER BY created_at ASC",
            )?;
            let iter = stmt.query_map(params![tracker_id], map_row)?;
            let out: Result<Vec<_>, _> = iter.collect();
            out?
        }
    };
    Ok(rows)
}

fn handle_list_tracker_item_ids(
    conn: &Connection,
    tracker_id: i64,
) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT public_id FROM tracker_items WHERE tracker_id = ?1",
    )?;
    let rows = stmt
        .query_map(params![tracker_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn handle_get_tracker_by_id(
    conn: &Connection,
    tracker_id: i64,
) -> Result<Option<Tracker>, DbError> {
    let row = conn
        .query_row(
            "SELECT id, workspace_id, name, yaml_source, schema_json, enabled \
             FROM trackers WHERE id = ?1",
            params![tracker_id],
            |row| {
                let enabled_int: i64 = row.get(5)?;
                Ok(Tracker {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    name: row.get(2)?,
                    yaml_source: row.get(3)?,
                    schema_json: row.get(4)?,
                    enabled: enabled_int != 0,
                })
            },
        )
        .optional()?;
    Ok(row)
}
