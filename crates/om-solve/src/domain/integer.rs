//! A Hermite certificate produces the complete affine lattice over Z.
use super::{condition, periods};
use crate::{
    Domain, Level, Solution, SolutionSet, SolveError, SolveOptions, Step, StepKind, StepSink,
    Verification,
    normalize::NormalizedBranch,
    univariate::{PolynomialRoots, transcendental::check},
};
use om_core::{BUILTIN as B, Expr, Message, MsgLevel, add, mul};
use om_num::{Integer, ctx::Interrupt};
use om_poly::IntegerLinearResult;

pub(crate) fn solve(
    input: &Expr,
    vars: &[Expr],
    branch: &NormalizedBranch,
    opts: &SolveOptions,
    ctx: &Interrupt,
    sink: &mut impl StepSink,
    mut messages: Vec<Message>,
) -> Result<PolynomialRoots, SolveError> {
    let Some(matrix) = crate::linear::convert::build(&branch.equations, vars, ctx, sink)? else {
        return Ok(unsupported(messages));
    };
    if !matrix.parameters.is_empty() {
        return Ok(unsupported(messages));
    }
    let scalar =
        |p: &om_poly::MPoly<Integer>| p.terms.iter().fold(Integer::ZERO, |s, (_, c)| s + c);
    let a = matrix
        .a
        .iter()
        .map(|r| r.iter().map(scalar).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let b = matrix.b.iter().map(scalar).collect::<Vec<_>>();
    let Some(result) = om_poly::integer_linear_solve(&a, &b, vars.len(), ctx)? else {
        return Ok(unsupported(messages));
    };
    let IntegerLinearResult::Consistent(lattice) = result else {
        return Ok(PolynomialRoots {
            set: SolutionSet::Finite(vec![]),
            assumptions: vec![],
            messages,
        });
    };
    if branch.equations.is_empty() && branch.exclusions.is_empty() && branch.conditions.is_empty() {
        return Ok(PolynomialRoots {
            set: SolutionSet::All,
            assumptions: vec![],
            messages,
        });
    }
    let (inherited, mut next) = periods(input, opts, ctx)?;
    let mut constants = vec![];
    for _ in &lattice.nullspace {
        ctx.tick()?;
        constants.push((
            Expr::call(opts.generated_parameter, [Expr::int(i64::from(next))]),
            Domain::Integers,
        ));
        next = next
            .checked_add(1)
            .ok_or_else(|| SolveError::Unsupported("integer parameter index exhaustion".into()))?;
    }
    let mut rules = vec![];
    for (i, var) in vars.iter().enumerate() {
        let value = add(
            std::iter::once(Expr::integer(lattice.particular[i].clone())).chain(
                lattice
                    .nullspace
                    .iter()
                    .zip(&constants)
                    .map(|(v, (c, _))| mul([Expr::integer(v[i].clone()), c.clone()])),
            ),
        );
        sink.record(|| {
            Step::new(
                StepKind::BackSubstitute {
                    var: var.clone(),
                    value: value.clone(),
                },
                branch.original.clone(),
                vec![Expr::call(B::RULE, [var.clone(), value.clone()])],
                Level::Major,
            )
        });
        rules.push((var.clone(), value));
    }
    let mut root = Solution {
        rules,
        condition: None,
        constants,
        multiplicity: 1,
        verification: Verification::ByConstruction,
        numeric: None,
    };
    for c in &branch.conditions {
        root.condition = condition(root.condition.take(), c.replace_all(&root.rules))
    }
    let owned = root.constants.len();
    root.constants
        .extend(inherited.into_iter().map(|c| (c, Domain::Integers)));
    let mut roots = vec![root];
    let mut numeric = false;
    let identity = Expr::int(0);
    for original in branch.original.iter().chain(std::iter::once(&identity)) {
        check::candidates(
            check::Source {
                original,
                exclusions: &branch.exclusions,
                guards: &[],
            },
            &mut roots,
            opts,
            ctx,
            sink,
            &mut messages,
        )?;
        if roots.is_empty() {
            break;
        }
        numeric |= matches!(roots[0].verification, Verification::Numeric { .. });
    }
    for root in &mut roots {
        root.constants.truncate(owned);
        if numeric {
            root.verification = Verification::Numeric { digits: 33 }
        }
    }
    Ok(PolynomialRoots {
        set: SolutionSet::Finite(roots),
        assumptions: vec![],
        messages,
    })
}
fn unsupported(mut messages: Vec<Message>) -> PolynomialRoots {
    messages.push(Message {
        symbol: "Solve".into(),
        tag: "nsmet".into(),
        text: "Integer affine solving requires exact rational coefficients.".into(),
        level: MsgLevel::Warning,
    });
    PolynomialRoots {
        set: SolutionSet::Unevaluated,
        assumptions: vec![],
        messages,
    }
}
