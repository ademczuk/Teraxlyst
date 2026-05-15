// Item-fields validation against a loaded TrackerDef.
//
// The caller submits a JSON object of field -> value. We:
// - Reject unknown fields (typo guard).
// - Coerce known fields to their declared type and accumulate errors.
// - Enforce required + select-options constraints.
//
// Returns a normalized fields_json that callers can persist verbatim.
// Auto-populated fields (created_at, updated_at, reporter) are NOT
// inserted here - they live alongside the user-edited fields and are
// applied by the actor handlers.

use serde_json::{Map, Value};

use super::error::TrackerError;
use super::schema::{FieldType, TrackerDef};

pub fn validate_item_fields(
    schema: &TrackerDef,
    submitted: &Value,
) -> Result<Value, TrackerError> {
    let obj = submitted.as_object().ok_or_else(|| {
        TrackerError::Validation("item fields must be a JSON object".into())
    })?;

    // Reject any unknown field name to catch typos at the boundary.
    let known_names: Vec<&str> = schema.fields.iter().map(|f| f.name.as_str()).collect();
    for key in obj.keys() {
        if !known_names.contains(&key.as_str()) {
            return Err(TrackerError::Validation(format!(
                "unknown field '{}' (known: {:?})",
                key, known_names
            )));
        }
    }

    let mut normalized: Map<String, Value> = Map::new();
    for field in &schema.fields {
        let provided = obj.get(&field.name);
        // Auto fields are exempt from the "required" check - the actor
        // fills them later.
        let is_auto = field.auto.is_some();
        match provided {
            Some(v) if !v.is_null() => {
                let coerced = coerce(field.field_type, field.options.as_deref(), v, &field.name)?;
                normalized.insert(field.name.clone(), coerced);
            }
            _ => {
                if field.required && !is_auto {
                    return Err(TrackerError::Validation(format!(
                        "required field '{}' missing",
                        field.name
                    )));
                }
                // Optional field absent: skip. Renderer treats missing as null.
            }
        }
    }
    Ok(Value::Object(normalized))
}

// Per-type coercion. We accept slightly looser types than strict (numbers
// as numeric strings, booleans as 0/1) because the YAML / form layer
// sometimes can't disambiguate. Failures surface as Validation errors with
// the offending field name.
fn coerce(
    ty: FieldType,
    options: Option<&[String]>,
    value: &Value,
    field_name: &str,
) -> Result<Value, TrackerError> {
    match ty {
        FieldType::String | FieldType::Text => match value {
            Value::String(s) => Ok(Value::String(s.clone())),
            _ => Err(TrackerError::Validation(format!(
                "field '{}' expected string, got {}",
                field_name, value
            ))),
        },
        FieldType::Number => match value {
            Value::Number(_) => Ok(value.clone()),
            Value::String(s) => s
                .parse::<f64>()
                .map(|f| {
                    serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                })
                .map_err(|_| {
                    TrackerError::Validation(format!(
                        "field '{}' expected number, got string '{}'",
                        field_name, s
                    ))
                }),
            _ => Err(TrackerError::Validation(format!(
                "field '{}' expected number",
                field_name
            ))),
        },
        FieldType::Boolean => match value {
            Value::Bool(_) => Ok(value.clone()),
            Value::Number(n) => Ok(Value::Bool(n.as_i64().unwrap_or(0) != 0)),
            _ => Err(TrackerError::Validation(format!(
                "field '{}' expected boolean",
                field_name
            ))),
        },
        FieldType::Select => {
            let s = value.as_str().ok_or_else(|| {
                TrackerError::Validation(format!(
                    "field '{}' expected select string",
                    field_name
                ))
            })?;
            let opts = options.ok_or_else(|| {
                TrackerError::Schema(format!(
                    "field '{}' is select but has no options in schema",
                    field_name
                ))
            })?;
            if !opts.iter().any(|o| o == s) {
                return Err(TrackerError::Validation(format!(
                    "field '{}' value '{}' not in options {:?}",
                    field_name, s, opts
                )));
            }
            Ok(Value::String(s.to_string()))
        }
        FieldType::Multiselect => {
            let arr = value.as_array().ok_or_else(|| {
                TrackerError::Validation(format!(
                    "field '{}' expected multiselect array",
                    field_name
                ))
            })?;
            let opts = options.ok_or_else(|| {
                TrackerError::Schema(format!(
                    "field '{}' is multiselect but has no options",
                    field_name
                ))
            })?;
            for item in arr {
                let s = item.as_str().ok_or_else(|| {
                    TrackerError::Validation(format!(
                        "field '{}' multiselect entry not a string",
                        field_name
                    ))
                })?;
                if !opts.iter().any(|o| o == s) {
                    return Err(TrackerError::Validation(format!(
                        "field '{}' value '{}' not in options",
                        field_name, s
                    )));
                }
            }
            Ok(value.clone())
        }
        // Date / datetime are stored as strings; deeper format validation
        // is M4.1.
        FieldType::Date | FieldType::Datetime => match value {
            Value::String(_) => Ok(value.clone()),
            _ => Err(TrackerError::Validation(format!(
                "field '{}' expected date string",
                field_name
            ))),
        },
        FieldType::User | FieldType::Reference => match value {
            Value::String(_) => Ok(value.clone()),
            _ => Err(TrackerError::Validation(format!(
                "field '{}' expected reference string",
                field_name
            ))),
        },
        FieldType::Array => match value {
            Value::Array(_) => Ok(value.clone()),
            _ => Err(TrackerError::Validation(format!(
                "field '{}' expected array",
                field_name
            ))),
        },
        FieldType::Object => match value {
            Value::Object(_) => Ok(value.clone()),
            _ => Err(TrackerError::Validation(format!(
                "field '{}' expected object",
                field_name
            ))),
        },
    }
}
