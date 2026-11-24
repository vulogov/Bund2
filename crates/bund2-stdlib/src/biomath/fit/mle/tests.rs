//! RFC-0019 criterion 12's `fit.mle` bullets, and 14, for §S11.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use crate::biomath::testkit::{err, fixture, floats, run};

const MLE: &str = include_str!("../../fixtures/mle.txt");

fn answer(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    assert_eq!(i.depth(), 0, "{src}");
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn key(m: &BTreeMap<String, BundValue>, k: &str) -> Vec<f64> {
    floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")))
}

fn data(case: &str) -> Vec<f64> {
    fixture(MLE, &format!("{case} data"))
}

/// The exponential log-likelihood n ln r − r·S of the fixture's sample,
/// `par -- FLOAT`; `a b -` is b − a.
fn exponential() -> String {
    let xs = data("e");
    let (n, s): (f64, f64) = (xs.len() as f64, xs.iter().sum());
    format!("{{ unfold :er swap var :er var? math.ln {n:?} * :er var? {s:?} * swap - }}")
}

/// The normal log-likelihood −n ln|σ| − Σ(x − μ)²/(2σ²) of the fixture's
/// sample, the constant left out; Σ(x − μ)² = SS − 2μS + nμ². |σ| keeps
/// a line search's trial at σ < 0 finite.
fn normal() -> String {
    let xs = data("m");
    let n = xs.len() as f64;
    let s: f64 = xs.iter().sum();
    let ss: f64 = xs.iter().map(|x| x * x).sum();
    let two_s = 2.0 * s;
    format!(
        "{{ unfold :ns swap var :nm swap var \
           :nm var? dup * {n:?} * :nm var? {two_s:?} * - {ss:?} - \
           :ns var? dup * 2.0 * swap / \
           :ns var? math.abs math.ln {n:?} * + -1.0 * }}"
    )
}

/// Criterion 12's tolerance for an estimate and its standard errors.
fn near(what: &str, got: f64, want: f64, tol: f64) {
    assert!((got - want).abs() <= tol, "{what}: {got} against {want}");
}

// Criterion 1.

#[test]
fn the_word_is_registered_without_a_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    let e = i
        .registry
        .effect_of("fit.mle")
        .unwrap_or_else(|| panic!("fit.mle"));
    assert!(e.opaque);
    assert_eq!(e.consumes, 3);
    assert!(i.registry.effect_of("fit.mle.").is_none());
}

// Criterion 12.

/// A normal sample's MLEs are x̄ and σ̂ on n, and the standard errors are
/// σ̂/√n and σ̂/√(2n) within 10⁻⁵, by either method.
#[test]
fn a_normal_sample_gives_the_closed_form() {
    let xs = data("m");
    let n = xs.len() as f64;
    let (mean, sigma) = (fixture(MLE, "m mean")[0], fixture(MLE, "m sigma")[0]);
    // Nelder–Mead at R's default `reltol` stops within about 3·10⁻⁵ of the
    // mean, as on Rosenbrock (D275 4A), so it is given 10⁻¹².
    for method in ["\"bfgs\"", "\"nm\" set \"reltol\" 0.000000000001"] {
        let m = answer(&format!(
            "{} [ 5.0 1.0 ] dict \"method\" {method} set \"n\" 12 set fit.mle",
            normal()
        ));
        let (par, se) = (key(&m, "par"), key(&m, "se"));
        near("mean", par[0], mean, 1e-5);
        near("sigma", par[1].abs(), sigma, 1e-5);
        near("se mean", se[0], sigma / n.sqrt(), 1e-5);
        near("se sigma", se[1], sigma / (2.0 * n).sqrt(), 1e-5);
        let l = key(&m, "loglik")[0];
        near("aic", key(&m, "aic")[0], 4.0 - 2.0 * l, 1e-12);
        near("bic", key(&m, "bic")[0], 2.0 * n.ln() - 2.0 * l, 1e-12);
        let keys: Vec<&str> = m.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            ["aic", "bic", "iterations", "loglik", "par", "se", "vcov"]
        );
    }
}

