# Bund2 — RFC Workflow with Claude Code

**Status:** Process
**Date:** 2026-08-23

The governing idea: **don't paste context, make it retrievable.** Put the research
documents, the registers, and the source under analysis in one repository, then let each
session read what it needs. A prompt becomes a pointer, not a payload — which is what makes
"verified source grounding before design claims" enforceable rather than aspirational.

---

## 1. Repository scaffold

```
Bund2/
  CLAUDE.md                        standing instructions, loaded every session
  .claude/
    settings.json                  committed; attribution off, permissions
    commands/
      rfc-draft.md                 /rfc-draft <n>
      rfc-ground.md                /rfc-ground <path>
      rfc-review.md                /rfc-review <n>
  docs/
    research/                      the five documents produced so far
      00-jit-feasibility.md
      01-extensibility-async.md
      02-native-binaries.md
      03-metaprogramming-oop-debugger.md
      04-consolidated-architecture.md
      05-rfc-roadmap.md
      ERRATA.md                    supersessions; never edit the originals
    rfc/
      0000-template.md
      RFC-0000-architecture.md
      ...
    registers/
      decisions.md                 D1..D13, append-only, each with status
      defects.md                   F1..F11
      open-questions.md            claims that could not be grounded
  reference/                       READ-ONLY. git submodules, pinned by SHA.
    Bund/
    bund_language_parser/
    bundcore/
    rust_multistackvm/
    rust_multistack/
    rust_dynamic/
  tools/                           Phase 0 harnesses (real code, see §6)
  crates/                          the actual Bund2 workspace, later
```

**Use submodules pinned at a SHA, not clones or subtrees.** An RFC that cites
`reference/Bund/src/stdlib/functions/oop/base_classes.rs:12` is worthless if the file
moved. Record the pinned SHA in each RFC's front matter. When you bump a submodule,
re-verify the RFCs that cite it — that is a feature, not a chore.

---

## 2. CLAUDE.md

This is the one file that shapes every session. Keep it short enough to actually be
followed.

```markdown
# Bund2

Re-implementation of the Bund concatenative language: multi-stack, dynamically typed,
metaprogramming, with a tiered execution model (IR interpreter + optional Cranelift JIT +
optional AOT). Bund syntax and logic are preserved 100%.

## Attribution — absolute

Never add AI attribution anywhere, in any form. No `Co-Authored-By` trailers, no
"Generated with" lines, no AI credit in commit messages, PR bodies, RFCs, code comments,
or documentation. Authorship is the repository owner's alone. This applies even if a
settings key would otherwise inject it.

## Source grounding — required

Design claims must cite source. Every factual claim about existing behaviour carries a
`path:line` citation to a file under `reference/`, and you must have read that file in
this session — never cite from memory or from a filename.

If a claim cannot be grounded, do not soften it into prose. Add it to
`docs/registers/open-questions.md` and mark the spot in the RFC as `[UNGROUNDED]`.

## reference/ is read-only

`reference/` holds pinned submodules of the existing implementation, for analysis only.
Never edit, never `cargo build` in it, never treat a file there as a target.

## Registers are append-only

`docs/registers/decisions.md` and `defects.md` are the shared state between sessions.
Add entries; change an entry's `status` field; never delete or renumber.

## Research documents are immutable

`docs/research/` is the reasoning trail. When an RFC contradicts one, record the
supersession in `docs/research/ERRATA.md` — do not edit the original.

## Terminology

Tier 0 = BundIR interpreter (mandatory). Tier 1 = Cranelift JIT (optional).
AOT = cranelift-object build. Word = named callable. Slot = word table entry.
Workbench = the auxiliary stack. Effect = a word's stack arity.
```

---

## 3. `.claude/settings.json`

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "attribution": {
    "commit": "",
    "pr": "",
    "sessionUrl": false
  },
  "permissions": {
    "deny": [
      "Edit(./reference/**)",
      "Write(./reference/**)"
    ]
  }
}
```

`attribution` with empty strings suppresses the commit trailer and the PR footer;
`sessionUrl: false` suppresses the session-link trailer. It replaces the older
`includeCoAuthoredBy` boolean — set one or the other, not both. Verify the current key
names against `https://code.claude.com/docs/en/settings-reference` before relying on them,
since this area has changed.

Two belts, one brace: there have been reports of the trailer appearing despite the
setting, which is exactly why the CLAUDE.md rule in §2 is stated as well. After the first
commit of a session, check `git log -1 --format=%B` once.

The `deny` rules make `reference/` read-only structurally rather than by instruction.

---

## 4. Slash commands

`.claude/commands/rfc-ground.md` — the reading pass. Run this *before* drafting a blocked
RFC.

```markdown
Read every file under $ARGUMENTS in `reference/`. Do not draft anything.

Produce a grounding note at `docs/research/grounding/<area>.md` containing:
- what each file does, in one line, with a path:line anchor
- every word or method registered, with its observed stack arity if determinable
- every place a runtime-mutable table is read or written
- every place a value's identity, timestamp, tags, or attributes are read
- anything that contradicts a claim in `docs/research/` — cite both sides
- anything you could not determine from the source

Then append any new decisions to `docs/registers/decisions.md` and any new defects to
`docs/registers/defects.md`.
```

`.claude/commands/rfc-draft.md`:

```markdown
Draft RFC-$ARGUMENTS.

1. Read `docs/research/05-rfc-roadmap.md` for this RFC's scope, dependencies, and the
   improvements assigned to it in §6.
2. Read `docs/research/ERRATA.md`.
3. Read the grounding notes for this RFC's areas, and re-read the source they cite.
4. Read `docs/registers/decisions.md`. If an OPEN decision blocks this RFC, stop and say
   which; do not pick a default silently.
5. Draft to `docs/rfc/RFC-<n>-<slug>.md` using `docs/rfc/0000-template.md`.

Every behavioural claim gets a path:line citation. Mark ungrounded claims `[UNGROUNDED]`
and add them to open-questions.md. Status starts as `Draft`.
```

