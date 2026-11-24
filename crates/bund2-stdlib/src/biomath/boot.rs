//! RFC-0019 §S8: the bootstrap and the jackknife, `boot.sample` and
//! `boot.jackknife`.
//!
//! **Data** is a LIST of any values, resampled by element, or a frame
//! (RFC-0017), resampled by row with [`Frame::rows`], which keeps its
//! columns' order and its levels (N12). An empty one is E101.
//!
//! **The statistic** is a LAMBDA `resample -- FLOAT`, run by
//! [`super::dynamics::call1_on`] (E112 for anything but one finite number),
//! or one of the names `"mean"`, `"median"`, `"sd"` and `"var"` over a LIST
//! of numbers (N5, D260 C7). A name computes with the functions behind
//! `stat.mean`, `stat.median`, `stat.sd` and `stat.var` ([`mean`],
//! [`median_of`], [`variance`]), so its replicates are a lambda's calling
//! that word, to the bit. A name with a frame as data is E101 naming
//! `statistic`; `"sd"` and `"var"` over fewer than two values are E101
//! naming `data`.
//!
//! **Resampling** draws from the VM's generator, replicate by replicate:
//! the n `gen_range(0..n)` draws of the first on `u64`, as
//! `rng.choice.replace` draws, then its statistic, then the second. t₀ is
//! computed first and draws nothing. After the same `rng.seed`,
//! `boot.sample` repeats exactly. R is an INTEGER ≥ 2, and R·n above 10⁸ is
//! E111 before any draw (D39).
//!
//! **The answer** is `t0`, `t` (the R replicates), `bias` = mean(t*) − t₀,
//! `se` the replicates' standard deviation on R − 1, and one
//! `[ lower upper ]` per interval in `opts`' `type`: `"normal"`,
//! `"basic"`, `"percentile"` and `"bca"`, as `boot` 1.3-32's `norm.ci`,
//! `basic.ci`, `perc.ci` and `bca.ci` compute them (`R/bootfuns.q`,
//! `:1080-1213`), the order statistics interpolated on the normal-quantile
//! scale by `norm.inter` (`:1119-1149`). `conf` defaults to 0.95 and
//! `type` to `[ "percentile" ]`. [`summarise`] is that arithmetic, and
//! draws nothing.
//!
//! **BCa's acceleration** is the jackknife's (N4, option A):
//! a = Σ(t̄₍.₎ − t₍ᵢ₎)³ / (6(Σ(t̄₍.₎ − t₍ᵢ₎)²)^1.5), the leave-one-out values
//! `boot.jackknife`'s, computed only when `"bca"` is asked for and then
//! answered as `jack`. `boot`'s own `usual.jack` takes t₀ − t₍ᵢ₎ in place
//! of t̄₍.₎ − t₍ᵢ₎ (`:1785`); the two agree where t̄₍.₎ = t₀, as for the
//! mean. Every replicate on one side of t₀ (w infinite) or a constant
//! jackknife (a not finite) is E101 naming `statistic`, where `boot` stops
//! (`:1206`, `:1212`). An adjusted level of 1, past which `norm.inter`
//! would index beyond the last replicate, is E101 the same way.
//!
//! **`boot.jackknife`** answers `t0`, `values` (the n leave-one-out
//! values), `bias` = (n − 1)(t̄₍.₎ − t₀) and
//! `se` = √((n − 1)/n·Σ(t₍ᵢ₎ − t̄₍.₎)²). n < 2 is E101.
//!
//! **Effects.** Both may call a lambda, so both are opaque, plain only,
//! whichever statistic they are given (§S1).

use std::collections::BTreeMap;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, FLOAT, LAMBDA, LIST, MAP, STRING};
use rand::Rng as _;

use super::describe::{mean, median_of, variance};
use super::dist::{Family, normal_tails};
use super::dynamics::{call1_on, lambda};
use super::frame::check::{Frame, check};
use super::operand::{WIDTH, domain, finite, finite_all, list, operands, whole};
use super::refuse::Refusal;
use super::rng::with_rng;

/// The most draws one `boot.sample` makes, R·n (D39).
const DRAW_CAP: u64 = 100_000_000;

/// The data operand: elements or rows.
enum Data {
    List(Vec<BundValue>),
    Frame(Frame),
}

impl Data {
    fn len(&self) -> usize {
        match self {
            Data::List(xs) => xs.len(),
            Data::Frame(f) => f.nrow,
        }
    }

    /// The resample of the given elements or rows, in that order.
    fn pick(&self, rows: &[usize]) -> BundValue {
        match self {
            Data::List(xs) => {
                BundValue::list(rows.iter().filter_map(|&r| xs.get(r).cloned()).collect())
            }
            Data::Frame(f) => f.rows(rows).to_value(),
        }
    }
}

