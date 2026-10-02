use super::{Held, Wave, Zone};
use crate::render::Canvas;
use crate::render::fill::Fill;

/// the colors of the stroke, the fill under it, and the glow under both
pub struct Colors<'a> {
    pub stroke: &'a Fill,
    pub fill: &'a Fill,
    pub glow: &'a Fill,
}

pub fn compose(
    wave: &Wave,
    canvas: &mut Canvas<'_>,
    colors: &Colors<'_>,
    held: &mut Held,
    whole: bool,
) {
    if whole {
        if let Some(old) = held.bounds {
            canvas.clear(old);
        }
        held.zones.fill(Zone::default());
    }
    let along_x = wave.shape.along_x;
    let mut palette = None;
    for (cell, (new, old)) in wave.zones.iter().zip(&mut held.zones).enumerate() {
        let ranges = rewrite(wave, *old, *new);
        *old = *new;
        let Some(first) = ranges.iter().find(|range| range.0 < range.1) else {
            continue;
        };
        let cell = cell as u32;
        // the colors run along the cells, so any pixel of the cell has them
        let (x, y) = if along_x {
            (cell, first.0)
        } else {
            (first.0, cell)
        };
        let colors = Palette::at(colors, x, y, palette);
        palette = Some(colors);
        let near = Near::new(wave, cell as usize, colors);
        let plain = wave.plain(*new);
        let color = |across: u32| {
            if (new.start..new.end).contains(&across) {
                let (x, y) = if along_x {
                    (cell, across)
                } else {
                    (across, cell)
                };
                near.pixel(across, x, y)
            } else if (plain.0..plain.1).contains(&across) {
                colors.fill
            } else {
                [0; 4]
            }
        };
        for range in ranges.iter().filter(|range| range.0 < range.1) {
            if along_x {
                for across in range.0..range.1 {
                    if let Some(pixel) = canvas.pixel(cell, across) {
                        *pixel = color(across);
                    }
                }
            } else {
                let row = canvas.span(cell, range.0, range.1);
                for (pixel, across) in row.iter_mut().zip(range.0..) {
                    *pixel = color(across);
                }
            }
        }
    }
    held.bounds = wave.bounds;
}

/// the rows or columns of a cell that may differ between two zones, as up
/// to two ranges that do not overlap
fn rewrite(wave: &Wave, old: Zone, new: Zone) -> [(u32, u32); 2] {
    let hull = |a: (u32, u32), b: (u32, u32)| match (a.0 < a.1, b.0 < b.1) {
        (true, true) => (a.0.min(b.0), a.1.max(b.1)),
        (true, false) => a,
        _ => b,
    };
    let (was, is) = ((old.start, old.end), (new.start, new.end));
    if old.filled != new.filled {
        // the fill beyond the curve came or went
        return [
            hull(hull(hull(was, is), wave.plain(old)), wave.plain(new)),
            (0, 0),
        ];
    }
    if new.filled || (was.0 < is.1 && is.0 < was.1) {
        // with a fill, what lies between the two changes too
        return [hull(was, is), (0, 0)];
    }
    [was, is]
}

#[derive(Clone, Copy, PartialEq)]
struct Palette {
    stroke: [u8; 4],
    fill: [u8; 4],
    glow: [u8; 4],
    /// by glow, fill, stroke as bits 4, 2, 1
    whole: [[u8; 4]; 8],
}

impl Palette {
    fn new(stroke: [u8; 4], fill: [u8; 4], glow: [u8; 4]) -> Self {
        let mut whole = [[0; 4]; 8];
        for (layers, color) in whole.iter_mut().enumerate() {
            let on = |bit: usize| if layers & bit != 0 { 255 } else { 0 };
            *color = over(over(over([0; 4], glow, on(4)), fill, on(2)), stroke, on(1));
        }
        Self {
            stroke,
            fill,
            glow,
            whole,
        }
    }

    /// the palette at pixel x, y, reusing before when the colors are the same
    fn at(colors: &Colors<'_>, x: u32, y: u32, before: Option<Self>) -> Self {
        let (stroke, fill, glow) = (
            colors.stroke.color(0, x, y),
            colors.fill.color(0, x, y),
            colors.glow.color(0, x, y),
        );
        match before {
            Some(same) if (same.stroke, same.fill, same.glow) == (stroke, fill, glow) => same,
            _ => Self::new(stroke, fill, glow),
        }
    }
}

/// what every pixel near the curve in one cell shares
struct Near<'a> {
    wave: &'a Wave,
    palette: Palette,
    fill: Option<((f32, f32), (u32, u32))>,
    sides: (u32, u32),
    room: (u32, u32),
}

