//! anti-aliased rectangles with rounded corners, written row by row

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
        let corner_row =
            radius > 0.0 && (row_top < shape.top + radius || row_top + 1.0 > shape.bottom - radius);
        let inset = if corner_row { radius } else { 0.0 };
        // columns fully inside horizontally and clear of the corners
        let inner_start = ((shape.left + inset).ceil().max(0.0) as u32).clamp(x0, x1);
        let inner_end = ((shape.right - inset).floor().max(0.0) as u32).clamp(inner_start, x1);

        let span = canvas.span(y, x0, x1);
        let (before, rest) = span.split_at_mut((inner_start - x0) as usize);
        let (inner, after) = rest.split_at_mut((inner_end - inner_start) as usize);
        for (pixel, x) in before
            .iter_mut()
            .zip(x0..)
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

/// coverage of the pixel at `x`, `y` outside the plain interior
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
    // one-sample estimate from the distance to the corner's circle
    let distance = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
    rect.min((radius - distance + 0.5).clamp(0.0, 1.0))
}

/// how much of the unit interval starting at `start` lies inside `low..high`
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

#[cfg(test)]
mod tests {
    use super::{RoundedRect, fill};
    use crate::render::fill::Fill;
    use crate::render::{Canvas, PixelRect};

    const WHITE: Fill = Fill::Solid([255; 4]);

    fn render(shape: RoundedRect, size: (u32, u32), clip: PixelRect) -> Vec<u8> {
        let mut data = vec![0_u8; (size.0 * size.1 * 4) as usize];
        let Some(mut canvas) = Canvas::new(&mut data, size) else {
            panic!("canvas");
        };
        fill(&mut canvas, shape, clip, &WHITE, 0);
        data
    }

    fn alpha(data: &[u8], width: u32, x: u32, y: u32) -> u8 {
        data[((y * width + x) * 4 + 3) as usize]
    }

    fn rect(left: f32, top: f32, right: f32, bottom: f32, radius: f32) -> RoundedRect {
        RoundedRect {
            left,
            top,
            right,
            bottom,
            radius,
        }
    }

    #[test]
    fn square_rect_fills_whole_pixels_and_blends_a_fractional_edge() {
        let size = (6, 6);
        let data = render(rect(1.0, 2.5, 4.0, 6.0, 0.0), size, PixelRect::full(size));
        assert_eq!(alpha(&data, 6, 0, 4), 0);
        assert_eq!(alpha(&data, 6, 1, 4), 255);
        assert_eq!(alpha(&data, 6, 3, 5), 255);
        assert_eq!(alpha(&data, 6, 4, 5), 0);
        // half covered top row
        assert_eq!(alpha(&data, 6, 2, 2), 128);
        assert_eq!(alpha(&data, 6, 2, 1), 0);
    }

    #[test]
    fn rounded_corners_fade_out_symmetrically() {
        let size = (20, 20);
        let data = render(rect(0.0, 0.0, 20.0, 20.0, 8.0), size, PixelRect::full(size));
        // corners are empty, the middle and the edge midpoints are full
        assert_eq!(alpha(&data, 20, 0, 0), 0);
        assert_eq!(alpha(&data, 20, 19, 19), 0);
        assert_eq!(alpha(&data, 20, 10, 10), 255);
        assert_eq!(alpha(&data, 20, 10, 0), 255);
        assert_eq!(alpha(&data, 20, 0, 10), 255);
        for (x, y) in [(2, 2), (1, 4), (4, 1)] {
            let corner = alpha(&data, 20, x, y);
            assert_eq!(corner, alpha(&data, 20, 19 - x, y));
            assert_eq!(corner, alpha(&data, 20, x, 19 - y));
            assert_eq!(corner, alpha(&data, 20, y, x));
        }
        assert!(alpha(&data, 20, 2, 2) > 0 && alpha(&data, 20, 2, 2) < 255);
    }

    #[test]
    fn clip_limits_the_written_rows() {
        let size = (4, 8);
        let clip = PixelRect {
            x: 0,
            y: 2,
            width: 4,
            height: 3,
        };
        let data = render(rect(0.0, 0.0, 4.0, 8.0, 0.0), size, clip);
        let rows: Vec<u8> = (0..8).map(|y| alpha(&data, 4, 1, y)).collect();
        assert_eq!(rows, vec![0, 0, 255, 255, 255, 0, 0, 0]);
    }

    #[test]
    fn a_clip_beside_the_shape_draws_nothing() {
        let size = (8, 4);
        let clip = PixelRect {
            x: 6,
            y: 0,
            width: 2,
            height: 4,
        };
        let data = render(rect(0.0, 0.0, 3.0, 4.0, 1.0), size, clip);
        assert!(data.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn touching_shapes_add_up_to_full_coverage() {
        let size = (2, 4);
        let mut data = vec![0_u8; 2 * 4 * 4];
        let Some(mut canvas) = Canvas::new(&mut data, size) else {
            panic!("canvas");
        };
        fill(
            &mut canvas,
            rect(0.0, 0.0, 2.0, 1.5, 0.0),
            PixelRect::full(size),
            &WHITE,
            0,
        );
        fill(
            &mut canvas,
            rect(0.0, 1.5, 2.0, 4.0, 0.0),
            PixelRect::full(size),
            &WHITE,
            0,
        );
        assert_eq!(alpha(&data, 2, 0, 1), 255);
    }
}
