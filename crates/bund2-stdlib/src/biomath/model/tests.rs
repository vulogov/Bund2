//! RFC-0018 criteria 1 and 8 for M1: `model.matrix`, its refusals, and the
//! design check.

use std::collections::BTreeMap;

use bund2_api::Vm as _;
use bund2_value::BundValue;

use super::WORDS;
use super::design;
use crate::biomath::na::none;
use crate::biomath::testkit::run_on;

fn s(x: &str) -> BundValue {
    BundValue::str(x)
}

fn list(xs: Vec<BundValue>) -> BundValue {
    BundValue::list(xs)
}

fn strs(xs: &[&str]) -> BundValue {
    list(xs.iter().map(|x| s(x)).collect())
}

fn int(i: i64) -> BundValue {
    BundValue::int(i)
}

fn float(x: f64) -> BundValue {
    BundValue::float(x)
}

fn map(kv: Vec<(&str, BundValue)>) -> BundValue {
    BundValue::map(kv.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn raw(names: &[&str], cols: Vec<BundValue>, levels: Vec<(&str, BundValue)>) -> BundValue {
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

/// `bund_examples/frames/`'s dose trial, `site` a factor, and a logical
/// column `treated` added.
fn trial() -> BundValue {
    let resp = [
        Some(4.2),
        Some(5.9),
        Some(3.8),
        None,
        Some(4.6),
        Some(6.3),
        Some(7.1),
        Some(6.8),
        None,
    ];
    raw(
        &["patient", "site", "dose", "response", "treated"],
        vec![
            list((1..=9).map(int).collect()),
            strs(&[
                "north", "north", "south", "south", "east", "east", "north", "south", "east",
            ]),
            list([10, 20, 10, 20, 10, 20, 30, 30, 30].map(int).to_vec()),
            list(resp.iter().map(|r| r.map_or_else(none, float)).collect()),
            list(
                [true, false, true, false, true, false, true, false, true]
                    .map(BundValue::boolean)
                    .to_vec(),
            ),
        ],
        vec![("site", strs(&["east", "north", "south"]))],
    )
}

fn on(stack: &[BundValue], src: &str) -> Result<BundValue, String> {
    let mut i = run_on(stack, None, src)?;
    i.pull().ok_or_else(|| format!("{src}: empty"))
}

fn fails(stack: &[BundValue], src: &str) -> String {
    match on(stack, src) {
        Ok(v) => panic!("{src} answered {}", v.display()),
        Err(e) => e,
    }
}

fn omit() -> BundValue {
    map(vec![("na", s("omit"))])
}

fn matrix(f: &BundValue, terms: BundValue, opts: BundValue) -> BundValue {
    on(&[f.clone(), s("response"), terms, opts], "model.matrix").unwrap_or_else(|e| panic!("{e}"))
}

fn mm_fails(f: &BundValue, response: &str, terms: BundValue, opts: BundValue) -> String {
    fails(&[f.clone(), s(response), terms, opts], "model.matrix")
}

fn get<'a>(d: &'a BundValue, k: &str) -> &'a BundValue {
    d.as_map()
        .and_then(|m| m.get(k))
        .unwrap_or_else(|| panic!("no key {k} in {}", d.display()))
}

fn names_of(d: &BundValue) -> Vec<String> {
    get(d, "names")
        .as_list()
        .unwrap_or_default()
        .iter()
        .filter_map(BundValue::as_str)
        .collect()
}

fn rows_of(d: &BundValue) -> Vec<Vec<f64>> {
    get(d, "matrix")
        .as_matrix()
        .unwrap_or_else(|| panic!("no MATRIX"))
        .iter()
        .map(|r| {
            r.iter()
                .map(|c| match c.unboxed() {
                    BundValue::Float(x, _) => *x,
                    other => panic!("a cell {other:?}"),
                })
                .collect()
        })
        .collect()
}

fn ints_of(v: &BundValue) -> Vec<i64> {
    v.as_list()
        .unwrap_or_default()
        .iter()
        .filter_map(BundValue::as_int)
        .collect()
}

fn floats_of(v: &BundValue) -> Vec<f64> {
    crate::biomath::testkit::floats(v)
}

fn e101(op: &str, t: &str) -> String {
    format!("MODEL.MATRIX: E101 {op} is outside model.matrix's domain: {t}")
}

fn dose_site() -> BundValue {
    list(vec![s("dose"), s("site"), strs(&["dose", "site"])])
}

#[test]
fn every_word_is_registered_with_its_effect() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for w in WORDS {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        let want = if w.ends_with('.') { (3, 0) } else { (4, 1) };
        assert_eq!((e.consumes, e.produces, e.opaque), (want.0, want.1, false), "{w}");
    }
    let ours = i
        .registry
        .word_names()
        .into_iter()
        .filter(|n| n.starts_with("model."))
        .count();
    assert_eq!(ours, WORDS.len());
}

