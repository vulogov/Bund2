# Acceptance review — RFC-0006: Ahead-of-time output

- Reviewed: `docs/rfc/RFC-0006-ahead-of-time-output.md` as of `9c33a0f`
  (941 lines), read whole, as an adversarial reader. Fifth review; the first
  whose question is whether the document can be Accepted.
- Date: 2026-10-08.
- Reference SHAs: `git submodule status` matches `reference/PINNED.txt` in all
  six submodules; `git status --porcelain` is empty in each.
- Nothing in the RFC, the registers or ERRATA was edited.
- `cargo xtask conform`: **133/145, CEILING 133/145**, twelve approved
  deviations. `cargo xtask coverage`: IMPLEMENTED 500/505, COVERAGE 489/505.

**Verdict: not yet. The product is sound and every criterion that names a
check still passes. Five things block, and four of the five are the document
or an unruled decision rather than the code.**

## What was checked, and how

- **Every test the criteria name was found and run.**
  `cargo test -p bund2-cli --test bundle_build`: 17 passed. The two unit tests
  criteria 11 and 12 name are in `crates/bund2-cli/src/bundle.rs`
  (`a_program_over_capacity_is_refused_and_writes_nothing`,
  `an_unbuilt_runtime_carries_nothing`); the two criterion 7 names are in
  `crates/bund2-jit/src/lower.rs`.
- **`cargo xtask conform --bundles` was run**: 133/145, CEILING 133/145, no
  failing golden — equal to the source run above, default configuration.
- **`cargo xtask bundle` was run**: debug and release both produce a working
  artefact and the release artefact's signature validates.
- **Bundles were built and run by hand** for the three measurements in B2, B4
  and B5 below. Each is quoted where it is used.
- **The environment variables §B3 names were looked for in the binary**
  (`crates/bund2-cli/src/main.rs`, the carried-program front end).
- **Registers searched by subject**: `parse-at-build`, `--emit=native`,
  `codesign`, `noeval`, `bundle`. Read D82 and D83 whole, and Q38 and Q40.
- **The reference's registration of `debug` and `debug.shell` was read**
  (`reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:155`,
  `reference/Bund/src/stdlib/functions/debug_fun/debug_shell.rs:67`), for B4.

**Not re-run:** `conform --bundles` under `--features jit` and at threshold
1 — criterion 2's third and fourth rows. Criterion 1's size figures were not
re-measured. Nothing on Linux or Windows was run; nothing in this repository
can.

## Criteria

| # | RFC says | found |
|---|---|---|
| 1 | answered | Not re-measured. The ERRATA entry it cites exists. |
| 2 | met, 107/116 in four runs | **Holds today at 133/145** for source and default bundles. The table is a week stale in its numbers and right in its claim. `jit` rows not re-run. |
| 3 | met | Test passes. It is `cfg(not(feature = "jit"))`, as the criterion says it should be. |
| 4 | met | Test passes. |
| 5 | met | Test passes. |
| 6 | met | The test is `cfg(feature = "jit")`; not run in this review. |
| 7 | met, reformulated | Both tests exist. **Contradicted by RFC-0005** — S1. |
| 8 | withdrawn, D83 | D83 says so. **The status paragraph says otherwise** — B1. |
| 9 | met | Test passes. |
| 10 | met | Test passes. **The floor has routes round it the RFC does not name** — B4. |
| 11 | met | Tests pass. `a_built_artefact_validates` is macOS only. |
| 12 | met | Tests pass. The cases tested are not §B1's four — S3. |
| 13 | met | Holds: source `conform` is at its ceiling. |

## Blocking

**B1 — The status paragraph and three passages of the body describe a
document that D83 ended.** D83 withdrew `--emit=native` on 2026-09-30 and
criterion 7 was met the same day without §B5's work. Still standing:

- Status: "8 waits on `--emit=native` existing and 7 is deferred behind §B5."
  Criterion 8 reads "Withdrawn with the mode" and criterion 7 reads "Met".
- Summary, last paragraph: "`--emit=native` is specified to the depth D10 and
  RFC-0005's criterion 4 require, and gated on the measurement §B8 names" —
  two paragraphs under the bullet that says it is withdrawn.
- §B5: "Criterion 7 is deferred behind (1) and (2)."
- The closing paragraph counts "one ungrounded assumption (Q40)". Q40 is
  resolved by measurement in the register and in the bullet above it.

RFC-0005 and RFC-0007 both record that a status line nothing contradicts is
read as authoritative. This one is contradicted by its own criteria list.
Accepting it accepts the contradiction.

**B2 — `BUND2_DUMP_STACK` is named as part of the design and is not what the
binary reads.** §B3: "the names are part of this design rather than left to
the implementation: `BUND2_STATS`, `BUND2_DUMP_STACK`, `BUND2_RAW_VALUES`".
The front end reads `BUND2_NO_DUMP_STACK`, and the default is on. Measured on
a bundle of `1 nosuch`:

| environment | stack dump in the report |
|---|---|
| none | yes |
| `BUND2_DUMP_STACK=0` | **yes** — the variable the RFC names does nothing |
| `BUND2_NO_DUMP_STACK=1` | no |

The implementation's choice is the sensible one, since the CLI's default is
also on. The document is what is wrong, and no criterion would have caught
either reading.

**B3 — Parse-at-build is built, tested, and has never been ruled on.** The
open questions list it as "flagged rather than assumed", D82 lists it under
"what is not settled", and the preservation table files it as "deliberately
changed" with nothing behind it. `a_syntax_error_fails_the_build_and_writes_nothing`
holds the behaviour. The closing paragraph then says "no decision waits and
no default is being adopted by omission". It is an observable change in when
an error is seen, taken without a ruling. It is very likely the right call.
It still needs the owner's sentence before the document that carries it is
Accepted.

**B4 — §B3a does not name what the restriction flags leave ungated, and D78
requires it to, by name.** It names `compile lambda! !` and stops. A bundle
built `--noeval --noio` reads text from standard input and runs it by three
more routes, measured on such a bundle:

| program | typed | printed |
|---|---|---|
| `debug.shell` | `40 2 + println` | `42` |
| `debug.step`, then anything | `40 2 + println` at the stop | `42` |
| the same | `"1" bund.eval` at the stop | refused — the stub holds |

`debug` and `debug.shell` are the reference's own and it registers both with
no gate (`debug_debug.rs:155`, `debug_shell.rs:67`), so this is D79's boundary
and not a defect in the flag. The console route is new since D113 and no
review of this RFC has seen it. All three belong in §B3a's list, because D76
puts the risk on the person running the artefact and the list is what they
would read.

**B5 — A bundle now stops at a debugger console, and the front end's own
comment says it never does.** The carried-program path sets `debugger: false`
and explains why at length: a switch that "would let an environment variable
turn a shipped program into one that blocks on stdin" is "a worse failure than
not being able to debug it". Since D113.5 a word reaches the same place.
Measured, a bundle of `"a" println debug.step "b" println`:

- with standard input closed: prints `a`, the console's banner, `detached`,
  `b`. Exit 0.
- with lines on standard input: stops, answers `st`, runs a typed line.

So the route is a word in the program rather than a variable in the
environment, which is the author's choice in the way `input` is. That may be
exactly what is wanted. But §B3 says nothing about it, the comment says the
opposite, and a bundle whose input never closes will wait at the stop. It is
a decision about what a shipped artefact does and it was made by a different
RFC's amendment. It needs a ruling and a sentence in §B3.

## Should fix

**S1 — RFC-0005 and RFC-0006 disagree about RFC-0005's criterion 4.** This
document's status and criterion 7, and D83, say it is discharged by
inspection. RFC-0005's status list says "Not met — 4's reachable remainder
alone … belongs to RFC-0006's AOT path". One of the two is stale. It is
raised again in RFC-0005's review, where the criterion lives.

**S2 — The preservation table cites the wrong criterion.** "A damaged or
absent trailer … (§B1, criterion 11)". Criterion 11 is signing; the damaged
artefact is criterion 12, as §B1 itself says.

**S3 — §B1's four cases are not the four criterion 12 tests.** §B1 lists no
trailer, a truncated trailer, an implausible length and unreadable text. The
test drives a `state` byte of 7, a length past capacity and a lone
continuation byte. A truncated trailer cannot occur in a fixed in-image
region, and a bad state byte is not in §B1's list. The test is right for what
was built; §B1 still describes the appended form.

**S4 — "The trailer" survives the design that removed it.** §B1 opens by
specifying an appended payload and a trailer read from the executable's end,
then reverses to an in-image region. §B3, §B3a, §B4 and the preservation
table go on saying "the trailer". A reader who stops before the reversal has
the wrong construction, and the reversal is 40 lines in.

**S5 — Criterion 2's figures are a week old.** They read 107/116. Today's
run is 133/145 for both. The claim is unchanged and the note should carry a
dated re-run, as RFC-0004's acceptance note does.

**S6 — D82's fourth unsettled item stands.** `cargo xtask bundle` is outside
`cargo test` and outside `conform`. It is the check that found both
release-only blockers. Nothing runs it unless someone remembers to.

## What accepting would mean

Accepting with the five blockers answered accepts three stated limits, and
they should be in the status line in the way RFC-0009's dormant criterion is:

- **macOS on arm64 is the only platform anything was measured on.** The ELF
  and PE cases are "reasoning from the formats".
- **A notarised, Developer-ID-signed artefact is untested.**
- **Whether `/usr/bin/codesign` is a Command Line Tools stub on a machine
  without them is unverified** (D81).

## Summary

| | count |
|---|---|
| Blocking | 5 |
| Should fix | 6 |
| Criteria that fail today | 0 |

B1, B2, S2, S3, S4 and S5 are edits to the document. B3 and B5 are rulings.
B4 is three lines in §B3a. None needs code, unless B5 is ruled the other way.
