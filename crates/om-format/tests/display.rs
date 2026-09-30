//! Mathematical display snapshots and renderer fixtures.
use om_core::{Expr, canonicalize};
use om_format::{latex, unicode_form};
use om_parse::{Dialect, parse_expr};

fn cases() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("1/2", r"\frac{1}{2}", "1/2"),
        ("-7/11", r"-\frac{7}{11}", "−7/11"),
        ("x^2", r"x^{2}", "x²"),
        ("x^-2", r"\frac{1}{x^{2}}", "1/x²"),
        ("Sqrt[2]", r"\sqrt{2}", "√2"),
        ("Sqrt[x+1]", r"\sqrt{x + 1}", "√(x + 1)"),
        ("x^(1/3)", r"\sqrt[3]{x}", "∛x"),
        ("x^2+2*x-3", r"x^{2} + 2 x - 3", "x² + 2x − 3"),
        ("(x+1)/(x-1)", r"\frac{x + 1}{x - 1}", "(x + 1)/(x − 1)"),
        ("Pi", r"\pi", "π"),
        ("α", r"\alpha", "α"),
        ("θ", r"\theta", "θ"),
        ("Sin[x]", r"\sin\left(x\right)", "sin(x)"),
        ("Cos[x]", r"\cos\left(x\right)", "cos(x)"),
        ("Log[x]", r"\log\left(x\right)", "log(x)"),
        ("I", r"\mathrm{i}", "i"),
        ("Infinity", r"\infty", "∞"),
        ("-Infinity", r"-\infty", "−∞"),
        ("x^2==1", r"x^{2} = 1", "x² = 1"),
        ("0<=x", r"0 \le x", "0 ≤ x"),
        ("{1,2,x}", r"\left[1, 2, x\right]", "[1, 2, x]"),
        ("Part[v,1]", r"v_{1}", "v[1]"),
        ("x->1", r"x \to 1", "x → 1"),
        ("Factorial[x]", r"x!", "x!"),
        ("Abs[x]", r"\left|x\right|", "|x|"),
        ("f[x]", r"\operatorname{f}\left(x\right)", "f(x)"),
    ]
}
fn expr(src: &str) -> Expr {
    canonicalize(&parse_expr(src, Dialect::Wolfram).unwrap())
}
#[test]
fn fractions_roots_powers_greek_functions_and_operators_have_mathematical_display() {
    assert_eq!(latex(&Expr::rational(1, 2)), r"\frac{1}{2}");
    for (src, tex, unicode) in cases() {
        let e = expr(src);
        assert_eq!(latex(&e), tex, "{src}");
        assert_eq!(unicode_form(&e), unicode, "{src}");
    }
    renderer_fixture();
}
#[test]
fn text_is_escaped_and_large_exponents_keep_their_meaning() {
    assert_eq!(latex(&Expr::symbol("a_b")), r"\mathrm{a\_b}");
    assert_eq!(
        latex(&Expr::string("a_{b}%&$#")),
        r"\text{a\_\{b\}\%\&\$\#}"
    );
    assert_eq!(unicode_form(&expr("x^123")), "x¹²³");
    assert_eq!(unicode_form(&expr("x^(-123)")), "1/x¹²³");
    assert_eq!(unicode_form(&expr("(x^2)^(1/2)")), "√(x²)");
    assert_eq!(unicode_form(&expr("x^y")), "x^y");
    assert_eq!(
        latex(&expr("1.5*^20^x")),
        r"\left(1.5 \times 10^{20}\right)^{x}"
    );
    assert_eq!(latex(&expr("(-1/2)^x")), r"\left(-\frac{1}{2}\right)^{x}");
    for src in [
        "(-x)^2",
        "x^(y+1)",
        "x^(y^z)",
        "Root[#^2-2&,1]",
        "Derivative[1][f][x]",
        "x>0&&x<1",
        "ConditionalExpression[x,x>0]",
        "{x->Sqrt[2],y->1/3}",
    ] {
        let e = expr(src);
        let original = om_format::full_form(&e);
        assert!(!latex(&e).is_empty());
        assert!(!unicode_form(&e).is_empty());
        assert_eq!(om_format::full_form(&e), original);
    }
}
fn renderer_fixture() {
    // The local verification runner can request the exact outputs for KaTeX.
    if let Ok(path) = std::env::var("OM_LATEX_FIXTURE") {
        let mut outputs: Vec<_> = cases()
            .into_iter()
            .map(|(src, _, _)| latex(&expr(src)))
            .collect();
        for line in include_str!("../../../docs/plan/PLAN.md")
            .split("#### 8.1.5 ")
            .nth(1)
            .unwrap()
            .split("#### 8.1.6")
            .next()
            .unwrap()
            .lines()
            .filter(|line| {
                line.starts_with('|') && line.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
            })
        {
            if let Some(src) = line.split('`').nth(1) {
                outputs.push(latex(&expr(src)));
            }
        }
        outputs.extend([
            latex(&expr("1.5*^20^x")),
            latex(&expr("(-1/2)^x")),
            latex(&Expr::string("a_{b}%&$#\\\n😀")),
            latex(&Expr::symbol("a_b")),
            latex(&expr("Root[#^2-2&,1]")),
            latex(&expr("Derivative[1][f][x]")),
        ]);
        std::fs::write(path, outputs.join("\n")).unwrap();
    }
}
