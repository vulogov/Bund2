//! RFC-0015 §S6's tests, corrections and model: `stat.test.*`,
//! `stat.padjust.*` and `stat.lm`.
//!
//! **A test answers a MAP** (§S2) holding `statistic`, `p`, `df` where the
//! test has one, and `method`, R's name for it. Every test is two-sided.
//! Each follows the convention §S6's table states, R's where R's default is
//! a choice:
//!
//! - the t tests: one-sample, Student's pooled, Welch's, paired;
//! - χ² goodness of fit, with `expected` as probabilities when it sums to 1
//!   within 1e-9 and as counts otherwise, and χ² on a contingency table with
//!   no Yates correction;
//! - Fisher's exact test on a 2×2 table and the exact binomial test, each
//!   two-sided by R's rule: the outcomes whose probability is at most the
//!   observed one's × (1 + 10⁻⁷);
//! - Mann–Whitney by the normal approximation, tie- and
//!   continuity-corrected, its statistic W for `a`;
//! - the two-sample Kolmogorov–Smirnov test, its p the asymptotic one.
//!
//! **What `statistic` is where R has none.** Fisher's test answers the
//! count in the table's first cell, the hypergeometric variable the test
//! conditions on; R reports the conditional MLE of the odds ratio instead,
//! which is infinite at the support's ends (B12). The binomial test answers
//! k. §S6's note of 2026-11-15 records both.
//!
//! **The p values come from §S7's own tails** (`dist`), the t's, the χ²'s and
//! the normal's, so they share its accuracy and its refusals.

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm};
use bund2_value::{BundValue, FLOAT, MATRIX};

use super::describe::{mean, ranks, variance};
use super::dist::Family;
use super::operand::{
    WIDTH, at_least, domain, finite_all, four_forms, list, operands, real, same_length,
    scalar_forms, two, two_forms, whole,
};
use super::refuse::Refusal;

/// Fisher's and the binomial test's tolerance for "as probable as the
/// observed outcome", R's.
const REL_ERR: f64 = 1.0 + 1e-7;

/// The most outcomes Fisher's test sums (D39).
const SUPPORT_CAP: u64 = 100_000_000;

// The answer.

fn answer(statistic: f64, p: f64, df: Option<f64>, method: &str) -> BundValue {
    let mut m = BTreeMap::new();
    m.insert("statistic".to_string(), BundValue::float(statistic));
    m.insert("p".to_string(), BundValue::float(p.clamp(0.0, 1.0)));
    if let Some(df) = df {
        m.insert("df".to_string(), BundValue::float(df));
    }
    m.insert("method".to_string(), BundValue::str(method));
    BundValue::map(m)
}

fn constant(word: &str, operand: &str) -> Error {
    domain(word, operand, "a constant series".into())
}

/// A statistic that is not finite: only an overflow can make one from
/// finite operands.
fn statistic(word: &str, s: f64) -> Result<f64, Error> {
    if s.is_finite() {
        Ok(s)
    } else {
        Err(Refusal::Overflow {
            expression: "the test statistic".into(),
        }
        .error(word))
    }
}

/// The two-sided p of a t statistic.
fn t_p(word: &str, t: f64, df: f64) -> Result<f64, Error> {
    Ok(2.0 * Family::T { df }.tails(word, -t.abs())?.0)
}

/// The upper tail of a χ² statistic.
fn chisq_p(word: &str, x: f64, df: f64) -> Result<f64, Error> {
    Ok(Family::ChiSq { df }.tails(word, x)?.1)
}

// The t tests.

fn t_one(word: &str, xs: &[f64], mu: f64, method: &str) -> Result<BundValue, Error> {
    at_least(word, xs, 2)?;
    let n = xs.len() as f64;
    let se = (variance(xs) / n).sqrt();
    if se == 0.0 {
        return Err(constant(word, "series"));
    }
    let t = statistic(word, (mean(xs) - mu) / se)?;
    let df = n - 1.0;
    Ok(answer(t, t_p(word, t, df)?, Some(df), method))
}

