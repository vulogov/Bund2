//! RFC-0018 §S7: `model.compare` and `model.anova`, comparing nested models
//! by their deviances.
//!
//! **The tests** are R's `stat.anova` (`anova.R:34-50`): LR is Δdev/φ on
//! χ²(Δdf), F is (Δdev/Δdf)/φ on Δdf and the larger model's residual df,
//! φ the larger model's dispersion, 1 for the binomial and Poisson. The
//! default is R 4.4's: F when the dispersion is estimated, LR when it is
//! fixed (K13). F under a fixed dispersion, which R computes and warns of
//! (`glm.R:660-661`), is refused. A negative Δdeviance is taken as 0, as
//! `anova.glm` takes it (`glm.R:520-521`), and answers statistic 0 and p 1.
//!
//! **`model.anova`** fits the full model first, so its refusals are the
//! fit word's own, then each sequential submodel by `assign`, a refusal
//! there passed on with its code and `, fitting the model through term
//! "x"` added. Under `gaussian` and `identity` the models are `fit.lm`'s
//! least squares, the constant refusal not applied; otherwise `fit.glm`'s
//! IRLS. The first row is the null model, its deviance the full fit's
//! null deviance with an intercept, and at linkinv(offset) without one.

use std::collections::BTreeMap;

use bund2_api::{Error, Vm};
use bund2_value::BundValue;

use super::code::{members_of, variable_order};
use super::design::{self, Design};
use super::glm::{self, Family, Link};
use super::lm;
use super::opts::{self, Opts};
use super::reading::{self, Fit};
use crate::biomath::dist::Family as Dist;
use crate::biomath::frame::check::Frame;
use crate::biomath::frame::render;
use crate::biomath::na::none;
use crate::biomath::operand::{WIDTH, domain, operands};
use crate::biomath::refuse::Refusal;

/// The test a comparison makes (§S7).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Test {
    F,
    Lr,
}

impl Test {
    fn name(self) -> &'static str {
        match self {
            Test::F => "F",
            Test::Lr => "LR",
        }
    }
}

/// The test `opts` asks for, or R 4.4's default for the larger model's
/// family; F under a fixed dispersion refused.
fn test_of(word: &str, o: &Opts, family: Family) -> Result<Test, Error> {
    let t = match o.test.as_deref() {
        None if family.estimated() => Test::F,
        None => Test::Lr,
        Some("F") => Test::F,
        Some("LR") => Test::Lr,
        Some(other) => {
            return Err(opts::refuse(
                word,
                format!("test is {}", render::text(other)),
            ));
        }
    };
    if t == Test::F && !family.estimated() {
        return Err(opts::refuse(
            word,
            format!(
                "test \"F\" needs an estimated dispersion, and the {} family's is 1",
                family.name()
            ),
        ));
    }
    Ok(t)
}

/// One comparison's answer.
struct Tested {
    deviance: f64,
    statistic: f64,
    p: f64,
}

/// `stat.anova`'s statistic and p for a Δdeviance on `df`, φ the larger
/// model's dispersion and `df2` its residual df.
fn tested(
    word: &str,
    test: Test,
    delta: f64,
    df: i64,
    phi: f64,
    df2: i64,
) -> Result<Tested, Error> {
    if delta <= 0.0 {
        return Ok(Tested {
            deviance: 0.0,
            statistic: 0.0,
            p: 1.0,
        });
    }
    let statistic = match test {
        Test::F => (delta / df as f64) / phi,
        Test::Lr => delta / phi,
    };
    if !statistic.is_finite() {
        return Err(Refusal::Overflow {
            expression: "the test statistic".into(),
        }
        .error(word));
    }
    let p = match test {
        Test::F => {
            Dist::F {
                df1: df as f64,
                df2: df2 as f64,
            }
            .tails(word, statistic)?
            .1
        }
        Test::Lr => Dist::ChiSq { df: df as f64 }.tails(word, statistic)?.1,
    };
    Ok(Tested {
        deviance: delta,
        statistic,
        p,
    })
}

fn shown(x: f64) -> String {
    BundValue::float(x).summary(WIDTH)
}

/// The first element where two per-row LISTs differ, for nesting.
fn same_rows(word: &str, key: &str, small: &[f64], big: &[f64]) -> Result<(), Error> {
    if small.len() != big.len() {
        return Err(domain(
            word,
            "small",
            format!(
                "{key} has {} items, where big's has {}",
                small.len(),
                big.len()
            ),
        ));
    }
    if let Some(i) = small.iter().zip(big).position(|(a, b)| a != b) {
        return Err(domain(
            word,
            "small",
            format!(
                "{key} row {} is {}, where big's is {}",
                i + 1,
                shown(small[i]),
                shown(big[i])
            ),
        ));
    }
    Ok(())
}

