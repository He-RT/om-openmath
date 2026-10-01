//! Iterative disjunctive normal form, retaining source-order leaves.
use om_core::{BUILTIN as B, Expr};
use om_num::ctx::{Abort, Interrupt};
pub(super) fn branches(e: &Expr, ctx: &Interrupt) -> Result<Option<Vec<Vec<Expr>>>, Abort> {
    enum Frame<'a> {
        Enter(&'a Expr),
        Build(&'a Expr),
    }
    let mut stack = vec![Frame::Enter(e)];
    let mut values: Vec<Vec<Vec<Expr>>> = vec![];
    while let Some(frame) = stack.pop() {
        ctx.tick()?;
        match frame {
            Frame::Enter(e) => {
                let conjunction = e.is_head(B::AND) || e.is_head(B::LIST);
                let disjunction = e.is_head(B::OR);
                let mut absorbing = false;
                if conjunction || disjunction {
                    for arg in e.args() {
                        ctx.tick()?;
                        if arg.as_symbol() == Some(if conjunction { B::FALSE } else { B::TRUE }) {
                            absorbing = true;
                            break;
                        }
                    }
                }
                if absorbing {
                    values.push(if conjunction { vec![] } else { vec![vec![]] });
                } else if conjunction || disjunction {
                    stack.push(Frame::Build(e));
                    stack.extend(e.args().iter().rev().map(Frame::Enter));
                } else if e.as_symbol() == Some(B::TRUE) {
                    values.push(vec![vec![]]);
                } else if e.as_symbol() == Some(B::FALSE) {
                    values.push(vec![]);
                } else {
                    values.push(vec![vec![e.clone()]]);
                }
            }
            Frame::Build(e) => {
                let args = values.split_off(values.len() - e.args().len());
                let mut result = if e.is_head(B::OR) {
                    vec![]
                } else {
                    vec![vec![]]
                };
                for arg in args {
                    ctx.tick()?;
                    if e.is_head(B::OR) {
                        if result.len() + arg.len() > 64 {
                            return Ok(None);
                        }
                        result.extend(arg);
                    } else {
                        if result.len().saturating_mul(arg.len()) > 64 {
                            return Ok(None);
                        }
                        let mut next = vec![];
                        for prefix in &result {
                            for suffix in &arg {
                                ctx.tick()?;
                                let mut branch = prefix.clone();
                                branch.extend(suffix.iter().cloned());
                                next.push(branch);
                            }
                        }
                        result = next;
                    }
                }
                values.push(result);
            }
        }
    }
    Ok(values.pop())
}
