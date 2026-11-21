# models

RFC-0018's models over frames: a design coded from a frame, a linear model
and a Poisson one, their residuals and influence, predictions at new rows,
and nested models compared.

    make            # run demo.bund
    make check      # run it and compare the output with expected.txt

or, without `make`, from this directory:

    bund2 demo.bund

Nothing is drawn at random, so a run prints the same lines each time. The
numbers are rounded to six decimals before they are printed.

## The output

`expected.txt` is what a run prints. A test in `crates/bund2-cli/tests`,
`models_demo.rs`, runs the demo and compares the two. It is not a golden:
the reference has no model words, so there is no oracle to capture one
from.

| | What it shows | Words | It prints |
|---|---|---|---|
| 1 | The dose trial of `frames/` coded as a design: an intercept, dose, and `site` one column per level after `east`; the two rows with no response left out under `"omit"` | `frame.csv.parse`, `model.matrix` | four columns, 2 dropped, the seven responses |
| 2 | Least squares over the design | `fit.lm` | coefficients 3.2, 0.15, −0.466667, −0.9; r² 0.993611; F 155.514286 on p 0.000865 |
| 3 | Each row's residual, leverage and Cook's distance | `model.residuals`, `model.influence` | deviance and standardized residuals, leverages, Cook's distances |
| 4 | Predictions at two new rows, with 95% confidence and prediction intervals; a site the fit never saw, refused | `model.predict` | 4.983333 and 6.95, the intervals, the refusal naming `"west"` |
| 5 | Dobson's trial, R's `glm` example, as a Poisson model | `fit.glm`, `model.predict` | R's coefficients, deviance 5.129141, null deviance 10.581446, AIC 56.761318, 4 iterations, the expected counts |
| 6 | Outcome alone against outcome and treatment, and the sequential table | `model.compare`, `model.anova`, `frame.column` | LR on 2 df, deviance 0 and p 1; residual deviances 10.581446, 5.129141, 5.129141 |

## Three idioms

- **A design and a fit are plain MAPs.** A design holds the MATRIX, the
  column names, `y`, the weights and offset, and `spec`, the record of how
  the frame was coded. A fit keeps that `spec`, so `model.predict` codes a
  new frame the same way, and refuses a level the fit never saw.
- **A factor is a column of STRINGs.** `frame.csv.parse`'s `factors`
  refuses a numeric column, so codes such as 1, 2, 3 are written `o1`,
  `o2`, `o3`. The first level is the baseline.
- **`fit.glm` needs its family.** `dict "family" "poisson" set`; the link
  defaults to the family's canonical one, `log` here. `model.compare` and
  `model.anova` choose the test from the family: F when its dispersion is
  estimated, likelihood ratio when it is fixed, as here.
