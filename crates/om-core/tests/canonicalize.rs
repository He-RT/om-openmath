//! Public traversal and derived canonical-constructor contracts.

use om_core::{
    BUILTIN as B, Expr, Symbol, add, canonicalize, div, exp, func, mul, neg, pow, sqrt, sub,
    with_canonical_messages,
};
use om_num::{Complex, Number};

fn x() -> Expr {
    Expr::symbol("x")
}
fn p(b: Expr, e: Expr) -> Expr {
    Expr::call(B::POWER, [b, e])
}
fn a(args: impl IntoIterator<Item = Expr>) -> Expr {
    Expr::call(B::PLUS, args)
}
fn m(args: impl IntoIterator<Item = Expr>) -> Expr {
    Expr::call(B::TIMES, args)
}

#[test]
fn ten_raw_nested_trees_match_manually_canonical_results() {
    let y = Expr::symbol("y");
    let cases = [
        (
            a([Expr::int(1), a([Expr::int(2), x()])]),
            add([Expr::int(3), x()]),
        ),
        (
            m([Expr::int(2), m([x(), Expr::int(3)])]),
            mul([Expr::int(6), x()]),
        ),
        (a([m([Expr::int(2), x()]), x()]), mul([Expr::int(3), x()])),
        (p(p(x(), Expr::rational(1, 2)), Expr::int(2)), x()),
        (
            p(m([x(), y.clone()]), Expr::int(2)),
            mul([pow(x(), Expr::int(2)), pow(y, Expr::int(2))]),
        ),
        (
            p(a([Expr::int(1), Expr::int(1)]), Expr::int(3)),
            Expr::int(8),
        ),
        (
            m([
                p(Expr::int(2), Expr::rational(1, 2)),
                p(Expr::int(3), Expr::rational(1, 2)),
            ]),
            sqrt(Expr::int(6)),
        ),
        (
            a([
                Expr::call(B::SQRT, [Expr::int(12)]),
                Expr::call(B::SQRT, [Expr::int(3)]),
            ]),
            mul([Expr::int(3), sqrt(Expr::int(3))]),
        ),
        (
            m([Expr::int(-1), a([Expr::int(1), x()])]),
            neg(add([Expr::int(1), x()])),
        ),
        (
            Expr::call(
                B::EXP,
                [Expr::call(B::LOG, [a([Expr::int(1), Expr::int(2)])])],
            ),
            Expr::int(3),
        ),
    ];
    for (raw, expected) in cases {
        let e = canonicalize(&raw);
        assert_eq!(e, expected);
        assert_eq!(canonicalize(&e), e);
    }
}

#[test]
fn derived_constructors_and_func_respect_minimal_evaluation() {
    assert_eq!(sub(x(), x()), Expr::int(0));
    assert_eq!(div(Expr::int(2), Expr::int(4)), Expr::rational(1, 2));
    assert_eq!(sqrt(Expr::int(12)), mul([Expr::int(2), sqrt(Expr::int(3))]));
    assert_eq!(exp(Expr::call(B::LOG, [x()])), x());
    assert_eq!(
        func(B::PLUS, vec![Expr::int(1), Expr::int(2)]),
        Expr::int(3)
    );
    assert_eq!(
        func(B::POWER, vec![Expr::int(2)]),
        Expr::call(B::POWER, [Expr::int(2)])
    );
    assert_eq!(
        func(B::SIN, vec![Expr::int(0)]),
        Expr::call(B::SIN, [Expr::int(0)])
    );
    assert_eq!(
        func(B::LOG, vec![Expr::int(1)]),
        Expr::call(B::LOG, [Expr::int(1)])
    );
    let f = Symbol::intern("unknown_f");
    assert_eq!(func(f, vec![x()]), Expr::call(f, [x()]));
}

