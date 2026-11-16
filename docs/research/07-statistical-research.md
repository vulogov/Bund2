# 07 — Statistical research: what the standard library would still need

*Research, 2026-11-15, on the owner's request: "extension of the standard
library mathematical functions that will make Bund suitable for
statistical research". Nothing here is decided. Every choice below is a
question for an RFC and its D-entry, not a default to be adopted
(CLAUDE.md, "Registers are append-only").*

## 1. The question

RFC-0015 gave Bund2 the arithmetic of mathematical biology: special
functions, distributions, descriptive statistics, a set of classical tests,
linear algebra, seeded randomness, ODEs and two fitting words. Statistical
*research* asks for more than a set of answers to textbook questions. A
researcher has a table of observations, some of them missing, some
categorical; fits a family of models to it and compares them; asks for
intervals and effect sizes as well as p-values; checks the assumptions;
resamples when the theory runs out; and writes the results out to be read
and reproduced.

What would a Bund program need for that workflow, beyond what RFC-0015
built? In what order could it be built, and which questions would only the
owner be able to answer?

The rules that framed research 06 frame this one too:

- **D72.** Every word here would be Bund2's own. `conform` (143/159,
  CEILING 143/159) and `IMPLEMENTED`/`COVERAGE` (597/602, 577/602) do not
  move, whatever is added.
