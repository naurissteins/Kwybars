#[cfg(test)]
mod tests;

use crate::config::{Edge, LineMode, SurfaceConfig};

const PADDING: f32 = 6.0;
const DEADZONE: f32 = 0.035;
const GAIN: f32 = 1.35;
const MAX_OFFSET: f32 = 0.46;

/// where the curve's points go, in buffer pixels
#[derive(Debug, Clone, PartialEq)]
pub struct WaveShape {
    pub along_x: bool,
    pub from_start: bool,
    positions: Vec<f32>,
    center: f32,
    span: f32,
    amplitude: f32,
    pub control: f32,
    pub stroke: f32,
    pub glow: Option<f32>,
    pub baseline: Option<f32>,
}

impl WaveShape {
    pub fn new(config: &SurfaceConfig, size: (u32, u32), scale: f32, bars: usize) -> Self {
        let visualizer = &config.visualizer;
        let edge = config.overlay.position;
        let along_x = matches!(edge, Edge::Bottom | Edge::Top);
        let from_start = matches!(edge, Edge::Top | Edge::Left);
        let (length, depth) = if along_x {
            (size.0 as f32, size.1 as f32)
        } else {
            (size.1 as f32, size.0 as f32)
        };
        let stroke = visualizer.wave_stroke_width.max(1) as f32;
        let glow = (stroke * 3.0).max(stroke + 2.0);
        let widest = if visualizer.wave_glow { glow } else { stroke };
        let padding = (widest * 0.5 + PADDING) * scale;
        let far = (depth - padding).max(padding);
        let split = match visualizer.line_mode {
            LineMode::Split if bars >= 2 => Some(visualizer.line_split_gap as f32 * scale),
            _ => None,
        };
        Self {
            along_x,
            from_start,
            positions: positions(bars, length, padding, split),
            center: (padding + far) * 0.5,
            span: far - padding,
            amplitude: visualizer.wave_amplitude.clamp(0.0, 2.0),
            control: visualizer.wave_smoothing.clamp(0.0, 2.0) / 6.0,
            stroke: stroke * scale,
            glow: visualizer.wave_glow.then_some(glow * scale),
            baseline: visualizer
                .wave_fill
                .then_some(if from_start { padding } else { far }),
        }
    }

    /// the point of every value as x, y, replacing what out held
    pub fn points(&self, values: &[f32], out: &mut Vec<(f32, f32)>) {
        out.clear();
        let count = values.len().max(1) as f32;
        let mean = values
            .iter()
            .map(|value| value.clamp(0.0, 1.0))
            .sum::<f32>()
            / count;
        for (value, along) in values.iter().zip(&self.positions) {
            let offset = offset(*value, mean, self.amplitude) * self.span;
            // louder than average moves away from the edge
            let across = if self.from_start {
                self.center + offset
            } else {
                self.center - offset
            };
            out.push(if self.along_x {
                (*along, across)
            } else {
                (across, *along)
            });
        }
    }
}

/// how far a value sits from the middle, as a share of the span
fn offset(value: f32, mean: f32, amplitude: f32) -> f32 {
    let deviation = (value.clamp(0.0, 1.0) - mean).clamp(-1.0, 1.0);
    let magnitude = deviation.abs();
    if magnitude <= DEADZONE {
        return 0.0;
    }
    let normalized = ((magnitude - DEADZONE) / (1.0 - DEADZONE)).clamp(0.0, 1.0);
    deviation.signum() * (normalized * GAIN * amplitude).clamp(0.0, MAX_OFFSET)
}

/// points spread along length, padding in from both ends; split leaves a
/// gap of that size in the middle
fn positions(count: usize, length: f32, padding: f32, split: Option<f32>) -> Vec<f32> {
    let (start, end) = (padding, (length - padding).max(padding));
    let mut out = Vec::with_capacity(count);
    let Some(gap) = split else {
        spread(count, start, end, &mut out);
        return out;
    };
    let usable = (length - gap.max(0.0)).max(0.0);
    let left = count / 2;
    spread(left, start, (usable * 0.5 - padding).max(start), &mut out);
    let right_start = (length - usable * 0.5 + padding).min(end);
    spread(count - left, right_start, end, &mut out);
    out
}

fn spread(count: usize, start: f32, end: f32, out: &mut Vec<f32>) {
    match count {
        0 => {}
        1 => out.push((start + end) * 0.5),
        _ => {
            let step = (end - start) / (count - 1) as f32;
            out.extend((0..count).map(|index| start + step * index as f32));
        }
    }
}
