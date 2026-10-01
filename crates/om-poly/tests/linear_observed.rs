//! Observations certify actual Bareiss matrices and do not change unobserved solutions.
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
use om_poly::{BareissOp, LinearResult, linear_solve_observed};
#[test]
fn observed_fraction_free_updates_have_exact_certificates() {
    let a = vec![
        vec![Integer::from(0), Integer::from(2)],
        vec![Integer::from(3), Integer::from(1)],
    ];
    let b = vec![Integer::from(4), Integer::from(5)];
    let mut before = vec![
        vec![Integer::from(0), Integer::from(2), Integer::from(4)],
        vec![Integer::from(3), Integer::from(1), Integer::from(5)],
    ];
    let mut count = 0;
    let result = linear_solve_observed(&a, &b, 2, &Interrupt::default(), &mut |op, matrix| {
        match op {
            BareissOp::Swap { a, b } => before.swap(a, b),
            BareissOp::Eliminate {
                target,
                source,
                pivot,
                entry,
                previous,
            } => {
                let from = before[source].clone();
                for (v, f) in before[target].iter_mut().zip(from) {
                    let n = &pivot * &*v - &entry * f;
                    assert!((&n % &previous).is_zero());
                    *v = n / &previous
                }
            }
        }
        assert_eq!(&before, matrix);
        count += 1;
        Ok(())
    })
    .unwrap()
    .unwrap();
    assert!(matches!(result, LinearResult::Consistent(_)));
    assert!(count >= 2);
}
#[test]
fn observation_abort_is_propagated() {
    let a = vec![
        vec![Integer::from(1), Integer::from(1)],
        vec![Integer::from(1), Integer::from(2)],
    ];
    assert!(matches!(
        linear_solve_observed(
            &a,
            &[1.into(), 2.into()],
            2,
            &Interrupt::default(),
            &mut |_, _| Err(Abort::Interrupted)
        ),
        Err(Abort::Interrupted)
    ));
}