`.claude/commands/rfc-review.md`:

```markdown
Review RFC-$ARGUMENTS as an adversarial reader.

- Verify every path:line citation by reading the file. Report any that don't say what the
  RFC claims.
- Find claims with no citation that need one.
- Check each acceptance criterion is actually checkable, and name what would check it.
- Check consistency with accepted RFCs and with ERRATA.
- List what this RFC assumes but doesn't state.

Write findings to `docs/rfc/reviews/RFC-<n>-review-<date>.md`. Do not edit the RFC.
```

Separating draft from review into distinct sessions matters more than it looks: a session
that just wrote a claim is poorly placed to doubt it.

---

## 5. RFC template

```markdown
# RFC-NNNN: <title>

- Status: Draft | Proposed | Accepted | Superseded
- Depends on: RFC-xxxx
- Decisions consumed: D1, D4
- Reference SHA: <submodule commit this RFC was grounded against>
- Supersedes: <research doc §, if any>

## Summary
One paragraph.

## Motivation
What's wrong today, with path:line citations.

## Current behaviour
What the existing implementation does. Every claim cited. This section is the
preservation contract — the design cannot be judged without it.

## Design
The proposal.

## Preservation analysis
For each behaviour in "Current behaviour": preserved exactly / preserved with a stated
deviation / deliberately changed. Deviations need explicit sign-off.

## Alternatives considered
Including the one that was rejected and why.

## Acceptance criteria
Checkable. "Faster" is not checkable; "≥3× on benches/arith_loop.bund vs the Phase 0
baseline" is.

## Open questions
Cross-referenced to the registers.
```

The **Preservation analysis** section is the one that earns its keep here. Given the 100%
constraint, an RFC that doesn't enumerate what it might change hasn't done its job — and
it is where the four behavioural defects (F1, F2, F3, F11) get their explicit
preserve-or-fix decision.

---

## 6. What to submit, session by session

**Start with code, not prose.** The first sessions should build tools, because tools
produce evidence and evidence unblocks four decisions:

| Session | Task | Unblocks |
|---|---|---|
| S1 | `tools/bench/` — Criterion corpus over the 144 examples plus targeted microbenchmarks; strip `time_graph` first | RFC-1 acceptance |
| S2 | `tools/layout/` — print `size_of::<Value>()`, count allocations via a counting allocator | D4, RFC-1 |
| S3 | `tools/arity/` — probe every registered word against instrumented stacks; emit a first-cut effect table | RFC-4, D12 |
| S4 | `tools/diff/` — run the 144 examples, capture final stack + workbench + error as golden output | RFC-10, all later tiers |
| S5 | `tools/corpus/` — grep the examples for `.id`, `.timestamp`, lambda mutation, `bund.eval` reachability | D1, D2, D5 |

S5 is the cheap one that answers three open decisions with evidence rather than judgement.
Run it early.

**Then the prose sessions,** in roadmap order: RFC-0, then RFC-2 and RFC-9 (both fully
grounded), then `/rfc-ground` on `conditional/ values/ bund/` before RFC-3, then
`/rfc-ground` on the bus/zenoh layer before RFC-7.

One RFC per session. Review in a separate session. Don't batch.

---

## 7. What to feed back

After every session, three things move:

**Into the registers.** New decisions → `decisions.md` with status `OPEN`. Decisions
resolved → status `RESOLVED` with the rationale and the session that resolved it. New
defects → `defects.md` with a preserve-or-fix field.

**Into ERRATA.** Every time an RFC contradicts a research document, one line:
`04-consolidated-architecture.md §3.1 → superseded by RFC-0001 §4.2 (reason)`. Do not edit
the research documents. They contain reasoning that was correct given what was known, and
the trail of what changed and why is worth more than a tidy current-state document. Two
supersessions already exist from this analysis — the `.id`/`.timestamp` finding invalidating
the 16-byte value, and `bund.eval` invalidating the closed-world plan.

**Into the next prompt.** The only thing worth carrying by hand between sessions is what
the last one *couldn't* determine. Everything else is in the repo.

---

## 8. Two failure modes to watch for

**Plausible-but-uncited claims.** The failure mode of an agent drafting a design document
is fluent prose about code it hasn't opened. The `/rfc-review` command exists specifically
to catch it, and the check that matters is re-reading each cited line — not re-reading the
RFC. Budget a review session per RFC and treat unverifiable citations as blocking.

**Silent default-taking.** The decision register has defaults for every open item. Those
are for planning, not for drafting: an RFC that quietly adopts a default has made a
language decision on your behalf. Hence the explicit stop in `/rfc-draft` step 4.

---

## 9. First three commands

```
git init Bund2 && cd Bund2
git submodule add https://github.com/vulogov/Bund reference/Bund
# … the other five, then commit the pinned SHAs

# write CLAUDE.md, .claude/settings.json, docs/rfc/0000-template.md,
# and drop the five research documents into docs/research/

claude
> Build tools/corpus: scan reference/Bund/examples/*.bund and tests/*.bund for uses of
> `.id`, `.timestamp`, `bund.eval`, `load.lambdas`, `register`, `lambda*`, and mutation of
> a LAMBDA after construction. Report counts and cite examples. Write the findings to
> docs/registers/open-questions.md against decisions D1, D2, D3 and D5.
```

That single session converts four judgement calls into evidence, which is the right way to
start a project whose method is grounding before design.
