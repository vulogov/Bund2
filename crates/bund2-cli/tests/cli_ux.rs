//! **D173 at process level: how loud, and how a failure is drawn.**
//!
//! Everything here runs the binary on a pipe, where `auto` draws the
//! reference's table and a progress bar draws nothing, so the bytes a capture
//! sees are asserted first and the new renderings are reached by their
//! flags. Nothing here runs a program that grows.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::{Command, Stdio};

struct Run {
    out: String,
    err: String,
    code: Option<i32>,
}

fn bund2(args: &[&str], env: &[(&str, &str)]) -> Run {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bund2"));
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("bund2 ran");
    Run {
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code(),
    }
}

fn program(name: &str, src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bund2-cli-ux-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, src).unwrap();
    path
}

const FAILING: &str = ":g { 1 list / } register\n:f { 7 g } register\n\"hello\" println\nf\n";

/// On a pipe nothing changes: the table, the stack, exit 0.
#[test]
fn a_pipe_still_sees_the_reference_s_table() {
    let p = program("table.bund", FAILING);
    let r = bund2(&["script", "--file", p.to_str().unwrap()], &[]);
    assert_eq!(r.code, Some(0));
    assert!(
        r.out.contains("│ Error    ┆ DIV returns error"),
        "{}",
        r.out
    );
    assert!(r.out.contains("[BUND]  Content of the stack"), "{}", r.out);
    assert!(!r.out.contains("╭─["), "no traceback on a pipe:\n{}", r.out);
    assert!(
        !r.err.contains("bund2:"),
        "nothing of its own at the default level:\n{}",
        r.err
    );
}

/// `--report trace` draws the traceback: the file and position, the term
/// underlined, the frames innermost first, the stack name, then the stack
/// box the table would have drawn too.
#[test]
fn report_trace_draws_the_traceback_with_the_frames() {
    let p = program("trace.bund", FAILING);
    let r = bund2(
        &["script", "--report", "trace", "--file", p.to_str().unwrap()],
        &[],
    );
    assert_eq!(r.code, Some(0));
    assert!(r.out.starts_with("hello\n"), "{}", r.out);
    assert!(r.out.contains("Error: DIV returns error"), "{}", r.out);
    assert!(r.out.contains(&format!("{}:4:1", p.display())), "{}", r.out);
    assert!(
        r.out.contains("the step that failed began here"),
        "{}",
        r.out
    );
    let g = r.out.find("#0  g at 3").expect("innermost frame");
    let f = r.out.find("#1  f at 2").expect("outer frame");
    assert!(g < f, "innermost first:\n{}", r.out);
    assert!(r.out.contains("on stack `main`"), "{}", r.out);
    assert!(r.out.contains("[BUND]  Content of the stack"), "{}", r.out);
    assert!(
        !r.out.contains("│ Error    ┆"),
        "no table beside it:\n{}",
        r.out
    );
}

/// `--report box` is the table even where `trace` was asked for in the
/// environment, and an unknown rendering is refused before anything runs.
#[test]
fn report_box_wins_over_the_environment_and_an_unknown_mode_is_refused() {
    let p = program("box.bund", FAILING);
    let r = bund2(
        &["script", "--report", "box", "--file", p.to_str().unwrap()],
        &[("BUND2_REPORT", "trace")],
    );
    assert!(r.out.contains("│ Error    ┆"), "{}", r.out);
    let r = bund2(
        &["script", "--report", "loud", "--file", p.to_str().unwrap()],
        &[],
    );
    assert_eq!(r.code, Some(2));
    assert!(
        r.err.contains("`--report loud` names no rendering"),
        "{}",
        r.err
    );
    assert!(r.out.is_empty());
}

