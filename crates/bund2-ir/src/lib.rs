//! BundIR: linear, span-carrying, effect-annotated IR. See RFC-0003.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

/// A word's specialised arm, for a code generator to inline. RFC-0005 §S6.
pub mod fragment;

pub use fragment::{Fragment, Guard, Op};
