//! RFC-0013 criterion 25: `bund_examples/logic_demo/` runs every worked
//! example unattended, and prints `expected.txt`.
//!
//! Standard input is closed, so a question that reached the terminal would
//! read end of file and fail example 5 rather than wait. It is not a golden:
//! the oracle has none of the `kb.` words.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn the_logic_demo_prints_its_expected_output() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bund_examples/logic_demo");
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
