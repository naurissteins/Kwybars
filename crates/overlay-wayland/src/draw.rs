use kwybars_common::config::OverlayPosition;
use kwybars_common::spectrum::SpectrumFrame;

const TRANSPARENT: u32 = 0x00000000;
const BAR_COLOR: u32 = 0xFFF1F5F9;
const HORIZONTAL_PADDING: u32 = 24;
const VERTICAL_PADDING: u32 = 12;
const MIN_BAR_WIDTH: u32 = 6;
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
) {
    clear(canvas);

    if width == 0 || height == 0 {
        return;
    }

    if frame.bars.is_empty() {
        return;
    }

    match position {
        OverlayPosition::Bottom => render_horizontal_bars(canvas, width, height, frame, false),
        OverlayPosition::Top => render_horizontal_bars(canvas, width, height, frame, true),
        OverlayPosition::Left => render_vertical_bars(canvas, width, height, frame, true),
        OverlayPosition::Right => render_vertical_bars(canvas, width, height, frame, false),
    }
}

fn render_horizontal_bars(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    frame: &SpectrumFrame,
    from_top: bool,
) {
    let drawable_width = width.saturating_sub(HORIZONTAL_PADDING * 2);
    let drawable_height = height.saturating_sub(VERTICAL_PADDING * 2);
    if drawable_width == 0 || drawable_height < MIN_BAR_HEIGHT {
        return;
    }

    let bar_count = frame.bar_count();
    let gap = (drawable_width / 180).max(3);
    let total_gap = gap.saturating_mul(bar_count.saturating_sub(1) as u32);
    let bar_width =
        ((drawable_width.saturating_sub(total_gap)) / bar_count as u32).max(MIN_BAR_WIDTH);
    let occupied_width = bar_width * bar_count as u32 + total_gap;
    let start_x = ((width.saturating_sub(occupied_width)) / 2) as i32;
    let top_y = VERTICAL_PADDING as i32;
    let bottom_y = (height.saturating_sub(VERTICAL_PADDING)) as i32;

    for index in 0..bar_count {
        let x = start_x + index as i32 * (bar_width + gap) as i32;
        let bar_height = frame_bar_extent(frame.bars[index], drawable_height);
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
                x,
                y,
                width: bar_width,
                height: bar_height,
            },
            BAR_COLOR,
        );
    }
}

fn render_vertical_bars(
    canvas: &mut [u8],
    width: u32,
    height: u32,
    frame: &SpectrumFrame,
    from_left: bool,
) {
    let drawable_width = width.saturating_sub(HORIZONTAL_PADDING * 2);
    let drawable_height = height.saturating_sub(VERTICAL_PADDING * 2);
    if drawable_width < MIN_BAR_HEIGHT || drawable_height == 0 {
        return;
    }

    let bar_count = frame.bar_count();
    let gap = (drawable_height / 180).max(3);
    let bar_height = ((drawable_height
        .saturating_sub(gap.saturating_mul(bar_count.saturating_sub(1) as u32)))
        / bar_count as u32)
        .max(MIN_BAR_WIDTH);
    let occupied_height = bar_height * bar_count as u32 + gap * bar_count.saturating_sub(1) as u32;
    let start_y = ((height.saturating_sub(occupied_height)) / 2) as i32;
    let left_x = HORIZONTAL_PADDING as i32;
    let right_x = (width.saturating_sub(HORIZONTAL_PADDING)) as i32;

    for index in 0..bar_count {
        let y = start_y + index as i32 * (bar_height + gap) as i32;
        let bar_width = frame_bar_extent(frame.bars[index], drawable_width);
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
                y,
                width: bar_width,
                height: bar_height,
            },
            BAR_COLOR,
        );
    }
}

fn frame_bar_extent(value: f32, drawable_extent: u32) -> u32 {
    let normalized = 0.16 + value.clamp(0.0, 1.0) as f64 * 0.76;
    let extent = (drawable_extent as f64 * normalized).round() as u32;
    extent.max(MIN_BAR_HEIGHT).min(drawable_extent)
}

