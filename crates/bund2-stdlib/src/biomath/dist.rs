//! RFC-0015 §S7: distributions, as MAPs (D250 B2), and the words that ask
//! them for a density, a tail, a quantile, a moment or a draw.
//!
//! **A distribution is a MAP** with a `dist` key naming the family and one
//! key per parameter (§S2). The keys are R's argument names, so
//! `1.0 2.0 dist.normal` is `{ dist=normal :: mean=1.0 :: sd=2.0 }`; the
//! hypergeometric's are §S7's own `N`, `K` and `n`, since R's `m`, `n` and
//! `k` would name different things. A MAP built by hand is read the same
//! way, and checked the same way, on every call.
//!
//! **The parameterisations are R's** (§S7): the geometric counts failures
//! before the first success, the negative binomial failures before the
//! `size`-th, the gamma takes a rate.
//!
//! **What is `statrs`'s and what is Bund2's.** `statrs` 0.18 checks every
//! family's parameters, and its refusal is E104's text; it draws for the
//! continuous families, the Poisson, the geometric and the Bernoulli. The
//! densities, tails, quantiles and moments are Bund2's own, over §S4's
//! functions, because `statrs`'s ride on the incomplete beta and gamma §S4's
//! note of 2026-11-14 replaced (unbounded for a large shape, unconverged
//! after 140 terms) and, for the normal and log-normal, on its `erfc`, off by
//! about 1e-10. §S7's note of 2026-11-15 records each change:
//!
//! - the tails are computed as the pair (P(X ≤ x), P(X > x)), each on the
//!   side the series or continued fraction gives directly, so an upper tail
//!   is not 1 less something close to 1;
//! - every continuous quantile is polished by bracketed Newton until
//!   |cdf(x) − p| ≤ 10⁻¹², 100 steps at most, E110 beyond; a discrete one is
//!   the smallest x with cdf(x) ≥ p, R's left-continuity fuzz included;
//! - the binomial and the hypergeometric draw with `rand_distr` (`statrs`'s
//!   hypergeometric draws one unit at a time, O(n) per draw, D39), the
//!   multinomial by a chain of binomials, and the negative binomial and
//!   the Dirichlet by their gamma mixtures here, since `statrs`'s Dirichlet
//!   returns its gamma draws without dividing by their sum.
//!
//! **No NaN or ±∞ [B12].** A NaN or ±∞ operand is E101; a density, moment
//! or draw that is not finite is E114; a quantile at an unbounded end of the
//! support is E114 too.

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST, MAP, METRICS};
use rand::distributions::Distribution as _;
use rand::rngs::StdRng;
use statrs::distribution as sd;
use statrs::function::erf::erfc_inv;

use super::operand::{WIDTH, domain, finite, operands, real};
use super::refuse::Refusal;
use super::rng::with_rng;
use super::special::{beta_both, eti, gamma_both, lgamma};

/// The most draws one `dist.sample*` makes (§S7, D39).
const DRAW_CAP: u64 = 100_000_000;

/// The most steps a continuous quantile's polish takes (§S7).
const POLISH_CAP: u64 = 100;

/// How near cdf(x) the polish must bring p.
const POLISH_TOL: f64 = 1e-12;

/// The most doublings a quantile's bracket or a discrete search makes.
const SEARCH_CAP: u64 = 2_100;

/// The most terms a hypergeometric tail sums (D39).
const SUM_CAP: u64 = 100_000_000;

/// The largest Poisson mean a draw is taken at, 2⁵³. Past it a draw is no
/// longer an integer a FLOAT holds exactly, and from about 1.8·10¹⁹ `statrs`'s
/// sampler casts its candidate to a saturating u64 for ln n!, so its
/// acceptance test is no longer the one it was derived for. Below the cap the
/// sampler was tried at means up to 10¹⁵ and answered 2 000 draws in
/// milliseconds with the right mean.
const LAMBDA_CAP: f64 = 9_007_199_254_740_992.0;

// What a parameter is.

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// A finite number; an INTEGER converts (B15).
    Real,
    /// A non-negative INTEGER.
    Count,
    /// A LIST (or METRICS) of finite numbers.
    Reals,
}

use Kind::{Count, Real, Reals};

/// Each family, its parameters in the constructor's operand order, deepest
/// first.
const FAMILIES: [(&str, &[(&str, Kind)]); 17] = [
    ("normal", &[("mean", Real), ("sd", Real)]),
    ("lognormal", &[("meanlog", Real), ("sdlog", Real)]),
    ("uniform", &[("min", Real), ("max", Real)]),
    ("exponential", &[("rate", Real)]),
    ("gamma", &[("shape", Real), ("rate", Real)]),
    ("beta", &[("shape1", Real), ("shape2", Real)]),
    ("chisq", &[("df", Real)]),
    ("t", &[("df", Real)]),
    ("f", &[("df1", Real), ("df2", Real)]),
    ("binomial", &[("size", Count), ("prob", Real)]),
    ("poisson", &[("lambda", Real)]),
    ("negbinomial", &[("size", Real), ("prob", Real)]),
    (
        "hypergeometric",
        &[("N", Count), ("K", Count), ("n", Count)],
    ),
    ("geometric", &[("prob", Real)]),
    ("bernoulli", &[("prob", Real)]),
    ("multinomial", &[("size", Count), ("prob", Reals)]),
    ("dirichlet", &[("alpha", Reals)]),
];

fn keys_of(name: &str) -> &'static [(&'static str, Kind)] {
    FAMILIES
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(&[], |(_, k)| k)
}

/// One parameter's value.
#[derive(Clone, Debug)]
enum Param {
    Real(f64),
    Count(u64),
    Reals(Vec<f64>),
}

impl Param {
    fn value(&self) -> BundValue {
        match self {
            Param::Real(x) => BundValue::float(*x),
            Param::Count(n) => BundValue::int(i64::try_from(*n).unwrap_or(i64::MAX)),
            Param::Reals(xs) => super::operand::list(xs.clone()),
        }
    }
}

/// A distribution, read from its MAP and checked.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Family {
    Normal { mean: f64, sd: f64 },
    LogNormal { meanlog: f64, sdlog: f64 },
    Uniform { min: f64, max: f64 },
    Exponential { rate: f64 },
    Gamma { shape: f64, rate: f64 },
    Beta { a: f64, b: f64 },
    ChiSq { df: f64 },
    T { df: f64 },
    F { df1: f64, df2: f64 },
    Binomial { size: u64, prob: f64 },
    Poisson { lambda: f64 },
    NegBinomial { size: f64, prob: f64 },
    Hypergeometric { pop: u64, succ: u64, draws: u64 },
    Geometric { prob: f64 },
    Bernoulli { prob: f64 },
    Multinomial { size: u64, prob: Vec<f64> },
    Dirichlet { alpha: Vec<f64> },
}

