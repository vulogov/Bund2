//! **`--debugger`: the host side of RFC-0008 §D1's safepoint.**
//!
//! `bund2-interp` defines the seam and names no transport, for the reason
//! `Reporter` names none. This is the transport: lines on stdin, text on
//! stdout. A TUI would implement the same trait over its own event loop, and
//! nothing in the interpreter changes.
//!
//! **Lines under a pipe, an editor at a terminal.** Under a pipe — how the
//! criterion 6 differential drives it, feeding `step` until the program ends
//! — commands are plain lines. At a terminal they are read by the line
//! editor `input` uses, with §D8's history in the same file the console a
//! word attaches keeps (D191; RFC-0008 had left this console without one).
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
    input: Source,
    /// Suppresses the banner after the first stop.
    greeted: bool,
}

/// Where the commands come from.
enum Source {
    /// A pipe or a file: one command a line.
    Lines(std::io::Lines<std::io::StdinLock<'static>>),
    /// A terminal: the line editor, with a history (D191).
    Terminal(Box<bund2_stdlib::terminal::Terminal>),
}

impl Stdio {
    pub fn new() -> Self {
        use std::io::IsTerminal;
        let input = if std::io::stdin().is_terminal() {
            Source::Terminal(Box::new(bund2_stdlib::terminal::Terminal::new()))
        } else {
            Source::Lines(std::io::stdin().lock().lines())
        };
        Self {
            input,
            greeted: false,
        }
    }

    /// One line, or `None` at the end of input or a failed read.
    fn read(&mut self) -> Option<String> {
        use bund2_api::input::{Ask, Input, Read};
        match &mut self.input {
            Source::Lines(lines) => {
                let mut out = std::io::stderr();
                let _ = write!(out, "(bund2) ");
                let _ = out.flush();
                lines.next().and_then(Result::ok)
            }
            Source::Terminal(t) => match t.line(&Ask::in_history("(bund2) ", HISTORY)) {
                Ok(Read::Line(line)) => Some(line),
                _ => None,
            },
        }
    }

