//! RFC-0019 criterion 11, and 1 and 14, for §S9.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::describe::mean;
use crate::biomath::testkit::{err, fixture, floats, run};

const PERMUTE: &str = include_str!("../fixtures/permute.txt");

const STATISTICS: [&str; 3] = ["mean.diff", "median.diff", "t"];
const ALTERNATIVES: [&str; 3] = ["two.sided", "greater", "less"];

const A: &str = "[ 12 15 9 20 17 ]";
const B: &str = "[ 8 11 6 14 10 7 ]";

fn answer(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    assert_eq!(i.depth(), 0, "{src}");
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn num(m: &BTreeMap<String, BundValue>, k: &str) -> f64 {
    let v = floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")));
    assert_eq!(v.len(), 1, "{k}");
    v[0]
}

fn text(m: &BTreeMap<String, BundValue>, k: &str) -> String {
    m.get(k)
        .and_then(BundValue::as_str)
        .unwrap_or_else(|| panic!("no STRING {k} in {m:?}"))
        .to_string()
}

fn test(a: &str, b: &str, statistic: &str, opts: &str) -> String {
    format!("{a} {b} {statistic} dict {opts} stat.test.permutation")
}

/// Two answers alike: the same keys, the same numbers to the bit and the
/// same STRINGs.
fn same(a: &BTreeMap<String, BundValue>, b: &BTreeMap<String, BundValue>, what: &str) {
    assert_eq!(
        a.keys().collect::<Vec<_>>(),
        b.keys().collect::<Vec<_>>(),
        "{what}"
    );
    for k in ["statistic", "p", "permutations"] {
        assert_eq!(num(a, k).to_bits(), num(b, k).to_bits(), "{what} {k}");
    }
    for k in ["method", "alternative"] {
        assert_eq!(text(a, k), text(b, k), "{what} {k}");
    }
}

// Criterion 1.

#[test]
fn the_word_is_registered_without_a_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        assert!(e.opaque, "{w}");
        assert_eq!(e.consumes, 4, "{w}");
        assert!(i.registry.effect_of(&format!("{w}.")).is_none(), "{w}.");
    }
    let ours: Vec<String> = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("stat.test.permutation"))
        .collect();
    assert_eq!(ours.len(), WORDS.len(), "{ours:?}");
}

// Criterion 11.

