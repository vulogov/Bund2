//! RFC-0019 criteria 8, and 1 and 14, for §S6.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::{Bracket, Extend, Stop, WORDS, extend, zeroin};
use crate::biomath::quadpack::{self, Range};
use crate::biomath::testkit::{err, fixture, floats, run};

const OPTIM: &str = include_str!("../fixtures/optim.txt");

fn answer(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn key(m: &BTreeMap<String, BundValue>, k: &str) -> f64 {
    floats(&m[k])[0]
}

// Criterion 1.

#[test]
fn every_word_is_registered_without_a_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        assert!(e.opaque, "{w}");
        assert!(i.registry.effect_of(&format!("{w}.")).is_none(), "{w}.");
    }
    let ours: Vec<String> = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("optim."))
        .collect();
    assert_eq!(ours.len(), WORDS.len(), "{ours:?}");
}

// Criterion 8: roots.

/// x³ − 2, cos x − x, and the functions the extensions need, as lambdas;
/// `a b -` is b − a, and `a b /` is b/a.
const ROOTS: [(&str, &str, &str); 6] = [
    ("cubic", "{ dup dup * * 2.0 swap - }", "0.0 2.0 dict"),
    ("cos", "{ dup math.cos swap - }", "0.0 1.0 dict"),
    (
        "yes",
        "{ 10.0 swap - }",
        "0.0 1.0 dict \"extend\" \"yes\" set",
    ),
    (
        "upX.lower",
        "{ 4.0 + }",
        "0.0 1.0 dict \"extend\" \"upX\" set",
    ),
    (
        "upX.upper",
        "{ 10.0 swap - }",
        "0.0 1.0 dict \"extend\" \"upX\" set",
    ),
    (
        "downX",
        "{ 7.0 - }",
        "0.0 1.0 dict \"extend\" \"downX\" set",
    ),
];

#[test]
fn each_root_is_found_to_its_tolerance() {
    for (name, f, rest) in ROOTS {
        let src = format!("{f} {rest} optim.root");
        let m = answer(&src);
        let want = fixture(OPTIM, &format!("root {name}"))[0];
        let (root, prec) = (key(&m, "root"), key(&m, "estim.prec"));
        // The root lies within estim.prec = |c − b| of the answer, and
        // Brent stops once that is within 2ε|b| + tol/2 of the step.
        assert!(
            (root - want).abs() <= prec.max(1e-15),
            "{src}: {root} against {want}"
        );
        // Unless f is exactly 0 there, which ends the search however wide
        // [b, c] is, as in R.
        let tol = 1e-12 * 11.23;
        let exact = key(&m, "f.root") == 0.0;
        assert!(
            exact || prec <= 4.0 * f64::EPSILON * root.abs() + tol,
            "{src}: estim.prec {prec}"
        );
        assert!(key(&m, "f.root").abs() <= 1e-9, "{src}: f.root");
        assert!(key(&m, "iterations") >= 1.0, "{src}");
    }
}

#[test]
fn the_answer_has_uniroots_keys() {
    let m = answer("{ dup dup * * 2.0 swap - } 0.0 2.0 dict optim.root");
    let keys: Vec<&str> = m.keys().map(String::as_str).collect();
    assert_eq!(keys, ["estim.prec", "f.root", "iterations", "root"]);
    // A root at an end is found before any iteration.
    let m = answer("{ 1.0 - } 1.0 2.0 dict optim.root");
    assert_eq!((key(&m, "root"), key(&m, "iterations")), (1.0, 0.0));
    assert_eq!(key(&m, "estim.prec"), 0.0);
}

#[test]
fn an_extension_reaches_the_end_rs_rule_reaches() {
    type F = fn(f64) -> f64;
    let cases: [(&str, F, Extend); 4] = [
        ("yes", |x| x - 10.0, Extend::Yes),
        ("upX.lower", |x| x + 4.0, Extend::UpX),
        ("upX.upper", |x| x - 10.0, Extend::UpX),
        ("downX", |x| 7.0 - x, Extend::DownX),
    ];
    for (name, g, how) in cases {
        let line = OPTIM
            .lines()
            .find(|l| l.starts_with(&format!("extend {name} ")))
            .unwrap();
        let v: Vec<&str> = line.split_whitespace().collect();
        let (lo, hi, steps): (f64, f64, u64) = (
            v[2].parse().unwrap(),
            v[3].parse().unwrap(),
            v[4].parse().unwrap(),
        );
        let mut f = |x: f64| Ok::<f64, ()>(g(x));
        let start = Bracket {
            lower: 0.0,
            f_lower: g(0.0),
            upper: 1.0,
            f_upper: g(1.0),
            steps: 0,
        };
        let b = extend(&mut f, start, how, 1000).unwrap();
        // Bit for bit: the same doubles, by the same steps.
        assert_eq!((b.lower, b.upper, b.steps), (lo, hi, steps), "{name}");
        // The word adds those steps to Brent's, as `uniroot` does; its
        // tolerance is from the ends as given, 10⁻¹²·max(1, 0, 1).
        let z = zeroin(&mut f, b.lower, b.upper, b.f_lower, b.f_upper, 1e-12, 1000).unwrap();
        let (src_f, rest) = ROOTS
            .iter()
            .find(|r| r.0 == name)
            .map(|r| (r.1, r.2))
            .unwrap();
        let m = answer(&format!("{src_f} {rest} optim.root"));
        assert_eq!(
            key(&m, "iterations"),
            (z.iterations + steps) as f64,
            "{name}"
        );
    }
}

