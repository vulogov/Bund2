# Bund2

A JIT-capable reimplementation of the Bund concatenative language: multi-stack,
dynamically typed, metaprogramming, with a tiered execution model.

Bund syntax and logic are preserved 100%.

## Latest release: 0.57.0

The release of resampling, optimisation, power and further tests
(RFC-0019): noncentral distributions, root finding and quadrature, power
and sample size, rank, count, normality and variance tests, one-way
ANOVA with Tukey's HSD, the bootstrap, the jackknife, permutation tests,
Nelder–Mead and BFGS minimisers, and maximum likelihood with profile
intervals. Every word is Bund2's own and new; no existing word changes
what it answers, except `dist.t`, whose tails are corrected at a large
df.

conform 143/159, CEILING 143/159; IMPLEMENTED 597/602, COVERAGE 577/602.

The notes are in `docs/release_notes/0.57.0.md`; the examples are in
`bund_examples/`, `inference` among them.

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
