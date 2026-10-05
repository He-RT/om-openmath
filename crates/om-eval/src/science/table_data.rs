//! Tables operate on ordered record rows. Subsets preserve columns; map/zip/fold retain their value semantics.
use super::*;
use std::cmp::Ordering;
#[derive(PartialEq, PartialOrd)]
enum Cell {
    Number(Rational),
    Text(String),
    Boolean(bool),
    Null,
}
fn key(e: &Expr) -> Result<Cell, EvalError> {
    match e.kind() {
        ExprKind::Number(_) => Ok(Cell::Number(rational(e)?)),
        ExprKind::String(s) => Ok(Cell::Text(s.to_string())),
        ExprKind::Symbol(s) if *s == B::TRUE || *s == B::FALSE => Ok(Cell::Boolean(*s == B::TRUE)),
        ExprKind::Symbol(s) if *s == B::NULL => Ok(Cell::Null),
        _ => Err(error(
            "记录排序的字段需要有限实数、字符串、布尔或Null；复杂字段请提供标量排序键",
        )),
    }
}
pub(super) fn record_sort(
    values: &[Expr],
    keys: &[Expr],
    ctx: &Interrupt,
) -> Result<Expr, EvalError> {
    let schema: Vec<_> = keys
        .first()
        .map(|e| {
            e.args()
                .iter()
                .map(|r| {
                    r.args()
                        .first()
                        .cloned()
                        .ok_or_else(|| error("记录字段无效"))
                })
                .collect::<Result<_, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut prepared = vec![];
    for e in keys {
        let fields = super::csv_data::entries(e, ctx)?;
        if fields.len() != schema.len() {
            return Err(error("记录排序需要相同字段"));
        }
        let mut row = vec![];
        for column in &schema {
            ctx.tick()?;
            row.push(key(fields
                .get(string(column)?)
                .ok_or_else(|| error("记录排序需要相同字段"))?)?);
        }
        prepared.push(row);
    }
    let order = super::ordering::indices_checked(values.len(), ctx, |a, b| {
        for (a, b) in prepared[a].iter().zip(&prepared[b]) {
            ctx.tick()?;
            let order = a.partial_cmp(b).ok_or_else(|| error("记录字段无法比较"))?;
            if order != Ordering::Equal {
                return Ok(order);
            }
        }
        Ok(Ordering::Equal)
    })?;
    Ok(list(order.into_iter().map(|i| values[i].clone())))
}
fn rows(e: &Expr, ctx: &Interrupt) -> Result<Expr, EvalError> {
    super::csv_data::validate_table(e, ctx)?;
    let mut rows = vec![];
    for row in e.args()[1].args() {
        let values = super::csv_data::entries(row, ctx)?;
        let mut fields = vec![];
        for column in e.args()[0].args() {
            ctx.tick()?;
            let value = values
                .get(string(column)?)
                .ok_or_else(|| error("表格字段与列不一致"))?;
            fields.push(Expr::call(B::RULE, [column.clone(), (*value).clone()]));
        }
        rows.push(Expr::call(B::RECORD, fields));
    }
    Ok(list(rows))
}
fn wrap(source: &Expr, rows: Expr) -> Expr {
    Expr::call(B::DATA_TABLE, [source.args()[0].clone(), rows])
}
pub(super) fn field(source: &Expr, key: &str, ctx: &Interrupt) -> Result<Expr, EvalError> {
    let ordered = rows(source, ctx)?;
    match key {
        "columns" => Ok(source.args()[0].clone()),
        "rows" => Ok(ordered),
        _ => {
            if !source.args()[0]
                .args()
                .iter()
                .any(|c| string(c).ok() == Some(key))
            {
                return Err(error("表格没有这个列或属性"));
            }
            let mut values = vec![];
            for row in ordered.args() {
                ctx.tick()?;
                let fields = super::csv_data::entries(row, ctx)?;
                values.push((*fields.get(key).ok_or_else(|| error("表格字段缺失"))?).clone());
            }
            Ok(list(values))
        }
    }
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
            | "Slice"
            | "Flatten"
            | "Reshape"
            | "Zip"
            | "Fold"
            | "GroupBy"
            | "Counts"
            | "Map"
            | "Length"
            | "First"
            | "Last"
            | "Rest"
            | "Append"
    ) || !args.values.iter().any(|e| e.is_head(B::DATA_TABLE))
    {
        return Ok(None);
    }
    let index = match name {
        "Fold" => 2,
        "Map" => 1,
        _ => 0,
    };
    if args
        .values
        .get(index)
        .is_none_or(|e| !e.is_head(B::DATA_TABLE))
        && name != "Zip"
    {
        return Ok(None);
    }
    let mut owned: Vec<_> = args.values.iter().map(|e| (*e).clone()).collect();
    for (i, e) in args.values.iter().enumerate() {
        if e.is_head(B::DATA_TABLE) && (i == index || name == "Zip") {
            owned[i] = rows(e, ctx)?;
        }
    }
    if matches!(
        name,
        "Length" | "First" | "Last" | "Rest" | "Append" | "Map"
    ) {
        let values = owned[index].args();
        let source = args.values[index];
        return Ok(match name {
            "Length" => Some(Expr::int(values.len() as i64)),
            "First" => values.first().cloned(),
            "Last" => values.last().cloned(),
            "Rest" => {
                if values.is_empty() {
                    None
                } else {
                    Some(wrap(source, list(values[1..].iter().cloned())))
                }
            }
            "Append" => {
                let value = wrap(
                    source,
                    list(
                        values
                            .iter()
                            .cloned()
                            .chain(std::iter::once(args.values[1].clone())),
                    ),
                );
                super::csv_data::validate_table(&value, ctx)?;
                Some(value)
            }
            _ => {
                let mut out = vec![];
                for row in values {
                    ctx.tick()?;
                    out.push(
                        ev.evaluate(&Expr::normal(args.values[0].clone(), [row.clone()]), ctx)?,
                    );
                }
                Some(list(out))
            }
        });
    }
    let adapted = Args {
        values: owned.iter().collect(),
        options: args.options.clone(),
    };
    let Some(result) = super::data::dispatch(ev, name, &adapted, ctx)? else {
        return Ok(None);
    };
    Ok(Some(match name {
        "Filter" | "Sort" | "SortBy" | "Unique" | "Take" | "Drop" | "Slice" => {
            wrap(args.values[index], result)
        }
        "GroupBy" => {
            let mut out = vec![];
            for group in result.args() {
                ctx.tick()?;
                out.push(record([
                    ("key", group.args()[0].args()[1].clone()),
                    (
                        "values",
                        wrap(args.values[index], group.args()[1].args()[1].clone()),
                    ),
                ]));
            }
            list(out)
        }
        _ => result,
    }))
}