- **D21.** No golden can hold a Bund2-only word. Unit tests against
  published or independently computed values are the only pin, as they are
  for RFC-0015 (`docs/rfc/RFC-0015-biomathematics.md`, §S17, "No golden is
  added").

## 2. What Bund2 has after RFC-0015

Taken from `bund2 words` on `development/0.56.0` after RFC-0015's M13,
plain forms only.

| Area | Words | What a researcher misses |
|---|---|---|
| Descriptive | `stat.count`, `stat.mean.*`, `stat.var`, `stat.sd`, `stat.se`, `stat.median`, `stat.mode`, `stat.quantile` (R's type 7), `stat.skewness`, `stat.kurtosis`, `stat.zscore`, `math.min`, `math.max` | weighted variance, trimmed mean, MAD, IQR, ECDF, histograms and binning, a one-call summary |
| Association | `stat.cov`, `stat.cor.pearson`, `stat.cor.spearman`, `stat.cor.kendall` | each answers the coefficient only (`crates/bund2-stdlib/src/biomath/describe.rs`, `pearson` and `stat_cor_pearson`): no test, no interval, no correlation matrix, no partial correlation |
| Tests | `stat.test.t1`, `.t2`, `.welch`, `.paired`, `.chisq`, `.chisq.table`, `.fisher`, `.binom`, `.mannwhitney`, `.ks2` | every test answers `statistic`, `p`, `df` and `method` (`crates/bund2-stdlib/src/biomath/infer.rs`, `answer`): no confidence interval, no estimate, no effect size, no alternative but two-sided |
| Corrections | `stat.padjust.bonferroni`, `.holm`, `.bh` | Hochberg, Hommel, Benjamini–Yekutieli |
| Distributions | 17 constructors (`dist.normal` … `dist.dirichlet`) and `dist.pdf`, `.lpdf`, `.cdf`, `.sf`, `.quantile`, `.sample`, `.mean`, `.var` | noncentral t, F and χ² (power), the studentized range (Tukey), Weibull, Cauchy, Laplace, logistic, beta-binomial |
| Models | `stat.lm` (OLS with an intercept: `coef`, `se`, `t`, `p`, `r2`, `df`; `infer.rs`, `stat_lm`), `fit.nls`, `fit.logistic` | GLMs in general, weights, offsets, F and likelihood-ratio tests between models, ANOVA tables, residuals, fitted values, diagnostics, intervals for predictions |
| Linear algebra | `linalg.mul`, `.mulv`, `.t`, `.inv`, `.solve`, `.det`, `.eigen.sym`, `.eigenvalues`, `.eigenvalues.complex`, `.dominant`, `.power`, `.identity` | SVD, QR and Cholesky as words, least squares on a MATRIX, rank, pseudo-inverse |
| Randomness | `rng.seed`, `rng.uniform`, `dist.sample`, `dist.sample*`; `vm.spawn` for replicates | shuffle, sampling with and without replacement, the bootstrap |
| Optimisation | `fit.nls`'s Levenberg–Marquardt, internal to it | a general minimiser over a lambda, maximum likelihood with standard errors, root finding, quadrature |
| Time series | `forecast.*`, `outlier.detect*`, `sample.analysis` (D215) | ACF and PACF, Ljung–Box, ARIMA fitting as a model, differencing beyond `vec.diff` |
| Data | `csv` (a column as a LIST, or a row at a time); MAP, LIST, MATRIX; `json` | a table with named columns, categorical variables, missing values |

Two observations from building RFC-0015's examples (M13) belong here
because they are about the workflow, not about any one word:

- **There is no rounding to a number of places, and no numeric format.**
  `math.round` rounds to an integer; `format` fills a `leon` template, one
  stack value per key, with no precision specifier
  (`crates/bund2-stdlib/src/control.rs`, `format` and `format_base`). The example had to define
  `{ 1000000.0 * math.round 1000000.0 swap / }` to print six decimals.
- **A model or a statistic given as a lambda already works.** RFC-0015
  runs `ode.*`, `sim.gillespie` and `fit.nls`'s models through
  `kb::bridge::ask` on a scratch stack (`crates/bund2-stdlib/src/biomath/dynamics.rs`,
  `call`; `fit.rs`, `Nls::residuals`). A bootstrap's statistic, a
  likelihood for `optim` or a permutation test's statistic would use the
  same mechanism, so no new re-entering function is needed for any of
  them.

### What is linked, and what is not

| Crate | Status | Offers | Lacks |
|---|---|---|---|
| `statrs` 0.18.0 | direct | 33 distribution families: the directory `statrs-0.18.0/src/distribution/` holds Cauchy, Erlang, Gumbel, InverseGamma, Laplace, Pareto, Triangular, Weibull, MultivariateStudentsT and Empirical beside those RFC-0015 exposes | no noncentral distribution and no studentized range: no file in that directory names either, and no source file under `src/` mentions "noncentral" |
| `nalgebra` 0.33.3 | direct (RFC-0015 M1) | SVD (`nalgebra-0.33.3/src/linalg/decomposition.rs:80`), QR, Cholesky, LU, eigen | sparse matrices |
| `argmin` 0.10.0 | linked, not usable | — | its Gauss–Newton needs `argmin-math` backends the lock does not compile (RFC-0015, "What is linked", lines 238-241); the same is likely true of its other solvers, **[UNGROUNDED]** until each is checked (Q61) |
| `augurs-*` | direct (D215) | ETS, MSTL, outlier detection | ARIMA as a fitted model with its standard errors |

So noncentral distributions and Tukey's studentized range would be Bund2's
own code, as RFC-0015's Levenberg–Marquardt is (D250 B8). PCA, rank and
the pseudo-inverse are `nalgebra`'s SVD.

## 3. The gaps, by what a researcher does

### 3.1 Represent the data: a table, factors and missing values

This is the structural gap; every later section is easier with it closed
and awkward without it.

- **A table.** Statistical work is column-oriented with named columns of
  equal length. Bund has MAP, LIST and MATRIX; a MAP of equal-length LISTs
  is the natural convention (`"dose" get`), and a `frame.` family could
  check, select, filter, sort, group, join and summarise it. The `csv`
  conditional already infers column types (`bund_examples/csv_stats/README.md`,
  "How it works"); reading a whole file into such a MAP is a small step.
- **Categorical variables.** A model needs to know that a column of
  STRINGs is a factor with levels, and how to code it: treatment, sum or
  Helmert contrasts. Without that, `stat.lm`'s user builds dummy columns by
  hand.
- **Missing values.** RFC-0015 refuses a NaN or ±∞ operand with E101
  (`docs/rfc/RFC-0015-biomathematics.md:854`, B12) and has no notion of a
  missing value. Real data have gaps, and R's `na.rm` and complete-case
  analysis are routine. **The obvious marker, NODATA, is already taken**:
  the series reader drains the stack until it meets a NODATA
  (`crates/bund2-stdlib/src/series.rs`, its module doc and `get_data`), so a NODATA inside a
  LIST would mean one thing to a series word and another to a `frame.`
  word. That is question C2.

### 3.2 Specify a model: design matrices and formulas

- **`model.matrix`**: a design MATRIX from a frame and a list of terms,
  with an intercept, factor contrasts and interactions. R's formula
  language (`y ~ a * b + log(x)`) is a parser of its own; a LIST of terms
  (`[ "a" "b" [ "a" "b" ] ]`) gives the same matrix with no new syntax.
  Question C3.
- **Weights and offsets** for every model word.

### 3.3 Fit and compare models

| Need | Why | Source |
|---|---|---|
| `fit.glm`: binomial (with counts), Poisson, Gamma, Gaussian, inverse Gaussian; canonical and common links; offsets and weights | count data, rates, positive skewed responses; `fit.logistic` becomes one case | own IRLS, as `fit.logistic` (`fit.rs`) |
| Negative binomial and quasi-families | overdispersed counts, the norm for sequencing and ecological counts | own; θ by alternating ML |
| Model comparison: F test, likelihood-ratio test, AIC, BIC, `anova` tables (type I sequential; II and III with contrasts) | every model-building study | own, from the fits' deviances and residual sums |
| Diagnostics: residuals (raw, Pearson, deviance, standardised), fitted values, leverage, Cook's distance, variance inflation | checking a model before believing it | own, from the QR already computed |
| Robust standard errors (HC0–HC3 sandwich) | heteroscedastic data | own |
| Prediction with confidence and prediction intervals | using a model | own |
| Mixed models (random intercepts and slopes, REML) | repeated measures, clustered designs | own, and large: penalised least squares with a sparse Cholesky. A phase of its own, or an RFC of its own |
| Survival: Kaplan–Meier with Greenwood intervals, the log-rank test, Cox proportional hazards (Breslow or Efron ties) | clinical and ecological time-to-event data | own |

### 3.4 Infer: intervals, effect sizes, power

- **Every test answers its interval and estimate**, as R's `htest` does:
  the mean difference and its t interval, the Hodges–Lehmann estimate for
  Mann–Whitney, the odds ratio and its exact interval for Fisher's test,
  Clopper–Pearson for the binomial, and a one- or two-sided alternative.
  RFC-0015's answer is a MAP (`crates/bund2-stdlib/src/biomath/infer.rs`, `answer`), so adding keys breaks no
  program that reads `p`.
- **Correlation tests**: Pearson's t and Fisher-z interval, Spearman's and
  Kendall's exact and asymptotic p.
- **Effect sizes**: Cohen's d and Hedges' g, η² and ω², Cramér's V, odds
  and risk ratios with intervals.
- **Power and sample size** for t tests, proportions, ANOVA and χ²: these
  need the noncentral t, F and χ² distributions, which `statrs` lacks
  (§2).
- **More tests**: Wilcoxon signed-rank, Kruskal–Wallis, Friedman,
  Shapiro–Wilk, Levene and Bartlett, McNemar, Cochran–Mantel–Haenszel, the
  one-sample Kolmogorov–Smirnov test against a distribution MAP, and
  one-way ANOVA with Tukey's HSD (the studentized range, §2) and Dunnett.
- **Corrections**: Hochberg, Hommel and Benjamini–Yekutieli, beside
  RFC-0015's three.

### 3.5 Resample and simulate

- **The bootstrap**: `stat.bootstrap` (`data statistic B -- MAP`) with
  percentile, basic and BCa intervals; the statistic a lambda run as a
  model is.
- **Permutation tests**: `stat.test.permutation` for two samples and for
  a statistic over a frame, with exact enumeration below a cap (D39) and
  Monte Carlo above it.
- **The jackknife**, for bias and standard error.
- **Sampling words**: `rng.shuffle`, `rng.choice` with and without
  replacement and with weights.
- **Parallel replicates**: `vm.spawn` already gives a replicate its own
  seed and thread (RFC-0015 worked example 3). A word that seeds child
  streams from one master seed, so that a whole study reproduces from one
  number, would make that idiom safe. Question C6.

### 3.6 Estimate by optimisation and integration

- **`optim`**: Nelder–Mead and BFGS over a lambda, with bounds by
  transformation, answering the minimum, the parameters and the Hessian.
- **`fit.mle`**: maximum likelihood over a log-likelihood lambda, with
  standard errors from the inverse numerical Hessian and profile-likelihood
  intervals; most bespoke models in research are fitted this way.
- **Root finding** (Brent), **quadrature** (adaptive Gauss–Kronrod), and
  numerical derivatives, for expectations, normalising constants and
  inverse problems.
- **Bayesian computation**: a random-walk Metropolis or slice sampler over
  a log-density lambda, with effective sample size and R̂ as diagnostics.
  At Tier 0 each density call is an interpretation (RFC-0015 §S15 says the
  same of a model), so a long chain is slow; question C7.

### 3.7 Multivariate and exploratory

- **PCA** by SVD (centred, optionally scaled), with loadings, scores and
  explained variance; classical MDS from `linalg.eigen.sym`.
- **Clustering**: k-means with seeded starts; hierarchical clustering with
  single, complete, average (UPGMA, already in `phylo.`) and Ward linkage.
- **Hotelling's T², MANOVA, linear discriminant analysis.**
- **Density estimation**: kernel density with a Silverman or
  Sheather–Jones bandwidth; the ECDF; histogram bins (Sturges, Scott,
  Freedman–Diaconis).

### 3.8 Time series as statistics

ACF and PACF with their bands, Ljung–Box, differencing and seasonal
differencing, and ARIMA fitted as a model with coefficients and standard
errors. `forecast.*` forecasts (D215) but does not answer a fitted model a
researcher can test.

### 3.9 Report

- **Rounding to places and significant figures**, and a number format with
  a precision (§2's first observation). `math.round` is the reference's,
  so a word with places would be a new name, not a change.
- **Writing a table**: CSV out, and a plain-text table of a frame.
- **A model summary** as text, in the layout statisticians expect.

## 4. A possible order of work

Each phase depends only on those before it, and each is usable alone.

| Phase | Delivers | Needs first |
|---|---|---|
| A | Rounding and number formatting; intervals, estimates and alternatives on RFC-0015's tests; correlation tests; the three further corrections; effect sizes; sampling words | nothing |
| B | Frames, factors and missing values (§3.1); reading a CSV into a frame and writing one out | C1, C2 |
| C | `model.matrix`; `fit.glm` with weights and offsets; residuals, diagnostics, prediction; F and likelihood-ratio tests; `anova` | B, C3 |
| D | The bootstrap, permutation tests, the jackknife; `optim`, `fit.mle`, root finding, quadrature | A |
| E | Noncentral distributions and power; the studentized range and Tukey's HSD; the further nonparametric and normality tests | A |
| F | Multivariate: PCA, MDS, clustering, Hotelling, LDA; density estimation | B |
| G | Survival analysis; time-series statistics | C |
| H | Mixed models; MCMC | C, D, and C7 |

Phases A, D and E change no data representation and could be an RFC of
their own soon. Phase B is the one with language-level consequences and
needs its decisions first.

## 5. Questions an RFC must put to the owner

None of these has a default to adopt.

- **C1. A table's representation.** A MAP of equal-length LISTs by
  convention, checked by each `frame.` word; a CLASS instance (RFC-0009),
  so that a frame knows its column order and factor levels; or a new value
  type, which touches the value crate, the wire format and the JIT, as
  research 06's B3 said of a packed vector.
- **C2. Missing values.** A NaN that `frame.` and model words treat as
  missing while every RFC-0015 word keeps refusing it (B12 stands); NODATA
  inside a LIST, against the series reader's use of it (§3.1); or a value
  of its own. And the policy: refuse, drop the case (`na.omit`), or an
  option per word.
- **C3. Model specification.** A LIST of terms, a formula STRING parsed by
  a word, or both.
- **C4. Reference values.** R is not installed where RFC-0015 was built;
  its M5 and M12 fixtures compute answers from the definitions with mpmath
  (`crates/bund2-stdlib/src/biomath/fixtures/infer.py` and
  `crates/bund2-stdlib/src/biomath/fixtures/fit.py`). That
  method does not reach Cox regression, mixed models or BCa intervals,
  whose answers depend on an algorithm's choices. Install R and commit its
  output with its version, use `statsmodels` and SciPy as the oracle, or
  stay with definitions and published worked examples.
- **C5. Names.** Extend `stat.` and `fit.`, or new prefixes: `frame.`,
  `model.`, `surv.`, `mv.`, `boot.`, `optim.`.
- **C6. Reproducible studies.** A master seed that derives each child
  VM's and each resample's stream, or the program's own seeding as today.
- **C7. Speed of lambda-heavy work.** A bootstrap of 10⁴ resamples or an
  MCMC chain of 10⁵ steps runs the lambda that often at Tier 0. Accept
  it, provide the common statistics as names a word can run natively
  (`"mean"` in place of a lambda), or wait for Tier 1 to cross lambdas.
- **C8. One RFC or several.** Phase A alone is small and RFC-0015-shaped;
  B to H are a programme. RFC-0015 took thirteen milestones for a
  comparable breadth.

## 6. What would make the work tractable

- **RFC-0015's machinery carries over.** S11's refusals (E101–E115), the
  operand readers in `biomath/operand.rs`, the bridge for lambdas, the
  `.` forms and the PROMOTABLE and unvaried-operand audits are in place,
  and each new word joins them as RFC-0015's did.
- **The QR is already there.** `stat.lm` and `fit.logistic` both solve by
  `nalgebra`'s QR with a rank test (`infer.rs`, `stat_lm`; `fit.rs`,
  `weighted_solve`), so diagnostics, GLMs and ANOVA are extensions of code
  that exists, not new numerics.
- **Fixtures by definition work, within limits** (C4): RFC-0015's
  M5, M9 and M12 fixtures reproduced R's printed answers from first
  principles, and the same method serves Phases A, D and E.
