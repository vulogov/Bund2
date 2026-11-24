//! RFC-0019 §S5's further tests. M4 builds the rank and table tests: the
//! signed-rank test, one-sample and paired, with their `.opts` words,
//! Kruskal–Wallis, Friedman, McNemar with its `.opts` word, and
//! Cochran–Mantel–Haenszel. M5 adds the normality and variance tests in
//! `normality`: Shapiro–Wilk, one-sample KS, Bartlett, and Levene with its
//! `.opts` word. M6 adds §S12's one-way tests in `anova`: the one-way F
//! with its `.opts` word, and Tukey's HSD.
//!
//! **Each answers RFC-0015's test MAP** (`statistic`, `p`, `df` where there
//! is one, `method`, R's name for the test). The signed-rank `.opts` words
//! add RFC-0016's `estimate`, `lower`, `upper`, `conf` and `alternative`;
//! CMH, whose strata are 2×2 MATRIXes or LISTs of two rows (D271), and
//! which has no `.opts` word, answers its Mantel–Haenszel common odds
//! ratio and the interval about it at 0.95 beside its test keys. All are
//! fixed, plain and `.`, the `.` form taking the top operand from the
//! workbench and answering there.
//!
//! **The signed-rank test** follows R 4.4.2's `wilcox.test` (§What R does):
//! zeros of x − μ are dropped; the test is exact when fewer than 50 remain
//! and no |x − μ| ties another and none was dropped, unless `exact` says
//! otherwise; otherwise it is normal, with the tie-corrected variance and a
//! continuity correction unless `correct` is false. `exact` true with ties
//! or zeros is E101 (D267), where R falls back with a warning. Its estimate
//! is the Hodges–Lehmann pseudomedian, the median of the Walsh averages of
//! x with its zeros dropped, as R's interval takes them. On the exact path
//! the interval is R's, from the signed-rank quantiles; otherwise its ends
//! are where the corrected z of x − d crosses the normal quantile, found by
//! §S6's `zeroin` at N7's tolerance. Where R would warn that the level is
//! not achievable and widen the interval, Bund2 refuses, E101, as
//! `stat.test.mannwhitney.opts` does.

use bund2_api::{Error, Vm};
use bund2_value::{BOOL, BundValue, FLOAT, LIST, MAP, STRING};

use super::opts::{Alternative, Extra, Opts, end, with_extra};
use super::{answer_map, chisq_p, constant, statistic, table};
use crate::biomath::describe::ranks;
use crate::biomath::dist::Family;
use crate::biomath::operand::{WIDTH, domain, finite, finite_all, operands, real, whole};
use crate::biomath::optim::{MAXITER, zeroin};
use crate::biomath::refuse::Refusal;

/// The largest n the exact signed-rank distribution is computed for when
/// `exact` asks for it (D39): about n³/4 = 2.5·10⁸ steps.
const EXACT_CAP: u64 = 1_000;

/// The most Walsh averages an estimate forms (D39), as
/// `stat.test.mannwhitney.opts` caps its differences.
const WALSH_CAP: u64 = 10_000_000;

/// The normal path's default when `exact` is not given: exact below 50.
const EXACT_BELOW: usize = 50;

fn normal() -> Family {
    Family::Normal { mean: 0.0, sd: 1.0 }
}

/// R's `sign`, 0 at 0.
fn sign(z: f64) -> f64 {
    if z == 0.0 { 0.0 } else { z.signum() }
}

/// Σ (t³ − t) over the groups of equal values.
fn tie_term(xs: &[f64]) -> f64 {
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    let mut total = 0.0;
    let mut i = 0;
    while i < s.len() {
        let mut j = i;
        while j + 1 < s.len() && s[j + 1] == s[i] {
            j += 1;
        }
        let t = (j - i + 1) as f64;
        total += t * t * t - t;
        i = j + 1;
    }
    total
}

// The options.

/// A signed-rank or McNemar options MAP, read.
struct RankOpts {
    alternative: Alternative,
    conf: f64,
    exact: Option<bool>,
    correct: bool,
}

/// The keys any of these `.opts` words knows.
const KEYS: [&str; 4] = ["alternative", "conf", "exact", "correct"];

