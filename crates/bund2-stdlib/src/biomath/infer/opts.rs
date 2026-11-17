//! RFC-0016 §S2: the `.opts` words, each a test's operands with an options
//! MAP above them, answering the plain word's keys for the chosen
//! alternative and `mu`, and the estimate and interval beside them.
//!
//! M2 builds the t tests' and the binomial's: `stat.test.t1.opts`,
//! `t2.opts`, `welch.opts`, `paired.opts` and `binom.opts`; M3 builds
//! `fisher.opts`, `mannwhitney.opts` and `ks2.opts`.
//!
//! **The plain words are unchanged.** Each `.opts` word computes through
//! the plain word's own code (`TCase`, `binom_p`), so with the defaults,
//! two-sided and `mu` 0, its `statistic`, `p` and `df` are the plain
//! word's, bit for bit (criterion 3). Fisher's, Mann–Whitney's and the KS
//! test's share the plain words' parts (`FisherCase`, `mannwhitney_parts`,
//! `ks2_parts`), and their two-sided answers are the plain words' own.
//!
//! **Intervals** are at level `conf`, two-sided with α = 1 − conf split
//! evenly, one-sided with α on the one end kept: [lower, ∞) for "greater",
//! (−∞, upper] for "less". An infinite end is left out of the answer [A3];
//! Clopper–Pearson's far end is 0 or 1 and is answered.

use std::collections::BTreeMap;

use bund2_api::{Error, Vm};
use bund2_value::{BundValue, FLOAT, MAP, STRING};

use super::{
    Family, FisherCase, REL_ERR, TCase, answer_map, binom_operands, binom_p, fisher_case, ks2,
    ks2_parts, mannwhitney, mannwhitney_parts, paired_case, t_one_case, t_p, t2_case, welch_case,
};
use crate::biomath::operand::{WIDTH, domain, finite, finite_all, operands, real, series, whole};
use crate::biomath::refuse::Refusal;
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
pub(super) fn end(word: &str, which: &str, y: f64) -> Result<f64, Error> {
    finite(word, y, || format!("the interval's {which} end"))
}

/// What an `.opts` word adds to the plain word's keys: the estimate and
/// the interval's ends where they are answered, and `conf` where the word
/// takes it.
pub(super) struct Extra {
    pub(super) estimate: Option<f64>,
    pub(super) ends: (Option<f64>, Option<f64>),
    pub(super) conf: bool,
}

/// The plain word's keys, with the estimate, the interval's ends that are
/// answered, `conf` and `alternative` beside them.
fn answer(
    statistic: f64,
    p: f64,
    df: Option<f64>,
    method: &str,
    extra: Extra,
    o: &Opts,
) -> BundValue {
    let mut m: BTreeMap<String, BundValue> = answer_map(statistic, p, df, method);
    with_extra(&mut m, extra, o);
    BundValue::map(m)
}

pub(super) fn with_extra(m: &mut BTreeMap<String, BundValue>, extra: Extra, o: &Opts) {
    let (lower, upper) = extra.ends;
    if let Some(e) = extra.estimate {
        m.insert("estimate".to_string(), BundValue::float(e));
    }
    if let Some(l) = lower {
        m.insert("lower".to_string(), BundValue::float(l));
    }
    if let Some(u) = upper {
        m.insert("upper".to_string(), BundValue::float(u));
    }
    if extra.conf {
        m.insert("conf".to_string(), BundValue::float(o.conf));
    }
    m.insert(
        "alternative".to_string(),
        BundValue::str(o.alternative.name()),
    );
}

fn full(estimate: Option<f64>, ends: (Option<f64>, Option<f64>)) -> Extra {
    Extra {
        estimate,
        ends,
        conf: true,
    }
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
    Ok(answer(
        t,
        p,
        Some(c.df),
        method,
        full(Some(estimate), ends),
        o,
    ))
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
        full(Some(estimate), ends),
        &o,
    ));
    Ok(())
}

// Fisher's exact test.

/// The most doublings that bracket one of Fisher's roots, and the most
/// bisection steps that refine it (S2, "Intervals").
const BRACKET_CAP: u64 = 64;
const BISECT_CAP: u64 = 200;

impl FisherCase {
    /// Under Fisher's noncentral hypergeometric at odds ratio e^t: the mean
    /// of X, P(X ≥ x) and P(X ≤ x). The masses are taken in log space, the
    /// largest subtracted before exponentiating, so no t overflows.
    fn noncentral(&self, t: f64) -> (f64, f64, f64) {
        let at = |i: usize| self.logs[i] + i as f64 * t;
        let top = (0..self.logs.len())
            .map(at)
            .fold(f64::NEG_INFINITY, f64::max);
        let ix = (self.x - self.lo) as usize;
        let (mut total, mut first, mut ge, mut le) = (0.0, 0.0, 0.0, 0.0);
        for i in 0..self.logs.len() {
            let w = (at(i) - top).exp();
            total += w;
            first += i as f64 * w;
            if i >= ix {
                ge += w;
            }
            if i <= ix {
                le += w;
            }
        }
        (self.lo as f64 + first / total, ge / total, le / total)
    }

