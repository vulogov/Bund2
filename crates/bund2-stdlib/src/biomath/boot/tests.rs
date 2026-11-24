//! RFC-0019 criteria 9 and 10, and 1 and 14, for §S7 and §S8.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;
use rand::RngCore as _;

use super::{Kind, WORDS, summarise};
use crate::biomath::rng::{reseed, with_rng};
use crate::biomath::testkit::{err, fixture, floats, run};

const BOOT: &str = include_str!("../fixtures/boot.txt");

/// The fixture's interval cases and their levels.
const CASES: [(&str, &[&str]); 2] = [("a", &["0.95", "0.9", "0.8"]), ("b", &["0.95", "0.9"])];

const KINDS: [Kind; 4] = [Kind::Normal, Kind::Basic, Kind::Percentile, Kind::Bca];

fn answer(src: &str) -> BTreeMap<String, BundValue> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap_or_else(|| panic!("{src}: empty"));
    assert_eq!(i.depth(), 0, "{src}");
    v.as_map().unwrap_or_else(|| panic!("{src}: {v:?}")).clone()
}

fn key(m: &BTreeMap<String, BundValue>, k: &str) -> Vec<f64> {
    floats(m.get(k).unwrap_or_else(|| panic!("no {k} in {m:?}")))
}

fn bits(xs: &[f64]) -> Vec<u64> {
    xs.iter().map(|x| x.to_bits()).collect()
}

/// Criterion 10's tolerance: 10⁻⁹ relative and 10⁻¹² absolute.
fn near(what: &str, got: f64, want: f64) {
    assert!(
        (got - want).abs() <= 1e-9 * want.abs() + 1e-12,
        "{what}: {got} against {want}"
    );
}

/// Two answers alike: the same keys, and each the same numbers to the bit
/// (a LIST carries a heap identity, which `==` would compare).
fn same(a: &BTreeMap<String, BundValue>, b: &BTreeMap<String, BundValue>, what: &str) {
    assert_eq!(
        a.keys().collect::<Vec<_>>(),
        b.keys().collect::<Vec<_>>(),
        "{what}"
    );
    for k in a.keys() {
        assert_eq!(bits(&key(a, k)), bits(&key(b, k)), "{what} {k}");
    }
}

fn bund_list(xs: &[f64]) -> String {
    let items: Vec<String> = xs.iter().map(|x| format!("{x:?}")).collect();
    format!("[ {} ]", items.join(" "))
}

// Criterion 1.

#[test]
fn every_word_is_registered_without_a_dot_form() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for (w, n) in WORDS.iter().zip([4, 2]) {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        assert!(e.opaque, "{w}");
        assert_eq!(e.consumes, n, "{w}");
        assert!(i.registry.effect_of(&format!("{w}.")).is_none(), "{w}.");
    }
    let ours: Vec<String> = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("boot."))
        .collect();
    assert_eq!(ours.len(), WORDS.len(), "{ours:?}");
    // §S7: a producer, plain only.
    let e = i
        .registry
        .effect_of("rng.derive")
        .unwrap_or_else(|| panic!("rng.derive"));
    assert!(!e.opaque);
    assert_eq!((e.consumes, e.produces), (0, 1));
    assert!(i.registry.effect_of("rng.derive.").is_none());
}

// Criterion 9.

fn derived(src: &str) -> Vec<i64> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let mut out = Vec::new();
    while let Some(v) = i.pull() {
        out.push(v.as_int().unwrap_or_else(|| panic!("{v:?}")));
    }
    out.reverse();
    out
}

#[test]
fn rng_derive_answers_the_same_seeds_after_the_same_seed() {
    let src = "42 rng.seed rng.derive rng.derive rng.derive";
    let first = derived(src);
    assert_eq!(first.len(), 3);
    assert_eq!(derived(src), first);
    // Each is the generator's next u64, its bits as an INTEGER.
    let mut vm = bund2_interp::Interp::new();
    reseed(&mut vm, 42);
    let want: Vec<i64> = (0..3)
        .map(|_| i64::from_ne_bytes(with_rng(&mut vm, |g| g.next_u64()).to_ne_bytes()))
        .collect();
    assert_eq!(first, want);
    // They differ, and another seed derives others.
    assert!(first[0] != first[1] && first[1] != first[2]);
    assert_ne!(
        derived("43 rng.seed rng.derive rng.derive rng.derive"),
        first
    );
    // A VM seeded with each repeats its own stream, and the streams differ.
    let mut streams = Vec::new();
    for d in &first {
        let src = format!("{d} rng.seed rng.uniform rng.uniform rng.uniform");
        let a = derived_floats(&src);
        assert_eq!(bits(&a), bits(&derived_floats(&src)), "{d}");
        streams.push(bits(&a));
    }
    assert!(streams[0] != streams[1] && streams[1] != streams[2]);
}