/// Read an options MAP as RFC-0016 §S1 reads one (RFC-0019 §S1): E101 for
/// anything but a MAP, an unknown key, a key `takes` does not hold, or a
/// value of the wrong type or range, each naming the key.
fn read(word: &str, v: &BundValue, takes: &[&str]) -> Result<RankOpts, Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(word, "opts", v.summary(WIDTH))),
    };
    let mut o = RankOpts {
        alternative: Alternative::TwoSided,
        conf: 0.95,
        exact: None,
        correct: true,
    };
    for (k, x) in m {
        if !KEYS.contains(&k.as_str()) {
            return Err(domain(word, "opts", format!("an unknown key {k:?}")));
        }
        if !takes.contains(&k.as_str()) {
            return Err(domain(
                word,
                "opts",
                format!("the key {k:?}, which this word does not take"),
            ));
        }
        let boolean = || match x.unboxed() {
            BundValue::Bool(b, _) if x.dt() == BOOL => Ok(*b),
            _ => Err(domain(word, k, x.summary(WIDTH))),
        };
        match k.as_str() {
            "alternative" => {
                let s = match x.as_str() {
                    Some(s) if x.dt() == STRING => s,
                    _ => return Err(domain(word, k, x.summary(WIDTH))),
                };
                o.alternative = match s.as_str() {
                    "two.sided" => Alternative::TwoSided,
                    "less" => Alternative::Less,
                    "greater" => Alternative::Greater,
                    _ => return Err(domain(word, k, x.summary(WIDTH))),
                };
            }
            "conf" => {
                o.conf = match x.unboxed() {
                    BundValue::Float(c, _) if x.dt() == FLOAT && *c > 0.0 && *c < 1.0 => *c,
                    _ => return Err(domain(word, k, x.summary(WIDTH))),
                };
            }
            "exact" => o.exact = Some(boolean()?),
            _ => o.correct = boolean()?,
        }
    }
    Ok(o)
}

// The signed-rank test.

/// The signed-rank distribution of n: P(V = v) for v ≤ ⌊u/2⌋, u = n(n+1)/2,
/// the rest by symmetry. Built as R's `csignrank` builds its counts, one
/// factor (1 + q^j)/2 at a time, in probabilities, so nothing overflows.
struct SignRankDist {
    u: usize,
    w: Vec<f64>,
}

impl SignRankDist {
    fn new(n: usize) -> Self {
        let u = n * (n + 1) / 2;
        let c = u / 2;
        let mut w = vec![0.0; c + 1];
        w[0] = 1.0;
        for j in 1..=n {
            let top = (j * (j + 1) / 2).min(c);
            for i in (0..=top).rev() {
                let shifted = if i >= j { w[i - j] } else { 0.0 };
                w[i] = (w[i] + shifted) / 2.0;
            }
        }
        SignRankDist { u, w }
    }

    /// P(V = v).
    fn mass(&self, v: usize) -> f64 {
        if v > self.u {
            0.0
        } else if v < self.w.len() {
            self.w[v]
        } else {
            self.w.get(self.u - v).copied().unwrap_or(0.0)
        }
    }

    /// P(V ≤ v), the shorter side summed.
    fn lower(&self, v: i64) -> f64 {
        if v < 0 {
            return 0.0;
        }
        let v = v as usize;
        if v >= self.u {
            return 1.0;
        }
        if v < self.w.len() {
            self.w[..=v].iter().sum()
        } else {
            // P(V ≤ v) = 1 − P(V ≥ v + 1) = 1 − P(V ≤ u − v − 1).
            1.0 - self.w[..=self.u - v - 1].iter().sum::<f64>()
        }
    }

    /// P(V ≥ v) = P(V ≤ u − v).
    fn upper(&self, v: i64) -> f64 {
        self.lower(self.u as i64 - v)
    }

    /// R's `qsignrank(p, n)`: the least q with P(V ≤ q) ≥ p, with R's fuzz
    /// of 10ε, summed from the side p is nearer.
    fn quantile(&self, p: f64) -> usize {
        let mut acc = 0.0;
        let mut q = 0usize;
        if p <= 0.5 {
            let target = p - 10.0 * f64::EPSILON;
            loop {
                acc += self.mass(q);
                if acc >= target || q >= self.u {
                    return q;
                }
                q += 1;
            }
        } else {
            let target = 1.0 - p + 10.0 * f64::EPSILON;
            loop {
                acc += self.mass(q);
                if acc > target || q >= self.u {
                    return self.u - q;
                }
                q += 1;
            }
        }
    }
}

