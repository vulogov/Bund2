//! RFC-0016 criteria 1, 2 and 10, for §S6's rounding and formatting.

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use crate::biomath::testkit::{err, floats, run};

fn value(src: &str) -> BundValue {
    let mut i = run(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    i.pull().unwrap_or_else(|| panic!("{src}: empty"))
}

fn text(src: &str) -> String {
    let v = value(src);
    v.as_str().unwrap_or_else(|| panic!("{src}: {v:?}"))
}

// Criterion 1.

#[test]
fn every_word_and_form_is_registered() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        assert!(i.registry.effect_of(w).is_some(), "{w}");
    }
    for w in [
        "math.round.places",
        "math.signif",
        "string.fixed",
        "string.sci",
    ] {
        let plain = i.registry.effect_of(w).unwrap();
        assert_eq!(
            (plain.consumes, plain.produces, plain.opaque),
            (2, 1, false),
            "{w}"
        );
        let dot = i.registry.effect_of(&format!("{w}.")).unwrap();
        assert_eq!(
            (dot.consumes, dot.produces, dot.opaque),
            (1, 0, false),
            "{w}."
        );
    }
}

// Criterion 10.

#[test]
fn rounding_is_the_formatters_ties_to_even_on_the_binary_value() {
    assert_eq!(floats(&value("2.675 2 math.round.places")), [2.67]);
    assert_eq!(floats(&value("0.125 2 math.round.places")), [0.12]);
    assert_eq!(floats(&value("2.5 0 math.round.places")), [2.0]);
    let z = floats(&value("-0.4 0 math.round.places"))[0];
    assert_eq!(z.to_bits(), 0.0f64.to_bits(), "{z}");
    let z = floats(&value("-0.0 2 math.round.places"))[0];
    assert_eq!(z.to_bits(), 0.0f64.to_bits(), "{z}");
    assert_eq!(floats(&value("123456 2 math.signif")), [120000.0]);
    assert_eq!(floats(&value("0.000123456 3 math.signif")), [0.000123]);
    assert_eq!(floats(&value("[ \"2.5\" ] 0 math.round.places")), [2.0]);
    assert_eq!(
        floats(&value("[ 1.23456 2.5 ] 3 math.round.places")),
        [1.235, 2.5]
    );
    assert_eq!(floats(&value("7 0 math.round.places")), [7.0]);
}

#[test]
fn the_strings() {
    assert_eq!(text("3.14159 2 string.fixed"), "3.14");
    assert_eq!(text("-0.001 2 string.fixed"), "0.00");
    assert_eq!(text("12345.678 2 string.sci"), "1.23e4");
    assert_eq!(text("0.0000001 1 string.sci"), "1.0e-7");
    assert_eq!(text("-0.0 2 string.sci"), "0.00e0");
    assert_eq!(text("-2.5 0 string.fixed"), "-2");
    let v = value("[ 1.0 2.25 ] 1 string.fixed");
    let items: Vec<String> = v
        .as_list()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert_eq!(items, ["1.0", "2.2"]);
}

/// `string.sci`'s strings read back as the number with n ≥ 1 (§S6).
#[test]
fn sci_reads_back() {
    for (src, want) in [
        ("12345.678 2 string.sci", 12300.0),
        ("0.0000001 1 string.sci", 1e-7),
    ] {
        let s = text(src);
        assert_eq!(floats(&value(&s)), [want], "{src} → {s}");
    }
}

#[test]
fn the_reference_round_is_unchanged() {
    assert_eq!(floats(&value("2.5 math.round")), [3.0]);
}

#[test]
fn the_dot_forms_take_n_from_the_workbench_and_answer_there() {
    let mut i = run("2.675 2 . math.round.places.").unwrap();
    assert_eq!(i.depth(), 0);
    assert_eq!(floats(&i.pull_workbench().unwrap()), [2.67]);
    let mut i = run("3.14159 2 . string.fixed.").unwrap();
    assert_eq!(i.pull_workbench().unwrap().as_str().unwrap(), "3.14");
}

// Criterion 2.

#[test]
fn each_refusal_is_its_template() {
    assert_eq!(
        err("1.7976931348623157e308 1 math.signif"),
        "MATH.SIGNIF: E114 the answer overflows a FLOAT: 1.7976931348623157e308 to 1 significant digits"
    );
    assert_eq!(
        err("1.5 2.0 math.round.places"),
        "MATH.ROUND.PLACES: E101 n is outside math.round.places's domain: 2.0"
    );
    assert_eq!(
        err("1.5 18 math.round.places"),
        "MATH.ROUND.PLACES: E101 n is outside math.round.places's domain: 18"
    );
    assert_eq!(
        err("1.5 -1 string.fixed"),
        "STRING.FIXED: E101 n is outside string.fixed's domain: -1"
    );
    assert_eq!(
        err("1.5 0 math.signif"),
        "MATH.SIGNIF: E101 n is outside math.signif's domain: 0"
    );
    assert_eq!(
        err("1.5 17 string.sci"),
        "STRING.SCI: E101 n is outside string.sci's domain: 17"
    );
    assert_eq!(
        err("\"2.5\" 1 math.round.places"),
        "MATH.ROUND.PLACES: E103 x is not a LIST of numbers: \"2.5\""
    );
    assert_eq!(
        err("[ 1.0 \"a\" ] 1 math.round.places"),
        "MATH.ROUND.PLACES: E103 x is not a LIST of numbers: \"a\""
    );
    assert!(err("float.NaN 1 math.round.places").starts_with("MATH.ROUND.PLACES: E101 x "));
    assert!(err("1 list + float.+Inf swap + 1 string.sci").starts_with("STRING.SCI: E101 x "));
    assert!(err("1.0 math.signif").contains("too shallow"));
    assert!(matches!(value("list 2 math.round.places").as_list(), Some(v) if v.is_empty()));
}

#[test]
fn no_operand_aborts() {
    let operands = [
        "float.NaN",
        "float.+Inf",
        "-0.1",
        "1.1",
        "0",
        "1",
        "17",
        "18",
        "-1",
        "\"x\"",
        "list",
        "[ 1 ]",
        "[ 1 float.NaN ]",
        "[ [ 1 2 ] [ 3 4 ] ]",
        "[ [ 1 2 ] [ 3 4 ] ] matrix",
        "1.7976931348623157e308",
    ];
    for w in WORDS {
        for o in operands {
            for src in [
                format!("{o} {w}"),
                format!("{o} {o} {w}"),
                format!("{o} {o} . {w}"),
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
