use serde_json::Value;

/// Validates the JSON-Schema subset used by built-in tools.
///
/// Supported keywords: `type`, `enum`, `required`, `properties`,
/// `additionalProperties`, `items`, `minimum`, `maximum`, `minLength`,
/// `maxLength`. This is intentionally small: tool schemas are authored in this
/// repository and reviewed together with their validators.
pub fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    validate_at(schema, value, "$")
}

fn validate_at(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    if let Some(expected) = schema.get("type") {
        check_type(expected, value, path)?;
    }

    if let Some(values) = schema.get("enum").and_then(Value::as_array)
        && !values.iter().any(|candidate| candidate == value)
    {
        return Err(format!("{path}: value is not one of the allowed values"));
    }

    match value {
        Value::Object(map) => {
            if let Some(required) = schema.get("required").and_then(Value::as_array) {
                for key in required {
                    let Some(key) = key.as_str() else { continue };
                    if !map.contains_key(key) {
                        return Err(format!("{path}: missing required field '{key}'"));
                    }
                }
            }
            let properties = schema.get("properties").and_then(Value::as_object);
            let deny_extra = schema
                .get("additionalProperties")
                .and_then(Value::as_bool)
                .is_some_and(|allowed| !allowed);
            if let Some(properties) = properties {
                for (key, item) in map {
                    match properties.get(key) {
                        Some(property_schema) => {
                            validate_at(property_schema, item, &format!("{path}.{key}"))?;
                        }
                        None if deny_extra => {
                            return Err(format!("{path}: unknown field '{key}'"));
                        }
                        None => {}
                    }
                }
            }
        }
        Value::Array(items) => {
            if let Some(items_schema) = schema.get("items") {
                for (index, item) in items.iter().enumerate() {
                    validate_at(items_schema, item, &format!("{path}[{index}]"))?;
                }
            }
            if let Some(min) = schema.get("minItems").and_then(Value::as_u64)
                && (items.len() as u64) < min
            {
                return Err(format!("{path}: expected at least {min} items"));
            }
            if let Some(max) = schema.get("maxItems").and_then(Value::as_u64)
                && (items.len() as u64) > max
            {
                return Err(format!("{path}: expected at most {max} items"));
            }
        }
        Value::String(text) => {
            if let Some(min) = schema.get("minLength").and_then(Value::as_u64)
                && (text.chars().count() as u64) < min
            {
                return Err(format!("{path}: string is too short"));
            }
            if let Some(max) = schema.get("maxLength").and_then(Value::as_u64)
                && (text.chars().count() as u64) > max
            {
                return Err(format!("{path}: string is too long"));
            }
        }
        Value::Number(number) => {
            if let Some(min) = schema.get("minimum").and_then(Value::as_f64)
                && number.as_f64().is_some_and(|value| value < min)
            {
                return Err(format!("{path}: value must be >= {min}"));
            }
            if let Some(max) = schema.get("maximum").and_then(Value::as_f64)
                && number.as_f64().is_some_and(|value| value > max)
            {
                return Err(format!("{path}: value must be <= {max}"));
            }
        }
        _ => {}
    }

    Ok(())
}

fn check_type(expected: &Value, value: &Value, path: &str) -> Result<(), String> {
    let matches = match expected {
        Value::String(kind) => matches_type(kind, value),
        Value::Array(kinds) => kinds
            .iter()
            .filter_map(Value::as_str)
            .any(|kind| matches_type(kind, value)),
        _ => true,
    };
    if matches {
        Ok(())
    } else {
        Err(format!("{path}: unexpected type"))
    }
}

fn matches_type(kind: &str, value: &Value) -> bool {
    match kind {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "additionalProperties": false,
            "properties": {
                "path": {"type": "string", "minLength": 1},
                "limit": {"type": "integer", "minimum": 1}
            }
        })
    }

    #[test]
    fn accepts_valid_arguments() {
        validate(&schema(), &json!({"path": "Cargo.toml", "limit": 10})).expect("valid");
    }

    #[test]
    fn rejects_missing_required() {
        let error = validate(&schema(), &json!({})).expect_err("missing");
        assert!(error.contains("path"), "{error}");
    }

    #[test]
    fn rejects_unknown_field() {
        let error = validate(&schema(), &json!({"path": "a", "bogus": true})).expect_err("extra");
        assert!(error.contains("bogus"), "{error}");
    }

    #[test]
    fn rejects_wrong_type() {
        let error = validate(&schema(), &json!({"path": 5})).expect_err("type");
        assert!(error.contains("unexpected type"), "{error}");
    }

    #[test]
    fn rejects_below_minimum() {
        let error = validate(&schema(), &json!({"path": "a", "limit": 0})).expect_err("min");
        assert!(error.contains(">="), "{error}");
    }
}
