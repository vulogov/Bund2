# Bund → JIT: State of the Project and Feasibility Study

**Subject:** Re-implementing Bund as a JIT-compiled, dynamically typed, metaprogramming, multi-stack concatenative language on Cranelift
**Status:** Research / pre-RFC
**Date:** 2026-08-23

---

## 0. Summary of findings

The five crates were cloned and read at HEAD. Every claim below is grounded in a specific
file in that source; no design conclusion is drawn from the READMEs alone.

Three findings drive the whole recommendation:

1. **Bund has no IR.** The parser emits `Vec<rust_dynamic::value::Value>` — live runtime
   objects, not an AST. `compile()` wraps that vector into a `LIST` value. There is nothing
   in the system that a code generator could consume. Building a JIT means first building
   the missing middle of the compiler.

2. **The dominant cost is the value representation, not interpretive dispatch.** A single
   `2 2 +` costs on the order of a dozen heap allocations, and roughly none of them are
   removed by compiling the dispatch away. Cranelift generates code within ~2% of V8's
   optimizing tier; that number is irrelevant when every integer literal calls `nanoid!()`
   and `SystemTime::now()`.

3. **`cranelift-jit` cannot redefine or free an individual function.** The 0.134/0.135 API
   is `declare` → `define` → `finalize_definitions` → `get_finalized_function`, plus
   `free_memory(self)` which consumes the entire module. For a language whose `register`,
   `alias`, and REPL workflows rebind words at runtime, this is the single hardest
   constraint in the project and it must be designed around at the architecture level, not
   patched later.

**Verdict:** feasible, but not as a single project. The work splits cleanly into a
representation rewrite that is worth doing on its own merits and yields most of the
available speedup, and a Cranelift tier that is worth doing only after the first is
measured. Attempting them together is how this kind of project stalls.

---

## 1. Current state of the project

### 1.1 Crate inventory (measured at HEAD)

| Crate | Version | `src/*.rs` files | LOC | Role |
|---|---|---|---|---|
| `rust_dynamic` | 0.50.0 | 59 | 5,044 | Dynamic value type |
| `rust_multistack` | 0.33.0 | 41 | 2,047 | Named stacks + workbench |
| `rust_multistackvm` | 0.38.0 | 82 | 5,292 | VM, dispatch, 156-word stdlib |
| `bund_language_parser` | 0.15.0 | 18 | 323 | pest grammar → `Vec<Value>` |
| `bundcore` | 0.8.0 | 11 | 583 | `eval`/`run` façade |
| **Total** | | **211** | **13,289** | |

The dependency graph is a clean stack: `rust_dynamic` ← `rust_multistack` ←
`rust_multistackvm` ← `bundcore`, with `bund_language_parser` depending only on
`rust_dynamic`. That layering is genuinely good and should survive the rewrite.

The test suite is a real asset: ~30 integration test files across the crates, covering
math, logic, lambdas, stack ops, JSON, string, conversion, classes, and vars. It is the
differential-testing oracle for everything proposed below.

**Dependency pinning is a hazard.** Four crates depend on each other with `">=0.*.*"`.
That is effectively unbounded. A `Value` layout change in `rust_dynamic` propagates
silently into the VM. Before any of this work starts, pin to caret ranges within a
workspace.

### 1.2 The front end

`bund.pest` is small — 13 value rules. The term alternation is:

```
float | integer | lambda | list | ctx | ptr | name | command | atom | stack | string | literal
```

Lexical surface, verified against the grammar:

