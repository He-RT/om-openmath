//! Deterministic budget, cancellation and injected deadline tests.

use om_num::ctx::{Abort, Clock, Interrupt};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

struct FakeClock {
    ms: AtomicU64,
    calls: AtomicU64,
}
impl Clock for FakeClock {
    fn now_ms(&self) -> f64 {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.ms.load(Ordering::Relaxed) as f64
    }
}
fn fake(ms: u64) -> Arc<FakeClock> {
    Arc::new(FakeClock {
        ms: AtomicU64::new(ms),
        calls: AtomicU64::new(0),
    })
}

#[test]
fn default_budget_and_short_quotas_do_not_underflow() {
    let ctx = Interrupt::default();
    assert_eq!(ctx.steps_left.get(), 500_000_000);
    assert!(!ctx.flag.load(Ordering::Relaxed));
    assert!(ctx.clock.is_none() && ctx.deadline_ms.is_none());
    for n in 0..100 {
        ctx.steps_left.set(n);
        for _ in 0..n {
            assert_eq!(ctx.tick(), Ok(()));
        }
        assert_eq!(ctx.steps_left.get(), 0);
        assert_eq!(ctx.tick(), Err(Abort::Budget));
        assert_eq!(ctx.steps_left.get(), 0);
    }
}

#[test]
fn shared_atomic_flag_interrupts_immediately_even_between_clock_polls() {
    let ctx = Interrupt::default();
    assert_eq!(ctx.tick(), Ok(()));
    let flag = ctx.flag.clone();
    std::thread::spawn(move || flag.store(true, Ordering::Relaxed))
        .join()
        .unwrap();
    assert_eq!(ctx.tick(), Err(Abort::Interrupted));
    ctx.steps_left.set(0);
    assert_eq!(ctx.tick(), Err(Abort::Interrupted));
    ctx.flag.store(false, Ordering::Relaxed);
    assert_eq!(ctx.tick(), Err(Abort::Budget));
}

#[test]
fn expired_default_deadline_is_detected_on_the_first_tick() {
    let clock = fake(10);
    let ctx = Interrupt {
        clock: Some(clock.clone()),
        deadline_ms: Some(10.0),
        ..Interrupt::default()
    };
    assert_eq!(ctx.tick(), Err(Abort::Timeout));
    assert_eq!(clock.calls.load(Ordering::Relaxed), 1);
}

#[test]
fn deadline_polling_is_bounded_and_not_per_iteration() {
    let clock = fake(0);
    let ctx = Interrupt {
        clock: Some(clock.clone()),
        deadline_ms: Some(10.0),
        ..Interrupt::default()
    };
    ctx.steps_left.set(8192);
    assert_eq!(ctx.tick(), Ok(()));
    clock.ms.store(10, Ordering::Relaxed);
    for n in 1..=4096 {
        match ctx.tick() {
            Err(Abort::Timeout) => {
                assert!(n <= 4096);
                assert_eq!(clock.calls.load(Ordering::Relaxed), 2);
                return;
            }
            Ok(()) => {}
            other => panic!("unexpected {other:?}"),
        }
    }
    panic!("deadline was not polled within 4096 ticks");
}

#[test]
fn missing_clock_or_deadline_needs_no_native_time() {
    let ctx = Interrupt {
        deadline_ms: Some(-1.0),
        ..Interrupt::default()
    };
    assert_eq!(ctx.tick(), Ok(()));
    let clock = fake(100);
    let ctx = Interrupt {
        clock: Some(clock.clone()),
        ..Interrupt::default()
    };
    assert_eq!(ctx.tick(), Ok(()));
    assert_eq!(clock.calls.load(Ordering::Relaxed), 0);
}

#[test]
fn short_deadlines_and_error_priority_are_explicit() {
    let clock = fake(20);
    let ctx = Interrupt {
        clock: Some(clock),
        deadline_ms: Some(10.0),
        ..Interrupt::default()
    };
    ctx.steps_left.set(100);
    assert_eq!(ctx.tick(), Err(Abort::Timeout));
    ctx.steps_left.set(0);
    assert_eq!(ctx.tick(), Err(Abort::Budget));
    ctx.flag.store(true, Ordering::Relaxed);
    assert_eq!(ctx.tick(), Err(Abort::Interrupted));
    assert_eq!(Abort::Interrupted.to_string(), "interrupted");
    assert_eq!(Abort::Timeout.to_string(), "time limit exceeded");
    assert_eq!(Abort::Budget.to_string(), "step budget exceeded");
}
