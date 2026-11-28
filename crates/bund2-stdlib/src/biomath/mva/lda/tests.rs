//! RFC-0020 criteria 1, 7 and 11, for §S8.
//!
//! `lda_r.txt` is R 4.6.2's and MASS 7.3-66's answers and inputs, written
//! once by `lda.R`. No test runs R.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use crate::biomath::testkit::{err, fixture, floats, run};

const R: &str = include_str!("../../fixtures/lda_r.txt");

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

/// A LIST, or a MATRIX row-major.
fn get(m: &BTreeMap<String, BundValue>, k: &str) -> Vec<f64> {
    let v = m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}"));
    match v.as_matrix() {
        Some(rows) => rows.iter().flatten().flat_map(floats).collect(),
        None => floats(v),
    }
}

fn labels(m: &BTreeMap<String, BundValue>, k: &str) -> Vec<i64> {
    m.get(k)
        .and_then(|v| v.as_list().map(<[BundValue]>::to_vec))
        .unwrap_or_else(|| panic!("no LIST {k} in {m:?}"))
        .iter()
        .map(|v| v.as_int().unwrap_or_else(|| panic!("{k}: {v:?}")))
        .collect()
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

/// Row-major matrices of `cols` columns, each column equal up to sign.
fn near_up_to_sign(what: &str, got: &[f64], want: &[f64], cols: usize) {
    assert_eq!(got.len(), want.len(), "{what}: {got:?} against {want:?}");
    let mut flipped = got.to_vec();
    for j in 0..cols {
        let dot: f64 = (j..got.len()).step_by(cols).map(|i| got[i] * want[i]).sum();
        if dot < 0.0 {
            for i in (j..got.len()).step_by(cols) {
                flipped[i] = -flipped[i];
            }
        }
    }
    near(what, &flipped, want);
}

/// A posterior: each entry at 10⁻⁹ relative or 10⁻¹² absolute.
fn posterior_near(what: &str, got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{what}");
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        assert!(
            (g - w).abs() <= 1e-9 * w.abs() + 1e-12,
            "{what}[{i}]: {g} against {w}"
        );
    }
}

/// Reverse the columns of a row-major matrix of `cols` columns.
fn reverse_columns(xs: &[f64], cols: usize) -> Vec<f64> {
    xs.chunks(cols)
        .flat_map(|r| r.iter().rev().copied().collect::<Vec<_>>())
        .collect()
}

fn iris() -> (String, String) {
    (
        source(&fixture(R, "iris"), 4),
        ints(&fixture(R, "iris.groups")),
    )
}

/// The rows of iris R predicts, 1-based.
fn rows() -> String {
    let x = fixture(R, "iris");
    let picked: Vec<f64> = fixture(R, "rows")
        .iter()
        .flat_map(|r| {
            let i = (*r as usize - 1) * 4;
            x[i..i + 4].to_vec()
        })
        .collect();
    source(&picked, 4)
}

fn check_fit(label: &str, m: &BTreeMap<String, BundValue>) {
    near(
        &format!("{label}.prior"),
        &get(m, "prior"),
        &fixture(R, &format!("{label}.prior")),
    );
    near(
        &format!("{label}.counts"),
        &get(m, "counts"),
        &fixture(R, &format!("{label}.counts")),
    );
    near(
        &format!("{label}.means"),
        &get(m, "means"),
        &fixture(R, &format!("{label}.means")),
    );
    let svd = fixture(R, &format!("{label}.svd"));
    near(&format!("{label}.svd"), &get(m, "svd"), &svd);
    near_up_to_sign(
        &format!("{label}.scaling"),
        &get(m, "scaling"),
        &fixture(R, &format!("{label}.scaling")),
        svd.len(),
    );
    let lev: Vec<i64> = fixture(R, &format!("{label}.lev"))
        .iter()
        .map(|x| *x as i64)
        .collect();
    assert_eq!(labels(m, "lev"), lev, "{label}.lev");
    assert_eq!(get(m, "N"), [150.0], "{label}.N");
}

