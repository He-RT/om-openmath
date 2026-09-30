//! Core and expression-independent numeric consumers share one interrupt type.

use om_core::ctx::{Abort, Clock, Interrupt, Message, Messages, MsgLevel};

#[test]
fn core_reexports_numeric_context_without_a_wrapper() {
    let core = Interrupt::default();
    let num: &om_num::ctx::Interrupt = &core;
    assert_eq!(num.tick(), Ok(()));
    let typed_abort: om_num::ctx::Abort = Abort::Budget;
    assert_eq!(typed_abort, om_num::ctx::Abort::Budget);
    let clock: Option<std::sync::Arc<dyn Clock>> = None;
    let numeric_clock: Option<std::sync::Arc<dyn om_num::ctx::Clock>> = clock;
    assert!(numeric_clock.is_none());
    let mut msgs = Messages::default();
    msgs.push(Message {
        symbol: "Solve".into(),
        tag: "svars".into(),
        text: "variables".into(),
        level: MsgLevel::Warning,
    });
    assert_eq!(msgs.take().len(), 1);
}
