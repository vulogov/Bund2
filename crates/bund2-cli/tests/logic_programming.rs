//! `bund_examples/logic_programming/` solves the Zebra Puzzle by search and
//! by constraints, and prints `expected.txt`.
//!
//! Standard input is closed, so nothing can wait on the terminal. It is not
//! a golden: the oracle has none of the `kb.` words.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn the_zebra_puzzle_prints_its_expected_output() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bund_examples/logic_programming");
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .arg("zebra.bund")
        .current_dir(&dir)
        .stdin(Stdio::null())
        .output()
        .expect("bund2 runs");
    let expected = std::fs::read_to_string(dir.join("expected.txt")).expect("expected.txt");
    assert!(
        out.status.success(),
        "the example failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), expected);
    assert!(
        out.stderr.is_empty(),
        "the example wrote to stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
