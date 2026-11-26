//! RFC-0020: multivariate analysis of a data matrix, the `mva.` words.
//!
//! M1 adds §S3 and §S4:
//!
//! - **`mva.pca`** (`X opts -- MAP`) is R's `prcomp.default`
//!   (`src/library/stats/R/prcomp.R:21-58`). `opts` takes `center` (BOOL,
//!   default true), `scale` (BOOL, default false), `rank` (INTEGER ≥ 1) and
//!   `tol` (a number ≥ 0). The columns are centred, then each is divided by
//!   √(Σv²/max(1, n − 1)) of the column as it then is, as base R's `scale`
//!   (`src/library/base/R/scale.R:38-46`): the sd when centred, the root
//!   mean square when not. A scale of 0 is E101 naming the column, as R
//!   stops (`prcomp.R:30-31`). The SVD is `linalg.svd`'s, under §S1's sign
//!   rule; `sdev` = d/√max(1, n − 1), every one of them, and `rank` and
//!   `tol` cut only `rotation` and `x`, as R's (`prcomp.R:34-50`). It
//!   answers `sdev`, `rotation` (p × k), `x` (n × k), `center` and `scale`
//!   (a LIST, or false when not applied), and `prop` and `cumprop` over
//!   every `sdev`, unrounded (D278 choice 2). A matrix with no variance has
//!   no proportions, and is E101 (B12).
//! - **`mva.mds`** (`D opts -- MAP`) is R's `cmdscale`
//!   (`src/library/stats/R/cmdscale.R:19-88`). D passes the family's
//!   distance checks ([`super::phylo::distances`]) with a cap of 5000
//!   objects (§S11). `opts` takes `k` (INTEGER in 1 … n − 1, default 2) and
//!   `add` (BOOL, default false). D² is double-centred row then column, as
//!   `dblcen.c`; with `add`, Cailliez's constant is the largest real part
//!   of every eigenvalue of R's 2n × 2n matrix (`cmdscale.R:55-69`), by
//!   `linalg.eigenvalues.complex`'s Schur form, and D + c replaces D off
//!   the diagonal. −½ of the result is decomposed by `linalg.eigen.sym`'s
//!   function and reordered descending. Of the first k eigenvalues, those
//!   strictly above 0 are kept, R's rule (D278 choice 3), so `k` may answer
//!   fewer. It answers `points` (n × k, column j the jth eigenvector times
//!   √λⱼ, under §S1's sign rule), `eig` (all n), `gof` (R's two ratios),
//!   `ac` (0.0 without `add`) and `k`. A D of zeros has no goodness of fit,
//!   and is E101 (B12).
//!
//! Both are fixed, plain and `.`. Neither is a port of R's code: each
//! computes R's definition with `nalgebra`'s decompositions, so its file
//! stays Apache-2.0 (D279).

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm};
use bund2_value::{BOOL, BundValue, MAP};
use nalgebra::DMatrix;

use super::linalg::{
    answer, answer_list, matrix, overflow, sign_rule, spectrum, svd_of, sym_eigen,
};
use super::operand::{WIDTH, domain, list, operands, real};
use super::phylo::distances;

/// The most objects `mva.mds` reads (RFC-0020 §S11).
const MDS_CAP: u64 = 5_000;

// Options (RFC-0016 §S1).

pub(crate) fn opts_map<'a>(
    word: &str,
    v: &'a BundValue,
) -> Result<&'a BTreeMap<String, BundValue>, Error> {
    match v.as_map() {
        Some(m) if v.dt() == MAP => Ok(m),
        _ => Err(domain(word, "opts", v.summary(WIDTH))),
    }
}

pub(crate) fn unknown(word: &str, k: &str) -> Error {
    domain(word, "opts", format!("an unknown key {k:?}"))
}

pub(crate) fn flag(word: &str, key: &str, x: &BundValue) -> Result<bool, Error> {
    match x.unboxed() {
        BundValue::Bool(b, _) if x.dt() == BOOL => Ok(*b),
        _ => Err(domain(word, key, x.summary(WIDTH))),
    }
}

/// An INTEGER in `lo ..= hi` under `key`.
pub(crate) fn integer(
    word: &str,
    key: &str,
    x: &BundValue,
    lo: i64,
    hi: i64,
) -> Result<usize, Error> {
    match x.unboxed() {
        BundValue::Int(i, _) if (lo..=hi).contains(i) => Ok(i.unsigned_abs() as usize),
        _ => Err(domain(word, key, x.summary(WIDTH))),
    }
}

