# RFC-0018: Models: design matrices, generalised linear models, diagnostics and comparison

- Status: **Draft** (2026-11-19). Drafted on the owner's `/rfc-draft 0018`,
  after RFC-0017 (research 07's Phase B) was built and D260 ruled
  research 07's C3–C8.
- Scope: research 07's Phase C (`docs/research/07-statistical-research.md:218`;
  §3.2 at `:109-116`, §3.3 at `:118-127`): `model.matrix`, `fit.glm` with
  weights and offsets, residuals, diagnostics, prediction, F and
  likelihood-ratio tests, and `anova`.
- Depends on: RFC-0015 (the `biomath` conventions: §S3's forms, §S11's
  refusals E101–E115, the operand readers, `stat.lm` and `fit.logistic`),
  RFC-0016 (§S1's options-MAP rules), RFC-0017 (the frame, its check, its
  kinds and levels, NONE and the `na` option), RFC-0004 (effects)
- Decisions consumed: **D265** (the sixth revision's restated [author] choice, as written), **D264** (the fifth revision's [author] choices, as written), **D263** (the fourth revision's [author] choices, as written), **D262** (the [author] choices, as written), **D261** (K1–K16, as recommended), **D260** (C3 A, C4 D, C5 C, C6 C, C7 B, C8 A),
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
option. All sixteen were ruled as recommended on 2026-11-20 (D261), so
the marks now show where each ruling applies (§Open questions).

**Revised, 2026-11-19, before review** (the owner's review of K1–K16).
- §S2 drops a factor's unused levels, as R's `lm` and `glm` do, and
  refuses a factor term left with one level.
- §S2 refuses two columns with one name.
- Under K6, an interaction needs only its members as earlier terms.
- E106 names the first dependent column (K8).
- `bic` is added to every fit (K1).
- Criterion 3's tolerance for IRLS estimates is now 10⁻⁶. At 10⁻⁹,
  R's stopping rule could fail correct code (K11).
- `model.predict` takes `weights` for a prediction interval (K16).
- The reasons given for K1, K4, K12 and the zero-weight choice are
  restated.

**Revised, 2026-11-20** (`docs/rfc/reviews/RFC-0018-review-2026-11-20.md`).
- **B1.** `fit.lm`'s `aic` is −2 log L + 2(k + 1), equal to `fit.glm`'s
  Gaussian AIC. The previous text was 2 too high.
- **B2.** The null deviance with an offset and an intercept comes from
  the refitted intercept-plus-offset model, as `glm()` computes it.
- **B3.** A first IRLS step out of range is refused, as R stops there.
  Halving applies from the second step.
- **B4.** A design of no columns is refused, and `model.anova` computes
  its no-intercept null row itself. A new solve function answers the
  dependent column, and `weighted_solve` is unchanged.
- **SF1.** `se` uses |μ′|, and `lower` and `upper` are ordered.
- **SF2.** r² is 1 − RSS/TSS about the mean of y − offset, which is
  bit-for-bit compatible with `stat.lm`; `df` is compared by value.
- **SF3.** The model words use an options reader of their own, and
  `frame/opts.rs` is unchanged.
- **SF4.** Each link gets R's clamp, and there is one separation test
  per link.
- **SF5.** The binomial logit takes `fit.logistic`'s arithmetic.
- **SF6.** Poisson rates numerically 0 are refused.
- **SF7.** A constant `y` is refused only with an intercept.
- **Notes and assumptions.** N1–N7 are applied; U1–U8 are stated in
  §S1, §S2, §S4, §S7 and §S9.

**Revised, 2026-11-20, second review**
(`docs/rfc/reviews/RFC-0018-review-2026-11-20-2.md`).
- **SF1.** The logit's clamp is described as `fit.logistic`'s, and R's
  piecewise rule is cited beside it (`family.c`). The separation
  thresholds are stated for each link.
- **SF2.** Each family's valid μ cites R's `validmu`. The inverse
  Gaussian's μ > 0 is marked **[author]**, since R accepts any μ.
- **SF3.** A fit answers `halvings`, and criterion 3 states its halving
  model.
- **SF4.** Criterion 9 snapshots `stat.lm`'s and `fit.logistic`'s bits
  before M2.
- **N1–N4.** `lm.R:673-674`; R's start-value check has no case here;
  E114's expression for a leverage of 1; what a failure at 10⁻⁶ means.
- **U1–U6** of that review are not yet stated in the text.
- **K1–K16** were ruled as recommended (D261).

**Revised, 2026-11-20, third review**
(`docs/rfc/reviews/RFC-0018-review-2026-11-20-3.md`).
- **B1.** IRLS is Newton's method only under a canonical link.
  Elsewhere it converges linearly, and R's rule can stop it some 10⁻⁵
  from the solution of the score equations. The fixture therefore runs
  §S4's own iteration at 50 digits, stopped by the same rule. Criterion
  3 compares against where that iteration stops, and `iterations` and
  `halvings` exactly. The "quadratic" sentences are corrected, and D261
  carries a dated note.
- **B2.** Every tolerance has an absolute floor of 10⁻¹⁰, because
  Dobson's treatment effect is exactly 0. A Δdeviance below 0 is taken
  as 0, as R's `anova.glm` takes it (§S7).
- **SF1.** An intercept-only `fit.lm` answers r² 0 and no F, as R does.
- **SF2.** A null deviance that is not finite is left out of the fit,
  and `model.anova` refuses it with E114.
- **SF3.** Each link's valid η is stated. Halving tests η and μ both.
- **SF4.** Inside an interaction, a factor is coded by its contrast
  columns.
- **SF5.** The bit snapshot records every element of a LIST.
- **SF6.** The Poisson refusal's rationale is corrected: it is R's
  warning made a refusal. A group of zero counts is answered.
- **SF7.** R's logit warning comes from |η| > 30.
- **SF8.** Under the binomial, a weight is refused more than 10⁻³ from
  an integer.
- **SF9.** A non-integer Poisson `y` is fitted, and `aic` and `bic` are
  left out **[author]**.
- **SF10.** A leverage within 10⁻¹⁰ of 1 is refused by both words that
  divide by 1 − h.
- **SF11.** `model.anova`'s first row has `df`, `deviance`, `statistic`
  and `p` NONE. Criterion 6 states the Dobson frame.
- **N1–N10** are applied.
- **U1–U10** of that review, and **U1–U6** of the second, are stated
  in §S1–§S7.
- **The [author] choices**, all twenty-three, were ruled as written
  (D262).

**Revised, 2026-11-20, fourth review**
(`docs/rfc/reviews/RFC-0018-review-2026-11-20-4.md`).
- **B1.** `model.predict` codes new rows by the spec's columns and
  levels. Of §S2's refusals it applies the frame check, the kinds, NONE
  and the cells, and from step 7 only "no rows left" and the cap. A
  one-row prediction joins criterion 5.
- **SF1.** `fit.lm` and `fit.glm` join `fit.rs`'s `WORDS`, so
  `fit/tests.rs`'s registration test counts them unchanged.
- **SF2.** A logical column is one column, where R codes a logical as a
  two-level factor. The difference without an intercept is stated
  (`models.R:652`, `model.c`).
- **SF3.** Contrast coding inside an interaction matches R's only where
  the lower-order margins are earlier terms. The three-way case is
  stated (`model.c`, `TermCode`).
- **SF4.** `model.anova`'s F takes the full model's residual df, and F
  under a fixed dispersion is refused. Its Gaussian identity rows'
  refusals are stated.
- **SF5.** On the `fit.logistic` path, `aic` is deviance + 2k.
- **N1–N10 and U1–U7** are applied. Six new **[author]** choices are
  listed apart (§Open questions), and were ruled as written (D263).

**Revised, 2026-11-20, fifth review**
(`docs/rfc/reviews/RFC-0018-review-2026-11-20-5.md`).
- **B1.** `model.predict`'s response scale: a prediction or interval end
  outside the link's valid η is refused, `type` `"link"` answering it;
  an infinite end leaves its key out, as D251's A3 rules; an end
  outside the family's μ is answered. Criterion 5 gains the zero-group
  and `inverse` cases.
- **SF1.** An interaction's members are distinct, and two interactions
  of the same members are one term (`model.c`, `OrBits`, `TrimRepeats`).
- **SF2.** The design check enforces the invariants the words rely on:
  `spec`'s shape, the intercept from `spec.intercept`, `assign`, finite
  values and n > k.
- **SF3.** An interaction's columns vary the first member fastest, as
  R's do. `model.predict`'s default `type` stays `"response"`, marked as
  a difference from R.
- **N1–N11 and U1–U5** are applied. Seven new **[author]** choices are
  listed apart (§Open questions), and were ruled as written (D264).

**Revised, 2026-11-20, sixth review**
(`docs/rfc/reviews/RFC-0018-review-2026-11-20-6.md`), its should-fix
items only.
- **SF1.** An interaction's members are taken in R's variable order,
  the order their columns first appear in `terms` (`model.c:701`,
  `model.c:1091-1112`), not the order the interaction writes them; names
  follow it (`model.c:2053-2075`). `addfactor` (`model.c:263-278`) places the columns
  and does not choose the order, which the fifth revision's text and
  D264's premise had read it as doing. Criterion 1 codes both orders.
- **SF2.** The design check asks every `offset` 0.0 without
  `spec.offset`, and every weight 1.0 without `spec.weights`.
- **SF3.** `model.predict` maps through §S4's clamped inverse link and
  μ′, as R's `predict.glm` does (`predict.glm.R:60-61`); a non-finite
  `fit` or `se` is E114. Criterion 5's zero-group `lower` is ε.
- **SF4.** The fit check asks lengths, and `model.predict` refuses a fit
  whose `spec` codes other columns than its `names`. §S8 and criterion
  8 gain the rows.
- One **[author]** choice is restated (§Open questions), and was ruled
  as written (D265).
- **N1–N8 and U1–U3**, applied after D265: `vcov` computed newly beside
  `stat.lm`'s `se`; the Gaussian null-deviance exception under
  `identity` only; "members distinct" in step 3; `model.predict`'s
  missing-column text and the columns it reads; three texts for the η
  refusal; the prediction-interval paragraph out of the bullet; the
  keys left out named beside B12 in §S1 and §S8; `assign` and `names`
  bounded; the other words' order of refusals; no suffix on
  `model.anova`'s full-model refusal; the cost of leaving a key out.
  N9 is the register's, and is a dated note on D264.

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
  Bund2 resolves `lambda` before `native` for a plain call
  (`crates/bund2-api/src/lib.rs`, `Registry::resolve_uncached`; a
  registered command resolves first, and a sigiled call skips the
  lambda). A program that already defines one of this RFC's names keeps
  its own meaning.

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
  iterations (`IRLS_CAP`, E110). η is clamped continuously to ±36.04
  (`ETA_LIMIT`, −ln ε); a final η at that bound is E101, "X separates
  y" (probed). R's logit, which is in C, is not that clamp. It computes
  the logistic exactly up to |η| = 30, and beyond 30 sets μ to
  ε/(1 + ε) or 1/(1 + ε) and μ′ to ε
  (`src/library/stats/src/family.c:31-33`, `:83-84`, `:103`). The value
  it saturates at is the logistic of ±36.04, which is where `fit.rs`'s
  "R's" description of `ETA_LIMIT` comes from. It answers `coef`, `se`, `z`, `p` (normal, two-sided),
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
  The range test is `valideta(eta) && validmu(mu)` (`:289`). `valideta`
  is the link's: η > 0 under `sqrt` and `1/mu^2`, η ≠ 0 under `inverse`
  (`family.R:109`, `:115`, `:121`), and any finite η elsewhere. Each
  of the two halving loops counts from 1 again for each step
  (`glm.R:274-277`, `:293-296`).
- **The iteration's rate.** Under a canonical link, IRLS is Newton's
  method and converges quadratically. Under any other link it is Fisher
  scoring and converges linearly. The rule stops on the deviance, which
  moves with the square of the coefficients' error near the optimum.
  So a non-canonical fit can stop with the coefficients some 10⁻⁵ from
  the solution of the score equations: 5.95·10⁻⁶ relative for
  criterion 3's halving example, in a reimplementation of R's loop.
- **A non-integer count.** R's Poisson AIC is −2 Σ w ln `dpois`(y; μ)
  (`family.R:165`). `dpois` answers its zero, so −∞ as a log, for an
  x more than 10⁻⁹·max(1, |x|) from an integer, with a warning
  (`src/nmath/dpois.c:79`; `R_D_nonint_check`, `src/nmath/dpq.h:124-128`;
  `R_nonint`, `src/nmath/nmath.h:63`). So R's Poisson AIC on such a
  response is +∞.
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
- **The null deviance.** `glm.fit` computes it at the weighted mean
  with an intercept, else at linkinv(offset) (`glm.R:367-369`). With an
  offset and an intercept, `glm()` replaces that value. It refits the
  intercept-plus-offset model from the full fit's μ and takes that
  fit's deviance, and if the refit does not converge it only warns
  (`glm.R:99-119`; the comment at `:366`). The null df is n − 1 with an
  intercept (`:371-372`).
- **The first step.** `coefold` starts as NULL (`glm.R:203`). A first
  step with a non-finite deviance, or an η or μ out of range, has
  nothing to halve towards, so R stops: "no valid set of coefficients
  has been found: please supply starting values" (`:271-272`,
  `:290-291`).
- **Clamps.** R's links bound η or μ near the edges
  (`src/library/stats/R/family.R`):
  - probit at ±(−Φ⁻¹(ε)) (`:61-62`);
  - cauchit at ±(−F⁻¹_Cauchy(ε)) (`:72-73`);
  - cloglog's μ in [ε, 1 − ε], with η ≤ 700 for μ′ (`:83-87`);
  - the log link's μ at least ε (`:100-102`).
  Each link's μ′ is at least ε where R bounds it (`:66`, `:77`, `:87`).
- **Warnings R gives where this RFC refuses.**
  - Non-convergence (`glm.R:319`).
  - Fitted probabilities numerically 0 or 1 (`:322-325`).
  - Fitted Poisson rates numerically 0 (`:326-329`), "numerically"
    meaning within 10ε.
- **Prediction.** `predict.glm` scales a link-scale standard error by
  |μ′(η)| (`src/library/stats/R/predict.glm.R:60`). `predict.lm`'s
  prediction variance is σ̂²/w with w = 1 by default (`lm.R:673-674`), and
  it warns when the fit was weighted (`:773-774`).
- **r² with an offset.** `summary.lm` subtracts the offset from the
  fitted values. It takes r² = mss/(mss + rss), mss about the weighted
  mean of those values with an intercept and about 0 without
  (`lm.R:304-340`). For the intercept-only model it computes no F and
  sets r² and adjusted r² to 0 (`lm.R:338-344`).
- **Comparison.** `anova` of GLMs uses the larger model's dispersion,
  and by default an F test when the family's dispersion is estimated and
  a χ² (likelihood-ratio) test when it is fixed (`glm.R:536-541`,
  `:647-653`).
  - Its table takes a negative deviance difference as 0, and its first
    row's differences are `NA` (`glm.R:520-521`).
  - `stat.anova` sets a negative statistic to `NA`, "rather than p = 0"
    (`anova.R:34`, `:43`).
  - An F test under a fixed dispersion takes the denominator df as ∞
    (`glm.R:660-661`) and warns that it is inappropriate (`:670-675`).
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
  singular design E106. Three non-finite quantities are left out of
  the answer rather than refused, each a key the program did not need
  for the rest: a `null.deviance` that is not finite (§S4), `aic` and
  `bic` for a non-integer Poisson `y` (§S4), and an infinite interval
  end (§S6, D251's A3). Every text is in §S8.
- **Forms** follow RFC-0015 §S3 **[B10]**: each word plain and `.`, the
  `.` form taking from the workbench the operand the plain word has on
  top (the `opts`, or the `fit` for `model.influence`), answering on the
  workbench (`bench!`). `model.influence.` takes nothing from the stack,
  as the `.` forms of RFC-0015's one-operand words take nothing.
- **Options MAPs** follow RFC-0016 §S1. They are read by a reader of
  this RFC's own, with the same rules and the same text forms as the
  frame words' reader (`frame/opts.rs`, `read`). An unknown key, a key
  the word does not take, or a value of the wrong type or range is E101
  naming the key. `frame/opts.rs` is not changed: its `KEYS` is one list
  for every frame word (`:28`), and adding this RFC's keys to it would
  change the frame words' refusal texts for those keys from "an unknown
  key" to "which this word does not take" (`:148-155`). `conf` is a FLOAT in (0, 1), default
  0.95 **[A4]**, read as RFC-0016 §S1 reads it, so an INTEGER is E101.
  `na` is read with `frame/opts.rs`'s texts for it, so that
  `model.matrix`'s and `model.predict`'s `na` refusals read as
  RFC-0017's.
- **Missing values [K14].** `model.matrix` and `model.predict` compute
  from cells, so they take the `na` option, `"refuse"` by default or
  `"omit"` (D252 C2, its third note). Every other word here reads a
  design or a fit, which hold no NONE.
- **Printing.** A fit and every answer over it hold only FLOATs,
  INTEGERs, STRINGs, BOOLs, LISTs and MAPs, and `model.anova`'s frame
  NONE in its first row as well, so `println` shows them
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
not repeat, where R silently drops a repeated term
(`src/library/stats/src/model.c:1415-1430`, `TrimRepeats`); and an interaction's every column must appear as a term of
its own before it (E101 naming it), so no column is coded except as a
main effect or a product of main effects already present. Only the
members are required: a three-way interaction needs its three columns as
earlier terms, not the two-way interactions among them **[K6]**. The
order is the order `model.anova` adds terms in.

**Coding [K5] [K7].**

| Column kind | Columns | Named [K4] |
|---|---|---|
| numeric | one, its cells as FLOATs (B15) | `dose` |
| logical | one, 1.0 for `true`, 0.0 for `false` | `treated.true` |
| factor, levels ℓ₁ … ℓₘ | m − 1, treatment coding: column j is 1.0 where the cell is ℓⱼ₊₁, the first level the baseline (R's `contr.treatment`) | `site.north`, `site.south` |
| factor, no intercept, the first factor term | m: every level its own column, as R codes the first factor it finds (`model.c:461-475`) | `site.east`, `site.north`, `site.south` |
| string, or no kind | refused: E101 naming the column; `frame.factor` makes a string column a factor | |

An interaction's columns are the products of its members' columns.
Its members are taken in R's variable order: the order in which their
columns first appear in `terms`, read left to right, each interaction's
members as written. `InstallVar` numbers a variable when the formula
first names it (`model.c:1091-1112`), and the column loop takes a
term's variables by that number (`:701`, `:711`). The member taken
first varies fastest: `addfactor` writes the next factor's column k
after the existing columns j, at k·ncx + j (`model.c:263-278`, called
at `:740-748`). The columns are named by joining the members' names
with `:` in the same order, as R builds a term's label
(`model.c:2053-2075`) **[author]**. So with `a` and `b` two
three-level factors, `[ "a" "b" [ "a" "b" ] ]` codes the interaction
as `a.2:b.2`, `a.3:b.2`, `a.2:b.3`, `a.3:b.3`, and
`[ "b" "a" [ "a" "b" ] ]` codes it as `b.2:a.2`, `b.3:a.2`, `b.2:a.3`,
`b.3:a.3`, as R codes `y ~ b + a + a:b`. The order in which an
interaction lists its members therefore changes neither its columns nor
their names: `[ "site" "dose" ]` after `"dose"` and `"site"` codes
`dose:site.north`. Inside an interaction, a factor is always coded
by its m − 1 contrast columns, whatever its main effect's coding. So
without an intercept, `[ "dose" "site" [ "dose" "site" ] ]` codes
`dose`, `site.east`, `site.north`, `site.south`, `dose:site.north` and
`dose:site.south`, six columns of full rank. The fully coded factor's
three products would sum to `dose`, and the design would be singular.

Where every lower-order margin of an interaction is an earlier term, as
in any two-way interaction under K6, the contrast columns lose nothing,
and R codes the same columns: its `TermCode` contrast-codes a factor
when the term without it is contained in one earlier term
(`model.c:1715-1749`). For an interaction of three or more members
without its lower-order interactions, which K6 allows, the two differ.
In `[ "a" "b" "c" [ "a" "b" "c" ] ]` of three factors, removing `a`
leaves {b, c}, which no single earlier term contains, so R codes each
factor of the three-way term by its dummies, and its columns reach every
cell mean. §S2 codes them by contrasts, a smaller model. This is K6's
option B, which was not ruled; a program that wants R's model adds the
two-way interactions first.

**A logical column is not a factor here, and is in R.** R's
`model.matrix` treats a logical as a two-level factor
(`src/library/stats/R/models.R:652`; `nlevs` 2, `model.c:439-443`), so
without an intercept it may be the first factor R codes in full
(`model.c:461-475`). §S2 codes a logical as one column always **[K7]**.
With an intercept the two span the same space. Without one they
differ: `[ "treated" ]` is one column here, so the `false` group's mean
is fitted at 0, and two in R; `[ "treated" "site" ]` spans the same
space as R's but R fully codes `treated` where §S2 fully codes `site`.
A program that wants R's model makes the column a factor with
`frame.factor`.

The intercept's column is all 1.0 and is named
`(intercept)`.

**Response, weights and offset.** A term may not read the response's
column, and `weights` and `offset` may not name the response, nor both
name one column (E101 naming it) **[author]**. R drops a response that
appears among the terms, with a warning (`model.c:547-548`). Here it is refused, because a model of y on y
is a mistake in the call.

**Unused levels.** A factor is coded from the levels that occur in the
rows the design keeps, in the frame's level order; a level with no such
row, left empty by `"omit"` or by an earlier `frame.filter`, codes no
column. R drops unused levels in the same way (`lm` and `glm` set
`drop.unused.levels`, `src/library/stats/R/lm.R:32`,
`src/library/stats/R/glm.R:50`). Coded instead, such a level's column
would be all 0.0 and the design singular, refused with nothing to say
why. A factor left with one level is coded as having none, which leaves
its term with no columns, and that is E101 naming the term.

**An interaction's members [author].** They must be distinct:
`[ "dose" "dose" ]` is E101 naming the term, where R's `:` takes the
union of its members' variables (`model.c:1343-1349`, `OrBits`, used by
`InteractTerms` at `:1475-1489`), so that `a:a` is `a`. Read as a
product, it would code dose², or a factor's products that copy its main
effect. Two interactions with the same members in any order are one
term, so `[ "site" "dose" ]` after `[ "dose" "site" ]` is refused as a
repeat, as R's `TrimRepeats` drops it (`model.c:1415-1430`). Its
columns' names follow the term as first given.

**Names are unique.** Two columns with the same name, such as a column
`site.north` beside the factor `site`'s level `north`, or a column named
`(intercept)`, are E101 naming both sources **[K4]**. Here `model.compare` tests nesting by name (§S7), and a
duplicate name would defeat that test.

**The response** must be numeric or logical (logical read as 1.0 and
0.0); a factor or string response is E101 **[K7]**. **Weights** must be
finite and positive; **offset** finite (E101 naming the column and row).

**NONE [K14].** Under `"refuse"` a NONE in any column the design reads,
the response, the terms' columns, `weights` and `offset`, is E101
naming the column and row, in that order of columns, each from row 1
(as D259's third choice orders `frame.aggregate`'s). Under `"omit"` each
row with a NONE in any of them is dropped. Zero rows left, or no more
rows than columns, is E101. So is a design of no columns, `intercept`
false with no terms (§S8): no fit word takes one. `model.anova` computes
its own null model without a fit word (§S7).

**The order of refusals [author].** `model.matrix` checks its operands
in this order, and refuses at the first failure:
0. `frame`: RFC-0017's check of the frame, its texts unchanged.
1. `opts`: its keys and their types (§S1).
2. `response`: a STRING naming a column. Then `weights` and `offset`,
   each naming a column.
3. `terms`: item by item, each a STRING or a LIST of two or more
   STRINGs, naming a column, its members distinct, not repeated, its
   members earlier. Then no
   columns at all, `intercept` false with no terms.
4. The overlaps between the response, terms, `weights` and `offset`
   (above).
5. Each column's kind: the response, then the terms' columns in order,
   then `weights` and `offset`. A column of no kind is refused here, so
   its NONE cells are never reached. Under `"refuse"` its first cell
   would be one, and under `"omit"` it would leave no rows.
6. NONE under `"refuse"`, as above. Then the cells of `weights` and
   `offset`.
7. The rows kept: none left, then a factor term with one used level,
   then two columns of one name, then n ≤ k, then the cap.

Each refusal of criterion 1 is produced by an input that fails only
that check.

**A logical column** has no unused level to drop. If it holds only
`true` in the rows kept, its column is all 1.0: with an intercept it
duplicates it, and the design is E106 naming it (§S3); without one it is
a valid fit. If it holds only `false`, its column is all 0.0, and the
design is E106 naming it with or without an intercept.

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
| `spec` | a MAP: `response`, `terms` (as given), `intercept`, `levels` (each factor column the design read, to the levels it coded from, its used levels), `logical` (the logical columns read), and `weights` and `offset` when given (their column names) |

`fit.lm`, `fit.glm` and `model.anova` check a design on entry: each key
present with its type, the MATRIX n × k, every LIST of its length, as the
frame words check a frame. A design may be made or changed by hand, so
the check also enforces what the words rely on, each failure E101
naming the key:
- `spec` holds `response`, `terms`, `intercept`, `levels` and
  `logical` of their types, and `weights` and `offset` when the design
  was given them;
- "with an intercept" is read from `spec.intercept`. When it is true,
  `assign`'s first item is 0, no other is, and that column is all 1.0;
  when it is false, no `assign` is 0;
- `assign` is non-decreasing, no item exceeds the length of
  `spec.terms`, and every term from 1 to that length codes at least one
  column;
- `names` holds k distinct STRINGs, since `model.compare` tests nesting
  by name;
- every cell of the MATRIX, `y`, `weights` and `offset` is finite, and
  every weight positive;
- without `spec.offset`, every `offset` is 0.0, and without
  `spec.weights`, every weight is 1.0. `spec.offset` decides whether
  `fit.glm` refits the null model (§S4) and whether `model.predict`
  reads an offset column (§S6), while the fits read the values; the two
  must agree;
- n > k, for every word that fits.
A design whose MATRIX a `linalg.` word has changed passes only if it
still meets these. `model.predict` checks its fit's `spec` in the same
way, and that the columns `spec` codes are the fit's `names`, in count
and in order (§S6). `spec` is what `model.predict` needs to code a new
frame the same way.

**Caps (D39).** n·k above 10⁸ cells is E111, as `frame.join`'s output
is capped (`frame/summary.rs`, `CAP`).

### S3. `fit.lm`

`design opts fit.lm` fits weighted least squares, minimising
Σ wᵢ (yᵢ − offsetᵢ − xᵢβ)², by the QR of √W·X. `opts` must be a MAP
and takes no key yet, any key being E101; it is there so that a later
key needs no new word, as `fit.glm`'s takes its family **[author]**.
n ≤ k is E101. A singular design is E106 by `stat.lm`'s rank test,
naming the first column that depends on those before it **[K8]**. The
QR is not pivoted, so that column is the first whose diagonal fails the
test. With an intercept, a constant `y − offset` is E101 (`constant`),
as `stat.lm` refuses a constant `y`, since r² would divide by 0. Without
an intercept, a constant `y` is a fit like any other **[author]**. A
perfect fit is E101, as `stat.lm` refuses it.

The answer, a fit, is a MAP:

| Key | Holds |
|---|---|
| `coef`, `se`, `t`, `p` | per column, in `names` order; t on n − k df |
| `names` | the design's `names` |
| `vcov` | σ̂²(XᵀWX)⁻¹, a LIST of k LISTs |
| `sigma` | σ̂, √(RSS/(n − k)), RSS the weighted residual sum |
| `deviance` | RSS |
| `df.residual` | n − k, an INTEGER |
| `r2`, `adj.r2` | 1 − RSS/TSS, TSS = Σ wᵢ(yᵢ − offsetᵢ − m)², m the weighted mean of y − offset with an intercept and 0 without. This equals R's mss/(mss + rss) on the fitted values less the offset (`lm.R:304-340`), since TSS = MSS + RSS. It is computed as `stat.lm` computes its `r2`, so that the two agree bit for bit (criterion 2). `adj.r2` = 1 − (1 − r2)(n − i)/(n − k), i being 1 with an intercept and 0 without |
| `f`, `f.df`, `f.p` | ((TSS − RSS)/(k − i))/(RSS/(n − k)): the overall F test against the intercept-plus-offset model, or against the offset alone without an intercept; `f.df` a LIST of two INTEGERs |
| `null.deviance`, `df.null` | the null fit's RSS and df (below) |
| `aic` | n(ln(2π·RSS/n) + 1) − Σ ln wᵢ + 2(k + 1), i.e. −2 log L + 2(k + 1). That is R's Gaussian `aic` (`family.R:278-281`), whose `+2` counts σ, plus 2k (`glm.R:376-377`), so it equals `fit.glm`'s Gaussian identity AIC |
| `bic` | `aic` − 2(k + 1) + (k + 1)·ln n, n the rows; every weight is positive (§S2), so n is R's `nobs.lm` (`src/library/stats/R/logLik.R:110-111`) |
| `fitted`, `y`, `weights`, `offset`, `hat` | n FLOATs each: μ̂, the response, the prior weights, the offset, and the leverages hᵢ, the diagonal of √W X (XᵀWX)⁻¹ Xᵀ √W |
| `family`, `link` | `"gaussian"`, `"identity"` |
| `dispersion` | σ̂² |
| `spec` | the design's `spec` |

With all weights 1.0, no offset and an intercept, `coef`, `se`, `t`,
`p` and `r2` equal `stat.lm`'s on the design's MATRIX without its first
column, bit for bit **[K15]**. `df.residual` is an INTEGER equal in
value to `stat.lm`'s `df`, a FLOAT. With unit weights √W·X is X exactly,
and an offset of 0.0 leaves y unchanged, so the shared computation sees
`stat.lm`'s numbers. The shared function does all of the arithmetic
after the scaling: RSS is the dot product of the √W-scaled residual
with itself, as `stat.lm` takes `resid.dot(&resid)`, and σ̂, `se`, `t`,
`p` and `r2` come from the same lines. The weighted mean is
Σ wᵢ(yᵢ − offsetᵢ)/Σ wᵢ, which with unit weights is `describe.rs`'s
`mean`, Σ/n, bit for bit.

**The intercept-only model.** When k = i, so that the design is the
intercept alone, `r2` and `adj.r2` are 0 and `f`, `f.df` and `f.p` are
left out of the fit, as R computes no F there (`lm.R:338-344`). The F
statistic would be 0/0. `r2` is set to 0, not computed, because RSS and
TSS are reached by different arithmetic and need not round alike.

**The null fit.** `null.deviance` is RSS of the intercept-plus-offset
model, Σ wᵢ(yᵢ − offsetᵢ − m)², with an intercept. Without one it is
Σ wᵢ(yᵢ − offsetᵢ)². `df.null` is n − 1 with an intercept and n
without. Both are closed forms; for `fit.glm` the same quantities need
a refit (§S4).

### S4. `fit.glm` [K9] [K10] [K11] [K12]

`design opts fit.glm` fits a generalised linear model by IRLS, as R's
`glm.fit` does (§Current behaviour).

**Options:** `family` (`"binomial"`, `"poisson"`, `"gaussian"`,
`"gamma"`, `"inverse.gaussian"`; required), `link` (default the
family's canonical link: logit, log, identity, inverse, 1/mu^2). The
links are R's `okLinks` for each family **[K9]**:

| Family | Variance V(μ) | Links | Valid μ (R's `validmu`) | Response |
|---|---|---|---|---|
| `binomial` | μ(1 − μ) | `logit`, `probit`, `cloglog`, `cauchit`, `log` | 0 < μ < 1 (`family.R:354`) | in [0, 1], w·y and w each an integer within 10⁻³ **[K10] [K12]** |
| `poisson` | μ | `log`, `identity`, `sqrt` | μ > 0 (`:157`) | ≥ 0 |
| `gaussian` | 1 | `identity`, `log`, `inverse` | any (`:292`) | any; > 0 under `log`, ≠ 0 under `inverse` (R's `initialize`) |
| `gamma` | μ² | `inverse`, `log`, `identity` | μ > 0 (`:468`) | > 0 |
| `inverse.gaussian` | μ³ | `1/mu^2`, `inverse`, `log`, `identity` | μ > 0 **[author]**; R's is any μ (`:537`) | > 0 |

Each link's valid η is R's `valideta` (§Current behaviour): η > 0
under `sqrt` and `1/mu^2`, η ≠ 0 under `inverse`, any finite η under
the others. Under `sqrt` a step to η < 0 gives μ = η² > 0, which every
family's μ test passes. Only the η test sends it to halving, which
keeps the fit on the increasing branch of the link.

A response outside its family's range is E101 naming the row. Under the
binomial, a weight more than 10⁻³ from an integer is E101 naming the
row, as K12 refuses w·y **[author]**. The weights are the trials, and
R's AIC takes `round(m)` of them (`family.R:356-359`). With w and w·y
each within 10⁻³ of an integer, neither rounding meets a half, so how
a half is rounded does not matter. The inverse Gaussian's valid μ is
this RFC's choice. R accepts any μ, and
its deviance (y − μ)²/(yμ²) (`family.R:528`) stays finite for μ < 0. But then V(μ) = μ³
is negative and the working weight √(w·μ′²/V) is not a number. Here a
step to μ ≤ 0 is halved, as for the Gamma.

Every family's `mustart` is valid under each of its links, given this
table's response ranges and R's `initialize` rules (§Current behaviour),
except at the edges of the FLOATs, where η = g(μ_start) is not finite:
under `1/mu^2`, 1/y² overflows for y below about 7·10⁻¹⁵⁵, and under
`inverse`, 1/y overflows for a subnormal y. R's `valideta` needs a
finite η (`family.R:115`, `:121`), and R stops there with "cannot find
valid starting values" (`glm.R:218-219`). Here a start with a non-finite
η is E101 naming the row (§S8), before any step.

**The iteration [K11].** Start from the family's `mustart` (R's, above)
and η = g(μ_start). Each step is R's: z and w as at `glm.R:244-248`,
then a QR solve of √W·X with `weighted_solve`'s rank test. It stops when
|dev − dev_old|/(0.1 + |dev|) < 10⁻⁸.
- **Step halving.** On the second or a later step, a non-finite
  deviance, or an η outside the link's valid range or a μ outside the
  family's, halves the step towards the previous coefficients, R's test
  of both (`glm.R:289`). A step may be halved at most 100 times, counted
  afresh for each step as R's loops count (`glm.R:274-277`,
  `:293-296`), else E110 naming `IRLS step halving`.
- **The first step.** On the first step there are no previous
  coefficients, and the same condition is E101, as R stops there
  (§Current behaviour, `glm.R:203`, `:271-272`, `:290-291`).
- **The cap.** At most 100 iterations (`fit.logistic`'s cap, not R's
  25), else E110. R warns and answers instead (`glm.R:319`); here an
  unconverged fit is refused, as `fit.logistic` refuses it.
- **No observation is dropped.** R drops an observation whose μ′(η)
  is 0 from a step (`glm.R:236`). Under the clamps below, μ′ is not 0
  in the valid range except by underflow. Under `inverse`, μ′ = −1/η²
  (`family.R:120`) underflows for |η| above about 10¹⁵⁴, and `1/mu^2`
  is similar. There z is not finite, and the step is E114 naming the
  row **[author]**, where R would drop the observation from that step.
- **Non-finite coefficients.** A solve that answers a coefficient not
  finite is E114 naming the coefficient and the iteration **[author]**.
  R stops the loop there with a warning (`glm.R:249-253`). Halving
  towards the previous coefficients cannot make it finite, and the
  halving and first-step texts would name the wrong cause.
- **E106** is a singular design, naming the first dependent column, as
  §S3 **[K8]**. `fit.glm` and `fit.lm` solve through a crate-internal
  function with `weighted_solve`'s QR and rank test that also answers
  the failing column. That function is for the solve and the rank test
  only. `fit.lm`'s `se` comes from `stat.lm`'s line, σ̂² times the row
  sums of R⁻¹ squared (`infer.rs`, `stat_lm`), so criterion 2's bits
  hold. `stat.lm` computes no covariance, so `vcov`, σ̂²·R⁻¹R⁻ᵀ, is
  computed newly, and its diagonal need not equal `se`² to the last
  bit; `weighted_solve`'s covariance, a matrix product, need not round
  alike either. `weighted_solve` itself, and `fit.logistic`'s
  text, are unchanged.

**Clamps**, as R's `make.link` bounds each link (§Current behaviour):

| Link | Bound |
|---|---|
| `logit` | η at ±`ETA_LIMIT` (`fit.rs`), −ln ε, continuously: `fit.logistic`'s clamp, kept for K15. R's is piecewise (§Current behaviour, `family.c:83-84`) |
| `probit` | η at ±(−Φ⁻¹(ε)) |
| `cauchit` | η at ±(−F⁻¹_Cauchy(ε)) |
| `cloglog` | μ in [ε, 1 − ε]; μ′ at least ε, η ≤ 700 inside it |
| `log` | μ = max(exp η, ε), and μ′ the same |
| `identity`, `inverse`, `sqrt`, `1/mu^2` | none; the valid-range test and halving apply |

**Separation.**
- **Binomial.** Under `logit`, a final |η| at `ETA_LIMIT` is E101, "X
  separates y". That is `fit.logistic`'s test (`fit.rs`,
  `fit_logistic`), so the two words refuse the same 0/1 data. Under the
  other links, a fitted μ within 10ε of 0 or 1 is the same refusal,
  R's test (`glm.R:322-325`), where R only warns **[author]**. The
  thresholds therefore differ by link. Under `logit` the refusal comes
  at |η| ≥ 36.04, later than R's warning. R's warning comes at
  |η| > 30, where its piecewise logit saturates and puts μ within ε of
  0 or 1 (`family.c:83-84`). Under the other links the refusal comes
  where R's warning does.
- **Poisson.** A fitted μ below 10ε, under any link, is E101 naming the
  row: R's test for "fitted rates numerically 0" (`glm.R:326-329`),
  made a refusal where R only warns **[author]**. That is all it is.
  It does not catch a group of zero counts. The deviance rule stops
  such a fit while μ is still far above 10ε: an intercept and an
  indicator, counts 3, 5, 4 and 0, 0, 0, stop at the 19th iteration
  with the indicator's coefficient −22.69 and the smallest μ 5.6·10⁻¹⁰
  (a reimplementation of §S4's iteration). That fit is answered, its
  large coefficient and standard error included, as R answers it
  **[author]**. The refusal fires when μ is small by design, as with
  an offset for a tiny exposure: an intercept, counts 3, 5, 4, 0, and
  an offset of 0, 0, 0, ln 10⁻¹⁶ give μ₄ = 4·10⁻¹⁶.
- **Once, at convergence.** Both tests run once, on the converged fit.
  A step that passes near 0 or 1 and recovers is not refused.

**The null deviance** is the deviance of the null fit:
- **no intercept:** at μ = linkinv(offset);
- **an intercept and no offset:** at the weighted mean of y;
- **an intercept and an offset:** the intercept-plus-offset model, fitted
  by this section's IRLS from the full fit's μ̂, as `glm()` refits it
  (`glm.R:99-119`). "An offset" is one the design was given
  (`spec.offset`), zeros included, as R refits whenever an offset is
  supplied (`glm.R:108`). The `fit.logistic` path, by contrast, is
  chosen by the offset's values (D263).
That refit's non-convergence is E110 naming `IRLS (null model)`, where R
only warns **[author]**. The refit is §S4's iteration in full: its
first step out of range is the first-step refusal, and separation and
the Poisson test apply to it, each text naming `(null model)`.

Without an intercept the null deviance can be infinite. With no offset,
linkinv(0) is 0 under `identity` and `sqrt`, ∞ under `inverse` and
`1/mu^2`, and 1 under the binomial's `log`. At those μ the deviance of
any y away from them is infinite or not a number, except under the
Gaussian with `identity`, whose deviance Σ w(y − μ)² (`family.R:277`)
is finite at μ = 0, as §S3 computes it. The Gaussian under `inverse`
has μ = ∞ and an infinite null deviance. R answers the
non-finite value (`glm.R:367-369`). Here a `null.deviance` that is not
finite is left out of the fit, `df.null` kept, and the fit is answered
**[author]**. The model itself is valid, and B12 would otherwise refuse
it for a quantity it did not ask for. `model.anova` refuses such a
design (§S7).

**The binomial AIC** rounds m·y and m, as R's does (`family.R:356-360`).
K12 and the weight check above have already refused any count or trial
more than 10⁻³ from an integer.

**The Poisson AIC.** A Poisson `y` need not be an integer: a rate, or a
non-integer abundance, is fitted as R fits it. R's AIC on such a
response is +∞ (§Current behaviour, `dpois`), which B12 cannot answer.
So when any `y` is more than 10⁻⁹·max(1, |y|) from an integer, R's test
(`nmath.h:63`), `aic` and `bic` are left out of the fit, and every other
key is answered **[author]**. The alternatives are E101, as K12 does for
the binomial, or the continuous extension y ln μ − μ − ln Γ(y + 1),
which R does not compute. A `y` within that test of an integer is
rounded to it for the AIC, as `dpois` rounds x by `R_forceint`
(`dpois.c:83`; `nearbyint`, or `round` where that is missing,
`nmath.h:57-59`). Within 10⁻⁹ of an integer no half arises, so the two
agree.

**A perfect fit [author].** Under the Gaussian, Gamma and inverse
Gaussian, whose dispersion is estimated, a deviance of 0 makes φ 0, so
`se` is 0 and t is 0/0, and the Gamma and inverse Gaussian AICs take
ln 0. Such a fit is E101 with §S3's perfect-fit text, as `fit.lm`
refuses it, not E114. The test is a deviance ≤ 0: at y ≈ μ the Gamma's
terms can round a few ulps below 0, and a negative φ would otherwise
reach the AIC as a negative shape. Under the binomial and Poisson φ is 1, and a
deviance of 0 is answered.

**Where IRLS stops [K11].** Iteration stops by R's rule, the one
`fit.logistic` uses (`fit.rs`, `DEVIANCE_TOL`). Under a canonical link
IRLS is Newton's method and converges quadratically, and when the rule
stops it the coefficients are some 10⁻⁸ from the solution of the score
equations. Under any other link it converges linearly, and the stopped
coefficients can be some 10⁻⁵ away (§Current behaviour). Criterion 3
therefore checks an IRLS fit against §S4's own iteration, run at 50
digits and stopped by the same rule, not against the exact solution.

**The answer** is a fit. It has the keys below and no others. In
particular it has no `sigma`, `r2`, `adj.r2`, `f`, `f.df` or `f.p`,
whatever the family; a Gaussian identity fit that needs them is
`fit.lm`'s. A key the table marks as sometimes left out (`null.deviance`,
`aic`, `bic`) is absent, never NONE.

| Key | Holds |
|---|---|
| `coef`, `se`, `p` | per column; se from φ·(XᵀWX)⁻¹ at the estimate |
| `t` or `z` | coef/se: `z`, p normal, when φ is fixed (binomial, Poisson); `t`, p on the residual df, when φ is estimated, as `stat.lm` and `fit.logistic` name them. A program that reads either fit tests which key is present, or reads `family`; the example (criterion 7) shows it |
| `names`, `vcov`, `spec`, `y`, `weights`, `offset` | as §S3 |
| `fitted` | μ̂ |
| `linear` | η̂ |
| `hat` | the leverages at the estimate, from √W X |
| `working.weights` | W at the estimate |
| `dispersion` | 1 for the binomial and Poisson; else Pearson's Σ wᵢ(yᵢ − μᵢ)²/V(μᵢ) over n − k |
| `deviance`, `null.deviance` | the family's deviance at μ̂, and at the null model (R's rule); `null.deviance` left out when not finite (above) |
| `df.residual`, `df.null` | n − k and n − 1 (n without an intercept), INTEGERs |
| `aic` | the family's AIC, R's formula, plus 2k; left out for a non-integer Poisson `y` (above), and so is `bic` |
| `bic` | `aic` − 2p + p·ln n, p being k, plus 1 for the Gaussian, Gamma and inverse Gaussian, whose dispersion is estimated, and n the rows (R's `logLik.glm` and `nobs.glm`, `src/library/stats/R/logLik.R:54-68`, `:113-114`) |
| `iterations` | an INTEGER |
| `halvings` | an INTEGER: the halvings, summed over every iteration (the cap of 100 is per step) |
| `family`, `link` | the STRINGs used |

For the binomial under `logit` on a 0/1 response with unit weights and
no offset, `fit.glm` takes `fit.logistic`'s arithmetic. "Unit weights
and no offset" is read from the values: every weight exactly 1.0 and
every offset exactly 0.0, however the design was made. On that path:
- W = μ(1 − μ) rather than μ′²/V;
- z from the clamped η;
- the deviance from η by `fit.logistic`'s softplus form (`fit.rs`,
  `deviance`) rather than from μ;
- `aic` = deviance + 2k, as `fit.logistic` takes it (`fit.rs`,
  `fit_logistic`, the `aic` line). On 0/1 data with unit weights this
  equals R's binomial AIC, since every saturated term is 0. Computed
  through μ instead, −2 ln(1 − μ) at a 0 with η = 15 is already
  1.5·10⁻¹¹ off relative, outside the 10⁻¹² below.
Off that path, under `logit` as under every link, z is formed as R forms
it (`glm.R:244-248`): from the linear predictor as solved, with μ and μ′
taken through the link's clamp.
The deviance sequence, and so the stopping iteration, is then
`fit.logistic`'s. The coefficients, standard errors, deviance and AIC
agree to a relative 10⁻¹² (criterion 3). They are not promised bit for
bit, since the two words reach the solve through different code
**[K15]**.

### S5. Residuals and influence

`fit opts model.residuals` answers a LIST of n FLOATs. `opts`: `type`,
one of `"response"` (y − μ), `"pearson"` ((y − μ)√w/√V(μ)), `"deviance"`
(sign(y − μ)√dᵢ, dᵢ the observation's deviance contribution),
`"working"` ((y − μ)/μ′(η)), `"standardized"` (the deviance residual
over √(φ(1 − hᵢ)), R's `rstandard.glm`). The default is `"deviance"`,
R's for a GLM (`glm.R:860-862`, `residuals.glm`), which for an
unweighted `fit.lm` is y − μ **[author]**.

`fit model.influence` answers a MAP of three LISTs of n FLOATs: `hat`,
`std.residuals` (the deviance residual standardised as above), and
`cooks` (r_P/(1 − h))²·h/(φ·k), R's `cooks.distance.glm`, which is R's
`cooks.distance.lm` for `fit.lm`. A leverage of 1 is refused
**[author]**. The test is 1 − hᵢ ≤ 10⁻¹⁰, as the rank test uses 10⁻¹⁰,
because hᵢ is rarely exactly 1 in floating point. A factor level with
one row gives hᵢ = 1 − δ, δ near ε. At such a row the fit passes
through yᵢ, so the standardised residual and Cook's distance are 0/0,
and computed they would be a meaningless finite number, ±∞, or NaN by
rounding. R turns only an infinite value into NaN
(`lm.influence.R:172`, `:262`). The refusal is E114 naming the row, and
both `model.influence` and `model.residuals` with `type`
`"standardized"` give it, the two words that divide by 1 − hᵢ.

Both read only the fit; neither needs the design.

### S6. `model.predict` [K16]

`fit frame opts model.predict` codes `frame` by the fit's `spec`, as
`model.matrix` coded the design: the same columns, a factor cell
checked against the spec's levels (a level the fit never saw is E101
naming it), the response not read, `offset`'s column read when the spec
names one. `opts`: `type` (`"link"` or `"response"`, default
`"response"` **[author]**, where R's `predict.glm` defaults to `"link"`,
`predict.glm.R:20`; a program porting `predict(fit, newdata)` asks for
`"link"`), `interval` (`"none"`, `"confidence"` or `"prediction"`,
default `"none"`), `conf`, `na`, and `weights`, a column of `frame`
giving each new row's prior weight for a prediction interval, default
1.0 for every row. R's `predict.lm` assumes 1 and warns when the fit
was weighted (`lm.R:673-674`, `:773-774`); here the default is stated and
no warning is given **[K16]**.

Each column the spec reads must have, in `frame`, the kind it had when
the design was coded **[author]**. A numeric column must be numeric, a
logical column logical, and a factor a factor, else E101 naming the
column and both kinds. A factor's cells are checked by their level's
value against the spec's levels, so a frame whose `levels` list for the
column differs, by order or by levels no cell uses, is coded the same.
The new rows are coded with the spec's columns and levels, whatever
levels they use: a factor whose new cells use one level, or a frame of
one row, is coded and predicted. Of §S2's checks, `model.predict`
applies, after its own check of `fit` and `opts`:
- RFC-0017's check of `frame`;
- each column the spec reads present in `frame`, else E101,
  `frame is outside model.predict's domain: no column "dose"`. The
  spec reads the terms' columns and, when it names one, `spec.offset`'s
  column; `spec.weights`' column is not read, and its absence is not
  refused;
- step 5, each column's kind as coded (above), the response not read;
- step 6, NONE under `"refuse"`, then the cells of `offset` and of the
  `weights` option's column (the spec's `weights` column is not read);
- from step 7, only "no rows left" and the cap.
It does not refuse a level the new rows leave unused, a factor with one
used level, or fewer rows than the fit has coefficients.

The `weights` option names a numeric column of `frame`. It is read as
`model.matrix` reads its `weights` column: NONE under `na` as any other
column it reads, and a cell not finite and positive E101 naming the
row.

The answer is a MAP:
- `fit`, a LIST of predictions;
- `se`, their standard errors: √(xᵀ V x) with V the fit's `vcov` on
  the link scale, and on the response scale that times |μ′(η)| (the
  delta method; R's `predict.glm` takes the absolute value,
  `predict.glm.R:60`);
- with an interval, `lower` and `upper`.

On the response scale, η̂ and the interval ends map through §S4's
clamped inverse link, and `se` scales by its clamped μ′, as R's
`predict.glm` maps through the family's own `linkinv` and `mu.eta`
(`predict.glm.R:60-61`), which are bounded: `pmax(exp(eta),
.Machine$double.eps)` under `log` (`family.R:99-102`), and the
piecewise logit (`family.c:83-84`, `:103`). Under `logit` the clamp is
`fit.logistic`'s, at ±`ETA_LIMIT`, not R's piecewise one, as in §S4.
So under `log` no response-scale end is below ε. A non-finite `fit` or
`se`, as `log` gives at η̂ = 800, is E114 naming the row (§S1). D251's
A3, below, applies only to `lower` and `upper`.

`model.predict`'s check of `fit` is §S7's, and then one more: the
columns `spec` codes, by §S2's rules from its `terms`, `intercept`,
`levels` and `logical`, must be the fit's `names`, in count and in
order, else E101 on `fit`. A design whose MATRIX a `linalg.` word has
changed can meet every §S2 invariant and still code other columns
than the fit has coefficients.

A confidence interval is η̂ ± q·se on the link scale, q normal when φ is
fixed and t on the residual df when it is estimated. q comes from
`dist.rs`'s `Family::quantile`, whose polish can be E110 as RFC-0016
§S1 notes, and that refusal is passed on. For `"response"`
both ends are mapped through the inverse link **[author]**. `lower` is
the smaller of the two mapped ends and `upper` the larger, since
`inverse` and `1/mu^2` are decreasing. That mapping is an interval only
inside the link's valid η (§S4), so on the response scale, per row
**[author]**:
- **η̂ or an end outside the link's valid η**, or an interval that
  contains 0 under `inverse`, is E101 naming the row and the link, and
  `type` `"link"` answers it. Under `inverse` an interval across 0 maps
  to two half-lines, under `sqrt` an interval across 0 folds, and under
  `1/mu^2` an end ≤ 0 is not a number. R's `predict.glm` applies
  `linkinv` to η̂ wherever it lies (`predict.glm.R:61`).
- **An end that maps to ±∞**, as the zero group's upper end does
  (exp of about 4.8·10⁴), follows D251's A3: the key is left out. A3
  rules a scalar; here `lower` or `upper` is a LIST, and the whole key
  is left out when any of its ends is infinite, the other key answered.
- **An end outside the family's valid μ**, such as a binomial `log`
  end above 1 or a Poisson `identity` end below 0, is answered as
  computed; it is an interval for the mean of the linear model, not a
  bound the family enforces.

Leaving a whole key out costs the other rows' ends on that side. A
program that needs them predicts those rows apart, or asks for `type`
`"link"`, whose ends are always finite; it tests for the key as D251's
note describes.

A prediction interval adds σ̂²/wᵢ to the
variance, wᵢ the new row's weight, and is for `fit.lm`, and `fit.glm`
under `gaussian` and `identity`, only; elsewhere E101. The answer
always holds `dropped`, an INTEGER: the rows left out under `"omit"`,
and 0 under `"refuse"`, as the design's `dropped`.

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
χ²(Δdf), φ_big 1 for the binomial and Poisson. A `fit.lm` fit and a
`fit.glm` fit under `gaussian` and `identity` have the same family and
link and can be compared. Their deviances are each word's own, equal to
rounding.

- **A negative Δdeviance** is taken as 0, as R's `anova.glm` takes it
  in its sequential table (`glm.R:520-521`). The answer's `deviance` is
  then 0, the statistic 0 and p 1. Between nested fits Δdeviance can be
  negative only by rounding, as when the larger model adds nothing:
  Dobson's `treatment` (criterion 6). R's two-model comparison,
  `anova.glmlist`, does not clamp (`glm.R:631-632`), and its
  `stat.anova` answers `NA` for a negative statistic (`anova.R:34`,
  `:43`). B12 has no `NA` to answer, so both words here clamp, as
  `anova.glm` does.
- **Equality.** "The same `y`, `weights` and `offset`" is exact FLOAT
  equality, element by element, and equal lengths.
- **F under a fixed dispersion.** `test` `"F"` on a binomial or Poisson
  pair is E101 **[author]**. R computes it with an infinite denominator
  df and warns that it is inappropriate (`glm.R:660-661`, `:670-675`).
  Here, as elsewhere, the warning becomes a refusal. `"LR"` under an
  estimated dispersion is answered, scaled by φ_big, as R scales it.
- **Same frame.** Nesting is checked by family, link, `y`, `weights`,
  `offset` and names. Two fits from different frames whose columns
  share names can pass that check. The word cannot see it, and assumes
  that both fits came from one frame.
- **Reading a fit.** Every `model.` word that reads a fit checks the
  keys both fit words answer (`coef`, `se`, `p`, `names`, `vcov`,
  `fitted`, `y`, `weights`, `offset`, `hat`, `deviance`,
  `df.residual`, `dispersion`, `family`, `link`, `spec`), and then the
  keys it reads itself. A `fit.glm` key a `fit.lm` fit lacks
  (`linear`, `working.weights`, `iterations`, `halvings`) is derived
  for it: η̂ = μ̂, W = the prior weights. A missing or mistyped key is
  E101 naming it. A fit is a MAP a program can edit, so the check also
  asks the lengths the words index by, each failure E101 naming the
  key: `coef`, `se`, `p` and `names` of one length k, `vcov` k × k, and
  `fitted`, `y`, `weights`, `offset` and `hat`, with `linear` and
  `working.weights` when present, of one length n.

`design opts model.anova` starts from the null fit and then adds the
design's terms one at a time, in order, by `assign`. It takes
`fit.glm`'s options and `test`. `family` is required, as for `fit.glm`,
and `link` defaults to the family's canonical link. Under `"gaussian"`
and `"identity"` each model is fitted by `fit.lm`'s computation, closed
form; otherwise by `fit.glm`'s IRLS.
- **Tests.** Each row's test uses the full model's dispersion, and an F
  test the full model's residual df as its denominator df, as R's
  `anova.glm` does (`glm.R:548-551`). `test` `"F"` under a fixed
  dispersion is E101, as in `model.compare`.
- **Gaussian identity rows [author].** A row needs only RSS, so §S3's
  refusal of a constant `y − offset` with an intercept, which is about
  r², does not apply to a row. A full model whose deviance is 0 is
  refused with §S3's perfect-fit text, since every F statistic would
  divide by a dispersion of 0. The full model is fitted first, and this
  refusal comes from it, with no row suffix, before any other row is
  fitted. A constant `y` therefore meets that refusal, and not the
  constant one.

A refusal while fitting a row is passed on with its code, and its text
ends `, fitting the model through term "treatment"` (or `, fitting the
null model`) **[author]**. A refusal from the full model, which is
fitted first, has no suffix: it is the refusal `fit.glm` or `fit.lm`
would give on the same design and options. A submodel can refuse where the full model
does not, for instance with a first IRLS step out of range or E110.
Every refusal of §S4 applies to every row's fit.

The first row is the null fit: with an intercept, the intercept-only
model (with the offset). Its deviance is the full fit's `null.deviance`,
by §S4's rule, refitted from the full fit's μ̂ when there is an offset,
as R's `anova.glm` takes `object$null.deviance` (`glm.R:516`). Fitted
afresh from `mustart`, it could stop at another iterate under a
non-canonical link. Without an intercept
it is the model of no columns, which no fit word takes (§S2). Its
deviance is at μ = linkinv(offset), its residual df n, and nothing is
solved. If that deviance is not finite (§S4), `model.anova` is E114,
naming `the null model's deviance at linkinv(offset)` **[author]**:
every later row's difference would be infinite. It answers a frame
(RFC-0017's shape), one row per model: `term` (a string column:
`(intercept)` or `NULL` first, then each term's name, an interaction's
joined by `:`), `df` and `resid.df` (INTEGER columns), `deviance`,
`resid.deviance`, `statistic` and `p` (numeric columns). The first row
has no earlier model, so its `df`, `deviance`, `statistic` and `p` are
NONE, as R's are `NA` (`glm.R:520-521`). This is R's type I table; types II
and III are not in this RFC **[K1]**.

### S8. Refusals

Every refusal is E101 unless the row says otherwise; `<w>` is the word.
A row number names the operand's row, from 1: `model.matrix` and
`model.predict` name the frame's row, so under `"omit"` a row number is
the frame's, not the design's; the fit words and `model.anova` name the
design's row, which a program maps back through the rows it kept.
Values from the program are rendered as RFC-0017's are (`render.rs`):
escaped, bounded, quoted.

`model.matrix` (§S2) and `model.predict` (§S6) state their order of
refusals. Every other word checks its operands deepest first (`design`,
or `fit`, or `small` then `big`), each by §S2's design check or §S7's
fit check in full, then `opts`, then computes. So a malformed design
with an unknown `family` is refused for the design, and a malformed fit
with an unknown residual `type` for the fit.

| Case | Code | Text |
|---|---|---|
| `response` not a STRING, or not a column | E101 | `response is outside <w>'s domain: <summary>` / `no column "y"` |
| the response a factor or string column | E101 | `response is outside <w>'s domain: column "y" is a factor` / `is string` |
| `terms` not a LIST, an item neither a STRING nor a LIST of two or more STRINGs | E101 | `terms is outside <w>'s domain: item 2 is <summary>` |
| a term naming no column, repeated, or an interaction before its main effect | E101 | `terms is outside <w>'s domain: no column "zz"` / `term 3 repeats term 1` / `term 3 needs "dose" as a term before it` |
| a string or no-kind column in a term | E101 | `terms is outside <w>'s domain: column "g" is string` |
| a term reading the response, `weights` or `offset` naming the response, or both naming one column | E101 | `terms is outside <w>'s domain: term 2 reads the response "y"` / `opts is outside <w>'s domain: weights names the response "y"` / `offset names "w", as weights does` |
| `weights` or `offset` naming no column, not numeric, or a cell not finite and positive (`weights`) | E101 | `opts is outside <w>'s domain: weights row 4 is 0` (a 0 adds `; frame.filter sets a row aside`) |
| NONE under `"refuse"` | E101 | `frame is outside <w>'s domain: column "x" row 2 is none (na "refuse")` |
| no rows, or n ≤ k | E101 | `frame is outside <w>'s domain: no rows` / `5 observations for 6 coefficients` (`model.matrix`); `design is outside <w>'s domain: 5 observations for 6 coefficients` (the fit words and `model.anova`) |
| an interaction repeating a member | E101 | `terms is outside <w>'s domain: term 3 repeats "dose"` |
| a malformed design's invariant (§S2) | E101 | `design is outside <w>'s domain: assign item 4 is 1, after 2` / `matrix row 3 column 2 is NaN` |
| a non-finite coefficient from a solve | E114 | RFC-0015's text, the expression `the coefficient of "dose" at IRLS iteration 4` |
| a response-scale prediction outside the link's valid η (`model.predict`) | E101 | `frame is outside <w>'s domain: row 2's prediction is outside the sqrt link's valid range; type "link" answers it` / `row 2's interval crosses 0 under the inverse link; type "link" answers it` / `row 2's lower end is outside the 1/mu^2 link's valid range; type "link" answers it` |
| no columns: `intercept` false and no terms | E101 | `terms is outside <w>'s domain: no columns, with no intercept and no terms` |
| a constant `y − offset` with an intercept (`fit.lm`) | E101 | `stat.lm`'s `constant` text, on `design` |
| n·k above 10⁸ | E111 | RFC-0015's text, `<what>` being `the design's cells` |
| two columns with one name | E101 | `terms is outside <w>'s domain: column name "site.north" is both column "site.north" and "site" level "north"` |
| a factor term left with one used level | E101 | `terms is outside <w>'s domain: term 2, "site", has one used level, "north"` |
| a malformed design | E101 | `design is outside <w>'s domain: <key> <what is wrong>` |
| a response outside its family's range | E101 | `design is outside <w>'s domain: y row 3 is -1, below the poisson family's 0` |
| a non-integer binomial count | E101 | `design is outside <w>'s domain: y row 3 times its weight is 2.5, not a count` |
| a non-integer binomial weight | E101 | `design is outside <w>'s domain: weights row 3 is 2.5, not a count of trials` |
| an unknown family, or a link the family does not take | E101 | `opts is outside <w>'s domain: link is "sqrt", not a binomial link` |
| a singular design | E106 | RFC-0015's text, then `: column "dose:site.south" depends on those before it` (the first column whose pivot fails `stat.lm`'s rank test). `stat.lm`'s and `fit.logistic`'s texts are unchanged |
| IRLS or step halving past 100 | E110 | RFC-0015's text, method `IRLS` / `IRLS step halving` / `IRLS (null model)`. For `IRLS step halving` the template's "iterations" counts the halvings of one step |
| a start with a non-finite η | E101 | `design is outside <w>'s domain: y row 3 is 1e-200, whose starting η under the 1/mu^2 link is not finite` |
| the first IRLS step out of range | E101 | `design is outside <w>'s domain: the first IRLS step leaves the gamma family's valid range, with no earlier coefficients to halve towards`, or `the sqrt link's valid range` when the failing test is η's |
| a perfect fit under an estimated dispersion (`fit.glm`, and `model.anova`'s full model) | E101 | §S3's text, `a perfect fit, its residual variance 0` |
| Poisson rates numerically 0 | E101 | `design is outside <w>'s domain: fitted rate numerically 0 at row 4` |
| separation | E101 | `design is outside <w>'s domain: fitted probabilities numerically 0 or 1: X separates y` |
| a perfect fit (`fit.lm`) | E101 | `design is outside <w>'s domain: a perfect fit, its residual variance 0` |
| an unknown residual type | E101 | `opts is outside <w>'s domain: type is "raw"` |
| μ′ underflowing to 0 in a step | E114 | RFC-0015's text, the expression `the working response at row 3, whose dμ/dη is 0` |
| a leverage within 10⁻¹⁰ of 1 (`model.influence`, `model.residuals` `"standardized"`) | E114 | RFC-0015's text, `the answer overflows a FLOAT: <expression>`, the expression naming the division, as `Cook's distance at row 3, whose leverage is 1` |
| a level the fit never saw (`model.predict`) | E101 | `frame is outside <w>'s domain: column "site" row 2 is "west", not one of the fit's levels` |
| a column of another kind than the fit coded (`model.predict`) | E101 | `frame is outside <w>'s domain: column "site" is string, where the fit coded a factor` |
| a prediction interval outside the Gaussian identity model | E101 | `opts is outside <w>'s domain: interval "prediction" needs a gaussian identity fit` |
| an interval's quantile not converging | E110 | RFC-0015's text, method `the quantile's polish` (`dist.rs`) |
| fits not nested | E101 | `small is outside <w>'s domain: <what differs>` |
| `test` `"F"` under a fixed dispersion | E101 | `opts is outside <w>'s domain: test "F" needs an estimated dispersion, and the poisson family's is 1` |
| the null deviance not finite (`model.anova` without an intercept) | E114 | RFC-0015's text, the expression `the null model's deviance at linkinv(offset)` |
| a refusal while `model.anova` fits a row | its own | its own text, then `, fitting the model through term "treatment"` or `, fitting the null model` |
| a malformed fit | E101 | `fit is outside <w>'s domain: <key> <what is wrong>`, as `vcov is 6 × 6, where coef has 7` |
| a fit whose `spec` codes other columns than its `names` (`model.predict`) | E101 | `fit is outside <w>'s domain: spec codes 7 columns, names has 6` / `spec codes column 3 as "site.north", names has "site.south"` |
| a non-finite prediction or `se` (`model.predict`) | E114 | RFC-0015's text, the expression `the prediction at row 2` / `the standard error at row 2` |
| a non-finite answer from finite operands, except the keys §S4 and §S6 leave out (§S1) | E114 | RFC-0015's text |

### S9. Tiers, audits and generated files

As RFC-0017 §S7: effects as §S1 declares, fixed for every word, none
calling a lambda; `PROMOTABLE.txt` regenerated; `UNVARIED`
(`crates/bund2-stdlib/src/lib.rs`, the audit array, 75 today) gains
exactly the natives the audit's own rule selects, a plain word of three
or more operands or a `.` form of two or more on the stack (the same
file, `varied_of`): `model.matrix` and
`model.matrix.` (four and three operands), and `model.predict`,
`model.predict.`, `model.compare` and `model.compare.` (three and
two); each with the operands the palette leaves at 7. The audit
compares the array with the registry's order (`assert_eq!`), so the six
names go where `declared_effects` lists them. `fit.lm` and `fit.glm`
are registered beside `fit.logistic`, and their four names join
`fit.rs`'s `WORDS`. `fit/tests.rs` counts every registered name that
starts with `fit.` against `WORDS` (its registration test), so that
unchanged test becomes their counting test, and a module of their own
with its own `WORDS` would fail it. The `model.` words are a module of
their own, with a counting test as RFC-0016's and RFC-0017's modules
have.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| `stat.lm`'s answers, refusals and texts | **Preserved exactly.** Its computation is factored into a crate-internal function that it and `fit.lm` call **[K15]**. Criterion 9's snapshot, recorded before M2, checks its bits; its tests alone, at 10⁻⁸, would not. Criterion 2 adds a bit-for-bit check against `fit.lm` |
| `fit.logistic`'s answers, refusals and texts | **Preserved exactly**; its code and `weighted_solve` are not changed. The model words solve through a new crate-internal function with the same QR and rank test (§S4) |
| `frame.matrix`, and every RFC-0017 word, their options and refusal texts | **Preserved exactly**; `frame/opts.rs` is not changed, and this RFC's keys are read by its own reader (§S1), so no frame word's text for any key moves |
| `dist.rs`'s tails | **Preserved exactly**; read only |
| A program's own word of one of these names | **Preserved**: a program's word is found first (§Current behaviour) |
| `println` of a MATRIX, and of a MAP holding one | **Preserved exactly**: a design printed omits its `matrix` (§S1); no other answer holds a MATRIX |
| `bund2 words` and `bund2 effects` | **Changed by design**: the new names, forms and effects appear |
| `PROMOTABLE.txt` | **Regenerated**; no existing name line moves; its count line changes. The file already lacks three fixed-effect frame natives, `frame.aggregate`, `frame.join` and `frame.matrix`, which its writer lists (`crates/bund2-stdlib/src/lib.rs`, the `PROMOTABLE.txt` writer). The test compares entry lines only and still passes. M1's regeneration adds them beside the model words, and that is not a stray change |
| `fit/tests.rs` | **Unchanged in code, wider in reach.** Its registration test counts `fit.` names against `WORDS`, and its `no_operand_aborts` iterates `WORDS` (`fit/tests.rs`, `no_operand_aborts`), so with §S9's four names it also runs `fit.lm` and `fit.glm` on its palette, accepting only an E1nn or a shallow stack, as criterion 8 asks |
| `refuse.rs`'s `Refusal::Singular` | **Changed in shape, preserved in text.** It has no fields today, and its text is `the matrix is singular`. E106 naming a column (§S8) needs a field or a new variant. Every existing site keeps that text exactly, and `refuse.rs`'s tests and every word's E106 test pass unchanged |
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
- **R's warnings kept as warnings**: non-convergence, fitted
  probabilities or rates numerically 0, and a null-model refit that
  does not converge (`glm.R:117-118`, `:319`, `:322-329`). R answers
  after each one. This RFC refuses each, as `fit.logistic` already
  refuses non-convergence and separation, since an answer R flags this
  way is not one a program should go on with. A program porting from R
  meets a refusal where R printed a warning.
- **Every column kind accepted** (a string column coded as a factor of
  its sorted values, as R's `model.frame` does) **[K7] B**. One word,
  `frame.factor`, already does it, and an explicit factor is a choice of
  levels a model's baseline depends on.

## Acceptance criteria

Reference values are computed from the definitions (D260 C4), in
mpmath at 50 digits, and every quantity after an estimate by its
formula. The script and its output are new files beside RFC-0015's
fixtures, written as `crates/bund2-stdlib/src/biomath/fixtures/fit.py`
is written. Under D260 C4 D no R output is needed: the one algorithmic
choice that touches the answers, where IRLS stops, is §S4's own and is
defined here.
- **`fit.lm`**'s estimate is closed-form, the exact least-squares
  solution.
- **An IRLS fit's** reference is §S4's iteration itself, run at 50
  digits from the same start and stopped by the same rule: the
  stopping iterate, not the solution of the score equations. Under a
  non-canonical link the two differ by up to some 10⁻⁵ (§S4 **[K11]**),
  which no tolerance on the exact solution could absorb. The script
  also solves the score equations by Newton's method and records the
  gap, for a reader, not for a test. `iterations` and `halvings` must
  equal the script's. If any iteration's ratio in the 50-digit run lies
  within 10⁻¹² of the rule's 10⁻⁸, the f64 run could stop one step from it.
  The script reports that, and the case is replaced by another input.
  That margin assumes the f64 deviance is good to about 10⁻¹⁵. Off the
  `fit.logistic` path a binomial deviance through μ loses digits as |η|
  grows (1.8·10⁻⁹ relative at η = 20), so criterion 3's binomial cases
  keep every fitted |η| below 15, and the script checks it.
- **The script** needs mpmath. Its header records mpmath's version and
  how it was run, as `fit.py`'s header does.

A computed a matches its reference b when
|a − b| ≤ r·|b| + 10⁻¹⁰, with r = 10⁻⁹ for `fit.lm` and its functions
and 10⁻⁶ for an IRLS fit and every quantity `model.` words compute from
one, as `fit.rs`'s tests compare `fit.nls` with its fixture. The
absolute 10⁻¹⁰ is for quantities whose exact value is 0, which no
relative tolerance can match: Dobson's treatment coefficients, and the
deviance difference and statistic they give (criteria 3 and 6). A p
below 10⁻⁶ is compared to an absolute 10⁻¹².

1. **Design.** On the frame of `bund_examples/frames/` with `site` a
   factor and a logical column added: `"response" [ "dose" "site" [
   "dose" "site" ] ] dict model.matrix` answers the 7 × 6 MATRIX its seven complete rows give, written out in the test,
   (after `"na" "omit"`), `names` `[ (intercept) dose site.north
   site.south dose:site.north dose:site.south ]`, `assign`
   `[ 0 1 2 2 3 3 ]`, `dropped` 2; with `intercept` false, `site`
   alone codes three columns, and the interaction design codes `names`
   `[ dose site.east site.north site.south dose:site.north
   dose:site.south ]`, of full rank; a logical column codes one. The same call
   on the frame after `frame.filter` removes every `east` row codes
   `site` from `north` and `south` alone, `site.south` its one column,
   and `spec.levels` maps `site` to `[ north south ]`. Each row of §S8 for
   `model.matrix` is produced by a stated input with its exact text,
   duplicate names, a one-level factor, a repeated member and a
   reordered interaction among them. Two three-level factors' interaction
   codes its columns in R's order, the member whose column comes first
   in `terms` fastest: `[ "a" "b" [ "a" "b" ] ]` and
   `[ "b" "a" [ "a" "b" ] ]` give the two orders and names §S2 states.
2. **`fit.lm`.** Bit for bit with `stat.lm` as §S3 states, on
   `stat.lm`'s own test data and on criterion 1's design without
   factors (`df.residual` equal in value to `df`). Weighted and offset
   fits, and `r2`, `adj.r2`, `f`, `aic`, `bic`, `vcov`, `hat` and
   `null.deviance`, against the fixture. A Gaussian identity
   `fit.glm` of the same design gives the same `aic` to 10⁻⁹. A
   constant `y` without an intercept is fitted, and with one is
   refused. The intercept-only design answers `r2` and `adj.r2` 0 and
   no `f`, `f.df` or `f.p` key.
3. **`fit.glm`.** Against the fixture: the Poisson model of Dobson's
   randomised trial, R's own example
   (`src/library/stats/man/glm.Rd:335-340`: counts 18, 17, 15, 20, 10,
   20, 25, 13, 12, outcome and treatment factors of three levels), with
   its deviance, null deviance, AIC, BIC and dispersion; a Poisson rate model
   with a log offset, its null deviance that of the refitted
   intercept-plus-offset model; the binomial with proportions and trials as
   weights, under logit, probit and cloglog; a Gamma under inverse and
   log; an inverse Gaussian under 1/mu^2; a Gaussian under log. The
   binomial logit on 0/1 data agrees with `fit.logistic` as §S4 states.
   Step halving is exercised by a Poisson model under `identity`:
   x = 1, 2, …, 8 and y = 1, 1, 2, 0, 6, 2, 0, 6 (an intercept and x).
   Its first step is valid, its second leaves μ > 0 and is halved once,
   and it converges at the seventh iteration, `halvings` 1. That count
   is from a reimplementation of §S4's iteration written while revising
   (the session's scratchpad, not committed); the fixture script
   confirms it, and the estimate is checked against the fixture's
   stopping iterate. A model whose first step leaves the range is
   refused with §S8's text. A Poisson model under `sqrt` whose step
   leaves η ≤ 0 is halved, with an input the fixture script finds and
   the test states. A Poisson `y` with a non-integer value is fitted,
   with no `aic` or `bic` key; a binomial weight of 2.5 is refused.
   Separation under `logit` and under `probit`, a singular
   design (an interaction with an empty combination of levels, the
   E106 text naming its column) and each §S8 row for `fit.glm` are
   produced with their texts. Poisson rates numerically 0 are refused
   on §S4's input (counts 3, 5, 4, 0, an intercept, an offset of 0, 0,
   0, ln 10⁻¹⁶), and the zero group (counts 3, 5, 4, 0, 0, 0, an
   intercept and an indicator) is answered, its indicator's
   coefficient below −20.
4. **Residuals and influence.** Every `type`, `hat`, `std.residuals` and
   `cooks`, for criterion 3's Poisson and Gamma fits and criterion 2's
   weighted fit, against the fixture. A design with a factor level of
   one row is refused by `model.influence` and by `model.residuals`
   `"standardized"` with §S8's text, and its other residual types are
   answered.
5. **Prediction.** The fits are criterion 3's Dobson fit, its Gamma
   fit under `inverse`, criterion 2's Gaussian fit, and criterion 2's
   weighted fit. For each, against the fixture: `fit` and `se` on both
   scales and confidence intervals. For the Gamma fit, every `se` is
   positive and every `lower` below its `upper`. For the Gaussian and
   weighted fits, prediction intervals too, the weighted fit's with and
   without `weights` for the new rows. A prediction at one new row of
   criterion 1's factor design, its `site` cell a single level, is
   answered. An unseen level and a prediction interval on the Poisson
   fit are refused with their texts. On criterion 3's zero-group fit, a
   response-scale confidence interval in the zero group answers no
   `upper` key and its `lower`, ε (2.220446049250313·10⁻¹⁶) through the
   clamped `log` inverse. On a Gamma `inverse` fit, a new row
   whose link-scale interval contains 0 is refused with §S8's text, and
   `type` `"link"` answers it.
6. **Comparison.** `model.compare` of the Dobson model without and with
   `treatment` gives the LR test by default, and of two Gamma fits the F
   test by default, each against the fixture, and `model.anova` of a
   Gamma design gives F on the full model's residual df; the first has
   `deviance` and `statistic` 0 within the absolute floor and `p` 1.
   `model.anova` of the Dobson design (`"outcome"` then `"treatment"`,
   Poisson) answers this frame, its numbers against the fixture:

   | `term` | `df` | `deviance` | `resid.df` | `resid.deviance` | `statistic` | `p` |
   |---|---|---|---|---|---|---|
   | `(intercept)` | NONE | NONE | 8 | the null deviance | NONE | NONE |
   | `outcome` | 2 | Δ | 6 | | Δ | |
   | `treatment` | 2 | 0 | 4 | as the row above | 0 | 1 |

   Of the same design without an intercept, the first row is `NULL`,
   `resid.df` 9, and `outcome` then has `df` 3. Fits that are not
   nested are refused with each text, and so is `test` `"F"` on the
   Poisson pair.
7. **Example.** `bund_examples/models/` fits, checks, predicts and
   compares a model on a frame, prints `expected.txt`, and a test in
   `crates/bund2-cli/tests` compares the two, as `bund_examples/frames/`
   does.
8. **No abort.** Every name and form, on a palette of NaN, ±∞, wrong
   types, empty, ragged, NONE-filled and malformed frames, designs and
   fits, answers or refuses (among them a design with offsets but no
   `spec.offset`, a fit whose lengths disagree, and a fit whose `spec`
   codes a column its `names` lack) with an E1nn, a shallow-stack error or a
   bridge error.
9. **Health.** `cargo xtask conform` prints 143/159 and CEILING 143/159;
   `coverage` IMPLEMENTED 597/602 and COVERAGE 577/602; clippy and
   `cargo deny` are clean; `PROMOTABLE.txt` is regenerated with no
   existing name line moved; the `UNVARIED` audit passes with §S9's
   names; `fit.rs`'s and `infer.rs`'s tests pass unchanged; and
   `stat.lm`'s and `fit.logistic`'s answers match, f64 bit for f64 bit,
   a snapshot recorded before M2 changes any code. That snapshot is
   recorded as RFC-0016 recorded `infer.snapshot`
   (`crates/bund2-stdlib/src/biomath/infer/tests.rs`, the comment above
   `SNAPSHOT`), over `stat.lm`'s and `fit.logistic`'s test programs,
   with one difference. `infer.snapshot`'s writer records the first
   float of each answer only (`floats(&v)[0]`), which sufficed for its
   scalar keys. This snapshot records every element of each LIST, in
   order, so every slope's bits are checked as well as the intercept's.
   `infer.snapshot` itself is unchanged. The existing tests could not
   see a change in the last bits: `stat.lm`'s compare at 10⁻⁸ (`r2` at
   10⁻¹², `infer/tests.rs`, `lm_against_the_fixture`), and
   `fit.logistic`'s at 10⁻⁶ (`fit/tests.rs`, `logistic_against_glm`).

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

**Ruled 2026-11-20 (D261):** the owner accepted every recommendation,
"rule on K1–K16: recomendations". The "Recommended" column below is what
was ruled. The **[author]** choices that follow were not part of that
ruling. The owner ruled them as written on 2026-11-20 (D262), "rule on
the author choices: recommendations".

| | Question | Options | Recommended |
|---|---|---|---|
| **K1** | Scope | **A** research 07's Phase C row: design, `fit.lm`, `fit.glm` (five families), residuals, influence, prediction, F and LR tests, the type I `anova`. **B** A, plus robust (HC0–HC3) standard errors and variance inflation, both in §3.3's table. **C** design and `fit.glm` only, the rest to another RFC | **A**: research 07's Phase C row as drawn (`07-statistical-research.md:218`), with BIC added beside AIC (§S3, §S4). It narrows §3.3's table (`:118-127`) in six places, each for ERRATA on acceptance: variance inflation (the diagnostics row), HC0–HC3 standard errors, anova types II and III, the quasi families and the negative binomial (K9), and `fit.logistic` not becoming one of `fit.glm`'s cases (K15). Each is a self-contained addition for a later RFC |
| **K2** | Names | **A** `model.matrix`, `fit.lm`, `fit.glm`, `model.residuals`, `.influence`, `.predict`, `.compare`, `.anova`. **B** everything under `fit.`. **C** `stat.anova`, `stat.predict` and the like under `stat.` | **A**: D260 C5's rule, a new prefix for a new kind of operand (a design, a fit), `fit.` beside `fit.logistic` |
| **K3** | How a frame reaches a fit | **A** a design MAP made once by `model.matrix`, which the fit words take. **B** the fit words take `frame response terms` themselves | **A**: inspectable, reusable, and the shape `model.anova`'s refits need |
| **K4** | Coefficient names | **A** R's: `(Intercept)`, `sitesouth`, `dose:sitesouth`, `treatedTRUE`. **B** `(intercept)`, `site.south`, `dose:site.south`, `treated.true` | **B**: RFC-0017's `<column>.<statistic>` style and Bund's lowercase `true`. Either way two columns can end up with one name; §S2 refuses that, because §S7 tests nesting by name |
| **K5** | Factor coding | **A** treatment contrasts, the first level the baseline, no option. **B** A, with a `contrasts` option for sum and Helmert coding | **A**: R's default; the baseline is chosen by the levels' order, which `frame.factor.levels` sets |
| **K6** | Terms and interactions | **A** terms in the order given; an interaction needs each member as an earlier term. **B** R's marginality rules, coding an interaction without its main effects by R's algorithm | **A**: every design B allows that A refuses is unusual, and A's coding is the one a reader expects. Only the members are required, not the lower interactions (§S2) |
| **K7** | Column kinds | **A** numeric and logical coded, a factor by contrasts, a string column refused. **B** a string column coded as a factor of its sorted values | **A**: the baseline is a choice the program should make with `frame.factor` |
| **K8** | A rank-deficient design | **A** E106, as `stat.lm`, naming the first dependent column. **B** R's: the aliased column's coefficient NONE | **A**: a design whose factors have unused levels is not the cause, since §S2 drops those as R does. What remains, such as an interaction with an empty combination, is a mistake in the design that the text names |
| **K9** | Families and links | **A** the five families with R's `okLinks` (cauchit included). **B** canonical links only. **C** A plus the quasi families and the negative binomial | **A**: R's set; quasi families and the negative binomial (§3.3's second row) need their own estimation of φ or θ |
| **K10** | A binomial response | **A** 0/1, or a proportion with the trials as `weights` (R's one-column form). **B** A, plus a `trials` option naming a column of counts | **A**: one form to learn; a program makes the proportion with `frame.with` |
| **K11** | IRLS control | **A** R's iteration and step halving, ε 10⁻⁸, at most 100 iterations. **B** R's 25 | **A**: `fit.logistic`'s cap and stopping rule, so K15's agreement holds. The rule leaves the estimate some 10⁻⁸ from exact, so criterion 3 compares IRLS estimates at a relative 10⁻⁶. One more step after convergence would tighten that but break the agreement with `fit.logistic`. *Corrected 2026-11-20 (third review, B1): the 10⁻⁸ holds under a canonical link only, so criterion 3 compares with §S4's own iteration (§S4, D261's note)* |
| **K12** | A non-integer binomial count | **A** E101 when w·y is more than 10⁻³ from an integer (R's threshold). **B** a warning, as R | **A**: R's binomial AIC rounds m·y (`src/library/stats/R/family.R:359`), so on such counts R answers a wrong AIC and only warns. The cost: a fractional response, which R fits with a warning, is refused, and K9 offers no quasi-binomial in its place |
| **K13** | `model.compare`'s and `model.anova`'s default test | **A** R 4.4's: F when the dispersion is estimated, LR when fixed. **B** always LR | **A** |
| **K14** | NONE | **A** `model.matrix` and `model.predict` take `na`; nothing else sees NONE. **B** the fit words take `na` too | **A**: D252's rule, where the cells are read |
| **K15** | `stat.lm` and `fit.logistic` | **A** both unchanged; `fit.lm` shares `stat.lm`'s computation, bit for bit; `fit.glm` agrees with `fit.logistic` to 10⁻¹². **B** `fit.logistic` rewritten on `fit.glm` | **A** |
| **K16** | Intervals from `model.predict` | **A** confidence intervals on the link scale mapped through the inverse link; prediction intervals for Gaussian identity fits only. **B** no intervals, `se` only, as R's `predict.glm` | **A**: an interval is what a program wants from a prediction. A weighted fit's prediction interval takes each new row's weight from a `weights` column, default 1.0, as R's `predict.lm` assumes (§S6) |

Choices marked **[author]** where they stand, ruled as written (D262):

- the default residual type `"deviance"` (§S5);
- a leverage of 1 refused, where R answers NaN (§S5);
- nesting checked by family, link, `y`, `weights`, `offset` and names
  (§S7);
- `model.anova`'s first row `(intercept)`, or `NULL` without one, and
  its test columns NONE there (§S7);
- weights strictly positive, where R keeps a zero-weight row out of the fit (§S2). A weight of 0 is a common way to set a row aside, so the refusal's text should point to `frame.filter`; the alternative is R's behaviour;
- a design's NONE checked column by column in the order response,
  terms, `weights`, `offset` (§S2);
- `fit.lm`'s `opts`, taking no key yet (§S3);
- a constant `y` fitted without an intercept and refused with one (§S3);
- Poisson rates numerically 0 refused, where R warns (§S4);
- a null-model refit that does not converge refused (E110), where R
  warns (§S4);
- the first IRLS step out of range refused, as R stops (§S4);
- the inverse Gaussian's valid μ > 0, where R accepts any μ (§S4);
- separation refused under `probit`, `cloglog`, `cauchit` and `log` at
  R's 10ε test, where R warns; `fit.logistic`'s precedent covers only
  the logit (§S4);
- a group of zero Poisson counts answered with its large coefficient,
  as R answers it, the rate refusal catching only a μ below 10ε (§S4);
- a non-integer Poisson `y` fitted with `aic` and `bic` left out,
  rather than E101 or the continuous extension (§S4), the third
  review's SF9;
- a binomial weight more than 10⁻³ from an integer refused (§S4);
- a non-finite null deviance left out of the fit, and `model.anova`
  refusing it with E114 (§S4, §S7);
- μ′ underflowing to 0 refused with E114, where R drops the
  observation from the step (§S4);
- a term reading the response, and `weights` or `offset` naming it or
  one column, refused (§S2);
- `model.matrix`'s order of refusals (§S2);
- `model.predict` requiring each column's kind as coded (§S6);
- `test` `"F"` refused under a fixed dispersion, where R warns (§S7);
- a refusal while `model.anova` fits a row passed on, naming the row
  (§S7).

Added by the fourth revision, after D262, and ruled as written on 2026-11-20 (D263):
- `model.anova`'s Gaussian identity rows: §S3's constant-`y` refusal
  not applied, a full model of deviance 0 refused with the perfect-fit
  text (§S7);
- a `fit.glm` perfect fit under an estimated dispersion refused with
  §S3's text, E101 (§S4);
- a start with a non-finite η refused, as R stops (§S4);
- the `fit.logistic` path chosen by the values of the weights and
  offset (§S4);
- off that path, z formed from the unclamped linear predictor, as R
  forms it (§S4);
- a logical column coded as one column, never fully, where R codes it
  as a two-level factor; this follows from K7 (§S2).

Added by the fifth revision, after D263, and ruled as written on 2026-11-20 (D264):
- a response-scale prediction or interval end outside the link's valid
  η refused, `type` `"link"` answering it, where R applies `linkinv`
  (§S6);
- an infinite interval end leaving its whole LIST key out, D251's A3
  applied to a LIST (§S6);
- an interval end outside the family's valid μ answered as computed
  (§S6);
- an interaction's members distinct (E101), and two interactions of the
  same members one term (§S2);
- an interaction's columns in R's order, the first member fastest
  (§S2);
- `model.predict`'s `type` defaulting to `"response"`, where R's
  defaults to `"link"` (§S6);
- a non-finite coefficient from a solve refused with E114, where R
  stops with a warning (§S4).

Restated by the sixth revision, after D264, and ruled as written on 2026-11-20 (D265):
- an interaction's columns in R's variable order: its members taken in
  the order their columns first appear in `terms`, the first of them
  fastest, and its names joined in that order (§S2). D264 ruled "R's
  order" on the premise that R varies the member written first
  fastest; R does so only when the interaction writes its members in
  that order. The alternative is the order as written, which differs
  from R's for `[ "b" "a" [ "a" "b" ] ]`.

On acceptance, ERRATA also records two refinements of research 07
beside K1's six narrowings:
- §3.2 says `model.matrix` answers "a design MATRIX"
  (`07-statistical-research.md:111`), and §S2 answers a MAP holding one;
- §3.2 asks for "Weights and offsets for every model word" (`:116`),
  and `stat.lm` and `fit.logistic` take neither, unchanged under K15.

None is open in `docs/registers/open-questions.md`: R's behaviour is
read from its source, and every Bund2 behaviour cited is read or probed.
