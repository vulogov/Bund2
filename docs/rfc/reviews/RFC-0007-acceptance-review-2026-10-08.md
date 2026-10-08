# Acceptance review — RFC-0007: Concurrency, and the two buses

- Reviewed: `docs/rfc/RFC-0007-concurrency-and-the-two-buses.md` as of
  `9c33a0f` (745 lines), read whole, as an adversarial reader. Second review;
  the first whose question is whether the document can be Accepted.
- Date: 2026-10-08.
- Reference SHAs: `git submodule status` matches `reference/PINNED.txt` in all
  six submodules; `git status --porcelain` is empty in each.
- Nothing in the RFC, the registers or ERRATA was edited.
- `cargo xtask conform`: **133/145, CEILING 133/145**, twelve approved
  deviations. `cargo xtask coverage`: IMPLEMENTED 500/505, COVERAGE 489/505.
  `bund2 words` lists 610 names.

**Verdict: not yet. Every test the criteria name exists and passes, in the
default build and under `--features jit`. One blocker is in the code: the
binary takes a deviation this document says is not taken. The other four are
the document contradicting itself or the register.**

## What was checked, and how

- **Every test the criteria name was found and run.**
  `cargo test -p bund2-async`: 11 passed, 1 ignored (a measurement, D91).
  `noio_replaces_every_one_of_the_eight` and `a_received_value_keeps_its_stamp`
  are in `crates/bund2-stdlib/src/bus.rs`; criterion 9's `compile_fail`
  doctest is on `Interp` in `crates/bund2-interp/src/lib.rs`, with a positive
  control beside it. The whole workspace under `--features jit`: 748 passed,
  0 failed.
- **`cargo xtask cite` was run.** Three of its four defects are in this
  document — B5.
- **The oracle was run** (`target/oracle/release/bund --nocolor script`) on
  the program in B1, beside Bund2.
- **Registers read**: D85 to D89, D91 to D93 by their status lines; F118 for
  the codec's bound. Searched for `300-deep`, `nests 257` and
  `Error enveloping`, for B1.
- **Counted**: `WordKind::Sync` in `crates/bund2-stdlib/src`, and any writer
  of `Blocking` or `Async` outside `bund2-api`.

**Not checked:** §C8's address-space arithmetic, the D91 measurement, and
every claim about zenoh, which is out of scope by D87.

## Criteria

| # | RFC says | found |
|---|---|---|
| 1 | met by probe and unit test | `tests/probes/bus-words.bund` and its golden exist; `conform` passes it. |
| 2 | met | Holds. The golden is cited at a path that does not exist — B5. |
| 3 | met, D88 | The test passes and D88 is RESOLVED. **The open questions say D88 is OPEN** — B4. |
| 4 | deferred | **The behaviour it defers is built** — B1. |
| 5 | met | Dated figures. Today all eight are still implemented and covered. |
| 6 | met | Dated figures. Today 133/145 at its ceiling. |
| 7 | withdrawn | Still no reader of `kind`, and no writer of `Blocking` or `Async` in any crate. |
| 8 | met | Test passes. |
| 9 | met | Doctest present. |
| 10 | met | Test passes in both builds. |
| 11 | met | Test passes. |
| 12 | met | Test passes. |
| 13 | met | Five façade tests pass. |
| 14 | met | Test passes. |

## Blocking

**B1 — Bund2 refuses a deep value at `send`, the oracle sends it, and this
document says the refusal does not exist.** Criterion 4: "Refusing would be
**new behaviour**, so it needs a decision and a deviation entry, not a
criterion." The open questions repeat it. Measured today, a 300-deep list
sent and received on one channel:

| | `send` | `recv` |
|---|---|---|
| oracle | pushes `true` | returns the list; the program prints `got` |
| Bund2 | `SEND returns error Error enveloping data: the value nests 257 deep, and 256 is the` … | not reached |

The bound is F118's, ruled by the owner on 2026-09-12 for the world file,
where the alternative was a save that could not be loaded. `send` reaches the
same encoder, so the bus inherited the refusal the day the words were built.
Nothing in the registers records it for the bus: no entry mentions a
300-deep send or this message.

