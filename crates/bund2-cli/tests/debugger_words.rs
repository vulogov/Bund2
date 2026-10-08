//! **RFC-0008's amendment "debugger words", Part A, at process level.**
//!
//! The claims are about what a person sees at a console and what a program
//! prints, so this spawns the binary, as `debugger_step.rs` does and for its
//! reason. Criteria W1 to W6; W4 is `debugger_step.rs`'s own sweep, which
//! passes unchanged.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

fn scratch(stem: &str) -> std::path::PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("bund2-dbw-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir.join(format!("{stem}.bund"))
}

/// Run a program, under the debugger driven by `script` if one is given, and
/// return (stdout, stderr, exit code). Standard input is closed after the
/// script, which detaches and runs on; a plain run is given none (F158).
fn session(src: &str, script: Option<&str>) -> (String, String, Option<i32>) {
    let path = scratch("w");
    std::fs::write(&path, src).expect("script");
    let mut c = Command::new(env!("CARGO_BIN_EXE_bund2"));
    if script.is_some() {
        c.arg("--debugger");
    }
    c.args(["script", "--file"]).arg(&path);
    c.stdout(Stdio::piped()).stderr(Stdio::piped());
    c.stdin(if script.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut ch = c.spawn().expect("bund2 runs");
    if let Some(script) = script {
        let mut pipe = ch.stdin.take().expect("stdin");
        let _ = pipe.write_all(script.as_bytes());
        drop(pipe);
    }
    let out = ch.wait_with_output().expect("waits");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// **W1: one view, one text, wherever the word was typed** — in the script,
/// in `debug.shell`, and at a console stop, each with `1 2 3` on the stack.
#[test]
fn a_view_is_the_same_text_in_a_script_a_shell_and_a_console() {
    let want = "* @main  3  top: 3\n  workbench  0\n";

    let (out, _, code) = session("1 2 3 debug.stacks\n", None);
    assert_eq!(out, want, "in a script");
    assert_eq!(code, Some(0));

    let (out, _, _) = session("1 2 3 \"debug.stacks\" debug.feed debug.shell\n", None);
    assert_eq!(out, want, "in debug.shell");

    // Three steps run `1 2 3`; the stop is before the fourth value.
    let (out, _, _) = session("1 2 3 \"x\" drop\n", Some("s\ns\ns\ndebug.stacks\nc\n"));
    assert_eq!(out, want, "at a console stop");
}

/// A backtrace asked from inside a word names the word, and `debug.info`
/// answers for the session it is in.
#[test]
fn the_views_see_the_run_they_are_asked_from() {
    let (out, _, _) = session(":w { debug.backtrace } register\nw\n", None);
    assert!(out.starts_with("#0  w at "), "the frame is the word's:\n{out}");

    let (out, _, _) = session("debug.info\n", None);
    assert_eq!(out, "no debugger is attached\n");

    // Typed at a stop, it reads what the console armed.
    let (out, _, _) = session("1 2\n", Some("break w\ndebug.info\nc\n"));
    assert_eq!(out, "break w\n", "the word sees what `break` armed");
}

/// **W2: a typed line runs in the program's own VM and is not itself
/// stepped.** The sum is on the program's stack afterwards, and the session
/// reported one stop — the one the line was typed at.
#[test]
fn a_typed_line_runs_in_the_program_and_is_not_stepped() {
    let (out, err, code) = session("println\n", Some("10 20 +\nst\nc\n"));
    assert!(err.contains("@main\n  30\n"), "the line's effect is on the stack:\n{err}");
    assert_eq!(err.matches("next:").count(), 1, "the line's values were stops:\n{err}");
    assert_eq!(out, "30\n", "and the program ran on with it");
    assert_eq!(code, Some(0));
}

/// **W3: a line may be typed at each of the four places a session stops** —
/// the top-level stream, the head of the loop inside a body, a breakpoint
/// before its frame, and both watch hooks, which stop inside the push of the
/// native that is pushing — and the program then runs to its end.
#[test]
fn a_line_may_be_typed_at_every_kind_of_stop() {
    let src = ":w { 7 8 + } register\nw println\n\"end\" println\n";
    let script = "\"top\" println\nbreak w\nc\n\"bp\" println\ns\n\"head\" println\nc\n";
    let (out, err, code) = session(src, Some(script));
    assert!(err.contains("breakpoint: w"), "{err}");
    assert_eq!(out, "top\nbp\nhead\n15\nend\n");
    assert_eq!(code, Some(0));

    // A watch on a named stack: the stop is inside the push of `3`.
    let src = "@other 3 @main\n\"end\" println\n";
    let (out, err, code) = session(src, Some("watch @other\nc\n\"named\" println\n77\nc\n"));
    assert!(err.contains("watchpoint: @other"), "{err}");
    assert_eq!(out, "named\nend\n");
    assert_eq!(code, Some(0));

    // A watch on the workbench: the second hook.
    let src = "1 { true } ?. \n\"end\" println\n";
    let (out, err, code) = session(src, Some("watch workbench\nc\n\"bench\" println\nc\nc\n"));
    assert!(err.contains("watchpoint: workbench"), "{err}");
    assert_eq!(out, "bench\nend\n");
    assert_eq!(code, Some(0));
}

/// A line pushed at a watch stop lands *under* the value whose push was the
/// stop, because that push has not happened yet. Stated as a test so the
/// order is a fact and not a surprise.
#[test]
fn a_line_typed_at_a_watch_stop_runs_before_the_watched_push() {
    let src = "@other 3 debug.display_stack\n";
    let (out, _, _) = session(src, Some("watch @other\nc\n77\nc\n"));
    let (a, b) = (out.find("I64(77)"), out.find("I64(3)"));
    assert!(a.is_some() && b.is_some(), "both values are on the stack:\n{out}");
}

/// **W5, the console half: a fed line reaches a debugged program's read**,
/// and without one the read is the end of input, as D112 made it.
#[test]
fn a_fed_line_is_how_a_debugged_program_is_typed_into() {
    let src = "\"name? \" input debug.display_stack\n\"end\" println\n";
    let (out, _, code) = session(src, Some("\"alice\" debug.feed\nc\n"));
    assert!(out.contains("alice"), "the read was answered:\n{out}");
    assert!(out.ends_with("end\n"), "{out}");
    assert_eq!(code, Some(0));

    // Nothing fed: the read is the end of input and pushes nothing.
    let (out, _, code) = session(src, Some("c\n"));
    assert!(!out.contains("alice") && out.ends_with("end\n"), "{out}");
    assert_eq!(code, Some(0));
}

/// **Q41's second half, closed: a breakpoint on a native stops before it
/// runs.** §D3 breaks when a frame is pushed and a native pushes none, so
/// `break input` once armed something never reached. The stop is at the call:
/// the operand is still on the stack, a line fed there is the line the read
/// gets, and a line that exits there keeps the native from running.
#[test]
fn a_breakpoint_on_a_native_stops_before_it_runs() {
    let src = "\"name? \" input println\n\"end\" println\n";
    let (out, err, code) = session(src, Some("break input\nc\nst\n\"alice\" debug.feed\nc\n"));
    assert_eq!(err.matches("breakpoint: input").count(), 1, "{err}");
    assert!(err.contains("name? "), "the prompt is still an operand:\n{err}");
    assert_eq!(out, "alice\nend\n", "the read took the line fed at its own stop");
    assert_eq!(code, Some(0));

    // Every call is a stop. An alias stops under the name it was armed by,
    // and the word it resolves to under its own: `dup` and `stack` are
    // aliases, `ensure_stack` is what `stack` resolves to.
    let src = "1 dup dup drop drop drop \"x\" stack\n";
    let (_, err, _) = session(src, Some("break dup\nbreak ensure_stack\nc\nc\nc\nc\n"));
    assert_eq!(err.matches("breakpoint: dup").count(), 2, "{err}");
    assert_eq!(err.matches("breakpoint: ensure_stack").count(), 1, "{err}");

    // The same for an alias of a lambda, which was as unreachable.
    let src = ":w { 1 drop } register \"w\" \"v\" alias\nv w\n";
    let (_, err, _) = session(src, Some("s\ns\ns\ns\ns\ns\nbreak v\nc\nc\n"));
    assert_eq!(err.matches("breakpoint: v").count(), 1, "{err}");

    // A condition is shown the operands, as a lambda's is.
    let src = "1 println 2 println 3 println\n";
    let (out, err, _) = session(src, Some("break println if { 2 == }\nc\nc\n"));
    assert_eq!(err.matches("breakpoint: println").count(), 1, "{err}");
    assert_eq!(out, "1\n2\n3\n");

    // A line that exits at the stop: the native does not run.
    let src = "\"shown\" println\n";
    let (out, _, code) = session(src, Some("break println\nc\n7 bund.exit\n"));
    assert_eq!(out, "", "the native ran after the program was told to exit");
    assert_eq!(code, Some(7));

    // Words in a line typed at a stop are not stops.
    let src = "\"end\" println\n";
    let (out, err, _) = session(src, Some("break println\n\"typed\" println\nc\nc\n"));
    assert_eq!(err.matches("breakpoint: println").count(), 1, "{err}");
    assert_eq!(out, "typed\nend\n");
}

/// **W6: a line that fails is reported and the session stays where it was;
/// a line that exits ends the program with its code.**
#[test]
fn a_failing_line_stays_stopped_and_an_exiting_line_ends_the_program() {
    let src = "\"end\" println\n";
    let (out, err, code) = session(src, Some("stpe\nst\nc\n"));
    assert!(err.contains("bund2: stpe not registered"), "the failure is said:\n{err}");
    assert!(!err.contains("Attempt to evaluate"), "in the word's own words:\n{err}");
    assert_eq!(err.matches("next:").count(), 1, "the typo moved the program:\n{err}");
    assert!(err.contains("@main"), "and the next command was answered:\n{err}");
    assert_eq!(out, "end\n");
    assert_eq!(code, Some(0));

    let (out, _, code) = session(src, Some("7 bund.exit\nst\n"));
    assert_eq!(out, "", "nothing more ran");
    assert_eq!(code, Some(7));
}

/// `stack` at the console is the Bund word and `st` is the session's view
/// (D113.4).
#[test]
fn stack_is_the_bund_word_and_st_is_the_view() {
    // `stack` is `ensure_stack`: it makes the stack it names and moves to it,
    // so the session's own view then shows that stack.
    let (out, err, code) = session("1\n", Some("\"side\" stack\ndebug.stacks\nst\nc\n"));
    assert!(out.contains("* @side"), "`stack` ran as the Bund word:\n{out}");
    assert!(err.contains("@side\n"), "`st` renders the current stack:\n{err}");
    assert_eq!(code, Some(0));
}

// ---------------------------------------------------------------------------
// Part B: arming and moving from a word (RFC-0008 §W5).
// ---------------------------------------------------------------------------

/// Run a program with **no** `--debugger`, its standard input these lines:
/// the console a word attaches reads through the program's own input.
fn scripted(src: &str, typed: &str, flags: &[&str]) -> (String, String, Option<i32>) {
    let path = scratch("b");
    std::fs::write(&path, src).expect("script");
    let mut ch = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(flags)
        .args(["script", "--file"])
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bund2 runs");
    let mut pipe = ch.stdin.take().expect("stdin");
    let _ = pipe.write_all(typed.as_bytes());
    drop(pipe);
    let out = ch.wait_with_output().expect("waits");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// **D113.5: a script arms a breakpoint with no `--debugger`, and the stop
/// is a console on the program's own input.** Nothing stops until the word
/// is called; the stop is before its body; a line typed there is Bund.
#[test]
fn a_script_arms_a_breakpoint_and_is_stopped_at_it() {
    let src = ":w { 7 8 + } register\n\"w\" debug.break\n\"before\" println\nw println\n\"end\" println\n";
    let (out, err, code) = scripted(src, "\"at\" println\nc\n", &[]);
    assert_eq!(err.matches("breakpoint: w").count(), 1, "{err}");
    assert_eq!(out, "before\nat\n15\nend\n", "armed quietly, stopped before the body");
    assert_eq!(code, Some(0));
}

/// `debug.step` in a script is where the program hands itself over: the
/// stop is before the next value, and `s` then walks on from there.
#[test]
fn debug_step_in_a_script_stops_before_the_next_value() {
    let src = "\"a\" println\ndebug.step\n\"b\" println\n\"end\" println\n";
    let (out, err, code) = scripted(src, "st\ns\ns\nc\n", &[]);
    assert!(err.contains("next: \"b\""), "the stop names what is next:\n{err}");
    assert_eq!(err.matches("next:").count(), 3, "one stop, then two steps:\n{err}");
    assert_eq!(out, "a\nb\nend\n");
    assert_eq!(code, Some(0));
}

/// **With no input the console detaches, and for good.** Under a capture a
/// script that arms and steps runs to its end, and says where it stopped
/// once rather than at every hit.
#[test]
fn a_script_that_arms_runs_on_when_nobody_is_there() {
    let src = ":w { 1 drop } register\n\"w\" debug.break\nw w w\n\"end\" println\n";
    let (out, err, code) = scripted(src, "", &[]);
    assert_eq!(out, "end\n");
    assert_eq!(err.matches("breakpoint: w").count(), 1, "said once:\n{err}");
    assert_eq!(code, Some(0));
}

/// **W7: what a word arms and what the console arms are one state.**
/// `debug.info` lists both, and either may be deleted from either side.
#[test]
fn a_word_and_the_console_arm_the_same_debugger() {
    let src = "\"w\" debug.break \"@errors\" debug.watch debug.watch.workbench\n\
               debug.step\n\"end\" println\n";
    let typed = "break v\ndebug.info\n\"w\" debug.delete\ndelete v\n\
                 \"workbench\" debug.delete\ndebug.info\nc\n";
    let (out, _, code) = scripted(src, typed, &[]);
    assert_eq!(
        out,
        "break v\nbreak w\nwatch @errors\nwatch workbench\nwatch @errors\nend\n",
        "both armings, then what is left"
    );
    assert_eq!(code, Some(0));
}

/// The moving words typed at a stop do what the short forms do, and
/// `debug.finish` and `debug.next` in a script stop where they say.
#[test]
fn the_moving_words_move() {
    // `debug.continue` as a line resumes, as `c` does.
    let src = "debug.step\n\"a\" println\n\"b\" println\n";
    let (out, err, _) = scripted(src, "debug.continue\n", &[]);
    assert_eq!(err.matches("next:").count(), 1, "{err}");
    assert_eq!(out, "a\nb\n");

    // `debug.step` as a line is one step, as `s` is.
    let (_, err, _) = scripted(src, "debug.step\ndebug.step\nc\n", &[]);
    assert_eq!(err.matches("next:").count(), 3, "{err}");

    // `debug.finish` inside a body stops once the body has returned.
    let src = ":w { debug.finish 1 drop 2 drop } register\nw\n\"after\" println\n";
    let (out, err, _) = scripted(src, "c\n", &[]);
    assert_eq!(err.matches("next:").count(), 1, "{err}");
    assert!(err.contains("next: \"after\""), "the stop is outside the body:\n{err}");
    assert_eq!(out, "after\n");

    // `debug.next` inside a body stops at the body's next value.
    let src = ":w { debug.next 41 drop } register\nw\n";
    let (_, err, _) = scripted(src, "c\n", &[]);
    assert!(err.contains("w at 1  next: 41"), "the stop is in the body:\n{err}");
}

/// **D113.5's ruling: `debug.run` evaluates a string stopped at its first
/// term, and the string's own terms are stops.** `debug.step` before
/// `bund.eval` stops only inside the words the string calls, which is the gap
/// the word exists to close; that half is asserted too, so the difference
/// stays measured.
#[test]
fn debug_run_steps_the_terms_of_its_string() {
    let src = ":w { 7 + } register\n\"1 2 w println\" debug.run\n\"end\" println\n";

    // `s` walks the four terms and goes into `w`'s body.
    let (out, err, code) = scripted(src, "s\ns\ns\ns\ns\ns\nc\n", &[]);
    for (i, next) in [(0, "1"), (1, "2"), (2, "w"), (3, "println")] {
        assert!(
            err.contains(&format!("debug.run's string at {i}  next: {next}")),
            "term {i}:\n{err}"
        );
    }
    assert!(err.contains("w at 0  next: 7"), "into the body:\n{err}");
    assert_eq!(out, "9\nend\n");
    assert_eq!(code, Some(0));

    // `n` walks the four terms and stays out of it.
    let (out, err, _) = scripted(src, "n\nn\nn\nc\n", &[]);
    assert_eq!(err.matches("debug.run's string at").count(), 4, "{err}");
    assert!(!err.contains("w at 0"), "over the body:\n{err}");
    assert_eq!(out, "9\nend\n");

    // `c` at the first term runs the rest, and nobody there runs all of it.
    for typed in ["c\n", ""] {
        let (out, err, _) = scripted(src, typed, &[]);
        assert_eq!(err.matches("debug.run's string at").count(), 1, "{err}");
        assert_eq!(out, "9\nend\n");
    }

    // The gap: the same string through `bund.eval` has no stop at a term.
    let src = ":w { 7 + } register\ndebug.step \"1 2 w println\" bund.eval\n";
    let (_, err, _) = scripted(src, "s\ns\ns\ns\ns\nc\n", &[]);
    assert!(err.contains("w at 0  next: 7"), "{err}");
    assert!(!err.contains("next: 2"), "a term of the string was a stop:\n{err}");
}

/// Inside a word the stop names the string's position and not the frame's,
/// and a failing term is the failure of the word.
#[test]
fn debug_run_inside_a_word_and_on_a_failing_term() {
    let src = ":v { \"1 println\" debug.run } register\nv\n";
    let (out, err, _) = scripted(src, "bt\nc\n", &[]);
    assert!(err.contains("debug.run's string at 0  next: 1"), "{err}");
    assert!(err.contains("#0  v at"), "the frame is still in the backtrace:\n{err}");
    assert_eq!(out, "1\n");

    let (out, err, _) = scripted("\"1 nosuch\" debug.run \"after\" println\n", "c\n", &[]);
    assert!(out.contains("nosuch not registered"), "{out}\n{err}");
    assert!(!out.contains("after\n"), "the program stopped at the failure:\n{out}");
}

/// A condition given by a word is the console's condition: Bund source, run
/// in a child VM, stopping when it holds and not when it does not.
#[test]
fn a_word_arms_a_conditional_breakpoint() {
    let src = ":w { 1 drop } register\n\"w\" \"{ 1 1 == }\" debug.break.if\nw\n\"end\" println\n";
    let (_, err, _) = scripted(src, "c\n", &[]);
    assert_eq!(err.matches("breakpoint: w").count(), 1, "{err}");

    let src = ":w { 1 drop } register\n\"w\" \"{ 1 2 == }\" debug.break.if\nw\n\"end\" println\n";
    let (out, err, _) = scripted(src, "c\n", &[]);
    assert_eq!(err.matches("breakpoint: w").count(), 0, "{err}");
    assert_eq!(out, "end\n");
}

/// **A condition runs under the program's restrictions — F180.** It is
/// evaluated in a child VM, which was built with default options: under
/// `--noio --noeval` this condition printed the working directory and reached
/// `bund.eval`. Each flag is tried alone, so neither can stand in for the
/// other.
#[test]
fn a_condition_is_no_freer_than_the_program_that_armed_it() {
    for (flag, cond, refusal) in [
        ("--noio", "fs.cwd println true", "disabled with --noio"),
        ("--noeval", ":q bund.eval true", "disabled with --noeval"),
    ] {
        let path = scratch("restricted");
        let src = format!(
            ":w {{ 1 drop }} register\n\"w\" \"{cond}\" debug.break.if\nw\n\"end\" println\n"
        );
        std::fs::write(&path, src).expect("script");
        let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
            .args(["script", flag, "--file"])
            .arg(&path)
            .stdin(Stdio::null())
            .output()
            .expect("bund2 runs");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(stdout, "end\n", "{flag}: the condition printed nothing\n{stderr}");
        assert!(stderr.contains(refusal), "{flag}: and was refused by the stub:\n{stderr}");
    }
}

/// **W8, D113.6: a breakpoint armed while a tier is installed still fires.**
/// `w` is called until it is compiled, then armed. Offered to the tier first,
/// as every body is, a compiled `w` would run whole before the breakpoint
/// was looked at; arming turns the tier off for every body from then on.
///
/// Both builds are asserted, as `exit_tier.rs` does: the stop is checked
/// always, and that a body really was compiled only where there is a tier.
#[test]
fn a_breakpoint_armed_beside_a_tier_still_fires() {
    let src = ":w { 1 2 + drop } register\nw w w w w w\n\"w\" debug.break\nw\n\"end\" println\n";
    let (out, err, code) = scripted(src, "c\n", &["--stats", "--jit-threshold", "1"]);
    assert_eq!(err.matches("breakpoint: w").count(), 1, "the stop was missed:\n{err}");
    assert_eq!(out, "end\n");
    assert_eq!(code, Some(0));
    if err.contains("no tier") {
        eprintln!(
            "debugger_words: no tier in this build, so W8 checked the stop and not \
             that a compiled body was bypassed. Run with `--features jit`."
        );
    } else {
        assert!(
            !err.contains("compiled 0 "),
            "the precondition failed: nothing was compiled, so the tier was never beside it:\n{err}"
        );
    }
}
