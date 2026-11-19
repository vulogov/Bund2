//! RFC-0017 §S6's `frame.text`: a plain-text table for reading (D257 S9).
//!
//! A header row, then at most 20 rows, one space between columns. A numeric
//! column is right-aligned, its header and NONEs included; every other
//! column, one of no kind included, is left-aligned. A column is as wide as
//! its widest printed cell or header, in characters. NONE prints `NA`
//! (P12), a FLOAT as Bund prints it, a STRING as it is. No line ends in
//! padding or a separator: after the last non-empty cell nothing is
//! written, but a STRING's own trailing spaces stay. Past 20 rows a line
//! `… <n> more rows` (`… 1 more row`); a frame of no columns prints the
//! empty STRING. Lines are joined by `\n`, with none after the last.

use bund2_value::{BundValue, NONE};

use super::check::{Frame, Kind};

/// The most rows `frame.text` prints.
const ROWS: usize = 20;

/// One cell as `frame.text` prints it.
fn printed(v: &BundValue) -> String {
    if v.dt() == NONE {
        "NA".into()
    } else if let (Some(s), bund2_value::STRING) = (v.as_str(), v.dt()) {
        s
    } else {
        v.display()
    }
}

/// `text` padded to `width` characters, on the left when `right`.
fn pad(text: &str, width: usize, right: bool) -> String {
    let fill = " ".repeat(width.saturating_sub(text.chars().count()));
    if right {
        format!("{fill}{text}")
    } else {
        format!("{text}{fill}")
    }
}

/// One line of cells, with no padding or separator after its last
/// non-empty cell.
fn line(cells: &[String], widths: &[usize], right: &[bool]) -> String {
    let Some(last) = cells.iter().rposition(|c| !c.is_empty()) else {
        return String::new();
    };
    let mut parts = Vec::with_capacity(last + 1);
    for (i, c) in cells.iter().enumerate().take(last + 1) {
        let (w, r) = (
            widths.get(i).copied().unwrap_or(0),
            right.get(i) == Some(&true),
        );
        if i == last && !r {
            parts.push(c.clone());
        } else {
            parts.push(pad(c, w, r));
        }
    }
    parts.join(" ")
}

/// The table of a checked frame.
pub(super) fn table(f: &Frame) -> String {
    if f.names.is_empty() {
        return String::new();
    }
    let shown = f.nrow.min(ROWS);
    let right: Vec<bool> = (0..f.names.len())
        .map(|i| f.kind(i) == Kind::Numeric)
        .collect();
    let rows: Vec<Vec<String>> = (0..shown)
        .map(|r| {
            f.cols
                .iter()
                .map(|c| c.get(r).map(printed).unwrap_or_default())
                .collect()
        })
        .collect();
    let widths: Vec<usize> = f
        .names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            rows.iter()
                .filter_map(|row| row.get(i))
                .map(|c| c.chars().count())
                .chain(std::iter::once(n.chars().count()))
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut lines = vec![line(&f.names, &widths, &right)];
    lines.extend(rows.iter().map(|row| line(row, &widths, &right)));
    let more = f.nrow.saturating_sub(shown);
    if more > 0 {
        let noun = if more == 1 { "row" } else { "rows" };
        lines.push(format!("… {more} more {noun}"));
    }
    lines.join("\n")
}
