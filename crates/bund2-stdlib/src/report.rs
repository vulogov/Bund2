//! The terminal reporter — one implementation of `Reporter`, not the only one.
//!
//! # What is preserved and what is not
//!
//! The **frame** is the reference's: a `comfy_table` report with an `Error` row
//! and a `Location` row, then `[BUND]  Content of the stack` and the stack box,
//! written to stdout, after which the program exits **0**
//! (`reference/Bund/src/stdlib/helpers/print_error.rs:104-132`).
//!
//! The **content of the `Location` row** is not. The reference recovers a
//! **Rust** source path by regex from the tail of the message (`:12-46`), which
//! on the capture machine reads
//! `/Users/…/.cargo/registry/…/rust_multistackvm-0.38.0/src/stdlib/math/math_op.rs:111:29`
//! — F66. Bund2 names a position in the Bund program instead, which is the
//! thing the reader can act on and which the parser has always known. D36.
//!
//! # Delivery is proportional to severity
//!
//! An `Error` has stopped the program and gets the full report. A `Warning` or
//! `Notice` has not, and gets one line — no table, no stack, and on stderr so
//! it cannot corrupt a program's output. A report that shouts about everything
//! trains its reader to skip it.

use std::io::Write;

use bund2_api::diag::{Diagnostic, Reporter, Severity};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{ColumnConstraint, ContentArrangement, Table, Width};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Writes diagnostics to the terminal.
pub struct TextReporter {
    /// Whether an error report includes the stack and workbench.
    ///
    /// On by default, matching the reference, which always dumps. Off is for
    /// a caller that wants the reason alone — a build script, a test harness,
    /// or a TUI that renders the stack in its own pane and would only be
    /// duplicating it here.
    pub dump_stack: bool,
    /// Show the raw `Debug` rendering instead of a compact summary.
    ///
    /// **For a debug session only.** The raw form names every header field and
    /// runs about 150 columns for a single integer; a stack of ten overflows
    /// any terminal and buries the one fact the reader wanted.
    pub raw_values: bool,
    /// The widest a report may be. Bounds both the table and each value inside
    /// it, so nothing wraps into unreadability.
    pub width: usize,
    /// Where the fatal report goes. Stdout, as the reference's does, because
    /// the goldens capture it there.
    fatal_to_stdout: bool,
}

impl Default for TextReporter {
    fn default() -> Self {
        Self {
            dump_stack: true,
            raw_values: false,
            width: terminal_width(),
            fatal_to_stdout: true,
        }
    }
}

/// The terminal's width, or a readable default when there is no terminal.
///
/// A pipe has no width, and `comfy_table`'s `Dynamic` arrangement then lets a
/// row grow without bound — which is exactly how the reference's stack dumps
/// reach 190 columns in the goldens. For a *report* that is never what is
/// wanted, so an explicit bound is set either way.
fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.parse::<usize>().ok())
        .filter(|c| *c >= 40)
        .unwrap_or(100)
}

impl TextReporter {
    pub fn new(dump_stack: bool) -> Self {
        Self {
            dump_stack,
            ..Self::default()
        }
    }

    /// The two-row report. `Location` is present only when there is one to
    /// give — an error raised by a word called from a lambda body has no
    /// top-level position, and an empty row would claim more than is known.
    fn render_fatal(&self, d: &Diagnostic) -> String {
        let rows = Self::rows(d);
        let label = rows.iter().map(|(l, _)| l.width()).max().unwrap_or(0);
        let room = room_for_text(self.width, label);
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            // Without this the report grows to its widest cell, which on a
            // piped stream is unbounded.
            .set_width(self.width.min(u16::MAX as usize) as u16);
        let texts: Vec<&str> = rows.iter().map(|(_, text)| text.as_str()).collect();
        let (cells, fixed) = lay_out(&texts, room);
        for ((label, _), cell) in rows.iter().zip(cells) {
            table.add_row(vec![label.to_string(), cell]);
        }
        // The table fits a column to its text when the text fits. Text broken
        // here always fits, so the column is told the width it was broken for.
        if let (Some(fixed), Some(column)) = (fixed, table.column_mut(1)) {
            let wide = (fixed + 2).min(u16::MAX as usize) as u16;
            column.set_constraint(ColumnConstraint::Absolute(Width::Fixed(wide)));
        }
        table.to_string()
    }

    /// The report's rows, label and text, before either is laid out.
    fn rows(d: &Diagnostic) -> Vec<(&'static str, String)> {
        let mut rows = vec![(d.severity.label(), d.reason.clone())];
        if let Some(loc) = &d.location {
            rows.push(("Location", loc.to_string()));
            if let Some(ex) = &loc.excerpt {
                rows.push(("Source", ex.trim().to_string()));
            }
        }
        if let Some(name) = &d.stack_name {
            rows.push(("Stack", name.clone()));
        }
        rows
    }
}

