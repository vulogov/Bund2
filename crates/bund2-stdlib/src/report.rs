//! The terminal reporter — one implementation of `Reporter`, not the only one.
//!
//! # What is preserved and what is not
//!
//! The **frame** is the reference's: a `comfy_table` report with an `Error` row
//! and a `Location` row, then `[BUND] Content of the stack` and the stack box,
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

use bund2_api::diag::{Diagnostic, Reporter};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{ContentArrangement, Table};

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
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            // Without this the report grows to its widest cell, which on a
            // piped stream is unbounded.
            .set_width(self.width.min(u16::MAX as usize) as u16)
            .add_row(vec![d.severity.label().to_string(), d.reason.clone()]);
        if let Some(loc) = &d.location {
            table.add_row(vec!["Location".to_string(), loc.to_string()]);
            if let Some(ex) = &loc.excerpt {
                table.add_row(vec!["Source".to_string(), ex.trim().to_string()]);
            }
        }
        if let Some(name) = &d.stack_name {
            table.add_row(vec!["Stack".to_string(), name.clone()]);
        }
        table.to_string()
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
    fn wants_stack(&self) -> bool {
        self.dump_stack
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
            println!("{text}");
        } else {
            eprintln!("{text}");
        }

        if let Some(rows) = &d.stack {
            println!("[BUND] Content of the stack");
            println!("{}", stack_box(rows));
        }
        if let Some(rows) = &d.workbench {
            println!("{}", stack_box(rows));
        }
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
        assert!(TextReporter::new(true).wants_stack());
        assert!(!TextReporter::new(false).wants_stack());
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
