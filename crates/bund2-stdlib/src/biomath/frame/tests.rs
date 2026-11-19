//! RFC-0017 criteria 1, 2, 4 (its first part) and 9 for M1's `frame.`
//! words.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::na::none;
use crate::biomath::testkit::{frame_eq, run_on};

pub(super) fn s(x: &str) -> BundValue {
    BundValue::str(x)
}

pub(super) fn list(xs: Vec<BundValue>) -> BundValue {
    BundValue::list(xs)
}

pub(super) fn strs(xs: &[&str]) -> BundValue {
    list(xs.iter().map(|x| s(x)).collect())
}

/// A MAP of exactly the given keys.
pub(super) fn map(kv: Vec<(&str, BundValue)>) -> BundValue {
    BundValue::map(kv.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// A frame as a raw MAP, nothing checked.
pub(super) fn raw(
    names: &[&str],
    cols: Vec<BundValue>,
    levels: Vec<(&str, BundValue)>,
) -> BundValue {
    let data = names
        .iter()
        .map(|n| n.to_string())
        .zip(cols)
        .collect::<BTreeMap<_, _>>();
    map(vec![
        ("columns", strs(names)),
        ("data", BundValue::map(data)),
        ("levels", map(levels)),
    ])
}

/// The fixed 5-row frame: an INTEGER column with a NONE, a FLOAT column,
/// and a factor whose levels are not in sorted order.
pub(super) fn five() -> BundValue {
    raw(
        &["dose", "conc", "group"],
        vec![
            list(vec![
                BundValue::int(1),
                BundValue::int(2),
                none(),
                BundValue::int(4),
                BundValue::int(5),
            ]),
            list([0.5, 1.5, 2.5, 3.5, 4.5].map(BundValue::float).to_vec()),
            strs(&["a", "b", "a", "b", "b"]),
        ],
        vec![("group", strs(&["b", "a"]))],
    )
}

/// Run `src` on `stack`; its error text, or its top.
pub(super) fn on(stack: &[BundValue], src: &str) -> Result<BundValue, String> {
    let mut i = run_on(stack, None, src)?;
    i.pull().ok_or_else(|| format!("{src}: empty"))
}

pub(super) fn fails(stack: &[BundValue], src: &str) -> String {
    match on(stack, src) {
        Ok(v) => panic!("{src} answered {}", v.display()),
        Err(e) => e,
    }
}

#[test]
fn every_word_is_registered_with_its_effect() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        let consumes = match w.trim_end_matches('.') {
            "frame.new" | "frame.column" => 2,
            _ => 1,
        };
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
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("frame."))
        .count();
    assert_eq!(
        ours,
        WORDS.len()
            + super::reshape::WORDS.len()
            + super::csv::WORDS.len()
            + super::summary::WORDS.len()
    );
}

/// Criterion 1: one `.` form of each effect shape.
#[test]
fn forms_use_the_workbench() {
    let mut i = run_on(&[], Some(&five()), "frame.nrow.").unwrap();
    assert_eq!(i.pull_workbench().unwrap().display(), "5");
    let mut i = run_on(&[five()], Some(&s("conc")), "frame.column.").unwrap();
    let c = i.pull_workbench().unwrap();
    assert_eq!(c.as_list().unwrap().len(), 5);
    let mut i = run_on(
        &[strs(&["x"])],
        Some(&list(vec![strs(&["a"])])),
        "frame.new.",
    )
    .unwrap();
    let f = i.pull_workbench().unwrap();
    assert!(frame_eq(&f, &raw(&["x"], vec![strs(&["a"])], vec![])));
}

