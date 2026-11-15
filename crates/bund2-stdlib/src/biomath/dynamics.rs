//! RFC-0015 §S15: ODE integration and Gillespie's direct method.
//!
//! **The model** is a LAMBDA run as RFC-0013's bridges run one
//! ([`crate::kb::bridge::ask`]): on a scratch stack named per nesting depth,
//! with its arguments the only values there, returning to the caller's stack
//! however it leaves. The value it leaves on top is read; anything else it
//! left goes with the scratch stack, so the caller's stack is never visible
//! to it and is unchanged by it. That value must be a LIST of finite numbers
//! of the length required, else E112.
//!
//! `ask` pushes one value and runs the body; a two-argument model's body is
//! its second argument followed by the lambda's items, so `t` is pushed and
//! then `y`, and the lambda sees `t y` as §S15's effect says.
//!
//! **Effects.** Each word calls a lambda, so it is `StackEffect::opaque` and
//! has the plain form only (§S3).
//!
//! **Work (D39).** Steps and events are capped at 10⁶ (E111); a step below
//! 10⁻¹²·|t1 − t0| is E110. A refusal mid-integration consumes the word's
//! operands and pushes nothing (D36).

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LAMBDA, MAP};
use rand::Rng as _;

use super::infer::table;
use super::operand::{WIDTH, domain, list, operands, real, whole};
use super::refuse::Refusal;
use super::rng::with_rng;

/// The most steps or events one call takes (§S15).
const STEP_CAP: u64 = 1_000_000;

/// A step smaller than this fraction of the span is E110 (§S15).
const STEP_FLOOR: f64 = 1e-12;

/// A LAMBDA operand's items, or E101.
fn lambda<'a>(word: &str, operand: &str, v: &'a BundValue) -> Result<&'a [BundValue], Error> {
    match v.as_lambda() {
        Some(items) if v.dt() == LAMBDA => Ok(items),
        _ => Err(domain(word, operand, v.summary(WIDTH))),
    }
}

/// Run `f` on `args`, deepest first, and read the LIST of `n` finite
/// numbers it leaves on top, or E112.
fn call(
    vm: &mut dyn Vm,
    word: &str,
    f: &[BundValue],
    args: Vec<BundValue>,
    n: usize,
    nonnegative: bool,
) -> Result<Vec<f64>, Error> {
    let mut args = args.into_iter();
    let first = args.next().unwrap_or_else(BundValue::nodata);
    let body: Vec<BundValue> = args.chain(f.iter().cloned()).collect();
    let left = crate::kb::bridge::ask(vm, &body, first)?;
    let expected = || {
        format!(
            "a LIST of {n} {}numbers",
            if nonnegative {
                "non-negative "
            } else {
                "finite "
            }
        )
    };
    let refuse = |left: String| {
        Refusal::Model {
            left,
            expected: expected(),
        }
        .error(word)
    };
    let Some(v) = left else {
        return Err(refuse("nothing".into()));
    };
    let xs = match v.as_list() {
        Some(items) if v.dt() == bund2_value::LIST && items.len() == n => items
            .iter()
            .map(|x| match x.unboxed() {
                BundValue::Float(f, _) => Some(*f),
                BundValue::Int(i, _) => Some(*i as f64),
                _ => None,
            })
            .collect::<Option<Vec<f64>>>(),
        _ => None,
    };
    match xs {
        Some(xs)
            if xs
                .iter()
                .all(|x| x.is_finite() && (!nonnegative || *x >= 0.0)) =>
        {
            Ok(xs)
        }
        _ => Err(refuse(v.summary(WIDTH))),
    }
}

fn state(word: &str, what: &str, y: &[f64]) -> Result<(), Error> {
    if y.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(Refusal::Overflow {
            expression: what.into(),
        }
        .error(word))
    }
}

fn cap(word: &str, what: &str) -> Error {
    Refusal::Cap {
        what: what.into(),
        cap: STEP_CAP,
    }
    .error(word)
}

