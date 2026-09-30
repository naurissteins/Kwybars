//! where a strip of bars sits in the buffer

use crate::config::Edge;

/// a rectangle of the buffer, in buffer pixels
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Region {
    pub fn whole((width, height): (u32, u32)) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
        }
    }
}

/// where a strip sits: bars grow from `edge` of `region` into it
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strip {
    pub edge: Edge,
    pub region: Region,
    pub min_extent: f32,
}
