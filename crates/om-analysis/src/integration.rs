//! Self-written adaptive GK15/7. Error is an estimate, not an enclosure certificate.
use crate::Error;
use om_num::ctx::{Abort, Interrupt};
use std::{cmp::Ordering, collections::BinaryHeap};
/// Machine tolerances and bounded subdivision controls.
#[derive(Clone, Debug)]
pub struct Options {
    /// Absolute estimated error target.
    pub abs_tol: f64,
    /// Relative estimated error target.
    pub rel_tol: f64,
    /// Maximum live subintervals, including explicit pieces.
    pub max_intervals: usize,
    /// Finite strictly interior points where integration is split.
    pub breakpoints: Vec<f64>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            abs_tol: 1e-10,
            rel_tol: 1e-8,
            max_intervals: 10000,
            breakpoints: vec![],
        }
    }
}
/// Actual approximation and work statistics.
#[derive(Clone, Debug)]
pub struct Integral {
    /// Approximate integral with the requested orientation.
    pub value: f64,
    /// Absolute error estimate, not a strict bound.
    pub error_estimate: f64,
    /// Actual original callback evaluations.
    pub evaluations: usize,
    /// Number of active subintervals.
    pub intervals: usize,
}
/// Distinct unsuccessful integration states.
#[derive(Debug)]
pub enum FailureKind {
    /// Invalid bounds, controls or pieces.
    Input(&'static str),
    /// A callback produced a nonfinite sample or conversion could not represent it.
    NonFiniteSample,
    /// The caller's actual interrupt terminated work.
    Abort(Abort),
    /// A callback rejected evaluation for a reason other than interruption.
    Callback(Error),
    /// Subinterval limit reached before the error target.
    IntervalLimit,
    /// Subdivision/error stagnation at machine resolution.
    Roundoff,
}
/// Failure retains a real partial approximation when one exists; it is never a converged result.
#[derive(Debug)]
pub struct Failure {
    /// Actual reason.
    pub kind: FailureKind,
    /// Partial estimate and work statistics, if already computed.
    pub partial: Option<Integral>,
}
fn failure(kind: FailureKind) -> Failure {
    Failure {
        kind,
        partial: None,
    }
}
impl From<Abort> for Failure {
    fn from(e: Abort) -> Self {
        failure(FailureKind::Abort(e))
    }
}
#[derive(Clone, Copy)]
enum Mapping {
    Finite,
    Upper(f64),
    Lower(f64),
}
#[derive(Clone)]
struct Piece {
    a: f64,
    b: f64,
    map: usize,
    value: f64,
    error: f64,
    serial: usize,
}
impl PartialEq for Piece {
    fn eq(&self, b: &Self) -> bool {
        self.serial == b.serial
    }
}
impl Eq for Piece {}
impl PartialOrd for Piece {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Piece {
    fn cmp(&self, b: &Self) -> Ordering {
        self.error
            .total_cmp(&b.error)
            .then_with(|| b.serial.cmp(&self.serial))
    }
}
const X: [f64; 8] = [
    0.9914553711208126,
    0.9491079123427585,
    0.8648644233597691,
    0.7415311855993945,
    0.5860872354676911,
    0.4058451513773972,
    0.2077849550078985,
    0.,
];
const K: [f64; 8] = [
    0.022935322010529225,
    0.06309209262997855,
    0.10479001032225018,
    0.14065325971552592,
    0.1690047266392679,
    0.19035057806478542,
    0.20443294007529889,
    0.20948214108472783,
];
const G: [f64; 4] = [
    0.1294849661688697,
    0.27970539148927667,
    0.3818300505051189,
    0.4179591836734694,
];
fn sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let (mut s, mut c) = (0., 0.);
    for value in values {
        let next = s + value;
        c += if s.abs() >= value.abs() {
            (s - next) + value
        } else {
            (value - next) + s
        };
        s = next;
    }
    s + c
}
fn product(a: f64, b: f64, c: f64) -> f64 {
    let mut x = [a, b, c];
    x.sort_by(|a, b| a.abs().total_cmp(&b.abs()));
    (x[0] * x[2]) * x[1]
}
fn sample<F>(
    f: &mut F,
    map: Mapping,
    t: f64,
    count: &mut usize,
    ctx: &Interrupt,
) -> Result<f64, Failure>
where
    F: FnMut(f64, &Interrupt) -> Result<f64, Error>,
{
    ctx.tick()?;
    let (x, jac) = match map {
        Mapping::Finite => (t, 1.),
        Mapping::Upper(a) => (a + t / (1. - t), 1. / (1. - t).powi(2)),
        Mapping::Lower(b) => (b - t / (1. - t), 1. / (1. - t).powi(2)),
    };
    if !x.is_finite() || !jac.is_finite() {
        return Err(failure(FailureKind::NonFiniteSample));
    }
    *count += 1;
    let value = f(x, ctx).map_err(|e| match e {
        Error::Abort(e) => Failure::from(e),
        e => failure(FailureKind::Callback(e)),
    })? * jac;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(failure(FailureKind::NonFiniteSample))
    }
}
fn rule<F>(
    f: &mut F,
    a: f64,
    b: f64,
    map: usize,
    mappings: &[Mapping],
    serial: usize,
    state: (&mut usize, &Interrupt),
) -> Result<Piece, Failure>
where
    F: FnMut(f64, &Interrupt) -> Result<f64, Error>,
{
    let (count, ctx) = state;
    let center = a * 0.5 + b * 0.5;
    let radius = b * 0.5 - a * 0.5;
    if !center.is_finite() || !radius.is_finite() || radius <= 0. {
        return Err(failure(FailureKind::Roundoff));
    }
    let mut samples = [0.; 15];
    samples[14] = sample(f, mappings[map], center, count, ctx)?;
    for i in 0..7 {
        ctx.tick()?;
        let delta = radius * X[i];
        let lo = center - delta;
        let hi = center + delta;
        if lo <= a || hi >= b || lo == center || hi == center {
            return Err(failure(FailureKind::Roundoff));
        }
        samples[2 * i] = sample(f, mappings[map], lo, count, ctx)?;
        samples[2 * i + 1] = sample(f, mappings[map], hi, count, ctx)?;
    }
    let scale = samples.iter().map(|x| x.abs()).fold(0., f64::max);
    if scale == 0. {
        return Ok(Piece {
            a,
            b,
            map,
            value: 0.,
            error: 0.,
            serial,
        });
    }
    let mut ks = vec![K[7] * (samples[14] / scale)];
    let mut gs = vec![G[3] * (samples[14] / scale)];
    let mut absolute = vec![K[7] * (samples[14] / scale).abs()];
    for i in 0..7 {
        ctx.tick()?;
        ks.push(K[i] * (samples[2 * i] / scale));
        ks.push(K[i] * (samples[2 * i + 1] / scale));
        absolute.push(K[i] * (samples[2 * i] / scale).abs());
        absolute.push(K[i] * (samples[2 * i + 1] / scale).abs());
        if i % 2 == 1 {
            gs.push(G[i / 2] * (samples[2 * i] / scale));
            gs.push(G[i / 2] * (samples[2 * i + 1] / scale));
        }
    }
    let k = sum(ks);
    let g = sum(gs);
    let mean = k * 0.5;
    let mut asc = vec![K[7] * (samples[14] / scale - mean).abs()];
    for i in 0..7 {
        asc.push(K[i] * (samples[2 * i] / scale - mean).abs());
        asc.push(K[i] * (samples[2 * i + 1] / scale - mean).abs());
    }
    let value = product(k, scale, radius);
    let resabs = product(sum(absolute), scale, radius);
    let resasc = product(sum(asc), scale, radius);
    let mut error = product((k - g).abs(), scale, radius);
    if resasc > 0. && error > 0. {
        error = resasc * (200. * error / resasc).powf(1.5).min(1.);
    }
    error = error.max(50. * f64::EPSILON * resabs);
    if !value.is_finite() || !error.is_finite() {
        return Err(failure(FailureKind::NonFiniteSample));
    }
    Ok(Piece {
        a,
        b,
        map,
        value,
        error,
        serial,
    })
}
/// Integrate a real callback on finite or improper bounds. Two-sided tails are evaluated separately,
/// so cancellation of divergent halves cannot manufacture a principal-value success.
pub fn integrate<F>(
    mut f: F,
    mut a: f64,
    mut b: f64,
    options: &Options,
    ctx: &Interrupt,
) -> Result<Integral, Failure>
where
    F: FnMut(f64, &Interrupt) -> Result<f64, Error>,
{
    ctx.tick()?;
    if a.is_nan()
        || b.is_nan()
        || !options.abs_tol.is_finite()
        || !options.rel_tol.is_finite()
        || options.abs_tol < 0.
        || options.rel_tol < 0.
        || options.abs_tol == 0. && options.rel_tol < 50. * f64::EPSILON
        || !(1..=100000).contains(&options.max_intervals)
        || options.breakpoints.len() > 4096
    {
        return Err(failure(FailureKind::Input("无效积分边界、容差或资源选项")));
    }
    if a == b {
        if a.is_finite() {
            return Ok(Integral {
                value: 0.,
                error_estimate: 0.,
                evaluations: 0,
                intervals: 0,
            });
        }
        return Err(failure(FailureKind::Input("相同无限端点未定义")));
    }
    let orientation = if a > b {
        std::mem::swap(&mut a, &mut b);
        -1.
    } else {
        1.
    };
    let mut points = options.breakpoints.clone();
    if points.iter().any(|p| !p.is_finite() || *p <= a || *p >= b) {
        return Err(failure(FailureKind::Input(
            "分段点必须有限且严格位于积分区间内部",
        )));
    }
    points.sort_by(f64::total_cmp);
    points.dedup();
    let mut physical = vec![a];
    physical.extend(points);
    physical.push(b);
    if a == f64::NEG_INFINITY && b == f64::INFINITY && !physical.contains(&0.) {
        physical.push(0.);
        physical.sort_by(f64::total_cmp);
    }
    let mut mappings = vec![];
    let mut intervals = vec![];
    for bounds in physical.windows(2) {
        ctx.tick()?;
        let lo = bounds[0];
        let hi = bounds[1];
        let (map, left, right) = if lo.is_finite() && hi.is_finite() {
            (Mapping::Finite, lo, hi)
        } else if lo.is_finite() && hi == f64::INFINITY {
            (Mapping::Upper(lo), 0., 1.)
        } else if lo == f64::NEG_INFINITY && hi.is_finite() {
            (Mapping::Lower(hi), 0., 1.)
        } else {
            return Err(failure(FailureKind::Input("不支持的无限端点")));
        };
        let index = mappings.len();
        mappings.push(map);
        intervals.push((left, right, index));
    }
    if intervals.len() > options.max_intervals {
        return Err(failure(FailureKind::Input("分段数超过积分区间限额")));
    }
    let mut evaluations = 0;
    let mut heap = BinaryHeap::new();
    let mut serial = 0;
    for (left, right, map) in intervals {
        heap.push(rule(
            &mut f,
            left,
            right,
            map,
            &mappings,
            serial,
            (&mut evaluations, ctx),
        )?);
        serial += 1;
    }
    let mut stagnation = 0;
    loop {
        ctx.tick()?;
        let mut values = Vec::with_capacity(heap.len());
        let mut errors = Vec::with_capacity(heap.len());
        for piece in &heap {
            ctx.tick()?;
            values.push(piece.value);
            errors.push(piece.error);
        }
        let value = sum(values);
        let error = sum(errors);
        let current = Integral {
            value: value * orientation,
            error_estimate: error,
            evaluations,
            intervals: heap.len(),
        };
        if !value.is_finite() || !error.is_finite() {
            return Err(Failure {
                kind: FailureKind::NonFiniteSample,
                partial: Some(current),
            });
        }
        if error <= options.abs_tol.max(options.rel_tol * value.abs()) {
            return Ok(current);
        }
        if heap.len() >= options.max_intervals {
            return Err(Failure {
                kind: FailureKind::IntervalLimit,
                partial: Some(current),
            });
        }
        let old = heap
            .pop()
            .ok_or_else(|| failure(FailureKind::Input("积分区间为空")))?;
        let midpoint = old.a * 0.5 + old.b * 0.5;
        if midpoint <= old.a || midpoint >= old.b {
            return Err(Failure {
                kind: FailureKind::Roundoff,
                partial: Some(current),
            });
        }
        let mut refine = || -> Result<(Piece, Piece), Failure> {
            let left = rule(
                &mut f,
                old.a,
                midpoint,
                old.map,
                &mappings,
                serial,
                (&mut evaluations, ctx),
            )?;
            serial += 1;
            let right = rule(
                &mut f,
                midpoint,
                old.b,
                old.map,
                &mappings,
                serial,
                (&mut evaluations, ctx),
            )?;
            serial += 1;
            Ok((left, right))
        };
        let (left, right) = refine().map_err(|mut e| {
            let mut partial = current.clone();
            partial.evaluations = evaluations;
            e.partial = Some(partial);
            e
        })?;
        if (left.value + right.value - old.value).abs() <= 1e-5 * (left.value + right.value).abs()
            && left.error + right.error >= 0.99 * old.error
        {
            stagnation += 1;
        } else {
            stagnation = 0;
        }
        if stagnation >= 64 {
            return Err(Failure {
                kind: FailureKind::Roundoff,
                partial: Some(current),
            });
        }
        heap.push(left);
        heap.push(right);
    }
}
