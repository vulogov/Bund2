// SPDX-License-Identifier: GPL-2.0-or-later
// Ports R 4.6.2's density.default (src/library/stats/R/density.R:21-184),
// BinDist (src/library/stats/src/massdist.c:34-68), bw.nrd0, bw.nrd, bw.SJ
// and bw_pair_cnts (src/library/stats/R/bandwidths.R:23-104), bw_den,
// bw_den_binned, bw_phi4 and bw_phi6 (src/library/stats/src/bandwidths.c:
// 68-177), uniroot's truncation (src/library/stats/R/nlm.R:55-110) with
// zeroin2's fcn2 (src/library/stats/src/optimize.c:292-327), approx1's
// linear interpolation (src/library/stats/src/approx.c:52-91) and dnorm4
// (src/nmath/dnorm.c:33-91).

//! RFC-0020 §S9 [V8] [V9] [V10]: kernel density and its bandwidths.
//!
//! - **`stat.bw`** (`x rule -- FLOAT`) is a bandwidth by `"nrd0"`
//!   (`bw.nrd0`, with its fallbacks to sd, |x₁| and 1), `"nrd"` (`bw.nrd`,
//!   E101 where it is 0), `"sj"` or `"sj-ste"` (`bw.SJ`'s "ste") and
//!   `"sj-dpi"` (`bw.SJ`'s "dpi"). sd is `stat.sd`'s and the IQR
//!   `stat.quantile`'s type 7. `bw.SJ` counts pairs into 1000 bins, over
//!   every pair for n ≤ 500 and by binned counts above (`bandwidths.R:42-53`,
//!   `:71`); "ste" brackets as R does, widening by 1.2 up to 99 times
//!   (`:92-100`), and solves by [`super::optim`]'s `zeroin` with R's
//!   tolerance, 0.1 × `lower` after any widening (V10), so it answers R's
//!   answer rather than the exact root. Constant data, R's "sample is too
//!   sparse" and "no solution in the specified range" are E101, as is a
//!   series of fewer than two values. Plain and `.`, `scalar_forms!`'s
//!   shape: the `.` form takes the rule from the workbench and answers
//!   there.
//! - **`stat.density`** (`x opts -- MAP`) is `density.default` ported: the
//!   grid of max(512, 2^⌈log₂ n⌉) points from `from` − 4·bw to `to` + 4·bw
//!   (`density.R:99-101`, `:146-147`), linear binning (`massdist.c`), the
//!   kernel's circular convolution by FFT, the clamp at 0 and linear
//!   interpolation onto `n` points (`density.R:151-180`). The FFT is
//!   Bund2's own radix-2 transform in f64, so `y` agrees with R's to
//!   rounding, not to the bit (§S9; Q68). `opts` takes `bw` (a number
//!   above 0 or a rule name, default `"nrd0"`), `adjust` (above 0, default
//!   1), `kernel` (R's seven names, default `"gaussian"`), `n` (an INTEGER
//!   from 1 to 2²⁰, default 512; above is E111), `from` and `to` (finite,
//!   from < to where either is given) and `cut` (≥ 0, default 3). It
//!   answers `x` and `y`, LISTs of `n` FLOATs, `bw` after `adjust`, and
//!   `n`, the sample size. No weights, and a non-finite x is E101 where R
//!   drops it (§S13 (8), (11)). An empty x is E101; a rule on one value is
//!   E101, as R stops (`density.R:121-123`), and a number is not. A
//!   bandwidth so narrow beside x's magnitude that the grid's points do not
//!   increase is E101 naming `bw`. Fixed, plain and `.`.

use std::collections::BTreeMap;
use std::f64::consts::PI;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, STRING};

use super::describe::{type7, variance};
use super::hist::seq_out;
use super::mva::{opts_map, unknown};
use super::operand::{WIDTH, at_least, domain, finite_list, operands, real, series, whole};
use super::optim::zeroin;
use super::refuse::Refusal;
use crate::series::Mode;
use crate::wb::Side;

/// `bw.SJ`'s `nb` (`bandwidths.R:55`).
const NB: usize = 1000;

/// `bandwidths.c`'s `DELMAX` (`:28`).
const DELMAX: f64 = 1000.0;

