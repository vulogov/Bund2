//! RFC-0019 §S4: power and sample size.
//!
//! Each word takes one options MAP, `opts -- MAP`, solves for the one
//! parameter left out, and answers every parameter with `method` naming the
//! calculation, as R's `power.t.test`, `power.prop.test` and
//! `power.anova.test` (R 4.4.2, `src/library/stats/R/power.R`,
//! `power.anova.test.R`):
//!
//! - **`power.t`**: the noncentral t's upper tail above the central t's
//!   upper `sig.level/sides` quantile, ν = max(10⁻⁷, n − 1)·tsample and
//!   ncp = √(n/tsample)·δ/sd; with `strict` and two sides, plus the lower
//!   tail below minus that quantile. Two-sided takes |delta| as given.
//!   Above 4·10⁵ degrees of freedom both tails are R's own normal
//!   approximation, as `pt` and `pnt` answer there.
//! - **`power.prop`**: R's normal approximation.
//! - **`power.anova`**: the noncentral F's upper tail above the central F's
//!   upper `sig.level` quantile, on groups − 1 and (n − 1)·groups, with
//!   λ = (groups − 1)·n·between.var/within.var.
//! - **`power.chisq`**, Bund2's own: the noncentral χ²'s upper tail above
//!   the central χ²'s upper `sig.level` quantile on `df`, λ = n·w² (Cohen's
//!   w). `df` cannot be solved for.
//!
//! **Left out** is a key missing without a default, or set to NONE (§S1):
//! this module reads its options itself, and NONE marks the parameter to
//! solve for. Exactly one must be left out, else E101 naming them.
//!
//! **Solving** is [`super::optim`]'s `uniroot` port over R's intervals, with
//! R's `extendInt` where R extends (`power.R:53-64`, `:121-135`); `power.prop`'s
//! p1 and p2 and every `power.anova` parameter without extension, as §S4
//! rules. Outside an equation's domain (a probability outside (0, 1), a
//! negative n) the power is NaN, as R's, so that an extension steps back
//! or stops where R's does. The tolerance is 10⁻¹⁰ of the smaller end of
//! the bracket, so the root is found to 10⁻¹⁰ relative (N7), not R's ε^¼.
//! No sign change is E110 for "the search for a sign change"; Brent's
//! method at its cap is E110 for "the power equation's root". `n` is
//! answered as a FLOAT.

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BOOL, BundValue, INTEGER, MAP, NONE, STRING};

use super::dist::{Family, normal_tails};
use super::operand::{WIDTH, domain, finite, operands, real};
use super::optim::{Bracket, Extend, MAXITER, Stop, extend, search_failed, zeroin};
use super::refuse::Refusal;

/// R's `sig.level` interval, `c(1e-10, 1-1e-10)`.
const SIG: (f64, f64) = (1e-10, 1.0 - 1e-10);

/// A word's numeric parameters, in its answer's order, and their defaults.
struct Spec {
    word: &'static str,
    params: &'static [&'static str],
    defaults: &'static [(&'static str, f64)],
    others: &'static [&'static str],
}

/// The options MAP, every key one the word takes.
fn options<'a>(s: &Spec, v: &'a BundValue) -> Result<&'a BTreeMap<String, BundValue>, Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(s.word, "opts", v.summary(WIDTH))),
    };
    for k in m.keys() {
        if !s.params.contains(&k.as_str()) && !s.others.contains(&k.as_str()) {
            return Err(domain(s.word, "opts", format!("an unknown key {k:?}")));
        }
    }
    Ok(m)
}

/// Each parameter given, or `None` when left out: missing without a
/// default, or NONE.
fn given(s: &Spec, m: &BTreeMap<String, BundValue>) -> Result<Vec<Option<f64>>, Error> {
    s.params
        .iter()
        .map(|k| match m.get(*k) {
            None => Ok(s.defaults.iter().find(|d| d.0 == *k).map(|d| d.1)),
            Some(x) if x.dt() == NONE => Ok(None),
            Some(x) => real(s.word, k, x).map(Some),
        })
        .collect()
}

