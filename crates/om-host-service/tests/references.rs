//! Authenticated runtime-local reference lifetimes and immutable bounded values.
use om_host_service::references::{
    ReferenceError, ReferenceKind, ReferenceRegistry, ReferenceScope,
};
fn scope() -> ReferenceScope {
    ReferenceScope {
        runtime: "runtime".into(),
        document: "doc".into(),
        generation: 1,
        revision: 0,
        execution_epoch: 0,
        snapshot_hash: "a".repeat(64),
        task: None,
        task_generation: 0,
        grant_revision: 1,
        config_revision: 0,
        definition_revision: 0,
        metadata_revision: 26,
        editor_state_hash: String::new(),
        execution_mode: true,
        can_read: true,
        can_preview: true,
        can_write: true,
    }
}
#[test]
fn wrong_issuer_and_nonascii_forgery_cannot_resolve_or_panic() {
    let mut first = ReferenceRegistry::new([1; 32]);
    let second = ReferenceRegistry::<String>::new([2; 32]);
    let token = first
        .issue(
            ReferenceKind::Snapshot,
            scope(),
            "actual frozen data".to_owned(),
            0,
            100,
        )
        .unwrap();
    assert_eq!(
        *first
            .resolve(&token, ReferenceKind::Snapshot, &scope(), 99)
            .unwrap(),
        "actual frozen data"
    );
    assert_eq!(
        second
            .resolve(&token, ReferenceKind::Snapshot, &scope(), 1)
            .unwrap_err(),
        ReferenceError::Invalid
    );
    assert_eq!(
        first
            .resolve(
                &format!("snapshot-1-{}", "🙂".repeat(16)),
                ReferenceKind::Snapshot,
                &scope(),
                1
            )
            .unwrap_err(),
        ReferenceError::Invalid
    );
}
#[test]
fn exact_deadline_purpose_and_counter_limits_are_not_implicit_permission() {
    let mut refs = ReferenceRegistry::new([1; 32]);
    let token = refs
        .issue(ReferenceKind::Preview, scope(), "data".to_owned(), 10, 20)
        .unwrap();
    assert_eq!(
        refs.resolve(&token, ReferenceKind::Snapshot, &scope(), 11)
            .unwrap_err(),
        ReferenceError::WrongKind
    );
    assert_eq!(
        refs.resolve(&token, ReferenceKind::Preview, &scope(), 9)
            .unwrap_err(),
        ReferenceError::Invalid
    );
    assert_eq!(
        refs.resolve(&token, ReferenceKind::Preview, &scope(), 30)
            .unwrap_err(),
        ReferenceError::Expired
    );
    assert_eq!(
        refs.issue(
            ReferenceKind::Preview,
            scope(),
            "data".to_owned(),
            0,
            300001
        )
        .unwrap_err(),
        ReferenceError::Budget
    );
}
#[test]
fn retained_bytes_and_count_are_bounded_and_expired_resources_release_capacity() {
    let mut refs = ReferenceRegistry::new([1; 32]);
    for _ in 0..128 {
        refs.issue(ReferenceKind::Preview, scope(), "data".to_owned(), 0, 1)
            .unwrap();
    }
    assert_eq!(
        refs.issue(ReferenceKind::Preview, scope(), "next".to_owned(), 0, 1)
            .unwrap_err(),
        ReferenceError::Budget
    );
    assert!(
        refs.issue(ReferenceKind::Preview, scope(), "fresh".to_owned(), 1, 1)
            .is_ok()
    );
    let mut other = ReferenceRegistry::new([1; 32]);
    assert_eq!(
        other
            .issue(
                ReferenceKind::Preview,
                scope(),
                "x".repeat(32 * 1024 * 1024),
                0,
                1
            )
            .unwrap_err(),
        ReferenceError::Budget
    );
}
