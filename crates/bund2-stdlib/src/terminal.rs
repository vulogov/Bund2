//! The terminal words: `input`, `input*`, `password`, `bund.prompt`,
//! `io.banner`, `debug.display_hostinfo`, `debug.display_memstat`,
//! `debug.display_distributed_info`, and the two debug REPLs `debug` and
//! `debug.shell`.
//!
//! **What a golden can hold.** The capture runs a program with stdin at
//! end-of-file, so `input` and `input*` only ever see Ctrl-D there. That path
//! is the reference's own: it pushes nothing and runs nothing
//! (`reference/Bund/src/stdlib/functions/io/input.rs:46-51,117-124`). The
//! prompt is a fixed string with fixed colour codes, because the reference
//! never turns yansi off. `io.banner` falls back to an 80-column width when
//! stdout is not a terminal, as it is under capture. The host table depends on
//! the machine, so no golden can hold it.

use bund2_api::input::{Ask, Input, Read};
use bund2_api::{diag::Diagnostic, Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, EXIT, LAMBDA, NONE, STRING};
use rustyline::error::ReadlineError;

use crate::host::{guard, HostOptions};
use crate::wb::Side;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// The prompt a reader was given, or `"> "` when there was none or it would
/// not cast (`input.rs:30-38`).
fn prompt_of(v: Option<BundValue>) -> String {
    v.and_then(|v| v.as_str()).unwrap_or_else(|| "> ".to_string())
}

/// **The terminal, as an input** — D112. This is what the CLI installs on its
/// interpreter; nothing in this crate reads a terminal unless an embedder
/// hands it one of these.
///
/// # The words no longer read; they ask
///
/// `input`, `input*`, `debug` and `debug.shell` ask the VM for a line
/// ([`Vm::read_line`]) and `password` for a secret ([`Vm::read_secret`]). The
/// VM asks its [`Input`], and this type is the implementor that asks
/// `rustyline` and `yapp`. So where a line comes from is the embedder's
/// choice, as where a diagnostic goes already was (D36): the terminal for a
/// plain run, nothing for a debugged one whose own commands arrive on the
/// same stream (F165), a script for a test.
///
/// # What F158 and D99 left here
///
/// The two reads are still in exactly two functions, [`Session::line`] and
/// [`secret`], and `every_terminal_read_goes_through_the_two_helpers`
/// (`crates/bund2-stdlib/src/lib.rs`) still scans the crate to say so. Under
/// this crate's own test build both still answer without touching standard
/// input. That guard is now the second lock and not the first: an interpreter
/// starts with [`bund2_api::input::NoInput`], so a test meets a terminal only
/// if it installs one.
///
/// One editor is kept per history, and one more for reads that have none, so
/// a session's recall survives from one word to the next within a program.
#[derive(Default)]
pub struct Terminal {
    plain: Option<Session>,
    sessions: std::collections::BTreeMap<String, Session>,
}

impl Terminal {
    pub fn new() -> Self {
        Terminal::default()
    }

    /// The editor a read belongs to, opened the first time it is asked for.
    fn session(&mut self, history: Option<&str>) -> Result<&mut Session, String> {
        match history {
            None => {
                if self.plain.is_none() {
                    self.plain = Some(Session::open()?);
                }
                self.plain
                    .as_mut()
                    .ok_or_else(|| "the terminal was opened and is not there".to_string())
            }
            Some(name) => {
                if !self.sessions.contains_key(name) {
                    self.sessions
                        .insert(name.to_string(), Session::open_with_history(name)?);
                }
                self.sessions
                    .get_mut(name)
                    .ok_or_else(|| "the terminal was opened and is not there".to_string())
            }
        }
    }
}

impl Input for Terminal {
    fn line(&mut self, ask: &Ask<'_>) -> Result<Read, String> {
        match self.session(ask.history)?.line(ask.prompt) {
            Ok(line) => Ok(Read::Line(line)),
            // Ctrl-C and Ctrl-D are one answer; see `Read::End`.
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => Ok(Read::End),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Remembered, and written at once. The file was written when the word
    /// returned while each word owned its editor; an editor that outlives the
    /// word has no such moment, and a history kept only until the process
    /// ends is one a crash loses.
    fn remember(&mut self, history: &str, line: &str) {
        if let Some(session) = self.sessions.get_mut(history) {
            session.remember(line);
            session.save();
        }
    }

    fn secret(&mut self, prompt: &str) -> Result<String, String> {
        secret(prompt)
    }
}

/// One `rustyline` editor and the history it keeps, if it keeps one.
struct Session {
    #[cfg(not(test))]
    rl: rustyline::DefaultEditor,
    /// Where this session's history is kept, when it keeps one.
    history: Option<std::path::PathBuf>,
    /// Whether a line has been remembered since the history was loaded.
    dirty: bool,
}

/// The directory a platform keeps per-user configuration in.
///
/// `XDG_CONFIG_HOME` wins wherever it is set to an absolute path — the
/// specification calls a relative one invalid — and otherwise the platform's
/// own convention applies: `%APPDATA%` on Windows, `~/Library/Application
/// Support` on macOS, `~/.config` elsewhere.
///
/// **Written out rather than taken from a crate.** It is fifteen lines, D28
/// asks the default build to stay small, and taking its inputs as arguments
/// lets a test ask every branch without touching the process environment —
/// which tests share, being threads of one process.
fn config_home(
    xdg: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
    appdata: Option<std::ffi::OsString>,
) -> Option<std::path::PathBuf> {
    let dir = |v: Option<std::ffi::OsString>| {
        v.filter(|s| !s.is_empty()).map(std::path::PathBuf::from)
    };
    if let Some(x) = dir(xdg).filter(|p| p.is_absolute()) {
        return Some(x);
    }
    if cfg!(windows) {
        return dir(appdata);
    }
    let home = dir(home)?;
    Some(if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support")
    } else {
        home.join(".config")
    })
}

/// Where a named history file lives — RFC-0008 §D8, F10.
///
/// **The reference writes these into the working directory**
/// (`reference/Bund/src/stdlib/functions/debug_fun/debug_shell.rs`,
/// `debug_debug.rs`). Bund2 does not, deliberately: the file is the session's
/// state and not the program's output, and a program's directory is not the
/// session's to write in. The names are the reference's, under a `bund2`
/// directory of the platform's own.
///
/// `None` when no such directory can be named, and then no history is kept —
/// silently, since a session with nowhere to remember is still a session.
fn history_path(file: &str) -> Option<std::path::PathBuf> {
    config_home(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
        std::env::var_os("APPDATA"),
    )
    .map(|d| d.join("bund2").join(file))
}

impl Session {
    fn open() -> Result<Self, String> {
        #[cfg(not(test))]
        let terminal = Self {
            rl: rustyline::DefaultEditor::new().map_err(|e| e.to_string())?,
            history: None,
            dirty: false,
        };
        #[cfg(test)]
        let terminal = Self {
            history: None,
            dirty: false,
        };
        Ok(terminal)
    }

