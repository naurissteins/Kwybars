//! state owned by the stream callbacks: sample ring, analysis pacing, publishing

use std::sync::Arc;
use std::time::{Duration, Instant};

use pipewire::spa::param::audio::AudioInfoRaw;
use pipewire::spa::pod::Pod;
use tracing::debug;

use super::Shared;
use super::ring::SampleRing;
use super::status::CaptureState;
use super::tuning::SharedTuning;
use crate::audio::pipeline::Pipeline;

/// seconds of audio the ring holds
const RING_SECONDS: usize = 1;

/// everything the callbacks of one stream share, all on the capture thread
pub(super) struct StreamData {
    shared: Arc<Shared>,
    tuning: SharedTuning,
    generation: u64,
    format: AudioInfoRaw,
    ring: SampleRing,
    /// created when the format is known
    pipeline: Option<Pipeline>,
    channels: u64,
    /// ring position the last analysis stopped at
    cursor: u64,
    pacing: Pacing,
}

impl StreamData {
    pub(super) fn new(shared: Arc<Shared>, tuning: SharedTuning) -> Self {
        let (interval, generation) = {
            let tuning = tuning.borrow();
            (tuning.settings.interval, tuning.generation)
        };
        Self {
            pacing: Pacing::new(interval),
            shared,
            tuning,
            generation,
            format: AudioInfoRaw::default(),
            ring: SampleRing::default(),
            pipeline: None,
            channels: 1,
            cursor: 0,
        }
    }

    /// resizes buffers for a newly negotiated format; not called per buffer
    pub(super) fn format_changed(&mut self, param: &Pod) {
        if self.format.parse(param).is_err() {
            return;
        }
        let (rate, channels) = (self.format.rate(), self.format.channels());
        debug!("capture format: {rate} Hz, {channels} channels, f32");
        self.shared.status.set_format(rate, channels);
        self.channels = u64::from(channels.max(1));
        self.ring
            .reset(rate as usize * channels as usize * RING_SECONDS);
        self.cursor = 0;
        self.build_pipeline();
    }

    /// allocates the pipeline for the negotiated format and current tuning
    fn build_pipeline(&mut self) {
        let (rate, channels) = (self.format.rate(), self.format.channels());
        let tuning = self.tuning.borrow();
        self.pipeline = (rate > 0 && channels > 0)
            .then(|| Pipeline::new(rate, channels, &tuning.settings.spectrum));
    }

    /// picks up new settings; allocates only when they changed
    fn retune(&mut self) {
        let (generation, interval) = {
            let tuning = self.tuning.borrow();
            (tuning.generation, tuning.settings.interval)
        };
        if generation == self.generation {
            return;
        }
        self.generation = generation;
        self.pacing = Pacing::new(interval);
        if self.pipeline.is_some() {
            self.build_pipeline();
        }
    }

    /// a stream that stops delivering audio counts as silence
    pub(super) fn state_changed(&mut self, state: CaptureState) {
        self.shared.status.set_state(state);
        if state != CaptureState::Streaming {
            self.shared.publish(&self.tuning.borrow().frames, None, 0.0);
        }
    }

    pub(super) fn push_le_bytes(&mut self, bytes: &[u8]) {
        self.ring.push_le_bytes(bytes);
    }

    /// analyzes new samples at most once per interval; must not allocate
    pub(super) fn tick(&mut self, now: Instant) {
        self.retune();
        if !self.pacing.due(now) {
            return;
        }
        let mut peak = 0.0_f32;
        let pipeline = &mut self.pipeline;
        self.cursor = self.ring.read_since(self.cursor, |sample| {
            peak = peak.max(sample.abs());
            if let Some(pipeline) = pipeline.as_mut() {
                pipeline.push_interleaved(sample);
            }
        });

        let status = &self.shared.status;
        status.set_peak(peak);
        status.set_frames(self.ring.written() / self.channels);
        if let Some(pipeline) = self.pipeline.as_mut() {
            let frames = &self.tuning.borrow().frames;
            self.shared.publish(frames, pipeline.analyze(peak), peak);
        }
    }
}

/// analysis deadlines one interval apart, so a quantum shorter than the
/// interval still gives the full rate on average
#[derive(Debug)]
struct Pacing {
    interval: Duration,
    next: Option<Instant>,
}

impl Pacing {
    fn new(interval: Duration) -> Self {
        Self {
            interval,
            next: None,
        }
    }

    /// whether an analysis is due at `now`, moving the deadline if so
    fn due(&mut self, now: Instant) -> bool {
        if self.next.is_some_and(|next| now < next) {
            return false;
        }
        let next = self.next.map_or(now, |next| next + self.interval);
        // after a gap (a large quantum or a pause) start again from now
        self.next = Some(if next > now {
            next
        } else {
            now + self.interval
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::Pacing;

    fn analyses(interval_ms: u64, callback_us: u64, seconds: u64) -> usize {
        let start = Instant::now();
        let mut pacing = Pacing::new(Duration::from_millis(interval_ms));
        (0..seconds * 1_000_000 / callback_us)
            .filter(|step| pacing.due(start + Duration::from_micros(step * callback_us)))
            .count()
    }

    #[test]
    fn short_quanta_are_analyzed_at_the_interval_rate() {
        // 256 frames at 48 khz arrive every 5.33 ms; a 16 ms interval gives
        // 62.5 analyses per second, not every fourth callback (47/s)
        let count = analyses(16, 5_333, 10);
        assert!((620..=630).contains(&count), "{count}");
    }

    #[test]
    fn long_quanta_are_analyzed_every_time() {
        // 2048 frames at 48 khz arrive every 42.7 ms, 234 times in 10 s
        let callbacks = (10_000_000 / 42_667) as usize;
        assert_eq!(analyses(16, 42_667, 10), callbacks);
    }
}
