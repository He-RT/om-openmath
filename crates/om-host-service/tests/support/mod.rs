//! Shared deterministic test clocks/cancellation. Real math/services must still execute.
use om_num::ctx::Clock;
use std::sync::atomic::{AtomicU64, Ordering};
pub struct ControlledClock(AtomicU64);
impl ControlledClock {
    pub fn new() -> Self {
        Self(AtomicU64::new(0))
    }
    pub fn advance(&self, ms: u64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}
impl Clock for ControlledClock {
    fn now_ms(&self) -> f64 {
        self.0.load(Ordering::SeqCst) as f64
    }
}
