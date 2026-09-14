//! **The compiled cache and the promotion counter** — RFC-0005 §S3 and §S7.
//!
//! Two maps over the same bodies, keyed on the body's payload address
//! (`BundValue::payload_key`, D35) and each holding a
//! `Weak<Payload>` (`BundValue::payload_weak`, D35 as amended by Q32, and
//! RFC-0001's 2026-09-13 amendment).
//!
//! # Why a `Weak`, and why it is the whole design
//!
//! A freed body's address may be reused. An entry that outlived its body would
//! then be a **false hit** — wrong code executed, which §S3 calls the worst
//! failure available here. A `Weak` prevents it outright: an `Rc`'s allocation
//! is freed only when its strong *and* weak counts reach zero, so a live entry
//! keeps the address out of reuse. And it pins nothing, because D42's frames
//! supply the liveness: a body is alive whenever anything is executing it.
//!
//! So an entry whose `Weak` no longer upgrades is **dead**: it answers for
//! nothing and is dropped on the next sweep.
//!
//! # What this is not
//!
//! There is no `Interp` integration. Nothing here is consulted when a word
//! runs; `Tiering` is driven by its caller, and today that caller is its tests.
//! **Criterion 23 is not met by this**: it requires one cache, one `JITModule`
//! and one set of cells *per `Interp`*, and `lower`'s entry points each build a
//! module of their own. That is recorded in §S6 rather than implied away here.

use std::collections::HashMap;
use std::rc::Weak;

use bund2_api::Symbol;
use bund2_value::{BundValue, Payload};

use crate::lower::WordHandle;

/// §S7's knobs, with its defaults.
///
/// **Configurable, and the configuration is recorded rather than silent** —
/// §S7 says so, and criterion 6 depends on it: it sets the function cap to 4,
/// the recompile cap to 2 and the counter cap to 8 rather than exercising a
/// thousand bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    /// Evaluations of one body before it has earned compilation. §S7's tuning
    /// knob: no correctness argument rests on it.
    pub threshold: u32,
    /// How many bodies may hold compiled code. **At the cap, compilation
    /// stops** — code memory is never reclaimed (§S4), so this is the only
    /// bound on it, and a body past it stays interpreted.
    pub functions: usize,
    /// Redefinitions of one **slot** before the body live at the time is
    /// demoted. Without it a word redefined in a REPL loop orphans a function
    /// per redefinition.
    pub recompiles: u32,
    /// How many bodies the counter may track. **At the cap the coldest go
    /// first** — unlike the function cap, this one evicts.
    pub counter: usize,
}

impl Default for Caps {
    fn default() -> Self {
        Self {
            threshold: 64,
            functions: 1024,
            recompiles: 4,
            counter: 4096,
        }
    }
}

/// What the caller should do with a body it is about to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Run it at Tier 0. Below the threshold, past the function cap, or
    /// permanently demoted.
    Interpret,
    /// It has crossed the threshold and has no code yet.
    Compile,
    /// Compiled code is held for it.
    Compiled,
}

/// One body's entry in either map: the `Weak` that decides whether it is still
/// alive, beside whatever that map holds.
struct Entry<T> {
    weak: Weak<Payload>,
    value: T,
}

impl<T> Entry<T> {
    /// Dead when the body's last strong reference has gone.
    fn is_dead(&self) -> bool {
        self.weak.strong_count() == 0
    }
}

/// The compiled cache and the promotion counter, together.
///
/// They are one type because they are two maps over the same bodies with the
/// same lifetime rule, and keeping them apart invites exactly the drift §S7
/// warns about — a counter that pins what the cache does not.
pub struct Tiering {
    caps: Caps,
    /// §S7's promotion counter: how many times each body has been evaluated.
    counter: HashMap<usize, Entry<u32>>,
    /// §S3's compiled cache.
    ///
    /// It holds **handles**, not code: the code lives in the `Compiler` the
    /// tier owns, one module per `Interp`. So a cache entry is inert on its
    /// own, and this map neither owns code memory nor can outlive it usefully.
    cache: HashMap<usize, Entry<WordHandle>>,
    /// Redefinitions per **slot** — a `Symbol`, not a body. §S7's recompile cap
    /// is per slot while demotion is per body, and this is the half that counts.
    redefinitions: HashMap<Symbol, u32>,
    /// Bodies demoted permanently. Holds a `Weak` for the same reason the other
    /// two maps do: a demoted body's address must not be reused under another
    /// body that would inherit the demotion.
    demoted: HashMap<usize, Entry<()>>,
}

impl Default for Tiering {
    fn default() -> Self {
        Self::new(Caps::default())
    }
}

impl Tiering {
    pub fn new(caps: Caps) -> Self {
        Self {
            caps,
            counter: HashMap::new(),
            cache: HashMap::new(),
            redefinitions: HashMap::new(),
            demoted: HashMap::new(),
        }
    }

