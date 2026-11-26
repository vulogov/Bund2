//! RFC-0020 criteria 1, 3 and 11, for §S3 and §S4.
//!
//! `mva_r.txt` is R 4.6.2's answers and inputs, written once by `mva.R`;
//! `mva.txt` is the definitional fixture, by mpmath from `mva.py`. No test
//! runs R or Python.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{err, fixture, floats, run};

const R: &str = include_str!("../fixtures/mva_r.txt");
const DEF: &str = include_str!("../fixtures/mva.txt");

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

fn usarrests() -> String {
    source(&fixture(R, "usarrests"), 4)
}

fn square(label: &str) -> (String, usize) {
    let d = fixture(R, label);
    let n = (d.len() as f64).sqrt().round() as usize;
    (source(&d, n), n)
}

fn answer(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

/// A MATRIX's cells row-major, and its column count.
fn cells(v: &BundValue) -> (Vec<f64>, usize) {
    let rows = v
        .as_matrix()
        .unwrap_or_else(|| panic!("not a MATRIX: {v:?}"));
    let cols = rows.first().map_or(0, Vec::len);
    (
        rows.iter()
            .flat_map(|r| r.iter().flat_map(floats))
            .collect(),
        cols,
    )
}

/// |a − b| ≤ 10⁻⁹|b| + 10⁻¹²·max|want|: the criteria's tolerance, its
/// absolute part scaled to the vector, so a value that is 0 by definition
/// (an eigenvalue of the centring's null vector, a loading of 0) compares
/// at the rounding of its neighbours.
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

/// Row-major matrices compared column by column up to each column's sign.
fn near_up_to_sign(what: &str, got: &[f64], want: &[f64], cols: usize) {
    assert_eq!(got.len(), want.len(), "{what}");
    for j in 0..cols {
        let g: Vec<f64> = got.iter().skip(j).step_by(cols).copied().collect();
        let w: Vec<f64> = want.iter().skip(j).step_by(cols).copied().collect();
        let dot: f64 = g.iter().zip(&w).map(|(a, b)| a * b).sum();
        let g: Vec<f64> = if dot < 0.0 {
            g.iter().map(|x| -x).collect()
        } else {
            g
        };
        near(&format!("{what} column {}", j + 1), &g, &w);
    }
}

/// §S1's sign rule holds for each column.
fn signs_ruled(what: &str, xs: &[f64], cols: usize) {
    for j in 0..cols {
        let col: Vec<f64> = xs.iter().skip(j).step_by(cols).copied().collect();
        let mut best = 0;
        for (i, x) in col.iter().enumerate() {
            if x.abs() > col[best].abs() {
                best = i;
            }
        }
        assert!(col[best] >= 0.0, "{what} column {}: {col:?}", j + 1);
    }
}

// Criterion 1.

#[test]
fn every_word_is_registered_with_its_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
        assert!(i.registry.effect_of(&format!("{w}.")).is_some(), "{w}.");
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("mva."))
        .count();
    assert_eq!(ours, 2 * WORDS.len());
}

// Criterion 3: principal components.

