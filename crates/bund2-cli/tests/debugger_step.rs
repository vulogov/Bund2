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
/// The binary, with its configuration directory pointed at scratch. The
/// debugger keeps a line history under that directory, and a test run once
/// rewrote the developer's own.
fn bund2() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_bund2"));
    c.env(
        "XDG_CONFIG_HOME",
        std::env::temp_dir().join(format!("bund2-test-config-{}", std::process::id())),
    );
    c
}

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

    let mut c = bund2();
    if under.is_some() {
        c.arg("--debugger");
    }
    c.args(["script", "--file"]).arg(&script);
    c.stdout(Stdio::piped()).stderr(Stdio::null());
    // A plain run is given no input at all rather than this test's own
    // stdin, which under a backgrounded `cargo test` is a pipe that never
    // closes (F158).
    c.stdin(if under.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
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

/// The workspace root, from this crate's manifest directory.
fn repo() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

/// Every program `conform` runs: each `HERMETIC.txt` entry, and each probe
/// that has a golden. With the directory each is run from — the corpus from
/// `reference/Bund`, probes from the root — because that is where the capture
/// runs them and some resolve paths against it.
fn suite() -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let root = repo();
    let mut out = Vec::new();
    let hermetic =
        std::fs::read_to_string(root.join("tests/golden/HERMETIC.txt")).expect("HERMETIC.txt");
    for line in hermetic.lines().map(str::trim) {
        if !line.is_empty() && !line.starts_with('#') {
            out.push((root.join(line), root.join("reference/Bund")));
        }
    }
    let mut probes: Vec<_> = std::fs::read_dir(root.join("tests/probes"))
        .expect("probes")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "bund"))
        .collect();
    probes.sort();
    for p in probes {
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        if root
            .join("tests/golden/probes")
            .join(format!("{stem}.golden"))
            .exists()
        {
            out.push((p, root.clone()));
        }
    }
    out
}

/// Does this program call a word that reads standard input?
///
/// Decided from the source so the exclusion below is *derived* and cannot go
/// stale: a new probe that reads input is excluded by what it says, not by
/// someone remembering to list it.
///
/// **String literals are removed first, and before comments.** A first
/// version split on whitespace alone and excluded `convert-to-dict.bund` for
/// the sentence "on the same input" inside a `println` — a program the sweep
/// had already shown stepping through cleanly. And a literal must go before
/// `//` is cut, or `"http://x"` loses everything after its colon and the rest
/// of the line is read as code.
fn reads_stdin(src: &str) -> bool {
    const READERS: [&str; 5] = ["input", "input*", "debug", "debug.shell", "password"];
    src.lines().any(|line| {
        let mut code = String::with_capacity(line.len());
        let mut in_string = false;
        let mut escaped = false;
        for c in line.chars() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_string = false;
                }
                continue;
            }
            if c == '"' {
                in_string = true;
                // A literal is one token; keep the boundary.
                code.push(' ');
            } else {
                code.push(c);
            }
        }
        code.split("//")
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .any(|tok| READERS.contains(&tok))
    })
}