    pub fn caps(&self) -> Caps {
        self.caps
    }

    /// **Count one evaluation of `body`, and say what to do with it.**
    ///
    /// A value with no payload — an unboxed scalar — is not a body and is never
    /// counted; `payload_key` answers `None` for it, as does `payload_weak`.
    /// `Vm::scoped_call`'s LIST is a body without a stable key for a different
    /// reason (§S3), and never reaches here because its caller has no key to
    /// offer.
    pub fn observe(&mut self, body: &BundValue) -> Decision {
        let (Some(key), Some(weak)) = (body.payload_key(), body.payload_weak()) else {
            return Decision::Interpret;
        };
        if self.demoted.contains_key(&key) {
            return Decision::Interpret;
        }
        if self.cache.contains_key(&key) {
            return Decision::Compiled;
        }

        // The counter is capped, and at the cap the coldest go first — which is
        // the opposite of the function cap's behaviour, deliberately: this map
        // holds counts, not code, so evicting one costs a recount rather than
        // orphaning a compiled function.
        if !self.counter.contains_key(&key) && self.counter.len() >= self.caps.counter {
            self.sweep();
            if self.counter.len() >= self.caps.counter {
                self.evict_coldest();
            }
        }

        let seen = match self.counter.get_mut(&key) {
            Some(e) => {
                // **Saturating**, as `Slot::bump` is: a body evaluated
                // 4 billion times should stay compiled, not wrap to cold.
                e.value = e.value.saturating_add(1);
                e.value
            }
            None => {
                self.counter.insert(key, Entry { weak, value: 1 });
                1
            }
        };

        if seen < self.caps.threshold {
            Decision::Interpret
        } else if self.cache.len() >= self.caps.functions {
            // **The function cap refuses rather than evicting.** Code memory is
            // never reclaimed (§S4), so evicting an entry would orphan its
            // function and buy nothing; past the cap a body stays interpreted.
            Decision::Interpret
        } else {
            Decision::Compile
        }
    }

    /// File compiled code for `body`.
    ///
    /// Refused past the function cap, so a caller that ignored [`Decision`]
    /// cannot grow code memory past the bound. Refused for a demoted body, so
    /// demotion is permanent in fact and not only in intent.
    pub fn insert(&mut self, body: &BundValue, code: WordHandle) -> bool {
        let (Some(key), Some(weak)) = (body.payload_key(), body.payload_weak()) else {
            return false;
        };
        if self.demoted.contains_key(&key) || self.cache.len() >= self.caps.functions {
            return false;
        }
        self.cache.insert(key, Entry { weak, value: code });
        true
    }

    /// The compiled code for `body`, if the cache holds it **and it is still
    /// the same body**.
    ///
    /// The `Weak` is what makes the second half true: a dead entry answers for
    /// nothing, so a reused address cannot be served another body's code.
    pub fn compiled(&self, body: &BundValue) -> Option<WordHandle> {
        let key = body.payload_key()?;
        let entry = self.cache.get(&key)?;
        if entry.is_dead() {
            return None;
        }
        Some(entry.value)
    }

    /// **A slot was redefined — §S7's recompile cap.**
    ///
    /// Counts per **slot**, and demotes the body **live at the time**, which is
    /// the owner's decision of 2026-09-13. The two keys differ on purpose: a
    /// redefinition replaces one body with another, so the slot is what
    /// persists across it and the body is what the demotion can name.
    ///
    /// `live` is the body the slot held when it was redefined — the one whose
    /// compiled code the redefinition orphans. `None` when the slot held no
    /// body with a key.
    pub fn redefined(&mut self, slot: Symbol, live: Option<&BundValue>) {
        let count = self.redefinitions.entry(slot).or_insert(0);
        *count = count.saturating_add(1);
        if *count <= self.caps.recompiles {
            return;
        }
        let Some(body) = live else {
            return;
        };
        let (Some(key), Some(weak)) = (body.payload_key(), body.payload_weak()) else {
            return;
        };
        // Permanent, per body: it returns to Tier 0 and is never promoted
        // again. Dropping its cache entry is what returns it; the `demoted`
        // entry is what keeps it there.
        self.cache.remove(&key);
        self.counter.remove(&key);
        self.demoted.insert(key, Entry { weak, value: () });
    }

    /// Whether `body` has been demoted permanently.
    pub fn is_demoted(&self, body: &BundValue) -> bool {
        body.payload_key()
            .is_some_and(|k| self.demoted.contains_key(&k))
    }

