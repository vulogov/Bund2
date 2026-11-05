//! **RFC-0014 criterion 17:** `bund_examples/smysl_demo/` forms a library,
//! researches and queries it, and reasons over it with `smysl.kb` and
//! `smysl.prove`, printing `expected.txt`. Only in the `smysl` build.
//!
//! The demo names its library by a relative root, so each run starts from
//! none: `demo.bund` is copied into a fresh directory and run there as the
//! working directory. A second run against the same library would find the
//! first's retraction and proofs there, and print otherwise. It is not a
//! golden: the reference has no `smysl.` words, so there is no oracle to
//! capture one from.

#![cfg(feature = "smysl")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn the_smysl_demo_prints_its_expected_output() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bund_examples/smysl_demo");
    let run = std::env::temp_dir().join(format!("bund2-cli-smysl-demo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&run);
    std::fs::create_dir_all(&run).unwrap();
    std::fs::copy(dir.join("demo.bund"), run.join("demo.bund")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .arg("demo.bund")
        .current_dir(&run)
        .stdin(Stdio::null())
        .output()
        .expect("bund2 runs");
    let expected = std::fs::read_to_string(dir.join("expected.txt")).expect("expected.txt");
    let _ = std::fs::remove_dir_all(&run);
    assert!(
        out.status.success(),
        "the demo failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), expected);
    assert!(
        out.stderr.is_empty(),
        "the demo wrote to stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
