//! Source authority stages without CAS, uses exact UTF-8 and only accepts matching IO facts.
use om_host_service::{
    document::{SourceDocument, snapshot_hash, validate_commit},
    protocol::{Nullable, Serial, generated::*},
};
fn file(source: &str) -> NativeSourceFile {
    NativeSourceFile {
        version: 1,
        title: "笔记🙂".into(),
        cells: vec![NativeSourceCell {
            id: "one".into(),
            kind: NativeCellKind::Math,
            source: source.into(),
            dialect: Dialect::Modern,
        }],
    }
}
fn plan(owner: &SourceDocument, source: &str) -> NativeSourceCommit {
    owner
        .prepare(
            file(source),
            "op".into(),
            "tx".into(),
            "event".into(),
            "2026-10-09T00:00:00Z".into(),
        )
        .unwrap()
}
fn receipt(p: &NativeSourceCommit) -> NativeDurableSourceReceipt {
    NativeDurableSourceReceipt {
        protocol_version: 1,
        receipt: OperationReceipt {
            record_type: OperationReceiptRecordType::OperationReceipt,
            store_id: "store".into(),
            document_id: Nullable(Some(p.commit.document_id.clone())),
            operation_id: p.commit.operation_id.clone(),
            operation_kind: OperationReceiptOperationKind::SourceEdit,
            request_hash: p.commit.request_hash.clone(),
            phase: OperationReceiptPhase::Completed,
            transaction_id: Nullable(Some(p.commit.transaction_id.clone())),
            committed_revision: Nullable(Some(p.after.revision)),
            accepted_result_ids: vec![],
            details_blob_hash: Nullable(None),
            details_retention: OperationReceiptDetailsRetention::Full,
            error_code: Nullable(None),
            updated_at: p.commit.committed_at.clone(),
        },
        snapshot_hash: p.after.snapshot_hash.clone(),
        inverse_plan_hash: p.before.snapshot_hash.clone(),
        execution_epoch: p.after.execution_epoch,
        outbox_event_id: p.commit.outbox_event_id.clone(),
    }
}
#[test]
fn staged_invalid_math_is_source_and_unknown_receipt_never_changes_authority() {
    let mut doc =
        SourceDocument::new("doc".into(), Serial::new(1).unwrap(), file("let a=2")).unwrap();
    let p = plan(&doc, "let f(x)=;");
    assert_eq!(doc.snapshot().file.cells[0].source, "let a=2");
    assert_eq!(p.after.file.cells[0].source, "let f(x)=;");
    let mut r = receipt(&p);
    r.receipt.phase = OperationReceiptPhase::Unknown;
    assert!(doc.accept(&p, &r).is_err());
    assert_eq!(doc.snapshot().revision.get(), 0);
    r.receipt.phase = OperationReceiptPhase::Completed;
    doc.accept(&p, &r).unwrap();
    assert_eq!(doc.snapshot().revision.get(), 1);
    assert!(doc.accept(&p, &r).is_err());
}

#[test]
fn input_group_is_bound_to_the_original_manual_request_and_not_an_agent_grant() {
    let doc = SourceDocument::new("doc".into(), Serial::new(1).unwrap(), file("2+2")).unwrap();
    let mut original = plan(&doc, "3+3");
    let plain = original.commit.request_hash.clone();
    original.input_group_id = Some("native-input-group".into());
    original.commit.request_hash = om_host_service::document::request_hash(&original);
    assert_ne!(plain, original.commit.request_hash);
    validate_commit(&original).unwrap();
    let mut modified = original.clone();
    modified.input_group_id = Some("other-group".into());
    assert!(validate_commit(&modified).is_err());
    modified.commit.actor = DocumentCommitActor::Agent;
    modified.commit.task_id = Nullable(Some("task".into()));
    modified.commit.request_hash = om_host_service::document::request_hash(&modified);
    assert!(validate_commit(&modified).is_err());
}
#[test]
fn raw_unicode_order_and_per_cell_revision_are_never_normalized() {
    let doc = SourceDocument::new("doc".into(), Serial::new(1).unwrap(), file("e\u{301}")).unwrap();
    let p = plan(&doc, "é");
    assert_ne!(p.before.snapshot_hash, p.after.snapshot_hash);
    assert_eq!(p.before.file.cells[0].source.as_bytes(), b"e\xcc\x81");
    assert_eq!(p.after.cell_revisions[0].revision.get(), 1);
    let unchanged = plan(&doc, "e\u{301}");
    assert_eq!(unchanged.after.cell_revisions[0].revision.get(), 0);
    let mut malformed = p.clone();
    malformed.after.file.cells[0].source = "forged".into();
    assert!(validate_commit(&malformed).is_err());
    let mut wrong_revision = p;
    wrong_revision.after.cell_revisions[0].revision = Serial::new(999).unwrap();
    wrong_revision.after.snapshot_hash = snapshot_hash(&wrong_revision.after);
    assert!(validate_commit(&wrong_revision).is_err());
}
#[test]
fn duplicate_cells_overflow_and_reopen_generation_cannot_commit_half_a_document() {
    let doc = SourceDocument::new("doc".into(), Serial::new(1).unwrap(), file("2+2")).unwrap();
    let mut duplicate = file("a");
    duplicate.cells.push(duplicate.cells[0].clone());
    assert!(
        doc.prepare(
            duplicate,
            "op".into(),
            "tx".into(),
            "event".into(),
            "time".into()
        )
        .is_err()
    );
    let p = plan(&doc, "3+3");
    let mut restored =
        SourceDocument::restore(doc.snapshot().clone(), Serial::new(2).unwrap()).unwrap();
    assert!(restored.accept(&p, &receipt(&p)).is_err());
    let mut snapshot = doc.snapshot().clone();
    snapshot.revision = Serial::new(9007199254740991).unwrap();
    snapshot.snapshot_hash = snapshot_hash(&snapshot);
    let max = SourceDocument::restore(snapshot, Serial::new(1).unwrap()).unwrap();
    assert!(
        max.prepare(
            file("7"),
            "op".into(),
            "tx".into(),
            "event".into(),
            "time".into()
        )
        .is_err()
    );
}

#[test]
fn legacy_unicode_cell_ids_are_byte_distinct_and_roundtrip_without_reassignment() {
    let mut f = file("2+2");
    f.cells[0].id = "é".into();
    let mut c = f.cells[0].clone();
    c.id = "e\u{301}".into();
    f.cells.push(c);
    let doc = SourceDocument::new("doc".into(), Serial::new(1).unwrap(), f.clone()).unwrap();
    let mut replacement = f;
    replacement.cells.reverse();
    let p = doc
        .prepare(
            replacement,
            "op".into(),
            "tx".into(),
            "event".into(),
            "2026-10-09T00:00:00Z".into(),
        )
        .unwrap();
    assert_eq!(p.commit.changed_cell_ids, vec!["e\u{301}", "é"]);
    validate_commit(&p).unwrap();
}

#[test]
fn execution_epoch_remains_independent_from_source_revision() {
    let doc = SourceDocument::new("doc".into(), Serial::new(1).unwrap(), file("2+2")).unwrap();
    let mut source = doc.snapshot().clone();
    source.execution_epoch = Serial::new(42).unwrap();
    source.snapshot_hash = snapshot_hash(&source);
    let restored = SourceDocument::restore(source, Serial::new(1).unwrap()).unwrap();
    let p = plan(&restored, "3+3");
    assert_eq!(p.after.revision.get(), 1);
    assert_eq!(p.after.execution_epoch.get(), 43);
}
