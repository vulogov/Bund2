//! RFC-0016 §S7: shuffling and sampling, `rng.shuffle`, `rng.choice`,
//! `rng.choice.replace` and `rng.choice.weighted`.
//!
//! **Draws** come from the VM's generator ([`with_rng`]), so after the same
//! `rng.seed` they repeat within one Bund2 version, as every drawing word
//! of the family does. The algorithms fix the streams, over `rand` 0.8's
//! `StdRng`:
//!
//! - `rng.shuffle`: Fisher–Yates over a copy, for i from n − 1 down to 1,
//!   j = `gen_range(0..=i)` on `u64`, then swap i and j;
//! - `rng.choice`: the first k steps of the same, answering the element
//!   that lands at each i in turn; at k = n the last takes no draw;
//! - `rng.choice.replace`: `gen_range(0..n)` on `u64` per draw;
//! - `rng.choice.weighted`: u = Σw·`gen::<f64>()` per draw, answering the
//!   first i whose cumulative weight is strictly above u, by binary search,
//!   or the last positive-weight element if rounding leaves none.
//!
//! **The list** may hold any values; they are moved, not converted. k = 0
//! answers an empty LIST with no draw. `gen_range` on an empty range
//! panics in `rand`, so an empty list with k ≥ 1 is refused before any
//! draw (D37).

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST};
use rand::Rng as _;

use super::operand::{WIDTH, domain, operands, whole};
use super::refuse::Refusal;
use super::rng::with_rng;

/// The most elements one word draws (D39).
const DRAW_CAP: u64 = 100_000_000;

/// The list operand, any values: a non-LIST is E101.
fn items(word: &str, v: &BundValue) -> Result<Vec<BundValue>, Error> {
    match v.as_list() {
        Some(xs) if v.dt() == LIST => Ok(xs.to_vec()),
        _ => Err(domain(word, "list", v.summary(WIDTH))),
    }
}

/// k: a non-negative INTEGER (E101), at most 10⁸ (E111).
fn count(word: &str, v: &BundValue) -> Result<usize, Error> {
    let k = match v.unboxed() {
        BundValue::Int(k, _) if *k >= 0 => k.unsigned_abs(),
        _ => return Err(domain(word, "k", v.summary(WIDTH))),
    };
    if k > DRAW_CAP {
        return Err(Refusal::Cap {
            what: format!("a draw of {k} elements"),
            cap: DRAW_CAP,
        }
        .error(word));
    }
    usize::try_from(k).map_err(|_| domain(word, "k", v.summary(WIDTH)))
}

/// A uniform index in 0..=i.
fn upto(g: &mut rand::rngs::StdRng, i: usize) -> usize {
    g.gen_range(0..=i as u64) as usize
}

/// `rng.shuffle` (`list -- list`).
fn rng_shuffle(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "rng.shuffle";
    let v = operands(vm, W, 1)?;
    let mut xs = items(W, &v[0])?;
    with_rng(vm, |g| {
        for i in (1..xs.len()).rev() {
            let j = upto(g, i);
            xs.swap(i, j);
        }
    });
    vm.push(BundValue::list(xs));
    Ok(())
}

/// `rng.choice` (`list k -- list`): k elements without replacement.
fn rng_choice(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "rng.choice";
    let v = operands(vm, W, 2)?;
    let k = count(W, &v[1])?;
    let mut xs = items(W, &v[0])?;
    let n = xs.len();
    if k > n {
        return Err(domain(W, "k", format!("{k}, above the length {n}")));
    }
    let mut out = Vec::with_capacity(k);
    with_rng(vm, |g| {
        for step in 0..k {
            let i = n - 1 - step;
            if i > 0 {
                let j = upto(g, i);
                xs.swap(i, j);
            }
            out.push(xs[i].clone());
        }
    });
    vm.push(BundValue::list(out));
    Ok(())
}

/// `rng.choice.replace` (`list k -- list`): k elements with replacement.
fn rng_choice_replace(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "rng.choice.replace";
    let v = operands(vm, W, 2)?;
    let k = count(W, &v[1])?;
    let xs = items(W, &v[0])?;
    if k == 0 {
        vm.push(BundValue::list(Vec::new()));
        return Ok(());
    }
    if xs.is_empty() {
        return Err(domain(W, "list", "an empty list".into()));
    }
    let n = xs.len() as u64;
    let out: Vec<BundValue> = with_rng(vm, |g| {
        (0..k)
            .map(|_| xs[g.gen_range(0..n) as usize].clone())
            .collect()
    });
    vm.push(BundValue::list(out));
    Ok(())
}

/// `rng.choice.weighted` (`list weights k -- list`): k elements with
/// replacement, element i with probability wᵢ/Σw.
fn rng_choice_weighted(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "rng.choice.weighted";
    let v = operands(vm, W, 3)?;
    let k = count(W, &v[2])?;
    let xs = items(W, &v[0])?;
    if v[1].dt() != LIST {
        return Err(domain(W, "weights", v[1].summary(WIDTH)));
    }
    let w = whole(W, "weights", &v[1])?;
    if w.len() != xs.len() {
        return Err(Refusal::Disagree {
            what: "lengths".into(),
            n: xs.len(),
            m: w.len(),
        }
        .error(W));
    }
    if let Some(i) = w.iter().position(|x| *x < 0.0) {
        return Err(domain(W, "weights", format!("{} at {}", w[i], i + 1)));
    }
    if k == 0 {
        vm.push(BundValue::list(Vec::new()));
        return Ok(());
    }
    let mut cum = Vec::with_capacity(w.len());
    let mut total = 0.0;
    for x in &w {
        total += x;
        cum.push(total);
    }
    if total.is_infinite() {
        return Err(Refusal::Overflow {
            expression: "the sum of the weights".into(),
        }
        .error(W));
    }
    if total <= 0.0 {
        return Err(domain(W, "weights", "a sum of 0".into()));
    }
    // The last element with a positive weight, for a u that rounding puts
    // at or above every cumulative weight.
    let last = w.iter().rposition(|x| *x > 0.0).unwrap_or(0);
    let out: Vec<BundValue> = with_rng(vm, |g| {
        (0..k)
            .map(|_| {
                let u = total * g.r#gen::<f64>();
                let i = cum.partition_point(|c| *c <= u);
                xs[if i < xs.len() { i } else { last }].clone()
            })
            .collect()
    });
    vm.push(BundValue::list(out));
    Ok(())
}

/// Register §S7's words, each plain and `.`: `rng.shuffle.` takes the list
/// from the workbench, the others k, the top operand (RFC-0015 §S3).
pub fn register(r: &mut Registry) {
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("rng.shuffle", rng_shuffle, 1);
    word!("rng.choice", rng_choice, 2);
    word!("rng.choice.replace", rng_choice_replace, 2);
    word!("rng.choice.weighted", rng_choice_weighted, 3);
}

/// §S7's words with their forms. RFC-0015's counting test for `dist` and
/// `rng` leaves them out by this list (§Preservation analysis).
pub const WORDS: [&str; 8] = [
    "rng.shuffle",
    "rng.shuffle.",
    "rng.choice",
    "rng.choice.",
    "rng.choice.replace",
    "rng.choice.replace.",
    "rng.choice.weighted",
    "rng.choice.weighted.",
];

#[cfg(test)]
mod tests;
