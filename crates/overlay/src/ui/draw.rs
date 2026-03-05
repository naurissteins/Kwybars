use std::f64::consts::{FRAC_PI_2, PI};

#[derive(Clone, Copy)]
pub struct BarStyle {
    pub thickness: f64,
    pub gap: f64,
    pub corner_radius: f64,
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
            append_bar_path(ctx, x, y, bar_width, bar_height, style.corner_radius);
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
            append_bar_path(ctx, x, y, bar_width, bar_height, style.corner_radius);
        },
    );
}

pub fn append_bar_path(
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

pub fn bar_color_index(bar_index: usize, bar_count: usize, color_count: usize) -> usize {
    if bar_count == 0 || color_count == 0 {
        return 0;
    }
    let idx = bar_index.saturating_mul(color_count) / bar_count;
    idx.min(color_count - 1)
}

#[cfg(test)]
mod tests {
    use super::bar_color_index;

    #[test]
    fn spreads_colors_evenly() {
        let indices: Vec<usize> = (0..12).map(|index| bar_color_index(index, 12, 6)).collect();
        assert_eq!(indices, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
    }
}
