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
/// layout: `at::PAYLOAD` minus `at::STATE` (36).
///
/// **These moved once already, and the guard below caught it.** Adding the
/// `features` and `pinned` fields took `at::PAYLOAD` from 332 to 652; the
/// assertion in `payload_at` failed immediately rather than letting the test
/// patch 320 bytes into the middle of the header. That is the whole reason it
/// is there — an integration test cannot import the container's own offsets,
/// so the next best thing is to fail loudly when its copy is stale.
const STATE_BEFORE_PAYLOAD: usize = 652 - 36;
/// And the length field: `at::PAYLOAD` minus `at::LEN` (40).
const LEN_BEFORE_PAYLOAD: usize = 652 - 40;

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

/// **`--output` may not be the binary doing the building.**
///
/// It used to succeed: the interpreter was replaced by the artefact, the
/// success message said nothing, and afterwards `bund2 --file x.bund` ran the
/// embedded program and ignored the argument — because a bundle gives all of
/// argv to its program. Destructive, silent, and one keystroke from `-o` on
/// the wrong path.
#[test]
fn a_build_refuses_to_overwrite_the_binary_doing_it() {
    let copy = scratch("self-target");
    std::fs::copy(env!("CARGO_BIN_EXE_bund2"), &copy).expect("copying the binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(&copy).expect("mode").permissions();
        p.set_mode(p.mode() | 0o111);
        std::fs::set_permissions(&copy, p).expect("setting the mode");
    }
    let src = scratch("self-target.bund");
    std::fs::write(&src, "1 println\n").expect("writing the source");

    let r = Command::new(&copy)
        .arg("build")
        .arg("--file")
        .arg(&src)
        .arg("--output")
        .arg(&copy)
        .output()
        .expect("it runs");
    assert!(!r.status.success(), "the build must refuse");
    let err = String::from_utf8_lossy(&r.stderr);
    assert!(
        err.contains("nothing was written") || err.contains("Nothing was written"),
        "and say so: {err}"
    );

    // And the binary is still an interpreter, not an artefact.
    let after = Command::new(&copy)
        .arg("--file")
        .arg(&src)
        .output()
        .expect("it still runs");
    assert!(
        String::from_utf8_lossy(&after.stdout).contains('1'),
        "it must still interpret a file given on the command line"
    );
    let _ = std::fs::remove_file(&copy);
}

/// **A bundle cannot be used as a builder**, because `carried()` is consulted
/// before the `build` arm — all of argv belongs to the program (§B3).
///
/// True by construction and pinned here, because it is the kind of property
/// that would change quietly if the arms were reordered.
#[test]
fn a_bundle_cannot_build_another_bundle() {
    let exe = build("1 println\n", "notabuilder", &[]);
    let out = scratch("second");
    let _ = std::fs::remove_file(&out);
    let src = scratch("notabuilder.bund");
    let (code, printed) = run(
        &exe,
        &["build", "--file", src.to_str().unwrap_or(""), "--output", out.to_str().unwrap_or("")],
        &[],
    );
    assert_eq!(code, Some(0), "it runs its own program: {printed}");
    assert!(printed.contains('1'), "which prints 1: {printed}");
    assert!(!out.exists(), "and builds nothing");
    let _ = std::fs::remove_file(&exe);
}

/// **An empty program is still a program.** `state` is filled with a length of
/// zero, so the artefact must run nothing and **must not** fall back to the
/// CLI — which would parse the program's own arguments as flags.
#[test]
fn an_empty_program_does_not_become_the_interpreter() {
    let exe = build("", "emptyprog", &[]);
    let (code, out) = run(&exe, &["words", "--stats"], &[]);
    assert_eq!(code, Some(0), "output was: {out}");
    assert!(
        out.trim().is_empty(),
        "an empty program prints nothing, and in particular not the word table \
         that `words` would have printed had argv reached the runner: {out}"
    );
    let _ = std::fs::remove_file(&exe);
}

/// **An exit code travels** — D52 through a bundle, matching a source run.
#[test]
fn an_exit_code_matches_a_source_run() {
    let src = scratch("exitcode.bund");
    std::fs::write(&src, "7 bund.exit\n").expect("writing the source");
    let source = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .arg("--file")
        .arg(&src)
        .output()
        .expect("the source run");
    let exe = build("7 bund.exit\n", "exitcode", &[]);
    let (code, _) = run(&exe, &[], &[]);
    assert_eq!(
        code,
        source.status.code(),
        "a bundle's exit code must equal its source run's"
    );
    assert_eq!(code, Some(7), "and be the code the program asked for");
    let _ = std::fs::remove_file(&exe);
}

