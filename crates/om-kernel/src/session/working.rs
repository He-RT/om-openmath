//! Owned session work candidates preserve real math/source/results, never host IO or model jobs.
use super::Session;
use crate::{KernelConfig, protocol::CellStatus};
use std::sync::{Arc, atomic::AtomicBool};
/// Unaccepted independent session. Its private evaluator is writable; stored Explore forks stay readonly.
pub struct WorkingSession {
    candidate: Session,
}
impl WorkingSession {
    /// Execute against the candidate; none of its definition/result mutations affect the parent.
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.candidate
    }
    /// Consume the candidate after host source/epoch/cancel and durable acceptance checks.
    /// This move itself certifies neither document authority nor persisted checkpoint state.
    pub fn into_session(self) -> Session {
        self.candidate
    }
}
impl Session {
    /// Fork the actual idle working state with a fresh operation cancel token and no IO/AI owner.
    /// Definitions are cloned, never reexecuted from notebook text; record provenance is retained.
    pub fn fork_working_session(
        &self,
        cancel: Arc<AtomicBool>,
    ) -> Result<WorkingSession, om_eval::EvalError> {
        if Arc::ptr_eq(&cancel, &self.interrupt) {
            return Err(om_eval::EvalError::Other(
                "候选取消标志必须独立于主会话".into(),
            ));
        }
        if self
            .notebook
            .cells
            .iter()
            .any(|cell| cell.status == CellStatus::Running)
        {
            return Err(om_eval::EvalError::Other("工作候选需要空闲文档状态".into()));
        }
        let evaluator = self.eval.fork_working_stage()?.into_evaluator();
        let mut config = KernelConfig {
            general: self.config.general.clone(),
            ..KernelConfig::default()
        };
        config.llm.enabled = false;
        config.llm.profiles.clear();
        config.llm.translate.clear();
        config.llm.explain.clear();
        config.llm.chat.clear();
        config.llm.complete.clear();
        config.llm.fix.clear();
        let mut candidate = Session::with_cancel_token(config, self.clock.clone(), cancel);
        candidate.eval = evaluator;
        candidate.notebook = self.notebook.clone();
        candidate.owners = self.owners.clone();
        candidate.output_serial = self.output_serial;
        candidate.host_platform = self.host_platform;
        candidate.system_language = self.system_language;
        Ok(WorkingSession { candidate })
    }
}
