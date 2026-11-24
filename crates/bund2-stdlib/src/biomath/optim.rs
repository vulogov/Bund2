//! RFC-0019 §S6: roots and quadrature over a lambda; §S10's minimisers and
//! Hessian are [`minimise`]'s.
//!
//! **`optim.root`** (`f lower upper opts -- MAP`) is Brent's method as R's
//! `zeroin` (R 4.4.2, `src/library/stats/src/zeroin.c`, `R_zeroin2`),
//! ported step for step, with R's `uniroot` around it
//! (`src/library/stats/R/nlm.R`): the interval extended first under
//! `extend` ("no", "yes", "downX" or "upX", R's `extendInt`), each end moving
//! by δ = 0.01·max(10⁻⁴, |end|), δ doubling each step, the steps counted
//! against `maxiter` and added to Brent's iterations in the answer. It
//! answers `root`, `f.root`, `iterations` and `estim.prec`, as `uniroot`
//! does. The tolerance defaults to 10⁻¹²·max(1, |lower|, |upper|) (N7),
//! not R's ε^¼.
//!
//! **`optim.integrate`** (`f lower upper opts -- MAP`) is QUADPACK's QAGS,
//! or QAGI when an end is the STRING `"inf"` or `"-inf"` (N8), as R's
//! `integrate` ([`super::quadpack`]). It answers `value`, `abs.error` and
//! `subdivisions`. `rel.tol` defaults to 10⁻¹⁰ (N7), `abs.tol` to `rel.tol`,
//! `subdivisions` to 100. Ends in either order are integrated as written,
//! ∫ from lower to upper; equal infinite ends integrate to 0.
//!
//! **Refusals.** f is a LAMBDA `x -- y`, run by [`super::dynamics::call1`];
//! anything but one finite number left on top is E112. The same sign at both
//! ends with `extend` "no" is E101; an extension that finds no sign change
//! within `maxiter` steps, or whose end overflows, is E110 for "the search
//! for a sign change"; Brent's method at `maxiter` is E110 for `optim.root`.
//! QUADPACK's `ier` other than 0 — the subdivisions spent short of the
//! tolerance, and also roundoff, a bad integrand or divergence, each of
//! which R's `integrate` stops on — is E110 for `optim.integrate` at its
//! `subdivisions`.
//!
//! **Effects.** Both call a lambda, so both are opaque, plain only (B10).

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, MAP, STRING};

use super::dynamics::{call1, lambda};
use super::operand::{WIDTH, domain, operands, real};
use super::quadpack::{self, Range};
use super::refuse::Refusal;

/// `optim.root`'s default `maxiter`, as `uniroot`'s.
pub(super) const MAXITER: u64 = 1000;

/// `extend`, as R's `extendInt`: `Sig` NULL, −1, 0 or 1 in `uniroot`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Extend {
    No,
    Yes,
    DownX,
    UpX,
}

/// An options MAP: E101 for anything but a MAP, an unknown key naming
/// `opts`, a value of the wrong type or range naming its key.
fn opts_map<'a>(word: &str, v: &'a BundValue) -> Result<&'a BTreeMap<String, BundValue>, Error> {
    match v.as_map() {
        Some(m) if v.dt() == MAP => Ok(m),
        _ => Err(domain(word, "opts", v.summary(WIDTH))),
    }
}

fn unknown(word: &str, k: &str) -> Error {
    domain(word, "opts", format!("an unknown key {k:?}"))
}

/// A positive finite number under `key`.
fn positive(word: &str, key: &str, x: &BundValue) -> Result<f64, Error> {
    let y = real(word, key, x)?;
    if y > 0.0 {
        Ok(y)
    } else {
        Err(domain(word, key, x.summary(WIDTH)))
    }
}

/// A positive INTEGER under `key`.
fn count(word: &str, key: &str, x: &BundValue) -> Result<u64, Error> {
    match x.unboxed() {
        BundValue::Int(i, _) if *i >= 1 => Ok(i.unsigned_abs()),
        _ => Err(domain(word, key, x.summary(WIDTH))),
    }
}

