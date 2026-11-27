// SPDX-License-Identifier: GPL-2.0-or-later
// Contains code ported line for line from R 4.6.2's src/library/stats/R/mlm.R
// (Pillai, Wilks, HL and Roy) and src/library/stats/R/manova.R
// (summary.manova's scaling and its ok test), which are GPL-2.0-or-later
// (D279).

//! RFC-0020 §S7: Hotelling's T² and one-way MANOVA, the words of M4.
//!
//! - **`stat.test.hotelling1`** (`X mu -- MAP`): T² = n(x̄ − μ)ᵀS⁻¹(x̄ − μ),
//!   S on n − 1, and F = (n − p)/(p(n − 1))·T² on p and n − p. `mu` is a
//!   LIST of p numbers (E102 otherwise); n − p < 1 is E101.
//! - **`stat.test.hotelling2`** (`X Y -- MAP`): T² = n₁n₂/(n₁ + n₂)·
//!   (x̄₁ − x̄₂)ᵀS_p⁻¹(x̄₁ − x̄₂), S_p pooled on n₁ + n₂ − 2, and
//!   F = (n₁ + n₂ − p − 1)/(p(n₁ + n₂ − 2))·T² on p and n₁ + n₂ − p − 1.
//!   The column counts must agree (E102); n₁ + n₂ − p − 1 < 1 is E101.
//!   Base R has no Hotelling word; for two groups `summary.manova`'s
//!   Hotelling–Lawley F is this F exactly (RFC-0020 §What R does).
//! - **The covariance is solved by Cholesky,** a pivot d_j = L_jj² at most
//!   10⁻⁸ of the largest diagonal entry E106 with `fit.mle`'s text, naming
//!   the pivot, as `fit/mle.rs`'s `inverse` tests it.
//! - **`stat.test.manova`** (`X groups opts -- MAP`) is one-way (V12),
//!   `summary.manova` over `manova(X ~ groups)` (`manova.R:38-128`). H is
//!   the between-groups SSCP and E the within. E is scaled as R scales it,
//!   sc = √diag(E) with sc_j = 1 where sc_j < 10⁻⁶·√(E_jj + H_jj)
//!   (`:98-102`); the rank test is the Cholesky of the scaled E, a pivot at
//!   most 10⁻⁷ of its largest diagonal entry E106 naming the column (R's
//!   own tolerance on the same scale, `:41`, `:103-106`). The eigenvalues
//!   of E⁻¹H are those of the symmetric L⁻¹HL⁻ᵀ, descending. `opts` takes
//!   `test`: `"pillai"` (the default), `"wilks"`, `"hotelling-lawley"` or
//!   `"roy"`, each statistic and F `mlm.R:363-406`'s line for line. One
//!   column, as R stops ("need multiple responses", `manova.R:61`), fewer
//!   than two groups, or N − g < p, is E101 before the statistic; where R's
//!   `ok` test fails (F < 0, or a df not positive, `:117-118`), so that R
//!   answers p NA, or the F is not finite, the word refuses with E101
//!   naming `test`. N·p·min(N, p) above 10¹⁰ is E111 (§S11). It answers
//!   `statistic`, `F`, `df1`, `df2`, `p`, `test`, `eig` and `method`.
//!
//! All three are fixed, plain and `.`; an F test answers `df1` and `df2`
//! (D272).

use std::cmp::Ordering;
use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm};
use bund2_value::{BundValue, LIST, METRICS, STRING};
use nalgebra::DMatrix;

use super::dist::Family;
use super::linalg::{matrix, sym_eigen};
use super::mva::{groups, opts_map, unknown};
use super::operand::{WIDTH, domain, operands, whole};
use super::refuse::Refusal;

/// Hotelling's pivot floor, `fit.mle`'s (`fit/mle.rs`, `PIVOT_FLOOR`).
const HOTELLING_FLOOR: f64 = 1e-8;

/// MANOVA's rank floor: R's `tol` (`manova.R:41`), on d_j's scale.
const MANOVA_FLOOR: f64 = 1e-7;

