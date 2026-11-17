//! RFC-0016 §S2: the `.opts` words, each a test's operands with an options
//! MAP above them, answering the plain word's keys for the chosen
//! alternative and `mu`, and the estimate and interval beside them.
//!
//! M2 builds the t tests' and the binomial's: `stat.test.t1.opts`,
//! `t2.opts`, `welch.opts`, `paired.opts` and `binom.opts`.
//!
//! **The plain words are unchanged.** Each `.opts` word computes through
//! the plain word's own code (`TCase`, `binom_p`), so with the defaults,
//! two-sided and `mu` 0, its `statistic`, `p` and `df` are the plain
//! word's, bit for bit (criterion 3).
//!
//! **Intervals** are at level `conf`, two-sided with α = 1 − conf split
//! evenly, one-sided with α on the one end kept: [lower, ∞) for "greater",
//! (−∞, upper] for "less". An infinite end is left out of the answer [A3];
//! Clopper–Pearson's far end is 0 or 1 and is answered.

use std::collections::BTreeMap;

use bund2_api::{Error, Vm};
use bund2_value::{BundValue, FLOAT, MAP, STRING};

use super::{
    Family, TCase, answer_map, binom_operands, binom_p, paired_case, t_one_case, t_p, t2_case,
    welch_case,
};
use crate::biomath::operand::{WIDTH, domain, finite, operands, real, series, whole};
use crate::series::Mode;
use crate::wb::Side;

/// The hypothesis a test's `alternative` names as H₁.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Alternative {
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
}

/// An options MAP, read.
#[derive(Clone, Copy, Debug)]
pub(super) struct Opts {
    pub(super) alternative: Alternative,
    pub(super) conf: f64,
    pub(super) mu: f64,
}

/// The keys any `.opts` word knows.
const KEYS: [&str; 3] = ["alternative", "conf", "mu"];

/// Read an options MAP (S1, "Options MAPs"): E101 for anything but a MAP,
/// an unknown key, a key `takes` does not hold, or a value of the wrong
/// type or range, each naming the key.
pub(super) fn read(word: &str, v: &BundValue, takes: &[&str]) -> Result<Opts, Error> {
    let m = match v.as_map() {
        Some(m) if v.dt() == MAP => m,
        _ => return Err(domain(word, "opts", v.summary(WIDTH))),
    };
    let mut o = Opts {
        alternative: Alternative::TwoSided,
        conf: 0.95,
        mu: 0.0,
    };
    for (k, x) in m {
        if !KEYS.contains(&k.as_str()) {
            return Err(domain(word, "opts", format!("an unknown key {k:?}")));
        }
        if !takes.contains(&k.as_str()) {
            return Err(domain(
                word,
                "opts",
                format!("the key {k:?}, which this word does not take"),
            ));
        }
        match k.as_str() {
            "alternative" => {
                let s = match x.as_str() {
                    Some(s) if x.dt() == STRING => s,
                    _ => return Err(domain(word, k, x.summary(WIDTH))),
                };
                o.alternative = match s.as_str() {
                    "two.sided" => Alternative::TwoSided,
                    "less" => Alternative::Less,
                    "greater" => Alternative::Greater,
                    _ => return Err(domain(word, k, x.summary(WIDTH))),
                };
            }
            "conf" => {
                // A FLOAT: no INTEGER lies in (0, 1).
                o.conf = match x.unboxed() {
                    BundValue::Float(c, _) if x.dt() == FLOAT && *c > 0.0 && *c < 1.0 => *c,
                    _ => return Err(domain(word, k, x.summary(WIDTH))),
                };
            }
            _ => o.mu = real(word, k, x)?,
        }
    }
    Ok(o)
}

/// An interval's end, or E114 when it is not finite.
fn end(word: &str, which: &str, y: f64) -> Result<f64, Error> {
    finite(word, y, || format!("the interval's {which} end"))
}

