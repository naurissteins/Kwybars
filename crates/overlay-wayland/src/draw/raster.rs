use super::geometry::BarGeometry;

const AA_SAMPLES: [f64; 4] = [0.125, 0.375, 0.625, 0.875];
const EDGE_AA_MARGIN: u32 = 2;

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
}

impl Bounds {
    fn width(self) -> u32 {
        self.x1 - self.x0
    }

    fn height(self) -> u32 {
        self.y1 - self.y0
    }
}

pub fn fill_rect(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    rect: Rect,
    color: u32,
    geometry: &BarGeometry,
    row_span: &mut Vec<u8>,
) {
    let x0 = rect.x.max(0) as u32;
    let y0 = rect.y.max(0) as u32;
    let x1 = (rect.x + rect.width as i32).min(width as i32).max(0) as u32;
    let y1 = (rect.y + rect.height as i32).min(height as i32).max(0) as u32;
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let bounds = Bounds { x0, x1, y0, y1 };
    let radius = geometry.rounded_radius(bounds.width(), bounds.height());
    if radius <= 0.0 {
        fill_rect_rows(canvas, width, bounds, color, row_span);
        return;
    }

    fill_rounded_rect_rows(canvas, width, bounds, radius, color, row_span);
}

fn fill_rect_rows(
    canvas: &mut [u8],
    canvas_width: u32,
    bounds: Bounds,
    color: u32,
    row_span: &mut Vec<u8>,
) {
    fill_row_span(
        canvas,
        canvas_width,
        bounds.x0,
        bounds.x1,
        bounds.y0..bounds.y1,
        color,
        row_span,
    );
}

fn fill_rounded_rect_rows(
    canvas: &mut [u8],
    canvas_width: u32,
    bounds: Bounds,
    radius: f64,
    color: u32,
    row_span: &mut Vec<u8>,
) {
    let corner_band = radius.ceil() as u32;
    let top_end = bounds.y0.saturating_add(corner_band).min(bounds.y1);
    let bottom_start = bounds.y1.saturating_sub(corner_band).max(bounds.y0);

    for row in bounds.y0..top_end {
        fill_rounded_corner_row(canvas, canvas_width, bounds, row, radius, color, row_span);
    }
    if top_end < bottom_start {
        fill_row_span(
            canvas,
            canvas_width,
            bounds.x0,
            bounds.x1,
            top_end..bottom_start,
            color,
            row_span,
        );
    }
    for row in bottom_start..bounds.y1 {
        fill_rounded_corner_row(canvas, canvas_width, bounds, row, radius, color, row_span);
    }
}

fn fill_rounded_corner_row(
    canvas: &mut [u8],
    canvas_width: u32,
    bounds: Bounds,
    row: u32,
    radius: f64,
    color: u32,
    row_span: &mut Vec<u8>,
) {
    let row_center = f64::from(row) + 0.5;
    let top_center_y = f64::from(bounds.y0) + radius;
    let bottom_center_y = f64::from(bounds.y1) - radius;
    let dy = if row_center < top_center_y {
        top_center_y - row_center
    } else if row_center > bottom_center_y {
        row_center - bottom_center_y
    } else {
        0.0
    };

    if dy <= 0.0 {
        fill_row_span(
            canvas,
            canvas_width,
            bounds.x0,
            bounds.x1,
            row..row + 1,
            color,
            row_span,
        );
        return;
    }

    let dx = ((radius * radius) - (dy * dy)).max(0.0).sqrt();
    let left_edge = f64::from(bounds.x0) + radius - dx;
    let right_edge = f64::from(bounds.x1) - radius + dx;
    let solid_x0 = ceil_clamped(left_edge, bounds.x0, bounds.x1);
    let solid_x1 = floor_clamped(right_edge, bounds.x0, bounds.x1);

    paint_edge_range(
        canvas,
        canvas_width,
        bounds,
        row,
        solid_x0.saturating_sub(EDGE_AA_MARGIN).max(bounds.x0)..solid_x0,
        radius,
        color,
    );
    if solid_x0 < solid_x1 {
        fill_row_span(
            canvas,
            canvas_width,
            solid_x0,
            solid_x1,
            row..row + 1,
            color,
            row_span,
        );
    }
    paint_edge_range(
        canvas,
        canvas_width,
        bounds,
        row,
        solid_x1..solid_x1.saturating_add(EDGE_AA_MARGIN).min(bounds.x1),
        radius,
        color,
    );
}

