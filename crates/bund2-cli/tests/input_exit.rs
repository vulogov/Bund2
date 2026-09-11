//! RFC-0005 criterion 30's `input*` harness, Tier 0's half (F112).
//!
//! A body `input*` runs that ends in `exit` must stop `input*` too. Before
//! F112 it read another line. Piped input may be buffered past the first
//! line, so counting lines consumed proves nothing. Instead the test writes
//! one line, **holds the pipe open**, and asserts that the process exits
//! before a second line is written: a second read would block, not see EOF.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn input_loop_reads_no_line_after_an_exit() {
    let dir = std::env::temp_dir().join(format!("bund2-input-exit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let script = dir.join("input-exit.bund");
    std::fs::write(&script, "\"p> \" { println 7 exit } input*\n").expect("script");

    let mut child = Command::new(env!("CARGO_BIN_EXE_bund2"))
        .args(["script", "--file"])
        .arg(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("bund2 starts");
    let mut stdin = child.stdin.take().expect("stdin");
    stdin.write_all(b"a\n").expect("first line");
    stdin.flush().expect("flush");

    let deadline = Instant::now() + Duration::from_secs(10);
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
        panic!("bund2 was still reading input after the exit");
    };
    drop(stdin);

    let mut out = String::new();
    child
        .stdout
        .take()
        .expect("stdout")
        .read_to_string(&mut out)
        .expect("reads");
    assert_eq!(status.code(), Some(7));
    assert!(out.contains('a'), "the first line reached the body: {out:?}");
    let _ = std::fs::remove_dir_all(&dir);
}
