// SPDX-License-Identifier: GPL-2.0-or-later
// Ports R 4.6.2's hist.default (src/library/graphics/R/hist.R:21-138),
// R_pretty (src/appl/pretty.c:76-270) with pretty.default
// (src/library/base/R/pretty.R:31-53) and seq.int's length.out arithmetic
// (src/main/seq.c:962-1008), nclass.Sturges, nclass.scott and nclass.FD
// (src/library/grDevices/R/calc.R:94-114), C_bincount
// (src/library/graphics/src/stem.c:175-206), quantile.default's type 7
// (src/library/stats/R/quantile.R:57-67) and fprec
// (src/nmath/fprec.c:58-112).

//! RFC-0020 §S10 [V11]: `stat.hist` (`x opts -- MAP`), R's `hist.default`
//! without the plot.
//!
//! x is a whole series (LIST or METRICS), finite, not empty (E101). `opts`
//! takes:
//!
//! - `breaks`: `"sturges"` (the default), `"scott"`, `"fd"`, an INTEGER class
//!   count ≥ 1, or a LIST of at least two strictly increasing breaks
//!   (E101 otherwise). R sorts a LIST and accepts duplicates (`hist.R:48`),
//!   where a duplicate gives a zero-width class of NaN density; a count
//!   above 10⁶ and a LIST of more than 10⁶ breaks are E111, where R clamps
//!   the count with a warning (`:77-81`) (§S10 [author]). A rule or a count
//!   becomes breaks by `pretty(range(x), n, min.n = 1)` (`:82`), ported
//!   from `pretty.c` with R's defaults; its power of ten is `powf`'s, as
//!   R's `pow` (`pretty.c:157-159`), and the four a·b + c that clang fuses
//!   in R's build are fused here (D280). Scott's and FD's rules on one value
//!   are E101, where R fails on an NA variance.
//! - `right` and `include.lowest`, BOOLs, default true. `include.lowest`
//!   false with a rule or a count is E101 naming it, where R ignores it
//!   with a warning (`:50-53`) (§S10 [author]).
//!
//! The breaks are fuzzed by 10⁻⁷ of the median width (more than five
//! breaks), the range of x (at most three) or the least positive width
//! (`:117-125`), and x is counted by `C_bincount`'s rules. A value outside
//! a LIST of breaks is E101, R's "some 'x' not counted" (`:131-132`). It
//! answers `breaks`, `counts` (INTEGERs), `density` = counts/(n·width),
//! `mids` and `equidist` (`:133-138`). Fixed, plain and `.`.

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm};
use bund2_value::{BundValue, INTEGER, LIST, STRING};

use super::describe::variance;
use super::mva::{flag, opts_map, unknown};
use super::operand::{WIDTH, domain, finite_list, list, operands, real, whole};
use super::refuse::Refusal;

/// R's own limit on a class count (`hist.R:77`), and §S11's on a LIST of
/// breaks.
const CLASS_CAP: u64 = 1_000_000;

/// A loop of `R_pretty` that moves `ns` or `nu` by one unit at a time ends
/// after a step or two: each starts within one unit of its bound, and
/// overflow is undone a unit at a time from a value within a few units of
/// ±`DBL_MAX`. This many steps is a defect in the port.
const PRETTY_STEPS: u32 = 1_000;

// fprec (`fprec.c:58-112`), signif's arithmetic.

/// `R_pow_di` (`src/main/arithmetic.c:256-275`): x to an integer power by
/// repeated squaring, as R computes the powers `fprec` divides by.
fn pow_di(mut x: f64, mut n: i32) -> f64 {
    let mut xn = 1.0;
    if n != 0 {
        if !x.is_finite() {
            return x.powf(f64::from(n));
        }
        let neg = n < 0;
        if neg {
            n = -n;
        }
        loop {
            if n & 1 != 0 {
                xn *= x;
            }
            n >>= 1;
            if n != 0 {
                x *= x;
            } else {
                break;
            }
        }
        if neg {
            xn = 1.0 / xn;
        }
    }
    xn
}

