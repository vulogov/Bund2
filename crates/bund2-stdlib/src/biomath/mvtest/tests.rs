//! RFC-0020 criteria 1, 6 and 11, for §S7.
//!
//! `mvtest_r.txt` is R 4.6.2's answers and inputs, written once by
//! `mvtest.R`; `mvtest.txt` is the definitional fixture for Hotelling's
//! words, by mpmath from `mvtest.py`. No test runs R or Python.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{err, fixture, floats, run};

const R: &str = include_str!("../fixtures/mvtest_r.txt");
const DEF: &str = include_str!("../fixtures/mvtest.txt");

const TESTS: [(&str, &str); 4] = [
    ("Pillai", "pillai"),
    ("Wilks", "wilks"),
    ("Hotelling-Lawley", "hotelling-lawley"),
    ("Roy", "roy"),
];

/// A float as a Bund literal.
fn lit(x: f64) -> String {
    let s = format!("{x:?}");
    match s.find('e') {
        Some(i) if !s[..i].contains('.') => format!("{}.0{}", &s[..i], &s[i..]),
        _ => s,
    }
}

/// A row-major list as a Bund MATRIX of `cols` columns.
fn source(xs: &[f64], cols: usize) -> String {
    let rows: Vec<String> = xs
        .chunks(cols)
        .map(|r| {
            let cells: Vec<String> = r.iter().map(|x| lit(*x)).collect();
            format!("[ {} ]", cells.join(" "))
        })
        .collect();
    format!("[ {} ] matrix", rows.join(" "))
}

fn list(xs: &[f64]) -> String {
    let cells: Vec<String> = xs.iter().map(|x| lit(*x)).collect();
    format!("[ {} ]", cells.join(" "))
}

fn ints(xs: &[f64]) -> String {
    let cells: Vec<String> = xs.iter().map(|x| format!("{}", *x as i64)).collect();
    format!("[ {} ]", cells.join(" "))
}

fn answer(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    v.as_map()
        .unwrap_or_else(|| panic!("not a MAP: {v:?}"))
        .clone()
}

fn get(m: &BTreeMap<String, BundValue>, k: &str) -> Vec<f64> {
    floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")))
}

/// |a − b| ≤ 10⁻⁹|b| + 10⁻¹²·max|want|, the criteria's tolerance.
fn near(what: &str, got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{what}: {got:?} against {want:?}");
    let scale = want.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!(
            (g - w).abs() <= 1e-9 * w.abs() + 1e-12 * scale,
            "{what}[{i}]: {g} against {w}"
        );
    }
}

/// A p: relative at 10⁻⁹, or absolute at 10⁻¹² below 10⁻⁶.
fn p_near(what: &str, got: f64, want: f64) {
    let ok = if want < 1e-6 {
        (got - want).abs() <= 1e-12
    } else {
        (got - want).abs() <= 1e-9 * want
    };
    assert!(ok, "{what}: p {got} against {want}");
}

/// statistic, F, df1, df2 and p against a fixture's five, the statistic
/// first scaled by `t2_per_stat`.
fn against(what: &str, m: &BTreeMap<String, BundValue>, want: &[f64], t2_per_stat: f64) {
    near(
        &format!("{what}.statistic"),
        &get(m, "statistic"),
        &[want[0] * t2_per_stat],
    );
    near(&format!("{what}.F"), &get(m, "F"), &[want[1]]);
    near(
        &format!("{what}.df"),
        &[get(m, "df1")[0], get(m, "df2")[0]],
        &want[2..4],
    );
    p_near(what, get(m, "p")[0], want[4]);
}

fn versicolor() -> String {
    source(&fixture(R, "versicolor"), 4)
}

fn virginica() -> String {
    source(&fixture(R, "virginica"), 4)
}

fn iris() -> (String, String) {
    (
        source(&fixture(R, "iris"), 4),
        ints(&fixture(R, "iris.groups")),
    )
}

fn manova(x: &str, g: &str, test: &str) -> String {
    format!("{x} {g} dict \"test\" \"{test}\" set stat.test.manova")
}

// Criterion 1.

#[test]
fn every_word_is_registered_with_its_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("stat.test.hotelling") || n.starts_with("stat.test.manova"))
        .count();
    assert_eq!(ours, WORDS.len());
}

// Criterion 6.

