//! Independent reconstruction, orthogonality, analytic solutions and portable abort contracts.
use om_analysis::{
    Error,
    matrix::{Matrix, lu, qr},
};
use om_num::ctx::{Abort, Interrupt};
fn matrix(rows: usize, cols: usize, data: &[f64]) -> Matrix {
    Matrix::new(rows, cols, data.to_vec()).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-11 * (1.0 + b.abs()), "{a} != {b}");
}
#[test]
fn pivoted_lu_reconstructs_and_solves_analytic_systems() {
    let ctx = Interrupt::default();
    let a = matrix(3, 3, &[0., 2., 1., 1., 1., 0., 2., 0., 1.]);
    let f = lu(&a, &ctx).unwrap();
    let product = f.l.multiply(&f.u, &ctx).unwrap();
    for i in 0..3 {
        for j in 0..3 {
            close(product.get(i, j), a.get(f.permutation[i], j));
        }
    }
    let b = [3., 3., 8.];
    let x = f.solve(&b, 1e-12, &ctx).unwrap();
    for (i, rhs) in b.iter().enumerate() {
        close((0..3).map(|j| a.get(i, j) * x[j]).sum(), *rhs);
    }
    close(f.determinant(&ctx).unwrap(), -4.);
    let singular = lu(&matrix(2, 2, &[1., 2., 2., 4.]), &ctx).unwrap();
    assert!(matches!(
        singular.solve(&[1., 2.], 1e-12, &ctx),
        Err(Error::Singular)
    ));
    let tiny = lu(&matrix(2, 2, &[1., 0., 0., 1e-15]), &ctx).unwrap();
    assert!(matches!(
        tiny.solve(&[1., 1.], 1e-12, &ctx),
        Err(Error::IllConditioned)
    ));
}
#[test]
fn householder_qr_reconstructs_rectangles_and_is_orthogonal() {
    let ctx = Interrupt::default();
    for a in [
        matrix(3, 2, &[1., 2., 3., 4., 5., 6.]),
        matrix(2, 3, &[1., 2., 3., 4., 5., 6.]),
        matrix(3, 2, &[0., 0., 0., 0., 0., 0.]),
    ] {
        let f = qr(&a, &ctx).unwrap();
        let product = f.q.multiply(&f.r, &ctx).unwrap();
        for (x, y) in product.data().iter().zip(a.data()) {
            close(*x, *y);
        }
        let qtq = f.q.transpose(&ctx).unwrap().multiply(&f.q, &ctx).unwrap();
        for i in 0..a.rows() {
            for j in 0..a.rows() {
                close(qtq.get(i, j), f64::from(i == j));
            }
        }
    }
}
#[test]
fn least_squares_matches_independent_normal_equations_residual_and_large_scales() {
    let ctx = Interrupt::default();
    let a = matrix(3, 2, &[1., 0., 1., 1., 1., 2.]);
    let f = qr(&a, &ctx).unwrap();
    let x = f.least_squares(&[1., 2., 2.], 1e-12, &ctx).unwrap();
    close(x[0], 7. / 6.);
    close(x[1], 0.5);
    let residual: Vec<_> = (0..3)
        .map(|i| a.get(i, 0) * x[0] + a.get(i, 1) * x[1] - [1., 2., 2.][i])
        .collect();
    for j in 0..2 {
        close((0..3).map(|i| a.get(i, j) * residual[i]).sum(), 0.);
    }
    let huge = matrix(2, 1, &[1e200, 1e200]);
    let f = qr(&huge, &ctx).unwrap();
    let x = f.least_squares(&[2e200, 2e200], 1e-12, &ctx).unwrap();
    close(x[0], 2.);
}
#[test]
fn invalid_shapes_nonfinite_overflow_and_abort_cannot_report_success() {
    assert!(Matrix::new(65, 1, vec![0.; 65]).is_err());
    assert!(Matrix::new(1, 1, vec![f64::NAN]).is_err());
    let a = matrix(3, 3, &[1., 2., 3., 4., 5., 6., 7., 8., 9.]);
    {
        let algorithm = lu as fn(&Matrix, &Interrupt) -> Result<_, _>;
        let ctx = Interrupt::default();
        ctx.steps_left.set(2);
        assert!(matches!(
            algorithm(&a, &ctx),
            Err(Error::Abort(Abort::Budget))
        ));
    }
    let ctx = Interrupt::default();
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        qr(&a, &ctx),
        Err(Error::Abort(Abort::Interrupted))
    ));
}

