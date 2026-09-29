# RFC-0006: Ahead-of-time output — `bund2 build`

- Status: **Draft** (2026-09-29). Not proposed: §B7 is blocked on Q38, which is
  an OPEN decision this RFC must not adopt on the owner's behalf.
- Depends on: RFC-0003 (BundIR and Tier 0), RFC-0005 (the Cranelift tier)
- Decisions consumed: D10, D11, D16, D40, D44, and decisions.md's
  "What this forecloses" clause on tree-shaking
- Blocked on: **Q38** — what `use` does in a built artefact
- Reference SHA: `reference/Bund` at `21b40b0`, `rust_dynamic` at `ceb27c9`,
  per `reference/PINNED.txt`
- Supersedes: nothing. `docs/research/02-native-binaries.md` is the reasoning
  trail and stays as written; where this RFC departs from it, §B8 says so and
  `docs/research/ERRATA.md` records it.

## Summary

`bund2 build` produces a runnable artefact from a Bund program. Two output
modes, both in scope here because D10 decides them together and neither is
meaningful alone:

- **`--emit=bundle`** — the runtime with the program's BundIR embedded. No
  Cranelift, no code generator, no linker, no `cc`. Runs on every target Rust
  runs on, including those Cranelift does not support.
- **`--emit=native`** — Cranelift's `ObjectModule` emits a `.o`, which is
  linked against the runtime. Faster to start and free of the code generator
  in the shipped binary; requires a linker.

`--emit=bundle` is specified in full and built first. `--emit=native` is
specified to the depth D10 and RFC-0005's criterion 4 require and is gated on
a measurement §B8 names, because the research that proposed it also named the
number that decides whether it is worth the phase, and that number has never
been taken.

## Motivation

Bund2 has no output. Everything it can do, it does by being handed source and
evaluating it, which means a Bund program cannot be distributed as a program.

Three things in the registers already depend on this RFC existing, and two of
them are *constraints it inherits* rather than goals it chooses:

- **D10** resolves that `--emit=native` may require `cc` **and** that
  `--emit=bundle` must stay toolchain-free, and calls the clause after the
  semicolon "load-bearing, not decoration". Its status names both halves:
  "nothing below `bund2 build` may require `cc`."
- **D40** feature-gates `string.grok` off by default precisely because a C
  dependency below the toolchain-free artefact would invert D10's escape
  hatch — producing the artefact that needs no toolchain would need one.
- **RFC-0005's criterion 4** cannot be run at all against the JIT.
  `cranelift-jit` consumes relocations when it finalises a definition and
  exposes no accessor; `cranelift-object` keeps them behind `relocs()`. The
  criterion was explicitly assigned here rather than quietly discharged by
  reading CLIF text, which is the inspection it was rewritten to avoid.

## Current behaviour

**The reference has no build output, and this is close to a proof rather than
a judgement** — D11 established it and this RFC re-checked it against the
pinned SHA rather than citing the register from memory.

`Value::compile` is the object-format producer, and it refuses anything that
is not a LAMBDA before delegating to `to_binary`
(`reference/rust_dynamic/src/bincode.rs:38-50`). Its only caller in the
submodules is a test of itself
(`reference/rust_dynamic/tests/bincode-test.rs:103`). No Bund word reaches it.
So **no Bund program has ever produced a file in this format**, there are no
artefacts in the wild, and D11's resolution — "no external dependents; version
the IR format freshly" — is the licence this RFC builds on.

**What Bund2 has instead** is the pair the CLI already uses, and a bundle is a
different front end onto exactly these:

- `bund2_syntax::compile` (and `lower_with_spans`, which adds spans) turns
  source into the program IR, which **is a `Vec<BundValue>`** — there is no
  separate instruction type (`crates/bund2-syntax/src/lib.rs`, `lower`).
- `Vm::eval_indexed` evaluates that vector, reporting the index of a failure
  so a diagnostic can carry a source location (`crates/bund2-interp`).
- `bund2_value::wire` already serialises a `BundValue` and reads it back,
  faithful to the reference's format (`to_binary`, `from_binary`).

## Design

### §B1 — What separates the two products

`docs/research/02-native-binaries.md:33-64` names three products and this RFC
builds the first two. The separation that matters is not speed:

> An AOT binary does not need to contain Cranelift.

A bundle embeds IR and interprets it, optionally tiering with the JIT if the
feature is on. A native image has its code generated at build time and ships
**no code generator at all**. For a language distributed as CLI tools that is
the larger practical difference, and §B8 makes it the gate.

### §B2 — The bundle container