#[test]
fn hotelling1_matches_the_definition_and_rs_hotelling_lawley_route() {
    let m = answer(&format!(
        "{} {} stat.test.hotelling1",
        versicolor(),
        list(&fixture(R, "mu"))
    ));
    against("definition", &m, &fixture(DEF, "hotelling1"), 1.0);
    // R's statistic is Hotelling-Lawley's trace, T²/(n − 1).
    against("R", &m, &fixture(R, "h1.stat"), 49.0);
    near("F", &get(&m, "F"), &[1.834_789_379_5]);
}

#[test]
fn hotelling2_matches_the_definition_and_rs_hotelling_lawley_route() {
    let m = answer(&format!(
        "{} {} stat.test.hotelling2",
        versicolor(),
        virginica()
    ));
    against("definition", &m, &fixture(DEF, "hotelling2"), 1.0);
    // T² is (n₁ + n₂ − 2) times Hotelling-Lawley's trace.
    against("R", &m, &fixture(R, "h2.stat"), 98.0);
    assert!((get(&m, "statistic")[0] - 355.472_145_199).abs() < 1e-8);
}

#[test]
fn hotelling2_is_manovas_hotelling_lawley_for_two_groups() {
    let two: Vec<f64> = (0..100)
        .map(|i| f64::from(1 + i32::from(i >= 50)))
        .collect();
    let rows = [fixture(R, "versicolor"), fixture(R, "virginica")].concat();
    let m = answer(&manova(&source(&rows, 4), &ints(&two), "hotelling-lawley"));
    let h = answer(&format!(
        "{} {} stat.test.hotelling2",
        versicolor(),
        virginica()
    ));
    near("F", &get(&m, "F"), &get(&h, "F"));
    near("p", &get(&m, "p"), &get(&h, "p"));
}

#[test]
fn manova_gives_rs_four_statistics_on_iris() {
    let (x, g) = iris();
    for (r, ours) in TESTS {
        let m = answer(&manova(&x, &g, ours));
        against(r, &m, &fixture(R, &format!("m3.{r}.stat")), 1.0);
        near(
            &format!("{r}.eig"),
            &get(&m, "eig"),
            &fixture(R, &format!("m3.{r}.eig")),
        );
        assert_eq!(m["test"].as_str().as_deref(), Some(ours));
    }
    // Pillai's is the default.
    let plain = answer(&format!("{x} {g} dict stat.test.manova"));
    let named = answer(&manova(&x, &g, "pillai"));
    assert_eq!(format!("{plain:?}"), format!("{named:?}"));
}

#[test]
fn string_labels_answer_as_integer_labels() {
    let (x, g) = iris();
    let names: Vec<String> = fixture(R, "iris.groups")
        .iter()
        .map(|v| ["\"setosa\"", "\"versicolor\"", "\"virginica\""][*v as usize - 1].to_string())
        .collect();
    let s = format!("[ {} ]", names.join(" "));
    let a = answer(&manova(&x, &g, "wilks"));
    let b = answer(&manova(&x, &s, "wilks"));
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}

#[test]
fn a_rank_deficient_e_is_e106() {
    assert!(R.contains("deficient.stop residuals have rank 3 < 4"));
    let mut rows = fixture(R, "iris");
    for r in rows.chunks_mut(4) {
        r[3] = r[0] + r[1];
    }
    let e = err(&manova(&source(&rows, 4), &iris().1, "pillai"));
    assert_eq!(
        e,
        "STAT.TEST.MANOVA: E106 the matrix is singular: column 4 depends on those before it"
    );
}

#[test]
fn the_near_collinear_pair_answers_either_side_of_rs_boundary() {
    let g: Vec<f64> = (0..20).map(|i| f64::from(1 + i32::from(i >= 10))).collect();
    let x = source(&fixture(R, "near.1e-3"), 2);
    for (r, ours) in TESTS {
        let m = answer(&manova(&x, &ints(&g), ours));
        against(r, &m, &fixture(R, &format!("near.1e-3.{r}.stat")), 1.0);
    }
    let x = source(&fixture(R, "near.1e-4"), 2);
    for (r, ours) in TESTS {
        assert!(R.contains(&format!("near.1e-4.{r}.stop residuals have rank 1 < 2")));
        let e = err(&manova(&x, &ints(&g), ours));
        assert!(e.starts_with("STAT.TEST.MANOVA: E106 "), "{e}");
    }
}

