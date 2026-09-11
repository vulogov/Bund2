//! Tier 0: the interpreter. Mandatory on every target.
//!
//! **Scope.** This implements RFC-0002's *dispatch* — the resolution order, the
//! stacks it dispatches over, and the `Vm` receiver `bund2-api` declares. It
//! stops short of RFC-0003's IR and frame loop, which is not written yet. What
//! it makes measurable is RFC-0002's criterion 3, which asks what a dispatch
//! allocates and until now had no VM to be measured in.
//!
//! **Not blocked on D3.** D3 rules what tier `bund.eval`'s output runs at,
//! which is a Tier-1 question; nothing here needs it. `bund.eval` is
//! implemented in `bund2-stdlib` (`singles.rs`), and applies each parsed value
//! through `Vm::apply`, so an eval'd token stream is never a compilation unit.

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

/// Executing a word's specialised arm — RFC-0005 §S6's first consumer.
pub mod frag;

use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use bund2_api::{Error, Registry, Resolved, Symbol, Vm};
use bund2_value::BundValue;

/// One named stack.
///
/// A `VecDeque`, because the reference's `Stack<T>` is one
/// (`reference/rust_multistack/src/stack.rs:11`) and the Library Guide
/// describes stacks as **circular buffers** you can rotate in either
/// direction while consuming from one end
/// (`reference/Bund/Documentation/Bund_Library_Guide/Introduction_the_art_of_stack_operations.typ:15`).
/// A `Vec` would make `rotate_left` O(n) and `stacks_left` is a real word.
///
/// **LIFO only.** The reference carries a `policy` flag for FIFO
/// (`reference/rust_multistack/src/stack.rs:30`), but nothing constructs one:
/// `add_named_fifo` has no caller, so every stack in a running Bund is LIFO
/// and both FIFO branches are dead. That is F27, and exposing the policy would
/// add a feature the reference advertises and does not have.
#[derive(Debug)]
pub struct Stack {
    /// This stack's own name, for the tag every push writes.
    ///
    /// Held here rather than passed in because `current_mut` had to hand back
    /// an owned `String` alongside the `&mut Stack` — the borrow checker will
    /// not lend both out of one map — and that cost **two allocations per
    /// push**: one for `current_name().to_string()` and one for the
    /// `entry(name.clone())` lookup. Neither had anything to do with the tag.
    ///
    /// `Rc<str>` rather than `String` so `Stacks::to_stack` can hand the name
    /// over without copying it.
    name: Rc<str>,
    /// The interned `"stack"` key, cached so a push clones an `Rc` instead of
    /// allocating a `String`. Two `String` allocations per push measured
    /// 36.5 ns against 16.3 ns for two `Rc` clones
    /// (`crates/bund2-bench/benches/boxing.rs`).
    tag_key: Rc<str>,
    /// This stack's name, interned. **D41**: a pushed scalar stores this
    /// symbol inline instead of being boxed to carry a map entry.
    sym: bund2_value::StackSym,
    items: VecDeque<BundValue>,
    /// A bound set by `ensure_stack_with_capacity`, if one was.
    ///
    /// Per-stack rather than a side table, so a push cannot consult one
    /// stack's length against another's bound — which is exactly what the
    /// reference does (`ts_push.rs:47-48` reads `current_stack_len()` and
    /// `stack_capacity(name)`), and F39 records.
    cap: Option<usize>,
}

impl Default for Stack {
    fn default() -> Self {
        Self::named("main")
    }
}

impl Stack {
    /// A stack that knows its own name.
    pub fn named(name: &str) -> Self {
        Self::from_rc(Rc::from(name))
    }

    /// The same, sharing an already-interned name — so the map key and the
    /// stack's own copy are one allocation, not two.
    pub fn from_rc(name: Rc<str>) -> Self {
        Self {
            sym: bund2_value::intern_stack_name(&name),
            name,
            tag_key: bund2_value::stack_tag_key(),
            items: VecDeque::new(),
            cap: None,
        }
    }

    /// Push, **writing the stack tag**.
    ///
    /// `TS::push` calls `set_tag("stack", …)` on every push with no type test
    /// (`reference/rust_multistack/src/ts_push.rs:25`), which is why a scalar
    /// that reaches a stack is boxed: an inline `Int` has nowhere to keep a
    /// tag. 47 of the 54 scalar renderings in the goldens carry one.
    ///
    /// This is the faithful shape, and it is what criterion 3 measures.
    /// RFC-0001 floats an alternative — carry the tag in the *slot* rather
    /// than the value, so a scalar on a stack stays unboxed — and defers it
    /// here, to RFC-0003, because it depends on how stacks are represented.
    /// It cannot fix the fossil case either: a value collected into a list
    /// keeps the tag of the stack it *was* on, which is why the inner values
    /// of the `valuemap` probe render `tags: {"stack": "main"}` while sitting
    /// inside a map.
    /// Push, dropping the **newest** value first when a capacity is set.
    ///
    /// The reference reads the capacity, and if the stack is already that
    /// deep, calls `curr.pull()` before pushing
    /// (`reference/rust_multistack/src/ts_push.rs:52-57`). `pull` takes from
    /// the top, so a capped stack discards the value most recently pushed,
    /// not the oldest — a stack with capacity 2 given `1 2 3 4` ends up
    /// holding `1` and `4`, confirmed against the oracle.
    ///
    /// That is not what "capacity" usually means, and it is preserved as
    /// written.
    fn push(&mut self, v: BundValue) {
        let (sym, name) = (self.sym, Rc::clone(&self.name));
        self.push_as(v, sym, &name);
    }


    /// Push, tagging with a name that is **not** this stack's own.
    ///
    /// One caller, and it is the reason this exists: the workbench "does not
    /// carry a specific name", so a value pushed there keeps the tag of the
    /// stack it came from — a fossil rather than a location. Tagging it with
    /// the workbench's own name would erase that.
    fn push_as(&mut self, v: BundValue, sym: bund2_value::StackSym, stack_name: &Rc<str>) {
        if let Some(cap) = self.cap
            && self.items.len() >= cap
        {
            self.items.pop_back();
        }
        // **D41.** A scalar takes the symbol inline — no allocation, no
        // boxing. A heap value already has a header, so its tag goes in the
        // map as it always did.
        let tagged = match v {
            BundValue::Heap(_) => v.with_tag(Rc::clone(&self.tag_key), Rc::clone(stack_name)),
            scalar => scalar.with_stack_sym(sym),
        };
        self.items.push_back(tagged);
    }

    fn pull(&mut self) -> Option<BundValue> {
        self.items.pop_back()
    }

    /// Bottom-first, which is the order the reference's box draws in.
    pub fn contents(&self) -> Vec<BundValue> {
        self.items.iter().cloned().collect()
    }

    /// **`pull` does not honour the policy and is right not to.** The
    /// reference's `pull` always pops the back, with the FIFO branch commented
    /// out (`reference/rust_multistack/src/stack_pull.rs:9-13`) — and that is
    /// correct, because pushing at the opposite end is what makes a queue. The
    /// error F27 records is in `peek`, which *does* branch and would disagree
    /// with `pull` on a FIFO stack.
    pub fn peek(&self) -> Option<&BundValue> {
        self.items.back()
    }

    /// The value `n` places below the top, without copying the stack.
    ///
    /// `n == 0` is the top, so this is `peek` generalised. It exists for
    /// fragment guards (RFC-0005 §S5), which ask about the top *few* values:
    /// the obvious spelling — `snapshot()` and index — clones every value on
    /// the stack to look at two of them, which is `O(depth)` allocation on the
    /// path whose whole purpose is to be cheaper than a call.
    pub fn peek_at(&self, n: usize) -> Option<&BundValue> {
        let len = self.items.len();
        if n >= len {
            return None;
        }
        self.items.get(len - 1 - n)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `stacks_left`'s per-stack cousin. Rotation is why this is a `VecDeque`.
    pub fn rotate_left(&mut self) {
        if !self.items.is_empty() {
            self.items.rotate_left(1);
        }
    }

    pub fn rotate_right(&mut self) {
        if !self.items.is_empty() {
            self.items.rotate_right(1);
        }
    }
}

/// The stack of stacks, plus the workbench.
#[derive(Debug)]
pub struct Stacks {
    stacks: BTreeMap<Rc<str>, Stack>,
    /// Names in rotation order. The reference's stack-of-stacks is itself a
    /// circular buffer, and selecting a named stack **rotates it to the top**
    /// (`…/Introduction_the_art_of_stack_operations.typ:43`).
    order: VecDeque<Rc<str>>,
    /// "a circular stack that … does not carry a specific name" (`:72`).
    workbench: Stack,
}

impl Default for Stacks {
    fn default() -> Self {
        let mut stacks = BTreeMap::new();
        let main: Rc<str> = Rc::from("main");
        stacks.insert(Rc::clone(&main), Stack::from_rc(Rc::clone(&main)));
        Self {
            stacks,
            order: VecDeque::from([main]),
            workbench: Stack::named("main"),
        }
    }
}

impl Stacks {
    pub fn current_name(&self) -> &str {
        self.order.front().map(|n| &**n).unwrap_or("main")
    }

