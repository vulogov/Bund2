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

/// `help` in every spelling, for one command too, and `--version`; no
/// arguments is a mistake and is told as one.
#[test]
fn help_and_version_are_answered_and_no_arguments_is_told() {
    for args in [&["help"][..], &["--help"], &["-h"], &["-v", "--help"]] {
        let r = bund2(args, &[]);
        assert_eq!(r.code, Some(0), "{args:?}: {}", r.err);
        assert!(
            r.out.contains("USAGE") && r.out.contains("script --file <path>"),
            "{}",
            r.out
        );
        assert!(
            r.out.contains("--verbose=") && r.out.contains("BUND2_REPORT"),
            "{}",
            r.out
        );
    }
    for args in [
        &["help", "build"][..],
        &["build", "--help"],
        &["build", "-h", "--file", "x"],
    ] {
        let r = bund2(args, &[]);
        assert_eq!(r.code, Some(0), "{args:?}: {}", r.err);
        assert!(r.out.starts_with("bund2 build — "), "{}", r.out);
        assert!(
            r.out.contains("--inspect") && r.out.contains("--output"),
            "{}",
            r.out
        );
    }
    let r = bund2(&["script", "--help"], &[]);
    assert!(
        r.out.contains("--report <mode>") && r.out.contains("--no-dump-stack"),
        "{}",
        r.out
    );

    let r = bund2(&["help", "nope"], &[]);
    assert_eq!(r.code, Some(2));
    assert!(r.err.contains("`nope` is not a command"), "{}", r.err);

    for args in [&["--version"][..], &["-V"]] {
        let r = bund2(args, &[]);
        assert_eq!(r.code, Some(0));
        assert!(
            r.out
                .starts_with(&format!("bund2 {}", env!("CARGO_PKG_VERSION"))),
            "{}",
            r.out
        );
        assert!(r.out.contains("features"), "{}", r.out);
    }

    let r = bund2(&[], &[]);
    assert_eq!(r.code, Some(2));
    assert!(
        r.err.contains("expected a command") && r.err.contains("--help"),
        "{}",
        r.err
    );
    assert!(r.out.is_empty());
}

/// The conventions a command line is expected to keep: a bare path runs
/// and ends the flags, a flag takes `=value`, an unknown command and an
/// unknown flag are named with where to look, and a missing file is a
/// usage error.
#[test]
fn a_path_runs_flags_take_equals_and_mistakes_point_at_help() {
    let p = program("conv.bund", "1 2 + println\n");
    let path = p.to_str().unwrap();
    for args in [
        &[path][..],
        &["script", path],
        &["script", &format!("--file={path}")],
        &["--no-dump-stack", path],
        &["script", "--no-dump-stack", path],
    ] {
        let r = bund2(args, &[]);
        assert_eq!(r.code, Some(0), "{args:?}: {}", r.err);
        assert_eq!(r.out, "3\n", "{args:?}");
    }
    let r = bund2(&["frobnicate"], &[]);
    assert_eq!(r.code, Some(2));
    assert!(
        r.err.contains("`frobnicate` is not a command") && r.err.contains("--help"),
        "{}",
        r.err
    );
    let r = bund2(&["script", "--file", path, "--loud"], &[]);
    assert_eq!(r.code, Some(2));
    assert!(
        r.err.contains("unknown argument `--loud`") && r.err.contains("help script"),
        "{}",
        r.err
    );
    let r = bund2(&["script"], &[]);
    assert_eq!(r.code, Some(2));
    assert!(
        r.err.contains("expected: bund2 script --file <path>"),
        "{}",
        r.err
    );
    let r = bund2(&["script", "--file"], &[]);
    assert_eq!(r.code, Some(2));
    assert!(r.err.contains("`--file` needs a path"), "{}", r.err);
}

