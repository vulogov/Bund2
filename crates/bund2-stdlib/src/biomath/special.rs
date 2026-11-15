//! RFC-0015 §S4: the scalar special functions, `math.log2` to
//! `math.lfactorial`.
//!
//! Each pushes one FLOAT. **The deepest operand is the formula's first
//! parameter** (§S3), so `10 3 math.choose` is C(10, 3) and
//! `0.5 2 3 math.betainc` is I₀.₅(2, 3). **An INTEGER operand converts to
//! FLOAT** (D250 B15), unlike the reference's unary `math.*` words, which
//! refuse one.
//!
//! **Every operand is checked before the call [B12]**, so no `statrs` panic
//! arm is reachable and no word answers NaN or ±∞ from finite operands:
//!
//! - a NaN, ±∞ or non-number operand, or one outside the function's domain,
//!   is E101;
//! - an answer that overflows f64 is E114. So is a NaN the arithmetic
//!   produces from operands that passed the checks, which only an
//!   intermediate overflow can cause (`math.lbeta` at 1e308 1e308);
//! - an iterative function that stops short of converging is E110.
//!
//! **`math.betainc` and `math.gammainc` are Bund2's own** continued
//! fractions and series, the algorithms `statrs` 0.18 uses (`checked_beta_reg`,
//! `checked_gamma_lr`), with an iteration cap and E110 at it. `statrs`'s
//! `checked_gamma_lr` runs its series in an unbounded `loop` that never ends
//! once `a + k` stops changing in f64 (a ≳ 1e16), which D39 forbids, and its
//! `checked_beta_reg` stops after 140 terms and answers whatever it has,
//! converged or not. §S4's note of 2026-11-14 records the change.
//!
//! **`math.erf` and `math.erfc` are `libm`'s** (fdlibm's, within an ulp).
//! `statrs` 0.18's are off by about 1e-10, which B11's 1e-12 for a closed
//! form does not allow. `math.erfinv` stays `statrs`'s `erf_inv`, which agrees
//! with mpmath to about 1e-16.
//!
//! Each word has a plain and a `.` form (§S3; D111's "anything else" row):
//! the `.` form takes the operand the plain word has on top from the
//! workbench and answers there.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;
use statrs::function::{beta as sbeta, erf as serf, factorial as sfact, gamma as sgamma};

use super::refuse::Refusal;

/// How wide an operand's text is in a refusal.
const WIDTH: usize = 40;

/// The most terms `math.betainc`'s continued fraction takes. It needs about
/// √max(a, b) of them, so this reaches a and b of 10⁹.
const BETA_CAP: u64 = 100_000;

/// The most terms `math.gammainc`'s series or continued fraction takes. Each
/// needs about √a near x = a, so this reaches a of about 10¹⁰.
const GAMMA_CAP: u64 = 1_000_000;

/// Where a continued fraction's denominators are lifted off zero (Lentz).
const FPMIN: f64 = f64::MIN_POSITIVE / f64::EPSILON;

/// When a term no longer moves the answer.
const EPS: f64 = 1e-15;

/// Pull `n` operands, deepest first, or refuse as the house does.
fn operands(vm: &mut dyn Vm, word: &str, n: usize) -> Result<Vec<BundValue>, Error> {
    if vm.depth() < n {
        return Err(Error(format!("Stack is too shallow for inline {word}")));
    }
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let Some(v) = vm.pull() else {
            return Err(Error::internal(format!(
                "{word}: the stack was deep enough and gave nothing"
            )));
        };
        out.push(v);
    }
    out.reverse();
    Ok(out)
}

fn domain(word: &str, operand: &str, value: String) -> Error {
    Refusal::Domain {
        operand: operand.into(),
        function: word.into(),
        value,
    }
    .error(word)
}

/// A FLOAT or INTEGER operand, finite, as f64 (B15).
fn real(word: &str, operand: &str, v: &BundValue) -> Result<f64, Error> {
    let x = match v.unboxed() {
        BundValue::Float(x, _) => *x,
        BundValue::Int(i, _) => *i as f64,
        _ => return Err(domain(word, operand, v.summary(WIDTH))),
    };
    if !x.is_finite() {
        return Err(domain(word, operand, v.summary(WIDTH)));
    }
    Ok(x)
}

