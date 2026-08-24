# Bund2 — Scaffolding and RFC-to-Implementation

**Status:** Process
**Date:** 2026-08-23
**Companion to:** the RFC workflow document (CLAUDE.md, slash commands, session recipe).
This covers what to build before the first RFC, and how an accepted RFC becomes work.

---

## Part 1 — Scaffolding

### 1.0 The framing that makes everything else easier

**Bund2 is not a refactor of Bund. It is a reimplementation against an oracle.**

That distinction decides the whole plan. You are not migrating 13,289 lines of library code
plus 18,996 lines of CLI in place, keeping it compiling at every step — that would be
brutal, because the `Value` rewrite touches everything at once. You are writing a new
implementation in a new repository, with the existing one pinned in `reference/` as a
**conformance oracle**: run an example through old Bund, record the final state, and make
new Bund produce the same thing.

Everything below follows from that. The scaffold's job is to make the oracle exist before
any Bund2 code does.

### 1.1 Day one

```bash
git init Bund2 && cd Bund2

for r in Bund bund_language_parser bundcore \
         rust_multistackvm rust_multistack rust_dynamic; do
  git submodule add https://github.com/vulogov/$r reference/$r
done
git submodule status > reference/PINNED.txt   # the SHAs your RFCs cite
```

License: Apache-2.0, matching the existing repositories.

Then the standing files from the workflow document — `CLAUDE.md`,
`.claude/settings.json` (attribution off, `reference/` denied for Edit/Write),
`.claude/commands/`, `docs/rfc/0000-template.md` — and drop the six research documents
into `docs/research/`.

### 1.2 The workspace skeleton, on day one

Create all eleven crates immediately, each a stub. Not because you need them, but because
crate boundaries drift if they aren't real, and `bund2-api` needs its stability marker from
the first commit rather than after something already depends on it wrongly.

```toml
# Cargo.toml
[workspace]
resolver = "3"
members = [
  "crates/bund2-api",      # STABLE surface. Depends only on bund2-value.
  "crates/bund2-value",
  "crates/bund2-syntax",
  "crates/bund2-ir",
  "crates/bund2-interp",   # Tier 0 — mandatory
  "crates/bund2-jit",      # Tier 1 — feature-gated
  "crates/bund2-stdlib",
  "crates/bund2-runtime",
  "crates/bund2-async",
  "crates/bund2",          # library façade
  "crates/bund2-cli",      # produces the `bund2` binary
  "xtask",                 # tooling; see 1.4
]

[workspace.package]
edition = "2024"
license = "Apache-2.0"
repository = "https://github.com/vulogov/Bund2"

[workspace.dependencies]
# Cranelift: exact pins. This crate self-describes as experimental and moves monthly.
cranelift-codegen  = { version = "=0.135.0", optional = true }
cranelift-frontend = { version = "=0.135.0", optional = true }
cranelift-module   = { version = "=0.135.0", optional = true }
cranelift-jit      = { version = "=0.135.0", optional = true }
cranelift-object   = { version = "=0.135.0", optional = true }
```

Put a `crates/bund2-api/README.md` that says, in one line, that this crate is the only
stability guarantee in the workspace and every change to it is a breaking change. It will
be read by whoever writes the first external word package, including future you.

`rust-toolchain.toml` pinned. `deny.toml` with `cargo-deny` in CI — given the pure-Rust
preference, an explicit check beats a convention.

### 1.3 Seed the registers before the first RFC

`docs/registers/decisions.md` with D1–D13 from the roadmap, each `status: OPEN`, each with
its default and the RFC it blocks. `docs/registers/defects.md` with F1–F11, each with a
`disposition:` field that is empty until someone decides preserve-or-fix.

These two files are the shared state between every session. Seeding them means session one
already has something to consume rather than something to invent.

### 1.4 `xtask`, not scripts

Put every tool behind `cargo xtask <cmd>` rather than shell scripts or a Makefile. Stays
cargo-native, works on every platform, no extra runtime, and the tools get type-checked by
CI like everything else.

```
cargo xtask golden      # run the oracle, record expected outputs
cargo xtask conform     # run Bund2 against the goldens, print N/M
cargo xtask arity       # probe every word's stack effect
cargo xtask layout      # size_of, allocation counts
cargo xtask bench       # Criterion corpus
cargo xtask corpus      # grep the examples for decision evidence
```

