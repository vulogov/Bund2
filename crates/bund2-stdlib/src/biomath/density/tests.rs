//! RFC-0020 criteria 1, 9 and 11, for §S9's `stat.bw` and `stat.density`.
//!
//! `density_r.txt` is R 4.6.2's answers and inputs, written once by
//! `density.R`. No test runs R.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::{WORDS, approx1, dnorm, fft};
use crate::biomath::testkit::{err, fixture, floats, run};

const R: &str = include_str!("../fixtures/density_r.txt");

const KERNELS: [&str; 7] = [
    "gaussian",
    "epanechnikov",
    "rectangular",
    "triangular",
    "biweight",
    "cosine",
    "optcosine",
];

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

fn sample(label: &str) -> String {
    list(&fixture(R, label))
}

fn stop(label: &str) -> String {
    R.lines()
        .find_map(|l| l.strip_prefix(&format!("{label}.stop ")))
        .unwrap_or_else(|| panic!("no {label}.stop"))
        .to_string()
}

fn top(src: &str) -> f64 {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    floats(&i.pull().unwrap_or_else(|| panic!("{src}: empty")))[0]
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

fn rel(label: &str, got: f64, want: f64, tol: f64) {
    assert!(
        (got - want).abs() <= tol * want.abs(),
        "{label}: {got:?} against R's {want:?}"
    );
}

#[test]
fn every_form_is_registered() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("stat.bw") || n.starts_with("stat.density"))
        .count();
    assert_eq!(ours, WORDS.len());
}

/// Criterion 9's first half: each rule at 10⁻¹² relative on x₁₀ and x₁₀₀₀,
/// and either side of `bw.SJ`'s n > 500 switch.
#[test]
fn bw_matches_r_on_each_rule_and_either_side_of_the_switch() {
    for s in ["x10", "x1000", "x500", "x501"] {
        let x = match s {
            "x500" => list(&fixture(R, "x1000")[..500]),
            "x501" => list(&fixture(R, "x1000")[..501]),
            _ => sample(s),
        };
        for (rule, label) in [
            ("nrd0", "nrd0"),
            ("nrd", "nrd"),
            ("sj-ste", "ste"),
            ("sj", "ste"),
            ("sj-dpi", "dpi"),
        ] {
            let want = fixture(R, &format!("bw.{label}.{s}"))[0];
            let got = top(&format!("{x} \"{rule}\" stat.bw"));
            rel(&format!("{rule} on {s}"), got, want, 1e-12);
        }
    }
}

#[test]
fn nrd0_falls_back_as_r_does() {
    for s in ["spike", "const", "zeros"] {
        let want = fixture(R, &format!("bw.nrd0.{s}"))[0];
        rel(
            s,
            top(&format!("{} \"nrd0\" stat.bw", sample(s))),
            want,
            1e-15,
        );
    }
    // |x₁| is the first value as given: 5 then 0 against 0 then 5 is no
    // longer constant, so this asks sd's arm instead; a 0 first with sd 0 is
    // `zeros`, which falls through to 1.
    rel(
        "-5 first",
        top("[ -5 -5 -5 ] \"nrd0\" stat.bw"),
        fixture(R, "bw.nrd0.const")[0],
        1e-15,
    );
}

