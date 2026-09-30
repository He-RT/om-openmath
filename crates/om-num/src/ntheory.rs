//! Exact integer utilities and deterministic bounded factorization.

use crate::{Integer, rng::SplitMix64};
use dashu::base::BitTest;
use std::{collections::BTreeMap, sync::OnceLock};

#[path = "primality.rs"]
mod primality;

/// Nonnegative greatest common divisor, with gcd(0, 0) = 0.
pub fn gcd(a: &Integer, b: &Integer) -> Integer {
    let (mut a, mut b) = (abs(a), abs(b));
    while !b.is_zero() {
        (a, b) = (b.clone(), a % b);
    }
    a
}
/// Return (g, s, t) with g>=0 and s*a+t*b=g.
pub fn ext_gcd(a: &Integer, b: &Integer) -> (Integer, Integer, Integer) {
    let (mut old_r, mut r) = (abs(a), abs(b));
    let (mut old_s, mut s) = (Integer::ONE, Integer::ZERO);
    let (mut old_t, mut t) = (Integer::ZERO, Integer::ONE);
    while !r.is_zero() {
        let q = &old_r / &r;
        (old_r, r) = (r.clone(), old_r - &q * &r);
        (old_s, s) = (s.clone(), old_s - &q * &s);
        (old_t, t) = (t.clone(), old_t - &q * &t);
    }
    if a < &Integer::ZERO {
        old_s = -old_s;
    }
    if b < &Integer::ZERO {
        old_t = -old_t;
    }
    (old_r, old_s, old_t)
}
/// Floor square root; requires n>=0.
pub fn isqrt(n: &Integer) -> Integer {
    assert!(n >= &Integer::ZERO, "isqrt requires a nonnegative integer");
    if n <= &Integer::ONE {
        return n.clone();
    }
    let mut x = Integer::ONE << n.bit_len().div_ceil(2);
    loop {
        let next = (&x + n / &x) >> 1;
        if next >= x {
            return x;
        }
        x = next;
    }
}
/// Exact kth root; k=0 and negative even radicands return None.
pub fn exact_root(n: &Integer, k: u32) -> Option<Integer> {
    if k == 0 || (n < &Integer::ZERO && k.is_multiple_of(2)) {
        return None;
    }
    if k == 1 || n.is_zero() || abs(n).is_one() {
        return Some(n.clone());
    }
    let magnitude = abs(n);
    if k as usize >= magnitude.bit_len() {
        return None;
    }
    if k == 2 {
        let root = isqrt(&magnitude);
        return (&root * &root == magnitude).then_some(root);
    }
    let mut lo = Integer::ONE;
    let mut hi = Integer::ONE << magnitude.bit_len().div_ceil(k as usize);
    while lo <= hi {
        let mid = (&lo + &hi) >> 1;
        match mid.pow(k as usize).cmp(&magnitude) {
            std::cmp::Ordering::Equal => return Some(if n < &Integer::ZERO { -mid } else { mid }),
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid - 1,
        }
    }
    None
}
/// Largest perfect-power exponent; returns None for 0 and ±1.
pub fn perfect_power(n: &Integer) -> Option<(Integer, u32)> {
    let magnitude = abs(n);
    if magnitude <= Integer::ONE {
        return None;
    }
    let max = u32::try_from(magnitude.bit_len() - 1).expect("invariant: root exponent fits u32");
    for k in (2..=max).rev() {
        if let Some(root) = exact_root(n, k) {
            return Some((root, k));
        }
    }
    None
}
/// Deterministic primality below 2^64; strong Baillie-PSW above it.
pub fn is_probable_prime(n: &Integer) -> bool {
    primality::is_probable_prime(n)
}
/// Trial division and fixed-seed Pollard-Brent; budget exhaustion retains a cofactor.
/// Negative inputs include -1; zero returns [(0,1)]. Unresolved composites are last.
pub fn factor_integer(n: &Integer, ctx_budget: &mut u64) -> Vec<(Integer, u32)> {
    if n.is_zero() {
        return vec![(Integer::ZERO, 1)];
    }
    let mut remainder = abs(n);
    let mut found = BTreeMap::<Integer, u32>::new();
    if n < &Integer::ZERO {
        found.insert((-1).into(), 1);
    }
    if is_probable_prime(&remainder) {
        found.insert(remainder, 1);
        return found.into_iter().collect();
    }
    for &p in small_primes().iter().take_while(|&&p| p <= 10_000) {
        if Integer::from(p) * p > remainder {
            break;
        }
        loop {
            if !charge(ctx_budget) {
                return finish_factors(found, remainder);
            }
            if &remainder % p == 0 {
                remainder /= p;
                *found.entry(p.into()).or_default() += 1;
            } else {
                break;
            }
        }
    }
    let mut pending = vec![remainder];
    let mut unresolved = Integer::ONE;
    let mut rng = SplitMix64::new(0x4f4d_4e54_4845_4f52);
    while let Some(n) = pending.pop() {
        if n.is_one() {
            continue;
        }
        if is_probable_prime(&n) {
            *found.entry(n).or_default() += 1;
            continue;
        }
        if *ctx_budget == 0 {
            unresolved *= n;
            continue;
        }
        let root = isqrt(&n);
        if &root * &root == n {
            pending.push(root.clone());
            pending.push(root);
            continue;
        }
        match pollard_brent(&n, ctx_budget, &mut rng) {
            Some(p) => {
                pending.push(&n / &p);
                pending.push(p);
            }
            None => unresolved *= n,
        }
    }
    finish_factors(found, unresolved)
}
/// Extract bounded kth-power factors; requires k>0 and nonnegative n for even k.
/// Trial extraction is bounded; large unresolved composites can retain kth-power factors.
pub fn extract_root_factor(n: &Integer, k: u32) -> (Integer, Integer) {
    assert!(k > 0, "root degree must be positive");
    assert!(n >= &Integer::ZERO || k % 2 == 1, "negative even radicand");
    if n.is_zero() {
        return (Integer::ZERO, Integer::ONE);
    }
    if k == 1 {
        return (n.clone(), Integer::ONE);
    }
    let mut remainder = abs(n);
    let mut outside = if n < &Integer::ZERO {
        (-1).into()
    } else {
        Integer::ONE
    };
    let mut inside = Integer::ONE;
    for &p in small_primes() {
        if Integer::from(p) * p > remainder {
            break;
        }
        let mut count = 0_u32;
        while &remainder % p == 0 {
            remainder /= p;
            count += 1;
        }
        if count > 0 {
            outside *= Integer::from(p).pow((count / k) as usize);
            inside *= Integer::from(p).pow((count % k) as usize);
        }
    }
    if let Some(root) = exact_root(&remainder, k) {
        outside *= root;
    } else {
        inside *= remainder;
    }
    (outside, inside)
}

