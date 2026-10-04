//! **`--debugger`: the host side of RFC-0008 §D1's safepoint.**
//!
//! `bund2-interp` defines the seam and names no transport, for the reason
//! `Reporter` names none. This is the transport: lines on stdin, text on
//! stdout. A TUI would implement the same trait over its own event loop, and
//! nothing in the interpreter changes.
//!
//! **Lines rather than readline.** `rustyline` is already a dependency, for
//! `input` and `password`, and it is deliberately not used here: a line reader
//! with history and editing wants a terminal, and this has to work under a
//! pipe — which is how the criterion 6 differential drives it, feeding `step`
//! until the program ends. History is §D8's, and §D8 is not built.
//!
//! **EOF detaches rather than killing the program.** A debugger that dies must
//! not take the debuggee with it, so a closed stdin lets the program run to
//! completion — which is also what makes `echo c | bund2 --debugger …` a
//! sensible thing to type.
//!
//! **The session writes to stderr, and the program keeps stdout.** Prompts,
//! position lines and `bt` output are not the program's output, and mixing them
//! would make two things impossible: comparing a stepped run against a plain
//! one, which is criterion 6's whole method, and piping a debugged program's
//! output anywhere useful. It is the same reasoning D36 gives for a warning —
//! "a warning that interrupts the program's own output is worse than one that
//! is easy to miss" — and the reference's own `debug` REPL, which prints to
//! stdout, is not a constraint here because no golden can hold it either way
//! (criterion 13).

use std::io::{BufRead, Write};

use bund2_interp::debug::{Command, Console};

/// A debugger driven by lines on stdin.
pub struct Stdio {
    input: std::io::Lines<std::io::StdinLock<'static>>,
    /// Suppresses the banner after the first stop.
    greeted: bool,
}

impl Stdio {
    pub fn new() -> Self {
        Self {
            input: std::io::stdin().lock().lines(),
            greeted: false,
        }
    }
}

/// The command vocabulary, long and short forms.
///
/// **Unknown input is a message, not a command**, and does not advance the
/// program: a typo at a breakpoint that silently stepped would be the worst
/// possible behaviour in a debugger.
fn parse(line: &str) -> Result<Command, String> {
    match line.trim() {
        "s" | "step" => Ok(Command::Step),
        "n" | "next" => Ok(Command::Next),
        "f" | "finish" => Ok(Command::Finish),
        "c" | "cont" | "continue" => Ok(Command::Continue),
        "bt" | "backtrace" | "where" => Ok(Command::Backtrace),
        "st" | "stack" => Ok(Command::Stack),
        "" => Err(String::new()),
        other => Err(format!(
            "bund2: `{other}` is not a command. One of: s(tep), n(ext), \
             f(inish), c(ontinue), bt, stack."
        )),
    }
}

impl Console for Stdio {
    fn stopped(&mut self, at: &str) {
        let mut out = std::io::stderr();
        if !self.greeted {
            self.greeted = true;
            let _ = writeln!(
                out,
                "bund2: stopped. s(tep), n(ext), f(inish), c(ontinue), bt, stack. \
                 EOF detaches and runs on."
            );
        }
        let _ = writeln!(out, "{at}");
        let _ = out.flush();
    }

    fn next_command(&mut self) -> Option<Command> {
        loop {
            let mut out = std::io::stderr();
            let _ = write!(out, "(bund2) ");
            let _ = out.flush();
            // `None` is EOF, which detaches. A read error is treated the same
            // way: there is no further input to be had, and hanging is worse.
            let line = self.input.next()?.ok()?;
            match parse(&line) {
                Ok(c) => return Some(c),
                // An empty line repeats nothing and asks again, which is what
                // pressing return at a prompt should do.
                Err(msg) if msg.is_empty() => {}
                Err(msg) => {
                    let _ = writeln!(out, "{msg}");
                    let _ = out.flush();
                }
            }
        }
    }

    fn answer(&mut self, text: &str) {
        let mut out = std::io::stderr();
        let _ = write!(out, "{text}");
        let _ = out.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every spelling, and the two things that are not commands.
    #[test]
    fn the_command_vocabulary_is_what_the_banner_advertises() {
        for (text, want) in [
            ("s", Command::Step),
            ("step", Command::Step),
            ("  step  ", Command::Step),
            ("n", Command::Next),
            ("next", Command::Next),
            ("f", Command::Finish),
            ("finish", Command::Finish),
            ("c", Command::Continue),
            ("cont", Command::Continue),
            ("continue", Command::Continue),
            ("bt", Command::Backtrace),
            ("where", Command::Backtrace),
            ("stack", Command::Stack),
        ] {
            assert_eq!(parse(text), Ok(want), "{text}");
        }
        assert_eq!(parse(""), Err(String::new()), "a bare return asks again");
        let e = parse("stpe").expect_err("a typo is not a command");
        assert!(e.contains("not a command"), "{e}");
        assert!(e.contains("s(tep)"), "and it says what is: {e}");
    }

    /// **A typo must not advance the program.** The parser returning `Err`
    /// rather than a default is what guarantees it, and this asserts that no
    /// unrecognised spelling maps to a `Command` at all.
    #[test]
    fn nothing_unrecognised_becomes_a_step() {
        for text in ["", "x", "ste", "stepp", "quit", "run", "0", "s s"] {
            assert!(parse(text).is_err(), "`{text}` became a command");
        }
    }
}
