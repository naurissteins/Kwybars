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

    /// the smallest rectangle holding both
    pub fn union(self, other: Self) -> Self {
        let (x, y) = (self.x.min(other.x), self.y.min(other.y));
        Self {
            x,
            y,
            width: self.right().max(other.right()) - x,
            height: self.bottom().max(other.bottom()) - y,
        }
    }

    pub fn intersect(self, other: Self) -> Option<Self> {
        let (x, y) = (self.x.max(other.x), self.y.max(other.y));
        let (right, bottom) = (
            self.right().min(other.right()),
            self.bottom().min(other.bottom()),
        );
        (right > x && bottom > y).then(|| Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }

    /// the parts of this rectangle outside other, as up to four rectangles
    pub fn minus(self, other: Self, mut each: impl FnMut(Self)) {
        let Some(shared) = self.intersect(other) else {
            return each(self);
        };
        let mut strip = |x: u32, y: u32, right: u32, bottom: u32| {
            if right > x && bottom > y {
                each(Self {
                    x,
                    y,
                    width: right - x,
                    height: bottom - y,
                });
            }
        };
        strip(self.x, self.y, self.right(), shared.y);
        strip(self.x, shared.bottom(), self.right(), self.bottom());
        strip(self.x, shared.y, shared.x, shared.bottom());
        strip(shared.right(), shared.y, self.right(), shared.bottom());
    }

    pub fn pixels(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
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

    #[test]
    fn intersect_keeps_the_shared_pixels() {
        let a = pixels(2, 10, 4, 6);
        assert_eq!(a.intersect(pixels(4, 0, 10, 12)), Some(pixels(4, 10, 2, 2)));
        assert_eq!(a.intersect(pixels(6, 10, 3, 3)), None);
        assert_eq!(a.pixels(), 24);
    }

    #[test]
    fn minus_leaves_the_strips_around_the_shared_part() {
        let mut parts = Vec::new();
        pixels(0, 0, 10, 10).minus(pixels(2, 3, 4, 20), |part| parts.push(part));
        assert_eq!(
            parts,
            vec![pixels(0, 0, 10, 3), pixels(0, 3, 2, 7), pixels(6, 3, 4, 7)]
        );
        parts.clear();
        pixels(0, 0, 4, 4).minus(pixels(8, 8, 2, 2), |part| parts.push(part));
        assert_eq!(parts, vec![pixels(0, 0, 4, 4)]);
        parts.clear();
        pixels(1, 1, 2, 2).minus(pixels(0, 0, 9, 9), |part| parts.push(part));
        assert!(parts.is_empty());
    }

    #[test]
    fn union_holds_both() {
        assert_eq!(
            pixels(2, 10, 4, 6).union(pixels(3, 1, 1, 2)),
            pixels(2, 1, 4, 15)
        );
    }
}
