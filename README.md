# Bund2

A JIT-capable reimplementation of the Bund concatenative language: multi-stack,
dynamically typed, metaprogramming, with a tiered execution model.

Bund syntax and logic are preserved 100%.

## Latest release: 0.56.0

The release of statistics over data. Words of Bund2's own for
mathematical biology and genetics (RFC-0015), statistical inference
(RFC-0016), tables of data with factors and missing values (RFC-0017),
and linear and generalised linear models over them, with diagnostics,
prediction and model comparison (RFC-0018). No existing word changes
what it answers. The SMYSL features now take SMYSL 1.15.0 from
crates.io.

conform 143/159, CEILING 143/159; IMPLEMENTED 597/602, COVERAGE 577/602.

The notes are in `docs/release_notes/0.56.0.md`; the examples are in
`bund_examples/`, `biomath`, `frames` and `models` among them.

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
