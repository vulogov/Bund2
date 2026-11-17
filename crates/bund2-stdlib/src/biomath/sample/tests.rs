//! RFC-0016 criteria 1, 2 and 11, for §S7's shuffling and sampling.

use bund2_api::Vm as _;
use bund2_value::BundValue;
use rand::rngs::StdRng;
use rand::{Rng as _, SeedableRng as _};

use super::WORDS;
use crate::biomath::testkit::{err, floats, run};

const TEN: &str = "[ 1 2 3 4 5 6 7 8 9 10 ]";

/// The LIST `src` leaves on top, as f64s.
fn drawn(src: &str) -> Vec<f64> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    floats(&i.pull().unwrap())
}

fn texts(src: &str) -> Vec<String> {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    let v = i.pull().unwrap();
    v.as_list()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap_or_default())
        .collect()
}

#[test]
fn every_word_is_registered_with_its_effect() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for (w, n) in [
        ("rng.shuffle", 1),
        ("rng.choice", 2),
        ("rng.choice.replace", 2),
        ("rng.choice.weighted", 3),
    ] {
        let e = i.registry.effect_of(w).unwrap();
        assert_eq!((e.consumes, e.produces, e.opaque), (n, 1, false), "{w}");
        let e = i.registry.effect_of(&format!("{w}.")).unwrap();
        assert_eq!(
            (e.consumes, e.produces, e.opaque),
            (n - 1, 0, false),
            "{w}."
        );
    }
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
    }
}

/// Criterion 11: the same seed, the same draws; and the first answers of
/// each word after `7 rng.seed`, pinned as this Bund2 version's.
#[test]
fn seeded_draws_repeat_and_are_pinned() {
    let pinned: [(&str, Vec<f64>); 4] = [
        (
            "rng.shuffle",
            vec![3.0, 8.0, 4.0, 10.0, 6.0, 9.0, 7.0, 5.0, 2.0, 1.0],
        ),
        ("4 rng.choice", vec![1.0, 2.0, 5.0, 7.0]),
        ("5 rng.choice.replace", vec![1.0, 4.0, 2.0, 6.0, 10.0]),
        (
            "[ 1 2 3 4 5 6 7 8 9 10 ] 6 rng.choice.weighted",
            vec![2.0, 6.0, 4.0, 8.0, 5.0, 10.0],
        ),
    ];
    for (tail, want) in pinned {
        let src = format!("7 rng.seed {TEN} {tail}");
        let first = drawn(&src);
        assert_eq!(first, want, "{tail}");
        // Twice in one program: the second seed repeats the first draw.
        let mut i = run(&format!("{src} 7 rng.seed {TEN} {tail}")).unwrap();
        assert_eq!(floats(&i.pull().unwrap()), first, "{tail}");
    }
    assert_eq!(
        texts("7 rng.seed [ \"a\" \"b\" \"c\" \"d\" ] [ 1 2 3 4 ] 6 rng.choice.weighted"),
        ["a", "c", "b", "c", "b", "d"]
    );
}

/// The shuffle is S7's Fisher–Yates over `StdRng`: recomputed here from
/// the seed `rng.seed` gives, it is the pinned answer.
#[test]
fn the_shuffle_is_s7s_algorithm() {
    let mut g = StdRng::seed_from_u64(7);
    let mut xs: Vec<u32> = (1..=10).collect();
    for i in (1..xs.len()).rev() {
        let j = g.gen_range(0..=i as u64) as usize;
        xs.swap(i, j);
    }
    let want: Vec<f64> = xs.iter().map(|x| f64::from(*x)).collect();
    assert_eq!(drawn(&format!("7 rng.seed {TEN} rng.shuffle")), want);
}

/// So is the weighted draw: u = Σw·`gen::<f64>()`, the first cumulative
/// weight strictly above u.
#[test]
fn the_weighted_draw_is_s7s_algorithm() {
    let mut g = StdRng::seed_from_u64(7);
    let cum: Vec<f64> = (1..=10)
        .scan(0.0, |s, w| {
            *s += f64::from(w);
            Some(*s)
        })
        .collect();
    let want: Vec<f64> = (0..6)
        .map(|_| {
            let u = 55.0 * g.r#gen::<f64>();
            (cum.partition_point(|c| *c <= u) + 1) as f64
        })
        .collect();
    assert_eq!(
        drawn(&format!("7 rng.seed {TEN} {TEN} 6 rng.choice.weighted")),
        want
    );
}