    /// **Drop every entry whose body is gone.**
    ///
    /// §S7: "An entry whose `Weak` no longer upgrades is dead and is evicted on
    /// the next sweep." **D39 applies**: this runs over each map as it stands,
    /// never until a condition holds, so it is bounded on data already taken.
    ///
    /// **What calls it, since §S7 does not say.** The RFC describes the sweep
    /// and names no trigger, and criteria 3 and 6 both assert state "after the
    /// sweep", so it has to be callable. It is public for that, and
    /// [`Tiering::observe`] also calls it when the counter reaches its cap —
    /// before evicting a live entry, because dropping a dead one is free and
    /// evicting a live one costs a recount. Chosen here rather than specified;
    /// it changes no observable behaviour, only when memory is given back.
    pub fn sweep(&mut self) {
        self.counter.retain(|_, e| !e.is_dead());
        self.cache.retain(|_, e| !e.is_dead());
        self.demoted.retain(|_, e| !e.is_dead());
    }

    /// How many bodies the counter tracks.
    pub fn counted(&self) -> usize {
        self.counter.len()
    }

    /// How many bodies hold compiled code.
    pub fn compiled_count(&self) -> usize {
        self.cache.len()
    }

    /// How many bodies are demoted.
    pub fn demoted_count(&self) -> usize {
        self.demoted.len()
    }