#[test]
fn pca_matches_the_definition_and_r_on_usarrests() {
    for (label, opts) in [
        ("cs", "dict"),
        ("cS", "dict \"scale\" true set"),
        ("nS", "dict \"center\" false set \"scale\" true set"),
    ] {
        let m = answer(&format!("{} {opts} mva.pca", usarrests()));
        let at = |k: &str| format!("pca.{label}.{k}");
        let sdev = floats(&m["sdev"]);
        let (rotation, k) = cells(&m["rotation"]);
        let (x, kx) = cells(&m["x"]);
        assert_eq!((k, kx), (4, 4), "{label}");
        near(&at("sdev"), &sdev, &fixture(DEF, &at("sdev")));
        near(&at("rotation"), &rotation, &fixture(DEF, &at("rotation")));
        near(&at("x"), &x, &fixture(DEF, &at("x")));
        near(&at("prop"), &floats(&m["prop"]), &fixture(DEF, &at("prop")));
        near(
            &at("cumprop"),
            &floats(&m["cumprop"]),
            &fixture(DEF, &at("cumprop")),
        );
        signs_ruled(&at("rotation"), &rotation, 4);
        // R's, up to each column's sign.
        near(&at("sdev"), &sdev, &fixture(R, &at("sdev")));
        near_up_to_sign(&at("rotation"), &rotation, &fixture(R, &at("rotation")), 4);
        near_up_to_sign(&at("x"), &x, &fixture(R, &at("x")), 4);
        match label {
            "nS" => assert_eq!(m["center"], BundValue::boolean(false)),
            _ => near(
                &at("center"),
                &floats(&m["center"]),
                &fixture(R, &at("center")),
            ),
        }
        match label {
            "cs" => assert_eq!(m["scale"], BundValue::boolean(false)),
            _ => near(
                &at("scale"),
                &floats(&m["scale"]),
                &fixture(R, &at("scale")),
            ),
        }
    }
}

#[test]
fn rank_and_tol_cut_the_rotation_only() {
    for (label, opts) in [("rank2", "\"rank\" 2 set"), ("tol", "\"tol\" 0.6 set")] {
        let m = answer(&format!(
            "{} dict \"scale\" true set {opts} mva.pca",
            usarrests()
        ));
        let at = |k: &str| format!("pca.{label}.{k}");
        near(&at("sdev"), &floats(&m["sdev"]), &fixture(R, &at("sdev")));
        let (rotation, k) = cells(&m["rotation"]);
        assert_eq!(k, 2, "{label}");
        near_up_to_sign(&at("rotation"), &rotation, &fixture(R, &at("rotation")), 2);
        assert_eq!(cells(&m["x"]).1, 2, "{label}");
        assert_eq!(floats(&m["prop"]).len(), 4, "{label}");
    }
    // A rank above min(n, p) keeps every component, as R's min().
    let m = answer(&format!("{} dict \"rank\" 9 set mva.pca", usarrests()));
    assert_eq!(cells(&m["rotation"]).1, 4);
}

#[test]
fn pca_scales_by_the_column_as_it_then_is() {
    // Uncentred, a constant non-zero column has the root mean square as its
    // scale, so R accepts it; centred, it is 0 and R stops.
    let x = "[ [ 2 1 ] [ 2 3 ] [ 2 4 ] ] matrix";
    let m = answer(&format!(
        "{x} dict \"center\" false set \"scale\" true set mva.pca"
    ));
    near(
        "scale",
        &floats(&m["scale"]),
        &[6.0f64.sqrt(), 13.0f64.sqrt()],
    );
    assert_eq!(
        err(&format!("{x} dict \"scale\" true set mva.pca")),
        "MVA.PCA: E101 X is outside mva.pca's domain: column 1, whose scale is 0"
    );
    assert_eq!(
        err("[ [ 0 1 ] [ 0 3 ] ] matrix dict \"center\" false set \"scale\" true set mva.pca"),
        "MVA.PCA: E101 X is outside mva.pca's domain: column 1, whose scale is 0"
    );
}

// Criterion 3: classical MDS.

#[test]
fn mds_matches_the_definition_and_r_on_eurodist() {
    let (d, _) = square("eurodist");
    let m = answer(&format!("{d} dict mva.mds"));
    let (points, k) = cells(&m["points"]);
    assert_eq!(k, 2);
    assert_eq!(m["k"].as_int(), Some(2));
    near("points", &points, &fixture(DEF, "mds.euro.points"));
    near("eig", &floats(&m["eig"]), &fixture(DEF, "mds.euro.eig"));
    near("gof", &floats(&m["gof"]), &fixture(DEF, "mds.euro.gof"));
    signs_ruled("points", &points, 2);
    near_up_to_sign("points", &points, &fixture(R, "mds.euro.points"), 2);
    near("eig", &floats(&m["eig"]), &fixture(R, "mds.euro.eig"));
    near("gof", &floats(&m["gof"]), &fixture(R, "mds.euro.gof"));
    assert_eq!(floats(&m["ac"]), [0.0]);
}

