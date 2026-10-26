//! the `theme` key: a theme name or a palette written inline

use std::fmt;

use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, Visitor, value::MapAccessDeserializer};

use super::{Theme, parse_hex};
use crate::config::color::Rgba;

/// name given to an inline palette that sets none
const INLINE_NAME: &str = "inline";

/// what `theme` is set to
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeChoice {
    /// looked up in the theme directories, then the built-in themes
    Named(String),
    Inline(Theme),
}

/// a palette table, with the same keys as a theme file
#[derive(Deserialize)]
struct InlineTheme {
    name: Option<String>,
    red: HexColor,
    green: HexColor,
    yellow: HexColor,
    blue: HexColor,
    magenta: HexColor,
    cyan: HexColor,
}

struct HexColor(Rgba);

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        parse_hex(&raw)
            .map(Self)
            .ok_or_else(|| de::Error::custom(format!("expected #rrggbb or #rrggbbaa, got {raw:?}")))
    }
}

impl From<InlineTheme> for Theme {
    fn from(inline: InlineTheme) -> Self {
        let InlineTheme {
            name,
            red,
            green,
            yellow,
            blue,
            magenta,
            cyan,
        } = inline;
        Self {
            name: name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| INLINE_NAME.to_owned()),
            colors: [red.0, green.0, yellow.0, blue.0, magenta.0, cyan.0],
        }
    }
}

impl<'de> Deserialize<'de> for ThemeChoice {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ChoiceVisitor)
    }
}

// a visitor rather than an untagged enum, so palette errors keep their key
struct ChoiceVisitor;

impl<'de> Visitor<'de> for ChoiceVisitor {
    type Value = ThemeChoice;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a theme name or a table of theme colors")
    }

    fn visit_str<E: de::Error>(self, name: &str) -> Result<Self::Value, E> {
        Ok(ThemeChoice::Named(name.to_owned()))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let inline = InlineTheme::deserialize(MapAccessDeserializer::new(map))?;
        Ok(ThemeChoice::Inline(inline.into()))
    }
}
