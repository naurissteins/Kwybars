//! anti-aliased rectangles with rounded corners, written row by row

pub mod edge;
#[cfg(test)]
mod tests;
mod turned;

pub use turned::{RowSpan, Scan, Turned, clear_turned, fill_turned};

use std::f32::consts::FRAC_1_SQRT_2;

use super::fill::Fill;
use super::{Canvas, PixelRect};

/// a rectangle in pixel coordinates with every corner rounded by `radius`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoundedRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    /// clamped to half the shorter side
    pub radius: f32,
}

/// writes the pixels of `shape` inside `clip` in bar `bar`'s colors, each
/// scaled by how much of the pixel the shape covers
pub fn fill(canvas: &mut Canvas<'_>, shape: RoundedRect, clip: PixelRect, fill: &Fill, bar: usize) {
    let Some(area) = PixelRect::covering(
        (shape.left, shape.top, shape.right, shape.bottom),
        canvas.size(),
    ) else {
        return;
    };
    let radius = shape
        .radius
        .min((shape.right - shape.left) * 0.5)
        .min((shape.bottom - shape.top) * 0.5)
        .max(0.0);
    let (x0, x1) = (area.x.max(clip.x), area.right().min(clip.right()));
    let (y0, y1) = (area.y.max(clip.y), area.bottom().min(clip.bottom()));
    // `clamp` below needs x0 <= x1; a shape outside the clip draws nothing
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    for y in y0..y1 {
        let row_top = y as f32;
        let cy = overlap(row_top, shape.top, shape.bottom);
        let (reach, full) = columns(&shape, radius, row_top);
        let to_column = |edge: f32| edge.max(0.0) as u32;
        let inner_start = to_column(full.0.ceil()).clamp(x0, x1);
        let inner_end = to_column(full.1.floor()).clamp(inner_start, x1);
        let start = to_column(reach.0.floor()).clamp(x0, inner_start);
        let end = to_column(reach.1.ceil()).clamp(inner_end, x1);

        let span = canvas.span(y, start, end);
        let (before, rest) = span.split_at_mut((inner_start - start) as usize);
        let (inner, after) = rest.split_at_mut((inner_end - inner_start) as usize);
        for (pixel, x) in before
            .iter_mut()
            .zip(start..)
            .chain(after.iter_mut().zip(inner_end..))
        {
            let coverage = edge_coverage(&shape, radius, x as f32, row_top, cy);
            blend(pixel, fill.color(bar, x, y), coverage);
        }
        if cy >= 1.0 {
            // the bulk of every bar: whole pixels, a copy or a constant fill
            fill.span(bar, inner_start, y, inner);
        } else {
            for (pixel, x) in inner.iter_mut().zip(inner_start..) {
                blend(pixel, fill.color(bar, x, y), cy);
            }
        }
    }
}

fn columns(shape: &RoundedRect, radius: f32, row_top: f32) -> ((f32, f32), (f32, f32)) {
    let plain = (shape.left, shape.right);
    let corner_row =
        radius > 0.0 && (row_top < shape.top + radius || row_top + 1.0 > shape.bottom - radius);
    if !corner_row {
        return (plain, plain);
    }
    let py = row_top + 0.5;
    // max/min rather than clamp, which panics on crossed bounds
    let dy = (py - py.max(shape.top + radius).min(shape.bottom - radius)).abs();
    let chord = |r: f32| {
        if dy < r {
            (r * r - dy * dy).sqrt()
        } else {
            0.0
        }
    };
    // a pixel is touched up to half a diagonal from the circle
    let (inner, outer) = (
        chord((radius - FRAC_1_SQRT_2).max(0.0)),
        chord(radius + FRAC_1_SQRT_2),
    );
    let (left, right) = (shape.left + radius, shape.right - radius);
    (
        (left - outer - 0.5, right + outer + 0.5),
        (left - inner - 0.5, right + inner + 0.5),
    )
}

fn edge_coverage(shape: &RoundedRect, radius: f32, x: f32, y: f32, cy: f32) -> f32 {
    let rect = overlap(x, shape.left, shape.right) * cy;
    if radius <= 0.0 {
        return rect;
    }
    let (px, py) = (x + 0.5, y + 0.5);
    // max/min rather than clamp, which panics on crossed bounds
    let cx = px.max(shape.left + radius).min(shape.right - radius);
    let cy = py.max(shape.top + radius).min(shape.bottom - radius);
    if cx == px || cy == py {
        return rect;
    }
    rect.min(edge::circle(px - cx, py - cy, radius))
}

fn overlap(start: f32, low: f32, high: f32) -> f32 {
    ((start + 1.0).min(high) - start.max(low)).clamp(0.0, 1.0)
}

fn blend(pixel: &mut [u8; 4], color: [u8; 4], coverage: f32) {
    if coverage >= 1.0 {
        *pixel = color;
    } else if coverage > 0.0 {
        for (out, channel) in pixel.iter_mut().zip(color) {
            let add = (f32::from(channel) * coverage + 0.5) as u8;
            *out = out.saturating_add(add);
        }
    }
}
