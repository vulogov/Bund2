//! RFC-0017 criteria 1, 4 (the rest) and 9 for M2's `frame.` words.

use bund2_api::Vm as _;
use bund2_value::{BundValue, NONE};

use super::WORDS;
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

fn bool_(b: bool) -> BundValue {
    BundValue::boolean(b)
}

/// The frame `src` answers on `stack`, checked: it passes `frame.check`.
fn framed(stack: &[BundValue], src: &str) -> BundValue {
    let f = on(stack, src).unwrap_or_else(|e| panic!("{src}: {e}"));
    on(std::slice::from_ref(&f), "frame.check")
        .unwrap_or_else(|e| panic!("{src} answered a frame that fails the check: {e}"));
    f
}

fn same(got: &BundValue, want: &BundValue) {
    assert!(
        frame_eq(got, want),
        "got  {}\nwant {}",
        got.display(),
        want.display()
    );
}

/// `five()`'s rows, picked by 1-based index, as a raw frame.
fn five_rows(rows: &[usize]) -> BundValue {
    let dose = [Some(1), Some(2), None, Some(4), Some(5)];
    let conc = [0.5, 1.5, 2.5, 3.5, 4.5];
    let group = ["a", "b", "a", "b", "b"];
    raw(
        &["dose", "conc", "group"],
        vec![
            list(
                rows.iter()
                    .map(|&r| dose[r - 1].map_or_else(none, int))
                    .collect(),
            ),
            list(rows.iter().map(|&r| float(conc[r - 1])).collect()),
            list(rows.iter().map(|&r| s(group[r - 1])).collect()),
        ],
        vec![("group", strs(&["b", "a"]))],
    )
}

#[test]
fn every_word_is_registered_with_its_effect() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        let base = w.trim_end_matches('.');
        let consumes = match base {
            "frame.with" | "frame.sort" | "frame.factor.levels" => 3,
            _ => 2,
        };
        if base == "frame.filter" {
            assert_eq!((e.consumes, e.opaque), (2, true), "{w}");
            continue;
        }
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
    // `frame.filter` runs a lambda, so it has no `.` form (§S1).
    assert!(i.registry.effect_of("frame.filter.").is_none());
}

/// Criterion 1: a `.` form of three operands, and of two.
#[test]
fn forms_use_the_workbench() {
    let col = list((1..=5).map(int).collect());
    let mut i = run_on(&[five(), s("n")], Some(&col), "frame.with.").unwrap();
    let f = i.pull_workbench().unwrap();
    assert_eq!(i.depth(), 0);
    assert_eq!(
        on(&[f], "frame.names").unwrap().display(),
        strs(&["dose", "conc", "group", "n"]).display()
    );
    let mut i = run_on(&[five()], Some(&int(2)), "frame.head.").unwrap();
    same(&i.pull_workbench().unwrap(), &five_rows(&[1, 2]));
}

