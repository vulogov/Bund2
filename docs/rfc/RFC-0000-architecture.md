# RFC-0000: Architecture, crate boundaries, and the tier model

- Status: **Accepted** (2026-08-25)
- Depends on: —
- Accepted against: B1 conform 0/63; B2 coverage 121/497; B3 `bund2-stdlib`
  free of `bund2-jit`; B4 six clean submodules; B5 `cite` 0 defects over 1059
  citations with 5/5 oracle crates byte-verified. D-1 and D-2 are deferred to
  RFC-0002 and are not part of this acceptance. Four adversarial review passes
  are folded in; the fourth is the last that found anything.
- Decisions consumed: D15, D16, D20, D21, D26, D27, D28
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- Supersedes: `docs/research/04-consolidated-architecture.md` §2-§3 as the
  statement of record for layout and tiering. §1.1 is deliberately excluded:
  it mixes layout with value, symbol and lambda claims that belong to
  RFC-0001, RFC-0002 and RFC-0003, and superseding it here would take scope
  this RFC disclaims. See `docs/research/ERRATA.md`,
  which holds seven corrections — two predating this session, three from the
  corpus scan, this RFC's own supersession entry, and the `$name` correction
  from F26.

## Summary

Bund2 re-implements the Bund concatenative language against the pinned
reference in `reference/`, which is treated as an oracle rather than a
codebase to refactor. This RFC fixes the things every later RFC depends on and
nothing else: how the workspace is laid out, where the boundaries between
crates fall and why, what the execution tiers are, what the shared vocabulary
means, and how the project knows whether it is succeeding. It makes no claim
about value representation (RFC-0001), symbols or the word table (RFC-0002),
or the IR (RFC-0003).

## Motivation

The reference is six crates and roughly 32,000 lines, and its structure
encodes decisions that were never written down. Three of them are load-bearing
enough that re-implementing without stating them first would repeat them by
accident.

**The dependency graph is a clean stack, and that is worth keeping.**
`rust_dynamic` depends on nothing internal; `rust_multistack` depends on
`rust_dynamic`; `rust_multistackvm` on both; `bundcore` on the parser, values
and VM; `Bund` on all of them (`reference/rust_multistack/Cargo.toml`,
`reference/rust_multistackvm/Cargo.toml`, `reference/bundcore/Cargo.toml`,
`reference/Bund/Cargo.toml`). No cycles. Bund2 should preserve that property
deliberately rather than inherit it by luck.

**All twelve internal version pins are unbounded.** The edges are
`reference/Bund/Cargo.toml:13,14,23,24,43`,
`reference/rust_multistackvm/Cargo.toml:21,22`,
`reference/rust_multistack/Cargo.toml:14`,
`reference/bundcore/Cargo.toml:12,17,18` and
`reference/bund_language_parser/Cargo.toml:17`.

Two earlier drafts got this wrong in opposite directions, and the second error
is worth naming because it weakened the argument it was fixing.
`reference/rust_multistackvm/Cargo.toml:21` reads `">=0.33.*"`, which looks
like a bounded minor series and is not: `>=` is a floor with no ceiling.
`reference/Bund/Cargo.lock` resolves `rust_dynamic` **0.49.0** under exactly
that spec. There is no exception edge — a `Value` layout change propagates to
every dependent, across all twelve, without a version bump. Recorded as F8,
and the monorepo resolves it for Bund2 — but see the Terminology note on the
oracle, where the same mechanism is live inside this project's own
methodology.

**The binary is 381 MB, and about 9.6 ms of every run is spent loading it.**
Measured by decomposition: a bare process spawn is 1.4 ms, `bund --version` —
which returns before any stdlib initialisation — is 11.0 ms, an empty program
is 15.8 ms. Interpretation itself sits below the noise floor. The cause is the
dependency set: `reference/Bund/Cargo.toml` links `lingua` (line 59),
`hyphenation` with `embed_all` (line 128), `duckdb` with `bundled` (line 97),
`polars` (line 93), `arrow` (line 101), `prqlc` (line 96), `charabia`
(line 67), `neurons` (line 57), `augurs` (line 104), `rustface` (line 114),
`imageproc` (line 115), `viuer` (line 112), `zenoh` (line 68), `dryoc`
(line 82), `reqwest` (line 129) and `deepseek-api` (line 122). Each embeds
data or a native library. D28 turns them off by default.