/// Criterion 1: the interaction design on the dose trial, under `"omit"`.
#[test]
fn the_trial_codes_its_interaction() {
    let d = matrix(&trial(), dose_site(), omit());
    assert_eq!(
        names_of(&d),
        [
            "(intercept)",
            "dose",
            "site.north",
            "site.south",
            "dose:site.north",
            "dose:site.south"
        ]
    );
    assert_eq!(ints_of(get(&d, "assign")), [0, 1, 2, 2, 3, 3]);
    assert_eq!(get(&d, "dropped").as_int(), Some(2));
    let want: Vec<Vec<f64>> = vec![
        vec![1.0, 10.0, 1.0, 0.0, 10.0, 0.0],
        vec![1.0, 20.0, 1.0, 0.0, 20.0, 0.0],
        vec![1.0, 10.0, 0.0, 1.0, 0.0, 10.0],
        vec![1.0, 10.0, 0.0, 0.0, 0.0, 0.0],
        vec![1.0, 20.0, 0.0, 0.0, 0.0, 0.0],
        vec![1.0, 30.0, 1.0, 0.0, 30.0, 0.0],
        vec![1.0, 30.0, 0.0, 1.0, 0.0, 30.0],
    ];
    assert_eq!(rows_of(&d), want);
    assert_eq!(
        floats_of(get(&d, "y")),
        [4.2, 5.9, 3.8, 4.6, 6.3, 7.1, 6.8]
    );
    assert_eq!(floats_of(get(&d, "weights")), [1.0; 7]);
    assert_eq!(floats_of(get(&d, "offset")), [0.0; 7]);
    let spec = get(&d, "spec");
    assert_eq!(get(spec, "response").as_str().as_deref(), Some("response"));
    assert_eq!(get(spec, "intercept").display(), "true");
    assert_eq!(
        get(get(spec, "levels"), "site").display(),
        strs(&["east", "north", "south"]).display()
    );
    assert_eq!(get(spec, "terms").display(), dose_site().display());
    assert!(get(spec, "logical").as_list().is_some_and(<[BundValue]>::is_empty));
    // The design passes its own check.
    let checked = design::check("fit.lm", "design", &d).unwrap_or_else(|e| panic!("{}", e.0));
    assert_eq!(checked.x, want);
}

/// Criterion 1, through Bund source: the CSV read, the options set.
#[test]
fn the_call_as_a_program_writes_it() {
    let src = r#"
"patient,site,dose,response
1,north,10,4.2
2,north,20,5.9
3,south,10,3.8
4,south,20,
5,east,10,4.6
6,east,20,6.3
7,north,30,7.1
8,south,30,6.8
9,east,30,NA
" dict [ "site" ] "factors" swap set frame.csv.parse
"response" [ "dose" "site" [ "dose" "site" ] ] dict "omit" "na" swap set
model.matrix "names" get
"#;
    let v = on(&[], src).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        v.display(),
        strs(&[
            "(intercept)",
            "dose",
            "site.north",
            "site.south",
            "dose:site.north",
            "dose:site.south"
        ])
        .display()
    );
}