    /// A terminal that remembers its lines between sessions, in `file`.
    ///
    /// A history that is missing or will not load is not an error and is not
    /// reported: the reference logs it at a level its default configuration
    /// does not show, and a first run has none by definition.
    fn open_with_history(file: &str) -> Result<Self, String> {
        let mut terminal = Self::open()?;
        terminal.history = history_path(file);
        #[cfg(not(test))]
        if let Some(path) = &terminal.history {
            let _ = terminal.rl.load_history(path);
        }
        Ok(terminal)
    }

    /// Add a line to the history. A session that never calls this writes no
    /// file, which is what keeps a capture — where every read is end-of-input
    /// — from creating one.
    fn remember(&mut self, line: &str) {
        #[cfg(not(test))]
        {
            let _ = self.rl.add_history_entry(line);
        }
        let _ = line;
        self.dirty = true;
    }

    /// Write the history back, if there is anything new and anywhere to put
    /// it. A failure is dropped, as the reference drops it: losing a history
    /// is not a reason to fail the program that was being debugged.
    fn save(&mut self) {
        if !self.dirty {
            return;
        }
        let Some(path) = self.history.as_ref() else {
            return;
        };
        #[cfg(not(test))]
        {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = self.rl.save_history(path);
        }
        let _ = path;
        self.dirty = false;
    }

    /// One line, with `rustyline`'s own result type so every caller keeps the
    /// match it had: a line, `Interrupted` or `Eof` for the end, anything else
    /// a failure.
    fn line(&mut self, prompt: &str) -> Result<String, ReadlineError> {
        #[cfg(not(test))]
        {
            self.rl.readline(prompt)
        }
        #[cfg(test)]
        {
            let _ = prompt;
            Err(ReadlineError::Eof)
        }
    }
}

/// The history is written on the way out, **whichever way out it is** — the
/// REPLs below return early on a failed read, and a `save` at the bottom of
/// the loop would be skipped on exactly the session that ended badly.
impl Drop for Session {
    fn drop(&mut self) {
        self.save();
    }
}

/// The other place: a line read without echo, for `password`.
///
/// It goes through `yapp` rather than `rustyline`, which is why a fix at the
/// line reader alone would have left it reading. Under the test build it
/// refuses, naming why, rather than answering an empty secret a caller might
/// take for a real one.
fn secret(prompt: &str) -> Result<String, String> {
    #[cfg(not(test))]
    {
        use yapp::PasswordReader;
        yapp::Yapp::new()
            .with_echo_symbol('.')
            .read_password_with_prompt(prompt)
            .map_err(|e| e.to_string())
    }
    #[cfg(test)]
    {
        let _ = prompt;
        Err("no terminal is read under the test build (F158)".to_string())
    }
}

/// `input` — read one line from the terminal (`input.rs:20-57`).
///
/// The prompt is pulled, and the line is pushed with its surrounding space
/// trimmed. Ctrl-C and Ctrl-D push nothing and are not errors, so the word
/// answers zero or one value: its effect is opaque.
fn input(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline INPUT".into()));
    }
    let prompt = prompt_of(vm.pull());
    match vm.read_line(&Ask::new(&prompt)) {
        Ok(Read::Line(line)) => vm.push(BundValue::str(line.trim())),
        Ok(Read::End) => {}
        Err(e) => return Err(Error(format!("INPUT line returns: {e}"))),
    }
    Ok(())
}

/// `input*` — read lines until Ctrl-C or Ctrl-D, running a lambda on each
/// (`input.rs:76-131`). The lambda is on top and the prompt beneath it.
///
/// **The lambda's type is never checked** (F108): the reference writes
/// `! lambda_value.type_of() == LAMBDA` (`:91`), a bitwise NOT of the tag
/// compared with `LAMBDA`, which is always false. So anything is accepted, and
/// a value that is not a lambda fails only when the first line arrives, where
/// the reference's lambda evaluator refuses it:
/// `reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:27-29`.
fn input_loop(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline INPUT".into()));
    }
    let lambda = vm
        .pull()
        .ok_or_else(|| Error("Error getting INPUT* lambda from stack".into()))?;
    let prompt = prompt_of(vm.pull());
    loop {
        match vm.read_line(&Ask::new(&prompt)) {
            Ok(Read::Line(line)) => {
                vm.push(BundValue::str(line.trim()));
                if lambda.dt() != LAMBDA {
                    return Err(Error(
                        "INPUT* returned error from LAMBDA: This is not a lambda".into(),
                    ));
                }
                vm.eval_lambda(&lambda)
                    .map_err(|e| Error(format!("INPUT* returned error from LAMBDA: {}", e.0)))?;
            }
            Ok(Read::End) => break,
            Err(e) => return Err(Error(format!("INPUT line returns: {e}"))),
        }
    }
    Ok(())
}

/// `password` — read a line without echoing it (`input.rs:59-74`). Each key
/// shows as a `.`.
fn password(vm: &mut dyn Vm) -> Result<(), Error> {
    let m = vm
        .pull()
        .ok_or_else(|| Error("PASSWORD: NO DATA #1".into()))?;
    let msg = m.as_str().ok_or_else(|| {
        Error("PASSWORD error casting message: This Dynamic type is not string".into())
    })?;
    let res = vm
        .read_secret(&msg)
        .map_err(|e| Error(format!("PASSWORD returns: {e}")))?;
    vm.push(BundValue::str(res));
    Ok(())
}

/// `bund.prompt` — push the coloured `[BUND> ` prompt (`input.rs:14-18`).
fn bund_prompt(vm: &mut dyn Vm) -> Result<(), Error> {
    use yansi::Paint;
    let prompt = format!(
        "{}{}{}{}{} {} ",
        Paint::yellow("["),
        Paint::red("B"),
        Paint::blue("U").bold(),
        Paint::white("N"),
        Paint::cyan("D"),
        Paint::green(">").bold()
    );
    vm.push(BundValue::str(prompt));
    Ok(())
}