fn stat_test_t1(word: &str, xs: &[f64], mu: f64) -> Result<BundValue, Error> {
    t_one(word, xs, mu, "One Sample t-test")
}

fn t2(word: &str, a: &[f64], b: &[f64]) -> Result<BundValue, Error> {
    at_least(word, a, 2)?;
    at_least(word, b, 2)?;
    let (n1, n2) = (a.len() as f64, b.len() as f64);
    let df = n1 + n2 - 2.0;
    let pooled = ((n1 - 1.0) * variance(a) + (n2 - 1.0) * variance(b)) / df;
    let se = (pooled * (1.0 / n1 + 1.0 / n2)).sqrt();
    if se == 0.0 {
        return Err(constant(word, "a and b"));
    }
    let t = statistic(word, (mean(a) - mean(b)) / se)?;
    Ok(answer(t, t_p(word, t, df)?, Some(df), "Two Sample t-test"))
}

fn welch(word: &str, a: &[f64], b: &[f64]) -> Result<BundValue, Error> {
    at_least(word, a, 2)?;
    at_least(word, b, 2)?;
    let (n1, n2) = (a.len() as f64, b.len() as f64);
    let (v1, v2) = (variance(a) / n1, variance(b) / n2);
    let se2 = v1 + v2;
    if se2 == 0.0 {
        return Err(constant(word, "a and b"));
    }
    let df = statistic(
        word,
        se2 * se2 / (v1 * v1 / (n1 - 1.0) + v2 * v2 / (n2 - 1.0)),
    )?;
    let t = statistic(word, (mean(a) - mean(b)) / se2.sqrt())?;
    Ok(answer(
        t,
        t_p(word, t, df)?,
        Some(df),
        "Welch Two Sample t-test",
    ))
}

fn paired(word: &str, a: &[f64], b: &[f64]) -> Result<BundValue, Error> {
    same_length(word, a, b)?;
    let d: Vec<f64> = a.iter().zip(b).map(|(x, y)| x - y).collect();
    finite_all(word, "a - b", &d)?;
    t_one(word, &d, 0.0, "Paired t-test")
}

// χ².

fn chisq(word: &str, observed: &[f64], expected: &[f64]) -> Result<BundValue, Error> {
    same_length(word, observed, expected)?;
    at_least(word, observed, 2)?;
    if let Some(i) = observed.iter().position(|o| *o < 0.0) {
        return Err(domain(
            word,
            "observed",
            format!("{} at {}", observed[i], i + 1),
        ));
    }
    if let Some(i) = expected.iter().position(|e| *e <= 0.0) {
        return Err(domain(
            word,
            "expected",
            format!("{} at {}", expected[i], i + 1),
        ));
    }
    let n: f64 = observed.iter().sum();
    let probabilities = (expected.iter().sum::<f64>() - 1.0).abs() <= 1e-9;
    let x2: f64 = observed
        .iter()
        .zip(expected)
        .map(|(o, e)| {
            let e = if probabilities { n * e } else { *e };
            (o - e) * (o - e) / e
        })
        .sum();
    let x2 = statistic(word, x2)?;
    let df = observed.len() as f64 - 1.0;
    Ok(answer(
        x2,
        chisq_p(word, x2, df)?,
        Some(df),
        "Chi-squared test for given probabilities",
    ))
}