/// One draw's work, built once per `dist.sample*`.
type Draw = Box<dyn Fn(&mut StdRng) -> Result<BundValue, Refusal>>;

/// x·ln v, taken as 0 when x is 0 whatever v is: 0·ln 0 is 0 in a mass.
fn xlny(x: f64, lnv: f64) -> f64 {
    if x == 0.0 { 0.0 } else { x * lnv }
}

fn lchoose(n: f64, k: f64) -> f64 {
    lgamma(n + 1.0) - lgamma(k + 1.0) - lgamma(n - k + 1.0)
}

fn lbeta(a: f64, b: f64) -> f64 {
    lgamma(a) + lgamma(b) - lgamma(a + b)
}

/// The standard normal's quantile, before the polish.
fn z_of(p: f64) -> f64 {
    -std::f64::consts::SQRT_2 * erfc_inv(2.0 * p)
}

/// The normal tails of z, from `libm`'s `erfc` (§S4).
fn normal_tails(z: f64) -> (f64, f64) {
    let r = std::f64::consts::FRAC_1_SQRT_2;
    (0.5 * libm::erfc(-z * r), 0.5 * libm::erfc(z * r))
}

/// A finite draw, or E114.
fn drawn(name: &str, x: f64) -> Result<BundValue, Refusal> {
    if x.is_finite() {
        Ok(BundValue::float(x))
    } else {
        Err(Refusal::Overflow {
            expression: format!("a {name} draw"),
        })
    }
}

fn poisson_draw(g: &mut StdRng, lambda: f64) -> Result<f64, Refusal> {
    if lambda == 0.0 {
        return Ok(0.0);
    }
    if lambda > LAMBDA_CAP {
        return Err(Refusal::Cap {
            what: format!("a Poisson draw's mean of {lambda}"),
            cap: LAMBDA_CAP as u64,
        });
    }
    let d = sd::Poisson::new(lambda).map_err(|e| Refusal::Parameters {
        family: "poisson".into(),
        message: e.to_string(),
    })?;
    Ok(d.sample(g))
}

/// p's from a non-negative weight vector, normalised as R's `dmultinom` does.
fn normalised(w: &[f64]) -> Vec<f64> {
    let s: f64 = w.iter().sum();
    w.iter().map(|x| x / s).collect()
}

impl Family {
    fn build(name: &str, ps: &[Param]) -> Option<Family> {
        use Param as P;
        Some(match (name, ps) {
            ("normal", [P::Real(mean), P::Real(sd)]) => Family::Normal {
                mean: *mean,
                sd: *sd,
            },
            ("lognormal", [P::Real(meanlog), P::Real(sdlog)]) => Family::LogNormal {
                meanlog: *meanlog,
                sdlog: *sdlog,
            },
            ("uniform", [P::Real(min), P::Real(max)]) => Family::Uniform {
                min: *min,
                max: *max,
            },
            ("exponential", [P::Real(rate)]) => Family::Exponential { rate: *rate },
            ("gamma", [P::Real(shape), P::Real(rate)]) => Family::Gamma {
                shape: *shape,
                rate: *rate,
            },
            ("beta", [P::Real(a), P::Real(b)]) => Family::Beta { a: *a, b: *b },
            ("chisq", [P::Real(df)]) => Family::ChiSq { df: *df },
            ("t", [P::Real(df)]) => Family::T { df: *df },
            ("f", [P::Real(df1), P::Real(df2)]) => Family::F {
                df1: *df1,
                df2: *df2,
            },
            ("binomial", [P::Count(size), P::Real(prob)]) => Family::Binomial {
                size: *size,
                prob: *prob,
            },
            ("poisson", [P::Real(lambda)]) => Family::Poisson { lambda: *lambda },
            ("negbinomial", [P::Real(size), P::Real(prob)]) => Family::NegBinomial {
                size: *size,
                prob: *prob,
            },
            ("hypergeometric", [P::Count(pop), P::Count(succ), P::Count(draws)]) => {
                Family::Hypergeometric {
                    pop: *pop,
                    succ: *succ,
                    draws: *draws,
                }
            }
            ("geometric", [P::Real(prob)]) => Family::Geometric { prob: *prob },
            ("bernoulli", [P::Real(prob)]) => Family::Bernoulli { prob: *prob },
            ("multinomial", [P::Count(size), P::Reals(prob)]) => Family::Multinomial {
                size: *size,
                prob: prob.clone(),
            },
            ("dirichlet", [P::Reals(alpha)]) => Family::Dirichlet {
                alpha: alpha.clone(),
            },
            _ => return None,
        })
    }

