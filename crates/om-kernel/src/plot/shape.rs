//! Source classification preserves raw curves and refuses unrelated solver shapes.
use super::PlotError;
use crate::protocol::{PlotDomain, PlotKind};
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol};
use om_solve::Domain;
#[derive(Clone)]
pub(super) struct Shape {
    pub equations: Vec<(Expr, Expr)>,
    pub inequalities: Vec<(Expr, Expr)>,
}
impl Shape {
    pub fn parse(source: &Expr, axes: usize, ctx: &Interrupt) -> Result<Option<Self>, PlotError> {
        let mut result = Self {
            equations: vec![],
            inequalities: vec![],
        };
        let mut work = vec![source];
        let mut disjunction = false;
        while let Some(e) = work.pop() {
            ctx.tick()?;
            match e.head_symbol() {
                Some(B::LIST | B::AND | B::OR) => {
                    if e.is_head(B::OR) {
                        disjunction = true;
                    }
                    work.extend(e.args().iter().rev());
                }
                Some(B::EQUAL) if e.args().len() == 2 => result
                    .equations
                    .push((e.args()[0].clone(), e.args()[1].clone())),
                Some(B::LESS | B::LESS_EQUAL | B::GREATER | B::GREATER_EQUAL) => {
                    for p in e.args().windows(2) {
                        result.inequalities.push((p[0].clone(), p[1].clone()));
                    }
                }
                Some(B::INEQUALITY) => {
                    for p in e.args().windows(3).step_by(2) {
                        if !p[1].as_symbol().is_some_and(|s| {
                            matches!(s, B::LESS | B::LESS_EQUAL | B::GREATER | B::GREATER_EQUAL)
                        }) {
                            return Ok(None);
                        }
                        result.inequalities.push((p[0].clone(), p[2].clone()));
                    }
                }
                Some(B::ELEMENT | B::UNEQUAL) => {}
                _ if matches!(e.as_symbol(), Some(B::TRUE | B::FALSE)) => {}
                _ => return Ok(None),
            }
        }
        if axes == 1
            && result.equations.len() == 1
            && result.inequalities.is_empty()
            && !disjunction
            || axes == 2
                && result.equations.len() == 2
                && result.inequalities.is_empty()
                && !disjunction
            || axes == 1
                && result.equations.is_empty()
                && !result.inequalities.is_empty()
                && result.inequalities.len() <= 64
        {
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
    pub fn region(&self) -> bool {
        !self.inequalities.is_empty()
    }
    pub fn kind(&self, axes: usize) -> PlotKind {
        if axes == 2 {
            PlotKind::Implicit
        } else {
            PlotKind::Function
        }
    }
    pub fn curves(&self, axes: usize) -> Vec<Expr> {
        if axes == 1 && !self.region() {
            vec![self.equations[0].0.clone(), self.equations[0].1.clone()]
        } else {
            self.equations
                .iter()
                .chain(&self.inequalities)
                .map(|(a, b)| residual(a, b))
                .collect()
        }
    }
}
fn residual(a: &Expr, b: &Expr) -> Expr {
    Expr::call(
        B::PLUS,
        [a.clone(), Expr::call(B::TIMES, [Expr::int(-1), b.clone()])],
    )
}
pub(super) fn parameters(
    source: &Expr,
    vars: &[Symbol],
) -> std::collections::BTreeMap<String, Symbol> {
    source
        .free_symbols()
        .into_iter()
        .filter(|s| {
            !vars.contains(s)
                && !om_core::builtins::names().contains(&s.name())
                && om_eval::Evaluator::doc(*s).is_none()
        })
        .map(|s| (s.name().into(), s))
        .collect()
}
pub(super) fn to_domain(d: PlotDomain) -> Domain {
    match d {
        PlotDomain::Complexes => Domain::Complexes,
        PlotDomain::Reals => Domain::Reals,
        PlotDomain::Integers => Domain::Integers,
        PlotDomain::Rationals => Domain::Rationals,
    }
}
pub(super) fn from_domain(d: Domain) -> PlotDomain {
    match d {
        Domain::Complexes => PlotDomain::Complexes,
        Domain::Reals => PlotDomain::Reals,
        Domain::Integers => PlotDomain::Integers,
        Domain::Rationals => PlotDomain::Rationals,
    }
}
