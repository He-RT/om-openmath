//! Independent bounded codec state. No credential, HTTP, CAS, tool loop or document grant.
use crate::protocol::{Nullable, generated::*, validation};
use om_llm::single_turn::{self, Decoder, Delta, Error};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

struct Slot {
    decoder: Decoder,
    reply: NativeModelCodecReply,
}
/// Owned by one native host, separately locked from its source/CAS/control owners.
#[derive(Default)]
pub struct ModelChannel {
    slots: BTreeMap<String, Slot>,
    used: BTreeSet<String>,
    contexts: BTreeSet<String>,
}
fn schema() -> &'static Value {
    static ROOT: OnceLock<Value> = OnceLock::new();
    ROOT.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../docs/design/native-model-codec.schema.json"
        ))
        .expect("checked model schema")
    })
}
fn decode(bytes: &[u8]) -> Result<NativeModelCodecCommand, String> {
    let value =
        om_agent_contract::json::strict_json_bounded(bytes, crate::protocol::MAX_CONTROL_BYTES)
            .map_err(str::to_owned)?;
    validation::validate(
        &value,
        &schema()["$defs"]["NativeModelCodecCommand"],
        schema(),
    )
    .map_err(|_| "INVALID_ARGUMENT".to_owned())?;
    serde_json::from_value(value).map_err(|_| "INVALID_ARGUMENT".into())
}
fn category(error: Error) -> NativeModelCodecReplyErrorCode {
    use NativeModelCodecReplyErrorCode as E;
    match error {
        Error::Input => E::Input,
        Error::Unsupported => E::Unsupported,
        Error::Replay => E::Replay,
        Error::Response => E::Response,
        Error::Incomplete => E::Incomplete,
        Error::Limit => E::Limit,
        Error::Http { .. } => E::Http,
        Error::Cancelled => E::Cancelled,
        Error::Closed => E::Closed,
        Error::Provider => E::Provider,
    }
}
impl ModelChannel {
    /// Actual raw native command; an unguessable codec identity never confers a tool/IO grant.
    pub fn command(
        &mut self,
        runtime: &str,
        bytes: &[u8],
    ) -> Result<NativeModelCodecReply, String> {
        let command = decode(bytes)?;
        let value = serde_json::to_value(&command).map_err(|_| "INTERNAL_ERROR")?;
        if value["runtime_instance_id"] != runtime {
            return Err("STALE_RUNTIME".into());
        }
        if let NativeModelCodecCommand::NativeModelCodecPrepare(c) = command {
            if self.slots.len() >= 8 || self.used.len() >= 4096 {
                return Err("BUDGET_EXCEEDED".into());
            }
            if self.used.contains(&c.request_ref) || self.contexts.contains(&c.context_ref) {
                return Err("INVALID_REFERENCE".into());
            }
            let value = om_agent_contract::json::strict_json(c.request_json.as_bytes())
                .map_err(str::to_owned)?;
            let request: single_turn::Request =
                serde_json::from_value(value).map_err(|_| "INVALID_ARGUMENT")?;
            let (wire, decoder) =
                single_turn::prepare(&request, &c.message_id, &c.task_id, &c.agent_turn_id)
                    .map_err(|_| "INVALID_ARGUMENT")?;
            let mut token = [0u8; 24];
            getrandom::fill(&mut token).map_err(|_| "INTERNAL_ERROR")?;
            let id = format!(
                "codec-{}",
                token.iter().map(|b| format!("{b:02x}")).collect::<String>()
            );
            let reply = NativeModelCodecReply {
                protocol_version: 1,
                runtime_instance_id: runtime.into(),
                codec_ref: id.clone(),
                request_ref: c.request_ref.clone(),
                context_ref: c.context_ref,
                task_id: c.task_id,
                agent_turn_id: c.agent_turn_id,
                message_id: c.message_id,
                phase: NativeModelCodecReplyPhase::Prepared,
                stream: wire.stream,
                body_json: Nullable(Some(
                    String::from_utf8(wire.body).map_err(|_| "INTERNAL_ERROR")?,
                )),
                text_deltas: vec![],
                result_json: Nullable(None),
                error_code: Nullable(None),
                http_status: Nullable(None),
            };
            let mut retained = reply.clone();
            retained.body_json = Nullable(None);
            self.used.insert(c.request_ref);
            self.contexts.insert(reply.context_ref.clone());
            self.slots.insert(
                id,
                Slot {
                    decoder,
                    reply: retained,
                },
            );
            return Ok(reply);
        }
        let id = value["codec_ref"]
            .as_str()
            .ok_or("INVALID_ARGUMENT")?
            .to_owned();
        let slot = self.slots.get_mut(&id).ok_or("INVALID_REFERENCE")?;
        let mut reply = slot.reply.clone();
        let mut terminal = false;
        let result: Result<(), Error> = match command {
            NativeModelCodecCommand::NativeModelCodecHeaders(c) => {
                reply.http_status = Nullable(Some(c.status));
                reply.phase = NativeModelCodecReplyPhase::Headers;
                slot.decoder.headers(c.status as u16)
            }
            NativeModelCodecCommand::NativeModelCodecFeed(c) => {
                let bytes = c.bytes.into_iter().map(|b| b as u8).collect::<Vec<_>>();
                reply.phase = NativeModelCodecReplyPhase::Streaming;
                slot.decoder.feed(&bytes).map(|deltas| {
                    reply.text_deltas = deltas
                        .into_iter()
                        .filter_map(|d| match d {
                            Delta::Text(t) => Some(t),
                            _ => None,
                        })
                        .collect();
                })
            }
            NativeModelCodecCommand::NativeModelCodecFinish(_) => {
                terminal = true;
                reply.phase = NativeModelCodecReplyPhase::Decoded;
                slot.decoder.finish().and_then(|result| {
                    let json = serde_json::to_string(&result).map_err(|_| Error::Response)?;
                    if json.len() > 4 * 1024 * 1024 {
                        return Err(Error::Limit);
                    }
                    reply.result_json = Nullable(Some(json));
                    Ok(())
                })
            }
            NativeModelCodecCommand::NativeModelCodecCancel(_) => {
                terminal = true;
                slot.decoder.cancel();
                reply.phase = NativeModelCodecReplyPhase::Cancelled;
                Ok(())
            }
            NativeModelCodecCommand::NativeModelCodecPrepare(_) => unreachable!(),
        };
        if let Err(error) = result {
            reply.phase = NativeModelCodecReplyPhase::Failed;
            reply.error_code = Nullable(Some(category(error)));
            terminal = true;
        }
        slot.reply = reply.clone();
        slot.reply.text_deltas.clear();
        if terminal {
            self.slots.remove(&id);
        }
        Ok(reply)
    }
    /// Revoke pending decoders; no response/cache can recreate a channel after host close.
    pub fn close(&mut self) {
        for slot in self.slots.values_mut() {
            slot.decoder.cancel();
        }
        self.slots.clear();
    }
}