    fn name(&self) -> &'static str {
        match self {
            Family::Normal { .. } => "normal",
            Family::LogNormal { .. } => "lognormal",
            Family::Uniform { .. } => "uniform",
            Family::Exponential { .. } => "exponential",
            Family::Gamma { .. } => "gamma",
            Family::Beta { .. } => "beta",
            Family::ChiSq { .. } => "chisq",
            Family::T { .. } => "t",
            Family::F { .. } => "f",
            Family::Binomial { .. } => "binomial",
            Family::Poisson { .. } => "poisson",
            Family::NegBinomial { .. } => "negbinomial",
            Family::Hypergeometric { .. } => "hypergeometric",
            Family::Geometric { .. } => "geometric",
            Family::Bernoulli { .. } => "bernoulli",
            Family::Multinomial { .. } => "multinomial",
            Family::Dirichlet { .. } => "dirichlet",
        }
    }

    fn multivariate(&self) -> bool {
        matches!(self, Family::Multinomial { .. } | Family::Dirichlet { .. })
    }

    fn discrete(&self) -> bool {
        matches!(
            self,
            Family::Binomial { .. }
                | Family::Poisson { .. }
                | Family::NegBinomial { .. }
                | Family::Hypergeometric { .. }
                | Family::Geometric { .. }
                | Family::Bernoulli { .. }
        )
    }

    /// The family's sampler, which is also the parameter check: `statrs`
    /// refuses what it would refuse to construct, and its message is E104's.
    fn sampler(&self) -> Result<Draw, String> {
        let s = |e: &dyn std::fmt::Display| e.to_string();
        let name = self.name();
        Ok(match self.clone() {
            Family::Normal { mean, sd } => {
                let d = sd::Normal::new(mean, sd).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g)))
            }
            Family::LogNormal { meanlog, sdlog } => {
                let d = sd::LogNormal::new(meanlog, sdlog).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g)))
            }
            Family::Uniform { min, max } => {
                let d = sd::Uniform::new(min, max).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g)))
            }
            Family::Exponential { rate } => {
                let d = sd::Exp::new(rate).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g)))
            }
            Family::Gamma { shape, rate } => {
                let d = sd::Gamma::new(shape, rate).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g)))
            }
            Family::Beta { a, b } => {
                let d = sd::Beta::new(a, b).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g)))
            }
            Family::ChiSq { df } => {
                let d = sd::ChiSquared::new(df).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g)))
            }
            Family::T { df } => {
                let d = sd::StudentsT::new(0.0, 1.0, df).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g)))
            }
            Family::F { df1, df2 } => {
                let d = sd::FisherSnedecor::new(df1, df2).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g)))
            }
            Family::Binomial { size, prob } => {
                sd::Binomial::new(prob, size).map_err(|e| s(&e))?;
                let d = rand_distr::Binomial::new(size, prob).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g) as f64))
            }
            Family::Poisson { lambda } => {
                sd::Poisson::new(lambda).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, poisson_draw(g, lambda)?))
            }
            Family::NegBinomial { size, prob } => {
                sd::NegativeBinomial::new(size, prob).map_err(|e| s(&e))?;
                if prob == 0.0 {
                    // statrs takes p in [0, 1]; at 0 no success ever comes,
                    // so there is no distribution, and R refuses it too.
                    return Err("p is 0, outside R's (0, 1]".into());
                }
                // A Poisson draw at a Gamma(size, p / (1 − p)) mean.
                let mix = if size == 0.0 || prob == 1.0 {
                    None
                } else {
                    Some(sd::Gamma::new(size, prob / (1.0 - prob)).map_err(|e| s(&e))?)
                };
                Box::new(move |g| match &mix {
                    None => Ok(BundValue::float(0.0)),
                    Some(m) => {
                        let lambda = m.sample(g);
                        drawn(name, poisson_draw(g, lambda)?)
                    }
                })
            }
            Family::Hypergeometric { pop, succ, draws } => {
                sd::Hypergeometric::new(pop, succ, draws).map_err(|e| s(&e))?;
                let d = rand_distr::Hypergeometric::new(pop, succ, draws).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g) as f64))
            }
            Family::Geometric { prob } => {
                // statrs counts trials, R failures: one less (§S7).
                let d = sd::Geometric::new(prob).map_err(|e| s(&e))?;
                Box::new(move |g| {
                    let trials: u64 = d.sample(g);
                    drawn(name, trials.saturating_sub(1) as f64)
                })
            }
            Family::Bernoulli { prob } => {
                let d = sd::Bernoulli::new(prob).map_err(|e| s(&e))?;
                Box::new(move |g| drawn(name, d.sample(g)))
            }
            Family::Multinomial { size, prob } => {
                sd::Multinomial::new(prob.clone(), size).map_err(|e| s(&e))?;
                let q = normalised(&prob);
                Box::new(move |g| {
                    // A binomial for each cell of what the cells before it
                    // left: O(cells) per draw where statrs's is O(size).
                    let (mut left, mut mass) = (size, 1.0);
                    let mut out = Vec::with_capacity(q.len());
                    for (i, qi) in q.iter().enumerate() {
                        let x = if i + 1 == q.len() || left == 0 {
                            left
                        } else {
                            let p = if mass > 0.0 {
                                (qi / mass).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            let b = rand_distr::Binomial::new(left, p).map_err(|e| {
                                Refusal::Parameters {
                                    family: name.into(),
                                    message: e.to_string(),
                                }
                            })?;
                            b.sample(g)
                        };
                        out.push(x as f64);
                        left -= x;
                        mass -= qi;
                    }
                    Ok(super::operand::list(out))
                })
            }
            Family::Dirichlet { alpha } => {
                sd::Dirichlet::new(alpha.clone()).map_err(|e| s(&e))?;
                let gammas = alpha
                    .iter()
                    .map(|a| sd::Gamma::new(*a, 1.0).map_err(|e| s(&e)))
                    .collect::<Result<Vec<_>, _>>()?;
                Box::new(move |g| {
                    let xs: Vec<f64> = gammas.iter().map(|d| d.sample(g)).collect();
                    let total: f64 = xs.iter().sum();
                    let ys: Vec<f64> = xs.iter().map(|x| x / total).collect();
                    if total > 0.0 && ys.iter().all(|y| y.is_finite()) {
                        Ok(super::operand::list(ys))
                    } else {
                        Err(Refusal::Overflow {
                            expression: format!("a {name} draw"),
                        })
                    }
                })
            }
        })
    }

    /// ln f(x): −∞ outside the support, +∞ at a density's pole.
    pub(crate) fn lpdf(&self, x: f64) -> f64 {
        const NEG: f64 = f64::NEG_INFINITY;
        let ln_2pi_half = 0.5 * (2.0 * std::f64::consts::PI).ln();
        // A discrete family's mass is 0 off the integers.
        if self.discrete() && x.fract() != 0.0 {
            return NEG;
        }
        match *self {
            Family::Normal { mean, sd } => {
                let z = (x - mean) / sd;
                -0.5 * z * z - sd.ln() - ln_2pi_half
            }
            Family::LogNormal { meanlog, sdlog } => {
                if x <= 0.0 {
                    return NEG;
                }
                let z = (x.ln() - meanlog) / sdlog;
                -0.5 * z * z - sdlog.ln() - ln_2pi_half - x.ln()
            }
            Family::Uniform { min, max } => {
                if x < min || x > max {
                    NEG
                } else {
                    -(max - min).ln()
                }
            }
            Family::Exponential { rate } => {
                if x < 0.0 {
                    NEG
                } else {
                    rate.ln() - rate * x
                }
            }
            Family::Gamma { shape, rate } => gamma_lpdf(x, shape, rate),
            Family::ChiSq { df } => gamma_lpdf(x, df / 2.0, 0.5),
            Family::Beta { a, b } => {
                if !(0.0..=1.0).contains(&x) {
                    return NEG;
                }
                xlny(a - 1.0, x.ln()) + xlny(b - 1.0, (-x).ln_1p()) - lbeta(a, b)
            }
            Family::T { df } => {
                let r = x / df.sqrt();
                // ln(1 + x²/ν), without squaring past f64's range.
                let l = if r.abs() > 1e150 {
                    2.0 * r.abs().ln()
                } else {
                    (r * r).ln_1p()
                };
                lgamma((df + 1.0) / 2.0)
                    - lgamma(df / 2.0)
                    - 0.5 * (df * std::f64::consts::PI).ln()
                    - (df + 1.0) / 2.0 * l
            }
            Family::F { df1, df2 } => {
                if x < 0.0 {
                    return NEG;
                }
                if x == 0.0 {
                    return if df1 < 2.0 {
                        f64::INFINITY
                    } else if df1 == 2.0 {
                        0.0
                    } else {
                        NEG
                    };
                }
                0.5 * df1 * (df1 / df2).ln() + (df1 / 2.0 - 1.0) * x.ln()
                    - (df1 + df2) / 2.0 * (df1 * x / df2).ln_1p()
                    - lbeta(df1 / 2.0, df2 / 2.0)
            }
            Family::Binomial { size, prob } => {
                let n = size as f64;
                if x < 0.0 || x > n {
                    return NEG;
                }
                lchoose(n, x) + xlny(x, prob.ln()) + xlny(n - x, (-prob).ln_1p())
            }
            Family::Poisson { lambda } => {
                if x < 0.0 {
                    return NEG;
                }
                x * lambda.ln() - lambda - lgamma(x + 1.0)
            }
            Family::NegBinomial { size, prob } => {
                if x < 0.0 {
                    return NEG;
                }
                if size == 0.0 {
                    return if x == 0.0 { 0.0 } else { NEG };
                }
                lgamma(x + size) - lgamma(size) - lgamma(x + 1.0)
                    + xlny(size, prob.ln())
                    + xlny(x, (-prob).ln_1p())
            }
            Family::Hypergeometric { pop, succ, draws } => {
                let (nn, kk, n) = (pop as f64, succ as f64, draws as f64);
                let (lo, hi) = hyper_support(pop, succ, draws);
                if x < lo || x > hi {
                    return NEG;
                }
                lchoose(kk, x) + lchoose(nn - kk, n - x) - lchoose(nn, n)
            }
            Family::Geometric { prob } => {
                if x < 0.0 {
                    return NEG;
                }
                prob.ln() + xlny(x, (-prob).ln_1p())
            }
            Family::Bernoulli { prob } => {
                if x == 0.0 {
                    (-prob).ln_1p()
                } else if x == 1.0 {
                    prob.ln()
                } else {
                    NEG
                }
            }
            Family::Multinomial { .. } | Family::Dirichlet { .. } => NEG,
        }
    }

    /// (P(X ≤ x), P(X > x)).
    pub(crate) fn tails(&self, word: &str, x: f64) -> Result<(f64, f64), Error> {
        let beta = |x: f64, a: f64, b: f64| {
            beta_both(x, a, b)
                .map_err(|cap| eti(word, "the incomplete beta's continued fraction", cap))
        };
        let gamma =
            |x: f64, a: f64| gamma_both(x, a).map_err(|cap| eti(word, "the incomplete gamma", cap));
        let below = (0.0, 1.0);
        let above = (1.0, 0.0);
        // A discrete family's tails step at the integers.
        let k = x.floor();
        Ok(match *self {
            Family::Normal { mean, sd } => normal_tails((x - mean) / sd),
            Family::LogNormal { meanlog, sdlog } => {
                if x <= 0.0 {
                    below
                } else {
                    normal_tails((x.ln() - meanlog) / sdlog)
                }
            }
            Family::Uniform { min, max } => {
                if x <= min {
                    below
                } else if x >= max {
                    above
                } else {
                    ((x - min) / (max - min), (max - x) / (max - min))
                }
            }
            Family::Exponential { rate } => {
                if x <= 0.0 {
                    below
                } else {
                    (-(-rate * x).exp_m1(), (-rate * x).exp())
                }
            }
            Family::Gamma { shape, rate } => {
                if x <= 0.0 {
                    below
                } else {
                    gamma(rate * x, shape)?
                }
            }
            Family::ChiSq { df } => {
                if x <= 0.0 {
                    below
                } else {
                    gamma(x / 2.0, df / 2.0)?
                }
            }
            Family::Beta { a, b } => {
                if x <= 0.0 {
                    below
                } else if x >= 1.0 {
                    above
                } else {
                    beta(x, a, b)?
                }
            }
            Family::T { df } => {
                if x == 0.0 {
                    (0.5, 0.5)
                } else {
                    let r = x / df.sqrt();
                    let w = 1.0 / (1.0 + r * r);
                    let tail = beta(w, df / 2.0, 0.5)?.0 / 2.0;
                    if x < 0.0 {
                        (tail, 1.0 - tail)
                    } else {
                        (1.0 - tail, tail)
                    }
                }
            }
            Family::F { df1, df2 } => {
                if x <= 0.0 {
                    below
                } else {
                    let w = df1 * x / (df1 * x + df2);
                    beta(w, df1 / 2.0, df2 / 2.0)?
                }
            }
            Family::Binomial { size, prob } => {
                if k < 0.0 {
                    below
                } else if k >= size as f64 {
                    above
                } else {
                    // P(X > k) = I_p(k + 1, n − k).
                    let (upper, lower) = beta(prob, k + 1.0, size as f64 - k)?;
                    (lower, upper)
                }
            }
            Family::Poisson { lambda } => {
                if k < 0.0 {
                    below
                } else {
                    // P(X ≤ k) = Q(k + 1, λ).
                    let (p, q) = gamma(lambda, k + 1.0)?;
                    (q, p)
                }
            }
            Family::NegBinomial { size, prob } => {
                if k < 0.0 {
                    below
                } else if size == 0.0 || prob == 1.0 {
                    above
                } else {
                    // P(X ≤ k) = I_p(size, k + 1).
                    beta(prob, size, k + 1.0)?
                }
            }
            Family::Hypergeometric { pop, succ, draws } => hyper_tails(word, pop, succ, draws, k)?,
            Family::Geometric { prob } => {
                if k < 0.0 {
                    below
                } else {
                    let l = (k + 1.0) * (-prob).ln_1p();
                    (-l.exp_m1(), l.exp())
                }
            }
            Family::Bernoulli { prob } => {
                if k < 0.0 {
                    below
                } else if k < 1.0 {
                    (1.0 - prob, prob)
                } else {
                    above
                }
            }
            Family::Multinomial { .. } | Family::Dirichlet { .. } => {
                return Err(self.undefined(word, "cdf"));
            }
        })
    }

    fn undefined(&self, word: &str, operation: &str) -> Error {
        Refusal::Undefined {
            family: self.name().into(),
            operation: operation.into(),
        }
        .error(word)
    }

    /// The support's ends.
    fn support(&self) -> (f64, f64) {
        const INF: f64 = f64::INFINITY;
        match *self {
            Family::Normal { .. } | Family::T { .. } => (-INF, INF),
            Family::LogNormal { .. }
            | Family::Exponential { .. }
            | Family::Gamma { .. }
            | Family::ChiSq { .. }
            | Family::F { .. }
            | Family::Poisson { .. }
            | Family::Geometric { .. } => (0.0, INF),
            Family::NegBinomial { size, prob } => {
                if size == 0.0 || prob == 1.0 {
                    (0.0, 0.0)
                } else {
                    (0.0, INF)
                }
            }
            Family::Uniform { min, max } => (min, max),
            Family::Beta { .. } | Family::Bernoulli { .. } => (0.0, 1.0),
            Family::Binomial { size, .. } => (0.0, size as f64),
            Family::Hypergeometric { pop, succ, draws } => hyper_support(pop, succ, draws),
            Family::Multinomial { .. } | Family::Dirichlet { .. } => (0.0, 0.0),
        }
    }

    /// The quantile at p in [0, 1]; ±∞ at an unbounded end, for the caller
    /// to refuse.
    pub(crate) fn quantile(&self, word: &str, p: f64) -> Result<f64, Error> {
        let (lo, hi) = self.support();
        if p == 0.0 {
            return Ok(lo);
        }
        if p == 1.0 {
            return Ok(hi);
        }
        if self.discrete() {
            return self.discrete_quantile(word, p, lo, hi);
        }
        let z = z_of(p);
        let guess = match *self {
            Family::Uniform { min, max } => return Ok(min + p * (max - min)),
            Family::Exponential { rate } => return Ok(-(-p).ln_1p() / rate),
            Family::Normal { mean, sd } => mean + sd * z,
            Family::LogNormal { meanlog, sdlog } => (meanlog + sdlog * z).exp(),
            Family::Gamma { shape, rate } => gamma_guess(p, z, shape) / rate,
            Family::ChiSq { df } => gamma_guess(p, z, df / 2.0) * 2.0,
            Family::Beta { a, b } => {
                let m = a / (a + b);
                let g = m + z * (a * b / ((a + b) * (a + b) * (a + b + 1.0))).sqrt();
                if g > 0.0 && g < 1.0 { g } else { m }
            }
            Family::T { df } => {
                if df == 1.0 {
                    (std::f64::consts::PI * (p - 0.5)).tan()
                } else {
                    z + (z * z * z + z) / (4.0 * df)
                }
            }
            Family::F { df2, .. } => {
                if df2 > 2.0 {
                    df2 / (df2 - 2.0)
                } else {
                    1.0
                }
            }
            _ => return Err(self.undefined(word, "quantile")),
        };
        self.polish(word, p, guess, lo, hi)
    }

    /// Bracketed Newton on cdf(x) − p: a Newton step where it stays inside
    /// the bracket, a bisection where it does not (§S7).
    fn polish(&self, word: &str, p: f64, guess: f64, lo: f64, hi: f64) -> Result<f64, Error> {
        let cdf = |x: f64| self.tails(word, x).map(|t| t.0);
        let s = guess.abs().max(1.0);
        let (mut a, mut b) = (lo, hi);
        if b.is_infinite() {
            b = s;
            let mut n = 0;
            while cdf(b)? < p {
                b *= 2.0;
                n += 1;
                if n > SEARCH_CAP || b.is_infinite() {
                    return Err(eti(word, "the quantile's bracket", SEARCH_CAP));
                }
            }
        }
        if a.is_infinite() {
            a = -s;
            let mut n = 0;
            while cdf(a)? > p {
                a *= 2.0;
                n += 1;
                if n > SEARCH_CAP || a.is_infinite() {
                    return Err(eti(word, "the quantile's bracket", SEARCH_CAP));
                }
            }
        }
        let mid = |a: f64, b: f64| {
            if a > 0.0 && b / a > 8.0 {
                (a * b).sqrt()
            } else if a == 0.0 && b > 0.0 {
                b / 8.0
            } else {
                0.5 * (a + b)
            }
        };
        let mut x = if guess > a && guess < b {
            guess
        } else {
            mid(a, b)
        };
        for _ in 0..POLISH_CAP {
            let f = cdf(x)? - p;
            if f == 0.0 {
                return Ok(x);
            }
            if f < 0.0 {
                a = x;
            } else {
                b = x;
            }
            let d = self.lpdf(x).exp();
            let newton = x - f / d;
            let next = if d > 0.0 && d.is_finite() && newton > a && newton < b {
                newton
            } else {
                mid(a, b)
            };
            let still = (next - x).abs() > 2.0 * f64::EPSILON * x.abs().max(f64::MIN_POSITIVE);
            x = next;
            if !still || b - a <= 4.0 * f64::EPSILON * a.abs().max(b.abs()) {
                break;
            }
        }
        if (cdf(x)? - p).abs() <= POLISH_TOL {
            Ok(x)
        } else {
            Err(eti(word, "the quantile's polish", POLISH_CAP))
        }
    }

    /// The smallest integer x with cdf(x) ≥ p, with R's fuzz for the
    /// rounding of a cdf that should equal p exactly.
    fn discrete_quantile(&self, word: &str, p: f64, lo: f64, hi: f64) -> Result<f64, Error> {
        let target = p * (1.0 - 64.0 * f64::EPSILON);
        let cdf = |x: f64| self.tails(word, x).map(|t| t.0);
        let start = self
            .moments()
            .0
            .unwrap_or(lo)
            .floor()
            .clamp(lo, hi.min(9e15));
        let (mut a, mut b);
        let mut step = 1.0;
        let mut n = 0;
        let mut count = || {
            n += 1;
            if n > SEARCH_CAP {
                Err(eti(word, "the quantile's search", SEARCH_CAP))
            } else {
                Ok(())
            }
        };
        if cdf(start)? >= target {
            // Down: a is below the answer, b at or above it.
            b = start;
            loop {
                count()?;
                a = b - step;
                if a < lo {
                    a = lo - 1.0;
                    break;
                }
                if cdf(a)? < target {
                    break;
                }
                b = a;
                step *= 2.0;
            }
        } else {
            a = start;
            loop {
                count()?;
                b = a + step;
                if b >= hi {
                    b = hi;
                    break;
                }
                if cdf(b)? >= target {
                    break;
                }
                a = b;
                step *= 2.0;
            }
        }
        if b.is_infinite() {
            return Err(eti(word, "the quantile's search", SEARCH_CAP));
        }
        while b - a > 1.0 {
            count()?;
            let m = (a + (b - a) / 2.0).floor();
            if cdf(m)? >= target {
                b = m;
            } else {
                a = m;
            }
        }
        Ok(b)
    }

    /// The mean and the variance, where they exist.
    fn moments(&self) -> (Option<f64>, Option<f64>) {
        match *self {
            Family::Normal { mean, sd } => (Some(mean), Some(sd * sd)),
            Family::LogNormal { meanlog, sdlog } => {
                let s2 = sdlog * sdlog;
                (
                    Some((meanlog + s2 / 2.0).exp()),
                    Some(s2.exp_m1() * (2.0 * meanlog + s2).exp()),
                )
            }
            Family::Uniform { min, max } => {
                (Some((min + max) / 2.0), Some((max - min).powi(2) / 12.0))
            }
            Family::Exponential { rate } => (Some(1.0 / rate), Some(1.0 / (rate * rate))),
            Family::Gamma { shape, rate } => (Some(shape / rate), Some(shape / (rate * rate))),
            Family::Beta { a, b } => (
                Some(a / (a + b)),
                Some(a * b / ((a + b) * (a + b) * (a + b + 1.0))),
            ),
            Family::ChiSq { df } => (Some(df), Some(2.0 * df)),
            Family::T { df } => (
                (df > 1.0).then_some(0.0),
                (df > 2.0).then(|| df / (df - 2.0)),
            ),
            Family::F { df1, df2 } => (
                (df2 > 2.0).then(|| df2 / (df2 - 2.0)),
                (df2 > 4.0).then(|| {
                    2.0 * df2 * df2 * (df1 + df2 - 2.0)
                        / (df1 * (df2 - 2.0) * (df2 - 2.0) * (df2 - 4.0))
                }),
            ),
            Family::Binomial { size, prob } => {
                let n = size as f64;
                (Some(n * prob), Some(n * prob * (1.0 - prob)))
            }
            Family::Poisson { lambda } => (Some(lambda), Some(lambda)),
            Family::NegBinomial { size, prob } => (
                Some(size * (1.0 - prob) / prob),
                Some(size * (1.0 - prob) / (prob * prob)),
            ),
            Family::Hypergeometric { pop, succ, draws } => {
                let (nn, kk, n) = (pop as f64, succ as f64, draws as f64);
                let q = kk / nn;
                let var = if pop > 1 {
                    n * q * (1.0 - q) * (nn - n) / (nn - 1.0)
                } else {
                    0.0
                };
                (Some(if pop == 0 { 0.0 } else { n * q }), Some(var))
            }
            Family::Geometric { prob } => (
                Some((1.0 - prob) / prob),
                Some((1.0 - prob) / (prob * prob)),
            ),
            Family::Bernoulli { prob } => (Some(prob), Some(prob * (1.0 - prob))),
            Family::Multinomial { .. } | Family::Dirichlet { .. } => (None, None),
        }
    }

    /// The label E105 gives a moment that does not exist: the family and the
    /// parameter that decides it.
    fn moment_label(&self) -> String {
        match *self {
            Family::T { df } => format!("t with df = {df}"),
            Family::F { df2, .. } => format!("f with df2 = {df2}"),
            _ => self.name().into(),
        }
    }

    /// The multivariate families' ln density at a LIST.
    fn lpdf_at(&self, word: &str, xs: &[f64]) -> Result<f64, Error> {
        let k = match self {
            Family::Multinomial { prob, .. } => prob.len(),
            Family::Dirichlet { alpha } => alpha.len(),
            _ => return Err(Error::internal("lpdf_at: a univariate family")),
        };
        if xs.len() != k {
            return Err(Refusal::Disagree {
                what: "lengths".into(),
                n: xs.len(),
                m: k,
            }
            .error(word));
        }
        Ok(match self {
            Family::Multinomial { size, prob } => {
                if let Some(i) = xs.iter().position(|x| *x < 0.0 || x.fract() != 0.0) {
                    return Err(domain(word, "x", format!("{} at {}", xs[i], i + 1)));
                }
                if xs.iter().sum::<f64>() != *size as f64 {
                    return Ok(f64::NEG_INFINITY);
                }
                let q = normalised(prob);
                xs.iter()
                    .zip(&q)
                    .fold(lgamma(*size as f64 + 1.0), |acc, (x, q)| {
                        acc - lgamma(x + 1.0) + xlny(*x, q.ln())
                    })
            }
            Family::Dirichlet { alpha } => {
                let sum: f64 = xs.iter().sum();
                let off =
                    xs.iter().any(|x| *x < 0.0 || *x > 1.0) || (sum - 1.0).abs() > 1e-10 * k as f64;
                if off {
                    return Ok(f64::NEG_INFINITY);
                }
                let a0: f64 = alpha.iter().sum();
                xs.iter().zip(alpha).fold(lgamma(a0), |acc, (x, a)| {
                    acc - lgamma(*a) + xlny(a - 1.0, x.ln())
                })
            }
            _ => f64::NEG_INFINITY,
        })
    }
}