/// The plain word's keys, with the estimate, the interval's ends that are
/// answered, `conf` and `alternative` beside them.
#[allow(clippy::too_many_arguments)]
fn answer(
    statistic: f64,
    p: f64,
    df: Option<f64>,
    method: &str,
    estimate: f64,
    (lower, upper): (Option<f64>, Option<f64>),
    o: &Opts,
) -> BundValue {
    let mut m: BTreeMap<String, BundValue> = answer_map(statistic, p, df, method);
    m.insert("estimate".to_string(), BundValue::float(estimate));
    if let Some(l) = lower {
        m.insert("lower".to_string(), BundValue::float(l));
    }
    if let Some(u) = upper {
        m.insert("upper".to_string(), BundValue::float(u));
    }
    m.insert("conf".to_string(), BundValue::float(o.conf));
    m.insert(
        "alternative".to_string(),
        BundValue::str(o.alternative.name()),
    );
    BundValue::map(m)
}

// The t tests.

/// A t test's answer for `o`: the statistic of the hypothesis that the
/// estimate is `mu`, its p for the alternative, and the interval about
/// the estimate itself.
fn t_answer(word: &str, c: &TCase, mu: f64, o: &Opts, method: &str) -> Result<BundValue, Error> {
    let t = c.t(word, mu)?;
    let family = Family::T { df: c.df };
    let p = match o.alternative {
        Alternative::TwoSided => t_p(word, t, c.df)?,
        Alternative::Less => family.tails(word, t)?.0,
        Alternative::Greater => family.tails(word, t)?.1,
    };
    let estimate = finite(word, c.estimate, || "the estimate".into())?;
    let alpha = 1.0 - o.conf;
    let ends = match o.alternative {
        Alternative::TwoSided => {
            let q = family.quantile(word, 1.0 - alpha / 2.0)?;
            (
                Some(end(word, "lower", estimate - q * c.se)?),
                Some(end(word, "upper", estimate + q * c.se)?),
            )
        }
        Alternative::Greater => {
            let q = family.quantile(word, 1.0 - alpha)?;
            (Some(end(word, "lower", estimate - q * c.se)?), None)
        }
        Alternative::Less => {
            let q = family.quantile(word, 1.0 - alpha)?;
            (None, Some(end(word, "upper", estimate + q * c.se)?))
        }
    };
    Ok(answer(t, p, Some(c.df), method, estimate, ends, o))
}

/// `stat.test.t1.opts`, from the stack or, its `.` form, the options from
/// the workbench: the series by the house reader, μ₀, the options.
fn t1_opts(vm: &mut dyn Vm, side: Side, word: &str) -> Result<(), Error> {
    let which = |s: Side| match s {
        Side::Stack => "Stack",
        Side::Bench => "Workbench",
    };
    if side.depth(vm) < 1 {
        return Err(Error(format!(
            "{} is too shallow for inline {word}",
            which(side)
        )));
    }
    let Some(v) = side.pull(vm) else {
        return Err(Error::internal(format!(
            "{word}: the options' side was deep enough and gave nothing"
        )));
    };
    let o = read(word, &v, &["alternative", "conf"])?;
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {word}")));
    }
    let Some(mu) = vm.pull() else {
        return Err(Error::internal(format!(
            "{word}: the stack was deep enough and gave nothing"
        )));
    };
    let mu = real(word, "mu", &mu)?;
    let xs = series(vm, Side::Stack, Mode::Consume, word)?;
    let c = t_one_case(word, &xs)?;
    let y = t_answer(word, &c, mu, &o, "One Sample t-test")?;
    side.push(vm, y);
    Ok(())
}

fn stat_test_t1_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    t1_opts(vm, Side::Stack, "stat.test.t1.opts")
}

fn stat_test_t1_opts_bench(vm: &mut dyn Vm) -> Result<(), Error> {
    t1_opts(vm, Side::Bench, "stat.test.t1.opts.")
}

/// A two-sample t test: a and b, each a LIST or a METRICS, and the options.
type TCaseOf = fn(&str, &[f64], &[f64]) -> Result<TCase, Error>;

