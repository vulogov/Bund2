//! **§D1's safepoint: the debuggee stops on its own thread.**
//!
//! A debugger needs a program to *stop*, not to suspend. Suspension was never
//! available: `eval_lambda` records a floor, pushes a frame and then calls
//! `run_to(floor)` — a nested loop inside the native's own Rust frame — and
//! there are twenty-seven such call sites, several holding Rust state across
//! the body (`map_base` keeps a `Vec` and a loop position). Nothing can unwind
//! out of those and back in.
//!
//! **D84 is what makes stopping enough.** The debuggee runs on its own thread
//! and blocks at a safepoint. A thread blocked inside `map`'s Rust frame has
//! its state intact on its own stack, so step-into works by stopping *inside*
//! the nested loop rather than escaping it.
//!
//! **The debuggee inspects itself.** `Interp` holds `Box<dyn Reporter>` and,
//! through every value, `Rc<HeapValue>`: two independent reasons it is not
//! `Send` and a debugger thread may not touch it. So a safepoint receives a
//! [`Command`], runs it against the debuggee's own `Interp`, and hands back
//! **rendered text**. Only commands and text cross the boundary.
//!
//! That is the `Reporter` seam's shape (D36, D45), and it is what the reference
//! already does — `debug`'s readline loop hands each line to
//! `bund_compile_and_eval` in the same VM
//! (`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs`).
//!
//! **No transport appears here**, for the same reason none appears in
//! `Reporter`. [`Console`] is the seam; a CLI implements it over stdin, a host
//! thread over a channel pair, and a test over a script of commands.

use bund2_value::BundValue;

/// What the host asks of a stopped debuggee.
///
/// **Two kinds, and the difference is whether the program moves.** The
/// stepping commands set where to stop next and resume; the inspecting
/// commands answer and leave the debuggee stopped, so `bt` followed by
/// `stack` is two answers at one safepoint rather than two steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// One step, **into** whatever it enters — a word's body, a lambda, a
    /// `bund.eval`'d stream.
    Step,
    /// One step, **over** any body it enters.
    Next,
    /// Run until the frame that is current now has left.
    Finish,
    /// Run on, stopping for nothing this understands yet.
    Continue,
    /// Render the frame stack.
    Backtrace,
    /// Render the current stack.
    Stack,
}

/// Where the debuggee stops next. Private: the host says it in [`Command`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// At the very next step, whatever the depth.
    Step,
    /// At the next step no deeper than this — `next` over a body.
    Over(usize),
    /// When the depth has fallen below this — `finish` out of a frame.
    Out(usize),
    /// Nowhere.
    Run,
}

/// **The host side of the safepoint — the seam this RFC adds.**
///
/// `next_command` **blocks**: that is what stops the debuggee, and the reason
/// §D1 needs no suspension. Returning `None` means the host is gone, and the
/// debuggee runs to completion rather than hanging — a debugger that dies must
/// not take the program with it.
pub trait Console {
    /// **Called once per stop, before the first command.**
    ///
    /// A host that could not be told *where* the debuggee stopped would be
    /// unusable: `step` would advance an invisible cursor. The text is
    /// rendered by the debuggee, like every other answer, because `Interp` is
    /// not `Send` and the host may not read a value.
    ///
    /// Default is to ignore it, so a scripted test console says nothing.
    fn stopped(&mut self, _at: &str) {}
    /// Block until the host says what to do. `None` detaches.
    fn next_command(&mut self) -> Option<Command>;
    /// Rendered text for an inspecting command.
    fn answer(&mut self, text: &str);
}

/// What an attached debugger holds. One allocation, reached only when
/// something is attached.
pub struct Debug {
    console: Box<dyn Console>,
    mode: Mode,
    /// How many safepoints this run stopped at. The differential in
    /// `the_stepped_run_and_the_plain_run_agree` reads it to show the stepping
    /// happened rather than silently detaching.
    pub stops: usize,
}

