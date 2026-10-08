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

/// **Measured for Q41: a breakpoint on a native does not fire.** §D3 breaks
/// when a frame is pushed and a native pushes none, so `break input` arms
/// something that is never reached. The way to feed a read at the moment it
/// happens is therefore to step to it; this pins the answer so a change to
/// it is noticed.
#[test]
fn a_breakpoint_on_a_native_is_never_reached() {
    let src = "\"name? \" input println\n";
    let (_, err, code) = session(src, Some("break input\nc\n"));
    assert_eq!(err.matches("breakpoint: input").count(), 0, "{err}");
    assert_eq!(code, Some(0));
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
