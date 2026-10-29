//! Logic programming over knowledge bases: a family of Bund2's own
//! (RFC-0013, D226).
//!
//! **None of this is the reference's.** No reference word is named `query` or
//! `kb.…`, so `conform`, `IMPLEMENTED` and `COVERAGE` move by zero (D72), and
//! what checks the family is its unit tests rather than a golden.
//!
//! The engine is built in the RFC's milestones, in order. M1 to M3 are
//! here:
//!
//! - [`term`]: the arena, the atoms' interner, and conversion from a Bund
//!   value and back (§S1);
//! - [`unify`]: unification with the occurs check always on (L7), and the
//!   trail (§S4);
//! - [`store`]: reading, validating and writing a KB's MAP (§S2);
//! - [`solve`]: the loop, its choicepoints, control, indexing and caps
//!   (§S4, §S6, §S7);
//! - [`words`]: the building words and their `.` forms, and the asking
//!   words `kb.query`, `kb.once` and `kb.holds` (§S9). Bridges are M4's.
//!
//! This is the crate's first subdirectory. Every source scan in `lib.rs`
//! walks subdirectories, criterion 25's among them since M1 (§S6).

pub mod solve;
pub mod store;
pub mod term;
pub mod unify;
pub mod words;

/// Register the family's words.
pub fn register(r: &mut bund2_api::Registry) {
    words::register(r);
}
