//! The generator RFC-0015's drawing words draw from (§S8, D250 B4).
//!
//! **The VM's own**, through [`Vm::rng`]: `Interp` keeps a `StdRng`, seeded
//! from the operating system on first use, so two VMs never share draws,
//! whichever threads they run on. A `Vm` that keeps none (a test double, an
//! embedder's own) is given one per thread here instead.
//!
//! **Separate from every existing source.** `generator.sample`,
//! `math.random.int` and `math.securerandom.int` keep theirs (`random.rs`),
//! unseeded as the reference's are, and draw nothing from this one.
//!
//! **What "repeats" means.** After [`reseed`], every draw repeats exactly
//! within one Bund2 version: `rand`, `rand_distr` and `statrs` are pinned
//! (D38), so their algorithms are that version's.

use std::cell::Cell;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;
use rand::rngs::StdRng;
use rand::{Rng as _, RngCore as _, SeedableRng};

use super::operand::{WIDTH, domain, operands};

thread_local! {
    /// The generator for a `Vm` that keeps none of its own.
    static FALLBACK: Cell<Option<StdRng>> = const { Cell::new(None) };
}

/// Run `f` with `vm`'s generator, or this thread's when `vm` keeps none.
///
/// The thread's generator is taken out for the call and put back after, so
/// nothing is borrowed while `f` runs; `f` is given the generator and not the
/// VM, so it cannot reach here again.
pub fn with_rng<T>(vm: &mut dyn Vm, f: impl FnOnce(&mut StdRng) -> T) -> T {
    if let Some(g) = vm.rng() {
        return f(g);
    }
    let mut g = FALLBACK
        .with(Cell::take)
        .unwrap_or_else(StdRng::from_entropy);
    let out = f(&mut g);
    FALLBACK.with(|c| c.set(Some(g)));
    out
}

/// `rng.seed`'s work: replace `vm`'s generator with one seeded from `n`.
///
/// Every INTEGER is a seed. A negative one is taken as its two's complement,
/// so `-1` and `18446744073709551615` would be the same seed, and only the
/// first is an INTEGER Bund can write.
pub fn reseed(vm: &mut dyn Vm, n: i64) {
    let g = StdRng::seed_from_u64(u64::from_ne_bytes(n.to_ne_bytes()));
    match vm.rng() {
        Some(mine) => *mine = g,
        None => FALLBACK.with(|c| c.set(Some(g))),
    }
}

/// `rng.seed` (`n --`): reseed the calling VM's generator (§S8).
fn rng_seed(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "rng.seed";
    let v = operands(vm, W, 1)?;
    match v[0].unboxed() {
        BundValue::Int(n, _) => {
            reseed(vm, *n);
            Ok(())
        }
        _ => Err(domain(W, "n", v[0].summary(WIDTH))),
    }
}

/// `rng.uniform` (`-- x`): one draw in [0, 1) from the VM's generator.
fn rng_uniform(vm: &mut dyn Vm) -> Result<(), Error> {
    let x: f64 = with_rng(vm, |g| g.r#gen());
    vm.push(BundValue::float(x));
    Ok(())
}

/// `rng.derive` (`-- seed`, RFC-0019 §S7): one `u64` from the VM's
/// generator, answered as an INTEGER by the bits, two's complement, as
/// [`reseed`] takes them back. After `n rng.seed` the k-th `rng.derive`
/// answers the same INTEGER within a Bund2 version, so a child VM seeded
/// with it by `rng.seed` repeats its own stream (D260 C6). Each call
/// advances this VM's stream by one `u64`.
fn rng_derive(vm: &mut dyn Vm) -> Result<(), Error> {
    let x = with_rng(vm, |g| g.next_u64());
    vm.push(BundValue::int(i64::from_ne_bytes(x.to_ne_bytes())));
    Ok(())
}

/// Register §S8's words. `rng.seed` has its `.` form, taking n from the
/// workbench; `rng.uniform` is a producer, so has none (§S3, D24).
pub fn register(r: &mut Registry) {
    r.register_native(
        "rng.seed",
        rng_seed,
        StackEffect::fixed(1, 0),
        WordKind::Sync,
    );
    crate::wb::bench!(r, "rng.seed", rng_seed, 1, 0);
    r.register_native(
        "rng.uniform",
        rng_uniform,
        StackEffect::fixed(0, 1),
        WordKind::Sync,
    );
    // RFC-0019 §S7: a producer, plain only.
    r.register_native(
        "rng.derive",
        rng_derive,
        StackEffect::fixed(0, 1),
        WordKind::Sync,
    );
}

/// §S8's words.
pub const WORDS: [&str; 2] = ["rng.seed", "rng.uniform"];

#[cfg(test)]
mod tests {
    use super::{reseed, with_rng};
    use bund2_interp::Interp;
    use rand::RngCore;

    fn draws(vm: &mut Interp, n: usize) -> Vec<u64> {
        (0..n).map(|_| with_rng(vm, |g| g.next_u64())).collect()
    }

    /// Criterion 11's first bullet, at the generator: one seed, one stream.
    #[test]
    fn the_same_seed_repeats() {
        let mut vm = Interp::new();
        reseed(&mut vm, 42);
        let first = draws(&mut vm, 8);
        reseed(&mut vm, 42);
        assert_eq!(draws(&mut vm, 8), first);
        reseed(&mut vm, 43);
        assert_ne!(draws(&mut vm, 8), first);
    }

    /// Criterion 11's fourth bullet: two `Interp`s on one thread, one seeded,
    /// do not share draws. Drawing from the other between the seeded VM's
    /// draws leaves the seeded stream as it was.
    #[test]
    fn two_vms_on_one_thread_do_not_share_draws() {
        let mut a = Interp::new();
        let mut b = Interp::new();
        reseed(&mut a, 7);
        let alone = draws(&mut a, 6);
        reseed(&mut a, 7);
        let mut mixed = draws(&mut a, 2);
        let _ = draws(&mut b, 5);
        reseed(&mut b, 99);
        mixed.extend(draws(&mut a, 4));
        assert_eq!(mixed, alone);
    }

    /// Every INTEGER is a seed, the negative ones included.
    #[test]
    fn a_negative_seed_is_a_seed() {
        let mut vm = Interp::new();
        reseed(&mut vm, -1);
        let first = draws(&mut vm, 3);
        reseed(&mut vm, -1);
        assert_eq!(draws(&mut vm, 3), first);
    }
}
