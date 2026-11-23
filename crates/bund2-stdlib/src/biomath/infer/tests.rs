//! RFC-0015 criteria 2, 3, 5, 6 and 9, for §S6's tests, corrections and
//! `stat.lm`.

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{close, err, fixture, floats, run, top};

const FIXTURE: &str = include_str!("../fixtures/infer.txt");

/// R's `sleep` data, `extra` by group.
const A: &str = "[ 0.7 -1.6 -0.2 -1.2 -0.1 3.4 3.7 0.8 0.0 2.0 ]";
const B: &str = "[ 1.9 0.8 1.1 0.1 -0.1 4.4 5.5 1.6 4.6 3.4 ]";

const LM: &str = "[ [ 1 2.5 ] [ 2 1.0 ] [ 3 4.0 ] [ 4 3.5 ] [ 5 6.0 ] [ 6 5.0 ] [ 7 8.5 ] [ 8 7.0 ] ] \
                  matrix [ 3.1 4.9 7.2 8.8 11.1 13.2 14.8 17.1 ]";

fn programs() -> Vec<(&'static str, String)> {
    vec![
        ("t1", format!("{A} 0.5 stat.test.t1")),
        ("t2", format!("{A} {B} stat.test.t2")),
        ("welch", format!("{A} {B} stat.test.welch")),
        ("paired", format!("{A} {B} stat.test.paired")),
        (
            "chisq",
            "[ 89 37 30 28 2 ] [ 0.40 0.20 0.20 0.19 0.01 ] stat.test.chisq".into(),
        ),
        (
            "chisq.counts",
            "[ 89 37 30 28 2 ] [ 74.8 37.4 37.4 35.53 1.87 ] stat.test.chisq".into(),
        ),
        (
            "chisq.table",
            "[ [ 762 327 468 ] [ 484 239 477 ] ] matrix stat.test.chisq.table".into(),
        ),
        (
            "fisher.tea",
            "[ [ 3 1 ] [ 1 3 ] ] matrix stat.test.fisher".into(),
        ),
        (
            "fisher.big",
            "[ [ 12 5 ] [ 7 21 ] ] matrix stat.test.fisher".into(),
        ),
        ("binom", "682 925 0.75 stat.test.binom".into()),
        ("binom.low", "3 20 0.5 stat.test.binom".into()),
        ("mannwhitney", format!("{A} {B} stat.test.mannwhitney")),
        ("ks2", format!("{A} {B} stat.test.ks2")),
    ]
}

/// The MAP `src` leaves on top.
fn map(src: &str) -> std::collections::BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap();
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn field(m: &std::collections::BTreeMap<String, BundValue>, k: &str) -> Vec<f64> {
    floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")))
}

// Criterion 2.

#[test]
fn every_word_is_registered() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| {
            (n.starts_with("stat.test.")
                || n.starts_with("stat.padjust.")
                || n.starts_with("stat.lm"))
                // RFC-0019 §S5's, which `further`'s own test holds.
                && !super::further::WORDS.contains(&n.as_str())
        })
        .count();
    assert_eq!(ours, WORDS.len());
}

// Criterion 9.

/// Each test's statistic, p and df against the fixture: a statistic to 1e-12,
/// a closed form; p and Welch's df to 1e-8, since a p is a series or
/// continued fraction (B11).
#[test]
fn each_test_against_the_fixture() {
    for (label, src) in programs() {
        let m = map(&src);
        for (k, tol) in [("statistic", 1e-12), ("p", 1e-8), ("df", 1e-8)] {
            let line = format!("{label} {k} ");
            if FIXTURE.lines().any(|l| l.starts_with(&line)) {
                let want = fixture(FIXTURE, &format!("{label} {k}"));
                close(&format!("{label} {k}"), &field(&m, k), &want, tol);
            }
        }
        assert!(m.contains_key("method"), "{label}");
    }
}

/// R's printed answers for the same data, from R's help pages and the
/// `sleep` data: what a reader of `?t.test`, `?chisq.test`, `?fisher.test`
/// and `?binom.test` would see, to the digits R prints. R is not installed
/// where this was written; these agree with the fixture's to every printed
/// digit, which is the check.
#[test]
fn rs_printed_answers() {
    let near = |label: &str, k: &str, want: f64, digits: f64| {
        let src = programs().into_iter().find(|(l, _)| *l == label).unwrap().1;
        let got = field(&map(&src), k)[0];
        assert!(
            (got - want).abs() <= 0.5 * 10f64.powf(-digits),
            "{label} {k}: {got} against R's {want}"
        );
    };
    // t.test(extra ~ group, data = sleep): t = -1.8608, df = 17.776, p = 0.07939.
    near("welch", "statistic", -1.8608, 4.0);
    near("welch", "df", 17.776, 3.0);
    near("welch", "p", 0.07939, 5.0);
    // var.equal = TRUE: p = 0.07919.
    near("t2", "p", 0.07919, 5.0);
    // paired = TRUE: t = -4.0621, df = 9, p = 0.002833.
    near("paired", "statistic", -4.0621, 4.0);
    near("paired", "p", 0.002833, 6.0);
    // chisq.test(x, p = p): X-squared = 5.7947, df = 4, p = 0.215.
    near("chisq", "statistic", 5.7947, 4.0);
    near("chisq", "p", 0.215, 3.0);
    // chisq.test(M): X-squared = 30.07, df = 2, p = 2.954e-07.
    near("chisq.table", "statistic", 30.07, 2.0);
    near("chisq.table", "p", 2.954e-7, 10.0);
    // fisher.test(TeaTasting): p = 0.4857.
    near("fisher.tea", "p", 0.4857, 4.0);
    // binom.test(682, 925, p = 0.75): p = 0.3825.
    near("binom", "p", 0.3825, 4.0);
    // wilcox.test(extra ~ group, data = sleep): W = 25.5, p = 0.06933.
    near("mannwhitney", "statistic", 25.5, 1.0);
    near("mannwhitney", "p", 0.06933, 5.0);
}

#[test]
fn the_corrections_against_the_fixture() {
    let ps = "[ 0.01 0.02 0.03 0.04 0.05 0.5 0.001 0.04 ]";
    for w in ["bonferroni", "holm", "bh"] {
        let got = top(&format!("{ps} stat.padjust.{w}"));
        close(w, &got, &fixture(FIXTURE, &format!("padjust {w}")), 1e-15);
    }
    assert_eq!(top("list stat.padjust.bh"), Vec::<f64>::new());
}

