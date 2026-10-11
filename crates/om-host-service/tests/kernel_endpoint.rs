//! Actual safe command decoding and the shared native source/math owner, no physical fake success.
use om_host_service::{
    document::{SourceDocument, endpoint::SourceEndpoint},
    protocol::Serial,
};
use om_host_service::{kernel::endpoint::decode_command, protocol::generated::*};
use sha2::{Digest, Sha256};
#[test]
fn state_command_is_the_same_closed_shape_as_generated_swift() {
    let state = decode_command(br#"{"type":"kernel_state"}"#).unwrap();
    assert!(matches!(state, NativeKernelHostCommand::KernelState(_)));
}
fn owner() -> (SourceEndpoint, String) {
    let snapshot = SourceDocument::new(
        "restore-doc".into(),
        Serial::new(1).unwrap(),
        NativeSourceFile {
            version: 1,
            title: "恢复".into(),
            cells: vec![NativeSourceCell {
                id: "a".into(),
                kind: NativeCellKind::Math,
                source: "let a=2".into(),
                dialect: Dialect::Modern,
            }],
        },
    )
    .unwrap()
    .snapshot()
    .clone();
    let hash = snapshot.snapshot_hash.clone();
    (
        SourceEndpoint::open(
            snapshot,
            "restore-store".into(),
            "restore-runtime".into(),
            Serial::new(1).unwrap(),
            None,
            Serial::new(0).unwrap(),
        )
        .unwrap(),
        hash,
    )
}
fn start(
    endpoint: &mut SourceEndpoint,
    source: &str,
    operation: &str,
    bytes: &[u8],
) -> NativeKernelHostReply {
    endpoint
        .kernel_command(NativeKernelHostCommand::KernelRestoreStart(
            KernelRestoreStart {
                r#type: KernelRestoreStartType::KernelRestoreStart,
                operation_id: operation.into(),
                expected_source_hash: source.into(),
                byte_length: Serial::new(bytes.len() as u64).unwrap(),
                payload_hash: format!("{:x}", Sha256::digest(bytes)),
            },
        ))
        .unwrap()
}
fn write(
    endpoint: &mut SourceEndpoint,
    id: &str,
    offset: u64,
    bytes: Vec<u32>,
) -> Result<NativeKernelHostReply, String> {
    endpoint.kernel_command(NativeKernelHostCommand::KernelRestoreWrite(
        KernelRestoreWrite {
            r#type: KernelRestoreWriteType::KernelRestoreWrite,
            operation_id: id.into(),
            offset: Serial::new(offset).unwrap(),
            bytes,
        },
    ))
}
#[test]
fn recovery_upload_is_bounded_idempotent_and_directly_cancelled_without_a_state() {
    let (mut endpoint, source) = owner();
    let bytes = b"OMKR\x01\0\0\0\0";
    let reply = start(&mut endpoint, &source, "restore-upload", bytes);
    assert!(reply.pending_document_write);
    assert_eq!(reply.blob_offset.0.unwrap().get(), 0);
    let first = write(&mut endpoint, "restore-upload", 0, vec![79, 77, 75]).unwrap();
    assert_eq!(first.blob_offset.0.unwrap().get(), 3);
    let duplicate = write(&mut endpoint, "restore-upload", 0, vec![79, 77, 75]).unwrap();
    assert_eq!(duplicate.blob_offset.0.unwrap().get(), 3);
    assert_eq!(
        write(&mut endpoint, "restore-upload", 0, vec![0]).unwrap_err(),
        "DUPLICATE_OPERATION"
    );
    assert_eq!(
        write(&mut endpoint, "restore-upload", 4, vec![1]).unwrap_err(),
        "INVALID_ARGUMENT"
    );
    assert_eq!(
        write(&mut endpoint, "restore-upload", 3, vec![256]).unwrap_err(),
        "INVALID_ARGUMENT"
    );
    assert_eq!(endpoint.cancel("restore-upload"), Some(true));
    let state = endpoint
        .kernel_command(NativeKernelHostCommand::KernelStatus(KernelStatus {
            r#type: KernelStatusType::KernelStatus,
            operation_id: "restore-upload".into(),
        }))
        .unwrap();
    assert_eq!(state.phase, NativeKernelHostReplyPhase::Discarded);
    assert!(!state.pending_document_write);
    assert!(state.active_checkpoint_id.0.is_none());
    assert_eq!(state.kernel_state_revision.get(), 0);
}
#[test]
fn malformed_complete_payload_fails_in_background_and_releases_the_gate() {
    let (mut endpoint, source) = owner();
    let bytes = b"OMKR\x01\0\0\0\0";
    start(&mut endpoint, &source, "invalid-codec", bytes);
    write(
        &mut endpoint,
        "invalid-codec",
        0,
        bytes.iter().copied().map(u32::from).collect(),
    )
    .unwrap();
    endpoint
        .kernel_command(NativeKernelHostCommand::KernelRestoreFinish(
            KernelRestoreFinish {
                r#type: KernelRestoreFinishType::KernelRestoreFinish,
                operation_id: "invalid-codec".into(),
            },
        ))
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let reply = endpoint
            .kernel_command(NativeKernelHostCommand::KernelStatus(KernelStatus {
                r#type: KernelStatusType::KernelStatus,
                operation_id: "invalid-codec".into(),
            }))
            .unwrap();
        if reply.phase == NativeKernelHostReplyPhase::Failed {
            assert!(reply.active_checkpoint_id.0.is_none());
            assert!(!reply.pending_document_write);
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
}
#[test]
fn closed_wire_does_not_allow_oversized_or_untyped_recovery_chunks() {
    assert!(
        decode_command(
            br#"{"type":"kernel_restore_write","operation_id":"x","offset":0,"bytes":[256]}"#
        )
        .is_err()
    );
    assert!(decode_command(br#"{"type":"kernel_restore_start","operation_id":"x","byte_length":9,"payload_hash":"bogus"}"#).is_err());
}
