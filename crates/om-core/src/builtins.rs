//! Built-in names in contract order; append new entries to preserve all existing IDs.

/// Built-in names in the stable contract order, for syntax and completion lookup.
pub fn names() -> &'static [&'static str] {
    NAMES
}

/// Pre-.3 reserved lookup for declarations; new aliases cannot rewrite old user definitions.
pub fn legacy_names() -> &'static [&'static str] {
    &NAMES[..BuiltinId::LENGTH as usize + 1]
}

macro_rules! define_builtins {
    ($($ident:ident=$name:literal),* $(,)?) => {
        // The private ordinal enum mirrors the contract's public constant identifiers.
        #[allow(non_camel_case_types,clippy::upper_case_acronyms)]
        enum BuiltinId {$($ident),*}
        /// Fixed symbols shared by constructors, parsers and evaluators.
        #[allow(non_snake_case)]
        pub mod BUILTIN {
            use super::BuiltinId;
            use crate::Symbol;
            $(#[doc=$name] pub const $ident:Symbol=Symbol::builtin(BuiltinId::$ident as u32);)*
        }
        pub(crate) const NAMES:&[&str]=&[$($name),*];
    }
}

define_builtins! {
    LIST = "List", PLUS = "Plus", TIMES = "Times", POWER = "Power", RULE = "Rule", RULE_DELAYED = "RuleDelayed",
    EQUAL = "Equal", UNEQUAL = "Unequal", LESS = "Less", LESS_EQUAL = "LessEqual", GREATER = "Greater",
    GREATER_EQUAL = "GreaterEqual", INEQUALITY = "Inequality", AND = "And", OR = "Or", NOT = "Not",
    TRUE = "True", FALSE = "False", NULL = "Null", PI = "Pi", E = "E", I = "I", INFINITY = "Infinity",
    COMPLEX_INFINITY = "ComplexInfinity", DIRECTED_INFINITY = "DirectedInfinity", INDETERMINATE = "Indeterminate",
    SIN = "Sin", COS = "Cos", TAN = "Tan", COT = "Cot", SEC = "Sec", CSC = "Csc",
    ARCSIN = "ArcSin", ARCCOS = "ArcCos", ARCTAN = "ArcTan", ARCCOT = "ArcCot", ARCSEC = "ArcSec", ARCCSC = "ArcCsc",
    SINH = "Sinh", COSH = "Cosh", TANH = "Tanh", COTH = "Coth", SECH = "Sech", CSCH = "Csch",
    ARCSINH = "ArcSinh", ARCCOSH = "ArcCosh", ARCTANH = "ArcTanh",
    EXP = "Exp", LOG = "Log", SQRT = "Sqrt", ABS = "Abs", SIGN = "Sign", RE = "Re", IM = "Im", ARG = "Arg",
    CONJUGATE = "Conjugate", FLOOR = "Floor", CEILING = "Ceiling", ROUND = "Round", PRODUCT_LOG = "ProductLog",
    FACTORIAL = "Factorial", BINOMIAL = "Binomial", MOD = "Mod", GCD = "GCD", LCM = "LCM",
    ROOT = "Root", FUNCTION = "Function", SLOT = "Slot", C = "C", CONDITIONAL_EXPRESSION = "ConditionalExpression",
    ELEMENT = "Element", NOT_ELEMENT = "NotElement", REALS = "Reals", INTEGERS = "Integers", COMPLEXES = "Complexes",
    RATIONALS = "Rationals", ALGEBRAICS = "Algebraics", PRIMES = "Primes", BOOLEANS = "Booleans",
    INTEGER = "Integer", RATIONAL = "Rational", REAL = "Real", COMPLEX = "Complex", SYMBOL = "Symbol", STRING = "String",
    SOLVE = "Solve", NSOLVE = "NSolve", FIND_ROOT = "FindRoot", REDUCE = "Reduce", ELIMINATE = "Eliminate",
    SOLVE_VALUES = "SolveValues", N = "N", SET = "Set", SET_DELAYED = "SetDelayed", CLEAR = "Clear",
    REPLACE_ALL = "ReplaceAll", COMPOUND_EXPRESSION = "CompoundExpression", PART = "Part", SPAN = "Span",
    PATTERN = "Pattern", BLANK = "Blank", BLANK_SEQUENCE = "BlankSequence", BLANK_NULL_SEQUENCE = "BlankNullSequence",
    CONDITION = "Condition", HOLD = "Hold", HOLD_FORM = "HoldForm", SEQUENCE = "Sequence", MESSAGE_NAME = "MessageName",
    EXPAND = "Expand", FACTOR = "Factor", TOGETHER = "Together", CANCEL = "Cancel", APART = "Apart", SIMPLIFY = "Simplify",
    FULL_SIMPLIFY = "FullSimplify", COLLECT = "Collect", D = "D", PLOT = "Plot", CONTOUR_PLOT = "ContourPlot",
    GRAPHICS = "Graphics", OUT = "Out", ASSUMPTIONS = "Assumptions", AUTOMATIC = "Automatic", ALL = "All", NONE = "None",
    METHOD = "Method", CUBICS = "Cubics", QUARTICS = "Quartics", VERIFY_SOLUTIONS = "VerifySolutions",
    MAX_EXTRA_CONDITIONS = "MaxExtraConditions", GENERATED_PARAMETERS = "GeneratedParameters",
    WORKING_PRECISION = "WorkingPrecision", MAX_ITERATIONS = "MaxIterations", ACCURACY_GOAL = "AccuracyGoal",
    PRECISION_GOAL = "PrecisionGoal", TABLE = "Table", RANGE = "Range", MAP = "Map", APPLY = "Apply", LENGTH = "Length",
    RECORD = "Record", DOT = "Dot",
}
