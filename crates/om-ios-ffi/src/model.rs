//! Mobile ownership wrapper for the same single-turn codec used on Mac and Web.
use om_llm::single_turn::{self, Decoder, Delta, Request};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Prepare {
        id: String,
        request: Request,
        message: String,
        task: String,
        turn: String,
    },
    Headers {
        id: String,
        status: u16,
    },
    Feed {
        id: String,
        bytes: Vec<u8>,
    },
    Finish {
        id: String,
    },
    Cancel {
        id: String,
    },
}

#[derive(Default)]
pub(crate) struct ModelStore(BTreeMap<String, Decoder>);
impl ModelStore {
    pub(crate) fn command(&mut self, source: &str) -> Result<String, String> {
        if source.len() > 8 * 1024 * 1024 {
            return Err("Model input exceeds limit".into());
        }
        let command: Command = serde_json::from_str(source).map_err(|_| "Invalid model command")?;
        let value: Value = match command {
            Command::Prepare {
                id,
                request,
                message,
                task,
                turn,
            } => {
                if id.is_empty() || id.len() > 256 || self.0.contains_key(&id) || self.0.len() >= 4
                {
                    return Err("Invalid or duplicate model identity".into());
                }
                let (wire, decoder) = single_turn::prepare(&request, &message, &task, &turn)
                    .map_err(|e| e.to_string())?;
                let body = String::from_utf8(wire.body).map_err(|_| "Invalid model body")?;
                self.0.insert(id, decoder);
                json!({"body":body})
            }
            Command::Headers { id, status } => {
                self.0
                    .get_mut(&id)
                    .ok_or("Unknown model request")?
                    .headers(status)
                    .map_err(|e| e.to_string())?;
                json!({"ok":true})
            }
            Command::Feed { id, bytes } => {
                if bytes.len() > 4096 {
                    return Err("Model chunk exceeds limit".into());
                }
                let deltas = self
                    .0
                    .get_mut(&id)
                    .ok_or("Unknown model request")?
                    .feed(&bytes)
                    .map_err(|e| e.to_string())?;
                let text: String = deltas
                    .into_iter()
                    .filter_map(|d| match d {
                        Delta::Text(t) => Some(t),
                        _ => None,
                    })
                    .collect();
                json!({"text":text})
            }
            Command::Finish { id } => {
                let mut decoder = self.0.remove(&id).ok_or("Unknown model request")?;
                serde_json::to_value(decoder.finish().map_err(|e| e.to_string())?)
                    .map_err(|_| "Invalid model result")?
            }
            Command::Cancel { id } => {
                if let Some(mut decoder) = self.0.remove(&id) {
                    decoder.cancel();
                }
                json!({"ok":true})
            }
        };
        serde_json::to_string(&value).map_err(|_| "Cannot encode model reply".into())
    }
}
