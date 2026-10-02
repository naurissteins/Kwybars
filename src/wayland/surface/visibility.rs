//! mapping a surface when audio plays and unmapping it once faded out

use std::time::Instant;

use smithay_client_toolkit::shell::WaylandSurface;
use tracing::debug;

use super::OutputSurface;
use super::layer::place;
use crate::render::Painter;

impl OutputSurface {
    pub fn set_active(&mut self, active: bool, now: Instant) {
        self.fade.set_visible(active, now);
        if active && !self.shown {
            self.map();
        } else if !active && self.shown && self.fade.opacity(now) <= 0.0 {
            self.unmap();
        }
    }

    pub fn hide_deadline(&self) -> Option<Instant> {
        if self.shown && !self.fade.is_visible() {
            self.fade.end()
        } else {
            None
        }
    }

    /// mapped, or asked to be
    pub fn is_shown(&self) -> bool {
        self.shown
    }

    /// whether the opacity still changes after `now`
    pub fn is_fading(&self, now: Instant) -> bool {
        self.shown && self.fade.is_fading(now)
    }

    /// shown and drawing something that moves on its own
    pub fn is_animated(&self) -> bool {
        self.shown && self.painter.as_ref().is_some_and(Painter::animates)
    }

    fn map(&mut self) {
        place(&self.layer, &self.placement);
        self.layer.commit();
        self.shown = true;
        debug!("{}: showing", self.label);
    }

    /// detaches the buffer and frees every buffer and the painter
    fn unmap(&mut self) {
        self.layer.wl_surface().attach(None, 0, 0);
        self.layer.commit();
        self.shown = false;
        self.configured = None;
        self.applied = None;
        self.drawn = None;
        self.frame_pending = false;
        self.ring.release();
        self.painter = None;
        debug!("{}: hidden, buffers released", self.label);
    }
}
