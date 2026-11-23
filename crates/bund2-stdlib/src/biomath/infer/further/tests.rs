//! RFC-0019 criterion 5, and 1 and 14, for §S5's rank and table tests.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{err, floats, run, run_on};

const FURTHER: &str = include_str!("../../fixtures/further.txt");

fn refusal(src: &str) -> String {
    err(src)
}

fn answer(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    assert_eq!(i.depth(), 0, "{src}");
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn key(m: &BTreeMap<String, BundValue>, k: &str) -> f64 {
    floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")))[0]
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

/// The criteria's tolerance: 10⁻⁹ relative and 10⁻¹² absolute, and a p
/// below 10⁻⁶ absolutely at 10⁻¹².
fn agrees(k: &str, got: f64, want: f64) -> bool {
    if k == "p" && want < 1e-6 {
        return (got - want).abs() <= 1e-12;
    }
    (got - want).abs() <= 1e-9 * want.abs() + 1e-12
}

// Criterion 1.

#[test]
fn every_word_is_registered_with_its_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for (w, n) in [
        ("stat.test.wilcoxon", 2),
        ("stat.test.wilcoxon.paired", 2),
        ("stat.test.wilcoxon.opts", 3),
        ("stat.test.wilcoxon.paired.opts", 3),
        ("stat.test.kruskal", 1),
        ("stat.test.friedman", 1),
        ("stat.test.mcnemar", 1),
        ("stat.test.mcnemar.opts", 2),
        ("stat.test.cmh", 1),
    ] {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        assert!(!e.opaque, "{w}");
        assert_eq!((e.consumes, e.produces), (n, 1), "{w}");
        let d = i
            .registry
            .effect_of(&format!("{w}."))
            .unwrap_or_else(|| panic!("{w}."));
        assert!(!d.opaque, "{w}.");
        assert_eq!((d.consumes, d.produces), (n - 1, 0), "{w}.");
    }
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
    }
}

// Criterion 5.

#[test]
fn each_answer_matches_the_fixture() {
    let all = lines(FURTHER);
    assert_eq!(all.len(), 98);
    for (program, k, want) in all {
        let got = key(&answer(program), k);
        assert!(
            agrees(k, got, want),
            "{program}: {k} {got:e} against {want:e}"
        );
    }
}

#[test]
fn each_method_is_rs() {
    let paired = "[ 1.83 0.50 1.62 2.48 1.68 1.88 1.55 3.06 1.30 ] \
                  [ 0.878 0.647 0.598 2.05 1.06 1.29 1.06 3.14 1.29 ]";
    let tied = "[ 1 2 2 3 4 5 5 5 6 7 8 0.5 -1 -2 3 9 11 ] 3";
    for (src, want) in [
        (
            format!("{paired} stat.test.wilcoxon.paired"),
            "Wilcoxon signed rank exact test",
        ),
        (
            format!("{tied} stat.test.wilcoxon"),
            "Wilcoxon signed rank test with continuity correction",
        ),
        (
            format!("{tied} dict \"correct\" false set stat.test.wilcoxon.opts"),
            "Wilcoxon signed rank test",
        ),
        (
            "[ [ 1 2 ] [ 3 4 5 ] ] stat.test.kruskal".into(),
            "Kruskal-Wallis rank sum test",
        ),
        (
            "[ [ 1 2 ] [ 4 3 ] ] matrix stat.test.friedman".into(),
            "Friedman rank sum test",
        ),
        (
            "[ [ 794 150 ] [ 86 570 ] ] matrix stat.test.mcnemar".into(),
            "McNemar's Chi-squared test with continuity correction",
        ),
        (
            "[ [ 10 5 ] [ 5 20 ] ] matrix stat.test.mcnemar".into(),
            "McNemar's Chi-squared test",
        ),
        (
            "[ [ 794 150 ] [ 86 570 ] ] matrix dict \"correct\" false set stat.test.mcnemar.opts"
                .into(),
            "McNemar's Chi-squared test",
        ),
        (
            "[ [ [ 3 2 ] [ 2 3 ] ] [ [ 4 3 ] [ 3 3 ] ] ] stat.test.cmh".into(),
            "Mantel-Haenszel chi-squared test with continuity correction",
        ),
        (
            "[ [ [ 2 2 ] [ 2 2 ] ] [ [ 1 1 ] [ 1 1 ] ] ] stat.test.cmh".into(),
            "Mantel-Haenszel chi-squared test without continuity correction",
        ),
    ] {
        let m = answer(&src);
        let got = m["method"].as_str().unwrap_or_else(|| panic!("{src}"));
        assert_eq!(got, want, "{src}");
    }
}

