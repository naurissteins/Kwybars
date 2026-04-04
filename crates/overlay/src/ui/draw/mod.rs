mod centered;
mod linear;
mod particles;
mod path;
mod types;
mod wave;

pub use centered::{for_each_polygon_bar, for_each_radial_bar};
pub use linear::{
    bar_color_index, distributed_chunk, draw_horizontal_bars_mode,
    draw_horizontal_mirror_bars_mode, draw_vertical_bars_mode, draw_vertical_mirror_bars_mode,
    for_each_horizontal_bar, for_each_horizontal_bar_mode, for_each_horizontal_mirror_bar_mode,
    for_each_vertical_bar, for_each_vertical_bar_mode, for_each_vertical_mirror_bar_mode,
};
pub use particles::{for_each_floating_particle, for_each_particle};
pub use path::{append_bar_path, append_directed_bar_path, append_radial_bar_path, draw_particle};
pub use types::{
    BarOrientation, BarRect, BarStyle, FloatingParticleLayout, FrameEdgeRect, HorizontalBarLayout,
    LinearBarMode, MirrorHorizontalLayout, MirrorVerticalLayout, PolygonLayout, RadialLayout,
    VerticalBarLayout, WaveLayout,
};
pub use wave::{
    append_horizontal_wave_fill_path, append_horizontal_wave_path, append_vertical_wave_fill_path,
    append_vertical_wave_path,
};

#[cfg(test)]
pub(crate) use centered::radial_distribution;
#[cfg(test)]
pub(crate) use path::for_each_segment_span;
#[cfg(test)]
pub(crate) use wave::{curve_control_scale, horizontal_wave_points, vertical_wave_points};

#[cfg(test)]
mod tests;
