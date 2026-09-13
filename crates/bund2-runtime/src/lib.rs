//! VM assembly: word slot table, tiering policy, execution context.
//!
//! This is where Tier 0 and Tier 1 are put together. `bund2-interp` knows a
//! tier *might* exist — it holds an `Option<Box<dyn Tier>>` (RFC-0002's
//! 2026-09-13 amendment) — and nothing about what one is. `bund2-jit` knows how
//! to compile a body and nothing about when. This crate is the only one that
//! names both, which is what keeps Tier 1 optional by structure rather than by
//! feature flag alone.

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

use bund2_api::Error;
use bund2_interp::Interp;

#[cfg(feature = "jit")]
mod tier;

#[cfg(feature = "jit")]
pub use tier::JitTier;

/// One interpreter, assembled: the full vocabulary, and a tier when the feature
/// is on.
///
/// **The tier is not beside the `Interp`, it is inside it.** `Interp` owns the
/// `Box<dyn Tier>`, so a `Tiering` held here as a sibling field would be
/// unreachable while the tier was installed. The cache and counter therefore
/// live *within* [`JitTier`], and reaching them means taking the tier back out
/// — which [`Runtime::tier`] does.
pub struct Runtime {
    /// The interpreter. Public because an embedder needs its stacks, registry
    /// and reporter, exactly as the CLI does today.
    pub interp: Interp,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    /// An interpreter with the full vocabulary and, under `--features jit`, a
    /// tier installed with §S7's default caps.
    pub fn new() -> Self {
        Self::with_options(&bund2_stdlib::host::HostOptions::default())
    }

    /// As [`Runtime::new`], with the host options the CLI passes — `--noio`,
    /// `--noeval` and the rest register failing stubs under the same names.
    pub fn with_options(opts: &bund2_stdlib::host::HostOptions) -> Self {
        let mut interp = Interp::new();
        bund2_stdlib::register_all_with(&mut interp.registry, opts);
        #[cfg(feature = "jit")]
        {
            interp.tier = Some(Box::new(JitTier::default()));
        }
        Self { interp }
    }

    /// Run a source string.
    ///
    /// Parse failures and evaluation failures arrive the same way, as the CLI
    /// reports them.
    pub fn eval_str(&mut self, src: &str) -> Result<(), Error> {
        let stream = bund2_syntax::compile(src).map_err(|e| Error(format!("{e:?}")))?;
        self.interp.eval(&stream)
    }

    /// The tier, taken out for inspection.
    ///
    /// Taking rather than borrowing, because the tier holds the cache and the
    /// counter and `Interp` owns the tier: there is no way to look at them
    /// while they are installed. Put it back with [`Runtime::install_tier`].
    #[cfg(feature = "jit")]
    pub fn take_tier(&mut self) -> Option<Box<dyn bund2_api::Tier>> {
        self.interp.tier.take()
    }

    /// Install a tier, replacing any already there.
    #[cfg(feature = "jit")]
    pub fn install_tier(&mut self, tier: Box<dyn bund2_api::Tier>) {
        self.interp.tier = Some(tier);
    }
}