/// A signed-rank test's data: x with its zeros dropped, the differences
/// x − μ, V, and whether anything tied or was dropped.
struct SignRank {
    kept: Vec<f64>,
    d: Vec<f64>,
    v: f64,
    ties: f64,
    zeroes: bool,
}

/// `operand` names x in refusals, `zero` what an all-zero x is.
fn sign_rank(word: &str, operand: &str, x: &[f64], mu: f64, zero: &str) -> Result<SignRank, Error> {
    if x.is_empty() {
        return Err(domain(word, operand, "an empty series".into()));
    }
    let shifted: Vec<f64> = x.iter().map(|v| v - mu).collect();
    finite_all(word, &format!("{operand} - mu"), &shifted)?;
    let zeroes = shifted.contains(&0.0);
    let (mut kept, mut d) = (Vec::new(), Vec::new());
    for (xi, di) in x.iter().zip(&shifted) {
        if *di != 0.0 {
            kept.push(*xi);
            d.push(*di);
        }
    }
    if d.is_empty() {
        return Err(domain(word, operand, zero.into()));
    }
    let (v, ties) = v_and_ties(&d);
    Ok(SignRank {
        kept,
        d,
        v,
        ties,
        zeroes,
    })
}

/// V, the sum of the ranks of |d| over the positive d, and the tie term of
/// |d|.
fn v_and_ties(d: &[f64]) -> (f64, f64) {
    let abs: Vec<f64> = d.iter().map(|x| x.abs()).collect();
    let r = ranks(&abs);
    let v = r
        .iter()
        .zip(d)
        .filter(|(_, x)| **x > 0.0)
        .map(|(r, _)| r)
        .sum();
    (v, tie_term(&abs))
}

/// The normal path's z, with the continuity correction for `alternative`
/// when `correct`: NaN for an empty d.
fn normal_z(d: &[f64], alternative: Alternative, correct: bool) -> f64 {
    let n = d.len() as f64;
    let (v, ties) = v_and_ties(d);
    let z = v - n * (n + 1.0) / 4.0;
    let sigma = (n * (n + 1.0) * (2.0 * n + 1.0) / 24.0 - ties / 48.0).sqrt();
    let c = if correct {
        match alternative {
            Alternative::TwoSided => sign(z) * 0.5,
            Alternative::Greater => 0.5,
            Alternative::Less => -0.5,
        }
    } else {
        0.0
    };
    (z - c) / sigma
}

/// Which way a signed-rank test went.
enum Path {
    Exact(SignRankDist),
    Normal,
}

/// The test's path, statistic and p, and its method.
fn signed_test(word: &str, s: &SignRank, o: &RankOpts) -> Result<(Path, f64, &'static str), Error> {
    let n = s.d.len();
    let tied = s.ties > 0.0;
    let exact = match o.exact {
        Some(true) if tied || s.zeroes => {
            return Err(domain(
                word,
                "exact",
                format!(
                    "true, where x - mu has {}",
                    if tied { "ties" } else { "zeros" }
                ),
            ));
        }
        Some(true) => {
            if n as u64 > EXACT_CAP {
                return Err(Refusal::Cap {
                    what: format!("an exact signed-rank distribution of {n} values"),
                    cap: EXACT_CAP,
                }
                .error(word));
            }
            true
        }
        Some(false) => false,
        None => n < EXACT_BELOW && !tied && !s.zeroes,
    };
    if exact {
        let dist = SignRankDist::new(n);
        let v = s.v as i64;
        let p = match o.alternative {
            Alternative::TwoSided => {
                let p = if s.v > dist.u as f64 / 2.0 {
                    dist.upper(v)
                } else {
                    dist.lower(v)
                };
                (2.0 * p).min(1.0)
            }
            Alternative::Greater => dist.upper(v),
            Alternative::Less => dist.lower(v),
        };
        return Ok((Path::Exact(dist), p, "Wilcoxon signed rank exact test"));
    }
    let z = normal_z(&s.d, o.alternative, o.correct);
    let (lower, upper) = normal().tails(word, z)?;
    let p = match o.alternative {
        Alternative::TwoSided => 2.0 * lower.min(upper),
        Alternative::Less => lower,
        Alternative::Greater => upper,
    };
    let method = if o.correct {
        "Wilcoxon signed rank test with continuity correction"
    } else {
        "Wilcoxon signed rank test"
    };
    Ok((Path::Normal, p, method))
}