/// The answer: `t` and the state at each, under `key`.
fn path(ts: Vec<f64>, ys: Vec<Vec<f64>>, key: &str) -> BundValue {
    let mut m = BTreeMap::new();
    m.insert("t".to_string(), list(ts));
    m.insert(
        key.to_string(),
        BundValue::list(ys.into_iter().map(list).collect()),
    );
    BundValue::map(m)
}

fn axpy(y: &[f64], h: f64, ks: &[(&[f64], f64)]) -> Vec<f64> {
    (0..y.len())
        .map(|i| y[i] + h * ks.iter().map(|(k, c)| c * k[i]).sum::<f64>())
        .collect()
}

/// The model's derivative at (t, y).
fn deriv(
    vm: &mut dyn Vm,
    word: &str,
    f: &[BundValue],
    t: f64,
    y: &[f64],
) -> Result<Vec<f64>, Error> {
    call(
        vm,
        word,
        f,
        vec![BundValue::float(t), list(y.to_vec())],
        y.len(),
        false,
    )
}

/// `f y0 t0 t1` and the span's checks.
fn problem(word: &str, v: &[BundValue]) -> Result<(Vec<f64>, f64, f64), Error> {
    let y0 = whole(word, "y0", &v[1])?;
    if y0.is_empty() {
        return Err(domain(word, "y0", "an empty LIST".into()));
    }
    let t0 = real(word, "t0", &v[2])?;
    let t1 = real(word, "t1", &v[3])?;
    if !(t1 - t0).is_finite() {
        return Err(domain(word, "t1", v[3].summary(WIDTH)));
    }
    Ok((y0, t0, t1))
}

/// Classical fourth-order Runge–Kutta with step h, the last step shortened
/// to land on t1. t1 below t0 integrates backwards.
fn ode_rk4(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "ode.rk4";
    let v = operands(vm, W, 5)?;
    let f = lambda(W, "f", &v[0])?.to_vec();
    let (y0, t0, t1) = problem(W, &v)?;
    let h = real(W, "h", &v[4])?;
    if h <= 0.0 {
        return Err(domain(W, "h", v[4].summary(WIDTH)));
    }
    let span = t1 - t0;
    let steps = (span.abs() / h).ceil();
    if steps > STEP_CAP as f64 {
        return Err(cap(W, &format!("an integration of {steps} steps")));
    }
    let steps = steps as u64;
    let dir = span.signum();
    let (mut ts, mut ys) = (vec![t0], vec![y0.clone()]);
    let (mut t, mut y) = (t0, y0);
    for k in 1..=steps {
        let next = if k == steps {
            t1
        } else {
            t0 + dir * h * k as f64
        };
        let dt = next - t;
        let k1 = deriv(vm, W, &f, t, &y)?;
        let k2 = deriv(vm, W, &f, t + dt / 2.0, &axpy(&y, dt / 2.0, &[(&k1, 1.0)]))?;
        let k3 = deriv(vm, W, &f, t + dt / 2.0, &axpy(&y, dt / 2.0, &[(&k2, 1.0)]))?;
        let k4 = deriv(vm, W, &f, next, &axpy(&y, dt, &[(&k3, 1.0)]))?;
        y = axpy(
            &y,
            dt / 6.0,
            &[(&k1, 1.0), (&k2, 2.0), (&k3, 2.0), (&k4, 1.0)],
        );
        state(W, &format!("y at t = {next}"), &y)?;
        t = next;
        ts.push(t);
        ys.push(y.clone());
    }
    vm.push(path(ts, ys, "y"));
    Ok(())
}

/// `opts`: each key optional, each a positive finite number; an unknown key
/// is E101.
fn rk45_opts(word: &str, v: &BundValue, span: f64) -> Result<(f64, f64, f64, f64), Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(word, "opts", v.summary(WIDTH))),
    };
    let mut out = [1e-6, 1e-9, span / 100.0, span];
    for (k, x) in m {
        let at = match k.as_str() {
            "rtol" => 0,
            "atol" => 1,
            "h0" => 2,
            "hmax" => 3,
            _ => return Err(domain(word, "opts", format!("an unknown key {k:?}"))),
        };
        let x = real(word, k, x)?;
        if x <= 0.0 {
            return Err(domain(word, k, x.to_string()));
        }
        out[at] = x;
    }
    Ok((out[0], out[1], out[2], out[3]))
}