#[test]
fn canonicalization_visits_compound_heads_and_all_arguments() {
    let raw = Expr::normal(
        a([Expr::int(1), Expr::int(2)]),
        [a([Expr::int(2), Expr::int(3)])],
    );
    assert_eq!(
        canonicalize(&raw),
        Expr::normal(Expr::int(3), [Expr::int(5)])
    );
    let e = Expr::call(B::HOLD, [a([Expr::int(1), Expr::int(2)])]);
    // Holding evaluator recursion belongs to M4; canonicalization is structural everywhere.
    assert_eq!(canonicalize(&e), Expr::call(B::HOLD, [Expr::int(3)]));
}

#[test]
fn literal_aliases_and_directed_infinities_are_stable() {
    let i = Expr::number(Number::Complex(Box::new(Complex {
        re: Number::Integer(0.into()),
        im: Number::Integer(1.into()),
    })));
    assert_eq!(canonicalize(&Expr::sym(B::I)), i);
    assert_eq!(
        canonicalize(&Expr::sym(B::INFINITY)),
        Expr::call(B::DIRECTED_INFINITY, [Expr::int(1)])
    );
    assert_eq!(
        canonicalize(&Expr::sym(B::COMPLEX_INFINITY)),
        Expr::call(B::DIRECTED_INFINITY, [])
    );
    let directed = Expr::call(B::DIRECTED_INFINITY, [Expr::int(2)]);
    assert_eq!(
        canonicalize(&directed),
        Expr::call(B::DIRECTED_INFINITY, [Expr::int(1)])
    );
    let complex = Expr::number(Number::Complex(Box::new(Complex {
        re: Number::Integer(1.into()),
        im: Number::Integer(1.into()),
    })));
    let directed = func(B::DIRECTED_INFINITY, vec![complex]);
    assert_eq!(canonicalize(&directed), directed);
}

#[test]
fn direct_arithmetic_calls_resolve_constant_aliases() {
    assert_eq!(pow(Expr::sym(B::I), Expr::int(2)), Expr::int(-1));
    assert_eq!(mul([Expr::sym(B::I), Expr::sym(B::I)]), Expr::int(-1));
    assert_eq!(
        add([Expr::sym(B::I), Expr::int(1)]),
        Expr::number(Number::Complex(Box::new(Complex {
            re: Number::Integer(1.into()),
            im: Number::Integer(1.into()),
        })))
    );
}

#[test]
fn structural_replacement_normalizes_once_without_revisiting_values() {
    let raw = a([x(), Expr::int(1)]);
    assert_eq!(raw.replace_all(&[(x(), Expr::int(2))]), Expr::int(3));
    assert_eq!(
        x().replace_all(&[(x(), Expr::call(B::SQRT, [Expr::int(12)]))]),
        sqrt(Expr::int(12))
    );
    assert_eq!(
        raw.replace_all(&[(x(), raw.clone())]),
        add([x(), Expr::int(2)])
    );
    assert_eq!(
        a([Expr::int(1), Expr::int(2)]).replace_all(&[]),
        Expr::int(3)
    );
}

#[test]
fn map_args_normalizes_the_rebuilt_expression_and_nested_results() {
    let e = Expr::call(B::PLUS, [Expr::int(1), Expr::int(2)]);
    assert_eq!(e.map_args(|n| n.clone()), Expr::int(3));
    assert_eq!(
        e.map_args(|_| a([Expr::int(2), Expr::int(3)])),
        Expr::int(10)
    );
    let atom = x();
    assert_eq!(atom.map_args(|_| panic!("no atom arguments")), atom);
}

#[test]
fn traversal_captures_nested_constructor_messages() {
    let raw = Expr::call(B::LIST, [p(Expr::int(0), Expr::int(-1)), x()]);
    let (e, msgs) = with_canonical_messages(|| canonicalize(&raw));
    assert_eq!(e.args()[0], Expr::call(B::DIRECTED_INFINITY, []));
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].tag, "infy");
}