/// `#!/usr/bin/env bund2`: the file runs itself, its arguments reach the
/// program, the first line is skipped and the line numbers are not.
#[cfg(unix)]
#[test]
fn a_shebang_file_runs_itself_with_its_arguments_and_its_line_numbers() {
    use std::os::unix::fs::PermissionsExt;
    let p = program(
        "she.bund",
        &format!(
            "#!{}\nargs println\n1 list /\n",
            env!("CARGO_BIN_EXE_bund2")
        ),
    );
    let mut perm = std::fs::metadata(&p).unwrap().permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(&p, perm).unwrap();
    // **F212.** The tests run as threads of one process, and on Linux a
    // child another test forks between this file's write and its close
    // inherits the write handle until it execs. Executing the file while
    // that child holds it is `ETXTBSY`, "Text file busy", which CI met once.
    // The handle is gone within milliseconds, so the exec is retried.
    let mut tries = 0;
    let out = loop {
        let r = Command::new(&p)
            .args(["--alpha", "beta"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        match r {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy && tries < 50 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            other => break other.expect("the file ran"),
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--alpha") && stdout.contains("beta"),
        "{stdout}"
    );
    assert!(stdout.contains("DIV returns error"), "{stdout}");
    // Line 3 of the file, not line 2 of what was parsed.
    assert!(
        stdout.contains(&format!("{}:3:", p.display())) || stdout.contains(":3:"),
        "{stdout}"
    );
    assert_eq!(out.status.code(), Some(0));
    // After a bare path, `--` is the separator and the rest is the program's.
    let r = bund2(&[p.to_str().unwrap(), "--", "-q"], &[]);
    assert!(r.out.contains("-q"), "{}", r.out);
}

/// **RFC-0011, D185 and D187 at process level.** A bare `--profile` is
/// refused naming the operand, as is one whose operand is the next flag;
/// `--profile <base>` writes the two files, changes nothing on `stdout` and
/// says nothing on `stderr` at the default level; `-v` names the files; and
/// a base nobody can write to is an error with exit 1 after a program that
/// succeeded.
#[test]
fn profile_needs_a_base_and_then_writes_two_files_and_nothing_else() {
    let p = program(
        "profiled.bund",
        ":inc { 1 + } register 5 inc inc dup drop println\n",
    );
    let path = p.to_str().unwrap();

    let bare = bund2(&["script", "--file", path, "--profile"], &[]);
    assert_eq!(bare.code, Some(2), "{}", bare.err);
    assert!(
        bare.err
            .contains("`--profile` needs a path for the two files"),
        "{}",
        bare.err
    );
    let flag_as_operand = bund2(&["script", "--profile", "--file", path], &[]);
    assert_eq!(flag_as_operand.code, Some(2), "{}", flag_as_operand.err);
    assert!(
        flag_as_operand
            .err
            .contains("needs a path for the two files"),
        "{}",
        flag_as_operand.err
    );

    let plain = bund2(&["script", "--file", path], &[]);
    let base = p.with_file_name("run1");
    let base_s = base.to_str().unwrap();
    let profiled = bund2(&["script", "--profile", base_s, "--file", path], &[]);
    assert_eq!(profiled.code, Some(0), "{}", profiled.err);
    assert_eq!(
        profiled.out, plain.out,
        "a profiled run's stdout is the plain run's"
    );
    assert_eq!(profiled.out, "7\n");
    assert_eq!(profiled.err, "", "nothing on stderr at the default level");
    let md = std::fs::read_to_string(format!("{base_s}.profile.md")).expect("the report");
    let dot = std::fs::read_to_string(format!("{base_s}.profile.dot")).expect("the graph");
    assert!(md.starts_with("# bund2 profile\n"), "{md}");
    assert!(
        md.contains("| program | ended | tier | console | wall | steps | calls |"),
        "{md}"
    );
    assert!(md.contains("| normally | declined (profiling)"), "{md}");
    // D188: the clock's cost and resolution, the overhead and the floor are
    // in the header, and the version with them.
    assert!(
        md.contains("| bund2 | date | clock cost / reading | clock resolution | clock readings | clock overhead | % of wall | per-call floor |"),
        "{md}"
    );
    assert!(md.contains(env!("CARGO_PKG_VERSION")), "{md}");
    assert!(md.contains("| steps | % of steps |"), "{md}");
    assert!(md.contains("| `inc` |"), "{md}");
    // Item 3: a fixed pair is printed as one.
    assert!(md.contains("| `inc` |  | word | (1, 1) |"), "{md}");
    assert!(md.contains("## Call tree"), "{md}");
    // Items 1 and 2 of 2026-10-17: a section only for words the tree does
    // not already describe, and a legend in the graph.
    assert!(
        md.contains(
            "## Callers and callees of the hottest words with more than one caller or any callee"
        ),
        "{md}"
    );
    assert!(md.contains("### `inc`"), "{md}");
    assert!(
        !md.contains("### `println`"),
        "a single-caller leaf has no section: {md}"
    );
    // The legend is the graph's own label, pinned bottom left.
    assert!(dot.contains("labelloc=\"b\"; labeljust=\"l\";"), "{dot}");
    assert!(
        dot.contains("label=\"shape: double box the program"),
        "{dot}"
    );
    // A node spelled differently from its resolved name breaks its label
    // with `\n`, not a literal backslash `dot` would print as text.
    assert!(dot.contains("dup\\n(dup_one)"), "{dot}");
    assert!(!dot.contains("\\\\n"), "an escaped line break: {dot}");
    assert!(dot.contains("\\n1 call\\n"), "singular for one call: {dot}");
    assert!(dot.contains("peripheries=2"), "{dot}");
    assert!(dot.starts_with("digraph profile {"), "{dot}");
    assert_eq!(
        dot.matches('{').count(),
        dot.matches('}').count(),
        "balanced braces: {dot}"
    );
    // Every edge names two declared nodes.
    let declared: Vec<String> = dot
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix('n'))
        .filter_map(|l| {
            l.split_once(' ')
                .map(|(id, _)| id.trim_end_matches("->").to_string())
        })
        .filter(|id| id.chars().all(|c| c.is_ascii_digit()))
        .collect();
    for line in dot.lines().filter(|l| l.contains(" -> ")) {
        let (a, rest) = line.trim().split_once(" -> ").unwrap();
        let b = rest.split(' ').next().unwrap();
        for id in [a, b] {
            let id = id.trim_start_matches('n');
            assert!(
                declared.contains(&id.to_string()),
                "{id} undeclared in {dot}"
            );
        }
    }
    // `dot` itself, where the host has it.
    match Command::new("dot")
        .arg("-Tsvg")
        .arg(format!("{base_s}.profile.dot"))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(out) => assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(_) => eprintln!(
            "dot is not installed here; the parse half of criterion 2 ran, the render half did not"
        ),
    }

    let verbose = bund2(&["-v", "script", "--profile", base_s, "--file", path], &[]);
    assert_eq!(verbose.code, Some(0));
    assert!(
        verbose.err.contains(&format!(
            "wrote {base_s}.profile.md and {base_s}.profile.dot"
        )),
        "{}",
        verbose.err
    );

    // A base in a directory that does not exist: the program ran and its
    // output is there, the files are not, and the process says so with 1.
    let nowhere = p.with_file_name("no-such-dir").join("run");
    let failed = bund2(
        &[
            "script",
            "--profile",
            nowhere.to_str().unwrap(),
            "--file",
            path,
        ],
        &[],
    );
    assert_eq!(failed.out, "7\n");
    assert_eq!(failed.code, Some(1), "{}", failed.err);
    assert!(failed.err.contains("--profile:"), "{}", failed.err);
    assert!(
        failed.err.contains("No profile was written"),
        "{}",
        failed.err
    );

    // A program that fails keeps its code, 0, even when its profile is
    // written; and the profile is written.
    let bad = program("profiled-bad.bund", ":inc { 1 + } register 5 inc boom\n");
    let base2 = p.with_file_name("run2");
    let base2_s = base2.to_str().unwrap();
    let r = bund2(
        &[
            "script",
            "--profile",
            base2_s,
            "--file",
            bad.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(r.code, Some(0), "{}", r.err);
    let md2 = std::fs::read_to_string(format!("{base2_s}.profile.md")).expect("the report");
    // Item 3: `boom` declares no effect, so `inc` is still a pair, and the
    // recursive case is the record test's; here an opaque native says its
    // opaque is declared.
    let rec = program(
        "profiled-rec.bund",
        ":r { dup 3 > { 1 + r } if } register 0 r println\n",
    );
    let base3 = p.with_file_name("run3");
    let base3_s = base3.to_str().unwrap();
    let r3 = bund2(
        &[
            "script",
            "--profile",
            base3_s,
            "--file",
            rec.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(r3.code, Some(0), "{}", r3.err);
    let md3 = std::fs::read_to_string(format!("{base3_s}.profile.md")).expect("the report");
    assert!(
        md3.contains(
            "| `r` |  | word | opaque: recursive, `r`, whose effect is not a fixed pair |"
        ),
        "{md3}"
    );
    assert!(
        md3.contains("| `if` |  | native | opaque, declared |"),
        "{md3}"
    );
    // D188: the header says the run ended in an error, names the word that
    // was innermost — the top level, where `boom` was named — and gives
    // the reason and where.
    assert!(md2.contains("| error in `⟨program⟩`: "), "{md2}");
    assert!(md2.contains("boom"), "{md2}");
    assert!(md2.contains("profiled-bad.bund:1:"), "{md2}");
    // D189: nothing in the first run was called often enough to be sampled,
    // and the report says so; a loop of cheap natives is sampled, every
    // estimated figure carries a `~`, and the graph's legend says what one
    // means.
    assert!(md.contains("## Timing\n\n- every call was timed."), "{md}");
    assert!(!md.contains("~"), "{md}");
    let lp = program("profiled-loop.bund", "0 5000 { 1 + } times println\n");
    let base4 = p.with_file_name("run4");
    let base4_s = base4.to_str().unwrap();
    let r4 = bund2(
        &[
            "script",
            "--profile",
            base4_s,
            "--file",
            lp.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(r4.code, Some(0), "{}", r4.err);
    assert_eq!(r4.out, "5000\n");
    let md4 = std::fs::read_to_string(format!("{base4_s}.profile.md")).expect("the report");
    let dot4 = std::fs::read_to_string(format!("{base4_s}.profile.dot")).expect("the graph");
    assert!(
        md4.contains("**Some calls were estimated, not measured (D189).**"),
        "{md4}"
    );
    assert!(
        md4.contains("| `+` |  | native | (2, 1) | 5 000 | 0 | 0.0 | ~"),
        "{md4}"
    );
    assert!(md4.contains("| `+` | 5 000 | 372 | 4 628 | "), "{md4}");
    assert!(md4.contains("| sampled to the end |"), "{md4}");
    assert!(dot4.contains("\\n~"), "{dot4}");
    assert!(dot4.contains("~: a native timed one call in 16"), "{dot4}");
}

/// RFC-0011 §S8, `debug.profile`: a mid-run report beside the end-of-run
/// one, its header saying the run had not ended; without `--profile` a
/// warning and the program goes on; under `--noio` the word fails as the
/// other I/O stubs do.
#[test]
fn debug_profile_writes_the_run_so_far() {
    let dir = std::env::temp_dir().join(format!("bund2-cli-ux-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mid = dir.join("midrun");
    let mid_s = mid.to_str().unwrap();
    let p = program(
        "midrun.bund",
        &format!(":w {{ 1 drop }} register w \"{mid_s}\" debug.profile w 5 println\n"),
    );
    let path = p.to_str().unwrap();
    let end = dir.join("endrun");
    let end_s = end.to_str().unwrap();
    let r = bund2(&["script", "--profile", end_s, "--file", path], &[]);
    assert_eq!(r.code, Some(0), "{}", r.err);
    assert_eq!(r.out, "5\n");
    assert_eq!(r.err, "", "nothing on stderr at the default level");
    let md = std::fs::read_to_string(format!("{mid_s}.profile.md")).expect("the mid-run report");
    assert!(std::path::Path::new(&format!("{mid_s}.profile.dot")).exists());
    assert!(
        md.contains(
            "| not yet: written mid-run by `debug.profile`, 1 activation still open and counted to that moment |"
        ),
        "{md}"
    );
    assert!(md.contains("| `w` |  | word | (0, 0) | 1 |"), "{md}");
    let md_end = std::fs::read_to_string(format!("{end_s}.profile.md")).expect("the end report");
    assert!(md_end.contains("| normally |"), "{md_end}");
    assert!(
        md_end.contains("| `w` |  | word | (0, 0) | 2 |"),
        "{md_end}"
    );

    let plain = bund2(&["script", "--file", path], &[]);
    assert_eq!(plain.code, Some(0), "{}", plain.err);
    assert_eq!(plain.out, "5\n");
    assert!(
        plain
            .err
            .contains("DEBUG.PROFILE: this run keeps no profile, so nothing was written"),
        "{}",
        plain.err
    );

    // A write that fails names the file the program asked for, not the
    // staged name it is first written under.
    let bad = program(
        "midrun-bad.bund",
        "\"/no/such/directory/x\" debug.profile\n",
    );
    let rb = bund2(
        &[
            "script",
            "--profile",
            end_s,
            "--file",
            bad.to_str().unwrap(),
        ],
        &[],
    );
    assert!(
        rb.out.contains("writing /no/such/directory/x.profile.dot:"),
        "{}",
        rb.out
    );
    assert!(!rb.out.contains(".bund2-profile-"), "{}", rb.out);

    // Once a run, however many snapshots ask.
    let looped = program(
        "midrun-loop.bund",
        "3 { drop \"x\" debug.profile } times 1 println\n",
    );
    let rl = bund2(&["script", "--file", looped.to_str().unwrap()], &[]);
    assert_eq!(rl.out, "1\n", "{}", rl.err);
    assert_eq!(
        rl.err.matches("this run keeps no profile").count(),
        1,
        "{}",
        rl.err
    );

    let noio = bund2(&["script", "--noio", "--file", path], &[]);
    // The error table goes where the reference prints it, to stdout.
    assert!(
        noio.out
            .contains("bund DEBUG.PROFILE functions disabled with --noio"),
        "{}",
        noio.out
    );
    assert!(!noio.out.contains("\n5\n"), "the run stopped at the word");
}