    /// The current stack, creating it if it has somehow gone missing.
    ///
    /// The invariant is that a current stack always exists, and asserting it
    /// put a panic on the hottest path in the interpreter. Creating an empty
    /// one instead is indistinguishable in every reachable case and cannot
    /// abort the program in an unreachable one.
    fn current_mut(&mut self) -> &mut Stack {
        // **No allocation.** Cloning the front `Rc` is a refcount bump; this
        // used to be `current_name().to_string()`, which allocated on every
        // push and every pull.
        let name = match self.order.front() {
            Some(n) => Rc::clone(n),
            None => Rc::from("main"),
        };
        self.stacks
            .entry(name)
            .or_insert_with_key(|k| Stack::from_rc(Rc::clone(k)))
    }

    /// Put a newly created stack in the ring as the current one — **F89**.
    ///
    /// The reference appends a new stack to the back of its deque
    /// (`reference/rust_multistack/src/ts_add.rs:14`) and reads the current
    /// stack from the back (`reference/rust_multistack/src/ts_current.rs:7`),
    /// so creating a stack makes it current. This deque keeps the current
    /// stack at the front and the others after it in the reference's cyclic
    /// order. Mirrored, the reference's append is two moves: the old current
    /// goes from the front to the back, and the new name goes on the front.
    /// A bare `push_front` gets the current stack right and the ring wrong, and
    /// `stacks_right` then reaches a different stack than the reference's.
    fn add_as_current(&mut self, name: Rc<str>) {
        if !self.order.is_empty() {
            self.order.rotate_left(1);
        }
        self.order.push_front(name);
    }

    /// Make a named stack current, creating it if needed.
    ///
    /// Rotates rather than reassigns, matching the guide: "when positioning a
    /// named stack to become the current stack, the buffer rotates to bring
    /// the required stack to the proper position".
    pub fn to_stack(&mut self, name: &str) {
        self.stacks
            .entry(Rc::from(name))
            .or_insert_with_key(|k| Stack::from_rc(Rc::clone(k)));
        if !self.order.iter().any(|n| &**n == name) {
            self.add_as_current(Rc::from(name));
            return;
        }
        // **Bounded by the deque's length, so termination is structural.**
        //
        // The membership test above already guarantees `name` is present, so
        // an unbounded `while current_name() != name` would in fact stop. But
        // that argument lives ten lines from the loop, and F70 is what happens
        // when a loop's termination depends on a fact established elsewhere:
        // the reference's drain is correct exactly until pushing to a stack
        // creates it, and then it never returns.
        //
        // The reference guards the same rotation by counting a full circle and
        // failing — "We made a full circle over stacks and did not find {}"
        // (`reference/rust_multistack/src/ts_to_current.rs:24-26`). This bounds
        // it instead: at most one full rotation, after which the deque is back
        // where it started and nothing has been lost.
        for _ in 0..self.order.len() {
            if self.current_name() == name {
                break;
            }
            self.order.rotate_left(1);
        }
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.stacks.keys().map(|k| &**k)
    }

    pub fn workbench(&mut self) -> &mut Stack {
        &mut self.workbench
    }
}

// --- the machine-stack floor: RFC-0005 §S8, F85 ------------------------------

thread_local! {
    /// The machine stack this thread runs on, if whoever started the thread
    /// said so: `(top, size)` in bytes. `bund2`'s evaluation thread declares
    /// it; a thread that has not is treated conservatively (see
    /// [`ASSUMED_BUDGET`]).
    static STACK_REGION: std::cell::Cell<Option<(usize, usize)>> =
        const { std::cell::Cell::new(None) };
}

/// Bytes kept below the Tier 0 floor, for the frames between one check and
/// the next. Reporting the error needs none of it: by the time it is
/// reported, the stack has unwound.
pub const STACK_RESERVE: usize = 256 * 1024;

/// How far below its construction point an `Interp` assumes it may recurse,
/// on a thread whose stack nobody declared: half of Rust's default for a
/// spawned thread, 2 MiB, because the constructor is rarely at the very top.
const ASSUMED_BUDGET: usize = 1024 * 1024;

/// Declare the stack this thread runs on — its top, as [`stack_marker`] read
/// in the thread's entry function, and its size in bytes. Every `Interp` built
/// on this thread afterwards takes its floor from it. RFC-0005 §S8.
pub fn set_stack_region(top: usize, size: usize) {
    STACK_REGION.with(|r| r.set(Some((top, size))));
}

/// The address of a local in this call's frame, as an integer: the stack
/// pointer, near enough, taken in safe Rust. Nothing ever dereferences it.
/// Stacks grow downward on every target Bund2 builds for, so a smaller value
/// is deeper.
#[inline(never)]
pub fn stack_marker() -> usize {
    let marker = 0u8;
    std::ptr::addr_of!(marker) as usize
}

/// The Tier 0 floor for an `Interp` built on the current thread.
fn tier0_floor() -> usize {
    match STACK_REGION.with(std::cell::Cell::get) {
        Some((top, size)) => top.saturating_sub(size).saturating_add(STACK_RESERVE),
        None => stack_marker()
            .saturating_sub(ASSUMED_BUDGET)
            .saturating_add(STACK_RESERVE),
    }
}

/// The items a frame's body value carries: a LAMBDA's body, or a LIST's
/// elements for a body assembled at run time (`scoped_call`'s).
fn frame_items(v: &BundValue) -> Option<&[BundValue]> {
    v.as_lambda().or_else(|| v.as_list())
}

/// The interpreter.
/// One body being executed, and how far through it we are.
///
/// **RFC-0003 §S4.** A frame carries the body, an instruction pointer, and an
/// optional exit action that runs when the frame leaves — however it leaves.
/// That last part is what fixes F57: the reference restores a context's stack
/// with a statement placed after three early returns, so a failure skips it.
struct Frame {
    /// The value whose body this frame runs — a LAMBDA, or a LIST for a body
    /// assembled at run time. **Held, not copied (D42)**, so the body's `Rc`,
    /// D35's cache key, is in hand for as long as the frame runs, and a body is
    /// alive whenever anything is executing it.
    body: BundValue,
    ip: usize,
    /// Run when this frame is popped, on success **and** on failure.
    exit: Option<ExitAction>,
}

/// What a frame does on the way out.
enum ExitAction {
    /// Return to a named stack. `( … )` and `context` both need this.
    ToStack(String),
}

pub struct Interp {
    /// Contexts opened by `( … )` and not yet closed, each with the stack to
    /// restore. **Separate from the stack-of-stacks on purpose** — the
    /// reference conflates the two and so cannot tell whether a context is
    /// open, which is F60.
    contexts: Vec<(String, String)>,
    /// Where diagnostics go. Silent by default, so a `Vm` built in a test
    /// writes to nobody's terminal; the CLI swaps in a text reporter and a TUI
    /// would swap in its own.
    pub reporter: Box<dyn bund2_api::diag::Reporter>,
    /// **The frame stack — RFC-0003 §S4.** Bund call depth lives here, on the
    /// heap, instead of on the Rust stack.
    frames: Vec<Frame>,
    /// A body a native asked the loop to run **after it returns** — §S4a's
    /// request, in its tail-position form. The native sets it and returns; the
    /// loop pushes a frame. Nothing recurses.
    pending_tail: Option<BundValue>,
    /// **Where bodies start running, by key — RFC-0005 criterion 20.** `None`
    /// in production, so the cost is one branch per body entry. When `Some`,
    /// every frame pushed for a body records that body's `payload_key`, which
    /// is how a test shows one key reaching every iteration of a loop (D42).
    pub entry_log: Option<Vec<usize>>,
    /// **The effect audit — RFC-0005 criterion 24.** `None` in production,
    /// so the cost is one branch per native call. When `Some`, every native
    /// that declares a fixed effect is held to it while it runs: it may not
    /// start a body, file a tail request or dispatch another word, and when it
    /// returns `Ok` on the stack it started on, that stack's depth must have
    /// moved by exactly what it declares. Each breach is recorded as a
    /// sentence. RFC-0005 §S5 keeps values in registers across such a native,
    /// so every breach is a value the native could not have seen.
    pub effect_audit: Option<Vec<String>>,
    /// The fixed-effect native running now. Only ever `Some` while the audit
    /// is on, so the checks that read it cost production one branch.
    audit_inside: Option<Symbol>,
    /// The innermost native running now, whatever its effect. Only ever `Some`
    /// while the audit is on. A report at `Error` severity made while one runs
    /// is a breach: RFC-0005 criterion 25's run-time half.
    audit_native: Option<Symbol>,
    /// **The Tier 0 floor — RFC-0005 §S8, F85.** The lowest stack address at
    /// which a native may still re-enter evaluation. Below it,
    /// `Vm::eval_lambda`, `Vm::apply` and `Vm::scoped_call` refuse with
    /// [`Error::stack_exhausted`] instead of nesting until the process aborts.
    stack_floor: usize,
    pub registry: Registry,
    pub stacks: Stacks,
    /// `apply` tests this in three places, and it does **not** precede the
    /// command check — `is_command` returns at
    /// `reference/rust_multistackvm/src/multistackvm_apply.rs:17`, before the
    /// `autoadd` test at `:19`.
    pub autoadd: bool,
    /// The code `bund.exit` asked to end with — D52. Once set, every step
    /// refuses, so whatever is running unwinds, and the top level returns
    /// cleanly instead of reporting.
    exit_code: Option<i32>,
    /// **D55's observation audit.** While the effect audit holds a
    /// fixed-effect native, every read beyond its operands — the whole stack,
    /// the whole workbench, a stack's depth by name — is recorded here as a
    /// sentence. Empty in production, where `audit_inside`
    /// is never set; criterion 28's palette reads it.
    pub observations: std::cell::RefCell<Vec<String>>,
}

impl Default for Interp {
    fn default() -> Self {
        Self::new()
    }
}

impl Interp {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
            stacks: Stacks::default(),
            autoadd: false,
            contexts: Vec::new(),
            reporter: Box::new(bund2_api::diag::SilentReporter),
            frames: Vec::new(),
            pending_tail: None,
            entry_log: None,
            effect_audit: None,
            audit_inside: None,
            audit_native: None,
            stack_floor: tier0_floor(),
            exit_code: None,
            observations: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Record, for D55, that the fixed-effect native running under the audit
    /// read beyond its operands, and how. Outside the audit this is a single
    /// branch on `audit_inside`, which production never sets.
    fn note_observation(&self, how: &str) {
        let Some(who) = self.audit_inside else {
            return;
        };
        if let Ok(mut log) = self.observations.try_borrow_mut() {
            log.push(format!("`{}` {how}", self.registry.interner.name(who)));
        }
    }

