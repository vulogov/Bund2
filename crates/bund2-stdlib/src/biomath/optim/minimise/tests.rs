//! RFC-0019 criterion 12's minimiser and Hessian bullets, and 1 and 14, for
//! §S10.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use crate::biomath::testkit::{err, floats, run};

fn answer(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    assert_eq!(i.depth(), 0, "{src}");
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn key(m: &BTreeMap<String, BundValue>, k: &str) -> Vec<f64> {
    floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")))
}

fn rows(m: &BTreeMap<String, BundValue>, k: &str) -> Vec<Vec<f64>> {
    m.get(k)
        .and_then(BundValue::as_matrix)
        .unwrap_or_else(|| panic!("no MATRIX {k} in {m:?}"))
        .iter()
        .map(|r| r.iter().flat_map(floats).collect())
        .collect()
}

/// Rosenbrock's 100(y − x²)² + (1 − x)², `x -- y`; `a b -` is b − a.
const ROSENBROCK: &str = "{ unfold :ry swap var :rx swap var \
     :rx var? dup * :ry var? swap - dup * 100.0 * \
     1.0 :rx var? - dup * + }";

/// 3x² + 2xy + 5y² − x, whose Hessian is [[6, 2], [2, 10]].
const QUADRATIC: &str = "{ unfold :qy swap var :qx swap var \
     :qx var? dup * 3.0 * \
     :qx var? :qy var? * 2.0 * + \
     :qy var? dup * 5.0 * + \
     :qx var? swap - }";

// Criterion 1.

#[test]
fn every_word_is_registered_without_a_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in ["optim.nm", "optim.bfgs", "optim.hessian"] {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        assert!(e.opaque, "{w}");
        assert_eq!(e.consumes, 3, "{w}");
        assert!(i.registry.effect_of(&format!("{w}.")).is_none(), "{w}.");
    }
}

// Criterion 12.

/// Nelder–Mead reaches (1, 1) from (−1.2, 1) to 10⁻⁴ with `reltol` 10⁻¹²,
/// and BFGS to 10⁻⁶ with `ndeps` 10⁻⁶ (D275 4A): at R's defaults both
/// stop short, as R's own `optim` does.
#[test]
fn rosenbrock_is_minimised() {
    let nm = answer(&format!(
        "{ROSENBROCK} [ -1.2 1.0 ] dict \"reltol\" 0.000000000001 set optim.nm"
    ));
    for x in key(&nm, "par") {
        assert!((x - 1.0).abs() <= 1e-4, "nm {nm:?}");
    }
    let bfgs = answer(&format!(
        "{ROSENBROCK} [ -1.2 1.0 ] dict \"ndeps\" 0.000001 set optim.bfgs"
    ));
    for x in key(&bfgs, "par") {
        assert!((x - 1.0).abs() <= 1e-6, "bfgs {bfgs:?}");
    }
    assert_eq!(key(&bfgs, "gradient").len(), 2);
    // At R's defaults: short of the criterion, as R 4.6.2 is.
    let nm = answer(&format!("{ROSENBROCK} [ -1.2 1.0 ] dict optim.nm"));
    let off = key(&nm, "par")
        .iter()
        .map(|x| (x - 1.0).abs())
        .fold(0.0, f64::max);
    assert!(off > 1e-4 && off < 1e-3, "{nm:?}");
    let bfgs = answer(&format!("{ROSENBROCK} [ -1.2 1.0 ] dict optim.bfgs"));
    let off = key(&bfgs, "par")
        .iter()
        .map(|x| (x - 1.0).abs())
        .fold(0.0, f64::max);
    assert!(off > 1e-6 && off < 1e-3, "{bfgs:?}");
}

/// The answer's keys, and the counts as integers.
#[test]
fn each_minimiser_answers_its_keys() {
    let nm = answer(&format!("{QUADRATIC} [ 1.0 1.0 ] dict optim.nm"));
    let keys: Vec<&str> = nm.keys().map(String::as_str).collect();
    assert_eq!(keys, ["evaluations", "iterations", "par", "value"]);
    let bfgs = answer(&format!("{QUADRATIC} [ 1.0 1.0 ] dict optim.bfgs"));
    let keys: Vec<&str> = bfgs.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["evaluations", "gradient", "iterations", "par", "value"]
    );
    // The minimum of 3x² + 2xy + 5y² − x is at (5/28, −1/28).
    for m in [&nm, &bfgs] {
        let p = key(m, "par");
        assert!((p[0] - 5.0 / 28.0).abs() < 1e-3, "{m:?}");
        assert!((p[1] + 1.0 / 28.0).abs() < 1e-3, "{m:?}");
        assert!(m["iterations"].as_int().is_some());
        assert!(m["evaluations"].as_int().is_some());
    }
}

