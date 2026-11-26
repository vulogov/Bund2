// SPDX-License-Identifier: GPL-2.0-or-later
// Contains code ported line for line from R 4.4.2's src/library/stats/src/swilk.c,
// which is GPL-2.0-or-later (D279).

//! RFC-0019 §S5's normality and variance tests, M5: Shapiro–Wilk, the
//! one-sample Kolmogorov–Smirnov test, Bartlett's test and Levene's test
//! with its `.opts` word.
//!
//! **Shapiro–Wilk** ports `swilk.c` (R 4.4.2, `src/library/stats/src`),
//! Royston's AS R94, for 3 ≤ n ≤ 5000, after `shapiro.test.R`'s steps: the
//! sample is sorted, all values identical is E101 naming `x`, and a range
//! below 10⁻¹⁰ divides x by it. Its reference values are R's (N10).
//!
//! **The one-sample KS test** takes a distribution MAP (RFC-0015 §S7) and
//! follows `ks.test.R`'s two-sided one-sample path: exact by Marsaglia, Tsang
//! and Wang's matrix power (`ks.c`'s `K2x`) when n < 100 and no two values
//! tie, otherwise Kolmogorov's limit at √n·D. A family with no cdf is E105.
//!
//! **Bartlett's test** is `bartlett.test.R`'s χ² on k − 1; a group of one
//! value is E101 naming it, as R stops on it, and a group of equal values,
//! whose log variance is −∞, is E101 naming it too.
//!
//! **Levene's test** is the one-way F on |x − centre| within each group,
//! the centre each group's median (Brown–Forsythe) unless `center` is
//! `"mean"`. Base R has none; the method string follows `car`'s heading. It
//! answers `df1` and `df2`, k − 1 and N − k, in place of `df` (D272).

use bund2_api::{Error, Vm};
use bund2_value::{BundValue, MAP, STRING};

use super::super::{answer_map, chisq_p, constant, kolmogorov_upper, statistic};
use super::{groups, normal};
use crate::biomath::dist::{Family, decode, normal_tails};
use crate::biomath::operand::{WIDTH, domain, finite_all, operands, whole};

// Shapiro–Wilk.

/// AS 181.2's polynomial: cc[0] + cc[1]·x + … + cc[nord−1]·x^(nord−1).
fn poly(cc: &[f64], x: f64) -> f64 {
    let nord = cc.len();
    let mut ret = cc[0];
    if nord > 1 {
        let mut p = x * cc[nord - 1];
        for c in cc[1..nord - 1].iter().rev() {
            p = (p + c) * x;
        }
        ret += p;
    }
    ret
}

/// R's `sign` on the integer difference i − j.
fn isign(i: usize, j: usize) -> f64 {
    match i.cmp(&j) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    }
}

