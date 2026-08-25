# RFC-0000: Architecture, crate boundaries, and the tier model

- Status: Draft
- Depends on: —
- Decisions consumed: D15, D16, D20, D21, D26, D27, D28
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- Supersedes: `docs/research/04-consolidated-architecture.md` §2-§3 as the
  statement of record for layout and tiering. §1.1 is deliberately excluded:
  it mixes layout with value, symbol and lambda claims that belong to
  RFC-0001, RFC-0002 and RFC-0003, and superseding it here would take scope
  this RFC disclaims. See `docs/research/ERRATA.md`,
  which holds six corrections — two predating this session, three from the
  corpus scan, and this RFC's own supersession entry.

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

**The internal version pins are almost all unbounded.** Six of the seven
internal dependencies across the reference are `">=0.*.*"` — the five in
`reference/Bund/Cargo.toml:13,14,23,24,43`, plus
`reference/bund_language_parser/Cargo.toml:17`,
`reference/bundcore/Cargo.toml:12,17,18` and
`reference/rust_multistack/Cargo.toml:14`. The exception is
`reference/rust_multistackvm/Cargo.toml:21`, pinned `">=0.33.*"`, which bounds
the minor series but not the patch. So a `Value` layout change propagates to
every dependent without a version bump in all but one edge. Recorded as F8,
and the monorepo resolves it.

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

**That chain is one arm of a branch, not the whole story.** `apply` first tests
`self.autoadd` (`reference/rust_multistackvm/src/multistackvm_apply.rs:19`);
when it is set, the name is *appended to the value on top of the stack*
(`:20-27`) and no resolution happens at all. The chain above is the `else`
arm. `autoadd` is toggled by the `:` and `;` commands, so a program can switch
the interpreter between resolving names and collecting them. Any statement
that "resolution order is the contract" has to say which arm it means.

**Aliases resolve twice, and exactly twice.** `apply` resolves an alias at
`:39`, then calls `i()`, which tests and resolves it again
(`reference/rust_multistackvm/src/multistackvm_inline.rs:71-72`). The second
resolution is a no-op on an already-resolved name, but it is on the dispatch
path for every call. Recorded as F6.

**There is a fourth registration table, and dispatch never reaches it.**
`register_function` (`reference/rust_multistack/src/ts_functions.rs:6`) fills a
separate `functions` map with 29 call sites across
`reference/rust_multistack/src/stdlib/`. `i_direct` consults only the two
inline tables, so nothing registered there is callable as a word. Three
*dispatch* tiers, four *tables* — a re-implementation that ports the table
because it exists would add a name space the language does not have.

### The world cannot be closed

`ptr` casts a stack value to a string and pushes a PTR naming a word
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:80-93`); `!`, an alias of
`execute` (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:5`), hands
that name to `vm.call` (`reference/rust_multistackvm/src/stdlib/execute.rs:26-33`).
`reference/Bund/examples/bund_dynamic_demos/dynamic_demo_2.bund:29-34` builds a
word name by string concatenation and calls it, so a callee's name need not
appear anywhere in the source. `execute` also accepts a bare STRING
(`execute.rs:27`), so `ptr` is not even required. D16 preserves this.

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
arity. **Oracle** — `reference/Bund` built at the pinned SHA, the authority on
what a program does. **Suite** — the programs in `tests/golden/HERMETIC.txt`.
**Probe** — an authored program testing behaviour the reference examples never
reach (D21).

**Conformance** — goldens passed over goldens captured. **Coverage** —
in-scope words with a test over in-scope words. These are two numbers and
neither substitutes for the other: 63 goldens cover 121 of 497 in-scope words,
so conformance can read 100% with three quarters of the language untested.

### Registers

Three files are the shared state between sessions, and this RFC does not
duplicate them:

- `docs/registers/decisions.md` — 28 entries. Append-only; a status may
  change, an entry may not be deleted or renumbered.
- `docs/registers/defects.md` — 18 entries. The roadmap's §5 listed eleven;
  seven more were found by running the oracle rather than reading it. F14 and
  F15 are why 18 of 77 hermetic programs could not be captured — those are the
  only two causes `tests/golden/UNSTABLE.txt` tags. F17 is why three of the
  eighteen stayed unreproducible after F14 and F15 were normalised. An empty `disposition` blocks any work item touching that area.
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

1. `cargo xtask conform` reports **0/63** at this commit, and the number is
   recorded in `tests/golden/CONFORMANCE.txt`. RFC-0000 changes no behaviour,
   so any movement means something else did.
2. `cargo xtask coverage` reports **121/497** in-scope words covered.
   The denominator moved from 586 when D28 deferred five more subsystems;
   it moves again as D14 resolves, which is why criterion 1 and not this one
   is the regression gate.
3. **Not yet checkable, and stated as such.** No Bund2 crate declares an
   `ai`, `image`, `bus`, `forecast`, `statistics`, `internaldb` or `console`
   feature — the only features that exist today are `aot`, `jit` and `async`
   (`crates/bund2/Cargo.toml`, `crates/bund2-jit/Cargo.toml`). So
   `cargo tree --no-default-features` can produce no evidence either way, and
   asserting it would be a criterion that passes by vacuity. It becomes
   checkable when RFC-0002 declares the feature set; until then D28 is a
   commitment, not a verified property.
4. `cargo tree -p bund2-interp` does not list `bund2-value` as reaching back
   into it, and `cargo tree -p bund2-stdlib` does not list `bund2-jit`. Note
   the direction: the rule forbids *stdlib depending on jit*, so the check is
   the forward graph, not `--invert`, which enumerates dependents. Also
   vacuous today, because both crates are empty scaffolds — it becomes
   meaningful with RFC-0002 and RFC-0003.
5. `git status --porcelain` inside every `reference/` submodule is empty after
   a full `cargo xtask golden` run.
6. `cargo xtask cite` reports zero defects: every `reference/...:N` citation
   in this RFC and in the registers names a file that exists and a line that
   exists. Implemented in response to this RFC's review, which found five
   citation defects by hand.

Criteria 1 and 2 are the two health numbers. Criterion 5 catches an accidental
edit to the oracle, which would invalidate every citation in every RFC.
Criteria 3 and 4 are recorded as **not yet checkable**: both would pass today
against empty crates, and a criterion that cannot fail is not a criterion. They
are carried so RFC-0002 inherits them rather than rediscovering them.

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
  criterion 2's figure moves as it resolves.
- The reference's `Documentation/Bund_Library_Guide/` is described in
  `docs/research/05-rfc-roadmap.md` §1.5 as the closest thing to a language
  specification and the normative reference for judging preservation. This RFC
  does not adopt it as normative, because it has not been read.
  `[UNGROUNDED]` — recorded as Q17.