/// Criterion 2: each of §S8's first eleven rows, alone; a frame of no
/// columns and one of no rows accepted.
#[test]
fn the_frame_check() {
    let ok = |v: BundValue| on(&[v], "frame.check").map(|_| ());
    let e101 = |t: &str| format!("FRAME.CHECK: E101 frame is outside frame.check's domain: {t}");
    let x3 = |cells: Vec<BundValue>| raw(&["x"], vec![list(cells)], vec![]);
    let one = || BundValue::int(1);
    let cases: Vec<(BundValue, String)> = vec![
        // Row 1.
        (BundValue::int(3), e101("3")),
        // Row 2.
        (
            map(vec![("columns", strs(&[])), ("data", map(vec![]))]),
            e101("a missing key \"levels\""),
        ),
        (
            map(vec![
                ("columns", strs(&[])),
                ("data", map(vec![])),
                ("levels", map(vec![])),
                ("x", one()),
            ]),
            e101("an extra key \"x\""),
        ),
        // Row 3.
        (
            map(vec![
                ("columns", list(vec![s("a"), one()])),
                ("data", map(vec![])),
                ("levels", map(vec![])),
            ]),
            e101("column name 2 is 1"),
        ),
        (
            map(vec![
                ("columns", strs(&["a", "a"])),
                ("data", map(vec![])),
                ("levels", map(vec![])),
            ]),
            e101("a repeated name \"a\""),
        ),
        (
            map(vec![
                ("columns", strs(&[""])),
                ("data", map(vec![])),
                ("levels", map(vec![])),
            ]),
            e101("column name 1 is \"\""),
        ),
        // Row 4.
        (
            map(vec![
                ("columns", strs(&["a", "b"])),
                ("data", map(vec![("a", list(vec![]))])),
                ("levels", map(vec![])),
            ]),
            e101("data has no column \"b\""),
        ),
        (
            map(vec![
                ("columns", strs(&["a"])),
                ("data", map(vec![("a", list(vec![])), ("y", list(vec![]))])),
                ("levels", map(vec![])),
            ]),
            e101("data has a key \"y\" not in columns"),
        ),
        // Row 5.
        (
            raw(&["a"], vec![BundValue::int(3)], vec![]),
            e101("column \"a\" is 3"),
        ),
        // Row 6.
        (
            raw(
                &["a", "b"],
                vec![list(vec![one(), one()]), list(vec![one()])],
                vec![],
            ),
            "FRAME.CHECK: E102 the lengths of \"a\" and \"b\" do not agree: 2 and 1".into(),
        ),
        // Row 7.
        (
            raw(&["g"], vec![strs(&["a"])], vec![("z", strs(&["a"]))]),
            e101("levels has a key \"z\" not in columns"),
        ),
        (
            raw(&["g"], vec![strs(&["a"])], vec![("g", strs(&["a", "a"]))]),
            e101("levels of \"g\" repeat \"a\""),
        ),
        (
            raw(&["g"], vec![strs(&["a"])], vec![("g", list(vec![one()]))]),
            e101("levels of \"g\" hold 1"),
        ),
        // Row 8.
        (
            x3(vec![one(), one(), list(vec![])]),
            e101("column \"x\" row 3 is a LIST"),
        ),
        // Row 9.
        (
            x3(vec![one(), one(), BundValue::float(f64::NAN)]),
            e101("column \"x\" row 3 is NaN"),
        ),
        // Row 10.
        (
            x3(vec![one(), one(), s("abc")]),
            "FRAME.CHECK: E103 column \"x\" is not a LIST of numbers: \"abc\" at row 3".into(),
        ),
        // Row 11.
        (
            raw(
                &["ok"],
                vec![list(vec![BundValue::boolean(true), none(), s("y")])],
                vec![],
            ),
            e101("column \"ok\" row 3 is a STRING; the column is logical"),
        ),
        (
            raw(
                &["g"],
                vec![strs(&["a", "b", "c"])],
                vec![("g", strs(&["a", "b"]))],
            ),
            e101("column \"g\" row 3 is \"c\", not one of its levels"),
        ),
        // A factor column holding a number beside a valid level: row 11,
        // naming the number, not E103.
        (
            raw(
                &["g"],
                vec![list(vec![s("a"), one()])],
                vec![("g", strs(&["a"]))],
            ),
            e101("column \"g\" row 2 is 1, not one of its levels"),
        ),
    ];
    for (v, want) in cases {
        assert_eq!(ok(v.clone()).unwrap_err(), want, "{}", v.display());
    }
    // Rule by rule: a LIST in the last column before a NaN in the first.
    let both = raw(
        &["a", "b"],
        vec![
            list(vec![BundValue::float(f64::INFINITY)]),
            list(vec![list(vec![])]),
        ],
        vec![],
    );
    assert_eq!(ok(both).unwrap_err(), e101("column \"b\" row 1 is a LIST"));
    // A STRING holding a line break is escaped, on one line.
    let long = "q\"\n".repeat(40);
    let e = ok(raw(&["a"], vec![list(vec![one(), s(&long)])], vec![])).unwrap_err();
    assert!(!e.contains('\n') && e.contains("\\\"\\n"), "{e}");
    // Accepted: no columns; no rows; a mixed numeric column; all NONE; a
    // factor with an unused level and a NONE.
    for v in [
        raw(&[], vec![], vec![]),
        raw(
            &["a", "g"],
            vec![list(vec![]), list(vec![])],
            vec![("g", strs(&["x"]))],
        ),
        x3(vec![one(), BundValue::float(2.5), none()]),
        x3(vec![none(), none(), none()]),
        raw(
            &["g"],
            vec![list(vec![s("a"), none()])],
            vec![("g", strs(&["b", "a"]))],
        ),
        five(),
    ] {
        ok(v.clone()).unwrap_or_else(|e| panic!("{}: {e}", v.display()));
    }
}

