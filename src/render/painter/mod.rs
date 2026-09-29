//! keeps a surface's bars on screen with as little pixel work as possible

#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicU64, Ordering};

use super::fill::Fill;
use super::line::LineLayout;
use super::{ByteOrder, Canvas, PixelRect};
use crate::config::SurfaceConfig;

/// source of painter ids, so buffers know which geometry drew them
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// the bar extents one buffer holds, so a reused buffer is only patched
#[derive(Debug, Clone, Default)]
pub struct BufferContents {
    extents: Vec<f32>,
    /// the painter that drew them; 0 for a new buffer, whose pixels are unknown
    painter: u64,
}

/// lays out bars for one surface and paints the difference into buffers
#[derive(Debug)]
pub struct Painter {
    id: u64,
    layout: LineLayout,
    fill: Fill,
    scale: f32,
    /// bar extents to draw next
    next: Vec<f32>,
    /// bar extents on screen, what the compositor's damage is relative to
    shown: Vec<f32>,
    shown_valid: bool,
}

impl Painter {
    /// allocates everything for a buffer of `size` at `scale` buffer pixels
    /// per logical pixel; later calls do not allocate
    pub fn new(
        config: &SurfaceConfig,
        bars: usize,
        size: (u32, u32),
        scale: f32,
        order: ByteOrder,
    ) -> Self {
        let edge = config.overlay.position;
        let layout = LineLayout::new(&config.visualizer, edge, size, scale, bars);
        let fill = Fill::new(config, edge, size, bars, order);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            next: vec![0.0; layout.bars()],
            shown: vec![0.0; layout.bars()],
            layout,
            fill,
            scale,
            shown_valid: false,
        }
    }

    /// whether this painter was laid out for `size` and `scale`
    pub fn fits(&self, size: (u32, u32), scale: f32) -> bool {
        self.layout.size() == size && self.scale == scale
    }

    /// lays out `heights`; false when the result matches what is shown
    pub fn layout(&mut self, heights: &[f32]) -> bool {
        for (index, extent) in self.next.iter_mut().enumerate() {
            *extent = self
                .layout
                .extent(heights.get(index).copied().unwrap_or(0.0));
        }
        !self.shown_valid || self.next != self.shown
    }

    /// empty contents sized for this painter's bars, for a new buffer
    pub fn new_contents(&self) -> BufferContents {
        BufferContents {
            extents: vec![0.0; self.next.len()],
            painter: 0,
        }
    }

    /// brings `canvas`, which holds `contents`, up to the laid out bars
    pub fn paint(&self, canvas: &mut Canvas<'_>, contents: &mut BufferContents) {
        // a buffer drawn with other geometry, or never, is drawn whole
        if contents.painter != self.id || contents.extents.len() != self.next.len() {
            canvas.clear(PixelRect::full(canvas.size()));
            for (index, extent) in self.next.iter().enumerate() {
                if let Some(area) = self.layout.area(index, *extent) {
                    self.layout.paint(canvas, index, *extent, area, &self.fill);
                }
            }
            contents.extents.clone_from(&self.next);
            contents.painter = self.id;
            return;
        }
        for (index, (held, next)) in contents.extents.iter_mut().zip(&self.next).enumerate() {
            if let Some(area) = self.layout.change(index, *held, *next) {
                canvas.clear(area);
                self.layout.paint(canvas, index, *next, area, &self.fill);
                *held = *next;
            }
        }
    }

    /// calls `each` with every area that differs from what is on screen,
    /// then records the laid out bars as shown
    pub fn present(&mut self, mut each: impl FnMut(PixelRect)) {
        if self.shown_valid {
            for (index, (shown, next)) in self.shown.iter().zip(&self.next).enumerate() {
                if let Some(area) = self.layout.change(index, *shown, *next) {
                    each(area);
                }
            }
        } else {
            each(PixelRect::full(self.layout.size()));
        }
        self.shown.copy_from_slice(&self.next);
        self.shown_valid = true;
    }
}
