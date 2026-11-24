//! RFC-0019 §S9: the two-sample permutation test, `stat.test.permutation`
//! (`a b statistic opts -- MAP`).
//!
//! **The samples** are two non-empty LISTs. A lambda statistic receives
//! them as they are; a named one reads them as numbers (E115, E103 and
//! E101 as every series reader refuses).
//!
//! **The statistic** is a LAMBDA `a b -- FLOAT`, run through
//! [`super::dynamics::call1_on`] with `b` pushed above `a`, as
//! `dynamics.rs`'s `call` pushes its arguments (E112 for anything but one
//! finite number), or one of the names (N5, D260 C7):
//!
//! - `"mean.diff"`: mean(a) − mean(b);
//! - `"median.diff"`: median(a) − median(b);
//! - `"t"`: Welch's t, (mean(a) − mean(b))/√(var(a)/n + var(b)/m), as
//!   `stat.test.welch` computes its statistic; each sample needs two
//!   values, and a constant a and b is E101 naming them, as there.
//!
//! The names compute with `describe.rs`'s [`mean`], [`median_of`] and
//! [`variance`], so a name's statistic is a lambda's calling `stat.mean`,
//! `stat.median` or `stat.var`, to the bit. There is no default: NONE, or
//! anything but a LAMBDA or a name, is E101 naming `statistic` (D269, 9).
//! A split under `"t"` whose halves are each constant gives ±∞, which
//! compares as the most extreme value; the observed t is finite, since a
//! and b are not both constant.
//!
//! **Exact** when C(n + m, n) ≤ 10⁶ (D267, 4): every split is enumerated,
//! a's positions in the pooled sample a ++ b as the combinations of n of
//! n + m in lexicographic order, the first being the observed split; each
//! half keeps the pooled order. p is the share of splits as or more
//! extreme. **Monte Carlo** otherwise, or when `exact` is false (D274): B
//! shuffles, each `rng.shuffle`'s Fisher–Yates on the pooled sample as
//! given (`sample.rs`), a the first n and b the rest, and
//! p = (1 + #{as or more extreme})/(B + 1).
//!
//! **As extreme** is |T| ≥ |T₀| two-sided, T ≥ T₀ for `"greater"` and
//! T ≤ T₀ for `"less"`, each within 10⁻¹²|T₀|, so that splits tied with
//! T₀ in exact arithmetic count.
//!
//! **`opts`** (D274): `alternative`, R's three values, default
//! `"two.sided"`; `exact`, a BOOL: left out, exact up to the cap and Monte
//! Carlo above it; false, Monte Carlo; true above the cap, E111, as
//! `stat.test.wilcoxon.opts`' `exact` above its own; `B`, an INTEGER ≥ 1,
//! default 9999, read only under Monte Carlo. Any other key is E101.
//!
//! **Caps** (D39), each E111 before any statistic: B·(n + m) above 10⁸
//! under Monte Carlo (§S9), and C(n + m, n)·(n + m) above 10⁸ under the
//! exact test, §S1's work cap, since a split costs n + m as a shuffle
//! does.
//!
//! **The answer** is `statistic` (T₀), `p`, `method` (`"exact
//! permutation"` or `"Monte Carlo permutation"`), `permutations` (the
//! splits enumerated, or B) and `alternative`. The word may call a
//! lambda, so it is opaque, plain only (§S1).

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BOOL, BundValue, LAMBDA, LIST, MAP, STRING};
use rand::Rng as _;

use super::describe::{mean, median_of, variance};
use super::dynamics::{call1_on, lambda};
use super::operand::{WIDTH, domain, finite, operands, whole};
use super::refuse::Refusal;
use super::rng::with_rng;

const W: &str = "stat.test.permutation";

/// The most splits the exact test enumerates (D267, 4).
const EXACT_CAP: u64 = 1_000_000;

/// The most elements one test arranges, B·(n + m) or C·(n + m) (D39).
const WORK_CAP: u64 = 100_000_000;

/// B's default (§S9).
const B_DEFAULT: u64 = 9_999;

/// The relative tolerance of "as extreme" (§S9).
const TIE: f64 = 1e-12;

/// The hypothesis `alternative` names as H₁.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Alternative {
    TwoSided,
    Less,
    Greater,
}

impl Alternative {
    fn name(self) -> &'static str {
        match self {
            Alternative::TwoSided => "two.sided",
            Alternative::Less => "less",
            Alternative::Greater => "greater",
        }
    }

    /// Whether `t` is as or more extreme than `t0`.
    fn extreme(self, t: f64, t0: f64) -> bool {
        let tol = TIE * t0.abs();
        match self {
            Alternative::TwoSided => t.abs() >= t0.abs() - tol,
            Alternative::Greater => t >= t0 - tol,
            Alternative::Less => t <= t0 + tol,
        }
    }
}