pub(crate) fn int(n: usize) -> BundValue {
    BundValue::int(i64::try_from(n).unwrap_or(i64::MAX))
}

/// Every cell finite, or E114 naming what overflowed.
fn finite(word: &str, what: &str, m: &DMatrix<f64>) -> Result<(), Error> {
    if m.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(overflow(word, what))
    }
}

// §S3.

struct PcaOpts {
    center: bool,
    scale: bool,
    rank: Option<usize>,
    tol: Option<f64>,
}

fn pca_opts(word: &str, v: &BundValue) -> Result<PcaOpts, Error> {
    let mut o = PcaOpts {
        center: true,
        scale: false,
        rank: None,
        tol: None,
    };
    for (k, x) in opts_map(word, v)? {
        match k.as_str() {
            "center" => o.center = flag(word, k, x)?,
            "scale" => o.scale = flag(word, k, x)?,
            "rank" => o.rank = Some(integer(word, k, x, 1, i64::MAX)?),
            "tol" => {
                let t = real(word, k, x)?;
                if t < 0.0 {
                    return Err(domain(word, k, x.summary(WIDTH)));
                }
                o.tol = Some(t);
            }
            _ => return Err(unknown(word, k)),
        }
    }
    Ok(o)
}

fn mva_pca(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "mva.pca";
    let v = operands(vm, W, 2)?;
    let mut y = matrix(W, "X", &v[0])?;
    let o = pca_opts(W, &v[1])?;
    let (n, p) = (y.nrows(), y.ncols());
    let denom = (n.max(2) - 1) as f64;
    let mut center = None;
    if o.center {
        let means: Vec<f64> = (0..p).map(|j| y.column(j).sum() / n as f64).collect();
        for (j, m) in means.iter().enumerate() {
            y.column_mut(j).add_scalar_mut(-m);
        }
        finite(W, "a centred column", &y)?;
        center = Some(means);
    }
    let mut scale = None;
    if o.scale {
        let mut scales = Vec::with_capacity(p);
        for j in 0..p {
            let s = (y.column(j).iter().map(|x| x * x).sum::<f64>() / denom).sqrt();
            if !s.is_finite() {
                return Err(overflow(W, "a column's scale"));
            }
            if s == 0.0 {
                return Err(domain(
                    W,
                    "X",
                    format!("column {}, whose scale is 0", j + 1),
                ));
            }
            y.column_mut(j).unscale_mut(s);
            scales.push(s);
        }
        scale = Some(scales);
    }
    let (d, _, rot) = svd_of(W, y.clone())?;
    let sdev: Vec<f64> = d.iter().map(|x| x / denom.sqrt()).collect();
    let r = sdev.len();
    let mut k = o.rank.map_or(r, |k| k.min(r));
    if let Some(tol) = o.tol {
        let first = sdev.first().copied().unwrap_or(0.0);
        let kept = sdev.iter().filter(|s| **s > first * tol).count();
        k = k.min(kept);
    }
    let rotation = rot.columns(0, k).into_owned();
    let scores = &y * &rotation;
    let total: f64 = sdev.iter().map(|s| s * s).sum();
    if !total.is_finite() {
        return Err(overflow(W, "the total variance"));
    }
    if total == 0.0 {
        return Err(domain(W, "X", "a MATRIX with no variance".into()));
    }
    let prop: Vec<f64> = sdev.iter().map(|s| s * s / total).collect();
    let cumprop: Vec<f64> = prop
        .iter()
        .scan(0.0, |acc, x| {
            *acc += x;
            Some(*acc)
        })
        .collect();
    let mut m = BTreeMap::new();
    m.insert(
        "sdev".to_string(),
        answer_list(W, "a standard deviation", sdev)?,
    );
    m.insert("rotation".to_string(), answer(W, "a loading", &rotation)?);
    m.insert("x".to_string(), answer(W, "a score", &scores)?);
    m.insert(
        "center".to_string(),
        center.map_or(BundValue::boolean(false), list),
    );
    m.insert(
        "scale".to_string(),
        scale.map_or(BundValue::boolean(false), list),
    );
    m.insert("prop".to_string(), answer_list(W, "a proportion", prop)?);
    m.insert(
        "cumprop".to_string(),
        answer_list(W, "a proportion", cumprop)?,
    );
    vm.push(BundValue::map(m));
    Ok(())
}

// §S4.

