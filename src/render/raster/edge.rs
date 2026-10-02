use std::f32::consts::FRAC_1_SQRT_2;

/// how a straight edge with one normal covers square pixels; exact for a
/// square pixel, and a linear ramp for an axis-aligned edge
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ramp {
    /// how far past the edge a pixel center can be and still be touched
    pub reach: f32,
    inner: f32,
    slope: f32,
    curve: f32,
}

impl Ramp {
    /// for an edge whose unit normal is nx, ny
    pub fn new(nx: f32, ny: f32) -> Self {
        let (x, y) = (nx.abs(), ny.abs());
        let (a, b) = (x.max(y), x.min(y));
        Self {
            reach: (a + b) * 0.5,
            inner: (a - b) * 0.5,
            slope: 1.0 / a.max(f32::EPSILON),
            // only used between inner and reach, which are equal when b is 0
            curve: 1.0 / (2.0 * a * b).max(f32::EPSILON),
        }
    }

    /// share of a pixel whose center is d inside the edge (negative outside)
    #[inline]
    pub fn cover(&self, d: f32) -> f32 {
        if d <= -self.reach {
            0.0
        } else if d >= self.reach {
            1.0
        } else if d <= -self.inner {
            (d + self.reach) * (d + self.reach) * self.curve
        } else if d < self.inner {
            0.5 + d * self.slope
        } else {
            1.0 - (self.reach - d) * (self.reach - d) * self.curve
        }
    }

    /// share of a pixel between two parallel edges, low and high inside them
    #[inline]
    pub fn between(&self, low: f32, high: f32) -> f32 {
        if low >= self.reach && high >= self.reach {
            return 1.0;
        }
        (self.cover(low) + self.cover(high) - 1.0).max(0.0)
    }
}

/// share of a pixel inside a circle of radius around a center dx, dy away
/// from the pixel center, in buffer axes
pub fn circle(dx: f32, dy: f32, radius: f32) -> f32 {
    let squared = dx * dx + dy * dy;
    // no ramp reaches further than half a pixel's diagonal
    let (inside, outside) = (radius - FRAC_1_SQRT_2, radius + FRAC_1_SQRT_2);
    if inside > 0.0 && squared <= inside * inside {
        return 1.0;
    }
    if squared >= outside * outside {
        return 0.0;
    }
    let distance = squared.sqrt();
    if distance <= 0.0 {
        return 1.0;
    }
    Ramp::new(dx / distance, dy / distance).cover(radius - distance)
}

#[cfg(test)]
mod tests {
    use super::{Ramp, circle};

    /// the share of the unit pixel centered at the origin with n . p <= d
    fn sampled(d: f32, angle: f32) -> f32 {
        let (nx, ny) = (angle.cos(), angle.sin());
        let mut hits = 0;
        for sy in 0..64 {
            for sx in 0..64 {
                let (x, y) = (
                    (sx as f32 + 0.5) / 64.0 - 0.5,
                    (sy as f32 + 0.5) / 64.0 - 0.5,
                );
                hits += u32::from(nx * x + ny * y <= d);
            }
        }
        hits as f32 / 4096.0
    }

    #[test]
    fn matches_a_sampled_pixel_at_any_angle() {
        for step in 0..=18 {
            let angle = (step as f32 * 5.0).to_radians();
            let ramp = Ramp::new(angle.cos(), angle.sin());
            for tenth in -9..=9 {
                let d = tenth as f32 / 10.0;
                let (exact, sampled) = (ramp.cover(d), sampled(d, angle));
                assert!(
                    (exact - sampled).abs() < 0.02,
                    "{angle} rad, {d}: {exact} vs {sampled}"
                );
            }
        }
    }

    #[test]
    fn an_axis_aligned_edge_is_a_linear_ramp() {
        for tenth in -10..=10 {
            let d = tenth as f32 / 10.0;
            assert!((Ramp::new(0.0, -1.0).cover(d) - (0.5 + d).clamp(0.0, 1.0)).abs() < 1e-6);
        }
    }

    #[test]
    fn a_circle_is_whole_inside_and_empty_outside() {
        assert_eq!(circle(0.0, 0.0, 3.0), 1.0);
        assert_eq!(circle(1.0, 1.0, 3.0), 1.0);
        assert_eq!(circle(3.0, 3.0, 3.0), 0.0);
        assert!((circle(3.0, 0.0, 3.0) - 0.5).abs() < 1e-6);
    }
}
