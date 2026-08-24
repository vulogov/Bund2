# Bund2 — RFC Roadmap and Readiness Assessment

**Status:** Planning
**Date:** 2026-08-23
**Purpose:** Define the RFC set, fold the accumulated improvements and fixes into their
homes, and state honestly which RFCs can be drafted now and which cannot.

---

## 0. Short answer

**RFC-0, RFC-2, RFC-6, RFC-8 and RFC-9 can be drafted now.**
**RFC-1 can be drafted but not accepted** — it depends on Phase 0 measurement and two
semantic decisions.
**RFC-3, RFC-4 and RFC-7 cannot be drafted responsibly yet** — each has a specific,
bounded body of unread source behind it, listed in §3.

The blocker is not analysis. It is that roughly 6,000 LOC of the 19,000-line CLI directly
determine the IR design, the effect signatures, and the async model, and I have read the
word *names* but not the *implementations*. For a project whose stated method is verified
source grounding before design claims, that gap has to close before those three go on
paper.

---

## 1. New findings from this pass

Four things surfaced while surveying the repository that change earlier documents.

### 1.1 Control flow is partly data

`conditional/` (872 LOC) implements `?ifthenelse`, `?try`, `?error`, `curry`, `context`,
`fmt`, `csv`, `sqlite` as **CONDITIONAL values**: `?ifthenelse` pushes an empty
`Value::conditional()` tagged `type: "ifthenelse"`; its `if` / `then` / `else` slots are
populated with lambdas afterwards; `conditional_run` pulls them out and `lambda_eval`s
them. `curry` is the same shape with `name` and `data` slots.

So `?ifthenelse` is not a control-flow construct the compiler can see — it is a MAP built
at runtime and dispatched through `execute`. BundIR needs CONDITIONAL as a first-class
value with a helper-call execution path, not as structured control flow. Literal
`if` / `times` / `while` / `loop` remain statically analysable; the `?`-family does not.

### 1.2 A fourth suffix convention

Earlier documents recorded `.` (workbench) and `*` (fold whole stack). There is also `,` —
`math.max,`, `stat.count,`, `get,`, `set,`, `forecast.mstl,` — and the combined form `,.`
(`math.max.,`). The parser/IR RFC must treat suffixes as a systematic naming scheme, not
three ad-hoc cases.

### 1.3 Stack-consuming lambda construction is a core idiom, not an edge case

`examples/bund_dynamic_demos/` shows the canonical metaprogramming pattern: switch to a
scratch stack, push values, `call,` to fabricate a CALL from an atom, then `lambda*` to
fold the *entire stack* into a LAMBDA, then `register`. This answers the open question
"how common is dynamically-created code" — it is the documented, idiomatic way to write
Bund. Dynamic lambdas are not a corner to be tolerated; they are the feature.

It also means `lambda*` is variadic over the whole stack, so metaprogramming regions are
optimization barriers by construction. That is fine — they are cold — but it should be
stated in the effect RFC rather than discovered later.

### 1.4 A distributed layer exists that no document has accounted for

`bus/` + `crossbus.rs`, `helpers/zenoh/`, `helpers/world/`, `cmd/bund_bus.rs`,
`bund_bbus.rs`, `bund_cluster.rs`, and the words `send`, `recv`, `send.quick`,
`bus.data`, `debug.display_distributed_info`. Zenoh is a real pub/sub transport.

The async addendum proposed an actor model with envelope message passing as though it were
a new design. **It may already exist.** RFC-7 cannot be written without reading this, and
if the existing model is sound the async RFC becomes "preserve and formalise" rather than
"design".

### 1.5 Corpus correction

The CLI has **12** test files but **144** example programs. The examples are the better
differential-testing corpus and the better empirical evidence for every "how common is X"
question. `Documentation/Bund_Library_Guide/` (Typst source, per-word `description` /
`sample` / `algorithm` fragments) is the closest thing to a language specification and
should be the **normative reference** against which "100% preserved" is judged.

---

## 2. The RFC set

Numbered by dependency, not by importance.