#[test]
fn select_take_head() {
    let f = five();
    let sel = framed(&[f.clone(), strs(&["group", "dose"])], "frame.select");
    let want = raw(
        &["group", "dose"],
        vec![
            strs(&["a", "b", "a", "b", "b"]),
            list(vec![int(1), int(2), none(), int(4), int(5)]),
        ],
        vec![("group", strs(&["b", "a"]))],
    );
    same(&sel, &want);
    same(
        &framed(&[f.clone(), list(vec![])], "frame.select"),
        &raw(&[], vec![], vec![]),
    );
    assert_eq!(
        fails(&[f.clone(), strs(&["dose", "dose"])], "frame.select"),
        "FRAME.SELECT: E101 names is outside frame.select's domain: \"dose\" given twice"
    );
    assert_eq!(
        fails(&[f.clone(), strs(&["zz"])], "frame.select"),
        "FRAME.SELECT: E101 names is outside frame.select's domain: no column \"zz\""
    );
    assert_eq!(
        fails(&[f.clone(), s("dose")], "frame.select"),
        "FRAME.SELECT: E101 names is outside frame.select's domain: \"dose\""
    );
    assert_eq!(
        fails(&[f.clone(), list(vec![s("dose"), int(1)])], "frame.select"),
        "FRAME.SELECT: E101 names is outside frame.select's domain: item 2 is 1"
    );

    // 1-based, in order, a repeat allowed.
    let took = framed(
        &[f.clone(), list(vec![int(3), int(1), int(3)])],
        "frame.take",
    );
    same(&took, &five_rows(&[3, 1, 3]));
    for (r, text) in [
        (int(0), "row 0 of a frame of 5 rows"),
        (int(6), "row 6 of a frame of 5 rows"),
        (int(-1), "row -1 of a frame of 5 rows"),
        (float(1.0), "item 1 is 1.0"),
    ] {
        assert_eq!(
            fails(&[f.clone(), list(vec![r])], "frame.take"),
            format!("FRAME.TAKE: E101 rows is outside frame.take's domain: {text}")
        );
    }

    same(
        &framed(&[f.clone(), int(2)], "frame.head"),
        &five_rows(&[1, 2]),
    );
    same(&framed(&[f.clone(), int(9)], "frame.head"), &f);
    same(&framed(&[f.clone(), int(0)], "frame.head"), &five_rows(&[]));
    assert_eq!(
        fails(&[f.clone(), int(-1)], "frame.head"),
        "FRAME.HEAD: E101 n is outside frame.head's domain: -1"
    );
    assert_eq!(
        fails(&[f, float(2.0)], "frame.head"),
        "FRAME.HEAD: E101 n is outside frame.head's domain: 2.0"
    );
}

/// Cells keep their q through a move (§S1).
#[test]
fn cells_keep_their_q() {
    let f = raw(
        &["x"],
        vec![list(vec![int(1).with_q(0.25), float(2.0).with_q(0.75)])],
        vec![],
    );
    let took = framed(&[f, list(vec![int(2), int(1)])], "frame.take");
    let x = on(&[took, s("x")], "frame.column").unwrap();
    let x = x.as_list().unwrap();
    assert_eq!((x[0].q(), x[1].q()), (0.75, 0.25));
    assert_eq!(x[1].unboxed(), &int(1));
}

#[test]
fn with_replaces_and_appends() {
    let f = five();
    // Append, last.
    let flags = list(vec![
        bool_(true),
        bool_(false),
        none(),
        bool_(true),
        bool_(true),
    ]);
    let got = framed(&[f.clone(), s("ok"), flags.clone()], "frame.with");
    let mut want_cols = vec![
        list(vec![int(1), int(2), none(), int(4), int(5)]),
        list([0.5, 1.5, 2.5, 3.5, 4.5].map(float).to_vec()),
        strs(&["a", "b", "a", "b", "b"]),
    ];
    want_cols.push(flags);
    same(
        &got,
        &raw(
            &["dose", "conc", "group", "ok"],
            want_cols,
            vec![("group", strs(&["b", "a"]))],
        ),
    );
    // Replace in place; a factor replaced loses its levels.
    let g = strs(&["x", "y", "x", "y", "z"]);
    let got = framed(&[f.clone(), s("group"), g.clone()], "frame.with");
    same(
        &got,
        &raw(
            &["dose", "conc", "group"],
            vec![
                list(vec![int(1), int(2), none(), int(4), int(5)]),
                list([0.5, 1.5, 2.5, 3.5, 4.5].map(float).to_vec()),
                g,
            ],
            vec![],
        ),
    );
    assert_eq!(
        fails(&[f.clone(), s("n"), list(vec![int(1); 4])], "frame.with"),
        "FRAME.WITH: E102 the list and the frame's rows do not agree: 4 and 5"
    );
    // On a frame of no columns, the new column sets the row count.
    let empty = raw(&[], vec![], vec![]);
    let three = list(vec![int(1), none(), int(3)]);
    let got = framed(&[empty, s("n"), three.clone()], "frame.with");
    same(&got, &raw(&["n"], vec![three.clone()], vec![]));
    assert_eq!(
        on(std::slice::from_ref(&got), "frame.nrow")
            .unwrap()
            .display(),
        "3"
    );
    // And the next column must agree with it.
    assert_eq!(
        fails(&[got, s("m"), list(vec![int(1); 2])], "frame.with"),
        "FRAME.WITH: E102 the list and the frame's rows do not agree: 2 and 3"
    );
    assert_eq!(
        fails(&[f.clone(), s("n"), s("x")], "frame.with"),
        "FRAME.WITH: E101 list is outside frame.with's domain: \"x\""
    );
    assert_eq!(
        fails(&[f.clone(), s(""), list(vec![int(1); 5])], "frame.with"),
        "FRAME.WITH: E101 name is outside frame.with's domain: an empty name"
    );
    // The new cells pass the column rules, naming `list`.
    let mixed = list(vec![bool_(true), s("x"), none(), none(), none()]);
    assert_eq!(
        fails(&[f.clone(), s("n"), mixed], "frame.with"),
        "FRAME.WITH: E101 list is outside frame.with's domain: column \"n\" row 2 is \
         a STRING; the column is logical"
    );
    let nan = list(vec![float(f64::NAN), none(), none(), none(), none()]);
    assert_eq!(
        fails(&[f, s("n"), nan], "frame.with"),
        "FRAME.WITH: E101 list is outside frame.with's domain: column \"n\" row 1 is NaN"
    );
}

