//! Exact UTF-8 diagnostic spans and repairs that are verified by reparsing.

use om_core::{Expr, Symbol};
use om_parse::{Dialect, ParseEnv, Severity, Span, parse, parse_with};

fn repaired(src: &str, fix: &om_parse::Fix) -> String {
    format!(
        "{}{}{}",
        &src[..fix.span.start as usize],
        fix.replacement,
        &src[fix.span.end as usize..]
    )
}

#[test]
fn modern_ambiguity_diagnostics_point_to_the_exact_source_problem() {
    for (src, code, span, severity) in [
        ("f (x)", "W001", Span { start: 1, end: 2 }, Severity::Hint),
        ("α  (x)", "W001", Span { start: 2, end: 4 }, Severity::Hint),
        ("a(b+c)", "W002", Span { start: 0, end: 1 }, Severity::Hint),
        ("α(b+c)", "W002", Span { start: 0, end: 2 }, Severity::Hint),
        ("sin x", "E010", Span { start: 0, end: 5 }, Severity::Error),
        ("sin α", "E010", Span { start: 0, end: 6 }, Severity::Error),
        (
            "%modern\nsin α",
            "E010",
            Span { start: 8, end: 14 },
            Severity::Error,
        ),
    ] {
        let out = parse(src, Dialect::Auto);
        let d = out.diagnostics.iter().find(|d| d.code == code).unwrap();
        assert_eq!(d.span, span, "{src}");
        assert_eq!(d.severity, severity);
        let fix = d.fix.as_ref().unwrap();
        let source = repaired(src, fix);
        assert!(
            !parse(&source, Dialect::Modern)
                .diagnostics
                .iter()
                .any(|d| d.code == code || d.severity == Severity::Error),
            "{source}"
        );
        if code == "E010" {
            assert_eq!(d.message, format!("函数需要括号：{}", fix.replacement));
        }
    }
}

#[test]
fn bracket_diagnostics_are_precise_and_a_single_fix_completes_the_grouping() {
    for (src, dialect, code, span, expected) in [
        (
            "[",
            Dialect::Modern,
            "E023",
            Span { start: 0, end: 1 },
            "[]",
        ),
        (
            "f(",
            Dialect::Modern,
            "E023",
            Span { start: 1, end: 2 },
            "f()",
        ),
        (
            "f[",
            Dialect::Wolfram,
            "E023",
            Span { start: 1, end: 2 },
            "f[]",
        ),
        (
            "|α",
            Dialect::Modern,
            "E023",
            Span { start: 0, end: 1 },
            "|α|",
        ),
        (
            "(α",
            Dialect::Modern,
            "E023",
            Span { start: 0, end: 1 },
            "(α)",
        ),
        (
            "f([α",
            Dialect::Modern,
            "E023",
            Span { start: 2, end: 3 },
            "f([α])",
        ),
        (
            "v[[α",
            Dialect::Wolfram,
            "E023",
            Span { start: 2, end: 3 },
            "v[[α]]",
        ),
        (
            "(α]",
            Dialect::Modern,
            "E024",
            Span { start: 3, end: 4 },
            "(α)",
        ),
        (
            "α)",
            Dialect::Modern,
            "E024",
            Span { start: 2, end: 3 },
            "α",
        ),
        (
            "f[x}",
            Dialect::Wolfram,
            "E024",
            Span { start: 3, end: 4 },
            "f[x]",
        ),
        (
            "%wl\nf[α",
            Dialect::Auto,
            "E023",
            Span { start: 5, end: 6 },
            "%wl\nf[α]",
        ),
    ] {
        let out = parse(src, dialect);
        let errors: Vec<_> = out
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert_eq!(errors.len(), 1, "{src}: {:?}", out.diagnostics);
        assert_eq!(errors[0].code, code);
        assert_eq!(errors[0].span, span);
        let source = repaired(src, errors[0].fix.as_ref().unwrap());
        assert_eq!(source, expected);
        assert!(
            !parse(&source, dialect)
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{source}"
        );
    }
}

#[test]
fn comments_and_strings_do_not_confuse_bracket_repair() {
    for (src, expected) in [
        ("f(α # ignored ) ]", "f(α # ignored ) ]\n)"),
        ("[\"(\", α", "[\"(\", α]"),
        ("f(α (* ] } *)", "f(α (* ] } *))"),
    ] {
        let out = parse(src, Dialect::Modern);
        let d = out.diagnostics.iter().find(|d| d.code == "E023").unwrap();
        let source = repaired(src, d.fix.as_ref().unwrap());
        assert_eq!(source, expected);
        assert!(
            !parse(&source, Dialect::Modern)
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error)
        );
    }
}

#[test]
fn unterminated_literals_repair_without_losing_content() {
    for (src, code, expected, expr) in [
        ("\"α", "E002", "\"α\"", Expr::string("α")),
        ("\"α\\", "E002", "\"α\\\\\"", Expr::string("α\\")),
        (
            "x (* outer (* inner",
            "E003",
            "x (* outer (* inner*)*)",
            Expr::symbol("x"),
        ),
    ] {
        let out = parse(src, Dialect::Modern);
        let d = out.diagnostics.iter().find(|d| d.code == code).unwrap();
        assert_eq!(d.span.end, src.len() as u32);
        let source = repaired(src, d.fix.as_ref().unwrap());
        assert_eq!(source, expected);
        let result = parse(&source, Dialect::Modern);
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error)
        );
        assert_eq!(result.statements[0].expr, expr);
    }
}

#[test]
fn known_function_fixes_keep_the_callers_environment_and_comments_are_preserved() {
    let mut env = ParseEnv::default();
    env.known_functions.insert(Symbol::intern("α"));
    let src = "α (x)";
    let out = parse_with(src, Dialect::Modern, &env);
    let fix = out
        .diagnostics
        .iter()
        .find(|d| d.code == "W001")
        .unwrap()
        .fix
        .as_ref()
        .unwrap();
    let source = repaired(src, fix);
    assert!(
        parse_with(&source, Dialect::Modern, &env)
            .diagnostics
            .is_empty()
    );
    let out = parse("f (* keep this *)(x)", Dialect::Modern);
    assert!(!out.diagnostics.iter().any(|d| {
        d.fix
            .as_ref()
            .is_some_and(|f| f.span.start == 1 && f.replacement.is_empty())
    }));
}
