//! Inverse-source behavior: later unrelated source stays intact; a related conflict rejects all.
use om_host_service::{
    document::{
        SourceDocument,
        undo::{UndoError, merge_group, merge_inverse},
    },
    protocol::{Serial, generated::*},
};
fn cell(id: &str, source: &str) -> NativeSourceCell {
    NativeSourceCell {
        id: id.into(),
        kind: NativeCellKind::Math,
        source: source.into(),
        dialect: Dialect::Modern,
    }
}
fn file(ids: &[(&str, &str)]) -> NativeSourceFile {
    NativeSourceFile {
        version: 1,
        title: "original".into(),
        cells: ids.iter().map(|(id, source)| cell(id, source)).collect(),
    }
}
fn stage(before: NativeSourceFile, after: NativeSourceFile) -> NativeSourceCommit {
    SourceDocument::new("document".into(), Serial::new(1).unwrap(), before)
        .unwrap()
        .prepare(
            after,
            "original-operation".into(),
            "original-transaction".into(),
            "original-event".into(),
            "time".into(),
        )
        .unwrap()
}
fn current(original: &NativeSourceCommit, file: NativeSourceFile) -> NativeSourceSnapshot {
    SourceDocument::restore(original.after.clone(), Serial::new(2).unwrap())
        .unwrap()
        .prepare(
            file,
            "later-operation".into(),
            "later-transaction".into(),
            "later-event".into(),
            "later".into(),
        )
        .unwrap()
        .after
}
#[test]
fn inverse_preserves_later_unrelated_content_title_and_new_cells() {
    let original = stage(
        file(&[("a", "let a=2"), ("b", "a+1")]),
        file(&[("a", "let a=5"), ("b", "a+1")]),
    );
    let mut newer = file(&[("new", "11"), ("a", "let a=5"), ("b", "later user content")]);
    newer.title = "later title".into();
    let merged = merge_inverse(&original, &current(&original, newer)).unwrap();
    assert_eq!(merged.title, "later title");
    assert_eq!(
        merged
            .cells
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["new", "a", "b"]
    );
    assert_eq!(merged.cells[1].source, "let a=2");
    assert_eq!(merged.cells[2].source, "later user content");
}
#[test]
fn later_related_edit_rejects_whole_inverse_without_a_partial_result() {
    let original = stage(
        file(&[("a", "2"), ("b", "3")]),
        file(&[("a", "5"), ("b", "6")]),
    );
    let newer = current(&original, file(&[("a", "5"), ("b", "new manual edit")]));
    assert!(matches!(
        merge_inverse(&original, &newer),
        Err(UndoError::Conflict)
    ));
    assert_eq!(newer.file.cells[0].source, "5");
}
#[test]
fn structural_inverse_retains_unrelated_edits_outside_original_anchors() {
    let original = stage(
        file(&[("left", "1"), ("deleted", "2"), ("right", "3")]),
        file(&[("left", "1"), ("right", "3")]),
    );
    let newer = current(
        &original,
        file(&[
            ("extra", "9"),
            ("left", "later left"),
            ("right", "3"),
            ("tail", "10"),
        ]),
    );
    let merged = merge_inverse(&original, &newer).unwrap();
    assert_eq!(
        merged
            .cells
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["extra", "left", "deleted", "right", "tail"]
    );
    assert_eq!(merged.cells[1].source, "later left");
}
#[test]
fn new_content_inside_structural_edit_or_reused_deleted_identity_is_a_conflict() {
    let original = stage(
        file(&[("left", "1"), ("deleted", "2"), ("right", "3")]),
        file(&[("left", "1"), ("right", "3")]),
    );
    for extra in [("new", "9"), ("deleted", "new identity contents")] {
        let newer = current(&original, file(&[("left", "1"), extra, ("right", "3")]));
        assert!(matches!(
            merge_inverse(&original, &newer),
            Err(UndoError::Conflict)
        ));
    }
}
#[test]
fn inverse_move_restores_order_but_keeps_unmodified_cells_current_content() {
    let original = stage(
        file(&[("left", "0"), ("a", "1"), ("b", "2"), ("right", "3")]),
        file(&[("left", "0"), ("b", "2"), ("a", "1"), ("right", "3")]),
    );
    let newer = current(
        &original,
        file(&[
            ("left", "0"),
            ("b", "later edit"),
            ("a", "1"),
            ("right", "3"),
        ]),
    );
    let merged = merge_inverse(&original, &newer).unwrap();
    assert_eq!(merged.cells[1].id, "a");
    assert_eq!(merged.cells[2].source, "later edit");
}
#[test]
fn raw_unicode_related_source_and_later_title_are_not_normalized_or_overwritten() {
    let mut after = file(&[("a", "é")]);
    after.title = "changed".into();
    let original = stage(file(&[("a", "old")]), after);
    let newer = current(&original, file(&[("a", "e\u{301}")]));
    assert!(matches!(
        merge_inverse(&original, &newer),
        Err(UndoError::Conflict)
    ));
    let mut later = original.after.clone();
    later.file.title = "later title".into();
    later.snapshot_hash = om_host_service::document::snapshot_hash(&later);
    assert!(matches!(
        merge_inverse(&original, &later),
        Err(UndoError::Conflict)
    ));
}

