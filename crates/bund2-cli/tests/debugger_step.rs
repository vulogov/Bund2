//! **RFC-0008 criterion 6 at process level: a stepped run equals a plain one.**
//!
//! "step, next and finish agree with an uninterrupted run" cannot be shown
//! in process: the claim is about a *program's* output and final state, and
//! nothing in process captures stdout. So this spawns the binary, as
//! `exit_tier.rs` and `input_exit.rs` do for the same reason.
//!
//! **Stepping is driven to the end, not for a fixed count.** A writer thread
//! keeps feeding the command until the child exits, so every safepoint is
//! stepped rather than the first N — a finite script would hit EOF, detach, and
//! let the rest run uninterrupted, which is the thing being tested passing by
//! not happening.
//!
//! **Only stdout is compared, which is why the session writes to stderr.**
//! Prompts and position lines are not the program's output.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// **A unique script path per call.** Keying on the process id alone put every
/// test in this file on one file: they run as threads of one process, in
/// parallel, so each overwrote the others' program and a watchpoint test read
/// a breakpoint test's source. Found exactly that way.
fn scratch(stem: &str) -> std::path::PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("bund2-dbg-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir.join(format!("{stem}.bund"))
}

/// Run a program, optionally under the debugger driven by `cmd`, and return
/// its normalised stdout and exit code.
fn run(src: &str, under: Option<&'static str>) -> (String, Option<i32>) {
    let script = scratch("p");
    std::fs::write(&script, src).expect("script");

    let mut c = Command::new(env!("CARGO_BIN_EXE_bund2"));
    if under.is_some() {
        c.arg("--debugger");
    }
    c.args(["script", "--file"]).arg(&script);
    c.stdout(Stdio::piped()).stderr(Stdio::null());
    if under.is_some() {
        c.stdin(Stdio::piped());
    }
    let mut child = c.spawn().expect("bund2 runs");

    if let Some(cmd) = under {
        let mut pipe = child.stdin.take().expect("stdin");
        // Detached: it ends when the write fails, which is when the child has
        // gone. Writing in the test thread would block once the pipe filled.
        std::thread::spawn(move || {
            let line = format!("{cmd}\n");
            while pipe.write_all(line.as_bytes()).is_ok() {
                if pipe.flush().is_err() {
                    break;
                }
            }
        });
    }

    let out = child.wait_with_output().expect("waits");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    // F14: the id and stamp are not behaviour the reference defines, and two
    // runs of anything differ in them.
    let text = text
        .split('\n')
        .map(|l| {
            let mut s = l.to_string();
            while let Some(a) = s.find("id: \"") {
                if let Some(b) = s[a + 5..].find('"') {
                    s.replace_range(a + 5..a + 5 + b, "X");
                    s.replace_range(a..a + 4, "id:_");
                } else {
                    break;
                }
            }
            s.replace("id:_", "id: ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let text = regex_lite_strip_stamp(&text);
    (text, out.status.code())
}

/// `stamp: <float>` → `stamp: X`, without a regex dependency.
fn regex_lite_strip_stamp(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("stamp: ") {
        out.push_str(&rest[..at + 7]);
        let tail = &rest[at + 7..];
        let end = tail
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(tail.len());
        out.push('X');
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// Programs chosen to cover what §D1's safepoint has to reach: top-level
/// values, a word's body, a lambda run by a native, and a loop — the last
/// because `times` re-enters evaluation through `eval_lambda`, which is the
/// nested-loop shape §D1 says stepping could not have escaped.
const PROGRAMS: [(&str, &str); 4] = [
    ("literals", "1 2 + 4 * \ndebug.display_stack\n"),
    (
        "a word's body",
        ":w { 7 8 + } register\n1 w +\ndebug.display_stack\n",
    ),
    (
        "a lambda through a native",
        "3 { \"tick\" println } times\ndebug.display_stack\n",
    ),
    (
        "output and a named stack",
        "@other 5 6 + \n@main \"done\" println\n@other debug.display_stack\n",
    ),
];

#[test]
fn a_stepped_run_matches_an_uninterrupted_one() {
    for (what, src) in PROGRAMS {
        let (plain, plain_code) = run(src, None);
        assert!(!plain.is_empty(), "{what}: the plain run produced nothing");

        for cmd in ["s", "n", "f"] {
            let (stepped, code) = run(src, Some(cmd));
            assert_eq!(
                stepped, plain,
                "{what}: `{cmd}` to completion changed the program's output"
            );
            assert_eq!(code, plain_code, "{what}: `{cmd}` changed the exit code");
        }
    }
}

/// **The control: the debugger really did stop.** Without this the test above
/// would pass against a `--debugger` that did nothing at all, which is the
/// tautology RFC-0008's criterion 10 warns about in its own terms.
#[test]
fn the_debugger_stops_and_says_where() {
    let script = scratch("c");
    std::fs::write(&script, ":w { 7 8 + } register\n1 w\n").expect("script");

    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["--debugger", "script", "--file"])
        .arg(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut ch| {
            // Enough steps to enter the body, then run on.
            let mut pipe = ch.stdin.take().expect("stdin");
            let _ = pipe.write_all(b"s\ns\ns\ns\ns\ns\nbt\nc\n");
            drop(pipe);
            ch.wait_with_output()
        })
        .expect("bund2 runs");

    let session = String::from_utf8_lossy(&out.stderr);
    assert!(
        session.contains("the top-level stream at 0"),
        "no position line for the top level:\n{session}"
    );
    assert!(
        session.contains("#0  w at"),
        "never stopped inside the word, so §D2's symbol is not reaching the \
         session:\n{session}"
    );
    assert!(
        session.contains("next: "),
        "a position line must say what runs next:\n{session}"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).is_empty(),
        "the session leaked into the program's stdout"
    );
}

/// Drive a debugged program with a fixed script, returning (stdout, stderr,
/// exit code). Stdin is closed after the script, which detaches and runs on.
fn session(src: &str, script: &str) -> (String, String, Option<i32>) {
    let script_path = scratch("s");
    std::fs::write(&script_path, src).expect("script");

    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["--debugger", "script", "--file"])
        .arg(&script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut ch| {
            let mut pipe = ch.stdin.take().expect("stdin");
            let _ = pipe.write_all(script.as_bytes());
            drop(pipe);
            ch.wait_with_output()
        })
        .expect("bund2 runs");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

const TWO_CALLS: &str = ":w { 7 8 + } register\n1 w 2 w\ndebug.display_stack\n";

/// **Criterion 7 at process level: a breakpoint on a word stops before its
/// body runs**, and `continue` does not suppress it.
#[test]
fn a_breakpoint_stops_at_every_call_and_continue_does_not_suppress_it() {
    let (_, err, code) = session(TWO_CALLS, "break w\nc\nc\nc\n");
    assert_eq!(
        err.matches("breakpoint: w").count(),
        2,
        "both calls must stop:\n{err}"
    );
    assert!(
        err.contains("break w\n"),
        "setting a breakpoint must report what is watched:\n{err}"
    );
    assert_eq!(code, Some(0));
}

/// **Criterion 8: a conditional breakpoint cannot change the program it
/// watches** — the three cases that matter, at process level.
#[test]
fn a_conditional_breakpoint_holds_fails_or_exits_without_touching_the_program() {
    // Holds: both calls stop.
    let (_, err, _) = session(TWO_CALLS, "break w if { 1 1 == }\nc\nc\nc\n");
    assert_eq!(err.matches("breakpoint: w").count(), 2, "{err}");

    // Does not hold: nothing stops, and the program still ran.
    let (out, err, code) = session(TWO_CALLS, "break w if { 1 2 == }\nc\n");
    assert_eq!(err.matches("breakpoint: w").count(), 0, "{err}");
    assert!(out.contains("I64(15)"), "the program did not run:\n{out}");
    assert_eq!(code, Some(0));

    // **Exits: the condition's `bund.exit` must not end the debugged
    // program.** This is the case §D3 says the child VM is *required* for —
    // `request_exit` is `get_or_insert`, so in the program's own VM nothing
    // could have cleared it.
    let (out, err, code) = session(TWO_CALLS, "break w if { 9 bund.exit }\nc\n");
    assert!(
        err.contains("the condition did not stop"),
        "a failing condition must be reported:\n{err}"
    );
    assert_eq!(err.matches("breakpoint: w").count(), 0, "it stopped anyway");
    assert_eq!(
        out.matches("I64(15)").count(),
        2,
        "the program must have run both calls to completion:\n{out}"
    );
    assert_eq!(
        code,
        Some(0),
        "the condition's exit code reached the debugged program"
    );
}

/// **Criterion 9 at process level: both hooks, and each sees only its own.**
#[test]
fn a_watchpoint_fires_on_its_own_stack_and_the_workbench_is_separate() {
    // A push to `@other` stops; pushes to `@main` do not.
    let src = "1 2 \n@other 3 \n@main 4\n";
    let (_, err, _) = session(src, "watch @other\nc\nc\nc\n");
    assert_eq!(
        err.matches("watchpoint: @other").count(),
        1,
        "the named watch fired wrongly:\n{err}"
    );

    // **`@workbench` is refused rather than silently taken as either**, since
    // §D4's whole point is that the workbench is not a named stack.
    let (_, err, _) = session(src, "watch @workbench\nc\n");
    assert!(
        err.contains("not a named stack"),
        "`watch @workbench` must be refused:\n{err}"
    );

    // And a named watch on every stack the program uses never sees a
    // workbench push — the two hooks are not the same place.
    let wb = "1 { true } ?. \n";
    let (_, err, _) = session(wb, "watch @main\nc\n");
    assert_eq!(
        err.matches("watchpoint: workbench").count(),
        0,
        "a named watch saw the workbench:\n{err}"
    );
    let (_, err, _) = session(wb, "watch workbench\nc\nc\n");
    assert!(
        err.contains("watchpoint: workbench"),
        "the workbench hook did not fire:\n{err}"
    );
}
