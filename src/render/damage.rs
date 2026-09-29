//! whole-pixel rectangles for clearing and damage

use tiny_skia::Rect;

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

    /// the pixels `rect` touches, clipped to a buffer of `size`; `None` when
    /// nothing is left
    pub fn covering(rect: Rect, (width, height): (u32, u32)) -> Option<Self> {
        let clamp = |value: f32, max: u32| value.clamp(0.0, max as f32) as u32;
        let (left, top) = (
            clamp(rect.left().floor(), width),
            clamp(rect.top().floor(), height),
        );
        let right = clamp(rect.right().ceil(), width);
        let bottom = clamp(rect.bottom().ceil(), height);
        (right > left && bottom > top).then_some(Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }

    /// the smallest rectangle holding both
    pub fn union(self, other: Self) -> Self {
        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }
}

/// pixels that change when a bar at `old` is replaced by one at `new`;
/// `None` when nothing changes
pub fn change(old: Option<Rect>, new: Option<Rect>, size: (u32, u32)) -> Option<PixelRect> {
    if old == new {
        return None;
    }
    let area = match (old, new) {
        (Some(old), Some(new)) => moved_edge(old, new).unwrap_or_else(|| union(old, new)),
        (Some(rect), None) | (None, Some(rect)) => rect,
        (None, None) => return None,
    };
    PixelRect::covering(area, size)
}

/// the band between the one edge that differs, when the other three match
fn moved_edge(a: Rect, b: Rect) -> Option<Rect> {
    let band =
        |left: f32, top: f32, right: f32, bottom: f32| Rect::from_ltrb(left, top, right, bottom);
    let same = |x: f32, y: f32| x == y;
    let columns = same(a.left(), b.left()) && same(a.right(), b.right());
    let rows = same(a.top(), b.top()) && same(a.bottom(), b.bottom());
    if columns && same(a.bottom(), b.bottom()) {
        band(
            a.left(),
            a.top().min(b.top()),
            a.right(),
            a.top().max(b.top()),
        )
    } else if columns && same(a.top(), b.top()) {
        band(
            a.left(),
            a.bottom().min(b.bottom()),
            a.right(),
            a.bottom().max(b.bottom()),
        )
    } else if rows && same(a.left(), b.left()) {
        band(
            a.right().min(b.right()),
            a.top(),
            a.right().max(b.right()),
            a.bottom(),
        )
    } else if rows && same(a.right(), b.right()) {
        band(
            a.left().min(b.left()),
            a.top(),
            a.left().max(b.left()),
            a.bottom(),
        )
    } else {
        None
    }
}

fn union(a: Rect, b: Rect) -> Rect {
    Rect::from_ltrb(
        a.left().min(b.left()),
        a.top().min(b.top()),
        a.right().max(b.right()),
        a.bottom().max(b.bottom()),
    )
    .unwrap_or(a)
}

impl PixelRect {
    /// the same area as a float rectangle, for clipping shapes
    pub fn to_rect(self) -> Option<Rect> {
        Rect::from_xywh(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
        )
    }
}

#[cfg(test)]
mod tests {
    use tiny_skia::Rect;

    use super::{PixelRect, change};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Option<Rect> {
        Rect::from_xywh(x, y, w, h)
    }

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
            rect(2.5, 10.2, 3.0, 5.0).and_then(|r| PixelRect::covering(r, size)),
            Some(pixels(2, 10, 4, 6))
        );
        assert_eq!(
            rect(-5.0, 40.0, 10.0, 20.0).and_then(|r| PixelRect::covering(r, size)),
            Some(pixels(0, 40, 5, 10))
        );
        assert_eq!(
            rect(200.0, 0.0, 10.0, 10.0).and_then(|r| PixelRect::covering(r, size)),
            None
        );
    }

    #[test]
    fn a_moving_bar_damages_only_the_band_it_moved_through() {
        let size = (100, 100);
        let (short, tall) = (rect(10.0, 80.0, 4.0, 20.0), rect(10.0, 30.5, 4.0, 69.5));
        assert_eq!(change(short, tall, size), Some(pixels(10, 30, 4, 50)));
        assert_eq!(change(tall, short, size), Some(pixels(10, 30, 4, 50)));
        // top edge bars grow downwards, side bars sideways
        let (top_a, top_b) = (rect(0.0, 0.0, 4.0, 10.0), rect(0.0, 0.0, 4.0, 12.5));
        assert_eq!(change(top_a, top_b, size), Some(pixels(0, 10, 4, 3)));
        let (right_a, right_b) = (rect(90.0, 5.0, 10.0, 4.0), rect(85.2, 5.0, 14.8, 4.0));
        assert_eq!(change(right_a, right_b, size), Some(pixels(85, 5, 5, 4)));
    }

    #[test]
    fn appearing_vanishing_and_unrelated_rects() {
        let size = (100, 100);
        let tall = rect(10.0, 30.5, 4.0, 69.5);
        assert_eq!(change(tall, None, size), Some(pixels(10, 30, 4, 70)));
        assert_eq!(change(None, tall, size), Some(pixels(10, 30, 4, 70)));
        assert_eq!(change(tall, tall, size), None);
        assert_eq!(change(None, None, size), None);
        let elsewhere = rect(50.0, 0.0, 4.0, 4.0);
        assert_eq!(change(tall, elsewhere, size), Some(pixels(10, 0, 44, 100)));
    }
}
