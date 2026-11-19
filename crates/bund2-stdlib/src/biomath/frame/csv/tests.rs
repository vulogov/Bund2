//! RFC-0017 criteria 1, 6, 7, 8 and 9 for M3's words.

use bund2_api::Vm as _;
use bund2_value::{BundValue, NONE};

use super::WORDS;
use crate::biomath::frame::tests::{abort_free, fails, five, list, map, on, palette, raw, s, strs};
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

fn same(got: &BundValue, want: &BundValue) {
    assert!(
        frame_eq(got, want),
        "got  {}\nwant {}",
        got.display(),
        want.display()
    );
}

/// `frame.csv.parse` of `text` under `opts`, checked by `frame.check`.
fn parse_with(text: &str, opts: BundValue) -> BundValue {
    let f = on(&[s(text), opts], "frame.csv.parse").unwrap_or_else(|e| panic!("{text:?}: {e}"));
    on(std::slice::from_ref(&f), "frame.check")
        .unwrap_or_else(|e| panic!("{text:?} parsed to a frame failing the check: {e}"));
    f
}

fn parse(text: &str) -> BundValue {
    parse_with(text, map(vec![]))
}

fn refused_with(text: &str, opts: BundValue) -> String {
    fails(&[s(text), opts], "frame.csv.parse")
}

fn refused(text: &str) -> String {
    refused_with(text, map(vec![]))
}

/// S8's E101 prefix on `text`.
fn text_e101(t: &str) -> String {
    format!("FRAME.CSV.PARSE: E101 text is outside frame.csv.parse's domain: {t}")
}

fn opts_e101(t: &str) -> String {
    format!("FRAME.CSV.PARSE: E101 opts is outside frame.csv.parse's domain: {t}")
}

fn column(f: &BundValue, name: &str) -> Vec<BundValue> {
    on(&[f.clone(), s(name)], "frame.column")
        .unwrap_or_else(|e| panic!("{name}: {e}"))
        .as_list()
        .unwrap()
        .to_vec()
}

#[test]
fn every_word_is_registered_with_its_effect() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        let consumes = if w.starts_with("frame.csv.parse") {
            2
        } else {
            1
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
}

/// Criterion 1: the `.` forms.
#[test]
fn forms_use_the_workbench() {
    let mut i = run_on(&[s("x\n1\n")], Some(&map(vec![])), "frame.csv.parse.").unwrap();
    same(
        &i.pull_workbench().unwrap(),
        &raw(&["x"], vec![list(vec![int(1)])], vec![]),
    );
    let mut i = run_on(&[], Some(&five()), "frame.text.").unwrap();
    assert!(
        i.pull_workbench()
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("dose")
    );
}

/// Criterion 6: kinds and types, NONE in place.
#[test]
fn reading_kinds() {
    let text = "id,name,flag,mix,note\n\
                1,\"NA\",true,1,\"a,b\"\n\
                2,NA,false,2.5,\"line\nbreak\"\n\
                3,,TRUE,3,\"007\"\n";
    let want = raw(
        &["id", "name", "flag", "mix", "note"],
        vec![
            list(vec![int(1), int(2), int(3)]),
            list(vec![s("NA"), none(), none()]),
            list(vec![bool_(true), bool_(false), bool_(true)]),
            list(vec![int(1), float(2.5), int(3)]),
            list(vec![s("a,b"), s("line\nbreak"), s("007")]),
        ],
        vec![],
    );
    let got = parse(text);
    same(&got, &want);
    // Each `na` field's NONE has q 0.0.
    for x in column(&got, "name").iter().filter(|x| x.dt() == NONE) {
        assert_eq!(x.q(), 0.0);
    }
    // `"NA"` kept as a STRING when `na` is `[ "" ]`.
    let got = parse_with(text, map(vec![("na", strs(&[""]))]));
    assert_eq!(
        column(&got, "name")
            .iter()
            .map(|x| x.dt() == NONE || x.as_str().as_deref() == Some("NA"))
            .collect::<Vec<_>>(),
        [true, true, true]
    );
    assert_eq!(column(&got, "name")[1].as_str().as_deref(), Some("NA"));

    // A FLOAT first at row 150 of an integer column.
    let mut t = String::from("x\n");
    for i in 1..150 {
        t.push_str(&format!("{i}\n"));
    }
    t.push_str("1.5\n");
    let x = column(&parse(&t), "x");
    assert_eq!(x.len(), 150);
    assert_eq!(x[148].unboxed(), &int(149));
    assert_eq!(x[149].unboxed(), &float(1.5));
    // A numeric column whose first 120 cells are empty.
    let t = format!("x\n{}7\n", "\n".repeat(120));
    let x = column(&parse(&t), "x");
    assert_eq!(x.len(), 121);
    assert!(x[..120].iter().all(|c| c.dt() == NONE));
    assert_eq!(x[120].unboxed(), &int(7));

    // `inf` and `ctl` read as STRINGs; `nan` makes a string column.
    let x = column(&parse("x\ninf\nctl\n"), "x");
    assert_eq!(x[0].as_str().as_deref(), Some("inf"));
    let x = column(&parse("x\n1.5\nnan\n"), "x");
    assert_eq!(x[0].as_str().as_deref(), Some("1.5"));
    // An integer beyond i64 is a STRING in a column with `A1`.
    let x = column(&parse("x\nA1\n99999999999999999999\n"), "x");
    assert_eq!(x[1].as_str().as_deref(), Some("99999999999999999999"));
    // An empty field under `na` `[ "NA" ]` is an empty STRING; so is an
    // empty line in a one-column text.
    let na = || map(vec![("na", strs(&["NA"]))]);
    let got = parse_with("x,y\n1,\n2,b\n", na());
    assert_eq!(column(&got, "y")[0].as_str().as_deref(), Some(""));
    let got = parse_with("x\n\nb\n", na());
    assert_eq!(column(&got, "x")[0].as_str().as_deref(), Some(""));
    // Under the default `na`, that empty line is NONE.
    assert_eq!(column(&parse("x\n\nb\n"), "x")[0].dt(), NONE);
}

