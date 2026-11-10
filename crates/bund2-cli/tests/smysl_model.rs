//! **RFC-0014 criterion 30:** a model answers, by hand. `bund_examples/smysl_model/`
//! ingests a short document and commits it, answers a question over it and
//! commits the answer, ingests and commits a second document, and ingests a
//! text window by window, against a local Ollama with `llama3.2`.
//!
//! A model's answers are not fixed, so this asserts only that every step
//! returns without a refusal and every staged batch was clean, and so
//! committed. A refusal stops the program with a report, so a run that
//! reaches its last section returned from every word.
//! CI has no Ollama, so the test is `#[ignore]`d and run by hand:
//!
//!     cargo test -p bund2-cli --features smysl-ollama --test smysl_model -- --ignored
//!
//! The implementation record names the model and the date it last passed.

#![cfg(feature = "smysl-ollama")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Stdio};

#[test]
#[ignore = "needs a local Ollama with llama3.2"]
fn a_model_ingests_answers_and_commits() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bund_examples/smysl_model");
    let run = std::env::temp_dir().join(format!("bund2-cli-smysl-model-{}", std::process::id()));
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "the demo was refused:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("-- the library"),
        "the demo stopped early:\n{stdout}\n{stderr}"
    );
    assert!(
        !stdout.contains("not committed"),
        "a staged batch was not clean:\n{stdout}\n{stderr}"
    );
    println!("{stdout}{stderr}");
}
