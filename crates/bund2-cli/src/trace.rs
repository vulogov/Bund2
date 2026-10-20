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
//! error, coloured where that is a terminal — except **a failure a `?try`
//! caught**, which the interpreter hands on with its frames and, for a string
//! `bund.eval` ran, the string and where in it. That draws as a traceback on
//! standard error, led by the failure's own text rather than the reference's
//! wrappers, which embed each value's raw rendering.

use std::ops::Range;

use ariadne::{Color, Config, IndexType, Label, Report, ReportKind};
use bund2_api::diag::{Diagnostic, Location, Reporter, Severity};
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
    /// The program's `(file, text)` first, then every included file (D176),
    /// so a traceback is drawn over the file the diagnostic names.
    sources: Vec<(String, String)>,
    trace: bool,
}

impl TraceReporter {
    pub fn new(inner: TextReporter, src: &str, file: &str, trace: bool) -> Self {
        Self {
            inner,
            sources: vec![(file.to_string(), src.to_string())],
            trace,
        }
    }

    /// The files a bundle's `--include` put ahead of the program, as
    /// `(path, text)`, so a failure inside one is drawn over its own text.
    pub fn with_sources(mut self, more: &[(String, String)]) -> Self {
        self.sources.extend(more.iter().cloned());
        self
    }

    /// The file a diagnostic names, or the program when it names none or
    /// one this reporter was not given.
    fn source_for(&self, file: Option<&str>) -> (&str, &str) {
        file.and_then(|f| self.sources.iter().find(|(name, _)| name == f))
            .or(self.sources.first())
            .map(|(name, text)| (name.as_str(), text.as_str()))
            .unwrap_or(("", ""))
    }

    /// The report, as text, or `None` where `ariadne` could not write it —
    /// the caller then draws the table, so a failure to report never hides
    /// the failure being reported.
    fn render(&self, d: &Diagnostic) -> Option<String> {
        self.draw(d, ReportKind::Error, console::colors_enabled())
    }

    /// A caught failure's report, or `None` where it carries nothing a
    /// traceback would add: no frames and no position.
    fn render_caught(&self, d: &Diagnostic) -> Option<String> {
        let cause = d.cause.as_deref()?;
        let colour = console::colors_enabled_stderr();
        if let Some(text) = self.draw(d, ReportKind::Custom("Caught", Color::Cyan), colour) {
            return Some(text);
        }
        // No position to underline — the failing body was built at run time,
        // so nothing was written where it could point: the reason and the
        // frames.
        let frames = d.frames.as_ref().filter(|f| !f.is_empty())?;
        let label = console::style("Caught").for_stderr().bold().cyan();
        Some(format!(
            "{label}: {cause}\n  traceback, innermost first:\n    {}\n",
            frames.join("\n    ")
        ))
    }

