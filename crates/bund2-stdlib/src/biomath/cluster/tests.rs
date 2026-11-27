//! RFC-0020 criteria 1, 4 and 11, for §S5.
//!
//! `cluster_r.txt` is R 4.6.2's answers and inputs, written once by
//! `cluster.R`; `cluster.txt` is the definitional fixture for
//! `cluster.dist`, by mpmath from `cluster.py`. No test runs R or Python.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::{METHODS, WORDS};
use crate::biomath::testkit::{err, fixture, floats, run, run_on};

const R: &str = include_str!("../fixtures/cluster_r.txt");
const DEF: &str = include_str!("../fixtures/cluster.txt");

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

/// A square fixture matrix as source, and its order.
fn square(label: &str) -> (String, usize) {
    let d = fixture(R, label);
    let n = (d.len() as f64).sqrt().round() as usize;
    (source(&d, n), n)
}

fn top(src: &str) -> BundValue {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    i.pull().unwrap_or_else(|| panic!("{src}: empty"))
}

fn map(v: &BundValue) -> BTreeMap<String, BundValue> {
    v.as_map()
        .unwrap_or_else(|| panic!("not a MAP: {v:?}"))
        .clone()
}

/// A MATRIX's cells row-major.
fn cells(v: &BundValue) -> Vec<f64> {
    v.as_matrix()
        .unwrap_or_else(|| panic!("not a MATRIX: {v:?}"))
        .iter()
        .flat_map(|r| r.iter().flat_map(floats))
        .collect()
}

/// A LIST or MATRIX of INTEGERs, row-major, exactly.
fn ints(v: &BundValue) -> Vec<i64> {
    let items: Vec<BundValue> = match v.as_matrix() {
        Some(rows) => rows.iter().flatten().cloned().collect(),
        None => v
            .as_list()
            .unwrap_or_else(|| panic!("not a LIST: {v:?}"))
            .to_vec(),
    };
    items
        .iter()
        .map(|x| {
            x.as_int()
                .unwrap_or_else(|| panic!("not an INTEGER: {x:?}"))
        })
        .collect()
}

fn as_ints(xs: &[f64]) -> Vec<i64> {
    xs.iter().map(|x| *x as i64).collect()
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

const INPUTS: [&str; 3] = ["tied", "untied", "inversion"];

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
        .filter(|n| n.starts_with("cluster."))
        .count();
    assert_eq!(ours, 2 * WORDS.len());
}

// Criterion 4: hierarchical clustering.

#[test]
fn hclust_equals_r_for_each_method_on_tied_untied_and_inverted_inputs() {
    for input in INPUTS {
        let (d, _) = square(input);
        for m in METHODS {
            let what = format!("{input} {m}");
            let a = map(&top(&format!(
                "{d} dict \"method\" \"{m}\" set cluster.hclust"
            )));
            assert_eq!(
                ints(&a["merge"]),
                as_ints(&fixture(R, &format!("hc.{input}.{m}.merge"))),
                "{what} merge"
            );
            assert_eq!(
                ints(&a["order"]),
                as_ints(&fixture(R, &format!("hc.{input}.{m}.order"))),
                "{what} order"
            );
            near(
                &format!("{what} height"),
                &floats(&a["height"]),
                &fixture(R, &format!("hc.{input}.{m}.height")),
            );
            assert_eq!(a["method"].as_str().as_deref(), Some(m), "{what}");
        }
    }
}

#[test]
fn the_default_method_is_complete_and_the_tie_order_is_rs() {
    let (d, _) = square("tied");
    let a = map(&top(&format!("{d} dict cluster.hclust")));
    assert_eq!(a["method"].as_str().as_deref(), Some("complete"));
    // §What R does: single linkage merges (−1, −2), (−3, 1), (−4, 2).
    let s = map(&top(&format!(
        "{d} dict \"method\" \"single\" set cluster.hclust"
    )));
    assert_eq!(ints(&s["merge"])[..6], [-1, -2, -3, 1, -4, 2]);
}

#[test]
fn members_weigh_the_merges_as_rs() {
    let (d, _) = square("untied");
    let members: Vec<String> = fixture(R, "members").iter().map(|x| lit(*x)).collect();
    let members = format!("[ {} ]", members.join(" "));
    for m in METHODS {
        let a = map(&top(&format!(
            "{d} dict \"method\" \"{m}\" set \"members\" {members} set cluster.hclust"
        )));
        assert_eq!(
            ints(&a["merge"]),
            as_ints(&fixture(R, &format!("hc.members.{m}.merge"))),
            "{m} merge"
        );
        assert_eq!(
            ints(&a["order"]),
            as_ints(&fixture(R, &format!("hc.members.{m}.order"))),
            "{m} order"
        );
        near(
            &format!("{m} height"),
            &floats(&a["height"]),
            &fixture(R, &format!("hc.members.{m}.height")),
        );
    }
}

