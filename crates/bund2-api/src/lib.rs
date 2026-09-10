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
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

pub mod diag;

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

/// What the interpreter offers a native word.
///
/// **The tier merge forces this wider than an external word needs, and
/// RFC-0002 did not say so.** The reference splits natives in two: the VM tier
/// takes `&mut VM` and the stack tier takes `&mut TS`
/// (`reference/rust_multistackvm/src/multistackvm.rs:8`,
/// `reference/rust_multistack/src/ts.rs:9`). RFC-0002 merges them into one
/// `NativeFn`, which means a stack word like `rotate_current_left` is the same
/// type as an external package's word — so the receiver has to expose stack
/// rotation, named stacks and the workbench, none of which an external word
/// wants.
///
/// The alternative is two receivers, which is the split the merge exists to
/// remove. Carried as a stated consequence: **the stable surface is as wide
/// as the widest native**, and the 31 stack-layer words are the widest.
pub trait Vm {
    // --- the current stack -------------------------------------------------
    fn push(&mut self, v: BundValue);
    fn pull(&mut self) -> Option<BundValue>;
    fn depth(&self) -> usize;
    fn peek(&self) -> Option<BundValue>;
    /// The value `n` places below the top; `n == 0` is [`Vm::peek`].
    ///
    /// A fragment guard asks about the top few values and nothing else
    /// (RFC-0005 §S5). Written with [`Vm::snapshot`] instead, that question
    /// clones the whole stack to read two of its values — `O(depth)`
    /// allocation on the path whose entire justification is being cheaper than
    /// a call.
    fn peek_at(&self, n: usize) -> Option<BundValue>;
    fn clear(&mut self);
    /// The stack's contents **bottom-first**, without disturbing it.
    ///
    /// The order is the reference's: `debug.display_stack` iterates
    /// `&current_stack.stack` directly
    /// (`reference/Bund/src/stdlib/functions/debug_fun/debug_display_stack.rs:25`),
    /// and pushes append, so the first row of the box is the *bottom* of the
    /// stack. `string_concatenation.golden:9-11` confirms it: the program
    /// leaves `true` then swaps a string over it, and the string prints first.
    ///
    /// This exists so a reader is not a mutator. Pulling everything and
    /// pushing it back reverses that order and, worse, re-runs `push`, which
    /// rewrites the stack tag on every value it touches.
    fn snapshot(&self) -> Vec<BundValue>;
    fn snapshot_workbench(&self) -> Vec<BundValue>;
    /// Circular, which is why a stack is a `VecDeque`.
    fn rotate_left(&mut self);
    fn rotate_right(&mut self);

    // --- named stacks ------------------------------------------------------
    fn current_name(&self) -> String;
    /// Make a stack current, creating it if absent. Rotates the
    /// stack-of-stacks rather than reassigning.
    fn to_stack(&mut self, name: &str);
    fn stack_exists(&self, name: &str) -> bool;
    /// Create a stack with a bound on its depth, if it does not exist.
    fn ensure_stack_with_capacity(&mut self, name: &str, cap: usize);
    fn ensure_stack(&mut self, name: &str);
    fn depth_of(&self, name: &str) -> usize;
    fn push_to(&mut self, name: &str, v: BundValue);
    fn pull_from(&mut self, name: &str) -> Option<BundValue>;
    fn clear_stack(&mut self, name: &str);
    fn drop_stack(&mut self, name: &str);
    /// Rotate the stack **of stacks**, which is what `stacks_left` acts on.
    fn rotate_stacks_left(&mut self);
    fn rotate_stacks_right(&mut self);

    // --- the workbench -----------------------------------------------------
    fn push_workbench(&mut self, v: BundValue);
    fn pull_workbench(&mut self) -> Option<BundValue>;
    /// How deep the workbench is.
    ///
    /// Every `.`-suffixed word guards on this before pulling
    /// (`reference/Bund/src/stdlib/functions/string/prefix_suffix.rs:23-26`),
    /// and `snapshot_workbench().is_empty()` would clone the whole thing to
    /// answer a question about its length.
    fn workbench_depth(&self) -> usize;

