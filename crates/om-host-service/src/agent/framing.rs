//! Strict bounded NDJSON, including duplicate JSON member and UTF8 rejection.
use crate::protocol::{generated::AgentIPCFrame, validation};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::{collections::BTreeSet, fmt, sync::OnceLock};

/// Ordinary private IPC payloads never carry large artifact bytes.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
/// Bound syntax independently from the generated business shapes.
pub const MAX_JSON_DEPTH: usize = 64;
/// Bound container/atom count independently from byte length.
pub const MAX_JSON_NODES: usize = 32768;
static SCHEMA: OnceLock<Value> = OnceLock::new();
/// Self-contained checked-in contract; external schema/network lookup is impossible.
pub fn schema() -> &'static Value {
    SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/design/native-agent.schema.json"
        ))
        .expect("checked-in Agent schema")
    })
}
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
pub(super) fn strict_json_bounded(bytes: &[u8], maximum: usize) -> Result<Value, &'static str> {
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
/// Decode only known typed frame fields, then validate the complete local shape.
pub fn decode(bytes: &[u8]) -> Result<AgentIPCFrame, &'static str> {
    let value = strict_json(bytes)?;
    validation::validate(&value, &schema()["$defs"]["AgentIPCFrame"], schema())
        .map_err(|_| "IPC_INVALID_SHAPE")?;
    serde_json::from_value(value).map_err(|_| "IPC_INVALID_SHAPE")
}
/// A single private pipe decoder. Framing failure poisons only this connection.
/// New connections get new decoders; incomplete EOF is never silently accepted.
#[derive(Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    failed: bool,
}
impl FrameDecoder {
    /// Deliver complete real frames, preserving UTF8 boundaries across arbitrary chunks.
    /// A malformed chunk returns no frames from that chunk; no caller has acted on them yet.
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<AgentIPCFrame>, &'static str> {
        if self.failed {
            return Err("IPC_CONNECTION_FAILED");
        }
        let result = self.consume(chunk);
        if result.is_err() {
            self.failed = true;
            self.buffer.clear();
        }
        result
    }
    fn consume(&mut self, chunk: &[u8]) -> Result<Vec<AgentIPCFrame>, &'static str> {
        let mut frames = Vec::new();
        for &byte in chunk {
            if byte == b'\n' {
                if self.buffer.last() == Some(&b'\r') {
                    self.buffer.pop();
                }
                frames.push(decode(&self.buffer)?);
                self.buffer.clear();
                if frames.len() > 32 {
                    return Err("IPC_BATCH_SIZE");
                }
            } else {
                if self.buffer.len() >= MAX_FRAME_BYTES {
                    return Err("IPC_FRAME_SIZE");
                }
                self.buffer.push(byte);
            }
        }
        Ok(frames)
    }
    /// Close/join paths check EOF explicitly; a partial frame is a transport failure.
    pub fn finish(&mut self) -> Result<(), &'static str> {
        if self.failed {
            return Err("IPC_CONNECTION_FAILED");
        }
        if self.buffer.is_empty() {
            Ok(())
        } else {
            self.failed = true;
            self.buffer.clear();
            Err("IPC_INCOMPLETE_EOF")
        }
    }
}
