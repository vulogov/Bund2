//! How the family's words take operands and give answers (RFC-0015 §S3).
//!
//! - **Scalars** are pulled first, and an INTEGER converts to FLOAT [B15].
//! - **One series** is read by the house reader, `crate::series::get_data`:
//!   a LIST, a METRICS or a drain, in the four forms of the `stat.`
//!   neighbours.
//! - **Two series** must each be a LIST or a METRICS, the top the second
//!   parameter; a drain is E115, because a drained first series would consume
//!   the second.
//! - **No NaN or ±∞ in, none out [B12]:** a non-finite element is E101, an
//!   answer that overflows is E114.

use bund2_api::{Error, Vm};
use bund2_value::{BundValue, FLOAT, LIST, METRICS};

use super::refuse::Refusal;
use crate::series::Mode;
use crate::wb::Side;

/// How wide an operand's text is in a refusal.
pub(crate) const WIDTH: usize = 40;

/// Pull `n` operands, deepest first, or refuse as the house does.
pub(crate) fn operands(vm: &mut dyn Vm, word: &str, n: usize) -> Result<Vec<BundValue>, Error> {
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

/// E101.
pub(crate) fn domain(word: &str, operand: &str, value: String) -> Error {
    Refusal::Domain {
        operand: operand.into(),
        function: word.into(),
        value,
    }
    .error(word)
}

/// A FLOAT or INTEGER operand, finite, as f64 (B15).
pub(crate) fn real(word: &str, operand: &str, v: &BundValue) -> Result<f64, Error> {
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

/// The answer, or E114 when it is not a finite FLOAT.
pub(crate) fn finite(
    word: &str,
    y: f64,
    expression: impl FnOnce() -> String,
) -> Result<f64, Error> {
    if y.is_finite() {
        Ok(y)
    } else {
        Err(Refusal::Overflow {
            expression: expression(),
        }
        .error(word))
    }
}

/// Every element finite, or E101 naming the first that is not, 1-based as
/// `at` counts.
pub(crate) fn finite_all(word: &str, operand: &str, xs: &[f64]) -> Result<(), Error> {
    match xs.iter().position(|x| !x.is_finite()) {
        None => Ok(()),
        Some(i) => Err(domain(word, operand, format!("{} at {}", xs[i], i + 1))),
    }
}

/// Each answer finite, or E114 naming the first that is not.
pub(crate) fn finite_list(word: &str, what: &str, ys: Vec<f64>) -> Result<BundValue, Error> {
    if let Some(i) = ys.iter().position(|y| !y.is_finite()) {
        return Err(Refusal::Overflow {
            expression: format!("{what} at {}", i + 1),
        }
        .error(word));
    }
    Ok(list(ys))
}

/// A LIST of FLOATs.
pub(crate) fn list(ys: Vec<f64>) -> BundValue {
    BundValue::list(ys.into_iter().map(BundValue::float).collect())
}

/// One whole series, a LIST or a METRICS: E115 for anything else, E103 for
/// a cell that will not convert to FLOAT, E101 for one that is not finite.
pub(crate) fn whole(word: &str, operand: &str, v: &BundValue) -> Result<Vec<f64>, Error> {
    let xs: Vec<f64> = match v.dt() {
        LIST => {
            let items = v.as_list().unwrap_or(&[]);
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let f = crate::convert::conv_value(item, FLOAT).ok();
                match f.as_ref().map(BundValue::unboxed) {
                    Some(BundValue::Float(x, _)) => out.push(*x),
                    _ => {
                        return Err(Refusal::NotNumbers {
                            operand: operand.into(),
                            container: "LIST",
                            cell: item.summary(WIDTH),
                        }
                        .error(word));
                    }
                }
            }
            out
        }
        METRICS => v
            .as_metrics()
            .unwrap_or(&[])
            .iter()
            .map(|m| m.data)
            .collect(),
        _ => {
            return Err(Refusal::Drained {
                operand: operand.into(),
            }
            .error(word));
        }
    };
    finite_all(word, operand, &xs)?;
    Ok(xs)
}

/// Two whole series, `a` beneath and `b` on top.
pub(crate) fn two(vm: &mut dyn Vm, word: &str) -> Result<(Vec<f64>, Vec<f64>), Error> {
    let v = operands(vm, word, 2)?;
    Ok((whole(word, "a", &v[0])?, whole(word, "b", &v[1])?))
}

/// Two series of one length, or E102.
pub(crate) fn same_length(word: &str, a: &[f64], b: &[f64]) -> Result<(), Error> {
    if a.len() == b.len() {
        Ok(())
    } else {
        Err(Refusal::Disagree {
            what: "lengths".into(),
            n: a.len(),
            m: b.len(),
        }
        .error(word))
    }
}

/// One series by the house reader, from `side`, kept or consumed as the form
/// says, its elements finite. `word` is the form's name, which the reader's
/// own errors carry upper-cased, as the `stat.` neighbours' do.
pub(crate) fn series(
    vm: &mut dyn Vm,
    side: Side,
    mode: Mode,
    word: &str,
) -> Result<Vec<f64>, Error> {
    let prefix = word.to_uppercase();
    let xs = crate::series::get_data(vm, side, mode, &prefix)
        .map_err(|e| Error(format!("{prefix} returned: {}", e.0)))?;
    finite_all(word, "series", &xs)?;
    Ok(xs)
}

/// A one-series word in one of its four forms: read, compute, answer on the
/// side the series came from.
pub(crate) fn one_series(
    vm: &mut dyn Vm,
    side: Side,
    mode: Mode,
    word: &str,
    f: fn(&str, &[f64]) -> Result<BundValue, Error>,
) -> Result<(), Error> {
    let xs = series(vm, side, mode, word)?;
    let y = f(word, &xs)?;
    side.push(vm, y);
    Ok(())
}

/// A word of a series beneath a scalar. The plain form takes the scalar from
/// the stack; the `.` form takes it from the workbench, D111's "anything
/// else" row, and answers there. The series is the stack's either way.
pub(crate) fn series_and_scalar(
    vm: &mut dyn Vm,
    side: Side,
    word: &str,
    scalar: &str,
    f: fn(&str, &[f64], f64) -> Result<BundValue, Error>,
) -> Result<(), Error> {
    if side.depth(vm) < 1 {
        let which = match side {
            Side::Stack => "Stack",
            Side::Bench => "Workbench",
        };
        return Err(Error(format!("{which} is too shallow for inline {word}")));
    }
    let Some(k) = side.pull(vm) else {
        return Err(Error::internal(format!(
            "{word}: the {} was deep enough and gave nothing",
            if side == Side::Stack {
                "stack"
            } else {
                "workbench"
            }
        )));
    };
    let k = real(word, scalar, &k)?;
    let xs = series(vm, Side::Stack, Mode::Consume, word)?;
    let y = f(word, &xs, k)?;
    side.push(vm, y);
    Ok(())
}

/// An empty or too-short series, E101.
pub(crate) fn at_least(word: &str, xs: &[f64], n: usize) -> Result<(), Error> {
    if xs.len() >= n {
        Ok(())
    } else {
        let v = match xs.len() {
            0 => "an empty series".to_string(),
            1 => "a series of 1 value".to_string(),
            k => format!("a series of {k} values"),
        };
        Err(domain(word, "series", v))
    }
}

/// Register a one-series word in its four forms: plain and `,` read the
/// stack and are `opaque(1)` for the drain's sake; `.` and `.,` read the
/// workbench and are `opaque(0)`, as the `stat.` neighbours' are.
macro_rules! four_forms {
    ($r:expr, $base:literal, $f:path) => {{
        use bund2_api::{Error, StackEffect, Vm, WordKind};
        use $crate::biomath::operand::one_series;
        use $crate::series::Mode;
        use $crate::wb::Side;
        fn s(vm: &mut dyn Vm) -> Result<(), Error> {
            one_series(vm, Side::Stack, Mode::Consume, $base, $f)
        }
        fn sk(vm: &mut dyn Vm) -> Result<(), Error> {
            one_series(vm, Side::Stack, Mode::Keep, concat!($base, ","), $f)
        }
        fn w(vm: &mut dyn Vm) -> Result<(), Error> {
            one_series(vm, Side::Bench, Mode::Consume, concat!($base, "."), $f)
        }
        fn wk(vm: &mut dyn Vm) -> Result<(), Error> {
            one_series(vm, Side::Bench, Mode::Keep, concat!($base, ".,"), $f)
        }
        $r.register_native($base, s, StackEffect::opaque(1), WordKind::Sync);
        $r.register_native(
            concat!($base, ","),
            sk,
            StackEffect::opaque(1),
            WordKind::Sync,
        );
        $r.register_native(
            concat!($base, "."),
            w,
            StackEffect::opaque(0),
            WordKind::Sync,
        );
        $r.register_native(
            concat!($base, ".,"),
            wk,
            StackEffect::opaque(0),
            WordKind::Sync,
        );
    }};
}
pub(crate) use four_forms;

/// Register a word of a series beneath a scalar, plain and `.`: `opaque(2)`
/// and `opaque(1)`, since the series may be a drain.
macro_rules! scalar_forms {
    ($r:expr, $base:literal, $scalar:literal, $f:path) => {{
        use bund2_api::{Error, StackEffect, Vm, WordKind};
        use $crate::biomath::operand::series_and_scalar;
        use $crate::wb::Side;
        fn s(vm: &mut dyn Vm) -> Result<(), Error> {
            series_and_scalar(vm, Side::Stack, $base, $scalar, $f)
        }
        fn w(vm: &mut dyn Vm) -> Result<(), Error> {
            series_and_scalar(vm, Side::Bench, concat!($base, "."), $scalar, $f)
        }
        $r.register_native($base, s, StackEffect::opaque(2), WordKind::Sync);
        $r.register_native(
            concat!($base, "."),
            w,
            StackEffect::opaque(1),
            WordKind::Sync,
        );
    }};
}
pub(crate) use scalar_forms;

/// Register a word of two whole series, plain and `.`: a fixed `2 -> 1`, and
/// the `.` form taking `b`, the top, from the workbench (D111).
macro_rules! two_forms {
    ($r:expr, $base:literal, $f:path) => {{
        $r.register_native(
            $base,
            $f,
            bund2_api::StackEffect::fixed(2, 1),
            bund2_api::WordKind::Sync,
        );
        $crate::wb::bench!($r, $base, $f, 2, 0);
    }};
}
pub(crate) use two_forms;