/// Criterion 1: without an intercept, the first factor term codes every
/// level, and inside an interaction a factor keeps its contrasts.
#[test]
fn no_intercept_codes_the_first_factor_fully() {
    let no = map(vec![("na", s("omit")), ("intercept", BundValue::boolean(false))]);
    let d = matrix(&trial(), strs(&["site"]), no.clone());
    assert_eq!(names_of(&d), ["site.east", "site.north", "site.south"]);
    let d = matrix(&trial(), dose_site(), no);
    assert_eq!(
        names_of(&d),
        [
            "dose",
            "site.east",
            "site.north",
            "site.south",
            "dose:site.north",
            "dose:site.south"
        ]
    );
    assert_eq!(ints_of(get(&d, "assign")), [1, 2, 2, 2, 3, 3]);
    design::check("fit.lm", "design", &d).unwrap_or_else(|e| panic!("{}", e.0));
}

/// Criterion 1: a logical column codes one column; `spec.logical` names it.
#[test]
fn a_logical_column_codes_one() {
    let d = matrix(&trial(), strs(&["dose", "treated"]), omit());
    assert_eq!(names_of(&d), ["(intercept)", "dose", "treated.true"]);
    let col: Vec<f64> = rows_of(&d).iter().map(|r| r[2]).collect();
    assert_eq!(col, [1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0]);
    assert_eq!(
        get(get(&d, "spec"), "logical").display(),
        strs(&["treated"]).display()
    );
}