fn read_data(word: &str, v: &BundValue) -> Result<Data, Error> {
    let d = match v.dt() {
        LIST => Data::List(v.as_list().unwrap_or(&[]).to_vec()),
        MAP => Data::Frame(check(word, "data", v)?),
        _ => return Err(domain(word, "data", v.summary(WIDTH))),
    };
    if d.len() == 0 {
        let what = match d {
            Data::List(_) => "an empty LIST",
            Data::Frame(_) => "a frame of 0 rows",
        };
        return Err(domain(word, "data", what.into()));
    }
    Ok(d)
}

/// A named statistic (N5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Named {
    Mean,
    Median,
    Sd,
    Var,
}

impl Named {
    fn name(self) -> &'static str {
        match self {
            Named::Mean => "mean",
            Named::Median => "median",
            Named::Sd => "sd",
            Named::Var => "var",
        }
    }

    /// The fewest values it takes.
    fn least(self) -> usize {
        match self {
            Named::Mean | Named::Median => 1,
            Named::Sd | Named::Var => 2,
        }
    }

    fn of(self, xs: &[f64]) -> f64 {
        match self {
            Named::Mean => mean(xs),
            Named::Median => median_of(xs),
            Named::Sd => variance(xs).sqrt(),
            Named::Var => variance(xs),
        }
    }
}

enum Statistic {
    Lambda(Vec<BundValue>),
    /// The name and the data's numbers.
    Named(Named, Vec<f64>),
}

fn read_statistic(
    word: &str,
    v: &BundValue,
    data: &Data,
    raw: &BundValue,
) -> Result<Statistic, Error> {
    if v.dt() == LAMBDA {
        return Ok(Statistic::Lambda(lambda(word, "statistic", v)?.to_vec()));
    }
    let named = match v.as_str() {
        Some(s) if v.dt() == STRING => match s.as_str() {
            "mean" => Named::Mean,
            "median" => Named::Median,
            "sd" => Named::Sd,
            "var" => Named::Var,
            _ => return Err(domain(word, "statistic", v.summary(WIDTH))),
        },
        _ => return Err(domain(word, "statistic", v.summary(WIDTH))),
    };
    if let Data::Frame(_) = data {
        return Err(domain(
            word,
            "statistic",
            format!("{:?} with a frame as data", named.name()),
        ));
    }
    let xs = whole(word, "data", raw)?;
    finite_all(word, "data", &xs)?;
    Ok(Statistic::Named(named, xs))
}

/// The statistic of the resample at `rows`, or of the data as given.
fn evaluate(
    vm: &mut dyn Vm,
    word: &str,
    s: &Statistic,
    data: &Data,
    raw: &BundValue,
    rows: Option<&[usize]>,
) -> Result<f64, Error> {
    match s {
        Statistic::Lambda(f) => {
            let x = rows.map_or_else(|| raw.clone(), |r| data.pick(r));
            call1_on(vm, word, f, x)
        }
        Statistic::Named(named, all) => {
            let xs: Vec<f64> = match rows {
                Some(r) => r.iter().filter_map(|&i| all.get(i).copied()).collect(),
                None => all.clone(),
            };
            if xs.len() < named.least() {
                return Err(domain(
                    word,
                    "data",
                    format!(
                        "{} values, where {:?} takes at least {}",
                        xs.len(),
                        named.name(),
                        named.least()
                    ),
                ));
            }
            finite(word, named.of(&xs), || format!("the {}", named.name()))
        }
    }
}

/// The n leave-one-out values.
fn jackknife(
    vm: &mut dyn Vm,
    word: &str,
    s: &Statistic,
    data: &Data,
    raw: &BundValue,
) -> Result<Vec<f64>, Error> {
    let n = data.len();
    if n < 2 {
        return Err(domain(
            word,
            "data",
            format!("{n} value, where the jackknife takes at least 2"),
        ));
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let rows: Vec<usize> = (0..n).filter(|&j| j != i).collect();
        out.push(evaluate(vm, word, s, data, raw, Some(&rows))?);
    }
    Ok(out)
}

/// An interval `boot.sample` computes (§S8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Normal,
    Basic,
    Percentile,
    Bca,
}

impl Kind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Kind::Normal => "normal",
            Kind::Basic => "basic",
            Kind::Percentile => "percentile",
            Kind::Bca => "bca",
        }
    }
}

/// §S8's arithmetic over drawn replicates: bias, se and the intervals.
#[derive(Debug)]
pub(crate) struct Summary {
    pub(crate) bias: f64,
    pub(crate) se: f64,
    pub(crate) intervals: Vec<(Kind, [f64; 2])>,
}

fn qnorm(word: &str, p: f64) -> Result<f64, Error> {
    Family::Normal { mean: 0.0, sd: 1.0 }.quantile(word, p)
}

