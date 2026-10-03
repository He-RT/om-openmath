//! Immutable proofs are reusable; rejected or interrupted computation is never memoized.
use super::RootDisk;
use crate::UPoly;
use om_num::{BitTest, Integer};
use std::cell::RefCell;
struct Entry {
    polynomial: UPoly<Integer>,
    bits: u32,
    disks: Vec<RootDisk>,
}
thread_local! {
    static ROOTS: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
}
fn eligible(p: &UPoly<Integer>, bits: u32) -> bool {
    bits <= 4096
        && p.degree().is_some_and(|degree| degree <= 64)
        && p.coeffs.iter().all(|c| c.bit_len() <= 4096)
}
pub(super) fn get(p: &UPoly<Integer>, bits: u32) -> Option<Vec<RootDisk>> {
    if !eligible(p, bits) {
        return None;
    }
    ROOTS.with(|roots| {
        let mut roots = roots.borrow_mut();
        let position = roots
            .iter()
            .position(|entry| entry.bits == bits && entry.polynomial == *p)?;
        let entry = roots.remove(position);
        let disks = entry.disks.clone();
        roots.push(entry);
        Some(disks)
    })
}
pub(super) fn remember(p: &UPoly<Integer>, bits: u32, disks: &[RootDisk]) {
    if !eligible(p, bits) {
        return;
    }
    ROOTS.with(|roots| {
        let mut roots = roots.borrow_mut();
        if roots.len() == 16 {
            roots.remove(0);
        }
        roots.push(Entry {
            polynomial: p.clone(),
            bits,
            disks: disks.to_vec(),
        });
    });
}