/// The lambda on two columns, guarding `dose`'s NONE with `na.is`: rows
/// whose dose is above 1 and whose conc is below 4.0. A comparison takes
/// its top operand first, so `x 1 <` asks whether 1 is below x.
const KEEP: &str = "{ dup \"dose\" get na.is \
                    { dup \"dose\" get 1 < swap \"conc\" get 4.0 > and } \
                    { drop false } ?true* }";

#[test]
fn filter() {
    let f = five();
    let got = framed(std::slice::from_ref(&f), &format!("{KEEP} frame.filter"));
    same(&got, &five_rows(&[2, 4]));
    let all = framed(std::slice::from_ref(&f), "{ drop true } frame.filter");
    same(&all, &f);
    assert_eq!(
        fails(std::slice::from_ref(&f), "{ drop 1 } frame.filter"),
        "FRAME.FILTER: E112 the model lambda left an INTEGER, where a BOOL was required"
    );
    assert_eq!(
        fails(std::slice::from_ref(&f), "{ drop } frame.filter"),
        "FRAME.FILTER: E112 the model lambda left nothing, where a BOOL was required"
    );
    assert_eq!(
        fails(&[f.clone(), s("x")], "frame.filter"),
        "FRAME.FILTER: E101 lambda is outside frame.filter's domain: \"x\""
    );
    // The caller's stack is not the lambda's: a value under the frame stays.
    let mut i = run_on(&[int(7), f], None, "{ drop true } frame.filter").unwrap();
    i.pull().unwrap();
    assert_eq!(i.pull().map(|v| v.display()), Some("7".into()));
}

#[test]
fn complete() {
    let f = raw(
        &["x", "y"],
        vec![
            list(vec![int(1), none(), int(3), int(4)]),
            list(vec![s("a"), s("b"), none(), s("d")]),
        ],
        vec![],
    );
    let only_x = framed(&[f.clone(), strs(&["x"])], "frame.complete");
    same(
        &only_x,
        &raw(
            &["x", "y"],
            vec![
                list(vec![int(1), int(3), int(4)]),
                list(vec![s("a"), none(), s("d")]),
            ],
            vec![],
        ),
    );
    let every = framed(&[f.clone(), list(vec![])], "frame.complete");
    same(
        &every,
        &raw(
            &["x", "y"],
            vec![list(vec![int(1), int(4)]), strs(&["a", "d"])],
            vec![],
        ),
    );
    assert_eq!(
        fails(&[f, strs(&["x", "x"])], "frame.complete"),
        "FRAME.COMPLETE: E101 names is outside frame.complete's domain: \"x\" given twice"
    );
}

fn sorted(f: &BundValue, names: &[&str], desc: &[&str]) -> BundValue {
    let opts = if desc.is_empty() {
        map(vec![])
    } else {
        map(vec![("descending", strs(desc))])
    };
    framed(&[f.clone(), strs(names), opts], "frame.sort")
}

fn column(f: &BundValue, name: &str) -> Vec<String> {
    let c = on(&[f.clone(), s(name)], "frame.column").unwrap();
    c.as_list()
        .unwrap()
        .iter()
        .map(|x| match x.dt() {
            NONE => "none".into(),
            _ => x.display(),
        })
        .collect()
}

