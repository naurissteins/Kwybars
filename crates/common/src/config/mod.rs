mod model;
mod parse;

pub use model::*;
pub use parse::{
    default_colors_path, default_config_path, load_color_overrides, load_or_default,
    resolve_image_overlay_path,
};
#[cfg(test)]
pub(crate) use parse::{parse_color_overrides, parse_config};

#[cfg(test)]
mod tests;
