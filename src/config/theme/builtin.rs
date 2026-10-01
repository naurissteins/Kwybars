//! themes compiled into the binary, used when no theme file is found

const BUILTIN: [(&str, &str); 8] = [
    (
        "ayu-dark",
        include_str!("../../../assets/themes/ayu-dark.toml"),
    ),
    (
        "catppuccin-mocha",
        include_str!("../../../assets/themes/catppuccin-mocha.toml"),
    ),
    (
        "dracula",
        include_str!("../../../assets/themes/dracula.toml"),
    ),
    (
        "everforest",
        include_str!("../../../assets/themes/everforest.toml"),
    ),
    (
        "gruvbox",
        include_str!("../../../assets/themes/gruvbox.toml"),
    ),
    ("nord", include_str!("../../../assets/themes/nord.toml")),
    (
        "rose-pine",
        include_str!("../../../assets/themes/rose-pine.toml"),
    ),
    (
        "tokyo-night",
        include_str!("../../../assets/themes/tokyo-night.toml"),
    ),
];

/// source text of a built-in theme
pub fn find(name: &str) -> Option<&'static str> {
    BUILTIN
        .iter()
        .find(|(builtin, _)| *builtin == name)
        .map(|(_, raw)| *raw)
}

pub fn names() -> impl Iterator<Item = &'static str> {
    BUILTIN.iter().map(|(name, _)| *name)
}

#[cfg(test)]
mod tests {
    use super::BUILTIN;
    use crate::config::theme::Theme;

    #[test]
    fn every_builtin_theme_parses_with_its_own_name() {
        for (name, raw) in BUILTIN {
            match Theme::parse(raw, "fallback") {
                Ok(theme) => assert_eq!(theme.name, name),
                Err(err) => panic!("built-in theme {name}: {err}"),
            }
        }
    }
}
