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
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Weak;

use bund2_api::Symbol;
use bund2_value::{BundValue, Payload};

use crate::lower::WordHandle;

/// 2^64 divided by the golden ratio: the odd multiplier Fibonacci hashing uses.
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// **A hasher for keys that are already addresses** — F131.
///
/// These three maps key on `BundValue::payload_key`, which is a pointer (D35).
/// The standard library's default is SipHash, chosen so that a map keyed on
/// attacker-supplied data cannot be made to collide. These keys come from the
/// allocator, and F131 measured what the default costs here: **~15 ns per
/// entry**, against ~22 ns to interpret a whole word — paid on every body
/// entered, including the great majority that never reach the threshold and can
/// never repay it.
///
/// Multiply by `GOLDEN` and keep the result: the multiply carries entropy
/// upward, and `finish` folds the top half down because a hash map wants bits
/// at both ends — the high bits pick the control byte and the low bits the
/// bucket. A pointer used unmixed would be worse than SipHash rather than
/// better: allocations are aligned, so the low bits are constant and every body
/// would land in a handful of buckets.
///
/// **This changes no semantics.** D35 fixes the *key* — the payload address —
/// and says nothing about how a map hashes it; the entries, the liveness rule
/// and the answers are identical either way.
#[derive(Default, Clone, Copy)]
pub struct AddrHasher(u64);

impl Hasher for AddrHasher {
    fn finish(&self) -> u64 {
        // Fold the high half down: the multiply put the entropy there, and the
        // low bits choose the bucket.
        self.0 ^ (self.0 >> 32)
    }

    fn write(&mut self, bytes: &[u8]) {
        // A `usize` key reaches `write_usize` below and never arrives here, but
        // a `Hasher` answers for whatever it is handed.
        for b in bytes {
            self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(GOLDEN);
        }
    }

    fn write_usize(&mut self, n: usize) {
        // Widening on every target Bund2 builds for; `unwrap_or` rather than a
        // cast keeps D37's no-panic rule structural instead of argued.
        self.0 = u64::try_from(n).unwrap_or(u64::MAX).wrapping_mul(GOLDEN);
    }
}