/// Criterion 1: after `frame.filter` removes every `east` row, `site` codes
/// from `north` and `south` alone.
#[test]
fn unused_levels_code_nothing() {
    let f = on(
        &[trial()],
        r#"{ "site" get "east" == not } frame.filter"#,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let d = matrix(&f, strs(&["dose", "site"]), omit());
    assert_eq!(names_of(&d), ["(intercept)", "dose", "site.south"]);
    assert_eq!(
        get(get(get(&d, "spec"), "levels"), "site").display(),
        strs(&["north", "south"]).display()
    );
}

/// Two three-level factors: R's variable order, the member whose column
/// comes first in `terms` varying fastest (D265).
#[test]
fn interactions_take_r_variable_order() {
    let a = ["1", "2", "3", "1", "2", "3", "1", "2", "3", "1"];
    let b = ["1", "1", "1", "2", "2", "2", "3", "3", "3", "2"];
    let y: Vec<BundValue> = (0..10).map(|i| float(f64::from(i))).collect();
    let f = raw(
        &["y", "a", "b"],
        vec![list(y), strs(&a), strs(&b)],
        vec![("a", strs(&["1", "2", "3"])), ("b", strs(&["1", "2", "3"]))],
    );
    let ab = list(vec![s("a"), s("b"), strs(&["a", "b"])]);
    let d = on(&[f.clone(), s("y"), ab, map(vec![])], "model.matrix").unwrap();
    assert_eq!(
        names_of(&d)[5..],
        ["a.2:b.2", "a.3:b.2", "a.2:b.3", "a.3:b.3"]
    );
    let ba = list(vec![s("b"), s("a"), strs(&["a", "b"])]);
    let d = on(&[f.clone(), s("y"), ba, map(vec![])], "model.matrix").unwrap();
    assert_eq!(
        names_of(&d)[5..],
        ["b.2:a.2", "b.3:a.2", "b.2:a.3", "b.3:a.3"]
    );
    // Row 5 is a = 2, b = 2: only b.2:a.2 is 1.
    assert_eq!(rows_of(&d)[4][5..], [1.0, 0.0, 0.0, 0.0]);
    // The order an interaction writes its members in changes nothing.
    let ab2 = list(vec![s("a"), s("b"), strs(&["b", "a"])]);
    let d2 = on(&[f, s("y"), ab2, map(vec![])], "model.matrix").unwrap();
    assert_eq!(
        names_of(&d2)[5..],
        ["a.2:b.2", "a.3:b.2", "a.2:b.3", "a.3:b.3"]
    );
}

/// Weights and offset: read as FLOATs, named in `spec`.
#[test]
fn weights_and_offset_are_read() {
    let f = raw(
        &["y", "x", "w", "o"],
        vec![
            list([1.0, 2.0, 4.0, 3.0].map(float).to_vec()),
            list([1, 2, 3, 4].map(int).to_vec()),
            list([1, 2, 1, 2].map(int).to_vec()),
            list([0.5, 0.0, 0.5, 0.0].map(float).to_vec()),
        ],
        vec![],
    );
    let o = map(vec![("weights", s("w")), ("offset", s("o"))]);
    let d = on(&[f, s("y"), strs(&["x"]), o], "model.matrix").unwrap();
    assert_eq!(floats_of(get(&d, "weights")), [1.0, 2.0, 1.0, 2.0]);
    assert_eq!(floats_of(get(&d, "offset")), [0.5, 0.0, 0.5, 0.0]);
    let spec = get(&d, "spec");
    assert_eq!(get(spec, "weights").as_str().as_deref(), Some("w"));
    assert_eq!(get(spec, "offset").as_str().as_deref(), Some("o"));
    design::check("fit.glm", "design", &d).unwrap_or_else(|e| panic!("{}", e.0));
}

/// Criterion 1: each of §S8's rows for `model.matrix`, by an input that
/// fails only that check, with its exact text.
#[test]
fn refusals_have_their_texts() {
    let t = trial();
    let none_map = map(vec![]);
    // Step 0: the frame.
    assert_eq!(
        fails(&[int(1), s("response"), strs(&[]), none_map.clone()], "model.matrix"),
        e101("frame", "1")
    );
    // Step 1: the options.
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose"]), map(vec![("zz", int(1))])),
        e101("opts", "an unknown key \"zz\"")
    );
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose"]), map(vec![("family", s("poisson"))])),
        e101("opts", "the key \"family\", which this word does not take")
    );
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose"]), map(vec![("intercept", int(1))])),
        e101("opts", "intercept is 1")
    );
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose"]), map(vec![("na", s("drop"))])),
        e101("opts", "na is \"drop\"")
    );
    // Step 2: the response, then `weights` and `offset`.
    assert_eq!(
        fails(&[t.clone(), int(3), strs(&[]), omit()], "model.matrix"),
        e101("response", "3")
    );
    assert_eq!(
        mm_fails(&t, "y", strs(&["dose"]), omit()),
        e101("response", "no column \"y\"")
    );
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose"]), map(vec![("weights", s("w"))])),
        e101("opts", "weights names no column \"w\"")
    );
    // Step 3: the terms.
    assert_eq!(
        mm_fails(&t, "response", int(1), omit()),
        e101("terms", "1")
    );
    assert_eq!(
        mm_fails(&t, "response", list(vec![s("dose"), strs(&["site"])]), omit()),
        e101("terms", &format!("item 2 is {}", strs(&["site"]).summary(40)))
    );
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose", "zz"]), omit()),
        e101("terms", "no column \"zz\"")
    );
    assert_eq!(
        mm_fails(
            &t,
            "response",
            list(vec![s("dose"), strs(&["dose", "dose"])]),
            omit()
        ),
        e101("terms", "term 2 repeats \"dose\"")
    );
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose", "site", "dose"]), omit()),
        e101("terms", "term 3 repeats term 1")
    );
    assert_eq!(
        mm_fails(
            &t,
            "response",
            list(vec![
                s("dose"),
                s("site"),
                strs(&["dose", "site"]),
                strs(&["site", "dose"])
            ]),
            omit()
        ),
        e101("terms", "term 4 repeats term 3")
    );
    assert_eq!(
        mm_fails(&t, "response", list(vec![s("dose"), strs(&["dose", "site"])]), omit()),
        e101("terms", "term 2 needs \"site\" as a term before it")
    );
    assert_eq!(
        mm_fails(
            &t,
            "response",
            strs(&[]),
            map(vec![("intercept", BundValue::boolean(false))])
        ),
        e101("terms", "no columns, with no intercept and no terms")
    );
    // Step 4: the overlaps.
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose", "response"]), omit()),
        e101("terms", "term 2 reads the response \"response\"")
    );
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose"]), map(vec![("weights", s("response"))])),
        e101("opts", "weights names the response \"response\"")
    );
    assert_eq!(
        mm_fails(
            &t,
            "response",
            strs(&["site"]),
            map(vec![("weights", s("dose")), ("offset", s("dose"))])
        ),
        e101("opts", "offset names \"dose\", as weights does")
    );
    // Step 5: each column's kind.
    assert_eq!(
        mm_fails(&t, "site", strs(&["dose"]), omit()),
        e101("response", "column \"site\" is a factor")
    );
    let g = raw(
        &["y", "g", "e"],
        vec![
            list([1.0, 2.0, 3.0].map(float).to_vec()),
            strs(&["a", "b", "c"]),
            list(vec![none(), none(), none()]),
        ],
        vec![],
    );
    assert_eq!(
        mm_fails(&g, "g", strs(&[]), map(vec![])),
        e101("response", "column \"g\" is string")
    );
    assert_eq!(
        mm_fails(&g, "y", strs(&["g"]), map(vec![])),
        e101("terms", "column \"g\" is string")
    );
    assert_eq!(
        mm_fails(&g, "y", strs(&["e"]), map(vec![])),
        e101("terms", "column \"e\" is of no kind")
    );
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose"]), map(vec![("weights", s("site"))])),
        e101("opts", "weights column \"site\" is a factor")
    );
    // Step 6: NONE under "refuse", then the weights' cells.
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose"]), map(vec![])),
        e101("frame", "column \"response\" row 4 is none (na \"refuse\")")
    );
    let wz = raw(
        &["y", "x", "w"],
        vec![
            list([1.0, 2.0, 3.0, 5.0].map(float).to_vec()),
            list([1, 2, 3, 4].map(int).to_vec()),
            list([1, 1, 1, 0].map(int).to_vec()),
        ],
        vec![],
    );
    assert_eq!(
        mm_fails(&wz, "y", strs(&["x"]), map(vec![("weights", s("w"))])),
        e101("opts", "weights row 4 is 0; frame.filter sets a row aside")
    );
    // Step 7: the rows kept.
    let empty = raw(
        &["y", "x"],
        vec![list(vec![float(1.0), none()]), list(vec![none(), int(2)])],
        vec![],
    );
    assert_eq!(
        mm_fails(&empty, "y", strs(&["x"]), omit()),
        e101("frame", "no rows")
    );
    let one = on(std::slice::from_ref(&t), r#"{ "site" get "north" == } frame.filter"#).unwrap();
    assert_eq!(
        mm_fails(&one, "response", strs(&["dose", "site"]), omit()),
        e101("terms", "term 2, \"site\", has one used level, \"north\"")
    );
    let dup = raw(
        &["y", "site.north", "site"],
        vec![
            list([1.0, 2.0, 3.0, 4.0, 5.0].map(float).to_vec()),
            list([1, 2, 3, 4, 6].map(int).to_vec()),
            strs(&["north", "south", "north", "south", "north"]),
        ],
        vec![("site", strs(&["south", "north"]))],
    );
    assert_eq!(
        mm_fails(&dup, "y", strs(&["site.north", "site"]), map(vec![])),
        e101(
            "terms",
            "column name \"site.north\" is both column \"site.north\" and \"site\" level \"north\""
        )
    );
    let few = on(std::slice::from_ref(&t), "3 frame.head").unwrap();
    assert_eq!(
        mm_fails(&few, "response", strs(&["dose", "site"]), omit()),
        e101("frame", "3 observations for 3 coefficients")
    );
}

