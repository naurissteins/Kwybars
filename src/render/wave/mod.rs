mod compose;
mod curve;
mod mask;
mod shape;
#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use super::fill::Fill;
use super::{ByteOrder, Canvas, PixelRect};
use crate::config::{Edge, SurfaceConfig};
use compose::{Colors, compose};
use curve::{MAX_PIECES, Profile, flatten};
use mask::Mask;
use shape::WaveShape;

const FILL_ALPHA: f32 = 0.24;
const GLOW_ALPHA: f32 = 0.18;
const RISE: f32 = 1.35;
const FALL: f32 = 0.55;
const LEGACY_RATE_HZ: f32 = 60.0;
const ARRIVED: f32 = 1e-4;
const MAX_STEP: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Zone {
    start: u32,
    end: u32,
    filled: bool,
}

/// what a buffer holds of a wave: where each cell drew near the curve, and
/// everything it drew
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Held {
    zones: Vec<Zone>,
    bounds: Option<PixelRect>,
}

impl Held {
    /// forgets everything, for a buffer that was cleared
    pub fn clear(&mut self) {
        self.zones.fill(Zone::default());
        self.bounds = None;
    }
}

/// the wave of one surface: its values, its curve, and where it draws
#[derive(Debug)]
pub struct Wave {
    size: (u32, u32),
    shape: WaveShape,
    values: Vec<f32>,
    rise: f32,
    fall: f32,
    last: Option<Instant>,
    moving: bool,
    points: Vec<(f32, f32)>,
    line: Vec<(f32, f32)>,
    profile: Profile,
    zones: Vec<Zone>,
    reach: Vec<f32>,
    stroke: Mask,
    glow: Option<Mask>,
    bounds: Option<PixelRect>,
    curve_area: Option<PixelRect>,
    /// changes whenever the curve does
    version: u64,
}

impl Wave {
    pub fn new(config: &SurfaceConfig, size: (u32, u32), scale: f32, bars: usize) -> Self {
        let shape = WaveShape::new(config, size, scale, bars);
        let cells = if shape.along_x { size.0 } else { size.1 } as usize;
        let smoothing = config.visualizer.wave_motion_smoothing.clamp(0.0, 1.0);
        Self {
            size,
            values: vec![0.0; bars],
            rise: (smoothing * RISE).min(1.0),
            fall: (smoothing * FALL).min(1.0),
            last: None,
            moving: false,
            points: Vec::with_capacity(bars),
            line: Vec::with_capacity(bars * MAX_PIECES + 1),
            profile: Profile::new(cells),
            zones: vec![Zone::default(); cells],
            reach: reach(shape.glow.unwrap_or(shape.stroke) * 0.5 + 1.0),
            stroke: Mask::new(size),
            glow: shape.glow.map(|_| Mask::new(size)),
            bounds: None,
            curve_area: None,
            version: 0,
            shape,
        }
    }