fn check_pred(label: &str, m: &BTreeMap<String, BundValue>, dimen: usize) {
    let class: Vec<i64> = fixture(R, &format!("{label}.class"))
        .iter()
        .map(|x| *x as i64)
        .collect();
    assert_eq!(labels(m, "class"), class, "{label}.class");
    posterior_near(
        &format!("{label}.posterior"),
        &get(m, "posterior"),
        &fixture(R, &format!("{label}.posterior")),
    );
    near_up_to_sign(
        &format!("{label}.x"),
        &get(m, "x"),
        &fixture(R, &format!("{label}.x")),
        dimen,
    );
}

// Criterion 1.

#[test]
fn both_words_are_registered_as_manova_is_with_their_dot_forms() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    let effect = |w: &str| {
        format!(
            "{:?}",
            i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"))
        )
    };
    for w in ["mva.lda", "mva.lda.predict"] {
        assert_eq!(effect(w), effect("stat.test.manova"), "{w}");
        assert_eq!(
            effect(&format!("{w}.")),
            effect("stat.test.manova."),
            "{w}."
        );
    }
}

// Criterion 7.

#[test]
fn lda_on_iris_gives_rs_fit() {
    let (x, g) = iris();
    check_fit("fit", &answer(&format!("{x} {g} dict mva.lda")));
}

#[test]
fn predict_gives_rs_class_posterior_and_scores_on_rs_four_rows() {
    let (x, g) = iris();
    let fit = format!("{x} {g} dict mva.lda");
    let r = rows();
    check_pred(
        "pred",
        &answer(&format!("{fit} {r} dict mva.lda.predict")),
        2,
    );
    check_pred(
        "pred.dimen1",
        &answer(&format!("{fit} {r} dict \"dimen\" 1 set mva.lda.predict")),
        1,
    );
    // dimen above r is clamped, as R's min (`lda.R:260`).
    check_pred(
        "pred",
        &answer(&format!("{fit} {r} dict \"dimen\" 7 set mva.lda.predict")),
        2,
    );
}

#[test]
fn a_zero_prior_gives_posterior_zero() {
    let (x, g) = iris();
    let m = answer(&format!(
        "{x} {g} dict mva.lda {} dict \"prior\" [ 0 0.5 0.5 ] set mva.lda.predict",
        rows()
    ));
    check_pred("pred.prior0", &m, 2);
    let post = get(&m, "posterior");
    for i in 0..4 {
        assert_eq!(post[3 * i], 0.0, "row {i}");
    }
}

#[test]
fn a_given_prior_gives_rs_fit_and_prediction() {
    let (x, g) = iris();
    let fit = format!("{x} {g} dict \"prior\" [ 0.5 0.3 0.2 ] set mva.lda");
    check_fit("fitp", &answer(&fit));
    check_pred(
        "predp",
        &answer(&format!("{fit} {} dict mva.lda.predict", rows())),
        2,
    );
}