    // --- re-entering evaluation --------------------------------------------
    /// Apply one value, as `VM::apply` does
    /// (`reference/rust_multistackvm/src/multistackvm_apply.rs:8-104`).
    ///
    /// `execute` needs this: its PTR/STRING/CALL arm calls by name and its
    /// LAMBDA arm evaluates a body
    /// (`reference/rust_multistackvm/src/stdlib/execute.rs:30,94`), so a native
    /// word has to be able to re-enter evaluation.
    ///
    /// **This is the surface RFC-0003's S4a replaces.** Under the frame loop a
    /// native returns a *request* to push a frame rather than calling back, so
    /// that Rust depth stops tracking Bund depth. Until the loop exists this is
    /// a direct call and the recursion is real.
    fn apply(&mut self, v: BundValue) -> Result<(), Error>;

    /// Evaluate the body a LAMBDA value carries **now**, for a native that
    /// inspects the stack afterwards
    /// (`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:8-32`).
    ///
    /// **The value, not its items — D42.** RFC-0005's compiled cache and
    /// promotion counter key on the body's `Rc` (D35), and a copied slice has
    /// none: the `eval_body(&[BundValue])` this replaces discarded the key at
    /// every entry, and `times` rebuilt the copy on every call. A LIST value is
    /// accepted as a body too. Anything else is an internal error, since every
    /// caller has already checked the tag.
    ///
    /// Costs one Rust frame per *native*, not per Bund call. Prefer
    /// [`Vm::tail_lambda`] wherever nothing runs after the body.
    fn eval_lambda(&mut self, lambda: &BundValue) -> Result<(), Error>;

    /// Run the body `lambda` carries **after this native returns** — RFC-0003
    /// §S4a's request. The value is kept, not copied, for the reason
    /// [`Vm::eval_lambda`] gives (D42).
    ///
    /// For tail positions, which is most of them: a lambda call, `if`'s
    /// branch, `execute` on a LAMBDA. The loop pushes a frame, so nothing
    /// recurses and Bund call depth costs heap rather than Rust stack.
    ///
    /// A native must not touch the stack after calling this expecting the
    /// body's effect — the body has not run yet. That is the distinction from
    /// [`Vm::eval_lambda`], and the reason both exist.
    fn tail_lambda(&mut self, lambda: BundValue);

    /// Run `body` on `stack`, returning to the current stack **however the
    /// body leaves** — RFC-0003 §S4's exit action.
    ///
    /// This is F57's fix made structural. The reference restores with a
    /// statement placed after three early returns
    /// (`reference/Bund/src/stdlib/functions/conditional/conditional_ctx.rs:60-74`),
    /// so a failure inside a context strands the interpreter on the context's
    /// stack. A frame that carries its exit action cannot skip it, because the
    /// unwinder runs it on the way past.
    fn scoped_call(&mut self, stack: &str, body: Vec<BundValue>) -> Result<(), Error>;

    // --- the word table ----------------------------------------------------
    /// Bind a name to a lambda, as `register` does
    /// (`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:5-38`).
    fn register_lambda(&mut self, name: &str, body: BundValue);
    /// Unbind a lambda, leaving any native under the same name standing (F32).
    fn unregister_lambda(&mut self, name: &str);
    /// Bind `alias` to `target`, and say whether that closed a cycle.
    fn register_alias(&mut self, alias: &str, target: &str) -> bool;
    /// Unbind an alias.
    fn unregister_alias(&mut self, alias: &str);
    /// Is this name bound to a lambda?
    fn is_lambda(&self, name: &str) -> bool;
    /// Is this name bound to a native?
    fn is_native(&self, name: &str) -> bool;
    /// Is this name bound to an alias?
    fn is_alias(&self, name: &str) -> bool;
    /// The lambda bound to this name, if any.
    fn get_lambda(&self, name: &str) -> Option<BundValue>;
    /// The handler for a conditional `type`, if one is bound (S7).
    fn conditional(&self, ty: &str) -> Option<ConditionalFn>;

    // --- classes and methods (RFC-0009) ------------------------------------
    fn register_class(&mut self, name: &str, class: BundValue);
    fn class(&self, name: &str) -> Option<BundValue>;
    fn is_class(&self, name: &str) -> bool;
    fn unregister_class(&mut self, name: &str);
    fn method(&self, name: &str) -> Option<NativeFn>;
    fn is_method(&self, name: &str) -> bool;

    // --- variables, the sixth namespace ------------------------------------
    /// A word's declared effect, or `None` if it has none.
    fn effect_of(&self, name: &str) -> Option<StackEffect>;
    fn register_var(&mut self, name: &str, value: BundValue);
    fn var(&self, name: &str) -> Option<BundValue>;
    fn unregister_var(&mut self, name: &str);

