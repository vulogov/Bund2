//! Executor integration and async native words. Feature-gated. See RFC-0007.
//!
//! **§C8's VM host is in [`host`]**: a thread per VM, bounded by D85, with each
//! thread declaring its own stack region and Tier 1 share. The `async` façade
//! over a single VM — §C8's other half, criterion 13 — is not here yet, and
//! what it waits on is a dependency question rather than a design one.

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

pub mod host;