#[test]
fn reversed_levels_reverse_the_groups_and_change_nothing_else() {
    let (x, g) = iris();
    let fit = |extra: &str| format!("{x} {g} dict {extra} mva.lda");
    let plain = answer(&fit(""));
    let rev = answer(&fit("\"levels\" [ 3 2 1 ] set"));
    assert_eq!(labels(&rev, "lev"), [3, 2, 1]);
    let rv = |v: Vec<f64>| v.into_iter().rev().collect::<Vec<_>>();
    near("prior", &get(&rev, "prior"), &rv(get(&plain, "prior")));
    near("counts", &get(&rev, "counts"), &rv(get(&plain, "counts")));
    let means: Vec<f64> = get(&plain, "means")
        .chunks(4)
        .rev()
        .flatten()
        .copied()
        .collect();
    near("means", &get(&rev, "means"), &means);
    near("svd", &get(&rev, "svd"), &get(&plain, "svd"));
    // §S1's sign rule makes the columns equal, not only up to sign.
    near("scaling", &get(&rev, "scaling"), &get(&plain, "scaling"));

    let r = rows();
    let p0 = answer(&format!("{} {r} dict mva.lda.predict", fit("")));
    let p1 = answer(&format!(
        "{} {r} dict mva.lda.predict",
        fit("\"levels\" [ 3 2 1 ] set")
    ));
    assert_eq!(labels(&p0, "class"), labels(&p1, "class"));
    near("x", &get(&p1, "x"), &get(&p0, "x"));
    posterior_near(
        "posterior",
        &get(&p1, "posterior"),
        &reverse_columns(&get(&p0, "posterior"), 3),
    );

    // A non-uniform prior in lev's order, reversed with levels: R's own
    // answers both ways, and the same posteriors, columns reversed.
    let fp = format!("{x} {g} dict \"prior\" [ 0.5 0.3 0.2 ] set mva.lda");
    let fr = format!("{x} {g} dict \"prior\" [ 0.2 0.3 0.5 ] set \"levels\" [ 3 2 1 ] set mva.lda");
    check_fit("fitr", &answer(&fr));
    let pp = answer(&format!("{fp} {r} dict mva.lda.predict"));
    let pr = answer(&format!("{fr} {r} dict mva.lda.predict"));
    check_pred("predr", &pr, 2);
    assert_eq!(labels(&pp, "class"), labels(&pr, "class"));
    near("x", &get(&pr, "x"), &get(&pp, "x"));
    posterior_near(
        "posterior",
        &get(&pr, "posterior"),
        &reverse_columns(&get(&pp, "posterior"), 3),
    );
}

#[test]
fn string_labels_answer_as_integer_labels() {
    let (x, _) = iris();
    let names = ["setosa", "versicolor", "virginica"];
    let g: Vec<String> = fixture(R, "iris.groups")
        .iter()
        .map(|k| format!("{:?}", names[*k as usize - 1]))
        .collect();
    let g = format!("[ {} ]", g.join(" "));
    let m = answer(&format!("{x} {g} dict mva.lda"));
    let shown: Vec<String> = m["lev"]
        .as_list()
        .unwrap_or(&[])
        .iter()
        .map(BundValue::display)
        .collect();
    assert_eq!(shown, names);
    near("svd", &get(&m, "svd"), &fixture(R, "fit.svd"));
    let p = answer(&format!(
        "{x} {g} dict mva.lda {} dict mva.lda.predict",
        rows()
    ));
    let class: Vec<String> = p["class"]
        .as_list()
        .unwrap_or(&[])
        .iter()
        .map(BundValue::display)
        .collect();
    assert_eq!(class, ["setosa", "versicolor", "virginica", "virginica"]);
}

#[test]
fn a_collinear_variable_answers_fewer_dimensions_of_rs_fit() {
    let mut x = fixture(R, "iris");
    for r in x.chunks_mut(4) {
        r[3] = r[0] + r[1];
    }
    let x = source(&x, 4);
    let g = ints(&fixture(R, "iris.groups"));
    let fit = format!("{x} {g} dict mva.lda");
    let m = answer(&fit);
    check_fit("collinear", &m);
    let mut r = Vec::new();
    let xs = fixture(R, "iris");
    for k in fixture(R, "rows") {
        let i = (k as usize - 1) * 4;
        r.extend_from_slice(&[xs[i], xs[i + 1], xs[i + 2], xs[i] + xs[i + 1]]);
    }
    check_pred(
        "collinear.pred",
        &answer(&format!("{fit} {} dict mva.lda.predict", source(&r, 4))),
        2,
    );
}

