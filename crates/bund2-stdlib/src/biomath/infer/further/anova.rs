//! RFC-0019 §S12's one-way tests, M6: `stat.test.anova1`, its `.opts` word,
//! and `stat.test.tukey`.
//!
//! **`stat.test.anova1`** is the classical one-way F on k − 1 and N − k,
//! as R's `oneway.test` with `var.equal = TRUE` (`oneway.test.R`, R 4.4.2):
//! each group needs two values, as R stops on fewer. Beside the F it
//! answers η² = SSB/SST and ω² = (SSB − (k − 1)·MSE)/(SST + MSE), with
//! either F (D273). The F test's degrees of freedom are `df1` and `df2`
//! (D272). **`.opts`** takes `var.equal`; false gives Welch's F, R's
//! `var.equal = FALSE` arithmetic line for line, and no key is true, the
//! plain word (D273).
//!
//! **`stat.test.tukey`** is Tukey–Kramer, as `TukeyHSD.R`: the MSE of the
//! one-way fit on N − k, for each pair the difference of the means, the
//! standard error √(MSE/2·(1/nᵢ + 1/nⱼ)), the interval from §S3's quantile
//! at `conf` and p from §S3's upper tail at |diff|/se. It answers a frame
//! (RFC-0017) of `pair`, `diff`, `lower`, `upper` and `p`, one row per pair
//! in R's order, `"2-1"`, `"3-1"`, …, `"3-2"`, …, the groups named by their
//! 1-based positions (D267). N − k < 2 is E101 naming `groups`, before the
//! distribution's own E104.

use std::collections::BTreeMap;

use bund2_api::{Error, Vm};
use bund2_value::{BOOL, BundValue, MAP};

use super::super::opts::read;
use super::super::{answer_map, constant, statistic};
use super::normality::checked_groups;
use crate::biomath::dist::Family;
use crate::biomath::frame::check::Frame;
use crate::biomath::operand::{WIDTH, domain, operands};

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Σ (x − mean)² of a group.
fn ss(xs: &[f64]) -> f64 {
    let m = mean(xs);
    xs.iter().map(|x| (x - m) * (x - m)).sum()
}

/// The one-way fit's parts: each group's size and mean, SSB, SSW and N.
struct Fit {
    n: Vec<f64>,
    m: Vec<f64>,
    ssb: f64,
    ssw: f64,
    total: f64,
}

fn fit(g: &[Vec<f64>]) -> Fit {
    let n: Vec<f64> = g.iter().map(|x| x.len() as f64).collect();
    let m: Vec<f64> = g.iter().map(|x| mean(x)).collect();
    let total: f64 = n.iter().sum();
    let grand = g.iter().flatten().sum::<f64>() / total;
    let ssb = n
        .iter()
        .zip(&m)
        .map(|(ni, mi)| ni * (mi - grand) * (mi - grand))
        .sum();
    let ssw = g.iter().map(|x| ss(x)).sum();
    Fit {
        n,
        m,
        ssb,
        ssw,
        total,
    }
}

fn no_spread(word: &str) -> Error {
    domain(word, "groups", "no spread within any group".into())
}

