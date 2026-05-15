# M1 Implementation: Native Persistence Layer

Date: 2026-05-15
Milestone: M1 (per ROADMAP.md)

## Files created

All under `src-tauri/src/db/`:

- `mod.rs` (13 LOC) - module entry, re-exports `DbHandle`, `DbError`, `spawn_at_path`. Gates `tests` on `#[cfg(test)]`.
- `schema.sql` (71 LOC) - the v1 schema, verbatim from ARCHITECTURE.md section 2: six tables (workspaces, sessions, session_events, trackers, tracker_items, diff_proposals) plus three indexes.
- `migrations.rs` (93 LOC) - migration runner. `run_migrations(&mut Connection) -> Result<u32, DbError>` returns the resulting schema version. Per-connection pragmas (foreign_keys, journal_mode=WAL, synchronous=NORMAL) applied via a helper.
- `types.rs` (67 LOC) - the six DTO structs. All `Debug + Clone + Serialize + Deserialize`. `i64` for IDs and millis timestamps; `Option<T>` for nullable columns; `serde_json::Value` for `SessionEvent.payload`.
- `error.rs` (29 LOC) - `DbError` via `thiserror`, with a custom `Serialize` impl that emits the Display string so Tauri commands can return it.
- `actor.rs` (537 LOC) - the `DbActor` task plus the cloneable `DbHandle`. Eight request variants covering every command in the milestone. `spawn_at_path` and `spawn_in_memory` constructors. Handler functions are private (one per request variant).
- `commands.rs` (103 LOC) - eight `#[tauri::command]` async wrappers, each grabbing `State<'_, DbHandle>` and mapping `DbError` to `String` at the IPC boundary.
- `tests.rs` (152 LOC) - four `#[tokio::test]` cases.

Also touched:

- `src-tauri/Cargo.toml` - added `rusqlite 0.31` (bundled + serde_json), `tokio 1` (rt-multi-thread, macros, sync, fs), `thiserror 1`, `ulid 1`, plus dev-deps `tempfile 3` and a tokio dev profile including `test-util`. `ulid` is wired for future session-ID work (not yet used in v1 since SQLite INTEGER PRIMARY KEY is sufficient for M1; kept in the dep list per the brief).
- `src-tauri/src/lib.rs` - added `mod db;`, a `.setup()` closure that resolves `<app_data_dir>/teraxlyst.sqlite`, creates the directory, calls `db::spawn_at_path`, and stores the `DbHandle` via `app.manage()`. Registered the eight new commands in `tauri::generate_handler!`.

## Migration design

- Single source of truth: `schema.sql` embedded via `include_str!`. No external migration tool.
- Versioning: a `schema_version (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL)` table tracks applied migrations. `current_version()` returns 0 when that table doesn't exist, else `COALESCE(MAX(version), 0)`.
- Ordering: a `for version in (from+1)..=LATEST_VERSION` loop applies each migration in order. v1 is the only registered branch.
- Transaction boundary: every migration runs inside its own `conn.transaction()` so a partial apply rolls back cleanly. The schema apply and the `schema_version` insert are in the same transaction; the DB can never be in a state where the tables exist but the version isn't recorded.
- Pragmas applied before migrations on the same connection. WAL is persisted on the DB file; `foreign_keys` is per-connection so the actor's connection (the same one) inherits it automatically since `spawn_at_path` opens, migrates, and hands off the same `Connection` to the actor.

## Concurrency model

One rusqlite `Connection` lives inside the actor task, which runs on a dedicated OS thread via `tokio::task::spawn_blocking`. All requests arrive on an `mpsc::Receiver<DbRequest>` and are consumed via `blocking_recv()`; each variant carries a `oneshot::Sender` for the reply. `DbHandle` is a cheaply cloneable wrapper around `mpsc::Sender<DbRequest>` held by every Tauri command handler. This serializes all writes through one connection, eliminating `SQLITE_BUSY` by construction at the cost of v1 having no parallel readers (a reader pool is explicitly scoped to v1.1).

## Integration test design

`tests::concurrent_event_writes_serialize_correctly` is the milestone exit-criterion check. It:

1. Spawns the actor against an in-memory DB.
2. Creates 1 workspace + 10 sessions.
3. Spawns 10 concurrent `tokio::spawn` writers, each appending 100 events to its assigned session.
4. After all join, lists events per session and asserts `len == 100`, seqs equal `1..=100` exactly, and seqs are unique (HashSet check).
5. Failure modes covered: a `SQLITE_BUSY` would surface as an `append_event` returning `Err`, which the test `expect`s away and would panic; a seq gap would fail the equality check; a duplicate seq would fail the HashSet check.

Three companion tests cover the other surface area:

- `workspace_list_round_trip` - basic create + list ordering.
- `diff_proposal_create_resolve` - create, list pending, resolve approve, verify status flip + resolved_at timestamp + that the same proposal can't be resolved twice (NotFound on second attempt).
- `migrations_idempotent_on_reopen` - opens the same on-disk SQLite file twice via `tempfile`, asserts the second `run_migrations` returns version 1 without re-applying. Uses `tempfile = "3"` from dev-deps.

## Known v1.1 limitations (deliberate cuts)

1. **Single connection, no reader pool.** Architecture doc calls for 4-8 read-only connections; v1 ships one. Trivially additive when needed (a `Vec<Connection>` + round-robin on the read paths). Deferred until profiling shows reads are blocking writes.
2. **No FTS5 / full-text search** on `session_events.payload`. Will be added when the transcript-search UI lands (post-M2).
3. **No batched event insert.** ROADMAP M1 risk callout flags write-amplification under 500 tok/s streams. The handler exists for one event at a time. v1.1 will add `AppendEventsBatch` accepting `Vec<(kind, payload)>` and a single transaction for the lot, debounced upstream at the SessionManager layer in M2.
4. **No tracker CRUD commands yet.** `Tracker` and `TrackerItem` types are in `types.rs` because the schema includes them, but the DbRequest variants for tracker CRUD are M4 scope. A `#[allow(dead_code)] fn _ensure_unused_types_compile` keeps them compiling without warnings.
5. **No session pause/resume/kill DB hooks.** Status is settable only at create time (defaults to `running`). M2 will add an `UpdateSessionStatus` request when the SessionManager lands.
6. **ULID dep is unused in v1 code.** Kept per the brief because sessions will need a stable external identifier once exposed over MCP or via deep links; bringing it in now avoids a churn-y dep bump later.

## Lines of code

| File | LOC |
|------|-----|
| actor.rs | 537 |
| tests.rs | 152 |
| commands.rs | 103 |
| migrations.rs | 93 |
| schema.sql | 71 |
| types.rs | 67 |
| error.rs | 29 |
| mod.rs | 13 |
| Total | 1065 |

Plus ~25 lines of Cargo.toml + lib.rs wiring changes. Within budget for a 2-week milestone.