#[test]
fn cholesky_and_real_symmetric_eigenvectors_have_independent_certifying_residuals() {
    use om_analysis::matrix::{cholesky, eigensystem};
    let ctx = Interrupt::default();
    let a = matrix(3, 3, &[4., 2., 0., 2., 5., 1., 0., 1., 3.]);
    let l = cholesky(&a, &ctx).unwrap();
    let reconstruction = l.multiply(&l.transpose(&ctx).unwrap(), &ctx).unwrap();
    for (x, y) in a.data().iter().zip(reconstruction.data()) {
        close(*x, *y);
    }
    for a in [
        matrix(2, 2, &[2., 1., 1., 2.]),
        matrix(3, 3, &[2., 0., 0., 0., 2., 0., 0., 0., 2.]),
        matrix(2, 2, &[2e200, 1e200, 1e200, 2e200]),
    ] {
        let e = eigensystem(&a, &ctx).unwrap();
        for j in 0..a.cols() {
            for i in 0..a.rows() {
                let av: f64 = (0..a.cols())
                    .map(|k| a.get(i, k) * e.vectors.get(k, j))
                    .sum();
                close(av, e.values[j] * e.vectors.get(i, j));
            }
        }
        let gram = e
            .vectors
            .transpose(&ctx)
            .unwrap()
            .multiply(&e.vectors, &ctx)
            .unwrap();
        for i in 0..a.rows() {
            for j in 0..a.cols() {
                close(gram.get(i, j), f64::from(i == j));
            }
        }
    }
    let eigen = eigensystem(&matrix(2, 2, &[2., 1., 1., 2.]), &ctx).unwrap();
    close(eigen.values[0], 1.);
    close(eigen.values[1], 3.);
    assert!(cholesky(&matrix(2, 2, &[1., 2., 2., 1.]), &ctx).is_err());
    assert!(eigensystem(&matrix(2, 2, &[1., 2., 0., 1.]), &ctx).is_err());
}

#[test]
fn one_sided_svd_reconstructs_rectangles_and_both_orthogonal_factors() {
    use om_analysis::matrix::svd;
    let ctx = Interrupt::default();
    for a in [
        matrix(3, 2, &[1., 2., 3., 4., 5., 6.]),
        matrix(2, 3, &[1., 2., 3., 4., 5., 6.]),
        matrix(2, 2, &[1., 2., 2., 4.]),
        matrix(3, 2, &[0., 0., 0., 0., 0., 0.]),
        matrix(2, 2, &[1., 0., 0., 1e-150]),
    ] {
        let f = svd(&a, &ctx).unwrap();
        let rebuilt =
            f.u.multiply(&f.s, &ctx)
                .unwrap()
                .multiply(&f.v.transpose(&ctx).unwrap(), &ctx)
                .unwrap();
        for (x, y) in rebuilt.data().iter().zip(a.data()) {
            close(*x, *y);
        }
        for factor in [&f.u, &f.v] {
            let gram = factor
                .transpose(&ctx)
                .unwrap()
                .multiply(factor, &ctx)
                .unwrap();
            for i in 0..gram.rows() {
                for j in 0..gram.cols() {
                    close(gram.get(i, j), f64::from(i == j));
                }
            }
        }
        assert!(f.values.windows(2).all(|v| v[0] >= v[1] && v[1] >= 0.));
    }
    let f = svd(&matrix(2, 2, &[3., 0., 0., 4.]), &ctx).unwrap();
    close(f.values[0], 4.);
    close(f.values[1], 3.);
    assert!(svd(&matrix(2, 2, &[1e308, 0., 0., 1e-308]), &ctx).is_err());
}

