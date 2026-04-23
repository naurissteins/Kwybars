mod geometry;
mod paint;
mod raster;

use kwybars_common::config::{FrameMirrorMode, OverlayPosition};
use kwybars_common::spectrum::SpectrumFrame;

pub use geometry::BarGeometry;
use geometry::{BarSlots, FrameMetrics};
pub use paint::BarPaint;
use raster::{Rect, clear_rect, fill_rect};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderDamage {
    None,
    Full,
    Rects(Vec<DamageRect>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl From<Rect> for DamageRect {
    fn from(rect: Rect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
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
) -> RenderDamage {
    let width = target.width;
    let height = target.height;
    if width == 0 || height == 0 {
        return RenderDamage::None;
    }

    let metrics = RenderMetrics::new(target.scale, width, height);
    let scaled_geometry = geometry.scaled(f64::from(metrics.scale));
    let mut context = RenderContext {
        frame,
        paint,
        geometry: &scaled_geometry,
        metrics,
        row_span: Vec::new(),
    };

    if scaled_geometry.is_frame_layout() {
        let damage_rects = frame_damage_rects(width, height, &scaled_geometry);
        clear_damage_rects(canvas, width, height, &damage_rects);
        if !frame.bars.is_empty() {
            render_frame_bars(canvas, width, height, &mut context);
        }
        return if damage_rects.is_empty() {
            RenderDamage::None
        } else {
            RenderDamage::Rects(damage_rects)
        };
    }

    clear(canvas);
    if frame.bars.is_empty() {
        return RenderDamage::Full;
    }

    match position {
        OverlayPosition::Bottom => {
            render_horizontal_bars(canvas, width, height, false, &mut context)
        }
        OverlayPosition::Top => render_horizontal_bars(canvas, width, height, true, &mut context),
        OverlayPosition::Left => render_vertical_bars(canvas, width, height, true, &mut context),
        OverlayPosition::Right => render_vertical_bars(canvas, width, height, false, &mut context),
    }

    RenderDamage::Full
}

struct RenderContext<'a> {
    frame: &'a SpectrumFrame,
    paint: &'a BarPaint,
    geometry: &'a BarGeometry,
    metrics: RenderMetrics,
    row_span: Vec<u8>,
}

struct EdgeSlice<'a> {
    values: &'a [f32],
    global_offset: usize,
}

#[derive(Debug, Clone, Copy)]
struct RenderMetrics {
    scale: u32,
    canvas_width: u32,
    canvas_height: u32,
    horizontal_padding: u32,
    vertical_padding: u32,
    min_bar_extent: u32,
}

impl RenderMetrics {
    fn new(scale: u32, canvas_width: u32, canvas_height: u32) -> Self {
        let scale = scale.max(1);
        Self {
            scale,
            canvas_width,
            canvas_height,
            horizontal_padding: HORIZONTAL_PADDING.saturating_mul(scale),
            vertical_padding: VERTICAL_PADDING.saturating_mul(scale),
            min_bar_extent: MIN_BAR_HEIGHT.saturating_mul(scale),
        }
    }
}

fn frame_damage_rects(width: u32, height: u32, geometry: &BarGeometry) -> Vec<DamageRect> {
    let metrics = geometry.frame_metrics(width, height);
    geometry
        .frame_edges()
        .iter()
        .map(|edge| frame_edge_rect(edge, metrics).rect.into())
        .collect()
}

fn clear_damage_rects(canvas: &mut [u8], width: u32, height: u32, rects: &[DamageRect]) {
    for rect in rects {
        clear_rect(
            canvas,
            width,
            height,
            Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            },
        );
    }
}

fn render_frame_bars(canvas: &mut [u8], width: u32, height: u32, context: &mut RenderContext<'_>) {
    let edges = context.geometry.frame_edges();
    if edges.is_empty() {
        return;
    }

    let metrics = context.geometry.frame_metrics(width, height);
    for (edge_index, edge) in edges.iter().enumerate() {
        let edge_slice = resolve_frame_edge_slice(
            &context.frame.bars,
            edges,
            edge_index,
            context.geometry.frame_mirror_mode(),
        );
        if edge_slice.values.is_empty() {
            continue;
        }

        let edge_rect = frame_edge_rect(edge, metrics);
        render_frame_edge(canvas, edge_slice, edge_rect, context);
    }
}

