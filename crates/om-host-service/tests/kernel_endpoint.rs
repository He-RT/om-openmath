//! Actual safe command decoding and the shared native source/math owner, no physical fake success.
use om_host_service::{kernel::endpoint::decode_command, protocol::generated::*};
#[test]
fn state_command_is_the_same_closed_shape_as_generated_swift() {
    let state = decode_command(br#"{"type":"kernel_state"}"#).unwrap();
    assert!(matches!(state, NativeKernelHostCommand::KernelState(_)));
}