/// The index of the one parameter left out.
fn the_one(s: &Spec, p: &[Option<f64>]) -> Result<usize, Error> {
    let out: Vec<usize> = (0..p.len()).filter(|&i| p[i].is_none()).collect();
    if let [i] = out[..] {
        return Ok(i);
    }
    let all = s.params.join(", ");
    let value = if out.is_empty() {
        format!("none of {all} is left out; exactly one must be")
    } else {
        let names: Vec<&str> = out.iter().map(|&i| s.params[i]).collect();
        format!(
            "{} are left out; exactly one of {all} must be",
            names.join(" and ")
        )
    };
    Err(domain(s.word, "opts", value))
}

/// E101 naming `key` unless `ok`, showing the value as the program wrote it.
fn check(
    s: &Spec,
    m: &BTreeMap<String, BundValue>,
    key: &str,
    x: Option<f64>,
    ok: impl Fn(f64) -> bool,
) -> Result<(), Error> {
    match x {
        Some(y) if !ok(y) => Err(domain(
            s.word,
            key,
            m.get(key)
                .map_or_else(|| format!("{y}"), |v| v.summary(WIDTH)),
        )),
        _ => Ok(()),
    }
}

fn probability(y: f64) -> bool {
    y > 0.0 && y < 1.0
}

/// `sides`: the INTEGER 1 or 2, 2 by default.
fn sides(s: &Spec, m: &BTreeMap<String, BundValue>) -> Result<f64, Error> {
    match m.get("sides") {
        None => Ok(2.0),
        Some(x) => match x.unboxed() {
            BundValue::Int(1, _) => Ok(1.0),
            BundValue::Int(2, _) => Ok(2.0),
            _ => Err(domain(s.word, "sides", x.summary(WIDTH))),
        },
    }
}

/// `strict`: a BOOL, false by default.
fn strict(s: &Spec, m: &BTreeMap<String, BundValue>) -> Result<bool, Error> {
    match m.get("strict") {
        None => Ok(false),
        Some(x) => match x.unboxed() {
            BundValue::Bool(b, _) if x.dt() == BOOL => Ok(*b),
            _ => Err(domain(s.word, "strict", x.summary(WIDTH))),
        },
    }
}

/// E101 naming `key` for an interval its value empties.
fn empties(s: &Spec, m: &BTreeMap<String, BundValue>, key: &str, solve: &str) -> Error {
    let shown = m.get(key).map_or_else(String::new, |v| v.summary(WIDTH));
    domain(
        s.word,
        key,
        format!("{shown}, which leaves {solve} no interval"),
    )
}

/// The root of `f` over [lower, upper], extended as `how`, by §S6's port of
/// `uniroot`.
fn solve(
    word: &str,
    f: &mut dyn FnMut(f64) -> Result<f64, Error>,
    lower: f64,
    upper: f64,
    how: Extend,
) -> Result<f64, Error> {
    let start = Bracket {
        lower,
        f_lower: f(lower)?,
        upper,
        f_upper: f(upper)?,
        steps: 0,
    };
    let b = extend(f, start, how, MAXITER).map_err(|s| match s {
        Stop::F(e) => e,
        Stop::Search | Stop::SameSign => search_failed(word, MAXITER),
    })?;
    let cap = || {
        Refusal::NoConvergence {
            method: "the power equation's root".into(),
            cap: MAXITER,
        }
        .error(word)
    };
    let tol = 1e-10 * b.lower.abs().min(b.upper.abs());
    let mut g = |x: f64| match f(x)? {
        y if y.is_nan() => Err(cap()),
        y => Ok(y),
    };
    let z = zeroin(&mut g, b.lower, b.upper, b.f_lower, b.f_upper, tol, MAXITER)?;
    if z.converged { Ok(z.root) } else { Err(cap()) }
}

/// The upper `alpha` quantile of a continuous family on (0, ∞): its
/// quantile at 1 − alpha, then Newton's steps on the upper tail itself, so
/// that a small alpha keeps its relative precision.
fn upper_quantile(word: &str, f: &Family, alpha: f64) -> Result<f64, Error> {
    let x = f.quantile(word, 1.0 - alpha)?;
    if !x.is_finite() {
        return Ok(x);
    }
    newton_upper(word, f, alpha, x).map(|y| if y > 0.0 { y } else { x })
}

