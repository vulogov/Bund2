//! RFC-0018 §S7's "Reading a fit": what every `model.` word that reads a
//! fit asks of it, for `fit.lm`'s fits and `fit.glm`'s alike.
//!
//! The keys both fit words answer, each with its type: `coef`, `se`, `p`,
//! `names`, `vcov`, `fitted`, `y`, `weights`, `offset`, `hat`, `deviance`,
//! `df.residual`, `dispersion`, `family`, `link` and `spec`. A fit is a MAP
//! a program can edit, so the check also asks the lengths the words index
//! by (the sixth review's SF4): `coef`, `se`, `p` and `names` of one length
//! k, `vcov` k × k, and the per-row LISTs of one length n. A `fit.glm` key
//! that a `fit.lm` fit lacks is derived for it: η̂ = μ̂, W = the prior
//! weights. Each failure is E101 naming the key.

use bund2_api::Error;
use bund2_value::{BundValue, FLOAT, INTEGER, LIST, MAP, STRING};

use super::design::{self, Spec};
use super::glm::{Family, Link};
use crate::biomath::frame::render;
use crate::biomath::operand::{WIDTH, domain};

/// A fit that passed the check.
pub(crate) struct Fit {
    pub coef: Vec<f64>,
    pub names: Vec<String>,
    /// k rows of k.
    pub vcov: Vec<Vec<f64>>,
    /// μ̂.
    pub fitted: Vec<f64>,
    /// η̂.
    pub linear: Vec<f64>,
    pub y: Vec<f64>,
    pub weights: Vec<f64>,
    pub hat: Vec<f64>,
    pub df_residual: i64,
    pub dispersion: f64,
    pub family: Family,
    pub link: Link,
    pub spec: Spec,
}

fn no(word: &str, op: &str, text: String) -> Error {
    domain(word, op, text)
}

/// A LIST of finite numbers.
fn numbers(word: &str, op: &str, key: &str, v: &BundValue) -> Result<Vec<f64>, Error> {
    let items = match v.as_list() {
        Some(xs) if v.dt() == LIST => xs,
        _ => return Err(no(word, op, format!("{key} is {}", v.summary(WIDTH)))),
    };
    items
        .iter()
        .enumerate()
        .map(|(i, x)| match x.unboxed() {
            BundValue::Float(f, _) if x.dt() == FLOAT && f.is_finite() => Ok(*f),
            BundValue::Int(n, _) if x.dt() == INTEGER => Ok(*n as f64),
            _ => Err(no(
                word,
                op,
                format!("{key} item {} is {}", i + 1, render::value(x)),
            )),
        })
        .collect()
}

