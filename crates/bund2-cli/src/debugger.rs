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

use bund2_api::Vm as _;
use bund2_interp::debug::{Command, Console};
use bund2_value::BundValue;

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
    let line = line.trim();
    // **The forms that take an argument, before the bare words.** `break w if
    // <lambda>` is split on the first ` if ` so a condition may contain
    // anything, including the word `if`.
    if let Some(rest) = line.strip_prefix("break ").or_else(|| line.strip_prefix("b ")) {
        let rest = rest.trim();
        if let Some((word, cond)) = rest.split_once(" if ") {
            let (word, cond) = (word.trim(), cond.trim());
            if word.is_empty() || cond.is_empty() {
                return Err("bund2: `break <word> if <lambda>` needs both".to_string());
            }
            return Ok(Command::BreakIf(word.to_string(), cond.to_string()));
        }
        if rest.is_empty() {
            return Err("bund2: `break` needs a word".to_string());
        }
        return Ok(Command::Break(rest.to_string()));
    }
    if let Some(rest) = line.strip_prefix("delete ").or_else(|| line.strip_prefix("d ")) {
        let rest = rest.trim().trim_start_matches('@');
        if rest.is_empty() {
            return Err("bund2: `delete` needs a word or @stack".to_string());
        }
        return Ok(Command::Delete(rest.to_string()));
    }
    if let Some(rest) = line.strip_prefix("watch ").or_else(|| line.strip_prefix("w ")) {
        let rest = rest.trim();
        // **`workbench` is a different hook, not a stack called
        // "workbench"** — §D4's correction to the first draft. Spelled without
        // an `@` for that reason, and `@workbench` is refused rather than
        // silently taken as either.
        if rest == "workbench" {
            return Ok(Command::WatchWorkbench);
        }
        if rest == "@workbench" {
            return Err("bund2: the workbench is not a named stack — say `watch workbench`"
                .to_string());
        }
        let name = rest.trim_start_matches('@');
        if name.is_empty() {
            return Err("bund2: `watch @stack` or `watch workbench`".to_string());
        }
        return Ok(Command::Watch(name.to_string()));
    }
    match line {
        "s" | "step" => Ok(Command::Step),
        "n" | "next" => Ok(Command::Next),
        "f" | "finish" => Ok(Command::Finish),
        "c" | "cont" | "continue" => Ok(Command::Continue),
        "bt" | "backtrace" | "where" => Ok(Command::Backtrace),
        "st" | "stack" => Ok(Command::Stack),
        "i" | "info" => Ok(Command::Info),
        "" => Err(String::new()),
        other => Err(format!(
            "bund2: `{other}` is not a command. One of: s(tep), n(ext), \
             f(inish), c(ontinue), bt, stack, info, break <word> [if <lambda>], \
             watch @stack, watch workbench, delete <word>."
        )),
    }
}

/// **A breakpoint condition's child VM — §D3, criterion 8.**
///
/// A fresh `Runtime` per evaluation, built here because `bund2-interp` cannot:
/// a usable child needs the standard vocabulary, and `bund2-stdlib` depends on
/// the interpreter rather than the reverse.
///
/// **Fresh rather than reused, and the exit cell is why.** `request_exit` is
/// `get_or_insert`, so a child that once called `bund.exit` could not be
/// cleared and every later evaluation would inherit the exit. Rebuilding costs
/// a registration — tens of microseconds — and it is paid only when a watched
/// word is actually reached, which is a debugger's own definition of rare.
///
/// **The condition sees the program's stack and cannot change it.** The
/// operands are a snapshot the debuggee took; pushing them into the child hands
/// over the same `Rc` payloads, which is safe because the child lives and dies
/// on this thread, and anything the condition does — a push, a stack switch, a
/// rebind, an exit — happens to the child.
///
/// **What counts as true.** The value the condition leaves on top, read as
/// Bund reads a truth: a `BOOL` is itself. Anything else, including an empty
/// stack, is not a stop — §D3 says a condition that "leaves no value" does not
/// stop, and guessing a truthiness for a LIST would be inventing a rule the
/// reference does not have.
fn evaluate_in_child(source: &str, operands: &[BundValue]) -> Result<bool, String> {
    let mut child = bund2_runtime::Runtime::new();
    for v in operands {
        child.interp.push(v.clone());
    }
    child.eval_str(source).map_err(|e| e.0)?;
    if let Some(code) = child.interp.exit_requested() {
        return Err(format!(
            "the condition called bund.exit({code}); the debugged program is untouched"
        ));
    }
    // **A lambda literal pushes the lambda; running it is what a condition
    // means.** §D3 says the condition *is* a lambda, so `{ 1 1 == }` has to be
    // executed and not merely constructed — otherwise every lambda condition
    // would answer "left lambda/3 rather than a BOOL". A bare expression like
    // `1 1 ==` leaves its answer directly and needs no execution, so both
    // spellings work.
    if let Some(top) = child.interp.peek()
        && top.dt() == bund2_value::LAMBDA
    {
        let lambda = child.interp.pull().unwrap_or(top);
        child
            .interp
            .eval_lambda(&lambda)
            .map_err(|e| format!("the condition's lambda failed: {}", e.0))?;
        if let Some(code) = child.interp.exit_requested() {
            return Err(format!(
                "the condition called bund.exit({code}); the debugged program is untouched"
            ));
        }
    }
    let top = child.interp.peek();
    match top.as_ref().map(bund2_value::BundValue::unboxed) {
        Some(bund2_value::BundValue::Bool(b, _)) => Ok(*b),
        Some(other) => Err(format!(
            "the condition left {} rather than a BOOL",
            other.summary(32)
        )),
        None => Err("the condition left no value".to_string()),
    }
}

impl Console for Stdio {
    fn stopped(&mut self, at: &str) {
        let mut out = std::io::stderr();
        if !self.greeted {
            self.greeted = true;
            let _ = writeln!(
                out,
                "bund2: stopped. s(tep), n(ext), f(inish), c(ontinue), bt, stack, \
                 info, break <word> [if <lambda>], watch @stack, watch workbench, \
                 delete <word>. EOF detaches and runs on."
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

    fn evaluate_condition(
        &mut self,
        source: &str,
        operands: &[BundValue],
    ) -> Result<bool, String> {
        evaluate_in_child(source, operands)
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