fn clear(canvas: &mut [u8]) {
    for chunk in canvas.chunks_exact_mut(4) {
        chunk.copy_from_slice(&TRANSPARENT.to_le_bytes());
    }
}

fn fill_rect(canvas: &mut [u8], width: u32, height: u32, rect: Rect, color: u32) {
    let x0 = rect.x.max(0) as u32;
    let y0 = rect.y.max(0) as u32;
    let x1 = (rect.x + rect.width as i32).min(width as i32).max(0) as u32;
    let y1 = (rect.y + rect.height as i32).min(height as i32).max(0) as u32;
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let bytes = color.to_le_bytes();
    for row in y0..y1 {
        for column in x0..x1 {
            let offset = ((row * width + column) * 4) as usize;
            canvas[offset..offset + 4].copy_from_slice(&bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use kwybars_common::config::OverlayPosition;
    use kwybars_common::spectrum::SpectrumFrame;

    use super::render_bars;

    #[test]
    fn bars_leave_transparent_background() {
        let width = 320;
        let height = 96;
        let mut canvas = vec![0xAA; (width * height * 4) as usize];
        let frame = SpectrumFrame::new(vec![0.25; 20], 0);

        render_bars(&mut canvas, width, height, &frame, &OverlayPosition::Bottom);

        assert_eq!(&canvas[0..4], &[0, 0, 0, 0]);
        assert!(canvas.chunks_exact(4).any(|pixel| pixel[3] != 0));
    }

    #[test]
    fn bars_handle_small_buffers() {
        let mut canvas = vec![0; 4 * 8 * 8];
        let frame = SpectrumFrame::new(vec![0.25; 8], 0);
        render_bars(&mut canvas, 8, 8, &frame, &OverlayPosition::Bottom);
        assert_eq!(canvas.len(), 256);
    }

    #[test]
    fn different_frame_values_change_output() {
        let width = 320;
        let height = 96;
        let mut first = vec![0; (width * height * 4) as usize];
        let mut second = vec![0; (width * height * 4) as usize];
        let low = SpectrumFrame::new(vec![0.15; 20], 0);
        let high = SpectrumFrame::new(vec![0.85; 20], 16);

        render_bars(&mut first, width, height, &low, &OverlayPosition::Bottom);
        render_bars(&mut second, width, height, &high, &OverlayPosition::Bottom);

        assert_ne!(first, second);
    }

    #[test]
    fn top_position_draws_near_top_edge() {
        let width = 320;
        let height = 96;
        let mut canvas = vec![0; (width * height * 4) as usize];
        let frame = SpectrumFrame::new(vec![0.9; 20], 0);

        render_bars(&mut canvas, width, height, &frame, &OverlayPosition::Top);

        assert!(row_has_opaque_pixels(&canvas, width, 12));
        assert!(!row_has_opaque_pixels(&canvas, width, height - 4));
    }

    #[test]
    fn right_position_draws_near_right_edge() {
        let width = 96;
        let height = 320;
        let mut canvas = vec![0; (width * height * 4) as usize];
        let frame = SpectrumFrame::new(vec![0.9; 20], 0);

        render_bars(&mut canvas, width, height, &frame, &OverlayPosition::Right);

        assert!(column_has_opaque_pixels(&canvas, width, width - 25));
        assert!(!column_has_opaque_pixels(&canvas, width, 4));
    }

    fn row_has_opaque_pixels(canvas: &[u8], width: u32, row: u32) -> bool {
        let start = (row * width * 4) as usize;
        let end = start + (width * 4) as usize;
        canvas[start..end]
            .chunks_exact(4)
            .any(|pixel| pixel[3] != 0)
    }

    fn column_has_opaque_pixels(canvas: &[u8], width: u32, column: u32) -> bool {
        canvas
            .chunks_exact((width * 4) as usize)
            .any(|row| row[(column * 4) as usize + 3] != 0)
    }
}
