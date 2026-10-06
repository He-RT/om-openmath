//! Bounded CPU drawing with a pinned OFL font; no system font, GPU, IO or mathematical reevaluation.
use super::{
    ArtifactError,
    figure::{Anchor, Color, Drawing, Point, Shape},
    invalid,
};
use om_core::Interrupt;
fn font() -> Result<swash::FontRef<'static>, ArtifactError> {
    swash::FontRef::from_index(
        include_bytes!("../../assets/OpenMathPlotLabels-Regular.otf"),
        0,
    )
    .ok_or_else(|| invalid("导出字体资源无效"))
}
struct Image {
    width: i32,
    height: i32,
    pixels: Vec<u8>,
}
impl Image {
    fn pixel(&mut self, x: i32, y: i32, color: Color, alpha: f64) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        let at = (y as usize * self.width as usize + x as usize) * 4;
        for i in 0..3 {
            self.pixels[at + i] = (self.pixels[at + i] as f64 * (1. - alpha)
                + color.0[i] as f64 * alpha)
                .round() as u8;
        }
    }
    fn disk(&mut self, p: Point, r: f64, c: Color, ctx: &Interrupt) -> Result<(), ArtifactError> {
        let x0 = (p.0 - r - 1.).floor().max(0.) as i32;
        let x1 = (p.0 + r + 1.).ceil().min(self.width as f64) as i32;
        let y0 = (p.1 - r - 1.).floor().max(0.) as i32;
        let y1 = (p.1 + r + 1.).ceil().min(self.height as f64) as i32;
        for y in y0..y1 {
            ctx.tick()?;
            for x in x0..x1 {
                let d = (x as f64 + 0.5 - p.0).hypot(y as f64 + 0.5 - p.1);
                let a = (r + 0.5 - d).clamp(0., 1.);
                if a > 0. {
                    self.pixel(x, y, c, a);
                }
            }
        }
        Ok(())
    }
    fn line(
        &mut self,
        a: Point,
        b: Point,
        c: Color,
        width: f64,
        ctx: &Interrupt,
    ) -> Result<(), ArtifactError> {
        let steps = ((a.0 - b.0).abs().max((a.1 - b.1).abs()) * 2.).ceil() as usize;
        for i in 0..=steps.max(1) {
            if i % 64 == 0 {
                ctx.tick()?;
            }
            let t = i as f64 / steps.max(1) as f64;
            self.disk(
                (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t),
                width * 0.5,
                c,
                ctx,
            )?;
        }
        Ok(())
    }
}
pub(super) fn render(d: &Drawing, ctx: &Interrupt) -> Result<Vec<u8>, ArtifactError> {
    ctx.tick()?;
    let mut image = Image {
        width: d.width as i32,
        height: d.height as i32,
        pixels: vec![255; (d.width as usize * d.height as usize) * 4],
    };
    let mut scale_context = swash::scale::ScaleContext::new();
    for shape in &d.shapes {
        ctx.tick()?;
        match shape {
            Shape::Line(a, b, c, w) => image.line(*a, *b, *c, *w, ctx)?,
            Shape::Rect(a, b, c, alpha) => {
                let x0 = a.0.floor().max(0.) as i32;
                let x1 = b.0.ceil().min(image.width as f64) as i32;
                let y0 = a.1.floor().max(0.) as i32;
                let y1 = b.1.ceil().min(image.height as f64) as i32;
                for y in y0..y1 {
                    ctx.tick()?;
                    for x in x0..x1 {
                        let coverage = ((b.0.min(x as f64 + 1.) - a.0.max(x as f64)).max(0.))
                            * ((b.1.min(y as f64 + 1.) - a.1.max(y as f64)).max(0.));
                        image.pixel(x, y, *c, alpha * coverage);
                    }
                }
            }
            Shape::Circle(p, r, c) => image.disk(*p, *r, *c, ctx)?,
            Shape::Text(p, s, px, c, anchor) => {
                let font = font()?;
                ctx.tick()?;
                let mapping = font.charmap();
                let metrics = font.glyph_metrics(&[]).scale(*px);
                let advance: f64 = s
                    .chars()
                    .map(|ch| f64::from(metrics.advance_width(mapping.map(ch))))
                    .sum();
                let mut x = p.0
                    - match anchor {
                        Anchor::Left => 0.,
                        Anchor::Center => advance / 2.,
                        Anchor::Right => advance,
                    };
                let mut scaler = scale_context.builder(font).size(*px).hint(false).build();
                for ch in s.chars() {
                    ctx.tick()?;
                    let glyph = mapping.map(ch);
                    if glyph == 0 && !ch.is_whitespace() {
                        return Err(ArtifactError::Invalid(format!(
                            "PNG标签字体未覆盖U+{:04X}；可导出保留原文字的SVG",
                            ch as u32
                        )));
                    }
                    if let Some(bitmap) =
                        swash::scale::Render::new(&[swash::scale::Source::Outline])
                            .format(swash::zeno::Format::Alpha)
                            .render(&mut scaler, glyph)
                    {
                        let placement = bitmap.placement;
                        for y in 0..placement.height {
                            ctx.tick()?;
                            for i in 0..placement.width {
                                let a =
                                    bitmap.data[(y * placement.width + i) as usize] as f64 / 255.;
                                if a > 0. {
                                    image.pixel(
                                        x.round() as i32 + placement.left + i as i32,
                                        (p.1 + f64::from(*px)).round() as i32 - placement.top
                                            + y as i32,
                                        *c,
                                        a,
                                    );
                                }
                            }
                        }
                    } else if !ch.is_whitespace() {
                        return Err(invalid("字形光栅化失败"));
                    }
                    x += f64::from(metrics.advance_width(glyph));
                }
            }
        }
    }
    Ok(image.pixels)
}
