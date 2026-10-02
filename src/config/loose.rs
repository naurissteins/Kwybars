//! lenient `key = value` reader for generated files (colors.toml, themes)

/// one `key = value` line
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry<'a> {
    /// last `[section]` header seen, if any
    pub section: Option<&'a str>,
    pub key: &'a str,
    pub value: String,
    /// 1-based line number
    pub line: usize,
}

/// yields every `key = value` line; comments, headers, and other lines are skipped
pub fn entries(raw: &str) -> impl Iterator<Item = Entry<'_>> {
    let mut section = None;
    raw.lines().enumerate().filter_map(move |(index, line)| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        if let Some(name) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = Some(name.trim());
            return None;
        }
        let (key, value) = trimmed.split_once('=')?;
        Some(Entry {
            section,
            key: key.trim(),
            value: clean_value(value),
            line: index + 1,
        })
    })
}

/// drops an inline comment, trailing `,`/`;`, and surrounding quotes
fn clean_value(raw: &str) -> String {
    let mut in_quotes = false;
    let mut escaped = false;
    let mut end = raw.len();
    for (index, ch) in raw.char_indices() {
        match ch {
            '"' if !escaped => in_quotes = !in_quotes,
            '#' if !in_quotes => {
                end = index;
                break;
            }
            _ => {}
        }
        escaped = ch == '\\' && !escaped;
    }

    let value = raw[..end].trim().trim_end_matches([',', ';']).trim();
    let unquoted = ['"', '\'']
        .iter()
        .find_map(|quote| value.strip_prefix(*quote)?.strip_suffix(*quote))
        .unwrap_or(value);
    unquoted.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::entries;

    #[test]
    fn reads_sections_keys_and_cleaned_values() {
        let raw = "# header\nname = \"nord\"\n[visualizer]\ncolor_rgba = \"rgba(1, 2, 3, 0.5)\"; # matugen\nbad line\nx = 'single',\n";
        let found: Vec<_> = entries(raw)
            .map(|entry| (entry.section, entry.key, entry.value, entry.line))
            .collect();
        assert_eq!(
            found,
            vec![
                (None, "name", "nord".to_owned(), 2),
                (
                    Some("visualizer"),
                    "color_rgba",
                    "rgba(1, 2, 3, 0.5)".to_owned(),
                    4
                ),
                (Some("visualizer"), "x", "single".to_owned(), 6),
            ]
        );
    }

    #[test]
    fn keeps_hash_inside_quotes() {
        let value = entries("red = \"#bf616a\" # comment\n")
            .map(|entry| entry.value)
            .next();
        assert_eq!(value.as_deref(), Some("#bf616a"));
    }
}
