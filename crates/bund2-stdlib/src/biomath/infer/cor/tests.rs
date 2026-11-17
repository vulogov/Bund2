//! RFC-0016 criteria 1, 2 and 7, for §S3's correlation tests. The fixture
//! is `infer16.py`'s, at 50 digits; no R output is committed for these, so
//! it alone is the check (A7).

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{close, err, fixture, floats, run};

const FIXTURE: &str = include_str!("../../fixtures/infer16.txt");

/// R's `sleep` data, `extra` by group.
const A: &str = "[ 0.7 -1.6 -0.2 -1.2 -0.1 3.4 3.7 0.8 0.0 2.0 ]";
const B: &str = "[ 1.9 0.8 1.1 0.1 -0.1 4.4 5.5 1.6 4.6 3.4 ]";

fn map(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap();
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn one(m: &BTreeMap<String, BundValue>, k: &str) -> f64 {
    floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")))[0]
}

fn want(key: &str) -> f64 {
    fixture(FIXTURE, key)[0]
}

fn text(m: &BTreeMap<String, BundValue>, k: &str) -> String {
    m.get(k)
        .and_then(BundValue::as_str)
        .unwrap_or_else(|| panic!("no {k} in {m:?}"))
}

/// `a b` under `word`, the plain word for two-sided, `.opts` otherwise.
fn test(a: &str, b: &str, word: &str, alt: &str) -> BTreeMap<String, BundValue> {
    if alt == "two.sided" {
        map(&format!("{a} {b} stat.cor.test.{word}"))
    } else {
        map(&format!(
            "{a} {b} dict \"alternative\" \"{alt}\" set stat.cor.test.{word}.opts"
        ))
    }
}

const ALTS: [&str; 3] = ["two.sided", "greater", "less"];

#[test]
fn every_word_is_registered_with_its_effect() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        let want = match (w.ends_with(".opts"), w.ends_with('.')) {
            (true, _) => (3, 1),
            (false, true) if w.ends_with(".opts.") => (2, 0),
            (false, true) => (1, 0),
            (false, false) => (2, 1),
        };
        assert_eq!(
            (e.consumes, e.produces, e.opaque),
            (want.0, want.1, false),
            "{w}"
        );
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("stat.cor.test."))
        .count();
    assert_eq!(ours, WORDS.len());
}

#[test]
fn pearson_against_the_fixture() {
    let n = "cor.pearson";
    for alt in ALTS {
        let m = test(A, B, "pearson", alt);
        let k = format!("{n}.{alt}");
        close(
            alt,
            &[one(&m, "estimate")],
            &[want(&format!("{n} estimate"))],
            1e-12,
        );
        close(
            alt,
            &[one(&m, "statistic")],
            &[want(&format!("{k} statistic"))],
            1e-12,
        );
        close(alt, &[one(&m, "p")], &[want(&format!("{k} p"))], 1e-8);
        assert_eq!(one(&m, "df"), 8.0);
        assert_eq!(one(&m, "conf"), 0.95);
        assert_eq!(text(&m, "method"), "Pearson's product-moment correlation");
        assert_eq!(text(&m, "alternative"), alt);
        match alt {
            "two.sided" => {
                close(
                    alt,
                    &[one(&m, "lower")],
                    &[want(&format!("{k} lower"))],
                    1e-9,
                );
                close(
                    alt,
                    &[one(&m, "upper")],
                    &[want(&format!("{k} upper"))],
                    1e-9,
                );
            }
            "greater" => {
                close(
                    alt,
                    &[one(&m, "lower")],
                    &[want(&format!("{k} lower"))],
                    1e-9,
                );
                assert_eq!(one(&m, "upper"), 1.0);
            }
            _ => {
                assert_eq!(one(&m, "lower"), -1.0);
                close(
                    alt,
                    &[one(&m, "upper")],
                    &[want(&format!("{k} upper"))],
                    1e-9,
                );
            }
        }
        // The estimate is `stat.cor.pearson`'s, bit for bit.
        let r = floats(
            &run(&format!("{A} {B} stat.cor.pearson"))
                .unwrap()
                .pull()
                .unwrap(),
        )[0];
        assert_eq!(one(&m, "estimate").to_bits(), r.to_bits());
    }
    // n = 3: no interval and no conf.
    let (a3, b3) = ("[ 1.0 2.0 4.0 ]", "[ 1.5 2.0 5.0 ]");
    for alt in ALTS {
        let m = test(a3, b3, "pearson", alt);
        let k = format!("cor.pearson3.{alt}");
        close(
            alt,
            &[one(&m, "statistic")],
            &[want(&format!("{k} statistic"))],
            1e-12,
        );
        close(alt, &[one(&m, "p")], &[want(&format!("{k} p"))], 1e-8);
        for key in ["lower", "upper", "conf"] {
            assert!(!m.contains_key(key), "{alt} {key}");
        }
    }
    // A wider level through the options.
    let m = map(&format!(
        "{A} {B} dict \"conf\" 0.5 set stat.cor.test.pearson.opts"
    ));
    assert!(one(&m, "lower") > want("cor.pearson.two.sided lower"));
    assert_eq!(one(&m, "conf"), 0.5);
}

