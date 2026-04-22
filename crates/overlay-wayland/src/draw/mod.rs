mod geometry;
mod paint;

use kwybars_common::config::OverlayPosition;
use kwybars_common::spectrum::SpectrumFrame;

pub use geometry::BarGeometry;
use geometry::BarSlots;
pub use paint::BarPaint;

const HORIZONTAL_PADDING: u32 = 24;
const VERTICAL_PADDING: u32 = 12;
const MIN_BAR_HEIGHT: u32 = 10;

struct Rect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

pub fn render_bars(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    frame: &SpectrumFrame,
    position: &OverlayPosition,
    paint: &BarPaint,
    geometry: &BarGeometry,
) {
    clear(canvas);

    if width == 0 || height == 0 {
        return;
    }

    if frame.bars.is_empty() {
        return;
    }

    let mut context = RenderContext {
        frame,
        paint,
        geometry,
        row_span: Vec::new(),
    };

    match position {
        OverlayPosition::Bottom => {
            render_horizontal_bars(canvas, width, height, false, &mut context)
        }
        OverlayPosition::Top => render_horizontal_bars(canvas, width, height, true, &mut context),
        OverlayPosition::Left => render_vertical_bars(canvas, width, height, true, &mut context),
        OverlayPosition::Right => render_vertical_bars(canvas, width, height, false, &mut context),
    }
}

struct RenderContext<'a> {
    frame: &'a SpectrumFrame,
    paint: &'a BarPaint,
    geometry: &'a BarGeometry,
    row_span: Vec<u8>,
}

fn render_horizontal_bars(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    from_top: bool,
    context: &mut RenderContext<'_>,
) {
    let drawable_width = width.saturating_sub(HORIZONTAL_PADDING * 2);
    let drawable_height = height.saturating_sub(VERTICAL_PADDING * 2);
    if drawable_width == 0 || drawable_height < MIN_BAR_HEIGHT {
        return;
    }

    let bar_count = context.frame.bar_count();
    let slots = BarSlots::new(bar_count, f64::from(drawable_width), context.geometry);
    let start_x = f64::from(HORIZONTAL_PADDING) + slots.start;
    let top_y = VERTICAL_PADDING as i32;
    let bottom_y = (height.saturating_sub(VERTICAL_PADDING)) as i32;

    for index in 0..bar_count {
        let x = start_x + (index as f64 * slots.step);
        let bar_height = frame_bar_extent(context.frame.bars[index], drawable_height);
        let color = context.paint.color_for_bar(index, bar_count);
        let y = if from_top {
            top_y
        } else {
            bottom_y - bar_height as i32
        };
        fill_rect(
            canvas,
            width,
            height,
            Rect {
                x: x.round() as i32,
                y,
                width: slots.thickness.round().max(1.0) as u32,
                height: bar_height,
            },
            color,
            &mut context.row_span,
        );
    }
}

fn render_vertical_bars(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    from_left: bool,
    context: &mut RenderContext<'_>,
) {
    let drawable_width = width.saturating_sub(HORIZONTAL_PADDING * 2);
    let drawable_height = height.saturating_sub(VERTICAL_PADDING * 2);
    if drawable_width < MIN_BAR_HEIGHT || drawable_height == 0 {
        return;
    }

    let bar_count = context.frame.bar_count();
    let slots = BarSlots::new(bar_count, f64::from(drawable_height), context.geometry);
    let start_y = f64::from(VERTICAL_PADDING) + slots.start;
    let left_x = HORIZONTAL_PADDING as i32;
    let right_x = (width.saturating_sub(HORIZONTAL_PADDING)) as i32;

    for index in 0..bar_count {
        let y = start_y + (index as f64 * slots.step);
        let bar_width = frame_bar_extent(context.frame.bars[index], drawable_width);
        let color = context.paint.color_for_bar(index, bar_count);
        let x = if from_left {
            left_x
        } else {
            right_x - bar_width as i32
        };
        fill_rect(
            canvas,
            width,
            height,
            Rect {
                x,
                y: y.round() as i32,
                width: bar_width,
                height: slots.thickness.round().max(1.0) as u32,
            },
            color,
            &mut context.row_span,
        );
    }
}

fn frame_bar_extent(value: f32, drawable_extent: u32) -> u32 {
    let normalized = 0.16 + value.clamp(0.0, 1.0) as f64 * 0.76;
    let extent = (drawable_extent as f64 * normalized).round() as u32;
    extent.max(MIN_BAR_HEIGHT).min(drawable_extent)
}

fn clear(canvas: &mut [u8]) {
    canvas.fill(0);
}

fn fill_rect(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    rect: Rect,
    color: u32,
    row_span: &mut Vec<u8>,
) {
    let x0 = rect.x.max(0) as u32;
    let y0 = rect.y.max(0) as u32;
    let x1 = (rect.x + rect.width as i32).min(width as i32).max(0) as u32;
    let y1 = (rect.y + rect.height as i32).min(height as i32).max(0) as u32;
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let row_bytes = ((x1 - x0) * 4) as usize;
    fill_solid_span(row_span, row_bytes, color);
    for row in y0..y1 {
        let start = ((row * width + x0) * 4) as usize;
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

#[cfg(test)]
mod tests;
