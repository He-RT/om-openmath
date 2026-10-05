//! Public modern matrix interfaces expose real factors, machine solves and failure states.
use om_core::{BUILTIN as B, Expr, ExprKind, Interrupt};
use om_eval::Evaluator;
use om_parse::Dialect;
fn evaluate(ev: &mut Evaluator, s: &str) -> Expr {
    ev.evaluate(
        &om_parse::parse_expr(s, Dialect::Modern).unwrap(),
        &Interrupt::default(),
    )
    .unwrap()
}
fn field<'a>(e: &'a Expr, key: &str) -> &'a Expr {
    e.args()
        .iter()
        .find(|r| r.args()[0] == Expr::string(key))
        .unwrap()
        .args()
        .get(1)
        .unwrap()
}
fn matrix(e: &Expr) -> Vec<Vec<f64>> {
    e.args()
        .iter()
        .map(|r| {
            r.args()
                .iter()
                .map(|x| x.as_number().unwrap().to_f64().unwrap())
                .collect()
        })
        .collect()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-11 * (1. + b.abs()), "{a} != {b}");
}
#[test]
fn lu_and_qr_return_genuine_reconstructable_factors() {
    let mut ev = Evaluator::new();
    let f = evaluate(&mut ev, "lu([[0.0,2.0],[1.0,3.0]])");
    assert!(f.is_head(B::RECORD));
    let l = matrix(field(&f, "l"));
    let u = matrix(field(&f, "u"));
    let p = field(&f, "permutation")
        .args()
        .iter()
        .map(|e| e.as_number().unwrap().to_f64().unwrap() as usize - 1)
        .collect::<Vec<_>>();
    let a = [[0., 2.], [1., 3.]];
    for i in 0..2 {
        for j in 0..2 {
            close((0..2).map(|k| l[i][k] * u[k][j]).sum(), a[p[i]][j]);
        }
    }
    let f = evaluate(&mut ev, "qr([[1.0,2.0],[3.0,4.0],[5.0,6.0]])");
    let q = matrix(field(&f, "q"));
    let r = matrix(field(&f, "r"));
    let a = [[1., 2.], [3., 4.], [5., 6.]];
    for i in 0..3 {
        for j in 0..2 {
            close((0..3).map(|k| q[i][k] * r[k][j]).sum(), a[i][j]);
        }
    }
    assert!(matches!(field(&f, "method").kind(), ExprKind::String(_)));
}
#[test]
fn numeric_inverse_determinant_and_explicit_local_solve_use_real_algorithms() {
    let mut ev = Evaluator::new();
    close(
        evaluate(&mut ev, "det([[1.0,2.0],[3.0,4.0]])")
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap(),
        -2.,
    );
    let inverse = matrix(&evaluate(&mut ev, "inverse([[1.0,2.0],[3.0,4.0]])"));
    let a = [[1., 2.], [3., 4.]];
    for (i, row) in a.iter().enumerate() {
        for (j, _) in row.iter().enumerate() {
            close(
                (0..2).map(|k| row[k] * inverse[k][j]).sum(),
                f64::from(i == j),
            );
        }
    }
    let x = evaluate(
        &mut ev,
        "linear_solve([[2.0,1.0],[1.0,3.0]],[1.0,2.0],mode:\"numeric\")",
    );
    close(x.args()[0].as_number().unwrap().to_f64().unwrap(), 0.2);
    close(x.args()[1].as_number().unwrap().to_f64().unwrap(), 0.6);
    let f = evaluate(
        &mut ev,
        "least_squares([[1.0,0.0],[1.0,1.0],[1.0,2.0]],[1.0,2.0,2.0])",
    );
    let x = field(&f, "solution");
    close(x.args()[0].as_number().unwrap().to_f64().unwrap(), 7. / 6.);
    close(x.args()[1].as_number().unwrap().to_f64().unwrap(), 0.5);
    close(
        field(&f, "residual_norm")
            .as_number()
            .unwrap()
            .to_f64()
            .unwrap(),
        (1. / 6f64).sqrt(),
    );
}
#[test]
fn rank_deficiency_invalid_modes_and_nonreal_inputs_decline() {
    let mut ev = Evaluator::new();
    for s in [
        "inverse([[1.0,2.0],[2.0,4.0]])",
        "least_squares([[1.0,2.0],[2.0,4.0]],[1.0,2.0])",
        "qr([[i,1],[2,3]])",
        "linear_solve([[1.0]],[1.0])",
    ] {
        ev.messages.take();
        let value = evaluate(&mut ev, s);
        assert!(!ev.messages.take().is_empty(), "{s}: {value:?}");
    }
}

#[test]
fn symmetric_eigenvalues_and_cholesky_expose_actual_sampled_algebra() {
    let mut ev = Evaluator::new();
    let l = matrix(&evaluate(&mut ev, "cholesky([[4.0,2.0],[2.0,3.0]])"));
    let a = [[4., 2.], [2., 3.]];
    for i in 0..2 {
        for j in 0..2 {
            close((0..2).map(|k| l[i][k] * l[j][k]).sum(), a[i][j]);
        }
    }
    let eigen = evaluate(&mut ev, "eigenvalues([[2.0,1.0],[1.0,2.0]])");
    close(eigen.args()[0].as_number().unwrap().to_f64().unwrap(), 1.);
    close(eigen.args()[1].as_number().unwrap().to_f64().unwrap(), 3.);
    let e = evaluate(&mut ev, "eigensystem([[2.0,1.0],[1.0,2.0]])");
    assert!(field(&e, "residual").as_number().unwrap().to_f64().unwrap() < 1e-12);
    for s in [
        "eigenvalues([[1.0,2.0],[0.0,1.0]])",
        "cholesky([[1.0,2.0],[2.0,1.0]])",
    ] {
        ev.messages.take();
        evaluate(&mut ev, s);
        assert!(!ev.messages.take().is_empty());
    }
}

#[test]
fn singular_value_factors_reconstruct_and_are_not_placeholder_answers() {
    let mut ev = Evaluator::new();
    let e = evaluate(&mut ev, "svd([[1.0,2.0],[3.0,4.0],[5.0,6.0]])");
    let u = matrix(field(&e, "u"));
    let s = matrix(field(&e, "s"));
    let v = matrix(field(&e, "v"));
    let a = [[1., 2.], [3., 4.], [5., 6.]];
    for i in 0..3 {
        for j in 0..2 {
            close(
                (0..3)
                    .map(|k| u[i][k] * (0..2).map(|l| s[k][l] * v[j][l]).sum::<f64>())
                    .sum(),
                a[i][j],
            );
        }
    }
    assert!(field(&e, "residual").as_number().unwrap().to_f64().unwrap() < 1e-11);
}
