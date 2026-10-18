//! Only compile one trusted snapshot into a protocol body. No credential/URL or tool loop.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use om_agent_contract::{
    AgentCanonicalMessageRole as Role, AgentMediaPartSubmissionKind, AgentMessagePart as Part,
    AgentToolResultPartOutcome,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
const MAX_BODY: usize = 2 * 1024 * 1024;
pub(super) fn identity(s: &str) -> bool {
    s.len() <= 256
        && !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        && s.as_bytes()[0].is_ascii_alphanumeric()
}
fn name(part: &om_agent_contract::AgentToolProposalPart) -> String {
    serde_json::to_value(&part.name)
        .expect("enum")
        .as_str()
        .expect("string")
        .into()
}
fn portable(
    message: &AgentCanonicalMessage,
    results: &BTreeMap<(&str, &str), &ToolResult>,
) -> std::result::Result<(String, Vec<Value>), Error> {
    let mut text = String::new();
    let mut calls = vec![];
    for part in &message.parts {
        match part {
            Part::AgentTextPart(p) => text.push_str(&p.text),
            Part::AgentToolProposalPart(p) => {
                let arguments = om_agent_contract::json::strict_json(p.arguments_json.as_bytes())
                    .map_err(|_| Error::Input)?;
                if !arguments.is_object() {
                    return Err(Error::Input);
                }
                calls.push(json!({"id":p.tool_call_id,"type":"function","function":{"name":name(p),"arguments":p.arguments_json}}));
            }
            Part::AgentToolResultPart(p) => {
                let resolved = results
                    .get(&(message.message_id.as_str(), p.tool_call_id.as_str()))
                    .ok_or(Error::Input)?;
                text.push_str(&resolved.text);
            }
            Part::AgentMediaPart(_) => {}
            _ => return Err(Error::Unsupported),
        }
    }
    Ok((text, calls))
}
/// Compile a single text/tool request, preserving canonical order and exact original arguments.
pub fn build(request: &Request) -> std::result::Result<Wire, Error> {
    if !identity(&request.binding.provider_ref)
        || !identity(&request.binding.route_ref)
        || !identity(&request.binding.model_ref)
        || !identity(&request.binding.adapter_version)
        || request.api_model.is_empty()
        || request.api_model.len() > 256
        || request.api_model.chars().any(char::is_control)
        || request.messages.is_empty()
        || request.messages.len() > 1024
        || request.tools.len() > 32
        || request.max_output_tokens == 0
        || request
            .temperature
            .is_some_and(|t| !t.is_finite() || !(0.0..=2.0).contains(&t))
    {
        return Err(Error::Input);
    }
    let mut declared = BTreeSet::new();
    for t in &request.tools {
        if !identity(&t.name)
            || !declared.insert(&t.name)
            || t.description.len() > 8192
            || !t.parameters.is_object()
            || serde_json::to_vec(&t.parameters)
                .map_err(|_| Error::Input)?
                .len()
                > 65536
        {
            return Err(Error::Input);
        }
    }
    let mut resolved = BTreeMap::new();
    for r in &request.results {
        if r.text.len() > 262144
            || resolved
                .insert((r.message_id.as_str(), r.tool_call_id.as_str()), r)
                .is_some()
        {
            return Err(Error::Input);
        }
    }
    let mut messages = vec![];
    let mut images = BTreeMap::new();
    if request.images.len() > 16 {
        return Err(Error::Limit);
    }
    for image in &request.images {
        if !identity(&image.media_ref) || image.data.len() > 1024 * 1024 {
            return Err(Error::Input);
        }
        let bytes = STANDARD.decode(&image.data).map_err(|_| Error::Input)?;
        let valid = match image.media_type.as_str() {
            "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
            "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
            _ => false,
        };
        if !valid
            || images
                .insert((image.message_id.as_str(), image.part_id.as_str()), image)
                .is_some()
        {
            return Err(Error::Input);
        }
    }
    let mut images_used = BTreeSet::new();
    let mut system = vec![];
    let mut pending = BTreeSet::new();
    let mut calls_seen = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut parts_seen = BTreeSet::new();
    let mut result_used = BTreeSet::new();
    let mut conversation = false;
    for record in &request.messages {
        let m = &record.canonical;
        om_agent_contract::validate_message(m).map_err(|_| Error::Input)?;
        if !ids.insert(&m.message_id)
            || m.parts.iter().any(|p| {
                let value = serde_json::to_value(p).expect("generated part");
                !parts_seen.insert((
                    m.message_id.clone(),
                    value["part_id"].as_str().unwrap_or("").to_owned(),
                ))
            })
        {
            return Err(Error::Input);
        }
        if m.role != Role::Tool && !pending.is_empty() {
            return Err(Error::Input);
        }
        if m.role == Role::System && conversation {
            return Err(Error::Input);
        }
        if m.role != Role::System {
            conversation = true;
        }
        for p in &m.parts {
            match p {
                Part::AgentToolProposalPart(c) => {
                    if m.role != Role::Assistant || !calls_seen.insert(c.tool_call_id.clone()) {
                        return Err(Error::Input);
                    }
                    pending.insert(c.tool_call_id.clone());
                }
                Part::AgentToolResultPart(r) => {
                    if m.role != Role::Tool || !pending.remove(&r.tool_call_id) {
                        return Err(Error::Input);
                    }
                    let data = resolved
                        .get(&(m.message_id.as_str(), r.tool_call_id.as_str()))
                        .ok_or(Error::Input)?;
                    if data.is_error == (r.outcome == AgentToolResultPartOutcome::Completed) {
                        return Err(Error::Input);
                    }
                    result_used.insert((m.message_id.as_str(), r.tool_call_id.as_str()));
                }
                Part::AgentTextPart(_) => {
                    if m.role == Role::Tool {
                        return Err(Error::Input);
                    }
                }
                Part::AgentMediaPart(p) => {
                    if m.role != Role::User
                        || p.submission_kind != AgentMediaPartSubmissionKind::Image
                    {
                        return Err(Error::Unsupported);
                    }
                    let key = (m.message_id.as_str(), p.part_id.as_str());
                    let image = images.get(&key).ok_or(Error::Unsupported)?;
                    if image.media_ref != p.media_ref {
                        return Err(Error::Input);
                    }
                    images_used.insert(key);
                }
                _ => return Err(Error::Unsupported),
            }
        }
        let (text, calls) = portable(m, &resolved)?;
        if let Some(replay) = &record.replay {
            if m.role != Role::Assistant
                || replay.binding != request.binding
                || replay.response_id.is_empty()
            {
                return Err(Error::Replay);
            }
            match request.binding.protocol {
                Protocol::ChatCompletions => {
                    let body = &replay.assistant;
                    if body["role"] != "assistant"
                        || format!(
                            "{}{}",
                            body["content"].as_str().unwrap_or(""),
                            body["refusal"].as_str().unwrap_or("")
                        ) != text
                        || body.get("tool_calls").cloned().unwrap_or(json!([])) != json!(calls)
                    {
                        return Err(Error::Replay);
                    }
                    if !body.is_object()
                        || body.as_object().expect("checked").keys().any(|k| {
                            !matches!(
                                k.as_str(),
                                "role" | "content" | "tool_calls" | "refusal" | "reasoning_content"
                            )
                        })
                    {
                        return Err(Error::Replay);
                    }
                    messages.push(body.clone());
                }
                Protocol::AnthropicMessages => {
                    let blocks = replay.assistant.as_array().ok_or(Error::Replay)?;
                    let mut replay_text = String::new();
                    let mut replay_calls = vec![];
                    for b in blocks {
                        match b["type"].as_str() {
                            Some("text") => {
                                replay_text.push_str(b["text"].as_str().ok_or(Error::Replay)?)
                            }
                            Some("tool_use") => {
                                replay_calls.push((
                                    b["id"].as_str().ok_or(Error::Replay)?,
                                    b["name"].as_str().ok_or(Error::Replay)?,
                                    b["input"].clone(),
                                ));
                            }
                            Some("thinking") => {
                                if !b["signature"].is_string() || !b["thinking"].is_string() {
                                    return Err(Error::Replay);
                                }
                            }
                            Some("redacted_thinking") => {
                                if !b["data"].is_string() {
                                    return Err(Error::Replay);
                                }
                            }
                            _ => return Err(Error::Replay),
                        }
                    }
                    if replay_text != text || replay_calls.len() != calls.len() {
                        return Err(Error::Replay);
                    }
                    for ((id, n, args), call) in replay_calls.iter().zip(&calls) {
                        let original = om_agent_contract::json::strict_json(
                            call["function"]["arguments"]
                                .as_str()
                                .ok_or(Error::Replay)?
                                .as_bytes(),
                        )
                        .map_err(|_| Error::Replay)?;
                        if Some(*id) != call["id"].as_str()
                            || Some(*n) != call["function"]["name"].as_str()
                            || &original != args
                        {
                            return Err(Error::Replay);
                        }
                    }
                    messages.push(json!({"role":"assistant","content":blocks}));
                }
            }
            continue;
        }
        match request.binding.protocol {
            Protocol::ChatCompletions => match m.role {
                Role::System => messages.push(json!({"role":"system","content":text})),
                Role::User => {
                    if m.parts.iter().any(|p| matches!(p, Part::AgentMediaPart(_))) {
                        let content = m.parts.iter().map(|p| match p {
                            Part::AgentTextPart(p) => Ok(json!({"type":"text","text":p.text})),
                            Part::AgentMediaPart(p) => {
                                let image = images[&(m.message_id.as_str(), p.part_id.as_str())];
                                Ok(json!({"type":"image_url","image_url":{"url":format!("data:{};base64,{}", image.media_type, image.data)}}))
                            }
                            _ => Err(Error::Unsupported),
                        }).collect::<std::result::Result<Vec<_>, Error>>()?;
                        messages.push(json!({"role":"user","content":content}));
                    } else {
                        messages.push(json!({"role":"user","content":text}));
                    }
                }
                Role::Assistant => {
                    let mut v = json!({"role":"assistant","content":text});
                    if !calls.is_empty() {
                        v["tool_calls"] = json!(calls);
                    }
                    messages.push(v);
                }
                Role::Tool => {
                    for part in &m.parts {
                        if let Part::AgentToolResultPart(r) = part {
                            messages.push(json!({"role":"tool","tool_call_id":r.tool_call_id,"content":resolved[&(m.message_id.as_str(),r.tool_call_id.as_str())].text}));
                        }
                    }
                }
            },
            Protocol::AnthropicMessages => {
                if m.role == Role::System {
                    system.push(json!({"type":"text","text":text}));
                    continue;
                }
                let role = if m.role == Role::Assistant {
                    "assistant"
                } else {
                    "user"
                };
                let mut content = vec![];
                for part in &m.parts {
                    match part {
     Part::AgentTextPart(p)=>content.push(json!({"type":"text","text":p.text})),
     Part::AgentToolProposalPart(c)=>content.push(json!({"type":"tool_use","id":c.tool_call_id,"name":name(c),"input":om_agent_contract::json::strict_json(c.arguments_json.as_bytes()).map_err(|_|Error::Input)?})),
     Part::AgentToolResultPart(r)=>{let result=resolved[&(m.message_id.as_str(),r.tool_call_id.as_str())];content.push(json!({"type":"tool_result","tool_use_id":r.tool_call_id,"content":result.text,"is_error":result.is_error}));},
     Part::AgentMediaPart(p)=>{let image=images[&(m.message_id.as_str(),p.part_id.as_str())];content.push(json!({"type":"image","source":{"type":"base64","media_type":image.media_type,"data":image.data}}));},
     _=>return Err(Error::Unsupported),
    }
                }
                if let Some(last) = messages.last_mut().filter(|v| v["role"] == role) {
                    last["content"]
                        .as_array_mut()
                        .ok_or(Error::Input)?
                        .extend(content);
                } else {
                    messages.push(json!({"role":role,"content":content}));
                }
            }
        }
    }
    if !conversation
        || !pending.is_empty()
        || result_used.len() != resolved.len()
        || images_used.len() != images.len()
    {
        return Err(Error::Input);
    }
    let mut body = json!({"model":request.api_model,"messages":messages,"stream":request.stream});
    match request.binding.protocol {
        Protocol::ChatCompletions => {
            body["max_completion_tokens"] = json!(request.max_output_tokens);
            if request.stream {
                body["stream_options"] = json!({"include_usage":true});
            }
            if !request.tools.is_empty() {
                body["tools"]=json!(request.tools.iter().map(|t|json!({"type":"function","function":{"name":t.name,"description":t.description,"parameters":t.parameters}})).collect::<Vec<_>>());
                body["tool_choice"] = json!("auto");
            }
        }
        Protocol::AnthropicMessages => {
            body["max_tokens"] = json!(request.max_output_tokens);
            if !system.is_empty() {
                body["system"] = json!(system);
            }
            if !request.tools.is_empty() {
                body["tools"]=json!(request.tools.iter().map(|t|json!({"name":t.name,"description":t.description,"input_schema":t.parameters})).collect::<Vec<_>>());
            }
        }
    }
    if let Some(t) = request.temperature {
        body["temperature"] = json!(t);
    }
    let body = serde_json::to_vec(&body).map_err(|_| Error::Input)?;
    if body.len() > MAX_BODY {
        return Err(Error::Limit);
    }
    Ok(Wire {
        binding: request.binding.clone(),
        body,
        stream: request.stream,
    })
}
