# Acceptance review — RFC-0006: Ahead-of-time output

- Reviewed: `docs/rfc/RFC-0006-ahead-of-time-output.md` at `d469bf6` (2,605
  lines), read whole. The eighteenth review, and the second acceptance
  review; the first is `RFC-0006-acceptance-review-2026-10-08.md`.
- Date: 2026-10-10.
- **Who read it.** The session that wrote the answers to the fifteenth,
  sixteenth and seventeenth reviews. The seventeen before this one were
  written by another reader. So this is not an independent reading of the
  last three revisions, and what it is worth is what it ran: every check the
  seventeenth review listed as "not done" was done here.
- Reference SHAs: `git submodule status` matches the RFC's header in the
  four submodules it names and `reference/PINNED.txt` in all six;
  `git status --porcelain` is empty in each.
- Nothing in the RFC, the registers, the measurements or ERRATA was edited
  before this file was written.
- `cargo xtask conform`: **133/145, CEILING 133/145**, twelve approved
  deviations, baseline 133. `cargo xtask coverage`: IMPLEMENTED 500/505,
  COVERAGE 489/505. `cargo xtask cite`: every citation resolves.
  `cargo xtask lint`: no contradictions.

**Verdict: ready to accept once one line is added to a workflow. No
finding is in what a bundle does or in what the fetch does, and none needs
a ruling. Every criterion that can be run was run and passed. Criterion
14's gate does not run on a change to the file that holds two of the
fetch's eight words, or to the one that draws a failed fetch's report,
and its text reads as though it did (S1). Seven sentences were left
behind by the two commits after the seventeenth review; each says less
than is now true, or counts wrong, and none denies a deviation.**

## What was checked, and how

- **The document was read whole**, with the seventeenth review beside it,
  and each of that review's findings was looked for in the text.
- **Run on this tree, macOS arm64, 2026-10-10:**

  | what | result |
  |---|---|
  | `cargo test --workspace --no-fail-fast` | 667 passed, 0 failed, 1 ignored |
  | the same with `--features jit,relocation-test` | 792 passed, 0 failed, 1 ignored |
  | `cargo xtask conform` | 133/145, CEILING 133/145, none failing |
  | `cargo xtask conform --bundles` | 133/145, CEILING 133/145, none failing |
  | `cargo xtask conform --features jit` | 133/145, CEILING 133/145, none failing |
  | `cargo xtask conform --features jit --jit-threshold 1` | 133/145, CEILING 133/145, none failing |
  | `cargo xtask conform --bundles --features jit` | 133/145, CEILING 133/145, none failing |
  | `cargo xtask conform --bundles --features jit --jit-threshold 1` | 133/145, CEILING 133/145, none failing |
  | `cargo xtask bundle` | debug and release artefacts run and see their argument; the release signature validates |

- **The Linux gate**: `gh run list` shows run 38049339369 of
  `measure-fetch` on `dde6ed4` completed with success. The workflow file
  was read whole; its last step fails on an empty extraction and on any
  differing line.
- **Measured by hand on a bundle built `--noeval`** from a copy of the
  debug binary: `./p -- x y` gives the program `-- x y` and
  `bund2 script --file p.bund -- x y` gives it `x y`; neither holds the
  artefact's name; `3 bund.exit` exits 3 from both; `--inspect` prints the
  restriction, `bund2 0.0.0` and the six pinned SHAs; and
  `"40 2 + println" !!` with `BUND2_NOEVAL=0` answers
  `bund EVAL functions disabled with --noeval` and exits 0.
- **The citations added since `e2fc0b0` were opened**:
  `reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:48` and
  `:119` load and save `bund_debug_debugger_history.txt`, and
  `reference/Bund/src/stdlib/functions/create_aliases.rs:13` registers `!!`
  as `bund.eval`. The preservation row "How long a fetch waits" was checked
  on both sides: `reference/Bund/src/stdlib/helpers/file_helper.rs:42-54`
  sets a user agent, a method and a URL and no timeout, and
  `crates/bund2-stdlib/src/http.rs`, `get`, calls `TcpStream::connect` and
  sets none.
- **Symbols named since `e2fc0b0`**: `place`, `reseal`, `run`, `run_cli`,
  `run_carried`, `env_set`, `build_request` and `features_built_in` are in
  `crates/bund2-cli/src/main.rs`; `exit_requested` is called once, in
  `run`, and `run_carried` returns what `run` returns. The test
  `both_floors_hold_for_every_word_they_name` was read: it builds a bundle
  for `!!` and one for `cwd`.
