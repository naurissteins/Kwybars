//! latest spectrum frame, shared from the capture thread without locks

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering, fence};

/// one value per bar plus a counter that changes with every publish
#[derive(Debug)]
pub struct FrameSlot {
    sequence: AtomicU64,
    bars: Box<[AtomicU32]>,
    silent: AtomicBool,
}

/// what [`FrameSlot::read_into`] found
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    /// frames published so far
    pub number: u64,
    pub silent: bool,
}

impl FrameSlot {
    pub fn new(bars: usize) -> Self {
        Self {
            sequence: AtomicU64::new(0),
            bars: (0..bars).map(|_| AtomicU32::new(0)).collect(),
            silent: AtomicBool::new(true),
        }
    }

    pub fn len(&self) -> usize {
        self.bars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    /// stores a new frame of sound; must only be called from one thread
    pub fn publish(&self, values: &[f32]) {
        self.write(|slot| {
            for (bar, value) in slot.bars.iter().zip(values) {
                bar.store(value.to_bits(), Ordering::Relaxed);
            }
            slot.silent.store(false, Ordering::Relaxed);
        });
    }

    /// marks the capture as silent; must only be called from one thread
    pub fn publish_silence(&self) {
        self.write(|slot| slot.silent.store(true, Ordering::Relaxed));
    }

    fn write(&self, store: impl FnOnce(&Self)) {
        let start = self.sequence.load(Ordering::Relaxed);
        self.sequence
            .store(start.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release);
        store(self);
        self.sequence
            .store(start.wrapping_add(2), Ordering::Release);
    }

    /// frames published so far, for cheap change detection
    pub fn frame_number(&self) -> u64 {
        self.sequence.load(Ordering::Acquire) / 2
    }

    /// copies the newest complete frame into `out`
    pub fn read_into(&self, out: &mut [f32]) -> FrameInfo {
        loop {
            let before = self.sequence.load(Ordering::Acquire);
            if before % 2 == 1 {
                std::hint::spin_loop();
                continue;
            }
            for (value, bar) in out.iter_mut().zip(self.bars.iter()) {
                *value = f32::from_bits(bar.load(Ordering::Relaxed));
            }
            let silent = self.silent.load(Ordering::Relaxed);
            fence(Ordering::Acquire);
            if self.sequence.load(Ordering::Relaxed) == before {
                return FrameInfo {
                    number: before / 2,
                    silent,
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::{FrameInfo, FrameSlot};

    #[test]
    fn reads_what_was_published() {
        let slot = FrameSlot::new(3);
        let mut out = [9.0; 3];
        let info = |number, silent| FrameInfo { number, silent };
        assert_eq!(slot.read_into(&mut out), info(0, true));
        assert_eq!(out, [0.0; 3]);
        slot.publish(&[0.1, 0.5, 1.0]);
        assert_eq!(
            (slot.read_into(&mut out), out),
            (info(1, false), [0.1, 0.5, 1.0])
        );
        slot.publish_silence();
        assert_eq!(
            (slot.read_into(&mut out), out),
            (info(2, true), [0.1, 0.5, 1.0])
        );
        assert_eq!(slot.frame_number(), 2);
    }

    #[test]
    fn concurrent_readers_never_see_a_torn_frame() {
        let slot = Arc::new(FrameSlot::new(64));
        let done = Arc::new(AtomicBool::new(false));
        let writer = {
            let (slot, done) = (Arc::clone(&slot), Arc::clone(&done));
            std::thread::spawn(move || {
                let mut frame = [0.0_f32; 64];
                for round in 0..200_000_u32 {
                    frame.fill(round as f32);
                    slot.publish(&frame);
                }
                done.store(true, Ordering::Release);
            })
        };
        let mut out = [0.0_f32; 64];
        let mut reads = 0_u64;
        while !done.load(Ordering::Acquire) || reads == 0 {
            slot.read_into(&mut out);
            assert!(
                out.iter().all(|value| *value == out[0]),
                "torn frame {out:?}"
            );
            reads += 1;
        }
        assert!(writer.join().is_ok());
        assert!(reads > 0);
    }
}
