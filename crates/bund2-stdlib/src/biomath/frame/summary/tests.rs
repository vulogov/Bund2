//! RFC-0017 criteria 1, 5 and 9 for M4's words.

use bund2_api::Vm as _;
use bund2_value::{BundValue, NONE};

use super::WORDS;
use crate::biomath::frame::render;
use crate::biomath::frame::tests::{
    abort_free, fails, five, list, map, ok_error, on, palette, raw, s, strs,
};
use crate::biomath::na::none;
use crate::biomath::testkit::{frame_eq, run_on};

fn int(i: i64) -> BundValue {
    BundValue::int(i)
}

fn float(x: f64) -> BundValue {
    BundValue::float(x)
}

fn floats(xs: &[f64]) -> BundValue {
    list(xs.iter().copied().map(float).collect())
}

fn ints(xs: &[i64]) -> BundValue {
    list(xs.iter().copied().map(int).collect())
}

fn same(got: &BundValue, want: &BundValue) {
    assert!(
        frame_eq(got, want),
        "got  {}\nwant {}",
        got.display(),
        want.display()
    );
}

fn stat(c: &str, st: &str) -> BundValue {
    strs(&[c, st])
}

fn omit() -> BundValue {
    map(vec![("na", s("omit"))])
}

/// `frame.aggregate` of `f`, checked by `frame.check`.
fn aggregate(f: &BundValue, by: &[&str], stats: Vec<BundValue>, opts: BundValue) -> BundValue {
    let a = on(&[f.clone(), strs(by), list(stats), opts], "frame.aggregate")
        .unwrap_or_else(|e| panic!("{e}"));
    on(std::slice::from_ref(&a), "frame.check").unwrap_or_else(|e| panic!("{e}"));
    a
}

fn agg_fails(f: &BundValue, by: &[&str], stats: Vec<BundValue>, opts: BundValue) -> String {
    fails(&[f.clone(), strs(by), list(stats), opts], "frame.aggregate")
}

fn agg_e101(op: &str, t: &str) -> String {
    format!("FRAME.AGGREGATE: E101 {op} is outside frame.aggregate's domain: {t}")
}

/// A frame answer's column.
fn column(f: &BundValue, name: &str) -> Vec<BundValue> {
    on(&[f.clone(), s(name)], "frame.column")
        .unwrap_or_else(|e| panic!("{e}"))
        .as_list()
        .unwrap()
        .to_vec()
}

fn bits(v: &BundValue) -> u64 {
    match v.unboxed() {
        BundValue::Float(x, _) => x.to_bits(),
        _ => panic!("not a FLOAT: {}", v.display()),
    }
}

/// The named word on `cells`, as a FLOAT's bits.
fn named(word: &str, cells: &[BundValue]) -> u64 {
    bits(&on(&[list(cells.to_vec())], word).unwrap_or_else(|e| panic!("{word}: {e}")))
}

#[test]
fn every_word_is_registered_with_its_effect() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        let consumes = if w.starts_with("frame.matrix") { 3 } else { 4 };
        let want = if w.ends_with('.') {
            (consumes - 1, 0)
        } else {
            (consumes, 1)
        };
        assert_eq!(
            (e.consumes, e.produces, e.opaque),
            (want.0, want.1, false),
            "{w}"
        );
    }
}

/// Criterion 1: a `.` form takes its top operand from the workbench.
#[test]
fn forms_use_the_workbench() {
    let mut i = run_on(
        &[five(), strs(&["group"]), list(vec![stat("conc", "sum")])],
        Some(&map(vec![])),
        "frame.aggregate.",
    )
    .unwrap();
    let a = i.pull_workbench().unwrap();
    assert_eq!(column(&a, "conc.sum").len(), 2);
    let mut i = run_on(
        &[five(), strs(&["conc"])],
        Some(&map(vec![])),
        "frame.matrix.",
    )
    .unwrap();
    assert_eq!(i.pull_workbench().unwrap().as_matrix().unwrap().len(), 5);
}

