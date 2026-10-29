//! Logic programming over knowledge bases: a family of Bund2's own
//! (RFC-0013, D226).
//!
//! **None of this is the reference's.** No reference word is named `query` or
//! `kb.…`, so `conform`, `IMPLEMENTED` and `COVERAGE` move by zero (D72), and
//! what checks the family is its unit tests rather than a golden.
//!
//! The engine is built in the RFC's milestones, in order. M1 is here:
//!
//! - [`term`]: the arena, the atoms' interner, and conversion from a Bund
//!   value and back (§S1);
//! - [`unify`]: unification with the occurs check always on (L7), and the
//!   trail (§S4).
//!
//! No word is registered yet; M2 adds the building words.
//!
//! This is the crate's first subdirectory. Every source scan in `lib.rs`
//! walks subdirectories, criterion 25's among them since M1 (§S6).

pub mod term;
pub mod unify;
