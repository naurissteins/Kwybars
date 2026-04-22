mod geometry;
mod paint;
mod raster;

use kwybars_common::config::OverlayPosition;
use kwybars_common::spectrum::SpectrumFrame;

pub use geometry::BarGeometry;
use geometry::BarSlots;
pub use paint::BarPaint;
use raster::{Rect, fill_rect};

const HORIZONTAL_PADDING: u32 = 24;
const VERTICAL_PADDING: u32 = 12;
const MIN_BAR_HEIGHT: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderTarget {
    width: u32,
    height: u32,
    scale: u32,
}

impl RenderTarget {
    pub fn new(width: u32, height: u32, scale: u32) -> Self {
        Self {
            width,
            height,
            scale: scale.max(1),
        }
    }
}

pub fn render_bars(
    canvas: &mut [u8],
    target: RenderTarget,
    frame: &SpectrumFrame,
    position: &OverlayPosition,
    paint: &BarPaint,
    geometry: &BarGeometry,
) {
    clear(canvas);

    let width = target.width;
    let height = target.height;
    if width == 0 || height == 0 {
        return;
    }

    if frame.bars.is_empty() {
        return;
    }

    let metrics = RenderMetrics::new(target.scale);
    let scaled_geometry = geometry.scaled(f64::from(metrics.scale));
    let mut context = RenderContext {
        frame,
        paint,
        geometry: &scaled_geometry,
        metrics,
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
    metrics: RenderMetrics,
    row_span: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct RenderMetrics {
    scale: u32,
    horizontal_padding: u32,
    vertical_padding: u32,
    min_bar_extent: u32,
}

impl RenderMetrics {
    fn new(scale: u32) -> Self {
        let scale = scale.max(1);
        Self {
            scale,
            horizontal_padding: HORIZONTAL_PADDING.saturating_mul(scale),
            vertical_padding: VERTICAL_PADDING.saturating_mul(scale),
            min_bar_extent: MIN_BAR_HEIGHT.saturating_mul(scale),
        }
    }
}

fn render_horizontal_bars(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    from_top: bool,
    context: &mut RenderContext<'_>,
) {
    let horizontal_padding = context.metrics.horizontal_padding;
    let vertical_padding = context.metrics.vertical_padding;
    let drawable_width = width.saturating_sub(horizontal_padding * 2);
    let drawable_height = height.saturating_sub(vertical_padding * 2);
    if drawable_width == 0 || drawable_height < context.metrics.min_bar_extent {
        return;
    }

    let bar_count = context.frame.bar_count();
    let slots = BarSlots::new(bar_count, f64::from(drawable_width), context.geometry);
    let start_x = f64::from(horizontal_padding) + slots.start;
    let top_y = vertical_padding as i32;
    let bottom_y = (height.saturating_sub(vertical_padding)) as i32;

    for index in 0..bar_count {
        let x = start_x + (index as f64 * slots.step);
        let bar_height = frame_bar_extent(
            context.frame.bars[index],
            drawable_height,
            context.metrics.min_bar_extent,
        );
        let color = context.paint.color_for_bar(index, bar_count);
        let y = if from_top {
            top_y
        } else {
            bottom_y - bar_height as i32
        };
        fill_bar(
            canvas,
            CanvasSize { width, height },
            Rect {
                x: x.round() as i32,
                y,
                width: slots.thickness.round().max(1.0) as u32,
                height: bar_height,
            },
            color,
            context.geometry,
            SegmentAxis::Vertical {
                from_start: from_top,
            },
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
    let horizontal_padding = context.metrics.horizontal_padding;
    let vertical_padding = context.metrics.vertical_padding;
    let drawable_width = width.saturating_sub(horizontal_padding * 2);
    let drawable_height = height.saturating_sub(vertical_padding * 2);
    if drawable_width < context.metrics.min_bar_extent || drawable_height == 0 {
        return;
    }

    let bar_count = context.frame.bar_count();
    let slots = BarSlots::new(bar_count, f64::from(drawable_height), context.geometry);
    let start_y = f64::from(vertical_padding) + slots.start;
    let left_x = horizontal_padding as i32;
    let right_x = (width.saturating_sub(horizontal_padding)) as i32;

    for index in 0..bar_count {
        let y = start_y + (index as f64 * slots.step);
        let bar_width = frame_bar_extent(
            context.frame.bars[index],
            drawable_width,
            context.metrics.min_bar_extent,
        );
        let color = context.paint.color_for_bar(index, bar_count);
        let x = if from_left {
            left_x
        } else {
            right_x - bar_width as i32
        };
        fill_bar(
            canvas,
            CanvasSize { width, height },
            Rect {
                x,
                y: y.round() as i32,
                width: bar_width,
                height: slots.thickness.round().max(1.0) as u32,
            },
            color,
            context.geometry,
            SegmentAxis::Horizontal {
                from_start: from_left,
            },
            &mut context.row_span,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegmentAxis {
    Horizontal { from_start: bool },
    Vertical { from_start: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CanvasSize {
    width: u32,
    height: u32,
}

fn fill_bar(
    canvas: &mut [u8],
    canvas_size: CanvasSize,
    rect: Rect,
    color: u32,
    geometry: &BarGeometry,
    segment_axis: SegmentAxis,
    row_span: &mut Vec<u8>,
) {
    if !geometry.segmented() {
        fill_rect(
            canvas,
            canvas_size.width,
            canvas_size.height,
            rect,
            color,
            geometry,
            row_span,
        );
        return;
    }

    let total_length = match segment_axis {
        SegmentAxis::Horizontal { .. } => rect.width,
        SegmentAxis::Vertical { .. } => rect.height,
    };
    let from_start = match segment_axis {
        SegmentAxis::Horizontal { from_start } | SegmentAxis::Vertical { from_start } => from_start,
    };

    for_each_segment_span(
        f64::from(total_length),
        geometry.segment_length(),
        geometry.segment_gap(),
        from_start,
        |offset, length| {
            let start = offset.round() as i32;
            let end = (offset + length).round() as i32;
            let length = (end - start).max(1) as u32;
            let segment_rect = match segment_axis {
                SegmentAxis::Horizontal { .. } => Rect {
                    x: rect.x + start,
                    y: rect.y,
                    width: length,
                    height: rect.height,
                },
                SegmentAxis::Vertical { .. } => Rect {
                    x: rect.x,
                    y: rect.y + start,
                    width: rect.width,
                    height: length,
                },
            };
            fill_rect(
                canvas,
                canvas_size.width,
                canvas_size.height,
                segment_rect,
                color,
                geometry,
                row_span,
            );
        },
    );
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

fn frame_bar_extent(value: f32, drawable_extent: u32, min_extent: u32) -> u32 {
    let normalized = 0.16 + value.clamp(0.0, 1.0) as f64 * 0.76;
    let extent = (drawable_extent as f64 * normalized).round() as u32;
    extent.max(min_extent).min(drawable_extent)
}

fn clear(canvas: &mut [u8]) {
    canvas.fill(0);
}

#[cfg(test)]
mod tests;