#[test]
fn where_r_stops_or_answers_0_stat_bw_is_e101() {
    assert!(fixture(R, "bw.nrd.spike")[0] == 0.0);
    assert!(stop("bw.ste.spike").contains("too sparse"));
    assert!(stop("bw.ste.const").contains("constant"));
    assert!(stop("bw.dpi.const").contains("too sparse"));
    assert!(stop("bw.nrd0.one").contains("at least 2"));
    let spike = sample("spike");
    let c600 = list(&[5.0; 600]);
    for (src, want) in [
        (
            format!("{spike} \"nrd\" stat.bw"),
            "STAT.BW: E101 rule is outside stat.bw's domain: \"nrd\", which is 0 on an x whose IQR or sd is 0",
        ),
        (
            format!("{spike} \"sj\" stat.bw"),
            "STAT.BW: E101 rule is outside stat.bw's domain: \"sj\", on a sample too sparse to find TD",
        ),
        (
            format!("{spike} \"sj-dpi\" stat.bw"),
            "STAT.BW: E101 rule is outside stat.bw's domain: \"sj-dpi\", on a sample too sparse to find TD",
        ),
        (
            "[ 5 5 5 ] \"sj\" stat.bw".to_string(),
            "STAT.BW: E101 series is outside stat.bw's domain: constant data",
        ),
        (
            format!("{c600} \"sj-dpi\" stat.bw"),
            "STAT.BW: E101 series is outside stat.bw's domain: constant data",
        ),
        (
            "[ 7 ] \"nrd0\" stat.bw".to_string(),
            "STAT.BW: E101 series is outside stat.bw's domain: a series of 1 value",
        ),
        (
            "[ 5.0e-324 0 ] \"sj\" stat.bw".to_string(),
            "STAT.BW: E101 series is outside stat.bw's domain: a range of 5e-324, too narrow to bin by 1000",
        ),
        (
            "list \"nrd0\" stat.bw".to_string(),
            "STAT.BW: E101 series is outside stat.bw's domain: an empty series",
        ),
        (
            "[ 1 2 ] \"ucv\" stat.bw".to_string(),
            "STAT.BW: E101 rule is outside stat.bw's domain: \"ucv\"",
        ),
    ] {
        assert_eq!(err(&src), want, "{src}");
    }
}

fn density(x: &str, opts: &str) -> BTreeMap<String, BundValue> {
    answer(&format!("{x} dict {opts} stat.density"))
}

/// y at 10⁻¹² absolute (§S9's FFT is not R's), x at 10⁻¹⁵ relative, bw at
/// 10⁻¹² relative, n exactly.
fn against(label: &str, grid: &str, m: &BTreeMap<String, BundValue>) {
    let want = fixture(R, &format!("{label}.y"));
    let got = get(m, "y");
    assert_eq!(got.len(), want.len(), "{label}.y");
    for (i, (g, w)) in got.iter().zip(&want).enumerate() {
        assert!(
            (g - w).abs() <= 1e-12,
            "{label}.y[{i}]: {g:?} against {w:?}"
        );
    }
    let want = fixture(R, &format!("{grid}.x"));
    for (i, (g, w)) in get(m, "x").iter().zip(&want).enumerate() {
        assert!(
            (g - w).abs() <= 1e-15 * w.abs().max(1.0),
            "{label}.x[{i}]: {g:?} against {w:?}"
        );
    }
    rel(
        &format!("{label}.bw"),
        get(m, "bw")[0],
        fixture(R, &format!("{label}.bw"))[0],
        1e-12,
    );
    assert_eq!(
        get(m, "n")[0],
        fixture(R, &format!("{label}.n"))[0],
        "{label}.n"
    );
}

/// Criterion 9's second half: every kernel, both samples, the default and a
/// given `n`, `from` and `to`.
#[test]
fn density_matches_r_for_each_kernel_on_both_samples() {
    for k in KERNELS {
        let kern = format!("\"kernel\" \"{k}\" set");
        for (s, given) in [
            ("x10", "\"n\" 60 set \"from\" 0.0 set \"to\" 12.0 set"),
            ("x1000", "\"n\" 40 set \"from\" -3.0 set \"to\" 3.0 set"),
        ] {
            let x = sample(s);
            let label = format!("d.{s}.{k}");
            against(&label, &format!("d.{s}.gaussian"), &density(&x, &kern));
            let label = format!("d.{s}.{k}.given");
            against(
                &label,
                &format!("d.{s}.gaussian.given"),
                &density(&x, &format!("{kern} {given}")),
            );
        }
    }
}

