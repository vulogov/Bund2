//! A failure inside a compiled body is reported as Tier 0 reports it — D173's
//! traceback, with the tier on (D218's first tier finding).
//!
//! `{ < 100 }` compiles: `<` is an inlined site guarded on two integers. Under
//! `map`, after `{ 5 3 } execute` has left two integers beneath the items, the
//! guard admits the first items and then meets a boolean, so compiled code
//! calls the native `<`, which fails. Tier 0 notes that failure in `run_to`
//! while the body's frame is still there, and its report names the value
//! inside the body, the innermost message and a traceback. The tier's frame
//! (D139) was removed before anything noted it, so the report fell back to
//! `map`'s place and the message with every wrapper.
//!
//! **One binary, two thresholds**, as `exit_tier.rs`: 1 compiles every body on
//! its first evaluation, and 1,000,000 compiles none.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

const PROGRAM: &str = "{ 5 3 } execute\n[ 4 6 5 ] { < 100 } map\n";

/// `(stdout with the stats line removed, the stats line)`.
fn run_at(script: &std::path::Path, threshold: &str) -> (String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args([
            "script",
            "--report",
            "trace",
            "--stats",
            "--jit-threshold",
            threshold,
            "--file",
        ])
        .arg(script)
        .output()
        .expect("bund2 runs");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let stats = stdout
        .lines()
        .chain(stderr.lines())
        .find(|l| l.contains("tier compiled") || l.contains("no tier"))
        .unwrap_or_default()
        .to_string();
    let report = stdout
        .lines()
        .filter(|l| !l.contains("tier compiled"))
        .collect::<Vec<_>>()
        .join("\n");
    (report, stats)
}

#[test]
fn a_failure_inside_a_compiled_body_is_reported_as_tier_0_reports_it() {
    let dir = std::env::temp_dir().join(format!("bund2-report-tier-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("p.bund");
    std::fs::write(&script, PROGRAM).unwrap();

    let (plain, _) = run_at(&script, "1000000");
    let (tiered, stats) = run_at(&script, "1");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        plain.contains("Error: < returns error: COMPARE: unsupported operand #2"),
        "Tier 0 names the innermost failure: {plain}"
    );
    assert!(
        plain.contains("the value that failed, inside"),
        "Tier 0 places it inside the body: {plain}"
    );
    if stats.contains("no tier") {
        // A default build: the half above is all there is to compare.
        return;
    }
    assert!(
        stats.contains("(1 entered)"),
        "the failing body must run compiled, or this compares Tier 0 with itself: {stats}"
    );
    assert_eq!(tiered, plain, "the report, with the tier and without");
}
