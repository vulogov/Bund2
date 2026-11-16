# RFC-0016: Statistical inference — intervals, effect sizes, correlation tests, rounding and resampling primitives

- Status: **Draft** (2026-11-16), on the owner's "RFC for Phase A", after
  `docs/research/07-statistical-research.md`. Its owner questions are
  D251's A1–A8, **OPEN**; every passage that rests on one is marked
  **[A*n*]** and is written against the recommended option, which is not
  adopted until ruled.
- Depends on: RFC-0015 (the `biomath` family: its operand readers, S3's
  forms, S8's generator, S11's refusals), RFC-0004 (effects)
- Decisions consumed: D21, D24, D36, D37, D38, D39, D72, D111, D250 (B3,
  B4, B5, B10, B11, B12, B15); F123
- Reference SHA: `reference/rust_multistackvm`
  4605832678d4792c2b8e0564b7757d5aee1539a4; `reference/Bund`
  21b40b0213a76f3f39bd6de0d2bdfdf0a53b0dca; `reference/rust_dynamic`
  ceb27c96fa10c4edb09106f3f904f22f94d63119. Bund2's own code is cited at
  `development/0.56.0` after b976f2a, by file and symbol.
- Supersedes: nothing. Grounds and refines research 07's Phase A (§4).

## Summary

RFC-0015 gave Bund2 the classical tests, but each answers only a
statistic and a p-value, two-sided. Statistical research reports
estimates, confidence intervals and effect sizes, tests one-sided
hypotheses, and prints its numbers to a stated precision. This RFC adds,
as words of Bund2's own beside RFC-0015's family:

- an estimate, a confidence interval and an alternative on every
  RFC-0015 test that has them in R, through new `.opts` variants;
- correlation tests;
- the Hochberg, Hommel and Benjamini–Yekutieli corrections;
- effect sizes for two samples and for 2×2 and r×c tables;
- rounding to decimal places and significant figures, and fixed and
  scientific number formatting;
- shuffling and sampling from the VM's seeded generator.

No data representation changes, so research 07's C1 and C2 do not block
it. No existing word's operands or effect change. Existing test words'
answers gain keys (S2), which is the one change to an existing answer.

## Motivation

- **Tests answer no interval.** Every RFC-0015 test answers a MAP of
  `statistic`, `p`, `df` and `method`
  (`crates/bund2-stdlib/src/biomath/infer.rs`, `answer`). A reported
  result needs an estimate and its interval: R's `t.test`, `binom.test`,
  `fisher.test` and `wilcox.test` print one.
- **Every test is two-sided.** RFC-0015 §S6: "Each is two-sided". A
  directional hypothesis, the norm in a pre-registered study, cannot be
  tested.
- **Correlation has no test.** `stat.cor.pearson` and its siblings answer
  the coefficient alone
  (`crates/bund2-stdlib/src/biomath/describe.rs`, `pearson` and
  `stat_cor_pearson`). RFC-0015's own criterion 9 named `cor.test`, and
  its M5 note records that no word answers it
  (`docs/rfc/RFC-0015-biomathematics.md`, the M5 note under §Acceptance
  criteria).
- **Numbers cannot be printed to a precision.** `math.round` rounds to an
  integer (`reference/rust_multistackvm/src/stdlib/math/float_math.rs:57-58`,
  `:127`), and `format` fills a template with each value's STRING
  conversion, with no precision
  (`reference/rust_multistackvm/src/stdlib/string/format.rs:23-35`).
  RFC-0015's example had to define
  `{ 1000000.0 * math.round 1000000.0 swap / }`
  (`bund_examples/biomath/demo.bund`, `:round`).
- **Resampling has no primitives.** A bootstrap or a permutation test
  needs a shuffle and draws with and without replacement; the only
  seeded draws are `rng.uniform` and `dist.sample`
  (`docs/rfc/RFC-0015-biomathematics.md`, §S8).

## Current behaviour

### What the reference does

- **`math.round`** pulls one value, casts it to FLOAT and pushes
  `f64::round` of it (`reference/rust_multistackvm/src/stdlib/math/float_math.rs:93-100`,
  `:126-128`), registered as `math.round` (`:176`). `f64::round` rounds
  half away from zero, so `2.5 math.round` is 3.0. A cast failure is
  `FLOAT_OP returns error: …` (`:155-156`).
