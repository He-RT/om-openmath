//! Shared immutable expression trees with cached structural hashes.

use crate::{BUILTIN as B, Symbol};
use om_num::{Integer, Number, Rational, Real};
use rustc_hash::FxHasher;
use smallvec::SmallVec;
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    sync::Arc,
};

/// A cheap-to-clone immutable expression.
#[derive(Clone)]
pub struct Expr(Arc<ExprNode>);

/// Expression data and its hash, computed together at construction.
pub struct ExprNode {
    /// The atom or compound expression.
    pub kind: ExprKind,
    /// Cached structural hash; equal expressions always share this value.
    pub hash: u64,
}

/// The four structural expression categories.
pub enum ExprKind {
    /// A finite normalized number.
    Number(Number),
    /// An interned symbol.
    Symbol(Symbol),
    /// A string literal.
    String(Box<str>),
    /// A compound expression with an arbitrary expression as its head.
    Normal(Normal),
}

/// A head followed by ordered arguments.
pub struct Normal {
    /// The function or operator expression.
    pub head: Expr,
    /// Arguments, stored inline for the common small-arity case.
    pub args: SmallVec<[Expr; 3]>,
}

impl Expr {
    // Used solely for in-memory encode deduplication. Never emitted or decoded as a pointer.
    pub(crate) fn allocation_key(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
    fn new(kind: ExprKind) -> Self {
        let mut h = FxHasher::default();
        std::mem::discriminant(&kind).hash(&mut h);
        match &kind {
            ExprKind::Number(n) => hash_number(n, &mut h),
            ExprKind::Symbol(s) => s.hash(&mut h),
            ExprKind::String(s) => s.hash(&mut h),
            ExprKind::Normal(n) => {
                n.head.hash(&mut h);
                n.args.hash(&mut h);
            }
        }
        Self(Arc::new(ExprNode {
            kind,
            hash: h.finish(),
        }))
    }
    /// Construct a small exact integer.
    pub fn int(i: i64) -> Expr {
        Self::integer(i.into())
    }
    /// Construct an arbitrary-size exact integer.
    pub fn integer(i: Integer) -> Expr {
        Self::number(Number::Integer(i))
    }
    /// Construct and reduce a small rational. Panics for a zero denominator.
    pub fn rational(n: i64, d: i64) -> Expr {
        assert_ne!(d, 0, "rational denominator must be nonzero");
        Self::number(Number::Rational(Rational::from(n) / Rational::from(d)))
    }
    /// Normalize a finite number before storing it as an atom.
    pub fn number(n: Number) -> Expr {
        Self::new(ExprKind::Number(n.normalize()))
    }
    /// Construct a finite machine real. Panics for infinity or NaN.
    pub fn real(f: f64) -> Expr {
        Self::number(Number::Real(Real::Machine(f)))
    }
    /// Construct an atom from an interned symbol.
    pub fn sym(s: Symbol) -> Expr {
        Self::new(ExprKind::Symbol(s))
    }
    /// Intern a name and construct its symbol atom.
    pub fn symbol(name: &str) -> Expr {
        Self::sym(Symbol::intern(name))
    }
    /// Construct a string literal.
    pub fn string(s: &str) -> Expr {
        Self::new(ExprKind::String(s.into()))
    }
    /// Construct a raw compound expression, preserving order and nesting.
    pub fn normal(head: Expr, args: impl IntoIterator<Item = Expr>) -> Expr {
        Self::new(ExprKind::Normal(Normal {
            head,
            args: args.into_iter().collect(),
        }))
    }
    /// Construct a raw call with a symbol head.
    pub fn call(head: Symbol, args: impl IntoIterator<Item = Expr>) -> Expr {
        Self::normal(Self::sym(head), args)
    }
    /// Borrow the structural category and its data.
    pub fn kind(&self) -> &ExprKind {
        &self.0.kind
    }
    /// Borrow the numeric atom, if present.
    pub fn as_number(&self) -> Option<&Number> {
        if let ExprKind::Number(n) = self.kind() {
            Some(n)
        } else {
            None
        }
    }
    /// Return the interned symbol, if this is a symbol atom.
    pub fn as_symbol(&self) -> Option<Symbol> {
        if let ExprKind::Symbol(s) = self.kind() {
            Some(*s)
        } else {
            None
        }
    }
    /// Return a compound head or an atom's built-in type head.
    pub fn head(&self) -> Expr {
        Self::sym(match self.kind() {
            ExprKind::Normal(n) => return n.head.clone(),
            ExprKind::Symbol(_) => B::SYMBOL,
            ExprKind::String(_) => B::STRING,
            ExprKind::Number(Number::Integer(_)) => B::INTEGER,
            ExprKind::Number(Number::Rational(_)) => B::RATIONAL,
            ExprKind::Number(Number::Real(_)) => B::REAL,
            ExprKind::Number(Number::Complex(_)) => B::COMPLEX,
        })
    }
    /// Return a symbol head only for compound expressions.
    pub fn head_symbol(&self) -> Option<Symbol> {
        if let ExprKind::Normal(n) = self.kind() {
            n.head.as_symbol()
        } else {
            None
        }
    }
    /// Borrow the arguments; atoms have no arguments.
    pub fn args(&self) -> &[Expr] {
        if let ExprKind::Normal(n) = self.kind() {
            &n.args
        } else {
            &[]
        }
    }
    /// Whether a compound expression has the given symbol head.
    pub fn is_head(&self, s: Symbol) -> bool {
        self.head_symbol() == Some(s)
    }
    /// Whether this is a literal numeric zero.
    pub fn is_zero(&self) -> bool {
        self.as_number().is_some_and(Number::is_zero)
    }
    /// Whether this is a literal numeric one.
    pub fn is_one(&self) -> bool {
        self.as_number().is_some_and(Number::is_one)
    }
    /// Whether no subtree, including explicit heads, structurally equals `x`.
    pub fn free_of(&self, x: &Expr) -> bool {
        let mut stack = vec![self];
        while let Some(e) = stack.pop() {
            if e == x {
                return false;
            }
            if let ExprKind::Normal(n) = e.kind() {
                stack.push(&n.head);
                stack.extend(n.args.iter());
            }
        }
        true
    }
    /// Symbols in argument positions, excluding built-in constants and function names.
    /// The set uses ID order for storage; consumers displaying it must sort by name.
    pub fn free_symbols(&self) -> BTreeSet<Symbol> {
        let mut symbols = BTreeSet::new();
        let mut stack = vec![self];
        while let Some(e) = stack.pop() {
            match e.kind() {
                ExprKind::Symbol(s) if !is_constant(*s) => {
                    symbols.insert(*s);
                }
                ExprKind::Normal(n) => {
                    // In f[x][y], x is a variable even though f is a function name.
                    if matches!(n.head.kind(), ExprKind::Normal(_)) {
                        stack.push(&n.head);
                    }
                    stack.extend(n.args.iter());
                }
                _ => {}
            }
        }
        symbols
    }
    /// Apply the first matching structural rule to each subtree simultaneously.
    /// Replacements are not revisited; the final result is canonicalized.
    pub fn replace_all(&self, rules: &[(Expr, Expr)]) -> Expr {
        crate::canonicalize(&self.replace_raw(rules))
    }
    fn replace_raw(&self, rules: &[(Expr, Expr)]) -> Expr {
        if let Some((_, value)) = rules.iter().find(|(key, _)| key == self) {
            return value.clone();
        }
        if let ExprKind::Normal(n) = self.kind() {
            let head = n.head.replace_raw(rules);
            let args: SmallVec<[Expr; 3]> = n.args.iter().map(|e| e.replace_raw(rules)).collect();
            if head != n.head || args != n.args {
                return Self::normal(head, args);
            }
        }
        self.clone()
    }
    /// Map immediate arguments in order. Atoms are returned unchanged.
    /// Canonicalize the rebuilt compound tree, including mapped results.
    pub fn map_args(&self, f: impl FnMut(&Expr) -> Expr) -> Expr {
        if let ExprKind::Normal(n) = self.kind() {
            crate::canonicalize(&Self::normal(n.head.clone(), n.args.iter().map(f)))
        } else {
            self.clone()
        }
    }
    /// Count atomic leaves, including the leaves of compound heads.
    pub fn leaf_count(&self) -> usize {
        let mut count = 0;
        let mut stack = vec![self];
        while let Some(e) = stack.pop() {
            if let ExprKind::Normal(n) = e.kind() {
                stack.push(&n.head);
                stack.extend(n.args.iter());
            } else {
                count += 1;
            }
        }
        count
    }
}

fn is_constant(s: Symbol) -> bool {
    matches!(
        s,
        B::TRUE
            | B::FALSE
            | B::NULL
            | B::PI
            | B::E
            | B::I
            | B::INFINITY
            | B::COMPLEX_INFINITY
            | B::INDETERMINATE
            | B::REALS
            | B::INTEGERS
            | B::COMPLEXES
            | B::RATIONALS
            | B::ALGEBRAICS
            | B::PRIMES
            | B::BOOLEANS
            | B::AUTOMATIC
            | B::ALL
            | B::NONE
    )
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.0, &other.0) {
            return true;
        }
        if self.0.hash != other.0.hash {
            return false;
        }
        match (self.kind(), other.kind()) {
            (ExprKind::Number(a), ExprKind::Number(b)) => number_eq(a, b),
            (ExprKind::Symbol(a), ExprKind::Symbol(b)) => a == b,
            (ExprKind::String(a), ExprKind::String(b)) => a == b,
            (ExprKind::Normal(a), ExprKind::Normal(b)) => a.head == b.head && a.args == b.args,
            _ => false,
        }
    }
}
impl Eq for Expr {}
impl Hash for Expr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.hash);
    }
}

