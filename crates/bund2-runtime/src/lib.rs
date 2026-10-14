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

/// Re-exported so an embedder that builds VMs through `bund2-async`'s host
/// can say which restrictions they run under (D165) without depending on
/// `bund2-stdlib` itself.
pub use bund2_stdlib::host::HostOptions;
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
    /// **A defect found while assembling the runtime, kept until there is a
    /// reporter to give it to** — RFC-0005 §S6, the twenty-third review's S5.
    ///
    /// `build` runs before an embedder has installed its reporter, so a
    /// report made there would go to the default one, which discards.
    /// [`Runtime::take_startup_defect`] hands it over; [`Runtime::eval_str`]
    /// reports it itself before the first line runs.
    startup_defect: Option<Error>,
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

/// **Tier 0's part of an evaluation thread's stack — RFC-0005 §S8, F85.**
///
/// The size Bund2 ran on before the tier existed (8 MiB). **Not `8 MiB + m`**:
/// §S8 has Tier 0's part carry a margin `m` under `jit` over every re-entering
/// path, and says `m` is "a measurement taken when the tier exists, not a
/// number guessed now". Until then this stays at the tier-off size and the
/// share below is added *above* it rather than taken out of it, which is what
/// keeps D44's "never less".
pub const TIER0_PART: usize = 8 * 1024 * 1024;

/// **Tier 1's share — §S8's proposed default**, added to the region rather
/// than carved out of Tier 0's part. Zero without the feature, so a default
/// build asks for exactly the stack it asked for before.
#[cfg(feature = "jit")]
pub const TIER1_SHARE: usize = 8 * 1024 * 1024;
/// Tier 1's share, zero without the feature.
#[cfg(not(feature = "jit"))]
pub const TIER1_SHARE: usize = 0;

/// An evaluation thread's stack: Tier 0's part, plus Tier 1's share under
/// `jit`, plus the reserve beneath the floor.
pub const EVAL_STACK: usize = TIER0_PART + TIER1_SHARE + bund2_interp::STACK_RESERVE;

/// **Declare the running thread's stack region — call this first on any thread
/// that will evaluate Bund.**
///
/// **A share is declared, never inferred** (§S8, the ninth review's S3).
/// Declaring through `set_stack_region` instead would leave the share at zero,
/// putting the Tier 1 floor above the thread's own top so that **every
/// compiled body declines at entry while every figure still reports success**
/// — the failure RFC-0005 criterion 18 and `compiled_entries` exist to catch.
///
/// **It lives here because there is now more than one caller.** `bund2-cli`'s
/// `main` spawns one such thread; RFC-0007 §C8's host spawns N. Two copies of
/// three constants and a two-line call is two chances to omit the share, and
/// §C8 criterion 10 is precisely the assertion that no thread omitted it. One
/// function removes the opportunity rather than testing for it.
///
/// The thread's stack must have been created with [`EVAL_STACK`] bytes, which
/// is why both callers pass it to `stack_size`.
pub fn declare_region() {
    bund2_interp::set_stack_region_with_share(
        bund2_interp::stack_marker(),
        EVAL_STACK,
        TIER1_SHARE,
    );
}

impl Runtime {
    /// An interpreter with the full vocabulary and, under `--features jit`, a
    /// tier installed with §S7's default caps.
    pub fn new() -> Self {
        Self::with_options(&bund2_stdlib::host::HostOptions::default())
    }

    /// As [`Runtime::new`], with §S7's promotion threshold overridden and the
    /// default host options.
    ///
    /// The threshold is how many evaluations of one body earn it compilation;
    /// D74 ships 1024, so a short program compiles nothing at all (F139). A
    /// caller that needs the tier to *act* — a test, a benchmark, RFC-0007
    /// §C8's host under criterion 10 — says so with this rather than
    /// constructing `HostOptions` it does not care about.
    pub fn with_threshold(threshold: Option<u32>) -> Self {
        Self::with_options_and_threshold(&bund2_stdlib::host::HostOptions::default(), threshold)
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
        Self::build(opts, threshold, true)
    }

