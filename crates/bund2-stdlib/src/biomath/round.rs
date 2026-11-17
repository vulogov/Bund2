//! RFC-0016 §S6: rounding to places and significant digits, and fixed and
//! scientific formatting: `math.round.places`, `math.signif`, `string.fixed`
//! and `string.sci`, each `x n -- y` in a plain and a `.` form.
//!
//! **Each is Rust's formatter's rounding** [A6]: the exact binary value is
//! rounded, ties to even. `math.round.places` parses `{:.n$}` and
//! `math.signif` parses `{:.m$e}` (m = n − 1), so `2.675 2
//! math.round.places` is 2.67, 2.675 being stored just below it, and `2.5 0
//! math.round.places` is 2.0 where the reference's `math.round` is 3.0.
//! `string.fixed` and `string.sci` answer those strings.
//!
//! **Negative zero** is normalised: an answer that is zero is +0.0, and its
//! string carries no `-`.
//!
//! **x** is a FLOAT or an INTEGER [B15], or a LIST answered element-wise,
//! its cells read as `operand::whole` reads them (a numeric STRING cell is
//! its number). A STRING x is E103.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST, STRING};

use super::operand::{WIDTH, domain, finite, operands, whole};
use super::refuse::Refusal;

/// What a word does to one f64, given n.
type Each = fn(&str, f64, usize) -> Result<BundValue, Error>;

/// n, an INTEGER in `lo..=hi`, or E101.
fn places(word: &str, v: &BundValue, lo: i64, hi: i64) -> Result<usize, Error> {
    match v.unboxed() {
        BundValue::Int(n, _) if (lo..=hi).contains(n) => Ok(*n as usize),
        _ => Err(domain(word, "n", v.summary(WIDTH))),
    }
}

/// +0.0 for either zero.
fn unsigned_zero(y: f64) -> f64 {
    if y == 0.0 { 0.0 } else { y }
}

/// A formatted number without the sign of a negative zero.
fn unsigned_text(s: String) -> String {
    match s.parse::<f64>() {
        Ok(0.0) => s.trim_start_matches('-').to_string(),
        _ => s,
    }
}

fn parse(word: &str, s: &str) -> Result<f64, Error> {
    s.parse::<f64>().map_err(|_| {
        Error::internal(format!(
            "{word}: Rust's formatter wrote {s}, which does not parse as an f64"
        ))
    })
}

fn round_places(word: &str, x: f64, n: usize) -> Result<BundValue, Error> {
    let y = parse(word, &format!("{x:.n$}"))?;
    Ok(BundValue::float(unsigned_zero(y)))
}

fn signif(word: &str, x: f64, n: usize) -> Result<BundValue, Error> {
    let m = n - 1;
    let y = parse(word, &format!("{x:.m$e}"))?;
    let y = finite(word, y, || format!("{x:e} to {n} significant digits"))?;
    Ok(BundValue::float(unsigned_zero(y)))
}

fn fixed(_word: &str, x: f64, n: usize) -> Result<BundValue, Error> {
    Ok(BundValue::str(unsigned_text(format!("{x:.n$}"))))
}

fn sci(_word: &str, x: f64, n: usize) -> Result<BundValue, Error> {
    Ok(BundValue::str(unsigned_text(format!("{x:.n$e}"))))
}

/// x n -- y: x a number or a LIST of numbers.
fn apply(vm: &mut dyn Vm, word: &str, lo: i64, hi: i64, each: Each) -> Result<(), Error> {
    let v = operands(vm, word, 2)?;
    let n = places(word, &v[1], lo, hi)?;
    let x = &v[0];
    let y = match x.unboxed() {
        BundValue::Float(f, _) if f.is_finite() => each(word, *f, n)?,
        BundValue::Int(i, _) => each(word, *i as f64, n)?,
        _ => match x.dt() {
            LIST => {
                let xs = whole(word, "x", x)?;
                let mut out = Vec::with_capacity(xs.len());
                for f in xs {
                    out.push(each(word, f, n)?);
                }
                BundValue::list(out)
            }
            STRING => {
                return Err(Refusal::NotNumbers {
                    operand: "x".into(),
                    container: "LIST",
                    cell: x.summary(WIDTH),
                }
                .error(word));
            }
            _ => return Err(domain(word, "x", x.summary(WIDTH))),
        },
    };
    vm.push(y);
    Ok(())
}

fn math_round_places(vm: &mut dyn Vm) -> Result<(), Error> {
    apply(vm, "math.round.places", 0, 17, round_places)
}

fn math_signif(vm: &mut dyn Vm) -> Result<(), Error> {
    apply(vm, "math.signif", 1, 17, signif)
}

fn string_fixed(vm: &mut dyn Vm) -> Result<(), Error> {
    apply(vm, "string.fixed", 0, 17, fixed)
}

fn string_sci(vm: &mut dyn Vm) -> Result<(), Error> {
    apply(vm, "string.sci", 0, 16, sci)
}

/// Register the words, plain and `.`: a fixed `2 -> 1`, the `.` form taking
/// n, the top, from the workbench and answering there (D111, `bench!`).
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr) => {
            r.register_native($name, $f, StackEffect::fixed(2, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, 2, 0);
        };
    }
    word!("math.round.places", math_round_places);
    word!("math.signif", math_signif);
    word!("string.fixed", string_fixed);
    word!("string.sci", string_sci);
}

/// The words with their forms.
pub const WORDS: [&str; 8] = [
    "math.round.places",
    "math.round.places.",
    "math.signif",
    "math.signif.",
    "string.fixed",
    "string.fixed.",
    "string.sci",
    "string.sci.",
];

#[cfg(test)]
mod tests;