/// What a report's frame takes of its width beside the two columns' text:
/// three border lines and a blank each side of each cell.
const FRAME: usize = 7;

/// The narrowest report `wrap_cell` lays out. Below it `comfy_table` shares
/// the width between the columns by rules this module does not model, and the
/// cell is handed over as it is.
const NARROWEST: usize = 40;

/// The columns left for the second column's text in a report `width` wide
/// whose widest label is `label`, or `None` where this module does not say.
fn room_for_text(width: usize, label: usize) -> Option<usize> {
    (width >= NARROWEST).then(|| width.saturating_sub(FRAME + label))
}

/// A margin `comfy_table` keeps: a column whose broken text leaves this many
/// columns free is narrowed to the text.
const WORTH_NARROWING: usize = 3;

/// The second column's cells with every line that is too wide broken where
/// `comfy_table` would break it, and the width they were broken for, so that
/// it finds nothing left to break — F186. `None` for the width where no line
/// is too wide, or `room` is not known: the cells are then as they came.
///
/// `comfy_table` breaks a word wider than its column by taking a line's worth
/// off the front and measuring and copying what is left, once for each line.
/// That is the square of the word's length: a `use` that failed on a URL of
/// 80,000 bytes took eleven seconds to say so. The same break is made here in
/// one pass over the word.
///
/// It breaks twice, as `comfy_table` does: once for the room there is, and,
/// if the longest line that makes is at least `WORTH_NARROWING` short of the
/// room, again for a column just that wide. The longest line is counted in
/// bytes, as it counts it.
///
/// The text is not shortened. The reference prints a message whole, and so
/// does this.
fn lay_out(texts: &[&str], room: Option<usize>) -> (Vec<String>, Option<usize>) {
    let whole = || texts.iter().map(|t| t.to_string()).collect();
    let Some(room) = room else {
        return (whole(), None);
    };
    if !texts.iter().flat_map(|t| t.split('\n')).any(|line| line.width() > room) {
        return (whole(), None);
    }
    let broken = |width: usize| -> Vec<Vec<String>> {
        texts.iter().map(|text| wrap_cell(text, width)).collect()
    };
    let mut cells = broken(room);
    let longest = cells.iter().flatten().map(String::len).max().unwrap_or(0);
    let mut width = room;
    if room.saturating_sub(longest) >= WORTH_NARROWING {
        width = longest;
        cells = broken(width);
    }
    (cells.iter().map(|lines| lines.join("\n")).collect(), Some(width))
}

/// A cell's lines, each one wider than `room` broken.
fn wrap_cell(text: &str, room: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.split('\n') {
        if line.width() > room {
            wrap_line(line, room, &mut lines);
        } else {
            lines.push(line.to_string());
        }
    }
    lines
}

/// A word of a line, or what is left of one after its front was taken.
struct Word<'a> {
    text: &'a str,
    /// The columns it takes. For what is left of a word this is the whole
    /// word's less what was taken, and not a new measurement: measuring again
    /// is the cost this module exists to avoid.
    width: usize,
    /// Every byte is a printable ASCII character, so a column is a byte.
    plain: bool,
}

impl<'a> Word<'a> {
    fn new(text: &'a str) -> Self {
        let plain = text.bytes().all(|b| (0x20..=0x7e).contains(&b));
        let width = if plain { text.len() } else { text.width() };
        Self { text, width, plain }
    }

    /// The front of the word that fits in `room` columns, and the rest.
    /// With `some`, a front that would be empty is one character instead,
    /// which is how a line that could hold nothing still moves on.
    fn cut(self, room: usize, some: bool) -> (&'a str, Word<'a>) {
        let (mut at, mut taken) = (0, 0);
        if self.plain {
            at = room.min(self.text.len());
            taken = at;
        } else {
            for g in self.text.graphemes(true) {
                if taken + g.width() > room {
                    break;
                }
                taken += g.width();
                at += g.len();
            }
        }
        if some
            && at == 0
            && let Some(c) = self.text.chars().next()
        {
            at = c.len_utf8();
            taken = c.width().unwrap_or(0);
        }
        let (front, rest) = self.text.split_at_checked(at).unwrap_or((self.text, ""));
        let width = if rest.is_empty() { 0 } else { self.width.saturating_sub(taken) };
        (front, Word { text: rest, width, plain: self.plain })
    }
}

/// The fewest free columns a line must have for another word to be tried on
/// it: `comfy_table`'s `MIN_FREE_CHARS`.
const FEWEST_FREE: usize = 2;

