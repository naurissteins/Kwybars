use std::f64::consts::TAU;

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

pub fn render_fake_bars(canvas: &mut [u8], width: u32, height: u32, phase: f64) {
    clear(canvas);

    if width == 0 || height == 0 {
        return;
    }

    let drawable_width = width.saturating_sub(HORIZONTAL_PADDING * 2);
    let drawable_height = height.saturating_sub(VERTICAL_PADDING * 2);
    if drawable_width == 0 || drawable_height < MIN_BAR_HEIGHT {
        return;
    }

    let bar_count = ((drawable_width / 28).clamp(12, 48)) as usize;
    let gap = (drawable_width / 180).max(3);
    let total_gap = gap.saturating_mul(bar_count.saturating_sub(1) as u32);
    let bar_width =
        ((drawable_width.saturating_sub(total_gap)) / bar_count as u32).max(MIN_BAR_WIDTH);
    let occupied_width = bar_width * bar_count as u32 + total_gap;
    let start_x = ((width.saturating_sub(occupied_width)) / 2) as i32;
    let baseline_y = (height.saturating_sub(VERTICAL_PADDING)) as i32;

    for index in 0..bar_count {
        let x = start_x + index as i32 * (bar_width + gap) as i32;
        let bar_height = fake_bar_height(index, bar_count, drawable_height, phase);
        fill_rect(
            canvas,
            width,
            height,
            Rect {
                x,
                y: baseline_y - bar_height as i32,
                width: bar_width,
                height: bar_height,
            },
            BAR_COLOR,
        );
    }
}

fn fake_bar_height(index: usize, bar_count: usize, drawable_height: u32, phase: f64) -> u32 {
    let relative = if bar_count <= 1 {
        0.0
    } else {
        index as f64 / (bar_count - 1) as f64
    };
    let wave_phase = relative * TAU;
    let primary = (wave_phase * 1.15 + phase).sin() * 0.5 + 0.5;
    let secondary = (wave_phase * 0.55 - phase * 1.4 + 0.8).cos() * 0.5 + 0.5;
    let tertiary = (wave_phase * 2.1 + phase * 0.7).sin() * 0.5 + 0.5;
    let amplitude = primary * 0.55 + secondary * 0.3 + tertiary * 0.15;
    let normalized = 0.16 + amplitude * 0.76;
    let height = (drawable_height as f64 * normalized).round() as u32;
    height.max(MIN_BAR_HEIGHT).min(drawable_height)
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
    use super::render_fake_bars;

    #[test]
    fn fake_bars_leave_transparent_background() {
        let width = 320;
        let height = 96;
        let mut canvas = vec![0xAA; (width * height * 4) as usize];

        render_fake_bars(&mut canvas, width, height, 0.0);

        assert_eq!(&canvas[0..4], &[0, 0, 0, 0]);
        assert!(canvas.chunks_exact(4).any(|pixel| pixel[3] != 0));
    }

    #[test]
    fn fake_bars_handle_small_buffers() {
        let mut canvas = vec![0; 4 * 8 * 8];
        render_fake_bars(&mut canvas, 8, 8, 0.0);
        assert_eq!(canvas.len(), 256);
    }

    #[test]
    fn fake_bars_change_across_animation_phases() {
        let width = 320;
        let height = 96;
        let mut first = vec![0; (width * height * 4) as usize];
        let mut second = vec![0; (width * height * 4) as usize];

        render_fake_bars(&mut first, width, height, 0.0);
        render_fake_bars(&mut second, width, height, 1.7);

        assert_ne!(first, second);
    }
}
