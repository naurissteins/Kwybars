use gtk::cairo::Context;

use super::types::{LinearBarMode, WaveLayout};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WavePoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

pub fn horizontal_wave_points(values: &[f64], layout: WaveLayout) -> Vec<WavePoint> {
    wave_points(
        values,
        layout,
        layout.width,
        layout.height,
        |primary, secondary| WavePoint {
            x: primary,
            y: secondary,
        },
    )
}

pub fn vertical_wave_points(values: &[f64], layout: WaveLayout) -> Vec<WavePoint> {
    wave_points(
        values,
        layout,
        layout.height,
        layout.width,
        |primary, secondary| WavePoint {
            x: secondary,
            y: primary,
        },
    )
}

pub fn append_horizontal_wave_path(
    ctx: &Context,
    values: &[f64],
    layout: WaveLayout,
    origin_x: f64,
    origin_y: f64,
) {
    let points = horizontal_wave_points(values, layout);
    append_curve_path(
        ctx,
        &points,
        WavePoint {
            x: origin_x,
            y: origin_y,
        },
        curve_control_scale(layout.smoothing),
    );
}

pub fn append_vertical_wave_path(
    ctx: &Context,
    values: &[f64],
    layout: WaveLayout,
    origin_x: f64,
    origin_y: f64,
) {
    let points = vertical_wave_points(values, layout);
    append_curve_path(
        ctx,
        &points,
        WavePoint {
            x: origin_x,
            y: origin_y,
        },
        curve_control_scale(layout.smoothing),
    );
}

pub fn append_horizontal_wave_fill_path(
    ctx: &Context,
    values: &[f64],
    layout: WaveLayout,
    origin_x: f64,
    origin_y: f64,
) {
    let points = horizontal_wave_points(values, layout);
    append_fill_path(
        ctx,
        &points,
        WavePoint {
            x: origin_x,
            y: origin_y,
        },
        curve_control_scale(layout.smoothing),
        |first, last| {
            let baseline_y = if layout.from_start {
                layout.edge_padding
            } else {
                layout.height - layout.edge_padding
            };
            [
                WavePoint {
                    x: last.x,
                    y: baseline_y,
                },
                WavePoint {
                    x: first.x,
                    y: baseline_y,
                },
            ]
        },
    );
}

pub fn append_vertical_wave_fill_path(
    ctx: &Context,
    values: &[f64],
    layout: WaveLayout,
    origin_x: f64,
    origin_y: f64,
) {
    let points = vertical_wave_points(values, layout);
    append_fill_path(
        ctx,
        &points,
        WavePoint {
            x: origin_x,
            y: origin_y,
        },
        curve_control_scale(layout.smoothing),
        |first, last| {
            let baseline_x = if layout.from_start {
                layout.edge_padding
            } else {
                layout.width - layout.edge_padding
            };
            [
                WavePoint {
                    x: baseline_x,
                    y: last.y,
                },
                WavePoint {
                    x: baseline_x,
                    y: first.y,
                },
            ]
        },
    );
}

fn wave_points(
    values: &[f64],
    layout: WaveLayout,
    primary_extent: f64,
    secondary_extent: f64,
    make_point: impl Fn(f64, f64) -> WavePoint,
) -> Vec<WavePoint> {
    if values.is_empty() {
        return Vec::new();
    }

    let edge_padding = layout
        .edge_padding
        .max((layout.stroke_width * 0.5).max(0.5));
    let secondary_min = edge_padding;
    let secondary_max = (secondary_extent - edge_padding).max(secondary_min);
    let secondary_span = (secondary_max - secondary_min).max(0.0);

    let center_secondary = secondary_min + (secondary_span * 0.5);
    let mean_value = mean_value(values);
    let primary_positions = wave_positions(values.len(), primary_extent, edge_padding, layout.mode);

    primary_positions
        .into_iter()
        .enumerate()
        .map(|(index, primary)| {
            let centered_offset = wave_centered_offset(values[index], mean_value, layout.amplitude);
            let secondary = if layout.from_start {
                center_secondary + (centered_offset * secondary_span)
            } else {
                center_secondary - (centered_offset * secondary_span)
            };
            make_point(primary, secondary)
        })
        .collect()
}