## Current behaviour

### Crate structure and sizes

Measured over `src/` at the pinned SHAs:

| Crate | LOC | Files | Role |
|---|---|---|---|
| `rust_dynamic` | 5,044 | 59 | the `Value` type, conversions, comparison, serialisation |
| `rust_multistack` | 2,047 | 41 | named stacks and the workbench |
| `rust_multistackvm` | 5,292 | 82 | the VM, dispatch, and the core word set |
| `bundcore` | 583 | 11 | instance construction and bootstrap |
| `bund_language_parser` | 323 | 18 | the pest grammar and its token handlers |
| `Bund` | 18,996 | 200 | the CLI and the domain-library word set |

The parser is 323 lines because the grammar does the work:
`reference/bund_language_parser/bund.pest` is 54 lines defining twelve `value`
alternatives, and each handler in `src/vm/` converts one token to a `Value`.

### Word registration is three tiers, not two

`i_direct` tries the VM's own inline table
(`reference/rust_multistackvm/src/multistackvm_inline.rs:42`) and, on a miss,
falls through to the stack layer's own table (`:52`). That third tier is
`rust_multistack`, and it holds the words programs use most —
`take` (`reference/rust_multistack/src/stdlib/workbench.rs:81`) and
`drop` (`reference/rust_multistack/src/stdlib/drop.rs:71`), used by 29 and 16
corpus programs respectively.

Full resolution order, from `reference/rust_multistackvm/src/multistackvm_apply.rs:16-60`:
command, then a `$`-prefixed name forcing an internal word (`:33`), then alias
(`:39`), then lambda (`:46`), then inline (`:59`) with the two-tier fallthrough
above.

**That chain is one arm of a branch, not the whole story.** `apply` tests
`is_command` first (`reference/rust_multistackvm/src/multistackvm_apply.rs:16`)
and only then `self.autoadd` (`:19`); when `autoadd` is set the name is
*appended to the value on top of the stack* (`:20-27`) and no resolution
happens at all. The chain above is the `else` arm of that second test. `autoadd` is toggled by the `:` and `;` commands, so a program can switch
the interpreter between resolving names and collecting them. Any statement
that "resolution order is the contract" has to say which arm it means.

**Aliases resolve twice, and exactly twice.** `apply` resolves an alias at
`:39`, then calls `i()`, which tests and resolves it again
(`reference/rust_multistackvm/src/multistackvm_inline.rs:71-72`). The second
resolution is a no-op on an already-resolved name, but it is on the dispatch
path for every call. Recorded as F6.

**There is a fourth registration table, and it is dead code.**
`register_function` (`reference/rust_multistack/src/ts_functions.rs:6`) fills a
separate `functions` map from 27 call sites across
`reference/rust_multistack/src/stdlib/`. The map is read only by
`get_function` (`:25`), called only by `TS::f` (`:36`), and `TS::f` is called
from nowhere in any of the six crates. So it is neither a second dispatch path
nor an embedding API. Recorded as F19.

Three dispatch tiers, three live tables. **RFC-0002's slot table absorbs three,
not four**, and porting the fourth because it exists would add a name space
the language does not have.

### The world cannot be closed

`ptr` casts a stack value to a string and pushes a PTR naming a word
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:80-93`); `!`, an alias of
`execute` (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:5`), hands
that name to `vm.call` (`reference/rust_multistackvm/src/stdlib/execute.rs:26-33`).
`reference/Bund/examples/bund_dynamic_demos/dynamic_demo_2.bund:29-34` builds a
word name by string concatenation and calls it, so a callee's name need not
appear anywhere in the source. `execute` also accepts a bare STRING, so `ptr`
is not even required:

```rust reference/rust_multistackvm/src/stdlib/execute.rs:27
                PTR | STRING | CALL => {
```

D16 preserves this.

That block is verified verbatim by `cargo xtask cite`. Where an RFC quotes
source rather than pointing at it, the quotation is checked line for line
against the cited file — which is the check that catches an off-by-one, and
the form to prefer for any claim that turns on exact text.

### Persistence

The world file is SQLite, holding `LAMBDAS`, `ALIASES`, `STACKS`,
`STACK_DATA`, `MODELS` and `BOOTSTRAP`
(`reference/Bund/src/stdlib/helpers/world/lambdas.rs:69`, `aliases.rs:60`,
`stacks.rs:129,187`, `models.rs:11,79`, `bootstrap.rs:179`), with whole
`Value`s stored as bincode BLOBs (`lambdas.rs:81-84`).

