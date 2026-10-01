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
/// 2 sites, promotes 2 values and crosses 2 calls.
const PROGRAM: &str = "3 { \"tick\" println 1 2 + drop true { \"bye\" println 7 exit } if \"after if\" println } times\n";

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
