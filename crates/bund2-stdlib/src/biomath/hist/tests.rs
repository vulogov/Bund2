//! RFC-0020 criteria 1, 8 and 11, for §S10's `stat.hist` and `pretty`.
//!
//! `hist_r.txt` is R 4.6.2's answers and inputs, written once by `hist.R`.
//! No test runs R.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::{WORDS, pretty, signif};
use crate::biomath::testkit::{err, fixture, floats, run};

const R: &str = include_str!("../fixtures/hist_r.txt");

fn lit(x: f64) -> String {
    let s = format!("{x:?}");
    match s.find('e') {
        Some(i) if !s[..i].contains('.') => format!("{}.0{}", &s[..i], &s[i..]),
        _ => s,
    }
}

fn list(xs: &[f64]) -> String {
    let cells: Vec<String> = xs.iter().map(|x| lit(*x)).collect();
    format!("[ {} ]", cells.join(" "))
}

fn answer(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    v.as_map()
        .unwrap_or_else(|| panic!("not a MAP: {v:?}"))
        .clone()
}

fn get(m: &BTreeMap<String, BundValue>, k: &str) -> Vec<f64> {
    floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")))
}

fn has(label: &str) -> bool {
    R.lines().any(|l| l.starts_with(&format!("{label} [")))
}

/// breaks, counts and mids exactly, density at 10⁻⁹|b| + 10⁻¹², equidist.
fn against(label: &str, m: &BTreeMap<String, BundValue>) {
    for k in ["breaks", "counts", "mids"] {
        let want = fixture(R, &format!("{label}.{k}"));
        let got = get(m, k);
        assert_eq!(
            got.len(),
            want.len(),
            "{label}.{k}: {got:?} against {want:?}"
        );
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            assert!(g == w, "{label}.{k}[{i}]: {g:?} against {w:?}");
        }
    }
    let want = fixture(R, &format!("{label}.density"));
    let got = get(m, "density");
    assert_eq!(got.len(), want.len(), "{label}.density");
    for (i, (g, w)) in got.iter().zip(&want).enumerate() {
        assert!(
            (g - w).abs() <= 1e-9 * w.abs() + 1e-12,
            "{label}.density[{i}]: {g} against {w}"
        );
    }
    let eq = m
        .get("equidist")
        .and_then(|v| match v.unboxed() {
            BundValue::Bool(b, _) => Some(*b),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{label}: no equidist"));
    assert_eq!(
        f64::from(u8::from(eq)),
        fixture(R, &format!("{label}.equidist"))[0],
        "{label}.equidist"
    );
}

fn hist(x: &str, opts: &str) -> BTreeMap<String, BundValue> {
    answer(&format!("{} dict {opts} stat.hist", list(&fixture(R, x))))
}

// Criterion 1.

#[test]
fn stat_hist_is_registered_fixed_with_its_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
    }
    let effect = |w: &str| format!("{:?}", i.registry.effect_of(w));
    assert_eq!(effect("stat.hist"), effect("stat.test.hotelling1"));
    assert_eq!(effect("stat.hist."), effect("stat.test.hotelling1."));
}

// Criterion 8.

#[test]
fn each_rule_and_count_gives_rs_histogram() {
    for x in ["x10", "x1000", "spike"] {
        for rule in ["sturges", "scott", "fd"] {
            let label = format!("{x}.{rule}");
            let opts = if rule == "sturges" {
                String::new()
            } else {
                format!("\"breaks\" \"{rule}\" set")
            };
            against(&label, &hist(x, &opts));
            if rule == "sturges" {
                against(&label, &hist(x, "\"breaks\" \"sturges\" set"));
            }
        }
    }
    for x in ["x10", "x1000"] {
        for k in [1, 3, 7, 25] {
            against(
                &format!("{x}.count{k}"),
                &hist(x, &format!("\"breaks\" {k} set")),
            );
        }
    }
}

