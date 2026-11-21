//! RFC-0018 §S3: `fit.lm`, weighted least squares over a design.
//!
//! It minimises Σ wᵢ(yᵢ − offsetᵢ − xᵢβ)² by the QR of √W·X, through
//! `stat.lm`'s own computation ([`least_squares`]): with every weight 1.0,
//! no offset and an intercept, √W·X is X exactly and y − 0.0 is y, so the
//! two words answer the same `coef`, `se`, `t`, `p` and `r2`, bit for bit
//! (K15). The weighted mean is Σ wᵢ(yᵢ − offsetᵢ)/Σ wᵢ, which with unit
//! weights is `describe.rs`'s `mean`, Σ/n, bit for bit.
//!
//! **Refusals**, deepest operand first: the design's check (§S2), then
//! `opts`, a MAP that takes no key yet, then the fit's own: a singular
//! design (E106 naming the first dependent column), a constant
//! `y − offset` with an intercept, a perfect fit, and E114 for any answer
//! that is not finite.

use std::collections::BTreeMap;

use bund2_api::{Error, Vm};
use bund2_value::BundValue;
use nalgebra::{DMatrix, DVector};

use super::design::{self, Design};
use super::opts;
use crate::biomath::dist::Family;
use crate::biomath::frame::render;
use crate::biomath::infer::{LeastSquares, least_squares};
use crate::biomath::operand::{list, operands};
use crate::biomath::refuse::Refusal;

const W: &str = "fit.lm";

/// An INTEGER from a count.
fn count(x: usize) -> BundValue {
    BundValue::int(i64::try_from(x).unwrap_or(i64::MAX))
}

/// A finite answer, or E114 naming it.
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

/// What the closed forms need besides least squares: the scaled design,
/// the response less its offset, and the total sum of squares.
struct Scaled {
    a: DMatrix<f64>,
    ys: DVector<f64>,
    tss: f64,
}

/// √W·X, √W·(y − offset), and TSS = Σ wᵢ(yᵢ − offsetᵢ − m)², m the weighted
/// mean of y − offset with an intercept and 0 without.
fn scale(d: &Design) -> Scaled {
    let n = d.y.len();
    let k = d.names.len();
    let sw: Vec<f64> = d.weights.iter().map(|w| w.sqrt()).collect();
    let z: Vec<f64> = d.y.iter().zip(&d.offset).map(|(y, o)| y - o).collect();
    let a = DMatrix::from_fn(n, k, |i, j| {
        sw[i] * d.x.get(i).and_then(|r| r.get(j)).copied().unwrap_or(0.0)
    });
    let ys = DVector::from_fn(n, |i, _| sw[i] * z[i]);
    let m = if d.spec.intercept {
        let num: f64 = d.weights.iter().zip(&z).map(|(w, z)| w * z).sum();
        let den: f64 = d.weights.iter().sum();
        num / den
    } else {
        0.0
    };
    let tss: f64 = d
        .weights
        .iter()
        .zip(&z)
        .map(|(w, z)| w * ((z - m) * (z - m)))
        .sum();
    Scaled { a, ys, tss }
}

/// The fit's least squares: `stat.lm`'s computation on √W·X, its
/// refusals naming `word`; `constant` refuses a constant `y − offset` with
/// an intercept, as `fit.lm` does and `model.anova`'s rows do not (§S7).
fn solve(word: &str, d: &Design, s: &Scaled, constant: bool) -> Result<LeastSquares, Error> {
    let singular = |i: usize| {
        let column = d
            .names
            .get(i)
            .map_or_else(|| format!("{}", i + 1), |n| render::text(n));
        Refusal::SingularColumn { column }.error(word)
    };
    least_squares(
        word,
        "design",
        &s.a,
        &s.ys,
        s.tss,
        constant && d.spec.intercept,
        &singular,
    )
}

/// A Gaussian identity row of `model.anova` (§S7): the RSS of `d`'s least
/// squares, every refusal of `fit.lm`'s computation but the constant one,
/// and the TSS, the deviance of its intercept-and-offset model.
pub(crate) fn sums(word: &str, d: &Design) -> Result<(f64, f64), Error> {
    let s = scale(d);
    let ls = solve(word, d, &s, false)?;
    Ok((ls.rss, s.tss))
}

/// `fit.lm` (`design opts -- fit`).
pub(crate) fn fit_lm(vm: &mut dyn Vm) -> Result<(), Error> {
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
    opts::read(W, opts_v, &[])?;
    let spec = design_v
        .as_map()
        .and_then(|m| m.get("spec").cloned())
        .ok_or_else(|| Error::internal(format!("{W}: the checked design has no spec")))?;
    let fit = fit(&d, spec)?;
    vm.push(fit);
    Ok(())
}