/// `M_1_SQRT_2PI`.
const M_1_SQRT_2PI: f64 = 0.398_942_280_401_432_677_939_946_059_934;

/// `uniroot`'s default `maxiter` (`nlm.R:60`).
const MAXITER: u64 = 1000;

/// The most output points `stat.density` lays (§S9).
const N_CAP: u64 = 1 << 20;

/// The bandwidth rules, as `stat.bw` and `stat.density`'s `bw` name them.
const RULES: [&str; 5] = ["nrd0", "nrd", "sj", "sj-ste", "sj-dpi"];

const KERNELS: [&str; 7] = [
    "gaussian",
    "epanechnikov",
    "rectangular",
    "triangular",
    "biweight",
    "cosine",
    "optcosine",
];

fn overflow(word: &str, what: &str) -> Error {
    Refusal::Overflow {
        expression: what.into(),
    }
    .error(word)
}

fn sparse(word: &str, rule: &str, what: &str) -> Error {
    domain(
        word,
        "rule",
        format!("{rule:?}, on a sample too sparse to find {what}"),
    )
}

fn sd_iqr(word: &str, xs: &[f64]) -> Result<(f64, f64), Error> {
    let sd = variance(xs).sqrt();
    if !sd.is_finite() {
        return Err(overflow(word, "the variance of x"));
    }
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    let iqr = type7(&s, 0.75) - type7(&s, 0.25);
    if !iqr.is_finite() {
        return Err(overflow(word, "the IQR of x"));
    }
    Ok((sd, iqr))
}

/// A bandwidth by `rule` on a finite series of at least two values.
fn bandwidth(word: &str, xs: &[f64], rule: &str) -> Result<f64, Error> {
    let n = xs.len() as f64;
    let bw = match rule {
        // `bandwidths.R:23-30`.
        "nrd0" => {
            let (hi, iqr) = sd_iqr(word, xs)?;
            let mut lo = hi.min(iqr / 1.34);
            if lo == 0.0 {
                lo = hi;
                if lo == 0.0 {
                    lo = xs[0].abs();
                    if lo == 0.0 {
                        lo = 1.0;
                    }
                }
            }
            0.9 * lo * n.powf(-0.2)
        }
        // `:32-38`.
        "nrd" => {
            let (sd, iqr) = sd_iqr(word, xs)?;
            let h = iqr / 1.34;
            let bw = 1.06 * sd.min(h) * n.powf(-1.0 / 5.0);
            if bw == 0.0 {
                return Err(domain(
                    word,
                    "rule",
                    "\"nrd\", which is 0 on an x whose IQR or sd is 0".into(),
                ));
            }
            bw
        }
        "sj" | "sj-ste" => sj(word, rule, xs, false)?,
        "sj-dpi" => sj(word, rule, xs, true)?,
        _ => return Err(domain(word, "rule", format!("{rule:?}"))),
    };
    if !bw.is_finite() {
        return Err(overflow(word, "the bandwidth"));
    }
    Ok(bw)
}

/// `bw_den` (`bandwidths.c:112-155`): pair counts over every pair, by bin
/// of width 1.01·range/nb.
fn bw_den(word: &str, x: &[f64]) -> Result<(f64, Vec<f64>), Error> {
    let (xmin, xmax) = x
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
            (a.min(*v), b.max(*v))
        });
    let rang = (xmax - xmin) * 1.01;
    if rang == 0.0 {
        return Err(constant(word));
    }
    if !rang.is_finite() {
        return Err(overflow(word, "the range of x"));
    }
    let dd = rang / NB as f64;
    // A range of a few subnormals bins by a width of 0, where R's casts are
    // undefined; nothing distinguishes the values at that resolution.
    if dd == 0.0 {
        return Err(narrow(word, rang));
    }
    let mut cnt = vec![0.0; NB];
    // `(int)` truncates toward zero; in range by the test R makes.
    let far = xmin / dd < f64::from(i32::MIN) || xmax / dd > f64::from(i32::MAX);
    let bin = |v: f64| -> i64 {
        if far {
            ((v - xmin) / dd) as i32 as i64
        } else {
            (v / dd) as i32 as i64
        }
    };
    for i in 1..x.len() {
        let ii = bin(x[i]);
        for xj in &x[..i] {
            let k = (ii - bin(*xj)).unsigned_abs() as usize;
            match cnt.get_mut(k) {
                Some(c) => *c += 1.0,
                None => return Err(Error::internal(bin_defect(k))),
            }
        }
    }
    Ok((dd, cnt))
}

