//! Cancellation and the final physical COMMIT admission share one short lifecycle lock.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AcceptancePhase {
    Working,
    Prepared,
    Committing,
    Unknown,
    Accepted,
    Discarded,
}

pub(crate) struct Lifecycle {
    pub(crate) token: Arc<AtomicBool>,
    phase: Mutex<AcceptancePhase>,
    started: AtomicBool,
}
impl Lifecycle {
    pub(crate) fn new(token: Arc<AtomicBool>) -> Self {
        Self {
            token,
            phase: Mutex::new(AcceptancePhase::Working),
            started: AtomicBool::new(false),
        }
    }
    pub(crate) fn start(&self) {
        self.started.store(true, Ordering::Release);
    }
    pub(crate) fn started(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }
    pub(crate) fn prepared(&self) -> Result<(), KernelWorkerError> {
        let mut phase = self.phase.lock().map_err(|_| KernelWorkerError::Internal)?;
        if *phase != AcceptancePhase::Working {
            return Err(KernelWorkerError::Invalid);
        }
        *phase = AcceptancePhase::Prepared;
        Ok(())
    }
    pub(crate) fn cancel(&self) -> Result<bool, KernelWorkerError> {
        let phase = self.phase.lock().map_err(|_| KernelWorkerError::Internal)?;
        if matches!(
            *phase,
            AcceptancePhase::Accepted | AcceptancePhase::Discarded
        ) {
            return Ok(false);
        }
        self.token.store(true, Ordering::Release);
        Ok(true)
    }
    pub(crate) fn enter(&self) -> Result<(), KernelWorkerError> {
        let mut phase = self.phase.lock().map_err(|_| KernelWorkerError::Internal)?;
        if *phase != AcceptancePhase::Prepared {
            return Err(KernelWorkerError::Invalid);
        }
        if self.token.load(Ordering::Acquire) {
            return Err(KernelWorkerError::Cancelled);
        }
        *phase = AcceptancePhase::Committing;
        Ok(())
    }
    pub(crate) fn unknown(&self) -> Result<(), KernelWorkerError> {
        let mut phase = self.phase.lock().map_err(|_| KernelWorkerError::Internal)?;
        if !matches!(
            *phase,
            AcceptancePhase::Committing | AcceptancePhase::Unknown
        ) {
            return Err(KernelWorkerError::Invalid);
        }
        *phase = AcceptancePhase::Unknown;
        Ok(())
    }
    pub(crate) fn accept(&self) -> Result<(), KernelWorkerError> {
        let mut phase = self.phase.lock().map_err(|_| KernelWorkerError::Internal)?;
        if !matches!(
            *phase,
            AcceptancePhase::Committing | AcceptancePhase::Unknown | AcceptancePhase::Accepted
        ) {
            return Err(KernelWorkerError::Invalid);
        }
        // Cancel after enter is a stop request, never a fabricated rollback of durable bytes.
        *phase = AcceptancePhase::Accepted;
        Ok(())
    }
    pub(crate) fn discard(&self) -> Result<(), KernelWorkerError> {
        let mut phase = self.phase.lock().map_err(|_| KernelWorkerError::Internal)?;
        if matches!(
            *phase,
            AcceptancePhase::Accepted | AcceptancePhase::Committing | AcceptancePhase::Unknown
        ) {
            return Err(KernelWorkerError::Invalid);
        }
        *phase = AcceptancePhase::Discarded;
        Ok(())
    }
    pub(crate) fn confirmed_no_commit(&self) -> Result<(), KernelWorkerError> {
        let mut phase = self.phase.lock().map_err(|_| KernelWorkerError::Internal)?;
        if *phase == AcceptancePhase::Accepted {
            return Err(KernelWorkerError::Invalid);
        }
        self.token.store(true, Ordering::Release);
        *phase = AcceptancePhase::Discarded;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    #[test]
    fn cancel_and_actual_commit_barrier_have_one_linearized_order() {
        for _ in 0..64 {
            let lifecycle = Arc::new(Lifecycle::new(Arc::new(AtomicBool::new(false))));
            lifecycle.prepared().unwrap();
            let barrier = Arc::new(Barrier::new(2));
            let stop = lifecycle.clone();
            let ready = barrier.clone();
            let thread = std::thread::spawn(move || {
                ready.wait();
                stop.cancel().unwrap()
            });
            barrier.wait();
            let entered = lifecycle.enter();
            assert!(thread.join().unwrap());
            match entered {
                Ok(()) => {
                    lifecycle.unknown().unwrap();
                    assert!(lifecycle.discard().is_err());
                    lifecycle.accept().unwrap();
                    assert!(!lifecycle.cancel().unwrap());
                }
                Err(KernelWorkerError::Cancelled) => {
                    assert!(lifecycle.accept().is_err());
                    lifecycle.discard().unwrap();
                    assert!(!lifecycle.cancel().unwrap());
                }
                _ => panic!("unexpected barrier result"),
            }
        }
    }
    #[test]
    fn unstarted_or_prepared_or_unknown_is_never_optimistically_accepted_or_discarded() {
        let lifecycle = Lifecycle::new(Arc::new(AtomicBool::new(false)));
        assert!(lifecycle.enter().is_err());
        assert!(lifecycle.accept().is_err());
        assert!(lifecycle.unknown().is_err());
        lifecycle.prepared().unwrap();
        assert!(lifecycle.accept().is_err());
        assert!(lifecycle.unknown().is_err());
        lifecycle.enter().unwrap();
        lifecycle.cancel().unwrap();
        lifecycle.unknown().unwrap();
        assert!(lifecycle.discard().is_err());
        assert!(lifecycle.enter().is_err());
        lifecycle.accept().unwrap();
        lifecycle.accept().unwrap();
        assert!(!lifecycle.cancel().unwrap());
    }
}
