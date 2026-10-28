//! **D171: an OOM guard trip is an ordinary run-time error.**
//!
//! The guard's thread only sets `bund2_api::memguard`'s flag; these tests set
//! it the same way and watch what the interpreter does with it. No test here
//! applies real memory pressure: the reading and the trip decision are
//! `bund2_host::memwatch`'s, tested there with samples.
//!
//! **A binary of its own, and one test in it at a time,** because the flag is
//! process-wide: a trip set by one test would be taken by whichever test's
//! interpreter reached a call first.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use bund2_api::Vm;
use bund2_api::memguard::{self, Trip};
use bund2_runtime::Runtime;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

const TRIP: Trip = Trip {
    resident: 18_400_000_000,
    available: 900_000_000,
    swap_free: 200_000_000,
};

/// Run `f` on a thread sized and declared as the CLI's evaluation thread is.
fn on_eval_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(bund2_runtime::EVAL_STACK)
        .spawn(move || {
            bund2_runtime::declare_region();
            f()
        })
        .unwrap()
        .join()
        .unwrap()
}

#[test]
fn a_trip_fails_the_next_call_like_any_error_and_is_catchable() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    on_eval_thread(|| {
        let mut rt = Runtime::new();
        rt.eval_str(":f { 1 } register").unwrap();

        // The next call fails, with the guard's text, as the program's
        // error and not Bund2's.
        memguard::trip(TRIP);
        let e = rt.eval_str("f").expect_err("the call after a trip fails");
        assert!(e.0.contains("OOM GUARD"), "{}", e.0);
        assert!(!e.is_internal(), "{}", e.0);

        // Taken once: the program can go on.
        rt.eval_str("f").unwrap();
        assert_eq!(rt.interp.peek().and_then(|v| v.as_int()), Some(1));

        // `?try` catches it like any other run-time error.
        memguard::trip(TRIP);
        rt.eval_str(
            "?try :try { f } set :except { \"caught\" } set \
             :recovery { \"recovered\" } set !",
        )
        .unwrap();
        let top = rt.interp.peek().and_then(|v| v.as_str());
        assert!(
            matches!(top.as_deref(), Some("caught" | "recovered")),
            "?try ran a branch: {top:?}"
        );
        assert_eq!(memguard::take(), None, "the trip was consumed");
    });
}

/// A program that would never end on its own is stopped at its next call
/// after the trip, from another thread, as the guard's thread would. Under
/// `--features jit` the loop's word is compiled within the first few
/// thousand rounds, so this also holds the check on the tier's path.
#[test]
fn a_running_program_is_stopped_at_its_next_call() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (tx, rx) = mpsc::channel();
    let runner = std::thread::Builder::new()
        .stack_size(bund2_runtime::EVAL_STACK)
        .spawn(move || {
            bund2_runtime::declare_region();
            let mut rt = Runtime::new();
            let r = rt.eval_str(
                ":w { 1 2 + drop } register \
                 9223372036854775807 { drop w } times",
            );
            let _ = tx.send(r.map_err(|e| e.0));
        })
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    memguard::trip(TRIP);
    let r = rx
        .recv_timeout(Duration::from_secs(20))
        .expect("the loop stopped after the trip");
    let e = r.expect_err("the loop ends in an error");
    assert!(e.contains("OOM GUARD"), "{e}");
    runner.join().unwrap();
}

/// **F231.** `scoped_call` switches to its stack before the frame that
/// switches back exists, and the guard's poll refuses that frame. The call
/// fails, and the stack it came from is current again — through `context`,
/// which takes the same path, as well as directly.
#[test]
fn a_trip_refusing_a_scoped_call_leaves_the_stack_where_it_was() {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    on_eval_thread(|| {
        let mut rt = Runtime::new();
        let home = rt.interp.current_name();

        memguard::trip(TRIP);
        let e = rt
            .interp
            .scoped_call("elsewhere", vec![bund2_value::BundValue::int(1)])
            .expect_err("the scoped call after a trip fails");
        assert!(e.0.contains("OOM GUARD"), "{}", e.0);
        assert_eq!(rt.interp.current_name(), home, "the stack was restored");

        rt.eval_str(":ctx :elsewhere context :body { 1 } set var")
            .unwrap();
        memguard::trip(TRIP);
        let e = rt
            .eval_str(":body :ctx var? !")
            .expect_err("the context after a trip fails");
        assert!(e.0.contains("OOM GUARD"), "{}", e.0);
        assert_eq!(rt.interp.current_name(), home, "context restored it");
        assert_eq!(memguard::take(), None, "the trip was consumed");
    });
}