/// The cap, counted before any cell is made (D39): 20 000 rows by the
/// intercept and a factor's 5 000 contrasts is 100 020 000 cells.
#[test]
fn the_cap_is_checked_before_the_cells() {
    let n = 20_000_usize;
    let m = 5_001_usize;
    let levels: Vec<String> = (0..m).map(|i| format!("l{i}")).collect();
    let f = raw(
        &["y", "g"],
        vec![
            list((0..n).map(|i| float(i as f64)).collect()),
            list((0..n).map(|i| s(&levels[i % m])).collect()),
        ],
        vec![("g", list(levels.iter().map(|l| s(l)).collect()))],
    );
    assert_eq!(
        fails(&[f, s("y"), strs(&["g"]), map(vec![])], "model.matrix"),
        "MODEL.MATRIX: E111 the design's cells exceeds its cap of 100000000"
    );
}

fn good() -> BundValue {
    matrix(&trial(), strs(&["dose", "site"]), omit())
}

fn with(d: &BundValue, k: &str, v: BundValue) -> BundValue {
    let mut m = d.as_map().cloned().unwrap_or_default();
    m.insert(k.to_string(), v);
    BundValue::map(m)
}

fn with_spec(d: &BundValue, k: &str, v: Option<BundValue>) -> BundValue {
    let mut sp = get(d, "spec").as_map().cloned().unwrap_or_default();
    match v {
        Some(v) => {
            sp.insert(k.to_string(), v);
        }
        None => {
            sp.remove(k);
        }
    }
    with(d, "spec", BundValue::map(sp))
}

