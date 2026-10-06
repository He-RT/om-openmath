//! Piecewise normalized polynomials. Shapes, domains, monotonicity and endpoint continuity are checked.
use crate::Error;
use om_num::ctx::Interrupt;
/// Actual source of interpolation coefficients.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// Linear segments through the data.
    Linear,
    /// Cubic Hermite with supplied or finite-difference derivatives.
    Hermite,
    /// Actual DP5(4) quartic continuous extension.
    DormandPrince,
}
/// Finite scalar/vector interpolation, accepting strictly increasing or decreasing time.
#[derive(Clone, Debug)]
pub struct Interpolation {
    /// Actual sample times, in integration orientation.
    pub(crate) times: Vec<f64>,
    /// Sample states; all rows share 1..64 dimensions.
    pub(crate) values: Vec<Vec<f64>>,
    /// For each segment/component, coefficients of normalized u,u²,u³,u⁴.
    pub(crate) coefficients: Vec<Vec<[f64; 4]>>,
    /// Actual method, distinct from precision or exactness claims.
    pub(crate) method: Method,
}
impl Interpolation {
    /// Borrow saved coordinates without permitting invalid mutations.
    pub fn times(&self) -> &[f64] {
        &self.times
    }
    /// Borrow the saved node values.
    pub fn values(&self) -> &[Vec<f64>] {
        &self.values
    }
    /// Borrow normalized segment coefficients.
    pub fn coefficients(&self) -> &[Vec<[f64; 4]>] {
        &self.coefficients
    }
    /// Actual interpolation method.
    pub fn method(&self) -> Method {
        self.method
    }
    /// Construct and validate a complete finite interpolation object. A sole point has zero extent.
    pub fn new(
        times: Vec<f64>,
        values: Vec<Vec<f64>>,
        coefficients: Vec<Vec<[f64; 4]>>,
        method: Method,
        ctx: &Interrupt,
    ) -> Result<Self, Error> {
        let out = Self {
            times,
            values,
            coefficients,
            method,
        };
        out.validate(ctx)?;
        Ok(out)
    }
    /// Check actual shape, scalar resources, finite inputs and continuity at saved knots.
    pub fn validate(&self, ctx: &Interrupt) -> Result<(), Error> {
        ctx.tick()?;
        let n = self.times.len();
        let d = self.values.first().map_or(0, Vec::len);
        if n == 0
            || !(1..=64).contains(&d)
            || self.values.len() != n
            || self.coefficients.len() != n - 1
            || n.saturating_mul(d.saturating_mul(5) + 1) > 100000
        {
            return Err(Error::Input("插值维度/数据形状或100000标量资源界限无效"));
        }
        let direction = if n > 1 && self.times[1] < self.times[0] {
            -1.
        } else {
            1.
        };
        for (i, t) in self.times.iter().enumerate() {
            ctx.tick()?;
            if !t.is_finite()
                || self.values[i].len() != d
                || self.values[i].iter().any(|x| !x.is_finite())
            {
                return Err(Error::NonFinite);
            }
            if i > 0 {
                let delta = t - self.times[i - 1];
                if !delta.is_finite() || delta * direction <= 0. {
                    return Err(Error::Input("插值坐标需要有限跨度且严格单调"));
                }
            }
        }
        for (i, segment) in self.coefficients.iter().enumerate() {
            if segment.len() != d {
                return Err(Error::Input("插值段维度不一致"));
            }
            for (j, c) in segment.iter().enumerate() {
                ctx.tick()?;
                if (self.method == Method::Linear && c[1..].iter().any(|v| *v != 0.))
                    || (self.method == Method::Hermite && c[3] != 0.)
                {
                    return Err(Error::Input("插值系数次数与方法不符"));
                }
                if c.iter().any(|x| !x.is_finite()) {
                    return Err(Error::NonFinite);
                }
                let end = self.values[i][j] + c.iter().sum::<f64>();
                let scale = c
                    .iter()
                    .map(|c| c.abs())
                    .sum::<f64>()
                    .max(self.values[i][j].abs())
                    .max(self.values[i + 1][j].abs())
                    .max(1e-300);
                if !end.is_finite()
                    || !scale.is_finite()
                    || (end - self.values[i + 1][j]).abs() > 128. * f64::EPSILON * scale
                {
                    return Err(Error::Input("插值系数与节点端点不连续"));
                }
            }
        }
        Ok(())
    }
    /// Create ordinary linear or cubic Hermite interpolation; missing Hermite slopes are estimated.
    pub fn from_points(
        times: Vec<f64>,
        values: Vec<Vec<f64>>,
        slopes: Option<Vec<Vec<f64>>>,
        method: Method,
        ctx: &Interrupt,
    ) -> Result<Self, Error> {
        if method == Method::Linear && slopes.is_some() {
            return Err(Error::Input("线性插值不接受Hermite导数"));
        }
        ctx.tick()?;
        if times.len() < 2 || values.len() != times.len() || method == Method::DormandPrince {
            return Err(Error::Input(
                "普通插值需要至少两个同形节点和linear/hermite方法",
            ));
        }
        let d = values.first().map_or(0, Vec::len);
        let n = times.len();
        if !(1..=64).contains(&d)
            || n.saturating_mul(5 * d + 1) > 100000
            || values
                .iter()
                .any(|y| y.len() != d || y.iter().any(|v| !v.is_finite()))
            || times.iter().any(|x| !x.is_finite())
        {
            return Err(Error::Input("插值点的维度、有限值或资源界限无效"));
        }
        let direction = if times[1] > times[0] { 1. } else { -1. };
        let mut secants = vec![];
        for i in 0..n - 1 {
            ctx.tick()?;
            let h = times[i + 1] - times[i];
            if !h.is_finite() || h * direction <= 0. {
                return Err(Error::Input("插值节点必须严格单调"));
            }
            let mut row = vec![];
            for (a, b) in values[i].iter().zip(&values[i + 1]) {
                ctx.tick()?;
                let v = if method == Method::Linear || slopes.is_some() {
                    0.
                } else {
                    (b - a) / h
                };
                if !v.is_finite() {
                    return Err(Error::NonFinite);
                }
                row.push(v);
            }
            secants.push(row);
        }
        let slopes = if method == Method::Linear {
            vec![vec![0.; d]; n]
        } else if let Some(slopes) = slopes {
            if slopes.len() != n
                || slopes
                    .iter()
                    .any(|s| s.len() != d || s.iter().any(|x| !x.is_finite()))
            {
                return Err(Error::Input("Hermite导数与节点维度不符"));
            }
            slopes
        } else {
            let mut slopes = vec![vec![0.; d]; n];
            slopes[0] = secants[0].clone();
            slopes[n - 1] = secants[n - 2].clone();
            for i in 1..n - 1 {
                for j in 0..d {
                    ctx.tick()?;
                    slopes[i][j] =
                        (values[i + 1][j] - values[i - 1][j]) / (times[i + 1] - times[i - 1]);
                    if !slopes[i][j].is_finite() {
                        return Err(Error::NonFinite);
                    }
                }
            }
            slopes
        };
        let mut coefficients = vec![];
        for i in 0..n - 1 {
            let h = times[i + 1] - times[i];
            let mut segment = vec![];
            for j in 0..d {
                ctx.tick()?;
                let delta = values[i + 1][j] - values[i][j];
                let c = if method == Method::Linear {
                    [delta, 0., 0., 0.]
                } else {
                    let (a, b) = (h * slopes[i][j], h * slopes[i + 1][j]);
                    [a, 3. * delta - 2. * a - b, -2. * delta + a + b, 0.]
                };
                segment.push(c);
            }
            coefficients.push(segment);
        }
        Self::new(times, values, coefficients, method, ctx)
    }
    /// Evaluate inside the saved domain. No extrapolation or missing-result zero filling occurs.
    pub fn evaluate(&self, t: f64, ctx: &Interrupt) -> Result<Vec<f64>, Error> {
        ctx.tick()?;
        if !t.is_finite() {
            return Err(Error::NonFinite);
        }
        let n = self.times.len();
        if n == 0 {
            return Err(Error::Input("插值对象为空"));
        }
        let dir = if n > 1 && self.times[n - 1] < self.times[0] {
            -1.
        } else {
            1.
        };
        if (t - self.times[0]) * dir < 0. || (t - self.times[n - 1]) * dir > 0. {
            return Err(Error::Input("插值默认拒绝域外外推"));
        }
        if t == self.times[n - 1] {
            return Ok(self.values[n - 1].clone());
        }
        let (mut lo, mut hi) = (0, n - 1);
        while hi - lo > 1 {
            ctx.tick()?;
            let mid = lo + (hi - lo) / 2;
            if (t - self.times[mid]) * dir >= 0. {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let h = self.times[lo + 1] - self.times[lo];
        let u = (t - self.times[lo]) / h;
        Self::segment(&self.values[lo], &self.coefficients[lo], u, ctx)
    }
    pub(crate) fn segment(
        y: &[f64],
        c: &[[f64; 4]],
        u: f64,
        ctx: &Interrupt,
    ) -> Result<Vec<f64>, Error> {
        let mut out = vec![];
        for (v, c) in y.iter().zip(c) {
            ctx.tick()?;
            let value = v + u * (c[0] + u * (c[1] + u * (c[2] + u * c[3])));
            if !value.is_finite() {
                return Err(Error::NonFinite);
            }
            out.push(value);
        }
        Ok(out)
    }
}
