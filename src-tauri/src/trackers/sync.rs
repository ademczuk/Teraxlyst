// sync.rs: upsert TrackerDef instances into the `trackers` table.
//
// The yaml source is the user-facing truth; schema_json is the parsed
// representation that the renderer (and validate.rs) consume. We serialize
// the TrackerDef back to JSON before storing rather than re-parsing the
// YAML at read time, so the DB schema_json column is canonical.

use crate::db::actor::DbHandle;
use crate::db::types::Tracker;

use super::error::TrackerError;
use super::schema::TrackerDef;

// Persist a set of parsed tracker definitions for a workspace. Each
// (workspace_id, name) is upserted; the per-tracker yaml_source must be
// supplied alongside the parsed TrackerDef so the DB keeps the original
// text for round-trip viewing.
pub async fn sync_to_db(
    db: &DbHandle,
    workspace_id: i64,
    defs: &[(String, TrackerDef)],
) -> Result<Vec<Tracker>, TrackerError> {
    let mut out = Vec::with_capacity(defs.len());
    for (yaml_source, def) in defs {
        let schema_json = serde_json::to_string(def)?;
        let tracker = db
            .upsert_tracker(
                workspace_id,
                def.name.clone(),
                yaml_source.clone(),
                schema_json,
            )
            .await?;
        out.push(tracker);
    }
    Ok(out)
}
