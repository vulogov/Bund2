//! `bund_examples/smysl/`: SMYSL and logic programming together, every
//! `smysl.` word and the `kb.` features that meet them, prints
//! `expected.txt` and, on stderr, the two warnings `expected.stderr.txt`
//! holds. Only in the `smysl` build.
//!
//! The demo names its library by a relative root, so each run starts from
//! none: `demo.bund` is copied into a fresh directory and run there as the
//! working directory. Standard input is closed, so a question that reached
//! the terminal would fail rather than wait. It is not a golden: the
//! reference has no `smysl.` words.

#![cfg(feature = "smysl")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn the_smysl_logic_demo_prints_its_expected_output() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bund_examples/smysl");
    let run =
        std::env::temp_dir().join(format!("bund2-cli-smysl-logic-demo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&run);
    std::fs::create_dir_all(&run).unwrap();
    std::fs::copy(dir.join("demo.bund"), run.join("demo.bund")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .arg("demo.bund")
        .current_dir(&run)
        .stdin(Stdio::null())
        .output()
        .expect("bund2 runs");
    let _ = std::fs::remove_dir_all(&run);
    let expected = std::fs::read_to_string(dir.join("expected.txt")).expect("expected.txt");
    let warned =
        std::fs::read_to_string(dir.join("expected.stderr.txt")).expect("expected.stderr.txt");
    assert!(
        out.status.success(),
        "the demo failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), expected);
    assert_eq!(String::from_utf8_lossy(&out.stderr), warned);
}