/// Each statistic and alternative on the fixture's two inputs: the
/// statistic within criterion 15's tolerance, and p the fixture's count
/// over its splits, to the bit.
#[test]
fn the_exact_test_matches_the_enumeration() {
    let mut cases = 0;
    for case in ["x", "y"] {
        let line = |k: &str| fixture(PERMUTE, &format!("{case} {k}"));
        let list = |k: &str| {
            let xs: Vec<String> = line(k).iter().map(|x| format!("{x:?}")).collect();
            format!("[ {} ]", xs.join(" "))
        };
        let (a, b) = (list("a"), list("b"));
        for s in STATISTICS {
            for alt in ALTERNATIVES {
                let k = |key: &str| line(&format!("{s} {alt} {key}"))[0];
                let m = answer(&test(
                    &a,
                    &b,
                    &format!("{s:?}"),
                    &format!("\"alternative\" {alt:?} set"),
                ));
                let (t0, hits, splits) = (k("t0"), k("hits"), k("splits"));
                let got = num(&m, "statistic");
                assert!(
                    (got - t0).abs() <= 1e-9 * t0.abs() + 1e-12,
                    "{case} {s} {alt}: {got} against {t0}"
                );
                assert_eq!(
                    num(&m, "p").to_bits(),
                    (hits / splits).to_bits(),
                    "{case} {s} {alt}"
                );
                assert_eq!(num(&m, "permutations"), splits, "{case} {s} {alt}");
                assert_eq!(text(&m, "method"), "exact permutation");
                assert_eq!(text(&m, "alternative"), alt);
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 18);
}

/// A name's answer is a lambda's calling the word behind it, to the bit;
/// the lambda takes a beneath b.
#[test]
fn a_named_statistic_is_its_lambda() {
    for (name, lambda) in [
        ("mean.diff", "{ stat.mean swap stat.mean - }"),
        ("median.diff", "{ stat.median swap stat.median - }"),
    ] {
        for alt in ALTERNATIVES {
            let opts = format!("\"alternative\" {alt:?} set");
            let named = answer(&test(A, B, &format!("{name:?}"), &opts));
            let by_lambda = answer(&test(A, B, lambda, &opts));
            same(&named, &by_lambda, name);
        }
    }
    // Welch's t is `stat.test.welch`'s statistic.
    let welch = answer(&format!("{A} {B} stat.test.welch"));
    let t = answer(&test(A, B, "\"t\"", ""));
    assert_eq!(
        num(&t, "statistic").to_bits(),
        num(&welch, "statistic").to_bits()
    );
}

/// After a seed the Monte Carlo test repeats, and it is a Bund program's
/// own loop of `rng.shuffle` on the pooled sample, to the bit.
#[test]
fn the_monte_carlo_test_is_a_loop_of_shuffles() {
    const N: usize = 5;
    const DRAWS: usize = 300;
    let opts = format!("\"exact\" false set \"B\" {DRAWS} set");
    let src = format!("17 rng.seed {}", test(A, B, "\"mean.diff\"", &opts));
    let m = answer(&src);
    same(&m, &answer(&src), "repeat");
    assert_eq!(text(&m, "method"), "Monte Carlo permutation");
    assert_eq!(num(&m, "permutations"), DRAWS as f64);
    let t0 = num(&m, "statistic");

    let mut i = run("17 rng.seed").unwrap_or_else(|e| panic!("{e}"));
    let step = bund2_syntax::compile("[ 12 15 9 20 17 8 11 6 14 10 7 ] rng.shuffle")
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut hits = 0u32;
    for _ in 0..DRAWS {
        i.eval(&step).unwrap_or_else(|e| panic!("{e:?}"));
        let xs = floats(&i.pull().unwrap_or_else(|| panic!("shuffle")));
        let t = mean(&xs[..N]) - mean(&xs[N..]);
        if t.abs() >= t0.abs() - 1e-12 * t0.abs() {
            hits += 1;
        }
    }
    let want = (1.0 + f64::from(hits)) / (DRAWS as f64 + 1.0);
    assert_eq!(num(&m, "p").to_bits(), want.to_bits());
}

/// On an input where both apply, the Monte Carlo p is within three of its
/// standard errors of the exact p.
#[test]
fn the_monte_carlo_p_is_near_the_exact_p() {
    for s in STATISTICS {
        for alt in ALTERNATIVES {
            let opts = format!("\"alternative\" {alt:?} set");
            let exact = num(&answer(&test(A, B, &format!("{s:?}"), &opts)), "p");
            let mc = num(
                &answer(&format!(
                    "23 rng.seed {}",
                    test(
                        A,
                        B,
                        &format!("{s:?}"),
                        &format!("{opts} \"exact\" false set")
                    )
                )),
                "p",
            );
            let se = (exact * (1.0 - exact) / 9999.0).sqrt();
            assert!(
                (mc - exact).abs() <= 3.0 * se,
                "{s} {alt}: {mc} against {exact}, se {se}"
            );
        }
    }
}

/// Above C(n + m, n) = 10⁶ the test is Monte Carlo when `exact` is not
/// given, and `exact` true there is E111.
#[test]
fn above_the_cap_the_test_is_monte_carlo() {
    let a = format!(
        "[ {} ]",
        (1..=15)
            .map(|x| format!("{x}.5"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let b = format!(
        "[ {} ]",
        (3..=17)
            .map(|x| format!("{x}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let m = answer(&format!(
        "1 rng.seed {}",
        test(&a, &b, "\"mean.diff\"", "\"B\" 99 set")
    ));
    assert_eq!(text(&m, "method"), "Monte Carlo permutation");
    assert_eq!(num(&m, "permutations"), 99.0);
    assert_eq!(
        err(&test(&a, &b, "\"mean.diff\"", "\"exact\" true set")),
        "STAT.TEST.PERMUTATION: E111 an exact test over C(30, 15) splits exceeds its cap of 1000000"
    );
    // At the cap's edge: C(10, 5) = 252 splits, exact true or left out.
    same(
        &answer(&test(A, B, "\"t\"", "")),
        &answer(&test(A, B, "\"t\"", "\"exact\" true set")),
        "exact true",
    );
}

#[test]
fn each_refusal_is_its_template() {
    let e = |src: &str, want: &str| assert_eq!(err(src), want, "{src}");
    let w = "STAT.TEST.PERMUTATION";
    e(
        &test("list", B, "\"mean.diff\"", ""),
        &format!("{w}: E101 a is outside stat.test.permutation's domain: an empty LIST"),
    );
    e(
        &test(A, "1.5", "\"mean.diff\"", ""),
        &format!("{w}: E101 b is outside stat.test.permutation's domain: 1.5"),
    );
    e(
        &test(A, B, "\"mean\"", ""),
        &format!("{w}: E101 statistic is outside stat.test.permutation's domain: \"mean\""),
    );
    e(
        &test(A, B, "\"t\"", "\"alternative\" \"both\" set"),
        &format!("{w}: E101 alternative is outside stat.test.permutation's domain: \"both\""),
    );
    e(
        &test(A, B, "\"t\"", "\"exact\" 1 set"),
        &format!("{w}: E101 exact is outside stat.test.permutation's domain: 1"),
    );
    e(
        &test(A, B, "\"t\"", "\"B\" 0 set"),
        &format!("{w}: E101 B is outside stat.test.permutation's domain: 0"),
    );
    e(
        &test(A, B, "\"t\"", "\"conf\" 0.9 set"),
        &format!(
            "{w}: E101 opts is outside stat.test.permutation's domain: an unknown key \"conf\""
        ),
    );
    e(
        &test("[ 1.0 ]", B, "\"t\"", ""),
        &format!(
            "{w}: E101 a is outside stat.test.permutation's domain: 1 value, where \"t\" takes at least 2"
        ),
    );
    e(
        &test("[ 1.0 1.0 ]", "[ 2.0 2.0 2.0 ]", "\"t\"", ""),
        &format!("{w}: E101 a and b is outside stat.test.permutation's domain: a constant series"),
    );
    e(
        &test(A, B, "{ drop drop \"s\" }", ""),
        &format!("{w}: E112 the model lambda left \"s\", where one finite number was required"),
    );
    e(
        &test(A, B, "\"t\"", "\"exact\" false set \"B\" 10000000 set"),
        &format!("{w}: E111 10000000 shuffles of 11 values exceeds its cap of 100000000"),
    );
    // C(10001, 1) splits of 10001 values: within the split cap, above §S1's
    // work cap.
    let many = format!("[ {} ]", vec!["1.5"; 10_000].join(" "));
    e(
        &test("[ 2.0 ]", &many, "\"mean.diff\"", ""),
        &format!("{w}: E111 10001 splits of 10001 values exceeds its cap of 100000000"),
    );
}

#[test]
fn a_statistic_of_nodata_is_refused() {
    let got = err(&test(A, B, "nodata", ""));
    assert!(
        got.starts_with(
            "STAT.TEST.PERMUTATION: E101 statistic is outside stat.test.permutation's domain: "
        ),
        "{got}"
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
        "2",
        "1.0",
        "\"t\"",
        "\"mean.diff\"",
        "list",
        "dict",
        "nodata",
        "[ 1.0 ]",
        "[ 1.0 float.NaN push ]",
        "[ 1 \"a\" ]",
        "[ \"x\" ] [ [ 1.0 2.0 ] ] frame.new",
        "{ 1 }",
        "{ drop }",
        "{ drop drop float.NaN }",
        "{ drop drop \"s\" }",
        "{ no.such.word }",
        "{ stat.mean }",
        "dict \"exact\" false set",
        "dict \"B\" -3 set",
    ];
    for w in WORDS {
        for o in operands {
            for k in 1..=4 {
                let src = format!("{} {w}", vec![o; k].join(" "));
                let _ = run(&src);
            }
            let _ = run(&format!("{o} {B} \"t\" dict {w}"));
            let _ = run(&format!("{A} {o} \"t\" dict {w}"));
            let _ = run(&format!("{A} {B} {o} dict {w}"));
            let _ = run(&format!("{A} {B} \"t\" {o} {w}"));
            let _ = run(&format!("{o} {o} \"median.diff\" dict {w}"));
            let _ = run(&format!(
                "1 rng.seed {o} {o} \"t\" dict \"exact\" false set \"B\" 5 set {w}"
            ));
        }
    }
}

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":stat.test.permutation { 42 } register 1 stat.test.permutation")
        .unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