fn bin_defect(k: usize) -> String {
    format!(
        "stat.bw: a pair's bin distance {k} is at least nb = {NB}, where a range of \
         nb/1.01 bins bounds it"
    )
}

fn narrow(word: &str, rang: f64) -> Error {
    domain(
        word,
        "series",
        format!("a range of {rang:e}, too narrow to bin by 1000"),
    )
}

fn constant(word: &str) -> Error {
    domain(word, "series", "constant data".into())
}

/// `bw_pair_cnts`'s binned arm (`bandwidths.R:44-51`) and `bw_den_binned`
/// (`bandwidths.c:158-177`).
fn bw_binned(word: &str, x: &[f64]) -> Result<(f64, Vec<f64>), Error> {
    let (lo, hi) = x
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
            (a.min(*v), b.max(*v))
        });
    let d = (hi - lo) * 1.01 / NB as f64;
    // R's x/0 here makes every count NA and stops as "too sparse"; §S9
    // names it constant data, as the unbinned path does.
    if hi == lo {
        return Err(constant(word));
    }
    if d == 0.0 {
        return Err(narrow(word, (hi - lo) * 1.01));
    }
    if !d.is_finite() {
        return Err(overflow(word, "the range of x"));
    }
    let xx: Vec<f64> = x
        .iter()
        .map(|v| {
            (v.abs() / d).trunc()
                * if *v > 0.0 {
                    1.0
                } else if *v < 0.0 {
                    -1.0
                } else {
                    0.0
                }
        })
        .collect();
    let m = xx.iter().copied().fold(f64::INFINITY, f64::min);
    // `tabulate(xx - min(xx) + 1, nb)`: as.integer, then 1 ..= nb counted.
    let mut tab = vec![0.0; NB];
    for v in &xx {
        let b = (v - m + 1.0).trunc();
        if (1.0..=NB as f64).contains(&b) {
            tab[b as usize - 1] += 1.0;
        }
    }
    let mut cnt = vec![0.0; NB];
    for ii in 0..NB {
        let w = tab[ii];
        cnt[0] += w * (w - 1.0);
        for jj in 0..ii {
            cnt[ii - jj] += w * tab[jj];
        }
    }
    cnt[0] *= 0.5;
    Ok((d, cnt))
}

/// `bw_phi4` (`bandwidths.c:68-84`).
fn phi4(n: f64, d: f64, cnt: &[f64], h: f64) -> f64 {
    let mut sum = 0.0;
    for (i, c) in cnt.iter().enumerate() {
        let mut delta = i as f64 * d / h;
        delta *= delta;
        if delta >= DELMAX {
            break;
        }
        let term = (-delta / 2.0).exp() * (delta * delta - 6.0 * delta + 3.0);
        sum += term * c;
    }
    sum = 2.0 * sum + n * 3.0;
    sum / (n * (n - 1.0) * h.powf(5.0)) * M_1_SQRT_2PI
}

/// `bw_phi6` (`bandwidths.c:86-103`).
fn phi6(n: f64, d: f64, cnt: &[f64], h: f64) -> f64 {
    let mut sum = 0.0;
    for (i, c) in cnt.iter().enumerate() {
        let mut delta = i as f64 * d / h;
        delta *= delta;
        if delta >= DELMAX {
            break;
        }
        let term = (-delta / 2.0).exp()
            * (delta * delta * delta - 15.0 * delta * delta + 45.0 * delta - 15.0);
        sum += term * c;
    }
    sum = 2.0 * sum - 15.0 * n;
    sum / (n * (n - 1.0) * h.powf(7.0)) * M_1_SQRT_2PI
}

