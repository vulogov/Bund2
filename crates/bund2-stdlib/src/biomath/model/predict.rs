//! RFC-0018 §S6: `model.predict`, a fit's predictions at a new frame.
//!
//! The new rows are coded by the fit's `spec` as `model.matrix` coded the
//! design: the same columns, by §S2's plan, a factor's cells checked
//! against the spec's levels, the response not read, the offset's column
//! read when the spec names one. The checks run after the fit's and the
//! options': RFC-0017's of the frame, each column the spec reads present
//! and of the kind it was coded as, NONE under `"refuse"`, the cells of
//! the `weights` option's column, then only "no rows left" and the cap.
//!
//! **The answer**: `fit`, `se` (√(xᵀVx) on the link scale, times |μ′(η)|
//! on the response scale, the delta method), and with an interval `lower`
//! and `upper`, and `dropped`. On the response scale every value maps
//! through §S4's clamped inverse link, which is an interval only inside
//! the link's valid η: an η̂ or end outside it, or an interval across 0
//! under `inverse`, is E101 naming the row, and `type` `"link"` answers it.
//! An infinite end leaves its whole key out (D251's A3, D264).

use std::collections::BTreeMap;

use bund2_api::{Error, Vm};
use bund2_value::BundValue;

use super::code::{self, CAP, Column};
use super::glm::{Family, Link};
use super::opts;
use super::reading::{self, Fit};
use crate::biomath::dist::Family as Dist;
use crate::biomath::frame::check::{self, Frame, Kind};
use crate::biomath::frame::opts::Missing;
use crate::biomath::frame::render;
use crate::biomath::operand::{domain, list, operands};
use crate::biomath::refuse::Refusal;

const W: &str = "model.predict";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scale {
    Link,
    Response,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Interval {
    None,
    Confidence,
    Prediction,
}

/// What a spec codes: its terms, each column's kind, and the columns.
struct Coding {
    terms: Vec<Vec<String>>,
    kinds: BTreeMap<String, Kind>,
    columns: Vec<Column>,
}

/// The columns the fit's `spec` codes, which must be the fit's `names`.
fn coding(f: &Fit) -> Result<Coding, Error> {
    let fit_no = |t: String| domain(W, "fit", t);
    let items = f.spec.terms.as_list().unwrap_or_default();
    let mut terms = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let members = code::members_of(item).ok_or_else(|| {
            fit_no(format!(
                "spec.terms item {} is {}",
                i + 1,
                render::value(item)
            ))
        })?;
        terms.push(members);
    }
    let mut kinds = BTreeMap::new();
    for c in code::variable_order(&terms) {
        let k = if f.spec.levels.contains_key(&c) {
            Kind::Factor
        } else if f.spec.logical.contains(&c) {
            Kind::Logical
        } else {
            Kind::Numeric
        };
        kinds.insert(c, k);
    }
    let columns = code::plan(W, &terms, &kinds, &f.spec.levels, f.spec.intercept)?;
    if columns.len() != f.names.len() {
        return Err(fit_no(format!(
            "spec codes {} columns, names has {}",
            columns.len(),
            f.names.len()
        )));
    }
    for (j, (c, n)) in columns.iter().zip(&f.names).enumerate() {
        if &c.name != n {
            return Err(fit_no(format!(
                "spec codes column {} as {}, names has {}",
                j + 1,
                render::text(&c.name),
                render::text(n)
            )));
        }
    }
    Ok(Coding {
        terms,
        kinds,
        columns,
    })
}

/// How a coded kind reads after "where the fit coded".
fn coded(k: Kind) -> &'static str {
    match k {
        Kind::Factor => "a factor",
        Kind::Logical => "a logical column",
        _ => "a numeric column",
    }
}

fn frame_no(t: String) -> Error {
    domain(W, "frame", t)
}

/// An index the presence check found.
fn index(f: &Frame, n: &str) -> Result<usize, Error> {
    f.index(n)
        .ok_or_else(|| Error::internal(format!("{W}: column {n:?} was found and is gone")))
}

fn overflow(what: String) -> Error {
    Refusal::Overflow { expression: what }.error(W)
}

