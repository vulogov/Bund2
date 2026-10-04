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

/// Run a program, optionally under the debugger driven by `cmd`, and return
/// its normalised stdout and exit code.
fn run(src: &str, under: Option<&'static str>) -> (String, Option<i32>) {
    let dir = std::env::temp_dir().join(format!("bund2-dbg-step-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let script = dir.join("p.bund");
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
    let dir = std::env::temp_dir().join(format!("bund2-dbg-ctl-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let script = dir.join("c.bund");
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
