//! Explicit three-dimensional mathematical domains; no reinterpretation of legacy 2D coordinates.
use super::*;
pub(crate) fn from_expr(
    e: &Expr,
    ev: &Evaluator,
    ctx: &Interrupt,
) -> Result<Option<Scene3DRequest>, PlotError> {
    if e.head_symbol().is_some_and(|s| s.name() == "Scene") {
        return crate::scene_graph::request(e, ev, ctx).map(Some);
    }
    let name = e.head_symbol().map(Symbol::name).unwrap_or("");
    let kind = match name {
        "Plot3D" => SceneKind::Surface,
        "ParametricPlot3D" => SceneKind::Parametric,
        "ImplicitPlot3D" => SceneKind::Implicit,
        _ => return Ok(None),
    };
    if e.args().is_empty() {
        return Err(invalid("缺少三维表达式"));
    }
    let mut axes = vec![];
    let mut mesh_points = if kind == SceneKind::Implicit { 24 } else { 48 };
    let mut color = None;
    let mut seen = std::collections::BTreeSet::new();
    for value in &e.args()[1..] {
        ctx.tick()?;
        if value.is_head(B::LIST)
            && value.args().len() == 3
            && value.args()[0].as_symbol().is_some()
        {
            if !seen.is_empty() {
                return Err(invalid("数学轴必须位于命名选项之前"));
            }
            let symbol = value.args()[0].as_symbol().unwrap();
            crate::plot::axis(symbol.name())?;
            let range = (
                crate::plot::machine_value(&value.args()[1], ev, ctx)?,
                crate::plot::machine_value(&value.args()[2], ev, ctx)?,
            );
            crate::plot::range(range)?;
            axes.push(PlotAxis {
                name: symbol.name().into(),
                range,
            });
        } else if value.is_head(B::RULE) && value.args().len() == 2 {
            let key = value.args()[0]
                .as_symbol()
                .ok_or_else(|| invalid("选项键需要符号"))?
                .name();
            if !seen.insert(key) {
                return Err(invalid("重复选项"));
            }
            match key {
                "MeshPoints" => {
                    let e = ev.fork_readonly().evaluate(&value.args()[1], ctx)?;
                    mesh_points = match e.as_number() {
                        Some(om_num::Number::Integer(n)) => u32::try_from(n)
                            .ok()
                            .filter(|v| (8..=64).contains(v))
                            .ok_or_else(|| invalid("mesh_points需要8..64整数"))?,
                        _ => return Err(invalid("mesh_points需要整数")),
                    };
                }
                "Color" => {
                    color = Some(om_format::input_form(&value.args()[1]));
                }
                _ => return Err(invalid("此三维入口不支持给定选项")),
            }
        } else {
            return Err(invalid("三维尾参数需要数学轴或命名选项"));
        }
    }
    let expr = e.args().first().ok_or_else(|| invalid("缺少三维表达式"))?;
    let expressions = if kind == SceneKind::Parametric {
        if !expr.is_head(B::LIST) || expr.args().len() != 3 {
            return Err(invalid("三维参数图需要三个坐标"));
        }
        expr.args().iter().map(om_format::input_form).collect()
    } else {
        vec![om_format::input_form(expr)]
    };
    let request = Scene3DRequest {
        kind,
        expressions,
        axes,
        mesh_points,
        color,
        parameters: Default::default(),
    };
    setup(&request, ev, ctx)?;
    Ok(Some(request))
}
