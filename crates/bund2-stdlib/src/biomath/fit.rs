//! RFC-0015 §S16: nonlinear least squares and logistic regression.
//!
//! **`fit.nls`** is Levenberg–Marquardt, Bund2's own (D250 B8). Its model
//! is a LAMBDA, `x p -- y`, run as §S15's are ([`super::dynamics::call`]):
//! on a scratch stack, with `x` and `p` the only values there, its answer a
//! LIST of finite numbers as long as `x`, else E112. The Jacobian is by
//! forward differences, one call of the model per parameter per accepted
//! step; the damped normal equations are solved by `nalgebra`'s Cholesky.
//!
//! **`fit.logistic`** is iteratively reweighted least squares, as R's
//! `glm(family = binomial)` fits, with an intercept as `stat.lm` adds one.
//!
//! **Effects.** `fit.nls` calls a lambda, so it is `StackEffect::opaque` and
//! has the plain form only; `fit.logistic` is fixed, with its `.` form
//! (§S3).
//!
//! **Work (D39).** `fit.nls` takes at most 200 iterations and
//! `fit.logistic` 100, else E110.

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;
use nalgebra::{DMatrix, DVector};

use super::dynamics::{call, lambda};
use super::infer::table;
use super::operand::{domain, finite_all, list, operands, whole};
use super::refuse::Refusal;

/// `fit.nls`'s iteration cap (§S16).
const NLS_CAP: u64 = 200;

/// `fit.logistic`'s iteration cap (§S16).
const IRLS_CAP: u64 = 100;

/// A relative change in the residual sum of squares below this converges.
const RSS_TOL: f64 = 1e-10;

/// A relative change in every parameter below this converges.
const PARAM_TOL: f64 = 1e-8;

/// R's `glm.control()` epsilon: a relative change in the deviance below
/// this converges.
const DEVIANCE_TOL: f64 = 1e-8;

/// R's binomial `linkinv` threshold, −ln ε: beyond it a fitted
/// probability is numerically 0 or 1.
const ETA_LIMIT: f64 = 36.04365338911715;

fn too_few(word: &str, n: usize, k: usize, what: &str) -> Error {
    domain(word, "y", format!("{n} observations for {k} {what}"))
}

fn lengths(word: &str, n: usize, m: usize) -> Error {
    Refusal::Disagree {
        what: "lengths".into(),
        n,
        m,
    }
    .error(word)
}

fn no_convergence(word: &str, method: &str, cap: u64) -> Error {
    Refusal::NoConvergence {
        method: method.into(),
        cap,
    }
    .error(word)
}

fn count(n: u64) -> BundValue {
    BundValue::int(i64::try_from(n).unwrap_or(i64::MAX))
}

// fit.nls.

/// The model's answer at `p`, and its residuals y − f.
struct Nls<'a> {
    f: &'a [BundValue],
    x: BundValue,
    y: &'a [f64],
}

impl Nls<'_> {
    fn residuals(&self, vm: &mut dyn Vm, word: &str, p: &[f64]) -> Result<Vec<f64>, Error> {
        let fx = call(
            vm,
            word,
            self.f,
            vec![self.x.clone(), list(p.to_vec())],
            self.y.len(),
            false,
        )?;
        Ok(self.y.iter().zip(fx).map(|(y, f)| y - f).collect())
    }

    /// J of f, by forward differences with hⱼ = √ε·max(|pⱼ|, 1). The step
    /// taken is the one representable, (pⱼ + hⱼ) − pⱼ.
    fn jacobian(
        &self,
        vm: &mut dyn Vm,
        word: &str,
        p: &[f64],
        r: &[f64],
    ) -> Result<DMatrix<f64>, Error> {
        let n = r.len();
        let mut j = DMatrix::zeros(n, p.len());
        for c in 0..p.len() {
            let mut q = p.to_vec();
            q[c] = p[c] + f64::EPSILON.sqrt() * p[c].abs().max(1.0);
            let h = q[c] - p[c];
            let rq = self.residuals(vm, word, &q)?;
            // f = y − r, so ∂f = (r − r_q)/h.
            for i in 0..n {
                j[(i, c)] = (r[i] - rq[i]) / h;
            }
        }
        Ok(j)
    }
}

fn sum_sq(r: &[f64]) -> f64 {
    r.iter().map(|x| x * x).sum()
}

