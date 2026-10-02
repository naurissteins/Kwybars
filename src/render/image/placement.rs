use super::MAX_PIXELS;
use crate::config::{ImageFit, ImageOverlayConfig};

/// a rectangle that may start left of or above its parent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub draw: (u32, u32),
    pub origin: (i32, i32),
}

pub fn layout(
    config: &ImageOverlayConfig,
    source: (u32, u32),
    logical: (u32, u32),
    scale: f32,
) -> Option<Layout> {
    let (canvas_w, canvas_h) = (logical.0 as f32, logical.1 as f32);
    let (source_w, source_h) = (source.0.max(1) as f32, source.1.max(1) as f32);
    let or_canvas = |size: u32, canvas: f32| if size > 0 { size as f32 } else { canvas };
    let (box_w, box_h) = (
        or_canvas(config.width, canvas_w),
        or_canvas(config.height, canvas_h),
    );
    let (ratio_w, ratio_h) = (box_w / source_w, box_h / source_h);
    let (draw_w, draw_h) = match config.fit {
        ImageFit::Contain => (
            source_w * ratio_w.min(ratio_h),
            source_h * ratio_w.min(ratio_h),
        ),
        ImageFit::Cover => (
            source_w * ratio_w.max(ratio_h),
            source_h * ratio_w.max(ratio_h),
        ),
        ImageFit::Stretch => (box_w, box_h),
    };
    if canvas_w <= 0.0 || canvas_h <= 0.0 || draw_w <= 0.0 || draw_h <= 0.0 {
        return None;
    }
    // legacy rounds the size to whole logical pixels, at least one
    let (draw_w, draw_h) = (draw_w.round().max(1.0), draw_h.round().max(1.0));
    let x = (canvas_w - draw_w) * 0.5 + config.offset_x;
    let y = (canvas_h - draw_h) * 0.5 + config.offset_y;

    let pixels = |logical: f32| (logical * scale).round();
    let draw = (
        pixels(draw_w).max(1.0) as u32,
        pixels(draw_h).max(1.0) as u32,
    );
    if u64::from(draw.0) * u64::from(draw.1) > MAX_PIXELS {
        return None;
    }
    Some(Layout {
        draw,
        origin: (pixels(x) as i32, pixels(y) as i32),
    })
}

/// the child surface that holds the image
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Child {
    pub logical: Rect,
    pub buffer: Rect,
}

impl Child {
    pub fn around(placed: &Layout, (per_step, step): (u32, u32)) -> Self {
        let (per_step, step) = (per_step.max(1), step.max(1));
        // one axis: the logical and buffer start and length
        let axis = |start: i32, length: u32| {
            let per = i64::from(per_step);
            let first = i64::from(start).div_euclid(per);
            let end = i64::from(start) + i64::from(length);
            let last = (end + per - 1).div_euclid(per);
            let steps = last - first;
            (
                (first * i64::from(step), steps * i64::from(step)),
                (first * per, steps * per),
            )
        };
        let (lx, bx) = axis(placed.origin.0, placed.draw.0);
        let (ly, by) = axis(placed.origin.1, placed.draw.1);
        let rect = |x: (i64, i64), y: (i64, i64)| Rect {
            x: x.0 as i32,
            y: y.0 as i32,
            width: x.1 as u32,
            height: y.1 as u32,
        };
        Self {
            logical: rect(lx, ly),
            buffer: rect(bx, by),
        }
    }
}
