//! RFC-0015 criteria 2, 3, 5, 6 and 12, for §S9.

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{close, err, fixture, floats, run, top};

const FIXTURE: &str = include_str!("../fixtures/linalg.txt");

const A: &str = "[ [ 2 -1 0 ] [ 1 3 2 ] [ 0 1 4 ] ] matrix";
const B: &str = "[ [ 1 2 0 ] [ 0 1 -1 ] [ 3 0 2 ] ] matrix";
const S: &str = "[ [ 4 1 2 ] [ 1 3 0 ] [ 2 0 5 ] ] matrix";
/// Worked example 5's Leslie matrix.
const L: &str = "[ [ 0.0 1.5 0.8 ] [ 0.5 0.0 0.0 ] [ 0.0 0.7 0.0 ] ] matrix";

/// A MATRIX's cells, row by row.
fn cells(v: &BundValue) -> Vec<f64> {
    v.as_matrix()
        .unwrap_or_else(|| panic!("not a MATRIX: {v:?}"))
        .iter()
        .flat_map(|row| row.iter().flat_map(floats))
        .collect()
}

fn pulled(src: &str) -> BundValue {
    run(src)
        .unwrap_or_else(|e| panic!("{src}: {e}"))
        .pull()
        .unwrap()
}

fn map(src: &str) -> std::collections::BTreeMap<String, BundValue> {
    pulled(src).as_map().unwrap().clone()
}

// Criterion 2.

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
        .filter(|n| n.starts_with("linalg."))
        .count();
    assert_eq!(ours, 2 * WORDS.len());
}

// Criterion 12.

#[test]
fn the_3x3_fixtures_exactly() {
    for (label, src) in [
        ("mul", format!("{A} {B} linalg.mul")),
        ("t", format!("{A} linalg.t")),
        ("inv", format!("{A} linalg.inv")),
        ("power", format!("{A} 5 linalg.power")),
    ] {
        close(
            label,
            &cells(&pulled(&src)),
            &fixture(FIXTURE, label),
            1e-15,
        );
    }
    close(
        "det",
        &top(&format!("{A} linalg.det")),
        &fixture(FIXTURE, "det"),
        1e-15,
    );
    close(
        "solve",
        &top(&format!("{A} [ 1 2 3 ] linalg.solve")),
        &fixture(FIXTURE, "solve"),
        1e-15,
    );
    close(
        "mulv",
        &top(&format!("{A} [ 1 -1 2 ] linalg.mulv")),
        &fixture(FIXTURE, "mulv"),
        0.0,
    );
    assert_eq!(
        cells(&pulled("3 linalg.identity")),
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(
        cells(&pulled(&format!("{A} 0 linalg.power"))),
        cells(&pulled("3 linalg.identity"))
    );
}

#[test]
fn a_singular_matrix_is_e106() {
    for w in ["linalg.inv", "linalg.solve"] {
        let src = match w {
            "linalg.solve" => "[ [ 1 2 ] [ 2 4 ] ] matrix [ 1 2 ] linalg.solve".to_string(),
            _ => format!("[ [ 1 2 ] [ 2 4 ] ] matrix {w}"),
        };
        assert_eq!(
            err(&src),
            format!("{}: E106 the matrix is singular", w.to_uppercase())
        );
    }
    assert_eq!(top("[ [ 1 2 ] [ 2 4 ] ] matrix linalg.det"), [0.0]);
}

#[test]
fn eigen_sym_satisfies_av_equals_lambda_v() {
    let m = map(&format!("{S} linalg.eigen.sym"));
    let values = floats(&m["values"]);
    let vectors = cells(&m["vectors"]);
    assert!(values.windows(2).all(|w| w[0] <= w[1]), "{values:?}");
    let a = [[4.0, 1.0, 2.0], [1.0, 3.0, 0.0], [2.0, 0.0, 5.0]];
    for (c, lambda) in values.iter().enumerate() {
        let v: Vec<f64> = (0..3).map(|r| vectors[3 * r + c]).collect();
        for r in 0..3 {
            let av: f64 = (0..3).map(|k| a[r][k] * v[k]).sum();
            assert!((av - lambda * v[r]).abs() <= 1e-10, "λ = {lambda}, row {r}");
        }
    }
}

#[test]
fn worked_example_5() {
    // Real eigenvalues only: the Leslie matrix's subdominant pair is complex.
    let e = err(&format!("{L} linalg.eigenvalues"));
    assert!(
        e.starts_with("LINALG.EIGENVALUES: E108 an eigenvalue is complex: -0.50655170061899"),
        "{e}"
    );
    // Every eigenvalue, by descending modulus, as [ re im ].
    let got = top(&format!("{L} linalg.eigenvalues.complex"));
    let want = fixture(FIXTURE, "leslie.complex");
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        assert!((g - w).abs() <= 1e-12, "{got:?} against {want:?}");
    }
    // The growth rate and the stable stage distribution.
    let m = map(&format!("{L} linalg.dominant"));
    close(
        "value",
        &floats(&m["value"]),
        &fixture(FIXTURE, "leslie.value"),
        1e-12,
    );
    close(
        "vector",
        &floats(&m["vector"]),
        &fixture(FIXTURE, "leslie.vector"),
        1e-12,
    );
    // The RFC's twelve digits.
    let v = floats(&m["vector"]);
    for (g, w) in v
        .iter()
        .zip([0.545096414296, 0.269023089662, 0.185880496042])
    {
        assert!((g - w).abs() < 5e-13, "{v:?}");
    }
}

