//! keeps a surface's bars on screen with as little pixel work as possible

#[cfg(test)]
mod tests;
mod wave;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use super::dots::Drift;
use super::fill::Fill;
use super::geometry::Geometry;
use super::wave::{Held, Wave};
use super::{ByteOrder, Canvas, PixelRect, Pose};
use crate::config::{Layout, SurfaceConfig};

/// source of painter ids, so buffers know which geometry drew them
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// the element poses one buffer holds, so a reused buffer is only patched
#[derive(Debug, Clone, Default)]
pub struct BufferContents {
    poses: Vec<Pose>,
    wave: Held,
    /// the wave's version
    version: u64,
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
    drift: Option<Drift>,
    wave: Option<Wave>,
    /// version, bounds, and curve area
    shown_wave: (u64, Option<PixelRect>, Option<PixelRect>),
    /// when the layout first showed, for its turn
    epoch: Option<Instant>,
    scale: f32,
    next: Vec<Pose>,
    opacity: u8,
    shown: Vec<Pose>,
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
        let wave = (config.visualizer.layout == Layout::Wave)
            .then(|| Wave::new(config, size, scale, bars));
        let bases = match &wave {
            Some(_) => Wave::fills(config, size, order),
            None => layout.fills(config, bars, order),
        };
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            bars,
            drift: layout.drifting().then(|| Drift::new(bars)),
            wave,
            shown_wave: (0, None, None),
            epoch: None,
            next: vec![Pose::default(); layout.elements()],
            opacity: u8::MAX,
            shown: vec![Pose::default(); layout.elements()],
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
        self.layout.turning()
            || self.drift.as_ref().is_some_and(|drift| !drift.settled())
            || self.wave.as_ref().is_some_and(Wave::moving)
    }

    pub fn layout(&mut self, heights: &[f32], opacity: u8, now: Instant) -> bool {
        if let Some(drift) = &mut self.drift {
            drift.step(heights, now);
        }
        if let Some(wave) = &mut self.wave {
            wave.step(heights, now);
        }
        let epoch = *self.epoch.get_or_insert(now);
        let turn = self
            .layout
            .turn(now.saturating_duration_since(epoch).as_secs_f64());
        for (element, pose) in self.next.iter_mut().enumerate() {
            let bar = self.layout.bar(element);
            let value = heights.get(bar).copied().unwrap_or(0.0);
            let lift = self.drift.as_ref().map_or(0.0, |drift| drift.lift(bar));
            *pose = self.layout.pose(element, value, lift, turn);
        }
        if opacity != self.opacity {
            for (fill, base) in self.fills.iter_mut().zip(&self.bases) {
                fill.fade_from(base, opacity);
            }
            self.opacity = opacity;
        }
        let waved = self.wave.as_ref().map_or(0, Wave::version) != self.shown_wave.0;
        !self.shown_valid || waved || self.next != self.shown || self.opacity != self.shown_opacity
    }

    /// empty contents sized for this painter's bars, for a new buffer
    pub fn new_contents(&self) -> BufferContents {
        BufferContents {
            poses: vec![Pose::default(); self.next.len()],
            wave: self.wave.as_ref().map(Wave::held).unwrap_or_default(),
            version: 0,
            opacity: 0,
            painter: 0,
        }
    }

    /// brings canvas, which holds contents, up to the laid out bars
    pub fn paint(&self, canvas: &mut Canvas<'_>, contents: &mut BufferContents) {
        if let Some(wave) = &self.wave {
            return self.paint_wave(wave, canvas, contents);
        }
        // a buffer drawn with other geometry, or never, is drawn whole
        if contents.painter != self.id || contents.poses.len() != self.next.len() {
            canvas.clear(PixelRect::full(canvas.size()));
            for (index, pose) in self.next.iter().enumerate() {
                if let Some(area) = self.layout.area(index, *pose) {
                    self.layout.paint(canvas, index, *pose, area, &self.fills);
                }
            }
            contents.poses.clone_from(&self.next);
            contents.opacity = self.opacity;
            contents.painter = self.id;
            return;
        }
        let faded = contents.opacity != self.opacity;
        contents.opacity = self.opacity;
        let changes = || self.changes(&contents.poses, faded);
        if !self.layout.overlapping() {
            for (index, area) in changes() {
                canvas.clear(area);
                if let Some(pose) = self.next.get(index) {
                    self.layout.paint(canvas, index, *pose, area, &self.fills);
                }
            }
        } else if self.redraw_is_cheaper(&contents.poses, changes()) {
            // every drawn pixel is in some element's held footprint
            for (index, pose) in contents.poses.iter().enumerate() {
                self.layout.clear(canvas, index, *pose);
            }
            for (index, pose) in self.next.iter().enumerate() {
                if let Some(area) = self.layout.area(index, *pose) {
                    self.layout.paint(canvas, index, *pose, area, &self.fills);
                }
            }
        } else {
            // other elements reach into a changed area, so all are redrawn there
            for (_, area) in changes() {
                canvas.clear(area);
                for (index, pose) in self.next.iter().enumerate() {
                    let clip = self
                        .layout
                        .area(index, *pose)
                        .and_then(|a| a.intersect(area));
                    if let Some(clip) = clip {
                        self.layout.paint(canvas, index, *pose, clip, &self.fills);
                    }
                }
            }
        }
        contents.poses.copy_from_slice(&self.next);
    }

    pub fn present(&mut self, mut each: impl FnMut(PixelRect)) {
        if self.wave.is_some() {
            self.present_wave(&mut each);
        } else if self.shown_valid {
            let faded = self.shown_opacity != self.opacity;
            for (_, area) in self.changes(&self.shown, faded) {
                each(area);
            }
        } else {
            each(PixelRect::full(self.layout.size()));
        }
        self.shown.copy_from_slice(&self.next);
        self.shown_opacity = self.opacity;
        self.shown_valid = true;
    }

    /// whether clearing and drawing every element costs fewer pixels than
    /// redrawing each changed area with the elements reaching into it
    fn redraw_is_cheaper(
        &self,
        poses: &[Pose],
        changes: impl Iterator<Item = (usize, PixelRect)>,
    ) -> bool {
        let areas: u64 = changes.map(|(_, area)| area.pixels()).sum();
        let footprints = poses.iter().zip(&self.next).enumerate();
        let redraw: u64 = footprints
            .map(|(index, (held, next))| {
                self.layout.footprint(index, *held) + self.layout.footprint(index, *next)
            })
            .sum();
        redraw < areas
    }

    /// the elements whose pixels differ from poses, and where; a new
    /// opacity changes every element pixel, but nothing outside the elements
    fn changes<'a>(
        &'a self,
        poses: &'a [Pose],
        faded: bool,
    ) -> impl Iterator<Item = (usize, PixelRect)> + 'a {
        poses
            .iter()
            .zip(&self.next)
            .enumerate()
            .filter_map(move |(index, (held, next))| {
                let area = if faded {
                    self.layout.cover(index, *held, *next)
                } else {
                    self.layout.change(index, *held, *next)
                };
                area.map(|area| (index, area))
            })
    }
}
