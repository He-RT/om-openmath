//! Integral Bareiss updates have exact Scale/Add certificates in the expression field.
use super::convert;
use crate::{Level, RowOp, Step, StepKind, StepSink};
use om_core::{BUILTIN as B, Expr, div, mul, neg};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
use om_poly::{BareissOp, MPoly};
pub(super) fn expression(matrix: &[Vec<Expr>]) -> Expr {
    Expr::call(
        B::LIST,
        matrix
            .iter()
            .map(|r| Expr::call(B::LIST, r.iter().cloned())),
    )
}
pub(super) fn matrix(
    values: &[Vec<MPoly<Integer>>],
    parameters: &[Expr],
    ctx: &Interrupt,
) -> Result<Vec<Vec<Expr>>, Abort> {
    let mut result = vec![];
    for row in values {
        let mut next = vec![];
        for p in row {
            next.push(convert::render(p, parameters, ctx)?)
        }
        result.push(next)
    }
    Ok(result)
}
pub(super) struct Trace<'a, S> {
    pub current: Vec<Vec<Expr>>,
    pub parameters: &'a [Expr],
    pub ctx: &'a Interrupt,
    pub sink: &'a mut S,
}
impl<S: StepSink> Trace<'_, S> {
    pub fn observe(
        &mut self,
        op: BareissOp<MPoly<Integer>>,
        values: &[Vec<MPoly<Integer>>],
    ) -> Result<(), Abort> {
        if !self.sink.enabled() {
            return Ok(());
        }
        let after = matrix(values, self.parameters, self.ctx)?;
        match op {
            BareissOp::Swap { a, b } => {
                self.sink.record(|| {
                    Step::new(
                        StepKind::RowReduce {
                            op: RowOp::Swap { a, b },
                            matrix: after.clone(),
                        },
                        vec![expression(&self.current)],
                        vec![expression(&after)],
                        Level::Minor,
                    )
                });
            }
            BareissOp::Eliminate {
                target,
                source,
                pivot,
                entry,
                previous,
            } => {
                let previous = convert::render(&previous, self.parameters, self.ctx)?;
                let pivot = convert::render(&pivot, self.parameters, self.ctx)?;
                let entry = convert::render(&entry, self.parameters, self.ctx)?;
                let factor = div(pivot, previous.clone());
                let mut intermediate = self.current.clone();
                for value in &mut intermediate[target] {
                    *value = mul([factor.clone(), value.clone()])
                }
                self.sink.record(|| {
                    Step::new(
                        StepKind::RowReduce {
                            op: RowOp::Scale {
                                row: target,
                                factor,
                            },
                            matrix: intermediate.clone(),
                        },
                        vec![expression(&self.current)],
                        vec![expression(&intermediate)],
                        Level::Minor,
                    )
                });
                let factor = neg(div(entry, previous));
                self.sink.record(|| {
                    Step::new(
                        StepKind::RowReduce {
                            op: RowOp::Add {
                                target,
                                source,
                                factor,
                            },
                            matrix: after.clone(),
                        },
                        vec![expression(&intermediate)],
                        vec![expression(&after)],
                        Level::Minor,
                    )
                });
            }
        }
        self.current = after;
        Ok(())
    }
}
