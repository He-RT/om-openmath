//! Project retained expression data into bounded pages; no evaluator calls or mutable session access.
use crate::{notebook::StatementRecord, protocol::*};
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt};
use om_num::Precision;
use std::collections::{BTreeMap, BTreeSet};
fn error() -> crate::plot::PlotError {
    crate::plot::PlotError::Invalid("结果数据形状或分页路径无效".into())
}
fn fields(e: &Expr) -> Result<Vec<(&str, &Expr, usize)>, crate::plot::PlotError> {
    let mut out = vec![];
    let mut names = BTreeSet::new();
    if !e.is_head(B::RECORD) {
        return Err(error());
    }
    for (i, p) in e.args().iter().enumerate() {
        if !p.is_head(B::RULE) || p.args().len() != 2 {
            return Err(error());
        }
        let ExprKind::String(key) = p.args()[0].kind() else {
            return Err(error());
        };
        if !names.insert(key.as_ref()) {
            return Err(error());
        }
        out.push((key.as_ref(), &p.args()[1], i));
    }
    Ok(out)
}
pub(crate) fn kind(e: &Expr) -> ValueKind {
    if e.is_head(B::DATA_TABLE) {
        ValueKind::Table
    } else if e.is_head(B::RECORD) {
        ValueKind::Record
    } else if e.is_head(B::FITTED_MODEL_DATA) {
        ValueKind::Model
    } else if e.is_head(B::INTERPOLATION_DATA) {
        ValueKind::Interpolation
    } else if e.is_head(B::SERIES_DATA) {
        ValueKind::Series
    } else if e.head_symbol().is_some_and(|s| s.name() == "Quantity") {
        ValueKind::Quantity
    } else if e.is_head(B::LIST) {
        let cols = e.args().first().map_or(0, |e| e.args().len());
        if cols > 0
            && e.args()
                .iter()
                .all(|r| r.is_head(B::LIST) && r.args().len() == cols)
        {
            ValueKind::Matrix
        } else {
            ValueKind::List
        }
    } else {
        ValueKind::Scalar
    }
}
fn nature(e: &Expr) -> ValueNature {
    match e.kind() {
        ExprKind::Number(n) => match n.precision() {
            Precision::Exact => ValueNature::Exact,
            Precision::Machine => ValueNature::Machine,
            Precision::Bits(_) => ValueNature::HighPrecision,
        },
        ExprKind::String(_) => ValueNature::Text,
        ExprKind::Symbol(B::TRUE | B::FALSE) => ValueNature::Boolean,
        ExprKind::Symbol(B::NULL) => ValueNature::Null,
        _ => ValueNature::Symbolic,
    }
}
fn count(e: &Expr, k: ValueKind) -> usize {
    match k {
        ValueKind::Table => e.args().get(1).map_or(0, |r| r.args().len()),
        ValueKind::Model | ValueKind::Interpolation => {
            e.args().first().map_or(0, |r| r.args().len())
        }
        ValueKind::Series => e.args().get(2).map_or(0, |r| r.args().len()),
        ValueKind::Quantity => 1,
        ValueKind::Scalar => 0,
        _ => e.args().len(),
    }
}
fn identity(view_id: &str, path: &[u32]) -> String {
    format!(
        "{view_id}:{}",
        path.iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(".")
    )
}
fn entry(
    e: &Expr,
    path: Vec<u32>,
    view_id: &str,
    ctx: &Interrupt,
) -> Result<ValueEntry, crate::plot::PlotError> {
    ctx.tick()?;
    let kind = kind(e);
    let source = (kind == ValueKind::Scalar).then(|| super::expression_view(e));
    Ok(ValueEntry {
        id: identity(view_id, &path),
        path,
        kind,
        nature: nature(e),
        count: count(e, kind) as u32,
        source,
    })
}
/// Build a page for a verified owner/snapshot. Paths are data references, never mathematical evaluation.
pub(crate) fn page(
    record: &StatementRecord,
    query: &ValueQuery,
    ctx: &Interrupt,
) -> Result<ValuePage, crate::plot::PlotError> {
    let path = query.path.clone();
    let (offset, limit, column_offset, column_limit, include_source) = (
        query.offset,
        query.limit,
        query.column_offset,
        query.column_limit,
        query.include_source,
    );
    ctx.tick()?;
    if path.len() > 32 || !(1..=100).contains(&limit) || !(1..=32).contains(&column_limit) {
        return Err(error());
    }
    let mut value = &record.value;
    for part in &path {
        ctx.tick()?;
        value = value.args().get(*part as usize).ok_or_else(error)?;
    }
    let k = kind(value);
    let mut columns = vec![];
    let mut rows = vec![];
    let (total, width) = match k {
        ValueKind::Table => {
            if value.args().len() != 2
                || !value.args()[0].is_head(B::LIST)
                || !value.args()[1].is_head(B::LIST)
            {
                return Err(error());
            }
            let mut unique = BTreeSet::new();
            for c in value.args()[0].args() {
                ctx.tick()?;
                let ExprKind::String(s) = c.kind() else {
                    return Err(error());
                };
                if !unique.insert(s.to_string()) {
                    return Err(error());
                }
                columns.push(s.to_string());
            }
            for r in value.args()[1].args() {
                ctx.tick()?;
                let f = fields(r)?;
                if f.len() != columns.len() || f.iter().any(|(key, _, _)| !unique.contains(*key)) {
                    return Err(error());
                }
            }
            (value.args()[1].args().len(), columns.len())
        }
        ValueKind::Record => {
            fields(value)?;
            columns.push("value".into());
            (value.args().len(), 1)
        }
        ValueKind::Matrix => {
            let width = value.args()[0].args().len();
            columns = (1..=width).map(|i| i.to_string()).collect();
            (value.args().len(), width)
        }
        ValueKind::List => {
            columns.push("value".into());
            (value.args().len(), 1)
        }
        _ => (0, 0),
    };
    if offset as usize > total || column_offset as usize > width {
        return Err(error());
    }
    let end = (offset as usize + limit as usize).min(total);
    let cend = (column_offset as usize + column_limit as usize).min(width);
    for i in offset as usize..end {
        ctx.tick()?;
        let mut cells = vec![];
        let mut row_path = path.clone();
        let label;
        match k {
            ValueKind::Table => {
                row_path.extend([1, i as u32]);
                label = (i + 1).to_string();
                let fields = fields(&value.args()[1].args()[i])?;
                let map: BTreeMap<_, _> = fields
                    .iter()
                    .map(|(key, e, index)| (*key, (*e, *index)))
                    .collect();
                for key in &columns[column_offset as usize..cend] {
                    let (cell, index) = map.get(key.as_str()).ok_or_else(error)?;
                    let mut p = row_path.clone();
                    p.extend([*index as u32, 1]);
                    cells.push(entry(cell, p, &record.view_id, ctx)?);
                }
            }
            ValueKind::Record => {
                let fields = fields(value)?;
                let (key, cell, index) = fields[i];
                label = key.into();
                row_path.extend([index as u32, 1]);
                if column_offset == 0 {
                    cells.push(entry(cell, row_path.clone(), &record.view_id, ctx)?);
                }
            }
            ValueKind::Matrix => {
                label = (i + 1).to_string();
                row_path.push(i as u32);
                for j in column_offset as usize..cend {
                    let mut p = row_path.clone();
                    p.push(j as u32);
                    cells.push(entry(&value.args()[i].args()[j], p, &record.view_id, ctx)?);
                }
            }
            ValueKind::List => {
                label = (i + 1).to_string();
                row_path.push(i as u32);
                if column_offset == 0 {
                    cells.push(entry(
                        &value.args()[i],
                        row_path.clone(),
                        &record.view_id,
                        ctx,
                    )?);
                }
            }
            _ => return Err(error()),
        }
        rows.push(ValueRow {
            id: identity(&record.view_id, &row_path),
            label,
            cells,
        });
    }
    let origin = record
        .scientific
        .as_ref()
        .and_then(|r| om_core::catalog::by_runtime(r.name))
        .map(|d| ScientificOrigin {
            function_id: d.id.clone(),
            name: d.modern_name.clone(),
        });
    Ok(ValuePage {
        view_id: record.view_id.clone(),
        path,
        kind: k,
        row_count: total as u32,
        column_count: width as u32,
        offset,
        column_offset,
        columns: columns[column_offset as usize..cend].to_vec(),
        rows,
        source: (include_source || total == 0).then(|| super::expression_view(value)),
        origin,
    })
}
