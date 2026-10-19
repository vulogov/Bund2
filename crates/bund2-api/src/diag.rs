//! Diagnostics: what went wrong, where, and how loudly to say so.
//!
//! **Structured, not rendered.** A [`Diagnostic`] carries the reason, the
//! position and — only when asked for — a snapshot of the stacks. Turning that
//! into text is a [`Reporter`]'s job, and there can be more than one: a
//! terminal writer today, a TUI pane later, a test collector in between.
//! Nothing in the language layer formats anything.
//!
//! # Why not the reference's shape exactly
//!
//! The reference reports an error as a two-row table whose `Location` row holds
//! a **Rust** source path — the file and line inside `rust_multistackvm` where
//! `bail!` was written — recovered by regex from the tail of the message
//! (`reference/Bund/src/stdlib/helpers/print_error.rs:12-46,104-132`). On the
//! capture machine that path runs through `~/.cargo/registry`, which is F66 and
//! is why four goldens cannot reproduce anywhere.
//!
//! Bund2 keeps the frame and changes what fills it. The `Location` row names a
//! position in **the Bund program** — the thing whose author can act on it —
//! because the parser has spans and the reference never did. That is a
//! deviation, recorded as D36.

/// How loud a diagnostic should be.
///
/// The distinction is about **delivery**, not about severity in the abstract:
/// an `Error` has stopped the program and has earned a table, a `Warning` has
/// not and gets one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Evaluation stopped here.
    Error,
    /// Something is wrong and evaluation continued.
    Warning,
    /// The program asked for this to be said — `?error`'s report, for
    /// instance. Not a fault.
    Notice,
}

impl Severity {
    /// Whether this one has stopped the program.
    pub fn is_fatal(self) -> bool {
        self == Severity::Error
    }

    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Notice => "Notice",
        }
    }
}

/// Where in the *Bund program* something happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// The source file, when it came from one.
    pub file: Option<String>,
    /// 1-based.
    pub line: usize,
    /// 1-based, in characters.
    pub column: usize,
    /// The source line itself, so a reporter can show it without re-reading
    /// the file — and so a TUI can highlight within it.
    pub excerpt: Option<String>,
    /// The byte range of the term in the source, when the parser gave one.
    /// `line` and `column` say where a reader looks; this says what a
    /// renderer underlines (D173), and a TUI can select it.
    pub span: Option<(usize, usize)>,
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.file {
            Some(p) => write!(f, "{p}:{}:{}", self.line, self.column),
            None => write!(f, "line {}, column {}", self.line, self.column),
        }
    }
}

/// One thing worth telling the user.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// The precise reason, with no location glued to the end of it. The
    /// reference concatenates the two and then recovers them by regex; keeping
    /// them apart means a reporter never has to parse a message.
    pub reason: String,
    pub location: Option<Location>,
    /// The current stack, rendered a row per value — **present only when the
    /// dump is switched on**, so that deciding to collect it is separate from
    /// deciding to show it.
    pub stack: Option<Vec<String>>,
    pub workbench: Option<Vec<String>>,
    /// The stack the program was on. A multi-stack language can fail on a
    /// stack the reader did not expect to be current.
    pub stack_name: Option<String>,
    /// The bodies that were running when a fatal error was raised, innermost
    /// first, one line each — the Bund traceback (D173). **Present only when
    /// the reporter asked** ([`Reporter::wants_frames`]), since the evaluator
    /// renders it at the failure, before the frames unwind, and a program
    /// that catches errors in a loop would pay for every one.
    pub frames: Option<Vec<String>>,
    /// **The failure a `?try` caught, by its own text** — set on the report
    /// the `except` block makes of it, when the reporter wanted frames. The
    /// reason keeps the reference's wording, wrappers and all, because a
    /// program reads it as data; this is the innermost failure without them,
    /// which is what a traceback leads with.
    pub cause: Option<String>,
    /// The text [`Diagnostic::location`] points into, when that is not a file
    /// the reporter holds — a string `bund.eval` ran, say. The location's
    /// `file` then names it.
    pub source: Option<String>,
    /// **Where inside a body the failure was** — RFC-0003 §S5. `location`
    /// stays the top-level step that led there, which is what the reference's
    /// table shows and the goldens hold; this is the place in the lambda,
    /// for a renderer that can show both.
    pub inner: Option<Location>,
}

impl Diagnostic {
    pub fn error(reason: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            reason: reason.into(),
            location: None,
            stack: None,
            workbench: None,
            stack_name: None,
            frames: None,
            cause: None,
            source: None,
            inner: None,
        }
    }

    pub fn warning(reason: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(reason)
        }
    }

    pub fn notice(reason: impl Into<String>) -> Self {
        Self {
            severity: Severity::Notice,
            ..Self::error(reason)
        }
    }

    pub fn at(mut self, l: Location) -> Self {
        self.location = Some(l);
        self
    }

    pub fn on_stack(mut self, name: impl Into<String>) -> Self {
        self.stack_name = Some(name.into());
        self
    }

    pub fn with_stack(mut self, rows: Vec<String>) -> Self {
        self.stack = Some(rows);
        self
    }

    pub fn with_workbench(mut self, rows: Vec<String>) -> Self {
        self.workbench = Some(rows);
        self
    }

    pub fn with_frames(mut self, frames: Vec<String>) -> Self {
        self.frames = Some(frames);
        self
    }
}

