// SPDX-License-Identifier: GPL-2.0-or-later
// Ports MASS's lda.default (method "moment") and predict.lda (method
// "plug-in"), MASS 7.3-66, R/lda.R:73-211 and :213-291.

//! RFC-0020 §S8 [V13]: linear discriminant analysis, `mva.lda` and
//! `mva.lda.predict`.
//!
//! - **`mva.lda`** (`X groups opts -- MAP`) is `lda.default` under
//!   `method = "moment"` (`lda.R:73-211`), line for line. The within-group
//!   residuals' sds (divisor n − 1, as `var`) below `tol` are E101 naming
//!   each such variable, as R stops (`:113-121`). The residuals, scaled to
//!   unit sd and by √(1/(n − g)), are decomposed by `linalg.svd`'s
//!   function; the singular values above `tol` are kept, and none is E101
//!   (`:153-161`). Fewer than p answers fewer discriminants, where R warns
//!   "variables are collinear" (RFC-0020 §S8 [author]). The group means
//!   about the prior-weighted mean, each row weighted by √(n·priorₖ/(g − 1)),
//!   are decomposed in the whitened space; the singular values above `tol`
//!   times the first are kept, and none is E101 (`:192-198`). `scaling` is
//!   the product, p × r, under §S1's sign rule. `opts` takes `prior` (a LIST
//!   of g non-negative numbers whose sum rounds to 1 at five places, as
//!   `:90`, in `lev`'s order), `tol` (a number ≥ 0, default 10⁻⁴) and
//!   `levels` (a LIST naming each label once, §S1). It answers `prior`,
//!   `counts`, `means` (g × p), `scaling`, `lev`, `svd` and `N`
//!   (`:207-209`).
//! - **`mva.lda.predict`** (`fit X opts -- MAP`) is `predict.lda`'s plug-in
//!   method (`:245-290`). `fit` is an `mva.lda` answer, checked for its keys
//!   and shapes (E101 naming `fit`); X has the fit's p columns (E102).
//!   `opts` takes `prior` (default the fit's, with `mva.lda`'s checks) and
//!   `dimen` (INTEGER ≥ 1, clamped to r as `:260`). X and the means are
//!   centred on the prior-weighted mean under the prior in use (`:255-258`).
//!   A group whose prior is 0 has posterior 0 directly, where R reaches it
//!   through log 0 (`:264`), and an exact tie takes the first group in
//!   `lev`'s order, where R's `max.col` breaks it at random (§S8 [author]).
//!   It answers `class` (a LIST of labels), `posterior` (n × g) and `x`
//!   (n × dimen).

use std::collections::BTreeMap;

use bund2_api::{Error, Vm};
use bund2_value::{BundValue, INTEGER, LIST, MAP, STRING};
use nalgebra::DMatrix;

use super::super::linalg::{answer, answer_list, matrix, overflow, sign_rule, svd_of};
use super::super::operand::{WIDTH, domain, operands, real};
use super::super::refuse::Refusal;
use super::{groups, int, integer, opts_map, unknown};

/// MASS's default `tol` (`lda.R:74`).
const TOL: f64 = 1.0e-4;

/// Two labels equal: both INTEGERs of one value, or both STRINGs of one text.
fn same(a: &BundValue, b: &BundValue) -> bool {
    match (a.dt(), b.dt()) {
        (INTEGER, INTEGER) => matches!(
            (a.unboxed(), b.unboxed()),
            (BundValue::Int(x, _), BundValue::Int(y, _)) if x == y
        ),
        (STRING, STRING) => a.as_str().is_some() && a.as_str() == b.as_str(),
        _ => false,
    }
}

fn items<'a>(word: &str, key: &str, v: &'a BundValue) -> Result<&'a [BundValue], Error> {
    match v.as_list() {
        Some(xs) if v.dt() == LIST => Ok(xs),
        _ => Err(domain(word, key, v.summary(WIDTH))),
    }
}

/// `levels`: a LIST naming each of `labels` once. Answers, for each place in
/// the order wanted, the index into `labels`.
fn levels(word: &str, v: &BundValue, labels: &[BundValue]) -> Result<Vec<usize>, Error> {
    let xs = items(word, "levels", v)?;
    if xs.len() != labels.len() {
        return Err(domain(
            word,
            "levels",
            format!(
                "{}, naming {} labels where groups has {}",
                v.summary(WIDTH),
                xs.len(),
                labels.len()
            ),
        ));
    }
    let mut perm = Vec::with_capacity(xs.len());
    for x in xs {
        match labels.iter().position(|l| same(l, x)) {
            Some(j) if !perm.contains(&j) => perm.push(j),
            Some(_) => {
                return Err(domain(
                    word,
                    "levels",
                    format!("{}, named twice", x.summary(WIDTH)),
                ));
            }
            None => {
                return Err(domain(
                    word,
                    "levels",
                    format!("{}, not a label of groups", x.summary(WIDTH)),
                ));
            }
        }
    }
    Ok(perm)
}

