//! Exercise real exported symbols and all owned byte paths, never a mock host.
use crate::*;
use serde_json::{Value, json};
use std::{
    ptr, thread,
    time::{Duration, Instant},
};
fn json_buffer(buffer: OmHostBuffer) -> Value {
    assert!(!buffer.ptr.is_null());
    assert!(buffer.len > 0);
    // SAFETY: this is an outstanding Rust-owned output and exactly its valid allocation length.
    let bytes = unsafe { std::slice::from_raw_parts(buffer.ptr, buffer.len) };
    let value = serde_json::from_slice(bytes).unwrap();
    om_host_buffer_free(buffer);
    value
}
fn create() -> *mut OmHostHandle {
    let bytes=serde_json::to_vec(&json!({"protocol_version":1,"runtime_instance_id":"ffi-test","max_pending_operations":8,"event_capacity":32})).unwrap();
    // SAFETY: the Vec remains readable through the call.
    let result = unsafe { om_host_create(bytes.as_ptr(), bytes.len()) };
    assert!(!result.handle.is_null());
    assert!(result.error.ptr.is_null());
    assert_eq!(result.error.len, 0);
    result.handle
}
fn submit(host: *mut OmHostHandle, id: &str, body: Value) -> Value {
    let mut bytes=serde_json::to_vec(&json!({"protocol_version":1,"runtime_instance_id":"ffi-test","request_ref":id,"operation_id":id,"document_binding":null,"task_binding":null,"body":body})).unwrap();
    // SAFETY: the input allocation is readable during the call, mutated only after return.
    let result = unsafe { om_host_submit(host, bytes.as_ptr(), bytes.len()) };
    bytes.fill(0);
    drop(bytes);
    json_buffer(result)
}
fn math(source: &str) -> Value {
    json!({"kind":"evaluate_scratch","source":source,"dialect":"Modern","definition_snapshot_ref":null,"use_notebook_definitions":false,"timeout_ms":10000})
}
fn finished(host: *mut OmHostHandle, id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let batch = json_buffer(om_host_next_events(host, 100, 512 * 1024));
        for event in batch["events"].as_array().unwrap() {
            if event["operation_ref"] == id && event["event_kind"] == "operation_finished" {
                return event["payload"].clone();
            }
        }
        assert!(
            Instant::now() < deadline,
            "unsettled exported operation: {id}"
        );
    }
}
fn close(host: *mut OmHostHandle) {
    assert_eq!(json_buffer(om_host_close_begin(host))["closing"], true);
    assert_eq!(om_host_close_finish(host), 0);
}
#[test]
fn create_rejects_null_invalid_utf8_unknown_fields_and_overlimit_with_owned_errors() {
    let _serial = crate::registry::TEST_LOCK.lock().unwrap();
    let before = crate::registry::counts();
    // SAFETY: invalid null/length and overlimit inputs are rejected before any dereference.
    for (pointer, len, expected) in [
        (ptr::null(), 1, "INVALID_ARGUMENT"),
        (ptr::null(), 0, "INVALID_ARGUMENT"),
        (ptr::null(), usize::MAX, "BUDGET_EXCEEDED"),
    ] {
        let result = unsafe { om_host_create(pointer, len) };
        assert!(result.handle.is_null());
        assert_eq!(json_buffer(result.error)["error"]["code"], expected);
    }
    for bytes in [vec![0xff],b"{}".to_vec(),b"{\"protocol_version\":1,\"runtime_instance_id\":\"ffi-test\",\"max_pending_operations\":8,\"event_capacity\":32,\"unsafe\":true}".to_vec()] {
        // SAFETY: allocated bytes remain valid through this call.
        let result=unsafe { om_host_create(bytes.as_ptr(),bytes.len()) };
        assert!(result.handle.is_null());
        assert_eq!(json_buffer(result.error)["error"]["code"],"INVALID_ARGUMENT");
    }
    assert_eq!(crate::registry::counts(), before);
}
#[test]
fn real_math_borrowed_input_is_copied_and_all_returned_bytes_are_freed() {
    let _serial = crate::registry::TEST_LOCK.lock().unwrap();
    let before = crate::registry::counts();
    let host = create();
    let receipt = submit(host, "sum", math("2+2"));
    assert_eq!(receipt["accepted"], true);
    let result = finished(host, "sum");
    assert_eq!(result["phase"], "completed");
    assert_eq!(
        result["result"]["response"]["output"]["items"][0]["input_form"],
        "4"
    );
    assert_eq!(result["result"]["run_outcome"], "completed");
    submit(host, "caps", json!({"kind":"get_capabilities"}));
    assert_eq!(
        finished(host, "caps")["result"]["kernel"]["platform"],
        "desktop"
    );
    submit(host, "broken", math("let f(x)=;"));
    let broken = finished(host, "broken");
    assert_eq!(broken["result"]["run_outcome"], "failed");
    assert_eq!(broken["result"]["effect_committed"], false);
    close(host);
    assert_eq!(crate::registry::counts(), before);
}
#[test]
fn free_drops_only_owned_matching_allocation_and_empty_is_harmless() {
    let _serial = crate::registry::TEST_LOCK.lock().unwrap();
    let before = crate::registry::counts();
    let buffer = om_host_next_events(ptr::null_mut(), 0, 512 * 1024);
    om_host_buffer_free(OmHostBuffer {
        ptr: buffer.ptr,
        len: buffer.len + 1,
    });
    assert_eq!(crate::registry::counts().1, before.1 + 1);
    json_buffer(buffer);
    // Immediate duplicate free is ignored; callers still must free exactly once.
    om_host_buffer_free(buffer);
    om_host_buffer_free(OmHostBuffer {
        ptr: ptr::null_mut(),
        len: 0,
    });
    om_host_buffer_free(OmHostBuffer {
        ptr: ptr::without_provenance_mut(1),
        len: 55,
    });
    assert_eq!(crate::registry::counts(), before);
}
#[test]
fn consumed_handles_never_alias_and_cancel_is_repeatable_without_rearming() {
    let _serial = crate::registry::TEST_LOCK.lock().unwrap();
    let before = crate::registry::counts();
    let host = create();
    submit(host, "long", math("map(fn(k)=>sin(k),range(1,100000))"));
    // SAFETY: the literal is valid for exactly four bytes.
    assert_eq!(unsafe { om_host_cancel(host, b"long".as_ptr(), 4) }, 1);
    assert_eq!(finished(host, "long")["phase"], "cancelled");
    assert_eq!(unsafe { om_host_cancel(host, b"long".as_ptr(), 4) }, 0);
    assert_eq!(unsafe { om_host_cancel(host, ptr::null(), 1) }, -1);
    assert_eq!(unsafe { om_host_cancel(host, b"no-such".as_ptr(), 7) }, -2);
    close(host);
    assert_eq!(om_host_close_finish(host), -2);
    assert_eq!(
        submit(host, "stale", math("2+2"))["error"]["code"],
        "INVALID_REFERENCE"
    );
    let new = create();
    assert_ne!(new, host);
    assert_eq!(unsafe { om_host_cancel(host, b"long".as_ptr(), 4) }, -2);
    close(new);
    assert_eq!(crate::registry::counts(), before);
}
#[test]
fn concurrent_event_poll_submit_cancel_and_close_settle_without_stale_pointer_access() {
    let _serial = crate::registry::TEST_LOCK.lock().unwrap();
    let before = crate::registry::counts();
    let host = create();
    submit(host, "long", math("map(fn(k)=>sin(k),range(1,100000))"));
    let address = host as usize;
    let start = std::sync::Arc::new(std::sync::Barrier::new(5));
    let mut threads = Vec::new();
    for lane in 0..4 {
        let start = start.clone();
        threads.push(thread::spawn(move || {
            start.wait();
            let host = address as *mut OmHostHandle;
            for i in 0..12 {
                match lane {
                    0 => {
                        let packet = json_buffer(om_host_next_events(host, 2, 512 * 1024));
                        assert!(packet["events"].is_array() || packet["error"].is_object());
                    }
                    1 => {
                        let packet =
                            submit(host, &format!("state-{i}"), json!({"kind":"get_state"}));
                        assert!(packet["accepted"] == true || packet["error"].is_object());
                    }
                    2 => {
                        let status = unsafe { om_host_cancel(host, b"long".as_ptr(), 4) };
                        assert!([-2, 0, 1].contains(&status));
                    }
                    _ => {
                        let packet = json_buffer(om_host_close_begin(host));
                        assert!(packet["closing"] == true || packet["error"].is_object());
                    }
                }
            }
        }));
    }
    start.wait();
    assert_eq!(om_host_close_finish(host), 0);
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(crate::registry::counts(), before);
}
#[test]
fn malformed_submit_and_event_byte_limits_report_actual_failure_or_resync() {
    let _serial = crate::registry::TEST_LOCK.lock().unwrap();
    let before = crate::registry::counts();
    let host = create();
    let bad = unsafe { om_host_submit(host, std::ptr::null(), 1) };
    assert_eq!(json_buffer(bad)["error"]["code"], "INVALID_ARGUMENT");
    let huge = unsafe { om_host_submit(host, std::ptr::null(), usize::MAX) };
    assert_eq!(json_buffer(huge)["error"]["code"], "BUDGET_EXCEEDED");
    let invalid_poll = json_buffer(om_host_next_events(host, 101, 512 * 1024));
    assert_eq!(invalid_poll["error"]["code"], "INVALID_ARGUMENT");
    submit(host, "completed", math("2+2"));
    finished(host, "completed");
    assert_eq!(
        submit(host, "completed", math("100+1"))["error"]["code"],
        "INVALID_REFERENCE"
    );
    assert_eq!(json_buffer(om_host_close_begin(host))["closing"], true);
    assert_eq!(
        submit(host, "closing", math("2+2"))["error"]["code"],
        "NOT_AVAILABLE"
    );
    assert_eq!(om_host_close_finish(host), 0);
    assert_eq!(crate::registry::counts(), before);
}
