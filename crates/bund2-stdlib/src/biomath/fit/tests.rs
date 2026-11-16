//! RFC-0015 criteria 2, 3, 6 and 20, for §S16.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{close, err, fixture, floats, run};

const FIXTURE: &str = include_str!("../fixtures/fit.txt");

fn map(src: &str) -> BTreeMap<String, Vec<f64>> {
    let v = run(src)
        .unwrap_or_else(|e| panic!("{src}: {e}"))
        .pull()
        .unwrap_or_else(|| panic!("{src}: empty"));
    v.as_map()
        .unwrap_or_else(|| panic!("{src}: {v:?}"))
        .iter()
        .filter(|(k, _)| k.as_str() != "converged")
        .map(|(k, v)| (k.clone(), floats(v)))
        .collect()
}

/// R's `Puromycin`, the treated state.
const CONC: &str = "[ 0.02 0.02 0.06 0.06 0.11 0.11 0.22 0.22 0.56 0.56 1.10 1.10 ]";
const RATE: &str = "[ 76 47 97 107 123 139 159 152 191 201 207 200 ]";

/// rate = Vm·conc/(K + conc), `x p -- y`. K + x is x + K·(x/x), conc
/// being nowhere 0.
const MICHAELIS: &str = "{ unfold :mmK swap var :mmVm swap var \
     dup :mmVm var? vec.scale \
     swap dup dup vec.div :mmK var? vec.scale vec.add vec.div }";

fn puromycin(p0: &str) -> String {
    format!("{MICHAELIS} {CONC} {RATE} {p0} fit.nls")
}

/// R's `mtcars`, vs on mpg.
const MPG: [f64; 32] = [
    21.0, 21.0, 22.8, 21.4, 18.7, 18.1, 14.3, 24.4, 22.8, 19.2, 17.8, 16.4, 17.3, 15.2, 10.4, 10.4,
    14.7, 32.4, 30.4, 33.9, 21.5, 15.5, 15.2, 13.3, 19.2, 27.3, 26.0, 30.4, 15.8, 19.7, 15.0, 21.4,
];
const VS: [u8; 32] = [
    0, 0, 1, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1,
];

fn column(xs: &[f64]) -> String {
    let rows: Vec<String> = xs.iter().map(|x| format!("[ {x:?} ]")).collect();
    format!("[ {} ] matrix", rows.join(" "))
}

fn series<T: std::fmt::Debug>(xs: &[T]) -> String {
    let items: Vec<String> = xs.iter().map(|x| format!("{x:?}")).collect();
    format!("[ {} ]", items.join(" "))
}

fn mtcars() -> String {
    format!("{} {} fit.logistic", column(&MPG), series(&VS))
}

// Criterion 2.

#[test]
fn every_word_is_registered_with_its_forms() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    let nls = i.registry.effect_of("fit.nls").unwrap();
    assert!(nls.opaque);
    assert!(i.registry.effect_of("fit.nls.").is_none());
    for w in ["fit.logistic", "fit.logistic."] {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        assert!(!e.opaque, "{w}");
    }
    let ours: Vec<String> = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("fit."))
        .collect();
    assert_eq!(ours.len(), WORDS.len(), "{ours:?}");
    for w in WORDS {
        assert!(ours.iter().any(|n| n == w), "{w}");
    }
}

// Criterion 20.

#[test]
fn nls_on_puromycin_as_r_prints_it() {
    let m = map(&puromycin("[ 200.0 0.05 ]"));
    // R's nls, as criterion 20 states it.
    close("params", &m["params"], &[212.68, 0.06412], 1e-4);
    close("se", &m["se"], &[6.947, 0.008281], 1e-4);
    close("rss", &m["rss"], &[1195.45], 1e-4);
    assert_eq!(m["df"], [10.0]);
    assert!(m["iterations"][0] >= 1.0 && m["iterations"][0] <= 200.0);
    // And the exact least-squares answer, far tighter.
    for key in ["params", "se", "rss"] {
        close(key, &m[key], &fixture(FIXTURE, &format!("nls.{key}")), 1e-6);
    }
    let v = run(&puromycin("[ 200.0 0.05 ]")).unwrap().pull().unwrap();
    assert_eq!(
        v.as_map().unwrap()["converged"],
        BundValue::boolean(true),
        "{v:?}"
    );
}

#[test]
fn nls_from_a_distant_start() {
    let m = map(&puromycin("[ 100 1 ]"));
    close(
        "params",
        &m["params"],
        &fixture(FIXTURE, "nls.params"),
        1e-6,
    );
}