/// `stat.test.manova`'s cap on N·p·min(N, p) (§S11).
const MANOVA_CAP: u64 = 10_000_000_000;

type Rows = Vec<Vec<f64>>;

fn rows_of(m: &DMatrix<f64>) -> Rows {
    (0..m.nrows())
        .map(|i| m.row(i).iter().copied().collect())
        .collect()
}

fn means(x: &[Vec<f64>]) -> Vec<f64> {
    let p = x.first().map_or(0, Vec::len);
    let n = x.len() as f64;
    (0..p)
        .map(|j| x.iter().map(|r| r[j]).sum::<f64>() / n)
        .collect()
}

/// Σ (x − m)(x − m)ᵀ over the rows, added into `into`.
fn add_sscp(into: &mut [Vec<f64>], x: &[Vec<f64>], m: &[f64]) {
    for r in x {
        for i in 0..m.len() {
            let di = r[i] - m[i];
            for j in 0..m.len() {
                into[i][j] += di * (r[j] - m[j]);
            }
        }
    }
}

fn overflow(word: &str, what: &str) -> Error {
    Refusal::Overflow {
        expression: what.into(),
    }
    .error(word)
}

fn all_finite(word: &str, what: &str, a: &[Vec<f64>]) -> Result<(), Error> {
    if a.iter().flatten().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(overflow(word, what))
    }
}

/// The lower Cholesky factor of `a`, or the first pivot d_j = L_jj² at most
/// `floor` times the largest diagonal entry (or not above 0), 1-based, with
/// its value: `fit/mle.rs`'s `inverse`'s test.
#[expect(clippy::needless_range_loop, reason = "Cholesky's index form")]
fn cholesky(a: &[Vec<f64>], floor: f64) -> Result<Rows, (usize, f64)> {
    let n = a.len();
    let largest = (0..n).map(|i| a[i][i]).fold(f64::NEG_INFINITY, f64::max);
    let mut l = vec![vec![0.0f64; n]; n];
    for j in 0..n {
        let mut d = a[j][j];
        for k in 0..j {
            d -= l[j][k] * l[j][k];
        }
        if d.partial_cmp(&(floor * largest)) != Some(Ordering::Greater) || d <= 0.0 {
            return Err((j + 1, d));
        }
        let ljj = d.sqrt();
        l[j][j] = ljj;
        for i in j + 1..n {
            let mut s = a[i][j];
            for k in 0..j {
                s -= l[i][k] * l[j][k];
            }
            l[i][j] = s / ljj;
        }
    }
    Ok(l)
}

/// L⁻¹b, forward substitution.
fn forward(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let mut y = vec![0.0; b.len()];
    for i in 0..b.len() {
        let mut s = b[i];
        for k in 0..i {
            s -= l[i][k] * y[k];
        }
        y[i] = s / l[i][i];
    }
    y
}

/// dᵀS⁻¹d by S's Cholesky factor: |L⁻¹d|².
fn quadratic(word: &str, s: &[Vec<f64>], d: &[f64]) -> Result<f64, Error> {
    let l = cholesky(s, HOTELLING_FLOOR)
        .map_err(|(pivot, value)| Refusal::NotPositiveDefinite { pivot, value }.error(word))?;
    Ok(forward(&l, d).iter().map(|y| y * y).sum())
}

fn f_answer(
    word: &str,
    t2: f64,
    f: f64,
    df1: f64,
    df2: f64,
    method: &str,
) -> Result<BundValue, Error> {
    if !t2.is_finite() || !f.is_finite() {
        return Err(overflow(word, "the test statistic"));
    }
    let p = Family::F { df1, df2 }.tails(word, f)?.1;
    let mut m = BTreeMap::new();
    m.insert("statistic".to_string(), BundValue::float(t2));
    m.insert("F".to_string(), BundValue::float(f));
    m.insert("df1".to_string(), BundValue::float(df1));
    m.insert("df2".to_string(), BundValue::float(df2));
    m.insert("p".to_string(), BundValue::float(p.clamp(0.0, 1.0)));
    m.insert("method".to_string(), BundValue::str(method));
    Ok(BundValue::map(m))
}