#[test]
fn zero_df_refuses_hotelling_lawley_and_answers_wilks() {
    let x = source(&fixture(R, "zero"), 2);
    let g = "[ 1 1 2 2 3 ]";
    assert_eq!(
        err(&manova(&x, g, "hotelling-lawley")),
        "STAT.TEST.MANOVA: E101 test is outside stat.test.manova's domain: \
         \"hotelling-lawley\", whose F is 0 on 4 and 0 here"
    );
    for (r, ours) in [("Pillai", "pillai"), ("Wilks", "wilks"), ("Roy", "roy")] {
        let m = answer(&manova(&x, g, ours));
        against(r, &m, &fixture(R, &format!("zero.{r}.stat")), 1.0);
    }
    let m = answer(&manova(&x, g, "wilks"));
    near("Wilks", &get(&m, "statistic"), &[0.001_862_891_207_15]);
    near("F", &get(&m, "F"), &[11.084_472_366_1]);
}

#[test]
fn each_refusal_is_its_template() {
    let x = "[ [ 1 2 ] [ 2 1 ] [ 3 5 ] [ 4 3 ] [ 6 2 ] ] matrix";
    for (src, want) in [
        (
            format!("{x} [ 1 2 3 ] stat.test.hotelling1"),
            "STAT.TEST.HOTELLING1: E102 mu and the columns of X do not agree: 3 and 2",
        ),
        (
            "[ [ 1 2 ] [ 2 1 ] ] matrix [ 0 0 ] stat.test.hotelling1".into(),
            "STAT.TEST.HOTELLING1: E101 X is outside stat.test.hotelling1's domain: 2 rows for 2 columns",
        ),
        (
            format!("{x} 0 stat.test.hotelling1"),
            "STAT.TEST.HOTELLING1: E101 mu is outside stat.test.hotelling1's domain: 0",
        ),
        (
            "[ [ 1 2 ] [ 2 4 ] [ 3 6 ] ] matrix [ 0 0 ] stat.test.hotelling1".into(),
            "STAT.TEST.HOTELLING1: E106 the Hessian of -loglik at par is not positive definite: pivot 2 is 0",
        ),
        (
            "[ [ 1.0e200 2 ] [ -1.0e200 1 ] [ 3 5 ] ] matrix [ 0 0 ] stat.test.hotelling1".into(),
            "STAT.TEST.HOTELLING1: E114 the answer overflows a FLOAT: the covariance of X",
        ),
        (
            format!("{x} [ [ 1 2 3 ] ] matrix stat.test.hotelling2"),
            "STAT.TEST.HOTELLING2: E102 the columns of X and Y do not agree: 2 and 3",
        ),
        (
            "[ [ 1 2 ] ] matrix [ [ 3 4 ] [ 5 7 ] ] matrix stat.test.hotelling2".into(),
            "STAT.TEST.HOTELLING2: E101 Y is outside stat.test.hotelling2's domain: 1 and 2 rows for 2 columns",
        ),
        (
            format!("{x} [ 1 1 2 2 3 ] dict \"test\" \"Pillai\" set stat.test.manova"),
            "STAT.TEST.MANOVA: E101 test is outside stat.test.manova's domain: \"Pillai\"",
        ),
        (
            format!("{x} [ 1 1 2 2 3 ] dict \"tol\" 1 set stat.test.manova"),
            "STAT.TEST.MANOVA: E101 opts is outside stat.test.manova's domain: an unknown key \"tol\"",
        ),
        (
            "[ [ 1 ] [ 2 ] [ 4 ] ] matrix [ 1 1 2 ] dict stat.test.manova".into(),
            "STAT.TEST.MANOVA: E101 X is outside stat.test.manova's domain: one column, where MANOVA needs two or more",
        ),
        (
            format!("{x} [ 1 1 1 1 1 ] dict stat.test.manova"),
            "STAT.TEST.MANOVA: E101 groups is outside stat.test.manova's domain: one group",
        ),
        (
            format!("{x} [ 1 1 2 3 4 ] dict stat.test.manova"),
            "STAT.TEST.MANOVA: E101 X is outside stat.test.manova's domain: 5 rows in 4 groups for 2 columns",
        ),
        (
            format!("{x} [ 1 1 2 2 ] dict stat.test.manova"),
            "STAT.TEST.MANOVA: E102 the labels of groups and the rows of X do not agree: 4 and 5",
        ),
        (
            format!("{x} [ 1 1 2 2 \"a\" ] dict stat.test.manova"),
            "STAT.TEST.MANOVA: E101 groups is outside stat.test.manova's domain: \"a\", among labels of another type",
        ),
        (
            format!("{x} [ 1 1 2 2 3.0 ] dict stat.test.manova"),
            "STAT.TEST.MANOVA: E101 groups is outside stat.test.manova's domain: 3.0",
        ),
        (
            format!("{x} 1 dict stat.test.manova"),
            "STAT.TEST.MANOVA: E101 groups is outside stat.test.manova's domain: 1",
        ),
        (
            "[ [ 1.0e200 2 ] [ -1.0e200 1 ] [ 3 5 ] [ 1 1 ] ] matrix [ 1 1 2 2 ] dict stat.test.manova".into(),
            "STAT.TEST.MANOVA: E114 the answer overflows a FLOAT: the SSCP matrices",
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}

// Criterion 11.

#[test]
fn no_operand_aborts() {
    let xs = [
        "float.NaN",
        "list",
        "dict",
        "list matrix",
        "[ [ 1 ] ] matrix",
        "[ [ 1 2 ] [ 3 ] ] matrix",
        "[ [ 1 \"a\" ] [ 3 4 ] ] matrix",
        "[ [ 1 float.NaN ] [ 3 4 ] ] matrix",
        "[ [ 0 0 ] [ 0 0 ] [ 0 0 ] [ 0 0 ] ] matrix",
        "[ [ 1 2 ] [ 2 1 ] [ 3 5 ] [ 4 3 ] [ 6 2 ] ] matrix",
        "[ [ 1.0e307 -1.0e307 ] [ -1.0e307 1.0e307 ] [ 1.0e307 1.0e307 ] [ 1 2 ] ] matrix",
        "[ [ 1.0e160 1 ] [ -1.0e160 2 ] [ 3 4 ] [ 5 5 ] ] matrix",
        "[ [ 1.0e-300 1 ] [ 2.0e-300 2 ] [ 3.0e-300 4 ] [ 5.0e-300 5 ] ] matrix",
    ];
    let seconds = [
        "[ 0 0 ]",
        "[ float.NaN 0 ]",
        "[ 1 1 2 2 ]",
        "[ 1 1 2 2 3 ]",
        "[ \"a\" \"a\" \"b\" \"b\" ]",
        "[ na 1 2 2 ]",
        "list",
        "na",
        "1",
        "[ [ 1 2 ] [ 3 4 ] [ 5 7 ] ] matrix",
        "[ [ 1.0e160 1 ] [ -1.0e160 2 ] ] matrix",
    ];
    let opts = [
        "dict",
        "\"x\"",
        "na",
        "dict \"test\" \"wilks\" set",
        "dict \"test\" \"hotelling-lawley\" set",
        "dict \"test\" \"roy\" set",
        "dict \"test\" 1 set",
    ];
    let ok = |src: &str| {
        if let Err(e) = run(src) {
            assert!(
                e.contains(": E1") || e.contains("too shallow"),
                "{src}: {e}"
            );
        }
    };
    for a in xs {
        for b in seconds {
            for w in ["stat.test.hotelling1", "stat.test.hotelling2"] {
                ok(&format!("{a} {b} {w}"));
                ok(&format!("{a} {b} . {w}."));
            }
            for o in opts {
                ok(&format!("{a} {b} {o} stat.test.manova"));
                ok(&format!("{a} {b} {o} . stat.test.manova."));
            }
        }
        for w in WORDS {
            ok(&format!("{a} {w}"));
        }
    }
}

// The `.` forms, and a program's own word.

#[test]
fn each_dot_form_answers_on_the_workbench() {
    let x = "[ [ 1 2 ] [ 2 1 ] [ 3 5 ] [ 4 3 ] [ 6 2 ] [ 2 2 ] ] matrix";
    for (args, w) in [
        (format!("{x} [ 0 0 ]"), "stat.test.hotelling1"),
        (
            format!("{x} [ [ 0 1 ] [ 3 3 ] [ 1 4 ] ] matrix"),
            "stat.test.hotelling2",
        ),
        (format!("{x} [ 1 1 2 2 3 3 ] dict"), "stat.test.manova"),
    ] {
        let plain = run(&format!("{args} {w}")).unwrap().pull().unwrap();
        let mut i = run(&format!("7 {args} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        let got = i.pull_workbench().unwrap();
        assert_eq!(format!("{got:?}"), format!("{plain:?}"), "{w}.");
    }
}

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":stat.test.manova { 42 } register 1 stat.test.manova").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
