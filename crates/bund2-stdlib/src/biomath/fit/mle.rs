//! RFC-0019 §S11: `fit.mle` (`loglik start opts -- MAP`), maximum
//! likelihood over a log-likelihood LAMBDA `par -- FLOAT`.
//!
//! **The fit** minimises −loglik from `start` by `optim.bfgs`, or by
//! `optim.nm` with `method` `"nm"` ([`bfgs`], [`nelder_mead`]), with their
//! `reltol`, `ndeps` and `maxit` (D275 3A). The Hessian of −loglik at `par`
//! is `optim.hessian`'s ([`hessian`]), and its inverse is `vcov`, by
//! Cholesky. A pivot at most 10⁻⁸ times the largest diagonal entry counts
//! as not positive (D269, 7), and is E106 with §S11's text, naming the
//! 1-based pivot and its value: the standard errors do not exist there. A
//! non-finite `se` is E114 (B12).
//!
//! **The answer** is `par`, `se` (√ of `vcov`'s diagonal), `vcov`, `loglik`
//! (ℓ̂), `iterations`, `aic` = 2k − 2ℓ̂, and `bic` = k·ln n − 2ℓ̂ when
//! `opts` gives `n` (an INTEGER ≥ 1).
//!
//! **Profile intervals** (N13, A), with `profile` true: for each
//! parameter θⱼ, the two roots of g(θ) = 2(ℓ̂ − ℓₚ(θ)) − χ²₁(`conf`),
//! `conf` 0.95 by default, ℓₚ(θ) maximised over the other parameters by the
//! same method and settings from their estimates (ℓ itself when k = 1).
//! g(θ̂ⱼ) = −χ²₁(`conf`) by definition and is not evaluated. Each root is
//! §S6's: the bracket from θ̂ⱼ to its Wald end θ̂ⱼ ± z·seⱼ,
//! z = Φ⁻¹((1 + `conf`)/2), extended outward by `uniroot`'s `"upX"` for the
//! upper end and `"downX"` for the lower ([`extend`]), then Brent's method
//! ([`zeroin`]) at N7's tolerance, 10⁻¹²·max(1, |ends|), each within
//! `optim.root`'s 1000 iterations, else E110. They are answered as `lower`
//! and `upper`, LISTs of k.
//!
//! **Cost** (§S11): every call of the lambda counts, and the 10⁷ + 1st is
//! E111, as the ODE words count steps.
//!
//! Opaque, plain only.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use bund2_api::{Error, Vm};
use bund2_value::BundValue;

use crate::biomath::dist::Family;
use crate::biomath::dynamics::lambda;
use crate::biomath::operand::{finite, list, operands};
use crate::biomath::optim::minimise::{
    BFGS_MAXIT, Counted, Minimum, NM_MAXIT, Settings, bfgs, hessian, matrix, nelder_mead,
    read_opts, start,
};
use crate::biomath::optim::{Bracket, Extend, MAXITER, Stop, extend, search_failed, zeroin};
use crate::biomath::refuse::Refusal;

const W: &str = "fit.mle";

/// A pivot at most this times the largest diagonal entry is not positive
/// (D269, 7).
const PIVOT_FLOOR: f64 = 1e-8;

/// The keys `fit.mle` takes (D275 3A).
const KEYS: [&str; 7] = ["method", "n", "profile", "conf", "reltol", "ndeps", "maxit"];

/// Minimise `f` from `x0` by the chosen method.
fn minimise(
    f: &mut dyn FnMut(&[f64]) -> Result<f64, Error>,
    x0: &[f64],
    s: &Settings,
    nm: bool,
) -> Result<Minimum, Error> {
    if nm {
        nelder_mead(W, f, x0, s.reltol, s.maxit.unwrap_or(NM_MAXIT))
    } else {
        bfgs(W, f, x0, s.reltol, &s.ndeps, s.maxit.unwrap_or(BFGS_MAXIT))
    }
}

