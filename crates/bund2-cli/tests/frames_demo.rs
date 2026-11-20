//! RFC-0017: `bund_examples/frames/` runs the frame words over a
//! small trial and prints `expected.txt`.
//!
//! Nothing is random, so the output is fixed. Standard input is closed,
//! as the Makefile runs it. It is not a golden: the oracle has none of the
//! frame words (D21).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn the_frames_demo_prints_its_expected_output() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bund_examples/frames");
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