impl Debug {
    pub fn new(console: Box<dyn Console>) -> Self {
        // **`Mode::Step`, so an attached debugger stops before the first
        // value.** Attaching and then missing the program's start would make
        // every breakpoint a race with the host's first command.
        Self {
            console,
            mode: Mode::Step,
            stops: 0,
        }
    }

    /// Whether this depth is where the host asked to stop.
    ///
    /// **`step`, `next` and `finish` are three comparisons of frame depth**,
    /// which is gdb's model: step stops anywhere, next stops no deeper than it
    /// started, finish stops only once the frame is gone.
    pub(crate) fn should_stop(&self, depth: usize) -> bool {
        match self.mode {
            Mode::Step => true,
            Mode::Over(at) => depth <= at,
            Mode::Out(at) => depth < at,
            Mode::Run => false,
        }
    }

    pub(crate) fn next_command(&mut self) -> Option<Command> {
        self.console.next_command()
    }

    pub(crate) fn answer(&mut self, text: &str) {
        self.console.answer(text);
    }

    pub(crate) fn stopped(&mut self, at: &str) {
        self.console.stopped(at);
    }

    /// **The host went away.** Run on rather than block forever.
    pub(crate) fn detach(&mut self) {
        self.mode = Mode::Run;
    }

    /// Apply one command.
    ///
    /// A stepping command says *where to stop next* and resumes; an inspecting
    /// command names a renderer and leaves the debuggee stopped. The caller
    /// owes the rendering, because only it can reach the `Interp`.
    pub(crate) fn apply(&mut self, cmd: Command, depth: usize) -> Next {
        match cmd {
            Command::Step => {
                self.mode = Mode::Step;
                Next::Resume
            }
            Command::Next => {
                self.mode = Mode::Over(depth);
                Next::Resume
            }
            Command::Finish => {
                // **`Out(0)` cannot fire**, because no depth is below zero, so
                // `finish` at the top level is `continue` — which is what gdb
                // does in the outermost frame and the honest reading of "run
                // until this frame has left" when there is no frame.
                self.mode = Mode::Out(depth);
                Next::Resume
            }
            Command::Continue => {
                self.mode = Mode::Run;
                Next::Resume
            }
            Command::Backtrace => Next::Render(Render::Backtrace),
            Command::Stack => Next::Render(Render::Stack),
        }
    }
}

/// What the caller does after [`Debug::apply`].
pub(crate) enum Next {
    /// Leave the safepoint and run.
    Resume,
    /// Render this, answer with it, and stay stopped.
    Render(Render),
}

/// Which renderer an inspecting command wants. The debuggee owns both,
/// because rendering reads values.
pub(crate) enum Render {
    Backtrace,
    Stack,
}