fn derived_floats(src: &str) -> Vec<f64> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let mut out = Vec::new();
    while let Some(v) = i.pull() {
        out.extend(floats(&v));
    }
    out
}

/// Each `rng.derive` advances the parent's stream by one u64.
#[test]
fn rng_derive_advances_the_stream_by_one_draw() {
    let mut vm = bund2_interp::Interp::new();
    reseed(&mut vm, 5);
    let _ = with_rng(&mut vm, |g| g.next_u64());
    let want = with_rng(&mut vm, |g| g.next_u64());
    let got = derived("5 rng.seed rng.derive drop rng.derive");
    assert_eq!(got, vec![i64::from_ne_bytes(want.to_ne_bytes())]);
}

// Criterion 10: the interval function against the fixture.

#[test]
fn the_interval_function_matches_the_fixture() {
    for (case, confs) in CASES {
        let t0 = fixture(BOOT, &format!("{case} t0"))[0];
        let t = fixture(BOOT, &format!("{case} t"));
        let jack = fixture(BOOT, &format!("{case} jack"));
        for conf_s in confs {
            let conf: f64 = conf_s.parse().unwrap_or_default();
            let s = summarise("boot.sample", t0, &t, Some(&jack), conf, &KINDS)
                .unwrap_or_else(|e| panic!("{case}@{conf_s}: {}", e.0));
            near(case, s.bias, fixture(BOOT, &format!("{case} bias"))[0]);
            near(case, s.se, fixture(BOOT, &format!("{case} se"))[0]);
            assert_eq!(s.intervals.len(), 4);
            for (kind, ends) in &s.intervals {
                for (end, which) in ends.iter().zip(["lower", "upper"]) {
                    let k = format!("{case}@{conf_s} {}.{which}", kind.name());
                    near(&k, *end, fixture(BOOT, &k)[0]);
                }
            }
        }
    }
}

/// `boot.sample`'s bias and se, on the fixture's replicates, are the
/// fixture's: the words compute them through the same function.
#[test]
fn bias_and_se_are_the_fixtures() {
    for (case, _) in CASES {
        let t0 = fixture(BOOT, &format!("{case} t0"))[0];
        let t = fixture(BOOT, &format!("{case} t"));
        let s = summarise("boot.sample", t0, &t, None, 0.95, &[Kind::Percentile])
            .unwrap_or_else(|e| panic!("{case}: {}", e.0));
        near(case, s.bias, fixture(BOOT, &format!("{case} bias"))[0]);
        near(case, s.se, fixture(BOOT, &format!("{case} se"))[0]);
    }
}

#[test]
fn boot_jackknife_matches_the_fixture() {
    for case in ["m", "d", "s", "v"] {
        let data = fixture(BOOT, &format!("{case} data"));
        let line = BOOT
            .lines()
            .find(|l| l.starts_with(&format!("{case} statistic ")))
            .unwrap_or_default();
        let stat = line.rsplit(' ').next().unwrap_or_default();
        let m = answer(&format!("{} {stat:?} boot.jackknife", bund_list(&data)));
        near(
            case,
            key(&m, "t0")[0],
            fixture(BOOT, &format!("{case} t0"))[0],
        );
        let values = key(&m, "values");
        assert_eq!(values.len(), data.len());
        for (i, v) in values.iter().enumerate() {
            near(case, *v, fixture(BOOT, &format!("{case} values:{i}"))[0]);
        }
        near(
            case,
            key(&m, "bias")[0],
            fixture(BOOT, &format!("{case} bias"))[0],
        );
        near(
            case,
            key(&m, "se")[0],
            fixture(BOOT, &format!("{case} se"))[0],
        );
        // The lambda calling the word answers the same, to the bit.
        let word = match stat {
            "mean" => "stat.mean",
            "median" => "stat.median",
            "sd" => "stat.sd",
            _ => "stat.var",
        };
        let l = answer(&format!("{} {{ {word} }} boot.jackknife", bund_list(&data)));
        for k in ["t0", "values", "bias", "se"] {
            assert_eq!(bits(&key(&l, k)), bits(&key(&m, k)), "{case} {k}");
        }
    }
}

// Criterion 10: the words.

const DATA: &str = "[ 2.1 3.4 1.9 4.2 2.8 3.1 2.5 3.9 2.2 3.6 ]";

