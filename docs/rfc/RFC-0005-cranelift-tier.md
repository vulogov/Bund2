# RFC-0005: Tier 1 — the Cranelift backend

- Status: **Draft** (2026-09-08, revised 2026-09-09, 2026-09-10 and 2026-09-11).
  `docs/research/00-jit-feasibility.md` §2.2 sets a hard gate — "Project B is
  worth doing only if Project A's measurements show that dispatch and boxing
  are still the bottleneck". When this was drafted the gate did **not** pass:
  the dominant cost was one function in the value layer. **D41 changed that**,
  and §S1's update records it — push/pull 126.9 → 9.8 ns, a 3.8–4.2×
  improvement per program shape.

  **Half the gate is met and half is not, and this RFC does not claim
  otherwise.** The 20 ns prerequisite is met. The second — that dispatch is now
  the dominant term — **cannot be shown with these benchmarks**, because none
  of them separates dispatching a word from the work the word does once
  dispatched (§S1). Criterion 10 makes that an experiment that can fail rather
  than a claim.

  So §S2–§S11 are worth designing and are not yet authorised to be built. Two
  correctness problems stand between this RFC and Proposed. **§S8's frame
  consumption** now has a mechanism — a stack floor Bund2 measures in Rust and
  compiled code compares against its own stack pointer (§S8, *How the guard
  reads the stack*), read by the seventh and eighth reviews.
  And **§S6's inlining freezes a name** unless every inlined site re-checks
  what the name means — the fifth review's B1, answered in §S6 by a per-site
  meaning guard that criteria 5 and 17 test, and reviewed since the sixth.

  The sixth review's B1 — **no body's `Rc` reached the point where it starts
  running** — was the owner's, and is decided and built: D42 carries the value
  to every entry point, with dated amendments to RFC-0002 and RFC-0003. D43
  decides the `bund2-api` additions §S6's guards need, to be built when this
  RFC reaches Proposed.

  The seventh review's two blockers are answered. **B1** was the owner's:
  D44 decides that the level at which evaluation reports stack exhaustion is
  not part of a program's meaning, so §S8 now promises that evaluation never
  aborts and that Tier 0 never has less room with the tier on. It no longer
  promises that the level is the same. **B2** was this RFC's: §S5's pre-call
  generation check now applies §S6's rules, so nothing stays promoted across a
  call through an alias. That review's S1 went to the owner too. D45 makes the
  reporter answer `wants_stack` per severity, which lets values be promoted
  across calls under the CLI's default reporter. D45 is built.

  The eighth review's two blockers are answered. **B1** was the owner's: a
  lambda's effect is inferred from slots the pre-call check cannot pin, so
  D46 keeps nothing promoted across a call that resolves to a lambda. **B2**
  was a Bund2 defect: `execute.` declared a fixed effect while running
  whatever it was handed (F87, fixed). Criterion 24 now checks every native
  for that mechanically, and criterion 25 checks that no native reports at
  `Error` severity.

  The ninth review's three blockers are answered. The owner chose each answer
  among options the review set out. **B1**: a call can leave a body for
  the loop to run after it returns. Compiled code now drains that request after
  every call, or hands it to its caller in tail position (§S5, *A call may leave
  a body to run*). **B2**: `object` ran a class's `.init` under a fixed effect,
  and `display` ran a `fmt` runner (F91). Criterion 24 is now an audit of every
  native the corpus calls, and its first run found ten more declared pairs that
  miscount the stack (F92). **B3**: promotion crosses only the natives
  `bund2-stdlib` registered (D47).

  The ninth review's significant items are answered too, again by the owner's
  choice among the review's options:
  - **S1:** callees are classified by `Registry::resolve`, and `effect_of` now
    follows it (F93).
  - **S2:** the tier adds nothing to a Tier 0 level below the Tier 1 floor,
    and Tier 0's part carries a measured margin (§S8).
  - **S3:** `bund2` declares a Tier 1 share, and criterion 2 must see compiled
    bodies.
  - **S4:** the effect audit also catches a native reporting at `Error`.

  The tenth review's two blockers are answered, both by the owner's choice
  among the review's options. **B1**: most declared pairs had been checked only
  on the operands the corpus passes, and `drop_stack`'s was wrong (F94). A
  palette audit now runs every fixed-effect native against fourteen operand
  kinds and the workbench, and promotion crosses only the natives it brought to
  `Ok`, listed in `tests/golden/PROMOTABLE.txt` (D48, criterion 28). **B2**:
  a `NativeFn` cannot sit in a call slot. §S8 now specifies the boundary: a
  context pointer and a status under `CallConv::Tail`, a Rust adapter per
  native, `Tail` thunks and an entry trampoline. A panic in a native is caught
  where the native is called, in both tiers (D49), which fixes F95. Its
  significant items are answered in §S5 and §S8: D48's id set, a margin over
  every re-entering path, and the request protocol's four edges, one of them a
  Tier 0 defect now fixed (F96).

  The eleventh review's blocker is answered, with its significant items.
  **B1**: D52 makes `bund.exit` record a code and return `Ok`, and Tier 0
  stops at its next step; compiled code, which reads three cells after a call
  and none of them an exit, would have run on. A recorded exit now becomes the
  error status at the call that made it, and the entry trampoline refuses to
  start a body (§S5, *A call may end the program*; criterion 30). **S1**:
  §S8's re-entering paths are a derived set rather than a list, and
  `eval_source` is a path of its own; Tier 0's exhaustion message now names
  `bund.eval` and `use`. **S2–S4**: five stale sentences are corrected, every
  figure is re-derived on 2026-09-11 beside its command, criterion 28 states
  the palette's six unrun natives and that its list certifies the default
  registration, and §S7 cites *Q22 (cache)*.

  **Criterion 10 has a first measurement**, from a throwaway lowering outside
  this RFC's gate (2026-09-11, branch `spike/lowering-1`). With §S8's call
  boundary on every word and nothing inlined, `1 2 + drop` compiled runs
  **1.93×** faster than Tier 0, against the criterion's floor of 1.2×. The
  question this Status line opened with — whether the tier can earn its keep —
  now has evidence on the side of yes. The criterion itself stays open until
  the tier as shipped is measured.
- Depends on: RFC-0001 (the value, whose representation §S1 indicts),
  RFC-0002 (`StackEffect`, the word slot table, and the open world that forces
  indirect calls), RFC-0003 (BundIR as a cache over a body, and the frame
  loop), RFC-0004 (declared and inferred effects, which order guards)
- Decisions consumed: **D3** (eval'd code is structurally unable to hit the
  cache, so no tier rule is needed), **D5** (lambda bodies are write-once, so
  no invalidation), **D9** as amended 2026-09-09 (**no Cranelift type may
  appear in `bund2-stdlib` or `bund2-api`**; the original sentence governs the
  stable ABI, not where Bund2's own lowerings live), **D10** (a C toolchain is permitted to `bund2 build`, not below
  it), **D12** (the `*` fold family is a permanent optimisation barrier),
  **D16** (dispatch by computed name — the open world), **D20**
  (materialisation points), **D32** as amended 2026-09-10 on Q35's answer (`q` is
  kept on every value and **not** averaged by arithmetic; §S6's constraint 2
  rests on it),
  **D33** (**OPEN**; §S6 states what it withholds
  rather than taking its default), **D35** (the cache keys on the body's `Rc`
  pointer; as amended 2026-09-10 on Q32, it holds a `Weak`), **D42** (a body's
  `Rc` reaches the point where it starts running), **D43** (the registration
  id and stable generation cells §S6's guards need), **D44** (the level at
  which stack exhaustion is reported is not meaning), **D45** (a reporter
  wants a stack snapshot per severity), **D46** (nothing stays promoted
  across a call that resolves to a lambda), **D47** (promotion crosses only
  the natives `bund2-stdlib` registered), **D48** (and only those a palette
  audit has brought to `Ok`), **D52** (`bund.exit` is a request the
  embedder honours; compiled code must see it at the call that made it — §S5,
  *A call may end the program*), **D55** (the words that read beyond their
  arity are found by audit and kept off `PROMOTABLE.txt`, so they are
  barriers — Q34's answer; criterion 14), **D49** (a panic in a native is caught where the
  native is called, in both tiers), **D37** (no panic), **D39** (an
  internal loop must be bounded)
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- Supersedes: `00-jit-feasibility.md` §2.1's expectation table, whose rows are
  replaced by measurement in §S1. The document's own framing anticipated this —
  "order-of-magnitude reasoning, to be replaced by measurement in Phase 0" — so
  this is the substitution it asked for, not a contradiction. Recorded in
  `docs/research/ERRATA.md`.

## Summary

Tier 1 compiles a lambda body to native code through Cranelift, behind a
`cargo` feature, keyed on the body's `Rc` pointer, entered only through
runtime-owned indirect slots, and specialised by guard-and-branch with no
deoptimisation. It changes speed and not meaning: `cargo xtask conform` must
move by **exactly zero**.

That is the design. §S1's measurement no longer says "not yet" on performance
grounds — D41 removed that objection. Its two correctness mechanisms, §S8's
stack floor and §S6's meaning guard, are designed and have been reviewed: the
meaning guard since the sixth review, the floor since the seventh. Only Tier
0's half of the floor is built. Each review has found the next level of one
question, what a promoted value's callee can do, and the RFC stays Draft until
a review finds none.

## Motivation

Bund2 is already **5.5× faster than the reference** over the corpus — 153.9 ms
against 845.2 ms for 57 programs, measured through `cargo xtask bench` with
both targets in release (`docs/registers/open-questions.md`, Q14). That factor is startup: the fastest program fell from
13.7 ms to 2.3 ms, which is what D28's dependency cut bought.

So the motivation for a JIT is not "Bund2 is slow relative to the oracle". It
is that interpretation itself is slow in absolute terms, and the question this
RFC has to answer first is *which part* of it.

## Current behaviour

There is no Tier 1 *code*: `crates/bund2-jit/src/lib.rs` is a doc comment and
lint configuration — twelve lines — and `crates/bund2-ir/src/lib.rs` now
declares `pub mod fragment` and re-exports `Fragment`, `Guard` and `Op`: the
representation §S6 describes, and no lowering.

**The wiring, however, already exists**, and an earlier draft said it did not.
Cranelift is pinned at `=0.135.0` in the workspace and declared as optional
dependencies of `bund2-jit` behind `jit` and `aot` features
— the `jit` and `aot` features in `crates/bund2-jit/Cargo.toml`, which
`crates/bund2-runtime/Cargo.toml` re-exports. So the feature gate §S10 requires
is built; what is missing is everything it would gate.

Tier 0 is what runs: `Interp::eval` walks a `Vec<BundValue>` and dispatches
each `CALL` through the slot table.

---

# S1. The gate, measured — half met

`00-jit-feasibility.md` §2.2 divides the work in two and puts a decision gate
between them:

> Project A — representation. […] Delivers the majority of the achievable
> performance. […] Project B — Cranelift tier. […] Project B is worth doing
> only if Project A's measurements show that dispatch and boxing are still the
> bottleneck.

and warns what happens if the gate is skipped:

> A JIT that emits `call stdlib_add_inline` in sequence is a slower, more
> fragile version of the interpreter.

`crates/bund2-bench` now measures this in process. Medians, this machine,
release with debug info:

| benchmark | median | **per group** |
|---|---|---|
| `dispatch/literal_push/w2000` — `1 drop` ×1000 | 188 µs | **188 ns** |
| `dispatch/dup_drop/w3000` — `1 dup drop` ×1000 | 366 µs | **366 ns** |
| `dispatch/native_call/w4000` — `1 2 + drop` ×1000 | 446 µs | **446 ns** |

**Per *group*, not per word.** An earlier draft divided by word count and then
compared the result against `value/push_pull/balanced`, which produced an
impossibility: a whole average word read 112 ns while a single push/pull round
trip read 126.9. A word is not one push/pull — `1 2 + drop` is four words but
three pushes and three pulls — and the two benchmarks do not share a harness,
so the units were never commensurable. **Only differences within the
`dispatch/*` family are meaningful**, and this table is restated in the unit
that supports them.

What the value-layer benchmarks below do establish is the cost of the
operations themselves, which is a claim about `with_tag` and not about any
split between dispatch and the value layer:

| benchmark | median |
|---|---|
| `value/clone/scalar` — clone a `BundValue::Int` | **4.1 ns** |
| `value/with_tag/scalar`\* — one `with_tag("stack", …)` | **73.1 ns** |
| `value/push_pull/balanced` — one push and one pull through `Vm` | **126.9 ns** |

\* **No benchmark of that name exists today.** `crates/bund2-bench` has
`value/with_tag/scalar_unique` — the production case, a freshly boxed scalar
with one holder — and `value/with_tag/heap_shared`. Whether 73.1 ns came from
one of them under an earlier name cannot now be established: the bench crate is
not yet under version control, so it has no history to consult. The figure is
kept because this subsection records the state that motivated the gate; it is
not a figure anything later rests on.

**`with_tag` was 58% of a push/pull round trip and roughly 60–75% of an average
word.** It was not incidental. `Stack::push_as` did not store the value it was
handed, it stored `v.with_tag("stack", name)` — and `with_tag`, *as it stood
then*, boxed the scalar, materialised its identity, cloned the entire
`HeapValue` including its `BTreeMap` of tags, inserted two freshly allocated
`String`s, and wrapped the result in a new `Rc`. Four allocations and a map
clone, per value, per push. Cloning the same value cost 4.1 ns; tagging it cost
73.1.

**No line is cited for that, deliberately: the code no longer exists.** D41 and
RFC-0001's Q25 amendment rewrote it, and `crates/bund2-value/src/lib.rs`
today takes `Rc<str>` parameters — no `String` allocation — and mints and clones
only on the shared arm, which the production path does not take. An earlier
draft cited that line for the sentence above, which `cargo xtask cite` passed
because the line resolves; it checks that a citation points somewhere, not that
the prose describes what is there. **This whole subsection is a record of the
state that motivated the gate**, and it is kept in the past tense for that
reason. The current numbers are in the update below.

That tag is not decoration — a value's `tags` carry `stack: <name>` and the
reference's own values do too, which is why goldens capture it. The cost is in
*how* it is written, not *that* it is written.

### What this means for this RFC

A Cranelift tier lowers the interpreter's dispatch, and the question is how
much of a run that is.

**An earlier draft answered "at most a quarter" and derived an Amdahl bound of
about 1.3×. That figure is withdrawn.** It rested on the unit error above, and
its inputs were superseded by D41 the same week. It is not replaced with a
corrected number, because **these benchmarks cannot separate dispatch from the
work a word does once dispatched** — removing dispatch would not remove `+`'s
addition or `drop`'s pop, and nothing here measures that split. Criterion 10
makes it an experiment rather than an estimate.

What survives without arithmetic: `with_tag` cost 73.1 ns against a 4.1 ns
clone, on a path that runs for every value the interpreter touches. That is a
representation cost, it is large, and it is not something a code generator
addresses. The study's own precondition — that the representation work be done
first — was therefore not met.

**So the gate's answer is: not yet.** Concretely, the following must land
before §S2–§S11 are worth implementing:

1. **`push` must stop reallocating.** The stack tag is a property of *where a
   value is*, and it is being stored *in* the value. Candidate fixes — a tag
   written only when observed, a stack-name interning so the insert is a `u32`,
   or moving the tag to the stack's own bookkeeping — are RFC-0001's to weigh,
   not this RFC's. The requirement here is a number: `value/push_pull/balanced`
   under **20 ns**.
2. **Re-run the gate.** With push cheap, `dispatch/*` re-measured tells us
   whether dispatch has become the bottleneck. If an average word is then
   dominated by the dispatch loop, Project B is justified and this RFC's design
   sections apply unchanged.

This is not a rejection of Tier 1. It is the sequencing the study asked for,
with the measurement it said to take.

### Update, 2026-09-08 — prerequisite 1 met, prerequisite 2 not settled

Prerequisite 1 is satisfied; prerequisite 2 is not settled (below). **D41** moved the stack tag into the
value's existing padding as an interned symbol, and RFC-0001's Q25 amendment
records the chain:

Per group, the unit the `dispatch/*` family supports:

| program | baseline | **now** | |
|---|---|---|---|
| `1 drop` | 188 ns | **46.9 ns** | 4.0× |
| `1 dup drop` | 366 ns | **95.8 ns** | 3.8× |
| `1 2 + drop` | 446 ns | **105.2 ns** | 4.2× |
| `value/push_pull/balanced` | 126.9 ns | **9.8 ns** | 12.9× |

**Prerequisite 1 is met**: it asked for under 20 ns and `push_pull/balanced` is
9.8.

**Prerequisite 2 is not settled, and this update no longer claims it is.** It
asked whether dispatch has become the bottleneck. Differencing *within* the
family — the only subtraction these numbers support — gives:

| | cost |
|---|---|
| a literal push, `dispatch/literal_only/w1000` — **no dispatch at all** | **13.2 ns** |
| `drop` = `1 drop` − `1` | 33.7 ns |
| `dup` = `1 dup drop` − `1 drop` | 48.9 ns |
| `+` = `1 2 + drop` − `1 drop` − `1` | 45.1 ns |

A literal push runs no word and costs 13.2 ns; a `drop` adds 33.7 for a
dispatch plus a `VecDeque::pop_back`. So dispatch is **at most** 33.7 ns and
plainly the larger term in a word — but "at most" is the honest quantifier,
because nothing here separates the dispatch from the pop, or from `+`'s
addition. That separation is criterion 10.

`dispatch/literal_only/w1000` exists because of this: it is the non-dispatch path
exactly, and it replaced a `nl`-based benchmark that was meant to isolate
dispatch and in fact measured stdout at ~450 ns per word.

Conformance did not move across D41 — 73/86, ceiling 79/86, before and after
— and `BundValue` is still 16 bytes (a test in `crates/bund2-value/src/lib.rs`
asserts `size_of::<BundValue>() == 16`). Conformance has since reached its ceiling,
**105/113** on 2026-09-11, through Tier 0 work unrelated to this RFC
(criterion 2). It read 79/86 until three probes were added on 2026-09-10,
82/89 until a fourth, an approved deviation (D50), and 82/90 until that day's
words and probes.

So §S2–§S11 are worth designing. The Status line says what still stands
between them and being built.

# S2. What Tier 1 is, and the one invariant

Tier 1 compiles a **lambda body** to native code. It is never required: Tier 0
interprets every body and must, because a body built and run once can never
repay compilation (RFC-0003 §S3).

**The invariant is that conformance moves by exactly zero.** CLAUDE.md states
it for this milestone specifically — "the JIT and AOT milestones must move it
by exactly zero: they change speed, not meaning, so any movement is a bug."
This RFC adds no word, changes no word's behaviour, and adds nothing to the
conformance denominator.

# S3. The compilation unit, and the cache

**D35 resolves the key: the body's `Rc` pointer.** RFC-0003 §S3 states the
shape, and D42 carries the key to every point where a stored body starts
running. **`Vm::scoped_call` is the exception.** D42 left it taking a `Vec`,
which it wraps as a LIST built on each call (`Interp::scoped_call`,
`crates/bund2-interp/src/lib.rs`), so a `context` body has no key and never
reaches the tier. That is not a meaning risk, since such a body runs at Tier 0
as it does today. It is a body this RFC does not compile.
Two consequences this RFC owns:

- **A freed body's address may be reused**, so an entry that outlives its body
  would become a *false hit* — wrong code executed, the worst failure
  available here. **A `Weak` prevents it**: an `Rc`'s allocation is freed only
  when its strong and weak counts both reach zero, so a live entry keeps the
  address out of reuse. D35 first required a strong reference for this; its
  amendment (Q32) moves the cache to a `Weak`, since the strong reference was
  buying liveness rather than safety, and D42's frames now supply the
  liveness. D35's "an invariant to test, not merely to document" stands, and
  criterion 3 tests it.
- **The cache does not pin bodies.** D35 first said it did, and that the cap
  was therefore load-bearing for heap; with a `Weak` it pins nothing, and the
  cap bounds code memory alone (§S7).

**An eval'd token stream is never a compilation unit.** It is parsed and each
token applied straight into the VM, retaining nothing (D3), so there is no body
and no `Rc` for the cache to key — under pointer keying as under identity
keying. That, not a threshold, is what D3's amended resolution rests on.

**A lambda *inside* eval'd code is an ordinary body.** `1000 { … } times`
evaluated from a string runs its inner body 1000 times under one `Rc` — D42
carries the key to the entry point — and compiles like any other. Each
re-evaluation of the string mints a new body and compiles it again, orphaning
the previous code, which §S4 says is never reclaimed. **Only the 1024-body cap
bounds that**, and a REPL re-evaluating such lines reaches it: after 1024,
Tier 1 is off for the rest of the process. That is Q27's REPL profile arriving
through D3's door. This RFC accepts it for v1 — bounding it is what the cap is
for — and names the eventual answer: content-hash keying, which D35 calls "a
strict upgrade" and defers, would let a re-evaluated line find its earlier
code.

# S4. Every inter-word call is indirect, through a runtime-owned slot

