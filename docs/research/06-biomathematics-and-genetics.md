# 06 — Biomathematics and genetics: what the standard library would need

*Research, 2026-11-14, on the owner's request: "extension of the standard
library mathematical functions that will make Bund suitable for
biomathematics and genetics computational tasks". Nothing here is decided.
Every choice below is a question for an RFC and its D-entry, not a default
to be adopted (CLAUDE.md, "Registers are append-only").*

## 1. The question

What would a Bund program need from the standard library to do the
everyday computations of mathematical biology and genetics? That means
population models, epidemics, enzyme kinetics, probability and testing for
genetic data, sequence manipulation and alignment, population genetics and
phylogenetics. How much of it does Bund2 already have, and in what order
could the rest be built without breaking the rules Bund2 already keeps?

Two such rules frame everything below:

- **D72.** Bund2 may carry words the reference does not. Every word here
  would be Bund2's own, as RFC-0012's graph algorithms, RFC-0013's logic
  programming (D226) and RFC-0007's `vm.` words (D249) are.
- **No golden can hold a Bund2-only word.** Goldens are captured from the
  oracle (D21), and the oracle has none of these words. COVERAGE joins
  against the reference's registry, so it would not move either. Unit tests
  against published values are the only pin. The health numbers stay
  conform 143/159, CEILING 143/159; IMPLEMENTED 597/602, COVERAGE 577/602,
  whatever this family adds.

## 2. What Bund2 has today

Taken from `bund2 words` at `development/0.56.0` and from the crates
`bund2-stdlib` links.

| Area | Words now | Fit for biology |
|---|---|---|
| Scalar maths | `math.abs`, `sqrt`, `cbrt`, `nroot`, `power`, `exp`, `ln`, `log10`, trig and hyperbolic, `floor`, `ceil`, `round`, `fract`, `signum`, `factorial` | the basics; no `log2`, no gamma or beta, no log-space arithmetic |
| Series | `math.max`, `math.min`, `math.normalize`, `math.smoothing`, `math.interpolation`; `stat.count`; `stat.mean.*` (arithmetic, geometric, harmonic, weighted forms) | means only: no variance, median, quantiles or correlation |
| Distributions | `generator` / `generator.sample` over Normal, Uniform and LogNormal (`Gen` in `crates/bund2-stdlib/src/random.rs`) | sampling only, three distributions, no pdf, cdf or quantile, **unseeded** |
| Random | `math.random.int`, `math.securerandom.int`, `string.random.*` | unseeded; no reproducible simulation |
| Matrices | `matrix` / `convert.to_matrix`; the MATRIX value is `Vec<Vec<BundValue>>` | a container with no arithmetic |
| Time series | `forecast.*`, `outlier.detect*`, `sample.analysis` (D215) | growth-curve forecasting; not mechanistic models |
| Strings | `string.distance.hamming`, `.levenshtein`, `.dameraulevenshtein`, `.jarowinkler`; `string.regex*` | Hamming distance between equal-length sequences works today; there is no scored alignment |
| Graphs | `graph.*`, 34 words: paths, SCC, MST, max flow, toposort (RFC-0012) | pathway and interaction networks, tree traversal |
| Logic | `kb.*`, 25 words, with CLP(FD) (RFC-0013) | Mendelian crosses, pedigree rules, constraints on genotypes |
| Concurrency | `vm.spawn` / `vm.join` (D249) | independent simulation replicates on threads |

Every series word reads its operand the same way: a LIST whole, a
METRICS' samples, or the stack drained to a NODATA (`get_data`, described
in `crates/bund2-stdlib/src/series.rs`'s module doc). New words should
read their input the same way, so that `1.0 2.0 3.0 stat.var` and
`[ 1.0 2.0 3.0 ] stat.var` both work.

A native word can call a Bund lambda and get its effect back:
`Vm::eval_lambda` in `crates/bund2-api/src/lib.rs`, which RFC-0013's
bridges use (`eval_lambda` in `crates/bund2-stdlib/src/kb/bridge.rs`).
That is what an ODE solver, an optimiser or a simulation needs in order to
take its model as a lambda.

