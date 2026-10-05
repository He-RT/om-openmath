//! Stable indirect merge sorting checks the interrupt outside its total-order comparator.
use crate::EvalError;
use om_core::Interrupt;
use std::cmp::Ordering;
pub(super) fn indices(
    len: usize,
    ctx: &Interrupt,
    mut compare: impl FnMut(usize, usize) -> Ordering,
) -> Result<Vec<usize>, EvalError> {
    indices_checked(len, ctx, |a, b| Ok(compare(a, b)))
}
pub(super) fn indices_checked(
    len: usize,
    ctx: &Interrupt,
    mut compare: impl FnMut(usize, usize) -> Result<Ordering, EvalError>,
) -> Result<Vec<usize>, EvalError> {
    ctx.tick()?;
    let mut source: Vec<_> = (0..len).collect();
    let mut target = vec![0; len];
    let mut width = 1;
    while width < len {
        for begin in (0..len).step_by(2 * width) {
            let middle = (begin + width).min(len);
            let end = (middle + width).min(len);
            let (mut left, mut right) = (begin, middle);
            for slot in &mut target[begin..end] {
                ctx.tick()?;
                if left < middle
                    && (right == end || compare(source[left], source[right])? != Ordering::Greater)
                {
                    *slot = source[left];
                    left += 1;
                } else {
                    *slot = source[right];
                    right += 1;
                }
            }
        }
        std::mem::swap(&mut source, &mut target);
        width *= 2;
    }
    Ok(source)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_comparison_returns_an_error_without_inventing_an_order() {
        let ctx = Interrupt::default();
        let result = indices_checked(4, &ctx, |_, _| Err(om_core::Abort::Interrupted.into()));
        assert!(matches!(
            result,
            Err(EvalError::Abort(om_core::Abort::Interrupted))
        ));
        let values = [2, 1, 2, 1];
        assert_eq!(
            indices_checked(4, &ctx, |a, b| Ok(values[a].cmp(&values[b]))).unwrap(),
            vec![1, 3, 0, 2]
        );
    }
}