/// `norm.inter` (`bootfuns.q:1119-1149`) at one level, over the sorted
/// replicates: the order statistic (R + 1)·α when it is whole, the first
/// below 1 and the last at R, else interpolated between its neighbours on
/// the normal-quantile scale.
fn norm_inter(word: &str, t: &[f64], alpha: f64) -> Result<f64, Error> {
    let r = t.len();
    if alpha.is_nan() || alpha >= 1.0 {
        return Err(domain(
            word,
            "statistic",
            format!("BCa's adjusted level {alpha}"),
        ));
    }
    let rk = (r + 1) as f64 * alpha;
    let k = rk.trunc();
    let ki = k as usize;
    if ki == 0 {
        return Ok(t[0]);
    }
    if ki >= r {
        return Ok(t[r - 1]);
    }
    if k == rk {
        return Ok(t[ki - 1]);
    }
    let z = qnorm(word, alpha)?;
    let z2 = qnorm(word, k / (r + 1) as f64)?;
    let z3 = qnorm(word, (k + 1.0) / (r + 1) as f64)?;
    let (tk, tk1) = (t[ki - 1], t[ki]);
    Ok(tk + (z - z2) / (z3 - z2) * (tk1 - tk))
}

/// §S8's one function: from t₀, the R ≥ 2 finite replicates, the jackknife
/// values when `"bca"` is asked for, `conf` and the kinds, the bias, the
/// standard error and each interval, as `boot`'s `norm.ci`, `basic.ci`,
/// `perc.ci` and `bca.ci`. It draws nothing.
pub(crate) fn summarise(
    word: &str,
    t0: f64,
    t: &[f64],
    jack: Option<&[f64]>,
    conf: f64,
    kinds: &[Kind],
) -> Result<Summary, Error> {
    let bias = finite(word, mean(t) - t0, || "the bias".into())?;
    let se = finite(word, variance(t).sqrt(), || "the standard error".into())?;
    let mut sorted = t.to_vec();
    sorted.sort_by(f64::total_cmp);
    let lo = (1.0 - conf) / 2.0;
    let hi = (1.0 + conf) / 2.0;
    let mut intervals = Vec::with_capacity(kinds.len());
    for &kind in kinds {
        let ends = match kind {
            Kind::Normal => {
                let merr = se * qnorm(word, hi)?;
                [t0 - bias - merr, t0 - bias + merr]
            }
            Kind::Basic => [
                2.0 * t0 - norm_inter(word, &sorted, hi)?,
                2.0 * t0 - norm_inter(word, &sorted, lo)?,
            ],
            Kind::Percentile => [
                norm_inter(word, &sorted, lo)?,
                norm_inter(word, &sorted, hi)?,
            ],
            Kind::Bca => {
                let below = t.iter().filter(|x| **x < t0).count();
                if below == 0 || below == t.len() {
                    let side = if below == 0 { "at or above" } else { "below" };
                    return Err(domain(
                        word,
                        "statistic",
                        format!("every replicate {side} t0, so BCa's w is infinite"),
                    ));
                }
                let w = qnorm(word, below as f64 / t.len() as f64)?;
                let jack = jack.unwrap_or(&[]);
                let tbar = mean(jack);
                let (s2, s3) = jack.iter().fold((0.0, 0.0), |(s2, s3), ti| {
                    let d = tbar - ti;
                    (s2 + d * d, s3 + d * d * d)
                });
                let a = s3 / (6.0 * s2.powf(1.5));
                if !a.is_finite() {
                    return Err(domain(
                        word,
                        "statistic",
                        "a constant jackknife, so BCa's a is not finite".into(),
                    ));
                }
                let mut ends = [0.0; 2];
                for (end, level) in ends.iter_mut().zip([lo, hi]) {
                    let z = qnorm(word, level)?;
                    let adj = normal_tails(w + (w + z) / (1.0 - a * (w + z))).0;
                    *end = norm_inter(word, &sorted, adj)?;
                }
                ends
            }
        };
        for (end, which) in ends.iter().zip(["lower", "upper"]) {
            finite(word, *end, || {
                format!("the {} interval's {which} end", kind.name())
            })?;
        }
        intervals.push((kind, ends));
    }
    Ok(Summary {
        bias,
        se,
        intervals,
    })
}