/// The central t's upper `alpha` quantile, by bracketed Newton on the
/// upper tail itself: `dist.t`'s quantile polishes the lower tail to a
/// relative 10⁻¹² check that it misses near p = ½ and above ν ≈ 10⁶, both
/// of which R's intervals reach. Above ½ it is minus the quantile at
/// 1 − alpha, the t being symmetric. A bracket that overflows, as at
/// ν = 2·10⁻⁷ (`pmax(1e-7, n - 1)`), is +∞, as R's `qt`, so that the power
/// there is 0. The start is the Cornish–Fisher z + (z³ + z)/(4ν).
fn t_upper(w: &str, nu: f64, alpha: f64) -> Result<f64, Error> {
    if alpha > 0.5 {
        return t_upper(w, nu, 1.0 - alpha).map(|x| -x);
    }
    if alpha == 0.5 {
        return Ok(0.0);
    }
    let t = Family::T { df: nu };
    let sf = |x: f64| t.tails(w, x).map(|p| p.1);
    let z = z_upper(w, alpha)?;
    let guess = z + (z * z * z + z) / (4.0 * nu);
    // sf(a) > alpha ≥ sf(b).
    let (mut a, mut b) = (0.0, guess.max(1.0));
    while sf(b)? > alpha {
        a = b;
        b *= 2.0;
        if !b.is_finite() {
            return Ok(f64::INFINITY);
        }
    }
    let mut x = if guess > a && guess < b {
        guess
    } else {
        0.5 * (a + b)
    };
    for _ in 0..200 {
        let f = sf(x)? - alpha;
        if f == 0.0 {
            return Ok(x);
        }
        if f > 0.0 {
            a = x;
        } else {
            b = x;
        }
        let d = t.ln_pdf(w, x)?.exp();
        let newton = x + f / d;
        let next = if d > 0.0 && newton > a && newton < b {
            newton
        } else {
            0.5 * (a + b)
        };
        if (next - x).abs() <= 2.0 * f64::EPSILON * next.abs() || b - a <= 4.0 * f64::EPSILON * b {
            return Ok(next);
        }
        x = next;
    }
    Ok(x)
}

/// Newton's steps on f's upper tail toward `alpha` from `guess`, at most 20,
/// stopping at a step of a few ulps or where the density fails.
fn newton_upper(word: &str, f: &Family, alpha: f64, guess: f64) -> Result<f64, Error> {
    let mut x = guess;
    for _ in 0..20 {
        let sf = f.tails(word, x)?.1;
        let d = f.ln_pdf(word, x)?.exp();
        if !(d > 0.0 && d.is_finite()) {
            break;
        }
        let step = (sf - alpha) / d;
        if !(x + step).is_finite() {
            break;
        }
        x += step;
        if step.abs() <= 4.0 * f64::EPSILON * x.abs() {
            break;
        }
    }
    Ok(x)
}

/// The standard normal's upper `alpha` quantile.
fn z_upper(word: &str, alpha: f64) -> Result<f64, Error> {
    Family::Normal { mean: 0.0, sd: 1.0 }
        .quantile(word, alpha)
        .map(|z| -z)
}

/// The answer MAP: every parameter as a FLOAT, then the rest.
fn answer(s: &Spec, p: &[f64], rest: Vec<(&str, BundValue)>) -> BundValue {
    let mut m = BTreeMap::new();
    for (k, x) in s.params.iter().zip(p) {
        m.insert((*k).to_string(), BundValue::float(*x));
    }
    for (k, v) in rest {
        m.insert(k.to_string(), v);
    }
    BundValue::map(m)
}

/// A word's power at its parameters.
type Power<'a> = dyn Fn(&str, &[f64]) -> Result<f64, Error> + 'a;

