# RFC-0006: Ahead-of-time output — `bund2 build`

- Status: **Draft**, and `--emit=bundle` is **built and reviewed** as of
  2026-09-30. The implementation review
  (`docs/rfc/reviews/RFC-0006-implementation-review-2026-09-30.md`) found two
  release-only blockers that eleven passing tests had not: the payload region's
  sentinel was unique only in debug, and the runtime read it from an immutable
  `static` that release folded to its initialiser. Both fixed;
  `cargo xtask bundle` now checks both profiles, which is the gap that let them
  through. Built —
  `crates/bund2-cli/src/bundle.rs` and `bund2 build`. Criteria 3, 5, 6, 10, 11
  and 12 pass, **criterion 2 is met in three configurations**, and
  **criterion 1's gate is answered — against Product B's premise**. 13 follows
  from criterion 2's runs; 7 is deferred behind §B5.
  Revised 2026-09-29 after the first adversarial review
  (`docs/rfc/reviews/RFC-0006-review-2026-09-29.md`). The review raised three
  blockers; all three were reproduced against the code before this revision,
  and one is worse than it reported. §B2's deviation is **ruled on — D77**. A
  **second review** found one blocker, ruled on as D78. A **third** found two
  more, both now ruled on: what `--noeval` means (**D79**) and whether a bundle
  may carry the JIT (**D80**).
- Depends on: RFC-0003 (the program stream and Tier 0), RFC-0005 (the
  Cranelift tier)
- Decisions consumed: D10, D11, D16, D20, D40, D44, D54, D74, D76, D77, D78, D79,
  D80, D81, and
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

**Neither is reachable from Rust, but the bytes are reachable from Bund**, and
the first revision of this RFC got that wrong by repeating D11's own gap.
`encode.base64` pulls a value and calls **`to_binary`** on it
(`reference/Bund/src/stdlib/functions/encoding/base64.rs:33`) — the same
serialiser — so `"1 2 +" compile encode.base64` yields the base64 of exactly
the bytes `compile_to_binary` would write. D11 is amended accordingly — **twice**, because the first amendment was also
wrong. Those bytes do not "reach only a Bund string": `save.*` writes **every
lambda** to the world file with `to_binary`, which is exactly
`Value::compile`'s format, into a SQLite table on disk
(`reference/Bund/src/stdlib/helpers/world/lambdas.rs:80-95`). So the format
does reach a file, and whether anything outside Bund2 reads that file is
**D31**, which is OPEN. D11's answer is therefore not independent of D31 the
way D11's split assumed; this RFC does not resolve either and does not rest on
them, since §B2 embeds source text and uses neither format.

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

**What the runtime does when the trailer is not there, or is wrong.** D37
forbids a panic, and a bundle runtime is the same binary as the interpreter, so
this path is reached by anyone who runs the prebuilt runtime directly. Four
cases, all of them errors and none of them aborts:

- **No trailer** — the magic is absent. The runtime behaves as `bund2` does
  with no program: it is the plain interpreter, not a failure.
- **Magic present, trailer truncated**, so the length or the SHAs cannot be
  read: refused with a diagnostic naming the artefact as damaged.
- **Length implausible** — beyond the file, or overlapping the trailer:
  refused the same way, and the length is checked against the file's size
  before any read.
- **Payload unreadable as text.** The payload is source (§B2), so this is a
  UTF-8 failure and is refused before the parser sees it.

Criterion 12 checks all four, because a corrupted artefact is the one input a
bundle's front end is guaranteed to meet eventually and the only one that could
reach for an `unwrap`.

**The trailer records Bund2's own version, not only `reference/`'s SHAs.** A
builder and a prebuilt runtime can be different Bund2 builds — the pinned SHAs
say which oracle the *meaning* was fixed against and say nothing about the code
doing the interpreting. Without Bund2's version in the trailer, a mismatch is
undetectable; with it, the runtime can refuse or warn on one it does not
recognise.

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
everything on a spawned thread sized `TIER0_PART + TIER1_SHARE +
STACK_RESERVE`, then declares the region with `set_stack_region_with_share`
(`crates/bund2-cli/src/main.rs`). The reason is recorded at the call:

> A share is declared, never inferred — RFC-0005 §S8, the ninth review's S3.
> Declaring through `set_stack_region` would leave the share at zero, putting
> the Tier 1 floor above the thread's top so that every compiled body declines:
> criterion 2 would then pass with no compiled code having run at all.

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

**Four things the artefact's front end must settle, and does here:**