/// Criterion 4, its first part.
#[test]
fn reading_a_frame() {
    let f = five();
    // `frame.check` answers its operand, unchanged.
    assert!(frame_eq(
        &on(std::slice::from_ref(&f), "frame.check").unwrap(),
        &f
    ));
    assert_eq!(
        on(std::slice::from_ref(&f), "frame.nrow")
            .unwrap()
            .display(),
        "5"
    );
    assert_eq!(
        on(std::slice::from_ref(&f), "frame.ncol")
            .unwrap()
            .display(),
        "3"
    );
    let names = on(std::slice::from_ref(&f), "frame.names").unwrap();
    assert_eq!(names.display(), strs(&["dose", "conc", "group"]).display());
    let dose = on(&[f.clone(), s("dose")], "frame.column").unwrap();
    let dose = dose.as_list().unwrap();
    assert_eq!(dose.len(), 5);
    assert_eq!(dose[0].unboxed(), &BundValue::int(1));
    assert_eq!(dose[2].dt(), bund2_value::NONE);
    assert_eq!(dose[2].q(), 0.0, "a NONE keeps its q");
    // Each row's MAP.
    let rows = on(std::slice::from_ref(&f), "frame.rows").unwrap();
    let rows = rows.as_list().unwrap();
    assert_eq!(rows.len(), 5);
    let row3 = rows[2].as_map().unwrap();
    assert_eq!(row3.len(), 3);
    assert_eq!(row3["dose"].dt(), bund2_value::NONE);
    assert_eq!(row3["conc"].unboxed(), &BundValue::float(2.5));
    assert_eq!(row3["group"].as_str().as_deref(), Some("a"));
    // `frame.new`: cells keep their types; no factors.
    let built = on(
        &[
            strs(&["n", "x"]),
            list(vec![
                list(vec![BundValue::int(1), none()]),
                list(vec![BundValue::float(1.0), BundValue::int(2)]),
            ]),
        ],
        "frame.new",
    )
    .unwrap();
    let want = raw(
        &["n", "x"],
        vec![
            list(vec![BundValue::int(1), none()]),
            list(vec![BundValue::float(1.0), BundValue::int(2)]),
        ],
        vec![],
    );
    assert!(frame_eq(&built, &want), "{}", built.display());
    assert!(!frame_eq(
        &built,
        &raw(
            &["n", "x"],
            vec![
                list(vec![BundValue::int(1), none()]),
                list(vec![BundValue::int(1), BundValue::int(2)]),
            ],
            vec![],
        )
    ));
    on(&[built], "frame.check").unwrap();
    // A frame of no columns has 0 rows.
    let empty = on(&[strs(&[]), list(vec![])], "frame.new").unwrap();
    assert_eq!(on(&[empty], "frame.nrow").unwrap().display(), "0");
}