    /// Drop the least-evaluated entry, to keep the counter under its cap.
    ///
    /// Bounded on the map as it stands (D39). Ties go to whichever the
    /// iteration reaches first, which is not a property worth pinning: the
    /// entry is a count, and losing one costs a recount.
    fn evict_coldest(&mut self) {
        let coldest = self
            .counter
            .iter()
            .min_by_key(|(_, e)| e.value)
            .map(|(k, _)| *k);
        if let Some(k) = coldest {
            self.counter.remove(&k);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::{Compiler, LastCall};

    fn body(n: i64) -> BundValue {
        BundValue::lambda(vec![BundValue::int(n)])
    }

    /// A real compiled word, from a real compiler.
    ///
    /// The cache never dereferences a handle, so a fabricated one would pass
    /// these tests — which is exactly why they use the real thing instead: what
    /// the cache files has to be what the tier would file.
    fn code(c: &mut Compiler) -> WordHandle {
        c.compile_word(1, LastCall::Ordinary).expect("lowers")
    }

    /// Small caps, as criterion 6 requires: "with the compiled-function cap set
    /// to 4 … the recompile cap set to 2 … the counter cap set to 8".
    fn small() -> Caps {
        Caps {
            threshold: 2,
            functions: 4,
            recompiles: 2,
            counter: 8,
        }
    }

    /// **Criterion 3.** "Drop every strong reference to a compiled body and
    /// assert its contents are dropped, that its entry no longer upgrades and
    /// answers for nothing, and that after the sweep the entry is gone."
    #[test]
    fn a_cache_entry_cannot_answer_for_a_different_body() {
        let mut t = Tiering::new(small());
        let mut c = Compiler::new().expect("a compiler");
        let key = {
            let b = body(1);
            let key = b.payload_key().expect("a body has a key");
            assert!(t.insert(&b, code(&mut c)), "filed");
            assert!(t.compiled(&b).is_some(), "and answers while it lives");
            key
        };
        // The body is gone. The entry is still in the map, and must answer for
        // nothing: this is the false hit §S3 calls the worst failure here.
        assert_eq!(t.compiled_count(), 1, "the entry outlives the body");
        let ghost = body(1);
        if ghost.payload_key() == Some(key) {
            // The allocator reused the address — which the `Weak` is supposed to
            // prevent. If it ever does, the entry must still refuse.
            assert!(
                t.compiled(&ghost).is_none(),
                "a dead entry must never answer for a body at a reused address"
            );
        }
        t.sweep();
        assert_eq!(t.compiled_count(), 0, "and is gone after the sweep");
    }

    /// **Criterion 6, the function cap.** "With the compiled-function cap set
    /// to 4, compiling five distinct bodies leaves the fifth interpreted."
    ///
    /// The cap **refuses** rather than evicting: code memory is never reclaimed
    /// (§S4), so evicting would orphan a function and buy nothing.
    #[test]
    fn the_function_cap_leaves_the_body_past_it_interpreted() {
        let mut t = Tiering::new(small());
        let mut c = Compiler::new().expect("a compiler");
        let held: Vec<BundValue> = (0..5).map(body).collect();
        for b in held.iter().take(4) {
            assert!(t.insert(b, code(&mut c)), "the first four are compiled");
        }
        assert_eq!(t.compiled_count(), 4);

        let fifth = &held[4];
        assert!(!t.insert(fifth, code(&mut c)), "the fifth is refused");
        assert!(t.compiled(fifth).is_none(), "and stays interpreted");

        // And `observe` agrees, rather than saying `Compile` for a body the
        // cache would refuse.
        for _ in 0..small().threshold {
            let _ = t.observe(fifth);
        }
        assert_eq!(t.observe(fifth), Decision::Interpret, "past the cap");
    }

    /// **Criterion 6, the recompile cap.** "With the recompile cap set to 2, a
    /// third redefinition demotes the body permanently."
    ///
    /// Per the owner's decision of 2026-09-13: the count is per **slot**, and
    /// what is demoted is the body **live at the time**.
    #[test]
    fn the_third_redefinition_demotes_the_body_live_at_the_time() {
        let mut t = Tiering::new(small());
        let mut c = Compiler::new().expect("a compiler");
        let mut r = bund2_api::Registry::new();
        // `Registry` has no by-name symbol method of its own; interning goes
        // through the public `interner`, as `bund2-interp`'s dispatch does.
        let slot = r.interner.intern("w");

        let first = body(1);
        assert!(t.insert(&first, code(&mut c)));

        t.redefined(slot, Some(&first));
        t.redefined(slot, Some(&first));
        assert!(!t.is_demoted(&first), "two is within the cap of 2");
        assert!(t.compiled(&first).is_some(), "and it keeps its code");

        t.redefined(slot, Some(&first));
        assert!(t.is_demoted(&first), "the third exceeds it");
        assert!(t.compiled(&first).is_none(), "the code is dropped");
        assert!(!t.insert(&first, code(&mut c)), "and it is never promoted again");

        // **Per slot, not per body**: a different slot has its own count.
        let other = r.interner.intern("x");
        let second = body(2);
        t.redefined(other, Some(&second));
        assert!(!t.is_demoted(&second), "another slot starts at zero");
    }

    /// **Criterion 6, the counter cap.** "With the counter cap set to 8,
    /// evaluating nine distinct bodies leaves the map at 8."
    #[test]
    fn the_counter_cap_holds() {
        let mut t = Tiering::new(small());
        let held: Vec<BundValue> = (0..9).map(body).collect();
        for b in &held {
            let _ = t.observe(b);
        }
        assert_eq!(t.counted(), 8, "the map is capped");
    }

    /// **Criterion 6's other half: the counter does not pin.** "Evaluate a body
    /// below the threshold, drop every strong reference to it, and assert the
    /// body's contents are dropped while the counter still holds its entry —
    /// then that the entry is gone after a sweep."
    ///
    /// Without this, "the counter holds a `Weak`" is a comment rather than a
    /// property, and it fails the same way criterion 3 does: silently, and only
    /// under memory pressure.
    #[test]
    fn the_counter_does_not_pin_the_bodies_it_counts() {
        let mut t = Tiering::new(small());
        let weak = {
            let b = body(1);
            let _ = t.observe(&b);
            assert_eq!(t.counted(), 1, "counted while it lives");
            b.payload_weak().expect("a weak")
        };
        assert!(
            weak.upgrade().is_none(),
            "the counter must not keep the body's contents alive"
        );
        assert_eq!(t.counted(), 1, "the entry outlives the body");
        t.sweep();
        assert_eq!(t.counted(), 0, "and is gone after the sweep");
    }

    /// **Below the threshold a body is interpreted; at it, compiled.** §S7's
    /// threshold is the one knob no correctness argument rests on, so this
    /// pins the boundary rather than the number.
    #[test]
    fn the_threshold_decides_when_a_body_has_earned_compilation() {
        let mut t = Tiering::new(small());
        let mut c = Compiler::new().expect("a compiler");
        let b = body(1);
        assert_eq!(t.observe(&b), Decision::Interpret, "first evaluation");
        assert_eq!(t.observe(&b), Decision::Compile, "at the threshold of 2");
        assert!(t.insert(&b, code(&mut c)));
        assert_eq!(t.observe(&b), Decision::Compiled, "and after it is filed");
    }

    /// **D35's argument for pointer keying, as a property.** A `dup`'d lambda
    /// shares its original's payload, so the two share a key and one cache
    /// entry — where an identity-keyed cache "cannot hit for any dup'd lambda,
    /// in principle, forever".
    #[test]
    fn a_dup_shares_the_originals_cache_entry() {
        let mut t = Tiering::new(small());
        let mut c = Compiler::new().expect("a compiler");
        let original = body(1);
        assert!(t.insert(&original, code(&mut c)));
        let copy = original.dup();
        assert!(
            t.compiled(&copy).is_some(),
            "a dup'd body must find its original's code"
        );
    }

    /// A value with no payload is not a body: it is never counted, never
    /// cached, and never demoted.
    #[test]
    fn an_unboxed_scalar_is_not_a_body() {
        let mut t = Tiering::new(small());
        let mut c = Compiler::new().expect("a compiler");
        let scalar = BundValue::int(7);
        assert_eq!(t.observe(&scalar), Decision::Interpret);
        assert_eq!(t.counted(), 0);
        assert!(!t.insert(&scalar, code(&mut c)));
        assert!(!t.is_demoted(&scalar));
    }
}