### What is already linked

The default build already compiles most of the machinery this family would
need, pulled in by dependencies D215 and D38 brought in
(`cargo tree -i`, 2026-11-14):

| Crate | In through | Offers |
|---|---|---|
| `statrs` 0.18.0 | `bund2-stdlib` directly | 33 distributions (Bernoulli, Beta, Binomial, Categorical, Cauchy, Chi, ChiSquared, Dirichlet, Exponential, FisherSnedecor, Gamma, Geometric, Hypergeometric, LogNormal, Multinomial, MultivariateNormal, NegativeBinomial, Normal, Poisson, StudentsT, Weibull and others), each with pdf/pmf, cdf, inverse cdf and moments; `gamma`, `ln_gamma`, `digamma`, the incomplete gammas, `beta`, `ln_beta`, `beta_reg` and its inverse, `erf`, `erfc`, their inverses, `binomial`, `ln_binomial`, `multinomial`; order statistics |
| `nalgebra` 0.33.3 | `augurs-ets` | dense matrices, LU, QR, Cholesky, SVD, symmetric and general eigen-decomposition |
| `argmin` 0.10.0 | `augurs-forecaster` | Nelder–Mead, BFGS, L-BFGS, Gauss–Newton, line searches |
| `rand_distr` 0.4.3, `rand_chacha` 0.9.0 | `rand` / `bund2-stdlib` | seedable generators; sampling |
| `ndarray` 0.16.1 | `algos` | n-dimensional arrays |

**So Phases 1 and 4 below would add no new crate to the default build, and
D28's concern, binary size, barely arises.** Only sequence alignment and
phylogenetics would need code of their own or a new crate. `rust-bio` is the
obvious candidate, and its dependency set is large; question B9 is about
it.

## 3. The gaps, by domain

Each row is what a working biologist would expect to find, with the crate
that already covers it where one does.

### 3.1 Numerical foundation (every other domain rests on it)

| Need | Why biology needs it | Source |
|---|---|---|
| `math.log2`, `math.log1p`, `math.expm1`, `math.logsumexp` | likelihoods are multiplied in log space; `log2` for information and fold change | std / own |
| `math.gamma`, `math.lgamma`, `math.beta`, `math.lbeta`, `math.erf`, `math.erfc`, `math.digamma` | the closed forms of every distribution, and Beta-binomial models | `statrs::function` |
| `math.choose`, `math.lchoose`, `math.multinomial` | binomial sampling, segregation ratios | `statrs::function::factorial` |
| Element-wise vector arithmetic, `dot`, `cumsum`, `cumprod`, `diff` | allele frequency vectors, survival, discrete growth | own |
| Matrix arithmetic: multiply, transpose, inverse, solve, determinant, eigenvalues and eigenvectors | Leslie and Lefkovitch projection (the dominant eigenvalue is the growth rate), Markov chains of genotypes, linear regression | `nalgebra` |

### 3.2 Probability

The pdf/pmf, cdf, quantile, sample and moments of Normal, Binomial,
Poisson, NegativeBinomial, Hypergeometric, Beta, Gamma, Exponential,
ChiSquared, StudentsT, FisherSnedecor, Multinomial and Dirichlet. All of it
is in `statrs`. The design question is the vocabulary's shape (B2): one
word per distribution and function (about 15 × 6 = 90 words), or a
distribution *value* plus about six generic words.

### 3.3 Statistics and tests

- **Descriptive:** variance, standard deviation, median, quantiles, mode,
  min/max with their index, standard error.
- **Association:** Pearson, Spearman and Kendall correlation; covariance.
- **Tests:** one- and two-sample t, Welch's t, χ² goodness of fit and
  contingency, Fisher's exact test (it is Hypergeometric), Mann–Whitney U,
  Kolmogorov–Smirnov, the binomial test.
