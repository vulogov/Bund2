//! RFC-0016 §S3: correlation tests, `stat.cor.test.pearson`, `.spearman`
//! and `.kendall`, each `a b -- MAP`, and their `.opts` variants, `a b opts
//! -- MAP`, taking `alternative` ("greater" is positive association) and,
//! for Pearson, `conf`.
//!
//! Each answers `estimate`, `statistic`, `p`, `method` and `alternative`;
//! Pearson's also `df` and, for n ≥ 4, Fisher's z interval with its `conf`.
//! The estimates are RFC-0015's own words' (`describe`, `pearson`,
//! `ranks`, `kendall_counts`), so `estimate` is what `stat.cor.pearson`,
//! `stat.cor.spearman` and `stat.cor.kendall` answer.
//!
//! **At r = ±1.0 in f64** Pearson's t is infinite: `statistic` is left
//! out, p is 0 (1 for the opposite one-sided alternative), and the
//! interval is [r, r]. Spearman's S is finite there and kept; its p
//! follows Pearson's rule.
//!
//! **Kendall's exact distribution** (no ties, n < 50) is carried in f64 as
//! probabilities, P(n, t) = (1/n)·Σ_{j<n} P(n − 1, t − j), every term
//! non-negative, so no count outgrows a float; each tail is summed on its
//! own side, P(T ≥ t) read by symmetry as P(T ≤ N − t).

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, Vm};
use bund2_value::BundValue;

use super::opts::{Alternative, Extra, Opts, end, read, with_extra};
use super::{Family, answer_map, t_p};
use crate::biomath::describe::{kendall_counts, pearson, ranks};
use crate::biomath::operand::{at_least, operands, same_length, two, whole};

/// The defaults a plain test answers for.
const DEFAULTS: Opts = Opts {
    alternative: Alternative::TwoSided,
    conf: 0.95,
    mu: 0.0,
};

/// Fewer than three pairs is E101; so is a constant series, which the
/// estimate refuses.
fn pairs(word: &str, a: &[f64], b: &[f64]) -> Result<f64, Error> {
    same_length(word, a, b)?;
    at_least(word, a, 3)?;
    Ok(a.len() as f64)
}

/// The p of a correlation's t on `df`, for the alternative; at |r| = 1,
/// where t is infinite, 0, or 1 for the opposite one-sided alternative.
fn t_tail(word: &str, r: f64, df: f64, o: &Opts) -> Result<(Option<f64>, f64), Error> {
    if r.abs() == 1.0 {
        let p = match o.alternative {
            Alternative::TwoSided => 0.0,
            Alternative::Greater => f64::from(u8::from(r < 0.0)),
            Alternative::Less => f64::from(u8::from(r > 0.0)),
        };
        return Ok((None, p));
    }
    let t = r * (df / (1.0 - r * r)).sqrt();
    let family = Family::T { df };
    let p = match o.alternative {
        Alternative::TwoSided => t_p(word, t, df)?,
        Alternative::Greater => family.tails(word, t)?.1,
        Alternative::Less => family.tails(word, t)?.0,
    };
    Ok((Some(t), p))
}

/// The answer: the statistic where there is one, and the rest.
fn answer(
    statistic: Option<f64>,
    p: f64,
    df: Option<f64>,
    method: &str,
    extra: Extra,
    o: &Opts,
) -> BundValue {
    let mut m: BTreeMap<String, BundValue> = answer_map(statistic.unwrap_or(0.0), p, df, method);
    if statistic.is_none() {
        m.remove("statistic");
    }
    with_extra(&mut m, extra, o);
    BundValue::map(m)
}