    /// Refuse to take another step once an exit has been requested (D52).
    ///
    /// The refusal is an error only so that it unwinds: every native between
    /// here and the top level passes it up, including one that catches errors,
    /// because that native's next step is refused too. The top level recognises
    /// it by the request, not by the text, and returns cleanly.
    fn exit_gate(&self) -> Result<(), Error> {
        match self.exit_code {
            Some(code) => Err(Error(format!("the program asked to exit with code {code}"))),
            None => Ok(()),
        }
    }

    /// Is there room above the Tier 0 floor to re-enter evaluation?
    fn stack_ok(&self) -> bool {
        stack_marker() > self.stack_floor
    }

    /// Call a native, holding it to its declared effect when the audit is on
    /// ([`Interp::effect_audit`]).
    ///
    /// An opaque native is not held to anything, and the words it reaches are
    /// audited on their own. A fixed-effect native is marked as running, so
    /// that a body, a tail request or a dispatch it causes is recorded, and its
    /// depth change is compared with its declaration. The comparison is skipped
    /// when the native fails, or leaves a different stack current: a switch is
    /// §S5's epoch, not a depth.
    fn call_native(&mut self, name: Symbol, n: bund2_api::Native) -> Result<(), Error> {
        if self.effect_audit.is_none() {
            return self.invoke(name, n);
        }
        if let Some(outer) = self.audit_inside {
            let callee = self.registry.interner.name(name).to_string();
            self.audit_breach(outer, &format!("declares a fixed effect and dispatched `{callee}`"));
        }
        let outer_native = self.audit_native.replace(name);
        let r = self.call_audited(name, n);
        self.audit_native = outer_native;
        r
    }

    /// [`Interp::call_native`]'s audited half: everything but tracking which
    /// native is innermost.
    fn call_audited(&mut self, name: Symbol, n: bund2_api::Native) -> Result<(), Error> {
        let outer = self.audit_inside.take();
        if n.effect.opaque {
            let r = self.invoke(name, n);
            self.audit_inside = outer;
            return r;
        }
        // **The current stack only.** `StackEffect` has one axis, and RFC-0004
        // §S1 says what it counts: the main stack, with the workbench axis not
        // yet built. It is also the only count §S5 relies on, since promotion
        // holds current-stack values and models the depth after a call from
        // this pair. The workbench delta is reported beside it, as evidence.
        // The interpreter's own reads go to the stacks directly, so they are
        // never taken for the native's (D55).
        let stack = self.stacks.current_name().to_string();
        let (main0, wb0) = (self.depth(), self.stacks.workbench.len());
        self.audit_inside = Some(name);
        let r = self.invoke(name, n);
        self.audit_inside = outer;
        if r.is_ok() && self.stacks.current_name() == stack {
            let (main1, wb1) = (self.depth(), self.stacks.workbench.len());
            let (c, p) = (usize::from(n.effect.consumes), usize::from(n.effect.produces));
            if main0.checked_sub(c).map(|d| d + p) != Some(main1) {
                self.audit_breach(
                    name,
                    &format!(
                        "declares a fixed effect and declares ({c}, {p}) and moved `{stack}` from {main0} to {main1} and the workbench from {wb0} to {wb1}"
                    ),
                );
            }
        }
        r
    }

    /// Run a native's function: **the one place Tier 0 calls one.**
    ///
    /// - A panic becomes `Error::internal` naming the native (D49), where it
    ///   used to unwind out of the evaluation thread and end the process with
    ///   exit 1 (F95).
    /// - A native that fails leaves no tail request behind (F96). A request it
    ///   filed before failing would otherwise wait in `pending_tail` and run at
    ///   the next `take_pending`, after `?try` had caught the error, as a body
    ///   nobody asked for.
    fn invoke(&mut self, name: Symbol, n: bund2_api::Native) -> Result<(), Error> {
        let r = match bund2_api::catch_panic(|| (n.f)(&mut *self)) {
            Ok(r) => r,
            Err(msg) => Err(bund2_api::panicked(
                &format!("native `{}`", self.registry.interner.name(name)),
                &msg,
            )),
        };
        if r.is_err() {
            self.pending_tail = None;
        }
        r
    }

    /// Record one breach, while the audit is on: `who` did `what`.
    fn audit_breach(&mut self, who: Symbol, what: &str) {
        let line = format!("`{}` {what}", self.registry.interner.name(who));
        if let Some(log) = self.effect_audit.as_mut() {
            log.push(line);
        }
    }

