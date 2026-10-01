//! Configuration, source files, clocks and cancellation via the real Session.
/// Shared fixture helpers.
pub mod support;
use om_kernel::{
    Session,
    config::{Constants, KernelConfig, Language},
    protocol::*,
};
use om_num::ctx::Clock;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use support::*;
#[test]
fn config_roundtrip_resolves_masks_and_pure_requests_do_not_reset_the_interrupt() {
    let mut session = sequential();
    session.config.llm.profiles[0].api_key = Some("test-secret".into());
    let handle = session.interrupt_handle();
    assert!(Arc::ptr_eq(&handle, &session.interrupt_handle()));
    assert!(matches!(session.handle(Request::Interrupt).0, Response::Ok));
    assert!(handle.load(Ordering::Relaxed));
    let (response, _) = session.handle(Request::GetConfig);
    let Response::Config { mut config } =
        serde_json::from_str(&serde_json::to_string(&response).unwrap()).unwrap()
    else {
        panic!()
    };
    assert_eq!(config.llm.profiles[0].api_key.as_deref(), Some("***"));
    assert!(handle.load(Ordering::Relaxed));
    config.llm.profiles.reverse();
    config.general.constants = Constants::Strict;
    config.general.show_steps = false;
    assert!(matches!(
        session.handle(Request::SetConfig { config }).0,
        Response::Ok
    ));
    assert_eq!(
        session.config.llm.profiles[1].api_key.as_deref(),
        Some("test-secret")
    );
    assert!(handle.load(Ordering::Relaxed));
    assert_eq!(
        expressions(&output(&mut session, "v", "e", Dialect::Modern)),
        vec![(1, "e".into())]
    );
    assert!(!handle.load(Ordering::Relaxed));
    let mut config = session.config.clone();
    config.llm.profiles[1].api_key = None;
    assert!(matches!(
        session
            .handle(Request::SetConfig {
                config: config.clone()
            })
            .0,
        Response::Ok
    ));
    assert!(session.config.llm.profiles[1].api_key.is_none());
    config.llm.profiles.push(config.llm.profiles[0].clone());
    assert!(matches!(
        session.handle(Request::SetConfig { config }).0,
        Response::Error { .. }
    ));
    assert_eq!(session.config.llm.profiles.len(), 2);
    session.config.general.language = Language::En;
    let Response::Error { message } = session.handle(Request::RunAll).0 else {
        panic!()
    };
    assert!(message.contains("err.not_implemented"));
    assert!(message.is_ascii());
    assert_eq!(
        expressions(&output(&mut session, "history", "Out[1]", Dialect::Wolfram)),
        vec![(2, "e".into())]
    );
}

struct TickingClock {
    value: AtomicU64,
    increment: AtomicU64,
    cancel: AtomicBool,
    handle: std::sync::Mutex<Option<Arc<AtomicBool>>>,
}
impl TickingClock {
    fn new(increment: u64) -> Self {
        Self {
            value: AtomicU64::new(0),
            increment: AtomicU64::new(increment),
            cancel: AtomicBool::new(false),
            handle: Default::default(),
        }
    }
}
impl Clock for TickingClock {
    fn now_ms(&self) -> f64 {
        if self.cancel.load(Ordering::Relaxed)
            && let Some(handle) = self.handle.lock().unwrap().as_ref()
        {
            handle.store(true, Ordering::Relaxed);
        }
        self.value
            .fetch_add(self.increment.load(Ordering::Relaxed), Ordering::Relaxed) as f64
    }
}

#[test]
fn injected_clock_and_shared_flag_stop_real_execution_and_allow_recovery() {
    let clock = Arc::new(TickingClock::new(1));
    let mut config = KernelConfig::default();
    config.general.reactive = false;
    let mut session = Session::new(config, Some(clock.clone()));
    let fast = output(&mut session, "timed", "1", Dialect::Modern);
    assert!(fast.timing_ms > 0.0);
    session.config.general.eval_timeout_ms = 0;
    let timedout = output(&mut session, "timeout", "let a=5", Dialect::Modern);
    assert!(
        timedout
            .items
            .iter()
            .any(|i| matches!(i,OutputItem::Error{message,..} if message.contains("$Aborted")))
    );
    assert!(timedout.messages.iter().any(|m| m.tag == "timeout"));
    assert_eq!(
        session.notebook.cells.last().unwrap().status,
        CellStatus::Error
    );
    session.config.general.eval_timeout_ms = 30_000;
    assert_eq!(
        expressions(&output(&mut session, "a", "a", Dialect::Modern))[0].1,
        "a"
    );
    *clock.handle.lock().unwrap() = Some(session.interrupt_handle());
    clock.cancel.store(true, Ordering::Relaxed);
    let canceled = output(&mut session, "cancel", "2", Dialect::Modern);
    assert!(canceled.messages.iter().any(|m| m.tag == "interrupted"));
    clock.cancel.store(false, Ordering::Relaxed);
    assert_eq!(
        expressions(&output(&mut session, "recover", "3", Dialect::Modern))[0].1,
        "3"
    );
    assert!(!session.interrupt_handle().load(Ordering::Relaxed));
}
