# inference

RFC-0019's study, from planning to a likelihood: a sample size planned by
power, a three-diet trial tested, a mean and a difference resampled, and
two models fitted by maximum likelihood.

    make            # run demo.bund
    make check      # run it and compare the output with expected.txt

or, without `make`, from this directory:

    bund2 demo.bund

Every draw follows `42 rng.seed`, so a run prints the same lines each time.
The numbers are rounded to six decimals before they are printed.

## The output

`expected.txt` is what a run prints. A test in `crates/bund2-cli/tests`,
`inference_demo.rs`, runs the demo and compares the two. It is not a
golden: the reference has none of these words, so there is no oracle to
capture one from.

| | What it shows | Words | It prints |
|---|---|---|---|
| 1 | How many animals per diet a two-sample t test needs for a 0.8 g gain with sd 0.6 at 90% power; the power of the eight actually used; the one-way plan | `power.t`, `power.anova` | n 12.861845, so 13; power 0.698722 at n = 8; 6.809607 per group |
| 2 | Each diet checked for normality; whether the diets differ, and by how much; every pair compared; diet 2 against diet 1 as an effect size | `stat.test.shapiro`, `stat.test.anova1`, `stat.test.tukey`, `stat.effect.cohend.opts` | F 11.452741 on 2 and 21, p 0.000433; Tukey's three pairs, 2-1 and 3-2 apart; d 2.112213, 0.839869 to 3.338418 |
| 3 | Diet 2's mean bootstrapped 2000 times; the jackknife's se; diets 1 and 2 permuted, every split and then 9999 shuffles | `rng.seed`, `boot.sample`, `boot.jackknife`, `stat.test.permutation` | percentile and BCa intervals; exact p 0.001554 over 12870 splits, Monte Carlo 0.0022 |
| 4 | Diet 2 as normal, against its closed form; ten waits as exponential, the rate's profile interval beside its Wald one | `fit.mle` | μ 6.125, σ 0.446514; rate 0.8, profile 0.400861 to 1.403148, Wald 0.304165 to 1.295836 |

The power, Shapiro–Wilk, ANOVA, Tukey and exact permutation numbers are
R 4.6.2's (`power.t.test`, `power.anova.test`, `shapiro.test`,
`oneway.test`, `TukeyHSD`, and a count over `combn(16, 8)`). The
bootstrap and Monte Carlo draws are Bund2's own generator's, so R's
`sample` would draw others.

## Three idioms

- **A power word solves for the key left out.** Give every parameter but
  one; `power.t` with no `n` answers `n`, with no `power` answers the
  power. `n` is a FLOAT, as R's; `math.ceil` rounds it up to a design.
- **A statistic is a name or a lambda.** `boot.sample` takes `"mean"`,
  `"median"`, `"sd"` or `"var"`, or a lambda `resample -- FLOAT`;
  `stat.test.permutation` takes `"mean.diff"`, `"median.diff"` or `"t"`,
  or a lambda `a b -- FLOAT`.
- **`fit.mle` maximises a log-likelihood lambda `par -- FLOAT`.** `par`
  is a LIST; `unfold` spreads it on the stack, last parameter on top.
  The fit is R's `optim` BFGS by default, `"method" "nm"` for
  Nelder–Mead, with R's `reltol`, `ndeps` and `maxit`. At R's defaults
  BFGS stops near 10⁻⁵ of the optimum, as R's own does (6.124967 for this
  mean), so section 4 tightens them. `"profile" true` adds the
  likelihood-ratio interval for each parameter.