#[test]
fn given_breaks_give_rs_histogram() {
    for (label, breaks, opts) in [
        ("x10.list", "[ 0 2.5 5 7.5 11 ]", ""),
        ("x10.uneven", "[ 1 2.5 3 9.1 10.4 ]", ""),
        (
            "x10.uneven.left",
            "[ 1 2.5 3 9.1 10.4 ]",
            "\"right\" false set",
        ),
    ] {
        against(
            label,
            &hist("x10", &format!("\"breaks\" {breaks} set {opts}")),
        );
    }
    let tb = "\"breaks\" [ 1 2 3 4 ] set";
    against("tied.list.right", &hist("tied", tb));
    against(
        "tied.list.left",
        &hist("tied", &format!("{tb} \"right\" false set")),
    );
    against("tied.right", &hist("tied", ""));
    against("tied.left", &hist("tied", "\"right\" false set"));
    against("fuzz", &hist("fuzz", "\"breaks\" [ 0 0.1 0.2 0.3 1 ] set"));
}

#[test]
fn a_constant_and_a_single_value_give_rs_histogram() {
    against("constant", &answer("[ 5 5 5 ] dict stat.hist"));
    against("single", &answer("[ 7 ] dict stat.hist"));
}

#[test]
fn rs_stops_are_e101() {
    assert!(has("x10.sturges.breaks"));
    for (src, want) in [
        (
            format!(
                "{} dict \"breaks\" [ 2 5 9 ] set stat.hist",
                list(&fixture(R, "x10"))
            ),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: breaks from 2 to 9, which leave 3 of x's 10 values uncounted",
        ),
        (
            format!(
                "{} dict \"breaks\" [ 1 2 3 4 ] set \"include.lowest\" false set stat.hist",
                list(&fixture(R, "tied"))
            ),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: breaks from 1 to 4, which leave 1 of x's 7 values uncounted",
        ),
        (
            format!(
                "{} dict \"breaks\" [ 1 2 3 4 ] set \"include.lowest\" false set \"right\" false set stat.hist",
                list(&fixture(R, "tied"))
            ),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: breaks from 1 to 4, which leave 1 of x's 7 values uncounted",
        ),
        (
            "[ 7 ] dict \"breaks\" \"scott\" set stat.hist".to_string(),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: \"scott\", on a series of 1 value, which has no variance",
        ),
        (
            "[ 7 ] dict \"breaks\" \"fd\" set stat.hist".to_string(),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: \"fd\", on a series of 1 value, which has no variance",
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
    for label in [
        "x10.short.stop",
        "tied.list.right.open.stop",
        "single.scott.stop",
    ] {
        assert!(R.lines().any(|l| l.starts_with(label)), "{label}");
    }
}

#[test]
fn bund2s_own_refusals_are_their_templates() {
    let x = "[ 1 2 3 ]";
    for (src, want) in [
        (
            format!("{x} dict \"breaks\" [ 0 2 2 4 ] set stat.hist"),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: list/4 [0, 2, 2, 4], not strictly increasing at 3",
        ),
        (
            format!("{x} dict \"breaks\" [ 4 0 ] set stat.hist"),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: list/2 [4, 0], not strictly increasing at 2",
        ),
        (
            format!("{x} dict \"breaks\" [ 4 ] set stat.hist"),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: list/1 [4]",
        ),
        (
            format!("{x} dict \"breaks\" 0 set stat.hist"),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: 0",
        ),
        (
            format!("{x} dict \"breaks\" 1000001 set stat.hist"),
            "STAT.HIST: E111 a class count of 1000001 exceeds its cap of 1000000",
        ),
        (
            format!("{x} dict \"breaks\" \"Sturges\" set stat.hist"),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: \"Sturges\"",
        ),
        (
            format!("{x} dict \"breaks\" 2.5 set stat.hist"),
            "STAT.HIST: E101 breaks is outside stat.hist's domain: 2.5",
        ),
        (
            format!("{x} dict \"include.lowest\" false set stat.hist"),
            "STAT.HIST: E101 include.lowest is outside stat.hist's domain: false, where breaks is a rule or a count",
        ),
        (
            format!("{x} dict \"breaks\" 3 set \"include.lowest\" false set stat.hist"),
            "STAT.HIST: E101 include.lowest is outside stat.hist's domain: false, where breaks is a rule or a count",
        ),
        (
            format!("{x} dict \"right\" 1 set stat.hist"),
            "STAT.HIST: E101 right is outside stat.hist's domain: 1",
        ),
        (
            format!("{x} dict \"plot\" true set stat.hist"),
            "STAT.HIST: E101 opts is outside stat.hist's domain: an unknown key \"plot\"",
        ),
        (
            "list dict stat.hist".to_string(),
            "STAT.HIST: E101 x is outside stat.hist's domain: an empty series",
        ),
        (
            // A word inside a LIST literal is not evaluated: E103.
            "[ 1 float.NaN ] dict stat.hist".to_string(),
            "STAT.HIST: E103 x is not a LIST of numbers: float.NaN",
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}

#[test]
fn include_lowest_true_given_with_a_rule_answers() {
    let a = answer("[ 1 2 3 ] dict \"include.lowest\" true set stat.hist");
    let b = answer("[ 1 2 3 ] dict stat.hist");
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}

#[test]
fn the_ported_pretty_equals_rs() {
    for label in [
        "a", "a10", "ones", "zeros", "wide", "tiny", "close", "neg", "huge",
    ] {
        let input = fixture(R, &format!("pretty.{label}.in"));
        let want = fixture(R, &format!("pretty.{label}"));
        let got = pretty(input[0], input[1], input[2] as i32, input[3] as i32)
            .unwrap_or_else(|e| panic!("{label}: {}", e.0));
        assert_eq!(got.len(), want.len(), "{label}: {got:?} against {want:?}");
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            assert!(g == w, "pretty.{label}[{i}]: {g:?} against {w:?}");
        }
    }
}

#[test]
fn signif_rounds_as_fprec() {
    for (x, want) in [
        (123456.789, 123460.0),
        (-0.000123456789, -0.00012346),
        (2.5, 2.5),
        (1.234565, 1.2346),
        (0.0, 0.0),
        // R 4.6.2: signif(1e300 * 1.234567, 5) is 1.2346000000000006e+300.
        (1e300 * 1.234567, 1.2346000000000006e300),
    ] {
        assert_eq!(signif(x, 5), want, "{x}");
    }
}

// Criterion 11.

#[test]
fn no_operand_aborts() {
    let xs = [
        "float.NaN",
        "list",
        "dict",
        "[ 1 ]",
        "[ 0 0 0 ]",
        "[ 1.0e307 -1.0e307 ]",
        "[ 1.7e308 -1.7e308 3 ]",
        "[ 1.0e-300 2.0e-300 ]",
        "[ 5.0e-324 1.0e-323 ]",
        "[ 1.0e160 1 2 3 4 5 6 7 ]",
        "[ 1 \"a\" ]",
        "[ 1 na ]",
        "[ 1 2 3 4 5 6 7 8 9 10 ]",
    ];
    let opts = [
        "dict",
        "na",
        "1",
        "dict \"breaks\" \"scott\" set",
        "dict \"breaks\" \"fd\" set",
        "dict \"breaks\" 1 set",
        "dict \"breaks\" 1000000 set",
        "dict \"breaks\" [ -1.7e308 0 1.7e308 ] set",
        "dict \"breaks\" [ 0 5.0e-324 1 ] set",
        "dict \"breaks\" [ 0 float.Inf ] set",
        "dict \"breaks\" list set",
        "dict \"right\" false set",
        "dict \"breaks\" [ 0 1 2 3 4 5 6 7 8 9 10 ] set \"include.lowest\" false set",
    ];
    let ok = |src: &str| {
        if let Err(e) = run(src) {
            assert!(
                e.contains(": E1") || e.contains("too shallow"),
                "{src}: {e}"
            );
        }
    };
    for a in xs {
        for o in opts {
            ok(&format!("{a} {o} stat.hist"));
            ok(&format!("{a} {o} . stat.hist."));
        }
        ok(&format!("{a} stat.hist"));
    }
}

#[test]
fn the_dot_form_answers_on_the_workbench() {
    let args = "[ 1 2 2 3 3 3 4 ] dict";
    let plain = run(&format!("{args} stat.hist")).unwrap().pull().unwrap();
    let mut i = run(&format!("7 {args} . stat.hist.")).unwrap();
    let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
    assert_eq!(shown, ["7"]);
    let got = i.pull_workbench().unwrap();
    assert_eq!(format!("{got:?}"), format!("{plain:?}"));
}

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":stat.hist { 42 } register 1 stat.hist").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