| RFC | Scope | Depends on |
|---|---|---|
| **RFC-0** | Architecture overview, monorepo layout, crate boundaries, tier model, terminology, defect register | — |
| **RFC-1** | `BundValue`: 24-byte layout, `Rc::make_mut` value semantics, lazy `.id`, sampled `.timestamp`, heap payload types, serialisation | Phase 0 |
| **RFC-2** | Symbols and the word slot table: interning, `WordEntry`, generations, `bund2-api` stable surface, `StackEffect` declaration, native word kinds (`Sync`/`Blocking`/`Async`), registry builder replacing the `STDLIB` global | RFC-1 |
| **RFC-3** | `bund2-syntax` + `bund2-ir` + Tier 0: AST, BundIR, spans, scoped blocks, lambda representation, object format, flat frame loop, inline caches | RFC-1, RFC-2 |
| **RFC-4** | Stack effects: native annotations, inference for Bund words, abstract-stack analysis, `bund2 check` linter, `?effect` | RFC-3 |
| **RFC-5** | Cranelift backend: `BundIR → CLIF`, slot promotion, guard-and-branch specialization, type feedback, tail calls, escape analysis, tiering policy, code-memory caps | RFC-4 |
| **RFC-6** | AOT and linking: `cranelift-object`, `cc` driver, `--emit` matrix, runtime archive distribution, ABI marker symbol, closed-world analysis | RFC-5 |
| **RFC-7** | Concurrency and async: actor model, envelopes, `Blocking`/`Async` native words, tier pinning, shared compile service — **and reconciliation with the existing bus/zenoh layer** | RFC-2, RFC-3 |
| **RFC-8** | Debugger and observability: Tier-0 step mode, breakpoints, stack watchpoints, backtraces, trace stream, profiler | RFC-3 |
| **RFC-9** | OOP: class/object representation, flattened vtables, polymorphic inline caches, `m()` dispatch, `.id`/`.timestamp` resolution | RFC-1, RFC-2 |
| **RFC-10** | Tooling and infrastructure: benchmark corpus, differential harness, grammar fuzzer, image/snapshot startup, superinstruction mining | RFC-3 |

---

## 3. Readiness, per RFC

### Draftable now

**RFC-0.** Everything needed is in the four prior documents. Add the defect register (§5)
and the decision register (§4).

**RFC-2.** The resolution chain, the five name-keyed tables, the alias double-resolution,
`methods_fun`, `register`/`unregister`/`alias`, and the `STDLIB` global have all been read
in full. Nothing outstanding.

**RFC-6.** `cranelift-object` capability and the `cc` decision are settled. Closed-world
analysis is specified; §1.1 and §1.3 only reinforce that it will rarely fire.

**RFC-8.** `debug_fun/` read in full (853 LOC). The design rests on RFC-3's frame stack,
which is specified even if RFC-3 itself isn't final.

**RFC-9.** `oop/` read in full (1,183 LOC), plus `bund_object.rs` and
`multistackvm_object.rs`. The vtable-flattening / field-tree-preservation split is
grounded.

### Draftable, not acceptable

**RFC-1.** The design is settled. What is missing:

- **Phase 0 measurement.** `size_of::<Value>()` is layout arithmetic, not measured. The
  allocation counts are hand-traced. No benchmark exists, so the RFC has no acceptance
  criteria that can be checked.
- **Two semantic decisions** (§4, D1 and D2).

Draft it, mark it *Proposed*, and hold acceptance until Phase 0 returns numbers.

### Not draftable yet

**RFC-3** — blocked on ~3,100 LOC:

| Must read | LOC | Why it blocks |
|---|---|---|
| `functions/conditional/` | 872 | CONDITIONAL/curry/try as data (§1.1) — determines whether IR needs a value form for control flow |
| `functions/values/` | 798 | `push`/`pull`/`get,`/`set,`/`merge`/`unfold`/`listop`/`make_call` — the value-manipulation vocabulary the IR must express |
| `functions/bund/` | 1,411 | `bund_eval`, `bund_fun` (`lambda!`/`lambda*`/`lambda=`), `bund_save`/`bund_load`, `bund_interpreter`, `bund_class`, `bund_use` — the metaprogramming surface, which is the IR's hardest constraint |

Plus: the `,` and `,.` suffix semantics (§1.2), and a sample of `examples/` to validate that
the IR can express the idioms in §1.3.

**RFC-4** — blocked on the largest single gap: **stack effects for all 357 words**. I have
the names, not the arities. This is mechanical but not small — roughly 12,000 LOC of stdlib
to read, or a harness that infers arity empirically by executing each word against
instrumented stacks. The empirical route is probably faster and should be considered:
generate candidate stacks, call each word, record depth delta. It will not resolve the
variadic cases, but it will resolve the other ~90%.

