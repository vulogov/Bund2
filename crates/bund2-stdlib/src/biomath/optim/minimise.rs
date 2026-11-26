// SPDX-License-Identifier: GPL-2.0-or-later
// Contains code ported line for line from R 4.4.2's src/appl/optim.c (nmmin, vmmin) and src/library/stats/src/optim.c (optimhess),
// which is GPL-2.0-or-later (D279).

//! RFC-0019 §S10: the minimisers and the Hessian, `optim.nm`, `optim.bfgs`
//! and `optim.hessian`, and what `fit.mle` (§S11) minimises with.
//!
//! **Bund2's own** (N3, option A), each R 4.4.2's code ported line for
//! line:
//!
//! - [`nelder_mead`] is `nmmin` (`src/appl/optim.c:268-454`): α = 1,
//!   β = ½, γ = 2; the initial simplex steps every coordinate by
//!   0.1·max|x0ᵢ|, or 0.1 when that is 0 (`:313-318`), a vertex that
//!   rounds back onto x0 stepping ten times further; stopped when the
//!   highest vertex is within `reltol`·(|f(x0)| + `reltol`) of the lowest
//!   (`:302`, `:367`). `maxit` counts evaluations, 500 by default, and
//!   `funcount` past it is E110 where R answers `convergence = 1`; a
//!   shrink that does not reduce the simplex is E110 too (`:433-440`,
//!   D275 1A, 2A). `iterations` counts the simplex steps.
//! - [`bfgs`] is `vmmin` (`:108-260`): the backtracking line search, step
//!   shrunk by 0.2 with sufficient decrease at 10⁻⁴ (`:104-106`,
//!   `:182-196`), a coordinate unchanged when 10 + x does not change, the
//!   update skipped and the metric reset when Dᵢ ≤ 0, a periodic restart
//!   after 2k gradients; `maxit` 100 iterations by default, reached is
//!   E110 (N6), where `vmmin` answers `fail = 1`. R's `abstol` is −∞ and
//!   is not a key.
//! - [`gradient`] is `fmingr`'s numerical arm
//!   (`src/library/stats/src/optim.c:121-158`): central differences
//!   (f(x + hᵢ) − f(x − hᵢ))/(2hᵢ), `ndeps` hᵢ = 10⁻³ by default
//!   (`optim.R:36`).
//! - [`hessian`] is `optimhess` (`optim.c:414-473`): the central
//!   difference of that gradient along each coordinate at the same step,
//!   then symmetrised as ½(H + Hᵀ).
//!
//! **f** is a LAMBDA `x -- y`, x a LIST of k FLOATs, run through
//! [`call1_on`]: anything but one finite number is E112, so `nmmin`'s
//! substitution of 10³⁵ for a non-finite value never arises. x0 is a LIST
//! of k ≥ 1 finite numbers; k above 100 is E111. A difference quotient that
//! overflows is E114.
//!
//! **Work** (D39): each word counts its calls of f and refuses the
//! 10⁷ + 1st with E111, §S11's cap for `fit.mle`, so that a large `maxit`
//! is bounded too.
//!
//! **Options** (D275 3A): `optim.nm` takes `reltol` and `maxit`;
//! `optim.bfgs` `reltol`, `ndeps` and `maxit`; `optim.hessian` `ndeps`.
//! `reltol` is a positive number, √ε by default; `ndeps` a positive number
//! or a LIST of k; `maxit` an INTEGER ≥ 1. Any other key is E101.
//!
//! All three are opaque, plain only (§S1).

use std::collections::BTreeMap;

use bund2_api::{Error, Vm};
use bund2_value::{BOOL, BundValue, FLOAT, LIST, STRING};

use super::{count, opts_map, positive, unknown};
use crate::biomath::dynamics::{call1_on, lambda};
use crate::biomath::operand::{WIDTH, domain, finite, list, operands, whole};
use crate::biomath::refuse::Refusal;

/// The most parameters (§S10).
pub(crate) const K_CAP: u64 = 100;

/// The most calls of f one word makes (§S11, D39).
pub(crate) const CALL_CAP: u64 = 10_000_000;

