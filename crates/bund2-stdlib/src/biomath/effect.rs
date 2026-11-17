//! RFC-0016 §S5: effect sizes, `stat.effect.cohend`, `.oddsratio`,
//! `.riskratio` and `.cramerv`.
//!
//! - **`cohend`** (`a b -- MAP`) answers `d`, the difference of the means
//!   over `t2`'s pooled SD, and `g`, Hedges' J·d. J is
//!   exp(lgamma(df/2) − lgamma((df − 1)/2))/√(df/2) up to df 200, and above
//!   that the series Γ(z + ½)/Γ(z) = √z·(1 − 1/(8z) + 1/(128z²) +
//!   5/(1024z³) − 21/(32768z⁴)), z = (df − 1)/2, over √(df/2), since the
//!   lgamma difference cancels as df grows.
//! - **`oddsratio`** and **`riskratio`** (`table -- MAP`) answer the sample
//!   ratio, its `log`, Woolf's or Katz's `se`, and exp(`log` ∓
//!   z₀.₉₇₅·`se`) as `lower` and `upper` at `conf` 0.95, with a `method`
//!   naming which ratio it is: `stat.test.fisher.opts`'s odds ratio is the
//!   conditional MLE, a different estimate on the same table.
//! - **`cramerv`** (`table -- V`) is √(χ²/(n·(min(r, c) − 1))), χ² as
//!   `stat.test.chisq.table` computes it, with its refusals.
//!
//! A zero cell that makes a ratio 0 or infinite is E101 naming the cell
//! [B12]; a program wanting Haldane's correction adds 0.5 itself.

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;

use super::describe::{mean, variance};
use super::dist::Family;
use super::infer::{chisq_table, counts};
use super::operand::{at_least, domain, finite, operands, two, two_forms};
use super::refuse::Refusal;
use super::special::lgamma;

/// Where J switches from the lgamma difference to the series.
const J_SERIES_ABOVE: f64 = 200.0;

/// Hedges' correction J at `df`.
fn hedges_j(df: f64) -> f64 {
    if df <= J_SERIES_ABOVE {
        (lgamma(df / 2.0) - lgamma((df - 1.0) / 2.0)).exp() / (df / 2.0).sqrt()
    } else {
        let z = (df - 1.0) / 2.0;
        let ratio = z.sqrt()
            * (1.0 - 1.0 / (8.0 * z) + 1.0 / (128.0 * z * z) + 5.0 / (1024.0 * z * z * z)
                - 21.0 / (32768.0 * z * z * z * z));
        ratio / (df / 2.0).sqrt()
    }
}

/// `stat.effect.cohend`: `d` and Hedges' `g`.
fn stat_effect_cohend(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.effect.cohend";
    let (a, b) = two(vm, W)?;
    at_least(W, &a, 2)?;
    at_least(W, &b, 2)?;
    let (n1, n2) = (a.len() as f64, b.len() as f64);
    let df = n1 + n2 - 2.0;
    let pooled = ((n1 - 1.0) * variance(&a) + (n2 - 1.0) * variance(&b)) / df;
    let s = finite(W, pooled.sqrt(), || "the pooled SD".into())?;
    if s == 0.0 {
        return Err(domain(W, "a and b", "a constant series".into()));
    }
    let d = finite(W, (mean(&a) - mean(&b)) / s, || "d".into())?;
    let g = hedges_j(df) * d;
    let mut m = BTreeMap::new();
    m.insert("d".to_string(), BundValue::float(d));
    m.insert("g".to_string(), BundValue::float(g));
    vm.push(BundValue::map(m));
    Ok(())
}

/// A 2×2 table of counts, as `[a, b, c, d]`: E101 for a negative or
/// non-finite cell, E102 for any other shape.
fn two_by_two(word: &str, v: &BundValue) -> Result<[f64; 4], Error> {
    let t = counts(word, v)?;
    if t.len() != 2 {
        return Err(Refusal::Disagree {
            what: "rows".into(),
            n: t.len(),
            m: 2,
        }
        .error(word));
    }
    if t[0].len() != 2 {
        return Err(Refusal::Disagree {
            what: "columns".into(),
            n: t[0].len(),
            m: 2,
        }
        .error(word));
    }
    Ok([t[0][0], t[0][1], t[1][0], t[1][1]])
}

/// The cell `k` of a 2×2 table, 0-based by rows, as `[i,j]`.
fn cell(k: usize) -> String {
    format!("[{},{}]", k / 2 + 1, k % 2 + 1)
}

/// A zero cell the ratio cannot take: E101 naming it.
fn refuse_zero(word: &str, cells: &[f64; 4], which: &[usize]) -> Result<(), Error> {
    match which.iter().find(|&&k| cells[k] == 0.0) {
        Some(&k) => Err(domain(word, "table", format!("0 at {}", cell(k)))),
        None => Ok(()),
    }
}

