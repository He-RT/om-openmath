//! Wolfram subset grammar, precedence and the complete §14 input corpus.

use om_core::{Expr, canonicalize};
use om_parse::{Dialect, Severity, parse, parse_expr};

fn raw(src: &str) -> String {
    format!(
        "{:?}",
        parse_expr(src, Dialect::Wolfram).unwrap_or_else(|d| panic!("{src}: {d:?}"))
    )
}

#[test]
fn the_wolfram_subset_has_raw_expression_vectors() {
    for (src, expected) in [
        ("f[x]", "f[x]"),
        ("f [x, y]", "f[x, y]"),
        ("f[x][y]", "f[x][y]"),
        ("v[[1]]", "Part[v, 1]"),
        ("v[\n[1]\n]", "Part[v, 1]"),
        ("v[[1,2]]", "Part[v, 1, 2]"),
        ("f[x][[1]]", "Part[f[x], 1]"),
        ("{1,{2,3}}", "List[1, List[2, 3]]"),
        ("{}", "List[]"),
        ("x==y", "Equal[x, y]"),
        ("x!=y", "Unequal[x, y]"),
        ("x===y", "SameQ[x, y]"),
        ("x===y===z", "SameQ[x, y, z]"),
        ("0<x<=1", "Inequality[0, Less, x, LessEqual, 1]"),
        ("x<y<z", "Less[x, y, z]"),
        ("x!=y!=z", "Unequal[x, y, z]"),
        ("x&&y||!z", "Or[And[x, y], Not[z]]"),
        ("x->y", "Rule[x, y]"),
        ("x:>y", "RuleDelayed[x, y]"),
        ("x /. x->1", "ReplaceAll[x, Rule[x, 1]]"),
        ("x //. {x->y}", "ReplaceRepeated[x, List[Rule[x, y]]]"),
        ("x=1", "Set[x, 1]"),
        (
            "f[x_]:=x^2",
            "SetDelayed[f[Pattern[x, Blank[]]], Power[x, 2]]",
        ),
        ("x=.", "Unset[x]"),
        ("x_ /; x>0", "Condition[Pattern[x, Blank[]], Greater[x, 0]]"),
        ("_", "Blank[]"),
        ("x_", "Pattern[x, Blank[]]"),
        ("x_Integer", "Pattern[x, Blank[Integer]]"),
        ("x__", "Pattern[x, BlankSequence[]]"),
        ("x___", "Pattern[x, BlankNullSequence[]]"),
        ("__Integer", "BlankSequence[Integer]"),
        (r"_\[Pi]", "Blank[Pi]"),
        ("x___Real", "Pattern[x, BlankNullSequence[Real]]"),
        ("#", "Slot[1]"),
        ("#1", "Slot[1]"),
        ("#12", "Slot[12]"),
        ("#^2&", "Function[Power[Slot[1], 2]]"),
        ("f@x", "f[x]"),
        ("x//f", "f[x]"),
        ("f@@x", "Apply[f, x]"),
        ("f/@x", "Map[f, x]"),
        ("f'[x]", "Derivative[1][f][x]"),
        ("f''[x]", "Derivative[2][f][x]"),
        ("x (* a (* b *) c *)+1", "Plus[x, 1]"),
        ("\"abc\\nα\"", "\"abc\\nα\""),
        (r"\[Pi]", "Pi"),
        (r"\[Alpha]", "α"),
        (r"\[Omega]", "ω"),
        (r"\[Infinity]", "Infinity"),
        (r"a\[Alpha]b", "aαb"),
        (r"x \[Element] Reals", "Element[x, Reals]"),
        (r"x\[Equal]1", "Equal[x, 1]"),
        (r"x\[LessEqual]1", "LessEqual[x, 1]"),
        (r"x\[GreaterEqual]1", "GreaterEqual[x, 1]"),
        (r"x\[NotEqual]1", "Unequal[x, 1]"),
        (r"x\[Rule]1", "Rule[x, 1]"),
        ("2^^1011", "11"),
        ("2x", "Times[2, x]"),
        ("x y", "Times[x, y]"),
        ("(a)(b)", "Times[a, b]"),
        ("f(x)", "Times[f, x]"),
        ("sin[x]", "sin[x]"),
        ("e+i+pi", "Plus[e, i, pi]"),
        ("%", "Out[]"),
        ("%3", "Out[3]"),
        ("5!", "Factorial[5]"),
        ("1e-3", "Plus[Times[1, e], Times[-1, 3]]"),
        ("0x1F", "Times[0, x1F]"),
    ] {
        assert_eq!(raw(src), expected, "{src}");
    }
}