/// Dormand and Prince's 5(4) pair: the fifth-order solution is kept, the
/// embedded fourth-order one estimates the error, and the step is scaled by
/// 0.9·err^(−1/5), between 0.2 and 5 times.
fn ode_rk45(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "ode.rk45";
    const C: [f64; 7] = [0.0, 1.0 / 5.0, 3.0 / 10.0, 4.0 / 5.0, 8.0 / 9.0, 1.0, 1.0];
    const A: [[f64; 6]; 7] = [
        [0.0; 6],
        [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0],
        [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0],
        [
            19372.0 / 6561.0,
            -25360.0 / 2187.0,
            64448.0 / 6561.0,
            -212.0 / 729.0,
            0.0,
            0.0,
        ],
        [
            9017.0 / 3168.0,
            -355.0 / 33.0,
            46732.0 / 5247.0,
            49.0 / 176.0,
            -5103.0 / 18656.0,
            0.0,
        ],
        [
            35.0 / 384.0,
            0.0,
            500.0 / 1113.0,
            125.0 / 192.0,
            -2187.0 / 6784.0,
            11.0 / 84.0,
        ],
    ];
    // Fifth order less fourth: the error estimate's weights.
    const E: [f64; 7] = [
        35.0 / 384.0 - 5179.0 / 57600.0,
        0.0,
        500.0 / 1113.0 - 7571.0 / 16695.0,
        125.0 / 192.0 - 393.0 / 640.0,
        -2187.0 / 6784.0 + 92097.0 / 339200.0,
        11.0 / 84.0 - 187.0 / 2100.0,
        -1.0 / 40.0,
    ];
    let v = operands(vm, W, 5)?;
    let f = lambda(W, "f", &v[0])?.to_vec();
    let (y0, t0, t1) = problem(W, &v)?;
    let span = (t1 - t0).abs();
    let (rtol, atol, h0, hmax) = rk45_opts(W, &v[4], span)?;
    if span > 0.0 && span / hmax > STEP_CAP as f64 {
        return Err(cap(
            W,
            &format!("an integration of at least {} steps", (span / hmax).ceil()),
        ));
    }
    let dir = (t1 - t0).signum();
    let floor = STEP_FLOOR * span;
    let (mut ts, mut ys) = (vec![t0], vec![y0.clone()]);
    let (mut t, mut y) = (t0, y0);
    let mut h = h0.min(hmax);
    let mut steps: u64 = 0;
    while dir * (t1 - t) > 0.0 {
        steps += 1;
        if steps > STEP_CAP {
            return Err(cap(W, "the step count"));
        }
        if h < floor {
            return Err(super::special::eti(W, "rk45's step size control", steps));
        }
        let dt = dir * h.min(dir * (t1 - t));
        let mut k: Vec<Vec<f64>> = Vec::with_capacity(7);
        k.push(deriv(vm, W, &f, t, &y)?);
        for s in 1..7 {
            let ks: Vec<(&[f64], f64)> = (0..s).map(|j| (k[j].as_slice(), A[s][j])).collect();
            let ys = axpy(&y, dt, &ks);
            k.push(deriv(vm, W, &f, t + C[s] * dt, &ys)?);
        }
        let ks: Vec<(&[f64], f64)> = (0..6).map(|j| (k[j].as_slice(), A[6][j])).collect();
        let next = axpy(&y, dt, &ks);
        let err_ks: Vec<(&[f64], f64)> = (0..7).map(|j| (k[j].as_slice(), E[j])).collect();
        let errv = axpy(&vec![0.0; y.len()], dt, &err_ks);
        let err = (errv
            .iter()
            .zip(y.iter().zip(&next))
            .map(|(e, (a, b))| (e / (atol + rtol * a.abs().max(b.abs()))).powi(2))
            .sum::<f64>()
            / y.len() as f64)
            .sqrt();
        let factor = if err == 0.0 {
            5.0
        } else {
            (0.9 * err.powf(-0.2)).clamp(0.2, 5.0)
        };
        if err.is_finite() && err <= 1.0 {
            t = if (t1 - (t + dt)) * dir <= 0.0 {
                t1
            } else {
                t + dt
            };
            state(W, &format!("y at t = {t}"), &next)?;
            y = next;
            ts.push(t);
            ys.push(y.clone());
            h = (h * factor).min(hmax);
        } else {
            h *= if err.is_finite() {
                factor.min(1.0)
            } else {
                0.2
            };
        }
    }
    vm.push(path(ts, ys, "y"));
    Ok(())
}