fn gamma_lpdf(x: f64, shape: f64, rate: f64) -> f64 {
    if x < 0.0 {
        return f64::NEG_INFINITY;
    }
    if x == 0.0 {
        return if shape < 1.0 {
            f64::INFINITY
        } else if shape == 1.0 {
            rate.ln()
        } else {
            f64::NEG_INFINITY
        };
    }
    shape * rate.ln() + (shape - 1.0) * x.ln() - rate * x - lgamma(shape)
}

/// A start for the gamma's quantile at rate 1: Wilson–Hilferty, or, for a
/// small shape, the lower tail's leading term P(a, x) ≈ xᵃ / Γ(a + 1).
fn gamma_guess(p: f64, z: f64, shape: f64) -> f64 {
    let wh = shape * (1.0 - 1.0 / (9.0 * shape) + z / (3.0 * shape.sqrt())).powi(3);
    if shape >= 1.0 && wh > 0.0 {
        wh
    } else {
        ((p.ln() + lgamma(shape + 1.0)) / shape).exp()
    }
}

fn hyper_support(pop: u64, succ: u64, draws: u64) -> (f64, f64) {
    let lo = draws.saturating_sub(pop.saturating_sub(succ));
    (lo as f64, draws.min(succ) as f64)
}

/// The hypergeometric's tails at k, summed from k outward over the shorter
/// side of the mode, each term from the last by the mass's ratio, stopping
/// where the terms stop counting (D39 caps the count).
fn hyper_tails(word: &str, pop: u64, succ: u64, draws: u64, k: f64) -> Result<(f64, f64), Error> {
    let (lo, hi) = hyper_support(pop, succ, draws);
    if k < lo {
        return Ok((0.0, 1.0));
    }
    if k >= hi {
        return Ok((1.0, 0.0));
    }
    let (nn, kk, n) = (pop as f64, succ as f64, draws as f64);
    let lpmf = |x: f64| lchoose(kk, x) + lchoose(nn - kk, n - x) - lchoose(nn, n);
    let mode = ((n + 1.0) * (kk + 1.0) / (nn + 2.0)).floor();
    let cap = || {
        Refusal::Cap {
            what: "a hypergeometric tail's terms".into(),
            cap: SUM_CAP,
        }
        .error(word)
    };
    if k < mode {
        // P(X ≤ k), down from k.
        let (mut x, mut term) = (k, lpmf(k).exp());
        let mut sum = term;
        let mut steps = 0;
        while x > lo && term > sum * f64::EPSILON / 4.0 {
            term *= x * (nn - kk - n + x) / ((kk - x + 1.0) * (n - x + 1.0));
            x -= 1.0;
            sum += term;
            steps += 1;
            if steps > SUM_CAP {
                return Err(cap());
            }
        }
        Ok((sum.min(1.0), 1.0 - sum.min(1.0)))
    } else {
        // P(X > k), up from k + 1.
        let (mut x, mut term) = (k + 1.0, lpmf(k + 1.0).exp());
        let mut sum = term;
        let mut steps = 0;
        while x < hi && term > sum * f64::EPSILON / 4.0 {
            term *= (kk - x) * (n - x) / ((x + 1.0) * (nn - kk - n + x + 1.0));
            x += 1.0;
            sum += term;
            steps += 1;
            if steps > SUM_CAP {
                return Err(cap());
            }
        }
        Ok((1.0 - sum.min(1.0), sum.min(1.0)))
    }
}

