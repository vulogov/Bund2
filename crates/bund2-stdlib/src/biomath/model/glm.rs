//! RFC-0018 §S4: `fit.glm`, a generalised linear model by IRLS, as R's
//! `glm.fit` fits one (R 4.4.2's `src/library/stats/R/glm.R:195-335`).
//!
//! **The iteration** starts from the family's `mustart` and η = g(μ), and
//! each step forms z = (η − offset) + (y − μ)/μ′(η) and W = w·μ′²/V(μ),
//! solves by the QR of √W·X with `weighted_solve`'s rank test, and stops
//! when |dev − dev_old|/(0.1 + |dev|) < 10⁻⁸. A step whose deviance is not
//! finite, then a step whose η or μ leaves the valid range, is halved
//! towards the previous coefficients, each of R's two loops at most 100
//! times for each step; on the first step there is nothing to halve
//! towards, and that is a refusal. At most 100 iterations.
//!
//! **The logistic path.** Under the binomial's `logit` on a 0/1 response,
//! every weight exactly 1.0 and every offset exactly 0.0, the arithmetic is
//! `fit.logistic`'s: W = μ(1 − μ), z from the clamped η, and the deviance by
//! its softplus form, so the two words' deviance sequences, and stopping
//! iterations, are the same (K15).
//!
//! **Where R warns, this word refuses**: an unconverged fit (E110),
//! separation, fitted Poisson rates numerically 0, and a perfect fit under
//! an estimated dispersion (E101). Each is tested once, on the converged
//! fit.

use std::collections::BTreeMap;
use std::f64::consts::{FRAC_1_SQRT_2, PI};

use bund2_api::{Error, Vm};
use bund2_value::BundValue;
use nalgebra::{DMatrix, DVector};

use super::design::{self, Design};
use super::opts;
use crate::biomath::frame::render;
use crate::biomath::infer::t_p;
use crate::biomath::operand::{WIDTH, domain, list, operands};
use crate::biomath::refuse::Refusal;

const W: &str = "fit.glm";

/// ε, `.Machine$double.eps`.
const EPS: f64 = f64::EPSILON;

/// `fit.logistic`'s clamp on η under `logit`, −ln ε (`fit.rs`).
const ETA_LIMIT: f64 = 36.04365338911715;

/// R's `glm.control()` epsilon.
const DEVIANCE_TOL: f64 = 1e-8;

/// The iterations, and the halvings of one step in each of R's loops.
const CAP: u64 = 100;

/// A link, with R's `make.link` bounds (`family.R:50-130`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Link {
    Logit,
    Probit,
    Cauchit,
    Cloglog,
    Identity,
    Log,
    Sqrt,
    InverseSquare,
    Inverse,
}

