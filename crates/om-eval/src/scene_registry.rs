//! Real held scene node constructors; kernel builders validate and generate actual finite geometry.
use crate::{Arity, Attributes as A, BuiltinSpec, DocEntry, EvalError, Evaluator};
use om_core::{Expr, Interrupt, Symbol};
use std::collections::BTreeMap;
fn held(_: &mut Evaluator, _: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    ctx.tick()?;
    Ok(None)
}
pub(crate) fn register(specs: &mut BTreeMap<&'static str, BuiltinSpec>) {
    for (name, min, modern, wolfram, examples) in [
        (
            "Scene",
            1,
            "scene(nodes,dimensions:2)",
            "Scene[nodes,Dimensions->2]",
            &["Scene[{Point[{0,0}],Line[{{0,0},{1,1}}]}]"][..],
        ),
        (
            "Point",
            1,
            "point(position,color:\"blue\")",
            "Point[position]",
            &["Point[{0,0}]"][..],
        ),
        (
            "Line",
            1,
            "line(points,color:\"blue\")",
            "Line[points]",
            &["Line[{{0,0},{1,1}}]"][..],
        ),
        (
            "Arrow",
            2,
            "arrow(start,end)",
            "Arrow[start,end]",
            &["Arrow[{0,0},{1,1}]"][..],
        ),
        (
            "Circle",
            2,
            "circle(center,radius)",
            "Circle[center,radius]",
            &["Circle[{0,0},1]"][..],
        ),
        (
            "Disk",
            2,
            "disk(center,radius)",
            "Disk[center,radius]",
            &["Disk[{0,0},1]"][..],
        ),
        (
            "Polygon",
            1,
            "polygon(vertices)",
            "Polygon[vertices]",
            &["Polygon[{{0,0},{1,0},{0,1}}]"][..],
        ),
        (
            "Sphere",
            2,
            "sphere(center,radius)",
            "Sphere[center,radius]",
            &["Sphere[{0,0,0},1]"][..],
        ),
        (
            "Ellipsoid",
            2,
            "ellipsoid(center,radii)",
            "Ellipsoid[center,radii]",
            &["Ellipsoid[{0,0,0},{1,2,3}]"][..],
        ),
        (
            "Box",
            2,
            "box(min,max)",
            "Box[min,max]",
            &["Box[{-1,-1,-1},{1,1,1}]"][..],
        ),
        (
            "Cylinder",
            3,
            "cylinder(start,end,radius)",
            "Cylinder[start,end,radius]",
            &["Cylinder[{0,0,0},{0,0,1},1]"][..],
        ),
        (
            "Cone",
            3,
            "cone(start,end,radius)",
            "Cone[start,end,radius]",
            &["Cone[{0,0,0},{0,0,1},1]"][..],
        ),
        (
            "Tube",
            2,
            "tube(path,radius)",
            "Tube[path,radius]",
            &["Tube[{{0,0,0},{0,0,1},{1,0,2}},0.1]"][..],
        ),
        (
            "Label",
            2,
            "label(text,position)",
            "Label[text,position]",
            &["Label[\"地球\",{0,0}]"][..],
        ),
        (
            "Translate",
            2,
            "translate(node,vector)",
            "Translate[node,vector]",
            &["Translate[Point[{0,0}],{1,2}]"][..],
        ),
        (
            "Rotate",
            2,
            "rotate(node,angle,axis:[0,0,1],center:[0,0,0])",
            "Rotate[node,angle,Axis->axis,Center->center]",
            &["Rotate[Line[{{0,0},{1,0}}],Pi/2]"][..],
        ),
        (
            "Scale",
            2,
            "scale(node,factors,center:[0,0,0])",
            "Scale[node,factors,Center->center]",
            &["Scale[Circle[{0,0},1],{2,1}]"][..],
        ),
        (
            "Style",
            1,
            "style(node,color:\"green\",opacity:1)",
            "Style[node,Color->color,Opacity->opacity]",
            &["Style[Circle[{0,0},1],Color->\"green\",Opacity->0.8]"][..],
        ),
    ] {
        specs.insert(name,BuiltinSpec{symbol:Symbol::intern(name),f:held,attrs:A::PROTECTED|A::HOLD_ALL,arity:if name == "Translate" { Arity::Exactly(2) } else { Arity::AtLeast(min) },doc:DocEntry{name,modern,wolfram,examples,summary_zh:"真实有限场景图元与变换；源码构造保持，几何由共享内核生成。",summary_en:"Finite scene primitives and transforms, with held source and real shared-kernel geometry.",category:"Graphics"}});
    }
    specs.insert("Blend",BuiltinSpec{symbol:Symbol::intern("Blend"),f:blend,attrs:A::PROTECTED,arity:Arity::Exactly(2),doc:DocEntry{name:"Blend",modern:"blend([color1,color2],weight)",wolfram:"Blend[{color1,color2},weight]",examples:&["Blend[{\"dark_green\",\"light_green\"},0.5]"],summary_zh:"两个固定RGB/RGBA颜色按0..1权重逐分量插值；机器精度，非线性光学混色。",summary_en:"Componentwise interpolation of two fixed RGB/RGBA colors with weight in 0..1 at machine precision.",category:"Graphics"}});
}
fn component(e: &Expr) -> Result<f64, EvalError> {
    let n = e
        .as_number()
        .ok_or_else(|| EvalError::Other("blend需要有限实数分量/权重".into()))?;
    if matches!(n, om_num::Number::Complex(_))
        || matches!(n.precision(), om_num::Precision::Bits(_))
    {
        return Err(EvalError::Other(
            "blend仅机器实数路径；不静默降低高精度".into(),
        ));
    }
    n.to_f64()
        .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
        .ok_or_else(|| EvalError::Other("blend分量/权重必须0..1".into()))
}
fn rgba(e: &Expr, ctx: &Interrupt) -> Result<[f64; 4], EvalError> {
    ctx.tick()?;
    if let om_core::ExprKind::String(s) = e.kind() {
        let rgb = om_core::graphics_color::rgb(s)
            .ok_or_else(|| EvalError::Other("blend不支持此颜色名".into()))?;
        return Ok([
            f64::from(rgb[0]) / 255.,
            f64::from(rgb[1]) / 255.,
            f64::from(rgb[2]) / 255.,
            1.,
        ]);
    }
    if !e.is_head(om_core::BUILTIN::LIST) || !(3..=4).contains(&e.args().len()) {
        return Err(EvalError::Other("blend颜色需要字符串或RGB/RGBA列表".into()));
    }
    let mut c = [0., 0., 0., 1.];
    for (i, v) in e.args().iter().enumerate() {
        ctx.tick()?;
        c[i] = component(v)?;
    }
    Ok(c)
}
fn blend(_: &mut Evaluator, args: &[Expr], ctx: &Interrupt) -> Result<Option<Expr>, EvalError> {
    ctx.tick()?;
    if !args[0].is_head(om_core::BUILTIN::LIST) || args[0].args().len() != 2 {
        return Err(EvalError::Other("blend首版需要恰好两个颜色".into()));
    }
    let a = rgba(&args[0].args()[0], ctx)?;
    let b = rgba(&args[0].args()[1], ctx)?;
    let w = component(&args[1])?;
    Ok(Some(Expr::call(
        om_core::BUILTIN::LIST,
        (0..4).map(|i| Expr::real(a[i] * (1. - w) + b[i] * w)),
    )))
}
