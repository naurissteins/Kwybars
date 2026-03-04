use kwybars_common::spectrum::SpectrumFrame;

pub trait FrameSource {
    fn next_frame(&mut self) -> SpectrumFrame;
}

#[derive(Debug)]
pub struct DummySineSource {
    bar_count: usize,
    phase: f32,
    frame_index: u64,
}

impl DummySineSource {
    pub fn new(bar_count: usize) -> Self {
        Self {
            bar_count,
            phase: 0.0,
            frame_index: 0,
        }
    }
}

impl FrameSource for DummySineSource {
    fn next_frame(&mut self) -> SpectrumFrame {
        let mut bars = Vec::with_capacity(self.bar_count);
        let spread = 0.35_f32;

        for index in 0..self.bar_count {
            let position = index as f32 * spread + self.phase;
            let value = (position.sin() * 0.5) + 0.5;
            bars.push(value);
        }

        self.phase += 0.2;
        self.frame_index += 1;

        SpectrumFrame::new(bars, self.frame_index * 16)
    }
}

#[cfg(test)]
mod tests {
    use super::{DummySineSource, FrameSource};

    #[test]
    fn produces_expected_bar_count() {
        let mut source = DummySineSource::new(12);
        let frame = source.next_frame();
        assert_eq!(frame.bar_count(), 12);
    }
}
