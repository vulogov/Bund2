//! **RFC-0013 criterion 4: no Rust depth across answers.**
//!
//! `[ nat _ ]` over `[ nat z ]` and `[ [ nat [ s N ] ] ← [ nat N ] ]`, with
//! `limit` 1,000,000, answers 1,000,000 empty MAPs, and the Rust stack depth
//! sampled from `lambda` at the 100,000th and the 1,000,000th answer is the
//! same.
//!
//! **The probe** is a native this test registers, `StackEffect::opaque(0)`,
//! which takes the address of one of its own locals: equal addresses mean
//! equal depth. It keeps the two samples, and its count, in statics rather
//! than pushing them, so `lambda` stays `{ drop probe }`.
//!
//! **Here, not in `bund2-stdlib`,** because a tier exists only where
//! `Runtime`'s constructor installs one, under this crate's `jit` feature;
//! the suite runs these tests with and without it, so the criterion holds at
//! both tiers. By the 100,000th answer `lambda` has been entered far beyond
//! D74's threshold of 1024, so under `jit` the tier has compiled or declined
//! it (D146), and both samples come through the same caller.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::atomic::{AtomicUsize, Ordering};

use bund2_api::Vm as _;
use bund2_runtime::Runtime;

static CALLS: AtomicUsize = AtomicUsize::new(0);
static AT_100K: AtomicUsize = AtomicUsize::new(0);
static AT_1M: AtomicUsize = AtomicUsize::new(0);

fn probe(_vm: &mut dyn bund2_api::Vm) -> Result<(), bund2_api::Error> {
    let local = 0u8;
    let here = std::hint::black_box(&local) as *const u8 as usize;
    match CALLS.fetch_add(1, Ordering::Relaxed) + 1 {
        100_000 => AT_100K.store(here, Ordering::Relaxed),
        1_000_000 => AT_1M.store(here, Ordering::Relaxed),
        _ => {}
    }
    Ok(())
}

#[test]
fn a_million_answers_cost_no_rust_depth() {
    std::thread::Builder::new()
        .stack_size(bund2_runtime::EVAL_STACK)
        .spawn(|| {
            bund2_runtime::declare_region();
            let mut rt = Runtime::new();
            rt.interp.registry.register_native(
                "probe",
                probe,
                bund2_api::StackEffect::opaque(0),
                bund2_api::WordKind::Sync,
            );
            rt.eval_str(
                ":nats [ [ nat z ] [ [ nat [ s N ] ] ← [ nat N ] ] ] kb! var \
                 query :kb :nats var? set :goal [ nat _ ] set \
                 :lambda { drop probe } set :limit 1000000 set !",
            )
            .unwrap();
            assert_eq!(CALLS.load(Ordering::Relaxed), 1_000_000, "one per answer");
            assert_eq!(rt.interp.depth(), 0, "each answer was consumed");
            let (a, b) = (
                AT_100K.load(Ordering::Relaxed),
                AT_1M.load(Ordering::Relaxed),
            );
            assert_ne!(a, 0);
            assert_eq!(a, b, "the probe's frame moved between the two samples");
        })
        .unwrap()
        .join()
        .unwrap();
}