#[test]
fn density_matches_r_with_a_rule_adjust_cut_one_point_and_one_value() {
    let x = sample("x10");
    for (label, opts) in [
        ("d.x10.sj", "\"bw\" \"sj\" set \"n\" 50 set"),
        ("d.x10.dpi", "\"bw\" \"sj-dpi\" set \"n\" 50 set"),
        ("d.x10.nrd", "\"bw\" \"nrd\" set \"n\" 50 set"),
        (
            "d.x10.adjust",
            "\"bw\" 0.7 set \"adjust\" 2 set \"cut\" 1 set \"n\" 50 set",
        ),
        ("d.x10.n1", "\"n\" 1 set"),
    ] {
        against(label, label, &density(&x, opts));
    }
    against(
        "d.one",
        "d.one",
        &density("[ 7 ]", "\"bw\" 0.5 set \"n\" 50 set"),
    );
    assert!(stop("d.one.rule").contains("at least 2 points"));
}

#[test]
fn density_refuses_as_s9_says() {
    for (src, want) in [
        (
            "list dict stat.density",
            "STAT.DENSITY: E101 x is outside stat.density's domain: an empty series",
        ),
        (
            "[ 7 ] dict stat.density",
            "STAT.DENSITY: E101 x is outside stat.density's domain: a series of 1 value, where the rule \"nrd0\" needs 2",
        ),
        (
            "[ 1 2 ] dict \"n\" 0 set stat.density",
            "STAT.DENSITY: E101 n is outside stat.density's domain: 0",
        ),
        (
            "[ 1 2 ] dict \"n\" 1048577 set stat.density",
            "STAT.DENSITY: E111 an n of 1048577 output points exceeds its cap of 1048576",
        ),
        (
            "[ 1 2 ] dict \"n\" 5.0 set stat.density",
            "STAT.DENSITY: E101 n is outside stat.density's domain: 5.0",
        ),
        (
            "[ 1 2 ] dict \"from\" 3 set \"to\" 3 set stat.density",
            "STAT.DENSITY: E101 to is outside stat.density's domain: from 3 and to 3, where from < to",
        ),
        (
            "[ 1 2 ] dict \"from\" 9 set stat.density",
            "STAT.DENSITY: E101 from is outside stat.density's domain: from 9 and to 2.877047209290871, where from < to",
        ),
        (
            "[ 1 2 ] dict \"bw\" 0 set stat.density",
            "STAT.DENSITY: E101 bw is outside stat.density's domain: 0",
        ),
        (
            "[ 1 2 ] dict \"bw\" \"ucv\" set stat.density",
            "STAT.DENSITY: E101 bw is outside stat.density's domain: \"ucv\"",
        ),
        (
            "[ 1 2 ] dict \"adjust\" -1 set stat.density",
            "STAT.DENSITY: E101 adjust is outside stat.density's domain: -1",
        ),
        (
            "[ 1 2 ] dict \"cut\" -1 set stat.density",
            "STAT.DENSITY: E101 cut is outside stat.density's domain: -1",
        ),
        (
            "[ 1 2 ] dict \"kernel\" \"box\" set stat.density",
            "STAT.DENSITY: E101 kernel is outside stat.density's domain: \"box\"",
        ),
        (
            "[ 1 2 ] dict \"weights\" [ 1 1 ] set stat.density",
            "STAT.DENSITY: E101 opts is outside stat.density's domain: an unknown key \"weights\"",
        ),
        (
            "[ 1.0e300 1.0e300 ] dict \"bw\" 1.0e-300 set stat.density",
            "STAT.DENSITY: E101 bw is outside stat.density's domain: 1e-300, too narrow for a grid of 512 points at x's magnitude",
        ),
    ] {
        assert_eq!(err(src), want, "{src}");
    }
}

#[test]
fn the_grid_is_rs_power_of_two() {
    // n = 513 and n = 1023 both lay R's 1024-point grid, so their y agree
    // at the ends they share; n = 512 lays 512 points, and does not.
    let x = sample("x10");
    let y = |n: usize| get(&density(&x, &format!("\"n\" {n} set")), "y");
    let (a, b, c) = (y(513), y(1023), y(512));
    assert_eq!(a[0], b[0]);
    assert_eq!(a[512], b[1022]);
    assert_ne!(a[512], c[511]);
}

