//! Real budget and streaming decoders exercised with sealed self-authored fixture inputs.
mod support;
use om_llm::{SseDecoder, StreamEvent, decode_openai_chunk};
use om_num::ctx::{Abort, Interrupt};
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use support::ControlledClock;
#[test]
fn controlled_clock_and_cancel_drive_actual_budget_checks() {
    let clock = Arc::new(ControlledClock::new());
    let flag = Arc::new(AtomicBool::new(false));
    let budget = Interrupt {
        flag: flag.clone(),
        deadline_ms: Some(10.0),
        clock: Some(clock.clone()),
        steps_left: Cell::new(4000),
    };
    budget.tick().unwrap();
    clock.advance(11);
    assert_eq!(budget.tick(), Err(Abort::Timeout));
    flag.store(true, Ordering::Release);
    assert_eq!(budget.tick(), Err(Abort::Interrupted));
}
#[test]
fn own_unicode_fixture_passes_the_real_sans_io_provider_decoder_for_every_split() {
    let stream =
        include_bytes!("../../../macos/OpenMathNativeTests/Fixtures/Network/utf8-stream.sse");
    for width in [1, 2, 3, 5, 13, 64] {
        let mut decoder = SseDecoder::new();
        let mut events = vec![];
        for part in stream.chunks(width) {
            for message in decoder.feed_bytes(part).unwrap() {
                events.extend(decode_openai_chunk(&message.data));
            }
        }
        assert_eq!(
            events
                .iter()
                .filter_map(|e| if let StreamEvent::Text(t) = e {
                    Some(t.as_str())
                } else {
                    None
                })
                .collect::<String>(),
            "中文🙂 π"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StreamEvent::Finish { .. }))
        );
    }
}