/// §S7's nesting, E101 on `small` naming the first disagreement: family,
/// link, `y`, `weights`, `offset`, every name among `big`'s, fewer
/// columns, and so a larger residual df.
fn nested(word: &str, small: &Fit, big: &Fit) -> Result<(), Error> {
    let no = |t: String| domain(word, "small", t);
    if small.family != big.family {
        return Err(no(format!(
            "family is {}, where big's is {}",
            render::text(small.family.name()),
            render::text(big.family.name())
        )));
    }
    if small.link != big.link {
        return Err(no(format!(
            "link is {}, where big's is {}",
            render::text(small.link.name()),
            render::text(big.link.name())
        )));
    }
    same_rows(word, "y", &small.y, &big.y)?;
    same_rows(word, "weights", &small.weights, &big.weights)?;
    same_rows(word, "offset", &small.offset, &big.offset)?;
    if let Some(i) = small.names.iter().position(|n| !big.names.contains(n)) {
        return Err(no(format!(
            "names item {}, {}, is not among big's names",
            i + 1,
            render::text(&small.names[i])
        )));
    }
    if small.names.len() >= big.names.len() {
        return Err(no(format!(
            "has {} columns, not fewer than big's {}",
            small.names.len(),
            big.names.len()
        )));
    }
    if small.df_residual <= big.df_residual {
        return Err(no(format!(
            "df.residual is {}, not above big's {}",
            small.df_residual, big.df_residual
        )));
    }
    Ok(())
}

/// `model.compare` (`small big opts -- map`).
pub(crate) fn model_compare(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "model.compare";
    let v = operands(vm, W, 3)?;
    let [small_v, big_v, opts_v] = <[BundValue; 3]>::try_from(v).map_err(|v| {
        Error::internal(format!(
            "{W}: three operands were pulled and {} are held",
            v.len()
        ))
    })?;
    let small = reading::check(W, "small", &small_v)?;
    let big = reading::check(W, "big", &big_v)?;
    let o = opts::read(W, &opts_v, &["test"])?;
    let test = test_of(W, &o, big.family)?;
    nested(W, &small, &big)?;
    let phi = if big.family.estimated() {
        big.dispersion
    } else {
        1.0
    };
    if phi <= 0.0 {
        return Err(domain(
            W,
            "big",
            format!("dispersion is {}, not above 0", shown(phi)),
        ));
    }
    let df = small.df_residual - big.df_residual;
    let t = tested(
        W,
        test,
        small.deviance - big.deviance,
        df,
        phi,
        big.df_residual,
    )?;
    let mut m = BTreeMap::new();
    m.insert("df".to_string(), BundValue::int(df));
    m.insert("deviance".to_string(), BundValue::float(t.deviance));
    m.insert("statistic".to_string(), BundValue::float(t.statistic));
    m.insert("p".to_string(), BundValue::float(t.p));
    m.insert("test".to_string(), BundValue::str(test.name()));
    if test == Test::F {
        m.insert("df2".to_string(), BundValue::int(big.df_residual));
    }
    vm.push(BundValue::map(m));
    Ok(())
}

/// The design of the columns whose term is at most `through`.
fn through(d: &Design, through: usize) -> Design {
    let keep: Vec<usize> = (0..d.assign.len())
        .filter(|&j| d.assign[j] <= through)
        .collect();
    Design {
        x: d.x
            .iter()
            .map(|r| keep.iter().filter_map(|&j| r.get(j).copied()).collect())
            .collect(),
        names: keep
            .iter()
            .filter_map(|&j| d.names.get(j).cloned())
            .collect(),
        assign: keep
            .iter()
            .filter_map(|&j| d.assign.get(j).copied())
            .collect(),
        ..d.clone()
    }
}

