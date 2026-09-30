//! Ordered sparse multivariate polynomials and integer coefficient-ring adapters.
use crate::Ring;
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};
use smallvec::SmallVec;
use std::cmp::Ordering;

/// A monomial with cached total degree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Monomial {
    /// Variable exponents in fixed variable order.
    pub exps: SmallVec<[u32; 4]>,
    /// Sum of the exponents, fitting u32.
    pub deg: u32,
}
/// Supported admissible monomial orders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonoOrder {
    /// Lexicographic order, with the first variable most significant.
    Lex,
    /// Graded reverse lexicographic order.
    GrevLex,
}
impl Monomial {
    /// Build a monomial; None if the total degree exceeds u32.
    pub fn new(exps: impl IntoIterator<Item = u32>) -> Option<Self> {
        let exps: SmallVec<[u32; 4]> = exps.into_iter().collect();
        let deg = exps.iter().try_fold(0_u32, |d, e| d.checked_add(*e))?;
        Some(Self { exps, deg })
    }
    /// Compare monomials in the chosen order; variable widths must agree.
    pub fn cmp(&self, o: &Self, order: MonoOrder) -> Ordering {
        assert_eq!(
            self.exps.len(),
            o.exps.len(),
            "monomial variable widths must match"
        );
        match order {
            MonoOrder::Lex => self.exps.cmp(&o.exps),
            MonoOrder::GrevLex => self.deg.cmp(&o.deg).then_with(|| {
                self.exps
                    .iter()
                    .rev()
                    .zip(o.exps.iter().rev())
                    .find_map(|(a, b)| (a != b).then(|| b.cmp(a)))
                    .unwrap_or(Ordering::Equal)
            }),
        }
    }
    fn product(&self, o: &Self) -> Option<Self> {
        let deg = self.deg.checked_add(o.deg)?;
        let exps = self
            .exps
            .iter()
            .zip(&o.exps)
            .map(|(a, b)| a.checked_add(*b))
            .collect::<Option<SmallVec<[u32; 4]>>>()?;
        Some(Self { exps, deg })
    }
}
/// Sparse terms in descending monomial order, without duplicates or zero coefficients.
#[derive(Clone, Debug)]
pub struct MPoly<R: Ring> {
    /// Number of variables in the fixed ring context.
    pub nvars: usize,
    /// Descending monomial/coefficient pairs.
    pub terms: Vec<(Monomial, R)>,
    /// Monomial order.
    pub order: MonoOrder,
}
impl<R: Ring> PartialEq for MPoly<R> {
    fn eq(&self, o: &Self) -> bool {
        if self.is_zero() || o.is_zero() {
            return self.is_zero() && o.is_zero();
        }
        if self.terms.len() == 1
            && o.terms.len() == 1
            && self.terms[0].0.deg == 0
            && o.terms[0].0.deg == 0
        {
            return self.terms[0].1 == o.terms[0].1;
        }
        self.nvars == o.nvars && self.order == o.order && self.terms == o.terms
    }
}
impl<R: Ring> MPoly<R> {
    /// Normalize valid monomials and compatible coefficient contexts.
    pub fn new(
        nvars: usize,
        terms: Vec<(Monomial, R)>,
        order: MonoOrder,
        ctx: &Interrupt,
    ) -> Result<Self, Abort> {
        normalize(nvars, terms, order, Some(ctx))
    }
    /// Context-free zero, usable as a generic coefficient factory.
    pub fn zero() -> Self {
        Self {
            nvars: 0,
            terms: vec![],
            order: MonoOrder::Lex,
        }
    }
    /// Context-free one, usable as a generic coefficient factory.
    pub fn one() -> Self {
        Self {
            nvars: 0,
            terms: vec![(
                Monomial {
                    exps: SmallVec::new(),
                    deg: 0,
                },
                R::one(),
            )],
            order: MonoOrder::Lex,
        }
    }
    /// Zero with a specific variable/order context.
    pub fn zero_in(nvars: usize, order: MonoOrder) -> Self {
        Self {
            nvars,
            terms: vec![],
            order,
        }
    }
    /// Whether the canonical sparse representation is zero.
    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }
    /// Whether the polynomial is a constant unit.
    pub fn is_one(&self) -> bool {
        self.terms.len() == 1
            && self.terms[0].0.deg == 0
            && self.terms[0].1.sub(&R::one()).is_zero()
    }
    /// Add compatible sparse polynomials.
    pub fn add(&self, o: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        self.combine(o, false, Some(ctx))
    }
    /// Subtract compatible sparse polynomials.
    pub fn sub(&self, o: &Self, ctx: &Interrupt) -> Result<Self, Abort> {
        self.combine(o, true, Some(ctx))
    }
    /// Negate coefficients, preserving order and variable context.
    pub fn neg(&self, ctx: &Interrupt) -> Result<Self, Abort> {
        self.neg_raw(Some(ctx))
    }
    /// Multiply; None if a product monomial cannot fit its u32 degree representation.
    pub fn mul(&self, o: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        self.mul_raw(o, Some(ctx))
    }
    fn context(&self, o: &Self) -> (usize, MonoOrder) {
        if self.nvars == 0 {
            return (o.nvars, o.order);
        }
        if o.nvars == 0 {
            return (self.nvars, self.order);
        }
        assert_eq!(
            self.nvars, o.nvars,
            "sparse polynomial variable contexts must match"
        );
        assert_eq!(
            self.order, o.order,
            "sparse polynomial monomial orders must match"
        );
        (self.nvars, self.order)
    }
    fn lifted(&self, nvars: usize, ctx: Option<&Interrupt>) -> Result<Vec<(Monomial, R)>, Abort> {
        let mut out = Vec::with_capacity(self.terms.len());
        for (m, c) in &self.terms {
            tick(ctx)?;
            let mut m = m.clone();
            if self.nvars == 0 {
                m.exps.resize(nvars, 0);
            }
            out.push((m, c.clone()));
        }
        Ok(out)
    }
    fn combine(&self, o: &Self, subtract: bool, ctx: Option<&Interrupt>) -> Result<Self, Abort> {
        tick(ctx)?;
        let (nvars, order) = self.context(o);
        let mut terms = self.lifted(nvars, ctx)?;
        for (m, c) in o.lifted(nvars, ctx)? {
            tick(ctx)?;
            terms.push((m, if subtract { c.neg() } else { c }));
        }
        normalize(nvars, terms, order, ctx)
    }
    fn neg_raw(&self, ctx: Option<&Interrupt>) -> Result<Self, Abort> {
        tick(ctx)?;
        let mut terms = Vec::with_capacity(self.terms.len());
        for (m, c) in &self.terms {
            tick(ctx)?;
            terms.push((m.clone(), c.neg()));
        }
        Ok(Self {
            nvars: self.nvars,
            terms,
            order: self.order,
        })
    }
    fn mul_raw(&self, o: &Self, ctx: Option<&Interrupt>) -> Result<Option<Self>, Abort> {
        tick(ctx)?;
        let (nvars, order) = self.context(o);
        let (a, b) = (self.lifted(nvars, ctx)?, o.lifted(nvars, ctx)?);
        let mut terms = vec![];
        for (ma, ca) in &a {
            for (mb, cb) in &b {
                tick(ctx)?;
                let Some(m) = ma.product(mb) else {
                    return Ok(None);
                };
                terms.push((m, ca.mul(cb)));
            }
        }
        Ok(Some(normalize(nvars, terms, order, ctx)?))
    }
}

