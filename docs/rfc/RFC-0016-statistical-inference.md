# RFC-0016: Statistical inference — intervals, effect sizes, correlation tests, rounding and resampling primitives

- Status: **Accepted** (2026-11-17) by the repository owner ("accepting
  RFC-0016"), at revision 6. Accepting means D251's A1–A9 as ruled and
  amended; S1–S8 as specified; the milestones M1–M5 built in order, each
  gated by its criteria and by criterion 12; and the 64 words as Bund2's
  own (D72), as RFC-0015's family is.
- Drafted 2026-11-16 on the owner's "RFC for
  Phase A", after `docs/research/07-statistical-research.md`. Revisions
  2 to 6 answer `docs/rfc/reviews/RFC-0016-review-2026-11-16.md`,
  `…-2.md`, `…-3.md`, `…-4.md` and `…-5.md`; §Revision 2 to §Revision 6
  map each finding to the change it made. Its owner
  questions are D251's A1–A9, **RESOLVED 2026-11-16**: A1 and A3–A8 as
  recommended (**A**), A2 re-ruled **C** after the first review's B1, A9,
  raised for its B2, ruled **A** and then re-ruled **C** in its k after
  the second review's S1, and A7's wording confirmed after that review's
  S7; after the third review, A3 confirmed on a corrected premise (B1) and
  A9's exact branch widened (S1); after the fourth, the floor's shortfall
  corrected and accepted (S1); after the fifth, D251's figures corrected
  without a new ruling (D251's amendments). Every passage that rests on one is marked
  **[A*n*]**.
- Depends on: RFC-0015 (the `biomath` family: its operand readers, S3's
  forms, S8's generator, S11's refusals), RFC-0004 (effects)
- Decisions consumed: D251 (A1–A9), D252 (context only), D21, D24, D36,
  D37, D38, D39, D72, D111, D250 (B3, B4, B5, B10, B11, B12, B15); F123.
  For what each governs: D24 and D111 the `.` forms; D36 how a refusal is
  reported; D37 that no word panics (S7's empty-list refusal); D38 the
  pinned `rand` 0.8.7 whose calls S7 names; D39 the work caps; B3 that
  series are LISTs read by `get_data`; B4 the VM's generator. B11 ruled
  RFC-0015's reference values (R 4.4.1 fixtures); this RFC's are A7's
  instead.
- Reference SHA: `reference/rust_multistackvm`
  4605832678d4792c2b8e0564b7757d5aee1539a4; `reference/Bund`
  21b40b0213a76f3f39bd6de0d2bdfdf0a53b0dca; `reference/rust_dynamic`
  ceb27c96fa10c4edb09106f3f904f22f94d63119. Bund2's own code is cited by
  file and symbol, at `development/0.56.0`.
- Supersedes: nothing. Grounds and refines research 07's Phase A (§4).

## Summary

RFC-0015 gave Bund2 the classical tests, but each answers only a
statistic and a p-value, two-sided. Statistical research reports
estimates, confidence intervals and effect sizes, tests one-sided
hypotheses, and prints its numbers to a stated precision. This RFC adds,
as words of Bund2's own beside RFC-0015's family:

- estimates and confidence intervals for seven of RFC-0015's tests, and
  one-sided alternatives for eight, through new `.opts` variants **[A2]**;
- correlation tests;
- the Hochberg, Hommel and Benjamini–Yekutieli corrections;
- effect sizes for two samples and for 2×2 and r×c tables;
- rounding to decimal places and significant figures, and fixed and
  scientific number formatting;
- shuffling and sampling from the VM's seeded generator.

**No existing word changes** in operands, effect, answer, refusals or
cost **[A2]**. No data representation changes, so D252 does not bear on
it. 64 names are added.

## Motivation

- **Tests answer no interval.** Every RFC-0015 test answers a MAP of
  `statistic`, `p`, `df` and `method`
  (`crates/bund2-stdlib/src/biomath/infer.rs`, `answer`). A reported
  result needs an estimate and its interval. R's `t.test`, `binom.test`
  and `fisher.test` print one, and `wilcox.test` does with
  `conf.int = TRUE` [UNGROUNDED: R is not in `reference/`; Q62].
- **Every test is two-sided.** RFC-0015 §S6: "Each is two-sided". A
  directional hypothesis, the norm in a pre-registered study, cannot be
  tested.
- **Correlation has no test.** `stat.cor.pearson` and its siblings answer
  the coefficient alone
  (`crates/bund2-stdlib/src/biomath/describe.rs`, `pearson` and
  `stat_cor_pearson`). RFC-0015's criterion 9 named `cor.test`, and the
  M5 note under its §Acceptance criteria records that no word answers it.
- **Numbers cannot be printed to a precision.** `math.round` rounds to an
  integer (`reference/rust_multistackvm/src/stdlib/math/float_math.rs:57-58`,
  `:127`). `format` fills a template with each value's STRING conversion
  (`reference/rust_multistackvm/src/stdlib/string/format.rs:23-35`), which
  for a FLOAT is `dtoa`'s shortest form
  (`reference/rust_dynamic/src/conv.rs:694-696`, `:127-130`), with no
  precision. RFC-0015's example had to define
  `{ 1000000.0 * math.round 1000000.0 swap / }`
  (`bund_examples/biomath/demo.bund`, `:round`).
- **Resampling has no primitives.** A bootstrap or a permutation test
  needs a shuffle and draws with and without replacement. The seeded
  drawing words are `rng.uniform`, `dist.sample`, `dist.sample*`,
  `popgen.wf*` and `sim.gillespie` (RFC-0015 §S8); none draws from a
  LIST.

## Current behaviour

### What the reference does

- **`math.round`** pulls one value, casts it to FLOAT and pushes
  `f64::round` of it (`reference/rust_multistackvm/src/stdlib/math/float_math.rs:93-100`,
  `:126-128`), registered as `math.round` (`:176`). `f64::round` rounds
  half away from zero, so `2.5 math.round` is 3.0 (probed). The cast does
  not convert an INTEGER, so `5 math.round` is
  `FLOAT_OP returns error: This Dynamic type is not float: 2`
  (`:99`, `:155-156`; probed). It takes one operand and no places.
- **`format`** pulls a template, then one value per distinct key, each
  converted to a STRING (`format.rs:13-50`), and renders with `leon`
  (`:51-57`); a FLOAT converts to `dtoa`'s shortest form (`conv.rs:127-130`).
  `format.` takes the template from the workbench (`format.rs:72-132`,
  registered `:136`).

### What Bund2's own family does

- **Test answers.** `answer(statistic, p, df, method)` builds every
  test's MAP: `statistic`, `p` clamped to [0, 1], `df` when given, and
  `method` (`infer.rs`, `answer`). The two-sided p of a t statistic is
  twice the lower tail at −|t| (`infer.rs`, `t_p`).
- **Tests' operands and forms** are RFC-0015 §S6's table and its M5 note:
  `stat.test.t1` (series μ₀), `t2`, `welch`, `paired`, `chisq`,
  `mannwhitney`, `ks2` (two LISTs or METRICS), `chisq.table` and
  `fisher` (a MATRIX), `binom` (k n p); each with a plain and a `.`
  form; `t1` opaque, the rest fixed. The Mann–Whitney test is the normal
  approximation with tie and continuity correction (`infer.rs`,
  `mannwhitney`). Its correction at W = n·m/2 was fixed on 2026-11-16 to
  `sign(z)·½`, which is 0 there (RFC-0015 §S6's note of that date); that
  this is R's correction is [UNGROUNDED; Q62].
- **Corrections** take one series in four forms and answer in input
  order (`infer.rs`, `bonferroni`, `holm`, `bh`).
- **Distributions.** `dist.rs`'s `Family` answers each family's tails and
  quantiles with Bund2's own code over RFC-0015 §S4's functions; `statrs`
  checks parameters and draws (`crates/bund2-stdlib/src/biomath/dist.rs`,
  the module doc, "What is `statrs`'s and what is Bund2's"). `tails` is
  `pub(crate)`; `quantile` is private, polished by bracketed Newton to
  |cdf(x) − p| ≤ 10⁻¹², at most 100 steps, else E110 (`dist.rs`,
  `quantile`, `POLISH_CAP`, `POLISH_TOL`).
- **The generator.** `with_rng` runs a closure over the VM's `StdRng`
  (`crates/bund2-stdlib/src/biomath/rng.rs`, `with_rng`); `rng.seed`
  reseeds it (RFC-0015 §S8).
- **Forms.** A `.` form built by `bench!` takes one operand from the
  workbench and answers on the workbench, declaring `consumes − 1` from
  the stack (`crates/bund2-stdlib/src/wb.rs`, `bench!`).
- **Refusals** are RFC-0015 §S11's E101–E115. A LIST cell that will not
  convert is E103, a non-finite one E101 (`operand.rs`, `whole` and
  `finite_all`). No word answers NaN or ±∞ from finite operands (B12).
- **The audits.** A fixed-effect native that consumes more operands than
  the palette varies, two for a plain word and one for a form, must be
  listed in `UNVARIED` with a reason (`crates/bund2-stdlib/src/lib.rs`,
  the unvaried-operand audit, `varied_of`).
- **Registration tests count by prefix.** RFC-0015's criterion-2 tests
  assert that every `stat.test.`, `stat.padjust.` and `stat.lm` name is in
  `infer`'s `WORDS`, that every other `stat.` name outside `stat.count*`
  and `stat.mean*` is in `describe`'s, and that the `dist.` and `rng.`
  names number `2 × WORDS.len() + 3` (`infer/tests.rs`, `describe/tests.rs`,
  `dist/tests.rs`, each file's registration test).
- **Bund's number reader** reads `1.23e4` and `1.0e-7`, and reads `1e4`
  as the INTEGER 1 followed by an unknown word `e4` (probed).

### What is linked

Nothing this RFC needs is missing. Its tails and quantiles are `dist.rs`'s
own, which S2 uses after making `Family::quantile` `pub(crate)`.
Clopper–Pearson's interval is two beta quantiles; Hedges' correction uses
§S4's `lgamma`. Rust's float formatting rounds the exact binary value to
the requested places, ties to even: `{:.2}` of 0.125 is `0.12`, of 2.675
`2.67`, and `{:.0}` of 0.5, 1.5, 2.5 and 3.5 is `0`, `2`, `2`, `4` (run
with rustc 1.95.0, by the author and the review). No crate is added.

## Design

### S1. Shape

- **Bund2's own (D72),** in the default build **[A8]**, under RFC-0015's
  prefixes **[B5]**: `stat.` for tests, corrections and effect sizes,
  `rng.` for draws, `math.` for rounding, and `string.` for formatting
  **[A1]**. `bund2 words` lists each; neither health number moves.
- **Additive, strictly [A2].** No existing word changes. The reference's
  `math.round` (`float_math.rs:176`), `format` and `format.`
  (`format.rs:136`), and Bund2's D111 form `math.round.` (`wb.rs`,
  `ADDED`), are untouched, and RFC-0015's tests answer exactly what they
  answer today. A program that already defines one of the 64 new names
  keeps its own meaning: a program's word is found before a native of the
  same name, in the reference (`multistackvm_apply.rs:46-59`, a lambda
  before an inline) and in Bund2 (`crates/bund2-api/src/lib.rs`,
  `Registry::resolve_uncached`, `lambda` before `native`);
  `:math.abs { 42 } register -2.5 math.abs` answers 42 (probed).
- **Refusals** are RFC-0015 §S11's codes and texts; no new code. B12
  holds: no word answers NaN or ±∞ from finite operands. An estimate, an
  interval end or a statistic that is not finite from finite operands is
  E114, as RFC-0015's tests refuse a non-finite statistic (`infer.rs`,
  `statistic`) and its other words a non-finite result (`operand.rs`,
  `finite`): a Hodges–Lehmann difference (1e308 − (−1e308)), a t end, or
  `oddsratio`'s ad/(bc) can overflow. A key left out is only ever an
  infinite end **[A3]**, never an overflowed one. The refusal is the
  whole word's, as E111's is, so an `.opts` word that refuses answers no
  p either.
- **Forms** follow RFC-0015 §S3 **[B10]**: one-series words in four
  forms, other consumers plain and `.`, the `.` form taking from the
  workbench the operand the plain word has on top (D111's "anything else"
  row, as RFC-0015 §S3 chose for its family). Each `.` form answers on the
  workbench (`bench!`).
- **Operand order** is RFC-0015 §S3's: the deepest operand is the first
  parameter.
- **Options MAPs.** Keys are STRINGs; an unknown key, a key the word does
  not take, or a value of the wrong type or range is E101 naming the key,
  as `ode.rk45`'s options are (RFC-0015 §S15's M11 note). D252's future
  `na` key would be an addition to these MAPs. `conf` is a FLOAT: no
  INTEGER lies in (0, 1), so an INTEGER `conf` is E101. `mu` converts an
  INTEGER, as B15 converts operands.
- **Quantiles.** Every quantile this RFC uses, the normal's z included, is
  `dist.rs`'s `Family::quantile`, and so can be E110 as any polish can.

### S2. Estimates, intervals and alternatives [A2] [A3] [A4] [A9]

**The plain test words are unchanged.** Estimates and intervals come
only from eight new words, `<test>.opts`, each taking the plain word's
operands with an options MAP above them:

`stat.test.t1.opts`, `t2.opts`, `welch.opts`, `paired.opts`,
`binom.opts`, `fisher.opts`, `mannwhitney.opts`, `ks2.opts`.

`stat.test.chisq` and `chisq.table` gain none: they have no alternative
and no interval.

**Options.**

| Key | Values | Default | Words |
|---|---|---|---|
| `alternative` | `"two.sided"`, `"less"`, `"greater"` | `"two.sided"` | all eight |
| `conf` | a FLOAT in (0, 1) | 0.95 **[A4]** | all but `ks2.opts` |
| `mu` | a finite number, an INTEGER converting **[B15]**, the difference under H₀ | 0 | `t2`, `welch`, `paired`, `mannwhitney` |

`t1`'s μ₀ and `binom`'s p stay operands, as in the plain words. An empty
`dict` asks for the defaults.

**The answer** has the plain word's keys, `statistic`, `p`, `df` where
the plain word gives it, and `method`, with these differences:

- **`statistic` and `p`** are for the chosen `alternative` and `mu`: the
  t of the shifted hypothesis, W of a − μ against b, and for `ks2.opts`
  one-sided, D⁺ or D⁻ (the alternatives table below). With the defaults,
  two-sided and `mu` 0, they are the plain word's, bit for bit.
- **`method`** is the plain word's string (`infer.rs`, each test), except
  for two words: `fisher.opts`'s is "Fisher's Exact Test for Count Data;
  conditional maximum likelihood estimate of the odds ratio", and
  `mannwhitney.opts`'s is given under "Intervals" below. So the six
  others answer "One Sample t-test", "Two Sample t-test", "Welch Two Sample
  t-test", "Paired t-test", "Exact binomial test" and "Asymptotic
  two-sample Kolmogorov-Smirnov test".

And these keys are added:

| Word | `estimate` | Interval (`lower`, `upper`) | Also |
|---|---|---|---|
| `t1.opts` | the mean | t interval for the mean | `conf`, `alternative` |
| `t2.opts` | mean(a) − mean(b) | pooled t interval for the difference | `conf`, `alternative` |
| `welch.opts` | mean(a) − mean(b) | Welch interval, Welch–Satterthwaite df | `conf`, `alternative` |
| `paired.opts` | the mean of a − b | t interval for it | `conf`, `alternative` |
| `binom.opts` | k/n | Clopper–Pearson | `conf`, `alternative` |
| `fisher.opts` | the conditional MLE of the odds ratio | the exact conditional interval | `conf`, `alternative` |
| `mannwhitney.opts` | the Hodges–Lehmann shift **[A9]** | the order-statistic interval **[A9]** | `conf`, `alternative` |
| `ks2.opts` | — | — | `alternative` |

R reports both means for `t.test`'s two-sample forms; Bund2 answers their
difference, the quantity the interval is for [UNGROUNDED as R's; Q62].
With a non-zero `mu`, the p is of the shifted hypothesis while the
estimate and interval are of the difference itself.

**The alternatives.** "less" and "greater" name H₁:

| Word | H₁ for "less" | H₁ for "greater" |
|---|---|---|
| `t1.opts` | mean < μ₀ | mean > μ₀ |
| `t2.opts`, `welch.opts`, `paired.opts` | mean(a) − mean(b) < μ | > μ |
| `binom.opts` | the true p < the operand p | > |
| `fisher.opts` | the odds ratio < 1 | > 1 |
| `mannwhitney.opts` | a is shifted below b + μ | above |
| `ks2.opts` | the cdf of a lies below that of b: D⁻ = max(F_b − F_a) | the cdf of a lies above that of b: D⁺ = max(F_a − F_b) |

`ks2.opts`'s "greater" therefore means a is stochastically *smaller*, as
R's `ks.test` documents it [UNGROUNDED; Q62]; Bund2 follows R's sense so
that a program ported from R means the same thing.

**One-sided p.**
- **t tests:** one tail of t at the statistic.
- **binomial:** P(X ≤ k) for "less", P(X ≥ k) for "greater", exact.
- **Fisher:** P(X ≤ x) or P(X ≥ x) under the hypergeometric, x the first
  cell, exact.
- **Mann–Whitney:** z = (W − nm/2 − c)/σ, with c = +½ for "greater" and
  −½ for "less", σ as the plain word's; p is the upper normal tail of z
  for "greater" and the lower for "less" [UNGROUNDED as R's; Q62].
- **KS:** exp(−2·nm/(n + m)·D²) at D = D⁺ for "greater" and D⁻ for
  "less", the asymptotic one-sided tail [UNGROUNDED as R's; Q62].

**Intervals.** At level conf, two-sided with α = 1 − conf split evenly,
one-sided with α on one end. A one-sided interval keeps the end that
bounds H₁'s side: [lower, ∞) for "greater", (−∞, upper] for "less", the
infinite end left out **[A3]**.
- **t:** estimate ± t₁₋α/₂ · se on the test's df.
- **Clopper–Pearson:** lower = B⁻¹(α/2; k, n − k + 1), upper =
  B⁻¹(1 − α/2; k + 1, n − k). At k = 0 the lower end is 0, and at k = n
  the upper is 1, by definition, without calling the quantile at a zero
  shape.
- **Fisher:** ψ̂ solves E[X; ψ] = x, the mean of Fisher's noncentral
  hypergeometric at odds ratio ψ. The lower end solves P(X ≥ x; ψ) = α/2,
  the upper P(X ≤ x; ψ) = α/2. Each is found on log ψ: bracketed by
  doubling from [−1, 1], at most 64 doublings, then bisected to 10⁻¹⁰
  relative in ψ, at most 200 steps; either cap is E110. The noncentral
  masses are evaluated in log space, the largest log-mass subtracted
  before exponentiating, as `stat_test_fisher` does for the central case
  (`infer.rs`), so no ψ along the bracket overflows. At x the support's
  minimum, ψ̂ = 0 and the lower end 0 are answered; at its maximum, ψ̂ and
  the upper end are infinite and are left out **[A3]**.
- **Mann–Whitney [A9]:** with D₍₁₎ ≤ … ≤ D₍ₙₘ₎ the sorted differences
  aᵢ − bⱼ (n = |a|, m = |b|), the estimate is their median, the mean of
  D₍ₙₘ/₂₎ and D₍ₙₘ/₂₊₁₎ when nm is even, formed as (D₍ₙₘ/₂₎ +
  D₍ₙₘ/₂₊₁₎)/2 when that sum is finite and as D₍ₙₘ/₂₎/2 + D₍ₙₘ/₂₊₁₎/2
  when it is not, so that a finite median of finite differences is never
  refused (1.5e308 and 1.6e308 give 1.55e308), and the two-sided interval is
  [D₍ₖ₎, D₍ₙₘ₊₁₋ₖ₎]. Its k is:
  - **exact** when the n + m values of a and b, before any `mu` shift,
    have no ties, equality being f64 `==` (so −0.0 and 0.0 tie, and
    values one ulp apart do not), and C(n + m, n) < 2¹²⁸ (every
    min(n, m) ≤ 6 up to the cap on n·m; min(n, m) = 7 from a larger
    sample of 1 080 104 is not, whichever operand it is; a balanced
    n = m up to 65): the largest k in 0 … nm + 1 with
    P(U ≤ k − 1) ≤ (α/2)·(1 + 10⁻⁷), U the
    count of pairs with aᵢ > bⱼ under H₀. The factor is Fisher's `REL_ERR` (`infer.rs`), so that a
    level read from a decimal, 0.9 whose f64 half-complement is just below
    1/20, still admits P(U ≤ 0) = 1/20 at n = m = 3; the coverage it gives
    up is below 10⁻⁷·α. P(U ≤ u) is the count of the lower tail over
    C(n + m, n), the ratio taken in f64. The counts are the coefficients
    of the Gaussian binomial ∏_{i=1}^{s} (1 − q^{l+i})/(1 − q^i), s =
    min(n, m), l = max(n, m), truncated at degree ⌊nm/2⌋: each factor is
    applied in place as a stride-i running sum, ascending (the division),
    and then a stride l + i difference, **descending** (the
    multiplication); an ascending difference divides by 1 + q^{l+i}
    instead and goes wrong from u = 22 at n = m = 10. Above ⌊nm/2⌋ the
    counts are read by symmetry, c(u) = c(nm − u), so the lower-tail
    count L(u) = Σ_{v ≤ u} c(v) at u > ⌊nm/2⌋ is C(n + m, n) −
    L(nm − u − 1), with L(−1) = 0 at u = nm, exact in `u128`: a one-sided `conf` below ½ selects such a k (n = m = 3,
    "greater", conf 0.3: k = 6). Every intermediate is a count
    of at most C(n + m, n), so `u128` holds it exactly, and no
    intermediate is negative (checked against the recurrence
    c(n, m, u) = c(n − 1, m, u − m) + c(n, m − 1, u) for n, m ≤ 8, and at
    n = 60, m = 61; scratch `widen.py`).
  - **by the normal approximation** otherwise: k = ⌊nm/2 −
    z₁₋α/₂·√(nm(n + m + 1)/12)⌋. The floor is Bund2's choice, and the
    interval it gives is approximate. Inside the exact branch's sizes it
    would be too narrow at n = 2, m = 52 and 90% (0.8994), which is why
    that branch is wide. Where it is used without ties, a sample of 7 to
    66 values against one large enough to leave the exact branch, its
    coverage falls short of conf by up to 4.7·10⁻³ at 80% two-sided,
    4.0·10⁻³ at 80% one-sided, 2.3·10⁻³ at 90% one-sided, 8.1·10⁻⁴ at 90%
    two-sided and 4.1·10⁻⁴ at 95% one-sided, each worst at s = 7, the
    same to four digits from m = 1 080 104 to the cap; it is conservative
    at 95% and 99% two-sided and 99% one-sided; and the shortfall falls
    with s, at 80% two-sided to about 4.6·10⁻⁴ at s = 66 and 67, 3·10⁻⁴
    at 100 and 1·10⁻⁴ at 300 (D251's amendments; the fifth review's
    figures, scratch `rev6_floor2.py`; accepted). With ties no exact distribution
    applies. The √ has no tie
    term, unlike the test's σ (`infer.rs`, `mannwhitney`): the interval is
    defined over the differences, not the ranks.

  One-sided, α replaces α/2 and the one end above is kept. k < 1 or
  k > nm is E101 (`a and b`: too few for the level): k > nm comes of a
  very low one-sided `conf`, at most about 10⁻⁷ in the exact branch, or,
  in the normal branch, 0.005 at n = m = 3, where z₁₋α < 0 gives k = 10. The p is the plain word's normal
  approximation in both cases. This is Bund2's own definition, not R's,
  and the `method` says so: "Wilcoxon rank sum test with continuity
  correction; Hodges–Lehmann estimate, order-statistic interval".

**Infinite ends [A3].** A one-sided t or Mann–Whitney interval's far end
is infinite and is left out; so are Fisher's at the support's maximum.
Clopper–Pearson's far end is 0 or 1, Fisher's lower end 0, and a
one-sided Pearson interval's far end −1 or 1 (S3), which are finite and
answered. `get` on an absent key is an error, `GET returns error: Key
not found: upper` (`Key not found` from
`reference/rust_dynamic/src/get.rs:10-12`, the prefix from the word's
bail, `reference/rust_multistackvm/src/stdlib/values/value_dict.rs:62-67`;
probed), so
a program tests `"upper" ?key` before it gets the end **[A3]**, confirmed
on that premise (D251's amendment).

**Work (D39).** `mannwhitney.opts` forms n·m differences; n·m above 10⁷
is E111 before allocating (80 MB of f64 at the cap), and the refusal is
the whole word's, so above the cap a one-sided p has no word either
(§Alternatives). Its exact k takes at most s·(⌊nm/2⌋ + 1) `u128`
additions and as many subtractions, s ≤ 65 and nm ≤ 10⁷, so below
3.3·10⁸ of each, and holds ⌊nm/2⌋ + 1 `u128`, 80 MB at its largest
(nm = 10⁷, which is exact for s = 1, 2, 4 and 5), beside the 80 MB of differences. `fisher.opts` finds up to three roots, ψ̂ and the two
ends, each by its two starting ends, at most 64 doublings and 200
bisection steps, each evaluation one sum over the support: at most 798
times the plain word's work (266 per root), whose
support is already capped at 10⁸ (`infer.rs`, `SUPPORT_CAP`). A
quantile's polish can be E110, as `dist.quantile`'s is.

**Forms.** Each `.opts` word has a plain and a `.` form; the `.` form
takes the options MAP from the workbench. `stat.test.t1.opts` reads its
series by `get_data` and is opaque, as `stat.test.t1` is; the rest have
fixed effects.

### S3. Correlation tests

`stat.cor.test.pearson`, `stat.cor.test.spearman` and
`stat.cor.test.kendall` (`a b -- MAP`), and their `.opts` variants taking
`alternative` ("greater" is positive association) and, for Pearson,
`conf`. Each answers `estimate`, `statistic`, `p`, `method` and
`alternative`, and:

- **Pearson:** t = r·√((n − 2)/(1 − r²)) on n − 2 df (`df`), and, for
  n ≥ 4, Fisher's z interval tanh(atanh r ± z₁₋α/₂/√(n − 3)) (`lower`,
  `upper`, `conf`); for n = 3 no interval and no `conf`
  [UNGROUNDED as `cor.test`'s; Q62].
- **Spearman:** ρ, the statistic S = (n³ − n)(1 − ρ)/6, and the p of the
  t approximation on n − 2 df **[A5]**.
- **Kendall:** τ-b, as `stat.cor.kendall`.
  - **Exact, with no ties in either series and n < 50:** `statistic` is
    T, the count of concordant pairs. Under H₀, T has the distribution of
    the number of non-inversions of a uniform permutation of n, counted by
    the recurrence I(n, t) = Σ_{j=0}^{n−1} I(n − 1, t − j), I(1, 0) = 1,
    over n!. "greater" is P(T ≥ t), "less" P(T ≤ t), and two-sided is
    min(1, 2·P(T ≥ t)) when t > n(n − 1)/4, else min(1, 2·P(T ≤ t)).
    The counts outgrow `u128` from n = 36, and n! from n = 35, so the
    recurrence is carried in f64 as probabilities, P(n, t) =
    (1/n)·Σ_{j=0}^{n−1} P(n − 1, t − j), P(1, 0) = 1, each sum formed
    directly: every term is non-negative, and no count is formed. Each
    tail is summed on its own side, P(T ≥ t) read by symmetry as
    P(T ≤ n(n − 1)/2 − t), never as 1 − P(T ≤ t − 1); neither that nor a
    sliding-window sum is used, since both cancel in a far tail (relative
    errors past 10³⁸ at n = 45; the fourth review's N6).
  - **Otherwise, the normal approximation:** `statistic` is z = S/√v with
    S = n_c − n_d and, for tie groups of sizes tᵢ in a and uⱼ in b,
    v = (v₀ − v_t − v_u)/18 + v₁/(2n(n − 1)) + v₂/(9n(n − 1)(n − 2)),
    where v₀ = n(n − 1)(2n + 5), v_t = Σ tᵢ(tᵢ − 1)(2tᵢ + 5), v_u = Σ
    uⱼ(uⱼ − 1)(2uⱼ + 5), v₁ = Σ tᵢ(tᵢ − 1) · Σ uⱼ(uⱼ − 1), and v₂ =
    Σ tᵢ(tᵢ − 1)(tᵢ − 2) · Σ uⱼ(uⱼ − 1)(uⱼ − 2). No continuity correction.
    p is the normal tail, doubled for two-sided.

  These definitions are Bund2's; that R's `cor.test` computes the same is
  [UNGROUNDED; Q62]. Its pair loop is `stat.cor.kendall`'s, n(n − 1)/2
  comparisons, bounded by the program's series (RFC-0015's M3 note).

**At r = ±1.0 in f64** Pearson's t is infinite (a value a hair inside
±1, as `[0.1 0.2 0.3 0.7] [1.0 2.0 3.0 7.0]` gives, takes the ordinary
path with a finite t): `statistic` is left out, p is 0
(or 1 for the opposite one-sided alternative), and the Fisher-z interval
is [r, r], tanh(±∞) = ±1 at both ends, answered. Spearman's S is finite
at |ρ| = 1 (0 or (n³ − n)/3) and is kept; its t, and so its p, follows
Pearson's rule. A one-sided Pearson interval's far end is −1 or 1, finite
and answered. Fewer than three pairs, or a constant series, is E101.

**Effects.** As RFC-0015 §S3's two-series words: each test is `a b --
MAP`, fixed, consuming 2, with a plain and a `.` form, the `.` form taking
b from the workbench; each `.opts` is `a b opts -- MAP`, consuming 3, its
`.` form taking the options MAP from the workbench.

### S4. Further corrections

`stat.padjust.hochberg`, `stat.padjust.hommel` and `stat.padjust.by`
(`p-list -- p-list`), in RFC-0015's four forms, opaque by RFC-0015 §S3's
one-series rule, as its three corrections are. With p₍₁₎ ≤ … ≤ p₍ₙ₎ the
sorted p-values, each adjusted value is computed in sorted order and
answered in input order:

- **Hochberg:** p̃₍ᵢ₎ = min_{j ≥ i} min(1, (n − j + 1)·p₍ⱼ₎).
- **Benjamini–Yekutieli:** with c = Σ_{k=1}^{n} 1/k, p̃₍ᵢ₎ =
  min_{j ≥ i} min(1, c·n·p₍ⱼ₎/j): Benjamini–Hochberg's values times c,
  capped at 1.
- **Hommel:** p̃ᵢ is the largest Simes p over every subset containing i,
  min(1, max_{I ∋ i} min_{j} |I|·p₍ⱼ₎^I/j), the closed-testing procedure
  over Simes' tests. It is computed without enumerating subsets:
  1. q = a = min_{i} n·p₍ᵢ₎/i, in every position;
  2. for m from n − 1 down to 2: let q₁ = min_{i=2}^{m} m·p₍ₙ₋ₘ₊ᵢ₎/i;
     set qᵢ = min(m·p₍ᵢ₎, q₁) for i ≤ n − m + 1 and qᵢ = q₍ₙ₋ₘ₊₁₎ for
     i > n − m + 1; then aᵢ = max(aᵢ, qᵢ);
  3. p̃₍ᵢ₎ = min(1, max(aᵢ, p₍ᵢ₎)).

  The steps agree with the subset definition to the last bit on 300
  random cases of n ≤ 9 (mpmath scratch `hommel.py`, by the author); the
  fixture checks them against the subset definition for its 20 p-values.

That R's `p.adjust` gives the same is [UNGROUNDED; Q62]. A p outside
[0, 1] is E101; an empty LIST answers an empty LIST. Hommel's steps are
O(n²): above 10⁴ p-values (10⁸ steps) it is E111 before it starts (D39).

### S5. Effect sizes

| Word | Operands | Answers |
|---|---|---|
| `stat.effect.cohend` | a b | `d`, (mean(a) − mean(b))/s_pooled with s_pooled = √(((n_a − 1)s_a² + (n_b − 1)s_b²)/df), `t2`'s pooled variance (`infer.rs`, `t2`); `g`, J·d with J = Γ(df/2)/(Γ((df − 1)/2)·√(df/2)), df = n_a + n_b − 2 |
| `stat.effect.oddsratio` | 2×2 MATRIX [[a b] [c d]] | `estimate` ad/(bc), `log`, `se` √(1/a + 1/b + 1/c + 1/d) (Woolf), `lower`, `upper`, `conf` 0.95 |
| `stat.effect.riskratio` | 2×2 MATRIX, rows exposed then unexposed, first column the event | `estimate` (a/(a + b))/(c/(c + d)), `log`, `se` √(1/a − 1/(a + b) + 1/c − 1/(c + d)) (Katz), `lower`, `upper`, `conf` 0.95 |
| `stat.effect.cramerv` | r×c MATRIX | V = √(χ²/(n·(min(r, c) − 1))), without Yates' correction |

- **Refusals.** `cohend` needs n_a, n_b ≥ 2 and a pooled SD above 0
  (E101, as the t tests refuse a constant series). `oddsratio` refuses
  any zero cell, which makes it 0 or infinite; `riskratio` refuses a = 0
  or c = 0, which make it 0 or infinite, and answers b = 0 or d = 0, where
  the estimate and Katz's se are finite (both zero gives the interval
  [estimate, estimate]). Each refusal is E101 naming the cell **[B12]**;
  a program wanting Haldane's correction adds 0.5 itself.
  `cramerv` has `stat.test.chisq.table`'s refusals (a table at least
  2×2, no row or column summing to 0, cells non-negative) and so
  min(r, c) ≥ 2; a zero *cell* is legitimate there.
- **J** is computed as exp(lgamma(df/2) − lgamma((df − 1)/2))/√(df/2)
  for df ≤ 200, and above that by the series Γ(z + ½)/Γ(z) =
  √z·(1 − 1/(8z) + 1/(128z²) + 5/(1024z³) − 21/(32768z⁴)) with
  z = (df − 1)/2, divided by √(df/2). The lgamma difference cancels as df
  grows, erratically: with §S4's `lgamma`, which is `statrs` 0.18.0's
  `ln_gamma` (`special.rs`, `lgamma`), 3.5e-10 relative at df 400 000
  and 4.6e-10 at 399 998, and at most 3.3e-13 (df 69) up to df 200. The
  series does not (below 1e-14 from df 398 up; 5e-12 at df 100).
  Measured against mpmath (scratch, by the author and the second, third
  and fifth reviews; the df 69 and 400 000 figures recomputed with
  `statrs` itself).
- **The ratios' intervals** are exp(`log` ∓ z₁₋α/₂·`se`) at conf 0.95.
- **Two odds ratios.** `stat.effect.oddsratio`'s is the sample ratio;
  `stat.test.fisher.opts`'s `estimate` is the conditional MLE. They differ
  on the same table, and each word's `method` names which it is.
- **Not here.** Intervals for d and g need the noncentral t, which
  `statrs` lacks (research 07, §2): research 07's Phase E. η² and ω² wait
  for ANOVA (its Phase C).
- **Forms.** Each has a plain and a `.` form; `cohend.` takes b from the
  workbench, the others their one operand. Fixed effects: `cohend`
  consumes 2, the others 1.

### S6. Rounding and formatting [A1] [A6]

| Word | Operands | Answers |
|---|---|---|
| `math.round.places` | x n | x rounded to n decimal places |
| `math.signif` | x n | x rounded to n significant digits |
| `string.fixed` | x n | a STRING with exactly n decimals |
| `string.sci` | x n | a STRING in scientific notation, n decimals in the mantissa |

- **n** is an INTEGER: [0, 17] for `math.round.places` and
  `string.fixed`, [1, 17] for `math.signif`, [0, 16] for `string.sci`.
  For `math.signif` and `string.sci`'s mantissa, seventeen significant
  digits name every f64 exactly, so a larger n changes nothing. For
  decimal places the cap of 17 is a choice, not a consequence: `{:.17}` of
  1.5e-18 is 0.0 and `{:.18}` is 1e-18. It bounds the strings' length and
  can be raised. A FLOAT n, or one out of range, is E101.
- **x** is a number, an INTEGER converting to FLOAT **[B15]** (the
  reference's `math.round` refuses one), or a LIST of numbers, answered
  element-wise as a LIST. The LIST is read as a LIST only, not by
  `get_data`. Cells are read as `whole` reads them (`operand.rs`,
  `whole`), through the FLOAT conversion, so a numeric STRING cell is its
  number (`[ "2.5" ]` is [2.5]); a cell that will not convert is E103, a
  non-finite one E101. An empty LIST answers an empty LIST. A scalar x
  is a FLOAT or an INTEGER; a STRING x is E103.
- **Overflow.** A result that is not finite is E114, as `finite` refuses
  (`operand.rs`, `finite`): `math.signif` can round up past `f64::MAX`,
  `1.7976931348623157e308 1 math.signif` being 2e308, which parses to ∞.
  `math.round.places` cannot, since rounding to n ≥ 0 places never passes
  an integer f64 already holds; the strings are not parsed.
- **Ties [A6].** The exact binary value is rounded, ties to even, which
  is Rust's formatter's rule (§Current behaviour). So `2.675 2
  math.round.places` is 2.67, because 2.675 is stored as
  2.67499999999999982236431605997495353221893310546875, and
  `2.5 0 math.round.places` is 2.0 where `2.5 math.round` is 3.0. Whether
  R ≥ 4.0.0's `round` always agrees is [UNGROUNDED; Q62]; Bund2's rule is
  stated here independently of it.
- **Implementation.** `math.round.places` parses `format!("{:.n$}", x)`,
  and `math.signif` parses `format!("{:.m$e}", x)` with m = n − 1, so each
  is exactly the formatter's rounding. `string.fixed` and `string.sci` are
  those strings.
- **Negative zero** is normalised by all four words, −0.0 included as an
  operand: `-0.4 0 math.round.places` is +0.0 and `-0.001 2 string.fixed`
  is `"0.00"`, not `-0.00`; `-0.0 2 string.sci` is `"0.00e0"`.
- **`string.sci`'s exponent** is written as Rust writes it, `1.23e4`,
  `1.0e-7`, and not as C's and R's `sprintf` write it, `1.23e+04`
  [UNGROUNDED as C's and R's; Q62]. With
  n ≥ 1 Bund's reader reads the string back as the number (§Current
  behaviour); with n = 0, Rust's `1e4` does not read back, which the word
  does not change.
- **Forms.** Fixed effects; a plain and a `.` form, the `.` form taking n
  from the workbench and answering there.

### S7. Shuffling and sampling

| Word | Operands | Answers |
|---|---|---|
| `rng.shuffle` | list | the LIST in a random order, by Fisher–Yates |
| `rng.choice` | list k | k elements without replacement, in draw order; k ≤ the length |
| `rng.choice.replace` | list k | k elements with replacement |
| `rng.choice.weighted` | list weights k | k elements with replacement, element i with probability wᵢ/Σw |

- **Draws** come from the VM's generator (`with_rng`; a VM that keeps
  none falls back to a thread-local `StdRng`, as RFC-0015 §S8 has it,
  seeded from entropy (`rng.rs`, `with_rng`, `StdRng::from_entropy`), and criterion 11 assumes a VM that keeps one), so reseeding with
  the same `rng.seed` repeats them within one Bund2 version, as RFC-0015
  §S8 says of every drawing word.
- **Algorithms**, which fix the streams, over `rand` 0.8's `StdRng`:
  - `rng.shuffle`: Fisher–Yates over a copy, for i from n − 1 down to 1,
    j = `gen_range(0..=i)` on `u64`, then swap i and j: n − 1 draws, none
    for position 0.
  - `rng.choice`: the first k steps of the same, answering the element
    that lands at each i in turn; at k = n the last element takes no draw.
  - `rng.choice.replace`: `gen_range(0..n)` on `u64` per draw.
  - `rng.choice.weighted`: u = Σw·`gen::<f64>()` per draw, answering the
    first i whose cumulative weight cumᵢ = w₁ + … + wᵢ is strictly above u,
    found by binary search; a zero-weight element is never drawn, since
    its cumᵢ equals its predecessor's. If rounding leaves no cumᵢ above u,
    the last element with a positive weight is answered.
- **Empty and zero.** k = 0 answers an empty LIST for every word, with no
  draw. An empty `list` with k ≥ 1 is E101 for `rng.choice.replace` and
  `rng.choice.weighted` (the latter by its positive sum), since
  `gen_range(0..0)` panics in `rand` 0.8.7 (`rng.rs:134` of that crate,
  `assert!(!range.is_empty(), "cannot sample empty range")`), which D37
  forbids; for `rng.choice` it is
  "k above the length". `rng.shuffle` of an empty LIST answers it.
- **Weights** whose sum overflows to ∞ are E114, before any draw.
- **The list** may hold any values; they are moved, not converted, NONE
  included: D252's refusal of NONE applies to numeric operands. A
  non-LIST is E101, deliberately, as for `ode.rk45`'s non-MAP options:
  its template names the operand as outside the word's domain. `weights` is a LIST as long as `list` (E102) of
  numbers (E103) that are finite and non-negative (E101) with a positive
  sum (E101). k is a non-negative INTEGER (E101); k above the length for
  `rng.choice` is E101; k above 10⁸ is E111 before allocating.
- **Forms.** Fixed effects; a plain and a `.` form. `rng.shuffle.` takes
  its one operand, the list, from the workbench. The others' `.` forms
  take k, the top operand, by RFC-0015 §S3's rule rather than D111's
  "works on a container" row, so that every word of the family moves the
  same operand.

### S8. The tiers, the audits and the generated files

As RFC-0015 §S17: effects are declared as S1 says; the palette audit
decides what `PROMOTABLE.txt` lists (F123); `PROMOTABLE.txt`,
`bund2 effects` and `docs/arity.md` are regenerated in each milestone.
No golden is added (D21).

**`UNVARIED` gains 20 names,** each a fixed native consuming more than
the palette varies (§Current behaviour, "The audits"), with its reason:

| Names, each with its `.` form | Reason (the unvaried deepest operands) |
|---|---|
| `stat.test.t2.opts`, `welch.opts`, `paired.opts`, `mannwhitney.opts`, `ks2.opts` | `a`, a series |
| `stat.cor.test.pearson.opts`, `spearman.opts`, `kendall.opts` | `a`, a series |
| `stat.test.binom.opts` | k and n, counts |
| `rng.choice.weighted` | `list`, the list drawn from |

`stat.test.t1.opts` is opaque, so not audited; `stat.test.fisher.opts`
consumes two.

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| The reference's `math.round` (`float_math.rs:57-58,127,176`) and Bund2's D111 form `math.round.` (`wb.rs`, `ADDED`): one operand, half away from zero, an INTEGER refused | **Preserved exactly.** `math.round.places` is a new name with its own tie rule **[A6]** and converts an INTEGER **[B15]** |
| `format` and `format.` (`format.rs:9-68,72-132`) | **Preserved exactly.** `string.fixed` and `string.sci` are new names |
| RFC-0015's ten test words: operands, effects, forms, answers, refusals and cost | **Preserved exactly [A2]** |
| `stat.test.mannwhitney`'s p at W = n·m/2 | **Changed before this RFC, as a Bund2 bug fix** (RFC-0015 §S6's note of 2026-11-16): p is 1 there, as the committed fixture's formula gives (and R's [UNGROUNDED; Q62]) |
| `stat.cor.*` and RFC-0015's corrections | **Preserved exactly.** The tests and corrections here are new names |
| The VM's generator and every seeded stream of RFC-0015's words | **Preserved exactly:** the new words draw only when called |
| RFC-0015's criterion-2 tests, which count names by prefix (`infer/tests.rs`, `describe/tests.rs`, `dist/tests.rs`) | **Amended deliberately:** each milestone adds its names to the module's `WORDS` (new modules' names to an excluded prefix or the counting test's list), so RFC-0015 criterion 2's "nothing else is added" is extended by this RFC's criterion 1 |
| `UNVARIED` | **Extended** by S8's 20 names, each with its reason |
| A program that defines one of the new names itself | **Preserved:** a program's word is found before a native of the same name, in the reference and in Bund2 (S1, cited and probed) |
| A program that asks whether one of the 64 names exists (`?stdlib`, `?word`) | **Changed by design:** the answer becomes true. No such program is known |
| `PROMOTABLE.txt`, `bund2 effects`, `docs/arity.md` | **Regenerated;** existing words' lines do not move, since no existing word changes; new words appear as the palette reaches them |
| `conform` 143/159, CEILING 143/159; IMPLEMENTED 597/602, COVERAGE 577/602 | **Preserved exactly** (D72): no reference word is touched |

## Alternatives considered

- **Interval keys on the plain test words** (A2's option A, the first
  draft). Rejected after the review's B1: the plain words would gain new
  refusals (E110 from a polish or a bisection, E111 from the Hodges–Lehmann
  cap), new costs (O(n·m) for Mann–Whitney, up to 798 support sums for
  Fisher), and a changed key order on printing, since a MAP's keys sort.
- **Leaving an interval key out when it cannot be computed** on the plain
  words. Rejected: an absent key would mean either "infinite" or "failed",
  and the cost would remain.
- **Separate `stat.ci.*` words.** Rejected: they would recompute the test.
- **A trailing optional options MAP on the plain words.** A word whose
  arity depends on its top operand cannot declare a fixed effect, so every
  test would become opaque and leave the palette audit.
- **R's asymptotic Hodges–Lehmann estimate and interval.** The statistic
  it inverts is a step function in the shift, with no unique root
  [UNGROUNDED as R's; Q62], and R's definition is not in `reference/`.
- **k always by the normal approximation** (D251 A9's first ruling, A),
  or **rounded to nearest** (its option B). The floor is up to 4 below the
  exact k at 99% for small samples, and degenerate on criterion 6's first
  case; rounding gives an interval below conf in 17 of 186 size pairs at
  90%. Re-ruled C: exact where it applies.
- **A lower support cap for `fisher.opts`.** Its 798 sums at the plain
  word's cap of 10⁸ outcomes are about 8·10¹⁰ terms: bounded (D39) but
  minutes at Tier 0. A separate cap would add an E111 that the plain word
  does not have on the same table; left as the plain word's, since a
  table with 10⁸ outcomes is already one the plain word takes minutes on.
- **The exact Mann–Whitney k only for n, m < 50** (D251 A9's C as first
  ruled). Widened to C(n + m, n) < 2¹²⁸ after the third review's S1.
- **A one-sided Mann–Whitney p above the Hodges–Lehmann cap.** n·m above
  10⁷ refuses the whole of `mannwhitney.opts`, so a one-sided test of two
  samples of 5000 has no word. Selecting the two order statistics and the
  median without forming every difference, in O((n + m) log(n + m)), would
  lift the cap; left until a use needs it.
- **±∞ for an infinite end**, as R prints `-Inf` [UNGROUNDED; Q62].
  Rejected by B12 **[A3]**.
- **A `places` operand on `math.round`.** It would change a reference
  word's arity, a deviation rather than an addition.
- **Spearman's exact p** by AS 89, R's default for n < 1290 without ties
  [UNGROUNDED; Q62]. Rejected for now **[A5]**: no fixture here can check
  it.
- **Negative places** (`1234 -2 math.round.places` → 1200). Rounding
  x/10² through the formatter is not exact; E101 until asked.
- **Weighted sampling without replacement.** Several definitions exist and
  their inclusion probabilities are not the weights; left out until a use
  names which it wants.
- **`.opts` for `chisq` and `chisq.table`.** Neither has an alternative or
  an interval, so the words would accept only an empty MAP.

## Acceptance criteria

Each is a named test in `crates/bund2-stdlib` unless placed otherwise.
Expected values are stated below, or come from a committed fixture
computed from the definitions with mpmath at 50 digits, as RFC-0015's M5
and M12 fixtures were **[A7]**. R's printed outputs are compared where a
source for them is committed beside the fixture; where none is, the
fixture alone is the check, and the milestone's note says so (A7's
wording, confirmed by D251's amendment). Every `method` is compared
exactly. Tolerance: 1e-12 relative for closed forms, as RFC-0015's; 1e-9 for an
interval end that is a quantile, since the polish bounds |cdf(x) − p| at
10⁻¹², not x (`dist.rs`, `POLISH_TOL`); 1e-8 for roots by bisection.

1. **Registered.** The 64 names of S2–S7 (16 `.opts` names, 12
   correlation, 12 corrections, 8 effect sizes, 8 rounding and
   formatting, 8 sampling) are in the registry with the effects S1 and
   each section give, and RFC-0015's criterion-2 tests pass with them.
2. **Refusals.** Each word's refusals render their templates: the
   options MAP's (an unknown key, a key the word does not take, `conf`
   outside (0, 1), an INTEGER `conf`, a non-STRING `alternative`); S2's
   E110 and E111; S5's and S7's listed refusals, an empty `list` with
   k = 1 for `rng.choice.replace` and weights summing to ∞ among them;
   `1.7976931348623157e308 1 math.signif` is E114; so is
   `mannwhitney.opts` on a = [1e308 1.0 2.0 3.0], b = [−1e308 1.5 2.5
   3.5] at 95% (k = 1, D₍₁₆₎ = ∞), which the plain word answers; and on
   a = [1.7e308 1.6e308 1.5e308 0.0], b = [0.0 0.1 0.2 0.3] the estimate
   is 1.55e308, not E114. No operand from a palette of NaN, ±∞, wrong
   types, empty and ragged values aborts, for every name and form.
3. **Plain words unchanged.** M1's first commit, before any code moves,
   records each of RFC-0015's ten tests' answers on RFC-0015's fixtures as
   a snapshot, every key and the f64 bits of every number, taken from the
   code as it stands before this RFC (after cae380d). Every milestone, M1
   included, answers that snapshot bit for bit, `method` included. An
   `.opts` word's default `statistic`, `p` and `df` equal its plain
   word's wherever both answer; where the `.opts` word refuses an
   overflowing estimate or end (S1), the plain word still answers. `[ 1.0 4.0 ] [ 2.0 3.0 ]
   stat.test.mannwhitney` answers p = 1. With the defaults, each `.opts`
   word's `statistic`, `p` and `df` equal its plain word's bit for bit.
4. **t intervals.** On `sleep`, `t2.opts`, `welch.opts`, `paired.opts`
   and `t1.opts` (μ₀ 0, the fixture's) answer the fixture's estimate and
   two-sided 95% ends and the plain words' `method`; with `"less"` each
   answers only `upper` and the lower tail's p; with `conf` 0.99 the
   fixture's wider ends; with `mu` 1.0 the two-sample words answer the
   fixture's shifted t and p and the unshifted estimate and ends; `"mu" 1`
   (an INTEGER) answers the same as `"mu" 1.0`. Absent ends are tested by
   `?key`, which answers false.
5. **Exact intervals and one-sided p.**
   - `binom.opts` on 682 of 925 at p = 0.75: the fixture's Clopper–Pearson
     ends, "Exact binomial test" as `method`, and P(X ≤ 682) for "less"
     and P(X ≥ 682) for "greater"; at k = 0 the lower end is exactly 0,
     and at k = n the upper is exactly 1.
   - `fisher.opts` on [[3 1] [1 3]], two-sided and "greater": the
     fixture's ψ̂, ends and p, and S2's `method`; on [[3 0] [0 3]] the
     estimate and upper end are absent and the lower end present.
   - `ks2.opts` on `sleep`'s two groups: for each alternative, the
     fixture's D, D⁺ or D⁻ and p, and "Asymptotic two-sample
     Kolmogorov-Smirnov test" as `method`.
6. **Hodges–Lehmann [A9].** With a = [1.1 2.2 3.3 4.4] (n = 4), two
   cases of m = 5; in each the 20 differences' median is 0.75 and the
   `method` is S2's.
   - **Exact k:** b = [0.5 1.5 2.0 2.5 3.0], no ties in the pooled nine.
     The sorted differences are −1.9, −1.4, −0.9, −0.8, −0.4, −0.3, 0.2,
     0.3, 0.6, 0.7, 0.8, 1.3, 1.4, 1.7, 1.8, 1.9, 2.4, 2.8, 2.9, 3.9. At
     95%, P(U ≤ 1) = 2/126 ≤ 0.025 < P(U ≤ 2) = 4/126, so k = 2 and the
     ends are D₍₂₎ = −1.4 and D₍₁₉₎ = 2.9 (the normal floor would give
     k = 1, the whole range).
   - **Normal k:** b = [0.5 1.5 1.5 2.5 3.0], a tie. At 90%,
     nm/2 − 1.644854·√(200/12) = 3.285, so k = 3, and the sorted
     differences −1.9, −1.4, −0.8, … , 2.8, 2.9, 2.9, 3.9 give ends
     D₍₃₎ = −0.8 and D₍₁₈₎ = 2.9.

   - **The boundary:** a = [1.0 2.0 3.0], b = [1.5 2.5 3.5] at conf 0.9,
     where P(U ≤ 0) = 1/20 exactly: k = 1, both ends answered, D₍₁₎ and
     D₍₉₎, not E101.
   - **One-sided and `mu`:** on the exact case, "greater" answers only
     `lower`, D₍₃₎ = −0.9 (k = 3: P(U ≤ 2) = 4/126 ≤ 0.05 < P(U ≤ 3) =
     7/126), and the upper normal tail as p;
     "less" answers only `upper` and the lower tail; with `mu` 0.5 the p
     is the fixture's for a − 0.5 against b, and the estimate and ends are
     unchanged.
   - The cases below take bⱼ = 1.001·j + 0.5 for j = 0 … m − 1, so that
     no value is pooled twice and all nm differences are distinct: the
     k each asserts and the wrong k each is there to catch select
     different ends. The ends are f64 values as Python prints them
     (scratch `rev6v.py`).
   - **A wide exact case:** a = [1.0 2.3], m = 52, at 90%: the exact
     k = 15, ends −43.245 and −5.505999999999999; the floor's 16 would
     give −42.54299999999999 and −6.207999999999999.
   - **A large exact case:** a = [1.0 2.0 … 65.0], m = 65, C(130, 65) <
     2¹²⁸, at 95% two-sided: k = 1692, ends −6.525999999999996 and
     7.462000000000003. The floor's 1691 would give −6.5269999999999975
     and 7.463000000000001, and an ascending difference (k = 1535)
     −9.514999999999999 and 10.451000000000008.
   - **Past the bound:** a = [1.0 … 66.0], m = 66, C(132, 66) ≥ 2¹²⁸, at
     95%: the floor's k = 1747, ends −6.529999999999998 and
     7.465000000000003, where the exact 1748 would give
     −6.528999999999996 and 7.464000000000006.
   - **The floor's documented shortfall:** a = [1.0 … 20.0], m = 691, at
     80% two-sided: C(711, 20) ≥ 2¹²⁸, so the floor's k = 5749, ends
     −393.895 and −276.79499999999996, where the exact 5746 would give
     −393.89799999999997 and −276.792.
   - **Low one-sided levels:** on a = [1.0 2.0 3.0], b = [1.1 2.3 3.6]
     (all nine differences distinct), "greater" at conf 0.3 answers
     `lower` = D₍₆₎ = −0.10000000000000009 (k = 6, past ⌊nm/2⌋, read by
     symmetry); truncated counts would give k = 5 and
     −0.2999999999999998. On the same data at conf 10⁻⁸, the exact k is
     10 > 9, E101. On a = [1.0 2.0 3.0], b = [1.0 2.5 3.5] (a tie, the
     floor), "greater" at conf 0.005 is E101 (k = 10 > 9).

   k < 1 at a level too high for n = m = 2 is E101; n·m above 10⁷ is
   E111 before allocation.
7. **Correlation tests.** On a committed fixture: Pearson's t, df, p and
   Fisher-z ends, no interval at n = 3, and, on [1.0 2.0 3.0 4.0] against
   [2.0 4.0 6.0 8.0] (r exactly 1.0, probed), no `statistic`, p 0 and the
   interval [1, 1]; Spearman's S and p; Kendall's exact T and p, by S3's
   two-sided rule, for n < 50 without ties, at n = 10 and at n = 45 with
   a strong correlation, so that p lies in a far tail, within 1e-10
   relative; its z and p with S3's tie-corrected variance otherwise;
   each one-sided alternative.
8. **Corrections.** `hochberg`, `hommel` and `by` on a fixture of 20
   p-values match S4's formulas in input order, the fixture computing
   Hommel's by the subset definition; Hommel above 10⁴ p-values is E111.
9. **Effect sizes.** Cohen's d and Hedges' g on `sleep`, and g at
   n_a = n_b = 101 and 101 against 102 (df 200 and 201, either side of
   the switch to the series), at n_a = n_b = 200 (df 398, where Γ alone
   overflows) and at n_a = n_b = 2·10⁵, finite and within 1e-12 of the
   fixture; the odds and risk ratios with their intervals on a 2×2
   fixture; `oddsratio` refusing a zero in each cell in turn and
   `riskratio` refusing a = 0 and c = 0 (E101 naming the cell) and
   answering b = 0 and d = 0; Cramér's V on a 3×3 with a zero cell,
   answered.
10. **Rounding and formatting.** `2.675 2 math.round.places` is 2.67,
    `0.125 2 math.round.places` 0.12, `2.5 0 math.round.places` 2.0,
    `-0.4 0 math.round.places` +0.0 (its sign bit compared),
    `123456 2 math.signif` 120000.0, `[ "2.5" ] 0 math.round.places`
    `[2.0]`,
    `[ 1.23456 2.5 ] 3 math.round.places` `[1.235, 2.5]`;
    `3.14159 2 string.fixed` is `"3.14"`, `-0.001 2 string.fixed`
    `"0.00"`, `12345.678 2 string.sci` `"1.23e4"`; and `2.5 math.round` is
    still 3.0.
11. **Sampling.** After `7 rng.seed`, each S7 word answers a value, and
    after `7 rng.seed` again, the same value; `rng.shuffle` answers a
    permutation of its list; `rng.choice` draws no index twice; over 10⁵
    seeded draws of `rng.choice.weighted` with weights [1 2 3 4], each
    category's frequency is within 4 SE of its weight's share, and with
    weights [0 1 0 1] no zero-weight element is drawn; k = 0 answers an
    empty LIST. The first answers of each word after `7 rng.seed` are
    pinned as values of this Bund2 version, so a change of sampler, say to
    `SliceRandom::shuffle`, fails the test.
12. **Health.** `cargo xtask conform` and `coverage` print the numbers of
    §Preservation analysis; clippy and `cargo deny` are clean;
    `PROMOTABLE.txt`, `bund2 effects` and `docs/arity.md` are regenerated,
    with no existing word's line moved.

## Implementation plan

| M | Delivers |
|---|---|
| M1 | criterion 3's snapshot first; then S6 and S4; criteria 1, 2, 3, 8 and 10 for them |
| M2 | S2 for the t tests and the binomial (`Family::quantile` made `pub(crate)`); criteria 3, 4 and 5's first part |
| M3 | S2 for Fisher, Mann–Whitney and KS; criteria 5's second part and 6 |
| M4 | S3; criterion 7 |
| M5 | S5 and S7; criteria 9 and 11 |

Each ends with criterion 12.

M1 landed 2026-11-17. Its first change records criterion 3's snapshot
before any code moves: `biomath/fixtures/infer.snapshot`, every key of
RFC-0015's ten tests' answers on their fixtures and of
`[ 1.0 4.0 ] [ 2.0 3.0 ] stat.test.mannwhitney`, each number as its f64
bits, written from the code at cae380d by the ignored test
`write_the_snapshot` and compared by
`the_plain_tests_answer_the_snapshot_bit_for_bit`. Then:

- **S4** in `biomath::infer`: `stat.padjust.hochberg`, `.hommel` and
  `.by`, in four forms, with criteria 1, 2 and 8. The fixture
  (`fixtures/infer16.py`, exact rationals) computes Hommel's values by
  enumerating all 2²⁰ subsets of its 20 p-values; a unit test also checks
  the steps against the subset definition on every subset of six small
  series. No R output is committed for these, so the fixture alone is the
  check (A7). Hommel above 10⁴ p-values is E111, `a series of 10001
  p-values exceeds its cap of 10000`.
- **S6** in a new module, `biomath::round`: `math.round.places`,
  `math.signif`, `string.fixed` and `string.sci`, plain and `.`, with
  criteria 1, 2 and 10. n is read before x. S6 names a STRING x as E103;
  any other operand that is neither a number nor a LIST, a MAP or NONE
  say, is E101, as RFC-0015's scalar reader `real` refuses one.
  `math.signif`'s E114 names the operand in `{:e}` form:
  `1.7976931348623157e308 to 1 significant digits`.

`PROMOTABLE.txt` gains the eight rounding and formatting names (the
corrections are opaque). Criterion 12: conform and coverage unchanged.

## Rulings — D251 (RESOLVED 2026-11-16)

| | Question | Options | Ruled |
|---|---|---|---|
| **A1** | Names for rounding and formatting | **A** `math.round.places`, `math.signif`, `string.fixed`, `string.sci`. **B** one `math.format` taking a mode | **A** |
| **A2** | Where intervals go | **A** keys on the plain tests, plus `.opts` variants. **B** separate `stat.ci.*` words. **C** `.opts` variants only, the plain answers unchanged | **C** (re-ruled after the review's B1; first ruled A) |
| **A3** | An infinite interval end | **A** leave the key out. **B** ±∞, against B12. **C** NODATA in a two-element LIST | **A**, confirmed after the third review on the corrected premise that `get` on an absent key is an error and `?key` tests it |
| **A4** | Default confidence level | **A** 0.95. **B** none, `conf` required | **A** |
| **A5** | Spearman's p | **A** the t approximation. **B** AS 89's exact p | **A** |
| **A6** | Ties in rounding to places | **A** the exact binary value, ties to even. **B** half away from zero | **A** |
| **A7** | Reference values | **A** mpmath fixtures from the definitions, plus R's printed help-page outputs; worded "R's printed output where a source is committed; where none is, the fixture alone is the check" by D251's amendment, confirmed by the owner. **B** install R | **A** |
| **A8** | Which build | **A** the default build. **B** a feature | **A** |
| **A9** | `mannwhitney.opts`'s estimate and interval | **A** the Hodges–Lehmann median and the order-statistic interval, n·m capped at 10⁷, k by the normal approximation, floored. **B** R's asymptotic estimate and interval. Re-ruled after the second review on k: **A** the floor always, **B** rounded to nearest, **C** exact without ties and n, m < 50, the floor otherwise; C's condition widened after the third review to C(n + m, n) < 2¹²⁸; after the fourth, the floor's shortfall recorded: **A** accept, **B** make the fallback conservative | **A**, then **C** in its k, widened; the shortfall accepted, **A** (D251's amendments) |

## Open questions

- **Q62** (`docs/registers/open-questions.md`): every claim here about
  R's behaviour, each marked [UNGROUNDED] where it is made. R's source is
  not in `reference/` and R is not installed. The claims this RFC makes
  are: which tests `htest` gives an estimate or interval, and `t.test`'s
  two means; `ks.test`'s one-sided statistics, tail and sense of
  "greater"; `cor.test`'s Fisher-z interval and its Kendall exact and
  tie-corrected p; `p.adjust`'s Hochberg, Hommel and BY; the one-sided
  Mann–Whitney correction; p = 1 at W = n·m/2; R's Hodges–Lehmann
  inversion; `round`'s rule since R 4.0.0; AS 89 as R's default; R's
  `-Inf`; `sprintf`'s exponent. Q62 also keeps `fisher.test`'s and
  `binom.test`'s one-sided p and R's exact Wilcoxon interval, which this
  RFC no longer claims as R's. Each milestone's fixture is the check,
  against the definitions this RFC states, which stand whether or not R
  agrees.
- D252 (tables and missing values) does not bear on this RFC; its words
  take LISTs, METRICS and MATRIX as RFC-0015's do.

## Revision 2

Revision 2 answers `docs/rfc/reviews/RFC-0016-review-2026-11-16.md`.

| Finding | Change |
|---|---|
| B1 plain words gain refusals and costs | A2 re-ruled C (D251's amendment): the plain words are unchanged; intervals only in `.opts`; Preservation row and criterion 3 say so |
| B2 Hodges–Lehmann undefined | A9 ruled A: S2 defines the estimate and the order-statistic interval exactly; criterion 6 has a worked case |
| S1 R claims unmarked, Q62 incomplete | each marked [UNGROUNDED] where made; Q62 extended; §Open questions lists its scope |
| S2 "What is linked" said statrs | now `dist.rs`'s own tails and quantiles, statrs checking parameters; `quantile` to become `pub(crate)` |
| S3 "less"/"greater" undefined for four tests | S2's alternatives table, per word; KS follows R's sense, said so |
| S4 key set unspecified | S2's answer table, per `.opts` word; plain words unchanged |
| S5 `chisq.opts` words useless | dropped; eight `.opts` words; in §Alternatives |
| S6 J overflow, edge cases | J by `lgamma`; `cohend`'s and `cramerv`'s refusals; zero-cell rule scoped to the ratios; criterion 9 tests df 398 |
| S7 Preservation missed tests, audits, files | rows for the prefix-count tests, `UNVARIED` (20 names with reasons in S8), `docs/arity.md`, PROMOTABLE |
| S8 E101 for a non-number | E103 for a non-number, E101 for a non-finite value, in S6 and S7 |
| S9 which operand `.` takes | stated per word in S5, S6, S7, with the reason in S7 |
| S10 R's values unstated | criteria state values or name the fixture; R's output only where a source is committed |
| S11 the z = 0 defect | fixed in RFC-0015's code with a test (RFC-0015 §S6's note of 2026-11-16); criterion 3 checks it; one-sided correction stated as ±½ by direction |
| N1 `t_p` | "twice the lower tail at −\|t\|" |
| N2 the probe | the probe now evidences the INTEGER refusal; arity rests on the citation |
| N3 seeded draws | all five listed |
| N4 `format`'s FLOAT conversion | cited to `conv.rs:694-696,127-130` |
| N5 forms omitted | `math.round.` and `format.` in Preservation |
| N6 negative zero | normalised, criterion 10 |
| N7 `string.sci`'s exponent | stated as a deliberate departure, with when Bund reads it back |
| N8 places' ranges and types | INTEGER n, ranges with a reason, FLOAT n refused |
| N9 the ratios | orientation, Katz's se, the two odds ratios distinguished |
| N10 finite far ends | S2's "Infinite ends" paragraph |
| N11 R's estimates | `t.test`'s two means noted; `wilcox.test`'s `conf.int` in Motivation |
| N12 η², ω² | wait for ANOVA, S5 |
| N13 Q62 in §Open questions | full scope listed |
| N14 polish tolerance | criteria's tolerance 1e-9 for quantile ends, with the reason |
| N15 Clopper–Pearson at k = 0, n | ends 0 and 1 by definition, criterion 5 |
| N16 Hommel's work | O(n²), capped at 10⁴ p-values, E111 |
| N17 tolerances | 1e-12 for closed forms, as RFC-0015 |
| N18 Fisher's bracket | doubling from [−1, 1], 64 at most, then bisection |
| Assumptions: STRING keys, workbench answers, LIST-only reading, sampling algorithm, Kendall's loop, data-dependent absent keys, D252's `na` | each stated in S1, S6, S7 or S3 |

## Revision 3

Revision 3 answers `docs/rfc/reviews/RFC-0016-review-2026-11-16-2.md`.

| Finding | Change |
|---|---|
| S1 criterion 6 degenerate | A9 re-ruled C (D251's amendment): exact k without ties and n, m < 50, the floor otherwise, stated as conservative with the measurement; the Hollander and Wolfe attribution dropped; the even-nm median defined; criterion 6 has two cases with every value stated, an exact k = 2 and a normal k = 3 |
| S2 Fisher's work bound | 792 support sums, 264 per root, three roots |
| S3 Hochberg, Hommel, BY undefined | S4 states each as a formula, Hommel by its subset definition and by step-by-step computation checked against it; criterion 8 points at them |
| S4 Kendall's exact two-sided p and variance | S3 defines the null distribution, the two-sided rule, the tie-corrected variance, no continuity correction, and `statistic` per branch |
| S5 `.opts` answers and `method` | S2 says which keys change with `alternative` and `mu`, gives every `method` string, and criteria 3–6 check them |
| S6 research 07 contradicted | ERRATA entry for `07-statistical-research.md:133-140,216`, with A5's narrowing |
| S7 A7's wording | confirmed by the owner (D251's amendment); the rulings table gives both texts; Q62 aligned |
| S8 R and C claims unmarked | `sprintf`, Preservation row 4, and R's Hodges–Lehmann inversion marked; Q62 extended a second time |
| S9 criterion 3's baseline | a bit-for-bit snapshot committed first in M2, taken after cae380d |
| N1 the reason for 17 places | given as a choice for places, with the 1.5e-18 case |
| N2 Pearson's one-sided far end | finite, answered, in "Infinite ends" and S3 |
| N3 \|r\| = 1 | Spearman keeps S; Pearson's interval is [r, r] |
| N4 zero weights | strict search, a zero weight never drawn; criterion 11 checks it |
| N5 the stream | `gen_range` on `u64`, n − 1 draws, `gen::<f64>()` for weights |
| N6 J at large n | the series above df 200, measured; criterion 9 adds df ≈ 4·10⁵ |
| N7 ratio intervals | exp(log ∓ z·se) stated |
| N8 the interval's tie term and one-sided end | no tie term, said and why; one-sided ends stated |
| N9 the cap refuses a one-sided p | stated in "Work" and §Alternatives, with the way out |
| N10 S3's and S4's effects | stated |
| N11 D251's stale heading and bullet | noted in D251's amendment |
| N12 §Open questions over-inclusive | split into what this RFC claims and what Q62 keeps |
| N13 eight tests | seven with intervals, eight with alternatives |
| N14 t1's μ₀, two-sample `mu` | μ₀ 0 named; a `mu` 1.0 case added |
| N15 E101 for a non-LIST | said to be deliberate |
| N16 NONE in S7 | moved like any value; D252's refusal is for numeric operands |
| Assumptions: two-sided `.opts` p equals the plain one, Fisher in log space, the normal quantile's E110, an INTEGER `conf` | each stated in S1 or S2; criterion 3 checks the first |

## Revision 4

Revision 4 answers `docs/rfc/reviews/RFC-0016-review-2026-11-16-3.md`.

| Finding | Change |
|---|---|
| B1 an absent key is not NODATA | `get` on an absent key is an error (`get.rs:10-12`); a program tests `?key`; A3 confirmed by the owner on that premise (D251's amendment); criterion 4 checks `?key` |
| S1 the floor not conservative | A9's exact branch widened by the owner to no ties and C(n + m, n) < 2¹²⁸; the floor's behaviour stated as measured, including where it falls short; counting by the Gaussian binomial in `u128`; criterion 6 adds n = 2, m = 52 |
| S2 the f64 boundary | the comparison carries Fisher's `REL_ERR`, 1 + 10⁻⁷, so 0.9 admits 1/20 at n = m = 3; criterion 6 adds that case |
| S3 `math.signif` overflows | E114 for a non-finite result; criterion 2 |
| S4 Kendall overflows `u128` | the recurrence is carried as f64 probabilities; criterion 7 at 1e-10 at n = 10 and 45 |
| S5 empty `rng.choice.replace` | E101 for an empty list with k ≥ 1; k = 0 an empty LIST; criteria 2 and 11 |
| S6 unchecked paths | criterion 5 adds `binom.opts`'s one-sided p and `method` and `ks2.opts`; criterion 6 adds one-sided and `mu` |
| S7 the risk ratio's zero cells | refuses a = 0 or c = 0 only; criterion 9 checks each cell |
| S8 the snapshot after M1 | taken in M1's first commit; every milestone answers it |
| S9 `math.round.` attributed to the reference | split: the reference's `math.round`, Bund2's D111 `math.round.` (`wb.rs`, `ADDED`) |
| N1 the lgamma figure | 1.2e-10 at df 400 000 and 4.6e-10 at 399 998 |
| N2 the switch untested | criterion 9 adds df 200 and 201 |
| N3 r = 1 in f64 | the rule is r = ±1.0 in f64; criterion 7 names data giving exactly 1.0 (probed) |
| N4 negative zero | all four words, −0.0 operands included; criterion 10 compares the sign bit |
| N5 `whole` converts STRING cells | S6 reads cells as `whole` does, stated, and criterion 10 checks `[ "2.5" ]` |
| N6 an INTEGER `mu` | converts (B15); criterion 4 checks it |
| N7 ties under `mu` | tested on a and b before the shift |
| N8 one-sided tails | named per alternative for Mann–Whitney and KS |
| N9 work bounds | 798 Fisher sums; the exact k's additions bounded; no lower Fisher cap, in §Alternatives |
| N10 weights overflowing | E114 before any draw |
| N11 line 107's R claim | marked [UNGROUNDED; Q62] |
| N12 D251's status line | brought up to date |
| N13 `s_pooled` | defined, as `t2`'s |
| N14 decisions consumed | each one's use stated; A7 in place of B11 |
| N15 streams unpinned | criterion 11 pins the first answers |
| N16 a program's own word | Preservation row and S1, probed |

## Revision 5

Revision 5 answers `docs/rfc/reviews/RFC-0016-review-2026-11-16-4.md`.

| Finding | Change |
|---|---|
| S1 the floor's shortfall understated | the measured figures stated by level, worst 4.7·10⁻³ at 80% two-sided, conservative at 95% and 99% two-sided; accepted by the owner, A (D251's amendment); criterion 6 adds the floor at n = 20, m = 691 |
| S2 low one-sided `conf` | counts above ⌊nm/2⌋ read by symmetry; k > nm is E101; criterion 6 adds conf 0.3 (k = 6) and conf 0.005 with a tie (E101) |
| N1 "the seven others" | six |
| N2 n ≤ 5 | n ≤ 6, with the first size past it |
| N3 the difference's direction | descending, in place, with what ascending does; criterion 6 adds n = m = 65 |
| N4 the count table's memory | 80 MB at its largest, stated |
| N5 792 | 798 in §Alternatives and in the ERRATA entry |
| N6 Kendall's summation | each sum formed directly, the upper tail by symmetry; criterion 7's n = 45 in a far tail |
| N7 n! | outgrows `u128` from 35 |
| N8 the `GET returns error:` prefix | cited to `value_dict.rs:62-67` |
| N9 rand's panic, the resolver | `rng.rs:134` of `rand` 0.8.7; `multistackvm_apply.rs:46-59` and `Registry::resolve_uncached` |
| N10 existence probes | a Preservation row, changed by design |
| N11 ties by f64 equality | stated |
| N12 an empty LIST in S6 | answers an empty LIST |
| N13 "10 of 128" | superseded by D251's amendment, which states the new figures |
| N14 `with_rng`'s fallback | noted in S7, and criterion 11's assumption |

## Revision 6

Revision 6 answers `docs/rfc/reviews/RFC-0016-review-2026-11-16-5.md`.
Every value it states was recomputed for this revision (scratch
`rev6v.py`; the Hedges' J figures with `statrs` 0.18.0 itself).

| Finding | Change |
|---|---|
| S1 criterion 6's data cannot see k | bⱼ = 1.001·j + 0.5 throughout, every difference distinct, each case's ends and the wrong k's ends stated; n = 2, m = 52 given data; n = m = 66 added past the bound; the exact branch's E101 at conf 10⁻⁸ added |
| S2 overflow from finite operands | E114 for a non-finite estimate, end or statistic, stated in S1; the even-nm median formed without overflow; criterion 2 adds both; criterion 3 says where the equality holds |
| N1 the shortfall's tail | about 4.6·10⁻⁴ at s = 66 and 67, 3·10⁻⁴ at 100, 1·10⁻⁴ at 300; D251's amendment corrects its 3.8·10⁻⁴ |
| N2 the symmetry identity | stated for the lower-tail count L, with L(−1) = 0 |
| N3 the k search | bounded to 0 … nm + 1 |
| N4 8.2·10⁻⁴ | 8.1·10⁻⁴; the worst case a plateau at s = 7 |
| N5 S5's lgamma figures | Bund2's `lgamma` (`statrs`): 3.5e-10 at df 400 000, 3.3e-13 at most up to df 200 |
| N6 the table's largest size | nm = 10⁷ |
| N7 n ≤ 6 | min(n, m) ≤ 6; D251's amendment notes its third amendment's n ≤ 5 is superseded |
| N8 entropy seeding | cited to `with_rng`, not to RFC-0015 §S8 |
| N9 the record of the fourth amendment | the A9 row and D251's Status line name it |