#[test]
fn later_edit_then_same_bytes_are_not_mistaken_for_the_original_revision() {
    let original = stage(file(&[("a", "2")]), file(&[("a", "5")]));
    let first = current(&original, file(&[("a", "9")]));
    let back = SourceDocument::restore(first, Serial::new(2).unwrap())
        .unwrap()
        .prepare(
            file(&[("a", "5")]),
            "back-op".into(),
            "back-tx".into(),
            "back-event".into(),
            "back".into(),
        )
        .unwrap()
        .after;
    assert!(matches!(
        merge_inverse(&original, &back),
        Err(UndoError::Conflict)
    ));
}

#[test]
fn inverse_insert_removes_only_original_cell_and_refuses_its_later_modification() {
    let original = stage(
        file(&[("left", "1"), ("right", "3")]),
        file(&[("left", "1"), ("inserted", "2"), ("right", "3")]),
    );
    let newer = current(
        &original,
        file(&[
            ("extra", "9"),
            ("left", "1"),
            ("inserted", "2"),
            ("right", "3"),
            ("tail", "10"),
        ]),
    );
    let merged = merge_inverse(&original, &newer).unwrap();
    assert_eq!(
        merged
            .cells
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["extra", "left", "right", "tail"]
    );
    let changed = current(
        &original,
        file(&[("left", "1"), ("inserted", "user edit"), ("right", "3")]),
    );
    assert!(matches!(
        merge_inverse(&original, &changed),
        Err(UndoError::Conflict)
    ));
}
#[test]
fn unrelated_later_deletion_is_not_resurrected_by_content_inverse() {
    let original = stage(
        file(&[("a", "2"), ("b", "3")]),
        file(&[("a", "5"), ("b", "3")]),
    );
    let newer = current(&original, file(&[("a", "5")]));
    let merged = merge_inverse(&original, &newer).unwrap();
    assert_eq!(merged.cells.len(), 1);
    assert_eq!(merged.cells[0].source, "2");
}
#[test]
fn tampered_transaction_other_document_and_precommit_snapshot_cannot_be_inverted() {
    let original = stage(file(&[("a", "2")]), file(&[("a", "5")]));
    assert!(matches!(
        merge_inverse(&original, &original.before),
        Err(UndoError::InvalidRecord)
    ));
    let other =
        SourceDocument::new("other".into(), Serial::new(1).unwrap(), file(&[("a", "5")])).unwrap();
    assert!(matches!(
        merge_inverse(&original, other.snapshot()),
        Err(UndoError::InvalidRecord)
    ));
    let mut forged = original.clone();
    forged.before.file.cells[0].source = "forged".into();
    assert!(matches!(
        merge_inverse(&forged, &original.after),
        Err(UndoError::InvalidRecord)
    ));
}

#[test]
fn disjoint_original_order_changes_do_not_protect_the_unchanged_gap_between_them() {
    let original = stage(
        file(&[
            ("left", "0"),
            ("a", "1"),
            ("middle1", "2"),
            ("middle2", "3"),
            ("b", "4"),
            ("right", "5"),
        ]),
        file(&[
            ("left", "0"),
            ("middle1", "2"),
            ("middle2", "3"),
            ("right", "5"),
        ]),
    );
    let newer = current(
        &original,
        file(&[
            ("left", "0"),
            ("middle1", "2"),
            ("user", "new gap"),
            ("middle2", "3"),
            ("right", "5"),
        ]),
    );
    let merged = merge_inverse(&original, &newer).unwrap();
    assert_eq!(
        merged
            .cells
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["left", "a", "middle1", "user", "middle2", "b", "right"]
    );
}