// Reading operands and MAPs.

/// A non-negative INTEGER operand, or E101.
fn count(word: &str, operand: &str, v: &BundValue) -> Result<u64, Error> {
    match v.unboxed() {
        BundValue::Int(i, _) if *i >= 0 => Ok(i.unsigned_abs()),
        _ => Err(domain(word, operand, v.summary(WIDTH))),
    }
}

/// A LIST or METRICS of finite numbers, or E101 for anything else.
fn reals(word: &str, operand: &str, v: &BundValue) -> Result<Vec<f64>, Error> {
    match v.dt() {
        LIST | METRICS => super::operand::whole(word, operand, v),
        _ => Err(domain(word, operand, v.summary(WIDTH))),
    }
}

fn parameters(word: &str, name: &str, message: String) -> Error {
    Refusal::Parameters {
        family: name.into(),
        message,
    }
    .error(word)
}

/// Check a family's parameters, E104 with `statrs`'s message.
fn checked(word: &str, name: &str, ps: &[Param]) -> Result<Family, Error> {
    let f = Family::build(name, ps).ok_or_else(|| {
        Error::internal(format!("{word}: {name}'s parameters had the wrong kinds"))
    })?;
    f.sampler()
        .map(drop)
        .map_err(|m| parameters(word, name, m))?;
    Ok(f)
}

