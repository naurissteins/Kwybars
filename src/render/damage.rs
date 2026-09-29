//! whole-pixel rectangles for clearing and damage

/// a rectangle of whole pixels inside a buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    /// the whole buffer
    pub fn full((width, height): (u32, u32)) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    /// the pixels touched by `left..right` x `top..bottom`, clipped to a
    /// buffer of `size`; `None` when nothing is left
    pub fn covering(
        (left, top, right, bottom): (f32, f32, f32, f32),
        (width, height): (u32, u32),
    ) -> Option<Self> {
        let clamp = |value: f32, max: u32| value.clamp(0.0, max as f32) as u32;
        let (x0, y0) = (clamp(left.floor(), width), clamp(top.floor(), height));
        let (x1, y1) = (clamp(right.ceil(), width), clamp(bottom.ceil(), height));
        (x1 > x0 && y1 > y0).then_some(Self {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        })
    }

    pub fn right(self) -> u32 {
        self.x + self.width
    }

    pub fn bottom(self) -> u32 {
        self.y + self.height
    }
}

#[cfg(test)]
mod tests {
    use super::PixelRect;

    fn pixels(x: u32, y: u32, width: u32, height: u32) -> PixelRect {
        PixelRect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn covering_rounds_out_and_clips() {
        let size = (100, 50);
        assert_eq!(
            PixelRect::covering((2.5, 10.2, 5.5, 15.2), size),
            Some(pixels(2, 10, 4, 6))
        );
        assert_eq!(
            PixelRect::covering((-5.0, 40.0, 5.0, 60.0), size),
            Some(pixels(0, 40, 5, 10))
        );
        assert_eq!(PixelRect::covering((200.0, 0.0, 210.0, 10.0), size), None);
        assert_eq!(PixelRect::covering((3.0, 3.0, 3.0, 9.0), size), None);
    }
}