/// Run one suite program to the end, plainly or stepped, with a deadline.
///
/// A deadline because the claim under test is that stepping *finishes*, and a
/// run that does not must fail by name rather than stall the suite — which is
/// what `wait_with_output` alone would do.
fn run_file(
    file: &std::path::Path,
    cwd: &std::path::Path,
    under: Option<&'static str>,
) -> Result<(String, Option<i32>), String> {
    let mut c = bund2();
    if under.is_some() {
        c.arg("--debugger");
    }
    c.args(["script", "--file"]).arg(file).current_dir(cwd);
    c.stdout(Stdio::piped()).stderr(Stdio::null());
    c.stdin(if under.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    let mut child = c.spawn().map_err(|e| format!("spawn: {e}"))?;
    if let Some(cmd) = under {
        let mut pipe = child.stdin.take().ok_or("stdin")?;
        std::thread::spawn(move || {
            let line = format!("{cmd}\n");
            while pipe.write_all(line.as_bytes()).is_ok() {
                if pipe.flush().is_err() {
                    break;
                }
            }
        });
    }
    // Drained on its own thread while the child runs: waiting first deadlocks
    // any program whose output exceeds the pipe buffer (F43).
    let mut stdout = child.stdout.take().ok_or("stdout")?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut stdout, &mut buf);
        buf
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let status = loop {
        match child.try_wait().map_err(|e| format!("wait: {e}"))? {
            Some(s) => break s,
            None if std::time::Instant::now() > deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("did not finish within 30 s".into());
            }
            None => std::thread::sleep(std::time::Duration::from_millis(2)),
        }
    };
    let bytes = reader.join().map_err(|_| "reader thread")?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    // F14, as `run` does it: ids and stamps differ between any two runs.
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
    Ok((regex_lite_strip_stamp(&text), status.code()))
}

/// **Criterion 6 over the suite, which is what it says** — "over every suite
/// program". The test above shows it for four programs chosen to cover what
/// the safepoint must reach; this shows it for every program `conform` runs.
///
/// **Every program, the two that read input included** — F165, closed by
/// D112. The debugger takes its commands from standard input, and `input`,
/// `input*`, `debug` and `debug.shell` used to read the same stream: each `s`
/// meant for the session was taken by the program, and the run did not
/// finish. This test excluded those programs and said why.
///
/// A debugged program is now given no input of its own, so its reads end at
/// once — which is what the plain run sees too, with standard input closed —
/// and the two agree. The readers are still **found from each program's
/// source** and compared against the two names expected, now to show they
/// were stepped rather than to leave them out.
#[test]
fn stepping_agrees_with_an_uninterrupted_run_over_the_whole_suite() {
    let programs = suite();
    assert!(
        programs.len() > 100,
        "found only {} programs",
        programs.len()
    );

    let name = |p: &std::path::Path| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string()
    };
    let checked = programs;
    let mut readers: Vec<String> = checked
        .iter()
        .filter(|(file, _)| std::fs::read_to_string(file).is_ok_and(|src| reads_stdin(&src)))
        .map(|(f, _)| name(f))
        .collect();
    readers.sort();
    assert_eq!(
        readers,
        ["debug-repl-words", "terminal-words"],
        "the suite's programs that read standard input changed; they are stepped below"
    );

    // Spread over the available cores: four hundred process runs in sequence
    // is the better part of a minute, and none depends on another.
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    let next = AtomicUsize::new(0);
    let problems = std::sync::Mutex::new(Vec::<String>::new());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((file, cwd)) = checked.get(i) else {
                        break;
                    };
                    let what = name(file);
                    let plain = match run_file(file, cwd, None) {
                        Ok(p) => p,
                        Err(e) => {
                            problems
                                .lock()
                                .expect("lock")
                                .push(format!("{what}: the plain run {e}"));
                            continue;
                        }
                    };
                    for cmd in ["s", "n", "f"] {
                        let verdict = match run_file(file, cwd, Some(cmd)) {
                            Ok(stepped) if stepped == plain => continue,
                            Ok((_, code)) if code != plain.1 => {
                                format!("`{cmd}` changed the exit code: {:?} for {code:?}", plain.1)
                            }
                            Ok(_) => format!("`{cmd}` changed the program's output"),
                            Err(e) => format!("`{cmd}` {e}"),
                        };
                        problems
                            .lock()
                            .expect("lock")
                            .push(format!("{what}: {verdict}"));
                    }
                }
            });
        }
    });
    let mut problems = problems.into_inner().expect("lock");
    problems.sort();
    assert!(
        problems.is_empty(),
        "{} of {} stepped runs disagreed with the plain one:\n{}",
        problems.len(),
        checked.len() * 3,
        problems.join("\n")
    );
}