/// The verbosity flags set two thresholds at once: the command line's own
/// lines, and the program's `log.*` words.
#[test]
fn verbosity_raises_what_the_command_line_and_the_log_words_say() {
    let p = program(
        "log.bund",
        "\"x\" log.info \"d\" log.debug \"t\" log.trace \"w\" log.warning \"e\" log.error\n",
    );
    let path = p.to_str().unwrap();
    let r = bund2(&["script", "--file", path], &[]);
    assert_eq!(r.err, "Warning: log.error: e\n", "{}", r.err);

    let r = bund2(&["-q", "script", "--file", path], &[]);
    assert_eq!(r.err, "", "-q hides warnings:\n{}", r.err);

    let r = bund2(&["script", "-v", "--file", path], &[]);
    assert!(r.err.contains("bund2: info: running "), "{}", r.err);
    assert!(r.err.contains("Notice: log.info: x"), "{}", r.err);
    assert!(!r.err.contains("log.debug"), "{}", r.err);

    let r = bund2(&["-vv", "script", "--file", path], &[]);
    assert!(r.err.contains("bund2: debug: parsed "), "{}", r.err);
    assert!(r.err.contains("Notice: log.debug: d"), "{}", r.err);
    assert!(!r.err.contains("log.trace"), "{}", r.err);

    let r = bund2(&["--verbose=trace", "script", "--file", path], &[]);
    assert!(r.err.contains("Notice: log.trace: t"), "{}", r.err);

    let r = bund2(&["--verbose=loud", "script", "--file", path], &[]);
    assert_eq!(r.code, Some(2));
    assert!(r.err.contains("names no level"), "{}", r.err);
}

/// Flags after `--` are the program's.
#[test]
fn flags_after_the_double_dash_reach_the_program() {
    let p = program("args.bund", "args println\n");
    let r = bund2(
        &["script", "--file", p.to_str().unwrap(), "--", "-v", "-q"],
        &[],
    );
    assert!(r.out.contains("-v") && r.out.contains("-q"), "{}", r.out);
    assert_eq!(r.err, "", "{}", r.err);
}

/// `build` says one line at the default level and its steps at `-v` and
/// above; on a pipe the progress bar draws nothing, so the lines are the
/// whole of standard error.
#[test]
fn build_reports_its_steps_by_level_and_nothing_else_on_a_pipe() {
    let p = program("built.bund", "1 2 + println\n");
    let out = p.with_extension("bundle");
    let r = bund2(
        &[
            "build",
            "--file",
            p.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(r.code, Some(0), "{}", r.err);
    assert_eq!(r.err.lines().count(), 1, "{}", r.err);
    assert!(r.err.starts_with("bund2: wrote "), "{}", r.err);

    let r = bund2(
        &[
            "-vvv",
            "build",
            "--file",
            p.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(r.code, Some(0), "{}", r.err);
    for expected in [
        "bund2: info: read ",
        "bund2: info: parsed ",
        "bund2: debug: the runtime to copy is ",
        "bund2: trace: payload region at offset ",
        "bund2: info: wrote the program into the image",
        "bund2: debug: staging the artefact at ",
        "bund2: wrote ",
    ] {
        assert!(r.err.contains(expected), "missing `{expected}`:\n{}", r.err);
    }
    assert!(
        !r.err.contains('\r'),
        "no progress redraws on a pipe:\n{}",
        r.err
    );

    // A bundle reads the environment for the same switches.
    let mut cmd = Command::new(&out);
    cmd.env("BUND2_VERBOSE", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let ran = cmd.output().expect("the bundle ran");
    let err = String::from_utf8_lossy(&ran.stderr);
    assert!(err.contains("bund2: info: running "), "{err}");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "3\n");
}

/// A build that fails keeps the words the scripts around it know.
#[test]
fn a_failed_build_still_says_nothing_was_written() {
    let r = bund2(&["build", "--file"], &[]);
    assert_eq!(r.code, Some(2));
    assert!(
        r.err.contains("bund2: build: `--file` needs a source path"),
        "{}",
        r.err
    );
    assert!(r.err.contains("bund2: nothing was written"), "{}", r.err);
    let r = bund2(
        &[
            "build",
            "--file",
            "/nonexistent/p.bund",
            "--output",
            "/tmp/x",
        ],
        &[],
    );
    assert_eq!(r.code, Some(1));
    assert!(
        r.err.starts_with("bund2: reading /nonexistent/p.bund: "),
        "{}",
        r.err
    );
}