    /// The ψ at which `f(ln ψ)` crosses `target`, `f` rising in ln ψ (or
    /// falling, with `rising` false): bracketed by doubling from [−1, 1],
    /// then bisected to 10⁻¹⁰ relative in ψ; either cap is E110.
    fn root(
        &self,
        word: &str,
        f: impl Fn(&Self, f64) -> f64,
        target: f64,
        rising: bool,
    ) -> Result<f64, Error> {
        let h = |t: f64| {
            let d = f(self, t) - target;
            if rising { d } else { -d }
        };
        let (mut lo, mut hi) = (-1.0f64, 1.0f64);
        let mut doublings = 0;
        while h(lo) > 0.0 || h(hi) < 0.0 {
            if doublings == BRACKET_CAP {
                return Err(Refusal::NoConvergence {
                    method: "the odds ratio's bracket".into(),
                    cap: BRACKET_CAP,
                }
                .error(word));
            }
            if h(lo) > 0.0 {
                lo *= 2.0;
            } else {
                hi *= 2.0;
            }
            doublings += 1;
        }
        let mut steps = 0;
        while (hi - lo).exp_m1() > 1e-10 {
            if steps == BISECT_CAP {
                return Err(Refusal::NoConvergence {
                    method: "the odds ratio's bisection".into(),
                    cap: BISECT_CAP,
                }
                .error(word));
            }
            let mid = lo + (hi - lo) / 2.0;
            if h(mid) < 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
            steps += 1;
        }
        Ok((lo + (hi - lo) / 2.0).exp())
    }
}

/// `stat.test.fisher.opts`: a 2×2 table and the options. The estimate is
/// the conditional MLE of the odds ratio, the interval the exact
/// conditional one; at x the support's minimum both the estimate and the
/// lower end are 0, at its maximum the estimate and upper end are infinite
/// and left out [A3].
fn stat_test_fisher_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.fisher.opts";
    let v = operands(vm, W, 2)?;
    let o = read(W, &v[1], &["alternative", "conf"])?;
    let c = fisher_case(W, &v[0])?;
    let (_, ge, le) = c.noncentral(0.0);
    let p = match o.alternative {
        Alternative::TwoSided => c.two_sided_p(),
        Alternative::Less => le,
        Alternative::Greater => ge,
    };
    let (at_lo, at_hi) = (c.x == c.lo, c.x == c.hi);
    let x = c.x as f64;
    let estimate = if at_lo {
        Some(0.0)
    } else if at_hi {
        None
    } else {
        let e = c.root(W, |c, t| c.noncentral(t).0, x, true)?;
        Some(finite(W, e, || "the estimate".into())?)
    };
    let lower = |a: f64| -> Result<f64, Error> {
        if at_lo {
            Ok(0.0)
        } else {
            end(W, "lower", c.root(W, |c, t| c.noncentral(t).1, a, true)?)
        }
    };
    let upper = |a: f64| -> Result<Option<f64>, Error> {
        if at_hi {
            Ok(None)
        } else {
            let u = c.root(W, |c, t| c.noncentral(t).2, a, false)?;
            Ok(Some(end(W, "upper", u)?))
        }
    };
    let alpha = 1.0 - o.conf;
    let ends = match o.alternative {
        Alternative::TwoSided => (Some(lower(alpha / 2.0)?), upper(alpha / 2.0)?),
        Alternative::Greater => (Some(lower(alpha)?), None),
        Alternative::Less => (Some(0.0), upper(alpha)?),
    };
    vm.push(answer(
        x,
        p,
        None,
        "Fisher's Exact Test for Count Data; \
         conditional maximum likelihood estimate of the odds ratio",
        full(estimate, ends),
        &o,
    ));
    Ok(())
}

// The Kolmogorov–Smirnov test.