/// A MATRIX of finite numbers, by rows: E101 for anything that is not a
/// MATRIX or holds a non-finite cell, E103 for a cell that is not a number
/// or a ragged row.
pub(crate) fn table(word: &str, operand: &str, v: &BundValue) -> Result<Vec<Vec<f64>>, Error> {
    let rows = match v.as_matrix() {
        Some(rows) if v.dt() == MATRIX => rows,
        _ => return Err(domain(word, operand, v.summary(WIDTH))),
    };
    let width = rows.first().map_or(0, Vec::len);
    let mut out = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        if row.len() != width {
            return Err(Refusal::NotNumbers {
                operand: operand.into(),
                container: "MATRIX",
                cell: format!("row {} has {} cells, row 1 {width}", i + 1, row.len()),
            }
            .error(word));
        }
        let mut r = Vec::with_capacity(width);
        for (j, cell) in row.iter().enumerate() {
            let f = crate::convert::conv_value(cell, FLOAT).ok();
            let x = match f.as_ref().map(BundValue::unboxed) {
                Some(BundValue::Float(x, _)) => *x,
                _ => {
                    return Err(Refusal::NotNumbers {
                        operand: operand.into(),
                        container: "MATRIX",
                        cell: cell.summary(WIDTH),
                    }
                    .error(word));
                }
            };
            if !x.is_finite() {
                return Err(domain(
                    word,
                    operand,
                    format!("{x} at [{},{}]", i + 1, j + 1),
                ));
            }
            r.push(x);
        }
        out.push(r);
    }
    Ok(out)
}

/// A table of counts, every cell finite and not negative.
fn counts(word: &str, v: &BundValue) -> Result<Vec<Vec<f64>>, Error> {
    let t = table(word, "table", v)?;
    for (i, row) in t.iter().enumerate() {
        if let Some(j) = row.iter().position(|x| *x < 0.0) {
            return Err(domain(
                word,
                "table",
                format!("{} at [{},{}]", row[j], i + 1, j + 1),
            ));
        }
    }
    Ok(t)
}

fn stat_test_chisq_table(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.chisq.table";
    let v = operands(vm, W, 1)?;
    let t = counts(W, &v[0])?;
    let (r, c) = (t.len(), t.first().map_or(0, Vec::len));
    if r < 2 || c < 2 {
        return Err(domain(W, "table", format!("a table of {r} by {c}")));
    }
    let rows: Vec<f64> = t.iter().map(|row| row.iter().sum()).collect();
    let cols: Vec<f64> = (0..c).map(|j| t.iter().map(|row| row[j]).sum()).collect();
    if let Some(i) = rows.iter().position(|s| *s == 0.0) {
        return Err(domain(W, "table", format!("row {} sums to 0", i + 1)));
    }
    if let Some(j) = cols.iter().position(|s| *s == 0.0) {
        return Err(domain(W, "table", format!("column {} sums to 0", j + 1)));
    }
    let n: f64 = rows.iter().sum();
    let mut x2 = 0.0;
    for (i, row) in t.iter().enumerate() {
        for (j, o) in row.iter().enumerate() {
            let e = rows[i] * cols[j] / n;
            x2 += (o - e) * (o - e) / e;
        }
    }
    let x2 = statistic(W, x2)?;
    let df = ((r - 1) * (c - 1)) as f64;
    vm.push(answer(
        x2,
        chisq_p(W, x2, df)?,
        Some(df),
        "Pearson's Chi-squared test",
    ));
    Ok(())
}

// The exact tests.

/// A count: a cell or operand that is a whole number.
fn whole_count(x: f64) -> Option<u64> {
    (x >= 0.0 && x.fract() == 0.0 && x < 9.007_199_254_740_992e15).then_some(x as u64)
}