#[test]
fn shuffle_and_choice_answer_their_lists() {
    let mut s = drawn(&format!("11 rng.seed {TEN} rng.shuffle"));
    s.sort_by(f64::total_cmp);
    assert_eq!(s, (1..=10).map(f64::from).collect::<Vec<_>>());
    for k in 0..=10 {
        let mut c = drawn(&format!("11 rng.seed {TEN} {k} rng.choice"));
        assert_eq!(c.len(), k);
        c.sort_by(f64::total_cmp);
        c.dedup();
        assert_eq!(c.len(), k, "no index twice");
    }
    // Any values move unconverted, NONE included.
    let mut i = run("3 rng.seed [ \"x\" 1 nodata ] 3 rng.choice").unwrap();
    let v = i.pull().unwrap();
    assert_eq!(v.as_list().map(<[BundValue]>::len), Some(3));
    // k = 0 answers an empty LIST for every word.
    for src in [
        "list 0 rng.choice",
        "list 0 rng.choice.replace",
        "list list 0 rng.choice.weighted",
        "list rng.shuffle",
    ] {
        assert!(drawn(src).is_empty(), "{src}");
    }
    // The forms take k, or the list for `rng.shuffle.`, from the workbench.
    let mut i = run(&format!("7 rng.seed {TEN} 4 . rng.choice.")).unwrap();
    assert_eq!(floats(&i.pull_workbench().unwrap()), [1.0, 2.0, 5.0, 7.0]);
    let mut i = run(&format!("7 rng.seed {TEN} . rng.shuffle.")).unwrap();
    assert_eq!(i.depth(), 0);
    assert_eq!(
        floats(&i.pull_workbench().unwrap()),
        [3.0, 8.0, 4.0, 10.0, 6.0, 9.0, 7.0, 5.0, 2.0, 1.0]
    );
}

/// Over 10⁵ seeded draws with weights [1 2 3 4], each category within 4
/// SE of its share; with [0 1 0 1], no zero-weight element drawn.
#[test]
fn weighted_draws_follow_their_weights() {
    let n = 100_000;
    let xs = drawn(&format!(
        "5 rng.seed [ 0 1 2 3 ] [ 1 2 3 4 ] {n} rng.choice.weighted"
    ));
    for (c, w) in [1.0, 2.0, 3.0, 4.0].iter().enumerate() {
        let p = w / 10.0;
        let got = xs.iter().filter(|x| **x == c as f64).count() as f64 / n as f64;
        let se = (p * (1.0 - p) / n as f64).sqrt();
        assert!((got - p).abs() <= 4.0 * se, "{c}: {got} against {p}");
    }
    let xs = drawn("5 rng.seed [ 0 1 2 3 ] [ 0 1 0 1 ] 10000 rng.choice.weighted");
    assert!(xs.iter().all(|x| *x == 1.0 || *x == 3.0));
}

#[test]
fn each_refusal_is_its_template() {
    assert_eq!(
        err("list 1 rng.choice.replace"),
        "RNG.CHOICE.REPLACE: E101 list is outside rng.choice.replace's domain: an empty list"
    );
    assert_eq!(
        err("list list 1 rng.choice.weighted"),
        "RNG.CHOICE.WEIGHTED: E101 weights is outside rng.choice.weighted's domain: a sum of 0"
    );
    assert_eq!(
        err(&format!("{TEN} 11 rng.choice")),
        "RNG.CHOICE: E101 k is outside rng.choice's domain: 11, above the length 10"
    );
    assert_eq!(
        err("[ 1 2 ] [ 1.7e308 1.7e308 ] 1 rng.choice.weighted"),
        "RNG.CHOICE.WEIGHTED: E114 the answer overflows a FLOAT: the sum of the weights"
    );
    assert_eq!(
        err("[ 1 2 ] 100000001 rng.choice.replace"),
        "RNG.CHOICE.REPLACE: E111 a draw of 100000001 elements exceeds its cap of 100000000"
    );
    assert!(err("[ 1 2 ] -1 rng.choice").starts_with("RNG.CHOICE: E101 k "));
    assert!(err("[ 1 2 ] 1.0 rng.choice").starts_with("RNG.CHOICE: E101 k "));
    assert!(err("\"ab\" rng.shuffle").starts_with("RNG.SHUFFLE: E101 list "));
    assert!(err("[ 1 2 ] [ 1 ] 1 rng.choice.weighted").starts_with("RNG.CHOICE.WEIGHTED: E102 "));
    assert!(
        err("[ 1 2 ] [ 1 \"x\" ] 1 rng.choice.weighted").starts_with("RNG.CHOICE.WEIGHTED: E103 ")
    );
    assert!(
        err("[ 1 2 ] [ 1 -1 ] 1 rng.choice.weighted")
            .starts_with("RNG.CHOICE.WEIGHTED: E101 weights ")
    );
    assert!(
        err("[ 1 2 ] 1 list + float.+Inf swap + 1 rng.choice.weighted")
            .starts_with("RNG.CHOICE.WEIGHTED: E101 weights ")
    );
    assert!(
        err("[ 1 2 ] \"w\" 1 rng.choice.weighted")
            .starts_with("RNG.CHOICE.WEIGHTED: E101 weights ")
    );
}

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "0",
        "1",
        "3",
        "-1",
        "\"x\"",
        "list",
        "[ 1 ]",
        "[ 1 2 3 ]",
        "[ 0 0 0 ]",
        "[ 1 float.NaN ]",
        "dict",
        "nodata",
    ];
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        for o in operands {
            for p in operands {
                for src in [
                    format!("{o} {w}"),
                    format!("{o} {p} {w}"),
                    format!("{o} {p} {o} {w}"),
                    format!("{p} {o} . {w}."),
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
