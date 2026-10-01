//! Numeric display preserves exact discrete-domain membership certificates.
use om_core::{Expr, Interrupt};
use om_solve::{Domain, SolutionSet, SolveOptions, Verification};
fn raw(s: &str) -> Expr {
    om_parse::parse_expr(s, om_parse::Dialect::Wolfram).unwrap()
}
#[test]
fn actual_nsolve_discrete_membership_is_checked_before_numeric_rounding() {
    for (source, domain, values) in [
        ("2*x==4", Domain::Integers, vec![2.0]),
        ("2*x==5", Domain::Integers, vec![]),
        ("3*x==1", Domain::Rationals, vec![1.0 / 3.0]),
        ("x^2==2", Domain::Rationals, vec![]),
    ] {
        let result = om_solve::nsolve_with_options(
            &raw(source),
            &[Expr::symbol("x")],
            om_num::Precision::Machine,
            &SolveOptions {
                domain,
                ..Default::default()
            },
            &Interrupt::default(),
        )
        .unwrap();
        let SolutionSet::Finite(rows) = result.set else {
            panic!("{source}: {:?}", result.set)
        };
        assert_eq!(rows.len(), values.len());
        for (row, value) in rows.iter().zip(values) {
            assert!(matches!(row.verification, Verification::Numeric { .. }));
            assert!((row.rules[0].1.as_number().unwrap().to_f64().unwrap() - value).abs() < 1e-14);
        }
    }
}
