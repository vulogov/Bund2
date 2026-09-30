# Review — RFC-0006's implementation of `--emit=bundle`

- Reviewed: `crates/bund2-cli/src/bundle.rs`, `bund2 build` and `run_carried`
  in `crates/bund2-cli/src/main.rs`, `crates/bund2-cli/tests/bundle_build.rs`,
  and `conform --bundles`, as of `a427e18`.
- Date: 2026-09-30. **A self-review**, which is weaker than the three
  adversarial reviews of the prose and is recorded as such.
- Working tree clean at the start; every `reference/` submodule clean.

## Verdict

**Two blockers, both release-only, both found by running the shipping profile
rather than by reading.** Eleven tests passed over an implementation that could
not work in release at all.

The root cause is one omission, not two mistakes: **`cargo test` compiles in
debug, and nothing exercised release.** Both blockers are properties of the
optimiser.

## B1 — the sentinel's uniqueness was a debug-only property

`locate` scanned for a 32-byte sentinel and refused if it matched twice. The
argument for uniqueness was that the constant was stored masked and unmasked by
a `const fn`, keeping the plain bytes out of the text section.

**Release const-folds the `const fn`.** The plain 32 bytes appear in the code as
well as in the region, so every release build refused with "the sentinel occurs
more than once in this binary". Measured: two matches at `10377584` and
`10377984` in `target/release/bund2`.

The unit test `the_sentinel_occurs_exactly_once` asserted the false property
and passed throughout, because it ran in debug.

**Fixed by identifying the region structurally.** A candidate is now a sentinel
match whose header then validates — a known `format`, a `state` that is `EMPTY`
or `FILLED`, a length within capacity, and the whole region inside the file. A
bare constant satisfies none of that, because what follows it is instructions.
Measured on the same binary: two sentinel matches, **one** validating header.
Refusing on two *validating* candidates is kept, because that cannot arise from
one `static` and would mean something about the image the code does not
understand.

## B2 — an immutable `static` cannot observe a patched file

The runtime read its payload from `static REGION`, on the reasoning that a
bundle should not have to find its own executable. A compiler is entitled to
fold every read of an immutable `static` to its initialiser, and release did.

**The artefact was correct and behaved as the plain interpreter.** The file said
`state = 1, len = 39` at the region the builder had patched — verified by
reading the bytes — and the running program took the `EMPTY` arm and printed
`bund2: expected: bund2 script --file <path>`. That is the worst shape of
failure available here: a build that reports success and an artefact that
silently is not one.

**Fixed with `std::hint::black_box`**, which exists for this. It is a **hint,
not a guarantee**, and that is recorded at the call site along with the
fallback if a future compiler folds through it: RFC-0006 §B1's separate
prebuilt runtime, which reads its payload from its own file and cannot be
folded at all.

## What the fix for the class of bug is

`cargo xtask bundle` builds `bund2` in **both profiles**, builds an artefact
with each, runs it, and checks it prints its answer and sees its argument;
on macOS it also checks the release artefact validates.

**Verified load-bearing**: with `black_box` removed, it reports

    debug    runs, prints its answer, and sees its argument
    release  FAILED: the release artefact exited Some(2) and printed:
             bund2: unknown argument `one`

which is the silent failure named, in the profile that ships, by a check that
takes one command.

## Concerns raised and cleared by measurement

- **A short program leaving a previous long one readable in the artefact.** It
  does not: each build starts from a clean `current_exe()` image and
  `write_into` fills the remainder of the capacity. A 200-statement program
  with a marker, then a one-line program: the marker occurs once in the first
  artefact and **zero** times in the second.
- **`--features jit` in release**, the fourth configuration and the one no test
  covered. A release JIT bundle reports `compiled 1 bodies (38 entered)` at
  threshold 2 — the same figure as debug, so the region survives LTO with
  Cranelift linked in.
- **The test's hand-computed offsets** (`STATE_BEFORE_PAYLOAD`) drifting from
  the container's layout. `payload_at` asserts the `state` byte reads `FILLED`
  at the offset it assumes, so a layout change fails the test rather than
  patching something else. `LEN_BEFORE_PAYLOAD` has no such guard; it is
  exercised by the same test, so a drift would still surface, but less
  directly.

## Second pass, same day — one more defect, and three properties pinned

The three concerns the first pass named as unexamined were then attacked.

### B3 — `--output` could be the binary doing the building

**`bund2 build --output $(which bund2)` succeeded.** It reported "wrote …",
replaced the interpreter with the artefact, and afterwards
`bund2 --file x.bund` ran the *embedded* program and ignored the argument,
because a bundle gives all of argv to its program. Destructive, silent, and
one keystroke from `-o` on the wrong path.

**Fixed by refusing**, compared by identity rather than spelling — canonical
paths, so a symlink or a relative path cannot slip past — and an output that
does not exist yet cannot be the running binary, so absence is not an error.
Pinned by `a_build_refuses_to_overwrite_the_binary_doing_it`, which also
asserts the binary still interprets a file afterwards.

### Pinned, having been true only by construction

- **Two candidates that both validate are refused, not guessed between.** The
  release fix rests on a bare constant never carrying a valid header, so the
  test plants a *whole* region — header and full-size body — and asserts a
  refusal. Patching the wrong candidate would produce an artefact that runs
  the wrong program, which nothing downstream would catch.
- **A bundle cannot build another bundle**, because `carried()` is consulted
  before the `build` arm. True by argv ordering and now pinned, since it is
  the kind of property that changes quietly when arms are reordered.
- **An empty program does not become the interpreter.** `state` filled with
  length zero must run nothing and must *not* fall back to the CLI, which
  would parse the program's own arguments as flags. Checked with argv of
  `words --stats`: no word table, no stats, no output.
- **An exit code travels.** `7 bund.exit` gives 7 from a bundle and 7 from a
  source run, asserted against each other rather than against a constant.

### Cleared by inspection

**D78's floor is structurally protected.** `run_carried` builds `HostOptions`
as an exhaustive literal with no `..Default::default()`, so a field added to
that struct without a decision about what a bundle does with it is a compile
error rather than a silent default. A comment at the site says so, because the
protection is invisible otherwise and one `..` would remove it.

## Not examined

- Anything not on macOS arm64. ELF and PE are reasoned about in §B1 and
  measured nowhere.
- A Developer ID signature and notarisation. No identity in this repository.
- `bund2 build` run *from* a bundle. It cannot happen — `carried()` is
  consulted before the `build` arm, so a bundle's argv goes to its program —
  but no test pins that, and it is the kind of thing that would change
  quietly.