fn abs(n: &Integer) -> Integer {
    Integer::from(n.clone().into_parts().1)
}

fn small_primes() -> &'static [u32] {
    static PRIMES: OnceLock<Vec<u32>> = OnceLock::new();
    PRIMES.get_or_init(|| {
        let mut composite = vec![false; 65536];
        let mut primes = Vec::new();
        for p in 2_usize..65536 {
            if !composite[p] {
                primes.push(p as u32);
                for j in (p * p..65536).step_by(p) {
                    composite[j] = true;
                }
            }
        }
        primes
    })
}

fn charge(budget: &mut u64) -> bool {
    if *budget == 0 {
        false
    } else {
        *budget -= 1;
        true
    }
}

fn finish_factors(found: BTreeMap<Integer, u32>, unresolved: Integer) -> Vec<(Integer, u32)> {
    let mut factors: Vec<_> = found.into_iter().collect();
    if !unresolved.is_one() {
        factors.push((unresolved, 1));
    }
    factors
}

fn random_below(limit: &Integer, rng: &mut SplitMix64) -> Integer {
    let bits = limit.bit_len();
    let mut value = Integer::ZERO;
    // One draw per limb; reduction makes even adversarial moduli terminate immediately.
    for start in (0..bits).step_by(64) {
        value += Integer::from(rng.next_u64()) << start;
    }
    value % limit
}

fn rho_step(x: Integer, c: &Integer, n: &Integer, budget: &mut u64) -> Option<Integer> {
    charge(budget).then(|| (&x * &x + c) % n)
}

// Brent (1980), https://maths-people.anu.edu.au/~brent/pub/pub051.html.
fn pollard_brent(n: &Integer, budget: &mut u64, rng: &mut SplitMix64) -> Option<Integer> {
    while *budget > 0 {
        let c = random_below(&(n - 1), rng) + 1;
        let mut y = random_below(&(n - 1), rng) + 1_u8;
        let mut r = 1_u64;
        loop {
            let x = y.clone();
            for _ in 0..r {
                y = rho_step(y, &c, n, budget)?;
            }
            let mut k = 0;
            let mut g = Integer::ONE;
            let mut saved = y.clone();
            while k < r && g.is_one() {
                saved = y.clone();
                let mut product = Integer::ONE;
                for _ in 0..64.min(r - k) {
                    y = rho_step(y, &c, n, budget)?;
                    product = (product * abs(&(&x - &y))) % n;
                }
                g = gcd(&product, n);
                k += 64.min(r - k);
            }
            if g == *n {
                loop {
                    saved = rho_step(saved, &c, n, budget)?;
                    g = gcd(&abs(&(&x - &saved)), n);
                    if !g.is_one() {
                        break;
                    }
                }
            }
            if !g.is_one() {
                if g < *n {
                    return Some(g);
                }
                break; // Degenerate cycle: choose another fixed-seed polynomial.
            }
            r = r.checked_mul(2)?;
        }
    }
    None
}

#[cfg(test)]
#[path = "ntheory_tests.rs"]
mod tests;