    /// Emit a diagnostic.
    ///
    /// **The hook a word uses to say something without deciding how it looks.**
    /// `?error` reports through this rather than printing, so a TUI receives a
    /// structured [`Diagnostic`] instead of finding text on stdout. The
    /// implementation attaches the current stack name, and a stack snapshot
    /// only if the reporter asks for one.
    fn report(&mut self, d: diag::Diagnostic);

    // --- contexts ----------------------------------------------------------
    /// How many contexts `( … )` has opened and not yet closed.
    ///
    /// The reference cannot ask this: `stacks_stack` conflates the base stack
    /// with stacks opened by `(`, which is why `endcontext`'s own guard can
    /// never fire (F60). Tracking the depth separately is what lets it.
    fn context_depth(&self) -> usize;
    fn push_context(&mut self, name: &str);
    /// Pop one context, returning the stack to restore to. `None` when no
    /// context is open.
    fn pop_context(&mut self) -> Option<String>;
}

/// A word's failure. RFC-0003 replaces this with a spanned error value.
#[derive(Debug, Clone, PartialEq)]
pub struct Error(pub String);

impl Error {
    /// **An invariant this code relies on did not hold.**
    ///
    /// Bund2 does not abort on an internal inconsistency. An interpreter that
    /// panics takes the user's program state with it, explains nothing, and
    /// leaves a stack trace through Rust internals that names no Bund word. So
    /// an impossible state becomes an ordinary error, routed through the same
    /// diagnostic path as any other — with a reason that says plainly which
    /// invariant broke, that it is a defect in Bund2 rather than in the
    /// program, and where to report it.
    ///
    /// Reach for this only where the condition genuinely cannot arise from any
    /// input. Anything a program can cause is a normal error and deserves a
    /// message about the program, not about Bund2.
    pub fn internal(what: impl std::fmt::Display) -> Self {
        Error(format!(
            "internal error: {what}. This is a defect in Bund2, not in the              program being run — the interpreter reached a state it believes              impossible and stopped rather than continue with values it cannot              trust. Please report it with the program that produced it."
        ))
    }

    /// Whether this reports a broken invariant rather than a bad program.
    pub fn is_internal(&self) -> bool {
        self.0.starts_with("internal error: ")
    }

    /// **The machine stack is nearly exhausted** — RFC-0005 §S8's Tier 0
    /// floor, and F85's fix.
    ///
    /// A native that runs a body synchronously spends a Rust frame per level,
    /// so recursion through one is bounded by the machine stack rather than by
    /// the heap RFC-0003's frame loop gives direct recursion. Below the floor
    /// evaluation is refused with this, instead of nesting until the process
    /// aborts. It is a Bund-level error: reported like any other, and
    /// catchable by `?try`.
    pub fn stack_exhausted() -> Self {
        Error(format!(
            "{STACK_EXHAUSTED}recursion through a word that runs a lambda — `times`, `loop`, \
             `map`, `while`, a conditional, `?try` or a method — spends machine stack at \
             every level, and this recursion ran out of it. A word that calls itself \
             directly runs on the heap instead and has no such limit."
        ))
    }

    /// Whether this is [`Error::stack_exhausted`].
    pub fn is_stack_exhausted(&self) -> bool {
        self.0.starts_with(STACK_EXHAUSTED)
    }

    /// Prefix this error with the word that ran the failing body, as a native
    /// reporting a failure inside its lambda does — **except a stack
    /// exhaustion, which passes through unchanged.**
    ///
    /// That exception is not cosmetic. The exhaustion is raised thousands of
    /// levels deep and returns through every one of them; if each level
    /// prefixed it, the message would grow by a line per level and be copied
    /// in full at each, which is quadratic in the depth — megabytes of text and
    /// seconds of copying to report that the stack ran out.
    pub fn context(self, prefix: impl std::fmt::Display) -> Self {
        if self.is_stack_exhausted() {
            self
        } else {
            Error(format!("{prefix}{}", self.0))
        }
    }
}

