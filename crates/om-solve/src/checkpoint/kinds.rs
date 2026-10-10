use super::atoms::*;
use super::*;
use crate::StepKind;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) enum KindData {
    Normalize,
    RecordExclusion {
        cond: u32,
        reason: ReasonData,
    },
    GenericAssumption {
        cond: u32,
    },
    ClearDenominators {
        factor: u32,
    },
    Expand,
    Factor {
        factors: Vec<(u32, u32)>,
    },
    ZeroProduct,
    SquareFree {
        parts: Vec<(u32, u32)>,
    },
    Substitute {
        new_var: u32,
        def: u32,
    },
    BackSubstitute {
        var: u32,
        value: u32,
    },
    ApplyFormula {
        formula: FormulaData,
        bindings: Vec<(String, u32)>,
        results: Vec<u32>,
    },
    Discriminant {
        value: u32,
        sign: Option<SignData>,
    },
    IsolateTerm {
        term: u32,
    },
    RaiseToPower {
        n: u32,
    },
    Resultant {
        var: u32,
        result: u32,
    },
    InvertFunction {
        func: u32,
        branches: Vec<u32>,
        constants: Vec<u32>,
    },
    RowReduce {
        op: RowData,
        matrix: Vec<Vec<u32>>,
    },
    Groebner {
        order: OrderData,
        basis: Vec<u32>,
    },
    Eliminant {
        var: u32,
        poly: u32,
    },
    SplitComponent {
        factor: u32,
    },
    RootObjects {
        poly: u32,
        real_count: u32,
    },
    Verify {
        candidate: Vec<(u32, u32)>,
        outcome: TriData,
        residual: Option<u32>,
    },
    DropExtraneous {
        candidate: Vec<(u32, u32)>,
        why: String,
    },
    DomainFilter {
        domain: DomainData,
        kept: u32,
        dropped: u32,
    },
    SignChart {
        points: Vec<u32>,
        signs: Vec<SignData>,
    },
    Branch {
        label: String,
    },
    Note {
        msg: MessageData,
    },
}
impl KindData {
    pub fn encode(
        kind: &StepKind,
        roots: &mut Vec<Expr>,
        limits: EvidenceLimits,
        ctx: &Interrupt,
    ) -> Result<Self, EvidenceError> {
        ctx.tick()?;
        Ok(match kind {
            StepKind::Normalize => Self::Normalize,
            StepKind::RecordExclusion { cond, reason } => Self::RecordExclusion {
                cond: put(roots, cond, limits, ctx)?,
                reason: ReasonData::encode(*reason),
            },
            StepKind::GenericAssumption { cond } => Self::GenericAssumption {
                cond: put(roots, cond, limits, ctx)?,
            },
            StepKind::ClearDenominators { factor } => Self::ClearDenominators {
                factor: put(roots, factor, limits, ctx)?,
            },
            StepKind::Expand => Self::Expand,
            StepKind::Factor { factors } => Self::Factor {
                factors: factors
                    .iter()
                    .map(|(e, n)| Ok((put(roots, e, limits, ctx)?, *n)))
                    .collect::<Result<_, EvidenceError>>()?,
            },
            StepKind::ZeroProduct => Self::ZeroProduct,
            StepKind::SquareFree { parts } => Self::SquareFree {
                parts: parts
                    .iter()
                    .map(|(e, n)| Ok((put(roots, e, limits, ctx)?, *n)))
                    .collect::<Result<_, EvidenceError>>()?,
            },
            StepKind::Substitute { new_var, def } => Self::Substitute {
                new_var: put(roots, new_var, limits, ctx)?,
                def: put(roots, def, limits, ctx)?,
            },
            StepKind::BackSubstitute { var, value } => Self::BackSubstitute {
                var: put(roots, var, limits, ctx)?,
                value: put(roots, value, limits, ctx)?,
            },
            StepKind::ApplyFormula {
                formula,
                bindings,
                results,
            } => Self::ApplyFormula {
                formula: FormulaData::encode(*formula),
                bindings: bindings
                    .iter()
                    .map(|(s, e)| {
                        texts(s, limits)?;
                        Ok((s.clone(), put(roots, e, limits, ctx)?))
                    })
                    .collect::<Result<_, EvidenceError>>()?,
                results: put_many(roots, results, limits, ctx)?,
            },
            StepKind::Discriminant { value, sign } => Self::Discriminant {
                value: put(roots, value, limits, ctx)?,
                sign: sign.map(SignData::encode),
            },
            StepKind::IsolateTerm { term } => Self::IsolateTerm {
                term: put(roots, term, limits, ctx)?,
            },
            StepKind::RaiseToPower { n } => Self::RaiseToPower { n: *n },
            StepKind::Resultant { var, result } => Self::Resultant {
                var: put(roots, var, limits, ctx)?,
                result: put(roots, result, limits, ctx)?,
            },
            StepKind::InvertFunction {
                func,
                branches,
                constants,
            } => Self::InvertFunction {
                func: put(roots, &Expr::sym(*func), limits, ctx)?,
                branches: put_many(roots, branches, limits, ctx)?,
                constants: put_many(roots, constants, limits, ctx)?,
            },
            StepKind::RowReduce { op, matrix } => Self::RowReduce {
                op: RowData::encode(op, roots, limits, ctx)?,
                matrix: matrix
                    .iter()
                    .map(|row| put_many(roots, row, limits, ctx))
                    .collect::<Result<_, EvidenceError>>()?,
            },
            StepKind::Groebner { order, basis } => Self::Groebner {
                order: OrderData::encode(*order),
                basis: put_many(roots, basis, limits, ctx)?,
            },
            StepKind::Eliminant { var, poly } => Self::Eliminant {
                var: put(roots, var, limits, ctx)?,
                poly: put(roots, poly, limits, ctx)?,
            },
            StepKind::SplitComponent { factor } => Self::SplitComponent {
                factor: put(roots, factor, limits, ctx)?,
            },
            StepKind::RootObjects { poly, real_count } => Self::RootObjects {
                poly: put(roots, poly, limits, ctx)?,
                real_count: *real_count,
            },
            StepKind::Verify {
                candidate,
                outcome,
                residual,
            } => Self::Verify {
                candidate: candidate
                    .iter()
                    .map(|(a, b)| Ok((put(roots, a, limits, ctx)?, put(roots, b, limits, ctx)?)))
                    .collect::<Result<_, EvidenceError>>()?,
                outcome: TriData::encode(*outcome),
                residual: residual
                    .as_ref()
                    .map(|e| put(roots, e, limits, ctx))
                    .transpose()?,
            },
            StepKind::DropExtraneous { candidate, why } => Self::DropExtraneous {
                candidate: candidate
                    .iter()
                    .map(|(a, b)| Ok((put(roots, a, limits, ctx)?, put(roots, b, limits, ctx)?)))
                    .collect::<Result<_, EvidenceError>>()?,
                why: {
                    texts(why, limits)?;
                    why.clone()
                },
            },
            StepKind::DomainFilter {
                domain,
                kept,
                dropped,
            } => Self::DomainFilter {
                domain: DomainData::encode(*domain),
                kept: u32::try_from(*kept).map_err(|_| EvidenceError::Limit)?,
                dropped: u32::try_from(*dropped).map_err(|_| EvidenceError::Limit)?,
            },
            StepKind::SignChart { points, signs } => Self::SignChart {
                points: put_many(roots, points, limits, ctx)?,
                signs: signs.iter().copied().map(SignData::encode).collect(),
            },
            StepKind::Branch { label } => Self::Branch {
                label: {
                    texts(label, limits)?;
                    label.clone()
                },
            },
            StepKind::Note { msg } => Self::Note {
                msg: MessageData::encode(msg, limits)?,
            },
        })
    }
    pub fn decode(
        self,
        roots: &[Expr],
        limits: EvidenceLimits,
        ctx: &Interrupt,
    ) -> Result<StepKind, EvidenceError> {
        ctx.tick()?;
        Ok(match self {
            Self::Normalize => StepKind::Normalize,
            Self::RecordExclusion { cond, reason } => StepKind::RecordExclusion {
                cond: get(roots, cond, ctx)?,
                reason: reason.decode(),
            },
            Self::GenericAssumption { cond } => StepKind::GenericAssumption {
                cond: get(roots, cond, ctx)?,
            },
            Self::ClearDenominators { factor } => StepKind::ClearDenominators {
                factor: get(roots, factor, ctx)?,
            },
            Self::Expand => StepKind::Expand,
            Self::Factor { factors } => StepKind::Factor {
                factors: factors
                    .into_iter()
                    .map(|(id, n)| {
                        if n == 0 {
                            return Err(EvidenceError::Invalid);
                        };
                        Ok((get(roots, id, ctx)?, n))
                    })
                    .collect::<Result<_, EvidenceError>>()?,
            },
            Self::ZeroProduct => StepKind::ZeroProduct,
            Self::SquareFree { parts } => StepKind::SquareFree {
                parts: parts
                    .into_iter()
                    .map(|(id, n)| {
                        if n == 0 {
                            return Err(EvidenceError::Invalid);
                        };
                        Ok((get(roots, id, ctx)?, n))
                    })
                    .collect::<Result<_, EvidenceError>>()?,
            },
            Self::Substitute { new_var, def } => StepKind::Substitute {
                new_var: get(roots, new_var, ctx)?,
                def: get(roots, def, ctx)?,
            },
            Self::BackSubstitute { var, value } => StepKind::BackSubstitute {
                var: get(roots, var, ctx)?,
                value: get(roots, value, ctx)?,
            },
            Self::ApplyFormula {
                formula,
                bindings,
                results,
            } => StepKind::ApplyFormula {
                formula: formula.decode(),
                bindings: bindings
                    .into_iter()
                    .map(|(s, id)| {
                        texts(&s, limits)?;
                        Ok((s, get(roots, id, ctx)?))
                    })
                    .collect::<Result<_, EvidenceError>>()?,
                results: get_many(roots, &results, ctx)?,
            },
            Self::Discriminant { value, sign } => StepKind::Discriminant {
                value: get(roots, value, ctx)?,
                sign: sign.map(SignData::decode),
            },
            Self::IsolateTerm { term } => StepKind::IsolateTerm {
                term: get(roots, term, ctx)?,
            },
            Self::RaiseToPower { n } => StepKind::RaiseToPower { n },
            Self::Resultant { var, result } => StepKind::Resultant {
                var: get(roots, var, ctx)?,
                result: get(roots, result, ctx)?,
            },
            Self::InvertFunction {
                func,
                branches,
                constants,
            } => StepKind::InvertFunction {
                func: get(roots, func, ctx)?
                    .as_symbol()
                    .ok_or(EvidenceError::Invalid)?,
                branches: get_many(roots, &branches, ctx)?,
                constants: get_many(roots, &constants, ctx)?,
            },
            Self::RowReduce { op, matrix } => StepKind::RowReduce {
                op: op.decode(roots, ctx)?,
                matrix: matrix
                    .iter()
                    .map(|row| get_many(roots, row, ctx))
                    .collect::<Result<_, EvidenceError>>()?,
            },
            Self::Groebner { order, basis } => StepKind::Groebner {
                order: order.decode(),
                basis: get_many(roots, &basis, ctx)?,
            },
            Self::Eliminant { var, poly } => StepKind::Eliminant {
                var: get(roots, var, ctx)?,
                poly: get(roots, poly, ctx)?,
            },
            Self::SplitComponent { factor } => StepKind::SplitComponent {
                factor: get(roots, factor, ctx)?,
            },
            Self::RootObjects { poly, real_count } => StepKind::RootObjects {
                poly: get(roots, poly, ctx)?,
                real_count,
            },
            Self::Verify {
                candidate,
                outcome,
                residual,
            } => StepKind::Verify {
                candidate: candidate
                    .into_iter()
                    .map(|(a, b)| Ok((get(roots, a, ctx)?, get(roots, b, ctx)?)))
                    .collect::<Result<_, EvidenceError>>()?,
                outcome: outcome.decode(),
                residual: residual.map(|id| get(roots, id, ctx)).transpose()?,
            },
            Self::DropExtraneous { candidate, why } => StepKind::DropExtraneous {
                candidate: candidate
                    .into_iter()
                    .map(|(a, b)| Ok((get(roots, a, ctx)?, get(roots, b, ctx)?)))
                    .collect::<Result<_, EvidenceError>>()?,
                why: {
                    texts(&why, limits)?;
                    why
                },
            },
            Self::DomainFilter {
                domain,
                kept,
                dropped,
            } => StepKind::DomainFilter {
                domain: domain.decode(),
                kept: kept as usize,
                dropped: dropped as usize,
            },
            Self::SignChart { points, signs } => StepKind::SignChart {
                points: get_many(roots, &points, ctx)?,
                signs: signs.into_iter().map(SignData::decode).collect(),
            },
            Self::Branch { label } => StepKind::Branch {
                label: {
                    texts(&label, limits)?;
                    label
                },
            },
            Self::Note { msg } => StepKind::Note {
                msg: msg.decode(limits)?,
            },
        })
    }
}
