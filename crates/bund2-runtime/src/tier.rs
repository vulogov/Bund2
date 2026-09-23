//! **The tiering policy** — RFC-0005 §S7, wired to the seam.
//!
//! This is the only place that knows both when a body has earned compilation
//! (`bund2-jit`'s `Tiering`) and where a body starts running (`bund2-interp`'s
//! `Tier` seam). Neither crate names the other.

use bund2_api::{Error, Tier, Vm};
use bund2_jit::cache::{Caps, Decision, Tiering};
use bund2_jit::lower::{Compiler, LastCall};
use bund2_value::BundValue;

/// A body's values, whichever container carries it.
///
/// `Interp` accepts a LAMBDA or a LIST for a frame's body (`frame_items`), so
/// the tier must too — a `Vec` assembled at run time arrives as a LIST.
fn items(body: &BundValue) -> Option<&[BundValue]> {
    body.as_lambda().or_else(|| body.as_list())
}

/// The tier: §S7's policy over `bund2-jit`'s cache and counter.
///
/// **It owns the `Tiering`.** `Interp` owns the `Box<dyn Tier>`, so a cache
/// held beside the `Interp` would be unreachable while the tier was installed.
pub struct JitTier {
    tiering: Tiering,
    /// **The code, and the one `JITModule` behind it.**
    ///
    /// One compiler per tier, one tier per `Interp`, so one module per `Interp`
    /// — criterion 23. The cache holds handles into *this* compiler, which is
    /// why the two must live together: a handle is meaningless without it.
    ///
    /// `None` until the first body earns compilation. A program that never goes
    /// hot never reserves code memory, and a compiler that cannot be built
    /// leaves the tier interpreting rather than failing the program.
    compiler: Option<Compiler>,
    /// **§S6's published fragments, carried down to the compiler.**
    ///
    /// Built by [`crate::Runtime`] rather than fetched here:
    /// `bund2_stdlib::fragments::published` takes a `&Registry`, and a tier
    /// holds only a `&mut dyn Vm`, which §S6 deliberately gives no registry. A
    /// `Vm` method answering with fragments would put `Fragment` into
    /// `bund2-api`, which D9 forbids.
    ///
    /// Empty means no inlining — every site takes the generic path, which is
    /// correct and merely slower.
    table: Vec<(bund2_api::RegistrationId, bund2_ir::Fragment)>,
    /// **D47 and D48's table, carried down for the same reason the fragments
    /// are** — `bund2_stdlib::promotable::crossable` takes a `&Registry`, which
    /// a tier does not hold.
    ///
    /// Empty means no call is classified crossable, which is the conservative
    /// answer and what a tier built by [`JitTier::new`] or
    /// [`JitTier::with_fragments`] gets.
    crossable: std::collections::BTreeSet<bund2_api::RegistrationId>,
}

impl Default for JitTier {
    /// A tier that inlines nothing. [`JitTier::with_fragments`] is what a
    /// `Runtime` uses, because only it can build the table.
    fn default() -> Self {
        Self::new(Caps::default())
    }
}

impl JitTier {
    pub fn new(caps: Caps) -> Self {
        Self::with_fragments(caps, Vec::new())
    }

    /// A tier with §S6's fragment table, so a compiled body can inline.
    pub fn with_fragments(
        caps: Caps,
        table: Vec<(bund2_api::RegistrationId, bund2_ir::Fragment)>,
    ) -> Self {
        Self {
            tiering: Tiering::new(caps),
            compiler: None,
            table,
            crossable: std::collections::BTreeSet::new(),
        }
    }

    /// **D47 and D48's table, added to a tier** — the set of registrations
    /// promotion may cross, from `bund2_stdlib::promotable::crossable`.
    ///
    /// A builder rather than a fourth parameter on [`JitTier::with_fragments`]:
    /// that signature has five call sites, and every one of them means "a tier
    /// that inlines", not "a tier that crosses". Without this the set stays
    /// empty, which classifies no call as crossable — the conservative answer,
    /// and the behaviour every caller had before D68.
    #[must_use]
    pub fn with_crossable(
        mut self,
        crossable: std::collections::BTreeSet<bund2_api::RegistrationId>,
    ) -> Self {
        self.crossable = crossable;
        self
    }

    /// The cache and counter, for a test or an embedder that wants the figures.
    pub fn tiering(&self) -> &Tiering {
        &self.tiering
    }

    /// How many words this tier has compiled into its module.
    pub fn compiled_words(&self) -> usize {
        self.compiler.as_ref().map_or(0, Compiler::compiled_words)
    }
}

impl JitTier {
    /// The figure [`bund2_api::Tier::compiled_bodies`] reports.
    fn compiled(&self) -> usize {
        self.compiled_words()
    }
}

impl Tier for JitTier {
    fn compiled_bodies(&self) -> Option<usize> {
        Some(self.compiled())
    }

    fn inlined_sites(&self) -> Option<usize> {
        Some(self.compiler.as_ref().map_or(0, Compiler::inlined_total))
    }

    fn promoted_values(&self) -> Option<usize> {
        Some(self.compiler.as_ref().map_or(0, Compiler::promoted_total))
    }

    fn compiled_values(&self) -> Option<usize> {
        Some(self.compiler.as_ref().map_or(0, Compiler::values_total))
    }

    fn counted_bodies(&self) -> Option<usize> {
        Some(self.tiering.counted())
    }

    fn crossed_calls(&self) -> Option<usize> {
        Some(self.compiler.as_ref().map_or(0, Compiler::crossable_total))
    }

    fn threshold(&self) -> Option<u32> {
        Some(self.tiering.caps().threshold)
    }

