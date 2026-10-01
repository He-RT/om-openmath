//! Every PLAN §14 authority row executes the public solver, with 200-digit residual checks.
use om_core::{BUILTIN as B, Expr, canonicalize};
use om_num::{BigFloat, ctx::Interrupt};
use om_parse::{Dialect, parse_expr};
use om_solve::{Domain, SolutionSet, SolveOptions, Verification};
#[derive(serde::Deserialize)]
struct Corpus {
    case: Vec<Case>,
}
#[derive(serde::Deserialize)]
struct Case {
    id: String,
    check: String,
    input: String,
    authority: String,
    notes: String,
    expected: Option<String>,
    count: Option<usize>,
    #[serde(default)]
    values: Vec<Vec<String>>,
}
fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
fn corpus() -> Vec<Case> {
    toml::from_str::<Corpus>(include_str!("../../../tests/corpus/solve.toml"))
        .unwrap()
        .case
}
fn variables(e: &Expr) -> Vec<Expr> {
    if e.is_head(B::LIST) {
        e.args().iter().map(canonicalize).collect()
    } else {
        vec![canonicalize(e)]
    }
}
fn run(case: &Case) -> Result<(SolutionSet, Expr, Vec<Expr>), String> {
    let call = parse_expr(&case.input, Dialect::Wolfram).map_err(|e| format!("parse {e:?}"))?;
    let vars = variables(&call.args()[1]);
    let ctx = Interrupt::default();
    if call.is_head(B::ELIMINATE) {
        return om_solve::eliminate(&call.args()[0], &vars, &ctx)
            .map(|c| {
                (
                    SolutionSet::Region {
                        cond: c,
                        intervals: vec![],
                    },
                    call.args()[0].clone(),
                    vars,
                )
            })
            .map_err(|e| e.to_string());
    }
    let mut opts = SolveOptions::default();
    for arg in &call.args()[2..] {
        match arg.as_symbol() {
            Some(B::REALS) => opts.domain = Domain::Reals,
            Some(B::INTEGERS) => opts.domain = Domain::Integers,
            Some(B::RATIONALS) => opts.domain = Domain::Rationals,
            _ => {}
        }
        if arg.is_head(B::RULE) && arg.args().len() == 2 {
            let value = arg.args()[1].as_symbol() == Some(B::TRUE);
            match arg.args()[0].as_symbol() {
                Some(B::CUBICS) => opts.cubics = value,
                Some(B::QUARTICS) => opts.quartics = value,
                _ => {}
            }
        }
    }
    let result = if call.is_head(B::REDUCE) {
        om_solve::reduce(&call.args()[0], &vars, opts.domain, &ctx)
    } else {
        om_solve::solve(&call.args()[0], &vars, &opts, &ctx)
    };
    result
        .map(|r| (r.set, call.args()[0].clone(), vars))
        .map_err(|e| e.to_string())
}
fn small(e: &Expr) -> Result<(), String> {
    let Some(z) =
        om_simplify::numeval::enclose(e, 700, &Interrupt::default()).map_err(|e| e.to_string())?
    else {
        return Err(format!("200-digit residual unavailable: {e:?}"));
    };
    let limit =
        om_num::Rational::from(1) / om_num::Rational::from(om_num::Integer::from(10).pow(100));
    let limit: BigFloat = limit.to_float(700).value();
    if [z.re, z.im].iter().any(|b| {
        (if b.mid < BigFloat::ZERO {
            -&b.mid
        } else {
            b.mid.clone()
        }) + &b.rad
            >= limit
    }) {
        return Err(format!("residual exceeds 1e-100: {e:?}"));
    }
    Ok(())
}
fn originals(e: &Expr) -> Vec<Expr> {
    if e.is_head(B::LIST) || e.is_head(B::AND) {
        e.args().iter().flat_map(originals).collect()
    } else if e.is_head(B::EQUAL) {
        e.args()
            .windows(2)
            .map(|p| om_core::sub(p[0].clone(), p[1].clone()))
            .collect()
    } else {
        vec![]
    }
}
fn numeric(case: &Case, set: &SolutionSet, original: &Expr) -> Result<(), String> {
    let SolutionSet::Finite(roots) = set else {
        return Err("expected complete finite/family candidates".into());
    };
    let count = roots.iter().map(|r| r.multiplicity as usize).sum::<usize>();
    if Some(count) != case.count {
        return Err(format!("root count {count}, expected {:?}", case.count));
    }
    if roots
        .iter()
        .any(|r| r.verification == Verification::Unverified)
    {
        return Err("unverified authority root".into());
    }
    for root in roots {
        for k in [0, 1, -1] {
            let params = root
                .constants
                .iter()
                .map(|(c, _)| (c.clone(), Expr::int(k)))
                .chain(
                    [("a", 2), ("b", 3), ("c", 4), ("z", 5)]
                        .into_iter()
                        .map(|(s, v)| (e(s), Expr::int(v))),
                )
                .collect::<Vec<_>>();
            let rules = root
                .rules
                .iter()
                .map(|(v, r)| (v.clone(), r.replace_all(&params)))
                .collect::<Vec<_>>();
            for residual in originals(original) {
                small(&residual.replace_all(&rules).replace_all(&params))?;
            }
        }
    }
    if !case.values.is_empty() {
        let mut used = vec![false; case.values.len()];
        for root in roots {
            let mut found = None;
            for (i, row) in case.values.iter().enumerate() {
                if used[i] || row.len() != root.rules.len() {
                    continue;
                }
                let substitutions = root
                    .constants
                    .iter()
                    .map(|(c, _)| (c.clone(), Expr::int(0)))
                    .chain([(e("z"), Expr::int(5))])
                    .collect::<Vec<_>>();
                if root.rules.iter().zip(row).all(|((_, v), expected)| {
                    small(&om_core::sub(v.clone(), e(expected)).replace_all(&substitutions)).is_ok()
                }) {
                    found = Some(i);
                    break;
                }
            }
            let i = found.ok_or_else(|| {
                format!("coordinate vector lacks authority match: {:?}", root.rules)
            })?;
            used[i] = true;
        }
        if used.iter().any(|b| !*b) {
            return Err("missing authority coordinate vector".into());
        }
    }
    for (i, a) in roots.iter().enumerate() {
        for b in &roots[i + 1..] {
            let zero = |r: &om_solve::Solution| {
                r.constants
                    .iter()
                    .map(|(c, _)| (c.clone(), Expr::int(0)))
                    .collect::<Vec<_>>()
            };
            let different = a.rules.iter().zip(&b.rules).any(|((_, a), (_, b))| {
                om_simplify::numeval::enclose(
                    &om_core::sub(a.clone(), b.clone()).replace_all(&zero(&roots[i])),
                    700,
                    &Interrupt::default(),
                )
                .ok()
                .flatten()
                .is_some_and(|z| z.re.excludes_zero() || z.im.excludes_zero())
            });
            if !different {
                return Err("distinct authority roots were duplicated".into());
            }
        }
    }
    Ok(())
}
#[test]
fn all_fifty_three_authority_rows_execute_and_validate() {
    let cases = corpus();
    assert_eq!(cases.len(), 53);
    let mut errors = vec![];
    for case in cases {
        let result = (|| {
            let (set, original, _vars) = run(&case)?;
            if case.check == "=" {
                let expected = e(case
                    .expected
                    .as_ref()
                    .ok_or("missing exact authority form")?);
                let actual = canonicalize(&set.to_expr());
                let expected = om_format::input_form(&expected);
                let actual = om_format::input_form(&actual);
                if actual != expected {
                    return Err(format!("exact form {actual}; expected {expected}"));
                }
            } else {
                numeric(&case, &set, &original)?;
            }
            Ok(())
        })();
        if let Err(reason) = result {
            errors.push(format!(
                "#{} {}: {reason}; authority {} {}",
                case.id, case.input, case.authority, case.notes
            ));
        }
    }
    assert!(
        errors.is_empty(),
        "{} corpus failures:\n{}",
        errors.len(),
        errors.join("\n")
    );
}
