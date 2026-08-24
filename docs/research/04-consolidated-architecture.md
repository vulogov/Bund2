# Bund2 — Consolidated Architecture, Gap Assessment, and Further Directions

**Status:** Research / pre-RFC consolidation
**Date:** 2026-08-23
**Supersedes nothing; folds together:** the JIT feasibility study and the addenda on
extensibility/async, native binaries, and metaprogramming/OOP/debugger.

---

## Part 1 — Where the architecture landed

### 1.1 The design in one page

```
                            source (.bund)
                                  │
                          bund2-syntax  ── pest → AST (spans)
                                  │
                           bund2-ir  ── BundIR: linear, span-carrying,
                                  │      stack-effect annotated, serialisable
                ┌─────────────────┼─────────────────┐
                │                 │                 │
         bund2-interp       bund2-jit          bund2-jit
          (Tier 0)          JITModule         ObjectModule
        MANDATORY           optional           optional
                │                 │                 │
          flat loop over    hot words,          .o → cc → ./program
          explicit frame    call_indirect
          stack, no Rust    through slots
          recursion
                └─────────────────┴─────────────────┘
                                  │
                        one mutable word slot table
                        Vec<WordSlot> indexed by SymbolId
                     Native | Intrinsic | Interp | Jitted | Aot | Alias
```

**Values.** 24 bytes, `Copy`, `{ tag: u64, payload: u64, birth: u64 }`. `i64` and `f64`
unboxed at full width. Heap payloads behind `Rc`. `birth` is a monotonic counter serving as
`.id` (nanoid derived lazily) and as the basis for `.timestamp` (sampled clock). Down from
~180 bytes with a `nanoid!()` allocation and a clock read per construction.

**Symbols.** Every word name interned to `SymbolId(u32)` at parse time. The five
name-keyed `HashMap`s and the `format!("{}_inline", name)` suffix scheme collapse into one
array index. Removes ~10 heap allocations and 5 hash lookups per word call.

**Lambdas.** Canonical body stays `Vec<BundValue>` — program-visible, editable,
serialisable, which `to_lambda` / `make.call` / `save.lambdas` require. A compiled form
(`IrBody` + optional code pointer) hangs off it as a cache keyed by content hash,
invalidated on mutation. Dynamic lambdas therefore become compilable, which they have never
been.

**Words.** Bund-defined and Rust-defined both live in the slot table. Rust registration
gains one required field — a declared `StackEffect` — without which every native call
forces a full spill of the abstract stack. `bund2-api` is the one crate with a stability
guarantee.

**Redefinition.** `register` / `alias` / `load.lambdas` mutate the table at runtime, so all
inter-word calls are indirect through slots with a generation counter. This holds in AOT
too, and is Bund's own semantics rather than a Cranelift limitation.

**OOP.** Ports unchanged — it is a library over MAP values, PTR/LAMBDA method slots and
`.super` lookup, all of which are preserved. The one addition: flatten the `.super` chain
into a per-class vtable at `register_class` (pure function of the class graph), leaving the
field tree alone because `set_value_in_object` mutates embedded parents.

**Async.** `Rc` + `!Send` VMs as actors, envelopes across boundaries. Tier 0 suspends
freely because its state is data; JIT'd frames don't suspend, so await points pin a word to
Tier 0 transitively.

**Debugger.** An execution mode of Tier 0, not a separate parser. `step`/`next`/`finish`,
real backtraces, breakpoints (symbol / location / Bund-lambda condition), and watchpoints
on named stacks.

**Deployment.** `--emit=bundle` (runtime + IR, all platforms, no Cranelift),
`--emit=obj`, `--emit=native` (cranelift-object → `cc`).

### 1.2 The convergences

Three separate features reduce to **one Phase 3 refactor** — Tier 0 as a flat loop over an
explicit frame stack: async suspension, the debugger, and the stack-overflow-on-deep-recursion
bug. Two separate features reduce to **one tier-pinning mechanism**: await points and
breakpoints. Two separate features reduce to **one generation counter**: word redefinition
and class redefinition.

That much convergence on mechanisms arrived at independently is the strongest available
evidence the shape is right.

---

## Part 2 — How close is this to the original idea?

The original framing: *"re-implementing this language as a JIT-compiled, metaprogramming,
dynamically typed, concatenative language operating on multiple stack VMs."*

| Goal | Status |
|---|---|
| Concatenative | preserved exactly |
| Multiple stack VMs + workbench | preserved exactly, and the workbench becomes the promotable slot |
| Dynamically typed | preserved exactly |
| Metaprogramming | preserved exactly, and dynamic lambdas gain a compilation path they never had |
| Syntax and logic 100% preserved | held, at a measured cost (§2.2) |
| **JIT-compiled** | **partially — and this is the drift** |

### 2.1 The drift, stated plainly

We began at *"Bund is a JIT-compiled language"* and converged on *"Bund is a language with
a mandatory interpreter, an optional JIT tier, and an optional AOT compiler."* The JIT
moved from **identity** to **tier**, and plausibly not even the primary production tier —
AOT looks better for shipped programs, and the JIT is best understood as the REPL's
execution mode.