/// Nelder–Mead's default `maxit`, in evaluations (`optim.R:43`).
pub(crate) const NM_MAXIT: u64 = 500;

/// BFGS's default `maxit`, in iterations (`optim.R:37`).
pub(crate) const BFGS_MAXIT: u64 = 100;

/// `ndeps`' default (`optim.R:36`).
const NDEPS: f64 = 1e-3;

/// `vmmin`'s constants (`appl/optim.c:104-106`).
const STEPREDN: f64 = 0.2;
const ACCTOL: f64 = 0.0001;
const RELTEST: f64 = 10.0;

/// `nmmin`'s coefficients (`optim.R:38`).
const ALPHA: f64 = 1.0;
const BETA: f64 = 0.5;
const GAMMA: f64 = 2.0;

/// The objective, as the minimisers see it.
pub(crate) type Objective<'a> = dyn FnMut(&[f64]) -> Result<f64, Error> + 'a;

/// A lambda `x -- y` with its calls counted against [`CALL_CAP`].
pub(crate) struct Counted {
    word: &'static str,
    f: Vec<BundValue>,
    /// −1 to minimise −f.
    sign: f64,
    calls: u64,
}

impl Counted {
    pub(crate) fn new(word: &'static str, f: Vec<BundValue>, sign: f64) -> Self {
        Counted {
            word,
            f,
            sign,
            calls: 0,
        }
    }

    pub(crate) fn eval(&mut self, vm: &mut dyn Vm, x: &[f64]) -> Result<f64, Error> {
        self.calls += 1;
        if self.calls > CALL_CAP {
            return Err(Refusal::Cap {
                what: "the count of lambda calls".into(),
                cap: CALL_CAP,
            }
            .error(self.word));
        }
        Ok(self.sign * call1_on(vm, self.word, &self.f, list(x.to_vec()))?)
    }
}

/// A minimum: where, the value there, the iterations and the evaluations.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Minimum {
    pub(crate) par: Vec<f64>,
    pub(crate) value: f64,
    pub(crate) iterations: u64,
    pub(crate) evaluations: u64,
}

/// The minimisers' and the Hessian's settings.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Settings {
    pub(crate) reltol: f64,
    pub(crate) ndeps: Vec<f64>,
    pub(crate) maxit: Option<u64>,
}

impl Settings {
    pub(crate) fn new(k: usize) -> Self {
        Settings {
            reltol: f64::EPSILON.sqrt(),
            ndeps: vec![NDEPS; k],
            maxit: None,
        }
    }
}

