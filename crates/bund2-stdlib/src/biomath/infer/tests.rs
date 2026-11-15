//! RFC-0015 criteria 2, 3, 5, 6 and 9, for §S6's tests, corrections and
//! `stat.lm`.

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{close, err, fixture, floats, run, top};

const FIXTURE: &str = include_str!("../fixtures/infer.txt");

/// R's `sleep` data, `extra` by group.
const A: &str = "[ 0.7 -1.6 -0.2 -1.2 -0.1 3.4 3.7 0.8 0.0 2.0 ]";
const B: &str = "[ 1.9 0.8 1.1 0.1 -0.1 4.4 5.5 1.6 4.6 3.4 ]";

const LM: &str = "[ [ 1 2.5 ] [ 2 1.0 ] [ 3 4.0 ] [ 4 3.5 ] [ 5 6.0 ] [ 6 5.0 ] [ 7 8.5 ] [ 8 7.0 ] ] \
                  matrix [ 3.1 4.9 7.2 8.8 11.1 13.2 14.8 17.1 ]";

fn programs() -> Vec<(&'static str, String)> {
    vec![
        ("t1", format!("{A} 0.5 stat.test.t1")),
        ("t2", format!("{A} {B} stat.test.t2")),
        ("welch", format!("{A} {B} stat.test.welch")),
        ("paired", format!("{A} {B} stat.test.paired")),
        (
            "chisq",
            "[ 89 37 30 28 2 ] [ 0.40 0.20 0.20 0.19 0.01 ] stat.test.chisq".into(),
        ),
        (
            "chisq.counts",
            "[ 89 37 30 28 2 ] [ 74.8 37.4 37.4 35.53 1.87 ] stat.test.chisq".into(),
        ),
        (
            "chisq.table",
            "[ [ 762 327 468 ] [ 484 239 477 ] ] matrix stat.test.chisq.table".into(),
        ),
        (
            "fisher.tea",
            "[ [ 3 1 ] [ 1 3 ] ] matrix stat.test.fisher".into(),
        ),
        (
            "fisher.big",
            "[ [ 12 5 ] [ 7 21 ] ] matrix stat.test.fisher".into(),
        ),
        ("binom", "682 925 0.75 stat.test.binom".into()),
        ("binom.low", "3 20 0.5 stat.test.binom".into()),
        ("mannwhitney", format!("{A} {B} stat.test.mannwhitney")),
        ("ks2", format!("{A} {B} stat.test.ks2")),
    ]
}

/// The MAP `src` leaves on top.
fn map(src: &str) -> std::collections::BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap();
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn field(m: &std::collections::BTreeMap<String, BundValue>, k: &str) -> Vec<f64> {
    floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")))
}

// Criterion 2.

#[test]
fn every_word_is_registered() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| {
            n.starts_with("stat.test.")
                || n.starts_with("stat.padjust.")
                || n.starts_with("stat.lm")
        })
        .count();
    assert_eq!(ours, WORDS.len());
}

// Criterion 9.

/// Each test's statistic, p and df against the fixture: a statistic to 1e-12,
/// a closed form; p and Welch's df to 1e-8, since a p is a series or
/// continued fraction (B11).
#[test]
fn each_test_against_the_fixture() {
    for (label, src) in programs() {
        let m = map(&src);
        for (k, tol) in [("statistic", 1e-12), ("p", 1e-8), ("df", 1e-8)] {
            let line = format!("{label} {k} ");
            if FIXTURE.lines().any(|l| l.starts_with(&line)) {
                let want = fixture(FIXTURE, &format!("{label} {k}"));
                close(&format!("{label} {k}"), &field(&m, k), &want, tol);
            }
        }
        assert!(m.contains_key("method"), "{label}");
    }
}

