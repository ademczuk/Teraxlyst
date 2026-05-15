// Tracker schema types: the in-memory representation of a parsed YAML
// tracker definition. Mirrors nimbalyst's tracker schema with the role
// markers documented in planning/ARCHITECTURE.md section 7.
//
// Field types are intentionally a closed enum: extending requires a
// migration on stored items so legacy fields can be coerced. The
// architecture doc calls out the v1 set; this module enforces it.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::error::TrackerError;

// Top-level YAML schema. Field order matches what users see in the
// generated UI form (preserved by serde_yaml_ng for sequence types).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerDef {
    pub name: String,
    pub id_format: IdFormat,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IdFormat {
    // Sequential prefix-padded ids: prefix="BUG", pad=3 -> "BUG-001".
    Sequential { prefix: String, pad: u32 },
    Ulid,
    Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub auto: Option<AutoField>,
}

// Field types per ARCHITECTURE §7. Closed set - extending requires a
// schema_version bump on stored items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    String,
    Text,
    Number,
    Select,
    Multiselect,
    Date,
    Datetime,
    Boolean,
    User,
    Reference,
    Array,
    Object,
}

// Auto-populated field hints. v1 supports created_at, updated_at, reporter.
// Renderer keeps these read-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoField {
    CreatedAt,
    UpdatedAt,
    Reporter,
}

// Semantic role markers. Closed list - the renderer keys off these.
// Unique per tracker except for `tags` which is allowed multi-field.
pub const UNIQUE_ROLES: &[&str] = &[
    "title",
    "workflowStatus",
    "priority",
    "assignee",
    "reporter",
    "startDate",
    "dueDate",
    "progress",
];

pub const KNOWN_ROLES: &[&str] = &[
    "title",
    "workflowStatus",
    "priority",
    "assignee",
    "reporter",
    "tags",
    "startDate",
    "dueDate",
    "progress",
];

impl TrackerDef {
    // Validate the full schema. Errors return on the first violation.
    // Checks performed:
    // - name is non-empty.
    // - At least one field.
    // - Each role is in KNOWN_ROLES.
    // - Roles listed in UNIQUE_ROLES appear at most once across fields.
    // - select / multiselect must have non-empty options.
    // - reference field must have options (target tracker names).
    // - id_format.Sequential prefix is non-empty + pad in 1..=10.
    pub fn validate(&self) -> Result<(), TrackerError> {
        if self.name.trim().is_empty() {
            return Err(TrackerError::Schema(
                "tracker name must be non-empty".into(),
            ));
        }
        if self.fields.is_empty() {
            return Err(TrackerError::Schema(format!(
                "tracker {} has no fields",
                self.name
            )));
        }
        match &self.id_format {
            IdFormat::Sequential { prefix, pad } => {
                if prefix.trim().is_empty() {
                    return Err(TrackerError::InvalidIdFormat(
                        "sequential prefix must be non-empty".into(),
                    ));
                }
                if *pad == 0 || *pad > 10 {
                    return Err(TrackerError::InvalidIdFormat(format!(
                        "sequential pad must be in 1..=10, got {}",
                        pad
                    )));
                }
            }
            IdFormat::Ulid | IdFormat::Uuid => {}
        }

        // Field-level checks + role uniqueness tracking.
        let mut seen_unique: HashSet<&str> = HashSet::new();
        let mut field_names: HashSet<&str> = HashSet::new();
        for field in &self.fields {
            if field.name.trim().is_empty() {
                return Err(TrackerError::Schema(format!(
                    "field in tracker {} has empty name",
                    self.name
                )));
            }
            if !field_names.insert(field.name.as_str()) {
                return Err(TrackerError::Schema(format!(
                    "duplicate field name '{}' in tracker {}",
                    field.name, self.name
                )));
            }
            for role in &field.roles {
                let r = role.as_str();
                if !KNOWN_ROLES.contains(&r) {
                    return Err(TrackerError::Schema(format!(
                        "unknown role '{}' on field '{}' in tracker {}",
                        r, field.name, self.name
                    )));
                }
                if UNIQUE_ROLES.contains(&r) && !seen_unique.insert(r) {
                    return Err(TrackerError::Schema(format!(
                        "role '{}' assigned to multiple fields in tracker {}",
                        r, self.name
                    )));
                }
            }
            match field.field_type {
                FieldType::Select | FieldType::Multiselect => {
                    let opts = field.options.as_ref();
                    if opts.map(|o| o.is_empty()).unwrap_or(true) {
                        return Err(TrackerError::Schema(format!(
                            "field '{}' is select/multiselect but has no options",
                            field.name
                        )));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    // Return the field marked with the given role, if any. Used by the
    // renderer and the create-item path to extract the workflowStatus value.
    pub fn field_with_role(&self, role: &str) -> Option<&FieldDef> {
        self.fields
            .iter()
            .find(|f| f.roles.iter().any(|r| r == role))
    }
}
