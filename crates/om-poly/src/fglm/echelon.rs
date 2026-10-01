//! Exact incremental dependence tests retain coordinates in the original NF vectors.
use om_num::{
    Rational,
    ctx::{Abort, Interrupt},
};
struct Row {
    pivot: usize,
    vector: Vec<Rational>,
    coordinates: Vec<Rational>,
}
pub(super) struct Echelon {
    rows: Vec<Row>,
}
impl Echelon {
    pub fn new() -> Self {
        Self { rows: vec![] }
    }
    /// Some(c) proves vector=sum(c_i*original_i); None adds an independent vector.
    pub fn reduce(
        &mut self,
        mut vector: Vec<Rational>,
        ctx: &Interrupt,
    ) -> Result<Option<Vec<Rational>>, Abort> {
        ctx.tick()?;
        let mut coordinates = vec![Rational::ZERO; self.rows.len()];
        for row in &self.rows {
            ctx.tick()?;
            let factor = vector[row.pivot].clone();
            if factor == Rational::ZERO {
                continue;
            }
            for (v, a) in vector.iter_mut().zip(&row.vector) {
                ctx.tick()?;
                *v -= &factor * a;
            }
            for (c, a) in coordinates.iter_mut().zip(&row.coordinates) {
                ctx.tick()?;
                *c += &factor * a;
            }
        }
        let Some(pivot) = vector.iter().position(|v| v != &Rational::ZERO) else {
            return Ok(Some(coordinates));
        };
        let value = vector[pivot].clone();
        for v in &mut vector {
            ctx.tick()?;
            *v /= &value;
        }
        for c in &mut coordinates {
            ctx.tick()?;
            *c = -&*c / &value;
        }
        coordinates.push(Rational::ONE / &value);
        for row in &mut self.rows {
            ctx.tick()?;
            row.coordinates.push(Rational::ZERO);
        }
        self.rows.push(Row {
            pivot,
            vector,
            coordinates,
        });
        Ok(None)
    }
}