**D16 makes the world permanently open**: a call target may be a name assembled
at run time. `!` is the corpus's spelling of `execute`, and for `PTR | STRING |
CALL` it hands the name to `vm.call`
(`reference/rust_multistackvm/src/stdlib/execute.rs:26-30`).

## The chain, followed all the way

An earlier draft of this section cited `i`/`i_direct` and stopped there. That
was one call short, and it is the mistake CLAUDE.md names: `vm.call` does not
reach `i` directly — it wraps the name and applies it,
`self.apply(Value::call(name.clone(), Vec::new()))`
(`reference/rust_multistackvm/src/multistackvm_call.rs:8`). Everything below
happens *before* the inline table is consulted, and all of it is contract:

| # | step | source |
|---|---|---|
| 1 | an empty name bails | `reference/rust_multistackvm/src/multistackvm_apply.rs:13-14` |
| 2 | **`is_command` → `c(name)`**, ahead of everything else | `reference/rust_multistackvm/src/multistackvm_apply.rs:16-17` |
| 3 | **`autoadd`**, which does not call at all — see below | `reference/rust_multistackvm/src/multistackvm_apply.rs:19-27` |
| 4 | a leading `$` → `call_internal_word` | `reference/rust_multistackvm/src/multistackvm_apply.rs:33-34` |
| 5 | alias resolution | `reference/rust_multistackvm/src/multistackvm_apply.rs:39-40` |
| 6 | `is_lambda` → `lambda_eval` | `reference/rust_multistackvm/src/multistackvm_apply.rs:46-49` |
| 7 | otherwise `i(real_name)` | `reference/rust_multistackvm/src/multistackvm_apply.rs:59` |

Only at step 7 does the chain reach the inline tables, where `i` resolves
aliases **again** (`reference/rust_multistackvm/src/multistackvm_inline.rs:69-75`).
Then `i_direct` tries the VM's own table and falls through to the stack
layer's (`reference/rust_multistackvm/src/multistackvm_inline.rs:41-67`). D16 declares that order
contract.

*(Every path in this table is spelled in full because `cargo xtask cite` cannot
seed a scope from a bare filename, and a table after a blank line has none in
scope; written as `:16-17`, these were thirteen citations the tool could not
check. The fifth review found that, and opened all thirteen by hand.)*

For lowering, steps 2, 4, 5 and 6 mean a compiled call site cannot assume its
target is a native: the same name may be a command, a `$`-forced internal, an
alias, or a lambda, and which one is a run-time property.

**And "the resolved target" is ambiguous, because the two paths resolve to
different depths.** A plain name is resolved twice — once by `apply`
(`reference/rust_multistackvm/src/multistackvm_apply.rs:39-40`) and again by `i`
(`reference/rust_multistackvm/src/multistackvm_inline.rs:69-75`). A `$`-prefixed name is resolved **once**:
`call_internal_word` strips the sigil and calls `i` directly
(`reference/rust_multistackvm/src/multistackvm_call_internal_word.rs:7-8`),
skipping `apply`'s resolution.

On a one-deep alias chain the two agree, which is why this has not surfaced. On
a two-deep chain `a → b → c` they do not: `a` reaches `c`, `$a` reaches `b` —
because `get_alias` answers with one `name_mapping` lookup and never follows the
chain (`reference/rust_multistackvm/src/multistackvm_alias.rs:31-39`).
F26 already records that `$` does **not** bypass alias resolution — it skips
the lambda check only — and this is the finer consequence: it skips one *level*
of it. A slot keyed on "the resolved target" therefore needs to say which
resolution, and a compiled `$name` call must key on the one-level answer.

## Step 3 is the one that changes what a call means

**Under `autoadd`, a CALL is not a call.** `apply` pulls the value beneath and
appends the CALL to it — `self.stack.push(val.push(value))` — and never
dispatches (`reference/rust_multistackvm/src/multistackvm_apply.rs:19-27`); an empty stack is an error, not a
no-op. The flag is VM-wide and mutable, toggled by two words registered as
**commands**:

```rust reference/rust_multistackvm/src/stdlib/autoadd.rs:28
    let _ = vm.register_command(":".to_string(), stdlib_autoadd_enable_inline);