/// Solve for parameter `i` of `p` (its target `power` at `ip`), or compute
/// the power when `i` is `ip`; `p` is complete afterwards.
fn complete(
    s: &Spec,
    p: &mut [f64],
    i: usize,
    ip: usize,
    power: &Power<'_>,
    interval: (f64, f64, Extend),
) -> Result<(), Error> {
    let w = s.word;
    if i == ip {
        let y = power(w, p)?;
        p[ip] = finite(w, y, || format!("the power at {}", s.params.join(", ")))?;
        return Ok(());
    }
    let target = p[ip];
    let mut q = p.to_vec();
    let mut f = |x: f64| {
        q[i] = x;
        power(w, &q).map(|y| y - target)
    };
    let (lo, hi, how) = interval;
    p[i] = solve(w, &mut f, lo, hi, how)?;
    Ok(())
}

// power.t

const T: Spec = Spec {
    word: "power.t",
    params: &["n", "delta", "sd", "sig.level", "power"],
    defaults: &[("sd", 1.0), ("sig.level", 0.05)],
    others: &["type", "sides", "strict"],
};

/// `power.t.test`'s `p.body` (`power.R:38-49`).
fn t_power(w: &str, p: &[f64], tsample: f64, sides: f64, strict: bool) -> Result<f64, Error> {
    let (n, delta, sd, alpha) = (p[0], p[1], p[2], p[3]);
    if !(n >= 0.0 && sd > 0.0 && probability(alpha)) {
        return Ok(f64::NAN);
    }
    let nu = (n - 1.0).max(1e-7) * tsample;
    let ncp = (n / tsample).sqrt() * delta / sd;
    if nu > LARGE_DF {
        return t_power_large(w, nu, ncp, alpha / sides, strict && sides == 2.0);
    }
    let qu = t_upper(w, nu, alpha / sides)?;
    let mut power = nct_above(w, nu, ncp, qu)?;
    if strict && sides == 2.0 {
        // P(T ≤ −qu) at ncp is P(T ≥ qu) at −ncp, T's law mirrored.
        power += nct_above(w, nu, -ncp, qu)?;
    }
    Ok(power)
}

/// Below this, a tail is 0 to double precision beside 1, and 1 − it is 1.
const NEGLIGIBLE: f64 = 1e-17;

/// The noncentral t's upper tail at t ≥ 0, from `dist.nct`'s series, except
/// where its value rounds to 1.0 or 0.0 and the series would run past its
/// cap: the ends of R's intervals, ncp to 10⁷·√n.
///
/// T ≤ t needs Z + δ ≤ δ/2 or tS ≥ δ/2, S² = V/ν, so for δ > 0
/// P(T ≤ t) ≤ Φ(−δ/2) + P(V/ν ≥ c), c = (δ/2t)², the second at most
/// Chernoff's exp(−ν/2·(c − 1 − ln c)) when c > 1. Below [`NEGLIGIBLE`] the
/// tail is 1.0. For δ < 0, T > t ≥ 0 needs Z + δ > 0, so P(T > t) ≤ Φ(δ),
/// below the least subnormal when δ < −39, and the tail is 0.0.
fn nct_above(w: &str, nu: f64, ncp: f64, t: f64) -> Result<f64, Error> {
    if t == f64::INFINITY {
        return Ok(0.0);
    }
    if ncp > 0.0 {
        let c = if t == 0.0 {
            f64::INFINITY
        } else {
            (ncp / (2.0 * t)).powi(2)
        };
        let chernoff = if c.is_infinite() {
            0.0
        } else if c > 1.0 {
            (-nu / 2.0 * (c - 1.0 - c.ln())).exp()
        } else {
            1.0
        };
        if normal_tails(-ncp / 2.0).0 + chernoff < NEGLIGIBLE {
            return Ok(1.0);
        }
    } else if ncp < -39.0 {
        return Ok(0.0);
    }
    Ok(Family::Nct { df: nu, ncp }.tails(w, t)?.1)
}

/// Above this many degrees of freedom R's `pt` and `pnt` answer by a
/// normal approximation (R 4.4.2, `src/nmath/pt.c:44-48`, `pnt.c:79-87`),
/// and so does `power.t`: n's interval reaches 10⁷, where `dist.t`'s
/// quantile does not converge.
const LARGE_DF: f64 = 4e5;