/// One line of a backtrace: what a frame is running and how far through.
///
/// **§D2's field is what makes this a name.** A `Frame` carries a body, an
/// `ip` and an exit action — enough to *resume* a body and not enough to *say
/// what it is* — so before the symbol landed every line read `a lambda at 3`.
/// With it, a frame pushed for a word reads `#1  w at 3`.
///
/// **A body no word names still reads the old way, and that is not a
/// shortfall.** A `( … )` scratch context, a LIST assembled at run time and a
/// lambda a native handed to `eval_lambda` belong to no symbol — D16's
/// consequence — so the line says what the body *is* rather than inventing a
/// caller for it.
///
/// **There is still no source position**, and §D2 says why: `lower_with_spans`
/// is top level only, so no value inside a body carries one, and producing
/// them is the IR RFC-0003 §S5 describes and that does not exist. A backtrace
/// gives a word and a value index today.
pub fn frame_line(body: &BundValue, ip: usize, depth: usize, who: Option<&str>) -> String {
    match who {
        Some(name) => format!("#{depth}  {name} at {ip}"),
        None => {
            let what = match body.dt() {
                bund2_value::LAMBDA => "a lambda",
                bund2_value::LIST => "a body assembled at run time",
                _ => "a body",
            };
            format!("#{depth}  {what} at {ip}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Interp;
    use bund2_api::{StackEffect, Symbol, Vm, WordKind};

    /// A host that reads from a script and records what it was told.
    ///
    /// Running out of commands returns `None`, which is the detach path — so a
    /// test that under-supplies commands finishes the program instead of
    /// hanging, and a test that means to check detaching says so by name.
    struct Scripted {
        script: Vec<Command>,
        at: usize,
        answers: Vec<String>,
    }

    impl Scripted {
        fn new(script: Vec<Command>) -> Self {
            Self {
                script,
                at: 0,
                answers: Vec::new(),
            }
        }
    }

    impl Console for Scripted {
        fn next_command(&mut self) -> Option<Command> {
            let c = self.script.get(self.at).cloned();
            self.at += 1;
            c
        }
        fn answer(&mut self, text: &str) {
            self.answers.push(text.to_string());
        }
    }

    /// A host that answers one command forever, counting how often it is
    /// asked. Shared so the test can read the count after the run.
    struct Always(Command, std::rc::Rc<std::cell::Cell<usize>>);

    impl Console for Always {
        fn next_command(&mut self) -> Option<Command> {
            self.1.set(self.1.get() + 1);
            Some(self.0.clone())
        }
        fn answer(&mut self, _: &str) {}
    }

    /// A native that runs a lambda **through the loop**, so the body gets a
    /// frame and the program has a depth for `next` and `finish` to compare
    /// against. This crate has no stdlib and no parser, so the programs below
    /// are hand-built streams, as every other test here is.
    fn runs_a_body(vm: &mut dyn Vm) -> Result<(), bund2_api::Error> {
        let body = BundValue::lambda(vec![BundValue::int(8), BundValue::int(9)]);
        vm.tail_lambda(body);
        Ok(())
    }

    fn with_body_word() -> (Interp, Symbol) {
        let mut i = Interp::new();
        let s = i.registry.register_native(
            "runbody",
            runs_a_body,
            StackEffect::opaque(0),
            WordKind::Sync,
        );
        (i, s)
    }

    /// `1 2 runbody` — two top-level literals, then a word whose body pushes
    /// two more from inside a frame.
    fn program(_s: Symbol) -> Vec<BundValue> {
        vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("runbody"),
        ]
    }

    fn final_state(i: &Interp) -> Vec<String> {
        i.snapshot().iter().map(|v| v.summary(72)).collect()
    }

    /// **Criterion 6, in the small.** A run stepped to completion leaves the
    /// same final state as a run that was never stopped — and the stop count
    /// proves the stepping happened, because a run that silently detached
    /// would agree for the wrong reason.
    #[test]
    fn a_stepped_run_and_a_plain_run_agree() {
        let (mut plain_vm, s) = with_body_word();
        plain_vm.eval(&program(s)).expect("runs");
        let want = final_state(&plain_vm);
        assert_eq!(want.len(), 4, "two literals and two from the body");

        let count = std::rc::Rc::new(std::cell::Cell::new(0));
        let (mut i, s) = with_body_word();
        i.attach_debugger(Box::new(Always(Command::Step, count.clone())));
        i.eval(&program(s)).expect("runs");

        assert_eq!(final_state(&i), want, "stepping changed the program");
        let stops = i.debugger_stops().expect("still attached");
        assert!(stops > 3, "the body's steps were not stopped at: {stops}");
        assert_eq!(
            stops,
            count.get(),
            "every stop asked for exactly one command"
        );
    }

    /// **`next` steps over a body and `step` steps into it**, which is the
    /// only observable difference between them — and it is a stop count, since
    /// neither changes the program.
    #[test]
    fn next_steps_over_the_body_that_step_steps_into() {
        let into = std::rc::Rc::new(std::cell::Cell::new(0));
        let (mut i, s) = with_body_word();
        i.attach_debugger(Box::new(Always(Command::Step, into.clone())));
        i.eval(&program(s)).expect("runs");
        let stepped = i.debugger_stops().expect("attached");

        let over = std::rc::Rc::new(std::cell::Cell::new(0));
        let (mut j, s) = with_body_word();
        j.attach_debugger(Box::new(Always(Command::Next, over.clone())));
        j.eval(&program(s)).expect("runs");
        let nexted = j.debugger_stops().expect("attached");

        assert_eq!(final_state(&i), final_state(&j), "neither changes the program");
        assert!(
            nexted < stepped,
            "next stopped as often as step: {nexted} vs {stepped}"
        );
    }

    /// **`continue` stops once and then never again.** The control for the
    /// counts above: whatever `step` costs, `continue` costs one.
    #[test]
    fn continue_stops_once() {
        let count = std::rc::Rc::new(std::cell::Cell::new(0));
        let (mut i, s) = with_body_word();
        i.attach_debugger(Box::new(Always(Command::Continue, count.clone())));
        i.eval(&program(s)).expect("runs");
        assert_eq!(i.debugger_stops(), Some(1));
    }

    /// **An inspecting command answers and does not advance.** So `bt` then
    /// `stack` then `continue` is two answers at one safepoint — the property
    /// that makes a debugger usable, and the reason `apply` distinguishes the
    /// two kinds of command rather than resuming after every one.
    #[test]
    fn inspecting_does_not_step() {
        let (mut i, s) = with_body_word();
        i.attach_debugger(Box::new(Scripted::new(vec![
            Command::Backtrace,
            Command::Stack,
            Command::Continue,
        ])));
        i.eval(&program(s)).expect("runs");
        assert_eq!(i.debugger_stops(), Some(1), "one safepoint, three commands");
    }

    /// **The host going away does not take the program with it.** A console
    /// that returns `None` detaches, and the program finishes — the
    /// alternative at that line is a block that never returns.
    #[test]
    fn a_host_that_leaves_lets_the_program_finish() {
        let (mut plain_vm, s) = with_body_word();
        plain_vm.eval(&program(s)).expect("runs");
        let want = final_state(&plain_vm);

        let (mut i, s) = with_body_word();
        i.attach_debugger(Box::new(Scripted::new(Vec::new())));
        i.eval(&program(s)).expect("runs");
        assert_eq!(final_state(&i), want);
        assert_eq!(i.debugger_stops(), Some(1), "stopped once, then detached");
    }

    /// **The three modes are three depth comparisons**, asserted directly
    /// rather than through a program, because that is the whole of `step`,
    /// `next` and `finish`.
    #[test]
    fn the_modes_are_depth_comparisons() {
        let mut d = Debug::new(Box::new(Scripted::new(Vec::new())));
        assert!(d.should_stop(0), "step stops anywhere");
        assert!(d.should_stop(9));

        // `next` at depth 2: not deeper, so depth 3 runs through and 2 stops.
        assert!(matches!(d.apply(Command::Next, 2), Next::Resume));
        assert!(!d.should_stop(3), "next does not stop inside a body");
        assert!(d.should_stop(2), "next stops where it started");
        assert!(d.should_stop(1), "and on the way out");

        // `finish` at depth 2: only once the frame is gone.
        assert!(matches!(d.apply(Command::Finish, 2), Next::Resume));
        assert!(!d.should_stop(2), "finish does not stop in the frame it left");
        assert!(d.should_stop(1), "it stops once the frame has gone");

        assert!(matches!(d.apply(Command::Continue, 0), Next::Resume));
        assert!(!d.should_stop(0), "continue stops nowhere");
    }

    /// **`finish` at the top level is `continue`**, because no depth is below
    /// zero. Stated as a test so the reading is not left to a reader of
    /// `Mode::Out(0)`.
    #[test]
    fn finish_at_the_top_level_runs_on() {
        let mut d = Debug::new(Box::new(Scripted::new(Vec::new())));
        assert!(matches!(d.apply(Command::Finish, 0), Next::Resume));
        for depth in 0..4 {
            assert!(!d.should_stop(depth), "nothing is below zero");
        }
    }

    /// **§D2's field, from the rendering side.** A frame pushed for a word
    /// names it; one pushed for a body no word owns says what the body is
    /// rather than inventing a caller.
    ///
    /// This test is the one §D1 pinned deliberately so that this commit had to
    /// change it.
    #[test]
    fn a_frame_line_names_the_word_when_a_word_named_it() {
        let lam = BundValue::lambda(vec![]);
        assert_eq!(frame_line(&lam, 3, 0, Some("w")), "#0  w at 3");
        assert_eq!(frame_line(&lam, 3, 0, None), "#0  a lambda at 3");
        let list = BundValue::list(vec![]);
        assert_eq!(
            frame_line(&list, 0, 2, None),
            "#2  a body assembled at run time at 0"
        );
    }

    /// **A backtrace over a real frame stack names the word that was called.**
    /// The rendering test above cannot show that the symbol *arrives*; this
    /// runs a word whose body stops at a safepoint and reads the answer.
    #[test]
    fn a_backtrace_taken_inside_a_word_names_it() {
        /// Asks for a backtrace at every safepoint, keeping the answers.
        struct Nosy(std::rc::Rc<std::cell::RefCell<Vec<String>>>, bool);

        impl Console for Nosy {
            fn next_command(&mut self) -> Option<Command> {
                // One backtrace, then move on — otherwise this never resumes.
                if self.1 {
                    self.1 = false;
                    Some(Command::Backtrace)
                } else {
                    self.1 = true;
                    Some(Command::Step)
                }
            }
            fn answer(&mut self, text: &str) {
                self.0.borrow_mut().push(text.to_string());
            }
        }

        // **A word registered as a lambda**, which is the path that carries a
        // symbol: `dispatch`'s `Resolved::Lambda` arm resolves the name and
        // files the request *for* it. The native in `with_body_word` goes
        // through `tail_lambda`, where there is no symbol to carry — a native
        // building a body is not a word calling one, and the `None` arm of
        // `frame_line` is for exactly that.
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut i = Interp::new();
        i.registry.register_lambda(
            "named",
            BundValue::lambda(vec![BundValue::int(8), BundValue::int(9)]),
        );
        i.attach_debugger(Box::new(Nosy(seen.clone(), true)));
        i.eval(&[BundValue::int(1), BundValue::call("named")])
            .expect("runs");

        let all = seen.borrow().join("");
        assert!(
            all.contains("#0  named at"),
            "no frame named the word that was called:\n{all}"
        );
        assert!(
            all.contains("#0  the top-level stream"),
            "the top level must say what it is:\n{all}"
        );
    }

    /// **A body no word named still says what it is.** The native path files a
    /// request with no symbol, which is correct rather than a gap: a lambda a
    /// native built belongs to nobody, and a backtrace that guessed a caller
    /// would be worse than one that declines.
    #[test]
    fn a_body_no_word_named_is_not_given_one() {
        struct Nosy(std::rc::Rc<std::cell::RefCell<Vec<String>>>, bool);
        impl Console for Nosy {
            fn next_command(&mut self) -> Option<Command> {
                self.1 = !self.1;
                if self.1 {
                    Some(Command::Backtrace)
                } else {
                    Some(Command::Step)
                }
            }
            fn answer(&mut self, text: &str) {
                self.0.borrow_mut().push(text.to_string());
            }
        }

        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let (mut i, s) = with_body_word();
        i.attach_debugger(Box::new(Nosy(seen.clone(), false)));
        i.eval(&program(s)).expect("runs");

        let all = seen.borrow().join("");
        assert!(all.contains("#0  a lambda at"), "{all}");
        assert!(
            !all.contains("runbody"),
            "the native's own name is not the body's: {all}"
        );
    }
}