fn stat_test_hotelling1(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.hotelling1";
    let v = operands(vm, W, 2)?;
    let x = rows_of(&matrix(W, "X", &v[0])?);
    let mu = match v[1].dt() {
        LIST | METRICS => whole(W, "mu", &v[1])?,
        _ => return Err(domain(W, "mu", v[1].summary(WIDTH))),
    };
    let (n, p) = (x.len(), x[0].len());
    if mu.len() != p {
        return Err(Refusal::Disagree {
            what: "mu and the columns of X".into(),
            n: mu.len(),
            m: p,
        }
        .error(W));
    }
    if n <= p {
        return Err(domain(W, "X", format!("{n} rows for {p} columns")));
    }
    let m = means(&x);
    let mut s = vec![vec![0.0; p]; p];
    add_sscp(&mut s, &x, &m);
    let nf = n as f64;
    let pf = p as f64;
    s.iter_mut().flatten().for_each(|c| *c /= nf - 1.0);
    all_finite(W, "the covariance of X", &s)?;
    let d: Vec<f64> = m.iter().zip(&mu).map(|(a, b)| a - b).collect();
    if !d.iter().all(|x| x.is_finite()) {
        return Err(overflow(W, "the mean of X less mu"));
    }
    let t2 = nf * quadratic(W, &s, &d)?;
    let f = (nf - pf) / (pf * (nf - 1.0)) * t2;
    vm.push(f_answer(
        W,
        t2,
        f,
        pf,
        nf - pf,
        "Hotelling's one-sample T-squared test",
    )?);
    Ok(())
}

fn stat_test_hotelling2(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.hotelling2";
    let v = operands(vm, W, 2)?;
    let x = rows_of(&matrix(W, "X", &v[0])?);
    let y = rows_of(&matrix(W, "Y", &v[1])?);
    let p = x[0].len();
    if y[0].len() != p {
        return Err(Refusal::Disagree {
            what: "the columns of X and Y".into(),
            n: p,
            m: y[0].len(),
        }
        .error(W));
    }
    let (n1, n2) = (x.len() as f64, y.len() as f64);
    let pf = p as f64;
    let df2 = n1 + n2 - pf - 1.0;
    if df2 < 1.0 {
        return Err(domain(
            W,
            "Y",
            format!("{} and {} rows for {p} columns", x.len(), y.len()),
        ));
    }
    let (m1, m2) = (means(&x), means(&y));
    let mut s = vec![vec![0.0; p]; p];
    add_sscp(&mut s, &x, &m1);
    add_sscp(&mut s, &y, &m2);
    s.iter_mut().flatten().for_each(|c| *c /= n1 + n2 - 2.0);
    all_finite(W, "the pooled covariance", &s)?;
    let d: Vec<f64> = m1.iter().zip(&m2).map(|(a, b)| a - b).collect();
    if !d.iter().all(|x| x.is_finite()) {
        return Err(overflow(W, "the difference of the means"));
    }
    let t2 = n1 * n2 / (n1 + n2) * quadratic(W, &s, &d)?;
    let f = df2 / (pf * (n1 + n2 - 2.0)) * t2;
    vm.push(f_answer(
        W,
        t2,
        f,
        pf,
        df2,
        "Hotelling's two-sample T-squared test",
    )?);
    Ok(())
}

// MANOVA.

#[derive(Clone, Copy, PartialEq)]
enum Test {
    Pillai,
    Wilks,
    HotellingLawley,
    Roy,
}

