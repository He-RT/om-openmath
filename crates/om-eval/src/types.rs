//! Evaluator interfaces and portable execution settings.

use crate::Evaluator;
use om_core::{Abort, Expr, Interrupt, Symbol};

/// Evaluation limits and whether solvers should record derivations.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EvalSettings {
    /// Maximum tail rewrites for one evaluated expression.
    pub iteration_limit: u32,
    /// Maximum logical depth of nested evaluation and own-value references.
    pub recursion_limit: u32,
    /// Request structured steps from Solve-class builtins.
    pub record_steps: bool,
}
impl Default for EvalSettings {
    fn default() -> Self {
        Self {
            iteration_limit: 4096,
            recursion_limit: 1024,
            record_steps: true,
        }
    }
}
/// Holding, threading and protection flags, without a bitflags dependency.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attributes(u16);
impl Attributes {
    /// Hold every argument.
    pub const HOLD_ALL: Self = Self(1);
    /// Hold the first argument.
    pub const HOLD_FIRST: Self = Self(1 << 1);
    /// Hold all arguments except the first.
    pub const HOLD_REST: Self = Self(1 << 2);
    /// Thread over list arguments.
    pub const LISTABLE: Self = Self(1 << 3);
    /// Numeric arguments produce a numeric result when supported.
    pub const NUMERIC_FUNCTION: Self = Self(1 << 4);
    /// Reject definition changes.
    pub const PROTECTED: Self = Self(1 << 5);
    /// Arguments have canonical order.
    pub const ORDERLESS: Self = Self(1 << 6);
    /// Nested equal heads flatten.
    pub const FLAT: Self = Self(1 << 7);
    /// A sole argument can stand for the whole expression.
    pub const ONE_IDENTITY: Self = Self(1 << 8);
    /// Raw audited attribute flags for a readonly snapshot.
    pub fn bits(self) -> u16 {
        self.0
    }
    /// Decode only the current documented flag set.
    pub fn from_bits(bits: u16) -> Option<Self> {
        (bits & !255 == 0).then_some(Self(bits))
    }
    /// Whether all requested flags are present.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
impl std::ops::BitOr for Attributes {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
/// Accepted argument counts for a builtin.
#[derive(Clone, Copy, Debug)]
pub enum Arity {
    /// Exactly this many arguments.
    Exactly(u8),
    /// Inclusive minimum and maximum.
    Range(u8, u8),
    /// A minimum with no maximum.
    AtLeast(u8),
    /// Any count, including zero.
    Any,
}
impl Arity {
    pub(crate) fn accepts(self, count: usize) -> bool {
        match self {
            Self::Exactly(n) => count == n as usize,
            Self::Range(a, b) => (a as usize..=b as usize).contains(&count),
            Self::AtLeast(n) => count >= n as usize,
            Self::Any => true,
        }
    }
}
/// A builtin returns None to retain its rebuilt input.
pub type BuiltinFn = fn(&mut Evaluator, &[Expr], &Interrupt) -> Result<Option<Expr>, EvalError>;
/// An implementation, attributes, arity and its user-facing documentation.
pub struct BuiltinSpec {
    /// Interned builtin name.
    pub symbol: Symbol,
    /// Evaluation callback.
    pub f: BuiltinFn,
    /// Evaluation and definition flags.
    pub attrs: Attributes,
    /// Valid argument counts.
    pub arity: Arity,
    /// The same entry used for help and completion.
    pub doc: DocEntry,
}
/// Bilingual help and source syntax for one implemented builtin.
#[derive(serde::Serialize, Clone)]
pub struct DocEntry {
    /// Wolfram name.
    pub name: &'static str,
    /// Modern invocation syntax.
    pub modern: &'static str,
    /// Wolfram invocation syntax.
    pub wolfram: &'static str,
    /// Chinese help summary.
    pub summary_zh: &'static str,
    /// English help summary.
    pub summary_en: &'static str,
    /// Small executable examples.
    pub examples: &'static [&'static str],
    /// Help category.
    pub category: &'static str,
}
/// Evaluation failure, with portable cancellation preserved as a typed cause.
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    /// Computation was cancelled or exhausted its host budget.
    #[error(transparent)]
    Abort(#[from] Abort),
    /// Nested evaluation exceeded its logical depth limit.
    #[error("recursion limit {0} exceeded")]
    Recursion(u32),
    /// An execution-mode or other evaluation error.
    #[error("{0}")]
    Other(String),
}