- **Multiple testing:** Bonferroni, Holm, Benjamini–Hochberg FDR. These are
  essential for any genome-scale question.
- **Models:** ordinary least squares (`nalgebra`'s QR), logistic regression
  by IRLS, and non-linear least squares for Michaelis–Menten, Hill and
  logistic growth fits (`argmin`'s Gauss–Newton or Nelder–Mead).

### 3.4 Reproducible randomness

Simulation in biology is only useful when it can be repeated. Today every
random word is unseeded, by the reference's design and D214/D215's reading
of it. A family of its own could take an explicit seeded generator value
(ChaCha, through `rand_chacha`, already linked) without changing any
existing word. B4 asks how.

### 3.5 Dynamical systems

- **ODEs:** fixed-step RK4 and adaptive Dormand–Prince (RK45), driven by a
  Bund lambda `( t state -- dstate )`. That covers logistic growth,
  Lotka–Volterra, SIR/SEIR, Michaelis–Menten and gene-regulatory models.
  Neither `nalgebra` nor `argmin` has an integrator; RK4 and RK45 are about
  150 lines of Rust each, and a crate (`ode_solvers`) is the alternative
  (B8).
- **Discrete maps:** iterating `N(t+1) = f(N(t))` is already a Bund loop, so
  it needs no word.
- **Stochastic simulation:** Gillespie's direct method, given reactions as
  propensity lambdas and stoichiometry vectors.
- **Cost:** each RHS evaluation is a call into Bund through
  `Vm::eval_lambda`. Tier 0 pays interpretation per step and Tier 1 (the
  JIT) is where this would be fast. B6 asks whether a model given as data
  (a MAP of rates and stoichiometries) should be offered too.

### 3.6 Sequences

| Need | Notes |
|---|---|
| Alphabets and validation: DNA, RNA, protein, IUPAC ambiguity codes | own; a STRING is a sequence, so no new type is required (B3) |
| Complement, reverse complement, transcription, GC content, molecular weight | own; a few lines each |
| Translation under the standard genetic code and NCBI's alternatives; ORF finding | own; the codon tables are data |
| k-mer counting, a minimal k-mer, a sequence's Shannon entropy | own |
| Pairwise alignment: Needleman–Wunsch (global), Smith–Waterman (local), semi-global, affine gaps; BLOSUM62 and PAM250 scoring | own, or `rust-bio` (B9) |
| Motifs: consensus, position weight matrix, scanning a sequence with a PWM | own |
| FASTA and FASTQ reading, Phred qualities | I/O, so it is refused under `--noio`, as `fs.*` is |

`string.distance.hamming` already gives the p-distance's numerator for
aligned sequences.

### 3.7 Population genetics

- **Frequencies:** allele and genotype frequencies from counts; observed and
  expected heterozygosity.
- **Hardy–Weinberg:** expected genotype counts, the χ² test, and the exact
  test (Wigginton et al. 2005).
- **Structure:** F_ST (Weir & Cockerham, Hudson), F_IS, F_IT.
- **Linkage disequilibrium:** D, D′, r².
- **Diversity and neutrality:** nucleotide diversity π, Watterson's θ,
  Tajima's D, segregating sites.
- **Models:** the Wright–Fisher step (binomial resampling, needing seeded
  randomness), selection recurrence, mutation–selection balance, effective
  population size, and coalescent times under the Kingman coalescent.

Almost all of these are short formulas over the probability words above,
which is why the order of work in §4 puts probability first.

### 3.8 Phylogenetics

- **Distances:** p-distance, Jukes–Cantor, Kimura two-parameter; a
  distance MATRIX between aligned sequences.
- **Tree building:** UPGMA and neighbour joining.
- **Trees:** reading and writing Newick. A tree is a GRAPH (RFC-0012), so
  `graph.*` traverses it; B7 asks whether it is a GRAPH or a nested LIST.
- **Out of scope for a first pass:** maximum likelihood and Bayesian
  inference. They are what dedicated tools are for.

### 3.9 Genetics by rules

Punnett squares, pedigree inference and epistasis are rules over facts,
which is RFC-0013's `kb.*`. Their probabilities can be computed with
CLP(FD) and the probability words. These want worked examples in
`bund_examples/`, not words.

## 4. A possible order of work

Each phase is usable on its own, and each ends with the health numbers
unchanged (D72), clippy clean and unit tests against published values.

1. **Foundation:** §3.1, §3.2, §3.3 and §3.4. About 60 words, nothing new
   linked, and the biggest gain: after it, Bund can do the statistics of a
   genetics paper.
2. **Sequences:** §3.6. Strings and tables, with alignment the one sizeable
   piece of work.
3. **Population genetics and phylogenetics:** §3.7 and §3.8, over Phases 1
   and 2.
4. **Dynamics and fitting:** §3.5 and §3.3's non-linear fits; most
   sensitive to Tier 0's per-call cost.

`bund_examples/` would gain one demo per phase: a Hardy–Weinberg and
association-test session, a gene's translation and alignment, a
Wright–Fisher drift run on `vm.spawn` replicates, and an SIR epidemic.

## 5. Questions an RFC must put to the owner

None of these has a default to adopt. Each would be a lettered point in
the RFC's D-entry.

- **B1. Default build, or a feature?** D28 keeps the default build to
  essentials; D215 and L9 (D226) put families in it. Phases 1 and 4 add no
  crate; Phases 2 and 3 might (B9).
- **B2. The shape of the distributions:** a word per distribution and
  function (`dist.normal.cdf`), or a distribution value (a MAP or a CLASS
  instance) and generic words (`dist.cdf`, `dist.pdf`, `dist.quantile`,
  `dist.sample`, `dist.mean`, `dist.var`).
- **B3. Numeric vectors:** a LIST of FLOATs, which boxes every element
  (`BundValue`), or a packed FLOAT vector type beside MATRIX. A million
  allele frequencies is where the difference starts to matter, and a new
  type touches the value crate, the wire format and the JIT.
- **B4. Seeded randomness:** a generator value passed to each word, a
  per-VM seed word, or both. Existing words stay unseeded either way.
- **B5. Names:** reuse `math.*` and `stat.*`, which the reference uses for
  its own words (the reference registers only `stat.count` and the
  `stat.mean.*` family under `stat.`, so a new `stat.var` collides with
  nothing), or new prefixes: `dist.`, `linalg.`, `ode.`, `dna.`/`bio.`,
  `popgen.`, `phylo.`. `seq` is taken: it is the reference's numeric
  sequence word.
- **B6. Models as lambdas only, or as data too** (§3.5). Data is faster on
  Tier 0 and can be checked; a lambda is general.
- **B7. A phylogenetic tree:** a GRAPH or a nested LIST.
- **B8. ODE integrators:** Bund2's own, or a pinned crate under D38's rule.
- **B9. Alignment:** Bund2's own, or `rust-bio` behind a feature of its own.
  Its size needs measuring first, as D28 measured the reference's.
- **B10. The `.` and `,` variants** that the reference's series words carry
  (D18): give the new words them too, or not, as `kb.*` mostly does not.
- **B11. Numerical tolerance in tests:** what a unit test compares against,
  for example R's output at a stated version, and to how many ULPs or what
  relative error.

## 6. What would make the work tractable

- **Precedent:** RFC-0012 (`graph.*`) and RFC-0013 (`kb.*`) built a
  Bund2-only family by phases, each gated by criteria. This would be RFC-0015
  in the same form.
- **Errors, not panics.** Domain errors such as `ln` of a negative,
  probabilities outside [0, 1], unequal sequence lengths for Hamming or a
  non-square matrix for `inverse` become `Diagnostic`s through `Vm::report`
  (D36, D37). A singular matrix is an error, not NaN. Whether NaN may flow
  through series words is B12.
- **The vocabulary audit.** Each new word appears in `bund2 words`,
  PROMOTABLE.txt's audit and the palette, as RFC-0013's did.