/// Criterion 6: lines, quotes, the byte-order mark, sizes.
#[test]
fn reading_text() {
    let x = |t: &str| column(&parse(t), "x");
    // A quoted CRLF is kept as written.
    assert_eq!(x("x\n\"a\r\nb\"\n")[0].as_str().as_deref(), Some("a\r\nb"));
    // CR and CRLF terminators.
    assert_eq!(x("x\r1\r\n2\n3").len(), 3);
    // A leading byte-order mark is removed; a header named `NA` is a name.
    assert_eq!(x("\u{feff}x\n1\n").len(), 1);
    assert_eq!(column(&parse("NA\n1\n"), "NA").len(), 1);
    // Empty text, and a text of only a byte-order mark: no columns, no rows.
    let nothing = raw(&[], vec![], vec![]);
    same(&parse(""), &nothing);
    same(&parse("\u{feff}"), &nothing);
    // `"\n"`: an empty header name, or `column_1` with one NONE.
    assert_eq!(refused("\n"), text_e101("header name 1 is empty"));
    same(
        &parse_with("\n", map(vec![("header", bool_(false))])),
        &raw(&["column_1"], vec![list(vec![none()])], vec![]),
    );
    // `header` false names the columns `column_1`, …
    same(
        &parse_with("1,a\n2,b\n", map(vec![("header", bool_(false))])),
        &raw(
            &["column_1", "column_2"],
            vec![list(vec![int(1), int(2)]), strs(&["a", "b"])],
            vec![],
        ),
    );
    // `sep`.
    same(
        &parse_with("a;b\n1;2\n", map(vec![("sep", s(";"))])),
        &raw(
            &["a", "b"],
            vec![list(vec![int(1)]), list(vec![int(2)])],
            vec![],
        ),
    );
    // Quoting faults, in a data row and in the header.
    for (t, want) in [
        ("x\n\"abc\n", "a quote opened at row 1 is not closed"),
        ("x\n\"a\"b\n", "row 1 has \"b\" after a closing quote"),
        ("x\na\"b\n", "row 1 has a quote inside an unquoted field"),
        (
            "x\n1\n \"a\"\n",
            "row 2 has a quote inside an unquoted field",
        ),
        ("\"x\n", "a quote opened in the header is not closed"),
        ("\"x\"y\n", "the header has \"y\" after a closing quote"),
        ("x\"y\n", "the header has a quote inside an unquoted field"),
    ] {
        assert_eq!(refused(t), text_e101(want), "{t:?}");
    }
    // Widths: a ragged row, and an empty line in a two-column text.
    assert_eq!(
        refused("a,b\n1,2\n3\n"),
        "FRAME.CSV.PARSE: E102 row 2's fields and the header's do not agree: 1 and 2"
    );
    assert_eq!(
        refused("a,b\n1,2\n\n3,4\n"),
        "FRAME.CSV.PARSE: E102 row 2's fields and the header's do not agree: 1 and 2"
    );
    assert_eq!(
        refused_with("1,2\n3\n", map(vec![("header", bool_(false))])),
        "FRAME.CSV.PARSE: E102 row 2's fields and row 1's do not agree: 1 and 2"
    );
    // Header names: repeated, empty.
    assert_eq!(refused("x,x\n"), text_e101("a repeated header name \"x\""));
    assert_eq!(refused("x,\n"), text_e101("header name 2 is empty"));
    // Text above 10⁸ bytes.
    let big = "x".repeat(100_000_001);
    assert_eq!(
        refused(&big),
        "FRAME.CSV.PARSE: E111 text exceeds its cap of 100000000"
    );
    // Not a STRING.
    assert_eq!(
        fails(&[int(3), map(vec![])], "frame.csv.parse"),
        text_e101("3")
    );
}