/// A non-negative INTEGER operand.
fn whole(word: &str, operand: &str, v: &BundValue) -> Result<u64, Error> {
    match v.unboxed() {
        BundValue::Int(i, _) if *i >= 0 => Ok(i.unsigned_abs()),
        _ => Err(domain(word, operand, v.summary(WIDTH))),
    }
}

/// The answer, or E114 when it is not a finite FLOAT.
fn finite(word: &str, y: f64, expression: impl FnOnce() -> String) -> Result<f64, Error> {
    if y.is_finite() {
        Ok(y)
    } else {
        Err(Refusal::Overflow {
            expression: expression(),
        }
        .error(word))
    }
}

/// 0 or a negative integer: Γ's poles, and ψ's.
fn pole(x: f64) -> bool {
    x <= 0.0 && x.fract() == 0.0
}

/// Γ(x). `statrs`'s Lanczos sum overflows its intermediate power from about
/// 170 (`gamma(170.0)` is ∞), before Γ itself does at 171.62, so there Γ is
/// exp(ln Γ).
pub(crate) fn gamma(x: f64) -> f64 {
    let g = sgamma::gamma(x);
    if g.is_finite() || x < 0.5 {
        g
    } else {
        sgamma::ln_gamma(x).exp()
    }
}

/// ln |Γ(x)|, as R's `lgamma`. `statrs`'s `ln_gamma` takes the logarithm of
/// sin πx below 0.5, which is NaN wherever sin πx < 0; Bund2 reflects with
/// the absolute value: ln π − ln |sin πx| − ln Γ(1 − x).
pub(crate) fn lgamma(x: f64) -> f64 {
    if x >= 0.5 {
        sgamma::ln_gamma(x)
    } else {
        std::f64::consts::PI.ln()
            - (std::f64::consts::PI * x).sin().abs().ln()
            - sgamma::ln_gamma(1.0 - x)
    }
}

/// I_x(a, b) for a, b > 0 and x in [0, 1], or the number of terms it ran out
/// at. Numerical Recipes' `betai`, the algorithm `statrs` uses.
pub(crate) fn beta_reg(x: f64, a: f64, b: f64) -> Result<f64, u64> {
    if x == 0.0 {
        return Ok(0.0);
    }
    if x == 1.0 {
        return Ok(1.0);
    }
    let front = (lgamma(a + b) - lgamma(a) - lgamma(b) + a * x.ln() + b * (-x).ln_1p()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        Ok(front * beta_cf(x, a, b)? / a)
    } else {
        Ok(1.0 - front * beta_cf(1.0 - x, b, a)? / b)
    }
}

/// The continued fraction of I_x(a, b), by the modified Lentz method.
fn beta_cf(x: f64, a: f64, b: f64) -> Result<f64, u64> {
    let lift = |v: f64| if v.abs() < FPMIN { FPMIN } else { v };
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 / lift(1.0 - qab * x / qap);
    let mut h = d;
    for m in 1..=BETA_CAP {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 / lift(1.0 + aa * d);
        c = lift(1.0 + aa / c);
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 / lift(1.0 + aa * d);
        c = lift(1.0 + aa / c);
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() <= EPS {
            return Ok(h);
        }
    }
    Err(BETA_CAP)
}

/// P(a, x) for a > 0 and x ≥ 0, or the number of terms it ran out at. The
/// series below x = a + 1, the continued fraction for Q above it: Numerical
/// Recipes' `gser` and `gcf`, as `statrs` splits them.
pub(crate) fn gamma_lr(x: f64, a: f64) -> Result<f64, u64> {
    if x == 0.0 {
        return Ok(0.0);
    }
    let front = (a * x.ln() - x - lgamma(a)).exp();
    if x < a + 1.0 {
        let mut ap = a;
        let mut del = 1.0 / a;
        let mut sum = del;
        for _ in 0..GAMMA_CAP {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * EPS {
                return Ok((sum * front).min(1.0));
            }
        }
        Err(GAMMA_CAP)
    } else {
        let lift = |v: f64| if v.abs() < FPMIN { FPMIN } else { v };
        let mut b = x + 1.0 - a;
        let mut c = 1.0 / FPMIN;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..=GAMMA_CAP {
            let i = i as f64;
            let an = -i * (i - a);
            b += 2.0;
            d = 1.0 / lift(an * d + b);
            c = lift(b + an / c);
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() <= EPS {
                return Ok((1.0 - front * h).max(0.0));
            }
        }
        Err(GAMMA_CAP)
    }
}