- **argv.** The runner consumes nothing. A bundle is the program, so all of
  argv reaches `args` and `args.parse` (`crates/bund2-stdlib/src/host.rs`,
  `script_args`). The CLI's own flags — `--stats`, `--dump-stack`,
  `--raw-values`, `--jit-threshold` — are **not** available in a bundle,
  because a bundle that swallowed `--stats` would shadow a program's own
  argument. They move to environment variables, and the names are part of this
  design rather than left to the implementation: `BUND2_STATS`,
  `BUND2_DUMP_STACK`, `BUND2_RAW_VALUES`, and **`BUND2_JIT_THRESHOLD`, which
  the interpreter already reads** (`crates/bund2-runtime/src/lib.rs`,
  `threshold_from_env`). Criteria 2, 6 and 10 name which of these they set.
- **The exit code** is `vm.exit_requested()`, as the CLI returns
  (`crates/bund2-cli/src/main.rs`, `run_cli`).
- **The reporter** is the CLI's `TextReporter`, with the same `wants_stack`
  policy, so a fatal report carries a stack snapshot and a warning does not
  (D45, `crates/bund2-stdlib/src/report.rs`).
- **One program.** `bund2 build` takes exactly one source file. Not a
  directory, not several files, not standard input; a program that needs more
  reaches them through `use`, which D76 settles.
- **`bund2 build` parses before it writes.** A syntax error is a build error,
  reported with its span against the source it was given, and the parse result
  is then discarded — the artefact re-parses at start-up, which is what keeps
  §B2's preservation exact. Refusing at build costs one parse, measured at
  ~4.3 µs for 891 bytes, and is the difference between a broken program found
  by whoever built it and one found by whoever ran it.
- **The diagnostic file name comes from the trailer.** A bundle has no file on
  disk to name, and `Diagnostic`'s location carries one. `bund2 build` records
  the source path it was given, verbatim, and the artefact reports locations
  against it — so a bundle's stderr matches a `script` run of the same program
  from the same path. Without this, criterion 2 would compare stderr that
  differs by construction, and one approved deviation's recorded hash pins a
  path Bund2 prints, so the CEILING would move.

### §B3a — What a bundle may switch off, and what that does not mean

**`--noio` and `--noeval` may be recorded in the trailer, and an environment
variable at start-up may add either and never remove one** — D78. Both are
`HostOptions` fields: `--noio` registers the I/O words as stubs that fail, and
`--noeval` does the same to `bund.eval`, `use` and `use.`
(`crates/bund2-stdlib/src/host.rs`, `HostOptions` and `register_noeval_stubs`;
from the reference's own command line,
`reference/Bund/src/cmd/mod.rs:139-146`).

**The direction is the design.** A restriction an environment variable could
switch off would not be a restriction, and the failure would be silent — the
artefact would still report itself as built `--noeval` while evaluating
everything. So the trailer is a **floor**: run time may tighten, never loosen.

**The trailer is readable**, because a restriction the runner cannot observe is
one they cannot rely on. `bund2 build --inspect <artefact>` prints the
features, the restrictions and the pinned SHAs.

**These are word-group switches, and this RFC does not call them a sandbox.**
D78 rules the word out. What follows is what each flag leaves ungated, by name,
which D78 requires and the second revision of this RFC gave for `--noio` only.

**`--noio` leaves ungated:** `args`, `sleep.seconds` and `io.graph` — no gate
in the reference and none here (`crates/bund2-stdlib/src/host.rs`, module
documentation).

**`--noeval` disables the `bund.eval` group of functions — D79 — which is what
its own help text says it does**: `Disable bund.eval group of functions`
(`reference/Bund/src/cmd/mod.rs:141-142`). The group is `bund.eval`,
`bund.eval.`, `use` and `use.` (`crates/bund2-stdlib/src/host.rs`,
`register_noeval_stubs`).

**It is not a claim that a program evaluates nothing**, and this RFC's second
revision wrongly described it as failing to be one. `compile` is registered
unconditionally
(`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:76`). So

    "40 2 +" compile lambda! ! println

prints `42` under `--noeval --noio`. **Verified on both binaries** — Bund2 and
the oracle at `21b40b0` agree. `compile` parses a string to a LIST, `lambda!`
makes it callable, `!` runs it, and none of the three is in the group the flag
names. **That is the boundary of what the flag is for, not a shortfall in it**
(D79), and naming it is what D78 requires.

**So the earlier sentence "fetching is gated by `--noeval`, not `--noio`" was
half wrong, and D78 repeats it.** `url`, `url.`, `file` and `file.` are
`--noio` stubs, so `--noio` *does* gate fetching through them; `use` is gated
by `--noeval`. The accurate statement is that **no single flag stops a program
obtaining text and running it**: under `--noeval` alone, `url` fetches and the
three words above evaluate the result.

