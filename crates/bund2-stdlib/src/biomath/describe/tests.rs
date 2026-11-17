//! RFC-0015 criteria 2, 3, 5 and 9, for §S6's descriptive words.
//!
//! The fixture X = [ 2 4 4 4 5 5 7 9 ], by hand: mean 5, deviations
//! −3 −1 −1 −1 0 0 2 4, so Σd² = 32 and the variance 32/7; sorted, the middle
//! pair is 4 and 5, so the median 4.5; 4 is the mode, three times. The
//! population moments are m₂ = 4, m₃ = 42/8, m₄ = 356/8, so g₁ = 0.65625 and
//! g₂ = 2.78125 − 3 = −0.21875.

use bund2_api::Vm as _;

use super::WORDS;
use crate::biomath::testkit::{close, err, fixture, floats, run, top};

const FIXTURE: &str = include_str!("../fixtures/describe.txt");
const X: &str = "[ 2 4 4 4 5 5 7 9 ]";
const Y: &str = "[ 1 3 2 5 4 6 8 7 ]";

fn want(key: &str) -> Vec<f64> {
    fixture(FIXTURE, key)
}

#[test]
fn every_word_and_form_is_registered() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
    }
    // Beside the reference's `stat.count` and `stat.mean*`, M5's tests,
    // corrections and model, which `infer`'s own test holds, and RFC-0016's
    // correlation tests, which `infer::cor`'s does, nothing else.
    let ours: Vec<String> = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| {
            n.starts_with("stat.")
                && !n.starts_with("stat.count")
                && !n.starts_with("stat.mean")
                && !crate::biomath::infer::WORDS.contains(&n.as_str())
                && !crate::biomath::infer::cor::WORDS.contains(&n.as_str())
        })
        .collect();
    assert_eq!(ours.len(), WORDS.len(), "{ours:?}");
}

#[test]
fn the_hand_worked_values() {
    close("var", &top(&format!("{X} stat.var")), &[32.0 / 7.0], 1e-12);
    close("median", &top(&format!("{X} stat.median")), &[4.5], 0.0);
    close("mode", &top(&format!("{X} stat.mode")), &[4.0], 0.0);
    close(
        "skewness",
        &top(&format!("{X} stat.skewness")),
        &[0.65625],
        1e-12,
    );
    close(
        "kurtosis",
        &top(&format!("{X} stat.kurtosis")),
        &[-0.21875],
        1e-12,
    );
}

#[test]
fn every_word_against_the_fixture() {
    for w in [
        "stat.var",
        "stat.sd",
        "stat.se",
        "stat.median",
        "stat.mode",
        "stat.skewness",
        "stat.kurtosis",
        "stat.zscore",
    ] {
        close(
            w,
            &top(&format!("{X} {w}")),
            &want(&format!("{w} X")),
            1e-12,
        );
    }
    for p in ["0", "0.1", "0.25", "0.5", "0.9", "1"] {
        close(
            p,
            &top(&format!("{X} {p} stat.quantile")),
            &want(&format!("stat.quantile X {p}")),
            1e-12,
        );
    }
    for w in [
        "stat.cov",
        "stat.cor.pearson",
        "stat.cor.spearman",
        "stat.cor.kendall",
    ] {
        close(
            w,
            &top(&format!("{X} {Y} {w}")),
            &want(&format!("{w} X Y")),
            1e-12,
        );
    }
}

#[test]
fn the_mode_takes_the_smallest_of_a_tie() {
    close("mode", &top("[ 3 1 3 1 2 ] stat.mode"), &[1.0], 0.0);
}

