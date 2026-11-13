//! **D249 at process level:** the `vm.` words are in the default build
//! (R1 A), and a program that ends with VMs it never joined is told once
//! (R9 C).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::{Command, Stdio};

struct Run {
    out: String,
    err: String,
    code: Option<i32>,
}

fn bund2(args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("bund2 ran");
    Run {
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code(),
    }
}

fn program(name: &str, src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bund2-vm-words-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, src).unwrap();
    path
}

/// R1 A: the default build has all three. `bund2 words` does not list them:
/// it lists the standard library's registry, the join key `xtask coverage`
/// needs, and these are registered by the runtime, as `test.panic` is.
#[test]
fn the_default_build_has_the_vm_words() {
    let p = program(
        "all_three.bund",
        ":h { 6 7 * } list vm.spawn var\n:h var? vm.done? drop\n:h var? vm.join println\n",
    );
    let r = bund2(&[p.to_str().unwrap()]);
    assert_eq!(r.code, Some(0), "{}", r.err);
    assert!(r.out.contains("42"), "{}", r.out);
}

/// R9 C: one Warning naming how many were not joined, and the program's
/// own output and exit code as before.
#[test]
fn an_unjoined_vm_is_warned_about_once_at_the_end() {
    let p = program(
        "unjoined.bund",
        "{ 1 } list vm.spawn drop\n{ 2 } list vm.spawn vm.join println\n",
    );
    let r = bund2(&[p.to_str().unwrap()]);
    assert_eq!(r.code, Some(0), "{}", r.err);
    assert!(r.out.contains('2'), "{}", r.out);
    assert_eq!(r.err.matches("were not joined").count(), 1, "{}", r.err);
    assert!(
        r.err
            .contains("1 VM(s) started with vm.spawn were not joined; they end with the program"),
        "{}",
        r.err
    );
}

/// A program that joins every VM it started ends with nothing to say.
#[test]
fn a_program_that_joins_everything_is_not_warned() {
    let p = program("joined.bund", "{ 1 } list vm.spawn vm.join drop\n");
    let r = bund2(&[p.to_str().unwrap()]);
    assert_eq!(r.code, Some(0), "{}", r.err);
    assert!(!r.err.contains("not joined"), "{}", r.err);
}