Three forcing functions produced that, all found in your source rather than assumed:

1. **`bund.eval` exists**, alongside `compile`, `load.script` and `load.lambdas`. The front
   end is a mandatory runtime component. Once the compiler is a runtime service and all 357
   word implementations must exist as Rust functions regardless, Tier 0 costs a dispatch
   loop and a `Vec<Frame>` on top of code you have to write anyway. Refusing to build it
   costs *more* than building it.
2. **`cranelift-jit` cannot free or redefine an individual function.** The mitigation is a
   code-memory cap that demotes to a lower tier — which requires a lower tier to exist.
   Without Tier 0 the cap becomes hard failure instead of graceful degradation, in a
   language whose REPL and `debug` word both drive nested eval loops.
3. **`.id` and `.timestamp` are public API** on the base `Object` class, so the value could
   not shrink to 16 bytes.

Only the second is Cranelift's doing. The other two are Bund's, and they are consequences
of the preservation constraint you set — which was the right constraint.

### 2.2 What the constraint cost, itemised

| Concession | Cause |
|---|---|
| 24-byte value instead of 16 | `.id` / `.timestamp` on `Object` |
| Indirect calls everywhere, even in AOT | `register` / `load.lambdas` |
| Closed-world analysis rarely fires | `bund.eval` reachable in most programs |
| Front end in every binary | `bund.eval` |
| Variadic `*`-family is a permanent optimization barrier | language design |
| Await points and breakpoints pin words to Tier 0 | no OSR in Cranelift |

None of these is fatal. Together they mean Bund2 will be a fast dynamic language, not a
fast static one — which was always the honest ceiling.

### 2.3 What you gained that wasn't in the original idea

- **AOT native binaries** with no Cranelift in the shipped artifact.
- **Dynamic lambdas become compilable** — the thing the current implementation is worst at.
- **OOP method dispatch becomes an array index** instead of an O(depth) chain of
  string-keyed hash lookups.
- **A real debugger** with step-into, backtraces, and stack watchpoints.
- **A viable async story** that doesn't tax the synchronous hot path.
- **Static stack-underflow detection** as a side effect of the effect analysis (§3.4).

### 2.4 How close, concretely

**Design: converged.** The architecture is coherent, the hard constraints are identified
from source rather than assumed, and the open questions are decision-shaped rather than
research-shaped.

**Evidence: zero.** Phase 0 has not happened. Every performance number in these four
documents is reasoning from source, not measurement. The ~180-byte value size is layout
arithmetic; the allocation counts are traced by hand; the expected speedups are
order-of-magnitude guesses. The single most valuable next action is not writing Bund2 code —
it is building the benchmark corpus and measuring the current interpreter, because that is
what every subsequent decision gate is measured against.

**Code: zero.** Fair estimate for Phases 1–3 (value, symbols, IR, Tier 0, 357 words,
differential harness) is a substantial multiple of the current 13.3k library LOC. Phase 5
adds perhaps 3–5k for a narrow Cranelift slice.

---

## Part 3 — What else would help, that hasn't been discussed

Ordered by expected value.

### 3.1 Value semantics under `Rc` — a correctness issue, not a performance one

**This is the most important item here and it is a bug waiting to happen.**

Today `Value` is deep-cloned everywhere. That means Bund has **value semantics** and cyclic
structures are impossible by construction. Naively switching heap payloads to `Rc` gives
**reference semantics** — `dup` on a LIST would produce two names for one list, and
mutating through one would be visible through the other. That is a silent behaviour change,
and it also makes cycles constructible for the first time, which `Rc` leaks.

**Fix: `Rc::make_mut`.** Clone-on-write. Reads share; the first write clones. This
preserves today's value semantics *exactly*, keeps cycles impossible, and still eliminates
the deep clones on the read-only paths that dominate (`get_lambda`, `times`, argument
passing). It costs one refcount check per mutation.

Write this into the Phase 1 RFC explicitly. It is easy to get wrong by omission and hard to
diagnose afterwards.

### 3.2 Inline caches

Not yet discussed, and standard for exactly this language shape.

Calls go through slots guarded by a generation counter. That is precisely the structure an
inline cache wants:

- **Monomorphic cache at each call site:** remember the resolved target plus the generation
  it was resolved at; on hit, compare one `u32` and branch. Turns an indirect call plus a
  slot load into a predicted direct call.
- **Polymorphic cache for OOP dispatch:** key on class id, cache 2–4 entries per `m()` site.
  Combined with the flattened vtables (§2.1), method dispatch approaches static-language
  cost.

Works at both tiers — Tier 0 gets it as a per-IR-op cache field, Tier 1 as emitted code.

### 3.3 Type feedback from Tier 0 into Tier 1

The main study proposed guard-and-branch specialization with both arms compiled, since
Cranelift has no deopt. That's correct, but with no ordering information the guards are
guesses.

Tier 0 already visits every arithmetic and comparison site. Have it record a two-entry tag
histogram per site — a few bytes, one increment. Tier 1 then orders the guard arms by
observed frequency, so the common case is the fall-through. Cheap, and it recovers most of
what a deopt-capable JIT gets from speculation.

