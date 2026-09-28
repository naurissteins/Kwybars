//! terminal bars for `kwybars debug spectrum`

use std::fmt::Write as _;
use std::io::{self, Write as _};

use super::meter;
use crate::audio::capture::StatusSnapshot;

/// bar height in terminal rows
const ROWS: usize = 16;
/// partial cells, one eighth of a row each
const EIGHTHS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// redraws the bars in place
#[derive(Debug, Default)]
pub struct View {
    drawn: bool,
    text: String,
}

impl View {
    pub fn show(&mut self, bars: &[f32], status: &StatusSnapshot, gain: f32) {
        self.text.clear();
        if self.drawn {
            // back to the header line
            let _ = write!(self.text, "\x1b[{}A", ROWS + 1);
        } else {
            // hide the cursor while drawing
            self.text.push_str("\x1b[?25l");
        }
        let _ = writeln!(
            self.text,
            "\r{}  gain {:8.2}\x1b[K",
            meter::summary(status),
            gain
        );
        render_rows(bars, ROWS, &mut self.text);

        let mut stdout = io::stdout().lock();
        // a closed stdout is not worth failing over
        let _ = stdout.write_all(self.text.as_bytes());
        let _ = stdout.flush();
        self.drawn = true;
    }

    /// shows the cursor again below the bars
    pub fn finish(&self) {
        println!("\x1b[?25h");
    }
}

/// draws `bars` as columns `rows` high, top row first
fn render_rows(bars: &[f32], rows: usize, out: &mut String) {
    let (width, gap) = match bars.len() {
        0..=40 => (2, 1),
        41..=80 => (1, 1),
        _ => (1, 0),
    };
    for row in 0..rows {
        let floor = (rows - 1 - row) as f32;
        out.push('\r');
        for bar in bars {
            let fill = (bar.clamp(0.0, 1.0) * rows as f32 - floor).clamp(0.0, 1.0);
            let eighths = (fill * 8.0).floor() as usize;
            let cell = eighths
                .checked_sub(1)
                .and_then(|i| EIGHTHS.get(i))
                .copied()
                .unwrap_or(' ');
            out.extend(std::iter::repeat_n(cell, width));
            out.extend(std::iter::repeat_n(' ', gap));
        }
        out.push_str("\x1b[K\n");
    }
}

#[cfg(test)]
mod tests {
    use super::render_rows;

    fn columns(bars: &[f32], rows: usize) -> Vec<String> {
        let mut out = String::new();
        render_rows(bars, rows, &mut out);
        out.lines()
            .map(|line| {
                line.trim_start_matches('\r')
                    .trim_end_matches("\x1b[K")
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn draws_full_half_and_empty_bars() {
        let lines = columns(&[0.0, 0.5, 1.0], 4);
        assert_eq!(
            lines,
            vec!["      ██ ", "      ██ ", "   ██ ██ ", "   ██ ██ "]
        );
    }

    #[test]
    fn partial_rows_use_eighth_blocks() {
        let lines = columns(&[0.3125], 2);
        // 0.3125 * 2 rows = 0.625 of the bottom row, five eighths
        assert_eq!(lines, vec!["   ", "▅▅ "]);
    }

    #[test]
    fn many_bars_get_narrow_columns() {
        let lines = columns(&vec![1.0; 100], 1);
        assert_eq!(lines[0].chars().count(), 100);
    }
}
