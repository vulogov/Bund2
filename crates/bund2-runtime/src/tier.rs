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
        }
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
                        None => match Compiler::new(self.table.clone()) {
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
                    if let Ok(code) = compiler.compile_word(&values, LastCall::Ordinary, cells, vm)
                    {
                        self.tiering.insert(body, code);
                    }
                }
                None
            }
            Decision::Compiled => {
                let values = items(body)?;
                // The handle is `Copy` and inert; the code it names lives in
                // this tier's own compiler, which is what makes running it
                // safe. A cache without its compiler simply interprets.
                let code = self.tiering.compiled(body)?;
                let compiler = self.compiler.as_ref()?;
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

        // **`None` and `Some(0)` say different things**, which is the
        // distinction the CLI's match arms turn on: no tier at all, against a
        // tier that compiled nothing.
        let mut bare = crate::Runtime::new();
        bare.take_tier();
        assert_eq!(bare.compiled_bodies(), None, "no tier, not an empty one");
        assert_eq!(bare.inlined_sites(), None);
        assert_eq!(bare.promoted_values(), None);
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
    #[test]
    fn a_current_stack_switch_mid_body_gives_tier_zeros_result() {
        for (body, label) in [
            ("1 2 :s to_stack +", "to_stack"),
            ("1 2 :s to_current +", "to_current"),
            ("1 2 stacks_left +", "stacks_left"),
            ("1 2 @s 9 endcontext", "a CONTEXT literal and endcontext"),
            ("1 2 @s +", "a CONTEXT literal"),
            ("1 2 true { 7 } { @s 9 } ifthenelse", "a conditional on another stack"),
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
        let mut first = JitTier::new(hot);
        let second = JitTier::new(hot);

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
