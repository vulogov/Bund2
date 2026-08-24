# Decision register

Append-only. Add entries, change `status`, never delete or renumber.

Status: OPEN | RESOLVED | SUPERSEDED. A default is for planning only — an RFC
or work item may never adopt one silently.

---

## D1 — `.id` format contract
Does `.id`'s exact nanoid format matter to existing programs, or is the
contract "unique opaque string"?
- Blocks: RFC-0001
- Default: lazy nanoid derived from counter plus VM seed (preserves format)
- Evidence: `cargo xtask corpus`
- Status: OPEN

## D2 — `.timestamp` precision
Is millisecond granularity the contract, or must two values constructed in
sequence differ?
- Blocks: RFC-0001
- Default: sampled clock at millisecond granularity
- Evidence: `cargo xtask corpus`
- Status: OPEN

## D3 — tier policy for `bund.eval` output
JIT-eligible, or permanently Tier 0? Permanently Tier 0 removes a whole class
of unbounded code-memory growth.
- Blocks: RFC-0003, RFC-0005
- Default: permanently Tier 0
- Status: OPEN

## D4 — integer width
Is full `i64` required, or are 51-bit integers acceptable? The latter allows
NaN-boxing and a smaller value.
- Blocks: RFC-0001
- Default: full `i64`
- Status: OPEN

## D5 — lambda body mutability
Can a LAMBDA body be mutated after construction, or is it write-once? Write-once
lets the compiled cache skip invalidation.
- Blocks: RFC-0003
- Default: assume mutable; invalidate on write
- Evidence: `cargo xtask corpus`
- Status: OPEN

## D6 — async granularity
Is fine-grained suspension inside a word required, or is VM-per-task enough?
- Blocks: RFC-0007
- Default: VM-per-task
- Status: OPEN

## D7 — concurrent VM count
Tens (actor model is fine) or thousands (per-VM word tables become the memory
story)?
- Blocks: RFC-0007
- Default: tens
- Status: OPEN

## D8 — existing external word packages
Do any Rust word packages exist outside this repository? If so, `bund2-api` is
a migration rather than a clean design.
- Blocks: RFC-0002
- Default: none; design freely
- Status: OPEN

## D9 — third-party CLIF lowerings
Should `Intrinsic` lowerings ever be exposed through `bund2-api`? Doing so pins
external packages to an exact Cranelift version.
- Blocks: RFC-0002
- Default: no
- Status: OPEN

## D10 — C toolchain requirement
May `bund2 build` require `cc`, or must the compiler be self-contained?
- Blocks: RFC-0006
- Default: yes, `cc`; `--emit=bundle` covers toolchain-free targets
- Status: OPEN

## D11 — external dependents of `compile_to_binary`
Does anything outside the project depend on the current bincode object format?
- Blocks: RFC-0003
- Default: no; version the IR format freshly
- Status: OPEN

## D12 — the `*` fold-family
Restrict the whole-stack variadic words in JIT-able positions, or accept them
as a permanent optimization barrier?
- Blocks: RFC-0004
- Default: accept as barrier
- Evidence: `cargo xtask corpus`
- Status: OPEN

## D13 — value semantics under Rc
Today `Value` is deep-cloned everywhere, so Bund has value semantics and cycles
are impossible by construction. Naive `Rc` would give reference semantics and
make cycles constructible. Confirm `Rc::make_mut` (clone-on-write) as the
correct preservation.
- Blocks: RFC-0001
- Default: yes, `Rc::make_mut`
- Status: OPEN

## D14 — library scope
Which of the 357 words are language core (100% preservation) and which are
library (deferrable, re-implementable as out-of-tree word packages)?
Preservation applies to Bund syntax and logic, not to the domain libraries.
- Blocks: RFC-0002 (bund2-api shape), RFC-0004, the M6 target and denominator
- Default: none — decide from corpus evidence
- Evidence: cargo xtask corpus
- Status: OPEN