/// `model.predict` (`fit frame opts -- map`).
pub(crate) fn model_predict(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = operands(vm, W, 3)?;
    let [fit_v, frame_v, opts_v] = <[BundValue; 3]>::try_from(v).map_err(|v| {
        Error::internal(format!("{W}: three operands were pulled and {} are held", v.len()))
    })?;
    // The fit, and the columns its spec codes.
    let f = reading::check(W, "fit", &fit_v)?;
    let c = coding(&f)?;
    // The options.
    let o = opts::read(W, &opts_v, &["conf", "interval", "na", "type", "weights"])?;
    let scale = match o.kind.as_deref() {
        None | Some("response") => Scale::Response,
        Some("link") => Scale::Link,
        Some(s) => return Err(opts::refuse(W, format!("type is {}", render::text(s)))),
    };
    let interval = match o.interval.as_deref() {
        None | Some("none") => Interval::None,
        Some("confidence") => Interval::Confidence,
        Some("prediction") => Interval::Prediction,
        Some(s) => return Err(opts::refuse(W, format!("interval is {}", render::text(s)))),
    };
    let gaussian_identity = f.family == Family::Gaussian && f.link == Link::Identity;
    if interval == Interval::Prediction && !gaussian_identity {
        return Err(opts::refuse(
            W,
            "interval \"prediction\" needs a gaussian identity fit".into(),
        ));
    }
    let conf = o.conf.unwrap_or(0.95);
    let missing = o.missing.unwrap_or(Missing::Refuse);
    // RFC-0017's check of the frame.
    let fr = check::check(W, "frame", &frame_v)?;
    // Each column the spec reads present, then the `weights` option's.
    let order = code::variable_order(&c.terms);
    let mut reads: Vec<String> = order.clone();
    if let Some(off) = &f.spec.offset {
        reads.push(off.clone());
    }
    for n in &reads {
        if fr.index(n).is_none() {
            return Err(frame_no(format!("no column {}", render::text(n))));
        }
    }
    let wi = match &o.weights {
        Some(n) => Some(fr.index(n).ok_or_else(|| {
            opts::refuse(W, format!("weights names no column {}", render::text(n)))
        })?),
        None => None,
    };
    // Each column's kind as coded.
    for n in &order {
        let want = c.kinds.get(n).copied().unwrap_or(Kind::Numeric);
        let got = fr.kind(index(&fr, n)?);
        if got != want {
            return Err(frame_no(format!(
                "column {} is {}, where the fit coded {}",
                render::text(n),
                got.word(),
                coded(want)
            )));
        }
    }
    for (key, col) in [("offset", f.spec.offset.clone()), ("weights", o.weights.clone())] {
        let Some(n) = col else { continue };
        let k = fr.kind(index(&fr, &n)?);
        if k != Kind::Numeric {
            let t = format!("{key} column {} is {}", render::text(&n), k.word());
            return Err(if key == "weights" {
                opts::refuse(W, t)
            } else {
                frame_no(t)
            });
        }
    }
    // NONE under "refuse", then the levels and the weights' cells.
    let mut idx: Vec<usize> = Vec::new();
    for n in &reads {
        let i = index(&fr, n)?;
        if !idx.contains(&i) {
            idx.push(i);
        }
    }
    if let Some(i) = wi
        && !idx.contains(&i)
    {
        idx.push(i);
    }
    let rows = code::kept(W, &fr, &idx, missing)?;
    for n in &order {
        let Some(levels) = f.spec.levels.get(n) else {
            continue;
        };
        let cells = fr.cols.get(index(&fr, n)?).map(Vec::as_slice).unwrap_or(&[]);
        for &r in &rows {
            let Some(cell) = cells.get(r) else { continue };
            let s = cell.as_str().unwrap_or_default();
            if !levels.contains(&s) {
                return Err(frame_no(format!(
                    "column {} row {} is {}, not one of the fit's levels",
                    render::text(n),
                    r + 1,
                    render::value(cell)
                )));
            }
        }
    }
    let mut new_w = vec![1.0; rows.len()];
    if let Some(i) = wi {
        let cells = fr.cols.get(i).map(Vec::as_slice).unwrap_or(&[]);
        for (j, &r) in rows.iter().enumerate() {
            let Some(cell) = cells.get(r) else { continue };
            let x = code::cell_value(cell)?;
            if x <= 0.0 {
                let aside = if x == 0.0 {
                    "; frame.filter sets a row aside"
                } else {
                    ""
                };
                return Err(opts::refuse(
                    W,
                    format!("weights row {} is {}{aside}", r + 1, render::value(cell)),
                ));
            }
            new_w[j] = x;
        }
    }
    // From step 7: no rows left, and the cap.
    if rows.is_empty() {
        return Err(frame_no("no rows".into()));
    }
    let k = c.columns.len();
    if (rows.len() as u64).saturating_mul(k as u64) > CAP {
        return Err(Refusal::Cap {
            what: "the design's cells".into(),
            cap: CAP,
        }
        .error(W));
    }
    let x = code::cells(W, &fr, &c.columns, &rows)?;
    let offset: Vec<f64> = match &f.spec.offset {
        Some(n) => {
            let cells = fr.cols.get(index(&fr, n)?).map(Vec::as_slice).unwrap_or(&[]);
            rows.iter()
                .map(|&r| cells.get(r).map_or(Ok(0.0), code::cell_value))
                .collect::<Result<_, _>>()?
        }
        None => vec![0.0; rows.len()],
    };
    let ask = Ask {
        scale,
        interval,
        conf,
    };
    let answer = predict(&f, &x, &offset, &rows, &new_w, ask)?;
    let mut m = answer;
    m.insert(
        "dropped".to_string(),
        BundValue::int(i64::try_from(fr.nrow - rows.len()).unwrap_or(i64::MAX)),
    );
    vm.push(BundValue::map(m));
    Ok(())
}