fn stat_test_fisher(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.fisher";
    let v = operands(vm, W, 1)?;
    let t = counts(W, &v[0])?;
    if t.len() != 2 {
        return Err(Refusal::Disagree {
            what: "rows".into(),
            n: t.len(),
            m: 2,
        }
        .error(W));
    }
    if t[0].len() != 2 {
        return Err(Refusal::Disagree {
            what: "columns".into(),
            n: t[0].len(),
            m: 2,
        }
        .error(W));
    }
    let mut cells = [0u64; 4];
    for (k, x) in [t[0][0], t[0][1], t[1][0], t[1][1]].into_iter().enumerate() {
        cells[k] = whole_count(x)
            .ok_or_else(|| domain(W, "table", format!("{x} at [{},{}]", k / 2 + 1, k % 2 + 1)))?;
    }
    let [a, b, c, d] = cells;
    // R's names: m the first column's total, n the second's, k the first
    // row's, x the first cell; X is hypergeometric on m + n holding m, k drawn.
    let (m, n, k) = (a + c, b + d, a + b);
    let family = Family::Hypergeometric {
        pop: m + n,
        succ: m,
        draws: k,
    };
    let lo = k.saturating_sub(n);
    let hi = k.min(m);
    if hi - lo >= SUPPORT_CAP {
        return Err(Refusal::Cap {
            what: format!("a support of {} tables", hi - lo + 1),
            cap: SUPPORT_CAP,
        }
        .error(W));
    }
    let logs: Vec<f64> = (lo..=hi).map(|x| family.lpdf(x as f64)).collect();
    let top = logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let bar = family.lpdf(a as f64) + REL_ERR.ln();
    let (mut total, mut tail) = (0.0, 0.0);
    for l in &logs {
        let w = (l - top).exp();
        total += w;
        if *l <= bar {
            tail += w;
        }
    }
    vm.push(answer(
        a as f64,
        tail / total,
        None,
        "Fisher's Exact Test for Count Data",
    ));
    Ok(())
}

fn stat_test_binom(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.binom";
    let v = operands(vm, W, 3)?;
    let count = |operand: &str, x: &BundValue| match x.unboxed() {
        BundValue::Int(i, _) if *i >= 0 => Ok(i.unsigned_abs()),
        _ => Err(domain(W, operand, x.summary(WIDTH))),
    };
    let k = count("k", &v[0])?;
    let n = count("n", &v[1])?;
    if k > n {
        return Err(domain(W, "k", k.to_string()));
    }
    let p = real(W, "p", &v[2])?;
    if !(0.0..=1.0).contains(&p) {
        return Err(domain(W, "p", v[2].summary(WIDTH)));
    }
    let f = Family::Binomial { size: n, prob: p };
    let pv = binom_p(W, &f, k, n, p)?;
    vm.push(answer(k as f64, pv, None, "Exact binomial test"));
    Ok(())
}

