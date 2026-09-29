//! when the overlay shows: audio activity with delays, and the fade towards it

mod fade;
mod tracker;

pub use fade::Fade;
pub use tracker::ActivityTracker;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Presence {
    pub shown: bool,
    pub fading: bool,
}