/// `opts`, read (D274).
struct Opts {
    alternative: Alternative,
    exact: Option<bool>,
    b: u64,
}

fn read_opts(v: &BundValue) -> Result<Opts, Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(W, "opts", v.summary(WIDTH))),
    };
    let mut o = Opts {
        alternative: Alternative::TwoSided,
        exact: None,
        b: B_DEFAULT,
    };
    for (k, x) in m {
        let bad = || domain(W, k, x.summary(WIDTH));
        match k.as_str() {
            "alternative" => {
                o.alternative = match x.as_str() {
                    Some(s) if x.dt() == STRING => match s.as_str() {
                        "two.sided" => Alternative::TwoSided,
                        "less" => Alternative::Less,
                        "greater" => Alternative::Greater,
                        _ => return Err(bad()),
                    },
                    _ => return Err(bad()),
                };
            }
            "exact" => {
                o.exact = match x.unboxed() {
                    BundValue::Bool(b, _) if x.dt() == BOOL => Some(*b),
                    _ => return Err(bad()),
                };
            }
            "B" => {
                o.b = match x.unboxed() {
                    BundValue::Int(b, _) if *b >= 1 => b.unsigned_abs(),
                    _ => return Err(bad()),
                };
            }
            _ => return Err(domain(W, "opts", format!("an unknown key {k:?}"))),
        }
    }
    Ok(o)
}

/// A sample: a non-empty LIST.
fn sample(name: &str, v: &BundValue) -> Result<Vec<BundValue>, Error> {
    let items = match v.as_list() {
        Some(items) if v.dt() == LIST => items,
        _ => return Err(domain(W, name, v.summary(WIDTH))),
    };
    if items.is_empty() {
        return Err(domain(W, name, "an empty LIST".into()));
    }
    Ok(items.to_vec())
}

/// A named statistic (N5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Named {
    MeanDiff,
    MedianDiff,
    T,
}

impl Named {
    fn of(self, a: &[f64], b: &[f64]) -> f64 {
        match self {
            Named::MeanDiff => mean(a) - mean(b),
            Named::MedianDiff => median_of(a) - median_of(b),
            Named::T => {
                let se2 = variance(a) / a.len() as f64 + variance(b) / b.len() as f64;
                (mean(a) - mean(b)) / se2.sqrt()
            }
        }
    }
}

enum Statistic {
    Lambda(Vec<BundValue>),
    /// The name and the pooled numbers, a's first.
    Named(Named, Vec<f64>),
}

fn read_statistic(
    v: &BundValue,
    raw: (&BundValue, &BundValue),
    n: usize,
    m: usize,
) -> Result<Statistic, Error> {
    if v.dt() == LAMBDA {
        return Ok(Statistic::Lambda(lambda(W, "statistic", v)?.to_vec()));
    }
    let named = match v.as_str() {
        Some(s) if v.dt() == STRING => match s.as_str() {
            "mean.diff" => Named::MeanDiff,
            "median.diff" => Named::MedianDiff,
            "t" => Named::T,
            _ => return Err(domain(W, "statistic", v.summary(WIDTH))),
        },
        _ => return Err(domain(W, "statistic", v.summary(WIDTH))),
    };
    let a = whole(W, "a", raw.0)?;
    let b = whole(W, "b", raw.1)?;
    if named == Named::T {
        for (name, k) in [("a", n), ("b", m)] {
            if k < 2 {
                return Err(domain(
                    W,
                    name,
                    format!("{k} value, where \"t\" takes at least 2"),
                ));
            }
        }
        if variance(&a) == 0.0 && variance(&b) == 0.0 {
            return Err(domain(W, "a and b", "a constant series".into()));
        }
    }
    let mut pooled = a;
    pooled.extend(b);
    Ok(Statistic::Named(named, pooled))
}

/// The statistic of the split putting the pooled positions `left` in a and
/// `right` in b.
fn evaluate(
    vm: &mut dyn Vm,
    s: &Statistic,
    pooled: &[BundValue],
    left: &[usize],
    right: &[usize],
) -> Result<f64, Error> {
    match s {
        Statistic::Lambda(f) => {
            let pick = |ix: &[usize]| {
                BundValue::list(ix.iter().filter_map(|&i| pooled.get(i).cloned()).collect())
            };
            let mut body = Vec::with_capacity(f.len() + 1);
            body.push(pick(right));
            body.extend(f.iter().cloned());
            call1_on(vm, W, &body, pick(left))
        }
        Statistic::Named(named, xs) => {
            let pick = |ix: &[usize]| -> Vec<f64> {
                ix.iter().filter_map(|&i| xs.get(i).copied()).collect()
            };
            Ok(named.of(&pick(left), &pick(right)))
        }
    }
}

