-- Teraxlyst v1 initial schema. Source: planning/ARCHITECTURE.md section 2.
-- Applied as a single transaction by migrations::run_migrations on first open.

-- workspaces map to a directory on disk + git remote
CREATE TABLE workspaces (
    id         INTEGER PRIMARY KEY,
    path       TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- AI sessions are agent runs; one workspace -> many sessions
CREATE TABLE sessions (
    id            INTEGER PRIMARY KEY,
    workspace_id  INTEGER NOT NULL REFERENCES workspaces(id),
    parent_id     INTEGER REFERENCES sessions(id),
    provider      TEXT NOT NULL,
    title         TEXT,
    status        TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

-- Append-only event log per session; raw payloads kept verbatim
CREATE TABLE session_events (
    id         INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    seq        INTEGER NOT NULL,
    kind       TEXT NOT NULL,
    payload    TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (session_id, seq)
);

-- Trackers are YAML-defined entity types (Plans, Decisions, Bugs, Tasks, ...)
CREATE TABLE trackers (
    id            INTEGER PRIMARY KEY,
    workspace_id  INTEGER NOT NULL REFERENCES workspaces(id),
    name          TEXT NOT NULL,
    yaml_source   TEXT NOT NULL,
    schema_json   TEXT NOT NULL,
    enabled       INTEGER NOT NULL DEFAULT 1,
    UNIQUE (workspace_id, name)
);

CREATE TABLE tracker_items (
    id          INTEGER PRIMARY KEY,
    tracker_id  INTEGER NOT NULL REFERENCES trackers(id),
    public_id   TEXT NOT NULL,
    status      TEXT,
    fields_json TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    UNIQUE (tracker_id, public_id)
);

-- Diff proposals from agents, awaiting approval
CREATE TABLE diff_proposals (
    id          INTEGER PRIMARY KEY,
    session_id  INTEGER NOT NULL REFERENCES sessions(id),
    file_path   TEXT NOT NULL,
    base_hash   TEXT,
    patch       TEXT NOT NULL,
    status      TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    resolved_at INTEGER
);

CREATE INDEX idx_session_events_session_seq ON session_events(session_id, seq);
CREATE INDEX idx_tracker_items_tracker_status ON tracker_items(tracker_id, status);
CREATE INDEX idx_diff_proposals_session_status ON diff_proposals(session_id, status);
