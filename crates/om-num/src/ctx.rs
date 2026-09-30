//! Portable computation limits; frontends inject their own clocks.

use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const DEFAULT_STEPS: u64 = 500_000_000;

/// A monotonic or wall-clock time source supplied by the host.
pub trait Clock: Send + Sync {
    /// Current host time in milliseconds, in the same epoch as the deadline.
    fn now_ms(&self) -> f64;
}

/// Shared cancellation plus a local step budget and optional host deadline.
pub struct Interrupt {
    /// External cancellation flag; true stops the computation.
    pub flag: Arc<AtomicBool>,
    /// Absolute deadline in the injected clock's millisecond epoch.
    pub deadline_ms: Option<f64>,
    /// Optional native or browser clock; no platform clock is used by default.
    pub clock: Option<Arc<dyn Clock>>,
    /// Remaining successful ticks, local to this computation.
    pub steps_left: Cell<u64>,
}
/// The reason a computation stopped before producing a result.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Abort {
    /// The external flag was set.
    #[error("interrupted")]
    Interrupted,
    /// The host deadline was reached.
    #[error("time limit exceeded")]
    Timeout,
    /// No computation steps remain.
    #[error("step budget exceeded")]
    Budget,
}
impl Default for Interrupt {
    fn default() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            deadline_ms: None,
            clock: None,
            steps_left: Cell::new(DEFAULT_STEPS),
        }
    }
}
impl Interrupt {
    /// Charge one step, checking cancellation immediately and deadlines periodically.
    /// N available steps permit N ticks; subsequent ticks return Budget without wrapping.
    /// Clock polls are at most 4096 ticks apart, with eager checks for small budgets.
    #[inline]
    pub fn tick(&self) -> Result<(), Abort> {
        if self.flag.load(Ordering::Relaxed) {
            return Err(Abort::Interrupted);
        }
        let remaining = self.steps_left.get();
        if remaining == 0 {
            return Err(Abort::Budget);
        }
        self.steps_left.set(remaining - 1);
        if (remaining.is_multiple_of(4096) || remaining == DEFAULT_STEPS || remaining <= 4096)
            && let (Some(deadline), Some(clock)) = (self.deadline_ms, &self.clock)
            && clock.now_ms() >= deadline
        {
            return Err(Abort::Timeout);
        }
        Ok(())
    }
}