#[test]
fn lm_against_the_fixture() {
    let m = map(&format!("{LM} stat.lm"));
    for k in ["coef", "se", "t", "p"] {
        close(
            k,
            &field(&m, k),
            &fixture(FIXTURE, &format!("lm {k}")),
            1e-8,
        );
    }
    close("r2", &field(&m, "r2"), &fixture(FIXTURE, "lm r2"), 1e-12);
    assert_eq!(field(&m, "df"), [5.0]);
}

/// All four forms of a correction, and both of every other word.
#[test]
fn the_forms() {
    let ps = "[ 0.01 0.04 0.03 ]";
    let plain = top(&format!("{ps} stat.padjust.holm"));
    // `,` keeps the series.
    let mut i = run(&format!("{ps} stat.padjust.holm,")).unwrap();
    assert_eq!(floats(&i.pull().unwrap()), plain);
    assert_eq!(floats(&i.pull().unwrap()), [0.01, 0.04, 0.03]);
    // `.` reads the workbench and answers there; `.,` keeps it.
    let mut i = run(&format!("{ps} . stat.padjust.holm.")).unwrap();
    assert_eq!(floats(&i.pull_workbench().unwrap()), plain);
    let mut i = run(&format!("{ps} . stat.padjust.holm.,")).unwrap();
    assert_eq!(floats(&i.pull_workbench().unwrap()), plain);
    assert_eq!(floats(&i.pull_workbench().unwrap()), [0.01, 0.04, 0.03]);
    // The rest take the plain word's top operand from the workbench.
    for (below, top_, w) in [
        (A.to_string(), "0.5".to_string(), "stat.test.t1"),
        (A.to_string(), B.to_string(), "stat.test.welch"),
        ("682 925".to_string(), "0.75".to_string(), "stat.test.binom"),
        (
            String::new(),
            "[ [ 3 1 ] [ 1 3 ] ] matrix".to_string(),
            "stat.test.fisher",
        ),
        (
            "[ [ 1 ] [ 2 ] [ 3 ] [ 4 ] ] matrix".to_string(),
            "[ 1.0 2.5 2.5 4.5 ]".to_string(),
            "stat.lm",
        ),
    ] {
        let plain = run(&format!("{below} {top_} {w}")).unwrap().pull().unwrap();
        let mut i = run(&format!("7 {below} {top_} . {w}.")).unwrap();
        let shown: Vec<String> = i.snapshot().iter().map(BundValue::display).collect();
        assert_eq!(shown, ["7"], "{w}.");
        assert_eq!(
            i.pull_workbench().unwrap().display(),
            plain.display(),
            "{w}."
        );
    }
}

// Criterion 3.

#[test]
fn each_refusal_is_its_template() {
    for (src, want) in [
        (
            "[ 1 1 1 ] 0 stat.test.t1",
            "STAT.TEST.T1: E101 series is outside stat.test.t1's domain: a constant series",
        ),
        (
            "[ 1 ] [ 2 3 ] stat.test.t2",
            "STAT.TEST.T2: E101 series is outside stat.test.t2's domain: a series of 1 value",
        ),
        (
            "[ 1 2 3 ] [ 1 2 ] stat.test.paired",
            "STAT.TEST.PAIRED: E102 lengths do not agree: 3 and 2",
        ),
        (
            "[ 1 2 ] 3 stat.test.welch",
            "STAT.TEST.WELCH: E115 b must be a LIST or METRICS; a drained series is not allowed here",
        ),
        (
            "[ 5 -1 ] [ 0.5 0.5 ] stat.test.chisq",
            "STAT.TEST.CHISQ: E101 observed is outside stat.test.chisq's domain: -1 at 2",
        ),
        (
            "[ 5 1 ] [ 1.0 0.0 ] stat.test.chisq",
            "STAT.TEST.CHISQ: E101 expected is outside stat.test.chisq's domain: 0 at 2",
        ),
        (
            "[ [ 1 2 ] [ 0 0 ] ] matrix stat.test.chisq.table",
            "STAT.TEST.CHISQ.TABLE: E101 table is outside stat.test.chisq.table's domain: \
             row 2 sums to 0",
        ),
        (
            "[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ] matrix stat.test.fisher",
            "STAT.TEST.FISHER: E102 rows do not agree: 3 and 2",
        ),
        (
            "[ [ 1 2.5 ] [ 3 4 ] ] matrix stat.test.fisher",
            "STAT.TEST.FISHER: E101 table is outside stat.test.fisher's domain: 2.5 at [1,2]",
        ),
        (
            "[ [ 1 2 ] [ 3 ] ] matrix stat.test.chisq.table",
            "STAT.TEST.CHISQ.TABLE: E103 table is not a MATRIX of numbers: row 2 has 1 cells, row 1 2",
        ),
        (
            "[ 1 2 ] stat.test.fisher",
            "STAT.TEST.FISHER: E101 table is outside stat.test.fisher's domain: list/2 [1, 2]",
        ),
        (
            "5 3 0.5 stat.test.binom",
            "STAT.TEST.BINOM: E101 k is outside stat.test.binom's domain: 5",
        ),
        (
            "1 3 1.5 stat.test.binom",
            "STAT.TEST.BINOM: E101 p is outside stat.test.binom's domain: 1.5",
        ),
        (
            "[ 0.5 1.5 ] stat.padjust.bh",
            "STAT.PADJUST.BH: E101 series is outside stat.padjust.bh's domain: 1.5 at 2",
        ),
        (
            "[ [ 1 2 ] [ 2 4 ] [ 3 6 ] [ 4 8 ] ] matrix [ 1.0 2.0 3.5 3.0 ] stat.lm",
            "STAT.LM: E106 the matrix is singular",
        ),
        (
            "[ [ 1 ] [ 2 ] ] matrix [ 1.0 2.0 ] stat.lm",
            "STAT.LM: E101 y is outside stat.lm's domain: 2 observations for 2 coefficients",
        ),
        (
            "[ [ 1 ] [ 2 ] [ 3 ] ] matrix [ 1.0 2.0 ] stat.lm",
            "STAT.LM: E102 lengths do not agree: 3 and 2",
        ),
    ] {
        assert_eq!(err(src), want, "{src}");
    }
}

// Criterion 5: no abort, whatever the operand.

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "float.+Inf",
        "-0.1",
        "1.1",
        "0",
        "1",
        "\"x\"",
        "list",
        "[ 1 ]",
        "[ 1 1 1 ]",
        "[ 1 float.NaN ]",
        "[ 10.0 308.0 ]",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
        "[ [ 0 0 ] [ 0 0 ] ] matrix",
    ];
    for w in WORDS
        .iter()
        .filter(|w| !w.ends_with('.') && !w.ends_with(','))
    {
        for o in operands {
            for src in [
                format!("{o} {w}"),
                format!("{o} {o} {w}"),
                format!("{o} {o} {o} {w}"),
            ] {
                match run(&src) {
                    Ok(_) => {}
                    Err(e) => assert!(
                        e.contains(": E1") || e.contains("too shallow") || e.contains("returned"),
                        "{src}: {e}"
                    ),
                }
            }
        }
    }
}

