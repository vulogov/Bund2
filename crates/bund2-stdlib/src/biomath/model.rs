//! RFC-0018: models over frames, research 07's Phase C. M1 adds §S2's
//! design: `model.matrix`, which codes a frame's columns as a design
//! ([`code`]); the design's check, which the fitting words run on entry
//! ([`design`]); and this RFC's own options reader ([`opts`]), which knows
//! every model word's keys from the start. M2 adds §S3's `fit.lm`
//! ([`lm`]), registered beside `fit.logistic` in `fit.rs`, over
//! `stat.lm`'s computation factored out as `infer.rs`'s `least_squares`.
//!
//! **A design is a plain MAP** (§S2): `matrix`, a MATRIX of FLOATs, the
//! intercept's column first; `names`; `assign`, each column's term; `y`,
//! `weights` and `offset`, LISTs of FLOATs; `dropped`; and `spec`, what a
//! new frame is coded by. Nothing here is the reference's: the words are
//! Bund2's own (D72), so `conform` and `coverage` do not move, and the
//! unit tests are what check them (D21).
//!
//! **Refusals** are RFC-0015 §S11's codes with §S8's texts; a value a text
//! takes from the program is rendered by RFC-0017's `render`.

pub(crate) mod code;
pub mod design;
pub(crate) mod lm;
pub(crate) mod opts;

use bund2_api::{Registry, StackEffect, WordKind};

/// Register M1's word, plain and `.`: the `.` form takes `opts` from the
/// workbench and answers there (RFC-0015 §S3).
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("model.matrix", code::model_matrix, 4);
}

/// The `model.` words with their forms.
pub const WORDS: [&str; 2] = ["model.matrix", "model.matrix."];

#[cfg(test)]
mod tests;