/// R's prior check (`lda.R:90-91`): g numbers, none negative, whose sum
/// rounds to 1 at five places. `round(s, 5)` is taken as s·10⁵ rounded half
/// away from zero; the two differ only at a sum within an ulp of 1 ± 5·10⁻⁶.
fn prior(word: &str, v: &BundValue, g: usize) -> Result<Vec<f64>, Error> {
    let xs = items(word, "prior", v)?;
    if xs.len() != g {
        return Err(domain(
            word,
            "prior",
            format!(
                "{}, of {} where there are {g} groups",
                v.summary(WIDTH),
                xs.len()
            ),
        ));
    }
    let mut out = Vec::with_capacity(g);
    for x in xs {
        let p = real(word, "prior", x)?;
        if p < 0.0 {
            return Err(domain(
                word,
                "prior",
                format!("{}, a negative prior", v.summary(WIDTH)),
            ));
        }
        out.push(p);
    }
    let s: f64 = out.iter().sum();
    if (s * 1e5).round() != 1e5 {
        return Err(domain(
            word,
            "prior",
            format!("{}, whose sum is {s}, not 1", v.summary(WIDTH)),
        ));
    }
    Ok(out)
}

/// The rows of `m` scaled by `w`.
fn weigh_rows(m: &mut DMatrix<f64>, w: &[f64]) {
    for (i, wi) in w.iter().enumerate() {
        m.row_mut(i).scale_mut(*wi);
    }
}

/// Σₖ priorₖ·meansₖ, a row.
fn weighted_mean(means: &DMatrix<f64>, prior: &[f64]) -> Vec<f64> {
    (0..means.ncols())
        .map(|j| (0..means.nrows()).map(|k| prior[k] * means[(k, j)]).sum())
        .collect()
}

/// The first `r` right singular vectors of `a` as aᵀuⱼ/dⱼ, the SVD's own
/// identity, from its u and d. Both of R's SVDs here can meet a zero
/// singular value: the between-group matrix always (it is g × rank of rank
/// g − 1), the within-group one when variables are collinear. There
/// `nalgebra`'s v can carry a trace of the null direction: on iris under
/// the prior (0.5, 0.3, 0.2), about 10⁻⁹ in v₁, where its u₁ and d₁ agree
/// with R's and with mpmath's to 10⁻¹⁵ (criterion 7's `fitp`).
fn right(a: &DMatrix<f64>, u: &DMatrix<f64>, d: &[f64], r: usize) -> DMatrix<f64> {
    let mut v = a.transpose() * u.columns(0, r);
    for (j, dj) in d.iter().take(r).enumerate() {
        v.column_mut(j).unscale_mut(*dj);
    }
    v
}

fn centre(m: &DMatrix<f64>, c: &[f64]) -> DMatrix<f64> {
    DMatrix::from_fn(m.nrows(), m.ncols(), |i, j| m[(i, j)] - c[j])
}

