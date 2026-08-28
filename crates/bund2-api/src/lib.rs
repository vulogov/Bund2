//! Stable ABI for external Rust word packages. See RFC-0002.
//!
//! Four things here are easy to get wrong, and each cost RFC-0002 a review
//! before it was written down:
//!
//! - **A slot is a set of bindings, not one binding.** A name can be a lambda
//!   *and* a native simultaneously, told apart only by a `$` prefix. A single
//!   enum makes that unrepresentable, and review 1 rejected the RFC for it.
//! - **The `$` sigil is honoured at dispatch, not stripped when interning.**
//!   D16 makes a call target a string built at run time, so `"$println" !`
//!   must work on a name no parser ever saw. `$println` and `println` share
//!   one `Symbol`; the sigil rides on the call.
//! - **Interning a miss must not retain.** The reference's tables are pure
//!   reads on lookup; a design that adds a slot per lookup grows without
//!   bound on a computed miss, which D16 makes expressible.
//! - **Registration is replayed, never deduped.** F32 depends on the second
//!   of two identical registrations winning; deduping would silently change
//!   which handler runs.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use bund2_value::BundValue;

/// An interned name.
///
/// `u32` because the reference registers 617 names and the open world (D16)
/// adds more at run time; nothing here needs more than four billion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(u32);

impl Symbol {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A native word.
///
/// One receiver, where the reference has two — `fn(&mut VM)` for the VM tier
/// and `fn(&mut TS)` for the stack tier, which is *why* the reference needs
/// two tables and a fallthrough. Merging them removes a receiver distinction
/// no Bund program can observe, because dispatch reaches both through
/// `i_direct`. RFC-0002 carries that as a stated deviation, along with the
/// stack tier's distinct error wrapping, which disappears with it.
///
/// This type was named in the Design and defined nowhere until RFC-0002's
/// fourth review; it is one of the types `bund2-api` guarantees.
pub type NativeFn = fn(&mut dyn Vm) -> Result<(), Error>;

/// What the interpreter offers a native word. RFC-0003 fills this in; it
/// exists here so `NativeFn` has a receiver and the stable surface can be
/// written against something.
pub trait Vm {
    fn push(&mut self, v: BundValue);
    fn pull(&mut self) -> Option<BundValue>;
    fn depth(&self) -> usize;
}

/// A word's failure. RFC-0003 replaces this with a spanned error value.
#[derive(Debug, Clone, PartialEq)]
pub struct Error(pub String);

/// How a native word may block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordKind {
    Sync,
    Blocking,
    Async,
}

/// A native word's declared arity.
///
/// **The probed arity, not the guard's** — F18's disposition is FIX. Fourteen
/// words guard on a smaller depth than they consume, so the guard passes and
/// the second pull fails with a different message; declaring the guard's
/// number would put a static arity that lies into RFC-0004's inference and
/// RFC-0005's guard ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackEffect {
    pub consumes: u8,
    pub produces: u8,
}

/// A native binding.
#[derive(Clone, Copy)]
pub struct Native {
    pub f: NativeFn,
    pub effect: StackEffect,
    pub kind: WordKind,
}

impl std::fmt::Debug for Native {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Native")
            .field("effect", &self.effect)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// One name's bindings.
///
/// Six `Option`s, not one enum. The namespaces are independent in the
/// reference — `register_lambda` writes `vm.lambdas` and never touches
/// `inline_fun` — so writing a lambda must not disturb the native that `$name`
/// reaches.
#[derive(Debug, Default, Clone)]
pub struct Slot {
    /// Bumped whenever any binding is rewritten, so an inline cache can be
    /// invalidated without a scan.
    generation: u32,
    pub command: Option<Native>,
    pub alias: Option<Symbol>,
    pub lambda: Option<BundValue>,
    pub native: Option<Native>,
    pub class: Option<BundValue>,
    pub method: Option<Native>,
}

impl Slot {
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// **Saturating, deliberately.** `register` is a word, so a program can
    /// rewrite a name in a loop; at `u32::MAX` a wrapping counter would let a
    /// stale inline cache match a generation it should not. A saturated slot
    /// stops caching instead, trading a fast path for correctness on a path
    /// no program is likely to reach.
    fn touch(&mut self) {
        self.generation = self.generation.saturating_add(1);
    }

    fn is_empty(&self) -> bool {
        self.command.is_none()
            && self.alias.is_none()
            && self.lambda.is_none()
            && self.native.is_none()
            && self.class.is_none()
            && self.method.is_none()
    }
}

/// What a name resolved to, and by which binding.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved {
    Command,
    Lambda,
    Native,
    /// The name resolved nowhere. Distinct from "the name is unknown": an
    /// interned name with an empty slot is a name that was looked up.
    Unbound,
}

