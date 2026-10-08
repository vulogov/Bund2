//! A reader that leaves early does not abort Bund2 (D37).
//!
//! `println!` panics when its write fails, and a write to a pipe whose reader
//! has gone fails: `bund2 words | head -1` ended in a Rust backtrace and exit
//! status 101. Nothing in the workspace uses the `print` family any more
//! (`bund2_api::out`), and this is the test that the replacement holds.
//!
//! **The program prints more than a pipe holds.** A short output is written
//! whole before the reader is dropped and never sees the failure; this one
//! blocks on a full pipe until the read end closes, so the failing write is
//! certain rather than a race.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn a_program_whose_reader_left_stops_without_a_panic() {
    let dir = std::env::temp_dir().join(format!("bund2-closed-output-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let script = dir.join("many-lines.bund");
    std::fs::write(&script, "0 200000 { dup println 1 + } times\n").expect("script");

    let mut child = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["script", "--file"])
        .arg(&script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bund2 starts");
    let mut stdout = child.stdout.take().expect("stdout");
    let mut first = [0u8; 1];
    stdout.read_exact(&mut first).expect("the program printed something");
    drop(stdout);

    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(s) = child.try_wait().expect("wait") {
            break Some(s);
        }
        if Instant::now() > deadline {
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let Some(status) = status else {
        let _ = child.kill();
        panic!("bund2 was still running after its reader left");
    };

    let mut said = String::new();
    child
        .stderr
        .take()
        .expect("stderr")
        .read_to_string(&mut said)
        .expect("stderr is text");
    let _ = std::fs::remove_dir_all(&dir);

    assert!(!said.contains("panicked"), "bund2 panicked:\n{said}");
    assert_ne!(status.code(), Some(101), "exit status of a Rust panic");
    assert!(status.code().is_some(), "ended by a signal: {status:?}");
}
