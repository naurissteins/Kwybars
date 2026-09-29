//! keeps a surface's bars on screen with as little pixel work as possible

#[cfg(test)]
mod tests;

use tiny_skia::{Color, Paint, Rect};

use super::damage::{self, PixelRect};
use super::pattern::Pattern;
use super::{ByteOrder, Canvas};
use crate::config::{Rgba, SurfaceConfig};

/// the bars one buffer holds, so a reused buffer is only patched
#[derive(Debug, Clone, Default)]
pub struct BufferContents {
    bars: Vec<Option<Rect>>,
    /// false for a new buffer, whose pixels are unknown
    valid: bool,
}

/// lays out bars for one surface and paints the difference into buffers
#[derive(Debug)]
pub struct Painter {
    pattern: Pattern,
    paint: Paint<'static>,
    size: (u32, u32),
    next: Vec<Option<Rect>>,
    shown: Vec<Option<Rect>>,
    shown_valid: bool,
}

impl Painter {
    /// allocates everything; later calls do not allocate
    pub fn new(config: &SurfaceConfig, bars: usize, size: (u32, u32), order: ByteOrder) -> Self {
        let mut paint = Paint::default();
        paint.set_color(color(config, order));
        paint.anti_alias = true;
        Self {
            pattern: Pattern::new(config.overlay.position, size, bars),
            paint,
            size,
            next: vec![None; bars],
            shown: vec![None; bars],
            shown_valid: false,
        }
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// a new buffer size; everything is redrawn and damaged
    pub fn resize(&mut self, size: (u32, u32), edge: crate::config::Edge) {
        self.pattern = Pattern::new(edge, size, self.next.len());
        self.size = size;
        self.shown_valid = false;
    }

    /// lays out `heights`; false when the result matches what is shown
    pub fn layout(&mut self, heights: &[f32]) -> bool {
        for (index, bar) in self.next.iter_mut().enumerate() {
            let value = heights.get(index).copied().unwrap_or(0.0);
            *bar = self.pattern.bar(index, value);
        }
        !self.shown_valid || self.next != self.shown
    }

    /// empty contents sized for this painter's bars, for a new buffer
    pub fn new_contents(&self) -> BufferContents {
        BufferContents {
            bars: vec![None; self.next.len()],
            valid: false,
        }
    }

    /// brings `canvas`, which holds `contents`, up to the laid out bars
    pub fn paint(&self, canvas: &mut Canvas<'_>, contents: &mut BufferContents) {
        let size = canvas.size();
        if !contents.valid || contents.bars.len() != self.next.len() {
            let full = PixelRect::full(size);
            canvas.clear(full);
            for rect in self.next.iter().flatten() {
                canvas.fill(*rect, &self.paint);
            }
            contents.bars.clone_from(&self.next);
            contents.valid = true;
            return;
        }
        for (held, next) in contents.bars.iter_mut().zip(&self.next) {
            // neighbours never share pixels, so each bar is patched alone
            if let Some(area) = damage::change(*held, *next, size) {
                canvas.clear(area);
                // only the part of the bar inside the cleared band is drawn,
                // so translucent colors are never blended twice
                let visible = next
                    .zip(area.to_rect())
                    .and_then(|(rect, area)| rect.intersect(&area));
                if let Some(rect) = visible {
                    canvas.fill(rect, &self.paint);
                }
                *held = *next;
            }
        }
    }

    /// calls `each` with every area that differs from what is on screen,
    /// then records the laid out bars as shown
    pub fn present(&mut self, mut each: impl FnMut(PixelRect)) {
        if self.shown_valid {
            for (shown, next) in self.shown.iter().zip(&self.next) {
                if let Some(area) = damage::change(*shown, *next, self.size) {
                    each(area);
                }
            }
        } else {
            each(PixelRect::full(self.size));
        }
        self.shown.copy_from_slice(&self.next);
        self.shown_valid = true;
    }
}

/// the first theme color, else the configured bar color, in `order`
fn color(config: &SurfaceConfig, order: ByteOrder) -> Color {
    let Rgba { r, g, b, a } = config
        .theme_colors
        .map_or(config.visualizer.color_rgba, |colors| colors[0]);
    let (r, b) = match order {
        ByteOrder::Rgba => (r, b),
        ByteOrder::Bgra => (b, r),
    };
    Color::from_rgba(r, g, b, a).unwrap_or(Color::WHITE)
}