#[test]
fn all_four_forms_and_both_quantile_forms() {
    let want_var = 32.0 / 7.0;
    // Plain: the answer replaces the LIST.
    let i = run(&format!("7 {X} stat.var")).unwrap();
    assert_eq!(i.snapshot().len(), 2);
    // `,`: the LIST stays beneath its answer.
    let mut i = run(&format!("7 {X} stat.var,")).unwrap();
    assert_eq!(i.snapshot().len(), 3);
    close(",", &floats(&i.pull().unwrap()), &[want_var], 1e-12);
    // `.`: from the workbench, the answer there.
    let mut i = run(&format!("7 {X} . stat.var.")).unwrap();
    assert_eq!((i.snapshot().len(), i.workbench_depth()), (1, 1));
    close(
        ".",
        &floats(&i.pull_workbench().unwrap()),
        &[want_var],
        1e-12,
    );
    // `.,`: the LIST stays on the workbench beneath.
    let mut i = run(&format!("7 {X} . stat.var.,")).unwrap();
    assert_eq!((i.snapshot().len(), i.workbench_depth()), (1, 2));
    close(
        ".,",
        &floats(&i.pull_workbench().unwrap()),
        &[want_var],
        1e-12,
    );
    // `stat.quantile.` takes p from the workbench and the series from the
    // stack, and answers on the workbench.
    let mut i = run(&format!("{X} 0.9 . stat.quantile.")).unwrap();
    assert_eq!(i.snapshot().len(), 0);
    close(
        "quantile.",
        &floats(&i.pull_workbench().unwrap()),
        &[7.6],
        1e-12,
    );
    // `stat.cor.pearson.` takes b from the workbench.
    let mut i = run(&format!("{X} {Y} . stat.cor.pearson.")).unwrap();
    assert_eq!(i.snapshot().len(), 0);
    close(
        "pearson.",
        &floats(&i.pull_workbench().unwrap()),
        &want("stat.cor.pearson X Y"),
        1e-12,
    );
}

#[test]
fn a_drain_reads_for_one_series_and_is_e115_for_two() {
    close(
        "drain",
        &top("2 4 4 4 5 5 7 9 stat.var"),
        &[32.0 / 7.0],
        1e-12,
    );
    close(
        "drain",
        &top("2 4 4 4 5 5 7 9 0.5 stat.quantile"),
        &[4.5],
        0.0,
    );
    assert_eq!(
        err(&format!("1 2 {Y} stat.cov")),
        "STAT.COV: E115 a must be a LIST or METRICS; a drained series is not allowed here"
    );
}

#[test]
fn the_refusals() {
    assert_eq!(
        err("[ 3 ] stat.var"),
        "STAT.VAR: E101 series is outside stat.var's domain: a series of 1 value"
    );
    assert_eq!(
        err("list stat.median"),
        "STAT.MEDIAN: E101 series is outside stat.median's domain: an empty series"
    );
    assert_eq!(
        err("[ 2 2 2 ] stat.skewness"),
        "STAT.SKEWNESS: E101 series is outside stat.skewness's domain: a constant series"
    );
    assert_eq!(
        err("[ 2 2 2 ] stat.zscore"),
        "STAT.ZSCORE: E101 series is outside stat.zscore's domain: a constant series"
    );
    assert_eq!(
        err(&format!("{X} 1.1 stat.quantile")),
        "STAT.QUANTILE: E101 p is outside stat.quantile's domain: 1.1"
    );
    assert_eq!(
        err(&format!("{X} [ 1 2 ] stat.cov")),
        "STAT.COV: E102 lengths do not agree: 8 and 2"
    );
    assert_eq!(
        err(&format!("{X} [ 1 1 1 1 1 1 1 1 ] stat.cor.kendall")),
        "STAT.COR.KENDALL: E101 b is outside stat.cor.kendall's domain: a constant series"
    );
    assert_eq!(
        err("1 list + float.+Inf swap + stat.var,"),
        "STAT.VAR,: E101 series is outside stat.var,'s domain: inf at 2"
    );
}

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "float.+Inf",
        "-0.1",
        "1.1",
        "0",
        "\"x\"",
        "list",
        "[ 0 ]",
        "[ 1 2 ]",
    ];
    for w in WORDS
        .iter()
        .filter(|w| !w.ends_with('.') && !w.ends_with(','))
    {
        for o in operands {
            for src in [format!("{o} {w}"), format!("{o} {o} {w}")] {
                if let Ok(mut i) = run(&src)
                    && let Some(v) = i.pull()
                {
                    let ys = floats(&v);
                    assert!(ys.iter().all(|y| y.is_finite()), "{src}: {ys:?}");
                }
            }
        }
    }
}