#[test]
fn precedence_follows_the_official_operator_table() {
    for (src, expected) in [
        ("-x^2", "Times[-1, Power[x, 2]]"),
        ("2^-1", "Power[2, -1]"),
        ("a^b^c", "Power[a, Power[b, c]]"),
        ("f@x+y", "Plus[f[x], y]"),
        ("f@g@x", "f[g[x]]"),
        ("f@x!", "Factorial[f[x]]"),
        ("f@@x^2", "Power[Apply[f, x], 2]"),
        ("f/@x+y", "Plus[Map[f, x], y]"),
        ("a/b c", "Times[a, Power[b, -1], c]"),
        ("a b/c", "Times[a, b, Power[c, -1]]"),
        ("x!y", "Times[Factorial[x], y]"),
        ("!x==y", "Not[Equal[x, y]]"),
        ("a->b->c", "Rule[a, Rule[b, c]]"),
        (
            "x /. x->y /. y->1",
            "ReplaceAll[ReplaceAll[x, Rule[x, y]], Rule[y, 1]]",
        ),
        ("x /. #->1&", "Function[ReplaceAll[x, Rule[Slot[1], 1]]]"),
        ("x=1&", "Set[x, Function[1]]"),
        (
            "#^2&/@{1,2}",
            "Map[Function[Power[Slot[1], 2]], List[1, 2]]",
        ),
        ("x//f//g", "g[f[x]]"),
        (
            "f[x_]/;x>0:=x",
            "SetDelayed[Condition[f[Pattern[x, Blank[]]], Greater[x, 0]], x]",
        ),
    ] {
        assert_eq!(raw(src), expected, "{src}");
    }
}

#[test]
fn semicolons_build_compound_expressions_and_newlines_separate_statements() {
    assert_eq!(raw("a;b"), "CompoundExpression[a, b]");
    assert_eq!(raw("a;b;"), "CompoundExpression[a, b, Null]");
    assert_eq!(raw("f[a;b]"), "f[CompoundExpression[a, b]]");
    let out = parse("a;\nb\nf[x,\ny]", Dialect::Wolfram);
    assert!(out.diagnostics.is_empty());
    assert_eq!(out.statements.len(), 3);
    assert!(out.statements[0].suppress_output);
    assert!(!out.statements[1].suppress_output);
    assert_eq!(format!("{:?}", out.statements[2].expr), "f[x, y]");
}