/// `bw.SJ` (`bandwidths.R:55-104`).
fn sj(word: &str, rule: &str, x: &[f64], dpi: bool) -> Result<f64, Error> {
    let n = x.len() as f64;
    let (d, cnt) = if x.len() > NB / 2 {
        bw_binned(word, x)?
    } else {
        bw_den(word, x)?
    };
    let (sd, iqr) = sd_iqr(word, x)?;
    let scale = sd.min(iqr / 1.349);
    let a = 1.24 * scale * n.powf(-1.0 / 7.0);
    let b = 1.23 * scale * n.powf(-1.0 / 9.0);
    let c1 = 1.0 / (2.0 * PI.sqrt() * n);
    let td = -phi6(n, d, &cnt, b);
    if !td.is_finite() || td <= 0.0 {
        return Err(sparse(word, rule, "TD"));
    }
    if dpi {
        return Ok((c1 / phi4(n, d, &cnt, (2.394 / (n * td)).powf(1.0 / 7.0))).powf(1.0 / 5.0));
    }
    let hmax = 1.144 * scale * n.powf(-1.0 / 5.0);
    let (mut lower, mut upper) = (0.1 * hmax, hmax);
    let alph2 = 1.357 * (phi4(n, d, &cnt, a) / td).powf(1.0 / 7.0);
    if !alph2.is_finite() {
        return Err(sparse(word, rule, "alph2"));
    }
    let fsd = |h: f64| (c1 / phi4(n, d, &cnt, alph2 * h.powf(5.0 / 7.0))).powf(1.0 / 5.0) - h;
    let mut itry = 1u32;
    // R's `>` is false on NaN, which leaves the loop for `uniroot`'s NA stop.
    while fsd(lower) * fsd(upper) > 0.0 {
        if itry > 99 {
            return Err(domain(
                word,
                "rule",
                format!("{rule:?}, which finds no solution in the range of bandwidths"),
            ));
        }
        if itry % 2 == 1 {
            upper *= 1.2;
        } else {
            lower /= 1.2;
        }
        itry += 1;
    }
    // `tol = 0.1*lower`, a promise forced here, after any widening (V10).
    let tol = 0.1 * lower;
    let (fl, fu) = (fsd(lower), fsd(upper));
    if fl.is_nan() || fu.is_nan() || lower >= upper {
        return Err(sparse(word, rule, "the bandwidth's equation at an end"));
    }
    let truncate = |v: f64| v.clamp(-f64::MAX, f64::MAX);
    // fcn2: −∞ to −DBL_MAX, +∞ and NaN to DBL_MAX (`optimize.c:311-318`).
    let mut f = |h: f64| -> Result<f64, Error> {
        let v = fsd(h);
        Ok(if v.is_finite() {
            v
        } else if v == f64::NEG_INFINITY {
            -f64::MAX
        } else {
            f64::MAX
        })
    };
    // `uniroot` answers its last iterate with a warning when `maxiter`
    // runs out (`nlm.R:105-110`); so does this, without the warning.
    let z = zeroin(
        &mut f,
        lower,
        upper,
        truncate(fl),
        truncate(fu),
        tol,
        MAXITER,
    )?;
    Ok(z.root)
}

fn rule_of(word: &str, operand: &str, v: &BundValue) -> Result<String, Error> {
    match v.as_str() {
        Some(s) if v.dt() == STRING && RULES.contains(&s.as_str()) => Ok(s),
        _ => Err(domain(word, operand, v.summary(WIDTH))),
    }
}

/// `stat.bw` from `side`: the rule from the stack or the workbench, the
/// series beneath it on the stack, the answer to `side`.
fn bw_word(vm: &mut dyn Vm, side: Side, word: &str) -> Result<(), Error> {
    if side.depth(vm) < 1 {
        let which = match side {
            Side::Stack => "Stack",
            Side::Bench => "Workbench",
        };
        return Err(Error(format!("{which} is too shallow for inline {word}")));
    }
    let Some(r) = side.pull(vm) else {
        return Err(Error::internal(format!(
            "{word}: the side was deep enough and gave nothing"
        )));
    };
    let rule = rule_of(word, "rule", &r)?;
    let xs = series(vm, Side::Stack, Mode::Consume, word)?;
    at_least(word, &xs, 2)?;
    let bw = bandwidth(word, &xs, &rule)?;
    side.push(vm, BundValue::float(bw));
    Ok(())
}

fn stat_bw(vm: &mut dyn Vm) -> Result<(), Error> {
    bw_word(vm, Side::Stack, "stat.bw")
}

fn stat_bw_bench(vm: &mut dyn Vm) -> Result<(), Error> {
    bw_word(vm, Side::Bench, "stat.bw.")
}