/// What the options ask for.
#[derive(Clone, Copy)]
struct Ask {
    scale: Scale,
    interval: Interval,
    conf: f64,
}

/// The predictions, their standard errors and intervals.
fn predict(
    f: &Fit,
    x: &[Vec<f64>],
    offset: &[f64],
    rows: &[usize],
    new_w: &[f64],
    ask: Ask,
) -> Result<BTreeMap<String, BundValue>, Error> {
    let Ask {
        scale,
        interval,
        conf,
    } = ask;
    let estimated = f.family.estimated();
    let q = if interval == Interval::None {
        0.0
    } else {
        let p = (1.0 + conf) / 2.0;
        if estimated {
            Dist::T {
                df: f.df_residual as f64,
            }
            .quantile(W, p)?
        } else {
            Dist::Normal { mean: 0.0, sd: 1.0 }.quantile(W, p)?
        }
    };
    let link = f.link;
    let (mut fits, mut ses, mut lows, mut highs) = (vec![], vec![], vec![], vec![]);
    for (j, xr) in x.iter().enumerate() {
        let row = rows.get(j).map_or(j + 1, |r| r + 1);
        let eta: f64 = xr.iter().zip(&f.coef).map(|(a, b)| a * b).sum::<f64>() + offset[j];
        let mut var = 0.0;
        for (a, va) in xr.iter().zip(&f.vcov) {
            for (b, v) in xr.iter().zip(va) {
                var += a * v * b;
            }
        }
        let se = var.max(0.0).sqrt();
        let half = match interval {
            Interval::None => 0.0,
            Interval::Confidence => q * se,
            Interval::Prediction => q * (se * se + f.dispersion / new_w[j]).sqrt(),
        };
        let (lo, hi) = (eta - half, eta + half);
        let (fit, se_out, low, high) = match scale {
            Scale::Link => (eta, se, lo, hi),
            Scale::Response => {
                let out = |what: &str| {
                    domain(
                        W,
                        "frame",
                        format!(
                            "row {row}'s {what} the {} link's valid range; type \"link\" answers it",
                            link.name()
                        ),
                    )
                };
                if !link.valid_eta(eta) {
                    return Err(out("prediction is outside"));
                }
                if interval != Interval::None {
                    if link == Link::Inverse && lo < 0.0 && hi > 0.0 {
                        return Err(domain(
                            W,
                            "frame",
                            format!(
                                "row {row}'s interval crosses 0 under the inverse link; type \
                                 \"link\" answers it"
                            ),
                        ));
                    }
                    if !link.valid_eta(lo) {
                        return Err(out("lower end is outside"));
                    }
                    if !link.valid_eta(hi) {
                        return Err(out("upper end is outside"));
                    }
                }
                let (a, b) = (link.inverse(lo), link.inverse(hi));
                (
                    link.inverse(eta),
                    se * link.mu_eta(eta).abs(),
                    a.min(b),
                    a.max(b),
                )
            }
        };
        if !fit.is_finite() {
            return Err(overflow(format!("the prediction at row {row}")));
        }
        if !se_out.is_finite() {
            return Err(overflow(format!("the standard error at row {row}")));
        }
        fits.push(fit);
        ses.push(se_out);
        lows.push(low);
        highs.push(high);
    }
    let mut m = BTreeMap::new();
    m.insert("fit".to_string(), list(fits));
    m.insert("se".to_string(), list(ses));
    if interval != Interval::None {
        // D251's A3 on a LIST: an infinite end leaves its whole key out.
        if lows.iter().all(|v| v.is_finite()) {
            m.insert("lower".to_string(), list(lows));
        }
        if highs.iter().all(|v| v.is_finite()) {
            m.insert("upper".to_string(), list(highs));
        }
    }
    Ok(m)
}