/// The median of sorted values, formed without overflow, as
/// `stat.test.mannwhitney.opts` forms its own.
fn median_sorted(word: &str, a: &[f64]) -> Result<f64, Error> {
    let m = a.len();
    let at = |i: usize| {
        a.get(i)
            .copied()
            .ok_or_else(|| Error::internal(format!("{word}: Walsh average {i} of {m}")))
    };
    let e = if m % 2 == 1 {
        at(m / 2)?
    } else {
        let (u, w) = (at(m / 2 - 1)?, at(m / 2)?);
        let sum = u + w;
        if sum.is_finite() {
            sum / 2.0
        } else {
            u / 2.0 + w / 2.0
        }
    };
    finite(word, e, || "the estimate".into())
}

/// The sorted Walsh averages (xᵢ + xⱼ)/2, i ≤ j, each formed without
/// overflow.
fn walsh(word: &str, xs: &[f64]) -> Result<Vec<f64>, Error> {
    let n = xs.len() as u64;
    let count = n * (n + 1) / 2;
    if count > WALSH_CAP {
        return Err(Refusal::Cap {
            what: format!("{count} Walsh averages"),
            cap: WALSH_CAP,
        }
        .error(word));
    }
    let mut a = Vec::with_capacity(count as usize);
    for i in 0..xs.len() {
        for j in i..xs.len() {
            let s = xs[i] + xs[j];
            a.push(if s.is_finite() {
                s / 2.0
            } else {
                xs[i] / 2.0 + xs[j] / 2.0
            });
        }
    }
    a.sort_by(f64::total_cmp);
    Ok(a)
}

fn too_few(word: &str, operand: &str) -> Error {
    domain(word, operand, "too few for the level".into())
}

/// The exact interval: [A₍q₎, A₍N−q+1₎] over the sorted Walsh averages A,
/// q R's `qsignrank` of α (α/2 two-sided), at least 1; refused where R
/// warns that the level is not achievable.
fn exact_ends(
    word: &str,
    operand: &str,
    dist: &SignRankDist,
    a: &[f64],
    o: &RankOpts,
) -> Result<(Option<f64>, Option<f64>), Error> {
    let alpha = 1.0 - o.conf;
    let side = match o.alternative {
        Alternative::TwoSided => alpha / 2.0,
        _ => alpha,
    };
    let q = dist.quantile(side).max(1);
    let achieved = match o.alternative {
        Alternative::TwoSided => 2.0 * dist.lower(q as i64 - 1),
        _ => dist.lower(q as i64 - 1),
    };
    if achieved - alpha > alpha / 2.0 {
        return Err(too_few(word, operand));
    }
    let at = |i: usize| {
        a.get(i).copied().ok_or_else(|| {
            Error::internal(format!("{word}: Walsh average {} of {}", i + 1, a.len()))
        })
    };
    let lower = || -> Result<f64, Error> { end(word, "lower", at(q - 1)?) };
    let upper = || -> Result<f64, Error> { end(word, "upper", at(dist.u - q)?) };
    Ok(match o.alternative {
        Alternative::TwoSided => (Some(lower()?), Some(upper()?)),
        Alternative::Greater => (Some(lower()?), None),
        Alternative::Less => (None, Some(upper()?)),
    })
}

/// The normal interval: the d at which the corrected z of x − d, its
/// zeros dropped, crosses the normal quantile, by `zeroin` over
/// [min x, max x] at 10⁻¹²·max(1, |min|, |max|) (N7).
fn normal_ends(
    word: &str,
    operand: &str,
    xs: &[f64],
    o: &RankOpts,
) -> Result<(Option<f64>, Option<f64>), Error> {
    let w = |d: f64| -> f64 {
        let xd: Vec<f64> = xs.iter().map(|x| x - d).filter(|x| *x != 0.0).collect();
        normal_z(&xd, o.alternative, o.correct)
    };
    let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (w_lo, w_hi) = (w(lo), w(hi));
    if !w_lo.is_finite() || !w_hi.is_finite() {
        return Err(domain(word, operand, "too few for an interval".into()));
    }
    let alpha = 1.0 - o.conf;
    let side = match o.alternative {
        Alternative::TwoSided => alpha / 2.0,
        _ => alpha,
    };
    // z₁₋ₛ, the upper quantile; the lower is its negative.
    let zq = normal().quantile(word, 1.0 - side)?;
    let wants_lower = o.alternative != Alternative::Less;
    let wants_upper = o.alternative != Alternative::Greater;
    if (wants_lower && w_lo < zq) || (wants_upper && w_hi > -zq) {
        return Err(too_few(word, operand));
    }
    let tol = 1e-12 * 1f64.max(lo.abs()).max(hi.abs());
    let root = |target: f64| -> Result<f64, Error> {
        let mut f = |d: f64| -> Result<f64, Error> { Ok(w(d) - target) };
        let z = zeroin(&mut f, lo, hi, w_lo - target, w_hi - target, tol, MAXITER)?;
        if !z.converged || !z.root.is_finite() {
            return Err(Refusal::NoConvergence {
                method: "the interval's root".into(),
                cap: MAXITER,
            }
            .error(word));
        }
        Ok(z.root)
    };
    let lower = if wants_lower {
        Some(end(word, "lower", root(zq)?)?)
    } else {
        None
    };
    let upper = if wants_upper {
        Some(end(word, "upper", root(-zq)?)?)
    } else {
        None
    };
    Ok((lower, upper))
}