/// `power.t`'s body above [`LARGE_DF`]: with s = 1/(4ν), P(T ≤ t) is
/// Φ((t(1 − s) − ncp)/√(1 + 2st²)) (Abramowitz and Stegun 26.7.10, as
/// `pnt.c`), and the central quantile is that equation solved at ncp = 0,
/// t = z/√((1 − s)² − 2sz²), as `pt.c`'s 26.7.8 inverted. Its error is
/// O(ν⁻²) at the critical values a test uses.
fn t_power_large(w: &str, nu: f64, ncp: f64, alpha: f64, both: bool) -> Result<f64, Error> {
    let s = 1.0 / (4.0 * nu);
    let z = z_upper(w, alpha)?;
    let qu = z / ((1.0 - s) * (1.0 - s) - 2.0 * s * z * z).sqrt();
    let at = |t: f64| normal_tails((t * (1.0 - s) - ncp) / (1.0 + 2.0 * s * t * t).sqrt());
    let mut power = at(qu).1;
    if both {
        power += at(-qu).0;
    }
    Ok(power)
}

fn power_t(vm: &mut dyn Vm) -> Result<(), Error> {
    let s = &T;
    let w = s.word;
    let v = operands(vm, w, 1)?;
    let m = options(s, &v[0])?;
    let mut given = given(s, m)?;
    let (tsample, kind) = match m.get("type") {
        None => (2.0, "two.sample"),
        Some(x) => match x.as_str().as_deref() {
            Some("two.sample") if x.dt() == STRING => (2.0, "two.sample"),
            Some("one.sample") if x.dt() == STRING => (1.0, "one.sample"),
            Some("paired") if x.dt() == STRING => (1.0, "paired"),
            _ => return Err(domain(w, "type", x.summary(WIDTH))),
        },
    };
    let sides = sides(s, m)?;
    let strict = strict(s, m)?;
    let i = the_one(s, &given)?;
    check(s, m, "n", given[0], |n| n >= 2.0)?;
    check(s, m, "sd", given[2], |sd| sd > 0.0)?;
    check(s, m, "sig.level", given[3], probability)?;
    check(s, m, "power", given[4], probability)?;
    if sides == 2.0 {
        given[1] = given[1].map(f64::abs);
    }
    let mut p: Vec<f64> = given.iter().map(|x| x.unwrap_or(f64::NAN)).collect();
    let interval = match i {
        0 => (2.0, 1e7, Extend::UpX),
        1 => (p[2] * 1e-7, p[2] * 1e7, Extend::UpX),
        2 => {
            if p[1].is_nan() || p[1] <= 0.0 {
                return Err(empties(s, m, "delta", "sd"));
            }
            (p[1] * 1e-7, p[1] * 1e7, Extend::DownX)
        }
        _ => (SIG.0, SIG.1, Extend::Yes),
    };
    complete(
        s,
        &mut p,
        i,
        4,
        &|w, q| t_power(w, q, tsample, sides, strict),
        interval,
    )?;
    let method = match kind {
        "one.sample" => "One-sample t test power calculation",
        "paired" => "Paired t test power calculation",
        _ => "Two-sample t test power calculation",
    };
    vm.push(answer(
        s,
        &p,
        vec![
            ("type", BundValue::str(kind)),
            ("sides", BundValue::int(sides as i64)),
            ("strict", BundValue::boolean(strict)),
            ("method", BundValue::str(method)),
        ],
    ));
    Ok(())
}

// power.prop

const PROP: Spec = Spec {
    word: "power.prop",
    params: &["n", "p1", "p2", "sig.level", "power"],
    defaults: &[("sig.level", 0.05)],
    others: &["sides", "strict"],
};

/// `power.prop.test`'s `p.body` (`power.R:96-116`).
fn prop_power(w: &str, p: &[f64], sides: f64, strict: bool) -> Result<f64, Error> {
    let (n, p1, p2, alpha) = (p[0], p[1], p[2], p[3]);
    let unit = |x: f64| (0.0..=1.0).contains(&x);
    if !(n >= 0.0 && unit(p1) && unit(p2) && probability(alpha)) {
        return Ok(f64::NAN);
    }
    let qu = z_upper(w, alpha / sides)?;
    let d = (p1 - p2).abs();
    let pbar = (p1 + p2) / 2.0;
    let spread = (2.0 * pbar * (1.0 - pbar)).sqrt();
    let sd = (p1 * (1.0 - p1) + p2 * (1.0 - p2)).sqrt();
    let power = normal_tails((n.sqrt() * d - qu * spread) / sd).0;
    if strict && sides == 2.0 {
        return Ok(power + normal_tails((n.sqrt() * d + qu * spread) / sd).1);
    }
    Ok(power)
}

