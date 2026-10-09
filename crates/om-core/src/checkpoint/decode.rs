use super::{ExprCodecError, ExprLimits, Reader};
use crate::{Expr, Interrupt, Symbol, builtins};
use om_num::{Number, checkpoint::decode_number};
use std::collections::BTreeSet;
enum Node<'a> {
    Number(Number),
    Symbol(&'a str),
    String(&'a str),
    Normal(usize, Vec<usize>),
}
/// Validate the entire flat graph and roots before interning symbols or constructing expressions.
pub fn decode_expressions(
    bytes: &[u8],
    limits: ExprLimits,
    ctx: &Interrupt,
) -> Result<Vec<Expr>, ExprCodecError> {
    ctx.tick()?;
    if bytes.len() > limits.max_bytes {
        return Err(ExprCodecError::Limit);
    }
    let mut reader = Reader { bytes, position: 0 };
    if reader.take(5)? != b"OMEX\x01" {
        return Err(ExprCodecError::Invalid);
    }
    let roots = reader.size()?;
    let count = reader.size()?;
    if roots > limits.max_roots || count > limits.max_nodes {
        return Err(ExprCodecError::Limit);
    }
    if count
        .checked_add(roots.checked_mul(4).ok_or(ExprCodecError::Invalid)?)
        .is_none_or(|n| n > bytes.len() - reader.position)
    {
        return Err(ExprCodecError::Invalid);
    }
    let mut nodes = Vec::with_capacity(count);
    let mut heights = Vec::with_capacity(count);
    let mut symbols = BTreeSet::new();
    let mut edges = 0usize;
    for index in 0..count {
        ctx.tick()?;
        let mut height = 1;
        let node = match reader.take(1)?[0] {
            0 => Node::Number(decode_number(reader.blob()?, limits.numbers, ctx)?),
            tag @ (1 | 2) => {
                let name = text(reader.blob()?, limits)?;
                if builtins::names().contains(&name) != (tag == 1) {
                    return Err(ExprCodecError::Invalid);
                }
                symbols.insert(name);
                if symbols.len() > limits.max_symbols {
                    return Err(ExprCodecError::Limit);
                }
                Node::Symbol(name)
            }
            3 => Node::String(text(reader.blob()?, limits)?),
            4 => {
                let head = reader.size()?;
                let args = reader.size()?;
                if head >= index {
                    return Err(ExprCodecError::Invalid);
                }
                edges = edges
                    .checked_add(args.checked_add(1).ok_or(ExprCodecError::Limit)?)
                    .filter(|&n| n <= limits.max_edges)
                    .ok_or(ExprCodecError::Limit)?;
                if args > (bytes.len() - reader.position) / 4 {
                    return Err(ExprCodecError::Invalid);
                }
                height = heights[head] + 1;
                let mut refs = Vec::with_capacity(args);
                for _ in 0..args {
                    ctx.tick()?;
                    let id = reader.size()?;
                    if id >= index {
                        return Err(ExprCodecError::Invalid);
                    }
                    height = height.max(heights[id] + 1);
                    refs.push(id);
                }
                Node::Normal(head, refs)
            }
            _ => return Err(ExprCodecError::Invalid),
        };
        if height > limits.max_depth {
            return Err(ExprCodecError::Limit);
        }
        nodes.push(node);
        heights.push(height);
    }
    let mut root_ids = Vec::with_capacity(roots);
    for _ in 0..roots {
        ctx.tick()?;
        let id = reader.size()?;
        if id >= count {
            return Err(ExprCodecError::Invalid);
        };
        root_ids.push(id);
    }
    if reader.position != bytes.len() {
        return Err(ExprCodecError::Invalid);
    }
    let mut reachable = vec![false; count];
    let mut work = root_ids.clone();
    while let Some(id) = work.pop() {
        ctx.tick()?;
        if reachable[id] {
            continue;
        }
        reachable[id] = true;
        if let Node::Normal(head, args) = &nodes[id] {
            work.push(*head);
            work.extend(args.iter().copied());
        }
    }
    if reachable.iter().any(|seen| !seen) {
        return Err(ExprCodecError::Invalid);
    }
    let mut values: Vec<Expr> = Vec::with_capacity(count);
    for node in nodes {
        ctx.tick()?;
        let value = match node {
            Node::Number(n) => Expr::number(n),
            Node::Symbol(s) => Expr::sym(Symbol::intern(s)),
            Node::String(s) => Expr::string(s),
            Node::Normal(head, args) => Expr::normal(
                values[head].clone(),
                args.into_iter().map(|id| values[id].clone()),
            ),
        };
        values.push(value);
    }
    Ok(root_ids.into_iter().map(|id| values[id].clone()).collect())
}
fn text(bytes: &[u8], limits: ExprLimits) -> Result<&str, ExprCodecError> {
    if bytes.len() > limits.max_text_bytes {
        return Err(ExprCodecError::Limit);
    }
    std::str::from_utf8(bytes).map_err(|_| ExprCodecError::Invalid)
}
