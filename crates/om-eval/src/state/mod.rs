//! Owned staged mathematics. Durable acceptance and document identity are kernel/host duties.
use crate::{EvalError, Evaluator};
mod codec;
pub use codec::{EvalStateError, EvalStateLimits};

/// An unaccepted writable evaluator. Dropping it discards definitions/history/random together.
/// It is not a checkpoint codec and cannot turn a readonly projection into a writable owner.
pub struct WorkingEvaluator {
    evaluator: Evaluator,
}
impl WorkingEvaluator {
    /// Work in the candidate, never in the active evaluator borrowed to create it.
    pub fn evaluator_mut(&mut self) -> &mut Evaluator {
        &mut self.evaluator
    }
    /// Consume the owned candidate after the caller's real document/IO acceptance checks.
    /// This function itself grants no document permission and does not claim persistence.
    pub fn into_evaluator(self) -> Evaluator {
        self.evaluator
    }
}
impl Evaluator {
    /// Clone persistent mathematical state at an idle writable boundary, without reevaluating
    /// definitions or changing Out/history/random. In-flight lexical scopes cannot be captured.
    pub fn fork_working_stage(&self) -> Result<WorkingEvaluator, EvalError> {
        if self.readonly || self.evaluating != 0 || self.depth != 0 || !self.scopes.is_empty() {
            return Err(EvalError::Other(
                "工作状态需要空闲的可写owner，不能提升只读快照".into(),
            ));
        }
        let mut evaluator = Evaluator::new();
        evaluator.defs = self.defs.clone();
        evaluator.builtins = self.builtins;
        evaluator.history = self.history.clone();
        evaluator.settings = self.settings.clone();
        evaluator.random = self.random.clone();
        Ok(WorkingEvaluator { evaluator })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lexical_frames_and_evaluation_depth_are_not_persistent_working_boundaries() {
        let mut owner = Evaluator::new();
        owner.evaluating = 1;
        assert!(owner.fork_working_stage().is_err());
        owner.evaluating = 0;
        owner.depth = 1;
        assert!(owner.fork_working_stage().is_err());
        owner.depth = 0;
        owner.scopes.push(Default::default());
        assert!(owner.fork_working_stage().is_err());
        owner.scopes.clear();
        assert!(owner.fork_working_stage().is_ok());
    }
    #[test]
    fn real_user_holding_attributes_are_cloned_and_stage_changes_remain_local() {
        let mut owner = Evaluator::new();
        let head = om_core::Symbol::intern("HeldUser");
        owner.defs.attrs.insert(head, crate::Attributes::HOLD_FIRST);
        let mut stage = owner.fork_working_stage().unwrap();
        let input = om_parse::parse_expr("HeldUser[a=5]", om_parse::Dialect::Wolfram).unwrap();
        stage
            .evaluator_mut()
            .evaluate(&input, &om_core::Interrupt::default())
            .unwrap();
        assert!(
            stage
                .evaluator_mut()
                .defs
                .own_value(om_core::Symbol::intern("a"))
                .is_none()
        );
        stage.evaluator_mut().defs.attrs.remove(&head);
        stage
            .evaluator_mut()
            .evaluate(&input, &om_core::Interrupt::default())
            .unwrap();
        assert_eq!(
            stage
                .evaluator_mut()
                .defs
                .own_value(om_core::Symbol::intern("a")),
            Some(&om_core::Expr::int(5))
        );
        assert_eq!(
            owner.defs.attrs.get(&head),
            Some(&crate::Attributes::HOLD_FIRST)
        );
        assert!(owner.defs.own_value(om_core::Symbol::intern("a")).is_none());
    }
}
