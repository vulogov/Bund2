//! Writing to the terminal without aborting when it has gone away.
//!
//! `println!` panics when the write fails, and a write to a pipe whose reader
//! has left fails: `bund2 words | head -1` ended in a Rust backtrace. Bund2
//! does not panic (D37), so nothing in the workspace uses the `print` family.
//! These are what it uses instead.
//!
//! **Two shapes, because there are two callers.** A word's output is the
//! program's, and a program whose output is gone should stop and be told
//! why: [`out!`](crate::out!) and [`outln!`](crate::outln!) answer a `Result`
//! a word returns with `?`. A front end's own messages, and anything on
//! standard error, have nowhere left to complain to: [`say!`](crate::say!),
//! [`sayln!`](crate::sayln!), [`err!`](crate::err!) and
//! [`errln!`](crate::errln!) drop the failure.
//!
//! Buffering is unchanged. `print!` writes through the same line-buffered
//! handle these do.

use std::fmt::Arguments;
use std::io::Write;

use crate::Error;

/// Write `text` to standard output, with a newline when `newline` is set.
///
/// The error names the stream rather than a word: the write is the same
/// whichever word asked for it, and the report that carries this names the
/// word.
pub fn stdout(text: Arguments<'_>, newline: bool) -> Result<(), Error> {
    let mut out = std::io::stdout().lock();
    let wrote = if newline {
        out.write_fmt(text).and_then(|()| out.write_all(b"\n"))
    } else {
        out.write_fmt(text)
    };
    wrote.map_err(|e| Error(format!("writing to standard output: {e}")))
}

/// Write `text` to standard error. A failure is dropped: there is no stream
/// left to report it on.
pub fn stderr(text: Arguments<'_>, newline: bool) {
    let mut err = std::io::stderr().lock();
    let _ = err.write_fmt(text);
    if newline {
        let _ = err.write_all(b"\n");
    }
}

/// `print!`, answering a `Result` instead of panicking.
#[macro_export]
macro_rules! out {
    ($($t:tt)*) => { $crate::out::stdout(::core::format_args!($($t)*), false) };
}

/// `println!`, answering a `Result` instead of panicking.
#[macro_export]
macro_rules! outln {
    () => { $crate::out::stdout(::core::format_args!(""), true) };
    ($($t:tt)*) => { $crate::out::stdout(::core::format_args!($($t)*), true) };
}

/// `print!` for a front end's own messages: a failed write is dropped.
#[macro_export]
macro_rules! say {
    ($($t:tt)*) => {{ let _ = $crate::out::stdout(::core::format_args!($($t)*), false); }};
}

/// `println!` for a front end's own messages: a failed write is dropped.
#[macro_export]
macro_rules! sayln {
    () => {{ let _ = $crate::out::stdout(::core::format_args!(""), true); }};
    ($($t:tt)*) => {{ let _ = $crate::out::stdout(::core::format_args!($($t)*), true); }};
}

/// `eprint!` that does not panic.
#[macro_export]
macro_rules! err {
    ($($t:tt)*) => { $crate::out::stderr(::core::format_args!($($t)*), false) };
}

/// `eprintln!` that does not panic.
#[macro_export]
macro_rules! errln {
    () => { $crate::out::stderr(::core::format_args!(""), true) };
    ($($t:tt)*) => { $crate::out::stderr(::core::format_args!($($t)*), true) };
}