/// The prefix that identifies [`Error::stack_exhausted`], as `internal error: `
/// identifies [`Error::internal`].
const STACK_EXHAUSTED: &str = "machine stack exhausted: ";

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
    /// **The word's effect is not this pair.** RFC-0004 §S1's `Opaque` arm,
    /// carried as a flag rather than a third variant so the 30-odd existing
    /// `eff(a, b)` sites keep compiling.
    ///
    /// Set for a word whose effect depends on what it is handed rather than on
    /// how many operands it takes: `!` dispatches on eight tags, `apply` runs
    /// whatever it is given, the `*` folds consume the whole stack (D12), and
    /// `graph!` pushes back what is not a LIST. `bund2 check` stops tracking
    /// depth at one of these rather than guessing, and **says how many it
    /// stopped at** — criterion 5, because a checker that reports "no problems"
    /// without saying it skipped every `!` is worse than none.
    pub opaque: bool,
}

impl StackEffect {
    /// A word that takes `consumes` and leaves `produces`.
    pub const fn fixed(consumes: u8, produces: u8) -> Self {
        Self {
            consumes,
            produces,
            opaque: false,
        }
    }

    /// A word whose effect cannot be stated as a pair. `consumes` is still a
    /// floor — the depth it needs before it can run at all.
    pub const fn opaque(floor: u8) -> Self {
        Self {
            consumes: floor,
            produces: 0,
            opaque: true,
        }
    }
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
#[derive(Debug, Default, Clone)]
pub struct Interner {
    names: Vec<String>,
    index: HashMap<String, Symbol>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every interned name, in symbol order.
    pub fn names(&self) -> &[String] {
        &self.names
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
#[derive(Debug, Default, Clone)]
pub struct Registry {
    pub interner: Interner,
    slots: Vec<Slot>,
    /// **The conditional table — RFC-0003 §S7.**
    ///
    /// The reference keeps this in a `lazy_static` `Mutex<BTreeMap<…>>` called
    /// `CF` (`reference/rust_multistackvm/src/stdlib/execute_types/mod.rs:11-16`),
    /// a fourth live dispatch table alongside the three word tables. It is
    /// keyed by a conditional's `type` **string**, not by a word name, so it
    /// cannot fold into the slot table: `!` on a CONDITIONAL reads `type` and
    /// looks it up (`execute_conditionals.rs:9-25`).
    ///
    /// Here it is registry state, populated at construction. Same lookup, same
    /// failure message, no global mutable state — the `BUND` mutex went the
    /// same way in RFC-0002.
    ///
    /// A `BTreeMap`, as the reference's is, so iteration order is stable.
    conditionals: std::collections::BTreeMap<String, ConditionalFn>,
    /// **The class registry — RFC-0009.** Separate from the word table, as the
    /// reference's is (`reference/rust_multistackvm/src/multistackvm.rs:28`):
    /// `register` files a class here, not among the words, which is why
    /// `:Probe register` then `Probe` reports `Inline Probe not registered`.
    classes: std::collections::BTreeMap<String, BundValue>,
    /// **The variable table** — a sixth namespace
    /// (`reference/rust_multistackvm/src/multistackvm_vars.rs:14-70`).
    ///
    /// Separate from the word table, and **not consulted by name resolution**:
    /// the only reader is the `var?` word
    /// (`reference/rust_multistackvm/src/stdlib/vars/resolve.rs:12`). So
    /// registering a var called `dup` does not shadow `dup`, and a bare `x`
    /// after `:x 1 var` is still an unregistered word.
    vars: std::collections::BTreeMap<String, BundValue>,
    /// **The method table** — the fifth name-keyed table
    /// (`reference/rust_multistackvm/src/multistackvm_methods.rs:8`). A method
    /// is reachable only through a class slot; nothing dispatches it by name.
    methods: std::collections::BTreeMap<String, NativeFn>,
    /// Bumped by `register_class` and `register_method`. RFC-0002's generation
    /// is **per word-table slot** and never sees either of these tables, so a
    /// dispatch cache keyed on a class needs its own.
    class_generation: u32,
    method_generation: u32,
}

/// A conditional handler: it receives the CONDITIONAL value itself, because
/// every branch it might run is in one of that value's slots
/// (`reference/rust_multistackvm/src/stdlib/execute_types/mod.rs:8`).
pub type ConditionalFn = fn(&mut dyn Vm, BundValue) -> Result<(), Error>;

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a conditional `type` string to its handler.
    ///
    /// The reference populates this from two crates — eight types from `Bund`
    /// (`reference/Bund/src/stdlib/functions/conditional/mod.rs:20-27`) and
    /// `through` from `rust_multistackvm`
    /// (`reference/rust_multistackvm/src/stdlib/execute_types/mod.rs:36`) —
    /// which is what makes the global mutex load-bearing there and merely
    /// convenient here.
    pub fn register_conditional(&mut self, ty: &str, f: ConditionalFn) {
        self.conditionals.insert(ty.to_string(), f);
    }

    /// The handler for a conditional `type`, if one is bound.
    pub fn conditional(&self, ty: &str) -> Option<ConditionalFn> {
        self.conditionals.get(ty).copied()
    }

    /// File a class under a name.
    ///
    /// The reference validates only the CLASS tag and inserts
    /// (`reference/rust_multistackvm/src/multistackvm_classes.rs:7-20`) — it
    /// does **not** check that `.super` names registered parents, so a class
    /// may be registered whose parents are not. RFC-0009 §S1 flattens at
    /// construction rather than here for exactly that reason.
    pub fn register_class(&mut self, name: &str, class: BundValue) {
        self.classes.insert(name.to_string(), class);
        self.class_generation = self.class_generation.saturating_add(1);
    }

    pub fn class(&self, name: &str) -> Option<BundValue> {
        self.classes.get(name).cloned()
    }

    /// Every name bound to a lambda, for `bund2 infer`.
    pub fn lambda_names(&self) -> Vec<String> {
        self.interner
            .names()
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                self.slots
                    .get(*i)
                    .is_some_and(|s| s.lambda.is_some() && s.native.is_none())
            })
            .map(|(_, n)| n.clone())
            .collect()
    }

