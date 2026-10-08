# RFC-0006: Ahead-of-time output — `bund2 build`

- Status: **Proposed** (2026-09-30, on the owner's authorisation — **D82**),
  after three adversarial reviews of this document and one of the code.
  `--emit=bundle` is **built and reviewed**. **`--emit=native` is withdrawn —
  D83** — and RFC-0005's criterion 4, the one thing it was uniquely needed for,
  is discharged by inspection instead. The implementation review
  (`docs/rfc/reviews/RFC-0006-implementation-review-2026-09-30.md`) found two
  release-only blockers that eleven passing tests had not: the payload region's
  sentinel was unique only in debug, and the runtime read it from an immutable
  `static` that release folded to its initialiser. Both fixed;
  `cargo xtask bundle` now checks both profiles, which is the gap that let them
  through. Built —
  `crates/bund2-cli/src/bundle.rs` and `bund2 build`. Criteria 3, 5, 6, 10, 11
  and 12 pass, **criterion 2 is met in three configurations**, and
  **criterion 1's gate is answered — against Product B's premise**. **Every
  criterion now carries its evidence in the list below**; 7 is met without
  §B5's work and 8 is withdrawn with the mode (D83). *(Until 2026-10-08 this
  line said 8 waited on `--emit=native` and 7 was deferred behind §B5, which
  D83 and the criteria list had both ended.)* Three rulings of 2026-10-08
  are landed below: **D115**, a bundle is never given a debugger; **D116**,
  parse-at-build is ratified; and §B3a names two more ungated routes.
  **A sixth review the same day**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-08.md`) found four blockers.
  Three are answered: `--noeval`'s group is six words and not four, `--noio`'s
  gated and ungated words are listed by name from a measurement, and
  `bund2 build` now refuses arguments it does not understand. **The fourth is
  ruled on — D118**: §B1 specified a prebuilt runtime per target, what is
  built copies the building binary, and the building binary is the design.
  **D119**, the same day: `--noio` gates `csv` and `sqlite`, which the
  reference's does not.
  Revised 2026-09-29 after the first adversarial review
  (`docs/rfc/reviews/RFC-0006-review-2026-09-29.md`). The review raised three
  blockers; all three were reproduced against the code before this revision,
  and one is worse than it reported. §B2's deviation is **ruled on — D77**. A
  **second review** found one blocker, ruled on as D78. A **third** found two
  more, both now ruled on: what `--noeval` means (**D79**) and whether a bundle
  may carry the JIT (**D80**).
- Depends on: RFC-0003 (the program stream and Tier 0), RFC-0005 (the
  Cranelift tier)
- Decisions consumed: D10, D11, D16, D20, D40, D54, D74, D76, D77, D78, D79,
  D80, D81, D82, D83, D115, D116, D118, D119, and
  decisions.md's "What this forecloses" clause on tree-shaking. *(Until
  2026-10-08 this list named D44, which the body never uses, and omitted the
  last four, which it rests on.)*
- Touched but not consumed: D1, D2, D31, D36, D37, D45, D113 — see the
  preservation table for D1, D2 and D36; the others are cited where used
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

**"The trailer" means the header of the payload region.** §B1 began as an
appended payload with a trailer at the end of the file, and measurement
replaced that with a fixed region inside the image (Q40). The word survives
through §B3, §B3a, §B4 and the preservation table, and everywhere after §B1's
reversal it names the region's header fields — container version, state,
flags, length, Bund2 version, source path, features, pinned SHAs
(`crates/bund2-cli/src/bundle.rs`, `Region`). Nothing is read from the end of
a file.

## Summary

`bund2 build` produces a runnable artefact. Two modes, in scope together
because D10 decides them together:

- **`--emit=bundle`** — a copy of the building `bund2` (D118) with the
  program's **source text** written into it. **No compiler is invoked at all**: not Cranelift, not `rustc`, not
  a linker, not `cc`. Runs on every target Rust runs on.
- **`--emit=native`** — **withdrawn, D83.** Its stated justification was
  measured away by criterion 1, what remained was not enough against §B5's
  costs and a risk §B5 had missed, and criterion 4 turned out not to need the
  mode at all. §B5 stands as the record of what building it would take.

`--emit=bundle` is specified in full and built. `--emit=native` was specified
to the depth D10 and RFC-0005's criterion 4 required and gated on the
measurement §B8 names; that measurement went against it and D83 withdrew it.

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

**Neither is reachable from Rust, but the bytes are reachable from Bund**, and
the first revision of this RFC got that wrong by repeating D11's own gap.
`encode.base64` pulls a value and calls **`to_binary`** on it
(`reference/Bund/src/stdlib/functions/encoding/base64.rs:33`) — the same
serialiser — so `"1 2 +" compile encode.base64` yields base64 of a LIST in the
format `compile_to_binary` would write. **The same format, and not the same
bytes — corrected 2026-10-08.** This sentence said "exactly the bytes", which
is false twice over. The Bund word `compile` is not `compile_to_binary`'s front
half: it appends a newline before parsing and stops at the first `EXIT`
(`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:29-36`). And a
`Value` serialises its `id` and `stamp`, which are minted when it is
constructed (`reference/rust_dynamic/src/value.rs:15-36`), so two parses of
one text do not produce equal bytes at all. D11 is amended accordingly — **twice**, because the first amendment was also
wrong. Those bytes do not "reach only a Bund string": `save.*` writes **every
lambda** to the world file with `to_binary`, which is exactly
`Value::compile`'s format, into a SQLite table on disk
(`reference/Bund/src/stdlib/helpers/world/lambdas.rs:80-95`). So the format
does reach a file, and whether anything outside Bund2 reads that file is
**D31**. **D31 is RESOLVED** — on 2026-09-11, "no external readers" — and this paragraph said it was OPEN until 2026-10-08, eighteen days
after the ruling. So the dependency it drew is closed: nothing outside Bund2
reads the file the format reaches. This RFC rests on neither decision, since
§B2 embeds source text and uses neither format. **Which words write it**:
`save` and `save.lambdas` call `save_lambdas`
(`reference/Bund/src/stdlib/functions/bund/bund_save.rs:19`, `:53`);
`save.aliases` and `save.stacks` were not followed, and "every lambda" above
is a claim about those two words and not about the glob.

What this section does establish is the shape: the reference's whole-program
form is **one LIST value**, not a vector of them, so "one value per element" was
never the thing to preserve.

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

**The construction in the next three paragraphs was replaced — read on to
"So the payload goes inside the image".** It is kept because the measurement
that replaced it is only intelligible beside it.

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
  **Not the design — D118, 2026-10-08.** The runtime is the binary doing the
  building; see "What is built" below.
- **Criterion 3 is about the prebuilt runtime, not a per-build stub.** `cargo
  tree` on a stub crate would pass today, before any bundle exists, which makes
  it worthless as written; the criterion below asks the shipped runtime binary
  instead.
- **Appending is not linking.** No relocation, no symbol resolution, no
  platform object knowledge.

**Appending was measured on 2026-09-30, and it does not survive signing — Q40.**
The construction above was written on an assumption marked `[UNGROUNDED]`. It
was then tested on this machine, macOS on arm64, against the repository's own
`bund2` binary, which the linker signs ad hoc (`codesign -dv` reports
`adhoc,linker-signed`):

| case | runs | `codesign -v` |
|---|---|---|
| unmodified | yes | validates |
| **34 bytes appended** | **yes** | **"main executable failed strict validation"** |
| 5 MB appended | yes | fails the same way |
| appended, then `codesign -f -s -` | yes | **still fails** — re-signing does not repair it |
| unmodified, then `codesign -f -s -` | yes | **validates** |

The last two rows isolate the cause: **trailing data outside the Mach-O image
is what `codesign` refuses**, not modification. An unmodified binary re-signs
and validates; an appended one cannot be made to, and the payload survives the
attempt.

**Quarantine does not distinguish the two.** An ad-hoc-signed binary carrying
`com.apple.quarantine` is killed with SIGKILL whether or not anything was
appended — the unmodified control died the same way (`rc=137`). That is
Gatekeeper requiring notarisation, which is orthogonal to this design, except
that notarisation needs a valid signature and appending precludes one.

**So the payload goes inside the image, not after it** — the prebuilt runtime
reserves a fixed-capacity region, **1 MiB on the owner's ruling**, and
`bund2 build` writes into it without changing the file's size or layout. The
cost is a capacity ceiling, which the region records along with the used
length, and a program larger than it is refused at build with both numbers
named.

**The in-place form does not run until it is signed again, and the first
version of this section had that backwards.** It said signing "happens
afterwards … so D10 is untouched", as though it were optional. Building the
thing showed otherwise, on 2026-09-30:

| form | runs unsigned | validates | after `codesign -f -s -` |
|---|---|---|---|
| appended | **yes** | no | still no |
| in place | **no — SIGKILL** | no | **yes: runs and validates** |

The signature covers the bytes `bund2 build` writes, so an unsigned in-place
edit is not a validation warning — the kernel refuses to execute it, and a
bundle produced this way exits 137 with no output. **An ad-hoc re-sign restores
both properties**, which is what makes a later Developer ID signature and
notarisation possible at all. So `bund2 build` re-signs on macOS, and
`crates/bund2-cli/tests/bundle_build.rs` holds it there.

**This does not spend D10 — D81.** `/usr/bin/codesign` is a base-system binary,
root-owned and on the root volume, outside any Command Line Tools path; D10
forbids a *C toolchain* below `bund2 build`, and this needs neither a compiler
nor an install. D81 records one limit: several `/usr/bin` tools on macOS are
stubs that prompt for Command Line Tools, and whether `codesign` is among them
could not be checked on a machine that has them. If it is, the appending form
is the fallback.

**Appending is what the alternative would be**: toolchain-free and immediately
runnable, at the price of an artefact that can never be signed or notarised on
macOS. It is not implemented, because an unsignable artefact is a worse default
than one system tool, and because supporting both forms would mean the runtime
reading its own executable — the failure class the region was chosen to avoid.

**What was not tested, and is not claimed.** A Developer-ID-signed and
notarised binary — there is no signing identity in this repository. Linux and
Windows: ELF and PE carry no equivalent whole-file signature by default, so
appending is *expected* to be unaffected, and that is reasoning from the
formats rather than a measurement.

**What the runtime does when the region is empty, or is wrong.** D37 forbids
a panic, and a bundle runtime is the same binary as the interpreter, so this
path is reached by every `bund2` that starts: each one asks its own image
whether it carries a program before it reads argv
(`crates/bund2-cli/src/main.rs`, `run_cli`). One case that is not a failure
and four that are, none of them an abort (`crates/bund2-cli/src/bundle.rs`,
`carried` and `Damaged`):

- **The state byte says empty.** Nobody built from this runtime. It is the
  plain interpreter.
- **A container version this runtime does not read**: refused, naming both
  versions and saying the artefact was built by a different bund2.
- **A state byte that is neither empty nor filled**: refused as damaged.
- **A length past the region's capacity**: refused as damaged, before any
  read of the payload.
- **A payload that is not UTF-8.** The payload is source (§B2), so this is
  refused before the parser sees it.

Criterion 12 checks all five, because a corrupted artefact is the one input a
bundle's front end is guaranteed to meet eventually and the only one that could
reach for an `unwrap`.

*Until 2026-10-08 this list was the appended form's: no trailer, a truncated
trailer, a length beyond the file, unreadable text. A fixed region cannot be
truncated, and a bad state byte and an unknown container version were in
neither that list nor criterion 12's. The sixth review, S6.*

**Every `bund2` carries the region, bundle or not.** 1 MiB in every build, and
a damaged region stops the plain interpreter as it stops a bundle. That is a
change to the interpreter's start-up and not only to artefacts.

**The trailer records Bund2's own version, not only `reference/`'s SHAs.** A
builder and a prebuilt runtime can be different Bund2 builds — the pinned SHAs
say which oracle the *meaning* was fixed against and say nothing about the code
doing the interpreting. Without Bund2's version in the trailer, a mismatch is
undetectable; with it, the runtime can refuse or warn on one it does not
recognise.

**What is built, stated 2026-10-08 because the paragraphs above describe more
than exists — the sixth review, B4.** `bund2 build` copies
`std::env::current_exe()`: **the runtime is the binary doing the building**
(`crates/bund2-cli/src/main.rs`, `build`). Four things follow, and none was
written down:

- **A bundle is always for the builder's own target.** No prebuilt runtime is
  shipped or located, so a bundle for another target cannot be produced — the
  case §B6 gives as the reason the mode is mandatory.
- **A bundle always carries the builder's own feature set.** A JIT bundle is
  made by building `bund2` with `jit` and running *that* binary's `build`,
  which is how `a_jit_bundle_enters_compiled_code` gets one. `--features` on
  `bund2 build` is refused, with that explanation.
- **Builder and runtime cannot differ**, so the version skew described above
  cannot occur today. The version is recorded and `--inspect` prints it; the
  runtime does not read it, and "refuse or warn" is unbuilt.
- **§B3's "the artefact re-parses at start-up" is safe because of this.** The
  parser that accepted the program at build is the parser that reads it at
  start. With a separate prebuilt runtime that would stop being true.

**The built form is the design — D118, ruled 2026-10-08**: "bundle runtime is
the building binary". So the first bullet is a stated limit and not a gap, the
version field is for a reader of `--inspect`, and the skew paragraph above
describes a construction that was not taken. **Cross-target bundling is
deferred, not foreclosed.** Its trigger is a need to produce an artefact for a
target from a machine that cannot run that target's `bund2`; it would need
runtime discovery, a version check the runtime acts on, and a new answer for
parse-at-build, and it would be a new decision.

### §B2 — What is embedded: the source text

**The payload is the program's source text, not an encoding of its values.**
The first draft encoded the stream with `wire`, and the review found four
changes in meaning that follow. All four were reproduced.

1. **Stamps would be fixed at build time.** Encoding materialises the stamp
   (D20), so a literal would arrive at run time **already stamped**.

   **D2 decided the stamp is sampled when a value is first observed** — "The
   clock is not read at construction at all" — and recorded that as an approved
   deviation from preservation. The first revision of this RFC quoted D2's
   *constraint* ("stamp is creation time") as if it were D2's ruling, which is
   backwards; D77 carries the same correction. The finding survives on the rule
   as actually decided: the first observation in a run must set the stamp, and
   under an encoded stream every run would instead report the build moment.
2. **Every `( … )` context would carry the same name on every run**, because
   the name would be minted once, at build.
3. **Every scalar literal would come back boxed**, and **Tier 0's fragments
   would decline every one.** `Guard::TopAreInt` admits only
   `BundValue::Int(_, _)` (`crates/bund2-ir/src/fragment.rs`, `admits_with`),
   so no arithmetic fragment would admit and Tier 0's fast path would be dead
   for every literal in the program.

   **Tier 1 is not affected, and the first revision of this RFC said it was.**
   `plan_body` tests `dt() == INTEGER` and then `as_int()`, and `as_int`
   descends `BundValue::Heap` into `Payload::Scalar` before answering
   (`crates/bund2-jit/src/lower.rs`, `plan_body`;
   `crates/bund2-value/src/lib.rs`, `as_int`). Promotion and inlining still
   fire on a boxed literal. The claim that "all of §S6's inlining" would die
   was an overstatement, corrected on the second review and amended in D77.
4. **Diagnostics would lose their locations**, because an encoded stream
   carries no spans and no source, and `Diagnostic`'s Bund source location is
   part of D36's structured report rather than decoration.

**Embedding the source text removes all four at once**, because the artefact
then does what the CLI does: `lower_with_spans`, then `eval_indexed`. Literals
are constructed when the program runs, contexts are named per run, scalars stay
unboxed, and spans exist because the source does.

**The deviation, ruled on by D77.** D10's parenthetical describes
`--emit=bundle` as "runtime plus embedded IR". This design embeds source
instead. D10's *resolution* is about the toolchain and is untouched; its
*description* is not what this does. Two further costs, stated rather than
buried: the program is recoverable from the artefact in readable form, and
start-up pays a parse. **The deviation was not adopted silently: it was put to the owner and
decided on 2026-09-29 (D77), which also closed the alternative below.**

**The encoded-stream option is closed, on the measurement it was waiting
for — D77.** The first revision deferred it "behind a start-up parse
measurement"; that measurement is now taken, and deferring it further would
invite someone to build a second serialisation format for a saving the numbers
call negligible.

| | measured |
|---|---|
| `startup/parse/mixed`, 891 bytes | **4.29 µs** [4.2775, 4.3022] |
| `startup/registry/register_all` | **41.8 µs** [41.706, 41.938] |

**The parse is about a tenth of the registry construction every bundle pays
regardless.** Scaling to the corpus's largest program — `workbench-variants.bund`
at 4,893 bytes — gives roughly 24 µs, and **that is arithmetic on bytes, not a
measurement**; even a tenfold error leaves the parse under the setup cost.

**What the numbers are and are not.** Two absolute figures an order of
magnitude apart, taken on an unguarded host. They are not an A/B and F135's
protocol does not apply; the conclusion they support is the order of magnitude,
not the third digit.

So the ~20 µs a container would save is set against designing, testing and
versioning a second format that must independently re-solve all three of §B2's
mechanical findings: carry spans, or diagnostics lose locations again; encode
scalars unboxed, or `TopAreInt` declines again; and encode "unset" for stamps,
which is **D20's deferred step** — unblocked by D11 and never built. The
parser already gets all three right.

### §B3 — The stub, and the evaluation thread it must reproduce

**The start-up path cannot simply call `eval`.** `bund2-cli`'s `main` runs
everything on a spawned thread sized `EVAL_STACK` — `TIER0_PART + TIER1_SHARE
+ STACK_RESERVE` — and that thread declares its region through
`declare_region` (`crates/bund2-runtime/src/lib.rs`, both symbols; called from
`crates/bund2-cli/src/main.rs`, `main`). The reason is recorded at the call:

> **A share is declared, never inferred** — RFC-0005 §S8, the ninth review's
> S3. `declare_region` is the one place that does it, now that RFC-0007 §C8's
> host spawns threads of its own: two copies of three constants and a two-line
> call would be two chances to omit the share, which is the failure criterion
> 18 and `compiled_entries` exist to catch.

*(Until 2026-10-08 this quoted an earlier comment, since rewritten, and named
`set_stack_region_with_share` in `main.rs` as the call. The failure is the
same one: a share left at zero puts the Tier 1 floor above the thread's top,
so every compiled body declines and this RFC's criterion 2 would pass with no
compiled code having run.)*

That is the state `compiled_entries` was added to expose — a tier that compiles
everything, enters nothing, and reports success on every other figure
(`crates/bund2-runtime/src/tier.rs`,
`a_tier_with_no_share_compiles_bodies_and_enters_none`). A runtime that spawns
no thread, or spawns one without declaring the share, produces an artefact
whose JIT is dead and whose reports say it is working. Criterion 6 holds it to
that.

**Nothing is decoded, and the earlier revisions of this sentence said otherwise.**
They had the payload decoded on that thread "where a depth of 256 is known to
be safe", which was true of the encoded stream §B2 no longer uses. What happens
on that thread is the **parse**, and the depth that matters is the parser's
`MAX_NESTING` of 1024, not `MAX_WIRE_DEPTH`'s 256 — which is also why criterion
5 is set at 1024. Three reviews raised this sentence; it is the last of them.

**Six things the artefact's front end must settle, and does here:**

- **argv.** The runner consumes nothing. A bundle is the program, so all of
  argv reaches `args` and `args.parse` (`crates/bund2-stdlib/src/host.rs`,
  `script_args`). The CLI's own flags — `--stats`, `--no-dump-stack`,
  `--raw-values`, `--jit-threshold` — are **not** available in a bundle,
  because a bundle that swallowed `--stats` would shadow a program's own
  argument. They move to environment variables, and the names are part of this
  design rather than left to the implementation: `BUND2_STATS`,
  `BUND2_NO_DUMP_STACK`, `BUND2_RAW_VALUES`, and **`BUND2_JIT_THRESHOLD`, which
  the interpreter already reads** (`crates/bund2-runtime/src/lib.rs`,
  `threshold_from_env`). Criteria 2, 6 and 10 name which of these they set.
  **The stack variable switches the dump off, because it is on by default**,
  as it is for the CLI, whose flag for this is `--no-dump-stack`. Until
  2026-10-08 this list named `BUND2_DUMP_STACK`, which the binary has never
  read: measured on a bundle of `1 nosuch`, setting it to `0` left the dump
  in the report and `BUND2_NO_DUMP_STACK=1` removed it.

  **Three more are read, and were missing from this list until 2026-10-08:**
  `BUND2_NOIO` and `BUND2_NOEVAL`, which add §B3a's restrictions at run time
  and are the only way to exercise D78's floor, and `BUND2_NOCOLOR`. **The
  rule for all of them except the threshold is one rule**: a variable counts
  as set when it is present and is neither empty nor `0`
  (`crates/bund2-cli/src/main.rs`, `env_set`). So `BUND2_NOEVAL=0` and
  `BUND2_NOEVAL=` are both "unset", which is why those are the two values
  criterion 10 tries against a floor.
- **The exit code** is `vm.exit_requested()`, as the CLI returns
  (`crates/bund2-cli/src/main.rs`, `run_cli`).
- **The reporter** is the CLI's `TextReporter`, with the same `wants_stack`
  policy, so a fatal report carries a stack snapshot and a warning does not
  (D45, `crates/bund2-stdlib/src/report.rs`).
- **One program.** `bund2 build` takes exactly one source file. Not a
  directory, not several files, not standard input; a program that needs more
  reaches them through `use`, which D76 settles. **A second `--file` is
  refused** — see "The command line" below.
- **`bund2 build` parses before it writes.** A syntax error is a build error,
  reported with its span against the source it was given, and the parse result
  is then discarded — the artefact re-parses at start-up, which is what keeps
  §B2's preservation exact. Refusing at build costs one parse, measured at
  ~4.3 µs for 891 bytes, and is the difference between a broken program found
  by whoever built it and one found by whoever ran it.
- **The diagnostic file name comes from the trailer.** A bundle has no file on
  disk to name, and `Diagnostic`'s location carries one. `bund2 build` records
  the source path it was given, verbatim **up to 256 bytes**, and the artefact
  reports locations against it — so a bundle's stderr matches a `script` run
  of the same program from the same path. **A longer path keeps its last 256
  bytes**, so the file name survives and the leading directories do not
  (`crates/bund2-cli/src/bundle.rs`, `write_into`), and the build says so on
  stderr. Measured 2026-10-08 with a 353-byte path: the bundle's report names
  the shortened path, a `script` run names the whole one, and until that day
  the build said nothing. `a_source_path_too_long_to_record_is_reported_at_build`. Without this, criterion 2 would compare stderr that
  differs by construction, and one approved deviation's recorded hash pins a
  path Bund2 prints, so the CEILING would move.

**The command line, as built — added 2026-10-08, the sixth review's B3.**

    bund2 build --file <src> --output <path> [--noio] [--noeval]
    bund2 build --inspect <artefact>

`-o` is `--output`. `--emit=bundle` is accepted and changes nothing, because
it names the only mode there is; this document writes `bund2 build
--emit=bundle` throughout for that reason and the flag may be left off.
**Everything else is refused, with exit status 2 and nothing written**
(`crates/bund2-cli/src/main.rs`, `build_request`):

- `--emit=native` — "withdrawn and is not built (D83)".
- `--features` — a bundle is a copy of the building binary, so its features
  cannot be chosen here (§B1, "What is built").
- A second `--file`, `--output` or `--inspect`.
- Any argument it does not know.

Until 2026-10-08 the command looked up the flags it knew and skipped the
rest. Measured on that tree: `bund2 build --emit=native --features jit --file
p.bund --output n1` exited 0 and wrote a default-feature bundle — a withdrawn
mode accepted and a request for a different runtime answered with this one,
which is the silent failure §B3a argues against for the restriction flags.
`a_build_refuses_arguments_it_does_not_understand` holds the refusals.

### §B3a — What a bundle may switch off, and what that does not mean

**`--noio` and `--noeval` may be recorded in the trailer, and an environment
variable at start-up may add either and never remove one** — D78. Both are
`HostOptions` fields: `--noio` registers the I/O words as stubs that fail, and
`--noeval` does the same to the six words of its group, named below
(`crates/bund2-stdlib/src/host.rs`, `HostOptions` and `register_noeval_stubs`;
from the reference's own command line,
`reference/Bund/src/cmd/mod.rs:139-146`).

**The direction is the design.** A restriction an environment variable could
switch off would not be a restriction, and the failure would be silent — the
artefact would still report itself as built `--noeval` while evaluating
everything. So the trailer is a **floor**: run time may tighten, never loosen.

**The trailer is readable**, because a restriction the runner cannot observe is
one they cannot rely on. `bund2 build --inspect <artefact>` prints the
container version, the program's size against the capacity, the source path it
was built from, the restrictions, the Bund2 version and feature set, and the
pinned SHAs — **built 2026-09-30**, along with the two fields this RFC had
promised and the container had lacked.

An unrestricted artefact prints `restrictions none` rather than omitting the
line, because a missing line and "none" say different things to someone
deciding whether to trust what they were handed. The report also states the
floor's *direction*, or the restrictions line reads as the whole truth.

**These are word-group switches, and this RFC does not call them a sandbox.**
D78 rules the word out. What follows is what each flag leaves ungated, by name,
which D78 requires and the second revision of this RFC gave for `--noio` only.

**What `--noio` gates, by name.** Measured on Bund2 on 2026-10-08 by running
each of the 610 names `bund2 words` lists as a one-word program under
`--noio`: **56 answered with the stub, and 59 do since D119** added the last
line below the same day. They are —

- files and directories: `fs.cwd` `cwd` `fs.cp` `cp` `fs.mv` `mv` `fs.rm` `rm`
  `fs.is_file` `fs.ls` `fs.ls.` `ls` `ls.` `fs.ls.dir` `fs.ls.dir.`
  `fs.ls.files` `fs.ls.files.` `filename` `filename.` `file.write`
  `file.write.` `io.textfile` `io.textfile.`
- reading text from a file or a URL: `file` `file.` `url` `url.`
- the shell and the process: `system.shell` `system.shell.` `sh` `sh.`
  `system.setproctitle` `system.setproctitle.`
- the world file: `save` `save.aliases` `save.lambdas` `save.stacks`
  `save.model` `save.script` `load` `load.aliases` `load.lambdas`
  `load.stacks` `load.model` `load.script` `bootstrap`
- the bus: `send` `send.` `send.quick` `send.quick.` `recv` `recv.` `bus.data`
  `bus.data.current`
- `io.banner` `io.banner.`
- data files, **by D119 and not by the reference**: `csv` `csv.` `sqlite`

**What `--noio` leaves ungated — rewritten 2026-10-08, the sixth review's
B2.** Everything else. Until that day this paragraph named `args`,
`sleep.seconds` and `io.graph`, which are the ungated words *of one module*
(`crates/bund2-stdlib/src/host.rs`, module documentation) presented as the
flag's whole surface. Of the 551 names the flag does not touch, these reach
outside the program, and each was among the ones measured:

- **Two read a file the program names, and no longer do under the flag —
  D119.** `csv` and `sqlite` open a path and hand its rows to a lambda. The
  reference registers both with no gate
  (`reference/Bund/src/stdlib/functions/conditional/mod.rs:42-43`) and opens
  the file at `conditional_csv.rs:62` and `conditional_sqlite.rs:38`. Measured
  before the ruling, on a bundle built `--noio --noeval`:
  `"…/t.csv" csv :lambda { println } set !` printed the file's rows. **The
  owner ruled that `--noio` gates both**, an approved deviation. The words and
  their conditional handlers are stubs now, the handlers because
  `conditional :type "csv" set …` reaches the same read without the word.
  `noio_reaches_the_two_words_that_read_a_data_file`.
- **One writes a file.** `debug.shell` saves its line history. Measured on
  the same kind of bundle: one typed line wrote
  `bund2/bund_debug_shell_history.txt` under the configuration directory of
  whoever ran it (`crates/bund2-stdlib/src/terminal.rs`, `history_path`). The
  reference writes the same file into the working directory
  (`reference/Bund/src/stdlib/functions/debug_fun/debug_shell.rs:20`, `:52`)
  and gates it no more than Bund2 does. `debug` keeps one of its own,
  `bund_debug_debugger_history.txt`, by the same route — read from the code
  and not measured.
- **Standard input**: `input` `input*` `password` `bund.prompt`
  (`reference/Bund/src/stdlib/functions/io/input.rs:142-145`), and `debug`
  and `debug.shell`.
- **The host's identity**: `system.ip` `system.ipv6`
  (`reference/Bund/src/stdlib/functions/system/ip.rs:34-35`), `system.locale`
  (`system/locale.rs:27`), `sysinfo.hostname` `sysinfo.kernel_version`
  `sysinfo.os_version` (`sysinfo/host.rs:66-68`), `sysinfo.system`
  `sysinfo.version` `version` `sysinfo.virtualization`
  `sysinfo.virtualization?`, the twelve `sysinfo.mem.*` words, and
  `debug.display_hostinfo` `debug.display_memstat`
  `debug.display_distributed_info`.
- **The command line**: `args` `args.parse`.
- **The clock, sleeping and randomness**: `time.now` `time.timestamp`
  `time.timestamp.` `sleep` `sleep.seconds` `id.ulid` `id.uuid`
  `math.random.int` `math.securerandom.int` and the `string.random.*` words.
- **Standard output and standard error**: `print` `println` and their
  workbench forms, `display`, `nl`, `space`, `io.graph` `io.graph.`, the five
  `log.*` words and the `debug.display_*` words.
- **Ending the process**: `bund.exit` `exit`.

**These are the reference's boundary and not defects in the flag** — D79's
reading, applied to the other flag. The string `disabled with --noio` occurs in
sixteen files under `reference/Bund/src/stdlib/functions`, and none of them is
`conditional/`, `io/input.rs`, `system/ip.rs`, `system/locale.rs`, `sysinfo/`
or `debug_fun/`. What "I/O" means to the flag is the filesystem words, the
shell, the world file and the bus. It is not "the program touches nothing
outside itself", and a runner who reads `restrictions --noio` that way is
wrong about standard input, the address of the machine and the history file.
D119 moved two words across that line and left the line where it is.

**One line of this is a bundle's own.** The history file is written on the
machine of whoever *runs* the artefact, by a word its author left in. For a
`script` run that is RFC-0008's business; for a shipped artefact it belongs
here, and nothing switches it off.

**`--noeval` disables the `bund.eval` group of functions — D79 — which is what
its own help text says it does**: `Disable bund.eval group of functions`
(`reference/Bund/src/cmd/mod.rs:142-143`). **The group is six words**:
`bund.eval`, `bund.eval.`, `bund.eval-file` and `bund.eval-file.`
(`reference/Bund/src/stdlib/functions/bund/bund_eval.rs:117-121`), and `use`
and `use.` (`reference/Bund/src/stdlib/functions/bund/bund_use.rs:74-76`).
Bund2 stubs the same six (`crates/bund2-stdlib/src/host.rs`,
`register_noeval_stubs`). **Until 2026-10-08 this document said four**, three
times, citing the function that lists six; the two it left out are the pair
that read a *file* and run it. The behaviour was always right — measured, a
bundle built `--noeval` refuses `"p.bund" bund.eval-file` — and D79, which
carries the same four, has a dated note.

**It is not a claim that a program evaluates nothing**, and this RFC's second
revision wrongly described it as failing to be one. `compile` is registered
unconditionally
(`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:76`). So

    "40 2 +" compile lambda! ! println

prints `42` under `--noeval --noio`. **Verified on both binaries** — Bund2 and
the oracle at `21b40b0` agree. `compile` parses a string to a LIST, `lambda!`
makes it callable
(`reference/Bund/src/stdlib/functions/bund/bund_fun.rs:218`), `!` runs it — it
is an alias of `execute`
(`reference/rust_multistackvm/src/stdlib/create_aliases.rs:5`) — and none of
the three is in the group the flag names. **That is the boundary of what the flag is for, not a shortfall in it**
(D79), and naming it is what D78 requires.

**So the earlier sentence "fetching is gated by `--noeval`, not `--noio`" was
half wrong, and D78 repeats it.** `url`, `url.`, `file` and `file.` are
`--noio` stubs (`reference/Bund/src/stdlib/functions/filesystem/file.rs:92-98`),
so `--noio` *does* gate fetching through them; `use` is gated
by `--noeval`. The accurate statement is that **no single flag stops a program
obtaining text and running it**: under `--noeval` alone, `url` fetches and the
three words above evaluate the result.

**Two more routes run text a program was handed, and neither flag gates
them — added 2026-10-08, from the acceptance review's B4.** `debug` and
`debug.shell` read lines and evaluate each one
(`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:82-89`,
`reference/Bund/src/stdlib/functions/debug_fun/debug_shell.rs:25-29`), and the
reference registers both with no gate (`debug_debug.rs:155`,
`debug_shell.rs:67`). Measured
on a bundle built `--noeval --noio` whose program is `debug.shell`: the typed
line `40 2 + println` prints `42`. A typed `"1" bund.eval` is refused, because
the stub holds wherever the word is typed. This is D79's boundary again: the
flag names a group of words, and these two are not in it.

**A bundle is never given a debugger — D115, 2026-10-08.** In a `script` run
the first arming or moving debugger word attaches a console (D113.5), and
until D115 a bundle whose program called `debug.step` stopped at one. In a
bundle those words now do nothing: no console, no stop, and the tier stays
on. So a stop at a console is not a third route, and a shipped program does
not wait at a prompt because a breakpoint was left in it.
`a_bundles_debugger_words_do_nothing` holds it.

Naming any of this a boundary would mislead exactly where it is most costly:
D76 puts the risk of what an artefact fetches on the person running it, and a
builder who believed either flag prevented remote code from running would be
wrong.

**`--nocolor` is not in this class.** It is presentational — it changes how one
debug word draws its table — so it joins `--stats`, `--no-dump-stack` and
`--raw-values` in the environment-variable channel above, not in the trailer.

**Why a switch and not analysis.** D16 means no build can prove a program never
calls `fs.rm`: the name may be assembled at run time. The restriction has to be
a switch.

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
**The second half of that sentence is not the design — D118** (§B1, "What is built"): the
runtime is the building binary, `--features` is refused, and the record says
what that binary was compiled with — `jit`, `aot`, `async`, and `grok`.
**`grok` was missing from the record until 2026-10-08**, though it is this
section's own example: the feature is `bund2-stdlib`'s, `bund2-cli` does not
declare it, and a `cfg!` there could never see it. It is now asked of the
crate that owns it (`crates/bund2-stdlib/src/lib.rs`, `GROK_BUILT_IN`). Read
from the code; a `grok` build compiles C and was not made.

**`--inspect` on a runtime nobody built from prints no feature set.** The
fields are written by `bund2 build`, so the plain interpreter's are empty, and
an empty feature field used to print as "default features" — for a `jit`
build of `bund2` too. It now prints "unrecorded"
(`an_unbuilt_runtime_claims_no_feature_set`).

**A bundle may carry the JIT, opt-in and never by default — D80.** The default
runtime carries no code generator, and that default is what a target Cranelift
does not support receives; `--features jit` asks for the other one — today by
building `bund2` with `jit`, as above. No
deviation from D10 was needed: that sentence describes what an unsupported
target gets, and the research it cites marks Cranelift "optional (for JIT
tiering)" for this very product
(`docs/research/02-native-binaries.md:38-42`).

**What a JIT bundle is for: programs that run long enough to reach the
threshold.** D74 set §S7's threshold at 1024 entries of one body, and F139
found that **no program in `tests/golden/HERMETIC.txt` compiles a single body
even at 64** — "because they are demonstrations that run once". So a JIT bundle
of a one-shot script **compiles nothing and pays only size**, and the size is
the larger cost: `cranelift-codegen` with its ISLE tables is, per the research,
the single largest contributor to any binary embedding the JIT. Criterion 1 is
that measurement, and it exists only because D80 makes such a bundle
buildable.

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

**Criterion 7 was deferred behind (1) and (2)**, because defining it against
the current lowering would have defined a criterion that cannot be
implemented. **It is no longer deferred**: it was reformulated and met on
2026-09-30 without this section's work, and D83 withdrew the mode. This
section stands as the record of what building it would take.

**§B5 reopens an exclusion RFC-0005 closed this morning.** Criterion 30's two
mirror cases were excluded because "§S7 compiles a body *on* an entry and runs
that entry interpreted", and that row states its own trigger: "it holds only
while §S7 compiles on an entry rather than ahead of one. **If that changes** —
an ahead-of-entry or background compile, which RFC-0006 may want — the mirrors
become writable and this row reopens" (RFC-0005:6725-6739; the passage has
moved since this was written, and `cargo xtask cite` does not check a line
citation from one RFC into another). `--emit=native`
compiles ahead of every entry by definition, so **building this mode reopens
those two cases and they become owed**, not excluded. The trigger fired as
written, which is the argument for writing triggers that way.

**Which targets get `--emit=native` is not answered here.** RFC-0005 names
x86-64, aarch64, s390x and riscv64 as what Cranelift supports; whether Bund2
*ships* AOT for all four — s390x in particular, which nothing in this
repository can test — is a question for whoever takes the §B8 gate, and is
listed in the open questions.

### §B6 — Cross-compilation and the C dependency

D40 is inherited whole: `string.grok` is gated off by default because a C
dependency beneath the artefact would invert D10's escape hatch, and a
cross-compiled `cranelift-object` build inherits the same problem. A target
Cranelift does not support gets `--emit=bundle`, which is why that mode is
mandatory rather than convenient. **Under D118 that target is served by a
default-feature `bund2` built for it**, which needs no compiler where the
bundle is made; producing its bundle from another machine is the deferred
part. *(Until 2026-10-08 this ended "it needs a prebuilt runtime for that
target rather than a compiler on the build host".)*

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

**It has now been taken — criterion 1, 2026-09-30 — and the ordering argument
was right for a reason better than the one given.** `--emit=native`'s case
rested on a size saving nobody had weighed, and weighed it is **12.45% of the
binary**, with Cranelift owning 8.9% of `__text` where `graphitesql` alone owns
13.0%. The research's claim that the code generator is the single largest
contributor is false for Bund2 (ERRATA).

So the phase has to stand on **start-up without warm-up** instead, against
§B5's three blocking items — symbol-addressed slot tables and cells, planning
without a live VM, and deciding which bodies compile when no word is
registered — plus the reopened criterion 30 mirrors. **That trade is the
owner's, and this RFC does not assume it.** Nothing in `--emit=bundle` waits
on the answer.

## Preservation analysis

| behaviour | disposition |
|---|---|
| The reference's object formats | **Not preserved, and nothing can depend on either.** Two producers, `Value::compile` and `compile_to_binary`, both with zero reachable callers. D11. |
| Program meaning under a bundle | **Preserved, with one exception that is a ruling.** The artefact parses and evaluates the same source through the same entry points, and criterion 2 checks it per golden. The exception is the next row. |
| The arming and moving debugger words | **Deliberately changed — D115.** `"a" println debug.step "b" println` stops at a console under `script` and does not in a bundle: there the words do nothing. It is the one place the same source means two things. `a_bundles_debugger_words_do_nothing`. |
| `.id` of a literal | **Preserved, and not checked.** A literal is constructed when the program runs, so its identity is minted per run as its stamp is — the encoded stream would have fixed both at build (D1, D20). Criterion 9 checks the stamp alone; no test compares identities across two runs of one bundle. |
| `.timestamp` of a literal | **Preserved** — literals are constructed at run time, as §B2 requires. This is the row that failed under the encoded stream. |
| The anonymous `( … )` context name | **Preserved** — minted per run. |
| Scalar representation, fragment admission, tier agreement | **Preserved** — scalars stay unboxed, so `TopAreInt` admits and inlining still fires. |
| Diagnostic source locations | **Preserved** — the source is present, so spans exist and D36's reports carry locations. |
| Programs nesting 257–1024 deep | **Preserved under `--emit=bundle`.** The parser's `MAX_NESTING` of 1024 governs, and `MAX_WIRE_DEPTH` does not apply, because nothing is wire-encoded. The first draft claimed preservation while refusing these programs. |
| Word table and name resolver | **Preserved in full.** No tree-shaking; D16. |
| Which words exist | **Changed, and it is D40's gate seen from outside.** The runtime's feature set decides; a missing word fails at run time. Recorded in the trailer (§B4). |
| `args` / `args.parse` | **Preserved.** All of argv reaches the program; the runner consumes nothing (§B3). |
| Exit code | **Preserved** — `vm.exit_requested()`. |
| Diagnostic flags (`--stats`, `--no-dump-stack`, `--raw-values`) | **Deliberately changed.** Not argv flags in a bundle, because they would shadow the program's own arguments; they move to environment variables. |
| `use` | **Preserved exactly** — D76, under D54's scheme set. |
| `--noio`, `--noeval` | **Deliberately available as a build-time floor — D78.** Recorded in the trailer; run time may add either and never remove one. Not a boundary: §B3a names what each gates and what each leaves ungated — for `--noio` that includes one word that writes a file, standard input, and the host's address and name. |
| `csv` and `sqlite` under `--noio` | **Approved deviation — D119.** The reference leaves both ungated; Bund2 stubs them and their conditional handlers. Without the flag, preserved. |
| `--nocolor` | **Preserved as a per-run choice**, in the environment-variable channel with the diagnostic flags. Presentational, not a capability. |
| Diagnostic file name | **Preserved for a path of up to 256 bytes** via the trailer's recorded source path (§B3), without which a bundle's stderr differs by construction and the CEILING moves. **A longer path is changed**: its last 256 bytes are kept, and the build says so. |
| A program larger than 1 MiB | **Changed: it runs under `script` and cannot be bundled.** Refused at build with both numbers named (§B1, criterion 11). The limit is not discoverable before it is hit except by `--inspect`, which prints the capacity. |
| The debugger's history files in a shipped artefact | **New surface, and ungated.** A bundle whose program calls `debug.shell` or `debug` writes line history under the configuration directory of whoever runs it (§B3a). |
| `bund2` itself | **Changed.** Every `bund2` carries the 1 MiB region and reads it before argv, so a damaged region stops the plain interpreter too (§B1). |
| `bund2 build`'s arguments | **Refused when not understood** — `--emit=native`, `--features`, a repeated `--file`, anything unknown (§B3, "The command line"). |
| A syntax error's timing | **Deliberately changed**: found at build rather than at run (§B3). A build that wrote an unparseable program would move the error to whoever ran it. |
| RFC-0005 criterion 30's excluded mirrors | **Reopened by `--emit=native`**, on that row's own stated trigger. Owed once the mode exists, not excluded. |
| What `--noeval` stops | **Preserved exactly — D79.** It disables the `bund.eval` group, six words: `bund.eval`, `bund.eval.`, `bund.eval-file`, `bund.eval-file.`, `use`, `use.`. `compile` is not in the group, so `compile lambda! !` still evaluates, on both binaries. §B3a names the boundary. |
| A damaged or empty region | **New surface, specified.** An empty region is the plain interpreter; four kinds of damage are errors, none a panic (§B1, criterion 12). |
| Bund2's own version | **Recorded in the trailer, and read by `--inspect` only.** The pinned SHAs name the oracle, not the interpreter. No skew can occur, because the runtime is the builder (D118), and the runtime does not check the field. |
| Code signing of the artefact | **Measured, and the design changed — Q40.** Appending runs but can never validate, and re-signing does not repair it, so the payload goes inside a reserved region instead (§B1). |
| Run-time-registered words under `--emit=native` | **Speed only.** No code generator in the image, so they stay interpreted. |
| D10's "embedded IR" description | **Approved deviation — D77.** Source text is embedded instead, read as descriptive; D10's resolution about the toolchain is untouched. |

## Alternatives considered

- **`include_bytes!` into a stub and link it**, as the research describes.
  Rejected on D10: `rustc` links through `cc`. §B1.
- **Encoding the program with the `wire` codec.** Rejected on four measured
  changes in meaning, §B2 — not on taste, and the first draft proposed it.
- **A fresh program container** with unboxed scalars, no stamps and spans
  carried alongside, which D11 licences. **Closed on the measurement** §B2
  records — a ~20 µs saving against three behaviours the parser already gets
  right, one of them D20's unbuilt step (D77).
- **Source text, compressed.** Preserves everything option 1 does and makes the
  program not trivially readable in the artefact. Available whenever a size or
  opacity argument arrives; it is a change in bytes, not in meaning, and needs
  only a pure-Rust dependency to stay inside D10.
- **`--emit=native` first.** The research recommends object-output-first for
  the *lowering spike*; that ordering is moot, since the lowering exists and was
  built JIT-first. What remains is a size saving §B8 shows cannot be weighed
  until a bundle exists, against a cost §B5 shows is larger than assumed.
- **Tree-shaking behind a flag.** Foreclosed by D16, not by this RFC.
- **A front end that evaluates on the main thread.** Rejected on §B3.

## Acceptance criteria

1. **The gate is answered before `--emit=native` is built.** A figure for what
   the code generator costs a bundle runtime. No threshold: the number is an
   input to a decision.

   **Answered 2026-09-30 — and it answers against the premise it was set to
   test.** Release builds of `bund2-cli`, thin LTO and one codegen unit, with
   and without `jit`; Cranelift confirmed present in one and absent from the
   other by symbol inspection, not by trusting the feature flag:

   | | no `jit` | `jit` | delta |
   |---|---|---|---|
   | whole binary | 18,142,096 | 20,401,184 | **+2,259,088 (+12.45%)** |
   | `__text` | 10,261,804 | 11,845,112 | +1,583,308 |
   | `__const` | 2,556,480 | 2,617,408 | +60,928 |

   **`cargo bloat` was not used and is no longer named here.** It is not
   installed, and the figure the decision needs is the *delta*, which needs no
   tool: `size -m` gives the sections and the difference is the cost. What
   `cargo bloat` would have added is attribution, taken instead from `nm`
   symbol addresses with the mangling parsed, 3.4% unattributed:

   | crate's own symbols | share of `__text` |
   |---|---|
   | `graphitesql` | **13.0%** |
   | `core` | 11.0% |
   | `sqlparser` | 7.9% |
   | `cranelift_codegen` | 7.1% |
   | `prqlc` | 5.7% |
   | `redb` | 3.6% |
   | `regalloc2` | 1.2% |
   | **Cranelift and `regalloc2` together** | **8.9%** |

   **The research's argument for Product B does not survive this.** It holds
   that `cranelift-codegen` with its ISLE tables is "the single largest code
   contributor to any binary that embeds the JIT", and makes shedding it
   Product B's strongest case — "a bigger practical win than the arithmetic
   speedup". Measured, it is **8.9% of `__text` and 12.45% of the binary**, and
   `graphitesql` alone is larger. The library dependencies dominate:
   `sqlparser`, `prqlc` and `redb` together are 17.2%, nearly twice Cranelift.
   Recorded in `docs/research/ERRATA.md`.

   **So `--emit=native` must be justified on something other than size** —
   immediate start-up with no warm-up is the remaining argument — or not at
   all. That is the decision this figure was the input to, and it is the
   owner's.
2. **Every bundled golden matches its source run, per golden and not in
   total, in both runtime configurations.** `cargo xtask conform --bundles`
   builds an artefact from each prepared program and executes it with no
   arguments; everything downstream — normalisation, the per-golden
   comparison, the deviations, the CEILING, the recorded baseline — is the code
   the source run uses, so a difference in the numbers is a difference in
   meaning and not in how it was measured.

   **Met, 2026-09-30, in three configurations rather than the two asked for:**

   | run | conformance | ceiling | failing |
   |---|---|---|---|
   | source, default | 107/116 (+9 approved) | 107/116 | none |
   | **bundles, default** | **107/116 (+9)** | **107/116** | **none** |
   | **bundles, `--features jit`** | **107/116 (+9)** | **107/116** | **none** |
   | **bundles, `jit` at threshold 1** | **107/116 (+9)** | **107/116** | **none** |

   **Run again on 2026-10-08: 133/145, ceiling 133/145, none failing**, in
   source mode and for default bundles, and in source mode with `jit` and
   with `jit` at threshold 1. The corpus has grown since the table; the claim
   has not changed.

   Threshold 1 is beyond the criterion and is where it is worth most: every
   body compiles on its first evaluation, so it is the strongest statement the
   corpus can make about compiled code in an artefact preserving meaning.

   **The nine deviations matching is the sharper half of this result.** An
   approved deviation is judged against a recorded hash of Bund2's output, so
   each of those rows says a bundle produced **byte-identical** output, not
   merely a passing comparison.

   A totals comparison would let one new failure hide one new pass; zero
   failures in both modes makes the per-golden sets equal by construction.
3. **A produced artefact contains no code generator.** Not `cargo tree`, which
   passes on today's default `bund2` and so checks nothing about a bundle:
   **`nm`/`strings` over the artefact `bund2 build` actually wrote**, finding
   no Cranelift symbol. Stated against the default configuration, since
   criterion 6 needs a runtime built *with* the JIT — the two are about
   different artefacts, and D80 permits the JIT-carrying one only when asked
   for.

   **Met, 2026-09-30** — `a_default_artefact_contains_no_code_generator`
   (`crates/bund2-cli/tests/bundle_build.rs`), which searches the produced
   artefact's bytes rather than a crate graph.

   **Its companion was missing until 2026-10-08.** Criterion 7 argues for a
   non-vacuity test beside every absence check, and this absence check had
   none: nothing showed the search could find Cranelift where it is.
   `a_jit_artefact_contains_the_code_generator` does, under `--features jit`.
   Both run in debug; a stripped release binary was not searched.
4. **A bundle is produced with no C toolchain and no compiler.** Built in an
   environment with no `cc` **and no `rustc`**, which §B1 makes possible and is
   a stronger check than reading a dependency list.

   **Met, 2026-09-30, and more strongly than written** —
   `a_bundle_is_built_with_no_compiler_reachable` runs the build with the
   environment **cleared and `PATH` empty**, so nothing is reachable by name,
   and then runs the artefact the same way. It also pins D81's *shape*:
   `/usr/bin/codesign` is invoked by absolute path, so an empty `PATH` cannot
   reach it and cannot hide a dependency on it either. **Verified
   load-bearing** — changed to `Command::new("codesign")`, the test fails with
   "could not be signed … No such file or directory".
5. **A program nesting up to `MAX_NESTING` bundles and runs.** 1024 levels, not
   256: the wire cap does not apply, and a criterion set at 256 would pin the
   defect the first draft had.

   **Met, 2026-09-30** — `a_program_nested_to_the_parsers_limit_bundles_and_runs`,
   which also checks 1025 levels fails the *build* and writes nothing.
6. **The front end declares its Tier 1 share.** A bundle built with `jit`,
   running a body past the threshold, reports `compiled_entries() > 0` — the
   figure a broken front end leaves at zero while every other figure reports
   success.

   **Met, 2026-09-30** — `a_jit_bundle_enters_compiled_code`. Forty calls at
   threshold 2 report **1 body compiled, 38 entered**, which is exactly the
   counter's semantics: one warm-up, one compiling entry that runs interpreted,
   38 compiled. At a threshold nothing reaches it reads 0 and 0, so the
   assertion is load-bearing. Checked in release with `jit` too, by hand:
   the same 38.
7. **Met, 2026-09-30, and reformulated — without §B5's work.** RFC-0005's
   criterion 4 is discharged by
   `no_relocation_names_a_compiled_body_from_inside_another`
   (`crates/bund2-jit/src/lower.rs`), which emits two bodies into an
   `ObjectModule` and reads the relocation records `cranelift-jit` consumes.
   **§B5's items were not needed**: the lowering bakes heap addresses as
   *immediates*, and an immediate is not a relocation, so an object that is
   read rather than run answers the question. Making `Emitter` generic over
   `cranelift_module::Module` was the whole prerequisite.

   **The first version of this test was vacuous and its companion caught it.**
   It attributed relocations to functions by section name, relying on
   `per_function_section`; on Mach-O every relocation reports `__text`, so no
   offender could ever match. Containment is decided by symbol address now, and
   `the_permitted_relocations_are_present` is what exposed the first version —
   which is the argument for writing a non-vacuity test beside every absence
   check. That companion also pins the permission *by index*: a trampoline may
   call only **its own** body, since `entry_0 -> body_1` would be the same
   defect wearing a permitted shape.

   The original wording was: **"No relocation targeting a function" is the
   wrong test for an object file**: the lowering declares its runtime helpers
   `Linkage::Import` — `jit_pop_int`, `jit_push_int`, `jit_dup_top`,
   `jit_drop_top`, the admits adapter and more — and references them through
   `declare_func_in_func` (`crates/bund2-jit/src/lower.rs`, `emit_into`), so
   every call site is a relocation naming a function and the criterion could
   never pass. The property criterion 4 exists to protect is narrower, and RFC-0005 already
   worded it correctly: **no relocation names a compiled body's `FuncId` from
   inside another body**. The second revision of this RFC dropped "from inside
   another body", which forbade §S8's own entry trampoline — it reaches its
   body through `declare_func_in_func` (`crates/bund2-jit/src/lower.rs`,
   `emit_into`'s trampoline). Permitted: imports of runtime symbols, a native's
   `Tail` thunk calling its adapter, and the trampoline calling the body it
   enters. Forbidden: one compiled body naming another.
8. **`--emit=native` matches Tier 0's conformance exactly.** RFC-0005's
   criterion 2 applied to this mode: the same N/M and CEILING as the
   interpreter, per golden. Missing from the first draft entirely.

   **Withdrawn with the mode — D83.** `conform --bundles` is the instrument it
   would have used, and it exists; the mode does not. If `--emit=native` is
   ever revived this criterion revives with it, unchanged.
9. **A literal's stamp is a run-time value.** The same bundle run twice
   reports different stamps for the same literal, and neither is the build
   time. This is B2.1 made checkable rather than argued, and it is the
   criterion that pins §B2's reason for existing: an encoded payload
   materialises every stamp at build time (D20), so every run would report the
   same one.

   **Met, 2026-09-30** — `a_literals_stamp_is_taken_at_run_time_not_at_build_time`.
   Observed through `debug.display_stack`, because **`.timestamp` is not among
   the words Bund2 implements yet**; the stamp is sampled when a value is first
   observed, which is D2's ruling, so the dump *is* the observation. Each run's
   stamp must fall inside that run's own wall-clock window and be later than
   the build, and the two runs must differ. **Load-bearing by construction**: a
   frozen build-time stamp cannot lie inside two disjoint windows.
10. **A restriction cannot be loosened at run time.** A bundle built `--noeval`
    still refuses each of the group's six words, and one built `--noio` still
    refuses an I/O word, with `BUND2_NOEVAL` and `BUND2_NOIO` each set to `0`
    and to empty — the two values `env_set` reads as "unset" (§B3) — and
    `--inspect` still reports the restriction. *(Until 2026-10-08 this read
    "every environment variable … set to every value that would clear them",
    which no test can exhaust, and named three of the six words.)* D78's direction made
    checkable, because this is the failure that would otherwise be silent.

    **Met, 2026-09-30, both halves** —
    `a_restriction_cannot_be_cleared_by_the_environment`: built `--noeval` it
    refuses with `BUND2_NOEVAL` set to `0` and to empty, and an unrestricted
    artefact still accepts a restriction added at run time. The `--inspect`
    half, recorded as unbuilt earlier the same day, is
    `an_artefact_describes_itself` and `an_unrestricted_artefact_says_so`.

    **Widened 2026-10-08** — that test tried `bund.eval` alone and `--noeval`
    alone. `both_floors_hold_for_every_word_they_name` tries all six words,
    `bund.eval-file` and its workbench form among them, and tries `--noio`'s
    floor with `fs.cwd`, in both directions.

    **It checks the stubs and nothing more, deliberately.** A criterion that
    claimed more would be false: `"40 2 +" compile lambda! !` prints `42` under
    `--noeval` on both binaries (§B3a), so no criterion here may be read as
    "the artefact evaluates nothing".
11. **A produced artefact still validates where signing is enforced.**
    `codesign -v` on what `bund2 build` wrote reports no error on macOS, after
    an ad-hoc re-sign, which Q40's measurements show is possible for an
    in-image payload and impossible for an appended one. A program over the
    reserved capacity is refused at build with the capacity and the size named.

    **Met, 2026-09-30** — `a_built_artefact_validates`, plus
    `a_program_over_capacity_is_refused_and_writes_nothing`, which asserts the
    error names both numbers **and that the image is unchanged**.
12. **A damaged artefact is refused, not aborted.** All four of §B1's kinds of
    damage — an unknown container version, a state byte that is neither value,
    a length past capacity, a payload that is not UTF-8 — produce a diagnostic
    and an error status, and none reaches a panic; an empty region is the
    plain interpreter. *(Until 2026-10-08 this named the appended form's
    cases, one of which — a truncated trailer — cannot occur in a fixed
    region.)* D37,
    and the one input a bundle's front end will certainly meet.

    **Met, 2026-09-30** — `a_damaged_artefact_is_refused_with_an_explanation`
    drives three through the binary: a `state` byte of 7, a length past
    capacity, and a lone continuation byte in the payload. The fourth, an
    absent region, is `an_unbuilt_runtime_carries_nothing` — a runtime nobody
    built from is the plain interpreter, not a failure. Each damaged artefact
    is **re-signed before it is run**, or macOS kills it before the runtime can
    report and the test would pass for the wrong reason. **The container
    version was driven through the binary by no test until 2026-10-08**; it
    is the fourth case of the same test now.
13. **Conformance moves by exactly zero.** This RFC changes what Bund2 emits,
    not what a program means.

    **Met, 2026-09-30.** `cargo xtask conform` in source mode read **107/116,
    ceiling 107/116** before any of this work and reads the same after it —
    across the container, the builder, the carried-program front end, D78's
    floor, `conform --bundles` and the two release-only fixes. The number the
    JIT milestones had to leave alone, this one leaves alone too.

    **This is a dated comparison and not a standing check.** The recorded
    baseline is 106, so `conform` would not report a loss of up to 27 goldens
    as a regression; the CEILING line is what holds the number today.

**What runs these, stated because five of them do not run by themselves — the
sixth review.** Read from `.github/workflows/ci.yml`, which runs
`cargo test --workspace` and `cargo xtask conform` with default features on
Ubuntu, and `cargo check`s `jit` and `aot`:

| criterion | what checks it | does CI run it |
|---|---|---|
| 1 | a dated measurement; no script re-takes it | no |
| 2 | `cargo xtask conform --bundles` | **no** |
| 3 | `bundle_build`, debug | yes; its companion needs `jit` and does not |
| 4, 5, 9, 10, 12 | `bundle_build` | yes |
| 6 | `bundle_build` under `--features jit` | **no** |
| 7 | `bund2-jit`'s relocation tests, under `aot` | **no** |
| 11 | `bundle_build`; the signing half is macOS only | the capacity half only |
| 13 | a before-and-after `conform` on 2026-09-30 | no |

`cargo xtask bundle`, which found both release-only blockers, is in neither
`cargo test` nor CI — D82's fourth unsettled item, still unsettled. Each "no"
was last run by hand on the date its criterion gives.

## Open questions

- **The §B2 deviation — answered by D77**, 2026-09-29: embed source text, and
  the encoded container is closed rather than deferred. Left listed so a reader
  tracing D77 finds the question it answers.
- **Q38 — answered by D76**, 2026-09-29: fetch at run time.
- **A flag that embeds `use` targets.** Deferred by D76 with its trigger and
  form recorded. Nothing is blocked on it and no default waits to be adopted.

- **What a bundle may switch off — answered by D78**, 2026-09-29: a
  build-time floor in the trailer that run time may only tighten, and the word
  "sandbox" is ruled out because `--noio` leaves `args`, `sleep.seconds` and
  `io.graph` ungated and does not gate fetching at all.
- **May a bundle carry the JIT — answered by D80**, 2026-09-29: yes, opt-in and
  never by default. **No deviation from D10 was needed**, which the third
  review had assumed: that sentence describes what an unsupported target gets.
- **What `--noeval` means — answered by D79**, 2026-09-29: the `bund.eval`
  group, as its help text says. Behaviour unchanged; §B3a's framing corrected.
- **Is parse-at-build a decision or a design call? — answered by D116**,
  2026-10-08: a decision, and ratified as built. §B3 has `bund2 build` refuse
  a program that does not parse, which moves when a syntax error is found and
  who finds it. The preservation table's "deliberately changed" now has D116
  behind it.
- **Q40 — answered by measurement**, 2026-09-30, then corrected by building it:
  appending runs but can never validate; **in place does not run at all until
  re-signed**, and then does both. §B1 carries the table. Two limits remain
  unmeasured and are stated there: a Developer-ID notarised binary, and the ELF
  and PE cases.
- **Invoking `codesign` — answered by D81**, 2026-09-30: permitted. It is a
  base-system binary, not the C toolchain D10 forbids, and without it a macOS
  artefact cannot execute. D81 carries the one unverified limit.
- **Which targets get `--emit=native`.** s390x in particular is untestable
  here. For whoever takes §B8's gate.
- **Is the runtime the builder, or a prebuilt runtime per target? — answered
  by D118**, 2026-10-08: the builder. Raised the same day by the sixth review
  (B4). Cross-target bundling is deferred with its trigger recorded in §B1.
- **Should `--noio` gate `csv`, `sqlite` and the debugger's history file? —
  answered by D119**, 2026-10-08, for the first two: yes, an approved
  deviation. The ruling does not name the history file, so it stays ungated
  as the reference leaves it, and §B3a says so.

**Three reviews have found six blockers between them, and all six are
answered** — four in the design, the rest by D76, D77, D78, D79 and D80. What
remains listed is one question for whoever takes §B8's gate. The sixth review
on 2026-10-08 raised two more for the owner and both are ruled: which
construction §B1 means is D118, and how far `--noio` reaches is D119. (Until
2026-10-08 this sentence also counted parse-at-build, since ruled as D116,
and Q40, which the bullet above records as answered by measurement.) **No
default is being adopted by omission** — stated carefully, because the second
revision of this section claimed exactly that while two blockers were
outstanding.
