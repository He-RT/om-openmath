//! Column HNF lattice/unimodular certificates and exact integer solution families.
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
use om_poly::{HermiteForm, IntegerLinearResult, column_hnf, determinant, integer_linear_solve};
use proptest::prelude::*;
fn matrix(a: &[&[i64]]) -> Vec<Vec<Integer>> {
    a.iter()
        .map(|r| r.iter().map(|x| Integer::from(*x)).collect())
        .collect()
}
fn multiply(a: &[Vec<Integer>], u: &[Vec<Integer>], cols: usize) -> Vec<Vec<Integer>> {
    a.iter()
        .map(|r| {
            (0..cols)
                .map(|j| r.iter().enumerate().map(|(k, c)| c * &u[k][j]).sum())
                .collect()
        })
        .collect()
}
fn certificate(a: &[Vec<Integer>], cols: usize, h: &HermiteForm, ctx: &Interrupt) {
    assert_eq!(h.u.len(), cols);
    for row in &h.u {
        assert_eq!(row.len(), cols);
    }
    let d = determinant(&h.u, ctx).unwrap().unwrap();
    assert!(d == Integer::ONE || d == Integer::NEG_ONE);
    let mut target = h.h.clone();
    for row in &mut target {
        row.resize(cols, Integer::ZERO);
    }
    assert_eq!(multiply(a, &h.u, cols), target);
    assert_eq!(h.pivot_rows.len(), h.h.first().map_or(0, |r| r.len()));
    assert!(h.pivot_rows.windows(2).all(|p| p[0] < p[1]));
    for (j, &r) in h.pivot_rows.iter().enumerate() {
        let pivot = &h.h[r][j];
        assert!(pivot > &Integer::ZERO);
        for row in h.h.iter().skip(r + 1) {
            assert_eq!(row[j], Integer::ZERO);
        }
        for k in j + 1..h.pivot_rows.len() {
            assert!(h.h[r][k] >= Integer::ZERO && &h.h[r][k] < pivot);
        }
    }
}
#[test]
fn canonical_authority_lattice_forms_and_rank_deficiency() {
    let ctx = Interrupt::default();
    for (a, expected) in [
        (matrix(&[&[2, 3]]), matrix(&[&[1]])),
        (matrix(&[&[2, 4], &[6, 8]]), matrix(&[&[4, 2], &[0, 2]])),
        (
            matrix(&[&[2, 4, 6], &[1, 2, 3], &[0, 0, 0]]),
            matrix(&[&[2], &[1], &[0]]),
        ),
        (
            matrix(&[&[-2, 0, 0], &[0, -3, 0], &[0, 0, 4]]),
            matrix(&[&[2, 0, 0], &[0, 3, 0], &[0, 0, 4]]),
        ),
    ] {
        let cols = a[0].len();
        let form = column_hnf(&a, cols, &ctx).unwrap().unwrap();
        assert_eq!(form.h, expected);
        certificate(&a, cols, &form, &ctx);
    }
}
#[test]
fn authority_diophantine_two_x_plus_three_y_has_complete_integer_family() {
    let ctx = Interrupt::default();
    let a = matrix(&[&[2, 3]]);
    let b = vec![Integer::ONE];
    let IntegerLinearResult::Consistent(s) =
        integer_linear_solve(&a, &b, 2, &ctx).unwrap().unwrap()
    else {
        panic!()
    };
    assert_eq!(s.nullspace.len(), 1);
    for parameter in -20..=20 {
        let x: Vec<_> = s
            .particular
            .iter()
            .zip(&s.nullspace[0])
            .map(|(a, b)| a + b * Integer::from(parameter))
            .collect();
        assert_eq!(&x[0] * 2 + &x[1] * 3, Integer::ONE);
    }
    // Every small integer solution maps back to an integer kernel parameter.
    for x in -20..=20 {
        for y in -20..=20 {
            if 2 * x + 3 * y == 1 {
                let dx = Integer::from(x) - &s.particular[0];
                let dy = Integer::from(y) - &s.particular[1];
                let k = &dx / &s.nullspace[0][0];
                assert_eq!(&k * &s.nullspace[0][0], dx);
                assert_eq!(k * &s.nullspace[0][1], dy);
            }
        }
    }
    certificate(&a, 2, &s.hermite, &ctx);
}
#[test]
fn divisibility_overdetermined_zero_and_shape_boundaries() {
    let ctx = Interrupt::default();
    assert!(matches!(
        integer_linear_solve(&matrix(&[&[2, 4]]), &[1.into()], 2, &ctx).unwrap(),
        Some(IntegerLinearResult::Inconsistent)
    ));
    assert!(matches!(
        integer_linear_solve(&matrix(&[&[1], &[2]]), &[1.into(), 3.into()], 1, &ctx).unwrap(),
        Some(IntegerLinearResult::Inconsistent)
    ));
    let IntegerLinearResult::Consistent(s) =
        integer_linear_solve(&matrix(&[&[1], &[2]]), &[1.into(), 2.into()], 1, &ctx)
            .unwrap()
            .unwrap()
    else {
        panic!()
    };
    assert_eq!(s.particular, vec![Integer::ONE]);
    assert!(s.nullspace.is_empty());
    let IntegerLinearResult::Consistent(s) =
        integer_linear_solve(&[], &[], 3, &ctx).unwrap().unwrap()
    else {
        panic!()
    };
    assert_eq!(s.particular, vec![Integer::ZERO; 3]);
    assert_eq!(s.nullspace.len(), 3);
    let form = column_hnf(&vec![vec![Integer::ZERO; 3]; 2], 3, &ctx)
        .unwrap()
        .unwrap();
    certificate(&vec![vec![Integer::ZERO; 3]; 2], 3, &form, &ctx);
    assert!(
        column_hnf(&matrix(&[&[1, 2], &[3]]), 2, &ctx)
            .unwrap()
            .is_none()
    );
    assert!(
        integer_linear_solve(&matrix(&[&[1]]), &[], 1, &ctx)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        integer_linear_solve(&[vec![]], &[Integer::ONE], 0, &ctx).unwrap(),
        Some(IntegerLinearResult::Inconsistent)
    ));
}
#[test]
fn large_signed_coefficients_and_interruptible_gcd_column_updates() {
    let ctx = Interrupt::default();
    let n = Integer::ONE << 200;
    let a = vec![vec![&n + 1_u8, -&n]];
    let form = column_hnf(&a, 2, &ctx).unwrap().unwrap();
    assert_eq!(form.h, vec![vec![Integer::ONE]]);
    certificate(&a, 2, &form, &ctx);
    let a = matrix(&[&[2, 4, 3], &[7, 9, 11], &[17, 19, 23]]);
    for budget in [0, 5, 50] {
        ctx.steps_left.set(budget);
        assert!(matches!(column_hnf(&a, 3, &ctx), Err(Abort::Budget)));
    }
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        integer_linear_solve(&[], &[], 0, &ctx),
        Err(Abort::Interrupted)
    ));
}
proptest! {
 #![proptest_config(ProptestConfig{cases:128,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d372e35),..ProptestConfig::default()})]
 #[test]
 fn column_form_is_invariant_under_elementary_unimodular_changes(rows in 1usize..=4,cols in 1usize..=4,c in prop::collection::vec(-7i64..=7,16),shear in -5i64..=5){
  let ctx=Interrupt::default();let a:Vec<Vec<Integer>>=(0..rows).map(|r|(0..cols).map(|j|Integer::from(c[r*4+j])).collect()).collect();let form=column_hnf(&a,cols,&ctx).unwrap().unwrap();certificate(&a,cols,&form,&ctx);
  let mut changed=a.clone();for row in &mut changed{row[0] = -&row[0];if cols>1{row[1]=&row[1]+&row[0]*Integer::from(shear);row.swap(0,cols-1);}}
  let alternate=column_hnf(&changed,cols,&ctx).unwrap().unwrap();certificate(&changed,cols,&alternate,&ctx);prop_assert_eq!(form.h,alternate.h);prop_assert_eq!(form.pivot_rows,alternate.pivot_rows);
 }
 #[test]
 fn planted_integer_systems_return_all_solutions_over_z(rows in 0usize..=4,cols in 0usize..=4,c in prop::collection::vec(-4i64..=4,16),x in prop::collection::vec(-5i64..=5,4)){
  let ctx=Interrupt::default();let a:Vec<Vec<Integer>>=(0..rows).map(|r|(0..cols).map(|j|Integer::from(c[r*4+j])).collect()).collect();let b:Vec<Integer>=a.iter().map(|r|r.iter().enumerate().map(|(j,c)|c*Integer::from(x[j])).sum()).collect();let IntegerLinearResult::Consistent(s)=integer_linear_solve(&a,&b,cols,&ctx).unwrap().unwrap() else{panic!()};certificate(&a,cols,&s.hermite,&ctx);
  prop_assert_eq!(s.nullspace.len(),cols-s.hermite.pivot_rows.len());for (i,row) in a.iter().enumerate(){let value:Integer=row.iter().zip(&s.particular).map(|(c,x)|c*x).sum();prop_assert_eq!(value,b[i].clone());for v in &s.nullspace{let value:Integer=row.iter().zip(v).map(|(c,x)|c*x).sum();prop_assert_eq!(value,Integer::ZERO);}}
 }
}
