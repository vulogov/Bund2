//! Input — where a word's line comes from.
//!
//! [`Vm::report`](crate::Vm::report) is the seam a word speaks through
//! without deciding how it looks (D36). This is the same seam for listening:
//! a word asks the VM for a line and the embedder decides where lines come
//! from. D112, amending RFC-0002.
//!
//! **Shaped by two consumers that both exist**, which is the condition D101
//! set. The CLI answers from the terminal. The debugger must *not* answer a
//! program's read from the stream its own commands arrive on — F165: a
//! program that read input took the session's commands as its lines. A third
//! falls out for nothing: a test can supply lines, where before it could only
//! reach a word's end-of-input arm.
//!
//! # What it is, and what it is not
//!
//! Two questions: a line, and a line that is not echoed. A line read may name
//! a **history**, which is all a session needs said about itself — the
//! implementor decides whether a history exists, where it is kept and when it
//! is written.
//!
//! No completion, no cancellation, no asynchronous read, no structured
//! prompt. A TUI may need those; D101 declined to guess them and so does
//! this. Adding to a trait with three implementors is cheaper than removing
//! from one.

use std::collections::VecDeque;

/// What a read came back with, when it did not fail.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Read {
    /// A line, as typed, without its line ending.
    Line(String),
    /// There is no more input, or the reader was interrupted. The reference's
    /// words treat Ctrl-D and Ctrl-C alike wherever they read
    /// (`reference/Bund/src/stdlib/functions/io/input.rs:46-51,117-124`), so
    /// one answer serves both and no word has to decide between them.
    End,
}

/// One request for a line.
#[derive(Clone, Copy, Debug)]
pub struct Ask<'a> {
    /// Shown before the line, exactly as given — colour codes and all.
    pub prompt: &'a str,
    /// The history this read belongs to, if it belongs to one: a name, which
    /// the implementor may use as a file's. `None` is a read with no memory,
    /// which is what `input` is.
    pub history: Option<&'a str>,
}

impl<'a> Ask<'a> {
    /// A read with no history.
    pub fn new(prompt: &'a str) -> Self {
        Ask { prompt, history: None }
    }

    /// A read that belongs to the named history.
    pub fn in_history(prompt: &'a str, history: &'a str) -> Self {
        Ask { prompt, history: Some(history) }
    }
}

/// Where lines come from. Installed on an interpreter by its embedder.
///
/// A failure is a `String` because it is the embedder's own — a terminal that
/// would not open, a file that would not read — and the word that asked wraps
/// it in its own prefix, as it wraps every other failure.
pub trait Input {
    /// One line.
    fn line(&mut self, ask: &Ask<'_>) -> Result<Read, String>;

    /// Add `line` to the named history. Separate from [`Input::line`] because
    /// which lines are worth remembering is the word's business: `debug`
    /// does not remember the empty line that moves it on, and `debug.shell`
    /// remembers every one.
    fn remember(&mut self, _history: &str, _line: &str) {}

    /// One line that is not echoed.
    fn secret(&mut self, prompt: &str) -> Result<String, String>;
}

/// No input at all: every line is the end, and a secret is refused.
///
/// **The default**, so an interpreter nobody has given an input never waits
/// on one. That is F158's hazard closed by construction rather than by a
/// build flag: a suite that builds an interpreter and runs a word that reads
/// gets the end-of-input arm at once, whatever its standard input is.
///
/// A secret is refused rather than answered empty, because an empty secret is
/// one a caller might take for a real one.
#[derive(Clone, Copy, Default, Debug)]
pub struct NoInput;

impl Input for NoInput {
    fn line(&mut self, _: &Ask<'_>) -> Result<Read, String> {
        Ok(Read::End)
    }

    fn secret(&mut self, _: &str) -> Result<String, String> {
        Err("there is no terminal to read a secret from".to_string())
    }
}

/// Lines given in advance, then the end. For a test, and for any embedder
/// that has its input in hand before the program runs.
///
/// It keeps what it was asked and what it was told to remember, so a test can
/// read both back through [`Scripted::seen`] after handing the input over.
#[derive(Default, Debug)]
pub struct Scripted {
    lines: VecDeque<String>,
    secrets: VecDeque<String>,
    seen: std::rc::Rc<std::cell::RefCell<Seen>>,
}

/// What a [`Scripted`] input was asked for.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    /// Every prompt a line or a secret was asked with, in order.
    pub prompts: Vec<String>,
    /// Every `(history, line)` it was told to remember, in order.
    pub remembered: Vec<(String, String)>,
}

impl Scripted {
    /// Lines to answer with, in order.
    pub fn lines<I, S>(lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Scripted {
            lines: lines.into_iter().map(Into::into).collect(),
            ..Scripted::default()
        }
    }

    /// Secrets to answer with, in order. When they run out a secret is
    /// refused, as [`NoInput`] refuses it.
    pub fn with_secrets<I, S>(mut self, secrets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.secrets = secrets.into_iter().map(Into::into).collect();
        self
    }

    /// A handle on what this input is asked, which stays readable after the
    /// input itself has been moved into an interpreter.
    pub fn seen(&self) -> std::rc::Rc<std::cell::RefCell<Seen>> {
        self.seen.clone()
    }
}

impl Input for Scripted {
    fn line(&mut self, ask: &Ask<'_>) -> Result<Read, String> {
        self.seen.borrow_mut().prompts.push(ask.prompt.to_string());
        Ok(match self.lines.pop_front() {
            Some(line) => Read::Line(line),
            None => Read::End,
        })
    }

    fn remember(&mut self, history: &str, line: &str) {
        self.seen
            .borrow_mut()
            .remembered
            .push((history.to_string(), line.to_string()));
    }

    fn secret(&mut self, prompt: &str) -> Result<String, String> {
        self.seen.borrow_mut().prompts.push(prompt.to_string());
        self.secrets
            .pop_front()
            .ok_or_else(|| "there is no terminal to read a secret from".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_input_ends_at_once_and_refuses_a_secret() {
        let mut i = NoInput;
        assert_eq!(i.line(&Ask::new("> ")), Ok(Read::End));
        assert!(i.secret("pw: ").is_err());
    }

    #[test]
    fn a_script_answers_in_order_and_then_ends() {
        let mut s = Scripted::lines(["a", "b"]).with_secrets(["x"]);
        let seen = s.seen();
        assert_eq!(s.line(&Ask::new("1> ")), Ok(Read::Line("a".into())));
        assert_eq!(
            s.line(&Ask::in_history("2> ", "h")),
            Ok(Read::Line("b".into()))
        );
        s.remember("h", "b");
        assert_eq!(s.line(&Ask::new("3> ")), Ok(Read::End));
        assert_eq!(s.secret("pw: "), Ok("x".to_string()));
        assert!(s.secret("pw: ").is_err());
        let seen = seen.borrow();
        assert_eq!(seen.prompts, ["1> ", "2> ", "3> ", "pw: ", "pw: "]);
        assert_eq!(seen.remembered, [("h".to_string(), "b".to_string())]);
    }
}
