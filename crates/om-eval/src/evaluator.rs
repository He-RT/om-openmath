//! Evaluation order, stable rewrites and logical resource limits.

use crate::{Attributes as A, BuiltinTable, Definitions, DocEntry, EvalError, EvalSettings};
use om_core::{Expr, Interrupt, Message, Messages, MsgLevel, Symbol};

/// Stateful expression evaluator shared by native and WASM kernels.
pub struct Evaluator {
    /// User definitions for this session.
    pub defs: Definitions,
    pub(crate) builtins: &'static BuiltinTable,
    /// Messages in emission order.
    pub messages: Messages,
    /// Successful statement inputs and outputs.
    pub history: Vec<(Expr, Expr)>,
    /// Most recent solver steps, taken by the kernel before the next statement.
    pub last_steps: Option<om_solve::Steps>,
    /// Evaluation limits and solver preferences.
    pub settings: EvalSettings,
    pub(crate) depth: u32,
    pub(crate) readonly: bool,
    pub(crate) scopes: Vec<std::collections::BTreeMap<Symbol, Option<Expr>>>,
    pub(crate) last_solver_result: Option<crate::SolverResult>,
    pub(crate) evaluating: u32,
    pub(crate) random: om_num::rng::SplitMix64,
}
impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}
impl Evaluator {
    /// Start a new independent definition and history scope.
    pub fn new() -> Self {
        Self {
            defs: Definitions::default(),
            builtins: crate::builtins::table(),
            messages: Messages::default(),
            history: vec![],
            last_steps: None,
            settings: EvalSettings::default(),
            depth: 0,
            readonly: false,
            scopes: vec![],
            last_solver_result: None,
            evaluating: 0,
            random: om_num::rng::SplitMix64::new(0),
        }
    }
    /// Evaluate without recording history. All Result paths restore logical depth.
    pub fn evaluate(&mut self, e: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError> {
        let evaluating = self.evaluating;
        if evaluating == 0 {
            self.last_solver_result = None;
        }
        self.evaluating += 1;
        let previous = self.depth;
        let scopes = self.scopes.len();
        let result = self.run_frames(e.clone(), ctx);
        self.depth = previous;
        self.scopes.truncate(scopes);
        self.evaluating = evaluating;
        if evaluating == 0
            && self
                .last_solver_result
                .as_ref()
                .is_some_and(|r| result.as_ref().ok() != Some(&r.value))
        {
            self.last_solver_result = None;
        }
        result
    }
    /// Record a successful statement once, including its original input tree.
    pub fn evaluate_statement(&mut self, e: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError> {
        self.last_steps = None;
        let out = self.evaluate(e, ctx)?;
        self.history.push((e.clone(), out.clone()));
        Ok(out)
    }
    /// Take actual evidence for the most recent returned tail solver result.
    /// Absent for cached literals, unrelated wrappers, failures and discarded calls.
    pub fn take_solver_result(&mut self) -> Option<crate::SolverResult> {
        self.last_solver_result.take()
    }
    /// Expand numeric source in a readonly local scope without cancelling raw poles.
    /// None masks an axis's global value; Some supplies an actual local parameter value.
    pub fn prepare_numeric(
        &self,
        source: &Expr,
        locals: &[(Symbol, Option<Expr>)],
        ctx: &Interrupt,
    ) -> Result<Expr, EvalError> {
        let mut fork = self.fork_readonly();
        fork.scopes.push(locals.iter().cloned().collect());
        crate::solver::prepare_numeric(&mut fork, source, ctx)
    }
    /// Help for an implemented builtin.
    pub fn doc(sym: Symbol) -> Option<&'static DocEntry> {
        crate::builtins::table().get(sym).map(|spec| &spec.doc)
    }
    /// Implemented builtin help in deterministic name order.
    pub fn all_docs() -> impl Iterator<Item = &'static DocEntry> {
        crate::builtins::table().docs()
    }
    /// Actual immutable callback specifications, for capability and contract auditing.
    pub fn all_specs() -> impl Iterator<Item = &'static crate::BuiltinSpec> {
        crate::builtins::table().specs()
    }
    /// Snapshot the session for tools that may evaluate but cannot change definitions.
    pub fn fork_readonly(&self) -> Evaluator {
        Self {
            defs: self.defs.clone(),
            builtins: self.builtins,
            messages: Messages::default(),
            history: self.history.clone(),
            last_steps: None,
            settings: self.settings.clone(),
            depth: 0,
            readonly: true,
            scopes: self.scopes.clone(),
            last_solver_result: None,
            evaluating: 0,
            random: self.random.clone(),
        }
    }
    pub(crate) fn attributes(&self, symbol: Symbol) -> A {
        let registered = self
            .builtins
            .get(symbol)
            .map_or(A::default(), |spec| spec.attrs);
        let protected = if om_core::builtins::names().contains(&symbol.name()) {
            A::PROTECTED
        } else {
            A::default()
        };
        let intrinsic = match symbol {
            om_core::BUILTIN::SERIES_DATA => A::HOLD_ALL,
            om_core::BUILTIN::PATTERN
            | om_core::BUILTIN::BLANK
            | om_core::BUILTIN::BLANK_SEQUENCE
            | om_core::BUILTIN::BLANK_NULL_SEQUENCE
            | om_core::BUILTIN::CONDITION => A::HOLD_ALL,
            om_core::BUILTIN::PLUS | om_core::BUILTIN::TIMES => {
                A::LISTABLE | A::NUMERIC_FUNCTION | A::ORDERLESS | A::FLAT | A::ONE_IDENTITY
            }
            om_core::BUILTIN::POWER | om_core::BUILTIN::SQRT | om_core::BUILTIN::EXP => {
                A::LISTABLE | A::NUMERIC_FUNCTION
            }
            _ => A::default(),
        };
        registered
            | protected
            | intrinsic
            | self.defs.attrs.get(&symbol).copied().unwrap_or_default()
    }
    pub(crate) fn message(&mut self, symbol: &str, tag: &str, text: String, level: MsgLevel) {
        self.messages.push(Message {
            symbol: symbol.into(),
            tag: tag.into(),
            text,
            level,
        });
    }
}

impl Evaluator {
    pub(crate) fn own(&self, symbol: Symbol) -> Option<Expr> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(&symbol) {
                return value.clone();
            }
        }
        self.defs.own.get(&symbol).cloned()
    }
    pub(crate) fn set_own(&mut self, symbol: Symbol, value: Option<Expr>) {
        if let Some(scope) = self
            .scopes
            .iter_mut()
            .rev()
            .find(|scope| scope.contains_key(&symbol))
        {
            scope.insert(symbol, value);
        } else if let Some(value) = value {
            self.defs.changed.insert(symbol);
            self.defs.own.insert(symbol, value);
        } else {
            self.defs.changed.insert(symbol);
            self.defs.own.remove(&symbol);
        }
    }
}