- **The registers were searched by subject** for what the last two commits
  changed: D134 to D137, Q42, F186, F188.

**Not done, so not claimed.** The macOS fetch table was not run again; the
recorded one is of `6c84d9e` with a hand re-run noted in its header. No
citation into `reference/` older than `e2fc0b0` was re-opened: the
seventeenth review opened all of them and `cite` resolves them. Criterion
1's sizes were not taken again. No release artefact was built with a
restriction. Nothing was run on Linux by this review except through the
workflow's recorded run.

## The seventeenth review's findings, each looked for

| finding | answered |
|---|---|
| B1, D133 has no preservation row and the fetch row denies it | **Yes.** The row "How a response is read, and how long a request may be, on Linux" names the sixteen by kind and by number, and the fetch row says "On macOS, against libcurl 8.7.1" before "fails closed" |
| S1, 62 names | Yes: 66, with the old number noted |
| S2, `!!` | Yes: §B3a, the preservation row, criterion 10 and the test |
| S3, "the table in the repository is that run" | Yes |
| S4, the file's date | Yes: §B7 and the file's own header |
| S5, D52 | Yes: consumed, and cited at the exit code |
| `run_cli`, RFC-0005:3031-3039, D122's wording | Yes, all three |
| four uncited claims | Yes: `debug_debug.rs:48` and `:119`, F152, the summary of "I/O" reworded, and `place` and `reseal` named for what is read from code |
| nothing holds 133 | **Yes, and mechanically since `d469bf6`**: `tests/golden/CONFORMANCE.txt` reads `133/145` |
| no criterion covers the fetch | **Yes: criterion 14, D136, and it has passed once on Linux** |
| five things the preservation table did not enumerate | Yes: D133, `!!`, the wait, the disclosed path, a report's wrapping |
| seven assumptions | Yes: the section "What this document assumes" |

## Should fix

### S1 — criterion 14's gate does not run when two of the fetch's words change, or its report

Criterion 14: "It fails on a change to the fetch that moves a cell", and
"It runs on a push that touches the fetch, the script or the recorded
table". §B7 gives the list: "the script, `host.rs`, `http.rs` or the
recorded table". The workflow's `paths` are those and the workflow file
itself.

`fetch_uri` is called from two files, `crates/bund2-stdlib/src/host.rs`
and `crates/bund2-stdlib/src/singles.rs`. `use`, `file`, `url` and their
workbench forms are in the first. The second holds `bund.eval-file` and
`bund.eval-file.` (`bund_eval_file_base`), two of the eight words and rows 721 to
733 of the table, and it is not in `paths`. A cell of the table is what a
program printed. So a change to `bund.eval-file` that printed something
else, or evaluated what it should refuse, would be pushed with the gate
asleep and found on the next push that happens to touch one of the five
listed files. The same is true of `crates/bund2-stdlib/src/report.rs`, which draws the report a failed
fetch prints: F186 changed it the day the table was recorded, and the
seventeenth review listed "that the 741 rows still hold" as assumed for
exactly that commit.

The criterion says in so many words that the gate is not on every push.
What it does not say is that "the fetch" leaves out two of its eight words
and the report every failing row prints.

**What would answer it:** `crates/bund2-stdlib/src/singles.rs` and
`crates/bund2-stdlib/src/report.rs` in the workflow's `paths`, and §B7's
list and D136's widened to match. Or the sentence narrowed to the two
files. The first costs a 21-minute job on more pushes.

### S2 — "Each 'yes' means 'would', and none has happened" is false of criterion 14

The paragraph under the table "What runs these": "Every criterion here has
been run on one machine, macOS on arm64, by hand or by `cargo test`
there." The row above it for criterion 14 says "yes, for Linux", and the
criterion records run 38049339369. The paragraph was true of `ci.yml`,
whose one run is still the failure of 2026-09-11, read here with
`gh run list`. It is not true of `measure-fetch`, which has run five
times.

### S3 — the table's row for criterion 13 predates the baseline

"13 | a before-and-after `conform` on 2026-09-30 | no". Since `d469bf6`
what checks it is `cargo xtask conform` against a baseline of 133, and the
`conformance` job in `ci.yml` runs that command.

### S4 — "which CI runs" claims a run that has not happened

Criterion 13's last note: "From here a count below 133 fails `conform`,
which CI runs." `ci.yml` runs on a push to `main` and on pull requests.
The branch this is on has never run it. By the document's own S7 rule the
word is "would".

