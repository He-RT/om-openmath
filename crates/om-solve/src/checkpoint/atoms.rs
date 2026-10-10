use super::*;
use crate::{Domain, ExclReason, Formula, RowOp, Sign};
use om_core::{Message, MsgLevel};
use om_poly::MonoOrder;
use om_simplify::zero::{Tri, UnknownReason};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MessageData {
    symbol: String,
    tag: String,
    text: String,
    level: u8,
}
impl MessageData {
    pub fn encode(msg: &Message, limits: EvidenceLimits) -> Result<Self, EvidenceError> {
        for text in [&msg.symbol, &msg.tag, &msg.text] {
            texts(text, limits)?;
        }
        Ok(Self {
            symbol: msg.symbol.clone(),
            tag: msg.tag.clone(),
            text: msg.text.clone(),
            level: match msg.level {
                MsgLevel::Info => 0,
                MsgLevel::Warning => 1,
                MsgLevel::Error => 2,
            },
        })
    }
    pub fn decode(self, limits: EvidenceLimits) -> Result<Message, EvidenceError> {
        for text in [&self.symbol, &self.tag, &self.text] {
            texts(text, limits)?;
        }
        let level = match self.level {
            0 => MsgLevel::Info,
            1 => MsgLevel::Warning,
            2 => MsgLevel::Error,
            _ => return Err(EvidenceError::Invalid),
        };
        Ok(Message {
            symbol: self.symbol,
            tag: self.tag,
            text: self.text,
            level,
        })
    }
}
macro_rules! closed {
    ($wire:ident,$real:ident, $($v:ident),+)=>{
        #[derive(Serialize,Deserialize)]
        pub(super) enum $wire {$($v),+}
        impl $wire {
            pub fn encode(value:$real)->Self {match value {$($real::$v=>Self::$v),+}}
            pub fn decode(self)->$real {match self {$(Self::$v=>$real::$v),+}}
        }
    };
}
closed!(DomainData, Domain, Complexes, Reals, Integers, Rationals);
closed!(
    FormulaData,
    Formula,
    Linear,
    Quadratic,
    Cardano,
    Ferrari,
    Binomial,
    Palindromic
);
closed!(SignData, Sign, Negative, Zero, Positive);
closed!(
    ReasonData,
    ExclReason,
    ZeroDenominator,
    UndefinedFunction,
    BranchRestriction
);
closed!(OrderData, MonoOrder, Lex, GrevLex);
#[derive(Serialize, Deserialize)]
pub(super) enum TriData {
    Zero,
    NonZero,
    ProbablyZero,
    NoInfo,
}
impl TriData {
    pub fn encode(value: Tri) -> Self {
        match value {
            Tri::Zero => Self::Zero,
            Tri::NonZero => Self::NonZero,
            Tri::Unknown(UnknownReason::ProbablyZero) => Self::ProbablyZero,
            Tri::Unknown(UnknownReason::NoInfo) => Self::NoInfo,
        }
    }
    pub fn decode(self) -> Tri {
        match self {
            Self::Zero => Tri::Zero,
            Self::NonZero => Tri::NonZero,
            Self::ProbablyZero => Tri::Unknown(UnknownReason::ProbablyZero),
            Self::NoInfo => Tri::Unknown(UnknownReason::NoInfo),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) enum RowData {
    Swap {
        a: u32,
        b: u32,
    },
    Scale {
        row: u32,
        factor: u32,
    },
    Add {
        target: u32,
        source: u32,
        factor: u32,
    },
}
impl RowData {
    pub fn encode(
        op: &RowOp,
        roots: &mut Vec<Expr>,
        limits: EvidenceLimits,
        ctx: &Interrupt,
    ) -> Result<Self, EvidenceError> {
        let index = |n: usize| u32::try_from(n).map_err(|_| EvidenceError::Limit);
        Ok(match op {
            RowOp::Swap { a, b } => Self::Swap {
                a: index(*a)?,
                b: index(*b)?,
            },
            RowOp::Scale { row, factor } => Self::Scale {
                row: index(*row)?,
                factor: put(roots, factor, limits, ctx)?,
            },
            RowOp::Add {
                target,
                source,
                factor,
            } => Self::Add {
                target: index(*target)?,
                source: index(*source)?,
                factor: put(roots, factor, limits, ctx)?,
            },
        })
    }
    pub fn decode(self, roots: &[Expr], ctx: &Interrupt) -> Result<RowOp, EvidenceError> {
        Ok(match self {
            Self::Swap { a, b } => RowOp::Swap {
                a: a as usize,
                b: b as usize,
            },
            Self::Scale { row, factor } => RowOp::Scale {
                row: row as usize,
                factor: get(roots, factor, ctx)?,
            },
            Self::Add {
                target,
                source,
                factor,
            } => RowOp::Add {
                target: target as usize,
                source: source as usize,
                factor: get(roots, factor, ctx)?,
            },
        })
    }
}