    /// See `bund2_api::Tier` for the contract. In short: `None` interprets,
    /// `Some(..)` means compiled code ran the body.
    ///
    /// # Why a compiling entry still interprets
    ///
    /// `Decision::Compile` compiles the body, files it, and returns `None`.
    /// The body runs interpreted *this* time and the next entry hits the
    /// cache. Running the freshly compiled code immediately would be possible,
    /// but it would make the first entry after the threshold behave differently
    /// from every other — and the threshold is a tuning knob (§S7), not a
    /// semantic boundary, so the fewer behaviours that hang off it the better.
    ///
    /// # Why this cannot recurse into itself
    ///
    /// The seam **takes the tier out of the `Interp`** while this runs, so a
    /// body entered during compiled execution finds `None` and is interpreted.
    /// That is not merely a lost optimisation: without it, a self-recursive
    /// word would enter compiled code, whose `jit_apply` calls `Vm::apply`,
    /// which reaches `push_frame`, which would enter compiled code again —
    /// one Rust frame per Bund level, breaking RFC-0003's criterion 2 and
    /// making promotion "a conformance change, not an optimisation" (§S8). At
    /// most one compiled body runs at a time.
    fn enter(&mut self, body: &BundValue, vm: &mut dyn Vm) -> Option<Result<(), Error>> {
        match self.tiering.observe(body) {
            Decision::Interpret => None,
            Decision::Compile => {
                // A body whose values cannot be read is not one this tier
                // compiles; Tier 0 will report whatever is wrong with it.
                let len = items(body)?.len();
                if len > 0 {
                    // The module is built on the first promotion and kept for
                    // the tier's life. Failing to build one is treated exactly
                    // as a failed compilation, one line down.
                    let compiler = match self.compiler {
                        Some(ref mut c) => c,
                        // The table is cloned rather than moved: the tier may
                        // outlive a compiler that failed to build, and §S6's
                        // fragments are small and built once per `Runtime`.
                        // **Both tables, for the same reason.** §S6's fragments
                        // say what may be inlined; D47/D48's say what may be
                        // crossed. Neither is reachable from a `&mut dyn Vm`,
                        // so both are cloned down from the tier (D68).
                        None => match Compiler::with_crossable(
                            self.table.clone(),
                            self.crossable.clone(),
                        ) {
                            Ok(c) => self.compiler.insert(c),
                            Err(_) => return None,
                        },
                    };
                    // **§S6's addressing.** The emitted body loads the request
                    // cell through this address, so a `Vm` that keeps no cells
                    // gets no compiled body — the lowering refuses a zero base
                    // rather than emitting a load from null.
                    let cells = vm.cells().map(bund2_api::Cells::base)?;
                    // A failure to compile is not the program's fault and not
                    // its problem: the body is interpreted, as it would have
                    // been with no tier at all. It is dropped rather than
                    // reported because there is no diagnostic a *user* could
                    // act on, and the body still runs correctly.
                    // **The body itself, not just its length** — §S6's join
                    // decides per value whether to inline, which needs the
                    // values. `items` answered `Some` above, and the body is
                    // cloned because planning borrows `vm` mutably to ask
                    // `inline_site` while `body` borrows it immutably.
                    let values: Vec<BundValue> = items(body)?.to_vec();
                    // **F133: a body the tier cannot help is not compiled.**
                    // Every value that is not an inlined site goes through
                    // `Vm::apply` anyway — Tier 0's own path — with the boundary
                    // added around it, so a body with no site and nothing to
                    // promote is strictly slower compiled: **+24%** measured on
                    // `arith/float_mul/1000`, at 0 sites and 0 promoted.
                    //
                    // The demotion is what makes this affordable. Refusing alone
                    // would leave the counter hot, so every later entry would
                    // re-plan the body to reach the same answer; demoted, it is
                    // turned away at the top of `observe` instead. It also keeps
                    // the body out of §S7's 1024 function slots, which it would
                    // otherwise occupy to no purpose.
                    if !compiler.would_gain(&values, vm) {
                        self.tiering.demote(body);
                        return None;
                    }
                    if let Ok(code) = compiler.compile_word(&values, LastCall::Ordinary, cells, vm)
                    {
                        self.tiering.insert(body, code);
                    }
                }
                None
            }
            // **The handle arrives with the decision** — F130. This used to
            // call `self.tiering.compiled(body)`, which recomputed
            // `payload_key` and probed the cache a second time for the entry
            // `observe` had just found. That cost ~93 ns per entry against
            // 22.44 ns for a whole interpreted word, and it is why criterion 7
            // failed on `arith` and `dispatch`.
            Decision::Compiled(code) => {
                let values = items(body)?;
                // The handle is `Copy` and inert; the code it names lives in
                // this tier's own compiler, which is what makes running it
                // safe. A cache without its compiler simply interprets.
                let compiler = self.compiler.as_ref()?;
                // **§S5's reporter rule, read at each body's entry** — D71,
                // F137. A body that crosses a call holds values in registers
                // across it; a native reporting a `Warning` or `Notice` there
                // would snapshot a stack without them. The reporter is asked
                // *here*, not at compile time, because the CLI replaces it
                // after the `Interp` is built and the field is public, so the
                // answer can differ between two entries of the same body.
                //
                // Declining hands the body to Tier 0, which is always correct
                // and costs only the speed this body would have gained. It
                // stays compiled: the reporter may be swapped back, and
                // recompiling would pay F130's 125-141 µs to reach the same
                // code.
                //
                // **This is the second of two locks.** D71 already keeps every
                // native that reports mid-body out of the crossable table, so
                // under today's vocabulary this gate never changes an outcome.
                // It is what keeps the rule true if a native gains a report and
                // the table is not updated with it.
                if compiler.crossable_calls(code).is_some_and(|n| n > 0)
                    && (vm.wants_stack(bund2_api::diag::Severity::Warning)
                        || vm.wants_stack(bund2_api::diag::Severity::Notice))
                {
                    return None;
                }
                Some(compiler.run(code, vm, values))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A runtime whose tier compiles after `threshold` entries, so a test can
    /// reach Tier 1 in a few evaluations rather than sixty-four.
    fn runtime_with(threshold: u32) -> crate::Runtime {
        let mut r = crate::Runtime::new();
        r.install_tier(Box::new(JitTier::new(Caps {
            threshold,
            ..Caps::default()
        })));
        r
    }

    /// A runtime whose tier **can inline**, at a low threshold.
    ///
    /// [`runtime_with`] installs `JitTier::new`, whose own documentation says
    /// it is "a tier that inlines nothing" — its table is empty. Every tier
    /// test in this file used it, so **no test here has ever exercised §S6's
    /// inlining through the seam**; the lowering's own tests cover it, and the
    /// wiring between `register_all` and the table was covered only by the CLI
    /// (which is how F124 went unnoticed). This is that fixture.
    ///
    /// The table must be taken **after** registration, because it is keyed by
    /// the registration ids `register_all` mints — the same ordering
    /// `Runtime::with_options` documents.
    fn inlining_runtime_with(threshold: u32) -> crate::Runtime {
        let mut r = crate::Runtime::new();
        let table = bund2_stdlib::fragments::published(&r.interp.registry).unwrap_or_default();
        assert!(
            !table.is_empty(),
            "the fixture must publish fragments, or it tests the absence of inlining"
        );
        r.install_tier(Box::new(JitTier::with_fragments(
            Caps {
                threshold,
                ..Caps::default()
            },
            table,
        )));
        r
    }

    /// A runtime whose tier **can inline *and* cross**, at a low threshold.
    ///
    /// [`inlining_runtime_with`] builds its tier with `JitTier::with_fragments`
    /// alone, and `JitTier::with_crossable`'s own documentation says what that
    /// leaves behind: "Without this the set stays empty, which classifies no
    /// call as crossable." So a test of D68's crossing built on that fixture
    /// asserts nothing — the third instance of the shape F127 records, after
    /// `two_interps_share_no_compiled_code` and criterion 21's stack-switch
    /// rows, and the reason the precondition below is asserted rather than
    /// assumed.
    ///
    /// Both tables are taken **after** registration, because both are keyed by
    /// the registration ids `register_all` mints. This mirrors what
    /// `Runtime::with_options` does for a real run; it is a separate fixture
    /// rather than a change to `inlining_runtime_with` because the seven
    /// criterion-22 differentials use that one, and widening what they exercise
    /// is not this test's business.
    fn crossing_runtime_with(threshold: u32) -> crate::Runtime {
        let mut r = crate::Runtime::new();
        let table = bund2_stdlib::fragments::published(&r.interp.registry).unwrap_or_default();
        let crossable = bund2_stdlib::promotable::crossable(&r.interp.registry);
        assert!(
            !table.is_empty() && !crossable.is_empty(),
            "the fixture must publish fragments and admit crossings, or it tests \
             the absence of both"
        );
        r.install_tier(Box::new(
            JitTier::with_fragments(
                Caps {
                    threshold,
                    ..Caps::default()
                },
                table,
            )
            .with_crossable(crossable),
        ));
        r
    }

    /// `{ 1 2 + }` bound to a name, so calling it reaches `push_frame` the way
    /// a program does — dispatch, `request_tail`, `take_pending`.
    fn register_body(r: &mut crate::Runtime) -> BundValue {
        let body = BundValue::lambda(vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("+"),
        ]);
        r.interp.registry.register_lambda("f", body.clone());
        body
    }

    /// **F133: a body the tier cannot help is left interpreted.**
    ///
    /// The body is `{ 1.0 1.000001 * }`. Nothing in it is a site — §S6 publishes
    /// no fragment for float multiplication — and nothing in it promotes, since
    /// D66's `Plan::Literal` carries an `i64` and a float literal is not one. So
    /// a compiled form would run every value through `Vm::apply`, Tier 0's own
    /// path, and add the boundary around it: measured at **+24%** on
    /// `arith/float_mul/1000` before this rule existed.
    ///
    /// **The fixture must be the inlining one.** `runtime_with` installs an
    /// empty table, under which *every* body has zero sites, so it would pass
    /// this test for the wrong reason — the same shape as F127, where a fixture
    /// that could not inline was used to assert something about inlining.
    #[test]
    fn a_body_with_nothing_to_gain_is_not_compiled() {
        let mut r = inlining_runtime_with(2);
        let body = BundValue::lambda(vec![
            BundValue::float(1.0),
            BundValue::float(1.000001),
            BundValue::call("*"),
        ]);
        r.interp.registry.register_lambda("f", body.clone());

        for _ in 0..6 {
            r.interp
                .eval(&[BundValue::call("f")])
                .expect("the body runs");
            r.interp.clear();
        }

        assert_eq!(
            r.compiled_bodies(),
            Some(0),
            "a body with no site and nothing to promote must stay at Tier 0"
        );
    }

    /// **The other half of F133's rule, or it is only half tested.**
    ///
    /// The same path, the same fixture, a body that *does* inline: `{ 1 2 + }`
    /// has a published fragment for `+` and two int literals to promote. A rule
    /// that refused this too would "fix" the regression by turning the tier off,
    /// which is why both directions are asserted.
    #[test]
    fn a_body_with_a_site_still_compiles() {
        let mut r = inlining_runtime_with(2);
        register_body(&mut r);

        for _ in 0..6 {
            r.interp
                .eval(&[BundValue::call("f")])
                .expect("the body runs");
            r.interp.clear();
        }

        assert_eq!(
            r.compiled_bodies(),
            Some(1),
            "a body with an inlinable site must still be compiled"
        );
    }

    /// **The refusal is remembered, not re-decided.**
    ///
    /// Refusing without recording it would leave the counter past the threshold,
    /// so every later entry would re-plan the body to reach the same answer —
    /// paying `plan_body` forever to learn what was already known. The tier
    /// demotes instead, and `observe` turns a demoted body away before planning.
    /// Asserted through the seam: the result never changes and nothing is ever
    /// compiled, however many times it is entered.
    #[test]
    fn a_refused_body_is_demoted_rather_than_re_planned() {
        let mut r = inlining_runtime_with(1);
        let body = BundValue::lambda(vec![
            BundValue::float(2.5),
            BundValue::float(4.0),
            BundValue::call("*"),
        ]);
        r.interp.registry.register_lambda("f", body.clone());

        for _ in 0..40 {
            r.interp
                .eval(&[BundValue::call("f")])
                .expect("the body runs");
            assert_eq!(
                r.interp.depth(),
                1,
                "the product is the one value the body leaves"
            );
            r.interp.clear();
        }

        assert_eq!(
            r.compiled_bodies(),
            Some(0),
            "forty entries must not compile a body that cannot gain"
        );
    }

    /// **The whole chain, end to end.** Seam, counter, threshold, cache,
    /// lowering, trampoline, thunks and adapter, running as one: a body
    /// evaluated past the threshold is compiled and filed, and the next entry
    /// runs compiled code.
    #[test]
    fn a_hot_body_is_compiled_and_then_run_compiled() {
        let mut r = runtime_with(2);
        register_body(&mut r);

        // Five entries across the threshold of two: the first interpret, one
        // compiles and files, the rest run compiled code. Every entry must
        // leave the same thing, whichever tier ran it — that is the invariant a
        // tier may not move (§S2), and it is what this asserts. The counter
        // itself is not readable through `dyn Tier`, so the observable result
        // is the evidence rather than a count.
        for _ in 0..5 {
            r.eval_str("f").expect("runs");
        }
        let stack = r.interp.snapshot();
        assert_eq!(stack.len(), 5, "one sum per call: {stack:?}");
        for v in &stack {
            assert_eq!(v.as_int(), Some(3), "every call left 3: {stack:?}");
        }
    }

    /// **What `--stats` reports, asserted** — RFC-0005 criterion 10.
    ///
    /// `a_hot_body_is_compiled_and_then_run_compiled` says "the counter itself
    /// is not readable through `dyn Tier`, so the observable result is the
    /// evidence rather than a count". That was true when it was written and is
    /// no longer: three figures now come through the trait, and criterion 10's
    /// attribution rests on them — a timing that cannot tell "the lowering does
    /// not pay" from "the lowering did not happen" is the confusion those
    /// figures exist to prevent. So they need a test of their own, or the
    /// measurement rests on an unasserted number.
    #[test]
    fn the_tier_reports_what_it_compiled_inlined_and_promoted() {
        let mut r = inlining_runtime_with(2);
        register_body(&mut r);
        for _ in 0..5 {
            r.eval_str("f").expect("runs");
        }

        assert_eq!(r.compiled_bodies(), Some(1), "one body: `{{ 1 2 + }}`");
        assert_eq!(r.inlined_sites(), Some(1), "the `+`");
        assert_eq!(
            r.promoted_values(),
            Some(2),
            "both literals promoted; the `+` is a call, not a literal"
        );
        // **The denominator, and the only figure here that reports a cost**
        // (F136). `{ 1 2 + }` is three values: two promoted literals and one
        // inlined site, so nothing in it takes the generic path. A body whose
        // values exceed its sites plus promotions is paying 5.13 ns a value for
        // the remainder (criterion 10), which is what F133's rule cannot
        // currently see.
        assert_eq!(
            r.compiled_values(),
            Some(3),
            "three values: `1`, `2`, `+` — so zero generic, the body pays nothing extra"
        );

        // **`None` and `Some(0)` say different things**, which is the
        // distinction the CLI's match arms turn on: no tier at all, against a
        // tier that compiled nothing.
        let mut bare = crate::Runtime::new();
        bare.take_tier();
        assert_eq!(bare.compiled_bodies(), None, "no tier, not an empty one");
        assert_eq!(bare.inlined_sites(), None);
        assert_eq!(bare.promoted_values(), None);
        assert_eq!(bare.compiled_values(), None);
    }

    /// **Criterion 22's differential.** A body that promotes is run with a tier
    /// and without one, and every observable must match.
    ///
    /// The sibling of [`assert_switch_matches_tier0`], which serves criterion 21
    /// and prepends its own stack-switch preamble. Criterion 22's bullets are
    /// about promotion surviving a *rebinding*, not about switching stacks, so
    /// they need the same five-channel comparison without that setup.
    ///
    /// # The precondition, which is the whole point
    ///
    /// **Promotion is asserted before the comparison**, not assumed. A body that
    /// promoted nothing would make this a differential of Tier 0 against itself
    /// and every bullet would pass while exercising nothing — F127's shape, and
    /// how four earlier tests passed for a while. Two figures are checked: a
    /// body compiled at all, and at least one value promoted in it.
    ///
    /// The literals and the inlinable call must live **inside** the registered
    /// body. A body of literals alone is refused by F136's rule, and one whose
    /// literals sit in the caller promotes nothing — under `times`, the figure
    /// that comes back is the loop driver's, not the body's.
    ///
    /// # The fixture crosses, because the shipped tier does
    ///
    /// This used [`inlining_runtime_with`], which supplies §S6's fragments and
    /// **no crossable table**, so every call was synced and these differentials
    /// asserted promotion against a tier that could not cross one. That is the
    /// configuration no `Runtime` ever runs: `Runtime::with_options_and_threshold`
    /// chains `.with_crossable`. It now uses [`crossing_runtime_with`], which
    /// supplies both tables, so what these bullets compare is what a program
    /// gets.
    ///
    /// **Two of the eight cross a call; six cross none, by construction.**
    /// The mid-body variants — an effect changed mid-body, an alias rebound
    /// mid-body — call `register` *inside* the body, and `register` is
    /// `eff(2, 0)`, certified and unpublished, so D68 crosses it. Those two now
    /// assert something the old fixture could not: that promotion may hold
    /// values in registers across a call **that rebinds a word slot**, and
    /// still agree with Tier 0.
    ///
    /// The other six cross nothing because their subjects are exactly what
    /// crossing excludes — a lambda callee (D46), a name reached through an
    /// alias (no registration id, so D47/D48 refuse it), and natives like
    /// `clear` that the audit never certified. Asserting a crossing in those
    /// would contradict the rule each is there to check, so the helper does not
    /// demand one.
    fn assert_promoted_matches_tier0(setup: &str, calls: usize, label: &str) {
        let tier_seen = SharedReporter::default();
        let mut tiered = crossing_runtime_with(1);
        tiered.interp.reporter = Box::new(tier_seen.clone());
        tiered.eval_str(setup).expect("setup runs");

        let mut tier_outcome = Ok(());
        for _ in 0..calls {
            if tier_outcome.is_ok() {
                tier_outcome = tiered.eval_str("w");
            }
        }

        assert!(
            tiered.compiled_bodies().unwrap_or(0) > 0,
            "{label}: nothing compiled, so this asserts a path the tier never took"
        );
        assert!(
            tiered.promoted_values().unwrap_or(0) > 0,
            "{label}: nothing promoted, so this says nothing about promotion"
        );

        let plain_seen = SharedReporter::default();
        let mut plain = crate::Runtime::new();
        plain.take_tier();
        plain.interp.reporter = Box::new(plain_seen.clone());
        plain.eval_str(setup).expect("setup runs");
        let mut plain_outcome = Ok(());
        for _ in 0..calls {
            if plain_outcome.is_ok() {
                plain_outcome = plain.eval_str("w");
            }
        }

        let a = observe(&mut tiered, tier_outcome, &tier_seen);
        let b = observe(&mut plain, plain_outcome, &plain_seen);
        assert_eq!(a.outcome, b.outcome, "{label}: outcome");
        assert_eq!(a.current, b.current, "{label}: current stack");
        assert_eq!(a.stacks, b.stacks, "{label}: stacks");
        assert_eq!(a.workbench, b.workbench, "{label}: workbench");
        assert_eq!(a.diagnostics, b.diagnostics, "{label}: diagnostics");
    }

    /// **Criterion 5's missing assertion, carried into criterion 22's
    /// differentials: the redefinition must actually change the result.**
    ///
    /// Criterion 5 asks to "`register` a word, force promotion of a caller,
    /// `register` it again with different behaviour, and **assert the caller's
    /// next result changes**". Criterion 22's rows exercise exactly those
    /// shapes — a rebound alias target, a rebound lambda callee — with bodies
    /// that inline `+`, but they assert only that the two *tiers* agree. A
    /// rebinding that silently failed to take would leave both tiers answering
    /// identically and every such row would pass while asserting nothing. That
    /// is F127's shape, and it is what
    /// `assert_redefinition_matches_tier0`'s expectations guard against in
    /// `crates/bund2-jit/src/lower.rs`.
    ///
    /// This is the same guard for the runtime-level rows. `control` is the
    /// setup **without** the redefinition; `changed` is the row's own. Tier 0
    /// runs both, and they must differ — then the tier is held to `changed`
    /// exactly as before.
    ///
    /// **What differs here is the outcome, not the stack.** These bodies end in
    /// `clear`, so the current stack is empty either way; what the rebinding
    /// changes is arity — `{ drop }` becomes `{ drop drop }`, which drops from
    /// an empty stack and fails. `Observed` carries the outcome beside the
    /// stacks, so the comparison sees it.
    fn assert_redefinition_changes_the_result(
        control: &str,
        changed: &str,
        calls: usize,
        label: &str,
    ) {
        let run_tier0 = |setup: &str| {
            let seen = SharedReporter::default();
            let mut rt = crate::Runtime::new();
            rt.take_tier();
            rt.interp.reporter = Box::new(seen.clone());
            rt.eval_str(setup).expect("setup runs");
            let mut outcome = Ok(());
            for _ in 0..calls {
                if outcome.is_ok() {
                    outcome = rt.eval_str("w");
                }
            }
            let observed = observe(&mut rt, outcome, &seen);
            observed
        };

        assert_ne!(
            run_tier0(control),
            run_tier0(changed),
            "{label}: the redefinition must change Tier 0's result, or this row \
             asserts only that two tiers agree about a change that never happened"
        );

        assert_promoted_matches_tier0(changed, calls, label);
    }

    /// **Criterion 22, bullet 1 — an error with values promoted.**
    ///
    /// "Compile `1 2 true +` with `1` promoted. `+`'s type guard declines, and
    /// its generic counterpart, the word called through its slot, returns
    /// `ADD returns error: Incompartible Y argument for the math operations`, as
    /// Tier 0 does. The report and the stack dump must match."
    ///
    /// The body is `1 2 + true + clear`: the first `+` inlines and promotes both
    /// literals, and the second is handed a `true` its type guard declines, so
    /// the generic path runs and fails **with values already promoted**. The
    /// failure must leave the same stacks and the same diagnostic in both arms —
    /// a lowering that lost a promoted value on the error path would differ
    /// here and nowhere else.
    ///
    /// `a_compiled_word_stops_at_the_first_failure`
    /// (`crates/bund2-jit/src/lower.rs`) covers this bullet's other half, a
    /// failing callee; this is the error-with-promotion case.
    #[test]
    fn an_error_with_values_promoted_matches_tier_zero() {
        assert_promoted_matches_tier0(
            ":w { 1 2 + true + clear } register\n",
            3,
            "an error with values promoted",
        );
    }

    /// **Criterion 22, bullet 2 — an effect changed at run time.**
    ///
    /// "Promote across a call, and rebind that call's name to a word with a
    /// different effect, once before the body runs and once mid-body through
    /// `register`. The stacks must match."
    ///
    /// `h` is registered as a lambda consuming one, then rebound to one
    /// consuming two. Compiled code that trusted the effect it saw at
    /// compile time would keep a value in a register the callee now consumes;
    /// D66/D67 sync before every call, so both arms must agree.
    #[test]
    fn an_effect_changed_before_the_body_runs_matches_tier_zero() {
        assert_promoted_matches_tier0(
            ":h { drop } register\n\
             :w { 1 2 + h clear } register\n\
             :h { drop drop } register\n",
            3,
            "an effect changed before the body runs",
        );
    }

    /// The same bullet's second half: the rebinding happens **mid-body**,
    /// through `register` inside the word itself, so the callee's effect
    /// changes between the body being compiled and the call being reached.
    #[test]
    fn an_effect_changed_mid_body_matches_tier_zero() {
        assert_promoted_matches_tier0(
            ":h { drop } register\n\
             :w { 1 2 + :h { drop drop } register h clear } register\n",
            3,
            "an effect changed mid-body",
        );
    }

    /// **Criterion 22, bullet 3 — an alias whose target is rebound.**
    ///
    /// "Promote across a call to `<-`, and rebind `stacks_left` to a lambda that
    /// consumes two, before the body runs and mid-body. The stacks must match.
    /// Repeat with `$stacks_left`."
    ///
    /// `<-` is a registered alias for `stacks_left`
    /// (`crates/bund2-stdlib/src/stack.rs`, `register_alias`), so this is the
    /// case where the name compiled code saw and the binding it reaches differ
    /// by an indirection — the one F93 showed `effect_of` used to get wrong.
    #[test]
    fn an_alias_whose_target_is_rebound_matches_tier_zero() {
        assert_redefinition_changes_the_result(
            ":w { 1 2 + <- clear } register\n",
            ":w { 1 2 + <- clear } register\n\
             :stacks_left { drop drop } register\n",
            3,
            "an alias whose target is rebound",
        );
    }

    /// The same bullet, rebound **mid-body**.
    #[test]
    fn an_alias_rebound_mid_body_matches_tier_zero() {
        assert_redefinition_changes_the_result(
            ":w { 1 2 + <- clear } register\n",
            ":w { 1 2 + :stacks_left { drop drop } register <- clear } register\n",
            3,
            "an alias rebound mid-body",
        );
    }

    /// **Criterion 22, bullet 4 — a lambda callee whose callee is rebound
    /// (D46).**
    ///
    /// "With `:g { drop } register  :f { g } register`, run a body `1 2 3 f`,
    /// and rebind `g` to `{ drop drop drop }` before the body runs. Then use
    /// `:f { :g { drop drop drop } register g } register`, which rebinds `g`
    /// during the call."
    ///
    /// D46: promotion never crosses a lambda, because a lambda's callee can be
    /// rebound underneath it and no guard on `f` would see it. The promoted
    /// values live in the outer body, where the sync before calling `f` is what
    /// makes both arms agree.
    #[test]
    fn a_lambda_callee_rebound_before_the_body_runs_matches_tier_zero() {
        assert_redefinition_changes_the_result(
            ":g { drop } register\n\
             :f { g } register\n\
             :w { 1 2 + f clear } register\n",
            ":g { drop } register\n\
             :f { g } register\n\
             :w { 1 2 + f clear } register\n\
             :g { drop drop } register\n",
            3,
            "a lambda callee rebound before the body runs",
        );
    }

    /// The same bullet's second half: `f` rebinds `g` **during** its own call.
    #[test]
    fn a_lambda_callee_rebound_during_the_call_matches_tier_zero() {
        assert_redefinition_changes_the_result(
            ":g { drop } register\n\
             :f { g } register\n\
             :w { 1 2 + f clear } register\n",
            ":g { drop } register\n\
             :f { :g { drop drop } register g } register\n\
             :w { 1 2 + f clear } register\n",
            3,
            "a lambda callee rebound during the call",
        );
    }

    /// **Criterion 22, bullet 5 — a lambda that shadows a native (F93).**
    ///
    /// "After `:drop { 1 } register`, run a promoted body `1 2 3 drop`. `drop`
    /// resolves to the lambda, so nothing stays promoted across it, although
    /// the slot still holds the native's `(1, 0)`."
    ///
    /// **The criterion's original wording recursed.** It read
    /// `:drop { drop drop } register` — a lambda calling the name it shadows,
    /// which recurses without bound and is killed rather than answering. F93's
    /// entry uses `:drop { 1 } register`, where `5 drop` leaves `5 1`, and the
    /// repository owner settled on that shape (2026-09-16). It keeps the
    /// bullet's teeth: the lambda's real effect, `(0, 1)`, differs from the
    /// `(1, 0)` the slot still declares, so a tier trusting the declared effect
    /// diverges in the stacks rather than hiding.
    ///
    /// **Two things must hold at once, and `drop` is the one native where they
    /// meet.** It is a D68 survivor, so its declared `(1, 0)` would make it a
    /// callee promotion may cross; and it is one of §S6's three published
    /// fragments, so a tier reading the slot would also *inline* it. Shadowed,
    /// it must do neither. The `+` is what leaves the body a site, since the
    /// shadowed `drop` no longer supplies one — which is also why this body
    /// survives F136's rule.
    #[test]
    fn a_lambda_shadowing_a_native_matches_tier_zero() {
        assert_promoted_matches_tier0(
            ":drop { 1 } register\n\
             :w { 1 2 + drop clear } register\n",
            3,
            "a lambda that shadows a native",
        );
    }

    /// **Criterion 22, bullet 6 — the reporter, observed through the
    /// diagnostic.** D71, F137, §S5.
    ///
    /// "Under `CollectingReporter` with `wants_stack` set, run a promoted body
    /// … `alias` is `eff(2, 0)` and warns mid-body that `x` resolves back to
    /// itself, with values promoted below its arity. The warning's snapshot in
    /// `CollectingReporter::seen` must equal Tier 0's … That snapshot is the
    /// seam that shows whether anything was held across the call."
    ///
    /// **This is the bullet the whole reporter rule exists for.** If a value
    /// were held in a register across `alias`, the snapshot it takes would be
    /// short by exactly that value, and the two tiers would disagree in the
    /// diagnostic while agreeing everywhere else. D71 makes that impossible
    /// structurally — `alias` reports mid-body, so it is excluded from the
    /// crossable table and nothing is ever held across it.
    ///
    /// **The body deviates from the criterion's `1 2 :x :x alias`, recorded
    /// rather than substituted.** That body has no inlinable site, so F136's
    /// rule refuses to compile it and the comparison would be Tier 0 against
    /// itself — F127's shape, and the same trap bullet 5's wording carried.
    /// `1 2 + nl :x :x alias` keeps what the bullet asks for and pays F136:
    /// `+` publishes an arm, so it inlines and promotes the sum; `nl` is
    /// `eff(0, 0)`, certified and unpublished, so it is a generic call D68
    /// crosses, leaving the sum in a register across it; then `alias` — which
    /// D71 refuses to cross — forces the sync, and warns with the sum on the
    /// stack where Tier 0 also has it.
    #[test]
    fn a_mid_body_warning_carries_tier_zeros_snapshot() {
        let setup = ":w { 1 2 + nl :x :x alias clear } register\n";

        let seen = WantingReporter::default();
        let mut tiered = crossing_runtime_with(1);
        tiered.interp.reporter = Box::new(seen.clone());
        tiered.eval_str(setup).expect("setup runs");

        let mut outcome = Ok(());
        for _ in 0..3 {
            if outcome.is_ok() {
                outcome = tiered.eval_str("w");
            }
        }

        assert!(
            tiered.compiled_bodies().unwrap_or(0) > 0,
            "nothing compiled, so this compares Tier 0 with itself"
        );
        assert!(
            tiered.promoted_values().unwrap_or(0) > 0,
            "nothing promoted, so the snapshot says nothing about what was held"
        );

        let plain_seen = WantingReporter::default();
        let mut plain = crate::Runtime::new();
        plain.take_tier();
        plain.interp.reporter = Box::new(plain_seen.clone());
        plain.eval_str(setup).expect("setup runs");
        let mut plain_outcome = Ok(());
        for _ in 0..3 {
            if plain_outcome.is_ok() {
                plain_outcome = plain.eval_str("w");
            }
        }

        let a = observe(&mut tiered, outcome, &SharedReporter(seen.0.clone()));
        let b = observe(&mut plain, plain_outcome, &SharedReporter(plain_seen.0.clone()));
        assert!(
            a.diagnostics.iter().any(|(sev, _, _)| sev == "Warning"),
            "`alias` did not warn, so this test asserts nothing: {:?}",
            a.diagnostics
        );
        assert_eq!(a.diagnostics, b.diagnostics, "the warning itself");

        // **The snapshot is not in `Observed`.** Its diagnostic channel carries
        // severity, reason and `stack_name`; the snapshot bullet 6 is about is
        // `Diagnostic::stack`, a separate field. Comparing `a.diagnostics`
        // alone would assert nothing about what was held across the call, which
        // is the only thing this bullet exists to check — so the reporters are
        // read directly rather than widening `Observed`, which seven other
        // differentials share.
        let snaps = |w: &WantingReporter| -> Vec<(Option<Vec<String>>, Option<Vec<String>>)> {
            w.0.borrow()
                .iter()
                .filter(|d| d.severity == bund2_api::diag::Severity::Warning)
                .map(|d| (d.stack.clone(), d.workbench.clone()))
                .collect()
        };
        let tier_snaps = snaps(&seen);
        let plain_snaps = snaps(&plain_seen);

        // Both sides `None` would compare equal and prove nothing — the trap
        // this test was written into once already. The snapshot must exist and
        // must contain the promoted sum, which is the value a crossing would
        // have left in a register and hidden from `alias`.
        assert!(
            tier_snaps
                .iter()
                .any(|(s, _)| s.as_ref().is_some_and(|rows| !rows.is_empty())),
            "the warning carried no stack snapshot, so this asserts nothing: \
             {tier_snaps:?}"
        );
        assert!(
            tier_snaps.iter().any(|(s, _)| s
                .as_ref()
                .is_some_and(|rows| rows.iter().any(|r| r.contains('3')))),
            "the promoted sum is missing from the snapshot, which is exactly \
             what a value held across `alias` would look like: {tier_snaps:?}"
        );
        assert_eq!(
            tier_snaps, plain_snaps,
            "the mid-body warning's snapshot must be Tier 0's, value for value"
        );
        assert_eq!(a.outcome, b.outcome, "outcome");
        assert_eq!(a.current, b.current, "current stack");
        assert_eq!(a.stacks, b.stacks, "stacks");
        assert_eq!(a.workbench, b.workbench, "workbench");
    }

    /// **Criterion 22, bullet 6's second half.** "Then, under
    /// `TextReporter::new(true)`, the lowering's side table (criterion 17) must
    /// record at least one call crossed by promotion in the same body. Under
    /// the default reporter, promotion across calls must not be zero (D45)."
    ///
    /// `TextReporter::new(true)` is the CLI's own configuration, and its
    /// `wants_stack` is `dump_stack && severity.is_fatal()`
    /// (`crates/bund2-stdlib/src/report.rs`) — so it wants a snapshot for a
    /// fatal report and never for a warning. That is what makes it the right
    /// reporter for this half: D71's dynamic gate does **not** fire under it,
    /// so a crossing must still be recorded. A rule that suppressed promotion
    /// across calls in every `bund2 script` run would make §S6's promotion
    /// figures numbers for a configuration nobody runs, which is the error D45
    /// was taken to correct.
    #[test]
    fn the_same_body_still_crosses_under_the_cli_reporter() {
        let mut r = crossing_runtime_with(1);
        r.interp.reporter = Box::new(bund2_stdlib::report::TextReporter::new(true));
        r.eval_str(":w { 1 2 + nl :x :x alias clear } register\n")
            .expect("setup runs");
        for _ in 0..3 {
            r.eval_str("w").expect("the body runs");
        }

        assert!(
            r.crossed_calls().unwrap_or(0) > 0,
            "under a reporter that wants only fatal snapshots, promotion must \
             still cross a call (D45); zero here means §S5's rule was applied \
             to a configuration it does not govern"
        );
    }

    /// **Criterion 30, the twelfth review's first B1 case: a compiled caller of
    /// a cold lambda whose last word is `exit`.**
    ///
    /// The criterion requires this run "with the callee held **below the
    /// compile threshold**, since threshold 1 compiles the callee and hides the
    /// defect". That is the whole point of the row: the caller is compiled and
    /// the callee is not, so the exit is recorded by an *interpreted* lambda
    /// underneath a compiled frame, and the status has to travel back out
    /// through §S5's protocol rather than being produced by compiled code.
    ///
    /// **How the callee is kept cold.** A body containing `exit` cannot be
    /// warmed — the first entry ends the program. So `f` is registered as a
    /// no-op, the caller `w` is entered until it is compiled, and only then is
    /// `f` rebound to the exiting lambda. The rebound body is a fresh `Rc`, so
    /// D35's counter keys on a payload nothing has entered: cold by
    /// construction, not by arithmetic about the threshold.
    ///
    /// **What is compared** is what the criterion names — the exit code and the
    /// final stacks — and not the `Result`. Tier 0 records an exit and returns
    /// `Ok`; a compiled body returns `Err(Error::exited)`, which is §S5's
    /// status protocol. Neither is observable to a program, and comparing them
    /// would fail while nothing was wrong.
    #[test]
    fn a_compiled_caller_of_a_cold_exiting_lambda_matches_tier_zero() {
        // `{ 1 drop }` rather than `{ }`: Bund refuses an empty block, and a
        // balanced no-op is what the warm-up needs.
        let setup = ":f { 1 drop } register\n:w { 1 2 + f clear } register\n";
        let rebind = ":f { 7 exit } register\n";

        let mut tiered = crossing_runtime_with(1);
        tiered.eval_str(setup).expect("setup runs");
        for _ in 0..3 {
            tiered.eval_str("w").expect("the warm-up entries run");
        }
        assert!(
            tiered.compiled_bodies().unwrap_or(0) > 0,
            "the caller must be compiled, or this row asserts the opposite of \
             what it is for"
        );
        tiered.eval_str(rebind).expect("the rebind runs");
        let tier_outcome = tiered.eval_str("w");

        let mut plain = crate::Runtime::new();
        plain.take_tier();
        plain.eval_str(setup).expect("setup runs");
        for _ in 0..3 {
            plain.eval_str("w").expect("the warm-up entries run");
        }
        plain.eval_str(rebind).expect("the rebind runs");
        let plain_outcome = plain.eval_str("w");

        assert_eq!(
            bund2_api::Vm::exit_requested(&tiered.interp),
            Some(7),
            "the compiled caller must record the cold callee's exit"
        );
        assert_eq!(
            bund2_api::Vm::exit_requested(&plain.interp),
            bund2_api::Vm::exit_requested(&tiered.interp),
            "the recorded exit code"
        );

        let a = observe(&mut tiered, tier_outcome, &SharedReporter::default());
        let b = observe(&mut plain, plain_outcome, &SharedReporter::default());
        assert_eq!(a.current, b.current, "current stack");
        assert_eq!(a.stacks, b.stacks, "stacks");
        assert_eq!(a.workbench, b.workbench, "workbench");
    }

    /// **F140's regression: a promoted value must not ride across a stack
    /// switch.**
    ///
    /// `:w { 1 2 + stacks_left }` promotes the sum and then rotates which stack
    /// is current. `stacks_left` is `eff(0, 0)`, so a crossing would sync
    /// **nothing** before it and the body's final sync would push the sum onto
    /// the stack in force *after* the rotation — where Tier 0 pushed it before.
    /// Measured at the shipped threshold, `main` held 33 values without the
    /// tier and 32 with it.
    ///
    /// D73's gate keeps `stacks_left` out of the crossable table, and this is
    /// the differential that says so from the outside. It compares **every**
    /// stack, which is the only way to see it: the counts are right and the
    /// placement is wrong, so a test that looked at the current stack alone
    /// would pass.
    ///
    /// **An even number of compiled entries hides the bug** — each one moves a
    /// value from the stack it belonged on to the next, and with alternating
    /// rotation the counts net out. Three calls is odd on purpose.
    #[test]
    fn a_promoted_value_does_not_ride_across_a_stack_switch() {
        assert_promoted_matches_tier0(
            ":other ensure_stack\n             :main to_stack\n             :w { 1 2 + stacks_left } register\n",
            3,
            "a promoted value across `stacks_left` (F140)",
        );
    }

    /// **Criterion 20: a body run by a loop word reaches the counter under one
    /// key.**
    ///
    /// "Run a lambda through `times` 100 times and assert §S7's counter holds
    /// one entry for it, at 100." The precondition is already proven at Tier 0
    /// by `times_enters_one_body_under_one_key`
    /// (`crates/bund2-stdlib/src/seq.rs`), which shows one key across all 100
    /// entries through the `entry_log` seam; this is the same property asked of
    /// the counter that D35 keys on a payload pointer.
    ///
    /// # Why the threshold is raised above the iteration count
    ///
    /// **The counter stops at the threshold, not at 100.** `Tiering::observe`
    /// answers `Decision::Compiled` from the cache *before* it increments, so a
    /// body that compiles freezes its count — at the default threshold of 64,
    /// a hundred iterations would leave 64 and this test would assert the knob
    /// rather than the key. F136's rule reaches the same result by the other
    /// road: a refused body is demoted, and a demoted body is turned away at the
    /// top of `observe`, before counting.
    ///
    /// A threshold above the iteration count keeps the body away from both
    /// paths, which is what isolates the property this row is about. What the
    /// counter does *at* the threshold is
    /// `the_threshold_decides_when_a_body_has_earned_compilation`'s subject, and
    /// its capacity is `the_counter_cap_holds`'; neither is this.
    /// # How the count is read
    ///
    /// Through `Tier::counted_bodies`, a defaulted trait method beside the four
    /// `--stats` already reports. `Interp` owns the tier and `take_tier` hands
    /// back a `Box<dyn Tier>`, so the alternative was adding `Any` to the trait
    /// and downcasting — new surface for a test, which is the objection
    /// criterion 4 records against reading `Word::_slots`.
    #[test]
    fn a_loop_body_reaches_the_counter_under_one_key() {
        let mut r = runtime_with(1_000_000);
        r.eval_str("100 { drop } times").expect("the loop runs");

        assert_eq!(
            r.counted_bodies(),
            Some(1),
            "one key for the loop body, not one per iteration"
        );

        // **And nothing compiled**, which is what keeps the assertion above
        // about the key rather than about the threshold: a body that compiled
        // would have frozen its count, since `observe` answers `Compiled` from
        // the cache before it increments.
        assert_eq!(r.compiled_bodies(), Some(0), "the threshold is never reached");

        // A tier that is absent answers `None`, as the other figures do — the
        // distinction the CLI's match arms turn on.
        let mut bare = crate::Runtime::new();
        bare.take_tier();
        assert_eq!(bare.counted_bodies(), None, "no tier, not an empty counter");
    }

    /// **§S7's threshold knob, and its precedence** — F125.
    ///
    /// The flag, the environment variable and the default were verified by hand
    /// at a shell when they were built, which is precisely how F124, F127 and
    /// F128 each passed for a while: a knob nothing asserts is a knob that can
    /// stop working silently. `Runtime::jit_threshold` reports what was
    /// *adopted*, not what was asked for, so this can check the resolution
    /// rather than the intent.
    ///
    /// **Serialised through one test** because it sets a process-wide
    /// environment variable; two tests doing that in parallel would race.
    #[test]
    fn the_threshold_takes_the_flag_then_the_environment_then_the_default() {
        let default = Caps::default().threshold;

        // SAFETY: `set_var`/`remove_var` are unsafe since Rust 2024 because
        // they race with other threads reading the environment. This test owns
        // the variable — no other test in this crate touches it — and restores
        // it before returning.
        unsafe { std::env::remove_var("BUND2_JIT_THRESHOLD") };

        let plain = crate::Runtime::new();
        assert_eq!(
            plain.jit_threshold(),
            Some(default),
            "with neither knob set, §S7's default stands"
        );

        let flagged = crate::Runtime::with_options_and_threshold(
            &bund2_stdlib::host::HostOptions::default(),
            Some(1),
        );
        assert_eq!(flagged.jit_threshold(), Some(1), "the flag is adopted");

        unsafe { std::env::set_var("BUND2_JIT_THRESHOLD", "2") };
        let from_env = crate::Runtime::new();
        assert_eq!(
            from_env.jit_threshold(),
            Some(2),
            "the environment is adopted when no flag is given"
        );

        // **The flag wins.** A command line is about this run; an environment
        // variable is about the shell it happened to run in.
        let both = crate::Runtime::with_options_and_threshold(
            &bund2_stdlib::host::HostOptions::default(),
            Some(1),
        );
        assert_eq!(both.jit_threshold(), Some(1), "the flag beats the environment");

        // **A malformed value is ignored, not adopted and not fatal** — the
        // runtime reads this in a constructor that cannot report. What keeps
        // that from being a silent misconfiguration is that the *adopted*
        // threshold is what gets reported, here and by `bund2 --stats`.
        unsafe { std::env::set_var("BUND2_JIT_THRESHOLD", "banana") };
        let malformed = crate::Runtime::new();
        assert_eq!(
            malformed.jit_threshold(),
            Some(default),
            "a value that is not a number falls back to the default"
        );

        unsafe { std::env::remove_var("BUND2_JIT_THRESHOLD") };
    }

    /// The threshold is a **tuning knob, not a semantic boundary** (§S7): the
    /// same program must leave the same stack at any threshold.
    ///
    /// This is criterion 2's third run in miniature. Over the corpus it reads
    /// 106/114 at threshold 1 and at 64 alike; here it is one body, so the
    /// assertion can be exact rather than a count.
    #[test]
    fn the_threshold_changes_speed_and_not_meaning() {
        let mut hot = inlining_runtime_with(1);
        register_body(&mut hot);
        let mut cold = inlining_runtime_with(64);
        register_body(&mut cold);

        for _ in 0..5 {
            hot.eval_str("f").expect("runs");
            cold.eval_str("f").expect("runs");
        }

        assert!(
            hot.compiled_bodies().unwrap_or(0) > 0,
            "threshold 1 must compile, or this compares two interpreters"
        );
        assert_eq!(
            cold.compiled_bodies(),
            Some(0),
            "threshold 64 must not compile in five calls, or the two are the same run"
        );

        let (a, b) = (hot.interp.snapshot(), cold.interp.snapshot());
        assert_eq!(a.len(), b.len(), "depth");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.as_int(), y.as_int(), "value");
            assert_eq!(x.dt(), y.dt(), "dt");
        }
    }