/// Criterion 6: the fields the reader refuses.
#[test]
fn reading_refusals() {
    assert_eq!(
        refused("x\n1.5\nNaN\n"),
        text_e101(
            "field \"NaN\" at row 2, column \"x\" is not finite; list it in the na \
             option to read it as missing"
        )
    );
    let got = parse_with("x\n1.5\nNaN\n", map(vec![("na", strs(&["", "NaN"]))]));
    assert_eq!(column(&got, "x")[1].dt(), NONE);
    assert_eq!(
        refused("x\n1.5\n1e400\n"),
        text_e101(
            "field \"1e400\" at row 2, column \"x\" is not finite; list it in the na \
             option to read it as missing"
        )
    );
    assert_eq!(
        refused("x\n1\n99999999999999999999\n"),
        text_e101("field \"99999999999999999999\" at row 2, column \"x\" does not fit an INTEGER")
    );
    // A quoted number in a numeric column: E103, naming the column and row.
    assert_eq!(
        refused("x\n1\n\"007\"\n"),
        "FRAME.CSV.PARSE: E103 column \"x\" is not a LIST of numbers: \"007\" at row 2"
    );
}

/// Criterion 6: the options.
#[test]
fn reading_options() {
    let t = "g,n\nb,1\na,2\nb,3\n";
    assert_eq!(
        refused_with(t, map(vec![("zz", int(1))])),
        opts_e101("an unknown key \"zz\"")
    );
    assert_eq!(
        refused_with(t, map(vec![("how", s("inner"))])),
        opts_e101("the key \"how\", which this word does not take")
    );
    assert_eq!(
        refused_with(t, map(vec![("header", int(1))])),
        opts_e101("header is 1")
    );
    // The first bad key in sorted order; the column checks after them all.
    assert_eq!(
        refused_with(t, map(vec![("header", int(1)), ("sep", s(";;"))])),
        opts_e101("header is 1")
    );
    assert_eq!(
        refused_with(t, map(vec![("factors", strs(&["zz"])), ("header", int(1))])),
        opts_e101("header is 1")
    );
    assert_eq!(
        refused_with(t, map(vec![("na", list(vec![s(""), int(1)]))])),
        opts_e101("na item 2 is 1")
    );
    assert_eq!(
        refused_with(t, map(vec![("sep", s("\""))])),
        opts_e101("sep is \"\\\"\"")
    );
    assert_eq!(
        refused_with(t, map(vec![("sep", s(";;"))])),
        opts_e101("sep is \";;\"")
    );
    // `factors`: sorted levels; a name that is no column; a numeric column.
    let got = parse_with(t, map(vec![("factors", strs(&["g"]))]));
    same(
        &got,
        &raw(
            &["g", "n"],
            vec![strs(&["b", "a", "b"]), list(vec![int(1), int(2), int(3)])],
            vec![("g", strs(&["a", "b"]))],
        ),
    );
    assert_eq!(
        refused_with(t, map(vec![("factors", strs(&["zz"]))])),
        opts_e101("no column \"zz\"")
    );
    assert_eq!(
        refused_with(t, map(vec![("factors", strs(&["n"]))])),
        opts_e101("column \"n\" is numeric")
    );
}

/// Criterion 6: `frame.csv.parse` of `frame.csv.text` is the frame.
#[test]
fn round_trip() {
    let f = raw(
        &["\u{feff}id", "a,b", "flag", "g"],
        vec![
            list(vec![int(1), float(2.5), none(), float(3.0), float(1.0e-7)]),
            list(vec![s(""), s("NA"), s("007"), s("true"), none()]),
            list(vec![
                bool_(true),
                none(),
                bool_(false),
                bool_(true),
                bool_(false),
            ]),
            strs(&["x", "y", "x", "y", "x"]),
        ],
        vec![("g", strs(&["y", "x"]))],
    );
    let text = on(std::slice::from_ref(&f), "frame.csv.text").unwrap();
    let text = text.as_str().unwrap();
    assert!(
        text.starts_with("\"\u{feff}id\",\"a,b\",\"flag\",\"g\"\n"),
        "{text}"
    );
    // Factors aside: without `factors`, `g` is a string column.
    let back = parse(&text);
    let mut plain = f.as_map().unwrap().clone();
    plain.insert("levels".into(), map(vec![]));
    same(&back, &BundValue::map(plain));
    // With `factors`, levels sorted.
    let back = parse_with(&text, map(vec![("factors", strs(&["g"]))]));
    let mut sorted = f.as_map().unwrap().clone();
    sorted.insert("levels".into(), map(vec![("g", strs(&["x", "y"]))]));
    same(&back, &BundValue::map(sorted));
    // A one-column frame with NONE at its start, middle and end.
    let one = raw(
        &["x"],
        vec![list(vec![none(), int(1), none(), int(2), none()])],
        vec![],
    );
    let text = on(std::slice::from_ref(&one), "frame.csv.text").unwrap();
    assert_eq!(text.as_str().as_deref(), Some("\"x\"\n\n1\n\n2\n\n"));
    same(&parse(&text.as_str().unwrap()), &one);
    // A frame of no columns is the empty STRING, and reads back as itself.
    let nothing = raw(&[], vec![], vec![]);
    let text = on(std::slice::from_ref(&nothing), "frame.csv.text").unwrap();
    assert_eq!(text.as_str().as_deref(), Some(""));
}

