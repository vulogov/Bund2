# Acceptance review — RFC-0005: The Cranelift tier

- Reviewed: `docs/rfc/RFC-0005-cranelift-tier.md` as of `9c33a0f` (6,789
  lines), as an adversarial reader. Twenty-second review; the first whose
  question is whether the document can be Accepted.
- Date: 2026-10-08.
- Reference SHAs: `git submodule status` matches `reference/PINNED.txt` in all
  six submodules; `git status --porcelain` is empty in each.
- Nothing in the RFC, the registers or ERRATA was edited.
- `cargo xtask conform`: **133/145, CEILING 133/145**, twelve approved
  deviations. `cargo xtask coverage`: IMPLEMENTED 500/505, COVERAGE 489/505.

**This is an audit of standing, not a re-reading of the design.** Read: the
status paragraph's lists and the passages they point to; criteria 2, 4, 8, 18
and 30 whole; the open questions; the amendment of 2026-10-07. **Not read:**
§S1 to §S11, the assumptions list, the preservation table, and the other
twenty-five criteria beyond their place in the tally. Twenty-one reviews have
read the design. None has asked whether the status line is true, and that is
what this one found wrong.

The amendment of 2026-10-07 was written by the same hand as this review. B4
is a finding against it.

**Verdict: not yet — but for different reasons than the document gives. By
its own criteria notes the tally is twenty-eight met and two measured, with
nothing partial, unmet or deferred. What blocks is that one met criterion
fails today, the status line misreports three others, the tier's seam has
changed without this document saying so, and the stated bar — a review pass
that finds nothing — has still not been reached.**

## What was checked, and how

- **The whole workspace under `--features jit`**: 748 passed, 0 failed,
  1 ignored. Default build: 626 passed, 0 failed.
- **Criterion 2's three runs**, today:

  | run | conformance | ceiling |
  |---|---|---|
  | `cargo xtask conform` | 133/145 | 133/145 |
  | `--features jit` | 133/145 | 133/145 |
  | `--features jit --jit-threshold 1` | 133/145 | 133/145 |

- **`cargo xtask cite` and `cargo xtask lint`**, for criterion 8.
- **The status lists against each criterion's latest dated note**, for 4, 18
  and 30.
- **`git log dff32be..HEAD -- crates/bund2-interp/src/lib.rs`** and the
  `push_frame` seam, for what D113 changed beside the tier.
- **`every_reentering_function_is_named`** in `crates/bund2-stdlib/src/lib.rs`,
  for the path set the amendment says it extended.

**Not done:** no benchmark was run, so criteria 7, 9, 10 and 17 are read as
recorded. Whether the two `jit` conformance runs compiled anything was not
checked; the tool's summary does not say.

## Blocking

**B1 — Criterion 8 is listed as met and fails today.** It asks for "`cite`
and `lint` clean". Today:

- `cargo xtask cite`: **4 citation defects.** One is in this document, at
  its line 3068: `crates/bund2-api/src/lib.rs:61`, quoting
  `pub type NativeFn = …`. That line is now 62. The other three are
  RFC-0007's. A line number into `crates/` is also what Q30 rules out — name
  the file and the symbol — and this is the reason for the rule: the citation
  went stale when a doc comment grew by a line.
- `cargo xtask lint`: **2 inconsistencies**, both RFC-0000's counts of
  register entries (94 against 113 decisions, 149 against 178 defects).

Neither is this RFC's design. Both are this RFC's criterion.

**B2 — The status paragraph misreports three criteria, each against its own
latest note.** The paragraph says "count the lists, not this sentence". The
lists are what is stale:

| # | status list says | the criterion's latest note says |
|---|---|---|
| 4 | "**Not met** — 4's reachable remainder alone … belongs to RFC-0006's AOT path" | "**Met, 2026-09-30**, by inspection rather than by a product — D83." The older note is kept beneath it as "what was true for seventeen days". |
| 18 | "**Deferred, with a named blocker** — 18 alone, behind `:` and `;` being bound" | "**Met, 2026-09-30 — both halves.** F84's blocker is gone." |
| 30 | "**Partial** — 30 alone" | "**Met, with the caller-is-a-native mirrors excluded as unreachable — 2026-09-29**" |

So "25 met, 2 measured, 1 partial, 1 not met, 1 deferred" should read 28 met
and 2 measured. The error runs against the document's own interest, which is
why nobody caught it: a status that understates is not challenged. RFC-0006's
status and D83 both cite criterion 4 as discharged, so two documents and a
decision already disagree with this line.