// Criterion 6.

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i = run(":stat.test.t2 { 42 } register 1 2 stat.test.t2").unwrap();
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}

/// W at its mean, n·m/2: the continuity correction is R's `sign(z)·½`,
/// which is 0 there, so z is 0 and p is 1. Bund2 once took
/// `f64::signum(0.0)`, which is 1, and answered p = 0.6985 here
/// (`docs/rfc/reviews/RFC-0016-review-2026-11-16.md`, S11).
#[test]
fn mannwhitney_at_the_mean_of_w_answers_p_1() {
    let mut i = run("[ 1.0 4.0 ] [ 2.0 3.0 ] stat.test.mannwhitney").unwrap();
    let v = i.pull().unwrap();
    let m = v.as_map().unwrap();
    assert_eq!(floats(&m["statistic"]), [2.0]);
    assert_eq!(floats(&m["p"]), [1.0]);
}

// RFC-0016 criterion 3: the plain tests' answers, every key, each number's
// f64 bits, recorded from the code before RFC-0016 moved any (cae380d).

const SNAPSHOT: &str = include_str!("../fixtures/infer.snapshot");

/// Each program's answer as lines `label key value`, a number as its f64
/// bits in hex and a STRING as itself.
fn snapshot() -> String {
    let mut progs = programs();
    progs.push((
        "mannwhitney.mean",
        "[ 1.0 4.0 ] [ 2.0 3.0 ] stat.test.mannwhitney".into(),
    ));
    let mut out = String::new();
    for (label, src) in progs {
        for (k, v) in map(&src) {
            let shown = match v.as_str() {
                Some(s) => s,
                None => format!("{:016x}", floats(&v)[0].to_bits()),
            };
            out.push_str(&format!("{label} {k} {shown}\n"));
        }
    }
    out
}

#[test]
fn the_plain_tests_answer_the_snapshot_bit_for_bit() {
    assert_eq!(snapshot(), SNAPSHOT);
}

/// Writes the snapshot. Run once, before RFC-0016's first change, with
/// `cargo test -p bund2-stdlib write_the_snapshot -- --ignored`.
#[test]
#[ignore = "writes the snapshot; run only to record it"]
fn write_the_snapshot() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/biomath/fixtures/infer.snapshot"
    );
    std::fs::write(path, snapshot()).unwrap();
}

// RFC-0016 criterion 8: Hochberg, Hommel and Benjamini–Yekutieli.

const FIXTURE16: &str = include_str!("../fixtures/infer16.txt");

fn p20() -> String {
    let ps = fixture(FIXTURE16, "padjust20 p");
    assert_eq!(ps.len(), 20);
    let cells: Vec<String> = ps.iter().map(|p| format!("{p:?}")).collect();
    format!("[ {} ]", cells.join(" "))
}

#[test]
fn rfc0016_corrections_against_the_fixture() {
    let ps = p20();
    for w in ["hochberg", "hommel", "by"] {
        let got = top(&format!("{ps} stat.padjust.{w}"));
        let want = fixture(FIXTURE16, &format!("padjust20 {w}"));
        close(w, &got, &want, 1e-12);
    }
}

/// Hommel's steps against its definition, the largest Simes p over every
/// subset containing i, on every subset of small series.
#[test]
fn hommel_is_its_subset_definition() {
    fn simes(ps: &[f64]) -> f64 {
        let mut s = ps.to_vec();
        s.sort_by(f64::total_cmp);
        let k = s.len() as f64;
        s.iter()
            .enumerate()
            .map(|(j, p)| k * p / (j + 1) as f64)
            .fold(f64::INFINITY, f64::min)
    }
    let cases: [&[f64]; 6] = [
        &[0.01],
        &[0.04, 0.01],
        &[0.01, 0.02, 0.03, 0.04, 0.05],
        &[0.2, 0.001, 0.04, 0.04, 0.5, 0.012, 1.0],
        &[0.003, 0.9, 0.02, 0.3, 0.0, 0.07, 0.011, 0.6],
        &[0.5, 0.5, 0.5, 0.01],
    ];
    for ps in cases {
        let n = ps.len();
        let mut want = vec![0.0f64; n];
        for mask in 1u32..(1 << n) {
            let sub: Vec<f64> = (0..n)
                .filter(|i| mask >> i & 1 == 1)
                .map(|i| ps[i])
                .collect();
            let s = simes(&sub);
            for i in (0..n).filter(|i| mask >> i & 1 == 1) {
                want[i] = want[i].max(s);
            }
        }
        let want: Vec<f64> = want.iter().map(|w| w.min(1.0)).collect();
        let cells: Vec<String> = ps.iter().map(|p| format!("{p:?}")).collect();
        let got = top(&format!("[ {} ] stat.padjust.hommel", cells.join(" ")));
        close(&format!("{ps:?}"), &got, &want, 1e-15);
    }
}

#[test]
fn rfc0016_corrections_edges() {
    for w in ["hochberg", "hommel", "by"] {
        let mut i = run(&format!("list stat.padjust.{w}")).unwrap();
        assert_eq!(i.pull().unwrap().as_list().map(<[_]>::len), Some(0), "{w}");
        assert_eq!(top(&format!("[ 0.3 ] stat.padjust.{w}")), [0.3], "{w}");
        assert_eq!(
            err(&format!("[ 0.1 1.5 ] stat.padjust.{w}")),
            format!(
                "STAT.PADJUST.{}: E101 series is outside stat.padjust.{w}'s domain: 1.5 at 2",
                w.to_uppercase()
            )
        );
    }
    // Every form answers the plain word's values.
    let ps = "[ 0.01 0.04 0.03 ]";
    for w in ["hochberg", "hommel", "by"] {
        let plain = top(&format!("{ps} stat.padjust.{w}"));
        let mut i = run(&format!("{ps} . stat.padjust.{w}.")).unwrap();
        assert_eq!(floats(&i.pull_workbench().unwrap()), plain, "{w}");
    }
    // BY is BH times Σ 1/k, capped at 1.
    let bh = top(&format!("{ps} stat.padjust.bh"));
    let by = top(&format!("{ps} stat.padjust.by"));
    let c = 1.0 + 0.5 + 1.0 / 3.0;
    close(
        "by",
        &by,
        &bh.iter().map(|b| (b * c).min(1.0)).collect::<Vec<_>>(),
        1e-15,
    );
}

