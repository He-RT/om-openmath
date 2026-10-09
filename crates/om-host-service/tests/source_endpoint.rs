//! Trusted source C-ABI endpoint uses the actual restored parser settings, not default math mode.
use om_host_service::{
    document::{
        SourceDocument,
        endpoint::{SourceEndpoint, decode_source_command},
    },
    protocol::{Serial, generated::*},
};
use serde_json::json;
use sha2::{Digest, Sha256};

fn endpoint(strict: bool) -> SourceEndpoint {
    let owner = SourceDocument::new(
        "doc".into(),
        Serial::new(1).unwrap(),
        NativeSourceFile {
            version: 1,
            title: String::new(),
            cells: vec![
                NativeSourceCell {
                    id: "cell".into(),
                    kind: NativeCellKind::Math,
                    source: "let e=2".into(),
                    dialect: Dialect::Modern,
                },
                NativeSourceCell {
                    id: "dependent".into(),
                    kind: NativeCellKind::Math,
                    source: "e+1".into(),
                    dialect: Dialect::Modern,
                },
            ],
        },
    )
    .unwrap();
    let mut config = om_kernel::config::KernelConfig::default().general;
    if strict {
        config.constants = om_kernel::config::Constants::Strict;
    }
    SourceEndpoint::open(
        owner.snapshot().clone(),
        "store".into(),
        "runtime".into(),
        Serial::new(1).unwrap(),
        Some(om_host_service::document::coordinator::calculation_values(&config).unwrap()),
        Serial::new(7).unwrap(),
    )
    .unwrap()
}
fn command(endpoint: &mut SourceEndpoint, value: serde_json::Value) -> NativeSourceHostReply {
    endpoint
        .command(decode_source_command(&serde_json::to_vec(&value).unwrap()).unwrap())
        .unwrap()
}
#[test]
fn restored_constants_are_used_in_the_actual_source_preview() {
    for strict in [false, true] {
        let mut owner = endpoint(strict);
        let read = command(
            &mut owner,
            json!({"type":"source_snapshot_ref","complete_cell_ids":["cell","dependent"]}),
        );
        let preview = command(
            &mut owner,
            json!({"type":"source_preview","arguments":{
                "snapshot_ref":read.reference.0.unwrap(),"input":{"kind":"patch","operations":[{
                    "type":"update_cell","target":{"cell_id":"cell"},"expected_source_hash":format!("{:x}",Sha256::digest("let e=2")),"source":"let e=5"
                }]}
            }}),
        );
        let data: NativePreviewData = serde_json::from_value(preview.preview.0.unwrap()).unwrap();
        assert!(data.valid);
        assert_eq!(
            data.affected_cell_ids.contains(&"dependent".to_owned()),
            strict
        );
    }
}
#[test]
fn host_command_schema_is_closed_and_requires_the_restored_configuration_binding() {
    assert!(decode_source_command(br#"{"type":"source_read","unexpected":true}"#).is_err());
    assert!(decode_source_command(br#"{"type":"source_open","store_id":"store"}"#).is_err());
}