    /// Dispatch a call.
    ///
    /// The order is the reference's, from
    /// `reference/rust_multistackvm/src/multistackvm_apply.rs:9-62`: command
    /// first and returning immediately, then the `autoadd` branch, then the
    /// sigil deciding whether `lambda` is consulted, then `native`.
    ///
    /// **Takes a `Symbol`, not a string.** That is the whole point of
    /// RFC-0002: the reference allocates thirteen strings and hashes eight
    /// times to dispatch `dup`, and none of it carries information a caller
    /// could not have resolved once.
    pub fn dispatch(&mut self, s: Symbol, sigil: bool) -> Result<(), Error> {
        match self.registry.resolve(s, sigil) {
            Resolved::Command => {
                // `resolve` said the slot holds a command; if the binding has
                // gone in between, that is a broken invariant and not the
                // program's fault, so it is reported rather than asserted.
                let n = self
                    .registry
                    .slot(s)
                    .and_then(|sl| sl.command)
                    .ok_or_else(|| {
                        Error::internal(format!(
                            "`{}` resolved to a command whose binding is absent",
                            self.registry.interner.name(s)
                        ))
                    })?;
                self.call_native(s, n)
            }
            _ if self.autoadd => {
                // `apply` appends the name to the value beneath it rather than
                // executing (`:20-27`). The name is what a `CALL` carries, so
                // this needs the interner — which is why `autoadd` is a
                // dispatch concern and not a resolution one.
                let name = self.registry.interner.name(s).to_string();
                let Some(beneath) = self.pull() else {
                    return Err(Error("Autoadd found no working data on stack".into()));
                };
                self.push(beneath);
                self.push(BundValue::call(name));
                Ok(())
            }
            Resolved::Lambda => {
                // `lambda_eval` applies each element of the body
                // (`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:13-14`).
                //
                // The reference recurses in Rust here, so its Bund call depth
                // is Rust call depth. Bund2 does not: the body is cloned out,
                // so the registry is not borrowed, and handed to the loop as a
                // tail request below (RFC-0003 §S4's frame loop).
                let target = self.registry.resolve_target(s);
                let body = self
                    .registry
                    .slot(target)
                    .and_then(|slot| slot.lambda.clone())
                    .ok_or_else(|| Error("resolved to a lambda that is not there".into()))?;
                // `lambda_eval` bails on a non-LAMBDA
                // (`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:27-29`).
                // The `register` word guards the type, but the registry API
                // does not, so the check belongs here too.
                if body.as_lambda().is_none() {
                    return Err(Error("This is not a lambda".into()));
                }
                // **The call that used to recurse.** Calling a lambda is a
                // tail position: nothing in `dispatch` runs after the body. So
                // it becomes a request, and the loop pushes a frame — Bund
                // call depth stops being Rust call depth.
                self.request_tail(body);
                Ok(())
            }
            Resolved::Native => {
                let target = self.registry.resolve_target(s);
                let n = self
                    .registry
                    .slot(target)
                    .and_then(|sl| sl.native)
                    .ok_or_else(|| {
                        Error::internal(format!(
                            "`{}` resolved to a native whose binding is absent",
                            self.registry.interner.name(target)
                        ))
                    })?;
                self.call_native(target, n)
            }
            Resolved::Unbound => Err(Error(format!(
                "{} not registered",
                self.registry.interner.name(s)
            ))),
        }
    }

    /// Dispatch by name, as `execute` does with a string off the stack.
    ///
    /// **A miss must not intern.** D16 makes a computed name expressible, and
    /// interning every lookup would grow memory without bound on a program
    /// that dispatches a miss in a loop — which the reference, whose tables
    /// are pure reads, does not do.
    pub fn dispatch_name(&mut self, name: &str) -> Result<(), Error> {
        match self.registry.interner.lookup_call(name) {
            Some((s, sigil)) => self.dispatch(s, sigil),
            None => Err(Error(format!("{name} not registered"))),
        }
    }

    /// Apply one value, as `VM::apply` does
    /// (`reference/rust_multistackvm/src/multistackvm_apply.rs:8-104`).
    ///
    /// Three arms, and the third is everything else:
    ///
    /// - **CALL** — dispatch. RFC-0002's registry already folds in the command,
    ///   sigil, alias and lambda ordering the reference spells out at `:16-59`.
    /// - **CONTEXT** — switch stacks (`:69-87`). `( … )` opens one this way and
    ///   `endcontext` closes it.
    /// - **anything else** — push (`:99`).
    ///
    /// `autoadd` is not implemented, so the branches at `:19` and `:89` are
    /// absent. When list construction lands it belongs here, not at the call
    /// sites.
    /// Apply one value **synchronously**: whatever it asks for has run by the
    /// time this returns.
    ///
    /// This is what a native gets through the `Vm` trait, because a native that
    /// applies a value and then inspects the stack must see the result. The
    /// evaluation loop uses [`Interp::apply_step`] instead, which leaves the
    /// request for the loop to fulfil — that is the difference between one Rust
    /// frame per Bund call and none.
    pub fn apply(&mut self, v: BundValue) -> Result<(), Error> {
        // A native re-entering evaluation spends Rust stack (RFC-0005 §S8).
        if !self.stack_ok() {
            return Err(Error::stack_exhausted());
        }
        let floor = self.frames.len();
        self.apply_step(v)?;
        self.take_pending();
        self.run_to(floor)?;
        // F112: a synchronous run that ended by requesting an exit returns
        // the refusal, not `Ok`, so the native that asked runs nothing more.
        self.exit_gate()
    }

    /// Apply one value, leaving any requested body for the caller's loop.
    pub fn apply_step(&mut self, v: BundValue) -> Result<(), Error> {
        self.exit_gate()?;
        match v.dt() {
            bund2_value::CALL => {
                let name = v
                    .as_str()
                    .ok_or_else(|| Error("Empty function name passed for CALL".into()))?;
                if name.is_empty() {
                    return Err(Error("Empty function name passed for CALL".into()));
                }
                self.dispatch_name(&name)
            }
            bund2_value::CONTEXT => {
                let name = v.as_str().ok_or_else(|| {
                    Error("Can not get the name of context from the CONTEXT value".into())
                })?;
                // `VM::to_stack` switches *and* pushes the name onto the
                // nesting stack (`reference/rust_multistackvm/src/multistackvm_to_stack.rs:5-19`),
                // which is what `endcontext` later pops. Every switch counts,
                // `@name` included — the difference from the reference is only
                // that this stack starts **empty** rather than holding `main`,
                // so a bare `endcontext` has nothing to pop and F60's guard can
                // finally fire.
                self.push_context(&name);
                self.to_stack(&name);
                Ok(())
            }
            _ => {
                self.push(v);
                Ok(())
            }
        }
    }

    /// **The one evaluator** — RFC-0003 §S6.
    ///
    /// The reference has three copies of this loop, two identical and one that
    /// prints and steps (F52): `reference/bundcore/src/bundcore_eval.rs:7-45`,
    /// `reference/Bund/src/stdlib/helpers/eval.rs:7-41`, and the debugger's at
    /// `reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:52-95`.
    /// This is all three, with the difference between them passed in as an
    /// observer rather than duplicated.
    ///
    /// The four arms are the reference's: `NONE` continues, `EXIT` breaks,
    /// `ERROR` bails, everything else applies. The error text on the last arm
    /// is preserved verbatim in *shape*; the interpolated value cannot
    /// reproduce, because `{:?}` embeds `id` and `stamp` (F14).
    pub fn eval(&mut self, stream: &[BundValue]) -> Result<(), Error> {
        self.eval_observed(stream, &mut |_| {})
            .map_err(|(i, e)| {
                // The reference wraps the failure with the value that caused
                // it (`reference/bundcore/src/bundcore_eval.rs:33`), which is
                // how it says *where* — it has no source positions. Bund2 has
                // spans, so `eval_indexed` hands back the index and the bare
                // reason and lets the caller say where properly. This wrapper
                // survives only for callers that want the reference's text.
                let v = stream.get(i).map(|v| v.render(false)).unwrap_or_default();
                Error(format!(
                    "Attempt to evaluate value {v} returned error: {}",
                    e.0
                ))
            })
    }

    /// [`Interp::eval`], reporting **which value** failed.
    ///
    /// The index is into the stream the caller lowered, so a caller holding
    /// `Lowered` can turn it into a source position. That is the whole of
    /// RFC-0003 §S5 that a flat `Vec<BundValue>` can support: top-level
    /// positions, not positions inside a lambda body.
    pub fn eval_indexed(&mut self, stream: &[BundValue]) -> Result<(), (usize, Error)> {
        self.eval_observed(stream, &mut |_| {})
    }

