mod pixels;
mod rows;

use std::ops::Range;

use super::edge::Ramp;
use crate::render::fill::Fill;
use crate::render::{Canvas, PixelRect};
use pixels::Pixels;
use rows::{Rows, merge};

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

/// the rows of a shape inside a clip, each split into the pixels to sample
/// and the whole ones between them
pub struct Scan {
    start: (f32, f32),
    axis: (f32, f32),
    pixels: Pixels,
    rows: Rows,
    columns: (u32, u32),
    lines: (u32, u32),
    touched: (f32, f32),
    whole: [(f32, f32); 2],
}

/// columns of one row: sampled from start, whole from inner_start to
/// inner_end, sampled again up to end
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowSpan {
    pub start: u32,
    pub inner_start: u32,
    pub inner_end: u32,
    pub end: u32,
}

impl Scan {
    /// none when shape draws nothing inside clip in a buffer of size
    pub fn new(shape: &Turned, clip: PixelRect, size: (u32, u32)) -> Option<Self> {
        let area = shape.area(size)?.intersect(clip)?;
        let pixels = Pixels::new(shape);
        let (hu, hv, radius, margin) = (pixels.hu, pixels.hv, pixels.radius, pixels.ramp.reach);
        Some(Self {
            start: shape.start,
            axis: shape.axis,
            rows: Rows::new(shape),
            columns: (area.x, area.right()),
            lines: (area.y, area.bottom()),
            touched: (hu + margin, hv + margin),
            whole: [
                (hu - radius.max(margin), hv - margin),
                (hu - margin, hv - radius.max(margin)),
            ],
            pixels,
        })
    }

    pub fn open(shape: &Turned, clip: PixelRect, size: (u32, u32)) -> Option<Self> {
        let mut scan = Self::new(shape, clip, size)?;
        scan.pixels.open = true;
        let (along, across) = scan.touched;
        let margin = scan.pixels.ramp.reach;
        scan.whole = [(along, across - 2.0 * margin); 2];
        Some(scan)
    }

    pub fn rows(&self) -> Range<u32> {
        self.lines.0..self.lines.1
    }

    #[inline]
    pub fn row(&self, y: u32) -> RowSpan {
        let dy = y as f32 + 0.5 - self.start.1;
        let (start, end) = self
            .rows
            .span(dy, self.touched, self.columns.0, self.columns.1);
        let (inner_start, inner_end) = merge(
            self.rows.span(dy, self.whole[0], start, end),
            self.rows.span(dy, self.whole[1], start, end),
        );
        let inner_start = inner_start.max(start);
        RowSpan {
            start,
            inner_start,
            inner_end: inner_end.min(end).max(inner_start),
            end,
        }
    }

    /// every column of row y the shape can touch
    #[inline]
    pub fn touched(&self, y: u32) -> (u32, u32) {
        let dy = y as f32 + 0.5 - self.start.1;
        self.rows
            .span(dy, self.touched, self.columns.0, self.columns.1)
    }

    /// share of pixel x, y the shape covers
    #[inline]
    pub fn cover(&self, x: u32, y: u32) -> f32 {
        // measured from the start, so a longer bar keeps the same pixels there
        let (dx, dy) = (x as f32 + 0.5 - self.start.0, y as f32 + 0.5 - self.start.1);
        let (cos, sin) = self.axis;
        self.pixels.cover(dx * cos + dy * sin, dy * cos - dx * sin)
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
    let Some(scan) = Scan::new(&shape, clip, canvas.size()) else {
        return;
    };
    for y in scan.rows() {
        let row = scan.row(y);
        let span = canvas.span(y, row.start, row.end);
        let (before, rest) = span.split_at_mut((row.inner_start - row.start) as usize);
        let (inner, after) = rest.split_at_mut((row.inner_end - row.inner_start) as usize);
        let edges = before
            .iter_mut()
            .zip(row.start..)
            .chain(after.iter_mut().zip(row.inner_end..));
        for (pixel, x) in edges {
            add(pixel, fill.color(bar, x, y), scan.cover(x, y));
        }
        match fill.row(bar, y) {
            // one color along the row: a plain loop the compiler vectorizes
            Some(color) => inner.iter_mut().for_each(|pixel| add_whole(pixel, color)),
            None => {
                for (pixel, x) in inner.iter_mut().zip(row.inner_start..) {
                    add_whole(pixel, fill.color(bar, x, y));
                }
            }
        }
    }
}

/// makes every pixel shape can touch transparent
pub fn clear_turned(canvas: &mut Canvas<'_>, shape: Turned) {
    let size = canvas.size();
    let Some(scan) = Scan::new(&shape, PixelRect::full(size), size) else {
        return;
    };
    for y in scan.rows() {
        let (start, end) = scan.touched(y);
        canvas.span(y, start, end).fill([0; 4]);
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