pub(super) fn mva_lda(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "mva.lda";
    let v = operands(vm, W, 3)?;
    let x = matrix(W, "X", &v[0])?;
    let (n, p) = (x.nrows(), x.ncols());
    let (mut group, mut labels) = groups(W, &v[1], n)?;
    let mut given_prior = None;
    let mut given_levels = None;
    let mut tol = TOL;
    for (k, o) in opts_map(W, &v[2])? {
        match k.as_str() {
            "prior" => given_prior = Some(o),
            "levels" => given_levels = Some(o),
            "tol" => {
                tol = real(W, k, o)?;
                if tol < 0.0 {
                    return Err(domain(W, k, o.summary(WIDTH)));
                }
            }
            _ => return Err(unknown(W, k)),
        }
    }
    if let Some(l) = given_levels {
        let perm = levels(W, l, &labels)?;
        let mut place = vec![0; perm.len()];
        for (new, old) in perm.iter().enumerate() {
            place[*old] = new;
        }
        for gi in group.iter_mut() {
            *gi = place[*gi];
        }
        labels = perm.iter().map(|j| labels[*j].clone()).collect();
    }
    let g = labels.len();
    if g < 2 {
        return Err(domain(
            W,
            "groups",
            "one group, where LDA needs two or more".into(),
        ));
    }
    if n <= g {
        return Err(domain(
            W,
            "groups",
            format!("{g} groups among {n} rows, where LDA needs more rows than groups"),
        ));
    }
    let mut counts = vec![0usize; g];
    for gi in &group {
        counts[*gi] += 1;
    }
    let prior = match given_prior {
        Some(pv) => prior(W, pv, g)?,
        None => counts.iter().map(|c| *c as f64 / n as f64).collect(),
    };

    // The group means and the within-group residuals (`lda.R:112-113`).
    let mut means: DMatrix<f64> = DMatrix::zeros(g, p);
    for (i, gi) in group.iter().enumerate() {
        for j in 0..p {
            means[(*gi, j)] += x[(i, j)];
        }
    }
    for (k, c) in counts.iter().enumerate() {
        means.row_mut(k).unscale_mut(*c as f64);
    }
    if means.iter().any(|m| !m.is_finite()) {
        return Err(overflow(W, "a group's mean"));
    }
    let resid = DMatrix::from_fn(n, p, |i, j| x[(i, j)] - means[(group[i], j)]);
    let mut f1 = Vec::with_capacity(p);
    for j in 0..p {
        let col = resid.column(j);
        let m = col.sum() / n as f64;
        let ss: f64 = col.iter().map(|r| (r - m) * (r - m)).sum();
        let s = (ss / (n - 1) as f64).sqrt();
        if !s.is_finite() {
            return Err(overflow(W, "a variable's within-group sd"));
        }
        f1.push(s);
    }
    let constant: Vec<String> = (0..p)
        // A sd of 0 is constant under any `tol`, 0 included, where R's 1/0
        // would reach `svd` as Inf and stop there.
        .filter(|j| f1[*j] < tol || f1[*j] == 0.0)
        .map(|j| (j + 1).to_string())
        .collect();
    match constant.len() {
        0 => {}
        1 => {
            return Err(domain(
                W,
                "X",
                format!("variable {}, constant within groups", constant[0]),
            ));
        }
        _ => {
            return Err(domain(
                W,
                "X",
                format!("variables {}, constant within groups", constant.join(" ")),
            ));
        }
    }

    // The within-group SVD, which whitens W (`lda.R:153-161`).
    let fac = (1.0 / (n - g) as f64).sqrt();
    let mut xw = resid;
    for (j, s) in f1.iter().enumerate() {
        xw.column_mut(j).scale_mut(fac / s);
    }
    if xw.iter().any(|c| !c.is_finite()) {
        return Err(overflow(W, "a whitened residual"));
    }
    let (d, uw, _) = svd_of(W, xw.clone())?;
    let rank = d.iter().filter(|s| **s > tol).count();
    if rank == 0 {
        return Err(domain(
            W,
            "X",
            "rank 0, its variables numerically constant".into(),
        ));
    }
    let vw = right(&xw, &uw, &d, rank);
    let scaling = DMatrix::from_fn(p, rank, |i, j| vw[(i, j)] / f1[i] / d[j]);

    // The between-group SVD (`lda.R:192-198`).
    let xbar = weighted_mean(&means, &prior);
    let mut xb = centre(&means, &xbar) * &scaling;
    let w: Vec<f64> = prior
        .iter()
        .map(|pk| (n as f64 * pk / (g - 1) as f64).sqrt())
        .collect();
    weigh_rows(&mut xb, &w);
    if xb.iter().any(|c| !c.is_finite()) {
        return Err(overflow(W, "a weighted group mean in the whitened space"));
    }
    let (d2, ub, _) = svd_of(W, xb.clone())?;
    let first = d2.first().copied().unwrap_or(0.0);
    let r = d2.iter().filter(|s| **s > tol * first).count();
    if r == 0 {
        return Err(domain(
            W,
            "groups",
            "groups whose means are numerically identical".into(),
        ));
    }
    let vb = right(&xb, &ub, &d2, r);
    let mut scaling = &scaling * &vb;
    sign_rule(&mut scaling, &mut []);

    let mut m = BTreeMap::new();
    m.insert("prior".to_string(), answer_list(W, "a prior", prior)?);
    m.insert(
        "counts".to_string(),
        BundValue::list(counts.into_iter().map(int).collect()),
    );
    m.insert("means".to_string(), answer(W, "a group's mean", &means)?);
    m.insert(
        "scaling".to_string(),
        answer(W, "a discriminant's coefficient", &scaling)?,
    );
    m.insert("lev".to_string(), BundValue::list(labels));
    m.insert(
        "svd".to_string(),
        answer_list(W, "a singular value", d2[..r].to_vec())?,
    );
    m.insert("N".to_string(), int(n));
    vm.push(BundValue::map(m));
    Ok(())
}

/// A fit as `mva.lda` answers it.
struct Fit {
    prior: Vec<f64>,
    means: DMatrix<f64>,
    scaling: DMatrix<f64>,
    lev: Vec<BundValue>,
    r: usize,
}