// Criterion 3.

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            format!("{A} [ [ 1 2 ] [ 3 4 ] ] matrix linalg.mul"),
            "LINALG.MUL: E102 inner dimensions do not agree: 3 and 2".to_string(),
        ),
        (
            "[ [ 1 2 3 ] [ 4 5 6 ] ] matrix linalg.det".into(),
            "LINALG.DET: E102 rows and columns do not agree: 2 and 3".into(),
        ),
        (
            format!("{A} [ 1 2 ] linalg.solve"),
            "LINALG.SOLVE: E102 lengths do not agree: 3 and 2".into(),
        ),
        (
            "0 linalg.identity".into(),
            "LINALG.IDENTITY: E101 n is outside linalg.identity's domain: 0".into(),
        ),
        (
            "10001 linalg.identity".into(),
            "LINALG.IDENTITY: E101 n is outside linalg.identity's domain: 10001".into(),
        ),
        (
            format!("{A} linalg.eigen.sym"),
            "LINALG.EIGEN.SYM: E107 the matrix is not symmetric: [1,2] and [2,1] differ by 2"
                .into(),
        ),
        (
            "[ [ 0 -1 ] [ 1 0 ] ] matrix linalg.dominant".into(),
            "LINALG.DOMINANT: E108 an eigenvalue is complex: 0 ± 1i".into(),
        ),
        (
            "[ [ 1 \"x\" ] [ 3 4 ] ] matrix linalg.t".into(),
            "LINALG.T: E103 A is not a MATRIX of numbers: \"x\"".into(),
        ),
        (
            "[ [ 1 2 ] [ 3 ] ] matrix linalg.t".into(),
            "LINALG.T: E103 A is not a MATRIX of numbers: row 2 has 1 cells, row 1 2".into(),
        ),
        (
            "[ [ 10000000000.0 ] ] matrix 40 linalg.power".into(),
            "LINALG.POWER: E114 the answer overflows a FLOAT: a cell of the power".into(),
        ),
        (
            format!("{A} -1 linalg.power"),
            "LINALG.POWER: E101 k is outside linalg.power's domain: -1".into(),
        ),
        (
            "[ 1 2 ] linalg.t".into(),
            "LINALG.T: E101 A is outside linalg.t's domain: list/2 [1, 2]".into(),
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}

/// An imprimitive matrix: its two eigenvalues, ±1, share the largest
/// modulus. The Schur form answers them to within an ulp or two of ±1.
#[test]
fn a_tie_for_dominance_is_e108() {
    let e = err("[ [ 0 1 ] [ 1 0 ] ] matrix linalg.dominant");
    let (head, values) = e.split_at(e.find(": E108").unwrap());
    assert_eq!(head, "LINALG.DOMINANT");
    let values = values
        .strip_prefix(": E108 the dominant eigenvalue is not unique: ")
        .unwrap_or_else(|| panic!("{e}"));
    let got: Vec<f64> = values.split(", ").map(|v| v.parse().unwrap()).collect();
    close("±1", &got, &[1.0, -1.0], 1e-12);
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
        "[ 1 2 ]",
        "[ [ 1 ] ] matrix",
        "[ [ 0 0 ] [ 0 0 ] ] matrix",
        "[ [ 1 2 3 ] [ 4 5 6 ] ] matrix",
        "[ [ 10.0 308.0 ] [ 1 1 ] ] matrix",
        "[ [ 1 1 ] [ 0 1 ] ] matrix",
    ];
    for w in WORDS {
        for o in operands {
            for src in [format!("{o} {w}"), format!("{o} {o} {w}")] {
                if let Err(e) = run(&src) {
                    assert!(
                        e.contains(": E1") || e.contains("too shallow"),
                        "{src}: {e}"
                    );
                }
            }
        }
    }
}