/// Criterion 6: the same file through `csv` with `column` set answers that
/// column with the empty cell dropped, unchanged.
#[test]
fn csv_is_unchanged() {
    let dir = std::env::temp_dir().join(format!("bund2-frame-csv-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t.csv");
    std::fs::write(&path, "x,y\n1,a\n,b\n3,c\n").unwrap();
    let p = path.display().to_string();
    let src = format!("\"{p}\" csv :column \"x\" set :lambda {{ dup drop }} set !");
    let mut i = run_on(&[], None, &src).unwrap();
    let col = i.pull().unwrap();
    assert_eq!(col.display(), list(vec![int(1), int(3)]).display());
    let text = std::fs::read_to_string(&path).unwrap();
    let f = parse(&text);
    assert_eq!(column(&f, "x")[1].dt(), NONE);
    std::fs::remove_dir_all(&dir).ok();
}

/// Criterion 7: JSON and base64 round trips.
#[test]
fn value_round_trips() {
    let f = five();
    let src = "encode.base64 decode.base64";
    let back = on(std::slice::from_ref(&f), src).unwrap_or_else(|e| panic!("{src}: {e}"));
    same(&back, &f);
    on(&[back], "frame.check").unwrap_or_else(|e| panic!("{src}: {e}"));
    // A frame's names are STRINGs, which the reference's JSON converter
    // refuses (Note, 2026-11-19).
    let e = fails(std::slice::from_ref(&f), "json.from_value");
    assert!(
        e.contains("This Dynamic type is not supported for JSON: 4"),
        "{e}"
    );
}

fn table(f: &BundValue) -> String {
    on(std::slice::from_ref(f), "frame.text")
        .unwrap()
        .as_str()
        .unwrap()
}

/// Criterion 8: `frame.text`.
#[test]
fn text_table() {
    let f = raw(
        &["x", "group"],
        vec![
            list(vec![int(1), float(2.5), none()]),
            list(vec![s("a"), s("bb"), none()]),
        ],
        vec![("group", strs(&["a", "bb"]))],
    );
    assert_eq!(table(&f), "  x group\n  1 a\n2.5 bb\n NA NA");
    // 25 rows print 20 and `… 5 more rows`; 21 rows end `… 1 more row`.
    let n = |k: i64| raw(&["n"], vec![list((1..=k).map(int).collect())], vec![]);
    let t = table(&n(25));
    assert_eq!(t.lines().count(), 22);
    assert!(t.ends_with("\n20\n… 5 more rows"), "{t}");
    assert!(table(&n(21)).ends_with("\n… 1 more row"));
    // A frame of no columns prints the empty STRING.
    assert_eq!(table(&raw(&[], vec![], vec![])), "");
    // Padded by characters, not bytes.
    let u = raw(
        &["unit", "v"],
        vec![strs(&["µg", "kg"]), list(vec![int(1), int(2)])],
        vec![],
    );
    assert_eq!(table(&u), "unit v\nµg   1\nkg   2");
    // A last-column STRING ending in a space keeps it; an empty last cell,
    // and two, leave no separator.
    let e = raw(
        &["a", "b", "c"],
        vec![
            strs(&["x", "y", "z"]),
            strs(&["p ", "", ""]),
            strs(&["", "", "w"]),
        ],
        vec![],
    );
    assert_eq!(table(&e), "a b  c\nx p \ny\nz    w");
    let t = raw(&["a", "b"], vec![strs(&["x"]), strs(&["q "])], vec![]);
    assert_eq!(table(&t), "a b\nx q ");
}

/// Criterion 9.
#[test]
fn no_operand_aborts() {
    let mut palette = palette();
    palette.extend([
        s("x\n1\n"),
        s("\"x"),
        s("a,b\n1\n"),
        map(vec![("header", bool_(false))]),
        map(vec![("sep", s(""))]),
    ]);
    for w in WORDS.iter().filter(|w| !w.ends_with('.')) {
        abort_free(w, &palette, true);
    }
}
