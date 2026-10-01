//! Conditional families are sampled only within their original domains.
use super::{
    super::{extract, radical::verify},
    inverse,
};
use crate::{
    Domain, Level, Solution, SolveError, SolveOptions, Step, StepKind, StepSink, Verification,
    normalize::Exclusion,
};
use om_core::{BUILTIN as B, Expr, ExprKind, Message, MsgLevel, func, sub};
use om_num::{BigFloat, Number, Rational, ctx::Interrupt, rng::SplitMix64};
use om_simplify::{
    numeval::enclose,
    zero::{Tri, UnknownReason},
};

fn lambert(e: &Expr, ctx: &Interrupt) -> Result<Expr, SolveError> {
    enum Frame<'a> {
        Enter(&'a Expr),
        Build(&'a om_core::Normal),
    }
    let mut stack = vec![Frame::Enter(e)];
    let mut values = vec![];
    while let Some(f) = stack.pop() {
        ctx.tick()?;
        match f {
            Frame::Enter(e) => {
                if let ExprKind::Normal(n) = e.kind() {
                    stack.push(Frame::Build(n));
                    stack.extend(n.args.iter().rev().map(Frame::Enter));
                    stack.push(Frame::Enter(&n.head))
                } else {
                    values.push(e.clone())
                }
            }
            Frame::Build(n) => {
                let args = values.split_off(values.len() - n.args.len());
                let head = values
                    .pop()
                    .expect("invariant: visited Lambert expression head");
                let mut e = if let Some(s) = head.as_symbol() {
                    func(s, args)
                } else {
                    Expr::normal(head, args)
                };
                if e.is_head(B::POWER)
                    && e.args().len() == 2
                    && e.args()[0].as_symbol() == Some(B::E)
                    && e.args()[1].is_head(B::PLUS)
                {
                    e = om_core::mul(
                        e.args()[1]
                            .args()
                            .iter()
                            .map(|t| om_core::pow(Expr::sym(B::E), t.clone())),
                    );
                }
                if e.is_head(B::TIMES) {
                    let mut factors = e.args().to_vec();
                    let mut replacement = None;
                    for (i, w) in factors.iter().enumerate() {
                        if w.is_head(B::PRODUCT_LOG)
                            && matches!(w.args().len(), 1 | 2)
                            && let Some(j) = factors.iter().position(|p| {
                                p.is_head(B::POWER)
                                    && p.args().len() == 2
                                    && p.args()[0].as_symbol() == Some(B::E)
                                    && p.args()[1] == *w
                            })
                        {
                            replacement = Some((
                                i,
                                j,
                                w.args()
                                    .last()
                                    .expect("invariant: ProductLog argument")
                                    .clone(),
                            ));
                            break;
                        }
                    }
                    if let Some((i, j, v)) = replacement {
                        factors.remove(i.max(j));
                        factors.remove(i.min(j));
                        factors.push(v);
                        e = om_core::mul(factors)
                    }
                }
                values.push(om_simplify::special::eval(&e).unwrap_or(e));
            }
        }
    }
    Ok(values
        .pop()
        .expect("invariant: one verified Lambert expression"))
}
fn nonzero(e: &Expr, ctx: &Interrupt) -> Result<bool, SolveError> {
    match verify::zero(e, ctx)? {
        Tri::Zero => Ok(false),
        Tri::NonZero => Ok(true),
        _ => {
            Ok(enclose(e, 512, ctx)?.is_some_and(|z| z.re.excludes_zero() || z.im.excludes_zero()))
        }
    }
}
fn allows(e: &Expr, ctx: &Interrupt) -> Result<Option<bool>, SolveError> {
    if e.is_head(B::AND) {
        let mut unknown = false;
        for c in e.args() {
            match allows(c, ctx)? {
                Some(false) => return Ok(Some(false)),
                None => unknown = true,
                _ => {}
            }
        }
        return Ok((!unknown).then_some(true));
    }
    let [a, b] = e.args() else { return Ok(None) };
    if e.is_head(B::EQUAL) {
        let difference = lambert(&sub(a.clone(), b.clone()), ctx)?;
        return Ok(match verify::zero(&difference, ctx)? {
            Tri::Zero => Some(true),
            Tri::NonZero => Some(false),
            Tri::Unknown(_) => {
                if difference.free_symbols().is_empty() {
                    Some(verify::numeric(&difference, ctx)?)
                } else {
                    None
                }
            }
        });
    }
    if e.is_head(B::UNEQUAL) {
        return Ok(Some(nonzero(&sub(a.clone(), b.clone()), ctx)?));
    }
    if e.is_head(B::ELEMENT) && b.as_symbol() == Some(B::REALS) {
        return Ok(enclose(a, 256, ctx)?.map(|z| !z.im.excludes_zero()));
    }
    let Some(z) = enclose(&sub(a.clone(), b.clone()), 256, ctx)? else {
        return Ok(None);
    };
    if z.im.mid != BigFloat::ZERO || z.im.rad != BigFloat::ZERO {
        return Ok(None);
    }
    let positive = z.re.mid > z.re.rad;
    let negative = z.re.mid < -&z.re.rad;
    let zero = z.re.mid == BigFloat::ZERO && z.re.rad == BigFloat::ZERO;
    Ok(match e.head_symbol() {
        Some(B::LESS) => {
            if negative {
                Some(true)
            } else if positive || zero {
                Some(false)
            } else {
                None
            }
        }
        Some(B::LESS_EQUAL) => {
            if negative || zero {
                Some(true)
            } else if positive {
                Some(false)
            } else {
                None
            }
        }
        Some(B::GREATER) => {
            if positive {
                Some(true)
            } else if negative || zero {
                Some(false)
            } else {
                None
            }
        }
        Some(B::GREATER_EQUAL) => {
            if positive || zero {
                Some(true)
            } else if negative {
                Some(false)
            } else {
                None
            }
        }
        _ => None,
    })
}
fn sample(
    e: &Expr,
    exclusions: &[Expr],
    condition: Option<&Expr>,
    opts: &SolveOptions,
    ctx: &Interrupt,
) -> Result<Option<Verification>, SolveError> {
    let mut vars = e.free_symbols().into_iter().collect::<Vec<_>>();
    for value in exclusions.iter().chain(condition) {
        for s in value.free_symbols() {
            if !vars.contains(&s) {
                vars.push(s)
            }
        }
    }
    vars.sort_by_key(|s| s.name().to_string());
    if vars.is_empty() {
        if let Some(c) = condition
            && allows(c, ctx)? == Some(false)
        {
            return Ok(None);
        }
        for value in exclusions {
            if !nonzero(value, ctx)? {
                return Ok(None);
            }
        }
        let residual = lambert(e, ctx)?;
        return Ok(match verify::zero(&residual, ctx)? {
            Tri::Zero => Some(Verification::Exact),
            Tri::NonZero => None,
            Tri::Unknown(_) => {
                verify::numeric(&residual, ctx)?.then_some(Verification::Numeric { digits: 33 })
            }
        });
    }
    let mut rng = SplitMix64::new(opts.seed);
    let mut passed = 0;
    for _ in 0..24 {
        ctx.tick()?;
        let rules = vars
            .iter()
            .map(|s| {
                (
                    Expr::sym(*s),
                    Expr::number(Number::Rational(
                        Rational::from(rng.next_range(0, 7) as i64 - 3)
                            / Rational::from(rng.next_range(1, 5)),
                    )),
                )
            })
            .collect::<Vec<_>>();
        if let Some(c) = condition
            && allows(&c.replace_all(&rules), ctx)? == Some(false)
        {
            continue;
        }
        let mut allowed = true;
        for v in exclusions {
            if !nonzero(&v.replace_all(&rules), ctx)? {
                allowed = false;
                break;
            }
        }
        if !allowed {
            continue;
        }
        let residual = lambert(&e.replace_all(&rules), ctx)?;
        match verify::zero(&residual, ctx)? {
            Tri::Zero => {}
            Tri::NonZero => return Ok(None),
            Tri::Unknown(_) => {
                if !verify::numeric(&residual, ctx)? {
                    return Ok(None);
                }
            }
        }
        passed += 1;
        if passed == 3 {
            return Ok(Some(Verification::Numeric { digits: 33 }));
        }
    }
    Ok(None)
}
pub(crate) struct Source<'a> {
    pub original: &'a Expr,
    pub exclusions: &'a [Exclusion],
    pub guards: &'a [Expr],
}
pub(crate) fn candidates(
    source: Source<'_>,
    roots: &mut Vec<Solution>,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
    messages: &mut Vec<Message>,
) -> Result<(), SolveError> {
    let Source {
        original,
        exclusions,
        guards,
    } = source;
    let guards = if guards.is_empty() {
        None
    } else if guards.len() == 1 {
        Some(guards[0].clone())
    } else {
        Some(Expr::call(B::AND, guards.iter().cloned()))
    };
    let mut kept = vec![];
    for mut root in roots.drain(..) {
        let values = exclusions
            .iter()
            .map(|e| e.value.replace_all(&root.rules))
            .collect::<Vec<_>>();
        for v in &values {
            let mut conditional = !v.free_symbols().is_empty();
            for (c, _) in &root.constants {
                conditional |= extract::depends(v, c, ctx)?;
            }
            if conditional {
                root.condition = inverse::and(
                    root.condition.take(),
                    Some(Expr::call(B::UNEQUAL, [v.clone(), Expr::int(0)])),
                )
            }
        }
        let residual = original.replace_all(&root.rules);
        let mut accepted = true;
        let mut numeric = false;
        let mut tested = 0;
        for k in if root.constants.is_empty() {
            vec![0]
        } else {
            vec![0, 1, -1, 2, -2]
        } {
            let rules = root
                .constants
                .iter()
                .map(|(c, _)| (c.clone(), Expr::int(k)))
                .collect::<Vec<_>>();
            let condition =
                inverse::and(root.condition.clone(), guards.clone()).map(|c| c.replace_all(&rules));
            if let Some(c) = &condition
                && allows(c, ctx)? == Some(false)
            {
                continue;
            }
            let sampled = lambert(&residual.replace_all(&rules), ctx)?;
            let exclusions = values
                .iter()
                .map(|v| v.replace_all(&rules))
                .collect::<Vec<_>>();
            let outcome = sample(&sampled, &exclusions, condition.as_ref(), opts, ctx)?;
            sink.record(|| {
                Step::new(
                    StepKind::Verify {
                        candidate: root.rules.clone(),
                        outcome: match outcome {
                            Some(Verification::Exact) => Tri::Zero,
                            Some(_) => Tri::Unknown(UnknownReason::ProbablyZero),
                            None => Tri::Unknown(UnknownReason::NoInfo),
                        },
                        residual: Some(sampled.clone()),
                    },
                    vec![original.clone()],
                    vec![sampled],
                    Level::Minor,
                )
            });
            let Some(v) = outcome else {
                accepted = false;
                break;
            };
            numeric |= matches!(v, Verification::Numeric { .. });
            tested += 1;
            if root.constants.is_empty() || tested == 3 {
                break;
            }
        }
        if accepted && tested > 0 {
            root.verification = if numeric {
                Verification::Numeric { digits: 33 }
            } else {
                Verification::Exact
            };
            kept.push(root)
        } else {
            sink.record(|| {
                Step::new(
                    StepKind::DropExtraneous {
                        candidate: root.rules.clone(),
                        why: "principal_branch_or_exclusion".into(),
                    },
                    vec![residual],
                    vec![],
                    Level::Major,
                )
            });
            messages.push(Message {
                symbol: "Solve".into(),
                tag: "verify".into(),
                text: "A transcendental candidate failed mandatory original-equation verification."
                    .into(),
                level: MsgLevel::Warning,
            });
        }
    }
    *roots = kept;
    Ok(())
}
pub(super) fn real_filter(
    roots: &mut Vec<Solution>,
    known: &[Expr],
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
) -> Result<(), SolveError> {
    if opts.domain != Domain::Reals {
        return Ok(());
    }
    let before = roots
        .iter()
        .map(|r| r.rules[0].1.clone())
        .collect::<Vec<_>>();
    let mut kept = vec![];
    for mut r in roots.drain(..) {
        let value = r.rules[0].1.clone();
        let mut certain = known.contains(&value);
        let mut reject = false;
        if r.constants.len() == 1 {
            let c = r.constants[0].0.clone();
            if let Some(p) = extract::coefficients(&value, &c, ctx)?
                && p.values.len() <= 2
            {
                let offset = p.values.first().cloned().unwrap_or_else(|| Expr::int(0));
                let step = p.values.get(1).cloned().unwrap_or_else(|| Expr::int(0));
                if let (Some(a), Some(b)) = (enclose(&offset, 256, ctx)?, enclose(&step, 256, ctx)?)
                {
                    let ar = a.im.mid == BigFloat::ZERO && a.im.rad == BigFloat::ZERO;
                    let br = b.im.mid == BigFloat::ZERO && b.im.rad == BigFloat::ZERO;
                    if ar && br {
                        certain = true
                    } else if ar
                        && b.re.mid == BigFloat::ZERO
                        && b.re.rad == BigFloat::ZERO
                        && b.im.excludes_zero()
                    {
                        r.rules[0].1 = offset;
                        r.constants.clear();
                        certain = true
                    } else if a.im.excludes_zero() && br {
                        reject = true
                    }
                }
            }
        } else if r.constants.is_empty()
            && !certain
            && let Some(z) = enclose(&value, 256, ctx)?
        {
            reject = z.im.excludes_zero();
            certain = z.im.mid == BigFloat::ZERO && z.im.rad == BigFloat::ZERO
        }
        if !reject {
            if !certain {
                r.condition = inverse::and(
                    r.condition.take(),
                    Some(Expr::call(
                        B::ELEMENT,
                        [r.rules[0].1.clone(), Expr::sym(B::REALS)],
                    )),
                )
            }
            kept.push(r)
        }
    }
    let kept_count = kept.len();
    let rejected = before.len() - kept_count;
    let after = kept
        .iter()
        .map(|r| r.rules[0].1.clone())
        .collect::<Vec<_>>();
    sink.record(|| {
        Step::new(
            StepKind::DomainFilter {
                domain: Domain::Reals,
                kept: kept_count,
                dropped: rejected,
            },
            before,
            after,
            Level::Major,
        )
    });
    *roots = kept;
    Ok(())
}