#[test]
fn deterministic_small_matrix_properties_cover_ranks_rectangles_and_repeated_spectra() {
    use om_analysis::matrix::{eigensystem, svd};
    let ctx = Interrupt::default();
    let mut rng = om_num::rng::SplitMix64::new(9);
    for index in 0..48 {
        let m = 2 + (index % 4);
        let n = 2 + ((index / 4) % 4);
        let k = 1 + index % m.min(n);
        let left: Vec<_> = (0..m * k)
            .map(|_| rng.next_range(0, 11) as f64 - 5.)
            .collect();
        let right: Vec<_> = (0..k * n)
            .map(|_| rng.next_range(0, 11) as f64 - 5.)
            .collect();
        let a = matrix(m, k, &left)
            .multiply(&matrix(k, n, &right), &ctx)
            .unwrap();
        let f = svd(&a, &ctx).unwrap_or_else(|e| panic!("case={index}, m={m}, n={n}, k={k}: {e}"));
        let reconstructed =
            f.u.multiply(&f.s, &ctx)
                .unwrap()
                .multiply(&f.v.transpose(&ctx).unwrap(), &ctx)
                .unwrap();
        for (x, y) in reconstructed.data().iter().zip(a.data()) {
            close(*x, *y);
        }
        for factor in [&f.u, &f.v] {
            let gram = factor
                .transpose(&ctx)
                .unwrap()
                .multiply(factor, &ctx)
                .unwrap();
            for i in 0..gram.rows() {
                for j in 0..gram.cols() {
                    close(gram.get(i, j), f64::from(i == j));
                }
            }
        }
        let lu = lu(&a, &ctx).unwrap();
        let reconstructed = lu.l.multiply(&lu.u, &ctx).unwrap();
        for i in 0..m {
            for j in 0..n {
                close(reconstructed.get(i, j), a.get(lu.permutation[i], j));
            }
        }
        let symmetric = a.transpose(&ctx).unwrap().multiply(&a, &ctx).unwrap();
        let eigen = eigensystem(&symmetric, &ctx).unwrap();
        for j in 0..n {
            for i in 0..n {
                let av: f64 = (0..n)
                    .map(|k| symmetric.get(i, k) * eigen.vectors.get(k, j))
                    .sum();
                assert!((av - eigen.values[j] * eigen.vectors.get(i, j)).abs() < 1e-8);
            }
        }
    }
}

#[test]
fn svd_solution_exposes_rectangular_free_axes_and_minimum_norm_without_fake_consistency() {
    use om_analysis::matrix::svd;
    let ctx = Interrupt::default();
    let a = matrix(1, 2, &[1., 2.]);
    let f = svd(&a, &ctx).unwrap();
    let s = f.solve(&a, &[3.], 1e-12, &ctx).unwrap();
    assert!(s.consistent);
    assert_eq!(s.rank, 1);
    close(s.particular[0], 0.6);
    close(s.particular[1], 1.2);
    assert_eq!(s.nullspace.len(), 1);
    for basis in &s.nullspace {
        close(basis[0] + 2. * basis[1], 0.);
        close(
            basis.iter().zip(&s.particular).map(|(a, b)| a * b).sum(),
            0.,
        );
    }
    let a = matrix(3, 2, &[1., 0., 1., 1., 1., 2.]);
    let f = svd(&a, &ctx).unwrap();
    let s = f.solve(&a, &[1., 2., 2.], 1e-12, &ctx).unwrap();
    assert!(!s.consistent);
    close(s.particular[0], 7. / 6.);
    close(s.particular[1], 0.5);
    let s = f.solve(&a, &[1., 2., 3.], 1e-12, &ctx).unwrap();
    assert!(s.consistent);
    assert!(s.nullspace.is_empty());
    let a = matrix(2, 3, &[1., 2., 3., 2., 4., 6.]);
    let f = svd(&a, &ctx).unwrap();
    let s = f.solve(&a, &[1., 2.], 1e-12, &ctx).unwrap();
    assert!(s.consistent);
    assert_eq!(s.rank, 1);
    assert_eq!(s.nullspace.len(), 2);
    assert!(!f.solve(&a, &[1., 3.], 1e-12, &ctx).unwrap().consistent);
    let zero = matrix(2, 3, &[0.; 6]);
    let f = svd(&zero, &ctx).unwrap();
    let s = f.solve(&zero, &[0., 0.], 1e-12, &ctx).unwrap();
    assert_eq!(s.rank, 0);
    assert!(s.consistent);
    assert_eq!(s.nullspace.len(), 3);
    assert!(!f.solve(&zero, &[0., 1.], 1e-12, &ctx).unwrap().consistent);
    assert!(f.rank(-1.).is_err());
    assert!(f.rank(f64::NAN).is_err());
}