/// One line broken into lines of at most `room` columns, as `comfy_table`'s
/// `split_line` breaks it: words are kept whole while they fit, and a word
/// wider than a line fills what is left of the current one.
///
/// Every pass of the loop either takes a word off `words`, or puts back a
/// word shorter than the one it took, or puts one back and ends a line that
/// was not empty, which the next pass cannot do again. So it ends.
fn wrap_line(line: &str, room: usize, lines: &mut Vec<String>) {
    let mut words: Vec<Word> = line.split(' ').map(Word::new).collect();
    words.reverse();
    let mut current = String::new();
    while let Some(next) = words.pop() {
        let held = current.width();
        let gap = usize::from(!current.is_empty());
        let free = room.saturating_sub(held).saturating_sub(gap);

        if held + gap + next.width <= room {
            if gap == 1 {
                current.push(' ');
            }
            current.push_str(next.text);
            end_if_full(&mut current, room, lines);
            continue;
        }
        if gap == 1 && free <= FEWEST_FREE {
            words.push(next);
            lines.push(std::mem::take(&mut current));
            continue;
        }
        if next.width > room {
            if gap == 1 {
                current.push(' ');
            }
            let (front, rest) = next.cut(free, gap == 0);
            current.push_str(front);
            words.push(rest);
            lines.push(std::mem::take(&mut current));
            continue;
        }
        lines.push(std::mem::take(&mut current));
        current.push_str(next.text);
        end_if_full(&mut current, room, lines);
    }
    if !current.is_empty() {
        lines.push(current);
    }
}

/// End the line if no further word could be tried on it.
fn end_if_full(current: &mut String, room: usize, lines: &mut Vec<String>) {
    if current.width() > room.saturating_sub(FEWEST_FREE) {
        lines.push(std::mem::take(current));
    }
}

/// The empty-stack box, and the frame for a non-empty one. Shared with
/// `debug.display_stack` so a dumped stack looks like a displayed one.
///
/// `debug.display_stack` keeps the raw rendering and the unbounded box, because
/// the goldens capture it byte for byte. Only the *report* summarises.
fn stack_box(rows: &[String]) -> String {
    crate::console::draw_box_rows(rows)
}

impl Reporter for TextReporter {
    /// Only for a fatal report, because only a fatal report shows one: a
    /// warning or notice is one line with no stack (below). Collecting one
    /// there was rendering every value for nothing, and it is what kept
    /// RFC-0005 §S5 from promoting values across a call under this reporter
    /// (D45).
    fn wants_stack(&self, severity: Severity) -> bool {
        self.dump_stack && severity.is_fatal()
    }

    fn wants_raw_values(&self) -> bool {
        self.raw_values
    }

    /// Leave room for the box's own borders and padding.
    fn value_width(&self) -> usize {
        self.width.saturating_sub(4).max(16)
    }

