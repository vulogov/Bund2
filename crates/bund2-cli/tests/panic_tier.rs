//! RFC-0005 criterion 29, at process level: a native that panics, called from
//! a compiled body, gives what Tier 0 gives — on **both streams**.
//!
//! The in-process tests (`a_panicking_native_in_a_non_tail_position_matches_tier_zero`
//! and its tail twin, `crates/bund2-jit/src/lower.rs`) compare the error's
//! text with a constant. The criterion also says "reported through the
//! reporter, with nothing on stderr and no abort", and nothing in process can
//! see a stream: a panic that reached the default hook would print there and
//! the test would still pass (the twenty-third review's S10).
//!
//! The panic is a real one, F95's: `string.distance.jarowinkler` panics inside
//! the `natural` crate for these operands, so no test native is needed and the
//! shipped binary is what runs. Both configurations are one binary, chosen by
//! `--jit-threshold`, as in `exit_tier.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

/// `w` calls `go`, which is `drop` while `w` is entered twice — the first
/// entry compiles it, the second runs it compiled — and the panicking word on
/// the third. `1 2 + drop` gives the body a site, so it compiles (F136).
/// `tail` is what follows the call inside the body: something, or nothing.
fn program(tail: &str) -> String {
    format!(
        ":drop :go alias\n\
         :w {{ \"tick\" println 1 2 + drop \"zz_nofile\" \"A\" go {tail} }} register\n\
         w w\n\
         :string.distance.jarowinkler :go alias\n\
         w \"unreached\" println\n"
    )
}

/// `(stdout, stderr without the stats line, exit code, stats line)`.
fn run_at(script: &std::path::Path, threshold: &str) -> (String, String, Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["script", "--stats", "--jit-threshold", threshold, "--file"])
        .arg(script)
        .output()
        .expect("bund2 runs");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let is_stats = |l: &&str| l.contains("tier compiled") || l.contains("no tier");
    // `--stats` prints a second line, the two stack floors, whose addresses
    // differ from run to run (D147). It is no part of what is compared.
    let is_floors = |l: &&str| l.starts_with("bund2: floors:");
    let stats = stderr.lines().find(is_stats).unwrap_or_default().to_string();
    let rest: String = stderr
        .lines()
        .filter(|l| !is_stats(l) && !is_floors(l))
        .map(|l| format!("{l}\n"))
        .collect();
    (stdout, rest, out.status.code(), stats)
}

#[test]
fn a_panicking_native_under_a_compiled_body_matches_without_the_tier() {
    let dir = std::env::temp_dir().join(format!("bund2-panic-tier-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");

    for (label, tail) in [("non-tail", "\"after\" println"), ("tail", "")] {
        let script = dir.join(format!("panic-{label}.bund"));
        std::fs::write(&script, program(tail)).expect("script");

        let (out, err, code, stats) = run_at(&script, "1");
        let (plain_out, plain_err, plain_code, _) = run_at(&script, "1000000");

        // What Tier 0 does, stated once so the comparison below has a subject.
        assert!(
            plain_out.contains("internal error: native `string.distance.jarowinkler` panicked"),
            "{label}: reported through the reporter, naming the native: {plain_out}"
        );
        // By whole line: the report quotes the source line that failed.
        assert!(
            !plain_out.lines().any(|l| l == "unreached"),
            "{label}: the program stopped there: {plain_out}"
        );
        assert_eq!(plain_out.matches("tick").count(), 3, "{label}: three entries: {plain_out}");
        assert_eq!(plain_err, "", "{label}: nothing on stderr, the panic hook's text included");
        assert_eq!(plain_code, Some(0), "{label}: a reported error, not an abort");

        assert_eq!(out, plain_out, "{label}: stdout");
        assert_eq!(err, plain_err, "{label}: stderr");
        assert_eq!(code, plain_code, "{label}: exit code");

        if stats.contains("no tier") {
            eprintln!(
                "panic_tier: no tier in this build, so the {label} case compared one \
                 interpreter with itself. Run with `--features jit` for the comparison."
            );
            continue;
        }
        assert!(
            stats.contains("compiled 1 bodies (2 entered)"),
            "{label}: `w` compiles on its first entry and runs compiled on the next \
             two, the second of them into the panic: {stats:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