impl<'a> Near<'a> {
    fn new(wave: &'a Wave, cell: usize, palette: Palette) -> Self {
        let filled = wave.zones.get(cell).is_some_and(|zone| zone.filled);
        let fill = match (filled, wave.profile.span(cell)) {
            (true, Some(span)) => Some((
                span,
                (span.0.max(0.0) as u32, span.1.max(0.0).ceil() as u32),
            )),
            _ => None,
        };
        let baseline = wave.baseline().unwrap_or(0);
        let (sides, room) = if wave.shape.from_start {
            ((255, 0), (baseline, u32::MAX))
        } else {
            ((0, 255), (0, baseline))
        };
        Self {
            wave,
            palette,
            fill,
            sides,
            room,
        }
    }

    #[inline]
    fn filled(&self, across: u32) -> u32 {
        let Some(((low, high), (first, end))) = self.fill else {
            return 0;
        };
        if across < self.room.0 || across >= self.room.1 {
            0
        } else if across < first {
            self.sides.0
        } else if across >= end {
            self.sides.1
        } else {
            let pixel = across as f32;
            let share = if self.wave.shape.from_start {
                mean_clamped(low - pixel, high - pixel)
            } else {
                mean_clamped(pixel + 1.0 - high, pixel + 1.0 - low)
            };
            (share * 255.0 + 0.5) as u32
        }
    }

    /// the color of pixel x, y, across pixels into the cell
    #[inline]
    fn pixel(&self, across: u32, x: u32, y: u32) -> [u8; 4] {
        let stroke = self.wave.stroke.at(x, y);
        let glow = self.wave.glow.as_ref().map_or(0, |glow| glow.at(x, y));
        let fill = self.filled(across);
        let whole = |share: u32| share == 0 || share == 255;
        if whole(stroke) && whole(glow) && whole(fill) {
            let layers = (glow & 4) | (fill & 2) | (stroke & 1);
            return self
                .palette
                .whole
                .get(layers as usize)
                .copied()
                .unwrap_or([0; 4]);
        }
        let color = over([0; 4], self.palette.glow, glow);
        let color = over(color, self.palette.fill, fill);
        over(color, self.palette.stroke, stroke)
    }
}

/// color laid over under at share out of 255, both premultiplied
#[inline]
fn over(under: [u8; 4], color: [u8; 4], share: u32) -> [u8; 4] {
    if share == 0 {
        return under;
    }
    let part = |channel: u8| (u32::from(channel) * share + 127) / 255;
    let keep = 255 - part(color[3]);
    let mut out = [0; 4];
    for ((out, under), color) in out.iter_mut().zip(under).zip(color) {
        *out = (part(color) + (u32::from(under) * keep + 127) / 255).min(255) as u8;
    }
    out
}

/// the mean of t clamped to 0..1 for t from low to high
fn mean_clamped(low: f32, high: f32) -> f32 {
    // the integral of the clamp from far below up to t
    let area = |t: f32| {
        if t <= 0.0 {
            0.0
        } else if t < 1.0 {
            t * t * 0.5
        } else {
            t - 0.5
        }
    };
    if high - low < 1e-4 {
        low.clamp(0.0, 1.0)
    } else {
        (area(high) - area(low)) / (high - low)
    }
}

#[cfg(test)]
mod tests {
    use super::{mean_clamped, over};

    #[test]
    fn a_layer_goes_over_what_is_under_it() {
        let under = [100, 0, 0, 100];
        assert_eq!(over(under, [0, 200, 0, 200], 0), under);
        // opaque covers, half alpha keeps half of what is under
        assert_eq!(over(under, [0, 255, 0, 255], 255), [0, 255, 0, 255]);
        assert_eq!(over(under, [0, 128, 0, 128], 255), [50, 128, 0, 178]);
        // at half coverage the layer counts half
        assert_eq!(over(under, [0, 255, 0, 255], 128), [50, 128, 0, 178]);
    }

    #[test]
    fn the_mean_of_a_clamp_follows_its_range() {
        assert_eq!(mean_clamped(-3.0, -1.0), 0.0);
        assert_eq!(mean_clamped(2.0, 4.0), 1.0);
        assert_eq!(mean_clamped(0.25, 0.25), 0.25);
        assert!((mean_clamped(0.0, 1.0) - 0.5).abs() < 1e-6);
        // half below 0, half rising to 1
        assert!((mean_clamped(-1.0, 1.0) - 0.25).abs() < 1e-6);
    }
}