#[test]
fn hommel_above_its_cap_is_e111() {
    let ps = format!("[ {} ]", vec!["0.5"; 10_001].join(" "));
    assert_eq!(
        err(&format!("{ps} stat.padjust.hommel")),
        "STAT.PADJUST.HOMMEL: E111 a series of 10001 p-values exceeds its cap of 10000"
    );
    let ps = format!("[ {} ]", vec!["0.5"; 10_000].join(" "));
    assert_eq!(top(&format!("{ps} stat.padjust.hommel")).len(), 10_000);
}

// RFC-0016 M2: the t tests' and the binomial's `.opts` words (§S2),
// criteria 1, 2, 3, 4 and 5's first part. The fixture is `infer16.py`'s,
// at 50 digits; no R output is committed for these, so it alone is the
// check (A7).

/// Each `.opts` word of M2 with its operands, the defaults' options, and
/// the plain word it extends.
fn opts_programs() -> Vec<(&'static str, String, String)> {
    vec![
        (
            "t1",
            format!("{A} 0.5 dict stat.test.t1.opts"),
            format!("{A} 0.5 stat.test.t1"),
        ),
        (
            "t2",
            format!("{A} {B} dict stat.test.t2.opts"),
            format!("{A} {B} stat.test.t2"),
        ),
        (
            "welch",
            format!("{A} {B} dict stat.test.welch.opts"),
            format!("{A} {B} stat.test.welch"),
        ),
        (
            "paired",
            format!("{A} {B} dict stat.test.paired.opts"),
            format!("{A} {B} stat.test.paired"),
        ),
        (
            "binom",
            "682 925 0.75 dict stat.test.binom.opts".into(),
            "682 925 0.75 stat.test.binom".into(),
        ),
        (
            "binom.low",
            "3 20 0.5 dict stat.test.binom.opts".into(),
            "3 20 0.5 stat.test.binom".into(),
        ),
    ]
}

/// Whether the MAP `src` leaves has `key`, asked as a program asks it.
fn has(src: &str, key: &str) -> bool {
    let mut i = run(&format!("{src} \"{key}\" ?key")).unwrap_or_else(|e| panic!("{src}: {e}"));
    match i.pull().unwrap() {
        BundValue::Bool(b, _) => b,
        v => panic!("{src} {key} ?key: {v:?}"),
    }
}

fn text(m: &std::collections::BTreeMap<String, BundValue>, k: &str) -> String {
    m.get(k)
        .and_then(BundValue::as_str)
        .unwrap_or_else(|| panic!("no {k} STRING in {m:?}"))
}

#[test]
fn rfc0016_m2_words_are_registered_with_their_effects() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    let shape = |w: &str| {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        (e.consumes, e.produces, e.opaque)
    };
    assert!(shape("stat.test.t1.opts").2);
    assert!(shape("stat.test.t1.opts.").2);
    for w in ["t2", "welch", "paired"] {
        assert_eq!(shape(&format!("stat.test.{w}.opts")), (3, 1, false), "{w}");
        assert_eq!(shape(&format!("stat.test.{w}.opts.")), (2, 0, false), "{w}");
    }
    assert_eq!(shape("stat.test.binom.opts"), (4, 1, false));
    assert_eq!(shape("stat.test.binom.opts."), (3, 0, false));
}

/// Criterion 3: with the defaults, each `.opts` word's `statistic`, `p` and
/// `df` are its plain word's, bit for bit, and so is its `method`.
#[test]
fn rfc0016_opts_defaults_are_the_plain_words_bit_for_bit() {
    for (label, opts, plain) in opts_programs() {
        let (o, p) = (map(&opts), map(&plain));
        for k in ["statistic", "p", "df"] {
            assert_eq!(
                o.get(k).map(|_| field(&o, k)[0].to_bits()),
                p.get(k).map(|_| field(&p, k)[0].to_bits()),
                "{label} {k}"
            );
        }
        assert_eq!(text(&o, "method"), text(&p, "method"), "{label}");
        assert_eq!(text(&o, "alternative"), "two.sided", "{label}");
        assert_eq!(field(&o, "conf"), [0.95], "{label}");
    }
}

/// Criterion 4: each t word against the fixture, two-sided at 95% and 99%,
/// one-sided, and with `mu` 1.0 for the two-sample words.
#[test]
fn rfc0016_t_intervals_against_the_fixture() {
    let want = |label: &str, k: &str| fixture(FIXTURE16, &format!("{label}.opts {k}"));
    let cases = [
        ("t1", format!("{A} 0.0"), "One Sample t-test"),
        ("t2", format!("{A} {B}"), "Two Sample t-test"),
        ("welch", format!("{A} {B}"), "Welch Two Sample t-test"),
        ("paired", format!("{A} {B}"), "Paired t-test"),
    ];
    for (label, ops, method) in &cases {
        let w = format!("stat.test.{label}.opts");
        let src = format!("{ops} dict {w}");
        let m = map(&src);
        close(
            label,
            &field(&m, "statistic"),
            &want(label, "statistic"),
            1e-12,
        );
        close(label, &field(&m, "p"), &want(label, "p"), 1e-8);
        close(label, &field(&m, "df"), &want(label, "df"), 1e-12);
        close(
            label,
            &field(&m, "estimate"),
            &want(label, "estimate"),
            1e-12,
        );
        close(label, &field(&m, "lower"), &want(label, "lower"), 1e-9);
        close(label, &field(&m, "upper"), &want(label, "upper"), 1e-9);
        assert_eq!(text(&m, "method"), *method, "{label}");

        let m = map(&format!("{ops} dict \"conf\" 0.99 set {w}"));
        close(label, &field(&m, "lower"), &want(label, "lower99"), 1e-9);
        close(label, &field(&m, "upper"), &want(label, "upper99"), 1e-9);
        assert_eq!(field(&m, "conf"), [0.99]);

        let less = format!("{ops} dict \"alternative\" \"less\" set {w}");
        let m = map(&less);
        close(label, &field(&m, "p"), &want(label, "less.p"), 1e-8);
        close(label, &field(&m, "upper"), &want(label, "less.upper"), 1e-9);
        assert!(!has(&less, "lower"), "{label}");
        assert!(has(&less, "upper"), "{label}");
        assert_eq!(text(&m, "alternative"), "less");

        let greater = format!("{ops} dict \"alternative\" \"greater\" set {w}");
        let m = map(&greater);
        close(label, &field(&m, "p"), &want(label, "greater.p"), 1e-8);
        close(
            label,
            &field(&m, "lower"),
            &want(label, "greater.lower"),
            1e-9,
        );
        assert!(!has(&greater, "upper"), "{label}");

        if *label != "t1" {
            let shifted = map(&format!("{ops} dict \"mu\" 1.0 set {w}"));
            close(
                label,
                &field(&shifted, "statistic"),
                &want(label, "mu1.statistic"),
                1e-12,
            );
            close(label, &field(&shifted, "p"), &want(label, "mu1.p"), 1e-8);
            // The estimate and interval are of the difference itself.
            let plain = map(&src);
            for k in ["estimate", "lower", "upper", "df"] {
                assert_eq!(field(&shifted, k), field(&plain, k), "{label} {k}");
            }
            // An INTEGER mu converts (B15).
            let int = map(&format!("{ops} dict \"mu\" 1 set {w}"));
            assert_eq!(int, shifted, "{label}");
        }
    }
}