### S5 — "Six things the artefact's front end must settle" has seven bullets

§B3. The bullet "`args` does not hold the artefact's own name" was added
for the seventeenth review and the count above the list was not moved. The
count of "Six things about this list" under the first bullet is right, and
so is "Seven more things `bund2 build` does".

### S6 — four sentences say an owner's action is still owed, and each is done

- The open question on Q42: "CLAUDE.md's terminology still defines AOT as
  'the cranelift-object build'; that file is the owner's and D134 proposes
  its wording without making the edit." The owner asked for the edit and
  it is in `d469bf6`.
- D134 in `docs/registers/decisions.md` says the same: "The owner did not
  ask for the edit, so it is not made."
- D137's status: "**Not yet carried out when this entry was written**".
  True as dated. It has no note that it was carried out, and criterion 13
  has one.

A fourth of the same kind is in
`docs/measurements/fetch-linux-2026-10-09.md`: "On Linux the first run of
the gate is what confirms it." It has.

### S7 — criterion 2's evidence is two days and four changes old

The six-row table is of 2026-10-08. Since then D131 replaced the client
that fetches, D132 moved four words behind a flag, F186 changed how every
report is broken, and D134 renamed a feature. None is expected to move a
golden and none did: the six configurations were run here and each reads
133/145, CEILING 133/145, none failing. The criterion should carry that
date, or a reader takes the table for the tree it is printed in.

### S8 — the status header's tally stops at twelve

"Criteria 3, 4, 5, 6, 9, 10, 11 and 12 pass … criterion 2 is met … 7 is met
… 8 is withdrawn." Criteria 13 and 14 are in the list below with their
evidence and are not in that line.

## Acceptance criteria

| # | asks for | found |
|---|---|---|
| 1 | a figure for what the code generator costs | **Answered**, 2026-09-30; a dated measurement, not retaken |
| 2 | every bundled golden matches its source run, in both configurations | **Met.** Run here in six configurations — S7 |
| 3 | no code generator in a default artefact | **Met.** `bundle_build` passes; its companion under `jit` passes |
| 4 | built with no compiler reachable | **Met**, on macOS |
| 5 | 1024 levels bundle and run | **Met** |
| 6 | a JIT bundle enters compiled code | **Met**, under `--features jit` |
| 7 | no relocation names one compiled body from another | **Met**: two tests more under `jit,relocation-test` than under `jit` |
| 8 | `--emit=native` conformance | **Withdrawn**, D83 |
| 9 | a literal's stamp is a run-time value | **Met** |
| 10 | a restriction cannot be loosened | **Met**, and by hand for `!!` |
| 11 | the artefact validates; over capacity is refused | **Met**, and `cargo xtask bundle` validates the release one |
| 12 | a damaged artefact is refused | **Met** |
| 13 | conformance moves by zero | **Met, and held**: baseline 133 |
| 14 | the fetch does what its recorded table says | **Met for Linux**, one run; macOS by hand. Its trigger is S1 |

## What accepting would mean

- **A bundle has been built and run on one machine.** macOS, arm64. The
  write into an ELF image has never run, and PE has no runner. The document
  says so in §B1, in its summary and under the criteria table. `ci.yml`
  would run `bundle_build` on Ubuntu the first time this branch reaches
  `main` or a pull request, and that run is the first evidence either way.
- **Sixteen rows on Linux are approved to differ** (D133), in four of which
  Bund2 evaluates a body the Linux reference refuses. A job keeps the list.
- **The fetch's reader is a model of one libcurl**, checked on 233 shapes
  of answer, and the document says it is not libcurl's reader.
- **`--noio` and `--noeval` are word-group switches.** The document names
  what each leaves and says the name to audit for is `!`.
- **A release artefact's floor rests on `cargo xtask bundle`**, which is in
  neither `cargo test` nor CI: D82's fourth item, still unsettled and said
  to be.
- **`--emit=native` stays withdrawn**, with §B5 as the record of what it
  would take.
- **Nothing is listed as open**, and no default is adopted: every decision
  in "Decisions consumed" is RESOLVED.

## Summary

| | count |
|---|---|
| Blocking | 0 |
| Should fix | 8: one line in a workflow, seven corrections of text |
| Criteria met | 12 of 12 that stand; 8 is withdrawn and 1 is a measurement |
| Needs a ruling | nothing |

S1 is the one with weight: the gate that holds ten decisions should wake
when any word it measures changes, and when the report it reads does.