/// The frame for the statistics: a factor `g` (levels b, a), a string `s`,
/// a numeric `x` mixing INTEGER and FLOAT, and a NONE in `g`.
fn groups_frame() -> BundValue {
    raw(
        &["g", "s", "x"],
        vec![
            list(vec![s("a"), s("b"), s("a"), s("a"), s("b"), none(), s("b")]),
            strs(&["p", "p", "q", "p", "p", "p", "q"]),
            list(vec![
                int(1),
                float(2.5),
                int(4),
                float(7.25),
                int(-3),
                int(10),
                float(0.5),
            ]),
        ],
        vec![("g", strs(&["b", "a"]))],
    )
}

/// Criterion 5: every statistic equals its named word's on the group's
/// cells, bit for bit; groups in `frame.sort`'s order, NONE last.
#[test]
fn aggregate_every_statistic() {
    let f = groups_frame();
    let names = ["count", "sum", "mean", "median", "min", "max"];
    let a = aggregate(
        &f,
        &["g", "s"],
        names.iter().map(|n| stat("x", n)).collect(),
        map(vec![]),
    );
    // Groups: (b,p) rows 2,5; (b,q) row 7; (a,p) rows 1,4; (a,q) row 3;
    // (NONE,p) row 6. The factor sorts by its levels, b first.
    let groups: [(&str, &str, Vec<BundValue>); 5] = [
        ("b", "p", vec![float(2.5), int(-3)]),
        ("b", "q", vec![float(0.5)]),
        ("a", "p", vec![int(1), float(7.25)]),
        ("a", "q", vec![int(4)]),
        ("", "p", vec![int(10)]),
    ];
    let g = column(&a, "g");
    let sc = column(&a, "s");
    assert_eq!(g.len(), 5);
    for (k, (gk, sk, cells)) in groups.iter().enumerate() {
        if gk.is_empty() {
            assert_eq!(g[k].dt(), NONE);
        } else {
            assert_eq!(g[k].as_str().unwrap(), *gk);
        }
        assert_eq!(sc[k].as_str().unwrap(), *sk);
        assert_eq!(column(&a, "x.count")[k].display(), cells.len().to_string());
        assert_eq!(bits(&column(&a, "x.sum")[k]), named("vec.sum", cells));
        assert_eq!(
            bits(&column(&a, "x.mean")[k]),
            named("stat.mean.arithmetic", cells)
        );
        assert_eq!(
            bits(&column(&a, "x.median")[k]),
            named("stat.median", cells)
        );
        let fl: Vec<f64> = cells
            .iter()
            .map(|c| match c.unboxed() {
                BundValue::Int(i, _) => *i as f64,
                BundValue::Float(x, _) => *x,
                _ => unreachable!(),
            })
            .collect();
        let min = fl.iter().copied().fold(f64::INFINITY, f64::min);
        let max = fl.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(bits(&column(&a, "x.min")[k]), min.to_bits());
        assert_eq!(bits(&column(&a, "x.max")[k]), max.to_bits());
    }
    // The factor keeps its levels; the columns are by, then results.
    let want_names = [
        "g", "s", "x.count", "x.sum", "x.mean", "x.median", "x.min", "x.max",
    ];
    assert_eq!(
        on(std::slice::from_ref(&a), "frame.names")
            .unwrap()
            .display(),
        strs(&want_names).display()
    );
    let lv = on(&[a.clone(), s("levels")], "get").unwrap();
    assert_eq!(
        on(&[lv, s("g")], "get").unwrap().display(),
        strs(&["b", "a"]).display()
    );

    // `"var"` and `"sd"` over groups of two or more.
    let a = aggregate(
        &f,
        &["s"],
        vec![stat("x", "var"), stat("x", "sd")],
        map(vec![]),
    );
    let p = vec![int(1), float(2.5), float(7.25), int(-3), int(10)];
    let q = vec![int(4), float(0.5)];
    for (k, cells) in [p, q].iter().enumerate() {
        assert_eq!(bits(&column(&a, "x.var")[k]), named("stat.var", cells));
        assert_eq!(bits(&column(&a, "x.sd")[k]), named("stat.sd", cells));
    }
}