fn tick(ctx: Option<&Interrupt>) -> Result<(), Abort> {
    ctx.map_or(Ok(()), Interrupt::tick)
}
fn normalize<R: Ring>(
    nvars: usize,
    terms: Vec<(Monomial, R)>,
    order: MonoOrder,
    ctx: Option<&Interrupt>,
) -> Result<MPoly<R>, Abort> {
    tick(ctx)?;
    for (m, _) in &terms {
        tick(ctx)?;
        assert_eq!(
            m.exps.len(),
            nvars,
            "monomial width must match the variable count"
        );
        let mut degree = 0_u32;
        for e in &m.exps {
            tick(ctx)?;
            degree = degree
                .checked_add(*e)
                .expect("invariant: valid monomial degree fits u32");
        }
        assert_eq!(
            m.deg, degree,
            "monomial degree cache must equal the exponent sum"
        );
    }
    let terms = sort_terms(terms, order, ctx)?;
    let mut out: Vec<(Monomial, R)> = Vec::with_capacity(terms.len());
    for (m, c) in terms {
        tick(ctx)?;
        if c.is_zero() {
            continue;
        }
        if let Some((last, sum)) = out.last_mut()
            && *last == m
        {
            *sum = sum.add(&c);
            if sum.is_zero() {
                out.pop();
            }
        } else {
            out.push((m, c));
        }
    }
    Ok(MPoly {
        nvars,
        terms: out,
        order,
    })
}
fn sort_terms<R: Ring>(
    mut source: Vec<(Monomial, R)>,
    order: MonoOrder,
    ctx: Option<&Interrupt>,
) -> Result<Vec<(Monomial, R)>, Abort> {
    // Bottom-up merge sort permits cancellation during comparisons, unlike sort_by.
    let mut width = 1_usize;
    let len = source.len();
    while width < len {
        let mut out = Vec::with_capacity(len);
        for start in (0..len).step_by(2 * width) {
            let mid = start.saturating_add(width).min(len);
            let end = mid.saturating_add(width).min(len);
            let (mut i, mut j) = (start, mid);
            while i < mid || j < end {
                tick(ctx)?;
                if i < mid && (j == end || source[i].0.cmp(&source[j].0, order) != Ordering::Less) {
                    out.push(source[i].clone());
                    i += 1;
                } else {
                    out.push(source[j].clone());
                    j += 1;
                }
            }
        }
        source = out;
        width *= 2;
    }
    Ok(source)
}

impl Ring for MPoly<Integer> {
    fn zero() -> Self {
        MPoly::zero()
    }
    fn one() -> Self {
        MPoly::one()
    }
    fn is_zero(&self) -> bool {
        MPoly::is_zero(self)
    }
    fn add(&self, o: &Self) -> Self {
        self.combine(o, false, None)
            .expect("invariant: coefficient arithmetic has no cancellation callback")
    }
    fn sub(&self, o: &Self) -> Self {
        self.combine(o, true, None)
            .expect("invariant: coefficient arithmetic has no cancellation callback")
    }
    fn mul(&self, o: &Self) -> Self {
        self.mul_raw(o, None)
            .expect("invariant: coefficient arithmetic has no cancellation callback")
            .expect("invariant: coefficient-ring product degree fits u32")
    }
    fn neg(&self) -> Self {
        self.neg_raw(None)
            .expect("invariant: coefficient arithmetic has no cancellation callback")
    }
}
