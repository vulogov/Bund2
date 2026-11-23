//! RFC-0019 criterion 4, and 1 and 14, for §S4.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{err, floats, run, run_on};

const POWER: &str = include_str!("../fixtures/power.txt");
const R: &str = include_str!("../fixtures/power_r.txt");

fn answer(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    assert_eq!(i.depth(), 0, "{src}");
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn key(m: &BTreeMap<String, BundValue>, k: &str) -> f64 {
    floats(&m[k])[0]
}

/// A fixture's lines: the program, the key, the value.
fn lines(text: &str) -> Vec<(&str, &str, f64)> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let (program, rest) = l.split_once(" | ").unwrap_or_else(|| panic!("{l}"));
            let (k, v) = rest.split_once(' ').unwrap_or_else(|| panic!("{l}"));
            (program, k, v.parse().unwrap_or_else(|_| panic!("{l}")))
        })
        .collect()
}

fn relative(got: f64, want: f64) -> f64 {
    ((got - want) / want).abs()
}

// Criterion 1.

#[test]
fn every_word_is_registered_with_its_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        assert!(!e.opaque, "{w}");
        assert_eq!((e.consumes, e.produces), (1, 1), "{w}");
        let d = i
            .registry
            .effect_of(&format!("{w}."))
            .unwrap_or_else(|| panic!("{w}."));
        assert_eq!((d.consumes, d.produces), (0, 0), "{w}.");
    }
    let ours: Vec<String> = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("power."))
        .collect();
    assert_eq!(ours.len(), WORDS.len(), "{ours:?}");
}

// Criterion 4.

#[test]
fn each_answer_matches_the_fixture() {
    let all = lines(POWER);
    assert_eq!(all.len(), 27);
    for (program, k, want) in all {
        let m = answer(program);
        let got = key(&m, k);
        assert!(
            relative(got, want) <= 1e-9,
            "{program}: {k} {got:e} against {want:e}, {:e}",
            relative(got, want)
        );
    }
}

#[test]
fn each_solved_parameter_put_back_gives_its_power() {
    for (program, k, _) in lines(POWER).into_iter().filter(|l| l.1 != "power") {
        let m = answer(program);
        let target = key(&m, "power");
        // The same call with the root given and the power left out.
        let word = program.rsplit(' ').next().unwrap_or_default();
        let opts = program.trim_end_matches(word);
        let src = format!("{opts}\"{k}\" {:?} set \"power\" na set {word}", key(&m, k));
        let back = key(&answer(&src), "power");
        assert!(
            (back - target).abs() <= 1e-9 * target,
            "{src}: {back:e} against {target:e}"
        );
    }
}

/// x as R's `format(x, digits = 7)`: 7 significant digits, or the integer.
fn signif7(x: f64) -> f64 {
    let e = x.abs().log10().floor() as i32;
    // `format` keeps every digit before the point.
    if e >= 6 {
        return x.round();
    }
    let scale = 10f64.powi(6 - e);
    (x * scale).round() / scale
}

/// R's printed values that N7's tolerance reproduces: every power, and
/// each solved value whose seven digits R's ε^¼ leaves unmoved.
#[test]
fn r_prints_the_same_digits() {
    let all = lines(R);
    assert_eq!(all.len(), 11);
    let mut same = 0;
    for (program, k, want) in &all {
        let got = signif7(key(&answer(program), k));
        if relative(got, *want) <= 1e-12 {
            same += 1;
        } else {
            // R's root is within its tolerance ε^¼ ≈ 1.2e-4, absolute, of
            // the exact one (`power.R:23`), Bund2's within 1e-10 relative.
            assert!(*k != "power", "{program}: {got} against R's {want}");
            assert!(
                (got - want).abs() <= f64::EPSILON.powf(0.25),
                "{program}: {k} {got} against R's {want}"
            );
        }
    }
    assert_eq!(same, 6, "R's printed digits reproduced");
}