    /// The body a name is bound to, following aliases.
    ///
    /// What `bund2 check` needs to infer a Bund word's effect rather than
    /// abandon at it (RFC-0004 §S3).
    /// The LAMBDA value bound to `name`, following aliases — the value itself,
    /// so its `Rc` survives to wherever it is run (D42). [`Registry::lambda_body`]
    /// copies the items, which suits a reader but not an evaluator.
    pub fn lambda_value(&self, name: &str) -> Option<BundValue> {
        let &s = self.interner.index.get(name)?;
        let t = self.follow(s);
        self.slots.get(t.index()).and_then(|slot| slot.lambda.clone())
    }

    pub fn lambda_body(&self, name: &str) -> Option<Vec<BundValue>> {
        let &s = self.interner.index.get(name)?;
        let t = self.follow(s);
        self.slots
            .get(t.index())
            .and_then(|slot| slot.lambda.as_ref())
            .and_then(|v| v.as_lambda().map(<[BundValue]>::to_vec))
    }

    /// The declared effect of one name, following aliases to a fixed point.
    ///
    /// `None` when the name resolves to nothing, or to something with no
    /// declared effect — a lambda, a class, a method. `bund2 check` treats
    /// that as "cannot tell" rather than "takes nothing", because D16 lets a
    /// name be bound at run time.
    pub fn effect_of(&self, name: &str) -> Option<StackEffect> {
        let &s = self.interner.index.get(name)?;
        let t = self.follow(s);
        self.slots
            .get(t.index())
            .and_then(|slot| slot.native.or(slot.command).map(|n| n.effect))
    }