/// `swilk`, for a sorted x of 3 to 5000 values whose range is at least
/// 10⁻¹⁰ (the caller rescales): (W, p).
// swilk.c's own truncated literals for √½ (:71) and asin(√¾) = π/3 (:154):
// the exact constants would answer other bits than R's.
#[allow(clippy::approx_constant)]
fn swilk(word: &str, x: &[f64]) -> Result<(f64, f64), Error> {
    let n = x.len();
    let nn2 = n / 2;
    let mut a = vec![0.0; nn2 + 1]; // 1-based
    let small = 1e-19;
    let g = [-2.273, 0.459];
    let c1 = [0.0, 0.221157, -0.147981, -2.07119, 4.434685, -2.706056];
    let c2 = [0.0, 0.042981, -0.293762, -1.752461, 5.682633, -3.582633];
    let c3 = [0.544, -0.39978, 0.025054, -6.714e-4];
    let c4 = [1.3822, -0.77857, 0.062767, -0.0020322];
    let c5 = [-1.5861, -0.31082, -0.083751, 0.0038915];
    let c6 = [-0.4803, -0.082676, 0.0030302];
    let an = n as f64;
    if n == 3 {
        a[1] = 0.70710678;
    } else {
        let an25 = an + 0.25;
        let mut summ2 = 0.0;
        for (i, ai) in a.iter_mut().enumerate().skip(1) {
            *ai = normal().quantile(word, (i as f64 - 0.375) / an25)?;
            summ2 += *ai * *ai;
        }
        summ2 *= 2.0;
        let ssumm2 = summ2.sqrt();
        let rsn = 1.0 / an.sqrt();
        let a1 = poly(&c1, rsn) - a[1] / ssumm2;
        let (i1, fac) = if n > 5 {
            let a2 = -a[2] / ssumm2 + poly(&c2, rsn);
            let fac = ((summ2 - 2.0 * (a[1] * a[1]) - 2.0 * (a[2] * a[2]))
                / (1.0 - 2.0 * (a1 * a1) - 2.0 * (a2 * a2)))
                .sqrt();
            a[2] = a2;
            (3, fac)
        } else {
            let fac = ((summ2 - 2.0 * (a[1] * a[1])) / (1.0 - 2.0 * (a1 * a1))).sqrt();
            (2, fac)
        };
        a[1] = a1;
        for ai in a.iter_mut().skip(i1) {
            *ai /= -fac;
        }
    }
    let range = x[n - 1] - x[0];
    if range < small {
        // ifault 6: shapiro.test.R rescales a range below 1e-10 first.
        return Err(constant(word, "x"));
    }
    // ifault 7, a sort-order check on the scaled x, is ignored, as R's
    // SWilk ignores it.
    let mut sx = x[0] / range;
    let mut sa = -a[1];
    let mut j = n - 1;
    let mut i = 1;
    while i < n {
        let xi = x[i] / range;
        sx += xi;
        i += 1;
        if i != j {
            sa += isign(i, j) * a[i.min(j)];
        }
        j -= 1;
    }
    sa /= an;
    sx /= an;
    let (mut ssa, mut ssx, mut sax) = (0.0, 0.0, 0.0);
    for (i, xi) in x.iter().enumerate() {
        let j = n - 1 - i;
        let asa = if i != j {
            isign(i, j) * a[1 + i.min(j)] - sa
        } else {
            -sa
        };
        let xsx = xi / range - sx;
        ssa += asa * asa;
        ssx += xsx * xsx;
        sax += asa * xsx;
    }
    let ssassx = (ssa * ssx).sqrt();
    let w1 = (ssassx - sax) * (ssassx + sax) / (ssa * ssx);
    let w = 1.0 - w1;
    if n == 3 {
        let pi6 = 1.90985931710274;
        let stqr = 1.04719755119660;
        let pw = pi6 * (w.sqrt().asin() - stqr);
        return Ok((w, pw.max(0.0)));
    }
    let mut y = w1.ln();
    let xx = an.ln();
    let (m, s) = if n <= 11 {
        let gamma = poly(&g, an);
        if y >= gamma {
            return Ok((w, 1e-99));
        }
        y = -(gamma - y).ln();
        (poly(&c3, an), poly(&c4, an).exp())
    } else {
        (poly(&c5, xx), poly(&c6, xx).exp())
    };
    Ok((w, normal_tails((y - m) / s).1))
}

pub(super) fn stat_test_shapiro(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.shapiro";
    let v = operands(vm, W, 1)?;
    let mut x = whole(W, "x", &v[0])?;
    finite_all(W, "x", &x)?;
    let n = x.len();
    if !(3..=5000).contains(&n) {
        return Err(domain(W, "x", format!("a sample of {n}")));
    }
    x.sort_by(f64::total_cmp);
    let rng = x[n - 1] - x[0];
    if rng == 0.0 {
        return Err(constant(W, "x"));
    }
    if rng < 1e-10 {
        for xi in x.iter_mut() {
            *xi /= rng;
        }
    }
    let (w, p) = swilk(W, &x)?;
    let w = statistic(W, w)?;
    vm.push(BundValue::map(answer_map(
        w,
        p,
        None,
        "Shapiro-Wilk normality test",
    )));
    Ok(())
}

// The one-sample Kolmogorov–Smirnov test.

/// The exact path's bound: n below it, and no ties (`ks.test.R`).
const KS_EXACT_BELOW: usize = 100;

/// C = A·B, m × m, row-major (`ks.c`'s `m_multiply`).
fn m_multiply(a: &[f64], b: &[f64], m: usize) -> Vec<f64> {
    let mut c = vec![0.0; m * m];
    for i in 0..m {
        for j in 0..m {
            let mut s = 0.0;
            for k in 0..m {
                s += a[i * m + k] * b[k * m + j];
            }
            c[i * m + j] = s;
        }
    }
    c
}

