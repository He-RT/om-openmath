//! Modern syntax, raw trees, precedence, literals and automatic dialect detection.

use om_core::{BUILTIN as B, Expr, Symbol};
use om_parse::{
    ConstantMode, Dialect, ParseEnv, Severity, TokenClass, detect_dialect, parse, parse_expr,
    parse_with,
};

fn raw(src: &str) -> String {
    format!(
        "{:?}",
        parse_expr(src, Dialect::Modern).unwrap_or_else(|d| panic!("{src}: {d:?}"))
    )
}

#[test]
fn every_modern_language_construct_has_a_raw_tree_vector() {
    for (src, expected) in [
        ("42", "42"),
        ("0x1F", "31"),
        ("1_000_000", "1000000"),
        ("1/3", "Times[1, Power[3, -1]]"),
        ("xy", "xy"),
        ("α", "α"),
        ("pi", "Pi"),
        ("π", "Pi"),
        ("e", "E"),
        ("i", "I"),
        ("inf", "Infinity"),
        ("∞", "Infinity"),
        ("infinity", "Infinity"),
        ("f(x, y)", "f[x, y]"),
        ("2x", "Times[2, x]"),
        ("2 x", "Times[2, x]"),
        ("x y", "Times[x, y]"),
        ("2(x+1)", "Times[2, Plus[x, 1]]"),
        ("(x+1)(x-1)", "Times[Plus[x, 1], Plus[x, Times[-1, 1]]]"),
        ("x (y+1)", "Times[x, Plus[y, 1]]"),
        ("2sin(x)", "Times[2, Sin[x]]"),
        ("3π", "Times[3, Pi]"),
        ("x^2", "Power[x, 2]"),
        ("x**2", "Power[x, 2]"),
        ("x²", "Power[x, 2]"),
        ("x⁻¹²", "Power[x, -12]"),
        ("sqrt(x)", "Sqrt[x]"),
        ("√x", "Sqrt[x]"),
        ("√(x+1)", "Sqrt[Plus[x, 1]]"),
        ("cbrt(x)", "CubeRoot[x]"),
        ("root(x,3)", "Power[x, Times[1, Power[3, -1]]]"),
        ("a=b", "Equal[a, b]"),
        ("a==b", "Equal[a, b]"),
        ("x!=y", "Unequal[x, y]"),
        ("x≠y", "Unequal[x, y]"),
        ("0<x<=1", "And[Less[0, x], LessEqual[x, 1]]"),
        ("0≤x<1", "And[LessEqual[0, x], Less[x, 1]]"),
        ("x>0", "Greater[x, 0]"),
        ("x≥0", "GreaterEqual[x, 0]"),
        ("x and y", "And[x, y]"),
        ("x&&y", "And[x, y]"),
        ("x∧y", "And[x, y]"),
        ("x or y", "Or[x, y]"),
        ("x||y", "Or[x, y]"),
        ("x∨y", "Or[x, y]"),
        ("not x", "Not[x]"),
        ("!x", "Not[x]"),
        ("¬x", "Not[x]"),
        ("[1,2,3]", "List[1, 2, 3]"),
        ("{1,2,3}", "List[1, 2, 3]"),
        ("[[1],[2]]", "List[List[1], List[2]]"),
        ("v[1]", "Part[v, 1]"),
        ("x->1", "Rule[x, 1]"),
        ("x→1", "Rule[x, 1]"),
        (
            "x+y where x=2,y=3",
            "ReplaceAll[Plus[x, y], List[Rule[x, 2], Rule[y, 3]]]",
        ),
        ("x /. x -> 2", "ReplaceAll[x, Rule[x, 2]]"),
        ("let a=5", "Set[a, 5]"),
        (
            "let f(x)=x^2",
            "SetDelayed[f[Pattern[x, Blank[]]], Power[x, 2]]",
        ),
        (
            "solve(x^2<4,x,domain: reals)",
            "Solve[Less[Power[x, 2], 4], x, Reals]",
        ),
        (
            "nsolve(x=1,x,precision:30)",
            "NSolve[Equal[x, 1], x, Rule[WorkingPrecision, 30]]",
        ),
        ("x # ignored", "x"),
        ("%", "Out[]"),
        ("%3", "Out[3]"),
        ("out(3)", "Out[3]"),
        ("5!", "Factorial[5]"),
        ("abs(x)", "Abs[x]"),
        ("|x|+|y|", "Plus[Abs[x], Abs[y]]"),
        ("\"a\\nα\"", "\"a\\nα\""),
        (
            "solve(x^2+2x=3,x)",
            "Solve[Equal[Plus[Power[x, 2], Times[2, x]], 3], x]",
        ),
    ] {
        assert_eq!(raw(src), expected, "{src}");
    }
}