Naming any of this a boundary would mislead exactly where it is most costly:
D76 puts the risk of what an artefact fetches on the person running it, and a
builder who believed either flag prevented remote code from running would be
wrong.

**`--nocolor` is not in this class.** It is presentational — it changes how one
debug word draws its table — so it joins `--stats`, `--dump-stack` and
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

**A bundle may carry the JIT, opt-in and never by default — D80.** The default
runtime carries no code generator, and that default is what a target Cranelift
does not support receives; `--features jit` asks for the other one. No
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

**Criterion 7 is deferred behind (1) and (2).** Defining it against the current
lowering would define a criterion that cannot be implemented.

**§B5 reopens an exclusion RFC-0005 closed this morning.** Criterion 30's two
mirror cases were excluded because "§S7 compiles a body *on* an entry and runs
that entry interpreted", and that row states its own trigger: "it holds only
while §S7 compiles on an entry rather than ahead of one. **If that changes** —
an ahead-of-entry or background compile, which RFC-0006 may want — the mirrors
become writable and this row reopens" (RFC-0005:6548-6561). `--emit=native`
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
| `--noio`, `--noeval` | **Deliberately available as a build-time floor — D78.** Recorded in the trailer; run time may add either and never remove one. Not a boundary: §B3a names what each leaves ungated. |
| `--nocolor` | **Preserved as a per-run choice**, in the environment-variable channel with the diagnostic flags. Presentational, not a capability. |
| Diagnostic file name | **Preserved** via the trailer's recorded source path (§B3), without which a bundle's stderr differs by construction and the CEILING moves. |
| A syntax error's timing | **Deliberately changed**: found at build rather than at run (§B3). A build that wrote an unparseable program would move the error to whoever ran it. |
| RFC-0005 criterion 30's excluded mirrors | **Reopened by `--emit=native`**, on that row's own stated trigger. Owed once the mode exists, not excluded. |
| What `--noeval` stops | **Preserved exactly — D79.** It disables the `bund.eval` group: `bund.eval`, `bund.eval.`, `use`, `use.`. `compile` is not in the group, so `compile lambda! !` still evaluates, on both binaries. §B3a names the boundary. |
| A damaged or absent trailer | **New surface, specified.** Four cases, all errors, none a panic (§B1, criterion 11). |
| Bund2's own version | **Recorded in the trailer.** The pinned SHAs name the oracle, not the interpreter, so a builder/runtime skew would otherwise be undetectable. |
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
7. **Deferred, and reformulated.** RFC-0005's criterion 4 is discharged here
   once §B5's (1) and (2) exist. **"No relocation targeting a function" is the
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
9. **A literal's `.timestamp` is a run-time value.** The same bundle run twice
   reports different stamps for the same literal, and neither is the build
   time. This is B2.1 made checkable rather than argued.
10. **A restriction cannot be loosened at run time.** A bundle built `--noeval`
    still refuses `bund.eval`, `use` and `use.` with every environment
    variable the start-up path reads set to every value that would clear them,
    and `--inspect` still reports the restriction. D78's direction made
    checkable, because this is the failure that would otherwise be silent.

    **It checks the stubs and nothing more, deliberately.** A criterion that
    claimed more would be false: `"40 2 +" compile lambda! !` prints `42` under
    `--noeval` on both binaries (§B3a), so no criterion here may be read as
    "the artefact evaluates nothing".
11. **A produced artefact still validates where signing is enforced.**
    `codesign -v` on what `bund2 build` wrote reports no error on macOS, after
    an ad-hoc re-sign, which Q40's measurements show is possible for an
    in-image payload and impossible for an appended one. A program over the
    reserved capacity is refused at build with the capacity and the size named.
12. **A damaged artefact is refused, not aborted.** All four of §B1's cases —
    absent magic, truncated trailer, implausible length, non-UTF-8 payload —
    produce a diagnostic and an error status, and none reaches a panic. D37,
    and the one input a bundle's front end will certainly meet.
13. **Conformance moves by exactly zero.** This RFC changes what Bund2 emits,
    not what a program means.

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
- **Is parse-at-build a decision or a design call?** §B3 has `bund2 build`
  refuse a program that does not parse, and the preservation table files it as
  "deliberately changed" with nothing behind it. It moves when a syntax error
  is found, which is observable. Flagged rather than assumed.
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

**Three reviews have found six blockers between them, and all six are
answered** — four in the design, the rest by D76, D77, D78, D79 and D80. What
remains listed is one design call flagged for confirmation (parse-at-build),
one ungrounded assumption (Q40), and one question for whoever takes §B8's
gate. **No decision waits and no default is being adopted by omission** —
stated carefully, because the second revision of this section claimed exactly
that while two blockers were outstanding.
