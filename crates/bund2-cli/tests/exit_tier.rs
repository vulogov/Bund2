//! RFC-0005 criterion 30's `if`-branch case, at process level.
//!
//! The criterion names a program and an assertion about **output**: `"after
//! if"` must never print, and the process must end with code 7 — in the tier
//! exactly as without it. Nothing in process captures stdout, which is why this
//! lives beside `input_exit.rs` and spawns the binary, as that test does for
//! the `input*` case.
//!
//! **Both configurations come from one binary**, chosen by `--jit-threshold`,
//! so this is not a feature A/B across two builds. A threshold of 1 compiles
//! every body on its first evaluation; one above any count the program reaches
//! compiles none. The difference between the runs is the tier and nothing else.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

/// The criterion's program **plus `1 2 + drop`**, and the addition is the
/// point of this comment.
///
/// As the criterion writes it —
/// `3 { "tick" println true { "bye" println 7 exit } if "after if" println } times`
/// — the body has **no inlinable site**, so F136's rule refuses to compile it
/// and `--stats` reports `0 bodies`. Run that way the differential compares
/// Tier 0 with Tier 0 and asserts nothing about the tier, which is how the
/// first draft of this test passed.
///
/// `1 2 + drop` gives the body a site (`+` and `drop` both publish arms) and
/// changes nothing it observes: the sum is dropped, and the program's output
/// and exit code are what they were. With it the tier compiles 1 body, inlines
/// 2 sites and promotes 2 values. *(This said "and crosses 2 calls" until
/// RFC-0005's twenty-third review: the shipped tier crosses none, by D75.)*
///
/// **What this program cannot show**, and the same review's B3: the `times`
/// body exits on its first iteration, which is the one that compiles it, so
/// the compiled form is produced and never entered. It is the criterion's
/// own program and stays for that reason; [`HOT_PROGRAM`] is the one whose
/// exit is reached from compiled code.
const PROGRAM: &str = "3 { \"tick\" println 1 2 + drop true { \"bye\" println 7 exit } if \"after if\" println } times\n";

/// The same branch, reached on an entry the cache answers.
///
/// `w` holds the `if`; its condition is the word `c`, registered `false`
/// while `w` is entered twice and then registered `true`. So the first entry
/// compiles `w`, the second runs it compiled past the branch, and the third
/// runs it compiled into the branch that exits. `--stats` then reports
/// `2 entered`: the exiting entry is counted, because a compiled entry that
/// ends in an error is one.
const HOT_PROGRAM: &str = ":c { false } register\n\
:w { \"tick\" println 1 2 + drop c { \"bye\" println 7 exit } if \"after if\" println } register\n\
w w :c { true } register w \"unreached\" println\n";

/// Run at a threshold, returning `(stdout, exit code, stats line)`.
fn run_at(script: &std::path::Path, threshold: &str) -> (String, Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["script", "--stats", "--jit-threshold", threshold, "--file"])
        .arg(script)
        .output()
        .expect("bund2 runs");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    // `--stats` may go to either stream; the caller only wants the line. **Both
    // shapes count**: a build with a tier reports `tier compiled …`, and one
    // without reports `no tier`. Matching only the first made a default build
    // indistinguishable from a missing `--stats`.
    let stats = stdout
        .lines()
        .chain(stderr.lines())
        .find(|l| l.contains("tier compiled") || l.contains("no tier"))
        .unwrap_or_default()
        .to_string();
    let program_output: String = stdout
        .lines()
        .filter(|l| !l.contains("tier compiled"))
        .map(|l| format!("{l}\n"))
        .collect();
    (program_output, out.status.code(), stats)
}