/// Criterion 5's first part: Clopper–Pearson and the exact one-sided p.
#[test]
fn rfc0016_binom_interval_against_the_fixture() {
    let want = |k: &str| fixture(FIXTURE16, &format!("binom.opts {k}"));
    let w = "682 925 0.75";
    let m = map(&format!("{w} dict stat.test.binom.opts"));
    close("estimate", &field(&m, "estimate"), &want("estimate"), 1e-15);
    close("lower", &field(&m, "lower"), &want("lower"), 1e-9);
    close("upper", &field(&m, "upper"), &want("upper"), 1e-9);
    assert_eq!(text(&m, "method"), "Exact binomial test");
    assert_eq!(field(&m, "statistic"), [682.0]);

    let m = map(&format!(
        "{w} dict \"alternative\" \"less\" set stat.test.binom.opts"
    ));
    close("less p", &field(&m, "p"), &want("less.p"), 1e-9);
    assert_eq!(field(&m, "lower"), [0.0]);
    close("less upper", &field(&m, "upper"), &want("less.upper"), 1e-9);

    let m = map(&format!(
        "{w} dict \"alternative\" \"greater\" set stat.test.binom.opts"
    ));
    close("greater p", &field(&m, "p"), &want("greater.p"), 1e-9);
    close(
        "greater lower",
        &field(&m, "lower"),
        &want("greater.lower"),
        1e-9,
    );
    assert_eq!(field(&m, "upper"), [1.0]);

    // The ends by definition, without a quantile at a zero shape.
    let m = map("0 10 0.5 dict stat.test.binom.opts");
    assert_eq!(field(&m, "lower")[0].to_bits(), 0.0f64.to_bits());
    assert!(field(&m, "upper")[0] > 0.0 && field(&m, "upper")[0] < 1.0);
    let m = map("10 10 0.5 dict stat.test.binom.opts");
    assert_eq!(field(&m, "upper"), [1.0]);
    assert!(field(&m, "lower")[0] > 0.0 && field(&m, "lower")[0] < 1.0);
    // P(X >= 0) is 1.
    let m = map("0 10 0.5 dict \"alternative\" \"greater\" set stat.test.binom.opts");
    assert_eq!(field(&m, "p"), [1.0]);
}

#[test]
fn rfc0016_opts_forms_answer_the_plain_words() {
    for (label, src) in [
        (
            "t1",
            format!("{A} 0.0 dict \"conf\" 0.9 set . stat.test.t1.opts."),
        ),
        (
            "t2",
            format!("{A} {B} dict \"mu\" 0.5 set . stat.test.t2.opts."),
        ),
        ("welch", format!("{A} {B} dict . stat.test.welch.opts.")),
        ("paired", format!("{A} {B} dict . stat.test.paired.opts.")),
        (
            "binom",
            "682 925 0.75 dict \"alternative\" \"less\" set . stat.test.binom.opts.".into(),
        ),
    ] {
        let mut i = run(&src).unwrap_or_else(|e| panic!("{src}: {e}"));
        assert_eq!(i.depth(), 0, "{label}");
        let got = i.pull_workbench().unwrap();
        let plain = map(&src.replace(" . ", " ").replace("opts.", "opts"));
        assert_eq!(got.as_map().unwrap(), &plain, "{label}");
    }
}

/// Criterion 2, for the options MAP.
#[test]
fn rfc0016_opts_refusals_are_their_templates() {
    let t2 = |o: &str| err(&format!("{A} {B} {o} stat.test.t2.opts"));
    assert_eq!(
        t2("dict \"level\" 0.9 set"),
        "STAT.TEST.T2.OPTS: E101 opts is outside stat.test.t2.opts's domain: an unknown key \"level\""
    );
    assert_eq!(
        t2("dict \"conf\" 1.0 set"),
        "STAT.TEST.T2.OPTS: E101 conf is outside stat.test.t2.opts's domain: 1.0"
    );
    assert!(t2("dict \"conf\" 0.0 set").starts_with("STAT.TEST.T2.OPTS: E101 conf "));
    assert!(t2("dict \"conf\" 0 set").starts_with("STAT.TEST.T2.OPTS: E101 conf "));
    assert!(t2("dict \"conf\" -0.5 set").starts_with("STAT.TEST.T2.OPTS: E101 conf "));
    assert!(t2("dict \"alternative\" 1.0 set").starts_with("STAT.TEST.T2.OPTS: E101 alternative "));
    assert!(
        t2("dict \"alternative\" \"two-sided\" set")
            .starts_with("STAT.TEST.T2.OPTS: E101 alternative ")
    );
    assert!(t2("dict \"mu\" \"x\" set").starts_with("STAT.TEST.T2.OPTS: E101 mu "));
    assert!(t2("dict \"mu\" float.+Inf set").starts_with("STAT.TEST.T2.OPTS: E101 mu "));
    assert!(t2("1.0").starts_with("STAT.TEST.T2.OPTS: E101 opts "));
    assert_eq!(
        err(&format!("{A} 0.0 dict \"mu\" 1.0 set stat.test.t1.opts")),
        "STAT.TEST.T1.OPTS: E101 opts is outside stat.test.t1.opts's domain: \
         the key \"mu\", which this word does not take"
    );
    assert!(
        err("682 925 0.75 dict \"mu\" 1.0 set stat.test.binom.opts")
            .starts_with("STAT.TEST.BINOM.OPTS: E101 opts ")
    );
    // The plain word's own refusals, under the `.opts` name.
    assert!(
        err("926 925 0.75 dict stat.test.binom.opts").starts_with("STAT.TEST.BINOM.OPTS: E101 k ")
    );
    assert!(
        err("[ 1.0 1.0 ] [ 1.0 1.0 ] dict stat.test.t2.opts")
            .starts_with("STAT.TEST.T2.OPTS: E101 a and b ")
    );
    // n = 0 has no estimate.
    assert!(err("0 0 0.5 dict stat.test.binom.opts").starts_with("STAT.TEST.BINOM.OPTS: E114 "));
    // An overflowing difference is the statistic's E114, as the plain word's.
    let big = "[ 1.7e308 1.6e308 ] [ -1.7e308 -1.6e308 ]";
    assert_eq!(
        err(&format!("{big} dict stat.test.welch.opts")),
        "STAT.TEST.WELCH.OPTS: E114 the answer overflows a FLOAT: the test statistic"
    );
    assert_eq!(
        err(&format!("{big} stat.test.welch")),
        "STAT.TEST.WELCH: E114 the answer overflows a FLOAT: the test statistic"
    );
    // S1: an end that overflows is the whole word's E114, where the plain
    // word, which has no interval, still answers (criterion 3).
    let near = "[ 1.0e308 5.0e307 ] 7.5e307";
    assert_eq!(
        err(&format!("{near} dict stat.test.t1.opts")),
        "STAT.TEST.T1.OPTS: E114 the answer overflows a FLOAT: the interval's lower end"
    );
    assert_eq!(
        field(&map(&format!("{near} stat.test.t1")), "statistic"),
        [0.0]
    );
}

