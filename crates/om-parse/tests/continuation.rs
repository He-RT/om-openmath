//! A newline continues only an expression that is missing its next operand.
use om_core::{BUILTIN as B, Expr};
use om_parse::{Dialect, Severity, Span, parse};

fn expressions(source: &str) -> Vec<Expr> {
    let output = parse(source, Dialect::Modern);
    assert!(
        !output
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error),
        "{source}: {:?}",
        output.diagnostics
    );
    output.statements.into_iter().map(|s| s.expr).collect()
}

#[test]
fn unfinished_modern_expressions_continue_without_changing_ast() {
    for (single, multi) in [
        ("let f(x)=x^2+1; f(3)", "let f(x)=\n x^2+1; f(3)"),
        ("let f=fn(x)=>x^2; f(3)", "let f=fn(x)=>\n x^2; f(3)"),
        ("1+2", "1 +\n 2"),
        ("[1,2,3] |> map(fn(x)=>x^2)", "[1,2,3] |>\n map(fn(x)=>x^2)"),
        ("x=2", "x =\n 2"),
        ("x==2", "x ==\n 2"),
        ("x^2", "x ^\n 2"),
        ("x/2", "x /\n 2"),
        ("x@v", "x @\n v"),
        ("x->2", "x ->\n 2"),
        ("x..2", "x ..\n 2"),
        ("x /. [x->2]", "x /.\n [x->2]"),
        ("x where x=2", "x where\n x=\n 2"),
        ("x&&y", "x &&\n y"),
        ("!x", "!\n x"),
        ("{color:1}.color", "{color:1}.\n color"),
        ("let f(x)=f(x-1)", "let f(x)=\n f(x-1)"),
    ] {
        assert_eq!(expressions(multi), expressions(single), "{multi}");
    }
}

#[test]
fn blank_lines_comments_and_crlf_preserve_byte_coordinates() {
    for gap in [
        "\n",
        "\r\n",
        "\r",
        "\n\n",
        " # 中文注释 🧮\r\n\r\n",
        " (*注释*)\n",
    ] {
        let source = format!("let 面积(x) = {gap} x^2;\n面积(3)");
        let output = parse(&source, Dialect::Modern);
        assert!(
            output.diagnostics.is_empty(),
            "{source}: {:?}",
            output.diagnostics
        );
        assert_eq!(output.statements.len(), 2);
        assert!(output.statements[0].suppress_output);
        for statement in &output.statements {
            assert!(
                source
                    .get(statement.span.start as usize..statement.span.end as usize)
                    .is_some()
            );
        }
        assert_eq!(output.statements[0].span.start, 0);
        assert_eq!(
            output.statements[0].span.end as usize,
            source.find(';').unwrap() + 1
        );
        assert_eq!(
            output.statements[1].expr.head().as_symbol().unwrap().name(),
            "面积"
        );
    }
}

#[test]
fn completed_expressions_and_explicit_terminators_still_end_statements() {
    assert_eq!(
        expressions("1\n2\n+3"),
        vec![Expr::int(1), Expr::int(2), Expr::int(3)]
    );
    let out = parse("1;\n2\n[3]", Dialect::Modern);
    assert_eq!(out.statements.len(), 3);
    assert!(out.statements[0].suppress_output);
    assert_eq!(out.statements[2].expr, Expr::call(B::LIST, [Expr::int(3)]));
    assert_eq!(expressions("let a=1\nlet b=2\na+b").len(), 3);
    // Existing Wolfram top-level newline behavior must stay distinct.
    assert!(
        parse("x+\n2", Dialect::Wolfram)
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    );
    assert!(
        parse("let a=1 +;\n2", Dialect::Modern)
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    );
}

#[test]
fn missing_operands_do_not_consume_the_next_independent_declaration() {
    for prefix in ["let f(x) =", "let f=fn(x)=>", "1 +", "[1,2] |>"] {
        for gap in ["\n", "\r\n", " # 中文\n\n"] {
            let source = format!("{prefix}{gap}let good(x)=x+1;\ngood(2)");
            let output = parse(&source, Dialect::Modern);
            let errors: Vec<_> = output
                .diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .collect();
            assert_eq!(errors.len(), 1, "{source}: {:?}", output.diagnostics);
            assert_eq!(errors[0].code, "E022");
            assert_eq!(output.statements.len(), 3, "{source}");
            assert_eq!(output.statements[0].expr, Expr::symbol("$Failed"));
            assert!(
                output.statements[1].expr.is_head(B::SET_DELAYED),
                "{source}"
            );
            assert_eq!(
                output.statements[2].expr.head().as_symbol().unwrap().name(),
                "good"
            );
            assert!(
                !output.diagnostics.iter().any(|d| d.code == "W002"),
                "{source}"
            );
        }
    }
}

#[test]
fn eof_and_semicolon_diagnostics_point_to_the_unfinished_operator() {
    for tail in ["", "\n", "\n\n", " #末尾注释\r\n", "; 7"] {
        let source = format!("α +{tail}");
        let out = parse(&source, Dialect::Modern);
        let errors: Vec<_> = out
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert_eq!(errors.len(), 1, "{source}: {:?}", out.diagnostics);
        assert_eq!(errors[0].code, "E022");
        assert_eq!(errors[0].span, Span { start: 3, end: 4 });
        if tail.starts_with(';') {
            assert_eq!(out.statements.last().unwrap().expr, Expr::int(7));
        }
    }
}

#[test]
fn invalid_function_definitions_do_not_register_successful_bindings() {
    for source in [
        "let f(x)=\n;f(1)",
        "let f(x)=x :\nf(1)",
        "let f(x)=sin x;\nf(1)",
    ] {
        let out = parse(source, Dialect::Modern);
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error)
        );
        assert!(
            out.diagnostics.iter().any(|d| d.code == "W002"),
            "{source}: {:?}",
            out.diagnostics
        );
    }
    let out = parse("let f(x)=;f(1);\nmystery(2)", Dialect::Modern);
    assert_eq!(
        out.diagnostics.iter().filter(|d| d.code == "W002").count(),
        2
    );
}
