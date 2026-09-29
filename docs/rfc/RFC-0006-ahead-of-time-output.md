# RFC-0006: Ahead-of-time output — `bund2 build`

- Status: **Draft**, revised 2026-09-29 after the first adversarial review
  (`docs/rfc/reviews/RFC-0006-review-2026-09-29.md`). The review raised three
  blockers; all three were reproduced against the code before this revision,
  and one is worse than it reported. §B2 now carries a **deviation awaiting
  sign-off**.
- Depends on: RFC-0003 (the program stream and Tier 0), RFC-0005 (the
  Cranelift tier)
- Decisions consumed: D10, D11, D16, D20, D40, D44, D54, D76, and
  decisions.md's "What this forecloses" clause on tree-shaking
- Touched but not consumed: D1, D2, D36 — see the preservation table
- Reference SHA: `reference/Bund` at `21b40b0`, `rust_dynamic` at `ceb27c9`,
  `bund_language_parser` at `8037772`, per `reference/PINNED.txt`
- Supersedes: `docs/research/02-native-binaries.md` §9's phasing, recorded in
  `docs/research/ERRATA.md`

## Terminology

**This RFC says "the program stream", never "BundIR".** RFC-0003 fixes BundIR
as "a **cache over** a body, never the body itself" (RFC-0003:422), and
RFC-0000 gives `bund2-ir` as the only crate defining an instruction set. What a
bundle carries is neither: it is the program, and a program in Bund2 is a
`Vec<BundValue>` from `bund2_syntax::compile`. D10's phrase "embedded IR"
predates RFC-0003's narrowing and is read here as "the program", which is what
it meant when written.

## Summary

`bund2 build` produces a runnable artefact. Two modes, in scope together
because D10 decides them together:

- **`--emit=bundle`** — a prebuilt runtime with the program's **source text**
  appended. **No compiler is invoked at all**: not Cranelift, not `rustc`, not
  a linker, not `cc`. Runs on every target Rust runs on.
- **`--emit=native`** — Cranelift's `ObjectModule` emits a `.o`, linked against
  the runtime. **Not buildable on the lowering as it stands**; §B5 says what
  must change and criterion 7 is deferred behind it.

`--emit=bundle` is specified in full and built first. `--emit=native` is
specified to the depth D10 and RFC-0005's criterion 4 require, and gated on the
measurement §B8 names.

## Motivation

Bund2 has no output: a Bund program cannot be distributed as a program.

Three register entries already depend on this RFC, and two are constraints it
inherits rather than goals it chooses:

- **D10** resolves that `--emit=native` may require `cc` **and** that
  `--emit=bundle` must stay toolchain-free, calling the clause after the
  semicolon "load-bearing, not decoration": "nothing below `bund2 build` may
  require `cc`."
- **D40** feature-gates `string.grok` off by default because a C dependency
  beneath the toolchain-free artefact would invert D10's escape hatch.
- **RFC-0005's criterion 4** cannot run against the JIT: `cranelift-jit`
  consumes relocations when it finalises a definition and exposes no accessor;
  `cranelift-object` keeps them behind `relocs()`.

## Current behaviour

### The reference's two object producers, both unreachable

D11 resolved that nothing external depends on the object format. Its proof
named **one** producer; there are **two**, and this RFC re-read both against
the pinned SHAs rather than citing the register.

- `Value::compile` refuses anything that is not a LAMBDA, then delegates to
  `to_binary` (`reference/rust_dynamic/src/bincode.rs:38-50`). Its only caller
  in the submodules is a test of itself
  (`reference/rust_dynamic/tests/bincode-test.rs:103`).
- **`compile_to_binary` is the whole-program one, and D11 does not mention
  it.** It parses source, folds the token vector into a **single LIST** through
  `bund_vec_to_list`, and serialises that one value
  (`reference/bund_language_parser/src/compile.rs:6-30`). It has **zero
  callers** anywhere in the submodules.

**This strengthens D11 rather than weakening it** — there are two producers and
neither is reachable from a Bund program, so no artefact of either shape exists
in the wild. It also corrects this RFC's earlier reasoning: the reference's
whole-program form is *one value*, not a vector of them, so "one value per
element" was never the shape to preserve.

### What Bund2 has

- `bund2_syntax::compile` and `lower_with_spans` turn source into the program
  stream, a `Vec<BundValue>`; `Lowered` additionally carries spans
  (`crates/bund2-syntax/src/lib.rs`).