    // --- criterion 21: a current-stack switch mid-body ----------------------

    /// A reporter that keeps every diagnostic where the test can still read it.
    ///
    /// `CollectingReporter` is moved into the `Box` the `Interp` owns, so its
    /// `seen` is unreachable afterwards. This shares the vector instead.
    ///
    /// **`wants_stack` is left at its default, `false`.** That is the CLI's
    /// configuration — `TextReporter` wants a snapshot only for a fatal report
    /// — and §S5 makes it load-bearing: while a reporter wants one for a
    /// severity natives emit mid-body, *nothing stays promoted across a call*.
    /// A test that flipped it on would be measuring a different lowering from
    /// the one `bund2 script` runs.
    #[derive(Clone, Default)]
    struct SharedReporter(std::rc::Rc<std::cell::RefCell<Vec<bund2_api::diag::Diagnostic>>>);

    impl bund2_api::diag::Reporter for SharedReporter {
        fn report(&mut self, d: &bund2_api::diag::Diagnostic) {
            self.0.borrow_mut().push(d.clone());
        }
    }

    /// [`SharedReporter`]'s sibling that **wants a snapshot for every
    /// severity** — D71's dynamic gate, F137.
    ///
    /// Added beside `SharedReporter` rather than by flipping its field, because
    /// that field's documentation is right: a test that turned it on would be
    /// measuring a different lowering from the one `bund2 script` runs, and
    /// every criterion-22 differential wants the CLI's configuration. This one
    /// wants the opposite, and says so in its name.
    #[derive(Clone, Default)]
    struct WantingReporter(std::rc::Rc<std::cell::RefCell<Vec<bund2_api::diag::Diagnostic>>>);