fn anova1(word: &str, v: &BundValue, var_equal: bool) -> Result<BundValue, Error> {
    let g = checked_groups(word, v, 2)?;
    let f = fit(&g);
    let k = g.len() as f64;
    let df1 = k - 1.0;
    let df_w = f.total - k;
    if f.ssw <= 0.0 {
        return Err(no_spread(word));
    }
    let mse = f.ssw / df_w;
    let sst = f.ssb + f.ssw;
    let eta2 = f.ssb / sst;
    let omega2 = (f.ssb - df1 * mse) / (sst + mse);
    let (stat, df2, method) = if var_equal {
        ((f.ssb / df1) / mse, df_w, "One-way analysis of means")
    } else {
        // oneway.test.R's var.equal = FALSE arm.
        let mut w = Vec::with_capacity(g.len());
        for (i, x) in g.iter().enumerate() {
            let var = ss(x) / (x.len() as f64 - 1.0);
            if var <= 0.0 {
                return Err(constant(word, &format!("group {}", i + 1)));
            }
            w.push(x.len() as f64 / var);
        }
        let sum_w: f64 = w.iter().sum();
        let tmp = w
            .iter()
            .zip(&f.n)
            .map(|(wi, ni)| (1.0 - wi / sum_w) * (1.0 - wi / sum_w) / (ni - 1.0))
            .sum::<f64>()
            / (k * k - 1.0);
        let mw = w.iter().zip(&f.m).map(|(wi, mi)| wi * mi).sum::<f64>() / sum_w;
        let num: f64 = w
            .iter()
            .zip(&f.m)
            .map(|(wi, mi)| wi * (mi - mw) * (mi - mw))
            .sum();
        (
            num / ((k - 1.0) * (1.0 + 2.0 * (k - 2.0) * tmp)),
            1.0 / (3.0 * tmp),
            "One-way analysis of means (not assuming equal variances)",
        )
    };
    let stat = statistic(word, stat)?;
    let p = Family::F { df1, df2 }.tails(word, stat)?.1;
    let mut m = answer_map(stat, p, None, method);
    m.insert("df1".to_string(), BundValue::float(df1));
    m.insert("df2".to_string(), BundValue::float(df2));
    m.insert("eta2".to_string(), BundValue::float(eta2));
    m.insert("omega2".to_string(), BundValue::float(omega2));
    Ok(BundValue::map(m))
}

pub(super) fn stat_test_anova1(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.anova1";
    let v = operands(vm, W, 1)?;
    vm.push(anova1(W, &v[0], true)?);
    Ok(())
}

/// An anova1 options MAP: only `var.equal`, a BOOL; E101 for anything but
/// a MAP, an unknown key or a bad value, naming the key.
fn read_var_equal(word: &str, v: &BundValue) -> Result<bool, Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(word, "opts", v.summary(WIDTH))),
    };
    let mut var_equal = true;
    for (k, x) in m {
        if k != "var.equal" {
            return Err(domain(word, "opts", format!("an unknown key {k:?}")));
        }
        var_equal = match x.unboxed() {
            BundValue::Bool(b, _) if x.dt() == BOOL => *b,
            _ => return Err(domain(word, k, x.summary(WIDTH))),
        };
    }
    Ok(var_equal)
}

pub(super) fn stat_test_anova1_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.anova1.opts";
    let v = operands(vm, W, 2)?;
    let var_equal = read_var_equal(W, &v[1])?;
    vm.push(anova1(W, &v[0], var_equal)?);
    Ok(())
}

// Tukey's HSD.

pub(super) fn stat_test_tukey(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.tukey";
    let v = operands(vm, W, 2)?;
    let conf = read(W, &v[1], &["conf"])?.conf;
    let g = checked_groups(W, &v[0], 1)?;
    let f = fit(&g);
    let k = g.len();
    let df = f.total - k as f64;
    if df < 2.0 {
        return Err(domain(
            W,
            "groups",
            format!("{} values in {k} groups", f.total),
        ));
    }
    if f.ssw <= 0.0 {
        return Err(no_spread(W));
    }
    let mse = f.ssw / df;
    let dist = Family::Tukey {
        nmeans: k as u64,
        df,
    };
    let q = dist.quantile(W, conf)?;
    let (mut pair, mut diff, mut lower, mut upper, mut p) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    // R's lower.tri of outer(means, means, "-"), column by column.
    for j in 0..k {
        for i in j + 1..k {
            let d = f.m[i] - f.m[j];
            let se = (mse / 2.0 * (1.0 / f.n[i] + 1.0 / f.n[j])).sqrt();
            let width = q * se;
            pair.push(BundValue::str(format!("{}-{}", i + 1, j + 1)));
            diff.push(BundValue::float(d));
            lower.push(BundValue::float(d - width));
            upper.push(BundValue::float(d + width));
            p.push(BundValue::float(dist.tails(W, (d / se).abs())?.1));
        }
    }
    let nrow = pair.len();
    let frame = Frame {
        names: ["pair", "diff", "lower", "upper", "p"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        cols: vec![pair, diff, lower, upper, p],
        levels: BTreeMap::new(),
        nrow,
    };
    vm.push(frame.to_value());
    Ok(())
}
