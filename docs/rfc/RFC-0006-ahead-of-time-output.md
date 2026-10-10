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
  `crates/bund2-cli/src/bundle.rs` and `bund2 build`. Criteria 3, 4, 5, 6, 9, 10,
  11 and 12 pass *(4 and 9 were missing from this line until the eleventh
  review; the list below has marked both met since 2026-09-30)*, **criterion 2 is met in three configurations**, and
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
  **A seventh review the same day**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-08-2.md`) found one blocker: a
  bundle built `--noeval` evaluated a string through `debug.run`, Bund2's own
  word, which no list here named. **Ruled on — D120**: the flag gates it.
  **D121**, the same day: it gates `debug.feed` too, which handed
  `debug.shell` a line nobody typed.
  The review's other findings are answered where they apply, each dated.
  **An eighth review the same day**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-08-3.md`) found one blocker: the
  reference's `debug` evaluates its string operand before it reads a line, so
  a bundle built `--noeval --noio` runs `"40 2 + println" debug` with nobody
  at the keyboard, and §B3a had described the word as one that reads lines.
  **Ruled on — D122**: `debug` is left as the reference has it, and §B3a
  names it as what it is. Gating it would have been a deviation, as D119 was.
  The review's S3, a breakpoint condition that ran under neither restriction,
  is a defect and is fixed — **F180**. Its other findings are answered where
  they apply, each dated.
  **A ninth review the same day**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-08-4.md`) found one blocker:
  `decode.base64` turns a string into a lambda and `!` runs it, under both
  flags, and no list here named the word — the search that closed the
  `--noeval` list looked for places a string is parsed, and this route parses
  nothing. **§B3a now names it and searches for both mechanisms. Ruled on —
  D123**: D122's "leave it and name it" covers this word too. Its S1, a
  build that destroyed the builder through a hard link, is a defect and is
  fixed — **F181**.
  **A tenth review the same day**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-08-5.md`) found one blocker: the
  "third mechanism" §B3a hedged about exists, and D79 had already said so. A
  program builds code from strings — `make.call`, `ptr`, `lambda*`,
  `lambda!`, and `!` applied to a string — with no parser and no decoder.
  **§B3a now names it and says the list of routes is open by construction;
  no gate and no new ruling, D79's own reasoning being this finding.** Its
  S2, a build that replaced its own source, is fixed with F181's other loose
  ends.
  **An eleventh review, 2026-10-09**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-09.md`) found one blocker: `use`
  is not "preserved exactly". Bund2's fetch obeys proxy variables the
  reference ignores and asks a proxy for a tunnel where the reference asks
  for the URL, and `use` evaluates what comes back. **Ruled a defect the same
  day and fixed, F182: the fetch now chooses a proxy as libcurl chooses it
  for the reference, measured against the oracle case by case (§B7). Two
  differences remain that `ureq` cannot close — the tunnel, and a proxy that
  is not plain HTTP — and D124 approves both as deviations.**
  **A twelfth review, 2026-10-09**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-09-2.md`) found one blocker:
  F182's fix left a third difference, and one it could close. A proxy that
  names no port was asked on port 80 where the reference asks on 1080.
  **Fixed under F182 the same day, with the review's S1 — a URL's host in a
  short spelling — and its S2, a scheme in upper case, which is F183. The
  settings measured are now in the repository, 175 of them
  (`docs/measurements/fetch-2026-10-09.md`), and §B7 no longer says the
  proxy is chosen "as libcurl chooses it" without saying for which
  settings. Measuring found one more difference, a `file:` URL with a literal space,
  which Bund2 reads and the reference refuses: approved the same day,
  D125.**
  **A thirteenth review, 2026-10-09**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-09-3.md`) found two blockers.
  The first was the twelfth's again, one step on: a port that is written and
  is not one — `:65536` — was read downstream as no port, so Bund2 fetched
  from port 80 a URL the reference refuses. **Fixed under F182: Bund2 reads
  the port as a number and writes it itself, in the URL and in the proxy.**
  The second is that the reference fetches schemes Bund2 refuses —
  `gopher://`, `dict://`, `ftp://` through a proxy — and nothing approves
  refusing them. **Approved the same day, D126: the sixth deviation for the
  fetch. The spellings of an `http:` URL the reference takes and Bund2
  refuses are approved as a rule, D127, the seventh. §B7 no
  longer says the rows of the measurement "agree": it says in which rows
  Bund2 refuses, and that in none does Bund2 ask a listener the reference
  does not. The measurement is 278 settings and records every connection.**
  **A fourteenth review, 2026-10-09**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-09-4.md`) found three blockers,
  all the thirteenth's first again with the field moved: the port was read,
  and the host, the user part, a proxy's credentials, a `no_proxy` entry's
  digits and a `file:` path's dot segments were handed on as text. So with a
  proxy set Bund2 fetched a URL the reference refuses; for some settings the
  two asked different listeners; and through a link `file` read a different
  file. **Fixed under F182 and F183: each of those parts is read in
  `host.rs` as libcurl reads it, and §B7 now lists every part of the string
  and who reads it. The measurement is 443 settings and records
  credentials. Two questions went to the owner and were ruled the same day:
  a proxy whose credentials carry a `%xx` escape fails the fetch, where the
  reference either decodes them or, for a control byte, fetches with no
  proxy at all (D128, F185); and two responses `ureq` will not read fail
  it too (D129). They are the eighth and ninth approved deviations for the
  fetch.** *(Until
  this revision the paragraph on the thirteenth review stood before the one
  on the twelfth.)*
  **A fifteenth review, 2026-10-09**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-09-5.md`) found one blocker,
  and **a sixteenth the same day**
  (`docs/rfc/reviews/RFC-0006-review-2026-10-09-6.md`), a second reading of
  the same text, reproduced it and found one more. The first: the string
  was read and the answer was not. §B7 said the response was "all `ureq`'s"
  and that the two clients decide two shapes differently. Of 94 shapes
  they decided 33 differently, and in sixteen Bund2 did not fail closed: a
  redirect with no length was the empty string, and a body the reference's
  fetch refuses was evaluated. **Ruled the same day, D131: Bund2 writes the
  request and reads the response itself, by rules measured against the
  oracle on 233 shapes, and `ureq` is gone. That ends three approved
  deviations — the tunnel, which was half of D124; a proxy's credentials
  with an escape, D128; and the responses `ureq` would not read, D129 —
  and narrows a fourth, D127. The defect is F187.** The second: under
  `--noio` alone `use`, `use.`, `bund.eval-file` and `bund.eval-file.` read
  a file or fetch a URL and run it, §B3a's list for the flag had lost that,
  and D119 had been ruled on a survey that missed the four. **Ruled the
  same day, D132: `--noio` gates them, an approved deviation.** The
  measurement is 741 settings, on macOS. *(Until 2026-10-10 this said
  Linux had not been measured since D131.)* **Linux was measured on
  2026-10-10, the same 741: 16 rows differ there that agree on macOS,
  because the reference links another libcurl, and D133 approves them.**
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
  D80, D81, D82, D83, D115, D116, D118, D119, D120, D121, D122, D123, D124, D125, D126, D127,
  D128, D129, D130, D131, D132, D133, and
  decisions.md's "What this forecloses" clause on tree-shaking. *(Until
  2026-10-08 this list named D44, which the body never uses, and omitted the
  last four, which it rests on.)*
- Touched but not consumed: D1, D2, D28, D31, D34, D36, D37, D45, D87, D113 —
  see the preservation table for D1, D2 and D36; the others are cited where
  used
- Reference SHA: `reference/Bund` at `21b40b0`, `rust_dynamic` at `ceb27c9`,
  `bund_language_parser` at `8037772`, `rust_multistackvm` at `4605832`, per
  `reference/PINNED.txt`