/// **Criterion 30: an `exit` in an `if` branch ends the program at the same
/// point, compiled or not.**
///
/// The branch lambda is a *cold callee* — it runs once and exits, so it never
/// approaches any threshold. That is the condition the criterion sets on this
/// row: "run with the callee held below the compile threshold, since threshold
/// 1 compiles the callee and hides the defect".
///
/// **What this asserts that an in-process differential could not**: the output.
/// `"tick"` prints once and `"bye"` prints once; `"after if"` must not print at
/// all, because the exit ends the program before the value after the `if` is
/// reached, and `times` must not run a second iteration.
#[test]
fn an_exit_in_an_if_branch_matches_without_the_tier() {
    let dir = std::env::temp_dir().join(format!("bund2-exit-tier-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let script = dir.join("if-branch.bund");
    std::fs::write(&script, PROGRAM).expect("script");

    let (tiered, tiered_code, tiered_stats) = run_at(&script, "1");
    let (plain, plain_code, plain_stats) = run_at(&script, "1000000");

    // **The precondition, because without it this row is Tier 0 against
    // itself.** F136 refuses a body with no inlinable site, and the
    // criterion's own program has none — checked, and the reason `PROGRAM`
    // carries `1 2 + drop`.
    //
    // **A default build has no tier at all, and that is a case rather than an
    // assumption.** Without `--features jit` there is no "tier compiled" line
    // to read: `--stats` says `no tier (built without \`jit\`)`, and the
    // precondition below was asserting `contains` on an empty string.
    // Written as an assumption it failed `cargo test --workspace` on a clean
    // checkout, which is worse than not testing it — a known-red test
    // camouflages the next real failure, which is the F109 shape exactly.
    //
    // So both builds are asserted, and the criterion's claims about output and
    // exit code are checked in both. Only the tier *comparison* needs a tier.
    let tiered_half = if tiered_stats.contains("no tier") {
        // Asserted rather than assumed, and this is the only place that checks
        // what `--stats` says when there is nothing to report.
        assert_eq!(
            tiered_stats, plain_stats,
            "without a tier the threshold cannot change what `--stats` reports"
        );
        assert!(
            tiered_stats.contains("built without `jit`"),
            "a build with no tier must say why: {tiered_stats:?}"
        );
        false
    } else {
        assert!(
            !tiered_stats.is_empty(),
            "`--stats` reported neither a tier nor its absence"
        );
        assert!(
            !tiered_stats.contains("compiled 0 bodies"),
            "the tier compiled nothing, so this compares Tier 0 with itself: {tiered_stats:?}"
        );
        assert!(
            plain_stats.contains("compiled 0 bodies"),
            "the control must compile nothing, or the two runs are the same: {plain_stats:?}"
        );
        true
    };

    assert_eq!(
        plain_code,
        Some(7),
        "without the tier the program exits with the code it asked for"
    );
    assert_eq!(tiered_code, plain_code, "exit code");
    assert_eq!(tiered, plain, "output");

    assert!(
        !plain.contains("after if"),
        "the exit must end the program before the value after the `if`: {plain:?}"
    );
    assert!(
        plain.contains("tick") && plain.contains("bye"),
        "the branch must have been reached, or this asserts nothing: {plain:?}"
    );
    assert_eq!(
        plain.matches("tick").count(),
        1,
        "`times` must not run a second iteration after the exit: {plain:?}"
    );

    // Said once, at the end, so a default run reports what it did and did not
    // cover rather than looking like the full row.
    if !tiered_half {
        eprintln!(
            "exit_tier: no tier in this build, so criterion 30's output and exit \
             code are checked and the tier comparison is not. Run with \
             `--features jit` for the other half."
        );
    }
}

/// **Criterion 30's `if`-branch case with the body hot** — RFC-0005's
/// twenty-third review, B3. The test above runs the criterion's program,
/// whose compiled body is never entered; this one enters it.
#[test]
fn an_exit_in_an_if_branch_of_a_hot_body_matches_without_the_tier() {
    let dir = std::env::temp_dir().join(format!("bund2-exit-tier-hot-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let script = dir.join("if-branch-hot.bund");
    std::fs::write(&script, HOT_PROGRAM).expect("script");

    let (tiered, tiered_code, tiered_stats) = run_at(&script, "1");
    let (plain, plain_code, plain_stats) = run_at(&script, "1000000");

    assert_eq!(
        plain_code,
        Some(7),
        "the program exits with the code it asked for"
    );
    assert_eq!(
        plain, "tick\nafter if\ntick\nafter if\ntick\nbye\n",
        "two entries pass the branch and the third takes it"
    );
    assert_eq!(tiered_code, plain_code, "exit code");
    assert_eq!(tiered, plain, "output");

    if tiered_stats.contains("no tier") {
        eprintln!(
            "exit_tier: no tier in this build, so the hot `if`-branch case checked \
             output and exit code only. Run with `--features jit` for the comparison."
        );
        return;
    }
    assert!(
        tiered_stats.contains("compiled 1 bodies (2 entered)"),
        "`w` must compile on its first entry and run compiled on the next two, \
         the second of them into the exit: {tiered_stats:?}"
    );
    assert!(
        plain_stats.contains("compiled 0 bodies"),
        "the control must compile nothing: {plain_stats:?}"
    );
}

/// **`--stats` names §S8's two floors** — RFC-0005 criterion 11 as D147
/// leaves it: "reported with the result: both floors". On stderr, on a line
/// of its own, with the upper floor above the lower by the same distance in
/// every run of one binary, and, where there is a tier, what that floor has
/// turned away — nothing, for a program that nests nothing.
#[test]
fn stats_names_both_floors() {
    let dir = std::env::temp_dir().join(format!("bund2-floors-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let script = dir.join("floors.bund");
    std::fs::write(&script, "1 2 + drop\n").expect("the program");

    let floors = || {
        let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
            .args(["script", "--stats", "--file"])
            .arg(&script)
            .output()
            .expect("bund2 runs");
        assert!(
            !String::from_utf8_lossy(&out.stdout).contains("floors:"),
            "the floors belong on stderr: a golden captures stdout"
        );
        String::from_utf8_lossy(&out.stderr)
            .lines()
            .find(|l| l.starts_with("bund2: floors: tier 0 at 0x"))
            .map(str::to_string)
            .expect("`--stats` prints the floors line")
    };
    let gap = |line: &str| {
        line.split('(')
            .nth(1)
            .and_then(|r| r.split(')').next())
            .map(str::to_string)
            .expect("the distance between the floors")
    };

    let (first, second) = (floors(), floors());
    assert!(first.contains(", tier 1 at 0x"), "{first}");
    assert!(
        gap(&first).ends_with("bytes above"),
        "tier 1's floor is the higher: {first}"
    );
    assert_eq!(
        gap(&first),
        gap(&second),
        "the distance is the binary's, not the run's"
    );
    // A build with a tier also says what the floor did; one without has
    // nothing to say, and says nothing rather than a zero that means less.
    let with_tier = first.contains("declined");
    assert!(
        !with_tier || first.ends_with("no entry declined at the tier 1 floor"),
        "{first}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The floors line of one `--stats` run, from stderr.
fn floors_of(script: &std::path::Path, threshold: &str) -> (String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["script", "--stats", "--jit-threshold", threshold, "--file"])
        .arg(script)
        .output()
        .expect("bund2 runs");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let line = |starts: &str| {
        stderr
            .lines()
            .find(|l| l.starts_with(starts))
            .unwrap_or_default()
            .to_string()
    };
    (
        line("bund2: floors: tier 0 at 0x"),
        line("bund2: tier compiled"),
    )
}

/// The number that precedes `after` in `line`.
fn number_before(line: &str, after: &str) -> Option<usize> {
    line.split(after)
        .next()?
        .split(|c: char| !c.is_ascii_digit())
        .rfind(|s| !s.is_empty())?
        .parse()
        .ok()
}

/// **`--stats` counts what the Tier 1 floor turned away** — RFC-0005
/// criterion 11: "with the tier, how many entries the Tier 1 floor declined
/// and how many compiled bodies were running at the first". The test above
/// reads the line in its "no entry declined" form; this is the other form,
/// which `cargo xtask depth` printed and no test asserted (the twenty-fourth
/// review's S7).
///
/// `down` has a site, so the tier compiles it, and it calls itself 20,000
/// deep: compiled bodies run one beneath another until the floor, and every
/// entry after that is declined and interpreted. The three figures have to
/// add up. One entry compiled the body and ran interpreted; of the other
/// 20,000 each was either entered compiled or declined; and the first
/// decline came with every compiled entry still running, since nothing had
/// returned.
///
/// **The inner lambda must have no site**, or it compiles too and the
/// figures count two bodies per level. It was `{ 1 swap - down }` until
/// 2026-10-15, when `-` gained an arm; the decrement now sits in `down`'s
/// own body, ahead of the test, and the lambda is the bare recursive call.
///
/// **`if` is not `down`'s last value** — `0 drop` follows it. Since
/// 2026-10-20 a body whose last value is a dispatched call hands what it
/// files back to its entry (RFC-0005 §S5), so a recursion through a last
/// `if` no longer nests and never meets the floor.
#[test]
fn stats_counts_what_the_tier_one_floor_declined() {
    let dir = std::env::temp_dir().join(format!("bund2-declines-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let script = dir.join("declines.bund");
    std::fs::write(
        &script,
        ":down { 1 2 + drop 1 swap - dup 0 < { down } if 0 drop } register\n20001 down drop\n",
    )
    .expect("the program");

    let (floors, tier) = floors_of(&script, "1");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!floors.is_empty(), "`--stats` prints the floors line");
    if tier.is_empty() {
        assert!(
            !floors.contains("declined"),
            "a build with no tier has nothing to say about its floor: {floors}"
        );
        eprintln!(
            "exit_tier: no tier in this build, so the decline count was not read. \
             Run with `--features jit`."
        );
        return;
    }

    let entered = number_before(&tier, " entered)").expect("the entered figure");
    let declined = number_before(&floors, " entries declined at the tier 1 floor")
        .unwrap_or_else(|| panic!("the line counts no decline: {floors}"));
    let running = number_before(&floors, " compiled bodies running")
        .unwrap_or_else(|| panic!("the line does not say how deep the first was: {floors}"));
    assert!(entered > 0, "some levels ran compiled: {tier}");
    assert!(declined > 0, "and the floor turned the rest away: {floors}");
    assert_eq!(
        running, entered,
        "the first decline came with every compiled entry still running"
    );
    assert_eq!(
        entered + declined,
        20_000,
        "every entry after the one that compiled the body was one or the other: \
         {tier} / {floors}"
    );
}

/// **A program that does not parse still reports both floors** — criterion
/// 11's "each run reports both floors", which was false of
/// `cargo xtask depth`'s `nesting` axis: the run ended at the parser's
/// refusal, before `--stats` was reached. The floors are the process's.
#[test]
fn stats_names_both_floors_for_a_program_that_does_not_parse() {
    let dir = std::env::temp_dir().join(format!("bund2-floors-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let script = dir.join("unclosed.bund");
    std::fs::write(&script, "[ [ [ 1\n").expect("the program");

    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["script", "--stats", "--file"])
        .arg(&script)
        .output()
        .expect("bund2 runs");
    let _ = std::fs::remove_dir_all(&dir);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stdout.contains("unclosed block"),
        "the program must have failed to parse, or this is the test above: {stdout}"
    );
    let floors = stderr
        .lines()
        .find(|l| l.starts_with("bund2: floors: tier 0 at 0x"))
        .unwrap_or_else(|| panic!("no floors line after a parse failure: {stderr}"));
    assert!(floors.contains(", tier 1 at 0x"), "{floors}");
    assert!(
        stderr.contains("bund2: tier compiled 0 bodies") || stderr.contains("bund2: no tier"),
        "and the tier's line, which has nothing to count: {stderr}"
    );
}

/// The body every mirror row runs hot: a site, so that F136 lets it compile,
/// then `7 go`, where `go` is `drop` while the body warms and `exit` after.
const MIRROR_HOT: &str = "{ 1 2 + drop 7 go }";

/// `?try` around `body`, with the two branches a failing body would print.
fn tried(body: &str) -> String {
    format!(
        "?try :try {body} set :except {{ \"EXCEPT\" println }} set \
         :recovery {{ \"RECOVERY\" println }} set !"
    )
}

/// **Criterion 30's mirror rows, at process level: output and exit code** —
/// the twenty-fourth review's S9. The in-process rows
/// (`a_hot_body_under_a_native_reaches_exit_as_tier_zero_does`, in
/// `crates/bund2-runtime/src/tier.rs`) compare final stacks and cannot see
/// what was printed. These are the same seven programs run as a program is:
/// a compiled body whose caller is a native, reaching `exit` on an entry the
/// cache answers.
///
/// `m` prints `in` and runs the native over [`MIRROR_HOT`]. It is called
/// three times with `go` aliased to `drop`, which compiles the hot body on
/// its first entry and runs it compiled after that; then `go` becomes `exit`
/// and `m` is called once more. `unreached` must not print, neither branch of
/// a `?try` may print, and the code is 7.
///
/// **The entered figure is what says the exit ran compiled.** Each row states
/// how many compiled entries the whole run makes, the exiting one included:
/// one fewer and the exit was reached interpreted. `map` over two items
/// enters twice a call, and its last call ends at the first item.
#[test]
fn a_hot_body_under_a_native_prints_and_exits_as_without_the_tier() {
    let rows: [(&str, String, usize); 7] = [
        ("map over one item", format!("[ 1 ] {MIRROR_HOT} map"), 3),
        ("map over two items", format!("[ 1 2 ] {MIRROR_HOT} map"), 6),
        ("?try, directly", tried(MIRROR_HOT), 3),
        (
            "?try through map",
            tried(&format!("{{ [ 1 ] {MIRROR_HOT} map }}")),
            3,
        ),
        (
            "?try through times",
            tried(&format!("{{ 1 {MIRROR_HOT} times }}")),
            3,
        ),
        (
            "a drained body",
            tried(&format!("{{ {{ [ 1 ] {MIRROR_HOT} map }} ! }}")),
            3,
        ),
        (
            "a residual-path value",
            tried(&format!(
                "{{ :scratch to_stack {{ [ 1 ] {MIRROR_HOT} map }} ! :main to_stack }}"
            )),
            3,
        ),
    ];

    let dir = std::env::temp_dir().join(format!("bund2-exit-mirror-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let mut tier_seen = false;
    for (label, body, entered) in rows {
        let script = dir.join("mirror.bund");
        std::fs::write(
            &script,
            format!(
                ":drop :go alias\n:m {{ \"in\" println {body} }} register\nm m m\n\
                 :exit :go alias\nm\n\"unreached\" println\n"
            ),
        )
        .expect("script");

        let (tiered, tiered_code, tiered_stats) = run_at(&script, "1");
        let (plain, plain_code, plain_stats) = run_at(&script, "1000000");

        assert_eq!(
            plain_code,
            Some(7),
            "{label}: the code the program asked for"
        );
        assert_eq!(
            plain, "in\nin\nin\nin\n",
            "{label}: four calls, nothing after the exit, and neither `?try` branch"
        );
        assert_eq!(tiered_code, plain_code, "{label}: exit code");
        assert_eq!(tiered, plain, "{label}: output");

        if tiered_stats.contains("no tier") {
            continue;
        }
        tier_seen = true;
        assert!(
            tiered_stats.contains(&format!("compiled 1 bodies ({entered} entered)")),
            "{label}: the hot body must compile on its first entry and run compiled \
             on every later one, the exiting one included: {tiered_stats:?}"
        );
        assert!(
            plain_stats.contains("compiled 0 bodies"),
            "{label}: the control must compile nothing: {plain_stats:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
    if !tier_seen {
        eprintln!(
            "exit_tier: no tier in this build, so the mirror rows checked output and \
             exit code only. Run with `--features jit` for the comparison."
        );
    }
}

/// **Criterion 30's `input*` mirror, with the harness the criterion
/// specifies**: "the test writes one line to the process's stdin and holds
/// the pipe open. It asserts that the process exits with code 7 before a
/// second line is written." The in-process row feeds its lines through
/// `Vm::feed_line`, where nothing can block.
///
/// The body has to be hot when it exits, and `input*` runs it once a line
/// inside one call, so the lines do the warming: three `a`s, on which `go`
/// is `drop`, and then an `x`, on which the body re-aliases `go` to `exit`
/// before it reaches it. The re-aliasing is in a lambda of its own, which is
/// a different body and stays cold. So "one line" is the fourth; the pipe is
/// held open after it and nothing more is written.
///
/// Run at both thresholds, and compared: the lines echoed, the code, and
/// that the process ended while its input was still open.
#[test]
fn a_hot_input_loop_body_exits_with_the_pipe_held_open() {
    use std::io::{Read, Write};
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let dir = std::env::temp_dir().join(format!("bund2-exit-input-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let script = dir.join("input-hot.bund");
    std::fs::write(
        &script,
        ":drop :go alias\n\
         \"p> \" { dup println \"x\" == { :exit :go alias } if 1 2 + drop 7 go } input*\n\
         \"unreached\" println\n",
    )
    .expect("script");

    let run = |threshold: &str| -> (String, Option<i32>, String) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_bund2"))
            .args(["script", "--stats", "--jit-threshold", threshold, "--file"])
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("bund2 starts");
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(b"a\na\na\nx\n").expect("the lines");
        stdin.flush().expect("flush");

        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            if let Some(s) = child.try_wait().expect("wait") {
                break Some(s);
            }
            if Instant::now() > deadline {
                break None;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let Some(status) = status else {
            let _ = child.kill();
            panic!("threshold {threshold}: bund2 was still reading input after the exit");
        };
        // Only now: the process ended with its input open.
        drop(stdin);

        let mut out = String::new();
        child
            .stdout
            .take()
            .expect("stdout")
            .read_to_string(&mut out)
            .expect("reads");
        let mut err = String::new();
        child
            .stderr
            .take()
            .expect("stderr")
            .read_to_string(&mut err)
            .expect("reads");
        let stats = err
            .lines()
            .find(|l| l.contains("tier compiled") || l.contains("no tier"))
            .unwrap_or_default()
            .to_string();
        (out, status.code(), stats)
    };

    let (tiered, tiered_code, tiered_stats) = run("1");
    let (plain, plain_code, plain_stats) = run("1000000");
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(plain_code, Some(7), "the code the body asked for");
    assert_eq!(
        plain, "a\na\na\nx\n",
        "four lines reached the body and nothing followed"
    );
    assert_eq!(tiered_code, plain_code, "exit code");
    assert_eq!(tiered, plain, "output");

    if tiered_stats.contains("no tier") {
        eprintln!(
            "exit_tier: no tier in this build, so the `input*` mirror checked output, \
             exit code and the open pipe only. Run with `--features jit`."
        );
        return;
    }
    assert!(
        tiered_stats.contains("compiled 1 bodies (3 entered)"),
        "the body compiles on the first line and runs compiled on the next three, \
         the last of them into the exit: {tiered_stats:?}"
    );
    assert!(
        plain_stats.contains("compiled 0 bodies"),
        "the control must compile nothing: {plain_stats:?}"
    );
}