/// R's printed answers for the same data, from R's help pages and the
/// `sleep` data: what a reader of `?t.test`, `?chisq.test`, `?fisher.test`
/// and `?binom.test` would see, to the digits R prints. R is not installed
/// where this was written; these agree with the fixture's to every printed
/// digit, which is the check.
#[test]
fn rs_printed_answers() {
    let near = |label: &str, k: &str, want: f64, digits: f64| {
        let src = programs().into_iter().find(|(l, _)| *l == label).unwrap().1;
        let got = field(&map(&src), k)[0];
        assert!(
            (got - want).abs() <= 0.5 * 10f64.powf(-digits),
            "{label} {k}: {got} against R's {want}"
        );
    };
    // t.test(extra ~ group, data = sleep): t = -1.8608, df = 17.776, p = 0.07939.
    near("welch", "statistic", -1.8608, 4.0);
    near("welch", "df", 17.776, 3.0);
    near("welch", "p", 0.07939, 5.0);
    // var.equal = TRUE: p = 0.07919.
    near("t2", "p", 0.07919, 5.0);
    // paired = TRUE: t = -4.0621, df = 9, p = 0.002833.
    near("paired", "statistic", -4.0621, 4.0);
    near("paired", "p", 0.002833, 6.0);
    // chisq.test(x, p = p): X-squared = 5.7947, df = 4, p = 0.215.
    near("chisq", "statistic", 5.7947, 4.0);
    near("chisq", "p", 0.215, 3.0);
    // chisq.test(M): X-squared = 30.07, df = 2, p = 2.954e-07.
    near("chisq.table", "statistic", 30.07, 2.0);
    near("chisq.table", "p", 2.954e-7, 10.0);
    // fisher.test(TeaTasting): p = 0.4857.
    near("fisher.tea", "p", 0.4857, 4.0);
    // binom.test(682, 925, p = 0.75): p = 0.3825.
    near("binom", "p", 0.3825, 4.0);
    // wilcox.test(extra ~ group, data = sleep): W = 25.5, p = 0.06933.
    near("mannwhitney", "statistic", 25.5, 1.0);
    near("mannwhitney", "p", 0.06933, 5.0);
}

#[test]
fn the_corrections_against_the_fixture() {
    let ps = "[ 0.01 0.02 0.03 0.04 0.05 0.5 0.001 0.04 ]";
    for w in ["bonferroni", "holm", "bh"] {
        let got = top(&format!("{ps} stat.padjust.{w}"));
        close(w, &got, &fixture(FIXTURE, &format!("padjust {w}")), 1e-15);
    }
    assert_eq!(top("list stat.padjust.bh"), Vec::<f64>::new());
}

#[test]
fn lm_against_the_fixture() {
    let m = map(&format!("{LM} stat.lm"));
    for k in ["coef", "se", "t", "p"] {
        close(
            k,
            &field(&m, k),
            &fixture(FIXTURE, &format!("lm {k}")),
            1e-8,
        );
    }
    close("r2", &field(&m, "r2"), &fixture(FIXTURE, "lm r2"), 1e-12);
    assert_eq!(field(&m, "df"), [5.0]);
}

