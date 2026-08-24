# Bund2 — Addendum: Word Extensibility and Asynchrony

**Subject:** Two design questions arising from the JIT feasibility study
**Status:** Research / pre-RFC addendum
**Date:** 2026-08-23
**Repository decision:** single monorepo, `github.com/vulogov/Bund2`; all supporting crates
vendored and rewritten in-tree; library crates prefixed `bund2-`; CLI binary `bund2`.

---

## 0. Consequences of the monorepo decision

Three things change relative to the main study:

1. **The `">=0.*.*"` pinning hazard disappears.** Intra-project crates become path
   dependencies in one workspace. Only genuinely external dependencies (pest, cranelift,
   serde) need version discipline, and Cranelift gets an exact pin.

2. **`rust_dynamic`, `rust_multistack` and `rust_multistackvm` are absorbed, not vendored.**
   The main study's §4.1 rewrites `Value` outright; there is nothing left of
   `rust_dynamic` to preserve except its type taxonomy and its conversion tables, both of
   which are worth keeping. `bund_language_parser` is the exception — `bund.pest` should be
   vendored close to verbatim and evolved, since the surface syntax is not what's changing.

3. **One crate must have a stability guarantee: `bund2-api`.** Everything else can churn
   freely inside the monorepo, but external Rust word packages have to depend on
   *something*, and if that something is the whole workspace, third-party packages become
   version-locked to the compiler. `bund2-api` is small, stable, and depends on nothing but
   `bund2-value`.

### Revised workspace

```
bund2-api        # STABLE. ABI types, NativeFn signatures, StackEffect, registration traits
bund2-value      # BundValue, heap payloads, interning
bund2-syntax     # pest grammar → AST
bund2-ir         # BundIR, stack-effect annotations, object format
bund2-interp     # Tier 0: flat-loop interpreter with explicit frame stack
bund2-jit        # Tier 1: BundIR → CLIF          [feature = "jit"]
bund2-stdlib     # the 156 words: native impls + effect signatures + JIT lowerings
bund2-runtime    # VM assembly, tiering policy, word slot table
bund2-async      # executor integration, async native words  [feature = "async"]
bund2            # library façade (today's `bundcore` API surface)
bund2-cli        # package producing binary `bund2`
```

`bund2-cli` produces the binary named `bund2`; the façade library is the `bund2` package.
Keeping them as separate packages avoids the lib/bin coupling that makes `cargo publish`
and feature resolution awkward.

---

## 1. Can Bund2 keep both ways of defining words?

**Yes — and the Bund-defined path gets strictly better, while the Rust-defined path needs
one addition to its registration API.**

### 1.1 What exists today

| Mechanism | Type | Storage | Registered by |
|---|---|---|---|
| Bund-defined word | `Value::Lambda(Vec<Value>)` | `VM.lambdas: HashMap<String, Value>` | `{ … } "name" register` |
| Rust inline word | `VMInlineFn = fn(&mut VM) -> Result<&mut VM, Error>` | `VM.inline_fun` | `vm.register_inline(name, f)` |
| Rust stack-level word | `InlineFn = fn(&mut TS) -> Result<&mut TS, Error>` | `TS.inline_fun` | `ts.register_inline(name, f)` |
| Rust command | `VMInlineFn` | `VM.command_fun` | `vm.register_command(name, f)` |
| External crate stdlib | `BundInitFn = fn(&mut Bund) -> Result<&mut Bund, Error>` | `bundcore::STDLIB` — a `lazy_static! Mutex<HashMap<…>>` | `bundcore::add_stdlib(name, f)` |

The last row is the extension point that external Rust crates use. It is a process-global
mutable registry.

### 1.2 How both map onto the slot table

Both collapse into the single `Vec<WordSlot>` from the main study §4.2. Four kinds of
entry, rather than three:

