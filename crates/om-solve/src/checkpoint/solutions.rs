use super::*;
use crate::{Bound, Interval, Solution, SolutionSet, Verification};
use atoms::DomainData;
/// Opaque complete computational solution data; deserialize does not imply successful validation.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolutionsData {
    version: u32,
    set: SetData,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum SetData {
    Finite(Vec<SolutionData>),
    All,
    Region {
        condition: u32,
        intervals: Vec<IntervalData>,
    },
    Unevaluated,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SolutionData {
    rules: Vec<(u32, u32)>,
    condition: Option<u32>,
    constants: Vec<(u32, DomainData)>,
    multiplicity: u32,
    verification: VerificationData,
    numeric: Option<Vec<(u64, u64)>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntervalData {
    lo: BoundData,
    hi: BoundData,
}
#[derive(Serialize, Deserialize)]
enum BoundData {
    NegInf,
    PosInf,
    Closed(u32),
    Open(u32),
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum VerificationData {
    Exact,
    ByConstruction,
    Numeric { digits: u32 },
    Unverified,
}
fn encode_bound(
    bound: &Bound,
    roots: &mut Vec<Expr>,
    limits: EvidenceLimits,
    ctx: &Interrupt,
) -> Result<BoundData, EvidenceError> {
    Ok(match bound {
        Bound::NegInf => BoundData::NegInf,
        Bound::PosInf => BoundData::PosInf,
        Bound::Closed(e) => BoundData::Closed(put(roots, e, limits, ctx)?),
        Bound::Open(e) => BoundData::Open(put(roots, e, limits, ctx)?),
    })
}
fn decode_bound(bound: BoundData, roots: &[Expr], ctx: &Interrupt) -> Result<Bound, EvidenceError> {
    Ok(match bound {
        BoundData::NegInf => Bound::NegInf,
        BoundData::PosInf => Bound::PosInf,
        BoundData::Closed(id) => Bound::Closed(get(roots, id, ctx)?),
        BoundData::Open(id) => Bound::Open(get(roots, id, ctx)?),
    })
}
/// Capture actual solution set/multiplicity/conditions/domains/verification and numeric bit patterns.
pub fn encode_solutions(
    set: &SolutionSet,
    roots: &mut Vec<Expr>,
    limits: EvidenceLimits,
    ctx: &Interrupt,
) -> Result<SolutionsData, EvidenceError> {
    ctx.tick()?;
    let set = match set {
        SolutionSet::All => SetData::All,
        SolutionSet::Unevaluated => SetData::Unevaluated,
        SolutionSet::Region { cond, intervals } => {
            if intervals.len() > limits.max_steps {
                return Err(EvidenceError::Limit);
            }
            let condition = put(roots, cond, limits, ctx)?;
            let intervals = intervals
                .iter()
                .map(|i| {
                    Ok(IntervalData {
                        lo: encode_bound(&i.lo, roots, limits, ctx)?,
                        hi: encode_bound(&i.hi, roots, limits, ctx)?,
                    })
                })
                .collect::<Result<_, EvidenceError>>()?;
            SetData::Region {
                condition,
                intervals,
            }
        }
        SolutionSet::Finite(solutions) => {
            if solutions.len() > limits.max_steps {
                return Err(EvidenceError::Limit);
            }
            let mut stored = Vec::new();
            for s in solutions {
                ctx.tick()?;
                if s.multiplicity == 0 {
                    return Err(EvidenceError::Invalid);
                }
                let rules = s
                    .rules
                    .iter()
                    .map(|(a, b)| Ok((put(roots, a, limits, ctx)?, put(roots, b, limits, ctx)?)))
                    .collect::<Result<_, EvidenceError>>()?;
                let condition = s
                    .condition
                    .as_ref()
                    .map(|c| put(roots, c, limits, ctx))
                    .transpose()?;
                let constants = s
                    .constants
                    .iter()
                    .map(|(e, d)| Ok((put(roots, e, limits, ctx)?, DomainData::encode(*d))))
                    .collect::<Result<_, EvidenceError>>()?;
                let verification = match s.verification {
                    Verification::Exact => VerificationData::Exact,
                    Verification::ByConstruction => VerificationData::ByConstruction,
                    Verification::Numeric { digits } => VerificationData::Numeric { digits },
                    Verification::Unverified => VerificationData::Unverified,
                };
                let numeric = s
                    .numeric
                    .as_ref()
                    .map(|ns| {
                        if ns.len() > limits.max_expressions || ns.len() != s.rules.len() {
                            return Err(EvidenceError::Invalid);
                        }
                        ns.iter()
                            .map(|&(a, b)| {
                                if !a.is_finite() || !b.is_finite() {
                                    return Err(EvidenceError::Invalid);
                                }
                                Ok((a.to_bits(), b.to_bits()))
                            })
                            .collect::<Result<_, EvidenceError>>()
                    })
                    .transpose()?;
                stored.push(SolutionData {
                    rules,
                    condition,
                    constants,
                    multiplicity: s.multiplicity,
                    verification,
                    numeric,
                });
            }
            SetData::Finite(stored)
        }
    };
    Ok(SolutionsData { version: 1, set })
}
/// Rebuild the actual set as data; does not run solve, infer exactness or turn failure into success.
pub fn decode_solutions(
    data: SolutionsData,
    roots: &[Expr],
    limits: EvidenceLimits,
    ctx: &Interrupt,
) -> Result<SolutionSet, EvidenceError> {
    ctx.tick()?;
    if data.version != 1 {
        return Err(EvidenceError::Invalid);
    }
    Ok(match data.set {
        SetData::All => SolutionSet::All,
        SetData::Unevaluated => SolutionSet::Unevaluated,
        SetData::Region {
            condition,
            intervals,
        } => {
            if intervals.len() > limits.max_steps {
                return Err(EvidenceError::Limit);
            }
            SolutionSet::Region {
                cond: get(roots, condition, ctx)?,
                intervals: intervals
                    .into_iter()
                    .map(|i| {
                        Ok(Interval {
                            lo: decode_bound(i.lo, roots, ctx)?,
                            hi: decode_bound(i.hi, roots, ctx)?,
                        })
                    })
                    .collect::<Result<_, EvidenceError>>()?,
            }
        }
        SetData::Finite(solutions) => {
            if solutions.len() > limits.max_steps {
                return Err(EvidenceError::Limit);
            }
            let mut decoded = Vec::new();
            for s in solutions {
                ctx.tick()?;
                if s.multiplicity == 0
                    || s.rules.len() > limits.max_expressions
                    || s.constants.len() > limits.max_expressions
                {
                    return Err(EvidenceError::Invalid);
                }
                let rules: Vec<(Expr, Expr)> = s
                    .rules
                    .into_iter()
                    .map(|(a, b)| Ok((get(roots, a, ctx)?, get(roots, b, ctx)?)))
                    .collect::<Result<_, EvidenceError>>()?;
                let condition = s.condition.map(|id| get(roots, id, ctx)).transpose()?;
                let constants = s
                    .constants
                    .into_iter()
                    .map(|(id, d)| Ok((get(roots, id, ctx)?, d.decode())))
                    .collect::<Result<_, EvidenceError>>()?;
                let verification = match s.verification {
                    VerificationData::Exact => Verification::Exact,
                    VerificationData::ByConstruction => Verification::ByConstruction,
                    VerificationData::Numeric { digits } => Verification::Numeric { digits },
                    VerificationData::Unverified => Verification::Unverified,
                };
                let numeric = s
                    .numeric
                    .map(|ns| {
                        if ns.len() > limits.max_expressions || ns.len() != rules.len() {
                            return Err(EvidenceError::Invalid);
                        }
                        ns.into_iter()
                            .map(|(a, b)| {
                                let a = f64::from_bits(a);
                                let b = f64::from_bits(b);
                                if !a.is_finite() || !b.is_finite() {
                                    return Err(EvidenceError::Invalid);
                                }
                                Ok((a, b))
                            })
                            .collect::<Result<_, EvidenceError>>()
                    })
                    .transpose()?;
                decoded.push(Solution {
                    rules,
                    condition,
                    constants,
                    multiplicity: s.multiplicity,
                    verification,
                    numeric,
                });
            }
            SolutionSet::Finite(decoded)
        }
    })
}
