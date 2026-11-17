//! RFC-0016 criteria 1, 2 and 9, for §S5's effect sizes. The fixture is
//! `infer16.py`'s, at 50 digits, J by mpmath's gamma; no R output is
//! committed for these, so it alone is the check (A7).

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::{WORDS, hedges_j};
use crate::biomath::testkit::{close, err, fixture, floats, run, top};

const FIXTURE: &str = include_str!("../fixtures/infer16.txt");

/// R's `sleep` data, `extra` by group.
const A: &str = "[ 0.7 -1.6 -0.2 -1.2 -0.1 3.4 3.7 0.8 0.0 2.0 ]";
const B: &str = "[ 1.9 0.8 1.1 0.1 -0.1 4.4 5.5 1.6 4.6 3.4 ]";

const TABLE: &str = "[ [ 12 5 ] [ 7 21 ] ] matrix";

fn map(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap();
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn one(m: &BTreeMap<String, BundValue>, k: &str) -> f64 {
    floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")))[0]
}

fn want(key: &str) -> f64 {
    fixture(FIXTURE, key)[0]
}

/// The fixture's long series: aᵢ = i mod 3, bᵢ = (i mod 4)/2 + 5/4.
fn series(n: usize, b: bool) -> String {
    let xs: Vec<String> = (0..n)
        .map(|i| {
            if b {
                format!("{:?}", (i % 4) as f64 / 2.0 + 1.25)
            } else {
                format!("{:?}", (i % 3) as f64)
            }
        })
        .collect();
    format!("[ {} ]", xs.join(" "))
}

#[test]
fn every_word_is_registered_with_its_effect() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        let want = match (w.starts_with("stat.effect.cohend"), w.ends_with('.')) {
            (true, false) => (2, 1),
            (true, true) | (false, false) => (1, if w.ends_with('.') { 0 } else { 1 }),
            (false, true) => (0, 0),
        };
        assert_eq!(
            (e.consumes, e.produces, e.opaque),
            (want.0, want.1, false),
            "{w}"
        );
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("stat.effect."))
        .count();
    assert_eq!(ours, WORDS.len());
}

/// Criterion 9: d and g on `sleep`, and g either side of the switch to the
/// series (df 200 and 201), at df 398, where Γ alone overflows, and at
/// df 399 998.
#[test]
fn cohend_against_the_fixture() {
    let m = map(&format!("{A} {B} stat.effect.cohend"));
    close("d", &[one(&m, "d")], &[want("cohend.sleep d")], 1e-12);
    close("g", &[one(&m, "g")], &[want("cohend.sleep g")], 1e-12);
    for (na, nb) in [(101, 101), (101, 102), (200, 200), (200_000, 200_000)] {
        let src = format!(
            "{} {} stat.effect.cohend",
            series(na, false),
            series(nb, true)
        );
        let m = map(&src);
        let label = format!("cohend.{na}.{nb}");
        close(
            &label,
            &[one(&m, "d")],
            &[want(&format!("{label} d"))],
            1e-12,
        );
        let g = one(&m, "g");
        assert!(g.is_finite(), "{label}");
        close(&label, &[g], &[want(&format!("{label} g"))], 1e-12);
    }
    // The form takes b from the workbench.
    let mut i = run(&format!("{A} {B} . stat.effect.cohend.")).unwrap();
    assert_eq!(i.pull_workbench().unwrap().as_map().unwrap(), &m_sleep());
}

fn m_sleep() -> BTreeMap<String, BundValue> {
    map(&format!("{A} {B} stat.effect.cohend"))
}

/// J's two branches meet at the switch: the series at df 201 continues the
/// lgamma difference at 200 smoothly, and both are below 1.
#[test]
fn hedges_j_is_continuous_at_the_switch() {
    let (j200, j201) = (hedges_j(200.0), hedges_j(201.0));
    assert!(j200 < j201 && j201 < 1.0, "{j200} {j201}");
    // J = 1 − 3/(4 df − 1) to O(df⁻²).
    for df in [200.0f64, 201.0, 398.0, 399_998.0] {
        let approx = 1.0 - 3.0 / (4.0 * df - 1.0);
        assert!((hedges_j(df) - approx).abs() < 1.0 / (df * df), "{df}");
    }
}

#[test]
fn the_ratios_against_the_fixture() {
    for (w, label, method) in [
        (
            "oddsratio",
            "oddsratio",
            "Sample odds ratio; Woolf's interval",
        ),
        ("riskratio", "riskratio", "Risk ratio; Katz's interval"),
    ] {
        let m = map(&format!("{TABLE} stat.effect.{w}"));
        for k in ["estimate", "log", "se"] {
            close(w, &[one(&m, k)], &[want(&format!("{label} {k}"))], 1e-12);
        }
        for k in ["lower", "upper"] {
            close(w, &[one(&m, k)], &[want(&format!("{label} {k}"))], 1e-9);
        }
        assert_eq!(one(&m, "conf"), 0.95);
        assert_eq!(
            m.get("method").and_then(BundValue::as_str).as_deref(),
            Some(method)
        );
        let mut i = run(&format!("{TABLE} . stat.effect.{w}.")).unwrap();
        assert_eq!(i.pull_workbench().unwrap().as_map().unwrap(), &m, "{w}");
    }
}