fn render_frame_edge(
    canvas: &mut [u8],
    edge_slice: EdgeSlice<'_>,
    edge_rect: FrameEdgeRect,
    context: &mut RenderContext<'_>,
) {
    match edge_rect.orientation {
        EdgeOrientation::Horizontal => render_horizontal_frame_edge(
            canvas,
            edge_slice,
            edge_rect.rect,
            edge_rect.from_start,
            context,
        ),
        EdgeOrientation::Vertical => render_vertical_frame_edge(
            canvas,
            edge_slice,
            edge_rect.rect,
            edge_rect.from_start,
            context,
        ),
    }
}

fn render_horizontal_frame_edge(
    canvas: &mut [u8],
    edge_slice: EdgeSlice<'_>,
    rect: Rect,
    from_top: bool,
    context: &mut RenderContext<'_>,
) {
    if rect.width == 0 || rect.height < context.metrics.min_bar_extent {
        return;
    }

    let slots = BarSlots::continuous(
        edge_slice.values.len(),
        f64::from(rect.width),
        context.geometry,
    );
    let top_y = rect.y;
    let bottom_y = rect.y + rect.height as i32;

    for slot in slots {
        let local_index = slot.index;
        let x = f64::from(rect.x) + slot.start;
        let bar_height = frame_bar_extent(
            edge_slice.values[local_index],
            rect.height,
            context.metrics.min_bar_extent,
        );
        let global_index = edge_slice.global_offset + local_index;
        let color = context
            .paint
            .color_for_bar(global_index, context.frame.bar_count());
        let y = if from_top {
            top_y
        } else {
            bottom_y - bar_height as i32
        };
        fill_bar(
            canvas,
            CanvasSize {
                width: context.metrics.canvas_width,
                height: context.metrics.canvas_height,
            },
            Rect {
                x: x.round() as i32,
                y,
                width: slot.thickness.round().max(1.0) as u32,
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

fn render_vertical_frame_edge(
    canvas: &mut [u8],
    edge_slice: EdgeSlice<'_>,
    rect: Rect,
    from_left: bool,
    context: &mut RenderContext<'_>,
) {
    if rect.width < context.metrics.min_bar_extent || rect.height == 0 {
        return;
    }

    let slots = BarSlots::continuous(
        edge_slice.values.len(),
        f64::from(rect.height),
        context.geometry,
    );
    let left_x = rect.x;
    let right_x = rect.x + rect.width as i32;

    for slot in slots {
        let local_index = slot.index;
        let y = f64::from(rect.y) + slot.start;
        let bar_width = frame_bar_extent(
            edge_slice.values[local_index],
            rect.width,
            context.metrics.min_bar_extent,
        );
        let global_index = edge_slice.global_offset + local_index;
        let color = context
            .paint
            .color_for_bar(global_index, context.frame.bar_count());
        let x = if from_left {
            left_x
        } else {
            right_x - bar_width as i32
        };
        fill_bar(
            canvas,
            CanvasSize {
                width: context.metrics.canvas_width,
                height: context.metrics.canvas_height,
            },
            Rect {
                x,
                y: y.round() as i32,
                width: bar_width,
                height: slot.thickness.round().max(1.0) as u32,
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

fn resolve_frame_edge_slice<'a>(
    values: &'a [f32],
    active_edges: &[OverlayPosition],
    edge_index: usize,
    mirror_mode: FrameMirrorMode,
) -> EdgeSlice<'a> {
    match mirror_mode {
        FrameMirrorMode::Off => EdgeSlice {
            values: distributed_chunk(values, edge_index, active_edges.len()),
            global_offset: values.len() * edge_index / active_edges.len(),
        },
        FrameMirrorMode::All => EdgeSlice {
            values,
            global_offset: 0,
        },
        FrameMirrorMode::Pairs => resolve_pairs_frame_edge_slice(values, active_edges, edge_index),
    }
}

fn resolve_pairs_frame_edge_slice<'a>(
    values: &'a [f32],
    active_edges: &[OverlayPosition],
    edge_index: usize,
) -> EdgeSlice<'a> {
    let has_horizontal = active_edges
        .iter()
        .any(|edge| matches!(edge, OverlayPosition::Top | OverlayPosition::Bottom));
    let has_vertical = active_edges
        .iter()
        .any(|edge| matches!(edge, OverlayPosition::Left | OverlayPosition::Right));

    if has_horizontal && has_vertical {
        let (group_index, global_offset) = if matches!(
            active_edges[edge_index],
            OverlayPosition::Top | OverlayPosition::Bottom
        ) {
            (0, 0)
        } else {
            (1, values.len() / 2)
        };

        return EdgeSlice {
            values: distributed_chunk(values, group_index, 2),
            global_offset,
        };
    }

    EdgeSlice {
        values,
        global_offset: 0,
    }
}

fn distributed_chunk<T>(values: &[T], group_index: usize, group_count: usize) -> &[T] {
    if values.is_empty() || group_count == 0 {
        return &[];
    }

    let start = values.len() * group_index / group_count;
    let end = values.len() * (group_index + 1) / group_count;
    &values[start..end]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameEdgeRect {
    rect: Rect,
    orientation: EdgeOrientation,
    from_start: bool,
}

fn frame_edge_rect(edge: &OverlayPosition, metrics: FrameMetrics) -> FrameEdgeRect {
    match edge {
        OverlayPosition::Top => FrameEdgeRect {
            rect: Rect {
                x: metrics.margin_left.round() as i32,
                y: (metrics.anchor_margin + metrics.margin_top).round() as i32,
                width: (metrics.width - metrics.margin_left - metrics.margin_right)
                    .max(1.0)
                    .round() as u32,
                height: metrics.top_thickness.max(1.0).round() as u32,
            },
            orientation: EdgeOrientation::Horizontal,
            from_start: true,
        },
        OverlayPosition::Bottom => FrameEdgeRect {
            rect: Rect {
                x: metrics.margin_left.round() as i32,
                y: (metrics.height
                    - metrics.anchor_margin
                    - metrics.margin_bottom
                    - metrics.bottom_thickness)
                    .max(0.0)
                    .round() as i32,
                width: (metrics.width - metrics.margin_left - metrics.margin_right)
                    .max(1.0)
                    .round() as u32,
                height: metrics.bottom_thickness.max(1.0).round() as u32,
            },
            orientation: EdgeOrientation::Horizontal,
            from_start: false,
        },
        OverlayPosition::Left => FrameEdgeRect {
            rect: Rect {
                x: (metrics.anchor_margin + metrics.margin_left).round() as i32,
                y: metrics.margin_top.round() as i32,
                width: metrics.left_thickness.max(1.0).round() as u32,
                height: (metrics.height - metrics.margin_top - metrics.margin_bottom)
                    .max(1.0)
                    .round() as u32,
            },
            orientation: EdgeOrientation::Vertical,
            from_start: true,
        },
        OverlayPosition::Right => FrameEdgeRect {
            rect: Rect {
                x: (metrics.width
                    - metrics.anchor_margin
                    - metrics.margin_right
                    - metrics.right_thickness)
                    .max(0.0)
                    .round() as i32,
                y: metrics.margin_top.round() as i32,
                width: metrics.right_thickness.max(1.0).round() as u32,
                height: (metrics.height - metrics.margin_top - metrics.margin_bottom)
                    .max(1.0)
                    .round() as u32,
            },
            orientation: EdgeOrientation::Vertical,
            from_start: false,
        },
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
    let top_y = vertical_padding as i32;
    let bottom_y = (height.saturating_sub(vertical_padding)) as i32;

    for slot in slots {
        let index = slot.index;
        let x = f64::from(horizontal_padding) + slot.start;
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
                width: slot.thickness.round().max(1.0) as u32,
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
    let left_x = horizontal_padding as i32;
    let right_x = (width.saturating_sub(horizontal_padding)) as i32;

    for slot in slots {
        let index = slot.index;
        let y = f64::from(vertical_padding) + slot.start;
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
                height: slot.thickness.round().max(1.0) as u32,
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