/// `boot.sample`'s options: `conf` and `type`, each E101 naming its key.
fn read_opts(word: &str, v: &BundValue) -> Result<(f64, Vec<Kind>), Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(word, "opts", v.summary(WIDTH))),
    };
    let mut conf = 0.95;
    let mut kinds = vec![Kind::Percentile];
    for (k, x) in m {
        match k.as_str() {
            "conf" => {
                // A FLOAT: no INTEGER lies in (0, 1).
                conf = match x.unboxed() {
                    BundValue::Float(c, _) if x.dt() == FLOAT && *c > 0.0 && *c < 1.0 => *c,
                    _ => return Err(domain(word, k, x.summary(WIDTH))),
                };
            }
            "type" => {
                let bad = || domain(word, k, x.summary(WIDTH));
                let items = match x.as_list() {
                    Some(items) if x.dt() == LIST && !items.is_empty() => items,
                    _ => return Err(bad()),
                };
                kinds.clear();
                for item in items {
                    let kind = match item.as_str() {
                        Some(s) if item.dt() == STRING => match s.as_str() {
                            "normal" => Kind::Normal,
                            "basic" => Kind::Basic,
                            "percentile" => Kind::Percentile,
                            "bca" => Kind::Bca,
                            _ => return Err(bad()),
                        },
                        _ => return Err(bad()),
                    };
                    if kinds.contains(&kind) {
                        return Err(bad());
                    }
                    kinds.push(kind);
                }
            }
            _ => return Err(domain(word, "opts", format!("an unknown key {k:?}"))),
        }
    }
    Ok((conf, kinds))
}

/// `boot.sample` (`data statistic R opts -- MAP`).
fn boot_sample(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "boot.sample";
    let v = operands(vm, W, 4)?;
    let data = read_data(W, &v[0])?;
    let stat = read_statistic(W, &v[1], &data, &v[0])?;
    let r = match v[2].unboxed() {
        BundValue::Int(r, _) if *r >= 2 => r.unsigned_abs(),
        _ => return Err(domain(W, "R", v[2].summary(WIDTH))),
    };
    let (conf, kinds) = read_opts(W, &v[3])?;
    let n = data.len();
    if r.saturating_mul(n as u64) > DRAW_CAP {
        return Err(Refusal::Cap {
            what: format!("{r} replicates of {n} draws"),
            cap: DRAW_CAP,
        }
        .error(W));
    }
    let t0 = evaluate(vm, W, &stat, &data, &v[0], None)?;
    let mut t = Vec::with_capacity(r as usize);
    for _ in 0..r {
        let rows: Vec<usize> = with_rng(vm, |g| {
            (0..n).map(|_| g.gen_range(0..n as u64) as usize).collect()
        });
        t.push(evaluate(vm, W, &stat, &data, &v[0], Some(&rows))?);
    }
    let jack = if kinds.contains(&Kind::Bca) {
        Some(jackknife(vm, W, &stat, &data, &v[0])?)
    } else {
        None
    };
    let s = summarise(W, t0, &t, jack.as_deref(), conf, &kinds)?;
    let mut m = BTreeMap::new();
    m.insert("t0".to_string(), BundValue::float(t0));
    m.insert("t".to_string(), list(t));
    m.insert("bias".to_string(), BundValue::float(s.bias));
    m.insert("se".to_string(), BundValue::float(s.se));
    for (kind, ends) in s.intervals {
        m.insert(kind.name().to_string(), list(ends.to_vec()));
    }
    if let Some(jack) = jack {
        m.insert("jack".to_string(), list(jack));
    }
    vm.push(BundValue::map(m));
    Ok(())
}

/// `boot.jackknife` (`data statistic -- MAP`).
fn boot_jackknife(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "boot.jackknife";
    let v = operands(vm, W, 2)?;
    let data = read_data(W, &v[0])?;
    let stat = read_statistic(W, &v[1], &data, &v[0])?;
    let t0 = evaluate(vm, W, &stat, &data, &v[0], None)?;
    let values = jackknife(vm, W, &stat, &data, &v[0])?;
    let n = values.len() as f64;
    let tbar = mean(&values);
    let bias = finite(W, (n - 1.0) * (tbar - t0), || "the bias".into())?;
    let ss: f64 = values.iter().map(|x| (x - tbar) * (x - tbar)).sum();
    let se = finite(W, ((n - 1.0) / n * ss).sqrt(), || {
        "the standard error".into()
    })?;
    let mut m = BTreeMap::new();
    m.insert("t0".to_string(), BundValue::float(t0));
    m.insert("values".to_string(), list(values));
    m.insert("bias".to_string(), BundValue::float(bias));
    m.insert("se".to_string(), BundValue::float(se));
    vm.push(BundValue::map(m));
    Ok(())
}

/// Register §S8's words, opaque and plain only (§S1).
pub fn register(r: &mut Registry) {
    r.register_native(
        "boot.sample",
        boot_sample,
        StackEffect::opaque(4),
        WordKind::Sync,
    );
    r.register_native(
        "boot.jackknife",
        boot_jackknife,
        StackEffect::opaque(2),
        WordKind::Sync,
    );
}

/// The `boot.` words, which have no `.` form.
pub const WORDS: [&str; 2] = ["boot.sample", "boot.jackknife"];

#[cfg(test)]
mod tests;