/// Names to symbols, and back.
///
/// Back matters as much as forward: `Symbol` is internal and a **name** is
/// what crosses every boundary — the `Debug` rendering the goldens capture,
/// and the world file, where `save.lambdas` bincodes whole values and a
/// per-run index would be meaningless on reload.
#[derive(Debug, Default)]
pub struct Interner {
    names: Vec<String>,
    index: HashMap<String, Symbol>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a name, creating a symbol if it is new.
    ///
    /// The `$` sigil is **not** stripped here. `$println` and `println` are
    /// different strings and would intern to different symbols, which is
    /// exactly what a caller must not do — see [`Interner::intern_call`].
    pub fn intern(&mut self, name: &str) -> Symbol {
        if let Some(s) = self.index.get(name) {
            return *s;
        }
        let s = Symbol(self.names.len() as u32);
        self.names.push(name.to_string());
        self.index.insert(name.to_string(), s);
        s
    }

    /// Intern a name *as a call*: strip a leading `$` into a flag, and intern
    /// the remainder.
    ///
    /// This is where the sigil is separated, and it is deliberately not the
    /// parser's job. The grammar admits `$` inside `element`
    /// (`reference/bund_language_parser/bund.pest:36`), so `$println` lexes as
    /// one name; and D16 means a call target can be a run-time string that no
    /// parser sees. Both spellings therefore reach the **same slot**, and the
    /// flag decides whether the `lambda` binding is consulted.
    pub fn intern_call(&mut self, name: &str) -> (Symbol, bool) {
        match name.strip_prefix('$') {
            Some(rest) => (self.intern(rest), true),
            None => (self.intern(name), false),
        }
    }

    /// Look a name up **without** interning it.
    ///
    /// The reference's tables are pure reads on lookup: a miss returns an
    /// error and allocates nothing. A design that interned on every lookup
    /// would grow without bound on a program that repeatedly dispatches a
    /// computed miss, which D16 makes expressible. So a miss is answered from
    /// this, and only a successful *bind* creates a slot.
    pub fn lookup_call(&self, name: &str) -> Option<(Symbol, bool)> {
        let (bare, sigil) = match name.strip_prefix('$') {
            Some(rest) => (rest, true),
            None => (name, false),
        };
        self.index.get(bare).map(|s| (*s, sigil))
    }

