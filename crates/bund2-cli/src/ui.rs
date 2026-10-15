//! The command line's own voice — D173: how loud, and in what colour.
//!
//! **Five levels, one threshold.** `bund2 -v`, `-vv` and `-vvv` raise it to
//! info, debug and trace; `--verbose=<level>` names one; `-q` lowers it to
//! errors alone. The threshold governs two things at once: what the command
//! line says about its own work (reading a file, locating a bundle's region,
//! signing an artefact), and what a program's `log.*` words are allowed to
//! say — the reference's `--debug` count raised `BUND_LOG_LEVEL` the same way
//! (`crates/bund2-stdlib/src/logging.rs`). A bundle takes no flags (RFC-0006
//! §B3), so it reads `BUND2_VERBOSE`.
//!
//! **Errors keep their text.** An error line is `bund2: <message>`, as it
//! always was and as the tests pin; the prefix is coloured where standard
//! error is a terminal and plain where it is not, so a captured stream reads
//! as before. The other levels carry their name: `bund2: info: …`.
//!
//! **Colour is `console`'s decision, narrowed by three switches.** It is off
//! on a pipe; `--nocolor` (the reference's own flag), `NO_COLOR` and
//! `BUND2_NOCOLOR` turn it off on a terminal too. Nothing here writes through
//! the `print` family (D37): the lines go through `bund2_api::errln!`, which
//! drops a failed write, and a progress bar goes through `indicatif`'s own
//! terminal handle, which does the same.

use std::fmt::Arguments;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

/// How much the command line says. Ordered: a level shows itself and every
/// level below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level {
    Error = 0,
    Warning = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

static LEVEL: AtomicU8 = AtomicU8::new(Level::Warning as u8);

impl Level {
    /// A level by name, or by the count `-v` would have given it.
    pub fn parse(text: &str) -> Option<Level> {
        match text.trim().to_ascii_lowercase().as_str() {
            "error" | "0" | "quiet" => Some(Level::Error),
            "warning" | "warn" | "1" => Some(Level::Warning),
            "info" | "2" => Some(Level::Info),
            "debug" | "3" => Some(Level::Debug),
            "trace" | "4" => Some(Level::Trace),
            _ => None,
        }
    }

    /// The level `n` repetitions of `-v` ask for: none is the default, one
    /// is info, two debug, three or more trace.
    fn from_count(n: u8) -> Level {
        match n {
            0 => Level::Warning,
            1 => Level::Info,
            2 => Level::Debug,
            _ => Level::Trace,
        }
    }

    fn from_u8(n: u8) -> Level {
        match n {
            0 => Level::Error,
            1 => Level::Warning,
            2 => Level::Info,
            3 => Level::Debug,
            _ => Level::Trace,
        }
    }

    /// The name, as `BUND_LOG_LEVEL` spells it.
    pub fn name(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Info => "info",
            Level::Debug => "debug",
            Level::Trace => "trace",
        }
    }

    fn label(self) -> console::StyledObject<&'static str> {
        let s = console::style(self.name()).for_stderr();
        match self {
            Level::Error => s.red().bold(),
            Level::Warning => s.yellow().bold(),
            Level::Info => s.green(),
            Level::Debug => s.blue(),
            Level::Trace => s.dim(),
        }
    }
}

/// The threshold now.
pub fn level() -> Level {
    Level::from_u8(LEVEL.load(Ordering::Relaxed))
}

pub fn set_level(level: Level) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

/// Take the verbosity flags out of `args`, wherever they stand before `--`,
/// and answer the level they ask for — `None` when none was given, so the
/// caller can fall back to the environment.
///
/// `-v`, `-vv`, `-vvv` and `--verbose` count up; `--verbose=<level>` names
/// a level outright; `-q` and `--quiet` ask for errors alone and win over
/// any count. Everything after `--` is the program's and is left alone.
pub fn take_verbosity(args: Vec<String>) -> Result<(Vec<String>, Option<Level>), String> {
    let mut kept = Vec::with_capacity(args.len());
    let mut count: u8 = 0;
    let mut named: Option<Level> = None;
    let mut quiet = false;
    let mut seen = false;
    let mut rest = args.into_iter();
    for a in rest.by_ref() {
        if a == "--" {
            kept.push(a);
            break;
        }
        let bare = a.as_str();
        if bare.len() > 1 && bare.starts_with('-') && bare[1..].bytes().all(|b| b == b'v') {
            count = count.saturating_add((bare.len() - 1).min(3) as u8);
            seen = true;
            continue;
        }
        match bare {
            "--verbose" => {
                count = count.saturating_add(1);
                seen = true;
            }
            "-q" | "--quiet" => {
                quiet = true;
                seen = true;
            }
            _ if bare.starts_with("--verbose=") => {
                let want = &bare["--verbose=".len()..];
                named = Some(Level::parse(want).ok_or_else(|| {
                    format!(
                        "`--verbose={want}` names no level; use error, warning, info, debug or trace"
                    )
                })?);
                seen = true;
            }
            _ => kept.push(a),
        }
    }
    kept.extend(rest);
    let level = if !seen {
        None
    } else if quiet {
        Some(Level::Error)
    } else {
        Some(named.unwrap_or_else(|| Level::from_count(count)))
    };
    Ok((kept, level))
}

