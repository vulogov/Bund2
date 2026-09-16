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

/// `BUND2_JIT_THRESHOLD`, when it names a number — F125.
///
/// **A malformed value is ignored rather than refused**, because this is read
/// in a constructor that cannot report. That would be a silent
/// misconfiguration, which is the failure mode F124, F127 and F128 all share —
/// so `Runtime::jit_threshold` reports what was *actually* adopted, and `bund2
/// --stats` prints it. The knob is observable even when the value was not
/// understood.
fn threshold_from_env() -> Option<u32> {
    std::env::var("BUND2_JIT_THRESHOLD")
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()
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
        Self::with_options_and_threshold(opts, None)
    }

    /// As [`Runtime::with_options`], with §S7's promotion threshold overridden.
    ///
    /// **`--jit-threshold`, which criterion 2's third run needs** (F125). The
    /// threshold is how many evaluations of one body earn it compilation;
    /// `Caps::default` fixes it at 64, and most corpus programs are far too
    /// short to reach that — so the corpus was only ever exercised at a setting
    /// where the tier does almost nothing. At 1 every body compiles on its
    /// first evaluation, which is the strongest test of *meaning* the corpus
    /// can give.
    ///
    /// **It is a tuning knob, not a semantic boundary** (§S7). If conformance
    /// moves when it changes, that is a defect the flag has found, not a defect
    /// in the flag.
    ///
    /// `None` keeps §S7's default. The parameter is taken even without the
    /// `jit` feature, where it is ignored: a build with no tier still has to
    /// accept the same command line, or a uniform `conform` invocation would
    /// fail on the run that has no tier to configure.
    pub fn with_options_and_threshold(
        opts: &bund2_stdlib::host::HostOptions,
        threshold: Option<u32>,
    ) -> Self {
        // **Flag, then environment, then §S7's default.** The flag wins because
        // it is the more specific statement: a command line is about *this*
        // run, an environment variable about the shell it happened to run in.
        let threshold = threshold.or_else(threshold_from_env);
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
            // **D47 and D48's table, taken at the same moment and for the same
            // reason** (D68). `promotable::crossable` keys on the registration
            // ids `register_all_with` just minted, so it must be taken after
            // registration; a tier holds only a `&mut dyn Vm` and could not
            // build it itself.
            let crossable = bund2_stdlib::promotable::crossable(&interp.registry);
            let caps = bund2_jit::cache::Caps {
                threshold: threshold.unwrap_or(bund2_jit::cache::Caps::default().threshold),
                ..bund2_jit::cache::Caps::default()
            };
            interp.tier = Some(Box::new(
                JitTier::with_fragments(caps, table).with_crossable(crossable),
            ));
        }
        #[cfg(not(feature = "jit"))]
        let _ = threshold;
        Self { interp }
    }

    /// **The threshold the tier is actually using**, so a report can name it.
    ///
    /// `None` without the feature. A run whose threshold came from the
    /// environment, or whose `BUND2_JIT_THRESHOLD` was malformed and ignored,
    /// is otherwise indistinguishable from one at the default — and a knob that
    /// looks set and is not is the shape of F124, F127 and F128. `bund2
    /// --stats` prints this beside the compiled-body count.
    #[cfg(feature = "jit")]
    pub fn jit_threshold(&self) -> Option<u32> {
        self.interp.tier.as_ref().and_then(|t| t.threshold())
    }

    /// `None` without the feature: there is no tier to configure.
    #[cfg(not(feature = "jit"))]
    pub fn jit_threshold(&self) -> Option<u32> {
        None
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

    /// **How many values the tier promoted** — the third figure a timing needs
    /// to be attributable. See [`bund2_api::Tier::promoted_values`].
    pub fn promoted_values(&self) -> Option<usize> {
        self.interp.tier.as_ref().and_then(|t| t.promoted_values())
    }

    /// Values across every compiled body — the denominator for the three
    /// figures above. See [`bund2_api::Tier::compiled_values`], F136.
    pub fn compiled_values(&self) -> Option<usize> {
        self.interp.tier.as_ref().and_then(|t| t.compiled_values())
    }

    /// Bodies the promotion counter is tracking — §S7's counter, criterion 20.
    /// See [`bund2_api::Tier::counted_bodies`].
    pub fn counted_bodies(&self) -> Option<usize> {
        self.interp.tier.as_ref().and_then(|t| t.counted_bodies())
    }

    /// Install a tier, replacing any already there.
    #[cfg(feature = "jit")]
    pub fn install_tier(&mut self, tier: Box<dyn bund2_api::Tier>) {
        self.interp.tier = Some(tier);
    }
}