/// `nmmin`, line for line.
#[expect(
    clippy::needless_range_loop,
    reason = "nmmin's index form, kept line for line"
)]
pub(crate) fn nelder_mead(
    word: &str,
    f: &mut Objective<'_>,
    x0: &[f64],
    reltol: f64,
    maxit: u64,
) -> Result<Minimum, Error> {
    let n = x0.len();
    let mut bvec = x0.to_vec();
    let first = f(&bvec)?;
    let mut funcount: u64 = 1;
    let convtol = reltol * (first.abs() + reltol);
    let n1 = n + 1;
    let c = n + 2;
    // P: n + 1 rows (the coordinates, then f), n + 2 columns (the
    // vertices, then the centroid).
    let mut p = vec![vec![0.0f64; n + 2]; n + 1];
    p[n1 - 1][0] = first;
    for i in 0..n {
        p[i][0] = bvec[i];
    }
    let mut l = 1usize;
    let mut size = 0.0;
    let mut step = 0.0f64;
    for x in &bvec {
        if 0.1 * x.abs() > step {
            step = 0.1 * x.abs();
        }
    }
    if step == 0.0 {
        step = 0.1;
    }
    for j in 2..=n1 {
        for i in 0..n {
            p[i][j - 1] = bvec[i];
        }
        let mut trystep = step;
        while p[j - 2][j - 1] == bvec[j - 2] {
            p[j - 2][j - 1] = bvec[j - 2] + trystep;
            trystep *= 10.0;
        }
        size += trystep;
    }
    let mut oldsize = size;
    let mut calcvert = true;
    let mut iterations: u64 = 0;
    let mut degenerate = false;
    loop {
        if calcvert {
            for j in 0..n1 {
                if j + 1 != l {
                    for i in 0..n {
                        bvec[i] = p[i][j];
                    }
                    let v = f(&bvec)?;
                    funcount += 1;
                    p[n1 - 1][j] = v;
                }
            }
            calcvert = false;
        }
        let mut vl = p[n1 - 1][l - 1];
        let mut vh = vl;
        let mut h = l;
        for j in 1..=n1 {
            if j != l {
                let v = p[n1 - 1][j - 1];
                if v < vl {
                    l = j;
                    vl = v;
                }
                if v > vh {
                    h = j;
                    vh = v;
                }
            }
        }
        // R's `VL <= abstol` is never true at abstol −∞.
        if vh <= vl + convtol {
            break;
        }
        iterations += 1;
        for i in 0..n {
            let mut temp = -p[i][h - 1];
            for j in 0..n1 {
                temp += p[i][j];
            }
            p[i][c - 1] = temp / n as f64;
        }
        for i in 0..n {
            bvec[i] = (1.0 + ALPHA) * p[i][c - 1] - ALPHA * p[i][h - 1];
        }
        let mut v = f(&bvec)?;
        funcount += 1;
        let vr = v;
        if vr < vl {
            p[n1 - 1][c - 1] = v;
            for i in 0..n {
                let t = GAMMA * bvec[i] + (1.0 - GAMMA) * p[i][c - 1];
                p[i][c - 1] = bvec[i];
                bvec[i] = t;
            }
            v = f(&bvec)?;
            funcount += 1;
            if v < vr {
                for i in 0..n {
                    p[i][h - 1] = bvec[i];
                }
                p[n1 - 1][h - 1] = v;
            } else {
                for i in 0..n {
                    p[i][h - 1] = p[i][c - 1];
                }
                p[n1 - 1][h - 1] = vr;
            }
        } else {
            if vr < vh {
                for i in 0..n {
                    p[i][h - 1] = bvec[i];
                }
                p[n1 - 1][h - 1] = vr;
            }
            for i in 0..n {
                bvec[i] = (1.0 - BETA) * p[i][h - 1] + BETA * p[i][c - 1];
            }
            v = f(&bvec)?;
            funcount += 1;
            if v < p[n1 - 1][h - 1] {
                for i in 0..n {
                    p[i][h - 1] = bvec[i];
                }
                p[n1 - 1][h - 1] = v;
            } else if vr >= vh {
                calcvert = true;
                size = 0.0;
                for j in 0..n1 {
                    if j + 1 != l {
                        for i in 0..n {
                            p[i][j] = BETA * (p[i][j] - p[i][l - 1]) + p[i][l - 1];
                            size += (p[i][j] - p[i][l - 1]).abs();
                        }
                    }
                }
                if size < oldsize {
                    oldsize = size;
                } else {
                    degenerate = true;
                    break;
                }
            }
        }
        if funcount > maxit {
            break;
        }
    }
    // `if (funcount > maxit) *fail = 1;` after `fail = 10`.
    if funcount > maxit {
        return Err(Refusal::NoConvergence {
            method: "Nelder–Mead".into(),
            cap: maxit,
        }
        .error(word));
    }
    if degenerate {
        return Err(Refusal::NoConvergence {
            method: "Nelder–Mead, whose shrink did not reduce the simplex,".into(),
            cap: iterations,
        }
        .error(word));
    }
    Ok(Minimum {
        par: (0..n).map(|i| p[i][l - 1]).collect(),
        value: p[n1 - 1][l - 1],
        iterations,
        evaluations: funcount,
    })
}

/// `fmingr`'s numerical arm: central differences at `ndeps`.
pub(crate) fn gradient(
    word: &str,
    f: &mut Objective<'_>,
    p: &[f64],
    ndeps: &[f64],
) -> Result<Vec<f64>, Error> {
    let mut x = p.to_vec();
    let mut df = Vec::with_capacity(p.len());
    for (i, &eps) in ndeps.iter().enumerate().take(p.len()) {
        x[i] = p[i] + eps;
        let val1 = f(&x)?;
        x[i] = p[i] - eps;
        let val2 = f(&x)?;
        df.push(finite(word, (val1 - val2) / (2.0 * eps), || {
            format!("the gradient's difference quotient at {}", i + 1)
        })?);
        x[i] = p[i];
    }
    Ok(df)
}