/// `signif(x, dig)`, `fprec`'s arithmetic for a finite x and 1 ≤ dig ≤ 22.
fn signif(x: f64, dig: i32) -> f64 {
    const MAX_DIGITS: i32 = 22;
    const MAX10E: i32 = f64::MAX_10_EXP;
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let (sgn, x) = if x < 0.0 { (-1.0, -x) } else { (1.0, x) };
    let l10 = x.log10();
    let mut e10 = dig - 1 - l10.floor() as i32;
    if l10.abs() < f64::from(MAX10E - 2) {
        let mut p10 = 1.0;
        if e10 > MAX10E {
            p10 = pow_di(10.0, e10 - MAX10E);
            e10 = MAX10E;
        }
        if e10 > 0 {
            let pow10 = pow_di(10.0, e10);
            sgn * (((x * pow10) * p10).round_ties_even() / pow10) / p10
        } else {
            let pow10 = pow_di(10.0, -e10);
            sgn * ((x / pow10).round_ties_even() * pow10)
        }
    } else {
        let do_round = f64::MAX.log10() - l10 >= pow_di(10.0, -dig);
        let e2 = dig + if e10 > 0 { 1 } else { -1 } * MAX_DIGITS;
        let p10 = pow_di(10.0, e2);
        let big = pow_di(10.0, e10 - e2);
        let mut y = x * p10;
        y *= big;
        if do_round {
            y += 0.5;
        }
        y = y.floor() / p10;
        sgn * y / big
    }
}

// quantile.default, type 7 (`quantile.R:57-67`), over a sorted series.

fn quantile7(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    let index = 1.0 + (n.max(1) - 1) as f64 * p;
    let lo = index.floor();
    let hi = index.ceil();
    let qs = sorted[lo as usize - 1];
    let xhi = sorted[hi as usize - 1];
    if index > lo && xhi != qs {
        let h = index - lo;
        (1.0 - h) * qs + h * xhi
    } else {
        qs
    }
}

fn range(xs: &[f64]) -> (f64, f64) {
    xs.iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
            (lo.min(*x), hi.max(*x))
        })
}

/// R's `var` of at least two values; one is E101, where R's NA variance
/// fails the rule's test (`calc.R:99`, `:113`).
fn sd_of(word: &str, rule: &str, xs: &[f64]) -> Result<f64, Error> {
    if xs.len() < 2 {
        return Err(domain(
            word,
            "breaks",
            format!("{rule:?}, on a series of 1 value, which has no variance"),
        ));
    }
    let s = variance(xs).sqrt();
    if !s.is_finite() {
        return Err(overflow(word, "the variance of x"));
    }
    Ok(s)
}

fn overflow(word: &str, what: &str) -> Error {
    Refusal::Overflow {
        expression: what.into(),
    }
    .error(word)
}

/// `nclass.Sturges`, `nclass.scott` and `nclass.FD` (`calc.R:94-114`), as
/// the double R hands to `pretty`.
fn nclass(word: &str, rule: &str, xs: &[f64]) -> Result<f64, Error> {
    let n = xs.len() as f64;
    let (lo, hi) = range(xs);
    let span = hi - lo;
    match rule {
        "sturges" => Ok((n.log2() + 1.0).ceil()),
        "scott" => {
            let h = 3.5 * sd_of(word, rule, xs)? * n.powf(-1.0 / 3.0);
            Ok(if h > 0.0 {
                (span / h).ceil().max(1.0)
            } else {
                1.0
            })
        }
        _ => {
            let mut s: Vec<f64> = xs.iter().map(|x| signif(*x, 5)).collect();
            s.sort_by(f64::total_cmp);
            let mut h = 2.0 * (quantile7(&s, 0.75) - quantile7(&s, 0.25));
            if h == 0.0 {
                let mut al: f64 = 1.0 / 4.0;
                let al_min = 1.0 / 512.0;
                loop {
                    al /= 2.0;
                    if h != 0.0 || al < al_min {
                        break;
                    }
                    h = (quantile7(&s, 1.0 - al) - quantile7(&s, al)) / (1.0 - 2.0 * al);
                }
            }
            if h == 0.0 {
                h = 3.5 * sd_of(word, rule, xs)?;
            }
            Ok(if h > 0.0 {
                (span / h * n.powf(1.0 / 3.0)).ceil()
            } else {
                1.0
            })
        }
    }
}

// R_pretty (`pretty.c:76-270`) and pretty.default (`pretty.R:31-53`).