```rust
pub enum WordEntry {
    /// Rust word. Opaque machine code. Optimization barrier.
    Native   { f: NativeFn, blocking: Blocking },
    /// Rust word the JIT knows how to lower into CLIF directly. stdlib only.
    Intrinsic{ f: NativeFn, lower: LowerFn },
    /// Bund word, not yet hot.
    Interp   (Rc<IrBody>),
    /// Bund word, compiled.
    Jitted   { code: *const u8, ir: Rc<IrBody>, gen: u32 },
    Alias    (SymbolId),
    Undefined,
}
```

- **Bund-defined words gain a real compilation path.** Today a lambda is a `Vec<Value>`
  re-walked and deep-cloned on every call. In Bund2 it becomes an `IrBody`, interpreted at
  Tier 0 and compiled at Tier 1. This is the feature that actually improves: *Bund-defined
  words become as fast as Rust-defined ones for arithmetic and stack work*, which they can
  never be today.

- **Rust-defined words are unchanged in kind.** A `NativeFn` is already machine code;
  JIT'd code reaches it with `call_indirect` on the raw function address materialised as an
  `iconst`. That deliberately avoids the `FuncId` relocation path and its ±2GB range
  constraint (main study §3.2d) — the address goes in a register, not in a 32-bit reloc.

### 1.3 The three things that must change

**(a) Native registration must declare a stack effect.**

This is the substantive API change. Today `register_inline("+", f)` carries no metadata, so
the compiler cannot know that `+` consumes two values and produces one. Without that, every
native word is an unconditional barrier: the abstract stack must be spilled before the call
and reloaded after, and no slot promotion survives across it.

```rust
// bund2-api
pub struct StackEffect {
    pub takes: Arity,        // Exactly(n) | AtLeast(n) | Variadic
    pub gives: Arity,
    pub touches_workbench: bool,
    pub may_switch_stack: bool,   // if true: hard barrier regardless of arity
    pub may_reenter: bool,        // calls back into the VM (lambda_eval etc.)
}

pub trait WordPackage {
    fn register(&self, reg: &mut Registry);
}

impl Registry {
    pub fn native(&mut self, name: &str, effect: StackEffect, f: NativeFn) -> SymbolId;
}
```

With a declared `Exactly(2) → Exactly(1)` effect and `may_switch_stack: false`, the JIT can
keep the rest of the abstract stack in registers and only materialise the two operands.
Without it, everything spills.

This is worth stating plainly: **the stack-effect declaration is not bureaucracy, it is the
entire difference between a native word costing a call and a native word costing a call
plus a full stack spill/reload.** The 156 stdlib words need these annotations anyway
(main study §4.4); external packages should be held to the same standard.

**(b) Native calls are spill/reload points, and that has to be designed, not discovered.**

A `NativeFn` receives a `*mut VmContext` and expects to see a coherent operand stack. JIT'd
code that is holding the top three operands in registers does not have one. So the calling
sequence is:

```
  spill promoted slots → current stack memory
  store current stack length into VmContext
  call_indirect native_fn(vm_ptr)
  branch on returned Status
  reload promoted slots (only those the effect says survive)
```

`may_reenter: true` words (anything that calls back into `lambda_eval`, `execute`, `map`,
`for`) need the *full* context coherent, not just the stack — the word slot table base, the
error slot, the frame stack. Since the reentrant path is the general one, the simplest
correct rule is: `VmContext` is fully coherent at every native call boundary, and the only
thing the JIT gets to keep in registers across a native call is what the effect explicitly
permits (which, for `may_reenter` words, is nothing).

**(c) `bundcore::STDLIB` should stop being a process-global `Mutex<HashMap>`.**

A `lazy_static!` global registry is workable for a single blocking VM. It is wrong the
moment there are multiple VMs with different word sets, and it is wrong under async where
two tasks may build VMs concurrently. Replace with an explicit builder:

```rust
let vm = Bund2::builder()
    .with_stdlib()
    .with_package(bund2_http::Package)     // external crate
    .with_package(my_crate::Package)
    .build()?;
```

Same capability, no global state, and it makes "this VM has these words" an inspectable
fact rather than a property of link order.

### 1.4 The `Intrinsic` variant — and why not to expose it