/// Check a fit operand `op` of `word`.
pub(crate) fn check(word: &str, op: &str, v: &BundValue) -> Result<Fit, Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(no(word, op, v.summary(WIDTH))),
    };
    let keys = [
        "coef",
        "se",
        "p",
        "names",
        "vcov",
        "fitted",
        "y",
        "weights",
        "offset",
        "hat",
        "deviance",
        "df.residual",
        "dispersion",
        "family",
        "link",
        "spec",
    ];
    for k in keys {
        if !m.contains_key(k) {
            return Err(no(word, op, format!("a missing key {}", render::text(k))));
        }
    }
    let get = |k: &str| {
        m.get(k)
            .ok_or_else(|| Error::internal(format!("{word}: the fit's key {k:?} was checked")))
    };
    let num = |k: &str| numbers(word, op, k, get(k)?);
    let coef = num("coef")?;
    let se = num("se")?;
    let p = num("p")?;
    let nv = get("names")?;
    let names: Vec<String> = match nv.as_list() {
        Some(xs) if nv.dt() == LIST => xs
            .iter()
            .enumerate()
            .map(|(i, x)| match x.as_str() {
                Some(s) if x.dt() == STRING => Ok(s),
                _ => Err(no(
                    word,
                    op,
                    format!("names item {} is {}", i + 1, render::value(x)),
                )),
            })
            .collect::<Result<_, _>>()?,
        _ => return Err(no(word, op, format!("names is {}", nv.summary(WIDTH)))),
    };
    let vv = get("vcov")?;
    let rows = match vv.as_list() {
        Some(xs) if vv.dt() == LIST => xs,
        _ => return Err(no(word, op, format!("vcov is {}", vv.summary(WIDTH)))),
    };
    let vcov = rows
        .iter()
        .map(|r| numbers(word, op, "vcov", r))
        .collect::<Result<Vec<_>, _>>()?;
    let fitted = num("fitted")?;
    let y = num("y")?;
    let weights = num("weights")?;
    let offset = num("offset")?;
    let hat = num("hat")?;
    let scalar = |k: &str| -> Result<f64, Error> {
        let x = get(k)?;
        match x.unboxed() {
            BundValue::Float(f, _) if x.dt() == FLOAT && f.is_finite() => Ok(*f),
            _ => Err(no(word, op, format!("{k} is {}", x.summary(WIDTH)))),
        }
    };
    let _deviance = scalar("deviance")?;
    let dispersion = scalar("dispersion")?;
    let dv = get("df.residual")?;
    let df_residual = match dv.unboxed() {
        BundValue::Int(d, _) if dv.dt() == INTEGER && *d > 0 => *d,
        _ => return Err(no(word, op, format!("df.residual is {}", dv.summary(WIDTH)))),
    };
    let text = |k: &str| -> Result<String, Error> {
        let x = get(k)?;
        match x.as_str() {
            Some(s) if x.dt() == STRING => Ok(s),
            _ => Err(no(word, op, format!("{k} is {}", x.summary(WIDTH)))),
        }
    };
    let fname = text("family")?;
    let family = Family::parse(&fname).ok_or_else(|| {
        no(
            word,
            op,
            format!("family is {}, not a family", render::text(&fname)),
        )
    })?;
    let lname = text("link")?;
    let link = Link::parse(&lname)
        .filter(|l| family.links().contains(l))
        .ok_or_else(|| {
            no(
                word,
                op,
                format!("link is {}, not a {} link", render::text(&lname), family.name()),
            )
        })?;
    let spec = design::spec(word, op, get("spec")?)?;
    // The lengths (SF4).
    let k = coef.len();
    for (key, len) in [("se", se.len()), ("p", p.len()), ("names", names.len())] {
        if len != k {
            return Err(no(
                word,
                op,
                format!("{key} has {len} items, where coef has {k}"),
            ));
        }
    }
    if vcov.len() != k || vcov.iter().any(|r| r.len() != k) {
        let c = vcov.first().map_or(0, Vec::len);
        return Err(no(
            word,
            op,
            format!("vcov is {} × {c}, where coef has {k}", vcov.len()),
        ));
    }
    let n = fitted.len();
    let linear = match m.get("linear") {
        Some(x) => numbers(word, op, "linear", x)?,
        None => fitted.clone(),
    };
    let working = match m.get("working.weights") {
        Some(x) => numbers(word, op, "working.weights", x)?,
        None => weights.clone(),
    };
    for (key, len) in [
        ("y", y.len()),
        ("weights", weights.len()),
        ("offset", offset.len()),
        ("hat", hat.len()),
        ("linear", linear.len()),
        ("working.weights", working.len()),
    ] {
        if len != n {
            return Err(no(
                word,
                op,
                format!("{key} has {len} items, where fitted has {n}"),
            ));
        }
    }
    if k == 0 || n == 0 {
        return Err(no(word, op, "a fit of no coefficients or no rows".into()));
    }
    Ok(Fit {
        coef,
        names,
        vcov,
        fitted,
        linear,
        y,
        weights,
        hat,
        df_residual,
        dispersion,
        family,
        link,
        spec,
    })
}