    impl bund2_api::diag::Reporter for WantingReporter {
        fn report(&mut self, d: &bund2_api::diag::Diagnostic) {
            self.0.borrow_mut().push(d.clone());
        }
        fn wants_stack(&self, _severity: bund2_api::diag::Severity) -> bool {
            true
        }
    }

    /// **D71's dynamic gate: a crossing body under a reporter that wants
    /// mid-body snapshots still answers as Tier 0 does** — F137, §S5.
    ///
    /// `:w { 1 2 + nl clear }` is a body that genuinely crosses: `+` publishes
    /// an arm, so it inlines and promotes both literals, and `nl` is `eff(0, 0)`
    /// — certified by the palette, registered by `bund2-stdlib`, and **not**
    /// one of §S6's three published fragments, so it is a generic call D68
    /// permits crossing. The precondition is asserted rather than assumed,
    /// because a fixture that crossed nothing would pass this test while
    /// exercising none of it, which is F127's shape.
    ///
    /// **What this test can and cannot show.** When the gate fires, `enter`
    /// returns `None` and Tier 0 runs the body — and Tier 0's answer is by
    /// construction the same answer, so no counter distinguishes "declined" from
    /// "ran compiled". This asserts the decline path is *safe*: same outcome,
    /// same stacks, same workbench, same diagnostics. That the branch was taken
    /// is not observable through any seam this crate exposes, and the honest
    /// statement is that the two halves are tested separately — the condition by
    /// `wants_stack_answers_for_the_reporter_in_place_now`
    /// (`crates/bund2-interp/src/lib.rs`), and the consequence here.
    ///
    /// Under today's vocabulary this gate never changes an outcome anyway: D71's
    /// static table already keeps every mid-body reporter uncrossed. It is the
    /// second of two locks, and this is what says the second lock does no harm.
    #[test]
    fn a_crossing_body_under_a_stack_wanting_reporter_matches_tier_zero() {
        let setup = ":w { 1 2 + nl clear } register\n";

        let seen = WantingReporter::default();
        let mut tiered = crossing_runtime_with(1);
        tiered.interp.reporter = Box::new(seen.clone());
        tiered.eval_str(setup).expect("setup runs");

        let mut outcome = Ok(());
        for _ in 0..3 {
            if outcome.is_ok() {
                outcome = tiered.eval_str("w");
            }
        }

        assert!(
            tiered.compiled_bodies().unwrap_or(0) > 0,
            "nothing compiled, so this asserts a path the tier never took"
        );
        assert!(
            tiered.crossed_calls().unwrap_or(0) > 0,
            "the body crossed no call, so the gate this test is about was never \
             reachable — `nl` must be a crossable generic call for it to be"
        );

        let plain_seen = SharedReporter::default();
        let mut plain = crate::Runtime::new();
        plain.take_tier();
        plain.interp.reporter = Box::new(plain_seen.clone());
        plain.eval_str(setup).expect("setup runs");
        let mut plain_outcome = Ok(());
        for _ in 0..3 {
            if plain_outcome.is_ok() {
                plain_outcome = plain.eval_str("w");
            }
        }

        let a = observe(&mut tiered, outcome, &SharedReporter(seen.0.clone()));
        let b = observe(&mut plain, plain_outcome, &plain_seen);
        assert_eq!(a.outcome, b.outcome, "outcome");
        assert_eq!(a.current, b.current, "current stack");
        assert_eq!(a.stacks, b.stacks, "stacks");
        assert_eq!(a.workbench, b.workbench, "workbench");
        assert_eq!(a.diagnostics, b.diagnostics, "diagnostics");
    }

