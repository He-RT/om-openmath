//! Shared edges and graph stitching preserve marching-squares topology.
use super::{PlotError, Point};
use crate::protocol::{Curve, PlotData, PlotRequest};
use om_core::Interrupt;
use om_eval::numeric::CompiledFn;
use std::collections::{BTreeMap, BTreeSet};
const N: usize = 160;
const WIDTH: usize = N + 1;
const HORIZONTAL: usize = N * WIDTH;
const VERTEX_OFFSET: usize = 2 * HORIZONTAL;
const CELL_OFFSET: usize = VERTEX_OFFSET + WIDTH * WIDTH;
struct Geometry<'a> {
    f: &'a CompiledFn,
    ctx: &'a Interrupt,
    x: (f64, f64),
    y: (f64, f64),
    values: Vec<f64>,
    work: Vec<f64>,
    points: BTreeMap<usize, Point>,
    edges: Vec<(usize, usize)>,
    pairs: BTreeSet<(usize, usize)>,
}
impl Geometry<'_> {
    fn coord(&self, vertex: usize) -> Point {
        let i = vertex % WIDTH;
        let j = vertex / WIDTH;
        (
            self.x.0 + (self.x.1 - self.x.0) * (i as f64 / N as f64),
            self.y.0 + (self.y.1 - self.y.0) * (j as f64 / N as f64),
        )
    }
    fn crossing(&mut self, edge: usize, a: usize, b: usize) -> Result<Option<usize>, PlotError> {
        let (va, vb) = (self.values[a], self.values[b]);
        if (va >= 0.0) == (vb >= 0.0) {
            return Ok(None);
        }
        let key = if va == 0.0 {
            VERTEX_OFFSET + a
        } else if vb == 0.0 {
            VERTEX_OFFSET + b
        } else {
            edge
        };
        if !self.points.contains_key(&key) {
            let (pa, pb) = (self.coord(a), self.coord(b));
            let scale = va.abs().max(vb.abs());
            let t = if va == 0.0 {
                0.0
            } else if vb == 0.0 {
                1.0
            } else {
                (va / scale) / ((va / scale) - (vb / scale))
            };
            let p = (pa.0 + (pb.0 - pa.0) * t, pa.1 + (pb.1 - pa.1) * t);
            // Sign changes across poles or jumps are not zeros. Keep linear geometry only
            // after bounded residual probes establish a real edge zero.
            if va != 0.0 && vb != 0.0 {
                let tolerance = 1e-6 * scale;
                let mut probe = self
                    .f
                    .eval_with_ctx(&[p.0, p.1], &mut self.work, self.ctx)?;
                let (mut lo, mut hi, mut vlo) = (0.0, 1.0, va);
                for _ in 0..24 {
                    if probe.is_finite() && probe.abs() <= tolerance {
                        break;
                    }
                    let mid = (lo + hi) * 0.5;
                    let q = (pa.0 + (pb.0 - pa.0) * mid, pa.1 + (pb.1 - pa.1) * mid);
                    probe = self
                        .f
                        .eval_with_ctx(&[q.0, q.1], &mut self.work, self.ctx)?;
                    if !probe.is_finite() {
                        return Ok(None);
                    }
                    if (probe >= 0.0) == (vlo >= 0.0) {
                        lo = mid;
                        vlo = probe;
                    } else {
                        hi = mid;
                    }
                }
                if !probe.is_finite() || probe.abs() > tolerance {
                    return Ok(None);
                }
            }
            self.points.insert(key, p);
        }
        Ok(Some(key))
    }
    fn connect(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        let pair = (a.min(b), a.max(b));
        if self.pairs.insert(pair) {
            self.edges.push(pair);
        }
    }
    fn cell(&mut self, i: usize, j: usize) -> Result<(), PlotError> {
        self.ctx.tick()?;
        let v = [
            j * WIDTH + i,
            j * WIDTH + i + 1,
            (j + 1) * WIDTH + i + 1,
            (j + 1) * WIDTH + i,
        ];
        let z = v.map(|id| self.values[id]);
        if z.iter().any(|z| !z.is_finite()) {
            return Ok(());
        }
        let edge_ids = [
            j * N + i,
            HORIZONTAL + j * WIDTH + i + 1,
            (j + 1) * N + i,
            HORIZONTAL + j * WIDTH + i,
        ];
        let mut hits = vec![];
        let mut keys = [None; 4];
        for e in 0..4 {
            keys[e] = self.crossing(edge_ids[e], v[e], v[(e + 1) % 4])?;
            if let Some(key) = keys[e] {
                hits.push(key);
            }
        }
        if hits.len() == 2 {
            self.connect(hits[0], hits[1]);
        } else if hits.len() == 4 {
            let scale = z.iter().map(|z| z.abs()).fold(0.0, f64::max);
            let q = z.map(|z| z / scale);
            let determinant = q[0] * q[2] - q[1] * q[3];
            if determinant == 0.0 {
                let d = q[0] - q[1] + q[2] - q[3];
                let sx = (q[0] - q[3]) / d;
                let sy = (q[0] - q[1]) / d;
                let origin = self.coord(v[0]);
                let other = self.coord(v[2]);
                let key = CELL_OFFSET + j * N + i;
                self.points.insert(
                    key,
                    (
                        origin.0 + (other.0 - origin.0) * sx,
                        origin.1 + (other.1 - origin.1) * sy,
                    ),
                );
                for hit in hits {
                    self.connect(hit, key);
                }
            } else if determinant > 0.0 {
                self.connect(hits[0], hits[1]);
                self.connect(hits[2], hits[3]);
            } else {
                self.connect(hits[0], hits[3]);
                self.connect(hits[1], hits[2]);
            }
        }
        Ok(())
    }
    fn stitch(&self) -> Result<Vec<Vec<Point>>, PlotError> {
        let mut adjacent: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
        for (id, &(a, b)) in self.edges.iter().enumerate() {
            adjacent.entry(a).or_default().push((b, id));
            adjacent.entry(b).or_default().push((a, id));
        }
        let mut used = vec![false; self.edges.len()];
        let mut segments = vec![];
        let mut starts: Vec<_> = adjacent
            .iter()
            .filter(|(_, v)| v.len() != 2)
            .flat_map(|(&a, neighbors)| neighbors.iter().map(move |&(_, e)| (a, e)))
            .collect();
        starts.extend(self.edges.iter().enumerate().map(|(id, &(a, _))| (a, id)));
        for (start, first) in starts {
            self.ctx.tick()?;
            if used[first] {
                continue;
            }
            let mut path = vec![self.points[&start]];
            let (mut current, mut edge) = (start, first);
            loop {
                self.ctx.tick()?;
                used[edge] = true;
                let (a, b) = self.edges[edge];
                let next = if a == current { b } else { a };
                path.push(self.points[&next]);
                if next == start || adjacent[&next].len() != 2 {
                    break;
                }
                let Some(&(_, next_edge)) = adjacent[&next].iter().find(|&&(_, e)| !used[e]) else {
                    break;
                };
                current = next;
                edge = next_edge;
            }
            if path.len() >= 2 {
                segments.push(path);
            }
        }
        Ok(segments)
    }
}
pub(super) fn sample(
    r: &PlotRequest,
    fs: &[CompiledFn],
    ctx: &Interrupt,
) -> Result<PlotData, PlotError> {
    let y = r
        .y_range
        .ok_or_else(|| PlotError::Invalid("implicit y range is missing".into()))?;
    let mut curves = vec![];
    let mut skipped = 0;
    for (f, label) in fs.iter().zip(&r.exprs) {
        let mut g = Geometry {
            f,
            ctx,
            x: r.x_range,
            y,
            values: vec![],
            work: vec![],
            points: BTreeMap::new(),
            edges: vec![],
            pairs: BTreeSet::new(),
        };
        for vertex in 0..WIDTH * WIDTH {
            let (x, y) = g.coord(vertex);
            g.values.push(g.f.eval_with_ctx(&[x, y], &mut g.work, ctx)?);
        }
        for j in 0..N {
            for i in 0..N {
                g.cell(i, j)?;
            }
        }
        skipped += g.values.iter().filter(|v| !v.is_finite()).count() as u32;
        curves.push(Curve {
            label: label.clone(),
            segments: g.stitch()?,
        });
    }
    Ok(PlotData {
        geometry: r.options.as_ref().map(|_| crate::protocol::PlotGeometry2D {
            skipped,
            ..Default::default()
        }),
        scale: None,
        curves,
        x_range: r.x_range,
        y_range: y,
        highlights: None,
    })
}
