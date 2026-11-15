//! RFC-0015 §S6's descriptive statistics: dispersion, location, shape,
//! quantiles, covariance and correlation.
//!
//! **Computed here, in f64, not through `rstats`** (§S6), so every answer
//! comes from one place. `stat.var`'s mean is a sum over a count, as
//! `stat.mean.arithmetic`'s is, but the two may differ in the last bit.
//!
//! - `stat.var`, `stat.sd` and `stat.se` divide by n − 1, and refuse fewer
//!   than two values, E101.
//! - `stat.mode` is the most frequent value, the smallest among ties.
//! - `stat.skewness` is g₁ = m₃/m₂^{3/2} and `stat.kurtosis` the excess
//!   g₂ = m₄/m₂² − 3, with m_k the population central moments. A constant
//!   series, m₂ = 0, is E101, as is one with no spread for `stat.zscore` and
//!   the correlations.
//! - `stat.quantile` is Hyndman–Fan type 7, R's default: h = (n − 1)p, and
//!   x₍⌊h⌋₎ + (h − ⌊h⌋)(x₍⌊h⌋+1₎ − x₍⌊h⌋₎) over the sorted series.
//! - `stat.cor.spearman` is Pearson's over ranks, ties given their average
//!   rank; `stat.cor.kendall` is τ-b, (n_c − n_d)/√((n₀ − n₁)(n₀ − n₂)), by
//!   comparing every pair, so its work is n(n − 1)/2 comparisons.

use bund2_api::{Error, Registry, Vm};
use bund2_value::BundValue;

use super::operand::{
    at_least, domain, finite, four_forms, list, same_length, scalar_forms, two, two_forms,
};

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// The sample variance, n − 1, two passes. The caller has checked n ≥ 2.
fn variance(xs: &[f64]) -> f64 {
    let m = mean(xs);
    xs.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (xs.len() - 1) as f64
}

fn checked_variance(word: &str, xs: &[f64]) -> Result<f64, Error> {
    at_least(word, xs, 2)?;
    finite(word, variance(xs), || "the variance".into())
}

fn var(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    Ok(BundValue::float(checked_variance(word, xs)?))
}

fn sd(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    Ok(BundValue::float(checked_variance(word, xs)?.sqrt()))
}

fn se(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    let v = checked_variance(word, xs)?;
    Ok(BundValue::float((v / xs.len() as f64).sqrt()))
}

fn sorted(xs: &[f64]) -> Vec<f64> {
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    s
}

fn median(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    at_least(word, xs, 1)?;
    let s = sorted(xs);
    let n = s.len();
    let y = if n % 2 == 1 {
        s[n / 2]
    } else {
        s[n / 2 - 1] / 2.0 + s[n / 2] / 2.0
    };
    Ok(BundValue::float(y))
}

fn mode(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    at_least(word, xs, 1)?;
    let s = sorted(xs);
    let (mut best, mut best_n) = (s[0], 0usize);
    let mut i = 0;
    while i < s.len() {
        let j = s[i..].iter().take_while(|&&v| v == s[i]).count();
        // Ascending, and only a strictly larger count replaces: the smallest
        // of tied values wins.
        if j > best_n {
            (best, best_n) = (s[i], j);
        }
        i += j.max(1);
    }
    Ok(BundValue::float(best))
}

/// The population central moments m₂ and m_k.
fn moments(word: &str, xs: &[f64], k: i32) -> Result<(f64, f64), Error> {
    at_least(word, xs, 1)?;
    let m = mean(xs);
    let n = xs.len() as f64;
    let m2 = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n;
    let mk = xs.iter().map(|x| (x - m).powi(k)).sum::<f64>() / n;
    if m2 == 0.0 {
        return Err(domain(word, "series", "a constant series".into()));
    }
    Ok((m2, mk))
}

fn skewness(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    let (m2, m3) = moments(word, xs, 3)?;
    let y = finite(word, m3 / m2.powf(1.5), || "m₃/m₂^{3/2}".into())?;
    Ok(BundValue::float(y))
}

fn kurtosis(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    let (m2, m4) = moments(word, xs, 4)?;
    let y = finite(word, m4 / (m2 * m2) - 3.0, || "m₄/m₂² − 3".into())?;
    Ok(BundValue::float(y))
}

fn zscore(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    let s = checked_variance(word, xs)?.sqrt();
    if s == 0.0 {
        return Err(domain(word, "series", "a constant series".into()));
    }
    let m = mean(xs);
    Ok(list(xs.iter().map(|x| (x - m) / s).collect()))
}

fn quantile(word: &str, xs: &[f64], p: f64) -> Result<BundValue, Error> {
    if !(0.0..=1.0).contains(&p) {
        return Err(domain(word, "p", format!("{p}")));
    }
    at_least(word, xs, 1)?;
    let s = sorted(xs);
    let h = (s.len() - 1) as f64 * p;
    let lo = h.floor() as usize;
    let hi = (lo + 1).min(s.len() - 1);
    let y = s[lo] + (h - lo as f64) * (s[hi] - s[lo]);
    Ok(BundValue::float(finite(word, y, || "the quantile".into())?))
}

/// The sample covariance, n − 1, of two series of one length, n ≥ 2.
fn covariance(word: &str, a: &[f64], b: &[f64]) -> Result<f64, Error> {
    same_length(word, a, b)?;
    at_least(word, a, 2)?;
    let (ma, mb) = (mean(a), mean(b));
    let c = a
        .iter()
        .zip(b)
        .map(|(x, y)| (x - ma) * (y - mb))
        .sum::<f64>()
        / (a.len() - 1) as f64;
    finite(word, c, || "the covariance".into())
}