/// `oddsratio` refuses a zero in each cell in turn; `riskratio` refuses
/// a = 0 and c = 0 and answers b = 0 and d = 0.
#[test]
fn the_ratios_zero_cells() {
    let tables = [
        ("[ [ 0 5 ] [ 7 21 ] ]", "[1,1]"),
        ("[ [ 12 0 ] [ 7 21 ] ]", "[1,2]"),
        ("[ [ 12 5 ] [ 0 21 ] ]", "[2,1]"),
        ("[ [ 12 5 ] [ 7 0 ] ]", "[2,2]"),
    ];
    for (t, at) in tables {
        assert_eq!(
            err(&format!("{t} matrix stat.effect.oddsratio")),
            format!(
                "STAT.EFFECT.ODDSRATIO: E101 table is outside stat.effect.oddsratio's \
                 domain: 0 at {at}"
            )
        );
    }
    for (t, at) in [tables[0], tables[2]] {
        assert_eq!(
            err(&format!("{t} matrix stat.effect.riskratio")),
            format!(
                "STAT.EFFECT.RISKRATIO: E101 table is outside stat.effect.riskratio's \
                 domain: 0 at {at}"
            )
        );
    }
    let m = map("[ [ 12 0 ] [ 7 21 ] ] matrix stat.effect.riskratio");
    close(
        "b = 0",
        &[one(&m, "estimate")],
        &[1.0 / (7.0 / 28.0)],
        1e-15,
    );
    assert!(one(&m, "lower") < one(&m, "estimate"));
    let m = map("[ [ 12 0 ] [ 7 0 ] ] matrix stat.effect.riskratio");
    assert_eq!(one(&m, "se"), 0.0);
    assert_eq!(
        (one(&m, "lower"), one(&m, "upper")),
        (one(&m, "estimate"), one(&m, "estimate"))
    );
    assert!(
        err("[ [ 1 2 3 ] [ 4 5 6 ] ] matrix stat.effect.oddsratio")
            .starts_with("STAT.EFFECT.ODDSRATIO: E102 ")
    );
    assert!(
        err("[ [ 1 -2 ] [ 4 5 ] ] matrix stat.effect.riskratio")
            .starts_with("STAT.EFFECT.RISKRATIO: E101 table ")
    );
}

#[test]
fn cramerv_against_the_fixture() {
    let t = "[ [ 10 0 5 ] [ 3 8 2 ] [ 4 6 9 ] ] matrix";
    close(
        "V",
        &top(&format!("{t} stat.effect.cramerv")),
        &[want("cramerv V")],
        1e-12,
    );
    let mut i = run(&format!("{t} . stat.effect.cramerv.")).unwrap();
    assert_eq!(i.depth(), 0);
    assert_eq!(
        floats(&i.pull_workbench().unwrap()),
        top(&format!("{t} stat.effect.cramerv"))
    );
    // `stat.test.chisq.table`'s refusals.
    assert_eq!(
        err("[ [ 1 2 3 ] ] matrix stat.effect.cramerv"),
        "STAT.EFFECT.CRAMERV: E101 table is outside stat.effect.cramerv's domain: \
         a table of 1 by 3"
    );
    assert!(err("[ [ 0 0 ] [ 1 2 ] ] matrix stat.effect.cramerv").ends_with("row 1 sums to 0"));
}

#[test]
fn cohend_refusals() {
    assert_eq!(
        err("[ 1.0 ] [ 2.0 3.0 ] stat.effect.cohend"),
        "STAT.EFFECT.COHEND: E101 series is outside stat.effect.cohend's domain: \
         a series of 1 value"
    );
    assert_eq!(
        err("[ 1.0 1.0 ] [ 2.0 2.0 ] stat.effect.cohend"),
        "STAT.EFFECT.COHEND: E101 a and b is outside stat.effect.cohend's domain: \
         a constant series"
    );
}

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "0.5",
        "\"x\"",
        "list",
        "[ 1 ]",
        "[ 1 1 1 ]",
        "[ 1 2 4 ]",
        "[ 1.0e308 -1.0e308 0.0 ]",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
        "[ [ 0 0 ] [ 0 5 ] ] matrix",
        "[ [ 1.0e308 1 ] [ 1 1.0e308 ] ] matrix",
        "[ [ 1 2 ] [ 3 ] ] matrix",
        "dict",
    ];
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        for o in operands {
            for p in operands {
                for src in [
                    format!("{o} {w}"),
                    format!("{o} {p} {w}"),
                    format!("{o} {p} . {w}."),
                ] {
                    if let Err(e) = run(&src) {
                        assert!(
                            e.contains(": E1")
                                || e.contains("too shallow")
                                || e.contains("returned"),
                            "{src}: {e}"
                        );
                    }
                }
            }
        }
    }
}