**The container is versioned freshly**, on D11's resolution. It is not the
reference's object format, which nothing produces, and it is not
`wire::to_binary` applied to the program as a whole — that function takes one
value, not a vector.

A bundle carries, in this order: a magic, a **format version**, the
`reference/PINNED.txt` SHAs the producing Bund2 was built against, and the
program's values.

**The version is not decoration and neither are the SHAs.** A bundle is read
by a runtime that may be newer than the one that wrote it. The version says
whether the container can be read; the SHAs say which oracle the program's
meaning was fixed against, so a bundle produced before a conformance change
can be identified rather than silently reinterpreted.

**Values are encoded with the existing wire codec, one value per element.**
Reusing it rather than inventing a second encoding keeps one implementation of
a format that already has fixtures, a depth cap and a defect history
(`crates/bund2-value/src/wire.rs`). Two of its properties are inherited
deliberately and must be stated:

- **`MAX_WIRE_DEPTH` is 256** and `to_binary` refuses a value nesting deeper,
  measured by its own heap walk so the check cannot itself overflow (F118). A
  program whose lambdas nest past 256 cannot be bundled, and **that refusal is
  the correct behaviour**: the same value could not be read back.
- **The JSON quirk does not reach a bundle.** `to_binary` wraps a top-level
  JSON value as a string with a fresh id and stamp, and a JSON value nested in
  a list is written in a form that cannot be read back — in the reference and
  in Bund2 alike. It cannot arise here, because **the parser has no JSON
  term**: `Term` is Int, Float, Str, Name, Command, Ptr, Stack, Lambda, List
  and Ctx (`crates/bund2-syntax/src/lib.rs`, `Term`). JSON enters a program at
  run time, through words, and never through lowering.

### §B3 — The stub, and the evaluation thread it must reproduce

A bundle is the runtime plus embedded IR plus a `main` that reads the IR and
evaluates it. The `main` is the part that looks trivial and is not.

**It cannot simply call `eval`.** `bund2-cli`'s `main` runs everything on a
thread it spawns, sized `TIER0_PART + TIER1_SHARE + STACK_RESERVE`, and then
declares the region with `set_stack_region_with_share`
(`crates/bund2-cli/src/main.rs`). The reason is recorded at the call and is
not about capacity:

> A share is declared, never inferred. Declaring through `set_stack_region`
> would leave the share at zero, putting the Tier 1 floor above the thread's
> top so that every compiled body declines.

That is the failure this session's `compiled_entries` counter was added to
expose: a tier that compiles everything, enters nothing, and reports success
on every figure a reader would think to check
(`crates/bund2-runtime/src/tier.rs`,
`a_tier_with_no_share_compiles_bodies_and_enters_none`). A stub that spawns no
thread, or spawns one without declaring the share, produces a binary whose JIT
is dead and whose reports say it is working.

**So the stub reproduces the CLI's thread construction, and the same test
holds it to that.** The main thread's stack is the operating system's to size,
so Bund2 cannot know where it ends; a thread it spawns, it can (§S8, F85).

### §B4 — What the image must retain

**No tree-shaking by word reachability.** D16 makes a call target a name that
may be assembled at run time, so no registered word can be proven dead. The
decision register states the consequence for this RFC in terms: "An AOT image
retains the word table and the name resolver."

This is not a size optimisation deferred. It is foreclosed, and a build that
dropped an unreferenced word would change what a program means, not how fast
it runs.

### §B5 — `--emit=native`

The same lowering, pointed at `ObjectModule` instead of `JITModule`. RFC-0005
§S6's lowering is shared; the two differ in which `Module` receives the CLIF.

**This is where criterion 4 is discharged.** Compile a body that calls another
word and assert the module's relocation records contain **no entry targeting a
function**. Two relocations are permitted and both come from §S8's call
boundary: a native's `Tail` thunk calling its Rust adapter, and the entry
trampoline calling the body it enters. Neither names a compiled body's
`FuncId` from inside another body, which is the property that would leave a
direct call pointing at orphaned code after a redefinition.

**Linking is permitted to require `cc`** — D10, for this mode only. Nothing
below `bund2 build` may.

### §B6 — Cross-compilation and the C dependency

D40 is inherited whole. `string.grok` is feature-gated off by default because
a C dependency beneath the artefact would invert D10's escape hatch, and a
cross-compiled `cranelift-object` build inherits the same problem. A target
Cranelift does not support gets `--emit=bundle`, which is the entire reason
that mode is mandatory rather than convenient.

### §B7 — `use` in a built artefact — **BLOCKED on Q38**

This section is deliberately unwritten.