- **`format`** pulls a template, then one value per distinct key, each
  converted to a STRING (`format.rs:13-50`), and renders with `leon`
  (`:51-57`). It has no notion of a number's precision; a FLOAT renders
  as its STRING conversion does.

Probed on `target/debug/bund2` while drafting:

    1.23456 3 math.round        → FLOAT_OP returns error: This Dynamic type is not float: 2
    1.23456 math.round          → 1.0

so `math.round` takes one operand and has no places argument.

### What Bund2's own family does

- **Test answers.** `answer(statistic, p, df, method)` builds every
  test's MAP: `statistic`, `p` clamped to [0, 1], `df` when given, and
  `method` (`infer.rs`, `answer`). The two-sided p of a t statistic is
  twice the lower tail of |t| (`infer.rs`, `t_p`).
- **Tests' operands and forms** are RFC-0015 §S6's table and its M5 note:
  `stat.test.t1` (series μ₀), `t2`, `welch`, `paired`, `chisq`,
  `mannwhitney`, `ks2` (two LISTs or METRICS), `chisq.table` and
  `fisher` (a MATRIX), `binom` (k n p); each with a plain and a `.`
  form; `t1` opaque, the rest fixed.
- **Corrections** take one series in four forms and answer in input
  order (`infer.rs`, `bonferroni`, `holm`, `bh`; RFC-0015 §S6).
- **The generator.** `with_rng` runs a closure over the VM's `StdRng`
  (`crates/bund2-stdlib/src/biomath/rng.rs`, `with_rng`); `rng.seed`
  reseeds it (RFC-0015 §S8).
- **Refusals** are RFC-0015 §S11's E101–E115, and no word answers NaN or
  ±∞ from finite operands (B12).
- **Two-series reading.** Two series must each be a LIST or a METRICS;
  a drain is E115 (RFC-0015 §S3).

### What is linked

Nothing this RFC needs is missing. Student's t, the normal, the χ², the
beta and the hypergeometric are `statrs`'s, through `dist.rs`'s
`Family`, which RFC-0015's tests already call (`infer.rs`, `t_p` and
`chisq_p`). Clopper–Pearson's interval is two beta quantiles. Rust's
float formatting rounds the exact binary value to the requested places,
ties to even: `format!("{:.2}", 0.125)` is `0.12` and
`format!("{:.2}", 2.675)` is `2.67` (run with rustc 1.95.0 while
drafting). No crate is added.

## Design

### S1. Shape

- **Bund2's own (D72),** in the default build, beside RFC-0015's words,
  under its prefixes **[B5]**: `stat.` for tests, corrections and effect
  sizes, `rng.` for draws, `math.` for rounding; and `string.` for
  formatting **[A1]**. `bund2 words` lists each; neither health number
  moves.
- **Additive.** No existing word's operands or effect change. The
  reference's `math.round` and `format` are untouched. RFC-0015's test
  answers gain keys (S2) and lose none.
- **Refusals** are RFC-0015 §S11's codes and texts; no new code is
  needed. B12 holds: no word answers NaN or ±∞ from finite operands.
- **Forms** follow RFC-0015 §S3 **[B10]**: one-series words in four
  forms, other consumers plain and `.`, producers none.
- **Operand order** is RFC-0015 §S3's: the deepest operand is the first
  parameter.

### S2. Estimates, intervals and alternatives on the tests [A2] [A3] [A4]

**The plain words gain keys.** Every RFC-0015 test that R's `htest`
gives an estimate or an interval answers them as well, two-sided at 95%,
so a program that reads `p` is unaffected and one that wants the interval
needs no new word:

| Word | `estimate` | Interval |
|---|---|---|
| `stat.test.t1` | the mean | t interval for the mean |
| `stat.test.t2` | mean(a) − mean(b) | pooled t interval for the difference |
| `stat.test.welch` | mean(a) − mean(b) | Welch interval, Welch–Satterthwaite df |
| `stat.test.paired` | the mean difference | t interval for it |
| `stat.test.binom` | k/n | Clopper–Pearson |
| `stat.test.fisher` | the conditional MLE of the odds ratio | its exact conditional interval, as R's `fisher.test` |
| `stat.test.mannwhitney` | the Hodges–Lehmann shift, the median of the n·m differences a − b | R's asymptotic interval, by inverting the normal approximation with its continuity correction |

