//! surface scale and the buffer size it needs

/// denominator of wp_fractional_scale_v1 scales
const FRACTIONAL_DENOMINATOR: u64 = 120;

/// how a surface maps logical pixels to buffer pixels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    Fractional(u32),
    Integer(u32),
}

impl Scale {
    /// buffer pixels per logical pixel
    pub fn factor(self) -> f32 {
        match self {
            Self::Fractional(scale) => scale as f32 / FRACTIONAL_DENOMINATOR as f32,
            Self::Integer(scale) => scale.max(1) as f32,
        }
    }

    pub fn step(self) -> (u32, u32) {
        match self {
            Self::Integer(scale) => (scale.max(1), 1),
            Self::Fractional(scale) => {
                let (mut a, mut b) = (u64::from(scale.max(1)), FRACTIONAL_DENOMINATOR);
                while b != 0 {
                    (a, b) = (b, a % b);
                }
                let per = u64::from(scale.max(1)) / a;
                (per as u32, (FRACTIONAL_DENOMINATOR / a) as u32)
            }
        }
    }

    /// buffer size in physical pixels for a logical surface size
    pub fn buffer_size(self, logical: (u32, u32)) -> (u32, u32) {
        let scale = |extent: u32| -> u32 {
            let scaled = match self {
                Self::Fractional(scale) => {
                    (u64::from(extent) * u64::from(scale) + FRACTIONAL_DENOMINATOR / 2)
                        / FRACTIONAL_DENOMINATOR
                }
                Self::Integer(scale) => u64::from(extent) * u64::from(scale.max(1)),
            };
            u32::try_from(scaled).unwrap_or(u32::MAX).max(1)
        };
        (scale(logical.0), scale(logical.1))
    }
}

impl std::fmt::Display for Scale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fractional(scale) => {
                write!(f, "{:.3} (fractional)", f64::from(*scale) / 120.0)
            }
            Self::Integer(scale) => write!(f, "{scale} (integer)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Scale;

    #[test]
    fn fractional_scale_rounds_to_physical_pixels() {
        // the default bar on a 4k output at 1.5
        assert_eq!(Scale::Fractional(180).buffer_size((2520, 500)), (3780, 750));
        // 1.25 of 101 is 126.25, 1.75 of 101 is 176.75
        assert_eq!(Scale::Fractional(150).buffer_size((101, 101)), (126, 126));
        assert_eq!(Scale::Fractional(210).buffer_size((101, 1)), (177, 2));
        assert_eq!(Scale::Fractional(120).buffer_size((7, 9)), (7, 9));
    }

    #[test]
    fn steps_are_the_scale_in_lowest_terms() {
        assert_eq!(Scale::Fractional(180).step(), (3, 2));
        assert_eq!(Scale::Fractional(150).step(), (5, 4));
        assert_eq!(Scale::Fractional(120).step(), (1, 1));
        assert_eq!(Scale::Fractional(160).step(), (4, 3));
        assert_eq!(Scale::Integer(2).step(), (2, 1));
    }

    #[test]
    fn integer_scale_multiplies_and_never_returns_zero() {
        assert_eq!(Scale::Integer(2).buffer_size((2520, 500)), (5040, 1000));
        assert_eq!(Scale::Integer(0).buffer_size((3, 4)), (3, 4));
        assert_eq!(Scale::Fractional(1).buffer_size((0, 1)), (1, 1));
    }
}