    /// [`Interp::eval`] with a per-word observer. The debugger's copy is this
    /// with a printing, stepping observer.
    pub fn eval_observed(
        &mut self,
        stream: &[BundValue],
        observe: &mut dyn FnMut(&BundValue),
    ) -> Result<(), (usize, Error)> {
        for (i, v) in stream.iter().enumerate() {
            match v.dt() {
                bund2_value::NONE => continue,
                bund2_value::EXIT => break,
                _ => {
                    observe(v);
                    // A step refused because the program asked to exit is not
                    // a failure: the program is over, and there is nothing to
                    // report (D52).
                    if let Err(e) = self.apply_step(v.clone()) {
                        if self.exit_code.is_some() {
                            return Ok(());
                        }
                        return Err((i, e));
                    }
                    // A top-level word may have asked for a body to run.
                    if let Err(e) = self.drain_frames() {
                        if self.exit_code.is_some() {
                            return Ok(());
                        }
                        return Err((i, e));
                    }
                }
            }
        }
        Ok(())
    }

    /// Ask the loop to run `body` **after the current native returns**.
    ///
    /// This is §S4a's request mechanism in the shape that covers tail
    /// positions — which is most of them: a lambda call, `if`'s branch, a
    /// `through` conditional's body, `execute` on a LAMBDA. The native sets the
    /// request and returns; the loop pushes a frame; **no Rust frame is added
    /// per Bund call**, which is the whole point.
    ///
    /// Natives that must inspect the stack *between* two evaluations —
    /// `?ifthenelse`, `?try`, `times` — still call [`Vm::eval_lambda`], which
    /// is synchronous. Those add one Rust frame per *native*, not per Bund
    /// call, so depth is bounded by how deeply such natives nest rather than by
    /// how deep the program recurses.
    pub fn request_tail(&mut self, body: BundValue) {
        if let Some(who) = self.audit_inside {
            self.audit_breach(who, "declares a fixed effect and filed a tail request");
        }
        self.pending_tail = Some(body);
    }

    /// Push a frame for whatever the last native requested, if anything.
    fn take_pending(&mut self) {
        if let Some(body) = self.pending_tail.take() {
            self.push_frame(body, None);
        }
    }

    /// Push a frame for `body` — **the one place a body starts running**, so
    /// the one place its key is observed (D42, RFC-0005 criterion 20).
    fn push_frame(&mut self, body: BundValue, exit: Option<ExitAction>) {
        if let Some(who) = self.audit_inside {
            self.audit_breach(who, "declares a fixed effect and started a body");
        }
        if let (Some(log), Some(k)) = (self.entry_log.as_mut(), body.payload_key()) {
            log.push(k);
        }
        self.frames.push(Frame { body, ip: 0, exit });
    }

    /// Pop frames down to `floor`, running every exit action on the way. This
    /// is what the reference cannot do: its restore is a statement after the
    /// early returns.
    fn unwind_to(&mut self, floor: usize) {
        while self.frames.len() > floor {
            if let Some(Frame {
                exit: Some(action), ..
            }) = self.frames.pop()
            {
                self.run_exit(action);
            }
        }
    }