/// The distribution a MAP describes: E101 if it is not one, E104 if a
/// parameter is missing, of the wrong kind, or refused.
fn decode(word: &str, v: &BundValue) -> Result<Family, Error> {
    let not_one = || domain(word, "d", v.summary(WIDTH));
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(not_one()),
    };
    let name = m
        .get("dist")
        .and_then(BundValue::as_str)
        .ok_or_else(not_one)?;
    let Some((name, keys)) = FAMILIES.iter().find(|(n, _)| *n == name) else {
        return Err(not_one());
    };
    let mut ps = Vec::with_capacity(keys.len());
    for (key, kind) in keys.iter() {
        let Some(x) = m.get(*key) else {
            return Err(parameters(word, name, format!("{key} is missing")));
        };
        let bad = |what: &str| parameters(word, name, format!("{key} is not {what}"));
        ps.push(match kind {
            Real => match x.unboxed() {
                BundValue::Float(f, _) if f.is_finite() => Param::Real(*f),
                BundValue::Int(i, _) => Param::Real(*i as f64),
                _ => return Err(bad("a finite number")),
            },
            Count => match x.unboxed() {
                BundValue::Int(i, _) if *i >= 0 => Param::Count(i.unsigned_abs()),
                _ => return Err(bad("a non-negative INTEGER")),
            },
            Reals => match reals(word, key, x) {
                Ok(xs) => Param::Reals(xs),
                Err(_) => return Err(bad("a LIST of finite numbers")),
            },
        });
    }
    checked(word, name, &ps)
}

