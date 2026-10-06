//! Held parameter exploration and explicit immutable readonly contexts, with no notebook or host IO.
use crate::{
    Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, EvalSettings, Evaluator, Rule,
};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt, Symbol};
use std::collections::{BTreeMap, BTreeSet};

/// One checked finite slider parameter; names never overwrite main-session bindings.
#[derive(Clone, Debug)]
pub struct ExploreControl {
    /// Local name.
    pub name: String,
    /// Finite increasing mathematical range.
    pub range: (f64, f64),
    /// Checked initial value.
    pub initial: f64,
}
/// Held mathematical expression and real numeric controls.
#[derive(Clone)]
pub struct ExplorePlan {
    /// Original unexecuted expression.
    pub expression: Expr,
    /// Ordered local controls.
    pub controls: Vec<ExploreControl>,
}
/// In-memory readonly state, deliberately separate from credentials, files and document mutation.
pub struct ReadonlyState {
    /// Actual global ownvalues, captured without evaluation.
    pub own: Vec<(Symbol, Expr)>,
    /// Actual user downvalues, captured without evaluation.
    pub down: Vec<(Symbol, Vec<Rule>)>,
    /// Actual user attributes as checked flag bits.
    pub attributes: Vec<(Symbol, u16)>,
    /// Successful history, needed for Out semantics.
    pub history: Vec<(Expr, Expr)>,
    /// Actual evaluator limits.
    pub settings: EvalSettings,
    /// Actual reproducible random stream state; readonly tasks never advance the main state.
    pub random_state: u64,
}
fn error(s: &str) -> EvalError {
    EvalError::Other(format!("explore: {s}"))
}
fn numeric(ev: &Evaluator, e: &Expr, ctx: &Interrupt) -> Result<f64, EvalError> {
    let value = ev.fork_readonly().evaluate(e, ctx)?;
    if value
        .as_number()
        .is_some_and(|n| matches!(n.precision(), om_num::Precision::Bits(_)))
    {
        return Err(error("首版滑块仅机器精度，不静默降低高精度范围/初值"));
    }
    let n = if let Some(n) = value.as_number() {
        n.clone()
    } else {
        om_simplify::numeval::approximate(&value, om_num::Precision::Machine, ctx)?
            .ok_or_else(|| error("范围/初值需要有限实数"))?
    };
    if matches!(n, om_num::Number::Complex(_)) {
        return Err(error("范围/初值需要实数"));
    }
    n.to_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| error("范围/初值超出有限机器表示"))
}
fn entries(e: &Expr) -> Result<&[Expr], EvalError> {
    if !e.is_head(B::RECORD) || e.args().is_empty() || e.args().len() > 16 {
        return Err(error("controls/initial需要1..16字段记录"));
    }
    Ok(e.args())
}
fn key(e: &Expr) -> Result<Symbol, EvalError> {
    let ExprKind::String(name) = e.kind() else {
        return Err(error("参数字段需要名字"));
    };
    let symbol = Symbol::intern(name);
    if name.is_empty()
        || !name
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
        || !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || om_core::builtins::names().contains(&symbol.name())
        || Evaluator::doc(symbol).is_some()
        || om_core::catalog::by_alias(name).is_some()
        || matches!(name.as_ref(), "pi" | "e" | "i" | "inf" | "infinity")
    {
        return Err(error("参数需要自由符号名，不能遮蔽内建函数/常量"));
    }
    Ok(symbol)
}
/// Decode actual held Explore syntax. Only ranges/initial values are read in a private readonly fork.
pub fn plan(e: &Expr, ev: &Evaluator, ctx: &Interrupt) -> Result<ExplorePlan, EvalError> {
    if e.head_symbol().is_none_or(|s| s.name() != "Explore") || !(2..=3).contains(&e.args().len()) {
        return Err(error("需要expression和controls，initial可省略"));
    }
    let mut controls = None;
    let mut initial = None;
    for option in &e.args()[1..] {
        ctx.tick()?;
        if !option.is_head(B::RULE) || option.args().len() != 2 {
            return Err(error("尾参数需要命名选项"));
        }
        match option.args()[0].as_symbol().map(Symbol::name) {
            Some("Controls") if controls.is_none() => controls = Some(&option.args()[1]),
            Some("Initial") if initial.is_none() => initial = Some(&option.args()[1]),
            _ => return Err(error("重复或不支持的选项")),
        }
    }
    let mut result = vec![];
    let mut names = BTreeSet::new();
    for entry in entries(controls.ok_or_else(|| error("缺少controls"))?)? {
        ctx.tick()?;
        if !entry.is_head(B::RULE) || entry.args().len() != 2 {
            return Err(error("controls字段无效"));
        }
        let symbol = key(&entry.args()[0])?;
        if !names.insert(symbol) {
            return Err(error("重复参数"));
        }
        let range = &entry.args()[1];
        if !(range.is_head(B::SPAN) || range.is_head(B::LIST)) || range.args().len() != 2 {
            return Err(error("参数范围需要lo..hi或[lo,hi]"));
        }
        let (lo, hi) = (
            numeric(ev, &range.args()[0], ctx)?,
            numeric(ev, &range.args()[1], ctx)?,
        );
        if lo >= hi || !(hi - lo).is_finite() {
            return Err(error("范围需要有限正宽度"));
        }
        result.push(ExploreControl {
            name: symbol.name().into(),
            range: (lo, hi),
            initial: lo + (hi - lo) * 0.5,
        });
    }
    if let Some(initial) = initial {
        let mut seen = BTreeSet::new();
        for entry in entries(initial)? {
            ctx.tick()?;
            if !entry.is_head(B::RULE) || entry.args().len() != 2 {
                return Err(error("initial字段无效"));
            }
            let symbol = key(&entry.args()[0])?;
            if !seen.insert(symbol) {
                return Err(error("重复initial参数"));
            }
            let control = result
                .iter_mut()
                .find(|c| c.name == symbol.name())
                .ok_or_else(|| error("initial包含controls之外的参数"))?;
            control.initial = numeric(ev, &entry.args()[1], ctx)?;
            if control.initial < control.range.0 || control.initial > control.range.1 {
                return Err(error("初值超出参数范围"));
            }
        }
    }
    let mut work = vec![&e.args()[0]];
    while let Some(value) = work.pop() {
        ctx.tick()?;
        if value.head_symbol().is_some_and(|s| s.name() == "Explore") {
            return Err(error("首版不支持嵌套explore"));
        }
        if let ExprKind::Normal(n) = value.kind() {
            work.push(&n.head);
            work.extend(n.args.iter());
        }
    }
    Ok(ExplorePlan {
        expression: e.args()[0].clone(),
        controls: result,
    })
}
pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    fn held(ev: &mut Evaluator, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
        plan(
            &Expr::call(Symbol::intern("Explore"), args.iter().cloned()),
            ev,
            ctx,
        )?;
        Ok(None)
    }
    specs.insert("Explore",BuiltinSpec{symbol:Symbol::intern("Explore"),f:held,attrs:A::HOLD_ALL|A::PROTECTED,arity:Arity::Range(2,3),doc:DocEntry{name:"Explore",modern:"explore(expression,controls:{a:lo..hi},initial:{a:value})",wolfram:"Explore[expression,Controls->Record[\"a\"->Span[lo,hi]]]",summary_zh:"真实独立只读参数探索，滑块不修改笔记本变量。",summary_en:"Actual isolated readonly parameter exploration, without changing notebook variables.",examples:&["Explore[a^2,Controls->Record[\"a\"->Span[0,4]]]"],category:"Visualization"}});
}
impl Evaluator {
    /// Create a private readonly evaluator whose local parameters remain available for output sampling.
    pub fn fork_with_locals(&self, locals: &[(Symbol, Expr)]) -> Self {
        let mut fork = self.fork_readonly();
        fork.scopes
            .push(locals.iter().map(|(s, v)| (*s, Some(v.clone()))).collect());
        fork
    }
    /// Capture actual state without executing delayed definitions or accessing host credentials.
    pub fn readonly_state(&self) -> ReadonlyState {
        ReadonlyState {
            own: self.defs.own.iter().map(|(s, e)| (*s, e.clone())).collect(),
            down: self
                .defs
                .down
                .iter()
                .map(|(s, r)| (*s, r.clone()))
                .collect(),
            attributes: self
                .defs
                .attrs
                .iter()
                .map(|(s, a)| (*s, a.bits()))
                .collect(),
            history: self.history.clone(),
            settings: self.settings.clone(),
            random_state: self.random.state(),
        }
    }
    /// Restore a bounded decoded state directly into a readonly evaluator; never replay definition source.
    pub fn from_readonly_state(state: ReadonlyState) -> Result<Self, EvalError> {
        let mut fork = Self::new().fork_readonly();
        fork.defs.own = state.own.into_iter().collect();
        fork.defs.down = state.down.into_iter().collect();
        for (symbol, bits) in state.attributes {
            fork.defs.attrs.insert(
                symbol,
                A::from_bits(bits).ok_or_else(|| error("无效属性位"))?,
            );
        }
        fork.history = state.history;
        fork.settings = state.settings;
        fork.random = om_num::rng::SplitMix64::new(state.random_state);
        Ok(fork)
    }
}
