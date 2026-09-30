use super::edge::{self, Ramp};
use crate::render::fill::Fill;
use crate::render::{Canvas, PixelRect};

/// a rounded rectangle from start along axis
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turned {
    /// middle of the starting side
    pub start: (f32, f32),
    /// unit vector along the length
    pub axis: (f32, f32),
    pub length: f32,
    pub half_width: f32,
    pub radius: f32,
}

impl Turned {
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let (cos, sin) = (self.axis.0.abs(), self.axis.1.abs());
        let margin = Ramp::new(cos, sin).reach;
        let (hu, hv) = (self.length * 0.5 + margin, self.half_width + margin);
        let (ex, ey) = (hu * cos + hv * sin, hu * sin + hv * cos);
        let half = self.length * 0.5;
        let (cx, cy) = (
            self.start.0 + self.axis.0 * half,
            self.start.1 + self.axis.1 * half,
        );
        (cx - ex, cy - ey, cx + ex, cy + ey)
    }

    pub fn area(&self, size: (u32, u32)) -> Option<PixelRect> {
        PixelRect::covering(self.bounds(), size)
    }
}

/// adds shape's pixels inside clip, each scaled by its coverage
pub fn fill_turned(
    canvas: &mut Canvas<'_>,
    shape: Turned,
    clip: PixelRect,
    fill: &Fill,
    bar: usize,
) {
    let Some((x0, x1, y0, y1)) = window(&shape, clip, canvas.size()) else {
        return;
    };
    let pixels = Pixels::new(&shape);
    let rows = Rows::new(&shape);
    let (hu, hv, radius, margin) = (pixels.hu, pixels.hv, pixels.radius, pixels.ramp.reach);
    let (cos, sin) = shape.axis;
    // pixel centers within the margin of the outline may be partly covered;
    // those the margin inside and clear of the corners, along the length or
    // across the width, are whole
    let touched = (hu + margin, hv + margin);
    let whole = [
        (hu - radius.max(margin), hv - margin),
        (hu - margin, hv - radius.max(margin)),
    ];
    for y in y0..y1 {
        let dy = y as f32 + 0.5 - shape.start.1;
        let (start, end) = rows.span(dy, touched, x0, x1);
        let (inner_start, inner_end) = merge(
            rows.span(dy, whole[0], start, end),
            rows.span(dy, whole[1], start, end),
        );
        let (inner_start, inner_end) = (
            inner_start.max(start),
            inner_end.min(end).max(inner_start.max(start)),
        );
        let span = canvas.span(y, start, end);
        let (before, rest) = span.split_at_mut((inner_start - start) as usize);
        let (inner, after) = rest.split_at_mut((inner_end - inner_start) as usize);
        let edges = before
            .iter_mut()
            .zip(start..)
            .chain(after.iter_mut().zip(inner_end..));
        for (pixel, x) in edges {
            let dx = x as f32 + 0.5 - shape.start.0;
            let coverage = pixels.cover(dx * cos + dy * sin, dy * cos - dx * sin);
            add(pixel, fill.color(bar, x, y), coverage);
        }
        match fill.row(bar, y) {
            // one color along the row: a plain loop the compiler vectorizes
            Some(color) => inner.iter_mut().for_each(|pixel| add_whole(pixel, color)),
            None => {
                for (pixel, x) in inner.iter_mut().zip(inner_start..) {
                    add_whole(pixel, fill.color(bar, x, y));
                }
            }
        }
    }
}

/// makes every pixel shape can touch transparent
pub fn clear_turned(canvas: &mut Canvas<'_>, shape: Turned) {
    let size = canvas.size();
    let Some((x0, x1, y0, y1)) = window(&shape, PixelRect::full(size), size) else {
        return;
    };
    let rows = Rows::new(&shape);
    let margin = Pixels::new(&shape).ramp.reach;
    let touched = (
        shape.length.max(0.0) * 0.5 + margin,
        shape.half_width.max(0.0) + margin,
    );
    for y in y0..y1 {
        let (start, end) = rows.span(y as f32 + 0.5 - shape.start.1, touched, x0, x1);
        canvas.span(y, start, end).fill([0; 4]);
    }
}

/// the columns and rows of shape's bounds inside clip
fn window(shape: &Turned, clip: PixelRect, size: (u32, u32)) -> Option<(u32, u32, u32, u32)> {
    let area = shape.area(size)?.intersect(clip)?;
    Some((area.x, area.right(), area.y, area.bottom()))
}

/// the columns of each row whose pixel centers lie within limits of a
/// shape's middle, along and across it
struct Rows {
    start: (f32, f32),
    axis: (f32, f32),
    half_length: f32,
    /// 1 / cos and -1 / sin, none where the axis makes them huge
    along: Option<f32>,
    across: Option<f32>,
}

