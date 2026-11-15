//! RFC-0015 criteria 2, 3, 6, 18 and 19, for §S15.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{err, floats, run};

fn pulled(src: &str) -> BundValue {
    run(src)
        .unwrap_or_else(|e| panic!("{src}: {e}"))
        .pull()
        .unwrap_or_else(|| panic!("{src}: empty"))
}

/// An answer's `t` and its states.
fn trajectory(src: &str, key: &str) -> (Vec<f64>, Vec<Vec<f64>>) {
    let v = pulled(src);
    let m: BTreeMap<String, BundValue> = v.as_map().unwrap_or_else(|| panic!("{v:?}")).clone();
    let ys = m[key].as_list().unwrap().iter().map(floats).collect();
    (floats(&m["t"]), ys)
}

/// dy/dt = y(1 − y/10): `y y² 0.1 ×`, then y − 0.1y², as a LIST.
const LOGISTIC: &str = "{ unfold dup dup * 0.1 * swap - list + }";

/// The closed form from y(0) = 0.5, r = 1, K = 10.
fn logistic(t: f64) -> f64 {
    10.0 / (1.0 + 19.0 * (-t).exp())
}

// Criterion 2.

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
        .filter(|n| n.starts_with("ode.") || n.starts_with("sim."))
        .collect();
    assert_eq!(ours.len(), WORDS.len(), "{ours:?}");
}

// Criterion 18.

#[test]
fn rk4_and_rk45_on_logistic_growth() {
    let (t, y) = trajectory(&format!("{LOGISTIC} [ 0.5 ] 0.0 10.0 0.01 ode.rk4"), "y");
    assert_eq!(t.len(), 1001);
    assert_eq!(*t.last().unwrap(), 10.0);
    for (ti, yi) in t.iter().zip(&y) {
        assert!((yi[0] - logistic(*ti)).abs() <= 1e-6, "rk4 at {ti}: {yi:?}");
    }
    let (t, y) = trajectory(
        &format!(
            "{LOGISTIC} [ 0.5 ] 0.0 10.0 dict \"rtol\" 1.0e-9 set \"atol\" 1.0e-12 set ode.rk45"
        ),
        "y",
    );
    assert_eq!(*t.last().unwrap(), 10.0);
    assert!(t.len() > 10 && t.len() < 1000, "{} steps", t.len());
    for (ti, yi) in t.iter().zip(&y) {
        assert!(
            (yi[0] - logistic(*ti)).abs() <= 1e-6,
            "rk45 at {ti}: {yi:?}"
        );
    }
    // Backwards, from y(10) to y(0).
    let (t, y) = trajectory(
        &format!("{LOGISTIC} [ {} ] 10.0 0.0 0.01 ode.rk4", logistic(10.0)),
        "y",
    );
    assert_eq!(*t.last().unwrap(), 0.0);
    assert!((y.last().unwrap()[0] - 0.5).abs() <= 1e-6);
}

/// Lotka–Volterra with α = β = γ = δ = 1: x′ = x − xy, y′ = xy − y. Its
/// first integral V = x − ln x + y − ln y is constant along a solution.
#[test]
fn lotka_volterra_conserves_its_first_integral() {
    let lv = "{ unfold :lvy swap var :lvx swap var drop \
               :lvx var? :lvy var? * :lvx var? - list + \
               :lvx var? :lvy var? * :lvy var? swap - swap + }";
    // From (2, 1) the period is about 6.6, so 70 is over ten periods.
    let (t, xy) = trajectory(
        &format!(
            "{lv} [ 2.0 1.0 ] 0.0 70.0 dict \"rtol\" 1.0e-11 set \"atol\" 1.0e-13 set ode.rk45"
        ),
        "y",
    );
    assert_eq!(*t.last().unwrap(), 70.0);
    let v = |p: &[f64]| p[0] - p[0].ln() + p[1] - p[1].ln();
    let v0 = v(&xy[0]);
    let drift = xy.iter().map(|p| (v(p) - v0).abs()).fold(0.0, f64::max);
    assert!(drift <= 1e-6, "V drifts by {drift}");
    // And the orbit really goes round: (2, 1) is x's maximum, and x rises
    // through the equilibrium's 1 once a lap.
    let laps = xy
        .windows(2)
        .filter(|w| w[0][0] < 1.0 && w[1][0] >= 1.0)
        .count();
    assert!(laps >= 10, "{laps} laps");
}

#[test]
fn a_model_leaving_an_integer_is_e112() {
    assert_eq!(
        err("{ drop drop 1 } [ 0.5 ] 0.0 1.0 0.5 ode.rk4"),
        "ODE.RK4: E112 the model lambda left 1, where a LIST of 1 finite numbers was required"
    );
    assert_eq!(
        err("{ drop drop } [ 0.5 ] 0.0 1.0 dict ode.rk45"),
        "ODE.RK45: E112 the model lambda left nothing, where a LIST of 1 finite numbers was required"
    );
    assert_eq!(
        err("{ drop drop [ 1 2 ] } [ 0.5 ] 0.0 1.0 0.5 ode.rk4"),
        "ODE.RK4: E112 the model lambda left list/2 [1, 2], where a LIST of 1 finite numbers was required"
    );
}