    /// Everything observable about a runtime after a program has ended.
    ///
    /// **Every stack, not just the current one.** Criterion 21 is about a body
    /// that switches stacks, so a comparison that read only the current stack
    /// would miss values left on the one the program switched away from — which
    /// is exactly the failure it exists to catch.
    ///
    /// The walk switches to each stack in turn and snapshots it. That mutates
    /// the current-stack epoch, which is harmless *here* because the program has
    /// already ended, and is done identically on both sides.
    #[derive(Debug, PartialEq)]
    struct Observed {
        outcome: Result<(), String>,
        current: String,
        stacks: Vec<(String, Vec<String>)>,
        workbench: Vec<String>,
        diagnostics: Vec<(String, String, Option<String>)>,
    }

    /// Render a value with the parts that move between runs blanked.
    ///
    /// `id` is a nanoid and `stamp` is wall-clock, so a raw comparison of two
    /// runs of the *same* binary fails. F14 normalises both in every golden for
    /// this reason, and a CLI-level `diff` of this program's output was briefly
    /// mistaken for a tier defect before the normalisation was applied.
    fn norm(v: &BundValue) -> String {
        format!("dt={} {}", v.dt(), v.summary(80))
    }

    /// Blank the parts of a **message** that move between runs.
    ///
    /// **This is F14, reproduced deliberately.** `Interp::eval`'s wrapper
    /// renders the offending value into its message — the reproduced form of
    /// the reference's `bail!("Attempt to evaluate value {:?} …")` — and that
    /// rendering carries `id` and `stamp`. So two runs of the *same* binary
    /// produce error strings a millisecond apart. F14 is the record, and the
    /// golden capture normalises the same two fields for the same reason; a
    /// differential that compares error text without doing so is comparing the
    /// clock. Found here the way F14 was found: by diffing two runs.
    fn norm_msg(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for part in s.split_inclusive(',') {
            if let Some(i) = part.find("stamp: ") {
                out.push_str(&part[..i + 7]);
                out.push_str("<stamp>");
                if part.ends_with(',') {
                    out.push(',');
                }
            } else if let Some(i) = part.find("id: \"") {
                out.push_str(&part[..i + 5]);
                out.push_str("<id>\"");
                if part.ends_with(',') {
                    out.push(',');
                }
            } else {
                out.push_str(part);
            }
        }
        out
    }