So the deviation the RFC says would need a decision was taken without one.
It may well be the right behaviour — an unbounded decode aborts the receiving
VM, which is F118's whole argument. It still needs its entry, and criterion 4
needs to say what the binary does. This is the failure CLAUDE.md names: a
required deviation filed as a non-deviation, so no criterion covers it.

**B2 — The preservation table says `recv` errors on an empty channel. It does
not, and the first review's first blocker was this sentence.** The row reads
"**Preserved**, including the immediate error on an empty channel." The body
says an existing but empty channel pushes `NODATA` and only an absent one
errors (`reference/Bund/src/stdlib/functions/bus/mod.rs:122-131`), criterion 1
says the same in bold, and the probe asserts it. The correction reached the
body and not the table.

**B3 — The preservation table gives the reading of D20 that §C2 calls the
error.** The row: "D20 materialises id and stamp, so a received value is equal
and not identical." §C2: "The error came from reading D20's *materialises* as
*replaces*; it means **set if unset**", and the oracle preserves both. What
Bund2 does is D88's ruling — the stamp crosses and a fresh identity is minted
— and the row should say that and cite it. As written it attributes the
deviation to a decision that does not make it.

**B4 — Three decisions the register has resolved are still open in this
document, and one criterion has two statuses.**

- "**D88 is OPEN** with keeping F109 as its default." The register: RESOLVED,
  keep F109. Criterion 3 says "D88 resolves it" forty lines earlier.
- The D89 bullet ends "so it is the owner's". The register: RESOLVED, narrow
  it; criterion 2 reports the narrowing as done.
- §C2 says D86 "names the choice it obliges" and stops, with the choice taken
  by D88 since.
- Criterion 4 is "**Deferred**" in the criteria and in the status line, and
  "**withdrawn** pending a decision" in the open questions.

A reader who takes the open questions as the list of what is unsettled is
told that an OPEN decision's default is in force. It is not; it was ruled.

**B5 — Three citations name a file that does not exist.** `cargo xtask cite`:
`tests/probes/bus-words.golden`, at the document's lines 296, 477 and 521. The
probe is `tests/probes/bus-words.bund`; the golden is
`tests/golden/probes/bus-words.golden`. Accepted RFCs record `cite` at zero
defects as part of acceptance, and these are three of the four it reports.

## Should fix

**S1 — The "current behaviour" section describes a repository that no longer
exists**, in the present tense and with no date:

- "Eight words, all unimplemented in Bund2." All eight are implemented.
- "All **258** stdlib declarations say `Sync`." There are 361.
- "`Slot` is 96 bytes over 396 names." `bund2 words` lists 610.
- "The crate exists as a twelve-line stub." `crates/bund2-async/src` is 1,067
  lines.

Each was true when written. RFC-0006 dates such passages; this one does not.

**S2 — The per-VM cost has moved with the registry and the document quotes
the old one.** D85 measured about 75 KiB a VM at 396 names. At 610 the same
tables are larger by roughly half. D85's conclusion — tens of VMs, megabytes
— survives any such factor. Criterion 14 quotes the old figure as "the one
D85 measured", which is accurate, and a reader will take it as current.

**S3 — Nothing says what a second VM does with a debugger word.** RFC-0008's
amendment (D113) lets a program attach a console with `debug.step`. A VM
spawned by `Host` is given no console, so the word is refused there in words.
That is the right behaviour for a thread with no terminal of its own, and it
is a consequence of two documents that neither states.

## What accepting would mean

- **§C4 is not built and is not authorised.** Zenoh, `--distributed` and the
  globals words stay deferred under D28 and D87.
- **Criteria 9 to 14 are design.** The reference has one VM, so the oracle
  cannot agree or disagree. The status paragraph says this well and it should
  survive acceptance unchanged.
- **Five language questions stay open**: how an address selects a transport,
  whether a program can create a task, what `Async` means, zenoh's scope, and
  the audit's reason for classifying `send` and `recv` as effectful. None is
  a default being adopted; each says so.

## Summary

| | count |
|---|---|
| Blocking | 5 |
| Should fix | 3 |
| Criteria that fail today | 0 |

B1 is a ruling and a register entry, then two sentences. B2 to B5 and S1 are
edits. No code changes unless B1 is ruled against the bound.