/// `R_pretty` with `pretty.default`'s defaults (shrink.sml 0.75, high.u.bias
/// 1.5, u5.bias 2.75, f.min 2⁻²⁰, eps.correct 0, bounds true): the bounds
/// `l` and `u` and the number of intervals.
fn r_pretty(lo_: f64, up_: f64, ndiv: i32, min_n: i32) -> Result<(f64, f64, i32), Error> {
    const ROUNDING_EPS: f64 = 1e-10;
    const H: f64 = 1.5;
    const H5: f64 = 0.5 + 1.5 * H;
    const SHRINK: f64 = 0.75;
    const MAX_F: f64 = 1.25;
    let f_min = 2f64.powi(-20);
    let dx = up_ - lo_;
    let mut cell;
    let i_small;
    if dx == 0.0 && up_ == 0.0 {
        cell = 1.0;
        i_small = true;
    } else {
        cell = lo_.abs().max(up_.abs());
        let mut u = 1.0
            + if H5 >= 1.5 * H + 0.5 {
                1.0 / (1.0 + H)
            } else {
                1.5 / (1.0 + H5)
            };
        u *= f64::from(ndiv.max(1)) * f64::EPSILON;
        i_small = dx < cell * u * 3.0;
    }
    if i_small {
        if cell > 10.0 {
            cell = 9.0 + cell / 10.0;
        }
        cell *= SHRINK;
        if min_n > 1 {
            cell /= f64::from(min_n);
        }
    } else {
        cell = dx;
        if dx.is_finite() {
            if ndiv > 1 {
                cell /= f64::from(ndiv);
            }
        } else if ndiv >= 2 {
            cell = up_ / f64::from(ndiv) - lo_ / f64::from(ndiv);
        }
    }
    let mut subsmall = f_min * f64::MIN_POSITIVE;
    if subsmall == 0.0 {
        subsmall = f64::MIN_POSITIVE;
    }
    if cell < subsmall {
        cell = subsmall;
    } else if cell > f64::MAX / MAX_F {
        cell = f64::MAX / MAX_F;
    }
    let base = 10f64.powf(cell.log10().floor());
    let mut unit = base;
    let mut u = 2.0 * base;
    if u - cell < H * (cell - unit) {
        unit = u;
        u = 5.0 * base;
        if u - cell < H5 * (cell - unit) {
            unit = u;
            u = 10.0 * base;
            if u - cell < H * (cell - unit) {
                unit = u;
            }
        }
    }
    let mut ns = (lo_ / unit + ROUNDING_EPS).floor();
    let mut nu = (up_ / unit - ROUNDING_EPS).ceil();
    let stuck = || {
        Error::internal(format!(
            "R_pretty's loops move ns and nu a unit at a time and end within a few \
             steps; {PRETTY_STEPS} were taken for the range {lo_} to {up_}"
        ))
    };
    let mut steps = 0;
    let mut step = || {
        steps += 1;
        if steps > PRETTY_STEPS {
            Err(stuck())
        } else {
            Ok(())
        }
    };
    // D280: the four a·b + c below are fused, as clang compiles R's
    // (`pretty.c:203`, `:216`; `seq.c:997`, `:1004`).
    while ns * unit > ROUNDING_EPS.mul_add(unit, lo_) {
        step()?;
        ns -= 1.0;
    }
    while !(ns * unit).is_finite() {
        step()?;
        ns += 1.0;
    }
    while nu * unit < (-ROUNDING_EPS).mul_add(unit, up_) {
        step()?;
        nu += 1.0;
    }
    while !(nu * unit).is_finite() {
        step()?;
        nu -= 1.0;
    }
    let mut k = (0.5 + nu - ns) as i32;
    let n_out;
    if k < min_n {
        k = min_n - k;
        if lo_ == 0.0 && ns == 0.0 && up_ != 0.0 {
            nu += f64::from(k);
        } else if up_ == 0.0 && nu == 0.0 && lo_ != 0.0 {
            ns -= f64::from(k);
        } else if ns >= 0.0 {
            nu += f64::from(k / 2);
            ns -= f64::from(k / 2 + k % 2);
        } else {
            ns -= f64::from(k / 2);
            nu += f64::from(k / 2 + k % 2);
        }
        n_out = min_n;
    } else {
        n_out = k;
    }
    let mut l = lo_;
    let mut up = up_;
    if ns * unit < l {
        l = ns * unit;
    }
    if nu * unit > up {
        up = nu * unit;
    }
    Ok((l, up, n_out))
}

/// `pretty(c(lo, up), n, min.n)`: `R_pretty`'s bounds, then `seq.int(l, u,
/// length.out = n + 1)` (`seq.c:962-1008`) and the zapping of values below
/// 10⁻¹⁴ of a step (`pretty.R:46-51`).
pub(crate) fn pretty(lo: f64, up: f64, n: i32, min_n: i32) -> Result<Vec<f64>, Error> {
    let (l, u, n) = r_pretty(lo, up, n, min_n)?;
    let mut s = seq_out(l, u, usize::try_from(n).unwrap_or(0) + 1);
    if n > 0 {
        let (a, b) = (l.min(u), l.max(u));
        let delta = b / f64::from(n) - a / f64::from(n);
        for si in s.iter_mut() {
            if si.abs() < 1e-14 * delta {
                *si = 0.0;
            }
        }
    }
    Ok(s)
}