    fn observe(r: &mut crate::Runtime, outcome: Result<(), Error>, seen: &SharedReporter) -> Observed {
        let names: Vec<String> = r.interp.stacks.names().map(str::to_string).collect();
        let current = bund2_api::Vm::current_name(&r.interp);
        let mut stacks = Vec::with_capacity(names.len());
        for n in &names {
            bund2_api::Vm::to_stack(&mut r.interp, n);
            stacks.push((
                n.clone(),
                bund2_api::Vm::snapshot(&r.interp).iter().map(norm).collect(),
            ));
        }
        Observed {
            outcome: outcome.map_err(|e| norm_msg(&e.0)),
            current,
            stacks,
            workbench: bund2_api::Vm::snapshot_workbench(&r.interp)
                .iter()
                .map(norm)
                .collect(),
            diagnostics: seen
                .0
                .borrow()
                .iter()
                .map(|d| {
                    (
                        format!("{:?}", d.severity),
                        d.reason.clone(),
                        d.stack_name.clone(),
                    )
                })
                .collect(),
        }
    }

    /// **Criterion 21's differential.** `body` is registered as a word and
    /// called until the tier has compiled it; the same source runs with no tier
    /// at all; and every observable must match.
    ///
    /// The tier's own counters are asserted first, so a program cannot pass by
    /// never being compiled — F127's lesson, where a fixture that inlined
    /// nothing made four tests assert paths they could not reach.
    fn assert_switch_matches_tier0(body: &str, calls: usize, label: &str) {
        // **`:main to_stack` is not decoration.** `ensure_stack` makes the
        // named stack *current* when it is new (`Interp::ensure_stack` calls
        // `add_as_current`), so without the return every body would already be
        // running on `s` and the switch inside it would be a switch to the
        // stack already current — a no-op. The six programs then pass while
        // testing nothing, which is exactly what the first version did.
        let src = format!(":s ensure_stack :main to_stack\n:w {{ {body} }} register\n");

        let tier_seen = SharedReporter::default();
        let mut tiered = inlining_runtime_with(1);
        tiered.interp.reporter = Box::new(tier_seen.clone());
        tiered.eval_str(&src).expect("setup runs");
        // **The guard that keeps this criterion honest.** If the body starts on
        // the stack it is about to switch to, the switch is a no-op and all six
        // programs pass while exercising nothing. That is what the first
        // version did, so the precondition is asserted rather than assumed.
        assert_eq!(
            bund2_api::Vm::current_name(&tiered.interp),
            "main",
            "{label}: the body must start on `main`, or its switch is a no-op"
        );
        let mut tier_outcome = Ok(());
        for _ in 0..calls {
            if tier_outcome.is_ok() {
                tier_outcome = tiered.eval_str("w");
            }
        }

        assert!(
            tiered.compiled_bodies().unwrap_or(0) > 0,
            "{label}: nothing compiled, so this asserts a path the tier never took"
        );

        let plain_seen = SharedReporter::default();
        let mut plain = crate::Runtime::new();
        plain.take_tier();
        plain.interp.reporter = Box::new(plain_seen.clone());
        plain.eval_str(&src).expect("setup runs");
        assert_eq!(
            bund2_api::Vm::current_name(&plain.interp),
            "main",
            "{label}: the untiered body must start on `main` too"
        );
        let mut plain_outcome = Ok(());
        for _ in 0..calls {
            if plain_outcome.is_ok() {
                plain_outcome = plain.eval_str("w");
            }
        }

        let a = observe(&mut tiered, tier_outcome, &tier_seen);
        let b = observe(&mut plain, plain_outcome, &plain_seen);
        assert_eq!(a.outcome, b.outcome, "{label}: outcome");
        assert_eq!(a.current, b.current, "{label}: current stack");
        assert_eq!(a.stacks, b.stacks, "{label}: stacks");
        assert_eq!(a.workbench, b.workbench, "{label}: workbench");
        assert_eq!(a.diagnostics, b.diagnostics, "{label}: diagnostics");
    }

