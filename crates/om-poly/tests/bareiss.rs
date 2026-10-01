//! Bareiss determinant/linear certificates with independent exact matrix oracles.
use om_num::{
    Integer, Rational,
    ctx::{Abort, Interrupt},
};
use om_poly::{
    ExactFraction, LinearResult, MPoly, MonoOrder, Monomial, bareiss, determinant, linear_solve,
};
use proptest::prelude::*;
fn matrix(a: &[&[i64]]) -> Vec<Vec<Integer>> {
    a.iter()
        .map(|r| r.iter().map(|x| Integer::from(*x)).collect())
        .collect()
}
fn as_q(f: &ExactFraction<Integer>) -> Rational {
    Rational::from(f.num.clone()) / Rational::from(f.den.clone())
}
fn p(c: &[i64]) -> MPoly<Integer> {
    MPoly::new(
        1,
        c.iter()
            .enumerate()
            .map(|(i, c)| (Monomial::new([i as u32]).unwrap(), Integer::from(*c)))
            .collect(),
        MonoOrder::Lex,
        &Interrupt::default(),
    )
    .unwrap()
}
#[test]
fn authority_determinant_and_unique_three_by_three_solution() {
    let ctx = Interrupt::default();
    assert_eq!(
        determinant(&matrix(&[&[2, 1], &[1, 3]]), &ctx).unwrap(),
        Some(Integer::from(5))
    );
    let a = matrix(&[&[1, 1, 1], &[2, -1, 1], &[1, 2, -1]]);
    let b = vec![6.into(), 3.into(), 2.into()];
    let LinearResult::Consistent(solution) = linear_solve(&a, &b, 3, &ctx).unwrap().unwrap() else {
        panic!()
    };
    assert_eq!(
        solution.particular.iter().map(as_q).collect::<Vec<_>>(),
        vec![1.into(), 2.into(), 3.into()]
    );
    assert!(solution.nullspace.is_empty());
    assert!(solution.free_columns.is_empty());
    assert!(solution.assumptions.is_empty());
}
#[test]
fn authority_underdetermined_and_inconsistent_systems() {
    let ctx = Interrupt::default();
    let a = matrix(&[&[1, 1, 1], &[1, -1, 0]]);
    let b = vec![1.into(), 0.into()];
    let LinearResult::Consistent(solution) = linear_solve(&a, &b, 3, &ctx).unwrap().unwrap() else {
        panic!()
    };
    let half = Rational::ONE / Rational::from(2);
    assert_eq!(
        solution.particular.iter().map(as_q).collect::<Vec<_>>(),
        vec![half.clone(), half.clone(), Rational::ZERO]
    );
    assert_eq!(solution.free_columns, vec![2]);
    assert_eq!(
        solution.nullspace[0].iter().map(as_q).collect::<Vec<_>>(),
        vec![-half.clone(), -half, Rational::ONE]
    );
    assert!(matches!(
        linear_solve(&matrix(&[&[1, 1], &[1, 1]]), &[1.into(), 2.into()], 2, &ctx).unwrap(),
        Some(LinearResult::Inconsistent)
    ));
    let all_free = linear_solve::<Integer>(&[], &[], 3, &ctx).unwrap().unwrap();
    let LinearResult::Consistent(all_free) = all_free else {
        panic!()
    };
    assert_eq!(all_free.free_columns, vec![0, 1, 2]);
    assert_eq!(all_free.nullspace.len(), 3);
}
#[test]
fn parameter_authority_uses_only_needed_generic_assumption_and_cancels_fractions() {
    let ctx = Interrupt::default();
    let a = vec![vec![p(&[0, 1]), p(&[1])], vec![p(&[1]), p(&[-1])]];
    let b = vec![p(&[1]), p(&[0])];
    let LinearResult::Consistent(solution) = linear_solve(&a, &b, 2, &ctx).unwrap().unwrap() else {
        panic!()
    };
    assert_eq!(solution.assumptions, vec![p(&[1, 1])]);
    for x in &solution.particular {
        assert_eq!(x.num, p(&[1]));
        assert_eq!(x.den, p(&[1, 1]));
    }
    let reduced = ExactFraction::new(p(&[-1, 0, 1]), p(&[-1, 1]), &ctx)
        .unwrap()
        .unwrap();
    assert_eq!(reduced.num, p(&[1, 1]));
    assert!(reduced.den.is_one());
    for row in 0..2 {
        let mut total = ExactFraction::new(p(&[0]), p(&[1]), &ctx).unwrap().unwrap();
        for (col, coefficient) in a[row].iter().enumerate() {
            let c = ExactFraction::new(coefficient.clone(), p(&[1]), &ctx)
                .unwrap()
                .unwrap();
            total = total
                .add(
                    &c.mul(&solution.particular[col], &ctx).unwrap().unwrap(),
                    &ctx,
                )
                .unwrap()
                .unwrap();
        }
        assert_eq!(total.num, b[row]);
        assert!(total.den.is_one());
    }
    assert_eq!(determinant(&a, &ctx).unwrap(), Some(p(&[-1, -1])));
}
#[test]
fn swaps_skipped_columns_zero_size_and_shape_context_rejections() {
    let ctx = Interrupt::default();
    assert_eq!(
        determinant::<Integer>(&[], &ctx).unwrap(),
        Some(Integer::ONE)
    );
    assert_eq!(
        determinant(&matrix(&[&[0, 2], &[3, 1]]), &ctx).unwrap(),
        Some(Integer::from(-6))
    );
    let a = matrix(&[&[0, 2, 4, 1], &[0, 0, 3, 2], &[0, 0, 0, 0]]);
    let result = bareiss(&a, 3, &ctx).unwrap().unwrap();
    assert_eq!(result.pivots, vec![(0, 1), (1, 2)]);
    assert_eq!(result.matrix[1][1], Integer::ZERO);
    assert!(
        bareiss(&matrix(&[&[1, 2], &[3]]), 1, &ctx)
            .unwrap()
            .is_none()
    );
    assert!(
        linear_solve(&matrix(&[&[1, 2]]), &[], 2, &ctx)
            .unwrap()
            .is_none()
    );
    assert!(
        determinant(&matrix(&[&[1, 2, 3], &[4, 5, 6]]), &ctx)
            .unwrap()
            .is_none()
    );
    let incompatible = MPoly::new(
        2,
        vec![(Monomial::new([1, 0]).unwrap(), Integer::ONE)],
        MonoOrder::Lex,
        &ctx,
    )
    .unwrap();
    assert!(
        bareiss(&[vec![p(&[1]), incompatible]], 1, &ctx)
            .unwrap()
            .is_none()
    );
    let f = ExactFraction::new(Integer::from(-6), Integer::from(-8), &ctx)
        .unwrap()
        .unwrap();
    assert_eq!(as_q(&f), Rational::from(3) / Rational::from(4));
    assert!(
        ExactFraction::new(Integer::ONE, Integer::ZERO, &ctx)
            .unwrap()
            .is_none()
    );
}
#[test]
fn parameter_contradictions_preserve_their_nonzero_pivot_conditions() {
    let ctx = Interrupt::default();
    let a = vec![vec![p(&[0, 1])], vec![p(&[0, 1])]];
    let b = vec![p(&[0]), p(&[0, 1])];
    let result = linear_solve(&a, &b, 1, &ctx).unwrap().unwrap();
    let LinearResult::GenericInconsistent { assumptions } = result else {
        panic!("expected conditional contradiction");
    };
    assert_eq!(assumptions, vec![p(&[0, 1])]);
    let zero = linear_solve(
        &[vec![p(&[0])], vec![p(&[0])]],
        &[p(&[0]), p(&[0])],
        1,
        &ctx,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(zero, LinearResult::Consistent(_)));
    let only_rhs = linear_solve(&[vec![p(&[0])]], &[p(&[0, 1])], 1, &ctx)
        .unwrap()
        .unwrap();
    let LinearResult::GenericInconsistent { assumptions } = only_rhs else {
        panic!("parameter rhs needs its own nonzero condition");
    };
    assert_eq!(assumptions, vec![p(&[0, 1])]);
}
fn perm_det(a: &[Vec<Integer>]) -> Integer {
    fn visit(
        a: &[Vec<Integer>],
        row: usize,
        used: &mut Vec<usize>,
        value: Integer,
        total: &mut Integer,
    ) {
        if row == a.len() {
            let inv = used
                .iter()
                .enumerate()
                .map(|(i, x)| used[i + 1..].iter().filter(|y| *y < x).count())
                .sum::<usize>();
            *total += if inv % 2 == 0 { value } else { -value };
            return;
        }
        for col in 0..a.len() {
            if !used.contains(&col) {
                used.push(col);
                visit(a, row + 1, used, value.clone() * &a[row][col], total);
                used.pop();
            }
        }
    }
    let mut total = Integer::ZERO;
    visit(a, 0, &mut vec![], Integer::ONE, &mut total);
    total
}
fn rank(a: &[Vec<Integer>]) -> usize {
    let mut a: Vec<Vec<Rational>> = a
        .iter()
        .map(|r| r.iter().cloned().map(Rational::from).collect())
        .collect();
    let cols = a.first().map_or(0, |r| r.len());
    let mut rank = 0;
    for col in 0..cols {
        let Some(pivot) = (rank..a.len()).find(|r| a[*r][col] != Rational::ZERO) else {
            continue;
        };
        a.swap(pivot, rank);
        let row = a[rank].clone();
        for r in a.iter_mut().skip(rank + 1) {
            let c = &r[col] / &row[col];
            for j in col..cols {
                r[j] -= &c * &row[j];
            }
        }
        rank += 1;
    }
    rank
}
#[test]
fn budgets_and_external_cancel_cover_elimination_and_back_substitution() {
    let ctx = Interrupt::default();
    let a = matrix(&[&[2, 1, 3, 4], &[1, 3, 2, 5], &[3, 2, 5, 1], &[4, 5, 1, 7]]);
    for budget in [0, 10, 100] {
        ctx.steps_left.set(budget);
        assert!(matches!(
            linear_solve(&a, &[1.into(), 2.into(), 3.into(), 4.into()], 4, &ctx),
            Err(Abort::Budget)
        ));
    }
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        determinant::<Integer>(&[], &ctx),
        Err(Abort::Interrupted)
    ));
}
proptest! {
 #![proptest_config(ProptestConfig{cases:128,rng_seed:proptest::test_runner::RngSeed::Fixed(0x4d372e34),..ProptestConfig::default()})]
 #[test]
 fn determinants_equal_independent_leibniz_expansion(n in 0usize..=4,c in prop::collection::vec(-5i64..=5,16)){
  let ctx=Interrupt::default();let a:Vec<Vec<Integer>>=(0..n).map(|r|(0..n).map(|j|Integer::from(c[r*n+j])).collect()).collect();prop_assert_eq!(determinant(&a,&ctx).unwrap().unwrap(),perm_det(&a));
 }
 #[test]
 fn planted_rectangular_solutions_and_complete_nullspace_are_certified(rows in 0usize..=4,cols in 0usize..=4,c in prop::collection::vec(-3i64..=3,16),x in prop::collection::vec(-3i64..=3,4)){
  let ctx=Interrupt::default();let a:Vec<Vec<Integer>>=(0..rows).map(|r|(0..cols).map(|j|Integer::from(c[r*4+j])).collect()).collect();let b:Vec<Integer>=a.iter().map(|row|row.iter().enumerate().map(|(j,c)|c*Integer::from(x[j])).sum()).collect();
  let LinearResult::Consistent(s)=linear_solve(&a,&b,cols,&ctx).unwrap().unwrap() else{panic!()};prop_assert_eq!(s.nullspace.len(),cols-rank(&a));prop_assert_eq!(s.nullspace.len(),s.free_columns.len());
  for (i,row) in a.iter().enumerate(){let sum:Rational=row.iter().zip(&s.particular).map(|(c,x)|Rational::from(c.clone())*as_q(x)).fold(Rational::ZERO,|a,b|a+b);prop_assert_eq!(sum,Rational::from(b[i].clone()));for v in &s.nullspace{let sum:Rational=row.iter().zip(v).map(|(c,x)|Rational::from(c.clone())*as_q(x)).fold(Rational::ZERO,|a,b|a+b);prop_assert_eq!(sum,Rational::ZERO);}}
  for (j,&column) in s.free_columns.iter().enumerate(){for (k,v) in s.nullspace.iter().enumerate(){prop_assert_eq!(as_q(&v[column]),if j==k{Rational::ONE}else{Rational::ZERO});}}
 }
}
