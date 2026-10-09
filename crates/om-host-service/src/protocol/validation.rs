//! Bounded structural validation of the checked-in host schemas; never grants task permissions.
use super::MAX_CONTROL_BYTES;
use serde_json::Value;

/// Validate JSON against the supported local contract vocabulary, with precise field paths.
pub fn validate(value: &Value, schema: &Value, root: &Value) -> Result<(), String> {
    check(value, schema, root, "$", 0)
}
fn check(
    value: &Value,
    schema: &Value,
    root: &Value,
    path: &str,
    depth: usize,
) -> Result<(), String> {
    if schema == &Value::Bool(false) {
        return Err(format!("{path}: forbidden by contract"));
    }
    if schema == &Value::Bool(true) {
        return Ok(());
    }
    if let Some(keywords) = schema.as_object() {
        for keyword in keywords.keys() {
            if !matches!(
                keyword.as_str(),
                "$schema"
                    | "x-openmath-contract"
                    | "x-implementation-status"
                    | "$id"
                    | "$defs"
                    | "$ref"
                    | "title"
                    | "description"
                    | "examples"
                    | "default"
                    | "type"
                    | "const"
                    | "enum"
                    | "oneOf"
                    | "anyOf"
                    | "allOf"
                    | "if"
                    | "then"
                    | "else"
                    | "properties"
                    | "required"
                    | "additionalProperties"
                    | "minLength"
                    | "maxLength"
                    | "pattern"
                    | "minimum"
                    | "maximum"
                    | "minItems"
                    | "maxItems"
                    | "items"
                    | "uniqueItems"
            ) {
                return Err(format!("{path}: unsupported contract keyword {keyword}"));
            }
        }
    }
    if depth > 128 {
        return Err(format!("{path}: contract depth exceeded"));
    }
    let recurse = |v, s, p: &str| check(v, s, root, p, depth + 1);
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let pointer = reference
            .strip_prefix('#')
            .ok_or_else(|| format!("{path}: external references are forbidden"))?;
        let target = root
            .pointer(pointer)
            .ok_or_else(|| format!("{path}: missing schema reference"))?;
        recurse(value, target, path)?;
    }
    if let Some(expected) = schema.get("const")
        && value != expected
    {
        return Err(format!("{path}: wrong constant"));
    }
    if let Some(choices) = schema.get("enum").and_then(Value::as_array)
        && !choices.contains(value)
    {
        return Err(format!("{path}: unknown enum value"));
    }
    for kind in ["oneOf", "anyOf"] {
        if let Some(choices) = schema.get(kind).and_then(Value::as_array) {
            let count = choices
                .iter()
                .filter(|s| recurse(value, s, path).is_ok())
                .count();
            if count == 0 || kind == "oneOf" && count != 1 {
                return Err(format!("{path}: {kind} mismatch"));
            }
        }
    }
    if let Some(all) = schema.get("allOf").and_then(Value::as_array) {
        for s in all {
            recurse(value, s, path)?;
        }
    }
    if let Some(condition) = schema.get("if") {
        let branch = if recurse(value, condition, path).is_ok() {
            "then"
        } else {
            "else"
        };
        if let Some(s) = schema.get(branch) {
            recurse(value, s, path)?;
        }
    }
    if let Some(kind) = schema.get("type") {
        let matches = |s: &str| match s {
            "null" => value.is_null(),
            "boolean" => value.is_boolean(),
            "string" => value.is_string(),
            "object" => value.is_object(),
            "array" => value.is_array(),
            "number" => value.is_number(),
            "integer" => value
                .as_f64()
                .is_some_and(|v| v.is_finite() && v.fract() == 0.0),
            _ => false,
        };
        let good = kind.as_str().is_some_and(matches)
            || kind
                .as_array()
                .is_some_and(|a| a.iter().any(|s| s.as_str().is_some_and(matches)));
        if !good {
            return Err(format!("{path}: wrong JSON type"));
        }
    }
    if let Some(object) = value.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for name in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(name) {
                    return Err(format!("{path}.{name}: required field missing"));
                }
            }
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        for (name, v) in object {
            let next = format!("{path}.{name}");
            if let Some(s) = properties.and_then(|p| p.get(name)) {
                recurse(v, s, &next)?;
            } else if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                return Err(format!("{next}: unknown field"));
            } else if let Some(s) = schema.get("additionalProperties").filter(|s| s.is_object()) {
                recurse(v, s, &next)?;
            }
        }
    }
    if let Some(text) = value.as_str() {
        let len = text.chars().count() as u64;
        range(len as f64, schema, "minLength", "maxLength", path)?;
        if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
            let good = match pattern {
                "^[A-Za-z0-9][A-Za-z0-9._:-]*$" => {
                    text.as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_alphanumeric)
                        && text
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || b"._:-".contains(&c))
                }
                "^[a-f0-9]{64}$" => {
                    text.len() == 64
                        && text
                            .bytes()
                            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                }
                _ => return Err(format!("{path}: unsupported contract pattern")),
            };
            if !good {
                return Err(format!("{path}: malformed identity/hash"));
            }
        }
    }
    if let Some(number) = value.as_f64() {
        range(number, schema, "minimum", "maximum", path)?;
    }
    if let Some(array) = value.as_array() {
        range(array.len() as f64, schema, "minItems", "maxItems", path)?;
        if schema.get("uniqueItems") == Some(&Value::Bool(true)) {
            for (i, v) in array.iter().enumerate() {
                if array[..i].contains(v) {
                    return Err(format!("{path}: duplicate item"));
                }
            }
        }
        if let Some(s) = schema.get("items") {
            for (i, v) in array.iter().enumerate() {
                recurse(v, s, &format!("{path}[{i}]"))?;
            }
        }
    }
    Ok(())
}
fn range(value: f64, schema: &Value, min: &str, max: &str, path: &str) -> Result<(), String> {
    if schema
        .get(min)
        .and_then(Value::as_f64)
        .is_some_and(|n| value < n)
        || schema
            .get(max)
            .and_then(Value::as_f64)
            .is_some_and(|n| value > n)
    {
        return Err(format!("{path}: value outside contract bounds"));
    }
    Ok(())
}
/// Decode a bounded frame only after structural validation. Business admission remains separate.
pub fn decode<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    schema: &Value,
    root: &Value,
) -> Result<T, String> {
    if bytes.len() > MAX_CONTROL_BYTES {
        return Err("control frame exceeds byte budget".into());
    }
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| "invalid UTF-8/JSON frame".to_owned())?;
    validate(&value, schema, root)?;
    serde_json::from_value(value).map_err(|e| format!("invalid typed frame: {e}"))
}