    /// **Criterion 21, the six programs.** A body that switches the current
    /// stack mid-run must give Tier 0's result — "it fails on any lowering that
    /// resolves the current stack once, or that syncs to the stack current at
    /// the sync rather than the one each value came from".
    ///
    /// Each program pushes two literals on `main`, switches, and then does
    /// something that can only be right if the switch was seen. The Tier 0
    /// answers were taken from the oracle-equivalent run before these were
    /// written, so the assertions are against observed behaviour and not
    /// against what the words are assumed to do:
    ///
    /// - `to_stack` and `to_current` leave `3` on `s`;
    /// - `stacks_left` rotates to an empty stack and `+` fails there;
    /// - `@s` — a CONTEXT literal, §S5's *static barrier* — leaves `3` on `s`;
    /// - `endcontext` drops `s` and leaves `9` on the **workbench**;
    /// - `ifthenelse` runs the lambda **on top** (F97), so the switch in it
    ///   happens and `1 2 9` end up on `s`.
    ///
    /// **Two rows gained a trailing `drop` on 2026-09-16, and it is F136's
    /// doing.** Every other body here ends in `+`, which §S6 publishes, so it
    /// plans an inlined site. The `endcontext` and `ifthenelse` bodies are built
    /// only from unpublished words, so once the tier began refusing zero-site
    /// bodies they stopped being compiled — and the precondition above caught
    /// that rather than letting the rows pass while exercising nothing, which is
    /// what it exists for. `drop` restores the site and runs in **both** arms,
    /// so the differential still compares like with like: with it the bodies
    /// plan 1 site / 3 promoted and 1 site / 2 promoted respectively, where
    /// before they planned nothing at all.
    ///
    /// The two bullets above describe the bodies **as first written**. What the
    /// amended ones leave is not restated here, because the `script` path prints
    /// no stack dump to a pipe and it has not been observed directly — and this
    /// register has had to withdraw enough claims made from memory. The test
    /// does not depend on it: it asserts that the tiered and untiered arms agree
    /// on outcome, current stack, every stack, the workbench and the
    /// diagnostics, whatever those turn out to be.
    #[test]
    fn a_current_stack_switch_mid_body_gives_tier_zeros_result() {
        for (body, label) in [
            ("1 2 :s to_stack +", "to_stack"),
            ("1 2 :s to_current +", "to_current"),
            ("1 2 stacks_left +", "stacks_left"),
            // **The trailing `drop` is F136's doing, not decoration.** Every
            // other body here ends in `+`, which §S6 publishes, so it plans a
            // site and still compiles. This one's words are all unpublished, so
            // under the rule that refuses zero-site bodies it would no longer be
            // compiled at all — and the guard above caught exactly that rather
            // than letting the row pass while testing nothing. `drop` is
            // published, so it restores the site; it runs in **both** arms, so
            // the differential is unchanged; and it drops the `9` from the
            // workbench after `endcontext` has already moved it there, which is
            // the switch this row exists to observe.
            ("1 2 @s 9 endcontext drop", "a CONTEXT literal and endcontext"),
            ("1 2 @s +", "a CONTEXT literal"),
            // The same F136 fix as the row above, for the same reason: `true`
            // and `ifthenelse` are unpublished, so the outer body planned zero
            // sites and stopped being compiled. The lambdas are separate bodies
            // and are unaffected. `drop` runs in both arms, so the differential
            // still compares like with like.
            (
                "1 2 true { 7 } { @s 9 } ifthenelse drop",
                "a conditional on another stack",
            ),
        ] {
            assert_switch_matches_tier0(body, 3, label);
        }
    }

