//! fixed-size ring of interleaved samples, filled by the process callback

/// newest samples captured, oldest overwritten first
#[derive(Debug, Default)]
pub struct SampleRing {
    samples: Vec<f32>,
    /// total samples ever pushed, used as a cursor by readers
    written: u64,
}

impl SampleRing {
    /// replaces the storage; the only call that allocates
    pub fn reset(&mut self, capacity: usize) {
        self.samples = vec![0.0; capacity];
        self.written = 0;
    }

    #[cfg(test)]
    pub fn capacity(&self) -> usize {
        self.samples.len()
    }

    /// cursor value after the newest sample
    pub fn written(&self) -> u64 {
        self.written
    }

    /// appends little-endian f32 samples, a trailing partial sample is dropped
    pub fn push_le_bytes(&mut self, bytes: &[u8]) {
        let capacity = self.samples.len() as u64;
        if capacity == 0 {
            return;
        }
        let (samples, _partial) = bytes.as_chunks::<4>();
        for raw in samples {
            let index = (self.written % capacity) as usize;
            if let Some(slot) = self.samples.get_mut(index) {
                *slot = f32::from_le_bytes(*raw);
            }
            self.written += 1;
        }
    }

    /// visits samples written after `since`, oldest first, and returns the new
    /// cursor; if the reader fell behind, the overwritten part is skipped
    pub fn read_since(&self, since: u64, mut visit: impl FnMut(f32)) -> u64 {
        let capacity = self.samples.len() as u64;
        if capacity == 0 {
            return self.written;
        }
        let start = since.max(self.written.saturating_sub(capacity));
        for position in start..self.written {
            if let Some(sample) = self.samples.get((position % capacity) as usize) {
                visit(*sample);
            }
        }
        self.written
    }
}

#[cfg(test)]
mod tests {
    use super::SampleRing;

    fn bytes(samples: &[f32]) -> Vec<u8> {
        samples.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    fn collect(ring: &SampleRing, since: u64) -> (Vec<f32>, u64) {
        let mut out = Vec::new();
        let cursor = ring.read_since(since, |s| out.push(s));
        (out, cursor)
    }

    #[test]
    fn reads_only_new_samples() {
        let mut ring = SampleRing::default();
        ring.reset(8);
        ring.push_le_bytes(&bytes(&[1.0, 2.0, 3.0]));
        let (first, cursor) = collect(&ring, 0);
        assert_eq!((first, cursor), (vec![1.0, 2.0, 3.0], 3));
        ring.push_le_bytes(&bytes(&[4.0]));
        assert_eq!(collect(&ring, cursor), (vec![4.0], 4));
    }

    #[test]
    fn slow_readers_skip_overwritten_samples() {
        let mut ring = SampleRing::default();
        ring.reset(3);
        ring.push_le_bytes(&bytes(&[1.0, 2.0, 3.0, 4.0, 5.0]));
        assert_eq!(collect(&ring, 0), (vec![3.0, 4.0, 5.0], 5));
    }

    #[test]
    fn ignores_partial_samples_and_empty_storage() {
        let mut ring = SampleRing::default();
        ring.push_le_bytes(&bytes(&[1.0]));
        assert_eq!(ring.written(), 0);
        ring.reset(4);
        let mut raw = bytes(&[1.0]);
        raw.push(0xff);
        ring.push_le_bytes(&raw);
        assert_eq!(collect(&ring, 0), (vec![1.0], 1));
    }

    #[test]
    fn pushing_does_not_reallocate() {
        let mut ring = SampleRing::default();
        ring.reset(16);
        let before = ring.capacity();
        ring.push_le_bytes(&bytes(&[0.5; 100]));
        assert_eq!(ring.capacity(), before);
    }
}