`stat.test.chisq`, `stat.test.chisq.table` and `stat.test.ks2` gain
none, as R gives none.

**Keys.** `estimate` (a FLOAT); `lower` and `upper`, the interval's
finite ends **[A3]**; `conf`, the level (0.95); and `alternative`
(`"two.sided"`, `"less"` or `"greater"`). An end that is infinite, as a
one-sided interval's far end is, or a Fisher odds ratio's at the
support's ends, is left out rather than answered as ±∞, so B12 holds and
a program asks `"upper" get` and meets NODATA, as for any absent key. An
estimate that is infinite (Fisher's, when a cell is 0) is left out the
same way.

**`.opts` variants [A2].** Each of the ten tests gains
`<word>.opts`, the plain word's operands with an options MAP above them:

| Key | Values | Default | Tests |
|---|---|---|---|
| `alternative` | `"two.sided"`, `"less"`, `"greater"` | `"two.sided"` | all but `chisq` and `chisq.table` |
| `conf` | a FLOAT in (0, 1) | 0.95 **[A4]** | those with an interval |
| `mu` | the difference under H₀ | 0 | `t2`, `welch`, `paired`, `mannwhitney` |

`t1`'s μ₀ and `binom`'s p stay operands, as now. An unknown key, or a
key the test does not take, is E101 naming it, as `ode.rk45`'s options
are (RFC-0015 §S15's M11 note). "less" means the first parameter is
less: for `a b stat.test.t2.opts`, mean(a) − mean(b) < μ.

**One-sided p.** The t tests use one tail of t. The binomial and Fisher's
tests use one tail of their exact distribution, as R does. The
Mann–Whitney test uses one tail of the normal approximation with the
continuity correction toward the null. The two-sample KS test uses the
one-sided statistic D⁺ or D⁻ and the asymptotic tail exp(−2·nm/(n + m)·D²).

**Work (D39).** The Hodges–Lehmann estimate forms n·m differences;
above 10⁸ it is E111 before allocating, as Fisher's support is. Fisher's
conditional MLE and its interval are roots of a monotone function of the
odds ratio, found by bisection on its logarithm to 10⁻¹⁰ relative, at
most 200 steps, else E110.

**Forms.** Each `.opts` word has a plain and a `.` form; the `.` form
takes the options MAP from the workbench, as D111's "anything else" row
gives the top operand. `stat.test.t1.opts` reads its series by
`get_data` and is opaque, as `stat.test.t1` is; the rest are fixed.

### S3. Correlation tests [A5]

`stat.cor.test.pearson`, `stat.cor.test.spearman` and
`stat.cor.test.kendall` (`a b -- MAP`), and their `.opts` variants with
`alternative` and `conf` (Pearson only), each answering `estimate`,
`statistic`, `p`, `method`, `alternative`, and:

- **Pearson:** t = r·√((n − 2)/(1 − r²)) on n − 2 df (`df`), and Fisher's
  z interval, tanh(atanh r ± z₁₋α/₂/√(n − 3)), for n ≥ 4 (`lower`,
  `upper`, `conf`).
- **Spearman:** ρ, the statistic S = (n³ − n)(1 − ρ)/6, and the p of the
  t approximation on n − 2 df, R's `exact = FALSE` **[A5]**.
- **Kendall:** τ-b, as `stat.cor.kendall`; with no ties and n < 50, the
  exact p of T, the number of concordant pairs, by the recurrence R's
  `ckendall` uses; otherwise the normal approximation with the
  tie-corrected variance, as R. `statistic` is T or z accordingly.

A perfect correlation (|r| = 1) answers p from the limit and leaves out
an infinite t, as S2 leaves out infinite ends. Fewer than three pairs,
or a constant series, is E101.

### S4. Further corrections

`stat.padjust.hochberg`, `stat.padjust.hommel` and `stat.padjust.by`
(`p-list -- p-list`), in RFC-0015's four forms, answering in input order
as R's `p.adjust` does with those methods. A p outside [0, 1] is E101;
an empty LIST answers an empty LIST.

### S5. Effect sizes

| Word | Operands | Answers |
|---|---|---|
| `stat.effect.cohend` | a b | `d` (pooled SD), `g` (Hedges' correction J = Γ(df/2)/(√(df/2)·Γ((df − 1)/2)), df = n_a + n_b − 2) |
| `stat.effect.oddsratio` | 2×2 MATRIX | `estimate`, `log`, `se` (Woolf's), `lower`, `upper`, `conf` 0.95 |
| `stat.effect.riskratio` | 2×2 MATRIX, rows exposed then unexposed, first column the event | `estimate`, `log`, `se`, `lower`, `upper`, `conf` |
| `stat.effect.cramerv` | r×c MATRIX | V = √(χ²/(n·(min(r, c) − 1))), without Yates' correction |

A zero cell, which makes an odds or risk ratio 0 or infinite, is E101
naming the cell **[B12]**; a program wanting Haldane's correction adds
0.5 itself. Intervals for d and g need the noncentral t, which `statrs`
lacks (research 07, §2), and are left to research 07's Phase E. Each
word has a plain and a `.` form and a fixed effect.

### S6. Rounding and formatting [A1] [A6]

| Word | Operands | Answers |
|---|---|---|
| `math.round.places` | x n | x rounded to n decimal places, n an INTEGER in [0, 15] |
| `math.signif` | x n | x rounded to n significant digits, n in [1, 17] |
| `string.fixed` | x n | a STRING with exactly n decimals, n in [0, 17] |
| `string.sci` | x n | a STRING in scientific notation with n decimals in the mantissa, n in [0, 16] |

- **x** is a number, an INTEGER converting **[B15]**, or a LIST of them,
  answered element-wise as a LIST; a non-finite or non-number element is
  E101 naming its position.
- **Ties [A6].** The exact binary value is rounded, ties to even, which
  is what Rust's formatter does (§Current behaviour) and what R's
  `round` and `signif` document since R 4.0.0 [UNGROUNDED: R's
  documentation, not in `reference/`; Q62]. So `2.675 2
  math.round.places` is 2.67, because 2.675 is stored as
  2.67499999999999982236431605997495353221893310546875, and
  `2.5 0 math.round.places` is 2.0 where `2.5 math.round` is 3.0. The
  reference's word keeps its own rule.
- **Implementation.** `math.round.places` parses `format!("{:.n$}", x)`,
  and `math.signif` parses `format!("{:.m$e}", x)` with m = n − 1, so
  each is exactly the formatter's rounding. `string.fixed` and
  `string.sci` are those strings; `string.sci`'s exponent is written as
  Rust writes it, `1.235e3`, `1.0e-7` (with n = 1).
- **Effects** are fixed; each has a plain and a `.` form, the `.` form
  taking n from the workbench.

### S7. Shuffling and sampling

| Word | Operands | Answers |
|---|---|---|
| `rng.shuffle` | list | the LIST in a random order, by Fisher–Yates |
| `rng.choice` | list k | k elements without replacement, in draw order; k ≤ the length |
| `rng.choice.replace` | list k | k elements with replacement |
| `rng.choice.weighted` | list weights k | k elements with replacement, each with probability its weight over their sum |

- **Draws** come from the VM's generator (`with_rng`), so `rng.seed`
  repeats them within one Bund2 version, as RFC-0015 §S8 says of every
  drawing word.
- **The list** may hold any values; they are moved, not converted.
  `weights` is a LIST of finite non-negative numbers, as long as `list`,
  with a positive sum (E101, E102).
- **Work (D39).** k above 10⁸ is E111 before allocating. `rng.choice`
  uses a partial Fisher–Yates over a copy, so its work is the list's
  length.
- **Forms.** Each consumes operands, so has a plain and a `.` form, and
  a fixed effect. `rng.choice.weighted` and its form join the
  unvaried-operand audit's list (`crates/bund2-stdlib/src/lib.rs`,
  `UNVARIED`), its first operand being the list drawn from; so do the
  three-operand `.opts` words of S2 and S3.

### S8. The tiers and the audits

As RFC-0015 §S17: effects are declared as S1 says, the palette audit
decides what `PROMOTABLE.txt` lists (F123), and `PROMOTABLE.txt`,
`bund2 effects` and the unvaried-operand list are regenerated in each
milestone. No golden is added (D21).

## Preservation analysis

| Behaviour | Disposition |
|---|---|
| `math.round` (`float_math.rs:57-58,127,176`): one operand, half away from zero | **Preserved exactly.** `math.round.places` is a new name with its own tie rule [A6] |
| `format` (`format.rs:9-68`) | **Preserved exactly.** `string.fixed` and `string.sci` are new names |
| RFC-0015's test words' operands, effects, forms, refusals and existing keys | **Preserved exactly** |
| RFC-0015's test answers | **Deliberately changed, additively:** they gain `estimate`, `lower`, `upper`, `conf` and `alternative` where S2 says [A2]. A program that compares a whole answer MAP, or counts its keys, sees the difference; RFC-0015's unit tests that assert the key set change with them |
| `stat.cor.*` | **Preserved exactly.** The tests are new names |
| RFC-0015's corrections | **Preserved exactly** |
| The VM's generator and every seeded stream of RFC-0015's words | **Preserved exactly:** the new words draw from the same generator only when called |
| `conform` 143/159, CEILING 143/159; IMPLEMENTED 597/602, COVERAGE 577/602 | **Preserved exactly** (D72): no reference word is touched |

## Alternatives considered

- **New words for intervals (`stat.ci.t2` …)** instead of keys on the
  tests. Rejected under A2's recommendation: R reports the interval with
  the test, and two words would compute the same quantities twice.
- **A trailing optional options MAP on the plain test words.** A word
  whose arity depends on its top operand cannot declare a fixed effect,
  so every test would become opaque and leave the palette audit (S8).
- **±∞ for a one-sided interval's far end,** as R prints `-Inf`. Rejected
  by B12, which this RFC keeps; A3 weighs it.
- **A `places` operand on `math.round`.** It would change a reference
  word's arity and its golden behaviour, so it would be a deviation, not
  an addition.
- **Spearman's exact p** by AS 89, R's default for n < 1290 without ties.
  A5 weighs it against the t approximation; RFC-0015's KS test made the
  same choice for its asymptotic p.
- **Negative places** (`1234 -2 math.round.places` → 1200, as R allows).
  Rounding x/10² through the formatter is not exact, and the use is rare;
  E101 until asked.
- **Weighted sampling without replacement.** R's sequential algorithm is
  order-dependent and its probabilities are not the weights; left out
  until a use names which definition it wants.

## Acceptance criteria

Each is a named test in `crates/bund2-stdlib` unless placed otherwise.
Expected values are stated, or from a committed fixture computed from the
definitions with mpmath as RFC-0015's M5 and M12 fixtures were, beside R's
printed output for the same data where R's help pages print it **[A7]**.
Tolerance: 1e-10 relative for closed forms, 1e-8 for roots and series.

1. **Registered.** Every word of S2–S7 and its forms is in the registry
   with the effect S1 and each section give; the count of new names is
   stated in the test.
2. **Refusals.** Each word's E1xx refusals render their templates; no
   operand drawn from a palette of NaN, ±∞, wrong types, empty and
   ragged values aborts.
3. **Additive answers.** For each of RFC-0015's ten tests, every key it
   answered before is unchanged in value on RFC-0015's fixtures, and the
   new keys are present exactly as S2's table says.
4. **t intervals.** On R's `sleep` data, `stat.test.t2`, `welch` and
   `paired` answer the interval R's `t.test` prints for the same call, to
   R's printed digits, and the fixture's to 1e-10; a one-sided `.opts`
   call answers one finite end and the p of one tail.
5. **Exact intervals.** `stat.test.binom` on `?binom.test`'s example and
   `stat.test.fisher` on `?fisher.test`'s `TeaTasting` answer R's printed
   estimate and interval; a Fisher table with a zero cell leaves out the
   infinite end.
6. **Hodges–Lehmann.** `stat.test.mannwhitney` answers the median of the
   pairwise differences on a fixture, and the interval of the fixture's
   inversion of the normal approximation; n·m above 10⁸ is E111 before
   allocation.
7. **Correlation tests.** On a committed fixture, Pearson's t, df, p and
   Fisher-z interval; Spearman's S and t-approximation p; Kendall's exact
   p for n < 50 without ties and its tie-corrected normal p otherwise.
8. **Corrections.** `stat.padjust.hochberg`, `.hommel` and `.by` on a
   fixture of 20 p-values match the definitions, in input order.
9. **Effect sizes.** Cohen's d and Hedges' g on `sleep`; the odds and
   risk ratios with Woolf intervals on a 2×2 fixture; Cramér's V on a
   3×3; a zero cell is E101.
10. **Rounding.** `2.675 2 math.round.places` is 2.67,
    `0.125 2 math.round.places` 0.12, `2.5 0 math.round.places` 2.0,
    `123456 2 math.signif` 120000.0, `[ 1.23456 2.5 ] 3 math.round.places`
    `[1.235, 2.5]`; `3.14159 2 string.fixed` is `"3.14"` and
    `12345.678 2 string.sci` is `"1.23e4"`; and `2.5 math.round` is still
    3.0.
11. **Sampling.** After `7 rng.seed`, each S7 word answers the same
    twice; `rng.shuffle` answers a permutation of its list;
    `rng.choice` draws no element twice; over 10⁵ seeded draws
    `rng.choice.weighted`'s frequencies are within 4 SE of the weights.
12. **Health.** `cargo xtask conform` and `coverage` print the numbers
    of §Preservation analysis; clippy and `cargo deny` are clean;
    `PROMOTABLE.txt` is regenerated.

## Implementation plan

| M | Delivers |
|---|---|
| M1 | S6 (rounding, formatting) and S4 (corrections); criteria 1, 2, 8, 10 for them |
| M2 | S2's plain-word keys and `.opts` for the t tests and the binomial; criteria 3, 4, part of 5 |
| M3 | S2 for Fisher's and the Mann–Whitney test; criteria 5, 6 |
| M4 | S3; criterion 7 |
| M5 | S5 and S7; criteria 9, 11 |

Each ends with criterion 12.

## Rulings needed — D251 (OPEN)

| | Question | Options | Recommended |
|---|---|---|---|
| **A1** | Names for rounding and formatting | **A** `math.round.places`, `math.signif`, `string.fixed`, `string.sci`. **B** one `math.format` taking a mode | **A**: one word, one job, beside the reference's `math.` and `string.` |
| **A2** | Where intervals go | **A** keys on the plain tests, at 95% two-sided, plus `.opts` variants. **B** separate `stat.ci.*` words. **C** `.opts` variants only, plain answers unchanged | **A**: R reports both together; adding keys breaks no reader of `p` |
| **A3** | An infinite interval end | **A** leave the key out. **B** ±∞ (against B12). **C** NODATA in a two-element LIST | **A**: B12 stands, and an absent key already reads as NODATA |
| **A4** | Default confidence level | **A** 0.95. **B** none, `conf` required | **A**, R's |
| **A5** | Spearman's p | **A** the t approximation (R's `exact = FALSE`). **B** AS 89's exact p for n < 1290 without ties, R's default | **A** for this RFC; B when a fixture can check AS 89 |
| **A6** | Ties in rounding to places | **A** the exact binary value, ties to even (Rust's formatter; R ≥ 4.0.0). **B** half away from zero, as `math.round` | **A**: it is what a statistician expects from R, and it is exact |
| **A7** | Reference values | **A** mpmath fixtures from the definitions, plus R's printed help-page outputs, as RFC-0015's M5. **B** install R and commit its output | **A** now; B when R is available, as research 07's C4 asks generally |
| **A8** | Which build | **A** the default build, adding no crate, as B1 for RFC-0015. **B** a feature | **A** |

## Open questions

- **Q62** (`docs/registers/open-questions.md`): the R algorithms this RFC
  reproduces — `fisher.test`'s conditional MLE and interval,
  `wilcox.test`'s asymptotic Hodges–Lehmann interval, `cor.test`'s
  Kendall exact p, and `round`'s tie rule since R 4.0.0 — are stated from
  R's documentation, not from source in `reference/`. Each milestone's
  fixture is the check (A7).
- Research 07's C1 and C2 (tables, missing values) do not block this
  RFC; its words take LISTs, METRICS and MATRIX as RFC-0015's do.