- `MAX_NESTING` is **1024** — how deeply `{ }`, `[ ]` and `( )` may nest,
  because the parser is recursive descent (F116, `crates/bund2-syntax/src/lib.rs`).
- `Vm::eval_indexed` evaluates the stream and reports a failing index, which is
  what lets a diagnostic carry a source location (`crates/bund2-interp`).
- `bund2_value::wire` encodes and decodes a `BundValue`. **`MAX_WIRE_DEPTH` is
  256.**
- `bund2-cli`'s `main` runs everything on a spawned thread and declares the
  Tier 1 share (`crates/bund2-cli/src/main.rs`).

### What the wire codec does to a value, which this RFC did not read before

Three properties, each verified in this session, and together they are why §B2
changed:

- **Encoding materialises the identity and the stamp** — D20, stated at
  `WireValue::from_value` in `crates/bund2-value/src/wire.rs`: "A value that
  has not been asked for either is given both here."
- **Decoding yields `BundValue::Heap`.** `into_value` builds
  `Rc<HeapValue>` with `stamp: Cell::new(node.stamp)`. Every scalar comes back
  boxed, carrying the stamp it was encoded with.
- **`Guard::TopAreInt` admits only `BundValue::Int`** —
  `admits_with` in `crates/bund2-ir/src/fragment.rs` matches
  `Some(BundValue::Int(_, _))` and nothing else.

## Design

### §B1 — How the artefact is constructed

**`bund2 build --emit=bundle` invokes no compiler.** This is the part the
first draft left unstated, and the obvious reading — `include_bytes!` into a
stub and link it, as `docs/research/02-native-binaries.md:44-49` describes —
**would violate D10**, because `rustc` links through `cc` on Linux and macOS.
That reading is rejected here, explicitly, so nobody reaches for it later.

Instead: `bund2 build` **copies a prebuilt runtime executable for the target
and appends the payload**, followed by a fixed-size trailer carrying a magic,
the format version, the pinned SHAs, and the payload's length. At start-up the
runtime reads its own executable, finds the trailer at the end, and takes the
payload from the offset the length implies.

Three consequences, stated because each is an assumption the first draft made
silently:

- **`bund2 build` ships or locates a prebuilt runtime per target it can bundle
  for**, including targets Cranelift does not support. Producing a bundle for
  a target is therefore gated on having that runtime, not on having a compiler.
- **Criterion 3 is about the prebuilt runtime, not a per-build stub.** `cargo
  tree` on a stub crate would pass today, before any bundle exists, which makes
  it worthless as written; the criterion below asks the shipped runtime binary
  instead.
- **Appending is not linking.** No relocation, no symbol resolution, no
  platform object knowledge. The cost is that the artefact is a runtime plus a
  tail rather than a single linked image, which is invisible to anyone running
  it.

### §B2 — What is embedded: the source text

**The payload is the program's source text, not an encoding of its values.**
The first draft encoded the stream with `wire`, and the review found four
changes in meaning that follow. All four were reproduced.

1. **Stamps would be fixed at build time.** Encoding materialises the stamp
   (D20), and decoding restores it. Every run of the bundle would report the
   *build* moment as each literal's creation time. **D2 rules that out in
   terms**: "`stamp` is creation time", and a lazy stamp "must still answer
   'when was this value constructed', which rules out sampling the clock at
   observation time". Build time is not construction time either.
2. **Every `( … )` context would carry the same name on every run**, because
   the name would be minted once, at build.
3. **Every scalar literal would come back boxed**, and this is the one the
   review understated. `Guard::TopAreInt` admits only `BundValue::Int`, so in a
   bundle **no arithmetic fragment would ever admit**: Tier 0's fragment fast
   path and all of §S6's inlining would be dead for every literal in the
   program. A bundle would be slower than the interpreter, and RFC-0005's
   promotion machinery would never fire — while every figure reported success,
   exactly as §B3's floor failure does.
4. **Diagnostics would lose their locations**, because an encoded stream
   carries no spans and no source, and `Diagnostic`'s Bund source location is
   part of D36's structured report rather than decoration.

**Embedding the source text removes all four at once**, because the artefact
then does what the CLI does: `lower_with_spans`, then `eval_indexed`. Literals
are constructed when the program runs, contexts are named per run, scalars stay
unboxed, and spans exist because the source does.