/// Criterion 5: `"omit"` counts `.dropped`; `"refuse"` names the NONE.
#[test]
fn aggregate_missing() {
    let f = raw(
        &["s", "x"],
        vec![
            strs(&["p", "p", "q", "q", "q"]),
            list(vec![int(1), none(), int(3), none(), int(5)]),
        ],
        vec![],
    );
    let a = aggregate(
        &f,
        &["s"],
        vec![stat("x", "mean"), stat("x", "count")],
        omit(),
    );
    same(
        &a,
        &raw(
            &["s", "x.mean", "x.count", ".dropped"],
            vec![
                strs(&["p", "q"]),
                floats(&[1.0, 4.0]),
                ints(&[1, 2]),
                ints(&[1, 1]),
            ],
            vec![],
        ),
    );
    assert_eq!(
        agg_fails(&f, &["s"], vec![stat("x", "mean")], map(vec![])),
        agg_e101("frame", "column \"x\" row 2 is none (na \"refuse\")")
    );
    // A NONE in a `"count"` column is neither refused nor dropped for.
    let a = aggregate(&f, &["s"], vec![stat("x", "count")], map(vec![]));
    assert_eq!(column(&a, "x.count")[0].display(), "2");
    let a = aggregate(&f, &["s"], vec![stat("x", "count")], omit());
    assert_eq!(column(&a, ".dropped")[0].display(), "0");
    // An emptied group is E101 with the count.
    let g = raw(
        &["s", "x"],
        vec![strs(&["p", "q"]), list(vec![int(1), none()])],
        vec![],
    );
    assert_eq!(
        agg_fails(&g, &["s"], vec![stat("x", "mean")], omit()),
        agg_e101("stats", "group [ \"q\" ] has 0 values for \"mean\"")
    );
    // A `by` column named `.dropped` under `"omit"`.
    let d = raw(&[".dropped", "x"], vec![strs(&["p"]), ints(&[1])], vec![]);
    assert_eq!(
        agg_fails(&d, &[".dropped"], vec![stat("x", "sum")], omit()),
        agg_e101("stats", "result name \".dropped\" repeats a by column")
    );
    aggregate(&d, &[".dropped"], vec![stat("x", "sum")], map(vec![]));
}

/// Criterion 5: E114 and the small-group E101, with S8's texts, in S5's
/// order.
#[test]
fn aggregate_group_refusals() {
    let big = raw(
        &["s", "x"],
        vec![strs(&["a", "a"]), floats(&[1e308, 1e308])],
        vec![],
    );
    for st in ["mean", "sum", "var"] {
        assert_eq!(
            agg_fails(&big, &["s"], vec![stat("x", st)], map(vec![])),
            format!(
                "FRAME.AGGREGATE: E114 the answer overflows a FLOAT: \"x.{st}\" of group [ \"a\" ]"
            )
        );
    }
    let one = raw(&["s", "x"], vec![strs(&["a"]), ints(&[1])], vec![]);
    assert_eq!(
        agg_fails(&one, &["s"], vec![stat("x", "var")], map(vec![])),
        agg_e101("stats", "group [ \"a\" ] has 1 value for \"var\"")
    );
    // The first failing group wins, by the answer's order.
    let ab = |a: Vec<f64>, b: Vec<f64>| {
        let n = (a.len(), b.len());
        let mut keys = vec!["a"; n.0];
        keys.extend(vec!["b"; n.1]);
        let xs: Vec<f64> = a.into_iter().chain(b).collect();
        raw(&["s", "x"], vec![strs(&keys), floats(&xs)], vec![])
    };
    let stats = || vec![stat("x", "var"), stat("x", "mean")];
    assert!(
        agg_fails(
            &ab(vec![1.0], vec![1e308, 1e308]),
            &["s"],
            stats(),
            map(vec![])
        )
        .contains(": E101 ")
    );
    assert!(
        agg_fails(
            &ab(vec![1e308, 1e308], vec![1.0]),
            &["s"],
            stats(),
            map(vec![])
        )
        .contains(": E114 ")
    );
    // A long STRING key with a quote and a line break: escaped, then
    // bounded, on one line; a NONE key shows as `none`.
    let long = format!("{}\"\n{}", "k".repeat(100), "k".repeat(98));
    let lf = raw(&["s", "x"], vec![strs(&[&long]), ints(&[1])], vec![]);
    let e = agg_fails(&lf, &["s"], vec![stat("x", "var")], map(vec![]));
    assert_eq!(
        e,
        agg_e101(
            "stats",
            &format!("group [ {} ] has 1 value for \"var\"", render::text(&long))
        )
    );
    assert!(!e.contains('\n') && e.len() < 200, "{e}");
    let nf = raw(&["s", "x"], vec![list(vec![none()]), ints(&[1])], vec![]);
    assert_eq!(
        agg_fails(&nf, &["s"], vec![stat("x", "var")], map(vec![])),
        agg_e101("stats", "group [ none ] has 1 value for \"var\"")
    );
    let e = agg_fails(&one, &[], vec![stat("x", "var")], map(vec![]));
    assert_eq!(
        e,
        agg_e101("stats", "group (all rows) has 1 value for \"var\"")
    );
}