/// The `[DEBUG> ` prompt both debug REPLs show — the same three paints in
/// `debug_shell.rs` and `debug_debug.rs`.
fn debug_prompt() -> String {
    use yansi::Paint;
    format!(
        "{}{} {} ",
        Paint::yellow("["),
        Paint::red("DEBUG"),
        Paint::white(">").bold()
    )
}

/// `debug.shell` — a REPL on the running VM
/// (`reference/Bund/src/stdlib/functions/debug_fun/debug_shell.rs`,
/// `debug_shell`).
///
/// Each line is evaluated against the *live* VM, so the session sees and
/// changes the stacks the program built. A line that fails does not end the
/// loop.
///
/// **Under capture it is a no-op**, which is the only reason a golden can hold
/// it: the runner gives a program stdin at `/dev/null`, so the first read is
/// end-of-file and the word returns having done nothing. Measured on the
/// oracle -- `debug.shell "after" println` prints `after`.
///
/// **Opaque, and that is what keeps it out of D55's palette.** It evaluates
/// whatever is typed, so it has no effect to declare; the audit skips opaque
/// natives before running anything, so F141's hang hazard is answered by the
/// type rather than by a list the next session has to remember.
///
/// Two departures from the reference, both deliberate. It **reports** a failing
/// line rather than printing it, because D36 says a word does not write to
/// stderr itself; the severity is `Warning` because the program has not
/// stopped. And a readline error that is neither Ctrl-C nor Ctrl-D ends the
/// word instead of looping -- the reference prints and continues, which spins
/// if the condition persists, and `input*` here already chose to error out.
fn debug_shell(vm: &mut dyn Vm) -> Result<(), Error> {
    // §D8: the reference keeps this history in the working directory.
    const HISTORY: &str = "bund_debug_shell_history.txt";
    let prompt = debug_prompt();
    loop {
        match vm.read_line(&Ask::in_history(&prompt, HISTORY)) {
            Ok(Read::Line(line)) => {
                // Every line, an empty one included, as the reference does.
                vm.remember_line(HISTORY, &line);
                if let Err(e) = crate::singles::eval_source(vm, &line) {
                    vm.report(Diagnostic::warning(e.0));
                }
            }
            Ok(Read::End) => break,
            Err(e) => return Err(Error(format!("INPUT line returns: {e}"))),
        }
    }
    Ok(())
}

/// One value's three-row table, as `debug` shows it before applying it
/// (`debug_debug.rs`, `bund_debug_print_word`).
///
/// The rows are the type name, the value converted to STRING, and the
/// `Debug` form. A conversion that fails shows `NONE`, as the reference's
/// `Value::none()` does.
///
/// **The third row is `render`, not Rust's `{:?}`.** `BundValue` is not the
/// reference's `Value`, so its derived `Debug` prints
/// `Int(1, StackSym(0))` where the oracle prints
/// `Value { id: …, dt: 2, …, data: I64(1), … }`. `render` is the emulation
/// `debug.display_stack`'s goldens are built on, and this row is the same
/// claim, so it uses the same function. Found by diffing against the oracle,
/// not by reading.
fn debug_print_word(v: &BundValue) {
    use comfy_table::modifiers::UTF8_ROUND_CORNERS;
    use comfy_table::presets::UTF8_FULL;
    use comfy_table::{ContentArrangement, Table};
    let shown = crate::convert::conv_value(v, STRING)
        .map(|s| s.display())
        .unwrap_or_else(|_| BundValue::none().display());
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .add_row(vec!["Value type", v.type_name()])
        .add_row(vec!["Value", &shown])
        .add_row(vec!["Debug", &v.render(false)]);
    println!("{table}");
}

/// `debug` — step a snippet, one top-level value at a time
/// (`debug_debug.rs`, `debug_debug` and `bund_debugger`).
///
/// The operand is a STRING of Bund source. It is parsed, and for each value:
/// the table above is printed, the value is **applied to the live VM**, and
/// then lines are read and evaluated until a blank one or end-of-file moves on
/// to the next value. So the pause is *after* each value, and the stack a
/// session inspects is the one that value left.
///
/// **The `EXIT` arm is load-bearing and nearly went missing.** A compiled
/// stream ends with an `EXIT` value (dt 93), so without the break this word
/// prints a fourth table for it, applies it, and leaves it on the stack --
/// which is what the first version did, and what diffing against the oracle
/// showed. The reference breaks on `EXIT`, and so does this.
///
/// The `ERROR` arm is the one with no representable input: Bund2's value model
/// has no such tag, so "bail on an error posted on the stack" is absent by
/// construction rather than written as an arm that cannot run -- D37's first
/// preference. A `NONE` value is skipped, as in the reference.
///
/// Opaque, for the same reason `debug.shell` is.
fn debug_word(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline DEBUG".into()));
    }
    // The reference guards the depth and *then* pulls, with a second message
    // for a pull that cannot fail after the guard. Both are reproduced.
    let v = vm
        .pull()
        .ok_or_else(|| Error("Stack is too shallow for debug()".into()))?;
    let snippet = v.as_str().ok_or_else(|| {
        Error("Casting debug snippet returns: This Dynamic type is not string".into())
    })?;
    // `\n` appended, as `bund_debugger` does before parsing.
    let source = format!("{snippet}\n");
    let words = bund2_syntax::compile(&source).map_err(|e| Error(e.render(&source)))?;
    const HISTORY: &str = "bund_debug_debugger_history.txt";
    let prompt = debug_prompt();
    for word in words {
        if word.dt() == NONE {
            continue;
        }
        if word.dt() == EXIT {
            break;
        }
        debug_print_word(&word);
        vm.apply(word.clone())
            .map_err(|e| {
                Error(format!(
                    "Attempt to evaluate value {} returned error: {}",
                    word.render(false),
                    e.0
                ))
            })?;
        loop {
            match vm.read_line(&Ask::in_history(&prompt, HISTORY)) {
                // An empty line moves on, and is *not* remembered here —
                // the reference breaks before it reaches `add_history_entry`.
                Ok(Read::Line(line)) if line.is_empty() => break,
                Ok(Read::Line(line)) => {
                    vm.remember_line(HISTORY, &line);
                    if let Err(e) = crate::singles::eval_source(vm, &line) {
                        vm.report(Diagnostic::warning(e.0));
                    }
                }
                Ok(Read::End) => break,
                Err(e) => return Err(Error(format!("INPUT line returns: {e}"))),
            }
        }
    }
    Ok(())
}