### 1.5 Build the oracle — the actual first task

This is the thing that must exist before anything else, and it is a real build, not a
document.

The corpus: **120 example programs and 12 test programs** in `reference/Bund`. Of the 120
examples, **114 touch no network, no filesystem, no image, no database and no bus** — those
are the hermetic conformance suite. The other six are smoke tests that run manually.

```
cargo xtask golden
  → builds reference/Bund with cargo
  → for each hermetic example:
      run it, capture: every named stack's final contents, the workbench,
      the exit status, the error text if any, stdout
  → writes tests/golden/<name>.json
```

Two things to get right here:

- **Capture state, not just stdout.** A concatenative program's meaning is what it leaves
  on the stacks. `debug.display_stack` output is the wrong granularity; serialise the
  actual final state.
- **The goldens are sacred.** When Bund2 later disagrees with a golden, the golden is not
  the thing that changes. Either Bund2 has a bug, or the RFC deliberately deviated — and
  that second case goes to the decision register as a preservation exception, not to a
  quiet edit of the expected output. Write this rule into `tests/golden/README.md` where
  whoever hits it first will see it.

### 1.6 Then the evidence tools

`cargo xtask corpus` is the cheapest high-value session in the project. Scan the 132
programs for `.id`, `.timestamp`, `bund.eval`, `load.lambdas`, post-construction LAMBDA
mutation, and reachability of `register`/`alias`. That single run converts D1, D2, D3, D5
and D12 from judgement calls into counted evidence.

`cargo xtask layout` and `cargo xtask bench` establish the Phase 0 baseline that RFC-1's
acceptance criteria are written against.

`cargo xtask arity` probes each registered word against instrumented stacks and emits a
first-cut effect table — the thing that unblocks RFC-4 without reading 12,000 lines.

### 1.7 CI from the first commit

`fmt`, `clippy -D warnings`, `test`, `cargo-deny`, and — the one that matters —
`cargo xtask conform` printing the conformance count and failing on regression.

---

## Part 2 — RFC to implementation plan

### 2.1 An RFC is not a plan

They fail differently. An RFC that is wrong produces a bad design. A plan that is wrong
produces six weeks of work that doesn't integrate. The conversion is a real decomposition
step, and its job is to answer three questions the RFC deliberately doesn't:

1. **In what order**, such that the system is in a working state at every step?
2. **Verified how**, at each step, without a human reading every line?
3. **Done means what**, in a number?

### 2.2 The single health metric

**Conformance: N of 114.**

Not story points, not a burndown, not "Phase 3 is 60% complete". The number of hermetic
examples whose final state matches the oracle byte for byte. It is unfakeable, it moves
monotonically if the work is real, and it means the same thing to everyone.

Two corollaries worth stating up front. Conformance may sit at 0 for a long while and then
jump — the first twenty words unlock nothing, then suddenly `helloworld.bund` passes and
the next five come cheaply. And the JIT and AOT milestones must move conformance **by
zero**: they change speed, not meaning, and any movement is a bug.

### 2.3 The milestone ladder

Milestones are conformance bands, not feature lists.

| M | Scope | Conformance target |
|---|---|---|
| **M0** | Oracle + goldens + xtask + CI | oracle green; Bund2 at 0 |
| **M1** | `bund2-value`, symbols, slot table, `bund2-api` | 0 — infrastructure only |
| **M2** | pest → AST → BundIR → Tier 0 flat loop; ~25 core words (stack ops, int/float math, comparison, `println`) | first passes; `helloworld` and the arithmetic tests |
| **M3** | Named stacks, workbench, contexts, lambdas, `if`/`times`/`while`/`loop`/`map` | the loop and control examples |
| **M4** | Metaprogramming: `register`, `alias`, `lambda*`/`lambda!`/`lambda=`, `call,`, `curry`, `bund.eval` | `bund_dynamic_demos/` — the whole directory |
| **M5** | OOP: classes, objects, flattened vtables, `m()` dispatch, `#` | the class examples |
| **M6** | The remaining stdlib, by area | → 114/114 |
| **M7** | Stack effects, `bund2 check`, inline caches | 114/114 unchanged; benchmarks improve |
| **M8** | Cranelift AOT (`ObjectModule` first) | 114/114 unchanged |
| **M9** | Cranelift JIT (`JITModule`) | 114/114 unchanged |
| **M10** | Async, debugger, image, superinstructions | 114/114 unchanged |