**The deviation, which needs sign-off.** D10's parenthetical describes
`--emit=bundle` as "runtime plus embedded IR". This design embeds source
instead. D10's *resolution* is about the toolchain and is untouched; its
*description* is not what this does. Two further costs, stated rather than
buried: the program is recoverable from the artefact in readable form, and
start-up pays a parse. **This RFC does not adopt the deviation silently — it
is listed in the preservation table and in the open questions, and it is the
one thing in this document that requires the owner before it is built.**

**The encoded-stream option is deferred, not rejected on taste.** If start-up
parse time ever matters, the answer is not the `wire` codec but a container
designed for a program — unboxed scalars, no stamps, spans carried alongside —
which D11's "version the IR format freshly" already licences. That is gated on
a measurement: the parse cost of the largest corpus program at start-up,
against the whole run. Nobody has taken it.

### §B3 — The stub, and the evaluation thread it must reproduce

**The start-up path cannot simply call `eval`.** `bund2-cli`'s `main` runs
everything on a spawned thread sized `TIER0_PART + TIER1_SHARE +
STACK_RESERVE`, then declares the region with `set_stack_region_with_share`
(`crates/bund2-cli/src/main.rs`). The reason is recorded at the call:

> A share is declared, never inferred. Declaring through `set_stack_region`
> would leave the share at zero, putting the Tier 1 floor above the thread's
> top so that every compiled body declines.

That is the state `compiled_entries` was added to expose — a tier that compiles
everything, enters nothing, and reports success on every other figure
(`crates/bund2-runtime/src/tier.rs`,
`a_tier_with_no_share_compiles_bodies_and_enters_none`). A runtime that spawns
no thread, or spawns one without declaring the share, produces an artefact
whose JIT is dead and whose reports say it is working. Criterion 6 holds it to
that, and the decode happens **on that thread**, where a depth of 256 is known
to be safe.

**Four things the artefact's front end must settle, and does here:**

- **argv.** The runner consumes nothing. A bundle is the program, so all of
  argv reaches `args` and `args.parse` (`crates/bund2-stdlib/src/host.rs`,
  `script_args`). The CLI's own flags — `--stats`, `--dump-stack`,
  `--raw-values`, `--jit-threshold` — are **not** available in a bundle,
  because a bundle that swallowed `--stats` would shadow a program's own
  argument. Diagnostic flags move to environment variables.
- **The exit code** is `vm.exit_requested()`, as the CLI returns
  (`crates/bund2-cli/src/main.rs`, `run_cli`).
- **The reporter** is the CLI's `TextReporter`, with the same `wants_stack`
  policy, so a fatal report carries a stack snapshot and a warning does not
  (D45, `crates/bund2-stdlib/src/report.rs`).
- **One program.** `bund2 build` takes exactly one source file. Not a
  directory, not several files, not standard input; a program that needs more
  reaches them through `use`, which D76 settles.

### §B4 — What the image retains

**No tree-shaking by word reachability.** D16 makes a call target a name
assembled at run time, so no registered word can be proven dead. The register
states the consequence in terms: "An AOT image retains the word table and the
name resolver."

**The corollary the first draft did not draw: the feature set the runtime was
built with decides which words exist.** A program calling `string.grok` works
under a `grok` build and fails in a default-feature bundle — as an unknown word
**at run time, never at build time**, since `bund2 build` does not resolve
names (it cannot, per D16). This is D40's feature gate seen from the artefact
side. `bund2 build` therefore records in the trailer which features its runtime
carries, and `--emit=bundle --features` selects among the runtimes it has.

### §B5 — `--emit=native`, and what blocks it

**The first draft said "the same lowering, pointed at `ObjectModule`". That is
wrong, and the reason is in the lowering's own safety argument.**

`Compiler::compile_word` takes `vm: &mut dyn Vm` and calls `plan_body(body,
vm)`: the plan is made against a **live** VM, resolving slots and effects from a
registry that exists. And the emitted code **bakes the compiling process's heap
addresses in as immediates** — `slots_base` is `slots.as_ptr() as i64`, issued
as `iconst`, with the comment saying exactly why that is safe:

> §S6's cells, addressed as an immediate. The allocation outlives every
> compiled function — both die with the `Interp` — which is what makes
> embedding its address safe.

**That argument is precisely what fails for AOT.** In an object file the
address belongs to a process that has exited before the artefact runs.

So `--emit=native` needs three things the lowering does not have, and this RFC
states them rather than assuming the mode is close:

1. **Slot tables and cells addressed through symbols resolved at load**, not
   baked immediates — which is also what would make criterion 4's relocation
   records meaningful rather than empty.
2. **Planning without a live VM**, or a build-time registry standing in for
   one, with the rule that anything it cannot resolve statically stays a
   generic call.
3. **A statement of which bodies compile at build time**, when no word has been
   registered: a program's lambdas are not bound until it runs. Words
   registered at run time (`register`, `bund.eval`, `use`) have no code
   generator in the shipped image and stay interpreted — a change in speed, not
   meaning, recorded so nobody later "fixes" it by shipping Cranelift.

**Criterion 7 is deferred behind (1) and (2).** Defining it against the current
lowering would define a criterion that cannot be implemented.

### §B6 — Cross-compilation and the C dependency

D40 is inherited whole: `string.grok` is gated off by default because a C
dependency beneath the artefact would invert D10's escape hatch, and a
cross-compiled `cranelift-object` build inherits the same problem. A target
Cranelift does not support gets `--emit=bundle`, which is why that mode is
mandatory rather than convenient — and, under §B1, why it needs a prebuilt
runtime for that target rather than a compiler on the build host.

### §B7 — `use` in a built artefact

**A bundle's `use` and `use.` fetch when they run, exactly as the
interpreter's do** — D76, on Q38. The operand is resolved at run time and the
text evaluated in the running VM, so a word the fetched file registers stays
registered. Nothing is embedded at build time, and nothing is refused that the
interpreter would accept.

**The fetch inherits D54 whole, stated so the artefact's behaviour is
documented rather than discovered:**

- `file://` follows curl's rules — an absolute path, optionally after the host
  `localhost`, with `%xx` decoded.
- `http://` is fetched by `ureq` built without TLS. It keeps the defaults the
  reference leaves curl at: no redirect is followed, the body of an error
  status is still the answer, and the body has no size limit. **These three are
  what `file_helper.rs:42-54` leaves unset rather than what any one line
  states**; the user agent `ZBUS` is set, at
  `reference/Bund/src/stdlib/helpers/file_helper.rs:43`.
- A string with no scheme is refused, and `https://` is refused — both approved
  deviations under D54, the second because a TLS stack compiles C or assembly
  that D10 does not allow below `bund2 build`.

A `file://` path resolves on the machine running the artefact, not the one that
built it, and an `http://` target is fetched in the clear.

**Whose risk that is, stated rather than implied.** The person running the
artefact is responsible for what it fetches and for the safety of doing so —
D76's ruling. `use` evaluates what it retrieves; that is what the word does in
the reference and this RFC preserves it. **A bundle adds no check the
interpreter does not have, and claims none.**

**Embedding is deferred, not foreclosed**, behind a flag with its own decision
and in the source-text form D76 fixes — which is now also the form §B2 takes
for the program itself.

### §B8 — The gate

RFC-0005 was gated on a measurement before implementation began; this RFC is
gated the same way. The research names the number for `--emit=native`:

> It should be measured early — `cargo bloat` on a Product A binary with and
> without the `jit` feature — because the number decides whether B is worth
> the phase.

**It has never been taken, and it cannot be until `--emit=bundle` exists**,
because it is a measurement of a bundle. That is the ordering argument, and it
is stronger than convenience: `--emit=native`'s case rests on a size saving
nobody has weighed, and §B5 now shows its cost is larger than the first draft
implied.

## Preservation analysis