    /// **A tier moves conformance by exactly zero** (§S2's one invariant), at
    /// the smallest scale it can be checked: the same program, with a tier and
    /// without, leaves the same stack.
    #[test]
    fn a_tiered_run_matches_an_untiered_one() {
        let mut tiered = runtime_with(1);
        register_body(&mut tiered);

        let mut plain = crate::Runtime::new();
        plain.take_tier();
        register_body(&mut plain);

        for _ in 0..5 {
            tiered.eval_str("f").expect("runs");
            plain.eval_str("f").expect("runs");
        }

        let (a, b) = (tiered.interp.snapshot(), plain.interp.snapshot());
        assert_eq!(a.len(), b.len(), "depth");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.as_int(), y.as_int(), "value");
            assert_eq!(x.dt(), y.dt(), "dt");
        }
    }

    /// **The recursion guard, which is a correctness property and not an
    /// optimisation.** A self-recursive word must not spend a Rust frame per
    /// Bund level: the seam takes the tier out while it runs, so a body entered
    /// during compiled execution is interpreted. Without that, this program
    /// would grow the machine stack per level and break RFC-0003's criterion 2
    /// — §S8 calls exactly that "a conformance change, not an optimisation".
    ///
    /// 2,000 levels is far past what a Rust-frame-per-level implementation
    /// survives, and well inside what the heap frame loop handles.
    #[test]
    fn a_self_recursive_word_does_not_spend_a_rust_frame_per_level() {
        let mut r = runtime_with(1);
        // **Every operand order here is the language's, not the obvious one.**
        // `dup 0 <` asks "0 < n", because a comparison takes the top as its
        // *first* operand — so this is "while n > 0". `1 swap -` decrements for
        // the same reason: `a b -` is `b - a`, so a bare `1 -` would compute
        // `1 - n` and the loop would end after one step with a negative. Each
        // of those was wrong in an earlier draft of this test, and each would
        // have left a test that passed while never recursing.
        r.eval_str(":countdown { dup 0 < { 1 swap - countdown } if } register")
            .expect("defines");
        r.eval_str("2000 countdown").expect("recurses on the heap");
        let stack = r.interp.snapshot();
        assert_eq!(
            stack.last().and_then(BundValue::as_int),
            Some(0),
            "counted down to zero: {stack:?}"
        );
    }

    /// **Criterion 23, first half: two `Interp`s share no compiled code.**
    ///
    /// One module per `Interp` means one compiler per tier, so the *same body*
    /// — the same payload, hence the same cache key — driven past one tier's
    /// threshold leaves the other tier with nothing: no compiled word, and a
    /// cache that misses. A shared module would make the second tier's cache a
    /// place another interpreter's code could be found.
    #[test]
    fn two_interps_share_no_compiled_code() {
        let hot = Caps {
            threshold: 1,
            ..Caps::default()
        };
        // **The inlining fixture, not `JitTier::new`** — F136. `new` is
        // documented as "a tier that inlines nothing": its table is empty, so
        // `{ 1 2 + }` plans zero sites and the rule now refuses it. This test is
        // about code *isolation between tiers*, not about inlining, so it takes
        // a populated table and asserts what it always asserted. F127's lesson
        // applied once more: a fixture that cannot inline cannot stand in for
        // one that does.
        let table = bund2_stdlib::fragments::published(&crate::Runtime::new().interp.registry)
            .unwrap_or_default();
        let mut first = JitTier::with_fragments(hot, table.clone());
        let second = JitTier::with_fragments(hot, table);

        // A vocabulary to run against, with its own tier removed so the only
        // tier in play is the one under test.
        let mut host = crate::Runtime::new();
        host.take_tier();

        let body = BundValue::lambda(vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("+"),
        ]);
        for _ in 0..3 {
            let _ = first.enter(&body, &mut host.interp);
        }

        assert!(first.compiled_words() >= 1, "the first tier compiled it");
        assert_eq!(second.compiled_words(), 0, "the second compiled nothing");
        assert!(
            second.tiering().compiled(&body).is_none(),
            "and its cache misses the very body the first tier compiled"
        );
    }

    /// **Criterion 23, second half: dropping an `Interp` drops its code, and
    /// another `Interp` running the same body is unaffected.**
    ///
    /// The body value is shared, so both interpreters key on the same payload —
    /// which is exactly the case where a shared module would let the survivor
    /// reach the dead interpreter's code. It must instead compile its own, and
    /// whatever it does, it must still leave Tier 0's answer.
    #[test]
    fn a_dropped_interps_code_does_not_serve_another() {
        let body = BundValue::lambda(vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("+"),
        ]);

        let mut first = runtime_with(1);
        first.interp.registry.register_lambda("f", body.clone());
        let mut second = runtime_with(1);
        second.interp.registry.register_lambda("f", body.clone());

        for _ in 0..5 {
            first.eval_str("f").expect("runs");
        }
        drop(first);

        for _ in 0..5 {
            second.eval_str("f").expect("the survivor still runs");
        }
        let stack = second.interp.snapshot();
        assert_eq!(stack.len(), 5, "one sum per call: {stack:?}");
        for v in &stack {
            assert_eq!(v.as_int(), Some(3), "Tier 0's answer: {stack:?}");
        }
    }

    /// A body with no values is never compiled — `compile_word` refuses an
    /// empty body, and the tier must not treat that refusal as a reason to stop
    /// interpreting.
    #[test]
    fn an_empty_body_still_runs() {
        let mut r = runtime_with(1);
        r.interp
            .registry
            .register_lambda("nothing", BundValue::lambda(Vec::new()));
        for _ in 0..3 {
            r.eval_str("nothing").expect("an empty body is still a body");
        }
        assert_eq!(r.interp.depth(), 0);
    }
}