/// `optimhess`: the gradient's central differences, symmetrised.
#[expect(
    clippy::needless_range_loop,
    reason = "optimhess's index form, kept line for line"
)]
pub(crate) fn hessian(
    word: &str,
    f: &mut Objective<'_>,
    par: &[f64],
    ndeps: &[f64],
) -> Result<Vec<Vec<f64>>, Error> {
    let n = par.len();
    let mut dpar = par.to_vec();
    let mut ans = vec![vec![0.0f64; n]; n];
    for (i, &eps) in ndeps.iter().enumerate().take(n) {
        dpar[i] += eps;
        let df1 = gradient(word, f, &dpar, ndeps)?;
        dpar[i] -= 2.0 * eps;
        let df2 = gradient(word, f, &dpar, ndeps)?;
        for j in 0..n {
            ans[i][j] = finite(word, (df1[j] - df2[j]) / (2.0 * eps), || {
                format!("the Hessian's difference quotient at [{},{}]", i + 1, j + 1)
            })?;
        }
        dpar[i] += eps;
    }
    for i in 0..n {
        for j in 0..i {
            let t = 0.5 * (ans[i][j] + ans[j][i]);
            ans[i][j] = t;
            ans[j][i] = t;
        }
    }
    Ok(ans)
}

/// `vmmin`, line for line, with every parameter free (`mask` all 1).
#[expect(
    clippy::manual_memcpy,
    reason = "vmmin's index form, kept line for line"
)]
pub(crate) fn bfgs(
    word: &str,
    f: &mut Objective<'_>,
    x0: &[f64],
    reltol: f64,
    ndeps: &[f64],
    maxit: u64,
) -> Result<Minimum, Error> {
    let n = x0.len();
    let mut b = x0.to_vec();
    let mut fv = f(&b)?;
    let mut fmin = fv;
    let mut funcount: u64 = 1;
    let mut gradcount: u64 = 1;
    let mut g = gradient(word, f, &b, ndeps)?;
    let mut iter: u64 = 1;
    let mut ilast = gradcount;
    // B, lower-triangular: B[i][j] for j ≤ i.
    let mut bm: Vec<Vec<f64>> = (0..n).map(|i| vec![0.0; i + 1]).collect();
    let mut t = vec![0.0f64; n];
    let mut xv = vec![0.0f64; n];
    let mut c = vec![0.0f64; n];
    loop {
        if ilast == gradcount {
            for (i, row) in bm.iter_mut().enumerate() {
                for x in row.iter_mut() {
                    *x = 0.0;
                }
                row[i] = 1.0;
            }
        }
        for i in 0..n {
            xv[i] = b[i];
            c[i] = g[i];
        }
        let mut gradproj = 0.0;
        for i in 0..n {
            let mut s = 0.0;
            for j in 0..=i {
                s -= bm[i][j] * g[j];
            }
            for j in i + 1..n {
                s -= bm[j][i] * g[j];
            }
            t[i] = s;
            gradproj += s * g[i];
        }
        let mut count;
        if gradproj < 0.0 {
            let mut steplength = 1.0;
            let mut accpoint = false;
            loop {
                count = 0;
                for i in 0..n {
                    b[i] = xv[i] + steplength * t[i];
                    if RELTEST + xv[i] == RELTEST + b[i] {
                        count += 1;
                    }
                }
                if count < n {
                    fv = f(&b)?;
                    funcount += 1;
                    accpoint = fv.is_finite() && fv <= fmin + gradproj * steplength * ACCTOL;
                    if !accpoint {
                        steplength *= STEPREDN;
                    }
                }
                if count == n || accpoint {
                    break;
                }
            }
            // `f > abstol` holds for every finite f at abstol −∞.
            let enough = (fv - fmin).abs() > reltol * (fmin.abs() + reltol);
            if !enough {
                count = n;
                fmin = fv;
            }
            if count < n {
                fmin = fv;
                g = gradient(word, f, &b, ndeps)?;
                gradcount += 1;
                iter += 1;
                let mut d1 = 0.0;
                for i in 0..n {
                    t[i] *= steplength;
                    c[i] = g[i] - c[i];
                    d1 += t[i] * c[i];
                }
                if d1 > 0.0 {
                    let mut d2 = 0.0;
                    for i in 0..n {
                        let mut s = 0.0;
                        for j in 0..=i {
                            s += bm[i][j] * c[j];
                        }
                        for j in i + 1..n {
                            s += bm[j][i] * c[j];
                        }
                        xv[i] = s;
                        d2 += s * c[i];
                    }
                    d2 = 1.0 + d2 / d1;
                    for i in 0..n {
                        for j in 0..=i {
                            bm[i][j] += (d2 * t[i] * t[j] - xv[i] * t[j] - t[i] * xv[j]) / d1;
                        }
                    }
                } else {
                    ilast = gradcount;
                }
            } else if ilast < gradcount {
                count = 0;
                ilast = gradcount;
            }
        } else {
            count = 0;
            if ilast == gradcount {
                count = n;
            } else {
                ilast = gradcount;
            }
        }
        if iter >= maxit {
            break;
        }
        if gradcount - ilast > 2 * n as u64 {
            ilast = gradcount;
        }
        if count == n && ilast == gradcount {
            break;
        }
    }
    if iter >= maxit {
        return Err(Refusal::NoConvergence {
            method: "BFGS".into(),
            cap: maxit,
        }
        .error(word));
    }
    Ok(Minimum {
        par: b,
        value: fmin,
        iterations: iter,
        evaluations: funcount,
    })
}

