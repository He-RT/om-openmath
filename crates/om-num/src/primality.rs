//! Miller-Rabin below 2^64, strong base-2 plus strong Lucas-Selfridge above it.
//! References: Baillie/Fiori/Wagstaff, https://arxiv.org/abs/2006.14425, §2.4.

use super::{Integer, gcd, isqrt};
use dashu::base::BitTest;

const BASES: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

pub(super) fn is_probable_prime(n: &Integer) -> bool {
    if n < &2.into() {
        return false;
    }
    for p in BASES {
        if n == &p.into() {
            return true;
        }
        if n % p == 0 {
            return false;
        }
    }
    if let Ok(n) = u64::try_from(n) {
        return BASES.into_iter().all(|a| mr_u64(n, a));
    }
    let square_root = isqrt(n);
    if &square_root * &square_root == *n || !mr_big(n, 2) {
        return false;
    }
    strong_lucas(n)
}

fn pow_u64(mut base: u64, mut exponent: u64, n: u64) -> u64 {
    let mut value = 1;
    while exponent > 0 {
        if exponent & 1 == 1 {
            value = ((value as u128 * base as u128) % n as u128) as u64;
        }
        base = ((base as u128 * base as u128) % n as u128) as u64;
        exponent >>= 1;
    }
    value
}

fn mr_u64(n: u64, base: u64) -> bool {
    let s = (n - 1).trailing_zeros();
    let d = (n - 1) >> s;
    let mut x = pow_u64(base % n, d, n);
    if x == 1 || x == n - 1 {
        return true;
    }
    for _ in 1..s {
        x = ((x as u128 * x as u128) % n as u128) as u64;
        if x == n - 1 {
            return true;
        }
    }
    false
}

fn residue(x: Integer, n: &Integer) -> Integer {
    let r = x % n;
    if r < Integer::ZERO { r + n } else { r }
}

fn pow_mod(mut base: Integer, e: &Integer, n: &Integer) -> Integer {
    base = residue(base, n);
    let mut value = Integer::ONE;
    for bit in 0..e.bit_len() {
        if e.bit(bit) {
            value = (value * &base) % n;
        }
        base = (&base * &base) % n;
    }
    value
}

fn mr_big(n: &Integer, base: u64) -> bool {
    let predecessor = n - 1_u8;
    let s = predecessor
        .trailing_zeros()
        .expect("invariant: predecessor is nonzero");
    let d = &predecessor >> s;
    let mut x = pow_mod(base.into(), &d, n);
    if x.is_one() || x == predecessor {
        return true;
    }
    for _ in 1..s {
        x = (&x * &x) % n;
        if x == predecessor {
            return true;
        }
    }
    false
}

fn jacobi(a: &Integer, n: &Integer) -> i8 {
    let (mut a, mut n) = (residue(a.clone(), n), n.clone());
    let mut sign = 1;
    while !a.is_zero() {
        let zeros = a.trailing_zeros().expect("invariant: nonzero numerator");
        if zeros % 2 == 1 && (n.clone() % 8 == 3 || n.clone() % 8 == 5) {
            sign = -sign;
        }
        a >>= zeros;
        if &a % 4 == 3 && &n % 4 == 3 {
            sign = -sign;
        }
        (a, n) = (&n % &a, a);
    }
    if n.is_one() { sign } else { 0 }
}

fn half_mod(x: Integer, n: &Integer) -> Integer {
    let mut x = residue(x, n);
    if x.bit(0) {
        x += n;
    }
    x >> 1
}

fn strong_lucas(n: &Integer) -> bool {
    let mut discriminant = Integer::from(5);
    loop {
        match jacobi(&discriminant, n) {
            -1 => break,
            0 if gcd(&discriminant, n) < *n => return false,
            _ => {}
        }
        discriminant = if discriminant > Integer::ZERO {
            -discriminant - 2
        } else {
            -discriminant + 2
        };
    }
    let q: Integer = (Integer::ONE - &discriminant) / 4;
    let successor = n + 1_u8;
    let s = successor
        .trailing_zeros()
        .expect("invariant: successor is nonzero");
    let index = &successor >> s;
    let (mut u, mut v, mut qk) = (Integer::ONE, Integer::ONE, residue(q.clone(), n));
    for bit in (0..index.bit_len() - 1).rev() {
        u = (&u * &v) % n;
        v = residue(&v * &v - &qk * 2, n);
        qk = (&qk * &qk) % n;
        if index.bit(bit) {
            (u, v) = (half_mod(&u + &v, n), half_mod(&discriminant * &u + &v, n));
            qk = residue(qk * &q, n);
        }
    }
    if u.is_zero() || v.is_zero() {
        return true;
    }
    for _ in 1..s {
        v = residue(&v * &v - &qk * 2, n);
        qk = (&qk * &qk) % n;
        if v.is_zero() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_lucas_accepts_independently_sieved_primes() {
        let mut composite = vec![false; 20000];
        for p in 2..20000 {
            if !composite[p] {
                for n in (p * 2..20000).step_by(p) {
                    composite[n] = true;
                }
                if p > 2 {
                    assert!(strong_lucas(&p.into()), "prime {p}");
                }
            }
        }
    }

    #[test]
    fn miller_rabin_and_lucas_reject_each_others_pseudoprimes() {
        assert!(mr_big(&2047.into(), 2));
        assert!(!strong_lucas(&2047.into()));
        for n in [5777, 10877, 16109, 18971, 22499] {
            assert!(strong_lucas(&n.into()), "Lucas pseudoprime {n}");
            assert!(!mr_big(&n.into(), 2), "Lucas pseudoprime {n}");
        }
    }
}