fn power_prop(vm: &mut dyn Vm) -> Result<(), Error> {
    let s = &PROP;
    let w = s.word;
    let v = operands(vm, w, 1)?;
    let m = options(s, &v[0])?;
    let given = given(s, m)?;
    let sides = sides(s, m)?;
    let strict = strict(s, m)?;
    let i = the_one(s, &given)?;
    check(s, m, "n", given[0], |n| n >= 1.0)?;
    check(s, m, "p1", given[1], probability)?;
    check(s, m, "p2", given[2], probability)?;
    check(s, m, "sig.level", given[3], probability)?;
    check(s, m, "power", given[4], probability)?;
    let mut p: Vec<f64> = given.iter().map(|x| x.unwrap_or(f64::NAN)).collect();
    let interval = match i {
        0 => (1.0, 1e7, Extend::UpX),
        // Without extension: R's "yes" can leave (0, 1) (§S4).
        1 => (0.0, p[2], Extend::No),
        2 => (p[1], 1.0, Extend::No),
        _ => (SIG.0, SIG.1, Extend::UpX),
    };
    complete(
        s,
        &mut p,
        i,
        4,
        &|w, q| prop_power(w, q, sides, strict),
        interval,
    )?;
    vm.push(answer(
        s,
        &p,
        vec![
            ("sides", BundValue::int(sides as i64)),
            ("strict", BundValue::boolean(strict)),
            (
                "method",
                BundValue::str("Two-sample comparison of proportions power calculation"),
            ),
        ],
    ));
    Ok(())
}

// power.anova

const ANOVA: Spec = Spec {
    word: "power.anova",
    params: &[
        "groups",
        "n",
        "between.var",
        "within.var",
        "sig.level",
        "power",
    ],
    defaults: &[("sig.level", 0.05)],
    others: &[],
};

/// `power.anova.test`'s `p.body` (`power.anova.test.R:34-38`).
fn anova_power(w: &str, p: &[f64]) -> Result<f64, Error> {
    let (g, n, between, within, alpha) = (p[0], p[1], p[2], p[3], p[4]);
    if !(g > 1.0 && n > 1.0 && between >= 0.0 && within > 0.0 && probability(alpha)) {
        return Ok(f64::NAN);
    }
    let (d1, d2) = (g - 1.0, (n - 1.0) * g);
    // F's upper quantile is the reciprocal of its mirror's lower one.
    let crit = 1.0 / Family::F { df1: d2, df2: d1 }.quantile(w, alpha)?;
    let ncf = Family::NcF {
        df1: d1,
        df2: d2,
        ncp: d1 * n * (between / within),
    };
    Ok(ncf.tails(w, crit)?.1)
}

fn power_anova(vm: &mut dyn Vm) -> Result<(), Error> {
    let s = &ANOVA;
    let w = s.word;
    let v = operands(vm, w, 1)?;
    let m = options(s, &v[0])?;
    let given = given(s, m)?;
    let i = the_one(s, &given)?;
    check(s, m, "groups", given[0], |g| g >= 2.0)?;
    check(s, m, "n", given[1], |n| n >= 2.0)?;
    check(s, m, "between.var", given[2], |b| b >= 0.0)?;
    check(s, m, "within.var", given[3], |x| x > 0.0)?;
    check(s, m, "sig.level", given[4], probability)?;
    check(s, m, "power", given[5], probability)?;
    let mut p: Vec<f64> = given.iter().map(|x| x.unwrap_or(f64::NAN)).collect();
    // R's intervals, none extended (`power.anova.test.R:42-55`).
    let interval = match i {
        0 => (2.0, 100.0, Extend::No),
        1 => (2.0, 1e5, Extend::No),
        2 => (p[3] * 1e-7, p[3] * 1e7, Extend::No),
        3 => {
            if p[2].is_nan() || p[2] <= 0.0 {
                return Err(empties(s, m, "between.var", "within.var"));
            }
            (p[2] * 1e-7, p[2] * 1e7, Extend::No)
        }
        _ => (SIG.0, SIG.1, Extend::No),
    };
    complete(s, &mut p, i, 5, &anova_power, interval)?;
    vm.push(answer(
        s,
        &p,
        vec![(
            "method",
            BundValue::str("Balanced one-way analysis of variance power calculation"),
        )],
    ));
    Ok(())
}