fn design_fails(d: &BundValue) -> String {
    match design::check("fit.lm", "design", d) {
        Ok(_) => panic!("{} passed", d.display()),
        Err(e) => e.0,
    }
}

fn de101(t: &str) -> String {
    format!("FIT.LM: E101 design is outside fit.lm's domain: {t}")
}

/// §S2's design check: each invariant, by a design that breaks only it.
#[test]
fn the_design_check_enforces_its_invariants() {
    let d = good();
    design::check("fit.lm", "design", &d).unwrap_or_else(|e| panic!("{}", e.0));
    assert_eq!(design_fails(&int(1)), de101("1"));
    let mut m = d.as_map().cloned().unwrap_or_default();
    m.remove("y");
    assert_eq!(design_fails(&BundValue::map(m)), de101("a missing key \"y\""));
    assert_eq!(
        design_fails(&with(&d, "matrix", strs(&["a"]))),
        de101(&format!("matrix is {}", strs(&["a"]).summary(40)))
    );
    let rows = rows_of(&d);
    let mat = |f: &dyn Fn(&mut Vec<Vec<f64>>)| {
        let mut r = rows.clone();
        f(&mut r);
        BundValue::matrix(
            r.into_iter()
                .map(|row| row.into_iter().map(float).collect())
                .collect(),
        )
    };
    assert_eq!(
        design_fails(&with(&d, "matrix", mat(&|r| r[2][3] = f64::NAN))),
        de101("matrix row 3 column 4 is NaN")
    );
    assert_eq!(
        design_fails(&with(&d, "matrix", mat(&|r| r[4][0] = 2.0))),
        de101("matrix row 5 column 1 is 2.0, not the intercept's 1")
    );
    assert_eq!(
        design_fails(&with(&d, "y", list(vec![float(1.0)]))),
        de101("y has 1 items, not 7")
    );
    assert_eq!(
        design_fails(&with(&d, "names", strs(&["a", "b", "c", "c"]))),
        de101("names repeat \"c\"")
    );
    assert_eq!(
        design_fails(&with(&d, "assign", list([0, 2, 1, 2].map(int).to_vec()))),
        de101("assign item 3 is 1, after 2")
    );
    assert_eq!(
        design_fails(&with(&d, "assign", list([0, 0, 1, 2].map(int).to_vec()))),
        de101("assign item 2 is 0, a second intercept")
    );
    assert_eq!(
        design_fails(&with(&d, "assign", list([0, 1, 2, 3].map(int).to_vec()))),
        de101("assign item 4 is 3, above spec's 2 terms")
    );
    assert_eq!(
        design_fails(&with(&d, "assign", list([0, 1, 1, 1].map(int).to_vec()))),
        de101("assign codes no column for term 2")
    );
    assert_eq!(
        design_fails(&with_spec(&d, "intercept", Some(BundValue::boolean(false)))),
        de101("assign item 1 is 0, and spec has no intercept")
    );
    assert_eq!(
        design_fails(&with_spec(&d, "levels", None)),
        de101("spec has no key \"levels\"")
    );
    let w = |x: f64| list((0..7).map(|i| float(if i == 2 { x } else { 1.0 })).collect());
    assert_eq!(
        design_fails(&with(&d, "weights", w(0.0))),
        de101("weights row 3 is 0.0")
    );
    assert_eq!(
        design_fails(&with(&d, "weights", w(2.0))),
        de101("weights row 3 is 2.0, and spec names no weights")
    );
    let off = list((0..7).map(|i| float(if i == 5 { 0.25 } else { 0.0 })).collect());
    assert_eq!(
        design_fails(&with(&d, "offset", off)),
        de101("offset row 6 is 0.25, and spec names no offset")
    );
    let short: Vec<Vec<f64>> = rows.iter().take(4).cloned().collect();
    let four = |xs: &[f64]| list(xs.iter().copied().map(float).collect());
    let small = with(
        &with(
            &with(
                &with(
                    &with(
                        &d,
                        "matrix",
                        BundValue::matrix(
                            short
                                .into_iter()
                                .map(|r| r.into_iter().map(float).collect())
                                .collect(),
                        ),
                    ),
                    "y",
                    four(&[1.0, 2.0, 3.0, 4.0]),
                ),
                "weights",
                four(&[1.0; 4]),
            ),
            "offset",
            four(&[0.0; 4]),
        ),
        "dropped",
        int(0),
    );
    assert_eq!(design_fails(&small), de101("4 observations for 4 coefficients"));
}

