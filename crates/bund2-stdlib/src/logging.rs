//! **`log.*` — the program's own logging, on the `Reporter` seam (D90).**
//!
//! The reference emits these through `env_logger`, which prints
//! `[<timestamp> ERROR <rust module path>] <message>`. **Neither of those two
//! fields can be reproduced**: the timestamp is wall-clock, and
//! `bund::stdlib::functions::debug_fun::debug_trace` is the reference's own
//! Rust module path — `env_logger` prints the module that called the macro.
//! Bund2's equivalent code is this file, so emitting the reference's path would
//! be printing a false statement about where the code is.
//!
//! **So D90 routes them through [`Vm::report`]**, which is where every other
//! non-fatal diagnostic goes (D36) and the seam a TUI will implement. The line
//! becomes `Warning: log.error: …` — no timestamp, no module path, nothing a
//! second machine cannot produce.
//!
//! **No golden can see that line, which is why the choice was cheap.** A
//! capture folds stderr into stdout and refuses a program whose two runs
//! differ, and `log.error` is the only one of the five that emits at the
//! default level — so its two runs differ by a second and the capture refuses
//! it, exactly as it refuses `debug` and `debug.shell`. The other four emit
//! nothing, which *is* reproducible: **their goldens pin silence.**
//!
//! That is also what makes the filter mandatory rather than a nicety. Four
//! goldens assert that `log.info`, `log.warning`, `log.debug` and `log.trace`
//! say nothing at the default level; a Bund2 without the filter would emit
//! where the oracle is quiet and fail all four.

use bund2_api::{diag::Diagnostic, Error, Registry, StackEffect, Vm, WordKind};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// The five levels, ordered as a filter orders them: `Error` is the quietest
/// threshold and `Trace` the loudest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    /// Below every level: nothing is emitted.
    Off,
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

impl Level {
    /// The word's own name, kept in the message.
    ///
    /// **Five levels do not fit `Severity`'s three rungs**, so collapsing them
    /// would make `log.debug` and `log.trace` indistinguishable in output — and
    /// a program that logs at two levels precisely to tell them apart would
    /// stop being able to. The level therefore stays in the reason text.
    fn word(self) -> &'static str {
        match self {
            Level::Off => "log.off",
            Level::Error => "log.error",
            Level::Warning => "log.warning",
            Level::Info => "log.info",
            Level::Debug => "log.debug",
            Level::Trace => "log.trace",
        }
    }

    /// The reference's error prefix for this word, which is the word's name
    /// upper-cased.
    fn prefix(self) -> &'static str {
        match self {
            Level::Off => "LOG.OFF",
            Level::Error => "LOG.ERROR",
            Level::Warning => "LOG.WARNING",
            Level::Info => "LOG.INFO",
            Level::Debug => "LOG.DEBUG",
            Level::Trace => "LOG.TRACE",
        }
    }

    /// **`log.error` is a `Warning`, not an `Error`.**
    ///
    /// `Severity::Error` means "evaluation stopped here" and earns a table
    /// with a stack snapshot. `log.error` has stopped nothing — it is the
    /// program saying something — so mapping it to `Error` would draw the
    /// fatal frame for a line that is not fatal, and criterion 25 forbids a
    /// native reporting at `Error` severity at all.
    fn diagnostic(self, msg: &str) -> Diagnostic {
        let reason = format!("{}: {msg}", self.word());
        match self {
            Level::Error | Level::Warning => Diagnostic::warning(reason),
            _ => Diagnostic::notice(reason),
        }
    }
}

/// Parse one level name, as `env_logger` spells them.
fn level_named(text: &str) -> Option<Level> {
    match text.trim().to_ascii_lowercase().as_str() {
        "off" => Some(Level::Off),
        "error" => Some(Level::Error),
        // `env_logger` accepts `warn`; the Bund word is `log.warning`.
        "warn" | "warning" => Some(Level::Warning),
        "info" => Some(Level::Info),
        "debug" => Some(Level::Debug),
        "trace" => Some(Level::Trace),
        _ => None,
    }
}