/// The keys: the test MAP for the plain words, RFC-0016's beside it for
/// the signed-rank `.opts` words, the infinite end left out one-sided, and
/// CMH's estimate and interval.
#[test]
fn each_answer_has_its_keys() {
    let x = "[ 1.83 0.50 1.62 2.48 1.68 1.88 1.55 3.06 1.30 ] 1.0";
    let names = |src: &str| -> Vec<String> { answer(src).keys().cloned().collect() };
    assert_eq!(
        names(&format!("{x} stat.test.wilcoxon")),
        ["method", "p", "statistic"]
    );
    assert_eq!(
        names(&format!("{x} dict stat.test.wilcoxon.opts")),
        [
            "alternative",
            "conf",
            "estimate",
            "lower",
            "method",
            "p",
            "statistic",
            "upper"
        ]
    );
    assert_eq!(
        names(&format!(
            "{x} dict \"alternative\" \"greater\" set stat.test.wilcoxon.opts"
        )),
        [
            "alternative",
            "conf",
            "estimate",
            "lower",
            "method",
            "p",
            "statistic"
        ]
    );
    assert_eq!(
        names("[ [ 1 2 ] [ 3 4 5 ] ] stat.test.kruskal"),
        ["df", "method", "p", "statistic"]
    );
    assert_eq!(
        names("[ [ [ 3 2 ] [ 2 3 ] ] [ [ 4 3 ] [ 3 3 ] ] ] stat.test.cmh"),
        [
            "conf",
            "df",
            "estimate",
            "lower",
            "method",
            "p",
            "statistic",
            "upper"
        ]
    );
}

/// With the defaults, an `.opts` word's test keys are its plain word's, bit
/// for bit, on either path.
#[test]
fn the_opts_words_agree_with_the_plain_words() {
    for (plain, opts) in [
        (
            "[ 1.83 0.50 1.62 2.48 1.68 1.88 1.55 3.06 1.30 ] 1.0 stat.test.wilcoxon",
            "[ 1.83 0.50 1.62 2.48 1.68 1.88 1.55 3.06 1.30 ] 1.0 dict stat.test.wilcoxon.opts",
        ),
        (
            "[ 1 2 2 3 4 5 5 5 6 7 8 0.5 -1 -2 3 9 11 ] 3 stat.test.wilcoxon",
            "[ 1 2 2 3 4 5 5 5 6 7 8 0.5 -1 -2 3 9 11 ] 3 dict stat.test.wilcoxon.opts",
        ),
        (
            "[ 3 1 4 1 5 9 2 6 ] [ 2 7 1 8 2 8 1 8 ] stat.test.wilcoxon.paired",
            "[ 3 1 4 1 5 9 2 6 ] [ 2 7 1 8 2 8 1 8 ] dict stat.test.wilcoxon.paired.opts",
        ),
        (
            "[ [ 20 5 ] [ 9 30 ] ] matrix stat.test.mcnemar",
            "[ [ 20 5 ] [ 9 30 ] ] matrix dict stat.test.mcnemar.opts",
        ),
    ] {
        let (a, b) = (answer(plain), answer(opts));
        for k in ["statistic", "p", "method"] {
            assert_eq!(a.get(k), b.get(k), "{plain}: {k}");
        }
    }
}

