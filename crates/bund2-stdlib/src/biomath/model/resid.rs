//! RFC-0018 §S5: `model.residuals` and `model.influence`, both read from
//! the fit alone.
//!
//! The residuals are R's `residuals.glm` (`glm.R:860-862`): `"response"`
//! y − μ, `"pearson"` (y − μ)√w/√V(μ), `"deviance"` sign(y − μ)√dᵢ,
//! `"working"` (y − μ)/μ′(η), and `"standardized"` the deviance residual
//! over √(φ(1 − hᵢ)), R's `rstandard.glm`. Cook's distance is
//! (r_P/(1 − h))²·h/(φ·k), R's `cooks.distance.glm`, which is
//! `cooks.distance.lm` for `fit.lm`. A leverage within 10⁻¹⁰ of 1 is E114
//! naming the row, from the two words that divide by 1 − hᵢ.

use std::collections::BTreeMap;

use bund2_api::{Error, Vm};
use bund2_value::BundValue;

use super::opts;
use super::reading::{self, Fit};
use crate::biomath::frame::render;
use crate::biomath::operand::{list, operands};
use crate::biomath::refuse::Refusal;

/// The residual types (§S5).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Type {
    Response,
    Pearson,
    Deviance,
    Working,
    Standardized,
}

impl Type {
    fn parse(s: &str) -> Option<Type> {
        Some(match s {
            "response" => Type::Response,
            "pearson" => Type::Pearson,
            "deviance" => Type::Deviance,
            "working" => Type::Working,
            "standardized" => Type::Standardized,
            _ => return None,
        })
    }
}

/// A leverage that 1 − h can divide by, or E114 naming the row.
fn room(word: &str, what: &str, h: f64, row: usize) -> Result<f64, Error> {
    let d = 1.0 - h;
    if d <= 1e-10 {
        return Err(Refusal::Overflow {
            expression: format!("{what} at row {row}, whose leverage is 1"),
        }
        .error(word));
    }
    Ok(d)
}

fn finite_each(word: &str, what: &str, xs: Vec<f64>) -> Result<BundValue, Error> {
    if let Some(i) = xs.iter().position(|x| !x.is_finite()) {
        return Err(Refusal::Overflow {
            expression: format!("{what} at row {}", i + 1),
        }
        .error(word));
    }
    Ok(list(xs))
}

/// The residuals of a type.
fn residuals(word: &str, f: &Fit, t: Type) -> Result<Vec<f64>, Error> {
    let n = f.y.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (y, mu, w, eta) = (f.y[i], f.fitted[i], f.weights[i], f.linear[i]);
        let r = match t {
            Type::Response => y - mu,
            Type::Pearson => (y - mu) * w.sqrt() / f.family.variance(mu).sqrt(),
            Type::Working => (y - mu) / f.link.mu_eta(eta),
            Type::Deviance | Type::Standardized => {
                let d = f.family.deviance(y, mu, w).max(0.0).sqrt();
                let signed = if y > mu {
                    d
                } else if y < mu {
                    -d
                } else {
                    0.0
                };
                if t == Type::Deviance {
                    signed
                } else {
                    let room = room(word, "the standardized residual", f.hat[i], i + 1)?;
                    signed / (f.dispersion * room).sqrt()
                }
            }
        };
        out.push(r);
    }
    Ok(out)
}

fn two(vm: &mut dyn Vm, word: &str) -> Result<(BundValue, BundValue), Error> {
    let v = operands(vm, word, 2)?;
    let [a, b] = <[BundValue; 2]>::try_from(v).map_err(|v| {
        Error::internal(format!(
            "{word}: two operands were pulled and {} are held",
            v.len()
        ))
    })?;
    Ok((a, b))
}

/// `model.residuals` (`fit opts -- list`).
pub(crate) fn model_residuals(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "model.residuals";
    let (fit_v, opts_v) = two(vm, W)?;
    let f = reading::check(W, "fit", &fit_v)?;
    let o = opts::read(W, &opts_v, &["type"])?;
    let t = match &o.kind {
        None => Type::Deviance,
        Some(s) => {
            Type::parse(s).ok_or_else(|| opts::refuse(W, format!("type is {}", render::text(s))))?
        }
    };
    let r = residuals(W, &f, t)?;
    vm.push(finite_each(W, "the residual", r)?);
    Ok(())
}

/// `model.influence` (`fit -- map`).
pub(crate) fn model_influence(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "model.influence";
    let v = operands(vm, W, 1)?;
    let fit_v = v
        .into_iter()
        .next()
        .ok_or_else(|| Error::internal(format!("{W}: one operand was pulled and none is held")))?;
    let f = reading::check(W, "fit", &fit_v)?;
    let std = residuals(W, &f, Type::Standardized)?;
    let pearson = residuals(W, &f, Type::Pearson)?;
    let k = f.coef.len() as f64;
    let mut cooks = Vec::with_capacity(pearson.len());
    for (i, (rp, h)) in pearson.iter().zip(&f.hat).enumerate() {
        let room = room(W, "Cook's distance", *h, i + 1)?;
        let s = rp / room;
        cooks.push(s * s * h / (f.dispersion * k));
    }
    let mut m = BTreeMap::new();
    m.insert(
        "hat".to_string(),
        finite_each(W, "the leverage", f.hat.clone())?,
    );
    m.insert(
        "std.residuals".to_string(),
        finite_each(W, "the standardized residual", std)?,
    );
    m.insert(
        "cooks".to_string(),
        finite_each(W, "Cook's distance", cooks)?,
    );
    vm.push(BundValue::map(m));
    Ok(())
}