impl Link {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Link::Logit => "logit",
            Link::Probit => "probit",
            Link::Cauchit => "cauchit",
            Link::Cloglog => "cloglog",
            Link::Identity => "identity",
            Link::Log => "log",
            Link::Sqrt => "sqrt",
            Link::InverseSquare => "1/mu^2",
            Link::Inverse => "inverse",
        }
    }

    fn parse(s: &str) -> Option<Link> {
        [
            Link::Logit,
            Link::Probit,
            Link::Cauchit,
            Link::Cloglog,
            Link::Identity,
            Link::Log,
            Link::Sqrt,
            Link::InverseSquare,
            Link::Inverse,
        ]
        .into_iter()
        .find(|l| l.name() == s)
    }

    /// −Φ⁻¹(ε): probit's bound on η.
    fn probit_bound() -> f64 {
        -inverse_normal(EPS)
    }

    /// −F⁻¹_Cauchy(ε): cauchit's bound on η.
    fn cauchit_bound() -> f64 {
        -(PI * (EPS - 0.5)).tan()
    }

    /// g(μ).
    pub(crate) fn link(self, mu: f64) -> f64 {
        match self {
            Link::Logit => (mu / (1.0 - mu)).ln(),
            Link::Probit => inverse_normal(mu),
            Link::Cauchit => (PI * (mu - 0.5)).tan(),
            Link::Cloglog => (-(1.0 - mu).ln()).ln(),
            Link::Identity => mu,
            Link::Log => mu.ln(),
            Link::Sqrt => mu.sqrt(),
            Link::InverseSquare => 1.0 / (mu * mu),
            Link::Inverse => 1.0 / mu,
        }
    }

    /// g⁻¹(η), clamped as R's links clamp it; `logit` by `fit.logistic`'s
    /// continuous clamp (K15).
    pub(crate) fn inverse(self, eta: f64) -> f64 {
        match self {
            Link::Logit => 1.0 / (1.0 + (-eta.clamp(-ETA_LIMIT, ETA_LIMIT)).exp()),
            Link::Probit => {
                let t = Link::probit_bound();
                normal_cdf(eta.clamp(-t, t))
            }
            Link::Cauchit => {
                let t = Link::cauchit_bound();
                0.5 + eta.clamp(-t, t).atan() / PI
            }
            Link::Cloglog => (-(-eta.exp()).exp_m1()).clamp(EPS, 1.0 - EPS),
            Link::Identity => eta,
            Link::Log => eta.exp().max(EPS),
            Link::Sqrt => eta * eta,
            Link::InverseSquare => 1.0 / eta.sqrt(),
            Link::Inverse => 1.0 / eta,
        }
    }

    /// dμ/dη, bounded below by ε where R bounds it.
    pub(crate) fn mu_eta(self, eta: f64) -> f64 {
        let raw = match self {
            Link::Logit => {
                let mu = self.inverse(eta);
                mu * (1.0 - mu)
            }
            Link::Probit => (-0.5 * eta * eta).exp() / (2.0 * PI).sqrt(),
            Link::Cauchit => 1.0 / (PI * (1.0 + eta * eta)),
            Link::Cloglog => {
                let e = eta.min(700.0);
                e.exp() * (-e.exp()).exp()
            }
            Link::Identity => 1.0,
            Link::Log => eta.exp(),
            Link::Sqrt => 2.0 * eta,
            Link::InverseSquare => -1.0 / (2.0 * eta.powf(1.5)),
            Link::Inverse => -1.0 / (eta * eta),
        };
        if matches!(self, Link::Probit | Link::Cauchit | Link::Cloglog | Link::Log) {
            raw.max(EPS)
        } else {
            raw
        }
    }

    /// R's `valideta`: finite, and η > 0 under `sqrt` and `1/mu^2`, η ≠ 0
    /// under `inverse` (`family.R:109`, `:115`, `:121`).
    pub(crate) fn valid_eta(self, eta: f64) -> bool {
        eta.is_finite()
            && match self {
                Link::Sqrt | Link::InverseSquare => eta > 0.0,
                Link::Inverse => eta != 0.0,
                _ => true,
            }
    }
}

/// Φ(x).
fn normal_cdf(x: f64) -> f64 {
    0.5 * libm::erfc(-x * FRAC_1_SQRT_2)
}

/// Φ⁻¹(p), by `statrs`.
fn inverse_normal(p: f64) -> f64 {
    use statrs::distribution::{ContinuousCDF, Normal};
    Normal::standard().inverse_cdf(p)
}

/// A family (§S4's table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Family {
    Binomial,
    Poisson,
    Gaussian,
    Gamma,
    InverseGaussian,
}