/// The paired test is the one-sample test of a − b against 0.
#[test]
fn paired_is_one_sample_on_the_differences() {
    let a = answer("[ 3 1 4 1 5 9 2 6 ] [ 2 7 1 8 2 8 1 8 ] stat.test.wilcoxon.paired");
    let b = answer("[ 1 -6 3 -7 3 1 1 -2 ] 0 stat.test.wilcoxon");
    for k in ["statistic", "p", "method"] {
        assert_eq!(a.get(k), b.get(k), "{k}");
    }
}

/// The exact distribution against an enumeration here: P(V ≤ v) for every v
/// of n = 10, counted over the 2¹⁰ sign vectors, through the one-sided p.
#[test]
fn the_exact_distribution_is_the_enumeration() {
    let n = 10usize;
    let u = n * (n + 1) / 2;
    let mut counts = vec![0u32; u + 1];
    for mask in 0u32..(1 << n) {
        let v: usize = (0..n).filter(|j| mask >> j & 1 == 1).map(|j| j + 1).sum();
        counts[v] += 1;
    }
    let dist = super::SignRankDist::new(n);
    let mut below = 0u32;
    for (v, c) in counts.iter().enumerate() {
        below += c;
        let want = f64::from(below) / 1024.0;
        let got = dist.lower(v as i64);
        assert!(
            (got - want).abs() <= 1e-15,
            "P(V <= {v}): {got} against {want}"
        );
        assert_eq!(dist.mass(v), f64::from(*c) / 1024.0, "P(V = {v})");
    }
}

/// Each `.` form answers its plain word's MAP on the workbench, its top
/// operand taken from there.
#[test]
fn the_dot_forms_take_the_top_from_the_workbench() {
    let mut seen = std::collections::BTreeSet::new();
    for (program, _, _) in lines(FURTHER) {
        if !seen.insert(program) {
            continue;
        }
        let (operands, word) = program.rsplit_once(' ').unwrap_or_default();
        let mut i = run(operands).unwrap_or_else(|e| panic!("{operands}: {e}"));
        let top = i.pull().unwrap_or_else(|| panic!("{operands}: empty"));
        let mut rest = Vec::new();
        while let Some(v) = i.pull() {
            rest.push(v);
        }
        rest.reverse();
        let mut d = run_on(&rest, Some(&top), &format!("{word}."))
            .unwrap_or_else(|e| panic!("{word}.: {e}"));
        let wb = d
            .pull_workbench()
            .unwrap_or_else(|| panic!("{word}.: empty workbench"));
        assert_eq!(d.depth(), 0, "{word}.");
        let plain = answer(program);
        assert_eq!(
            wb.as_map().unwrap_or_else(|| panic!("{wb:?}")),
            &plain,
            "{program}"
        );
    }
}

/// D271: a stratum is a 2×2 MATRIX or a LIST of two rows, and the two
/// answer the same, bit for bit.
#[test]
fn a_stratum_is_a_matrix_or_its_rows() {
    let rows =
        "[ [ [ 0 6 ] [ 0 5 ] ] [ [ 3 3 ] [ 0 6 ] ] [ [ 6 0 ] [ 2 4 ] ] [ [ 5 1 ] [ 6 0 ] ] ]";
    let mut i = run(rows).unwrap_or_else(|e| panic!("{e}"));
    let v = i.pull().unwrap_or_else(|| panic!("empty"));
    let strata: Vec<BundValue> = v
        .as_list()
        .unwrap_or_else(|| panic!("{v:?}"))
        .iter()
        .map(|s| {
            crate::convert::conv_value(s, bund2_value::MATRIX).unwrap_or_else(|e| panic!("{e:?}"))
        })
        .collect();
    assert!(strata.iter().all(|s| s.dt() == bund2_value::MATRIX));
    let mut m =
        run_on(&[BundValue::list(strata)], None, "stat.test.cmh").unwrap_or_else(|e| panic!("{e}"));
    let got = m.pull().unwrap_or_else(|| panic!("empty"));
    let want = answer(&format!("{rows} stat.test.cmh"));
    assert_eq!(got.as_map(), Some(&want));
}