    /// The name a symbol stands for.
    pub fn name(&self, s: Symbol) -> &str {
        &self.names[s.index()]
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// The word table.
#[derive(Debug, Default)]
pub struct Registry {
    pub interner: Interner,
    slots: Vec<Slot>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    fn slot_mut(&mut self, s: Symbol) -> &mut Slot {
        if self.slots.len() <= s.index() {
            self.slots.resize_with(s.index() + 1, Slot::default);
        }
        &mut self.slots[s.index()]
    }

    pub fn slot(&self, s: Symbol) -> Option<&Slot> {
        self.slots.get(s.index())
    }

    /// Register a native.
    ///
    /// **Last write wins, and registrations are replayed rather than deduped.**
    /// F32 depends on it: the lambda registry binds `unregister` twice in
    /// consecutive statements
    /// (`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:89,90`) and
    /// the second wins, which is why no lambda can be unregistered. A builder
    /// that deduped would silently change which handler runs.
    pub fn register_native(&mut self, name: &str, f: NativeFn, effect: StackEffect, kind: WordKind) -> Symbol {
        let s = self.interner.intern(name);
        let slot = self.slot_mut(s);
        slot.native = Some(Native { f, effect, kind });
        slot.touch();
        s
    }

    /// Register a lambda. **Does not disturb the native binding**, matching
    /// `register_lambda`, which writes `vm.lambdas` alone
    /// (`reference/rust_multistackvm/src/multistackvm_lambdas.rs:8,13`).
    pub fn register_lambda(&mut self, name: &str, body: BundValue) -> Symbol {
        let s = self.interner.intern(name);
        let slot = self.slot_mut(s);
        slot.lambda = Some(body);
        slot.touch();
        s
    }

    pub fn register_alias(&mut self, alias: &str, target: &str) -> Symbol {
        let t = self.interner.intern(target);
        let a = self.interner.intern(alias);
        let slot = self.slot_mut(a);
        slot.alias = Some(t);
        slot.touch();
        a
    }

    pub fn register_command(&mut self, name: &str, f: NativeFn, effect: StackEffect, kind: WordKind) -> Symbol {
        let s = self.interner.intern(name);
        let slot = self.slot_mut(s);
        slot.command = Some(Native { f, effect, kind });
        slot.touch();
        s
    }

    /// Remove a lambda binding, leaving the rest of the slot alone.
    ///
    /// This is F32's fix: the reference has no reachable way to do it, because
    /// `unregister` is bound twice and the class variant wins.
    pub fn unregister_lambda(&mut self, s: Symbol) {
        if self.slots.len() > s.index() {
            let slot = &mut self.slots[s.index()];
            slot.lambda = None;
            slot.touch();
        }
    }

    /// Follow alias links to a fixed point.
    ///
    /// **A deviation, and it starts at two links.** `apply` resolves one link
    /// and `i` resolves another, so a plain name follows two; `$name` enters
    /// at `i` and follows **one**. On the oracle, with `a2 → b2 → println`,
    /// `a2` succeeds and `$a2` fails with `Inline b2 not registered`. Resolving
    /// to a fixed point makes both succeed. No such chain exists in the
    /// reference's 70 registrations, but D16 lets a program build one.
    fn follow(&self, mut s: Symbol) -> Symbol {
        let mut guard = 0;
        while let Some(slot) = self.slots.get(s.index())
            && let Some(next) = slot.alias
        {
            s = next;
            guard += 1;
            // A cycle is constructible through `alias` at run time. Stopping
            // is better than looping; the caller sees the last link.
            if guard > 64 {
                break;
            }
        }
        s
    }

    /// Resolve a call, in the reference's order.
    ///
    /// Command first — `is_command` fires at
    /// `reference/rust_multistackvm/src/multistackvm_apply.rs:16` and returns
    /// at `:17`, *before* the `autoadd` test at `:19`, which is why `autoadd`
    /// does not precede everything. Then the sigil decides whether `lambda` is
    /// consulted, then `native`.
    pub fn resolve(&self, s: Symbol, sigil: bool) -> Resolved {
        if let Some(slot) = self.slots.get(s.index())
            && slot.command.is_some()
        {
            return Resolved::Command;
        }
        let target = self.follow(s);
        let Some(slot) = self.slots.get(target.index()) else {
            return Resolved::Unbound;
        };
        if !sigil && slot.lambda.is_some() {
            return Resolved::Lambda;
        }
        if slot.native.is_some() {
            return Resolved::Native;
        }
        Resolved::Unbound
    }

    /// Slots that carry at least one binding. A name interned but never bound
    /// is not a word.
    pub fn bound(&self) -> usize {
        self.slots.iter().filter(|s| !s.is_empty()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop(_: &mut dyn Vm) -> Result<(), Error> {
        Ok(())
    }
    fn eff() -> StackEffect {
        StackEffect { consumes: 0, produces: 0 }
    }
    fn native(r: &mut Registry, name: &str) -> Symbol {
        r.register_native(name, noop, eff(), WordKind::Sync)
    }

    /// RFC-0002's central claim, and what its first review rejected the RFC
    /// for getting wrong: a name is a lambda *and* a native at once, and the
    /// `$` tells them apart.
    ///
    /// Confirmed on the oracle: after registering a lambda named `println`,
    /// `println` runs the lambda and `$println` runs the native.
    #[test]
    fn a_name_is_a_lambda_and_a_native_at_once() {
        let mut r = Registry::new();
        native(&mut r, "println");
        r.register_lambda("println", BundValue::Int(1));

        let (s, sigil) = r.interner.intern_call("println");
        assert!(!sigil);
        assert_eq!(r.resolve(s, sigil), Resolved::Lambda);

        let (s2, sigil2) = r.interner.intern_call("$println");
        assert!(sigil2);
        assert_eq!(s, s2, "both spellings must reach the same slot");
        assert_eq!(r.resolve(s2, sigil2), Resolved::Native);
    }

    /// Writing a lambda must not destroy the native. A single-enum slot
    /// cannot express this, which is why `Slot` is six `Option`s.
    #[test]
    fn registering_a_lambda_leaves_the_native_alone() {
        let mut r = Registry::new();
        let s = native(&mut r, "w");
        r.register_lambda("w", BundValue::Int(1));
        let slot = r.slot(s).expect("slot");
        assert!(slot.native.is_some(), "the native was destroyed");
        assert!(slot.lambda.is_some());
    }

    /// D16 means a call target can be a run-time string. Interning a *miss*
    /// must not create a slot, or a program dispatching a computed miss in a
    /// loop grows memory without bound where the reference does not.
    #[test]
    fn looking_up_a_miss_does_not_retain() {
        let mut r = Registry::new();
        native(&mut r, "known");
        let before = r.interner.len();
        for i in 0..1000 {
            assert!(r.interner.lookup_call(&format!("miss{i}")).is_none());
        }
        assert_eq!(r.interner.len(), before, "a miss must not intern");
        assert_eq!(r.bound(), 1);
    }

    /// The sigil is honoured for a name no parser ever saw — `"$println" !`.
    #[test]
    fn a_runtime_string_carries_its_sigil() {
        let mut r = Registry::new();
        native(&mut r, "println");
        r.register_lambda("println", BundValue::Int(1));
        // As `execute` would: a string off the stack, never lexed as a name.
        let (s, sigil) = r
            .interner
            .lookup_call("$println")
            .expect("the bare name is known");
        assert!(sigil);
        assert_eq!(r.resolve(s, sigil), Resolved::Native);
    }

    /// F32: the second of two identical registrations wins, and a builder
    /// that deduped would silently change which handler runs.
    #[test]
    fn registration_is_last_write_wins() {
        fn first(_: &mut dyn Vm) -> Result<(), Error> {
            Err(Error("first".into()))
        }
        fn second(_: &mut dyn Vm) -> Result<(), Error> {
            Err(Error("second".into()))
        }
        let mut r = Registry::new();
        r.register_native("unregister", first, eff(), WordKind::Sync);
        let s = r.register_native("unregister", second, eff(), WordKind::Sync);
        let f = r.slot(s).unwrap().native.unwrap().f;
        struct NoVm;
        impl Vm for NoVm {
            fn push(&mut self, _: BundValue) {}
            fn pull(&mut self) -> Option<BundValue> {
                None
            }
            fn depth(&self) -> usize {
                0
            }
        }
        assert_eq!(f(&mut NoVm), Err(Error("second".into())));
    }

    /// F32's fix: a lambda can be unregistered, and doing so leaves the
    /// native standing.
    #[test]
    fn a_lambda_can_be_unregistered() {
        let mut r = Registry::new();
        let s = native(&mut r, "println");
        r.register_lambda("println", BundValue::Int(1));
        assert_eq!(r.resolve(s, false), Resolved::Lambda);
        r.unregister_lambda(s);
        assert_eq!(r.resolve(s, false), Resolved::Native);
    }

    /// The generation is what lets a redefinition invalidate inline caches
    /// without a scan.
    #[test]
    fn rewriting_a_binding_bumps_the_generation() {
        let mut r = Registry::new();
        let s = native(&mut r, "w");
        let g = r.slot(s).unwrap().generation();
        r.register_lambda("w", BundValue::Int(1));
        assert!(r.slot(s).unwrap().generation() > g);
    }

    /// Saturating, not wrapping: `register` is a word, so a program can
    /// rewrite in a loop, and a wrapped generation would let a stale cache
    /// match.
    #[test]
    fn the_generation_saturates() {
        let mut slot = Slot::default();
        slot.generation = u32::MAX;
        slot.touch();
        assert_eq!(slot.generation, u32::MAX);
    }

    /// The deviation RFC-0002 records: fixed-point resolution, where the
    /// reference follows two links for a plain name and one for `$name`.
    /// On the oracle `a2` succeeds and `$a2` fails.
    #[test]
    fn an_alias_chain_resolves_to_a_fixed_point() {
        let mut r = Registry::new();
        native(&mut r, "println");
        r.register_alias("b2", "println");
        let a2 = r.register_alias("a2", "b2");
        assert_eq!(r.resolve(a2, false), Resolved::Native);
        assert_eq!(
            r.resolve(a2, true),
            Resolved::Native,
            "$a2 must resolve too — this is the deviation"
        );
    }

    /// A cycle is constructible through `alias` at run time. Stopping beats
    /// looping.
    #[test]
    fn an_alias_cycle_terminates() {
        let mut r = Registry::new();
        r.register_alias("a", "b");
        let a = r.register_alias("b", "a");
        let _ = r.resolve(a, false);
    }

    /// Command fires before the sigil and before the lambda, matching
    /// `apply`'s order: `is_command` returns at `:17`, ahead of everything.
    #[test]
    fn a_command_wins_over_a_lambda_and_a_native() {
        let mut r = Registry::new();
        let s = r.register_command("c", noop, eff(), WordKind::Sync);
        r.register_native("c", noop, eff(), WordKind::Sync);
        r.register_lambda("c", BundValue::Int(1));
        assert_eq!(r.resolve(s, false), Resolved::Command);
        assert_eq!(r.resolve(s, true), Resolved::Command);
    }

    /// A name reaches its slot by index — no string hashing, and no
    /// `format!("{}_inline", …)`, which is where F31 lived.
    #[test]
    fn a_symbol_indexes_and_round_trips_to_its_name() {
        let mut r = Registry::new();
        let s = r.interner.intern("dup_one");
        assert_eq!(r.interner.name(s), "dup_one");
        assert_eq!(r.interner.intern("dup_one"), s, "interning is stable");
    }
}