#[test]
fn cut_and_cut_h_equal_cutree() {
    for input in INPUTS {
        let (d, n) = square(input);
        for m in METHODS {
            let what = format!("{input} {m}");
            let tree = top(&format!("{d} dict \"method\" \"{m}\" set cluster.hclust"));
            let by_k = as_ints(&fixture(R, &format!("cut.{input}.{m}.k")));
            for k in 1..=n {
                let mut i = run_on(
                    std::slice::from_ref(&tree),
                    None,
                    &format!("{k} cluster.cut"),
                )
                .unwrap_or_else(|e| panic!("{what} k {k}: {e}"));
                let got = ints(&i.pull().unwrap_or_else(|| panic!("{what}: empty")));
                let want: Vec<i64> = (0..n).map(|obs| by_k[obs * n + k - 1]).collect();
                assert_eq!(got, want, "{what} k {k}");
            }
            let key = format!("cut.{input}.{m}.hs");
            if !R.lines().any(|l| l.starts_with(&format!("{key} "))) {
                // An inversion: R stops, and so does cluster.cut.h.
                let e = run_on(std::slice::from_ref(&tree), None, "1.0 cluster.cut.h")
                    .err()
                    .unwrap_or_else(|| panic!("{what}: an inversion was cut"));
                assert!(
                    e.starts_with("CLUSTER.CUT.H: E101 tree is outside cluster.cut.h's domain: a height that falls at step"),
                    "{what}: {e}"
                );
                continue;
            }
            let hs = fixture(R, &key);
            let by_h = as_ints(&fixture(R, &format!("cut.{input}.{m}.h")));
            // A cut "at" a merge is at the tree's own height. R's Fortran
            // fuses multiply-adds on the owner's arm64 (Q69), so a Ward
            // height can differ from R's in the last place, within
            // criterion 4's tolerance; R's cut at R's height is compared
            // with Bund2's cut at Bund2's.
            let r_heights = fixture(R, &format!("hc.{input}.{m}.height"));
            let own = floats(&map(&tree)["height"]);
            let hs: Vec<f64> = hs
                .iter()
                .map(|h| r_heights.iter().position(|r| r == h).map_or(*h, |i| own[i]))
                .collect();
            for (c, h) in hs.iter().enumerate() {
                let mut i = run_on(
                    std::slice::from_ref(&tree),
                    None,
                    &format!("{} cluster.cut.h", lit(*h)),
                )
                .unwrap_or_else(|e| panic!("{what} h {h}: {e}"));
                let got = ints(&i.pull().unwrap_or_else(|| panic!("{what}: empty")));
                let want: Vec<i64> = (0..n).map(|obs| by_h[obs * hs.len() + c]).collect();
                assert_eq!(got, want, "{what} h {h}");
            }
        }
    }
}

#[test]
fn the_fixture_has_an_inversion_under_centroid() {
    // Criterion 4's inversion is in the input, not only assumed.
    let h = fixture(R, "hc.inversion.centroid.height");
    assert!(h.windows(2).any(|w| w[1] < w[0]), "{h:?}");
    assert!(!R.contains("cut.inversion.centroid.hs "));
}

// Criterion 4: distances.

#[test]
fn dist_matches_the_definition_and_r_for_each_method() {
    let x = source(&fixture(R, "x"), 3);
    let xc = source(&fixture(R, "xc"), 3);
    for (label, input, opts) in [
        ("dist.euclidean", &x, "dict"),
        ("dist.euclidean", &x, "dict \"method\" \"euclidean\" set"),
        ("dist.maximum", &x, "dict \"method\" \"maximum\" set"),
        ("dist.manhattan", &x, "dict \"method\" \"manhattan\" set"),
        ("dist.binary", &x, "dict \"method\" \"binary\" set"),
        (
            "dist.minkowski.3",
            &x,
            "dict \"method\" \"minkowski\" set \"p\" 3 set",
        ),
        (
            "dist.minkowski.0.5",
            &x,
            "dict \"method\" \"minkowski\" set \"p\" 0.5 set",
        ),
        ("dist.canberra", &xc, "dict \"method\" \"canberra\" set"),
    ] {
        let got = cells(&top(&format!("{input} {opts} cluster.dist")));
        near(
            &format!("{label} against the definition"),
            &got,
            &fixture(DEF, label),
        );
        near(&format!("{label} against R"), &got, &fixture(R, label));
    }
}

