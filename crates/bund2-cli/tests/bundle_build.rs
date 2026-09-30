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

/// Re-sign after damaging, or macOS kills the artefact before the runtime gets
/// to report anything — Q40. The damage is the subject of the test; the
/// signature is not.
fn reseal(path: &PathBuf) {
    #[cfg(target_os = "macos")]
    {
        let r = Command::new("/usr/bin/codesign")
            .args(["-f", "-s", "-"])
            .arg(path)
            .output()
            .expect("codesign runs");
        assert!(r.status.success(), "re-signing the damaged artefact");
    }
    #[cfg(not(target_os = "macos"))]
    let _ = path;
}

/// How far the `state` byte sits before the payload, from the container's
/// layout: `at::PAYLOAD` is 332 and `at::STATE` is 36.
const STATE_BEFORE_PAYLOAD: usize = 332 - 36;
/// And the length field: `at::PAYLOAD` minus `at::LEN` (40).
const LEN_BEFORE_PAYLOAD: usize = 332 - 40;

/// Find the payload by its own text, so the test needs none of the builder's
/// internals. The guard on the `state` byte is what makes the offsets above
/// safe: if the layout moves, this fails rather than patching something else.
fn payload_at(image: &[u8], marker: &str) -> usize {
    let m = marker.as_bytes();
    let at = image
        .windows(m.len())
        .position(|w| w == m)
        .expect("the program is in the artefact");
    assert_eq!(
        image[at - STATE_BEFORE_PAYLOAD], 1,
        "the state byte must read FILLED at the offset this test assumes; \
         if the container's layout changed, these constants must follow"
    );
    at
}

/// **Criterion 12: a damaged artefact is refused, not aborted.** Four cases,
/// each an error with an explanation, none a panic — D37.
#[test]
fn a_damaged_artefact_is_refused_with_an_explanation() {
    let marker = "\"unique-marker-for-locating\" println\n";
    /// One case: a name, the damage to apply, and what the report must say.
    /// Named because the tuple is otherwise unreadable at the call site.
    type Case = (&'static str, Box<dyn Fn(&mut Vec<u8>, usize)>, &'static str);
    let cases: [Case; 3] = [
        (
            "state",
            Box::new(|img, at| img[at - STATE_BEFORE_PAYLOAD] = 7),
            "neither empty",
        ),
        (
            "length",
            // 1 MiB + 1, little-endian, past the region's capacity.
            Box::new(|img, at| {
                img[at - LEN_BEFORE_PAYLOAD..at - LEN_BEFORE_PAYLOAD + 4]
                    .copy_from_slice(&(1024u32 * 1024 + 1).to_le_bytes());
            }),
            "at most",
        ),
        (
            "not-text",
            // A lone continuation byte is never valid UTF-8.
            Box::new(|img, at| img[at] = 0x80),
            "UTF-8",
        ),
    ];

    for (name, damage, needle) in cases {
        let exe = build(marker, &format!("dmg-{name}"), &[]);
        let mut image = std::fs::read(&exe).expect("reading the artefact");
        let at = payload_at(&image, marker);
        damage(&mut image, at);
        std::fs::write(&exe, &image).expect("writing it back");
        reseal(&exe);

        let (code, out) = run(&exe, &[], &[]);
        assert_eq!(
            code,
            Some(1),
            "{name}: a damaged artefact fails, and does not abort. Output: {out}"
        );
        assert!(
            out.contains(needle),
            "{name}: the report must say what was wrong — looked for {needle:?} in {out}"
        );
        assert!(
            !out.contains("panicked"),
            "{name}: and must not panic: {out}"
        );
        let _ = std::fs::remove_file(&exe);
    }
}

/// **Criterion 5: a program nesting to `MAX_NESTING` bundles and runs.** 1024,
/// the parser's limit — not `MAX_WIRE_DEPTH`'s 256, which does not apply
/// because nothing is wire-encoded (§B2).
#[test]
fn a_program_nested_to_the_parsers_limit_bundles_and_runs() {
    let n = 1024;
    let src = format!("{}1 {}\ndrop\n\"deep\" println\n", "{ ".repeat(n), "} ".repeat(n));
    let exe = build(&src, "deep", &[]);
    let (code, out) = run(&exe, &[], &[]);
    assert_eq!(code, Some(0), "output was: {out}");
    assert!(out.contains("deep"), "it ran: {out}");
    let _ = std::fs::remove_file(&exe);

    // And one level further is a *build* error, not a run-time surprise.
    let s = scratch("deeper.bund");
    let deeper = format!("{}1 {}\n", "{ ".repeat(n + 1), "} ".repeat(n + 1));
    std::fs::write(&s, &deeper).expect("writing the source");
    let out2 = scratch("deeper");
    let _ = std::fs::remove_file(&out2);
    let r = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["build", "--file"])
        .arg(&s)
        .arg("--output")
        .arg(&out2)
        .output()
        .expect("bund2 build runs");
    assert!(!r.status.success(), "1025 levels must fail the build");
    assert!(!out2.exists(), "and write nothing");
}

/// **Criterion 3: the default artefact carries no code generator.**
///
/// Over the artefact `bund2 build` actually wrote, not over a crate graph —
/// `cargo tree` on a stub passed before any bundle existed, which is why the
/// criterion was rewritten.
#[cfg(not(feature = "jit"))]
#[test]
fn a_default_artefact_contains_no_code_generator() {
    let exe = build("1 2 + println\n", "nojit", &[]);
    let image = std::fs::read(&exe).expect("reading the artefact");
    for needle in [b"cranelift".as_slice(), b"ISLE".as_slice()] {
        assert!(
            !image.windows(needle.len()).any(|w| w == needle),
            "a default bundle must not carry {}",
            String::from_utf8_lossy(needle)
        );
    }
    let _ = std::fs::remove_file(&exe);
}

/// **Criterion 6: a JIT bundle declares its Tier 1 share, so compiled code
/// actually runs.**
///
/// The figure a broken front end leaves at zero while every other figure
/// reports success — RFC-0005's `compiled_entries`, and the reason it exists.
#[cfg(feature = "jit")]
#[test]
fn a_jit_bundle_enters_compiled_code() {
    // One body, entered past the threshold the environment sets. Written out
    // rather than looped: a loop word's operand order is one more thing that
    // could fail here, and what this test is about is the share.
    let src = format!(
        ":w {{ 1 2 + drop }} register\n{}\n",
        "w ".repeat(40)
    );
    let src = src.as_str();
    let exe = build(src, "jit", &[]);
    let (code, out) = run(
        &exe,
        &[],
        &[("BUND2_JIT_THRESHOLD", "2"), ("BUND2_STATS", "1")],
    );
    assert_eq!(code, Some(0), "output was: {out}");
    assert!(
        out.contains("tier compiled"),
        "BUND2_STATS must reach a bundle: {out}"
    );
    let entered = out
        .split("entered")
        .next()
        .and_then(|s| s.rsplit('(').next().map(str::to_string))
        .unwrap_or_default();
    let n: usize = entered.trim().parse().unwrap_or(0);
    assert!(
        n > 0,
        "compiled code must have run in a bundle, not merely been compiled. \
         A front end that spawns no thread or leaves the share undeclared puts \
         the Tier 1 floor above the thread's top and every body declines, while \
         every other figure reports success. Stats line: {out}"
    );
    let _ = std::fs::remove_file(&exe);
}
