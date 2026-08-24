# Bund2

A JIT-capable reimplementation of the Bund concatenative language: multi-stack,
dynamically typed, metaprogramming, with a tiered execution model.

Bund syntax and logic are preserved 100%.

## Status

Pre-implementation. The RFC series is being drafted; see `docs/rfc/` and
`docs/research/05-rfc-roadmap.md`.

## Layout

    crates/         the Bund2 workspace
    docs/research/  the analysis that preceded the RFCs (immutable)
    docs/rfc/       the RFC series
    docs/registers/ decision and defect registers
    reference/      the existing implementation, pinned — analysis only
    tests/golden/   expected state captured from the reference implementation
    xtask/          project tooling

## Execution tiers

    Tier 0   BundIR interpreter          mandatory, every target
    Tier 1   Cranelift JIT               optional, feature = "jit"
    AOT      cranelift-object -> native  optional, feature = "aot"

## Health metric

    cargo xtask conform

Prints passing goldens over total. That number is the project's status. The JIT
and AOT milestones must move it by exactly zero.

## Building

    cargo build --workspace
    cargo build --workspace --features jit

## License

Apache-2.0.
