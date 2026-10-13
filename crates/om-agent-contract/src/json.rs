//! Strict data-only JSON: decoded duplicate names, UTF8, syntax depth/nodes/bytes bounded.
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::{collections::BTreeSet, fmt};
/// Shared ordinary canonical/provider frame bound.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
/// Independent syntax nesting bound.
pub const MAX_JSON_DEPTH: usize = 64;
/// Independent syntax node bound.
pub const MAX_JSON_NODES: usize = 32768;
struct StrictSeed<'a> {
    depth: usize,
    nodes: &'a mut usize,
}
impl<'de> DeserializeSeed<'de> for StrictSeed<'_> {
    type Value = Value;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Value, D::Error> {
        *self.nodes += 1;
        if self.depth > MAX_JSON_DEPTH || *self.nodes > MAX_JSON_NODES {
            return Err(serde::de::Error::custom("JSON_BOUND_EXCEEDED"));
        }
        decoder.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for StrictSeed<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded unique-member JSON")
    }
    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("INVALID_JSON_NUMBER"))
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.into()))
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = access.next_element_seed(StrictSeed {
            depth: self.depth + 1,
            nodes: self.nodes,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        let mut keys = BTreeSet::new();
        while let Some(key) = access.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(serde::de::Error::custom("DUPLICATE_JSON_MEMBER"));
            }
            let value = access.next_value_seed(StrictSeed {
                depth: self.depth + 1,
                nodes: self.nodes,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}
/// Parse data-only JSON without silently replacing invalid UTF8 or duplicate fields.
/// Error strings reveal no peer-controlled text or credential-shaped content.
pub fn strict_json(bytes: &[u8]) -> Result<Value, &'static str> {
    strict_json_bounded(bytes, MAX_FRAME_BYTES)
}
/// Parse with an explicit bound, retaining the shared syntax and UTF8 checks.
pub fn strict_json_bounded(bytes: &[u8], maximum: usize) -> Result<Value, &'static str> {
    if bytes.is_empty() || bytes.len() > maximum {
        return Err("IPC_FRAME_SIZE");
    }
    std::str::from_utf8(bytes).map_err(|_| "IPC_INVALID_UTF8")?;
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let mut nodes = 0;
    let value = StrictSeed {
        depth: 0,
        nodes: &mut nodes,
    }
    .deserialize(&mut decoder)
    .map_err(|_| "IPC_INVALID_JSON")?;
    decoder.end().map_err(|_| "IPC_INVALID_JSON")?;
    Ok(value)
}