| behaviour | disposition |
|---|---|
| The reference's object formats | **Not preserved, and nothing can depend on either.** Two producers, `Value::compile` and `compile_to_binary`, both with zero reachable callers. D11. |
| Program meaning under a bundle | **Preserved exactly**, because the artefact parses and evaluates the same source through the same entry points. Criterion 2 checks it per golden. |
| `.timestamp` of a literal | **Preserved** — literals are constructed at run time, as §B2 requires. This is the row that failed under the encoded stream. |
| The anonymous `( … )` context name | **Preserved** — minted per run. |
| Scalar representation, fragment admission, tier agreement | **Preserved** — scalars stay unboxed, so `TopAreInt` admits and inlining still fires. |
| Diagnostic source locations | **Preserved** — the source is present, so spans exist and D36's reports carry locations. |
| Programs nesting 257–1024 deep | **Preserved under `--emit=bundle`.** The parser's `MAX_NESTING` of 1024 governs, and `MAX_WIRE_DEPTH` does not apply, because nothing is wire-encoded. The first draft claimed preservation while refusing these programs. |
| Word table and name resolver | **Preserved in full.** No tree-shaking; D16. |
| Which words exist | **Changed, and it is D40's gate seen from outside.** The runtime's feature set decides; a missing word fails at run time. Recorded in the trailer (§B4). |
| `args` / `args.parse` | **Preserved.** All of argv reaches the program; the runner consumes nothing (§B3). |
| Exit code | **Preserved** — `vm.exit_requested()`. |
| Diagnostic flags (`--stats`, `--dump-stack`, `--raw-values`) | **Deliberately changed.** Not argv flags in a bundle, because they would shadow the program's own arguments; they move to environment variables. |
| `use` | **Preserved exactly** — D76, under D54's scheme set. |
| Run-time-registered words under `--emit=native` | **Speed only.** No code generator in the image, so they stay interpreted. |
| D10's "embedded IR" description | **Deviation, awaiting sign-off.** Source text is embedded instead; D10's resolution about the toolchain is untouched. |

## Alternatives considered

- **`include_bytes!` into a stub and link it**, as the research describes.
  Rejected on D10: `rustc` links through `cc`. §B1.
- **Encoding the program with the `wire` codec.** Rejected on four measured
  changes in meaning, §B2 — not on taste, and the first draft proposed it.
- **A fresh program container** with unboxed scalars, no stamps and spans
  carried alongside. **Deferred, not rejected**, behind a start-up parse
  measurement; D11 licences it.
- **`--emit=native` first.** The research recommends object-output-first for
  the *lowering spike*; that ordering is moot, since the lowering exists and was
  built JIT-first. What remains is a size saving §B8 shows cannot be weighed
  until a bundle exists, against a cost §B5 shows is larger than assumed.
- **Tree-shaking behind a flag.** Foreclosed by D16, not by this RFC.
- **A front end that evaluates on the main thread.** Rejected on §B3.

## Acceptance criteria

1. **The gate is answered before `--emit=native` is built.** `cargo bloat` on
   a bundle runtime with and without `jit`, reported as a figure. No threshold:
   the number is an input to a decision.
2. **Every bundled golden matches its source run, per golden and not in
   total.** `cargo xtask conform` over bundles reports the **same pass/fail for
   each golden** as the source run, and the same CEILING. A totals comparison
   would let one new failure hide one new pass.
3. **The shipped runtime contains no code generator.** `cargo tree` on the
   **prebuilt runtime binary's** crate with its bundling features, listing
   neither `cranelift-codegen` nor `cranelift-jit` in a default build — not on
   a stub crate, which would pass today with no bundle in existence.
4. **A bundle is produced with no C toolchain and no compiler.** Built in an
   environment with no `cc` **and no `rustc`**, which §B1 makes possible and is
   a stronger check than reading a dependency list.
5. **A program nesting up to `MAX_NESTING` bundles and runs.** 1024 levels, not
   256: the wire cap does not apply, and a criterion set at 256 would pin the
   defect the first draft had.
6. **The front end declares its Tier 1 share.** A bundle built with `jit`,
   running a body past the threshold, reports `compiled_entries() > 0` — the
   figure a broken front end leaves at zero while every other figure reports
   success.
7. **Deferred.** RFC-0005's criterion 4 — no relocation targeting a function
   beyond §S8's two — is discharged here once §B5's (1) and (2) exist. Stated
   as deferred rather than written against a lowering that cannot emit it.
8. **`--emit=native` matches Tier 0's conformance exactly.** RFC-0005's
   criterion 2 applied to this mode: the same N/M and CEILING as the
   interpreter, per golden. Missing from the first draft entirely.
9. **A literal's `.timestamp` is a run-time value.** The same bundle run twice
   reports different stamps for the same literal, and neither is the build
   time. This is B2.1 made checkable rather than argued.
10. **Conformance moves by exactly zero.** This RFC changes what Bund2 emits,
    not what a program means.

## Open questions

- **The §B2 deviation.** Embedding source text departs from D10's descriptive
  parenthetical. **Awaiting the owner**; it is the one thing here that is not
  this RFC's to decide.
- **Q38 — answered by D76**, 2026-09-29: fetch at run time. Left listed so a
  reader tracing D76 finds it.
- **A flag that embeds `use` targets**, and **a fresh program container**. Both
  deferred with their triggers recorded; neither blocks anything and no default
  waits to be adopted.