// The words.

/// A constructor: the family's parameters, deepest first, to its MAP.
fn construct(vm: &mut dyn Vm, word: &str, name: &str) -> Result<(), Error> {
    let keys = keys_of(name);
    let v = operands(vm, word, keys.len())?;
    let mut ps = Vec::with_capacity(keys.len());
    for ((key, kind), x) in keys.iter().zip(&v) {
        ps.push(match kind {
            Real => Param::Real(real(word, key, x)?),
            Count => Param::Count(count(word, key, x)?),
            Reals => Param::Reals(reals(word, key, x)?),
        });
    }
    checked(word, name, &ps)?;
    let mut m = BTreeMap::new();
    m.insert("dist".to_string(), BundValue::str(name));
    for ((key, _), p) in keys.iter().zip(&ps) {
        m.insert((*key).to_string(), p.value());
    }
    vm.push(BundValue::map(m));
    Ok(())
}

fn num(x: f64) -> String {
    format!("{x}")
}

/// `x d` to the density's logarithm, as the word asks it.
fn ln_density(word: &str, x: &BundValue, f: &Family) -> Result<f64, Error> {
    if f.multivariate() {
        let xs = reals(word, "x", x)?;
        f.lpdf_at(word, &xs)
    } else {
        let x = real(word, "x", x)?;
        Ok(f.lpdf(x))
    }
}