/// The profile interval for an exponential rate, and for a normal mean with
/// σ profiled out, are the fixture's χ² inversions to 10⁻⁶.
#[test]
fn the_profile_intervals_are_the_fixtures() {
    let rate = fixture(MLE, "e rate")[0];
    for conf in ["0.95", "0.9"] {
        let want = |case: &str, end: &str| fixture(MLE, &format!("{case}@{conf} {end}"))[0];
        let m = answer(&format!(
            "{} [ 0.5 ] dict \"profile\" true set \"conf\" {conf} set fit.mle",
            exponential()
        ));
        near("rate", key(&m, "par")[0], rate, 1e-6);
        near("e lower", key(&m, "lower")[0], want("e", "lower"), 1e-6);
        near("e upper", key(&m, "upper")[0], want("e", "upper"), 1e-6);
        let m = answer(&format!(
            "{} [ 5.0 1.0 ] dict \"profile\" true set \"conf\" {conf} set fit.mle",
            normal()
        ));
        near("m lower", key(&m, "lower")[0], want("m", "lower"), 1e-6);
        near("m upper", key(&m, "upper")[0], want("m", "upper"), 1e-6);
        assert_eq!(key(&m, "lower").len(), 2);
    }
}

/// ℓ(a, b) = −(a − 1)² does not depend on b: BFGS converges to a = 1, and
/// the Hessian's second pivot is exactly 0 (D269, 7).
#[test]
fn an_indefinite_hessian_is_e106() {
    assert_eq!(
        err("{ unfold drop 1.0 - dup * -1.0 * } [ 0.0 0.0 ] dict fit.mle"),
        "FIT.MLE: E106 the Hessian of -loglik at par is not positive definite: pivot 2 is 0"
    );
}

#[test]
fn each_refusal_is_its_template() {
    let e = |src: &str, want: &str| assert_eq!(err(src), want, "{src}");
    e(
        &format!(
            "{} [ 0.5 ] dict \"method\" \"lbfgsb\" set fit.mle",
            exponential()
        ),
        "FIT.MLE: E101 method is outside fit.mle's domain: \"lbfgsb\"",
    );
    e(
        &format!("{} [ 0.5 ] dict \"profile\" 1 set fit.mle", exponential()),
        "FIT.MLE: E101 profile is outside fit.mle's domain: 1",
    );
    e(
        &format!("{} [ 0.5 ] dict \"conf\" 1 set fit.mle", exponential()),
        "FIT.MLE: E101 conf is outside fit.mle's domain: 1",
    );
    e(
        &format!("{} [ 0.5 ] dict \"type\" 1 set fit.mle", exponential()),
        "FIT.MLE: E101 opts is outside fit.mle's domain: an unknown key \"type\"",
    );
    e(
        &format!("{} [ 0.5 ] dict \"maxit\" 1 set fit.mle", exponential()),
        "FIT.MLE: E110 BFGS did not converge within 1 iterations",
    );
    e(
        "{ drop float.NaN } [ 0.5 ] dict fit.mle",
        "FIT.MLE: E112 the model lambda left NaN, where one finite number was required",
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
        "{ 1 }",
        "{ drop }",
        "{ drop float.NaN }",
        "{ drop \"s\" }",
        "{ no.such.word }",
        "{ unfold }",
        "{ drop 1.0 }",
        "dict \"profile\" true set",
        "dict \"method\" \"nm\" set \"profile\" true set",
        "dict \"n\" 0 set",
    ];
    for o in operands {
        for k in 1..=3 {
            let _ = run(&format!("{} fit.mle", vec![o; k].join(" ")));
        }
        let _ = run(&format!("{o} [ 0.5 ] dict fit.mle"));
        let _ = run(&format!("{} {o} dict fit.mle", exponential()));
        let _ = run(&format!("{} [ 0.5 ] {o} fit.mle", exponential()));
    }
}

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":fit.mle { 42 } register 1 fit.mle").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
