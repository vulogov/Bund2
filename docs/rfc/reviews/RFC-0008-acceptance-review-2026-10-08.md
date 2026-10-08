# Acceptance review — RFC-0008: Debugger and observability

- Reviewed: `docs/rfc/RFC-0008-debugger-and-observability.md` as of `9c33a0f`
  (1,426 lines), read whole, as an adversarial reader. Second review; the
  first whose question is whether the document can be Accepted.
- Date: 2026-10-08.
- Reference SHAs: `git submodule status` matches `reference/PINNED.txt` in all
  six submodules; `git status --porcelain` is empty in each.
- Nothing in the RFC, the registers or ERRATA was edited.
- `cargo xtask conform`: **133/145, CEILING 133/145**, twelve approved
  deviations. `cargo xtask coverage`: IMPLEMENTED 500/505, COVERAGE 489/505.

**This review is not independent for the last third of the document.** The
amendment of 2026-10-07 — "debugger words (D113)" and everything after it,
lines 952 to 1,426 — was written in the two sessions before this one by the
same hand that writes this review. Findings against it are given, and a
reviewer who did not write it is still owed. B5.

**Verdict: not yet. Every named test exists and the suite passes. One
criterion is false as written, the hot path has changed since it was
measured, and the status line understates what is unbuilt.**

## What was checked, and how

- **The whole workspace was run**: 626 passed, 0 failed in the default build;
  748 passed, 0 failed under `--features jit`.
- **Each test the base criteria name was located by name**: in
  `crates/bund2-stdlib/src/logging.rs`, `crates/bund2-cli/tests/debugger_step.rs`,
  `crates/bund2-interp/src/debug.rs`, `crates/bund2-interp/src/lib.rs` and
  `crates/bund2-runtime/src/tier.rs`.
- **`tests/golden/`, `tests/probes/` and `xtask coverage`'s uncovered list
  were read** for criteria 5 and 13.
- **The CLI's flag parser and the console's command parser were read** for
  what §D3, §D4, §D5 and the flag table specify
  (`crates/bund2-cli/src/main.rs`, `crates/bund2-cli/src/debugger.rs`).
- **`config_home` was read** (`crates/bund2-stdlib/src/terminal.rs`), for §D8.
- **`git log dff32be..HEAD -- crates/bund2-interp/src/lib.rs`**, for what
  reached the frame loop after criteria 11 and 12 were measured.

**Not done:** no benchmark was run. Criteria 11 and 12 are read as recorded.

## Criteria

| # | RFC says | found |
|---|---|---|
| 1 | met | Test present; passes. |
| 2 | met as restated, D90 | Tests and probe present. It names the module `bund2_stdlib::console`; the code is in `logging.rs`. |
| 3 | met for four tags | `tests/probes/debug-dump.bund` and its golden pass. |
| 4 | met | Holds today: 133/145 at its ceiling. |
| 5 | met as restated | **Its table is false today** — four words it lists as not implemented are implemented, and two it calls ungoldenable have a golden. B1. |
| 6 | met over 134 of 136 | The sweep passes today over the current suite; the two exclusions are still the two. Its reason for them is out of date — S5. |
| 7 | met | Present. Extended on 2026-10-07 to natives and aliases. |
| 8 | met | Present. |
| 9 | met | All three tests present. |
| 10 | met | Test present; passes under `jit`. |
| 11 | met, inside the band | **Measured on code that has since changed** — B2. |
| 12 | met for the symbol | `a_frame_is_the_size_it_was_measured_at` passes. Same exposure as 11. |
| 13 | "refused as unstable" | **False** — B1. |

## Blocking

**B1 — Criterion 13 says a golden cannot hold `debug` and `debug.shell`. One
does.** The criterion: "the table embeds a fresh id and stamp per run, so two
captures differ and the capture refuses them … belongs in `UNSTABLE.txt`".
Found:

- `tests/probes/debug-repl-words.bund` runs both words, and
  `tests/golden/probes/debug-repl-words.golden` is captured and passes. Its
  header says why: the capture normalises the id and stamp in the `Debug`
  row, as it does for `debug.display_stack`.
- `tests/golden/UNSTABLE.txt` names neither word.
- `xtask coverage` lists eleven words no golden can hold. `debug` and
  `debug.shell` are not among them; both count as covered.

Criterion 5's table repeats the claim — "`debug`/`debug.shell` are goldenable
by no capture" — and lists `debug`, `debug.shell`, `debug.display_memstat`
and `debug.display_distributed_info` as not implemented. All four are
implemented.

**This had a consequence the day before this review.** The options put to
the owner for the `debug` word said no golden could hold it, so changing it
would move no health number. That was wrong: making `debug` a stepping
wrapper would have failed `debug-repl-words.golden`. The option was not
taken, so nothing broke. The recommendation rested on a false sentence from
this criterion, repeated without checking.

**B2 — The frame loop and the dispatch path have changed since criteria 11
and 12 were measured, and nothing has re-measured them.** Criterion 11's
claim is that "the per-step branch is inside the 5% band", read on
2026-10-03. Since `dff32be`, 232 lines have been added to
`crates/bund2-interp/src/lib.rs`, and three of the additions sit where calls
are made, attached debugger or not:

- `push_frame` tests `tier_off` before offering a body to the tier (D113.6).
- `call_native` tests whether a debugger is attached before every native
  runs (the native breakpoint, 2026-10-07).
- `dispatch` calls `alias_breakpoint` in both its lambda arm and its native
  arm, which tests the same thing again.

Each is one predictable branch and is very probably inside the band. That is
the argument criterion 11 was written to replace with a measurement. The
document also records that this host can no longer support a 5% band at all
— "it cannot have one" — so how the cost is to be shown is a decision before
it is a number.

**B3 — "That is the whole gap" is not the whole gap.** The status paragraph
names two things as specified and unbuilt: §D7's trace and §D8's console
history. Also specified, unbuilt, and covered by no criterion:

| specified in | what | found |
|---|---|---|
| §D3 | a breakpoint by source location | the console parses `break <word>` and `break <word> if …` only |
| §D4 | `watch depth > 100` | the console parses `watch @stack` and `watch workbench` only |
| §D5 | `words` as a view with tier, generation and call count; `classes` | neither exists as a word or a console command |
| the flag table | `--debug-shell`, `-d`/`--debug`, `--profile` | none is parsed by `main.rs` |

Criterion 2 and D90 record that the CLI has no `--debug` count, and the open
questions ask whether `--profile` is in scope. `--debug-shell` appears
nowhere after the table that introduces it. A status line that enumerates
the gap and stops short is the pattern RFC-0007's status paragraph was
rewritten to end.

**B4 — The preservation table promises things the document later withdraws
or disproves, with no mark at the row.**

- "`debug`, `debug.shell` — **Preserved, and extended.** `"…" debug` keeps
  working as a thin wrapper … It then steps *into* things." Withdrawn a
  thousand lines later ("`debug.run`, and the row for `debug`"). RFC-0003
  marks a superseded passage where it stands. This row is unmarked.
- "`Frame`'s size — **Changed**, and the cost is unmeasured — §D2."
  Criterion 12 measured it: 48 to 56 bytes, no detectable cost.
- The amendment's own table still carries two conditionals that were ruled
  on: "less `stack`/`st` if D113.4 rules so" and "`debug` may gain the
  stepping its row promised; that is D113.5's to say".

**B5 — A third of the document has never been reviewed, and this review
cannot do it.** The amendment adds fourteen words, two `Vm` methods, a new
console, nine criteria (W1 to W9) and four later sections. It changes two
claims in the reviewed part: criterion 6's "a program that reads standard
input cannot be debugged at all" is ended by `debug.feed`, and the status
line's account of history is ended for the attached console. Its only reader
so far is its author. Two defects in it were found by its author while
building — the alias breakpoint and the `debug.run` gap — which is evidence
that an outside reading would find more.

## Should fix

**S1 — §D8 gives the wrong directory for macOS.** It says the history goes
under "`$XDG_CONFIG_HOME`, else `$HOME/.config`, else `%APPDATA%`".
`config_home` uses `~/Library/Application Support` on macOS, and that is
where the file was found when the console was driven at a terminal.

**S2 — The current-behaviour table is a week out of date and undated.**
"Bund2 has three of the thirteen", with "no" against `debug`, `debug.shell`,
`debug.dump`, `debug.display_memstat`, `debug.display_distributed_info` and
the five `log.*`. All are implemented.

**S3 — An answered question is still open.** "Whether the safepoint check is
affordable … criterion 11 is it." Criterion 11 answered it on 2026-10-03.
(B2 reopens the measurement; the question as listed predates that.)

**S4 — The status tally does not match the list.** "Eleven met, criterion 4
met on each capture, and one withdrawn for naming no check." No criterion in
the list reads withdrawn; the span half of criterion 12 is, and criterion 13
is a statement that B1 shows to be false.

**S5 — Criterion 6's exclusions are right and their explanation is stale.**
`terminal-words` and `debug-repl-words` are still excluded and the sweep
still passes. The note says such a program "cannot be debugged at all".
Since §W4 it can, with `debug.feed`. The exclusion stands because the sweep
feeds nothing.

**S6 — The amendment corrects itself in sequence rather than in place.**
"Thirteen words" becomes fourteen four sections later; "no test in the suite
holds a terminal" stands above the section that adds one; "a breakpoint on a
native never fires" stands above the section that makes it fire. Each
correction is dated and honest. A reader who stops early has the old state.

## What accepting would mean

- **§D7 is unbuilt**: no execution trace, no profiler.
- **Source locations are unavailable** to `bt` and to breakpoints until
  RFC-0003 §S5's IR exists.
- **`--debug-shell`, `--debug` and `--profile` are reference flags Bund2
  does not have.**
- **The `--debugger` console keeps no history**; the console a word attaches
  does.

## Summary

| | count |
|---|---|
| Blocking | 5 |
| Should fix | 6 |
| Criteria that fail today | 1 (criterion 13, false as written) |

B1, B3 and B4 are edits. B2 is a decision about how to measure, then a
measurement. B5 is a review by someone else.