/// Each term's name, an interaction's members in R's variable order joined
/// by `:` (D265).
fn labels(word: &str, d: &Design) -> Result<Vec<String>, Error> {
    let items = d.spec.terms.as_list().unwrap_or_default();
    let terms = items
        .iter()
        .map(|t| {
            members_of(t).ok_or_else(|| {
                Error::internal(format!(
                    "{word}: a checked design's term is {}",
                    t.summary(WIDTH)
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let order = variable_order(&terms);
    Ok(terms
        .into_iter()
        .map(|mut t| {
            t.sort_by_key(|m| order.iter().position(|o| o == m));
            t.join(":")
        })
        .collect())
}

/// A count as an INTEGER.
fn count(x: usize) -> i64 {
    i64::try_from(x).unwrap_or(i64::MAX)
}

/// A refusal while fitting a row, its text naming the row.
fn at_row(e: Error, row: &str) -> Error {
    if e.is_internal() {
        e
    } else {
        Error(format!("{}, {row}", e.0))
    }
}

/// `model.anova` (`design opts -- frame`).
pub(crate) fn model_anova(vm: &mut dyn Vm) -> Result<(), Error> {
    const W: &str = "model.anova";
    let v = operands(vm, W, 2)?;
    let [design_v, opts_v] = <[BundValue; 2]>::try_from(v).map_err(|v| {
        Error::internal(format!(
            "{W}: two operands were pulled and {} are held",
            v.len()
        ))
    })?;
    let d = design::check(W, "design", &design_v)?;
    let o = opts::read(W, &opts_v, &["family", "link", "test"])?;
    let (family, link) = glm::family_and_link(W, &o)?;
    let test = test_of(W, &o, family)?;
    let gaussian = family == Family::Gaussian && link == Link::Identity;
    let n = d.y.len();
    let k = d.names.len();
    let df_full = count(n - k);
    let intercept = d.spec.intercept;

    // The full model first: its refusals are the fit word's own.
    let (full, phi, null_with_intercept) = if gaussian {
        let (rss, tss) = lm::sums(W, &d)?;
        (rss, rss / df_full as f64, Some(tss))
    } else {
        let spec = design_v
            .as_map()
            .and_then(|m| m.get("spec").cloned())
            .ok_or_else(|| Error::internal(format!("{W}: the checked design has no spec")))?;
        let fit = glm::fit(W, &d, family, link, spec)?;
        let float = |key: &str| {
            fit.as_map()
                .and_then(|m| m.get(key).cloned())
                .and_then(|x| match x.unboxed() {
                    BundValue::Float(f, _) => Some(*f),
                    _ => None,
                })
        };
        let dev = float("deviance")
            .ok_or_else(|| Error::internal(format!("{W}: fit.glm's fit has no deviance")))?;
        let phi = if family.estimated() {
            float("dispersion")
                .ok_or_else(|| Error::internal(format!("{W}: fit.glm's fit has no dispersion")))?
        } else {
            1.0
        };
        (dev, phi, float("null.deviance"))
    };

    // The null model.
    let (null_term, null_dev, null_df) = if intercept {
        let dev = null_with_intercept
            .filter(|x| x.is_finite())
            .ok_or_else(|| {
                Refusal::Overflow {
                    expression: "the null model's deviance".into(),
                }
                .error(W)
            })?;
        ("(intercept)", dev, count(n - 1))
    } else {
        let dev = glm::deviance_at_offset(&d, family, link);
        if !dev.is_finite() {
            return Err(Refusal::Overflow {
                expression: "the null model's deviance at linkinv(offset)".into(),
            }
            .error(W));
        }
        ("NULL", dev, count(n))
    };

    let labels = labels(W, &d)?;
    let nterms = d.spec.nterms;
    let mut resid: Vec<(f64, i64)> = vec![(null_dev, null_df)];
    for j in 1..=nterms {
        if j == nterms {
            resid.push((full, df_full));
            continue;
        }
        let sub = through(&d, j);
        let row = format!(
            "fitting the model through term {}",
            render::text(labels.get(j - 1).map_or("", String::as_str))
        );
        let dev = if gaussian {
            lm::sums(W, &sub).map(|(rss, _)| rss)
        } else {
            glm::deviance(W, &sub, family, link)
        }
        .map_err(|e| at_row(e, &row))?;
        resid.push((dev, count(n - sub.names.len())));
    }

    let rows = resid.len();
    let mut term = Vec::with_capacity(rows);
    let mut df = Vec::with_capacity(rows);
    let mut deviance = Vec::with_capacity(rows);
    let mut resid_df = Vec::with_capacity(rows);
    let mut resid_dev = Vec::with_capacity(rows);
    let mut statistic = Vec::with_capacity(rows);
    let mut p = Vec::with_capacity(rows);
    term.push(BundValue::str(null_term));
    df.push(none());
    deviance.push(none());
    statistic.push(none());
    p.push(none());
    for (r, (dev, rdf)) in resid.iter().enumerate() {
        resid_df.push(BundValue::int(*rdf));
        resid_dev.push(BundValue::float(*dev));
        if r == 0 {
            continue;
        }
        let (prev_dev, prev_df) = resid[r - 1];
        let ddf = prev_df - rdf;
        let t = tested(W, test, prev_dev - dev, ddf, phi, df_full)?;
        term.push(BundValue::str(
            labels.get(r - 1).cloned().unwrap_or_default(),
        ));
        df.push(BundValue::int(ddf));
        deviance.push(BundValue::float(t.deviance));
        statistic.push(BundValue::float(t.statistic));
        p.push(BundValue::float(t.p));
    }
    let names = [
        "term",
        "df",
        "deviance",
        "resid.df",
        "resid.deviance",
        "statistic",
        "p",
    ];
    let frame = Frame {
        names: names.iter().map(|s| s.to_string()).collect(),
        cols: vec![term, df, deviance, resid_df, resid_dev, statistic, p],
        levels: BTreeMap::new(),
        nrow: rows,
    };
    vm.push(frame.to_value());
    Ok(())
}
