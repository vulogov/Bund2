//! RFC-0015 §S5: element-wise series arithmetic, the `vec.` words.
//!
//! `+ - * /` keep their LIST meanings (§S1), so each operation here is a
//! word of its own. The binary words take two whole series (§S3: a LIST or
//! a METRICS each, the top the second parameter, a drain E115); the others
//! take one series by the house reader, in four forms, and `vec.scale` a
//! series beneath its factor.
//!
//! Every answer is checked: an element, sum or product that overflows is
//! E114, and a zero divisor is E101 (§S5), so no word answers ±∞ or NaN
//! [B12].

use bund2_api::{Error, Registry, Vm};
use bund2_value::BundValue;

use super::operand::{
    at_least, domain, finite, finite_list, four_forms, list, same_length, scalar_forms, two,
    two_forms,
};

/// a ∘ b element by element, equal lengths.
fn elementwise(
    vm: &mut dyn Vm,
    word: &str,
    sym: &str,
    op: fn(f64, f64) -> f64,
) -> Result<(), Error> {
    let (a, b) = two(vm, word)?;
    same_length(word, &a, &b)?;
    if sym == "/"
        && let Some(i) = b.iter().position(|&y| y == 0.0)
    {
        return Err(domain(word, "b", format!("0 at {}", i + 1)));
    }
    let c: Vec<f64> = a.iter().zip(&b).map(|(&x, &y)| op(x, y)).collect();
    let c = finite_list(word, &format!("a {sym} b"), c)?;
    vm.push(c);
    Ok(())
}

fn vec_add(vm: &mut dyn Vm) -> Result<(), Error> {
    elementwise(vm, "vec.add", "+", |x, y| x + y)
}

fn vec_sub(vm: &mut dyn Vm) -> Result<(), Error> {
    elementwise(vm, "vec.sub", "−", |x, y| x - y)
}

fn vec_mul(vm: &mut dyn Vm) -> Result<(), Error> {
    elementwise(vm, "vec.mul", "×", |x, y| x * y)
}

fn vec_div(vm: &mut dyn Vm) -> Result<(), Error> {
    elementwise(vm, "vec.div", "/", |x, y| x / y)
}

fn vec_dot(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "vec.dot";
    let (a, b) = two(vm, W)?;
    same_length(W, &a, &b)?;
    let y: f64 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
    let y = finite(W, y, || "Σ aᵢbᵢ".into())?;
    vm.push(BundValue::float(y));
    Ok(())
}

fn scale(word: &str, xs: &[f64], k: f64) -> Result<BundValue, Error> {
    finite_list(word, "a × k", xs.iter().map(|x| x * k).collect())
}

fn sum(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    let y = finite(word, xs.iter().sum(), || "Σ xᵢ".into())?;
    Ok(BundValue::float(y))
}

fn prod(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    let y = finite(word, xs.iter().product(), || "Π xᵢ".into())?;
    Ok(BundValue::float(y))
}

fn cumsum(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    let mut acc = 0.0;
    let ys = xs
        .iter()
        .map(|x| {
            acc += x;
            acc
        })
        .collect();
    finite_list(word, "the running sum", ys)
}

fn cumprod(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    let mut acc = 1.0;
    let ys = xs
        .iter()
        .map(|x| {
            acc *= x;
            acc
        })
        .collect();
    finite_list(word, "the running product", ys)
}

/// x₂ − x₁, x₃ − x₂, …: one shorter than the series, empty for one value.
fn diff(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    finite_list(
        word,
        "the difference",
        xs.windows(2).map(|w| w[1] - w[0]).collect(),
    )
}

/// ln Σ e^xᵢ as m + ln Σ e^(xᵢ − m), m the largest, so no term overflows.
fn logsumexp(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    at_least(word, xs, 1)?;
    let m = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let s: f64 = xs.iter().map(|x| (x - m).exp()).sum();
    let y = finite(word, m + s.ln(), || "ln Σ e^xᵢ".into())?;
    Ok(BundValue::float(y))
}

/// Each over the sum, so counts become frequencies.
fn normalize_sum(word: &str, xs: &[f64]) -> Result<BundValue, Error> {
    let s: f64 = finite(word, xs.iter().sum(), || "Σ xᵢ".into())?;
    if s == 0.0 {
        return Err(domain(word, "series", "a series summing to 0".into()));
    }
    Ok(list(xs.iter().map(|x| x / s).collect()))
}

pub fn register(r: &mut Registry) {
    two_forms!(r, "vec.add", vec_add);
    two_forms!(r, "vec.sub", vec_sub);
    two_forms!(r, "vec.mul", vec_mul);
    two_forms!(r, "vec.div", vec_div);
    scalar_forms!(r, "vec.scale", "k", scale);
    two_forms!(r, "vec.dot", vec_dot);
    four_forms!(r, "vec.sum", sum);
    four_forms!(r, "vec.prod", prod);
    four_forms!(r, "vec.cumsum", cumsum);
    four_forms!(r, "vec.cumprod", cumprod);
    four_forms!(r, "vec.diff", diff);
    four_forms!(r, "vec.logsumexp", logsumexp);
    four_forms!(r, "vec.normalize.sum", normalize_sum);
}

/// S5's words, each with every form §S3 gives it: criterion 2's list.
pub const WORDS: [&str; 40] = [
    "vec.add",
    "vec.add.",
    "vec.sub",
    "vec.sub.",
    "vec.mul",
    "vec.mul.",
    "vec.div",
    "vec.div.",
    "vec.scale",
    "vec.scale.",
    "vec.dot",
    "vec.dot.",
    "vec.sum",
    "vec.sum,",
    "vec.sum.",
    "vec.sum.,",
    "vec.prod",
    "vec.prod,",
    "vec.prod.",
    "vec.prod.,",
    "vec.cumsum",
    "vec.cumsum,",
    "vec.cumsum.",
    "vec.cumsum.,",
    "vec.cumprod",
    "vec.cumprod,",
    "vec.cumprod.",
    "vec.cumprod.,",
    "vec.diff",
    "vec.diff,",
    "vec.diff.",
    "vec.diff.,",
    "vec.logsumexp",
    "vec.logsumexp,",
    "vec.logsumexp.",
    "vec.logsumexp.,",
    "vec.normalize.sum",
    "vec.normalize.sum,",
    "vec.normalize.sum.",
    "vec.normalize.sum.,",
];

#[cfg(test)]
mod tests;