/// A signed-rank answer: the test keys, and with `interval`, the estimate,
/// the ends, `conf` and `alternative`.
fn signed_answer(
    word: &str,
    operand: &str,
    s: &SignRank,
    o: &RankOpts,
    interval: bool,
) -> Result<BundValue, Error> {
    let (path, p, method) = signed_test(word, s, o)?;
    let mut m = answer_map(statistic(word, s.v)?, p, None, method);
    if interval {
        let a = walsh(word, &s.kept)?;
        let estimate = median_sorted(word, &a)?;
        let ends = match &path {
            Path::Exact(dist) => exact_ends(word, operand, dist, &a, o)?,
            Path::Normal => normal_ends(word, operand, &s.kept, o)?,
        };
        let shown = Opts {
            alternative: o.alternative,
            conf: o.conf,
            mu: 0.0,
        };
        with_extra(
            &mut m,
            Extra {
                estimate: Some(estimate),
                ends,
                conf: true,
            },
            &shown,
        );
    }
    Ok(BundValue::map(m))
}

fn defaults() -> RankOpts {
    RankOpts {
        alternative: Alternative::TwoSided,
        conf: 0.95,
        exact: None,
        correct: true,
    }
}

const ALL_ZERO: &str = "every value equal to mu";
const ALL_SAME: &str = "every difference 0";

/// The paired differences a − b, each finite.
fn differences(word: &str, a: &BundValue, b: &BundValue) -> Result<Vec<f64>, Error> {
    let a = whole(word, "a", a)?;
    let b = whole(word, "b", b)?;
    if a.len() != b.len() {
        return Err(Refusal::Disagree {
            what: "lengths".into(),
            n: a.len(),
            m: b.len(),
        }
        .error(word));
    }
    let d: Vec<f64> = a.iter().zip(&b).map(|(x, y)| x - y).collect();
    finite_all(word, "a - b", &d)?;
    Ok(d)
}

fn stat_test_wilcoxon(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.wilcoxon";
    let v = operands(vm, W, 2)?;
    let x = whole(W, "x", &v[0])?;
    let mu = real(W, "mu", &v[1])?;
    let s = sign_rank(W, "x", &x, mu, ALL_ZERO)?;
    vm.push(signed_answer(W, "x", &s, &defaults(), false)?);
    Ok(())
}

fn stat_test_wilcoxon_paired(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.wilcoxon.paired";
    let v = operands(vm, W, 2)?;
    let d = differences(W, &v[0], &v[1])?;
    let s = sign_rank(W, "a - b", &d, 0.0, ALL_SAME)?;
    vm.push(signed_answer(W, "a - b", &s, &defaults(), false)?);
    Ok(())
}

const SIGNED_KEYS: [&str; 4] = ["alternative", "exact", "correct", "conf"];

fn stat_test_wilcoxon_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.wilcoxon.opts";
    let v = operands(vm, W, 3)?;
    let o = read(W, &v[2], &SIGNED_KEYS)?;
    let x = whole(W, "x", &v[0])?;
    let mu = real(W, "mu", &v[1])?;
    let s = sign_rank(W, "x", &x, mu, ALL_ZERO)?;
    vm.push(signed_answer(W, "x", &s, &o, true)?);
    Ok(())
}

