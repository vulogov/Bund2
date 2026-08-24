# Bund2 — Addendum: Producing System-Executable Binaries

**Subject:** Can `bund2` + Cranelift emit a native executable?
**Status:** Research / pre-RFC addendum
**Date:** 2026-08-23
**Governing constraint:** Bund syntax as defined and Bund logic must be 100% preserved.

---

## 0. Short answer

**Yes.** Cranelift ships `cranelift-object` (0.135.0, same release train as
`cranelift-jit`), which emits native ELF / Mach-O / COFF object files through the `object`
crate. The pipeline is:

```
.bund source → AST → BundIR → CLIF → ObjectModule → program.o
                                                      ↓
                        + libbund2_runtime.a  →  [linker]  →  ./program
```

But the governing constraint changes what that binary *is*. Because Bund logic must be
preserved exactly, the executable is **not** a compiled program in the C sense. It is a
native binary containing AOT-compiled machine code for the statically determinable parts,
plus the complete Bund2 runtime — word slot table, Tier-0 interpreter, stdlib — because
`register`, `alias` and `execute` remain live at runtime and something has to service them.

That is not a limitation of Cranelift. It is Bund's own semantics, and preserving them
100% is the stated requirement.

---

## 1. Three distinct products

These get conflated. They have different sizes, different startup costs, and different
build requirements.

| | Product | Build needs | Contains Cranelift? | Startup | Peak speed |
|---|---|---|---|---|---|
| **A** | Runtime + embedded IR | nothing extra | optional (for JIT tiering) | deserialise IR | JIT-tier, after warmup |
| **B** | AOT object + runtime, linked | `cranelift-object` + a linker | **no** | immediate | AOT-tier, no warmup |
| **C** | A + B together | both | yes | immediate | AOT then JIT re-tier |

**Product A** is the trivially achievable one and should exist first. Serialise the BundIR
object format (main study §4.3), `include_bytes!` it into a stub `main`, link against the
runtime. One file, no external dependencies, runs anywhere Rust runs — including the
32-bit and ARM32 targets Cranelift does not support. This is the portable distribution
story and it does not require a code generator at all.

**Product B** is the interesting one, and its strongest argument is not speed:

> An AOT binary does not need to contain Cranelift.

`cranelift-codegen` with its ISLE-generated instruction selection tables is the single
largest code contributor to any binary that embeds the JIT. Product B moves the compiler to
build time and ships only the runtime. For a language whose programs are distributed as
CLI tools, that is a bigger practical win than the arithmetic speedup. It should be
measured early — `cargo bloat` on a Product A binary with and without the `jit` feature —
because the number decides whether B is worth the phase.

**Product C** is for long-running services where you want fast startup *and* profile-driven
re-tiering. It is the last thing to build, not the first.

---

## 2. What has to be in the binary, given 100% preservation

Working from what the source actually requires:

**Mandatory in every product:**

- The word slot table, **mutable at runtime**. `stdlib_lambda_register` pulls the name off
  the stack and `cast_string()`s it — the name is a runtime value, not a syntactic literal.
  `{ … } :name register` looks static but the language does not guarantee it is.
- The Tier-0 interpreter. Anything not statically resolvable falls back to it.
- The full stdlib (156 words) as native code. Already true — these are Rust functions.
- The `execute` dispatcher, which dispatches on a value's type pulled off the stack,
  including MAP-key-driven dispatch where the key is computed.
- Lambda values as first-class runtime objects (see §3).

**Conditionally required:**

- **The parser (pest + `bund.pest`).** Only needed if a word can turn a string into code at
  runtime. None of the 156 words in `rust_multistackvm` does — there is no `eval`, `parse`,
  `load`, `source` or `import` word registered there. But `bundcore` exposes
  `Bund::eval()` and `run_bootstrap()` as Rust APIs, and `add_stdlib` is exactly the hook
  an external package would use to expose one as a word. **This is an open question that
  must be answered before designing the build:** if such a word exists anywhere in the full
  Bund distribution, then arbitrary strings become code at runtime, the parser and the
  IR-lowering pipeline both ship in every binary, and no closed-world analysis (§4) is ever
  sound unless it can prove that word unreachable.

**Not required in Product B:** Cranelift.

---

## 3. Lambdas — where AOT can and cannot reach

Per your note, `{ … }` serves double duty. That distinction maps directly onto AOT
coverage. Three cases:

**Case 1 — literal lambda, literal name.** `{ … } :name register` where both operands are
syntactic literals and the `register` is at top level or in a statically reachable path.
The compiler sees a word definition. **AOT-compiled to its own function**, slot
pre-populated at startup. This is the common case and probably covers most of a typical
program.

**Case 2 — anonymous literal lambda at a control-flow site.** `10 { … } times`,
`cond { … } if`, `{ … } while`, `[ … ] { … } map`. The lambda operand is a literal at the
call site, so the body is statically known. **AOT-compiled**, and better than case 1 — it
can be lowered as CLIF blocks inlined into the enclosing function rather than a call,
which is exactly the case where the stack-effect analysis (main study §4.4) pays off. Note
this also fixes the per-iteration deep clone in `stdlib_logic_times`.

**Case 3 — computed lambda, computed name, or a lambda flowing through the stack.**
Metaprogramming: building a lambda from parts, selecting one from a MAP, registering under
a name assembled at runtime, `execute` on whatever is on top. **Not AOT-compilable.** The
lambda is constructed at runtime as an IR body and runs at Tier 0 — or, in Product C, tiers
up through the JIT if it gets hot.

Case 3 is why the interpreter cannot be omitted, and it is also why the *lambda* must
remain a first-class runtime value carrying its IR body, not merely a compile-time
construct. An AOT'd case-1 word and a runtime-constructed case-3 lambda have to be
interchangeable everywhere a `LAMBDA` value is accepted. That means `WordEntry::Aot` and
`WordEntry::Interp` sit side by side in the same slot table, and a `LAMBDA` value on the
stack may point at either.

---

## 4. The sharp consequence: AOT does not get you direct calls, by default

The main study §3.2a explained that the JIT must call through indirection slots because
`cranelift-jit` cannot redefine a function. In AOT that specific limitation vanishes —
`ObjectModule` emits real relocations and direct `call` instructions are cheap and normal.

It does not help. **Bund's own semantics force the indirection anyway**, because `register`
can rebind any word at any point during execution, including a word that AOT'd code is
about to call. 100% preservation means the call target has to be read from a mutable slot
at call time. So the default AOT output is a sequence of indirect calls — roughly what the
JIT would produce.

**The escape hatch is closed-world analysis, and it can be automatic.**

If the reachable IR of a program contains no occurrence of `register`, `unregister`,
`alias`, `unalias`, or any word transitively reaching them — and no runtime-eval word, per
§2 — then the word table is provably immutable after startup. Under that proof the compiler
may:

- emit direct `call` relocations instead of slot loads;
- inline small word bodies across word boundaries;
- constant-fold `execute` on statically known callables;
- drop unreached stdlib words from the binary entirely (dead-code elimination against a
  known-closed word set, which is otherwise impossible because any word might be named by
  a computed string).

That last one compounds with the "no Cranelift in the binary" argument: a closed-world
Product B binary can be genuinely small.

Two important properties of this design: the analysis is **conservative** (any doubt →
open world → indirect calls, semantics preserved), and it is **automatic** (no flag, no
opt-in, no way for a user to accidentally break their program by requesting it). A
`--closed-world` flag that *asserts* the property rather than proving it would be a
deviation from the governing constraint and should not exist; a `bund2 explain --world`
diagnostic that reports *why* a program was compiled open-world is the right affordance
instead.

---

## 5. AOT versus JIT for Bund — the usual ordering inverts

Normally a JIT beats AOT on a dynamic language because it has profile feedback. For Bund
the trade is more balanced:

| | AOT (`cranelift-object`) | JIT (`cranelift-jit`) |
|---|---|---|
| Call overhead | direct calls + cross-word inlining **under closed world** | always indirect through slots |
| Type specialization | must emit both arms of every guard, no feedback | can specialize on observed types |
| Startup | zero | tier-up threshold, interpret until warm |
| Code memory | fixed, in the binary | grows monotonically, never reclaimed |
| Binary size | no Cranelift | Cranelift included |
| Dead code | eliminable under closed world | never |

The clean answer to the type-feedback gap is **profile-guided AOT**: `bund2 run --profile
prog.prof` records observed type tags at guard sites; `bund2 build --profile prog.prof`
orders the guard arms accordingly. This is a well-trodden design and it closes most of the
gap without putting a compiler in the shipped binary.

Given the code-memory row — which is the JIT's worst structural problem for a REPL-driven
language — **AOT is arguably the better long-term target for Bund2, and the JIT is best
understood as the REPL's execution mode rather than production's.** That is a reasonable
place to end up: `bund2 repl` uses Tier 0 + JIT, `bund2 build` uses AOT, and both share one
IR and one runtime.

