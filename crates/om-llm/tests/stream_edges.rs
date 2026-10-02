//! Bounded Unicode/framing and strict malformed-field regression cases.
use om_llm::*;
#[test]
fn every_utf8_crlf_split_preserves_exact_messages_and_only_one_bom_is_removed() {
    let input = "\u{feff}event: delta\r\ndata: 求解🧮α\r\n\r\ndata: tail\r\r";
    let mut whole = SseDecoder::new();
    let expected = whole.feed(input);
    for split in 0..=input.len() {
        let mut d = SseDecoder::new();
        let mut actual = d.feed_bytes(&input.as_bytes()[..split]).unwrap();
        actual.extend(d.feed_bytes(&input.as_bytes()[split..]).unwrap());
        actual.extend(d.finish().unwrap());
        assert_eq!(actual, expected, "split {split}");
    }
    let mut d = SseDecoder::new();
    assert_eq!(
        d.feed("data: a\n\nevent: reset\n\ndata: b\n\n")[1].event,
        None
    );
    let mut d = SseDecoder::new();
    assert!(d.feed("\u{feff}\u{feff}data: ignored\n\n").is_empty());
}
#[test]
fn ndjson_rejects_malformed_records_and_terminal_decoders_do_not_resume() {
    let mut d = NdjsonDecoder::new();
    assert!(d.try_feed("{invalid}\n").is_err());
    assert!(d.try_feed("{}\n").is_err());
    let mut d = NdjsonDecoder::new();
    d.try_feed("{").unwrap();
    assert!(d.finish().is_err());
    let mut d = SseDecoder::new();
    assert_eq!(d.feed_bytes(b"data: \xe4").unwrap(), vec![]);
    assert_eq!(d.feed_bytes(b"\xbd\xa0\n\n").unwrap()[0].data, "你");
    d.finish().unwrap();
    assert!(d.finish().unwrap().is_empty());
    assert!(d.try_feed("data: later\n\n").is_err());
    assert!(d.finish().is_err());
}
#[test]
fn known_malformed_provider_containers_and_required_tool_start_fields_emit_errors() {
    for data in [
        r#"{"choices":[false]}"#,
        r#"{"choices":[{"index":0,"delta":{"content":123}}]}"#,
        r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"function":{"arguments":"{}"}}]}}]}"#,
    ] {
        assert!(
            matches!(&decode_openai_chunk(data)[0], StreamEvent::Error(_)),
            "{data}"
        );
    }
    for data in [
        r#"{"type":"content_block_start","index":0,"content_block":false}"#,
        r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","input":{}}}"#,
    ] {
        assert!(
            matches!(
                &decode_anthropic_event("content_block_start", data)[0],
                StreamEvent::Error(_)
            ),
            "{data}"
        );
    }
    assert!(decode_anthropic_event("future_event", "not json").is_empty());
    assert!(decode_openai_chunk(r#"{"choices":[],"usage":{"output_tokens":1}}"#).is_empty());
}
