//! Every PLAN §14 authority row executes the public solver, with 200-digit residual checks.
use om_core::{BUILTIN as B, Expr, canonicalize};
use om_num::{BigFloat, ctx::Interrupt};
use om_parse::{Dialect, parse_expr};
use om_solve::{Domain, SolutionSet, Verification};
#[derive(serde::Deserialize)]
struct Corpus {
    case: Vec<Case>,
}
#[derive(serde::Deserialize)]
pub struct Case {
    pub id: String,
    pub check: String,
    pub input: String,
    pub authority: String,
    pub notes: String,
    pub expected: Option<String>,
    pub count: Option<usize>,
    #[serde(default)]
    pub values: Vec<Vec<String>>,
}
pub fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
pub fn corpus() -> Vec<Case> {
    toml::from_str::<Corpus>(include_str!("solve.toml"))
        .unwrap()
        .case
}
#[allow(dead_code)]
pub fn variables(e: &Expr) -> Vec<Expr> {
    if e.is_head(B::LIST) {
        e.args().iter().map(canonicalize).collect()
    } else {
        vec![canonicalize(e)]
    }
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
pub fn numeric(case: &Case, set: &SolutionSet, original: &Expr) -> Result<(), String> {
    let SolutionSet::Finite(roots) = set else {
        return Err("expected complete finite/family candidates".into());
    };
    let count = roots.iter().map(|r| r.multiplicity as usize).sum::<usize>();
    if Some(count) != case.count {
        return Err(format!("root count {count}, expected {:?}", case.count));
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

/// Decode the evaluator's displayed rule rows without assuming solver provenance.
/// Numeric residuals/coordinate references are checked independently below.
#[allow(dead_code)]
pub fn numeric_expr(case: &Case, expr: &Expr, original: &Expr) -> Result<(), String> {
    if !expr.is_head(B::LIST) {
        return Err("expected finite displayed rule rows".into());
    }
    let mut roots = vec![];
    for row in expr.args() {
        if !row.is_head(B::LIST) {
            return Err("expected a rule row".into());
        }
        let mut rules = vec![];
        let mut constants = vec![];
        for rule in row.args() {
            if !rule.is_head(B::RULE) || rule.args().len() != 2 {
                return Err("expected a rule".into());
            }
            let mut value = rule.args()[1].clone();
            if value.is_head(B::CONDITIONAL_EXPRESSION) {
                let mut scan = vec![&value.args()[1]];
                while let Some(c) = scan.pop() {
                    if c.is_head(B::ELEMENT)
                        && c.args().len() == 2
                        && c.args()[1].as_symbol() == Some(B::INTEGERS)
                    {
                        let pair = (c.args()[0].clone(), Domain::Integers);
                        if !constants.contains(&pair) {
                            constants.push(pair);
                        }
                    } else {
                        scan.extend(c.args());
                    }
                }
                value = value.args()[0].clone();
            }
            rules.push((rule.args()[0].clone(), value));
        }
        roots.push(om_solve::Solution {
            rules,
            constants,
            condition: None,
            multiplicity: 1,
            verification: Verification::Unverified,
            numeric: None,
        });
    }
    numeric(case, &SolutionSet::Finite(roots), original)
}