/// The fit's MAP (§S3's table).
fn fit(d: &Design, spec: BundValue) -> Result<BundValue, Error> {
    let n = d.y.len();
    let k = d.names.len();
    let i = usize::from(d.spec.intercept);
    let s = scale(d);
    let ls = solve(W, d, &s, true)?;
    let (nf, kf) = (n as f64, k as f64);
    let rss = ls.rss;
    let tss = s.tss;
    let only_intercept = k == i;
    let (r2, adj) = if only_intercept {
        (0.0, 0.0)
    } else {
        let r2 = ls.r2;
        (r2, 1.0 - (1.0 - r2) * ((n - i) as f64) / ((n - k) as f64))
    };
    // σ̂²(XᵀWX)⁻¹ = σ̂² R⁻¹R⁻ᵀ, R the QR's of √W·X.
    let cov = &ls.rinv * ls.rinv.transpose() * ls.sigma2;
    let mut vcov = Vec::with_capacity(k);
    for r in 0..k {
        let row: Vec<f64> = (0..k).map(|c| cov[(r, c)]).collect();
        vcov.push(finite_each("the covariance", row)?);
    }
    // hᵢ: the squared row norms of √W·X·R⁻¹.
    let q = &s.a * &ls.rinv;
    let hat: Vec<f64> = (0..n)
        .map(|r| (0..k).map(|c| q[(r, c)] * q[(r, c)]).sum())
        .collect();
    let fitted: Vec<f64> = (0..n)
        .map(|r| {
            let row = d.x.get(r).map(Vec::as_slice).unwrap_or(&[]);
            let eta: f64 = row.iter().zip(&ls.coef).map(|(x, b)| x * b).sum();
            eta + d.offset.get(r).copied().unwrap_or(0.0)
        })
        .collect();
    let sum_log_w: f64 = d.weights.iter().map(|w| w.ln()).sum();
    let two_pi = 2.0 * std::f64::consts::PI;
    let aic = finite(
        "the AIC",
        nf * ((two_pi * rss / nf).ln() + 1.0) - sum_log_w + 2.0 * (kf + 1.0),
    )?;
    let bic = finite("the BIC", aic - 2.0 * (kf + 1.0) + (kf + 1.0) * nf.ln())?;

    let floats = |xs: &[f64]| list(xs.to_vec());
    let mut m = BTreeMap::new();
    m.insert("coef".to_string(), list(ls.coef.clone()));
    m.insert("se".to_string(), list(ls.se.clone()));
    m.insert("t".to_string(), list(ls.t.clone()));
    m.insert("p".to_string(), list(ls.p.clone()));
    m.insert(
        "names".to_string(),
        BundValue::list(d.names.iter().map(|n| BundValue::str(n.clone())).collect()),
    );
    m.insert("vcov".to_string(), BundValue::list(vcov));
    m.insert(
        "sigma".to_string(),
        BundValue::float(finite("sigma", ls.sigma2.sqrt())?),
    );
    m.insert(
        "deviance".to_string(),
        BundValue::float(finite("RSS", rss)?),
    );
    m.insert("df.residual".to_string(), count(n - k));
    m.insert("r2".to_string(), BundValue::float(finite("r2", r2)?));
    m.insert(
        "adj.r2".to_string(),
        BundValue::float(finite("adj.r2", adj)?),
    );
    if !only_intercept {
        let (df1, df2) = (k - i, n - k);
        let f = finite(
            "the F statistic",
            ((tss - rss) / df1 as f64) / (rss / df2 as f64),
        )?;
        let fp = Family::F {
            df1: df1 as f64,
            df2: df2 as f64,
        }
        .tails(W, f)?
        .1;
        m.insert("f".to_string(), BundValue::float(f));
        m.insert(
            "f.df".to_string(),
            BundValue::list(vec![count(df1), count(df2)]),
        );
        m.insert("f.p".to_string(), BundValue::float(fp));
    }
    m.insert(
        "null.deviance".to_string(),
        BundValue::float(finite("the null deviance", tss)?),
    );
    m.insert("df.null".to_string(), count(n - i));
    m.insert("aic".to_string(), BundValue::float(aic));
    m.insert("bic".to_string(), BundValue::float(bic));
    m.insert(
        "fitted".to_string(),
        finite_each("the fitted value", fitted)?,
    );
    m.insert("y".to_string(), floats(&d.y));
    m.insert("weights".to_string(), floats(&d.weights));
    m.insert("offset".to_string(), floats(&d.offset));
    m.insert("hat".to_string(), finite_each("the leverage", hat)?);
    m.insert("family".to_string(), BundValue::str("gaussian"));
    m.insert("link".to_string(), BundValue::str("identity"));
    m.insert(
        "dispersion".to_string(),
        BundValue::float(finite("the dispersion", ls.sigma2)?),
    );
    m.insert("spec".to_string(), spec);
    Ok(BundValue::map(m))
}
