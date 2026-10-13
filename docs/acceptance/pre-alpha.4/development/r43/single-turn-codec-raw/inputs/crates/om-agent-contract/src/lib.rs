//! Canonical OpenMath data generated from one machine contract. No model loop, IO or authority.
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
/// Generated canonical message and part DTOs.
pub mod generated;
pub use generated::*;
/// Strict data-only JSON parser.
pub mod json;
/// Closed local machine-schema validation.
pub mod schema;
/// A required field whose value may be null. Missing and explicitly null remain distinct.
#[derive(Clone, Debug, PartialEq)]
pub struct Nullable<T>(pub Option<T>);
impl<T: Serialize> Serialize for Nullable<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
impl<'de, T: serde::de::DeserializeOwned> Deserialize<'de> for Nullable<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // deserialize_any (through Value) rejects a missing struct field; Option's missing-field
        // fallback would silently treat omission as null, violating the machine contract.
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.is_null() {
            Ok(Self(None))
        } else {
            serde_json::from_value(value)
                .map(|v| Self(Some(v)))
                .map_err(serde::de::Error::custom)
        }
    }
}

/// Validate full canonical message shape against the checked-in authoritative contract.
pub fn validate_message(message: &AgentCanonicalMessage) -> Result<(), String> {
    static ROOT: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let root = ROOT.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../docs/design/native-agent.schema.json"
        ))
        .expect("checked canonical schema")
    });
    let value = serde_json::to_value(message).map_err(|_| "invalid canonical shape")?;
    schema::validate(&value, &root["$defs"]["AgentCanonicalMessage"], root)
}
