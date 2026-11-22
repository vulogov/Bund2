//! RFC-0019 §S2: the noncentral t, χ² and F, each a mixture over k = 0, 1,
//! 2, … with weights of a Poisson kind.
//!
//! - **χ² and F** mix their central family's tails, at df + 2k and at
//!   df1/2 + k, with Poisson(ncp/2) weights.
//! - **The t**, for t ≥ 0, is §S2's two interleaved series. Writing
//!   z = δ²/2, its pⱼ is e^(−z)·zʲ/j! and its |qⱼ| is e^(−z)·z^(j+½)/Γ(j + 3/2),
//!   so the two are one sequence wₖ = e^(−z)·z^(k/2)/Γ(k/2 + 1) over k = 2j
//!   and k = 2j + 1, each against I_x(k/2 + ½, ν/2). The odd terms take δ's
//!   sign. A negative t is reflected: P(T ≤ t; ν, δ) = P(T ≥ −t; ν, −δ).
//!
//! **Bund2's own series** (§S2): each sum starts at the weights' mode and
//! goes outward, so a large ncp does not underflow its first term, and
//! stops where what is left, bounded above, is below 10⁻¹⁵ of what it has
//! summed. The bounds are rigorous. The weights are log-concave in k, so
//! once a step's ratio is below 1 the weight beyond it is at most a
//! geometric series in that ratio. A lower tail's terms fall as k rises
//! and an upper tail's rise, so each is bounded by the term at hand on the
//! side where it shrinks and by 1 on the other. A density's terms are
//! log-concave too, so the same geometric bound holds on the terms
//! themselves once they fall.
//!
//! **D39.** More than 10⁶ terms is E111. Where the weights' spread alone
//! would need more (two standard deviations, 2√mean terms, already hold
//! more than a 10⁻¹⁵ share of the weight), the refusal comes before the
//! work.

use bund2_api::Error;

use super::{gamma_lpdf, lbeta, normal_tails, xlny};
use crate::biomath::refuse::Refusal;
use crate::biomath::special::{beta_both, eti, gamma_both, lgamma};

/// The most terms a noncentral series sums (§S2, D39).
const SERIES_CAP: u64 = 1_000_000;

/// Where a series stops: what is left, bounded, below this share of the sum.
const SERIES_TOL: f64 = 1e-15;

/// A mixture's weights: ln wₖ, each term's sign and constant factor, and
/// the k the sum starts from.
struct Weights {
    lw: Box<dyn Fn(f64) -> f64>,
    sign: Box<dyn Fn(f64) -> f64>,
    start: f64,
}

fn too_many(word: &str) -> Error {
    Refusal::Cap {
        what: "a noncentral series' terms".into(),
        cap: SERIES_CAP,
    }
    .error(word)
}

/// E111 before the work when 2√mean terms already pass the cap; the mean
/// is k's, under the weights.
fn spread(word: &str, mean: f64) -> Result<(), Error> {
    if !mean.is_finite() || 2.0 * mean.sqrt() > SERIES_CAP as f64 {
        Err(too_many(word))
    } else {
        Ok(())
    }
}

/// Poisson(μ) weights.
fn poisson(word: &str, mu: f64) -> Result<Weights, Error> {
    spread(word, mu)?;
    let ln_mu = mu.ln();
    Ok(Weights {
        lw: Box::new(move |k| xlny(k, ln_mu) - mu - lgamma(k + 1.0)),
        sign: Box::new(|_| 1.0),
        start: mu.floor(),
    })
}

/// The t's weights over k, at z = δ²/2, the odd terms carrying δ's sign,
/// and every term §S2's ½.
fn half_poisson(word: &str, delta: f64) -> Result<Weights, Error> {
    let z = delta * delta / 2.0;
    // k/2 is Poisson(z)'s kind, so k's mean is about 2z.
    spread(word, 2.0 * z)?;
    let ln_z = z.ln();
    let odd = if delta < 0.0 { -0.5 } else { 0.5 };
    Ok(Weights {
        lw: Box::new(move |k| xlny(k / 2.0, ln_z) - z - lgamma(k / 2.0 + 1.0)),
        sign: Box::new(move |k| if k % 2.0 == 1.0 { odd } else { 0.5 }),
        start: (2.0 * z).floor(),
    })
}

/// The weight beyond a term of weight w whose next step changes ln w by
/// `step`: a geometric series once the ratio is below 1, unbounded before.
fn left(w: f64, step: f64) -> f64 {
    if w == 0.0 {
        return 0.0;
    }
    let r = step.exp();
    if r < 1.0 {
        w * r / (1.0 - r)
    } else {
        f64::INFINITY
    }
}

