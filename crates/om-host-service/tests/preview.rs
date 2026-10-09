//! Real source/patch preview with exact immutable scope, source exposure and opaque leases.
use om_host_service::{
    document::{
        SourceDocument,
        coordinator::SourceCoordinator,
        preview::{PreviewError, PreviewService},
    },
    protocol::{Serial, generated::*},
    references::{ReferenceError, ReferenceScope},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
fn owner(kind: NativeCellKind, source: &str) -> SourceDocument {
    SourceDocument::new(
        "doc".into(),
        Serial::new(1).unwrap(),
        NativeSourceFile {
            version: 1,
            title: "original".into(),
            cells: vec![NativeSourceCell {
                id: "原格🙂".into(),
                kind,
                source: source.into(),
                dialect: Dialect::Modern,
            }],
        },
    )
    .unwrap()
}
fn scope(doc: &SourceDocument) -> ReferenceScope {
    ReferenceScope {
        runtime: "runtime-one".into(),
        document: "doc".into(),
        generation: 1,
        revision: doc.snapshot().revision.get(),
        execution_epoch: doc.snapshot().execution_epoch.get(),
        snapshot_hash: doc.snapshot().snapshot_hash.clone(),
        task: Some("task-one".into()),
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
    }
}
fn setup(doc: &SourceDocument, full: bool) -> (PreviewService, ReferenceScope, String) {
    let mut service = PreviewService::new([7; 32]);
    let scope = scope(doc);
    let exposed = if full {
        doc.snapshot()
            .file
            .cells
            .iter()
            .map(|c| c.id.clone())
            .collect()
    } else {
        BTreeSet::new()
    };
    let snapshot = service
        .snapshot(doc, SourceCoordinator::default(), exposed, &scope, 0)
        .unwrap();
    (service, scope, snapshot)
}
fn args(snapshot: &str, input: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"snapshot_ref":snapshot,"input":input})).unwrap()
}
fn insert(key: &str, source: &str, after: Value) -> Value {
    json!({"type":"insert_cell","client_key":key,"after":after,"kind":"Math","dialect":"Modern","source":source})
}
fn preview(
    service: &mut PreviewService,
    scope: &ReferenceScope,
    doc: &SourceDocument,
    snapshot: &str,
    operations: Vec<Value>,
) -> Result<NativePreviewData, PreviewError> {
    service.inspect(
        &args(snapshot, json!({"kind":"patch","operations":operations})),
        doc,
        scope,
        1,
        "2026-10-09T00:00:00Z".into(),
    )
}
fn hash(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}
#[test]
fn host_assigns_new_cells_and_frozen_plan_is_retrieved_without_reconstructing_source() {
    let doc = owner(NativeCellKind::Math, "let a=2");
    let (mut service, scope, snapshot) = setup(&doc, true);
    let result=preview(&mut service,&scope,&doc,&snapshot,vec![insert("new-f","let f(x)=x+a",json!({"cell_id":"原格🙂"})),insert("new-use","f (2)",json!({"client_key":"new-f"})),json!({"type":"move_cell","target":{"client_key":"new-use"},"after":{"client_key":"new-f"}})]).unwrap();
    assert!(result.valid);
    assert_eq!(doc.snapshot().revision.get(), 0);
    assert_eq!(result.assigned_cell_ids.len(), 2);
    let id = result.preview_ref.0.unwrap();
    let frozen = service.resolve_for_commit(&id, &doc, &scope, 2).unwrap();
    assert_eq!(frozen.plan_hash, result.plan_hash);
    assert_eq!(
        frozen.commit.after.file.cells[1].id,
        result.assigned_cell_ids["new-f"]
    );
    assert_eq!(frozen.commit.after.file.cells[2].source, "f (2)");
    assert!(
        frozen.invalidation.analysis.cells[2]
            .uses
            .contains(&"f".into())
    );
    let again = service.resolve_for_commit(&id, &doc, &scope, 3).unwrap();
    assert!(std::sync::Arc::ptr_eq(&frozen, &again));
    assert_eq!(
        frozen.commit.commit.operation_id,
        again.commit.commit.operation_id
    );
}
#[test]
fn source_only_preview_never_returns_a_commit_ref_or_executes_a_definition() {
    let doc = owner(NativeCellKind::Math, "a");
    let (mut service, scope, snapshot) = setup(&doc, false);
    let checked = service
        .inspect(
            &args(
                &snapshot,
                json!({"kind":"source","source":"let a=99;a","dialect":"Modern"}),
            ),
            &doc,
            &scope,
            1,
            "time".into(),
        )
        .unwrap();
    assert!(checked.valid);
    assert!(checked.preview_ref.0.is_none());
    assert!(checked.assigned_cell_ids.is_empty());
    assert_eq!(doc.snapshot().file.cells[0].source, "a");
    assert!(matches!(
        service.resolve_for_commit(&snapshot, &doc, &scope, 2),
        Err(PreviewError::Reference(ReferenceError::WrongKind))
    ));
}
#[test]
fn partial_read_rejects_whole_update_but_allows_ordered_unique_raw_fragments() {
    let source = "first🙂 then e\u{301}";
    let doc = owner(NativeCellKind::Text, source);
    let (mut service, scope, snapshot) = setup(&doc, false);
    let update = json!({"type":"update_cell","target":{"cell_id":"原格🙂"},"expected_source_hash":hash(source),"source":"replacement"});
    assert!(matches!(
        preview(&mut service, &scope, &doc, &snapshot, vec![update]),
        Err(PreviewError::IncompleteSource)
    ));
    let replace = |needle: &str, text: &str| json!({"type":"replace_text","target":{"cell_id":"原格🙂"},"expected_source_hash":hash(source),"match":needle,"replacement":text});
    let checked = preview(
        &mut service,
        &scope,
        &doc,
        &snapshot,
        vec![replace("first🙂", "new🙂"), replace("then", "next")],
    )
    .unwrap();
    assert!(checked.valid);
    let plan = service
        .resolve_for_commit(checked.preview_ref.0.as_ref().unwrap(), &doc, &scope, 2)
        .unwrap();
    assert_eq!(
        plan.commit.after.file.cells[0].source,
        "new🙂 next e\u{301}"
    );
    assert_eq!(doc.snapshot().file.cells[0].source, source);
}
#[test]
fn overlapping_matches_duplicate_keys_wrong_kind_and_bad_late_operations_fail_closed() {
    let doc = owner(NativeCellKind::Text, "aaa");
    let (mut service, scope, snapshot) = setup(&doc, true);
    let replace = json!({"type":"replace_text","target":{"cell_id":"原格🙂"},"expected_source_hash":hash("aaa"),"match":"aa","replacement":"x"});
    assert!(matches!(
        preview(&mut service, &scope, &doc, &snapshot, vec![replace]),
        Err(PreviewError::NonuniqueReplacement)
    ));
    assert!(matches!(
        preview(
            &mut service,
            &scope,
            &doc,
            &snapshot,
            vec![
                insert("duplicate", "2", Value::Null),
                insert("duplicate", "3", Value::Null)
            ]
        ),
        Err(PreviewError::InvalidPatch)
    ));
    let mut ask = insert("ask", "question", Value::Null);
    ask["kind"] = json!("Ask");
    assert!(matches!(
        preview(&mut service, &scope, &doc, &snapshot, vec![ask]),
        Err(PreviewError::InvalidArgument)
    ));
    let operations = vec![
        insert("ok", "2+2", Value::Null),
        json!({"type":"move_cell","target":{"client_key":"future-not-declared"},"after":null}),
    ];
    assert!(matches!(
        preview(&mut service, &scope, &doc, &snapshot, operations),
        Err(PreviewError::InvalidPatch)
    ));
    assert_eq!(doc.snapshot().file.cells.len(), 1);
}
#[test]
fn invalid_syntax_cycle_and_definition_conflict_never_get_committable_refs() {
    let doc = owner(NativeCellKind::Math, "let a=b");
    let (mut service, scope, snapshot) = setup(&doc, true);
    for operation in [
        insert("broken", "let f(x)=;", Value::Null),
        insert("conflict", "let a=7", Value::Null),
        insert("cycle", "let b=a", Value::Null),
    ] {
        let result = preview(&mut service, &scope, &doc, &snapshot, vec![operation]).unwrap();
        assert!(!result.valid);
        assert!(result.preview_ref.0.is_none());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.severity == NativePreviewDiagnosticSeverity::Error)
        );
    }
}
#[test]
fn expiry_scope_mode_permission_and_token_tampering_never_commit_old_plans() {
    let doc = owner(NativeCellKind::Math, "2+2");
    let (mut service, scope, snapshot) = setup(&doc, true);
    let result = preview(
        &mut service,
        &scope,
        &doc,
        &snapshot,
        vec![insert("new", "3+3", Value::Null)],
    )
    .unwrap();
    let reference = result.preview_ref.0.unwrap();
    assert!(
        service
            .resolve_for_commit(&reference, &doc, &scope, 300000)
            .is_ok()
    );
    assert!(matches!(
        service.resolve_for_commit(&reference, &doc, &scope, 300001),
        Err(PreviewError::Reference(ReferenceError::Expired))
    ));
    for mutate in [0, 1, 2, 3, 4] {
        let mut stale = scope.clone();
        match mutate {
            0 => stale.grant_revision += 1,
            1 => stale.config_revision += 1,
            2 => stale.definition_revision += 1,
            3 => stale.editor_state_hash = "f".repeat(64),
            _ => stale.task_generation += 1,
        };
        assert!(matches!(
            service.resolve_for_commit(&reference, &doc, &stale, 2),
            Err(PreviewError::Reference(ReferenceError::StaleScope))
        ));
    }
    let mut discussion = scope.clone();
    discussion.execution_mode = false;
    assert!(matches!(
        service.resolve_for_commit(&reference, &doc, &discussion, 2),
        Err(PreviewError::PermissionDenied)
    ));
    let mut tampered = reference.clone();
    tampered.pop();
    tampered.push('z');
    assert!(matches!(
        service.resolve_for_commit(&tampered, &doc, &scope, 2),
        Err(PreviewError::Reference(ReferenceError::Invalid))
    ));
    service.revoke(&reference).unwrap();
    assert!(matches!(
        service.resolve_for_commit(&reference, &doc, &scope, 2),
        Err(PreviewError::Reference(ReferenceError::Revoked))
    ));
}
#[test]
fn patch_budget_and_actual_alias_binding_are_not_inferred_from_documentation() {
    let doc = owner(NativeCellKind::Math, "let sin(x)=x+1");
    let (mut service, scope, snapshot) = setup(&doc, true);
    let good = preview(
        &mut service,
        &scope,
        &doc,
        &snapshot,
        vec![insert("use", "sin (2)", json!({"cell_id":"原格🙂"}))],
    )
    .unwrap();
    assert!(good.valid);
    let plan = service
        .resolve_for_commit(good.preview_ref.0.as_ref().unwrap(), &doc, &scope, 2)
        .unwrap();
    assert!(
        plan.invalidation.analysis.cells[1]
            .uses
            .contains(&"sin".into())
    );
    let ops = vec![json!({"type":"rename_notebook","title":"changed"}); 64];
    assert!(
        preview(&mut service, &scope, &doc, &snapshot, ops.clone())
            .unwrap()
            .valid
    );
    let mut too_many = ops;
    too_many.push(json!({"type":"rename_notebook","title":"extra"}));
    assert!(matches!(
        preview(&mut service, &scope, &doc, &snapshot, too_many),
        Err(PreviewError::InvalidArgument)
    ));
}
#[test]
fn request_cannot_replace_permissions_or_claim_unknown_original_source_hash() {
    let doc = owner(NativeCellKind::Text, "original");
    let (mut service, scope, snapshot) = setup(&doc, true);
    let mut raw = json!({"snapshot_ref":snapshot,"input":{"kind":"source","source":"2+2","dialect":"Modern"}});
    raw["can_write"] = json!(true);
    assert!(PreviewService::decode(&serde_json::to_vec(&raw).unwrap()).is_err());
    let update = json!({"type":"update_cell","target":{"cell_id":"原格🙂"},"expected_source_hash":"0".repeat(64),"source":"replacement"});
    assert!(matches!(
        preview(&mut service, &scope, &doc, &snapshot, vec![update]),
        Err(PreviewError::SourceHashMismatch)
    ));
    let mut readonly = scope.clone();
    readonly.can_preview = false;
    assert!(matches!(
        service.inspect(
            &args(
                &snapshot,
                json!({"kind":"source","source":"2+2","dialect":"Modern"})
            ),
            &doc,
            &readonly,
            1,
            "time".into()
        ),
        Err(PreviewError::PermissionDenied)
    ));
}