    /// The text a location points into: the string the diagnostic carries
    /// when the location names it, else a file this reporter holds by that
    /// exact name. `None` for a file it was not given, which is never drawn
    /// over another file's text.
    fn text_of<'a>(&'a self, d: &'a Diagnostic, l: &Location) -> Option<(&'a str, &'a str)> {
        let file = l.file.as_deref()?;
        if let Some((name, text)) = &d.source
            && name == file
        {
            return Some((name.as_str(), text.as_str()));
        }
        self.sources
            .iter()
            .find(|(name, _)| name == file)
            .map(|(name, text)| (name.as_str(), text.as_str()))
    }

    fn draw(&self, d: &Diagnostic, kind: ReportKind<'_>, colour: bool) -> Option<String> {
        let loc = d.location.as_ref()?;
        let (id, src) = self
            .text_of(d, loc)
            .unwrap_or_else(|| self.source_for(loc.file.as_deref()));
        let range = |l: &Location, src: &str| {
            let span = l
                .span
                .map(|(s, e)| s..e)
                .unwrap_or_else(|| guess_span(src, l.line, l.column));
            let start = span.start.min(src.len());
            start..span.end.clamp(start, src.len())
        };
        let at = range(loc, src);
        // **RFC-0003 §S5: the value inside — a lambda's, or a string's that
        // `bund.eval` ran — when its text is to hand.** It is underlined as
        // what failed, and the top-level step becomes what reached it, in
        // its own file when that is another. A place whose text this
        // reporter does not have is left to the frames' lines, which name it.
        let inner = d
            .inner
            .as_ref()
            .and_then(|i| self.text_of(d, i).map(|(iid, isrc)| (iid, isrc, range(i, isrc))))
            .filter(|(iid, _, r)| *iid != id || *r != at);
        let (pid, primary, message) = match &inner {
            Some((iid, _, r)) => (*iid, r.clone(), "the value that failed, inside"),
            None => (id, at.clone(), "the step that failed began here"),
        };
        let mut report = Report::build(kind, (pid.to_string(), primary.clone()))
            .with_config(
                Config::default()
                    .with_color(colour)
                    .with_index_type(IndexType::Byte),
            )
            .with_message(d.cause.as_deref().unwrap_or(&d.reason))
            .with_label(
                Label::new((pid.to_string(), primary))
                    .with_message(message)
                    .with_color(Color::Red),
            );
        let mut texts = vec![(id.to_string(), src.to_string())];
        if let Some((iid, isrc, _)) = &inner {
            report = report.with_label(
                Label::new((id.to_string(), at))
                    .with_message("reached from this step")
                    .with_color(Color::Yellow),
            );
            if *iid != id {
                texts.push((iid.to_string(), isrc.to_string()));
            }
        }
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
            .write(ariadne::sources(texts), &mut out)
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
            if self.trace
                && let Some(text) = self.render_caught(d)
            {
                bund2_api::err!("{text}");
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

    /// A caught failure is drawn over the string it failed in, led by its own
    /// text: the reference's wrappers, and the raw value they embed, are not
    /// in it.
    #[test]
    fn a_caught_failure_is_drawn_over_its_string() {
        console::set_colors_enabled_stderr(false);
        let mut d = Diagnostic::notice(
            "Lambda content evaluation returned error: Attempt to evaluate value \
             Value { id: \"x\" } returned error: DIV returns error: Integer division to 0.0",
        )
        .at(Location {
            file: Some("bund.eval".into()),
            line: 1,
            column: 5,
            excerpt: None,
            span: Some((4, 5)),
        })
        .with_frames(vec!["#0  a lambda at 1".into()]);
        d.cause = Some("DIV returns error: Integer division to 0.0".into());
        d.source = Some(("bund.eval".into(), "0 1 /".into()));
        let text = reporter(":other { } register\n", true)
            .render_caught(&d)
            .expect("drawn");
        assert!(text.contains("Caught"), "{text}");
        assert!(text.contains("bund.eval:1:5") && text.contains("0 1 /"), "{text}");
        assert!(text.contains("Integer division to 0.0"), "{text}");
        assert!(!text.contains("Value {"), "{text}");
        assert!(text.contains("#0  a lambda at 1"), "{text}");

        // Without a position, the cause and the frames; without a cause, the
        // one line the table reporter writes.
        d.location = None;
        let text = reporter("f\n", true).render_caught(&d).expect("drawn");
        assert!(text.starts_with("Caught: DIV") && text.contains("#0  a lambda at 1"), "{text}");
        d.cause = None;
        assert!(reporter("f\n", true).render_caught(&d).is_none());
    }

    /// RFC-0003 §S5: with a place inside a lambda, that is what is
    /// underlined, and the top-level step is what reached it.
    #[test]
    fn a_failure_in_a_lambda_is_underlined_there_and_reached_from_the_step() {
        console::set_colors_enabled(false);
        let src = ":g { 1 list / } register\n:f { 7 g } register\nf\n";
        let mut d = failing_at(3, 1, Some((45, 46)));
        d.inner = Some(Location {
            file: Some("p.bund".into()),
            line: 1,
            column: 13,
            excerpt: None,
            span: Some((12, 13)),
        });
        let text = reporter(src, true).render(&d).expect("rendered");
        assert!(text.contains("p.bund:1:13"), "{text}");
        assert!(text.contains("the value that failed, inside"), "{text}");
        assert!(text.contains("reached from this step"), "{text}");
        // A place in a file this reporter was not given is not drawn over
        // the wrong text.
        d.inner = d.inner.map(|mut l| {
            l.file = Some("other.bund".into());
            l
        });
        let text = reporter(src, true).render(&d).expect("rendered");
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