fn fit_of(word: &str, v: &BundValue) -> Result<Fit, Error> {
    let bad = |what: &str| domain(word, "fit", format!("a MAP whose {what}"));
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(word, "fit", v.summary(WIDTH))),
    };
    let get = |k: &str| m.get(k).ok_or_else(|| bad(&format!("{k} is missing")));
    let means = matrix(word, "fit", get("means")?)?;
    let scaling = matrix(word, "fit", get("scaling")?)?;
    let (g, p) = (means.nrows(), means.ncols());
    let r = scaling.ncols();
    if scaling.nrows() != p {
        return Err(bad("scaling's rows are not means' columns"));
    }
    let list_of = |k: &str, len: usize| -> Result<&[BundValue], Error> {
        let x = get(k)?;
        match x.as_list() {
            Some(xs) if x.dt() == LIST && xs.len() == len => Ok(xs),
            _ => Err(bad(&format!("{k} is not a LIST of {len}"))),
        }
    };
    let mut prior = Vec::with_capacity(g);
    for x in list_of("prior", g)? {
        let pk = real(word, "fit", x)?;
        if pk < 0.0 {
            return Err(bad("prior holds a negative number"));
        }
        prior.push(pk);
    }
    if (prior.iter().sum::<f64>() * 1e5).round() != 1e5 {
        return Err(bad("prior does not sum to 1"));
    }
    for c in list_of("counts", g)? {
        if !matches!(c.unboxed(), BundValue::Int(k, _) if *k >= 0) {
            return Err(bad("counts holds a value other than a count"));
        }
    }
    let lev = list_of("lev", g)?.to_vec();
    for x in list_of("svd", r)? {
        real(word, "fit", x)?;
    }
    if !matches!(get("N")?.unboxed(), BundValue::Int(k, _) if *k >= 1) {
        return Err(bad("N is not an INTEGER ≥ 1"));
    }
    Ok(Fit {
        prior,
        means,
        scaling,
        lev,
        r,
    })
}

pub(super) fn mva_lda_predict(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "mva.lda.predict";
    let v = operands(vm, W, 3)?;
    let fit = fit_of(W, &v[0])?;
    let x = matrix(W, "X", &v[1])?;
    let (g, p) = (fit.means.nrows(), fit.means.ncols());
    if x.ncols() != p {
        return Err(Refusal::Disagree {
            what: "the columns of X and of the fit's means".into(),
            n: x.ncols(),
            m: p,
        }
        .error(W));
    }
    let mut prior_in_use = fit.prior.clone();
    let mut dimen = fit.r;
    for (k, o) in opts_map(W, &v[2])? {
        match k.as_str() {
            "prior" => prior_in_use = prior(W, o, g)?,
            "dimen" => dimen = integer(W, k, o, 1, i64::MAX)?.min(fit.r),
            _ => return Err(unknown(W, k)),
        }
    }
    let n = x.nrows();
    let centre_at = weighted_mean(&fit.means, &prior_in_use);
    let scores = centre(&x, &centre_at) * &fit.scaling;
    let dm = centre(&fit.means, &centre_at) * &fit.scaling;
    if scores.iter().chain(dm.iter()).any(|c| !c.is_finite()) {
        return Err(overflow(W, "a discriminant score"));
    }
    let half: Vec<f64> = (0..g)
        .map(|k| 0.5 * (0..dimen).map(|j| dm[(k, j)] * dm[(k, j)]).sum::<f64>())
        .collect();
    let mut posterior: DMatrix<f64> = DMatrix::zeros(n, g);
    let mut class = Vec::with_capacity(n);
    for i in 0..n {
        let dist: Vec<Option<f64>> = (0..g)
            .map(|k| {
                (prior_in_use[k] > 0.0).then(|| {
                    half[k]
                        - prior_in_use[k].ln()
                        - (0..dimen).map(|j| scores[(i, j)] * dm[(k, j)]).sum::<f64>()
                })
            })
            .collect();
        let least = dist.iter().flatten().fold(f64::INFINITY, |a, b| a.min(*b));
        let e: Vec<f64> = dist
            .iter()
            .map(|d| d.map_or(0.0, |d| (-(d - least)).exp()))
            .collect();
        let total: f64 = e.iter().sum();
        let mut best = 0;
        for k in 0..g {
            posterior[(i, k)] = e[k] / total;
            if posterior[(i, k)] > posterior[(i, best)] {
                best = k;
            }
        }
        class.push(fit.lev[best].clone());
    }
    let mut m = BTreeMap::new();
    m.insert("class".to_string(), BundValue::list(class));
    m.insert(
        "posterior".to_string(),
        answer(W, "a posterior", &posterior)?,
    );
    m.insert(
        "x".to_string(),
        answer(
            W,
            "a discriminant score",
            &scores.columns(0, dimen).into_owned(),
        )?,
    );
    vm.push(BundValue::map(m));
    Ok(())
}

#[cfg(test)]
mod tests;