impl Rows {
    fn new(shape: &Turned) -> Self {
        let (cos, sin) = shape.axis;
        let inverse = |k: f32| (k.abs() >= 1e-6).then(|| 1.0 / k);
        Self {
            start: shape.start,
            axis: shape.axis,
            half_length: shape.length.max(0.0) * 0.5,
            along: inverse(cos),
            across: inverse(-sin),
        }
    }

    /// columns inside x0..x1 of the row dy below the start
    #[inline]
    fn span(&self, dy: f32, (limit_u, limit_v): (f32, f32), x0: u32, x1: u32) -> (u32, u32) {
        if limit_u < 0.0 || limit_v < 0.0 {
            return (x1, x1);
        }
        let (cos, sin) = self.axis;
        // along: u - half = dx cos + dy sin - half; across: v = -dx sin + dy cos
        let (a, b) = slab(self.along, dy * sin - self.half_length, limit_u);
        let (c, d) = slab(self.across, dy * cos, limit_v);
        let (low, high) = (a.max(c), b.min(d));
        // pixel x has its center at x + 0.5; clamped first, so both are
        // whole or positive and a cast rounds them down
        let (x0, x1) = (x0 as f32, x1 as f32);
        let low = (self.start.0 + low - 0.5).clamp(x0, x1);
        let high = (self.start.0 + high + 0.5).clamp(low, x1);
        let first = low as u32 + u32::from((low as u32 as f32) < low);
        (first, (high as u32).max(first))
    }
}

/// the range of dx where |k dx + m| <= limit, given 1 / k
#[inline]
fn slab(inverse: Option<f32>, m: f32, limit: f32) -> (f32, f32) {
    let Some(inverse) = inverse else {
        return if m.abs() <= limit {
            (f32::NEG_INFINITY, f32::INFINITY)
        } else {
            (f32::INFINITY, f32::NEG_INFINITY)
        };
    };
    let (a, b) = ((-limit - m) * inverse, (limit - m) * inverse);
    (a.min(b), a.max(b))
}

/// one interval holding both when they meet, else the longer one
fn merge(a: (u32, u32), b: (u32, u32)) -> (u32, u32) {
    let (a, b) = if a.0 < a.1 { (a, b) } else { (b, a) };
    if b.0 >= b.1 {
        a
    } else if a.0 <= b.1 && b.0 <= a.1 {
        (a.0.min(b.0), a.1.max(b.1))
    } else if a.1 - a.0 >= b.1 - b.0 {
        a
    } else {
        b
    }
}

/// coverage of pixels of one shape, from their centers in its axes
#[derive(Debug, Clone, Copy)]
struct Pixels {
    ramp: Ramp,
    axis: (f32, f32),
    length: f32,
    hu: f32,
    hv: f32,
    radius: f32,
}

impl Pixels {
    fn new(shape: &Turned) -> Self {
        let (length, hv) = (shape.length.max(0.0), shape.half_width.max(0.0));
        let hu = length * 0.5;
        Self {
            ramp: Ramp::new(shape.axis.0, shape.axis.1),
            axis: shape.axis,
            length,
            hu,
            hv,
            radius: shape.radius.min(hu).min(hv).max(0.0),
        }
    }

    /// the pixel centered at u along from the start and v across
    #[inline]
    fn cover(&self, u: f32, v: f32) -> f32 {
        let (length, hv, radius) = (self.length, self.hv, self.radius);
        // the sides are square to the axis, so one ramp fits all four
        let rect = self.ramp.between(u, length - u) * self.ramp.between(v + hv, hv - v);
        if radius <= 0.0 || rect <= 0.0 {
            return rect;
        }
        let du = (radius - u).max(u - (length - radius));
        let dv = v.abs() - (hv - radius);
        if du <= 0.0 || dv <= 0.0 {
            return rect;
        }
        // back to buffer axes, where the pixel is square
        let (cos, sin) = self.axis;
        let ou = if u < radius { -du } else { du };
        let ov = dv.copysign(v);
        let (dx, dy) = (ou * cos - ov * sin, ou * sin + ov * cos);
        rect.min(edge::circle(dx, dy, radius))
    }
}

/// coverage of the pixel centered at u along from the start and v across
#[cfg(test)]
pub(super) fn coverage(u: f32, v: f32, shape: &Turned) -> f32 {
    Pixels::new(shape).cover(u, v)
}

#[inline]
fn add_whole(pixel: &mut [u8; 4], color: [u8; 4]) {
    for (out, channel) in pixel.iter_mut().zip(color) {
        *out = out.saturating_add(channel);
    }
}

/// shapes never overlap, so only their soft edges meet and add up
fn add(pixel: &mut [u8; 4], color: [u8; 4], coverage: f32) {
    if coverage <= 0.0 {
        return;
    }
    // one conversion per pixel, the channels in integers
    let share = (coverage.min(1.0) * 255.0 + 0.5) as u32;
    for (out, channel) in pixel.iter_mut().zip(color) {
        let value = (u32::from(channel) * share + 127) / 255;
        *out = out.saturating_add(value as u8);
    }
}
