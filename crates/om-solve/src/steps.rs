//! Structured events emitted by the executing algorithms; no mathematical reconstruction.
mod recorder;
mod registry;
use crate::Domain;
use om_core::{Expr, Message, Symbol};
use om_poly::MonoOrder;
use om_simplify::zero::Tri;
pub use recorder::{NoSteps, StepRecorder, StepSink};
pub use registry::rule_ids;

/// Hierarchical derivation in the order events were emitted.
#[derive(Clone, Debug, Default)]
pub struct Steps {
    /// Top-level computational events.
    pub root: Vec<Step>,
}
/// One actual computation, with stable references and expression snapshots.
#[derive(Clone, Debug)]
pub struct Step {
    /// Hierarchical identifier assigned by StepRecorder, e.g. S1.2.
    pub id: String,
    /// Stable language-independent rule identifier.
    pub rule_id: &'static str,
    /// Structured operation and its exact data.
    pub kind: StepKind,
    /// Expressions before the operation.
    pub before: Vec<Expr>,
    /// Expressions after the operation.
    pub after: Vec<Expr>,
    /// Default presentation detail level.
    pub level: Level,
    /// Computational events performed within this event.
    pub children: Vec<Step>,
}
impl Step {
    /// Construct an unnumbered event; the recorder supplies its hierarchical ID.
    pub fn new(kind: StepKind, before: Vec<Expr>, after: Vec<Expr>, level: Level) -> Self {
        Self {
            id: String::new(),
            rule_id: kind.rule_id(),
            kind,
            before,
            after,
            level,
            children: vec![],
        }
    }
}
/// Presentation level; UI initially expands major events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Main derivation event.
    Major,
    /// Supporting arithmetic or verification detail.
    Minor,
}
/// Closed family of elementary polynomial solution formulas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formula {
    /// Linear equation formula.
    Linear,
    /// Quadratic equation formula.
    Quadratic,
    /// Cardano cubic formula.
    Cardano,
    /// Ferrari quartic formula.
    Ferrari,
    /// Binomial roots.
    Binomial,
    /// Reciprocal/palindromic substitution.
    Palindromic,
}
/// Certified sign used for discriminants and sign charts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    /// Strictly negative.
    Negative,
    /// Exactly zero.
    Zero,
    /// Strictly positive.
    Positive,
}
/// Reason an original-expression condition must be retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExclReason {
    /// An original denominator cannot be zero.
    ZeroDenominator,
    /// A function must have a defined value.
    UndefinedFunction,
    /// An operation must respect its original branch.
    BranchRestriction,
}
/// Exact elementary row operation; indices are zero-based.
#[derive(Clone, Debug)]
pub enum RowOp {
    /// Exchange rows a and b.
    Swap {
        /// First row index.
        a: usize,
        /// Second row index.
        b: usize,
    },
    /// Multiply row by factor.
    Scale {
        /// Row to be scaled.
        row: usize,
        /// Exact multiplier used by the operation.
        factor: Expr,
    },
    /// Replace target by target + factor*source.
    Add {
        /// Destination row index.
        target: usize,
        /// Source row index.
        source: usize,
        /// Exact multiplier used by the operation.
        factor: Expr,
    },
}
/// Computation data from PLAN §8.10; localization is performed by the consumer.
#[derive(Clone, Debug)]
pub enum StepKind {
    /// Normalize original equations.
    Normalize,
    /// Preserve an original-expression domain restriction.
    RecordExclusion {
        /// Boolean condition retained by the computation.
        cond: Expr,
        /// Structured reason for the restriction.
        reason: ExclReason,
    },
    /// Record a condition for a generic parameter solution.
    GenericAssumption {
        /// Boolean condition retained by the computation.
        cond: Expr,
    },
    /// Clear polynomial denominators.
    ClearDenominators {
        /// Exact multiplier used by the operation.
        factor: Expr,
    },
    /// Expand arithmetic.
    Expand,
    /// Factor exactly with multiplicities.
    Factor {
        /// Exact factors paired with their multiplicities.
        factors: Vec<(Expr, u32)>,
    },
    /// Split a zero product into components.
    ZeroProduct,
    /// Split square-free parts with multiplicities.
    SquareFree {
        /// Square-free parts paired with their multiplicities.
        parts: Vec<(Expr, u32)>,
    },
    /// Introduce an algebraic/kernel substitution.
    Substitute {
        /// New substitution variable.
        new_var: Expr,
        /// Definition in terms of original variables.
        def: Expr,
    },
    /// Recover the original variable from a substitution.
    BackSubstitute {
        /// Variable involved in the operation.
        var: Expr,
        /// Computed exact value.
        value: Expr,
    },
    /// Apply a closed-form polynomial formula with exact bindings.
    ApplyFormula {
        /// Formula family actually applied.
        formula: Formula,
        /// Named exact coefficients and other formula data.
        bindings: Vec<(String, Expr)>,
        /// Computed roots or formula results.
        results: Vec<Expr>,
    },
    /// Compute a discriminant and optional certified sign.
    Discriminant {
        /// Computed exact value.
        value: Expr,
        /// Certified sign, when known.
        sign: Option<Sign>,
    },
    /// Isolate a term before inversion.
    IsolateTerm {
        /// Term isolated before inversion.
        term: Expr,
    },
    /// Raise both sides to an integer power.
    RaiseToPower {
        /// Positive integer exponent.
        n: u32,
    },
    /// Eliminate a variable using a resultant.
    Resultant {
        /// Variable involved in the operation.
        var: Expr,
        /// Resulting eliminant polynomial.
        result: Expr,
    },
    /// Invert an elementary kernel with explicit branches and parameters.
    InvertFunction {
        /// Head of the function being inverted.
        func: Symbol,
        /// Alternative inverse expressions.
        branches: Vec<Expr>,
        /// Generated parameter expressions.
        constants: Vec<Expr>,
    },
    /// Apply a row operation and retain the resulting matrix.
    RowReduce {
        /// Exact row operation performed.
        op: RowOp,
        /// Matrix after applying the row operation.
        matrix: Vec<Vec<Expr>>,
    },
    /// Compute a Groebner basis in the specified order.
    Groebner {
        /// Monomial order used by the computation.
        order: MonoOrder,
        /// Resulting basis polynomials.
        basis: Vec<Expr>,
    },
    /// Obtain a univariate eliminant.
    Eliminant {
        /// Variable involved in the operation.
        var: Expr,
        /// Polynomial whose roots or eliminant are represented.
        poly: Expr,
    },
    /// Solve an individual factor component.
    SplitComponent {
        /// Exact multiplier used by the operation.
        factor: Expr,
    },
    /// Represent roots exactly with certified real-root count.
    RootObjects {
        /// Polynomial whose roots or eliminant are represented.
        poly: Expr,
        /// Number of distinct real roots.
        real_count: u32,
    },
    /// Check an original equation at a candidate.
    Verify {
        /// Ordered candidate assignment rules.
        candidate: Vec<(Expr, Expr)>,
        /// Zero decision for the original residual.
        outcome: Tri,
        /// Original-equation residual, when retained.
        residual: Option<Expr>,
    },
    /// Discard a candidate; why is a language-independent reason code.
    DropExtraneous {
        /// Ordered candidate assignment rules.
        candidate: Vec<(Expr, Expr)>,
        /// Language-independent rejection reason code.
        why: String,
    },
    /// Restrict candidates to the requested domain.
    DomainFilter {
        /// Requested solution domain.
        domain: Domain,
        /// Number of retained candidates.
        kept: usize,
        /// Number of rejected candidates.
        dropped: usize,
    },
    /// Construct a rational/polynomial sign chart.
    SignChart {
        /// Ordered real critical points.
        points: Vec<Expr>,
        /// Certified signs for the chart intervals.
        signs: Vec<Sign>,
    },
    /// Group events for a computational branch; label is a stable label/key.
    Branch {
        /// Stable branch label or localization key.
        label: String,
    },
    /// Include an existing diagnostic.
    Note {
        /// Diagnostic emitted by the calculation.
        msg: Message,
    },
}