impl Test {
    fn name(self) -> &'static str {
        match self {
            Test::Pillai => "pillai",
            Test::Wilks => "wilks",
            Test::HotellingLawley => "hotelling-lawley",
            Test::Roy => "roy",
        }
    }

    /// `mlm.R:363-406`: the statistic, F, df1 and df2 from the eigenvalues,
    /// the term's df q and the residual df.
    fn stats(self, eig: &[f64], q: f64, df_res: f64) -> [f64; 4] {
        let p = eig.len() as f64;
        match self {
            Test::Pillai => {
                let test: f64 = eig.iter().map(|e| e / (1.0 + e)).sum();
                let s = p.min(q);
                let n = 0.5 * (df_res - p - 1.0);
                let m = 0.5 * ((p - q).abs() - 1.0);
                let tmp1 = 2.0 * m + s + 1.0;
                let tmp2 = 2.0 * n + s + 1.0;
                [test, (tmp2 / tmp1 * test) / (s - test), s * tmp1, s * tmp2]
            }
            Test::Wilks => {
                let test: f64 = eig.iter().map(|e| 1.0 / (1.0 + e)).product();
                let tmp1 = df_res - 0.5 * (p - q + 1.0);
                let tmp2 = (p * q - 2.0) / 4.0;
                let tmp3 = p * p + q * q - 5.0;
                let tmp3 = if tmp3 > 0.0 {
                    (((p * q) * (p * q) - 4.0) / tmp3).sqrt()
                } else {
                    1.0
                };
                [
                    test,
                    ((test.powf(-1.0 / tmp3) - 1.0) * (tmp1 * tmp3 - 2.0 * tmp2)) / p / q,
                    p * q,
                    tmp1 * tmp3 - 2.0 * tmp2,
                ]
            }
            Test::HotellingLawley => {
                let test: f64 = eig.iter().sum();
                let m = 0.5 * ((p - q).abs() - 1.0);
                let n = 0.5 * (df_res - p - 1.0);
                let s = p.min(q);
                let tmp1 = 2.0 * m + s + 1.0;
                let tmp2 = 2.0 * (s * n + 1.0);
                [test, (tmp2 * test) / s / s / tmp1, s * tmp1, tmp2]
            }
            Test::Roy => {
                let test = eig.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let tmp1 = p.max(q);
                let tmp2 = df_res - tmp1 + q;
                [test, (tmp2 * test) / tmp1, tmp1, tmp2]
            }
        }
    }
}