#[test]
fn precision_and_named_characters_are_not_discarded() {
    use om_num::{Number, Real};
    for (src, bits) in [("1.5`30", 100), ("1.5`30*^-3", 100)] {
        let expr = parse_expr(src, Dialect::Wolfram).unwrap();
        let Some(Number::Real(Real::Big(value))) = expr.as_number() else {
            panic!("{expr:?}")
        };
        assert_eq!(value.precision(), bits);
        let exact = if src.contains("*^") {
            om_num::Rational::from(3) / om_num::Rational::from(2000)
        } else {
            om_num::Rational::from(3) / om_num::Rational::from(2)
        };
        let represented = om_num::Rational::try_from(value.clone()).unwrap();
        let difference = if represented >= exact {
            represented - exact.clone()
        } else {
            exact.clone() - represented
        };
        assert!(difference < exact / om_num::Rational::from(om_num::Integer::from(2).pow(bits)));
    }
    assert_eq!(
        parse_expr("1.5*^-3", Dialect::Wolfram).unwrap(),
        Expr::real(0.0015)
    );
    assert_eq!(
        parse_expr("1.`", Dialect::Wolfram).unwrap(),
        Expr::real(1.0)
    );
    assert_eq!(parse_expr("1`", Dialect::Wolfram).unwrap(), Expr::real(1.0));
    assert_eq!(
        parse_expr("1.23456789012345678`", Dialect::Wolfram).unwrap(),
        Expr::real(1.2345678901234567)
    );
    assert_eq!(
        parse_expr("1.2345678901234567", Dialect::Wolfram).unwrap(),
        Expr::real(1.2345678901234567)
    );
    assert!(matches!(
        parse_expr("1.23456789012345678", Dialect::Wolfram)
            .unwrap()
            .as_number(),
        Some(Number::Real(Real::Big(_)))
    ));
    assert_eq!(
        parse_expr(r#""\[Pi] \[Alpha]""#, Dialect::Wolfram).unwrap(),
        Expr::string("π α")
    );
    assert_eq!(
        parse_expr(r#""\[Equal] \[Rule]""#, Dialect::Wolfram).unwrap(),
        Expr::string("\u{f431} \u{f522}")
    );
    assert_eq!(raw("x\u{f522}1"), "Rule[x, 1]");
    assert!(parse_expr("1.5`1000001", Dialect::Wolfram).is_err());
    assert!(parse_expr("x**y", Dialect::Wolfram).is_err());
}

#[test]
fn both_dialects_produce_equivalent_raw_and_canonical_arithmetic() {
    for (modern, wolfram) in [
        ("solve(x^2+2x=3,x)", "Solve[x^2+2 x==3,x]"),
        ("sin(x)+sqrt(2)", "Sin[x]+Sqrt[2]"),
        ("[x->1,y->2]", "{x->1,y->2}"),
        ("(x+1)(x-1)", "(x+1)(x-1)"),
    ] {
        let m = parse_expr(modern, Dialect::Modern).unwrap();
        let w = parse_expr(wolfram, Dialect::Wolfram).unwrap();
        assert_eq!(m, w, "{modern} vs {wolfram}");
        assert_eq!(canonicalize(&m), canonicalize(&w));
    }
    assert_eq!(
        parse_expr("%wl\nSolve[x==1,x]", Dialect::Auto).unwrap(),
        parse_expr("Solve[x==1,x]", Dialect::Wolfram).unwrap()
    );
}

#[test]
fn all_section_14_wolfram_inputs_parse_without_errors() {
    let plan = include_str!("../../../docs/plan/PLAN.md");
    let section = plan
        .split("## 14. Solve 验收语料")
        .nth(1)
        .unwrap()
        .split("## 15.")
        .next()
        .unwrap();
    let mut count = 0;
    for line in section
        .lines()
        .filter(|line| line.starts_with("| ") && line.contains(" | `"))
    {
        let cells: Vec<_> = line.split('|').collect();
        let src = cells[3].trim().trim_matches('`');
        let out = parse(src, Dialect::Wolfram);
        assert!(
            !out.diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{src}: {:?}",
            out.diagnostics
        );
        assert_eq!(out.statements.len(), 1, "{src}");
        assert_ne!(out.statements[0].expr, Expr::symbol("$Failed"));
        count += 1;
    }
    assert_eq!(count, 53);
}

#[test]
fn malformed_wolfram_inputs_terminate_recover_and_retain_utf8_positions() {
    for src in [
        "f[",
        "v[[1]",
        "#&[",
        "x:=",
        "x_ /;",
        "[1]",
        "1.5*^-",
        "\\[Unknown]",
    ] {
        assert!(parse_expr(src, Dialect::Wolfram).is_err(), "{src}");
    }
    let out = parse("f[\n☃]\nx", Dialect::Wolfram);
    assert!(out.statements.iter().any(|s| s.expr == Expr::symbol("x")));
    for d in &out.diagnostics {
        assert!(
            "f[\n☃]\nx"
                .get(d.span.start as usize..d.span.end as usize)
                .is_some()
        );
    }
}

#[test]
fn wolfram_highlighting_and_arbitrary_short_sources_keep_valid_positions() {
    use om_parse::TokenClass;
    let src = r"Solve[\[Pi],x]";
    let out = parse(src, Dialect::Wolfram);
    assert_eq!(out.tokens[0].1, TokenClass::Builtin);
    assert!(
        out.tokens
            .iter()
            .any(|(span, class)| *class == TokenClass::Builtin
                && &src[span.start as usize..span.end as usize] == r"\[Pi]")
    );
    let alphabet = [
        'α', '☃', '"', '\\', '[', ']', '(', ')', '^', '_', '#', '\n', '1', '!', '&', ';', '@',
    ];
    for a in alphabet {
        for b in alphabet {
            for c in alphabet {
                let src: String = [a, b, c].into_iter().collect();
                let out = parse(&src, Dialect::Wolfram);
                for stmt in out.statements {
                    assert!(stmt.span.start <= stmt.span.end);
                    assert!(
                        src.get(stmt.span.start as usize..stmt.span.end as usize)
                            .is_some()
                    );
                }
                for d in out.diagnostics {
                    assert!(
                        src.get(d.span.start as usize..d.span.end as usize)
                            .is_some()
                    );
                }
            }
        }
    }
    let nested = format!("{}x{}", "f[".repeat(1000), "]".repeat(1000));
    assert!(
        parse_expr(&nested, Dialect::Wolfram)
            .unwrap_err()
            .iter()
            .any(|d| d.code == "E013")
    );
}