#[test]
fn mds_with_add_gives_cailliez_constant_through_complex_eigenvalues() {
    let (d, _) = square("manhattan");
    let m = answer(&format!("{d} dict \"add\" true set mva.mds"));
    near("ac", &floats(&m["ac"]), &fixture(R, "mds.add.ac"));
    let (points, k) = cells(&m["points"]);
    assert_eq!(k, 2);
    near_up_to_sign("points", &points, &fixture(R, "mds.add.points"), 2);
    near("eig", &floats(&m["eig"]), &fixture(R, "mds.add.eig"));
    near("gof", &floats(&m["gof"]), &fixture(R, "mds.add.gof"));
}

#[test]
fn fewer_than_k_positive_eigenvalues_keep_only_those() {
    let (d, _) = square("few");
    let m = answer(&format!("{d} dict mva.mds"));
    let want = fixture(R, "mds.few.k");
    assert_eq!(m["k"].as_int(), Some(want[0] as i64));
    let (points, k) = cells(&m["points"]);
    assert_eq!(k as f64, want[0]);
    near_up_to_sign("points", &points, &fixture(R, "mds.few.points"), k);
}

// Refusals.

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            "[ [ 0 1 ] [ 1 0 ] [ 1 1 ] ] matrix dict mva.mds",
            "MVA.MDS: E102 rows and columns do not agree: 3 and 2",
        ),
        (
            "[ [ 0 1 ] [ 2 0 ] ] matrix dict \"k\" 1 set mva.mds",
            "MVA.MDS: E107 the matrix is not symmetric: [1,2] and [2,1] differ by 1",
        ),
        (
            "[ [ 0 1 ] [ 1 0 ] ] matrix dict mva.mds",
            "MVA.MDS: E101 k is outside mva.mds's domain: 2",
        ),
        (
            "[ [ 0 1 1 ] [ 1 0 1 ] [ 1 1 0 ] ] matrix dict \"k\" 3 set mva.mds",
            "MVA.MDS: E101 k is outside mva.mds's domain: 3",
        ),
        (
            "[ [ 0 0 0 ] [ 0 0 0 ] [ 0 0 0 ] ] matrix dict mva.mds",
            "MVA.MDS: E101 D is outside mva.mds's domain: a MATRIX of zero distances",
        ),
        (
            "[ [ 0 1 ] [ 1 0 ] ] matrix dict \"x\" 1 set mva.mds",
            "MVA.MDS: E101 opts is outside mva.mds's domain: an unknown key \"x\"",
        ),
        (
            "[ [ 1 2 ] [ 1 2 ] ] matrix dict mva.pca",
            "MVA.PCA: E101 X is outside mva.pca's domain: a MATRIX with no variance",
        ),
        (
            "[ [ 1 2 ] [ 3 5 ] ] matrix dict \"rank\" 0 set mva.pca",
            "MVA.PCA: E101 rank is outside mva.pca's domain: 0",
        ),
        (
            "[ [ 1 2 ] [ 3 5 ] ] matrix dict \"tol\" -1 set mva.pca",
            "MVA.PCA: E101 tol is outside mva.pca's domain: -1",
        ),
        (
            "[ [ 1 2 ] [ 3 5 ] ] matrix dict \"scale\" 1 set mva.pca",
            "MVA.PCA: E101 scale is outside mva.pca's domain: 1",
        ),
        (
            "list matrix dict mva.pca",
            "MVA.PCA: E101 X is outside mva.pca's domain: an empty MATRIX",
        ),
        (
            "[ [ 1.0e200 1 ] [ -1.0e200 2 ] [ 3 3 ] ] matrix dict \"center\" false set \"scale\" true set mva.pca",
            "MVA.PCA: E114 the answer overflows a FLOAT: a column's scale",
        ),
        (
            "[ [ 0 1.0e160 ] [ 1.0e160 0 ] ] matrix dict \"k\" 1 set mva.mds",
            "MVA.MDS: E114 the answer overflows a FLOAT: a squared distance",
        ),
    ] {
        assert_eq!(err(src), want, "{src}");
    }
}