    /// Keep a command in the history, at a terminal.
    fn remember(&mut self, line: &str) {
        use bund2_api::input::Input;
        if let Source::Terminal(t) = &mut self.input {
            t.remember(HISTORY, line);
        }
    }
}

/// What `?` answers. The short forms are the session's; every other line is
/// Bund.
const HELP: &str = "bund2: s(tep), n(ext), f(inish), c(ontinue), bt, st, info, \
                    break <word> [<n>] [if <lambda>], break [<file>:]<line>, until <word>, catch, \
                    watch @stack [if <lambda>], watch workbench, watch redefine, \
                    watch depth > <n>, delete <word|catch|redefine|depth>, \
                    words [all|<text>], classes. Any other line is run as Bund in the program.";

/// The command vocabulary, long and short forms.
///
/// **A line that is none of these is Bund** (§W2, D113), and does not advance
/// the program: a typo at a breakpoint that silently stepped would be the
/// worst possible behaviour in a debugger, and an unknown word is a reported
/// failure that leaves the session where it was.
fn parse(line: &str) -> Result<Command, String> {
    let line = line.trim();
    // **The forms that take an argument, before the bare words.** `break w if
    // <lambda>` is split on the first ` if ` so a condition may contain
    // anything, including the word `if`.
    if let Some(rest) = line
        .strip_prefix("break ")
        .or_else(|| line.strip_prefix("b "))
    {
        let rest = rest.trim();
        let (head, cond) = match rest.split_once(" if ") {
            Some((h, c)) => {
                let c = c.trim();
                if h.trim().is_empty() || c.is_empty() {
                    return Err("bund2: `break <word> if <lambda>` needs both".to_string());
                }
                (h.trim(), Some(c.to_string()))
            }
            None => (rest, None),
        };
        if head.is_empty() {
            return Err("bund2: `break` needs a word".to_string());
        }
        // RFC-0008 §D3 by source location: `break 12`, `break p.bund:12`.
        if let Some((file, line)) = bund2_interp::debug::line_spec(head) {
            if cond.is_some() {
                return Err("bund2: a line breakpoint takes no condition".to_string());
            }
            return Ok(Command::BreakLine(file, line));
        }
        // **D191: `break w 100` stops at the hundredth hit, once.** A word
        // has no spaces, so a second token is the hit.
        if let Some((word, n)) = head.split_once(char::is_whitespace) {
            let n: u64 = match n.trim().parse() {
                Ok(n) if n >= 1 => n,
                _ => {
                    return Err(
                        "bund2: `break <word> <n>` stops at the n-th hit, counting from 1"
                            .to_string(),
                    );
                }
            };
            return Ok(Command::BreakAt(word.to_string(), n, cond));
        }
        return Ok(match cond {
            Some(c) => Command::BreakIf(head.to_string(), c),
            None => Command::Break(head.to_string()),
        });
    }
    if let Some(rest) = line.strip_prefix("until ") {
        let word = rest.trim();
        if word.is_empty() || word.contains(char::is_whitespace) {
            return Err("bund2: `until <word>`".to_string());
        }
        return Ok(Command::Until(word.to_string()));
    }
    if let Some(rest) = line
        .strip_prefix("delete ")
        .or_else(|| line.strip_prefix("d "))
    {
        let rest = rest.trim().trim_start_matches('@');
        if rest.is_empty() {
            return Err("bund2: `delete` needs a word or @stack".to_string());
        }
        return Ok(Command::Delete(rest.to_string()));
    }
    if let Some(rest) = line
        .strip_prefix("watch ")
        .or_else(|| line.strip_prefix("w "))
    {
        let rest = rest.trim();
        // **`workbench` is a different hook, not a stack called
        // "workbench"** — §D4's correction to the first draft. Spelled without
        // an `@` for that reason, and `@workbench` is refused rather than
        // silently taken as either.
        if rest == "workbench" {
            return Ok(Command::WatchWorkbench);
        }
        // D191: a bound name registered again. No `@`, as `workbench` has
        // none: it is a hook, not a stack.
        if rest == "redefine" {
            return Ok(Command::WatchRedefine);
        }
        // §D4: `watch depth > 100`, on whichever stack is current. The `>`
        // is optional, since it is the only comparison there is.
        if let Some(n) = rest.strip_prefix("depth") {
            let n = n.trim().trim_start_matches('>').trim();
            return match n.parse::<usize>() {
                Ok(n) => Ok(Command::WatchDepth(n)),
                Err(_) => Err("bund2: `watch depth > <n>`, with n a whole number".to_string()),
            };
        }
        // D191: `watch @s if <lambda>`, split as `break … if` is.
        if let Some((stack, cond)) = rest.split_once(" if ") {
            let (stack, cond) = (stack.trim().trim_start_matches('@'), cond.trim());
            if stack.is_empty() || cond.is_empty() {
                return Err("bund2: `watch @stack if <lambda>` needs both".to_string());
            }
            if stack == "workbench" {
                return Err(
                    "bund2: a condition is for a named stack; `watch workbench` takes none"
                        .to_string(),
                );
            }
            return Ok(Command::WatchIf(stack.to_string(), cond.to_string()));
        }
        if rest == "@workbench" {
            return Err(
                "bund2: the workbench is not a named stack — say `watch workbench`".to_string(),
            );
        }
        let name = rest.trim_start_matches('@');
        if name.is_empty() {
            return Err("bund2: `watch @stack` or `watch workbench`".to_string());
        }
        return Ok(Command::Watch(name.to_string()));
    }
    // §D5's views. Neither is a Bund word, so a line saying one is this.
    if let Some(rest) = line.strip_prefix("words")
        && (rest.is_empty() || rest.starts_with(char::is_whitespace))
    {
        let rest = rest.trim();
        return Ok(Command::Words((!rest.is_empty()).then(|| rest.to_string())));
    }
    match line {
        "classes" => Ok(Command::Classes),
        "s" | "step" => Ok(Command::Step),
        "n" | "next" => Ok(Command::Next),
        "f" | "finish" => Ok(Command::Finish),
        "c" | "cont" | "continue" => Ok(Command::Continue),
        "bt" | "backtrace" | "where" => Ok(Command::Backtrace),
        // **`st`, and not `stack`** (D113.4). `stack` is a Bund word — the
        // reference's alias of `ensure_stack`
        // (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:35`) —
        // and a line here is Bund unless it is one of these short forms.
        "st" => Ok(Command::Stack),
        "i" | "info" => Ok(Command::Info),
        // D191. `catch` is no Bund word, so a line saying it is this.
        "catch" => Ok(Command::Catch),
        "?" | "help" => Err(HELP.to_string()),
        "" => Err(String::new()),
        // **Anything else is Bund, run in the program's own VM** — §W2, and
        // what the reference's `debug` does with every line that is not empty
        // (`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:81-95`).
        // The program does not move: a mistyped command is an unknown word,
        // reported, and the session is where it was.
        other => Ok(Command::Eval(other.to_string())),
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
/// The restrictions the debugged program runs under, for the child VM a
/// condition is evaluated in.
///
/// **A condition is a string of Bund, and it ran with neither restriction —
/// F180.** The child was built with default options, so under
/// `bund2 script --noio --noeval` a `debug.break.if` condition read the
/// working directory and reached `bund.eval`. A child that is freer than its
/// parent is a way round both flags, in the one place D78 says there is none.
///
/// Process-wide because the console is made by a plain function
/// (`Interp::console_factory`), which can carry nothing; one process runs one
/// program, so there is one answer.
static RESTRICTIONS: std::sync::OnceLock<bund2_stdlib::host::HostOptions> =
    std::sync::OnceLock::new();

/// Records the program's host options before any console can exist. The first
/// call wins: a restriction, once recorded, is not replaced.
pub fn restrict(opts: bund2_stdlib::host::HostOptions) {
    let _ = RESTRICTIONS.set(opts);
}

fn evaluate_in_child(source: &str, operands: &[BundValue]) -> Result<bool, String> {
    let opts = RESTRICTIONS.get().copied().unwrap_or_default();
    let mut child = bund2_runtime::Runtime::with_options(&opts);
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

/// **The console a word attaches — RFC-0008 §W5, D113.5.**
///
/// A script that calls `debug.break` or `debug.step` was started with no
/// `--debugger`, so there is no session on standard input to take commands
/// from. This one asks the VM for its lines, through the input the program
/// already has (D112): at a terminal that is a prompt with editing and a
/// history, and under a capture it is the end of input, which detaches and
/// lets the program run on — what [`Stdio`] does when its host is gone.
///
/// Its vocabulary, its conditions and where it writes are [`Stdio`]'s. Only
/// where a line comes from differs.
pub struct OverInput {
    greeted: bool,
}

impl OverInput {
    /// Shaped as `bund2_interp::ConsoleFactory`.
    pub fn boxed() -> Box<dyn Console> {
        Box::new(Self { greeted: false })
    }
}

/// §D8: the session's history, by name. The input decides where it lives.
const HISTORY: &str = "bund2_debugger_history.txt";

fn say_stopped(greeted: &mut bool, at: &str) {
    let mut out = std::io::stderr();
    if !*greeted {
        *greeted = true;
        let _ = writeln!(
            out,
            "bund2: stopped. s(tep), n(ext), f(inish), c(ontinue), bt, st, \
             info, break <word> [<n>] [if <lambda>], break [<file>:]<line>, until <word>, \
             catch, watch @stack [if <lambda>], watch workbench, watch redefine, \
             watch depth > <n>, delete <word>, words [all|<text>], classes. \
             Any other line is run as Bund. EOF detaches and runs on."
        );
    }
    let _ = writeln!(out, "{at}");
    let _ = out.flush();
}

/// Said once, when the session ends and the program does not: at a terminal
/// Ctrl-D leaves a program running with no prompt, and this is why.
fn say_detached() {
    let mut out = std::io::stderr();
    let _ = writeln!(out, "bund2: detached; the program runs on.");
    let _ = out.flush();
}

impl Console for OverInput {
    fn stopped(&mut self, at: &str) {
        say_stopped(&mut self.greeted, at);
    }

    /// Never asked: the debuggee calls [`Console::next_command_with`]. A host
    /// with no VM to read through has no line to give, which is a detach.
    fn next_command(&mut self) -> Option<Command> {
        None
    }

    fn next_command_with(&mut self, vm: &mut dyn bund2_api::Vm) -> Option<Command> {
        use bund2_api::input::{Ask, Read};
        loop {
            // The end of input and a failed read both detach, as for `Stdio`.
            let Ok(Read::Line(line)) = vm.read_line(&Ask::in_history("(bund2) ", HISTORY)) else {
                say_detached();
                return None;
            };
            match parse(&line) {
                Ok(c) => {
                    vm.remember_line(HISTORY, &line);
                    return Some(c);
                }
                Err(msg) if msg.is_empty() => {}
                Err(msg) => bund2_api::errln!("{msg}"),
            }
        }
    }

    fn answer(&mut self, text: &str) {
        let mut out = std::io::stderr();
        let _ = write!(out, "{text}");
        let _ = out.flush();
    }

    fn evaluate_condition(&mut self, source: &str, operands: &[BundValue]) -> Result<bool, String> {
        evaluate_in_child(source, operands)
    }
}

impl Console for Stdio {
    fn stopped(&mut self, at: &str) {
        say_stopped(&mut self.greeted, at);
    }

    fn next_command(&mut self) -> Option<Command> {
        loop {
            let mut out = std::io::stderr();
            // `None` is EOF, which detaches. A read error is treated the same
            // way: there is no further input to be had, and hanging is worse.
            let Some(line) = self.read() else {
                say_detached();
                return None;
            };
            match parse(&line) {
                Ok(c) => {
                    self.remember(&line);
                    return Some(c);
                }
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

    fn evaluate_condition(&mut self, source: &str, operands: &[BundValue]) -> Result<bool, String> {
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
            ("st", Command::Stack),
            ("stack", Command::Eval("stack".to_string())),
        ] {
            assert_eq!(parse(text), Ok(want), "{text}");
        }
        assert_eq!(parse(""), Err(String::new()), "a bare return asks again");
        let e = parse("help").expect_err("help is a message");
        assert!(
            e.contains("s(tep)"),
            "and it says what the short forms are: {e}"
        );
        assert!(
            e.contains("run as Bund"),
            "and what every other line is: {e}"
        );
    }

    /// RFC-0008 §D4's depth watch and §D5's two views, and the near misses
    /// that stay Bund.
    #[test]
    fn the_depth_watch_and_the_views_parse() {
        for (text, want) in [
            ("watch depth > 100", Command::WatchDepth(100)),
            ("watch depth 3", Command::WatchDepth(3)),
            ("w depth >0", Command::WatchDepth(0)),
            ("words", Command::Words(None)),
            ("break 12", Command::BreakLine(None, 12)),
            ("b p.bund:3", Command::BreakLine(Some("p.bund".into()), 3)),
            ("words all", Command::Words(Some("all".to_string()))),
            ("words  sw ", Command::Words(Some("sw".to_string()))),
            ("classes", Command::Classes),
            ("wordsmith", Command::Eval("wordsmith".to_string())),
            ("classes x", Command::Eval("classes x".to_string())),
        ] {
            assert_eq!(parse(text), Ok(want), "{text}");
        }
        for bad in [
            "watch depth",
            "watch depth > x",
            "watch depth > -1",
            "break 12 if { true }",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        assert!(HELP.contains("watch depth > <n>") && HELP.contains("classes"));
    }

    /// **A typo must not advance the program** — and since §W2 a line that is
    /// no short form is Bund, so the guarantee is a different one: it becomes
    /// a line to run, which leaves the debuggee stopped, and never one of the
    /// four commands that resume it.
    #[test]
    fn nothing_unrecognised_becomes_a_step() {
        for text in ["x", "ste", "stepp", "quit", "run", "0", "s s", "Step"] {
            assert_eq!(
                parse(text),
                Ok(Command::Eval(text.to_string())),
                "`{text}` is a line of Bund and nothing else"
            );
        }
    }
}