#[test]
fn a_variable_constant_within_groups_is_e101_naming_it() {
    // R: "variable 3 appears to be constant within groups".
    assert!(fixture_text("constant.stop").contains("variable 3 appears"));
    let mut x = fixture(R, "iris");
    let g = fixture(R, "iris.groups");
    for (r, k) in x.chunks_mut(4).zip(&g) {
        r[2] = *k;
    }
    assert_eq!(
        err(&format!("{} {} dict mva.lda", source(&x, 4), ints(&g))),
        "MVA.LDA: E101 X is outside mva.lda's domain: variable 3, constant within groups"
    );
    for (r, k) in x.chunks_mut(4).zip(&g) {
        r[0] = *k * 2.0;
    }
    assert_eq!(
        err(&format!("{} {} dict mva.lda", source(&x, 4), ints(&g))),
        "MVA.LDA: E101 X is outside mva.lda's domain: variables 1 3, constant within groups"
    );
}

fn fixture_text(label: &str) -> String {
    R.lines()
        .find_map(|l| l.strip_prefix(&format!("{label} ")))
        .unwrap_or_else(|| panic!("no {label}"))
        .to_string()
}

// Criterion 11 and the refusals.

#[test]
fn each_refusal_is_its_template() {
    let x = "[ [ 1 2 ] [ 2 1 ] [ 3 5 ] [ 4 3 ] [ 6 2 ] [ 2 2 ] ] matrix";
    let g = "[ 1 1 2 2 3 3 ]";
    let fit = format!("{x} {g} dict mva.lda");
    for (src, want) in [
        (
            format!("{x} [ 1 1 1 1 1 1 ] dict mva.lda"),
            "MVA.LDA: E101 groups is outside mva.lda's domain: one group, where LDA needs two or more",
        ),
        (
            format!("{x} [ 1 2 3 4 5 6 ] dict mva.lda"),
            "MVA.LDA: E101 groups is outside mva.lda's domain: 6 groups among 6 rows, where LDA needs more rows than groups",
        ),
        (
            format!("{x} [ 1 2 ] dict mva.lda"),
            "MVA.LDA: E102 the labels of groups and the rows of X do not agree: 2 and 6",
        ),
        (
            format!("{x} {g} dict \"prior\" [ 0.5 0.5 ] set mva.lda"),
            "MVA.LDA: E101 prior is outside mva.lda's domain: list/2 [0.5, 0.5], of 2 where there are 3 groups",
        ),
        (
            format!("{x} {g} dict \"prior\" [ 0.5 0.5 0.5 ] set mva.lda"),
            "MVA.LDA: E101 prior is outside mva.lda's domain: list/3 [0.5, 0.5, 0.5], whose sum is 1.5, not 1",
        ),
        (
            format!("{x} {g} dict \"prior\" [ 1.5 -0.5 0 ] set mva.lda"),
            "MVA.LDA: E101 prior is outside mva.lda's domain: list/3 [1.5, -0.5, 0], a negative prior",
        ),
        (
            format!("{x} {g} dict \"levels\" [ 1 2 ] set mva.lda"),
            "MVA.LDA: E101 levels is outside mva.lda's domain: list/2 [1, 2], naming 2 labels where groups has 3",
        ),
        (
            format!("{x} {g} dict \"levels\" [ 1 2 2 ] set mva.lda"),
            "MVA.LDA: E101 levels is outside mva.lda's domain: 2, named twice",
        ),
        (
            format!("{x} {g} dict \"levels\" [ 1 2 \"3\" ] set mva.lda"),
            "MVA.LDA: E101 levels is outside mva.lda's domain: \"3\", not a label of groups",
        ),
        (
            format!("{x} {g} dict \"tol\" -1 set mva.lda"),
            "MVA.LDA: E101 tol is outside mva.lda's domain: -1",
        ),
        (
            format!("{x} {g} dict \"method\" \"t\" set mva.lda"),
            "MVA.LDA: E101 opts is outside mva.lda's domain: an unknown key \"method\"",
        ),
        (
            "[ [ 1 2 ] [ 1 2 ] [ 3 4 ] [ 3 4 ] ] matrix [ 1 1 2 2 ] dict mva.lda".into(),
            "MVA.LDA: E101 X is outside mva.lda's domain: variables 1 2, constant within groups",
        ),
        (
            "[ [ 1 2 ] [ 2 3 ] [ 1 2 ] [ 2 3 ] ] matrix [ 1 1 2 2 ] dict mva.lda".into(),
            "MVA.LDA: E101 groups is outside mva.lda's domain: groups whose means are numerically identical",
        ),
        (
            format!("{fit} [ [ 1 2 3 ] ] matrix dict mva.lda.predict"),
            "MVA.LDA.PREDICT: E102 the columns of X and of the fit's means do not agree: 3 and 2",
        ),
        (
            format!("{fit} {x} dict \"dimen\" 0 set mva.lda.predict"),
            "MVA.LDA.PREDICT: E101 dimen is outside mva.lda.predict's domain: 0",
        ),
        (
            format!("{fit} {x} dict \"prior\" [ 1 0 ] set mva.lda.predict"),
            "MVA.LDA.PREDICT: E101 prior is outside mva.lda.predict's domain: list/2 [1, 0], of 2 where there are 3 groups",
        ),
        (
            format!("dict {x} dict mva.lda.predict"),
            "MVA.LDA.PREDICT: E101 fit is outside mva.lda.predict's domain: a MAP whose means is missing",
        ),
        (
            format!("{fit} \"svd\" [ 1 ] set {x} dict mva.lda.predict"),
            "MVA.LDA.PREDICT: E101 fit is outside mva.lda.predict's domain: a MAP whose svd is not a LIST of 2",
        ),
        (
            format!("{fit} \"prior\" [ 0 0 0 ] set {x} dict mva.lda.predict"),
            "MVA.LDA.PREDICT: E101 fit is outside mva.lda.predict's domain: a MAP whose prior does not sum to 1",
        ),
        (
            format!("1 {x} dict mva.lda.predict"),
            "MVA.LDA.PREDICT: E101 fit is outside mva.lda.predict's domain: 1",
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}

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
        "[ [ 1 2 ] [ 2 1 ] [ 3 5 ] [ 4 3 ] ] matrix",
        "[ [ 1.0e307 -1.0e307 ] [ -1.0e307 1.0e307 ] [ 1.0e307 1.0e307 ] [ 1 2 ] ] matrix",
        "[ [ 1.0e160 1 ] [ -1.0e160 2 ] [ 3 4 ] [ 5 5 ] ] matrix",
        "[ [ 1.0e-300 1 ] [ 2.0e-300 2 ] [ 3.0e-300 4 ] [ 5.0e-300 5 ] ] matrix",
    ];
    let seconds = [
        "[ 1 1 2 2 ]",
        "[ 1 2 1 2 ]",
        "[ \"a\" \"a\" \"b\" \"b\" ]",
        "[ na 1 2 2 ]",
        "[ 1 1 2 2 3 ]",
        "list",
        "na",
        "1",
    ];
    let opts = [
        "dict",
        "\"x\"",
        "na",
        "dict \"prior\" [ 0 1 ] set",
        "dict \"prior\" [ float.NaN 1 ] set",
        "dict \"prior\" 1 set",
        "dict \"levels\" [ 2 1 ] set",
        "dict \"levels\" [ \"b\" \"a\" ] set",
        "dict \"tol\" 0 set",
        "dict \"tol\" 1.0e300 set",
    ];
    let popts = [
        "dict",
        "na",
        "dict \"dimen\" 1 set",
        "dict \"prior\" [ 0 1 ] set",
        "dict \"prior\" [ 0.5 0.5 ] set",
    ];
    let fit =
        "[ [ 1 2 ] [ 2 1 ] [ 3 5 ] [ 4 3 ] [ 6 2 ] [ 2 2 ] ] matrix [ 1 1 2 2 3 3 ] dict mva.lda";
    let fits = [
        fit.to_string(),
        format!("{fit} \"prior\" [ 0 0 1 ] set"),
        format!("{fit} \"prior\" [ float.NaN 0 1 ] set"),
        format!("{fit} \"prior\" [ 0 0 0 ] set"),
        format!("{fit} \"means\" [ [ 1.0e307 1.0e307 ] [ -1.0e307 1 ] [ 0 0 ] ] matrix set"),
        format!("{fit} \"scaling\" [ [ 1.0e300 ] [ 1.0e300 ] ] matrix set \"svd\" [ 1 ] set"),
        format!("{fit} \"lev\" list set"),
        format!("{fit} \"N\" 0 set"),
        "dict".to_string(),
        "1".to_string(),
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
            for o in opts {
                ok(&format!("{a} {b} {o} mva.lda"));
                ok(&format!("{a} {b} {o} . mva.lda."));
            }
        }
        for f in &fits {
            for po in popts {
                ok(&format!("{f} {a} {po} mva.lda.predict"));
                ok(&format!("{f} {a} {po} . mva.lda.predict."));
            }
        }
        for w in ["mva.lda", "mva.lda.predict"] {
            ok(&format!("{a} {w}"));
        }
    }
}

