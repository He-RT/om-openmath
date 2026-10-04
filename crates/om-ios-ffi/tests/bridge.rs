//! Real shared Session results and independent cancellation, without native IO.
use om_ios_ffi::Host;
use serde_json::{Value, json};
use std::sync::Arc;

fn request(host: &Host, body: Value) -> Value {
    serde_json::from_str(
        &host
            .request(&json!({"id":1,"body":body}).to_string())
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn bridge_solves_reacts_preserves_source_and_rejects_bad_envelopes() {
    let host = Host::new(None).unwrap();
    let solved = request(
        &host,
        json!({"type":"evaluate","cell_id":"solve","source":"solve(x^2==4,x)","dialect":"Modern"}),
    );
    assert_eq!(
        solved["response"]["body"]["output"]["items"][0]["view"]["solutions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    request(
        &host,
        json!({"type":"evaluate","cell_id":"a","source":"let a=2","dialect":"Modern"}),
    );
    request(
        &host,
        json!({"type":"evaluate","cell_id":"b","source":"a+1","dialect":"Modern"}),
    );
    let changed = request(
        &host,
        json!({"type":"evaluate","cell_id":"a","source":"let a=5","dialect":"Modern"}),
    );
    assert!(changed.to_string().contains("6"));
    let saved = request(&host, json!({"type":"save_notebook"}));
    assert!(saved["response"]["body"]["file"].get("config").is_none());
    assert!(host.request("bad-json").is_err());
    assert!(
        host.request(r#"{"id":0,"body":{"type":"run_all"}}"#)
            .is_err()
    );
    host.close();
    assert!(
        host.request(r#"{"id":1,"body":{"type":"run_all"}}"#)
            .is_err()
    );
}

#[test]
fn independent_interrupt_recovers_without_resetting_definitions() {
    let host = Arc::new(Host::new(None).unwrap());
    request(
        &host,
        json!({"type":"evaluate","cell_id":"a","source":"let a=9","dialect":"Modern"}),
    );
    let worker = host.clone();
    let running = std::thread::spawn(move || {
        request(
            &worker,
            json!({"type":"evaluate","cell_id":"slow","source":"factorial(1000000)","dialect":"Modern"}),
        )
    });
    std::thread::sleep(std::time::Duration::from_millis(40));
    host.interrupt();
    assert!(running.join().unwrap().to_string().contains("Aborted"));
    let next = request(
        &host,
        json!({"type":"evaluate","cell_id":"next","source":"a+1","dialect":"Modern"}),
    );
    assert_eq!(
        next["response"]["body"]["output"]["items"][0]["input_form"],
        "10"
    );
}
