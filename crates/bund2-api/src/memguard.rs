//! The OOM guard's trip, as the interpreter sees it — D171.
//!
//! The guard is a host thread (`bund2_host::memwatch`); the interpreter is
//! single-threaded and holds `Rc`s, so the two meet at one process-wide flag.
//! The thread calls [`trip`]; the interpreter calls [`take`] where a body
//! starts running and, if the guard has tripped, fails that call with
//! [`Trip::error`] — an ordinary run-time error, reported with the
//! diagnostic, backtrace and stack any other error gets.
//!
//! **Process-wide on purpose.** Memory is the process's, not one VM's, so
//! whichever VM reaches a call first is the one stopped. A trip is consumed
//! by that one `take`: a program that catches the error and frees what it
//! held goes on, and one that goes on growing trips again.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::Error;

static TRIPPED: AtomicBool = AtomicBool::new(false);
static RESIDENT: AtomicU64 = AtomicU64::new(0);
static AVAILABLE: AtomicU64 = AtomicU64::new(0);
static SWAP_FREE: AtomicU64 = AtomicU64::new(0);

/// The figures the guard tripped on, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trip {
    pub resident: u64,
    pub available: u64,
    pub swap_free: u64,
}

/// Record a trip. Called from the guard's thread.
pub fn trip(t: Trip) {
    RESIDENT.store(t.resident, Ordering::Relaxed);
    AVAILABLE.store(t.available, Ordering::Relaxed);
    SWAP_FREE.store(t.swap_free, Ordering::Relaxed);
    TRIPPED.store(true, Ordering::Release);
}

/// Take a pending trip, if there is one.
///
/// **One load when nothing is pending**, which is every call of every program
/// that is not about to take the host down: this runs once per body started.
#[inline]
pub fn take() -> Option<Trip> {
    if !TRIPPED.load(Ordering::Relaxed) {
        return None;
    }
    take_slow()
}

#[inline(never)]
fn take_slow() -> Option<Trip> {
    if !TRIPPED.swap(false, Ordering::Acquire) {
        return None;
    }
    Some(Trip {
        resident: RESIDENT.load(Ordering::Relaxed),
        available: AVAILABLE.load(Ordering::Relaxed),
        swap_free: SWAP_FREE.load(Ordering::Relaxed),
    })
}

impl Trip {
    /// The run-time error a trip becomes.
    pub fn error(&self) -> Error {
        let gb = |b: u64| b as f64 / 1e9;
        Error(format!(
            "OOM GUARD: the host is under critical memory pressure and this program \
             is the cause ({:.1} GB resident, {:.1} GB available, {:.1} GB free swap). \
             It was stopped here so the host keeps running. \
             BUND2_OOM_GUARD_GB=0 disables the guard.",
            gb(self.resident),
            gb(self.available),
            gb(self.swap_free),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trip_is_taken_once_and_reads_as_a_plain_runtime_error() {
        let t = Trip {
            resident: 18_400_000_000,
            available: 900_000_000,
            swap_free: 200_000_000,
        };
        trip(t);
        assert_eq!(take(), Some(t));
        assert_eq!(take(), None, "a trip is consumed by the call it stops");
        let e = t.error();
        assert!(!e.is_internal(), "the program caused it, not Bund2");
        assert!(e.0.contains("18.4 GB resident"), "{}", e.0);
    }
}