#[test]
fn each_answer_holds_every_parameter() {
    let m = answer("dict \"n\" 20 set \"delta\" 1.0 set power.t");
    let keys: Vec<&str> = m.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "delta",
            "method",
            "n",
            "power",
            "sd",
            "sides",
            "sig.level",
            "strict",
            "type"
        ]
    );
    // n is a FLOAT, as R answers it.
    assert!(matches!(m["n"].unboxed(), BundValue::Float(x, _) if *x == 20.0));
    assert_eq!(
        m["method"].as_str().as_deref(),
        Some("Two-sample t test power calculation")
    );
    let m = answer("dict \"w\" 0.3 set \"n\" 100 set \"df\" 2 set power.chisq");
    assert_eq!(m["df"].as_int(), Some(2));
}

#[test]
fn two_sided_takes_delta_as_its_size() {
    let a = answer("dict \"n\" 20 set \"delta\" -1.0 set power.t");
    let b = answer("dict \"n\" 20 set \"delta\" 1.0 set power.t");
    assert_eq!(key(&a, "power").to_bits(), key(&b, "power").to_bits());
    assert_eq!(key(&a, "delta"), 1.0);
}

#[test]
fn the_dot_form_takes_opts_from_the_workbench() {
    let opts = answer("dict \"n\" 20 set \"delta\" 1.0 set");
    let opts = BundValue::map(opts);
    let mut i = run_on(&[], Some(&opts), "power.t.").unwrap();
    let wb = i
        .pull_workbench()
        .unwrap_or_else(|| panic!("empty workbench"));
    assert_eq!(i.depth(), 0);
    let plain = answer("dict \"n\" 20 set \"delta\" 1.0 set power.t");
    let got = wb.as_map().unwrap_or_else(|| panic!("{wb:?}"));
    assert_eq!(key(got, "power").to_bits(), key(&plain, "power").to_bits());
}

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            "dict \"w\" 0.3 set \"n\" 100 set power.chisq",
            "POWER.CHISQ: E101 df is outside power.chisq's domain: left out; df cannot be solved for",
        ),
        (
            "dict \"w\" 0.3 set \"n\" 100 set \"df\" na set \"power\" 0.8 set power.chisq",
            "POWER.CHISQ: E101 df is outside power.chisq's domain: left out; df cannot be solved for",
        ),
        (
            "dict \"w\" 0.3 set \"n\" 100 set \"df\" 2.0 set power.chisq",
            "POWER.CHISQ: E101 df is outside power.chisq's domain: 2.0",
        ),
        (
            "dict \"n\" 20 set \"delta\" 0.0 set \"sd\" na set \"power\" 0.8 set power.t",
            "POWER.T: E101 delta is outside power.t's domain: 0.0, which leaves sd no interval",
        ),
        (
            "dict \"groups\" 4 set \"n\" 5 set \"between.var\" 0.0 set \"power\" 0.8 set power.anova",
            "POWER.ANOVA: E101 between.var is outside power.anova's domain: 0.0, which leaves within.var no interval",
        ),
        (
            "dict \"groups\" 4 set \"n\" 5 set \"within.var\" 0.0 set \"power\" 0.8 set power.anova",
            "POWER.ANOVA: E101 within.var is outside power.anova's domain: 0.0",
        ),
        (
            "dict \"n\" 20 set \"delta\" 1.0 set \"power\" 0.9 set power.t",
            "POWER.T: E101 opts is outside power.t's domain: none of n, delta, sd, sig.level, power is left out; exactly one must be",
        ),
        (
            "dict \"delta\" 1.0 set power.t",
            "POWER.T: E101 opts is outside power.t's domain: n and power are left out; exactly one of n, delta, sd, sig.level, power must be",
        ),
        (
            "dict \"n\" 20 set \"delta\" 1.0 set \"alternative\" \"less\" set power.t",
            "POWER.T: E101 opts is outside power.t's domain: an unknown key \"alternative\"",
        ),
        (
            "dict \"n\" 20 set \"delta\" 1.0 set \"sides\" 3 set power.t",
            "POWER.T: E101 sides is outside power.t's domain: 3",
        ),
        (
            "dict \"n\" 20 set \"delta\" 1.0 set \"strict\" 1 set power.t",
            "POWER.T: E101 strict is outside power.t's domain: 1",
        ),
        (
            "dict \"n\" 20 set \"delta\" 1.0 set \"type\" \"welch\" set power.t",
            "POWER.T: E101 type is outside power.t's domain: \"welch\"",
        ),
        (
            "dict \"n\" 1 set \"delta\" 1.0 set power.t",
            "POWER.T: E101 n is outside power.t's domain: 1",
        ),
        (
            "dict \"n\" 20 set \"delta\" 1.0 set \"sig.level\" 1.0 set power.t",
            "POWER.T: E101 sig.level is outside power.t's domain: 1.0",
        ),
        (
            "dict \"n\" 50 set \"p1\" 0.5 set \"p2\" 1.0 set power.prop",
            "POWER.PROP: E101 p2 is outside power.prop's domain: 1.0",
        ),
        (
            "dict \"n\" 20 set \"delta\" \"big\" set power.t",
            "POWER.T: E101 delta is outside power.t's domain: \"big\"",
        ),
        (
            "dict \"n\" 20 set \"delta\" 1.0 set \"sd\" na set \"power\" na set power.t",
            "POWER.T: E101 opts is outside power.t's domain: sd and power are left out; exactly one of n, delta, sd, sig.level, power must be",
        ),
        (
            "dict \"groups\" 1 set \"n\" 5 set \"between.var\" 1.0 set \"within.var\" 3.0 set power.anova",
            "POWER.ANOVA: E101 groups is outside power.anova's domain: 1",
        ),
        (
            "1.0 power.t",
            "POWER.T: E101 opts is outside power.t's domain: 1.0",
        ),
        // R's own example of no p2 in (p1, 1), which R warns of and §S4
        // does not extend past.
        (
            "dict \"n\" 30 set \"p1\" 0.9 set \"power\" 0.8 set power.prop",
            "POWER.PROP: E110 the search for a sign change did not converge within 1000 iterations",
        ),
        // A power above every n's: n's extension runs to its cap.
        (
            "dict \"delta\" 0.0 set \"power\" 0.9 set power.t",
            "POWER.T: E110 the search for a sign change did not converge within 1000 iterations",
        ),
    ] {
        assert_eq!(err(src), want, "{src}");
    }
}