M8 before M9 is deliberate — object output can be disassembled, diffed and checked into a
test corpus; a JIT gives you a function pointer you can only call.

### 2.4 Ordering M6 with data, not intuition

M6 is the long tail: roughly 330 words. Don't implement them alphabetically or by
subsystem. Order by **examples unblocked per word**.

```
cargo xtask unblock
  → for each unimplemented word W: count hermetic examples that
    (a) use W and (b) use nothing else unimplemented
  → sort descending
```

That produces a work queue where every item visibly moves the health metric, and it
naturally surfaces the small clusters of words that gate whole example families. It also
tells you when to stop: the words that unblock nothing are the ones no real program uses,
and they can wait.

### 2.5 The work item

One RFC section becomes one or more work items. One work item is one Claude Code session.
The issue text *is* the prompt — write it that way.

```markdown
## WI-042 — `bund2-interp`: `times`, `loop`, `while`

RFC: RFC-0003 §5.2
Milestone: M3
Reference: reference/Bund/reference/rust_multistackvm/src/stdlib/logic/
           times_fun.rs, loop_fun.rs, while_fun.rs @ <pinned SHA>

### Preserve
- `times` evaluates the lambda N times; N pulled before the lambda
- the `.` workbench variants
- the `*` whole-stack variants
- error text on shallow stack (goldens capture it)

### Deviate (approved, RFC-0003 §5.2)
- no per-iteration deep clone of the body — `Rc` share instead
  (behaviourally identical under `Rc::make_mut`; see D13)

### Acceptance
- `cargo xtask conform` ≥ 31 (currently 24)
- tests/golden/test_times_loop.json, test_loop_loop.json, test_map_loop.json green
- no previously-green golden regresses
```

Four fields, and every one of them is doing work: the RFC section anchors the design, the
pinned reference anchors the behaviour, **Deviate** forces preservation exceptions to be
explicit rather than discovered, and acceptance is a number the harness produces.

### 2.6 When conformance and the RFC disagree

This will happen, and how it's handled determines whether the preservation constraint
survives contact with the work.

A failing golden has exactly three dispositions:

- **Bund2 bug** → fix Bund2. The common case.
- **Original bug** → one of F1, F2, F3, F11, or a new one. Record it in the defect
  register with a disposition, regenerate that specific golden, and note the exception in
  `tests/golden/EXCEPTIONS.md` with the defect number.
- **Deliberate deviation** → must already be listed in the work item's Deviate section. If
  it isn't, stop: an unplanned deviation is a decision, and it goes to the register before
  any code changes.

The thing to refuse is a fourth option — quietly regenerating the golden because it's
easier. Making the goldens read-only in CI (regeneration only through
`cargo xtask golden --accept <name> --reason <ref>`) turns that discipline into a
mechanism.

### 2.7 Where the sessions fit

```
1 work item  = 1 Claude Code session = 1 branch = 1 PR
```

The session reads the work item, the RFC section it cites, and the pinned reference source.
It writes code and tests. CI runs `conform`. The human reviews the *diff against the
oracle*, which is a much smaller thing to review than the code.

For the RFC-drafting sessions, the separation from the previous document holds: draft and
review are different sessions, and `/rfc-review` verifies citations by reopening files.

### 2.8 First four weeks, concretely

1. **Scaffold + oracle.** Repo, submodules, standing files, workspace stubs, `xtask golden`
   over the 114 hermetic examples. Ends with: the oracle runs and CI is green.
2. **Evidence.** `xtask corpus`, `layout`, `bench`, `arity`. Ends with: D1, D2, D3, D5, D12
   resolved from data, and a Phase 0 baseline recorded.
3. **RFC-0 and RFC-1.** Architecture and `BundValue`. RFC-1 accepted against week 2's
   numbers. Separate review sessions for each.
4. **RFC-2 and RFC-9**, both fully grounded already, plus `/rfc-ground` on
   `conditional/ values/ bund/` to unblock RFC-3.

M1 code starts in week five, against three accepted RFCs and a working oracle. That is
about as much groundwork as a language reimplementation warrants, and less than the
preservation constraint would otherwise cost you in rework.
