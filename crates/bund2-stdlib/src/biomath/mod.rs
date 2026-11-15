//! Biomathematics and genetics: a family of Bund2's own (RFC-0015, D250).
//!
//! **None of this is the reference's.** No reference word bears these names,
//! so `conform`, `IMPLEMENTED` and `COVERAGE` move by zero (D72), and what
//! checks the family is its unit tests rather than a golden (D21).
//!
//! The family is built in the RFC's milestones, in order. M1 lays what every
//! later one uses:
//!
//! - [`refuse`]: S11's refusals, E101 to E115, each with its text;
//! - [`rng`]: the generator a drawing word draws from, the VM's own through
//!   [`bund2_api::Vm::rng`] (§S8, D250 B4), and `rng.seed`'s reseeding.
//!
//! M2 adds the first words:
//!
//! - [`special`]: §S4's sixteen `math.` special functions.
//!
//! M3 adds §S5 and §S6's descriptive words, over [`operand`]'s readers:
//!
//! - [`vector`]: the `vec.` words;
//! - [`describe`]: `stat.var` to `stat.cor.kendall`.
//!
//! M4 adds §S7 and §S8's words:
//!
//! - [`dist`]: the distributions, as MAPs, and the words that ask them;
//! - [`rng`]: `rng.seed` and `rng.uniform`.
//!
//! M5 adds the rest of §S6:
//!
//! - [`infer`]: the `stat.test.` words, `stat.padjust.*` and `stat.lm`.
//!
//! The rest arrive with M6 onwards: `linalg.` (§S9),
//! `bio.` (§S10, §S12), `popgen.` (§S13), `phylo.` (§S14), `ode.` and `sim.`
//! (§S15) and `fit.` (§S16).
//!
//! **What M1 links.** `nalgebra` and `rand_distr` become direct dependencies
//! at the versions the lock already held, and `statrs` gains its `nalgebra`
//! feature, for the multivariate families (§S7). The lock gains no package.

pub mod describe;
pub mod dist;
pub mod infer;
pub(crate) mod operand;
pub mod refuse;
pub mod rng;
pub mod special;
pub mod vector;

/// Register the family's words.
pub fn register(r: &mut bund2_api::Registry) {
    special::register(r);
    vector::register(r);
    describe::register(r);
    dist::register(r);
    infer::register(r);
    rng::register(r);
}

#[cfg(test)]
mod testkit;
