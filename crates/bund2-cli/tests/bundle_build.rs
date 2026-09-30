//! `bund2 build --emit=bundle`, end to end — RFC-0006 §B1, §B3 and §B3a.
//!
//! **Through the binary, not the module.** `bundle`'s unit tests cover the
//! container; what these cover is the part no unit test can reach: that the
//! artefact a build produces **executes**, and that D78's floor survives the
//! environment.
//!
//! Q40's measurements are why the second of those matters here. On macOS an
//! in-place edit invalidates the signature covering those bytes, and the
//! artefact is **killed with SIGKILL** rather than merely failing validation,
//! so `bund2 build` re-signs it. If that step regresses, every test below
//! fails at once with no output — which is the signature of the bug rather
//! than of a program that printed nothing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("bund2-bundle-{}-{name}", std::process::id()));
    p
}

/// Build `src` into an artefact and return its path.
fn build(src: &str, name: &str, extra: &[&str]) -> PathBuf {
    let s = scratch(&format!("{name}.bund"));
    std::fs::write(&s, src).expect("writing the source");
    let out = scratch(name);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bund2"));
    cmd.arg("build")
        .arg("--file")
        .arg(&s)
        .arg("--output")
        .arg(&out);
    for e in extra {
        cmd.arg(e);
    }
    let r = cmd.output().expect("bund2 build runs");
    assert!(
        r.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&r.stderr)
    );
    out
}

fn run(exe: &PathBuf, args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, String) {
    let mut cmd = Command::new(exe);
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let r = cmd.output().expect("the artefact runs");
    let mut s = String::from_utf8_lossy(&r.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&r.stderr));
    (r.status.code(), s)
}

/// **An artefact runs the program it was built from.**
#[test]
fn a_bundle_runs_its_program() {
    let exe = build("\"carried\" println\n1 2 + println\n", "runs", &[]);
    let (code, out) = run(&exe, &[], &[]);
    assert_eq!(code, Some(0), "output was: {out}");
    assert!(out.contains("carried"), "the program ran: {out}");
    assert!(out.contains('3'), "and produced its answer: {out}");
    let _ = std::fs::remove_file(&exe);
}

/// **All of argv reaches the program** — §B3. The runner consumes nothing, so
/// a program whose first argument is one of the CLI's own subcommands still
/// sees it.
#[test]
fn every_argument_reaches_the_program_including_words() {
    let exe = build("args println\n", "argv", &[]);
    let (code, out) = run(&exe, &["words", "--stats"], &[]);
    assert_eq!(code, Some(0), "output was: {out}");
    assert!(
        out.contains("words") && out.contains("--stats"),
        "argv belongs to the program, not to the runner: {out}"
    );
    let _ = std::fs::remove_file(&exe);
}

/// **D78: the trailer is a floor and the environment may only tighten it.**
///
/// Four rows, and the third is the one that would otherwise be silent — an
/// artefact that reported itself built `--noeval` while evaluating everything.
#[test]
fn a_restriction_cannot_be_cleared_by_the_environment() {
    let prog = "\"1 2 +\" bund.eval println\n";
    let open = build(prog, "open", &[]);
    let shut = build(prog, "shut", &["--noeval"]);

    let (_, a) = run(&open, &[], &[]);
    assert!(a.contains('3'), "unrestricted, it evaluates: {a}");

    let (_, b) = run(&shut, &[], &[]);
    assert!(
        b.contains("disabled with --noeval"),
        "built --noeval, it refuses: {b}"
    );

    for clearing in [("BUND2_NOEVAL", "0"), ("BUND2_NOEVAL", "")] {
        let (_, c) = run(&shut, &[], &[clearing]);
        assert!(
            c.contains("disabled with --noeval"),
            "{}={} must not clear the floor: {c}",
            clearing.0,
            clearing.1
        );
    }

    let (_, d) = run(&open, &[], &[("BUND2_NOEVAL", "1")]);
    assert!(
        d.contains("disabled with --noeval"),
        "and run time may still add one: {d}"
    );

    let _ = std::fs::remove_file(&open);
    let _ = std::fs::remove_file(&shut);
}

/// **A program that does not parse is a build error** — §B3. It belongs to
/// whoever built the artefact, not to whoever ran it, and nothing is written.
#[test]
fn a_syntax_error_fails_the_build_and_writes_nothing() {
    let s = scratch("bad.bund");
    std::fs::write(&s, "{ 1 2 +\n").expect("writing the source");
    let out = scratch("bad");
    let _ = std::fs::remove_file(&out);
    let r = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["build", "--file"])
        .arg(&s)
        .arg("--output")
        .arg(&out)
        .output()
        .expect("bund2 build runs");
    assert!(!r.status.success(), "the build must fail");
    let err = String::from_utf8_lossy(&r.stderr);
    assert!(
        err.contains("nothing was written"),
        "and say so: {err}"
    );
    assert!(!out.exists(), "no artefact is left behind");
}

/// **The artefact still validates where signing is enforced** — criterion 11,
/// and Q40's whole point.
#[cfg(target_os = "macos")]
#[test]
fn a_built_artefact_validates() {
    let exe = build("1 2 + println\n", "signed", &[]);
    let r = Command::new("/usr/bin/codesign")
        .arg("-v")
        .arg(&exe)
        .output()
        .expect("codesign runs");
    assert!(
        r.status.success(),
        "an in-place edit is SIGKILLed until re-signed, so the build re-signs: {}",
        String::from_utf8_lossy(&r.stderr)
    );
    let _ = std::fs::remove_file(&exe);
}
