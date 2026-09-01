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
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

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

    /// The current stack, creating it if it has somehow gone missing.
    ///
    /// The invariant is that a current stack always exists, and asserting it
    /// put a panic on the hottest path in the interpreter. Creating an empty
    /// one instead is indistinguishable in every reachable case and cannot
    /// abort the program in an unreachable one.
    fn current_mut(&mut self) -> (&mut Stack, String) {
        let name = self.current_name().to_string();
        let s = self.stacks.entry(name.clone()).or_default();
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
        // `rotate_left` moves the front to the back without an `Option` to
        // unwrap, so there is no impossible case to explain.
        while self.current_name() != name && !self.order.is_empty() {
            self.order.rotate_left(1);
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
/// One body being executed, and how far through it we are.
///
/// **RFC-0003 §S4.** A frame carries the body, an instruction pointer, and an
/// optional exit action that runs when the frame leaves — however it leaves.
/// That last part is what fixes F57: the reference restores a context's stack
/// with a statement placed after three early returns, so a failure skips it.
struct Frame {
    body: Vec<BundValue>,
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
    pending_tail: Option<Vec<BundValue>>,
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
            contexts: Vec::new(),
            reporter: Box::new(bund2_api::diag::SilentReporter),
            frames: Vec::new(),
            pending_tail: None,
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
                let f = self
                    .registry
                    .slot(s)
                    .and_then(|sl| sl.command)
                    .ok_or_else(|| {
                        Error::internal(format!(
                            "`{}` resolved to a command whose binding is absent",
                            self.registry.interner.name(s)
                        ))
                    })?
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
            Resolved::Lambda => {
                // `lambda_eval` applies each element of the body
                // (`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:13-14`).
                //
                // **This recurses in Rust**, exactly as the reference does, so
                // Bund call depth is Rust call depth. RFC-0003's S4 replaces it
                // with a frame loop; the body is cloned out first so the
                // registry is not borrowed across the call.
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
                let items = body
                    .as_lambda()
                    .ok_or_else(|| Error("This is not a lambda".into()))?
                    .to_vec();
                // **The call that used to recurse.** Calling a lambda is a
                // tail position: nothing in `dispatch` runs after the body. So
                // it becomes a request, and the loop pushes a frame — Bund
                // call depth stops being Rust call depth.
                self.request_tail(items);
                Ok(())
            }
            Resolved::Native => {
                let target = self.registry.resolve_target(s);
                let f = self
                    .registry
                    .slot(target)
                    .and_then(|sl| sl.native)
                    .ok_or_else(|| {
                        Error::internal(format!(
                            "`{}` resolved to a native whose binding is absent",
                            self.registry.interner.name(target)
                        ))
                    })?
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
        let floor = self.frames.len();
        self.apply_step(v)?;
        self.take_pending();
        self.run_to(floor)
    }

    /// Apply one value, leaving any requested body for the caller's loop.
    pub fn apply_step(&mut self, v: BundValue) -> Result<(), Error> {
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
                    if let Err(e) = self.apply_step(v.clone()) {
                        return Err((i, e));
                    }
                    // A top-level word may have asked for a body to run.
                    if let Err(e) = self.drain_frames() {
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
    /// `?ifthenelse`, `?try`, `times` — still call [`Interp::eval_body`], which
    /// is synchronous. Those add one Rust frame per *native*, not per Bund
    /// call, so depth is bounded by how deeply such natives nest rather than by
    /// how deep the program recurses.
    pub fn request_tail(&mut self, body: Vec<BundValue>) {
        self.pending_tail = Some(body);
    }

    /// Push a frame for whatever the last native requested, if anything.
    fn take_pending(&mut self) {
        if let Some(body) = self.pending_tail.take() {
            self.frames.push(Frame {
                body,
                ip: 0,
                exit: None,
            });
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
            if frame.ip >= frame.body.len() {
                let done = self.frames.pop();
                if let Some(Frame {
                    exit: Some(action), ..
                }) = done
                {
                    self.run_exit(action);
                }
                continue;
            }
            let v = frame.body[frame.ip].clone();
            frame.ip += 1;
            match self.apply_step(v) {
                Ok(()) => self.take_pending(),
                Err(e) => {
                    // Unwind to the floor, running every exit action on the
                    // way. This is what the reference cannot do: its restore
                    // is a statement after the early returns.
                    while self.frames.len() > floor {
                        if let Some(Frame {
                            exit: Some(action), ..
                        }) = self.frames.pop()
                        {
                            self.run_exit(action);
                        }
                    }
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

    fn snapshot(&self) -> Vec<BundValue> {
        self.stacks
            .stacks
            .get(self.stacks.current_name())
            .map(Stack::contents)
            .unwrap_or_default()
    }

    fn snapshot_workbench(&self) -> Vec<BundValue> {
        self.stacks.workbench.contents()
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
    fn tail_call(&mut self, body: Vec<BundValue>) {
        Interp::request_tail(self, body);
    }

    fn scoped_call(&mut self, stack: &str, body: Vec<BundValue>) -> Result<(), Error> {
        let prev = self.current_name();
        let floor = self.frames.len();
        self.to_stack(stack);
        self.frames.push(Frame {
            body,
            ip: 0,
            // Carried by the frame, so the unwinder runs it on a failure just
            // as the loop runs it on success. F57.
            exit: Some(ExitAction::ToStack(prev)),
        });
        self.run_to(floor)
    }

    fn eval_body(&mut self, body: &[BundValue]) -> Result<(), Error> {
        let floor = self.frames.len();
        self.frames.push(Frame {
            body: body.to_vec(),
            ip: 0,
            exit: None,
        });
        self.run_to(floor)
            .map_err(|e| Error(format!("Lambda content evaluation returned error: {}", e.0)))
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
        // Collect the stacks only if something will show them.
        let d = if self.reporter.wants_stack() {
            // Compact by default; the raw `Debug` form only when a debug
            // session asks for it. A stack of raw renderings is ~150 columns a
            // row and tells the reader nothing they were looking for.
            let w = self.reporter.value_width();
            let raw = self.reporter.wants_raw_values();
            let fmt = |v: &BundValue| if raw { v.render(false) } else { v.summary(w) };
            let stack = self.snapshot().iter().map(&fmt).collect();
            let wb = self.snapshot_workbench().iter().map(&fmt).collect();
            d.on_stack(self.current_name())
                .with_stack(stack)
                .with_workbench(wb)
        } else {
            d.on_stack(self.current_name())
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

    fn context_depth(&self) -> usize {
        self.contexts.len()
    }

    fn push_context(&mut self, name: &str) {
        // The stack to come back to is the one current *before* the switch.
        let prev = self.current_name();
        self.contexts.push((name.to_string(), prev));
    }

    fn pop_context(&mut self) -> Option<String> {
        self.contexts.pop().map(|(_, prev)| prev)
    }

    fn get_lambda(&self, name: &str) -> Option<BundValue> {
        self.registry
            .interner
            .lookup_call(name)
            .and_then(|(s, _)| self.registry.slot(self.registry.resolve_target(s)))
            .and_then(|slot| slot.lambda.clone())
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
        // A real lambda now, with an effect that distinguishes it from the
        // native's. An earlier version of this test bound a non-lambda and read
        // the resulting error as proof the lambda arm was reached — which
        // stopped meaning anything once that arm was implemented.
        i.registry
            .register_lambda("println", BundValue::lambda(vec![BundValue::Int(1)]));
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
