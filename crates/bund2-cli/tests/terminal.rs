//! **The console at a real terminal** — RFC-0008's amendment, "The console at
//! a real terminal".
//!
//! Every other test in this crate gives the binary a pipe, and a pipe reaches
//! none of this: the prompt with its editing, the history, what Ctrl-C and
//! Ctrl-D do, and the program's own `input` and `password` on the terminal
//! the debugger has just been using. So this opens a pseudo-terminal and
//! types.
//!
//! Each wait is bounded, and a child that outlives its test is killed.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

const PROMPT: &str = "(bund2) ";
const PATIENCE: Duration = Duration::from_secs(20);

/// The binary on a pseudo-terminal, and everything it has written so far.
struct Terminal {
    child: Child,
    keys: std::fs::File,
    screen: Receiver<Vec<u8>>,
    seen: String,
    /// Where the next `expect` starts looking, so one text is not found twice.
    at: usize,
    home: std::path::PathBuf,
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Terminal {
    /// Run `src` with `flags`, all three standard streams on the terminal.
    /// `HOME` is a scratch directory, so the history is not the developer's.
    fn run(src: &str, flags: &[&str]) -> Self {
        use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let home = std::env::temp_dir().join(format!("bund2-tty-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&home).expect("scratch dir");
        let script = home.join("t.bund");
        std::fs::write(&script, src).expect("script");

        let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).expect("a pseudo-terminal");
        grantpt(&master).expect("grantpt");
        unlockpt(&master).expect("unlockpt");
        let name = ptsname(&master, Vec::new()).expect("ptsname");
        let name = name.to_str().expect("a terminal's name is text").to_string();
        let side = || {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&name)
                .expect("the terminal opens")
        };
        let child = Command::new(env!("CARGO_BIN_EXE_bund2"))
            .args(flags)
            .args(["script", "--file"])
            .arg(&script)
            .env("HOME", &home)
            .env_remove("XDG_CONFIG_HOME")
            .env("TERM", "xterm")
            .stdin(Stdio::from(side()))
            .stdout(Stdio::from(side()))
            .stderr(Stdio::from(side()))
            .spawn()
            .expect("bund2 runs");

        let keys = std::fs::File::from(master);
        let mut reads = keys.try_clone().expect("a second handle");
        let (tx, screen) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            // Ends on end-of-file or on the error a closed terminal answers.
            while let Ok(n) = reads.read(&mut buf) {
                if n == 0 || tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        Self { child, keys, screen, seen: String::new(), at: 0, home }
    }

    /// Wait until `what` has been written, past whatever was last expected.
    fn expect(&mut self, what: &str) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if let Some(i) = self.seen[self.at..].find(what) {
                self.at += i + what.len();
                return;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            match self.screen.recv_timeout(left) {
                Ok(bytes) => self.seen.push_str(&String::from_utf8_lossy(&bytes)),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                    panic!("never saw {what:?} after byte {}. The terminal held:\n{:?}", self.at, self.seen)
                }
            }
        }
    }

    fn type_keys(&mut self, keys: &str) {
        self.keys.write_all(keys.as_bytes()).expect("the terminal takes keys");
        self.keys.flush().expect("flush");
    }