/// `optim.root`'s options: `tol`, `maxiter`, `extend`.
fn root_opts(
    word: &str,
    v: &BundValue,
    lower: f64,
    upper: f64,
) -> Result<(f64, u64, Extend), Error> {
    let mut tol = 1e-12 * 1f64.max(lower.abs()).max(upper.abs());
    let mut maxiter = MAXITER;
    let mut extend = Extend::No;
    for (k, x) in opts_map(word, v)? {
        match k.as_str() {
            "tol" => tol = positive(word, k, x)?,
            "maxiter" => maxiter = count(word, k, x)?,
            "extend" => {
                extend = match x.as_str() {
                    Some(s) if x.dt() == STRING => match s.as_str() {
                        "no" => Extend::No,
                        "yes" => Extend::Yes,
                        "downX" => Extend::DownX,
                        "upX" => Extend::UpX,
                        _ => return Err(domain(word, k, x.summary(WIDTH))),
                    },
                    _ => return Err(domain(word, k, x.summary(WIDTH))),
                }
            }
            _ => return Err(unknown(word, k)),
        }
    }
    Ok((tol, maxiter, extend))
}

/// Both of a pair strictly on one side of 0.
fn same_sign(a: f64, b: f64) -> bool {
    (a > 0.0 && b > 0.0) || (a < 0.0 && b < 0.0)
}

/// An interval after `uniroot`'s extension: its ends, f at each, and the
/// steps taken.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Bracket {
    pub(super) lower: f64,
    pub(super) f_lower: f64,
    pub(super) upper: f64,
    pub(super) f_upper: f64,
    pub(super) steps: u64,
}

/// Why an extension stopped short.
#[derive(Debug)]
pub(super) enum Stop<E> {
    /// f's own failure.
    F(E),
    /// No sign change within `maxiter` steps, or an end that overflowed.
    Search,
    /// The same sign at both ends with `extend` "no".
    SameSign,
}

/// `uniroot`'s extension (`nlm.R`), R's steps exactly; f is never asked
/// at a non-finite end, which is the search's failure instead.
pub(super) fn extend<E>(
    f: &mut dyn FnMut(f64) -> Result<f64, E>,
    b: Bracket,
    extend: Extend,
    maxiter: u64,
) -> Result<Bracket, Stop<E>> {
    let Bracket {
        mut lower,
        f_lower: mut fl,
        mut upper,
        f_upper: mut fu,
        ..
    } = b;
    let sig = match extend {
        Extend::No => 0.0,
        Extend::DownX => -1.0,
        Extend::UpX => 1.0,
        Extend::Yes => f64::NAN,
    };
    let do_x = match extend {
        Extend::Yes => same_sign(fl, fu),
        Extend::No => false,
        _ => sig * fl > 0.0 || sig * fu < 0.0,
    };
    let mut it = 0u64;
    let delta = |u: f64| 0.01 * 1e-4f64.max(u.abs());
    let step = |it: &mut u64| {
        *it += 1;
        if *it > maxiter {
            Err(Stop::Search)
        } else {
            Ok(())
        }
    };
    if do_x {
        if extend == Extend::Yes {
            // Both ends at once.
            let (mut dl, mut du) = (delta(lower), delta(upper));
            // An end where f is NaN steps back and quarters its δ, as R's
            // `is.na` arm; a lambda never answers NaN (E112), so only §S4's
            // power equations, NaN outside their domains, reach it.
            while same_sign(fl, fu) {
                step(&mut it)?;
                let (ol, of) = (lower, fl);
                lower -= dl;
                if !lower.is_finite() {
                    return Err(Stop::Search);
                }
                fl = f(lower).map_err(Stop::F)?;
                if fl.is_nan() {
                    lower = ol;
                    fl = of;
                    dl /= 4.0;
                }
                let (ou, of) = (upper, fu);
                upper += du;
                if !upper.is_finite() {
                    return Err(Stop::Search);
                }
                fu = f(upper).map_err(Stop::F)?;
                if fu.is_nan() {
                    upper = ou;
                    fu = of;
                    du /= 4.0;
                }
                dl *= 2.0;
                du *= 2.0;
            }
        } else {
            // sig·f(lower) ≤ 0 and sig·f(upper) ≥ 0, one end at a time.
            let mut d = delta(lower);
            while sig * fl > 0.0 {
                step(&mut it)?;
                lower -= d;
                if !lower.is_finite() {
                    return Err(Stop::Search);
                }
                fl = f(lower).map_err(Stop::F)?;
                d *= 2.0;
            }
            let mut d = delta(upper);
            while sig * fu < 0.0 {
                step(&mut it)?;
                upper += d;
                if !upper.is_finite() {
                    return Err(Stop::Search);
                }
                fu = f(upper).map_err(Stop::F)?;
                d *= 2.0;
            }
        }
    }
    // R's `sign(f.lower) * sign(f.upper) <= 0`, which NaN fails.
    if same_sign(fl, fu) || fl.is_nan() || fu.is_nan() {
        return Err(if do_x { Stop::Search } else { Stop::SameSign });
    }
    Ok(Bracket {
        lower,
        f_lower: fl,
        upper,
        f_upper: fu,
        steps: it,
    })
}

