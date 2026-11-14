# Bund2

A JIT-capable reimplementation of the Bund concatenative language: multi-stack,
dynamically typed, metaprogramming, with a tiered execution model.

Bund syntax and logic are preserved 100%.

## Latest release: 0.55.0

The release in which a Bund program starts VMs itself (D249). Three words,
`vm.spawn`, `vm.join` and `vm.done?`, run a LAMBDA or a STRING on a VM of
its own thread and hand back its final stack; the VMs talk over the local
bus, as VMs started by a Rust host do. The words are in every build and
additive: a program's own word of the same name still wins, and
`bund2_async::host::Host` keeps its API.

conform 143/159, CEILING 143/159; IMPLEMENTED 597/602, COVERAGE 577/602.

The notes are in `docs/release_notes/0.55.0.md`; the examples are in
`bund_examples/`, `threads_demo` and `threads_demo_inner` among them.

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
