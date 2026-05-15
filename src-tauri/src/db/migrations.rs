// Migration runner. v1 ships exactly one migration: the initial schema.
//
// Design:
// - schema_version table tracks the highest applied migration version.
// - On open we check whether schema_version exists. If not, we apply v1
//   (the embedded schema.sql) inside a single transaction and record the
//   version.
// - For future versions, append branches to the match arm below and bump
//   LATEST_VERSION. Migrations strictly higher than the current version
//   are applied in order.
// - Foreign keys + WAL + synchronous=NORMAL are set on the connection
//   before migrations run. Pragmas are connection-scoped so the actor
//   re-applies them on its own connection too.

use rusqlite::Connection;

use super::error::DbError;

const SCHEMA_V1: &str = include_str!("schema.sql");
const SCHEMA_V2: &str = include_str!("schema_v2.sql");
const LATEST_VERSION: u32 = 2;

pub fn apply_connection_pragmas(conn: &Connection) -> Result<(), DbError> {
    // foreign_keys is per-connection in SQLite; the actor must re-apply.
    conn.pragma_update(None, "foreign_keys", "ON")?;
    // WAL is persisted on the database file, but setting it here is harmless.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(())
}

fn current_version(conn: &Connection) -> Result<u32, DbError> {
    // sqlite_master is always present; check whether schema_version exists.
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |row| row.get(0),
    )?;
    if exists == 0 {
        return Ok(0);
    }
    let version: u32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;
    Ok(version)
}

pub fn run_migrations(conn: &mut Connection) -> Result<u32, DbError> {
    apply_connection_pragmas(conn)?;

    let from = current_version(conn)?;
    if from >= LATEST_VERSION {
        return Ok(from);
    }

    for version in (from + 1)..=LATEST_VERSION {
        let tx = conn.transaction()?;
        match version {
            1 => {
                tx.execute_batch(SCHEMA_V1)?;
                tx.execute_batch(
                    "CREATE TABLE schema_version (\n\
                         version    INTEGER PRIMARY KEY,\n\
                         applied_at INTEGER NOT NULL\n\
                     );",
                )?;
                let now = now_millis();
                tx.execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                    rusqlite::params![1_i64, now],
                )?;
            }
            2 => {
                // Adds new_content column to diff_proposals (M5). Idempotent
                // by virtue of LATEST_VERSION gating; ALTER TABLE ADD COLUMN
                // itself is not idempotent in SQLite, but the runner only
                // invokes this branch when current_version < 2.
                tx.execute_batch(SCHEMA_V2)?;
                let now = now_millis();
                tx.execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                    rusqlite::params![2_i64, now],
                )?;
            }
            other => {
                return Err(DbError::Invalid(format!(
                    "no migration registered for version {}",
                    other
                )));
            }
        }
        tx.commit()?;
    }

    Ok(LATEST_VERSION)
}

fn now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
