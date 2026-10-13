//! Strict bounded NDJSON, including duplicate JSON member and UTF8 rejection.
use crate::protocol::{generated::AgentIPCFrame, validation};
use serde_json::Value;
use std::sync::OnceLock;

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
/// Parse bounded JSON with the same canonical/provider duplicate/UTF8 protection.
pub fn strict_json(bytes: &[u8]) -> Result<Value, &'static str> {
    om_agent_contract::json::strict_json(bytes)
}
pub(super) fn strict_json_bounded(bytes: &[u8], maximum: usize) -> Result<Value, &'static str> {
    om_agent_contract::json::strict_json_bounded(bytes, maximum)
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