/// A^n with its decimal exponent, rescaled by 10⁻¹⁴⁰ as it grows
/// (`ks.c`'s `m_power`, A's own exponent 0).
fn m_power(a: &[f64], m: usize, n: usize) -> (Vec<f64>, i32) {
    if n == 1 {
        return (a.to_vec(), 0);
    }
    let (v, ev) = m_power(a, m, n / 2);
    let b = m_multiply(&v, &v, m);
    let eb = 2 * ev;
    let (mut v, mut ev) = if n.is_multiple_of(2) {
        (b, eb)
    } else {
        (m_multiply(a, &b, m), eb)
    };
    if v[(m / 2) * m + m / 2] > 1e140 {
        for x in v.iter_mut() {
            *x *= 1e-140;
        }
        ev += 140;
    }
    (v, ev)
}

/// P(D < d) for the two-sided one-sample statistic of n, exactly:
/// Marsaglia, Tsang and Wang (2003), `ks.c`'s `K2x`.
fn k2x(n: usize, d: f64) -> f64 {
    let nf = n as f64;
    let k = (nf * d) as usize + 1;
    let m = 2 * k - 1;
    let h = k as f64 - nf * d;
    let mut hm = vec![0.0; m * m];
    for i in 0..m {
        for j in 0..m {
            hm[i * m + j] = if i + 1 < j { 0.0 } else { 1.0 };
        }
    }
    for i in 0..m {
        hm[i * m] -= h.powi(i as i32 + 1);
        hm[(m - 1) * m + i] -= h.powi((m - i) as i32);
    }
    hm[(m - 1) * m] += if 2.0 * h - 1.0 > 0.0 {
        (2.0 * h - 1.0).powi(m as i32)
    } else {
        0.0
    };
    for i in 0..m {
        for j in 0..m {
            if i + 1 > j {
                for g in 1..=(i + 1 - j) {
                    hm[i * m + j] /= g as f64;
                }
            }
        }
    }
    let (q, mut eq) = m_power(&hm, m, n);
    let mut s = q[(k - 1) * m + k - 1];
    for i in 1..=n {
        s = s * i as f64 / nf;
        if s < 1e-140 {
            s *= 1e140;
            eq -= 140;
        }
    }
    s * 10f64.powi(eq)
}

pub(super) fn stat_test_ks1(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.ks1";
    let v = operands(vm, W, 2)?;
    let mut x = whole(W, "x", &v[0])?;
    finite_all(W, "x", &x)?;
    if x.is_empty() {
        return Err(domain(W, "x", "an empty series".into()));
    }
    let d = decode(W, &v[1])?;
    x.sort_by(f64::total_cmp);
    let n = x.len();
    let nf = n as f64;
    let mut stat = 0.0f64;
    for (i, xi) in x.iter().enumerate() {
        let f = d.tails(W, *xi)?.0 - i as f64 / nf;
        stat = stat.max(f).max(1.0 / nf - f);
    }
    let ties = x.windows(2).any(|w| w[0] == w[1]);
    let exact = n < KS_EXACT_BELOW && !ties;
    // pkolmogorov's ends, then its two paths.
    let p = if stat <= 0.0 {
        1.0
    } else if stat > 1.0 {
        0.0
    } else if exact {
        1.0 - k2x(n, stat)
    } else {
        kolmogorov_upper(nf.sqrt() * stat)
    };
    let method = if exact {
        "Exact one-sample Kolmogorov-Smirnov test"
    } else {
        "Asymptotic one-sample Kolmogorov-Smirnov test"
    };
    vm.push(BundValue::map(answer_map(
        statistic(W, stat)?,
        p,
        None,
        method,
    )));
    Ok(())
}

// Bartlett.