/// Criterion 5: the operand refusals.
#[test]
fn aggregate_operand_refusals() {
    let f = groups_frame();
    let ok = map(vec![]);
    assert_eq!(
        agg_fails(&f, &["g"], vec![stat("s", "mean")], ok.clone()),
        agg_e101("stats", "\"mean\" of column \"s\", a string column")
    );
    assert_eq!(
        agg_fails(&f, &["s"], vec![stat("g", "mean")], ok.clone()),
        agg_e101("stats", "\"mean\" of column \"g\", a factor")
    );
    assert_eq!(
        agg_fails(&f, &["g"], vec![stat("x", "mode")], ok.clone()),
        agg_e101("stats", "\"mode\" is not a statistic")
    );
    assert_eq!(
        agg_fails(&f, &["g"], vec![], ok.clone()),
        agg_e101("stats", "no statistics")
    );
    assert_eq!(
        agg_fails(&f, &["g"], vec![strs(&["x"])], ok.clone()),
        agg_e101("stats", &format!("item 1 is {}", strs(&["x"]).summary(40)))
    );
    assert_eq!(
        agg_fails(&f, &["g"], vec![stat("zz", "sum")], ok.clone()),
        agg_e101("stats", "no column \"zz\"")
    );
    assert_eq!(
        agg_fails(&f, &["s", "s"], vec![stat("x", "sum")], ok.clone()),
        agg_e101("by", "\"s\" given twice")
    );
    assert_eq!(
        agg_fails(
            &f,
            &["g"],
            vec![stat("x", "sum"), stat("x", "sum")],
            ok.clone()
        ),
        agg_e101("stats", "result name \"x.sum\" repeats another")
    );
    let c = raw(&["x.mean", "x"], vec![strs(&["p"]), ints(&[1])], vec![]);
    assert_eq!(
        agg_fails(&c, &["x.mean"], vec![stat("x", "mean")], ok.clone()),
        agg_e101("stats", "result name \"x.mean\" repeats a by column")
    );
    assert_eq!(
        agg_fails(
            &f,
            &["g"],
            vec![stat("x", "sum")],
            map(vec![("na", s("keep"))])
        ),
        agg_e101("opts", "na is \"keep\"")
    );
    assert_eq!(
        agg_fails(
            &f,
            &["g"],
            vec![stat("x", "sum")],
            map(vec![("how", s("left"))])
        ),
        agg_e101("opts", "the key \"how\", which this word does not take")
    );
}