- Supersedes: `docs/research/02-native-binaries.md` §1's construction and
  §9's phasing, both recorded in `docs/research/ERRATA.md`

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
  a linker, not `cc`. **Run on macOS arm64 and nowhere else** (§B1). *(Until
  the tenth review this said "Runs on every target Rust runs on", which is
  the research's sentence about `include_bytes!` into a linked stub
  (`docs/research/02-native-binaries.md:46-47`), the construction ERRATA
  superseded. What is built writes into a copy of the running executable;
  the ELF and PE cases are unmeasured.)*
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
  `cranelift-object` writes them into the object it hands back. *(Until the
  eighth review this said it "keeps them behind `relocs()`"; `relocs` is a
  private field of `ObjectModule` there, `src/backend.rs:248`, and no
  accessor has that name.)* *(Read 2026-10-08 at
  0.135.0, the version `Cargo.lock` pins: `cranelift-jit`'s `src/backend.rs`
  collects a function's relocations at `:482-484` into private state, and no
  `pub fn` in that file returns them; `cranelift-object`'s `ObjectProduct`
  hands back the written `object` as a public field, `src/backend.rs:1137-1139`,
  which is what criterion 7's test reads.)*

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
**Followed 2026-10-08 — the seventh review's S9 — and "every lambda"
understates it.** `save.stacks` calls `save_stacks` (`bund_save.rs:62-70`),
which writes **every value on every stack** with `to_binary`
(`reference/Bund/src/stdlib/helpers/world/stacks.rs:100`), and a saved model
goes the same way (`reference/Bund/src/stdlib/helpers/world/models.rs:90`).
`save.aliases` calls `save_aliases` alone (`bund_save.rs:42-50`). So the
format reaches the file for any value a stack holds, not only for lambdas.
Nothing here rests on it: D31 is resolved and §B2 uses neither format.

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
  has not been asked for either is given both here, as the reference gave
  them at construction."
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
*(Measured on macOS, arm64, 2026-10-08, `rustc` 1.97.1: with nothing on
`PATH`, compiling `fn main(){}` ends in ``error: linker `cc` not found``.
Linux was not measured.)*
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

**That paragraph is about the form that was not built, and this one is about
the form that was — added 2026-10-08, the seventh review's S8.** What a Linux
or Windows user gets is the in-place write into a copy of the running
executable. **It has been run on macOS, arm64, and nowhere else.** *(One
half of it has, since 2026-10-09, the fifteenth review's S6: every `bund2`
reads its region before it acts on argv, and the workflow `measure-fetch`
ran a debug `bund2` 443 times on Ubuntu x86-64, in each of two runs. So the
empty-region path has run on ELF. The write, and a region that holds a
program, have not.)* The
workflow would run `bundle_build` on Ubuntu, and it has not: it runs on a
push to `main` and on pull requests, its one recorded run is from 2026-09-11,
and `bundle.rs` was first committed on 2026-09-30 on a branch. So ELF is
unmeasured in the built form too, and PE is unmeasured and has no runner.

**What the runtime does when the region is empty, or is wrong.** D37 forbids
a panic, and a bundle runtime is the same binary as the interpreter, so this
path is reached by every `bund2` that starts: each one asks its own image
whether it carries a program before it acts on argv
(`crates/bund2-cli/src/main.rs`, `run_cli`). One case that is not a failure
and five that are, none of them an abort (`crates/bund2-cli/src/bundle.rs`,
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
- **A restriction bit this bund2 does not define** — added 2026-10-09, the
  eleventh review's S2, and missing from this list until the twelfth's S4.
  Refused: a restriction that cannot be enforced is not passed over.

Criterion 12 checks all six, because a corrupted artefact is the one input a
bundle's front end is guaranteed to meet eventually and the only one that could
reach for an `unwrap`.

*Until 2026-10-08 this list was the appended form's: no trailer, a truncated
trailer, a length beyond the file, unreadable text. A fixed region cannot be
truncated, and a bad state byte and an unknown container version were in
neither that list nor criterion 12's. The sixth review, S6.*

**Every `bund2` carries the region, bundle or not.** 1 MiB in every build, and
a damaged region stops the plain interpreter as it stops a bundle. That is a
change to the interpreter's start-up and not only to artefacts.

**The runtime relies on the compiler not folding its reads of the region,
and that is a hint and not a guarantee.** The region is an immutable
`static`, which a compiler may read at compile time; the release build did,
and a filled bundle ran as the plain interpreter (the implementation review).
`carried` reads it through `std::hint::black_box`
(`crates/bund2-cli/src/bundle.rs`). The only check is `cargo xtask bundle`,
which builds and runs a release artefact and is in neither `cargo test` nor
CI. **No fallback is decided**: a runtime that reads its payload from its own
file cannot be folded, and that is the construction D118 did not take.
*(Stated 2026-10-08; three reviews listed it as assumed.)*

**The trailer records Bund2's own version, not only `reference/`'s SHAs.** A
builder and a prebuilt runtime can be different Bund2 builds — the pinned SHAs
say which oracle the *meaning* was fixed against and say nothing about the code
doing the interpreting. Without Bund2's version in the trailer, a mismatch is
undetectable; with it, the runtime can refuse or warn on one it does not
recognise.

*(Stated 2026-10-08, the eighth review's S4: the field reads `0.0.0` for
every build. It is `CARGO_PKG_VERSION` and the workspace has not been
versioned, so `--inspect` cannot tell two builds from different commits
apart. The mechanism is in place and what it records is not yet useful. What
should distinguish builds — a version that moves, or a commit — is not
decided here.)*

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
  `threshold_from_env`). Criteria 2 and 10 name which of these they set;
  criterion 6's test sets `BUND2_JIT_THRESHOLD=2` and `BUND2_STATS=1`, which
  its text did not say until the eleventh review.
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

  **Six things about this list. Three were not written down until
  2026-10-08 — the seventh review — and the last three were added by the
  eighth, tenth and eleventh.** *(This count has been wrong three times;
  the six are `--`, `--debugger`, the restriction variables, the two that
  shape reports, the three for the history file, and the five for a
  proxy.)*
  - **A literal `--` is data.** `bund2 script --file p.bund -- x y` gives the
    program `x y`; `./p -- x y` gives it `-- x y`. Measured with
    `args println`. It follows from "the runner consumes nothing" and is the
    one place a wrapper moved from one form to the other sees a difference.
  - **`--debugger` has no bundle form**, by a variable or otherwise. A
    `BUND2_DEBUGGER` would let an exported variable turn a shipped program
    into one that waits on its input (`crates/bund2-cli/src/main.rs`,
    `run_carried`). D115 is the same ruling for the words.
  - **The restriction variables are a bundle's alone.** `bund2 script` reads
    `--noio` and `--noeval` from its command line and does not read
    `BUND2_NOIO` or `BUND2_NOEVAL`, so the same program under the same
    environment may be restricted as a bundle and not as a script.
  - **Two more variables are read, by a bundle exactly as by `script`**:
    `BUND_LOG_LEVEL` and `COLUMNS` (`crates/bund2-stdlib/src/logging.rs`,
    `crates/bund2-stdlib/src/report.rs`). Not this design's; listed so the
    list is whole.
  - **And three decide where the debugger words keep their line history, or
    whether they keep any — added 2026-10-08, the tenth review's S5**:
    `XDG_CONFIG_HOME`, `HOME` and `APPDATA`
    (`crates/bund2-stdlib/src/terminal.rs`, `history_path`). With none of the
    three set no history file is written (§B3a). The list above was two short
    of whole while it said it was whole.
  - **And five decide who answers an `http://` fetch — added 2026-10-09,
    the eleventh review's B1, and cut from eight the same day by F182**:
    `http_proxy`, `all_proxy`, `ALL_PROXY`, and `no_proxy`, `NO_PROXY`.
    They are the five libcurl reads for the reference; `HTTP_PROXY`,
    `HTTPS_PROXY` and `https_proxy`, which `ureq` read by default, are no
    longer read (§B7). The reference also reads `ftp_proxy`, for a scheme
    Bund2 refuses (D126). **This list has said it was
    whole twice and been short both times, so it no longer says so**: it is
    the variables found by reading Bund2's own `std::env::var` calls and the
    one dependency a review traced, and a dependency not yet traced may read
    more.
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
  reports locations against it — so a bundle's report matches a `script` run's
  of the same program from the same path. *(Until the fourteenth review's S7
  this said "stderr" here and in the preservation table. A report is written
  to standard output: `a_url_the_reference_refuses_reaches_no_listener`
  reads it there.)* **A longer path keeps its last 256
  bytes** — or up to three fewer, so that what is kept begins on a character
  and not inside one *(fixed 2026-10-09, the eleventh review's S3: a cut
  inside a character reached `--inspect` and every diagnostic as U+FFFD;
  `a_shortened_source_path_is_cut_on_a_character`)* — so the file name survives and the leading directories do not
  (`crates/bund2-cli/src/bundle.rs`, `write_into`), and the build says so on
  stderr. Measured 2026-10-08 with a 353-byte path: the bundle's report names
  the shortened path, a `script` run names the whole one, and until that day
  the build said nothing. `a_source_path_too_long_to_record_is_reported_at_build`. Without this, criterion 2 would compare output that
  differs by construction, and one approved deviation's recorded hash pins a
  path Bund2 prints — `probes/execute-arm-not-executable`, the one of the
  twelve whose output names its own source file, found 2026-10-08 by running
  each — so the CEILING would move.

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

**Seven more things `bund2 build` does, four of them built and tested and
unwritten until 2026-10-08 — the seventh review's P5 — and three that came
with F181 and the review after it.**

- **`--output` naming the binary doing the building is refused**, compared by
  canonical path (`a_build_refuses_to_overwrite_the_binary_doing_it`).
  **A hard link to it is a second name the comparison cannot see, and until
  2026-10-08 that destroyed the builder — F181, the ninth review's S1.** The
  write was in place, so it went through the link into the builder's bytes
  and reported success. The artefact is now written, made executable and
  signed in a scratch directory beside the destination and then moved into
  place, which replaces a name and never a file's contents: the alias
  becomes the artefact and the builder is untouched
  (`a_hard_link_to_the_builder_is_replaced_and_the_builder_survives`). For
  the same reason an artefact rebuilt while it is running is not rewritten
  under itself. Measured by the tenth review: an artefact waiting on `input`
  was rebuilt from another program; the running copy finished as the old
  program and the next run was the new one. **A symlink at `--output` is
  followed, whether or not its target exists.** Between F181 and the tenth
  review's S1 a link to nothing was replaced by the artefact, where the
  in-place write had created its target
  (`a_dangling_link_at_output_is_followed`).
- **`--output` naming the program being built is refused — added
  2026-10-08, the tenth review's S2.** `bund2 build --file p.bund --output
  p.bund` exited 0 and left `p.bund` an executable. It was inside "replaced
  without a word" below to the letter, and it is the one instance that
  destroys what the build was given: the builder can be rebuilt and a source
  cannot. Compared by where each path leads, so a symlink to the source is
  refused too (`a_build_refuses_to_overwrite_its_own_source`). A hard link
  to the source is replaced as a name, and the source survives.
- **The destination's directory has to be writable — F181's cost.** The
  artefact is assembled beside where it will go, so a writable file in a
  read-only directory, which an in-place write could fill, is now refused
  with the reason
  (`a_build_into_a_read_only_directory_says_what_it_needed`). The file that
  was at `--output` is replaced and not rewritten, so its mode and ownership
  are not kept. A build killed between the write and the move leaves
  `.bund2-build-<pid>/` beside the destination, holding a copy of the
  runtime; a later build with the same process id clears it, and nothing
  else does. Read from the code, not measured.
- **A build that fails leaves `--output` as it was.** That now includes a
  signing failure, which used to leave a file there the system would kill
  (F181). Read from the code; no test makes `codesign` fail.
- **Any other existing file at `--output` is replaced without a word.**
  Measured: built over an existing text file, the command exits 0 and leaves
  an executable. That is what a compiler's `-o` does, and it is stated here
  and not defended; refusing would be a change to decide.
- **A bundle cannot build.** It owns all of argv, so `./bundle build …` is
  its program's arguments (`a_bundle_cannot_build_another_bundle`).
- **A bundle of an empty program is a bundle that does nothing**, not the
  interpreter: the state byte says filled and the length says zero
  (`an_empty_program_does_not_become_the_interpreter`).

An argument error exits 2. Every other failure to build — an unreadable
source, a syntax error, a program over capacity, a signing failure — exits 1.

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

**`--inspect` reads only a header its own bund2 reads — a limit, stated
2026-10-08 on the seventh review's S2.** It finds the region by a sentinel
followed by a header that validates: container version 1, a state byte of 0
or 1, a length within capacity (`crates/bund2-cli/src/bundle.rs`,
`header_validates`). That is what tells the region from a constant the
optimiser left in the code. So an artefact from another container version, or
a damaged one, is **not found**, and "container version" can only ever print
this bund2's own. Until that day the refusal said "the region is absent",
which was false of an artefact that names its version when run; it now says
the header is one this bund2 does not read, gives the three things that can
mean, and says that running the artefact reports which
(`inspecting_a_damaged_artefact_does_not_call_it_regionless`). So the Bund2
version field serves a reader of the *same* container version, and reading a
foreign header is part of what cross-target bundling would have to add
(§B1).

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

**And two the measurement could not see: `stdin` and `stdin.` — added
2026-10-08, the seventh review's S1.** The reference registers both *only*
under the flag, as stubs
(`reference/Bund/src/stdlib/functions/filesystem/file.rs:95-96`; the other
arm, `:100-103`, has neither), and Bund2 does the same. `bund2 words` lists
an unrestricted registry, so a name that exists only when the flag is set is
not among the 610. Run by hand, both answer
`bund FILE functions disabled with --noio`. **So 61 names answer with the
stub**, and the method above finds every one that is also a word without the
flag.

**A third of that kind, and one name the flag removes: `fs.is_file.` and
`fs_is_file.` — added 2026-10-08, the eighth review's S1. So 62, not 61.**
Under the flag the reference registers `fs.is_file` and `fs.is_file.` as
stubs; without it, `fs.is_file` and `fs_is_file.`, the workbench form under an
underscore
(`reference/Bund/src/stdlib/functions/filesystem/filesystem.rs:70-75`). F150
records the misnaming and Bund2 reproduces both halves. Measured again
2026-10-08 under `--noio`: `"/etc/hosts" fs.is_file. println` answers
`bund FILESYSTEM functions disabled with --noio`, and
`"/etc/hosts" fs_is_file. println` answers `fs_is_file. not registered`. So a
name under the flag has three outcomes and the method above looked for two: a
stub, untouched, and **removed**. Of the 610, 59 are stubs, one is removed,
and 550 are untouched; three more stubs exist only under the flag.

**The line D119 added to the list above, which the paragraphs between had
separated from it** *(moved 2026-10-08, the ninth review's S5)* —

- data files, **by D119 and not by the reference**: `csv` `csv.` `sqlite`
- reading a file or a URL in order to run it, **by D132 and not by the
  reference** *(added 2026-10-09, the sixteenth review's B1)*: `use` `use.`
  `bund.eval-file` `bund.eval-file.`

**So of the 610 names 63 are stubs under the flag, one is removed and 546
are untouched; with the three that exist only under it, 66 names answer
with the stub.** The 610 were not run again for this: the four are added to
the counts above, and each is tried by name
(`noio_reaches_the_four_words_that_fetch_and_run`).

**What `--noio` leaves ungated — rewritten 2026-10-08, the sixth review's
B2.** Everything else. Until that day this paragraph named `args`,
`sleep.seconds` and `io.graph`, which are the ungated words *of one module*
(`crates/bund2-stdlib/src/host.rs`, module documentation) presented as the
flag's whole surface. Of the 550 names the flag does not touch (551 until the
eighth review, which counted `fs_is_file.`; the flag removes it), these reach
outside the program, and each was among the ones measured. **The measurement
says which names are stubs; which of the rest reach outside is a selection by
reading, and this is how it was checked** (2026-10-08): every file under
`reference/Bund/src/stdlib/functions` that touches a file, a socket or a
process and carries no `disabled with` stub is `conditional_csv.rs`,
`conditional_sqlite.rs`, `debug_debug.rs`, `debug_shell.rs`, `ai/ollama.rs`
and `internaldb/mod.rs`. The last two register words Bund2 does not have.
*(Corrected 2026-10-09, the sixteenth review's B1 and S2. That check was
short in two ways. It passed over a file that carries any `disabled with`
stub, and `bund_use.rs` and `bund_eval.rs` carry the other flag's —
`disabled with --noeval`, at
`reference/Bund/src/stdlib/functions/bund/bund_use.rs:63` and
`bund_eval.rs:106` — so the four words that fetch and run were never looked
at for this flag. And there is a seventh file with no stub at all:
`bus/globals.rs` registers `global`, `global*` and `?global` ungated
(`reference/Bund/src/stdlib/functions/bus/globals.rs:109-111`), and each
goes to zenoh (`:37`, `:61`, `:93`). Bund2 has none of the three: D28 defers
them and D87 keeps that file deferred by name, so nothing is ungated today
that this section calls gated. Whoever lifts that deferral meets two
questions this RFC can only name. `global*` pushes whatever a peer sent
(`:65-73`), which is a third place a value arrives already built, beside
the two searches below; and whether `--noio` gates the three is D119's
question again.)* The
words below that carry no citation — the `sysinfo.mem.*` and
`sysinfo.virtualization` words, the `debug.display_*` words, the clock, sleep
and randomness words — are listed from Bund2's registry by what they do, and
were not each traced into the reference:

- **Four read a file or a URL the program names and run it, and no longer
  do under the flag — D132, 2026-10-09; the sixteenth review's B1.** `use`
  and `use.` fetch their operand and evaluate the answer
  (`reference/Bund/src/stdlib/functions/bund/bund_use.rs:31-33`), and
  `bund.eval-file` and `bund.eval-file.` do the same for a path
  (`reference/Bund/src/stdlib/functions/bund/bund_eval.rs:65-67`). The
  reference makes each a stub under `--noeval` alone (`bund_use.rs:74-79`,
  `bund_eval.rs:117-127`). Measured by that review before the ruling: the
  oracle with `--noio`, `bund2 script --noio` and a bundle built `--noio`
  each evaluated a library from a listener and one from a file. **This
  list did not have the four** from the day it was rewritten, 2026-10-08,
  until that review, though D78 had named the route since the day it was
  ruled. **The owner ruled that `--noio` gates them**, an approved
  deviation. They fail with `bund USE functions disabled with --noio` and
  `bund BUND.EVAL-FILE functions disabled with --noio`; with both flags set
  the message is `--noeval`'s, as it is in the reference.
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
- **Two write a file.** *(Headed "One" until the ninth review's S4; the
  second is at the end of this bullet.)* `debug.shell` saves its line history. Measured on
  the same kind of bundle: one typed line wrote
  `bund2/bund_debug_shell_history.txt` under the configuration directory of
  whoever ran it (`crates/bund2-stdlib/src/terminal.rs`, `history_path`). The
  reference writes the same file into the working directory
  (`reference/Bund/src/stdlib/functions/debug_fun/debug_shell.rs:20`, `:51`)
  and gates it no more than Bund2 does. `debug` keeps one of its own,
  `bund_debug_debugger_history.txt`, by the same route. Measured by the
  eighth review on a bundle built `--noio --noeval` whose program is
  `"40 2 + println" debug`: one typed line wrote it; with standard input
  closed nothing was written.
- **Standard input**: `input` `input*` `password` `bund.prompt`
  (`reference/Bund/src/stdlib/functions/io/input.rs:142-145`), and `debug`
  and `debug.shell`.
- **The host's identity**: `system.ip` `system.ipv6`
  (`reference/Bund/src/stdlib/functions/system/ip.rs:34-35`), `system.locale`
  (`system/locale.rs:27`), `sysinfo.hostname` `sysinfo.kernel_version`
  `sysinfo.os_version` (`sysinfo/host.rs:66-68`), `sysinfo.system`
  `sysinfo.version` (`sysinfo/host.rs:69-70`) `version`
  (`reference/Bund/src/stdlib/functions/create_aliases.rs:32`)
  `sysinfo.virtualization`
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
**D132 moved four more, 2026-10-09**: reading a file or a URL *in order to
run it* is on the gated side in Bund2, and in the reference it is the other
flag's. *(Until the sixteenth review this paragraph said what "I/O" means
to the flag and not what it leaves out, and what it left out was `use`.)*

**One line of this is a bundle's own.** The history file is written on the
machine of whoever *runs* the artefact, by a word its author left in. For a
`script` run that is RFC-0008's business; for a shipped artefact it belongs
here, and no flag switches it off. **The environment does** *(the tenth
review's S5; until then this said "nothing switches it off")*: the file goes
under `XDG_CONFIG_HOME`, or the platform's convention under `HOME` or
`APPDATA`, and with none of the three set none is written
(`crates/bund2-stdlib/src/terminal.rs`, `history_path`). Measured by the
tenth review on a bundle of `debug.shell` built `--noio --noeval`: with all
three unset the typed line still ran and no file appeared.

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
carries the same four, has a dated note. *(2026-10-09: four of the six —
the file pair, `use` and `use.` — are stubs under `--noio` as well in
Bund2, D132.)*

**Bund2 stubs a seventh, which is not the reference's: `debug.run` — D120,
2026-10-08.** It is Bund2's own word (RFC-0008), and a comment calls it
"`bund.eval` with a safepoint offered before each term": the one in the
list of functions allowed to re-enter evaluation
(`crates/bund2-stdlib/src/lib.rs`, `every_reentering_function_is_named`),
and since D120 the one beside its stub. *(Until the sixteenth review's S3
this said "its registration comment". The word is registered elsewhere, and
the comment quoted stands in a test's list.)* It sat outside the group because the
group is the reference's list and the word was added later. Measured before
the ruling, on a bundle built `--noeval --noio` with standard input closed:
`"40 2 + println" debug.run` printed `42`. **The owner ruled that `--noeval`
gates it.** It now fails with
`bund DEBUG.RUN functions disabled with --noeval`
(`noeval_reaches_debug_run`). Not a deviation: the reference has no such
word, so nothing of its is changed. The seventh review's B1.

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
three words above evaluate the result. *(2026-10-09, the sixteenth review's
B1: and under `--noio` alone the route was `use` itself, which this
paragraph mentioned in a clause and §B3a's list for that flag did not have.
D132 closes it in Bund2. The statement stands for the reference; for Bund2
under `--noio` alone it stands by reading and not by measurement — `input`
is ungated and `bund.eval` is the other flag's.)*

**`bootstrap` is another such route under `--noeval` alone — added
2026-10-08, the eighth review's S2.** It reads every script stored in a world
file and evaluates each
(`reference/Bund/src/stdlib/functions/bund/bund_load.rs:42-50`), and its only
gate is `--noio` (`:192-197`). Bund2 does the same through `eval_source`
(`crates/bund2-stdlib/src/world.rs`, `bootstrap`). Measured by the eighth
review, on a world file holding the script `40 2 + println` and a bundle of
`"<file>" bootstrap`: built `--noeval` it prints `42`; built
`--noeval --noio` it answers `bund LOAD functions disabled with --noio`. It
needs no `compile lambda! !`: it is `bund.eval-file` for a world file, and
outside the group. The reference's boundary, by D79.

**The world file brings in runnable values too, under `--noeval` alone —
added 2026-10-08, the ninth review's S3.** `load.lambdas` registers every
stored lambda and `load.stacks` restores every stored stack, each value
decoded by `Value::from_binary`
(`reference/Bund/src/stdlib/helpers/world/lambdas.rs:32`,
`reference/Bund/src/stdlib/helpers/world/stacks.rs:29`); `load` does both,
and `bootstrap` does `load` before it evaluates a script (`bund_load.rs:36`).
Bund2 decodes the same values (`crates/bund2-stdlib/src/world.rs`,
`load_lambdas`, `load_stacks`). Measured by the ninth review on bundles built
`--noeval`: a world holding the lambda `hidden`, `{ 40 2 + println }`, and
the program `"r9" load.lambdas hidden` — `42`; a world whose stack top is
that lambda, and `"r9s" load.stacks !` — `42`. With `BUND2_NOIO=1` the first
answers `bund LOAD functions disabled with --noio`. All are `--noio` stubs,
so this is the reference's boundary by D79, and these are its names.

**`decode.base64` makes a string runnable with no parser, under both flags —
added 2026-10-08, the ninth review's B1.** `encode.base64` serialises any
value (`reference/Bund/src/stdlib/functions/encoding/base64.rs:33`) and
`decode.base64` is its inverse: it casts its operand to a string (`:71`),
decodes it (`:77`) and hands the bytes to `Value::from_binary` (`:83`). Both
are registered with no gate (`:121-124`), and Bund2 does the same
(`crates/bund2-stdlib/src/encoding.rs`, `decode_base64`). A LAMBDA is a
value, so the result is something `!` runs. Measured 2026-10-08 on bundles
built `--noeval --noio`, where the string is the 572 characters
`{ 40 2 + println } encode.base64 println` prints:

    "<the string>" decode.base64 !        prints 42, standard input closed
    "> " input decode.base64 !            prints 42, the string piped in

`--inspect` reports both restrictions on each. The ninth review ran the first
on the oracle at `21b40b0` under both flags, and each binary ran the other's
string. **It gives a program nothing `compile lambda! !` does not** —
`"> " input compile lambda! !` already runs a line from standard input under
both flags — so the floor is where it was. What it changes is what a reader
auditing a program must look for: not text. **The owner ruled the same day that
D122's "leave it and name it" covers this word — D123.** It is the
reference's word, ungated there, and preserved; gating it would have been a
deviation, as D119 was.

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

**`debug` evaluates its operand before it reads anything — corrected
2026-10-08, the eighth review's B1. The paragraph above is right about
`debug.shell` and wrong about `debug`.** `debug` pulls a STRING
(`debug_debug.rs:127-135`), parses it (`:52-53`) and applies each word of it
(`:75`). The read loop the paragraph cites (`:82-89`) runs after each word;
the end of input leaves it (`:100-103`) and the outer loop goes on to the
next word. Bund2 does the same (`crates/bund2-stdlib/src/terminal.rs`,
`debug_word`). Measured 2026-10-08 on a bundle built `--noeval --noio`, with
standard input closed, whose whole program is

    "40 2 + println" debug

it prints `42`. The eighth review ran the same program on the oracle at
`21b40b0` under both flags, and it prints `42` there. So this is a route
that needs no person and no `debug.feed`, beside `compile lambda! !`: it is
`bund.eval` with a table drawn before each word. **The owner ruled the same day that
`--noeval` does not gate it — D122.** It is the reference's word and ungated
there, so it is D79's boundary; gating it would have been a deviation from
the reference, as D119 was and as D120 and D121 were not. It is named here as
what it is, which D78 requires.

**`debug.shell` needed nobody at the keyboard, and under the flag it does
again — D121.** *The measurement below is from before D121, which the owner
ruled the same day: `--noeval` stubs `debug.feed`, so this program is refused
at its first word with `bund DEBUG.FEED functions disabled with --noeval`.
The cost is that under the flag `debug.feed` cannot queue a line for `input`
either. What follows is kept as the record of what was found.* Added
2026-10-08, the seventh review's B1. The paragraph above describes a typed line. `debug.feed` is
Bund2's word and queues a line for the next word that reads (RFC-0008 §W4),
so a program can supply the line itself. Measured after D120, on a bundle
built `--noeval --noio` with standard input closed:

    "40 2 + println" debug.feed debug.shell

prints `42`. The word that evaluates is the reference's and ungated there;
the word that removes the operator is Bund2's. **D120 names `debug.run` and
gates `debug.run`**, so this route stays as it is, and it is named here
because D78 requires it. A fed `"1" bund.eval` is refused like a typed one.
*(The eighth review, B1: "needed nobody at the keyboard, and under the flag
it does again" is true of `debug.shell` only. `debug` never needed a person
or `debug.feed` — the paragraph before this one — so D121 closed the route
it names and not every unattended one.)*

**So "the reference's boundary" is the answer for the reference's words
only.** Until 2026-10-08 this section gave it for every ungated route.
`debug.run` showed that a word Bund2 adds has no boundary to inherit; whether
it evaluates under the flag is chosen, and D120 and D121 are that choice for
Bund2's two. The reference's own `debug` is the case the other way: a
boundary inherited, and one that runs a held string unattended.

**How this list was closed — added 2026-10-08, the eighth review.** By
reading every place outside tests where Bund2 parses a string at run time:
each call of `bund2_syntax::compile`, `bund2_syntax::parse`, `eval_source`,
`eval_line` and `eval_str` under `crates/`. The words that do are —

- `compile`, which parses and does not run;
- `bund.eval` `bund.eval.` `bund.eval-file` `bund.eval-file.` `use` `use.`,
  stubbed by the flag;
- `debug.run`, stubbed by D120;
- `bootstrap`, `debug` and `debug.shell`, the reference's, which this flag
  does not gate.

Two more sites are not words: the console's line evaluator
(`crates/bund2-stdlib/src/lib.rs`, `eval_line`) and the child VM a
breakpoint's condition runs in (`crates/bund2-cli/src/debugger.rs`,
`evaluate_in_child`). A bundle reaches neither, by D115. What is left is the
CLI parsing the program it was given and an embedder's own
`Runtime::eval_str`. `debug.feed` parses nothing; D121 gates it for what it
hands `debug.shell`. **A word added later that parses a string appears in
that search and not in this list**, which is how four reviews in a row found
one.

**That search was one of two, and the paragraph above read as a guarantee it
could not give — the ninth review's B1 and S2.** A value can also arrive
already built: the wire codec decodes a LAMBDA as readily as an integer, and
no parser runs. So the second search, made 2026-10-08, is every call of
`wire::from_binary` under `crates/` outside tests. There are six —

- `decode.base64` and its workbench form (`encoding.rs`, `decode_base64`):
  **ungated by either flag**;
- `load.model`, `load.lambdas`, `load.stacks`, and through them `load` and
  `bootstrap` (`world.rs`): `--noio` stubs;
- `sqlite`'s BLOB cells (`data.rs`): a `--noio` stub by D119;
- `recv` (`bus.rs`): a `--noio` stub.

A word added later that parses a string or decodes a value appears in one of
the two searches. *(Until the tenth review this paragraph ended "A third
mechanism, if there is one, appears in neither". There is one, and the
registers had it.)*

**The third mechanism: a program builds code out of strings. No search finds
it, because it has no site — corrected 2026-10-08, the tenth review's B1.**
The words are the language's ordinary ones —

- `make.call`, `make.call.` and the alias `call,` turn a string into a CALL
  (`reference/Bund/src/stdlib/functions/values/make_call_value.rs:31-38`,
  registered at `:61-62`; the alias at
  `reference/Bund/src/stdlib/functions/create_aliases.rs:33`);
- `ptr` turns a string into a PTR and applies it
  (`reference/rust_multistackvm/src/stdlib/artefacts.rs:80-93`);
- `lambda*` folds the whole stack into a LAMBDA
  (`reference/Bund/src/stdlib/functions/bund/bund_fun.rs:189-202`,
  registered at `:219`), and `lambda!` does it to a LIST (`:218`);
- `!` on a STRING, a PTR or a CALL calls the word of that name
  (`reference/rust_multistackvm/src/stdlib/execute.rs:27-30`).

None has a gate, and Bund2 has the same words
(`crates/bund2-stdlib/src/values.rs`). Measured 2026-10-08 on bundles built
`--noeval --noio`; each prints `42`:

    40 2 "+" make.call "println" make.call lambda* !               stdin closed
    40 2 "+" ptr ! "println" !                                     stdin closed
    42 "> " input !                                                stdin: println
    40 2 "> " input string.tokenize { make.call } map lambda! !    stdin: + println

The tenth review ran the four under `bund2 script` with both flags and on
the oracle at `21b40b0`, with the same answers. The last reads a line of
words from standard input, splits it, makes each callable and runs them; the
third is three words long.

**This was decided before it was found.** D16: "a call target may be named
by a string that exists only at run time". D79, on why `--noeval` does not
gate `compile`: "D16 means a call target can be assembled at run time, so
evaluation cannot be switched off by name-gating a word list." D34 calls
`call,` then `lambda*` then `register` the canonical metaprogramming idiom,
and a hermetic golden runs it, so criterion 2 exercises this mechanism in
every configuration. This section used D16 forty lines on to justify a
switch and had not applied it to what the switch leaves.

**So the routes are not a list a search can close, and this section no
longer presents them as one.** The parse sites and the decode sites are
sites: found by the two searches, and named above. Construction has none.
Any program with a source of strings — `input`, `args`, a literal — and `!`
is an interpreter for what it is handed, under both flags, in the reference
and in Bund2. A reader auditing a restricted program for the names above
has not audited it; the name to look for is `!`. What the flags do is what
they say: `--noeval` makes the `bund.eval` group and Bund2's two words fail,
`--noio` makes its 62 names fail, and neither says anything about what else
a program can cause to run. No gate is proposed: it would be a deviation,
and of the feature the language is for.

**A bundle is never given a debugger — D115, 2026-10-08.** In a `script` run
the first arming or moving debugger word attaches a console (D113.5), and
until D115 a bundle whose program called `debug.step` stopped at one. In a
bundle those words now do nothing: no console, no stop, and the tier stays
on. So a stop at a console is not a third route, and a shipped program does
not wait at a prompt because a breakpoint was left in it.
`a_bundles_debugger_words_do_nothing` holds it.

**D115 also holds up D78's floor, and until 2026-10-08 nothing said so — the
eighth review's S3.** A breakpoint's condition is a string of Bund run in a
child VM, and that VM was built with neither restriction. Measured before the
fix, under `bund2 script --noio --noeval` with standard input closed: a
`debug.break.if` condition of `fs.cwd println :q bund.eval true` printed the
working directory and reached `bund.eval`. So "the stub holds wherever the
word is typed" was false of a condition. A bundle was spared only because
D115 makes the arming words do nothing. Both halves are held now: the child
is given the program's restrictions (**F180**,
`a_condition_is_no_freer_than_the_program_that_armed_it`), and
`a_bundles_breakpoint_condition_is_never_evaluated` holds the bundle's half
by name, so relaxing D115 for one word cannot open the route unnoticed.

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
even at 64** — "These are demonstrations and tests — they run once". So a JIT
bundle of a one-shot script **compiles nothing and pays only size**:
**2,259,088 bytes, 12.45% of the binary**, by criterion 1. *(Until 2026-10-08
this sentence rested on the research's claim that `cranelift-codegen` is "the
single largest code contributor to any binary that embeds the JIT"
(`docs/research/02-native-binaries.md:54-55`), which criterion
1 measured as false for Bund2 and ERRATA records; it also gave F139's words
as a paraphrase inside quotation marks.)* Criterion 1 is that measurement,
and it exists only because D80 makes such a bundle buildable.

### §B5 — `--emit=native`, and what blocks it

**The first draft said "the same lowering, pointed at `ObjectModule`". That is
wrong, and the reason is in the lowering's own safety argument.**

`Compiler::compile_word` takes `vm: &mut dyn Vm` and calls `plan_body(body,
vm)`: the plan is made against a **live** VM, resolving slots and effects from a
registry that exists. And the emitted code **bakes the compiling process's heap
addresses in as immediates** — `slots_base` is `slots.as_ptr() as i64`, issued
as `iconst`, and the next instruction does the same for §S6's cells, with the
comment saying exactly why that is safe. *(The comment is on `cells_base`
and is about the cells; until 2026-10-08 this read as though it were on
`slots_base`. The argument is the same for both allocations.)*

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

**The record was short by its largest item — added 2026-10-08, the seventh
review's S5.** D83 names a fourth cost and says it "is not in §B5's list,
and it is the largest":

4. **The meaning guards have no cells at build time.** §S6's guard compares a
   per-name generation cell whose address is baked into the code, and those
   cells are minted at registration, per `Interp`. Nothing is registered at
   build time, so the cells would have to be created at load and found by
   name through relocations resolved then. That is the machinery that makes a
   redefinition observable, so an error in it is silent wrongness and not a
   crash.

**§B5 reopens an exclusion RFC-0005 closed this morning.** Criterion 30's two
mirror cases were excluded because "§S7 compiles a body *on* an entry and runs
that entry interpreted", and that row states its own trigger: "it holds only
while §S7 compiles on an entry rather than ahead of one. **If that changes** —
an ahead-of-entry or background compile, which RFC-0006 may want — the mirrors
become writable and this row reopens" (RFC-0005:6725-6740; the passage has
moved since this was written, and `cargo xtask cite` does not check a line
citation from one RFC into another). `--emit=native`
compiles ahead of every entry by definition, so **building this mode reopens
those two cases and they become owed**, not excluded. The trigger fired as
written, which is the argument for writing triggers that way.

**Which targets get `--emit=native` is not answered here.** RFC-0005 names
x86-64, aarch64, s390x and riscv64 as what Cranelift supports
(RFC-0005:3031-3039); whether Bund2
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

- `file:` follows curl's rules as far as they were measured — an absolute
  path after one slash or three or more, or after two and the host `localhost` or
  `127.0.0.1`, with dot segments removed from the text, then `%xx` decoded,
  and anything from `?` or `#` dropped. **The dot segments go before the
  file system sees the path**, as libcurl removes them for every scheme: so
  through a link `/d/link/../t.bund` is `/d/t.bund`, and a `..` after a file
  or after nothing at all still resolves. *(Until the fourteenth review's
  B3 Bund2 left `..` to the system, and `"/d/link/../t.bund" file` read one
  file in the reference and another in Bund2, both silently. `%2e%2e` is not
  a dot segment to either.)* A path whose
  decoded bytes are not UTF-8 — `%ff` — fails here; that is read from
  `percent_decode` and not measured against the oracle. The
  scheme is in any case, and so is `http`, which takes one to three slashes.
  *(Until F183, 2026-10-09, Bund2 took `file://` and `http://` in lower case
  only, and `localhost` alone; the twelfth review's S2.)* **One spelling
  differs, an approved deviation — D125: a `file:` URL with a literal space
  or control character fails in the reference and reads in Bund2, so
  `"/a b.txt" file` does. The reference's half is its defect, F184.**
  *(Until the sixteenth review's S4 this said "space or tab", which are the
  two that were measured. The ruling is wider, and a line feed in a path is
  inside it.)*
- `http://` is fetched by Bund2's own exchange, with no TLS —
  `crates/bund2-stdlib/src/http.rs`, D131. *(By `ureq` until 2026-10-09.)*
  It keeps three of the
  defaults the reference leaves curl at *(until the eleventh review this
  said "the defaults"; proxying is a fourth, kept since F182 except as said
  below)*:
  no redirect is followed, the body of an error
  status is still the answer, and the body has no size limit. **These three are
  what `file_helper.rs:42-54` leaves unset rather than what any one line
  states**; the user agent `ZBUS` is set, at
  `reference/Bund/src/stdlib/helpers/file_helper.rs:43`. **Which bytes are a
  response was `ureq`'s to decide until D131, and not libcurl's.** This
  bullet said "for two shapes they decide differently"; of 94 shapes they
  decided 33 differently (F187). Bund2 reads the response itself now, and
  how is the last bullet of this list.
- **A proxy is chosen as libcurl chooses it for the reference — F182,
  2026-10-09, the eleventh review's B1 and the owner's ruling that it was a
  defect.** Until then `fetch_uri` set no proxy and so had `ureq`'s default,
  which reads `ALL_PROXY`, `all_proxy`, `HTTPS_PROXY`, `https_proxy`,
  `HTTP_PROXY`, `http_proxy` in that order (`ureq` 3.4.0,
  `src/proxy.rs:222-240`). The reference sets no proxy option either
  (`reference/Bund/src/stdlib/helpers/file_helper.rs:42-46`), so it has
  libcurl's choice, and the two were not the same: Bund2 obeyed `HTTP_PROXY`,
  `HTTPS_PROXY` and `https_proxy`, which the reference ignores for an
  `http://` URL, and took `ALL_PROXY` ahead of `http_proxy`. **`use`
  evaluates what it fetches**, so the environment was deciding who supplies
  code for a URL the program names, by variables the reference does not read.

  Now `proxy_for` and `bypasses` (`crates/bund2-stdlib/src/host.rs`) make
  the choice, and the request is written from it *(`ureq` was handed it
  until D131)*. **The rules are libcurl's and no line
  of the reference states them, so each was measured**: the oracle at
  `21b40b0`, which links the system's libcurl, 8.7.1 on this machine, run
  with `"http://<host>:<port>/lib.bund" use` against three listeners — an
  origin and two proxies — for 73 settings of the variables, after the
  twelfth review for 175, after the thirteenth for 278, after the
  fourteenth for 443, and after the fifteenth and sixteenth for 741: the
  variables, the proxy's own value and its
  credentials, the URL's spelling, its host, user part, port and path, a
  `file:` path through a link, other schemes, a path handed to
  `bund.eval-file`, 233 shapes of response, and a request and a URL at
  libcurl's limits of size. **Every setting, every connection each one caused, and the script
  that ran them are in `docs/measurements/fetch-2026-10-09.md`.** "As
  libcurl chooses it" means for those rows, on that libcurl: it is a list
  somebody thought to try. The twelfth review found the proxy's default port
  by reading `ureq`, and the thirteenth found a port that is not a number
  the same way; neither was found by the list. The fourteenth found three
  more by reading `Authority`, which had three fields and read one. **So
  the list below the table is of the string's parts and not of settings:
  it says what Bund2 reads and what it hands on unread.** The fifteenth
  review found the response by reading that list: it was the one row whose
  reader was "nobody".

  | setting | oracle and Bund2 |
  |---|---|
  | none | direct |
  | `http_proxy`, `all_proxy` or `ALL_PROXY` | through it |
  | `HTTP_PROXY`, `HTTPS_PROXY` or `https_proxy` | direct — not read |
  | several set | `http_proxy`, then `all_proxy`, then `ALL_PROXY`; an empty one counts as unset |
  | a proxy with no scheme | an HTTP proxy |
  | a proxy with no port | **port 1080** — measured for no scheme, `http://` and `socks5h://`; `ureq` alone would take 80, and did until the twelfth review's B1. An `https://` proxy with no port did not reach 1080 on the oracle and which port it takes was not found; Bund2 refuses an HTTPS proxy before the port matters (D124) |
  | a port that is not a number from 1 to 65535, in the URL or the proxy — `:65536`, `:80x`, `:+80` | **the fetch fails and nobody is asked.** Until the thirteenth review's B1 Bund2 asked port 80: `http` read the text as no port and `ureq` supplied its default. Leading zeros are allowed, and an empty port in a URL is port 80 on both |
  | a proxy with a slash, path, query or fragment after its address | used, and what follows is ignored — with no scheme too, since the thirteenth review's S2 |
  | a proxy's scheme | **letters, digits, `+`, `-` and `.` after a letter, then `:/`, and one to three slashes after it**, as for a URL: `http:/host` and `http:///host` name a proxy, and `host:port/a://b` has no scheme. Until the fifteenth review's S2 Bund2 split a proxy at the first `://` and failed all three, where the reference asks the proxy |
  | a proxy with a user part | **sent as `Proxy-Authorization: Basic`, decoded as a URL's is** — `a%40b:c%3Ad@` is `a@b:c:d`, `u@` is `u:`, an empty one is `:`, `%zz` is as written. **One that decodes to a byte below `0x20` fails the fetch**, where the reference fetches with no proxy at all (F185). *(Until D131 Bund2 failed any proxy whose credentials held an escape — D128 — and refused `<`, `\|` and a byte above ASCII in them.)* |
  | `http://a@b@host/` | refused |
  | a URL's host | **a name is letters, digits and `-`, `.`, `_`, `~`, `\|`; any other punctuation byte is refused, and so is a bracketed host that is not an IPv6 address.** Until the fourteenth review's B1 Bund2 handed `ureq` the host as written, `http::Uri` took `a!b` and `[zz]`, and with a proxy set Bund2 asked the proxy for them and evaluated its answer, where the reference refuses the URL. *(`\|` Bund2 refused until D131, `http::Uri` not taking it.)* |
  | a blank or a control character anywhere in an `http:` URL | refused, a fragment's included. They are the bytes that would end a line of the request |
  | port 80 | not written: `:80` and `:080` go out as no port, in `Host` and in what a proxy is asked for |
  | an `http:` URL of 8,000,000 bytes or more, or a `file:` one of more than that | refused, and nobody is asked. Why the two differ by a byte was not traced |
  | a request of more than 1,048,575 bytes, from `GET` to its empty line | **the connection is made and nothing is sent**, and the fetch fails; the same through a proxy and with credentials. Bund2 does it in that order because the reference does: a listener sees a connection that says nothing from both |
  | an IPv6 host | sent in libcurl's text when that is shorter than what was written, and as written otherwise: `[0:0:0:0:0:0:0:1]` goes out as `[::1]`, `[::A]` and `[::ffff:7f00:1]` as they are. Bund2 sent it as written until the fourteenth review's S4 |
  | a URL's user part | **sent as `Authorization: Basic`, the name and the password each decoded** — `a%40b:c%3Ad@` is `a@b:c:d`, and `u@` is `u:`. `%00` in either is refused; an escape for any other byte is sent, a line feed included. Until the fourteenth review's S1 Bund2 sent the user part undecoded, and fetched `a%00b@host`, which the reference refuses |
  | the path of an `http:` URL | sent with dot segments removed — `/a/../b` is `/b` — and bytes above ASCII as `%xx`; a query as written. Bund2 sent the path as written until the thirteenth review's S5 |
  | `http_proxy=127.0.0.1:` | the fetch fails; `http://127.0.0.1:/` is port 1080 |
  | `http_proxy=http://` | the fetch fails; the next variable is not tried |
  | `no_proxy` and `NO_PROXY` both set | `no_proxy`, unless it is empty |
  | `no_proxy=*` | direct; `a,*` and ` * ` are not wildcards |
  | a host name | on the list by itself or a parent domain, ignoring case, one leading dot on the entry and one trailing dot on either; `*.example.com` matches nothing |
  | an IPv4 host | in any spelling libcurl reads as an address — `127.1`, `2130706433`, `0x7f.0.0.1`, `0177.0.0.1` are `127.0.0.1` (the twelfth review's S1); an entry is an address only as four decimal numbers, **and whether one with a leading zero is an address is the system's answer**: on macOS `0127.0.0.1` is one, with any count of zeros, and on Linux it is not and the proxy is used. Bund2 reads an entry as the system it runs on does (D130); or `address/bits` for 1 to 32 bits, **the bits being what C's `atoi` reads and libcurl then takes as unsigned**: `/0`, `/x` and `/+` mean the whole address, `/+8` and `/4294967304` are eight bits, `/-8` and `/+33` match nothing; an entry with a port matches nothing. **An entry of 128 bytes or more matches no address** and the entries after it are still read; Bund2 matched it until the fifteenth review's S1, and so went directly where the reference asks the proxy. *(Until the fourteenth review's B2 Bund2 refused `0127`, and read a sign as no digits; the two then asked different listeners, in both directions.)* |
  | an IPv6 host | **matched as a name is, by the text it is sent in.** So `[0:0:0:0:0:0:0:1]` is on a list naming `::1` and not on one naming itself; no prefix matches, `::1/128` included; `[::1]` as an entry matches nothing; and `[::FFFF:127.0.0.1]` is on a list naming `0.1`, its last "domain". *(This row said "by its shortest text, `::1`, and nothing else", which was true of the one address tried. Bund2 compared Rust's text for the address, which is another text for `::ffff:7f00:1`.)* |

  **What is handed on, and who reads it.** Until D131 `ureq` read the URL
  it was given with `http::Uri`, and the file system reads a path; each is
  a second reader with its own rules. Five reviews running found the fetch
  by asking which part of what is exchanged Bund2 does not itself read, so
  here is each part. **Since D131 the request is bytes Bund2 writes**, and
  the third column says what is written, or what the one second reader
  left — the file system — is handed:

  | part | read in `host.rs` by | what is written, or handed on |
  |---|---|---|
  | the scheme and its slashes | `Target::of` | `http://`, to a proxy; nothing otherwise |
  | a URL's user part | `credentials` | a header whose bytes Bund2 writes |
  | a URL's host | `curls_host` | a name of the bytes listed above, or an address Bund2 wrote. A name is then the system resolver's, as it is libcurl's. The fifteenth review compared one, `localhost` in four spellings, at `cd10669`: both binaries asked `::1` first and fell back to `127.0.0.1`. Measured again after D131 for which is asked first: `::1`, all four spellings, both binaries. The falling back was not measured again, and **nothing else of the resolver was compared** |
  | a URL's port | `Authority` | a number Bund2 wrote, or none |
  | a URL's path | `curls_path` | dot segments removed and bytes above ASCII escaped; **every other byte as written** |
  | a URL's query | `http_url` | **as written**, checked for a blank or a control character and for nothing else |
  | a fragment | `http_url` | nothing: it is dropped |
  | a proxy's scheme | `proxy_for`, `proxy_scheme` | nothing, or the fetch fails (D124) |
  | a proxy's user part | `curls_proxy`, `credentials` | a header whose bytes Bund2 writes |
  | a proxy's host and port | `curls_host`, `Authority` | as for a URL, with 1080 for no port |
  | what follows a proxy's address | `curls_proxy` | nothing |
  | a `no_proxy` list | `bypasses`, `ipv4`, `atoi` | nothing |
  | a `file:` path | `curls_path`, `percent_decode` | the decoded bytes, as a path. Bytes that are not UTF-8 fail here, read and not measured |
  | the request's lines | `Request::head`, in `http.rs` | the oracle's own, captured: the request line, `Host`, the two `Authorization` headers, `User-Agent`, `Accept`, and `Proxy-Connection` to a proxy, in that order |
  | the response | `read_response`, in `http.rs` | **read by Bund2, by rules measured on 233 shapes.** Until D131 this row said "nobody" and "all of it is `ureq`'s" |

  Not on this list because Bund2 sets them and they were compared: no
  redirect, no status treated as failure, no size limit, the user agent.
  Not on it and not compared: how long either waits, and what either does
  with a name that has several addresses beyond trying the first.
  *(Until D131 the order and case of the request's other headers differed
  too. They are the oracle's now, for the fourteen requests captured.)*

  **What the 741 rows say, stated as the measurement states it.** In 699
  the two cells are the same: the text printed or fetched, and every
  connection with its request line and credentials. In 40 Bund2 refuses
  and asks nobody: a SOCKS or HTTPS proxy (D124), a URL
  with no scheme or `https://` (D54), another scheme (D126), a host
  libcurl takes and Bund2 does not (D127), and a proxy whose credentials
  decode to a control byte, where the reference fetches directly (F185).
  In 39 of those the reference's cell shows somebody asked; the fortieth
  is `tftp://`, which is UDP, and no listener here could show whom it
  asked. In two, a literal space, Bund2 reads a file the reference refuses
  (D125): one through `use` and one through `bund.eval-file`.
  **In no row does Bund2 ask a listener the reference does not ask, in none
  does it ask one and then fail, and in none do the two fetch different
  text.** That sentence is about 741 rows, on one libcurl, on one system.
  It was written of 278 at the thirteenth review and of 443 at the
  fourteenth, was true of the rows both times, and was false of the fetch
  both times: in 70 of the first 159 rows the fourteenth
  review's cases added, Bund2 did something the reference did not do, other
  than refuse; and of the first 94 responses the fifteenth's question
  added, the two read 33 differently. **The honest form is the
  preservation row's: in no *measured* row.** *(Until
  the thirteenth review this said "All 175 agree on where the request goes
  but two". The script recorded only HTTP requests, so a `fail` that had
  reached a listener in another protocol read the same as one that reached
  nobody, and two rows that reached port 80 read as failures because
  nothing was listening there. Until the fourteenth it kept a request's
  first line and dropped its headers, so two requests with different
  credentials read the same. Until the fifteenth it sent one shape of
  answer in all but eight rows. And until this revision it said "In 54 of
  them Bund2 refuses and asks nobody, where the reference asks somebody",
  of a count that included `tftp://`.)*

  **In how a proxy is asked one difference remains, an approved deviation
  — D124, 2026-10-09.** *(Two until D131.)*

  1. **Gone, 2026-10-09: the request no longer differs on the wire.** The
     reference asks the proxy for the URL, `GET http://…/lib.bund`. Bund2
     asked it for a tunnel, `CONNECT`, then `GET /lib.bund`, because `ureq`
     3.4.0 offers no other form
     (`src/unversioned/transport/connect.rs:107`), and D124 approved that.
     Bund2 writes the request itself now (D131) and asks for the URL. Row
     443, a listener that answers `CONNECT` with 403 and `GET http://…`
     with a program, had its program evaluated by the reference and failed
     Bund2's fetch; it is the same cell in both now. In either binary a
     proxy's own answer to the `GET` is evaluated whatever its status.
  2. **A proxy that is not plain HTTP fails the fetch.** libcurl speaks SOCKS
     to `socks5://…` and TLS to `https://…`: a listener named that way
     received a SOCKS5 greeting and a TLS hello from the oracle, and nothing
     from Bund2, which speaks neither and fails the fetch. It does not fetch
     directly, which would ignore the setting. SOCKS would be Bund2's to
     write, as HTTP now is; an HTTPS proxy waits on whatever decides
     `https://`.

  And one thing the measurement cannot promise: **these are two systems'
  rules.** The reference links whatever libcurl its system has, and another
  may match differently. *(Until 2026-10-09 this said "one libcurl's" and
  "Linux was not measured".)* Linux was measured that day, by a workflow,
  `measure-fetch`, on Ubuntu 24.04 x86-64 with libcurl 8.5.0:
  `docs/measurements/fetch-linux-2026-10-09.md`, the 443 settings there
  were then. The
  reference differs from itself on macOS in 12 rows, and one kind matters:
  **a `no_proxy` entry with a leading zero is an address on macOS and not
  on Linux**, so there Bund2 went directly where the reference asked the
  proxy, in seven rows. Ruled the same day, D130: Bund2 reads such an entry
  as its system does. The other five are a non-ASCII host and two IPv6
  spellings that the reference refuses on Linux, as Bund2 does everywhere
  (rows 242, 304, 305); a connection refused at once where macOS waits
  (140); and the version libcurl announces (266). *(Until the fifteenth
  review's S3 this named three of the five.)*
  **The fix was measured on Linux too**: the workflow ran again on the
  commit that made it, the seven rows go through the proxy, and no other
  cell changed; the table in the repository is that run. No other system
  was measured at all; one that is not macOS gets Linux's rule, which is
  glibc's by one run. The rule is chosen when Bund2 is compiled, where the
  reference's is its C library's when it runs, and musl, the BSDs and
  Windows are unmeasured (D130's note).

  *(Until 2026-10-10 this paragraph said that since D131 Linux had not
  been measured at all.)* **Linux was measured again on 2026-10-10**, run
  38026701887 of Bund2 at `6c84d9e`, all 741 settings. In 686 rows the two
  binaries' cells are identical; in 37 Bund2 refuses and asks nobody, and
  two are D125's file, as on macOS. **In 16 the two differ on Linux and
  agree on macOS**, all among the rows about the response and the
  request's length: the reference links libcurl 8.5.0 there, and it reads
  an answer framed as chunked more than once, an answer cut short in two
  places, and a request of 1,048,575 bytes otherwise than 8.7.1 does
  (F188). Bund2 is 8.7.1's reading on both. In seven of the 16 Bund2 fails
  where the Linux reference fetches, in five the text differs, and **in
  four Bund2 fetches where the Linux reference fails**: the empty string
  once and a body three times. **D133 approves all 16: one reader on every
  system.** Only these two libcurls were measured.

  `url`, `url.`, `file`, `file.`, `bund.eval-file` and `bund.eval-file.`
  share `fetch_uri` and all of this: **the fetch has eight words.** *(Six
  until the sixteenth review's S1, and three before the fourteenth. D54
  says the reference fetches "in three places" and there are four: the
  fourth is `reference/Bund/src/stdlib/functions/bund/bund_eval.rs:65`.)*
  `bund.eval-file` takes a bare path as `file` does and evaluates as `use`
  does, so every row about a `file:` path is about it too; thirteen paths
  were measured for it, rows 721 to 733, and the two binaries agree in all
  but the literal space. It is the
  interpreter's behaviour — `bund2 script` does the same — and a bundle
  inherits it. A proxy with no port has its own:
  `a_proxy_that_names_no_port_is_asked_on_1080`, which needs port 1080 free.
  When it is not, the test passes without checking the wire and writes a
  line saying so straight to standard error; until the thirteenth review's
  S6 it used `eprintln!`, which the harness discards for a passing test.
  `a_url_is_rewritten_as_libcurl_sends_it` has the port, the host, the
  credentials and the path, and `fetch_takes_file_urls_by_curls_rules` a
  `file:` path through a link. Three more tests beside the code:
  `the_proxy_is_the_one_libcurl_would_take`,
  `no_proxy_matches_as_libcurl_matches` and
  `a_fetch_goes_through_the_proxy_and_only_the_one_libcurl_reads`. A fourth
  sets the variables on a spawned binary:
  `a_fetch_obeys_the_proxy_variables_the_reference_obeys_and_no_others`, in
  `crates/bund2-cli/tests/fetch_proxy.rs`. Two beside it count what
  listeners received: `a_url_the_reference_refuses_reaches_no_listener`,
  for hosts and credentials the reference refuses or cannot use, and
  `a_request_carries_what_the_reference_sends`, for the headers. A third,
  `a_proxy_is_asked_as_the_reference_asks_it`, holds a whole request to a
  proxy against the oracle's. **The response has eight tests of its own**,
  beside `read_response` in `crates/bund2-stdlib/src/http.rs`, each a table
  of answers the oracle gave: `a_status_line_is_read_as_libcurl_reads_it`,
  `a_status_decides_whether_a_body_follows`,
  `a_length_is_read_as_libcurl_reads_it`,
  `a_head_that_stops_short_is_the_empty_string_or_a_failure`,
  `a_header_line_is_kept_or_refuses_the_answer`,
  `chunked_is_named_or_the_body_is_taken_as_it_arrives`,
  `chunked_framing_is_taken_off_as_libcurl_takes_it_off` and
  `a_request_is_written_as_libcurl_writes_it`. No golden
  can reach a network, **and no acceptance criterion covers the fetch**: it
  is the interpreter's and not this RFC's artefact, and its instrument is
  `docs/measurements/fetch.py`. A person runs that on macOS. A workflow,
  `measure-fetch`, runs it on Ubuntu on a push that touches the script,
  `host.rs` or `http.rs`; it gates nothing and uploads the table for a
  person to read. *(Until the fifteenth review's S3 this said "nothing runs
  that but a person", after the workflow existed.)*
- A string with no scheme is refused, and `https://` is refused — both approved
  deviations under D54, the second because a TLS stack compiles C or assembly
  that D10 does not allow below `bund2 build`.
- **Every other scheme is refused too — an approved deviation, D126,
  2026-10-09; the thirteenth review's B2.** The reference fetches whatever its
  libcurl speaks. Measured: it evaluated what a listener returned for
  `gopher://`, `dict://` and `telnet://`, and for `ftp://` through an HTTP
  proxy named by `ftp_proxy` or `all_proxy`. Bund2 refuses each and asks
  nobody. D54's "Anything else fails" is Bund2's rule, and until D126 it was
  approved for `https://` by name and for nothing else.
- **Some hosts are refused where the reference fetches — an approved
  deviation, D127, 2026-10-09; the thirteenth review's S4.** The ruling is
  a rule and not a list, and since D131 the rule is narrower: nothing reads
  a URL after `host.rs`, so a spelling is refused only where `host.rs`
  refuses it. What it refuses and libcurl takes: `%xx` in a host, a URL's
  or a proxy's; an IPv6 zone; an IPv6 address Rust's parser refuses,
  `[00000::1]`; and a host that is not ASCII. On Linux the reference
  refuses the IPv6 spellings and the host that is not ASCII as well. Each
  fails closed. A spelling on which Bund2
  fetches and the reference does not, or asks another listener, is outside
  the rule and is a defect — the fourteenth review found five kinds and
  the fifteenth one more, all fixed above. **And removing `http::Uri` made
  one**: this bullet listed "a URL longer than 65,534 bytes", which was
  that parser's limit and not libcurl's. With it gone Bund2 had no limit,
  and fetched a URL of eight megabytes that the reference refuses. Found
  while this revision was written, by checking the sentence, and fixed the
  same day: libcurl's two limits are in the table above and in rows 734 to
  741. *(Until D131 this bullet also
  listed `<`, `>` and a backtick in a path or a query, `|` in a host, and
  `<`, `|` or a byte above ASCII in a proxy's user part. `http::Uri`
  refused those and Bund2 sends them now, as the reference does: rows 208
  to 211, 290, 410 to 412 and 442. The fifteenth review's S2 found three
  spellings of a proxy inside the rule, which are fixed and in the table
  above.)*
- **A proxy whose credentials carry a `%xx` escape failed the fetch — D128,
  2026-10-09 — and no longer does.** `ureq` sent a proxy's credentials as
  its URL spells them and took a proxy only as a URL, so Bund2 could not
  send `a@b`, and the owner ruled that such a proxy fails the fetch. Bund2
  writes the header itself since D131 and sends them decoded, as the
  reference does. **One case is left, and it is the reference's defect**:
  for an escape below `0x20` the reference does not use the proxy and
  fetches from the origin, silently (F185). Bund2 fails that fetch and
  asks nobody.
- **Bund2 reads the response itself — D131, 2026-10-09; the fifteenth
  review's B1 and the sixteenth's B2.** D129 had approved Bund2 failing two
  shapes of response `ureq` would not read. There were 33 of 94, and they
  ran three ways (F187): in 17 Bund2 failed where the reference fetches; in
  6 it fetched a body the reference's fetch refuses; and in 10 both fetched
  and the text differed — a `301` or `302` with no `Content-Length` was the
  body in the reference and **the empty string in Bund2, with no report**,
  so `use` loaded nothing and the program went on. Nothing approved the
  last two, and no setting of `ureq`'s changes any of it. The owner's
  ruling was that Bund2 read HTTP itself. `read_response` states each rule
  beside the code; the ones a program is likeliest to meet:

  | | as the reference's libcurl reads it, and Bund2 now |
  |---|---|
  | a redirect | is its own body, with a length or without: no redirect is followed |
  | `204` and `304` | have no body, whatever they say of one |
  | a status from 100 to 199 | is not the answer, and another head follows; `101` is, and its body runs to the end of the connection |
  | the status line | `HTTP/1.0`, `HTTP/1.1` or `HTTP/2` in upper case, then three digits; another version in upper case is refused, and so is `HTTP/3` written that way. A line that begins `HTTP/` in another case, or `HTTP/2` or `HTTP/3` without three digits after one blank, is status 200 whatever it says |
  | `Content-Length` | the last one read; `+17` and `17abc` are 17, a number past 63 bits is no length, anything else is a failure |
  | `Transfer-Encoding` | `chunked` is taken off once for each time it is named, up to four; the list is read up to the first coding that is not `chunked`, so `gzip, chunked` is taken as it arrives, framing and all |
  | chunked framing | a size is one to sixteen hex digits and nothing before them; a line ends at its line feed; a trailer has a colon |
  | a head | a line of at most 102,399 bytes, and 307,200 bytes in all |
  | an answer that ends inside its head | the empty string once a whole line has arrived, unless a `Content-Length` above zero was among them |

  **What this is and is not.** It is a model of libcurl 8.7.1's reader,
  checked on 233 shapes, every one of which both binaries now read alike
  (rows 444 to 694). It is not libcurl's reader. An answer that combines
  the rules in a way nobody tried is read by the same rules, and whether
  libcurl agrees there is not known — **in either direction**. D129's
  "Bund2 fails" was a property of `ureq`, and this reader has no such
  property to offer. Three things libcurl does and Bund2 does not: it
  tries a name's addresses side by side, where Bund2 tries them in turn;
  with `chunked` named more than once it stops with the piece of the
  connection in which a layer ended, where Bund2 reads to the end of the
  outermost layer and so may refuse what follows; and neither sets a limit
  on how long an answer takes, which was read and not compared.

- **A fetch that fails on a very long URL is slow to say so — F186, a
  defect outside this RFC, and not fixed.** The fetch fails at once. The
  report quotes the operand, and the line of source the word stands on, and
  takes time that grows with the square of either: 11 seconds for 80,000
  bytes on a debug build. `http::Uri` refused a URL past 65,534 bytes;
  since D131 one of up to eight megabytes reaches the report.

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
§B5's blocking items — symbol-addressed slot tables and cells, planning
without a live VM, deciding which bodies compile when no word is registered,
and the fourth that D83 added, guards with no cells at build time — plus the
reopened criterion 30 mirrors. **That trade is the
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
| `args` / `args.parse` | **Preserved for the arguments, changed for the separator.** All of argv reaches the program; the runner consumes nothing (§B3). So a literal `--`, which `script` consumes, is an argument to a bundle. |
| Exit code | **Preserved** — `vm.exit_requested()`. |
| Diagnostic flags (`--stats`, `--no-dump-stack`, `--raw-values`, `--jit-threshold`) | **Deliberately changed.** Not argv flags in a bundle, because they would shadow the program's own arguments; they move to environment variables. |
| `--debugger` | **Deliberately absent.** No flag and no variable: a bundle is never given a debugger (D115, and §B3). The debuggable form of the program is `bund2 script --file`. |
| `BUND2_NOIO`, `BUND2_NOEVAL` in the environment | **A bundle's alone.** `script` does not read them, so one environment can restrict the bundle of a program and not its `script` run (§B3). |
| `use`, `use.`, `url`, `url.`, `file`, `file.`, `bund.eval-file`, `bund.eval-file.` | **Not preserved whole: Bund2 fetches `file:` and `http:` and the reference fetches whatever its libcurl speaks.** For those two schemes, in the 741 settings measured and no further, it is preserved (F182, F183, F187, D131): the proxy is the one the reference's libcurl would choose, on its default port, and is asked as libcurl asks it; the URL's host, credentials and path are sent as libcurl sends them, in a request written line for line as the oracle's; a `file:` path loses its dot segments before the system resolves it; and the response is read by rules measured on 233 shapes of answer. Approved differences: no scheme and `https://` refused (D54); a SOCKS or HTTPS proxy failing the fetch (D124); a path with a literal space or control character read (D125, F184); every other scheme refused (D126); a host with `%xx`, a zone or a byte above ASCII in it refused (D127). And one that follows a defect of the reference's instead of reproducing it: a proxy whose credentials decode to a control byte fails the fetch, where the reference goes direct (F185). In all of these but D125's Bund2 fails closed, and in no measured row does it ask a listener the reference does not or fetch other text (`docs/measurements/fetch-2026-10-09.md`). **Preserved per system**: the reference links its system's libcurl and differs from itself between the two systems measured, and Bund2 follows it where that was found (D130). **The Linux measurement is of the tree before D131** and has none of the rows about a response. **What the measurement did not try is not claimed**: §B7 lists each part of what is exchanged and who reads it, and says of the response's reader that it is a model. *(Until the sixteenth review this row was headed with six of the eight words. Until D131 it listed a tunnel asked of a proxy (D124), a proxy whose credentials carry an escape failing the fetch (D128), and two shapes of response failing it (D129); the first is gone and the other two are superseded.)* *(Until the fourteenth review this row was headed with three of the six words, and `file` is where a path through a link matters most.)* *(This row said "Preserved exactly" until the eleventh review and "Preserved under D54's scheme set" until the thirteenth, when D54's set was Bund2's and not the reference's. Bund2 obeyed `HTTP_PROXY`, `HTTPS_PROXY` and `https_proxy` until F182, asked a port-less proxy on 80 until the twelfth review, refused `HTTP://` until F183, and fetched `:65536` from port 80 until the thirteenth. Until the fourteenth it asked a proxy for a host the reference refuses, sent credentials undecoded, read three kinds of `no_proxy` entry differently, and left `..` in a `file:` path to the system.)* |
| `--noio`, `--noeval` | **Deliberately available as a build-time floor — D78.** Recorded in the trailer; run time may add either and never remove one. Not a boundary: §B3a names what each gates and what each leaves ungated — for `--noio` that includes two words that write a history file, standard input, and the host's address and name. *(Until D132, 2026-10-09, it also included `use` and `bund.eval-file`, and this row's list did not say so.)* |
| `use`, `use.`, `bund.eval-file`, `bund.eval-file.` under `--noio` | **Approved deviation — D132.** The reference stubs the four under `--noeval` alone, so under its `--noio` a program fetches a URL or reads a file and runs it. Bund2 stubs them under `--noio` too. Without the flag, preserved; with both flags the message is `--noeval`'s on both binaries. |
| `csv` and `sqlite` under `--noio` | **Approved deviation — D119.** The reference leaves both ungated; Bund2 stubs them and their conditional handlers. Without the flag, preserved. |
| `--nocolor` | **Preserved as a per-run choice**, in the environment-variable channel with the diagnostic flags. Presentational, not a capability. |
| Diagnostic file name | **Preserved for a path of up to 256 bytes** via the trailer's recorded source path (§B3), without which a bundle's report differs by construction and the CEILING moves. **A longer path is changed**: its last 256 bytes are kept, and the build says so. |
| A program larger than 1 MiB | **Changed: it runs under `script` and cannot be bundled.** Refused at build with both numbers named (§B1, criterion 11). The limit is not discoverable before it is hit except by `--inspect`, which prints the capacity. |
| The debugger's history files in a shipped artefact | **New surface, and ungated.** A bundle whose program calls `debug.shell` or `debug` writes line history under the configuration directory of whoever runs it (§B3a). |
| `bund2` itself | **Changed.** Every `bund2` carries the 1 MiB region and reads it before it acts on argv, so a damaged region stops the plain interpreter too (§B1). |
| `bund2 build`'s arguments | **Refused when not understood** — `--emit=native`, `--features`, a repeated `--file`, anything unknown (§B3, "The command line"). |
| What is at `--output` | **Replaced without a word**, unless it is the building binary or the program being built, each of which is refused (§B3). New surface, stated and not defended. |
| `--output` that is another name for an existing file | **The name is replaced, the file is not — F181.** A hard link to the builder, or to anything else, becomes the artefact and every other name keeps its bytes. A symlink is followed, dangling or not. A failed build leaves `--output` as it was. |
| `decode.base64` under either flag | **Preserved — D123.** A string becomes a lambda and `!` runs it, under `--noeval --noio`, on both binaries; each runs the other's string (§B3a). |
| `load`, `load.lambdas`, `load.stacks` under `--noeval` alone | **Preserved.** They bring runnable values in from a world file; only `--noio` stubs them (§B3a). |
| A bundle asked to `build` | **It cannot.** A bundle owns all of argv, so `build` is its program's first argument (§B3). |
| A syntax error's timing | **Deliberately changed**: found at build rather than at run (§B3). A build that wrote an unparseable program would move the error to whoever ran it. |
| RFC-0005 criterion 30's excluded mirrors | **Reopened by `--emit=native`**, on that row's own stated trigger. Owed once the mode exists, not excluded. |
| What `--noeval` stops | **Preserved exactly for the reference's words — D79.** It disables the `bund.eval` group, six words: `bund.eval`, `bund.eval.`, `bund.eval-file`, `bund.eval-file.`, `use`, `use.`. `compile` is not in the group, so `compile lambda! !` still evaluates, on both binaries. So do `debug`'s operand (D122), `decode.base64` (D123), the world-file words under `--noeval` alone, and code built from strings. §B3a names each and says why the list is open. |
| Code built from strings, under either flag | **Preserved — D16, D79.** `make.call`, `ptr`, `lambda*`, `lambda!` and `!` on a string are ungated on both binaries; four programs print `42` under `--noeval --noio` (§B3a). A hermetic golden runs the idiom. |
| `--output` that is the program's own source | **Refused**, by where the path leads (§B3). New surface: until 2026-10-08 the source was replaced by its artefact and the build exited 0. |
| A dangling symlink at `--output` | **Followed**: its target is created and the link stays (§B3). |
| What `--output`'s directory must allow, and what may be left in it | **Changed by F181.** The directory must be writable, where a writable file used to do; the replaced file's mode and ownership are not kept; a killed build can leave `.bund2-build-<pid>/` (§B3). |
| `debug.run` under `--noeval` | **New surface, decided — D120.** Bund2's own word, which evaluates a string; the flag stubs it. Not a deviation, the reference having no such word. |
| `debug.feed` under `--noeval` | **New surface, decided — D121.** Bund2's own word; the flag stubs it, because before `debug.shell` it ran a line nobody typed. Not a deviation. Under the flag it cannot feed `input` either. |
| `stdin`, `stdin.` under `--noio` | **Preserved.** Stubs that exist only under the flag, in the reference and in Bund2 (§B3a). |
| `fs.is_file.`, `fs_is_file.` under `--noio` | **Preserved, both halves — F150.** The first is a stub that exists only under the flag; the second is a word without the flag and not registered under it (§B3a). |
| `debug`'s operand under `--noeval` | **Preserved — D122.** `"40 2 + println" debug` prints `42` under both flags with standard input closed, on both binaries. The word evaluates its string before it reads a line (§B3a). The owner ruled it stays, named. |
| `bootstrap` under `--noeval` alone | **Preserved.** It evaluates the scripts a world file holds; only `--noio` stubs it (§B3a). |
| `debug.run`, `debug.feed` under `bund2 script --noeval` | **Changed outside artefacts — D120, D121.** Both stubs hold in a `script` run as in a bundle, so this RFC moved what the plain interpreter does to two words RFC-0008 added. |
| A breakpoint condition under either flag | **Fixed — F180.** It ran in a child VM with neither restriction; the child now has the program's. A `script`-mode change; a bundle evaluates no condition (D115). |
| A damaged or empty region | **New surface, specified.** An empty region is the plain interpreter; five kinds of damage are errors, none a panic (§B1, criterion 12). The fifth, since 2026-10-09: a restriction bit this bund2 does not define. |
| A flags byte with a bit this bund2 does not define | **Refused, and never shown as "none".** The artefact is not run, and `--inspect` names the unknown bits. Until 2026-10-09 it inspected as `restrictions none` and ran unrestricted. No builder writes such a byte, and the runtime is the builder (D118), so at run time this is a patched or damaged file; at `--inspect` it may be a later bund2's artefact. *(Headed "A restriction a later bund2 defines" until the twelfth review's S3.)* |
| Bund2's own version | **Recorded in the trailer, and read by `--inspect` only.** The pinned SHAs name the oracle, not the interpreter. No skew can occur, because the runtime is the builder (D118), and the runtime does not check the field. **It reads `0.0.0` for every build today** (§B1), so the row is true of the mechanism and tells a reader nothing yet. |
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
  built JIT-first. What remained was a size saving, since weighed at 12.45%
  of the binary (criterion 1), against a cost §B5 shows is larger than
  assumed. D83 withdrew the mode on that trade.
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

   **Run again on 2026-10-08: 133/145, ceiling 133/145, twelve approved
   deviations, none failing, in every configuration** —

   | run | conformance | ceiling | failing |
   |---|---|---|---|
   | source, default | 133/145 (+12 approved) | 133/145 | none |
   | bundles, default | 133/145 (+12) | 133/145 | none |
   | source, `--features jit` | 133/145 (+12) | 133/145 | none |
   | source, `jit` at threshold 1 | 133/145 (+12) | 133/145 | none |
   | bundles, `--features jit` | 133/145 (+12) | 133/145 | none |
   | bundles, `jit` at threshold 1 | 133/145 (+12) | 133/145 | none |

   The corpus has grown since the first table; the claim has not changed.
   *(Until later the same day this paragraph gave the first four rows only:
   the two JIT-bundle rows, which are what the criterion asks for, had not
   been run at 145 goldens — the eighth review's S9. In bundle mode the
   threshold reaches each artefact through `BUND2_JIT_THRESHOLD`
   (`xtask/src/conform/mod.rs`, `run`), so the last row is its own run.)*

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
   `a_jit_artefact_contains_the_code_generator` does, under `--features jit`,
   **for both needles since the seventh review** — it searched for
   `cranelift` alone while the absence check also searches for `ISLE`.
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

   **How to run it: `cargo test -p bund2-jit --features jit,aot relocation`
   — stated 2026-10-09, the eleventh review's S1.** The two tests are gated
   on `aot` inside a module gated on `jit`
   (`crates/bund2-jit/src/lower.rs`, `mod relocations`;
   `crates/bund2-jit/src/lib.rs`, `mod lower`). Under `--features aot` alone
   the crate compiles no tests and the command reports `ok` with nothing
   run, which is what this RFC's table told a reader to do. Measured by the
   review: `aot` alone, 0 passed and nothing filtered; `jit,aot`, 2 passed.
   Run again 2026-10-09 with `--all-features`: 2 passed.

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
    still refuses each of the group's six words, `debug.run` (D120) and `debug.feed` (D121), and one built `--noio` still
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
    floor with `fs.cwd`, in both directions. **`debug.run` joined the list
    with D120, and `debug.feed` with D121.**

    **Widened 2026-10-09, D132** — `--noio`'s floor is tried with `use`,
    `use.`, `bund.eval-file` and `bund.eval-file.` as well, in the same
    test. The sixteenth review measured a bundle built `--noio` evaluating
    a remote library, and `fs.cwd` could not have shown it.

    **It checks the stubs and nothing more, deliberately.** A criterion that
    claimed more would be false: `"40 2 +" compile lambda! !` prints `42` under
    `--noeval` on both binaries (§B3a), so no criterion here may be read as
    "the artefact evaluates nothing". Nor do `"…" debug`, `decode.base64`, code built
    from strings with `make.call`, `ptr`, `lambda*` and `!`, and, under
    `--noeval` alone, `bootstrap` and the `load` words: none is a stub, and
    this criterion cannot see them. For `--noio` what it cannot see is what
    §B3a lists as ungated: standard input, the host's identity, the history
    files. *(Widened 2026-10-08, the tenth review's
    S4; it had the list as it stood two reviews earlier.)*

    **A refusal is text and not a status — stated 2026-10-08, the eighth
    review.** A program stopped by a stub prints its report and exits 0, in a
    bundle and under `script`, as every failing program does (the
    preservation table's exit row). The tests here read the text. A wrapper
    that checks `$?` sees a restricted artefact succeed.
11. **A produced artefact still validates where signing is enforced.**
    `codesign -v` on what `bund2 build` wrote reports no error on macOS, after
    an ad-hoc re-sign, which Q40's measurements show is possible for an
    in-image payload and impossible for an appended one. A program over the
    reserved capacity is refused at build with the capacity and the size named.

    **Met, 2026-09-30** — `a_built_artefact_validates`, plus
    `a_program_over_capacity_is_refused_and_writes_nothing`, which asserts the
    error names both numbers **and that the image is unchanged**. That one is
    a unit test of `write_into` on a buffer (`crates/bund2-cli/src/bundle.rs`).
    **Through the binary since 2026-10-08**:
    `a_program_over_capacity_fails_the_build_and_writes_nothing` builds a
    program one byte past 1 MiB and finds exit 1, both numbers, and no file.
12. **A damaged artefact is refused, not aborted.** The first four of §B1's
    kinds of damage — an unknown container version, a state byte that is neither value,
    a length past capacity, a payload that is not UTF-8 — produce a diagnostic
    and an error status, and none reaches a panic; an empty region is the
    plain interpreter. **A fifth since 2026-10-09 — the eleventh review's
    S2: a restriction bit this bund2 does not define.** Both readers tested
    the two bits they knew and passed over the rest, so an artefact built
    `--noeval` with its flags byte patched from 2 to 4 inspected as
    `restrictions none` and ran unrestricted. No builder writes such a byte,
    and under D118 the runtime that reads a flags byte is the binary that
    wrote it, so the case is a byte patched or damaged after the build —
    which is what the test does. The runtime now refuses to run it. What can
    meet a later bund2's flags is an earlier bund2's `--inspect`, and that
    names the bits it does not define rather than printing "none"
    (`an_unknown_restriction_is_not_inspected_as_none`); every bund2 before
    2026-10-09 still prints "none" for them. *(Until the twelfth review's S3
    this gave the reason as an old runtime meeting a new restriction, which
    D118 rules out.)* *(Until 2026-10-08 this named the appended form's
    cases, one of which — a truncated trailer — cannot occur in a fixed
    region.)* D37,
    and the one input a bundle's front end will certainly meet.

    **Met** — `a_damaged_artefact_is_refused_with_an_explanation` drives all
    five through the binary: a container version of 99, a `state` byte of 7,
    a length past capacity, a lone continuation byte in the payload, and a
    flags byte of 4. The
    first three have passed since 2026-09-30 and the container version since
    2026-10-08. The empty region, which is not damage, is
    `an_unbuilt_runtime_carries_nothing` — a runtime nobody built from is the
    plain interpreter. Each damaged artefact is **re-signed before it is
    run**, or macOS kills it before the runtime can report and the test would
    pass for the wrong reason. *(Until the seventh review this paragraph
    called two different things "the fourth".)* This criterion is about
    *running* a damaged artefact; inspecting one is §B3a's stated limit.
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
| 7 | `bund2-jit`'s relocation tests, under `jit` **and** `aot` together | **no** |
| 11 | `bundle_build`, and a unit test in `bundle.rs`; the signing half is macOS only | the capacity half only |
| 13 | a before-and-after `conform` on 2026-09-30 | no |

`cargo xtask bundle`, which found both release-only blockers, is in neither
`cargo test` nor CI — D82's fourth unsettled item, still unsettled. Each "no"
was last run by hand on the date its criterion gives.

**Each "yes" means "would", and none has happened — the seventh review's
S7.** The workflow runs on a push to `main` and on pull requests
(`.github/workflows/ci.yml:3-6`). Its one recorded run is on `main`,
2026-09-11, and failed; `bundle.rs` did not exist until 2026-09-30. Every
criterion here has been run on one machine, macOS on arm64, by hand or by
`cargo test` there.

## Open questions

- **The §B2 deviation — answered by D77**, 2026-09-29: embed source text, and
  the encoded container is closed rather than deferred. Left listed so a reader
  tracing D77 finds the question it answers.
- **Q38 — answered by D76**, 2026-09-29: fetch at run time.
- **A flag that embeds `use` targets.** Deferred by D76 with its trigger and
  form recorded. Nothing is blocked on it and no default waits to be adopted.

- **What a bundle may switch off — answered by D78**, 2026-09-29: a
  build-time floor in the trailer that run time may only tighten, and the word
  "sandbox" is ruled out. *(Until 2026-10-08 this bullet gave D78's two
  reasons as D78 words them — that `--noio` leaves `args`, `sleep.seconds`
  and `io.graph` ungated "and does not gate fetching at all" — both of which
  §B3a corrects: the three are one module's words, and `--noio` does gate
  fetching through `url` and `file`.)*
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
- **Does `--noeval` gate `debug.run`? — answered by D120**, 2026-10-08: yes.
  Raised the same day by the seventh review (B1). It is Bund2's own word, so
  no deviation.
- **Does it gate `debug.feed`? — answered by D121**, 2026-10-08: yes. Raised
  beside the question above, since the word let a program hand `debug.shell`
  its own line.
- **Does `--noeval` gate `debug`'s operand? — answered by D122**,
  2026-10-08: no. Raised the same day by the eighth review (B1). The
  reference's `debug` evaluates the string it is given, under both flags;
  Bund2 preserves that and §B3a names it.
- **Does D122's ruling cover `decode.base64`? — answered by D123**,
  2026-10-08: yes. Raised the same day by the ninth review (B1). The word
  decodes a string to a value, a lambda included, under both flags, in the
  reference and in Bund2; it stays, and §B3a names it.
- **How should the fetch treat a proxy? — answered 2026-10-09 by the
  owner: a defect, fixed.** Raised the same day by the eleventh review (B1).
  F182 reads what the reference's libcurl reads (§B7).
- **Are the fetch's two remaining proxy differences approved? — answered
  by D124**, 2026-10-09: yes. Bund2 tunnels where the reference asks for the
  URL, and fails a fetch through a SOCKS or HTTPS proxy where the reference
  uses it (§B7). They are the third and fourth deviations for the fetch
  under D54. `ureq` as built can do neither; its `socks-proxy` feature would
  close the SOCKS half at the cost of a dependency. *(Since D131 there is no
  tunnel and no `ureq`. The SOCKS and HTTPS half stands.)*
- **Should a `file:` URL with a literal space be read? — answered by
  D125**, 2026-10-09: yes, an approved deviation. The reference refuses it,
  so `"/a b.txt" file` fails there (F184); Bund2 reads it. Found the same
  day by the measurement, not by a review.
- **Is refusing every scheme but `file` and `http` approved? — answered by
  D126**, 2026-10-09: yes, the sixth deviation for the fetch. Raised the
  same day by the thirteenth review (B2).
- **Are four spellings of an `http:` URL, refused where the reference
  fetches, approved? — answered by D127**, 2026-10-09: yes, as a rule —
  Bund2 fetches what it and `http::Uri` both accept. Raised the same day by
  the thirteenth review (S4). *(Narrower since D131: what `host.rs`
  refuses of a host, and nothing else.)*
- **What does Bund2 do with a proxy whose credentials carry a `%xx`
  escape? — answered by D128**, 2026-10-09: it fails the fetch. The
  reference decodes them, and for a control byte uses no proxy at all
  (F185). Raised the same day by the fourteenth review (B2). *(Superseded
  by D131: Bund2 decodes them and sends them. F185's case alone still
  fails.)*
- **Are two responses `ureq` will not read approved as failures? —
  answered by D129**, 2026-10-09: yes. Raised the same day by the
  fourteenth review (S3). *(Superseded by D131.)*
- **How is a `no_proxy` entry with a leading zero read? — answered by
  D130**, 2026-10-09: as the system Bund2 is built for reads it. Raised the
  same day by the first measurement on Linux. *(This bullet was missing
  until the fifteenth review's S3.)*
- **Who reads a response? — answered by D131**, 2026-10-09: Bund2, by
  rules measured against the oracle; `ureq` is removed. Raised the same day
  by the fifteenth review (B1). It ends the tunnel and supersedes D128 and
  D129.
- **Does `--noio` gate the words that fetch and run? — answered by D132**,
  2026-10-09: yes, an approved deviation, as D119 was. Raised the same day
  by the sixteenth review (B1).
- **Does Bund2's reader of responses agree with the reference on Linux? —
  answered by D133**, 2026-10-10: not in 16 rows of 741, and that is an
  approved deviation there. *(Until that day this was open, awaiting the
  measurement.)* The reference links libcurl 8.5.0 on Ubuntu and differs
  from itself (F188); Bund2 keeps the one reader, 8.7.1's.
- **What is the `aot` feature called now? — Q42.** After D83 it compiles the
  relocation test and nothing else, a bundle built from such a `bund2`
  inspects as `features: aot`, and CLAUDE.md's terminology still defines AOT
  as "the cranelift-object build". Not this RFC's to rename alone.
- **Is the runtime the builder, or a prebuilt runtime per target? — answered
  by D118**, 2026-10-08: the builder. Raised the same day by the sixth review
  (B4). Cross-target bundling is deferred with its trigger recorded in §B1.
- **Should `--noio` gate `csv`, `sqlite` and the debugger's history file? —
  answered by D119**, 2026-10-08, for the first two: yes, an approved
  deviation. The ruling does not name the history file, so it stays ungated
  as the reference leaves it, and §B3a says so.

**The first three reviews found six blockers between them, and all six are
answered** — four in the design, the rest by D76, D77, D78, D79 and D80.
Fifteen reviews of this document and one of the code have found
thirty in all — thirty-one if the implementation review's second-pass
B3 is counted, which this sentence never has — and each is answered or
ruled. *(Thirteen and twenty-eight until this revision. The sixteenth
review's second blocker is the fifteenth's, reproduced, and is counted
once.)* Of
the last nine, eight are the fetch: which variables name a
proxy, a proxy's default port, a port that is not a number, the schemes
Bund2 refuses, then a host, a `no_proxy` entry and a `file:` path that
were handed on unread, and then the answer, which nobody read. The ninth
is `--noio`'s list, which had lost the route D78 was ruled on.
*(Until the eighth review this sentence said seven and eighteen when the
document had had six.)* What
remains listed is one question for whoever takes §B8's gate, and Q42. The
fourteenth raised two, and they are D128 and D129. The fifteenth raised
one, D131, and the sixteenth one, D132. One question that no review
raised, what the reader does on Linux, was measured on 2026-10-10 and is
D133.
The sixth review
on 2026-10-08 raised two more for the owner and both are ruled: which
construction §B1 means is D118, and how far `--noio` reaches is D119. The
seventh raised one, and it is D120, with D121 beside it. The eighth raised
one, and it is D122. The ninth raised one, and it is D123. The tenth raised none: its blocker is
D79's own sentence, applied. (Until
2026-10-08 this sentence also counted parse-at-build, since ruled as D116,
and Q40, which the bullet above records as answered by measurement.) **No
default is being adopted by omission** — stated carefully, because the second
revision of this section claimed exactly that while two blockers were
outstanding.