#[test]
fn precedence_and_associativity_vectors() {
    for (src, expected) in [
        ("-x^2", "Times[-1, Power[x, 2]]"),
        ("2^-1", "Power[2, -1]"),
        ("x^y^z", "Power[x, Power[y, z]]"),
        ("2x^2", "Times[2, Power[x, 2]]"),
        ("2^x y", "Times[Power[2, x], y]"),
        ("x^2y", "Times[Power[x, 2], y]"),
        ("a/b/c", "Times[a, Power[b, -1], Power[c, -1]]"),
        ("a-b-c", "Plus[a, Times[-1, b], Times[-1, c]]"),
        ("x^2!", "Power[x, Factorial[2]]"),
        ("x!^2", "Power[Factorial[x], 2]"),
        ("x²^3", "Power[Power[x, 2], 3]"),
        (
            "not x=1 and y=2 or z=3",
            "Or[And[Not[Equal[x, 1]], Equal[y, 2]], Equal[z, 3]]",
        ),
        ("x->y->z", "Rule[x, Rule[y, z]]"),
        (
            "x+y where x=1+2,y=3*4",
            "ReplaceAll[Plus[x, y], List[Rule[x, Plus[1, 2]], Rule[y, Times[3, 4]]]]",
        ),
        ("2|x|", "Times[2, Abs[x]]"),
        ("|x| |y|", "Times[Abs[x], Abs[y]]"),
        ("|x+|y| |", "Abs[Plus[x, Abs[y]]]"),
        ("x=[1,2]", "Equal[x, List[1, 2]]"),
        ("|not x|", "Abs[Not[x]]"),
        ("|!x|", "Abs[Not[x]]"),
    ] {
        assert_eq!(raw(src), expected, "{src}");
    }
}

#[test]
fn names_and_keyword_options_follow_section_7_4() {
    for (name, head) in [
        ("solve", "Solve"),
        ("nsolve", "NSolve"),
        ("find_root", "FindRoot"),
        ("reduce", "Reduce"),
        ("eliminate", "Eliminate"),
        ("solvevalues", "SolveValues"),
        ("SIN", "Sin"),
        ("Sin", "Sin"),
        ("arcsin", "ArcSin"),
        ("acos", "ArcCos"),
        ("atan", "ArcTan"),
        ("sinh", "Sinh"),
        ("asinh", "ArcSinh"),
        ("exp", "Exp"),
        ("ln", "Log"),
        ("log", "Log"),
        ("sgn", "Sign"),
        ("re", "Re"),
        ("im", "Im"),
        ("conj", "Conjugate"),
        ("arg", "Arg"),
        ("ceil", "Ceiling"),
        ("factorial", "Factorial"),
        ("choose", "Binomial"),
        ("diff", "D"),
        ("d", "D"),
        ("numeric", "N"),
        ("implicitplot", "ContourPlot"),
        ("len", "Length"),
        ("lambertw", "ProductLog"),
        ("productlog", "ProductLog"),
        ("Hold", "Hold"),
        ("Out", "Out"),
    ] {
        assert_eq!(raw(&format!("{name}(x)")), format!("{head}[x]"));
    }
    assert_eq!(raw("atan2(y,x)"), "ArcTan[x, y]");
    assert_eq!(raw("log10(x)"), "Log[10, x]");
    assert_eq!(raw("log2(x)"), "Log[2, x]");
    assert_eq!(raw("log(b,x)"), "Log[b, x]");
    assert_eq!(raw("Root(x,2)"), "Root[x, 2]");
    assert_eq!(
        raw("solve(x=1,x,domain: integers,steps:false,method:\"exact\",max_iterations:100)"),
        "Solve[Equal[x, 1], x, Integers, Rule[RecordSteps, False], Rule[Method, \"exact\"], Rule[MaxIterations, 100]]"
    );
    assert_eq!(
        raw("solve(x^2=1,x,domain:positives)"),
        "Solve[And[Equal[Power[x, 2], 1], Greater[x, 0]], x, Reals]"
    );
    assert_eq!(
        raw("solve([x+y=1],[x,y],domain:positives)"),
        "Solve[List[Equal[Plus[x, y], 1], Greater[x, 0], Greater[y, 0]], List[x, y], Reals]"
    );
}

