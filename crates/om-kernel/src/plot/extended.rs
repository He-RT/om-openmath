//! Real additive 2D request parsing and shared readonly sampling setup.
use super::*;
type LocalAxes = (Vec<Symbol>, Vec<(Symbol, Option<Expr>)>);
fn valid_color(s: &str) -> bool {
    matches!(s, "green" | "blue" | "red" | "orange" | "purple" | "cyan")
        || s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|c| c.is_ascii_hexdigit())
}

pub(super) fn invalid(s: &str) -> PlotError {
    PlotError::Invalid(s.into())
}
pub(crate) fn machine(e: &Expr, eval: &Evaluator, ctx: &Interrupt) -> Result<f64, PlotError> {
    let e = eval.fork_readonly().evaluate(e, ctx)?;
    if e.as_number()
        .is_some_and(|n| matches!(n.precision(), om_num::Precision::Bits(_)))
    {
        return Err(invalid("绘图新增路径仅机器精度，不能静默降低高精度输入"));
    }
    let n = if let Some(n) = e.as_number() {
        n.clone()
    } else {
        om_simplify::numeval::approximate(&e, om_num::Precision::Machine, ctx)?
            .ok_or_else(|| invalid("绘图需要有限实数"))?
    };
    if matches!(n, om_num::Number::Complex(_)) {
        return Err(invalid("绘图需要实数"));
    }
    n.to_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| invalid("绘图数值无法在有限机器范围表示"))
}
fn string(e: &Expr, eval: &Evaluator, ctx: &Interrupt) -> Result<String, PlotError> {
    let e = eval.fork_readonly().evaluate(e, ctx)?;
    if let om_core::ExprKind::String(s) = e.kind() {
        Ok(s.to_string())
    } else {
        Err(invalid("绘图模式/颜色需要字符串"))
    }
}
pub(super) fn bounds(values: &[f64]) -> Result<(f64, f64), PlotError> {
    if values.is_empty() || values.iter().any(|v| !v.is_finite()) {
        return Err(invalid("数据图需要有限非空样本"));
    }
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let pad = if hi > lo {
        0.05 * (hi - lo)
    } else {
        (lo.abs() * 0.05).max(0.5)
    };
    let r = (lo - pad, hi + pad);
    range(r)?;
    Ok(r)
}
pub(super) fn log_bounds(values: &[f64]) -> Result<(f64, f64), PlotError> {
    let positives: Vec<_> = values
        .iter()
        .copied()
        .filter(|v| v.is_finite() && *v > 0.)
        .collect();
    if positives.is_empty() {
        return Err(invalid("对数坐标没有正的有限样本"));
    }
    let lo = positives.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = positives.iter().copied().fold(0., f64::max);
    let result = (lo / 1.1, hi * 1.1);
    range(result)?;
    Ok(result)
}
pub(crate) fn checked_precision(e: &Expr, ctx: &Interrupt) -> Result<(), PlotError> {
    let mut work = vec![e];
    while let Some(e) = work.pop() {
        ctx.tick()?;
        if e.as_number()
            .is_some_and(|n| matches!(n.precision(), om_num::Precision::Bits(_)))
        {
            return Err(invalid(
                "新增二维采样仅支持机器精度，不能静默降低高精度输入",
            ));
        }
        if let om_core::ExprKind::Normal(n) = e.kind() {
            work.push(&n.head);
            work.extend(n.args.iter());
        }
    }
    Ok(())
}
fn rows(e: &Expr, eval: &Evaluator, ctx: &Interrupt) -> Result<Vec<Vec<f64>>, PlotError> {
    let e = eval.fork_readonly().evaluate(e, ctx)?;
    if e.is_head(B::DATA_TABLE) {
        if e.args().len() != 2 || !e.args()[1].is_head(B::LIST) || e.args()[1].args().len() > 50000
        {
            return Err(invalid("data_plot需要有效x/y数据表"));
        }
        let mut rows = vec![];
        for row in e.args()[1].args() {
            ctx.tick()?;
            if !row.is_head(B::RECORD) {
                return Err(invalid("数据表行需要记录"));
            }
            let get = |name: &str| {
                row.args()
                    .iter()
                    .find(|r| {
                        r.is_head(B::RULE)
                            && r.args().len() == 2
                            && r.args()[0] == Expr::string(name)
                    })
                    .map(|r| &r.args()[1])
                    .ok_or_else(|| invalid("data_plot表格需要x和y列"))
            };
            rows.push(vec![
                machine(get("x")?, eval, ctx)?,
                machine(get("y")?, eval, ctx)?,
            ]);
        }
        return Ok(rows);
    }
    if !e.is_head(B::LIST) || e.args().is_empty() || e.args().len() > 100000 {
        return Err(invalid("绘图数据需要有限非空列表"));
    }
    if e.args().iter().all(|e| !e.is_head(B::LIST)) {
        return e
            .args()
            .iter()
            .map(|e| Ok(vec![machine(e, eval, ctx)?]))
            .collect();
    }
    let mut result = vec![];
    let mut count = 0;
    for row in e.args() {
        ctx.tick()?;
        if !row.is_head(B::LIST) || row.args().is_empty() {
            return Err(invalid("绘图矩阵/点数据必须为二维实数列表"));
        }
        count += row.args().len();
        if count > 100000 {
            return Err(invalid("绘图数据超过100000标量"));
        }
        result.push(
            row.args()
                .iter()
                .map(|e| machine(e, eval, ctx))
                .collect::<Result<_, _>>()?,
        );
    }
    Ok(result)
}
pub(crate) fn from_expr(
    e: &Expr,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<Option<PlotRequest>, PlotError> {
    let name = e.head_symbol().map(|s| s.name()).unwrap_or("");
    let kind = match name {
        "ParametricPlot" => PlotKind::Parametric,
        "RegionPlot" => PlotKind::Region,
        "FieldPlot" => PlotKind::Field,
        "DataPlot" => PlotKind::Data,
        "Histogram" => PlotKind::Histogram,
        "DensityPlot" => PlotKind::Density,
        "Plot" | "ContourPlot" => {
            if !e.args().iter().any(|o| {
                o.is_head(B::RULE)
                    && o.args()
                        .first()
                        .and_then(Expr::as_symbol)
                        .is_some_and(|s| matches!(s.name(), "Scale" | "Levels" | "Color"))
            }) {
                return Ok(None);
            }
            if name == "Plot" {
                PlotKind::Function
            } else {
                PlotKind::Implicit
            }
        }
        _ => return Ok(None),
    };
    let required = match kind {
        PlotKind::Data | PlotKind::Histogram => 1,
        PlotKind::Function | PlotKind::Parametric => 2,
        _ => 3,
    };
    if e.args().len() < required {
        return Err(invalid("绘图缺少表达式/数学范围"));
    }
    let mut opts = PlotOptions2D {
        bins: 20,
        ..Default::default()
    };
    for a in &e.args()[1..required] {
        if !a.is_head(B::LIST) || a.args().len() != 3 {
            return Err(invalid("绘图坐标需要[变量,下界,上界]"));
        }
        let name = a.args()[0]
            .as_symbol()
            .ok_or_else(|| invalid("绘图坐标需要符号"))?
            .name()
            .to_string();
        let range = (
            machine(&a.args()[1], eval, ctx)?,
            machine(&a.args()[2], eval, ctx)?,
        );
        super::range(range)?;
        opts.axes.push(PlotAxis { name, range });
    }
    let mut y_view = None;
    let mut x_view = None;
    let mut seen = std::collections::BTreeSet::new();
    for rule in &e.args()[required..] {
        if !rule.is_head(B::RULE) || rule.args().len() != 2 {
            return Err(invalid("绘图尾参数需要已支持命名规则"));
        }
        let key = rule.args()[0]
            .as_symbol()
            .ok_or_else(|| invalid("绘图参数键需要符号"))?
            .name();
        if !seen.insert(key) {
            return Err(invalid("绘图参数重复"));
        }
        let value = &rule.args()[1];
        match key {
            "Scale" => {
                opts.scale = match string(value, eval, ctx)?.as_str() {
                    "linear" => PlotScale::Linear,
                    "log_x" => PlotScale::LogX,
                    "log_y" => PlotScale::LogY,
                    "log_log" => PlotScale::LogLog,
                    _ => return Err(invalid("scale只支持linear/log_x/log_y/log_log")),
                }
            }
            "Kind" if kind == PlotKind::Data => {
                opts.style = match string(value, eval, ctx)?.as_str() {
                    "scatter" => DataPlotStyle::Scatter,
                    "line" => DataPlotStyle::Line,
                    "heatmap" => DataPlotStyle::Heatmap,
                    _ => return Err(invalid("data_plot kind只支持scatter/line/heatmap")),
                }
            }
            "View" if kind == PlotKind::Field => {
                opts.stream = match string(value, eval, ctx)?.as_str() {
                    "arrows" => false,
                    "stream" => true,
                    _ => return Err(invalid("field_plot view只支持arrows/stream")),
                }
            }
            "Bins" if kind == PlotKind::Histogram => {
                let e = eval.fork_readonly().evaluate(value, ctx)?;
                opts.bins = match e.as_number() {
                    Some(om_num::Number::Integer(n)) => u32::try_from(n)
                        .ok()
                        .filter(|n| (1..=200).contains(n))
                        .ok_or_else(|| invalid("bins需要1..200整数"))?,
                    _ => return Err(invalid("bins需要整数")),
                };
            }
            "Levels" if kind == PlotKind::Implicit => {
                let e = eval.fork_readonly().evaluate(value, ctx)?;
                if !e.is_head(B::LIST) || e.args().is_empty() || e.args().len() > 32 {
                    return Err(invalid("levels需要1..32有限实数"));
                }
                opts.levels = e
                    .args()
                    .iter()
                    .map(|e| machine(e, eval, ctx))
                    .collect::<Result<_, _>>()?;
            }
            "Color" => {
                let s = string(value, eval, ctx)?;
                if !valid_color(&s) {
                    return Err(invalid(
                        "二维color首版支持颜色名或#RRGGBB，函数颜色留三维批次",
                    ));
                }
                opts.color = Some(s);
            }
            "PlotRange" => {
                if value.is_head(B::LIST)
                    && value.args().len() == 2
                    && value.args()[0].is_head(B::LIST)
                {
                    x_view = Some(explicit::pair(&value.args()[0], eval, ctx)?);
                    y_view = Some(explicit::pair(&value.args()[1], eval, ctx)?);
                } else {
                    y_view = Some(explicit::pair(value, eval, ctx)?);
                }
            }
            _ => return Err(invalid("此绘图类型不支持给定参数")),
        }
    }
    let mut expression = vec![om_format::input_form(&e.args()[0])];
    let (mut x, mut y, var_x, var_y) = if kind == PlotKind::Data || kind == PlotKind::Histogram {
        opts.samples = rows(&e.args()[0], eval, ctx)?;
        if kind == PlotKind::Histogram {
            if opts.samples.iter().any(|r| r.len() != 1) {
                return Err(invalid("histogram需要一维有限实数列表"));
            }
            let xs: Vec<_> = opts.samples.iter().map(|r| r[0]).collect();
            (bounds(&xs)?, None, "x".into(), None)
        } else if opts.style == DataPlotStyle::Heatmap {
            let n = opts.samples[0].len();
            if n == 0 || opts.samples.iter().any(|r| r.len() != n) {
                return Err(invalid("热图数据必须矩形"));
            }
            (
                (0., n as f64),
                Some((0., opts.samples.len() as f64)),
                "x".into(),
                Some("y".into()),
            )
        } else {
            if opts.samples.iter().any(|r| r.len() != 2) {
                return Err(invalid("data_plot需要[x,y]样本"));
            }
            (
                bounds(&opts.samples.iter().map(|r| r[0]).collect::<Vec<_>>())?,
                Some(bounds(
                    &opts.samples.iter().map(|r| r[1]).collect::<Vec<_>>(),
                )?),
                "x".into(),
                None,
            )
        }
    } else if kind == PlotKind::Parametric {
        if !e.args()[0].is_head(B::LIST) || e.args()[0].args().len() != 2 {
            return Err(invalid(
                "二维parametric_plot需要两坐标向量；三维稍后提供独立场景接口",
            ));
        }
        expression = e.args()[0]
            .args()
            .iter()
            .map(om_format::input_form)
            .collect();
        ((-1., 1.), None, opts.axes[0].name.clone(), None)
    } else {
        if kind == PlotKind::Field {
            if !e.args()[0].is_head(B::LIST) || e.args()[0].args().len() != 2 {
                return Err(invalid("field_plot需要两坐标向量"));
            }
            expression = e.args()[0]
                .args()
                .iter()
                .map(om_format::input_form)
                .collect();
        } else if e.args()[0].is_head(B::LIST) && kind != PlotKind::Region {
            expression = e.args()[0]
                .args()
                .iter()
                .map(om_format::input_form)
                .collect();
        }
        (
            opts.axes[0].range,
            opts.axes.get(1).map(|a| a.range),
            opts.axes[0].name.clone(),
            opts.axes.get(1).map(|a| a.name.clone()),
        )
    };
    if kind == PlotKind::Data && opts.style != DataPlotStyle::Heatmap || kind == PlotKind::Histogram
    {
        if opts.scale.log_x() && x_view.is_none() {
            x = log_bounds(&opts.samples.iter().map(|p| p[0]).collect::<Vec<_>>())?;
        }
        if kind == PlotKind::Data && opts.scale.log_y() && y_view.is_none() {
            y = Some(log_bounds(
                &opts.samples.iter().map(|p| p[1]).collect::<Vec<_>>(),
            )?);
        }
    }
    if let Some(v) = x_view {
        x = v;
    }
    if let Some(v) = y_view {
        y = Some(v);
    }
    if opts.color.is_some()
        && (kind == PlotKind::Density
            || kind == PlotKind::Data && opts.style == DataPlotStyle::Heatmap)
    {
        return Err(invalid("密度/热图使用真实数值调色板，不支持固定color覆盖"));
    }
    Ok(Some(PlotRequest {
        options: Some(opts),
        kind,
        exprs: expression,
        var_x,
        var_y,
        x_range: x,
        y_range: y,
        params: Default::default(),
        points: vec![],
        shade: vec![],
        param_ranges: Default::default(),
        solve: None,
    }))
}
pub(super) fn setup(
    r: &PlotRequest,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<LocalAxes, PlotError> {
    let opts = r
        .options
        .as_ref()
        .ok_or_else(|| invalid("扩展绘图缺少options"))?;
    if let Some(color) = &opts.color
        && !valid_color(color)
    {
        return Err(invalid("绘图颜色不在支持范围"));
    }
    if opts.color.is_some()
        && (r.kind == PlotKind::Density
            || r.kind == PlotKind::Data && opts.style == DataPlotStyle::Heatmap)
    {
        return Err(invalid("密度/热图不支持固定颜色"));
    }
    range(r.x_range)?;
    if opts.scale.log_x() && r.x_range.0 <= 0. && r.kind != PlotKind::Parametric {
        return Err(invalid("对数横轴需要正的显示/采样窗口"));
    }
    if let Some(y) = r.y_range {
        range(y)?;
    }
    let count = match r.kind {
        PlotKind::Parametric | PlotKind::Function => 1,
        PlotKind::Data | PlotKind::Histogram => 0,
        _ => 2,
    };
    if opts.axes.len() != count || r.exprs.is_empty() || r.exprs.len() > 64 {
        return Err(invalid("绘图数学坐标/表达式数无效"));
    }
    let mut vars = vec![];
    for a in &opts.axes {
        ctx.tick()?;
        range(a.range)?;
        let s = axis(&a.name)?;
        if vars.contains(&s) {
            return Err(invalid("绘图坐标必须不同"));
        }
        vars.push(s);
    }
    let mut locals: Vec<_> = vars.iter().map(|s| (*s, None)).collect();
    for (key, v) in &r.params {
        ctx.tick()?;
        let s = axis(key)?;
        if vars.contains(&s) || !v.is_finite() {
            return Err(invalid("绘图参数不能重名/非有限"));
        }
        locals.push((s, Some(Expr::real(*v))));
    }
    for (name, b) in &r.param_ranges {
        axis(name)?;
        range(*b)?;
    }
    let _ = eval;
    Ok((vars, locals))
}
pub(super) fn compile(
    source: &str,
    r: &PlotRequest,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<om_eval::numeric::CompiledFn, PlotError> {
    let (vars, locals) = setup(r, eval, ctx)?;
    let raw = parse(source)?;
    let raw = eval.prepare_numeric(&raw, &locals, ctx)?;
    checked_precision(&raw, ctx)?;
    compile_f64_with_ctx(&raw, &vars, ctx).map_err(PlotError::from)
}
pub(super) fn sample(
    r: &PlotRequest,
    eval: &Evaluator,
    ctx: &Interrupt,
) -> Result<PlotData, PlotError> {
    let (vars, locals) = setup(r, eval, ctx)?;
    let mut data = match r.kind {
        PlotKind::Parametric => parametric::sample(r, eval, ctx)?,
        PlotKind::Field => field::sample(r, eval, ctx)?,
        PlotKind::Region => grid::region(r, eval, ctx)?,
        PlotKind::Density => grid::density(r, eval, ctx)?,
        PlotKind::Data | PlotKind::Histogram => dataset::sample(r, ctx)?,
        PlotKind::Implicit | PlotKind::Function => {
            let mut programs = vec![];
            let mut effective = r.clone();
            effective.exprs.clear();
            for src in &r.exprs {
                let raw = parse(src)?;
                let raw = if r.kind == PlotKind::Implicit
                    && raw.is_head(B::EQUAL)
                    && raw.args().len() == 2
                {
                    Expr::call(
                        B::PLUS,
                        [
                            raw.args()[0].clone(),
                            Expr::call(B::TIMES, [Expr::int(-1), raw.args()[1].clone()]),
                        ],
                    )
                } else {
                    raw
                };
                let raw = eval.prepare_numeric(&raw, &locals, ctx)?;
                checked_precision(&raw, ctx)?;
                let levels = if r.kind == PlotKind::Implicit {
                    r.options.as_ref().unwrap().levels.clone()
                } else {
                    vec![]
                };
                if levels.is_empty() {
                    programs.push(compile_f64_with_ctx(&raw, &vars, ctx)?);
                    effective.exprs.push(src.clone());
                } else {
                    for level in levels {
                        let raw = Expr::call(B::PLUS, [raw.clone(), Expr::real(-level)]);
                        programs.push(compile_f64_with_ctx(&raw, &vars, ctx)?);
                        effective.exprs.push(format!("{src} = {level}"));
                    }
                }
            }
            if r.kind == PlotKind::Implicit {
                implicit::sample(&effective, &programs, ctx)?
            } else {
                function::sample(&effective, &programs, ctx)?
            }
        }
    };
    data.scale = Some(r.options.as_ref().unwrap().scale);
    dataset::validate_log(&mut data)?;
    Ok(data)
}