/// Print one of the debugger's views — RFC-0008 §W1.
///
/// The VM renders it, so a script, `debug.shell` and a line typed at a
/// console stop print the same text from the same code. **To standard
/// output**, as `debug.display_stack` does (D113.3): a word's output is the
/// program's, wherever the word was typed.
fn debug_view(vm: &mut dyn Vm, what: bund2_api::Debugging, who: &str) -> Result<(), Error> {
    match vm.debugging(what)? {
        Some(text) => {
            print!("{text}");
            Ok(())
        }
        None => Err(Error(format!("{who}: this VM renders no such view"))),
    }
}

/// Pull the name an arming word was given — a STRING, with the `@` a stack
/// may be written with taken off, as the console takes it off.
fn debug_name(vm: &mut dyn Vm, who: &str) -> Result<String, Error> {
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {who}")));
    }
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("{who} returns: NO DATA #1")))?;
    match v.as_str() {
        Some(name) if !name.trim_start_matches('@').is_empty() => {
            Ok(name.trim_start_matches('@').to_string())
        }
        Some(_) => Err(Error(format!("{who}: the name is empty"))),
        None => Err(Error(format!("{who}: the name is not a string"))),
    }
}

/// Ask the debugger to arm or move — RFC-0008 §W5. Prints nothing: a word
/// that arms from a script must not write into the program's output.
fn debug_ask(vm: &mut dyn Vm, what: bund2_api::Debugging, who: &str) -> Result<(), Error> {
    vm.debugging(what)
        .map(|_| ())
        .map_err(|e| Error(format!("{who}: {}", e.0)))
}

/// `debug.break` — stop when the named word is called. Bund2's own word.
fn debug_break(vm: &mut dyn Vm) -> Result<(), Error> {
    let word = debug_name(vm, "DEBUG.BREAK")?;
    debug_ask(vm, bund2_api::Debugging::Break(word), "DEBUG.BREAK")
}

/// `debug.break.if` — the same, when a condition holds. The word name, then
/// the condition **as Bund source in a STRING**: `"w" "{ depth 3 > }"`.
///
/// Source and not a LAMBDA value, though §D3 calls the condition a lambda:
/// it runs in a child VM so that it cannot change the program (§D3), the
/// child is handed text, and a lambda value has no source form to hand it.
/// The text may itself be a lambda literal, which the child then runs.
fn debug_break_if(vm: &mut dyn Vm) -> Result<(), Error> {
    const WHO: &str = "DEBUG.BREAK.IF";
    if vm.depth() < 2 {
        return Err(Error(format!("Stack is too shallow for inline {WHO}")));
    }
    let cond = vm
        .pull()
        .ok_or_else(|| Error(format!("{WHO} returns: NO DATA #1")))?;
    let Some(cond) = cond.as_str() else {
        return Err(Error(format!("{WHO}: the condition is not a string of Bund source")));
    };
    let word = debug_name(vm, WHO)?;
    debug_ask(vm, bund2_api::Debugging::BreakIf(word, cond), WHO)
}

/// `debug.watch` — stop on a push to the named stack. Bund2's own word.
fn debug_watch(vm: &mut dyn Vm) -> Result<(), Error> {
    let stack = debug_name(vm, "DEBUG.WATCH")?;
    debug_ask(vm, bund2_api::Debugging::Watch(stack), "DEBUG.WATCH")
}

/// `debug.watch.workbench` — stop on a push to the workbench.
fn debug_watch_workbench(vm: &mut dyn Vm) -> Result<(), Error> {
    debug_ask(vm, bund2_api::Debugging::WatchWorkbench, "DEBUG.WATCH.WORKBENCH")
}

/// `debug.delete` — stop watching the named word or stack; `"workbench"`
/// names the workbench, as at the console.
fn debug_delete(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = debug_name(vm, "DEBUG.DELETE")?;
    debug_ask(vm, bund2_api::Debugging::Delete(name), "DEBUG.DELETE")
}

/// `debug.step` — stop before the next value. In a script this is where the
/// program hands itself to a debugger.
fn debug_step(vm: &mut dyn Vm) -> Result<(), Error> {
    debug_ask(vm, bund2_api::Debugging::Step, "DEBUG.STEP")
}

/// `debug.run` — evaluate a STRING of Bund, stopped before its first term.
/// Bund2's own word (D113.5).
///
/// `debug.step` followed by `bund.eval` stops inside the words the string
/// calls and never at the string's own terms, because those reach the VM
/// through `apply` and not through a loop that has a safepoint. This word
/// offers the safepoint itself, one per term, so `s` walks the string and
/// goes into a word's body, and `n` walks it and does not.
///
/// The reference's `debug` is left as it is: it prints a table per term and
/// then reads lines (`reference/Bund/src/stdlib/functions/debug_fun/
/// debug_debug.rs:81-95`), which is a different thing to want.
///
/// **Typed at a stop, the string runs unstepped**: nothing in a typed line is
/// a stop (§W2), and the stepping the word asked for applies to the program
/// once the line is done.
fn debug_run(vm: &mut dyn Vm) -> Result<(), Error> {
    const WHO: &str = "DEBUG.RUN";
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {WHO}")));
    }
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("{WHO} returns: NO DATA #1")))?;
    let Some(src) = v.as_str() else {
        return Err(Error(format!("{WHO}: the source is not a string")));
    };
    let stream = bund2_syntax::compile(&src).map_err(|e| Error(e.render(&src)))?;
    debug_ask(vm, bund2_api::Debugging::Step, WHO)?;
    for (i, word) in stream.into_iter().enumerate() {
        if word.dt() == bund2_value::NONE {
            continue;
        }
        if word.dt() == bund2_value::EXIT {
            break;
        }
        debug_ask(vm, bund2_api::Debugging::Term(i, word.clone()), WHO)?;
        vm.apply(word)?;
    }
    Ok(())
}

/// `debug.next` — stop before the next value no deeper than here.
fn debug_next(vm: &mut dyn Vm) -> Result<(), Error> {
    debug_ask(vm, bund2_api::Debugging::Next, "DEBUG.NEXT")
}

/// `debug.finish` — stop when the body running now has returned.
fn debug_finish(vm: &mut dyn Vm) -> Result<(), Error> {
    debug_ask(vm, bund2_api::Debugging::Finish, "DEBUG.FINISH")
}

/// `debug.continue` — stop nowhere until something armed fires.
fn debug_continue(vm: &mut dyn Vm) -> Result<(), Error> {
    debug_ask(vm, bund2_api::Debugging::Continue, "DEBUG.CONTINUE")
}