#[test]
fn sort() {
    // Two keys, the second descending; NONE last in both; ties stable.
    let f = raw(
        &["k", "v", "id"],
        vec![
            list(vec![int(2), none(), int(1), int(2), float(1.0), int(2)]),
            list(vec![
                float(1.0),
                float(9.0),
                none(),
                float(3.0),
                float(2.0),
                none(),
            ]),
            list((1..=6).map(int).collect()),
        ],
        vec![],
    );
    let got = sorted(&f, &["k", "v"], &["v"]);
    // k: 1 (row 3) and 1.0 (row 5) tie as numbers; v descends within, NONE
    // last: row 5 (2.0) then row 3 (none). k 2: v 3.0, 1.0, none. k none.
    assert_eq!(column(&got, "id"), ["5", "3", "4", "1", "6", "2"]);
    // Descending k: NONE still last.
    let got = sorted(&f, &["k"], &["k"]);
    assert_eq!(column(&got, "id"), ["1", "4", "6", "3", "5", "2"]);
    // A factor key by its level order (b before a), stable within.
    let got = sorted(&five(), &["group"], &[]);
    same(&got, &five_rows(&[2, 4, 5, 1, 3]));
    // A BOOL key, false first.
    let b = raw(
        &["b", "id"],
        vec![
            list(vec![bool_(true), none(), bool_(false), bool_(true)]),
            list((1..=4).map(int).collect()),
        ],
        vec![],
    );
    assert_eq!(column(&sorted(&b, &["b"], &[]), "id"), ["3", "1", "4", "2"]);
    // `-0.0` after `0.0` keeps that order, and the other way round too.
    for xs in [[0.0, -0.0], [-0.0, 0.0]] {
        let z = raw(
            &["z", "id"],
            vec![list(xs.map(float).to_vec()), list(vec![int(1), int(2)])],
            vec![],
        );
        assert_eq!(column(&sorted(&z, &["z"], &[]), "id"), ["1", "2"]);
        assert_eq!(column(&sorted(&z, &["z"], &["z"]), "id"), ["1", "2"]);
    }
    // STRINGs by bytes.
    let t = raw(&["t"], vec![strs(&["b", "B", "a", "é"])], vec![]);
    assert_eq!(column(&sorted(&t, &["t"], &[]), "t"), ["B", "a", "b", "é"]);

    // Refusals.
    let e = |opts: BundValue| fails(&[f.clone(), strs(&["k"]), opts], "frame.sort");
    let head = "FRAME.SORT: E101 opts is outside frame.sort's domain:";
    assert_eq!(
        e(map(vec![("descending", strs(&["v"]))])),
        format!("{head} descending names \"v\", not among names")
    );
    assert_eq!(
        e(map(vec![("how", s("inner"))])),
        format!("{head} the key \"how\", which this word does not take")
    );
    assert_eq!(
        e(map(vec![("zz", int(1))])),
        format!("{head} an unknown key \"zz\"")
    );
    assert_eq!(e(int(1)), format!("{head} 1"));
    assert_eq!(
        e(map(vec![("descending", s("k"))])),
        format!("{head} descending is \"k\"")
    );
    assert_eq!(
        e(map(vec![("descending", list(vec![s("k"), int(2)]))])),
        format!("{head} descending item 2 is 2")
    );
    assert_eq!(
        e(map(vec![("descending", strs(&["k", "k"]))])),
        format!("{head} \"k\" given twice")
    );
    assert_eq!(
        fails(&[f, strs(&["zz"]), map(vec![])], "frame.sort"),
        "FRAME.SORT: E101 names is outside frame.sort's domain: no column \"zz\""
    );
}