// The `.` forms, and a program's own word.

#[test]
fn each_dot_form_answers_on_the_workbench() {
    let x = "[ [ 1 2 ] [ 2 1 ] [ 3 5 ] [ 4 3 ] [ 6 2 ] [ 2 2 ] ] matrix";
    let g = "[ 1 1 2 2 3 3 ]";
    let fit = format!("{x} {g} dict mva.lda");
    for (args, w) in [
        (format!("{x} {g} dict"), "mva.lda"),
        (format!("{fit} {x} dict"), "mva.lda.predict"),
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
    let mut i = run(":mva.lda { 42 } register 1 mva.lda").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}

#[test]
fn an_exact_tie_takes_the_first_group() {
    // Two groups symmetric about 0: a point at 0 under equal priors ties.
    let x = "[ [ -2 1 ] [ -1 -1 ] [ -3 0 ] [ 2 1 ] [ 1 -1 ] [ 3 0 ] ] matrix";
    let m = answer(&format!(
        "{x} [ \"b\" \"b\" \"b\" \"a\" \"a\" \"a\" ] dict mva.lda [ [ 0 0 ] ] matrix dict mva.lda.predict"
    ));
    let post = get(&m, "posterior");
    assert_eq!(post[0], post[1], "{post:?}");
    let class: Vec<String> = m["class"]
        .as_list()
        .unwrap_or(&[])
        .iter()
        .map(BundValue::display)
        .collect();
    assert_eq!(class, ["a"]);
}

#[test]
fn a_near_tie_takes_the_larger_posterior_as_max_col_first_does() {
    // D281: R's default `max.col` draws among posteriors within 1e-5 of
    // the largest; Bund2 answers `"first"`'s strict maximum. Just left of
    // 0 the second group, "b", is larger by far less than 1e-5.
    let x = "[ [ -2 1 ] [ -1 -1 ] [ -3 0 ] [ 2 1 ] [ 1 -1 ] [ 3 0 ] ] matrix";
    let m = answer(&format!(
        "{x} [ \"b\" \"b\" \"b\" \"a\" \"a\" \"a\" ] dict mva.lda [ [ -1.0e-7 0 ] ] matrix dict mva.lda.predict"
    ));
    let post = get(&m, "posterior");
    let gap = post[1] - post[0];
    assert!(gap > 0.0 && gap < 1e-5, "{post:?}");
    let class: Vec<String> = m["class"]
        .as_list()
        .unwrap_or(&[])
        .iter()
        .map(BundValue::display)
        .collect();
    assert_eq!(class, ["b"]);
}