#[test]
fn rfc0016_opts_no_operand_aborts() {
    let operands = [
        "float.NaN",
        "0.5",
        "0",
        "10",
        "\"x\"",
        "list",
        "[ 1 ]",
        "[ 1 1 1 ]",
        "[ 1 2 4 ]",
        "[ 1 float.NaN ]",
        "dict",
        "dict \"conf\" 0.999999999999 set",
        "dict \"conf\" 1.0e-300 set",
        "dict \"alternative\" \"less\" set",
        "dict \"mu\" 1.0e308 set",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
    ];
    let words = [
        "stat.test.t1.opts",
        "stat.test.t2.opts",
        "stat.test.welch.opts",
        "stat.test.paired.opts",
        "stat.test.binom.opts",
    ];
    for w in words {
        for (i, o) in operands.iter().enumerate() {
            let p = operands[(i + 3) % operands.len()];
            let q = operands[(i + 7) % operands.len()];
            for src in [
                format!("{o} {w}"),
                format!("{o} {p} {w}"),
                format!("{o} {p} {q} {w}"),
                format!("{o} {p} {q} {o} {w}"),
                format!("{o} {p} {o} dict {w}"),
                format!("{p} {o} {q} {o} {w}"),
                format!("{o} {p} {q} {o} . {w}."),
            ] {
                if let Err(e) = run(&src) {
                    assert!(
                        e.contains(": E1") || e.contains("too shallow") || e.contains("returned"),
                        "{src}: {e}"
                    );
                }
            }
        }
    }
}

// RFC-0016 M3: Fisher's, Mann–Whitney's and the KS test's `.opts` words
// (§S2), criteria 1, 2, 3, 5's second part and 6. The fixture is
// `infer16.py`'s; no R output is committed for these, so it alone is the
// check (A7), except criterion 6's ends, which the RFC states.

const HL_METHOD: &str = "Wilcoxon rank sum test with continuity correction; \
                         Hodges–Lehmann estimate, order-statistic interval";

/// bⱼ = 1.001·j + 0.5 for j = 0 … m − 1, each as Rust writes the f64, which
/// reads back as the same f64.
fn spaced(m: usize) -> String {
    let b: Vec<String> = (0..m)
        .map(|j| format!("{:?}", 1.001 * j as f64 + 0.5))
        .collect();
    format!("[ {} ]", b.join(" "))
}

/// 1.0 … n.0.
fn counting(n: usize) -> String {
    let a: Vec<String> = (1..=n).map(|i| format!("{i}.0")).collect();
    format!("[ {} ]", a.join(" "))
}

fn hl(a: &str, b: &str, opts: &str) -> std::collections::BTreeMap<String, BundValue> {
    map(&format!("{a} {b} dict {opts} stat.test.mannwhitney.opts"))
}

fn one(m: &std::collections::BTreeMap<String, BundValue>, k: &str) -> f64 {
    field(m, k)[0]
}

#[test]
fn rfc0016_m3_words_are_registered_with_their_effects() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    let shape = |w: &str| {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        (e.consumes, e.produces, e.opaque)
    };
    assert_eq!(shape("stat.test.fisher.opts"), (2, 1, false));
    assert_eq!(shape("stat.test.fisher.opts."), (1, 0, false));
    for w in ["mannwhitney", "ks2"] {
        assert_eq!(shape(&format!("stat.test.{w}.opts")), (3, 1, false), "{w}");
        assert_eq!(shape(&format!("stat.test.{w}.opts.")), (2, 0, false), "{w}");
    }
}

/// Criterion 3: with the defaults, `statistic`, `p` and `df` are the plain
/// word's, bit for bit.
#[test]
fn rfc0016_m3_defaults_are_the_plain_words_bit_for_bit() {
    let cases = [
        ("[ [ 3 1 ] [ 1 3 ] ] matrix", "fisher"),
        ("[ [ 12 5 ] [ 7 21 ] ] matrix", "fisher"),
        (&format!("{A} {B}") as &str, "mannwhitney"),
        ("[ 1.1 2.2 3.3 4.4 ] [ 0.5 1.5 2.0 2.5 3.0 ]", "mannwhitney"),
        (&format!("{A} {B}") as &str, "ks2"),
    ];
    for (ops, w) in cases {
        let o = map(&format!("{ops} dict stat.test.{w}.opts"));
        let p = map(&format!("{ops} stat.test.{w}"));
        for k in ["statistic", "p", "df"] {
            assert_eq!(
                o.get(k).map(|_| field(&o, k)[0].to_bits()),
                p.get(k).map(|_| field(&p, k)[0].to_bits()),
                "{ops} {w} {k}"
            );
        }
        assert_eq!(text(&o, "alternative"), "two.sided");
    }
    // The method: the plain word's for KS, S2's own for the other two.
    let ks = map(&format!("{A} {B} dict stat.test.ks2.opts"));
    assert_eq!(
        text(&ks, "method"),
        "Asymptotic two-sample Kolmogorov-Smirnov test"
    );
    assert!(!ks.contains_key("conf") && !ks.contains_key("estimate"));
    assert_eq!(text(&hl(A, B, ""), "method"), HL_METHOD);
}

