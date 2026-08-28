//! Tier 0: the interpreter. Mandatory on every target.
//!
//! **Scope.** This implements RFC-0002's *dispatch* — the resolution order, the
//! stacks it dispatches over, and the `Vm` receiver `bund2-api` declares. It
//! stops short of RFC-0003's IR and frame loop, which is not written yet. What
//! it makes measurable is RFC-0002's criterion 3, which asks what a dispatch
//! allocates and until now had no VM to be measured in.
//!
//! **Not blocked on D3.** D3 rules what tier `bund.eval`'s output runs at,
//! which is a Tier-1 question; nothing here needs it. `bund.eval` itself is
//! not implemented, and will need it.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};

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
#[derive(Debug, Default)]
pub struct Stack {
    items: VecDeque<BundValue>,
}

impl Stack {
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
    fn push(&mut self, v: BundValue, stack_name: &str) {
        self.items.push_back(v.with_tag("stack", stack_name));
    }

    fn pull(&mut self) -> Option<BundValue> {
        self.items.pop_back()
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
    stacks: BTreeMap<String, Stack>,
    /// Names in rotation order. The reference's stack-of-stacks is itself a
    /// circular buffer, and selecting a named stack **rotates it to the top**
    /// (`…/Introduction_the_art_of_stack_operations.typ:43`).
    order: VecDeque<String>,
    /// "a circular stack that … does not carry a specific name" (`:72`).
    workbench: Stack,
}

impl Default for Stacks {
    fn default() -> Self {
        let mut stacks = BTreeMap::new();
        stacks.insert("main".to_string(), Stack::default());
        Self {
            stacks,
            order: VecDeque::from([String::from("main")]),
            workbench: Stack::default(),
        }
    }
}

impl Stacks {
    pub fn current_name(&self) -> &str {
        self.order.front().map(String::as_str).unwrap_or("main")
    }

    fn current_mut(&mut self) -> (&mut Stack, String) {
        let name = self.current_name().to_string();
        let s = self
            .stacks
            .get_mut(&name)
            .expect("the current stack always exists");
        (s, name)
    }

    /// Make a named stack current, creating it if needed.
    ///
    /// Rotates rather than reassigns, matching the guide: "when positioning a
    /// named stack to become the current stack, the buffer rotates to bring
    /// the required stack to the proper position".
    pub fn to_stack(&mut self, name: &str) {
        self.stacks.entry(name.to_string()).or_default();
        if !self.order.iter().any(|n| n == name) {
            self.order.push_front(name.to_string());
            return;
        }
        while self.current_name() != name {
            let front = self.order.pop_front().expect("non-empty");
            self.order.push_back(front);
        }
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.stacks.keys().map(String::as_str)
    }

    pub fn workbench(&mut self) -> &mut Stack {
        &mut self.workbench
    }
}

/// The interpreter.
pub struct Interp {
    pub registry: Registry,
    pub stacks: Stacks,
    /// `apply` tests this in three places, and it does **not** precede the
    /// command check — `is_command` returns at
    /// `reference/rust_multistackvm/src/multistackvm_apply.rs:17`, before the
    /// `autoadd` test at `:19`.
    pub autoadd: bool,
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
                let f = self
                    .registry
                    .slot(s)
                    .and_then(|sl| sl.command)
                    .expect("resolve said command")
                    .f;
                f(self)
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
            Resolved::Lambda => Err(Error(
                "lambda evaluation is RFC-0003's, not implemented".into(),
            )),
            Resolved::Native => {
                let target = self.registry.resolve_target(s);
                let f = self
                    .registry
                    .slot(target)
                    .and_then(|sl| sl.native)
                    .expect("resolve said native")
                    .f;
                f(self)
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
}

impl Vm for Interp {
    fn push(&mut self, v: BundValue) {
        let (stack, name) = self.stacks.current_mut();
        stack.push(v, &name);
    }

    fn pull(&mut self) -> Option<BundValue> {
        self.stacks.current_mut().0.pull()
    }

    fn depth(&self) -> usize {
        self.depth_of(&self.current_name())
    }

    fn peek(&self) -> Option<BundValue> {
        self.stacks
            .stacks
            .get(self.stacks.current_name())
            .and_then(|s| s.peek().cloned())
    }

    fn clear(&mut self) {
        let name = self.current_name();
        self.clear_stack(&name);
    }

    fn rotate_left(&mut self) {
        self.stacks.current_mut().0.rotate_left();
    }

    fn rotate_right(&mut self) {
        self.stacks.current_mut().0.rotate_right();
    }

    fn current_name(&self) -> String {
        self.stacks.current_name().to_string()
    }