/// Brent's answer: the root, f there, the iterations, |c − b|; `converged`
/// false when `maxit` ran out, as `R_zeroin2`'s `*Maxit = -1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Zero {
    pub(super) root: f64,
    pub(super) f_root: f64,
    pub(super) iterations: u64,
    pub(super) estim_prec: f64,
    pub(super) converged: bool,
}

/// `R_zeroin2` on [ax, bx] with f(ax) = fa and f(bx) = fb, line for line.
pub(super) fn zeroin<E>(
    f: &mut dyn FnMut(f64) -> Result<f64, E>,
    ax: f64,
    bx: f64,
    fa: f64,
    fb: f64,
    tol: f64,
    maxit: u64,
) -> Result<Zero, E> {
    let eps = f64::EPSILON;
    let (mut a, mut b) = (ax, bx);
    let (mut fa, mut fb) = (fa, fb);
    let (mut c, mut fc) = (a, fa);
    if fa == 0.0 {
        return Ok(Zero {
            root: a,
            f_root: fa,
            iterations: 0,
            estim_prec: 0.0,
            converged: true,
        });
    }
    if fb == 0.0 {
        return Ok(Zero {
            root: b,
            f_root: fb,
            iterations: 0,
            estim_prec: 0.0,
            converged: true,
        });
    }
    // `maxit = *Maxit + 1`, and `while (maxit--)`.
    let mut left = maxit.saturating_add(1);
    while left > 0 {
        left -= 1;
        let prev_step = b - a;
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        let tol_act = 2.0 * eps * b.abs() + tol / 2.0;
        let mut new_step = (c - b) / 2.0;
        if new_step.abs() <= tol_act || fb == 0.0 {
            return Ok(Zero {
                root: b,
                f_root: fb,
                iterations: maxit - left,
                estim_prec: (c - b).abs(),
                converged: true,
            });
        }
        if prev_step.abs() >= tol_act && fa.abs() > fb.abs() {
            let cb = c - b;
            let (mut p, mut q);
            if a == c {
                let t1 = fb / fa;
                p = cb * t1;
                q = 1.0 - t1;
            } else {
                let qq = fa / fc;
                let t1 = fb / fc;
                let t2 = fb / fa;
                p = t2 * (cb * qq * (qq - t1) - (b - a) * (t1 - 1.0));
                q = (qq - 1.0) * (t1 - 1.0) * (t2 - 1.0);
            }
            if p > 0.0 {
                q = -q;
            } else {
                p = -p;
            }
            if p < 0.75 * cb * q - (tol_act * q).abs() / 2.0 && p < (prev_step * q / 2.0).abs() {
                new_step = p / q;
            }
        }
        if new_step.abs() < tol_act {
            new_step = if new_step > 0.0 { tol_act } else { -tol_act };
        }
        a = b;
        fa = fb;
        b += new_step;
        fb = f(b)?;
        if (fb > 0.0 && fc > 0.0) || (fb < 0.0 && fc < 0.0) {
            c = a;
            fc = fa;
        }
    }
    Ok(Zero {
        root: b,
        f_root: fb,
        iterations: maxit,
        estim_prec: (c - b).abs(),
        converged: false,
    })
}