fn two_t(vm: &mut dyn Vm, word: &str, case: TCaseOf, method: &str) -> Result<(), Error> {
    let v = operands(vm, word, 3)?;
    let o = read(word, &v[2], &KEYS)?;
    let a = whole(word, "a", &v[0])?;
    let b = whole(word, "b", &v[1])?;
    let c = case(word, &a, &b)?;
    let y = t_answer(word, &c, o.mu, &o, method)?;
    vm.push(y);
    Ok(())
}

fn stat_test_t2_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    two_t(vm, "stat.test.t2.opts", t2_case, "Two Sample t-test")
}

fn stat_test_welch_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    two_t(
        vm,
        "stat.test.welch.opts",
        welch_case,
        "Welch Two Sample t-test",
    )
}

fn stat_test_paired_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    two_t(vm, "stat.test.paired.opts", paired_case, "Paired t-test")
}

// The binomial.

/// `stat.test.binom.opts`: k, n, p and the options. The interval is
/// Clopper–Pearson's, two beta quantiles, its end 0 at k = 0 and 1 at
/// k = n by definition, without a quantile at a zero shape.
fn stat_test_binom_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.binom.opts";
    let v = operands(vm, W, 4)?;
    let o = read(W, &v[3], &["alternative", "conf"])?;
    let (k, n, p) = binom_operands(W, &v)?;
    let f = Family::Binomial { size: n, prob: p };
    let pv = match o.alternative {
        Alternative::TwoSided => binom_p(W, &f, k, n, p)?,
        Alternative::Less => f.tails(W, k as f64)?.0,
        Alternative::Greater if k == 0 => 1.0,
        Alternative::Greater => f.tails(W, k as f64 - 1.0)?.1,
    };
    let estimate = finite(W, k as f64 / n as f64, || "the estimate k/n".into())?;
    let (kf, nf) = (k as f64, n as f64);
    let lower = |a: f64| -> Result<f64, Error> {
        if k == 0 {
            Ok(0.0)
        } else {
            Family::Beta {
                a: kf,
                b: nf - kf + 1.0,
            }
            .quantile(W, a)
        }
    };
    let upper = |a: f64| -> Result<f64, Error> {
        if k == n {
            Ok(1.0)
        } else {
            Family::Beta {
                a: kf + 1.0,
                b: nf - kf,
            }
            .quantile(W, 1.0 - a)
        }
    };
    let alpha = 1.0 - o.conf;
    let (lo, hi) = match o.alternative {
        Alternative::TwoSided => (lower(alpha / 2.0)?, upper(alpha / 2.0)?),
        Alternative::Greater => (lower(alpha)?, 1.0),
        Alternative::Less => (0.0, upper(alpha)?),
    };
    let ends = (Some(end(W, "lower", lo)?), Some(end(W, "upper", hi)?));
    vm.push(answer(
        kf,
        pv,
        None,
        "Exact binomial test",
        estimate,
        ends,
        &o,
    ));
    Ok(())
}

/// Register M2's `.opts` words, each plain and `.`, the `.` form taking the
/// options from the workbench (S2, "Forms").
pub(super) fn register(r: &mut bund2_api::Registry) {
    use bund2_api::{StackEffect, WordKind};
    r.register_native(
        "stat.test.t1.opts",
        stat_test_t1_opts,
        StackEffect::opaque(3),
        WordKind::Sync,
    );
    r.register_native(
        "stat.test.t1.opts.",
        stat_test_t1_opts_bench,
        StackEffect::opaque(2),
        WordKind::Sync,
    );
    macro_rules! word {
        ($name:literal, $f:expr, $n:literal) => {
            r.register_native($name, $f, StackEffect::fixed($n, 1), WordKind::Sync);
            crate::wb::bench!(r, $name, $f, $n, 0);
        };
    }
    word!("stat.test.t2.opts", stat_test_t2_opts, 3);
    word!("stat.test.welch.opts", stat_test_welch_opts, 3);
    word!("stat.test.paired.opts", stat_test_paired_opts, 3);
    word!("stat.test.binom.opts", stat_test_binom_opts, 4);
}
