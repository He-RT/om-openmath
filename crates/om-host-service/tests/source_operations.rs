//! Atomic source-only batches, real dependency facts and mathematically relevant epochs.
use om_host_service::{
    document::{SourceDocument, coordinator::SourceCoordinator},
    protocol::{Serial, generated::*},
};
use om_kernel::config::{Constants, Language};
fn cell(id: &str, kind: NativeCellKind, source: &str) -> NativeSourceCell {
    NativeSourceCell {
        id: id.into(),
        kind,
        source: source.into(),
        dialect: Dialect::Modern,
    }
}
fn owner() -> SourceDocument {
    SourceDocument::new(
        "doc".into(),
        Serial::new(1).unwrap(),
        NativeSourceFile {
            version: 1,
            title: "".into(),
            cells: vec![
                cell("a", NativeCellKind::Math, "let a=2"),
                cell("b", NativeCellKind::Math, "let b=a+1"),
                cell("text", NativeCellKind::Text, "notes"),
            ],
        },
    )
    .unwrap()
}
fn operation(value: serde_json::Value) -> NativeSourceOperation {
    serde_json::from_value(value).unwrap()
}
fn prepare(
    owner: &SourceDocument,
    ops: Vec<NativeSourceOperation>,
) -> om_host_service::document::coordinator::SourceMutationPlan {
    SourceCoordinator::default()
        .prepare(
            owner,
            &ops,
            "op".into(),
            "tx".into(),
            "event".into(),
            "2026-10-09T00:00:00Z".into(),
        )
        .unwrap()
}
#[test]
fn whole_delete_insert_move_update_rename_is_frozen_once_without_changing_authority() {
    let owner = owner();
    let p = prepare(
        &owner,
        vec![
            operation(serde_json::json!({"kind":"delete_cells","cell_ids":["a","b"]})),
            operation(
                serde_json::json!({"kind":"insert_cell","cell":{"id":"new","kind":"Math","source":"let value=5","dialect":"Modern"},"after_cell_id":null}),
            ),
            operation(
                serde_json::json!({"kind":"move_cells","cell_ids":["text"],"after_cell_id":null}),
            ),
            operation(serde_json::json!({"kind":"rename_notebook","title":"changed"})),
        ],
    );
    assert_eq!(owner.snapshot().file.cells.len(), 3);
    assert_eq!(owner.snapshot().revision.get(), 0);
    assert_eq!(
        p.commit
            .after
            .file
            .cells
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["text", "new"]
    );
    assert_eq!(p.commit.after.revision.get(), 1);
    assert_eq!(p.commit.after.execution_epoch.get(), 1);
}
#[test]
fn pure_text_and_title_changes_do_not_invalidate_math_or_advance_execution_epoch() {
    let owner = owner();
    let p = prepare(
        &owner,
        vec![
            operation(
                serde_json::json!({"kind":"move_cells","cell_ids":["text"],"after_cell_id":null}),
            ),
            operation(serde_json::json!({"kind":"rename_notebook","title":"new"})),
            operation(
                serde_json::json!({"kind":"update_cell","cell":{"id":"text","kind":"Text","source":"中文🙂","dialect":"Modern"}}),
            ),
        ],
    );
    assert!(!p.invalidation.math_changed);
    assert!(p.invalidation.affected_cells.is_empty());
    assert_eq!(p.commit.after.execution_epoch.get(), 0);
    assert_eq!(p.commit.after.revision.get(), 1);
}
#[test]
fn affected_dependencies_and_real_conflicts_are_from_the_shared_parser() {
    let owner = owner();
    let p = prepare(
        &owner,
        vec![operation(
            serde_json::json!({"kind":"update_cell","cell":{"id":"a","kind":"Math","source":"let a=5","dialect":"Modern"}}),
        )],
    );
    assert_eq!(p.invalidation.affected_cells, vec!["a", "b"]);
    let p = prepare(
        &owner,
        vec![operation(
            serde_json::json!({"kind":"insert_cell","cell":{"id":"duplicate","kind":"Math","source":"let a=8","dialect":"Modern"},"after_cell_id":"b"}),
        )],
    );
    assert_eq!(p.invalidation.analysis.conflicts[0].symbol, "a");
}
#[test]
fn bad_late_operation_never_publishes_earlier_changes_and_patch_budget_is_fixed() {
    let owner = owner();
    let coordinator = SourceCoordinator::default();
    let ops = vec![
        operation(serde_json::json!({"kind":"delete_cells","cell_ids":["a"]})),
        operation(
            serde_json::json!({"kind":"move_cells","cell_ids":["b"],"after_cell_id":"missing"}),
        ),
    ];
    assert!(
        coordinator
            .prepare(
                &owner,
                &ops,
                "op".into(),
                "tx".into(),
                "event".into(),
                "time".into()
            )
            .is_err()
    );
    assert_eq!(owner.snapshot().file.cells[0].id, "a");
    let ops = vec![operation(serde_json::json!({"kind":"rename_notebook","title":"x"})); 65];
    assert!(
        coordinator
            .prepare(
                &owner,
                &ops,
                "op".into(),
                "tx".into(),
                "event".into(),
                "time".into()
            )
            .is_err()
    );
}
#[test]
fn actual_setting_values_are_frozen_and_semantic_setting_changes_advance_epoch() {
    let owner = owner();
    let before = om_kernel::KernelConfig::default().general;
    let mut after = before.clone();
    after.constants = Constants::Strict;
    let p = owner
        .prepare_settings(
            &before,
            &after,
            "setting".into(),
            "tx".into(),
            "event".into(),
            "2026-10-09T00:00:00Z".into(),
        )
        .unwrap();
    assert_eq!(p.after.execution_epoch.get(), 1);
    assert_eq!(p.before.file.cells[0].source, p.after.file.cells[0].source);
    assert_eq!(
        p.calculation_change.0.unwrap().after.constants,
        NativeCalculationSettingsConstants::Strict
    );
    after = before.clone();
    after.language = Language::En;
    let p = owner
        .prepare_settings(
            &before,
            &after,
            "locale".into(),
            "tx".into(),
            "event".into(),
            "2026-10-09T00:00:00Z".into(),
        )
        .unwrap();
    assert_eq!(p.after.execution_epoch.get(), 0);
}