    /// Every native's declared effect, by name, sorted.
    ///
    /// Only slots carrying a `native`: an alias has no effect of its own and a
    /// lambda's is inferred rather than declared, so neither has a number to
    /// cross-check. This is the left-hand side of `cargo xtask effects`.
    pub fn declared_effects(&self) -> Vec<(String, StackEffect)> {
        let mut out: Vec<(String, StackEffect)> = self
            .interner
            .names()
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                self.slots
                    .get(i)
                    .and_then(|s| s.native.as_ref())
                    .map(|nat| (n.clone(), nat.effect))
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Every name that resolves to something callable, sorted.
    ///
    /// **What "callable" means here decides a health number**, so it is
    /// spelled out: a name counts if its slot carries a `native`, a `command`,
    /// a `lambda` or an `alias`. A slot carrying only a `class` or a `method`
    /// does not — `register` files a class in its own table and a method is
    /// reachable only through a class slot, so neither is a word a program can
    /// call by name.
    ///
    /// This is what `cargo xtask coverage` joins against the reference's
    /// registry to answer how much of the language exists, so it must mean
    /// "a program can call this", not "the interner has seen this string".
    pub fn word_names(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .interner
            .names()
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                self.slots.get(*i).is_some_and(|s| {
                    s.native.is_some()
                        || s.command.is_some()
                        || s.lambda.is_some()
                        || s.alias.is_some()
                })
            })
            .map(|(_, n)| n.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    pub fn register_var(&mut self, name: &str, value: BundValue) {
        self.vars.insert(name.to_string(), value);
    }

    pub fn var(&self, name: &str) -> Option<BundValue> {
        self.vars.get(name).cloned()
    }

    pub fn unregister_var(&mut self, name: &str) {
        self.vars.remove(name);
    }

    pub fn is_class(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    pub fn unregister_class(&mut self, name: &str) {
        if self.classes.remove(name).is_some() {
            self.class_generation = self.class_generation.saturating_add(1);
        }
    }

    /// Bind a method name to a native. Methods are reached through a class
    /// slot holding a PTR, never by word dispatch.
    pub fn register_method(&mut self, name: &str, f: NativeFn) {
        self.methods.insert(name.to_string(), f);
        self.method_generation = self.method_generation.saturating_add(1);
    }

    pub fn method(&self, name: &str) -> Option<NativeFn> {
        self.methods.get(name).copied()
    }

    pub fn is_method(&self, name: &str) -> bool {
        self.methods.contains_key(name)
    }

    /// The two generations a dispatch cache keys on (§S5).
    pub fn oop_generation(&self) -> (u32, u32) {
        (self.class_generation, self.method_generation)
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
    pub fn register_native(
        &mut self,
        name: &str,
        f: NativeFn,
        effect: StackEffect,
        kind: WordKind,
    ) -> Symbol {
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

    /// Unbind an alias, leaving whatever else the slot holds.
    pub fn unregister_alias(&mut self, alias: &str) {
        let Some(&a) = self.interner.index.get(alias) else {
            return;
        };
        let slot = self.slot_mut(a);
        slot.alias = None;
        slot.touch();
    }

    /// Would resolving this name walk in a circle?
    ///
    /// **Asked after registering, not before**, because the answer is about
    /// the table as it now stands and the caller wants to warn about what it
    /// just did. The reference has no such check — `register_alias` inserts
    /// and returns (`reference/rust_multistackvm/src/multistackvm_alias.rs:5-15`)
    /// — so `:a :b alias :b :a alias` builds a loop there and resolution spins
    /// until [`follow`]'s guard stops it.
    ///
    /// Bund2 resolves to a fixed point with a 64-link guard, so a cycle does
    /// not hang; it silently resolves to whatever link the guard stopped on.
    /// That is the wrong kind of quiet, and this is what lets the `alias` word
    /// say so.
    ///
    /// [`follow`]: Registry::follow
    pub fn alias_cycles(&self, name: &str) -> bool {
        let Some(&start) = self.interner.index.get(name) else {
            return false;
        };
        let mut slow = start;
        let mut fast = start;
        loop {
            let Some(f1) = self.slots.get(fast.index()).and_then(|s| s.alias) else {
                return false;
            };
            let Some(f2) = self.slots.get(f1.index()).and_then(|s| s.alias) else {
                return false;
            };
            let Some(s1) = self.slots.get(slow.index()).and_then(|s| s.alias) else {
                return false;
            };
            slow = s1;
            fast = f2;
            if slow == fast {
                return true;
            }
        }
    }

    pub fn register_command(
        &mut self,
        name: &str,
        f: NativeFn,
        effect: StackEffect,
        kind: WordKind,
    ) -> Symbol {
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

    /// Where an alias chain lands, for a caller that needs the target slot
    /// rather than the verdict — dispatch does, because the handler lives on
    /// the target and not on the alias.
    pub fn resolve_target(&self, s: Symbol) -> Symbol {
        self.follow(s)
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

    /// A `Vm` that does nothing, for the tests that only need to *call* a
    /// native and look at what it returned.
    ///
    /// The trait has no default methods on purpose, so this has to spell out
    /// every one — a `Vm` whose stack operations silently succeed while
    /// dropping values is the kind of stub a real word could be tested against
    /// by accident.
    struct NoVm;
    impl Vm for NoVm {
        fn push(&mut self, _: BundValue) {}
        fn pull(&mut self) -> Option<BundValue> {
            None
        }
        fn depth(&self) -> usize {
            0
        }
        fn peek(&self) -> Option<BundValue> {
            None
        }
        fn peek_at(&self, _: usize) -> Option<BundValue> {
            None
        }
        fn clear(&mut self) {}
        fn snapshot(&self) -> Vec<BundValue> {
            Vec::new()
        }
        fn snapshot_workbench(&self) -> Vec<BundValue> {
            Vec::new()
        }
        fn rotate_left(&mut self) {}
        fn rotate_right(&mut self) {}
        fn current_name(&self) -> String {
            "main".into()
        }
        fn to_stack(&mut self, _: &str) {}
        fn ensure_stack_with_capacity(&mut self, _: &str, _: usize) {}
        fn stack_exists(&self, _: &str) -> bool {
            false
        }
        fn ensure_stack(&mut self, _: &str) {}
        fn depth_of(&self, _: &str) -> usize {
            0
        }
        fn push_to(&mut self, _: &str, _: BundValue) {}
        fn pull_from(&mut self, _: &str) -> Option<BundValue> {
            None
        }
        fn clear_stack(&mut self, _: &str) {}
        fn drop_stack(&mut self, _: &str) {}
        fn rotate_stacks_left(&mut self) {}
        fn rotate_stacks_right(&mut self) {}
        fn push_workbench(&mut self, _: BundValue) {}
        fn pull_workbench(&mut self) -> Option<BundValue> {
            None
        }
        fn workbench_depth(&self) -> usize {
            0
        }
        fn apply(&mut self, _: BundValue) -> Result<(), Error> {
            Ok(())
        }
        fn eval_lambda(&mut self, _: &BundValue) -> Result<(), Error> {
            Ok(())
        }
        fn tail_lambda(&mut self, _: BundValue) {}
        fn scoped_call(&mut self, _: &str, _: Vec<BundValue>) -> Result<(), Error> {
            Ok(())
        }
        fn register_lambda(&mut self, _: &str, _: BundValue) {}
        fn unregister_lambda(&mut self, _: &str) {}
        fn register_alias(&mut self, _: &str, _: &str) -> bool {
            false
        }
        fn unregister_alias(&mut self, _: &str) {}
        fn effect_of(&self, _: &str) -> Option<StackEffect> {
            None
        }
        fn register_var(&mut self, _: &str, _: BundValue) {}
        fn var(&self, _: &str) -> Option<BundValue> {
            None
        }
        fn unregister_var(&mut self, _: &str) {}
        fn is_lambda(&self, _: &str) -> bool {
            false
        }
        fn is_native(&self, _: &str) -> bool {
            false
        }
        fn is_alias(&self, _: &str) -> bool {
            false
        }
        fn get_lambda(&self, _: &str) -> Option<BundValue> {
            None
        }
        fn conditional(&self, _: &str) -> Option<ConditionalFn> {
            None
        }
        fn register_class(&mut self, _: &str, _: BundValue) {}
        fn class(&self, _: &str) -> Option<BundValue> {
            None
        }
        fn is_class(&self, _: &str) -> bool {
            false
        }
        fn unregister_class(&mut self, _: &str) {}
        fn method(&self, _: &str) -> Option<NativeFn> {
            None
        }
        fn is_method(&self, _: &str) -> bool {
            false
        }
        fn report(&mut self, _: diag::Diagnostic) {}
        fn context_depth(&self) -> usize {
            0
        }
        fn push_context(&mut self, _: &str) {}
        fn pop_context(&mut self) -> Option<String> {
            None
        }
    }
    fn eff() -> StackEffect {
        StackEffect::fixed(0, 0)
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
        r.register_lambda("println", BundValue::int(1));

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
        r.register_lambda("w", BundValue::int(1));
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
        r.register_lambda("println", BundValue::int(1));
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
        assert_eq!(f(&mut NoVm), Err(Error("second".into())));
    }

    /// F32's fix: a lambda can be unregistered, and doing so leaves the
    /// native standing.
    #[test]
    fn a_lambda_can_be_unregistered() {
        let mut r = Registry::new();
        let s = native(&mut r, "println");
        r.register_lambda("println", BundValue::int(1));
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
        r.register_lambda("w", BundValue::int(1));
        assert!(r.slot(s).unwrap().generation() > g);
    }

    /// Saturating, not wrapping: `register` is a word, so a program can
    /// rewrite in a loop, and a wrapped generation would let a stale cache
    /// match.
    #[test]
    fn the_generation_saturates() {
        let mut slot = Slot {
            generation: u32::MAX,
            ..Slot::default()
        };
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
        r.register_lambda("c", BundValue::int(1));
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
