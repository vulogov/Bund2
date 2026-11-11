# Bund2

A JIT-capable reimplementation of the Bund concatenative language: multi-stack,
dynamically typed, metaprogramming, with a tiered execution model.

Bund syntax and logic are preserved 100%.

## Latest release: 0.54.0

The release that connects Bund to SMYSL (RFC-0014). A Bund program can form
a SMYSL library, research it, query it and reason over it with the logic
engine, and, in a build with a model provider, have a model extract units
from a document or a text and answer a question from what a library holds.
The words are off by default: `--features smysl`, or `smysl-ollama` and the
other mapper features for the model steps.

conform 143/159, CEILING 143/159; IMPLEMENTED 597/602, COVERAGE 577/602.

The notes are in `docs/release_notes/0.54.0.md`; the examples are in
`bund_examples/`.

## Layout

    crates/         the Bund2 workspace
    bund_examples/  example programs, each with a README
    docs/research/  the analysis that preceded the RFCs (immutable)
    docs/rfc/       the RFC series
    docs/registers/ decision and defect registers
    docs/release_notes/  one file per release
    reference/      the existing implementation, pinned — analysis only
    tests/golden/   expected state captured from the reference implementation
    xtask/          project tooling

## Execution tiers

    Tier 0   BundIR interpreter          mandatory, every target
    Tier 1   Cranelift JIT               optional, feature = "jit"
    AOT      withdrawn (D83): there is no native build

## Health metric

    cargo xtask conform
    cargo xtask coverage

`conform` prints passing goldens over total, with the CEILING that approved
deviations leave reachable; it is the regression number, and the JIT must
move it by exactly zero. `coverage` prints in-scope words that Bund2
implements and a golden runs (COVERAGE), beside those it implements
(IMPLEMENTED). Neither substitutes for the other.

## Building

    cargo build --workspace
    cargo build --workspace --features jit
    cargo build --release -p bund2-cli --features smysl

## License

Apache-2.0.