/// **Criterion 4: a bundle is produced with no compiler on the path.**
///
/// D10's load-bearing half — "nothing below `bund2 build` may require `cc`" —
/// and the criterion originally said to check it by building in an environment
/// without `cc`. This is stronger and needs no such environment: the build runs
/// with the environment **cleared and `PATH` empty**, so neither `cc` nor
/// `rustc` nor anything else is reachable by name. It still has to succeed.
///
/// D81 permits exactly one platform tool, and this is also the test that pins
/// the permission's shape: `/usr/bin/codesign` is invoked by **absolute path**,
/// so an empty `PATH` cannot reach it and cannot hide a dependency on it
/// either. If the re-sign step were ever changed to `Command::new("codesign")`,
/// this test fails on macOS.
#[test]
fn a_bundle_is_built_with_no_compiler_reachable() {
    let src = scratch("nopath.bund");
    std::fs::write(&src, "\"built without a toolchain\" println\n").expect("writing the source");
    let out = scratch("nopath");
    let _ = std::fs::remove_file(&out);

    let r = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .env_clear()
        .env("PATH", "")
        .arg("build")
        .arg("--file")
        .arg(&src)
        .arg("--output")
        .arg(&out)
        .output()
        .expect("bund2 build runs");
    assert!(
        r.status.success(),
        "the build must not need anything on PATH: {}",
        String::from_utf8_lossy(&r.stderr)
    );

    // And what it produced runs, also with nothing on the path.
    let ran = Command::new(&out)
        .env_clear()
        .env("PATH", "")
        .output()
        .expect("the artefact runs");
    assert_eq!(ran.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&ran.stdout).contains("built without a toolchain"),
        "and prints its program's output"
    );
    let _ = std::fs::remove_file(&out);
}

/// **Criterion 9: a literal's stamp is a run-time value, not the build's.**
///
/// This is the criterion that pins §B2's reason for existing. An encoded
/// payload would have materialised every literal's stamp at build time (D20),
/// so every run of the artefact would report the same one — the failure D77
/// turns on. Source text cannot do that, and this is what says so.
///
/// Observed through `debug.display_stack`, since `.timestamp` is not among the
/// words Bund2 implements yet. The stamp is sampled when a value is first
/// observed, which is D2's ruling, so the dump is the observation.
#[test]
fn a_literals_stamp_is_taken_at_run_time_not_at_build_time() {
    let now_ms = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    };
    let stamp_of = |out: &str| -> u128 {
        let at = out.find("stamp: ").expect("the dump carries a stamp");
        let rest = &out[at + "stamp: ".len()..];
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        rest[..end].parse().unwrap_or(0)
    };

    let built_at = now_ms();
    let exe = build("1 debug.display_stack\n", "stamp", &[]);

    let mut seen = Vec::new();
    for _ in 0..2 {
        // Enough that two millisecond-resolution stamps cannot collide.
        std::thread::sleep(std::time::Duration::from_millis(30));
        let before = now_ms();
        let (code, out) = run(&exe, &[], &[("BUND2_RAW_VALUES", "1")]);
        let after = now_ms();
        assert_eq!(code, Some(0), "output was: {out}");
        let s = stamp_of(&out);
        assert!(
            s >= before && s <= after,
            "the stamp {s} must fall inside this run's window [{before}, {after}] — \
             that is what makes it a run-time value"
        );
        assert!(
            s > built_at,
            "and it must be later than the build at {built_at}, which an encoded \
             payload would have frozen it to"
        );
        seen.push(s);
    }
    assert_ne!(
        seen[0], seen[1],
        "two runs of one artefact must report different stamps for the same \
         literal; equal stamps are the encoded-stream failure D77 rejected"
    );
    let _ = std::fs::remove_file(&exe);
}

