//! a buffer of premultiplied 32-bit pixels drawn with tiny-skia

use tiny_skia::{Paint, PixmapMut, Rect, Transform};

use super::PixelRect;

/// drawing target over a mapped buffer
pub struct Canvas<'a> {
    pixmap: PixmapMut<'a>,
}

impl<'a> Canvas<'a> {
    /// `data` holds `width * height` pixels with no row padding
    pub fn new(data: &'a mut [u8], (width, height): (u32, u32)) -> Option<Self> {
        Some(Self {
            pixmap: PixmapMut::from_bytes(data, width, height)?,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.pixmap.width(), self.pixmap.height())
    }

    /// makes `area` fully transparent
    pub fn clear(&mut self, area: PixelRect) {
        self.rows(area, |row| row.fill(0));
    }

    pub fn fill(&mut self, rect: Rect, paint: &Paint<'_>) {
        self.pixmap
            .fill_rect(rect, paint, Transform::identity(), None);
    }

    /// calls `each` with the bytes of every row of `area` inside the buffer
    fn rows(&mut self, area: PixelRect, mut each: impl FnMut(&mut [u8])) {
        let width = self.pixmap.width() as usize;
        let height = self.pixmap.height() as usize;
        let left = (area.x as usize).min(width);
        let right = (area.x as usize + area.width as usize).min(width);
        let bottom = (area.y as usize + area.height as usize).min(height);
        let data = self.pixmap.data_mut();
        for y in (area.y as usize).min(bottom)..bottom {
            let start = (y * width + left) * 4;
            let end = (y * width + right) * 4;
            if let Some(row) = data.get_mut(start..end) {
                each(row);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tiny_skia::{Color, Paint, Rect};

    use super::Canvas;
    use crate::render::PixelRect;

    fn red() -> Paint<'static> {
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(255, 0, 0, 128));
        paint
    }

    fn pixel(data: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
        let at = (y * width + x) * 4;
        [data[at], data[at + 1], data[at + 2], data[at + 3]]
    }

    #[test]
    fn draws_premultiplied_rgba() {
        let mut data = vec![0_u8; 4 * 4 * 4];
        let (Some(mut canvas), Some(rect)) = (
            Canvas::new(&mut data, (4, 4)),
            Rect::from_xywh(1.0, 1.0, 2.0, 2.0),
        ) else {
            panic!("canvas and rect");
        };
        canvas.fill(rect, &red());
        assert_eq!(pixel(&data, 4, 1, 1), [128, 0, 0, 128]);
        assert_eq!(pixel(&data, 4, 0, 0), [0; 4]);
        assert_eq!(pixel(&data, 4, 3, 3), [0; 4]);
    }

    #[test]
    fn clear_touches_only_its_area() {
        let mut data = vec![7_u8; 3 * 2 * 4];
        let Some(mut canvas) = Canvas::new(&mut data, (3, 2)) else {
            panic!("canvas");
        };
        canvas.clear(PixelRect {
            x: 1,
            y: 1,
            width: 5,
            height: 5,
        });
        assert_eq!(pixel(&data, 3, 0, 1), [7; 4]);
        assert_eq!(pixel(&data, 3, 1, 1), [0; 4]);
        assert_eq!(pixel(&data, 3, 2, 1), [0; 4]);
        assert_eq!(pixel(&data, 3, 1, 0), [7; 4]);
    }

    #[test]
    fn wrong_sized_data_is_refused() {
        let mut data = vec![0_u8; 10];
        assert!(Canvas::new(&mut data, (4, 4)).is_none());
    }
}
