//! RFC-0017 criteria 1, 3 and 9 for §S3's `na` words.

use bund2_api::Vm as _;
use bund2_value::{BundValue, NONE};

use super::WORDS;
use crate::biomath::testkit::{err, run, run_on};

fn top(src: &str) -> BundValue {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    i.pull().unwrap_or_else(|| panic!("{src}: empty"))
}

fn shown(v: &BundValue) -> String {
    v.display()
}

#[test]
fn every_word_is_registered_with_its_effect() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        let consumes = match w.trim_end_matches('.') {
            "na" => 0,
            "na.fill" => 2,
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
        .filter(|n| n == "na" || n.starts_with("na."))
        .count();
    assert_eq!(ours, WORDS.len());
    // `na` consumes nothing, so it has no `.` form.
    assert!(i.registry.effect_of("na.").is_none());
}

/// Criterion 3.
#[test]
fn missing_values() {
    assert_eq!(shown(&top("na na.is")), "true");
    for x in ["0", "nodata", "\"\""] {
        assert_eq!(shown(&top(&format!("{x} na.is"))), "false", "{x}");
    }
    // NONE at the ends and in the middle.
    let list = BundValue::list(vec![
        super::none(),
        BundValue::int(1),
        super::none(),
        BundValue::float(2.5),
        super::none(),
    ]);
    let answer = |src: &str| {
        let mut i = run_on(std::slice::from_ref(&list), None, src).unwrap();
        i.pull().unwrap()
    };
    assert_eq!(shown(&answer("na.count")), "3");
    assert_eq!(shown(&answer("na.omit")), shown(&top("[ 1 2.5 ]")));
    assert_eq!(shown(&answer("0 na.fill")), shown(&top("[ 0 1 0 2.5 0 ]")));
    for w in ["na.count", "na.omit"] {
        let e = err(&format!("3 {w}"));
        assert_eq!(
            e,
            format!("{}: E101 list is outside {w}'s domain: 3", w.to_uppercase())
        );
    }
    assert_eq!(
        err("\"x\" 0 na.fill"),
        "NA.FILL: E101 list is outside na.fill's domain: \"x\""
    );
    // JSON's NONE: one counted; `na` the same type and q.
    assert_eq!(shown(&top("'[1, null]' json json.to_value na.count")), "1");
    let j = top("'[null]' json json.to_value");
    let j = &j.as_list().unwrap()[0];
    let n = top("na");
    assert_eq!((n.dt(), j.dt()), (NONE, NONE));
    assert_eq!(n.q().to_bits(), j.q().to_bits());
    assert_eq!(n.q(), 0.0);
}

/// Criterion 1: a `.` form takes its top operand from the workbench and
/// answers there.
#[test]
fn forms_use_the_workbench() {
    let list = BundValue::list(vec![super::none(), BundValue::int(1)]);
    let mut i = run_on(
        std::slice::from_ref(&list),
        Some(&BundValue::int(9)),
        "na.fill.",
    )
    .unwrap();
    assert_eq!(shown(&i.pull_workbench().unwrap()), shown(&top("[ 9 1 ]")));
    let mut i = run_on(&[], Some(&list), "na.count.").unwrap();
    assert_eq!(shown(&i.pull_workbench().unwrap()), "1");
}

/// Criterion 9.
#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "0.5",
        "\"x\"",
        "list",
        "na",
        "[ 1 ]",
        "dict",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
    ];
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        for o in operands {
            for p in operands {
                for src in [
                    w.to_string(),
                    format!("{o} {w}"),
                    format!("{o} {p} {w}"),
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
