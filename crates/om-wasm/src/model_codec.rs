//! Browser ownership wrapper for the same immutable codec used by the native agent.
use crate::js_error;
use om_llm::single_turn::{self, Decoder, Delta, Request};
use wasm_bindgen::prelude::*;

/// One model request. It has no endpoint, credentials, tool executor or loop.
#[wasm_bindgen]
pub struct ModelCodec {
    decoder: Decoder,
    body: String,
}

#[wasm_bindgen]
impl ModelCodec {
    /// Freeze the host's canonical snapshot and trusted output identities.
    #[wasm_bindgen(constructor)]
    pub fn new(
        source: &str,
        message_id: &str,
        task_id: &str,
        turn_id: &str,
    ) -> Result<Self, JsValue> {
        if source.len() > 8 * 1024 * 1024 {
            return Err(js_error("Model input exceeds limit"));
        }
        let request: Request =
            serde_json::from_str(source).map_err(|_| js_error("Invalid model input"))?;
        let (wire, decoder) = single_turn::prepare(&request, message_id, task_id, turn_id)
            .map_err(|error| js_error(&error.to_string()))?;
        let body = String::from_utf8(wire.body).map_err(|_| js_error("Invalid model body"))?;
        Ok(Self { decoder, body })
    }
    /// Actual serialized provider body; authentication is applied by the separate transport.
    pub fn body(&self) -> String {
        self.body.clone()
    }
    /// Accept the transport's status once, before feeding bytes.
    pub fn status(&mut self, status: u16) -> Result<(), JsValue> {
        self.decoder
            .headers(status)
            .map_err(|error| js_error(&error.to_string()))
    }
    /// Text deltas only. Partial tool arguments never grant permission to execute a tool.
    pub fn chunk(&mut self, text: &str) -> Result<String, JsValue> {
        let deltas = self
            .decoder
            .feed(text.as_bytes())
            .map_err(|error| js_error(&error.to_string()))?;
        Ok(deltas
            .into_iter()
            .filter_map(|delta| match delta {
                Delta::Text(text) => Some(text),
                Delta::ToolArguments { .. } => None,
            })
            .collect())
    }
    /// Complete canonical message, original call IDs, replay scope and provider-observed usage.
    pub fn finish(&mut self) -> Result<String, JsValue> {
        let result = self
            .decoder
            .finish()
            .map_err(|error| js_error(&error.to_string()))?;
        serde_json::to_string(&result).map_err(|_| js_error("Cannot encode model result"))
    }
    /// Cancel this request permanently.
    pub fn cancel(&mut self) {
        self.decoder.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn browser_wrapper_preserves_canonical_scope_and_closes_after_completion() {
        let input = json!({"binding":{"provider_ref":"provider-1","route_ref":"route-1","model_ref":"model-1","adapter_version":"single-turn-v1","protocol":"chat_completions"},
            "api_model":"loopback","messages":[{"canonical":{"record_type":"canonical_agent_message","message_version":1,"message_id":"user-1","role":"user","task_id":"task-1","agent_turn_id":null,"parts":[{"kind":"text","part_id":"text-1","text":"读取笔记本"}]},"replay":null}],
            "results":[],"tools":[],"max_output_tokens":1024,"temperature":null,"stream":false});
        let mut codec =
            ModelCodec::new(&input.to_string(), "assistant-1", "task-1", "turn-1").unwrap();
        let body: Value = serde_json::from_str(&codec.body()).unwrap();
        assert_eq!(body["messages"][0]["content"], "读取笔记本");
        codec.status(200).unwrap();
        codec.chunk(r#"{"id":"provider-response","model":"loopback","choices":[{"index":0,"message":{"role":"assistant","content":"看过了"},"finish_reason":"stop"}]}"#).unwrap();
        let result: Value = serde_json::from_str(&codec.finish().unwrap()).unwrap();
        assert_eq!(result["message"]["message_id"], "assistant-1");
        assert_eq!(result["message"]["task_id"], "task-1");
        assert_eq!(result["message"]["agent_turn_id"], "turn-1");
        assert_eq!(result["message"]["parts"][0]["text"], "看过了");
        assert!(result["usage"]["input"].is_null());
        assert!(codec.finish().is_err());
    }
}