pub(super) fn search_failed(word: &str, maxiter: u64) -> Error {
    Refusal::NoConvergence {
        method: "the search for a sign change".into(),
        cap: maxiter,
    }
    .error(word)
}

fn optim_root(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "optim.root";
    let v = operands(vm, W, 4)?;
    let body = lambda(W, "f", &v[0])?.to_vec();
    let lower = real(W, "lower", &v[1])?;
    let upper = real(W, "upper", &v[2])?;
    if lower >= upper {
        return Err(domain(
            W,
            "upper",
            format!(
                "{}, not above lower {}",
                v[2].summary(WIDTH),
                v[1].summary(WIDTH)
            ),
        ));
    }
    let (tol, maxiter, how) = root_opts(W, &v[3], lower, upper)?;
    let mut f = |x: f64| call1(vm, W, &body, x);
    let f_lower = f(lower)?;
    let f_upper = f(upper)?;
    let start = Bracket {
        lower,
        f_lower,
        upper,
        f_upper,
        steps: 0,
    };
    let b = extend(&mut f, start, how, maxiter).map_err(|s| match s {
        Stop::F(e) => e,
        Stop::Search => search_failed(W, maxiter),
        Stop::SameSign => domain(W, "f", "the same sign at lower and upper".into()),
    })?;
    let z = zeroin(&mut f, b.lower, b.upper, b.f_lower, b.f_upper, tol, maxiter)?;
    if !z.converged {
        return Err(Refusal::NoConvergence {
            method: W.into(),
            cap: maxiter,
        }
        .error(W));
    }
    let iterations = z.iterations.saturating_add(b.steps);
    let mut m = BTreeMap::new();
    m.insert("root".to_string(), BundValue::float(z.root));
    m.insert("f.root".to_string(), BundValue::float(z.f_root));
    m.insert(
        "iterations".to_string(),
        BundValue::int(i64::try_from(iterations).unwrap_or(i64::MAX)),
    );
    m.insert("estim.prec".to_string(), BundValue::float(z.estim_prec));
    vm.push(BundValue::map(m));
    Ok(())
}

/// An end of `optim.integrate`: a finite number, or ±∞ written as the
/// STRING `"inf"` or `"-inf"` (N8).
fn end(word: &str, operand: &str, v: &BundValue) -> Result<f64, Error> {
    if v.dt() == STRING {
        return match v.as_str().as_deref() {
            Some("inf") => Ok(f64::INFINITY),
            Some("-inf") => Ok(f64::NEG_INFINITY),
            _ => Err(domain(word, operand, v.summary(WIDTH))),
        };
    }
    real(word, operand, v)
}

