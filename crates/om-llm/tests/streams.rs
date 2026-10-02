//! Actual fixed-seed byte chunking across framing, UTF-8 and tool JSON fragments.
use om_llm::*;
use om_num::rng::SplitMix64;
use std::collections::BTreeMap;
fn events(frames: Vec<SseMessage>, anthropic: bool) -> Vec<StreamEvent> {
    frames
        .into_iter()
        .flat_map(|f| {
            if anthropic {
                decode_anthropic_event(f.event.as_deref().unwrap_or("message"), &f.data)
            } else {
                decode_openai_chunk(&f.data)
            }
        })
        .collect()
}
fn stream(bytes: &[u8], anthropic: bool, seed: Option<u64>) -> Vec<StreamEvent> {
    let mut decoder = SseDecoder::new();
    let mut frames = vec![];
    if let Some(seed) = seed {
        let mut rng = SplitMix64::new(seed);
        let mut i = 0;
        while i < bytes.len() {
            let end = (i + rng.next_range(1, 8) as usize).min(bytes.len());
            frames.extend(decoder.feed_bytes(&bytes[i..end]).unwrap());
            i = end;
        }
    } else {
        frames.extend(decoder.feed_bytes(bytes).unwrap());
    }
    frames.extend(decoder.finish().unwrap());
    events(frames, anthropic)
}
#[test]
fn mandated_multibyte_text_and_two_tool_fixtures_are_identical_for_seeded_byte_splits() {
    for (fixture, anthropic) in [
        (include_bytes!("fixtures/openai_text.sse").as_slice(), false),
        (
            include_bytes!("fixtures/openai_tools.sse").as_slice(),
            false,
        ),
        (
            include_bytes!("fixtures/anthropic_text.sse").as_slice(),
            true,
        ),
        (
            include_bytes!("fixtures/anthropic_tools.sse").as_slice(),
            true,
        ),
    ] {
        let whole = stream(fixture, anthropic, None);
        assert!(!whole.is_empty());
        for seed in 0..64 {
            assert_eq!(stream(fixture, anthropic, Some(seed)), whole, "seed {seed}");
        }
    }
    for (fixture, anthropic) in [
        (include_bytes!("fixtures/openai_text.sse").as_slice(), false),
        (
            include_bytes!("fixtures/anthropic_text.sse").as_slice(),
            true,
        ),
    ] {
        let text = stream(fixture, anthropic, None)
            .into_iter()
            .filter_map(|e| {
                if let StreamEvent::Text(t) = e {
                    Some(t)
                } else {
                    None
                }
            })
            .collect::<String>();
        assert_eq!(text, "你好 α=2");
    }
}
#[test]
fn actual_interleaved_fragments_preserve_ids_names_indices_and_final_json() {
    for (fixture, anthropic) in [
        (
            include_bytes!("fixtures/openai_tools.sse").as_slice(),
            false,
        ),
        (
            include_bytes!("fixtures/anthropic_tools.sse").as_slice(),
            true,
        ),
    ] {
        let mut calls: BTreeMap<u32, (String, String, String)> = BTreeMap::new();
        for event in stream(fixture, anthropic, Some(42)) {
            if let StreamEvent::ToolCallDelta {
                index,
                id,
                name,
                args_fragment,
            } = event
            {
                let c = calls.entry(index).or_default();
                if let Some(id) = id {
                    c.0 = id;
                }
                if let Some(name) = name {
                    c.1 = name;
                }
                c.2.push_str(&args_fragment);
            }
        }
        let expected = if anthropic {
            vec![(1, "tool_1", "evaluate"), (2, "tool_2", "solve")]
        } else {
            vec![(0, "call_1", "evaluate"), (1, "call_2", "solve")]
        };
        for (index, id, name) in expected {
            let c = &calls[&index];
            assert_eq!(c.0, id);
            assert_eq!(c.1, name);
        }
        assert_eq!(calls.len(), 2);
        for (_, c) in calls {
            assert!(!c.0.is_empty());
            let v: serde_json::Value = serde_json::from_str(&c.2).unwrap();
            if c.1 == "evaluate" {
                assert_eq!(v["code"], "α+1");
            } else {
                assert_eq!(c.1, "solve");
                assert_eq!(v["variables"][0], "x");
            }
        }
    }
}
#[test]
fn sse_bom_crlf_bare_cr_comments_multiline_and_empty_data_follow_actual_framing() {
    let input = "\u{feff}:keepalive\r\nevent: delta\rdata: first\r\ndata: second\r\n\r\nevent: discarded\n\ndata\n\nDATA: ignored\n\ndata:  leading\n\n";
    let mut d = SseDecoder::new();
    let frames = d.feed(input);
    assert_eq!(
        frames,
        vec![
            SseMessage {
                event: Some("delta".into()),
                data: "first\nsecond".into()
            },
            SseMessage {
                event: None,
                data: "".into()
            },
            SseMessage {
                event: None,
                data: " leading".into()
            }
        ]
    );
    assert!(d.finish().unwrap().is_empty());
    let mut d = SseDecoder::new();
    assert!(d.feed("data: incomplete").is_empty());
    assert!(d.finish().unwrap().is_empty());
    let mut d = SseDecoder::new();
    assert!(d.feed("data: incomplete\n").is_empty());
    assert!(d.finish().unwrap().is_empty());
}
#[test]
fn malformed_utf8_incomplete_scalars_and_oversized_lines_events_fail_explicitly() {
    let mut d = SseDecoder::new();
    assert!(d.feed_bytes(b"data: \xff\n\n").is_err());
    assert!(d.feed_bytes(b"data: x\n\n").is_err());
    let mut d = SseDecoder::new();
    d.feed_bytes(b"data: \xe4\xbd").unwrap();
    assert!(d.finish().is_err());
    let mut d = SseDecoder::with_limit(16);
    assert!(d.try_feed("data: 01234567890123456789").is_err());
    let mut d = SseDecoder::with_limit(24);
    assert!(
        d.try_feed("data: 123456789\ndata: 123456789\ndata: 123456789\n\n")
            .is_err()
    );
}
#[test]
fn provider_errors_metadata_choice_zero_and_anthropic_initial_inputs_are_truthful() {
    assert!(
        matches!(&decode_openai_chunk(include_str!("fixtures/openai_error_401.json"))[0],StreamEvent::Error(e) if e.contains("Incorrect API key"))
    );
    let es = decode_openai_chunk(
        r#"{"choices":[{"index":1,"delta":{"content":"wrong"}},{"index":0,"delta":{"content":"right"},"finish_reason":"stop"}]}"#,
    );
    assert_eq!(es[0], StreamEvent::Text("right".into()));
    assert!(matches!(
        decode_openai_chunk("{bad")[0],
        StreamEvent::Error(_)
    ));
    assert!(decode_anthropic_event("ping", r#"{"type":"ping"}"#).is_empty());
    let es = decode_anthropic_event(
        "content_block_start",
        r#"{"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"t","name":"solve","input":{"x":2}}}"#,
    );
    assert!(
        matches!(&es[0],StreamEvent::ToolCallDelta{index:2,args_fragment,..} if args_fragment=="{\"x\":2}")
    );
    assert!(matches!(
        decode_anthropic_event("error", r#"{"error":{"message":"overloaded"}}"#)[0],
        StreamEvent::Error(_)
    ));
}
#[test]
fn ndjson_incremental_unicode_crlf_and_final_record_are_actual_json_records() {
    let input = "{\"response\":\"你好\",\"done\":false}\r\n{\"response\":\" α\",\"done\":true}";
    let mut d = NdjsonDecoder::new();
    let mut frames = vec![];
    for b in input.as_bytes().chunks(1) {
        frames.extend(d.feed_bytes(b).unwrap());
    }
    frames.extend(d.finish().unwrap());
    assert_eq!(frames.len(), 2);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&frames[0]).unwrap()["response"],
        "你好"
    );
    let mut d = NdjsonDecoder::with_limit(8);
    assert!(d.try_feed("123456789").is_err());
}

#[test]
fn real_finish_markers_keep_specific_reasons_before_terminal_markers() {
    for (fixture, anthropic, reason) in [
        (
            include_bytes!("fixtures/openai_text.sse").as_slice(),
            false,
            "stop",
        ),
        (
            include_bytes!("fixtures/openai_tools.sse").as_slice(),
            false,
            "tool_calls",
        ),
        (
            include_bytes!("fixtures/anthropic_text.sse").as_slice(),
            true,
            "end_turn",
        ),
        (
            include_bytes!("fixtures/anthropic_tools.sse").as_slice(),
            true,
            "tool_use",
        ),
    ] {
        let reasons = stream(fixture, anthropic, Some(7))
            .into_iter()
            .filter_map(|e| {
                if let StreamEvent::Finish { reason } = e {
                    Some(reason)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(reasons, [reason, "done"]);
    }
}