fn dist_pdf(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "dist.pdf";
    let v = operands(vm, W, 2)?;
    let f = decode(W, &v[1])?;
    let l = ln_density(W, &v[0], &f)?;
    let y = finite(W, l.exp(), || {
        format!("the {} density at {}", f.name(), v[0].summary(WIDTH))
    })?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn dist_lpdf(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "dist.lpdf";
    let v = operands(vm, W, 2)?;
    let f = decode(W, &v[1])?;
    let l = ln_density(W, &v[0], &f)?;
    if l == f64::NEG_INFINITY {
        return Err(domain(W, "x", v[0].summary(WIDTH)));
    }
    let y = finite(W, l, || {
        format!("the {} density at {}", f.name(), v[0].summary(WIDTH))
    })?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn tail(vm: &mut dyn Vm, word: &str, upper: bool) -> Result<(), Error> {
    let v = operands(vm, word, 2)?;
    let f = decode(word, &v[1])?;
    if f.multivariate() {
        return Err(f.undefined(word, if upper { "sf" } else { "cdf" }));
    }
    let x = real(word, "x", &v[0])?;
    let (lower, higher) = f.tails(word, x)?;
    vm.push(BundValue::float(if upper { higher } else { lower }));
    Ok(())
}

fn dist_cdf(vm: &mut dyn Vm) -> Result<(), Error> {
    tail(vm, "dist.cdf", false)
}

fn dist_sf(vm: &mut dyn Vm) -> Result<(), Error> {
    tail(vm, "dist.sf", true)
}

fn dist_quantile(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "dist.quantile";
    let v = operands(vm, W, 2)?;
    let f = decode(W, &v[1])?;
    if f.multivariate() {
        return Err(f.undefined(W, "quantile"));
    }
    let p = real(W, "p", &v[0])?;
    if !(0.0..=1.0).contains(&p) {
        return Err(domain(W, "p", num(p)));
    }
    let x = f.quantile(W, p)?;
    let x = finite(W, x, || format!("the {} quantile at {}", f.name(), num(p)))?;
    vm.push(BundValue::float(x));
    Ok(())
}

fn moment(vm: &mut dyn Vm, word: &str, variance: bool) -> Result<(), Error> {
    let v = operands(vm, word, 1)?;
    let f = decode(word, &v[0])?;
    let (mean, var) = f.moments();
    let what = if variance { "variance" } else { "mean" };
    let Some(y) = (if variance { var } else { mean }) else {
        return Err(Refusal::Undefined {
            family: f.moment_label(),
            operation: what.into(),
        }
        .error(word));
    };
    let y = finite(word, y, || format!("the {} {what}", f.name()))?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn dist_mean(vm: &mut dyn Vm) -> Result<(), Error> {
    moment(vm, "dist.mean", false)
}

fn dist_var(vm: &mut dyn Vm) -> Result<(), Error> {
    moment(vm, "dist.var", true)
}

fn sampler_of(word: &str, f: &Family) -> Result<Draw, Error> {
    f.sampler().map_err(|m| parameters(word, f.name(), m))
}

fn dist_sample(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "dist.sample";
    let v = operands(vm, W, 1)?;
    let f = decode(W, &v[0])?;
    let draw = sampler_of(W, &f)?;
    let y = with_rng(vm, |g| draw(g)).map_err(|r| r.error(W))?;
    vm.push(y);
    Ok(())
}

fn dist_sample_n(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "dist.sample*";
    let v = operands(vm, W, 2)?;
    let f = decode(W, &v[0])?;
    let n = count(W, "n", &v[1])?;
    if n > DRAW_CAP {
        return Err(Refusal::Cap {
            what: format!("{n} draws"),
            cap: DRAW_CAP,
        }
        .error(W));
    }
    let draw = sampler_of(W, &f)?;
    let ys = with_rng(vm, |g| {
        (0..n).map(|_| draw(g)).collect::<Result<Vec<_>, Refusal>>()
    })
    .map_err(|r| r.error(W))?;
    vm.push(BundValue::list(ys));
    Ok(())
}

/// Register §S7's words, each with its `.` form (§S3).
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    macro_rules! family {
        ($word:literal, $name:literal, $n:literal) => {{
            fn f(vm: &mut dyn Vm) -> Result<(), Error> {
                construct(vm, $word, $name)
            }
            word!($word, f, $n);
        }};
    }
    family!("dist.normal", "normal", 2);
    family!("dist.lognormal", "lognormal", 2);
    family!("dist.uniform", "uniform", 2);
    family!("dist.exponential", "exponential", 1);
    family!("dist.gamma", "gamma", 2);
    family!("dist.beta", "beta", 2);
    family!("dist.chisq", "chisq", 1);
    family!("dist.t", "t", 1);
    family!("dist.f", "f", 2);
    family!("dist.binomial", "binomial", 2);
    family!("dist.poisson", "poisson", 1);
    family!("dist.negbinomial", "negbinomial", 2);
    family!("dist.hypergeometric", "hypergeometric", 3);
    family!("dist.geometric", "geometric", 1);
    family!("dist.bernoulli", "bernoulli", 1);
    family!("dist.multinomial", "multinomial", 2);
    family!("dist.dirichlet", "dirichlet", 1);
    word!("dist.pdf", dist_pdf, 2);
    word!("dist.lpdf", dist_lpdf, 2);
    word!("dist.cdf", dist_cdf, 2);
    word!("dist.sf", dist_sf, 2);
    word!("dist.quantile", dist_quantile, 2);
    word!("dist.mean", dist_mean, 1);
    word!("dist.var", dist_var, 1);
    word!("dist.sample", dist_sample, 1);
    word!("dist.sample*", dist_sample_n, 2);
}

/// §S7's words; each also has its `.` form.
pub const WORDS: [&str; 26] = [
    "dist.normal",
    "dist.lognormal",
    "dist.uniform",
    "dist.exponential",
    "dist.gamma",
    "dist.beta",
    "dist.chisq",
    "dist.t",
    "dist.f",
    "dist.binomial",
    "dist.poisson",
    "dist.negbinomial",
    "dist.hypergeometric",
    "dist.geometric",
    "dist.bernoulli",
    "dist.multinomial",
    "dist.dirichlet",
    "dist.pdf",
    "dist.lpdf",
    "dist.cdf",
    "dist.sf",
    "dist.quantile",
    "dist.mean",
    "dist.var",
    "dist.sample",
    "dist.sample*",
];

#[cfg(test)]
mod tests;