`Intrinsic` is a word that ships both a native implementation (for Tier 0) *and* a CLIF
lowering function (for Tier 1). This is how `+`, `<`, `dup`, `drop` and the rest of the
hot stdlib should be built: the JIT emits a tag check and an `iadd`, not a `call` to
`stdlib_add_inline`.

It is technically possible to expose `LowerFn` through `bund2-api` so external crates can
supply their own CLIF lowerings. **Recommend against it**, at least initially: a
`LowerFn` signature contains `cranelift_frontend::FunctionBuilder`, which means every
external package that uses it is pinned to the exact Cranelift version Bund2 was built
with. Given that `cranelift-jit` self-describes as "extremely experimental" and moves on a
monthly cadence, that turns `bund2-api`'s stability guarantee into a lie. Keep `Intrinsic`
internal to `bund2-stdlib`; external crates get `Native` with a declared effect, which is
already a large improvement on today.

### 1.5 Dynamic loading

Today's extension model is compile-time linking: an external crate calls `add_stdlib` and
is linked into the binary. **Keep it.** Runtime `dlopen`-style plugins would require a
`repr(C)` stable ABI for `BundValue` and `VmContext`, which forecloses exactly the layout
freedom the value rewrite depends on. The monorepo decision reinforces this — `bund2` is a
compiler you rebuild, not a runtime you extend at load time.

### 1.6 Summary

| Capability | Today | Bund2 |
|---|---|---|
| Define a word in Bund | yes, `Vec<Value>` walked per call | yes, IR-backed, interpreted then JIT-compiled |
| Define a word in Rust | yes, opaque `fn(&mut VM)` | yes, plus declared stack effect |
| Rust word uses external crates | yes | yes, unchanged |
| Rust word visible to the optimizer | n/a | via declared effect: no spill, no inline |
| Redefine a word at runtime | yes | yes, via slot generation counter |
| Bund word runs at native speed | no | yes, once hot |
| Stdlib word lowered to machine instructions | n/a | yes, via `Intrinsic` |

The answer to the question as asked is yes on both counts, with the caveat that the Rust
extension API acquires one required field it does not have today.

---

## 2. Can Bund2 be async?

**Partly, and the useful part is achievable — but "async" bundles four different features
with very different costs, and the two decisions that determine which of them are reachable
land in Phase 1 and Phase 3 of the main study, long before any async code is written.**

### 2.1 Disambiguating the question

| | Feature | Difficulty | Verdict |
|---|---|---|---|
| **(a)** `Bund2::run()` as an `async fn` that doesn't block the executor | low | do it |
| **(b)** Many independent VMs concurrently on a thread pool | low | do it |
| **(c)** A Bund word performs I/O and yields mid-execution | **high** | achievable at Tier 0; see §2.4 |
| **(d)** Parallel execution within one program (e.g. per-stack) | high, and semantically fraught | not recommended |

(a) and (b) are the ones that matter for realistic use — embedding Bund2 in a service,
running many scripts, message-driven workloads. (c) is the one people mean when they say
"make it async" and it is the one with a real cost. (d) is a different language.

### 2.2 The blocker nobody sees coming: Rust recursion in the interpreter

The current interpreter recurses on the *Rust* call stack:

```
apply → lambda_eval → apply → lambda_eval → …
```

`stdlib_execute_base_inline` also recurses into itself for `LIST` unfolding. This means the
VM's execution state is partly held in native stack frames, not in a data structure.

That single property forecloses all of (c) and causes stack-overflow-on-deep-recursion
today. Fix it in Phase 3 by construction:

> **Tier 0 is a flat loop over an explicit frame stack. No Rust recursion in the
> interpreter, ever.**

```rust
struct Frame { body: Rc<IrBody>, pc: usize, base: usize }
struct Interp { frames: Vec<Frame>, /* … */ }
```

With that, the *entire* VM state — operand stacks, workbench, frame stack, program counter
— is plain heap data. Suspending Tier 0 is then trivial: stop the loop and return. Resuming
is calling it again. This is worth doing on its own merits for the stack-depth fix alone,
and it happens to be the enabling condition for async.

