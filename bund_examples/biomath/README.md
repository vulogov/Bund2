# biomath

The five worked examples of RFC-0015, biomathematics and genetics, and the
rule base of its §S18, one after another:

    make            # run demo.bund
    make check      # run it and compare the output with expected.txt

or, without `make`, from this directory:

    bund2 demo.bund

Every draw comes from a seeded generator, so a run prints the same lines
each time. Numbers are rounded to six decimals before they are printed, so
the lines do not depend on the last bits of a long computation.

## The output

`expected.txt` is what a run prints. A test in `crates/bund2-cli/tests`,
`biomath_demo.rs`, runs the demo and compares the two. It is not a golden:
the reference has none of these words, so there is no oracle to capture
one from.

| | What it shows | Words | It prints |
|---|---|---|---|
| 1 | Hardy–Weinberg at one SNP, and p-values corrected for the false discovery rate | `popgen.hwe`, `stat.padjust.bh` | exact p 0.655663, χ² p 0.637907; adjusted 0.005, 0.05, 0.066667, 0.375, 0.5 |
| 2 | A primer's reverse complement, and its protein under NCBI's table 1 | `bio.revcomp`, `bio.translate` | `TCAGCGGCCCATTACAATGGCCAT`, `MAIVMGR*` |
| 3 | Eight Wright–Fisher replicates, each in its own VM on its own thread, seeded by its start-up value; seed 3 run again gives its path again | `rng.seed`, `popgen.wf`, `vm.spawn`, `vm.join` | the eight final frequencies; 0.38 twice for seed 3 |
| 4 | An SIR epidemic, β = 0.5 and γ = 0.1, by Dormand–Prince to t = 100, with ln(S/S₀) = −(β/γ)·R as the check | `ode.rk45` | S, I and R at t = 100; −4.964376 beside −4.964377 |
| 5 | A Leslie matrix's growth rate and stable stage distribution | `linalg.dominant` | λ 1.013103; 0.545096, 0.269023, 0.18588 |
| 6 | Rules that call the family's words through bridges: a p-value threshold, a genotype test, and an exact HWE test per locus | `kb!`, `kb.query`, `test`, `is` | `rs1` and `rs3`; `ann`; 0.655663 and 0.84228 |

## Four idioms

- **Operands go deepest first.** The deepest operand is a formula's first,
  so `298 489 213 popgen.hwe` is AA, Aa, aa, and `a b -` is b − a.
- **A child VM cannot call the parent's words**, so example 3's program is
  a LAMBDA, given to `vm.spawn` with `[ seed ]` as the child's stack.
- **A model is a LAMBDA run on its own stack.** Example 4's `:sir` sees only
  `t` and `[ S I R ]`, and leaves `[ dS dI dR ]`; the caller's stack is
  never visible to it.
- **A genotype in a fact is a quoted STRING.** A bare `Aa` begins with a
  capital, so the engine would read it as a variable.
