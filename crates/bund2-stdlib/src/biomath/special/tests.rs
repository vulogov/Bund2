//! RFC-0015 criteria 2, 3, 5, 6 and 7, for §S4.

// The answers are mpmath's, to 17 digits, as the fixture prints them.
#![allow(clippy::excessive_precision)]

use bund2_api::Vm as _;
use bund2_interp::Interp;
use bund2_value::BundValue;

use super::WORDS;

fn run(src: &str) -> Result<Interp, String> {
    let mut i = Interp::new();
    crate::register_all(&mut i.registry);
    let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
    i.eval(&stream).map_err(|e| {
        e.0.rsplit_once(" returned error: ")
            .map_or(e.0.clone(), |(_, m)| m.to_string())
    })?;
    Ok(i)
}

fn value(src: &str) -> f64 {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src} failed: {e}"));
    match i.pull().as_ref().map(BundValue::unboxed) {
        Some(BundValue::Float(x, _)) => *x,
        other => panic!("{src}: {other:?}"),
    }
}

fn err(src: &str) -> String {
    match run(src) {
        Ok(i) => panic!("{src} was expected to fail: {:?}", i.snapshot()),
        Err(e) => e,
    }
}

/// Relative error within `tol`.
fn near(src: &str, want: f64, tol: f64) {
    let got = value(src);
    let rel = if want == 0.0 {
        got.abs()
    } else {
        ((got - want) / want).abs()
    };
    assert!(rel <= tol, "{src}: {got} against {want}, relative {rel:e}");
}

// Criterion 2.

#[test]
fn every_word_is_registered_with_its_dot_form() {
    let mut i = Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
        assert!(i.registry.effect_of(&format!("{w}.")).is_some(), "{w}.");
    }
    let ours: Vec<String> = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| WORDS.contains(&n.trim_end_matches('.')))
        .collect();
    assert_eq!(ours.len(), 2 * WORDS.len(), "{ours:?}");
}

// Criterion 7.

#[test]
fn gamma_and_its_logarithm() {
    near("0.5 math.gamma", std::f64::consts::PI.sqrt(), 1e-12);
    let g = value("0.5 math.gamma");
    assert!((g * g / std::f64::consts::PI - 1.0).abs() <= 1e-12, "{g}");
    near("5 math.gamma", 24.0, 1e-12);
    near("-0.5 math.gamma", -3.5449077018110321, 1e-12);
    near("171.5 math.gamma", 9.4833675668247993e307, 1e-12);
    near("171.5 math.lgamma", 709.143163030928, 1e-12);
    near("-0.5 math.lgamma", 1.26551212348464, 1e-12);
    near("1 math.digamma", -0.57721566490153286, 1e-12);
    near("-0.5 math.digamma", 0.036489973978576521, 1e-10);
}

#[test]
fn the_overflow_edge_of_gamma() {
    assert_eq!(
        err("172 math.gamma"),
        "MATH.GAMMA: E114 the answer overflows a FLOAT: Γ(172)"
    );
    assert_eq!(
        err("710 math.expm1"),
        "MATH.EXPM1: E114 the answer overflows a FLOAT: exp(710) − 1"
    );
}

#[test]
fn the_poles_are_e101() {
    assert_eq!(
        err("0 math.lgamma"),
        "MATH.LGAMMA: E101 x is outside math.lgamma's domain: 0"
    );
    assert_eq!(
        err("-3.0 math.gamma"),
        "MATH.GAMMA: E101 x is outside math.gamma's domain: -3"
    );
    assert_eq!(
        err("-2 math.digamma"),
        "MATH.DIGAMMA: E101 x is outside math.digamma's domain: -2"
    );
}

#[test]
fn choose_exactly_and_past_u128() {
    near("10 3 math.choose", 120.0, 0.0);
    near("10 0 math.choose", 1.0, 0.0);
    near("10 10 math.choose", 1.0, 0.0);
    near("100 50 math.choose", 1.0089134454556419e29, 1e-15);
    let (choose, lchoose) = (
        fixture("math.choose 1000 500"),
        fixture("math.lchoose 1000 500"),
    );
    near("1000 500 math.choose", choose, 1e-12);
    near("1000 500 math.lchoose", lchoose, 1e-12);
    near("100 math.lfactorial", 363.73937555556349, 1e-12);
    near("0 math.lfactorial", 0.0, 0.0);
    assert_eq!(
        err("3 5 math.choose"),
        "MATH.CHOOSE: E101 k is outside math.choose's domain: 5"
    );
    assert_eq!(
        err("-3 1 math.lchoose"),
        "MATH.LCHOOSE: E101 n is outside math.lchoose's domain: -3"
    );
    assert_eq!(
        err("10 2.0 math.choose"),
        "MATH.CHOOSE: E101 k is outside math.choose's domain: 2.0"
    );
    assert!(err("100000 50000 math.choose").contains("E114"));
}

#[test]
fn erf_against_abramowitz_and_stegun() {
    // Table 7.1, erf 0.5 = 0.5204998778, to the table's ten digits.
    let y = value("0.5 math.erf");
    assert!((y - 0.5204998778).abs() < 5e-11, "{y}");
    near("0.5 math.erfc", 0.47950012218695346, 1e-12);
    near("3 math.erfc", 2.2090496998585441e-5, 1e-12);
    near("0.5 math.erf", 0.52049987781304654, 1e-12);
    near("0.5 math.erfinv", 0.47693627620446987, 1e-12);
    assert_eq!(
        err("1 math.erfinv"),
        "MATH.ERFINV: E101 x is outside math.erfinv's domain: 1"
    );
}

