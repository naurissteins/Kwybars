use super::Turned;

/// the columns of each row whose pixel centers lie within limits of a
/// shape's middle, along and across it
pub struct Rows {
    start: (f32, f32),
    axis: (f32, f32),
    half_length: f32,
    /// 1 / cos and -1 / sin, none where the axis makes them huge
    along: Option<f32>,
    across: Option<f32>,
}

impl Rows {
    pub fn new(shape: &Turned) -> Self {
        let (cos, sin) = shape.axis;
        let inverse = |k: f32| (k.abs() >= 1e-6).then(|| 1.0 / k);
        Self {
            start: shape.start,
            axis: shape.axis,
            half_length: shape.length.max(0.0) * 0.5,
            along: inverse(cos),
            across: inverse(-sin),
        }
    }

    /// columns inside x0..x1 of the row dy below the start
    #[inline]
    pub fn span(&self, dy: f32, (limit_u, limit_v): (f32, f32), x0: u32, x1: u32) -> (u32, u32) {
        if limit_u < 0.0 || limit_v < 0.0 {
            return (x1, x1);
        }
        let (cos, sin) = self.axis;
        // along: u - half = dx cos + dy sin - half; across: v = -dx sin + dy cos
        let (a, b) = slab(self.along, dy * sin - self.half_length, limit_u);
        let (c, d) = slab(self.across, dy * cos, limit_v);
        let (low, high) = (a.max(c), b.min(d));
        // pixel x has its center at x + 0.5; clamped first, so both are
        // whole or positive and a cast rounds them down
        let (x0, x1) = (x0 as f32, x1 as f32);
        let low = (self.start.0 + low - 0.5).clamp(x0, x1);
        let high = (self.start.0 + high + 0.5).clamp(low, x1);
        let first = low as u32 + u32::from((low as u32 as f32) < low);
        (first, (high as u32).max(first))
    }
}

/// the range of dx where |k dx + m| <= limit, given 1 / k
#[inline]
fn slab(inverse: Option<f32>, m: f32, limit: f32) -> (f32, f32) {
    let Some(inverse) = inverse else {
        return if m.abs() <= limit {
            (f32::NEG_INFINITY, f32::INFINITY)
        } else {
            (f32::INFINITY, f32::NEG_INFINITY)
        };
    };
    let (a, b) = ((-limit - m) * inverse, (limit - m) * inverse);
    (a.min(b), a.max(b))
}

/// one interval holding both when they meet, else the longer one
pub fn merge(a: (u32, u32), b: (u32, u32)) -> (u32, u32) {
    let (a, b) = if a.0 < a.1 { (a, b) } else { (b, a) };
    if b.0 >= b.1 {
        a
    } else if a.0 <= b.1 && b.0 <= a.1 {
        (a.0.min(b.0), a.1.max(b.1))
    } else if a.1 - a.0 >= b.1 - b.0 {
        a
    } else {
        b
    }
}
