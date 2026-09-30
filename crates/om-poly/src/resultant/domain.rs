//! Private exact integral domains; sparse polynomial rings are not Euclidean domains.
use crate::{EuclideanRing, MPoly, Ring, UPoly};
use om_num::{
    Integer,
    ctx::{Abort, Interrupt},
};

pub(super) trait ResultantDomain: Ring {
    fn exact_quotient(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort>;
    fn content_primitive(f: &UPoly<Self>, ctx: &Interrupt) -> Result<(Self, UPoly<Self>), Abort>;
}
impl ResultantDomain for Integer {
    fn exact_quotient(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        ctx.tick()?;
        Ok(EuclideanRing::exact_div(self, other))
    }
    fn content_primitive(f: &UPoly<Self>, ctx: &Interrupt) -> Result<(Self, UPoly<Self>), Abort> {
        f.content_pp(ctx)
    }
}
impl ResultantDomain for MPoly<Integer> {
    fn exact_quotient(&self, other: &Self, ctx: &Interrupt) -> Result<Option<Self>, Abort> {
        self.exact_div(other, ctx)
    }
    fn content_primitive(f: &UPoly<Self>, ctx: &Interrupt) -> Result<(Self, UPoly<Self>), Abort> {
        ctx.tick()?;
        let mut content = Self::zero();
        for c in &f.coeffs {
            ctx.tick()?;
            content = content.subresultant_gcd(c, ctx)?;
            if content.is_one() {
                break;
            }
        }
        let primitive = super::divide_coefficients(f, &content, ctx)?;
        Ok((content, primitive))
    }
}