/// Criterion 8: every name and form, on a palette of wrong types, empty,
/// NONE-filled and malformed frames, answers or refuses.
#[test]
fn no_operand_aborts() {
    let ok = |e: &str| e.contains(": E1") || e.contains("too shallow") || e.contains("returned");
    let palette = vec![
        float(f64::NAN),
        int(0),
        s("dose"),
        s(""),
        none(),
        list(vec![]),
        strs(&["dose"]),
        dose_site(),
        map(vec![]),
        omit(),
        trial(),
        raw(&[], vec![], vec![]),
        raw(&["a"], vec![list(vec![none(), none()])], vec![]),
        raw(&["g"], vec![strs(&["a"])], vec![("g", int(1))]),
    ];
    for a in &palette {
        for b in &palette {
            for c in &palette {
                for d in &palette {
                    if let Err(e) = on(&[a.clone(), b.clone(), c.clone(), d.clone()], "model.matrix") {
                        assert!(ok(&e), "model.matrix: {e}");
                    }
                }
                if let Err(e) =
                    run_on(&[a.clone(), b.clone()], Some(c), "model.matrix.").map(|_| ())
                {
                    assert!(ok(&e), "model.matrix.: {e}");
                }
            }
        }
    }
    for p in &palette {
        if let Err(e) = design::check("fit.lm", "design", p) {
            assert!(e.0.contains(": E101"), "{}", e.0);
        }
    }
}