---

## 6. The linker problem

`cranelift-object` produces a `.o`. It does not produce an executable. Something must link
`program.o` + `libbund2_runtime.a` + libc.

Options, in order of pragmatism:

1. **Shell out to `cc`.** What `rustc` itself does. Reliable, cross-platform, zero new
   dependencies. Costs: `bund2 build` requires a system toolchain, which sits awkwardly
   against the project's local-first/offline posture. Document it as a build-time-only
   requirement — running a Bund2 binary needs nothing.
2. **Bundle `rust-lld`.** Already present in any Rust toolchain. Removes the `cc`
   dependency on most targets.
3. **`wild-linker`** (0.10.0, updated 2026-08) — a fast pure-Rust linker. Matches the
   pure-Rust preference, but Linux-only today. Worth watching, not worth depending on yet.

There is no realistic path that avoids a linker. Pre-linking a template executable and
patching code into it is writing a linker with extra steps.

**Static linking:** for a genuinely self-contained artifact, build the runtime against
musl and link statically. Standard practice, no Bund2-specific work.

---

## 7. Cross-compilation

Cranelift can target any of its four ISAs regardless of host — `isa::lookup` takes a target
triple, so `bund2 build --target aarch64-unknown-linux-gnu` emitting the object file is the
easy half. The hard half is unchanged from any Rust project: you need `libbund2_runtime.a`
built for that target and a linker that can link it. So cross-compilation is possible and
the Cranelift layer is not the obstacle; the toolchain is.

Note the target set is still x86-64 / aarch64 / s390x / riscv64. For anything else,
Product A (runtime + embedded IR, pure interpreter) is the answer, and it should be a
first-class output of `bund2 build`, not an afterthought — `--emit=bundle` alongside
`--emit=native`.

---

## 8. Unwinding and diagnostics

Set `unwind_info` in the ISA flags so AOT'd frames appear in backtraces. Bund-level errors
stay a return-value protocol (previous addendum §1.3b), so unwinding is only for runtime
panics — but a native binary with opaque frames is miserable to debug, and the flag is
free. Source spans in BundIR (main study §4.3) should survive into a side table so a
runtime error can name a line rather than a `bail!` string.

---

## 9. Where this lands in the plan

AOT is not a separate project from the JIT. Both consume BundIR and share the entire
lowering layer; `cranelift-object` and `cranelift-jit` differ only in which `Module`
implementation receives the CLIF. Practically:

- **Phase 3 (BundIR + Tier 0):** add `--emit=bundle` — Product A. No Cranelift. This
  should be the first thing that produces a runnable artifact.
- **Phase 5 (Cranelift spike):** build the `BundIR → CLIF` lowering against
  **`ObjectModule` first, not `JITModule`.** Object output is easier to test — you can
  disassemble a `.o`, diff it, run it under a debugger, and check it into a test corpus.
  A JIT-first spike gives you a pointer you can only call.
- **Phase 5b:** point the same lowering at `JITModule` for the REPL. The delta is small
  once the lowering exists.
- **Phase 6:** closed-world analysis, dead-word elimination, profile-guided guard ordering.

Reordering AOT ahead of JIT is a change from the main study's phasing and I think it is the
right one: it de-risks the lowering work, it produces a demonstrable artifact earlier, and
it targets the deployment mode where Cranelift's weakest property — never reclaiming code
memory — does not apply at all.

---

## 10. Open questions

1. **Does any word in the full Bund distribution turn a string into code at runtime?**
   None of the 156 in `rust_multistackvm` does, but `bundcore::eval`/`run_bootstrap` make
   one trivial to add via `add_stdlib`. This single fact determines whether closed-world
   analysis is ever sound and whether the parser ships in every binary. Answer it first.
2. **How common is case 3 (computed lambdas / computed word names) in real Bund code?**
   Determines whether closed-world is the normal path or the rare one.
3. **What is the measured size of a Product A binary with and without the `jit` feature?**
   This is the number that justifies or kills the AOT phase; it should be measured on a
   throwaway prototype well before Phase 5.
4. **Is `bund2 build` allowed to require a C toolchain**, or must the compiler be
   self-contained? If the latter, bundling `rust-lld` becomes a Phase 5 requirement rather
   than a nicety.
5. **Should `--emit=bundle` and `--emit=native` produce byte-identical behaviour?** They
   should, and that is another arm of the differential-testing harness from the main
   study — the same test corpus run three ways: Tier 0, AOT, JIT.