/// Criterion 5's second part: Fisher's estimate, ends and p.
#[test]
fn rfc0016_fisher_against_the_fixture() {
    let want = |k: &str| fixture(FIXTURE16, k)[0];
    let tea = "[ [ 3 1 ] [ 1 3 ] ] matrix";
    let w = "stat.test.fisher.opts";
    let m = map(&format!("{tea} dict {w}"));
    assert_eq!(
        text(&m, "method"),
        "Fisher's Exact Test for Count Data; \
         conditional maximum likelihood estimate of the odds ratio"
    );
    close(
        "estimate",
        &[one(&m, "estimate")],
        &[want("fisher.tea.two.sided estimate")],
        1e-8,
    );
    close(
        "lower",
        &[one(&m, "lower")],
        &[want("fisher.tea.two.sided lower")],
        1e-8,
    );
    close(
        "upper",
        &[one(&m, "upper")],
        &[want("fisher.tea.two.sided upper")],
        1e-8,
    );
    close(
        "p",
        &[one(&m, "p")],
        &fixture(FIXTURE, "fisher.tea p"),
        1e-12,
    );

    let greater = format!("{tea} dict \"alternative\" \"greater\" set {w}");
    let m = map(&greater);
    close(
        "greater lower",
        &[one(&m, "lower")],
        &[want("fisher.tea.greater lower")],
        1e-8,
    );
    close(
        "greater p",
        &[one(&m, "p")],
        &[want("fisher.tea.greater p")],
        1e-12,
    );
    assert!(!has(&greater, "upper"));
    assert!(has(&greater, "estimate"));

    let m = map(&format!("{tea} dict \"alternative\" \"less\" set {w}"));
    close(
        "less upper",
        &[one(&m, "upper")],
        &[want("fisher.tea.less upper")],
        1e-8,
    );
    close(
        "less p",
        &[one(&m, "p")],
        &[want("fisher.tea.less p")],
        1e-12,
    );
    assert_eq!(one(&m, "lower"), 0.0);

    // x at the support's maximum: no estimate and no upper end.
    let top = format!("[ [ 3 0 ] [ 0 3 ] ] matrix dict {w}");
    assert!(!has(&top, "estimate"));
    assert!(!has(&top, "upper"));
    close(
        "zero lower",
        &[one(&map(&top), "lower")],
        &[want("fisher.zero.two.sided lower")],
        1e-8,
    );
    // At its minimum: both the estimate and the lower end are 0.
    let m = map(&format!("[ [ 0 3 ] [ 3 0 ] ] matrix dict {w}"));
    assert_eq!((one(&m, "estimate"), one(&m, "lower")), (0.0, 0.0));
    assert!(one(&m, "upper") > 0.0);
}

/// Criterion 5's second part: KS for each alternative.
#[test]
fn rfc0016_ks2_against_the_fixture() {
    let w = |alt: &str| {
        map(&format!(
            "{A} {B} dict \"alternative\" \"{alt}\" set stat.test.ks2.opts"
        ))
    };
    let m = w("two.sided");
    close(
        "D",
        &[one(&m, "statistic")],
        &fixture(FIXTURE, "ks2 statistic"),
        1e-12,
    );
    close("p", &[one(&m, "p")], &fixture(FIXTURE, "ks2 p"), 1e-8);
    for alt in ["greater", "less"] {
        let m = w(alt);
        let s = fixture(FIXTURE16, &format!("ks2.{alt} statistic"));
        let p = fixture(FIXTURE16, &format!("ks2.{alt} p"));
        close(alt, &[one(&m, "statistic")], &s, 1e-12);
        close(alt, &[one(&m, "p")], &p, 1e-12);
        assert_eq!(text(&m, "alternative"), alt);
        assert_eq!(
            text(&m, "method"),
            "Asymptotic two-sample Kolmogorov-Smirnov test"
        );
    }
    // Swapped, "greater" and "less" exchange.
    let swapped = map(&format!(
        "{B} {A} dict \"alternative\" \"less\" set stat.test.ks2.opts"
    ));
    assert_eq!(one(&swapped, "statistic"), one(&w("greater"), "statistic"));
}

/// Criterion 6: Hodges–Lehmann's estimate and the order-statistic interval.
#[test]
fn rfc0016_hodges_lehmann() {
    let a = "[ 1.1 2.2 3.3 4.4 ]";
    // Exact k = 2.
    let exact = "[ 0.5 1.5 2.0 2.5 3.0 ]";
    let m = hl(a, exact, "");
    close("estimate", &[one(&m, "estimate")], &[0.75], 1e-12);
    close("lower", &[one(&m, "lower")], &[-1.4], 1e-12);
    close("upper", &[one(&m, "upper")], &[2.9], 1e-12);
    assert_eq!(text(&m, "method"), HL_METHOD);
    // Normal k = 3, with a tie, at 90%.
    let m = hl(a, "[ 0.5 1.5 1.5 2.5 3.0 ]", "\"conf\" 0.9 set");
    close("estimate", &[one(&m, "estimate")], &[0.75], 1e-12);
    close("lower", &[one(&m, "lower")], &[-0.8], 1e-12);
    close("upper", &[one(&m, "upper")], &[2.9], 1e-12);
    // The boundary: P(U ≤ 0) = 1/20 exactly admits k = 1 at 90%.
    let m = hl("[ 1.0 2.0 3.0 ]", "[ 1.5 2.5 3.5 ]", "\"conf\" 0.9 set");
    assert_eq!((one(&m, "lower"), one(&m, "upper")), (-2.5, 1.5));
    // One-sided, and mu.
    let greater =
        format!("{a} {exact} dict \"alternative\" \"greater\" set stat.test.mannwhitney.opts");
    let m = map(&greater);
    close("greater lower", &[one(&m, "lower")], &[-0.9], 1e-12);
    assert!(!has(&greater, "upper"));
    close(
        "greater p",
        &[one(&m, "p")],
        &fixture(FIXTURE16, "mw.greater p"),
        1e-10,
    );
    close(
        "W",
        &[one(&m, "statistic")],
        &fixture(FIXTURE16, "mw.greater statistic"),
        0.0,
    );
    let less = format!("{a} {exact} dict \"alternative\" \"less\" set stat.test.mannwhitney.opts");
    let m = map(&less);
    assert!(!has(&less, "lower"));
    // k = 3 as for "greater": D₍₁₈₎.
    close("less upper", &[one(&m, "upper")], &[2.8], 1e-12);
    close(
        "less p",
        &[one(&m, "p")],
        &fixture(FIXTURE16, "mw.less p"),
        1e-10,
    );
    let shifted = hl(a, exact, "\"mu\" 0.5 set");
    close(
        "mu p",
        &[one(&shifted, "p")],
        &fixture(FIXTURE16, "mw.mu p"),
        1e-10,
    );
    close(
        "mu W",
        &[one(&shifted, "statistic")],
        &fixture(FIXTURE16, "mw.mu statistic"),
        0.0,
    );
    let plain = hl(a, exact, "");
    for k in ["estimate", "lower", "upper"] {
        assert_eq!(field(&shifted, k), field(&plain, k), "{k}");
    }

    // The f64 ends as the RFC states them (scratch `rev6v.py`).
    let ends = |a: &str, b: &str, opts: &str| {
        let m = hl(a, b, opts);
        (one(&m, "lower"), one(&m, "upper"))
    };
    // A wide exact case: k = 15, where the floor's 16 is narrower.
    assert_eq!(
        ends("[ 1.0 2.3 ]", &spaced(52), "\"conf\" 0.9 set"),
        (-43.245, -5.505999999999999)
    );
    // A large exact case: C(130, 65) < 2¹²⁸, k = 1692.
    assert_eq!(
        ends(&counting(65), &spaced(65), ""),
        (-6.525999999999996, 7.462000000000003)
    );
    // Past the bound: the floor's k = 1747.
    assert_eq!(
        ends(&counting(66), &spaced(66), ""),
        (-6.529999999999998, 7.465000000000003)
    );
    // The floor's documented shortfall: k = 5749 at 80%.
    assert_eq!(
        ends(&counting(20), &spaced(691), "\"conf\" 0.8 set"),
        (-393.895, -276.79499999999996)
    );
    // Low one-sided levels: past ⌊nm/2⌋, read by symmetry, k = 6.
    let m = hl(
        "[ 1.0 2.0 3.0 ]",
        "[ 1.1 2.3 3.6 ]",
        "\"alternative\" \"greater\" set \"conf\" 0.3 set",
    );
    assert_eq!(one(&m, "lower"), -0.10000000000000009);
}

