//! Genuine frozen preview -> Swift physical persistence -> original Rust authority acceptance.
use om_host_service::{
    document::{SourceDocument, coordinator::SourceCoordinator, preview::PreviewService},
    protocol::{Serial, generated::*},
    references::ReferenceScope,
};
use sha2::{Digest, Sha256};
fn main() {
    let mut owner = SourceDocument::new(
        "00000000-0000-0000-0000-000000000051".into(),
        Serial::new(1).unwrap(),
        NativeSourceFile {
            version: 1,
            title: "preview notebook".into(),
            cells: vec![NativeSourceCell {
                id: "原格🙂".into(),
                kind: NativeCellKind::Math,
                source: "let a=2".into(),
                dialect: Dialect::Modern,
            }],
        },
    )
    .unwrap();
    let mut service = PreviewService::new([19; 32]); // own deterministic fixture; real runtime uses host entropy
    let scope = ReferenceScope {
        runtime: "preview-fixture".into(),
        document: owner.snapshot().document_id.clone(),
        generation: 1,
        revision: 0,
        execution_epoch: 0,
        snapshot_hash: owner.snapshot().snapshot_hash.clone(),
        task: Some("task-fixture".into()),
        task_generation: 1,
        grant_revision: 1,
        config_revision: 0,
        definition_revision: 0,
        metadata_revision: u64::from(om_kernel::capabilities::function_catalog().metadata_version),
        editor_state_hash: String::new(),
        execution_mode: true,
        can_read: true,
        can_preview: true,
        can_write: true,
    };
    let snapshot = service
        .snapshot(
            &owner,
            SourceCoordinator::default(),
            owner
                .snapshot()
                .file
                .cells
                .iter()
                .map(|c| c.id.clone())
                .collect(),
            &scope,
            0,
        )
        .unwrap();
    let original_hash = format!("{:x}", Sha256::digest("let a=2".as_bytes()));
    let arguments=serde_json::to_vec(&serde_json::json!({"snapshot_ref":snapshot,"input":{"kind":"patch","operations":[{"type":"update_cell","target":{"cell_id":"原格🙂"},"expected_source_hash":original_hash,"source":"let a=5"},{"type":"insert_cell","client_key":"new-result","after":{"cell_id":"原格🙂"},"kind":"Math","dialect":"Modern","source":"a+1"}]}})).unwrap();
    let checked = service
        .inspect(&arguments, &owner, &scope, 1, "2026-10-09T00:00:00Z".into())
        .unwrap();
    assert!(checked.valid);
    let frozen = service
        .resolve_for_commit(checked.preview_ref.0.as_ref().unwrap(), &owner, &scope, 2)
        .unwrap();
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() == 3 && args[1] == "verify" {
        let receipt = serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
        owner.accept(&frozen.commit, &receipt).unwrap();
        assert_eq!(owner.snapshot().file.cells[0].source, "let a=5");
        assert_eq!(
            owner.snapshot().file.cells[1].id,
            checked.assigned_cell_ids["new-result"]
        );
        assert_eq!(owner.snapshot().file.cells[1].source, "a+1");
        println!(
            "Original Rust authority accepted actual frozen-preview SQLite receipt and host-assigned cell ID"
        );
        return;
    }
    println!(
        "{}",
        serde_json::json!({"initial":owner.snapshot(),"plan":frozen.commit,"preview":checked,"prepared_only":true})
    );
}