/// A ratio's answer: the estimate, its log, se, and the interval at 95%.
fn ratio(word: &str, estimate: f64, se: f64, method: &str) -> Result<BundValue, Error> {
    let estimate = finite(word, estimate, || "the ratio".into())?;
    let log = estimate.ln();
    let se = finite(word, se, || "the standard error".into())?;
    let z = Family::Normal { mean: 0.0, sd: 1.0 }.quantile(word, 0.975)?;
    // At se = 0 the interval is [estimate, estimate], which exp(ln x) can
    // miss by an ulp.
    let (lower, upper) = if se == 0.0 {
        (estimate, estimate)
    } else {
        (
            finite(word, (log - z * se).exp(), || {
                "the interval's lower end".into()
            })?,
            finite(word, (log + z * se).exp(), || {
                "the interval's upper end".into()
            })?,
        )
    };
    let mut m = BTreeMap::new();
    m.insert("estimate".to_string(), BundValue::float(estimate));
    m.insert("log".to_string(), BundValue::float(log));
    m.insert("se".to_string(), BundValue::float(se));
    m.insert("lower".to_string(), BundValue::float(lower));
    m.insert("upper".to_string(), BundValue::float(upper));
    m.insert("conf".to_string(), BundValue::float(0.95));
    m.insert("method".to_string(), BundValue::str(method));
    Ok(BundValue::map(m))
}

/// `stat.effect.oddsratio`: ad/(bc), Woolf's interval. Any zero cell is
/// E101.
fn stat_effect_oddsratio(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.effect.oddsratio";
    let v = operands(vm, W, 1)?;
    let cells = two_by_two(W, &v[0])?;
    refuse_zero(W, &cells, &[0, 1, 2, 3])?;
    let [a, b, c, d] = cells;
    let y = ratio(
        W,
        a * d / (b * c),
        (1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d).sqrt(),
        "Sample odds ratio; Woolf's interval",
    )?;
    vm.push(y);
    Ok(())
}

/// `stat.effect.riskratio`: (a/(a + b))/(c/(c + d)), Katz's interval, rows
/// exposed then unexposed, the first column the event. a = 0 or c = 0 is
/// E101; b = 0 and d = 0 are answered.
fn stat_effect_riskratio(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.effect.riskratio";
    let v = operands(vm, W, 1)?;
    let cells = two_by_two(W, &v[0])?;
    refuse_zero(W, &cells, &[0, 2])?;
    let [a, b, c, d] = cells;
    // Each difference is ≥ 0 after rounding too, since rounding is
    // monotone: 1/a ≥ 1/(a + b). Both are 0 when b = d = 0.
    let var = 1.0 / a - 1.0 / (a + b) + 1.0 / c - 1.0 / (c + d);
    let y = ratio(
        W,
        (a / (a + b)) / (c / (c + d)),
        var.sqrt(),
        "Risk ratio; Katz's interval",
    )?;
    vm.push(y);
    Ok(())
}

/// `stat.effect.cramerv`: √(χ²/(n·(min(r, c) − 1))).
fn stat_effect_cramerv(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.effect.cramerv";
    let v = operands(vm, W, 1)?;
    let t = chisq_table(W, &v[0])?;
    let k = t.rows.min(t.cols) as f64 - 1.0;
    let y = finite(W, (t.x2 / (t.n * k)).sqrt(), || "V".into())?;
    vm.push(BundValue::float(y));
    Ok(())
}

/// Register §S5's words, each plain and `.`: `cohend.` takes b from the
/// workbench, the others their one operand.
pub fn register(r: &mut Registry) {
    two_forms!(r, "stat.effect.cohend", stat_effect_cohend);
    macro_rules! word {
        ($name:literal, $f:expr) => {
            r.register_native($name, $f, StackEffect::fixed(1, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, 1, 0);
        };
    }
    word!("stat.effect.oddsratio", stat_effect_oddsratio);
    word!("stat.effect.riskratio", stat_effect_riskratio);
    word!("stat.effect.cramerv", stat_effect_cramerv);
}

/// §S5's words with their forms. RFC-0015's counting test for `describe`
/// leaves them out by this list (§Preservation analysis).
pub const WORDS: [&str; 8] = [
    "stat.effect.cohend",
    "stat.effect.cohend.",
    "stat.effect.oddsratio",
    "stat.effect.oddsratio.",
    "stat.effect.riskratio",
    "stat.effect.riskratio.",
    "stat.effect.cramerv",
    "stat.effect.cramerv.",
];

#[cfg(test)]
mod tests;
