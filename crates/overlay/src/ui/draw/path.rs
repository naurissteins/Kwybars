use std::f64::consts::{FRAC_PI_2, PI, TAU};

use super::types::{BarOrientation, BarRect, BarStyle, DirectedBarSpec, RadialBarSpec};

pub fn append_radial_bar_path(
    ctx: &gtk::cairo::Context,
    center_x: f64,
    center_y: f64,
    spec: RadialBarSpec,
    style: BarStyle,
) {
    ctx.save().ok();
    ctx.translate(center_x, center_y);
    ctx.rotate(spec.angle);
    append_bar_path(
        ctx,
        BarRect {
            x: spec.inner_radius,
            y: -spec.thickness * 0.5,
            width: spec.length,
            height: spec.thickness,
        },
        style,
        BarOrientation::Vertical,
        true,
    );
    ctx.restore().ok();
}

pub fn append_directed_bar_path(
    ctx: &gtk::cairo::Context,
    center_x: f64,
    center_y: f64,
    spec: DirectedBarSpec,
    style: BarStyle,
) {
    ctx.save().ok();
    ctx.translate(center_x + spec.x, center_y + spec.y);
    ctx.rotate(spec.angle);
    append_bar_path(
        ctx,
        BarRect {
            x: 0.0,
            y: -spec.thickness * 0.5,
            width: spec.length,
            height: spec.thickness,
        },
        style,
        BarOrientation::Vertical,
        true,
    );
    ctx.restore().ok();
}

pub fn append_bar_path(
    ctx: &gtk::cairo::Context,
    rect: BarRect,
    style: BarStyle,
    orientation: BarOrientation,
    forward: bool,
) {
    if style.segmented {
        let segment_length = style.segment_length.max(1.0);
        let segment_gap = style.segment_gap.max(0.0);

        match orientation {
            BarOrientation::Horizontal => {
                for_each_segment_span(
                    rect.height,
                    segment_length,
                    segment_gap,
                    forward,
                    |offset, len| {
                        append_rounded_rect_path(
                            ctx,
                            rect.x,
                            rect.y + offset,
                            rect.width,
                            len,
                            style.corner_radius,
                        );
                    },
                );
            }
            BarOrientation::Vertical => {
                for_each_segment_span(
                    rect.width,
                    segment_length,
                    segment_gap,
                    forward,
                    |offset, len| {
                        append_rounded_rect_path(
                            ctx,
                            rect.x + offset,
                            rect.y,
                            len,
                            rect.height,
                            style.corner_radius,
                        );
                    },
                );
            }
        }
        return;
    }

    append_rounded_rect_path(
        ctx,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        style.corner_radius,
    );
}

fn append_rounded_rect_path(
    ctx: &gtk::cairo::Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    corner_radius: f64,
) {
    let radius = corner_radius.max(0.0).min(width * 0.5).min(height * 0.5);
    if radius <= 0.0 {
        ctx.rectangle(x, y, width, height);
        return;
    }

    ctx.new_sub_path();
    ctx.move_to(x + radius, y);
    ctx.line_to(x + width - radius, y);
    ctx.arc(x + width - radius, y + radius, radius, -FRAC_PI_2, 0.0);
    ctx.line_to(x + width, y + height - radius);
    ctx.arc(
        x + width - radius,
        y + height - radius,
        radius,
        0.0,
        FRAC_PI_2,
    );
    ctx.line_to(x + radius, y + height);
    ctx.arc(x + radius, y + height - radius, radius, FRAC_PI_2, PI);
    ctx.line_to(x, y + radius);
    ctx.arc(x + radius, y + radius, radius, PI, PI + FRAC_PI_2);
    ctx.close_path();
}

pub(crate) fn for_each_segment_span(
    total_length: f64,
    segment_length: f64,
    segment_gap: f64,
    from_start: bool,
    mut segment: impl FnMut(f64, f64),
) {
    let total_length = total_length.max(0.0);
    if total_length <= 0.0 {
        return;
    }

    let segment_length = segment_length.max(1.0);
    let segment_gap = segment_gap.max(0.0);
    let step = segment_length + segment_gap;

    if from_start {
        let mut cursor = 0.0;
        while cursor < total_length {
            let length = (total_length - cursor).min(segment_length);
            if length <= 0.0 {
                break;
            }
            segment(cursor, length);
            cursor += step;
        }
        return;
    }

    let mut cursor = total_length;
    while cursor > 0.0 {
        let start = (cursor - segment_length).max(0.0);
        let length = cursor - start;
        if length <= 0.0 {
            break;
        }
        segment(start, length);
        if start <= 0.0 {
            break;
        }
        cursor = (start - segment_gap).max(0.0);
    }
}

pub(crate) fn centered_layout_outer_radius(width: f64, height: f64, style: BarStyle) -> f64 {
    let min_half_extent = (width * 0.5).min(height * 0.5);
    let padding = style.thickness.max(2.0) + style.gap.max(0.0);
    (min_half_extent - padding).max(10.0)
}

pub fn draw_particle(ctx: &gtk::cairo::Context, spec: super::types::ParticleSpec) {
    ctx.new_sub_path();
    ctx.arc(spec.x, spec.y, spec.radius, 0.0, TAU);
}