    fn to_stack(&mut self, name: &str) {
        self.stacks.to_stack(name);
    }

    fn stack_exists(&self, name: &str) -> bool {
        self.stacks.stacks.contains_key(name)
    }

    fn ensure_stack(&mut self, name: &str) {
        self.stacks.stacks.entry(name.to_string()).or_default();
        if !self.stacks.order.iter().any(|n| n == name) {
            self.stacks.order.push_back(name.to_string());
        }
    }

    fn depth_of(&self, name: &str) -> usize {
        self.stacks.stacks.get(name).map(Stack::len).unwrap_or(0)
    }

    fn push_to(&mut self, name: &str, v: BundValue) {
        self.ensure_stack(name);
        if let Some(s) = self.stacks.stacks.get_mut(name) {
            s.push(v, name);
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
        self.stacks.order.retain(|n| n != name);
        if self.stacks.order.is_empty() {
            self.stacks.order.push_back("main".to_string());
            self.stacks.stacks.entry("main".into()).or_default();
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

    fn push_workbench(&mut self, v: BundValue) {
        // The workbench "does not carry a specific name"
        // (`…/Introduction_the_art_of_stack_operations.typ:72`), so the tag it
        // receives is the stack the value came from — which is what makes a
        // workbench value's tag a fossil rather than a location.
        let name = self.current_name();
        self.stacks.workbench.push(v, &name);
    }

    fn pull_workbench(&mut self) -> Option<BundValue> {
        self.stacks.workbench.pull()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_api::{StackEffect, WordKind};

    fn eff() -> StackEffect {
        StackEffect {
            consumes: 0,
            produces: 0,
        }
    }
    fn marker(vm: &mut dyn Vm) -> Result<(), Error> {
        vm.push(BundValue::Int(7));
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
        assert_eq!(i.pull(), Some(BundValue::Int(7)));
    }

    /// RFC-0002's central behaviour, end to end this time: the same name
    /// reaches a lambda or the native depending only on the sigil.
    #[test]
    fn the_sigil_selects_the_native_over_the_lambda() {
        let (mut i, s) = with_native("println");
        i.registry.register_lambda("println", BundValue::Int(1));
        // Plain: finds the lambda, which RFC-0003 will evaluate.
        assert!(i.dispatch(s, false).is_err(), "plain must reach the lambda");
        // `$`: skips it and runs the native.
        i.dispatch(s, true).expect("$name must reach the native");
        assert_eq!(i.pull(), Some(BundValue::Int(7)));
    }

    /// The case D16 forces: a name built at run time, never lexed.
    #[test]
    fn a_runtime_string_dispatches_with_its_sigil() {
        let (mut i, _) = with_native("println");
        i.registry.register_lambda("println", BundValue::Int(1));
        i.dispatch_name("$println").expect("reaches the native");
        assert_eq!(i.pull(), Some(BundValue::Int(7)));
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
        assert_eq!(i.pull(), Some(BundValue::Int(7)));
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
        assert_eq!(i.pull(), Some(BundValue::Int(7)));
    }

    /// Under `autoadd` a non-command name is appended to the value beneath
    /// rather than executed (`apply.rs:20-27`).
    #[test]
    fn autoadd_appends_the_name_instead_of_running_it() {
        let (mut i, s) = with_native("w");
        i.autoadd = true;
        i.push(BundValue::Int(1));
        i.dispatch(s, false).expect("autoadd");
        assert_eq!(i.pull().map(|v| v.dt()), Some(bund2_value::CALL));
        assert_eq!(
            i.pull(),
            Some(BundValue::Int(1)),
            "the value is left beneath"
        );
    }

    /// Every push writes the stack tag, with no type test — which is why a
    /// scalar on a stack is boxed.
    #[test]
    fn push_tags_with_the_current_stack() {
        let mut i = Interp::new();
        i.push(BundValue::Int(1));
        let v = i.pull().expect("pushed");
        assert_eq!(v.tags().get("stack").map(String::as_str), Some("main"));
        assert!(v.is_boxed(), "tagging a scalar boxes it");
    }

    #[test]
    fn a_named_stack_becomes_current_and_carries_its_own_tag() {
        let mut i = Interp::new();
        i.stacks.to_stack("side");
        assert_eq!(i.stacks.current_name(), "side");
        i.push(BundValue::Int(1));
        assert_eq!(
            i.pull().unwrap().tags().get("stack").map(String::as_str),
            Some("side")
        );
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
            s.push(BundValue::Int(n), "main");
        }
        assert_eq!(s.peek().map(|v| v.dt()), Some(bund2_value::INTEGER));
        s.rotate_left();
        assert_eq!(s.len(), 3, "rotation moves, it does not consume");
    }
}