/// All four forms of a correction, and both of every other word.
#[test]
fn the_forms() {
    let ps = "[ 0.01 0.04 0.03 ]";
    let plain = top(&format!("{ps} stat.padjust.holm"));
    // `,` keeps the series.
    let mut i = run(&format!("{ps} stat.padjust.holm,")).unwrap();
    assert_eq!(floats(&i.pull().unwrap()), plain);
    assert_eq!(floats(&i.pull().unwrap()), [0.01, 0.04, 0.03]);
    // `.` reads the workbench and answers there; `.,` keeps it.
    let mut i = run(&format!("{ps} . stat.padjust.holm.")).unwrap();
    assert_eq!(floats(&i.pull_workbench().unwrap()), plain);
    let mut i = run(&format!("{ps} . stat.padjust.holm.,")).unwrap();
    assert_eq!(floats(&i.pull_workbench().unwrap()), plain);
    assert_eq!(floats(&i.pull_workbench().unwrap()), [0.01, 0.04, 0.03]);
    // The rest take the plain word's top operand from the workbench.
    for (below, top_, w) in [
        (A.to_string(), "0.5".to_string(), "stat.test.t1"),
        (A.to_string(), B.to_string(), "stat.test.welch"),
        ("682 925".to_string(), "0.75".to_string(), "stat.test.binom"),
        (
            String::new(),
            "[ [ 3 1 ] [ 1 3 ] ] matrix".to_string(),
            "stat.test.fisher",
        ),
        (
            "[ [ 1 ] [ 2 ] [ 3 ] [ 4 ] ] matrix".to_string(),
            "[ 1.0 2.5 2.5 4.5 ]".to_string(),
            "stat.lm",
        ),
    ] {
        let plain = run(&format!("{below} {top_} {w}")).unwrap().pull().unwrap();
        let mut i = run(&format!("7 {below} {top_} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        assert_eq!(
            i.pull_workbench().unwrap().display(),
            plain.display(),
            "{w}."
        );
    }
}

// Criterion 3.

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            "[ 1 1 1 ] 0 stat.test.t1",
            "STAT.TEST.T1: E101 series is outside stat.test.t1's domain: a constant series",
        ),
        (
            "[ 1 ] [ 2 3 ] stat.test.t2",
            "STAT.TEST.T2: E101 series is outside stat.test.t2's domain: a series of 1 value",
        ),
        (
            "[ 1 2 3 ] [ 1 2 ] stat.test.paired",
            "STAT.TEST.PAIRED: E102 lengths do not agree: 3 and 2",
        ),
        (
            "[ 1 2 ] 3 stat.test.welch",
            "STAT.TEST.WELCH: E115 b must be a LIST or METRICS; a drained series is not allowed here",
        ),
        (
            "[ 5 -1 ] [ 0.5 0.5 ] stat.test.chisq",
            "STAT.TEST.CHISQ: E101 observed is outside stat.test.chisq's domain: -1 at 2",
        ),
        (
            "[ 5 1 ] [ 1.0 0.0 ] stat.test.chisq",
            "STAT.TEST.CHISQ: E101 expected is outside stat.test.chisq's domain: 0 at 2",
        ),
        (
            "[ [ 1 2 ] [ 0 0 ] ] matrix stat.test.chisq.table",
            "STAT.TEST.CHISQ.TABLE: E101 table is outside stat.test.chisq.table's domain: \
             row 2 sums to 0",
        ),
        (
            "[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ] matrix stat.test.fisher",
            "STAT.TEST.FISHER: E102 rows do not agree: 3 and 2",
        ),
        (
            "[ [ 1 2.5 ] [ 3 4 ] ] matrix stat.test.fisher",
            "STAT.TEST.FISHER: E101 table is outside stat.test.fisher's domain: 2.5 at [1,2]",
        ),
        (
            "[ [ 1 2 ] [ 3 ] ] matrix stat.test.chisq.table",
            "STAT.TEST.CHISQ.TABLE: E103 table is not a MATRIX of numbers: row 2 has 1 cells, row 1 2",
        ),
        (
            "[ 1 2 ] stat.test.fisher",
            "STAT.TEST.FISHER: E101 table is outside stat.test.fisher's domain: list/2 [1, 2]",
        ),
        (
            "5 3 0.5 stat.test.binom",
            "STAT.TEST.BINOM: E101 k is outside stat.test.binom's domain: 5",
        ),
        (
            "1 3 1.5 stat.test.binom",
            "STAT.TEST.BINOM: E101 p is outside stat.test.binom's domain: 1.5",
        ),
        (
            "[ 0.5 1.5 ] stat.padjust.bh",
            "STAT.PADJUST.BH: E101 series is outside stat.padjust.bh's domain: 1.5 at 2",
        ),
        (
            "[ [ 1 2 ] [ 2 4 ] [ 3 6 ] [ 4 8 ] ] matrix [ 1.0 2.0 3.5 3.0 ] stat.lm",
            "STAT.LM: E106 the matrix is singular",
        ),
        (
            "[ [ 1 ] [ 2 ] ] matrix [ 1.0 2.0 ] stat.lm",
            "STAT.LM: E101 y is outside stat.lm's domain: 2 observations for 2 coefficients",
        ),
        (
            "[ [ 1 ] [ 2 ] [ 3 ] ] matrix [ 1.0 2.0 ] stat.lm",
            "STAT.LM: E102 lengths do not agree: 3 and 2",
        ),
    ] {
        assert_eq!(err(src), want, "{src}");
    }
}

// Criterion 5: no abort, whatever the operand.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "float.+Inf",
        "-0.1",
        "1.1",
        "0",
        "1",
        "\"x\"",
        "list",
        "[ 1 ]",
        "[ 1 1 1 ]",
        "[ 1 float.NaN ]",
        "[ 10.0 308.0 ]",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
        "[ [ 0 0 ] [ 0 0 ] ] matrix",
    ];
    for w in WORDS
        .iter()
        .filter(|w| !w.ends_with('.') && !w.ends_with(','))
    {
        for o in operands {
            for src in [
                format!("{o} {w}"),
                format!("{o} {o} {w}"),
                format!("{o} {o} {o} {w}"),
            ] {
                match run(&src) {
                    Ok(_) => {}
                    Err(e) => assert!(
                        e.contains(": E1") || e.contains("too shallow") || e.contains("returned"),
                        "{src}: {e}"
                    ),
                }
            }
        }
    }
}

// Criterion 6.

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":stat.test.t2 { 42 } register 1 2 stat.test.t2").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
