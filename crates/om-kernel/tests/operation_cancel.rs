//! External operation tokens cannot be cleared by Session's normal per-request reset.
use om_kernel::{Session, protocol::*};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[test]
fn cancelled_isolated_operation_never_rearms_its_token_or_advances_definitions() {
    let cancel = Arc::new(AtomicBool::new(true));
    let mut config = KernelConfig::default();
    config.general.reactive = false;
    let mut session = Session::with_cancel_token(config.clone(), None, cancel.clone());
    let (response, _) = session.handle(Request::Evaluate {
        cell_id: "cancelled".into(),
        source: "let cancelled_value=99; cancelled_value".into(),
        dialect: Dialect::Modern,
    });
    let Response::Evaluated { output, .. } = response else {
        panic!()
    };
    assert!(cancel.load(Ordering::Relaxed));
    assert!(
        output
            .items
            .iter()
            .any(|i| matches!(i, OutputItem::Error { .. }))
    );
    let mut other = Session::with_cancel_token(config, None, Arc::new(AtomicBool::new(false)));
    let (response, _) = other.handle(Request::Evaluate {
        cell_id: "other".into(),
        source: "2+2".into(),
        dialect: Dialect::Modern,
    });
    let Response::Evaluated { output, .. } = response else {
        panic!()
    };
    assert!(matches!(&output.items[0],OutputItem::Expr{input_form,..}if input_form=="4"));
    assert!(cancel.load(Ordering::Relaxed));
}