fn append_curve_path(ctx: &Context, points: &[WavePoint], origin: WavePoint, control_scale: f64) {
    let Some(first) = points.first() else {
        return;
    };

    ctx.new_path();
    ctx.move_to(origin.x + first.x, origin.y + first.y);

    if points.len() == 1 {
        return;
    }

    for index in 0..(points.len() - 1) {
        let p0 = if index == 0 {
            points[index]
        } else {
            points[index - 1]
        };
        let p1 = points[index];
        let p2 = points[index + 1];
        let p3 = if index + 2 < points.len() {
            points[index + 2]
        } else {
            points[index + 1]
        };

        let c1 = WavePoint {
            x: p1.x + ((p2.x - p0.x) * control_scale),
            y: p1.y + ((p2.y - p0.y) * control_scale),
        };
        let c2 = WavePoint {
            x: p2.x - ((p3.x - p1.x) * control_scale),
            y: p2.y - ((p3.y - p1.y) * control_scale),
        };

        ctx.curve_to(
            origin.x + c1.x,
            origin.y + c1.y,
            origin.x + c2.x,
            origin.y + c2.y,
            origin.x + p2.x,
            origin.y + p2.y,
        );
    }
}

fn append_fill_path(
    ctx: &Context,
    points: &[WavePoint],
    origin: WavePoint,
    control_scale: f64,
    baseline_points: impl Fn(WavePoint, WavePoint) -> [WavePoint; 2],
) {
    let Some(first) = points.first().copied() else {
        return;
    };
    let Some(last) = points.last().copied() else {
        return;
    };

    append_curve_path(ctx, points, origin, control_scale);
    let [corner1, corner2] = baseline_points(first, last);
    ctx.line_to(origin.x + corner1.x, origin.y + corner1.y);
    ctx.line_to(origin.x + corner2.x, origin.y + corner2.y);
    ctx.close_path();
}

fn mean_value(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values
        .iter()
        .map(|value| value.clamp(0.0, 1.0))
        .sum::<f64>()
        / values.len() as f64
}

fn wave_centered_offset(value: f64, mean_value: f64, amplitude: f64) -> f64 {
    const WAVE_DEADZONE: f64 = 0.035;
    const WAVE_GAIN: f64 = 1.35;
    const MAX_OFFSET: f64 = 0.46;

    let deviation = (value.clamp(0.0, 1.0) - mean_value).clamp(-1.0, 1.0);
    let magnitude = deviation.abs();

    if magnitude <= WAVE_DEADZONE {
        return 0.0;
    }

    let normalized = ((magnitude - WAVE_DEADZONE) / (1.0 - WAVE_DEADZONE)).clamp(0.0, 1.0);
    let amplitude = amplitude.clamp(0.0, 2.0);
    deviation.signum() * (normalized * WAVE_GAIN * amplitude).clamp(0.0, MAX_OFFSET)
}

fn wave_positions(
    count: usize,
    available_length: f64,
    edge_padding: f64,
    mode: LinearBarMode,
) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }

    let start = edge_padding;
    let end = (available_length - edge_padding).max(start);

    match mode {
        LinearBarMode::Continuous => evenly_spaced_positions(count, start, end),
        LinearBarMode::Split { center_gap } if count >= 2 => {
            let left_count = count / 2;
            let right_count = count - left_count;
            let center_gap = center_gap.max(0.0);
            let usable = (available_length - center_gap).max(0.0);
            let left_end = (usable * 0.5 - edge_padding).max(start);
            let right_start = (available_length - usable * 0.5 + edge_padding).min(end);

            let mut positions = evenly_spaced_positions(left_count, start, left_end);
            positions.extend(evenly_spaced_positions(right_count, right_start, end));
            positions
        }
        LinearBarMode::Split { .. } => evenly_spaced_positions(count, start, end),
    }
}

fn evenly_spaced_positions(count: usize, start: f64, end: f64) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![(start + end) * 0.5];
    }

    let step = (end - start) / (count.saturating_sub(1) as f64);
    (0..count)
        .map(|index| start + (step * index as f64))
        .collect()
}

pub(crate) fn curve_control_scale(smoothing: f64) -> f64 {
    smoothing.clamp(0.0, 2.0) / 6.0
}
