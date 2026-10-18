//! The browser uses the same closed tool contract and structural validator as the native host.
use crate::js_error;
use serde_json::{Value, json};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

fn root() -> &'static Value {
    static ROOT: OnceLock<Value> = OnceLock::new();
    ROOT.get_or_init(|| {
        serde_json::from_str(include_str!("../../../docs/design/agent-tools.schema.json"))
            .expect("checked tool schema")
    })
}
fn entry(name: &str) -> Result<&'static Value, String> {
    root()["x-openmath-catalog"]["model_tools"]
        .as_array()
        .expect("checked tool catalog")
        .iter()
        .find(|tool| tool["name"].as_str() == Some(name))
        .ok_or_else(|| "Unknown agent tool".into())
}
fn expand(value: &Value, depth: usize) -> Result<Value, String> {
    if depth > 64 {
        return Err("Tool schema nesting exceeds limit".into());
    }
    if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
        let target = reference
            .strip_prefix('#')
            .and_then(|path| root().pointer(path))
            .ok_or("Unknown tool schema reference")?;
        return expand(target, depth + 1);
    }
    match value {
        Value::Array(items) => items
            .iter()
            .map(|item| expand(item, depth + 1))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| Ok((key.clone(), expand(value, depth + 1)?)))
            .collect::<Result<serde_json::Map<_, _>, String>>()
            .map(Value::Object),
        _ => Ok(value.clone()),
    }
}
fn validate(name: &str, part: &str, source: &str) -> Result<(), String> {
    entry(name)?;
    let suffix = match part {
        "args" => "args",
        "data" => "data",
        "result" => "result",
        _ => return Err("Unknown tool payload part".into()),
    };
    let schema = &root()["$defs"][format!("{name}_{suffix}")];
    let value = om_agent_contract::json::strict_json_bounded(source.as_bytes(), 2 * 1024 * 1024)
        .map_err(str::to_owned)?;
    om_agent_contract::schema::validate(&value, schema, root())
}
/// Return declarations only for handlers registered by the trusted caller; this is not a grant.
#[wasm_bindgen]
pub fn agent_tool_declarations(names_json: &str) -> Result<String, JsValue> {
    let value = om_agent_contract::json::strict_json_bounded(names_json.as_bytes(), 4096)
        .map_err(js_error)?;
    let names: Vec<String> =
        serde_json::from_value(value).map_err(|_| js_error("Invalid tool names"))?;
    if names.len() > 12
        || names
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != names.len()
    {
        return Err(js_error("Invalid tool names"));
    }
    let tools = names.into_iter().map(|name| {
        let tool = entry(&name)?;
        Ok(json!({"name":name,"label":tool["label"],"description":tool["description"],"parameters":expand(&tool["parameters"],0)?,"modes":tool["modes"]}))
    }).collect::<Result<Vec<Value>, String>>().map_err(|error| js_error(&error))?;
    serde_json::to_string(&tools).map_err(|_| js_error("Cannot encode tools"))
}
/// Validate original argument/result bytes. Scope, permission and freshness checks remain with the owner.
#[wasm_bindgen]
pub fn validate_agent_payload(name: &str, part: &str, source: &str) -> Result<(), JsValue> {
    validate(name, part, source).map_err(|error| js_error(&error))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declarations_expand_refs_and_closed_validation_preserves_conditional_requirements() {
        let declarations =
            agent_tool_declarations(r#"["read_notebook","preview_source"]"#).unwrap();
        assert!(!declarations.contains("$ref"));
        let tools: Value = serde_json::from_str(&declarations).unwrap();
        assert_eq!(tools[0]["parameters"]["additionalProperties"], false);
        assert!(validate("read_notebook", "args", "{}").is_ok());
        assert!(validate("get_function_docs", "args", r#"{"name":"solve"}"#).is_ok());
        assert!(validate("get_function_docs", "args", r#"{"function_id":"om.solve"}"#).is_ok());
        assert!(
            validate(
                "get_function_docs",
                "args",
                r#"{"name":"solve","function_id":"om.solve"}"#
            )
            .is_err()
        );
        assert!(validate("get_function_docs", "args", "{}").is_err());
        for invalid in [
            r#"{"view":"cells"}"#,
            r#"{"unknown":true}"#,
            r#"{"view":"outline","view":"cells"}"#,
        ] {
            assert!(validate("read_notebook", "args", invalid).is_err());
        }
        assert!(
            validate(
                "read_notebook",
                "args",
                r#"{"view":"cells","cell_ids":["a"]}"#
            )
            .is_ok()
        );
        assert!(validate("shell", "args", "{}").is_err());
        assert!(validate("read_notebook", "$defs", "{}").is_err());
        assert!(validate("apply_notebook_patch", "data", "{}").is_err());
    }
}
