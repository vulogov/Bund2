//! The test-only probe (§S9): a write that panics, or pauses, at a named
//! point, so the tests can see what R20 and the re-open promise.
//!
//! Compiled in only under the `smysl-panic-probe` feature, which no shipped
//! build enables, as `panic-probe` gives `test.panic` (D220). Without it the
//! two points are empty functions.
//!
//! The probe is armed per canonical root, so a test that arms it does not
//! reach another test's library, nor disarm another test's probe.

use std::path::Path;

#[cfg(feature = "smysl-panic-probe")]
mod armed {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    /// What the probe does when a write to its root reaches its point.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Action {
        /// After the mutex and SMYSL's locks are taken, before the append.
        Panic,
        /// After the first open, before the locks: create this file, then
        /// wait until it is removed.
        Pause(PathBuf),
    }

    /// One action per root, so that tests running at once on different
    /// libraries do not disarm each other's.
    static ARMED: Mutex<BTreeMap<PathBuf, Action>> = Mutex::new(BTreeMap::new());

    fn armed() -> std::sync::MutexGuard<'static, BTreeMap<PathBuf, Action>> {
        ARMED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Arm the probe for writes to `root`, canonical.
    pub fn arm(root: &Path, action: Action) {
        armed().insert(root.to_path_buf(), action);
    }

    /// Disarm it for every root.
    pub fn disarm() {
        armed().clear();
    }

    fn take(root: &Path, want: impl Fn(&Action) -> bool) -> Option<Action> {
        let mut armed = armed();
        match armed.get(root) {
            Some(a) if want(a) => armed.remove(root),
            _ => None,
        }
    }

    #[expect(
        clippy::panic,
        reason = "RFC-0014 §S9: the probe's deliberate panic, under smysl-panic-probe only"
    )]
    pub fn at_panic(root: &Path) {
        if take(root, |a| *a == Action::Panic).is_some() {
            panic!("smysl-panic-probe: the armed panic inside a write (RFC-0014 §S9)");
        }
    }

    #[expect(
        clippy::panic,
        reason = "RFC-0014 §S9: a pause that is never released fails the test rather than hanging it"
    )]
    pub fn at_pause(root: &Path) {
        let Some(Action::Pause(flag)) = take(root, |a| matches!(a, Action::Pause(_))) else {
            return;
        };
        let _ = std::fs::write(&flag, b"arrived");
        let start = Instant::now();
        while flag.exists() {
            if start.elapsed() > Duration::from_secs(30) {
                panic!("smysl-panic-probe: the pause was not released within 30 s");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(feature = "smysl-panic-probe")]
pub use armed::{Action, arm, disarm};

pub(crate) fn at_panic(_root: &Path) {
    #[cfg(feature = "smysl-panic-probe")]
    armed::at_panic(_root);
}

pub(crate) fn at_pause(_root: &Path) {
    #[cfg(feature = "smysl-panic-probe")]
    armed::at_pause(_root);
}
