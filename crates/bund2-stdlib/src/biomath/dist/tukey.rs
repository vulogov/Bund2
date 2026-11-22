//! RFC-0019 §S3: the studentized range, by §S6's integrator.
//!
//! Q = R/s, with R the range of `k` standard normals and s² an independent
//! χ²_ν/ν. Its cdf is the double integral
//!
//! P(Q ≤ q) = ∫ f_s(s) · W(q·s) ds,   W(w) = k ∫ φ(z) [Φ(z + w) − Φ(z)]^(k−1) dz,
//!
//! W being the probability that the range of `k` normals is at most w
//! (the smallest at z, the others within w above it). Both integrals are
//! QAGS ([`crate::biomath::quadpack`]) over pieces of a finite range
//! outside which the integrand's mass is provably negligible:
//!
//! - **z** runs over [−10, 9]. Below −10 the mass is at most k·Φ(−10) <
//!   10⁻¹⁹ for k ≤ 10⁴; above 9 it is at most Q(9)^k < 10⁻¹⁸, the smallest
//!   of k normals being above 9. A piece whose bound k·φ·min(Q(z₁),
//!   Φ(z₂ + w))^(k−1)·width is below 10⁻²⁰ is skipped.
//! - **s** runs over s_m ± 10/√ν, s_m = √((ν − 1)/ν) the density's mode,
//!   cut at 0. The density is ν-strongly log-concave (−(ln f)″ = (ν − 1)/s²
//!   + ν ≥ ν), so beyond 10/√ν it is below e^(−50) of its mode.
//!
//! The χ density is used unnormalised, exp((ν − 1)·ln(s/s_m) −
//! ν(s² − s_m²)/2), 1 at its mode, and the integral is divided by its own
//! integral over the same pieces: no lgamma of a large ν, and the same rule
//! on both. The upper tail is that integral's complement.
//!
//! The density, which no word answers (§S3: `tukey has no pdf`), is the
//! same double integral over the range's density, k(k − 1)∫ φ(z)·φ(z + w)·
//! [Φ(z + w) − Φ(z)]^(k−2) dz, for the quantile's Newton steps.

use std::convert::Infallible;

use super::normal_tails;
use crate::biomath::quadpack::{Range, integrate};

const Z_LO: f64 = -10.0;
const Z_HI: f64 = 9.0;
const Z_PIECES: usize = 38;
const S_HALF: f64 = 10.0;
const S_PIECES: usize = 24;
const SKIP: f64 = 1e-20;

fn phi(z: f64) -> f64 {
    (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// Φ(z + w) − Φ(z), from the tail that keeps it exact.
fn between(z: f64, w: f64) -> f64 {
    if z > 0.0 {
        normal_tails(z).1 - normal_tails(z + w).1
    } else {
        normal_tails(z + w).0 - normal_tails(z).0
    }
}

/// QAGS on (a, b) of an infallible integrand; its estimate whatever `ier`
/// says, the integrands here being smooth and bounded.
fn quad(f: &mut dyn FnMut(f64) -> f64, a: f64, b: f64, epsabs: f64, epsrel: f64) -> f64 {
    let mut g = |x: f64| Ok::<f64, Infallible>(f(x));
    match integrate(&mut g, Range::Finite(a, b), epsabs, epsrel, 200) {
        Ok(q) => q.value,
        Err(never) => match never {},
    }
}

/// Over z, the integral of `g` across [−10, 9] in pieces, skipping a piece
/// whose bound k·φ·m^(k−1)·width is below 10⁻²⁰.
fn over_z(k: f64, w: f64, power: f64, g: &mut dyn FnMut(f64) -> f64, epsrel: f64) -> f64 {
    let h = (Z_HI - Z_LO) / Z_PIECES as f64;
    let mut sum = 0.0;
    for i in 0..Z_PIECES {
        let z1 = Z_LO + h * i as f64;
        let z2 = z1 + h;
        let near = z1.max(0.0).min(z2);
        let m = normal_tails(z1).1.min(normal_tails(z2 + w).0);
        if k * k * phi(near) * m.powf(power) * h < SKIP {
            continue;
        }
        sum += quad(g, z1, z2, 1e-18, epsrel);
    }
    sum
}

/// W(w): P(range of k standard normals ≤ w).
fn range_cdf(k: f64, w: f64) -> f64 {
    if w <= 0.0 {
        return 0.0;
    }
    let mut g = |z: f64| k * phi(z) * between(z, w).powf(k - 1.0);
    over_z(k, w, k - 1.0, &mut g, 1e-14).min(1.0)
}

/// The range's density at w.
fn range_pdf(k: f64, w: f64) -> f64 {
    if w <= 0.0 {
        return 0.0;
    }
    let mut g = |z: f64| k * (k - 1.0) * phi(z) * phi(z + w) * between(z, w).powf(k - 2.0);
    over_z(k, w, (k - 2.0).max(0.0), &mut g, 1e-10)
}

/// ∫ f̃(s)·h(s) ds over s_m ± 10/√ν in pieces, and ∫ f̃(s) ds over the same.
fn over_s(df: f64, h: &mut dyn FnMut(f64) -> f64, epsrel: f64) -> (f64, f64) {
    let sm = ((df - 1.0) / df).sqrt();
    let half = S_HALF / df.sqrt();
    let lo = (sm - half).max(0.0);
    let hi = sm + half;
    let ln_f = |s: f64| (df - 1.0) * ((s - sm) / sm).ln_1p() - df * (s - sm) * (s + sm) / 2.0;
    let width = (hi - lo) / S_PIECES as f64;
    // ∫ f̃ ≈ √(π/ν) at most; the absolute tolerance is relative to it.
    let scale = (std::f64::consts::PI / df).sqrt();
    let (mut top, mut all) = (0.0, 0.0);
    for i in 0..S_PIECES {
        let a = lo + width * i as f64;
        let b = if i + 1 == S_PIECES { hi } else { a + width };
        let near = sm.clamp(a, b);
        if ln_f(near).exp() * width < SKIP * scale {
            continue;
        }
        all += quad(&mut |s| ln_f(s).exp(), a, b, 1e-17 * scale, 1e-14);
        top += quad(&mut |s| ln_f(s).exp() * h(s), a, b, 1e-16 * scale, epsrel);
    }
    (top, all)
}

/// (P(Q ≤ q), P(Q > q)).
pub(super) fn tails(nmeans: u64, df: f64, q: f64) -> (f64, f64) {
    if q <= 0.0 {
        return (0.0, 1.0);
    }
    let k = nmeans as f64;
    let (top, all) = over_s(df, &mut |s| range_cdf(k, q * s), 1e-13);
    let lower = (top / all).clamp(0.0, 1.0);
    (lower, ((all - top) / all).clamp(0.0, 1.0))
}

/// ln f(q), for the quantile's Newton steps.
pub(super) fn lpdf(nmeans: u64, df: f64, q: f64) -> f64 {
    if q <= 0.0 {
        return f64::NEG_INFINITY;
    }
    let k = nmeans as f64;
    let (top, all) = over_s(df, &mut |s| s * range_pdf(k, q * s), 1e-8);
    (top / all).ln()
}

/// A starting point for the quantile: the q at which all k normals lie
/// within ±q/2 of 0 with probability p, (2Φ(q/2) − 1)^k = p.
pub(super) fn guess(nmeans: u64, p: f64) -> f64 {
    let inner = p.powf(1.0 / nmeans as f64);
    2.0 * super::z_of((1.0 + inner) / 2.0)
}