#[test]
fn brents_steps_are_zeroins() {
    // x³ − 2 on [0, 2] at tol 2·10⁻¹²: Brent's own iterations, the same
    // whether asked through the word or directly.
    let mut f = |x: f64| Ok::<f64, ()>(x * x * x - 2.0);
    let z = zeroin(&mut f, 0.0, 2.0, -2.0, 6.0, 2e-12, 1000).unwrap();
    let m = answer("{ dup dup * * 2.0 swap - } 0.0 2.0 dict optim.root");
    assert_eq!(key(&m, "root"), z.root);
    assert_eq!(key(&m, "iterations"), z.iterations as f64);
    assert_eq!(key(&m, "estim.prec"), z.estim_prec);
    // A tighter cap than the steps it needs is the failure flag.
    let z = zeroin(&mut f, 0.0, 2.0, -2.0, 6.0, 2e-12, 2).unwrap();
    assert!(!z.converged);
}

// Criterion 8: quadrature.

const INTEGRALS: [(&str, &str, &str, &str); 6] = [
    (
        "polynomial",
        "{ dup dup dup * * swap 2.0 * swap - 1.0 + }",
        "0.0",
        "3.0",
    ),
    (
        "gaussian",
        "{ dup * -1.0 * math.exp }",
        "\"-inf\"",
        "\"inf\"",
    ),
    ("rsqrt", "{ math.sqrt 1.0 / }", "0.0", "1.0"),
    ("peak", "{ 0.3 - dup * 0.0001 + 1.0 / }", "0.0", "1.0"),
    ("exp.upper", "{ -1.0 * math.exp }", "0.0", "\"inf\""),
    ("exp.lower", "{ math.exp }", "\"-inf\"", "0.0"),
];

#[test]
fn each_integral_matches_its_closed_form() {
    for (name, f, lo, hi) in INTEGRALS {
        let src = format!("{f} {lo} {hi} dict optim.integrate");
        let m = answer(&src);
        let want = fixture(OPTIM, &format!("integral {name}"))[0];
        let got = key(&m, "value");
        assert!(
            ((got - want) / want).abs() <= 1e-10,
            "{src}: {got} against {want}"
        );
        assert!(key(&m, "abs.error") <= 1e-10 * want.abs(), "{src}");
        assert!(key(&m, "subdivisions") >= 1.0, "{src}");
    }
}

#[test]
fn reversed_ends_negate_the_integral() {
    for (f, lo, hi) in [
        ("{ dup * }", "0.0", "3.0"),
        ("{ -1.0 * math.exp }", "0.0", "\"inf\""),
        ("{ dup * -1.0 * math.exp }", "\"-inf\"", "\"inf\""),
    ] {
        let a = key(
            &answer(&format!("{f} {lo} {hi} dict optim.integrate")),
            "value",
        );
        let b = key(
            &answer(&format!("{f} {hi} {lo} dict optim.integrate")),
            "value",
        );
        assert_eq!(a, -b, "{f} {lo} {hi}");
    }
    let m = answer("{ dup * } \"inf\" \"inf\" dict optim.integrate");
    assert_eq!(key(&m, "value"), 0.0);
}