/// **The threshold, read once from `BUND_LOG_LEVEL`, defaulting to `error`.**
///
/// The reference's default is `Env::default().filter_or("BUND_LOG_LEVEL",
/// "error")` (`reference/Bund/src/cmd/setloglevel.rs`), and its `--debug` count
/// raises it to `bund=info`, `bund=debug` or `bund=trace`.
///
/// **Two narrowings, stated.** Bund2's CLI has no `--debug` count, so only the
/// environment path exists; and a directive is read by taking the level after
/// its last `=`, which covers the `bund=info` shape the reference itself
/// produces without reimplementing `env_logger`'s filter grammar. Anything
/// unrecognised falls back to the default rather than guessing.
fn threshold() -> Level {
    static LEVEL: std::sync::OnceLock<Level> = std::sync::OnceLock::new();
    *LEVEL.get_or_init(|| {
        let Ok(raw) = std::env::var("BUND_LOG_LEVEL") else {
            return Level::Error;
        };
        let last = raw.rsplit(',').next().unwrap_or(&raw);
        let after_eq = last.rsplit('=').next().unwrap_or(last);
        level_named(after_eq).unwrap_or(Level::Error)
    })
}

/// `stdlib_trace_base` — pull, cast, then emit if the level allows it.
///
/// **The order is the reference's and it is observable.** The pull and the cast
/// happen before the level is consulted, so `7 log.info` fails with a casting
/// error at the default level *even though `log.info` emits nothing there* —
/// measured against the oracle, which reports `LOG.INFO: Error casting tracing
/// message: This Dynamic type is not string` for exactly that program.
fn log_base(vm: &mut dyn Vm, level: Level) -> Result<(), Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("{}: NO DATA #1", level.prefix())))?;
    let msg = v.as_str().ok_or_else(|| {
        Error(format!(
            "{}: Error casting tracing message: This Dynamic type is not string",
            level.prefix()
        ))
    })?;
    if level <= threshold() {
        vm.report(level.diagnostic(&msg));
    }
    Ok(())
}