| Form | Rule | Produces |
|---|---|---|
| `42`, `42.0` | `integer`, `float` | `INTEGER` / `FLOAT` value |
| `"..."`, `'...'`, `:atom` | `string`, `literal`, `atom` | `STRING` value |
| `word` | `name` | `CALL` value |
| `` `word `` | `ptr` | `PTR` value |
| `:` / `;` | `command` | `CALL` value (autoadd on/off) |
| `{ ... }` | `lambda` | `LAMBDA` value (nested `Vec<Value>`) |
| `[ ... ]` | `list` | `LIST` value |
| `( ... )` | `ctx` | anonymous `CONTEXT` + `endcontext` `CALL` |
| `@name` | `stack` | named `CONTEXT` value |

Two structural problems here:

- **No AST.** `bund_parse` returns `Vec<Value>`. `compile()` folds it into a `LIST`;
  `compile_to_binary()` bincode-serialises that. So Bund's "object format" is a serialised
  tree of runtime values, each carrying a `nanoid` and a wall-clock timestamp. There is no
  place to hang source spans, no place to record stack effects, and no representation of
  control flow — a lambda is just a `Vec<Value>`.

- **The parser has a side channel.** `vm/ctx.rs::process_token` pushes `Value::context()`
  onto the shared `state` vector and appends parsed terms to it, then returns an
  `endcontext` `CALL` as its own result. Parsing `( ... )` therefore mutates state
  belonging to the caller's accumulator rather than returning a subtree. This works today
  but it makes `( ... )` un-analysable and must be replaced by a proper scoped-block node.

### 1.3 The execution model

`Bund::eval` (`bundcore_eval.rs`) parses to `Vec<Value>` and feeds each element to
`VM::apply`. `VM::apply` (`multistackvm_apply.rs`) is the entire interpreter:

- `dt == CALL` → name resolution (below)
- `dt == CONTEXT` → `to_stack(name)`, switching the current stack
- anything else → `stack.push(value)`

…except that when `autoadd` is set, *every* branch changes meaning: values and calls alike
are popped-and-appended into the value on top of the stack instead of being pushed or
executed. `autoadd` is a `bool` field on `VM`, toggled by the `:` and `;` commands.

Name resolution for a `CALL`, in order:

1. `is_command(name)` → dispatch from `command_fun`
2. leading `$` → `call_internal_word`, bypassing lambdas and aliases
3. `get_alias(name)` → substitute
4. `is_lambda(real_name)` → `get_lambda` (which **clones** the lambda) → `lambda_eval`
5. otherwise `i(real_name)` → **alias resolution again** → `i_direct` → `inline_fun`, else
   fall through to the `TS`-level inline table

`lambda_eval` iterates the lambda's `Vec<Value>` calling `apply` on each — so lambdas are
re-walked as data on every invocation, with no memoisation of resolution.

`stdlib_execute_base_inline` (`stdlib/execute.rs`) is the dynamic-dispatch entry point:
it pops a value and dispatches on its type across `PTR|STRING|CALL`, `LIST`, `MAP|INFO|
CONFIG|ASSOCIATION`, `CONDITIONAL`, `CLASS`, `OBJECT`, `LAMBDA`. This is the fully dynamic
path and it is genuinely dynamic — it can execute a dictionary entry chosen by a key
pulled off the stack.

The VM registers **156 inline words** and **2 commands**. The naming conventions are
regular and worth preserving: a trailing `.` denotes the workbench variant (`+.`, `car.`,
`if.in_workbench`), a leading `*` denotes the fold-the-whole-stack variant (`*+`, `*-`,
`*loop`).

### 1.4 The multi-stack

`TS` (`rust_multistack/src/ts.rs`) holds `HashMap<String, Stack<Value>>`, a `VecDeque<String>`
of stack names, per-stack capacities, and a single `workbench: Stack<Value>`. `Stack<T>` wraps
`VecDeque<T>` with a `policy: bool` selecting LIFO or FIFO. The VM adds its own
`stacks_stack: VecDeque<String>` on top.

The model is coherent and is the distinctive thing about Bund. It is also the part that
constrains code generation hardest: the *current* stack is a runtime value, resolved by
name through a `HashMap` on every push and pull.

### 1.5 Cost analysis of the hot path

This is the core of the assessment. Trace `2 2 +` through the source.

**Per `Value` constructed** (`rust_dynamic/src/create.rs`, e.g. `from_int`):

```rust
Self {
    id:    nanoid!(),          // 21-char random string: RNG + heap allocation
    stamp: timestamp_ms(),     // SystemTime::now()
    dt:    INTEGER,
    q:     100.0,
    data:  Val::I64(value),
    attr:  Vec::new(),
    curr:  -1,
    tags:  HashMap::new(),
}
```

By layout arithmetic the struct is ~180 bytes (`String` 24 + `f64` 8 + `u16` + `f64` 8 +
`Val` ~64 + `Vec` 24 + `i32` + `HashMap` 48). *This is computed from field types, not
measured — no Rust toolchain was available in this session, so treat it as an estimate to
be confirmed by `size_of::<Value>()`.* The allocation and the clock read, however, are not
estimates; they are unconditional in every constructor.

**Per push** (`rust_multistack/src/ts_push.rs`):

```rust
value.set_tag("stack", &curr.stack_id());
```

and `set_tag` (`rust_dynamic/src/tags.rs`) is `self.tags.insert(key.as_ref().to_string(),
value.as_ref().to_string())` — two `String` allocations plus a `HashMap` insert, on every
push, to record a stack ID that nothing on the hot path ever reads.

**Per word call.** Counting `String` allocations for a plain, non-aliased inline word:

| # | Site | Allocation |
|---|---|---|
| 1 | `apply` | `value.cast_string()` → `s_val.to_string()` |
| 2 | `apply` | `is_command(fun_name.clone())` |
| 3 | `apply` | `get_alias(fun_name.clone())` |
| 4 | `apply` | `is_lambda(real_name.clone())` |
| 5 | `apply` | `i(real_name.clone())` |
| 6 | `i` | `is_alias(name.clone())` |
| 7 | `i` | `i_direct(name.clone())` |
| 8 | `is_inline` | `format!("{}_inline", &name)` |
| 9 | `get_inline` | `format!("{}_inline", &name)` — for `contains_key` |
| 10 | `get_inline` | `format!("{}_inline", &name)` — again for `get` |

Ten allocations and five hash lookups to reach `stdlib_add_inline`. The word table is
keyed by `name + "_inline"`, so the suffix is rebuilt from scratch three times per call.

**Totalled**, `2 2 +` costs roughly: 2 literal constructions (2 nanoid + 2 clock reads),
2 pushes (4 String allocs + 2 HashMap inserts), the dispatch above (10 allocs), the result
`Value` (1 nanoid + 1 clock read), and one more push (2 allocs + insert). Call it 20+ heap
allocations, 3 RNG calls and 3 clock reads for one integer addition.

**The implication for this project is decisive:** compiling the dispatch away removes items
2–10 in that table and nothing else. The nanoids, timestamps, tag inserts and 180-byte
copies survive intact, because they live in `Value`, and JIT'd code that manipulates
`Value` pays them exactly as the interpreter does.

### 1.6 Additional hot-path costs

- **Lambda invocation deep-clones the body.** `get_lambda` returns `lambda.clone()` — a
  clone of `Vec<Value>`, recursively cloning every element's `String` id, `Vec` attrs and
  `HashMap` tags.
- **`times` clones per iteration.** `stdlib_logic_times` calls `vm.lambda_eval(lambda_val.clone())`
  inside the loop. An N-iteration loop performs N deep clones of the loop body.
- **`push_to_workbench` clones redundantly**: `self.workbench.push(value.clone())` where
  `value` is owned and then dropped.
- **`time_graph::instrument` is on hot functions** — `apply`, `i`, `i_direct`, `c`, `call`,
  `lambda_eval`, `stdlib_execute_base_inline`, `stdlib_logic_if_base`, `stdlib_logic_times`.
  Instrumentation in the dispatch path should be feature-gated before any benchmarking.

### 1.7 Incidental defects found while reading

Not central to the study, but worth fixing regardless of which direction the project takes:

- `stdlib/lambdas/registry.rs::init_stdlib` registers `"unregister"` twice — first to
  `stdlib_lambda_unregister`, then to `stdlib_class_unregister`. Since `register_inline`
  unregisters before inserting, the second wins and lambda unregistration is unreachable
  by name. The class variant is presumably meant to be `unregister.class`.
- `stdlib/logic/if_fun.rs`: `stdlib_logic_if_false_in_workbench` passes
  `StackOps::FromStack`, not `FromWorkBench` — the `.in_workbench` behaviour is not what
  the name says.
- `stdlib_math_op_inline` checks `current_stack_len()` in the `FromWorkBench` arm before
  separately checking `workbench.len()`; the first check is against the wrong stack.

---

## 2. What "JIT-compiling Bund" can and cannot buy

### 2.1 Decomposing the available speedup

Order-of-magnitude reasoning, to be replaced by measurement in Phase 0:

| Change | Removes | Rough expectation | Needs Cranelift? |
|---|---|---|---|
| Value representation rework | nanoid, clock read, tags HashMap, 180-byte copies | large multiple | no |
| Symbol interning + slot table | 10 allocs + 5 lookups per word | large multiple | no |
| IR + threaded interpreter | per-token re-parse of `Value` trees, lambda clones | ~2× | no |
| Cranelift tier: naive lowering | interpreter loop overhead only | ~1.2–1.5× | yes |
| Cranelift tier: stack-slot promotion + type guards | stack traffic, tag checks, boxing | ~2–5× | yes |

The first two rows do not require a code generator and are prerequisites for the last row
being worth anything. A JIT that emits `call stdlib_add_inline` in sequence is a slower,
more fragile version of the interpreter.

### 2.2 The honest framing

There are two projects here:

- **Project A — representation.** Rewrite `Value`, intern symbols, introduce an IR, build a
  threaded interpreter over it. Touches most of `rust_dynamic` and `rust_multistackvm`.
  Delivers the majority of the achievable performance. Ships on every target Rust supports.
- **Project B — Cranelift tier.** A second execution tier for hot words. Delivers a
  further multiple on numeric and stack-heavy code. Constrains target platforms, adds a
  large dependency surface, and introduces the tiering/invalidation/code-memory problem.

Project A is worth doing whether or not Project B ever happens. Project B is worth doing
only if Project A's measurements show that dispatch and boxing are still the bottleneck.
The recommendation is to sequence them and put a hard decision gate between them.

---

## 3. Cranelift: capability assessment

Versions checked against crates.io on 2026-08-23: `cranelift-codegen`, `cranelift-frontend`,
`cranelift-module`, `cranelift-jit` all at **0.135.0**, published 2026-08-20. Release
cadence is roughly monthly, tied to Wasmtime's train.

### 3.1 What fits Bund well

- **Pure Rust, no LLVM.** Matches the project's existing dependency posture. No C++
  toolchain, no bindgen, no system library.
- **Compile speed.** Roughly an order of magnitude faster codegen than an LLVM-based
  pipeline, with output within ~2% of V8's optimizing tier on the published Wasmtime
  benchmarks. That trade — fast compile, good-enough code — is exactly the right shape for
  a REPL-driven language.
- **Tail calls.** `CallConv::Tail` with `return_call` / `return_call_indirect` is
  supported on x86-64, aarch64 and riscv64. For a concatenative language this is directly
  useful: a word body can be compiled as a chain of tail calls, and threaded-code designs
  become expressible without stack growth. Note s390x historically lacked tail-call support.
- **`cranelift-frontend` handles SSA construction.** `FunctionBuilder` with `Variable`
  declarations does the phi/block-parameter work, so lowering a stack machine to CLIF does
  not require writing an SSA constructor.

### 3.2 The hard constraints

**(a) No per-function redefinition or deallocation.** The `JITModule` surface is
`new`, `declare_function`, `define_function`, `define_function_bytes`, `declare_data`,
`define_data`, `finalize_definitions`, `get_finalized_function`, `get_finalized_data`,
`get_address`, and `free_memory(self)`. The documentation for `get_finalized_function`
states the pointer stays valid "until either `JITModule::free_memory` is called or in the
future some way of deallocating this individual function is used" — i.e. that mechanism
does not exist today.

Consequences for Bund specifically, because Bund's `register`/`unregister`/`alias` words
mutate the word table at runtime and the REPL is a primary workflow:

- every inter-word call in JIT'd code must be **indirect through a runtime-owned slot**,
  never a direct `call` relocation to a `FuncId`;
- redefining a word means compiling a *new* function and writing its pointer into the
  slot — the old code is orphaned and its pages are never reclaimed;
- code memory grows monotonically for the life of the process.

This is survivable but must be budgeted: cap total JIT'd functions, cap recompiles per
word, and demote pathological redefiners permanently to the interpreter tier. A
module-rotation scheme (build a fresh `JITModule`, re-JIT live words, `free_memory` the
old one) is possible in principle but requires proving no orphaned frame is live, which
needs a shadow stack. Do not attempt it in v1.

**(b) No deoptimization or OSR.** Cranelift is a code generator, not a JIT runtime. There
is no bailout mechanism, no deopt metadata, no on-stack replacement. This is the defining
constraint for a *dynamically typed* language, and it dictates the specialization strategy:

> Guard and branch to a compiled generic path. Never guard and bail out.

Every type-specialized region needs its generic counterpart compiled into the same
function, reachable by a conditional branch. This costs code size and forecloses the
aggressive speculation that a V8-style engine uses, but it is simple, correct, and has no
runtime metadata cost.

The absence of OSR also means: do not plan to enter compiled code in the middle of a
long-running loop. Compile at word granularity and rely on loop bodies being separately
JIT-able lambdas.

**(c) Target platforms.** x86-64, aarch64, s390x, riscv64. No 32-bit x86, no 32-bit ARM.
If Bund is to keep running everywhere Rust runs, the interpreter tier is not optional —
it is the portability story, and the JIT must be a `cargo` feature that compiles out
cleanly.

**(d) Relocation range on x86-64.** Calls use 32-bit relocations (±2GB). `cranelift-jit`
now offers memory providers including an `ArenaMemoryProvider` that reserves a contiguous
region up front; use it, and route runtime-helper calls through an indirection table so
helper addresses are never a relocation-range problem.

**(e) Maturity and churn.** `cranelift-jit` describes itself as "extremely experimental."
The API has moved across recent versions. Pin exact versions; budget for periodic
migration work; do not repeat the `">=0.*.*"` pattern here of all places.

**(f) Dynamic-language ergonomics are a known gap.** Wasmtime issue #9539 ("Cranelift:
introduce ArrayCall calling convention") is an open request from exactly this use case:
dynamically typed languages want callee-side argument-count checks and array-style argument
access, and the workarounds — passing `argc: usize, argv: *mut Value` — defeat tail calls
and force everything onto the stack. Bund's design should not depend on this landing.

**(g) Exceptions.** `try_call` / `try_call_indirect` exist but the unwinder story is
incomplete. Bund's `Result<&mut VM, Error>` error protocol should stay a return-value
convention in compiled code, not become native unwinding.

---

## 4. Proposed architecture

A workspace, with the JIT strictly optional:

```
bund-value      # BundValue representation, heap payload types, interning
bund-syntax     # pest grammar → AST (spans, no runtime objects)
bund-ir         # BundIR: linear typed IR, stack-effect annotations, serialisation
bund-interp     # Tier 0: threaded interpreter over BundIR  [always built]
bund-jit        # Tier 1: BundIR → CLIF → native            [feature = "jit"]
bund-stdlib     # the 156 words, as native fns + effect signatures
bund            # library façade (today's `bundcore` API)
bund-cli        # binary
```

### 4.1 Component 1 — value representation

This is the load-bearing decision and should be the first RFC.

**Recommendation: a 16-byte tagged value, `Copy`, no allocation for scalars.**

```rust
#[derive(Clone, Copy)]
pub struct BundValue { tag: u64, payload: u64 }
```

- `INTEGER` and `FLOAT` sit unboxed in `payload` at full width. This matters: Bund
  integers are `i64`, and NaN-boxing into a single `u64` would cap them at ~48–51 bits or
  force boxing. A 16-byte pair keeps both `i64` and `f64` native, and Cranelift passes it
  as two `i64`s with no memory traffic.
- Heap types (`STRING`, `LIST`, `MAP`, `LAMBDA`, `MATRIX`, `JSON`, …) put an `Rc<T>`
  pointer in `payload` and the type in `tag`. The VM is single-threaded (`VM` is not
  `Sync`), so `Rc` is correct and cheaper than `Arc`.
- `tag` has spare bits: use them for a "has side table entry" flag.

**What leaves the value:**

- `id: String` — the nanoid. Nothing in the hot path reads it. If object identity is
  genuinely required by some stdlib word, derive it from the heap pointer, or store it in
  a side table for the values that ask for it.
- `stamp: f64` — the wall-clock read. `Value::now()` already exists as an explicit way to
  get a timestamp; constructors should not do it implicitly.
- `tags: HashMap<String,String>` and `attr: Vec<Value>` — move to a side table keyed by
  heap handle. Scalars simply do not carry them; `tag` / `attribute` on a scalar either
  boxes it first or errors. This is a **semantic change** and needs to be an explicit
  decision in the RFC, not an implementation accident.
- `q: f64`, `curr: i32` — audit; both look vestigial.

**Alternative considered:** handles (`u32` index into typed arenas) instead of `Rc`.
Friendlier to JIT'd code because there is no refcount traffic, but it needs a reclamation
strategy Bund does not currently have. Start with `Rc` and refcount helpers; revisit if
profiling shows refcounting dominating.

**Serialisation:** `to_binary` / `from_binary` / `to_json` are public API and are used for
`compile_to_binary`. They move to the IR level (§4.3) and to explicit conversions on
heap types, not to a per-value bincode of a struct that no longer has an id or a timestamp.

### 4.2 Component 2 — symbols and the word table

Intern every word name at parse time to a `SymbolId(u32)`. Replace the five parallel
`HashMap<String, _>` tables (`inline_fun`, `command_fun`, `methods_fun`, `lambdas`,
`name_mapping`) plus the `TS`-level `inline_fun` with **one** `Vec<WordSlot>` indexed by
symbol:

```rust
enum WordEntry {
    Native(NativeFn),
    Interp(Rc<IrBody>),
    Jitted { code: *const u8, gen: u32 },
    Alias(SymbolId),
    Undefined,
}