    /// Wait until the terminal has stopped echoing keys.
    ///
    /// `password` writes its prompt and *then* turns the echo off, so keys
    /// typed on sight of the prompt are echoed by the terminal itself before
    /// the word has read anything. A person is slower than that; a test is
    /// not, and has to wait for the mode rather than for the text.
    fn expect_no_echo(&mut self) {
        use rustix::termios::{LocalModes, tcgetattr};
        let deadline = Instant::now() + PATIENCE;
        loop {
            let modes = tcgetattr(&self.keys).expect("the terminal's modes").local_modes;
            if !modes.contains(LocalModes::ECHO) {
                return;
            }
            assert!(Instant::now() < deadline, "the echo was never turned off");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Type a line at the console's prompt, having waited for the prompt.
    fn command(&mut self, line: &str) {
        self.expect(PROMPT);
        self.type_keys(line);
        self.type_keys("\r");
    }

    /// The exit code, once the program has ended.
    fn ends(&mut self) -> Option<i32> {
        let deadline = Instant::now() + PATIENCE;
        loop {
            if let Some(status) = self.child.try_wait().expect("wait") {
                return status.code();
            }
            assert!(Instant::now() < deadline, "the program never ended:\n{:?}", self.seen);
            // Keep the terminal drained, or a full buffer blocks the child.
            if let Ok(bytes) = self.screen.recv_timeout(Duration::from_millis(20)) {
                self.seen.push_str(&String::from_utf8_lossy(&bytes));
            }
        }
    }

    /// The one history file under the scratch home, if any was written.
    fn history(&self) -> Option<String> {
        fn find(dir: &std::path::Path) -> Option<std::path::PathBuf> {
            for e in std::fs::read_dir(dir).ok()?.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if let Some(found) = find(&p) {
                        return Some(found);
                    }
                } else if p.file_name().is_some_and(|n| n == "bund2_debugger_history.txt") {
                    return Some(p);
                }
            }
            None
        }
        std::fs::read_to_string(find(&self.home)?).ok()
    }
}

/// A script that hands itself over with `debug.step`, at a terminal: a prompt
/// that answers, a line recalled with the up arrow, a history kept, and the
/// program's own `input` on the same terminal once it runs on.
#[test]
fn a_console_attached_by_a_word_is_a_prompt_with_a_history() {
    let src = "\"start\" println\ndebug.step\n\"name? \" input string.upper println\n\"end\" println\n";
    let mut t = Terminal::run(src, &[]);
    t.expect("start");

    t.command("st");
    t.expect("@main");

    t.command("\"typed\" println");
    t.expect("typed\r\n");

    // Up, then Enter: the line before is run again.
    t.expect(PROMPT);
    t.type_keys("\x1b[A");
    t.type_keys("\r");
    t.expect("typed\r\n");

    t.command("c");
    t.expect("name? ");
    t.type_keys("bob\r");
    t.expect("BOB");
    t.expect("end");
    assert_eq!(t.ends(), Some(0));

    let history = t.history().expect("a history under the config directory");
    assert!(history.contains("\"typed\" println"), "{history}");
}

/// Ctrl-D and Ctrl-C at the prompt both detach, say so, and the program runs
/// to its end. Neither ends the program: the input seam answers the two alike
/// (D112).
#[test]
fn ctrl_d_and_ctrl_c_detach_and_the_program_runs_on() {
    for key in ["\x04", "\x03"] {
        let mut t = Terminal::run("debug.step\n\"ran on\" println\n", &[]);
        t.expect(PROMPT);
        t.type_keys(key);
        t.expect("detached; the program runs on.");
        t.expect("ran on");
        assert_eq!(t.ends(), Some(0), "after {key:?}");
    }
}

/// `password` after a session, on the terminal the console was using: a dot
/// for each key and none of the keys.
#[test]
fn a_password_is_read_on_the_terminal_after_a_session() {
    let mut t = Terminal::run("debug.step\n\"pw: \" password string.upper println\n", &[]);
    t.command("c");
    t.expect("pw: ");
    t.expect_no_echo();
    t.type_keys("hush\r");
    t.expect("....");
    t.expect("HUSH");
    assert_eq!(t.ends(), Some(0));
    assert!(!t.seen.contains("hush"), "the keys were echoed:\n{:?}", t.seen);
}

/// `--debugger` at a terminal: the plain console, a line fed to the program,
/// and a typo answered in the word's own words.
#[test]
fn the_debugger_flag_at_a_terminal_takes_a_fed_line() {
    let mut t = Terminal::run("\"name? \" input string.upper println\n", &["--debugger"]);
    t.command("stpe");
    t.expect("stpe not registered");
    t.command("\"bob\" debug.feed");
    t.command("c");
    t.expect("BOB");
    assert_eq!(t.ends(), Some(0));
}
