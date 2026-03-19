#[derive(Clone, Copy)]
pub struct BarStyle {
    pub thickness: f64,
    pub gap: f64,
    pub corner_radius: f64,
    pub segmented: bool,
    pub segment_length: f64,
    pub segment_gap: f64,
}

#[derive(Clone, Copy)]
pub enum BarOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
pub struct BarRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy)]
pub struct RadialBarSpec {
    pub angle: f64,
    pub inner_radius: f64,
    pub length: f64,
    pub thickness: f64,
}

#[derive(Clone, Copy)]
pub struct RadialLayout {
    pub width: f64,
    pub height: f64,
    pub inner_radius: f64,
    pub start_angle: f64,
    pub arc_radians: f64,
}

#[derive(Clone, Copy)]
pub struct PolygonLayout {
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub rotation_radians: f64,
    pub sides: usize,
}

#[derive(Clone, Copy)]
pub struct DirectedBarSpec {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub length: f64,
    pub thickness: f64,
}

#[derive(Clone, Copy)]
pub struct FrameEdgeRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub orientation: BarOrientation,
    pub from_start: bool,
}

#[derive(Clone, Copy)]
pub struct ParticleSpec {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

#[derive(Clone, Copy)]
pub struct FloatingParticleLayout {
    pub width: f64,
    pub height: f64,
    pub max_radius: f64,
    pub gap: f64,
    pub orientation: BarOrientation,
    pub from_start: bool,
}

#[derive(Clone, Copy)]
pub enum LinearBarMode {
    Continuous,
    Split { center_gap: f64 },
}

#[derive(Clone, Copy)]
pub struct HorizontalBarLayout {
    pub width: f64,
    pub height: f64,
    pub bar_thickness: f64,
    pub gap: f64,
    pub from_top: bool,
    pub mode: LinearBarMode,
}

#[derive(Clone, Copy)]
pub struct VerticalBarLayout {
    pub width: f64,
    pub height: f64,
    pub bar_thickness: f64,
    pub gap: f64,
    pub from_left: bool,
    pub mode: LinearBarMode,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RadialDistribution {
    pub(crate) first_angle: f64,
    pub(crate) angle_step: f64,
    pub(crate) tangential_thickness: f64,
}

#[derive(Clone, Copy)]
pub(crate) struct Point {
    pub(crate) x: f64,
    pub(crate) y: f64,
}
