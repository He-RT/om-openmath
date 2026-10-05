//! Stable indirect merge sorting checks the interrupt outside its total-order comparator.
use crate::EvalError;
use om_core::Interrupt;
use std::cmp::Ordering;
pub(super) fn indices(
    len: usize,
    ctx: &Interrupt,
    mut compare: impl FnMut(usize, usize) -> Ordering,
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
                    && (right == end || compare(source[left], source[right]) != Ordering::Greater)
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
