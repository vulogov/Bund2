//! Tier 1 and AOT: BundIR to CLIF. Feature-gated. See RFC-0005, RFC-0006.

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