/// Σ sign·w·(lower, upper) over k, for terms whose lower part falls and
/// upper part rises as k rises, each in [0, 1].
fn tails_of(
    word: &str,
    m: &Weights,
    mut term: impl FnMut(f64) -> Result<(f64, f64), Error>,
) -> Result<(f64, f64), Error> {
    let (mut lo, mut hi) = (0.0_f64, 0.0_f64);
    let mut n = 0_u64;
    let mut count = || {
        n += 1;
        if n > SERIES_CAP {
            Err(too_many(word))
        } else {
            Ok(())
        }
    };
    // Up from the mode: a lower part is at most the one at hand, an upper
    // part at most 1.
    let mut k = m.start;
    loop {
        count()?;
        let w = (m.lw)(k).exp();
        let (a, b) = if w == 0.0 { (0.0, 0.0) } else { term(k)? };
        let s = (m.sign)(k);
        lo += s * w * a;
        hi += s * w * b;
        let rest = left(w, (m.lw)(k + 1.0) - (m.lw)(k));
        if rest * a <= SERIES_TOL * lo.abs() && rest <= SERIES_TOL * hi.abs() {
            break;
        }
        k += 1.0;
    }
    // Down from below it, to 0 at most: the other way round.
    let mut k = m.start;
    while k > 0.0 {
        k -= 1.0;
        count()?;
        let w = (m.lw)(k).exp();
        let (a, b) = if w == 0.0 { (0.0, 0.0) } else { term(k)? };
        let s = (m.sign)(k);
        lo += s * w * a;
        hi += s * w * b;
        if k == 0.0 {
            break;
        }
        let rest = left(w, (m.lw)(k - 1.0) - (m.lw)(k));
        if rest <= SERIES_TOL * lo.abs() && rest * b <= SERIES_TOL * hi.abs() {
            break;
        }
    }
    Ok((lo, hi))
}

/// A signed sum of exponentials, kept as exp(top)·s so that terms far
/// below f64's range still add.
struct LogSum {
    top: f64,
    s: f64,
}

impl LogSum {
    fn add(&mut self, sign: f64, l: f64) {
        if l == f64::NEG_INFINITY {
            return;
        }
        if l > self.top {
            self.s *= (self.top - l).exp();
            self.top = l;
        }
        self.s += sign * (l - self.top).exp();
    }

    /// Whether a remainder of ln `rest` is within the tolerance of the sum.
    fn holds(&self, rest: f64) -> bool {
        rest == f64::NEG_INFINITY || (rest - self.top).exp() <= SERIES_TOL * self.s.abs()
    }

    fn ln(&self) -> f64 {
        if self.s > 0.0 {
            self.top + self.s.ln()
        } else {
            f64::NEG_INFINITY
        }
    }
}

/// ln Σ sign·exp(ln wₖ + lg(k)), lg(k) being term k's log density.
fn lpdf_of(word: &str, m: &Weights, lg: impl Fn(f64) -> f64) -> Result<f64, Error> {
    let mut sum = LogSum {
        top: f64::NEG_INFINITY,
        s: 0.0,
    };
    let mut n = 0_u64;
    let at = |k: f64| {
        let l = (m.lw)(k) + lg(k);
        if l.is_nan() { f64::NEG_INFINITY } else { l }
    };
    // Each direction stops once its terms fall, and the geometric series in
    // the latest ratio, which only falls further, is within the tolerance.
    for up in [true, false] {
        let mut k = m.start;
        let mut prev = f64::NEG_INFINITY;
        if !up {
            if k == 0.0 {
                break;
            }
            prev = at(k);
            k -= 1.0;
        }
        loop {
            n += 1;
            if n > SERIES_CAP {
                return Err(too_many(word));
            }
            let l = at(k);
            if l == f64::INFINITY {
                return Ok(f64::INFINITY);
            }
            sum.add((m.sign)(k), l);
            if up && l == f64::NEG_INFINITY {
                // A zero weight, or a density past a pole's other side: so
                // is every term above it.
                break;
            }
            if l.is_finite() && prev.is_finite() && l < prev {
                let r = (l - prev).exp();
                if sum.holds(l + (r / (1.0 - r)).ln()) {
                    break;
                }
            }
            if !up && k == 0.0 {
                break;
            }
            prev = l;
            k = if up { k + 1.0 } else { k - 1.0 };
        }
    }
    Ok(sum.ln())
}

fn beta(word: &str, x: f64, a: f64, b: f64) -> Result<(f64, f64), Error> {
    beta_both(x, a, b).map_err(|cap| eti(word, "the incomplete beta's continued fraction", cap))
}

/// (I_x(a, b), 1 − I_x(a, b)) given x and 1 − x, each computed directly,
/// run on whichever is the smaller so that neither is 1 less a rounding.
fn beta_pair(word: &str, x: f64, xc: f64, a: f64, b: f64) -> Result<(f64, f64), Error> {
    if x < 0.5 {
        beta(word, x, a, b)
    } else {
        let (upper, lower) = beta(word, xc, b, a)?;
        Ok((lower, upper))
    }
}