#[test]
fn quadpack_agrees_with_rs_integrate_on_its_own_example() {
    // `integrate`'s help page: dnorm over (−∞, ∞) is 1, and over (0, ∞) ½.
    let pdf = |x: f64| Ok::<f64, ()>((-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt());
    let q = quadpack::integrate(
        &mut { pdf },
        Range::Infinite { bound: 0.0, inf: 2 },
        1e-10,
        1e-10,
        100,
    )
    .unwrap();
    assert_eq!(q.ier, 0);
    assert!((q.value - 1.0).abs() <= 1e-12, "{q:?}");
    let q = quadpack::integrate(
        &mut { pdf },
        Range::Infinite { bound: 0.0, inf: 1 },
        1e-10,
        1e-10,
        100,
    )
    .unwrap();
    assert!((q.value - 0.5).abs() <= 1e-12, "{q:?}");
}

// Criterion 8: refusals.

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            "{ dup * 1.0 + } 0.0 1.0 dict optim.root",
            "OPTIM.ROOT: E101 f is outside optim.root's domain: the same sign at lower and upper",
        ),
        (
            "{ dup * 1.0 + } 0.0 1.0 dict \"extend\" \"yes\" set \"maxiter\" 5 set optim.root",
            "OPTIM.ROOT: E110 the search for a sign change did not converge within 5 iterations",
        ),
        (
            "{ dup math.cos swap - } 0.0 1.0 dict \"maxiter\" 2 set optim.root",
            "OPTIM.ROOT: E110 optim.root did not converge within 2 iterations",
        ),
        (
            "{ drop float.NaN } 0.0 1.0 dict optim.root",
            "OPTIM.ROOT: E112 the model lambda left NaN, where one finite number was required",
        ),
        (
            "{ drop } 0.0 1.0 dict optim.root",
            "OPTIM.ROOT: E112 the model lambda left nothing, where one finite number was required",
        ),
        (
            "{ drop \"x\" } 0.0 1.0 dict optim.integrate",
            "OPTIM.INTEGRATE: E112 the model lambda left \"x\", where one finite number was required",
        ),
        (
            "1.0 0.0 1.0 dict optim.root",
            "OPTIM.ROOT: E101 f is outside optim.root's domain: 1.0",
        ),
        (
            "{ 1.0 } \"infinity\" 1.0 dict optim.integrate",
            "OPTIM.INTEGRATE: E101 lower is outside optim.integrate's domain: \"infinity\"",
        ),
        (
            "{ 1.0 } 0.0 \"inf\" dict optim.root",
            "OPTIM.ROOT: E101 upper is outside optim.root's domain: \"inf\"",
        ),
        (
            "{ 1.0 - } 1.0 0.0 dict optim.root",
            "OPTIM.ROOT: E101 upper is outside optim.root's domain: 0.0, not above lower 1.0",
        ),
        (
            "{ 100.0 * math.sin } 0.0 10.0 dict \"subdivisions\" 2 set optim.integrate",
            "OPTIM.INTEGRATE: E110 optim.integrate did not converge within 2 iterations",
        ),
        (
            "{ 1.0 } 0.0 1.0 dict \"tol\" 1.0 set optim.integrate",
            "OPTIM.INTEGRATE: E101 opts is outside optim.integrate's domain: an unknown key \"tol\"",
        ),
        (
            "{ 1.0 } 0.0 1.0 dict \"extend\" \"sideways\" set optim.root",
            "OPTIM.ROOT: E101 extend is outside optim.root's domain: \"sideways\"",
        ),
        (
            "{ 1.0 } 0.0 1.0 dict \"rel.tol\" 0.0 set \"abs.tol\" 0.0 set optim.integrate",
            "OPTIM.INTEGRATE: E101 rel.tol is outside optim.integrate's domain: 0, below 50ε while abs.tol is 0",
        ),
    ] {
        assert_eq!(err(src), want, "{src}");
    }
}

#[test]
fn an_error_in_the_lambda_passes_through() {
    let e = err("{ no.such.word } 0.0 1.0 dict optim.root");
    assert!(!e.contains("OPTIM.ROOT: E1"), "{e}");
    let e = err("{ no.such.word } 0.0 1.0 dict optim.integrate");
    assert!(!e.contains("OPTIM.INTEGRATE: E1"), "{e}");
}

#[test]
fn the_callers_stack_is_unchanged_by_the_lambda() {
    let mut i = run("7 { 10.0 - } 0.0 1.0 dict \"extend\" \"yes\" set optim.root").unwrap();
    let _ = i.pull();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(7));
}

#[test]
fn an_extension_whose_f_fails_passes_the_failure_on() {
    let mut calls = 0;
    let mut f = |x: f64| {
        calls += 1;
        if calls > 3 { Err("stop") } else { Ok(x - 10.0) }
    };
    let start = Bracket {
        lower: 0.0,
        f_lower: -10.0,
        upper: 1.0,
        f_upper: -9.0,
        steps: 0,
    };
    assert!(matches!(
        extend(&mut f, start, Extend::Yes, 1000),
        Err(Stop::F("stop"))
    ));
}

// Criterion 14.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "float.+Inf",
        "float.-Inf",
        "-1",
        "0",
        "1.0",
        "\"x\"",
        "\"inf\"",
        "list",
        "dict",
        "[ 1.0 ]",
        "{ 1 }",
        "{ drop }",
        "{ drop float.NaN }",
        "{ drop \"s\" }",
        "{ no.such.word }",
        "[ [ 1 ] ] matrix",
        "nodata",
    ];
    for w in WORDS {
        for o in operands {
            for k in 1..=4 {
                let src = format!("{} {w}", vec![o; k].join(" "));
                let _ = run(&src);
            }
            for f in [
                "{ 1 }",
                "{ drop }",
                "{ drop float.NaN }",
                "{ no.such.word }",
                "{ dup }",
            ] {
                let _ = run(&format!("{f} {o} 1.0 dict {w}"));
                let _ = run(&format!("{f} 0.0 {o} dict {w}"));
                let _ = run(&format!("{f} 0.0 1.0 {o} {w}"));
            }
        }
    }
}

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":optim.root { 42 } register 1 optim.root").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