#[test]
fn factors() {
    let f = raw(
        &["s", "n", "e", "g"],
        vec![
            strs(&["b", "a", "b"]),
            list(vec![int(1), int(2), int(3)]),
            list(vec![none(), none(), none()]),
            strs(&["b", "a", "b"]),
        ],
        vec![("g", strs(&["c", "b", "a"]))],
    );
    let levels = |f: &BundValue, name: &str| {
        let m = f.as_map().unwrap()["levels"].as_map().unwrap().clone();
        m.get(name).map(|l| l.display())
    };
    // A string column: its distinct values, sorted.
    let got = framed(&[f.clone(), s("s")], "frame.factor");
    assert_eq!(levels(&got, "s"), Some(strs(&["a", "b"]).display()));
    // An all-NONE column: a factor with no levels.
    let got = framed(&[f.clone(), s("e")], "frame.factor");
    assert_eq!(levels(&got, "e"), Some(list(vec![]).display()));
    // A factor: its custom order kept, the unused level dropped.
    let got = framed(&[f.clone(), s("g")], "frame.factor");
    assert_eq!(levels(&got, "g"), Some(strs(&["b", "a"]).display()));
    // The cells are untouched.
    same(
        &framed(&[got, strs(&["s", "n", "e", "g"])], "frame.select"),
        &raw(
            &["s", "n", "e", "g"],
            vec![
                strs(&["b", "a", "b"]),
                list(vec![int(1), int(2), int(3)]),
                list(vec![none(), none(), none()]),
                strs(&["b", "a", "b"]),
            ],
            vec![("g", strs(&["b", "a"]))],
        ),
    );
    assert_eq!(
        fails(&[f.clone(), s("n")], "frame.factor"),
        "FRAME.FACTOR: E101 name is outside frame.factor's domain: column \"n\" is numeric"
    );
    assert_eq!(
        fails(&[f.clone(), s("zz")], "frame.factor"),
        "FRAME.FACTOR: E101 name is outside frame.factor's domain: no column \"zz\""
    );

    // `frame.factor.levels`.
    let got = framed(
        &[f.clone(), s("s"), strs(&["b", "a", "c"])],
        "frame.factor.levels",
    );
    assert_eq!(levels(&got, "s"), Some(strs(&["b", "a", "c"]).display()));
    let head = "FRAME.FACTOR.LEVELS: E101 levels is outside frame.factor.levels's domain:";
    assert_eq!(
        fails(&[f.clone(), s("s"), strs(&["b"])], "frame.factor.levels"),
        format!("{head} \"a\" at row 2 is not one of the levels")
    );
    assert_eq!(
        fails(
            &[f.clone(), s("s"), strs(&["a", "b", "a"])],
            "frame.factor.levels"
        ),
        format!("{head} \"a\" given twice")
    );
    assert_eq!(
        fails(&[f.clone(), s("s"), list(vec![])], "frame.factor.levels"),
        format!("{head} list/0")
    );
    assert_eq!(
        fails(&[f, s("n"), strs(&["a"])], "frame.factor.levels"),
        "FRAME.FACTOR.LEVELS: E101 name is outside frame.factor.levels's domain: \
         column \"n\" is numeric"
    );
}

/// Criterion 9: every M2 name and form on the palette, three operands
/// included.
#[test]
fn no_operand_aborts() {
    let palette = palette();
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        abort_free(w, &palette, *w != "frame.filter");
    }
    // Three operands, over a smaller palette.
    let small: Vec<BundValue> = [0, 3, 4, 5, 7, 9, 10, 15]
        .iter()
        .filter_map(|&i| palette.get(i).cloned())
        .collect();
    for w in ["frame.with", "frame.sort", "frame.factor.levels"] {
        for a in &small {
            for b in &small {
                for c in &small {
                    if let Err(e) = on(&[a.clone(), b.clone(), c.clone()], w) {
                        assert!(ok_error(&e), "{w} on {a:?} {b:?} {c:?}: {e}");
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
    // `frame.filter`'s lambdas, on each frame of the palette.
    for f in &palette {
        for l in [
            "{ drop true }",
            "{ drop }",
            "{ 1 }",
            "{ drop 1 }",
            "{ \"zz\" get }",
        ] {
            if let Err(e) = on(std::slice::from_ref(f), &format!("{l} frame.filter")) {
                // The lambda's own failure comes back through the bridge
                // as it was raised (criterion 9's bridge error).
                let bridged = e.starts_with("GET returns error");
                assert!(ok_error(&e) || bridged, "{l} on {}: {e}", f.display());
            }
        }
    }
}
