# RFC-0019: Resampling, optimisation and power — the bootstrap, permutation tests, minimisers, maximum likelihood, noncentral distributions, power and further tests

- Status: Draft. Its owner questions N1–N13 are D266, **RESOLVED
  2026-11-21** as recommended ("N1–N13: recomendations"); its five
  **[author]** choices are D267, **RESOLVED 2026-11-21** as written
  ("rule on the author choices: recommendations").
- Revised 2026-11-21 after its first review
  (`docs/rfc/reviews/RFC-0019-review-2026-11-21.md`), answering B1, B2
  and SF1–SF7 as the review recommended, on the owner's "answering B1,
  B2 and SF1–SF7: reccomendations" (D268). The review's notes N1–N8 and
  assumptions U1–U6 are not taken in this revision.
- Drafted 2026-11-21 on the owner's "RFC-0019 for one of the statistics
  roadmap's next phases: D and E", in 0.57.0, after
  `docs/research/07-statistical-research.md` (§4, Phases D and E). D260's
  C8 ruled one RFC per phase; the owner changed that for D and E the
  same day (D260's dated note of 2026-11-21), so this RFC covers both.
- Depends on: RFC-0015 (the `biomath` family: operand readers, §S3's
  forms, §S7's distributions, §S8's generator, §S11's refusals, §S15's
  lambda bridge), RFC-0016 (§S1's options-MAP rules, §S7's sampling
  words), RFC-0017 (the frame and its check), RFC-0004 (effects)
- Decisions consumed: D260 (C4 D, C5 C, C6 C, C7 B, and its note of
  2026-11-21 on C8), D250 (B4, B5, B8, B10, B11, B12, B15), D251 (A7,
  A8), D21, D36, D37, D38, D39, D72, D252, **D266** (N1–N13, as
  recommended), **D267** (the [author] choices, as written) and
  **D268** (the first review's fixes).
- Reference SHA: `reference/rust_multistackvm`
  4605832678d4792c2b8e0564b7757d5aee1539a4; `reference/Bund`
  21b40b0213a76f3f39bd6de0d2bdfdf0a53b0dca; `reference/rust_dynamic`
  ceb27c96fa10c4edb09106f3f904f22f94d63119. Bund2's own code is cited by
  file and symbol, at `development/0.57.0` d51119a. R is cited as
  reasoning, never as a reference this RFC must match: R 4.4.2's source,
  read through the `wch/r-source` mirror at the tag `R-4-4-2`
  (`https://raw.githubusercontent.com/wch/r-source/tags/R-4-4-2/`), as
  RFC-0018 read it, and the `boot` package 1.3-32 through the `cran/boot`
  mirror at the tag `1.3-32`. Paths under `src/` below are R's; `boot`'s
  file is `R/bootfuns.q`.
- Supersedes: nothing. Grounds and refines research 07's Phases D and E
  (§3.4–§3.6, §4).

## Summary

RFC-0015 to RFC-0018 gave Bund2 the classical tests, intervals, frames
and models. A study then needs what research 07 calls Phases D and E:
to resample when no formula gives an interval, to fit a model of its own
by maximum likelihood, to plan a sample size, and to run the tests that
are still missing. This RFC adds, as words of Bund2's own:

- **Phase E.** The noncentral t, χ² and F distributions and the
  studentized range; power and sample size for t tests, proportions,
  one-way ANOVA and χ²; the Wilcoxon signed-rank, Kruskal–Wallis,
  Friedman, Shapiro–Wilk, one-sample Kolmogorov–Smirnov, Bartlett,
  Levene, McNemar and Cochran–Mantel–Haenszel tests; one-way ANOVA with
  Tukey's HSD; and the interval for Cohen's d that RFC-0016 deferred.
- **Phase D.** A derived seed for reproducible studies across VMs; the
  bootstrap, with percentile, basic, normal and BCa intervals, and the
  jackknife; the two-sample permutation test; Brent's root finder and
  adaptive Gauss–Kronrod quadrature; Nelder–Mead and BFGS minimisers
  and a numerical Hessian; and `fit.mle`, maximum likelihood over a
  log-likelihood lambda with standard errors and profile intervals.

**No existing word changes** in operands, effect, answer, refusals or
cost. No data representation changes. Some 40 names are added (§S1),
under the prefixes N1 rules.

## Motivation

- **No interval without a formula.** RFC-0016's intervals are each a
  closed form or an inversion of one test (RFC-0016 §S2). A median, a
  ratio of means or any statistic a program computes has none. The
  drawing words exist (`rng.choice.replace`, `crates/bund2-stdlib/src/biomath/sample.rs`,
  the module doc), so a bootstrap can be written by hand. But every program
  rewrites the loop and the interval arithmetic, and BCa is easy to get
  wrong.
- **No test for an arbitrary statistic.** A permutation test is the
  general answer when no test fits the data. It needs exact enumeration
  below a cap and Monte Carlo above it, and both are bounded loops that
  D39 wants in native code.
- **No minimiser for a program's own function.** `fit.nls` minimises a
  residual sum of squares by Levenberg–Marquardt (`crates/bund2-stdlib/src/biomath/fit.rs`,
  the module doc), and nothing minimises any other objective. Most
  bespoke research models are fitted by maximum likelihood (research 07
  §3.6), which needs a minimiser, a Hessian for standard errors, and a
  root finder for profile intervals. Q61 asked whether the `argmin` the
  lock compiles could supply one. It cannot (§Current behaviour).
- **Power cannot be computed.** Power for a t, F or χ² test is a tail of
  the noncentral distribution. `dist.rs`'s families are central only
  (`crates/bund2-stdlib/src/biomath/dist.rs`, `FAMILIES`), and `statrs`
  has no noncentral family (research 07 §2). For the same reason,
  RFC-0016 left Cohen's d without an interval ("Intervals for d and g
  need the noncentral t, which `statrs` lacks",
  `docs/rfc/RFC-0016-statistical-inference.md:516-517`).
- **Common tests are missing.** No word computes a signed-rank test, a
  test across more than two groups (Kruskal–Wallis, Friedman, one-way
  ANOVA), a normality test, a test of equal variances, a paired-table
  test (McNemar), a stratified test (CMH), a one-sample KS test, or a
  multiple comparison (Tukey's HSD). The `stat.test.` names are
  RFC-0015's and RFC-0016's (`bund2 words`, run at d51119a).
- **A study across VMs cannot be reproduced from one number.** A VM's
  generator is seeded from the operating system on first use
  (`crates/bund2-stdlib/src/biomath/rng.rs`, the module doc). `rng.seed`
  reseeds it (`rng.rs`, `reseed`). A spawned child reproduces only if
  the program passes it a seed. D260's C6 ruled that the safe idiom is
  an explicit derivation word, whose name and derivation are Phase D's
  to propose.

## Current behaviour

### What the reference does

- **The reference has none of these words.** Its registry, read from
  every literal `register_inline("…")` in `reference/rust_multistackvm/src`
  and `reference/Bund/src`, has 513 distinct names and none under
  `boot.`, `optim.` or `power.`. (That is a count of literal
  registrations. Coverage's 602 is its own in-scope set, a different
  count.) It also has no `stat.test.` name, and its `stat.` names are
  the `stat.count` family
  (`reference/Bund/src/stdlib/functions/statistics/count.rs:52-55`) and
  the `stat.mean.*` family
  (`reference/Bund/src/stdlib/functions/statistics/statistics.rs:185-215`).
- **`bootstrap` is a reference word**, the world loader, registered
  twice: as `stdlib_bund_load_disabled` under `--noio` and as
  `stdlib_bund_bootstrap` otherwise
  (`reference/Bund/src/stdlib/functions/bund/bund_load.rs:197`, `:203`).
  No name proposed here is `bootstrap` or begins with it. N1's `boot.`
  prefix is chosen so that none does.
- **A program's word wins over a native** of the same name. The
  reference evaluates a lambda before an inline
  (`reference/rust_multistackvm/src/multistackvm_apply.rs:46-59`), and
  Bund2 does the same (`crates/bund2-api/src/lib.rs`,
  `Registry::resolve_uncached`, `lambda` before `native`). A program
  that already defines one of the new names keeps its own meaning.

### What Bund2's own family does

- **Distributions are MAPs** with a `dist` key and one key per
  parameter (`dist.rs`, the module doc). `decode` finds the family by
  `dist` and reads exactly its listed keys. **It ignores any other
  key** (`dist.rs`, `decode`), so a hand-built `{ dist=t :: df=5 ::
  ncp=2 }` reads today as the central t on 5 df. Tails come as the pair
  (P(X ≤ x), P(X > x)) (`Family::tails`). Quantiles are polished to
  |cdf(x) − p| ≤ 10⁻¹², 100 steps at most, else E110
  (`Family::quantile`, `POLISH_CAP`, `POLISH_TOL`). An operation a
  family lacks is E105, `<family> has no <operation>` (`dist.rs`,
  `Family::undefined`; `refuse.rs`, `Refusal::Undefined`).
- **Lambdas are run through the bridge.** `dynamics.rs`'s `call` runs a
  model lambda with `crate::kb::bridge::ask`: on a scratch stack named
  for the nesting depth, with its arguments the only values there. It
  then reads the value left on top, which must be a LIST of n finite
  numbers, else E112 (`crates/bund2-stdlib/src/biomath/dynamics.rs`,
  `call`; `crates/bund2-stdlib/src/kb/bridge.rs`, `ask`). An error
  inside the lambda is passed on (`ask` returns `ran`'s error). The
  caller's stack is never visible to the lambda.
- **Lambda-calling words are opaque**, with the plain form only
  (`dynamics.rs` and `fit.rs`, each module doc; D250 B10).
- **Iterative methods are capped and refuse at the cap**:
  - `fit.nls`: 200 iterations of Levenberg–Marquardt, E110.
  - `fit.logistic`: 100 iterations of IRLS, E110.
  - ODE steps: 10⁶, E111.

  (`fit.rs`, `NLS_CAP`, `IRLS_CAP`; `dynamics.rs`, `STEP_CAP`.)
- **The generator.** `with_rng` runs a closure over the VM's `StdRng`,
  or over a per-thread one if the VM keeps none. `reseed` replaces it
  with `StdRng::seed_from_u64` of the seed's bits, so after the same
  `rng.seed`, draws repeat within one Bund2 version (`rng.rs`,
  `with_rng`, `reseed`, the module doc).
- **Sampling** (`sample.rs`, the module doc):
  - `rng.shuffle` is Fisher–Yates.
  - `rng.choice.replace` draws one `gen_range(0..n)` per element.
  - Each word draws at most 10⁸ elements (`DRAW_CAP`).
- **Tests answer MAPs.** RFC-0015's tests answer `statistic`, `p`,
  `df` and `method`, and RFC-0016's `.opts` variants add `estimate`,
  `lower`, `upper` and `alternative`
  (`crates/bund2-stdlib/src/biomath/infer.rs`, `answer`; RFC-0016 §S2).
- **Registration tests count names by prefix:**
  - every `dist.` and `rng.` name outside `sample`'s `WORDS` numbers
    `2 × WORDS.len() + 3` (`crates/bund2-stdlib/src/biomath/dist/tests.rs`,
    `every_word_is_registered_with_its_dot_form`);
  - every `stat.test.` name must be in `infer`'s `WORDS`, and every
    `fit.` name in `fit.rs`'s (RFC-0016's Current behaviour; RFC-0018
    §S9).
- **The unvaried-operand audit** lists every fixed native that consumes
  more operands than the palette varies (`crates/bund2-stdlib/src/lib.rs`,
  `UNVARIED`, 81 names at d51119a).

### What is linked, and what is not

- **`argmin` 0.10.0 and `argmin-math` 0.4.0 are in the lock**, through
  `augurs-forecaster` (`cargo tree -i argmin`). `argmin`'s Nelder–Mead
  needs a parameter type with `ArgminSub`, `ArgminAdd` and `ArgminMul`.
  Its BFGS also needs `ArgminDot`, `ArgminEye` and `ArgminTranspose`
  over the parameter and Hessian types (`argmin-0.10.0/src/solver/neldermead/mod.rs:319-323`,
  `src/solver/quasinewton/bfgs.rs:133-147`). `argmin-math` implements
  these for `Vec<f64>` only under its `vec` feature
  (`argmin-math-0.4.0/src/lib.rs:213-214`), and for `nalgebra` only
  under its `nalgebra_*` features. The build enables neither: `cargo
  tree -e features -i argmin-math` shows only `primitives` and
  `num-complex_0_4`. **This answers Q61: no solver is buildable from the
  lock as compiled.** Enabling `vec` would need a direct dependency on
  `argmin-math` with that feature (N3).
- **Noncentral distributions** are in neither `statrs` (research 07 §2)
  nor `dist.rs`'s `FAMILIES`.
- Everything else here is built on functions `dist.rs` and
  `special.rs` already have: the incomplete beta and gamma, lgamma,
  normal quantiles and the polish.

### What R does, as reasoning

None of this is a reference Bund2 must match (D72). It is where the
algorithms and defaults below come from, and each difference from it is
stated where it falls.

- **Power** (`src/library/stats/R/power.R:19-66`, `:83-117`;
  `power.anova.test.R:19-45`). Each function solves for the one
  parameter given as `NULL`, by `uniroot` over a stated interval with
  `extendInt`. Power for the t test is the upper tail of the noncentral
  t, with ν = (n − 1)·tsample and ncp = √(n/tsample)·δ/sd, at
  `qt(sig.level/tside, ν)`. `strict = FALSE` by default counts only that
  tail. The proportions' test uses the normal approximation. ANOVA's
  power is the noncentral F upper tail with λ = (k − 1)·n·between/within.
- **The noncentral t.** R's `pnt` is Lenth's AS 243 (`src/nmath/pnt.c:21`),
  which sums from the Poisson term 0 upward, its first weight
  ½·exp(−ncp²/2) (`pnt.c:100-101`). That weight underflows for a large
  ncp, and R then warns and answers 0 (`:105-110`). R answers 0 outright
  for one tail when ncp > 40 (`:75`), and its own comment calls the
  result wrong for ncp > 100 (`:49`). So R is not a check beyond ncp ≈
  38 to 40.
- **The studentized range.** `ptukey` is Copenhaver and Holland's
  Gauss–Legendre quadrature (`src/nmath/ptukey.c:35`, `wprob`, `:45`).
  `qtukey` is a secant iteration that stops when two iterates differ by
  less than 10⁻⁴, at most 50 iterations (`qtukey.c:119-190`, `eps`,
  `maxiter` at `:122-123`). Both answer NaN with a warning for df < 2
  (`ptukey.c:321`). So R's `qtukey`, and the intervals
  `TukeyHSD` builds from it (`TukeyHSD.R:66-76`), are accurate to about
  four digits. Bund2's quantile is polished to 10⁻¹² (§S3).
- **The rank tests** (`wilcox.test.R:23-24`, `:60-80`;
  `kruskal.test.R:59-64`; `friedman.test.R:56-59`):
  - The signed-rank test is exact when n < 50 and there are no ties or
    zeros, and otherwise normal with a continuity correction by default.
    Zeros are dropped.
  - Kruskal–Wallis divides by the tie correction 1 − Σ(t³ − t)/(n³ − n).
- **McNemar** corrects for continuity on a 2×2 table by default, when
  the table is not symmetric (`mcnemar.test.R:19`, `:48-50`).
- **CMH** on a 2×2×K table: `correct = TRUE`, `exact = FALSE`, and the
  Mantel–Haenszel common odds ratio (`mantelhaen.test.R:22`, `:66-112`).
  It stops on a table dimension below 2 (`:28`) and on a stratum whose
  counts sum below 2, "sample size in each stratum must be > 1"
  (`:49-50`). Any dimension above 2 is the generalised I×J×K test.
- **Shapiro–Wilk refuses identical values**: "all 'x' values are
  identical" (`shapiro.test.R:28`).
- **Shapiro–Wilk** is Royston's AS R94 (`swilk.c:3-4`), for 3 ≤ n ≤ 5000
  (`shapiro.test.R:25`; `swilk.c:127`). Its W uses Royston's
  polynomial approximations to the coefficients, and its p uses his
  normalising transformation. Neither is fixed by a definition.
- **The one-sample KS test** is exact when n < 100 and there are no
  ties (`ks.test.R:87`).
- **One-way ANOVA.** `oneway.test` defaults to Welch's form
  (`var.equal = FALSE`, `oneway.test.R:20`). `TukeyHSD` is
  Tukey–Kramer: the standard error √(MSE/2·(1/nᵢ + 1/nⱼ)), with p from
  `ptukey` (`TukeyHSD.R:66-76`).
- **Optimisers** (`optim.R:36-46`):
  - Nelder–Mead uses α = 1, β = 0.5, γ = 2, `reltol` √ε, and 500
    iterations.
  - BFGS uses 100 iterations, and a numerical gradient with step 10⁻³
    when none is given.
  - At the cap R answers `convergence = 1` and does not stop.
- **`uniroot`** is Brent's `zeroin`, `tol` ε^¼ ≈ 1.2·10⁻⁴, 1000
  iterations (`nlm.R:55-60`).
- **`integrate`** is QUADPACK's `dqags`, the 21-point Gauss–Kronrod rule
  with Wynn's ε-algorithm, and `dqagi`, 15-point on a transformed range,
  on an infinite one, with
  `rel.tol` ε^¼ and 100 subdivisions (`integrate.R:19-35`;
  `integrate.c:57`, `rdqk21`).
- **The bootstrap** (`boot` 1.3-32, `R/bootfuns.q`):
  - Percentile, basic and BCa intervals take order statistics
    interpolated on the normal-quantile scale (`norm.inter`, `:1119-1149`;
    `basic.ci`, `perc.ci`, `bca.ci`, `:1152-1213`).
  - BCa's bias correction is w = Φ⁻¹(#{t* < t₀}/R), and its acceleration
    is a = ΣL³/(6(ΣL²)^1.5), from the empirical influence values L
    (`:1205-1213`).
  - **By default `empinf` estimates L by regression when R ≥ n, and by
    the jackknife only when R < n** (`:1638-1650`). This is the one
    algorithmic choice in the bootstrap that a definition does not fix
    (N4).

## Design

### S1. Shape

- **Bund2's own (D72)**, in the default build, adding no crate
  (D251 A8, as a default), under the prefixes N1 rules. **N1's
  recommended option A** is used throughout this draft:
  - `stat.test.` and `stat.effect.` for tests and effect sizes, as
    RFC-0016 extended them;
  - `dist.` for distributions;
  - `rng.` for the seed;
  - `power.` for power and sample size, a new kind of operand (a
    design's parameters);
  - `boot.` for resampling, a new kind of operand (a resample);
  - `optim.` for the numerical methods over a lambda: minimising, root
    finding, quadrature and the Hessian;
  - `fit.` for `fit.mle`, another fit (D260 C5: "`stat.` or `fit.` for
    another test or fit").
- **Additive, strictly.** No existing word changes. §Preservation
  analysis lists what could.
- **Refusals** are RFC-0015 §S11's codes; no new code, and one new
  E106 text (§S11). B12 holds: no
  word answers NaN or ±∞ from finite operands. An iterative method at
  its cap is E110 (N6, as `fit.nls`). A lambda that leaves the wrong
  thing is E112, as `call` refuses one. A work cap reached is E111
  before the work (D39).
- **Lambdas** are run by `dynamics.rs`'s `call`, or a sibling of it
  reading one FLOAT, on the scratch stack. A word that calls a lambda is
  opaque, with the plain form only (B10). One that also takes a named
  statistic (§S8, D260 C7) is opaque whichever it is given, since its
  effect is declared once.
- **Forms.** Fixed words follow RFC-0015 §S3 (B10): one-series words in
  four forms, other consumers plain and `.`. Producers (`rng.derive`)
  are plain.
- **Options MAPs** follow RFC-0016 §S1. Keys are STRINGs. An unknown key,
  a key the word does not take, or a value of the wrong type or range is
  E101 naming the key. `conf` is a FLOAT in (0, 1). `alternative` is
  `"two.sided"`, `"less"` or `"greater"` wherever a word takes it; no
  word of this RFC gives the key another meaning (§S4 uses `sides`).
- **NONE as an option value is a new rule, for `power.` only.** In a
  `power.` options MAP, a key whose value is NONE is the parameter to
  solve for, as R's `NULL` is (§S4). Every other options MAP, this RFC's
  included, refuses a NONE value as a value of the wrong type, E101
  naming the key, as RFC-0016 §S1 does. This NONE is not D252's missing
  value: it marks a parameter, not a cell, and no data operand is read
  with it, so no `na` option applies.
- **Names.** §S2 to §S12 list every name. Each fixed word has its `.`
  form; each opaque word has none.

### S2. Noncentral distributions [N2]

Three new families, by **N2's option A**: new `dist` names, so that no
existing MAP changes meaning.

| Constructor | Operands | MAP |
|---|---|---|
| `dist.nct` | df ncp | `{ dist=nct :: df :: ncp }` |
| `dist.ncchisq` | df ncp | `{ dist=ncchisq :: df :: ncp }` |
| `dist.ncf` | df1 df2 ncp | `{ dist=ncf :: df1 :: df2 :: ncp }` |

- **Parameters:**
  - df, df1 and df2 are finite and > 0;
  - ncp is finite, and ≥ 0 for `ncchisq` and `ncf`; it may be negative
    for `nct`;
  - anything else is E104, with a text of Bund2's own, since `statrs`
    has no family to check them.
- **Every existing distribution word takes them:** `dist.pdf`,
  `dist.lpdf`, `dist.cdf`, `dist.sf`, `dist.quantile`, `dist.sample`,
  `dist.sample*`, `dist.mean` and `dist.var`.
- **Computation.** Each tail is a Poisson-weighted mixture: of central
  χ² tails for `ncchisq`, and of incomplete-beta tails for `ncf` and
  `nct`. The series is Bund2's own, for all three: it starts at the
  Poisson mode and sums outward until the remaining weight is below
  10⁻¹⁵ of the sum, so a large ncp does not underflow its first term.
  For `nct` this is not R's AS 243, which sums from term 0 and fails
  beyond ncp ≈ 40 (§What R does); there the 50-digit fixture is the only
  check. The upper and lower tails are each summed directly, as `dist.rs`
  computes its pairs, so neither is 1 minus something close to 1. More
  than 10⁶ terms is E111.
  - The densities are the matching mixtures.
  - The quantiles are `Family::quantile`'s bracketed Newton polish.
  - The means and variances are the closed forms. `nct`'s mean needs
    df > 1 and its variance df > 2; `ncf`'s need df2 > 2 and df2 > 4.
    Outside those ranges they are E105.
  - The draws are by definition: `nct` as (Z + ncp)/√(χ²_df/df);
    `ncchisq` as a Poisson(ncp/2) mixture of central χ² draws;
    `ncf` as (χ²_{df1}(ncp)/df1)/(χ²_{df2}/df2).
- **ncp = 0** answers the central family's values within the tolerance
  below, not to the bit. Criterion 2 checks that.

### S3. The studentized range

`dist.tukey` (`nmeans df -- MAP`), `{ dist=tukey :: nmeans :: df }`:
- the range of `nmeans` ≥ 2 normal means over an independent s on df ≥ 2
  degrees of freedom; R's `nranges` is 1;
- `dist.cdf`, `dist.sf` and `dist.quantile` take it, the quantile
  polished to 10⁻¹² as every continuous quantile is;
- `dist.pdf`, `dist.lpdf`, `dist.sample`, `dist.sample*`, `dist.mean`
  and `dist.var` are E105 `tukey has no <operation>`, as R has no
  density or draw for it;
- the cdf is the double integral by adaptive Gauss–Kronrod (§S6's
  integrator): over the χ-distributed s, of the probability that a range
  of `nmeans` normals is below q·s;
- `nmeans` is an INTEGER ≥ 2, at most 10⁴ (E104 above). df is finite and
  ≥ 2 (E104), as R refuses df < 2 (`ptukey.c:321`).

R's `qtukey` stops at 10⁻⁴ (§What R does). So this word's quantile, and
the HSD intervals built on it, can differ from R's printed values in
the fifth digit. The fixture (criterion 3) is the integral at 50
digits, not R's output.

### S4. Power and sample size

Each word takes one options MAP, `opts -- MAP`. It solves for the one
parameter left out and answers every parameter, with `method` naming
the calculation. Exactly one must be left out. Leaving out none or two is
E101, naming the parameters that are missing.

| Word | Parameters (keys) | Power is |
|---|---|---|
| `power.t` | `n`, `delta`, `sd` (1.0), `sig.level` (0.05), `power`; `type` `"two.sample"` (default), `"one.sample"`, `"paired"`; `sides` 2 (default) or 1; `strict` (false) | R's `power.t.test` body, the noncentral t upper tail |
| `power.prop` | `n`, `p1`, `p2`, `sig.level`, `power`; `sides`; `strict` | R's `power.prop.test` body, normal |
| `power.anova` | `groups`, `n`, `between.var`, `within.var`, `sig.level`, `power` | R's `power.anova.test` body, the noncentral F upper tail |
| `power.chisq` | `w`, `n`, `df`, `sig.level`, `power` | the noncentral χ² upper tail at λ = n·w² above the central (1 − α) quantile on df (Cohen's w) |

- **`sides`** is an INTEGER, 1 or 2, else E101. It is R's
  `alternative` (`"one.sided"`, `"two.sided"`, `power.R:22`, `:35`), a
  different meaning from the family's `alternative` (§S1), so it has a
  different key. `sides` 1 is R's one-sided body, which for `power.t`
  takes `delta` as given, R taking |delta| only when two-sided
  (`power.R:36`), and for `power.prop` takes |p1 − p2| either way
  (`:113`). The answer MAP holds `sides`; an `alternative` key is E101,
  a key the word does not take.
- **Defaults.** A key with a default above is never the one solved for
  unless the program sets it explicitly to NONE (§S1). That is the one way
  to leave out `sd` or `sig.level`, since a missing key takes its default.
- **Solving** uses §S6's Brent root finder, over R's intervals and with
  R's interval extension (`extendInt`, §What R does). The tolerance is
  10⁻¹⁰ relative, not R's ε^¼ (N7). `n` is answered as a FLOAT, as R
  answers it; a program rounds it up itself.
- **Domains:**
  - `sig.level` and `power` must be in (0, 1);
  - `n` must be ≥ 2, and `groups` ≥ 2;
  - `sd`, `within.var` and `w` must be > 0;
  - p1 and p2 must be in (0, 1);
  - each out-of-domain value is E101 naming the key;
  - no root in the extended interval is E110, method `the power
    equation's root`.
- **Forms.** Fixed, `opts -- MAP`, each with a `.` form taking `opts`
  from the workbench.

### S5. Further tests

Each answers RFC-0015's test MAP (`statistic`, `p`, `df` where there is
one, `method`). Where the test has one, it also answers RFC-0016's
estimate and interval keys. All are fixed, plain and `.`, and the `.`
form takes the top operand from the workbench.

| Word | Operands | Test |
|---|---|---|
| `stat.test.wilcoxon` | x mu | one-sample signed rank, against `mu` |
| `stat.test.wilcoxon.paired` | a b | signed rank on a − b |
| `stat.test.wilcoxon.opts` | x mu opts | with `alternative`, `exact`, `correct`, `conf` |
| `stat.test.wilcoxon.paired.opts` | a b opts | the same |
| `stat.test.kruskal` | groups | Kruskal–Wallis, tie-corrected, χ² on k − 1 |
| `stat.test.friedman` | table | Friedman, blocks as rows, treatments as columns, χ² on k − 1 |
| `stat.test.shapiro` | x | Shapiro–Wilk, Royston's AS R94, 3 ≤ n ≤ 5000 |
| `stat.test.ks1` | x d | one-sample KS against a distribution MAP |
| `stat.test.bartlett` | groups | Bartlett's χ² on k − 1 |
| `stat.test.levene` | groups | Levene's F, centred on each group's median (Brown–Forsythe) |
| `stat.test.levene.opts` | groups opts | `center` `"median"` or `"mean"` |
| `stat.test.mcnemar` | table | McNemar's χ², 2×2 corrected unless `correct` false; r×r on r(r − 1)/2 |
| `stat.test.mcnemar.opts` | table opts | `correct` |
| `stat.test.cmh` | tables | Cochran–Mantel–Haenszel on a LIST of 2×2 MATRIXes, corrected, with the MH common odds ratio and its interval |

- **`groups`** is a LIST of at least two series (LISTs or METRICS), each
  read as `whole` reads a series. A group with too few elements for the
  test is E101 naming the group.
- **The signed-rank test.** Zeros are dropped, as R drops them:
  - exact by the signed-rank distribution when n < 50 with no ties or
    zeros, unless `exact` says otherwise;
  - otherwise normal, with the tie-corrected variance and a continuity
    correction unless `correct` is false;
  - `exact` true with ties or zeros is E101, where R falls back with a
    warning (B12's spirit; an **[author]** choice);
  - its `estimate` is the Hodges–Lehmann pseudomedian. On the exact path
    its interval is R's, from signed-rank quantiles
    (`wilcox.test.R:95-130`); otherwise the root
    of the corrected z, found by §S6's root finder.
- **Shapiro–Wilk** ports `swilk.c`'s arithmetic. Its reference values
  are R's (criterion 6, D260 C4 option D; N10). A sample whose values
  are all identical is E101 naming `x`, as R refuses it
  (`shapiro.test.R:28`), since W's denominator is 0.
- **The one-sample KS test** is exact by the Marsaglia–Tsang–Wang
  matrix power, as R's `pkolmogorov_two_exact` cites it
  (`src/library/stats/src/ks.c:403-422`), when n < 100 with no ties, and otherwise Kolmogorov's
  limit. A distribution MAP whose family has no cdf (multinomial,
  Dirichlet) is E105.
- **CMH** reads each stratum as a 2×2 MATRIX of counts.
  - A stratum whose counts sum below 2 is E101 naming the stratum, as R
    stops on it (`mantelhaen.test.R:49-50`).
  - A stratum that is not a 2×2 MATRIX is E101 naming the stratum. R
    takes any dimension ≥ 2 (`:28`) as the generalised I×J×K test,
    which this RFC does not add.

### S6. Roots and quadrature

| Word | Operands | Answers |
|---|---|---|
| `optim.root` | f lower upper opts | `root`, `f.root`, `iterations`, `estim.prec` |
| `optim.integrate` | f lower upper opts | `value`, `abs.error`, `subdivisions` |

- **f** is a LAMBDA `x -- y`, run by the bridge with x a FLOAT. It must
  leave one finite number, else E112.
- **`optim.root`** is Brent's method as `zeroin`. `opts` takes:
  - `tol`, absolute, default 10⁻¹²·max(1, |lower|, |upper|) (N7);
  - `maxiter`, default 1000;
  - `extend`, `"no"` (default), `"yes"`, `"up"` or `"down"`, R's
    `extendInt`, doubling the interval at most 60 times.

  f(lower) and f(upper) of the same sign with `extend` `"no"` is E101,
  `f has the same sign at both ends`. A cap reached is E110.
- **`optim.integrate`** is QUADPACK's QAGS: the 21-point Gauss–Kronrod
  rule, bisection of the worst subinterval, and Wynn's ε-algorithm.
  `opts` takes:
  - `rel.tol`, default 10⁻¹⁰ (N7);
  - `abs.tol`, default `rel.tol`;
  - `subdivisions`, default 100.

  **An infinite end** is the STRING `"inf"` or `"-inf"` (N8). It is
  QUADPACK's QAGI: the range is mapped to (0, 1] by x = bound ±
  (1 − t)/t, and QAGI's 15-point Kronrod rule is used
  (`integrate.c:1633-1643`, `rdqk15i`). When the subdivisions
  are spent short of the tolerance, the answer is E110. A
  non-finite f is E112.
- Both are opaque, plain only.

### S7. Reproducible studies: `rng.derive`

`rng.derive` (`-- seed`) draws one `u64` from the VM's generator and
answers it as an INTEGER, reinterpreting the bits two's-complement as
`reseed` does. A program seeds a child VM by passing that INTEGER to
`rng.seed` inside the child. After `n rng.seed`, the k-th `rng.derive`
always answers the same INTEGER within a Bund2 version, so a study of
many VMs reproduces from one seed (D260 C6, option C). `vm.spawn` is
unchanged. Plain only (a producer).

`StdRng` is a cryptographic generator, so its successive outputs are
independent for this purpose. That `seed_from_u64` of successive outputs
gives streams that do not overlap in practice rests on `rand`'s
documentation and is [UNGROUNDED] (Q66).

### S8. The bootstrap and the jackknife [N4] [N5] [N12]

| Word | Operands | Answers |
|---|---|---|
| `boot.sample` | data statistic R opts | `t0`, `t` (the R replicates), `bias`, `se`, and one `[ lower upper ]` per interval asked for |
| `boot.jackknife` | data statistic | `t0`, `values` (the n leave-one-out values), `bias`, `se` |

- **data** is a LIST of any values, resampled by element, or a frame
  (RFC-0017), resampled by row (N12).
- **statistic** is a LAMBDA, `resample -- FLOAT`, or one of the named
  statistics (D260 C7; N5). The names are `"mean"`, `"median"`, `"sd"`
  and `"var"` over a LIST of numbers, each computed by the named
  `stat.` word's own function, so a named statistic's replicates equal
  the lambda's to the bit.
- **Resampling** draws from the VM's generator: each replicate is n
  `gen_range(0..n)` draws, as `rng.choice.replace` draws (`sample.rs`).
  After the same `rng.seed`, `boot.sample` repeats exactly.
- **R** is an INTEGER ≥ 2. R·n above 10⁸ is E111 before any draw (D39).
- **`opts`:**
  - `conf`, default 0.95;
  - `type`, a LIST of `"normal"`, `"basic"`, `"percentile"` and
    `"bca"`, default `[ "percentile" ]`.
- **The intervals:**
  - `"basic"`, `"percentile"` and `"bca"` are `boot`'s:
    `norm.inter`'s interpolation on the normal-quantile scale, its
    end-point rules, and `bca.ci`'s bias correction w and adjusted
    levels.
  - `"normal"` is t₀ − bias ∓ z·se.
  - **The arithmetic is one crate-internal function** of t₀, the
    replicates, the jackknife values, `conf` and the types, which
    `boot.sample` calls once its replicates are drawn. It draws nothing,
    so criterion 10 checks it on a committed replicate vector.
  - **BCa's acceleration is the jackknife's** by N4's option A:
    a = Σ(t̄₍.₎ − t₍ᵢ₎)³ / (6(Σ(t̄₍.₎ − t₍ᵢ₎)²)^1.5). This costs n
    more statistic calls. It equals `boot`'s default only when R < n
    (§What R does).
  - A w or a that is not finite (every replicate on one side of t₀, or
    a constant jackknife) is E101, where `boot` stops too
    (`bootfuns.q:1206`, `:1212`).
- **`boot.jackknife`** answers the leave-one-out values. Its bias is
  (n − 1)(t̄₍.₎ − t₀), and its se is √((n − 1)/n·Σ(t₍ᵢ₎ − t̄₍.₎)²).
  n < 2 is E101.
- Both are opaque, plain only.

### S9. Permutation tests [N5]

`stat.test.permutation` (`a b statistic opts -- MAP`) tests whether two
samples come from one distribution:
- the statistic is a LAMBDA `a b -- FLOAT`, or a name: `"mean.diff"`
  (the default when the operand is NONE), `"median.diff"`, or `"t"`,
  Welch's t;
- **exact** when C(n + m, n) ≤ 10⁶: every split is enumerated in
  lexicographic order, and p is the share of splits as or more extreme;
- **Monte Carlo** otherwise, or when `opts` says so: B shuffles, B = 9999
  by default, each `rng.shuffle`'s Fisher–Yates on the pooled sample,
  and p = (1 + #{as or more extreme})/(B + 1);
- `alternative` takes R's three values. "As extreme" means |T| ≥ |T₀|
  for two-sided, T ≥ T₀ for greater and T ≤ T₀ for less, each with a
  relative tolerance of 10⁻¹² so that ties in exact arithmetic count;
- answers `statistic`, `p`, `method` (`"exact permutation"` or
  `"Monte Carlo permutation"`), `permutations`, and `alternative`;
- B·(n + m) above 10⁸ is E111 (D39). Opaque, plain only.

### S10. Minimisers and the Hessian [N3] [N6] [N11]

| Word | Operands | Answers |
|---|---|---|
| `optim.nm` | f x0 opts | `par`, `value`, `iterations`, `evaluations` |
| `optim.bfgs` | f x0 opts | the same, and `gradient` at `par` |
| `optim.hessian` | f x opts | `gradient` and `hessian` (a MATRIX) at x |

- **f** is a LAMBDA, `x -- y`, with x a LIST of k FLOATs. It must leave
  one finite number, else E112. x0 is a LIST of k ≥ 1 finite numbers.
- **Bund2's own** (N3, option A), as Levenberg–Marquardt is (D250 B8):
  - **Nelder–Mead:** R's coefficients α = 1, β = ½, γ = 2; R's initial
    simplex, each vertex stepped by 0.1·max|x0ᵢ| (0.1 when that is 0)
    (`appl/optim.c:316-318`); stopped when the simplex's highest value
    is within `reltol`·(|f(x0)| + `reltol`) of its lowest, f(x0)
    computed once at the start, as R's `convtol` is (`appl/optim.c:302`,
    `:367`); `reltol` = √ε by default; at most 500 iterations.
  - **BFGS:** R's `vmmin`, with its backtracking line search (step
    shrunk by 0.2, sufficient decrease at 10⁻⁴, `appl/optim.c:104-106`,
    `:182-196`), a
    central-difference gradient of step `ndeps` 10⁻³ by default, and
    `reltol` the same; at most 100 iterations.
  - The cap is E110 (N6), where R answers `convergence = 1`.
- **`optim.hessian`** uses central differences of the central-difference
  gradient, at step `ndeps` per coordinate, 10⁻³ by default as R's
  `optimHess` defaults it (`optim.R:80-89`). The differencing scheme is
  Bund2's own; R's `C_optimhess` was not read. It answers the Hessian
  symmetrised.
- **No bounds** in this RFC (N11). A program reparameterises a positive
  parameter by its log.
- k above 100 is E111. All three are opaque, plain only.

### S11. `fit.mle` [N13]

`fit.mle` (`loglik start opts -- MAP`) maximises a log-likelihood
LAMBDA (`par -- FLOAT`, par a LIST) from `start`:
- by `optim.bfgs` on −loglik, or `optim.nm` with `method` `"nm"`;
- answers:
  - `par`, `se`, `vcov` (the inverse of `optim.hessian`'s Hessian of
    −loglik at `par`), `loglik`, `iterations`, `aic` (2k − 2ℓ̂), and
    `bic` when `opts` gives `n`;
  - with `profile` true (N13), `lower` and `upper` for each parameter:
    the two roots of 2(ℓ̂ − ℓₚ(θⱼ)) = χ²₁(conf), ℓₚ maximised over the
    others, each root by §S6's root finder from ℓ̂ outward;
- a Hessian of −loglik that is not positive definite at `par` is E106,
  by a variant of its own beside RFC-0018's `SingularColumn`, with the
  template `the Hessian of -loglik at par is not positive definite:
  pivot <j> is <v>`. j is the 1-based Cholesky pivot that is not
  positive, and v its value. The code stays E106; only the text is new.
  The standard errors do not exist there;
- B12: a non-finite `se` is E114;
- opaque, plain only. It joins `fit.rs`'s `WORDS`, as `fit.lm` and
  `fit.glm` did, so that `fit/tests.rs`'s registration test counts it.

### S12. One-way ANOVA, Tukey's HSD and an interval for d

| Word | Operands | Answers |
|---|---|---|
| `stat.test.anova1` | groups | the classical F on k − 1 and N − k, with `eta2` and `omega2` |
| `stat.test.anova1.opts` | groups opts | `var.equal` false gives Welch's F, as `oneway.test`'s default |
| `stat.test.tukey` | groups opts | a frame (RFC-0017), one row per pair i < j: `pair`, `diff`, `lower`, `upper`, `p` |
| `stat.effect.cohend.opts` | a b opts | `stat.effect.cohend`'s d and g, with `lower` and `upper` for d at `conf` |

- **`stat.test.tukey`** is Tukey–Kramer, as `TukeyHSD`: the MSE from
  the one-way fit, the standard error √(MSE/2·(1/nᵢ + 1/nⱼ)), p from
  §S3's sf, and the interval from §S3's quantile. `pair` names the
  groups by their 1-based positions, `"2-1"`, as R orders them.
  `opts` takes `conf`.
- **d's interval** inverts the noncentral t on nₐ + n_b − 2 df at the
  observed pooled t = d·√(nₐn_b/(nₐ + n_b)), with α = 1 − `conf`, each
  ncp by §S6's root finder:
  - the lower end's ncp δ_L leaves the upper tail α/2 at t:
    P(T > t; δ_L) = α/2;
  - the upper end's ncp δ_U leaves the lower tail α/2 at t:
    P(T ≤ t; δ_U) = α/2;
  - each end for d is δ·√(1/nₐ + 1/n_b), the ncp mapped back to d's
    scale.
- All fixed, plain and `.`.

### S13. The tiers, the audits and the generated files

As RFC-0016 §S8:
- effects as declared;
- the palette audit decides what `PROMOTABLE.txt` lists;
- `PROMOTABLE.txt`, `bund2 effects` and `docs/arity.md` are regenerated
  in each milestone;
- no golden is added (D21).

**`UNVARIED`** gains each fixed native that consumes three or more on
the stack, or a `.` form two or more (`lib.rs`, `varied_of`):
- `dist.ncf` and its `.` form, as `dist.hypergeometric` is;
- `stat.test.wilcoxon.opts`, `stat.test.wilcoxon.paired.opts` and
  `stat.effect.cohend.opts`, with their `.` forms.

Each milestone states its additions against the audit's own selection.

**The registration tests:**
- `dist/tests.rs`'s count of `dist.` and `rng.` names is amended for
  `rng.derive` (one name, no form), and `dist.rs`'s `WORDS` gains the
  four constructors;
- `stat.test.` names join `infer`'s `WORDS`, or a module of their own
  with an exclusion in that test, as RFC-0016 did;
- `fit.mle` joins `fit.rs`'s `WORDS`;
- `boot.`, `optim.` and `power.` are modules of their own, each with a
  counting test.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| Every existing `dist.`, `stat.`, `fit.`, `rng.`, `math.` word: operands, effects, forms, answers, refusals, cost | **Preserved exactly.** Every name here is new |
| `dist.rs`'s `decode` ignoring keys it does not list (`{ dist=t :: df=5 :: ncp=2 }` is the central t) | **Preserved exactly** by N2's option A: `ncp` is read only by the new families. Option B would change this MAP's meaning, and is why A is recommended |
| A MAP whose `dist` is `nct`, `ncchisq`, `ncf` or `tukey`: E101 today (`decode`, `not_one`) | **Changed by design:** it becomes a distribution |
| The VM's generator and every seeded stream of RFC-0015's and RFC-0016's words | **Preserved exactly:** the new words draw only when called. `rng.derive`, `boot.sample` and a Monte Carlo `stat.test.permutation` each advance the generator by their own draws, which a program that reseeds sees only after calling them |
| `fit.logistic`, `fit.nls`, `fit.lm`, `fit.glm` | **Preserved exactly.** `fit.mle` is a new name; `fit.rs`'s `WORDS` gains it |
| The reference's `bootstrap` (`bund_load.rs:197`, `:203`) | **Preserved exactly:** no name proposed is `bootstrap` or begins with it |
| A program that defines one of the new names itself | **Preserved:** a program's word is found before a native (cited above) |
| A program that asks whether one of the new names exists (`?stdlib`, `?word`) | **Changed by design:** the answer becomes true. No such program is known |
| Registration counting tests (`dist/tests.rs`, `every_word_is_registered_with_its_dot_form`; `infer`'s; `fit/tests.rs`) | **Amended deliberately**, each milestone stating its additions (§S13) |
| RFC-0015 §S11's refusals, their codes and texts | **Preserved exactly.** `refuse.rs` gains one E106 variant with a new text (§S11); no existing variant's text changes |
| `UNVARIED` | **Extended** by §S13's names, each with its reason |
| `PROMOTABLE.txt`, `bund2 effects`, `docs/arity.md` | **Regenerated**, existing name lines not moving |
| `conform` 143/159, CEILING 143/159; IMPLEMENTED 597/602, COVERAGE 577/602 | **Preserved exactly** (D72): no reference word is touched |
| Q61's question, whether `argmin` can be used | **Answered** (§What is linked), and N3 asks the owner which way to go |

## Alternatives considered

- **Two RFCs, one per phase.** This was D260 C8's ruling; the owner
  changed it for D and E (D260's note of 2026-11-21). The milestones
  below keep the phases apart, E first, so either half could be accepted
  alone.
- **`argmin` for the minimisers** (N3, option B). It is in the lock, but
  compiled with no backend (§What is linked). Enabling `argmin-math`'s
  `vec` feature adds a feature to a locked crate and a direct
  dependency. In exchange it gives solvers whose refusals, caps and
  iteration counts are argmin's, not this family's (E110 at a stated
  cap, D37 for a panic-free solver). Nelder–Mead and R's `vmmin` are each
  a few hundred lines, as Levenberg–Marquardt was (D250 B8).
- **An `ncp` key on the existing families** (N2, option B). It reads as R
  does (`pt(q, df, ncp)`), but a MAP a program built with a stray `ncp`
  today would change meaning (§Preservation analysis).
- **`boot`'s regression influence values for BCa** (N4, option B). This
  matches R's default for R ≥ n, but its answer depends on a regression
  over the resample frequencies, which no definition fixes. Under D260
  C4 it would need R and `boot` installed for its fixture. The jackknife
  acceleration is Efron's original definition, checkable at 50 digits,
  and is `boot`'s own choice for R < n.
- **R's tolerances** (N7, option B): ε^¼ for `uniroot` and `integrate`.
  They are R's defaults, but they are about 10⁻⁴, so answers would be
  good to four digits. Every other word of the family is checked at
  10⁻⁹ or tighter (B11).
- **`alternative` `"less"` and `"greater"` in `power.`** (SF7 of the
  first review). It would keep one key across the family, but R's
  one-sided bodies have no direction for proportions (|p1 − p2|,
  `power.R:113`), so `"less"` would need a meaning of Bund2's own.
  `sides` keeps R's bodies as they are.
- **Dunnett's test** (N9). It needs the multivariate t's cdf, which R's
  `multcomp` computes by Genz's randomised quasi-Monte Carlo. Its answer
  varies between runs and depends on the algorithm. Left out.
- **MCMC.** Research 07 §3.6 lists it, but its own §4 puts it in Phase H.
  Left out.
- **Bounds by transformation in the minimisers** (N11, option B).
  Research 07 §3.6 suggests them; leaving them out keeps the words'
  answers in the parameters the program gave. A later RFC can add a
  `lower` and `upper` key.

## Acceptance criteria

Reference values follow D260 C4's option D. By default they come from
definitions, computed with mpmath at 50 digits by fixture scripts under
`crates/bund2-stdlib/src/biomath/fixtures/`, as RFC-0015 to RFC-0018's
are. R, at the version recorded in its committed output (4.6.2 on the
owner's machine), is used only for Shapiro–Wilk (N10, as amended).
D266's note of 2026-11-22 finds `swilk.c`, `shapiro.test.R` and
`qnorm.c` byte-identical at R 4.4.2 and 4.6.2, so the R 4.4.2 source
cited here is the code that produces the fixture. Tolerances are |a − b| ≤ 10⁻⁹|b| + 10⁻¹² unless a
criterion says otherwise, and a p below 10⁻⁶ is compared absolutely at
10⁻¹².

1. **Registration.** Every name of §S2 to §S12 is registered with its
   declared effect and forms. The counting tests pass as §S13 amends
   them, and nothing else is added.
2. **Noncentral distributions.** For `nct`, `ncchisq` and `ncf`:
   - pdf, cdf, sf and quantile are checked against the fixture at a grid
     of parameters, including ncp 0, a large ncp (100, beyond R's AS 243)
     and a negative ncp for `nct`;
   - the mean and variance match their closed forms;
   - at ncp = 0 each matches the central family within the tolerance;
   - 10⁵ seeded draws have a sample mean within four standard errors of
     the mean.
3. **The studentized range.** `dist.tukey`'s cdf matches the 50-digit
   double integral at a grid of (nmeans, df, q), and its quantile
   inverts it to 10⁻¹². Every other operation is E105 with its text,
   and df < 2 is E104.
4. **Power.**
   - Each of R's help-page examples for `power.t.test`,
     `power.prop.test` and `power.anova.test` gives R's printed values,
     to the digits R prints [UNGROUNDED: those printed values are R's
     documentation, to be read from `src/library/stats/man/*.Rd` when
     the fixture is written; Q67].
   - Each answer matches the fixture's definition-based value at 10⁻⁹.
   - Each solved parameter, put back, gives the stated power to 10⁻⁹.
5. **Rank and table tests.**
   - The signed-rank test is exact against the fixture's enumeration,
     and the normal path matches the fixture's formula.
   - Kruskal–Wallis, Friedman, McNemar and CMH each match the fixture's
     formula on a worked input, ties included.
   - CMH's common odds ratio and interval match the fixture.
6. **Normality and variance tests.**
   - Shapiro–Wilk's W and p match R's `shapiro.test`, at the version its
     output records (≥ 4.4), on the inputs
     of a committed script (`fixtures/shapiro.R`), with R's version in
     its output, at 10⁻⁹ (N10). Identical values are E101.
   - The one-sample KS test's exact p matches the fixture's
     Marsaglia–Tsang–Wang value, and the asymptotic p Kolmogorov's limit.
   - Bartlett's and Levene's tests match the fixture's formulas.
7. **ANOVA and HSD.**
   - `stat.test.anova1`'s F, p, η² and ω², and Welch's F, match the
     fixture on a worked three-group input.
   - Its F equals `model.anova`'s F for the same data coded as a factor,
     at 10⁻⁹.
   - `stat.test.tukey`'s rows match the fixture's Tukey–Kramer values
     from criterion 3's distribution.
   - Cohen's d interval matches the fixture's noncentral-t inversion.
8. **Roots and quadrature.**
   - `optim.root` finds the roots of a stated set of functions (a cubic,
     cos x − x, a function needing `extend`) to its tolerance.
   - `optim.integrate` matches closed-form integrals to 10⁻¹⁰ relative:
     a polynomial, e^(−x²) over (−inf, inf), 1/√x on (0, 1], a
     peaked function.
   - Each refusal (same sign, the cap, f not finite, f not a lambda)
     gives its text.
9. **`rng.derive`.** After `42 rng.seed`, three `rng.derive` calls answer
   the same three INTEGERs every run. A VM seeded with each of them
   repeats its own stream.
10. **The bootstrap and the jackknife.**
    - After a seed, `boot.sample` repeats exactly. Its replicates equal
      a Bund program's own loop of `rng.choice.replace` and the
      statistic, to the bit.
    - The interval function (§S8) matches the fixture on a committed
      replicate vector and jackknife vector: `fixtures/boot.py` applies
      `norm.inter`, `basic.ci`, `perc.ci` and `bca.ci`'s formulas, with
      the jackknife's acceleration, at 50 digits, and the test runs the
      function on the same vectors.
    - `boot.sample`'s intervals equal that function applied to its own
      `t`, `t0` and jackknife values, to the bit.
    - A named statistic's replicates equal the lambda's.
    - `boot.jackknife` matches the fixture's leave-one-out values.
11. **Permutation.**
    - The exact test's p matches the fixture's enumeration on a small
      input, for each alternative.
    - The Monte Carlo test repeats after a seed, and its p is within
      three standard errors of the exact p on an input where both apply.
12. **Minimisers and `fit.mle`.**
    - `optim.nm` and `optim.bfgs` find Rosenbrock's minimum (1, 1) from
      (−1.2, 1), to 10⁻⁴ and 10⁻⁶ respectively.
    - `optim.hessian` matches the closed-form Hessian of a quadratic to
      10⁻⁶.
    - `fit.mle` of a normal sample gives the closed-form MLEs, and
      standard errors within 10⁻⁵ of σ̂/√n and σ̂/√(2n).
    - `fit.mle`'s profile interval for an exponential rate matches the
      fixture's χ² inversion to 10⁻⁶.
    - The caps give E110, and a likelihood whose Hessian is indefinite at
      `par` gives §S11's E106 text.
13. **Example.** `bund_examples/inference/` plans a sample size, tests,
    resamples and fits by maximum likelihood, prints `expected.txt`, and
    a test in `crates/bund2-cli/tests` compares the two, as
    `bund_examples/models/` does.
14. **No abort.** Every name and form, on a palette of NaN, ±∞, wrong
    types, empty, ragged and NONE-filled operands, lambdas that leave
    nothing, NaN or a STRING, and lambdas that raise an error, answers or
    refuses with an E1nn, a shallow-stack error or a bridge error.
15. **Health.**
    - `cargo xtask conform` prints 143/159 and CEILING 143/159, and
      `coverage` IMPLEMENTED 597/602 and COVERAGE 577/602.
    - clippy, `cargo fmt --check` and `cargo deny` are clean.
    - `PROMOTABLE.txt` is regenerated with no existing name line moved.
    - The `UNVARIED` audit passes with §S13's names.
    - RFC-0015 to RFC-0018's tests pass unchanged, except the counting
      tests §S13 amends.

## Implementation plan

Phase E first: the power words need its distributions, and Phase D's
`fit.mle` needs Phase E's root finder.

| Milestone | Scope | Criteria |
|---|---|---|
| M1 | §S2: noncentral t, χ², F | 2 |
| M2 | §S6: `optim.root`, `optim.integrate`; §S3: the studentized range | 8, 3 |
| M3 | §S4: power | 4 |
| M4 | §S5: signed-rank, Kruskal–Wallis, Friedman, McNemar, CMH | 5 |
| M5 | §S5: Shapiro–Wilk, one-sample KS, Bartlett, Levene | 6 |
| M6 | §S12: one-way ANOVA, Tukey's HSD, d's interval | 7 |
| M7 | §S7: `rng.derive`; §S8: the bootstrap and the jackknife | 9, 10 |
| M8 | §S9: permutation tests | 11 |
| M9 | §S10, §S11: minimisers, the Hessian, `fit.mle` | 12 |
| M10 | the example | 13 |

Each milestone ends with criteria 1, 14 and 15. M5 needs R ≥ 4.4 for
Shapiro–Wilk's fixture (N10, as amended). R 4.6.2 is installed on the
owner's machine, so that setup step is done.

## Open questions

**Ruled 2026-11-21 (D266):** the owner accepted every recommendation,
"N1–N13: recomendations". The "Recommended" column below is what was
ruled. The **[author]** choices that follow were not part of that
ruling; the owner ruled them as written the same day (D267), "rule on
the author choices: recommendations".

| | Question | Options | Recommended |
|---|---|---|---|
| **N1** | Names | **A** `boot.` for resampling; `optim.` for minimising, roots, quadrature and the Hessian; `power.` for power; `dist.`, `stat.test.`, `stat.effect.`, `rng.`, `fit.` as now. **B** as A, but `num.root`, `num.integrate` and `num.hessian`, keeping `optim.` for minimisers. **C** everything under `stat.` | **A**: one prefix for "a numerical method over a function", and D260 C5's rule of a new prefix for a new kind of operand |
| **N2** | Noncentral distributions | **A** new families `nct`, `ncchisq`, `ncf`, each with a constructor. **B** an optional `ncp` key on `t`, `chisq` and `f` | **A**: no existing MAP changes meaning |
| **N3** | Minimisers | **A** Bund2's own Nelder–Mead and BFGS (R's `vmmin`), as D250 B8 wrote Levenberg–Marquardt. **B** `argmin`, enabling `argmin-math`'s `vec` feature by a direct dependency | **A**: the family's caps, refusals and no-panic rule hold by construction |
| **N4** | BCa's acceleration | **A** the jackknife's, by definition. **B** `boot`'s default, regression when R ≥ n, with R and `boot` installed for the fixture | **A** |
| **N5** | Named statistics (D260 C7) | **A** `"mean"`, `"median"`, `"sd"`, `"var"` for `boot.`; `"mean.diff"`, `"median.diff"`, `"t"` for permutation. **B** lambdas only | **A** |
| **N6** | A method at its cap | **A** E110, as `fit.nls` and `fit.glm`. **B** answer with `converged` false, as R's `optim` | **A** |
| **N7** | Default tolerances of `optim.root`, `optim.integrate` and the power solve | **A** Bund2's tight ones (10⁻¹²·scale, 10⁻¹⁰ relative). **B** R's ε^¼ | **A** |
| **N8** | An infinite end in `optim.integrate` | **A** the STRINGs `"inf"` and `"-inf"`. **B** none in this RFC | **A** |
| **N9** | Dunnett's test | **A** left out. **B** included, with an R fixture | **A** |
| **N10** | R for Shapiro–Wilk's fixture (D260 C4 rules that R is used; this asks how) | **A** CRAN's R 4.4.2 installed on the owner's machine, the script and its output committed. **B** any R ≥ 4.4, its version recorded. **C** no R: the port checked against Royston's published examples only | **A**, amended to **B** 2026-11-22 (D266's note): R 4.6.2 |
| **N11** | Bounds in the minimisers | **A** none in this RFC. **B** `lower` and `upper` by transformation | **A** |
| **N12** | What `boot.` resamples | **A** a LIST by element, or a frame by row. **B** a LIST only | **A** |
| **N13** | `fit.mle`'s profile intervals | **A** with `profile` true, off by default. **B** always. **C** none | **A** |

**[author]** choices, ruled as written (D267):
- `exact` true with ties is E101 (§S5);
- `dist.tukey` has no density or draw (§S3);
- `stat.test.tukey`'s `pair` names groups by position (§S12);
- the exact-permutation cap of 10⁶ splits (§S9);
- `power.` solving `n` as a FLOAT (§S4).