/// `BUND2_VERBOSE`, for a bundle, whose command line is its program's.
pub fn level_from_env() -> Option<Level> {
    std::env::var("BUND2_VERBOSE")
        .ok()
        .and_then(|v| Level::parse(&v))
}

/// Switch colour off where it is not wanted: `--nocolor`, `NO_COLOR`, or
/// `BUND2_NOCOLOR`. `console` has already switched it off on a pipe.
pub fn configure_colors(nocolor: bool) {
    let env_off = |name: &str| std::env::var_os(name).is_some_and(|v| !v.is_empty() && v != "0");
    if nocolor || env_off("NO_COLOR") || env_off("BUND2_NOCOLOR") {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
    }
}

/// One line on standard error, if `level` is within the threshold.
pub fn log(level: Level, msg: Arguments<'_>) {
    if level > self::level() {
        return;
    }
    let prefix = console::style("bund2:").for_stderr();
    match level {
        // The text the tests and the scripts around this binary know.
        Level::Error => bund2_api::errln!("{} {msg}", prefix.red()),
        other => bund2_api::errln!("{} {}: {msg}", prefix.dim(), other.label()),
    }
}

macro_rules! error {
    ($($t:tt)*) => { $crate::ui::log($crate::ui::Level::Error, ::core::format_args!($($t)*)) };
}
macro_rules! warning {
    ($($t:tt)*) => { $crate::ui::log($crate::ui::Level::Warning, ::core::format_args!($($t)*)) };
}
macro_rules! info {
    ($($t:tt)*) => { $crate::ui::log($crate::ui::Level::Info, ::core::format_args!($($t)*)) };
}
macro_rules! debug {
    ($($t:tt)*) => { $crate::ui::log($crate::ui::Level::Debug, ::core::format_args!($($t)*)) };
}
macro_rules! trace {
    ($($t:tt)*) => { $crate::ui::log($crate::ui::Level::Trace, ::core::format_args!($($t)*)) };
}
pub(crate) use {debug, error, info, trace, warning};

/// A progress bar over `steps` steps, on standard error.
///
/// **Hidden where it would be noise**: on a pipe, where nothing animates and
/// every redraw is a line in a log, and under `-q`. A hidden bar accepts
/// every call and draws nothing, so the caller does not branch.
pub fn progress(steps: u64, what: &str) -> ProgressBar {
    let bar = ProgressBar::new(steps);
    if !console::user_attended_stderr() || level() == Level::Error {
        bar.set_draw_target(ProgressDrawTarget::hidden());
        return bar;
    }
    let style = ProgressStyle::with_template(
        "{spinner:.cyan} {prefix:.bold} {msg:<32} [{bar:24.cyan/dim}] {pos}/{len}",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("=> ");
    bar.set_style(style);
    bar.set_prefix(what.to_string());
    bar.enable_steady_tick(Duration::from_millis(80));
    bar
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(args: &[&str]) -> (Vec<String>, Option<Level>) {
        take_verbosity(args.iter().map(|a| a.to_string()).collect()).expect("parses")
    }

    #[test]
    fn counts_names_and_quiet_map_to_levels() {
        assert_eq!(take(&["script", "--file", "p"]).1, None);
        assert_eq!(take(&["-v", "script"]).1, Some(Level::Info));
        assert_eq!(take(&["script", "-vv"]).1, Some(Level::Debug));
        assert_eq!(take(&["-vvv"]).1, Some(Level::Trace));
        assert_eq!(take(&["-v", "-v", "-v", "-v"]).1, Some(Level::Trace));
        assert_eq!(take(&["--verbose"]).1, Some(Level::Info));
        assert_eq!(take(&["--verbose=debug"]).1, Some(Level::Debug));
        assert_eq!(take(&["--verbose=2"]).1, Some(Level::Info));
        assert_eq!(take(&["-vvv", "-q"]).1, Some(Level::Error));
        assert_eq!(take(&["--quiet"]).1, Some(Level::Error));
    }

    #[test]
    fn the_flags_are_removed_and_the_program_s_arguments_are_not_read() {
        let (kept, level) = take(&["script", "-v", "--file", "p", "--", "-vv", "-q"]);
        assert_eq!(kept, ["script", "--file", "p", "--", "-vv", "-q"]);
        assert_eq!(level, Some(Level::Info));
    }

    #[test]
    fn a_level_that_does_not_exist_is_refused_by_name() {
        let e = take_verbosity(vec!["--verbose=loud".to_string()]).expect_err("refused");
        assert!(e.contains("loud"), "{e}");
    }

    #[test]
    fn a_hidden_bar_accepts_every_call() {
        let bar = ProgressBar::new(3);
        bar.set_draw_target(ProgressDrawTarget::hidden());
        bar.set_message("x");
        bar.inc(1);
        bar.finish_and_clear();
    }
}