Older paragraphs lower in the same status block compound it. "**What still
blocks Accepted**: criterion 10 reads below its own stop rule, and criteria
22 and 27" stands unmarked, 150 lines under the list that records 10 as not
triggering the rule and 22 and 27 as met.

**B3 — No review pass has found nothing.** The document states the bar
itself: "RFC-0000's bar for Accepted is a review pass that finds nothing …
this RFC's twenty-first pass still found two blockers. Both are answered
since, but **no pass has yet found nothing**". This is the twenty-second and
it found five. It also did not read the design, so a clean result here would
not have met the bar either.

**B4 — D113 changed the seam between Tier 0 and the tier, and this document
records one third of it.** The amendment of 2026-10-07 covers
`evaluate_typed` and one new re-entering function. It was written when Part
A landed. Since then:

- **`push_frame` no longer offers a body to the tier once `tier_off` is
  set.** §S7's seam is "offered only for a body with a key and no exit
  action". There is now a third condition, set by the first debugger word
  that arms or moves (D113.6). It is described in RFC-0008 and in the code,
  and not here, where the seam is specified.
- **The amendment's own account is out of date.** It says "a session started
  with `--debugger` installs no tier … Where an embedder attaches a debugger
  beside a tier …". Since Part B the CLI itself attaches one beside a tier
  whenever a script arms, which is no longer an embedder's edge case.
- **A compiled body that arms a breakpoint finishes compiled, with no stop
  inside it.** That is a statement about what compiled code does. It is
  recorded in D113's note and nowhere in this document.
- **§S8's path set gained a second entry**, `terminal.rs: debug_run`
  (2026-10-07). The amendment names only `lib.rs: eval_line`. Criterion 11
  measures a cost per path in that set.

None of this is believed to be unsound: `a_breakpoint_armed_beside_a_tier_still_fires`
passes in both builds. It is unreviewed design in an accepted-to-be document,
written by the reviewer.

**B5 — Criterion 7's verdict predates changes on the path it measures.** It
was met on three guarded runs on 2026-09-15: every row of five groups inside
5%. Since `dff32be` the frame push tests `tier_off`, every native call tests
for an attached debugger, and dispatch tests it again for aliases. RFC-0008's
review raises the same point against that document's criteria 11 and 12.
Each is one branch. The criterion exists because "one branch" was not to be
taken on trust, and RFC-0008 records that this host can no longer resolve a
5% band.

## Should fix

**S1 — Criterion 4's slot-target half is "deliberately not built".** Its
note argues the property holds by construction and declines to add an
accessor to test it. That is a defensible choice. It means the criterion is
met for relocations and argued for slot targets, and the status should say
so in the way RFC-0009's says "with criterion 2 dormant".

**S2 — Criterion 30's exclusion is a standing condition, not a closed
matter.** Its mirror cases are excluded because §S7 compiles on an entry.
The note names the trigger that reopens them: an ahead-of-entry or
background compile. D83 withdrew the mode most likely to pull that trigger.
D91's shared compile work is the next candidate and does not mention it.

**S3 — Two open questions are open.** Q27, which target profile the caps
are chosen for, "is still unanswered". Q28 is narrowed to whether s390x is
shipped at all. Neither blocks; both should be named in what acceptance
means.

**S4 — The status paragraph is 700 lines.** It carries five generations of
tallies, each kept "because it is accurate for its date". The practice is
sound for a criterion's note. In the paragraph a reader checks first, it has
produced B2: the current tally is not findable without reading the criteria.

## What accepting would mean

- **Two criteria are measurements, not passes**: 9 (the sync's crossover)
  and 10 (2.40× and 2.31× on the shape the stop rule names).
- **Criterion 30 excludes two cases on a stated mechanism** that could
  change.
- **Criterion 4 is half argued.**
- **Nothing in the default configuration compiles a body in any corpus
  program** (F139, D74's threshold of 1024). The tier is correct on the
  evidence and idle on the corpus.

## Summary

| | count |
|---|---|
| Blocking | 5 |
| Should fix | 4 |
| Criteria that fail today | 1 (criterion 8) |
| Criteria the status line misreports | 3, all in the pessimistic direction |

B1 is three citation fixes and two counts. B2 is an edit. B4 is an amendment
and then someone else's review of it. B5 is the same decision RFC-0008 needs.
B3 is the one that takes a full reading: the twenty-third pass, over the
whole document, by a reader who did not write any of it.