/// Criterion 2: Mann–Whitney's and the others' refusals.
#[test]
fn rfc0016_m3_refusals_are_their_templates() {
    let w = "stat.test.mannwhitney.opts";
    let too_few = "STAT.TEST.MANNWHITNEY.OPTS: E101 a and b is outside stat.test.mannwhitney.opts's \
         domain: too few for the level";
    // The exact k is 10 > 9.
    assert_eq!(
        err(&format!(
            "[ 1.0 2.0 3.0 ] [ 1.1 2.3 3.6 ] dict \"alternative\" \"greater\" set \
             \"conf\" 1.0e-8 set {w}"
        )),
        too_few
    );
    // A tie, so the floor: k = 10 > 9.
    assert_eq!(
        err(&format!(
            "[ 1.0 2.0 3.0 ] [ 1.0 2.5 3.5 ] dict \"alternative\" \"greater\" set \
             \"conf\" 0.005 set {w}"
        )),
        too_few
    );
    // k < 1: n = m = 2 at 95%.
    assert_eq!(err(&format!("[ 1.0 2.0 ] [ 3.5 4.5 ] dict {w}")), too_few);
    // n·m above 10⁷, before allocating.
    let big = counting(3163);
    assert_eq!(
        err(&format!("{big} {big} dict {w}")),
        "STAT.TEST.MANNWHITNEY.OPTS: E111 10004569 differences of a and b exceeds its cap \
         of 10000000"
    );
    // An infinite end is E114, where the plain word answers.
    let (a, b) = ("[ 1.0e308 1.0 2.0 3.0 ]", "[ -1.0e308 1.5 2.5 3.5 ]");
    assert_eq!(
        err(&format!("{a} {b} dict {w}")),
        "STAT.TEST.MANNWHITNEY.OPTS: E114 the answer overflows a FLOAT: the interval's upper end"
    );
    assert!(run(&format!("{a} {b} stat.test.mannwhitney")).is_ok());
    // A median whose sum overflows is halved first, and answered.
    let m = hl("[ 1.7e308 1.6e308 1.5e308 0.0 ]", "[ 0.0 0.1 0.2 0.3 ]", "");
    assert_eq!(one(&m, "estimate"), 1.55e308);
    // The options each word does not take.
    assert!(
        err("[ [ 3 1 ] [ 1 3 ] ] matrix dict \"mu\" 1.0 set stat.test.fisher.opts")
            .starts_with("STAT.TEST.FISHER.OPTS: E101 opts ")
    );
    assert_eq!(
        err(&format!("{A} {B} dict \"conf\" 0.9 set stat.test.ks2.opts")),
        "STAT.TEST.KS2.OPTS: E101 opts is outside stat.test.ks2.opts's domain: \
         the key \"conf\", which this word does not take"
    );
    assert!(
        err("[ [ 3 1 ] [ 1 3 ] [ 1 1 ] ] matrix dict stat.test.fisher.opts")
            .starts_with("STAT.TEST.FISHER.OPTS: E102 ")
    );
}

#[test]
fn rfc0016_m3_forms_answer_the_plain_words() {
    for src in [
        "[ [ 3 1 ] [ 1 3 ] ] matrix dict \"alternative\" \"less\" set . stat.test.fisher.opts."
            .to_string(),
        format!("{A} {B} dict \"mu\" 0.5 set . stat.test.mannwhitney.opts."),
        format!("{A} {B} dict \"alternative\" \"greater\" set . stat.test.ks2.opts."),
    ] {
        let mut i = run(&src).unwrap_or_else(|e| panic!("{src}: {e}"));
        assert_eq!(i.depth(), 0, "{src}");
        let got = i.pull_workbench().unwrap();
        let plain = map(&src.replace(" . ", " ").replace("opts.", "opts"));
        assert_eq!(got.as_map().unwrap(), &plain, "{src}");
    }
}

#[test]
fn rfc0016_m3_no_operand_aborts() {
    let operands = [
        "float.NaN",
        "0.5",
        "0",
        "\"x\"",
        "list",
        "[ 1 ]",
        "[ 1 1 1 ]",
        "[ 1 2 4 ]",
        "[ 1.0e308 -1.0e308 ]",
        "[ 1 float.NaN ]",
        "dict",
        "dict \"conf\" 0.999999999999 set",
        "dict \"conf\" 1.0e-300 set",
        "dict \"alternative\" \"greater\" set",
        "dict \"mu\" 1.0e308 set",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
        "[ [ 0 0 ] [ 0 5 ] ] matrix",
        "[ [ 1000 0 ] [ 0 1000 ] ] matrix",
    ];
    for w in [
        "stat.test.fisher.opts",
        "stat.test.mannwhitney.opts",
        "stat.test.ks2.opts",
    ] {
        for (i, o) in operands.iter().enumerate() {
            for j in 0..operands.len() {
                let p = operands[j];
                let q = operands[(i + j + 5) % operands.len()];
                for src in [
                    format!("{o} {w}"),
                    format!("{o} {p} {w}"),
                    format!("{o} {p} {q} {w}"),
                    format!("{o} {p} . {w}."),
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
