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
    c.env("XDG_CONFIG_HOME", std::env::temp_dir().join(format!("bund2-test-config-{}", std::process::id())));
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
    assert!(programs.len() > 100, "found only {} programs", programs.len());

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
            scope.spawn(|| loop {
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