fn stat_test_wilcoxon_paired_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.wilcoxon.paired.opts";
    let v = operands(vm, W, 3)?;
    let o = read(W, &v[2], &SIGNED_KEYS)?;
    let d = differences(W, &v[0], &v[1])?;
    let s = sign_rank(W, "a - b", &d, 0.0, ALL_SAME)?;
    vm.push(signed_answer(W, "a - b", &s, &o, true)?);
    Ok(())
}

// Kruskal–Wallis and Friedman.

/// A LIST of at least two series, each a LIST or a METRICS read as `whole`
/// reads one and holding at least one value, each named by its 1-based
/// position.
fn groups(word: &str, v: &BundValue) -> Result<Vec<Vec<f64>>, Error> {
    let items = match v.as_list() {
        Some(items) if v.dt() == LIST => items,
        _ => return Err(domain(word, "groups", v.summary(WIDTH))),
    };
    if items.len() < 2 {
        return Err(domain(
            word,
            "groups",
            format!(
                "a list of {} group{}",
                items.len(),
                if items.len() == 1 { "" } else { "s" }
            ),
        ));
    }
    let mut out = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let name = format!("group {}", i + 1);
        let g = whole(word, &name, item)?;
        if g.is_empty() {
            return Err(domain(word, &name, "an empty series".into()));
        }
        out.push(g);
    }
    Ok(out)
}

fn stat_test_kruskal(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.kruskal";
    let v = operands(vm, W, 1)?;
    let g = groups(W, &v[0])?;
    let all: Vec<f64> = g.iter().flatten().copied().collect();
    let n = all.len() as f64;
    let r = ranks(&all);
    let mut s = 0.0;
    let mut at = 0;
    for grp in &g {
        let sum: f64 = r[at..at + grp.len()].iter().sum();
        s += sum * sum / grp.len() as f64;
        at += grp.len();
    }
    let denominator = 1.0 - tie_term(&all) / (n * n * n - n);
    if denominator <= 0.0 {
        return Err(constant(W, "groups"));
    }
    let h = statistic(
        W,
        (12.0 * s / (n * (n + 1.0)) - 3.0 * (n + 1.0)) / denominator,
    )?;
    let df = (g.len() - 1) as f64;
    vm.push(BundValue::map(answer_map(
        h,
        chisq_p(W, h, df)?,
        Some(df),
        "Kruskal-Wallis rank sum test",
    )));
    Ok(())
}

fn stat_test_friedman(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.friedman";
    let v = operands(vm, W, 1)?;
    let t = table(W, "table", &v[0])?;
    let (rows, k) = (t.len(), t.first().map_or(0, Vec::len));
    if rows < 1 || k < 2 {
        return Err(domain(W, "table", format!("a table of {rows} by {k}")));
    }
    let (n, kf) = (rows as f64, k as f64);
    let mut cols = vec![0.0; k];
    let mut ties = 0.0;
    for row in &t {
        for (c, r) in cols.iter_mut().zip(ranks(row)) {
            *c += r;
        }
        ties += tie_term(row);
    }
    let denominator = n * kf * (kf + 1.0) - ties / (kf - 1.0);
    if denominator <= 0.0 {
        return Err(domain(W, "table", "every row tied throughout".into()));
    }
    let centre = n * (kf + 1.0) / 2.0;
    let ss: f64 = cols.iter().map(|c| (c - centre) * (c - centre)).sum();
    let q = statistic(W, 12.0 * ss / denominator)?;
    let df = kf - 1.0;
    vm.push(BundValue::map(answer_map(
        q,
        chisq_p(W, q, df)?,
        Some(df),
        "Friedman rank sum test",
    )));
    Ok(())
}

// McNemar.

fn mcnemar(word: &str, v: &BundValue, correct: bool) -> Result<BundValue, Error> {
    let t = super::counts(word, v)?;
    let (r, c) = (t.len(), t.first().map_or(0, Vec::len));
    if r < 2 || c != r {
        return Err(domain(word, "table", format!("a table of {r} by {c}")));
    }
    let asymmetric = (0..r).any(|i| (0..r).any(|j| t[i][j] != t[j][i]));
    let corrected = correct && r == 2 && asymmetric;
    let mut x2 = 0.0;
    let pairs = (0..r).flat_map(|i| (i + 1..r).map(move |j| (i, j)));
    for (i, j) in pairs {
        let both = t[i][j] + t[j][i];
        if both == 0.0 {
            return Err(domain(
                word,
                "table",
                format!("[{},{}] and [{},{}] both 0", i + 1, j + 1, j + 1, i + 1),
            ));
        }
        let d = (t[i][j] - t[j][i]).abs() - if corrected { 1.0 } else { 0.0 };
        x2 += d * d / both;
    }
    let x2 = statistic(word, x2)?;
    let df = (r * (r - 1) / 2) as f64;
    let method = if corrected {
        "McNemar's Chi-squared test with continuity correction"
    } else {
        "McNemar's Chi-squared test"
    };
    Ok(BundValue::map(answer_map(
        x2,
        chisq_p(word, x2, df)?,
        Some(df),
        method,
    )))
}