fn fit_nls(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "fit.nls";
    let v = operands(vm, W, 4)?;
    let f = lambda(W, "f", &v[0])?.to_vec();
    let x = whole(W, "x", &v[1])?;
    let y = whole(W, "y", &v[2])?;
    let mut p = whole(W, "p0", &v[3])?;
    if x.len() != y.len() {
        return Err(lengths(W, x.len(), y.len()));
    }
    let (n, k) = (y.len(), p.len());
    if k == 0 {
        return Err(domain(W, "p0", "an empty LIST".into()));
    }
    if n <= k {
        return Err(too_few(W, n, k, "parameters"));
    }
    let model = Nls {
        f: &f,
        x: list(x),
        y: &y,
    };
    let mut r = model.residuals(vm, W, &p)?;
    let mut rss = sum_sq(&r);
    let mut j = model.jacobian(vm, W, &p, &r)?;
    let mut damping = {
        let jtj = j.transpose() * &j;
        let top = (0..k).map(|i| jtj[(i, i)]).fold(0.0, f64::max);
        // Every column 0: no parameter moves the model.
        if top == 0.0 || !top.is_finite() {
            return Err(Refusal::Singular.error(W));
        }
        1e-3 * top
    };
    let mut iterations: u64 = 0;
    loop {
        iterations += 1;
        if iterations > NLS_CAP {
            return Err(no_convergence(W, "Levenberg–Marquardt", NLS_CAP));
        }
        let jt = j.transpose();
        let a = &jt * &j + DMatrix::identity(k, k) * damping;
        let g = &jt * DVector::from_column_slice(&r);
        let step = a.cholesky().map(|c| c.solve(&g));
        let Some(step) = step.filter(|s| s.iter().all(|d| d.is_finite())) else {
            damping *= 10.0;
            continue;
        };
        let q: Vec<f64> = p.iter().zip(step.iter()).map(|(a, d)| a + d).collect();
        let rq = model.residuals(vm, W, &q)?;
        let rss_q = sum_sq(&rq);
        if rss_q.is_finite() && rss_q <= rss {
            let converged = rss - rss_q <= RSS_TOL * rss
                || step
                    .iter()
                    .zip(&q)
                    .all(|(d, a)| d.abs() <= PARAM_TOL * a.abs());
            (p, r, rss) = (q, rq, rss_q);
            damping /= 10.0;
            j = model.jacobian(vm, W, &p, &r)?;
            if converged {
                break;
            }
        } else {
            damping *= 10.0;
        }
    }
    let df = (n - k) as f64;
    let jtj = j.transpose() * &j;
    let Some(inv) = jtj.cholesky().map(|c| c.inverse()) else {
        return Err(Refusal::Singular.error(W));
    };
    let s2 = rss / df;
    let se: Vec<f64> = (0..k).map(|i| (s2 * inv[(i, i)]).sqrt()).collect();
    finite_all(W, "the parameters", &p)?;
    if !se.iter().all(|s| s.is_finite()) || !rss.is_finite() {
        return Err(Refusal::Overflow {
            expression: "se".into(),
        }
        .error(W));
    }
    let mut m = BTreeMap::new();
    m.insert("params".to_string(), list(p));
    m.insert("se".to_string(), list(se));
    m.insert("rss".to_string(), BundValue::float(rss));
    m.insert("df".to_string(), BundValue::float(df));
    m.insert("iterations".to_string(), count(iterations));
    m.insert("converged".to_string(), BundValue::boolean(true));
    vm.push(BundValue::map(m));
    Ok(())
}

// fit.logistic.

/// The logistic function at η, η clamped to R's ±(−ln ε).
fn inverse_logit(eta: f64) -> f64 {
    1.0 / (1.0 + (-eta.clamp(-ETA_LIMIT, ETA_LIMIT)).exp())
}

/// The binomial deviance of 0/1 `y` at linear predictors `eta`:
/// 2·ln(1 + e^{−η}) for a 1, 2·ln(1 + e^{η}) for a 0.
fn deviance(y: &[f64], eta: &[f64]) -> f64 {
    let softplus = |t: f64| t.max(0.0) + (-t.abs()).exp().ln_1p();
    2.0 * y
        .iter()
        .zip(eta)
        .map(|(y, e)| {
            if *y == 1.0 {
                softplus(-e)
            } else {
                softplus(*e)
            }
        })
        .sum::<f64>()
}

/// (AᵀWA)⁻¹ by QR of √W·A, or E106 when A's columns are dependent.
fn weighted_solve(
    word: &str,
    a: &DMatrix<f64>,
    w: &[f64],
    z: &[f64],
) -> Result<(DVector<f64>, DMatrix<f64>), Error> {
    let (n, k) = a.shape();
    let sw: Vec<f64> = w.iter().map(|w| w.sqrt()).collect();
    let wa = DMatrix::from_fn(n, k, |i, j| sw[i] * a[(i, j)]);
    let wz = DVector::from_fn(n, |i, _| sw[i] * z[i]);
    let qr = wa.qr();
    let r = qr.r();
    let scale = (0..k).map(|i| r[(i, i)].abs()).fold(0.0, f64::max);
    if scale.is_nan() || scale <= 0.0 || (0..k).any(|i| r[(i, i)].abs() <= scale * 1e-10) {
        return Err(Refusal::Singular.error(word));
    }
    let qtz = qr.q().transpose() * wz;
    let (Some(beta), Some(rinv)) = (
        r.solve_upper_triangular(&qtz),
        r.solve_upper_triangular(&DMatrix::identity(k, k)),
    ) else {
        return Err(Refusal::Singular.error(word));
    };
    let cov = &rinv * rinv.transpose();
    Ok((beta, cov))
}

