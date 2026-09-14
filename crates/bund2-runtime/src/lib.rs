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
            // **§S6's fragment table, built here and nowhere else.** It is
            // keyed by the registration ids `register_all_with` just minted, so
            // it must be taken *after* registration — taken earlier it would be
            // empty, every site would compile generic, and nothing would report
            // why. `published` takes a `&Registry`, which this crate has and a
            // tier does not (D9 keeps `Fragment` out of `bund2-api`).
            let table = bund2_stdlib::fragments::published(&interp.registry).unwrap_or_default();
            interp.tier = Some(Box::new(JitTier::with_fragments(
                bund2_jit::cache::Caps::default(),
                table,
            )));
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

    /// **How many bodies the tier has compiled** — RFC-0005 criterion 2's
    /// statistics, which `bund2 --stats` reports.
    ///
    /// Criterion 2 asks for this by name: "`bund2` reports how many bodies it
    /// compiled when asked, through a statistics flag… A `jit` run that
    /// compiles no body over the corpus fails." Without it the only evidence a
    /// tier ran is timing, which cannot tell a compiled body from a fast
    /// interpreted one — and F124 is what that gap allowed.
    ///
    /// Asked here rather than after [`Runtime::take_tier`], because a
    /// `Box<dyn Tier>` cannot be narrowed back to a `JitTier`: the trait has
    /// one method and no `Any`, and widening it would put a reporting concern
    /// into `bund2-api` for every implementor.
    ///
    /// `None` without the feature, which is different from `Some(0)`: no tier
    /// at all, rather than a tier that compiled nothing.
    pub fn compiled_bodies(&self) -> Option<usize> {
        // Asked through the trait object the `Interp` holds: a `Box<dyn Tier>`
        // cannot be narrowed back to a `JitTier`, so the figure comes from a
        // defaulted trait method rather than a downcast.
        self.interp.tier.as_ref().and_then(|t| t.compiled_bodies())
    }

    /// **How many sites the tier inlined** — the figure that makes a timing
    /// attributable. See [`bund2_api::Tier::inlined_sites`].
    pub fn inlined_sites(&self) -> Option<usize> {
        self.interp.tier.as_ref().and_then(|t| t.inlined_sites())
    }

    /// Install a tier, replacing any already there.
    #[cfg(feature = "jit")]
    pub fn install_tier(&mut self, tier: Box<dyn bund2_api::Tier>) {
        self.interp.tier = Some(tier);
    }
}
