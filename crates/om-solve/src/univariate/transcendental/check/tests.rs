//! Independent integer samples must challenge candidate identities off the diagonal.
use super::*;
use crate::NoSteps;
use om_core::canonicalize;
use om_parse::{Dialect, parse_expr};
fn e(s: &str) -> Expr {
    canonicalize(&parse_expr(s, Dialect::Wolfram).unwrap())
}
fn family(n: usize) -> Solution {
    Solution {
        rules: ["x", "y", "z"]
            .iter()
            .take(n)
            .enumerate()
            .map(|(i, v)| (e(v), Expr::call(B::C, [Expr::int(i as i64 + 1)])))
            .collect(),
        condition: None,
        constants: (1..=n)
            .map(|i| (Expr::call(B::C, [Expr::int(i as i64)]), Domain::Integers))
            .collect(),
        multiplicity: 1,
        verification: Verification::Unverified,
        numeric: None,
    }
}
fn check(original: &Expr, root: Solution) -> Vec<Solution> {
    let mut roots = vec![root];
    candidates(
        Source {
            original,
            exclusions: &[],
            guards: &[],
        },
        &mut roots,
        &SolveOptions::default(),
        &Interrupt::default(),
        &mut NoSteps,
        &mut vec![],
    )
    .unwrap();
    roots
}
#[test]
fn two_independent_constants_do_not_prove_a_false_diagonal_identity() {
    assert!(check(&e("x-y"), family(2)).is_empty());
}
#[test]
fn pairwise_probes_challenge_a_false_three_parameter_identity() {
    assert!(check(&e("(x-y)*(y-z)*(x-z)"), family(3)).is_empty());
}
#[test]
fn valid_period_families_keep_all_independent_constants() {
    let mut root = family(2);
    root.rules[0].1 = e("2 Pi I C[1]");
    root.rules[1].1 = e("2 Pi I C[2]");
    let roots = check(&e("Exp[x]+Exp[y]-2"), root);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].constants.len(), 2);
}
#[test]
fn explicit_period_constraints_allow_only_valid_off_diagonal_members() {
    let mut root = family(2);
    root.condition = Some(e("C[1]==C[2]"));
    assert_eq!(check(&e("x-y"), root).len(), 1);
}

#[test]
fn exact_parameter_identity_keeps_exact_evidence_after_domain_probes() {
    let mut root = family(0);
    root.rules.push((e("x"), e("a")));
    root.condition = Some(e("a!=0"));
    let roots = check(&e("x-a"), root);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].verification, Verification::Exact);
}

#[test]
fn unavailable_numeric_equality_is_unknown_in_a_condition() {
    assert_eq!(
        allows(&e("ProductLog[-2]==1"), &Interrupt::default()).unwrap(),
        None
    );
}

#[test]
fn numeric_product_log_can_disprove_a_closed_equality() {
    assert_eq!(
        allows(&e("ProductLog[2]==1"), &Interrupt::default()).unwrap(),
        Some(false)
    );
}

#[test]
fn periodic_nonzero_requires_integer_pi_steps_and_a_nonzero_offset() {
    let constants = [(e("C[1]"), Domain::Integers), (e("C[2]"), Domain::Integers)];
    for src in ["Cos[Pi/4+Pi C[1]]", "Sin[Pi/6+2Pi C[1]-3Pi C[2]]"] {
        assert!(
            periodic_nonzero(&e(src), &constants, &Interrupt::default()).unwrap(),
            "{src}"
        );
    }
    for src in [
        "Cos[Pi/2+Pi C[1]]",
        "Cos[Pi C[1]/2]",
        "Cos[Pi/4+C[1]]",
        "Cos[a+Pi C[1]]",
        "Sin[Pi C[1]]",
    ] {
        assert!(
            !periodic_nonzero(&e(src), &constants, &Interrupt::default()).unwrap(),
            "{src}"
        );
    }
    assert!(
        !periodic_nonzero(
            &e("Cos[Pi/4+Pi C[1]]"),
            &[(e("C[1]"), Domain::Reals)],
            &Interrupt::default()
        )
        .unwrap()
    );
}
