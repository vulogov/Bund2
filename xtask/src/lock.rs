//! **One `xtask` command at a time.**
//!
//! Every `xtask` command that measures anything builds `bund2-cli` first and
//! then runs it, and every feature combination builds to the **same path** —
//! `target/debug/bund2` (`crate::buildcli`, `bund2`). Cargo's own lock
//! serialises the two *builds*, and nothing serialises a build against
//! another process's *execution* of the file that build replaces.
//!
//! So a second `xtask` started while the first is comparing goldens relinks
//! the binary underneath it. The first sees truncated output and attributes it
//! to whichever goldens happened to run during the relink — a failure list
//! that is a property of the timing, not of the code.
//!
//! **This was not hypothetical.** On 2026-09-29 two overlapping `conform` runs
//! reported 103/116 and 99/116 with *disjoint* failure sets: one blamed four
//! corpus programs, the other ten probes, with no program in common. A single
//! clean run then read 107/116 with nothing failing. The second of those runs
//! also recorded a deviation hash — `--accept-deviation` pins "what Bund2 must
//! keep producing" — which would have locked truncated output into
//! `DEVIATIONS.txt` had the timing fallen a little differently.
//!
//! A number that changes with what else is running is not a measurement, and
//! the reports here exist to be quoted. Refusing is the only honest option:
//! waiting would be worse, because a command that blocks for twenty minutes
//! and then prints a number gives no sign that it measured a different machine
//! than the one the reader had in mind.

use std::path::{Path, PathBuf};

/// Holds the lock for as long as it is alive, and releases it on the way out.
///
/// The release is a `Drop`, so it happens on the error paths too — `main`
/// returns an `ExitCode` rather than calling `process::exit` (D37), so locals
/// are dropped.
pub struct Guard {
    path: PathBuf,
}

impl Drop for Guard {
    fn drop(&mut self) {
        // A failure here leaves a stale file, which the next run detects by
        // the pid it carries. Nothing better is available and nothing worse
        // happens, so it is not worth reporting over the command's output.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Is that process still running?
///
/// `ps -p` rather than a signal probe, so this needs no `libc` dependency for
/// a check that runs once per command. An unreadable answer counts as *alive*:
/// refusing a command that could have run costs a retry, and running two that
/// should not have overlapped costs a measurement nobody can tell is wrong.
fn alive(pid: &str) -> bool {
    match std::process::Command::new("ps")
        .args(["-p", pid, "-o", "pid="])
        .output()
    {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => true,
    }
}

/// Take the lock, or explain who holds it.
pub fn acquire(repo: &Path, cmd: &str) -> Result<Guard, String> {
    let dir = repo.join("target");
    std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    let path = dir.join(".xtask-lock");

    for attempt in 0..2 {
        // `create_new` is the whole of the mutual exclusion: it is one atomic
        // `O_EXCL` open, so two processes racing here cannot both succeed.
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                use std::io::Write;
                let pid = std::process::id();
                // Best effort. An empty or short lock file still excludes,
                // because exclusion is the `create_new` above; the contents
                // only make the refusal below informative.
                let _ = writeln!(f, "{pid}\t{cmd}");
                return Ok(Guard { path });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                let held = std::fs::read_to_string(&path).unwrap_or_default();
                let mut parts = held.trim().splitn(2, '\t');
                let pid = parts.next().unwrap_or("").trim();
                let other = parts.next().unwrap_or("an xtask command").trim();

                if !pid.is_empty() && !alive(pid) && attempt == 0 {
                    // Interrupted before its `Drop` ran. Clear it and retry
                    // once — and only once, so a lock a live process keeps
                    // taking cannot spin here.
                    let _ = std::fs::remove_file(&path);
                    continue;
                }

                return Err(format!(
                    "`xtask {other}` is already running (pid {pid}).\n  \
                     Two xtask commands share target/debug/bund2: the second \
                     relinks it while\n  the first is still executing it, so \
                     the first reports failures that are a\n  property of the \
                     timing. Wait for it, or if it is gone, remove\n  {}",
                    path.strip_prefix(repo).unwrap_or(&path).display()
                ));
            }
            Err(e) => return Err(format!("opening {}: {e}", path.display())),
        }
    }
    Err(format!(
        "could not take {} after clearing a stale lock",
        path.strip_prefix(repo).unwrap_or(&path).display()
    ))
}
