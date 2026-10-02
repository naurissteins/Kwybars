//! `rgba(r, g, b, a)` colors

use serde::Deserialize;
use serde::de::{self, Deserializer};

/// color with components in `0.0..=1.0`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// a color string that is not `rgba(r, g, b, a)`
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid color {0:?}: expected \"rgba(r, g, b, a)\"")]
pub struct ColorParseError(String);

impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// parses `rgba(r, g, b, a)` or bare `r, g, b, a`
    ///
    /// rgb above 1 on any channel means the 0-255 scale; alpha is always 0-1
    pub fn parse(value: &str) -> Result<Self, ColorParseError> {
        let error = || ColorParseError(value.to_owned());
        let trimmed = value.trim();
        let inner = match trimmed.strip_prefix("rgba(") {
            Some(rest) => rest.strip_suffix(')').unwrap_or(rest),
            None => trimmed,
        };

        let mut parts = [0.0_f32; 4];
        let mut count = 0;
        for part in inner.split(',') {
            let slot = parts.get_mut(count).ok_or_else(error)?;
            *slot = part.trim().parse::<f32>().map_err(|_| error())?;
            if !slot.is_finite() {
                return Err(error());
            }
            count += 1;
        }
        if count != 4 {
            return Err(error());
        }

        let [mut r, mut g, mut b, a] = parts;
        if r > 1.0 || g > 1.0 || b > 1.0 {
            r /= 255.0;
            g /= 255.0;
            b /= 255.0;
        }
        Ok(Self::new(
            r.clamp(0.0, 1.0),
            g.clamp(0.0, 1.0),
            b.clamp(0.0, 1.0),
            a.clamp(0.0, 1.0),
        ))
    }
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::Rgba;

    #[test]
    fn parses_byte_scale() {
        assert_eq!(
            Rgba::parse("rgba(255, 0, 51, 0.5)"),
            Ok(Rgba::new(1.0, 0.0, 51.0 / 255.0, 0.5))
        );
    }

    #[test]
    fn parses_unit_scale_without_prefix() {
        assert_eq!(
            Rgba::parse(" 0.5, 1, 0, 1 "),
            Ok(Rgba::new(0.5, 1.0, 0.0, 1.0))
        );
    }

    #[test]
    fn clamps_out_of_range_components() {
        assert_eq!(
            Rgba::parse("rgba(300, -5, 0, 2)"),
            Ok(Rgba::new(1.0, 0.0, 0.0, 1.0))
        );
    }

    #[test]
    fn rejects_malformed_values() {
        for bad in [
            "",
            "rgba(1, 2, 3)",
            "rgba(1, 2, 3, 4, 5)",
            "rgba(a, b, c, d)",
            "rgba(nan, 0, 0, 1)",
        ] {
            assert!(Rgba::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }
}
