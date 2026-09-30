//! `cargo xtask bundle` — RFC-0006's artefact, checked **in both profiles**.
//!
//! **Why this exists.** `--emit=bundle` was built with eleven passing tests and
//! could not work in release. Two blockers, both invisible to `cargo test`
//! because tests are compiled in debug:
//!
//! 1. The sentinel that locates the payload region was argued to occur exactly
//!    once. Release const-folds the `const fn` that unmasked it, so the plain
//!    bytes appeared in the code as well, and `bund2 build` refused every
//!    release bundle. Fixed by identifying the region structurally — a
//!    sentinel followed by a header that validates — rather than by assuming
//!    the literal is unique.
//! 2. The runtime read its payload from an immutable `static`, which a
//!    compiler may fold to the initialiser. It did: the file said `state =
//!    FILLED`, and the running artefact behaved as the plain interpreter.
//!    Fixed with `std::hint::black_box`, which is a hint rather than a
//!    guarantee — so this check is what keeps it honest.
//!
//! **A test suite that runs in one profile cannot see a property of the other.**
//! That is the lesson, and this is the instrument.

use std::path::{Path, PathBuf};

const PROGRAM: &str = "\"bundled\" println\n1 2 + println\nargs println\n";

fn build_cli(repo: &Path, release: bool) -> Result<PathBuf, String> {
    crate::buildcli::bund2(repo, release, "")
}

/// Build an artefact with `bund2` and run it, returning what it printed.
fn round_trip(bund2: &Path, work: &Path, tag: &str) -> Result<String, String> {
    let src = work.join(format!("{tag}.bund"));
    std::fs::write(&src, PROGRAM).map_err(|e| format!("writing {}: {e}", src.display()))?;
    let artefact = work.join(tag);
    let _ = std::fs::remove_file(&artefact);

    let built = std::process::Command::new(bund2)
        .arg("build")
        .arg("--file")
        .arg(&src)
        .arg("--output")
        .arg(&artefact)
        .output()
        .map_err(|e| format!("spawning `bund2 build`: {e}"))?;
    if !built.status.success() {
        return Err(format!(
            "`bund2 build` failed in the {tag} profile: {}",
            String::from_utf8_lossy(&built.stderr).trim()
        ));
    }

    let ran = std::process::Command::new(&artefact)
        .arg("one")
        .output()
        .map_err(|e| format!("spawning the {tag} artefact: {e}"))?;
    let mut out = String::from_utf8_lossy(&ran.stdout).into_owned();
    out.push_str(&String::from_utf8_lossy(&ran.stderr));
    match ran.status.code() {
        Some(0) => Ok(out),
        // **137 is the signature failure, and it is worth naming.** An edit to
        // a signed binary is killed rather than run, so a missing or broken
        // re-sign looks exactly like a program that printed nothing (Q40).
        Some(137) => Err(format!(
            "the {tag} artefact was killed (137). On macOS an unsigned edit to \
             a binary does not run, so `bund2 build`'s re-sign step is the \
             thing to look at."
        )),
        other => Err(format!(
            "the {tag} artefact exited {other:?} and printed: {out}"
        )),
    }
}

pub fn run(_args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();
    let work = repo.join("target/bundle-check");
    std::fs::create_dir_all(&work).map_err(|e| format!("creating {}: {e}", work.display()))?;

    println!("# cargo xtask bundle\n");
    println!("RFC-0006's artefact, built and run in **both** profiles.");
    println!("`cargo test` compiles in debug only, and both of this feature's");
    println!("blockers were release-only — one in how the region is located,");
    println!("one in whether the runtime can observe it at all.\n");

    let mut failures = 0usize;
    for (tag, release) in [("debug", false), ("release", true)] {
        let bund2 = build_cli(&repo, release)?;
        match round_trip(&bund2, &work, tag) {
            Ok(out) => {
                let ok = out.contains("bundled") && out.contains('3') && out.contains("one");
                println!(
                    "  {tag:<8} {}",
                    if ok {
                        "runs, prints its answer, and sees its argument"
                    } else {
                        "RAN BUT PRINTED THE WRONG THING"
                    }
                );
                if !ok {
                    failures += 1;
                    println!("      output was: {}", out.replace('\n', " | "));
                }
            }
            Err(e) => {
                failures += 1;
                println!("  {tag:<8} FAILED: {e}");
            }
        }
    }

    // The artefact must still satisfy the platform's signature, which is the
    // whole reason the payload lives inside the image rather than after it.
    #[cfg(target_os = "macos")]
    {
        let artefact = work.join("release");
        if artefact.is_file() {
            let v = std::process::Command::new("/usr/bin/codesign")
                .arg("-v")
                .arg(&artefact)
                .output()
                .map_err(|e| format!("spawning codesign: {e}"))?;
            if v.status.success() {
                println!("  signature  the release artefact validates");
            } else {
                failures += 1;
                println!(
                    "  signature  FAILED: {}",
                    String::from_utf8_lossy(&v.stderr).trim()
                );
            }
        }
    }

    println!();
    if failures == 0 {
        println!("  both profiles produce a working artefact.");
        return Ok(());
    }
    Err(format!("{failures} check(s) failed"))
}
