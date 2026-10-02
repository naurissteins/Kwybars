//! value checks that fix out-of-range settings and record a warning

use std::fmt::Display;

/// checks the keys of one table, prefixing warnings with its path
pub struct Bounds<'a> {
    table: &'a str,
    warnings: &'a mut Vec<String>,
}

impl<'a> Bounds<'a> {
    pub fn new(table: &'a str, warnings: &'a mut Vec<String>) -> Self {
        Self { table, warnings }
    }

    /// raises a value below `min` to `min`
    pub fn at_least<T: PartialOrd + Copy + Display>(
        &mut self,
        key: &str,
        value: &mut Option<T>,
        min: T,
    ) {
        if let Some(current) = value.as_mut()
            && *current < min
        {
            self.warn(key, format!("{current} is below {min}, using {min}"));
            *current = min;
        }
    }

    /// clamps a float into `min..=max`, dropping non-finite values
    pub fn within(&mut self, key: &str, value: &mut Option<f32>, min: f32, max: f32) {
        self.finite(key, value);
        if let Some(current) = value.as_mut()
            && !(min..=max).contains(current)
        {
            let clamped = current.clamp(min, max);
            self.warn(
                key,
                format!("{current} is outside {min}..={max}, using {clamped}"),
            );
            *current = clamped;
        }
    }

    /// drops a nan or infinite value
    pub fn finite(&mut self, key: &str, value: &mut Option<f32>) {
        if value.is_some_and(|current| !current.is_finite()) {
            self.warn(key, "is not a finite number, ignored".to_owned());
            *value = None;
        }
    }

    /// drops a value that is not allowed in this table
    pub fn unsupported<T>(&mut self, key: &str, value: &mut Option<T>, reason: &str) {
        if value.take().is_some() {
            self.warn(key, format!("{reason}, ignored"));
        }
    }

    fn warn(&mut self, key: &str, message: String) {
        self.warnings
            .push(format!("{}.{key}: {message}", self.table));
    }
}

#[cfg(test)]
mod tests {
    use super::Bounds;

    #[test]
    fn fixes_values_and_names_the_key() {
        let mut warnings = Vec::new();
        let mut bounds = Bounds::new("visualizer", &mut warnings);

        let mut sides = Some(2_u32);
        bounds.at_least("polygon_sides", &mut sides, 3);
        let mut opacity = Some(1.5_f32);
        bounds.within("theme_opacity", &mut opacity, 0.0, 1.0);
        let mut angle = Some(f32::NAN);
        bounds.finite("radial_start_angle", &mut angle);
        let mut untouched = Some(10_u32);
        bounds.at_least("bars", &mut untouched, 1);

        assert_eq!(
            (sides, opacity, angle, untouched),
            (Some(3), Some(1.0), None, Some(10))
        );
        assert_eq!(warnings.len(), 3);
        assert!(warnings[0].starts_with("visualizer.polygon_sides: "));
    }
}
