//! The **library half** — D14's other 211 words, or the start of them.
//!
//! D14 splits the 497 in-scope words into a core of 286 that is a preservation
//! target and a library of 211 that is "deferrable, re-implementable as
//! out-of-tree word packages". Deferrable is not the same as absent: these are
//! still in scope, still counted by `cargo xtask coverage`, and a program that
//! calls one still expects the reference's answer.
//!
//! What is here is the part that can be **checked**: pure functions of their
//! operands, with no clock, no filesystem, no network and no randomness. Those
//! three exclusions are most of why the library half is deferrable at all —
//! `id.ulid` and `math.random.int` cannot be captured as goldens, and
//! `bund/filesystem` and `bund/sysinfo` answer differently on every machine.
//!
//! Two families to start:
//!
//! - **`vm/string`'s case words**, which go through `convert_case` — the same
//!   crate `graph!` needs for node names, so no new dependency.
//! - **`bund/math`'s pure functions**, which go through `mathlab`. Using the
//!   reference's own crate matters here for the same reason it did for
//!   `dtoa`: a golden captures the digits, and two libraries need not agree on
//!   the last one.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;
use convert_case::{Case, Casing};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// The case words (`reference/rust_multistackvm/src/stdlib/string/case.rs:10-146`).
///
/// Each **converts** its operand first — `value.conv(STRING)` at `:13` — so a
/// number is cased as its own text rather than refused: `42 string.upper` is
/// `"42"`. Then `to_case`, from `convert_case`.
fn case_word(vm: &mut dyn Vm, case: Case, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let v = crate::pull::operand(vm, prefix, 1)?;
    if !v.displayable() {
        return Err(Error(format!(
            "{prefix} return error: Can not convert Value from {}",
            v.dt()
        )));
    }
    vm.push(BundValue::str(v.display().to_case(case)));
    Ok(())
}

/// One float in, one float out (`reference/Bund/src/stdlib/functions/math/math.rs:62-76`).
///
/// The gate is `cast_float`, as the `math.*` core family's is: an INTEGER is
/// refused, not widened.
fn math1(vm: &mut dyn Vm, f: fn(f64) -> f64, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let v = crate::pull::operand(vm, prefix, 1)?;
    let BundValue::Float(x, _) = *v.unboxed() else {
        return Err(Error(format!(
            "{prefix} returns error: This Dynamic type is not float: {}",
            v.dt()
        )));
    };
    vm.push(BundValue::float(f(x)));
    Ok(())
}

fn as_float(v: &BundValue) -> Option<f64> {
    match *v.unboxed() {
        BundValue::Float(f, _) => Some(f),
        _ => None,
    }
}

/// Two floats in, one out (`math.rs:77-125`).
///
/// **The first pull is the second argument.** `math.power` computes
/// `pow(xvalue, fvalue)` where `fvalue` is the *first* pull and `xvalue` the
/// second (`:111-117`), so the source reads `<exponent> <base> math.power`.
/// `math.nroot` and `math.perimeter` take theirs in the other order —
/// `nrt(fvalue, nvalue)` uses the first pull first (`:79-83`) — so the family
/// is not consistent with itself and each is written from its own line.
fn math2(vm: &mut dyn Vm, f: fn(f64, f64) -> f64, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let a = crate::pull::operand(vm, prefix, 1)?;
    let b = crate::pull::operand(vm, prefix, 2)?;
    let (Some(x), Some(y)) = (as_float(&a), as_float(&b)) else {
        return Err(Error(format!(
            "{prefix} returns error: This Dynamic type is not float"
        )));
    };
    vm.push(BundValue::float(f(x, y)));
    Ok(())
}

pub fn register(r: &mut Registry) {
    macro_rules! case {
        ($name:literal, $case:expr, $prefix:literal) => {
            r.register_native(
                $name,
                |vm| case_word(vm, $case, $prefix),
                eff(1, 1),
                WordKind::Sync,
            );
        };
    }
    case!("string.upper", Case::Upper, "STRING_UPPER");
    case!("string.lower", Case::Lower, "STRING_LOWER");
    case!("string.snake", Case::Snake, "STRING_SNAKE");
    case!("string.title", Case::Title, "STRING_TITLE");
    case!("string.camel", Case::Camel, "STRING_CAMEL");

    macro_rules! m1 {
        ($name:literal, $f:expr, $prefix:literal) => {
            r.register_native($name, |vm| math1(vm, $f, $prefix), eff(1, 1), WordKind::Sync);
        };
    }
    m1!("math.exp", mathlab::math::exp, "MATH.EXP");
    m1!("math.ln", mathlab::math::ln, "MATH.LN");
    m1!("math.log10", mathlab::math::log10, "MATH.LOG10");
    m1!("math.cosecant", mathlab::math::csc, "MATH.COSECANT");
    // `fact` takes a `u64` and answers a `u64`; the reference casts in and out
    // (`math.rs:69`), so a negative or fractional operand is truncated rather
    // than refused.
    m1!(
        "math.factorial",
        |x: f64| mathlab::math::fact(x as u64) as f64,
        "MATH.FACTORIAL"
    );

    macro_rules! m2 {
        ($name:literal, $f:expr, $prefix:literal) => {
            r.register_native($name, |vm| math2(vm, $f, $prefix), eff(2, 1), WordKind::Sync);
        };
    }
    m2!("math.nroot", mathlab::math::nrt, "MATH.NROOT");
    m2!(
        "math.perimeter",
        mathlab::math::perimeter,
        "MATH.PERIMETER"
    );
    // Reversed against the other two: `pow(xvalue, fvalue)` with `fvalue` the
    // first pull (`math.rs:113-117`).
    m2!("math.power", |a, b| mathlab::math::pow(b, a), "MATH.POWER");
}
