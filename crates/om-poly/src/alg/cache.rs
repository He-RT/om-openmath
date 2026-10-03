//! Only complete immutable root batches are retained; callers always get owned clones.
use super::{Algebraic, Poly};
use om_num::BitTest;
use std::cell::RefCell;
struct Entry {
    polynomial: Poly,
    roots: Vec<Algebraic>,
}
thread_local! {
    static BATCHES: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
}
fn eligible(p: &Poly) -> bool {
    p.degree().is_some_and(|n| n <= 64) && p.coeffs.iter().all(|c| c.bit_len() <= 4096)
}
pub(super) fn get(p: &Poly) -> Option<Vec<Algebraic>> {
    if !eligible(p) {
        return None;
    }
    BATCHES.with(|batches| {
        let mut batches = batches.borrow_mut();
        let position = batches.iter().position(|entry| entry.polynomial == *p)?;
        let entry = batches.remove(position);
        let roots = entry.roots.clone();
        batches.push(entry);
        Some(roots)
    })
}
pub(super) fn remember(p: &Poly, roots: &[Algebraic]) {
    if !eligible(p) {
        return;
    }
    BATCHES.with(|batches| {
        let mut batches = batches.borrow_mut();
        if batches.len() == 8 {
            batches.remove(0);
        }
        batches.push(Entry {
            polynomial: p.clone(),
            roots: roots.to_vec(),
        });
    });
}