#[test]
fn boot_sample_repeats_after_a_seed() {
    let src = format!("9 rng.seed {DATA} \"mean\" 200 dict boot.sample");
    let a = answer(&src);
    let b = answer(&src);
    same(&a, &b, &src);
    assert_eq!(key(&a, "t").len(), 200);
    assert_ne!(
        bits(&key(&a, "t")),
        bits(&key(
            &answer(&format!("10 rng.seed {DATA} \"mean\" 200 dict boot.sample")),
            "t"
        ))
    );
    let keys: Vec<&str> = a.keys().map(String::as_str).collect();
    assert_eq!(keys, ["bias", "percentile", "se", "t", "t0"]);
}

/// The replicates are a Bund program's own loop of `rng.choice.replace`
/// and the statistic, after the same seed, to the bit; a name's are its
/// word's in a lambda.
#[test]
fn the_replicates_are_a_loop_of_choice_replace() {
    for (name, word) in [
        ("mean", "stat.mean"),
        ("median", "stat.median"),
        ("sd", "stat.sd"),
        ("var", "stat.var"),
    ] {
        let mut i = run("31 rng.seed").unwrap_or_else(|e| panic!("{}", e));
        let step = bund2_syntax::compile(&format!("{DATA} 10 rng.choice.replace {word}"))
            .unwrap_or_else(|e| panic!("{e:?}"));
        let mut want = Vec::new();
        for _ in 0..25 {
            i.eval(&step).unwrap_or_else(|e| panic!("{e:?}"));
            want.extend(floats(&i.pull().unwrap_or_else(|| panic!("{word}"))));
        }
        let named = answer(&format!("31 rng.seed {DATA} {name:?} 25 dict boot.sample"));
        assert_eq!(bits(&key(&named, "t")), bits(&want), "{name}");
        let lambda = answer(&format!(
            "31 rng.seed {DATA} {{ {word} }} 25 dict boot.sample"
        ));
        same(&lambda, &named, name);
    }
}

/// A frame is resampled by row with the same draws a LIST of its rows takes.
#[test]
fn a_frame_is_resampled_by_row() {
    let frame = format!("[ \"x\" \"y\" ] [ {DATA} [ 1 2 3 4 5 6 7 8 9 10 ] ] frame.new");
    let f = answer(&format!(
        "4 rng.seed {frame} {{ \"x\" frame.column stat.mean }} 30 dict boot.sample"
    ));
    let l = answer(&format!("4 rng.seed {DATA} \"mean\" 30 dict boot.sample"));
    assert_eq!(bits(&key(&f, "t")), bits(&key(&l, "t")));
    assert_eq!(bits(&key(&f, "t0")), bits(&key(&l, "t0")));
    // A name with a frame is E101 naming statistic.
    assert_eq!(
        err(&format!("{frame} \"mean\" 30 dict boot.sample")),
        "BOOT.SAMPLE: E101 statistic is outside boot.sample's domain: \"mean\" with a frame as data"
    );
}

/// The answer's intervals are the function's on its own `t`, `t0` and
/// `jack`, to the bit, and its `jack` is `boot.jackknife`'s `values`.
#[test]
fn the_intervals_are_the_function_on_the_answer() {
    let opts = "dict \"conf\" 0.9 set \"type\" [ \"normal\" \"basic\" \"percentile\" \"bca\" ] set";
    let m = answer(&format!(
        "17 rng.seed {DATA} \"median\" 300 {opts} boot.sample"
    ));
    let t0 = key(&m, "t0")[0];
    let t = key(&m, "t");
    let jack = key(&m, "jack");
    let s = summarise("boot.sample", t0, &t, Some(&jack), 0.9, &KINDS)
        .unwrap_or_else(|e| panic!("{}", e.0));
    assert_eq!(s.bias.to_bits(), key(&m, "bias")[0].to_bits());
    assert_eq!(s.se.to_bits(), key(&m, "se")[0].to_bits());
    for (kind, ends) in &s.intervals {
        assert_eq!(bits(&key(&m, kind.name())), bits(ends), "{}", kind.name());
    }
    let j = answer(&format!("{DATA} \"median\" boot.jackknife"));
    assert_eq!(bits(&key(&j, "values")), bits(&jack));
    // Without "bca" there is no jackknife.
    let p = answer(&format!(
        "17 rng.seed {DATA} \"median\" 300 dict boot.sample"
    ));
    assert!(!p.contains_key("jack"));
}

// Refusals.

