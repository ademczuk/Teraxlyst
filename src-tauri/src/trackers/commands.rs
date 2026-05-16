// Tauri command surface for tracker CRUD. Mirrors the db::commands and
// sessions::commands shape: each command takes State<'_, DbHandle>, maps
// any TrackerError to its Display string at the IPC boundary.
//
// Five commands ship in M4:
// - tracker_load_workspace: parse YAML, sync to DB. Returns parsed
//   trackers + any per-file parse errors so the renderer can surface
//   problems without blocking other trackers.
// - tracker_create_item: validate fields against schema, generate
//   public_id, insert.
// - tracker_update_item: validate + update.
// - tracker_list_items: list, optionally filter by status.
// - tracker_query: simple JSON filter; v1 supports exact-match on
//   top-level fields only.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::db::actor::DbHandle;
use crate::db::types::TrackerItem;

use super::error::TrackerError;
use super::ids::next_public_id;
use super::loader::{load_trackers_from_dir, LoadErrorEntry};
use super::schema::TrackerDef;
use super::sync::sync_to_db;
use super::validate::validate_item_fields;

#[derive(Debug, Serialize)]
pub struct LoadWorkspaceResult {
    pub trackers: Vec<LoadedTrackerDto>,
    pub errors: Vec<LoadErrorDto>,
}

#[derive(Debug, Serialize)]
pub struct LoadedTrackerDto {
    pub id: i64,
    pub name: String,
    pub schema: TrackerDef,
}

#[derive(Debug, Serialize)]
pub struct LoadErrorDto {
    pub path: String,
    pub message: String,
}

impl From<LoadErrorEntry> for LoadErrorDto {
    fn from(e: LoadErrorEntry) -> Self {
        LoadErrorDto {
            path: e.path.display().to_string(),
            message: e.message,
        }
    }
}

#[tauri::command]
pub async fn tracker_load_workspace(
    db: State<'_, DbHandle>,
    workspace_id: i64,
    workspace_path: String,
) -> Result<LoadWorkspaceResult, String> {
    let path = PathBuf::from(workspace_path);
    let report = load_trackers_from_dir(&path).map_err(|e: TrackerError| e.to_string())?;
    // Pair each parsed def with its serialized YAML form for storage.
    let mut pairs: Vec<(String, TrackerDef)> = Vec::with_capacity(report.trackers.len());
    for def in report.trackers {
        let yaml_text = serde_yaml_ng::to_string(&def).map_err(|e| e.to_string())?;
        pairs.push((yaml_text, def));
    }
    let stored = sync_to_db(&db, workspace_id, &pairs)
        .await
        .map_err(|e| e.to_string())?;
    let trackers: Vec<LoadedTrackerDto> = stored
        .into_iter()
        .zip(pairs)
        .map(|(t, (_, def))| LoadedTrackerDto {
            id: t.id,
            name: t.name,
            schema: def,
        })
        .collect();
    let errors = report.errors.into_iter().map(LoadErrorDto::from).collect();
    Ok(LoadWorkspaceResult { trackers, errors })
}

#[derive(Debug, Deserialize)]
pub struct CreateItemArgs {
    pub tracker_id: i64,
    pub fields: Value,
}

#[tauri::command]
pub async fn tracker_create_item(
    db: State<'_, DbHandle>,
    args: CreateItemArgs,
) -> Result<TrackerItem, String> {
    create_item_impl(&db, args)
        .await
        .map_err(|e| e.to_string())
}

async fn create_item_impl(
    db: &DbHandle,
    args: CreateItemArgs,
) -> Result<TrackerItem, TrackerError> {
    let tracker = db
        .get_tracker_by_id(args.tracker_id)
        .await?
        .ok_or_else(|| TrackerError::NotFound(format!("tracker {}", args.tracker_id)))?;
    let schema: TrackerDef = serde_json::from_str(&tracker.schema_json)?;
    let normalized = validate_item_fields(&schema, &args.fields)?;

    // Derive status from the workflowStatus role if present.
    let status: Option<String> = schema
        .field_with_role("workflowStatus")
        .and_then(|f| normalized.get(&f.name))
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    let existing_ids = db.list_tracker_item_ids(args.tracker_id).await?;
    let public_id = next_public_id(&schema.id_format, &existing_ids)?;

    let fields_json = serde_json::to_string(&normalized)?;
    let item = db
        .create_tracker_item(args.tracker_id, public_id, status, fields_json)
        .await?;
    Ok(item)
}