/// `optim.integrate`'s options: `rel.tol`, `abs.tol`, `subdivisions`.
fn integrate_opts(word: &str, v: &BundValue) -> Result<(f64, f64, u64), Error> {
    let mut rel = 1e-10;
    let mut abs = None;
    let mut subdivisions = 100;
    for (k, x) in opts_map(word, v)? {
        match k.as_str() {
            "rel.tol" | "abs.tol" => {
                let y = real(word, k, x)?;
                if y < 0.0 {
                    return Err(domain(word, k, x.summary(WIDTH)));
                }
                if k == "rel.tol" {
                    rel = y;
                } else {
                    abs = Some(y);
                }
            }
            "subdivisions" => subdivisions = count(word, k, x)?,
            _ => return Err(unknown(word, k)),
        }
    }
    let abs = abs.unwrap_or(rel);
    // `integrate`'s own check, "invalid parameter values".
    if abs <= 0.0 && rel < (50.0 * f64::EPSILON).max(0.5e-28) {
        return Err(domain(
            word,
            "rel.tol",
            format!("{rel}, below 50ε while abs.tol is 0"),
        ));
    }
    Ok((rel, abs, subdivisions))
}

/// The range from lower to upper, and the sign that orders it.
fn range_of(lower: f64, upper: f64) -> Option<(Range, f64)> {
    let (lo, hi, sign) = if lower <= upper {
        (lower, upper, 1.0)
    } else {
        (upper, lower, -1.0)
    };
    Some(match (lo.is_finite(), hi.is_finite()) {
        (true, true) => (Range::Finite(lower, upper), 1.0),
        // Equal infinite ends: an empty range.
        _ if lo == hi => return None,
        (true, false) => (Range::Infinite { bound: lo, inf: 1 }, sign),
        (false, true) => (Range::Infinite { bound: hi, inf: -1 }, sign),
        (false, false) => (Range::Infinite { bound: 0.0, inf: 2 }, sign),
    })
}

fn optim_integrate(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "optim.integrate";
    let v = operands(vm, W, 4)?;
    let body = lambda(W, "f", &v[0])?.to_vec();
    let lower = end(W, "lower", &v[1])?;
    let upper = end(W, "upper", &v[2])?;
    let (rel, abs, subdivisions) = integrate_opts(W, &v[3])?;
    let q = match range_of(lower, upper) {
        None => quadpack::Quad {
            value: 0.0,
            abs_error: 0.0,
            subdivisions: 0,
            ier: 0,
        },
        Some((range, sign)) => {
            let limit = usize::try_from(subdivisions).unwrap_or(usize::MAX);
            let mut f = |x: f64| call1(vm, W, &body, x);
            let mut q = quadpack::integrate(&mut f, range, abs, rel, limit)?;
            q.value *= sign;
            q
        }
    };
    if q.ier != 0 || !q.value.is_finite() {
        return Err(Refusal::NoConvergence {
            method: W.into(),
            cap: subdivisions,
        }
        .error(W));
    }
    let mut m = BTreeMap::new();
    m.insert("value".to_string(), BundValue::float(q.value));
    m.insert("abs.error".to_string(), BundValue::float(q.abs_error));
    m.insert(
        "subdivisions".to_string(),
        BundValue::int(i64::try_from(q.subdivisions).unwrap_or(i64::MAX)),
    );
    vm.push(BundValue::map(m));
    Ok(())
}

/// Register §S6's words: lambda-calling, so opaque and plain only (B10).
pub fn register(r: &mut Registry) {
    r.register_native(
        "optim.root",
        optim_root,
        StackEffect::opaque(4),
        WordKind::Sync,
    );
    r.register_native(
        "optim.integrate",
        optim_integrate,
        StackEffect::opaque(4),
        WordKind::Sync,
    );
    // §S10's, lambda-calling too.
    for (name, f) in [
        (
            "optim.nm",
            minimise::optim_nm as fn(&mut dyn Vm) -> Result<(), Error>,
        ),
        ("optim.bfgs", minimise::optim_bfgs),
        ("optim.hessian", minimise::optim_hessian),
    ] {
        r.register_native(name, f, StackEffect::opaque(3), WordKind::Sync);
    }
}

/// The `optim.` words, which have no `.` form: §S6's and §S10's.
pub const WORDS: [&str; 5] = [
    "optim.root",
    "optim.integrate",
    "optim.nm",
    "optim.bfgs",
    "optim.hessian",
];

pub(crate) mod minimise;

#[cfg(test)]
mod tests;
