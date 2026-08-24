# Probes

Authored `.bund` programs that exercise behaviour the reference examples never
reach. Established by D21.

The reference corpus is a demo set, not a specification. 77 goldens reach 140
of 586 in-scope words, and even a word the goldens do reach may be only partly
exercised — `!` dispatches on eight arms and the corpus reaches four. Probes
close that gap.

## The rule

**Expected output is never hand-written.** A probe says what to run; the
oracle says what it does. Capture goldens with `cargo xtask golden`, exactly as
for the corpus.

Writing a Rust test that asserts what you believe the reference does would
encode your reading rather than its behaviour. `execute`'s DICT arm is the
standing reminder: it pulls the dictionary first and *then* pulls a key from
underneath it (`reference/rust_multistackvm/src/stdlib/execute.rs:55-61`), so
the operand order a program must use is not evident from reading the code.

## Provenance — why these are not in `tests/golden/`

`tests/golden/` holds the reference's own examples. It is the preservation
contract, and CLAUDE.md's "a failure has exactly three dispositions" rule is
calibrated for programs the reference authors wrote.

A probe is ours. If a probe encodes a reference bug, that is a fourth
situation, and the probe may be the thing that changes. Keeping the two sets
apart keeps the corpus goldens sacred without making that rule vague.

Probe goldens live in `tests/golden/probes/` and are equally captured from the
oracle — the distinction is authorship, not authority.

## Naming and targeting

One probe per **behaviour**, not per word. Name it for the behaviour it pins:

    execute-arm-string.bund          `!` on a bare STRING
    execute-arm-list.bund            `!` on a LIST
    execute-arm-dict-key.bund        `!` on a MAP/DICT with a key
    execute-arm-class.bund           `!` on a CLASS
    ptr-literal-backtick.bund        the `\`name` grammar term

Each probe must:

- be **hermetic** — no network, filesystem, image, database, bus, clock,
  randomness, or host state, by the same rules as the corpus
  (`xtask/src/corpus/classify.rs`);
- be **in scope** — invoke no word deferred by a decision;
- **print something**, so the golden has observable content beyond final stack
  state;
- carry a header comment naming the behaviour it pins and the `path:line` in
  `reference/` that implements it.

## Status

The probes here are authored but their goldens are **not yet captured**:
`cargo xtask golden` is not implemented. Until it is, these are pending, and
`cargo xtask coverage` reports them as such rather than counting them.