fn pearson_test(word: &str, a: &[f64], b: &[f64], o: &Opts) -> Result<BundValue, Error> {
    let n = pairs(word, a, b)?;
    let r = pearson(word, a, b)?;
    let df = n - 2.0;
    let (t, p) = t_tail(word, r, df, o)?;
    // Fisher's z interval, for n ≥ 4: tanh(atanh r ± z/√(n − 3)), whose
    // ends are r at |r| = 1 and −1 or 1 at a one-sided far end.
    let (ends, conf) = if n >= 4.0 {
        let alpha = 1.0 - o.conf;
        let normal = Family::Normal { mean: 0.0, sd: 1.0 };
        let (z, se) = (r.atanh(), 1.0 / (n - 3.0).sqrt());
        let ends = match o.alternative {
            Alternative::TwoSided => {
                let q = normal.quantile(word, 1.0 - alpha / 2.0)?;
                ((z - q * se).tanh(), (z + q * se).tanh())
            }
            Alternative::Greater => {
                let q = normal.quantile(word, 1.0 - alpha)?;
                ((z - q * se).tanh(), 1.0)
            }
            Alternative::Less => {
                let q = normal.quantile(word, 1.0 - alpha)?;
                (-1.0, (z + q * se).tanh())
            }
        };
        (
            (
                Some(end(word, "lower", ends.0)?),
                Some(end(word, "upper", ends.1)?),
            ),
            true,
        )
    } else {
        ((None, None), false)
    };
    let extra = Extra {
        estimate: Some(r),
        ends,
        conf,
    };
    Ok(answer(
        t,
        p,
        Some(df),
        "Pearson's product-moment correlation",
        extra,
        o,
    ))
}

fn spearman_test(word: &str, a: &[f64], b: &[f64], o: &Opts) -> Result<BundValue, Error> {
    let n = pairs(word, a, b)?;
    let rho = pearson(word, &ranks(a), &ranks(b))?;
    let s = (n * n * n - n) * (1.0 - rho) / 6.0;
    let (_, p) = t_tail(word, rho, n - 2.0, o)?;
    let extra = Extra {
        estimate: Some(rho),
        ends: (None, None),
        conf: false,
    };
    Ok(answer(
        Some(s),
        p,
        None,
        "Spearman's rank correlation rho",
        extra,
        o,
    ))
}

/// Kendall's T without ties: P(T ≤ u) for u = 0 … N, N = n(n − 1)/2, from
/// the probabilities of the non-inversions of a uniform permutation.
fn kendall_lower_tails(n: usize) -> Vec<f64> {
    let mut p = vec![1.0f64];
    for k in 2..=n {
        let top = k * (k - 1) / 2;
        let mut next = vec![0.0f64; top + 1];
        for (t, slot) in next.iter_mut().enumerate() {
            let lo = t.saturating_sub(k - 1);
            let hi = t.min(p.len() - 1);
            if lo <= hi {
                *slot = p[lo..=hi].iter().sum::<f64>() / k as f64;
            }
        }
        p = next;
    }
    let mut below = 0.0;
    p.iter()
        .map(|x| {
            below += x;
            below
        })
        .collect()
}

/// The most pairs Kendall's exact branch takes.
const KENDALL_EXACT_BELOW: usize = 50;

/// Σ t(t − 1)(2t + 5), Σ t(t − 1) and Σ t(t − 1)(t − 2) over the tie
/// groups of `xs`.
fn tie_sums(xs: &[f64]) -> (f64, f64, f64) {
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    let (mut v, mut one, mut two) = (0.0, 0.0, 0.0);
    let mut i = 0;
    while i < s.len() {
        let mut j = i;
        while j + 1 < s.len() && s[j + 1] == s[i] {
            j += 1;
        }
        let t = (j - i + 1) as f64;
        v += t * (t - 1.0) * (2.0 * t + 5.0);
        one += t * (t - 1.0);
        two += t * (t - 1.0) * (t - 2.0);
        i = j + 1;
    }
    (v, one, two)
}

fn kendall_test(word: &str, a: &[f64], b: &[f64], o: &Opts) -> Result<BundValue, Error> {
    let n = pairs(word, a, b)?;
    let k = kendall_counts(word, a, b)?;
    let len = a.len();
    let (statistic, p) = if !k.ties && len < KENDALL_EXACT_BELOW {
        let lower = kendall_lower_tails(len);
        let top = len * (len - 1) / 2;
        let t = k.concordant as usize;
        let at = |u: usize| lower.get(u).copied().unwrap_or(1.0);
        let le = at(t);
        let ge = at(top - t);
        let p = match o.alternative {
            Alternative::Greater => ge,
            Alternative::Less => le,
            Alternative::TwoSided => {
                if 4 * t > len * (len - 1) {
                    (2.0 * ge).min(1.0)
                } else {
                    (2.0 * le).min(1.0)
                }
            }
        };
        (k.concordant as f64, p)
    } else {
        let s = k.concordant as f64 - k.discordant as f64;
        let (vt, t1, t2) = tie_sums(a);
        let (vu, u1, u2) = tie_sums(b);
        let v0 = n * (n - 1.0) * (2.0 * n + 5.0);
        let v = (v0 - vt - vu) / 18.0
            + t1 * u1 / (2.0 * n * (n - 1.0))
            + t2 * u2 / (9.0 * n * (n - 1.0) * (n - 2.0));
        let z = s / v.sqrt();
        let normal = Family::Normal { mean: 0.0, sd: 1.0 };
        let (lo, hi) = normal.tails(word, z)?;
        let p = match o.alternative {
            Alternative::TwoSided => 2.0 * lo.min(hi),
            Alternative::Greater => hi,
            Alternative::Less => lo,
        };
        (z, p)
    };
    let extra = Extra {
        estimate: Some(k.tau),
        ends: (None, None),
        conf: false,
    };
    Ok(answer(
        Some(super::statistic(word, statistic)?),
        p,
        None,
        "Kendall's rank correlation tau",
        extra,
        o,
    ))
}