/// `seq.int(l, u, length.out = lout)`, its real branch (`seq.c:989-1007`):
/// the ends as given, the middle l + i·by, fused as R's build fuses it
/// (D280), and through a quarter of each when u − l overflows. `stat.density`
/// lays its grids with it too.
pub(crate) fn seq_out(l: f64, u: f64, lout: usize) -> Vec<f64> {
    let mut s = vec![l; lout];
    if lout > 1 {
        s[lout - 1] = u;
    }
    if lout > 2 {
        let nint = (lout - 1) as f64;
        let del = u - l;
        if del.is_finite() {
            let by = del / nint;
            for (i, si) in s.iter_mut().enumerate().take(lout - 1).skip(1) {
                *si = (i as f64).mul_add(by, l); // D280
            }
        } else {
            let by = (u / nint - l / nint) / 4.0;
            let from = l / 4.0;
            for (i, si) in s.iter_mut().enumerate().take(lout - 1).skip(1) {
                *si = (i as f64).mul_add(by, from) * 4.0; // D280
            }
        }
    }
    s
}

// C_bincount (`stem.c:175-206`).

fn bincount(x: &[f64], breaks: &[f64], right: bool, include_border: bool) -> Vec<i64> {
    let nb1 = breaks.len() - 1;
    let mut count = vec![0i64; nb1];
    for xi in x {
        let (mut lo, mut hi) = (0, nb1);
        if breaks[lo] <= *xi && (*xi < breaks[hi] || (*xi == breaks[hi] && include_border)) {
            while hi - lo >= 2 {
                let new = (hi + lo) / 2;
                if *xi > breaks[new] || (!right && *xi == breaks[new]) {
                    lo = new;
                } else {
                    hi = new;
                }
            }
            count[lo] += 1;
        }
    }
    count
}

enum Breaks {
    Rule(String),
    Count(i64),
    Given(Vec<f64>),
}

fn breaks_of(word: &str, v: &BundValue) -> Result<Breaks, Error> {
    let bad = || domain(word, "breaks", v.summary(WIDTH));
    match v.dt() {
        STRING => match v.as_str().as_deref() {
            Some(r @ ("sturges" | "scott" | "fd")) => Ok(Breaks::Rule(r.to_string())),
            _ => Err(bad()),
        },
        INTEGER => match v.unboxed() {
            BundValue::Int(k, _) if *k >= 1 => {
                if u64::try_from(*k).unwrap_or(u64::MAX) > CLASS_CAP {
                    return Err(Refusal::Cap {
                        what: format!("a class count of {k}"),
                        cap: CLASS_CAP,
                    }
                    .error(word));
                }
                Ok(Breaks::Count(*k))
            }
            _ => Err(bad()),
        },
        LIST => {
            let items = v.as_list().unwrap_or(&[]);
            if items.len() < 2 {
                return Err(bad());
            }
            if items.len() as u64 > CLASS_CAP {
                return Err(Refusal::Cap {
                    what: format!("a LIST of {} breaks", items.len()),
                    cap: CLASS_CAP,
                }
                .error(word));
            }
            let mut b = Vec::with_capacity(items.len());
            for x in items {
                b.push(real(word, "breaks", x)?);
            }
            if let Some(i) = b.windows(2).position(|w| w[1] <= w[0]) {
                return Err(domain(
                    word,
                    "breaks",
                    format!("{}, not strictly increasing at {}", v.summary(WIDTH), i + 2),
                ));
            }
            Ok(Breaks::Given(b))
        }
        _ => Err(bad()),
    }
}

fn median(xs: &[f64]) -> f64 {
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    let m = s.len() / 2;
    if s.len() % 2 == 1 {
        s[m]
    } else {
        (s[m - 1] + s[m]) / 2.0
    }
}

