//! Named options are validated from actual executable metadata, without evaluating arguments.
use crate::{Fix, Severity, Span, parser::Parser};
use om_core::{
    Expr, ExprKind, Symbol,
    catalog::{self, ParameterSchema, ParameterType},
};

impl Parser<'_> {
    pub(super) fn named_option(
        &mut self,
        function: Symbol,
        name: &str,
        value: &Expr,
        span: Span,
    ) -> Result<Symbol, ()> {
        let Some(descriptor) = catalog::by_runtime(function.name()) else {
            return Ok(crate::names::option(name));
        };
        let candidate = crate::names::option(name);
        if descriptor
            .compatibility_syntax
            .iter()
            .any(|s| s.eq_ignore_ascii_case(name))
        {
            return Ok(candidate);
        }
        let Some(option) = descriptor.options.iter().find(|p| {
            p.name.eq_ignore_ascii_case(name) || p.runtime_name.as_deref() == Some(candidate.name())
        }) else {
            let fix = descriptor
                .options
                .iter()
                .min_by_key(|p| distance(name, &p.name))
                .filter(|p| distance(name, &p.name) <= 3)
                .map(|p| Fix {
                    span,
                    replacement: p.name.clone(),
                    label: format!("改为 {}", p.name),
                });
            self.report(
                span,
                Severity::Error,
                "E024",
                &format!("{} 不支持参数 {name}", descriptor.modern_name),
                fix,
            );
            return Err(());
        };
        if !literal_valid(option, value) {
            self.report(
                span,
                Severity::Error,
                "E025",
                &format!("参数 {} 的值不符合类型、枚举或范围要求", option.name),
                None,
            );
            return Err(());
        }
        Ok(option
            .runtime_name
            .as_deref()
            .map(Symbol::intern)
            .unwrap_or(candidate))
    }
}
fn literal_valid(p: &ParameterSchema, value: &Expr) -> bool {
    let spelling = match value.kind() {
        ExprKind::String(s) => Some(s.as_ref()),
        _ => value.as_symbol().map(Symbol::name),
    };
    if spelling.is_some_and(|s| p.enum_values.iter().any(|a| a == s)) {
        return true;
    }
    match (&p.value_type, value.kind()) {
        // User symbols and compound expressions must be checked by the callback after evaluation.
        (_, ExprKind::Symbol(_)) | (_, ExprKind::Normal(_)) => true,
        (ParameterType::Expression, _) => true,
        (ParameterType::Enum, ExprKind::String(_)) => false,
        (ParameterType::Integer, ExprKind::Number(om_num::Number::Integer(n))) => {
            let numeric = n.to_string().parse::<f64>().unwrap_or(f64::INFINITY);
            p.min.is_none_or(|min| numeric >= min) && p.max.is_none_or(|max| numeric <= max)
        }
        (ParameterType::Real, ExprKind::Number(n)) => {
            let Some(numeric) = n.to_f64() else {
                return false;
            };
            numeric.is_finite()
                && p.min.is_none_or(|min| numeric >= min)
                && p.max.is_none_or(|max| numeric <= max)
        }
        _ => false,
    }
}
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<_> = b.chars().collect();
    let mut row: Vec<_> = (0..=b.len()).collect();
    for (i, ac) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, bc) in b.iter().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j] + 1)
                .min(old + 1)
                .min(diagonal + usize::from(ac != *bc));
            diagonal = old;
        }
    }
    row[b.len()]
}