#[test]
fn all_small_distinct_identity_orders_and_memberships_have_exact_inverse() {
    fn collect(
        prefix: &mut Vec<&'static str>,
        remaining: &[&'static str],
        out: &mut Vec<NativeSourceFile>,
    ) {
        out.push(file(
            &prefix.iter().map(|id| (*id, *id)).collect::<Vec<_>>(),
        ));
        for (index, id) in remaining.iter().enumerate() {
            prefix.push(id);
            let mut rest = remaining.to_vec();
            rest.remove(index);
            collect(prefix, &rest, out);
            prefix.pop();
        }
    }
    let mut states = Vec::new();
    collect(&mut Vec::new(), &["a", "b", "c", "d"], &mut states);
    assert_eq!(states.len(), 65);
    for before in &states {
        for after in &states {
            let original = stage(before.clone(), after.clone());
            let merged = merge_inverse(&original, &original.after).unwrap();
            assert_eq!(
                serde_json::to_value(merged).unwrap(),
                serde_json::to_value(before).unwrap()
            );
        }
    }
}

#[test]
fn a_whole_group_can_reverse_its_own_versions_while_preserving_unrelated_later_input() {
    let first = stage(
        file(&[("a", "2"), ("b", "unrelated")]),
        file(&[("a", "3"), ("b", "unrelated")]),
    );
    let second = SourceDocument::restore(first.after.clone(), Serial::new(2).unwrap())
        .unwrap()
        .prepare(
            file(&[("a", "5"), ("b", "unrelated")]),
            "second-op".into(),
            "second-tx".into(),
            "second-event".into(),
            "second".into(),
        )
        .unwrap();
    let newer = current(&second, file(&[("a", "5"), ("b", "later input")]));
    let merged = merge_group(&[second, first], &newer).unwrap();
    assert_eq!(merged.cells[0].source, "2");
    assert_eq!(merged.cells[1].source, "later input");
}

#[test]
fn a_group_insert_then_edit_is_atomic_and_can_be_removed_without_an_intermediate_snapshot() {
    let first = stage(file(&[("a", "2")]), file(&[("a", "2"), ("new", "3")]));
    let second = SourceDocument::restore(first.after.clone(), Serial::new(2).unwrap())
        .unwrap()
        .prepare(
            file(&[("a", "2"), ("new", "5")]),
            "second-op".into(),
            "second-tx".into(),
            "second-event".into(),
            "second".into(),
        )
        .unwrap();
    let merged = merge_group(&[second.clone(), first], &second.after).unwrap();
    assert_eq!(merged.cells.len(), 1);
    assert_eq!(merged.cells[0].id, "a");
}

#[test]
fn group_order_identity_and_late_conflict_cannot_publish_partial_inverse() {
    let first = stage(
        file(&[("a", "2"), ("b", "3")]),
        file(&[("a", "5"), ("b", "3")]),
    );
    let second = SourceDocument::restore(first.after.clone(), Serial::new(2).unwrap())
        .unwrap()
        .prepare(
            file(&[("a", "5"), ("b", "6")]),
            "second-op".into(),
            "second-tx".into(),
            "second-event".into(),
            "second".into(),
        )
        .unwrap();
    assert!(matches!(
        merge_group(&[first.clone(), second.clone()], &second.after),
        Err(UndoError::InvalidRecord)
    ));
    assert!(matches!(
        merge_group(&[second.clone(), second.clone()], &second.after),
        Err(UndoError::InvalidRecord)
    ));
    let later = current(&second, file(&[("a", "later user input"), ("b", "6")]));
    assert!(matches!(
        merge_group(&[second.clone(), first], &later),
        Err(UndoError::Conflict)
    ));
    assert_eq!(later.file.cells[1].source, "6");
    assert!(matches!(
        merge_group(&[], &second.after),
        Err(UndoError::InvalidRecord)
    ));
    assert!(matches!(
        merge_group(&vec![second.clone(); 33], &second.after),
        Err(UndoError::InvalidRecord)
    ));
}

#[test]
fn maximum_32_transaction_group_reconstructs_the_original_source() {
    let mut owner = SourceDocument::new(
        "document".into(),
        Serial::new(1).unwrap(),
        file(&[("a", "0")]),
    )
    .unwrap();
    let mut plans = Vec::new();
    for n in 1..=32 {
        let plan = owner
            .prepare(
                file(&[("a", &n.to_string())]),
                format!("op-{n}"),
                format!("tx-{n}"),
                format!("event-{n}"),
                "time".into(),
            )
            .unwrap();
        owner = SourceDocument::restore(plan.after.clone(), Serial::new(1).unwrap()).unwrap();
        plans.push(plan);
    }
    plans.reverse();
    let merged = merge_group(&plans, owner.snapshot()).unwrap();
    assert_eq!(merged.cells[0].source, "0");
    assert_eq!(owner.snapshot().revision.get(), 32);
}
