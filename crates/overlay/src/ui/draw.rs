use std::f64::consts::{FRAC_PI_2, PI};

#[derive(Clone, Copy)]
pub struct BarStyle {
    pub thickness: f64,
    pub gap: f64,
    pub corner_radius: f64,
    pub segmented: bool,
    pub segment_length: f64,
    pub segment_gap: f64,
}

#[derive(Clone, Copy)]
pub enum BarOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
pub struct BarRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy)]
pub struct RadialBarSpec {
    pub angle: f64,
    pub inner_radius: f64,
    pub length: f64,
    pub thickness: f64,
}

pub fn for_each_horizontal_bar(
    values: &[f64],
    width: f64,
    height: f64,
    bar_thickness: f64,
    gap: f64,
    from_top: bool,
    mut paint: impl FnMut(usize, f64, f64, f64, f64),
) {
    let count = values.len() as f64;
    let total_nominal = (count * bar_thickness) + ((count - 1.0).max(0.0) * gap);
    let scale = if total_nominal > width {
        width / total_nominal
    } else {
        1.0
    };

    let bar_width = (bar_thickness * scale).max(1.0);
    let gap_width = gap * scale;
    let rendered_total = (count * bar_width) + ((count - 1.0).max(0.0) * gap_width);
    let start_x = (width - rendered_total).max(0.0) * 0.5;

    for (index, value) in values.iter().enumerate() {
        let bar_height = (height * value.clamp(0.0, 1.0)).max(2.0);
        let x = start_x + (index as f64 * (bar_width + gap_width));
        let y = if from_top { 0.0 } else { height - bar_height };
        paint(index, x, y, bar_width, bar_height);
    }
}

pub fn for_each_vertical_bar(
    values: &[f64],
    width: f64,
    height: f64,
    bar_thickness: f64,
    gap: f64,
    from_left: bool,
    mut paint: impl FnMut(usize, f64, f64, f64, f64),
) {
    let count = values.len() as f64;
    let total_nominal = (count * bar_thickness) + ((count - 1.0).max(0.0) * gap);
    let scale = if total_nominal > height {
        height / total_nominal
    } else {
        1.0
    };

    let bar_height = (bar_thickness * scale).max(1.0);
    let gap_height = gap * scale;
    let rendered_total = (count * bar_height) + ((count - 1.0).max(0.0) * gap_height);
    let start_y = (height - rendered_total).max(0.0) * 0.5;

    for (index, value) in values.iter().enumerate() {
        let bar_width = (width * value.clamp(0.0, 1.0)).max(2.0);
        let x = if from_left { 0.0 } else { width - bar_width };
        let y = start_y + (index as f64 * (bar_height + gap_height));
        paint(index, x, y, bar_width, bar_height);
    }
}

pub fn draw_horizontal_bars(
    ctx: &gtk::cairo::Context,
    values: &[f64],
    width: f64,
    height: f64,
    style: BarStyle,
    from_top: bool,
) {
    for_each_horizontal_bar(
        values,
        width,
        height,
        style.thickness,
        style.gap,
        from_top,
        |_, x, y, bar_width, bar_height| {
            append_bar_path(
                ctx,
                BarRect {
                    x,
                    y,
                    width: bar_width,
                    height: bar_height,
                },
                style,
                BarOrientation::Horizontal,
                from_top,
            );
        },
    );
}

pub fn draw_vertical_bars(
    ctx: &gtk::cairo::Context,
    values: &[f64],
    width: f64,
    height: f64,
    style: BarStyle,
    from_left: bool,
) {
    for_each_vertical_bar(
        values,
        width,
        height,
        style.thickness,
        style.gap,
        from_left,
        |_, x, y, bar_width, bar_height| {
            append_bar_path(
                ctx,
                BarRect {
                    x,
                    y,
                    width: bar_width,
                    height: bar_height,
                },
                style,
                BarOrientation::Vertical,
                from_left,
            );
        },
    );
}

pub fn for_each_radial_bar(
    values: &[f64],
    width: f64,
    height: f64,
    inner_radius: f64,
    style: BarStyle,
    mut paint: impl FnMut(usize, RadialBarSpec),
) {
    if values.is_empty() || width <= 0.0 || height <= 0.0 {
        return;
    }

    let min_half_extent = (width * 0.5).min(height * 0.5);
    let padding = style.thickness.max(2.0) + style.gap.max(0.0);
    let max_outer_radius = (min_half_extent - padding).max(10.0);
    let inner_radius = inner_radius
        .max(10.0)
        .min((max_outer_radius - 10.0).max(10.0));
    let max_length = (max_outer_radius - inner_radius).max(6.0);

    let count = values.len() as f64;
    let circumference = 2.0 * PI * inner_radius.max(1.0);
    let total_nominal = count * (style.thickness + style.gap.max(0.0));
    let scale = if total_nominal > circumference {
        circumference / total_nominal
    } else {
        1.0
    };

    let tangential_thickness = (style.thickness * scale).max(1.0);
    let gap = style.gap.max(0.0) * scale;
    let angle_step = (tangential_thickness + gap) / inner_radius.max(1.0);
    let start_angle = -FRAC_PI_2;
    for (index, value) in values.iter().enumerate() {
        let length = (value.clamp(0.0, 1.0) * max_length).max(2.0);
        let angle = start_angle + (index as f64 * angle_step);
        paint(
            index,
            RadialBarSpec {
                angle,
                inner_radius,
                length,
                thickness: tangential_thickness,
            },
        );
    }
}

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

fn for_each_segment_span(
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

pub fn bar_color_index(bar_index: usize, bar_count: usize, color_count: usize) -> usize {
    if bar_count == 0 || color_count == 0 {
        return 0;
    }
    let idx = bar_index.saturating_mul(color_count) / bar_count;
    idx.min(color_count - 1)
}

#[cfg(test)]
mod tests {
    use super::{bar_color_index, for_each_segment_span};

    #[test]
    fn spreads_colors_evenly() {
        let indices: Vec<usize> = (0..12).map(|index| bar_color_index(index, 12, 6)).collect();
        assert_eq!(indices, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
    }

    #[test]
    fn segments_from_start() {
        let mut spans = Vec::new();
        for_each_segment_span(10.0, 3.0, 1.0, true, |start, len| spans.push((start, len)));
        assert_eq!(spans, vec![(0.0, 3.0), (4.0, 3.0), (8.0, 2.0)]);
    }

    #[test]
    fn segments_from_end() {
        let mut spans = Vec::new();
        for_each_segment_span(10.0, 3.0, 1.0, false, |start, len| spans.push((start, len)));
        assert_eq!(spans, vec![(7.0, 3.0), (3.0, 3.0), (0.0, 2.0)]);
    }
}