    /// **A debugged session installs no tier — RFC-0008 §D6.**
    ///
    /// Research §3.3(f) offers two mechanisms and the RFC takes the second,
    /// "disable tiering globally", because the first — pinning a breakpointed
    /// word to Tier 0 by reusing the async colouring machinery — depends on
    /// machinery that does not exist. So this needs nothing new: the tier is
    /// installed in one place, and a debug session declines.
    ///
    /// **Every body is then Tier 0: steppable, with no opaque frames.** A
    /// compiled body runs without entering the loop that carries §D1's
    /// safepoint, so a debugger that left the tier installed would step over
    /// exactly the bodies a program spends its time in.
    ///
    /// **What it costs, stated:** a debugged program cannot observe the tier's
    /// behaviour. That is the right trade — a debugger that changes what it
    /// observes is worse than a slow one — and F139 measured that no corpus
    /// program compiles a body at the default threshold anyway.
    ///
    /// The threshold is still taken, and still ignored, for the reason
    /// [`Runtime::with_options_and_threshold`] takes it without the feature: a
    /// uniform command line must parse whether or not anything will use it.
    pub fn for_debugging(opts: &bund2_stdlib::host::HostOptions, threshold: Option<u32>) -> Self {
        Self::build(opts, threshold, false)
    }

    /// The one place a tier is installed, so §D6 is one `if` and not a policy
    /// spread over the constructors.
    fn build(opts: &bund2_stdlib::host::HostOptions, threshold: Option<u32>, tier: bool) -> Self {
        // **Flag, then environment, then §S7's default.** The flag wins because
        // it is the more specific statement: a command line is about *this*
        // run, an environment variable about the shell it happened to run in.
        let threshold = threshold.or_else(threshold_from_env);
        let mut interp = Interp::new();
        #[cfg_attr(not(feature = "jit"), allow(unused_mut))]
        let mut startup_defect = None;
        bund2_stdlib::register_all_with(&mut interp.registry, opts);
        // RFC-0008 §W2: the interpreter has no parser, so the line a console
        // is handed at a stop is run by the vocabulary's own `bund.eval`.
        interp.evaluator = Some(bund2_stdlib::eval_line);
        #[cfg(feature = "jit")]
        if tier {
            // **§S6's fragment table, built here and nowhere else.** It is
            // keyed by the registration ids `register_all_with` just minted, so
            // it must be taken *after* registration — taken earlier it would be
            // empty, every site would compile generic, and nothing would report
            // why. `published` takes a `&Registry`, which this crate has and a
            // tier does not (D9 keeps `Fragment` out of `bund2-api`).
            //
            // **A malformed fragment is kept and reported, not swallowed.**
            // `published` returns a `Result` so that this consumer can say
            // so (D37); `unwrap_or_default` here turned it into the empty
            // table the paragraph above warns about.
            let table = match bund2_stdlib::fragments::published(&interp.registry) {
                Ok(table) => table,
                Err(e) => {
                    startup_defect = Some(Error::internal(format!(
                        "a fragment `bund2-stdlib` publishes is malformed ({e}), so the tier \
                         has no fragment table and will inline nothing; programs run \
                         correctly and no faster than without the tier"
                    )));
                    Vec::new()
                }
            };
            // **D47 and D48's table, taken at the same moment and for the same
            // reason** (D68). `promotable::crossable` keys on the registration
            // ids `register_all_with` just minted, so it must be taken after
            // registration; a tier holds only a `&mut dyn Vm` and could not
            // build it itself.
            let caps = bund2_jit::cache::Caps {
                threshold: threshold.unwrap_or(bund2_jit::cache::Caps::default().threshold),
                ..bund2_jit::cache::Caps::default()
            };
            // **No crossable table — D75 withdraws D68's crossing from use.**
            //
            // `JitTier::with_crossable` is *not* chained here, so the set stays
            // empty and promotion syncs before every call: "the conservative
            // answer, and the behaviour every caller had before D68".
            //
            // The rule D68 states is unchanged and the machinery is intact —
            // `promotable::crossable` still builds the table, every gate still
            // answers, and the tests and benches that exercise crossing pass it
            // in explicitly. What is withdrawn is shipping it, on the
            // measurement: on the shape crossing was designed for it **costs**
            // ~1.7 ns a crossed call rather than saving, because each one now
            // carries F143's generation compare and D73's epoch compare and
            // those exceed the push and pop they avoid. Restoring it is this
            // one call.
            interp.install_tier(std::rc::Rc::new(JitTier::with_fragments(caps, table)));
        }
        #[cfg(not(feature = "jit"))]
        let _ = (threshold, tier);
        Self {
            interp,
            startup_defect,
        }
    }