type Test = fn(&str, &[f64], &[f64], &Opts) -> Result<BundValue, Error>;

fn plain(vm: &mut dyn Vm, word: &str, f: Test) -> Result<(), Error> {
    let (a, b) = two(vm, word)?;
    let y = f(word, &a, &b, &DEFAULTS)?;
    vm.push(y);
    Ok(())
}

fn with_opts(vm: &mut dyn Vm, word: &str, takes: &[&str], f: Test) -> Result<(), Error> {
    let v = operands(vm, word, 3)?;
    let o = read(word, &v[2], takes)?;
    let a = whole(word, "a", &v[0])?;
    let b = whole(word, "b", &v[1])?;
    let y = f(word, &a, &b, &o)?;
    vm.push(y);
    Ok(())
}

fn stat_cor_test_pearson(vm: &mut dyn Vm) -> Result<(), Error> {
    plain(vm, "stat.cor.test.pearson", pearson_test)
}

fn stat_cor_test_spearman(vm: &mut dyn Vm) -> Result<(), Error> {
    plain(vm, "stat.cor.test.spearman", spearman_test)
}

fn stat_cor_test_kendall(vm: &mut dyn Vm) -> Result<(), Error> {
    plain(vm, "stat.cor.test.kendall", kendall_test)
}

fn stat_cor_test_pearson_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    with_opts(
        vm,
        "stat.cor.test.pearson.opts",
        &["alternative", "conf"],
        pearson_test,
    )
}

fn stat_cor_test_spearman_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    with_opts(
        vm,
        "stat.cor.test.spearman.opts",
        &["alternative"],
        spearman_test,
    )
}

fn stat_cor_test_kendall_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    with_opts(
        vm,
        "stat.cor.test.kendall.opts",
        &["alternative"],
        kendall_test,
    )
}

/// Register the six words, each plain and `.` (S3, "Effects").
pub(super) fn register(r: &mut Registry) {
    use crate::biomath::operand::two_forms;
    use bund2_api::{StackEffect, WordKind};
    two_forms!(r, "stat.cor.test.pearson", stat_cor_test_pearson);
    two_forms!(r, "stat.cor.test.spearman", stat_cor_test_spearman);
    two_forms!(r, "stat.cor.test.kendall", stat_cor_test_kendall);
    macro_rules! word {
        ($name:literal, $f:expr) => {
            r.register_native($name, $f, StackEffect::fixed(3, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, 3, 0);
        };
    }
    word!("stat.cor.test.pearson.opts", stat_cor_test_pearson_opts);
    word!("stat.cor.test.spearman.opts", stat_cor_test_spearman_opts);
    word!("stat.cor.test.kendall.opts", stat_cor_test_kendall_opts);
}

/// The correlation tests' names with their forms. RFC-0015's counting test
/// for `describe` leaves them out by this list (§Preservation analysis).
pub const WORDS: [&str; 12] = [
    "stat.cor.test.pearson",
    "stat.cor.test.pearson.",
    "stat.cor.test.spearman",
    "stat.cor.test.spearman.",
    "stat.cor.test.kendall",
    "stat.cor.test.kendall.",
    "stat.cor.test.pearson.opts",
    "stat.cor.test.pearson.opts.",
    "stat.cor.test.spearman.opts",
    "stat.cor.test.spearman.opts.",
    "stat.cor.test.kendall.opts",
    "stat.cor.test.kendall.opts.",
];

#[cfg(test)]
mod tests;
