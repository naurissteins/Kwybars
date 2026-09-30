//! keeps a surface's bars on screen with as little pixel work as possible

#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicU64, Ordering};

use super::fill::Fill;
use super::geometry::Geometry;
use super::{ByteOrder, Canvas, PixelRect};
use crate::config::SurfaceConfig;

/// source of painter ids, so buffers know which geometry drew them
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// the bar extents one buffer holds, so a reused buffer is only patched
#[derive(Debug, Clone, Default)]
pub struct BufferContents {
    extents: Vec<f32>,
    opacity: u8,
    painter: u64,
}

/// lays out bars for one surface and paints the difference into buffers
#[derive(Debug)]
pub struct Painter {
    id: u64,
    bars: usize,
    layout: Geometry,
    bases: Vec<Fill>,
    fills: Vec<Fill>,
    scale: f32,
    next: Vec<f32>,
    opacity: u8,
    shown: Vec<f32>,
    shown_opacity: u8,
    shown_valid: bool,
}

impl Painter {
    pub fn new(
        config: &SurfaceConfig,
        bars: usize,
        size: (u32, u32),
        scale: f32,
        order: ByteOrder,
    ) -> Self {
        let layout = Geometry::new(config, size, scale, bars);
        let bases: Vec<Fill> = layout
            .axes()
            .map(|axis| Fill::new(config, axis, size, bars, order))
            .collect();
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            bars,
            next: vec![0.0; layout.elements()],
            opacity: u8::MAX,
            shown: vec![0.0; layout.elements()],
            shown_opacity: u8::MAX,
            layout,
            fills: bases.clone(),
            bases,
            scale,
            shown_valid: false,
        }
    }

    /// whether this painter was laid out for `size` and `scale`
    pub fn fits(&self, size: (u32, u32), scale: f32, bars: usize) -> bool {
        self.layout.size() == size && self.scale == scale && self.bars == bars
    }

    /// whether shown surfaces need frames with the bars at rest
    pub fn animates(&self) -> bool {
        self.layout.animates()
    }

    pub fn layout(&mut self, heights: &[f32], opacity: u8) -> bool {
        for (element, extent) in self.next.iter_mut().enumerate() {
            let value = heights.get(self.layout.bar(element)).copied();
            *extent = self.layout.extent(element, value.unwrap_or(0.0));
        }
        if opacity != self.opacity {
            for (fill, base) in self.fills.iter_mut().zip(&self.bases) {
                fill.fade_from(base, opacity);
            }
            self.opacity = opacity;
        }
        !self.shown_valid || self.next != self.shown || self.opacity != self.shown_opacity
    }

    /// empty contents sized for this painter's bars, for a new buffer
    pub fn new_contents(&self) -> BufferContents {
        BufferContents {
            extents: vec![0.0; self.next.len()],
            opacity: 0,
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
                    self.layout.paint(canvas, index, *extent, area, &self.fills);
                }
            }
            contents.extents.clone_from(&self.next);
            contents.opacity = self.opacity;
            contents.painter = self.id;
            return;
        }
        // a new opacity changes every bar pixel, but nothing outside the bars
        let faded = contents.opacity != self.opacity;
        contents.opacity = self.opacity;
        for (index, (held, next)) in contents.extents.iter_mut().zip(&self.next).enumerate() {
            let area = if faded {
                self.layout.area(index, held.max(*next))
            } else {
                self.layout.change(index, *held, *next)
            };
            if let Some(area) = area {
                canvas.clear(area);
                self.layout.paint(canvas, index, *next, area, &self.fills);
                *held = *next;
            }
        }
    }

    pub fn present(&mut self, mut each: impl FnMut(PixelRect)) {
        if self.shown_valid {
            let faded = self.shown_opacity != self.opacity;
            for (index, (shown, next)) in self.shown.iter().zip(&self.next).enumerate() {
                let area = if faded {
                    self.layout.area(index, shown.max(*next))
                } else {
                    self.layout.change(index, *shown, *next)
                };
                if let Some(area) = area {
                    each(area);
                }
            }
        } else {
            each(PixelRect::full(self.layout.size()));
        }
        self.shown.copy_from_slice(&self.next);
        self.shown_opacity = self.opacity;
        self.shown_valid = true;
    }
}