#[test]
fn each_refusal_is_its_template() {
    let e = |src: &str, want: &str| assert_eq!(err(src), want, "{src}");
    e(
        "list \"mean\" 10 dict boot.sample",
        "BOOT.SAMPLE: E101 data is outside boot.sample's domain: an empty LIST",
    );
    e(
        "1.5 \"mean\" 10 dict boot.sample",
        "BOOT.SAMPLE: E101 data is outside boot.sample's domain: 1.5",
    );
    e(
        &format!("{DATA} \"mode\" 10 dict boot.sample"),
        "BOOT.SAMPLE: E101 statistic is outside boot.sample's domain: \"mode\"",
    );
    e(
        &format!("{DATA} \"mean\" 1 dict boot.sample"),
        "BOOT.SAMPLE: E101 R is outside boot.sample's domain: 1",
    );
    e(
        &format!("{DATA} \"mean\" 10 dict \"conf\" 1.0 set boot.sample"),
        "BOOT.SAMPLE: E101 conf is outside boot.sample's domain: 1.0",
    );
    e(
        &format!("{DATA} \"mean\" 10 dict \"type\" [ \"stud\" ] set boot.sample"),
        "BOOT.SAMPLE: E101 type is outside boot.sample's domain: list/1 [\"stud\"]",
    );
    e(
        &format!("{DATA} \"mean\" 10 dict \"alternative\" \"less\" set boot.sample"),
        "BOOT.SAMPLE: E101 opts is outside boot.sample's domain: an unknown key \"alternative\"",
    );
    e(
        "[ 1.0 ] \"sd\" 10 dict boot.sample",
        "BOOT.SAMPLE: E101 data is outside boot.sample's domain: 1 values, where \"sd\" takes at least 2",
    );
    e(
        "[ 1.0 ] \"mean\" boot.jackknife",
        "BOOT.JACKKNIFE: E101 data is outside boot.jackknife's domain: 1 value, where the jackknife takes at least 2",
    );
    e(
        &format!("{DATA} \"mean\" 10000001 dict boot.sample"),
        "BOOT.SAMPLE: E111 10000001 replicates of 10 draws exceeds its cap of 100000000",
    );
    e(
        &format!("{DATA} {{ drop \"s\" }} 10 dict boot.sample"),
        "BOOT.SAMPLE: E112 the model lambda left \"s\", where one finite number was required",
    );
    // BCa: every replicate on one side of t0, and a constant jackknife.
    e(
        "[ 1.0 1.0 1.0 ] \"mean\" 10 dict \"type\" [ \"bca\" ] set boot.sample",
        "BOOT.SAMPLE: E101 statistic is outside boot.sample's domain: every replicate at or above t0, so BCa's w is infinite",
    );
    e(
        // The maximum of 1, 5, 5 leaving one out is always 5; a resample's
        // is 1 when it draws no 5.
        "3 rng.seed [ 1.0 5.0 5.0 ] { math.max } 200 dict \"type\" [ \"bca\" ] set boot.sample",
        "BOOT.SAMPLE: E101 statistic is outside boot.sample's domain: a constant jackknife, so BCa's a is not finite",
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
        "\"mean\"",
        "\"sd\"",
        "list",
        "dict",
        "nodata",
        "[ 1.0 ]",
        "[ 1.0 float.NaN push ]",
        "[ 1 \"a\" ]",
        "[ \"x\" ] [ [ 1.0 2.0 ] ] frame.new",
        "{ 1 }",
        "{ drop }",
        "{ drop float.NaN }",
        "{ drop \"s\" }",
        "{ no.such.word }",
        "{ stat.mean }",
    ];
    for w in WORDS {
        for o in operands {
            for k in 1..=4 {
                let src = format!("{} {w}", vec![o; k].join(" "));
                let _ = run(&src);
            }
            let _ = run(&format!("{o} \"mean\" 5 dict {w}"));
            let _ = run(&format!("{DATA} {o} 5 dict {w}"));
            let _ = run(&format!("{DATA} \"mean\" {o} dict {w}"));
            let _ = run(&format!("{DATA} \"mean\" 5 {o} {w}"));
            let _ = run(&format!("{DATA} {o} {w}"));
        }
        let _ = run(&format!(
            "{DATA} {{ drop }} 5 dict \"type\" [ \"bca\" \"basic\" \"normal\" ] set {w}"
        ));
    }
}

#[test]
fn a_program_defining_the_name_calls_its_own() {
    let mut i =
        run(":boot.sample { 42 } register 1 boot.sample").unwrap_or_else(|e| panic!("{}", e));
    assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
}