The same table serialises as the profile for profile-guided AOT.

### 3.4 Infer stack effects; don't annotate 357 words

Earlier documents proposed hand-annotating every word. That is a large, error-prone,
perpetually-stale task. Better split:

- **Annotate the native words only** — they are opaque, so declaration is unavoidable.
- **Infer effects for Bund-defined words** by abstract interpretation over their IR. This
  is a small analysis and it composes: a word's effect follows from its callees'.
- **Inference also covers runtime-built lambdas**, which cannot be hand-annotated at all
  and are exactly the case §3.1 of the metaprogramming addendum cares about.

Then surface it: a `?effect` word and an effect column in the debugger's `words` view.
Concatenative programmers reason in stack effects constantly and currently have to do it in
their heads.

### 3.5 Static stack-underflow checking as a user-facing linter

Once abstract stack depth is computed, underflow becomes detectable before execution. In a
concatenative language this is *the* characteristic bug class, and today it surfaces as
`"Stack is too shallow for inline …"` at runtime with no source position.

`bund2 check` reporting underflow, unreachable words, and arity mismatches at a source span
is probably the highest developer-experience return in the whole project, and it is a free
by-product of analysis you are building anyway for codegen.

### 3.6 Superinstructions, mined rather than hand-picked

Tier 0 is now mandatory and always present, so its dispatch cost matters permanently — and
the classic Forth answer is superinstructions: fuse frequent word sequences into single
dispatch units.

The interesting version: **don't pick them by hand, mine them.** Instrument Tier 0 to emit
an execution trace of word symbols, then run frequent-sequence mining over it to find the
n-grams worth fusing, and generate the fused handlers. This is squarely FP-Growth /
sequential-pattern territory, which is existing expertise here rather than new ground.

Two payoffs: a measurably faster mandatory tier, and the mined n-grams are also the right
inlining candidates for Tier 1.

### 3.7 Image-based startup

Bund already has `save.lambdas`, `save.aliases`, `save.stacks`, `save.model`. That is
two-thirds of an image system.

Once Phase 3 makes the whole VM state reified data, `bund2 image` can snapshot an
initialised VM — stdlib registered, user libraries loaded, classes built, vtables flattened
— and `bund2 run --image` starts from it. For a 357-word stdlib with a 13-class hierarchy,
startup is not trivial, and this removes it entirely. It composes with AOT rather than
competing: the image supplies the world, the AOT object supplies the code.

The SBCL/Smalltalk precedent is well-trodden, and Bund's existing `save.*` words mean users
already think in these terms.

### 3.8 Differential fuzzing across the three tiers

Tier 0, Tier 1 and AOT must agree exactly. Three implementations of one semantics is the
classic setting for divergence bugs that hand-written tests miss.

`bund.pest` gives you a grammar, so grammar-directed program generation is straightforward.
Generate, run under all three, compare final state of every stack plus the workbench plus
the error. This is higher-leverage than any additional hand-written test and it is the only
practical defence against the Tier-0/Tier-1 divergence risk flagged in main study §5.

### 3.9 Tail calls for self-recursive words

`CallConv::Tail` is supported on x86-64 / aarch64 / riscv64. Concatenative code leans on
recursion, and a self-recursive word in tail position should become a loop rather than a
frame. Cheap to implement once the IR exists, and it removes a real stack-depth limit —
complementing, not replacing, the flat-frame-stack fix at Tier 0.

### 3.10 Escape analysis for anonymous lambdas

`10 { … } times` creates a lambda that is consumed immediately and never stored. If it
provably doesn't escape, skip the `Rc`, skip the content hash, and inline the body directly
as CLIF blocks. This is the single most common lambda shape in real concatenative code, and
it is a straightforward analysis on a linear IR.

### 3.11 Structured errors with spans

Errors are currently `bail!` format strings composed through call chains, with no position.
BundIR carries spans; errors should carry them too. Given `?try` and `?error` already exist
as words, an error *value* flowing on the stack fits the language better than a Rust
`Result` threaded through everything — and it is closer to how concatenative languages
usually handle failure.

### 3.12 Benchmark corpus as a committed artifact

Every decision gate in the plan is phrased as a measurement. That only works if the
measurement is reproducible and versioned. A `benches/` corpus committed to the repo —
arithmetic loop, lambda call, OOP dispatch, string work, list pipeline, metaprogramming
round-trip, realistic program — with results tracked per commit is infrastructure, not
overhead. Build it in Phase 0, before anything else.

---

## Part 4 — If only three things happen next

1. **Phase 0: build the benchmark corpus and measure the current interpreter.** Confirm
   `size_of::<Value>()`, count the allocations empirically, get the numbers that every
   later gate is measured against. Nothing else is assessable without this.
2. **Write the Phase 1 RFC on `BundValue`**, and make sure it settles all four decisions
   that landed there across these documents: 24-byte layout, lazy `.id`, sampled
   `.timestamp`, and `Rc::make_mut` value semantics (§3.1).
3. **Decide the `bund.eval` tier policy** — permanently Tier 0, or eligible for tier-up.
   It bounds the code-memory problem and it changes how conservative everything downstream
   has to be.
