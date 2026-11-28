//! RFC-0020 criteria 1, 8 and 11, for §S10's ECDF: the definition with
//! ties, and R's `ecdf` from `hist_r.txt` (written once by `hist.R`) as a
//! second check. No test runs R.

use bund2_api::Vm as _;

use super::WORDS;
use crate::biomath::testkit::{err, fixture, floats, run};

const R: &str = include_str!("../fixtures/hist_r.txt");

fn lit(x: f64) -> String {
    format!("{x:?}")
}

fn list(xs: &[f64]) -> String {
    let cells: Vec<String> = xs.iter().map(|x| lit(*x)).collect();
    format!("[ {} ]", cells.join(" "))
}

fn ecdf(src: &str) -> (Vec<f64>, Vec<f64>) {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    let m = v.as_map().unwrap_or_else(|| panic!("not a MAP: {v:?}"));
    (floats(&m["x"]), floats(&m["y"]))
}

fn at(src: &str) -> f64 {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    floats(&i.pull().unwrap_or_else(|| panic!("{src}: empty")))[0]
}

/// The definition: each distinct value, and the count at or below it over n.
fn definition(xs: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut knots: Vec<f64> = xs.to_vec();
    knots.sort_by(f64::total_cmp);
    knots.dedup();
    let n = xs.len() as f64;
    let ys = knots
        .iter()
        .map(|k| xs.iter().filter(|x| *x <= k).count() as f64 / n)
        .collect();
    (knots, ys)
}

#[test]
fn every_form_is_registered() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("stat.ecdf"))
        .count();
    assert_eq!(ours, WORDS.len());
}

#[test]
fn ecdf_matches_the_definition_and_r_with_ties() {
    for x in ["tied", "x10"] {
        let xs = fixture(R, x);
        let (k, y) = ecdf(&format!("{} stat.ecdf", list(&xs)));
        let (dk, dy) = definition(&xs);
        assert_eq!(k, dk, "{x}: knots");
        assert_eq!(y, dy, "{x}: values");
        assert_eq!(k, fixture(R, &format!("ecdf.{x}.x")), "{x}: R's knots");
        assert_eq!(y, fixture(R, &format!("ecdf.{x}.y")), "{x}: R's values");
    }
}

#[test]
fn ecdf_at_matches_the_definition_and_r() {
    let xs = fixture(R, "tied");
    let ts = fixture(R, "ecdf.tied.at");
    let want = fixture(R, "ecdf.tied.at.y");
    for (t, w) in ts.iter().zip(&want) {
        let got = at(&format!("{} {} stat.ecdf.at", list(&xs), lit(*t)));
        let def = xs.iter().filter(|x| *x <= t).count() as f64 / xs.len() as f64;
        assert_eq!(got, def, "t = {t}");
        assert_eq!(got, *w, "t = {t}");
    }
}

#[test]
fn every_form_answers_on_its_side() {
    let x = "[ 3 1 2 2 ]";
    let plain = run(&format!("{x} stat.ecdf")).unwrap().pull().unwrap();
    let mut keep = run(&format!("{x} stat.ecdf,")).unwrap();
    let kept = keep.pull().unwrap();
    assert_eq!(kept.display(), plain.display());
    assert_eq!(
        keep.pull().map(|v| floats(&v)),
        Some(vec![3.0, 1.0, 2.0, 2.0])
    );
    let mut w = run(&format!("{x} . stat.ecdf.")).unwrap();
    assert_eq!(w.pull_workbench().unwrap().display(), plain.display());
    let mut i = run(&format!("{x} 2 stat.ecdf.at")).unwrap();
    assert_eq!(i.pull().map(|v| floats(&v)), Some(vec![0.75]));
    let mut i = run(&format!("7 {x} 2 . stat.ecdf.at.")).unwrap();
    assert_eq!(i.pull_workbench().map(|v| floats(&v)), Some(vec![0.75]));
}

#[test]
fn an_empty_series_and_a_non_finite_t_are_e101() {
    assert!(fixture_line("ecdf.empty.stop").contains("1 or more non-missing values"));
    for (src, want) in [
        (
            "list stat.ecdf",
            "STAT.ECDF: E101 series is outside stat.ecdf's domain: an empty series",
        ),
        (
            "list 1 stat.ecdf.at",
            "STAT.ECDF.AT: E101 series is outside stat.ecdf.at's domain: an empty series",
        ),
        (
            "[ 1 2 ] float.NaN stat.ecdf.at",
            "STAT.ECDF.AT: E101 t is outside stat.ecdf.at's domain: NaN",
        ),
        (
            "[ 1 2 ] float.+Inf stat.ecdf.at",
            "STAT.ECDF.AT: E101 t is outside stat.ecdf.at's domain: inf",
        ),
        (
            "[ 1 2 ] float.-Inf stat.ecdf.at",
            "STAT.ECDF.AT: E101 t is outside stat.ecdf.at's domain: -inf",
        ),
    ] {
        assert_eq!(err(src), want, "{src}");
    }
}

fn fixture_line(label: &str) -> String {
    R.lines()
        .find_map(|l| l.strip_prefix(&format!("{label} ")))
        .unwrap_or_else(|| panic!("no {label}"))
        .to_string()
}

#[test]
fn no_operand_aborts() {
    let xs = [
        "float.NaN",
        "list",
        "dict",
        "[ 1 ]",
        "[ -0.0 0.0 0 ]",
        "[ 1.0e308 -1.7e308 ]",
        "[ 1 \"a\" ]",
        "[ 1 na ]",
        "1",
    ];
    let ts = ["0", "float.NaN", "\"a\"", "1.0e308", "list"];
    let ok = |src: &str| {
        if let Err(e) = run(src) {
            assert!(
                e.contains(": E1") || e.contains("too shallow") || e.contains("returned"),
                "{src}: {e}"
            );
        }
    };
    for x in xs {
        for w in ["stat.ecdf", "stat.ecdf,", "stat.ecdf.", "stat.ecdf.,"] {
            ok(&format!("{x} {w}"));
            ok(&format!("{x} . {w}"));
        }
        for t in ts {
            ok(&format!("{x} {t} stat.ecdf.at"));
            ok(&format!("{x} {t} . stat.ecdf.at."));
        }
    }
}