    fn report(&mut self, d: &Diagnostic) {
        if !d.severity.is_fatal() {
            // Mindful: one line, on stderr, no table and no stack. A warning
            // that interrupts the program's own output is worse than one that
            // is easy to miss.
            let mut err = std::io::stderr();
            let _ = match &d.location {
                Some(l) => writeln!(err, "{}: {} ({l})", d.severity.label(), d.reason),
                None => writeln!(err, "{}: {}", d.severity.label(), d.reason),
            };
            return;
        }

        let text = self.render_fatal(d);
        if self.fatal_to_stdout {
            bund2_api::sayln!("{text}");
        } else {
            bund2_api::errln!("{text}");
        }

        if let Some(rows) = &d.stack {
            // Two spaces, and they are not a typo. The reference builds the
            // banner with a trailing space —
            // `format!("{}{}{}{}{}{} ", …)` — and then prints it with
            // `bund2_api::sayln!("{} {}", &bund, …)`
            // (`reference/Bund/src/stdlib/helpers/print_error.rs:133,153`),
            // so the coloured path emits `[BUND]  Content`. Its plain path at
            // `:126` emits one space, but the goldens were captured with
            // colour on and ANSI stripped afterwards, so two is what every
            // captured golden holds.
            bund2_api::sayln!("[BUND]  Content of the stack");
            bund2_api::sayln!("{}", stack_box(rows));
        }
        // **The workbench is carried but not printed.** The reference's error
        // path calls `stdlib_debug_display_stack` and nothing else
        // (`reference/Bund/src/stdlib/helpers/print_error.rs:126-131`), so a
        // failing program emits exactly one box — which is the frame D36
        // preserves, and what every F66 golden captured. Printing a second box
        // here made Bund2's output one box longer than the oracle's on every
        // uncaught error.
        //
        // `Diagnostic::workbench` stays populated: it is part of the structured
        // value a `Reporter` receives, and a TUI laying the parts out itself is
        // exactly the consumer D36's seam exists for. Only this renderer, whose
        // output the goldens pin, declines to show it.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_api::diag::{CollectingReporter, Location, Severity};

    #[test]
    fn a_fatal_report_names_the_reason_and_the_position() {
        let r = TextReporter::default();
        let d = Diagnostic::error("Stack is too shallow for inline ADD()").at(Location {
            file: Some("p.bund".into()),
            line: 3,
            column: 5,
            excerpt: Some("  1 +".into()),
        });
        let text = r.render_fatal(&d);
        assert!(text.contains("Stack is too shallow"), "{text}");
        assert!(text.contains("p.bund:3:5"), "{text}");
        assert!(text.contains("1 +"), "the source line is shown: {text}");
    }

    /// The report as it was before F186: each cell handed to `comfy_table`
    /// whole, for it to break.
    fn broken_by_the_table(width: usize, d: &Diagnostic) -> String {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(width as u16);
        for (label, text) in TextReporter::rows(d) {
            table.add_row(vec![label.to_string(), text]);
        }
        table.to_string()
    }

    /// A text of `n` pieces drawn from `seed`: words short and long, runs of
    /// blanks, line ends, wide and combining characters.
    fn drawn(seed: &mut u64, n: usize) -> String {
        const PIECES: [&str; 14] = [
            "a", "word", " ", "  ", "\n", "http://host/", "é", "日本", "e\u{301}", "-", "x=1&y=2",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "ü", "\t",
        ];
        let mut text = String::new();
        for _ in 0..n {
            *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let pick = (*seed >> 33) as usize;
            let piece = PIECES[pick % PIECES.len()];
            // Long words are the case: repeat a piece without a blank in it.
            let times = if piece.trim().is_empty() { 1 } else { 1 + (pick / 14) % 40 };
            for _ in 0..times {
                text.push_str(piece);
            }
        }
        text
    }

    /// F186's fix changes how long a report takes and no byte of it: breaking
    /// a cell here and letting `comfy_table` break it give the same report,
    /// for every width from the narrowest this module lays out.
    #[test]
    fn a_cell_is_broken_where_the_table_would_break_it() {
        let mut seed = 186;
        for width in NARROWEST..=200 {
            for round in 0..12 {
                let mut d = Diagnostic::error(drawn(&mut seed, 1 + round * 3));
                if round % 2 == 0 {
                    d = d.at(Location {
                        file: Some("p.bund".into()),
                        line: 3,
                        column: 5,
                        excerpt: Some(drawn(&mut seed, 2 + round * 2)),
                    });
                }
                let r = TextReporter { width, ..TextReporter::default() };
                assert_eq!(
                    r.render_fatal(&d),
                    broken_by_the_table(width, &d),
                    "width {width}, reason {:?}",
                    d.reason
                );
            }
        }
    }

    /// F186: a message of a megabyte with nowhere to break is reported, whole.
    /// Broken by the table this did not finish; the test is that it does.
    #[test]
    fn a_long_message_is_reported_whole() {
        let word = "a".repeat(1 << 20);
        let d = Diagnostic::error(format!("USE can not get from {word}")).at(Location {
            file: Some("p.bund".into()),
            line: 1,
            column: 1,
            excerpt: Some(format!("\"{word}\" use")),
        });
        let text = TextReporter { width: 100, ..TextReporter::default() }.render_fatal(&d);
        let kept = text.bytes().filter(|b| *b == b'a').count();
        assert!(kept >= 2 << 20, "{kept} of the message's and the excerpt's bytes");
        assert!(text.lines().all(|l| l.width() <= 100));
    }

    /// No location means no `Location` row. An empty one would assert a
    /// position that is not known.
    #[test]
    fn a_missing_location_is_omitted_not_blanked() {
        let r = TextReporter::default();
        let text = r.render_fatal(&Diagnostic::error("boom"));
        assert!(!text.contains("Location"), "{text}");
    }

    /// The stack dump is a switch, and it gates *collection*, not just
    /// display — the evaluator asks before it renders every value.
    #[test]
    fn the_stack_dump_is_a_switch() {
        assert!(TextReporter::new(true).wants_stack(Severity::Error));
        assert!(!TextReporter::new(false).wants_stack(Severity::Error));
    }

    /// A warning or notice renders as one line with no stack, so collecting
    /// one for it is wasted — and would make every native that reports
    /// mid-body read the whole stack (D45, RFC-0005 §S5).
    #[test]
    fn no_stack_is_wanted_where_none_is_shown() {
        let r = TextReporter::new(true);
        assert!(!r.wants_stack(Severity::Warning));
        assert!(!r.wants_stack(Severity::Notice));
    }

    /// The severity split is about delivery: a warning gets a line, an error
    /// gets a report.
    #[test]
    fn severity_decides_the_shape() {
        assert!(Severity::Error.is_fatal());
        assert!(!Severity::Warning.is_fatal());
        let mut c = CollectingReporter::default();
        c.report(&Diagnostic::warning("careful"));
        assert_eq!(c.seen[0].severity, Severity::Warning);
    }
}