    /// The defect `build` found, if any, once. An embedder that drives
    /// `interp` directly reports it after installing its reporter, as the CLI
    /// does; one that uses [`Runtime::eval_str`] need not ask.
    pub fn take_startup_defect(&mut self) -> Option<Error> {
        self.startup_defect.take()
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
        self.interp.tier().and_then(|t| t.threshold())
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
        // A `Warning`: the program runs, and runs correctly (D36).
        if let Some(e) = self.startup_defect.take() {
            bund2_api::Vm::report(&mut self.interp, bund2_api::diag::Diagnostic::warning(e.0));
        }
        let stream = bund2_syntax::compile(src).map_err(|e| Error(format!("{e:?}")))?;
        self.interp.eval(&stream)
    }

    /// The tier, taken out for inspection.
    ///
    /// Taking rather than borrowing, because the tier holds the cache and the
    /// counter and `Interp` owns the tier: there is no way to look at them
    /// while they are installed. Put it back with [`Runtime::install_tier`].
    #[cfg(feature = "jit")]
    pub fn take_tier(&mut self) -> Option<std::rc::Rc<dyn bund2_api::Tier>> {
        self.interp.take_tier()
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
        self.interp.tier().and_then(|t| t.compiled_bodies())
    }

    /// **How many sites the tier inlined** — the figure that makes a timing
    /// attributable. See [`bund2_api::Tier::inlined_sites`].
    pub fn inlined_sites(&self) -> Option<usize> {
        self.interp.tier().and_then(|t| t.inlined_sites())
    }

    /// **How many values the tier promoted** — the third figure a timing needs
    /// to be attributable. See [`bund2_api::Tier::promoted_values`].
    pub fn promoted_values(&self) -> Option<usize> {
        self.interp.tier().and_then(|t| t.promoted_values())
    }

    /// Values across every compiled body — the denominator for the three
    /// figures above. See [`bund2_api::Tier::compiled_values`], F136.
    pub fn compiled_values(&self) -> Option<usize> {
        self.interp.tier().and_then(|t| t.compiled_values())
    }

    /// Bodies the promotion counter is tracking — §S7's counter, criterion 20.
    /// See [`bund2_api::Tier::counted_bodies`].
    pub fn counted_bodies(&self) -> Option<usize> {
        self.interp.tier().and_then(|t| t.counted_bodies())
    }

    /// Calls promotion may cross — D68. See [`bund2_api::Tier::crossed_calls`].
    pub fn crossed_calls(&self) -> Option<usize> {
        self.interp.tier().and_then(|t| t.crossed_calls())
    }

    /// **How many entries compiled code actually ran** — see
    /// `bund2_api::Tier::compiled_entries`. The only figure here that reports
    /// what *ran* rather than what the compiler did.
    pub fn compiled_entries(&self) -> Option<usize> {
        self.interp.tier().and_then(|t| t.compiled_entries())
    }

    /// Entries the Tier 1 floor declined, and the compiled depth at the first
    /// — see `bund2_api::Tier::floor_declines`.
    pub fn floor_declines(&self) -> Option<(usize, Option<usize>)> {
        self.interp.tier().and_then(|t| t.floor_declines())
    }

    /// Install a tier, replacing any already there.
    #[cfg(feature = "jit")]
    pub fn install_tier(&mut self, tier: Box<dyn bund2_api::Tier>) {
        self.interp.install_tier(std::rc::Rc::from(tier));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Keeping(std::rc::Rc<std::cell::RefCell<Vec<String>>>);

    impl bund2_api::diag::Reporter for Keeping {
        fn report(&mut self, d: &bund2_api::diag::Diagnostic) {
            self.0.borrow_mut().push(d.reason.clone());
        }
    }

    /// **A defect found while the runtime was assembled reaches the reporter
    /// installed afterwards, once** — the twenty-third review's S5.
    ///
    /// `fragments::published` cannot be made to fail from here: its three
    /// fragments are constants and each validates. So the defect is put where
    /// `build` would put it, and what is tested is everything after that —
    /// that it waits for a reporter, is reported as a warning before the first
    /// line runs, says it is Bund2's, and does not stop the program.
    #[test]
    fn a_startup_defect_waits_for_the_reporter_and_is_reported_once() {
        let mut rt = Runtime::new();
        assert!(
            rt.startup_defect.is_none(),
            "the shipped fragments are well formed"
        );
        rt.startup_defect = Some(Error::internal("a fragment is malformed"));

        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        rt.interp.reporter = Box::new(Keeping(std::rc::Rc::clone(&seen)));
        rt.eval_str("1 2 +").expect("the program still runs");
        rt.eval_str("drop").expect("and goes on");

        let seen = seen.borrow();
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(
            seen[0].starts_with("internal error: a fragment is malformed"),
            "{seen:?}"
        );
        assert!(
            rt.take_startup_defect().is_none(),
            "reported, so nothing is left to take"
        );
    }
}