#[test]
fn beta_and_the_logarithms() {
    near("2.5 3.5 math.beta", 0.03681553890925539, 1e-12);
    near("2.5 3.5 math.lbeta", -3.3018352699620526, 1e-12);
    near("8 math.log2", 3.0, 0.0);
    near("0.0000000001 math.log1p", 9.9999999995e-11, 1e-12);
    near("0.0000000001 math.expm1", 1.00000000005e-10, 1e-12);
    assert_eq!(
        err("0 math.log2"),
        "MATH.LOG2: E101 x is outside math.log2's domain: 0"
    );
    assert_eq!(
        err("-1 math.log1p"),
        "MATH.LOG1P: E101 x is outside math.log1p's domain: -1"
    );
    assert_eq!(
        err("0 1 math.beta"),
        "MATH.BETA: E101 a is outside math.beta's domain: 0"
    );
}

/// The committed fixture's answer for a line's leading words.
fn fixture(key: &str) -> f64 {
    let line = include_str!("../fixtures/special.txt")
        .lines()
        .find(|l| l.starts_with(&format!("{key} ")))
        .unwrap_or_else(|| panic!("{key} is not in the fixture"));
    line.rsplit(' ')
        .next()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| panic!("{line}"))
}

#[test]
fn betainc_and_gammainc_against_the_fixture() {
    let mut seen = 0;
    for line in include_str!("../fixtures/special.txt").lines() {
        let parts: Vec<&str> = line.split(' ').collect();
        let (word, args, want) = (parts[0], &parts[1..parts.len() - 1], parts[parts.len() - 1]);
        if word != "math.betainc" && word != "math.gammainc" {
            continue;
        }
        let want: f64 = want.parse().unwrap();
        near(&format!("{} {word}", args.join(" ")), want, 1e-8);
        seen += 1;
    }
    assert_eq!(seen, 13);
    near("0 2 math.gammainc", 0.0, 0.0);
    near("0 2 3 math.betainc", 0.0, 0.0);
    near("1 2 3 math.betainc", 1.0, 0.0);
}

#[test]
fn the_incomplete_functions_refuse_at_their_caps() {
    assert_eq!(
        err("100000000000000000.0 100000000000000000.0 math.gammainc"),
        "MATH.GAMMAINC: E110 the incomplete gamma did not converge within 1000000 iterations"
    );
    assert_eq!(
        err("0.5 1000000000000000.0 1000000000000000.0 math.betainc"),
        "MATH.BETAINC: E110 the incomplete beta's continued fraction did not converge \
         within 100000 iterations"
    );
    assert_eq!(
        err("1.5 2 3 math.betainc"),
        "MATH.BETAINC: E101 x is outside math.betainc's domain: 1.5"
    );
    assert_eq!(
        err("-1 2 math.gammainc"),
        "MATH.GAMMAINC: E101 x is outside math.gammainc's domain: -1"
    );
    assert_eq!(
        err("1 0 math.gammainc"),
        "MATH.GAMMAINC: E101 a is outside math.gammainc's domain: 0"
    );
}

// B15: an INTEGER converts; the reference's unary words still refuse one.

#[test]
fn an_integer_operand_converts() {
    near("4 math.lgamma", 6.0f64.ln(), 1e-12);
    assert!(err("4 math.sqrt").contains("not float"));
}

// Criterion 5: no abort, whatever the operand.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "float.+Inf",
        "float.-Inf",
        "-0.1",
        "1.1",
        "0",
        "-1",
        "1",
        "0.5",
        "-0.5",
        "172",
        "710",
        "10.0 308.0 math.power",
        "-10.0 307.0 math.power",
        "0.1 323.0 math.power",
        "\"x\"",
        "true",
    ];
    for w in WORDS {
        let n = match w {
            "math.betainc" => 3,
            "math.beta" | "math.lbeta" | "math.gammainc" | "math.choose" | "math.lchoose" => 2,
            _ => 1,
        };
        for o in operands {
            let src = format!("{} {w}", vec![o; n].join(" "));
            match run(&src) {
                Ok(mut i) => match i.pull().as_ref().map(BundValue::unboxed) {
                    Some(BundValue::Float(y, _)) => assert!(y.is_finite(), "{src}: {y}"),
                    other => panic!("{src}: {other:?}"),
                },
                Err(e) => assert!(
                    e.starts_with(&format!("{}: E1", w.to_uppercase())),
                    "{src}: {e}"
                ),
            }
        }
    }
}

#[test]
fn a_shallow_stack_is_the_house_text() {
    assert_eq!(
        err("1 math.beta"),
        "Stack is too shallow for inline math.beta"
    );
}

// The `.` forms: the top operand from the workbench, the answer there.

#[test]
fn each_dot_form_answers_on_the_workbench() {
    for (args, top, w) in [
        ("", "0.5", "math.gamma"),
        ("2.5", "3.5", "math.beta"),
        ("0.5 2", "3", "math.betainc"),
        ("10", "3", "math.choose"),
    ] {
        let plain = value(&format!("{args} {top} {w}"));
        let mut i = run(&format!("7 {args} {top} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        match i.pull_workbench().as_ref().map(BundValue::unboxed) {
            Some(BundValue::Float(y, _)) => assert_eq!(*y, plain, "{w}."),
            other => panic!("{w}.: {other:?}"),
        }
    }
}

// Criterion 6.

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":math.gamma { 42 } register 0.5 math.gamma").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