#[test]
fn the_fft_inverts_and_the_helpers_hold_their_definitions() {
    let mut re: Vec<f64> = (0..16).map(|i| f64::from(i) * 0.5 - 2.0).collect();
    let orig = re.clone();
    let mut im = vec![0.0; 16];
    fft(&mut re, &mut im, false);
    // The DC term is the sum.
    assert!((re[0] - orig.iter().sum::<f64>()).abs() < 1e-12);
    fft(&mut re, &mut im, true);
    for (g, w) in re.iter().zip(&orig) {
        assert!((g / 16.0 - w).abs() < 1e-14);
    }
    // dnorm below and above dnorm4's split at 5, against the plain formula.
    for z in [0.0f64, 1.0, 4.9, 5.1, 8.0, 30.0] {
        let plain = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt() / 2.0;
        let got = dnorm(2.0 * z, 2.0);
        assert!((got - plain).abs() <= 1e-13 * plain, "{z}: {got} {plain}");
    }
    assert_eq!(dnorm(80.0, 2.0), 0.0);
    let (xs, ys) = ([0.0, 1.0, 2.0], [0.0, 10.0, 30.0]);
    assert_eq!(approx1(0.5, &xs, &ys), 5.0);
    assert_eq!(approx1(1.0, &xs, &ys), 10.0);
    assert_eq!(approx1(1.5, &xs, &ys), 20.0);
    assert_eq!(approx1(2.0, &xs, &ys), 30.0);
}

#[test]
fn every_form_answers_on_its_side() {
    let x = "[ 1.2 2.5 2.5 3.1 4.8 ]";
    let plain = top(&format!("{x} \"nrd0\" stat.bw"));
    let mut w = run(&format!("{x} \"nrd0\" . stat.bw.")).unwrap();
    assert_eq!(w.pull_workbench().map(|v| floats(&v)), Some(vec![plain]));
    let d = run(&format!("{x} dict stat.density"))
        .unwrap()
        .pull()
        .unwrap();
    let mut w = run(&format!("{x} dict . stat.density.")).unwrap();
    assert_eq!(w.pull_workbench().unwrap().display(), d.display());
}

#[test]
fn no_operand_aborts() {
    let xs = [
        "list",
        "[ 1 ]",
        "[ 5 5 5 ]",
        "[ 0 0 ]",
        "[ -0.0 0.0 1 ]",
        "[ 1.0e308 -1.7e308 ]",
        "[ 1.0e300 1.0e300 1.0000000000000002e300 ]",
        "[ 5.0e-324 0 ]",
        "[ 1 \"a\" ]",
        "[ 1 na ]",
        "1",
        "dict",
    ];
    let rules = [
        "\"nrd0\"",
        "\"nrd\"",
        "\"sj\"",
        "\"sj-dpi\"",
        "\"x\"",
        "1",
        "list",
    ];
    let opts = [
        "dict",
        "dict \"bw\" \"sj\" set",
        "dict \"bw\" \"nrd\" set",
        "dict \"bw\" 1.0e-300 set",
        "dict \"bw\" 1.0e300 set",
        "dict \"adjust\" 1.0e308 set",
        "dict \"cut\" 1.0e308 set",
        "dict \"n\" 1 set \"kernel\" \"optcosine\" set",
        "dict \"from\" -1.0e308 set \"to\" 1.0e308 set",
        "list",
        "1",
    ];
    let ok = |src: &str| {
        if let Err(e) = run(src) {
            assert!(
                e.contains(": E1") || e.contains("too shallow") || e.contains("returned"),
                "{src}: {e}"
            );
        }
    };
    for x in xs {
        for r in rules {
            ok(&format!("{x} {r} stat.bw"));
        }
        for o in opts {
            ok(&format!("{x} {o} stat.density"));
        }
        // The `.` forms read the same operands from the workbench's side.
        ok(&format!("{x} \"sj\" . stat.bw."));
        ok(&format!("{x} dict . stat.density."));
        ok(&format!("{x} . stat.bw."));
        ok(&format!("{x} . stat.density."));
    }
    ok("stat.bw");
    ok("stat.density");
}