// The `.` forms.

#[test]
fn each_dot_form_answers_on_the_workbench() {
    for (below, top_, w) in [
        (A, B, "linalg.mul"),
        ("", A, "linalg.det"),
        (A, "3", "linalg.power"),
        (A, "[ 1 -1 2 ]", "linalg.mulv"),
    ] {
        let plain = pulled(&format!("{below} {top_} {w}"));
        let mut i = run(&format!("7 {below} {top_} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        let got = i.pull_workbench().unwrap();
        assert_eq!(format!("{got:?}"), format!("{plain:?}"), "{w}.");
    }
}

// Criterion 6.

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":linalg.t { 42 } register 1 linalg.t").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}

// RFC-0020 criterion 2: `linalg.svd`.

const MVA: &str = include_str!("../fixtures/mva.txt");

/// A MATRIX's rows.
fn rows(v: &BundValue) -> Vec<Vec<f64>> {
    v.as_matrix()
        .unwrap_or_else(|| panic!("not a MATRIX: {v:?}"))
        .iter()
        .map(|row| row.iter().flat_map(floats).collect())
        .collect()
}

#[test]
fn svd_reconstructs_square_tall_wide_and_rank_deficient_inputs() {
    for (label, a) in [
        (
            "svd.square",
            vec![
                vec![2.0, -1.0, 0.0],
                vec![1.0, 3.0, 2.0],
                vec![0.0, 1.0, 4.0],
            ],
        ),
        (
            "svd.tall",
            vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]],
        ),
        (
            "svd.wide",
            vec![vec![1.0, 2.0, 3.0, 4.0], vec![2.0, 0.0, 1.0, -1.0]],
        ),
        (
            "svd.rankdef",
            vec![
                vec![1.0, 2.0, 3.0],
                vec![2.0, 4.0, 6.0],
                vec![1.0, 0.0, 1.0],
            ],
        ),
    ] {
        let (n, p) = (a.len(), a[0].len());
        let r = n.min(p);
        let src = format!(
            "[ {} ] matrix linalg.svd",
            a.iter()
                .map(|row| format!(
                    "[ {} ]",
                    row.iter()
                        .map(|x| format!("{x:?}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                ))
                .collect::<Vec<_>>()
                .join(" ")
        );
        let m = map(&src);
        let d = floats(&m["d"]);
        let u = rows(&m["u"]);
        let v = rows(&m["v"]);
        // §S13 assumption 3: the thin shapes, wide inputs included.
        assert_eq!((u.len(), u[0].len()), (n, r), "{label} u");
        assert_eq!((v.len(), v[0].len()), (p, r), "{label} v");
        let norm = a.iter().flatten().map(|x| x * x).sum::<f64>().sqrt();
        for i in 0..n {
            for j in 0..p {
                let back: f64 = (0..r).map(|k| u[i][k] * d[k] * v[j][k]).sum();
                assert!(
                    (back - a[i][j]).abs() <= 1e-12 * norm,
                    "{label} [{i},{j}]: {back} against {}",
                    a[i][j]
                );
            }
        }
        let want = fixture(MVA, label);
        for (g, w) in d.iter().zip(&want) {
            assert!(
                (g - w).abs() <= 1e-9 * w.abs() + 1e-12 * norm,
                "{label}: {d:?} against {want:?}"
            );
        }
        assert!(d.windows(2).all(|w| w[0] >= w[1]), "{label}: {d:?}");
        for k in 0..r {
            let col: Vec<f64> = v.iter().map(|row| row[k]).collect();
            let mut best = 0;
            for (i, x) in col.iter().enumerate() {
                if x.abs() > col[best].abs() {
                    best = i;
                }
            }
            assert!(col[best] > 0.0, "{label} v column {k}: {col:?}");
        }
    }
}

#[test]
fn svd_answers_only_d_u_and_v() {
    let m = map("[ [ 1 2 ] [ 3 4 ] ] matrix linalg.svd");
    let keys: Vec<&str> = m.keys().map(String::as_str).collect();
    assert_eq!(keys, ["d", "u", "v"]);
}
