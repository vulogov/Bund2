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
//! The words arrive with M2 onwards: `math.` special functions (§S4), `vec.`
//! (§S5), `stat.` (§S6), `dist.` and `rng.` (§S7, §S8), `linalg.` (§S9),
//! `bio.` (§S10, §S12), `popgen.` (§S13), `phylo.` (§S14), `ode.` and `sim.`
//! (§S15) and `fit.` (§S16).
//!
//! **What M1 links.** `nalgebra` and `rand_distr` become direct dependencies
//! at the versions the lock already held, and `statrs` gains its `nalgebra`
//! feature, for the multivariate families (§S7). The lock gains no package.

pub mod refuse;
pub mod rng;
