//! RFC-0015 criterion 21: `bund_examples/biomath/` runs every worked
//! example and §S18's rule base, and prints `expected.txt`.
//!
//! Every draw is seeded, so the output is fixed. Standard input is closed,
//! as the Makefile runs it. It is not a golden: the oracle has none of the
//! family's words (D21).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn the_biomath_demo_prints_its_expected_output() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bund_examples/biomath");
    let out = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .arg("demo.bund")
        .current_dir(&dir)
        .stdin(Stdio::null())
        .output()
        .expect("bund2 runs");
    let expected = std::fs::read_to_string(dir.join("expected.txt")).expect("expected.txt");
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