struct WordSlot {
    entry: WordEntry,
    gen:   u32,          // bumped on every redefinition
    effect: StackEffect, // arity in/out, or Variadic
}
```

The entire resolution chain in `apply` — command check, `$` check, alias, lambda check,
inline lookup, suffix formatting, double alias resolution — collapses to one array index
plus a match. This is the single highest-value change in the project relative to its cost,
and it is a prerequisite for the JIT: compiled code needs a stable, cheap, patchable
call target, and `format!("{}_inline", name)` is not it.

The `gen` counter is what makes redefinition safe in the presence of inlining (§4.6).

### 4.3 Component 3 — BundIR

The missing middle. Requirements:

- **Linear and explicit.** One op per node; literals, word calls, stack switches, block
  entry/exit as distinct opcodes.
- **Control flow as structure, not as data.** Today `if`, `times`, `while`, `loop`, `map`
  take a `LAMBDA` value off the stack. In IR they should have block operands where the
  lambda is statically known, and fall back to a dynamic form where it is not. This is
  what makes loops compilable.
- **Stack effects attached.** Every op carries its arity in/out, or is marked `Variadic`.
- **Source spans.** For error messages, which today come out as string-formatted `bail!`
  chains with no position information.
- **Serialisable.** Replaces `compile_to_binary`. This becomes Bund's actual object
  format, versioned, instead of a bincode of a value tree.
- **`( ... )` as a scoped-block op**, fixing the parser side channel from §1.2.

Tier 0 is a threaded interpreter over this IR. It must be complete — all 156 words — and it
is the correctness oracle for Tier 1.

### 4.4 Component 4 — stack-effect analysis

This is the enabling analysis for the JIT and it is also a **language-design decision**,
not just an implementation task.

For a straight-line run of words with known effects, you can compute the abstract stack
depth at every point and keep operands in Cranelift SSA values instead of touching the
`VecDeque`. This is the largest JIT-specific win available in a concatenative language:
`2 2 + 3 *` becomes three CLIF instructions with no memory traffic at all.

Barriers that force a spill of the abstract stack to the real one:

- **Variadic words.** `*+`, `*-`, `*loop` and friends consume the whole stack. They are
  unanalysable by construction.
- **Stack switches.** `@name`, `to_stack`, `endcontext`, `if.stack` change which stack is
  current.
- **Dynamic dispatch.** `execute`, `apply`, anything that pulls a callable off the stack.
- **`autoadd` regions** (§4.6).
- **Any call to a word whose slot could be redefined mid-region** — handled by inlining
  with a generation guard, or by treating the call as a barrier.

Practical consequence: annotate all 156 words with effects, and accept that the `*`-family
is a permanent optimization barrier. If that proves too costly, restricting or deprecating
those words in JIT-able positions is a legitimate language-level answer — but it is a
decision to make deliberately.

Multi-stack handling: **only the current stack is slot-promoted.** The workbench and named
stacks stay as real runtime structures reached through helper calls. This keeps the
distinctive part of Bund's semantics intact while still getting the win on the common case.

### 4.5 Component 5 — the Cranelift backend

Compilation unit: **one word (or one lambda) per CLIF function.**

Signature: `fn(vm: *mut VmContext) -> i32` where the return code distinguishes normal
return from error, and the error itself lands in `VmContext`. Do not attempt native
unwinding (§3.2g).

The `VmContext` layout is part of the ABI and must be stable and directly addressable from
CLIF: current-stack base pointer, current-stack length, workbench pointer, word-slot table
base, error slot. JIT'd code loads these as fixed offsets from the `vm` pointer.

Lowering strategy per op class:

| IR op class | Lowering |
|---|---|
| Integer/float literal | `iconst` / `f64const` into an SSA value |
| Arithmetic on promoted slots | tag check → native `iadd`/`fadd` with overflow branch → else `call` generic helper |
| Comparison | same guard-and-branch shape |
| Stack push/pull (unpromoted) | inline load/store against the current-stack pointer, with a bounds check |
| Direct word call | `call_indirect` through the slot's code pointer |
| Loops with static bodies (`times`, `while`, `loop`) | CLIF blocks with block parameters; `cranelift-frontend` builds the SSA |
| `if` with static lambda | CLIF `brif` to inlined blocks |
| Dynamic dispatch (`execute`, dynamic `if`) | `call` to the existing runtime helper |
| Stack switch, workbench ops | `call` to helper; abstract stack spilled first |
| Everything not yet lowered | `call` to the Tier-0 helper for that word |

That last row is what makes the project tractable: the backend can ship with a small set of
words lowered natively and everything else calling into the interpreter's implementation,
then widen coverage word by word with the test suite as the check.

### 4.6 Component 6 — metaprogramming under a JIT

This is where projects of this shape usually fail, so it needs an explicit answer per
feature.

**`autoadd` (`:` / `;`).** A global flag consulted on every `apply`, that changes what
every subsequent token means. Handle it as a **compile-time mode**: an IR region is
compiled either in autoadd mode or not. If the flag can be toggled by a runtime-computed
value inside a JIT candidate, that word is not a JIT candidate — demote to Tier 0. In
practice `:` and `;` are literal tokens, so this is statically determinable almost always.

**`register` / `unregister`.** Bumps `WordSlot.gen` and writes a new entry. Callers that
went through `call_indirect` on the slot pointer pick up the change for free. Callers that
**inlined** the old body must be invalidated: keep a reverse dependency list
(`SymbolId → Vec<SymbolId>` of words that inlined it) and mark dependents for
recompilation. Given (§3.2a) there is no way to free the stale code, cap the number of
recompiles per word — after N, stop inlining that callee anywhere and pin it to indirect
calls.

**`alias` / `unalias`.** Resolved at slot level (`WordEntry::Alias`). Same generation
protocol.

**`$name`.** Bypasses lambda and alias resolution. In IR this is a distinct opcode
resolving directly to a native entry — trivially compilable and actually *easier* than the
general case.

**Runtime-constructed lambdas.** `{ ... } "name" register` builds a lambda from values on
the stack. These start at Tier 0 and tier up only on a call-count threshold, which is the
right policy anyway: code generated at runtime is usually cold.

**`execute` on arbitrary values.** Always a helper call. No attempt to specialise. This is
correct — it is genuinely dynamic and infrequent in hot loops.

### 4.7 Tiering policy

- Every word starts at Tier 0.
- Count invocations in the slot; at threshold (start with something like 1,000) enqueue for
  Tier 1 compilation.
- Compile synchronously at first — Cranelift is fast enough, and background compilation
  adds a whole concurrency story to a VM that is currently single-threaded. Revisit later.
- Global caps: total JIT'd functions, total recompiles per word, total code bytes. On
  exceeding any of them, stop tiering up. This is the mitigation for §3.2a and it needs to
  exist from day one, not be retrofitted.
- A `--no-jit` flag and a `jit` cargo feature; Tier 0 must be able to run everything.

---

## 5. Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Code memory grows without bound under REPL/metaprogramming | **High** | Indirect calls only; hard caps; demote pathological redefiners; measure in a long REPL session early |
| Tier 0 / Tier 1 semantic divergence | **High** | Differential testing: run the existing ~30 test files under both tiers, compare final stack + workbench state. Build this harness in Phase 2, before any codegen |
| Stack-effect signatures turn out to be under-specified or genuinely variadic in common code | **High** | Survey real Bund programs before committing; be prepared to restrict the `*`-family in JIT-able positions |
| Value representation change breaks published API | Medium | It will. `rust_dynamic` 0.50 → 1.0 with a migration note; the `id`/`stamp`/`tags`-on-scalars removal is a real semantic change |
| Cranelift API churn | Medium | Exact version pins; isolate all Cranelift contact inside `bund-jit`; keep the `BundIR → CLIF` boundary narrow |
| Project A delivers most of the win and Project B never pays for itself | Medium | This is precisely what the Phase 4 decision gate is for. It is an acceptable outcome, not a failure |
| Loss of the multi-stack model's clarity in pursuit of speed | Medium | Only the current stack is promoted; workbench and named stacks stay concrete |
| Scope: ~13.3k LOC across five crates, most of it touched | Medium | Phase gating; Tier 0 completeness before any Tier 1 work |

**The single most likely failure mode** is starting with the Cranelift backend because it
is the interesting part, discovering that JIT'd code manipulating 180-byte `Value` structs
is barely faster than the interpreter, and concluding the approach doesn't work. It does
work — but only on top of a value representation that a code generator can do something
with.

---

## 6. Recommended phasing

**Phase 0 — Baseline.** Criterion benchmarks on the current interpreter: arithmetic loop,
lambda call, stack shuffle, string ops, a realistic program. Feature-gate
`time_graph::instrument` out of the hot path first. Confirm `size_of::<Value>()`. Nothing
after this phase is assessable without these numbers.

**Phase 1 — RFC: `BundValue`.** 16-byte tagged value, `Rc` heap payloads, side table for
tags/attrs, no nanoid, no implicit clock read. No JIT, no IR. Success criterion: the whole
existing test suite passes and the Phase 0 benchmarks improve by a measured multiple.

**Phase 2 — RFC: symbols and slot table.** Interning, single `Vec<WordSlot>`, removal of
the `_inline` suffix scheme and the double alias resolution. Success criterion: word
dispatch is allocation-free.

**Phase 3 — RFC: BundIR + Tier 0.** AST from pest, lowering to IR, threaded interpreter,
serialisable object format replacing `compile_to_binary`, scoped blocks replacing the
parser side channel. Build the differential-testing harness here. Success criterion:
full parity, and the IR interpreter beats the current `Value`-walking interpreter.

**Phase 4 — Stack effects.** Annotate all 156 words; implement the abstract-stack analysis;
validate it against the test suite by asserting predicted depth matches actual depth at
runtime under a debug flag.

**Phase 5 — Cranelift spike (decision gate).** Narrow vertical slice behind
`feature = "jit"`: integer and float arithmetic, comparisons, `times` loops, direct word
calls, everything else falling through to Tier-0 helpers. Measure against Phase 3.
**Gate: if the slice is not ≥3× on arithmetic-heavy benchmarks, stop and keep Tier 0.**
The fallback is not failure — a well-built threaded interpreter on a good value
representation is a legitimate final destination, and it ships on every platform.

**Phase 6 — Widen.** Type-guard fast paths, small-lambda inlining with generation guards,
redefinition invalidation, code-memory caps, more words lowered natively.

Phases 1–4 are worth doing on their own merits and carry no Cranelift risk. Phase 5 is the
first point at which the project is committed to a code generator, and by then there will
be real numbers to commit on.

---

## 7. Open questions for the RFC

1. **Integer width.** Is full `i64` a hard requirement? If 51-bit integers are acceptable,
   NaN-boxing into 8 bytes becomes viable and halves stack traffic. This changes the
   representation decision in §4.1.
2. **Are `id`, `stamp`, `q` and `curr` load-bearing anywhere?** A grep across real Bund
   programs, not just the crates, should settle whether removing them from scalars is a
   breaking change in practice or only on paper.
3. **How variadic is real Bund code?** The `*`-family's prevalence determines how much of
   a typical program is JIT-able at all.
4. **Is `autoadd` ever toggled dynamically**, or is it always the literal `:` / `;` tokens?
5. **Does anything depend on the bincode format** of `compile_to_binary` externally? If so,
   the IR object format needs a compatibility shim.
6. **Threading.** `VM` is `Clone` and `bundcore` has an `ephemeral()` path that spins up
   fresh VMs. Is concurrent execution on the roadmap? It changes `Rc` vs `Arc` and it
   changes whether background JIT compilation is worth designing for. `JITModule` is
   `Send` but not `Sync`.
7. **Which is the real target profile** — REPL/interactive, or long-running batch? The
   code-memory constraint in §3.2a bites hard on the former and barely at all on the latter,
   and that changes how conservative the tiering policy needs to be.

---

## Appendix A — source references

| Claim | File |
|---|---|
| Parser returns `Vec<Value>` | `bund_language_parser/src/lib.rs` |
| `compile()` folds to `LIST`, `compile_to_binary` | `bund_language_parser/src/compile.rs` |
| `( ... )` mutates caller state | `bund_language_parser/src/vm/ctx.rs` |
| Grammar, 13 value rules | `bund_language_parser/bund.pest` |
| Central dispatch, `autoadd`, `$` prefix | `rust_multistackvm/src/multistackvm_apply.rs` |
| `_inline` suffix formatting ×3 | `rust_multistackvm/src/multistackvm_inline.rs` |
| Second alias resolution in `i()` | `rust_multistackvm/src/multistackvm_inline.rs` |
| Lambda deep clone on call | `rust_multistackvm/src/multistackvm_lambdas.rs` |
| Lambda body re-walked per call | `rust_multistackvm/src/multistackvm_lambda_eval.rs` |
| Dynamic dispatch by value type | `rust_multistackvm/src/stdlib/execute.rs` |
| `times` clones body per iteration | `rust_multistackvm/src/stdlib/logic/times_fun.rs` |
| `unregister` registered twice | `rust_multistackvm/src/stdlib/lambdas/registry.rs` |
| `if.false.in_workbench` uses `FromStack` | `rust_multistackvm/src/stdlib/logic/if_fun.rs` |
| VM struct: 5 name-keyed tables | `rust_multistackvm/src/multistackvm.rs` |
| `TS`: named stacks + workbench | `rust_multistack/src/ts.rs` |
| `set_tag` on every push | `rust_multistack/src/ts_push.rs` |
| Redundant clone in `push_to_workbench` | `rust_multistack/src/ts_workbench.rs` |
| `Value` struct layout | `rust_dynamic/src/value.rs` |
| `nanoid!()` + `timestamp_ms()` per constructor | `rust_dynamic/src/create.rs` |
| `set_tag` allocates two Strings | `rust_dynamic/src/tags.rs` |
| `cast_string` allocates | `rust_dynamic/src/cast.rs` |
| `Val` enum, 20 variants | `rust_dynamic/src/types.rs` |

## Appendix B — external references

- Cranelift crate versions: crates.io API, checked 2026-08-23 (`cranelift-jit`,
  `cranelift-codegen`, `cranelift-module`, `cranelift-frontend` all 0.135.0, published
  2026-08-20).
- `JITModule` API surface: docs.rs `cranelift-jit` 0.134.3.
- Backend/ISA support and codegen-quality benchmarks: `bytecodealliance/wasmtime`,
  `cranelift/README.md`; cranelift.dev.
- `CallConv::Tail` semantics and per-ISA exception payload registers: `cranelift_codegen::isa::CallConv` docs.
- s390x tail-call gap: wasmtime issue #6530.
- Dynamic-language calling-convention gap: wasmtime issue #9539.
- `try_call` / exception support status: wasmtime PR #10510.
- x86-64 32-bit relocation range: wasmtime issue #4000.
