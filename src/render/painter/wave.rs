use super::{BufferContents, Painter};
use crate::render::wave::Wave;
use crate::render::{Canvas, PixelRect};

impl Painter {
    /// the wave is one shape: a buffer is brought up to it around the curve
    pub(super) fn paint_wave(
        &self,
        wave: &Wave,
        canvas: &mut Canvas<'_>,
        contents: &mut BufferContents,
    ) {
        let fresh = contents.painter != self.id;
        if fresh {
            canvas.clear(PixelRect::full(canvas.size()));
            contents.wave.clear();
        } else if contents.version == wave.version() && contents.opacity == self.opacity {
            return;
        }
        // a new opacity changes every pixel the wave draws
        let whole = fresh || contents.opacity != self.opacity;
        wave.paint(canvas, &self.fills, &mut contents.wave, whole);
        contents.version = wave.version();
        contents.opacity = self.opacity;
        contents.painter = self.id;
    }

    pub(super) fn present_wave(&mut self, each: &mut impl FnMut(PixelRect)) {
        let Some(wave) = &self.wave else {
            return;
        };
        let faded = self.shown_opacity != self.opacity;
        let now = if faded {
            wave.bounds()
        } else {
            wave.curve_area()
        };
        let before = if faded {
            self.shown_wave.1
        } else {
            self.shown_wave.2
        };
        if !self.shown_valid {
            each(PixelRect::full(self.layout.size()));
        } else {
            // where the wave was and where it is
            match (before, now) {
                (Some(old), Some(new)) => each(old.union(new)),
                (Some(area), None) | (None, Some(area)) => each(area),
                (None, None) => {}
            }
        }
        self.shown_wave = (wave.version(), wave.bounds(), wave.curve_area());
        self.shown_opacity = self.opacity;
        self.shown_valid = true;
    }
}
