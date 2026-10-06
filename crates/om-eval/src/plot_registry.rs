//! Plot syntax stays held until the shared kernel performs actual sampling.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
fn held(_: &mut Evaluator, _: &[Expr], _: &Interrupt) -> Result<Option<Expr>, EvalError> {
    Ok(None)
}
pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    crate::explore::register(specs);
    for (name, arity, modern, wolfram, examples) in [
        (
            "Plot3D",
            3,
            "plot(expr,x:a..b,y:c..d,view:\"surface\")",
            "Plot3D[expr,{x,a,b},{y,c,d}]",
            &["Plot3D[Sin[x]*Cos[y],{x,-2,2},{y,-2,2}]"][..],
        ),
        (
            "ParametricPlot3D",
            2,
            "parametric_plot([x,y,z],...axes)",
            "ParametricPlot3D[vector,{u,a,b},{v,c,d}]",
            &["ParametricPlot3D[{Cos[t],Sin[t],t},{t,0,2*Pi}]"][..],
        ),
        (
            "ImplicitPlot3D",
            4,
            "implicit_plot(equation,x:a..b,y:c..d,z:e..f)",
            "ImplicitPlot3D[equation,{x,a,b},{y,c,d},{z,e,f}]",
            &["ImplicitPlot3D[x^2+y^2+z^2==1,{x,-2,2},{y,-2,2},{z,-2,2}]"][..],
        ),
    ] {
        specs.insert(name,BuiltinSpec{symbol:Symbol::intern(name),f:held,attrs:A::HOLD_ALL|A::PROTECTED,arity:Arity::AtLeast(arity),doc:DocEntry{name,modern,wolfram,examples,summary_zh:"真实三维机器采样与内核颜色/法线；网格不是认证曲面。",summary_en:"Actual finite 3D machine samples with kernel colors and normals; meshes are not certified boundaries.",category:"Visualization"}});
    }
    for (name, symbol, arity, modern, wolfram, example, zh, en) in [
        (
            "Plot",
            B::PLOT,
            2,
            "plot(expr, [x,min,max])",
            "Plot[expr,{x,min,max}]",
            &["Plot[Sin[x],{x,0,2*Pi}]"][..],
            "采样一元实函数图像。",
            "Sample a real one-variable function.",
        ),
        (
            "ContourPlot",
            B::CONTOUR_PLOT,
            3,
            "implicitplot(equation, [x,min,max], [y,min,max])",
            "ContourPlot[eq,{x,min,max},{y,min,max}]",
            &["ContourPlot[x^2+y^2==1,{x,-2,2},{y,-2,2}]"][..],
            "采样二元隐函数等值轮廓。",
            "Sample a two-variable zero contour.",
        ),
    ] {
        specs.insert(
            name,
            BuiltinSpec {
                symbol,
                f: held,
                attrs: A::HOLD_ALL | A::PROTECTED,
                arity: Arity::AtLeast(arity),
                doc: DocEntry {
                    name,
                    modern,
                    wolfram,
                    summary_zh: zh,
                    summary_en: en,
                    examples: example,
                    category: "Visualization",
                },
            },
        );
    }
    for (name, arity, modern, wolfram, examples, summary) in [
        (
            "ParametricPlot",
            2,
            "parametric_plot(vector,t:a..b)",
            "ParametricPlot[vector,{t,a,b}]",
            &["ParametricPlot[{Cos[t],Sin[t]},{t,0,2*Pi}]"][..],
            "内核采样二维参数曲线，数学参数区间独立于显示窗口。",
        ),
        (
            "RegionPlot",
            3,
            "region_plot(condition,x:a..b,y:c..d)",
            "RegionPlot[condition,{x,a,b},{y,c,d}]",
            &["RegionPlot[x^2+y^2<1,{x,-2,2},{y,-2,2}]"][..],
            "真实布尔样本二维区域；网格近似不宣称认证边界。",
        ),
        (
            "FieldPlot",
            3,
            "field_plot(vector,x:a..b,y:c..d,view:\"arrows\")",
            "FieldPlot[vector,{x,a,b},{y,c,d}]",
            &["FieldPlot[{-y,x},{x,-2,2},{y,-2,2}]"][..],
            "真实二维向量场/流线，几何与样本值由内核生成。",
        ),
        (
            "DataPlot",
            1,
            "data_plot(data,kind:\"scatter\")",
            "DataPlot[data]",
            &["DataPlot[{{0,0},{1,1},{2,4}}]"][..],
            "真实有限数据散点、连线与矩阵热图。",
        ),
        (
            "Histogram",
            1,
            "histogram(data,bins:20)",
            "Histogram[data]",
            &["Histogram[{1,1,2,3,3}]"][..],
            "内核计算真实频数，最后箱包含最大值。",
        ),
        (
            "DensityPlot",
            3,
            "plot(expr,x:a..b,y:c..d,view:\"density\")",
            "DensityPlot[expr,{x,a,b},{y,c,d}]",
            &["DensityPlot[x*y,{x,-2,2},{y,-2,2}]"][..],
            "真实标量网格与有限颜色范围；非有限样本明确跳过。",
        ),
    ] {
        // Dynamic names append to the registry without reordering existing core symbol IDs.
        specs.insert(name,BuiltinSpec{symbol:Symbol::intern(name),f:held,attrs:A::HOLD_ALL|A::PROTECTED,arity:Arity::AtLeast(arity),doc:DocEntry{name,modern,wolfram,summary_zh:summary,summary_en:"Actual portable two-dimensional kernel samples and explicit supported geometry.",examples,category:"Visualization"}});
    }
}