/// Somewhere a diagnostic can go.
///
/// **This is the TUI hook.** A terminal front end implements it by writing
/// text; a TUI implements it by pushing onto a pane's model and never touching
/// stdout. Because a [`Diagnostic`] is structured, the TUI can lay out the
/// reason, the location and the stack independently — which is exactly what it
/// could not do if the language layer handed it a formatted string.
///
/// Reporters are expected to be cheap and non-panicking: they run on a failure
/// path, and a reporter that fails while reporting leaves nothing to read.
pub trait Reporter {
    fn report(&mut self, d: &Diagnostic);

    /// Whether this reporter wants a stack snapshot collected for a diagnostic
    /// of this severity.
    ///
    /// Collecting means rendering every value on every stack, which is not
    /// free on a deep stack and is wasted if nothing will show it. The
    /// evaluator asks before it collects.
    ///
    /// **Per severity, because the answer differs by severity** (D45). A
    /// terminal shows a stack only under a fatal report, and a fatal report
    /// is made after the error has returned to the top. A warning or notice is
    /// made mid-body, by a native, and a snapshot taken there is one RFC-0005
    /// §S5 has to make exact. Asking for one only where it will be shown keeps
    /// that cost to the reporters that actually show it.
    fn wants_stack(&self, _severity: Severity) -> bool {
        false
    }

    /// Whether snapshots should carry the **raw** `Debug` rendering rather
    /// than a compact summary.
    ///
    /// Off by default. The raw form names every header field — id, stamp, dt,
    /// q, attr, curr, tags — and runs about 150 columns for a single integer.
    /// That is what a golden captures and what a debug session wants; it is
    /// not what belongs in an error a person has to read.
    fn wants_raw_values(&self) -> bool {
        false
    }

    /// How wide a single value may be rendered. Summaries are cut to fit, so
    /// a row can never overflow the terminal.
    fn value_width(&self) -> usize {
        100
    }

    /// Whether a fatal diagnostic should carry the frames that were running
    /// — [`Diagnostic::frames`]. Off by default: the evaluator renders them
    /// at the failure, and nothing should pay for a traceback nobody shows.
    fn wants_frames(&self) -> bool {
        false
    }
}

/// A reporter that keeps what it is given.
///
/// For tests, and for a TUI that wants to drain diagnostics on its own
/// schedule rather than being called at the moment of failure.
#[derive(Debug, Default)]
pub struct CollectingReporter {
    pub seen: Vec<Diagnostic>,
    /// Wants a snapshot for every severity. That is the strictest reporter
    /// there can be, which is what a test of snapshot exactness wants.
    pub wants_stack: bool,
}

impl Reporter for CollectingReporter {
    fn report(&mut self, d: &Diagnostic) {
        self.seen.push(d.clone());
    }
    fn wants_stack(&self, _severity: Severity) -> bool {
        self.wants_stack
    }
}

/// A reporter that discards. The default, so that a `Vm` built in a test does
/// not write to anyone's terminal.
#[derive(Debug, Default)]
pub struct SilentReporter;

impl Reporter for SilentReporter {
    fn report(&mut self, _: &Diagnostic) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_warning_is_not_fatal() {
        assert!(Severity::Error.is_fatal());
        assert!(!Severity::Warning.is_fatal());
        assert!(!Severity::Notice.is_fatal());
    }

    /// The reason and the location stay apart. The reference glues them and
    /// recovers them by regex (`print_error.rs:12-46`), which fails whenever a
    /// message happens to end in a parenthesis.
    #[test]
    fn the_reason_carries_no_location() {
        let d = Diagnostic::error("Stack is too shallow").at(Location {
            file: Some("p.bund".into()),
            line: 3,
            column: 5,
            excerpt: None,
            span: None,
        });
        assert_eq!(d.reason, "Stack is too shallow");
        assert_eq!(d.location.unwrap().to_string(), "p.bund:3:5");
    }

    #[test]
    fn a_collecting_reporter_keeps_everything() {
        let mut r = CollectingReporter::default();
        r.report(&Diagnostic::warning("one"));
        r.report(&Diagnostic::notice("two"));
        assert_eq!(r.seen.len(), 2);
        assert_eq!(r.seen[0].severity, Severity::Warning);
    }

    /// Collecting a stack snapshot is opt-in, because rendering one is not free
    /// and is wasted if no reporter will show it.
    #[test]
    fn the_stack_is_only_collected_when_wanted() {
        assert!(!SilentReporter.wants_stack(Severity::Error));
        let r = CollectingReporter {
            wants_stack: true,
            ..Default::default()
        };
        assert!(r.wants_stack(Severity::Error));
        assert!(r.wants_stack(Severity::Warning));
    }
}