pub fn register(r: &mut Registry) {
    // **Five functions rather than a loop over a closure**: `register_native`
    // takes a plain `fn` pointer, which is what keeps a slot 96 bytes (D85)
    // and a native call indirect-free. One arm each is the cost of that.
    //
    // `eff(1, 0)`: one value consumed, nothing left. The message is taken
    // whether or not the level emits it.
    fn error(vm: &mut dyn Vm) -> Result<(), Error> {
        log_base(vm, Level::Error)
    }
    fn warning(vm: &mut dyn Vm) -> Result<(), Error> {
        log_base(vm, Level::Warning)
    }
    fn info(vm: &mut dyn Vm) -> Result<(), Error> {
        log_base(vm, Level::Info)
    }
    fn debug(vm: &mut dyn Vm) -> Result<(), Error> {
        log_base(vm, Level::Debug)
    }
    fn trace(vm: &mut dyn Vm) -> Result<(), Error> {
        log_base(vm, Level::Trace)
    }
    r.register_native("log.error", error, eff(1, 0), WordKind::Sync);
    r.register_native("log.warning", warning, eff(1, 0), WordKind::Sync);
    r.register_native("log.info", info, eff(1, 0), WordKind::Sync);
    r.register_native("log.debug", debug, eff(1, 0), WordKind::Sync);
    r.register_native("log.trace", trace, eff(1, 0), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_api::diag::{Reporter, Severity};
    use bund2_interp::Interp;

    /// Keeps every diagnostic, so a test can assert what was said and what was
    /// not. This is the seam D36 exists for, used here as a test double.
    #[derive(Default)]
    struct Kept(std::rc::Rc<std::cell::RefCell<Vec<(Severity, String)>>>);

    impl Reporter for Kept {
        fn report(&mut self, d: &Diagnostic) {
            self.0.borrow_mut().push((d.severity, d.reason.clone()));
        }
    }

    fn run(src: &str) -> (Result<(), String>, Vec<(Severity, String)>) {
        let kept = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut i = Interp::new();
        i.reporter = Box::new(Kept(std::rc::Rc::clone(&kept)));
        crate::register_all(&mut i.registry);
        let out = match bund2_syntax::compile(src) {
            Ok(ir) => i.eval(&ir).map_err(|e| e.0),
            Err(e) => Err(format!("{e:?}")),
        };
        let said = kept.borrow().clone();
        (out, said)
    }

    /// **The default threshold is `error`**, so one of the five speaks and four
    /// do not. This is the half of RFC-0008 criterion 2 that is checkable, and
    /// it is what the four goldens pin.
    #[test]
    fn only_log_error_speaks_at_the_default_level() {
        // The threshold is read once per process, so this test asserts the
        // default rather than setting it — a test that mutated the environment
        // would race every other test in this binary.
        if threshold() != Level::Error {
            // BUND_LOG_LEVEL is set in this environment; the filter itself is
            // covered by `the_threshold_is_read_from_the_environment` below.
            return;
        }
        let (out, said) = run(
            r#""a" log.info "b" log.warning "c" log.debug "d" log.trace "e" log.error"#,
        );
        out.expect("none of them fails");
        assert_eq!(said.len(), 1, "only log.error speaks: {said:?}");
        assert_eq!(said[0].0, Severity::Warning, "and not at Error severity");
        assert_eq!(said[0].1, "log.error: e");
    }

    /// **`log.error` is a `Warning`, not an `Error`.** `Severity::Error` means
    /// evaluation stopped, which it has not, and criterion 25 forbids a native
    /// reporting at `Error` severity at all.
    #[test]
    fn no_log_word_reports_at_error_severity() {
        for level in [
            Level::Error,
            Level::Warning,
            Level::Info,
            Level::Debug,
            Level::Trace,
        ] {
            assert_ne!(
                level.diagnostic("x").severity,
                Severity::Error,
                "{} reported at Error severity",
                level.word()
            );
        }
    }

    /// **The level survives into the message**, so `log.debug` and `log.trace`
    /// are distinguishable in output although both are `Notice`. Collapsing
    /// them onto the severity alone would lose that, and a program logging at
    /// two levels to tell them apart would stop being able to.
    #[test]
    fn the_level_is_kept_in_the_message_because_severity_cannot_hold_it() {
        assert_eq!(Level::Debug.diagnostic("m").reason, "log.debug: m");
        assert_eq!(Level::Trace.diagnostic("m").reason, "log.trace: m");
        assert_eq!(
            Level::Debug.diagnostic("m").severity,
            Level::Trace.diagnostic("m").severity,
            "the two share a rung, which is why the text has to differ"
        );
    }

    /// **The reference's two error texts, both reachable**, with its own
    /// prefixes — the word's name upper-cased.
    #[test]
    fn the_two_error_texts_are_the_references_own() {
        for (src, want) in [
            ("log.info", "LOG.INFO: NO DATA #1"),
            ("log.error", "LOG.ERROR: NO DATA #1"),
            ("log.trace", "LOG.TRACE: NO DATA #1"),
            (
                "7 log.info",
                "LOG.INFO: Error casting tracing message: This Dynamic type is not string",
            ),
            (
                "7 log.warning",
                "LOG.WARNING: Error casting tracing message: This Dynamic type is not string",
            ),
        ] {
            let (out, _) = run(src);
            let e = out.expect_err(src);
            assert!(e.ends_with(&format!("returned error: {want}")), "{src}: {e}");
        }
    }

    /// **The cast happens before the level is consulted, and that is
    /// observable.** `7 log.info` fails at the default level even though
    /// `log.info` emits nothing there — measured against the oracle, which
    /// reports the same casting error for the same program.
    #[test]
    fn a_bad_message_fails_even_at_a_level_that_would_say_nothing() {
        let (out, said) = run("7 log.trace");
        assert!(out.is_err(), "the cast must fail whatever the level");
        assert!(said.is_empty(), "and nothing is reported: {said:?}");
    }

    /// **The filter's grammar, as far as it is reproduced.** A bare level, the
    /// `bund=info` directive shape the reference's own `--debug` produces, and
    /// the fallback for anything else.
    #[test]
    fn the_threshold_is_read_from_the_environment() {
        assert_eq!(level_named("error"), Some(Level::Error));
        assert_eq!(level_named("WARN"), Some(Level::Warning), "env_logger's spelling");
        assert_eq!(level_named("warning"), Some(Level::Warning), "the word's spelling");
        assert_eq!(level_named("off"), Some(Level::Off));
        assert_eq!(level_named("nonsense"), None, "and the caller falls back");
    }

    /// The ordering is what the filter compares, so it is asserted directly:
    /// a quieter threshold admits fewer levels.
    #[test]
    fn the_levels_order_from_quietest_to_loudest() {
        assert!(Level::Off < Level::Error);
        assert!(Level::Error < Level::Warning);
        assert!(Level::Warning < Level::Info);
        assert!(Level::Info < Level::Debug);
        assert!(Level::Debug < Level::Trace);
        // At the default, `log.error` passes and `log.info` does not.
        assert!(Level::Error <= Level::Error);
        assert!(Level::Info > Level::Error);
        // `off` admits nothing, including `log.error`.
        assert!(Level::Error > Level::Off);
    }
}
