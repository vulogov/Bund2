//! Tier 1 and AOT: BundIR to CLIF. Feature-gated. See RFC-0005, RFC-0006.
//!
//! [`lower`] holds the first lowering: one [`bund2_ir::Fragment`] to machine
//! code, so criterion 16's third leg — the lowered arm against
//! `bund2_interp::frag::run` — has something to run against. It is **not**
//! §S8's call boundary, and its own module docs say what it does and does not
//! claim.

#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

// Behind the feature, because everything in it names a Cranelift type. Without
// `jit` the crate stays the placeholder it was, which is what keeps Tier 1
// optional and the portability story the interpreter's (§S10).
#[cfg(feature = "jit")]
pub mod lower;

/// The compiled cache and the promotion counter — §S3, §S7, D35 as amended.
///
/// Behind the feature because its entries are [`lower::WordHandle`]s, which
/// only a [`lower::Compiler`] — and so only Cranelift — can issue.
#[cfg(feature = "jit")]
pub mod cache;
