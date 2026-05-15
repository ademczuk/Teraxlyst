// Tracker module unit + integration tests.
//
// Coverage per M4 spec:
// - parse_valid_bugs_yaml: end-to-end YAML round-trip on the canonical
//   Bugs schema from ARCHITECTURE.md §7.
// - reject_invalid_role_duplication: two title roles -> schema error.
// - next_public_id_sequential: prefix-padded counter from existing ids.
// - validate_required_fields: missing required field -> validation error.
//
// Plus a couple of supporting tests:
// - parse_rejects_unknown_field_type
// - validate_select_options_enforced
// - db_round_trip_upsert_and_list (uses spawn_in_memory)
// - create_item_assigns_sequential_id

use super::ids::next_public_id;
use super::loader::parse_yaml;
use super::schema::{FieldType, IdFormat};
use super::validate::validate_item_fields;

const BUGS_YAML: &str = r#"
name: Bugs
id_format:
  type: sequential
  prefix: BUG
  pad: 3
fields:
  - name: title
    type: string
    required: true
    roles: [title]
  - name: severity
    type: select
    options: [low, medium, high, critical]
  - name: status
    type: select
    roles: [workflowStatus]
    options: [open, in_progress, fixed, wontfix]
  - name: reporter
    type: user
    roles: [reporter]
  - name: created
    type: datetime
    auto: created_at
"#;

#[test]
fn parse_valid_bugs_yaml() {
    let def = parse_yaml(BUGS_YAML).expect("Bugs YAML must parse");
    assert_eq!(def.name, "Bugs");
    match def.id_format {
        IdFormat::Sequential { ref prefix, pad } => {
            assert_eq!(prefix, "BUG");
            assert_eq!(pad, 3);
        }
        _ => panic!("expected sequential id_format"),
    }
    assert_eq!(def.fields.len(), 5);
    let title = def.field_with_role("title").expect("title role present");
    assert_eq!(title.name, "title");
    assert!(matches!(title.field_type, FieldType::String));
    let status = def
        .field_with_role("workflowStatus")
        .expect("workflowStatus role present");
    assert_eq!(status.name, "status");
}

#[test]
fn reject_invalid_role_duplication() {
    let yaml = r#"
name: Tasks
id_format:
  type: sequential
  prefix: TASK
  pad: 3
fields:
  - { name: title, type: string, roles: [title] }
  - { name: alt_title, type: string, roles: [title] }
"#;
    let err = parse_yaml(yaml).expect_err("duplicate title role must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("title") && msg.contains("multiple"),
        "expected role-duplication error, got: {}",
        msg
    );
}

#[test]
fn next_public_id_sequential() {
    let fmt = IdFormat::Sequential {
        prefix: "BUG".into(),
        pad: 3,
    };
    let next = next_public_id(&fmt, &[]).unwrap();
    assert_eq!(next, "BUG-001");

    let next = next_public_id(
        &fmt,
        &["BUG-001".into(), "BUG-002".into()],
    )
    .unwrap();
    assert_eq!(next, "BUG-003");

    // Gaps preserved: deletes don't reset the counter.
    let next = next_public_id(
        &fmt,
        &["BUG-001".into(), "BUG-005".into()],
    )
    .unwrap();
    assert_eq!(next, "BUG-006");

    // Unrelated ids ignored.
    let next = next_public_id(
        &fmt,
        &["TASK-010".into(), "BUG-007".into()],
    )
    .unwrap();
    assert_eq!(next, "BUG-008");
}

#[test]
fn validate_required_fields() {
    let def = parse_yaml(BUGS_YAML).unwrap();
    let missing = serde_json::json!({
        "severity": "high",
        "status": "open",
    });
    let err = validate_item_fields(&def, &missing).expect_err("title is required");
    assert!(err.to_string().contains("title"));

    let ok = serde_json::json!({
        "title": "CI broken",
        "severity": "high",
        "status": "open",
    });
    let normalized = validate_item_fields(&def, &ok).expect("valid item");
    assert_eq!(normalized.get("title").unwrap(), "CI broken");
    assert_eq!(normalized.get("status").unwrap(), "open");
}