#[derive(Debug, Deserialize)]
pub struct UpdateItemArgs {
    pub item_id: i64,
    pub tracker_id: i64,
    pub fields: Value,
}

#[tauri::command]
pub async fn tracker_update_item(
    db: State<'_, DbHandle>,
    args: UpdateItemArgs,
) -> Result<TrackerItem, String> {
    update_item_impl(&db, args)
        .await
        .map_err(|e| e.to_string())
}

async fn update_item_impl(
    db: &DbHandle,
    args: UpdateItemArgs,
) -> Result<TrackerItem, TrackerError> {
    let tracker = db
        .get_tracker_by_id(args.tracker_id)
        .await?
        .ok_or_else(|| TrackerError::NotFound(format!("tracker {}", args.tracker_id)))?;
    let schema: TrackerDef = serde_json::from_str(&tracker.schema_json)?;
    let normalized = validate_item_fields(&schema, &args.fields)?;
    let status: Option<String> = schema
        .field_with_role("workflowStatus")
        .and_then(|f| normalized.get(&f.name))
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let fields_json = serde_json::to_string(&normalized)?;
    let item = db
        .update_tracker_item(args.item_id, status, fields_json)
        .await?;
    Ok(item)
}

#[derive(Debug, Deserialize)]
pub struct ListItemsArgs {
    pub tracker_id: i64,
    #[serde(default)]
    pub status: Option<String>,
}

#[tauri::command]
pub async fn tracker_list_items(
    db: State<'_, DbHandle>,
    args: ListItemsArgs,
) -> Result<Vec<TrackerItem>, String> {
    db.list_tracker_items(args.tracker_id, args.status)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct QueryArgs {
    pub workspace_id: i64,
    pub tracker_name: String,
    // JSON object mapping field name -> expected value. v1 supports
    // exact-match equality only. Empty object returns all items.
    #[serde(default)]
    pub query: Value,
}

#[tauri::command]
pub async fn tracker_query(
    db: State<'_, DbHandle>,
    args: QueryArgs,
) -> Result<Vec<TrackerItem>, String> {
    query_impl(&db, args).await.map_err(|e| e.to_string())
}

async fn query_impl(db: &DbHandle, args: QueryArgs) -> Result<Vec<TrackerItem>, TrackerError> {
    let tracker = db
        .get_tracker_by_name(args.workspace_id, args.tracker_name.clone())
        .await?
        .ok_or_else(|| {
            TrackerError::NotFound(format!(
                "tracker '{}' in workspace {}",
                args.tracker_name, args.workspace_id
            ))
        })?;

    let q_obj = match &args.query {
        Value::Object(m) => m.clone(),
        Value::Null => serde_json::Map::new(),
        _ => {
            return Err(TrackerError::Validation(
                "query must be a JSON object".into(),
            ));
        }
    };

    let items = db.list_tracker_items(tracker.id, None).await?;
    if q_obj.is_empty() {
        return Ok(items);
    }
    let filtered: Vec<TrackerItem> = items
        .into_iter()
        .filter(|item| matches_query(item, &q_obj))
        .collect();
    Ok(filtered)
}

fn matches_query(item: &TrackerItem, q: &serde_json::Map<String, Value>) -> bool {
    let parsed: Value = match serde_json::from_str(&item.fields_json) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let obj = match parsed.as_object() {
        Some(o) => o,
        None => return false,
    };
    for (k, expected) in q {
        match obj.get(k) {
            Some(actual) if actual == expected => {}
            _ => return false,
        }
    }
    true
}