/// A program that defines one of the names calls its own word.
#[test]
fn a_program_word_comes_first() {
    let mut i = run(":stat.test.kruskal { 42 } register [ [ 1 ] [ 2 ] ] stat.test.kruskal")
        .unwrap_or_else(|e| panic!("{e}"));
    let v = i.pull().unwrap_or_else(|| panic!("empty"));
    assert_eq!(floats(&v), [42.0]);
}

#[test]
fn each_refusal_is_its_template() {
    let x = "[ 1.83 0.50 1.62 2.48 1.68 1.88 1.55 3.06 1.30 ]";
    for (src, want) in [
        (
            "list 0 stat.test.wilcoxon".to_string(),
            "STAT.TEST.WILCOXON: E101 x is outside stat.test.wilcoxon's domain: an empty series",
        ),
        (
            "[ 2 2 2 ] 2 stat.test.wilcoxon".into(),
            "STAT.TEST.WILCOXON: E101 x is outside stat.test.wilcoxon's domain: every value equal to mu",
        ),
        (
            "[ 1 2 ] [ 1 2 ] stat.test.wilcoxon.paired".into(),
            "STAT.TEST.WILCOXON.PAIRED: E101 a - b is outside stat.test.wilcoxon.paired's domain: every difference 0",
        ),
        (
            "[ 1 2 ] [ 1 ] stat.test.wilcoxon.paired".into(),
            "STAT.TEST.WILCOXON.PAIRED: E102",
        ),
        (
            "[ 1 2 2 4 ] 0 dict \"exact\" true set stat.test.wilcoxon.opts".into(),
            "STAT.TEST.WILCOXON.OPTS: E101 exact is outside stat.test.wilcoxon.opts's domain: true, where x - mu has ties",
        ),
        (
            "[ 1 2 3 0 ] 0 dict \"exact\" true set stat.test.wilcoxon.opts".into(),
            "STAT.TEST.WILCOXON.OPTS: E101 exact is outside stat.test.wilcoxon.opts's domain: true, where x - mu has zeros",
        ),
        (
            format!("{x} 1.0 dict \"mu\" 1 set stat.test.wilcoxon.opts"),
            "STAT.TEST.WILCOXON.OPTS: E101 opts is outside stat.test.wilcoxon.opts's domain: an unknown key \"mu\"",
        ),
        (
            format!("{x} 1.0 dict \"exact\" 1 set stat.test.wilcoxon.opts"),
            "STAT.TEST.WILCOXON.OPTS: E101 exact is outside stat.test.wilcoxon.opts's domain: 1",
        ),
        (
            format!("{x} 1.0 dict \"conf\" 1 set stat.test.wilcoxon.opts"),
            "STAT.TEST.WILCOXON.OPTS: E101 conf is outside stat.test.wilcoxon.opts's domain: 1",
        ),
        (
            format!("{x} 1.0 dict \"alternative\" \"up\" set stat.test.wilcoxon.opts"),
            "STAT.TEST.WILCOXON.OPTS: E101 alternative is outside stat.test.wilcoxon.opts's domain: \"up\"",
        ),
        (
            "[ 1 2 3 ] 0 dict stat.test.wilcoxon.opts".into(),
            "STAT.TEST.WILCOXON.OPTS: E101 x is outside stat.test.wilcoxon.opts's domain: too few for the level",
        ),
        (
            "[ 1 2 2 3 ] 0 dict stat.test.wilcoxon.opts".into(),
            "STAT.TEST.WILCOXON.OPTS: E101 x is outside stat.test.wilcoxon.opts's domain: too few for the level",
        ),
        (
            "[ 5 ] 0 dict \"exact\" false set stat.test.wilcoxon.opts".into(),
            "STAT.TEST.WILCOXON.OPTS: E101 x is outside stat.test.wilcoxon.opts's domain: too few for an interval",
        ),
        (
            "[ [ 1 2 ] [ 3 4 ] ] matrix dict \"exact\" true set stat.test.mcnemar.opts".into(),
            "STAT.TEST.MCNEMAR.OPTS: E101 opts is outside stat.test.mcnemar.opts's domain: the key \"exact\", which this word does not take",
        ),
        (
            "[ [ 1 2 ] ] stat.test.kruskal".into(),
            "STAT.TEST.KRUSKAL: E101 groups is outside stat.test.kruskal's domain: a list of 1 group",
        ),
        (
            "list [ 1 2 ] list push push stat.test.kruskal".into(),
            "STAT.TEST.KRUSKAL: E101 group 2 is outside stat.test.kruskal's domain: an empty series",
        ),
        (
            "[ [ 1 2 ] 3 ] stat.test.kruskal".into(),
            "STAT.TEST.KRUSKAL: E115",
        ),
        (
            "[ [ 1 1 ] [ 1 ] ] stat.test.kruskal".into(),
            "STAT.TEST.KRUSKAL: E101 groups is outside stat.test.kruskal's domain: a constant series",
        ),
        ("[ [ 1 2 ] ] matrix stat.test.friedman".into(), ""),
        (
            "[ [ 1 ] [ 2 ] ] matrix stat.test.friedman".into(),
            "STAT.TEST.FRIEDMAN: E101 table is outside stat.test.friedman's domain: a table of 2 by 1",
        ),
        (
            "[ [ 1 1 ] [ 2 2 ] ] matrix stat.test.friedman".into(),
            "STAT.TEST.FRIEDMAN: E101 table is outside stat.test.friedman's domain: every row tied throughout",
        ),
        (
            "[ [ 1 2 3 ] [ 4 5 6 ] ] matrix stat.test.mcnemar".into(),
            "STAT.TEST.MCNEMAR: E101 table is outside stat.test.mcnemar's domain: a table of 2 by 3",
        ),
        (
            "[ [ 1 0 ] [ 0 4 ] ] matrix stat.test.mcnemar".into(),
            "STAT.TEST.MCNEMAR: E101 table is outside stat.test.mcnemar's domain: [1,2] and [2,1] both 0",
        ),
        (
            "[ [ 1 -1 ] [ 2 4 ] ] matrix stat.test.mcnemar".into(),
            "STAT.TEST.MCNEMAR: E101 table is outside stat.test.mcnemar's domain: -1 at [1,2]",
        ),
        (
            "[ [ [ 3 2 ] [ 2 3 ] ] ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E101 tables is outside stat.test.cmh's domain: 1 stratum",
        ),
        (
            "[ [ [ 3 2 ] [ 2 3 ] ] [ [ 1 0 ] [ 0 0 ] ] ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E101 stratum 2 is outside stat.test.cmh's domain: a sample of 1",
        ),
        (
            "[ [ [ 3 2 ] [ 2 3 ] ] [ [ 1 2 3 ] [ 1 2 3 ] ] ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E101 stratum 2 is outside stat.test.cmh's domain: a table of 2 by 3",
        ),
        (
            "[ [ [ 3 2 ] [ 2 3 ] ] 5 ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E101 stratum 2 is outside stat.test.cmh's domain: 5",
        ),
        (
            "[ [ [ 3 2 ] [ 2 3 ] ] [ [ 1 2 ] 3 ] ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E101 stratum 2 is outside stat.test.cmh's domain:",
        ),
        (
            "[ [ [ 3 2 ] [ 2 3 ] ] [ [ 1 2 ] [ 3 ] ] ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E103",
        ),
        (
            "[ [ [ 3 2 ] [ 2 3 ] ] [ [ 1 \"a\" ] [ 3 4 ] ] ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E103",
        ),
        (
            "[ [ [ 3 2 ] [ 2 -3 ] ] [ [ 1 1 ] [ 1 1 ] ] ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E101 stratum 1 is outside stat.test.cmh's domain: -3 at [2,2]",
        ),
        (
            "[ [ [ 3 0 ] [ 2 3 ] ] [ [ 1 0 ] [ 1 1 ] ] ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E101 tables is outside stat.test.cmh's domain: an infinite common odds ratio",
        ),
        (
            "[ [ [ 0 3 ] [ 2 3 ] ] [ [ 0 1 ] [ 1 1 ] ] ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E101 tables is outside stat.test.cmh's domain: a common odds ratio of 0",
        ),
        (
            "[ [ [ 3 3 ] [ 0 0 ] ] [ [ 1 1 ] [ 0 0 ] ] ] stat.test.cmh".into(),
            "STAT.TEST.CMH: E101 tables is outside stat.test.cmh's domain: a variance of 0",
        ),
    ] {
        if want.is_empty() {
            // A one-row Friedman table is a complete block design of one
            // block, as R's: it answers.
            answer(&src);
            continue;
        }
        let e = refusal(&src);
        assert!(e.starts_with(want), "{src}: {e}");
    }
}