#[test]
fn canberra_rescales_and_binary_answers_zero_for_zero_rows() {
    let pair = cells(&top(
        "[ [ 0 1 ] [ 0 3 ] ] matrix dict \"method\" \"canberra\" set cluster.dist",
    ));
    assert_eq!(pair, [0.0, 1.0, 1.0, 0.0]);
    assert_eq!(fixture(R, "canberra.pair"), [1.0]);
    let zero = cells(&top(
        "[ [ 0 0 ] [ 0 0 ] ] matrix dict \"method\" \"binary\" set cluster.dist",
    ));
    assert_eq!(zero, [0.0; 4]);
}

#[test]
fn the_distances_feed_hclust() {
    // cluster.dist's answer is cluster.hclust's operand, as R's dist is
    // hclust's.
    let a = map(&top(
        "[ [ 0 ] [ 1 ] [ 2 ] [ 3 ] [ 10 ] [ 11 ] ] matrix dict cluster.dist \
         dict \"method\" \"single\" set cluster.hclust",
    ));
    assert_eq!(
        ints(&a["merge"]),
        as_ints(&fixture(R, "hc.tied.single.merge"))
    );
}

// Refusals.

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            "[ [ 0 0 ] [ 0 0 ] [ 1 2 ] ] matrix dict \"method\" \"canberra\" set cluster.dist",
            "CLUSTER.DIST: E101 X is outside cluster.dist's domain: rows 1 and 2, whose Canberra distance has no term",
        ),
        (
            "[ [ 1.0e200 ] [ -1.0e200 ] ] matrix dict cluster.dist",
            "CLUSTER.DIST: E114 the answer overflows a FLOAT: the distance between rows 1 and 2",
        ),
        (
            "[ [ 1 ] [ 2 ] ] matrix dict \"method\" \"Euclidean\" set cluster.dist",
            "CLUSTER.DIST: E101 method is outside cluster.dist's domain: \"Euclidean\"",
        ),
        (
            "[ [ 1 ] [ 2 ] ] matrix dict \"p\" 0 set cluster.dist",
            "CLUSTER.DIST: E101 p is outside cluster.dist's domain: 0",
        ),
        (
            "[ [ 1 ] [ 2 ] ] matrix dict \"q\" 1 set cluster.dist",
            "CLUSTER.DIST: E101 opts is outside cluster.dist's domain: an unknown key \"q\"",
        ),
        (
            "list matrix dict cluster.dist",
            "CLUSTER.DIST: E101 X is outside cluster.dist's domain: an empty MATRIX",
        ),
        (
            "[ [ 0 1.0e300 ] [ 1.0e300 0 ] ] matrix dict cluster.hclust",
            "CLUSTER.HCLUST: E101 D is outside cluster.hclust's domain: 1e300 at [1,2]",
        ),
        (
            "[ [ 0 1.0e160 2.0e160 ] [ 1.0e160 0 3.0e160 ] [ 2.0e160 3.0e160 0 ] ] matrix \
             dict \"method\" \"ward.D2\" set cluster.hclust",
            "CLUSTER.HCLUST: E101 D is outside cluster.hclust's domain: 1e160 at [1,2]",
        ),
        (
            "[ [ 0 1 ] [ 1 0 ] ] matrix dict \"method\" \"ward\" set cluster.hclust",
            "CLUSTER.HCLUST: E101 method is outside cluster.hclust's domain: \"ward\"",
        ),
        (
            "[ [ 0 1 ] [ 1 0 ] ] matrix dict \"members\" [ 1 ] set cluster.hclust",
            "CLUSTER.HCLUST: E102 members and objects do not agree: 1 and 2",
        ),
        (
            "[ [ 0 1 ] [ 1 0 ] ] matrix dict \"members\" [ 1 0 ] set cluster.hclust",
            "CLUSTER.HCLUST: E101 members is outside cluster.hclust's domain: 0 at 2",
        ),
        (
            "[ [ 0 ] ] matrix dict cluster.hclust",
            "CLUSTER.HCLUST: E101 D is outside cluster.hclust's domain: 1 objects, where 2 are needed",
        ),
        (
            "[ [ 0 1 ] [ 1 0 ] ] matrix dict cluster.hclust 3 cluster.cut",
            "CLUSTER.CUT: E101 k is outside cluster.cut's domain: 3",
        ),
        (
            "[ [ 0 1 ] [ 1 0 ] ] matrix dict cluster.hclust 0 cluster.cut",
            "CLUSTER.CUT: E101 k is outside cluster.cut's domain: 0",
        ),
        (
            "[ [ 0 1 ] [ 1 0 ] ] matrix dict cluster.hclust float.NaN cluster.cut.h",
            "CLUSTER.CUT.H: E101 h is outside cluster.cut.h's domain: NaN",
        ),
        (
            "dict 1 cluster.cut",
            "CLUSTER.CUT: E101 tree is outside cluster.cut's domain: a MAP without merge",
        ),
        (
            "dict \"merge\" [ [ -1 -2 ] [ -1 1 ] ] matrix set \"height\" [ 1 2 ] set 1 cluster.cut",
            "CLUSTER.CUT: E101 tree is outside cluster.cut's domain: -1 in merge row 2",
        ),
        (
            "dict \"merge\" [ [ -1 1 ] ] matrix set \"height\" [ 1 ] set 1 cluster.cut",
            "CLUSTER.CUT: E101 tree is outside cluster.cut's domain: 1 in merge row 1",
        ),
        (
            "dict \"merge\" [ [ -1 -2 ] ] matrix set \"height\" [ 1 2 ] set 1 cluster.cut",
            "CLUSTER.CUT: E101 tree is outside cluster.cut's domain: 2 heights for 1 merges",
        ),
        (
            "dict \"merge\" [ [ -1 -3 ] ] matrix set \"height\" [ 1 ] set 1 cluster.cut",
            "CLUSTER.CUT: E101 tree is outside cluster.cut's domain: -3 in merge row 1",
        ),
        (
            "dict \"merge\" [ [ -1 -2 ] [ -3 1 ] ] matrix set \"height\" [ 2 1 ] set 1.5 cluster.cut.h",
            "CLUSTER.CUT.H: E101 tree is outside cluster.cut.h's domain: a height that falls at step 2, an inversion",
        ),
    ] {
        assert_eq!(err(src), want, "{src}");
    }
}

