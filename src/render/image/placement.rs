use crate::config::{ImageFit, ImageOverlayConfig};
use crate::render::PixelRect;

/// where the scaled image goes in a surface's buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub draw: (u32, u32),
    pub origin: (i32, i32),
    pub visible: PixelRect,
}

pub fn layout(
    config: &ImageOverlayConfig,
    source: (u32, u32),
    logical: (u32, u32),
    buffer: (u32, u32),
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
    let origin = (pixels(x) as i32, pixels(y) as i32);
    let visible = PixelRect::covering(
        (
            origin.0 as f32,
            origin.1 as f32,
            origin.0 as f32 + draw.0 as f32,
            origin.1 as f32 + draw.1 as f32,
        ),
        buffer,
    )?;
    Some(Layout {
        draw,
        origin,
        visible,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Child {
    /// position and size in the parent, in pixels
    pub logical: PixelRect,
    /// the same in buffer pixels: the child's buffer
    pub buffer: PixelRect,
}

impl Child {
    pub fn around(
        visible: PixelRect,
        logical: (u32, u32),
        buffer: (u32, u32),
        (per_step, step): (u32, u32),
    ) -> Self {
        let (per_step, step) = (per_step.max(1), step.max(1));
        // one axis: the logical and buffer start and end
        let axis = |start: u32, end: u32, logical: u32, buffer: u32| {
            let first = start / per_step * step;
            let last = end.div_ceil(per_step) * step;
            // the far edge of the surface is aligned however its size rounds
            let (last, buffer_end) = if last >= logical {
                (logical, buffer)
            } else {
                (last, last / step * per_step)
            };
            let first = first.min(last);
            ((first, last), (first / step * per_step, buffer_end))
        };
        let (lx, bx) = axis(visible.x, visible.right(), logical.0, buffer.0);
        let (ly, by) = axis(visible.y, visible.bottom(), logical.1, buffer.1);
        let rect = |x: (u32, u32), y: (u32, u32)| PixelRect {
            x: x.0,
            y: y.0,
            width: x.1.saturating_sub(x.0),
            height: y.1.saturating_sub(y.0),
        };
        Self {
            logical: rect(lx, ly),
            buffer: rect(bx, by),
        }
    }
}
