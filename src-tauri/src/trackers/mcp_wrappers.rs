// MCP tool wrappers for trackers.
//
// Status: scaffold. These functions are the business-logic core of the
// four MCP tools (tracker_create_item, tracker_update_item,
// tracker_list_items, tracker_query) ready to be wired into rmcp's
// `#[tool_router]` once the MCP toolset gets DbHandle access. Kept
// alive deliberately; M3.2 will register them via TeraxlystToolset.
#![allow(dead_code)]
//
// Registration TODO: extend mcp::tools::TeraxlystToolset with a DbHandle
// field (alongside prompts + diffs), then add `#[tool]` methods that
// delegate here. Tracker tools share the same DbHandle the rest of the
// app uses; no separate connection needed.
//
// Keeping wrappers separate from the Tauri commands lets the two
// surfaces share identical logic without one depending on the other.
// The Tauri commands take `State<DbHandle>` and serialize TrackerError to
// String; the MCP wrappers take `&DbHandle` and return TrackerError
// directly for rmcp's error mapping layer.

use serde_json::Value;

use crate::db::actor::DbHandle;
use crate::db::types::TrackerItem;

use super::error::TrackerError;
use super::ids::next_public_id;
use super::schema::TrackerDef;
use super::validate::validate_item_fields;

pub async fn mcp_tracker_create_item(
    db: &DbHandle,
    tracker_id: i64,
    fields: Value,
) -> Result<TrackerItem, TrackerError> {
    let tracker = db
        .get_tracker_by_id(tracker_id)
        .await?
        .ok_or_else(|| TrackerError::NotFound(format!("tracker {}", tracker_id)))?;
    let schema: TrackerDef = serde_json::from_str(&tracker.schema_json)?;
    let normalized = validate_item_fields(&schema, &fields)?;
    let status: Option<String> = schema
        .field_with_role("workflowStatus")
        .and_then(|f| normalized.get(&f.name))
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let existing_ids = db.list_tracker_item_ids(tracker_id).await?;
    let public_id = next_public_id(&schema.id_format, &existing_ids)?;
    let fields_json = serde_json::to_string(&normalized)?;
    Ok(db
        .create_tracker_item(tracker_id, public_id, status, fields_json)
        .await?)
}

pub async fn mcp_tracker_update_item(
    db: &DbHandle,
    item_id: i64,
    tracker_id: i64,
    fields: Value,
) -> Result<TrackerItem, TrackerError> {
    let tracker = db
        .get_tracker_by_id(tracker_id)
        .await?
        .ok_or_else(|| TrackerError::NotFound(format!("tracker {}", tracker_id)))?;
    let schema: TrackerDef = serde_json::from_str(&tracker.schema_json)?;
    let normalized = validate_item_fields(&schema, &fields)?;
    let status: Option<String> = schema
        .field_with_role("workflowStatus")
        .and_then(|f| normalized.get(&f.name))
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let fields_json = serde_json::to_string(&normalized)?;
    Ok(db
        .update_tracker_item(item_id, status, fields_json)
        .await?)
}

pub async fn mcp_tracker_list_items(
    db: &DbHandle,
    tracker_id: i64,
    status_filter: Option<String>,
) -> Result<Vec<TrackerItem>, TrackerError> {
    Ok(db.list_tracker_items(tracker_id, status_filter).await?)
}

pub async fn mcp_tracker_query(
    db: &DbHandle,
    workspace_id: i64,
    tracker_name: String,
    query: Value,
) -> Result<Vec<TrackerItem>, TrackerError> {
    let tracker = db
        .get_tracker_by_name(workspace_id, tracker_name.clone())
        .await?
        .ok_or_else(|| {
            TrackerError::NotFound(format!(
                "tracker '{}' in workspace {}",
                tracker_name, workspace_id
            ))
        })?;
    let items = db.list_tracker_items(tracker.id, None).await?;
    let q_obj = match query {
        Value::Object(m) => m,
        Value::Null => serde_json::Map::new(),
        _ => {
            return Err(TrackerError::Validation(
                "query must be a JSON object".into(),
            ));
        }
    };
    if q_obj.is_empty() {
        return Ok(items);
    }
    let filtered = items
        .into_iter()
        .filter(|item| {
            let parsed: Value = match serde_json::from_str(&item.fields_json) {
                Ok(v) => v,
                Err(_) => return false,
            };
            let obj = match parsed.as_object() {
                Some(o) => o,
                None => return false,
            };
            q_obj.iter().all(|(k, expected)| {
                obj.get(k).map(|actual| actual == expected).unwrap_or(false)
            })
        })
        .collect();
    Ok(filtered)
}