#[test]
fn an_inverted_tree_cuts_by_k_as_rs() {
    // R's cutree by k does not check the heights (`cutree.R:35-40`).
    let mut i = run(
        "dict \"merge\" [ [ -1 -2 ] [ -3 1 ] ] matrix set \"height\" [ 2 1 ] set 2 cluster.cut",
    )
    .unwrap();
    assert_eq!(ints(&i.pull().unwrap()), [1, 1, 2]);
}

#[test]
fn the_distance_checks_are_phylos() {
    assert_eq!(
        err("[ [ 0 1 ] [ 2 0 ] ] matrix dict cluster.hclust"),
        "CLUSTER.HCLUST: E107 the matrix is not symmetric: [1,2] and [2,1] differ by 1"
    );
    assert_eq!(
        err("[ [ 0 -1 ] [ -1 0 ] ] matrix dict cluster.hclust"),
        "CLUSTER.HCLUST: E101 D is outside cluster.hclust's domain: -1 at [1,2]"
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
        "[ [ 0 1 2 ] [ 1 0 1 ] [ 2 1 0 ] ] matrix",
        "[ [ 0 0 ] [ 0 0 ] ] matrix",
        "[ [ 0 1.0e160 3.0e160 ] [ 1.0e160 0 2.0e160 ] [ 3.0e160 2.0e160 0 ] ] matrix",
        "[ [ 0 1.0e299 1.0e299 ] [ 1.0e299 0 1.0e299 ] [ 1.0e299 1.0e299 0 ] ] matrix",
        "[ [ 1.0e307 -1.0e307 ] [ -1.0e307 1.0e307 ] [ 1.0e307 1.0e307 ] ] matrix",
        "dict \"merge\" [ [ -1 -2 ] ] matrix set \"height\" [ 1 ] set",
        "dict \"merge\" [ [ -1 -2 ] ] matrix set \"height\" [ float.NaN ] set",
        "dict \"merge\" [ [ -1 1.5 ] ] matrix set \"height\" [ 1 ] set",
        "dict \"merge\" list matrix set \"height\" list set",
        "dict \"merge\" [ [ -1 -2 3 ] ] matrix set \"height\" [ 1 ] set",
    ];
    // The second operand: each option, k and h, and a few of the wrong kind.
    let second = [
        "dict",
        "\"x\"",
        "float.NaN",
        "float.-Inf",
        "1",
        "0.5",
        "na",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
        "dict \"method\" \"ward.D\" set",
        "dict \"method\" \"ward.D2\" set",
        "dict \"method\" \"centroid\" set \"members\" [ 1.0e308 1.0e308 1.0e308 ] set",
        "dict \"method\" \"canberra\" set",
        "dict \"method\" \"minkowski\" set \"p\" 1.0e-300 set",
        "dict \"method\" 1 set",
        "dict \"members\" [ float.NaN 1 1 ] set",
    ];
    for w in WORDS {
        for a in operands {
            for b in second {
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
    let tree = "[ [ 0 1 2 ] [ 1 0 1 ] [ 2 1 0 ] ] matrix dict cluster.hclust";
    for (x, y, w) in [
        ("[ [ 1 2 ] [ 3 5 ] [ 4 4 ] ] matrix", "dict", "cluster.dist"),
        (
            "[ [ 0 1 2 ] [ 1 0 1 ] [ 2 1 0 ] ] matrix",
            "dict",
            "cluster.hclust",
        ),
        (tree, "2", "cluster.cut"),
        (tree, "1.5", "cluster.cut.h"),
    ] {
        let plain = run(&format!("{x} {y} {w}")).unwrap().pull().unwrap();
        let mut i = run(&format!("7 {x} {y} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        let got = i.pull_workbench().unwrap();
        assert_eq!(format!("{got:?}"), format!("{plain:?}"), "{w}.");
    }
}

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":cluster.dist { 42 } register 1 cluster.dist").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}

// Criterion 5: k-means (M3).

const KM: &str = include_str!("../fixtures/kmeans_r.txt");

/// `kmeans_r.txt`'s inputs, each with its column count.
const KM_INPUTS: [(&str, usize); 3] = [("tied", 2), ("usarrests", 4), ("sim", 2)];

/// R's algorithm names and Bund2's.
const KM_ALGORITHMS: [(&str, &str); 3] = [
    ("Hartigan-Wong", "hartigan-wong"),
    ("Lloyd", "lloyd"),
    ("MacQueen", "macqueen"),
];

fn kmeans(x: &str, centers: &str, opts: &str) -> BTreeMap<String, BundValue> {
    map(&top(&format!("{x} {centers} {opts} cluster.kmeans")))
}

fn one(v: &BundValue) -> f64 {
    match v.unboxed() {
        BundValue::Float(x, _) => *x,
        _ => panic!("not a FLOAT: {v:?}"),
    }
}

#[test]
fn kmeans_equals_r_for_each_algorithm_given_centres() {
    for (name, p) in KM_INPUTS {
        let x = source(&fixture(KM, &format!("{name}.x")), p);
        let c = source(&fixture(KM, &format!("{name}.centres")), p);
        for (r, ours) in KM_ALGORITHMS {
            let l = format!("{name}.{r}");
            let z = kmeans(&x, &c, &format!("dict \"algorithm\" \"{ours}\" set"));
            assert_eq!(
                ints(&z["cluster"]),
                as_ints(&fixture(KM, &format!("{l}.cluster"))),
                "{l}"
            );
            assert_eq!(
                ints(&z["size"]),
                as_ints(&fixture(KM, &format!("{l}.size"))),
                "{l}"
            );
            assert_eq!(
                vec![z["iter"].as_int().unwrap()],
                as_ints(&fixture(KM, &format!("{l}.iter"))),
                "{l}"
            );
            near(
                &format!("{l}.centers"),
                &cells(&z["centers"]),
                &fixture(KM, &format!("{l}.centers")),
            );
            let wss = fixture(KM, &format!("{l}.withinss"));
            near(&format!("{l}.withinss"), &floats(&z["withinss"]), &wss);
            let totss = fixture(KM, &format!("{l}.totss"));
            near(&format!("{l}.totss"), &[one(&z["totss"])], &totss);
            let tot: f64 = wss.iter().sum();
            near(&format!("{l}.tot"), &[one(&z["tot.withinss"])], &[tot]);
            near(
                &format!("{l}.betweenss"),
                &[one(&z["betweenss"])],
                &[totss[0] - tot],
            );
        }
    }
}

#[test]
fn the_fixture_takes_more_than_one_pass_and_has_a_tie() {
    for (name, _) in KM_INPUTS {
        for (r, _) in KM_ALGORITHMS {
            assert!(fixture(KM, &format!("{name}.{r}.iter"))[0] >= 2.0);
            assert!(
                !KM.contains(&format!("{name}.{r}.warn")),
                "{name}.{r} warned in R"
            );
        }
    }
    // (2, 0) is as near (1, 0) as (3, 0), and R puts it with the first.
    let z = kmeans(
        &source(&fixture(KM, "tied.x"), 2),
        &source(&fixture(KM, "tied.centres"), 2),
        "dict \"algorithm\" \"lloyd\" set",
    );
    assert_eq!(ints(&z["cluster"])[2], 1);
}

#[test]
fn the_default_is_hartigan_wong_with_ten_iterations() {
    let x = source(&fixture(KM, "usarrests.x"), 4);
    let c = source(&fixture(KM, "usarrests.centres"), 4);
    let plain = kmeans(&x, &c, "dict");
    let named = kmeans(
        &x,
        &c,
        "dict \"algorithm\" \"hartigan-wong\" set \"iter.max\" 10 set",
    );
    assert_eq!(format!("{plain:?}"), format!("{named:?}"));
}

#[test]
fn big_is_fortrans_single_precision_1e30() {
    assert_eq!(super::kmeans::BIG, f64::from(1.0e30_f32));
}

/// X with duplicate rows: its distinct rows, in first-occurrence order,
/// are rows 1, 3, 4, 6 and 7.
const DUPS: &str = "[ [ 0 0 ] [ 0 0 ] [ 1 0 ] [ 5 0 ] [ 1 0 ] [ 6 0 ] [ 7 0 ] [ 6 0 ] ] matrix";
const DISTINCT: [[f64; 2]; 5] = [[0.0, 0.0], [1.0, 0.0], [5.0, 0.0], [6.0, 0.0], [7.0, 0.0]];

/// The centres `rng.choice` draws from the distinct rows' indices.
fn drawn(i: &mut bund2_interp::Interp) -> String {
    let picked = ints(&i.pull().unwrap());
    let rows: Vec<f64> = picked.iter().flat_map(|j| DISTINCT[*j as usize]).collect();
    source(&rows, 2)
}

#[test]
fn an_integer_k_draws_distinct_rows_by_rng_choice_and_repeats() {
    for (_, alg) in KM_ALGORITHMS {
        let opts = format!("dict \"algorithm\" \"{alg}\" set");
        let src = format!("7 rng.seed {DUPS} 2 {opts} cluster.kmeans");
        let a = top(&src);
        let b = top(&src);
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "{alg}");
        let mut i = run("7 rng.seed [ 0 1 2 3 4 ] 2 rng.choice").unwrap();
        let given = kmeans(DUPS, &drawn(&mut i), &opts);
        assert_eq!(format!("{:?}", map(&a)), format!("{given:?}"), "{alg}");
    }
}

#[test]
fn nstart_keeps_the_least_total_the_earliest_on_a_tie() {
    let opts = "dict \"algorithm\" \"lloyd\" set";
    let got = kmeans(
        &format!("7 rng.seed {DUPS}"),
        "2",
        &format!("{opts} \"nstart\" 4 set"),
    );
    let mut i = run(
        "7 rng.seed [ 0 1 2 3 4 ] 2 rng.choice [ 0 1 2 3 4 ] 2 rng.choice \
         [ 0 1 2 3 4 ] 2 rng.choice [ 0 1 2 3 4 ] 2 rng.choice",
    )
    .unwrap();
    let mut starts: Vec<String> = (0..4).map(|_| drawn(&mut i)).collect();
    starts.reverse();
    let mut best: Option<(f64, BTreeMap<String, BundValue>)> = None;
    for c in &starts {
        let z = kmeans(DUPS, c, opts);
        let t = one(&z["tot.withinss"]);
        if best.as_ref().is_none_or(|(b, _)| t < *b) {
            best = Some((t, z));
        }
    }
    assert_eq!(format!("{got:?}"), format!("{:?}", best.unwrap().1));
}

#[test]
fn nstart_is_ignored_with_a_matrix_of_centres_but_checked() {
    let x = source(&fixture(KM, "sim.x"), 2);
    let c = source(&fixture(KM, "sim.centres"), 2);
    let one_start = kmeans(&x, &c, "dict");
    let five = kmeans(&x, &c, "dict \"nstart\" 5 set");
    assert_eq!(format!("{one_start:?}"), format!("{five:?}"));
    let e = err(&format!("{x} {c} dict \"nstart\" 0 set cluster.kmeans"));
    assert!(e.contains(": E101 nstart is outside"), "{e}");
}

#[test]
fn k_of_one_answers_the_mean_with_iter_one() {
    for (_, alg) in KM_ALGORITHMS {
        for centers in ["1", "[ [ 9 9 ] ] matrix"] {
            let z = kmeans(
                "[ [ 1 2 ] [ 3 6 ] [ 5 1 ] ] matrix",
                centers,
                &format!("dict \"algorithm\" \"{alg}\" set"),
            );
            assert_eq!(ints(&z["cluster"]), [1, 1, 1]);
            assert_eq!(vec![z["iter"].as_int().unwrap()], [1]);
            near("centers", &cells(&z["centers"]), &[3.0, 3.0]);
            near("withinss", &floats(&z["withinss"]), &[8.0 + 14.0]);
            near("betweenss", &[one(&z["betweenss"])], &[0.0]);
        }
    }
}

#[test]
fn an_empty_cluster_is_e101_and_lloyd_at_one_iteration_e110() {
    for (_, alg) in KM_ALGORITHMS {
        let e = err(&format!(
            "[ [ 0 ] [ 1 ] [ 10 ] [ 11 ] ] matrix [ [ 0 ] [ 1 ] [ 100 ] ] matrix \
             dict \"algorithm\" \"{alg}\" set cluster.kmeans"
        ));
        assert!(
            e.starts_with("CLUSTER.KMEANS: E101 centers is outside cluster.kmeans's domain: a start that leaves"),
            "{alg}: {e}"
        );
    }
    let x = source(&fixture(KM, "tied.x"), 2);
    let c = source(&fixture(KM, "tied.centres"), 2);
    assert_eq!(
        err(&format!(
            "{x} {c} dict \"algorithm\" \"lloyd\" set \"iter.max\" 1 set cluster.kmeans"
        )),
        "CLUSTER.KMEANS: E110 Lloyd k-means did not converge within 1 iterations"
    );
    // R's Lloyd took three passes here, so two is still short.
    assert!(
        err(&format!(
            "{x} {c} dict \"algorithm\" \"lloyd\" set \"iter.max\" 2 set cluster.kmeans"
        ))
        .contains(": E110 ")
    );
    let e = err(&format!(
        "{} {} dict \"iter.max\" 1 set cluster.kmeans",
        source(&fixture(KM, "usarrests.x"), 4),
        source(&fixture(KM, "usarrests.centres"), 4),
    ));
    assert_eq!(
        e,
        "CLUSTER.KMEANS: E110 Hartigan-Wong k-means did not converge within 1 iterations"
    );
}

#[test]
fn each_kmeans_refusal_is_its_template() {
    let x = "[ [ 1 2 ] [ 3 4 ] [ 5 7 ] ] matrix";
    for (src, want) in [
        (
            format!("{x} [ [ 1 2 ] [ 1 2 ] ] matrix dict cluster.kmeans"),
            "CLUSTER.KMEANS: E101 centers is outside cluster.kmeans's domain: rows 1 and 2 are equal",
        ),
        (
            format!("{x} [ [ 1 ] [ 2 ] [ 3 ] [ 4 ] ] matrix dict cluster.kmeans"),
            "CLUSTER.KMEANS: E101 centers is outside cluster.kmeans's domain: 4 rows, above X's 3",
        ),
        (
            format!("{x} [ [ 1 2 3 ] ] matrix dict cluster.kmeans"),
            "CLUSTER.KMEANS: E102 the columns of X and centers do not agree: 2 and 3",
        ),
        (
            format!("{DUPS} 6 dict cluster.kmeans"),
            "CLUSTER.KMEANS: E101 centers is outside cluster.kmeans's domain: 6, above X's 5 distinct rows",
        ),
        (
            format!("{x} 0 dict cluster.kmeans"),
            "CLUSTER.KMEANS: E101 centers is outside cluster.kmeans's domain: 0",
        ),
        (
            format!("{x} 2.0 dict cluster.kmeans"),
            "CLUSTER.KMEANS: E101 centers is outside cluster.kmeans's domain: 2.0",
        ),
        (
            format!("{x} 3 dict cluster.kmeans"),
            "CLUSTER.KMEANS: E101 centers is outside cluster.kmeans's domain: 3 centres for 3 rows, which Hartigan-Wong refuses",
        ),
        (
            format!("{x} 2 dict \"algorithm\" \"Lloyd\" set cluster.kmeans"),
            "CLUSTER.KMEANS: E101 algorithm is outside cluster.kmeans's domain: \"Lloyd\"",
        ),
        (
            format!("{x} 2 dict \"iter.max\" 0 set cluster.kmeans"),
            "CLUSTER.KMEANS: E101 iter.max is outside cluster.kmeans's domain: 0",
        ),
        (
            format!("{x} 2 dict \"centers\" 2 set cluster.kmeans"),
            "CLUSTER.KMEANS: E101 opts is outside cluster.kmeans's domain: an unknown key \"centers\"",
        ),
        (
            format!("{x} 2 dict \"iter.max\" 1000000000 set cluster.kmeans"),
            "CLUSTER.KMEANS: E111 n·k·p·iter.max·nstart = 12000000000 exceeds its cap of 10000000000",
        ),
        (
            format!(
                "{x} [ [ 1 2 ] ] matrix dict \"nstart\" 1000000000 set \"iter.max\" 1000 set cluster.kmeans"
            ),
            "",
        ),
        (
            "[ [ 1.0e308 0 ] [ 1.0e308 1 ] [ 0 1 ] ] matrix 2 dict cluster.kmeans".into(),
            "CLUSTER.KMEANS: E114 the answer overflows a FLOAT: the column sums of X",
        ),
        (
            "[ [ 1.0e160 0 ] [ -1.0e160 1 ] [ 0 1 ] ] matrix 2 dict cluster.kmeans".into(),
            "CLUSTER.KMEANS: E114 the answer overflows a FLOAT: the sums of squares of X",
        ),
        (
            format!("{x} [ [ 1.0e160 0 ] ] matrix dict cluster.kmeans"),
            "CLUSTER.KMEANS: E114 the answer overflows a FLOAT: the sums of squares of X",
        ),
    ] {
        if want.is_empty() {
            // nstart counts 1 with a MATRIX of centres: no cap.
            assert!(run(&src).is_ok(), "{src}");
        } else {
            assert_eq!(err(&src), want, "{src}");
        }
    }
}

#[test]
fn kmeans_no_operand_aborts() {
    let xs = [
        "float.NaN",
        "list",
        "dict",
        "list matrix",
        "[ [ 1 ] ] matrix",
        "[ [ 1 2 ] [ 3 ] ] matrix",
        "[ [ 1 \"a\" ] [ 3 4 ] ] matrix",
        "[ [ 1 float.NaN ] [ 3 4 ] ] matrix",
        "[ [ 0 0 ] [ 0 0 ] [ 0 0 ] ] matrix",
        "[ [ 1 2 ] [ 3 4 ] [ 5 6 ] [ 1 2 ] ] matrix",
        "[ [ 1.0e307 -1.0e307 ] [ -1.0e307 1.0e307 ] [ 1.0e307 1.0e307 ] ] matrix",
        "[ [ 1.0e150 1 ] [ -1.0e150 2 ] [ 3 4 ] ] matrix",
    ];
    let centres = [
        "1",
        "2",
        "3",
        "-1",
        "na",
        "float.+Inf",
        "\"x\"",
        "list",
        "list matrix",
        "[ [ 0 0 ] ] matrix",
        "[ [ 0 0 ] [ 1 1 ] ] matrix",
        "[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ] matrix",
        "[ [ float.NaN 0 ] [ 1 1 ] ] matrix",
        "[ [ 1.0e307 0 ] [ 1 1 ] ] matrix",
    ];
    let opts = [
        "dict",
        "\"x\"",
        "na",
        "dict \"algorithm\" \"lloyd\" set",
        "dict \"algorithm\" \"macqueen\" set \"iter.max\" 1 set",
        "dict \"algorithm\" 1 set",
        "dict \"iter.max\" 1.5 set",
        "dict \"nstart\" 3 set",
        "dict \"nstart\" na set",
    ];
    for a in xs {
        for b in centres {
            for o in opts {
                for src in [
                    format!("{a} {b} {o} cluster.kmeans"),
                    format!("{a} {b} {o} . cluster.kmeans."),
                ] {
                    if let Err(e) = run(&src) {
                        assert!(
                            e.contains(": E1") || e.contains("too shallow"),
                            "{src}: {e}"
                        );
                    }
                }
            }
        }
        let src = format!("{a} 2 cluster.kmeans");
        if let Err(e) = run(&src) {
            assert!(
                e.contains(": E1") || e.contains("too shallow"),
                "{src}: {e}"
            );
        }
    }
}

#[test]
fn kmeans_dot_form_answers_on_the_workbench() {
    let src = "[ [ 1 2 ] [ 3 5 ] [ 4 4 ] [ 9 9 ] ] matrix [ [ 1 2 ] [ 9 9 ] ] matrix dict";
    let plain = run(&format!("{src} cluster.kmeans"))
        .unwrap()
        .pull()
        .unwrap();
    let mut i = run(&format!("7 {src} . cluster.kmeans.")).unwrap();
    let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
    assert_eq!(shown, ["7"]);
    let got = i.pull_workbench().unwrap();
    assert_eq!(format!("{got:?}"), format!("{plain:?}"));
}