/// `debug.backtrace` — the frame stack, innermost first. Bund2's own word.
fn debug_backtrace(vm: &mut dyn Vm) -> Result<(), Error> {
    debug_view(vm, bund2_api::Debugging::Backtrace, "DEBUG.BACKTRACE")
}

/// `debug.stacks` — every stack, its depth and its top. Bund2's own word.
fn debug_stacks(vm: &mut dyn Vm) -> Result<(), Error> {
    debug_view(vm, bund2_api::Debugging::Stacks, "DEBUG.STACKS")
}

/// `debug.info` — what a debugger has armed. Bund2's own word.
fn debug_info(vm: &mut dyn Vm) -> Result<(), Error> {
    debug_view(vm, bund2_api::Debugging::Info, "DEBUG.INFO")
}

/// `debug.feed` — queue one line for the next word that reads (§W4).
///
/// The way to type into a debugged program, whose console owns standard
/// input (F165, D112). The line waits in the VM and is answered before any
/// input is asked, so it reaches `input`, `input*`, `password` and
/// `debug.shell` alike. Bund2's own word; the reference has no counterpart.
fn debug_feed(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline DEBUG.FEED".into()));
    }
    let v = vm
        .pull()
        .ok_or_else(|| Error("DEBUG.FEED returns: NO DATA #1".into()))?;
    let Some(line) = v.as_str() else {
        return Err(Error("DEBUG.FEED: the line is not a string".into()));
    };
    if vm.feed_line(line.to_string()) {
        Ok(())
    } else {
        Err(Error("DEBUG.FEED: this VM keeps no lines for a read".into()))
    }
}

/// `debug.display_memstat` — this process's own memory use
/// (`reference/Bund/src/stdlib/functions/debug_fun/debug_display_memstats.rs`).
///
/// A `--nocolor` pair like `debug.display_hostinfo`, and the two differ only in
/// the cell colours. **No golden can hold the table**: the figures are this
/// process's, and two oracle runs a second apart reported 28.51 MB and
/// 28.56 MB. They are also textually identical under capture, because neither
/// comfy_table nor yansi colours a pipe -- so the `--nocolor` distinction is
/// unobservable there, which is F66's shape.
///
/// `memory_stats` answering `None` prints the reference's one-line fallback
/// rather than failing.
fn debug_display_memstat(vm: &mut dyn Vm) -> Result<(), Error> {
    let _ = vm;
    println!("{}", memstat_table());
    Ok(())
}

/// The table `debug.display_memstat` prints, built separately so a test can
/// read it without capturing stdout -- `hostinfo_table` is factored the same
/// way and for the same reason.
fn memstat_table() -> String {
    use comfy_table::modifiers::UTF8_ROUND_CORNERS;
    use comfy_table::presets::UTF8_FULL;
    use comfy_table::{ContentArrangement, Table};
    let Some(usage) = memory_stats::memory_stats() else {
        return "Couldn't get the current memory usage.".to_string();
    };
    use humansize::{format_size, DECIMAL};
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .add_row(vec![
            "Current physical memory usage",
            &format_size(usage.physical_mem, DECIMAL),
        ])
        .add_row(vec![
            "Current virtual memory usage",
            &format_size(usage.virtual_mem, DECIMAL),
        ]);
    format!("{table}")
}

/// `debug.display_distributed_info` — the bus node's identity
/// (`reference/Bund/src/stdlib/functions/debug_fun/debug_display_distributed_info.rs`).
///
/// **Its first act is to require `--distributed`**, and failing that it logs
/// one line and returns `Ok`. Bund2 has no `--distributed` and links no zenoh
/// session, so that is the word's whole reachable behaviour here: the table it
/// would otherwise draw needs a bus Bund2 does not have, which RFC-0007 and
/// D92 govern rather than this word.
///
/// The reference logs at ERROR and keeps going; this reports at `Warning`,
/// since the program has not stopped (D36), and the line is therefore the
/// reporter's rather than a timestamped `log::error!`. No golden can hold
/// either: the oracle's carries a wall clock, and stderr is concatenated into
/// the captured output -- the same reason `log.error` is permanently
/// unreachable.
fn debug_display_distributed_info(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.report(Diagnostic::warning(
        "BUND must be in distributed mode. You shall pass --distributed to CLI",
    ));
    Ok(())
}

/// `io.banner` — draw a value as large text in cfonts' tiny font
/// (`reference/Bund/src/stdlib/functions/io/banner.rs:11-60`).
///
/// The value is converted to STRING first (`:32`), so any value draws as it
/// prints. The banner always goes to the stack (`:43`), the `.` form included.
fn io_banner(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #1")))?;
    let s = crate::convert::conv_value(&v, STRING)
        .map_err(|e| Error(format!("{prefix} returns: NO CONVERSION #1: {}", e.0)))?;
    let text = s.as_str().ok_or_else(|| {
        Error(format!(
            "{prefix} returns: NO CASTING #1: This Dynamic type is not string"
        ))
    })?;
    let out = cfonts::render(cfonts::Options {
        text,
        font: cfonts::Fonts::FontTiny,
        ..cfonts::Options::default()
    })
    .text;
    vm.push(BundValue::str(out));
    Ok(())
}

/// The table `debug.display_hostinfo` prints
/// (`reference/Bund/src/stdlib/functions/debug_fun/debug_display_hostinfo.rs:12-136`).
///
/// **D53: the six version rows name Bund2's crates.** The reference lists its
/// own internal crates there (`:39-56`). Bund2's crates share one workspace
/// version, which is this crate's. The host rows are the reference's, read
/// through the same `sys_metrics`. A value that cannot be read shows as
/// `Unknown` (`:23-34`). Bund2 has no distributed mode, so that row is always
/// `false`.
fn hostinfo_table(color: bool) -> String {
    use comfy_table::modifiers::UTF8_ROUND_CORNERS;
    use comfy_table::presets::UTF8_FULL;
    use comfy_table::{Cell, Color, ContentArrangement, Table};
    let or_unknown = |r: Result<String, _>| r.unwrap_or_else(|_: std::io::Error| "Unknown".to_string());
    let version = env!("CARGO_PKG_VERSION").to_string();
    let rows: Vec<(&str, String, Color)> = vec![
        ("bund2-value version", version.clone(), Color::Green),
        ("bund2-api version", version.clone(), Color::Green),
        ("bund2-syntax version", version.clone(), Color::Green),
        ("bund2-ir version", version.clone(), Color::Green),
        ("bund2-interp version", version.clone(), Color::Green),
        ("bund2-stdlib version", version, Color::Green),
        ("Distributed mode", "false".to_string(), Color::Blue),
        ("Hostname", or_unknown(sys_metrics::host::get_hostname()), Color::Blue),
        ("OS version", or_unknown(sys_metrics::host::get_os_version()), Color::Blue),
        ("Virtualization", crate::sysinfo::virtualization(), Color::Blue),
        (
            "Kernel version",
            or_unknown(sys_metrics::host::get_kernel_version()),
            Color::Blue,
        ),
    ];
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    for (k, v, c) in rows {
        if color {
            table.add_row(vec![Cell::new(k).fg(c), Cell::new(v).fg(Color::White)]);
        } else {
            table.add_row(vec![Cell::new(k), Cell::new(v)]);
        }
    }
    table.to_string()
}