    /// Run frames until the stack returns to `floor`.
    ///
    /// The loop is flat: a body that calls a body pushes rather than recurses,
    /// so 100,000 Bund calls cost 100,000 heap frames and one Rust frame.
    fn run_to(&mut self, floor: usize) -> Result<(), Error> {
        while self.frames.len() > floor {
            let Some(frame) = self.frames.last_mut() else {
                break;
            };
            // Read through the held value (D42). `get`, not an index: a frame
            // whose value carries no body is a broken invariant, not a panic.
            let next = frame_items(&frame.body).map(|items| items.get(frame.ip).cloned());
            let v = match next {
                Some(Some(v)) => {
                    frame.ip += 1;
                    v
                }
                Some(None) => {
                    let done = self.frames.pop();
                    if let Some(Frame {
                        exit: Some(action), ..
                    }) = done
                    {
                        self.run_exit(action);
                    }
                    continue;
                }
                None => {
                    self.unwind_to(floor);
                    return Err(Error::internal(
                        "a frame's body is neither a LAMBDA nor a LIST value",
                    ));
                }
            };
            match self.apply_step(v) {
                Ok(()) => self.take_pending(),
                Err(e) => {
                    self.unwind_to(floor);
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn run_exit(&mut self, action: ExitAction) {
        match action {
            ExitAction::ToStack(name) => self.to_stack(&name),
        }
    }

    /// Drive any frame the top level's last word requested.
    fn drain_frames(&mut self) -> Result<(), Error> {
        self.take_pending();
        self.run_to(0)
    }
}

impl Vm for Interp {
    fn push(&mut self, v: BundValue) {
        self.stacks.current_mut().push(v);
    }

    fn pull(&mut self) -> Option<BundValue> {
        self.stacks.current_mut().pull()
    }

    fn depth(&self) -> usize {
        // Not through `depth_of`: a depth guard reads only as far as its
        // operands, which D55's differential checks, and must not be taken
        // for a read of a stack by name.
        self.stacks
            .stacks
            .get(self.stacks.current_name())
            .map(Stack::len)
            .unwrap_or(0)
    }

    fn peek(&self) -> Option<BundValue> {
        self.stacks
            .stacks
            .get(self.stacks.current_name())
            .and_then(|s| s.peek().cloned())
    }

    fn peek_at(&self, n: usize) -> Option<BundValue> {
        self.stacks
            .stacks
            .get(self.stacks.current_name())
            .and_then(|s| s.peek_at(n).cloned())
    }

    fn clear(&mut self) {
        let name = self.stacks.current_name().to_string();
        self.clear_stack(&name);
    }

    fn snapshot(&self) -> Vec<BundValue> {
        self.note_observation("reads the whole current stack");
        self.stacks
            .stacks
            .get(self.stacks.current_name())
            .map(Stack::contents)
            .unwrap_or_default()
    }

    fn snapshot_workbench(&self) -> Vec<BundValue> {
        self.note_observation("reads the whole workbench");
        self.stacks.workbench.contents()
    }

    fn rotate_left(&mut self) {
        self.stacks.current_mut().rotate_left();
    }

    fn rotate_right(&mut self) {
        self.stacks.current_mut().rotate_right();
    }

    fn current_name(&self) -> String {
        // Not observed: the name is not a value promotion holds, so asking for
        // it reads nothing a compiled caller might be keeping in registers. A
        // word that then *acts* on the current stack by name changes what it
        // answers when the values beneath change, which D55's differential
        // catches (F111 is what it caught).
        self.stacks.current_name().to_string()
    }

    fn to_stack(&mut self, name: &str) {
        self.stacks.to_stack(name);
    }

    fn stack_exists(&self, name: &str) -> bool {
        self.stacks.stacks.contains_key(name)
    }

    /// **A stack this creates becomes current — F89.** The reference creates
    /// through `add_named_stack`, which appends to the back of its deque
    /// (`reference/rust_multistack/src/ts_add.rs:14`), and its current stack
    /// *is* the back (`reference/rust_multistack/src/ts_current.rs:7`). A new
    /// name goes in through `Stacks::add_as_current`, as `Stacks::to_stack`
    /// puts one. A name that already exists is left where it is, as the
    /// reference leaves it (`reference/rust_multistack/src/ts_ensure.rs:10-15`).
    /// Every named push creates through here, as the reference's
    /// `push_to_stack` does (`reference/rust_multistack/src/ts_push.rs:46`),
    /// and F70 is the same mechanism seen through `move`.
    fn ensure_stack(&mut self, name: &str) {
        self.stacks
            .stacks
            .entry(Rc::from(name))
            .or_insert_with_key(|k| Stack::from_rc(Rc::clone(k)));
        if !self.stacks.order.iter().any(|n| &**n == name) {
            self.stacks.add_as_current(Rc::from(name));
        }
    }

    fn ensure_stack_with_capacity(&mut self, name: &str, cap: usize) {
        self.ensure_stack(name);
        // The reference only records a capacity the first time
        // (`reference/rust_multistack/src/ts_ensure.rs:20-22` inserts into
        // `stack_cap` only when absent), so a second call does not resize.
        if let Some(s) = self.stacks.stacks.get_mut(name)
            && s.cap.is_none()
        {
            s.cap = Some(cap);
        }
    }

    fn depth_of(&self, name: &str) -> usize {
        self.note_observation("reads the depth of a stack by name");
        self.stacks.stacks.get(name).map(Stack::len).unwrap_or(0)
    }

    fn push_to(&mut self, name: &str, v: BundValue) {
        self.ensure_stack(name);
        if let Some(s) = self.stacks.stacks.get_mut(name) {
            s.push(v);
        }
    }

    fn pull_from(&mut self, name: &str) -> Option<BundValue> {
        self.stacks.stacks.get_mut(name).and_then(Stack::pull)
    }

    fn clear_stack(&mut self, name: &str) {
        if let Some(s) = self.stacks.stacks.get_mut(name) {
            s.items.clear();
        }
    }

    fn drop_stack(&mut self, name: &str) {
        self.stacks.stacks.remove(name);
        self.stacks.order.retain(|n| &**n != name);
        if self.stacks.order.is_empty() {
            let main: Rc<str> = Rc::from("main");
            self.stacks.order.push_back(Rc::clone(&main));
            self.stacks
                .stacks
                .entry(main)
                .or_insert_with_key(|k| Stack::from_rc(Rc::clone(k)));
        }
    }

    fn rotate_stacks_left(&mut self) {
        if !self.stacks.order.is_empty() {
            self.stacks.order.rotate_left(1);
        }
    }

    fn rotate_stacks_right(&mut self) {
        if !self.stacks.order.is_empty() {
            self.stacks.order.rotate_right(1);
        }
    }

    fn apply(&mut self, v: BundValue) -> Result<(), Error> {
        Interp::apply(self, v)
    }

    /// Run a body **now**, for a native that must inspect the stack after it.
    ///
    /// Synchronous, but flat inside: it pushes a frame and drives the loop back
    /// down to its own floor, so however deeply the body recurses it costs one
    /// Rust frame — this one. `?ifthenelse` and `?try` need this shape because
    /// they act on what the body left behind; `if` does not, and uses
    /// [`Interp::request_tail`] instead.
    fn tail_lambda(&mut self, lambda: BundValue) {
        Interp::request_tail(self, lambda);
    }

    fn scoped_call(&mut self, stack: &str, body: Vec<BundValue>) -> Result<(), Error> {
        if !self.stack_ok() {
            return Err(Error::stack_exhausted());
        }
        let prev = self.stacks.current_name().to_string();
        let floor = self.frames.len();
        self.to_stack(stack);
        // The exit action is carried by the frame, so the unwinder runs it on a
        // failure just as the loop runs it on success. F57. The body is
        // assembled per call, so it is wrapped as a LIST value: there is no
        // key worth keeping (D42).
        self.push_frame(BundValue::list(body), Some(ExitAction::ToStack(prev)));
        self.run_to(floor)?;
        // F112, as in `Interp::apply`.
        self.exit_gate()
    }

    fn eval_lambda(&mut self, lambda: &BundValue) -> Result<(), Error> {
        self.exit_gate()?;
        if frame_items(lambda).is_none() {
            return Err(Error::internal(
                "eval_lambda was handed a value that carries no body; every caller checks the LAMBDA tag first",
            ));
        }
        // **F85's fix.** Every native that runs a body synchronously comes
        // through here, and each such level spends Rust stack. Below the floor,
        // refuse rather than nest until the process aborts (RFC-0005 §S8).
        if !self.stack_ok() {
            return Err(Error::stack_exhausted());
        }
        let floor = self.frames.len();
        self.push_frame(lambda.clone(), None);
        // **F112.** `run_to` pops a finished frame without the gate, so a body
        // whose last word is `bund.exit` returns `Ok`, and the native that ran
        // it went on: `map` collected, `input*` read another line. D52 says
        // nothing more runs, so the refusal comes here, at the return to Rust,
        // as it would have at the body's next step.
        self.run_to(floor)
            .and_then(|()| self.exit_gate())
            .map_err(|e| e.context("Lambda content evaluation returned error: "))
    }

    fn register_lambda(&mut self, name: &str, body: BundValue) {
        self.registry.register_lambda(name, body);
    }

    fn unregister_lambda(&mut self, name: &str) {
        if let Some((s, _)) = self.registry.interner.lookup_call(name) {
            self.registry.unregister_lambda(s);
        }
    }

    fn is_lambda(&self, name: &str) -> bool {
        self.registry
            .interner
            .lookup_call(name)
            .and_then(|(s, _)| self.registry.slot(self.registry.resolve_target(s)))
            .is_some_and(|slot| slot.lambda.is_some())
    }

    fn is_native(&self, name: &str) -> bool {
        self.registry
            .interner
            .lookup_call(name)
            .and_then(|(s, _)| self.registry.slot(self.registry.resolve_target(s)))
            .is_some_and(|slot| slot.native.is_some())
    }

    fn is_alias(&self, name: &str) -> bool {
        self.registry
            .interner
            .lookup_call(name)
            .is_some_and(|(s, _)| self.registry.resolve_target(s) != s)
    }

    fn report(&mut self, d: bund2_api::diag::Diagnostic) {
        // RFC-0005 criterion 25's run-time half: natives return errors, they
        // do not report them, because a mid-body `Error` report would take a
        // snapshot of a stack whose values promotion may be holding.
        if d.severity.is_fatal()
            && let Some(who) = self.audit_native
        {
            self.audit_breach(who, "reported at `Error` severity while it ran");
        }
        // Collect the stacks only if something will show them, for a
        // diagnostic of this severity (D45).
        let d = if self.reporter.wants_stack(d.severity) {
            // Compact by default; the raw `Debug` form only when a debug
            // session asks for it. A stack of raw renderings is ~150 columns a
            // row and tells the reader nothing they were looking for.
            let w = self.reporter.value_width();
            let raw = self.reporter.wants_raw_values();
            let fmt = |v: &BundValue| if raw { v.render(false) } else { v.summary(w) };
            // Read directly: a report's snapshot is the interpreter's, not the
            // reporting native's, and D45 already governs it (D55).
            let stack = self
                .stacks
                .stacks
                .get(self.stacks.current_name())
                .map(Stack::contents)
                .unwrap_or_default()
                .iter()
                .map(&fmt)
                .collect();
            let wb = self.stacks.workbench.contents().iter().map(&fmt).collect();
            d.on_stack(self.stacks.current_name().to_string())
                .with_stack(stack)
                .with_workbench(wb)
        } else {
            d.on_stack(self.stacks.current_name().to_string())
        };
        self.reporter.report(&d);
    }

    fn conditional(&self, ty: &str) -> Option<bund2_api::ConditionalFn> {
        self.registry.conditional(ty)
    }

    fn register_class(&mut self, name: &str, class: BundValue) {
        self.registry.register_class(name, class);
    }

    fn class(&self, name: &str) -> Option<BundValue> {
        self.registry.class(name)
    }

    fn is_class(&self, name: &str) -> bool {
        self.registry.is_class(name)
    }

    fn unregister_class(&mut self, name: &str) {
        self.registry.unregister_class(name);
    }

    fn method(&self, name: &str) -> Option<bund2_api::NativeFn> {
        self.registry.method(name)
    }

    fn is_method(&self, name: &str) -> bool {
        self.registry.is_method(name)
    }

    fn register_alias(&mut self, alias: &str, target: &str) -> bool {
        self.registry.register_alias(alias, target);
        self.registry.alias_cycles(alias)
    }

    fn unregister_alias(&mut self, alias: &str) {
        self.registry.unregister_alias(alias);
    }

    fn effect_of(&self, name: &str) -> Option<bund2_api::StackEffect> {
        self.registry.effect_of(name)
    }

    fn register_var(&mut self, name: &str, value: BundValue) {
        self.registry.register_var(name, value);
    }

    fn var(&self, name: &str) -> Option<BundValue> {
        self.registry.var(name)
    }

    fn unregister_var(&mut self, name: &str) {
        self.registry.unregister_var(name);
    }

    fn context_depth(&self) -> usize {
        self.contexts.len()
    }

    fn push_context(&mut self, name: &str) {
        // The stack to come back to is the one current *before* the switch.
        let prev = self.stacks.current_name().to_string();
        self.contexts.push((name.to_string(), prev));
    }

    fn pop_context(&mut self) -> Option<String> {
        self.contexts.pop().map(|(_, prev)| prev)
    }

    fn request_exit(&mut self, code: i32) {
        // A native that declares a fixed effect may not end the program: the
        // stop comes at the next step, after the native returned `Ok`, and a
        // compiled caller holding promoted values would not see it
        // (RFC-0005's eleventh review, B1). Only `bund.exit` asks, and it is
        // opaque; the audit makes that a property rather than an observation.
        if let Some(who) = self.audit_inside {
            self.audit_breach(who, "declares a fixed effect and requested an exit");
        }
        // The first request stands; a second, made while unwinding, does not
        // change the code.
        self.exit_code.get_or_insert(code);
    }

    fn exit_requested(&self) -> Option<i32> {
        self.exit_code
    }

    fn get_lambda(&self, name: &str) -> Option<BundValue> {
        self.registry
            .interner
            .lookup_call(name)
            .and_then(|(s, _)| self.registry.slot(self.registry.resolve_target(s)))
            .and_then(|slot| slot.lambda.clone())
    }

    fn push_workbench(&mut self, v: BundValue) {
        // **As it arrives, untagged — F90.** The workbench "does not carry a
        // specific name" (`…/Introduction_the_art_of_stack_operations.typ:72`),
        // and the reference pushes to it with no `set_tag`
        // (`reference/rust_multistack/src/ts_workbench.rs:25-28`). So a value
        // moved off a stack keeps the tag that stack gave it, a fossil rather
        // than a location, and a value made on the spot — a conversion's
        // result, a match's answer — has none and renders `tags: {}`. Tagging
        // here with the current stack gave the second kind a tag the
        // reference never writes.
        self.stacks.workbench.items.push_back(v);
    }

    fn pull_workbench(&mut self) -> Option<BundValue> {
        self.stacks.workbench.pull()
    }

    fn workbench_depth(&self) -> usize {
        self.stacks.workbench.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_api::{StackEffect, WordKind};

    fn eff() -> StackEffect {
        StackEffect::fixed(0, 0)
    }
    fn marker(vm: &mut dyn Vm) -> Result<(), Error> {
        vm.push(BundValue::int(7));
        Ok(())
    }

    fn with_native(name: &str) -> (Interp, Symbol) {
        let mut i = Interp::new();
        let s = i
            .registry
            .register_native(name, marker, eff(), WordKind::Sync);
        (i, s)
    }

    #[test]
    fn a_native_dispatches_and_reaches_the_stack() {
        let (mut i, s) = with_native("w");
        i.dispatch(s, false).expect("dispatches");
        assert_eq!(i.depth(), 1);
        assert_eq!(i.pull(), Some(BundValue::int(7)));
    }

    /// RFC-0002's central behaviour, end to end this time: the same name
    /// reaches a lambda or the native depending only on the sigil.
    #[test]
    fn the_sigil_selects_the_native_over_the_lambda() {
        let (mut i, s) = with_native("println");
        // A real lambda now, with an effect that distinguishes it from the
        // native's. An earlier version of this test bound a non-lambda and read
        // the resulting error as proof the lambda arm was reached — which
        // stopped meaning anything once that arm was implemented.
        i.registry
            .register_lambda("println", BundValue::lambda(vec![BundValue::int(1)]));
        // `dispatch` on a lambda now *requests* a frame rather than running the
        // body (§S4), so the caller drives the loop. `apply` is the
        // synchronous door; `dispatch` alone leaves the request pending.
        i.apply(BundValue::call("println"))
            .expect("plain must reach the lambda");
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(1), "the lambda ran");
        i.dispatch(s, true).expect("$name must reach the native");
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(7), "the native ran");
    }

    /// The case D16 forces: a name built at run time, never lexed.
    #[test]
    fn a_runtime_string_dispatches_with_its_sigil() {
        let (mut i, _) = with_native("println");
        i.registry.register_lambda("println", BundValue::int(1));
        i.dispatch_name("$println").expect("reaches the native");
        assert_eq!(i.pull(), Some(BundValue::int(7)));
    }

    /// A miss must not intern — a program dispatching a computed miss in a
    /// loop must not grow the table.
    #[test]
    fn dispatching_a_miss_does_not_grow_the_interner() {
        let (mut i, _) = with_native("known");
        let before = i.registry.interner.len();
        for n in 0..500 {
            assert!(i.dispatch_name(&format!("miss{n}")).is_err());
        }
        assert_eq!(i.registry.interner.len(), before);
    }

    /// Dispatch follows the alias to the target's handler, not the alias's
    /// empty slot.
    #[test]
    fn dispatch_follows_an_alias_chain() {
        let (mut i, _) = with_native("println");
        i.registry.register_alias("b2", "println");
        let a2 = i.registry.register_alias("a2", "b2");
        i.dispatch(a2, false).expect("two links");
        assert_eq!(i.pull(), Some(BundValue::int(7)));
    }

    /// `is_command` returns before `autoadd` is consulted
    /// (`apply.rs:17` against `:19`), so a command runs even with the mode on.
    #[test]
    fn a_command_runs_even_under_autoadd() {
        let mut i = Interp::new();
        let s = i
            .registry
            .register_command("c", marker, eff(), WordKind::Sync);
        i.autoadd = true;
        i.dispatch(s, false).expect("commands precede autoadd");
        assert_eq!(i.pull(), Some(BundValue::int(7)));
    }

    /// Under `autoadd` a non-command name is appended to the value beneath
    /// rather than executed (`apply.rs:20-27`).
    /// **RFC-0005 §S8's Tier 0 floor, driven directly.** With the floor set
    /// above the current stack pointer, re-entering evaluation is refused with
    /// the stack-exhausted error — and nothing runs.
    #[test]
    fn below_the_floor_evaluation_is_refused_not_attempted() {
        let here = stack_marker();
        // floor = top - size + reserve = here + 4 * reserve, above us.
        set_stack_region(here + 4 * STACK_RESERVE, STACK_RESERVE);
        let mut i = Interp::new();
        STACK_REGION.with(|r| r.set(None));
        let body = BundValue::lambda(vec![BundValue::int(1)]);
        let e = Vm::eval_lambda(&mut i, &body).expect_err("refused below the floor");
        assert!(e.is_stack_exhausted(), "{}", e.0);
        assert_eq!(i.depth(), 0, "nothing ran");
    }

    /// A declared region puts the floor `size - reserve` below its top.
    #[test]
    fn a_declared_region_sets_the_floor() {
        set_stack_region(10 * 1024 * 1024, 8 * 1024 * 1024);
        let i = Interp::new();
        STACK_REGION.with(|r| r.set(None));
        assert_eq!(i.stack_floor, 2 * 1024 * 1024 + STACK_RESERVE);
    }

    #[test]
    fn autoadd_appends_the_name_instead_of_running_it() {
        let (mut i, s) = with_native("w");
        i.autoadd = true;
        i.push(BundValue::int(1));
        i.dispatch(s, false).expect("autoadd");
        assert_eq!(i.pull().map(|v| v.dt()), Some(bund2_value::CALL));
        assert_eq!(
            i.pull(),
            Some(BundValue::int(1)),
            "the value is left beneath"
        );
    }

    /// Every push writes the stack tag, with no type test — and **since D41 a
    /// scalar is no longer boxed to carry it.**
    ///
    /// This test used to assert the opposite: "tagging a scalar boxes it". It
    /// was right, and it was the whole cost — `promote/scalar` measured
    /// 25.1 ns of allocation for a value that only needed four bytes of
    /// padding. The tag is still reported identically, which is what the 39
    /// tag-bearing goldens check; only the allocation is gone.
    #[test]
    fn push_tags_with_the_current_stack() {
        let mut i = Interp::new();
        i.push(BundValue::int(1));
        let v = i.pull().expect("pushed");
        assert_eq!(v.tags().get("stack").map(|s| &**s), Some("main"));
        assert!(
            !v.is_boxed(),
            "D41: a scalar carries its tag inline and must not be boxed"
        );
    }

    /// **D41 rule 2.** Boxing a tagged scalar carries the symbol into the
    /// header's map. Without this a value that gains an `attr` after being
    /// pushed would silently lose its stack tag — the one violation in D41
    /// that no compiler catches.
    #[test]
    fn boxing_a_tagged_scalar_carries_the_tag_into_the_header() {
        let mut i = Interp::new();
        i.stacks.to_stack("side");
        i.push(BundValue::int(1));
        let v = i.pull().expect("pushed");
        assert!(!v.is_boxed());
        let boxed = v.promote();
        assert!(boxed.is_boxed());
        assert_eq!(
            boxed.tags().get("stack").map(|s| &**s),
            Some("side"),
            "the symbol must survive promotion into the map"
        );
    }

    #[test]
    fn a_named_stack_becomes_current_and_carries_its_own_tag() {
        let mut i = Interp::new();
        i.stacks.to_stack("side");
        assert_eq!(i.stacks.current_name(), "side");
        i.push(BundValue::int(1));
        assert_eq!(
            i.pull().unwrap().tags().get("stack").map(|s| &**s),
            Some("side")
        );
    }

    /// **The workbench tag is a fossil of the source stack, not of the
    /// workbench.** The workbench "does not carry a specific name", so a value
    /// returned there keeps the tag of the stack it came from.
    ///
    /// This is why `Stack::push_as` exists beside `Stack::push`. When each
    /// stack learned its own name — to stop `current_mut` allocating it twice
    /// per push — the workbench acquired one too, and pushing with *its* name
    /// would have quietly erased the fossil. Confirmed against the oracle:
    /// `@foo 1 return take` leaves a value tagged `stack: "foo"`.
    #[test]
    fn the_workbench_keeps_the_tag_of_the_stack_a_value_came_from() {
        let mut i = Interp::new();
        i.stacks.to_stack("foo");
        i.push(BundValue::int(1));
        let v = i.pull().expect("pushed");
        i.push_workbench(v);
        let back = i.pull_workbench().expect("returned");
        assert_eq!(
            back.tags().get("stack").map(|s| &**s),
            Some("foo"),
            "the workbench must not retag with its own name"
        );
    }

    /// **D13, both branches.** `with_tag` splits only when the header is
    /// actually shared; a uniquely owned one is mutated in place and needs no
    /// identity, because there is no second half to diverge from.
    ///
    /// The property D13 protects is the *shared* case, and it is asserted in
    /// `bund2-value`. What this test pins is that the fast branch is reachable
    /// at all — before the fix, `with_tag` took `&self` and cloned first, so
    /// the answer to "is anyone else holding this?" was always yes and the
    /// in-place path was dead code.
    #[test]
    fn pushing_a_fresh_scalar_does_not_mint_an_identity() {
        let mut i = Interp::new();
        i.push(BundValue::int(7));
        let v = i.pull().expect("pushed");
        assert!(
            !v.has_identity(),
            "a value that was never shared must not have been minted on push"
        );
        assert_eq!(v.tags().get("stack").map(|s| &**s), Some("main"));
    }

    /// Selecting a stack rotates the stack-of-stacks rather than reassigning
    /// it, which is what the guide describes and what `stacks_left` acts on.
    #[test]
    fn selecting_a_stack_rotates_rather_than_reassigns() {
        let mut i = Interp::new();
        i.stacks.to_stack("a");
        i.stacks.to_stack("b");
        i.stacks.to_stack("main");
        assert_eq!(i.stacks.current_name(), "main");
        assert_eq!(i.stacks.names().count(), 3, "no stack was lost");
    }

    /// The stacks are circular buffers, which is why they are `VecDeque`s.
    #[test]
    fn a_stack_rotates() {
        let mut s = Stack::default();
        for n in 1..=3 {
            s.push(BundValue::int(n));
        }
        assert_eq!(s.peek().map(|v| v.dt()), Some(bund2_value::INTEGER));
        s.rotate_left();
        assert_eq!(s.len(), 3, "rotation moves, it does not consume");
    }

    /// **The effect audit's report half — RFC-0005 criterion 25.** A native
    /// that reports at `Error` severity while it runs is a breach whatever its
    /// effect, opaque included. A warning is not.
    #[test]
    fn the_audit_records_an_error_reported_mid_body() {
        fn reports_error(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.report(bund2_api::diag::Diagnostic::error("mid-body"));
            Ok(())
        }
        fn warns(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.report(bund2_api::diag::Diagnostic::warning("fine"));
            Ok(())
        }
        let mut i = Interp::new();
        i.registry
            .register_native("e", reports_error, StackEffect::opaque(0), WordKind::Sync);
        i.registry.register_native("w", warns, eff(), WordKind::Sync);
        i.effect_audit = Some(Vec::new());
        let _ = i.eval(&[BundValue::call("w"), BundValue::call("e")]);
        assert_eq!(
            i.effect_audit.take().unwrap_or_default(),
            vec!["`e` reported at `Error` severity while it ran".to_string()]
        );
    }

    /// **The effect audit's depth half — RFC-0005 criterion 24.** A native
    /// declaring `0 -> 0` that pushes is a breach. With the audit off, nothing
    /// is recorded and the word runs as it always did.
    #[test]
    fn the_audit_records_a_pair_that_miscounts() {
        fn pushes(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.push(BundValue::int(1));
            Ok(())
        }
        let mut off = Interp::new();
        off.registry.register_native("p", pushes, eff(), WordKind::Sync);
        assert!(off.eval(&[BundValue::call("p")]).is_ok());
        assert!(off.effect_audit.is_none());

        let mut on = Interp::new();
        on.registry.register_native("p", pushes, eff(), WordKind::Sync);
        on.effect_audit = Some(Vec::new());
        assert!(on.eval(&[BundValue::call("p")]).is_ok());
        let log = on.effect_audit.take().unwrap_or_default();
        assert_eq!(log.len(), 1, "{log:?}");
        assert!(
            log.iter().all(|l| l.starts_with("`p` declares a fixed effect and declares (0, 0)")),
            "{log:?}"
        );
    }

    /// **D49 and F95.** A native that panics returns `Error::internal` naming
    /// it, and evaluation goes on being usable: the panic does not unwind out
    /// of `eval`.
    #[test]
    fn a_panicking_native_is_an_internal_error_not_an_unwind() {
        fn boom(_: &mut dyn Vm) -> Result<(), Error> {
            panic!("boom from a dependency");
        }
        let mut i = Interp::new();
        i.registry.register_native("boom", boom, eff(), WordKind::Sync);
        let e = i.eval(&[BundValue::call("boom")]).expect_err("the panic is an error");
        // `eval` wraps what it returns, so the internal error is inside it.
        assert!(
            e.0.contains("internal error: native `boom` panicked: boom from a dependency"),
            "{}",
            e.0
        );
        i.eval(&[BundValue::int(1)]).expect("the interpreter still runs");
        assert_eq!(i.depth(), 1);
    }

    /// **F96.** A native that files a tail request and then fails leaves no
    /// request behind. Before the fix the request waited in `pending_tail`, and
    /// the next evaluation ran the body first — a body nobody asked for, run
    /// after its caller's error had been dealt with.
    #[test]
    fn a_failed_native_leaves_no_tail_request() {
        fn files_then_fails(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.tail_lambda(BundValue::lambda(vec![BundValue::int(99)]));
            Err(Error("failed after filing".into()))
        }
        let mut i = Interp::new();
        i.registry.register_native("ff", files_then_fails, StackEffect::opaque(0), WordKind::Sync);
        assert!(i.eval(&[BundValue::call("ff")]).is_err());
        i.eval(&[BundValue::int(1)]).expect("runs");
        assert_eq!(i.depth(), 1, "only the 1: the stale body did not run");
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(1));
    }

    /// RFC-0005 criterion 30's audit half (the eleventh review's B1). A
    /// native that declares a fixed effect and asks to end the program is a
    /// breach: compiled code holding promoted values across it would not see
    /// the stop, which comes at Tier 0's next step. Only `bund.exit` asks, and
    /// it is opaque; this makes that a property.
    #[test]
    fn an_exit_requested_under_a_fixed_effect_is_a_breach() {
        fn exits(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.request_exit(3);
            Ok(())
        }
        let mut i = Interp::new();
        i.registry.register_native("ex", exits, StackEffect::fixed(0, 0), WordKind::Sync);
        i.effect_audit = Some(Vec::new());
        let _ = i.eval(&[BundValue::call("ex")]);
        let log = i.effect_audit.take().unwrap_or_default();
        assert!(
            log.iter().any(|l| l.contains("requested an exit")),
            "the audit recorded no breach: {log:?}"
        );
        assert_eq!(i.exit_requested(), Some(3), "the exit itself still stands");
    }
}
