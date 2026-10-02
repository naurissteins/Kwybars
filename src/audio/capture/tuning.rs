//! what the capture thread analyzes with, replaced on reload without reconnecting

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use super::CaptureSettings;
use crate::audio::frame::FrameSlot;

/// current settings and the slot frames go to, only touched on the capture thread
#[derive(Debug)]
pub(super) struct Tuning {
    pub settings: CaptureSettings,
    pub frames: Arc<FrameSlot>,
    pub generation: u64,
}

pub(super) type SharedTuning = Rc<RefCell<Tuning>>;