/// Criterion 5: an empty `by`, no rows, ties, and `by` values equal by
/// value.
#[test]
fn aggregate_edges() {
    let f = groups_frame();
    let a = aggregate(&f, &[], vec![stat("x", "count")], map(vec![]));
    same(&a, &raw(&["x.count"], vec![ints(&[7])], vec![]));

    let empty = raw(
        &["g", "x"],
        vec![list(vec![]), list(vec![])],
        vec![("g", strs(&["b", "a"]))],
    );
    let a = aggregate(&empty, &["g"], vec![stat("x", "mean")], map(vec![]));
    same(
        &a,
        &raw(
            &["g", "x.mean"],
            vec![list(vec![]), list(vec![])],
            vec![("g", strs(&["b", "a"]))],
        ),
    );
    let a = aggregate(&empty, &[], vec![stat("x", "mean")], map(vec![]));
    same(&a, &raw(&["x.mean"], vec![list(vec![])], vec![]));

    // The first of a tie of `0.0` and `-0.0`, either way round.
    for (xs, want) in [([0.0, -0.0], 0.0f64), ([-0.0, 0.0], -0.0)] {
        let t = raw(&["x"], vec![floats(&xs)], vec![]);
        let a = aggregate(
            &t,
            &[],
            vec![stat("x", "min"), stat("x", "max")],
            map(vec![]),
        );
        assert_eq!(bits(&column(&a, "x.min")[0]), want.to_bits());
        assert_eq!(bits(&column(&a, "x.max")[0]), want.to_bits());
    }

    // `1` then `1.0` is one group whose cell is INTEGER 1; `0.0` then
    // `-0.0` one group whose cell is `0.0`.
    let k = raw(
        &["k", "x"],
        vec![list(vec![int(1), float(1.0)]), ints(&[2, 3])],
        vec![],
    );
    same(
        &aggregate(&k, &["k"], vec![stat("x", "count")], map(vec![])),
        &raw(&["k", "x.count"], vec![ints(&[1]), ints(&[2])], vec![]),
    );
    let z = raw(
        &["k", "x"],
        vec![floats(&[0.0, -0.0]), ints(&[2, 3])],
        vec![],
    );
    same(
        &aggregate(&z, &["k"], vec![stat("x", "count")], map(vec![])),
        &raw(&["k", "x.count"], vec![floats(&[0.0]), ints(&[2])], vec![]),
    );
}

fn join(l: &BundValue, r: &BundValue, by: &[&str], how: Option<&str>) -> BundValue {
    let opts = map(how.map(|h| ("how", s(h))).into_iter().collect());
    let j =
        on(&[l.clone(), r.clone(), strs(by), opts], "frame.join").unwrap_or_else(|e| panic!("{e}"));
    on(std::slice::from_ref(&j), "frame.check").unwrap_or_else(|e| panic!("{e}"));
    j
}

fn join_e101(t: &str) -> String {
    format!("FRAME.JOIN: E101 by is outside frame.join's domain: {t}")
}

/// Criterion 5: `frame.join`, inner and left.
#[test]
fn join_inner_and_left() {
    let l = raw(
        &["id", "v"],
        vec![
            list(vec![int(1), int(2), int(3), none()]),
            strs(&["a", "b", "c", "d"]),
        ],
        vec![],
    );
    let r = raw(
        &["id", "w"],
        vec![
            list(vec![float(2.0), int(1), int(2), int(9), none()]),
            ints(&[10, 20, 30, 40, 50]),
        ],
        vec![],
    );
    // A left row matching two right rows gets both, in right's order; an
    // INTEGER key matches a FLOAT; NONE matches nothing.
    same(
        &join(&l, &r, &["id"], None),
        &raw(
            &["id", "v", "w"],
            vec![
                ints(&[1, 2, 2]),
                strs(&["a", "b", "b"]),
                ints(&[20, 10, 30]),
            ],
            vec![],
        ),
    );
    let j = join(&l, &r, &["id"], Some("left"));
    same(
        &j,
        &raw(
            &["id", "v", "w"],
            vec![
                list(vec![int(1), int(2), int(2), int(3), none()]),
                strs(&["a", "b", "b", "c", "d"]),
                list(vec![int(20), int(10), int(30), none(), none()]),
            ],
            vec![],
        ),
    );
    let w = column(&j, "w");
    assert_eq!((w[3].q(), w[4].q()), (0.0, 0.0));

    // Two keys, and a factor key matching a string key by text; the answer
    // keeps left's levels, and right's factor column stays a factor.
    let l2 = raw(
        &["g", "n", "v"],
        vec![strs(&["x", "y", "x"]), ints(&[1, 1, 2]), ints(&[7, 8, 9])],
        vec![("g", strs(&["y", "x"]))],
    );
    let r2 = raw(
        &["n", "g", "f"],
        vec![
            ints(&[1, 2, 1]),
            strs(&["x", "x", "z"]),
            strs(&["u", "u", "t"]),
        ],
        vec![("f", strs(&["u", "t"]))],
    );
    for how in [None, Some("left")] {
        let j = join(&l2, &r2, &["g", "n"], how);
        let want_rows = if how.is_some() { 3 } else { 2 };
        assert_eq!(column(&j, "v").len(), want_rows);
        same(
            &j,
            &if how.is_some() {
                raw(
                    &["g", "n", "v", "f"],
                    vec![
                        strs(&["x", "y", "x"]),
                        ints(&[1, 1, 2]),
                        ints(&[7, 8, 9]),
                        list(vec![s("u"), none(), s("u")]),
                    ],
                    vec![("f", strs(&["u", "t"])), ("g", strs(&["y", "x"]))],
                )
            } else {
                raw(
                    &["g", "n", "v", "f"],
                    vec![
                        strs(&["x", "x"]),
                        ints(&[1, 2]),
                        ints(&[7, 9]),
                        strs(&["u", "u"]),
                    ],
                    vec![("f", strs(&["u", "t"])), ("g", strs(&["y", "x"]))],
                )
            },
        );
    }
}