fn stat_test_mcnemar(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.mcnemar";
    let v = operands(vm, W, 1)?;
    vm.push(mcnemar(W, &v[0], true)?);
    Ok(())
}

fn stat_test_mcnemar_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.mcnemar.opts";
    let v = operands(vm, W, 2)?;
    let o = read(W, &v[1], &["correct"])?;
    vm.push(mcnemar(W, &v[0], o.correct)?);
    Ok(())
}

// Cochran–Mantel–Haenszel.

/// The interval's level: CMH has no `.opts` word.
const CMH_CONF: f64 = 0.95;

/// A stratum written as a LIST of rows, each a LIST, as a MATRIX of those
/// rows (D271), so that `table` reads it and refuses it as it refuses a
/// MATRIX; anything else as it is.
fn as_rows(item: &BundValue) -> BundValue {
    let Some(rows) = item.as_list().filter(|_| item.dt() == LIST) else {
        return item.clone();
    };
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        match row.as_list() {
            Some(cells) if row.dt() == LIST => out.push(cells.to_vec()),
            _ => return item.clone(),
        }
    }
    BundValue::matrix(out)
}

/// The strata: a LIST of at least two 2×2 MATRIXes of counts, or LISTs of
/// two rows of two (D271), each summing to 2 or more, each named by its
/// 1-based position.
fn strata(word: &str, v: &BundValue) -> Result<Vec<[f64; 4]>, Error> {
    let items = match v.as_list() {
        Some(items) if v.dt() == LIST => items,
        _ => return Err(domain(word, "tables", v.summary(WIDTH))),
    };
    if items.len() < 2 {
        return Err(domain(
            word,
            "tables",
            format!(
                "{} strat{}",
                items.len(),
                if items.len() == 1 { "um" } else { "a" }
            ),
        ));
    }
    let mut out = Vec::with_capacity(items.len());
    for (k, item) in items.iter().enumerate() {
        let name = format!("stratum {}", k + 1);
        let t = table(word, &name, &as_rows(item))?;
        let (r, c) = (t.len(), t.first().map_or(0, Vec::len));
        if r != 2 || c != 2 {
            return Err(domain(word, &name, format!("a table of {r} by {c}")));
        }
        let cells = [t[0][0], t[0][1], t[1][0], t[1][1]];
        if let Some(i) = cells.iter().position(|x| *x < 0.0) {
            return Err(domain(
                word,
                &name,
                format!("{} at [{},{}]", cells[i], i / 2 + 1, i % 2 + 1),
            ));
        }
        let n: f64 = cells.iter().sum();
        if n < 2.0 {
            return Err(domain(word, &name, format!("a sample of {n}")));
        }
        out.push(cells);
    }
    Ok(out)
}

