//! The stacks: one named [`Stack`], and the ring of them, [`Stacks`].
//!
//! A module of its own so that `Stacks::order` and `Stacks::epoch` are private
//! to the code that keeps them in step — RFC-0005 assumption 40.

use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use bund2_api::Cells;
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
    pub(super) items: VecDeque<BundValue>,
    /// A bound set by `ensure_stack_with_capacity`, if one was.
    ///
    /// Per-stack rather than a side table, so a push cannot consult one
    /// stack's length against another's bound — which is exactly what the
    /// reference does (`ts_push.rs:47-48` reads `current_stack_len()` and
    /// `stack_capacity(name)`), and F39 records.
    pub(super) cap: Option<usize>,
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
    pub(super) fn push(&mut self, v: BundValue) {
        let (sym, name) = (self.sym, Rc::clone(&self.name));
        self.push_as(v, sym, &name);
    }

    /// Push, tagging with a name that is **not** this stack's own.
    ///
    /// One caller, and it is the reason this exists: the workbench "does not
    /// carry a specific name", so a value pushed there keeps the tag of the
    /// stack it came from — a fossil rather than a location. Tagging it with
    /// the workbench's own name would erase that.
    pub(super) fn push_as(
        &mut self,
        v: BundValue,
        sym: bund2_value::StackSym,
        stack_name: &Rc<str>,
    ) {
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

    pub(super) fn pull(&mut self) -> Option<BundValue> {
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
    pub(super) stacks: BTreeMap<Rc<str>, Stack>,
    /// Names in rotation order. The reference's stack-of-stacks is itself a
    /// circular buffer, and selecting a named stack **rotates it to the top**
    /// (`…/Introduction_the_art_of_stack_operations.typ:43`).
    order: VecDeque<Rc<str>>,
    /// "a circular stack that … does not carry a specific name" (`:72`).
    pub(super) workbench: Stack,
    /// **The current stack's epoch — RFC-0005 §S6, *Addressing*.**
    ///
    /// Bumped on every change of *which* stack is current, and on nothing else:
    /// pushing, pulling and clearing leave it alone, because compiled code
    /// holding promoted values only needs to know that the stack it resolved
    /// against is still in force.
    ///
    /// **It lives here, in the type that owns `order`, because that is what
    /// makes it a property rather than a list to keep in step.** Three writers
    /// used to reach past this type into `order` directly — `drop_stack` and
    /// the two rotations, on `Interp`'s `Vm` impl — and any of them could have
    /// moved the front while leaving an epoch held elsewhere untouched. `order`
    /// is now private to this type's own mutators, each of which bumps, so a
    /// switch that does not bump cannot be written. Same shape as
    /// `Registry::touch`, for the same reason (assumption 37).
    ///
    /// **Private to this module, which holds nothing but the two stack types**
    /// (the twenty-third review's S1). While `Stacks` sat in `Interp`'s module
    /// the privacy was nominal: `Interp` could write `order` directly, and
    /// `to_stack` was `pub` beside a `pub` field, so any crate could move the
    /// front and leave §S6's cell stale. Every mutator that moves the front is
    /// now `pub(super)`, and `Interp` publishes after each.
    ///
    /// A plain counter beside §S6's cell, which `bump_epoch` writes in the
    /// same statement. `Interp` used to copy one to the other by hand after
    /// each of five calls.
    epoch: u64,
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
            epoch: 0,
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
    pub(super) fn current_mut(&mut self) -> &mut Stack {
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
    pub(super) fn add_as_current(&mut self, name: Rc<str>, cells: &Cells) {
        if !self.order.is_empty() {
            self.order.rotate_left(1);
        }
        self.order.push_front(name);
        // A new stack becomes current, so the epoch always moves here.
        self.bump_epoch(cells);
    }

    /// The current stack's epoch — RFC-0005 §S6. See [`Stacks::epoch`].
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Move the epoch on. Saturating, so it can never wrap onto a value a
    /// guard is holding; at `u64` that bound is unreachable in practice, and
    /// saturating rather than wrapping makes it unreachable by construction.
    ///
    /// **It writes §S6's cell itself**, so the counter and the cell compiled
    /// code loads cannot differ: every function that moves the front takes
    /// the cells, and none can bump without them.
    fn bump_epoch(&mut self, cells: &Cells) {
        self.epoch = self.epoch.saturating_add(1);
        cells.set_epoch(self.epoch);
    }

    /// Drop a named stack, restoring `main` if that emptied the ring.
    ///
    /// **Sealed here rather than done through `order` from outside**, so the
    /// epoch cannot be forgotten: this is one of the three writers that used to
    /// reach past this type.
    ///
    /// **The bump is exact, not conservative.** `retain` moves the front only
    /// when the dropped name *is* the front, and the `main` restored below only
    /// changes it when the ring had emptied. Bumping unconditionally would fire
    /// every guard in a compiled body on a drop that touched some other stack.
    pub(super) fn drop_named(&mut self, name: &str, cells: &Cells) {
        let before = self.current_name().to_string();
        self.stacks.remove(name);
        self.order.retain(|n| &**n != name);
        if self.order.is_empty() {
            let main: Rc<str> = Rc::from("main");
            self.order.push_back(Rc::clone(&main));
            self.stacks
                .entry(main)
                .or_insert_with_key(|k| Stack::from_rc(Rc::clone(k)));
        }
        if self.current_name() != before {
            self.bump_epoch(cells);
        }
    }

    /// Rotate the ring one step left, as `stacks_left` does.
    ///
    /// Sealed for the epoch, as [`Stacks::drop_named`] is. **A ring of one does
    /// not move**, so it does not bump: the front name is unchanged and a guard
    /// that fired on it would be reacting to nothing.
    pub(super) fn rotate_left(&mut self, cells: &Cells) {
        if self.order.len() > 1 {
            self.order.rotate_left(1);
            self.bump_epoch(cells);
        }
    }

    /// Rotate the ring one step right, as `stacks_right` does.
    pub(super) fn rotate_right(&mut self, cells: &Cells) {
        if self.order.len() > 1 {
            self.order.rotate_right(1);
            self.bump_epoch(cells);
        }
    }

    /// Whether a stack of this name exists.
    pub(super) fn has(&self, name: &str) -> bool {
        self.stacks.contains_key(name)
    }

    /// Whether the ring already carries this name.
    pub(super) fn in_ring(&self, name: &str) -> bool {
        self.order.iter().any(|n| &**n == name)
    }

    /// Make a named stack current, creating it if needed.
    ///
    /// Rotates rather than reassigns, matching the guide: "when positioning a
    /// named stack to become the current stack, the buffer rotates to bring
    /// the required stack to the proper position".
    // Named for the word it implements, `to_stack`, and for `Vm::to_stack`;
    // clippy reads `to_` as a conversion. It said nothing while this was
    // `pub`, since it leaves exported names alone.
    #[allow(clippy::wrong_self_convention)]
    pub(super) fn to_stack(&mut self, name: &str, cells: &Cells) {
        self.stacks
            .entry(Rc::from(name))
            .or_insert_with_key(|k| Stack::from_rc(Rc::clone(k)));
        if !self.order.iter().any(|n| &**n == name) {
            self.add_as_current(Rc::from(name), cells);
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
        let before = self.current_name().to_string();
        for _ in 0..self.order.len() {
            if self.current_name() == name {
                break;
            }
            self.order.rotate_left(1);
        }
        // Exact, as in `drop_named`: `to_stack` to the stack already current
        // rotates zero times and changes nothing, and must not bump.
        if self.current_name() != before {
            self.bump_epoch(cells);
        }
    }

    /// The ring, current stack first. Read-only: `order` has no writer outside
    /// this module, which is what makes the epoch a property (assumption 40).
    pub(super) fn ring(&self) -> impl Iterator<Item = &str> {
        self.order.iter().map(|n| &**n)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.stacks.keys().map(|k| &**k)
    }

    pub fn workbench(&mut self) -> &mut Stack {
        &mut self.workbench
    }
}