/// Criterion 5: `frame.join`'s refusals and its cap.
#[test]
fn join_refusals() {
    let l = raw(&["id", "v"], vec![ints(&[1]), ints(&[2])], vec![]);
    let r = raw(&["id", "v"], vec![ints(&[1]), ints(&[3])], vec![]);
    let o = map(vec![]);
    assert_eq!(
        fails(
            &[l.clone(), r.clone(), strs(&["id"]), o.clone()],
            "frame.join"
        ),
        join_e101("\"v\" is in both frames")
    );
    assert_eq!(
        fails(&[l.clone(), r.clone(), strs(&[]), o.clone()], "frame.join"),
        join_e101("no keys")
    );
    assert_eq!(
        fails(
            &[l.clone(), r.clone(), strs(&["id", "id"]), o.clone()],
            "frame.join"
        ),
        join_e101("\"id\" given twice")
    );
    let r3 = raw(&["k"], vec![ints(&[1])], vec![]);
    assert_eq!(
        fails(&[l.clone(), r3, strs(&["id"]), o.clone()], "frame.join"),
        join_e101("no column \"id\" in right")
    );
    let rs = raw(&["id"], vec![strs(&["1"])], vec![]);
    assert_eq!(
        fails(&[l.clone(), rs, strs(&["id"]), o.clone()], "frame.join"),
        join_e101("key \"id\" is numeric in left and string in right")
    );
    assert_eq!(
        fails(
            &[
                l.clone(),
                l.clone(),
                strs(&["id", "v"]),
                map(vec![("how", s("outer"))])
            ],
            "frame.join"
        ),
        "FRAME.JOIN: E101 opts is outside frame.join's domain: how is \"outer\""
    );
    // 10001 × 10000 rows on one key: 100010000 rows, over the cap.
    let a = raw(&["k"], vec![ints(&vec![1; 10_001])], vec![]);
    let b = raw(&["k"], vec![ints(&vec![1; 10_000])], vec![]);
    assert_eq!(
        fails(&[a, b, strs(&["k"]), o], "frame.join"),
        "FRAME.JOIN: E111 the join's row count exceeds its cap of 100000000"
    );
}

/// Every FLOAT in two answers equal by `to_bits`, through LISTs and MAPs.
fn deep_bits(a: &BundValue, b: &BundValue) {
    match (a.unboxed(), b.unboxed()) {
        (BundValue::Float(x, _), BundValue::Float(y, _)) => {
            assert_eq!(x.to_bits(), y.to_bits(), "{x} and {y}");
        }
        _ => {
            if let (Some(p), Some(q)) = (a.as_list(), b.as_list()) {
                assert_eq!(p.len(), q.len());
                p.iter().zip(q).for_each(|(x, y)| deep_bits(x, y));
            } else if let (Some(p), Some(q)) = (a.as_map(), b.as_map()) {
                assert_eq!(p.len(), q.len());
                for ((k, x), (j, y)) in p.iter().zip(q) {
                    assert_eq!(k, j);
                    deep_bits(x, y);
                }
            } else {
                assert_eq!(a.display(), b.display());
            }
        }
    }
}