/// x0: a LIST of k ≥ 1 finite numbers, k at most 100.
pub(crate) fn start(word: &str, operand: &str, v: &BundValue) -> Result<Vec<f64>, Error> {
    let x = whole(word, operand, v)?;
    if x.is_empty() {
        return Err(domain(word, operand, "an empty LIST".into()));
    }
    if x.len() as u64 > K_CAP {
        return Err(Refusal::Cap {
            what: format!("{} parameters", x.len()),
            cap: K_CAP,
        }
        .error(word));
    }
    Ok(x)
}

/// `ndeps`: a positive number for every coordinate, or a LIST of k.
fn read_ndeps(word: &str, key: &str, x: &BundValue, k: usize) -> Result<Vec<f64>, Error> {
    if x.dt() == LIST {
        let bad = || domain(word, key, x.summary(WIDTH));
        let items = x.as_list().unwrap_or(&[]);
        if items.len() != k {
            return Err(bad());
        }
        let mut out = Vec::with_capacity(k);
        for item in items {
            let h = positive(word, key, item).map_err(|_| bad())?;
            out.push(h);
        }
        Ok(out)
    } else {
        Ok(vec![positive(word, key, x)?; k])
    }
}

/// What `fit.mle` reads beside the minimiser's settings.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MleKeys {
    pub(crate) nm: bool,
    pub(crate) n: Option<u64>,
    pub(crate) profile: bool,
    pub(crate) conf: f64,
}

/// Read an options MAP: `takes` lists the keys the word takes (D275 3A).
pub(crate) fn read_opts(
    word: &str,
    v: &BundValue,
    k: usize,
    takes: &[&str],
) -> Result<(Settings, MleKeys), Error> {
    let mut s = Settings::new(k);
    let mut m = MleKeys {
        nm: false,
        n: None,
        profile: false,
        conf: 0.95,
    };
    for (key, x) in opts_map(word, v)? {
        if !takes.contains(&key.as_str()) {
            return Err(unknown(word, key));
        }
        let bad = || domain(word, key, x.summary(WIDTH));
        match key.as_str() {
            "reltol" => s.reltol = positive(word, key, x)?,
            "ndeps" => s.ndeps = read_ndeps(word, key, x, k)?,
            "maxit" => s.maxit = Some(count(word, key, x)?),
            "method" => {
                m.nm = match x.as_str() {
                    Some(t) if x.dt() == STRING => match t.as_str() {
                        "bfgs" => false,
                        "nm" => true,
                        _ => return Err(bad()),
                    },
                    _ => return Err(bad()),
                }
            }
            "n" => m.n = Some(count(word, key, x)?),
            "profile" => {
                m.profile = match x.unboxed() {
                    BundValue::Bool(b, _) if x.dt() == BOOL => *b,
                    _ => return Err(bad()),
                }
            }
            "conf" => {
                // A FLOAT: no INTEGER lies in (0, 1).
                m.conf = match x.unboxed() {
                    BundValue::Float(c, _) if x.dt() == FLOAT && *c > 0.0 && *c < 1.0 => *c,
                    _ => return Err(bad()),
                }
            }
            _ => return Err(unknown(word, key)),
        }
    }
    Ok((s, m))
}