/// The Hessian of a quadratic is its closed form to 10⁻⁶, symmetric, and
/// the gradient at x is.
#[test]
fn the_hessian_of_a_quadratic_is_its_closed_form() {
    let m = answer(&format!("{QUADRATIC} [ 0.3 -0.7 ] dict optim.hessian"));
    let h = rows(&m, "hessian");
    let want = [[6.0, 2.0], [2.0, 10.0]];
    for i in 0..2 {
        for j in 0..2 {
            assert!((h[i][j] - want[i][j]).abs() <= 1e-6, "{h:?}");
        }
    }
    assert_eq!(h[0][1].to_bits(), h[1][0].to_bits());
    // ∇ = (6x + 2y − 1, 2x + 10y).
    let g = key(&m, "gradient");
    assert!((g[0] - (6.0 * 0.3 - 1.4 - 1.0)).abs() <= 1e-6, "{g:?}");
    assert!((g[1] - (0.6 - 7.0)).abs() <= 1e-6, "{g:?}");
    // ndeps as a LIST of k.
    let l = answer(&format!(
        "{QUADRATIC} [ 0.3 -0.7 ] dict \"ndeps\" [ 0.0001 0.01 ] set optim.hessian"
    ));
    let h = rows(&l, "hessian");
    assert!((h[1][1] - 10.0).abs() <= 1e-6, "{h:?}");
}

#[test]
fn each_refusal_is_its_template() {
    let e = |src: &str, want: &str| assert_eq!(err(src), want, "{src}");
    e(
        &format!("{ROSENBROCK} [ -1.2 1.0 ] dict \"maxit\" 20 set optim.nm"),
        "OPTIM.NM: E110 Nelder–Mead did not converge within 20 iterations",
    );
    e(
        &format!("{ROSENBROCK} [ -1.2 1.0 ] dict \"maxit\" 3 set optim.bfgs"),
        "OPTIM.BFGS: E110 BFGS did not converge within 3 iterations",
    );
    e(
        "{ drop \"s\" } [ 1.0 ] dict optim.nm",
        "OPTIM.NM: E112 the model lambda left \"s\", where one finite number was required",
    );
    e(
        "{ drop 1.0 } list dict optim.bfgs",
        "OPTIM.BFGS: E101 x0 is outside optim.bfgs's domain: an empty LIST",
    );
    e(
        "1.0 [ 1.0 ] dict optim.hessian",
        "OPTIM.HESSIAN: E101 f is outside optim.hessian's domain: 1.0",
    );
    e(
        "{ drop 1.0 } [ 1.0 ] dict \"ndeps\" 0.1 set optim.nm",
        "OPTIM.NM: E101 opts is outside optim.nm's domain: an unknown key \"ndeps\"",
    );
    e(
        "{ drop 1.0 } [ 1.0 2.0 ] dict \"ndeps\" [ 0.1 ] set optim.hessian",
        "OPTIM.HESSIAN: E101 ndeps is outside optim.hessian's domain: list/1 [0.1]",
    );
    e(
        "{ drop 1.0 } [ 1.0 ] dict \"reltol\" 0 set optim.nm",
        "OPTIM.NM: E101 reltol is outside optim.nm's domain: 0",
    );
    e(
        "{ drop 1.0 } [ 1.0 ] dict \"maxit\" 0 set optim.bfgs",
        "OPTIM.BFGS: E101 maxit is outside optim.bfgs's domain: 0",
    );
    let many = format!("[ {} ]", vec!["1.0"; 101].join(" "));
    e(
        &format!("{{ drop 1.0 }} {many} dict optim.nm"),
        "OPTIM.NM: E111 101 parameters exceeds its cap of 100",
    );
}

// Criterion 14.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "float.+Inf",
        "-1",
        "0",
        "1.0",
        "\"s\"",
        "list",
        "dict",
        "nodata",
        "[ 1.0 ]",
        "[ 1.0 float.NaN push ]",
        "[ 1 \"a\" ]",
        "[ [ 1.0 ] ]",
        "{ 1 }",
        "{ drop }",
        "{ drop float.NaN }",
        "{ drop float.+Inf }",
        "{ drop \"s\" }",
        "{ no.such.word }",
        "{ unfold }",
        "dict \"ndeps\" -1.0 set",
        "dict \"maxit\" 1 set",
        "dict \"reltol\" float.NaN set",
    ];
    for w in ["optim.nm", "optim.bfgs", "optim.hessian"] {
        for o in operands {
            for k in 1..=3 {
                let _ = run(&format!("{} {w}", vec![o; k].join(" ")));
            }
            let _ = run(&format!("{o} [ 1.0 2.0 ] dict {w}"));
            let _ = run(&format!("{ROSENBROCK} {o} dict {w}"));
            let _ = run(&format!("{ROSENBROCK} [ 1.0 2.0 ] {o} {w}"));
        }
    }
}

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":optim.bfgs { 42 } register 1 optim.bfgs").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