/// C(n, k), or None once it passes `cap`. Each partial product is
/// C(n − k + i, i), whole, and they do not decrease, so stopping early is
/// exact.
fn choose_upto(n: u64, k: u64, cap: u64) -> Option<u64> {
    let k = k.min(n - k);
    let mut c: u128 = 1;
    for i in 1..=u128::from(k) {
        c = c * (u128::from(n - k) + i) / i;
        if c > u128::from(cap) {
            return None;
        }
    }
    u64::try_from(c).ok()
}

/// The next combination of `ix.len()` of 0..n in lexicographic order, or
/// false after the last.
fn next_combination(ix: &mut [usize], n: usize) -> bool {
    let k = ix.len();
    let mut i = k;
    while i > 0 {
        i -= 1;
        if ix[i] < n - k + i {
            ix[i] += 1;
            for j in i + 1..k {
                ix[j] = ix[j - 1] + 1;
            }
            return true;
        }
    }
    false
}

/// The positions of 0..n not in the sorted `left`.
fn complement(left: &[usize], n: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(n - left.len());
    let mut it = left.iter().peekable();
    for i in 0..n {
        if it.peek() == Some(&&i) {
            it.next();
        } else {
            out.push(i);
        }
    }
    out
}

fn cap(what: String) -> Error {
    Refusal::Cap {
        what,
        cap: WORK_CAP,
    }
    .error(W)
}

/// `stat.test.permutation` (`a b statistic opts -- MAP`).
fn stat_test_permutation(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = operands(vm, W, 4)?;
    let a = sample("a", &v[0])?;
    let b = sample("b", &v[1])?;
    let (n, m) = (a.len(), b.len());
    let stat = read_statistic(&v[2], (&v[0], &v[1]), n, m)?;
    let o = read_opts(&v[3])?;
    let total = n + m;
    let splits = choose_upto(total as u64, n as u64, EXACT_CAP);
    let exact = match (o.exact, splits) {
        (Some(false), _) | (None, None) => None,
        (Some(true) | None, Some(c)) => Some(c),
        (Some(true), None) => {
            return Err(Refusal::Cap {
                what: format!("an exact test over C({total}, {n}) splits"),
                cap: EXACT_CAP,
            }
            .error(W));
        }
    };
    let per = total as u64;
    match exact {
        Some(c) if c.saturating_mul(per) > WORK_CAP => {
            return Err(cap(format!("{c} splits of {total} values")));
        }
        None if o.b.saturating_mul(per) > WORK_CAP => {
            return Err(cap(format!("{} shuffles of {total} values", o.b)));
        }
        _ => {}
    }
    let mut pooled = a;
    pooled.extend(b);
    let observed: Vec<usize> = (0..n).collect();
    let rest: Vec<usize> = (n..total).collect();
    let t0 = finite(W, evaluate(vm, &stat, &pooled, &observed, &rest)?, || {
        "the statistic".into()
    })?;
    let (p, method, count) = match exact {
        Some(c) => {
            let mut left = observed;
            let mut hits: u64 = 0;
            loop {
                let right = complement(&left, total);
                if o.alternative
                    .extreme(evaluate(vm, &stat, &pooled, &left, &right)?, t0)
                {
                    hits += 1;
                }
                if !next_combination(&mut left, total) {
                    break;
                }
            }
            (hits as f64 / c as f64, "exact permutation", c)
        }
        None => {
            let mut hits: u64 = 0;
            for _ in 0..o.b {
                let mut ix: Vec<usize> = (0..total).collect();
                with_rng(vm, |g| {
                    for i in (1..total).rev() {
                        let j = g.gen_range(0..=i as u64) as usize;
                        ix.swap(i, j);
                    }
                });
                if o.alternative
                    .extreme(evaluate(vm, &stat, &pooled, &ix[..n], &ix[n..])?, t0)
                {
                    hits += 1;
                }
            }
            (
                (1.0 + hits as f64) / (o.b as f64 + 1.0),
                "Monte Carlo permutation",
                o.b,
            )
        }
    };
    let mut out = BTreeMap::new();
    out.insert("statistic".to_string(), BundValue::float(t0));
    out.insert("p".to_string(), BundValue::float(p));
    out.insert("method".to_string(), BundValue::str(method));
    out.insert(
        "permutations".to_string(),
        BundValue::int(i64::try_from(count).unwrap_or(i64::MAX)),
    );
    out.insert(
        "alternative".to_string(),
        BundValue::str(o.alternative.name()),
    );
    vm.push(BundValue::map(out));
    Ok(())
}

/// Register §S9's word, opaque and plain only (§S1).
pub fn register(r: &mut Registry) {
    r.register_native(
        W,
        stat_test_permutation,
        StackEffect::opaque(4),
        WordKind::Sync,
    );
}

/// The word, which has no `.` form.
pub const WORDS: [&str; 1] = ["stat.test.permutation"];

#[cfg(test)]
mod tests;