#[test]
fn the_distance_checks_are_phylos() {
    // `taxa`'s order and texts, with D's name and "objects".
    assert_eq!(
        err("[ [ 0 ] ] matrix dict mva.mds"),
        "MVA.MDS: E101 D is outside mva.mds's domain: 1 objects, where 2 are needed"
    );
    assert_eq!(
        err("[ [ 1 1 ] [ 1 0 ] ] matrix dict mva.mds"),
        "MVA.MDS: E101 D is outside mva.mds's domain: 1 at [1,1]"
    );
    assert_eq!(
        err("[ [ 0 -1 ] [ -1 0 ] ] matrix dict mva.mds"),
        "MVA.MDS: E101 D is outside mva.mds's domain: -1 at [1,2]"
    );
}

// Criterion 11: no abort.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "float.+Inf",
        "-1",
        "0",
        "2",
        "\"x\"",
        "list",
        "dict",
        "[ 1 2 ]",
        "list matrix",
        "na",
        "list [ 1.0 2.0 ] push [ 3.0 ] na push push matrix",
        "[ [ 1 ] ] matrix",
        "[ [ 0 ] ] matrix",
        "[ [ 1 2 ] [ 3 ] ] matrix",
        "[ [ 1 \"a\" ] [ 3 4 ] ] matrix",
        "[ [ 0 1 ] [ 1 0 ] ] matrix",
        "[ [ 0 1 2 ] [ 1 0 1 ] [ 2 1 0 ] ] matrix",
        "[ [ 0 0 ] [ 0 0 ] ] matrix",
        "[ [ 0 1.0e160 3.0e160 ] [ 1.0e160 0 2.0e160 ] [ 3.0e160 2.0e160 0 ] ] matrix",
        "[ [ 1.0e307 -1.0e307 ] [ -1.0e307 1.0e307 ] [ 1.0e307 1.0e307 ] ] matrix",
        "[ [ 1.0e160 1 ] [ 2.0e160 3 ] [ 5 7 ] ] matrix",
    ];
    // The second operand: each option, and a few of the wrong kind.
    let opts = [
        "dict",
        "\"x\"",
        "float.NaN",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
        "dict \"k\" 1 set",
        "dict \"add\" true set",
        "dict \"scale\" true set",
        "dict \"center\" false set \"scale\" true set",
        "dict \"rank\" 1 set \"tol\" 0.5 set",
        "dict \"k\" float.NaN set",
        "dict \"tol\" float.+Inf set",
    ];
    for w in WORDS {
        for a in operands {
            for b in opts {
                for src in [format!("{a} {b} {w}"), format!("{a} {b} . {w}.")] {
                    if let Err(e) = run(&src) {
                        assert!(
                            e.contains(": E1") || e.contains("too shallow"),
                            "{src}: {e}"
                        );
                    }
                }
            }
            let src = format!("{a} {w}");
            if let Err(e) = run(&src) {
                assert!(
                    e.contains(": E1") || e.contains("too shallow"),
                    "{src}: {e}"
                );
            }
        }
    }
}

// The `.` forms, and a program's own word.

#[test]
fn each_dot_form_answers_on_the_workbench() {
    for (x, w) in [
        ("[ [ 1 2 ] [ 3 5 ] [ 4 4 ] ] matrix", "mva.pca"),
        ("[ [ 0 1 2 ] [ 1 0 1 ] [ 2 1 0 ] ] matrix", "mva.mds"),
    ] {
        let plain = run(&format!("{x} dict {w}")).unwrap().pull().unwrap();
        let mut i = run(&format!("7 {x} dict . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        let got = i.pull_workbench().unwrap();
        assert_eq!(format!("{got:?}"), format!("{plain:?}"), "{w}.");
    }
}

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":mva.pca { 42 } register 1 mva.pca").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