/// Criterion 5: the model path, through `frame.complete`, `frame.matrix`
/// and `frame.column`, equals `stat.lm` on the complete rows by hand.
#[test]
fn model_path() {
    let f = raw(
        &["x", "y"],
        vec![
            list(vec![int(1), int(2), none(), int(4), int(5), int(6), int(7)]),
            list(vec![
                float(2.1),
                float(3.9),
                float(6.2),
                none(),
                float(9.8),
                float(12.1),
                float(14.2),
            ]),
        ],
        vec![],
    );
    let c = on(&[f, strs(&["x", "y"])], "frame.complete").unwrap();
    let m = on(&[c.clone(), strs(&["x"]), map(vec![])], "frame.matrix").unwrap();
    let y = on(&[c, s("y")], "frame.column").unwrap();
    let got = on(&[m, y], "stat.lm").unwrap();
    let hand_x = BundValue::matrix(
        [1.0, 2.0, 5.0, 6.0, 7.0]
            .iter()
            .map(|&x| vec![float(x)])
            .collect(),
    );
    let hand_y = floats(&[2.1, 3.9, 9.8, 12.1, 14.2]);
    let want = on(&[hand_x, hand_y], "stat.lm").unwrap();
    deep_bits(&got, &want);
}

/// Criterion 5: `frame.matrix` under `"omit"`, and its refusals.
#[test]
fn matrix_omit_and_refusals() {
    let f = raw(
        &["x", "z", "g"],
        vec![
            list(vec![int(1), none(), int(3)]),
            list(vec![float(1.5), float(2.5), none()]),
            strs(&["a", "b", "a"]),
        ],
        vec![("g", strs(&["a", "b"]))],
    );
    let a = on(&[f.clone(), strs(&["x", "z"]), omit()], "frame.matrix").unwrap();
    let m = on(&[a.clone(), s("matrix")], "get").unwrap();
    let rows: Vec<Vec<u64>> = m
        .as_matrix()
        .unwrap()
        .iter()
        .map(|r| r.iter().map(bits).collect())
        .collect();
    assert_eq!(rows, vec![vec![1.0f64.to_bits(), 1.5f64.to_bits()]]);
    assert_eq!(on(&[a, s("dropped")], "get").unwrap().display(), "2");
    let e101 = |op: &str, t: &str| {
        format!("FRAME.MATRIX: E101 {op} is outside frame.matrix's domain: {t}")
    };
    assert_eq!(
        fails(&[f.clone(), strs(&["x", "g"]), map(vec![])], "frame.matrix"),
        e101("names", "column \"g\" is a factor")
    );
    assert_eq!(
        fails(&[f.clone(), strs(&[]), map(vec![])], "frame.matrix"),
        e101("names", "no columns")
    );
    assert_eq!(
        fails(&[f.clone(), strs(&["x"]), map(vec![])], "frame.matrix"),
        e101("frame", "column \"x\" row 2 is none (na \"refuse\")")
    );
    let g = raw(&["x"], vec![list(vec![none()])], vec![]);
    assert_eq!(
        fails(&[g, strs(&["x"]), omit()], "frame.matrix"),
        e101("frame", "no rows")
    );
    let h = raw(&["x"], vec![list(vec![])], vec![]);
    assert_eq!(
        fails(&[h, strs(&["x"]), map(vec![])], "frame.matrix"),
        e101("frame", "no rows")
    );
}

/// Criterion 9: every name and form on the palette, then with three and
/// four operands over a smaller one.
#[test]
fn no_operand_aborts() {
    let palette = palette();
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        abort_free(w, &palette, true);
    }
    let small: Vec<BundValue> = [0, 3, 7, 9, 10, 15]
        .iter()
        .filter_map(|&i| palette.get(i).cloned())
        .chain([omit(), strs(&["dose"]), list(vec![stat("conc", "var")])])
        .collect();
    for a in &small {
        for b in &small {
            for c in &small {
                let w = "frame.matrix";
                if let Err(e) = on(&[a.clone(), b.clone(), c.clone()], w) {
                    assert!(ok_error(&e), "{w} on {a:?} {b:?} {c:?}: {e}");
                }
                for w in ["frame.aggregate", "frame.join"] {
                    for d in &small {
                        if let Err(e) = on(&[a.clone(), b.clone(), c.clone(), d.clone()], w) {
                            assert!(ok_error(&e), "{w} on {a:?} {b:?} {c:?} {d:?}: {e}");
                        }
                    }
                    if let Err(e) =
                        run_on(&[a.clone(), b.clone()], Some(c), &format!("{w}.")).map(|_| ())
                    {
                        assert!(ok_error(&e), "{w}. on {a:?} {b:?} {c:?}: {e}");
                    }
                }
            }
        }
    }
}
