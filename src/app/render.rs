//! drawing and activity deadlines on the main loop

use super::deadline::{self, Change};
use super::{App, frame_time};
use calloop::timer::{TimeoutAction, Timer};
use std::time::Instant;
use tracing::{info, warn};

impl App {
    pub(super) fn render(&mut self) {
        let now = Instant::now();
        let level = self.animation.level();
        if self.activity.update(now, level) {
            info!(
                "audio {}",
                if self.activity.is_active() {
                    "active, showing"
                } else {
                    "inactive, hiding"
                }
            );
        }
        let presence = self.wayland.set_active(self.activity.is_active(), now);
        let audio = self.capture.frames().read_into(&mut []);
        let background_moving = self.wayland.render_backgrounds(now, audio.bass);
        let frame = self.animation.frame(now, presence);
        let animating = frame.animating || background_moving;
        let drawn = self.wayland.render(&frame);
        self.animation.drawn(drawn);
        self.background_again = None;
        if self.wayland.has_backgrounds()
            && !animating
            && !self.capture.frames().request_wake(audio.number)
        {
            self.background_again = Some(now + frame_time(self.config.visualizer.framerate));
        }
        self.watch_deadline();
    }

    fn watch_deadline(&mut self) {
        let deadlines = [
            self.activity.deadline(),
            self.wayland.deadline(),
            self.background_again,
        ];
        let wanted = deadlines.into_iter().flatten().min();
        let action = deadline::change(self.timer.map(|(_, at)| at), wanted, Instant::now());
        if action == Change::Keep {
            return;
        }
        if let Some((token, _)) = self.timer.take() {
            self.handle.remove(token);
        }
        let Change::Arm(deadline) = action else {
            return;
        };
        let timer = Timer::from_deadline(deadline);
        match self.handle.insert_source(timer, |_, (), app| {
            app.timer = None;
            app.render();
            TimeoutAction::Drop
        }) {
            Ok(token) => self.timer = Some((token, deadline)),
            Err(err) => warn!("could not wait for the activity deadline: {}", err.error),
        }
    }
}