/// `stat.test.ks2.opts`: two-sided, the plain word's answer; one-sided, D⁺
/// for "greater" and D⁻ for "less", with the asymptotic one-sided tail
/// exp(−2·nm/(n + m)·D²).
fn stat_test_ks2_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.ks2.opts";
    let v = operands(vm, W, 3)?;
    let o = read(W, &v[2], &["alternative"])?;
    let a = whole(W, "a", &v[0])?;
    let b = whole(W, "b", &v[1])?;
    let mut m = match o.alternative {
        Alternative::TwoSided => ks2(W, &a, &b)?.as_map().cloned().ok_or_else(|| {
            Error::internal("stat.test.ks2.opts: the two-sided test answered no MAP")
        })?,
        side => {
            let (_, plus, minus) = ks2_parts(W, &a, &b)?;
            let d = if side == Alternative::Greater {
                plus
            } else {
                minus
            };
            let (n, m) = (a.len() as f64, b.len() as f64);
            let p = (-2.0 * (n * m / (n + m)) * d * d).exp();
            answer_map(d, p, None, "Asymptotic two-sample Kolmogorov-Smirnov test")
        }
    };
    let extra = Extra {
        estimate: None,
        ends: (None, None),
        conf: false,
    };
    with_extra(&mut m, extra, &o);
    vm.push(BundValue::map(m));
    Ok(())
}

// The Mann–Whitney test, with Hodges–Lehmann's estimate [A9].

/// The most differences `mannwhitney.opts` forms (D39).
const DIFFERENCES_CAP: u64 = 10_000_000;

/// C(l + s, s) exactly, or `None` when it is 2¹²⁸ or more. Each step
/// multiplies by l + i and divides by i, the gcd of the running value and
/// i taken out first, so the division is exact and only the answer's own
/// size can overflow.
fn choose(l: u64, s: u64) -> Option<u128> {
    fn gcd(mut a: u128, mut b: u128) -> u128 {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    }
    let mut c: u128 = 1;
    for i in 1..=u128::from(s) {
        let g = gcd(c, i);
        let num = (u128::from(l) + i) / (i / g);
        c = (c / g).checked_mul(num)?;
    }
    Some(c)
}

/// The exact k (S2, "Mann–Whitney"): the largest k in 0 … nm + 1 with
/// P(U ≤ k − 1) ≤ a·(1 + 10⁻⁷), U's distribution counted by the Gaussian
/// binomial's coefficients up to ⌊nm/2⌋ and by symmetry above; `None` when
/// C(n + m, n) is 2¹²⁸ or more.
fn exact_k(n: u64, m: u64, a: f64) -> Result<Option<u64>, Error> {
    let (s, l) = (n.min(m), n.max(m));
    let Some(total) = choose(l, s) else {
        return Ok(None);
    };
    let nm = n * m;
    let half = (nm / 2) as usize;
    let overflow =
        || Error::internal("the Mann–Whitney counts left u128, which C(n + m, n) bounds");
    let mut c = vec![0u128; half + 1];
    c[0] = 1;
    for i in 1..=s as usize {
        // ÷ (1 − q^i): a stride-i running sum, ascending.
        for u in i..=half {
            c[u] = c[u].checked_add(c[u - i]).ok_or_else(overflow)?;
        }
        // × (1 − q^(l+i)): a stride l + i difference, descending.
        let step = l as usize + i;
        for u in (step..=half).rev() {
            c[u] = c[u].checked_sub(c[u - step]).ok_or_else(overflow)?;
        }
    }
    // The lower tail's counts, L(u) = Σ_{v ≤ u} c(v), for u ≤ ⌊nm/2⌋.
    let mut low = c;
    for u in 1..=half {
        low[u] = low[u].checked_add(low[u - 1]).ok_or_else(overflow)?;
    }
    // L(u) for 0 ≤ u ≤ nm, by symmetry above ⌊nm/2⌋: C − L(nm − u − 1),
    // with L(−1) = 0.
    let tail = |u: u64| -> Result<u128, Error> {
        if u as usize <= half {
            Ok(low[u as usize])
        } else if u == nm {
            Ok(total)
        } else {
            let below = low[(nm - u - 1) as usize];
            total.checked_sub(below).ok_or_else(overflow)
        }
    };
    let bound = a * REL_ERR;
    let whole = total as f64;
    // k qualifies when P(U ≤ k − 1) is within the bound; k = 0 always does.
    let mut k = 0u64;
    while k <= nm && tail(k)? as f64 / whole <= bound {
        k += 1;
    }
    Ok(Some(k))
}

/// The normal approximation's k, ⌊nm/2 − z₁₋ₐ·√(nm(n + m + 1)/12)⌋.
fn normal_k(word: &str, n: u64, m: u64, a: f64) -> Result<f64, Error> {
    let z = Family::Normal { mean: 0.0, sd: 1.0 }.quantile(word, 1.0 - a)?;
    let (n, m) = (n as f64, m as f64);
    Ok((n * m / 2.0 - z * (n * m * (n + m + 1.0) / 12.0).sqrt()).floor())
}

