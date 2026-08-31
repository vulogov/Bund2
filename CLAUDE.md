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
analysis only. Never edit it and never treat a file there as a target of work.

Building it is the one exception, and only to produce goldens: that is what the
oracle is for. Always build it out-of-tree so the submodule stays clean —

    cargo build --release --manifest-path reference/Bund/Cargo.toml \
                --target-dir target/oracle

`git status` inside `reference/` must stay empty. A dirty submodule means a
`path:line` citation somewhere no longer resolves against the recorded SHA.

## tests/golden/ is sacred

A golden that disagrees with Bund2 is not the thing that changes. A failure has
exactly three dispositions: a Bund2 bug (fix Bund2), an original-implementation
bug (record in `docs/registers/defects.md`, then
`cargo xtask golden --accept <name> --reason <ref>`), or a deviation already
approved in the work item. An unplanned deviation is a decision: stop and take
it to `docs/registers/decisions.md` before changing code.

## Bund2 does not panic — absolute

No `panic!`, no `unwrap()`, no `expect()`, no `unreachable!`, no `todo!`, no
`unimplemented!`, no `process::exit` in shipped code. Enforced by
`[workspace.lints.clippy]`, denied across every crate including `xtask`; tests
opt out at each crate root because a panicking assertion in a test is the point.

An interpreter that aborts takes the user's program state with it — the stacks,
the word table, whatever a session had built — and explains nothing, because
the trace names Rust frames and no Bund word.

Three ways out, in order of preference:

1. **Make the invariant structural.** `BundValue::into_heap` returns the `Rc`,
   so "promote always yields a heap value" is a type and there is no arm to
   write. Prefer this; it removes the question.
2. **Write the real error arm.** The reference guards a word's depth *and*
   writes `SET returns: NO DATA #1` for the pull that follows
   (`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:30-42`).
   Reproducing that is safer *and* more faithful than asserting it away.
3. **`Error::internal`.** For a broken invariant with no sensible continuation.
   It names the invariant, says the defect is in Bund2 rather than in the
   program being run, and routes through the diagnostic path so a reporter — a
   TUI's included — receives it. `Error::is_internal` separates the two
   audiences.

Every unrecoverable internal error is handled with a meaningful explanation.
"Cannot happen" is not an explanation. See D37.

## Errors are reported, not printed

Words emit a `Diagnostic` through `Vm::report`; they do not write to stdout or
stderr. A `Diagnostic` is structured — severity, reason, Bund source location,
optional stack snapshot — and a `Reporter` renders it. That is the seam a TUI
will implement, so anything that formats at the point of failure closes it.

Delivery is proportional to severity: an `Error` has stopped the program and
earns a report, a `Warning` or `Notice` has not and gets one line on stderr.
Values in a report use `BundValue::summary`, which is bounded; the raw `Debug`
form belongs to `debug.display_stack` and `--raw-values`. See D36.

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
Coverage = in-scope words with a test over in-scope words.

## Health metric

Two numbers. Neither substitutes for the other, and a report that quotes one
alone is misleading.

`cargo xtask conform` prints N/M over the goldens. That is the regression
number, and the JIT and AOT milestones must move it by exactly zero: they
change speed, not meaning, so any movement is a bug. Never add words to that
denominator — implementing a word would move the number and destroy the
invariant that makes it worth having.

`cargo xtask coverage` prints in-scope words with a test over in-scope words.
That is the completeness number. The goldens reach only about a quarter of the
in-scope words, so `conform` can read 100% with most of the language
unimplemented and nothing else would say so.
