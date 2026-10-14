//! Incremental provider stream/body decoder. A finish never initiates another model/tool round.
use super::*;
use crate::{SseDecoder, SseMessage};
use om_agent_contract::{
    AgentCanonicalMessageRecordType, AgentTextPart, AgentTextPartKind, AgentToolName,
    AgentToolProposalPart, AgentToolProposalPartKind, Nullable,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_VALUE: usize = 1024 * 1024;
struct Block {
    kind: String,
    text: String,
    signature: String,
    data: String,
    id: String,
    name: String,
    args: String,
    initial: Value,
    closed: bool,
}
impl Block {
    fn new(kind: &str) -> Self {
        Self {
            kind: kind.into(),
            text: String::new(),
            signature: String::new(),
            data: String::new(),
            id: String::new(),
            name: String::new(),
            args: String::new(),
            initial: json!({}),
            closed: false,
        }
    }
}
/// A single explicitly dispatched request. Poison/cancel/EOF are final for this instance.
pub struct Decoder {
    scope: ResponseScope,
    status: Option<u16>,
    sse: SseDecoder,
    body: Vec<u8>,
    bytes: usize,
    closed: bool,
    failed: bool,
    cancelled: bool,
    response_id: Option<String>,
    provider_model: Option<String>,
    text: String,
    refusal: String,
    reasoning: String,
    calls: BTreeMap<u32, Block>,
    blocks: BTreeMap<u32, Block>,
    usage: Usage,
    reason: Option<String>,
    terminal: bool,
    started: bool,
}
fn value(bytes: &[u8]) -> std::result::Result<Value, Error> {
    om_agent_contract::json::strict_json_bounded(bytes, MAX_VALUE).map_err(|_| Error::Response)
}
fn object(v: &Value) -> std::result::Result<(), Error> {
    if v.is_object() {
        Ok(())
    } else {
        Err(Error::Response)
    }
}
fn str_field<'a>(v: &'a Value, key: &str) -> std::result::Result<Option<&'a str>, Error> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        _ => Err(Error::Response),
    }
}
fn index(v: &Value) -> std::result::Result<u32, Error> {
    v.as_u64()
        .and_then(|i| u32::try_from(i).ok())
        .filter(|i| *i < 128)
        .ok_or(Error::Response)
}
fn append(s: &mut String, delta: &str, max: usize) -> std::result::Result<(), Error> {
    if s.len().checked_add(delta.len()).is_none_or(|n| n > max) {
        return Err(Error::Limit);
    }
    s.push_str(delta);
    Ok(())
}
fn token(v: &Value, key: &str) -> std::result::Result<Option<u64>, Error> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(n) => n
            .as_u64()
            .filter(|n| *n <= 9_007_199_254_740_991)
            .map(Some)
            .ok_or(Error::Response),
    }
}
impl Decoder {
    /// Freeze actual native request identity; this codec has no transport/client/profile lookup.
    pub fn new(scope: ResponseScope) -> Self {
        Self {
            scope,
            status: None,
            sse: SseDecoder::with_limit(MAX_VALUE),
            body: vec![],
            bytes: 0,
            closed: false,
            failed: false,
            cancelled: false,
            response_id: None,
            provider_model: None,
            text: String::new(),
            refusal: String::new(),
            reasoning: String::new(),
            calls: BTreeMap::new(),
            blocks: BTreeMap::new(),
            usage: Usage::default(),
            reason: None,
            terminal: false,
            started: false,
        }
    }
    /// Accept the actual HTTP status exactly once, before any body bytes.
    pub fn headers(&mut self, status: u16) -> std::result::Result<(), Error> {
        if self.status.is_some() || self.closed || self.failed || !(100..=599).contains(&status) {
            return Err(Error::Closed);
        }
        self.status = Some(status);
        Ok(())
    }
    /// Feed actual transport bytes. Returned deltas are presentation data, not tool admission.
    pub fn feed(&mut self, bytes: &[u8]) -> std::result::Result<Vec<Delta>, Error> {
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if self.closed || self.failed || self.status.is_none() {
            return Err(Error::Closed);
        }
        let result = self.ingest(bytes);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn ingest(&mut self, bytes: &[u8]) -> std::result::Result<Vec<Delta>, Error> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .filter(|n| *n <= MAX_BYTES)
            .ok_or(Error::Limit)?;
        if !self.status.is_some_and(|s| (200..300).contains(&s)) {
            return Ok(vec![]);
        }
        if !self.scope.stream {
            if self.body.len() + bytes.len() > MAX_VALUE {
                return Err(Error::Limit);
            }
            self.body.extend_from_slice(bytes);
            return Ok(vec![]);
        }
        let events = self.sse.feed_bytes(bytes).map_err(|_| Error::Response)?;
        let mut deltas = vec![];
        for event in events {
            match self.scope.binding.protocol {
                Protocol::ChatCompletions => self.chat_event(&event, &mut deltas)?,
                Protocol::AnthropicMessages => self.anthropic_event(&event, &mut deltas)?,
            }
        }
        Ok(deltas)
    }
    fn identity(&mut self, id: &str, model: &str) -> std::result::Result<(), Error> {
        if id.is_empty() || id.len() > 256 || model.is_empty() || model.len() > 256 {
            return Err(Error::Response);
        }
        if self.response_id.as_ref().is_some_and(|v| v != id)
            || self.provider_model.as_ref().is_some_and(|v| v != model)
        {
            return Err(Error::Response);
        }
        self.response_id = Some(id.into());
        self.provider_model = Some(model.into());
        Ok(())
    }
    fn chat_usage(&mut self, u: &Value) -> std::result::Result<(), Error> {
        if u.is_null() {
            return Ok(());
        }
        object(u)?;
        let usage = Usage {
            input: token(u, "prompt_tokens")?,
            output: token(u, "completion_tokens")?,
            total: token(u, "total_tokens")?,
            cache_read: token(&u["prompt_tokens_details"], "cached_tokens")?,
            cache_write: token(&u["prompt_tokens_details"], "cache_write_tokens")?,
        };
        if self.usage != Usage::default() && self.usage != usage {
            return Err(Error::Response);
        }
        self.usage = usage;
        Ok(())
    }
    fn chat_event(
        &mut self,
        event: &SseMessage,
        out: &mut Vec<Delta>,
    ) -> std::result::Result<(), Error> {
        if event.data.trim() == "[DONE]" {
            if self.terminal || self.reason.is_none() {
                return Err(Error::Incomplete);
            }
            self.terminal = true;
            return Ok(());
        }
        if self.terminal {
            return Err(Error::Response);
        }
        let v = value(event.data.as_bytes())?;
        object(&v)?;
        if v.get("error").is_some_and(|e| !e.is_null()) {
            return Err(Error::Provider);
        }
        self.identity(
            str_field(&v, "id")?.ok_or(Error::Response)?,
            str_field(&v, "model")?.ok_or(Error::Response)?,
        )?;
        if let Some(u) = v.get("usage") {
            self.chat_usage(u)?;
        }
        let choices = v["choices"].as_array().ok_or(Error::Response)?;
        if choices.is_empty() {
            if self.reason.is_none() {
                return Err(Error::Response);
            }
            return Ok(());
        }
        if choices.len() != 1 || choices[0]["index"].as_u64() != Some(0) {
            return Err(Error::Unsupported);
        }
        let choice = &choices[0];
        let delta = &choice["delta"];
        object(delta)?;
        if str_field(delta, "role")?.is_some_and(|s| s != "assistant") {
            return Err(Error::Response);
        }
        if delta.get("audio").is_some_and(|x| !x.is_null())
            || delta.get("function_call").is_some_and(|x| !x.is_null())
        {
            return Err(Error::Unsupported);
        }
        if let Some(t) = str_field(delta, "content")? {
            if self.reason.is_some() && !t.is_empty() {
                return Err(Error::Response);
            }
            append(&mut self.text, t, 262144)?;
            if !t.is_empty() {
                out.push(Delta::Text(t.into()));
            }
        }
        if let Some(t) = str_field(delta, "refusal")? {
            if self.reason.is_some() && !t.is_empty() {
                return Err(Error::Response);
            }
            append(&mut self.refusal, t, 262144)?;
        }
        if let Some(t) = str_field(delta, "reasoning_content")? {
            if self.reason.is_some() && !t.is_empty() {
                return Err(Error::Response);
            }
            append(&mut self.reasoning, t, 262144)?;
        }
        if let Some(calls) = delta.get("tool_calls").filter(|c| !c.is_null()) {
            if self.reason.is_some() {
                return Err(Error::Response);
            }
            for c in calls.as_array().ok_or(Error::Response)? {
                object(c)?;
                let i = index(&c["index"])?;
                if i >= 32 {
                    return Err(Error::Limit);
                }
                if str_field(c, "type")?.is_some_and(|t| t != "function") {
                    return Err(Error::Unsupported);
                }
                let call = self
                    .calls
                    .entry(i)
                    .or_insert_with(|| Block::new("tool_use"));
                if let Some(id) = str_field(c, "id")? {
                    append(&mut call.id, id, 256)?;
                }
                if let Some(f) = c.get("function").filter(|f| !f.is_null()) {
                    object(f)?;
                    if let Some(n) = str_field(f, "name")? {
                        append(&mut call.name, n, 256)?;
                    }
                    if let Some(a) = str_field(f, "arguments")? {
                        append(&mut call.args, a, 65536)?;
                        out.push(Delta::ToolArguments {
                            index: i,
                            fragment: a.into(),
                        });
                    }
                }
            }
        }
        if let Some(r) = str_field(choice, "finish_reason")? {
            if self.reason.as_ref().is_some_and(|old| old != r) {
                return Err(Error::Response);
            }
            self.reason = Some(r.into());
        }
        Ok(())
    }
    fn anthropic_usage(&mut self, u: &Value) -> std::result::Result<(), Error> {
        object(u)?;
        fn update(old: &mut Option<u64>, new: Option<u64>) -> std::result::Result<(), Error> {
            if let Some(n) = new {
                if old.is_some_and(|o| n < o) {
                    return Err(Error::Response);
                }
                *old = Some(n);
            }
            Ok(())
        }
        update(&mut self.usage.input, token(u, "input_tokens")?)?;
        update(&mut self.usage.output, token(u, "output_tokens")?)?;
        update(
            &mut self.usage.cache_read,
            token(u, "cache_read_input_tokens")?,
        )?;
        update(
            &mut self.usage.cache_write,
            token(u, "cache_creation_input_tokens")?,
        )?;
        Ok(())
    }
    fn anthropic_event(
        &mut self,
        event: &SseMessage,
        out: &mut Vec<Delta>,
    ) -> std::result::Result<(), Error> {
        if self.terminal {
            return Err(Error::Response);
        }
        let v = value(event.data.as_bytes())?;
        object(&v)?;
        let kind = str_field(&v, "type")?.ok_or(Error::Response)?;
        if event.event.as_deref().is_some_and(|name| name != kind) {
            return Err(Error::Response);
        }
        if kind == "ping" {
            return Ok(());
        }
        if kind == "error" {
            return Err(Error::Provider);
        }
        if kind == "message_start" {
            if self.started {
                return Err(Error::Response);
            }
            let m = &v["message"];
            object(m)?;
            if m["role"] != "assistant" || m["content"].as_array().is_none_or(|v| !v.is_empty()) {
                return Err(Error::Response);
            }
            self.identity(
                str_field(m, "id")?.ok_or(Error::Response)?,
                str_field(m, "model")?.ok_or(Error::Response)?,
            )?;
            self.anthropic_usage(&m["usage"])?;
            self.started = true;
            return Ok(());
        }
        if !self.started {
            return Err(Error::Response);
        }
        match kind {
            "content_block_start" => {
                if self.reason.is_some() {
                    return Err(Error::Response);
                }
                let i = index(&v["index"])?;
                if i as usize != self.blocks.len() {
                    return Err(Error::Response);
                }
                let b = &v["content_block"];
                object(b)?;
                let kind = str_field(b, "type")?.ok_or(Error::Response)?;
                let mut block = Block::new(kind);
                match kind {
                    "text" => {
                        block.text = str_field(b, "text")?.ok_or(Error::Response)?.into();
                    }
                    "tool_use" => {
                        if self
                            .blocks
                            .values()
                            .filter(|b| b.kind == "tool_use")
                            .count()
                            >= 32
                        {
                            return Err(Error::Limit);
                        }
                        block.id = str_field(b, "id")?.ok_or(Error::Response)?.into();
                        block.name = str_field(b, "name")?.ok_or(Error::Response)?.into();
                        if !b["input"].is_object() {
                            return Err(Error::Response);
                        }
                        block.initial = b["input"].clone();
                    }
                    "thinking" => {
                        block.text = str_field(b, "thinking")?.ok_or(Error::Response)?.into();
                        block.signature = str_field(b, "signature")?.unwrap_or("").into();
                    }
                    "redacted_thinking" => {
                        block.data = str_field(b, "data")?.ok_or(Error::Response)?.into();
                    }
                    _ => return Err(Error::Unsupported),
                }
                self.blocks.insert(i, block);
            }
            "content_block_delta" => {
                let i = index(&v["index"])?;
                let block = self.blocks.get_mut(&i).ok_or(Error::Response)?;
                if block.closed || self.reason.is_some() {
                    return Err(Error::Response);
                }
                let d = &v["delta"];
                object(d)?;
                match str_field(d, "type")?.ok_or(Error::Response)? {
                    "text_delta" if block.kind == "text" => {
                        let t = str_field(d, "text")?.ok_or(Error::Response)?;
                        append(&mut block.text, t, 262144)?;
                        out.push(Delta::Text(t.into()));
                    }
                    "input_json_delta" if block.kind == "tool_use" => {
                        let a = str_field(d, "partial_json")?.ok_or(Error::Response)?;
                        append(&mut block.args, a, 65536)?;
                        out.push(Delta::ToolArguments {
                            index: i,
                            fragment: a.into(),
                        });
                    }
                    "thinking_delta" if block.kind == "thinking" => append(
                        &mut block.text,
                        str_field(d, "thinking")?.ok_or(Error::Response)?,
                        262144,
                    )?,
                    "signature_delta" if block.kind == "thinking" => append(
                        &mut block.signature,
                        str_field(d, "signature")?.ok_or(Error::Response)?,
                        262144,
                    )?,
                    _ => return Err(Error::Unsupported),
                }
            }
            "content_block_stop" => {
                let i = index(&v["index"])?;
                let b = self.blocks.get_mut(&i).ok_or(Error::Response)?;
                if b.closed {
                    return Err(Error::Response);
                }
                b.closed = true;
            }
            "message_delta" => {
                if self.blocks.values().any(|b| !b.closed) {
                    return Err(Error::Response);
                }
                let d = &v["delta"];
                object(d)?;
                if let Some(r) = str_field(d, "stop_reason")? {
                    if self.reason.as_ref().is_some_and(|old| old != r) {
                        return Err(Error::Response);
                    }
                    self.reason = Some(r.into());
                }
                if let Some(u) = v.get("usage") {
                    self.anthropic_usage(u)?;
                }
            }
            "message_stop" => {
                if self.reason.is_none() || self.blocks.values().any(|b| !b.closed) {
                    return Err(Error::Incomplete);
                }
                self.terminal = true;
            }
            _ => {} // Versioned additive non-content events are ignored; unknown content blocks are refused.
        }
        Ok(())
    }
    fn body_response(&mut self) -> std::result::Result<(), Error> {
        let body = value(&self.body)?;
        object(&body)?;
        if body.get("error").is_some_and(|e| !e.is_null()) {
            return Err(Error::Provider);
        }
        match self.scope.binding.protocol {
            Protocol::ChatCompletions => {
                self.identity(
                    str_field(&body, "id")?.ok_or(Error::Response)?,
                    str_field(&body, "model")?.ok_or(Error::Response)?,
                )?;
                let choices = body["choices"].as_array().ok_or(Error::Response)?;
                if choices.len() != 1 || choices[0]["index"].as_u64() != Some(0) {
                    return Err(Error::Unsupported);
                }
                let m = &choices[0]["message"];
                if m["role"] != "assistant" {
                    return Err(Error::Response);
                }
                if ["audio", "function_call"]
                    .iter()
                    .any(|key| m.get(key).is_some_and(|v| !v.is_null()))
                {
                    return Err(Error::Unsupported);
                }
                self.text = str_field(m, "content")?.unwrap_or("").into();
                self.refusal = str_field(m, "refusal")?.unwrap_or("").into();
                self.reasoning = str_field(m, "reasoning_content")?.unwrap_or("").into();
                if let Some(calls) = m.get("tool_calls").filter(|v| !v.is_null()) {
                    for (i, c) in calls.as_array().ok_or(Error::Response)?.iter().enumerate() {
                        if i >= 32 || c["type"] != "function" {
                            return Err(Error::Unsupported);
                        }
                        let mut block = Block::new("tool_use");
                        block.id = str_field(c, "id")?.ok_or(Error::Response)?.into();
                        block.name = str_field(&c["function"], "name")?
                            .ok_or(Error::Response)?
                            .into();
                        block.args = str_field(&c["function"], "arguments")?
                            .ok_or(Error::Response)?
                            .into();
                        self.calls.insert(i as u32, block);
                    }
                }
                self.reason = Some(
                    str_field(&choices[0], "finish_reason")?
                        .ok_or(Error::Response)?
                        .into(),
                );
                if let Some(u) = body.get("usage") {
                    self.chat_usage(u)?;
                }
                self.terminal = true;
            }
            Protocol::AnthropicMessages => {
                if body["role"] != "assistant" {
                    return Err(Error::Response);
                }
                self.identity(
                    str_field(&body, "id")?.ok_or(Error::Response)?,
                    str_field(&body, "model")?.ok_or(Error::Response)?,
                )?;
                let blocks = body["content"].as_array().ok_or(Error::Response)?;
                if blocks.len() > 128 {
                    return Err(Error::Limit);
                }
                let start=SseMessage{event:Some("message_start".into()),data:json!({"type":"message_start","message":{"id":body["id"],"model":body["model"],"role":"assistant","content":[],"usage":body["usage"]}}).to_string()};
                self.anthropic_event(&start, &mut vec![])?;
                for (i, b) in blocks.iter().enumerate() {
                    self.anthropic_event(
                        &SseMessage {
                            event: None,
                            data: json!({"type":"content_block_start","index":i,"content_block":b})
                                .to_string(),
                        },
                        &mut vec![],
                    )?;
                    self.blocks.get_mut(&(i as u32)).expect("inserted").closed = true;
                }
                self.reason = Some(
                    str_field(&body, "stop_reason")?
                        .ok_or(Error::Response)?
                        .into(),
                );
                self.terminal = true;
            }
        }
        Ok(())
    }
    /// Finish after actual transport EOF. Incomplete finish keeps calls diagnostic-only.
    pub fn finish(&mut self) -> std::result::Result<Result, Error> {
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if self.closed || self.failed {
            return Err(Error::Closed);
        }
        self.closed = true;
        let status = self.status.ok_or(Error::Closed)?;
        if !(200..300).contains(&status) {
            return Err(Error::Http { status });
        }
        if self.scope.stream {
            self.sse.finish_strict().map_err(|_| Error::Incomplete)?;
        } else {
            self.body_response()?;
        }
        if !self.terminal || self.response_id.is_none() || self.reason.is_none() {
            return Err(Error::Incomplete);
        }
        self.result()
    }
    /// Explicit native cancellation cannot be revived by late response bytes.
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.closed = true;
        self.failed = true;
        self.body.clear();
    }
    fn result(&self) -> std::result::Result<Result, Error> {
        if [&self.text, &self.refusal, &self.reasoning]
            .iter()
            .any(|s| s.len() > 262144)
            || self.blocks.values().any(|b| {
                b.text.len() > 262144 || b.signature.len() > 262144 || b.args.len() > 65536
            })
        {
            return Err(Error::Limit);
        }
        let reason = self.reason.clone().ok_or(Error::Incomplete)?;
        let finish = match (self.scope.binding.protocol, reason.as_str()) {
            (Protocol::ChatCompletions, "stop")
            | (Protocol::AnthropicMessages, "end_turn" | "stop_sequence") => Finish::Complete,
            (Protocol::ChatCompletions, "tool_calls")
            | (Protocol::AnthropicMessages, "tool_use") => Finish::ToolCalls,
            _ => Finish::Incomplete,
        };
        let mut parts = vec![];
        let mut replay_blocks = vec![];
        let mut partial = vec![];
        let mut seen = BTreeSet::new();
        let mut replay_calls = vec![];
        if self.scope.binding.protocol == Protocol::ChatCompletions
            && (!self.text.is_empty() || !self.refusal.is_empty())
        {
            parts.push(AgentMessagePart::AgentTextPart(AgentTextPart {
                kind: AgentTextPartKind::Text,
                part_id: "text-0".into(),
                text: format!("{}{}", self.text, self.refusal),
            }));
        }
        let blocks = if self.scope.binding.protocol == Protocol::ChatCompletions {
            &self.calls
        } else {
            &self.blocks
        };
        for (i, b) in blocks {
            match b.kind.as_str() {
                "text" => {
                    parts.push(AgentMessagePart::AgentTextPart(AgentTextPart {
                        kind: AgentTextPartKind::Text,
                        part_id: format!("text-{i}"),
                        text: b.text.clone(),
                    }));
                    replay_blocks.push(json!({"type":"text","text":b.text}));
                }
                "thinking" => {
                    if b.signature.is_empty() {
                        return Err(Error::Response);
                    }
                    replay_blocks
                        .push(json!({"type":"thinking","thinking":b.text,"signature":b.signature}));
                }
                "redacted_thinking" => {
                    replay_blocks.push(json!({"type":"redacted_thinking","data":b.data}))
                }
                "tool_use" => {
                    let args = if !b.args.is_empty() {
                        b.args.clone()
                    } else if self.scope.binding.protocol == Protocol::AnthropicMessages {
                        b.initial.to_string()
                    } else {
                        String::new()
                    };
                    if finish == Finish::Incomplete {
                        partial.push(PartialCall {
                            index: *i,
                            id: b.id.clone(),
                            name: b.name.clone(),
                            arguments: args,
                        });
                        continue;
                    }
                    if finish != Finish::ToolCalls
                        || !seen.insert(&b.id)
                        || !self.scope.tools.contains(&b.name)
                    {
                        return Err(Error::Response);
                    }
                    let parsed = value(args.as_bytes())?;
                    if !parsed.is_object() || args.len() > 65536 || b.id.len() > 256 {
                        return Err(Error::Response);
                    }
                    let name: AgentToolName =
                        serde_json::from_value(json!(b.name)).map_err(|_| Error::Unsupported)?;
                    parts.push(AgentMessagePart::AgentToolProposalPart(
                        AgentToolProposalPart {
                            kind: AgentToolProposalPartKind::ToolCall,
                            part_id: format!("call-{i}"),
                            tool_call_id: b.id.clone(),
                            name,
                            arguments_json: args.clone(),
                        },
                    ));
                    replay_calls.push(json!({"id":b.id,"type":"function","function":{"name":b.name,"arguments":args}}));
                    replay_blocks
                        .push(json!({"type":"tool_use","id":b.id,"name":b.name,"input":parsed}));
                }
                _ => return Err(Error::Unsupported),
            }
        }
        if finish == Finish::ToolCalls && seen.is_empty() {
            return Err(Error::Response);
        }
        if parts.is_empty() {
            parts.push(AgentMessagePart::AgentTextPart(AgentTextPart {
                kind: AgentTextPartKind::Text,
                part_id: "text-0".into(),
                text: String::new(),
            }));
        }
        let message = AgentCanonicalMessage {
            record_type: AgentCanonicalMessageRecordType::CanonicalAgentMessage,
            message_version: 1,
            message_id: self.scope.message_id.clone(),
            role: AgentCanonicalMessageRole::Assistant,
            agent_turn_id: Nullable(Some(self.scope.turn_id.clone())),
            task_id: Nullable(Some(self.scope.task_id.clone())),
            parts,
        };
        om_agent_contract::validate_message(&message).map_err(|_| Error::Response)?;
        if serde_json::to_vec(&message)
            .map_err(|_| Error::Response)?
            .len()
            > MAX_VALUE
        {
            return Err(Error::Limit);
        }
        let replay = if finish == Finish::Incomplete {
            None
        } else {
            let assistant = match self.scope.binding.protocol {
                Protocol::ChatCompletions => {
                    let mut a = json!({"role":"assistant","content":self.text});
                    if !self.refusal.is_empty() {
                        a["refusal"] = json!(self.refusal);
                    }
                    if !self.reasoning.is_empty() {
                        a["reasoning_content"] = json!(self.reasoning);
                    }
                    if !replay_calls.is_empty() {
                        a["tool_calls"] = json!(replay_calls);
                    }
                    a
                }
                Protocol::AnthropicMessages => json!(replay_blocks),
            };
            Some(Replay {
                binding: self.scope.binding.clone(),
                response_id: self.response_id.clone().ok_or(Error::Response)?,
                provider_model: self.provider_model.clone().ok_or(Error::Response)?,
                assistant,
            })
        };
        Ok(Result {
            message,
            finish,
            provider_reason: reason,
            usage: self.usage.clone(),
            partial_calls: partial,
            replay,
        })
    }
}
