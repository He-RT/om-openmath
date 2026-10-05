//! Iterative lowering proves stack shape before any point is evaluated.
use super::instruction::{Instruction as I, binary, special, unary};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, Symbol};
use std::collections::BTreeSet;

/// Rejected numerical syntax, variables or compilation resources.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    /// Axes must be distinct user symbols, at most 64.
    #[error("numeric axes must be distinct user symbols (at most 64)")]
    Variables,
    /// Unsupported symbolic head, arity, unresolved symbol or dynamic Root.
    #[error("unsupported numeric expression: {0}")]
    Unsupported(String),
    /// A compilation contains more than 100,000 expression nodes.
    #[error("numeric compilation node limit exceeded")]
    Limit,
    /// The caller's budget, cancellation or clock rejected work.
    #[error(transparent)]
    Abort(#[from] om_core::Abort),
}
/// Immutable real stack program; no definitions or symbolic evaluation per point.
#[derive(Clone)]
pub struct CompiledFn {
    code: Vec<I>,
    variables: usize,
    stack: usize,
}
impl CompiledFn {
    /// Evaluate with temporary storage; invalid dimensions/non-real results return NaN.
    pub fn eval(&self, values: &[f64]) -> f64 {
        self.eval_with(values, &mut Vec::with_capacity(self.stack))
    }
    /// Evaluate using reusable stack storage, without allocating for each point.
    pub fn eval_with(&self, values: &[f64], work: &mut Vec<f64>) -> f64 {
        self.run(values, work, None).unwrap_or(f64::NAN)
    }
    /// Evaluate each instruction under the caller's actual budget, flag and deadline.
    pub fn eval_with_ctx(
        &self,
        values: &[f64],
        work: &mut Vec<f64>,
        ctx: &Interrupt,
    ) -> Result<f64, om_core::Abort> {
        self.run(values, work, Some(ctx))
    }
    fn run(
        &self,
        values: &[f64],
        work: &mut Vec<f64>,
        ctx: Option<&Interrupt>,
    ) -> Result<f64, om_core::Abort> {
        work.clear();
        let default_ctx = ctx.is_none().then(Interrupt::default);
        let special_ctx = ctx
            .or(default_ctx.as_ref())
            .expect("invariant: caller or local interrupt exists");
        if values.len() != self.variables || values.iter().any(|x| !x.is_finite()) {
            return Ok(f64::NAN);
        }
        for instruction in &self.code {
            if let Some(ctx) = ctx {
                ctx.tick()?;
            }
            let value = match *instruction {
                I::Constant(x) => x,
                I::Variable(i) => values[i],
                I::Add(n) | I::Mul(n) => {
                    let Some(start) = work.len().checked_sub(n) else {
                        return Ok(f64::NAN);
                    };
                    let v = if matches!(instruction, I::Add(_)) {
                        work[start..].iter().sum()
                    } else {
                        work[start..].iter().product()
                    };
                    work.truncate(start);
                    v
                }
                I::Pow => {
                    let b = work.pop().unwrap_or(f64::NAN);
                    let a = work.pop().unwrap_or(f64::NAN);
                    if a == 0.0 && b == 0.0 {
                        f64::NAN
                    } else {
                        a.powf(b)
                    }
                }
                I::Unary(f) => f(work.pop().unwrap_or(f64::NAN)),
                I::Binary(f) => {
                    let b = work.pop().unwrap_or(f64::NAN);
                    let a = work.pop().unwrap_or(f64::NAN);
                    f(a, b)
                }
                I::Special(f) => match f(work.pop().unwrap_or(f64::NAN), special_ctx) {
                    Ok(x) => x,
                    Err(om_analysis::Error::Abort(e)) => return Err(e),
                    Err(_) => f64::NAN,
                },
                I::SpecialBinary(f) => {
                    let b = work.pop().unwrap_or(f64::NAN);
                    let a = work.pop().unwrap_or(f64::NAN);
                    match f(a, b, special_ctx) {
                        Ok(x) => x,
                        Err(om_analysis::Error::Abort(e)) => return Err(e),
                        Err(_) => f64::NAN,
                    }
                }
            };
            if !value.is_finite() {
                work.clear();
                return Ok(f64::NAN);
            }
            work.push(value);
        }
        Ok(work.last().copied().unwrap_or(f64::NAN))
    }
}
/// Lower a numeric tree with the default portable step budget.
pub fn compile_f64(expr: &Expr, vars: &[Symbol]) -> Result<CompiledFn, CompileError> {
    compile_f64_with_ctx(expr, vars, &Interrupt::default())
}
/// Lower a numeric tree while respecting the caller's actual interruption scope.
pub fn compile_f64_with_ctx(
    expr: &Expr,
    vars: &[Symbol],
    ctx: &Interrupt,
) -> Result<CompiledFn, CompileError> {
    let mut seen = BTreeSet::new();
    if vars.len() > 64
        || vars.iter().any(|s| {
            s.name().is_empty()
                || om_core::builtins::names().contains(&s.name())
                || crate::Evaluator::doc(*s).is_some()
                || !seen.insert(*s)
        })
    {
        return Err(CompileError::Variables);
    }
    enum Work<'a> {
        Visit(&'a Expr),
        Emit(I, usize),
    }
    let mut work = vec![Work::Visit(expr)];
    let mut code = vec![];
    let (mut depth, mut stack, mut nodes) = (0usize, 0usize, 0usize);
    while let Some(task) = work.pop() {
        ctx.tick()?;
        match task {
            Work::Emit(i, n) => {
                depth = depth
                    .checked_sub(n)
                    .ok_or_else(|| CompileError::Unsupported("unbalanced stack".into()))?
                    + 1;
                stack = stack.max(depth);
                code.push(i);
            }
            Work::Visit(e) => {
                nodes += 1;
                if nodes > 100_000 {
                    return Err(CompileError::Limit);
                }
                let constant = if let Some(n) = e.as_number() {
                    Some(n.to_f64().unwrap_or(f64::NAN))
                } else if e.is_head(B::ROOT) {
                    Some(
                        om_simplify::numeval::approximate(e, om_num::Precision::Machine, ctx)?
                            .ok_or_else(|| {
                                CompileError::Unsupported("dynamic or invalid Root".into())
                            })?
                            .to_f64()
                            .unwrap_or(f64::NAN),
                    )
                } else if let Some(s) = e.as_symbol() {
                    if let Some(i) = vars.iter().position(|v| v == &s) {
                        work.push(Work::Emit(I::Variable(i), 0));
                        continue;
                    }
                    match s {
                        B::PI => Some(std::f64::consts::PI),
                        B::E => Some(std::f64::consts::E),
                        B::I | B::INFINITY | B::COMPLEX_INFINITY | B::INDETERMINATE => {
                            Some(f64::NAN)
                        }
                        _ => return Err(CompileError::Unsupported(s.name().into())),
                    }
                } else {
                    None
                };
                if let Some(value) = constant {
                    work.push(Work::Emit(I::Constant(value), 0));
                    continue;
                }
                let ExprKind::Normal(n) = e.kind() else {
                    return Err(CompileError::Unsupported("nonnumeric atom".into()));
                };
                let h = n
                    .head
                    .as_symbol()
                    .ok_or_else(|| CompileError::Unsupported("compound function head".into()))?;
                let count = n.args.len();
                let i = match h {
                    B::PLUS => I::Add(count),
                    B::TIMES => I::Mul(count),
                    B::POWER if count == 2 => I::Pow,
                    _ if special(h.name()).is_some_and(|i| {
                        matches!((i, count), (I::Special(_), 1) | (I::SpecialBinary(_), 2))
                    }) =>
                    {
                        special(h.name())
                            .ok_or_else(|| CompileError::Unsupported(h.name().into()))?
                    }
                    _ if count == 1 && unary(h.name()).is_some() => I::Unary(
                        unary(h.name())
                            .ok_or_else(|| CompileError::Unsupported(h.name().into()))?,
                    ),
                    _ if count == 2 && binary(h.name()).is_some() => I::Binary(
                        binary(h.name())
                            .ok_or_else(|| CompileError::Unsupported(h.name().into()))?,
                    ),
                    _ => {
                        return Err(CompileError::Unsupported(format!(
                            "{} with {count} arguments",
                            h.name()
                        )));
                    }
                };
                work.push(Work::Emit(i, count));
                work.extend(n.args.iter().rev().map(Work::Visit));
            }
        }
    }
    if depth != 1 {
        return Err(CompileError::Unsupported("unbalanced result".into()));
    }
    Ok(CompiledFn {
        code,
        variables: vars.len(),
        stack,
    })
}