/// Gillespie's direct method: draw the waiting time from the propensities'
/// sum, then the reaction by its share. The run ends at tmax, or when every
/// propensity is 0; either way the last point is the state at tmax.
fn sim_gillespie(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "sim.gillespie";
    let v = operands(vm, W, 4)?;
    let x0 = whole(W, "x0", &v[0])?;
    let props = lambda(W, "propensities", &v[1])?.to_vec();
    let stoich = table(W, "stoich", &v[2])?;
    let tmax = real(W, "tmax", &v[3])?;
    if tmax < 0.0 {
        return Err(domain(W, "tmax", v[3].summary(WIDTH)));
    }
    if x0.is_empty() {
        return Err(domain(W, "x0", "an empty LIST".into()));
    }
    let species = x0.len();
    if let Some(row) = stoich.iter().find(|r| r.len() != species) {
        return Err(Refusal::Disagree {
            what: "stoich's columns and x0".into(),
            n: row.len(),
            m: species,
        }
        .error(W));
    }
    let reactions = stoich.len();
    let (mut ts, mut xs) = (vec![0.0], vec![x0.clone()]);
    let (mut t, mut x) = (0.0f64, x0);
    let mut events: u64 = 0;
    loop {
        let a = call(vm, W, &props, vec![list(x.clone())], reactions, true)?;
        let total: f64 = a.iter().sum();
        if total <= 0.0 || !total.is_finite() {
            break;
        }
        let (u1, u2): (f64, f64) = with_rng(vm, |g| (g.r#gen(), g.r#gen()));
        // 1 − U is in (0, 1], so its logarithm is finite.
        let tau = -(1.0 - u1).ln() / total;
        if t + tau > tmax {
            break;
        }
        events += 1;
        if events > STEP_CAP {
            return Err(cap(W, "the event count"));
        }
        let target = u2 * total;
        let mut acc = 0.0;
        let mut j = reactions - 1;
        for (r, ar) in a.iter().enumerate() {
            acc += ar;
            if target < acc {
                j = r;
                break;
            }
        }
        for (xi, s) in x.iter_mut().zip(&stoich[j]) {
            *xi += s;
        }
        t += tau;
        ts.push(t);
        xs.push(x.clone());
    }
    if t < tmax {
        ts.push(tmax);
        xs.push(x);
    }
    vm.push(path(ts, xs, "x"));
    Ok(())
}

/// Register §S15's words: lambda-calling, so opaque and plain only (§S3).
pub fn register(r: &mut Registry) {
    r.register_native("ode.rk4", ode_rk4, StackEffect::opaque(5), WordKind::Sync);
    r.register_native("ode.rk45", ode_rk45, StackEffect::opaque(5), WordKind::Sync);
    r.register_native(
        "sim.gillespie",
        sim_gillespie,
        StackEffect::opaque(4),
        WordKind::Sync,
    );
}

/// §S15's words, which have no `.` form.
pub const WORDS: [&str; 3] = ["ode.rk4", "ode.rk45", "sim.gillespie"];

#[cfg(test)]
mod tests;
