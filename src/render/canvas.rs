//! a mapped buffer of premultiplied 32-bit pixels

use super::PixelRect;

/// bytes of a `width` x `height` buffer with no row padding
pub struct Canvas<'a> {
    data: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> Canvas<'a> {
    /// `None` when `data` is too short for the size
    pub fn new(data: &'a mut [u8], (width, height): (u32, u32)) -> Option<Self> {
        let len = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        (data.len() >= len).then_some(Self {
            data,
            width,
            height,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// makes `area` fully transparent
    pub fn clear(&mut self, area: PixelRect) {
        for y in area.y..area.y.saturating_add(area.height) {
            self.span(y, area.x, area.x.saturating_add(area.width))
                .fill([0; 4]);
        }
    }

    #[inline]
    pub fn pixel(&mut self, x: u32, y: u32) -> Option<&mut [u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = (y as usize * self.width as usize + x as usize) * 4;
        self.data.get_mut(at..at + 4)?.try_into().ok()
    }

    /// the pixels of row `y` from column `x0` up to `x1`, clipped to the buffer
    pub fn span(&mut self, y: u32, x0: u32, x1: u32) -> &mut [[u8; 4]] {
        if y >= self.height {
            return &mut [];
        }
        let x1 = x1.min(self.width);
        let x0 = x0.min(x1);
        let row = y as usize * self.width as usize;
        let range = (row + x0 as usize) * 4..(row + x1 as usize) * 4;
        match self.data.get_mut(range) {
            Some(bytes) => bytes.as_chunks_mut::<4>().0,
            None => &mut [],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Canvas;
    use crate::render::PixelRect;

    #[test]
    fn clear_and_spans_stay_inside_the_buffer() {
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
        assert_eq!(canvas.span(1, 0, 9), &[[7; 4], [0; 4], [0; 4]]);
        assert_eq!(canvas.span(0, 0, 3), &[[7; 4]; 3]);
        assert!(canvas.span(2, 0, 3).is_empty());
        assert!(canvas.span(0, 2, 1).is_empty());
    }

    #[test]
    fn wrong_sized_data_is_refused() {
        let mut data = vec![0_u8; 10];
        assert!(Canvas::new(&mut data, (4, 4)).is_none());
    }
}