fn stat_test_cmh(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.cmh";
    let v = operands(vm, W, 1)?;
    let s = strata(W, &v[0])?;
    let (mut delta, mut var, mut diag, mut offd) = (0.0, 0.0, 0.0, 0.0);
    let (mut p1, mut p2, mut p3) = (0.0, 0.0, 0.0);
    for [a, b, c, d] in &s {
        let n = a + b + c + d;
        delta += a - (a + b) * (a + c) / n;
        var += (a + b) * (c + d) * (a + c) * (b + d) / (n * n * (n - 1.0));
        diag += a * d / n;
        offd += b * c / n;
        p1 += (a + d) * a * d / (n * n);
        p2 += ((a + d) * b * c + (b + c) * a * d) / (n * n);
        p3 += (b + c) * b * c / (n * n);
    }
    if var <= 0.0 {
        return Err(domain(W, "tables", "a variance of 0".into()));
    }
    if diag == 0.0 {
        return Err(domain(W, "tables", "a common odds ratio of 0".into()));
    }
    if offd == 0.0 {
        return Err(domain(W, "tables", "an infinite common odds ratio".into()));
    }
    let yates = if delta.abs() >= 0.5 { 0.5 } else { 0.0 };
    let x2 = statistic(W, (delta.abs() - yates).powi(2) / var)?;
    let method = if yates > 0.0 {
        "Mantel-Haenszel chi-squared test with continuity correction"
    } else {
        "Mantel-Haenszel chi-squared test without continuity correction"
    };
    let mut m = answer_map(x2, chisq_p(W, x2, 1.0)?, Some(1.0), method);
    let estimate = finite(W, diag / offd, || "the common odds ratio".into())?;
    let sd =
        (p1 / (2.0 * diag * diag) + p2 / (2.0 * diag * offd) + p3 / (2.0 * offd * offd)).sqrt();
    let z = normal().quantile(W, 1.0 - (1.0 - CMH_CONF) / 2.0)?;
    let lower = end(W, "lower", estimate * (-z * sd).exp())?;
    let upper = end(W, "upper", estimate * (z * sd).exp())?;
    m.insert("estimate".to_string(), BundValue::float(estimate));
    m.insert("lower".to_string(), BundValue::float(lower));
    m.insert("upper".to_string(), BundValue::float(upper));
    m.insert("conf".to_string(), BundValue::float(CMH_CONF));
    vm.push(BundValue::map(m));
    Ok(())
}

/// Register the words, each plain and `.`, the `.` form taking the top
/// operand from the workbench and answering there.
pub(super) fn register(r: &mut bund2_api::Registry) {
    use bund2_api::{StackEffect, WordKind};
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("stat.test.wilcoxon", stat_test_wilcoxon, 2);
    word!("stat.test.wilcoxon.paired", stat_test_wilcoxon_paired, 2);
    word!("stat.test.wilcoxon.opts", stat_test_wilcoxon_opts, 3);
    word!(
        "stat.test.wilcoxon.paired.opts",
        stat_test_wilcoxon_paired_opts,
        3
    );
    word!("stat.test.kruskal", stat_test_kruskal, 1);
    word!("stat.test.friedman", stat_test_friedman, 1);
    word!("stat.test.mcnemar", stat_test_mcnemar, 1);
    word!("stat.test.mcnemar.opts", stat_test_mcnemar_opts, 2);
    word!("stat.test.cmh", stat_test_cmh, 1);
    word!("stat.test.shapiro", normality::stat_test_shapiro, 1);
    word!("stat.test.ks1", normality::stat_test_ks1, 2);
    word!("stat.test.bartlett", normality::stat_test_bartlett, 1);
    word!("stat.test.levene", normality::stat_test_levene, 1);
    word!("stat.test.levene.opts", normality::stat_test_levene_opts, 2);
    word!("stat.test.anova1", anova::stat_test_anova1, 1);
    word!("stat.test.anova1.opts", anova::stat_test_anova1_opts, 2);
    word!("stat.test.tukey", anova::stat_test_tukey, 2);
}

/// §S5's words with their `.` forms (M4's and M5's), and §S12's `stat.test.`
/// words (M6's).
pub const WORDS: [&str; 34] = [
    "stat.test.wilcoxon",
    "stat.test.wilcoxon.",
    "stat.test.wilcoxon.paired",
    "stat.test.wilcoxon.paired.",
    "stat.test.wilcoxon.opts",
    "stat.test.wilcoxon.opts.",
    "stat.test.wilcoxon.paired.opts",
    "stat.test.wilcoxon.paired.opts.",
    "stat.test.kruskal",
    "stat.test.kruskal.",
    "stat.test.friedman",
    "stat.test.friedman.",
    "stat.test.mcnemar",
    "stat.test.mcnemar.",
    "stat.test.mcnemar.opts",
    "stat.test.mcnemar.opts.",
    "stat.test.cmh",
    "stat.test.cmh.",
    "stat.test.shapiro",
    "stat.test.shapiro.",
    "stat.test.ks1",
    "stat.test.ks1.",
    "stat.test.bartlett",
    "stat.test.bartlett.",
    "stat.test.levene",
    "stat.test.levene.",
    "stat.test.levene.opts",
    "stat.test.levene.opts.",
    "stat.test.anova1",
    "stat.test.anova1.",
    "stat.test.anova1.opts",
    "stat.test.anova1.opts.",
    "stat.test.tukey",
    "stat.test.tukey.",
];

mod anova;
mod normality;

#[cfg(test)]
mod tests;
