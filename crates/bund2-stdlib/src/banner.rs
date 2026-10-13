//! `io.banner`'s font — D152.
//!
//! The reference draws a banner with `cfonts` in its "tiny" font
//! (`reference/Bund/src/stdlib/functions/io/banner.rs:36-43`). `cfonts` is
//! GPL-3.0-or-later and cannot be linked into a binary offered as
//! Apache-2.0, so the banner is drawn here.
//!
//! **Every glyph and every rule below was read from what the oracle
//! prints**, not from `cfonts`: `io.banner` was run on each printable
//! character alone, and on text chosen to show the spacing, the line break,
//! the wrap and what a character with no glyph does. What was observed:
//!
//! - Two rows a line. A row is a space, then the glyphs with one space
//!   between them. A line with no glyph at all is two empty rows.
//! - Lower case draws as upper case.
//! - A character with no glyph is left out, with no gap where it was. That
//!   is every character not in the table: ten of ASCII's printable ones,
//!   the backslash, the controls, and everything beyond ASCII.
//! - `|` ends a line.
//! - A glyph that would take a row past the display's width starts a new
//!   line instead, wherever in a word it falls. The width is the terminal's
//!   when any of standard output, error or input is one, and 80 otherwise.
//! - The whole is two newlines, the lines with an empty row between them,
//!   and two newlines.

/// The two rows of `c`'s glyph, or `None` where the font has none.
fn glyph(c: char) -> Option<(&'static str, &'static str)> {
    Some(match c.to_ascii_uppercase() {
        ' ' => (" ", " "),
        '!' => ("█", "▄"),
        '"' => ("▛ ▜", "   "),
        '#' => ("▟▄▙", "▜▀▛"),
        '$' => ("▖█▗", "▘█▝"),
        '%' => ("▀ ▄▀", "▄▀ ▄"),
        '&' => ("▄▄█", "█▄█"),
        '\'' => ("▀", " "),
        '(' => ("▄▀", "▀▄"),
        ')' => ("▀▄", "▄▀"),
        '+' => ("▄█▄", " ▀ "),
        ',' => (" ", "█"),
        '-' => ("▄▄", "  "),
        '.' => (" ", "▄"),
        '/' => ("  ▄▀", "▄▀  "),
        '0' => ("▞█▚", "▚█▞"),
        '1' => ("▄█", " █"),
        '2' => ("▀█", "█▄"),
        '3' => ("▀▀█", "▄██"),
        '4' => ("█ █", "▀▀█"),
        '5' => ("█▀", "▄█"),
        '6' => ("█▄▄", "█▄█"),
        '7' => ("▀▀█", "  █"),
        '8' => ("███", "█▄█"),
        '9' => ("█▀█", "▀▀█"),
        ':' => ("▀", "▄"),
        ';' => ("  ", "▄▀"),
        '=' => ("▀▀", "▀▀"),
        '?' => ("▀█", " ▄"),
        '@' => ("▛█▜", "▙▟▃"),
        'A' => ("▄▀█", "█▀█"),
        'B' => ("█▄▄", "█▄█"),
        'C' => ("█▀▀", "█▄▄"),
        'D' => ("█▀▄", "█▄▀"),
        'E' => ("█▀▀", "██▄"),
        'F' => ("█▀▀", "█▀ "),
        'G' => ("█▀▀", "█▄█"),
        'H' => ("█ █", "█▀█"),
        'I' => ("█", "█"),
        'J' => ("  █", "█▄█"),
        'K' => ("█▄▀", "█ █"),
        'L' => ("█  ", "█▄▄"),
        'M' => ("█▀▄▀█", "█ ▀ █"),
        'N' => ("█▄ █", "█ ▀█"),
        'O' => ("█▀█", "█▄█"),
        'P' => ("█▀█", "█▀▀"),
        'Q' => ("█▀█", "▀▀█"),
        'R' => ("█▀█", "█▀▄"),
        'S' => ("█▀▀", "▄▄█"),
        'T' => ("▀█▀", " █ "),
        'U' => ("█ █", "█▄█"),
        'V' => ("█ █", "▀▄▀"),
        'W' => ("█ █ █", "▀▄▀▄▀"),
        'X' => ("▀▄▀", "█ █"),
        'Y' => ("█▄█", " █ "),
        'Z' => ("▀█", "█▄"),
        '_' => ("  ", "▄▄"),
        _ => return None,
    })
}

/// How wide a row may be where nothing says: no terminal to ask.
const DEFAULT_WIDTH: usize = 80;

/// The display's width, as the reference's renderer finds it.
pub(crate) fn display_width() -> usize {
    terminal_size::terminal_size().map_or(DEFAULT_WIDTH, |(w, _)| usize::from(w.0))
}

