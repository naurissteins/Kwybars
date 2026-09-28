//! the newest mono samples, downmixed from interleaved input

/// fixed-length ring of mono samples, the newest `len` samples are kept
#[derive(Debug)]
pub struct History {
    samples: Vec<f32>,
    next: usize,
    channels: usize,
    frame_sum: f32,
    frame_fill: usize,
}

impl History {
    pub fn new(len: usize, channels: usize) -> Self {
        Self {
            samples: vec![0.0; len.max(1)],
            next: 0,
            channels: channels.max(1),
            frame_sum: 0.0,
            frame_fill: 0,
        }
    }

    /// adds one interleaved sample, each full frame becomes one mono sample,
    /// the average of its channels
    pub fn push_interleaved(&mut self, sample: f32) {
        self.frame_sum += sample;
        self.frame_fill += 1;
        if self.frame_fill < self.channels {
            return;
        }
        let mono = self.frame_sum / self.channels as f32;
        self.frame_sum = 0.0;
        self.frame_fill = 0;
        if let Some(slot) = self.samples.get_mut(self.next) {
            *slot = mono;
        }
        self.next = (self.next + 1) % self.samples.len();
    }

    /// copies the samples oldest first into `out`, which must be as long as the history
    pub fn copy_ordered(&self, out: &mut [f32]) {
        let (newer, older) = self.samples.split_at(self.next);
        for (slot, sample) in out.iter_mut().zip(older.iter().chain(newer)) {
            *slot = *sample;
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.samples.len()
    }
}

#[cfg(test)]
mod tests {
    use super::History;

    fn ordered(history: &History) -> Vec<f32> {
        let mut out = vec![0.0; history.len()];
        history.copy_ordered(&mut out);
        out
    }

    #[test]
    fn keeps_the_newest_samples_in_order() {
        let mut history = History::new(3, 1);
        for sample in [1.0, 2.0, 3.0, 4.0, 5.0] {
            history.push_interleaved(sample);
        }
        assert_eq!(ordered(&history), vec![3.0, 4.0, 5.0]);
    }

    #[test]
    fn averages_channels_per_frame() {
        let mut history = History::new(2, 2);
        for sample in [1.0, 0.0, 0.5, 0.5, 0.25] {
            history.push_interleaved(sample);
        }
        // the trailing 0.25 is half a frame and not written yet
        assert_eq!(ordered(&history), vec![0.5, 0.5]);
    }
}
