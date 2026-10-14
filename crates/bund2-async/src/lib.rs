//! The VM host and its `async` façade. See RFC-0007 §C8.
//!
//! **§C8's VM host is in [`host`]**: a thread per VM, bounded by D85, with each
//! thread declaring its own stack region and Tier 1 share. **The `async`
//! façade over a single VM is there too** — §C8's other half, criterion 13:
//! `host::Vm` implements `Future` by hand and links no executor (D92). Async
//! native words, the crate's other stated purpose, are not built; they need a
//! runtime, and that decision is untaken.

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