#[test]
fn the_callers_stack_is_unchanged_by_a_model_that_pushes_extra() {
    // The model leaves 5 and 6 beneath its answer; they go with its scratch
    // stack.
    let mut i = run("99 { drop drop 5 6 [ 0.0 ] } [ 1.0 ] 0.0 1.0 0.25 ode.rk4").unwrap();
    let answer = i.pull().unwrap();
    assert_eq!(i.snapshot().len(), 1);
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(99));
    let ys = answer.as_map().unwrap()["y"].as_list().unwrap().len();
    assert_eq!(ys, 5);
}

#[test]
fn the_caps() {
    assert_eq!(
        err(&format!("{LOGISTIC} [ 0.5 ] 0.0 1.0 0.0000001 ode.rk4")),
        "ODE.RK4: E111 an integration of 10000000 steps exceeds its cap of 1000000"
    );
    assert_eq!(
        err(&format!(
            "{LOGISTIC} [ 0.5 ] 0.0 1.0 dict \"hmax\" 0.0000001 set ode.rk45"
        )),
        "ODE.RK45: E111 an integration of at least 10000000 steps exceeds its cap of 1000000"
    );
    // y′ = y² from 1 blows up at t = 1, so the step shrinks without end.
    let e = err("{ unfold dup * list + } [ 1.0 ] 0.0 2.0 dict ode.rk45");
    assert!(
        e.starts_with("ODE.RK45: E110 rk45's step size control did not converge within "),
        "{e}"
    );
}

// Criterion 19.

/// Linear birth and death, birth 1·x and death 0.5·x from 10: the mean at
/// t is 10·e^{(1 − 0.5)t}, so 10·e^{0.5} at 1.
#[test]
fn gillespie_birth_death_mean_over_1000_replicates() {
    let props = "{ unfold dup 1.0 * list + swap 0.5 * swap + }";
    let one = format!("[ 10 ] {props} [ [ 1 ] [ -1 ] ] matrix 1.0 sim.gillespie ");
    let reps = 1000;
    let mut i = run(&format!("2026 rng.seed {}", one.repeat(reps))).unwrap();
    let finals: Vec<f64> = (0..reps)
        .map(|_| {
            let m = i.pull().unwrap();
            let m = m.as_map().unwrap();
            let t = floats(&m["t"]);
            assert_eq!(*t.last().unwrap(), 1.0);
            assert!(t.windows(2).all(|w| w[0] <= w[1]));
            floats(m["x"].as_list().unwrap().last().unwrap())[0]
        })
        .collect();
    let n = finals.len() as f64;
    let mean = finals.iter().sum::<f64>() / n;
    let sd = (finals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    let want = 10.0 * 0.5f64.exp();
    assert!(
        (mean - want).abs() <= 3.0 * sd / n.sqrt(),
        "mean {mean}, want {want}, se {}",
        sd / n.sqrt()
    );
    // Seeded alike, alike.
    let a = pulled(&format!("7 rng.seed {one}"));
    let b = pulled(&format!("7 rng.seed {one}"));
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}

#[test]
fn gillespie_stops_when_nothing_can_happen() {
    // Death only, from 3: three events, then the state held to tmax.
    let (t, x) = trajectory(
        "[ 3 ] { unfold list + } [ [ -1 ] ] matrix 1000.0 sim.gillespie",
        "x",
    );
    assert_eq!(
        x.iter().map(|p| p[0]).collect::<Vec<_>>(),
        [3.0, 2.0, 1.0, 0.0, 0.0]
    );
    assert_eq!(*t.last().unwrap(), 1000.0);
}

// Criterion 3.

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            "42 [ 1.0 ] 0.0 1.0 0.1 ode.rk4".to_string(),
            "ODE.RK4: E101 f is outside ode.rk4's domain: 42".to_string(),
        ),
        (
            format!("{LOGISTIC} [ 1.0 ] 0.0 1.0 0 ode.rk4"),
            "ODE.RK4: E101 h is outside ode.rk4's domain: 0".into(),
        ),
        (
            format!("{LOGISTIC} [ 1.0 ] 0.0 1.0 dict \"tol\" 1.0 set ode.rk45"),
            "ODE.RK45: E101 opts is outside ode.rk45's domain: an unknown key \"tol\"".into(),
        ),
        (
            format!("{LOGISTIC} [ 1.0 ] 0.0 1.0 dict \"rtol\" -1.0 set ode.rk45"),
            "ODE.RK45: E101 rtol is outside ode.rk45's domain: -1".into(),
        ),
        (
            "[ 1 2 ] { list } [ [ 1 ] ] matrix 1.0 sim.gillespie".into(),
            "SIM.GILLESPIE: E102 stoich's columns and x0 do not agree: 1 and 2".into(),
        ),
        (
            "[ 1 ] { drop [ -1.0 ] } [ [ 1 ] ] matrix 1.0 sim.gillespie".into(),
            "SIM.GILLESPIE: E112 the model lambda left list/1 [-1.0], where a LIST of 1 non-negative numbers was required"
                .into(),
        ),
        (
            "{ drop drop [ 1.0e308 ] } [ 1.0e308 ] 0.0 10.0 1.0 ode.rk4".into(),
            "ODE.RK4: E114 the answer overflows a FLOAT: y at t = 1".into(),
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
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
        "{ 1 }",
        "{ drop }",
        "[ [ 1 ] ] matrix",
    ];
    for w in WORDS {
        for o in operands {
            for k in 1..=5 {
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
    let mut i = run(":ode.rk4 { 42 } register 1 ode.rk4").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