/// `stat.test.mannwhitney.opts`: a, b and the options. The statistic and p
/// are of a − μ against b, by the plain word's normal approximation; the
/// estimate and interval are of the differences aᵢ − bⱼ themselves.
fn stat_test_mannwhitney_opts(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "stat.test.mannwhitney.opts";
    let v = operands(vm, W, 3)?;
    let o = read(W, &v[2], &KEYS)?;
    let a = whole(W, "a", &v[0])?;
    let b = whole(W, "b", &v[1])?;
    let (n, m) = (a.len() as u64, b.len() as u64);
    let nm = n.saturating_mul(m);
    if nm > DIFFERENCES_CAP {
        return Err(Refusal::Cap {
            what: format!("{nm} differences of a and b"),
            cap: DIFFERENCES_CAP,
        }
        .error(W));
    }
    let shifted: Vec<f64> = a.iter().map(|x| x - o.mu).collect();
    finite_all(W, "a - mu", &shifted)?;
    // The test, of a − μ against b.
    let (statistic, p) = match o.alternative {
        Alternative::TwoSided => {
            let plain = mannwhitney(W, &shifted, &b)?;
            let get = |k: &str| match plain
                .as_map()
                .and_then(|m| m.get(k))
                .map(BundValue::unboxed)
            {
                Some(BundValue::Float(x, _)) => Ok(*x),
                _ => Err(Error::internal(format!(
                    "stat.test.mannwhitney.opts: the two-sided test answered no {k}"
                ))),
            };
            (get("statistic")?, get("p")?)
        }
        side => {
            let (w, z, sigma) = mannwhitney_parts(W, &shifted, &b)?;
            let normal = Family::Normal { mean: 0.0, sd: 1.0 };
            if side == Alternative::Greater {
                (w, normal.tails(W, (z - 0.5) / sigma)?.1)
            } else {
                (w, normal.tails(W, (z + 0.5) / sigma)?.0)
            }
        }
    };
    // The differences, sorted, and their order statistics, 1-based.
    let mut d = Vec::with_capacity(nm as usize);
    for x in &a {
        for y in &b {
            d.push(x - y);
        }
    }
    d.sort_by(f64::total_cmp);
    let at = |i: u64| -> Result<f64, Error> {
        d.get((i as usize).wrapping_sub(1)).copied().ok_or_else(|| {
            Error::internal(format!(
                "stat.test.mannwhitney.opts: order statistic {i} of {nm}"
            ))
        })
    };
    let estimate = if nm % 2 == 1 {
        at(nm.div_ceil(2))?
    } else {
        let (u, w) = (at(nm / 2)?, at(nm / 2 + 1)?);
        let sum = u + w;
        if sum.is_finite() {
            sum / 2.0
        } else {
            u / 2.0 + w / 2.0
        }
    };
    let estimate = finite(W, estimate, || "the estimate".into())?;
    // k: exact when no value of a or b ties another, before the shift, and
    // C(n + m, n) is below 2¹²⁸; the normal approximation's floor otherwise.
    let alpha = 1.0 - o.conf;
    let a_side = match o.alternative {
        Alternative::TwoSided => alpha / 2.0,
        _ => alpha,
    };
    let mut pooled: Vec<f64> = a.iter().chain(&b).copied().collect();
    pooled.sort_by(f64::total_cmp);
    let ties = pooled.windows(2).any(|w| w[0] == w[1]);
    let exact = if ties { None } else { exact_k(n, m, a_side)? };
    let k = match exact {
        Some(k) => k as f64,
        None => normal_k(W, n, m, a_side)?,
    };
    if !(k >= 1.0 && k <= nm as f64) {
        return Err(domain(W, "a and b", "too few for the level".into()));
    }
    let k = k as u64;
    let lower = || -> Result<f64, Error> { end(W, "lower", at(k)?) };
    let upper = || -> Result<f64, Error> { end(W, "upper", at(nm + 1 - k)?) };
    let ends = match o.alternative {
        Alternative::TwoSided => (Some(lower()?), Some(upper()?)),
        Alternative::Greater => (Some(lower()?), None),
        Alternative::Less => (None, Some(upper()?)),
    };
    vm.push(answer(
        statistic,
        p,
        None,
        "Wilcoxon rank sum test with continuity correction; \
         Hodges–Lehmann estimate, order-statistic interval",
        full(Some(estimate), ends),
        &o,
    ));
    Ok(())
}

/// Register the `.opts` words, each plain and `.`, the `.` form taking the
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
    word!("stat.test.fisher.opts", stat_test_fisher_opts, 2);
    word!("stat.test.mannwhitney.opts", stat_test_mannwhitney_opts, 3);
    word!("stat.test.ks2.opts", stat_test_ks2_opts, 3);
}
