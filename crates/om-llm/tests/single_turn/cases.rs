use om_agent_contract::{AgentCanonicalMessage, AgentMessagePart, AgentToolResultPartOutcome};
use om_llm::single_turn::*;
use serde_json::{Value, json};
fn binding(protocol: Protocol) -> Binding {
    Binding {
        provider_ref: "provider-r1".into(),
        route_ref: "route-r1".into(),
        model_ref: "model-r1".into(),
        adapter_version: "single-turn-1".into(),
        protocol,
    }
}
fn scope(protocol: Protocol, stream: bool) -> ResponseScope {
    ResponseScope {
        binding: binding(protocol),
        message_id: "message-native".into(),
        task_id: "task-native".into(),
        turn_id: "turn-native".into(),
        tools: vec!["read_notebook".into(), "search_functions".into()],
        stream,
    }
}
fn message(role: &str, id: &str, parts: Value) -> AgentCanonicalMessage {
    serde_json::from_value(json!({"record_type":"canonical_agent_message","message_version":1,"message_id":id,"role":role,"agent_turn_id":"turn-native","task_id":"task-native","parts":parts})).unwrap()
}
fn text(role: &str, id: &str, t: &str) -> AgentCanonicalMessage {
    message(role, id, json!([{"kind":"text","part_id":"text","text":t}]))
}
fn request(protocol: Protocol) -> Request {
    Request {
        binding: binding(protocol),
        api_model: "configured-alias".into(),
        messages: vec![Message {
            canonical: text("user", "u1", "中文🙂 é"),
            replay: None,
        }],
        results: vec![],
        images: vec![],
        tools: vec![Tool {
            name: "read_notebook".into(),
            description: "读取当前已选笔记本".into(),
            parameters: json!({"type":"object","properties":{},"additionalProperties":false}),
        }],
        max_output_tokens: 128,
        temperature: None,
        stream: true,
    }
}

#[test]
fn prepared_body_and_response_share_declared_tools_binding_and_host_output_identity() {
    let request = request(Protocol::ChatCompletions);
    let (wire, mut decoder) = prepare(&request, "host-message", "host-task", "host-turn").unwrap();
    assert_eq!(wire.binding, request.binding);
    assert!(matches!(
        prepare(&request, "bad id", "host-task", "host-turn"),
        Err(Error::Input)
    ));
    decoder.headers(200).unwrap();
    // search_functions is a valid canonical tool but absent from this exact prepared body.
    decoder.feed(chat().as_bytes()).unwrap();
    assert!(matches!(decoder.finish(), Err(Error::Response)));
    let (_, mut decoder) = prepare(&request, "host-message", "host-task", "host-turn").unwrap();
    decoder.headers(200).unwrap();
    decoder
        .feed((chunk(json!({"content":"实际单轮"}), json!("stop")) + "data: [DONE]\n\n").as_bytes())
        .unwrap();
    let actual = decoder.finish().unwrap();
    assert_eq!(actual.message.message_id, "host-message");
    assert_eq!(actual.message.task_id.0.as_deref(), Some("host-task"));
    assert_eq!(actual.message.agent_turn_id.0.as_deref(), Some("host-turn"));
    assert_eq!(actual.usage, Usage::default());
    assert!(matches!(decoder.feed(b"late"), Err(Error::Closed)));
}