fn stat_hist(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.hist";
    let v = operands(vm, W, 2)?;
    let x = whole(W, "x", &v[0])?;
    if x.is_empty() {
        return Err(domain(W, "x", "an empty series".into()));
    }
    let mut spec = Breaks::Rule("sturges".into());
    let mut right = true;
    let mut lowest = true;
    let mut lowest_given = false;
    for (k, o) in opts_map(W, &v[1])? {
        match k.as_str() {
            "breaks" => spec = breaks_of(W, o)?,
            "right" => right = flag(W, k, o)?,
            "include.lowest" => {
                lowest = flag(W, k, o)?;
                lowest_given = true;
            }
            _ => return Err(unknown(W, k)),
        }
    }
    let n = x.len();
    let use_br = matches!(spec, Breaks::Given(_));
    if !use_br && lowest_given && !lowest {
        return Err(domain(
            W,
            "include.lowest",
            "false, where breaks is a rule or a count".into(),
        ));
    }
    let breaks = match spec {
        Breaks::Given(b) => b,
        Breaks::Rule(r) => {
            let k = nclass(W, &r, &x)?;
            // R stops on a count below 1 (`hist.R:75-76`); no rule answers
            // one on a finite x, which this guards.
            if k.is_nan() || k < 1.0 {
                return Err(domain(
                    W,
                    "breaks",
                    format!("{r:?}, which gives {k} classes"),
                ));
            }
            if k > CLASS_CAP as f64 {
                return Err(Refusal::Cap {
                    what: format!("the class count {k} of the rule {r:?}"),
                    cap: CLASS_CAP,
                }
                .error(W));
            }
            let (lo, hi) = range(&x);
            pretty(lo, hi, k as i32, 1)?
        }
        Breaks::Count(k) => {
            let (lo, hi) = range(&x);
            pretty(lo, hi, i32::try_from(k).unwrap_or(i32::MAX), 1)?
        }
    };
    let nb = breaks.len();
    if nb < 2 {
        return Err(Error::internal(
            "pretty answers n + 1 ≥ 2 breaks, since R_pretty's n is at least min.n = 1",
        ));
    }
    let h: Vec<f64> = breaks.windows(2).map(|w| w[1] - w[0]).collect();
    if h.iter().any(|w| !w.is_finite()) {
        return Err(overflow(W, "the width of a class"));
    }
    let (hmin, hmax) = range(&h);
    let hmean = h.iter().sum::<f64>() / h.len() as f64;
    let equidist = !use_br || hmax - hmin < 1e-7 * hmean;
    let diddle = 1e-7
        * if nb > 5 {
            median(&h)
        } else if nb <= 3 {
            let (lo, hi) = range(&x);
            hi - lo
        } else {
            h.iter()
                .copied()
                .filter(|w| *w > 0.0)
                .fold(f64::INFINITY, f64::min)
        };
    let fuzz: Vec<f64> = if right {
        std::iter::once(if lowest { -diddle } else { diddle })
            .chain(std::iter::repeat_n(diddle, nb - 1))
            .collect()
    } else {
        std::iter::repeat_n(-diddle, nb - 1)
            .chain(std::iter::once(if lowest { diddle } else { -diddle }))
            .collect()
    };
    let fuzzy: Vec<f64> = breaks.iter().zip(&fuzz).map(|(b, f)| b + f).collect();
    let counts = bincount(&x, &fuzzy, right, lowest);
    let counted: i64 = counts.iter().sum();
    if counted < n as i64 {
        return Err(domain(
            W,
            "breaks",
            format!(
                "breaks from {} to {}, which leave {} of x's {n} values uncounted",
                breaks[0],
                breaks[nb - 1],
                n as i64 - counted
            ),
        ));
    }
    let density: Vec<f64> = counts
        .iter()
        .zip(&h)
        .map(|(c, w)| *c as f64 / (n as f64 * w))
        .collect();
    let mids: Vec<f64> = breaks.windows(2).map(|w| 0.5 * (w[1] + w[0])).collect();
    let mut m = BTreeMap::new();
    m.insert("breaks".to_string(), list(breaks));
    m.insert(
        "counts".to_string(),
        BundValue::list(counts.into_iter().map(BundValue::int).collect()),
    );
    m.insert("density".to_string(), finite_list(W, "a density", density)?);
    m.insert("mids".to_string(), finite_list(W, "a mid-point", mids)?);
    m.insert("equidist".to_string(), BundValue::boolean(equidist));
    vm.push(BundValue::map(m));
    Ok(())
}

/// Register `stat.hist` and its `.` form (RFC-0015 §S3).
pub fn register(r: &mut Registry) {
    r.register_native(
        "stat.hist",
        stat_hist,
        StackEffect::fixed(2, 1),
        bund2_api::WordKind::Sync,
    );
    crate::wb::bench!(r, "stat.hist", stat_hist, 2, 0);
}

/// `stat.hist` and its `.` form.
pub const WORDS: [&str; 2] = ["stat.hist", "stat.hist."];

#[cfg(test)]
mod tests;