/// `groups`, each finite, with at least `least` values; each refusal names
/// the group.
pub(super) fn checked_groups(
    word: &str,
    v: &BundValue,
    least: usize,
) -> Result<Vec<Vec<f64>>, Error> {
    let g = groups(word, v)?;
    for (i, grp) in g.iter().enumerate() {
        let name = format!("group {}", i + 1);
        finite_all(word, &name, grp)?;
        if grp.len() < least {
            return Err(domain(word, &name, format!("a sample of {}", grp.len())));
        }
    }
    Ok(g)
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

pub(super) fn stat_test_bartlett(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.bartlett";
    let v = operands(vm, W, 1)?;
    let g = checked_groups(W, &v[0], 2)?;
    let k = g.len() as f64;
    let (mut n_total, mut nv, mut nlogv, mut inv) = (0.0, 0.0, 0.0, 0.0);
    for (i, grp) in g.iter().enumerate() {
        let n = grp.len() as f64 - 1.0;
        let m = mean(grp);
        let var = grp.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n;
        if var <= 0.0 {
            return Err(constant(W, &format!("group {}", i + 1)));
        }
        n_total += n;
        nv += n * var;
        nlogv += n * var.ln();
        inv += 1.0 / n;
    }
    let v_total = nv / n_total;
    let k2 = statistic(
        W,
        (n_total * v_total.ln() - nlogv) / (1.0 + (inv - 1.0 / n_total) / (3.0 * (k - 1.0))),
    )?;
    let df = k - 1.0;
    vm.push(BundValue::map(answer_map(
        k2,
        chisq_p(W, k2, df)?,
        Some(df),
        "Bartlett test of homogeneity of variances",
    )));
    Ok(())
}

// Levene.

#[derive(Clone, Copy)]
enum Center {
    Median,
    Mean,
}

/// A Levene options MAP: only `center`, `"median"` or `"mean"`; E101 for
/// anything but a MAP, an unknown key or a bad value, naming the key.
fn read_center(word: &str, v: &BundValue) -> Result<Center, Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(word, "opts", v.summary(WIDTH))),
    };
    let mut c = Center::Median;
    for (k, x) in m {
        if k != "center" {
            return Err(domain(word, "opts", format!("an unknown key {k:?}")));
        }
        c = match x.as_str() {
            Some(s) if x.dt() == STRING && s == "median" => Center::Median,
            Some(s) if x.dt() == STRING && s == "mean" => Center::Mean,
            _ => return Err(domain(word, k, x.summary(WIDTH))),
        };
    }
    Ok(c)
}

fn median(xs: &[f64]) -> f64 {
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    if !n.is_multiple_of(2) {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

fn levene(word: &str, v: &BundValue, center: Center) -> Result<BundValue, Error> {
    let g = checked_groups(word, v, 1)?;
    let k = g.len();
    let total: usize = g.iter().map(Vec::len).sum();
    if total <= k {
        return Err(domain(
            word,
            "groups",
            format!("{total} values in {k} groups"),
        ));
    }
    let z: Vec<Vec<f64>> = g
        .iter()
        .map(|grp| {
            let c = match center {
                Center::Median => median(grp),
                Center::Mean => mean(grp),
            };
            grp.iter().map(|x| (x - c).abs()).collect()
        })
        .collect();
    let grand = z.iter().flatten().sum::<f64>() / total as f64;
    let (mut between, mut within) = (0.0, 0.0);
    for zi in &z {
        let m = mean(zi);
        between += zi.len() as f64 * (m - grand) * (m - grand);
        within += zi.iter().map(|x| (x - m) * (x - m)).sum::<f64>();
    }
    if within <= 0.0 {
        return Err(domain(
            word,
            "groups",
            "every deviation from its centre equal within its group".into(),
        ));
    }
    let (df1, df2) = ((k - 1) as f64, (total - k) as f64);
    let f = statistic(word, (between / df1) / (within / df2))?;
    let p = Family::F { df1, df2 }.tails(word, f)?.1;
    let method = match center {
        Center::Median => "Levene's test for homogeneity of variance (center = median)",
        Center::Mean => "Levene's test for homogeneity of variance (center = mean)",
    };
    let mut m = answer_map(f, p, None, method);
    m.insert("df1".to_string(), BundValue::float(df1));
    m.insert("df2".to_string(), BundValue::float(df2));
    Ok(BundValue::map(m))
}

pub(super) fn stat_test_levene(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.levene";
    let v = operands(vm, W, 1)?;
    vm.push(levene(W, &v[0], Center::Median)?);
    Ok(())
}

pub(super) fn stat_test_levene_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.levene.opts";
    let v = operands(vm, W, 2)?;
    let c = read_center(W, &v[1])?;
    vm.push(levene(W, &v[0], c)?);
    Ok(())
}