#[test]
fn constants_can_be_strict_without_changing_other_name_mappings() {
    let env = ParseEnv {
        constants: ConstantMode::Strict,
        ..ParseEnv::default()
    };
    let out = parse_with("e+i+pi+sin(x)", Dialect::Modern, &env);
    assert_eq!(
        format!("{:?}", out.statements[0].expr),
        "Plus[e, i, Pi, Sin[x]]"
    );
}

#[test]
fn statement_boundaries_suppression_and_multiline_groups() {
    let src = "let a=5;\nlet f(x,y)=x+y\nf(a,\n2)\n[1,\n2];";
    let out = parse(src, Dialect::Modern);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(out.statements.len(), 4);
    assert!(out.statements[0].suppress_output);
    assert!(!out.statements[1].suppress_output);
    assert!(!out.statements[2].suppress_output);
    assert!(out.statements[3].suppress_output);
    assert_eq!(
        format!("{:?}", out.statements[1].expr),
        "SetDelayed[f[Pattern[x, Blank[]], Pattern[y, Blank[]]], Plus[x, y]]"
    );
    assert_eq!(format!("{:?}", out.statements[2].expr), "f[a, 2]");
    for stmt in &out.statements {
        assert!(
            src.get(stmt.span.start as usize..stmt.span.end as usize)
                .is_some()
        );
    }
    assert!(parse_expr("x;y", Dialect::Modern).is_err());
    assert!(parse_expr("# only comment", Dialect::Modern).is_err());
    assert_eq!(raw("2+2"), "Plus[2, 2]");
}

#[test]
fn function_aliases_do_not_rename_free_letters_or_bound_parameters() {
    assert_eq!(raw("a+b+c+d+n"), "Plus[a, b, c, d, n]");
    assert_eq!(raw("d(x)"), "D[x]");
    assert_eq!(raw("n(x)"), "N[x]");
    assert_eq!(
        raw("let f(e,i,pi)=e+i+pi"),
        "SetDelayed[f[Pattern[e, Blank[]], Pattern[i, Blank[]], Pattern[pi, Blank[]]], Plus[e, i, pi]]"
    );
    for name in om_core::builtins::names() {
        assert_eq!(
            parse_expr(name, Dialect::Modern).unwrap(),
            Expr::symbol(name)
        );
    }
}

#[test]
fn the_three_ambiguity_rulings_emit_the_required_diagnostics_and_fixes() {
    for (src, expected, code, severity) in [
        ("f (x)", "Times[f, x]", "W001", Severity::Hint),
        ("a(b+c)", "a[Plus[b, c]]", "W002", Severity::Hint),
        ("sin x", "Sin[x]", "E010", Severity::Error),
        ("|sin x|", "Abs[Sin[x]]", "E010", Severity::Error),
    ] {
        let out = parse(src, Dialect::Modern);
        assert_eq!(format!("{:?}", out.statements[0].expr), expected);
        let d = out.diagnostics.iter().find(|d| d.code == code).unwrap();
        assert_eq!(d.severity, severity);
        let fix = d.fix.as_ref().unwrap();
        let repaired = format!(
            "{}{}{}",
            &src[..fix.span.start as usize],
            fix.replacement,
            &src[fix.span.end as usize..]
        );
        assert!(
            !parse(&repaired, Dialect::Modern)
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{repaired}"
        );
    }
    let mut env = ParseEnv::default();
    env.known_functions.insert(Symbol::intern("a"));
    assert!(
        parse_with("a(x)", Dialect::Modern, &env)
            .diagnostics
            .is_empty()
    );
    assert!(
        parse("let a(x)=x; a(1)", Dialect::Modern)
            .diagnostics
            .is_empty()
    );
    assert!(
        parse("let f(x)=f(x)", Dialect::Modern)
            .diagnostics
            .is_empty()
    );
}

#[test]
fn literals_keep_machine_or_requested_binary_precision() {
    use om_num::{Number, Real};
    for src in ["3.14", "1e-3", "1.5e10"] {
        let e = parse_expr(src, Dialect::Modern).unwrap();
        let Some(Number::Real(Real::Machine(x))) = e.as_number() else {
            panic!("{src}: {e:?}")
        };
        assert_eq!(*x, src.parse::<f64>().unwrap());
    }
    let e = parse_expr("1.23456789012345678", Dialect::Modern).unwrap();
    let Some(Number::Real(Real::Big(x))) = e.as_number() else {
        panic!("{e:?}")
    };
    assert_eq!(x.precision(), 60);
    for src in ["1e400", "1e-400"] {
        let e = parse_expr(src, Dialect::Modern).unwrap();
        let Some(Number::Real(Real::Big(x))) = e.as_number() else {
            panic!("{e:?}")
        };
        assert_eq!(x.precision(), 53);
        assert!(!e.is_zero());
    }
    assert_eq!(raw("\"\\u03b1\""), "\"α\"");
    assert_eq!(raw("\"\\uD83D\\uDE00\""), "\"😀\"");
    for src in ["1e9999999999999999999", "1e1000001", "\"\\uD800\""] {
        assert!(parse_expr(src, Dialect::Modern).is_err(), "{src}");
    }
}

