use std::f64::consts::{FRAC_PI_2, PI, TAU};

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

#[derive(Clone, Copy)]
pub struct RadialLayout {
    pub width: f64,
    pub height: f64,
    pub inner_radius: f64,
    pub start_angle: f64,
    pub arc_radians: f64,
}

#[derive(Clone, Copy)]
pub struct PolygonLayout {
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub rotation_radians: f64,
    pub sides: usize,
}

#[derive(Clone, Copy)]
pub struct DirectedBarSpec {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub length: f64,
    pub thickness: f64,
}

#[derive(Clone, Copy)]
pub struct FrameEdgeRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub orientation: BarOrientation,
    pub from_start: bool,
}

#[derive(Clone, Copy, Debug)]
struct RadialDistribution {
    first_angle: f64,
    angle_step: f64,
    tangential_thickness: f64,
}

#[derive(Clone, Copy)]
pub struct ParticleSpec {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

#[derive(Clone, Copy)]
pub struct FloatingParticleLayout {
    pub width: f64,
    pub height: f64,
    pub max_radius: f64,
    pub gap: f64,
    pub orientation: BarOrientation,
    pub from_start: bool,
}

#[derive(Clone, Copy)]
pub enum LinearBarMode {
    Continuous,
    Split { center_gap: f64 },
}

#[derive(Clone, Copy)]
pub struct HorizontalBarLayout {
    pub width: f64,
    pub height: f64,
    pub bar_thickness: f64,
    pub gap: f64,
    pub from_top: bool,
    pub mode: LinearBarMode,
}

#[derive(Clone, Copy)]
pub struct VerticalBarLayout {
    pub width: f64,
    pub height: f64,
    pub bar_thickness: f64,
    pub gap: f64,
    pub from_left: bool,
    pub mode: LinearBarMode,
}

#[derive(Clone, Copy)]
pub struct Point {
    x: f64,
    y: f64,
}

pub fn for_each_particle(
    values: &[f64],
    width: f64,
    height: f64,
    max_radius: f64,
    gap: f64,
    orientation: BarOrientation,
    mut paint: impl FnMut(usize, ParticleSpec),
) {
    let count = values.len() as f64;
    let total_nominal = (count * (max_radius * 2.0)) + ((count - 1.0).max(0.0) * gap);

    match orientation {
        BarOrientation::Horizontal => {
            let mut scale = if total_nominal > width {
                width / total_nominal
            } else {
                1.0
            };

            // Ensure diameter doesn't exceed height
            let max_diameter = max_radius * 2.0;
            if max_diameter * scale > height {
                scale = (height / max_diameter).min(scale);
            }

            let particle_diameter = (max_diameter * scale).max(2.0);
            let gap_width = gap * scale;
            let rendered_total = (count * particle_diameter) + ((count - 1.0).max(0.0) * gap_width);
            let start_x = (width - rendered_total).max(0.0) * 0.5;

            for (index, value) in values.iter().enumerate() {
                let x = start_x
                    + (index as f64 * (particle_diameter + gap_width))
                    + (particle_diameter * 0.5);
                let y = height * 0.5;
                let radius = (particle_diameter * 0.5 * value.clamp(0.01, 1.0)).max(1.0);
                paint(index, ParticleSpec { x, y, radius });
            }
        }
        BarOrientation::Vertical => {
            let mut scale = if total_nominal > height {
                height / total_nominal
            } else {
                1.0
            };

            // Ensure diameter doesn't exceed width
            let max_diameter = max_radius * 2.0;
            if max_diameter * scale > width {
                scale = (width / max_diameter).min(scale);
            }

            let particle_diameter = (max_diameter * scale).max(2.0);
            let gap_height = gap * scale;
            let rendered_total =
                (count * particle_diameter) + ((count - 1.0).max(0.0) * gap_height);
            let start_y = (height - rendered_total).max(0.0) * 0.5;

            for (index, value) in values.iter().enumerate() {
                let x = width * 0.5;
                let y = start_y
                    + (index as f64 * (particle_diameter + gap_height))
                    + (particle_diameter * 0.5);
                let radius = (particle_diameter * 0.5 * value.clamp(0.01, 1.0)).max(1.0);
                paint(index, ParticleSpec { x, y, radius });
            }
        }
    }
}

pub fn for_each_floating_particle(
    values: &[f64],
    offsets: &[f64],
    layout: FloatingParticleLayout,
    mut paint: impl FnMut(usize, ParticleSpec),
) {
    let count = values.len() as f64;
    let total_nominal = (count * (layout.max_radius * 2.0)) + ((count - 1.0).max(0.0) * layout.gap);
    let offsets_len = offsets.len();

    match layout.orientation {
        BarOrientation::Horizontal => {
            let mut scale = if total_nominal > layout.width {
                layout.width / total_nominal
            } else {
                1.0
            };

            // Cap diameter by height to avoid clipping if travel range is small
            let max_diameter = layout.max_radius * 2.0;
            if max_diameter * scale > layout.height * 0.5 {
                scale = (layout.height * 0.5 / max_diameter).min(scale);
            }

            let particle_diameter = (max_diameter * scale).max(2.0);
            let gap_width = layout.gap * scale;
            let rendered_total = (count * particle_diameter) + ((count - 1.0).max(0.0) * gap_width);
            let start_x = (layout.width - rendered_total).max(0.0) * 0.5;

            let max_r = particle_diameter * 0.5;
            let travel = (layout.height - 2.0 * max_r).max(0.0);

            for (index, value) in values.iter().enumerate() {
                let x = start_x
                    + (index as f64 * (particle_diameter + gap_width))
                    + (particle_diameter * 0.5);
                let offset = offsets
                    .get(index % offsets_len.max(1))
                    .copied()
                    .unwrap_or(0.0);
                let y = if layout.from_start {
                    max_r + (offset * travel)
                } else {
                    (layout.height - max_r) - (offset * travel)
                };
                let radius = (particle_diameter * 0.5 * value.clamp(0.01, 1.0)).max(1.0);
                paint(index, ParticleSpec { x, y, radius });
            }
        }
        BarOrientation::Vertical => {
            let mut scale = if total_nominal > layout.height {
                layout.height / total_nominal
            } else {
                1.0
            };

            // Cap diameter by width to avoid clipping if travel range is small
            let max_diameter = layout.max_radius * 2.0;
            if max_diameter * scale > layout.width * 0.5 {
                scale = (layout.width * 0.5 / max_diameter).min(scale);
            }

            let particle_diameter = (max_diameter * scale).max(2.0);
            let gap_height = layout.gap * scale;
            let rendered_total =
                (count * particle_diameter) + ((count - 1.0).max(0.0) * gap_height);
            let start_y = (layout.height - rendered_total).max(0.0) * 0.5;

            let max_r = particle_diameter * 0.5;
            let travel = (layout.width - 2.0 * max_r).max(0.0);

            for (index, value) in values.iter().enumerate() {
                let offset = offsets
                    .get(index % offsets_len.max(1))
                    .copied()
                    .unwrap_or(0.0);
                let x = if layout.from_start {
                    max_r + (offset * travel)
                } else {
                    (layout.width - max_r) - (offset * travel)
                };
                let y = start_y
                    + (index as f64 * (particle_diameter + gap_height))
                    + (particle_diameter * 0.5);
                let radius = (particle_diameter * 0.5 * value.clamp(0.01, 1.0)).max(1.0);
                paint(index, ParticleSpec { x, y, radius });
            }
        }
    }
}

pub fn draw_particle(ctx: &gtk::cairo::Context, spec: ParticleSpec) {
    ctx.new_sub_path();
    ctx.arc(spec.x, spec.y, spec.radius, 0.0, TAU);
}

pub fn distributed_chunk<T>(values: &[T], group_index: usize, group_count: usize) -> &[T] {
    if values.is_empty() || group_count == 0 {
        return &[];
    }

    let start = values.len() * group_index / group_count;
    let end = values.len() * (group_index + 1) / group_count;
    &values[start..end]
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

pub fn for_each_horizontal_bar_mode(
    values: &[f64],
    layout: HorizontalBarLayout,
    mut paint: impl FnMut(usize, f64, f64, f64, f64),
) {
    for_each_linear_slot(
        values.len(),
        layout.width,
        layout.bar_thickness,
        layout.gap,
        layout.mode,
        |index, x, bar_width| {
            let value = values[index];
            let bar_height = (layout.height * value.clamp(0.0, 1.0)).max(2.0);
            let y = if layout.from_top {
                0.0
            } else {
                layout.height - bar_height
            };
            paint(index, x, y, bar_width, bar_height);
        },
    );
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

pub fn for_each_vertical_bar_mode(
    values: &[f64],
    layout: VerticalBarLayout,
    mut paint: impl FnMut(usize, f64, f64, f64, f64),
) {
    for_each_linear_slot(
        values.len(),
        layout.height,
        layout.bar_thickness,
        layout.gap,
        layout.mode,
        |index, y, bar_height| {
            let value = values[index];
            let bar_width = (layout.width * value.clamp(0.0, 1.0)).max(2.0);
            let x = if layout.from_left {
                0.0
            } else {
                layout.width - bar_width
            };
            paint(index, x, y, bar_width, bar_height);
        },
    );
}

pub fn draw_horizontal_bars_mode(
    ctx: &gtk::cairo::Context,
    values: &[f64],
    layout: HorizontalBarLayout,
    style: BarStyle,
) {
    for_each_horizontal_bar_mode(values, layout, |_, x, y, bar_width, bar_height| {
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
            layout.from_top,
        );
    });
}

pub fn draw_vertical_bars_mode(
    ctx: &gtk::cairo::Context,
    values: &[f64],
    layout: VerticalBarLayout,
    style: BarStyle,
) {
    for_each_vertical_bar_mode(values, layout, |_, x, y, bar_width, bar_height| {
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
            layout.from_left,
        );
    });
}

pub fn for_each_radial_bar(
    values: &[f64],
    layout: RadialLayout,
    style: BarStyle,
    mut paint: impl FnMut(usize, RadialBarSpec),
) {
    if values.is_empty() || layout.width <= 0.0 || layout.height <= 0.0 {
        return;
    }

    let min_half_extent = (layout.width * 0.5).min(layout.height * 0.5);
    let padding = style.thickness.max(2.0) + style.gap.max(0.0);
    let max_outer_radius = (min_half_extent - padding).max(10.0);
    let inner_radius = layout
        .inner_radius
        .max(10.0)
        .min((max_outer_radius - 10.0).max(10.0));
    let max_length = (max_outer_radius - inner_radius).max(6.0);

    let Some(distribution) = radial_distribution(
        values.len(),
        inner_radius,
        style.thickness,
        style.gap,
        layout.start_angle,
        layout.arc_radians,
    ) else {
        return;
    };

    for (index, value) in values.iter().enumerate() {
        let length = (value.clamp(0.0, 1.0) * max_length).max(2.0);
        let angle = distribution.first_angle + (index as f64 * distribution.angle_step);
        paint(
            index,
            RadialBarSpec {
                angle,
                inner_radius,
                length,
                thickness: distribution.tangential_thickness,
            },
        );
    }
}

pub fn for_each_polygon_bar(
    values: &[f64],
    layout: PolygonLayout,
    style: BarStyle,
    mut paint: impl FnMut(usize, DirectedBarSpec),
) {
    if values.is_empty() || layout.width <= 0.0 || layout.height <= 0.0 || layout.sides < 3 {
        return;
    }

    let max_outer_radius = centered_layout_outer_radius(layout.width, layout.height, style);
    let radius = layout
        .radius
        .max(10.0)
        .min((max_outer_radius - 10.0).max(10.0));
    let apothem = radius * (PI / layout.sides as f64).cos();
    let max_length = (max_outer_radius - apothem).max(6.0);
    let vertices = regular_polygon_vertices(layout.sides, radius, layout.rotation_radians);
    let edge_length = polygon_edge_length(&vertices);
    if edge_length <= 0.0 {
        return;
    }

    let perimeter = edge_length * layout.sides as f64;
    let gap_count = if values.len() <= 1 { 0 } else { values.len() } as f64;
    let total_nominal =
        (values.len() as f64 * style.thickness.max(1.0)) + (gap_count * style.gap.max(0.0));
    let scale = if total_nominal > perimeter {
        perimeter / total_nominal
    } else {
        1.0
    };

    let tangential_thickness = (style.thickness * scale).max(1.0);
    let base_gap = style.gap.max(0.0) * scale;
    let occupied_length = (values.len() as f64 * tangential_thickness) + (gap_count * base_gap);
    let extra_gap = if gap_count > 0.0 {
        (perimeter - occupied_length).max(0.0) / gap_count
    } else {
        0.0
    };
    let step_distance = tangential_thickness + base_gap + extra_gap;

    for (index, value) in values.iter().enumerate() {
        let center_distance = (tangential_thickness * 0.5) + (index as f64 * step_distance);
        let (point, normal) = polygon_point_and_normal(&vertices, center_distance % perimeter);
        let length = (value.clamp(0.0, 1.0) * max_length).max(2.0);
        paint(
            index,
            DirectedBarSpec {
                x: point.x,
                y: point.y,
                angle: normal.y.atan2(normal.x),
                length,
                thickness: tangential_thickness,
            },
        );
    }
}

fn radial_distribution(
    count: usize,
    inner_radius: f64,
    thickness: f64,
    gap: f64,
    start_angle: f64,
    arc_radians: f64,
) -> Option<RadialDistribution> {
    if count == 0 {
        return None;
    }

    let inner_radius = inner_radius.max(1.0);
    let clamped_arc = arc_radians.clamp(-TAU, TAU);
    let direction = if clamped_arc < 0.0 { -1.0 } else { 1.0 };
    let arc_magnitude = clamped_arc.abs().max(0.001);
    let full_circle = (arc_magnitude - TAU).abs() < 0.001;

    let gap_count = if count <= 1 {
        0
    } else if full_circle {
        count
    } else {
        count.saturating_sub(1)
    } as f64;
    let total_nominal = (count as f64 * thickness.max(1.0)) + (gap_count * gap.max(0.0));
    let available_arc_length = arc_magnitude * inner_radius;
    let scale = if total_nominal > available_arc_length {
        available_arc_length / total_nominal
    } else {
        1.0
    };

    let tangential_thickness = (thickness * scale).max(1.0);
    let base_gap = gap.max(0.0) * scale;
    let occupied_length = (count as f64 * tangential_thickness) + (gap_count * base_gap);
    let extra_gap = if gap_count > 0.0 {
        (available_arc_length - occupied_length).max(0.0) / gap_count
    } else {
        0.0
    };
    let effective_gap = base_gap + extra_gap;
    let angle_step = if count <= 1 {
        0.0
    } else {
        direction * (tangential_thickness + effective_gap) / inner_radius
    };
    let first_angle = if full_circle {
        start_angle
    } else if count == 1 {
        start_angle + (clamped_arc * 0.5)
    } else {
        start_angle + (direction * tangential_thickness * 0.5 / inner_radius)
    };

    Some(RadialDistribution {
        first_angle,
        angle_step,
        tangential_thickness,
    })
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

fn centered_layout_outer_radius(width: f64, height: f64, style: BarStyle) -> f64 {
    let min_half_extent = (width * 0.5).min(height * 0.5);
    let padding = style.thickness.max(2.0) + style.gap.max(0.0);
    (min_half_extent - padding).max(10.0)
}

fn for_each_linear_slot(
    count: usize,
    available_length: f64,
    item_thickness: f64,
    gap: f64,
    mode: LinearBarMode,
    mut paint: impl FnMut(usize, f64, f64),
) {
    if count == 0 {
        return;
    }

    match mode {
        LinearBarMode::Continuous => {
            let count = count as f64;
            let total_nominal = (count * item_thickness) + ((count - 1.0).max(0.0) * gap);
            let scale = if total_nominal > available_length {
                available_length / total_nominal
            } else {
                1.0
            };
            let item_size = (item_thickness * scale).max(1.0);
            let gap_size = gap * scale;
            let rendered_total = (count * item_size) + ((count - 1.0).max(0.0) * gap_size);
            let start = (available_length - rendered_total).max(0.0) * 0.5;

            for index in 0..count as usize {
                let position = start + (index as f64 * (item_size + gap_size));
                paint(index, position, item_size);
            }
        }
        LinearBarMode::Split { center_gap } if count >= 2 => {
            let left_count = count / 2;
            let right_count = count - left_count;
            let left_gaps = left_count.saturating_sub(1) as f64;
            let right_gaps = right_count.saturating_sub(1) as f64;
            let total_nominal = (count as f64 * item_thickness)
                + ((left_gaps + right_gaps) * gap)
                + center_gap.max(0.0);
            let scale = if total_nominal > available_length {
                available_length / total_nominal
            } else {
                1.0
            };
            let item_size = (item_thickness * scale).max(1.0);
            let gap_size = gap * scale;
            let center_gap = center_gap.max(0.0) * scale;
            let left_rendered = (left_count as f64 * item_size) + (left_gaps * gap_size);
            let right_rendered = (right_count as f64 * item_size) + (right_gaps * gap_size);
            let rendered_total = left_rendered + center_gap + right_rendered;
            let start = (available_length - rendered_total).max(0.0) * 0.5;

            for index in 0..left_count {
                let position = start + (index as f64 * (item_size + gap_size));
                paint(index, position, item_size);
            }

            let right_start = start + left_rendered + center_gap;
            for offset in 0..right_count {
                let index = left_count + offset;
                let position = right_start + (offset as f64 * (item_size + gap_size));
                paint(index, position, item_size);
            }
        }
        LinearBarMode::Split { .. } => {
            for_each_linear_slot(
                count,
                available_length,
                item_thickness,
                gap,
                LinearBarMode::Continuous,
                paint,
            );
        }
    }
}

fn regular_polygon_vertices(sides: usize, radius: f64, rotation_radians: f64) -> Vec<Point> {
    (0..sides)
        .map(|index| {
            let angle = rotation_radians + (index as f64 * TAU / sides as f64);
            Point {
                x: radius * angle.cos(),
                y: radius * angle.sin(),
            }
        })
        .collect()
}

fn polygon_edge_length(vertices: &[Point]) -> f64 {
    if vertices.len() < 2 {
        return 0.0;
    }

    let first = vertices[0];
    let second = vertices[1];
    point_distance(first, second)
}

fn polygon_point_and_normal(vertices: &[Point], distance: f64) -> (Point, Point) {
    let edge_length = polygon_edge_length(vertices).max(1.0);
    let edge_index = ((distance / edge_length).floor() as usize) % vertices.len();
    let edge_start = vertices[edge_index];
    let edge_end = vertices[(edge_index + 1) % vertices.len()];
    let along = (distance % edge_length) / edge_length;
    let point = Point {
        x: edge_start.x + ((edge_end.x - edge_start.x) * along),
        y: edge_start.y + ((edge_end.y - edge_start.y) * along),
    };
    let midpoint = Point {
        x: (edge_start.x + edge_end.x) * 0.5,
        y: (edge_start.y + edge_end.y) * 0.5,
    };
    let normal = normalize_point(midpoint);
    (point, normal)
}

fn normalize_point(point: Point) -> Point {
    let length = (point.x.powi(2) + point.y.powi(2)).sqrt();
    if length <= f64::EPSILON {
        return Point { x: 1.0, y: 0.0 };
    }

    Point {
        x: point.x / length,
        y: point.y / length,
    }
}

fn point_distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
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
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    use super::{
        BarStyle, HorizontalBarLayout, LinearBarMode, PolygonLayout, VerticalBarLayout,
        bar_color_index, for_each_horizontal_bar_mode, for_each_polygon_bar, for_each_segment_span,
        for_each_vertical_bar_mode, radial_distribution,
    };

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

    #[test]
    fn radial_partial_arc_stays_inside_requested_span() {
        let Some(distribution) = radial_distribution(5, 100.0, 8.0, 12.0, -PI, PI) else {
            panic!("expected radial distribution");
        };
        let first_center = distribution.first_angle;
        let last_center = distribution.first_angle + (4.0 * distribution.angle_step);
        let half_bar_angle = distribution.tangential_thickness * 0.5 / 100.0;

        assert!((first_center - (-PI + half_bar_angle)).abs() < 1e-6);
        assert!((last_center - (0.0 - half_bar_angle)).abs() < 1e-6);
    }

    #[test]
    fn radial_single_bar_centers_inside_partial_arc() {
        let Some(distribution) = radial_distribution(1, 120.0, 8.0, 12.0, -FRAC_PI_2, PI) else {
            panic!("expected radial distribution");
        };
        assert!((distribution.first_angle - 0.0).abs() < 1e-6);
        assert_eq!(distribution.angle_step, 0.0);
    }

    #[test]
    fn radial_full_circle_clamps_oversized_arc() {
        let Some(distribution) = radial_distribution(4, 100.0, 8.0, 12.0, -FRAC_PI_2, TAU * 2.0)
        else {
            panic!("expected radial distribution");
        };
        let expected_step = TAU / 4.0;

        assert!((distribution.first_angle - (-FRAC_PI_2)).abs() < 1e-6);
        assert!((distribution.angle_step - expected_step).abs() < 1e-6);
    }

    #[test]
    fn polygon_layout_distributes_bars_across_multiple_edges() {
        let mut angles = Vec::new();
        for_each_polygon_bar(
            &[1.0, 1.0, 1.0],
            PolygonLayout {
                width: 800.0,
                height: 800.0,
                radius: 180.0,
                rotation_radians: -FRAC_PI_2,
                sides: 3,
            },
            BarStyle {
                thickness: 8.0,
                gap: 0.0,
                corner_radius: 0.0,
                segmented: false,
                segment_length: 12.0,
                segment_gap: 6.0,
            },
            |_, spec| angles.push((spec.angle.to_degrees() * 10.0).round() as i32),
        );

        angles.sort_unstable();
        angles.dedup();
        assert_eq!(angles.len(), 3);
    }

    #[test]
    fn split_horizontal_mode_leaves_center_gap() {
        let mut positions = Vec::new();
        for_each_horizontal_bar_mode(
            &[1.0, 1.0, 1.0, 1.0],
            HorizontalBarLayout {
                width: 400.0,
                height: 100.0,
                bar_thickness: 20.0,
                gap: 10.0,
                from_top: false,
                mode: LinearBarMode::Split { center_gap: 80.0 },
            },
            |_, x, _, bar_width, _| positions.push((x, bar_width)),
        );

        assert_eq!(positions.len(), 4);
        let left_end = positions[1].0 + positions[1].1;
        let right_start = positions[2].0;
        assert!(right_start - left_end >= 80.0 - 1e-6);
    }

    #[test]
    fn split_vertical_mode_leaves_center_gap() {
        let mut positions = Vec::new();
        for_each_vertical_bar_mode(
            &[1.0, 1.0, 1.0, 1.0],
            VerticalBarLayout {
                width: 100.0,
                height: 400.0,
                bar_thickness: 20.0,
                gap: 10.0,
                from_left: false,
                mode: LinearBarMode::Split { center_gap: 80.0 },
            },
            |_, _, y, _, bar_height| positions.push((y, bar_height)),
        );

        assert_eq!(positions.len(), 4);
        let top_end = positions[1].0 + positions[1].1;
        let bottom_start = positions[2].0;
        assert!(bottom_start - top_end >= 80.0 - 1e-6);
    }
}