#[test]
fn unread_future_cell_ids_and_conflicting_content_changes_cannot_bypass_the_original_snapshot() {
    let doc = owner(NativeCellKind::Text, "unique text");
    let (mut service, scope, snapshot) = setup(&doc, true);
    let replace = json!({"type":"replace_text","target":{"cell_id":"原格🙂"},"expected_source_hash":hash("unique text"),"match":"unique","replacement":"new"});
    let update = json!({"type":"update_cell","target":{"cell_id":"原格🙂"},"expected_source_hash":hash("unique text"),"source":"full replacement"});
    assert!(matches!(
        preview(&mut service, &scope, &doc, &snapshot, vec![replace, update]),
        Err(PreviewError::InvalidPatch)
    ));
    let mut wrong = scope.clone();
    wrong.generation = 2;
    assert!(matches!(
        service.inspect(
            &args(
                &snapshot,
                json!({"kind":"source","source":"2+2","dialect":"Modern"})
            ),
            &doc,
            &wrong,
            1,
            "time".into()
        ),
        Err(PreviewError::StaleSnapshot)
    ));
    let foreign = SourceDocument::new(
        "other-document".into(),
        Serial::new(1).unwrap(),
        doc.snapshot().file.clone(),
    )
    .unwrap();
    assert!(matches!(
        service.inspect(
            &args(
                &snapshot,
                json!({"kind":"source","source":"2+2","dialect":"Modern"})
            ),
            &foreign,
            &scope,
            1,
            "time".into()
        ),
        Err(PreviewError::StaleSnapshot)
    ));
}
#[test]
fn discussion_can_inspect_a_patch_but_cannot_commit_and_reopened_runtime_cannot_reuse_it() {
    let doc = owner(NativeCellKind::Math, "2+2");
    let mut service = PreviewService::new([7; 32]);
    let mut discussion = scope(&doc);
    discussion.execution_mode = false;
    discussion.can_write = false;
    let snapshot = service
        .snapshot(
            &doc,
            SourceCoordinator::default(),
            doc.snapshot()
                .file
                .cells
                .iter()
                .map(|c| c.id.clone())
                .collect(),
            &discussion,
            0,
        )
        .unwrap();
    let result = preview(
        &mut service,
        &discussion,
        &doc,
        &snapshot,
        vec![insert("new", "3+3", Value::Null)],
    )
    .unwrap();
    assert!(result.valid);
    let token = result.preview_ref.0.unwrap();
    assert!(matches!(
        service.resolve_for_commit(&token, &doc, &discussion, 2),
        Err(PreviewError::PermissionDenied)
    ));
    let mut execution = discussion.clone();
    execution.execution_mode = true;
    execution.can_write = true;
    assert!(matches!(
        service.resolve_for_commit(&token, &doc, &execution, 2),
        Err(PreviewError::Reference(ReferenceError::StaleScope))
    ));
    let old = PreviewService::new([99; 32]);
    assert!(matches!(
        old.resolve_for_commit(&token, &doc, &execution, 2),
        Err(PreviewError::Reference(ReferenceError::Invalid))
    ));
}