/// `text` as a banner, in rows at most `width` columns wide.
pub(crate) fn render(text: &str, width: usize) -> String {
    // Each line is the glyphs on it. A line is ended by `|`, or by a glyph
    // that does not fit.
    let mut lines: Vec<Vec<(&str, &str)>> = vec![Vec::new()];
    let mut used = 0usize;
    for c in text.chars() {
        if c == '|' {
            lines.push(Vec::new());
            used = 0;
            continue;
        }
        let Some(g) = glyph(c) else {
            continue;
        };
        // A glyph and the space before it: the row's leading space for the
        // first, the gap for the rest.
        let takes = g.0.chars().count() + 1;
        // The first glyph of a line is always placed, so a width narrower
        // than a glyph still ends: not observed, since no terminal that
        // narrow was tried.
        if used > 0 && used + takes > width {
            lines.push(Vec::new());
            used = 0;
        }
        used += takes;
        if let Some(line) = lines.last_mut() {
            line.push(g);
        }
    }

    let mut out = String::from("\n\n");
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        for g in line {
            out.push(' ');
            out.push_str(g.0);
        }
        out.push('\n');
        for g in line {
            out.push(' ');
            out.push_str(g.1);
        }
    }
    out.push_str("\n\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the oracle printed for each, copied from its output.
    #[test]
    fn the_banner_is_what_the_oracle_prints() {
        assert_eq!(render("A", 80), "\n\n ▄▀█\n █▀█\n\n");
        assert_eq!(render("AB a", 80), "\n\n ▄▀█ █▄▄   ▄▀█\n █▀█ █▄█   █▀█\n\n");
        assert_eq!(render("", 80), "\n\n\n\n\n");
        assert_eq!(render(" ", 80), "\n\n  \n  \n\n");
        assert_eq!(render("I.I", 80), "\n\n █   █\n █ ▄ █\n\n");
        assert_eq!(render("A\"B", 80), "\n\n ▄▀█ ▛ ▜ █▄▄\n █▀█     █▄█\n\n");
    }

    #[test]
    fn a_character_with_no_glyph_leaves_no_gap() {
        let ab = render("AB", 80);
        for text in ["A*B", "AéB", "A\tB", "A\nB", "A\\B", "A~B"] {
            assert_eq!(render(text, 80), ab, "{text:?}");
        }
        assert_eq!(render("*", 80), render("", 80));
    }

    #[test]
    fn a_bar_ends_a_line() {
        assert_eq!(render("A|B", 80), "\n\n ▄▀█\n █▀█\n\n █▄▄\n █▄█\n\n");
        assert_eq!(render("|", 80), "\n\n\n\n\n\n\n\n");
        assert_eq!(render("A||B", 80).split('\n').count(), 12);
    }

    /// Forty `I` fill 80 columns exactly and the forty-first starts a line;
    /// thirteen `M` come to 78 and a fourteenth would be 84.
    #[test]
    fn a_row_is_filled_to_the_width_and_no_further() {
        let widths = |s: &str| -> Vec<usize> { s.split('\n').map(|r| r.chars().count()).collect() };
        assert_eq!(
            widths(&render(&"I".repeat(45), 80)),
            [0, 0, 80, 80, 0, 10, 10, 0, 0]
        );
        assert_eq!(
            widths(&render(&"M".repeat(20), 80)),
            [0, 0, 78, 78, 0, 42, 42, 0, 0]
        );
        assert_eq!(
            widths(&render(&"I".repeat(120), 40)),
            [
                0, 0, 40, 40, 0, 40, 40, 0, 40, 40, 0, 40, 40, 0, 40, 40, 0, 40, 40, 0, 0
            ]
        );
        assert_eq!(
            widths(&render(&"I".repeat(120), 200)),
            [0, 0, 200, 200, 0, 40, 40, 0, 0]
        );
        // Narrower than a glyph: one a line, and it ends.
        assert_eq!(render("MM", 2).split('\n').count(), 9);
    }

    #[test]
    fn both_rows_of_every_glyph_are_one_width() {
        for c in (0u8..=0x7f).map(char::from) {
            if let Some((top, bottom)) = glyph(c) {
                assert_eq!(top.chars().count(), bottom.chars().count(), "{c:?}");
                assert_eq!(glyph(c.to_ascii_lowercase()), glyph(c), "{c:?}");
            }
        }
        let drawn = (0u8..=0x7f)
            .filter(|b| glyph(char::from(*b)).is_some())
            .count();
        assert_eq!(drawn, 57 + 26);
    }
}