#[test]
fn highlighting_and_markers_preserve_original_source_offsets() {
    let src = "%modern\r\nsin(α) # hi";
    let out = parse(src, Dialect::Auto);
    assert_eq!(out.dialect, Dialect::Modern);
    assert_eq!(out.statements[0].span.start, 9);
    assert_eq!(
        out.statements[0].expr,
        Expr::call(B::SIN, [Expr::symbol("α")])
    );
    assert!(
        out.tokens
            .iter()
            .any(|(s, c)| *c == TokenClass::Builtin
                && &src[s.start as usize..s.end as usize] == "sin")
    );
    assert!(out.tokens.iter().any(|(s, c)| *c == TokenClass::Comment
        && src[s.start as usize..s.end as usize].starts_with("%modern")));
    let src = "c+d+n+sin(x)";
    let out = parse(src, Dialect::Modern);
    for (span, class) in &out.tokens {
        if matches!(
            &src[span.start as usize..span.end as usize],
            "c" | "d" | "n"
        ) {
            assert_eq!(*class, TokenClass::Identifier);
        }
    }
}

#[test]
fn automatic_dialect_detection_has_positive_and_negative_vectors() {
    for src in [
        "%wl\nx",
        "Solve[x==1,x]",
        "f[x]",
        "c[1]",
        "x==1 -> y",
        "x/.r (* comment *)",
        "x:=1 -> x",
    ] {
        assert_eq!(detect_dialect(src), Dialect::Wolfram, "{src}");
    }
    for src in [
        "%modern\nSolve[x]",
        "sin[x]",
        "solve(x=1,x)",
        "x==1",
        "x==1 && y==2",
        "x -> y",
        "# Solve[x] :=\nx",
        "\"Solve[x] == ->\"",
    ] {
        assert_eq!(detect_dialect(src), Dialect::Modern, "{src}");
    }
}

#[test]
fn malformed_inputs_recover_and_nesting_is_bounded() {
    for src in [
        "(", "[1,", "a+", "let = 2", "sin(", "x := 1", "|x", "x, y", "☃", "1__2",
    ] {
        assert!(parse_expr(src, Dialect::Modern).is_err(), "{src}");
    }
    let out = parse("☃\nx+1\n1__2\ny", Dialect::Modern);
    assert!(out.statements.iter().any(|s| s.expr == Expr::symbol("y")));
    let deeply_nested = format!("{}x{}", "(".repeat(1000), ")".repeat(1000));
    let out = parse(&deeply_nested, Dialect::Modern);
    assert!(out.diagnostics.iter().any(|d| d.code == "E013"));
    assert!(parse_expr("-1", Dialect::Modern).unwrap() == Expr::int(-1));
}

#[test]
fn arbitrary_short_sources_terminate_and_wide_and_deep_inputs_are_distinguished() {
    let alphabet = [
        'α', '☃', '"', '\\', '[', ']', '(', ')', '^', '_', '#', '\n', '1', '!', '|',
    ];
    for a in alphabet {
        for b in alphabet {
            for c in alphabet {
                let src: String = [a, b, c].into_iter().collect();
                let out = parse(&src, Dialect::Modern);
                for s in out.statements {
                    assert!(s.span.start <= s.span.end);
                    assert!(
                        src.get(s.span.start as usize..s.span.end as usize)
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
    let wide = vec!["x"; 2000].join("+");
    assert_eq!(
        parse_expr(&wide, Dialect::Modern).unwrap().args().len(),
        2000
    );
    let division = vec!["x"; 1000].join("/");
    assert_eq!(
        parse_expr(&division, Dialect::Modern).unwrap().args().len(),
        1000
    );
    let deep = vec!["x"; 1000].join("->");
    assert!(
        parse_expr(&deep, Dialect::Modern)
            .unwrap_err()
            .iter()
            .any(|d| d.code == "E013")
    );
}