fn fit_logistic(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "fit.logistic";
    let v = operands(vm, W, 2)?;
    let x = table(W, "X", &v[0])?;
    let y = whole(W, "y", &v[1])?;
    if x.len() != y.len() {
        return Err(lengths(W, x.len(), y.len()));
    }
    if let Some(i) = y.iter().position(|v| *v != 0.0 && *v != 1.0) {
        return Err(domain(W, "y", format!("{} at {}, not 0 or 1", y[i], i + 1)));
    }
    let n = y.len();
    let k = x.first().map_or(0, Vec::len) + 1;
    if n <= k {
        return Err(too_few(W, n, k, "coefficients"));
    }
    let a = DMatrix::from_fn(n, k, |i, j| if j == 0 { 1.0 } else { x[i][j - 1] });
    // R's start: μ = (y + ½)/2.
    let mut eta: Vec<f64> = y
        .iter()
        .map(|y| {
            let mu = (y + 0.5) / 2.0;
            (mu / (1.0 - mu)).ln()
        })
        .collect();
    let mut dev = deviance(&y, &eta);
    let mut iterations: u64 = 0;
    let beta = loop {
        iterations += 1;
        if iterations > IRLS_CAP {
            return Err(no_convergence(W, "IRLS", IRLS_CAP));
        }
        let mu: Vec<f64> = eta.iter().map(|e| inverse_logit(*e)).collect();
        let w: Vec<f64> = mu.iter().map(|m| m * (1.0 - m)).collect();
        let z: Vec<f64> = (0..n)
            .map(|i| eta[i].clamp(-ETA_LIMIT, ETA_LIMIT) + (y[i] - mu[i]) / w[i])
            .collect();
        let (beta, _) = weighted_solve(W, &a, &w, &z)?;
        eta = (&a * &beta).iter().copied().collect();
        let next = deviance(&y, &eta);
        let change = (next - dev).abs() / (next.abs() + 0.1);
        dev = next;
        if change < DEVIANCE_TOL {
            break beta;
        }
    };
    if eta.iter().any(|e| e.abs() >= ETA_LIMIT) {
        return Err(domain(
            W,
            "y",
            "fitted probabilities numerically 0 or 1: X separates y".into(),
        ));
    }
    // The standard errors from the information at the estimate.
    let mu: Vec<f64> = eta.iter().map(|e| inverse_logit(*e)).collect();
    let w: Vec<f64> = mu.iter().map(|m| m * (1.0 - m)).collect();
    let (_, cov) = weighted_solve(W, &a, &w, &eta)?;
    let coef: Vec<f64> = beta.iter().copied().collect();
    finite_all(W, "the coefficients", &coef)?;
    let se: Vec<f64> = (0..k).map(|i| cov[(i, i)].sqrt()).collect();
    let z: Vec<f64> = coef.iter().zip(&se).map(|(c, s)| c / s).collect();
    let p: Vec<f64> = z
        .iter()
        .map(|z| libm::erfc(z.abs() * std::f64::consts::FRAC_1_SQRT_2))
        .collect();
    let mut m = BTreeMap::new();
    m.insert("coef".to_string(), list(coef));
    m.insert("se".to_string(), list(se));
    m.insert("z".to_string(), list(z));
    m.insert("p".to_string(), list(p));
    m.insert("deviance".to_string(), BundValue::float(dev));
    m.insert("aic".to_string(), BundValue::float(dev + 2.0 * k as f64));
    vm.push(BundValue::map(m));
    Ok(())
}

/// Register §S16's words: `fit.nls` calls a lambda, so is opaque and plain
/// only; `fit.logistic` is fixed, with its `.` form (§S3).
pub fn register(r: &mut Registry) {
    r.register_native("fit.nls", fit_nls, StackEffect::opaque(4), WordKind::Sync);
    r.register_native(
        "fit.logistic",
        fit_logistic,
        StackEffect::fixed(2, 1),
        WordKind::Sync,
    );
    crate::wb::bench!(r, "fit.logistic", fit_logistic, 2, 0);
    // RFC-0018 §S9: the model words' fits, beside `fit.logistic`.
    r.register_native(
        "fit.lm",
        super::model::lm::fit_lm,
        StackEffect::fixed(2, 1),
        WordKind::Sync,
    );
    crate::wb::bench!(r, "fit.lm", super::model::lm::fit_lm, 2, 0);
    r.register_native(
        "fit.glm",
        super::model::glm::fit_glm,
        StackEffect::fixed(2, 1),
        WordKind::Sync,
    );
    crate::wb::bench!(r, "fit.glm", super::model::glm::fit_glm, 2, 0);
}

/// §S16's words with their forms, and RFC-0018's `fit.glm` and `fit.lm`
/// (§S9).
pub const WORDS: [&str; 7] = [
    "fit.glm",
    "fit.glm.",
    "fit.lm",
    "fit.lm.",
    "fit.logistic",
    "fit.logistic.",
    "fit.nls",
];

#[cfg(test)]
mod tests;