/// D39: the exact distribution and the Walsh averages are capped before
/// the work.
#[test]
fn the_caps_refuse_before_the_work() {
    let many = |n: usize| -> String {
        let xs: Vec<String> = (1..=n).map(|i| format!("{}.5", i)).collect();
        format!("[ {} ]", xs.join(" "))
    };
    let e = err(&format!(
        "{} 0 dict \"exact\" true set stat.test.wilcoxon.opts",
        many(1_001)
    ));
    assert!(
        e.starts_with(
            "STAT.TEST.WILCOXON.OPTS: E111 an exact signed-rank distribution of 1001 values \
             exceeds its cap of 1000"
        ),
        "{e}"
    );
    let e = err(&format!("{} 0 dict stat.test.wilcoxon.opts", many(4_473)));
    assert!(
        e.starts_with("STAT.TEST.WILCOXON.OPTS: E111 10006101 Walsh averages exceeds its cap"),
        "{e}"
    );
    // The plain word forms no Walsh average, so it answers.
    answer(&format!("{} 0 stat.test.wilcoxon", many(4_473)));
}

// Criterion 14.

#[test]
fn nothing_aborts() {
    let operands = [
        "float.NaN",
        "float.+Inf",
        "-1",
        "0",
        "1",
        "0.5",
        "\"x\"",
        "na",
        "list",
        "dict",
        "true",
        "{ 1 }",
        "[ 1 ]",
        "[ 1 1 1 ]",
        "[ 1 na ]",
        "[ 1 float.NaN ]",
        "[ 1.0e308 -1.0e308 ]",
        "[ [ 1 2 ] [ 3 ] ]",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
        "[ [ 0 0 ] [ 0 0 ] ] matrix",
        "[ [ 1 2 ] [ 3 float.NaN ] ] matrix",
        "[ [ [ 1 2 ] [ 3 4 ] ] [ [ 0 0 ] [ 0 2 ] ] ]",
        "dict \"exact\" true set",
        "dict \"conf\" 0.01 set",
        "dict \"conf\" 0.999999 set \"exact\" false set",
        "dict \"alternative\" \"less\" set \"correct\" false set",
    ];
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        for a in operands {
            for src in [
                w.to_string(),
                format!("{a} {w}"),
                format!("{a} {a} {w}"),
                format!("{a} [ 1 2 3 ] {w}"),
                format!("[ 1 2 3 4 5 6 7 8 ] {a} {w}"),
                format!("[ 1.5 -2 3 4 5 6 7 8 ] 0 {a} {w}"),
                format!("[ 1.5 -2 3 4 5 6 7 8 ] [ 1 1 1 1 1 1 1 1 ] {a} {w}"),
            ] {
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
