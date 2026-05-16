// Tracker module: YAML-defined entity types with schema validation,
// DB-backed CRUD, and Tauri commands. See planning/ARCHITECTURE.md
// section 7 and planning/ROADMAP.md M4.
//
// M4 scope (per planning/M4_IMPLEMENTATION.md):
// - YAML parse + validate (schema.rs, loader.rs, validate.rs)
// - ID generation (ids.rs): sequential / ulid / uuid
// - DB sync (sync.rs): upsert TrackerDef to trackers table
// - 4 Tauri commands (commands.rs): load_workspace, create_item,
//   update_item, list_items, plus query_items
//
// Deferred to M4.1:
// - File watcher for YAML changes
// - Inline #tracker[ID] markdown ref parser
// - Kanban board view (table only in M4)
// - MCP host registration (M3 not yet merged)

pub mod commands;
pub mod error;
pub mod ids;
pub mod loader;
pub mod mcp_wrappers;
pub mod schema;
pub mod sync;
pub mod validate;

#[cfg(test)]
mod tests;

// Re-exports trimmed to satisfy clippy -D warnings; modules accessed via
// path (e.g. trackers::schema::TrackerDef) where needed.
