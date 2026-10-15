//! The traceback: a fatal report drawn with `ariadne` — D173.
//!
//! **Two renderings, one seam.** [`bund2_stdlib::report::TextReporter`]
//! draws the reference's two-row table, which every golden captures and D36
//! preserves. This one, on a terminal, draws the same diagnostic the way a
//! compiler does: the reason, the source with the failing term underlined,
//! the stack the program was on, and the frames that were running —
//! innermost first — which the table never had room for. Both receive the
//! one structured `Diagnostic`; neither parses the other's text.
//!
//! **Which one draws is decided once, by [`Mode`].** `auto` picks the
//! traceback when standard output is a terminal and the table when it is
//! not, so `cargo xtask conform`, a pipe and a capture see exactly the bytes
//! they always saw, and a person at a terminal sees the traceback — the same
//! line the reference draws for colour. `--report box` and `--report trace`
//! force either; a bundle reads `BUND2_REPORT`.
//!
//! A diagnostic with no location falls back to the table, since there is
//! nothing to underline. Warnings and notices stay one line on standard
//! error, coloured where that is a terminal.

use std::ops::Range;

use ariadne::{Color, Config, IndexType, Label, Report, ReportKind, Source};
use bund2_api::diag::{Diagnostic, Reporter, Severity};
use bund2_stdlib::report::{TextReporter, stack_box};

/// How a fatal report is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The traceback on a terminal, the table elsewhere.
    Auto,
    /// The reference's table, always.
    Box,
    /// The traceback, always — for a pipe that wants it.
    Trace,
}

impl Mode {
    pub fn parse(text: &str) -> Result<Mode, String> {
        match text {
            "auto" => Ok(Mode::Auto),
            "box" => Ok(Mode::Box),
            "trace" => Ok(Mode::Trace),
            other => Err(format!(
                "`--report {other}` names no rendering; use auto, box or trace"
            )),
        }
    }

    /// `BUND2_REPORT`, for a bundle; unset or unknown is `auto`.
    pub fn from_env() -> Mode {
        std::env::var("BUND2_REPORT")
            .ok()
            .and_then(|v| Mode::parse(&v).ok())
            .unwrap_or(Mode::Auto)
    }

    /// Whether the traceback draws, given where standard output goes.
    pub fn traces(self) -> bool {
        match self {
            Mode::Auto => console::user_attended(),
            Mode::Box => false,
            Mode::Trace => true,
        }
    }
}

/// The CLI's reporter: the table's behaviour, with the traceback in front of
/// it where [`Mode`] says so.
pub struct TraceReporter {
    inner: TextReporter,
    src: String,
    file: String,
    trace: bool,
}

impl TraceReporter {
    pub fn new(inner: TextReporter, src: &str, file: &str, trace: bool) -> Self {
        Self {
            inner,
            src: src.to_string(),
            file: file.to_string(),
            trace,
        }
    }

    /// The report, as text, or `None` where `ariadne` could not write it —
    /// the caller then draws the table, so a failure to report never hides
    /// the failure being reported.
    fn render(&self, d: &Diagnostic) -> Option<String> {
        let loc = d.location.as_ref()?;
        let span = loc
            .span
            .map(|(s, e)| s..e)
            .unwrap_or_else(|| guess_span(&self.src, loc.line, loc.column));
        let start = span.start.min(self.src.len());
        let end = span.end.clamp(start, self.src.len());
        let id = self.file.as_str();
        let mut report = Report::build(ReportKind::Error, (id, start..end))
            .with_config(
                Config::default()
                    .with_color(console::colors_enabled())
                    .with_index_type(IndexType::Byte),
            )
            .with_message(&d.reason)
            .with_label(
                Label::new((id, start..end))
                    .with_message("the step that failed began here")
                    .with_color(Color::Red),
            );
        if let Some(name) = &d.stack_name {
            report = report.with_note(format!("on stack `{name}`"));
        }
        if let Some(frames) = &d.frames
            && !frames.is_empty()
        {
            report = report.with_help(format!(
                "traceback, innermost first:\n  {}",
                frames.join("\n  ")
            ));
        }
        let mut out = Vec::new();
        report
            .finish()
            .write((id, Source::from(self.src.as_str())), &mut out)
            .ok()?;
        Some(String::from_utf8_lossy(&out).into_owned())
    }
}

