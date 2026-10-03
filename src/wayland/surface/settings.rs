//! new settings for a live surface after a reload

use std::time::{Duration, Instant};

use tracing::debug;

use super::OutputSurface;
use crate::config::SurfaceConfig;
use crate::wayland::placement::Placement;

impl OutputSurface {
    pub fn reconfigure(&mut self, config: SurfaceConfig, now: Instant) -> bool {
        if config == self.config {
            return true;
        }
        if config.overlay.layer != self.config.overlay.layer {
            return false;
        }
        let placement = Placement::new(&config.overlay, config.visualizer.layout);
        if placement != self.placement {
            self.placement = placement;
            // a hidden surface gets it with the next map
            if self.shown {
                self.commit_placement();
            }
            debug!("{}: new placement {:?}", self.label, self.placement);
        }
        self.fade.set_durations(
            Duration::from_millis(config.overlay.fade_in_ms),
            Duration::from_millis(config.overlay.fade_out_ms),
            now,
        );
        self.config = config;
        self.painter = None;
        self.drawn = None;
        true
    }
}