**RFC-7** — blocked on §1.4. `bus/` (499 LOC), `helpers/zenoh/`, `helpers/world/`,
`cmd/bund_bus.rs`, `bund_bbus.rs`, `bund_cluster.rs`. Until that is read, the async
addendum's actor proposal is a design for something that may already be built.

**RFC-5** — not blocked on reading, blocked on RFC-4 and on the Phase 3 baseline it is
gated against. Draft last.

**RFC-10** — the benchmark corpus and differential harness parts can and should be built in
Phase 0, ahead of the RFC.

---

## 4. Decision register

These are choices, not research. Every one blocks an RFC.

| # | Decision | Blocks | Default if undecided |
|---|---|---|---|
| D1 | Does `.id`'s exact nanoid format matter, or is "unique opaque string" the contract? | RFC-1 | Lazy nanoid from counter + seed (preserves format) |
| D2 | Is `.timestamp` ms-granularity, or must two values built in sequence differ? | RFC-1 | Sampled clock at ms granularity |
| D3 | Is `bund.eval`'d code JIT-eligible, or permanently Tier 0? | RFC-5, RFC-3 | Permanently Tier 0 |
| D4 | Full `i64` required, or would 51-bit integers be acceptable? | RFC-1 | Full `i64` (24-byte value) |
| D5 | Can a lambda body be mutated after construction, or is it write-once? | RFC-3 | Assume mutable; invalidate compiled cache |
| D6 | Is fine-grained async required, or is VM-per-task enough? | RFC-7 | VM-per-task |
| D7 | Expected concurrent VM count — tens or thousands? | RFC-7 | Tens |
| D8 | Do external Rust word packages exist outside this repo? | RFC-2 | No; design `bund2-api` freely |
| D9 | Should `Intrinsic` CLIF lowerings ever be third-party? | RFC-2 | No |
| D10 | Is `bund2 build` allowed to require a C toolchain? | RFC-6 | Yes; `--emit=bundle` covers the rest |
| D11 | Does anything external depend on `compile_to_binary`'s bincode format? | RFC-3 | No; version the IR format |
| D12 | Restrict the `*`-family in JIT-able positions, or accept it as a permanent barrier? | RFC-4 | Accept as barrier |
| D13 | `Rc::make_mut` value semantics confirmed as the correct reading of today's behaviour? | RFC-1 | Yes |

D1, D2 and D13 are the urgent ones — they gate RFC-1, which gates everything.

---

## 5. Defect register

Found while reading; all should be recorded in RFC-0 and fixed regardless of direction.

| # | Defect | Location |
|---|---|---|
| F1 | `"unregister"` registered twice; the class variant shadows the lambda variant, making lambda unregistration unreachable by name | `rust_multistackvm/src/stdlib/lambdas/registry.rs` |
| F2 | `stdlib_logic_if_false_in_workbench` passes `StackOps::FromStack`, not `FromWorkBench` | `rust_multistackvm/src/stdlib/logic/if_fun.rs` |
| F3 | `stdlib_math_op_inline` checks `current_stack_len()` in the `FromWorkBench` arm before checking `workbench.len()` — wrong stack | `rust_multistackvm/src/stdlib/math/math_op.rs` |
| F4 | `push_to_workbench` clones an owned value then drops the original | `rust_multistack/src/ts_workbench.rs` |
| F5 | `get_inline` builds `format!("{}_inline", name)` twice (once for `contains_key`, once for `get`); `is_inline` builds it a third time | `rust_multistackvm/src/multistackvm_inline.rs` |
| F6 | Alias resolved twice per CALL — in `apply` and again in `i()` | `multistackvm_apply.rs`, `multistackvm_inline.rs` |
| F7 | `time_graph::instrument` on `apply`, `i`, `i_direct`, `call`, `lambda_eval`, `stdlib_execute_base_inline`, `stdlib_logic_if_base`, `stdlib_logic_times` — instrumentation in the dispatch path | multiple |
| F8 | Inter-crate dependencies pinned `">=0.*.*"` — unbounded; a `Value` layout change propagates silently | all five `Cargo.toml` |
| F9 | Parser's `ctx` rule mutates the caller's `state` vector — parsing has a side channel | `bund_language_parser/src/vm/ctx.rs` |
| F10 | Debugger history files written to CWD | `debug_fun/debug_debug.rs`, `debug_shell.rs` |
| F11 | `register_method_value_init` writes `if ! value.type_of() == OBJECT` — parses as `(!value.type_of()) == OBJECT`; the guard never fires as intended | `oop/value_class.rs` |