/// r is exactly 1.0 in f64: no statistic, p 0, the interval [1, 1].
#[test]
fn pearson_at_r_one() {
    let (a, b) = ("[ 1.0 2.0 3.0 4.0 ]", "[ 2.0 4.0 6.0 8.0 ]");
    let m = test(a, b, "pearson", "two.sided");
    assert_eq!(one(&m, "estimate"), 1.0);
    assert!(!m.contains_key("statistic"));
    assert_eq!(one(&m, "p"), 0.0);
    assert_eq!((one(&m, "lower"), one(&m, "upper")), (1.0, 1.0));
    assert_eq!(one(&test(a, b, "pearson", "greater"), "p"), 0.0);
    assert_eq!(one(&test(a, b, "pearson", "less"), "p"), 1.0);
    // Reversed, r = −1.
    let m = test(a, "[ 8.0 6.0 4.0 2.0 ]", "pearson", "less");
    assert_eq!((one(&m, "estimate"), one(&m, "p")), (-1.0, 0.0));
    // A hair inside 1 takes the ordinary path.
    let m = test(
        "[ 0.1 0.2 0.3 0.7 ]",
        "[ 1.0 2.0 3.0 7.0 ]",
        "pearson",
        "two.sided",
    );
    if one(&m, "estimate") < 1.0 {
        assert!(one(&m, "statistic").is_finite());
    }
    // Spearman keeps S at |ρ| = 1, and its p follows Pearson's rule.
    let m = test(a, b, "spearman", "two.sided");
    assert_eq!((one(&m, "statistic"), one(&m, "p")), (0.0, 0.0));
    let m = test(a, "[ 8.0 6.0 4.0 2.0 ]", "spearman", "two.sided");
    assert_eq!(one(&m, "statistic"), (64.0 - 4.0) / 3.0);
}

#[test]
fn spearman_against_the_fixture() {
    for alt in ALTS {
        let m = test(A, B, "spearman", alt);
        close(
            alt,
            &[one(&m, "estimate")],
            &[want("cor.spearman estimate")],
            1e-12,
        );
        close(
            alt,
            &[one(&m, "statistic")],
            &[want("cor.spearman statistic")],
            1e-12,
        );
        close(
            alt,
            &[one(&m, "p")],
            &[want(&format!("cor.spearman.{alt} p"))],
            1e-8,
        );
        assert_eq!(text(&m, "method"), "Spearman's rank correlation rho");
        assert!(!m.contains_key("conf") && !m.contains_key("df"));
    }
}

/// Kendall's exact T and p at n = 10 and in a far tail at n = 45, within
/// 1e-10; with ties, the normal approximation's z and p.
#[test]
fn kendall_against_the_fixture() {
    let a45: Vec<String> = (1..=45).map(|i| i.to_string()).collect();
    let mut b45: Vec<u32> = (1..=45).collect();
    for i in [0, 9, 19, 29, 39] {
        b45.swap(i, i + 1);
    }
    let b45: Vec<String> = b45.iter().map(u32::to_string).collect();
    let (a45, b45) = (
        format!("[ {} ]", a45.join(" ")),
        format!("[ {} ]", b45.join(" ")),
    );
    for (label, a, b) in [
        ("cor.kendall10", A.to_string(), B.to_string()),
        ("cor.kendall45", a45, b45),
    ] {
        for alt in ALTS {
            let m = test(&a, &b, "kendall", alt);
            assert_eq!(
                one(&m, "statistic"),
                want(&format!("{label} statistic")),
                "{label}"
            );
            close(
                &format!("{label} {alt}"),
                &[one(&m, "p")],
                &[want(&format!("{label}.{alt} p"))],
                1e-10,
            );
            assert_eq!(text(&m, "method"), "Kendall's rank correlation tau");
        }
    }
    let (a, b) = (
        "[ 1 2 2 3 4 4 5 6 7 8 ]",
        "[ 1.9 0.8 1.1 0.1 0.1 4.4 5.5 1.6 4.6 4.4 ]",
    );
    for alt in ALTS {
        let m = test(a, b, "kendall", alt);
        close(
            alt,
            &[one(&m, "statistic")],
            &[want("cor.kendall.ties statistic")],
            1e-12,
        );
        close(
            alt,
            &[one(&m, "p")],
            &[want(&format!("cor.kendall.ties.{alt} p"))],
            1e-10,
        );
        let tau = floats(
            &run(&format!("{a} {b} stat.cor.kendall"))
                .unwrap()
                .pull()
                .unwrap(),
        )[0];
        assert_eq!(one(&m, "estimate").to_bits(), tau.to_bits());
    }
}