#[test]
fn logistic_against_glm() {
    let m = map(&mtcars());
    // R's glm(vs ~ mpg, family = binomial), as it prints.
    close("coef", &m["coef"], &[-8.8331, 0.4304], 1e-4);
    close("se", &m["se"], &[3.1623, 0.1584], 2e-4);
    close("deviance", &m["deviance"], &[25.533], 1e-4);
    close("aic", &m["aic"], &[29.533], 1e-4);
    for key in ["coef", "se", "z", "p", "deviance", "aic"] {
        close(
            key,
            &m[key],
            &fixture(FIXTURE, &format!("logistic.{key}")),
            1e-6,
        );
    }
    assert_eq!(m.len(), 6, "{:?}", m.keys());
    // The `.` form takes y from the workbench and answers there.
    let plain = run(&mtcars()).unwrap().pull().unwrap();
    let mut i = run(&format!(
        "7 {} {} . fit.logistic.",
        column(&MPG),
        series(&VS)
    ))
    .unwrap();
    let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
    assert_eq!(shown, ["7"]);
    assert_eq!(i.pull_workbench().unwrap().display(), plain.display());
}

#[test]
fn the_callers_stack_is_unchanged_by_the_model() {
    // The model leaves 5 beneath its answer; it goes with the scratch stack.
    let src = format!(
        "99 {{ {} 5 swap }} {CONC} {RATE} [ 200.0 0.05 ] fit.nls",
        &MICHAELIS[1..MICHAELIS.len() - 1]
    );
    let mut i = run(&src).unwrap();
    let answer = i.pull().unwrap();
    assert!(answer.as_map().is_some(), "{answer:?}");
    assert_eq!(i.snapshot().len(), 1);
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(99));
}

// Criterion 3.

#[test]
fn each_refusal_is_its_template() {
    let lr = |x: &str, y: &str| format!("{x} {y} fit.logistic");
    for (src, want) in [
        (
            format!("42 {CONC} {RATE} [ 1 1 ] fit.nls"),
            "FIT.NLS: E101 f is outside fit.nls's domain: 42".to_string(),
        ),
        (
            format!("{MICHAELIS} [ 1 2 ] {RATE} [ 1 1 ] fit.nls"),
            "FIT.NLS: E102 lengths do not agree: 2 and 12".into(),
        ),
        (
            format!("{MICHAELIS} [ 1 2 ] [ 3 4 ] [ 1 1 ] fit.nls"),
            "FIT.NLS: E101 y is outside fit.nls's domain: 2 observations for 2 parameters"
                .into(),
        ),
        (
            format!("{MICHAELIS} {CONC} {RATE} list fit.nls"),
            "FIT.NLS: E101 p0 is outside fit.nls's domain: an empty LIST".into(),
        ),
        (
            format!("{{ drop drop [ 1 ] }} {CONC} {RATE} [ 1 1 ] fit.nls"),
            "FIT.NLS: E112 the model lambda left list/1 [1], where a LIST of 12 finite numbers was required"
                .into(),
        ),
        (
            // A model no parameter moves.
            format!("{{ drop }} {CONC} {RATE} [ 1 1 ] fit.nls"),
            "FIT.NLS: E106 the matrix is singular".into(),
        ),
        (
            lr("[ [ 1 ] [ 2 ] ] matrix", "[ 0 1 0 ]"),
            "FIT.LOGISTIC: E102 lengths do not agree: 2 and 3".into(),
        ),
        (
            lr("[ [ 1 ] [ 2 ] [ 3 ] ] matrix", "[ 0 2 1 ]"),
            "FIT.LOGISTIC: E101 y is outside fit.logistic's domain: 2 at 2, not 0 or 1".into(),
        ),
        (
            lr("[ [ 1 ] [ 2 ] ] matrix", "[ 0 1 ]"),
            "FIT.LOGISTIC: E101 y is outside fit.logistic's domain: 2 observations for 2 coefficients"
                .into(),
        ),
        (
            lr("[ [ 1 2 ] [ 2 4 ] [ 3 6 ] [ 4 8 ] ] matrix", "[ 0 1 0 1 ]"),
            "FIT.LOGISTIC: E106 the matrix is singular".into(),
        ),
        (
            lr("[ [ 1 ] [ 2 ] [ 3 ] [ 4 ] ] matrix", "[ 0 0 1 1 ]"),
            "FIT.LOGISTIC: E101 y is outside fit.logistic's domain: fitted probabilities numerically 0 or 1: X separates y"
                .into(),
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}

#[test]
fn a_singular_normal_matrix_is_damped_and_its_se_refused() {
    // y = (a + b)·x: a and b enter only through their sum, so JᵀJ is
    // singular. The damping keeps each step defined and the fit reaches
    // the least-squares sum, but the covariance does not exist.
    let src = "{ unfold + vec.scale } [ 1 2 3 ] [ 2 4 6.5 ] [ 1 1 ] fit.nls";
    assert_eq!(err(src), "FIT.NLS: E106 the matrix is singular");
}

// No abort, whatever the operand.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "-1",
        "0",
        "1.0",
        "\"x\"",
        "list",
        "dict",
        "[ 1.0 ]",
        "[ 1.0 2.0 3.0 ]",
        "{ 1 }",
        "{ drop }",
        "[ [ 1 ] ] matrix",
    ];
    for w in WORDS {
        for o in operands {
            for k in 1..=4 {
                let src = format!("{} {w}", vec![o; k].join(" "));
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

// Criterion 6.

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":fit.nls { 42 } register 1 fit.nls").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