```

with `;` disabling it at `reference/rust_multistackvm/src/stdlib/autoadd.rs:29`. `autoadd.rs` gives no reason
for registering them as commands. **The inference** is that they have to be:
step 2 outranks step 3, and without that `;` could never turn the mode off
again, because under autoadd it would be collected rather than run.

Two consequences this RFC owns, and an earlier draft had neither:

- **Compiled code must guard on `autoadd` at entry** and decline to run when it
  is set, and **re-read it after every call it makes**, because a callee can
  turn it on. The mode changes what the reference does with **every** value
  it applies, not only calls. A CALL and a literal are appended into the value
  beneath, `self.stack.push(val.push(value))`
  (`reference/rust_multistackvm/src/multistackvm_apply.rs:22`, `reference/rust_multistackvm/src/multistackvm_apply.rs:92`). A CONTEXT value is
  **pushed as a value of its own** instead of switching stacks,
  `self.stack.push(value)` (`reference/rust_multistackvm/src/multistackvm_apply.rs:73`). It is not collected;
  F84 has this right, and an earlier revision said "collected". So a check at
  calls alone cannot honour the mode. §S5's residual path does: once the flag is seen set, every remaining
  value goes through `apply`. No OSR is involved; the residual path is part of
  the compiled function.
- **`:` and `;` are opaque sites** in the sense §S5 defines: after either, the
  meaning of every following call has changed, so promotion stops there. They
  are reachable by computed name through `!`, since step 2 tests `is_command`
  before any of the machinery D16 makes dynamic — so this cannot be decided
  statically.

**No corpus program uses `:` or `;` as a word.** An earlier revision counted
one, from a `grep` for either character standing alone. That grep matches two
files today, and neither is a use: every match in
`reference/Bund/examples/code_snippets/textexpression_demo.bund` is inside a
string literal, and the one in `tests/probes/workbench-variants.bund` is in a
comment. A `grep` cannot tell a word from a string or a comment, so it is not
evidence for a word count. `bund2 check`'s tokeniser could be. This agrees with
Bund2 binding neither word while `conform` sits at its ceiling. D16 still means
one may appear at run time.

Cranelift compounds this. `JITModule` has no per-function redefinition or
deallocation; `get_finalized_function`'s pointer is valid until
`free_memory(self)` consumes the whole module
(`00-jit-feasibility.md` §3.2a). Bund's `register`, `unregister` and `alias`
mutate the word table at run time, and Bund2 implements all three
(`register` and `unregister` in `crates/bund2-stdlib/src/values.rs`, `alias` in
`crates/bund2-stdlib/src/singles.rs`).

Therefore:

- **No compiled call is a direct relocation to a `FuncId`.** Every inter-word
  call loads a pointer from a runtime-owned slot and calls it indirectly.
- **A call is not complete when the callee returns.** A native, or the
  dispatch of a lambda, may leave a body for the loop to run afterwards. So
  every call is followed by what §S5, *A call may leave a body to run*,
  describes.
- **Redefining a word writes a new pointer into its slot.** The old code is
  orphaned; its pages are never reclaimed.
- **Unregistering rewrites the call slot to whatever the name now resolves
  to**, and writes a failing stub only when nothing does. Bund2's registry
  `Slot` holds six independent bindings (`crates/bund2-api/src/lib.rs`,
  `Slot`), and the `unregister` word clears the lambda alone
  (`crates/bund2-stdlib/src/values.rs`, `unregister`). So unregistering a
  lambda that shadowed a native **reveals the native**, and Tier 0 runs it; a
  stub there would diverge. The call slot itself is never freed, because
  compiled code holds its address.

**Two structures, one name each.** This RFC says *call slot* for the
runtime-owned cell a compiled call site loads its target from, and *registry
`Slot`* for `bund2-api`'s record of a name's six bindings and their generation.
There is one call slot per name, rewritten whenever that name's registry
`Slot` is touched. A name whose resolution passes through another `Slot` — an
alias — has its call slot point at a resolving trampoline that runs the full
chain through `dispatch`, so an alias is never cached and never fanned out.
When that chain ends at a lambda, `dispatch` files a request rather than
running it, and the request is drained like any call's (§S5).
That is the same rule §S6 applies when it refuses to inline through an alias,
so the two sections now describe one model.
- **Code memory grows monotonically for the process lifetime**, which is what
  §S7's caps exist to bound.
- Module rotation — fresh `JITModule`, re-JIT live words, `free_memory` the old
  one — needs a shadow stack to prove no orphaned frame is live. **Out of scope
  for v1**, as the study directs.

**§3.2d is *not* discharged by the rule above, and an earlier draft claimed it
was.** Calls on x86-64 use 32-bit relocations (±2 GB), and the study's
mitigation has two halves: reserve a contiguous region up front, and "route
**runtime-helper** calls through an indirection table so helper addresses are
never a relocation-range problem".

The slot rule covers **inter-word** calls. Runtime-helper calls are a different
set and a larger one: §S5's second rule syncs promoted values back to the real
stack *through helpers*, so every opaque site emits them, and §S6's promotion
emits them at every boundary. None of that goes through a word slot, because a
helper is not a word.

So this RFC owes §3.2d a second mechanism it does not yet specify: **a helper
table, addressed indirectly, distinct from the word slots.** The allocator half
is settled — `cranelift-jit`'s `ArenaMemoryProvider`, adopted on §3.2d's
recommendation and not on evidence of its own. The helper table is not, and it
is exactly the path §S5 depends on most.

**D16 also forecloses static devirtualisation of `!`.** Speculation behind a
guard with a full-resolution fallback is permitted, because that is speed and
not meaning — but the health metric must still move by zero.

# S5. Guard and branch; never guard and bail

Cranelift is a code generator, not a JIT runtime: **no deoptimisation, no
bailout metadata, no on-stack replacement** (§3.2b). For a dynamically typed
language this is *the* defining constraint, and it fixes the specialisation
strategy:

> Guard and branch to a compiled generic path. Never guard and bail out.

Every type-specialised region carries its generic counterpart in the same
function, reachable by a conditional branch. This costs code size and
forecloses V8-style speculation; it has no runtime metadata cost and cannot go
wrong at run time, which under D37 is the property that matters.

The absence of OSR has a second consequence: **do not plan to enter compiled
code mid-loop.** Compile at word granularity and rely on loop bodies being
separately compilable lambdas — which in Bund they are: `times`, `loop`, `map`
and `while` each test their operand with `is_type(LAMBDA)` before running it
(`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:13`, `reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:12`,
`reference/rust_multistackvm/src/stdlib/logic/map_fun.rs:12`, `reference/rust_multistackvm/src/stdlib/logic/while_fun.rs:11`), and each
refuses anything else outright — `times` with `TIMES: #1 parameter must be
lambda` (`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:38`), and `loop`, `map` and `while` the
same way (`reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:41`, `reference/rust_multistackvm/src/stdlib/logic/map_fun.rs:99`,
`reference/rust_multistackvm/src/stdlib/logic/while_fun.rs:39`).

**Bund2 now keeps what that needs — D42.** The reference's `times` passes
`lambda_val.clone()` to `lambda_eval` on every iteration
(`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:20`), so the same `Rc` arrives each time.
Bund2's `times` used to copy the body out once per call and run a slice
through a `Vm::eval_body` that took `&[BundValue]`, so no key reached the entry
point (the sixth review's B1). Under D42 `Vm::eval_lambda` takes the value and
the frame holds it, and `100 { drop } times` enters one body under one key a
hundred times, which `times_enters_one_body_under_one_key` asserts
(`crates/bund2-stdlib/src/seq.rs`).

**RFC-0004 orders the guards, and only its declared effects are trusted across
a call.** A compiled body's analysis reads each callee's effect when the body
is compiled. It classifies each callee with `Registry::resolve`, the order
dispatch uses, and reads an effect only when that answers `Native` or
`Command`. The effect is the one declared on that binding, which §S5's
pre-call check pins. `Registry::effect_of` has followed the same order since
F93 (`crates/bund2-api/src/lib.rs`), so it answers `None` for any name that
resolves to a lambda, including a lambda that shadows a native. Until then it
returned the shadowed native's effect, which the ninth review found (S1). A
lambda has no declared effect, and RFC-0004 §S3 infers its effect by
composition from every slot its body calls through. That inferred effect is never trusted across a
call, because nothing stays promoted across a call that resolves to a lambda
(D46; *What a promoted value must not change*, below). An absent effect is
`Opaque` (criterion 13). This is the RFC-0004 dependency doing real work
rather than nominal work.

## `Opaque` is not a type question, and takes the other rule

The rule above is about **types**: *is this operand an Int?* It has a generic
counterpart — the boxed arithmetic the interpreter would have run — so it can
branch to it.

**`Opaque` has no such counterpart.** It is a static claim about *stack depth*:
after `!`, the depth is unknown, and no path recovers it. Calling that "the
generic path" is a category error, and an earlier draft of this section made
it. So opacity takes a second rule:

> **Promotion stops at an opaque site.** Values held in `Variable`s are synced
> back to the real stack before the call, and everything after it runs through
> runtime helpers.

**An opaque site is one whose native says so, so every native that runs a body
must say so.** `execute.` declared `eff(1, 0)` while running whatever it was
handed, so promotion would have run straight across arbitrary code. The eighth
review found it by hand (F87, fixed). The ninth found `object`, which runs a
class's `.init` and which a lambda-only check could not reach, and `display`
(F91). **Criterion 24 is now an audit over every program `conform` runs.**
While a native with a fixed effect runs, it may not start a body, file a tail
request or dispatch a word, and it must move the current stack by its pair. Its
first run found ten more pairs that miscount the stack (F92). It covers the
natives the corpus calls, with the operands the corpus hands them, and claims no
more.

**The sync writes through the path `Stack::push` uses**, not a bare
`push_back`, so a synced value carries its D41 stack symbol. A bare write would
leave `StackSym::NONE` and render `tags: {}` where the oracle renders
`tags: {"stack": "main"}` (criterion 12).

"Everything after it runs through runtime helpers" has one exception, and it is
§S6's: an **inlined fragment** after an opaque site still runs inline, because
its per-site meaning guard re-reads exactly the state an opaque site can
change. Without that guard the exception would be unsound, and an earlier
revision of §S6 claimed it without one.

Control never leaves compiled code, so **no OSR is required** — which is the
same constraint the type rule obeys, applied consistently.

**RFC-0004 §S1 said something different, and is amended.** Its closing sentence
— "a `Fold` or an `Opaque` site bails to Tier 0" — was a forward-looking claim
about this RFC, written before it existed. Read as a run-time bail it needs the
OSR machinery §3.2b denies; read as whole-body exclusion it would refuse every body that branches or
loops — control flow is 15 of the 74 first stops that §S6's *Where promotion
stops* counts. RFC-0004's amendment of 2026-09-09 withdraws it and points here.
**This is not a deviation**: nothing a program does changes, and the health
metric must still move by exactly zero.

**D12 is untouched.** `Fold` remains a barrier that cannot be optimised across;
it simply no longer kills the body it appears in.

## After any call, three things may have changed — and one path answers all three

*Added 2026-09-10, answering the sixth review's B2 and S1.*

A compiled body calls words through call slots, and a called word can change
state the body relied on:

- **which stack is current.** `to_stack` and `to_current` declare `eff(1, 0)`,
  `stacks_left` and `stacks_right` declare `eff(0, 0)` and rotate the stack
  ring whose front *is* the current stack (`crates/bund2-stdlib/src/stack.rs`),
  `endcontext` declares `eff(0, 0)` and switches back
  (`crates/bund2-stdlib/src/conditional.rs`), and a conditional that runs its
  body on another stack does so through `Vm::scoped_call`;
- **what a later name means** — `register`, `unregister`, `alias`;
- **whether values are applied or collected** — `:` sets `autoadd`.

RFC-0004 classifies none of these; its effects count stack depth. Rather than
enumerate every word that can change them — a list that would be wrong the day
a word landed — **compiled code re-reads the state after every call it
makes**, from runtime-owned cells (§S6, *Addressing*): the current stack's
epoch, bumped on every change of current stack, and the `autoadd` flag. Name
meaning is re-read at each inlined site, which is where it matters (§S6), and
a slot call always loads its target fresh.

If either cell has changed, the body takes its **residual path**. It syncs every
promoted value to **the stack it was taken from** — recorded when the value
was promoted, not the stack current now — and then applies the rest of the
body's values one at a time through the runtime's `apply`, exactly as Tier 0
would. That is guard-and-branch, not guard-and-bail: the residual path is
compiled into the same function as the generic counterpart of everything after
the call, and control never leaves compiled code. No OSR.

The sixth review's example is `1 2 "s" to_stack +`. `to_stack` bumps the epoch;
the body syncs `1` and `2` back to `main` and applies `+` through the runtime
on `s`, which fails `Stack is too shallow for inline ADD()` — as Tier 0 and the
reference both do, checked 2026-09-10. Without the check, promotion would add
the two and push `3` onto `s`.

Three rules follow:

- **A CONTEXT literal is a static barrier.** It switches stacks with no call at
  all (`reference/rust_multistackvm/src/multistackvm_apply.rs:69-87`, and `Interp::apply_step`'s CONTEXT
  arm), so the lowering sees it, syncs before it, and applies it through the
  runtime.
- **A lowered op addresses the current stack as of that op**, never a stack
  resolved once at entry. `frag::run` already does, through `Interp::pull`. A
  lowering may cache the current stack only between calls, which is exactly
  the window in which the epoch cannot move.
- **Words that reach another stack by name** — `swap_in`, `rotate_stack_left`,
  `rotate_stack_right` — do not switch the current stack, so the epoch does
  not move. When the name *is* the current stack's, they read or reorder values
  promotion may be holding. That is Q34's shape, a word observing beyond its
  arity. D55's audit keeps such words off `PROMOTABLE.txt`, so compiled code
  syncs before them.

The cost is one load and compare per cell. After every call three cells are
read: the epoch, `autoadd` and the request cell (*A call may leave a body to
run*, below). Before a call across which values stay promoted, a fourth is
read: the callee's generation (*What a promoted value must not change*, below).
Criterion 17 bounds these per-call checks as well as the inlined sites' checks.
Criterion 21 checks the behaviour, and criterion 18 the `autoadd` half.

## A call may end the program

*Added 2026-09-11, answering the eleventh review's B1.*

D52 makes `bund.exit` a request. It records a code through `Vm::request_exit`
and returns `Ok` (`crates/bund2-stdlib/src/host.rs`, `bund_exit`); nothing
ends at that moment. Tier 0 stops at its **next step**: `Interp::exit_gate`
refuses at the top of `Interp::apply_step` and of `Vm::eval_lambda`, the
refusal unwinds whatever is running, and the top level treats it as the end
of the program rather than an error (`crates/bund2-interp/src/lib.rs`,
`exit_gate` and `eval_observed`). Every Tier 0 call passes `apply_step`, so
the gate sees the step after the exit, whatever made the call.

Compiled code has no such step. The three cells it reads after a call record
no exit, `bund_exit`'s `Ok` becomes the success status, and literal pushes and
inlined fragments never pass `apply_step`. So a compiled body would run on past
`exit`: `tests/probes/bund-exit.bund`, whose golden stops at `inside, before
exit` with exit code 3, would print `inside, after exit` under
`--jit-threshold 1`, and criterion 2 would move. A `bund.eval` or `use` whose
source calls `bund.exit` is covered already, since its next `Vm::apply` is
refused and returns an error. The direct call is not.

**The rule adds no cell. A recorded exit becomes the error status at the call
that recorded it:**
- the per-native adapter (§S8) checks `Vm::exit_requested` after the native
  returns, and on `Some` parks `exit_gate`'s error in the error slot and
  returns the error status, whatever the native returned;
- the resolving trampoline does the same after `dispatch`, which does not pass
  the gate either;
- the entry trampoline refuses to start a body once an exit is recorded, as
  `Vm::eval_lambda` does before its cache lookup;
- the frame loop's compiled entry sits behind `apply_step`'s gate.

Compiled code's existing error path does the rest. It syncs every promoted
value and returns (*What a promoted value must not change*, below), and in
tail position the status travels with the return, so no body runs another op
after the exit. At the top the embedder reads `exit_requested` and ends the
program as Tier 0's does. The cost is one load and compare inside the adapter,
which is Rust already, and nothing in compiled code. The alternative, an exit
cell read after every call, would be a fourth per-call check under criterion
17, and in tail position the entry would have to read it instead of the body.
The status costs less and cannot be skipped.

A fixed-effect native may not request an exit: the effect audit records one
that does, as it records a tail request (`Interp::request_exit`). Only
`bund.exit` asks, and it is opaque. Criterion 30 checks the rule. **Is
`exit_gate` the only state Tier 0 gates per step rather than returns per
call?** Today it is: it is the only gate in `apply_step`. A second would need
the same treatment, and assumption 19 says so.

## A call may leave a body to run

*Added 2026-09-10, answering the ninth review's B1. The repository owner chose
this among the options the review set out.*

Tier 0 does not always run a body when it is called. `Vm::tail_lambda` is
`Interp::request_tail`, which files the body and returns: "Ask the loop to run
`body` **after the current native returns**" (`Interp::request_tail`,
`crates/bund2-interp/src/lib.rs`). Only `Interp::take_pending` pushes its
frame, and Tier 0 calls it straight after each value it applies
(`Interp::run_to`, `Interp::apply`). Five kinds of call file a request:
- `!` on a lambda (`execute_value`, `crates/bund2-stdlib/src/values.rs`);
- `if.stack`, when the stack it names is the current one (`if_stack`,
  `crates/bund2-stdlib/src/control.rs`; it declares `opaque(2)`, and the
  eleventh review found it missing here);
- `if` and `if.false`;
- `ifthenelse` (`crates/bund2-stdlib/src/control.rs`);
- every call to a lambda by name (`Interp::dispatch`, its `Resolved::Lambda`
  arm).

So a request means *run this before the next value*.
`{ 10 } ! 20`, `true { 10 } if 20` and `:f { 10 } register f 20` each leave
`10` beneath `20` (checked 2026-09-10).

A compiled call through a slot gets control back before the body has run. If
the compiled code went on, the body would run after the next value. And since
`request_tail` assigns, a later request could overwrite it, and it would never
run. So:

- **Non-tail: the request is state, read after every call.** A **request
  cell** (§S6, *Addressing*) is set by `request_tail` and cleared by
  `take_pending`. After every call, compiled code loads it beside the epoch and
  `autoadd`. If it is set, compiled code calls a drain helper that does what
  `Interp::apply` does after `apply_step`: `take_pending`, then `run_to` down to
  the frame count it found. That happens before the next op. Nothing decides in
  advance which words file requests, which D16 would leave open anyway.
- **Tail: the request is handed to the caller.** A body's last call does not
  drain. The compiled function returns with the request pending, and each entry
  takes it at once: the frame loop before its next `apply_step`, and
  `Vm::eval_lambda` before it returns. So a self-recursive word whose last word
  is a lambda call goes back to the loop, and recurses on the heap as RFC-0003's
  frame loop does.
- **A request is never lost.** A slot call that starts while the request cell
  is set returns `Error::internal`, naming the invariant: some earlier call's
  request was neither drained nor handed back. Tier 0 cannot reach that state
  since F96, because `Interp::invoke` clears a request when the native that
  filed it fails. Before F96 an embedder's native could leave one behind, and
  Tier 0 ran it late (the tenth review's S3.2). Under these rules compiled code
  cannot reach it either.

The drain helper runs a body synchronously, so it is a third place a body
starts, beside the frame loop and `Vm::eval_lambda` (§S8, *A decline is a
return*). It checks the Tier 0 floor before it re-enters, as `Vm::eval_lambda`
does. A body it starts is entered like any other: compiled if the cache holds it
and the Tier 1 floor allows, and declined otherwise.

**Three edges, settled** for the tenth review's S3:
- **The epoch and `autoadd` are read after the drain.** A drained body can
  switch the current stack, set `autoadd` or `register` a name, so the loads
  that decide the residual path come after it, not beside the request cell.
- **A refused drain clears the cell.** If the Tier 0 floor refuses the drain,
  the helper clears the request before it returns the exhaustion error, as
  `Interp::invoke` clears one a failing native filed (F96). No stale request
  outlives an error.
- **Where the frame loop enters compiled code.** `push_frame` stays the one
  place a body starts. It records the body's key in `entry_log` and in §S7's
  counter, and the frame carries its exit action. The loop calls the compiled
  function when it takes that frame's first step, and pops the frame, running
  its exit action, when the function returns, whichever tier ran the body. So a
  compiled entry is counted and observed exactly as an interpreted one.

This is independent of promotion. D46 syncs before a lambda call, and §S5 syncs
before `!`. After the sync, the call still has to run its body in order, and
this is what makes it do so. Criterion 26 checks it.

## What a promoted value must not change

*Added 2026-09-10, clearing items reviews had carried since the fourth: the
stack a report shows, identity and timestamps, diagnostics from compiled code,
and a fold word that arrives at run time.*

**An error's report shows the stack Tier 0 would have shown.** An uncaught
error is reported after it has returned to the top: the report's stack
snapshot is taken when `Vm::report` is called (`Interp::report`), and the
`[BUND]  Content of the stack` dump is printed after it
(`crates/bund2-stdlib/src/report.rs`). So a compiled body **syncs every
promoted value, each to the stack it came from, before it returns an error** —
whether the error is its own or a callee's — exactly as the residual path does.
By the time anything reads the stack, nothing is left in a register.

**A native that reports mid-body may read the whole stack.** `Interp::report`
takes a snapshot when the reporter wants one for the diagnostic's severity, so
a native that calls `Vm::report` while values below its arity are held in
registers would show a short stack. That is Q34's shape, a word observing
beyond its arity, and the rule is structural: **while the reporter wants a
snapshot for a severity natives report mid-body, `Warning` or `Notice`, no
value stays promoted across a call.** Natives return errors rather than
reporting them. That was a convention (D45) until criterion 25 made it a
test: no shipped code in `bund2-stdlib` reports at `Error` severity. Promotion
crosses no other crate's natives (D47), so a native from outside
`bund2-stdlib` that reports at `Error` never holds a value it cannot see. The
embedder's fatal report comes after evaluation has returned, by which time
every error path has synced.

**The reporter is read at each compiled body's entry.** It is not fixed when
the `Interp` is built, which an earlier revision claimed. The CLI replaces it
afterwards (`run`, `crates/bund2-cli/src/main.rs`), and the field is public.
What does hold is narrower: no `Vm` method reaches the reporter, so it cannot
change while a compiled body runs.

**In the default configuration the rule withholds nothing.** D45 made
`wants_stack` take the severity, and the CLI's `TextReporter` wants a snapshot
only for a fatal report, the only kind under which it renders one
(`TextReporter::wants_stack`, `crates/bund2-stdlib/src/report.rs`). So under
`bund2 script`, with or without `--no-dump-stack`, values are promoted across
calls. A reporter that wants mid-body snapshots gets exact ones and no
promotion across calls: `CollectingReporter` with `wants_stack` set, or a TUI
that shows the stack beside a warning. The seventh review found that the
earlier rule, which read `wants_stack` without a severity, meant no promotion
across any call in any `bund2 script` run, and §S6's promotion ceilings were
figures for a configuration nobody runs by default.

**`.id` and `.timestamp` need no rule of their own.** Both are lazy. An
identity is minted on first *need* — `.id`, equality, ordering, hashing or
serialisation (D1) — and a stamp is sampled when it is first observed, not at
construction (D2). A promoted value is observed only after it has been synced,
so its stamp is sampled at the same observation as in Tier 0.

**Its identity may be minted in a different order, and that is not meaning.**
Tier 0 also mints where nothing observes the result. `Stack::push_as` tags a
shared heap value through `with_tag`, and `with_tag`'s shared arm materialises
identity before it splits (`BundValue::with_tag`,
`crates/bund2-value/src/lib.rs`). `mint()` is a process-wide counter in the
same file. A promoted heap value skips that push, so it would be minted later,
and in a different order relative to other values. An earlier revision said
it is "minted at the same program point", which holds for scalars only. The
rule that survives a wider guard is that ids are opaque, and F14 normalises
them in every golden, so mint order is not meaning. Today the case cannot
arise, because `Guard::TopAreInt` admits unboxed scalars only.

The obligation falls on fragments instead: an arm that compares, orders or hashes
must materialise identity wherever its word does. Today's fragments compare
nothing, and scalars compare by content (D30). Criterion 16's differential
test, which already asserts `dup`'s fresh identity, is where a wider arm has to
show it.

**Compiled code emits no diagnostic of its own.** Errors are returned values
(§S11). Warnings and notices come from natives, which compiled code reaches
through their call slots, so they are emitted exactly as in Tier 0. And a
fragment *cannot* emit one: `Op` has no reporting operation. So an arm that
reports — `while`'s warning at ten million iterations, say — is not fragment
material and stays a call.

**A fold, or any change of effect, arriving at run time.** Promotion across a
call keeps the values below the callee's arity in registers, and it trusts the
callee's effect *as it was at compile time*. A `register` or `alias` can later
rebind that name to a word with a different effect — a fold that consumes the
whole stack, or simply one that consumes more — and then the promoted values
would be invisible to it. So before every call **across which any value stays
promoted**, compiled code compares the callee's generation cell (§S6,
*Addressing*) against the one it was compiled with. If it has changed, the body
syncs everything and takes the residual path from that call onward. That is one
load and compare per such call, and it is what "detected at run time" means for
D12.

**That check reads one slot, so it holds only for a call that resolves through
one.** This is the seventh review's B2, answered by carrying over §S6's three
inlining rules:

- **A direct resolution only.** A name that is an alias resolves through two
  registry `Slot`s, and each writer touches only the slot it writes
  (`Registry::register_alias`, `Registry::register_lambda`,
  `crates/bund2-api/src/lib.rs`). The stdlib's own aliases show the failure.
  `<-` and `←` alias `stacks_left`, `->` and `→` alias `stacks_right`
  (`stacks_left` in `crates/bund2-stdlib/src/stack.rs`), and `?` aliases
  `conditional` (`crates/bund2-stdlib/src/conditional.rs`). If `:stacks_left { … } register`
  binds a lambda that consumes two values, a call to `<-` reaches that lambda,
  and `<-`'s own generation has not moved. **So nothing stays promoted across a
  call through an alias.** The body syncs before it, as at an opaque site. A
  name that is direct at compile time and made an alias later is caught,
  because `register_alias` touches that name's own slot and the check fails.
- **`$name` reads its own name's slot, under the same rule.** `$` skips the
  lambda check and resolves one level of alias (§S4, *The chain*). On a direct
  name that level is the name itself, so the check reads that name's slot. On
  an alias, the body syncs before the call. The effect trusted is the
  **native** binding's, since `$` skips the lambda. A lambda registered on the
  same name bumps that slot's generation, so the check still fails when one
  arrives.
- **Never against a saturated slot.** `Slot::touch` stops at `u32::MAX`
  (`crates/bund2-api/src/lib.rs`), so a check compiled against a saturated
  generation can never fail. A callee whose slot is saturated at compile time
  is treated as an alias is: the body syncs before the call.
- **The third inlining rule, that the registration is recognised, belongs to
  inlining alone.** It asks whether a fragment belongs to the binding in the
  slot, and D43's registration id answers it. The pre-call check trusts only
  an effect. For a native, the generation pins the binding that effect was read
  from, because a native's effect is declared in its own slot. That is true of
  natives only, and the next rule is why.
- **Never across a call that resolves to a lambda (D46).** A lambda's effect
  is inferred from every slot its body calls through, so pinning the lambda's
  own slot pins nothing. The eighth review gave two programs. With
  `:g { drop } register  :f { g } register`, a body `1 2 3 f` promotes `1` and
  `2` across `f`, which infers as `(1, 0)`; then `:g { drop drop drop }
  register` leaves `f`'s slot untouched, and `g` pulls three values from a
  stack holding one. And `:f { :g { drop drop drop } register g } register`
  rebinds `g` during the call, where no check made before it can look. So the
  body syncs before any call whose name resolves to a lambda at compile time,
  as at an opaque site. A name that resolves to a native at compile time and
  gets a lambda later is caught by the generation, because `register_lambda`
  touches that name's slot.
- **Only across the natives an audit has brought to `Ok` (D47, D48).**
  Promotion crosses a call only when the callee's registration id (D43) is one
  `bund2-stdlib`'s `register_all` minted, *and* the native is listed in
  `tests/golden/PROMOTABLE.txt`, which criterion 28's palette keeps.
  `register_all` records the ids it mints in the `Registry`, a replay (F32)
  re-mints and re-records them, and `bund2-jit` reads that set when it
  compiles, as it reads the fragment table. An earlier revision called the set
  "the same set its fragments are keyed by", but the fragment table holds three
  ids (the tenth review's S1). A command carries no D43 id, so no command is
  ever crossed; `bund2-stdlib` registers none. Every other native is synced
  before, as at an opaque site: an embedder's, an external package's under D9,
  or a `bund2-stdlib` native no audit brought to `Ok`. That costs speed, never
  meaning. F87, F91, F92 and F94 showed that each check was needed. Criteria 27
  and 28 check it.

This is CLAUDE.md's "follow the call one level further". The check read the
call slot and stopped there, while dispatch went on to the target. The eighth
review took the same question one level further again, from the target's slot
into the target's body. Criterion 22 checks both, the alias case and the
lambda case.

# S6. Stack-slot promotion — the actual win, and what withholds it

§2.1's only row **that needs Cranelift** and promises more than ~1.5× is
"stack-slot promotion + type guards — ~2–5×". The other three rows above that
figure — two "large multiple"s and the ~2× threaded interpreter — are all in
the "Needs Cranelift? **no**" column, which is §2.2's whole argument and the
reason the qualification matters. An earlier draft dropped it and claimed this
was the *only* such row.

The idea: a compiled body's intermediate values live in Cranelift `Variable`s
rather than round-tripping through the `Stack`, so a sequence like `1 2 +`
never materialises a `BundValue` at all.

Promotion is valuable *precisely because* touching the stack is expensive — and
**D41 made it much less expensive**, from 126.9 ns per push/pull to 9.8. So
promotion's headroom shrank with the thing that motivated it, and the ~2–5×
figure was written against the old cost. **The two are not additive and must
not be claimed as such**; criterion 10 measures what is left.

## Promotion needs inlining, and §S8 is what makes that necessary

§S8's escape from §3.2f is that a compiled word's signature is uniform —
`fn(ctx: i64) -> i32` under `CallConv::Tail`, §S8's call boundary — **because
operands travel on the shared VM stack rather than in the call**. That answer is correct and it has a consequence this section
originally did not face: *a value held in a Cranelift `Variable` is invisible to
every word.*

So `1 2 +` skips materialising a `BundValue` only if `+` is **inlined** into the
compiled body. If `+` is *called* — through a slot, with the uniform signature —
it reads its operands off the real stack, and they have to be there. **Promotion
across a word therefore requires a lowering for that word, not a call to it.**

### The apparent trap, and why it is not one

`Intrinsic` is named by D9 and appears nowhere in `crates/`. D9 concludes that
it "stays internal to `bund2-stdlib`", and RFC-0000 criterion **B3** requires
that `cargo tree -p bund2-stdlib` not list `bund2-jit`. Read together those
seem to put the lowering where the code generator cannot reach it.

Two readings are wrong there, and D9's amendment of 2026-09-09 records both.
**The forbidden direction is `stdlib → jit`; `jit → stdlib` is permitted** and
nothing in RFC-0000 says otherwise. And D9's subject is the *stable ABI* —
whether external packages may ship CLIF — not where Bund2's own lowerings live.
The constraint that actually binds is narrower: **no Cranelift type may appear
in `bund2-stdlib` or `bund2-api`.**

### The mechanism: a word publishes BundIR, not CLIF

A word may carry an optional **BundIR fragment**. `bund2-jit` lowers BundIR to
CLIF on its own side of the boundary; `bund2-stdlib` mentions no Cranelift type
and gains no Cranelift dependency, so B3 and D9 stand exactly as written.
`bund2-ir` is already a dependency of `bund2-jit`.

Three constraints, and the second is what makes this affordable:

1. **The fragment is BundIR, never CLIF.** The permitted dependency direction
   is `bund2-jit → bund2-stdlib`, and criterion 15 asserts it so a later change
   cannot invert it quietly.

2. **A fragment specialises one arm and always has the word as its generic
   branch.** It is not an alternative implementation of the word.

   This is what keeps fragments small enough to be worth writing. A fragment
   for `+` cannot restate `numeric_op` (`crates/bund2-stdlib/src/math.rs`),
   which handles int, float, mixed kinds, **LIST append**, string
   concatenation and division by zero. It does not have to: under §S5's
   existing guard-and-branch rule the fragment covers `Int + Int → Int` — four
   IR operations — and every other shape branches to the call.

   **What makes that arm free of `q` is the guard.** `Guard::TopAreInt` admits
   only *unboxed* scalars, whose `q` is the 100.0 every constructor writes, and
   the word's result is a fresh value at 100.0 too
   (`crates/bund2-stdlib/src/math.rs`'s header). An earlier revision said the
   fragment carried "D32's `q` average". It carries none: the reference's `+`
   calls `numeric_op` directly and never reaches the code that averages, and
   **D32, amended on Q35's answer, says Bund2 keeps `q` but does not average
   it.** So an arithmetic result is a fresh value at 100.0 whatever its
   operands carried, and `PushInt` produces exactly that. Widening a guard to
   boxed values therefore owes no `q` arithmetic; it owes the differential test
   an operand whose `q` is not 100.0, if one can be built, or the `q` assertion
   stays unable to fail.

3. **Every fragment carries a differential test** running the specialised arm
   and the generic arm on the same inputs and asserting equality — value,
   `dt`, `q`, D41's stack tag and, for `dup`, F13's fresh identity.
   Criterion 16. On today's integer domain the `q` assertion cannot fail
   (constraint 2); it is kept so that it can once a guard widens.

Because the generic path *is* the word, a fragment can only be wrong on the arm
it claims — which is what bounds the divergence risk that would otherwise make
this a second implementation of the language.

### Inlining freezes a name — unless every inlined site asks

*Added 2026-09-10, answering the fifth review's B1. An earlier revision called
inlining the half that carried no risk. It carries one.*

§S4 builds the call architecture on a single rule: no compiled call binds a
name at compile time; every call loads its target from a slot, so a redefined
word is seen by every caller. **An inlined fragment breaks that rule by a
different route.** Its ops *are* the body, and no slot is on the path. Once
`+`'s `Int + Int` arm is inlined, the compiled body adds two integers for as
long as it lives, whatever `+` has since become — and §S4's chain lists what it
can become: a lambda (`reference/rust_multistackvm/src/multistackvm_apply.rs:46-49`), an alias to something
else (`reference/rust_multistackvm/src/multistackvm_apply.rs:39-40`), a command
(`reference/rust_multistackvm/src/multistackvm_apply.rs:16-17`), or, under `autoadd`, not a call at all
(`reference/rust_multistackvm/src/multistackvm_apply.rs:19-27`).

**It is not confined to opaque sites.** `register`, `unregister` and `alias`
declare fixed effects; they are not `Opaque`. So under §S5 a compiled body runs
straight through one and then reaches an inlined `+` whose meaning the word
before it just changed. RFC-0004's `Opaque` classifies stack depth, and nothing
classifies name meaning. Q34 is the stack-observation case of a fixed effect
understating what a word does; this is the name-table case.

**The mechanism is a meaning guard at every inlined site.** After the type
guard admits and before the first op, compiled code checks two things against
live runtime state:

1. **The site's slot generation still equals the one captured when the
   fragment was inlined.** The registry `Slot` already carries this counter —
   documented for inline caches, none of which exists yet, so the meaning
   guard would be its first reader — and each writer of a binding §S4's chain
   consults bumps it:
   `register_native`, `register_lambda`, `register_alias`, `unregister_alias`,
   `register_command` and `unregister_lambda` all call `touch()`
   (`crates/bund2-api/src/lib.rs`). A lambda shadowing `+`, an alias
   retargeting it, a command registered under it and an unregister all change
   the generation of `+`'s slot.
2. **`autoadd` is clear.**

If either check fails, the site branches to the slot call — the generic path,
in exactly §S5's shape, with no bail and no OSR. Both checks are loads at the
site, not facts fixed at compile time, so the guard holds after an opaque site
and after a table mutator earlier in the same body. Mutators need no
classification at all.

Three rules make this sound rather than nearly sound:

- **Inline only on a direct resolution.** A fragment is inlined only when the
  site's name resolves through its *own* slot — no alias on the path, and no
  lambda or command binding in the slot — so there is one generation to guard.
  A name reached through an alias resolves through a second slot, whose
  rewrite would not touch the first. Such a site is called, not inlined.
  Aliases are rare on arithmetic, and this keeps the guard to one compare.
- **Never inline against a saturated slot.** `touch()` saturates at
  `u32::MAX` so that a stale inline cache cannot match a wrapped counter; a
  saturated slot "stops caching instead". The same rule applies here: a slot
  at `u32::MAX` keeps that value through every later rewrite, so a fragment
  inlined against it would never see one.
- **Recognise the registration — not the name, and not the address.** At
  compile time the JIT must confirm the slot holds *`bund2-stdlib`'s own* `+`,
  not another native registered under the same name, and a name cannot show
  that. A function address cannot either: Rust guarantees neither that two
  distinct functions have distinct addresses nor that one function has only
  one — `std::ptr::fn_addr_eq`'s documentation says so. So a `Native` needs an
  identity of its own — a **registration id**
  assigned by `Registry::register_native` — which `Native` does not carry
  today, and `bund2-stdlib` publishes its fragments keyed by the ids of the
  registrations it made. Ids are per `Registry` — a fresh `Interp` mints new
  ones, and F32's replay gives a re-registration a fresh id — so the table is
  built per registry, at registration time, never as a static.

**Where the association lives: `bund2-jit`.** `bund2-stdlib` publishes
`(registration id, Fragment)` pairs from `crate::fragments`, and `bund2-jit` —
which may depend on `bund2-stdlib` (D9 amended) — reads them when it compiles a
site. **`bund2-api` carries no `Fragment` type.** Its only addition is the
opaque registration id on `Native`, which names no code generator and no IR.
External packages therefore cannot publish fragments: D9's amendment gives them
`Native` with a declared effect and no more, and this keeps it so.

**What it costs**: two loads and two compares per inlined site, both
predictable — addressed as *Addressing* below describes. Criterion 17 checks that every inlined region carries the guard
and measures what it costs. Criterion 5, extended, checks that it works — for
each way §S4's chain lets a name change meaning, before the caller runs and
mid-body.

**Rejected: one VM-wide epoch**, bumped on any table write. It needs one
compare rather than two, but in a REPL, where `register` is routine, a single
unrelated definition would demote every inlined site in every compiled body
until each was recompiled.

### Addressing: where the guards' loads point

*Added 2026-09-10, answering the sixth review's S2. The costing above said "two
loads" without saying of what.*

Compiled code receives a context pointer that holds the `&mut dyn Vm` (§S8),
and `Vm` exposes no registry, no
generation and no `autoadd`. Nor does a generation have a stable address:
`Registry` keeps its slots in a `Vec<Slot>` that `slot_mut` grows with
`resize_with` whenever a new name is registered (`crates/bund2-api/src/lib.rs`,
`Registry::slot_mut`), so a pointer into it dangles after the next `register`.

So the guards read **runtime-owned cells at stable addresses**, not the
registry:

- a **generation cell per name**, mirrored from the registry `Slot`'s counter
  by the same `touch()` that bumps it, and allocated in fixed-size chunks that
  never move when more are added;
- one **`autoadd` cell** and one **current-stack epoch cell** (§S5), owned by
  the runtime in a single allocation that lives as long as the `Interp`;
- one **request cell** (§S5, *A call may leave a body to run*), mirroring
  whether a tail request is pending: set by `request_tail`, cleared by
  `take_pending`, and in the same allocation as the two above;
- one **stack-floor cell** per `Interp` (§S8), holding the floor that `Interp`
  took from its thread's declared region, which the check at every compiled
  body's entry compares `get_stack_pointer` against.

At compile time the JIT embeds each cell's address as an immediate. That is
safe to do because the cells outlive every compiled function: both die with the
runtime. A check is then one load and one compare per cell. That makes **two
per inlined site** (generation, `autoadd`) and **three after every call**
(epoch, `autoadd`, request). **A third comes before a call across which values stay promoted**:
the callee's generation (§S5), so generation cells are read at call sites as
well as at inlined sites. Each compiled body entry adds **one** (the floor).
Criterion 17 bounds the per-site and per-call costs alike. AOT cannot embed an
address; it would reach the cells through a relocated data symbol or a helper,
and AOT's lowering is RFC-0006's.

**"The runtime" is the `Interp`, and there is one of each per `Interp`**: one
compiled cache, one `JITModule`, one set of cells and one fragment table. The
floor cell's value is the one each `Interp` takes from its thread's declared
region when it is built. Values are `Rc` and `!Send`, but two `Interp`s on one
thread share them freely, as the tests and embedders do. A cache shared by the
thread would find code compiled against another `Interp`'s cells, or against
freed cells once that `Interp` dropped. With a cache per `Interp`, a body
another `Interp` compiled is a miss, and runs at Tier 0 or is compiled again
against this `Interp`'s cells. Registration ids are per `Registry` (D43), so
the fragment table could not be shared in any case. Criterion 23 checks this.

**The cells add no `unsafe`.** They are `Cell`s written by safe Rust, and
compiled code reads them through addresses it was handed as integers. The tier
needs `unsafe` in two places, both in `bund2-jit`: calling JIT-emitted code at
all, and the native adapter's dereference of its context pointer (§S8's call
boundary). An earlier revision said the first was the only one. That was
before the boundary was specified (the tenth review's B2).

**What this changes outside this RFC.** The generation mirror belongs to
`Registry`, which is `bund2-api`'s, so `Registry` grows an accessor that hands
out a name's cell. With the registration id above, that is an addition to
RFC-0002's surface — **decided as D43** (Q37), and built when this RFC reaches
Proposed.

**So both halves of this design carry risk.** Inlining carries the
*redefinition* risk, closed by this guard. Promotion carries the
*stack-visibility* risk, closed by Q34 and criteria 9, 12 and 14. The staging
below weighs both.

### Measured — three times, and the third reverses the second

Fragments were prototyped and measured before the rest were written, as this
section required. `crates/bund2-bench/benches/fragment.rs`, run with

    cargo bench -p bund2-bench --bench fragment

on 2026-09-10 — one run, this machine, release. There is no Cranelift lowering
to time, so it measures **ceilings**, and each column is constructed rather than
subtracted:

| column | what it is |
|---|---|
| `tier0` | the program interpreted, every word dispatched |
| `inlined` | the fragment executed by `frag::run` — the model, as it runs |
| `lowered` | the fragment's own ops written out in Rust: literal pushed and pulled, guard asked, nothing folded — **the ceiling for inlining** |
| `promoted` | intermediates held in a register — **the ceiling for inlining plus promotion** |

Per operation — Criterion's point estimate for 1000 operations, divided by
1000:

| | `tier0` | `inlined` | `lowered` | `promoted` |
|---|---|---|---|---|
| `Int + Int` | 59.2 ns | 44.7 ns | **29.2 ns** | **6.7 ns** |
| `dup drop` | 88.8 ns | 44.7 ns | **29.7 ns** | **6.9 ns** |

| | inlining alone, `tier0`/`lowered` | promotion on top, `lowered`/`promoted` | together |
|---|---|---|---|
| `Int + Int` | **2.0×** | **4.4×** | 8.8× |
| `dup drop` | **3.0×** | **4.3×** | 12.9× |

Run-to-run spread is about 2%: `int_add/tier0` read 57.9 and 59.2 ns in two
runs the same day, and the sixth review's re-run agreed with every ratio to
within 3%. The two `inlined` cells are both 44.7 ns, and equal in that re-run
too (46.3 ns). That is `frag::run`'s fixed cost — guard, op loop, register
file — dominating two arms that each do about one push's work; it is not a
copying error.

**The inlining ceiling on arithmetic is 2.0×, and promotion is the larger
multiplier.** Once an inlined arm's operands live on the real stack, the stack
traffic — the literal's push, then two pulls and a push per `+` — is most of
what is left, and removing that traffic is exactly what promotion does. The
prize is promotion. Inlining is what makes it possible (*Promotion needs
inlining*, above).

**The two earlier versions of this table were each wrong, in opposite
directions**, and both are recorded because both errors are easy to repeat:

- **2026-09-09: inlining 3.8–5.4×.** Its `inlined` column was hand-written as
  one pull and one push, with the literal `1` folded into the arm as a constant
  and no guard. No fragment can express that: `Op` has no immediate operand,
  and `fragments::int_add()` pops two values behind `Guard::TopAreInt(2)`. Its
  `dup drop` half also measured `push(clone)` rather than `dup`, which is not
  what the word does (F13); correcting that alone moved 5.4× to 4.8×. The fifth
  review found the folded constant (B2).
- **2026-09-10, first re-run: inlining 0.9× — slower than the word.** That
  column ran the real fragment through `frag::run`, which at the time
  collected the top of the stack into a `Vec` and heap-allocated its register
  file on every entry: `int_add/inlined` read 65.0 ns against `tier0`'s 57.9.
  Removing both allocations brought it to 44.7. That gap is the model's
  overhead, not inlining's — which is why `lowered` is a separate column.

The earlier revision drew its staging from the first of these: "inline first,
promotion second", because inlining was the larger win and every risk belonged
to promotion. **Both halves of that are withdrawn.** Inlining is the smaller
win, and it carries the redefinition risk the meaning guard above closes.

**So the staging is:** inlining is built first because promotion cannot exist
without it, not because it pays on its own. On operand-free arms — `dup drop`,
3.0× — it clears criterion 10's threshold by itself. On arithmetic — 2.03× —
it clears it by a hair before compiled code pays anything, and criterion 10
expects the lowering to fall under it. Promotion follows,
and criterion 10 is where it has to earn its machinery.

### What has been built, 2026-09-09 and 2026-09-10

The representation and its first consumer, which is everything on this side of
a code generator:

| | where | what it is |
|---|---|---|
| `Fragment`, `Guard`, `Op` | `crates/bund2-ir/src/fragment.rs` | the arm, naming no code generator |
| `frag::run` | `crates/bund2-interp/src/frag.rs` | executes one against the real stack, allocating nothing; `Ok(false)` only when the guard declines |
| `Fragment::new` | `crates/bund2-ir/src/fragment.rs` | the only constructor outside a test-only feature; refuses a fragment whose ops, walked typed against its guard, could fail after it admits |
| `Vm::peek_at` | `crates/bund2-api/src/lib.rs` | the top *n* without copying the stack — what a guard asks |
| `int_add`, `dup`, `drop_top` | `crates/bund2-stdlib/src/fragments.rs` | the two measured arms |
| criterion 16's differential test | same file | the arm against the word, over the arm's boundaries |

**Criterion 15 holds**: `cargo tree -p bund2-stdlib` lists neither `bund2-jit`
nor any `cranelift-*` crate, with `bund2-stdlib` now depending on `bund2-ir`.

`frag::run` is **not Tier 1** — it generates no code and caches nothing. It
exists so the representation is exercised rather than assumed, and so
criterion 16 can run before a lowering is written. **The remaining half is the
Cranelift consumer**, which is what `bund2-jit` is for and what this RFC still
has to be Accepted before anyone writes.

Four properties are asserted by test rather than by prose, because each fails
silently:

- **A declined guard leaves the stack untouched.** If it did not, the word that
  runs instead would see operands the program never pushed.
- **A fragment that could fail after its guard admits cannot be built**, and
  one that escapes anyway is an internal error, never a decline and never a
  silent success: by then the operands are gone (criterion 19).
- **The arm agrees with the word on value, `dt`, `q` and D41's stack tag**,
  none of which any golden prints on this path. On today's domain the `q` half
  cannot fail, and constraint 2 says why.
- **`dup`'s copy has its own identity**, in the arm and in the word (F13). The
  render comparison normalises identities away and could not see this; an
  earlier version of the test relied on it alone.

**What the ceilings do not include.** `lowered` and `promoted` call `Interp`
directly rather than through `&mut dyn Vm` or §S4's runtime helper table, and
compiled code pays entry, exit, a type guard and — per inlined site — a meaning
guard. Both columns are optimistic, `promoted` more so: its `dup drop` case is a
value kept alive in a register, which is what "compiles to nothing" looks like
in Rust. Criterion 10 measures the real thing; these say whether it can be worth
measuring. For promotion the answer is yes. For inlining alone on arithmetic it
is almost certainly not, and criterion 10's measurement decides.

### Where promotion stops — the corpus, counted

*Restored 2026-09-10. An earlier revision carried this table, lost it in an
edit, and kept five references to it; the fifth review found them pointing at
nothing — one of them in RFC-0004's accepted amendment.*

`bund2 check` reports where RFC-0004's analysis stops: the first stop on each
analysed path, not every opaque site (the re-derivations below say why).
Re-derive with:

    for f in $(find reference/Bund/examples reference/Bund/tests tests/probes \
                    -name '*.bund' -not -path '*/features/*'); do
      ./target/debug/bund2 check --file "$PWD/$f" | grep -E '^ +[0-9]+ +`'
    done

