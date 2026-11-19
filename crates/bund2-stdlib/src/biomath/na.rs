//! RFC-0017 §S3: missing values, `na`, `na.is`, `na.count`, `na.omit` and
//! `na.fill`.
//!
//! **A missing value is NONE** (D253 P1), the value JSON's `null` converts
//! to, with q 0.0 as `json.to_value` gives it. `na.is` exists because `==`
//! refuses NONE. None of these words takes the `na` option: they move, test,
//! count or make NONE, and do not compute from cells (D252's third note).

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST, NONE};

use super::operand::{WIDTH, domain, operands};

/// NONE as a frame word makes it: q 0.0, as JSON's `null` gives it, not
/// `BundValue::none()`'s 100.0 (§S2).
pub(crate) fn none() -> BundValue {
    BundValue::none().with_q(0.0)
}

/// Whether a value is NONE, boxed or not.
pub(crate) fn is_none(v: &BundValue) -> bool {
    v.dt() == NONE
}

/// The LIST operand's items: E101 for anything else.
fn items<'a>(word: &str, v: &'a BundValue) -> Result<&'a [BundValue], Error> {
    match v.as_list() {
        Some(xs) if v.dt() == LIST => Ok(xs),
        _ => Err(domain(word, "list", v.summary(WIDTH))),
    }
}

/// `na` (`-- x`).
fn na(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(none());
    Ok(())
}

/// `na.is` (`x -- bool`): any value, never refused.
fn na_is(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = operands(vm, "na.is", 1)?;
    vm.push(BundValue::boolean(is_none(&v[0])));
    Ok(())
}

/// `na.count` (`list -- n`).
fn na_count(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "na.count";
    let v = operands(vm, W, 1)?;
    let n = items(W, &v[0])?.iter().filter(|x| is_none(x)).count();
    vm.push(BundValue::int(i64::try_from(n).unwrap_or(i64::MAX)));
    Ok(())
}

/// `na.omit` (`list -- list`): the cells that are not NONE, in order.
fn na_omit(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "na.omit";
    let v = operands(vm, W, 1)?;
    let kept = items(W, &v[0])?
        .iter()
        .filter(|x| !is_none(x))
        .cloned()
        .collect();
    vm.push(BundValue::list(kept));
    Ok(())
}

/// `na.fill` (`list x -- list`): each NONE replaced by x.
fn na_fill(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "na.fill";
    let v = operands(vm, W, 2)?;
    let filled = items(W, &v[0])?
        .iter()
        .map(|c| if is_none(c) { v[1].clone() } else { c.clone() })
        .collect();
    vm.push(BundValue::list(filled));
    Ok(())
}

/// Register §S3's words: `na` plain only, since it consumes nothing; the
/// rest plain and `.`, the `.` form taking the top operand from the
/// workbench (RFC-0015 §S3).
pub fn register(r: &mut Registry) {
    r.register_native("na", na, StackEffect::fixed(0, 1), WordKind::Sync);
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("na.is", na_is, 1);
    word!("na.count", na_count, 1);
    word!("na.omit", na_omit, 1);
    word!("na.fill", na_fill, 2);
}

/// §S3's words with their forms.
pub const WORDS: [&str; 9] = [
    "na",
    "na.count",
    "na.count.",
    "na.fill",
    "na.fill.",
    "na.is",
    "na.is.",
    "na.omit",
    "na.omit.",
];

#[cfg(test)]
mod tests;
