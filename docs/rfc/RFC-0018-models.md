# RFC-0018: Models: design matrices, generalised linear models, diagnostics and comparison

- Status: **Draft** (2026-11-19). Drafted on the owner's `/rfc-draft 0018`,
  after RFC-0017 (research 07's Phase B) was built and D260 ruled
  research 07's C3–C8.
- Scope: research 07's Phase C (`docs/research/07-statistical-research.md:218`;
  §3.2 at `:109-115`, §3.3 at `:118-127`): `model.matrix`, `fit.glm` with
  weights and offsets, residuals, diagnostics, prediction, F and
  likelihood-ratio tests, and `anova`.
- Depends on: RFC-0015 (the `biomath` conventions: §S3's forms, §S11's
  refusals E101–E115, the operand readers, `stat.lm` and `fit.logistic`),
  RFC-0016 (§S1's options-MAP rules), RFC-0017 (the frame, its check, its
  kinds and levels, NONE and the `na` option), RFC-0004 (effects)
- Decisions consumed: **D260** (C3 A, C4 D, C5 C, C6 C, C7 B, C8 A),
  D250 (B5, B10, B11, B12, B15), D251 (A3, A4), D252 (C1 B, C2 C, and its
  notes), D253 (P2, P8, P9), D256 (S6), D259, D21, D36, D37, D39, D72.
- Reference SHA: `reference/rust_multistackvm`
  4605832678d4792c2b8e0564b7757d5aee1539a4; `reference/Bund`
  21b40b0213a76f3f39bd6de0d2bdfdf0a53b0dca; `reference/rust_dynamic`
  ceb27c96fa10c4edb09106f3f904f22f94d63119. Bund2's own code is cited by
  file and symbol, at `development/0.56.0` after 0eaf6c4. R is cited as
  reasoning, never as a reference this RFC must match: R 4.4.2's source,
  read through the `wch/r-source` mirror at the tag `R-4-4-2`
  (`https://raw.githubusercontent.com/wch/r-source/tags/R-4-4-2/`), as
  Q64 read it.
- Supersedes: nothing. Grounds and refines research 07's Phase C (§4).

Every passage that rests on a question this RFC puts to the owner is
marked with it, **[K*n*]**, and is written against the recommended
option. None is adopted until ruled (§Open questions).

## Summary

A model in Bund2 today is two words that take a MATRIX of predictors
and a LIST of responses: `stat.lm`, ordinary least squares, and
`fit.logistic`, logistic regression for 0/1 responses. Neither reads a
frame, codes a factor, takes weights or an offset, or answers residuals,
fitted values or anything a later word could compare. This RFC adds a
**design**, made from a frame by `model.matrix` from a LIST of terms
(D260 C3); two fitting words over it, `fit.lm` and `fit.glm`, the latter
for the binomial, Poisson, Gaussian, Gamma and inverse Gaussian families
under R's links; and words over a fit: `model.residuals`,
`model.influence`, `model.predict`, `model.compare` and `model.anova`.
Every answer is a plain MAP or a frame. `stat.lm` and `fit.logistic` are
unchanged.

## Motivation

- **A frame cannot reach a model except as numbers.** `frame.matrix`
  refuses a factor column, "coding a factor is Phase C's
  `model.matrix`" (RFC-0017 §S5, D256 S6): `[ "x" "g" ] dict
  frame.matrix` on a frame whose `g` is a factor is `FRAME.MATRIX: E101
  names is outside frame.matrix's domain: column "g" is a factor`
  (probed). A categorical predictor has no way into `stat.lm`.
- **`stat.lm` always adds an intercept and takes no weights**
  (`crates/bund2-stdlib/src/biomath/infer.rs`, `stat_lm`: the design is
  `[1 X]`, built by `DMatrix::from_fn` with column 0 all 1.0). A model
  through the origin, a weighted fit, or an offset cannot be written.
- **`fit.logistic` takes 0/1 responses only**
  (`crates/bund2-stdlib/src/biomath/fit.rs`, `fit_logistic`: any `y` not
  0 or 1 is E101), so binomial counts, Poisson counts and rates, and
  positive skewed responses have no model at all.
- **A fit answers its coefficients and nothing after them.** `stat.lm`
  answers `coef`, `se`, `t`, `p`, `r2` and `df`; `fit.logistic` `coef`,
  `se`, `z`, `p`, `deviance` and `aic` (both functions, the `m.insert`
  lines). No residuals, fitted values, leverage or covariance, so no
  diagnostic, prediction or comparison between models can be computed
  from the answer.
- **Research 07 puts all of this in Phase C**, needing only Phase B and
  C3 (`07-statistical-research.md:218`); both are now done (RFC-0017,
  D260).

## Current behaviour

### What the reference does

The reference has no statistical model words; every name this RFC adds
is Bund2's own (D72), and no golden can exercise one (D21). Two of the
reference's behaviours bear on the design:

- **A MATRIX converts to a LIST and nothing else.** Its converter
  answers LIST for a MATRIX and refuses any other target with `Can not
  convert list to <type>` (`reference/rust_dynamic/src/conv.rs:268-291`).
  Bund2 preserves this (`crates/bund2-value/src/lib.rs`,
  `conv_refusal`, the MATRIX arm), and a container asks the same of each
  member and leaves out the ones it names (the same function's doc). So
  `println` of a MATRIX is refused with `Can not convert list to 4`, and
  `println` of a MAP holding a MATRIX prints the MAP without that key:
  `frame.matrix`'s `{ matrix, dropped }` prints as `{ dropped=2 :: }`
  (probed, `bund_examples/frames/` while it was written). §S2 follows
  from this: no answer a program would print holds a MATRIX, except the
  design, whose MATRIX is for computing with.
- **A program's word is found before a native of the same name.** The
  reference calls a lambda when one is registered under the name and an
  inline only otherwise (`reference/rust_multistackvm/src/multistackvm_apply.rs:46-59`);
  Bund2 resolves `lambda` before `native`
  (`crates/bund2-api/src/lib.rs`, `Registry::resolve_uncached`). A
  program that already defines one of this RFC's names keeps its own
  meaning.

### What Bund2 does

- **`stat.lm`** (`infer.rs`, `stat_lm`) reads `X` by `table`, a MATRIX
  of finite numbers by rows (E101 for anything else, E103 for a
  non-number cell or a ragged row), and `y` by `whole`; unequal lengths
  are E102; n ≤ k is E101 (`<n> observations for <k> coefficients`). It
  solves by `nalgebra`'s QR of `[1 X]`, refuses a diagonal of R at or
  below 10⁻¹⁰ of the largest as E106, a constant `y` (`constant`), and a
  perfect fit (`a perfect fit, its residual variance 0`, probed). It
  answers `coef` (intercept first), `se`, `t`, `p` (two-sided, on
  n − k df, by `t_p`), `r2` and `df`.
- **`fit.logistic`** (`fit.rs`, `fit_logistic`) reads `X` and `y` the
  same way, refuses a `y` not 0 or 1 and n ≤ k, and fits by IRLS from
  R's start μ = (y + ½)/2, with `weighted_solve`'s QR of √W·A, until the
  relative deviance change is below 10⁻⁸ (`DEVIANCE_TOL`), at most 100
  iterations (`IRLS_CAP`, E110). η is clamped to ±36.04
  (`ETA_LIMIT`, R's −ln ε); a final η at that bound is E101, "X separates
  y" (probed). It answers `coef`, `se`, `z`, `p` (normal, two-sided),
  `deviance` and `aic` = deviance + 2k.
- **`weighted_solve`** (`fit.rs`) answers (AᵀWA)⁻¹Aᵀ W z and
  (AᵀWA)⁻¹ by one QR, E106 by the same 10⁻¹⁰ rank test.
- **Distributions.** `dist.rs`'s `Family::tails` answers both tails of
  the t, χ² and F families, among others (`Family::T`, `Family::ChiSq`,
  `Family::F` arms); `infer.rs`'s `t_p` and `chisq_p` wrap them.
- **The frame** (RFC-0017): a MAP of `columns`, `data` and `levels`;
  each column's kind read from its non-NONE cells as numeric, logical,
  string or factor, or no kind (`frame/check.rs`, `Kind`). `frame.matrix`
  answers numeric columns as a MATRIX, INTEGERs converted
  (`frame/summary.rs`, `frame_matrix`). The frame words' options reader
  is `frame/opts.rs`, `read`.

### What R does, as reasoning

R 4.4.2's `glm.fit` is the algorithm research 07 names as the source
for `fit.glm` (`07-statistical-research.md:122`, "own IRLS, as
`fit.logistic`"). Read from its source:

- **The iteration.** Each step forms z = (η − offset) + (y − μ)/μ′(η)
  and w = √(prior·μ′(η)²/V(μ)) over the observations with positive
  weight and nonzero μ′, and solves the weighted least squares by QR
  (`src/library/stats/R/glm.R:244-248`). It stops when
  |dev − dev_old|/(0.1 + |dev|) < ε (`:308`), with ε = 10⁻⁸ and at most
  25 iterations by default (`glm.control`, `:135`).
- **Step halving.** A non-finite deviance, or an η or μ outside the
  family's valid range, halves the step towards the previous
  coefficients until it is valid, at most `maxit` times (`:271-303`).
- **Start values** are the family's `mustart`: y + 0.1 for the Poisson
  (`src/library/stats/R/family.R:170`), y for the Gaussian, Gamma and
  inverse Gaussian (`:291`, `:480`, `:535`), and (w·y + ½)/(w + 1) for
  the binomial's one-column response (`:309`), which is
  `fit.logistic`'s (y + ½)/2 at w = 1.
- **Links.** `okLinks` is logit, probit, cloglog, cauchit and log for
  the binomial (`family.R:335`); log, identity and sqrt for the Poisson
  (`:138`); inverse, log and identity for the Gaussian (`:254`) and the
  Gamma (`:448`); inverse, log, identity and 1/μ² for the inverse
  Gaussian (`:510`).
- **The binomial response** is 0/1, or a proportion in [0, 1] whose
  trials are the prior weights, R warning when w·y is more than 10⁻³
  from an integer (`family.R:299-312`).
- **Dispersion** is 1 for the binomial and Poisson, else the Pearson
  statistic Σ wᵢ rᵢ² over the residual df (`glm.R:686-701`, `summary.glm`).
- **AIC** is the family's `aic` plus 2·rank (`glm.R:376-377`): for the
  Gaussian n(ln(2π·dev/n) + 1) + 2 − Σ ln w (`family.R:278-281`); the
  Poisson −2 Σ w ln p(y; μ) (`:165`); the binomial −2 Σ (w/m) ln
  binom(m·y; m, μ) (`:356-360`); the Gamma −2 Σ w ln Γ-density at shape
  1/disp, disp = dev/Σw, plus 2 (`:471-475`); the inverse Gaussian
  Σw(1 + ln(2π·dev/Σw)) + 3 Σ w ln y + 2 (`:529-530`).
- **The null deviance** is at the weighted mean with an intercept, else
  at linkinv(offset) (`glm.R:367-369`); the null df n − 1 with an
  intercept (`:371-372`).
- **Comparison.** `anova` of GLMs uses the larger model's dispersion,
  and by default an F test when the family's dispersion is estimated and
  a χ² (likelihood-ratio) test when it is fixed (`glm.R:536-541`,
  `:647-653`).
- **Diagnostics.** Standardised residuals are r/√(φ(1 − h)) of the
  deviance or Pearson residual (`src/library/stats/R/lm.influence.R:164-173`);
  Cook's distance is (r_P/(1 − h))²·h/(φ·p) for a GLM (`:255-263`) and
  the same with the weighted residual and σ̂ for a linear model
  (`:243-253`).
- **Contrasts.** `contr.treatment` is R's default coding, the first
  level the baseline (`src/library/stats/R/contrast.R:131-134`).
- **R keeps an aliased column** as an `NA` coefficient when the design
  is rank-deficient (`glm.R:153`, `singular.ok = TRUE`); Bund2's model
  words refuse a singular design (E106), and so does this RFC **[K8]**.

## Design

### S1. Shape

- **Bund2's own (D72)**, in the default build, adding no crate:
  `nalgebra`, `statrs` and `libm` are already direct dependencies.
- **Names [K2].** `model.` for a design and for words over a fitted
  model, a new kind of operand (D260 C5); `fit.` for the fitting words,
  beside `fit.logistic`:

| Word | Effect | Answers |
|---|---|---|
| `model.matrix` | `frame response terms opts -- design` | §S2's design |
| `fit.lm` | `design opts -- fit` | weighted least squares (§S3) |
| `fit.glm` | `design opts -- fit` | a GLM by IRLS (§S4) |
| `model.residuals` | `fit opts -- list` | residuals of a type (§S5) |
| `model.influence` | `fit -- map` | leverage, standardised residuals and Cook's distance (§S5) |
| `model.predict` | `fit frame opts -- map` | predictions with standard errors and intervals (§S6) |
| `model.compare` | `small big opts -- map` | an F or likelihood-ratio test between nested fits (§S7) |
| `model.anova` | `design opts -- frame` | the sequential analysis-of-deviance table (§S7) |

- **Additive, strictly [K15].** `stat.lm`, `fit.logistic` and
  `frame.matrix` answer exactly what they answer today. `fit.lm` shares
  `stat.lm`'s computation by factoring it out, as RFC-0017 factored
  `stat.median`'s median (RFC-0017 §S5), so an unweighted `fit.lm` with
  an intercept answers `stat.lm`'s coefficients bit for bit.
  `fit.logistic` is not rewritten on `fit.glm`.
- **Refusals** are RFC-0015 §S11's codes; no new code (as D253 P9). B12
  holds: no word answers NaN or ±∞ from finite operands; a non-finite
  answer from finite operands is E114, an unconverged IRLS E110, a
  singular design E106. Every text is in §S8.
- **Forms** follow RFC-0015 §S3 **[B10]**: each word plain and `.`, the
  `.` form taking from the workbench the operand the plain word has on
  top (the `opts`, or the `fit` for `model.influence`), answering on the
  workbench (`bench!`).
- **Options MAPs** follow RFC-0016 §S1 and are read by the frame words'
  reader (`frame/opts.rs`), extended with this RFC's keys: an unknown
  key, a key the word does not take, or a value of the wrong type or
  range is E101 naming the key. `conf` is a FLOAT in (0, 1), default
  0.95 **[A4]**.
- **Missing values [K14].** `model.matrix` and `model.predict` compute
  from cells, so they take the `na` option, `"refuse"` by default or
  `"omit"` (D252 C2, its third note). Every other word here reads a
  design or a fit, which hold no NONE.
- **Printing.** A fit and every answer over it hold only FLOATs,
  INTEGERs, STRINGs, BOOLs, LISTs and MAPs, so `println` shows them
  whole. A covariance matrix is a LIST of row LISTs. Only the design
  holds a MATRIX, for `linalg.` words and refits; `println` of a design
  leaves the MATRIX out (§Current behaviour).

### S2. The design: `model.matrix` [K3] [K4] [K5] [K6] [K7]

`frame response terms opts model.matrix` reads the frame (RFC-0017's
check), the response column's name, and `terms`, a LIST whose items are
each a STRING, a column, or a LIST of two or more STRINGs, their
interaction (D260 C3).

**Options:** `intercept` (BOOL, default `true`), `weights` and `offset`
(each a column name, default none), `na` (`"refuse"` or `"omit"`).

**Columns, in order [K6].** The intercept, when `intercept` is true;
then each term's columns in the order the terms are given. A term may
not repeat, and an interaction's every column must appear as a term of
its own before it (E101 naming it), so no column is coded except as a
main effect or a product of main effects already present. The order is
the order `model.anova` adds terms in.

**Coding [K5] [K7].**

| Column kind | Columns | Named [K4] |
|---|---|---|
| numeric | one, its cells as FLOATs (B15) | `dose` |
| logical | one, 1.0 for `true`, 0.0 for `false` | `treated.true` |
| factor, levels ℓ₁ … ℓₘ | m − 1, treatment coding: column j is 1.0 where the cell is ℓⱼ₊₁, the first level the baseline (R's `contr.treatment`) | `site.north`, `site.south` |
| factor, no intercept, the first factor term | m: every level its own column, as R codes it | `site.east`, `site.north`, `site.south` |
| string, or no kind | refused: E101 naming the column; `frame.factor` makes a string column a factor | |

An interaction's columns are the products of its members' columns, the
first member varying slowest, named by joining the members' names with
`:` (`dose:site.north`). The intercept's column is all 1.0 and is named
`(intercept)`.

**The response** must be numeric or logical (logical read as 1.0 and
0.0); a factor or string response is E101 **[K7]**. **Weights** must be
finite and positive; **offset** finite (E101 naming the column and row).

**NONE [K14].** Under `"refuse"` a NONE in any column the design reads,
the response, the terms' columns, `weights` and `offset`, is E101
naming the column and row, in that order of columns, each from row 1
(as D259's third choice orders `frame.aggregate`'s). Under `"omit"` each
row with a NONE in any of them is dropped. Zero rows left, or no more
rows than columns, is E101.

**The answer** is a MAP, the design:

| Key | Holds |
|---|---|
| `matrix` | a MATRIX, n rows by k columns, the intercept's first |
| `names` | a LIST of the k column names |
| `assign` | a LIST of k INTEGERs: 0 for the intercept, else the 1-based index of the term the column codes |
| `y` | the response, a LIST of n FLOATs |
| `weights` | n FLOATs, all 1.0 when no `weights` was given |
| `offset` | n FLOATs, all 0.0 when no `offset` was given |
| `dropped` | an INTEGER, the rows dropped under `"omit"`; 0 under `"refuse"` |
| `spec` | a MAP: `response`, `terms` (as given), `intercept`, `levels` (each factor column the design read, to its levels), `logical` (the logical columns read), and `weights` and `offset` when given (their column names) |

`fit.lm`, `fit.glm` and `model.anova` check a design on entry: each key
present with its type, the MATRIX n × k, every LIST of its length, as the
frame words check a frame. A malformed design is E101 naming the key.
`spec` is what `model.predict` needs to code a new frame the same way.

**Caps (D39).** n·k above 10⁸ cells is E111, as `frame.join`'s output
is capped (`frame/summary.rs`, `CAP`).

### S3. `fit.lm`

`design opts fit.lm` fits weighted least squares, minimising
Σ wᵢ (yᵢ − offsetᵢ − xᵢβ)², by the QR of √W·X. `opts` must be a MAP
and takes no key yet, any key being E101; it is there so that a later
key needs no new word, as `fit.glm`'s takes its family **[author]**.
n ≤ k is E101; a singular design E106 by `stat.lm`'s rank test **[K8]**;
a constant `y` and a perfect fit E101, as `stat.lm` refuses them.

The answer, a fit, is a MAP:

| Key | Holds |
|---|---|
| `coef`, `se`, `t`, `p` | per column, in `names` order; t on n − k df |
| `names` | the design's `names` |
| `vcov` | σ̂²(XᵀWX)⁻¹, a LIST of k LISTs |
| `sigma` | σ̂, √(RSS/(n − k)), RSS the weighted residual sum |
| `deviance` | RSS |
| `df.residual` | n − k, an INTEGER |
| `r2`, `adj.r2` | about the weighted mean with an intercept, about 0 without (R's `summary.lm` rule) |
| `f`, `f.df`, `f.p` | the overall F test against the intercept-only model (against 0 without an intercept); `f.df` a LIST of two INTEGERs |
| `aic` | the Gaussian AIC, §Current behaviour's R formula, plus 2(k + 1) |
| `fitted`, `y`, `weights`, `offset`, `hat` | n FLOATs each: μ̂, the response, the prior weights, the offset, and the leverages hᵢ, the diagonal of √W X (XᵀWX)⁻¹ Xᵀ √W |
| `family`, `link` | `"gaussian"`, `"identity"` |
| `dispersion` | σ̂² |
| `spec` | the design's `spec` |

With all weights 1.0, no offset and an intercept, `coef`, `se`, `t`,
`p`, `r2` and `df.residual` equal `stat.lm`'s `coef`, `se`, `t`, `p`,
`r2` and `df` on the design's MATRIX without its first column, bit for
bit **[K15]**.

### S4. `fit.glm` [K9] [K10] [K11] [K12]

`design opts fit.glm` fits a generalised linear model by IRLS, as R's
`glm.fit` does (§Current behaviour).

**Options:** `family` (`"binomial"`, `"poisson"`, `"gaussian"`,
`"gamma"`, `"inverse.gaussian"`; required), `link` (default the
family's canonical link: logit, log, identity, inverse, 1/mu^2). The
links are R's `okLinks` for each family **[K9]**:

| Family | Variance V(μ) | Links | Valid μ | Response |
|---|---|---|---|---|
| `binomial` | μ(1 − μ) | `logit`, `probit`, `cloglog`, `cauchit`, `log` | 0 < μ < 1 | in [0, 1], w·y an integer within 10⁻³ **[K10] [K12]** |
| `poisson` | μ | `log`, `identity`, `sqrt` | μ > 0 | ≥ 0 |
| `gaussian` | 1 | `identity`, `log`, `inverse` | any | any; > 0 under `log`, ≠ 0 under `inverse` (R's `initialize`) |
| `gamma` | μ² | `inverse`, `log`, `identity` | μ > 0 | > 0 |
| `inverse.gaussian` | μ³ | `1/mu^2`, `inverse`, `log`, `identity` | μ > 0 | > 0 |

A response outside its family's range is E101 naming the row.

**The iteration [K11].** Start from the family's `mustart` (R's, above)
and η = g(μ_start); each step as R's (z, w, a QR solve by
`weighted_solve`), stopping when |dev − dev_old|/(0.1 + |dev|) < 10⁻⁸.
A non-finite deviance, or an η or μ outside the family's valid range,
halves the step towards the previous coefficients, at most 100 times,
else E110 naming `IRLS step halving`. At most 100 iterations
(`fit.logistic`'s cap, not R's 25), else E110. The binomial's η is
clamped as `fit.logistic` clamps it, and a fitted μ numerically 0 or 1
at convergence is E101, "X separates y", as `fit.logistic` refuses
it. A singular design is E106 **[K8]**.

**The answer** is a fit, §S3's keys where they apply:

| Key | Holds |
|---|---|
| `coef`, `se`, `p` | per column; se from φ·(XᵀWX)⁻¹ at the estimate |
| `t` or `z` | coef/se: `z`, p normal, when φ is fixed (binomial, Poisson); `t`, p on the residual df, when φ is estimated, as `stat.lm` and `fit.logistic` name them |
| `names`, `vcov`, `spec`, `y`, `weights`, `offset` | as §S3 |
| `fitted` | μ̂ |
| `linear` | η̂ |
| `hat` | the leverages at the estimate, from √W X |
| `working.weights` | W at the estimate |
| `dispersion` | 1 for the binomial and Poisson; else Pearson's Σ wᵢ(yᵢ − μᵢ)²/V(μᵢ) over n − k |
| `deviance`, `null.deviance` | the family's deviance at μ̂, and at the null model (R's rule) |
| `df.residual`, `df.null` | n − k and n − 1 (n without an intercept), INTEGERs |
| `aic` | the family's AIC, R's formula, plus 2k |
| `iterations` | an INTEGER |
| `family`, `link` | the STRINGs used |

For the binomial under `logit` on a 0/1 response with unit weights, the
coefficients, standard errors, deviance and AIC agree with
`fit.logistic`'s to a relative 10⁻¹² (criterion 3); they are not bit for
bit, since `fit.glm` forms W as μ′²/V where `fit.logistic` forms
μ(1 − μ) directly **[K15]**.

### S5. Residuals and influence

`fit opts model.residuals` answers a LIST of n FLOATs. `opts`: `type`,
one of `"response"` (y − μ), `"pearson"` ((y − μ)√w/√V(μ)), `"deviance"`
(sign(y − μ)√dᵢ, dᵢ the observation's deviance contribution),
`"working"` ((y − μ)/μ′(η)), `"standardized"` (the deviance residual
over √(φ(1 − hᵢ)), R's `rstandard.glm`). The default is `"deviance"`,
R's for a GLM, which for an unweighted `fit.lm` is y − μ **[author]**.

`fit model.influence` answers a MAP of three LISTs of n FLOATs: `hat`,
`std.residuals` (the deviance residual standardised as above), and
`cooks` (r_P/(1 − h))²·h/(φ·k), R's `cooks.distance.glm`, which is R's
`cooks.distance.lm` for `fit.lm`. An hᵢ of 1, which makes either
infinite, is E114 naming the row **[author]**, where R answers NaN.

Both read only the fit; neither needs the design.

### S6. `model.predict` [K16]

`fit frame opts model.predict` codes `frame` by the fit's `spec`, as
`model.matrix` coded the design: the same columns, a factor cell
checked against the spec's levels (a level the fit never saw is E101
naming it), the response not read, `offset`'s column read when the spec
names one. `opts`: `type` (`"link"` or `"response"`, default
`"response"`), `interval` (`"none"`, `"confidence"` or `"prediction"`,
default `"none"`), `conf`, `na`.

The answer is a MAP: `fit`, a LIST of predictions; `se`, their standard
errors, √(xᵀ V x) with V the fit's `vcov`, on the link scale, and on the
response scale multiplied by μ′(η) (the delta method, R's
`predict.glm`); and, with an interval, `lower` and `upper`. A confidence
interval is η̂ ± q·se on the link scale, q normal when φ is fixed and t
on the residual df when it is estimated, mapped through the inverse link
for `"response"` **[author]**. A prediction interval adds σ̂² to the
variance and is for `fit.lm`, and `fit.glm` under `gaussian` and
`identity`, only; elsewhere E101. Under `"omit"`, `dropped` counts the
rows left out.

### S7. Comparing models

`small big opts model.compare` tests whether `big` improves on `small`.
They must be nested: the same family, link, `y`, `weights` and
`offset`, every `small` name among `big`'s, and fewer columns in `small`
(E101 naming the first disagreement) **[author]**. `opts`: `test`, `"F"`
or `"LR"`; the default is R 4.4's: F when `big`'s dispersion is
estimated, LR when it is fixed **[K13]**. The answer is a MAP:
`df` (the difference in residual df, an INTEGER), `deviance` (the
difference in deviance), `statistic`, `p`, `test`; for F also `df2`. F
is (Δdev/Δdf)/φ_big on Δdf and `big`'s residual df; LR is Δdev/φ_big on
χ²(Δdf), φ_big 1 for the binomial and Poisson.

`design opts model.anova` fits the intercept-only model and then adds
the design's terms one at a time, in order, by `assign`, with
`fit.glm`'s options (`family`, `link`) and `test`; for a design meant
for `fit.lm`, `family` `"gaussian"`. It answers a frame (RFC-0017's
shape), one row per model: `term` (a string column: `(intercept)` or
`NULL` first, then each term's name, an interaction's joined by `:`),
`df`, `deviance`, `resid.df`, `resid.deviance`, `statistic` and `p`, the
first row's last two NONE. Each row's test uses the full model's
dispersion, as R's `anova.glm` does. This is R's type I table; types II
and III are not in this RFC **[K1]**.

### S8. Refusals

Every refusal is E101 unless the row says otherwise; `<w>` is the word.
Values from the program are rendered as RFC-0017's are (`render.rs`):
escaped, bounded, quoted.

| Case | Code | Text |
|---|---|---|
| `response` not a STRING, or not a column | E101 | `response is outside <w>'s domain: <summary>` / `no column "y"` |
| the response a factor or string column | E101 | `response is outside <w>'s domain: column "y" is a factor` / `is string` |
| `terms` not a LIST, an item neither a STRING nor a LIST of two or more STRINGs | E101 | `terms is outside <w>'s domain: item 2 is <summary>` |
| a term naming no column, repeated, or an interaction before its main effect | E101 | `terms is outside <w>'s domain: no column "zz"` / `term 3 repeats term 1` / `term 3 needs "dose" as a term before it` |
| a string or no-kind column in a term | E101 | `terms is outside <w>'s domain: column "g" is string` |
| `weights` or `offset` naming no column, not numeric, or a cell not finite and positive (`weights`) | E101 | `opts is outside <w>'s domain: weights row 4 is 0` |
| NONE under `"refuse"` | E101 | `frame is outside <w>'s domain: column "x" row 2 is none (na "refuse")` |
| no rows, or n ≤ k | E101 | `frame is outside <w>'s domain: no rows` / `5 observations for 6 coefficients` |
| n·k above 10⁸ | E111 | RFC-0015's text, `<what>` being `the design's cells` |
| a malformed design | E101 | `design is outside <w>'s domain: <key> <what is wrong>` |
| a response outside its family's range | E101 | `design is outside <w>'s domain: y row 3 is -1, below the poisson family's 0` |
| a non-integer binomial count | E101 | `design is outside <w>'s domain: y row 3 times its weight is 2.5, not a count` |
| an unknown family, or a link the family does not take | E101 | `opts is outside <w>'s domain: link is "sqrt", not a binomial link` |
| a singular design | E106 | RFC-0015's text |
| IRLS or step halving past 100 | E110 | RFC-0015's text, method `IRLS` / `IRLS step halving` |
| separation | E101 | `design is outside <w>'s domain: fitted probabilities numerically 0 or 1: X separates y` |
| a perfect fit (`fit.lm`) | E101 | `design is outside <w>'s domain: a perfect fit, its residual variance 0` |
| an unknown residual type | E101 | `opts is outside <w>'s domain: type is "raw"` |
| a leverage of 1 | E114 | RFC-0015's text, `<expression>` naming the row |
| a level the fit never saw (`model.predict`) | E101 | `frame is outside <w>'s domain: column "site" row 2 is "west", not one of the fit's levels` |
| a prediction interval outside the Gaussian identity model | E101 | `opts is outside <w>'s domain: interval "prediction" needs a gaussian identity fit` |
| fits not nested | E101 | `small is outside <w>'s domain: <what differs>` |
| a malformed fit | E101 | `fit is outside <w>'s domain: <key> <what is wrong>` |
| a non-finite answer from finite operands | E114 | RFC-0015's text |

### S9. Tiers, audits and generated files

As RFC-0017 §S7: effects as §S1 declares, fixed for every word, none
calling a lambda; `PROMOTABLE.txt` regenerated; `UNVARIED`
(`crates/bund2-stdlib/src/lib.rs`, the audit array, 75 today) gains
exactly the natives the audit's own rule selects, a plain word of three
or more operands or a `.` form of two or more on the stack (the same
file, `varied_of`): `model.matrix` and
`model.matrix.` (four and three operands), and `model.predict`,
`model.predict.`, `model.compare` and `model.compare.` (three and
two); each with the operands the palette leaves at 7. A counting test
per module, as RFC-0016's and RFC-0017's modules have.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| `stat.lm`'s answers, refusals and texts | **Preserved exactly.** Its computation is factored into a crate-internal function that it and `fit.lm` call **[K15]**; its tests pass unchanged, and criterion 2 adds a bit-for-bit check against `fit.lm` |
| `fit.logistic`'s answers, refusals and texts | **Preserved exactly**; its code is not changed. `weighted_solve` is shared, unchanged |
| `frame.matrix`, and every RFC-0017 word | **Preserved exactly**; `frame/opts.rs` gains keys, and the keys the frame words take are unchanged |
| The options reader's texts for existing keys | **Preserved exactly** |
| `dist.rs`'s tails | **Preserved exactly**; read only |
| A program's own word of one of these names | **Preserved**: a program's word is found first (§Current behaviour) |
| `println` of a MATRIX, and of a MAP holding one | **Preserved exactly**: a design printed omits its `matrix` (§S1); no other answer holds a MATRIX |
| `bund2 words` and `bund2 effects` | **Changed by design**: the new names, forms and effects appear |
| `PROMOTABLE.txt` | **Regenerated**; no existing name line moves; its count line changes |
| `UNVARIED` in `lib.rs`'s tests | **Changed by design**: §S9's six names join, 75 to 81 |
| conform, and the goldens | **Unchanged**: 143/159, CEILING 143/159; no golden runs a new word (D21) |
| coverage | **Unchanged**: 577/602, IMPLEMENTED 597/602; the new words are Bund2's own, outside the reference's registry, so neither number counts them |
| `--noio` (D119) | **Preserved exactly**: no word touches the file system |

## Alternatives considered

- **A formula STRING** (`"y ~ dose * site"`). Ruled out by D260 C3 for
  now; it may come later, parsed into §S2's LIST of terms.
- **Fit words that take the frame directly** (`frame response terms
  opts fit.glm`), with no design value **[K3] B**. Shorter, but the
  design could not be inspected, given to `linalg.` words, or reused
  across fits, and `model.anova`'s refits would have to re-code the frame
  each time.
- **R's pivoting of an aliased column** to an `NA` coefficient **[K8]
  B**. It hides a mistake in the design behind a NONE in the answer;
  `stat.lm` and `fit.logistic` refuse a singular design, and so does
  this RFC.
- **`fit.logistic` rewritten as `fit.glm`'s binomial case** **[K15] B**.
  It would change `fit.logistic`'s last bits; nothing gains from it.
- **R's 25-iteration cap** **[K11] B**. `fit.logistic` already caps at
  100, and one cap across the family is simpler to explain.
- **A warning, as R gives, for a non-integer binomial count** **[K12]
  B**. A warning is a `Diagnostic` of its own, once per word, and B12's
  spirit is refusal; but R's fit runs, and a program porting from R
  would meet a refusal R never gave.
- **Every column kind accepted** (a string column coded as a factor of
  its sorted values, as R's `model.frame` does) **[K7] B**. One word,
  `frame.factor`, already does it, and an explicit factor is a choice of
  levels a model's baseline depends on.

## Acceptance criteria

Reference values are computed from the definitions (D260 C4): a GLM's
estimate is the unique solution of its score equations, solved by
Newton's method in mpmath at 50 digits, and every quantity after it by
its formula. The script and its output are new files beside RFC-0015's
fixtures, written as `crates/bund2-stdlib/src/biomath/fixtures/fit.py`
is written. No answer in this RFC
depends on an algorithm's choices, so no R output is needed. Tolerances
are relative 10⁻⁹ for coefficients and their functions, absolute
10⁻¹² for p below 10⁻⁶.

1. **Design.** On the frame of `bund_examples/frames/` with `site` a
   factor and a logical column added: `"response" [ "dose" "site" [
   "dose" "site" ] ] dict model.matrix` answers the stated 7 × 6 MATRIX
   (after `"na" "omit"`), `names` `[ (intercept) dose site.north
   site.south dose:site.north dose:site.south ]`, `assign`
   `[ 0 1 2 2 3 3 ]`, `dropped` 2; with `intercept` false, `site`
   alone codes three columns; a logical column codes one. Each row of
   §S8 for `model.matrix` is produced by a stated input with its exact
   text.
2. **`fit.lm`.** Bit for bit with `stat.lm` as §S3 states, on
   `stat.lm`'s own test data and on criterion 1's design without
   factors. Weighted and offset fits, and `r2`, `adj.r2`, `f`, `aic`,
   `vcov` and `hat`, against the fixture.
3. **`fit.glm`.** Against the fixture: the Poisson model of Dobson's
   randomised trial, R's own example
   (`src/library/stats/man/glm.Rd:335-340`: counts 18, 17, 15, 20, 10,
   20, 25, 13, 12, outcome and treatment factors of three levels), with
   its deviance, null deviance, AIC and dispersion; a Poisson rate model
   with a log offset; the binomial with proportions and trials as
   weights, under logit, probit and cloglog; a Gamma under inverse and
   log; an inverse Gaussian under 1/mu^2; a Gaussian under log. The
   binomial logit on 0/1 data agrees with `fit.logistic` as §S4 states.
   Step halving is exercised by a stated Gamma model under `identity`
   whose first step leaves the valid range. Separation, a singular
   design and each §S8 row for `fit.glm` are produced with their texts.
4. **Residuals and influence.** Every `type`, `hat`, `std.residuals` and
   `cooks`, for criterion 3's Poisson and Gamma fits and criterion 2's
   weighted fit, against the fixture.
5. **Prediction.** For the Poisson and the Gaussian fits: `fit` and `se`
   on both scales and confidence intervals, and for the Gaussian
   prediction intervals, against the fixture; an unseen level and a
   prediction interval on the Poisson fit refused with their texts.
6. **Comparison.** `model.compare` of the Dobson model without and with
   `treatment` gives the LR test by default, and of two Gamma fits the F
   test by default, each against the fixture; `model.anova` of the
   Dobson design answers the stated frame. Fits that are not nested are
   refused with each text.
7. **Example.** `bund_examples/models/` fits, checks, predicts and
   compares a model on a frame, prints `expected.txt`, and a test in
   `crates/bund2-cli/tests` compares the two, as `bund_examples/frames/`
   does.
8. **No abort.** Every name and form, on a palette of NaN, ±∞, wrong
   types, empty, ragged, NONE-filled and malformed frames, designs and
   fits, answers or refuses with an E1nn, a shallow-stack error or a
   bridge error.
9. **Health.** `cargo xtask conform` prints 143/159 and CEILING 143/159;
   `coverage` IMPLEMENTED 597/602 and COVERAGE 577/602; clippy and
   `cargo deny` are clean; `PROMOTABLE.txt` is regenerated with no
   existing name line moved; the `UNVARIED` audit passes with §S9's
   names; `fit.rs`'s and `infer.rs`'s tests pass unchanged.

## Implementation plan

| M | Delivers |
|---|---|
| M1 | §S2: `model.matrix`, the design and its check, the options reader's keys; criterion 1 |
| M2 | §S3: `fit.lm`, with `stat.lm`'s computation factored out; criterion 2 |
| M3 | §S4: `fit.glm`, its families, links and IRLS; criterion 3 |
| M4 | §S5 and §S6: `model.residuals`, `model.influence`, `model.predict`; criteria 4 and 5 |
| M5 | §S7: `model.compare`, `model.anova`; criterion 6 |
| M6 | the example; criterion 7 |

Each ends with criteria 8 and 9. The fixture script is written in M2 and
grows with each milestone.

## Open questions

None of these is ruled; the RFC is written against each recommended
option. Each is for the owner, and none blocks drafting.

| | Question | Options | Recommended |
|---|---|---|---|
| **K1** | Scope | **A** research 07's Phase C row: design, `fit.lm`, `fit.glm` (five families), residuals, influence, prediction, F and LR tests, the type I `anova`. **B** A, plus robust (HC0–HC3) standard errors and variance inflation, both in §3.3's table. **C** design and `fit.glm` only, the rest to another RFC | **A**: the phase as research 07 drew it; HC errors and VIF (with GVIF for multi-column terms) are self-contained additions for a later RFC. On acceptance ERRATA records that §3.3's table is narrowed |
| **K2** | Names | **A** `model.matrix`, `fit.lm`, `fit.glm`, `model.residuals`, `.influence`, `.predict`, `.compare`, `.anova`. **B** everything under `fit.`. **C** `stat.anova`, `stat.predict` and the like under `stat.` | **A**: D260 C5's rule, a new prefix for a new kind of operand (a design, a fit), `fit.` beside `fit.logistic` |
| **K3** | How a frame reaches a fit | **A** a design MAP made once by `model.matrix`, which the fit words take. **B** the fit words take `frame response terms` themselves | **A**: inspectable, reusable, and the shape `model.anova`'s refits need |
| **K4** | Coefficient names | **A** R's: `(Intercept)`, `sitesouth`, `dose:sitesouth`, `treatedTRUE`. **B** `(intercept)`, `site.south`, `dose:site.south`, `treated.true` | **B**: RFC-0017's `<column>.<statistic>` style, and a level stays readable when it begins with a capital or a digit |
| **K5** | Factor coding | **A** treatment contrasts, the first level the baseline, no option. **B** A, with a `contrasts` option for sum and Helmert coding | **A**: R's default; the baseline is chosen by the levels' order, which `frame.factor.levels` sets |
| **K6** | Terms and interactions | **A** terms in the order given; an interaction needs each member as an earlier term. **B** R's marginality rules, coding an interaction without its main effects by R's algorithm | **A**: every design B allows that A refuses is unusual, and A's coding is the one a reader expects |
| **K7** | Column kinds | **A** numeric and logical coded, a factor by contrasts, a string column refused. **B** a string column coded as a factor of its sorted values | **A**: the baseline is a choice the program should make with `frame.factor` |
| **K8** | A rank-deficient design | **A** E106, as `stat.lm`. **B** R's: the aliased column's coefficient NONE | **A** |
| **K9** | Families and links | **A** the five families with R's `okLinks` (cauchit included). **B** canonical links only. **C** A plus the quasi families and the negative binomial | **A**: R's set; quasi families and the negative binomial (§3.3's second row) need their own estimation of φ or θ |
| **K10** | A binomial response | **A** 0/1, or a proportion with the trials as `weights` (R's one-column form). **B** A, plus a `trials` option naming a column of counts | **A**: one form to learn; a program makes the proportion with `frame.with` |
| **K11** | IRLS control | **A** R's iteration and step halving, ε 10⁻⁸, at most 100 iterations. **B** R's 25 | **A**: `fit.logistic`'s cap |
| **K12** | A non-integer binomial count | **A** E101 when w·y is more than 10⁻³ from an integer (R's threshold). **B** a warning, as R | **A**: B12's refusal |
| **K13** | `model.compare`'s and `model.anova`'s default test | **A** R 4.4's: F when the dispersion is estimated, LR when fixed. **B** always LR | **A** |
| **K14** | NONE | **A** `model.matrix` and `model.predict` take `na`; nothing else sees NONE. **B** the fit words take `na` too | **A**: D252's rule, where the cells are read |
| **K15** | `stat.lm` and `fit.logistic` | **A** both unchanged; `fit.lm` shares `stat.lm`'s computation, bit for bit; `fit.glm` agrees with `fit.logistic` to 10⁻¹². **B** `fit.logistic` rewritten on `fit.glm` | **A** |
| **K16** | Intervals from `model.predict` | **A** confidence intervals on the link scale mapped through the inverse link; prediction intervals for Gaussian identity fits only. **B** no intervals, `se` only, as R's `predict.glm` | **A**: an interval is what a program wants from a prediction |

Choices marked **[author]** where they stand, for the owner to see:

- the default residual type `"deviance"` (§S5);
- a leverage of 1 refused, where R answers NaN (§S5);
- nesting checked by family, link, `y`, `weights`, `offset` and names
  (§S7);
- `model.anova`'s first row `(intercept)`, or `NULL` without one, and
  its test columns NONE there (§S7);
- weights strictly positive, where R drops zero-weight rows (§S2);
- a design's NONE checked column by column in the order response,
  terms, `weights`, `offset` (§S2);
- `fit.lm`'s `opts`, taking no key yet (§S3).

None is open in `docs/registers/open-questions.md`: R's behaviour is
read from its source, and every Bund2 behaviour cited is read or probed.