Concatenative languages are unusually well-suited to this: almost all state is already in
the stacks, which are heap structures. Bund is one refactor away from having a fully
reifiable execution state — the current implementation just happens not to be written that
way.

### 2.3 The second decision: `Rc` vs `Arc`

The main study recommended `Rc` for heap payloads on the grounds that the VM is
single-threaded. Async changes the question, and it must be answered in **Phase 1**,
because retrofitting is a rewrite.

- **`Arc` everywhere:** atomic refcount traffic on every value clone. In JIT'd code that
  means a helper call with a `lock`-prefixed instruction on paths where `Rc` would be a
  non-atomic increment or nothing at all. A permanent tax on the hot path, paid by every
  program, to enable a feature most programs won't use.
- **`Rc` + VM-per-task actor model:** each VM is `!Send` and owns its values; VMs run on
  their own task; values crossing a VM boundary are converted to an owned, `Send`
  representation.

**Recommend `Rc` with the actor model.** This is not a compromise — it is the design that
matches what Bund already is. Note that `rust_dynamic` already has exactly the right
primitive for the boundary: `Value::wrap()` produces an `ENVELOPE` containing a bincode
serialisation, and `unwrap()` reconstructs it. An envelope is by construction `Send`.
The message-passing boundary between VMs is a mechanism the language already has a word
for.

Concretely: `!Send` VMs pinned to tasks on a `LocalSet` (or one executor thread per VM),
communicating by channels carrying envelopes. This gives (b) fully, and (d) in the only
form worth having — as explicit message passing, not as implicit shared-stack parallelism.

### 2.4 Suspending inside a word — and the coloring problem

This is (c), and it is the one that interacts badly with the JIT.

**At Tier 0 it works.** With §2.2's flat loop, an IR op that awaits simply returns control
to the async driver with the pending future stored in the frame. The native word ABI grows
a third variant:

```rust
pub enum NativeKind {
    Sync,                                    // runs to completion, must not block
    Blocking,                                // must be dispatched to spawn_blocking
    Async(fn(&mut Vm) -> BoxFuture<'_, Status>),
}
```

The `Blocking` variant matters more than it looks: an opaque external Rust word that does a
synchronous socket read will stall the executor thread. Today that is invisible and
harmless; under async it is a production incident. Making the registration declare it is
the only way the runtime can do anything about it.

**At Tier 1 it does not work.** A JIT'd frame lives on the native stack. Cranelift has no
mechanism to capture and resume a suspended native frame — no continuation capture, no
OSR, and (per the main study §3.2b) no deopt metadata either. A JIT'd word runs to
completion or it doesn't run.

The consequence is a **coloring problem**, and it colors *JIT eligibility*, not just
function signatures:

> A word whose IR contains an await point is pinned to Tier 0.
> Any word that calls such a word is also pinned to Tier 0, transitively —
> because a JIT'd frame cannot have a suspending frame above it.

Computed as a fixpoint over the call graph at registration time, invalidated when
`register` bumps a slot generation.

**Why this is mostly harmless in practice:** the async color and the JIT-hotness color are
anti-correlated. The words you want compiled are tight arithmetic and stack loops, which
don't await. The words that await are I/O-bound, where a few hundred nanoseconds of
interpretive dispatch is lost in the microseconds of the syscall. The pathological case is
an awaiting word called from inside a hot numeric loop — and that program has a latency
problem far larger than its dispatch overhead.

The transitive rule does have a sharp edge: a widely-used utility word that acquires an
await point de-optimises everything that calls it. Worth surfacing as a diagnostic
(`bund2 explain --tier <word>`) rather than letting it be a silent performance cliff.

### 2.5 If Tier 1 really must suspend: stackful coroutines

If (c) at Tier 1 turns out to be a hard requirement, the mechanism is stackful coroutines —
give each Bund task its own native stack and suspend by switching stacks, which works
regardless of whether the frames on it are interpreted or JIT'd.