/// The term at `line`:`column` — from there to the next blank — when the
/// diagnostic carries no span of its own.
fn guess_span(src: &str, line: usize, column: usize) -> Range<usize> {
    let line_start = src
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    let start = src[line_start..]
        .char_indices()
        .nth(column.saturating_sub(1))
        .map_or(line_start, |(i, _)| line_start + i);
    let end = src[start..]
        .find(char::is_whitespace)
        .map_or(src.len(), |i| start + i);
    start..end.max(start)
}

impl Reporter for TraceReporter {
    fn wants_stack(&self, severity: Severity) -> bool {
        self.inner.wants_stack(severity)
    }

    fn wants_raw_values(&self) -> bool {
        self.inner.wants_raw_values()
    }

    fn value_width(&self) -> usize {
        self.inner.value_width()
    }

    /// Only the traceback shows frames, so only it pays for them.
    fn wants_frames(&self) -> bool {
        self.trace
    }

    fn report(&mut self, d: &Diagnostic) {
        if !d.severity.is_fatal() {
            if self.inner.quiet && d.severity == Severity::Warning {
                return;
            }
            // Plain where standard error is not a terminal: the bytes are
            // the table reporter's, which a capture appends to the output.
            if !console::colors_enabled_stderr() {
                return self.inner.report(d);
            }
            let label = console::style(d.severity.label()).for_stderr().bold();
            let label = match d.severity {
                Severity::Warning => label.yellow(),
                _ => label.cyan(),
            };
            match &d.location {
                Some(l) => bund2_api::errln!("{label}: {} ({l})", d.reason),
                None => bund2_api::errln!("{label}: {}", d.reason),
            }
            return;
        }
        if !self.trace {
            return self.inner.report(d);
        }
        let Some(text) = self.render(d) else {
            return self.inner.report(d);
        };
        bund2_api::say!("{text}");
        if let Some(rows) = &d.stack {
            bund2_api::sayln!("[BUND]  Content of the stack");
            bund2_api::sayln!("{}", stack_box(rows));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_api::diag::Location;

    fn reporter(src: &str, trace: bool) -> TraceReporter {
        TraceReporter::new(TextReporter::new(true), src, "p.bund", trace)
    }

    fn failing_at(line: usize, column: usize, span: Option<(usize, usize)>) -> Diagnostic {
        Diagnostic::error("DIV returns error: Incompartible operation for the list")
            .at(Location {
                file: Some("p.bund".into()),
                line,
                column,
                excerpt: None,
                span,
            })
            .on_stack("main")
            .with_frames(vec!["#1  f at 1".into(), "#0  g at 2".into()])
    }

    #[test]
    fn the_traceback_names_the_file_the_reason_the_stack_and_the_frames() {
        console::set_colors_enabled(false);
        let src = ":g { 1 list / } register\n:f { 7 g } register\nf\n";
        // `f` is the 46th byte: two lines of 25 and 20 bytes before it.
        let text = reporter(src, true)
            .render(&failing_at(3, 1, Some((45, 46))))
            .expect("rendered");
        assert!(text.contains("p.bund:3:1"), "{text}");
        assert!(text.contains("Incompartible operation"), "{text}");
        assert!(text.contains("on stack `main`"), "{text}");
        assert!(
            text.contains("#1  f at 1") && text.contains("#0  g at 2"),
            "{text}"
        );
        assert!(text.contains("the step that failed began here"), "{text}");
    }

    #[test]
    fn a_span_is_guessed_from_the_position_when_none_was_given() {
        let src = "1 2 +\n\"a\" 3 swap -\n";
        assert_eq!(guess_span(src, 2, 5), 10..11);
        assert_eq!(guess_span(src, 1, 5), 4..5);
        // Past the end: empty, at the end, never out of range.
        assert_eq!(guess_span(src, 9, 9), src.len()..src.len());
    }

    #[test]
    fn modes_resolve_and_the_table_is_the_fallback_without_a_location() {
        assert!(!Mode::Box.traces());
        assert!(Mode::Trace.traces());
        assert!(Mode::parse("loud").is_err());
        let r = reporter("f\n", true);
        assert!(r.wants_frames());
        assert!(r.render(&Diagnostic::error("no position")).is_none());
        assert!(!reporter("f\n", false).wants_frames());
    }
}