/// C(n, k) exactly while it fits a u128, which is the answer rounded once.
/// `None` once a step would overflow; C(n, i) ≥ 2^i for i ≤ n/2, so that is
/// within about 128 steps whatever n is.
fn choose_exact(n: u64, k: u64) -> Option<u128> {
    let k = k.min(n - k);
    let mut c: u128 = 1;
    for i in 0..k {
        // C(n, i)·(n − i) = C(n, i + 1)·(i + 1), so the division is exact.
        c = c.checked_mul(u128::from(n - i))? / u128::from(i + 1);
    }
    Some(c)
}

fn eti(word: &str, method: &str, cap: u64) -> Error {
    Refusal::NoConvergence {
        method: method.into(),
        cap,
    }
    .error(word)
}

// One body per word, over its operands deepest first.

fn one(
    vm: &mut dyn Vm,
    word: &str,
    f: impl FnOnce(f64) -> Result<f64, Error>,
) -> Result<(), Error> {
    let v = operands(vm, word, 1)?;
    let x = real(word, "x", &v[0])?;
    let y = f(x)?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn math_log2(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.log2";
    one(vm, W, |x| {
        if x <= 0.0 {
            return Err(domain(W, "x", fmt(x)));
        }
        Ok(x.log2())
    })
}

fn math_log1p(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.log1p";
    one(vm, W, |x| {
        if x <= -1.0 {
            return Err(domain(W, "x", fmt(x)));
        }
        Ok(x.ln_1p())
    })
}

fn math_expm1(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.expm1";
    one(vm, W, |x| {
        finite(W, x.exp_m1(), || format!("exp({}) − 1", fmt(x)))
    })
}

fn math_gamma(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.gamma";
    one(vm, W, |x| {
        if pole(x) {
            return Err(domain(W, "x", fmt(x)));
        }
        finite(W, gamma(x), || format!("Γ({})", fmt(x)))
    })
}

fn math_lgamma(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.lgamma";
    one(vm, W, |x| {
        if pole(x) {
            return Err(domain(W, "x", fmt(x)));
        }
        finite(W, lgamma(x), || format!("ln |Γ({})|", fmt(x)))
    })
}

fn math_digamma(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.digamma";
    one(vm, W, |x| {
        if pole(x) {
            return Err(domain(W, "x", fmt(x)));
        }
        finite(W, sgamma::digamma(x), || format!("ψ({})", fmt(x)))
    })
}

fn math_erf(vm: &mut dyn Vm) -> Result<(), Error> {
    one(vm, "math.erf", |x| Ok(libm::erf(x)))
}

fn math_erfc(vm: &mut dyn Vm) -> Result<(), Error> {
    one(vm, "math.erfc", |x| Ok(libm::erfc(x)))
}

fn math_erfinv(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.erfinv";
    one(vm, W, |x| {
        if x <= -1.0 || x >= 1.0 {
            return Err(domain(W, "x", fmt(x)));
        }
        finite(W, serf::erf_inv(x), || format!("erf⁻¹({})", fmt(x)))
    })
}

/// a and b, both positive.
fn shape(word: &str, a: &BundValue, b: &BundValue) -> Result<(f64, f64), Error> {
    let a_ = real(word, "a", a)?;
    let b_ = real(word, "b", b)?;
    if a_ <= 0.0 {
        return Err(domain(word, "a", a.summary(WIDTH)));
    }
    if b_ <= 0.0 {
        return Err(domain(word, "b", b.summary(WIDTH)));
    }
    Ok((a_, b_))
}

fn math_beta(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.beta";
    let v = operands(vm, W, 2)?;
    let (a, b) = shape(W, &v[0], &v[1])?;
    let y = sbeta::checked_ln_beta(a, b).map_or(f64::NAN, f64::exp);
    let y = finite(W, y, || format!("B({}, {})", fmt(a), fmt(b)))?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn math_lbeta(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.lbeta";
    let v = operands(vm, W, 2)?;
    let (a, b) = shape(W, &v[0], &v[1])?;
    let y = sbeta::checked_ln_beta(a, b).unwrap_or(f64::NAN);
    let y = finite(W, y, || format!("ln B({}, {})", fmt(a), fmt(b)))?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn math_betainc(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.betainc";
    let v = operands(vm, W, 3)?;
    let x = real(W, "x", &v[0])?;
    let (a, b) = shape(W, &v[1], &v[2])?;
    if !(0.0..=1.0).contains(&x) {
        return Err(domain(W, "x", v[0].summary(WIDTH)));
    }
    let y =
        beta_reg(x, a, b).map_err(|cap| eti(W, "the incomplete beta's continued fraction", cap))?;
    let y = finite(W, y, || format!("I_{}({}, {})", fmt(x), fmt(a), fmt(b)))?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn math_gammainc(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.gammainc";
    let v = operands(vm, W, 2)?;
    let x = real(W, "x", &v[0])?;
    let a = real(W, "a", &v[1])?;
    if a <= 0.0 {
        return Err(domain(W, "a", v[1].summary(WIDTH)));
    }
    if x < 0.0 {
        return Err(domain(W, "x", v[0].summary(WIDTH)));
    }
    let y = gamma_lr(x, a).map_err(|cap| eti(W, "the incomplete gamma", cap))?;
    let y = finite(W, y, || format!("P({}, {})", fmt(a), fmt(x)))?;
    vm.push(BundValue::float(y));
    Ok(())
}

/// n and k, INTEGERs with 0 ≤ k ≤ n.
fn n_k(word: &str, v: &[BundValue]) -> Result<(u64, u64), Error> {
    let n = whole(word, "n", &v[0])?;
    let k = whole(word, "k", &v[1])?;
    if k > n {
        return Err(domain(word, "k", v[1].summary(WIDTH)));
    }
    Ok((n, k))
}

fn math_choose(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.choose";
    let v = operands(vm, W, 2)?;
    let (n, k) = n_k(W, &v)?;
    let y = match choose_exact(n, k) {
        Some(c) => c as f64,
        None => sfact::ln_binomial(n, k).exp(),
    };
    let y = finite(W, y, || format!("C({n}, {k})"))?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn math_lchoose(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.lchoose";
    let v = operands(vm, W, 2)?;
    let (n, k) = n_k(W, &v)?;
    let y = finite(W, sfact::ln_binomial(n, k), || format!("ln C({n}, {k})"))?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn math_lfactorial(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "math.lfactorial";
    let v = operands(vm, W, 1)?;
    let n = whole(W, "n", &v[0])?;
    let y = finite(W, sfact::ln_factorial(n), || format!("ln {n}!"))?;
    vm.push(BundValue::float(y));
    Ok(())
}

/// An f64 as a refusal names it: `0`, `-0.5`, `172`.
fn fmt(x: f64) -> String {
    format!("{x}")
}

pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("math.log2", math_log2, 1);
    word!("math.log1p", math_log1p, 1);
    word!("math.expm1", math_expm1, 1);
    word!("math.gamma", math_gamma, 1);
    word!("math.lgamma", math_lgamma, 1);
    word!("math.digamma", math_digamma, 1);
    word!("math.beta", math_beta, 2);
    word!("math.lbeta", math_lbeta, 2);
    word!("math.betainc", math_betainc, 3);
    word!("math.gammainc", math_gammainc, 2);
    word!("math.erf", math_erf, 1);
    word!("math.erfc", math_erfc, 1);
    word!("math.erfinv", math_erfinv, 1);
    word!("math.choose", math_choose, 2);
    word!("math.lchoose", math_lchoose, 2);
    word!("math.lfactorial", math_lfactorial, 1);
}

/// S4's words in registration order, each also registered with its `.` form:
/// criterion 2's list for this milestone.
pub const WORDS: [&str; 16] = [
    "math.log2",
    "math.log1p",
    "math.expm1",
    "math.gamma",
    "math.lgamma",
    "math.digamma",
    "math.beta",
    "math.lbeta",
    "math.betainc",
    "math.gammainc",
    "math.erf",
    "math.erfc",
    "math.erfinv",
    "math.choose",
    "math.lchoose",
    "math.lfactorial",
];

#[cfg(test)]
mod tests;