/// §S8's rows for M1's own operands.
#[test]
fn operand_refusals() {
    let f = five();
    assert_eq!(
        fails(&[f.clone(), s("zz")], "frame.column"),
        "FRAME.COLUMN: E101 name is outside frame.column's domain: no column \"zz\""
    );
    assert_eq!(
        fails(&[f.clone(), BundValue::int(1)], "frame.column"),
        "FRAME.COLUMN: E101 name is outside frame.column's domain: 1"
    );
    assert_eq!(
        fails(
            &[
                strs(&["a", "b", "c"]),
                list(vec![list(vec![]), list(vec![])])
            ],
            "frame.new"
        ),
        "FRAME.NEW: E102 names and columns do not agree: 3 and 2"
    );
    assert_eq!(
        fails(&[s("a"), list(vec![])], "frame.new"),
        "FRAME.NEW: E101 names is outside frame.new's domain: \"a\""
    );
    assert_eq!(
        fails(&[strs(&[]), s("a")], "frame.new"),
        "FRAME.NEW: E101 columns is outside frame.new's domain: \"a\""
    );
    // The frame check's rows name the operand the fault came from.
    assert_eq!(
        fails(
            &[strs(&["a", "a"]), list(vec![list(vec![]), list(vec![])])],
            "frame.new"
        ),
        "FRAME.NEW: E101 names is outside frame.new's domain: a repeated name \"a\""
    );
    assert_eq!(
        fails(&[strs(&["a"]), list(vec![BundValue::int(1)])], "frame.new"),
        "FRAME.NEW: E101 columns is outside frame.new's domain: column \"a\" is 1"
    );
    assert_eq!(
        fails(
            &[
                strs(&["a"]),
                list(vec![list(vec![s("x"), BundValue::int(1)])])
            ],
            "frame.new"
        ),
        "FRAME.NEW: E103 column \"a\" is not a LIST of numbers: \"x\" at row 1"
    );
}

/// Criterion 9: every name and form, on a palette of NaN, ±∞, wrong
/// types, empty, ragged, NONE-filled and malformed frames.
#[test]
fn no_operand_aborts() {
    let palette = palette();
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        abort_free(w, &palette, true);
    }
}

/// Whether an error is one criterion 9 allows: an E1nn, a shallow stack,
/// or a bridge error.
pub(super) fn ok_error(e: &str) -> bool {
    e.contains(": E1") || e.contains("too shallow") || e.contains("returned")
}

/// Criterion 9's palette.
pub(super) fn palette() -> Vec<BundValue> {
    vec![
        BundValue::float(f64::NAN),
        BundValue::float(f64::NEG_INFINITY),
        BundValue::int(0),
        s("dose"),
        s(""),
        none(),
        list(vec![]),
        strs(&["a"]),
        list(vec![list(vec![none()])]),
        map(vec![]),
        five(),
        raw(&[], vec![], vec![]),
        raw(&["a", "b"], vec![list(vec![none()]), list(vec![])], vec![]),
        raw(&["a"], vec![list(vec![none(), none()])], vec![]),
        raw(&["a"], vec![list(vec![BundValue::float(f64::NAN)])], vec![]),
        raw(&["g"], vec![strs(&["a"])], vec![("g", BundValue::int(1))]),
    ]
}

/// `w`, and its `.` form when `bench`, on no operand and on one and two
/// from `palette`.
pub(super) fn abort_free(w: &str, palette: &[BundValue], bench: bool) {
    {
        if let Err(e) = on(&[], w) {
            assert!(ok_error(&e), "{w}: {e}");
        }
        for a in palette {
            let mut runs = vec![(vec![a.clone()], w.to_string())];
            if bench {
                runs.push((vec![], format!("{w}.")));
            }
            for (stack, src) in runs {
                let wb = (stack.is_empty()).then_some(a);
                if let Err(e) = run_on(&stack, wb, &src).map(|_| ()) {
                    assert!(ok_error(&e), "{src} on {}: {e}", a.display());
                }
            }
            for b in palette {
                if let Err(e) = on(&[a.clone(), b.clone()], w) {
                    assert!(ok_error(&e), "{w} on {} {}: {e}", a.display(), b.display());
                }
                if !bench {
                    continue;
                }
                if let Err(e) =
                    run_on(std::slice::from_ref(a), Some(b), &format!("{w}.")).map(|_| ())
                {
                    assert!(ok_error(&e), "{w}. on {} {}: {e}", a.display(), b.display());
                }
            }
        }
    }
}
