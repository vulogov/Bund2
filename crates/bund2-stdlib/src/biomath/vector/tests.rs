//! RFC-0015 criteria 2, 3, 5 and 8, for §S5.

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{close, err, fixture, floats, run, top};

const FIXTURE: &str = include_str!("../fixtures/describe.txt");
const A: &str = "[ 1 2 3 4 ]";
const B: &str = "[ 0.5 -1 2 8 ]";

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
    let ours: Vec<String> = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("vec."))
        .collect();
    assert_eq!(ours.len(), WORDS.len(), "{ours:?}");
}

#[test]
fn the_binary_words_on_the_fixture() {
    for w in ["vec.add", "vec.sub", "vec.mul", "vec.div", "vec.dot"] {
        close(
            w,
            &top(&format!("{A} {B} {w}")),
            &want(&format!("{w} A B")),
            1e-12,
        );
    }
    close(
        "vec.scale",
        &top(&format!("{A} 2.5 vec.scale")),
        &want("vec.scale A 2.5"),
        1e-12,
    );
}

#[test]
fn the_series_words_on_the_fixture() {
    for (name, s) in [("A", A), ("B", B)] {
        for w in [
            "vec.sum",
            "vec.prod",
            "vec.cumsum",
            "vec.cumprod",
            "vec.diff",
            "vec.logsumexp",
            "vec.normalize.sum",
        ] {
            close(
                w,
                &top(&format!("{s} {w}")),
                &want(&format!("{w} {name}")),
                1e-12,
            );
        }
    }
    close(
        "vec.logsumexp",
        &top("[ 1000 1000 ] vec.logsumexp"),
        &[1000.0 + 2f64.ln()],
        1e-12,
    );
}

#[test]
fn a_drained_series_reads_and_a_drained_pair_is_e115() {
    // One series may be a drain: from the top down, so 4 3 2 1.
    close("drain", &top("1 2 3 4 vec.diff"), &[-1.0, -1.0, -1.0], 0.0);
    assert_eq!(
        err(&format!("1 2 {B} vec.add")),
        "VEC.ADD: E115 a must be a LIST or METRICS; a drained series is not allowed here"
    );
    assert_eq!(
        err(&format!("{A} 3 vec.dot")),
        "VEC.DOT: E115 b must be a LIST or METRICS; a drained series is not allowed here"
    );
}

#[test]
fn the_refusals() {
    assert_eq!(
        err("[ 1 2 3 ] [ 1 2 ] vec.add"),
        "VEC.ADD: E102 lengths do not agree: 3 and 2"
    );
    assert_eq!(
        err("[ 1 2 3 ] [ 1 0 2 ] vec.div"),
        "VEC.DIV: E101 b is outside vec.div's domain: 0 at 2"
    );
    assert_eq!(
        err("[ 1 \"x\" ] [ 1 2 ] vec.add"),
        "VEC.ADD: E103 a is not a LIST of numbers: \"x\""
    );
    assert_eq!(
        err("[ 1 -1 ] vec.normalize.sum"),
        "VEC.NORMALIZE.SUM: E101 series is outside vec.normalize.sum's domain: a series summing to 0"
    );
    assert_eq!(
        err("list vec.logsumexp"),
        "VEC.LOGSUMEXP: E101 series is outside vec.logsumexp's domain: an empty series"
    );
    let big = "10.0 300.0 math.power";
    assert_eq!(
        err(&format!("{big} list + {big} swap + vec.prod")),
        "VEC.PROD: E114 the answer overflows a FLOAT: Π xᵢ"
    );
    assert_eq!(
        err(&format!("{big} list + 1000000000 vec.scale")),
        "VEC.SCALE: E114 the answer overflows a FLOAT: a × k at 1"
    );
    assert_eq!(
        err("1 list + float.NaN swap + vec.sum"),
        "VEC.SUM: E101 series is outside vec.sum's domain: NaN at 2"
    );
}

#[test]
fn the_forms() {
    // `,` keeps the LIST beneath its answer; `.` and `.,` work on the
    // workbench.
    let mut i = run(&format!("{A} vec.sum,")).unwrap();
    assert_eq!(i.snapshot().len(), 2);
    assert_eq!(i.pull().map(|v| floats(&v)), Some(vec![10.0]));
    let mut i = run(&format!("7 {A} . vec.sum.")).unwrap();
    assert_eq!(i.snapshot().len(), 1);
    assert_eq!(i.pull_workbench().map(|v| floats(&v)), Some(vec![10.0]));
    let mut i = run(&format!("7 {A} . vec.sum.,")).unwrap();
    assert_eq!(i.workbench_depth(), 2);
    assert_eq!(i.pull_workbench().map(|v| floats(&v)), Some(vec![10.0]));
    // `vec.scale.` takes k from the workbench, the series from the stack.
    let mut i = run(&format!("{A} 2 . vec.scale.")).unwrap();
    assert_eq!(i.snapshot().len(), 0);
    let v = i.pull_workbench().unwrap();
    assert_eq!(floats(&v), [2.0, 4.0, 6.0, 8.0]);
    // `vec.add.` takes b from the workbench.
    let mut i = run(&format!("7 {A} {B} . vec.add.")).unwrap();
    let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
    assert_eq!(shown, ["7"]);
    let v = i.pull_workbench().unwrap();
    close("vec.add.", &floats(&v), &want("vec.add A B"), 1e-12);
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