`use` fetches a `file://` or `http://` URL and evaluates the text (D54). A
built program is harder in two ways Q38 states: the path may be a string built
at run time (D16), so `bund2 build` cannot always know what a program loads;
and `--emit=bundle` must stay self-contained (D10).

Q38 records three options and marks one **"proposed, not adopted"**. Adopting
it here would be this RFC making a language decision on the owner's behalf,
which CLAUDE.md forbids in terms. **This RFC is blocked on Q38** and says so
rather than defaulting.

### §B8 — The gate

**RFC-0005 was gated on a measurement before implementation began, and this
RFC is gated the same way.** Its criterion 1 required `value/push_pull` under
20 ns and reported 9.8 ns before any lowering was written.

The research names the number for `--emit=native` and the reason it is the
right one:

> It should be measured early — `cargo bloat` on a Product A binary with and
> without the `jit` feature — because the number decides whether B is worth
> the phase.

**That measurement has never been taken**, and it cannot be taken until
`--emit=bundle` exists, because it is a measurement *of a Product A binary*.
This is the ordering argument for building the bundle first, and it is
stronger than the convenience one: `--emit=native`'s case rests on a size
saving nobody has weighed.

## Preservation analysis

**Nothing in this RFC changes what a program means**, and that is checkable
rather than asserted: a bundle of a corpus program must produce the same final
state as evaluating its source, which is what `conform` already compares.

| behaviour | disposition |
|---|---|
| The reference's object format | **Not preserved, and nothing depends on it.** D11: `Value::compile` has no caller a Bund program can reach; no artefact exists to break. Versioned freshly. |
| Program meaning under a bundle | **Preserved exactly.** Same IR, same evaluator. Criterion 2 below makes it a measured claim. |
| `MAX_WIRE_DEPTH` | **Preserved.** A program too deep to read back is refused at build time rather than written and lost. |
| Word table and name resolver | **Preserved in full.** No tree-shaking; D16. |
| `use` | **Undecided — Q38.** |

## Alternatives considered

- **A second encoding for the bundle, tuned for a flat program vector.**
  Rejected: it would be a second implementation of a format that already has
  fixtures, a depth cap and a defect history, and the saving is unmeasured.
- **`--emit=native` first.** The research itself recommends object-output-first
  for the *lowering spike*, and that ordering is now moot — the lowering exists
  and was built JIT-first under RFC-0005. What remains of the argument is the
  size saving, which §B8 shows cannot be weighed until a bundle exists.
- **Tree-shaking behind a flag.** Rejected, and not by this RFC: D16's
  consequence is recorded as foreclosed.
- **A stub that evaluates on the main thread.** Rejected on §B3: it cannot know
  where its stack ends, and with the share undeclared every compiled body
  declines while every figure reports success.

## Acceptance criteria

Each names the tool that decides it and a threshold or a boolean outcome.

1. **The gate is answered before `--emit=native` is built.** `cargo bloat` on a
   bundle binary with and without `jit`, reported as a figure. No threshold is
   set here: the number is the input to a decision, and setting a bar now would
   be guessing at what it should say.
2. **A bundled corpus program produces the same final state as its source.**
   `cargo xtask conform` over the goldens, run against bundles rather than
   against source, reports the same N/M and the same CEILING. Not "similar":
   the same, or the bundle changes meaning.
3. **A bundle contains no code generator.** `cargo tree` on the bundle stub
   lists neither `cranelift-codegen` nor `cranelift-jit` in a default build.
4. **A bundle builds with no C toolchain.** D10's load-bearing half, checked by
   building it in an environment without `cc` rather than by reading the
   dependency list.
5. **A program too deep to read back is refused at build time**, with the depth
   and the cap in the message — not written and discovered unreadable later.
6. **The stub declares its Tier 1 share.** A bundle built with `jit` and run on
   a body past the threshold reports `compiled_entries() > 0`. This is
   criterion 4's shape from RFC-0005 applied here: the figure that a broken
   stub would leave at zero while every other figure reported success.
7. **Criterion 4 of RFC-0005 is discharged.** A body that calls another word,
   compiled through `ObjectModule`, has no relocation targeting a function
   beyond the two §S8 permits.
8. **Conformance moves by exactly zero.** As RFC-0005's criterion 2: this RFC
   changes what Bund2 can emit, not what a program means.

## Open questions

- **Q38 — what `use` does in a built artefact.** OPEN; blocks §B7 and
  criterion 2, since a corpus program that calls `use` cannot be bundled until
  it is answered. Cross-referenced to D10, D16, D54.
- **The bundle's own versioning policy.** §B2 records the SHAs; what a runtime
  should *do* on a version it does not recognise — refuse, or read and warn —
  is not decided here and is smaller than a register entry until a second
  version exists.