/// **The control: the debugger really did stop.** Without this the test above
/// would pass against a `--debugger` that did nothing at all, which is the
/// tautology RFC-0008's criterion 10 warns about in its own terms.
#[test]
fn the_debugger_stops_and_says_where() {
    let script = scratch("c");
    std::fs::write(&script, ":w { 7 8 + } register\n1 w\n").expect("script");

    let out = bund2()
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

    let out = bund2()
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

/// D191's program: a word called five times from a loop, a failure a `?try`
/// catches, and a word registered twice. It prints 7.
const D191: &str = ":w { 1 + } register\n\
:boom { 7 1 list / } register\n\
0 5 { drop w } times\n\
?try\n  :try { boom } set\n  :except { drop } set\n!\n\
:w { 2 + } register\n\
println\n";

/// **D191: `break w 3` stops at the third hit, once**, and `info` counts.
#[test]
fn a_breakpoint_with_a_hit_stops_there_once_and_counts() {
    let (out, err, code) = session(D191, "break w 3\nc\ninfo\nc\n");
    assert_eq!(err.matches("breakpoint: w").count(), 1, "{err}");
    assert!(err.contains("breakpoint: w (hit 3)"), "{err}");
    assert!(err.contains("break w 3  (hit 3 times)"), "{err}");
    assert_eq!((out.as_str(), code), ("7\n", Some(0)), "{err}");
}

/// **D191: `until boom` runs on to the next call of `boom`, stops, and is
/// gone.**
#[test]
fn until_runs_to_the_next_call_and_forgets_it() {
    let (out, err, _) = session(D191, "until boom\ninfo\nc\n");
    assert_eq!(err.matches("breakpoint: boom (hit 1)").count(), 1, "{err}");
    assert!(err.contains("nothing is watched"), "{err}");
    assert_eq!(out, "7\n");
}

/// **D191: `catch` stops where the failure is raised, with its frames**,
/// once although the failure unwinds through more than one loop, and the
/// program's `?try` then catches it as before.
#[test]
fn catch_stops_once_where_a_failure_is_raised_with_its_frames() {
    let (out, err, code) = session(D191, "catch\nc\nbt\nc\n");
    assert_eq!(err.matches("caught: ").count(), 1, "{err}");
    assert!(err.contains("caught: DIV returns error"), "{err}");
    assert!(
        err.contains("#0  boom at 4"),
        "the frames are still there:\n{err}"
    );
    // F217: the stop's position line names the innermost frame `#0`, as
    // `bt` does, two frames deep; it said `#1  boom` while `bt` said `#0`.
    assert!(
        err.contains(
            "caught: DIV returns error: Incompartible operation for the list\n#0  boom at 4  next:"
        ),
        "{err}"
    );
    assert!(!err.contains("#1  boom"), "{err}");
    assert!(
        err.contains("the failure unwinds when the program moves on"),
        "{err}"
    );
    assert_eq!((out.as_str(), code), ("7\n", Some(0)), "{err}");
    // A second failure, after the first was caught and the program moved
    // on, is a new one and stops again.
    let two = ":boom { 7 1 list / } register\n\
?try :try { boom } set :except { drop } set !\n\
?try :try { boom } set :except { drop } set !\n\"ok\" println\n";
    let (out, err, _) = session(two, "catch\nc\nc\nc\n");
    assert_eq!(err.matches("caught: ").count(), 2, "{err}");
    assert_eq!(out, "ok\n");
}

/// **D191: `watch redefine` stops when a bound name is registered again** —
/// the second `w`, and not the first registration of either word.
#[test]
fn watch_redefine_stops_on_the_second_registration_only() {
    let (out, err, _) = session(D191, "watch redefine\nc\nc\n");
    assert_eq!(err.matches("redefining: ").count(), 1, "{err}");
    assert!(err.contains("redefining: w"), "{err}");
    assert_eq!(out, "7\n");
}

/// **D191: `watch @main if …` stops only where the condition holds**, and a
/// condition that cannot read a value says so once, not at every push.
#[test]
fn a_conditional_watch_stops_where_it_holds_and_says_a_failure_once() {
    let (out, err, _) = session(D191, "watch @main if { 3 == }\nc\nc\nc\nc\nc\nc\n");
    assert!(err.contains("watchpoint: @main"), "{err}");
    let said: Vec<&str> = err
        .lines()
        .filter(|l| l.contains("the condition did not stop"))
        .collect();
    let mut distinct = said.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(said.len(), distinct.len(), "a reason repeated:\n{err}");
    assert_eq!(out, "7\n");
}

/// **D191: a stop says what changed since the last one.**
#[test]
fn a_stop_says_what_changed_since_the_last() {
    let (out, err, _) = session("1 2 + println\n", "s\ns\ns\nc\n");
    assert!(err.contains("changed: @main 0 → 1, pushed 1"), "{err}");
    assert!(err.contains("changed: @main 1 → 2, pushed 2"), "{err}");
    // `+` took two and left one: popped two, pushed its answer.
    assert!(
        err.contains("changed: @main 2 → 1, popped 2, pushed 3"),
        "{err}"
    );
    assert_eq!(out, "3\n");
}

/// **RFC-0008 §D4: `watch depth > N` stops on the crossing**, once: the
/// stack goes past 2 at the third push, stays past it for three more, and
/// is never back down to 2, so there is one stop.
#[test]
fn a_depth_watch_stops_once_where_the_stack_goes_past_it() {
    let (out, err, _) = session("1 2 3 4 5 6\n\"done\" println\n", "watch depth > 2\nc\nc\n");
    assert_eq!(
        err.matches("watchpoint: @main depth > 2").count(),
        1,
        "{err}"
    );
    assert!(
        err.contains("changed: @main 0 → 2"),
        "stopped before the third push:\n{err}"
    );
    assert_eq!(out, "done\n");
}

/// **And again after the stack has come back down to N.**
#[test]
fn a_depth_watch_fires_again_after_the_stack_comes_back_down() {
    let (_, err, _) = session("1 2 3 drop 4\n", "watch depth > 2\nc\nc\nc\n");
    assert_eq!(
        err.matches("watchpoint: @main depth > 2").count(),
        2,
        "{err}"
    );
}

/// **RFC-0008 §D5: `words` is the slot table, with calls since attaching,
/// and `classes` the class table.** `swap` shows both of its bindings (F20),
/// and a class shows its parents, methods and fields.
#[test]
fn the_words_and_classes_views_show_the_tables() {
    let src = ":Base class :greet { 1 } set register\n\
               :Kid class :.super [ :Base ] set :x 1 set register\n\
               :w { 7 } register\nw w 1 2 swap\n\"done\" println\n";
    let (out, err, _) = session(src, "until println\nwords\nwords swap\nclasses\nc\n");
    let calls = |name: &str| {
        err.lines()
            .find(|l| l.trim_start().starts_with(&format!("{name} ")))
            .unwrap_or_default()
            .to_string()
    };
    assert!(
        calls("w").contains("lambda") && calls("w").ends_with("calls 2"),
        "{err}"
    );
    // RFC-0011 §S8: where it was registered — line 3, at its `register`.
    assert!(
        calls("w").contains("registered at ") && calls("w").contains(":3:"),
        "{err}"
    );
    assert!(calls("swap").contains("alias → swap_one, native"), "{err}");
    assert!(
        err.contains("swap_one"),
        "the filter shows every name with swap:\n{err}"
    );
    assert!(
        err.contains("Kid\n  super: Base\n  methods: none\n  fields: x\n"),
        "{err}"
    );
    assert!(
        err.contains("Base\n  super: none\n  methods: greet\n"),
        "{err}"
    );
    assert_eq!(out, "done\n");
}

/// **RFC-0008 §D3 by source location.** `break <line>` stops before the
/// first value of the line each time a body reaches it — at the top level,
/// inside a word on each call, and inside a loop's body on each run — and
/// `break <file>:<line>` names the file by its tail.
#[test]
fn a_line_breakpoint_stops_where_the_line_begins() {
    let src = ":g {\n  1 2 +\n  println\n} register\n\"start\" println\ng g\n2 { 0 drop } times\n";
    let (out, err, _) = session(src, "break 3\nbreak s.bund:7\nc\nc\nc\nc\nc\nc\nc\n");
    let stops: Vec<&str> = err
        .lines()
        .filter_map(|l| l.split("breakpoint: ").nth(1))
        .map(|at| at.rsplit('/').next().unwrap_or(at))
        .collect();
    assert_eq!(
        stops,
        ["s.bund:3", "s.bund:3", "s.bund:7", "s.bund:7", "s.bund:7"],
        "{err}"
    );
    assert_eq!(out, "start\n3\n3\n");
}

/// **RFC-0003 §S5 for an included file's top level.** `bund.eval-file` runs
/// the file's values one by one; each is placed as the program's own are,
/// so `break lib.bund:3` stops at a top-level value there and the stop says
/// where. `next` from the call steps over the whole file, as over a body.
#[test]
fn a_line_breakpoint_stops_in_an_included_files_top_level() {
    let lib = scratch("lib");
    std::fs::write(&lib, ":inlib { 1 2 + } register\n10\n20 println\n").expect("lib");
    let lib = lib.display().to_string();
    let src = format!("\"{lib}\" bund.eval-file\n\"done\" println\n");
    let (out, err, _) = session(&src, "break lib.bund:3\nc\nc\n");
    let stops: Vec<&str> = err
        .lines()
        .filter_map(|l| l.split("breakpoint: ").nth(1))
        .map(|at| at.rsplit('/').next().unwrap_or(at))
        .collect();
    assert_eq!(stops, ["lib.bund:3"], "{err}");
    assert!(err.contains("the top-level stream at 4  next: 20"), "{err}");
    assert!(err.contains("lib.bund:3:1)"), "{err}");
    assert_eq!(out, "20\ndone\n");

    let (out, err, _) = session(&src, "n\nn\nc\n");
    assert!(
        !err.contains("lib.bund:"),
        "next steps over the file's top level: {err}"
    );
    assert_eq!(out, "20\ndone\n");
}

/// **`bt` names the streams between the frames.** A word runs
/// `bund.eval-file`, whose file's top level calls a word: the backtrace
/// reads the inner word, the file's stream at the value that called it, the
/// outer word, and the program's stream — each placed.
#[test]
fn a_backtrace_interleaves_an_included_files_stream() {
    let lib = scratch("lib2");
    std::fs::write(&lib, ":inner {\n  1 2 + drop\n} register\ninner\n").expect("lib");
    let src = format!(
        ":w {{ \"{}\" bund.eval-file }} register\nw\n",
        lib.display()
    );
    let (_, err, _) = session(&src, "break lib2.bund:2\nc\nbt\nc\n");
    let bt: Vec<String> = err
        .lines()
        .skip_while(|l| !l.starts_with("(bund2) #0  inner"))
        .take(4)
        .map(|l| {
            let l = l.trim_start_matches("(bund2) ");
            match l.split_once("  (") {
                Some((head, place)) => {
                    // File and line: a column after the path moves with it.
                    let file = place.rsplit('/').next().unwrap_or(place);
                    let line = file.rsplit_once(':').map_or(file, |(l, _)| l);
                    format!("{head}  ({line})")
                }
                None => l.to_string(),
            }
        })
        .collect();
    assert_eq!(
        bt,
        [
            "#0  inner at 0  (lib2.bund:2)",
            "#1  the top-level stream at 3  (lib2.bund:4)",
            "#2  w at 2  (s.bund:1)",
            "#3  the top-level stream at 3  (s.bund:2)",
        ],
        "{err}"
    );
}