// power.chisq

const CHISQ: Spec = Spec {
    word: "power.chisq",
    params: &["w", "n", "sig.level", "power"],
    defaults: &[("sig.level", 0.05)],
    others: &["df"],
};

/// The noncentral χ²'s upper tail at λ = n·w² above the central upper
/// `sig.level` quantile on `df`.
fn chisq_power(word: &str, p: &[f64], df: f64) -> Result<f64, Error> {
    let (w, n, alpha) = (p[0], p[1], p[2]);
    if !(w >= 0.0 && n >= 0.0 && probability(alpha)) {
        return Ok(f64::NAN);
    }
    let crit = upper_quantile(word, &Family::ChiSq { df }, alpha)?;
    let lambda = n * w * w;
    // X ≥ (Z + √λ)², so P(X ≤ crit) ≤ Φ(√crit − √λ): below NEGLIGIBLE
    // the tail is 1.0, as at the ends of w's interval, λ to 10²¹, where the
    // series would run past its cap.
    if normal_tails(crit.sqrt() - lambda.sqrt()).0 < NEGLIGIBLE {
        return Ok(1.0);
    }
    Ok(Family::NcChiSq { df, ncp: lambda }.tails(word, crit)?.1)
}

fn power_chisq(vm: &mut dyn Vm) -> Result<(), Error> {
    let s = &CHISQ;
    let w = s.word;
    let v = operands(vm, w, 1)?;
    let m = options(s, &v[0])?;
    let given = given(s, m)?;
    let df = match m.get("df") {
        Some(x) => match x.unboxed() {
            BundValue::Int(k, _) if x.dt() == INTEGER && *k >= 1 => *k,
            _ if x.dt() == NONE => {
                return Err(domain(w, "df", "left out; df cannot be solved for".into()));
            }
            _ => return Err(domain(w, "df", x.summary(WIDTH))),
        },
        None => return Err(domain(w, "df", "left out; df cannot be solved for".into())),
    };
    let i = the_one(s, &given)?;
    check(s, m, "w", given[0], |x| x > 0.0)?;
    check(s, m, "n", given[1], |n| n >= 1.0)?;
    check(s, m, "sig.level", given[2], probability)?;
    check(s, m, "power", given[3], probability)?;
    let mut p: Vec<f64> = given.iter().map(|x| x.unwrap_or(f64::NAN)).collect();
    let interval = match i {
        0 => (1e-7, 1e7, Extend::UpX),
        1 => (1.0, 1e7, Extend::UpX),
        _ => (SIG.0, SIG.1, Extend::UpX),
    };
    let k = df as f64;
    complete(s, &mut p, i, 3, &|w, q| chisq_power(w, q, k), interval)?;
    vm.push(answer(
        s,
        &p,
        vec![
            ("df", BundValue::int(df)),
            (
                "method",
                BundValue::str("Chi-squared test power calculation (Cohen's w)"),
            ),
        ],
    ));
    Ok(())
}

/// Register §S4's words: fixed, `opts -- MAP`, each with its `.` form.
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr) => {
            r.register_native($name, $f, StackEffect::fixed(1, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, 1, 0);
        };
    }
    word!("power.t", power_t);
    word!("power.prop", power_prop);
    word!("power.anova", power_anova);
    word!("power.chisq", power_chisq);
}

/// The `power.` words with their forms.
pub const WORDS: [&str; 8] = [
    "power.t",
    "power.t.",
    "power.prop",
    "power.prop.",
    "power.anova",
    "power.anova.",
    "power.chisq",
    "power.chisq.",
];

#[cfg(test)]
mod tests;