fn ceil_clamped(value: f64, min: u32, max: u32) -> u32 {
    value.ceil().clamp(f64::from(min), f64::from(max)) as u32
}

fn floor_clamped(value: f64, min: u32, max: u32) -> u32 {
    value.floor().clamp(f64::from(min), f64::from(max)) as u32
}

fn paint_edge_range(
    canvas: &mut [u8],
    canvas_width: u32,
    bounds: Bounds,
    row: u32,
    range: std::ops::Range<u32>,
    radius: f64,
    color: u32,
) {
    for x in range {
        paint_covered_pixel(canvas, canvas_width, x, row, bounds, radius, color);
    }
}

fn fill_row_span(
    canvas: &mut [u8],
    canvas_width: u32,
    x0: u32,
    x1: u32,
    rows: std::ops::Range<u32>,
    color: u32,
    row_span: &mut Vec<u8>,
) {
    let row_bytes = ((x1 - x0) * 4) as usize;
    fill_solid_span(row_span, row_bytes, color);
    for row in rows {
        let start = ((row * canvas_width + x0) * 4) as usize;
        let end = start + row_bytes;
        canvas[start..end].copy_from_slice(&row_span[..row_bytes]);
    }
}

fn fill_solid_span(span: &mut Vec<u8>, row_bytes: usize, color: u32) {
    if span.len() != row_bytes {
        span.resize(row_bytes, 0);
    }

    let color_bytes = color.to_le_bytes();
    for chunk in span.chunks_exact_mut(4) {
        chunk.copy_from_slice(&color_bytes);
    }
}

fn paint_covered_pixel(
    canvas: &mut [u8],
    canvas_width: u32,
    x: u32,
    y: u32,
    bounds: Bounds,
    radius: f64,
    color: u32,
) {
    let coverage = rounded_pixel_coverage(x, y, bounds, radius);
    if coverage <= 0.0 {
        return;
    }

    let color = if coverage >= 1.0 {
        color
    } else {
        scale_premultiplied_argb(color, coverage)
    };
    write_pixel(canvas, canvas_width, x, y, color);
}

fn rounded_pixel_coverage(x: u32, y: u32, bounds: Bounds, radius: f64) -> f64 {
    let mut inside = 0_u32;
    let total = (AA_SAMPLES.len() * AA_SAMPLES.len()) as u32;

    for sample_y in AA_SAMPLES {
        for sample_x in AA_SAMPLES {
            if point_inside_rounded_rect(
                f64::from(x) + sample_x,
                f64::from(y) + sample_y,
                bounds,
                radius,
            ) {
                inside += 1;
            }
        }
    }

    f64::from(inside) / f64::from(total)
}

fn point_inside_rounded_rect(x: f64, y: f64, bounds: Bounds, radius: f64) -> bool {
    let left = f64::from(bounds.x0);
    let right = f64::from(bounds.x1);
    let top = f64::from(bounds.y0);
    let bottom = f64::from(bounds.y1);

    let nearest_x = x.clamp(left + radius, right - radius);
    let nearest_y = y.clamp(top + radius, bottom - radius);
    let dx = x - nearest_x;
    let dy = y - nearest_y;
    (dx * dx + dy * dy) <= radius * radius
}

fn write_pixel(canvas: &mut [u8], canvas_width: u32, x: u32, y: u32, color: u32) {
    let start = ((y * canvas_width + x) * 4) as usize;
    let end = start + 4;
    canvas[start..end].copy_from_slice(&color.to_le_bytes());
}

fn scale_premultiplied_argb(color: u32, coverage: f64) -> u32 {
    let coverage = coverage.clamp(0.0, 1.0);
    let a = scale_byte((color >> 24) as u8, coverage);
    let r = scale_byte((color >> 16) as u8, coverage);
    let g = scale_byte((color >> 8) as u8, coverage);
    let b = scale_byte(color as u8, coverage);

    u32::from(a) << 24 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

fn scale_byte(value: u8, coverage: f64) -> u8 {
    (f64::from(value) * coverage).round().clamp(0.0, 255.0) as u8
}