fn pearson(word: &str, a: &[f64], b: &[f64]) -> Result<f64, Error> {
    let c = covariance(word, a, b)?;
    let (va, vb) = (variance(a), variance(b));
    if va == 0.0 {
        return Err(domain(word, "a", "a constant series".into()));
    }
    if vb == 0.0 {
        return Err(domain(word, "b", "a constant series".into()));
    }
    let r = finite(word, c / (va.sqrt() * vb.sqrt()), || {
        "the correlation".into()
    })?;
    // Rounding can carry |r| past 1 by an ulp.
    Ok(r.clamp(-1.0, 1.0))
}

/// Ranks from 1, ties given their average.
fn ranks(xs: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..xs.len()).collect();
    order.sort_by(|&i, &j| xs[i].total_cmp(&xs[j]));
    let mut r = vec![0.0; xs.len()];
    let mut i = 0;
    while i < order.len() {
        let mut j = i;
        while j + 1 < order.len() && xs[order[j + 1]] == xs[order[i]] {
            j += 1;
        }
        let avg = (i + j + 2) as f64 / 2.0;
        for &k in &order[i..=j] {
            r[k] = avg;
        }
        i = j + 1;
    }
    r
}

fn kendall(word: &str, a: &[f64], b: &[f64]) -> Result<f64, Error> {
    same_length(word, a, b)?;
    at_least(word, a, 2)?;
    let n = a.len();
    let (mut nc, mut nd, mut ta, mut tb) = (0u64, 0u64, 0u64, 0u64);
    for i in 0..n {
        for j in i + 1..n {
            let (da, db) = (a[i] - a[j], b[i] - b[j]);
            if da == 0.0 {
                ta += 1;
            }
            if db == 0.0 {
                tb += 1;
            }
            if da != 0.0 && db != 0.0 {
                if (da > 0.0) == (db > 0.0) {
                    nc += 1;
                } else {
                    nd += 1;
                }
            }
        }
    }
    let n0 = (n as u64) * (n as u64 - 1) / 2;
    if n0 == ta {
        return Err(domain(word, "a", "a constant series".into()));
    }
    if n0 == tb {
        return Err(domain(word, "b", "a constant series".into()));
    }
    let tau = (nc as f64 - nd as f64) / (((n0 - ta) as f64) * ((n0 - tb) as f64)).sqrt();
    Ok(tau.clamp(-1.0, 1.0))
}

/// A statistic of two series, under a word's name.
type OfTwo = fn(&str, &[f64], &[f64]) -> Result<f64, Error>;

fn two_series(vm: &mut dyn Vm, word: &str, f: OfTwo) -> Result<(), Error> {
    let (a, b) = two(vm, word)?;
    let y = f(word, &a, &b)?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn stat_cov(vm: &mut dyn Vm) -> Result<(), Error> {
    two_series(vm, "stat.cov", covariance)
}

fn stat_cor_pearson(vm: &mut dyn Vm) -> Result<(), Error> {
    two_series(vm, "stat.cor.pearson", pearson)
}

fn stat_cor_spearman(vm: &mut dyn Vm) -> Result<(), Error> {
    two_series(vm, "stat.cor.spearman", |w, a, b| {
        same_length(w, a, b)?;
        pearson(w, &ranks(a), &ranks(b))
    })
}

fn stat_cor_kendall(vm: &mut dyn Vm) -> Result<(), Error> {
    two_series(vm, "stat.cor.kendall", kendall)
}

pub fn register(r: &mut Registry) {
    four_forms!(r, "stat.var", var);
    four_forms!(r, "stat.sd", sd);
    four_forms!(r, "stat.se", se);
    four_forms!(r, "stat.median", median);
    four_forms!(r, "stat.mode", mode);
    four_forms!(r, "stat.skewness", skewness);
    four_forms!(r, "stat.kurtosis", kurtosis);
    four_forms!(r, "stat.zscore", zscore);
    scalar_forms!(r, "stat.quantile", "p", quantile);
    two_forms!(r, "stat.cov", stat_cov);
    two_forms!(r, "stat.cor.pearson", stat_cor_pearson);
    two_forms!(r, "stat.cor.spearman", stat_cor_spearman);
    two_forms!(r, "stat.cor.kendall", stat_cor_kendall);
}

/// S6's descriptive words, each with every form §S3 gives it: criterion 2's
/// list for M3.
pub const WORDS: [&str; 42] = [
    "stat.var",
    "stat.var,",
    "stat.var.",
    "stat.var.,",
    "stat.sd",
    "stat.sd,",
    "stat.sd.",
    "stat.sd.,",
    "stat.se",
    "stat.se,",
    "stat.se.",
    "stat.se.,",
    "stat.median",
    "stat.median,",
    "stat.median.",
    "stat.median.,",
    "stat.mode",
    "stat.mode,",
    "stat.mode.",
    "stat.mode.,",
    "stat.skewness",
    "stat.skewness,",
    "stat.skewness.",
    "stat.skewness.,",
    "stat.kurtosis",
    "stat.kurtosis,",
    "stat.kurtosis.",
    "stat.kurtosis.,",
    "stat.zscore",
    "stat.zscore,",
    "stat.zscore.",
    "stat.zscore.,",
    "stat.quantile",
    "stat.quantile.",
    "stat.cov",
    "stat.cov.",
    "stat.cor.pearson",
    "stat.cor.pearson.",
    "stat.cor.spearman",
    "stat.cor.spearman.",
    "stat.cor.kendall",
    "stat.cor.kendall.",
];

#[cfg(test)]
mod tests;
