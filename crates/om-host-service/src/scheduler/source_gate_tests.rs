//! The final native fence call must reject a busy parser owner instead of blocking UI input.
use super::*;
#[test]
fn barrier_does_not_wait_on_the_source_owner_lock() {
    let host = NativeHost::new(wire::decode_init(br#"{"protocol_version":1,"runtime_instance_id":"gate-test","max_pending_operations":32,"event_capacity":32}"#).unwrap()).unwrap();
    let held = host.shared.source.lock().unwrap();
    let reply = host.source_command(
        br#"{"type":"source_barrier","operation_id":"operation","fence_id":"fence"}"#,
    );
    assert_eq!(reply.unwrap_err(), "EDITING_BUSY");
    drop(held);
    host.close_finish().unwrap();
}
