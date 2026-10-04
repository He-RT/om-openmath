//! Ordered data transformations execute functional keys once per element.
use super::*;
use om_core::ExprKind;
fn values(e: &Expr) -> Result<&[Expr], EvalError> {
    if e.is_head(B::LIST) && e.args().len() <= 100_000 {
        Ok(e.args())
    } else {
        Err(error("数据操作需要列表"))
    }
}
fn count(e: &Expr) -> Result<i64, EvalError> {
    match e.as_number() {
        Some(Number::Integer(n)) => i64::try_from(n).map_err(|_| error("计数超出范围")),
        _ => Err(error("计数需要整数")),
    }
}
fn sorted(values: &[Expr], keys: &[Expr], ctx: &Interrupt) -> Result<Expr, EvalError> {
    let numeric = keys.iter().all(|e| e.as_number().is_some());
    let strings = keys.iter().all(|e| matches!(e.kind(), ExprKind::String(_)));
    if !numeric && !strings {
        return Err(error("排序键须为同类实数或字符串"));
    }
    let mut order: Vec<_> = (0..values.len()).collect();
    let numbers = if numeric {
        Some(keys.iter().map(rational).collect::<Result<Vec<_>, _>>()?)
    } else {
        None
    };
    let mut abort = None;
    order.sort_by(|a, b| {
        if abort.is_none()
            && let Err(e) = ctx.tick()
        {
            abort = Some(e);
        }
        if abort.is_some() {
            return std::cmp::Ordering::Equal;
        }
        if let Some(numbers) = &numbers {
            numbers[*a].cmp(&numbers[*b])
        } else {
            string(&keys[*a])
                .unwrap_or("")
                .cmp(string(&keys[*b]).unwrap_or(""))
        }
    });
    if let Some(e) = abort {
        return Err(e.into());
    }
    Ok(list(order.into_iter().map(|i| values[i].clone())))
}
pub(super) fn dispatch(
    ev: &mut Evaluator,
    name: &str,
    args: &Args<'_>,
    ctx: &Interrupt,
) -> Result<Option<Expr>, EvalError> {
    if !matches!(
        name,
        "Filter"
            | "Sort"
            | "SortBy"
            | "Unique"
            | "Take"
            | "Drop"
            | "Flatten"
            | "Reshape"
            | "Zip"
            | "Fold"
            | "GroupBy"
            | "Counts"
            | "Slice"
    ) {
        return Ok(None);
    }
    if name == "Fold" {
        if args.values.len() != 3 {
            return Err(error("fold需要函数、初始值和列表"));
        }
        let mut result = args.values[1].clone();
        for value in values(args.values[2])? {
            ctx.tick()?;
            result = ev.evaluate(
                &Expr::normal(args.values[0].clone(), [result, value.clone()]),
                ctx,
            )?;
        }
        return Ok(Some(result));
    }
    let data = values(args.values[0])?;
    let result = match name {
        "Sort" => sorted(data, data, ctx)?,
        "SortBy" => {
            if args.values.len() != 2 {
                return Err(error("需要列表和键函数"));
            }
            let keys = data
                .iter()
                .map(|e| {
                    ctx.tick()?;
                    ev.evaluate(&Expr::normal(args.values[1].clone(), [e.clone()]), ctx)
                })
                .collect::<Result<Vec<_>, _>>()?;
            sorted(data, &keys, ctx)?
        }
        "Filter" => {
            if args.values.len() != 2 {
                return Err(error("需要列表和谓词"));
            }
            let mut output = vec![];
            for e in data {
                ctx.tick()?;
                let predicate =
                    ev.evaluate(&Expr::normal(args.values[1].clone(), [e.clone()]), ctx)?;
                match predicate.as_symbol() {
                    Some(B::TRUE) => output.push(e.clone()),
                    Some(B::FALSE) => {}
                    _ => return Err(error("谓词必须返回布尔值")),
                }
            }
            list(output)
        }
        "Unique" => {
            let mut seen = std::collections::HashSet::new();
            let mut output = vec![];
            for e in data {
                ctx.tick()?;
                if seen.insert(e.clone()) {
                    output.push(e.clone());
                }
            }
            list(output)
        }
        "Take" | "Drop" => {
            if args.values.len() != 2 {
                return Err(error("需要列表和计数"));
            }
            let n = count(args.values[1])?;
            let size = usize::try_from(n.unsigned_abs()).map_err(|_| error("计数溢出"))?;
            if size > data.len() {
                return Err(error("取出或丢弃计数超过列表长度"));
            }
            let interval = match (name, n >= 0) {
                ("Take", true) => 0..size,
                ("Take", false) => data.len() - size..data.len(),
                ("Drop", true) => size..data.len(),
                _ => 0..data.len() - size,
            };
            list(data[interval].iter().cloned())
        }
        "Slice" => {
            if args.values.len() != 2 {
                return Err(error("需要列表和闭区间"));
            }
            crate::composition::part(ev, args.values[0], args.values[1], ctx)?
                .ok_or_else(|| error("无效切片"))?
        }
        "Flatten" => {
            let limit = match args.options.get("Depth") {
                Some(e) if e.as_symbol() == Some(B::ALL) => usize::MAX,
                Some(e) => usize::try_from(count(e)?).map_err(|_| error("depth须非负"))?,
                None => usize::MAX,
            };
            let mut work: Vec<_> = data.iter().rev().map(|e| (e, 0usize)).collect();
            let mut output = vec![];
            while let Some((e, depth)) = work.pop() {
                ctx.tick()?;
                if e.is_head(B::LIST) && depth < limit {
                    work.extend(e.args().iter().rev().map(|e| (e, depth + 1)));
                } else {
                    output.push(e.clone());
                }
            }
            list(output)
        }
        "Zip" => {
            let rows = args
                .values
                .iter()
                .map(|e| values(e))
                .collect::<Result<Vec<_>, _>>()?;
            if rows.iter().any(|r| r.len() != data.len()) {
                return Err(error("zip列表长度必须相同"));
            }
            let mut output = vec![];
            for i in 0..data.len() {
                ctx.tick()?;
                output.push(list(rows.iter().map(|r| r[i].clone())));
            }
            list(output)
        }
        "Fold" => unreachable!("invariant: fold returned before data validation"),
        "Counts" | "GroupBy" => {
            if name == "GroupBy" && args.values.len() != 2 {
                return Err(error("需要列表和键函数"));
            }
            let mut groups: Vec<(Expr, Vec<Expr>)> = vec![];
            let mut positions = std::collections::HashMap::new();
            for e in data {
                ctx.tick()?;
                let key = if name == "Counts" {
                    e.clone()
                } else {
                    ev.evaluate(&Expr::normal(args.values[1].clone(), [e.clone()]), ctx)?
                };
                let index = if let Some(index) = positions.get(&key) {
                    *index
                } else {
                    let index = groups.len();
                    positions.insert(key.clone(), index);
                    groups.push((key, vec![]));
                    index
                };
                groups[index].1.push(e.clone());
            }
            list(groups.into_iter().map(|(key, rows)| {
                Expr::call(
                    B::RECORD,
                    [
                        Expr::call(B::RULE, [Expr::string("key"), key]),
                        Expr::call(
                            B::RULE,
                            [
                                Expr::string(if name == "Counts" { "count" } else { "values" }),
                                if name == "Counts" {
                                    Expr::int(rows.len() as i64)
                                } else {
                                    list(rows)
                                },
                            ],
                        ),
                    ],
                )
            }))
        }
        "Reshape" => {
            if args.values.len() != 2 {
                return Err(error("reshape需要数据和维度列表"));
            }
            let dims = values(args.values[1])?
                .iter()
                .map(|e| usize::try_from(count(e)?).map_err(|_| error("维度须非负")))
                .collect::<Result<Vec<_>, _>>()?;
            if dims.len() > 8 || dims.iter().any(|n| *n > 100_000) {
                return Err(error("reshape维度超过资源界限"));
            }
            let mut product = 1usize;
            let mut nodes = 0usize;
            for n in &dims {
                product = product
                    .checked_mul(*n)
                    .ok_or_else(|| error("维度乘积溢出"))?;
                nodes = nodes
                    .checked_add(product)
                    .ok_or_else(|| error("节点数量溢出"))?;
                if nodes > 100_000 {
                    return Err(error("reshape节点超过资源界限"));
                }
            }
            let mut work: Vec<_> = data.iter().rev().collect();
            let mut flat = vec![];
            while let Some(e) = work.pop() {
                ctx.tick()?;
                if e.is_head(B::LIST) {
                    work.extend(e.args().iter().rev());
                } else {
                    flat.push(e.clone());
                }
            }
            if product != flat.len() {
                return Err(error("维度乘积与数据项数不符"));
            }
            fn build(
                flat: &[Expr],
                dims: &[usize],
                offset: &mut usize,
                ctx: &Interrupt,
            ) -> Result<Expr, EvalError> {
                ctx.tick()?;
                if dims.is_empty() {
                    let value = flat[*offset].clone();
                    *offset += 1;
                    return Ok(value);
                }
                let mut out = vec![];
                for _ in 0..dims[0] {
                    out.push(build(flat, &dims[1..], offset, ctx)?);
                }
                Ok(list(out))
            }
            build(&flat, &dims, &mut 0, ctx)?
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}
