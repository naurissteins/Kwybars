use super::Turned;
use crate::render::raster::edge::{self, Ramp};

/// coverage of pixels of one shape, from their centers in its axes
#[derive(Debug, Clone, Copy)]
pub struct Pixels {
    pub ramp: Ramp,
    axis: (f32, f32),
    length: f32,
    pub hu: f32,
    pub hv: f32,
    pub radius: f32,
    /// only the sides count: the shape runs on past both ends
    pub open: bool,
}

impl Pixels {
    pub fn new(shape: &Turned) -> Self {
        let (length, hv) = (shape.length.max(0.0), shape.half_width.max(0.0));
        let hu = length * 0.5;
        Self {
            ramp: Ramp::new(shape.axis.0, shape.axis.1),
            axis: shape.axis,
            length,
            hu,
            hv,
            radius: shape.radius.min(hu).min(hv).max(0.0),
            open: false,
        }
    }

    /// the pixel centered at u along from the start and v across
    #[inline]
    pub fn cover(&self, u: f32, v: f32) -> f32 {
        let (length, hv, radius) = (self.length, self.hv, self.radius);
        if self.open {
            return self.ramp.between(v + hv, hv - v);
        }
        // the sides are square to the axis, so one ramp fits all four
        let rect = self.ramp.between(u, length - u) * self.ramp.between(v + hv, hv - v);
        if radius <= 0.0 || rect <= 0.0 {
            return rect;
        }
        let du = (radius - u).max(u - (length - radius));
        let dv = v.abs() - (hv - radius);
        if du <= 0.0 || dv <= 0.0 {
            return rect;
        }
        // back to buffer axes, where the pixel is square
        let (cos, sin) = self.axis;
        let ou = if u < radius { -du } else { du };
        let ov = dv.copysign(v);
        let (dx, dy) = (ou * cos - ov * sin, ou * sin + ov * cos);
        rect.min(edge::circle(dx, dy, radius))
    }
}