/// `dnorm4(x, 0, sigma, FALSE)` (`dnorm.c:33-91`), for a finite x and a
/// finite sigma > 0.
fn dnorm(x: f64, sigma: f64) -> f64 {
    let x = (x / sigma).abs();
    if !x.is_finite() || x >= 2.0 * f64::MAX.sqrt() {
        return 0.0;
    }
    if x < 5.0 {
        return M_1_SQRT_2PI * (-0.5 * x * x).exp() / sigma;
    }
    // sqrt(-2·M_LN2·(DBL_MIN_EXP + 1 − DBL_MANT_DIG)), with -1021 and 53.
    let bound = (-2.0 * std::f64::consts::LN_2 * f64::from(-1021 + 1 - 53)).sqrt();
    if x > bound {
        return 0.0;
    }
    // ldexp(R_forceint(ldexp(x, 16)), -16): R_forceint is nearbyint, to
    // even under the default rounding mode.
    let x1 = (x * 65536.0).round_ties_even() / 65536.0;
    let x2 = x - x1;
    M_1_SQRT_2PI / sigma * ((-0.5 * x1 * x1).exp() * ((-0.5 * x2 - x1) * x2).exp())
}

/// The kernel at a grid offset `k`, as `density.R:153-175` writes each.
fn kernel_at(kernel: &str, k: f64, bw: f64) -> f64 {
    let ax = k.abs();
    match kernel {
        "gaussian" => dnorm(k, bw),
        "rectangular" => {
            let a = bw * 3f64.sqrt();
            if ax < a { 0.5 / a } else { 0.0 }
        }
        "triangular" => {
            let a = bw * 6f64.sqrt();
            if ax < a { (1.0 - ax / a) / a } else { 0.0 }
        }
        "epanechnikov" => {
            let a = bw * 5f64.sqrt();
            let r = ax / a;
            if ax < a {
                3.0 / 4.0 * (1.0 - r * r) / a
            } else {
                0.0
            }
        }
        "biweight" => {
            let a = bw * 7f64.sqrt();
            let r = ax / a;
            let u = 1.0 - r * r;
            if ax < a {
                15.0 / 16.0 * (u * u) / a
            } else {
                0.0
            }
        }
        "cosine" => {
            let a = bw / (1.0 / 3.0 - 2.0 / (PI * PI)).sqrt();
            if ax < a {
                (1.0 + (PI * k / a).cos()) / (2.0 * a)
            } else {
                0.0
            }
        }
        _ => {
            let a = bw / (1.0 - 8.0 / (PI * PI)).sqrt();
            if ax < a {
                PI / 4.0 * (PI * k / (2.0 * a)).cos() / a
            } else {
                0.0
            }
        }
    }
}