// Criterion 14.

#[test]
fn nothing_aborts() {
    let values = [
        "float.NaN",
        "float.+Inf",
        "float.-Inf",
        "-1",
        "0",
        "1",
        "0.5",
        "2.0",
        "1.0e7",
        "\"x\"",
        "na",
        "list",
        "dict",
        "[ 1.0 ]",
        "{ 1 }",
        "true",
        "nodata",
    ];
    let keys = [
        "n",
        "delta",
        "sd",
        "sig.level",
        "power",
        "p1",
        "p2",
        "groups",
        "between.var",
        "within.var",
        "w",
        "df",
        "sides",
        "strict",
        "type",
    ];
    let bases = [
        "dict \"n\" 20 set \"delta\" 1.0 set",
        "dict \"delta\" 1.0 set \"power\" 0.9 set",
        "dict \"n\" 50 set \"p1\" 0.5 set \"p2\" 0.75 set",
        "dict \"groups\" 4 set \"n\" 5 set \"between.var\" 1.0 set \"within.var\" 3.0 set",
        "dict \"w\" 0.3 set \"n\" 100 set \"df\" 2 set",
    ];
    let ok = |src: &str, r: Result<bund2_interp::Interp, String>| {
        if let Err(e) = r {
            assert!(
                e.contains(": E1") || e.contains("too shallow"),
                "{src}: {e}"
            );
        }
    };
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        ok(w, run(w));
        for v in values {
            let src = format!("{v} {w}");
            ok(&src, run(&src));
        }
        for base in bases {
            for k in keys {
                for v in values {
                    let src = format!("{base} \"{k}\" {v} set {w}");
                    ok(&src, run(&src));
                }
            }
        }
    }
}

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":power.t { 42 } register 1 power.t").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