/// R's two-sided rule for `binom.test`, counting the outcomes on the far
/// side of the mean as probable as k or less by a binary search, since the
/// mass is monotone there: O(log n) where R's `sum` is O(n).
fn binom_p(word: &str, f: &Family, k: u64, n: u64, p: f64) -> Result<f64, Error> {
    let m = n as f64 * p;
    let x = k as f64;
    if x == m {
        return Ok(1.0);
    }
    let bar = f.lpdf(x) + REL_ERR.ln();
    let cdf = |x: f64| -> Result<f64, Error> {
        if x < 0.0 {
            Ok(0.0)
        } else {
            Ok(f.tails(word, x)?.0)
        }
    };
    let sf = |x: f64| -> Result<f64, Error> {
        if x < 0.0 {
            Ok(1.0)
        } else {
            Ok(f.tails(word, x)?.1)
        }
    };
    let pv = if x < m {
        // On [⌈m⌉, n] the mass falls: the first i at or under the bar.
        let (mut lo, mut hi) = (m.ceil() as u64, n + 1);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if f.lpdf(mid as f64) <= bar {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        let y = n + 1 - lo;
        cdf(x)? + sf(n as f64 - y as f64)?
    } else {
        // On [0, ⌊m⌋] the mass rises: the count at or under the bar.
        let (mut lo, mut hi) = (0u64, m.floor() as u64 + 1);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if f.lpdf(mid as f64) <= bar {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let y = lo;
        cdf(y as f64 - 1.0)? + sf(x - 1.0)?
    };
    Ok(pv.min(1.0))
}

// The rank tests.

fn mannwhitney(word: &str, a: &[f64], b: &[f64]) -> Result<BundValue, Error> {
    at_least(word, a, 1)?;
    at_least(word, b, 1)?;
    let (n1, n2) = (a.len() as f64, b.len() as f64);
    let all: Vec<f64> = a.iter().chain(b).copied().collect();
    let r = ranks(&all);
    let w = r[..a.len()].iter().sum::<f64>() - n1 * (n1 + 1.0) / 2.0;
    // Σ (t³ − t) over the groups of ties.
    let mut sorted = all.clone();
    sorted.sort_by(f64::total_cmp);
    let mut ties = 0.0;
    let mut i = 0;
    while i < sorted.len() {
        let mut j = i;
        while j + 1 < sorted.len() && sorted[j + 1] == sorted[i] {
            j += 1;
        }
        let t = (j - i + 1) as f64;
        ties += t * t * t - t;
        i = j + 1;
    }
    let total = n1 + n2;
    let sigma = (n1 * n2 / 12.0 * ((total + 1.0) - ties / (total * (total - 1.0)))).sqrt();
    if sigma.is_nan() || sigma <= 0.0 {
        return Err(constant(word, "a and b"));
    }
    let z = w - n1 * n2 / 2.0;
    let z = (z - 0.5 * z.signum()) / sigma;
    let normal = Family::Normal { mean: 0.0, sd: 1.0 };
    let (lower, upper) = normal.tails(word, z)?;
    Ok(answer(
        w,
        2.0 * lower.min(upper),
        None,
        "Wilcoxon rank sum test with continuity correction",
    ))
}

/// The Kolmogorov distribution's upper tail, P(K > x), to f64's precision:
/// the alternating series above x = 1, the Jacobi theta form below it, where
/// the first converges slowly.
fn kolmogorov_upper(x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    if x < 1.0 {
        let z = -(std::f64::consts::PI * std::f64::consts::PI) / (8.0 * x * x);
        let mut s = 0.0;
        let mut k = 1.0;
        for _ in 0..1_000 {
            let term = (k * k * z).exp();
            s += term;
            if term <= s * f64::EPSILON / 4.0 {
                break;
            }
            k += 2.0;
        }
        1.0 - (2.0 * std::f64::consts::PI).sqrt() / x * s
    } else {
        let z = -2.0 * x * x;
        let mut s = 0.0;
        let mut sign = 1.0;
        let mut k = 1.0;
        for _ in 0..1_000 {
            let term = (z * k * k).exp();
            s += sign * term;
            if term <= s.abs() * f64::EPSILON / 4.0 {
                break;
            }
            sign = -sign;
            k += 1.0;
        }
        (2.0 * s).clamp(0.0, 1.0)
    }
}

fn ks2(word: &str, a: &[f64], b: &[f64]) -> Result<BundValue, Error> {
    at_least(word, a, 1)?;
    at_least(word, b, 1)?;
    let mut sa = a.to_vec();
    let mut sb = b.to_vec();
    sa.sort_by(f64::total_cmp);
    sb.sort_by(f64::total_cmp);
    let (n1, n2) = (sa.len() as f64, sb.len() as f64);
    let (mut i, mut j, mut d) = (0, 0, 0.0f64);
    while i < sa.len() && j < sb.len() {
        let x = sa[i].min(sb[j]);
        while i < sa.len() && sa[i] <= x {
            i += 1;
        }
        while j < sb.len() && sb[j] <= x {
            j += 1;
        }
        d = d.max((i as f64 / n1 - j as f64 / n2).abs());
    }
    let p = kolmogorov_upper((n1 * n2 / (n1 + n2)).sqrt() * d);
    Ok(answer(
        d,
        p,
        None,
        "Asymptotic two-sample Kolmogorov-Smirnov test",
    ))
}

// Corrections.

fn probabilities(word: &str, ps: &[f64]) -> Result<(), Error> {
    match ps.iter().position(|p| !(0.0..=1.0).contains(p)) {
        None => Ok(()),
        Some(i) => Err(domain(word, "series", format!("{} at {}", ps[i], i + 1))),
    }
}

/// The indices of `ps` in ascending order of p, ties in input order.
fn order(ps: &[f64]) -> Vec<usize> {
    let mut o: Vec<usize> = (0..ps.len()).collect();
    o.sort_by(|&i, &j| ps[i].total_cmp(&ps[j]));
    o
}

fn bonferroni(word: &str, ps: &[f64]) -> Result<BundValue, Error> {
    probabilities(word, ps)?;
    let n = ps.len() as f64;
    Ok(list(ps.iter().map(|p| (p * n).min(1.0)).collect()))
}

fn holm(word: &str, ps: &[f64]) -> Result<BundValue, Error> {
    probabilities(word, ps)?;
    let n = ps.len();
    let mut out = vec![0.0; n];
    let mut running = 0.0f64;
    for (rank, &i) in order(ps).iter().enumerate() {
        running = running.max(ps[i] * (n - rank) as f64);
        out[i] = running.min(1.0);
    }
    Ok(list(out))
}

fn bh(word: &str, ps: &[f64]) -> Result<BundValue, Error> {
    probabilities(word, ps)?;
    let n = ps.len();
    let mut out = vec![0.0; n];
    let mut running = 1.0f64;
    for (rank, &i) in order(ps).iter().enumerate().rev() {
        running = running.min(ps[i] * n as f64 / (rank + 1) as f64);
        out[i] = running.min(1.0);
    }
    Ok(list(out))
}

// The model.

fn stat_lm(vm: &mut dyn Vm) -> Result<(), Error> {
    use nalgebra::{DMatrix, DVector};
    const W: &str = "stat.lm";
    let v = operands(vm, W, 2)?;
    let x = table(W, "X", &v[0])?;
    let y = whole(W, "y", &v[1])?;
    if x.len() != y.len() {
        return Err(Refusal::Disagree {
            what: "lengths".into(),
            n: x.len(),
            m: y.len(),
        }
        .error(W));
    }
    let n = y.len();
    let k = x.first().map_or(0, Vec::len) + 1;
    if n <= k {
        return Err(domain(
            W,
            "y",
            format!("{n} observations for {k} coefficients"),
        ));
    }
    let a = DMatrix::from_fn(n, k, |i, j| if j == 0 { 1.0 } else { x[i][j - 1] });
    let yv = DVector::from_vec(y.clone());
    let qr = a.clone().qr();
    let r = qr.r();
    let scale = (0..k).map(|i| r[(i, i)].abs()).fold(0.0, f64::max);
    if (0..k).any(|i| r[(i, i)].abs() <= scale * 1e-10) {
        return Err(Refusal::Singular.error(W));
    }
    let qty = qr.q().transpose() * &yv;
    let Some(coef) = r.solve_upper_triangular(&qty) else {
        return Err(Refusal::Singular.error(W));
    };
    let Some(rinv) = r.solve_upper_triangular(&DMatrix::identity(k, k)) else {
        return Err(Refusal::Singular.error(W));
    };
    let resid = &yv - &a * &coef;
    let rss = resid.dot(&resid);
    let ybar = mean(&y);
    let tss: f64 = y.iter().map(|v| (v - ybar) * (v - ybar)).sum();
    if tss == 0.0 {
        return Err(constant(W, "y"));
    }
    if rss == 0.0 {
        return Err(domain(
            W,
            "y",
            "a perfect fit, its residual variance 0".into(),
        ));
    }
    let df = (n - k) as f64;
    let sigma2 = rss / df;
    let mut se = Vec::with_capacity(k);
    let mut t = Vec::with_capacity(k);
    let mut p = Vec::with_capacity(k);
    for i in 0..k {
        let s = (sigma2 * (0..k).map(|j| rinv[(i, j)] * rinv[(i, j)]).sum::<f64>()).sqrt();
        let ti = statistic(W, coef[i] / s)?;
        se.push(s);
        t.push(ti);
        p.push(t_p(W, ti, df)?);
    }
    let coef: Vec<f64> = coef.iter().copied().collect();
    finite_all(W, "the coefficients", &coef)?;
    let mut m = BTreeMap::new();
    m.insert("coef".to_string(), list(coef));
    m.insert("se".to_string(), list(se));
    m.insert("t".to_string(), list(t));
    m.insert("p".to_string(), list(p));
    m.insert("r2".to_string(), BundValue::float(1.0 - rss / tss));
    m.insert("df".to_string(), BundValue::float(df));
    vm.push(BundValue::map(m));
    Ok(())
}

// Two-series wrappers.

/// A test of two series.
type OfTwo = fn(&str, &[f64], &[f64]) -> Result<BundValue, Error>;

fn on_two(vm: &mut dyn Vm, word: &str, f: OfTwo) -> Result<(), Error> {
    let (a, b) = two(vm, word)?;
    let y = f(word, &a, &b)?;
    vm.push(y);
    Ok(())
}

fn stat_test_t2(vm: &mut dyn Vm) -> Result<(), Error> {
    on_two(vm, "stat.test.t2", t2)
}

fn stat_test_welch(vm: &mut dyn Vm) -> Result<(), Error> {
    on_two(vm, "stat.test.welch", welch)
}

fn stat_test_paired(vm: &mut dyn Vm) -> Result<(), Error> {
    on_two(vm, "stat.test.paired", paired)
}

fn stat_test_chisq(vm: &mut dyn Vm) -> Result<(), Error> {
    on_two(vm, "stat.test.chisq", chisq)
}

fn stat_test_mannwhitney(vm: &mut dyn Vm) -> Result<(), Error> {
    on_two(vm, "stat.test.mannwhitney", mannwhitney)
}

fn stat_test_ks2(vm: &mut dyn Vm) -> Result<(), Error> {
    on_two(vm, "stat.test.ks2", ks2)
}

/// Register the words, with the forms §S3 gives each.
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native(
                $name,
                $f,
                StackEffect::fixed($n, 1),
                bund2_api::WordKind::Sync,
            );
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    scalar_forms!(r, "stat.test.t1", "mu", stat_test_t1);
    two_forms!(r, "stat.test.t2", stat_test_t2);
    two_forms!(r, "stat.test.welch", stat_test_welch);
    two_forms!(r, "stat.test.paired", stat_test_paired);
    two_forms!(r, "stat.test.chisq", stat_test_chisq);
    word!("stat.test.chisq.table", stat_test_chisq_table, 1);
    word!("stat.test.fisher", stat_test_fisher, 1);
    word!("stat.test.binom", stat_test_binom, 3);
    two_forms!(r, "stat.test.mannwhitney", stat_test_mannwhitney);
    two_forms!(r, "stat.test.ks2", stat_test_ks2);
    four_forms!(r, "stat.padjust.bonferroni", bonferroni);
    four_forms!(r, "stat.padjust.holm", holm);
    four_forms!(r, "stat.padjust.bh", bh);
    word!("stat.lm", stat_lm, 2);
}

/// The words with their forms: four for a correction, two for the rest.
pub const WORDS: [&str; 34] = [
    "stat.test.t1",
    "stat.test.t1.",
    "stat.test.t2",
    "stat.test.t2.",
    "stat.test.welch",
    "stat.test.welch.",
    "stat.test.paired",
    "stat.test.paired.",
    "stat.test.chisq",
    "stat.test.chisq.",
    "stat.test.chisq.table",
    "stat.test.chisq.table.",
    "stat.test.fisher",
    "stat.test.fisher.",
    "stat.test.binom",
    "stat.test.binom.",
    "stat.test.mannwhitney",
    "stat.test.mannwhitney.",
    "stat.test.ks2",
    "stat.test.ks2.",
    "stat.padjust.bonferroni",
    "stat.padjust.bonferroni,",
    "stat.padjust.bonferroni.",
    "stat.padjust.bonferroni.,",
    "stat.padjust.holm",
    "stat.padjust.holm,",
    "stat.padjust.holm.",
    "stat.padjust.holm.,",
    "stat.padjust.bh",
    "stat.padjust.bh,",
    "stat.padjust.bh.",
    "stat.padjust.bh.,",
    "stat.lm",
    "stat.lm.",
];

#[cfg(test)]
mod tests;
