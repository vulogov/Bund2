# Bund2

Re-implementation of the Bund concatenative language: multi-stack, dynamically
typed, metaprogramming, with a tiered execution model (BundIR interpreter, plus
an optional Cranelift JIT and AOT compiler). Bund syntax and logic are
preserved 100%.

Bund2 is not a refactor of Bund. It is a reimplementation against an oracle:
`reference/` holds the existing implementation, pinned, and `tests/golden/`
holds the state it produces for the example corpus.

## Attribution — absolute

Never add AI attribution anywhere, in any form. No `Co-Authored-By` trailers,
no "Generated with" lines, no AI credit in commit messages, PR bodies, RFCs,
code comments, or documentation. Authorship is the repository owner's alone.
This applies even where a settings key would otherwise inject it.

## Source grounding — required

Design claims must cite source. Every factual claim about existing behaviour
carries a `path:line` citation into `reference/`, and you must have read that
file in this session — never cite from memory or infer from a filename.

If a claim cannot be grounded, do not soften it into prose. Add it to
`docs/registers/open-questions.md` and mark the spot `[UNGROUNDED]`.

## reference/ is read-only

`reference/` holds pinned submodules of the existing implementation, for
analysis only. Never edit it, never build inside it, never treat a file there
as a target of work.

## tests/golden/ is sacred

A golden that disagrees with Bund2 is not the thing that changes. A failure has
exactly three dispositions: a Bund2 bug (fix Bund2), an original-implementation
bug (record in `docs/registers/defects.md`, then
`cargo xtask golden --accept <name> --reason <ref>`), or a deviation already
approved in the work item. An unplanned deviation is a decision: stop and take
it to `docs/registers/decisions.md` before changing code.

## Registers are append-only

`docs/registers/decisions.md` and `defects.md` are the shared state between
sessions. Add entries and change an entry's `status`; never delete or renumber.

Never adopt an OPEN decision's default silently. The defaults are for planning.
An RFC or work item that quietly takes one has made a language decision on the
owner's behalf. Stop and say which decision blocks you.

## Research documents are immutable

`docs/research/` is the reasoning trail. When an RFC contradicts one, record it
in `docs/research/ERRATA.md` — do not edit the original.

## Terminology

Tier 0 = the BundIR interpreter (mandatory, every target).
Tier 1 = the Cranelift JIT (optional). AOT = the cranelift-object build.
Word = a named callable. Slot = a word table entry. Workbench = the auxiliary
stack. Effect = a word's stack arity. Conformance = passing goldens over total.

## Health metric

`cargo xtask conform` prints N/M. That number is the project's status. The JIT
and AOT milestones must move it by exactly zero: they change speed, not
meaning, so any movement is a bug.