impl Family {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Family::Binomial => "binomial",
            Family::Poisson => "poisson",
            Family::Gaussian => "gaussian",
            Family::Gamma => "gamma",
            Family::InverseGaussian => "inverse.gaussian",
        }
    }

    fn parse(s: &str) -> Option<Family> {
        [
            Family::Binomial,
            Family::Poisson,
            Family::Gaussian,
            Family::Gamma,
            Family::InverseGaussian,
        ]
        .into_iter()
        .find(|f| f.name() == s)
    }

    /// The canonical link, the default.
    fn canonical(self) -> Link {
        match self {
            Family::Binomial => Link::Logit,
            Family::Poisson => Link::Log,
            Family::Gaussian => Link::Identity,
            Family::Gamma => Link::Inverse,
            Family::InverseGaussian => Link::InverseSquare,
        }
    }

    /// R's `okLinks` (K9).
    fn links(self) -> &'static [Link] {
        match self {
            Family::Binomial => &[
                Link::Logit,
                Link::Probit,
                Link::Cloglog,
                Link::Cauchit,
                Link::Log,
            ],
            Family::Poisson => &[Link::Log, Link::Identity, Link::Sqrt],
            Family::Gaussian => &[Link::Identity, Link::Log, Link::Inverse],
            Family::Gamma => &[Link::Inverse, Link::Log, Link::Identity],
            Family::InverseGaussian => &[
                Link::InverseSquare,
                Link::Inverse,
                Link::Log,
                Link::Identity,
            ],
        }
    }

    /// Whether the dispersion is estimated.
    pub(crate) fn estimated(self) -> bool {
        !matches!(self, Family::Binomial | Family::Poisson)
    }

    /// V(μ).
    pub(crate) fn variance(self, mu: f64) -> f64 {
        match self {
            Family::Binomial => mu * (1.0 - mu),
            Family::Poisson => mu,
            Family::Gaussian => 1.0,
            Family::Gamma => mu * mu,
            Family::InverseGaussian => mu * mu * mu,
        }
    }

    /// R's `validmu`; the inverse Gaussian's is μ > 0 here (§S4).
    pub(crate) fn valid_mu(self, mu: f64) -> bool {
        match self {
            Family::Binomial => mu.is_finite() && mu > 0.0 && mu < 1.0,
            Family::Gaussian => true,
            _ => mu.is_finite() && mu > 0.0,
        }
    }

    /// One observation's deviance contribution (`family.R`, `dev.resids`;
    /// the binomial's `family.c:116-146`).
    pub(crate) fn deviance(self, y: f64, mu: f64, w: f64) -> f64 {
        let ylogy = |y: f64, mu: f64| if y != 0.0 { y * (y / mu).ln() } else { 0.0 };
        match self {
            Family::Binomial => 2.0 * w * (ylogy(y, mu) + ylogy(1.0 - y, 1.0 - mu)),
            Family::Poisson => {
                if y > 0.0 {
                    2.0 * (w * (y * (y / mu).ln() - (y - mu)))
                } else {
                    2.0 * (mu * w)
                }
            }
            Family::Gaussian => w * ((y - mu) * (y - mu)),
            Family::Gamma => {
                let r = if y == 0.0 { 1.0 } else { y / mu };
                -2.0 * w * (r.ln() - (y - mu) / mu)
            }
            Family::InverseGaussian => w * ((y - mu) * (y - mu)) / (y * (mu * mu)),
        }
    }

    /// R's `mustart` (§Current behaviour).
    fn start(self, y: f64, w: f64) -> f64 {
        match self {
            Family::Binomial => (w * y + 0.5) / (w + 1.0),
            Family::Poisson => y + 0.1,
            _ => y,
        }
    }
}

/// The deviance of every observation, summed in order.
fn total_deviance(f: Family, y: &[f64], mu: &[f64], w: &[f64]) -> f64 {
    y.iter()
        .zip(mu)
        .zip(w)
        .map(|((y, m), w)| f.deviance(*y, *m, *w))
        .sum()
}

/// The binomial deviance of 0/1 `y` at η, `fit.logistic`'s softplus form.
fn logistic_deviance(y: &[f64], eta: &[f64]) -> f64 {
    let softplus = |t: f64| t.max(0.0) + (-t.abs()).exp().ln_1p();
    2.0 * y
        .iter()
        .zip(eta)
        .map(|(y, e)| if *y == 1.0 { softplus(-e) } else { softplus(*e) })
        .sum::<f64>()
}

/// E101 on the design.
fn no(text: String) -> Error {
    domain(W, "design", text)
}

/// How a FLOAT reads in a refusal.
fn shown(x: f64) -> String {
    BundValue::float(x).summary(WIDTH)
}

/// Which model a fit is, for the texts: the model itself or its null.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Which {
    Full,
    Null,
}

impl Which {
    fn suffix(self) -> &'static str {
        match self {
            Which::Full => "",
            Which::Null => " (null model)",
        }
    }

    fn method(self) -> &'static str {
        match self {
            Which::Full => "IRLS",
            Which::Null => "IRLS (null model)",
        }
    }
}

/// What the iteration needs.
struct Problem<'a> {
    family: Family,
    link: Link,
    x: &'a DMatrix<f64>,
    names: &'a [String],
    y: &'a [f64],
    w: &'a [f64],
    offset: &'a [f64],
    logistic: bool,
    which: Which,
}

/// A solve: β, and R⁻¹ of the QR of √W·X.
struct Solved {
    beta: DVector<f64>,
    rinv: DMatrix<f64>,
}