## Design

### Workspace layout

One Cargo workspace, twelve members, already scaffolded in `Cargo.toml`. The
mapping from the reference is deliberate rather than incidental:

| Bund2 crate | Takes over from | Boundary rule |
|---|---|---|
| `bund2-value` | `rust_dynamic` | the value type and nothing that knows about stacks |
| `bund2-syntax` | `bund_language_parser` | source text to AST; no evaluation |
| `bund2-ir` | — (new) | AST to BundIR; the only crate that defines the instruction set |
| `bund2-interp` | `rust_multistack` + `rust_multistackvm` dispatch | Tier 0; owns the stacks, the frame loop, and word resolution |
| `bund2-stdlib` | `rust_multistackvm/stdlib` + `Bund/stdlib` | words only; no VM internals |
| `bund2-api` | — (new) | the stable surface a word package compiles against |
| `bund2-jit` | — (new) | Tier 1; optional, feature-gated |
| `bund2-runtime` | `bundcore` | instance construction, bootstrap, the world file |
| `bund2-async` | — (new) | optional; reconciles with the existing bus layer per RFC-0007 |
| `bund2` | — | the façade that composes the above |
| `bund2-cli` | `Bund/src/cmd` | argument parsing and the REPL; no language logic |
| `xtask` | — (new) | oracle capture, conformance, measurement |

The rule that makes these boundaries checkable: **`bund2-value` must not
depend on `bund2-interp`, and `bund2-stdlib` must not depend on
`bund2-jit`.** The first keeps the value type usable without a VM; the second
keeps Tier 1 genuinely optional. Both are mechanically verifiable.

`reference/` stays a set of pinned submodules, read-only, built only
out-of-tree to produce goldens.

### Feature policy

Per D28, the default build enables only what the language needs. The
subsystems whose dependencies produced the 381 MB binary — `ai`, `image`,
`bus`, `forecast`, `statistics`, `internaldb` (D26), `console` (D15) — are
feature-gated and off. Nothing is deleted; a gated word can be enabled.

This is why the boundary between `bund2-stdlib` and the rest matters: a
feature that is off should remove its dependency from the build graph
entirely, which only works if no core crate reaches into it.

### Tier model

**Tier 0** is the BundIR interpreter. Mandatory, present in every target,
including AOT output. **Tier 1** is the Cranelift JIT: optional, feature-gated,
and never required for correctness. **AOT** is the `cranelift-object` build.

D16 constrains all three. Because a call target may be named by a string built
at run time, no tier may assume a closed world:

- AOT cannot tree-shake by word reachability, and its output retains the word
  table and the name resolver.
- Tier 1 cannot statically devirtualise `!`. Speculation behind a guard with a
  full-resolution fallback is permitted, since that changes speed and not
  meaning.
- Tier 1 and AOT must move conformance by **exactly zero**. They change speed,
  not meaning, so any movement is a bug.

### Terminology

Fixed here so later RFCs need not redefine it.

**Tier 0** — the BundIR interpreter. **Tier 1** — the Cranelift JIT. **AOT** —
the `cranelift-object` build. **Word** — a named callable. **Slot** — a word
table entry. **Workbench** — the auxiliary stack
(`reference/rust_multistack/src/ts_workbench.rs`). **Effect** — a word's stack
arity. **Oracle** — the `bund` binary in `target/oracle`, the authority on what a
program does. Note what it is *not*: `reference/Bund`'s internal dependencies
are registry deps with no `[patch.crates-io]`
(`reference/Bund/Cargo.toml:13,14,23,24,43`), so building it links published
crates, not the sibling submodules — and three submodules currently declare
newer versions than the lockfile resolves. The submodule sources and the
linked crate sources are byte-identical today. `cargo xtask cite` compares
them byte for byte **where the vendored crate source is present**, which means
locally after an oracle build and *not* in CI — those five crates are not
Bund2 workspace dependencies, so CI never downloads them and only the weaker
version check runs there. Calling the oracle "the submodules built" would be
false. Recorded as F21. **Suite** — the programs in `tests/golden/HERMETIC.txt`.
**Probe** — an authored program testing behaviour the reference examples never
reach (D21).

