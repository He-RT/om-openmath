//! Terminal projections use actual kernel strings and verification evidence.
use om_kernel::{config::Language, protocol::*};
use std::io::{self, Write};
pub fn text(zh: bool, chinese: &str, english: &str) -> String {
    if zh { chinese } else { english }.into()
}
pub fn unicode(source: &str) -> String {
    om_parse::parse_expr(source, om_parse::Dialect::Wolfram)
        .map(|e| om_format::unicode_form(&e))
        .unwrap_or_else(|_| source.into())
}
pub fn output(output: &CellOutput, json: bool, labels: bool, zh: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if json {
        serde_json::to_writer(&mut out, output)?;
        writeln!(out)?;
        return Ok(());
    }
    for item in &output.items {
        match item {
            OutputItem::Expr {
                out_index,
                input_form,
                ..
            } => writeln!(
                out,
                "{}{}",
                if labels {
                    format!("Out[{out_index}]= ")
                } else {
                    String::new()
                },
                unicode(input_form)
            )?,
            OutputItem::Solutions {
                out_index,
                view,
                input_form,
                ..
            } => {
                let body = match view.kind {
                    SolutionKind::None => text(zh, "无解", "No solutions"),
                    SolutionKind::All => text(zh, "对所有值成立", "All values"),
                    SolutionKind::Region => unicode(input_form),
                    SolutionKind::Finite => {
                        let rows = view
                            .solutions
                            .iter()
                            .map(|s| {
                                s.bindings
                                    .iter()
                                    .map(|b| format!("{} = {}", b.var, unicode(&b.input_form)))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .collect::<Vec<_>>()
                            .join("  │  ");
                        let verified = view.solutions.iter().all(|s| {
                            matches!(
                                s.verified,
                                Verification::Exact | Verification::ByConstruction
                            )
                        });
                        format!(
                            "{rows}  ({} {}, {})",
                            view.solutions.len(),
                            text(zh, "个解", "solutions"),
                            if verified {
                                text(zh, "已验证", "verified")
                            } else {
                                text(zh, "参见验证信息", "see verification evidence")
                            }
                        )
                    }
                };
                writeln!(
                    out,
                    "{}{}",
                    if labels {
                        format!("Out[{out_index}]= ")
                    } else {
                        String::new()
                    },
                    body
                )?;
            }
            OutputItem::Error { message, .. } => writeln!(out, "{message}")?,
            OutputItem::Plot { data, .. } => writeln!(
                out,
                "{}: {}",
                text(zh, "已采样曲线", "Sampled curves"),
                data.curves.len()
            )?,
        }
    }
    for message in &output.messages {
        eprintln!("{}::{} — {}", message.symbol, message.tag, message.text);
    }
    Ok(())
}
pub fn diagnostics(source: &str, items: &[Diagnostic], zh: bool) {
    for item in items.iter().filter(|d| d.severity == Severity::Error) {
        let start = source
            .get(..item.span.start as usize)
            .map_or(0, |s| s.chars().count());
        let end = source
            .get(..item.span.end as usize)
            .map_or(start, |s| s.chars().count());
        let message = if zh {
            item.message.clone()
        } else {
            match item.code.as_str() {
                "E023" => "Missing closing bracket".into(),
                "E024" => "Unexpected closing bracket".into(),
                _ => format!("Parse error {}: {}", item.code, item.message),
            }
        };
        let report = ariadne::Report::build(ariadne::ReportKind::Error, ("input", start..end))
            .with_code(&item.code)
            .with_message(&message)
            .with_label(ariadne::Label::new(("input", start..end)).with_message(&message))
            .finish();
        let _ = report.eprint(("input", ariadne::Source::from(source)));
    }
}
pub fn steps(steps: &StepsView, zh: bool) {
    let mut work = steps.root.iter().rev().map(|s| (s, 0)).collect::<Vec<_>>();
    while let Some((step, depth)) = work.pop() {
        println!(
            "{}{} {}",
            "  ".repeat(depth),
            step.id,
            title(&step.rule_id, zh)
        );
        for (k, v) in &step.params {
            println!("{}  {k}: {v}", "  ".repeat(depth));
        }
        for value in &step.before_latex {
            println!("{}  {value}", "  ".repeat(depth));
        }
        if !step.after_latex.is_empty() {
            println!("{}  → {}", "  ".repeat(depth), step.after_latex.join(", "));
        }
        work.extend(step.children.iter().rev().map(|s| (s, depth + 1)));
    }
}
fn title(rule: &str, zh: bool) -> String {
    let words = match rule {
        "normalize" => ("整理方程", "Normalize equations"),
        "factor" => ("因式分解", "Factor polynomial"),
        "zero_product" => ("零乘积法则", "Zero-product rule"),
        "verify" => ("验证候选解", "Verify candidates"),
        "domain_filter" => ("定义域筛选", "Filter domain"),
        "square_free" => ("无平方分解", "Square-free factors"),
        "substitute" => ("代换", "Substitution"),
        "back_substitute" => ("回代", "Back substitution"),
        "quadratic_formula" => ("二次求根公式", "Quadratic formula"),
        "cardano_formula" => ("三次求根公式", "Cubic formula"),
        "ferrari_formula" => ("四次求根公式", "Quartic formula"),
        "binomial_formula" => ("二项式求根", "Binomial roots"),
        "branch" => ("选择分支", "Select branch"),
        "clear_denominators" => ("清除分母", "Clear denominators"),
        "discriminant" => ("计算判别式", "Compute discriminant"),
        "drop_extraneous" => ("排除增根", "Discard extraneous roots"),
        "eliminant" => ("求解消元多项式", "Solve eliminant"),
        "expand" => ("展开表达式", "Expand expression"),
        "generic_assumption" => ("记录一般参数条件", "Record generic assumptions"),
        "groebner" => ("计算 Gröbner 基", "Compute Gröbner basis"),
        "invert_function" => ("反解函数", "Invert function"),
        "isolate_term" => ("隔离未知项", "Isolate term"),
        "linear_formula" => ("一次方程求解", "Linear formula"),
        "note" => ("求解说明", "Solver note"),
        "palindromic_formula" => ("对称多项式代换", "Palindromic substitution"),
        "raise_to_power" => ("两边乘方", "Raise both sides to a power"),
        "record_exclusion" => ("记录原始定义域限制", "Record source exclusions"),
        "resultant" => ("计算结式", "Compute resultant"),
        "root_objects" => ("构造精确代数根", "Construct exact algebraic roots"),
        "row_reduce" => ("消元与行化简", "Row reduction"),
        "sign_chart" => ("构建符号表", "Build sign chart"),
        "split_component" => ("分别求解因子", "Solve factor components"),
        _ => return rule.into(),
    };
    text(zh, words.0, words.1)
}
pub fn zh(language: Language) -> bool {
    language == Language::ZhCn
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_actual_solver_rule_has_bilingual_terminal_titles() {
        for rule in include_str!("../../om-solve/rule_ids.txt").lines() {
            assert_ne!(title(rule, false), rule, "missing English title for {rule}");
            assert_ne!(title(rule, true), rule, "missing Chinese title for {rule}");
        }
    }
}
