//! Ranked executable names, actual live symbols and supported lexical call options.
use crate::protocol::{CompletionItem, CompletionKind};
use om_core::Symbol;
use om_eval::{DocEntry, Evaluator};
use om_parse::Dialect;
use std::collections::BTreeSet;
pub(crate) fn signature(doc: &DocEntry, dialect: Dialect) -> String {
    if dialect == Dialect::Wolfram {
        doc.wolfram.into()
    } else if let Some(tail) = doc.modern.strip_prefix(doc.name) {
        format!("{}{tail}", om_parse::modern_name(Symbol::intern(doc.name)))
    } else {
        doc.modern.into()
    }
}
fn initials(name: &str) -> String {
    let mut first = true;
    let mut result = String::new();
    for c in name.chars() {
        if c == '_' {
            first = true;
            continue;
        }
        if first || c.is_uppercase() {
            result.extend(c.to_lowercase());
        }
        first = false;
    }
    result
}
fn score(label: &str, original: &str, prefix: &str) -> Option<u8> {
    let query = prefix.to_lowercase();
    let label = label.to_lowercase();
    if label.starts_with(&query) {
        return Some(0);
    }
    if initials(original).starts_with(&query) || initials(&label).starts_with(&query) {
        return Some(1);
    }
    let mut chars = label.chars();
    if query.chars().all(|q| chars.by_ref().any(|c| c == q)) {
        Some(2)
    } else {
        None
    }
}
fn options(symbol: Symbol, dialect: Dialect) -> Vec<(String, String)> {
    let name = symbol.name();
    let mut options = vec![];
    if matches!(
        name,
        "Solve" | "NSolve" | "SolveValues" | "NSolveValues" | "Reduce" | "Roots"
    ) {
        if dialect == Dialect::Modern {
            options.push(("domain", ""));
        }
        options.extend([
            ("cubics", "Cubics"),
            ("quartics", "Quartics"),
            ("verify_solutions", "VerifySolutions"),
            ("max_extra_conditions", "MaxExtraConditions"),
            ("generated_parameters", "GeneratedParameters"),
            ("inverse_functions", "InverseFunctions"),
        ]);
    }
    if matches!(name, "NSolve" | "NSolveValues" | "FindRoot") {
        options.push(("precision", "WorkingPrecision"));
    }
    if name == "FindRoot" {
        options.extend([("method", "Method"), ("max_iterations", "MaxIterations")]);
    }
    if name == "Plot" {
        options.push(("plot_range", "PlotRange"));
    }
    options
        .into_iter()
        .map(|(modern, wolfram)| {
            if dialect == Dialect::Modern {
                (modern.into(), format!("{modern}: "))
            } else {
                (wolfram.into(), format!("{wolfram} -> "))
            }
        })
        .collect()
}
pub(crate) fn items(
    prefix: &str,
    dialect: Dialect,
    defined: &BTreeSet<Symbol>,
    context: Option<Symbol>,
) -> Vec<CompletionItem> {
    let mut ranked = vec![];
    let mut add = |item: CompletionItem, original: &str, priority: u8| {
        if let Some(score) = score(&item.label, original, prefix) {
            ranked.push((score, priority, item));
        }
    };
    for doc in Evaluator::all_docs() {
        let label = if dialect == Dialect::Modern {
            om_parse::modern_name(Symbol::intern(doc.name))
        } else {
            doc.name.into()
        };
        let insert_text = label.clone();
        let label =
            if dialect == Dialect::Modern && matches!(doc.name, "And" | "Or" | "Not" | "Root") {
                label.to_ascii_lowercase()
            } else {
                label
            };
        add(
            CompletionItem {
                insert_text,
                label,
                detail: Some(signature(doc, dialect)),
                kind: CompletionKind::Function,
            },
            doc.name,
            match doc.category {
                "Solving" => 1,
                "Algebra" => 2,
                _ => 3,
            },
        );
    }
    for symbol in defined {
        add(
            CompletionItem {
                label: symbol.name().into(),
                insert_text: symbol.name().into(),
                detail: None,
                kind: CompletionKind::Symbol,
            },
            symbol.name(),
            4,
        );
    }
    if dialect == Dialect::Modern {
        for keyword in ["let", "where", "and", "or", "not"] {
            add(
                CompletionItem {
                    label: keyword.into(),
                    insert_text: format!("{keyword} "),
                    detail: None,
                    kind: CompletionKind::Keyword,
                },
                keyword,
                5,
            );
        }
    }
    let (label, insert_text) = if dialect == Dialect::Modern {
        ("solve", "solve(${1:equation}, ${2:x})")
    } else {
        ("Solve", "Solve[${1:equation}, ${2:x}]")
    };
    add(
        CompletionItem {
            label: label.into(),
            insert_text: insert_text.into(),
            detail: Some("Solve".into()),
            kind: CompletionKind::Snippet,
        },
        "Solve",
        0,
    );
    if let Some(context) = context {
        for (label, insert_text) in options(context, dialect) {
            add(
                CompletionItem {
                    detail: None,
                    label: label.clone(),
                    insert_text,
                    kind: CompletionKind::Keyword,
                },
                &label,
                0,
            );
        }
    }
    let kind = |k: CompletionKind| match k {
        CompletionKind::Snippet => 0,
        CompletionKind::Function => 1,
        CompletionKind::Keyword => 2,
        CompletionKind::Symbol => 3,
    };
    ranked.sort_by(|a, b| {
        (a.0, a.1, &a.2.label, kind(a.2.kind), &a.2.insert_text).cmp(&(
            b.0,
            b.1,
            &b.2.label,
            kind(b.2.kind),
            &b.2.insert_text,
        ))
    });
    let mut seen = BTreeSet::new();
    ranked
        .into_iter()
        .filter_map(|(_, _, item)| {
            seen.insert((
                item.label.clone(),
                item.insert_text.clone(),
                kind(item.kind),
            ))
            .then_some(item)
        })
        .take(50)
        .collect()
}