/// An in-place iterative radix-2 FFT of a power-of-two length, forward
/// (e^{−2πijk/N}) or, unnormalised as R's `inverse = TRUE`, backward.
fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let stride = n / len;
        for start in (0..n).step_by(len) {
            for k in 0..half {
                // The twiddle from its own angle, not a recurrence.
                let ang = sign * 2.0 * PI * (k * stride) as f64 / n as f64;
                let (wi, wr) = ang.sin_cos();
                let (a, b) = (start + k, start + k + half);
                let tr = re[b] * wr - im[b] * wi;
                let ti = re[b] * wi + im[b] * wr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
}

/// `approx1`'s linear arm (`approx.c:52-91`) over an increasing grid; v
/// lies within it.
fn approx1(v: f64, x: &[f64], y: &[f64]) -> f64 {
    let (mut i, mut j) = (0, x.len() - 1);
    if v < x[i] || v > x[j] {
        return f64::NAN;
    }
    while i + 1 < j {
        let ij = (i + j) / 2;
        if v < x[ij] {
            j = ij;
        } else {
            i = ij;
        }
    }
    if v == x[j] {
        return y[j];
    }
    if v == x[i] {
        return y[i];
    }
    y[i] + (y[j] - y[i]) * ((v - x[i]) / (x[j] - x[i]))
}

struct Opts {
    bw: Bw,
    adjust: f64,
    kernel: &'static str,
    n: usize,
    from: Option<f64>,
    to: Option<f64>,
    cut: f64,
}

enum Bw {
    Rule(String),
    Given(f64),
}

fn positive(word: &str, key: &str, v: &BundValue) -> Result<f64, Error> {
    let x = real(word, key, v)?;
    if x.is_finite() && x > 0.0 {
        Ok(x)
    } else {
        Err(domain(word, key, v.summary(WIDTH)))
    }
}

fn finite_real(word: &str, key: &str, v: &BundValue) -> Result<f64, Error> {
    let x = real(word, key, v)?;
    if x.is_finite() {
        Ok(x)
    } else {
        Err(domain(word, key, v.summary(WIDTH)))
    }
}

fn density_opts(word: &str, v: &BundValue) -> Result<Opts, Error> {
    let mut o = Opts {
        bw: Bw::Rule("nrd0".into()),
        adjust: 1.0,
        kernel: "gaussian",
        n: 512,
        from: None,
        to: None,
        cut: 3.0,
    };
    for (k, x) in opts_map(word, v)? {
        match k.as_str() {
            "bw" => {
                o.bw = if x.dt() == STRING {
                    Bw::Rule(rule_of(word, k, x)?)
                } else {
                    Bw::Given(positive(word, k, x)?)
                }
            }
            "adjust" => o.adjust = positive(word, k, x)?,
            "kernel" => {
                o.kernel = match x.as_str() {
                    Some(s) if x.dt() == STRING => KERNELS
                        .iter()
                        .find(|kn| **kn == s.as_str())
                        .copied()
                        .ok_or_else(|| domain(word, k, x.summary(WIDTH)))?,
                    _ => return Err(domain(word, k, x.summary(WIDTH))),
                }
            }
            "n" => {
                o.n = match x.unboxed() {
                    BundValue::Int(i, _) if *i >= 1 && i.unsigned_abs() <= N_CAP => {
                        i.unsigned_abs() as usize
                    }
                    BundValue::Int(i, _) if *i > 1 => {
                        return Err(Refusal::Cap {
                            what: format!("an n of {i} output points"),
                            cap: N_CAP,
                        }
                        .error(word));
                    }
                    _ => return Err(domain(word, k, x.summary(WIDTH))),
                }
            }
            "from" => o.from = Some(finite_real(word, k, x)?),
            "to" => o.to = Some(finite_real(word, k, x)?),
            "cut" => {
                let c = finite_real(word, k, x)?;
                if c < 0.0 {
                    return Err(domain(word, k, x.summary(WIDTH)));
                }
                o.cut = c;
            }
            _ => return Err(unknown(word, k)),
        }
    }
    Ok(o)
}

fn density(word: &str, x: &[f64], o: &Opts) -> Result<BundValue, Error> {
    if x.is_empty() {
        return Err(domain(word, "x", "an empty series".into()));
    }
    let nx = x.len();
    let bw = match &o.bw {
        Bw::Given(b) => *b,
        Bw::Rule(r) => {
            if nx < 2 {
                return Err(domain(
                    word,
                    "x",
                    format!("a series of 1 value, where the rule {r:?} needs 2"),
                ));
            }
            bandwidth(word, x, r)?
        }
    };
    let bw = o.adjust * bw;
    if !bw.is_finite() {
        return Err(overflow(word, "adjust × bw"));
    }
    if bw <= 0.0 {
        return Err(domain(word, "bw", format!("{bw:e}, after adjust")));
    }
    let (xmin, xmax) = x
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
            (a.min(*v), b.max(*v))
        });
    let from = o.from.unwrap_or(xmin - o.cut * bw);
    let to = o.to.unwrap_or(xmax + o.cut * bw);
    if !from.is_finite() || !to.is_finite() {
        return Err(overflow(word, "from or to, x's extreme ∓ cut × bw"));
    }
    if (o.from.is_some() || o.to.is_some()) && from >= to {
        return Err(domain(
            word,
            if o.to.is_some() { "to" } else { "from" },
            format!("from {from} and to {to}, where from < to"),
        ));
    }
    let lo = from - 4.0 * bw;
    let up = to + 4.0 * bw;
    if !lo.is_finite() || !up.is_finite() {
        return Err(overflow(word, "the grid's ends, from − 4·bw and to + 4·bw"));
    }
    // `n <- max(n, 512); if (n > 512) n <- 2^ceiling(log2(n))`.
    let n = o.n.max(512).next_power_of_two();
    let xords = seq_out(lo, up, n);
    if xords.windows(2).any(|w| w[0] >= w[1]) {
        return Err(domain(
            word,
            "bw",
            format!("{bw:e}, too narrow for a grid of {n} points at x's magnitude"),
        ));
    }
    // BinDist (`massdist.c:34-68`), weights 1/nx and totMass 1.
    let mut y = vec![0.0; 2 * n];
    let xdelta = (up - lo) / (n - 1) as f64;
    let wi = 1.0 / nx as f64;
    let ixmax = n as i64 - 2;
    for xv in x {
        let xpos = (xv - lo) / xdelta;
        if xpos > f64::from(i32::MAX) || xpos < f64::from(i32::MIN) {
            continue;
        }
        let ix = xpos.floor() as i64;
        let fx = xpos - ix as f64;
        if (0..=ixmax).contains(&ix) {
            let i = ix as usize;
            y[i] += (1.0 - fx) * wi;
            y[i + 1] += fx * wi;
        } else if ix == -1 {
            y[0] += fx * wi;
        } else if ix == ixmax + 1 {
            y[ix as usize] += (1.0 - fx) * wi;
        }
    }
    // kords (`density.R:151-152`), then the kernel at each.
    let span = (2 * n - 1) as f64 / (n - 1) as f64 * (up - lo);
    let mut kords = seq_out(0.0, span, 2 * n);
    for j in n + 1..2 * n {
        kords[j] = -kords[2 * n - j];
    }
    let mut kr: Vec<f64> = kords.iter().map(|k| kernel_at(o.kernel, *k, bw)).collect();
    if kr.iter().any(|v| !v.is_finite()) {
        return Err(overflow(word, "the kernel on the grid"));
    }
    // fft(fft(y) * Conj(fft(kords)), inverse = TRUE) (`:176`).
    let mut yi = vec![0.0; 2 * n];
    let mut ki = vec![0.0; 2 * n];
    fft(&mut y, &mut yi, false);
    fft(&mut kr, &mut ki, false);
    let (mut cr, mut ci): (Vec<f64>, Vec<f64>) = (0..2 * n)
        .map(|i| {
            let (a, b) = (y[i], yi[i]);
            let (c, d) = (kr[i], -ki[i]);
            (a * c - b * d, a * d + b * c)
        })
        .unzip();
    fft(&mut cr, &mut ci, true);
    let len = (2 * n) as f64;
    let dens: Vec<f64> = cr[..n].iter().map(|v| (v / len).max(0.0)).collect();
    let xout = seq_out(from, to, o.n);
    let yout: Vec<f64> = xout.iter().map(|v| approx1(*v, &xords, &dens)).collect();
    let mut m = BTreeMap::new();
    m.insert("x".to_string(), finite_list(word, "an output point", xout)?);
    m.insert("y".to_string(), finite_list(word, "a density", yout)?);
    m.insert("bw".to_string(), BundValue::float(bw));
    m.insert(
        "n".to_string(),
        BundValue::int(i64::try_from(nx).unwrap_or(i64::MAX)),
    );
    Ok(BundValue::map(m))
}

fn stat_density(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.density";
    let v = operands(vm, W, 2)?;
    let x = whole(W, "x", &v[0])?;
    let o = density_opts(W, &v[1])?;
    let d = density(W, &x, &o)?;
    vm.push(d);
    Ok(())
}

/// Register `stat.bw` and `stat.density`, each plain and `.`.
pub fn register(r: &mut Registry) {
    r.register_native("stat.bw", stat_bw, StackEffect::opaque(2), WordKind::Sync);
    r.register_native(
        "stat.bw.",
        stat_bw_bench,
        StackEffect::opaque(1),
        WordKind::Sync,
    );
    r.register_native(
        "stat.density",
        stat_density,
        StackEffect::fixed(2, 1),
        WordKind::Sync,
    );
    crate::wb::bench!(r, "stat.density", stat_density, 2, 0);
}

/// `stat.bw` and `stat.density`, each with its `.` form.
pub const WORDS: [&str; 4] = ["stat.bw", "stat.bw.", "stat.density", "stat.density."];

#[cfg(test)]
mod tests;