`corosensei` (0.3.4, updated 2026-05, ~8M downloads) is the candidate: pure Rust, which
matches the project's dependency posture, x86-64/aarch64 support matching Cranelift's, and
unwinding support. `generator` is the higher-traffic alternative.

Costs, honestly: `unsafe` stack switching in the core of the VM; per-task stack allocation
(so tasks become expensive relative to futures); the interaction between Cranelift's unwind
info and the coroutine's stack switch needs verification, not assumption; and `!Send`
across yield points unless the stacks are carefully managed.

**Recommendation: do not build this in v1.** Adopt §2.4's coloring rule, ship it, and see
whether real programs actually hit the pinned-to-Tier-0 case. If they don't — and the
anti-correlation argument says they mostly won't — the stackful machinery is complexity
bought for nothing.

### 2.6 JIT compilation under concurrency

One detail that falls out of (b): `JITModule` is `Send` but **not** `Sync`, and
`finalize_definitions` needs `&mut`.

Two options:

- **Per-VM `JITModule`.** Simple, but every thread recompiles the same hot words and every
  thread's code memory grows independently. Given that code memory is never reclaimed
  (main study §3.2a), N threads means N× the unbounded growth. Bad.
- **One shared compile service.** A single `Mutex<JITModule>` on a dedicated compiler task.
  VMs enqueue tier-up requests; the compiler compiles and writes the resulting code pointer
  into the shared word slot. Executable code pointers are just addresses — calling them
  from any thread is fine. Compilation is serialised, execution is not.

**Recommend the shared compile service.** It also makes the global code-memory cap a single
enforceable number rather than a per-thread one, which is the only way the §3.2a mitigation
actually holds under concurrency.

### 2.7 Summary and sequencing

| Feature | Verdict | Decision lands in |
|---|---|---|
| Flat-loop Tier 0, no Rust recursion | **required regardless of async** | Phase 3 |
| `Rc` + `!Send` VM + envelope message passing | recommended | Phase 1 |
| `async fn` façade, non-blocking embedding | do it | after Phase 3 |
| Many VMs concurrently | do it, actor model | after Phase 3 |
| `Sync` / `Blocking` / `Async` native word kinds | required for any async | Phase 1 (`bund2-api`) |
| Await inside a Tier-0 word | yes | after Phase 3 |
| Await inside a JIT'd word | **no** — coloring rule pins to Tier 0 | Phase 5 |
| Stackful coroutines to lift that restriction | defer; probably unnecessary | post-v1, evidence-driven |
| Shared JIT compile service | required under concurrency | Phase 5 |
| Parallel execution within one program | not recommended | — |

**The actionable point:** async ships late, but three of its prerequisites are decisions
that cannot be deferred — the value refcount strategy (Phase 1), the native word ABI's
blocking declaration (Phase 1), and the absence of Rust recursion in Tier 0 (Phase 3). Get
those right while building the non-async system and async becomes an additive feature. Get
them wrong and it is a second rewrite.

---

## 3. Revised open questions

Superseding items 6 and 7 of the main study, and adding:

1. **Is fine-grained async (§2.1c) actually required, or is VM-per-task (§2.1b) enough?**
   This determines whether the coloring rule is a footnote or a central constraint. Answer
   it from the intended workloads before Phase 1, because it feeds the `Rc`/`Arc` decision.
2. **What is the expected number of concurrent VMs?** Tens (actor model, fine) versus
   thousands (per-task stacks and per-VM word tables become the memory story).
3. **Do external word packages exist today outside the five crates?** If so, the
   `StackEffect` requirement in §1.3a is a migration, and the `bund2-api` surface should be
   designed against those real packages rather than hypothetically.
4. **Should `Intrinsic` lowerings ever be third-party?** §1.4 says no; revisit only if
   Cranelift's API stabilises.
5. **Is `ENVELOPE`-as-message-boundary (§2.3) semantically adequate**, or do cross-VM
   messages need to carry things bincode can't round-trip (lambdas referencing native
   words, open handles)?
