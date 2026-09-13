//! **The tiering policy** — RFC-0005 §S7, wired to the seam.
//!
//! This is the only place that knows both when a body has earned compilation
//! (`bund2-jit`'s `Tiering`) and where a body starts running (`bund2-interp`'s
//! `Tier` seam). Neither crate names the other.

use bund2_api::{Error, Tier, Vm};
use bund2_jit::cache::{Caps, Decision, Tiering};
use bund2_jit::lower::{LastCall, compile_word_body};
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
}

impl Default for JitTier {
    fn default() -> Self {
        Self::new(Caps::default())
    }
}

impl JitTier {
    pub fn new(caps: Caps) -> Self {
        Self {
            tiering: Tiering::new(caps),
        }
    }

    /// The cache and counter, for a test or an embedder that wants the figures.
    pub fn tiering(&self) -> &Tiering {
        &self.tiering
    }
}

impl Tier for JitTier {
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
                    // A failure to compile is not the program's fault and not
                    // its problem: the body is interpreted, as it would have
                    // been with no tier at all. It is dropped rather than
                    // reported because there is no diagnostic a *user* could
                    // act on, and the body still runs correctly.
                    if let Ok(code) = compile_word_body(len, LastCall::Ordinary) {
                        self.tiering.insert(body, code);
                    }
                }
                None
            }
            Decision::Compiled => {
                let values = items(body)?;
                let code = self.tiering.compiled(body)?;
                Some(code.run(vm, values))
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

    /// A body with no values is never compiled — `compile_word_body` refuses an
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