**Conformance** — goldens passed over goldens captured. **Coverage** —
in-scope words with a test over in-scope words. These are two numbers and
neither substitutes for the other: 63 goldens cover 121 of 497 in-scope words,
so conformance can read 100% with three quarters of the language untested.

### Registers

Three files are the shared state between sessions, and this RFC does not
duplicate them:

- `docs/registers/decisions.md` — 29 entries. Append-only; a status may
  change, an entry may not be deleted or renumbered.
- `docs/registers/defects.md` — 27 entries. The roadmap's §5 listed eleven;
  the rest were found by running the oracle rather than reading it. F14, F15 and
  F17 are why 18 of 77 hermetic programs could not be captured, and
  `tests/golden/UNSTABLE.txt` tags each row with which: **13 F14, 2 F15, 3
  F17**. Normalising F14 and F15 recovered those 15; the three F17 rows are
  the remainder. An empty `disposition` blocks any work item touching that area.
- `docs/registers/open-questions.md` — questions get disposed of, not
  annotated: grounded, promoted to a decision, or deleted.

## Preservation analysis

RFC-0000 specifies no behaviour, so most of this is "not applicable". Three
items do carry a preservation consequence.

**Dependency graph: preserved exactly.** Bund2's crate stack is acyclic in the
same direction as the reference's.

**Three registration tiers: preserved exactly.** `bund2-interp` must reproduce
the fallthrough at `multistackvm_inline.rs:42,52`, not collapse the tables into
one. Collapsing would be invisible until a word registered in two tiers
resolved to the wrong one.

**Open world: preserved exactly**, per D16, and the tier model is written
around it rather than against it.

**Feature gating is additive, not a deviation.** A word behind a disabled
feature is absent, not different, and no *retained* behaviour changes.

Both denominators moved, and the RFC should not report only one. D28 narrowed
the suite from 59 programs to 57, orphaning two goldens which were removed;
the conformance denominator is 63 because six probes were added at the same
time. Coverage moved from 140/586 to 121/497. No golden's *content* changed —
the scope filter in `cargo xtask corpus` guarantees a suite program invokes no
gated word — but "no golden changes" would be the wrong claim: two stopped
existing.

**Four more behaviours the design touches, stated rather than left implicit.**

*`ptr` does not unconditionally push.* The Current behaviour section says it
pushes a PTR naming a word; under `autoadd` the same `apply` call appends it
to the top-of-stack value instead
(`reference/rust_multistackvm/src/multistackvm_apply.rs:19-27`). Every
statement in this RFC about what a word "pushes" carries that qualifier, which
is the same finding as the resolution-order one and has the same remedy:
RFC-0002 states the dispatch contract per `autoadd` arm.