#[test]
fn validate_select_options_enforced() {
    let def = parse_yaml(BUGS_YAML).unwrap();
    let bad = serde_json::json!({
        "title": "X",
        "severity": "extreme",
        "status": "open",
    });
    let err = validate_item_fields(&def, &bad).expect_err("invalid select option");
    let msg = err.to_string();
    assert!(msg.contains("severity") || msg.contains("extreme"));
}

#[test]
fn parse_rejects_select_without_options() {
    let yaml = r#"
name: Tasks
id_format: { type: ulid }
fields:
  - { name: title, type: string, roles: [title] }
  - { name: priority, type: select }
"#;
    let err = parse_yaml(yaml).expect_err("select without options must fail");
    let msg = err.to_string();
    assert!(msg.contains("select") || msg.contains("options"));
}

#[test]
fn parse_rejects_zero_pad_sequential() {
    let yaml = r#"
name: T
id_format:
  type: sequential
  prefix: T
  pad: 0
fields:
  - { name: title, type: string, roles: [title] }
"#;
    let err = parse_yaml(yaml).expect_err("pad=0 must fail");
    assert!(err.to_string().contains("pad"));
}

#[tokio::test]
async fn db_round_trip_upsert_and_list() {
    use crate::db::actor::spawn_in_memory;

    let db = spawn_in_memory().expect("actor");
    let ws = db
        .create_workspace("/w".into(), "ws".into())
        .await
        .unwrap();

    let def = parse_yaml(BUGS_YAML).unwrap();
    let yaml_text = serde_yaml_ng::to_string(&def).unwrap();
    let schema_json = serde_json::to_string(&def).unwrap();
    let t = db
        .upsert_tracker(ws.id, def.name.clone(), yaml_text, schema_json)
        .await
        .unwrap();
    assert_eq!(t.name, "Bugs");
    assert!(t.enabled);

    let list = db.list_trackers(ws.id).await.unwrap();
    assert_eq!(list.len(), 1);

    // Upsert again with edited yaml; same row id, updated schema_json.
    let updated_json = serde_json::to_string(&def).unwrap();
    let t2 = db
        .upsert_tracker(ws.id, def.name.clone(), "# edited".into(), updated_json)
        .await
        .unwrap();
    assert_eq!(t2.id, t.id);

    let by_name = db
        .get_tracker_by_name(ws.id, "Bugs".into())
        .await
        .unwrap();
    assert!(by_name.is_some());
    let by_id = db.get_tracker_by_id(t.id).await.unwrap();
    assert!(by_id.is_some());
}

#[tokio::test]
async fn create_and_list_tracker_items() {
    use crate::db::actor::spawn_in_memory;

    let db = spawn_in_memory().expect("actor");
    let ws = db
        .create_workspace("/w2".into(), "ws2".into())
        .await
        .unwrap();
    let def = parse_yaml(BUGS_YAML).unwrap();
    let schema_json = serde_json::to_string(&def).unwrap();
    let t = db
        .upsert_tracker(ws.id, def.name.clone(), BUGS_YAML.into(), schema_json)
        .await
        .unwrap();

    // First create item: should land on BUG-001.
    let existing = db.list_tracker_item_ids(t.id).await.unwrap();
    let id1 = next_public_id(&def.id_format, &existing).unwrap();
    assert_eq!(id1, "BUG-001");
    let fields = serde_json::json!({"title": "first"}).to_string();
    let _it = db
        .create_tracker_item(t.id, id1.clone(), Some("open".into()), fields)
        .await
        .unwrap();

    // Second create item: BUG-002.
    let existing = db.list_tracker_item_ids(t.id).await.unwrap();
    let id2 = next_public_id(&def.id_format, &existing).unwrap();
    assert_eq!(id2, "BUG-002");
    let fields = serde_json::json!({"title": "second"}).to_string();
    db.create_tracker_item(t.id, id2, Some("open".into()), fields)
        .await
        .unwrap();

    let items = db.list_tracker_items(t.id, None).await.unwrap();
    assert_eq!(items.len(), 2);
    let open = db
        .list_tracker_items(t.id, Some("open".into()))
        .await
        .unwrap();
    assert_eq!(open.len(), 2);
    let closed = db
        .list_tracker_items(t.id, Some("fixed".into()))
        .await
        .unwrap();
    assert!(closed.is_empty());
}