#[test]
fn the_forms_answer_the_plain_words() {
    for w in ["pearson", "spearman", "kendall"] {
        let plain = map(&format!("{A} {B} stat.cor.test.{w}"));
        let mut i = run(&format!("{A} {B} . stat.cor.test.{w}.")).unwrap();
        assert_eq!(i.depth(), 0, "{w}");
        assert_eq!(i.pull_workbench().unwrap().as_map().unwrap(), &plain, "{w}");
        // The `.opts` word with no options is the plain word.
        assert_eq!(
            map(&format!("{A} {B} dict stat.cor.test.{w}.opts")),
            plain,
            "{w}"
        );
        let mut i = run(&format!("{A} {B} dict . stat.cor.test.{w}.opts.")).unwrap();
        assert_eq!(i.depth(), 0, "{w}");
        assert_eq!(i.pull_workbench().unwrap().as_map().unwrap(), &plain, "{w}");
    }
}

#[test]
fn each_refusal_is_its_template() {
    for w in ["pearson", "spearman", "kendall"] {
        let up = w.to_uppercase();
        assert_eq!(
            err(&format!("[ 1.0 2.0 ] [ 3.0 4.0 ] stat.cor.test.{w}")),
            format!(
                "STAT.COR.TEST.{up}: E101 series is outside stat.cor.test.{w}'s domain: \
                 a series of 2 values"
            )
        );
        assert!(
            err(&format!(
                "[ 1.0 1.0 1.0 ] [ 3.0 4.0 5.0 ] stat.cor.test.{w}"
            ))
            .starts_with(&format!("STAT.COR.TEST.{up}: E101 a "))
        );
        assert!(
            err(&format!("[ 1.0 2.0 3.0 ] [ 3.0 4.0 ] stat.cor.test.{w}"))
                .starts_with(&format!("STAT.COR.TEST.{up}: E102 "))
        );
    }
    assert_eq!(
        err(&format!(
            "{A} {B} dict \"conf\" 0.9 set stat.cor.test.kendall.opts"
        )),
        "STAT.COR.TEST.KENDALL.OPTS: E101 opts is outside stat.cor.test.kendall.opts's \
         domain: the key \"conf\", which this word does not take"
    );
    assert!(
        err(&format!(
            "{A} {B} dict \"mu\" 1.0 set stat.cor.test.pearson.opts"
        ))
        .starts_with("STAT.COR.TEST.PEARSON.OPTS: E101 opts ")
    );
    assert!(
        err(&format!(
            "{A} {B} dict \"conf\" 1 set stat.cor.test.pearson.opts"
        ))
        .starts_with("STAT.COR.TEST.PEARSON.OPTS: E101 conf ")
    );
}

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "0.5",
        "\"x\"",
        "list",
        "[ 1 ]",
        "[ 1 1 1 ]",
        "[ 1 2 4 ]",
        "[ 3 2 1 ]",
        "[ 1.0e308 -1.0e308 0.0 ]",
        "[ 1 float.NaN 2 ]",
        "dict",
        "dict \"conf\" 0.999999999999 set",
        "dict \"alternative\" \"less\" set",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
    ];
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        for o in operands {
            for p in operands {
                for src in [
                    format!("{o} {w}"),
                    format!("{o} {p} {w}"),
                    format!("{o} {p} {o} {w}"),
                    format!("{o} {p} dict {w}"),
                ] {
                    if let Err(e) = run(&src) {
                        assert!(
                            e.contains(": E1")
                                || e.contains("too shallow")
                                || e.contains("returned"),
                            "{src}: {e}"
                        );
                    }
                }
            }
        }
    }
}
