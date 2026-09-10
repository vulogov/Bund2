//! No API. This crate exists to carry `benches/`; see `benches/interpret.rs`.
//!
//! Why in-process measurement exists at all is **Q14**: `cargo xtask bench`
//! times a subprocess, and across three whole-harness runs its total moved by
//! ~8 ms while the entire interpreted portion of the corpus is about 23 ms.
//! The noise is the same order as the signal, so no criterion for RFC-0001 or
//! RFC-0005 can rest on it.