/// A map from a body's payload address to whatever that map holds.
type AddrMap<T> = HashMap<usize, Entry<T>, BuildHasherDefault<AddrHasher>>;

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
    /// **Compiled code is held for it, and this is the handle** — F130.
    ///
    /// The handle rides on the decision because the caller would otherwise ask
    /// for it again: `observe` found the entry, and `Decision::Compiled` alone
    /// made `JitTier::enter` call [`Tiering::compiled`], which recomputed
    /// `payload_key` and probed the same map a second time. Entering a compiled
    /// body cost three hash probes and two key computations, ~93 ns, against
    /// 22.44 ns for a whole interpreted word — so a hot short body lost more at
    /// the door than the lowering saved inside.
    ///
    /// **The liveness check moved here with it, and that is a correction.**
    /// This arm used to answer on a bare `contains_key` while `compiled` did
    /// the `Weak` test afterwards, so a body whose payload had been dropped
    /// reported `Compiled` and then quietly did not run compiled. Now one
    /// lookup decides both, and a dead entry answers `Interpret` — which is
    /// what D35 as amended by Q32 always meant.
    Compiled(WordHandle),
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
    counter: AddrMap<u32>,
    /// §S3's compiled cache.
    ///
    /// It holds **handles**, not code: the code lives in the `Compiler` the
    /// tier owns, one module per `Interp`. So a cache entry is inert on its
    /// own, and this map neither owns code memory nor can outlive it usefully.
    cache: AddrMap<WordHandle>,
    /// Redefinitions per **slot** — a `Symbol`, not a body. §S7's recompile cap
    /// is per slot while demotion is per body, and this is the half that counts.
    redefinitions: HashMap<Symbol, u32>,
    /// Bodies demoted permanently. Holds a `Weak` for the same reason the other
    /// two maps do: a demoted body's address must not be reused under another
    /// body that would inherit the demotion.
    demoted: AddrMap<()>,
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
            counter: AddrMap::default(),
            cache: AddrMap::default(),
            redefinitions: HashMap::new(),
            demoted: AddrMap::default(),
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
        let Some(key) = body.payload_key() else {
            return Decision::Interpret;
        };
        // **The `Weak` is not taken here** — F131. It used to be, beside the
        // key, on every entry: `Rc::downgrade` writes the weak count and the
        // `Weak` is dropped a few lines later, which writes it again, on every
        // path except the one that files a new counter entry. It is taken at
        // that insertion instead. `payload_weak` answers `Some` for exactly the
        // values `payload_key` does — both are the `Heap` arm — so nothing that
        // used to be counted is now missed.
        //
        // **Demotion is rare and the map is usually empty.** Testing that first
        // costs a length compare and saves hashing the key.
        if !self.demoted.is_empty() && self.demoted.contains_key(&key) {
            return Decision::Interpret;
        }
        // **One lookup, and it carries the handle out** — F130. This was
        // `contains_key`, which forced the caller to ask again through
        // `compiled`, recomputing the key and probing this map a second time.
        // The liveness test lives here now rather than in that second call: a
        // dead entry is not compiled code, and saying so here is what keeps a
        // dropped body from reporting `Compiled` and then not running as one.
        // A dead entry is not answered for: its address may be reused by a
        // different body, so it falls through to the counter below exactly as
        // an unseen body would.
        // The empty test is F131's again: until something has been compiled
        // there is nothing here to find, and that is the state every body below
        // the threshold is looked up in.
        if !self.cache.is_empty()
            && let Some(entry) = self.cache.get(&key)
            && !entry.is_dead()
        {
            return Decision::Compiled(entry.value);
        }

        // The counter is capped, and at the cap the coldest go first — which is
        // the opposite of the function cap's behaviour, deliberately: this map
        // holds counts, not code, so evicting one costs a recount rather than
        // orphaning a compiled function.
        //
        // **The cap is tested before the membership probe** — F131. Written the
        // other way round, `!contains_key(&key) && len() >= cap`, Rust evaluates
        // the probe first, so every entry paid a whole lookup to guard a
        // condition that is false until the counter fills. The length compare
        // is free and answers the same question.
        if self.counter.len() >= self.caps.counter && !self.counter.contains_key(&key) {
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
                // First sight of this body, and the only place the `Weak` is
                // needed: it is what keeps the address out of reuse, so an
                // entry can never answer for a different body later.
                let Some(weak) = body.payload_weak() else {
                    return Decision::Interpret;
                };
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

    /// **Demote `body` permanently, on the caller's judgement** — F133.
    ///
    /// [`Tiering::redefined`] demotes on §S7's recompile cap. This is the same
    /// permanence reached by a different route: the tier asks for it when
    /// compiling a body could not pay — no inlinable site and nothing to
    /// promote — so the body stays at Tier 0 for good.
    ///
    /// **The record is the point, not the refusal.** Refusing without it would
    /// leave the counter past the threshold, so every later entry would re-plan
    /// the body and refuse again, paying `plan_body` forever to learn what was
    /// already known. A demoted body is answered at the top of
    /// [`Tiering::observe`] and never reaches planning again.
    ///
    /// Like every other entry here it holds a `Weak`, so a demoted body's
    /// address cannot be reused under a different body that would inherit the
    /// demotion.
    pub fn demote(&mut self, body: &BundValue) -> bool {
        let (Some(key), Some(weak)) = (body.payload_key(), body.payload_weak()) else {
            return false;
        };
        self.cache.remove(&key);
        self.counter.remove(&key);
        self.demoted.insert(key, Entry { weak, value: () });
        true
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

    /// **How many times this body has been counted** — RFC-0005 criterion 20.
    ///
    /// [`Tiering::counted`] answers how many bodies the map holds; this answers
    /// what one of them holds. The criterion needs both: "run a lambda through
    /// `times` 100 times and assert §S7's counter holds **one entry** for it,
    /// **at 100**" — one entry is the first figure, at 100 is this one, and the
    /// property is that a loop body reaches the counter under a single key
    /// rather than a fresh one per iteration.
    ///
    /// **A dead entry answers `None`**, like every other read here: its address
    /// may already belong to a different body, and a count served across that
    /// boundary would be the false hit §S3 forbids.
    pub fn count_of(&self, body: &BundValue) -> Option<u32> {
        let key = body.payload_key()?;
        let entry = self.counter.get(&key)?;
        if entry.is_dead() {
            return None;
        }
        Some(entry.value)
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
        // **An `Interp` purely for its cells' address** — §S6 has a lowering
        // embed it, so `compile_word` requires one even here, where the handle
        // is filed in the cache and never run. Building one per call is the
        // honest way to get a real address: a fabricated one would compile and
        // then read arbitrary memory if anything ever did run it.
        let mut vm = bund2_interp::Interp::new();
        let cells = vm.cells().base();
        // A one-literal body: a literal is not a `CALL`, so it plans as a
        // generic site and this compiler's empty table would refuse it anyway.
        // What the cache stores is the handle, which is all these tests touch.
        c.compile_word(
            &[bund2_value::BundValue::int(1)],
            LastCall::Ordinary,
            cells,
            &mut vm,
        )
        .expect("lowers")
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

    /// **F131's lazy downgrade must not cost the counter its `Weak`.**
    ///
    /// `observe` used to take `payload_weak` on every entry and drop it again
    /// unless the body was new; it now takes one only where an entry is filed.
    /// The entry must still hold it, because that `Weak` is what keeps the
    /// address out of reuse — without it the counter would key on an address
    /// the allocator could hand to another body, which is the false hit §S3
    /// calls the worst failure available here.
    #[test]
    fn a_counted_body_is_still_swept_when_its_body_dies() {
        let mut t = Tiering::new(small());
        {
            let b = body(7);
            // Below `small()`'s threshold of 2, so this counts and nothing else.
            assert_eq!(t.observe(&b), Decision::Interpret, "counted, not compiled");
            assert_eq!(t.counted(), 1, "and the counter holds it");
        }
        // The body is gone, so the entry is dead and the sweep must drop it.
        t.sweep();
        assert_eq!(t.counted(), 0, "a dead counter entry is swept");
    }

    /// **F131's hasher must spread aligned addresses.**
    ///
    /// A payload address is aligned, so neighbouring allocations share their low
    /// bits — and the low bits are what choose a bucket. A hasher that returned
    /// the pointer unmixed would be faster than SipHash and far worse, piling
    /// every body into a handful of buckets. This asserts the mixing, not the
    /// speed: the speed is F131's benchmark.
    #[test]
    fn the_address_hasher_spreads_aligned_neighbours() {
        use std::hash::BuildHasher;
        let bh = BuildHasherDefault::<AddrHasher>::default();
        let a = bh.hash_one(0x1000_usize);
        let b = bh.hash_one(0x1010_usize);
        let c = bh.hash_one(0x1020_usize);
        assert_ne!(a, b, "neighbouring allocations must not collide");
        assert_ne!(b, c, "nor the next pair");
        assert_ne!(
            a & 0x3f,
            b & 0x3f,
            "and they must not share a bucket either"
        );
    }

    /// **Criterion 3.** "Drop every strong reference to a compiled body and
    /// assert its contents are dropped, that its entry no longer upgrades and
    /// answers for nothing, and that after the sweep the entry is gone."
    #[test]
    fn a_cache_entry_cannot_answer_for_a_different_body() {
        let mut t = Tiering::new(small());
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
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
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
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
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
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

    /// **Criterion 20's second half, at the level the counter lives.** The
    /// runtime's `a_loop_body_reaches_the_counter_under_one_key` asserts the
    /// *one entry* half through the seam; this asserts the *at N* half against
    /// `Tiering` directly, where the count for a single body is readable.
    ///
    /// The two together are the criterion: one key, counted once per entry. A
    /// threshold above the entry count keeps the body out of the cache, since
    /// `observe` answers `Compiled` before it increments and a compiled body's
    /// count would freeze.
    #[test]
    fn one_body_entered_n_times_is_counted_n_times_under_one_key() {
        let mut t = Tiering::new(Caps {
            threshold: 1_000_000,
            ..small()
        });
        let b = body(1);
        for _ in 0..100 {
            assert_eq!(t.observe(&b), Decision::Interpret, "never reaches the cache");
        }
        assert_eq!(t.counted(), 1, "one key, not one per entry");
        assert_eq!(t.count_of(&b), Some(100), "counted once per entry");

        // A body the counter has never seen has no count — distinct from a
        // body counted zero times, which cannot exist.
        assert_eq!(t.count_of(&body(2)), None, "unseen bodies answer None");
    }

    /// **Below the threshold a body is interpreted; at it, compiled.** §S7's
    /// threshold is the one knob no correctness argument rests on, so this
    /// pins the boundary rather than the number.
    #[test]
    fn the_threshold_decides_when_a_body_has_earned_compilation() {
        let mut t = Tiering::new(small());
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let b = body(1);
        assert_eq!(t.observe(&b), Decision::Interpret, "first evaluation");
        assert_eq!(t.observe(&b), Decision::Compile, "at the threshold of 2");
        let filed = code(&mut c);
        assert!(t.insert(&b, filed));
        // **The handle comes back on the decision** — F130. Asserting the
        // handle and not merely the variant is what would catch a hit path
        // that answered `Compiled` for the wrong entry.
        assert_eq!(
            t.observe(&b),
            Decision::Compiled(filed),
            "and after it is filed, with the handle it was filed under"
        );
    }

    /// **D35's argument for pointer keying, as a property.** A `dup`'d lambda
    /// shares its original's payload, so the two share a key and one cache
    /// entry — where an identity-keyed cache "cannot hit for any dup'd lambda,
    /// in principle, forever".
    #[test]
    fn a_dup_shares_the_originals_cache_entry() {
        let mut t = Tiering::new(small());
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
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
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let scalar = BundValue::int(7);
        assert_eq!(t.observe(&scalar), Decision::Interpret);
        assert_eq!(t.counted(), 0);
        assert!(!t.insert(&scalar, code(&mut c)));
        assert!(!t.is_demoted(&scalar));
    }
}