/// H's inverse by Cholesky, or E106 at the first pivot that is not
/// positive.
#[expect(clippy::needless_range_loop, reason = "Cholesky's index form")]
fn inverse(h: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, Error> {
    let n = h.len();
    let largest = (0..n).map(|i| h[i][i]).fold(f64::NEG_INFINITY, f64::max);
    let mut l = vec![vec![0.0f64; n]; n];
    for j in 0..n {
        let mut d = h[j][j];
        for k in 0..j {
            d -= l[j][k] * l[j][k];
        }
        if d.partial_cmp(&(PIVOT_FLOOR * largest)) != Some(Ordering::Greater) || d <= 0.0 {
            return Err(Refusal::NotPositiveDefinite {
                pivot: j + 1,
                value: d,
            }
            .error(W));
        }
        let ljj = d.sqrt();
        l[j][j] = ljj;
        for i in j + 1..n {
            let mut s = h[i][j];
            for k in 0..j {
                s -= l[i][k] * l[j][k];
            }
            l[i][j] = s / ljj;
        }
    }
    // L Lᵀ X = I, a column at a time.
    let mut inv = vec![vec![0.0f64; n]; n];
    for c in 0..n {
        let mut y = vec![0.0f64; n];
        for i in 0..n {
            let mut s = if i == c { 1.0 } else { 0.0 };
            for k in 0..i {
                s -= l[i][k] * y[k];
            }
            y[i] = s / l[i][i];
        }
        for i in (0..n).rev() {
            let mut s = y[i];
            for k in i + 1..n {
                s -= l[k][i] * inv[k][c];
            }
            inv[i][c] = s / l[i][i];
        }
    }
    Ok(inv)
}

/// `fit.mle` (`loglik start opts -- MAP`).
pub(super) fn fit_mle(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = operands(vm, W, 3)?;
    let f = lambda(W, "loglik", &v[0])?.to_vec();
    let x0 = start(W, "start", &v[1])?;
    let k = x0.len();
    let (s, keys) = read_opts(W, &v[2], k, &KEYS)?;
    // −loglik, every call counted.
    let mut counted = Counted::new(W, f, -1.0);
    let mut obj = |x: &[f64]| counted.eval(vm, x);
    let m = minimise(&mut obj, &x0, &s, keys.nm)?;
    let loglik = -m.value;
    let h = hessian(W, &mut obj, &m.par, &s.ndeps)?;
    let vcov = inverse(&h)?;
    let mut se = Vec::with_capacity(k);
    for (j, row) in vcov.iter().enumerate() {
        se.push(finite(W, row[j].sqrt(), || {
            format!("the standard error of parameter {}", j + 1)
        })?);
    }
    let mut out = BTreeMap::new();
    if keys.profile {
        let normal = Family::Normal { mean: 0.0, sd: 1.0 };
        let z = normal.quantile(W, (1.0 + keys.conf) / 2.0)?;
        let crit = Family::ChiSq { df: 1.0 }.quantile(W, keys.conf)?;
        let (mut lower, mut upper) = (Vec::with_capacity(k), Vec::with_capacity(k));
        for j in 0..k {
            let theta = m.par[j];
            let others: Vec<f64> = (0..k).filter(|&i| i != j).map(|i| m.par[i]).collect();
            // g(θ) = 2(ℓ̂ − ℓₚ(θ)) − χ²₁(conf); the minimum of −ℓ is −ℓₚ.
            let mut g = |t: f64| -> Result<f64, Error> {
                let lp = if k == 1 {
                    -obj(&[t])?
                } else {
                    let mut inner = |rest: &[f64]| {
                        let mut x = Vec::with_capacity(k);
                        x.extend_from_slice(&rest[..j]);
                        x.push(t);
                        x.extend_from_slice(&rest[j..]);
                        obj(&x)
                    };
                    -minimise(&mut inner, &others, &s, keys.nm)?.value
                };
                Ok(2.0 * (loglik - lp) - crit)
            };
            let wald = z * se[j];
            for up in [false, true] {
                let far = if up { theta + wald } else { theta - wald };
                let f_far = g(far)?;
                let b = if up {
                    Bracket {
                        lower: theta,
                        f_lower: -crit,
                        upper: far,
                        f_upper: f_far,
                        steps: 0,
                    }
                } else {
                    Bracket {
                        lower: far,
                        f_lower: f_far,
                        upper: theta,
                        f_upper: -crit,
                        steps: 0,
                    }
                };
                let how = if up { Extend::UpX } else { Extend::DownX };
                let b = extend(&mut g, b, how, MAXITER).map_err(|e| match e {
                    Stop::F(e) => e,
                    Stop::Search | Stop::SameSign => search_failed(W, MAXITER),
                })?;
                let tol = 1e-12 * 1f64.max(b.lower.abs()).max(b.upper.abs());
                let zero = zeroin(
                    &mut g,
                    b.lower,
                    b.upper,
                    b.f_lower,
                    b.f_upper,
                    tol,
                    MAXITER - b.steps.min(MAXITER),
                )?;
                if !zero.converged {
                    return Err(Refusal::NoConvergence {
                        method: "the profile's root finder".into(),
                        cap: MAXITER,
                    }
                    .error(W));
                }
                if up {
                    upper.push(zero.root);
                } else {
                    lower.push(zero.root);
                }
            }
        }
        out.insert("lower".to_string(), list(lower));
        out.insert("upper".to_string(), list(upper));
    }
    let kf = k as f64;
    out.insert("par".to_string(), list(m.par.clone()));
    out.insert("se".to_string(), list(se));
    out.insert("vcov".to_string(), matrix(vcov));
    out.insert("loglik".to_string(), BundValue::float(loglik));
    out.insert(
        "iterations".to_string(),
        BundValue::int(i64::try_from(m.iterations).unwrap_or(i64::MAX)),
    );
    out.insert(
        "aic".to_string(),
        BundValue::float(finite(W, 2.0 * kf - 2.0 * loglik, || "the AIC".into())?),
    );
    if let Some(n) = keys.n {
        out.insert(
            "bic".to_string(),
            BundValue::float(finite(W, kf * (n as f64).ln() - 2.0 * loglik, || {
                "the BIC".into()
            })?),
        );
    }
    vm.push(BundValue::map(out));
    Ok(())
}

#[cfg(test)]
mod tests;