Re-derived on 2026-09-11, after that day's words and probes, across **189**
programs, it prints **137** sites (that morning it printed 124 over 165):

| why analysis stops | sites |
|---|---|
| the word's effect is not a fixed pair — `Opaque` or `Fold` | **116** |
| the name has no binding when analysed | **21** |

The 116, by word:
- `object` 25, `!` 17, `format` 14, `loop` 11, `times` 9, `if` 6, `graph!` 5,
  `clear` 5;
- two each of `math.interpolation`, `load.model`, `input` and `?true*`;
- one each of `unfold`, `**.`, `display`, `map`, `*+`, `?.`, `fold`, `pull.`,
  `notifthenelse`, `bund.exit`, `bund.eval`, `if.in_workbench`, `execute` and
  `*loop`;
- two user words, `HelloWorld` and `f`, whose inferred effects are opaque.

The probes `if-workbench-variants` and `stack-words-by-name` added the first
stops at `if.in_workbench` and `execute`. F94 added none, since no program
calls `drop_stack`, and neither did the `sysinfo-version` probe.

**Twenty-six are ordinary control flow**: `loop` 11, `times` 9 and `if` 6. That is
why whole-body exclusion, RFC-0004 §S1's original reading, would refuse nearly
every body that branches or loops.

**All 21 are names Bund2 does not register at all**: `classifier` 4,
`sample.analysis`, `internaldb.execute`, `global` and `console.spinner` 2 each,
and nine singletons from the postponed console, database and AI vocabulary.
`generator` and `cwd`, 24 of the morning's 48, are registered now. "No effect" there means *no binding at analysis time*, and D16 says
a binding may still arrive at run time; that is why criterion 13 treats an
absent effect as `Opaque` rather than as zero.

**What promotion can reach is the straight-line run before the first of these
sites in a body.** Inlining, behind the meaning guard above, is not bounded by
them.

The figures move as words land and effects are declared. The fourth review
counted 116, 61 and 55; since then `if` has gained a site and `#` has appeared.
That is why the command sits beside the numbers: a count with no derivation
beside it rots.

**Re-derived after F87, 2026-09-10: still 113, 63 and 50.** Making `execute.`
opaque did not move the count, although
`tests/probes/remaining-vocabulary.bund:52` and `:55` call `execute.` and
`!.`. `bund2 check` abandons that program at an earlier stack switch, and it
counts only where analysis *stops*, once per program path. A site after the
first stop is never reached. Stops at a stack switch also fall outside the
`grep` above, because that report names no word. So the table counts the first
word-named stop on each analysed path, not every opaque site in the corpus.
It is a lower bound, and the eighth review's "moves by at least two" assumed
the probe's calls were reached.

**Re-derived after F91 and F92, 2026-09-10: 122, 74 and 48**, from 113, 63
and 50. Making `object`, `format`, `clear`, `fold` and `display` opaque moved
first stops earlier on many paths:
- `object` became the commonest first stop, because construction usually comes
  before the first `!`;
- `!` fell from 32 to 15, and `if`, `loop` and `graph!` fell with it;
- two unbound names were no longer the first stop on their paths.

The shape is the one the paragraph above describes: the table counts first
stops, not every opaque site.

Three things bound promotion:

- **D12 — the `*` fold family is a permanent optimisation barrier.** `*+`, `**`
  and friends consume the whole stack, so the stack depth is not statically
  known across them. ERRATA records that the corpus uses none of them, so the
  barrier costs nothing measurable; it still has to be *represented*, because
  D16 means one may appear at run time. Bund2 registered none of the ten until
  2026-09-11 (`*.` is ordinary multiplication's workbench variant, not a fold).
  The eight arithmetic folds, with `Σ` and `Σ.`, and `*loop` and `*loop.` are
  now registered, and each declares `StackEffect::opaque`, since
  `StackEffect` has no separate fold kind. So a call to one stops promotion
  as an opaque site does. D12's "bails to Tier 0" is read as §S5's promotion stop (D12's dated
  note), and a fold bound at run time to a name a body was compiled against is
  caught by §S5's pre-call check.
- **`Opaque` effects**, counted in *Where promotion stops* above. Promotion
  stops; compilation does not.
- **D33 is OPEN.** Ordering across int and float currently answers **true to
  all four of `<`, `>`, `<=`, `>=` at once** (F47), which is not a machine
  representable order. A mixed-kind comparison therefore **cannot be lowered to
  a single machine compare** while D33 stands. This RFC does not take D33's
  default: it requires mixed-kind comparison to take the generic path, and
  notes that if D33 resolves to option 2 the lowering becomes available.

# S7. Tiering policy: threshold, cap, demotion — Q22 (cache), answered here

Q22 (cache) asks for the compiled-cache promotion threshold and cap and says RFC-0005
"must state them as load-bearing for D3, not as tuning". D35 first added
that the cap was load-bearing for heap as well, because the cache held strong
references to bodies; its amendment (Q32) moves the cache to a `Weak`, and the
cap now bounds code memory alone.

Stated once, for both:

| knob | value | why it is load-bearing |
|---|---|---|
| **promotion threshold** | 64 evaluations of one body | Below it, a body is interpreted. It is **not** what makes D3 true — an eval'd token stream is never a body at all (§S3). A lambda inside eval'd code *is* a body, can cross 64 within one evaluation, and is bounded only by the cap. An earlier revision said the threshold made D3 true by construction, and it does not |
| **compiled-function cap** | 1024 bodies | Code memory is never reclaimed (§S4). This is the only bound on it |
| **recompile cap** | 4 per slot | A word redefined in a REPL loop would otherwise orphan a function per redefinition |
| **demotion** | permanent, per body | A body that exceeds the recompile cap returns to Tier 0 and is never promoted again |
| **counter cap** | 4096 bodies | See below. The counter is a second structure and needs its own bound |
| **heap consequence** | neither structure pins a body: the cache and the counter both hold a `Weak` | D35 first required the cache's reference to be strong; its amendment (Q32) withdrew that |

These are **defaults with a stated basis**. The cap exists because
`free_memory` is all-or-nothing, and changing it changes a correctness argument,
not a benchmark. The threshold is the tuning knob of the two: it decides when a
body has earned compilation, and no correctness argument rests on it. Both are
configurable, and the configuration is recorded rather than silent.

## The counter's key and lifetime

An earlier draft named a threshold of "64 evaluations of one body" and said
nothing about what counts them. That is not a detail: the counter is a second
map over the same bodies as the cache, and the two obvious designs are both
wrong.

- **Key on the pointer, hold a strong `Rc`** — safe from address reuse, and it
  pins *every body ever evaluated*, not the 1024 that were compiled. The heap
  claim in the table above would be false and the growth unbounded.
- **Key on the pointer, hold nothing** — no pinning, and it inherits exactly
  the hazard §S3 guards against: a freed body's address is reused, the new body
  inherits a hot count, and it is compiled on its first evaluation.

**The counter holds a `Weak`.** That is not a compromise between the two; it
removes the dilemma, because of a property of `Rc` this RFC now depends on:
**the backing allocation is freed only when the strong *and* weak counts reach
zero** (the `std::rc` module's documentation of `Weak`). So a live `Weak` keeps the allocation — and therefore the address
`Rc::as_ptr` returns — out of circulation, while the body's *contents* are
dropped on the last strong reference. No pinning, and no reuse.

Their stale entries would fail differently, which is why the question of
strength was asked separately for each:

| | strength | a stale entry means |
|---|---|---|
| compiled cache | `Weak` (D35 as amended) | a dead entry that no longer upgrades — swept, and never a false hit, because its address cannot be reused while it lives |
| promotion counter | `Weak` | at worst a body compiled earlier than it earned. A performance mistake, never a wrong answer |

Both now hold a `Weak`, for the same reason: a `Weak` is enough to keep an
address out of reuse, and a strong reference would pin what nothing else
needs.

**And that argument cuts at D35, which this RFC should say rather than let a
reader notice.** D35's stated reason for the cache holding a *strong* reference
is that "if an entry outlives its body, the allocator may reuse the address and
a stale entry becomes a false hit". The paragraph above establishes that a
`Weak` prevents address reuse too — the allocation is not freed while one
lives. So **address safety is not what the strong reference buys**, and D35's
rationale as written is weaker than it appears.

What a strong reference does buy is separate and this RFC does not have the
standing to decide it: a demoted or evicted entry needs the *body* to fall back
to, and if the cache were the last holder, dropping the entry would drop the
lambda. Whether that is reachable — whether a body can be live for the cache
and dead for everything else — depends on how bodies are held elsewhere, which
is RFC-0003's territory.

**Settled 2026-09-10 (Q32, option A).** The owner amended D35: the cache
holds a `Weak`. The case above is why — address safety never needed a strong
reference — and D42 removes the other reason: every running frame now holds its
own clone of its body's `Rc`, so a body is alive whenever compiled code for it
can run. The key is unchanged.

**Lifetime.** An entry whose `Weak` no longer upgrades is dead and is evicted
on the next sweep. The map is capped at 4096; at the cap, the coldest entries
go first. A dead entry costs one `RcBox` header until it is swept — not the
body.

**D39 applies**: the sweep is an internal loop and is bounded on data already
taken. It runs over the map as it stands, never until a condition holds.

# S8. Tail calls, and why §3.2f does not take them away

`CallConv::Tail` with `return_call` / `return_call_indirect` is supported on
x86-64, aarch64 and riscv64 (§3.1); **s390x historically lacked it**, so the
lowering must degrade to an ordinary call there rather than assume it.

## The objection this section has to answer

§3.2f says dynamically typed languages want callee-side argument-count checks
and array-style access, and that the workaround — "passing `argc: usize,
argv: *mut Value`" — **defeats tail calls**. An earlier draft of this section
did not mention it, which left §S8 premised on something the study appears to
withdraw two pages later.

**The argument count does not apply to Bund, and the reason is structural.**
§3.2f is about passing *arguments*. A concatenative language passes none: a
word's operands are already on the VM's stack, which the callee shares. So no
signature here is variadic, and none becomes `argc/argv`.

**But a native's own signature is not a boundary compiled code can use, and an
earlier revision said it was** (the tenth review's B2):

```rust crates/bund2-api/src/lib.rs:61
pub type NativeFn = fn(&mut dyn Vm) -> Result<(), Error>;
```

That is Rust's unspecified ABI. `&mut dyn Vm` is two words whose layout is not
stable, and `Error` wraps a `String` (`crates/bund2-api/src/lib.rs`, `Error`),
so the return is not a status Cranelift can express. `cranelift-codegen`
0.135.0's `CallConv` has no Rust ABI (`src/isa/call_conv.rs`, `CallConv`).
`return_call` constrains the callee against the caller: the verifier requires
both to share a calling convention that supports tail calls, and to return the
same types. Parameters need not match (`src/verifier/mod.rs`,
`typecheck_tail_call`). Only `CallConv::Tail` supports tail calls
(`src/isa/call_conv.rs`, `supports_tail_calls`), and Rust cannot define a
`Tail` function. So the boundary is four pieces:

1. **One JIT signature**: `fn(ctx: i64) -> i32` under `CallConv::Tail`, a thin
   pointer to a per-call context and an integer status, `0` for success and `1`
   for an error. The context is a `bund2-jit` struct. It holds the
   `&mut dyn Vm` as a stored fat pointer, an error slot, and the cells of §S6's
   *Addressing*.
2. **A per-native adapter in Rust**: `extern "C" fn(ctx: *mut Ctx, native:
   usize) -> i32`. It rebuilds `&mut dyn Vm` from the context and calls the
   native's `NativeFn` through `bund2_api::catch_panic` (D49). It parks an
   `Err` in the error slot and returns the status. No panic unwinds out of it,
   so none reaches a compiled frame.
3. **A JIT-emitted `Tail` thunk for each native a call slot can hold.** The
   thunk makes an ordinary call to the adapter. A compiled body is already
   `Tail` and sits in its slot directly. So every slot target is `Tail`, and
   `return_call_indirect` is legal from any tail position: body to body, and
   body to a native's thunk.
4. **An entry trampoline**, JIT-emitted with the platform's C convention,
   through which Rust enters a compiled body: from the frame loop, from
   `Vm::eval_lambda` and from §S5's drain helper. It builds the context, calls
   the `Tail` body, and turns the status back into a `Result`.

The adapter's dereference of the context pointer is the second `unsafe` this
tier needs, beside calling JIT-emitted code at all (§S6, *Addressing*). Both
live in `bund2-jit`. The signature is uniform because operands do not travel
in it, which is what satisfies the verifier's rule. §S11's return-value
protocol is the other half: an error travels in the context, never as an
unwind. Criteria 4 and 29 check the boundary.

**What §3.2f does cost Bund is real, but it is not this.** "Force everything
onto the stack" lands here as §S5's rule — promoted values must be synced back
to the VM stack before an inter-word call, because the callee reads them there.
That is a promotion cost, measured by criterion 9, and it would exist whatever
calling convention Cranelift offered.

## What this RFC claims, and what it does not

Two different uses of tail calls sit behind §3.1's sentence "a word body can be
compiled as a chain of tail calls", and only one is claimed here:

- **Claimed: the last word of a body is in tail position.** Lowering it as
  `return_call_indirect` saves a frame per body call, and matters most for
  self-recursive words. RFC-0003's frame loop already makes Bund-level call
  depth cost heap rather than Rust stack, and `Vm::tail_lambda` exists for
  exactly these positions; a compiled body must preserve that property rather
  than reintroduce stack growth Tier 0 does not have. A request the last
  callee files goes back with the return, and the entry takes it at once
  (§S5, *A call may leave a body to run*).
- **Not claimed: threaded code**, in which every word tail-calls the next. That
  is a different compilation strategy with its own register-allocation and
  debugging consequences, and nothing in this RFC depends on it. §3.1's phrase
  describes it; this RFC does not adopt it.

The distinction matters because the second is what makes tail calls
load-bearing in other designs.

## And that is a correctness problem, not an optimisation

An earlier draft ended here with "if `CallConv::Tail` were withdrawn tomorrow
the design would lose a frame per call and nothing else". **That is wrong, and
it is the most serious defect this RFC has had.**

If only the last word of a body is a tail call, **every other inter-word call
consumes a machine frame** — and RFC-0003's flat frame loop consumes none.
That RFC's criterion 2 is not decorative: *"Bund call depth is bounded by heap,
not by the Rust stack"*, **Met** at a call depth of **100,000**, where before
the frame loop the same program aborted between 5,000 and 20,000 (RFC-0003,
acceptance criterion 2). It is cited by number because RFC-0003's amendments
append and move its lines. An earlier revision cited a line range, and after an
amendment moved the text that range pointed into another criterion.
`cite` could not tell, because the lines still existed.

So a self-recursive word that completes at Tier 0 **overflows the machine stack
once promoted**. Three things follow, and none of them is about speed:

- **It is a conformance change.** The program's observable behaviour differs
  between tiers, and CLAUDE.md requires this milestone to move the health
  metric by exactly zero.
- **It is an abort, which D37 forbids** outright. A stack overflow is precisely
  the failure D37 exists to prevent: the process dies, the user's stacks and
  word table die with it, and the trace names machine frames rather than a Bund
  word.
- **It silently un-meets an accepted criterion of another RFC.** RFC-0003's
  criterion 2 would fail with the `jit` feature on, and nothing in this RFC
  noticed.

### The guard

**Every compiled body checks machine-stack headroom on entry and declines if it
is low**, falling back to the interpreter for that call.

This is an *entry* guard, the same shape as §S4's `autoadd` guard and for the
same reason: there is no OSR to bail with mid-body (§3.2b), so the only safe
place to refuse is before the frame is taken. Declining recovers the RFC-0003
guarantee **for direct calls**, which Tier 0's frame loop runs flat: one
compiled frame is spent, and the recursion continues on the heap. It does not
for **native-mediated nesting**. `Vm::eval_lambda`, which `times`, `loop`,
`map`, the conditionals and the method paths use, pushes a frame and calls
`run_to` from inside the native — one Rust frame per nesting — so a compiled
body calling such a word, whose lambda calls a compiled body, alternates Rust
frames. The headroom threshold has to cover that, and criterion 11 gains a case
for it.

Tail calls reduce how often the guard fires; they do not replace it, because
they cover one call site per body. **Q28 sharpens accordingly**: on a target
where `CallConv::Tail` degrades, the guard fires sooner and more often, but
correctness does not depend on the platform.

### How the guard reads the stack — one floor, measured in Rust, compared in CLIF

*Added 2026-09-10. Earlier revisions required this check and gave it no
mechanism.*

Four ways of doing it were ruled out first:

- **Cranelift's own stack limit.** `Function::stack_limit` makes the prologue
  compare the stack pointer against a limit and **trap** on overflow — on x64,
  a `cmpq` against `rsp` followed by `TrapIf` with `TrapCode::STACK_OVERFLOW`
  (`cranelift-codegen` 0.135.0, `src/isa/x64/abi.rs`,
  `gen_stack_lower_bound_trap`). A trap is a hardware fault: with no signal
  handler it ends the process, which D37 forbids, and it is guard-and-bail in
  its hardest form (§S5). It takes its limit from a `VMContext` parameter
  (`src/machinst/abi.rs`, `generate_gv`). The JIT signature's context pointer
  could be declared as one, so that is no obstacle: the trap alone rules it
  out.
- **The `stacker` crate.** It measures remaining stack through `psm`, whose
  build script compiles assembly with a C toolchain. D10 forbids a C toolchain
  anywhere below `bund2 build`.
- **Asking the platform** for the current thread's stack bounds —
  `pthread_get_stackaddr_np` on macOS, `pthread_getattr_np` on Linux — is
  per-platform `unsafe` FFI for a number Bund2 can know without asking.
- **Counting frames instead of bytes.** A counter bounds calls, not bytes.
  Compiled frames differ in size from body to body, and the Rust frames between
  two compiled bodies differ from native to native, so a sound count needs a
  byte bound per frame that nothing provides.

**The mechanism: Bund2 owns the stack it runs on, so it knows where that stack
ends.**

1. **Evaluation runs on a thread Bund2 spawns, with a stack size it chooses**
   — `std::thread::Builder::stack_size`: the standard library, no crate, no
   `unsafe`, no C toolchain. `bund2` does this for every subcommand (`main`,
   `crates/bund2-cli/src/main.rs`). There is no REPL yet, and one would do the
   same. Nothing observable changes: standard output, standard error and the
   exit code pass straight through.
2. **At that thread's entry the runtime records the stack's top**: the address
   of a local in the entry function, taken with `std::ptr::addr_of!` and cast to
   an integer, which is safe Rust. Stacks grow downward on all four targets
   Cranelift supports. Each target's `gen_stack_lower_bound_trap` traps when
   the stack pointer falls *below* its limit (`cranelift-codegen` 0.135.0,
   `src/isa/x64/abi.rs`, `src/isa/aarch64/abi.rs`, `src/isa/riscv64/abi.rs`
   and `src/isa/s390x/abi.rs`), which is only a bound if the stack grows down.
   So the stack's end is the top minus the size.
3. **From those two numbers it computes two floors**, described next.
4. **A check is one comparison against a floor, and it branches — it never
   traps.** Tier 0 compares the address of a local in the function doing the
   check. Tier 1 compares CLIF's `get_stack_pointer`, which is lowered on x64,
   aarch64, s390x and riscv64 (`src/isa/*/lower.isle`, and riscv64's
   `inst.isle`), against the floor loaded from a runtime-owned cell (§S6,
   *Addressing*).

**Two floors, and what they promise (D44).**

- **The Tier 1 floor** sits one reserve above the bottom of Tier 1's share:
  `top − share + STACK_RESERVE`. A compiled body whose entry finds the stack
  pointer below it declines. The floor says where
  compiled frames may *start*, and it reserves no region for them. The stack
  is LIFO, and its outermost frames are always Tier 0's (`run_cli`,
  `Interp::eval`, the frame loop). So Tier 0 frames also sit above this floor,
  whenever a compiled body calls a native that re-enters evaluation.
- **The Tier 0 floor** sits one reserve above the stack's end. Below it, a
  native that would run a body synchronously reports a Bund-level error
  instead of nesting further.

The thread is sized at Tier 0's part plus Tier 1's share. Tier 0's part is
`EVAL_STACK`'s 8 MiB (`crates/bund2-cli/src/main.rs`). The main thread's stack
on this machine, 8176 KiB by `ulimit -s`, is the reason for that default,
since it is roughly what Tier 0 ran on before F85's fix. It is not the value.
Compiled frames start only above the Tier 1 floor. The reserve beneath that
floor holds the last compiled body entered and whatever it calls without
re-entering evaluation, so compiled frames never reach Tier 0's 8 MiB. **Tier 0's capacity with the tier on is
therefore never less than with it off**, and it is more whenever compiled
frames are not using the top part. A program whose native nesting fits today
still fits.

**Never less, and not the same.** How much more room Tier 0 gets depends on
what runs, so the level at which a program reports `machine stack exhausted`
can differ with the tier on. An earlier revision also required that level to
be *the same*, and no fixed floor gives both (the seventh review's B1). The
owner decided that the level is not meaning (D44). It already differs between
Bund2's own build profiles: the `loop` axis reports at level 10,923 in release
(`cargo xtask depth`) and 2,371 in dev (`cargo xtask depth --dev`). And the oracle aborts at every
depth (F85). What is meaning is that evaluation nesting never aborts, and that
the level with the tier on is never lower. Criterion 11 checks both.

**The reserve is for the frames between one check and the next.** It is not
for reporting the error, which an earlier revision said: by the time the
error is reported, the stack has unwound (`STACK_RESERVE`,
`crates/bund2-interp/src/lib.rs`). Each floor sits one reserve above the bottom
of its part. The Tier 0 floor is at `top − size + STACK_RESERVE`
(`tier0_floor`, same file). The Tier 1 floor is one reserve above the bottom of
Tier 1's share, so a compiled body entered just above it, and anything it calls
without re-entering evaluation, stays inside that share. Two things come out of
the reserve and are not measured, though both are small against 256 KiB:

- the thread-entry frames, because the declared top is `stack_marker()` called
  inside the spawned closure, below the true top;
- a guard page, on platforms where the size requested includes one.

The reserve rests on an assumption, stated under *What this design assumes*:
everything a leaf native does between two checks fits in it. A native that
recurses in Rust on the depth of its *data*, such as one rendering a deeply
nested value, is not bounded by a floor on evaluation, and this RFC claims
nothing about it.

Proposed defaults: 8 MiB for each part and a 256 KiB reserve. Under `jit`,
Tier 0's part also carries the margin `m` described below, measured by
criterion 11. Like §S7's knobs, these are defaults with a stated basis, and they
change only with a measurement behind the change.

**A decline is a return, not a call.** A compiled body is entered from the
three places a body starts:
- the frame loop, for a body a tail position handed back (`Vm::tail_lambda`);
- `Vm::eval_lambda`, for a native running one synchronously;
- since the ninth review, the drain helper that runs a body a compiled call
  asked for (§S5, *A call may leave a body to run*). The drain helper acts on a
  decline as `eval_lambda` does. Its entry check runs first. Below the Tier 1 floor the function
returns a *declined* status at once, before it touches the stack or holds any
promoted value, so its frame is gone by the time the caller acts on the
status. The frame loop then pushes the body as an interpreted frame and runs
it on the heap. `Vm::eval_lambda` runs it in the frame `eval_lambda` already
holds, exactly as it does with the tier off. A level beneath the floor
therefore costs what it costs with the tier off, plus whatever the feature
adds to `eval_lambda`'s own frame (a cache lookup). It never keeps a compiled
frame alive. Every compiled body entered after that point finds the stack
pointer still below the Tier 1 floor and declines in turn, so beneath the
floor nothing compiled runs, and the recursion continues on the heap. That
recovers RFC-0003's guarantee, and it never needs OSR.

**Room is bytes, and criterion 11 counts levels.** D44's second requirement is
about room: with the tier on, Tier 0 has at least its own part and at most
both parts. Criterion 11 measures levels, which are room divided by the bytes
one level spends. The two agree as long as a level costs no more than
(Tier 0's part + Tier 1's share) / Tier 0's part times what it costs with the
tier off, which is twice at the proposed 8 MiB each. A decline that returns
keeps the per-level cost almost unchanged, so the inequality has close to a
factor of two to spare. A decline that *called* `eval_lambda` would have added
a compiled prologue and a second `eval_lambda` frame to every level beneath
the floor, and the argument would then depend on frame sizes. That was the
eighth review's S2. Criterion 11 measures the result either way.

**When compiled frames already hold the share.** This is the ninth review's
S2, and the owner chose among its options. Take a program that recurses
directly through compiled code until it reaches the Tier 1 floor, and only
then nests through a native.
- **With the tier off**, the direct recursion is flat on the heap, and the
  nesting has Tier 0's whole part.
- **With the tier on**, the share is held by compiled frames, and the nesting
  has the same part.

So the factor of two above is not available to that nesting. Any byte the tier
adds to a Tier 0 level lowers its level below the tier-off one, and D44's "a
program whose native nesting fits … fits under the `jit` binary" would fail near
the limit. Two rules close it:

- **The tier adds nothing to a Tier 0 level below the floor.** The cache
  lookup at `Vm::eval_lambda` is a separate `#[inline(never)]` helper that
  returns before `run_to` is called. So no state the tier adds is live across
  the nesting, and `eval_lambda`'s own frame is the tier-off frame. The same
  holds for the compiled-entry test the loop makes at a frame's first step
  (§S5): it is a helper that returns before the loop goes on, so `run_to`'s own
  frame is the tier-off frame too. The Rust compiler still owns frame sizes,
  so this is a design rule and not a guarantee.
- **So Tier 0's part carries a margin under `jit`, over every re-entering
  path.** Let `c_p` be the bytes one level of path `p` spends with the tier
  off, and `δ_p` what it spends more beneath the floor with the tier on. The
  paths are every way a native re-enters evaluation, and **the set is derived,
  not listed**, because a list went stale within a day (the eleventh review's
  S1): every `bund2-stdlib` function that calls `Vm::eval_lambda`,
  `Vm::apply` or `Vm::scoped_call`, found by a source scan as criterion 25
  finds `Error` reports, plus §S5's drain helper. On 2026-09-11 that is
  `times`, `loop`, `map`, `while`, `for`, `*loop`, `input*`, the conditionals,
  `?try` and the method paths through `Vm::eval_lambda`, `context` through
  `Vm::scoped_call`, and **`eval_source`** — `bund.eval`, `!!` and `use` —
  through `Vm::apply`. `eval_source` is a path of its own: a Rust frame, a
  parse and a loop per level, so its `c_p` is not `Vm::apply`'s alone.
  Criterion 11 reports `c_p` and `δ_p` for each. Then Tier 0's part is
  `8 MiB + m`, with `m ≥ 8 MiB × max_p(δ_p / c_p)`, and the level with the tier
  on cannot be lower whatever holds the share. An earlier revision took `c` and
  `δ` from `loop` alone, and the native with the smallest `c` need not be
  `loop` (the tenth review's S2). `m` is a measurement taken when the
  tier exists, not a number guessed now. It costs address space rather than
  memory, because a thread's stack is committed as it is touched.

Criterion 11 has a case for it.

**§S5's residual path is bounded the same way.** It applies the rest of a body
through `Vm::apply`, which is synchronous. Each value there that runs a body
costs a Rust frame, as any native re-entering evaluation does. So recursion
that passes through residual paths spends machine stack. It stops spending it
at the Tier 1 floor, below which every compiled body declines and the
recursion continues on the heap. `Vm::apply` also checks the Tier 0 floor
before it re-enters (`Interp::apply`, `crates/bund2-interp/src/lib.rs`). The
worst case is therefore a Bund-level error, never an abort.

**Embedders.** A program that runs `Interp` on a thread Bund2 did not spawn
declares that thread's stack with `bund2_interp::set_stack_region` before
building the `Interp`. If it does not, the `Interp` assumes 1 MiB below the
point where it was built, and Tier 1 gets no share there, so compiled code does
not run on that thread. 1 MiB is half of Rust's default for a spawned thread,
since a constructor is rarely at the very top. `std::thread`'s module
documentation, *Stack size*, says "Currently, it is 2 MiB on all Tier-1
platforms". That was read in the 1.94.1 `rust-docs` HTML, and in the `stable`
toolchain's `rust-src` at `library/std/src/thread/mod.rs`, line 129. The
pinned 1.95.0 has neither component. The same section names the
`RUST_MIN_STACK` environment variable, at line 134, as changing that default.

**The arithmetic, for a thread that declares nothing.** The floor is
`marker − ASSUMED_BUDGET + STACK_RESERVE`, 768 KiB below the point where the
`Interp` is built (`tier0_floor`, `crates/bund2-interp/src/lib.rs`). The frames
between one check and the next may run up to one reserve past it. So the
undeclared case is safe only while at least 1 MiB of stack lies below the
constructor. On a default 2 MiB thread that leaves 1 MiB for whatever sits
above the constructor. An embedder that sets `RUST_MIN_STACK` below 1 MiB plus
that depth, or builds the `Interp` deep in its own call stack, must declare its
region.

**A region declared with `set_stack_region` is Tier 0's alone.** The function
takes a top and a size, and nothing in it says how much of that is Tier 1's.
So under `jit`, such a region gets no Tier 1 share, as an undeclared thread
gets none, and compiled code does not run there.

**A share is declared, never inferred.** This is the ninth review's S3, and the
owner chose among its options. `bund2-interp` gains
`set_stack_region_with_share(top, size, share)`, which names the part of the
region that is Tier 1's. `bund2`'s own thread declares through it under `jit`,
so the tier runs in the CLI. As written before, `main`'s `set_stack_region`
call would have kept compiled code out of `bund2` itself, and criterion 2 would
have passed with nothing compiled. An embedder opts in the same way, and one
that does not keeps all of its region for Tier 0. A default split was rejected:
it would take room from every embedder's Tier 0 without asking, against D44's
"never less". The function is added with the tier.

**What it costs.** Tier 1 pays a stack-pointer read, a load and a compare per
compiled body entry, under criterion 11's 2 ns bound. Tier 0 pays an address, a
load and a compare each time a native re-enters evaluation — `Vm::eval_lambda`,
`Vm::apply` and `Vm::scoped_call` — calls that already push a frame and run a
loop.

### Tier 0 needs the same floor, and needs it now — F85

`:f { 1 { f } times } register` followed by `f` aborts Bund2 today, and the
oracle with it: exit 134, `thread 'main' has overflowed its stack` (F85).
RFC-0003's frame loop makes *direct* recursion cost heap, but recursion through
a native that runs its body synchronously spends a Rust frame per level (*The
guard*, above). That is a D37 violation with no tier involved at all.

The Tier 0 floor is its fix. `Vm::eval_lambda`, `Vm::apply` and
`Vm::scoped_call` check it before re-entering evaluation, and below it they
return a Bund-level error — reported through `Vm::report` like any other, and
catchable by `?try` — that names the cause: recursion through `times`, `loop`,
`map`, `while`, `for`, `*loop`, a conditional, `?try`, a method, `bund.eval` or
`use` runs on the machine stack, and recursing directly runs on the heap
instead. (Until the eleventh review the message stopped at the method, so
recursion through `bund.eval` was blamed on the wrong words;
`Error::stack_exhausted`, `crates/bund2-api/src/lib.rs`.) **This half does not depend on Tier 1, and
it landed on 2026-09-10**, ahead of this RFC's acceptance. `bund2` spawns its
evaluation thread at 8 MiB plus the 256 KiB reserve, every `Interp::new` takes
its floor from the declared region, and `Error::context` passes the exhaustion
through each body wrapper unchanged, so it is reported once rather than
re-wrapped at every level. `cargo xtask depth`'s new `loop` axis now reports
instead of aborting, and prints the level at which the floor fired: 10,923 in
release on 2026-09-10, and 2,371 under `cargo xtask depth --dev`. Tier 1's share of the stack, and its floor, arrive with
the tier.

## What this design assumes

*Added 2026-09-10. The seventh review listed six assumptions the text relied on
without stating them, the eighth review five more, and the tenth seven more,
answered in 7 and 14–18. The eleventh named three more, answered in 19–21.* Each is stated here,
with the place that enforces or decides it.

1. **One compiled cache, one `JITModule`, one set of cells and one fragment
   table per `Interp`.** §S6, *Addressing*, and criterion 23.
2. **The reporter's `wants_stack` does not change while a compiled body
   runs.** It can change between runs, since the CLI replaces the reporter
   after construction, so it is read at each entry. No `Vm` method reaches
   it, so it cannot change within one. It is asked per severity (D45, §S5).
3. **The level at which evaluation reports stack exhaustion is not part of a
   program's meaning.** D44, §S8, criterion 11.
4. **Everything a leaf native does between two floor checks fits in the
   256 KiB reserve.** This is not measured. A native that recurses in Rust on
   the depth of its data is not bounded by a floor on evaluation, and this RFC
   claims nothing for it (§S8).
5. **The residual path's `apply` is `Vm::apply`, which is synchronous.**
   Recursion through residual paths spends machine stack until the Tier 1
   floor, and is bounded by §S8's floors rather than by the heap (§S8,
   *§S5's residual path is bounded the same way*).
6. **Every call crossed by promotion resolves through one registry `Slot`.**
   This one is enforced rather than assumed. A call through an alias, or
   against a saturated slot, is synced before (§S5).
7. **Every native promotion crosses has an honest declared effect.** It runs
   no body and moves the current stack by its pair. Criterion 24 checks this
   over the corpus, and criterion 28's palette over fourteen operand kinds and
   the workbench. Promotion crosses only the natives that palette brought to
   `Ok` (D48). Natives from any other crate are not trusted at all (D47).
8. **No `bund2-stdlib` native reports at `Error` severity mid-body.**
   Criterion 25. Other crates' natives are not promoted across (D47).
9. **A callee's effect is trusted across a call only when it is declared.**
   Enforced by D46: nothing stays promoted across a call that resolves to a
   lambda (§S5).
10. **A declined compiled body keeps no frame.** A decline is a return (§S8,
    *A decline is a return*).
11. **The Tier 1 floor sits one reserve above the bottom of Tier 1's share,
    and an embedder's declared region has no Tier 1 share** (§S8).
12. **A call is complete only when any body it asked the loop to run has
    run.** Compiled code drains the request after a non-tail call, and hands it
    to the entry after a tail call (§S5, *A call may leave a body to run*);
    criterion 26.
13. **The tier adds nothing to a Tier 0 level below the Tier 1 floor, and
    Tier 0's part under `jit` carries a margin for what it adds anyway.** A
    design rule and a measurement, not a guarantee, because frame sizes are
    the compiler's (§S8, *When compiled frames already hold the share*);
    criterion 11.
14. **No panic escapes a native.** It is caught where the native is called,
    in both tiers, and becomes `Error::internal` (D49); criterion 29.
15. **`bund2-jit` can obtain the set of registrations `bund2-stdlib` made.**
    `register_all` records the ids it mints, and a replay re-records them
    (§S5, D48).
16. **The margin covers every re-entering path.** `m` comes from the largest
    `δ_p / c_p` (§S8); criterion 11.
17. **The epoch and `autoadd` are read after the drain** (§S5, *A call may
    leave a body to run*).
18. **No native leaves a tail request behind when it fails.** Enforced by
    F96's fix, and a refused drain clears the cell too (§S5).
19. **Every way a program stops is visible to compiled code at the call that
    caused it.** A failed `Result` is, through the status. `bund.exit` is not
    by itself, since it returns `Ok` and Tier 0 stops only at its next step,
    so the adapter and the resolving trampoline turn a recorded exit into the
    error status (§S5, *A call may end the program*); criterion 30. Today
    `exit_gate` is the only state Tier 0 gates per step rather than returns
    per call, and a second would need the same treatment.
20. **`PROMOTABLE.txt` certifies the default registration.** It names
    natives, and `--noio` and `--noeval` register failing stubs under the same
    names. D47's id set comes from the registration actually made, and a stub
    that is crossed fails, so it takes the error path (criterion 28).
21. **The natives the palette leaves unrun are the ones that act on the
    host, and the list of them is kept by hand** (`ACTS_ON_HOST`; D48's
    dated notes). An unrun native is not listed and so not crossed, which is
    the safe side; a new host-acting native runs for real under `cargo test`
    until it is added (criterion 28).
22. **A native that reads beyond its operands shows it to D55's audit**,
    either by returning something different when the values beneath it
    change, or by reading the whole stack or workbench, or a stack's depth by
    name. A native that reads `depth()` and only prints it does neither, and
    is not caught; none is known (criterion 14).

# S9. Tier pinning

Two later RFCs need the same mechanism: RFC-0007 must pin a word to Tier 0
across an await point, and RFC-0008 must pin one to Tier 0 while a breakpoint
is set in it. The roadmap assigns it jointly to RFC-5, RFC-7 and RFC-8 as "one
mechanism" (`docs/research/05-rfc-roadmap.md:243`).

This RFC specifies **the mechanism and not its policy**: a body may be marked
`pinned`, which makes it ineligible for promotion and demotes it if already
promoted. What causes a pin for async is **D6 and D7's**, both OPEN and both
declaring `Blocks: RFC-0007`; this RFC does not anticipate them.

# S10. The feature gate, and portability

Cranelift targets x86-64, aarch64, s390x and riscv64 — no 32-bit x86, no 32-bit
ARM (§3.2c). **The interpreter is therefore the portability story**, and Tier 1
must be a `cargo` feature that compiles out cleanly, exactly as `string.grok`
is (D40).

D10's resolution is the governing rule and it cuts both ways here: `bund2 build
--emit=native` may require a C toolchain, and **nothing below `bund2 build` may
require one**. Cranelift is pure Rust and needs no `cc`, so the JIT feature
does not violate D10 — but it must not drag in anything that does.

Exact version pins are already in place (`=0.135.0`) and §3.2e's instruction —
"pin exact versions; budget for periodic migration work" — is honoured by the
workspace as it stands.

**The migration was owed and has now been paid — F80.**
`cranelift-codegen@0.135.0` requires rustc **1.95.0** while
`rust-toolchain.toml` pinned **1.90.0**, so `--features jit` failed before
compiling any Bund2 code, in every crate gating on it. The toolchain pin is now
**1.95.0** (Q31, decided by the repository owner), which was the only option of
three that keeps Tier 1 the same build as Tier 0 rather than a second one.

`--features jit` and `--features aot` are declared in five crates —
`bund2-jit`, `bund2-runtime`, the umbrella `bund2`, `bund2-cli` and
`bund2-bench`. `conform --features jit` builds `bund2-cli` with the feature,
and **criterion 2 has run**: 105/113 with the feature on and 105/113 with it
off, ceiling 105/113 in both, re-run on 2026-09-11 after that day's probes.
It read 79/86 before three probes were added on 2026-09-10, 82/89 before a
fourth, an approved deviation (D50), and 82/90 on the morning of 2026-09-11.

**D9, as amended 2026-09-09, applies**: no Cranelift type may appear in
`bund2-stdlib` or `bund2-api`, which is what §S6's fragments are shaped around.
D9's original sentence — no third-party CLIF lowerings — is about the stable
ABI and still holds: an external package gets `Native` with a declared effect
and no more, and cannot publish a fragment.

**AOT conformance is RFC-0006's**, and it inherits this RFC's invariant
unchanged: the output of `bund2 build --emit=native` must read the same N/M
and CEILING as Tier 0, because AOT changes speed and not meaning exactly as the
JIT does. Criterion 2 here covers the JIT; RFC-0006 states the AOT
counterpart.

# S11. Errors stay a return-value protocol

`try_call` / `try_call_indirect` exist but the unwinder story is incomplete
(§3.2g). Bund2's errors are already `Result`-shaped and route through `Vm::report`
as `Diagnostic`s (D36); compiled code keeps that convention and does not adopt
native unwinding. Under D37 this is also the only option that cannot abort.
A panic in a native is caught where the native is called, in both tiers
(D49), so nothing unwinds into or through a compiled frame. The error travels
in the call context (§S8).

## Preservation analysis

Tier 1 preserves everything by construction, because it adds no word and
changes no word's behaviour. The risks are all of the form "compiled code
disagrees with interpreted code", and each has a named guard:

| risk | guard |
|---|---|
| stale cache entry after a body is freed | the cache's `Weak` keeps the address out of reuse, and a dead entry is swept (D35 as amended); criterion 3 |
| a redefined word still calling old code | all calls indirect through a slot (§S4) |
| **an inlined fragment after its name changes meaning** — re-registered, aliased, shadowed by a lambda, made a command or unregistered, including by a `register`, `unregister` or `alias` earlier in the same compiled body | per-site meaning guard: the slot's generation and `autoadd`, re-read before the first op (§S6); criteria 5 and 17 |
| a **type** specialisation taking a path the interpreter would not | guard-and-branch, generic counterpart in the same function (§S5) |
| a compiled call running under `autoadd`, where the reference would collect the name instead | entry guard on the flag, and a per-site check at every inlined fragment; `:` and `;` are opaque sites (§S4, §S6); criterion 18 |
| **a promoted recursion overflowing the machine stack where Tier 0 runs it on the heap** | a stack floor Bund2 measures on a thread it spawns, compared against `get_stack_pointer` at every compiled body's entry; below it the body declines along the path an interpreted body takes (§S8); `cargo xtask depth` at 100,000 with the feature on, criterion 11 |
| **native-mediated recursion overflowing the machine stack in Tier 0 itself** — through `times`, `loop`, `map`, a conditional, `?try` or a method — which aborted until 2026-09-10 (F85) | the Tier 0 floor, built, and checked wherever a native re-enters evaluation; below it a Bund-level error, never an abort from evaluation nesting (§S8). A native recursing in Rust on the depth of its data is outside that guarantee (*What this design assumes*, 4); criterion 11's `loop` axis |
| the tier taking stack that Tier 0's native nesting would have had | the thread is sized at Tier 0's part plus Tier 1's share, and compiled frames start only above the Tier 1 floor, so Tier 0 never has less room with the tier on (§S8); criterion 11 checks that the `loop` level is no lower with the feature on |
| the level at which `machine stack exhausted` is reported, which moves with the build profile and with the feature | not meaning (D44): never an abort, and never lower with the tier on; criterion 11 |
| a value synced back from a `Variable` losing its D41 stack symbol, so a golden renders `tags: {}` | the sync writes through the same path `Stack::push` uses, not a bare `push_back` (§S5); criterion 12 |
| a slot naming a spelling rather than the resolved target, when `i` resolves aliases twice | slots key on the resolved name (§S4), **except for `$`**, which is resolved one level shallower and keys on that one-level answer (§S4, *The chain*) |
| `unregister` against a name a compiled body still calls | the call slot is rewritten to what the name now resolves to — the native a lambda shadowed, if any — and to a failing stub only when nothing resolves; never freed (§S4) |
| `alias` retargeted after a caller was compiled | a name reached through an alias is never cached: its call slot points at the resolving trampoline (§S4, *Two structures*), so a retarget is seen on the next call and there is nothing to fan out |
| a diagnostic raised in compiled code carrying no Bund source location | D36 requires a `Diagnostic` with a Bund location; compiled frames have none unless the lowering carries spans, which §S11's return-value protocol must thread |
| an **opaque** site leaving a stale promoted value behind | promotion stops and syncs to the real stack before the call (§S5); cost checked by criterion 9 |
| mixed-kind comparison lowered to a machine compare | forbidden while D33 is OPEN (§S6) |
| a `*`-family word crossing a promoted region — including one bound at run time to a name the body was compiled against | an `opaque` effect (D12; the ten folds are registered, each opaque, since 2026-09-11), and the pre-call slot-generation check, which syncs before any call whose binding changed (§S5, *What a promoted value must not change*); criterion 22 |
| **a callee reached through an alias or `$name` whose target is rebound while values are promoted across the call**, such as `<-` → `stacks_left` with `stacks_left` rebound to a lambda that consumes two | nothing stays promoted across a call that does not resolve through its own registry `Slot`, or whose slot's generation is saturated: the body syncs before it (§S5); criteria 5 and 22 |
| an error returned from compiled code while values are promoted, whose report or `[BUND]` stack dump would show a short stack | every error return syncs first (§S5); criterion 22 |
| a native that reports with a stack snapshot while values below its arity are promoted | the reporter says per severity whether it wants a snapshot (D45); while it wants one for `Warning` or `Notice`, nothing stays promoted across a call, and this is read at each entry (§S5); Q34; criterion 22 |
| a promoted value's `.id` or `.timestamp` | both are lazy (D1, D2), and a stamp is observed only after a sync. Once a guard admits heap values, a heap value's mint order may differ from Tier 0's; that is not meaning, because ids are opaque and F14 normalises them (§S5). A fragment that compares or hashes must materialise identity where its word does — criterion 16 |
| a warning or notice from compiled code | compiled code emits none of its own; a fragment has no reporting op, so an arm that reports stays a call (§S5) |
| unbounded code memory | caps and permanent demotion (§S7) |
| a fragment disagreeing with its word | differential test per fragment over the arm's boundaries, identity included (§S6); criterion 16 |
| an op failing after its guard admitted, with operands already pulled | `Fragment::new`, the only constructor, refuses such a fragment; anything that escapes is `Error::internal`, never a fall-through and never silent success (§S6); criterion 19 |
| a word reading beyond its declared arity while values are promoted | a promotion barrier. D55's audit, a four-run differential and an observation audit inside criterion 28's palette, keeps every such native off `PROMOTABLE.txt`, and promotion syncs before any native not listed; criterion 14 |
| the lowering and `frag::run` disagreeing about what a fragment means | criterion 16's third leg, required before a lowering ships |
| a body entered through a loop word, conditional or method path never reaching the tier, because no `Rc` survived to the entry | D42: `Vm::eval_lambda` and `Vm::tail_lambda` take the value, and the frame holds it; criterion 20. `Vm::scoped_call` is the exception: its body is a LIST built per call, so a `context` body has no key and stays at Tier 0 (§S3) |
| a **current-stack switch** mid-body — `to_stack`, `to_current`, `stacks_left`, `stacks_right`, `endcontext`, a scoped conditional, a CONTEXT literal — while values are promoted | the current-stack epoch, re-read after every call, and a static barrier at a CONTEXT literal; the residual path syncs each value to the stack it came from (§S5); criterion 21 |
| a named-stack word — `swap_in`, `rotate_stack_*` — reaching the current stack by name while promotion holds its values | a promotion barrier. The palette includes the current stack's name, so D55's differential sees such a word change what it leaves when the values beneath change, and the observation audit records `depth_of`; D55 keeps these natives off `PROMOTABLE.txt` (its first run found `rotate_stack_left` and `rotate_stack_right`, and F111's six miscounted pairs); criterion 14 |
| `autoadd` turned on mid-body, when the reference collects literals as well as calls, and pushes a CONTEXT value rather than switching to it | re-read after every call; the residual path applies the rest through `apply` (§S5); criterion 18, against a reference-captured probe |
| unregistering a lambda that shadowed a native | the call slot is rewritten to the revealed native, not stubbed (§S4) |
| a compiled body substituted at `eval_lambda`, whose errors Tier 0 wraps as `Lambda content evaluation returned error: …` and `times` wraps again as `TIMES: lambda execution returns error: …` | the compiled body returns its error unwrapped and the entry wraps it, so both prefixes come from the same code whichever tier ran (`Vm::eval_lambda`; `times_base` in `crates/bund2-stdlib/src/seq.rs`) |
| an eval'd string's inner lambda recompiled on every evaluation | the 1024-body cap (§S7); content-hash keying is the eventual answer (§S3) |
| a guard widened to boxed values, whose operands might carry a `q` other than 100.0 | no arithmetic word averages `q` (D32 as amended, Q35), so the result is a fresh 100.0 either way; criterion 16 must then include such an operand if one can be built (§S6, constraint 2) |
| a body compiled under one `Interp` and run by another on the same thread | one cache, `JITModule`, set of cells and fragment table per `Interp`, so another `Interp`'s code is never found (§S6, *Addressing*); criterion 23 |
| **a lambda callee whose inferred effect changes**, because a word its body calls is rebound before the call or by the callee during it | nothing stays promoted across a call that resolves to a lambda (D46, §S5); criteria 5 and 22 |
| **a native whose declared effect hides a body it runs** — `execute.` until F87, `object` and `display` until F91 | every native that runs a body declares `StackEffect::opaque`, checked by criterion 24's audit over the corpus, and by criterion 28's palette for the words and arms the corpus never reaches |
| **a native whose declared pair miscounts the current stack** — `clear`, `fold`, `move`, `dup_many`, `format`, `pull`, `fold_stack`, `swap_in`, `print.` and `println.` until F92, `drop_stack` until F94 | criterion 24's audit compares each fixed-effect native's depth change with its pair, as RFC-0004 §S1 reads it, and criterion 28's palette does the same over fourteen operand kinds and the workbench; promotion crosses only the natives the palette brought to `Ok` (D48) |
| **a native reporting at `Error` severity mid-body**, under a reporter that wants fatal snapshots | no shipped `bund2-stdlib` code reports at `Error`, checked by a source scan (criterion 25); no other crate's native is promoted across (D47) |
| **a native `bund2-stdlib` did not register** — an embedder's, or an external package's under D9 — whose declared effect nothing checks | promotion crosses only natives whose registration id `bund2-stdlib` made (D47, §S5); criterion 27 |
| **a body a call asked the loop to run** — `!` on a lambda, `if`, `ifthenelse`, a lambda call — running after the next value, or overwritten by a later request | the request cell, read after every call, with a drain for a non-tail call, a hand-back for a tail call, and an internal error if a request is ever pending when a call starts (§S5); criterion 26 |
| the per-level stack cost of a declined compiled body below the Tier 1 floor | a decline is a return, so no compiled frame is kept (§S8); criterion 11 |
| **native nesting that starts beneath compiled frames holding Tier 1's share**, where any byte the tier adds to a Tier 0 level lowers the level at which exhaustion is reported | the cache lookup returns before `run_to`, and Tier 0's part under `jit` carries a measured margin `m ≥ 8 MiB × max_p(δ_p / c_p)` over every re-entering path (§S8); criterion 11's share-held case |
| **a lambda that shadows a native**, whose name `effect_of` answered with the native's effect until F93 | callees are classified by `Registry::resolve`, and `effect_of` follows the same order (§S5, F93); criterion 22 |
| **the tier never running in `bund2` itself**, because its thread's declared region had no share, so that the meaning criteria pass vacuously | `bund2` declares its share with `set_stack_region_with_share` (§S8); criterion 2 requires compiled bodies, and runs again at threshold 1 |
| **a native reporting at `Error` severity through a spelling the source scan misses** | the effect audit records any `Error` report made while a native runs (criteria 24 and 25) |
| **a native panicking inside a dependency** — `jarowinkler` in `natural` (F95) — reached from compiled code | caught where the native is called, in both tiers, and reported as `Error::internal` (D49); no panic unwinds through a compiled frame (§S8); criterion 29 |
| **the call boundary itself**: a `NativeFn` whose ABI and `Result` CLIF cannot carry | a context pointer and an integer status under `CallConv::Tail`, a Rust adapter per native, `Tail` thunks in the slots, and a C-convention entry trampoline (§S8); criteria 4 and 29 |
| **a stale tail request** left by a native that failed after filing it | `Interp::invoke` clears it (F96), and a refused drain clears the cell (§S5); criterion 26 |
| **native nesting through a word other than `loop`**, whose per-level cost makes the margin too small | `m` is set from the largest `δ_p / c_p` over every re-entering path, a set derived by source scan rather than listed (§S8); criterion 11 |
| **a program ended by `bund.exit` (D52) while a compiled body runs** — the native returns `Ok`, and Tier 0 stops only at its next step, which compiled code does not take | the adapter and the resolving trampoline turn a recorded exit into the error status, the entry trampoline refuses to start a body, and compiled code's error path syncs and returns (§S5, *A call may end the program*); the effect audit records a fixed-effect native that requests an exit; criterion 30 |

## Alternatives considered

**Build Tier 1 before the representation work.** This was the live question when
this RFC was drafted, and it is now moot: D41 did the representation work, and
§S1's update records a 3.8–4.2× improvement per program shape from that alone.

It is kept as an alternative because the *reasoning* still applies to whatever
comes next. A compiled lowering of `push` written before D41 would have encoded
the expensive shape into generated code, making the representation fix harder
rather than easier — and the cheapest thing a JIT can do is not be asked to
compensate for a representation that has slack left in it. Whether slack
remains is criterion 10.

An earlier version of this entry rejected the alternative on an Amdahl bound of
"~1.3× at best". That figure is withdrawn (§S1) and this entry no longer rests
on it.

**A threaded interpreter instead of a JIT.** §2.1 rates this ~2× and it needs
no code generator, no platform restriction and no code-memory policy. It is a
genuine competitor to this RFC and it is **not** ruled out here — but it is
RFC-0003's territory (Tier 0's implementation), not this one's. §S1's
prerequisite may make it the better next step, and this RFC should not
pre-empt that.

**Content-hashed cache keys instead of pointer keys.** D35 considered and
rejected this for now; it remains "a strict upgrade" whose only cost is the key
function. Not reopened here.

**Module rotation to reclaim code memory.** Needs a shadow stack to prove no
orphaned frame is live. Out of scope for v1, per §3.2a.

## Acceptance criteria

Each names the tool that decides it **and either a threshold or a boolean
outcome**. Criteria 1, 7, 9, 10, 11 and 17 carry a number; the rest are boolean by
nature — a redefined word is observed or it is not, a cap holds or it does not,
`cite` exits 0 or it does not.

An adversarial review found 4, 6 and 7 naming neither, which is how a criterion
goes vacuous: 7 read "shows improvement", and nothing fails that. **Criteria 1
and 2 gate the RFC's own premise**; the rest are for the implementation.

**The `--features jit` criteria became runnable on 2026-09-09**, when the
toolchain pin was raised to 1.95.0 to match the pinned Cranelift (F80, Q31).
Before that none of 2, 4, 5, 6, 7, 9, 10, 11 or 12 could be executed at all.

Criterion 2 has since **run and passed** — 105/113 with the feature on and off, ceiling 105/113, re-run on 2026-09-11 after that day's probes —
and is **vacuous until a tier exists**, since the feature gates no code. That
is worth stating rather than counting: a criterion that cannot fail yet is not
evidence, and this one is listed as runnable rather than as met.

1. **The gate is passed before implementation begins.**
   `cargo bench -p bund2-bench -- value/push_pull` reports **under 20 ns**.
   **Met**: 9.8 ns, after D41.

   An earlier version also required "`dispatch/*` re-measured shows the
   dispatch loop, not the value layer, as the dominant term". **That half is
   withdrawn**, because §S1's update establishes it cannot be shown with these
   benchmarks — nothing separates dispatching a word from the work the word
   does once dispatched, and the difference `1 drop` − `dispatch/literal_only/w1000`
   bounds dispatch only from above. A criterion that cannot be decided is worse than none; the question
   it was reaching for is criterion 10, which can fail.

2. **Conformance moves by exactly zero.**

        cargo xtask conform
        cargo xtask conform --features jit
        cargo xtask conform --features jit --jit-threshold 1

   The third cannot run yet: `xtask conform` answers `unknown argument
   --jit-threshold` (2026-09-11), since the flag arrives with the tier. All
   three must read the same N/M **and the same CEILING**. Any movement is a bug,
   per CLAUDE.md — which also requires the ceiling beside the number, since an
   approved deviation can never enter the numerator and N/M alone overstates
   the remaining work. Today: **105/113, ceiling 105/113**, with the feature
   off and on, re-measured 2026-09-11 after that day's probes (82/90 that
   morning); eight approved deviations, D50's among them. This criterion measures the **dev** profile and criterion 11
   measures **release**; the split is deliberate, and stated in both places.

   **The two `jit` runs must compile something.** This is the ninth review's
   S3, and the owner chose among its options. Two things could make them
   vacuous.
   - `bund2`'s thread had no Tier 1 share, which §S8's
     `set_stack_region_with_share` now fixes.
   - §S7's threshold is 64 evaluations of one body, and most corpus programs
     are short.

   So `bund2` reports how many bodies it compiled when asked, through a
   statistics flag, and `conform` prints the total beside its `measured:`
   line. **A `jit` run that compiles no body over the corpus fails.** The
   second `jit` run sets §S7's threshold to 1, so every body compiles on its
   first evaluation. That is the strongest test of meaning the corpus can give,
   and it is why §S7 makes the threshold configurable and recorded.

   **Once a tier exists, this criterion exercises promotion across calls
   under the CLI's default reporter.** D45 made `TextReporter` want a snapshot
   only for a fatal report, which is made after evaluation returns. Before
   D45 the default reporter kept every value from being promoted across a
   call, so this criterion would have run promotion only between calls.

   **`--features` did not exist on `conform` when this criterion first claimed
   to have run.** `conform` rebuilt `bund2-cli` unconditionally and without
   features, so a jit-built binary was overwritten and the non-jit one
   measured — both numbers came from the same build and nothing said so. All
   five subcommands that measure the binary now share one builder
   (`xtask/src/buildcli/mod.rs`) — `depth` last, on 2026-09-10, after the fifth
   review found it still carrying a private one — and `conform` prints the line
   `measured: bund2-cli, dev profile, features: jit` above its result.

3. **A cache entry cannot answer for a different body.** D35 requires this as
   a test, not a comment, and names the failure: an entry whose body was freed
   can be answered for a *different* body at the reused address — wrong code
   executed.

   With D35 as amended (Q32) **the cache holds a `Weak`**, which keeps the
   address out of reuse while the entry lives. The test: drop every strong
   reference to a compiled body and assert its contents are dropped, that its
   entry no longer upgrades and answers for nothing, and that after the sweep
   the entry is gone. An earlier version tested that a strong reference kept
   the body alive, which D35 no longer requires.

   §S7's counter is criterion 6, and it now holds the same kind of reference:
   both must drop a body's contents with its last strong reference.

4. **Every inter-word call in compiled code is indirect**, checked
   mechanically rather than by reading.

   "Inspecting the emitted CLIF" was the earlier wording and it names no tool:
   inspection is a person, and a person will stop looking once the design feels
   settled. The property §S4 actually requires is about *relocations* — a
   direct call emits a relocation naming a `FuncId`, and that relocation is
   exactly what would still point at orphaned code after a word is redefined.

   The check: compile a body that calls another word, and assert the module's
   relocation records contain **no entry targeting a function**. Corroborated,
   not replaced, by asserting the CLIF text contains `call_indirect` and no
   bare `call fn`, which is readable but formatting-dependent.

   It fails if any lowering path emits a direct call, including one added later
   for a word that looks safely static — and D16 means none is.

   **Two kinds of relocation are allowed**, both from §S8's call boundary: a
   native's `Tail` thunk calling its Rust adapter, and the entry trampoline
   calling the body it enters. Both name an external symbol or the trampoline's
   own target, never a compiled body's `FuncId` from inside another body. The
   check also asserts that every call slot's target is a `Tail` thunk or a
   `Tail` body.

5. **A redefined word is observed by compiled callers — called *and*
   inlined.** `register` a word, force promotion of a caller, `register` it
   again with different behaviour, and assert the caller's next result
   changes.

   **As first written this passed vacuously for every inlined word**, because
   an inlined fragment goes through no slot (the fifth review's B1). So it runs
   a second time against a caller that inlines `+`'s fragment, changing what
   `+` means in each way §S4's chain allows — re-registered as a lambda,
   aliased to another word, unregistered, and registered as a command — once
   before the caller runs and once **mid-body**, by a `register`, `alias` or
   `unregister` earlier in the same compiled body. Each must change the
   caller's result exactly as it changes Tier 0's.

   **A third time through an alias.** A caller that calls `<-`, with
   `stacks_left` rebound to a lambda of a different effect, once before the
   caller runs and once mid-body. `<-`'s own generation does not move, which
   is the case §S5's direct-resolution rule exists for.

   **A fourth time through a lambda callee.** A caller that calls `f`, where
   `f` is `{ g }`, with `g` rebound to a word of a different effect, once
   before the caller runs and once by `f` itself mid-call. Nothing was
   promoted across `f` (D46), so the result must match Tier 0's.

6. **The caps hold**, checked by a test per row of §S7's table rather than by
   inspection. With the compiled-function cap set to 4, compiling five distinct
   bodies leaves the fifth interpreted. With the recompile cap set to 2, a
   third redefinition demotes the body permanently. With the counter cap set to
   8, evaluating nine distinct bodies leaves the map at 8.

   **And the counter does not pin.** Evaluate a body below the threshold, drop
   every strong reference to it, and assert the body's contents are dropped
   while the counter still holds its entry — then that the entry is gone after
   a sweep. This is §S7's claim that the *cache* pins ≤1024 bodies and the
   *counter* pins none; without it, "the counter holds a `Weak`" is a comment
   rather than a property. It is the counterpart of criterion 3, and it fails
   the same way: silently, and only under memory pressure.

7. **The feature does not move what it must not**, with a stated tolerance.

   This is the regression half; the *win* is criterion 10, and an earlier
   version conflated them into "shows improvement", which nothing can fail —
   0.1% is an improvement.

       cargo bench -p bund2-bench -- --save-baseline off      # feature off
       cargo bench -p bund2-bench --features jit -- --baseline off

   | group | requirement |
   |---|---|
   | `startup` | **no change**: Criterion reports no statistically significant difference, and the point estimate moves by **< 5%** |
   | `value` | no change, same tolerance — D41's work is below the tier and the tier must not disturb it |
   | `dispatch`, `arith`, `corpus` — the `bund2-bench` group, not `cargo xtask corpus` | free to improve; a **regression beyond 5%** on any of them fails |

   The 5% band is not arbitrary. Run-to-run spread on one machine is already a
   few percent — `fragment/int_add/tier0` read 57.9 and 59.2 ns in two runs on
   2026-09-10, and the sixth review's re-run of that table agreed to within 3%
   — and a threshold inside the noise would fail on weather. (An earlier
   wording quoted a range for `startup/registry/register_all` that no recorded
   run supports.)

   **A corpus wall-clock figure is not an acceptance criterion.** Q14
   establishes that the subprocess harness cannot resolve interpretation — an
   ordinary program evaluates in ~9 µs against a 2.3 ms process floor — so a
   percentage taken there measures process spawn. `cargo xtask bench` keeps
   only its regression role: catching a startup collapse.

8. **`cite` and `lint` clean**, with `cargo xtask cite` resolving every
   `path:line` in this document. **Note what this now checks and did not
   before:** citations into `crates/` were invisible to the extractor until
   F79, which is how two of this document's own citations were stale on the day
   they were written. It is **still partly blind**: the extractor cannot seed a
   scope from an unprefixed filename, so §S4's thirteen `multistackvm_apply.rs`
   and `multistackvm_inline.rs` citations were invisible to it until they were
   spelled with their `reference/rust_multistackvm/src/` prefix on 2026-09-10.
   A citation `cite` cannot see is one it cannot fail.

9. **The sync is worth its cost.** §S5's second rule requires promoted values
   to be written back — each to the stack it was taken from — before an opaque
   call. **It is not
   established that this pays.** For a short straight-line run the sync may
   cost more than the promotion saved, which would collapse the stop-at-the-site
   rule back into whole-body exclusion for those bodies.

   The criterion: a benchmark in `crates/bund2-bench` comparing a body with an
   opaque site in the middle against the same body interpreted, at straight-line
   run lengths of **1, 4, 16 and 64 words before the site**. At each length the
   compiled form must be **no more than 5% slower** — the same band as
   criterion 7, and for the same reason: run-to-run spread is already ~±2.5%.

   **It runs under the CLI's default reporter**, `TextReporter` with its stack
   dump on, which is the configuration `bund2 script` uses and which permits
   promotion across calls under D45. A figure taken under `--no-dump-stack` or
   a silent embedder must say so.

   **The crossover length is reported, not optional.** An earlier wording said
   that if the compiled form is slower below some length, "this RFC must state
   that length" — a criterion that lets its own failure be renamed as a
   parameter. It is now: the shortest length passing the 5% band is recorded,
   and promotion is skipped beneath it. **If no length passes, §S5's second
   rule is wrong** and the choice is between whole-body exclusion (RFC-0004's
   original reading; §S6's *Where promotion stops* counts what that costs) and abandoning promotion
   across opaque sites entirely.

   Recorded as a criterion rather than an assumption because it is the one place
   §S5's rule could be wrong in a way that no correctness test would catch.

10. **The dispatch share is measured, not estimated.** §S1 withdraws an Amdahl
    bound rather than correcting it, because no benchmark here separates
    dispatching a word from the work the word does once dispatched:
    the difference `1 drop` − `dispatch/literal_only/w1000` bounds dispatch at
    **≤ 33.7 ns** and cannot go finer. (An earlier wording attributed the bound
    to `dispatch/literal_only/w1000` itself, which reads 13.2.)

    The criterion: before any lowering is optimised, an A/B on one compiled
    body against the same body interpreted, reported per program shape as in
    §S1's update. **A speedup below 1.2× on `1 2 + drop` means the tier is not
    earning its keep** and §S1's gate should be reopened rather than the number
    explained. This is the criterion that decides whether Tier 1 was worth
    building, and it is deliberately the one that can fail.

    §S6's prototype puts ceilings on what this can report (*Measured*,
    2026-09-10): **inlining alone at most 2.0× on `Int + Int` and 3.0× on
    `dup drop`; inlining with promotion at most 8.8× and 12.9×** — in Rust,
    with no compiled-code overhead. Compiled code pays entry, exit, a type
    guard and a meaning guard per inlined site, and a helper call per sync, so
    the real figures sit below those.

    **The stop rule governs the measured lowering, not the ceiling.** If
    inlining alone, *compiled*, comes in under 2× on a shape, that shape's
    inlining is not worth its machinery on its own, and the constraint is the
    lowering rather than the ceiling — a reason to stop, not to tune. The
    ceiling cannot trigger the rule: on `Int + Int` it is **2.03×**
    (59.2 / 29.2), and the sixth review's re-run read 2.04×. What the ceiling
    does say is that there is almost no room. A lowering that adds more than
    about 0.4 ns per `+` to the 29.2 ns `lowered` measures will land under 2×.
    So this RFC **predicts** arithmetic inlining alone will not clear the rule,
    does not propose it as a performance feature on the strength of that
    prediction, and leaves the decision to this criterion's measurement. An
    earlier revision said the rule had already been applied; it cannot have
    been, before a lowering exists. Inlining stays necessary as the mechanism
    promotion rests on, and operand-free arms like `dup drop` have a 3.0×
    ceiling and room to spare.

    Reported per shape, then: inlining alone, and inlining with promotion. The
    1.2× floor above applies to the tier as shipped. It is measured under the
    CLI's default reporter, as criterion 9 is.

    **Measured by a throwaway lowering, 2026-09-11 — outside this RFC's gate.**
    A spike on the scratch branch `spike/lowering-1` (commit `3138d3c`,
    `bund2_jit::spike` and `bund2-bench`'s `spike` benchmark under
    `--features jit`) compiles `1 2 + drop` through Cranelift with §S8's call
    boundary: a `CallConv::Tail` body taking a context pointer and returning a
    status, a Rust adapter calling each native under `catch_panic` (D49), an
    entry trampoline, §S8's entry checks and §S5's post-call loads. It is not
    the tier and is not proposed for merging. Release build, Apple silicon,
    Criterion medians, against `Vm::eval_lambda` on the same body:

    | body | Tier 0 | every word a call | literals promoted, `+` and `drop` inline |
    |---|---|---|---|
    | `{ 1 2 + drop }` | 111.8 ns | 58.0 ns, **1.93×** | 1.86 ns, 60× |
    | the four words ×100 in one body | 10.55 µs | 5.68 µs, **1.86×** | 19.6 ns, ≈540× |

    What it shows and what it does not:
    - **The floor is cleared by removing dispatch alone.** The "every word a
      call" lowering inlines nothing and keeps §S8's full boundary on every
      word. It reaches 1.9×, close to the 2.03× inlining ceiling above,
      although the two are measured on different bases: this one per body
      entered through `eval_lambda`, that one per `+` in a chain.
    - **The promoted column is a ceiling for bodies made only of literals.**
      Cranelift folds `1 2 +`, and merges the meaning guards' cell loads,
      because no call lies between two guards, so no cell can change: the ×100
      body is 1,744 bytes, about 16 per group, where the call lowering grows by
      about 480 per group. That merge is sound, and it means criterion 17's
      per-site cost is near zero in straight-line code; the cost sits after
      calls. A body whose operands arrive on the stack pays pulls, type guards
      and a sync, for which §S6's fragment `promoted` column, about 6.7 ns per
      `+`, is the better estimate.
    - **The spike leaves out things that make the tier slower**: call slots and
      `Tail` thunks, the cache lookup at entry, §S7's counter, the residual
      path, type guards, and the exit check after each native (§S5, *A call
      may end the program*). None of them is plausibly worth the 0.9× of
      headroom above the floor, but this criterion is met only by the tier as
      shipped, and stays open until then.

11. **A promoted recursion does not overflow the machine stack.** §S8's
    correctness problem, and the criterion is one that already exists:

        cargo xtask depth                       # feature off — Met at 100,000
        cargo xtask depth --features jit        # loop level no lower (D44)

    **`--features` did not exist when this criterion was written**, and
    `xtask depth` shelled out to whichever `bund2` happened to be sitting in
    `target/`, so the command could not have measured a tier even in
    principle. `depth` now *builds* the binary it measures, with the features
    asked for — F80's lesson applied to the one check that stands between §S8
    and an abort.

    RFC-0003's criterion 2 is **Met** at a call depth of 100,000 (RFC-0003,
    acceptance criterion 2, cited by number as in §S8). It must stay met with
    the tier on. Before the frame loop the same program aborted between 5,000
    and 20,000, so this criterion has a demonstrated failure mode and is not
    hypothetical.

    Reported with the result: **both floors**, **the depth at which compiled
    bodies begin declining**, and the per-entry cost of the Tier 1 check
    measured in `crates/bund2-bench`. A check costing more than 2 ns per body
    entry should be reconsidered against simply not promoting bodies that can
    recurse — which D16 makes undecidable, so the check is the expected answer
    and the number is what says whether it is affordable.

    It also runs a recursion **through a loop word** — a body that calls itself
    from inside `times` — because `eval_lambda` spends a Rust frame per nesting
    (§S8, *The guard*), and the self-recursive word alone never exercises that.
    That axis must **report, not abort**, in both builds. It aborted in both
    until §S8's Tier 0 floor landed (F85). `cargo xtask depth`'s `loop` axis
    now runs it at 100,000 levels, reports a Bund-level error, and prints the
    level at which Tier 0's floor fired: **10,923** in release on 2026-09-10.

    **That level is not meaning (D44).** It is 2,371 in the dev profile on the
    same source (`cargo xtask depth --dev`), and the oracle aborts at every
    depth. In dev the `nesting` axis also aborts, in the parser's recursion,
    which RFC-0003 excludes. That is why this criterion runs release. So the criterion is that
    the axis reports rather than aborts in both builds, and that **its level
    with the feature on is no lower than with it off**. An earlier revision
    required the two levels to be equal, and no fixed floor can give that and
    "never less" together (the seventh review's B1; §S8, *Two floors*).

    **A case with the share held**, added for the ninth review's S2. A word
    recurses directly until compiled frames pass the Tier 1 floor, and then
    runs the `loop` axis from there. Its level with the feature on must be no
    lower than with it off, and the run reports `c` and `δ`, from which §S8's
    margin `m` is set. The `loop` axis alone starts at the top of the stack,
    with nothing compiled above it, so it cannot show this.

    **Every re-entering path, not `loop` alone**, for the tenth review's S2.
    The run reports `c_p` and `δ_p` for each path §S8 lists, and `m` is set
    from the largest ratio.

12. **A synced value keeps its stack tag.** §S5's rule writes promoted values
    back to the real stack; **D41 put the stack tag inside the value for
    scalars**, so a sync that pushes a bare `BundValue` leaves `StackSym::NONE`
    and the value renders `tags: {}` where the oracle renders
    `tags: {"stack": "main"}`.

    42 of 113 goldens carry exactly that text, and 46 carry some `"stack":`
    tag (2026-09-11, `grep -l` over `tests/golden`), so the failure is loud — but only if a
    golden exercises a compiled body with an opaque site in it, which none does
    today. The criterion: a probe that pushes, promotes, syncs and dumps, with
    `cargo xtask conform` green. **This row exists because a reviewer found it
    and the Preservation table did not**; the tag moved into the value one day
    and this RFC was written the next, without the two being connected.

13. **An absent effect is treated as `Opaque`, never as zero.** §S6's *Where
    promotion stops* counts **21 sites** (2026-09-11) whose word declares no effect at all. A defaulted
    `StackEffect` would read `Fixed(0, 0)` — "consumes nothing, produces
    nothing" — and a compiled body would keep promoting straight through a call
    that may do anything to the stack.

    The check: register nothing for a name, compile a body that calls it, and
    assert the lowering stops promoting at that site exactly as it does for
    `!`. It fails silently otherwise, which is why it is a criterion and not a
    remark.

14. **A word that reads beyond its arity is a promotion barrier.** Q34. `debug.display_stack` declares `eff(0, 0)` and calls `vm.snapshot()`;
    it runs in the golden capture epilogue, so it is on the conformance path.

    The check: compile a body that promotes, then calls a word which inspects
    the stack beyond its declared arity, and assert the observed depth and
    contents match Tier 0's. **D55 identifies the set.** Criterion 28's
    palette runs each fixed-effect native four ways: padded twice, with
    nothing beneath its operands, and with different values beneath. It also
    records any native that reads the whole stack or workbench, or a stack's
    depth by name, while audited. A native either check flags is left
    off `PROMOTABLE.txt`, so compiled code syncs before it. The Tier 0 half
    runs today, inside
    `every_fixed_effect_native_keeps_its_pair_over_the_promotable_palette`
    (`crates/bund2-stdlib/src/lib.rs`), and the flagged natives are listed in
    `PROMOTABLE.txt` as comments. Its first stable run, on 2026-09-11, flags
    seven. `debug.display_stack` and `debug.display_workbench` read the whole
    stack or workbench. `move_from`, `rotate_current_left`,
    `rotate_current_right`, `rotate_stack_left` and `rotate_stack_right`
    change the values beneath their operands. The same run found six
    named-stack words whose declared pair was wrong on the current stack
    (F111, now opaque). The compiled half needs a tier.

15. **The dependency direction is not inverted.** §S6's mechanism rests on
    `bund2-jit → bund2-stdlib` being permitted while the reverse is not.
    RFC-0000's B3 already checks one half:

        cargo tree -p bund2-stdlib      # must not list bund2-jit
        cargo tree -p bund2-stdlib      # must not list any cranelift-* crate

    The second line is new and is the one D9's amendment turns on: a fragment
    that reached for a Cranelift type would pull the optional subsystem into
    the mandatory crate, and B3 as written would not catch it because the
    dependency would be on `cranelift-frontend` rather than on `bund2-jit`.

16. **Every BundIR fragment agrees with the word it specialises.** A
    differential test per fragment runs the arm and the word on **the same
    constructed values** — not the same source text, which would route one side
    through the lexer — and asserts equality of value, `dt`, `q` and D41's
    stack tag. For `dup` it also asserts F13's property **directly**: the two
    values left behind have different identities. The render comparison cannot
    see that, because identities differ per run and are normalised away.

    **The inputs are the arm's boundaries, chosen by hand**, and the test says
    so; an earlier version called them generated. For `Int + Int` they include
    `i64::MAX`, `i64::MIN` and their neighbours, and a separate assertion pins
    `i64::MAX 1 +` to `i64::MIN` — wrap-around is where a lowering using a
    trapping or checked add would part company with the word.

    **On today's domain the `q` assertion cannot fail** (§S6, constraint 2),
    and this criterion does not count it as evidence; it is kept so that a
    widened guard can make it fail.

    **It certifies the model, not the compiled code**, until it has a third
    leg: the lowered code against `frag::run` over the same boundaries. That
    leg is required before any lowering ships, and cannot be written before one
    exists.

    Runs today: `cargo test -p bund2-stdlib fragments`.

17. **Every inlined site carries its meaning guard.** §S6: an inlined
    fragment is entered only while the site's slot still holds the binding it
    was inlined against, and `autoadd` is clear. Checked mechanically, as
    criterion 4 is: compile a body that inlines a fragment and assert every
    inlined region in the emitted code is preceded, on every path into it, by a
    load and compare of the name's generation cell and of the `autoadd` cell
    (§S6, *Addressing*). That is a dominance property, and it is checkable
    because the lowering records each inlined region's entry block in a side
    table and `cranelift-codegen`'s `DominatorTree` answers whether the guard's
    block dominates it.

    Its cost per site is measured in `crates/bund2-bench` and **must stay under
    2 ns** — the bound criterion 11 sets for the headroom check, and about 7% of
    the 29.2 ns `lowered` `+` it guards. Above that, the guard's design is
    reconsidered before inlining ships.

    **The per-call checks have the same bound.** The three loads after every
    call (epoch, `autoadd`, request), together with the callee's generation before a call
    across which values stay promoted (§S5), must cost under 2 ns per call.
    In a body that promotes, calls are far more common than inlined sites, and
    until the seventh review nothing bounded their checks.

18. **Compiled code honours `autoadd`** — at entry, after every call, and for
    every kind of value the mode affects. Bind `:` and `;`, compile a body that
    turns the mode on mid-body through `!`, and assert two things. The calls
    and **literals** after it must be appended into the value beneath rather
    than dispatched or pushed
    (`reference/rust_multistackvm/src/multistackvm_apply.rs:19-27`, `reference/rust_multistackvm/src/multistackvm_apply.rs:89-97`).
    A CONTEXT value must be **pushed as a value of its own** rather than
    switched to (`reference/rust_multistackvm/src/multistackvm_apply.rs:72-73`).
    An earlier wording had CONTEXT "collected", which contradicted F84; a probe
    written to it would have asserted the wrong shape.

    **The oracle is the reference, not Tier 0.** Tier 0's `autoadd` arm pushes
    the name *beside* the value where the reference appends it *into* the
    value, and it collects no literals or CONTEXT values at all — F84, whose
    unit test asserts the divergence. So the criterion runs against a probe
    golden captured from the oracle, like every other probe.

    **Not runnable today, and stated so the guard is not built blind.** Bund2
    binds neither `:` nor `;` (`bund2 words`), although `Interp` carries the
    flag and the parser recognises `:` as a command term. The probe is
    captured when they are bound — capturing it now would add a golden Tier 0
    cannot pass — and F84 is fixed with them.

19. **A fragment that could fail after its guard admits cannot be built.**
    By then operands have been pulled, so neither declining nor running the
    word next is safe. `Fragment`'s fields are private and `Fragment::new` is
    its only constructor outside a test-only feature. It walks the ops,
    typed, against what the guard promises, so every consuming op counts —
    `DropTop` and `DupTop` as well as `PopInt` — and every register is written
    before it is read. `frag::run` reports anything that escapes as
    `Error::internal`, `DropTop` on an empty stack included, never as success.

    **Met at the model level** by
    `every_consuming_op_counts_and_registers_are_written_before_read`
    (`crates/bund2-ir/src/fragment.rs`), and by
    `new_refuses_the_fragments_that_could_fail_after_the_guard` and
    `a_post_guard_failure_is_an_internal_error`
    (`crates/bund2-interp/src/frag.rs`). An earlier revision claimed this with
    `validate` optional and counting only `PopInt`; the sixth review found two
    fragments that passed it and then succeeded silently. A lowering must keep
    it: no path in the emitted code leads from a failed op back to the slot
    call.

20. **A body run by a loop word reaches the counter under one key.** Run a
    lambda through `times` 100 times and assert §S7's counter holds one entry
    for it, at 100. **Its precondition is met**: under D42 the key reaches the
    entry point, and `times_enters_one_body_under_one_key`
    (`crates/bund2-stdlib/src/seq.rs`) shows one key on all 100 entries through
    Tier 0's `entry_log` seam. The criterion itself needs the counter, and runs
    when one exists.

21. **A current-stack switch mid-body gives Tier 0's result.** Compile, with
    promotion on, `1 2 "s" to_stack +`; the same with `to_current`,
    `stacks_left`, `endcontext` and a CONTEXT literal in place of `to_stack`;
    and a conditional that runs its body on another stack. Assert each
    program's stacks and diagnostics match Tier 0's. It fails on any lowering
    that resolves the current stack once, or that syncs to the stack current
    at the sync rather than the one each value came from.

22. **What a promoted value must not change, doesn't.** Six parts, each
    asserted against Tier 0's result:

    - **An error with values promoted.** Compile `1 2 true +` with `1`
      promoted. `+`'s type guard declines, and its generic counterpart, the
      word called through its slot, returns `ADD returns error: Incompartible
      Y argument for the math operations`, as Tier 0 does (checked
      2026-09-10). The report and the `[BUND]  Content of the stack` dump must
      match. Then do the same with a callee that fails. A fragment op cannot
      be the one that fails, because criterion 19 makes a failure after the
      guard impossible to construct. An earlier wording used `1 2 "a" +`,
      which succeeds, because `+` joins an `Int` and a string (F64's
      pass-through family). It constructed no error.
    - **An effect changed at run time.** Promote across a call, and rebind that
      call's name to a word with a different effect, once before the body runs
      and once mid-body through `register`. The stacks must match.
    - **An alias whose target is rebound.** Promote across a call to `<-`, and
      rebind `stacks_left` to a lambda that consumes two, before the body runs
      and mid-body. The stacks must match. Repeat with `$stacks_left`, and
      with a callee whose slot generation is saturated.
    - **A lambda callee whose callee is rebound (D46).** With
      `:g { drop } register  :f { g } register`, run a body `1 2 3 f`, and
      rebind `g` to `{ drop drop drop }` before the body runs. Then use
      `:f { :g { drop drop drop } register g } register`, which rebinds `g`
      during the call. The stacks and diagnostics must match.
    - **A lambda that shadows a native (F93).** After
      `:drop { drop drop } register`, run a promoted body `1 2 3 drop`.
      `drop` resolves to the lambda, so nothing stays promoted across it,
      although the slot still holds the native's `(1, 0)`. The stacks must
      match Tier 0's.
    - **The reporter, observed through the diagnostic.** Under
      `CollectingReporter` with `wants_stack` set, run a promoted body
      `1 2 :x :x alias`. `alias` is `eff(2, 0)` and warns mid-body that `x`
      resolves back to itself (`alias`, `crates/bund2-stdlib/src/singles.rs`),
      with `1` and `2` promoted below its arity. The warning's snapshot in
      `CollectingReporter::seen` must equal Tier 0's, `1` and `2` included.
      That snapshot is the seam that shows whether anything was held across
      the call. `alias` is the only fixed-effect native that reports mid-body
      today; `execute.`, the other, is opaque since F87. An earlier wording
      used `?error`, which only pushes a CONDITIONAL. Its notice is reported
      when `!` runs that value, and `!` is opaque, so promotion had already
      stopped and the part could not fail. Then, under
      `TextReporter::new(true)`, the lowering's side table (criterion 17) must
      record at least one call crossed by promotion in the same body. Under the
      default reporter, promotion across calls must not be zero (D45).

    Needs a tier.

23. **A body compiled for one `Interp` is never run by another.** On one
    thread, build two `Interp`s, evaluate a body under the first until it is
    compiled, and evaluate the same `Rc` under the second. The second's cache
    must miss, so the body is interpreted or compiled again against the second
    `Interp`'s cells. After the first `Interp` is dropped, the second must still
    give Tier 0's result. This fails for any cache, module or cell shared
    across `Interp`s (§S6, *Addressing*). Needs a tier.

24. **Every `bund2-stdlib` native with a fixed effect keeps it.** Promotion
    stops at an opaque site (§S5), and after any other call it models the
    depth from the callee's pair. So a native that runs a body, dispatches a
    word, or moves the stack by something other than its pair breaks promotion
    silently.

    The check is an audit over every program `conform` runs: each line of
    `tests/golden/HERMETIC.txt`, and each probe with a golden. Every program
    runs in process with `Interp::effect_audit` on
    (`crates/bund2-interp/src/lib.rs`). While a fixed-effect native runs,
    three things are breaches: starting a body, filing a tail request, and
    dispatching a word. So is a change of the current stack's depth that
    differs from its pair, read as RFC-0004 §S1 reads it: a floor and a net,
    on the main stack. The test is
    `every_fixed_effect_native_keeps_its_effect_over_the_corpus`
    (`crates/bund2-stdlib/src/lib.rs`). **Met**, 2026-09-10.

    **It has failed on what it exists for, twice.**
    - At `961028c`, the lambda-only test beside it named `execute.` (F87).
    - Before F91, the corpus audit named `object`: a body started and three
      words dispatched in `class_constructors_demo.bund`. The same first run
      also found ten miscounted pairs (F92).

    It reaches the natives the corpus calls, with the operands the corpus hands
    them. A word the corpus never calls, or an arm the corpus never reaches, is
    criterion 28's: its palette checks the pair as well as body starts.
    `no_fixed_effect_native_runs_a_body`, beside it, checks body starts only.
    A dispatch the audit records is one through `Interp::call_native`. A method
    native or conditional runner that `bund2-stdlib` calls directly
    (`oop.rs`, `conditional.rs`) is invisible to it, and today only opaque
    words make such calls. A named-stack word
    handed the current stack's name is Q34's case, and F92 lists the words.
    Runs today: `cargo test -p bund2-stdlib honesty`.

25. **No native reports at `Error` severity.** §S5's reporter rule reads
    `Warning` and `Notice` because natives return errors rather than
    reporting them. `no_native_reports_at_error_severity`
    (`crates/bund2-stdlib/src/lib.rs`) scans the crate's shipped source for
    `Diagnostic::error` and `Severity::Error`. **Met**, 2026-09-10. A native
    that must report an error mid-body fails it, and §S5's rule then has to
    read `Error` too. It covers `bund2-stdlib` only, and so does the rule it
    supports: promotion crosses no other crate's native (D47, criterion 27).

    **The scan checks spellings, and knows it.** `Diagnostic`'s fields are
    public, so a native could build a warning and then assign its `severity`
    from a variable. So criterion 24's audit has a run-time half, added for
    the ninth review's S4 at the owner's choice. While any native runs, a
    `Vm::report` at `Error` severity is a breach, whatever spelling built it.
    The scan covers code the corpus never reaches. The audit covers every
    spelling on the paths it does reach.

26. **A body a call asks for runs before the next value.** Compile each of
    these twice, once with the call in a non-tail position and once in a tail
    position: `{ 10 } ! 20`, `true { 10 } if 20` and
    `:f { 10 } register f 20`. Each must leave `10` beneath `20`, as Tier 0
    does. Then compile `{ 1 } ! { 2 } !`, where a second request follows the
    first: both bodies must run, in order. The criterion fails for a lowering
    that ignores the request cell, one that drains in tail position instead of
    handing the request back, and one whose entry takes a returned request
    late (§S5, *A call may leave a body to run*). Needs a tier.

    Two more cases, for the tenth review's S3. A request filed inside a body
    the drain started must itself be drained before the drain returns. And a
    native that files a request and then fails, under `?try`, must leave no
    body to run afterwards (F96). The second case's Tier 0 half runs today:
    `a_failed_native_leaves_no_tail_request`
    (`crates/bund2-interp/src/lib.rs`).

27. **Promotion does not cross a native `bund2-stdlib` did not register
    (D47).** From the test, register a native declaring `eff(1, 1)` that
    replaces its operand with the current depth. Its pair is honest, and it
    still observes beyond its operand. Compile a body that promotes, calls it,
    and continues. The result must match Tier 0's, and the lowering's side
    table (criterion 17) must record the call as synced, not crossed. Needs a
    tier.

28. **Promotion crosses only natives a palette has checked (D48).** Criterion
    24 checks the arms the corpus reaches, and `drop_stack`, which no program
    calls, declared `1 -> 0` while removing the whole current stack (F94). So
    `every_fixed_effect_native_keeps_its_pair_over_the_promotable_palette`
    (`crates/bund2-stdlib/src/lib.rs`) runs every fixed-effect native under
    the effect audit against fourteen operand kinds. The top two operands take
    every pair of kinds. A workbench form also runs with every kind on the
    workbench. The test asserts two things: no run breaches its declared
    effect, and the natives some run brought to `Ok` are exactly those
    `tests/golden/PROMOTABLE.txt` lists. §S5's promotion crosses only those.
    **Met**, 2026-09-10, when 178 of 205 fixed-effect natives were listed;
    227 of 269 on 2026-09-11 (the list's own header line).
    **Mutation-checked**: before F94's fix it named `drop_stack` and nothing
    else, "declares (1, 0) and moved `main` from 4 to 0". The list certifies
    a pair, not what else a native observes, so a Q34 observer is still
    criterion 14's. Runs today: `cargo test -p bund2-stdlib promotable`.

    **Two limits, stated for the eleventh review's S4.** First, six natives
    that act on the host are never run: `fs.rm`, `sleep.seconds`,
    `system.setproctitle`, `system.setproctitle.`, `password` and
    `save.model` (`ACTS_ON_HOST`, `crates/bund2-stdlib/src/lib.rs`). So
    "every fixed-effect native" means every one but those. That is the
    conservative side, since an unrun native is not listed and not crossed.
    But the exclusions are kept by hand, so a new host-acting native runs for
    real under `cargo test` until it is added. Second, the list names natives,
    so it certifies the **default** registration. `--noio` and `--noeval`
    register failing stubs under the same names (`register_all_with`), and
    D47's id set is taken from the registration actually made. A listed stub
    is therefore crossed under those options, and since every stub fails, the
    crossing takes the error path, which syncs first.

29. **A native that panics gives Tier 0's result from compiled code (D49).**
    Register a native that panics. Call it from a compiled body in a non-tail
    and a tail position, and assert the same observable result Tier 0 gives:
    `Error::internal` naming the native, reported through the reporter, with
    nothing on stderr and no abort. It fails for an adapter that lets the
    panic unwind, which would abort through `extern "C"` or unwind through
    frames with no unwind tables (§S8). The Tier 0 half is **Met**:
    `a_panicking_native_is_an_internal_error_not_an_unwind`
    (`crates/bund2-interp/src/lib.rs`).
    F95's `jarowinkler` program reports, and exits 0. The compiled half needs
    a tier.

30. **A program ended by `bund.exit` ends at the same point in compiled code
    (D52).** Run `tests/probes/bund-exit.bund` and
    `tests/probes/bund-exit-word.bund` under criterion 2's three
    configurations. Then compile, with promotion on, bodies that call
    `bund.exit` directly, through the alias `exit`, and in tail position, each
    followed by a call and by an inlined `+`. Output, exit code and final
    stacks must match Tier 0's. It fails for an adapter that returns success
    after a recorded exit (§S5, *A call may end the program*). The audit's
    half runs today: a fixed-effect native that requests an exit is a breach,
    `an_exit_requested_under_a_fixed_effect_is_a_breach`
    (`crates/bund2-interp/src/lib.rs`). The compiled half needs a tier.

## Open questions

- **Q25 — what replaces the stack tag on `push`?** §S1 requires
  `value/push_pull/balanced` under 20 ns and deliberately does not say how.
  The tag is observable (goldens capture `tags: {"stack": "main"}`), so the
  options — write-on-observe, interned stack names, or moving the tag to the
  stack's bookkeeping — differ in what they preserve. **Answered by D41**
  (interned stack names, in the value's padding), which §S1's update records;
  this RFC is no longer blocked on it.
- **Q26 — resolved while drafting RFC-0001's amendment, not open.** The
  `identity()` call on every push is **D13's policy**: RFC-0001's "One policy
  for the `Rc` and the identity slot" requires a CoW split to materialise
  identity before copying, and names `set_tag` on push as the case that fires
  it. What is over-applied is that it is unconditional; that is option 1 of the
  amendment, not a separate question.
- **Q27 — which target profile is Bund2 optimising for?** §3.2a's code-memory
  constraint "bites hard" on a REPL and "barely at all" on long-running batch,
  and §S7's caps are chosen for the former. The study raised this as its own
  open question 7 and it is still unanswered.
- **Q28 — does `s390x` matter?** §S8 degrades tail calls there. If s390x is
  not a target, the degradation path is dead code that will not be tested.
- **Q34 — answered 2026-09-11 by the owner: D55.** The set is derived by
  criterion 28's palette, through a four-run differential and an observation
  audit, and kept off `PROMOTABLE.txt`. Criterion 14 is satisfiable.
- **Q35 — answered 2026-09-10 by the owner: `q` is kept but not averaged.**
  D32 is amended to match, and §S6's constraint 2 now rests on it.
- **Q36 — answered 2026-09-10: D42.** The value reaches every entry point;
  built.
- **Q37 — answered 2026-09-10: D43.** The registration id and the stable
  generation cells, built when this RFC reaches Proposed.