F8 is resolved by the monorepo decision. F1, F2, F3 and F11 are behaviour bugs; fixing them
is technically a deviation from "100% preserved" and each needs an explicit call — preserve
the bug or fix it. I would fix all four and note them in the RFC.

---

## 6. Improvements, assigned to RFCs

Everything recommended across the four prior documents, placed.

| Improvement | RFC |
|---|---|
| `Rc::make_mut` clone-on-write value semantics; cycles impossible | RFC-1 |
| 24-byte `{tag, payload, birth}`; lazy `.id`; sampled `.timestamp` | RFC-1 |
| Symbol interning; single slot table; generation counters | RFC-2 |
| `StackEffect` on native registration; `Sync`/`Blocking`/`Async` kinds | RFC-2 |
| Registry builder replacing the `STDLIB` global mutex | RFC-2 |
| `bund2-api` as the sole stability-guaranteed crate | RFC-2 |
| Lambda body stays `Vec<BundValue>`; compiled cache keyed by content hash | RFC-3 |
| Flat frame loop, no Rust recursion (fixes stack overflow; enables async + debugger) | RFC-3 |
| Spans in IR; structured error values carrying position | RFC-3 |
| Scoped blocks replacing the parser side channel (F9) | RFC-3 |
| Monomorphic inline caches on word call sites | RFC-3 |
| Effect *inference* for Bund words; annotation only for natives | RFC-4 |
| `bund2 check` — static stack-underflow detection | RFC-4 |
| `?effect` word; effect column in debugger `words` view | RFC-4 |
| Type feedback from Tier 0 into Tier 1 guard ordering | RFC-5 |
| Escape analysis for anonymous lambdas | RFC-5 |
| Tail calls for self-recursive words (`CallConv::Tail`) | RFC-5 |
| Code-memory caps; recompile caps; demotion policy | RFC-5 |
| Profile-guided AOT using the same feedback table | RFC-6 |
| ABI marker symbol (`__bund2_runtime_abi_v1`) for link-time version checking | RFC-6 |
| `--emit=bundle` / `obj` / `native`; `--print-link-command`; `--keep-temps` | RFC-6 |
| Actor model; envelopes; shared JIT compile service | RFC-7 |
| Tier pinning for await points and breakpoints (one mechanism) | RFC-5, RFC-7, RFC-8 |
| Tier-0 step mode; backtraces; breakpoints; **stack watchpoints** | RFC-8 |
| Trace stream replacing `time_graph`; profiler from tier counters | RFC-8 |
| Flattened per-class vtables; field tree preserved | RFC-9 |
| Polymorphic inline caches on `m()` dispatch | RFC-9 |
| Benchmark corpus committed to the repo | RFC-10 |
| Differential harness: Tier 0 vs Tier 1 vs AOT across the 144 examples | RFC-10 |
| Grammar-directed fuzzer from `bund.pest` | RFC-10 |
| Image/snapshot startup building on existing `save.*` words | RFC-10 |
| Superinstructions mined from execution traces by frequent-sequence mining | RFC-10 |

---

## 7. Recommended order

1. **Phase 0 now, before any RFC is accepted.** Benchmark corpus, `size_of::<Value>()`,
   empirical allocation counts, and a differential harness skeleton over the 144 examples.
   This is also the cheapest way to answer D1, D2, D5 and D12 — the examples are evidence.
2. **Draft RFC-0** with the defect and decision registers. It is the index everything else
   hangs from.
3. **Answer D1, D2, D13**, then draft RFC-1 and accept it against Phase 0 numbers.
4. **Draft RFC-2 and RFC-9** — both fully grounded, and RFC-9 validates RFC-1's `.id`
   decision against a real consumer.
5. **Read the three subsystems in §3** (conditional, values, bund — ~3,100 LOC), then draft
   RFC-3. This is the critical path.
6. **Build the arity-probing harness**, then RFC-4.
7. **Read the bus/zenoh layer**, then RFC-7.
8. RFC-5, RFC-6, RFC-8, RFC-10 follow.

The honest summary: **RFC-0 through RFC-2 and RFC-6, RFC-8, RFC-9 are ready or nearly
ready. RFC-3, RFC-4 and RFC-7 each need a specific, bounded read first — about 3,600 LOC
for RFC-3 and RFC-7 together, and a mechanical pass over the stdlib for RFC-4.** That is a
few days of reading, not a research programme, and it is exactly the kind of grounding this
project has been doing all along.