fn stat_test_manova(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.manova";
    let v = operands(vm, W, 3)?;
    let x = rows_of(&matrix(W, "X", &v[0])?);
    let (n, p) = (x.len(), x[0].len());
    let (group, labels) = groups(W, &v[1], n)?;
    let mut test = Test::Pillai;
    for (k, o) in opts_map(W, &v[2])? {
        match k.as_str() {
            "test" => {
                test = match (o.dt(), o.as_str().as_deref()) {
                    (STRING, Some("pillai")) => Test::Pillai,
                    (STRING, Some("wilks")) => Test::Wilks,
                    (STRING, Some("hotelling-lawley")) => Test::HotellingLawley,
                    (STRING, Some("roy")) => Test::Roy,
                    _ => return Err(domain(W, k, o.summary(WIDTH))),
                }
            }
            _ => return Err(unknown(W, k)),
        }
    }
    let work = (n as u128) * (p as u128) * (n.min(p) as u128);
    if work > u128::from(MANOVA_CAP) {
        return Err(Refusal::Cap {
            what: format!("N·p·min(N, p) = {work}, for a {n} × {p} MATRIX,"),
            cap: MANOVA_CAP,
        }
        .error(W));
    }
    if p < 2 {
        return Err(domain(
            W,
            "X",
            "one column, where MANOVA needs two or more".into(),
        ));
    }
    let g = labels.len();
    if g < 2 {
        return Err(domain(W, "groups", "one group".into()));
    }
    if n - g < p {
        return Err(domain(
            W,
            "X",
            format!("{n} rows in {g} groups for {p} columns"),
        ));
    }
    let mut members: Vec<Rows> = vec![Vec::new(); g];
    for (r, gi) in x.iter().zip(&group) {
        members[*gi].push(r.clone());
    }
    let grand = means(&x);
    let mut e = vec![vec![0.0; p]; p];
    let mut h = vec![vec![0.0; p]; p];
    for rows in &members {
        let m = means(rows);
        add_sscp(&mut e, rows, &m);
        let k = rows.len() as f64;
        for i in 0..p {
            for j in 0..p {
                h[i][j] += k * (m[i] - grand[i]) * (m[j] - grand[j]);
            }
        }
    }
    all_finite(W, "the SSCP matrices", &e)?;
    all_finite(W, "the SSCP matrices", &h)?;
    // `manova.R:98-102`.
    let sc: Vec<f64> = (0..p)
        .map(|j| {
            let s = e[j][j].sqrt();
            if s < (e[j][j] + h[j][j]).sqrt() * 1e-6 {
                1.0
            } else {
                s
            }
        })
        .collect();
    let scale = |a: &[Vec<f64>]| -> Rows {
        (0..p)
            .map(|i| (0..p).map(|j| a[i][j] / sc[i] / sc[j]).collect())
            .collect()
    };
    let (es, hs) = (scale(&e), scale(&h));
    let l = cholesky(&es, MANOVA_FLOOR).map_err(|(pivot, _)| {
        Refusal::SingularColumn {
            column: pivot.to_string(),
        }
        .error(W)
    })?;
    // L⁻¹ Hs L⁻ᵀ: the columns of L⁻¹Hs, then the rows of that.
    let mut a = vec![vec![0.0; p]; p];
    for j in 0..p {
        let col: Vec<f64> = (0..p).map(|i| hs[i][j]).collect();
        let y = forward(&l, &col);
        for i in 0..p {
            a[i][j] = y[i];
        }
    }
    let mut m = DMatrix::zeros(p, p);
    for (i, row) in a.iter().enumerate() {
        let y = forward(&l, row);
        for (j, yj) in y.into_iter().enumerate() {
            m[(i, j)] = yj;
        }
    }
    let m = (&m + m.transpose()) * 0.5;
    if m.iter().any(|x| !x.is_finite()) {
        return Err(overflow(W, "E⁻¹H"));
    }
    let (mut eig, _) = sym_eigen(W, m)?;
    eig.reverse();
    let q = (g - 1) as f64;
    let df_res = (n - g) as f64;
    let [stat, f, df1, df2] = test.stats(&eig, q, df_res);
    // `manova.R:117-118`'s `ok`, and a finite F.
    if !(f >= 0.0 && df1 > 0.0 && df2 > 0.0 && f.is_finite() && stat.is_finite()) {
        return Err(domain(
            W,
            "test",
            format!("{:?}, whose F is {f} on {df1} and {df2} here", test.name()),
        ));
    }
    let pv = Family::F { df1, df2 }.tails(W, f)?.1;
    let mut out = BTreeMap::new();
    out.insert("statistic".to_string(), BundValue::float(stat));
    out.insert("F".to_string(), BundValue::float(f));
    out.insert("df1".to_string(), BundValue::float(df1));
    out.insert("df2".to_string(), BundValue::float(df2));
    out.insert("p".to_string(), BundValue::float(pv.clamp(0.0, 1.0)));
    out.insert("test".to_string(), BundValue::str(test.name()));
    out.insert(
        "eig".to_string(),
        BundValue::list(eig.into_iter().map(BundValue::float).collect()),
    );
    out.insert("method".to_string(), BundValue::str("One-way MANOVA"));
    vm.push(BundValue::map(out));
    Ok(())
}

/// Register §S7's words, each with its `.` form (RFC-0015 §S3).
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
    word!("stat.test.hotelling1", stat_test_hotelling1, 2);
    word!("stat.test.hotelling2", stat_test_hotelling2, 2);
    word!("stat.test.manova", stat_test_manova, 3);
}

/// §S7's words with their `.` forms.
pub const WORDS: [&str; 6] = [
    "stat.test.hotelling1",
    "stat.test.hotelling1.",
    "stat.test.hotelling2",
    "stat.test.hotelling2.",
    "stat.test.manova",
    "stat.test.manova.",
];

#[cfg(test)]
mod tests;