pub fn register(r: &mut Registry, opts: &HostOptions) {
    // `reference/Bund/src/stdlib/functions/io/input.rs:142-145`. `input` and
    // `input*` are opaque: one answers zero or one value, the other runs a
    // body per line.
    r.register_native("input", input, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("input*", input_loop, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("password", password, eff(1, 1), WordKind::Sync);
    r.register_native("bund.prompt", bund_prompt, eff(0, 1), WordKind::Sync);
    // `reference/Bund/src/stdlib/functions/io/banner.rs:82-88`.
    if opts.noio {
        for name in ["io.banner", "io.banner."] {
            r.register_native(
                name,
                |_vm| Err(Error("bund IO.BANNER functions disabled with --noio".into())),
                if name == "io.banner" { eff(1, 1) } else { eff(0, 1) },
                WordKind::Sync,
            );
        }
    } else {
        r.register_native(
            "io.banner",
            |vm| io_banner(vm, Side::Stack, "IO.BANNER"),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "io.banner.",
            |vm| io_banner(vm, Side::Bench, "IO.BANNER."),
            eff(0, 1),
            WordKind::Sync,
        );
    }
    // `bund/debug_fun`'s remaining four. `debug` and `debug.shell` are opaque
    // because they evaluate whatever is typed -- which also keeps them out of
    // D55's palette, where a word that reads stdin is F141's hazard.
    r.register_native("debug", debug_word, StackEffect::opaque(1), WordKind::Sync);
    // RFC-0008 §W1, Part A. Opaque, as `debug.shell` is: what they read is
    // the run and not a stack effect, which keeps them out of D55's palette.
    r.register_native("debug.backtrace", debug_backtrace, StackEffect::opaque(0), WordKind::Sync);
    r.register_native("debug.stacks", debug_stacks, StackEffect::opaque(0), WordKind::Sync);
    r.register_native("debug.info", debug_info, StackEffect::opaque(0), WordKind::Sync);
    r.register_native("debug.feed", debug_feed, StackEffect::opaque(1), WordKind::Sync);
    // Part B: arming and moving. Opaque for the same reason, and because a
    // moving word changes where the *run* goes, which no effect describes.
    for (name, f, n) in [
        ("debug.break", debug_break as bund2_api::NativeFn, 1),
        ("debug.break.if", debug_break_if, 2),
        ("debug.watch", debug_watch, 1),
        ("debug.watch.workbench", debug_watch_workbench, 0),
        ("debug.delete", debug_delete, 1),
        ("debug.step", debug_step, 0),
        ("debug.run", debug_run, 1),
        ("debug.next", debug_next, 0),
        ("debug.finish", debug_finish, 0),
        ("debug.continue", debug_continue, 0),
    ] {
        r.register_native(name, f, StackEffect::opaque(n), WordKind::Sync);
    }
    r.register_native(
        "debug.shell",
        debug_shell,
        StackEffect::opaque(0),
        WordKind::Sync,
    );
    r.register_native(
        "debug.display_memstat",
        debug_display_memstat,
        eff(0, 0),
        WordKind::Sync,
    );
    r.register_native(
        "debug.display_distributed_info",
        debug_display_distributed_info,
        eff(0, 0),
        WordKind::Sync,
    );

    // `debug_display_hostinfo.rs:156-160`: `--nocolor` picks the plain table.
    if opts.nocolor {
        r.register_native(
            "debug.display_hostinfo",
            |_vm| {
                println!("{}", hostinfo_table(false));
                Ok(())
            },
            eff(0, 0),
            WordKind::Sync,
        );
    } else {
        r.register_native(
            "debug.display_hostinfo",
            |_vm| {
                println!("{}", hostinfo_table(true));
                Ok(())
            },
            eff(0, 0),
            WordKind::Sync,
        );
    }
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm;
    use bund2_interp::Interp;

    fn interp() -> Interp {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i
    }

    fn run(i: &mut Interp, src: &str) -> Result<(), String> {
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)
    }

    /// An interpreter whose input is these lines, and a handle on what it
    /// was asked.
    fn typed(
        lines: &[&str],
        secrets: &[&str],
    ) -> (Interp, std::rc::Rc<std::cell::RefCell<bund2_api::input::Seen>>) {
        let script = bund2_api::input::Scripted::lines(lines.iter().copied())
            .with_secrets(secrets.iter().copied());
        let seen = script.seen();
        let mut i = interp();
        i.input = Box::new(script);
        (i, seen)
    }

    fn shown(i: &Interp) -> Vec<String> {
        i.snapshot().iter().map(|v| v.display()).collect()
    }

    /// **D112: the words ask the VM, so a test can answer.** Before the seam
    /// every test of these words reached one arm, the end of input. This is
    /// `input` with a line typed: the prompt it was given is the prompt
    /// asked, and the line is pushed with its surrounding space trimmed
    /// (`reference/Bund/src/stdlib/functions/io/input.rs:20-57`).
    #[test]
    fn input_pushes_the_line_it_is_given() {
        let (mut i, seen) = typed(&["  hello world  "], &[]);
        run(&mut i, "\"name? \" input").expect("runs");
        assert_eq!(shown(&i), ["hello world"]);
        assert_eq!(seen.borrow().prompts, ["name? "]);

        // A prompt that is not a string is the default one (`:30-38`).
        let (mut i, seen) = typed(&["x"], &[]);
        run(&mut i, "42 input").expect("runs");
        assert_eq!(seen.borrow().prompts, ["> "]);

        // The end of input pushes nothing and is not an error.
        let (mut i, _) = typed(&[], &[]);
        run(&mut i, "\"> \" input").expect("runs");
        assert!(shown(&i).is_empty());
    }

    /// **RFC-0008 §W4, criterion W5: a fed line reaches every word that
    /// reads**, before the input is asked and in the order fed. The
    /// interpreter here has no input at all, which is what a debugged program
    /// has (D112): without the queue each of these reads is the end.
    #[test]
    fn a_fed_line_reaches_every_reading_word() {
        let mut i = interp();
        run(&mut i, "\"alice\" debug.feed \"name? \" input").expect("runs");
        assert_eq!(shown(&i), ["alice"]);

        let mut i = interp();
        run(&mut i, "\"a\" debug.feed \"b\" debug.feed \"> \" { string.upper } input*")
            .expect("runs");
        assert_eq!(shown(&i), ["A", "B"], "both, in the order fed, then the end");

        // `password` has no terminal here and is refused without a fed line.
        let mut i = interp();
        run(&mut i, "\"hunter2\" debug.feed \"pw: \" password").expect("runs");
        assert_eq!(shown(&i), ["hunter2"]);

        // `debug.shell` runs the fed line as Bund and then meets the end.
        let mut i = interp();
        run(&mut i, "\"40 2 +\" debug.feed debug.shell").expect("runs");
        assert_eq!(shown(&i), ["42"]);
    }

    /// A fed line is answered before the input, and the input is then asked
    /// as it always was.
    #[test]
    fn a_fed_line_comes_before_the_input() {
        let (mut i, seen) = typed(&["typed"], &[]);
        run(&mut i, "\"fed\" debug.feed \"> \" input \"> \" input").expect("runs");
        assert_eq!(shown(&i), ["fed", "typed"]);
        assert_eq!(seen.borrow().prompts.len(), 1, "the fed read asked nobody");
    }

    /// `debug.feed` refuses what is not a line, in its own words.
    #[test]
    fn debug_feed_refuses_what_is_not_a_string() {
        let mut i = interp();
        let e = run(&mut i, "debug.feed").expect_err("nothing to feed");
        assert!(e.ends_with("Stack is too shallow for inline DEBUG.FEED"), "{e}");
        let e = run(&mut i, "42 debug.feed").expect_err("not a line");
        assert!(e.ends_with("DEBUG.FEED: the line is not a string"), "{e}");
    }

    /// **§W5: with no console to attach, an arming or moving word refuses in
    /// words** and the views still answer. This interpreter is a bare one: the
    /// embedder that supplies a console is the CLI.
    #[test]
    fn arming_refuses_where_no_console_can_be_attached() {
        let mut i = interp();
        for src in ["\"w\" debug.break", "debug.step", "debug.watch.workbench", "\"1\" debug.run"] {
            let e = run(&mut i, src).expect_err("nothing to attach");
            assert!(
                e.ends_with("no debugger is attached, and this VM was given no console to attach"),
                "{src}: {e}"
            );
        }
        // And each says what it wanted, before it asks.
        for (src, want) in [
            ("debug.break", "Stack is too shallow for inline DEBUG.BREAK"),
            ("42 debug.break", "DEBUG.BREAK: the name is not a string"),
            ("\"@\" debug.watch", "DEBUG.WATCH: the name is empty"),
            ("\"w\" debug.break.if", "Stack is too shallow for inline DEBUG.BREAK.IF"),
            ("debug.run", "Stack is too shallow for inline DEBUG.RUN"),
            ("5 debug.run", "DEBUG.RUN: the source is not a string"),
            ("\"w\" { true } debug.break.if", "DEBUG.BREAK.IF: the condition is not a string of Bund source"),
        ] {
            let mut i = interp();
            let e = run(&mut i, src).expect_err("refused");
            assert!(e.ends_with(want), "{src}: {e}");
        }
    }

    /// **Criterion W1, the half a unit test can hold: the three views are the
    /// VM's, and a word only prints them.** With nothing attached `debug.info`
    /// says so; the frame stack names the word a view was asked from.
    #[test]
    fn the_views_are_rendered_by_the_vm() {
        use bund2_api::Debugging;
        let mut i = interp();
        run(&mut i, "1 2 3 @other 9 @main").expect("runs");
        let stacks = i.debugging(Debugging::Stacks).expect("answers").expect("a view");
        assert!(stacks.contains("* @main  3  top: 3\n"), "{stacks}");
        assert!(stacks.contains("  @other  1  top: 9\n"), "{stacks}");
        assert!(stacks.ends_with("  workbench  0\n"), "{stacks}");
        assert_eq!(
            i.debugging(Debugging::Info).expect("answers").as_deref(),
            Some("no debugger is attached\n")
        );
        assert_eq!(
            i.debugging(Debugging::Backtrace).expect("answers").as_deref(),
            Some("#0  the top-level stream\n")
        );
        // And the words run.
        run(&mut i, "debug.backtrace debug.stacks debug.info").expect("they print");
    }

    /// `input*` runs its lambda on every line until the input ends, and asks
    /// once more than it was answered.
    #[test]
    fn input_star_runs_its_lambda_on_each_line() {
        let (mut i, seen) = typed(&["a", " b ", "c"], &[]);
        run(&mut i, "\"> \" { string.upper } input*").expect("runs");
        assert_eq!(shown(&i), ["A", "B", "C"]);
        assert_eq!(seen.borrow().prompts.len(), 4);

        // F108: what is not a lambda is refused only when a line arrives.
        let (mut i, _) = typed(&[], &[]);
        run(&mut i, "\"> \" 7 input*").expect("no line, no refusal");
        let (mut i, _) = typed(&["a"], &[]);
        let e = run(&mut i, "\"> \" 7 input*").expect_err("refused");
        assert!(e.ends_with("INPUT* returned error from LAMBDA: This is not a lambda"), "{e}");
    }

    /// `password` pushes the secret, asked with the message it was given, and
    /// with no terminal it is refused in its own words.
    #[test]
    fn password_pushes_the_secret_it_is_given() {
        let (mut i, seen) = typed(&[], &["hunter2"]);
        run(&mut i, "\"pw: \" password").expect("runs");
        assert_eq!(shown(&i), ["hunter2"]);
        assert_eq!(seen.borrow().prompts, ["pw: "]);

        let mut i = interp();
        let e = run(&mut i, "\"pw: \" password").expect_err("no terminal");
        assert!(
            e.ends_with("PASSWORD returns: there is no terminal to read a secret from"),
            "{e}"
        );
    }

    /// `debug.shell` evaluates each line against the live VM, remembers every
    /// one — an empty one included — and goes on past a line that fails.
    #[test]
    fn the_debug_shell_evaluates_what_is_typed() {
        let (mut i, seen) = typed(&["1 2 +", "nosuchword", "", "10 *"], &[]);
        run(&mut i, "debug.shell").expect("runs");
        assert_eq!(shown(&i), ["30"]);
        let seen = seen.borrow();
        let lines: Vec<&str> = seen.remembered.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(lines, ["1 2 +", "nosuchword", "", "10 *"]);
        assert!(seen.remembered.iter().all(|(h, _)| h == "bund_debug_shell_history.txt"));
    }

    /// `debug` applies one value of its snippet at a time, and after each
    /// evaluates lines until an empty one moves it on. The empty line is not
    /// remembered.
    #[test]
    fn the_snippet_debugger_reads_between_values() {
        // After `1`: push 5, then an empty line. After `2`: nothing more.
        let (mut i, seen) = typed(&["5", ""], &[]);
        run(&mut i, "\"1 2\" debug").expect("runs");
        assert_eq!(shown(&i), ["1", "5", "2"]);
        let seen = seen.borrow();
        assert_eq!(
            seen.remembered,
            [("bund_debug_debugger_history.txt".to_string(), "5".to_string())]
        );
    }

    /// **The default is no input**, which is what closes F158 by construction:
    /// an interpreter nobody gave an input meets the end at once.
    #[test]
    fn an_interpreter_given_no_input_never_waits() {
        let mut i = interp();
        run(&mut i, "\"> \" input \"> \" { drop } input* debug.shell \"1\" debug").expect("runs");
        assert_eq!(shown(&i), ["1"]);
    }

    #[test]
    fn the_host_table_names_bund2_crates() {
        let t = super::hostinfo_table(false);
        assert!(t.contains("bund2-stdlib version"), "{t}");
        assert!(t.contains("Kernel version"), "{t}");
    }

    /// RFC-0008 §D8: where a session's history goes, on every branch.
    ///
    /// The inputs are arguments rather than the environment, so each platform
    /// rule is asked directly and no test changes a variable its neighbours
    /// read. `XDG_CONFIG_HOME` wins when it is an absolute path and is ignored
    /// when it is not, which is what its specification says of a relative one.
    #[test]
    fn history_goes_to_the_platforms_config_directory() {
        use std::ffi::OsString;
        use std::path::PathBuf;
        let s = |x: &str| Some(OsString::from(x));

        assert_eq!(
            super::config_home(s("/x/cfg"), s("/home/u"), None),
            Some(PathBuf::from("/x/cfg")),
            "an absolute XDG_CONFIG_HOME wins"
        );
        let fallback = super::config_home(s("relative"), s("/home/u"), None);
        assert_eq!(
            fallback,
            super::config_home(None, s("/home/u"), None),
            "a relative XDG_CONFIG_HOME is invalid and ignored"
        );
        assert_eq!(
            super::config_home(s(""), s("/home/u"), None),
            fallback,
            "an empty one is unset"
        );
        if cfg!(windows) {
            assert_eq!(
                super::config_home(None, None, s("C:\\Users\\u\\AppData\\Roaming")),
                Some(PathBuf::from("C:\\Users\\u\\AppData\\Roaming"))
            );
        } else {
            let want = if cfg!(target_os = "macos") {
                "/home/u/Library/Application Support"
            } else {
                "/home/u/.config"
            };
            assert_eq!(fallback, Some(PathBuf::from(want)));
            // Nowhere to put it is not an error; there is simply no history.
            assert_eq!(super::config_home(None, None, None), None);
        }
    }

    /// A session that reads nothing writes nothing — which is what keeps a
    /// capture, where every read is end-of-input, from leaving a file behind,
    /// and a reference whose history lands in the *working directory* is why
    /// that matters (F10).
    #[test]
    fn a_session_that_remembers_nothing_saves_nothing() {
        let mut t = super::Session::open_with_history("never-written.txt").expect("opens");
        assert!(!t.dirty);
        t.save();
        assert!(!t.dirty, "nothing to save, nothing saved");
        t.remember("1 2 +");
        assert!(t.dirty, "a remembered line is owed a save");
        t.save();
        assert!(!t.dirty);
    }

    /// `debug`'s guards. An error ends a program, so the probe cannot hold
    /// these; the texts are the oracle's, measured 2026-10-05.
    ///
    /// The second message is the reference's unreachable one -- it guards the
    /// depth and then writes a different text for a pull that cannot fail
    /// after the guard. It is reproduced where the reference puts it.
    #[test]
    fn debug_guards_its_snippet() {
        for (src, want) in [
            ("debug", "Stack is too shallow for inline DEBUG"),
            (
                "42 debug",
                "Casting debug snippet returns: This Dynamic type is not string",
            ),
        ] {
            let mut i = interp();
            let e = run(&mut i, src).expect_err(src);
            assert!(e.ends_with(want), "{src}:\n  got {e}\n want ...{want}");
        }
    }

    /// `debug` applies each value it steps over, and stops at the `EXIT` that
    /// ends every compiled stream rather than applying it.
    ///
    /// Without the break the word leaves the `EXIT` on the stack; with it the
    /// stack holds the snippet's own answer and nothing else. Stdin is at
    /// end-of-file under `cargo test` as it is under capture, so the inner
    /// read returns at once.
    #[test]
    fn debug_applies_each_value_and_stops_at_exit() {
        let mut i = interp();
        run(&mut i, "\"1 2 +\" debug").expect("steps the snippet");
        assert_eq!(i.depth(), 1, "one value, not two: the EXIT was not applied");
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(3));
    }

    /// The two siblings no golden can hold: one prints a figure that moves,
    /// the other reports a line the oracle timestamps.
    ///
    /// `debug.display_distributed_info` requires `--distributed`, which Bund2
    /// has not got, so reporting that is its whole reachable behaviour -- the
    /// table it would otherwise draw needs a bus RFC-0007 governs.
    #[test]
    fn memstat_prints_and_distributed_info_reports_that_it_is_not_distributed() {
        let mut i = interp();
        run(&mut i, "debug.display_memstat").expect("prints a table");
        assert_eq!(i.depth(), 0, "it answers on stdout, not the stack");

        let t = super::memstat_table();
        assert!(t.contains("Current physical memory usage"), "{t}");
        assert!(t.contains("Current virtual memory usage"), "{t}");

        let mut i = interp();
        run(&mut i, "debug.display_distributed_info").expect("reports and continues");
        assert_eq!(i.depth(), 0);
    }
}