/// `dblcen.c`'s double centring: each row's mean subtracted, then each
/// column's.
fn double_centre(a: &mut DMatrix<f64>) {
    let n = a.nrows();
    for i in 0..n {
        let mean = a.row(i).sum() / n as f64;
        a.row_mut(i).add_scalar_mut(-mean);
    }
    for j in 0..n {
        let mean = a.column(j).sum() / n as f64;
        a.column_mut(j).add_scalar_mut(-mean);
    }
}

fn mva_mds(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "mva.mds";
    let v = operands(vm, W, 2)?;
    let rows = distances(W, "D", &v[0], MDS_CAP, "objects")?;
    let n = rows.len();
    let d = DMatrix::from_fn(n, n, |i, j| rows[i][j]);
    let mut k = 2;
    let mut add = false;
    let mut k_given = None;
    for (key, x) in opts_map(W, &v[1])? {
        match key.as_str() {
            "k" => k_given = Some(x),
            "add" => add = flag(W, key, x)?,
            _ => return Err(unknown(W, key)),
        }
    }
    // R stops on k outside 1 … n − 1, its default 2 included
    // (`cmdscale.R:50-51`).
    let hi = i64::try_from(n).unwrap_or(i64::MAX) - 1;
    k = integer(W, "k", k_given.unwrap_or(&BundValue::int(k as i64)), 1, hi)?;
    let mut x = d.map(|c| c * c);
    finite(W, "a squared distance", &x)?;
    double_centre(&mut x);
    let mut ac = 0.0;
    if add {
        let mut z = DMatrix::zeros(2 * n, 2 * n);
        let mut d2 = d.map(|c| 2.0 * c);
        double_centre(&mut d2);
        for i in 0..n {
            z[(n + i, i)] = -1.0;
            for j in 0..n {
                z[(i, n + j)] = -x[(i, j)];
                z[(n + i, n + j)] = d2[(i, j)];
            }
        }
        finite(W, "Cailliez's matrix", &z)?;
        ac = spectrum(W, z)?
            .iter()
            .map(|e| e.re)
            .fold(f64::NEG_INFINITY, f64::max);
        x = DMatrix::from_fn(n, n, |i, j| {
            if i == j {
                0.0
            } else {
                (d[(i, j)] + ac).powi(2)
            }
        });
        finite(W, "a squared distance plus the constant", &x)?;
        double_centre(&mut x);
    }
    let b = x.map(|c| -c / 2.0);
    let (ascending, vectors) = sym_eigen(W, b)?;
    let eig: Vec<f64> = ascending.iter().rev().copied().collect();
    let column = |c: usize| n - 1 - c;
    let kept: Vec<usize> = (0..k).filter(|&c| eig[c] > 0.0).collect();
    let mut points = DMatrix::from_fn(n, kept.len(), |i, j| {
        vectors[(i, column(kept[j]))] * eig[kept[j]].sqrt()
    });
    sign_rule(&mut points, &mut []);
    let sum_kept: f64 = kept.iter().map(|&c| eig[c]).sum();
    let abs_total: f64 = eig.iter().map(|e| e.abs()).sum();
    let pos_total: f64 = eig.iter().map(|e| e.max(0.0)).sum();
    if !abs_total.is_finite() {
        return Err(overflow(W, "the sum of the eigenvalues"));
    }
    if abs_total == 0.0 || pos_total == 0.0 {
        return Err(domain(W, "D", "a MATRIX of zero distances".into()));
    }
    let mut m = BTreeMap::new();
    m.insert("points".to_string(), answer(W, "a coordinate", &points)?);
    m.insert("eig".to_string(), answer_list(W, "an eigenvalue", eig)?);
    m.insert(
        "gof".to_string(),
        answer_list(
            W,
            "a goodness of fit",
            vec![sum_kept / abs_total, sum_kept / pos_total],
        )?,
    );
    m.insert("ac".to_string(), BundValue::float(ac));
    m.insert("k".to_string(), int(kept.len()));
    vm.push(BundValue::map(m));
    Ok(())
}

/// Register the `mva.` words, each with its `.` form (RFC-0015 §S3).
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native(
                $name,
                $f,
                StackEffect::fixed($n, 1),
                bund2_api::WordKind::Sync,
            );
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("mva.pca", mva_pca, 2);
    word!("mva.mds", mva_mds, 2);
}

/// The `mva.` words; each also has its `.` form.
pub const WORDS: [&str; 2] = ["mva.pca", "mva.mds"];

#[cfg(test)]
mod tests;