fn number_eq(a: &Number, b: &Number) -> bool {
    match (a, b) {
        (Number::Real(Real::Big(a)), Number::Real(Real::Big(b))) => {
            a == b && a.precision() == b.precision()
        }
        (Number::Complex(a), Number::Complex(b)) => {
            number_eq(&a.re, &b.re) && number_eq(&a.im, &b.im)
        }
        _ => a == b,
    }
}
fn hash_number(n: &Number, h: &mut FxHasher) {
    std::mem::discriminant(n).hash(h);
    match n {
        Number::Integer(i) => i.hash(h),
        Number::Rational(q) => q.hash(h),
        Number::Real(r) => {
            std::mem::discriminant(r).hash(h);
            match r {
                Real::Machine(x) => {
                    // IEEE signed zeros compare equally; their hashes must too.
                    (if *x == 0.0 { 0 } else { x.to_bits() }).hash(h);
                }
                Real::Big(x) => {
                    x.repr().significand().hash(h);
                    x.repr().exponent().hash(h);
                    x.precision().hash(h);
                }
            }
        }
        Number::Complex(c) => {
            hash_number(&c.re, h);
            hash_number(&c.im, h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_hashes_still_require_equal_structure() {
        let a = Expr(Arc::new(ExprNode {
            kind: ExprKind::Symbol(Symbol::intern("collision_a")),
            hash: 42,
        }));
        let b = Expr(Arc::new(ExprNode {
            kind: ExprKind::Symbol(Symbol::intern("collision_b")),
            hash: 42,
        }));
        assert_ne!(a, b);
        assert_eq!(a, a.clone());
    }
}

#[path = "expr_fmt.rs"]
mod expr_fmt;