fn clamp(t: (f64, f64)) -> (f64, f64) {
    (t.0.clamp(0.0, 1.0), t.1.clamp(0.0, 1.0))
}

/// The χ²'s (P(X ≤ x), P(X > x)), x > 0.
pub(super) fn ncchisq_tails(word: &str, df: f64, ncp: f64, x: f64) -> Result<(f64, f64), Error> {
    let m = poisson(word, ncp / 2.0)?;
    let t = tails_of(word, &m, |k| {
        gamma_both(x / 2.0, df / 2.0 + k).map_err(|cap| eti(word, "the incomplete gamma", cap))
    })?;
    Ok(clamp(t))
}

pub(super) fn ncchisq_lpdf(word: &str, df: f64, ncp: f64, x: f64) -> Result<f64, Error> {
    if x < 0.0 {
        return Ok(f64::NEG_INFINITY);
    }
    let m = poisson(word, ncp / 2.0)?;
    lpdf_of(word, &m, |k| gamma_lpdf(x, df / 2.0 + k, 0.5))
}

/// y = df1·x/(df1·x + df2) and 1 − y, each directly.
fn f_args(df1: f64, df2: f64, x: f64) -> (f64, f64) {
    let d = df1 * x;
    if d.is_infinite() {
        (1.0, 0.0)
    } else {
        (d / (d + df2), df2 / (d + df2))
    }
}

/// The F's tails, x > 0.
pub(super) fn ncf_tails(
    word: &str,
    df1: f64,
    df2: f64,
    ncp: f64,
    x: f64,
) -> Result<(f64, f64), Error> {
    let m = poisson(word, ncp / 2.0)?;
    let (y, yc) = f_args(df1, df2, x);
    let t = tails_of(word, &m, |k| {
        beta_pair(word, y, yc, df1 / 2.0 + k, df2 / 2.0)
    })?;
    Ok(clamp(t))
}

pub(super) fn ncf_lpdf(word: &str, df1: f64, df2: f64, ncp: f64, x: f64) -> Result<f64, Error> {
    if x < 0.0 {
        return Ok(f64::NEG_INFINITY);
    }
    let m = poisson(word, ncp / 2.0)?;
    // ln(df1·x + df2), without forming df1·x past f64's range.
    let d = if x > 0.0 && df1 * x > df2 {
        df1.ln() + x.ln() + (df2 / (df1 * x)).ln_1p()
    } else {
        (df1 * x + df2).ln()
    };
    let (l1, l2, lx) = (df1.ln(), df2.ln(), x.ln());
    let b = df2 / 2.0;
    lpdf_of(word, &m, |k| {
        let a = df1 / 2.0 + k;
        a * l1 + xlny(a - 1.0, lx) + b * l2 - (a + b) * d - lbeta(a, b)
    })
}

/// t ≥ 0 and δ, after the reflection.
fn reflect(t: f64, delta: f64) -> (f64, f64, bool) {
    if t < 0.0 {
        (-t, -delta, true)
    } else {
        (t, delta, false)
    }
}

/// The t's tails.
pub(super) fn nct_tails(word: &str, df: f64, ncp: f64, t: f64) -> Result<(f64, f64), Error> {
    let (t, delta, flipped) = reflect(t, ncp);
    let m = half_poisson(word, delta)?;
    let s = t / df.sqrt();
    let q = s * s;
    // x = t²/(t² + ν) and 1 − x, each directly.
    let (x, xc) = if q.is_infinite() {
        (1.0, 0.0)
    } else {
        (q / (1.0 + q), 1.0 / (1.0 + q))
    };
    let (lo, hi) = tails_of(word, &m, |k| {
        beta_pair(word, x, xc, k / 2.0 + 0.5, df / 2.0)
    })?;
    // Φ(−δ), the lower tail's constant.
    let lo = normal_tails(delta).1 + lo;
    let (lo, hi) = clamp((lo, hi));
    Ok(if flipped { (hi, lo) } else { (lo, hi) })
}

pub(super) fn nct_lpdf(word: &str, df: f64, ncp: f64, t: f64) -> Result<f64, Error> {
    let (t, delta, _) = reflect(t, ncp);
    let m = half_poisson(word, delta)?;
    // ln(t² + ν), without squaring past f64's range.
    let l = if t > 1e150 {
        2.0 * t.ln() + (df / (t * t)).ln_1p()
    } else {
        (t * t + df).ln()
    };
    let (lt, ln_df, b) = (t.ln(), df.ln(), df / 2.0);
    let ln2 = std::f64::consts::LN_2;
    // Term k is I_x(a, ν/2)'s derivative in t, a = k/2 + ½:
    // t^k·ν^(ν/2)·2 / (B(a, ν/2)·(t² + ν)^(a + ν/2)).
    lpdf_of(word, &m, |k| {
        let a = k / 2.0 + 0.5;
        xlny(k, lt) + b * ln_df + ln2 - lbeta(a, b) - (a + b) * l
    })
}
