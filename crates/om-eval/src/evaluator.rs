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
    /// Evaluation limits and solver preferences.
    pub settings: EvalSettings,
    pub(crate) depth: u32,
    pub(crate) readonly: bool,
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
            settings: EvalSettings::default(),
            depth: 0,
            readonly: false,
        }
    }
    /// Evaluate without recording history. All Result paths restore logical depth.
    pub fn evaluate(&mut self, e: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError> {
        let previous = self.depth;
        let result = self.run_frames(e.clone(), ctx);
        self.depth = previous;
        result
    }
    /// Record a successful statement once, including its original input tree.
    pub fn evaluate_statement(&mut self, e: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError> {
        let out = self.evaluate(e, ctx)?;
        self.history.push((e.clone(), out.clone()));
        Ok(out)
    }
    /// Help for an implemented builtin.
    pub fn doc(sym: Symbol) -> Option<&'static DocEntry> {
        crate::builtins::table().get(sym).map(|spec| &spec.doc)
    }
    /// Implemented builtin help in deterministic name order.
    pub fn all_docs() -> impl Iterator<Item = &'static DocEntry> {
        crate::builtins::table().docs()
    }
    /// Snapshot the session for tools that may evaluate but cannot change definitions.
    pub fn fork_readonly(&self) -> Evaluator {
        Self {
            defs: self.defs.clone(),
            builtins: self.builtins,
            messages: Messages::default(),
            history: self.history.clone(),
            settings: self.settings.clone(),
            depth: 0,
            readonly: true,
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
