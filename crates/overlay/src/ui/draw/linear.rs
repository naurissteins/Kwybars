use super::path::append_bar_path;
use super::types::{
    BarOrientation, BarRect, BarStyle, HorizontalBarLayout, LinearBarMode, MirrorHorizontalLayout,
    MirrorVerticalLayout, VerticalBarLayout,
};

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

pub fn for_each_horizontal_mirror_bar_mode(
    values: &[f64],
    layout: MirrorHorizontalLayout,
    mut paint: impl FnMut(usize, f64, f64, f64, f64),
) {
    let mirror_gap = effective_mirror_gap(layout.height, layout.mirror_gap);
    let half_gap = mirror_gap * 0.5;
    let available_half_height = ((layout.height - mirror_gap) * 0.5).max(1.0);
    for_each_linear_slot(
        values.len(),
        layout.width,
        layout.bar_thickness,
        layout.gap,
        layout.mode,
        |index, x, bar_width| {
            let value = values[index];
            let half_height = (available_half_height * value.clamp(0.0, 1.0)).max(1.0);
            paint(index, x, bar_width, half_height, half_gap);
        },
    );
}

pub fn for_each_vertical_mirror_bar_mode(
    values: &[f64],
    layout: MirrorVerticalLayout,
    mut paint: impl FnMut(usize, f64, f64, f64, f64),
) {
    let mirror_gap = effective_mirror_gap(layout.width, layout.mirror_gap);
    let half_gap = mirror_gap * 0.5;
    let available_half_width = ((layout.width - mirror_gap) * 0.5).max(1.0);
    for_each_linear_slot(
        values.len(),
        layout.height,
        layout.bar_thickness,
        layout.gap,
        layout.mode,
        |index, y, bar_height| {
            let value = values[index];
            let half_width = (available_half_width * value.clamp(0.0, 1.0)).max(1.0);
            paint(index, y, bar_height, half_width, half_gap);
        },
    );
}

pub fn draw_horizontal_mirror_bars_mode(
    ctx: &gtk::cairo::Context,
    values: &[f64],
    layout: MirrorHorizontalLayout,
    style: BarStyle,
    origin_x: f64,
    origin_y: f64,
) {
    let center_y = origin_y + (layout.height * 0.5);
    for_each_horizontal_mirror_bar_mode(
        values,
        layout,
        |_, x, bar_width, half_height, half_gap| {
            append_bar_path(
                ctx,
                BarRect {
                    x: origin_x + x,
                    y: center_y - half_gap - half_height,
                    width: bar_width,
                    height: half_height,
                },
                style,
                BarOrientation::Horizontal,
                false,
            );
            append_bar_path(
                ctx,
                BarRect {
                    x: origin_x + x,
                    y: center_y + half_gap,
                    width: bar_width,
                    height: half_height,
                },
                style,
                BarOrientation::Horizontal,
                true,
            );
        },
    );
}

pub fn draw_vertical_mirror_bars_mode(
    ctx: &gtk::cairo::Context,
    values: &[f64],
    layout: MirrorVerticalLayout,
    style: BarStyle,
    origin_x: f64,
    origin_y: f64,
) {
    let center_x = origin_x + (layout.width * 0.5);
    for_each_vertical_mirror_bar_mode(values, layout, |_, y, bar_height, half_width, half_gap| {
        append_bar_path(
            ctx,
            BarRect {
                x: center_x - half_gap - half_width,
                y: origin_y + y,
                width: half_width,
                height: bar_height,
            },
            style,
            BarOrientation::Vertical,
            false,
        );
        append_bar_path(
            ctx,
            BarRect {
                x: center_x + half_gap,
                y: origin_y + y,
                width: half_width,
                height: bar_height,
            },
            style,
            BarOrientation::Vertical,
            true,
        );
    });
}

fn effective_mirror_gap(total_extent: f64, requested_gap: f64) -> f64 {
    requested_gap.clamp(0.0, (total_extent - 2.0).max(0.0))
}

pub(crate) fn for_each_linear_slot(
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

pub fn bar_color_index(bar_index: usize, bar_count: usize, color_count: usize) -> usize {
    if bar_count == 0 || color_count == 0 {
        return 0;
    }
    let idx = bar_index.saturating_mul(color_count) / bar_count;
    idx.min(color_count - 1)
}
