//! Real readonly three-dimensional sampling, never frontend function evaluation.
mod geometry;
mod implicit;
mod parametric;
mod parse;
use crate::{plot::PlotError, protocol::*};
pub(crate) use geometry::validate;
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol};
use om_eval::{
    Evaluator,
    numeric::{CompiledFn, compile_f64_with_ctx},
};
pub(crate) use parse::from_expr;
pub(super) fn invalid(s: &str) -> PlotError {
    PlotError::Invalid(format!("三维: {s}"))
}
pub(super) fn setup(
    r: &Scene3DRequest,
    ev: &Evaluator,
    ctx: &Interrupt,
) -> Result<Evaluator, PlotError> {
    let count = match r.kind {
        SceneKind::Surface => 2,
        SceneKind::Implicit => 3,
        SceneKind::Parametric => r.axes.len(),
    };
    if !(8..=64).contains(&r.mesh_points)
        || !(1..=3).contains(&count)
        || r.axes.len() != count
        || r.expressions.is_empty()
        || r.parameters.len() > 64
    {
        return Err(invalid("坐标/采样/表达式数量无效"));
    }
    if r.kind == SceneKind::Parametric && (count > 2 || r.expressions.len() != 3)
        || r.kind != SceneKind::Parametric && r.expressions.len() != 1
    {
        return Err(invalid(
            "三维参数曲线/曲面需要三坐标；标量曲面/隐式需要单一表达式",
        ));
    }
    let mut names = std::collections::BTreeSet::new();
    let mut locals = vec![];
    for axis in &r.axes {
        ctx.tick()?;
        crate::plot::range(axis.range)?;
        let symbol = crate::plot::axis(&axis.name)?;
        if !names.insert(symbol) {
            return Err(invalid("数学坐标重复"));
        }
        locals.push((symbol, None));
    }
    let mut fork = ev.fork_readonly();
    // Axis masking is needed by the numeric source preparer; parameters remain live local substitutions.
    for (name, value) in &r.parameters {
        ctx.tick()?;
        if !value.is_finite() {
            return Err(invalid("参数非有限"));
        }
        let symbol = crate::plot::axis(name)?;
        if names.contains(&symbol) {
            return Err(invalid("参数与坐标重名"));
        }
        locals.push((symbol, Some(Expr::real(*value))));
    }
    fork = fork.fork_with_optional_locals(&locals);
    Ok(fork)
}
pub(super) fn raw(source: &str) -> Result<Expr, PlotError> {
    if source.len() > 65536 {
        return Err(invalid("表达式源码超限"));
    }
    om_parse::parse_expr(source, om_parse::Dialect::Wolfram).map_err(|_| invalid("源码无效"))
}
pub(super) fn compile(
    source: &str,
    r: &Scene3DRequest,
    ev: &Evaluator,
    ctx: &Interrupt,
) -> Result<CompiledFn, PlotError> {
    let expression = raw(source)?;
    let expression = if r.kind == SceneKind::Implicit
        && expression.is_head(B::EQUAL)
        && expression.args().len() == 2
    {
        Expr::call(
            B::PLUS,
            [
                expression.args()[0].clone(),
                Expr::call(B::TIMES, [Expr::int(-1), expression.args()[1].clone()]),
            ],
        )
    } else {
        expression
    };
    let locals = r
        .axes
        .iter()
        .map(|a| Ok((crate::plot::axis(&a.name)?, None)))
        .collect::<Result<Vec<_>, PlotError>>()?;
    let expression = ev.prepare_numeric(&expression, &locals, ctx)?;
    crate::plot::check_machine_source(&expression, ctx)?;
    let vars = locals.iter().map(|(s, _)| *s).collect::<Vec<_>>();
    compile_f64_with_ctx(&expression, &vars, ctx).map_err(PlotError::from)
}
pub(super) struct Colorizer {
    fixed: Option<[f64; 4]>,
    function: Option<Expr>,
    eval: Evaluator,
}
impl Colorizer {
    fn new(source: &Option<String>, ev: &Evaluator, ctx: &Interrupt) -> Result<Self, PlotError> {
        let mut fixed = Some([0.1294, 0.5216, 0.2902, 1.]);
        let mut function = None;
        if let Some(source) = source {
            let expr = ev.fork_readonly().evaluate(&raw(source)?, ctx)?;
            if let om_core::ExprKind::String(name) = expr.kind() {
                fixed = Some(
                    crate::artifact::scene_color(name).map_err(|_| invalid("颜色不在支持范围"))?,
                );
            } else {
                if !expr.is_head(B::FUNCTION)
                    || expr.args().len() != 2
                    || !(expr.args()[0].is_head(B::LIST) || expr.args()[0].as_symbol().is_some())
                {
                    return Err(invalid(
                        "color需要颜色名字符串或fn(position,...axes)只读函数",
                    ));
                }
                function = Some(expr);
                fixed = None;
            }
        }
        Ok(Self {
            fixed,
            function,
            eval: ev.fork_readonly(),
        })
    }
    pub fn at(
        &mut self,
        p: [f64; 3],
        parameters: &[f64],
        ctx: &Interrupt,
    ) -> Result<[f64; 4], PlotError> {
        ctx.tick()?;
        if let Some(color) = self.fixed {
            return Ok(color);
        }
        let f = self
            .function
            .as_ref()
            .ok_or_else(|| invalid("缺少颜色函数"))?;
        let formal = &f.args()[0];
        let arity = if formal.is_head(B::LIST) {
            formal.args().len()
        } else {
            1
        };
        if arity != 1 && arity != parameters.len() + 1 {
            return Err(invalid(
                "颜色函数接收position向量或position加全部数学轴参数",
            ));
        }
        let mut args = vec![Expr::call(B::LIST, p.map(Expr::real))];
        if arity > 1 {
            args.extend(parameters.iter().map(|v| Expr::real(*v)));
        }
        let result = self.eval.evaluate(&Expr::normal(f.clone(), args), ctx)?;
        if !result.is_head(B::LIST) || !(3..=4).contains(&result.args().len()) {
            return Err(invalid("颜色函数必须返回3或4个实分量"));
        }
        let mut color = [0., 0., 0., 1.];
        for (i, e) in result.args().iter().enumerate() {
            ctx.tick()?;
            let number = if let Some(n) = e.as_number() {
                n.clone()
            } else {
                om_simplify::numeval::approximate(e, om_num::Precision::Machine, ctx)?
                    .ok_or_else(|| invalid("颜色分量需要已求值有限实数"))?
            };
            if matches!(number, om_num::Number::Complex(_))
                || matches!(number.precision(), om_num::Precision::Bits(_))
            {
                return Err(invalid("颜色分量不支持复数/高精度降级"));
            }
            color[i] = number
                .to_f64()
                .filter(|v| v.is_finite())
                .ok_or_else(|| invalid("颜色分量不是有限机器实数"))?;
            if !(0.0..=1.0).contains(&color[i]) {
                return Err(invalid("颜色分量需要0..1，不静默截断"));
            }
        }
        Ok(color)
    }
}
pub(crate) fn sample(
    r: &Scene3DRequest,
    ev: &Evaluator,
    ctx: &Interrupt,
) -> Result<Scene3DData, PlotError> {
    let ev = setup(r, ev, ctx)?;
    let mut colors = Colorizer::new(&r.color, &ev, ctx)?;
    let mut data = match r.kind {
        SceneKind::Implicit => implicit::sample(r, &ev, &mut colors, ctx)?,
        _ => parametric::sample(r, &ev, &mut colors, ctx)?,
    };
    geometry::finish(&mut data, ctx)?;
    geometry::validate(&data, ctx)?;
    Ok(data)
}