*Some names exist only in the dead table, and two aliases point into it.*
`stacks_left` is registered by `register_function` alone
(`reference/rust_multistack/src/stdlib/rotate.rs:93`) and never by
`register_inline`, so dispatch cannot reach it — yet `<-` and `←` are aliased
to it (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:22,23`).
Confirmed against the oracle: `1 2 <-` fails with *"Inline stacks_left not
registered"*, while `1 2 ->` gives `2` — `stacks_right` is registered inline
and works. Recorded as F22. (A separate defect, F23, affects
`rotate_stack_right`, which is a different word from `stacks_right` and
rotates the wrong way; an earlier draft conflated the two and claimed the
pair was broken in both directions, which it is not.) F19 covers the table; these aliases are why dropping it is
not purely subtractive — two documented words go with it.

*A JSON round trip discards identity.* `to_binary` on a JSON value converts to
a string and re-wraps (`reference/rust_dynamic/src/bincode.rs:9-28`), and
`from_binary` re-parses it (`:54-69`) through `Value::json`, which mints a
fresh identity with `nanoid!()`
(`reference/rust_dynamic/src/create_special.rs:205,207`), so the reconstructed
value is not the original. This is why F13's disposition calls
`dup` on a JSON value behavioural rather than merely slow, and RFC-0001 must
decide whether the round trip preserves identity or the reference's behaviour
is preserved as-is.

*The conformance gate compares normalised output, and error text is inside the
contract.* `cargo xtask conform` compares after stripping ANSI and replacing
`id`/`stamp` (F14) and map order (F15). Everything else in a golden — including
error messages, which the oracle prints to stdout — is compared verbatim. So a
Bund2 error whose wording differs from the reference's fails conformance. That
is deliberate, since error text is observable behaviour, but it is a heavier
contract than "the program computes the same answer" and should not be
discovered during M1.

**The world file container changes** from SQLite to redb (D27). The contained
values do not: they remain bincode-serialised `Value`s, and D20's rule that
serialisation materialises lazy identity is unaffected. This is a format
change gated on D11, which is still OPEN.

## Alternatives considered

**Keep five separate repositories, as the reference does.** Rejected: it is
what makes F8 possible. Unbounded `">=0.*.*"` pins between separately released
crates mean a `Value` layout change reaches four dependents without a version
bump. A monorepo makes the change and its consequences one commit.

**One crate.** Rejected: it would make Tier 1's optionality unenforceable and
D28's feature gating cosmetic, because nothing would stop a core path reaching
into a gated subsystem.

**Mirror the reference's crate split exactly.** Rejected in one place. The
reference separates `rust_multistack` from `rust_multistackvm`, but the split
is not a boundary — `i_direct` reaches across it on every word miss
(`multistackvm_inline.rs:52`), which is precisely the fallthrough that made
`take` and `drop` look unregistered to a two-crate scan. Bund2 keeps the three
tiers as *tables* inside `bund2-interp` rather than as crates, so the
fallthrough is visible in one file.

**Defer the feature-gating decision until after M1.** Rejected on measurement:
9.6 ms of a 14 ms run is binary load, so the dependency set is not a
late-stage tuning question. Deferring it would also mean the Phase 0 baseline
measured a binary Bund2 will never resemble.

## Acceptance criteria

Two kinds, kept apart on purpose. **Five are binding**: each can fail today,
and each is a command with an answer. **Two are deferred**: they describe real
rules, but both pass against empty crates, and a criterion that cannot fail is
not a criterion. Accepting this RFC means accepting the five. The two are
carried so RFC-0002 inherits them rather than rediscovering them.

Earlier drafts listed all seven as one set of six, which overstated what
acceptance rested on. Two drafts also got criterion B3 wrong in ways that
looked like passes — the record is kept below, since the failure mode is the
point.

### Binding — verified at this commit

**B1. `cargo xtask conform` reports 0/63**, and the number is recorded in
`tests/golden/CONFORMANCE.txt`. RFC-0000 changes no behaviour, so any movement
means something else did. This is the regression gate.

**B2. `cargo xtask coverage` reports 121/497** in-scope words covered. This is
the completeness number, and it is *not* a gate: it moves whenever D14 or D29
rules on another word. It is pinned here so a later RFC citing it can tell
whether it has shifted.

**B3. `cargo tree -p bund2-stdlib` does not list `bund2-jit`** — Tier 1 stays
optional. 0 occurrences today.

**B4. `git status --porcelain` is empty inside every `reference/` submodule**
after a full `cargo xtask golden` run. All six are clean. This guards the
citation targets, not the binary: since the oracle links crates.io rather than
the submodules (F21), an edit to a submodule cannot change what the oracle
runs, and this check cannot see it. B5 is what covers the binary.

**B5. `cargo xtask cite` reports zero defects** over 1059 citations: every
`reference/...:N` in this RFC and in the registers names a file that exists
and a line that exists, and every fenced block claiming a line matches it
verbatim. It also reports **5/5 oracle crates byte-verified**, which is the
part that covers the binary B4 cannot see. That comparison walks `**/*.rs`
under each crate's `src/`, and the grammar is neither — `bund.pest` sits at
the crate root. Every syntax decision in the registers rests on it, so a
divergence there is exactly what B5 exists to catch, and it was invisible;
root-level `*.pest` is now compared too. The two copies agree.

### Deferred to RFC-0002 — cannot fail today

**D-1. `cargo tree --no-default-features` pulls in no `ai`, `image`, `bus`,
`forecast`, `statistics`, `internaldb` or `console` dependency.** No Bund2
crate declares any of those features; the only features that exist are `aot`,
`jit` and `async` (`crates/bund2/Cargo.toml`, `crates/bund2-jit/Cargo.toml`).
The command can produce no evidence either way, so asserting it would pass by
vacuity. D28 is a commitment until RFC-0002 declares the feature set.

**D-2. `cargo tree -p bund2-value` does not list `bund2-interp`** — the value
type is usable without a VM. `cargo tree -p bund2-value` prints a single line
and lists no dependency at all, so nothing could fail it. It passes because
the crate is empty, not because the rule is enforced. Real with RFC-0001.

Two drafts got this one wrong. The first wrote it as
`cargo tree -p bund2-interp`, which lists `bund2-value` four times because
that is the *intended* direction — so it failed while appearing to test the
rule. The second corrected the command and then claimed the clause was real.

### Stated limits of the five

None of these blocks acceptance. All three are real and none is fixed here.

- **B5's byte check does not run in CI.** The five oracle crates are not Bund2
  workspace dependencies, so their vendored sources are absent in a clean
  checkout and the tier degrades to "not compared" without a defect. The
  strongest part of B5 is strong locally and hollow in the pipeline.
- **B2 has zero hand tests.** All 121 covered words are covered by goldens, so
  coverage and conformance rest on the same evidence seen twice. The second
  health number is not yet independent of the first. Residue of Q5.
- **B2's denominator has two open decisions under it.** D14 (library scope)
  and D29 (the four dead words) each move it, the latter by four either way.

### Provenance of both denominators

Pinned here so a later RFC can tell whether a figure has shifted. Each step is
a decision, not drift:

| Denominator | Was | Now | What moved it |
|---|---|---|---|
| registered names | 618 | **617** | `swap` is registered both ways, so the deduped count is 617 — but see F20: the alias *shadows* the inline word rather than duplicating it |
| hermetic programs | 82 | **80** | the effect audit caught `string.random.*` as a false hermetic (`reference/Bund/src/stdlib/functions/string/random.rs:7`) |
| suite programs | 80 | **57** | −3 out of scope (D15), −18 not reproducible (F14, F15, F17), then −2 more when D28 deferred five subsystems |
| conformance | 59 | **63** | the suite fell to 57, and six authored probes were captured (D21) |
| coverage | 140/586 | **121/497** | D26 and D28 together moved 89 words out of scope. The 120 now shown as out-of-scope is the running total, D15's 31 console words included — 617 − 586 = 31 was D15 alone |

The full narrowing is regenerated into `tests/golden/HERMETIC.txt` on every
`cargo xtask corpus` run, so it cannot drift from the filters that produce it.

## Open questions

- **Q14** — the Phase 0 baseline cannot resolve interpretation, because 9.6 ms
  of every 14 ms is binary load. D28 removes most of that cause for Bund2, so
  the question narrows to whether the corpus resolves interpretation once the
  dependency set is cut. Re-run `cargo xtask bench --target bund2` when there
  is a `bund2` to run.
- **Q15** — `cargo xtask unblock` needs redesigning before it can order M6
  work; as specified it can only see the words the goldens touch.
- **D11** is OPEN and gates D27. If an external reader of the world file
  exists, the redb change is a breaking format change.
- **D14** is OPEN and is the M6 denominator. It does not block this RFC, but
  B2's figure moves as it resolves.
- **D29** is OPEN: whether Bund2 revives or omits the four dead words
  (F19, F22, F24). Either choice deviates from the oracle, and it moves
  B2's denominator by four either way.
- **Q17 is closed.** `docs/research/05-rfc-roadmap.md` §1.5 described the
  reference's `Documentation/Bund_Library_Guide/` as the closest thing to a
  language specification and proposed it as the normative reference for
  judging preservation. It has now been read. It documents **99 words of the
  617 callable names** — 16% — with no grammar, no evaluation order and no
  resolution order, and six of those 99 pages are never rendered because
  `index.csv` does not name them. **This RFC does not adopt it as normative**,
  and now says so on the evidence rather than for want of reading: it is a
  partial standard-library reference, and a preservation standard that omits
  five sixths of the language is not one. The corpus and the registry remain
  the oracle; the guide is corroborating evidence, and where the two disagree
  the implementation is what Bund2 preserves.

  What the reading is worth is corroboration and three new facts. `cargo xtask
  guide` cross-references it on every run: the guide's own three-layer
  attribution agrees with `classify::subsystem` on **96 words with 0
  disagreements**, which is D14's axis confirmed from an independent source
  (the axis, not the cut — D14 stays OPEN); the effect audit catches every one
  of the author's 18 hand-flagged hazards and 12 more he did not flag; and the
  guide documents one word, `stacks_left`, that cannot be called at all, which
  is why D29's four are no longer symmetric. The full reading is in
  `docs/registers/open-questions.md`, and the FIFO-policy divergence it turned
  up is F27.