/// The weighted least-squares solve by the QR of √W·X, with
/// `weighted_solve`'s rank test, E106 naming the first failing column.
fn solve(p: &Problem, w: &[f64], z: &[f64]) -> Result<Solved, Error> {
    let (n, k) = p.x.shape();
    let sw: Vec<f64> = w.iter().map(|w| w.sqrt()).collect();
    let wa = DMatrix::from_fn(n, k, |i, j| sw[i] * p.x[(i, j)]);
    let wz = DVector::from_fn(n, |i, _| sw[i] * z[i]);
    let qr = wa.qr();
    let r = qr.r();
    let scale = (0..k).map(|i| r[(i, i)].abs()).fold(0.0, f64::max);
    let failing = if scale.is_nan() || scale <= 0.0 {
        Some(0)
    } else {
        (0..k).find(|&i| r[(i, i)].abs() <= scale * 1e-10)
    };
    let singular = |i: usize| {
        let column = p
            .names
            .get(i)
            .map_or_else(|| format!("{}", i + 1), |n| render::text(n));
        Refusal::SingularColumn { column }.error(W)
    };
    if let Some(i) = failing {
        return Err(singular(i));
    }
    let qtz = qr.q().transpose() * wz;
    let (Some(beta), Some(rinv)) = (
        r.solve_upper_triangular(&qtz),
        r.solve_upper_triangular(&DMatrix::identity(k, k)),
    ) else {
        return Err(singular(0));
    };
    Ok(Solved { beta, rinv })
}

/// η = Xβ + offset.
fn predictor(p: &Problem, beta: &DVector<f64>) -> Vec<f64> {
    let xb = p.x * beta;
    xb.iter().zip(p.offset).map(|(e, o)| e + o).collect()
}

/// The converged iteration.
struct Converged {
    beta: DVector<f64>,
    eta: Vec<f64>,
    mu: Vec<f64>,
    dev: f64,
    iterations: u64,
    halvings: u64,
}