    /// the colors of the stroke, the fill, and the glow, in that order
    pub fn fills(config: &SurfaceConfig, size: (u32, u32), order: ByteOrder) -> Vec<Fill> {
        let along_x = matches!(config.overlay.position, Edge::Bottom | Edge::Top);
        let length = if along_x { size.0 } else { size.1 };
        [1.0, FILL_ALPHA, GLOW_ALPHA]
            .map(|alpha| Fill::along(config, along_x, length, order, alpha))
            .to_vec()
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    /// every pixel the wave draws
    pub fn bounds(&self) -> Option<PixelRect> {
        self.bounds
    }

    /// whether the values still move towards bars at rest
    pub fn moving(&self) -> bool {
        self.moving
    }

    pub fn step(&mut self, heights: &[f32], now: Instant) {
        let elapsed = self
            .last
            .map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        let first = self.last.is_none();
        self.last = Some(now);
        let ticks = elapsed.min(MAX_STEP).as_secs_f32() * LEGACY_RATE_HZ;
        // the share left after a tick, raised to the ticks gone by
        let (rise, fall) = (
            1.0 - (1.0 - self.rise).powf(ticks),
            1.0 - (1.0 - self.fall).powf(ticks),
        );
        let (mut changed, mut moving) = (first, false);
        for (value, height) in self.values.iter_mut().zip(heights) {
            let gap = height - *value;
            if gap.abs() < ARRIVED {
                changed |= gap != 0.0;
                *value = *height;
                continue;
            }
            let moved = gap * if gap > 0.0 { rise } else { fall };
            changed |= moved != 0.0;
            // a value that cannot move any more is not on its way
            moving |= moved != 0.0;
            *value += moved;
        }
        self.moving = moving;
        if changed {
            self.lay_out();
        }
    }

    fn lay_out(&mut self) {
        self.version += 1;
        self.shape.points(&self.values, &mut self.points);
        flatten(&self.points, self.shape.control, &mut self.line);
        let along_x = self.shape.along_x;
        let swap = move |point: &(f32, f32)| if along_x { *point } else { (point.1, point.0) };
        self.profile.trace(self.line.iter().map(swap));
        self.stroke.stroke(&self.line, self.shape.stroke);
        if let (Some(mask), Some(width)) = (&mut self.glow, self.shape.glow) {
            mask.stroke(&self.line, width);
        }
        self.zone();
    }

    /// where each cell draws, and the bounds of it all
    fn zone(&mut self) {
        self.zones.fill(Zone::default());
        let depth = if self.shape.along_x {
            self.size.1
        } else {
            self.size.0
        };
        let reach = self.shape.glow.unwrap_or(self.shape.stroke) * 0.5 + 1.0;
        let window = reach.ceil() as usize + 1;
        let (first, last) = self.profile.cells();
        let cells = first.saturating_sub(window)..(last + window).min(self.zones.len());
        let (mut low, mut high) = (u32::MAX, 0);
        let (mut near_low, mut near_high) = (u32::MAX, 0);
        for cell in cells.clone() {
            let near = cell.saturating_sub(window)..=cell + window;
            let (lo, hi) = near.fold((f32::INFINITY, f32::NEG_INFINITY), |all, other| {
                let away = self.reach.get(other.abs_diff(cell)).copied();
                match (self.profile.span(other), away) {
                    (Some(span), Some(across)) if across > 0.0 => {
                        (all.0.min(span.0 - across), all.1.max(span.1 + across))
                    }
                    _ => all,
                }
            });
            if lo > hi {
                continue;
            }
            let zone = Zone {
                start: lo.floor().max(0.0) as u32,
                end: (hi.ceil().max(0.0) as u32).min(depth),
                filled: self.shape.baseline.is_some() && self.profile.share(cell) >= 0.5,
            };
            let (plain_start, plain_end) = self.plain(zone);
            low = low.min(zone.start).min(plain_start);
            high = high.max(zone.end).max(plain_end);
            (near_low, near_high) = (near_low.min(zone.start), near_high.max(zone.end));
            if let Some(slot) = self.zones.get_mut(cell) {
                *slot = zone;
            }
        }
        let along = (cells.start as u32, cells.end as u32);
        self.bounds = self.area(along, (low, high));
        self.curve_area = self.area(along, (near_low, near_high));
    }

    /// the pixels of cells along and rows or columns across, if any
    fn area(&self, along: (u32, u32), across: (u32, u32)) -> Option<PixelRect> {
        let (x, y) = if self.shape.along_x {
            (along, across)
        } else {
            (across, along)
        };
        (x.0 < x.1 && y.0 < y.1).then(|| PixelRect {
            x: x.0,
            y: y.0,
            width: x.1 - x.0,
            height: y.1 - y.0,
        })
    }

    /// the whole pixel across the fill reaches to
    fn baseline(&self) -> Option<u32> {
        self.shape
            .baseline
            .map(|across| across.round().max(0.0) as u32)
    }

    /// rows or columns of a cell that are fill and nothing else
    fn plain(&self, zone: Zone) -> (u32, u32) {
        match self.baseline() {
            Some(baseline) if zone.filled && self.shape.from_start => {
                (baseline.min(zone.start), zone.start)
            }
            Some(baseline) if zone.filled => (zone.end, baseline.max(zone.end)),
            _ => (zone.end, zone.end),
        }
    }

    /// what a clear buffer holds
    pub fn held(&self) -> Held {
        Held {
            zones: vec![Zone::default(); self.zones.len()],
            bounds: None,
        }
    }

    pub fn paint(&self, canvas: &mut Canvas<'_>, fills: &[Fill], held: &mut Held, whole: bool) {
        if held.zones.len() != self.zones.len() {
            *held = self.held();
        }
        if let [stroke, fill, glow] = fills {
            compose(self, canvas, &Colors { stroke, fill, glow }, held, whole);
        }
    }

    /// the pixels near the curve: all that changes from one wave to the next
    pub fn curve_area(&self) -> Option<PixelRect> {
        self.curve_area
    }
}

fn reach(radius: f32) -> Vec<f32> {
    let cells = radius.ceil() as usize + 2;
    (0..cells)
        .map(|away| {
            let along = (away as f32 - 1.0).max(0.0);
            (radius * radius - along * along).max(0.0).sqrt()
        })
        .collect()
}
