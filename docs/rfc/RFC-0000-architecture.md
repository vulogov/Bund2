# RFC-0000: Architecture, crate boundaries, and the tier model

- Status: Draft
- Depends on: —
- Decisions consumed: D15, D16, D20, D21, D26, D27, D28
- Reference SHA: `reference/Bund` at `21b40b0213a7`; `bund_language_parser`
  `80377728f45b`; `bundcore` `3b0b8ba219a6`; `rust_dynamic` `ceb27c96fa10`;
  `rust_multistack` `9a97675ee5d8`; `rust_multistackvm` `4605832678d4`
- Supersedes: `docs/research/04-consolidated-architecture.md` §1-§3 as the
  statement of record for layout and tiering. See `docs/research/ERRATA.md`
  for three corrections that predate this RFC.

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

**The crates are pinned `">=0.*.*"` against each other.** Every internal
dependency in `reference/Bund/Cargo.toml:13,23,24,43` is unbounded, so a
change to `Value`'s layout propagates silently across five crates. This is
recorded as F8, and the monorepo resolves it.

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
`reference/bund_language_parser/bund.pest` is 55 lines defining twelve `value`
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

### The world cannot be closed

`ptr` casts a stack value to a string and pushes a PTR naming a word
(`reference/rust_multistackvm/src/stdlib/artefacts.rs:80-93`); `!`, an alias of
`execute` (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:5`), hands
that name to `vm.call` (`reference/rust_multistackvm/src/stdlib/execute.rs:26-33`).
`reference/Bund/examples/bund_dynamic_demos/dynamic_demo_2.bund:29-34` builds a
word name by string concatenation and calls it, so a callee's name need not
appear anywhere in the source. `execute` also accepts a bare STRING
(`execute.rs:28`), so `ptr` is not even required. D16 preserves this.

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
  seven more were found by running the oracle rather than reading it, of which
  F14, F15 and F17 are the reason 18 of 77 hermetic programs could not be
  captured. An empty `disposition` blocks any work item touching that area.
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
feature is absent, not different. Programs in the conformance suite invoke no
gated word — that is what the scope filter in `cargo xtask corpus` enforces —
so no golden changes. The M6 denominator narrows, which D14 and D28 both
record.

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
3. The workspace builds with `--no-default-features` and every gated
   subsystem absent from the dependency graph, verifiable with
   `cargo tree --no-default-features`.
4. `bund2-value` does not appear in `bund2-interp`'s reverse dependencies, and
   `bund2-jit` does not appear in `bund2-stdlib`'s, verifiable with
   `cargo tree --invert`.
5. `git status --porcelain` inside every `reference/` submodule is empty after
   a full `cargo xtask golden` run.
6. Every `path:line` citation in this document resolves at the recorded SHAs.

Criteria 1 and 2 are the two health numbers. Criterion 5 is the one that
catches an accidental edit to the oracle, which would invalidate every
citation in every RFC.

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