/// **D78's readable trailer** — `bund2 build --inspect`.
///
/// "A restriction the runner cannot observe is one they cannot rely on", and
/// criterion 10 recorded this half as unbuilt until now. Three properties are
/// asserted together because each is useless alone: the restriction is shown,
/// the feature set is shown (§B4: it decides which words exist), and the
/// oracle the meaning was fixed against is shown (§B1).
#[test]
fn an_artefact_describes_itself() {
    let exe = build("1 println\n", "inspectme", &["--noeval"]);
    let r = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["build", "--inspect"])
        .arg(&exe)
        .output()
        .expect("inspect runs");
    assert!(r.status.success());
    let out = String::from_utf8_lossy(&r.stdout);

    assert!(out.contains("--noeval"), "the restriction is visible: {out}");
    assert!(
        out.contains("of 1048576 bytes"),
        "the capacity is visible, so a size refusal is predictable: {out}"
    );
    assert!(
        out.contains("inspectme.bund"),
        "the source it was built from is visible: {out}"
    );
    // **The oracle, by submodule name and short SHA.** Not merely non-empty:
    // the first version of the summary parsed `PINNED.txt`'s trailing human
    // line as a submodule and emitted `0.22.0,:bund`, so this checks a real
    // entry is there and that stray one is not.
    assert!(
        out.contains("Bund:") && out.contains("rust_dynamic:"),
        "the pinned oracle is named: {out}"
    );
    assert!(
        !out.contains("0.22.0,:"),
        "and PINNED.txt's version line is not parsed as a submodule: {out}"
    );
    assert!(
        out.contains("a floor"),
        "and the floor's direction is stated, or the line above reads as the \
         whole truth: {out}"
    );
    let _ = std::fs::remove_file(&exe);
}

/// **An unrestricted artefact says "none" rather than omitting the line.**
///
/// A missing line and "restrictions none" say different things to someone
/// deciding whether to trust an artefact.
#[test]
fn an_unrestricted_artefact_says_so() {
    let exe = build("1 println\n", "unrestricted", &[]);
    let r = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["build", "--inspect"])
        .arg(&exe)
        .output()
        .expect("inspect runs");
    let out = String::from_utf8_lossy(&r.stdout);
    assert!(
        out.contains("restrictions   none"),
        "an unrestricted artefact must say so: {out}"
    );
    let _ = std::fs::remove_file(&exe);
}

/// **Inspecting something that is not an artefact fails with a reason**, not a
/// panic and not a misleading empty report.
#[test]
fn inspecting_a_non_artefact_explains_itself() {
    let f = scratch("notanartefact");
    std::fs::write(&f, b"this is not a mach-o").expect("writing the file");
    let r = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["build", "--inspect"])
        .arg(&f)
        .output()
        .expect("inspect runs");
    assert!(!r.status.success());
    let err = String::from_utf8_lossy(&r.stderr);
    assert!(
        err.contains("not a bund2 artefact"),
        "and say what it looked for: {err}"
    );
    assert!(!err.contains("panicked"), "without panicking: {err}");
}

/// **A bundle's debugger words do nothing — D115.**
///
/// In a `script` run the first arming or moving word attaches a console and
/// the program stops at it (D113.5). A bundle is a shipped program: one that
/// stopped at a prompt would wait there for as long as its input stayed open.
/// So the same words in a bundle neither stop nor refuse.
///
/// **Standard input is held open with a line in it.** A console that did
/// attach would run that line and print `typed`; with input closed it would
/// detach and the test could not tell the two apart.
#[test]
fn a_bundles_debugger_words_do_nothing() {
    use std::io::Write;
    use std::process::Stdio;

    let exe = build(
        "\"a\" println debug.step \"b\" println \
         \"println\" debug.break \"c\" println \
         \"1 2 + println\" debug.run debug.next debug.finish debug.continue \
         \"d\" println",
        "inert-debugger",
        &[],
    );
    let mut child = Command::new(&exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the artefact runs");
    let mut stdin = child.stdin.take().expect("stdin");
    stdin.write_all(b"\"typed\" println\n").expect("a line to run");
    stdin.flush().expect("flush");
    // Held, not dropped: the program has to end with its input still open.
    let r = child.wait_with_output_keeping(stdin);
    assert_eq!(r.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&r.stdout), "a\nb\nc\n3\nd\n");
    assert_eq!(String::from_utf8_lossy(&r.stderr), "");
}

/// `wait_with_output`, with the child's standard input kept open until the
/// child has ended.
trait KeepInput {
    fn wait_with_output_keeping(self, stdin: std::process::ChildStdin) -> std::process::Output;
}

impl KeepInput for std::process::Child {
    fn wait_with_output_keeping(mut self, stdin: std::process::ChildStdin) -> std::process::Output {
        use std::io::Read;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let status = loop {
            if let Some(s) = self.try_wait().expect("wait") {
                break s;
            }
            if std::time::Instant::now() > deadline {
                let _ = self.kill();
                panic!("the artefact was still running: it is waiting at a prompt");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        drop(stdin);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        self.stdout.take().expect("stdout").read_to_end(&mut stdout).expect("stdout");
        self.stderr.take().expect("stderr").read_to_end(&mut stderr).expect("stderr");
        std::process::Output { status, stdout, stderr }
    }
}