#[test]
fn nonstream_known_unsupported_content_cannot_be_silently_discarded() {
    for field in ["audio", "function_call"] {
        let mut decoder = Decoder::new(scope(Protocol::ChatCompletions, false));
        decoder.headers(200).unwrap();
        let mut message = json!({"role":"assistant","content":"text"});
        message[field] = json!({"opaque":"unsupported"});
        let bytes = serde_json::to_vec(&json!({"id":"original","model":"original-model","choices":[{"index":0,"message":message,"finish_reason":"stop"}]})).unwrap();
        decoder.feed(&bytes).unwrap();
        assert!(matches!(decoder.finish(), Err(Error::Unsupported)));
    }
}
fn chunk(delta: Value, reason: Value) -> String {
    format!(
        "data: {}\n\n",
        json!({"id":"provider-id","model":"served-real-model","choices":[{"index":0,"delta":delta,"finish_reason":reason}]})
    )
}
fn chat() -> String {
    let mut s = chunk(
        json!({"role":"assistant","content":"中文🙂é","unknown_delta":9}),
        Value::Null,
    );
    s += &chunk(
        json!({"tool_calls":[{"index":0,"id":"call_","type":"function","function":{"name":"read_","arguments":"{ \"view\" : "}},{"index":1,"id":"call_b","type":"function","function":{"name":"search_functions","arguments":"{\"query\":\"sin\"}"}}]}),
        Value::Null,
    );
    s += &chunk(
        json!({"tool_calls":[{"index":0,"id":"a","function":{"name":"notebook","arguments":"\"outline\" }"}}]}),
        json!("tool_calls"),
    );
    s += &format!(
        "data: {}\n\n",
        json!({"id":"provider-id","model":"served-real-model","choices":[],"usage":{"prompt_tokens":12,"completion_tokens":8,"total_tokens":20,"prompt_tokens_details":{"cached_tokens":3}},"obfuscation":"ignored"})
    );
    s + "data: [DONE]\n\n"
}
fn anthropic() -> String {
    let events = vec![
        json!({"type":"message_start","message":{"id":"anthropic-id","role":"assistant","model":"served-anthropic","content":[],"usage":{"input_tokens":10,"output_tokens":1,"cache_read_input_tokens":4}}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"not user text"}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"opaque-signature"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"中文🙂 é"}}),
        json!({"type":"content_block_stop","index":1}),
        json!({"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"tool-a","name":"read_notebook","input":{}}}),
        json!({"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{ \"view\" : "}}),
        json!({"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"\"outline\" }"}}),
        json!({"type":"content_block_stop","index":2}),
        json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":15}}),
        json!({"type":"future_non_content_event","value":"ignored"}),
        json!({"type":"message_stop"}),
    ];
    events
        .into_iter()
        .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap()))
        .collect()
}
fn decode(
    protocol: Protocol,
    wire: &[u8],
    cuts: &[usize],
) -> std::result::Result<om_llm::single_turn::Result, Error> {
    let mut decoder = Decoder::new(scope(protocol, true));
    decoder.headers(200)?;
    let mut offset = 0;
    for &end in cuts {
        decoder.feed(&wire[offset..end])?;
        offset = end;
    }
    decoder.feed(&wire[offset..])?;
    decoder.finish()
}
#[test]
fn all_actual_utf8_cut_positions_keep_multiple_calls_raw_args_usage_and_identity() {
    for (protocol, wire) in [
        (Protocol::ChatCompletions, chat()),
        (Protocol::AnthropicMessages, anthropic()),
    ] {
        let expected = decode(protocol, wire.as_bytes(), &[]).unwrap();
        let value = serde_json::to_value(&expected).unwrap();
        for cut in 0..=wire.len() {
            let actual = decode(protocol, wire.as_bytes(), &[cut]).unwrap();
            assert_eq!(serde_json::to_value(actual).unwrap(), value);
        }
        let cuts = (0..=wire.len()).collect::<Vec<_>>();
        assert_eq!(
            serde_json::to_value(decode(protocol, wire.as_bytes(), &cuts).unwrap()).unwrap(),
            value
        );
        assert_eq!(expected.finish, Finish::ToolCalls);
        assert_eq!(expected.message.task_id.0.as_deref(), Some("task-native"));
        let calls = expected
            .message
            .parts
            .iter()
            .filter_map(|p| {
                if let AgentMessagePart::AgentToolProposalPart(c) = p {
                    Some(c)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(calls[0].arguments_json, "{ \"view\" : \"outline\" }");
        assert!(expected.replay.is_some());
        assert_eq!(
            expected.usage.input,
            Some(if protocol == Protocol::ChatCompletions {
                12
            } else {
                10
            })
        );
    }
}
#[test]
fn multiline_sse_unknown_fields_and_json_bodies_are_real() {
    let body = json!({"id":"response","model":"served","choices":[{"index":0,"message":{"role":"assistant","content":"完整🙂"},"finish_reason":"stop"}],"unknown":{"field":true}});
    let pretty=serde_json::to_string_pretty(&json!({"id":"response","model":"served","choices":[{"index":0,"delta":{"content":"完整🙂"},"finish_reason":"stop"}],"unknown":{"field":true}})).unwrap();
    let sse = pretty
        .lines()
        .map(|l| format!("data: {l}\r\n"))
        .collect::<String>()
        + "\r\ndata: [DONE]\r\n\r\n";
    let output = decode(Protocol::ChatCompletions, sse.as_bytes(), &[]).unwrap();
    assert_eq!(output.finish, Finish::Complete);
    let mut native = Decoder::new(scope(Protocol::ChatCompletions, false));
    native.headers(200).unwrap();
    native.feed(body.to_string().as_bytes()).unwrap();
    assert_eq!(native.finish().unwrap().message.parts.len(), 1);
}
#[test]
fn incomplete_finish_never_presents_partial_calls_as_eligible_proposals() {
    let wire = chunk(
        json!({"tool_calls":[{"index":0,"id":"call-a","type":"function","function":{"name":"read_notebook","arguments":"{\"view\":"}}]}),
        json!("length"),
    ) + "data: [DONE]\n\n";
    let output = decode(Protocol::ChatCompletions, wire.as_bytes(), &[]).unwrap();
    assert_eq!(output.finish, Finish::Incomplete);
    assert!(output.replay.is_none());
    assert_eq!(output.partial_calls[0].arguments, "{\"view\":");
    assert!(
        output
            .message
            .parts
            .iter()
            .all(|p| !matches!(p, AgentMessagePart::AgentToolProposalPart(_)))
    );
    let missing = chat().replace("data: [DONE]\n\n", "");
    assert_eq!(
        decode(Protocol::ChatCompletions, missing.as_bytes(), &[]).unwrap_err(),
        Error::Incomplete
    );
    let tail = chat() + "data: {\"unfinished\":";
    assert_eq!(
        decode(Protocol::ChatCompletions, tail.as_bytes(), &[]).unwrap_err(),
        Error::Incomplete
    );
}
#[test]
fn codec_refuses_malformed_json_duplicate_fields_http_errors_and_late_cancel_bytes() {
    let mut decoder = Decoder::new(scope(Protocol::ChatCompletions, true));
    decoder.headers(401).unwrap();
    decoder.feed(b"{\"message\":\"sk-do-not-echo\"}").unwrap();
    let error = decoder.finish().unwrap_err();
    assert_eq!(error, Error::Http { status: 401 });
    assert!(!format!("{error:?}").contains("sk-"));
    let mut cancelled = Decoder::new(scope(Protocol::ChatCompletions, true));
    cancelled.headers(200).unwrap();
    cancelled.cancel();
    assert_eq!(
        cancelled.feed(chat().as_bytes()).unwrap_err(),
        Error::Cancelled
    );
    let duplicate = "data: {\"id\":\"x\",\"id\":\"y\",\"model\":\"m\",\"choices\":[]}\n\n";
    assert_eq!(
        decode(Protocol::ChatCompletions, duplicate.as_bytes(), &[]).unwrap_err(),
        Error::Response
    );
    let malformed = chunk(
        json!({"tool_calls":[{"index":0,"id":"bad-call","type":"function","function":{"name":"read_notebook","arguments":"{\"view\":}"}}]}),
        json!("tool_calls"),
    ) + "data: [DONE]\n\n";
    assert_eq!(
        decode(Protocol::ChatCompletions, malformed.as_bytes(), &[]).unwrap_err(),
        Error::Response
    );
    let mut bad = Decoder::new(scope(Protocol::ChatCompletions, true));
    bad.headers(200).unwrap();
    assert!(bad.feed(&[0xff, 10, 10]).is_err());
    assert_eq!(bad.feed(b"anything").unwrap_err(), Error::Closed);
}
#[test]
fn builder_pairs_results_preserves_order_and_refuses_wrong_or_missing_results() {
    for protocol in [Protocol::ChatCompletions, Protocol::AnthropicMessages] {
        let mut req = request(protocol);
        let out = decode(
            protocol,
            if protocol == Protocol::ChatCompletions {
                chat()
            } else {
                anthropic()
            }
            .as_bytes(),
            &[],
        )
        .unwrap();
        req.messages.push(Message {
            canonical: out.message.clone(),
            replay: out.replay.clone(),
        });
        assert!(matches!(build(&req), Err(Error::Input)));
        let calls = out
            .message
            .parts
            .iter()
            .filter_map(|p| {
                if let AgentMessagePart::AgentToolProposalPart(c) = p {
                    Some(c)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for (i, c) in calls.iter().enumerate() {
            let id = format!("result-{i}");
            req.messages.push(Message{canonical:message("tool",&id,json!([{"kind":"tool_result","part_id":"result","tool_call_id":c.tool_call_id,"operation_ref":null,"result_ref":"native-result","outcome":"unavailable"}])),replay:None});
            req.results.push(ToolResult {
                message_id: id,
                tool_call_id: c.tool_call_id.clone(),
                text: "工具不可用，无执行".into(),
                is_error: true,
            });
        }
        let wire = build(&req).unwrap();
        let body: Value = serde_json::from_slice(&wire.body).unwrap();
        assert_eq!(body["model"], "configured-alias");
        assert_eq!(body["stream"], true);
        assert!(!body.to_string().contains("api_key"));
        req.results[0].is_error = false;
        assert!(matches!(build(&req), Err(Error::Input)));
        req.results[0].is_error = true;
        req.messages[1].replay.as_mut().unwrap().binding.model_ref = "changed-model-r2".into();
        assert!(matches!(build(&req), Err(Error::Replay)));
    }
}
#[test]
fn unsupported_media_opaque_server_tools_and_bad_part_identity_are_not_silently_omitted() {
    let mut req = request(Protocol::ChatCompletions);
    req.messages[0].canonical = message(
        "user",
        "u1",
        json!([{"kind":"media","part_id":"image","media_ref":"prepared-image-ref","submission_kind":"image"}]),
    );
    assert!(matches!(build(&req), Err(Error::Unsupported)));
    req.messages[0].canonical = text("user", "u1", "valid");
    req.messages[0].canonical.message_version = 2;
    assert!(matches!(build(&req), Err(Error::Input)));
    let server = anthropic().replace("\"type\":\"tool_use\"", "\"type\":\"server_tool_use\"");
    assert!(matches!(
        decode(Protocol::AnthropicMessages, server.as_bytes(), &[]),
        Err(Error::Unsupported)
    ));
    let _unused = AgentToolResultPartOutcome::Unavailable;
}

#[test]
fn native_images_preserve_part_order_and_require_exact_resolved_identity() {
    for protocol in [Protocol::ChatCompletions, Protocol::AnthropicMessages] {
        let mut req = request(protocol);
        req.messages[0].canonical = message(
            "user",
            "u1",
            json!([
                {"kind":"text","part_id":"before","text":"看图："},
                {"kind":"media","part_id":"picture","media_ref":"attachment-1","submission_kind":"image"},
                {"kind":"text","part_id":"after","text":"描述颜色"}
            ]),
        );
        // A complete native-prepared one-pixel PNG; no network or file reader in the codec.
        req.images.push(Image {message_id:"u1".into(), part_id:"picture".into(), media_ref:"attachment-1".into(),
            media_type:"image/png".into(), data:"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/lF8AAAAASUVORK5CYII=".into()});
        let body: Value = serde_json::from_slice(&build(&req).unwrap().body).unwrap();
        let content = &body["messages"][0]["content"];
        assert_eq!(content[0]["text"], "看图：");
        assert_eq!(content[2]["text"], "描述颜色");
        if protocol == Protocol::ChatCompletions {
            assert!(
                content[1]["image_url"]["url"]
                    .as_str()
                    .unwrap()
                    .starts_with("data:image/png;base64,")
            );
        } else {
            assert_eq!(content[1]["source"]["data"], req.images[0].data);
        }
        req.images[0].media_ref = "different-attachment".into();
        assert!(matches!(build(&req), Err(Error::Input)));
        req.images[0].media_ref = "attachment-1".into();
        req.images[0].data = "https://example.invalid/private.png".into();
        assert!(matches!(build(&req), Err(Error::Input)));
        req.images[0].data = "iVBORw0KGgo=".into();
        req.images.push(req.images[0].clone());
        assert!(matches!(build(&req), Err(Error::Input)));
        req.images.pop();
        req.images[0].part_id = "unreferenced".into();
        assert!(build(&req).is_err());
    }
}

#[test]
fn signed_thinking_is_opaque_replay_not_user_text_and_scope_switch_is_explicit() {
    let result = decode(Protocol::AnthropicMessages, anthropic().as_bytes(), &[]).unwrap();
    let replay = result.replay.as_ref().unwrap();
    assert_eq!(replay.assistant[0]["signature"], "opaque-signature");
    assert!(
        !serde_json::to_string(&result.message)
            .unwrap()
            .contains("not user text")
    );
    let mut req = request(Protocol::AnthropicMessages);
    req.messages.push(Message {
        canonical: result.message.clone(),
        replay: result.replay.clone(),
    });
    req.messages.push(Message{canonical:message("tool","reply",json!([{"kind":"tool_result","part_id":"result","tool_call_id":"tool-a","operation_ref":null,"result_ref":"result-ref","outcome":"unavailable"}])),replay:None});
    req.results.push(ToolResult {
        message_id: "reply".into(),
        tool_call_id: "tool-a".into(),
        text: "unavailable".into(),
        is_error: true,
    });
    let body: Value = serde_json::from_slice(&build(&req).unwrap().body).unwrap();
    assert_eq!(
        body["messages"][1]["content"][0]["signature"],
        "opaque-signature"
    );
    req.binding.route_ref = "new-route-r2".into();
    assert!(matches!(build(&req), Err(Error::Replay)));
    let truncated = anthropic().replace(
        "\"stop_reason\":\"tool_use\"",
        "\"stop_reason\":\"max_tokens\"",
    );
    let partial = decode(Protocol::AnthropicMessages, truncated.as_bytes(), &[]).unwrap();
    assert_eq!(partial.finish, Finish::Incomplete);
    assert_eq!(partial.partial_calls.len(), 1);
    assert!(partial.replay.is_none());
}
#[test]
fn refuses_unknown_finish_empty_tools_duplicate_call_ids_and_known_unsupported_modalities() {
    let unknown = chat().replace(
        "\"finish_reason\":\"tool_calls\"",
        "\"finish_reason\":\"future_reason\"",
    );
    assert_eq!(
        decode(Protocol::ChatCompletions, unknown.as_bytes(), &[])
            .unwrap()
            .finish,
        Finish::Incomplete
    );
    let false_tools = chunk(json!({"content":"text"}), json!("tool_calls")) + "data: [DONE]\n\n";
    assert_eq!(
        decode(Protocol::ChatCompletions, false_tools.as_bytes(), &[]).unwrap_err(),
        Error::Response
    );
    let duplicate = chunk(
        json!({"tool_calls":[{"index":0,"id":"same","function":{"name":"read_notebook","arguments":"{}"}},{"index":1,"id":"same","function":{"name":"read_notebook","arguments":"{}"}}]}),
        json!("tool_calls"),
    ) + "data: [DONE]\n\n";
    assert_eq!(
        decode(Protocol::ChatCompletions, duplicate.as_bytes(), &[]).unwrap_err(),
        Error::Response
    );
    let audio = chunk(json!({"audio":{"data":"opaque"}}), json!("stop")) + "data: [DONE]\n\n";
    assert_eq!(
        decode(Protocol::ChatCompletions, audio.as_bytes(), &[]).unwrap_err(),
        Error::Unsupported
    );
}