fn int(x: u64) -> BundValue {
    BundValue::int(i64::try_from(x).unwrap_or(i64::MAX))
}

/// A square MATRIX of FLOATs.
pub(crate) fn matrix(rows: Vec<Vec<f64>>) -> BundValue {
    BundValue::matrix(
        rows.into_iter()
            .map(|r| r.into_iter().map(BundValue::float).collect())
            .collect(),
    )
}

/// The minimum's keys.
fn answer(m: &Minimum) -> BTreeMap<String, BundValue> {
    let mut out = BTreeMap::new();
    out.insert("par".to_string(), list(m.par.clone()));
    out.insert("value".to_string(), BundValue::float(m.value));
    out.insert("iterations".to_string(), int(m.iterations));
    out.insert("evaluations".to_string(), int(m.evaluations));
    out
}

/// `optim.nm` (`f x0 opts -- MAP`).
pub(super) fn optim_nm(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "optim.nm";
    let v = operands(vm, W, 3)?;
    let f = lambda(W, "f", &v[0])?.to_vec();
    let x0 = start(W, "x0", &v[1])?;
    let (s, _) = read_opts(W, &v[2], x0.len(), &["reltol", "maxit"])?;
    let mut counted = Counted::new(W, f, 1.0);
    let mut obj = |x: &[f64]| counted.eval(vm, x);
    let m = nelder_mead(W, &mut obj, &x0, s.reltol, s.maxit.unwrap_or(NM_MAXIT))?;
    vm.push(BundValue::map(answer(&m)));
    Ok(())
}

/// `optim.bfgs` (`f x0 opts -- MAP`).
pub(super) fn optim_bfgs(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "optim.bfgs";
    let v = operands(vm, W, 3)?;
    let f = lambda(W, "f", &v[0])?.to_vec();
    let x0 = start(W, "x0", &v[1])?;
    let (s, _) = read_opts(W, &v[2], x0.len(), &["reltol", "ndeps", "maxit"])?;
    let mut counted = Counted::new(W, f, 1.0);
    let mut obj = |x: &[f64]| counted.eval(vm, x);
    let m = bfgs(
        W,
        &mut obj,
        &x0,
        s.reltol,
        &s.ndeps,
        s.maxit.unwrap_or(BFGS_MAXIT),
    )?;
    let g = gradient(W, &mut obj, &m.par, &s.ndeps)?;
    let mut out = answer(&m);
    out.insert("gradient".to_string(), list(g));
    vm.push(BundValue::map(out));
    Ok(())
}

/// `optim.hessian` (`f x opts -- MAP`).
pub(super) fn optim_hessian(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "optim.hessian";
    let v = operands(vm, W, 3)?;
    let f = lambda(W, "f", &v[0])?.to_vec();
    let x = start(W, "x", &v[1])?;
    let (s, _) = read_opts(W, &v[2], x.len(), &["ndeps"])?;
    let mut counted = Counted::new(W, f, 1.0);
    let mut obj = |p: &[f64]| counted.eval(vm, p);
    let g = gradient(W, &mut obj, &x, &s.ndeps)?;
    let h = hessian(W, &mut obj, &x, &s.ndeps)?;
    let mut out = BTreeMap::new();
    out.insert("gradient".to_string(), list(g));
    out.insert("hessian".to_string(), matrix(h));
    vm.push(BundValue::map(out));
    Ok(())
}

#[cfg(test)]
mod tests;