impl Problem<'_> {
    fn mu_of(&self, eta: &[f64]) -> Vec<f64> {
        eta.iter().map(|e| self.link.inverse(*e)).collect()
    }

    fn deviance(&self, eta: &[f64], mu: &[f64]) -> f64 {
        if self.logistic {
            logistic_deviance(self.y, eta)
        } else {
            total_deviance(self.family, self.y, mu, self.w)
        }
    }

    fn valid(&self, eta: &[f64], mu: &[f64]) -> bool {
        eta.iter().all(|e| self.link.valid_eta(*e)) && mu.iter().all(|m| self.family.valid_mu(*m))
    }

    /// The working weights W and responses z at η (`glm.R:244-248`), or on
    /// the logistic path `fit.logistic`'s.
    fn working(&self, eta: &[f64], mu: &[f64]) -> Result<(Vec<f64>, Vec<f64>), Error> {
        let n = eta.len();
        let mut w = Vec::with_capacity(n);
        let mut z = Vec::with_capacity(n);
        for i in 0..n {
            let (e, m, y) = (eta[i], mu[i], self.y[i]);
            if self.logistic {
                let wi = m * (1.0 - m);
                w.push(wi);
                z.push(e.clamp(-ETA_LIMIT, ETA_LIMIT) + (y - m) / wi);
                continue;
            }
            let d = self.link.mu_eta(e);
            let zi = (e - self.offset[i]) + (y - m) / d;
            if !zi.is_finite() || d == 0.0 {
                return Err(Refusal::Overflow {
                    expression: format!(
                        "the working response at row {}, whose dμ/dη is {}{}",
                        i + 1,
                        if d == 0.0 { "0".to_string() } else { shown(d) },
                        self.which.suffix()
                    ),
                }
                .error(W));
            }
            w.push(self.w[i] * d * d / self.family.variance(m));
            z.push(zi);
        }
        Ok((w, z))
    }

    /// The first step's refusal: no earlier coefficients to halve towards.
    fn first_step(&self, eta: &[f64]) -> Error {
        let range = if eta.iter().all(|e| self.link.valid_eta(*e)) {
            format!("the {} family's valid range", self.family.name())
        } else {
            format!("the {} link's valid range", self.link.name())
        };
        no(format!(
            "the first IRLS step leaves {range}, with no earlier coefficients to halve \
             towards{}",
            self.which.suffix()
        ))
    }

    /// §S4's iteration from μ = `start`.
    fn iterate(&self, start: &[f64]) -> Result<Converged, Error> {
        let mut eta: Vec<f64> = start.iter().map(|m| self.link.link(*m)).collect();
        if let Some(i) = eta.iter().position(|e| !e.is_finite()) {
            return Err(no(format!(
                "y row {} is {}, whose starting η under the {} link is not finite{}",
                i + 1,
                shown(self.y[i]),
                self.link.name(),
                self.which.suffix()
            )));
        }
        let mut mu = self.mu_of(&eta);
        if !self.valid(&eta, &mu) {
            return Err(Error::internal(format!(
                "{W}: the {} family's start under the {} link is outside its valid \
                 range; §S4's response checks were to exclude it",
                self.family.name(),
                self.link.name()
            )));
        }
        let mut devold = self.deviance(&eta, &mu);
        let mut coefold: Option<DVector<f64>> = None;
        let mut halvings: u64 = 0;
        for iteration in 1..=CAP {
            let (w, z) = self.working(&eta, &mu)?;
            let mut beta = solve(self, &w, &z)?.beta;
            if let Some(j) = beta.iter().position(|b| !b.is_finite()) {
                let name = self
                    .names
                    .get(j)
                    .map_or_else(|| format!("{}", j + 1), |n| render::text(n));
                return Err(Refusal::Overflow {
                    expression: format!(
                        "the coefficient of {name} at IRLS iteration {iteration}{}",
                        self.which.suffix()
                    ),
                }
                .error(W));
            }
            eta = predictor(self, &beta);
            mu = self.mu_of(&eta);
            let mut dev = self.deviance(&eta, &mu);
            // R's first loop: a deviance that is not finite.
            if !dev.is_finite() {
                let Some(old) = &coefold else {
                    return Err(self.first_step(&eta));
                };
                let mut ii = 1;
                while !dev.is_finite() {
                    if ii > CAP {
                        return Err(Refusal::NoConvergence {
                            method: format!("IRLS step halving{}", self.which.suffix()),
                            cap: CAP,
                        }
                        .error(W));
                    }
                    ii += 1;
                    halvings += 1;
                    beta = (&beta + old) / 2.0;
                    eta = predictor(self, &beta);
                    mu = self.mu_of(&eta);
                    dev = self.deviance(&eta, &mu);
                }
            }
            // R's second loop: η or μ outside the valid range.
            if !self.valid(&eta, &mu) {
                let Some(old) = &coefold else {
                    return Err(self.first_step(&eta));
                };
                let mut ii = 1;
                while !self.valid(&eta, &mu) {
                    if ii > CAP {
                        return Err(Refusal::NoConvergence {
                            method: format!("IRLS step halving{}", self.which.suffix()),
                            cap: CAP,
                        }
                        .error(W));
                    }
                    ii += 1;
                    halvings += 1;
                    beta = (&beta + old) / 2.0;
                    eta = predictor(self, &beta);
                    mu = self.mu_of(&eta);
                }
                dev = self.deviance(&eta, &mu);
            }
            if (dev - devold).abs() / (0.1 + dev.abs()) < DEVIANCE_TOL {
                return Ok(Converged {
                    beta,
                    eta,
                    mu,
                    dev,
                    iterations: iteration,
                    halvings,
                });
            }
            devold = dev;
            coefold = Some(beta);
        }
        Err(Refusal::NoConvergence {
            method: self.which.method().into(),
            cap: CAP,
        }
        .error(W))
    }

    /// The refusals R only warns of, on the converged fit.
    fn converged_checks(&self, c: &Converged) -> Result<(), Error> {
        let tiny = 10.0 * EPS;
        let suffix = self.which.suffix();
        match self.family {
            Family::Binomial => {
                let separated = if self.link == Link::Logit {
                    c.eta.iter().any(|e| e.abs() >= ETA_LIMIT)
                } else {
                    c.mu.iter().any(|m| *m > 1.0 - tiny || *m < tiny)
                };
                if separated {
                    return Err(no(format!(
                        "fitted probabilities numerically 0 or 1: X separates y{suffix}"
                    )));
                }
            }
            Family::Poisson => {
                if let Some(i) = c.mu.iter().position(|m| *m < tiny) {
                    return Err(no(format!(
                        "fitted rate numerically 0 at row {}{suffix}",
                        i + 1
                    )));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// A Poisson `y` an integer by R's `R_nonint` test (`nmath.h:63`).
fn integral(y: f64) -> bool {
    (y - y.round_ties_even()).abs() <= 1e-9 * y.abs().max(1.0)
}

/// ln Γ.
fn lgamma(x: f64) -> f64 {
    libm::lgamma(x)
}

/// The family's AIC, R's formula plus 2k (`glm.R:376-377`); `None` for a
/// non-integer Poisson `y`.
fn aic(f: Family, y: &[f64], mu: &[f64], w: &[f64], dev: f64, k: usize) -> Option<f64> {
    let n = y.len() as f64;
    let sum_w: f64 = w.iter().sum();
    let twok = 2.0 * k as f64;
    let core = match f {
        Family::Gaussian => {
            n * ((dev / n * 2.0 * PI).ln() + 1.0) + 2.0 - w.iter().map(|w| w.ln()).sum::<f64>()
        }
        Family::Poisson => {
            if !y.iter().all(|y| integral(*y)) {
                return None;
            }
            -2.0 * y
                .iter()
                .zip(mu)
                .zip(w)
                .map(|((y, m), w)| {
                    let x = y.round_ties_even();
                    let ld = if x == 0.0 {
                        -m
                    } else {
                        x * m.ln() - m - lgamma(x + 1.0)
                    };
                    ld * w
                })
                .sum::<f64>()
        }
        Family::Binomial => {
            -2.0 * y
                .iter()
                .zip(mu)
                .zip(w)
                .map(|((y, m), w)| {
                    let trials = w.round_ties_even();
                    let s = (w * y).round_ties_even();
                    let mut ld = lgamma(trials + 1.0) - lgamma(s + 1.0) - lgamma(trials - s + 1.0);
                    if s > 0.0 {
                        ld += s * m.ln();
                    }
                    if trials - s > 0.0 {
                        ld += (trials - s) * (1.0 - m).ln();
                    }
                    // (w/m)·ln p with m = w: 1 for every positive weight.
                    ld
                })
                .sum::<f64>()
        }
        Family::Gamma => {
            let disp = dev / sum_w;
            let shape = 1.0 / disp;
            -2.0 * y
                .iter()
                .zip(mu)
                .zip(w)
                .map(|((y, m), w)| {
                    let scale = m * disp;
                    let ld = -lgamma(shape) - shape * scale.ln() + (shape - 1.0) * y.ln()
                        - y / scale;
                    ld * w
                })
                .sum::<f64>()
                + 2.0
        }
        Family::InverseGaussian => {
            sum_w * (1.0 + (dev / sum_w * 2.0 * PI).ln())
                + 3.0 * y.iter().zip(w).map(|(y, w)| y.ln() * w).sum::<f64>()
                + 2.0
        }
    };
    Some(core + twok)
}

/// The response checks (§S4's table), each E101 naming the row.
fn responses(f: Family, link: Link, d: &Design) -> Result<(), Error> {
    for (i, y) in d.y.iter().enumerate() {
        let r = i + 1;
        let bad = match f {
            Family::Binomial if !(0.0..=1.0).contains(y) => Some(format!(
                "y row {r} is {}, outside the binomial family's [0, 1]",
                shown(*y)
            )),
            Family::Poisson if *y < 0.0 => Some(format!(
                "y row {r} is {}, below the poisson family's 0",
                shown(*y)
            )),
            Family::Gamma | Family::InverseGaussian if *y <= 0.0 => Some(format!(
                "y row {r} is {}, not above the {} family's 0",
                shown(*y),
                f.name()
            )),
            Family::Gaussian if link == Link::Log && *y <= 0.0 => Some(format!(
                "y row {r} is {}, not above 0, as the gaussian family's log link needs",
                shown(*y)
            )),
            Family::Gaussian if link == Link::Inverse && *y == 0.0 => Some(format!(
                "y row {r} is 0, which the gaussian family's inverse link cannot start from"
            )),
            _ => None,
        };
        if let Some(t) = bad {
            return Err(no(t));
        }
    }
    if f == Family::Binomial {
        let near = |x: f64| (x - x.round()).abs() <= 1e-3;
        if let Some(i) = d.weights.iter().position(|w| !near(*w)) {
            return Err(no(format!(
                "weights row {} is {}, not a count of trials",
                i + 1,
                shown(d.weights[i])
            )));
        }
        if let Some(i) = d
            .y
            .iter()
            .zip(&d.weights)
            .position(|(y, w)| !near(w * y))
        {
            return Err(no(format!(
                "y row {} times its weight is {}, not a count",
                i + 1,
                shown(d.y[i] * d.weights[i])
            )));
        }
    }
    Ok(())
}

/// Each element finite, or E114 naming the first that is not.
fn finite_each(what: &str, xs: Vec<f64>) -> Result<BundValue, Error> {
    if let Some(i) = xs.iter().position(|x| !x.is_finite()) {
        return Err(Refusal::Overflow {
            expression: format!("{what} at {}", i + 1),
        }
        .error(W));
    }
    Ok(list(xs))
}

fn finite(what: &str, x: f64) -> Result<f64, Error> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(Refusal::Overflow {
            expression: what.into(),
        }
        .error(W))
    }
}

fn count(x: u64) -> BundValue {
    BundValue::int(i64::try_from(x).unwrap_or(i64::MAX))
}

/// The family and link `opts` asks for.
pub(crate) fn family_and_link(word: &str, o: &opts::Opts) -> Result<(Family, Link), Error> {
    let Some(fname) = &o.family else {
        return Err(opts::refuse(word, "family is required".into()));
    };
    let Some(family) = Family::parse(fname) else {
        return Err(opts::refuse(
            word,
            format!("family is {}, not a family", render::text(fname)),
        ));
    };
    let link = match &o.link {
        None => family.canonical(),
        Some(l) => match Link::parse(l).filter(|l| family.links().contains(l)) {
            Some(l) => l,
            None => {
                return Err(opts::refuse(
                    word,
                    format!("link is {}, not a {} link", render::text(l), family.name()),
                ));
            }
        },
    };
    Ok((family, link))
}

/// `fit.glm` (`design opts -- fit`).
pub(crate) fn fit_glm(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = operands(vm, W, 2)?;
    let (design_v, opts_v) = match v.as_slice() {
        [a, b] => (a, b),
        _ => {
            return Err(Error::internal(format!(
                "{W}: two operands were pulled and {} are held",
                v.len()
            )));
        }
    };
    let d = design::check(W, "design", design_v)?;
    let o = opts::read(W, opts_v, &["family", "link"])?;
    let (family, link) = family_and_link(W, &o)?;
    let spec = design_v
        .as_map()
        .and_then(|m| m.get("spec").cloned())
        .ok_or_else(|| Error::internal(format!("{W}: the checked design has no spec")))?;
    let fit = fit(&d, family, link, spec)?;
    vm.push(fit);
    Ok(())
}

/// The fit's MAP (§S4's table).
fn fit(d: &Design, family: Family, link: Link, spec: BundValue) -> Result<BundValue, Error> {
    responses(family, link, d)?;
    let n = d.y.len();
    let k = d.names.len();
    let x = DMatrix::from_fn(n, k, |i, j| {
        d.x.get(i).and_then(|r| r.get(j)).copied().unwrap_or(0.0)
    });
    let logistic = family == Family::Binomial
        && link == Link::Logit
        && d.y.iter().all(|y| *y == 0.0 || *y == 1.0)
        && d.weights.iter().all(|w| *w == 1.0)
        && d.offset.iter().all(|o| *o == 0.0);
    let p = Problem {
        family,
        link,
        x: &x,
        names: &d.names,
        y: &d.y,
        w: &d.weights,
        offset: &d.offset,
        logistic,
        which: Which::Full,
    };
    let start: Vec<f64> = d
        .y
        .iter()
        .zip(&d.weights)
        .map(|(y, w)| {
            if logistic {
                (y + 0.5) / 2.0
            } else {
                family.start(*y, *w)
            }
        })
        .collect();
    let c = p.iterate(&start)?;
    p.converged_checks(&c)?;
    if family.estimated() && c.dev <= 0.0 {
        return Err(no("a perfect fit, its residual variance 0".into()));
    }
    // The working weights, the covariance and the leverages at the estimate.
    let ww: Vec<f64> = c
        .eta
        .iter()
        .zip(&c.mu)
        .zip(&d.weights)
        .map(|((e, m), w)| {
            if logistic {
                m * (1.0 - m)
            } else {
                let dm = link.mu_eta(*e);
                w * dm * dm / family.variance(*m)
            }
        })
        .collect();
    let z0: Vec<f64> = vec![0.0; n];
    let at = solve(&p, &ww, &z0)?;
    let df = n - k;
    let dispersion = if family.estimated() {
        let pearson: f64 = d
            .y
            .iter()
            .zip(&c.mu)
            .zip(&d.weights)
            .map(|((y, m), w)| w * (y - m) * (y - m) / family.variance(*m))
            .sum();
        pearson / df as f64
    } else {
        1.0
    };
    let cov = &at.rinv * at.rinv.transpose() * dispersion;
    let coef: Vec<f64> = c.beta.iter().copied().collect();
    let se: Vec<f64> = (0..k).map(|i| cov[(i, i)].sqrt()).collect();
    let stat: Vec<f64> = coef.iter().zip(&se).map(|(b, s)| b / s).collect();
    let pv: Vec<f64> = if family.estimated() {
        stat.iter()
            .map(|t| t_p(W, *t, df as f64))
            .collect::<Result<Vec<f64>, Error>>()?
    } else {
        stat.iter()
            .map(|z| libm::erfc(z.abs() * FRAC_1_SQRT_2))
            .collect()
    };
    let sw: Vec<f64> = ww.iter().map(|w| w.sqrt()).collect();
    let wa = DMatrix::from_fn(n, k, |i, j| sw[i] * x[(i, j)]);
    let q = &wa * &at.rinv;
    let hat: Vec<f64> = (0..n)
        .map(|r| (0..k).map(|j| q[(r, j)] * q[(r, j)]).sum())
        .collect();
    let mut vcov = Vec::with_capacity(k);
    for r in 0..k {
        vcov.push(finite_each(
            "the covariance",
            (0..k).map(|j| cov[(r, j)]).collect(),
        )?);
    }
    let intercept = d.spec.intercept;
    let null = null_deviance(d, &p, &c, intercept)?;
    let aic = aic(family, &d.y, &c.mu, &d.weights, c.dev, k);
    let mut m = BTreeMap::new();
    m.insert("coef".to_string(), finite_each("the coefficient", coef)?);
    m.insert("se".to_string(), finite_each("the standard error", se)?);
    let stat_key = if family.estimated() { "t" } else { "z" };
    m.insert(stat_key.to_string(), finite_each("the test statistic", stat)?);
    m.insert("p".to_string(), list(pv));
    m.insert(
        "names".to_string(),
        BundValue::list(d.names.iter().map(|s| BundValue::str(s.clone())).collect()),
    );
    m.insert("vcov".to_string(), BundValue::list(vcov));
    m.insert("spec".to_string(), spec);
    m.insert("y".to_string(), list(d.y.clone()));
    m.insert("weights".to_string(), list(d.weights.clone()));
    m.insert("offset".to_string(), list(d.offset.clone()));
    m.insert("fitted".to_string(), finite_each("the fitted value", c.mu.clone())?);
    m.insert("linear".to_string(), finite_each("the linear predictor", c.eta.clone())?);
    m.insert("hat".to_string(), finite_each("the leverage", hat)?);
    m.insert(
        "working.weights".to_string(),
        finite_each("the working weight", ww)?,
    );
    m.insert(
        "dispersion".to_string(),
        BundValue::float(finite("the dispersion", dispersion)?),
    );
    m.insert(
        "deviance".to_string(),
        BundValue::float(finite("the deviance", c.dev)?),
    );
    if let Some(nd) = null.filter(|x| x.is_finite()) {
        m.insert("null.deviance".to_string(), BundValue::float(nd));
    }
    m.insert("df.residual".to_string(), count(df as u64));
    m.insert(
        "df.null".to_string(),
        count((n - usize::from(intercept)) as u64),
    );
    if let Some(a) = aic {
        let a = finite("the AIC", a)?;
        let pk = k as f64 + if family.estimated() { 1.0 } else { 0.0 };
        let bic = finite("the BIC", a - 2.0 * pk + pk * (n as f64).ln())?;
        m.insert("aic".to_string(), BundValue::float(a));
        m.insert("bic".to_string(), BundValue::float(bic));
    }
    m.insert("iterations".to_string(), count(c.iterations));
    m.insert("halvings".to_string(), count(c.halvings));
    m.insert("family".to_string(), BundValue::str(family.name()));
    m.insert("link".to_string(), BundValue::str(link.name()));
    Ok(BundValue::map(m))
}

/// The null deviance (§S4): at linkinv(offset) without an intercept; at the
/// weighted mean of y with one and no `spec.offset`; else the deviance of
/// the intercept-plus-offset model, refitted from the full fit's μ̂.
fn null_deviance(d: &Design, p: &Problem, c: &Converged, intercept: bool) -> Result<Option<f64>, Error> {
    let f = p.family;
    if !intercept {
        let mu: Vec<f64> = d.offset.iter().map(|o| p.link.inverse(*o)).collect();
        return Ok(Some(total_deviance(f, &d.y, &mu, &d.weights)));
    }
    if d.spec.offset.is_none() {
        let num: f64 = d.weights.iter().zip(&d.y).map(|(w, y)| w * y).sum();
        let den: f64 = d.weights.iter().sum();
        let m = num / den;
        let mu = vec![m; d.y.len()];
        return Ok(Some(total_deviance(f, &d.y, &mu, &d.weights)));
    }
    let n = d.y.len();
    let ones = DMatrix::from_element(n, 1, 1.0);
    let names = ["(intercept)".to_string()];
    let q = Problem {
        family: f,
        link: p.link,
        x: &ones,
        names: &names,
        y: &d.y,
        w: &d.weights,
        offset: &d.offset,
        logistic: false,
        which: Which::Null,
    };
    let refit = q.iterate(&c.mu)?;
    q.converged_checks(&refit)?;
    Ok(Some(refit.dev))
}
