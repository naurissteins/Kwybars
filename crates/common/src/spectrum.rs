#[derive(Debug, Clone, PartialEq)]
pub struct SpectrumFrame {
    pub bars: Vec<f32>,
    pub peak: f32,
    pub timestamp_millis: u64,
}

impl SpectrumFrame {
    pub fn new(mut bars: Vec<f32>, timestamp_millis: u64) -> Self {
        let mut peak = 0.0_f32;
        for value in &mut bars {
            *value = value.clamp(0.0, 1.0);
            peak = peak.max(*value);
        }

        Self {
            bars,
            peak,
            timestamp_millis,
        }
    }

    pub fn bar_count(&self) -> usize {
        self.bars.len()
    }
}

#[cfg(test)]
mod tests {
    use super::SpectrumFrame;

    #[test]
    fn clamps_values_to_unit_range() {
        let frame = SpectrumFrame::new(vec![-1.0, 0.4, 2.0], 0);
        assert_eq!(frame.bars, vec![0.0, 0.4, 1.0]);
        assert_eq!(frame.peak, 1.0);
    }
}
