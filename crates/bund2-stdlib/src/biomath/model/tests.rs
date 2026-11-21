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
        let consumes: u8 = match w.trim_end_matches('.') {
            "model.anova" => 2,
            "model.compare" => 3,
            "model.matrix" => 4,
            "model.predict" => 3,
            "model.residuals" => 2,
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
    assert_eq!(floats_of(get(&d, "y")), [4.2, 5.9, 3.8, 4.6, 6.3, 7.1, 6.8]);
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
    assert!(
        get(spec, "logical")
            .as_list()
            .is_some_and(<[BundValue]>::is_empty)
    );
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
    let no = map(vec![
        ("na", s("omit")),
        ("intercept", BundValue::boolean(false)),
    ]);
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
    let f = on(&[trial()], r#"{ "site" get "east" == not } frame.filter"#)
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
        fails(
            &[int(1), s("response"), strs(&[]), none_map.clone()],
            "model.matrix"
        ),
        e101("frame", "1")
    );
    // Step 1: the options.
    assert_eq!(
        mm_fails(&t, "response", strs(&["dose"]), map(vec![("zz", int(1))])),
        e101("opts", "an unknown key \"zz\"")
    );
    assert_eq!(
        mm_fails(
            &t,
            "response",
            strs(&["dose"]),
            map(vec![("family", s("poisson"))])
        ),
        e101("opts", "the key \"family\", which this word does not take")
    );
    assert_eq!(
        mm_fails(
            &t,
            "response",
            strs(&["dose"]),
            map(vec![("intercept", int(1))])
        ),
        e101("opts", "intercept is 1")
    );
    assert_eq!(
        mm_fails(
            &t,
            "response",
            strs(&["dose"]),
            map(vec![("na", s("drop"))])
        ),
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
        mm_fails(
            &t,
            "response",
            strs(&["dose"]),
            map(vec![("weights", s("w"))])
        ),
        e101("opts", "weights names no column \"w\"")
    );
    // Step 3: the terms.
    assert_eq!(mm_fails(&t, "response", int(1), omit()), e101("terms", "1"));
    assert_eq!(
        mm_fails(
            &t,
            "response",
            list(vec![s("dose"), strs(&["site"])]),
            omit()
        ),
        e101(
            "terms",
            &format!("item 2 is {}", strs(&["site"]).summary(40))
        )
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
        mm_fails(
            &t,
            "response",
            list(vec![s("dose"), strs(&["dose", "site"])]),
            omit()
        ),
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
        mm_fails(
            &t,
            "response",
            strs(&["dose"]),
            map(vec![("weights", s("response"))])
        ),
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
        mm_fails(
            &t,
            "response",
            strs(&["dose"]),
            map(vec![("weights", s("site"))])
        ),
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
    let one = on(
        std::slice::from_ref(&t),
        r#"{ "site" get "north" == } frame.filter"#,
    )
    .unwrap();
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
    assert_eq!(
        design_fails(&BundValue::map(m)),
        de101("a missing key \"y\"")
    );
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
    let w = |x: f64| {
        list(
            (0..7)
                .map(|i| float(if i == 2 { x } else { 1.0 }))
                .collect(),
        )
    };
    assert_eq!(
        design_fails(&with(&d, "weights", w(0.0))),
        de101("weights row 3 is 0.0")
    );
    assert_eq!(
        design_fails(&with(&d, "weights", w(2.0))),
        de101("weights row 3 is 2.0, and spec names no weights")
    );
    let off = list(
        (0..7)
            .map(|i| float(if i == 5 { 0.25 } else { 0.0 }))
            .collect(),
    );
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
    assert_eq!(
        design_fails(&small),
        de101("4 observations for 4 coefficients")
    );
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
                    if let Err(e) = on(
                        &[a.clone(), b.clone(), c.clone(), d.clone()],
                        "model.matrix",
                    ) {
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

// RFC-0018 criterion 9: `stat.lm`'s and `fit.logistic`'s answers, every
// key and every element of each LIST, as f64 bits, recorded from the code
// before M2 factored `stat.lm`'s computation out (8424124). It differs from
// `infer.snapshot` only in recording every element, not each answer's
// first, so every slope's bits are checked as well as the intercept's.

const LM_SNAPSHOT: &str = include_str!("../fixtures/lm.snapshot");

/// `stat.lm`'s and `fit.logistic`'s answered test programs
/// (`infer/tests.rs`, `lm_against_the_fixture` and `the_forms`;
/// `fit/tests.rs`, `logistic_against_glm`).
fn lm_programs() -> Vec<(&'static str, String)> {
    const MPG: [f64; 32] = [
        21.0, 21.0, 22.8, 21.4, 18.7, 18.1, 14.3, 24.4, 22.8, 19.2, 17.8, 16.4, 17.3, 15.2, 10.4,
        10.4, 14.7, 32.4, 30.4, 33.9, 21.5, 15.5, 15.2, 13.3, 19.2, 27.3, 26.0, 30.4, 15.8, 19.7,
        15.0, 21.4,
    ];
    const VS: [u8; 32] = [
        0, 0, 1, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0,
        0, 1,
    ];
    let rows: Vec<String> = MPG.iter().map(|x| format!("[ {x:?} ]")).collect();
    let vs: Vec<String> = VS.iter().map(|x| format!("{x}")).collect();
    vec![
        (
            "lm.fixture",
            "[ [ 1 2.5 ] [ 2 1.0 ] [ 3 4.0 ] [ 4 3.5 ] [ 5 6.0 ] [ 6 5.0 ] [ 7 8.5 ] \
             [ 8 7.0 ] ] matrix [ 3.1 4.9 7.2 8.8 11.1 13.2 14.8 17.1 ] stat.lm"
                .to_string(),
        ),
        (
            "lm.forms",
            "[ [ 1 ] [ 2 ] [ 3 ] [ 4 ] ] matrix [ 1.0 2.5 2.5 4.5 ] stat.lm".to_string(),
        ),
        (
            "logistic.mtcars",
            format!(
                "[ {} ] matrix [ {} ] fit.logistic",
                rows.join(" "),
                vs.join(" ")
            ),
        ),
    ]
}

/// Each answer as lines `label key index bits`, every element of a LIST.
fn lm_snapshot() -> String {
    let mut out = String::new();
    for (label, src) in lm_programs() {
        let v = on(&[], &src).unwrap_or_else(|e| panic!("{src}: {e}"));
        let m = v.as_map().cloned().unwrap_or_default();
        for (k, x) in m {
            for (i, f) in floats_of(&x).iter().enumerate() {
                out.push_str(&format!("{label} {k} {i} {:016x}\n", f.to_bits()));
            }
        }
    }
    out
}

#[test]
fn stat_lm_and_fit_logistic_answer_the_snapshot_bit_for_bit() {
    assert_eq!(lm_snapshot(), LM_SNAPSHOT);
}

/// Writes the snapshot. Run once, before M2's first change, with
/// `cargo test -p bund2-stdlib write_the_lm_snapshot -- --ignored`.
#[test]
#[ignore = "writes the snapshot; run only to record it"]
fn write_the_lm_snapshot() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/biomath/fixtures/lm.snapshot"
    );
    std::fs::write(path, lm_snapshot()).unwrap_or_else(|e| panic!("{e}"));
}

// RFC-0018 criterion 2: `fit.lm`.

const MODEL_FIXTURE: &str = include_str!("../fixtures/model.txt");

fn fit_lm(d: &BundValue) -> BundValue {
    on(&[d.clone(), map(vec![])], "fit.lm").unwrap_or_else(|e| panic!("{e}"))
}

fn bits(v: &BundValue) -> Vec<u64> {
    floats_of(v).iter().map(|x| x.to_bits()).collect()
}

/// |a − b| ≤ r|b| + 10⁻¹⁰, RFC-0018's tolerance, r = 10⁻⁹ for `fit.lm`.
fn near(what: &str, got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{what}: {got:?} against {want:?}");
    for (g, w) in got.iter().zip(want) {
        assert!(
            (g - w).abs() <= 1e-9 * w.abs() + 1e-10,
            "{what}: {got:?} against {want:?}"
        );
    }
}

/// The dose trial's seven complete rows, numeric columns only, with
/// weights and an offset made up for the test (`fixtures/model.py`).
fn dose_frame() -> BundValue {
    raw(
        &["y", "dose", "patient", "w", "off"],
        vec![
            list([4.2, 5.9, 3.8, 4.6, 6.3, 7.1, 6.8].map(float).to_vec()),
            list([10, 20, 10, 10, 20, 30, 30].map(int).to_vec()),
            list([1, 2, 3, 5, 6, 7, 8].map(int).to_vec()),
            list([1, 2, 1, 3, 1, 2, 1].map(int).to_vec()),
            list([0.1, 0.0, 0.2, 0.0, 0.1, 0.0, 0.3].map(float).to_vec()),
        ],
        vec![],
    )
}

fn dose_design(terms: &[&str], opts: Vec<(&str, BundValue)>) -> BundValue {
    on(
        &[dose_frame(), s("y"), strs(terms), map(opts)],
        "model.matrix",
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn fit_lm_is_registered_beside_fit_logistic() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for (w, want) in [("fit.lm", (2, 1)), ("fit.lm.", (1, 0))] {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        assert_eq!(
            (e.consumes, e.produces, e.opaque),
            (want.0, want.1, false),
            "{w}"
        );
    }
    assert!(crate::biomath::fit::WORDS.contains(&"fit.lm"));
}

/// Criterion 2: on `stat.lm`'s own test data, `coef`, `se`, `t`, `p` and
/// `r2` bit for bit, `df.residual` equal in value to `df`.
#[test]
fn fit_lm_answers_stat_lm_bit_for_bit() {
    let x1 = [1, 2, 3, 4, 5, 6, 7, 8];
    let x2 = [2.5, 1.0, 4.0, 3.5, 6.0, 5.0, 8.5, 7.0];
    let y = [3.1, 4.9, 7.2, 8.8, 11.1, 13.2, 14.8, 17.1];
    let f = raw(
        &["y", "x1", "x2"],
        vec![
            list(y.map(float).to_vec()),
            list(x1.map(int).to_vec()),
            list(x2.map(float).to_vec()),
        ],
        vec![],
    );
    let d = on(
        &[f, s("y"), strs(&["x1", "x2"]), map(vec![])],
        "model.matrix",
    )
    .unwrap();
    let fit = fit_lm(&d);
    let lm = on(
        &[],
        "[ [ 1 2.5 ] [ 2 1.0 ] [ 3 4.0 ] [ 4 3.5 ] [ 5 6.0 ] [ 6 5.0 ] [ 7 8.5 ] \
         [ 8 7.0 ] ] matrix [ 3.1 4.9 7.2 8.8 11.1 13.2 14.8 17.1 ] stat.lm",
    )
    .unwrap();
    for k in ["coef", "se", "t", "p", "r2"] {
        assert_eq!(bits(get(&fit, k)), bits(get(&lm, k)), "{k}");
    }
    assert_eq!(
        get(&fit, "df.residual").as_int().map(|d| d as f64),
        floats_of(get(&lm, "df")).first().copied()
    );
}

/// Criterion 2: on criterion 1's design without factors, the same.
#[test]
fn fit_lm_answers_stat_lm_on_the_trial() {
    let d = matrix(&trial(), strs(&["dose", "patient"]), omit());
    let fit = fit_lm(&d);
    let rows = [
        (10, 1, 4.2),
        (20, 2, 5.9),
        (10, 3, 3.8),
        (10, 5, 4.6),
        (20, 6, 6.3),
        (30, 7, 7.1),
        (30, 8, 6.8),
    ];
    let x: Vec<String> = rows
        .iter()
        .map(|(d, p, _)| format!("[ {d} {p} ]"))
        .collect();
    let y: Vec<String> = rows.iter().map(|(_, _, y)| format!("{y:?}")).collect();
    let lm = on(
        &[],
        &format!("[ {} ] matrix [ {} ] stat.lm", x.join(" "), y.join(" ")),
    )
    .unwrap();
    for k in ["coef", "se", "t", "p", "r2"] {
        assert_eq!(bits(get(&fit, k)), bits(get(&lm, k)), "{k}");
    }
}

/// Criterion 2: weighted and offset fits, against the fixture.
#[test]
fn fit_lm_against_the_fixture() {
    let w = || ("weights", s("w"));
    let o = || ("offset", s("off"));
    let no = || ("intercept", BundValue::boolean(false));
    let cases: Vec<(&str, BundValue)> = vec![
        ("lm.plain", dose_design(&["dose", "patient"], vec![])),
        ("lm.weighted", dose_design(&["dose", "patient"], vec![w()])),
        (
            "lm.offset",
            dose_design(&["dose", "patient"], vec![w(), o()]),
        ),
        (
            "lm.origin",
            dose_design(&["dose", "patient"], vec![w(), o(), no()]),
        ),
        ("lm.mean", dose_design(&[], vec![w(), o()])),
    ];
    for (label, d) in cases {
        let fit = fit_lm(&d);
        let mut keys = vec![
            "coef",
            "se",
            "t",
            "p",
            "sigma",
            "deviance",
            "null.deviance",
            "aic",
            "bic",
            "vcov",
            "hat",
            "fitted",
        ];
        if label != "lm.mean" {
            keys.extend(["r2", "adj.r2", "f", "f.p"]);
        }
        for k in keys {
            near(
                &format!("{label} {k}"),
                &floats_of(get(&fit, k)),
                &crate::biomath::testkit::fixture(MODEL_FIXTURE, &format!("{label} {k}")),
            );
        }
        let n = 7;
        let kk = names_of(&d).len() as i64;
        let i = i64::from(label != "lm.origin");
        assert_eq!(get(&fit, "df.residual").as_int(), Some(n - kk), "{label}");
        assert_eq!(get(&fit, "df.null").as_int(), Some(n - i), "{label}");
        assert_eq!(get(&fit, "family").display(), "gaussian");
        assert_eq!(get(&fit, "link").display(), "identity");
        assert_eq!(get(&fit, "spec").display(), get(&d, "spec").display());
        assert_eq!(bits(get(&fit, "y")), bits(get(&d, "y")));
        let sigma = floats_of(get(&fit, "sigma"))[0];
        near(
            "dispersion",
            &floats_of(get(&fit, "dispersion")),
            &[sigma * sigma],
        );
    }
}

/// Criterion 2: the intercept-only design answers `r2` and `adj.r2` 0 and
/// no `f`, `f.df` or `f.p` key.
#[test]
fn fit_lm_intercept_only_has_no_f() {
    let fit = fit_lm(&dose_design(&[], vec![]));
    assert_eq!(floats_of(get(&fit, "r2")), [0.0]);
    assert_eq!(floats_of(get(&fit, "adj.r2")), [0.0]);
    let m = fit.as_map().cloned().unwrap_or_default();
    for k in ["f", "f.df", "f.p"] {
        assert!(!m.contains_key(k), "{k}");
    }
    let fit = fit_lm(&dose_design(&["dose"], vec![]));
    assert_eq!(
        get(&fit, "f.df").display(),
        list(vec![int(1), int(5)]).display()
    );
}

fn lm_fails(d: &BundValue, opts: BundValue) -> String {
    fails(&[d.clone(), opts], "fit.lm")
}

fn lm_e101(op: &str, t: &str) -> String {
    format!("FIT.LM: E101 {op} is outside fit.lm's domain: {t}")
}

/// Criterion 2 and §S8's rows for `fit.lm`: a constant `y` is fitted
/// without an intercept and refused with one; singular, perfect and
/// option refusals with their texts.
#[test]
fn fit_lm_refusals() {
    let f = |y: [f64; 5], x2: [f64; 5]| {
        raw(
            &["y", "x", "x2"],
            vec![
                list(y.map(float).to_vec()),
                list([1.0, 2.0, 3.0, 4.0, 6.0].map(float).to_vec()),
                list(x2.map(float).to_vec()),
            ],
            vec![],
        )
    };
    let design = |frame: BundValue, terms: &[&str], opts: Vec<(&str, BundValue)>| {
        on(&[frame, s("y"), strs(terms), map(opts)], "model.matrix")
            .unwrap_or_else(|e| panic!("{e}"))
    };
    let constant = f([2.0; 5], [1.0, 0.0, 1.0, 0.0, 1.0]);
    assert_eq!(
        lm_fails(&design(constant.clone(), &["x"], vec![]), map(vec![])),
        lm_e101("design", "a constant series")
    );
    let origin = design(
        constant,
        &["x"],
        vec![("intercept", BundValue::boolean(false))],
    );
    let fit = fit_lm(&origin);
    assert!(floats_of(get(&fit, "coef"))[0].is_finite());
    let dependent = f([1.0, 3.0, 2.0, 5.0, 4.0], [2.0, 4.0, 6.0, 8.0, 12.0]);
    assert_eq!(
        lm_fails(&design(dependent, &["x", "x2"], vec![]), map(vec![])),
        "FIT.LM: E106 the matrix is singular: column \"x2\" depends on those before it"
    );
    // A perfect fit is refused only when RSS is exactly 0, as `stat.lm`'s
    // is: y = 3x with x the first unit vector, no intercept, whose QR is
    // exact.
    let perfect = raw(
        &["y", "x"],
        vec![
            list([3.0, 0.0, 0.0, 0.0, 0.0].map(float).to_vec()),
            list([1.0, 0.0, 0.0, 0.0, 0.0].map(float).to_vec()),
        ],
        vec![],
    );
    assert_eq!(
        lm_fails(
            &design(
                perfect,
                &["x"],
                vec![("intercept", BundValue::boolean(false))]
            ),
            map(vec![])
        ),
        lm_e101("design", "a perfect fit, its residual variance 0")
    );
    let d = dose_design(&["dose"], vec![]);
    assert_eq!(
        lm_fails(&d, map(vec![("family", s("gaussian"))])),
        lm_e101("opts", "the key \"family\", which this word does not take")
    );
    assert_eq!(
        lm_fails(&d, map(vec![("zz", int(1))])),
        lm_e101("opts", "an unknown key \"zz\"")
    );
    assert_eq!(lm_fails(&d, int(1)), lm_e101("opts", "1"));
    // The design is checked before the options.
    assert_eq!(
        lm_fails(&int(2), map(vec![("zz", int(1))])),
        lm_e101("design", "2")
    );
    assert_eq!(
        lm_fails(&with(&d, "dropped", s("x")), map(vec![])),
        lm_e101("design", "dropped is \"x\"")
    );
    // The `.` form takes `opts` from the workbench and answers there.
    let mut i = run_on(std::slice::from_ref(&d), Some(&map(vec![])), "fit.lm.").unwrap();
    let wb = i.pull_workbench().unwrap();
    assert_eq!(bits(get(&wb, "coef")), bits(get(&fit_lm(&d), "coef")));
}

/// Criterion 8: `fit.lm` on malformed designs answers or refuses.
#[test]
fn fit_lm_never_aborts() {
    let ok = |e: &str| e.contains(": E1") || e.contains("too shallow");
    let d = dose_design(&["dose", "patient"], vec![]);
    let mut designs = vec![
        d.clone(),
        int(0),
        map(vec![]),
        with(&d, "matrix", BundValue::matrix(vec![])),
        with(&d, "y", list(vec![])),
        with(&d, "names", list(vec![])),
        with(&d, "assign", list(vec![int(9)])),
        with(&d, "weights", list([f64::NAN; 7].map(float).to_vec())),
        with_spec(&d, "terms", Some(int(1))),
        with_spec(&d, "offset", None),
        good(),
    ];
    let big: Vec<Vec<BundValue>> = (0..7)
        .map(|r| vec![float(1.0), float(f64::from(r) * 1e300), float(1e-300)])
        .collect();
    designs.push(with(&d, "matrix", BundValue::matrix(big)));
    for x in &designs {
        for o in [map(vec![]), int(1), omit()] {
            if let Err(e) = on(&[x.clone(), o.clone()], "fit.lm") {
                assert!(ok(&e), "fit.lm: {e}");
            }
            if let Err(e) = run_on(std::slice::from_ref(x), Some(&o), "fit.lm.").map(|_| ()) {
                assert!(ok(&e), "fit.lm.: {e}");
            }
        }
    }
}

// RFC-0018 criterion 3: `fit.glm`.

fn fl(xs: &[f64]) -> BundValue {
    list(xs.iter().copied().map(float).collect())
}

fn xs(n: usize) -> BundValue {
    list((1..=n).map(|i| int(i as i64)).collect())
}

/// A design from columns `y` and `x`, and those of `extra`.
fn glm_design(
    y: &[f64],
    extra: Vec<(&str, BundValue)>,
    terms: &[&str],
    opts: Vec<(&str, BundValue)>,
) -> BundValue {
    let mut names = vec!["y"];
    let mut cols = vec![fl(y)];
    let mut levels = vec![];
    for (n, c) in extra {
        names.push(n);
        if let Some(rest) = n.strip_prefix('f') {
            let _ = rest;
            levels.push((n, strs(&["1", "2", "3"])));
        }
        cols.push(c);
    }
    let f = raw(&names, cols, levels);
    on(&[f, s("y"), strs(terms), map(opts)], "model.matrix").unwrap_or_else(|e| panic!("{e}"))
}

fn glm_opts(family: &str, link: Option<&str>) -> BundValue {
    let mut kv = vec![("family", s(family))];
    if let Some(l) = link {
        kv.push(("link", s(l)));
    }
    map(kv)
}

fn fit_glm(d: &BundValue, family: &str, link: Option<&str>) -> Result<BundValue, String> {
    on(&[d.clone(), glm_opts(family, link)], "fit.glm")
}

/// |a − b| ≤ 10⁻⁶|b| + 10⁻¹⁰, RFC-0018's IRLS tolerance; a p below 10⁻⁶
/// is compared to an absolute 10⁻¹².
fn near_irls(what: &str, got: &[f64], want: &[f64], p: bool) {
    assert_eq!(got.len(), want.len(), "{what}: {got:?} against {want:?}");
    for (g, w) in got.iter().zip(want) {
        let ok = if p && w.abs() < 1e-6 {
            (g - w).abs() <= 1e-12
        } else {
            (g - w).abs() <= 1e-6 * w.abs() + 1e-10
        };
        assert!(ok, "{what}: {got:?} against {want:?}");
    }
}

fn want(label: &str, key: &str) -> Vec<f64> {
    crate::biomath::testkit::fixture(MODEL_FIXTURE, &format!("{label} {key}"))
}

fn against(label: &str, fit: &BundValue, estimated: bool) {
    let stat = if estimated { "t" } else { "z" };
    let mut keys = vec![
        "coef",
        "se",
        stat,
        "p",
        "deviance",
        "null.deviance",
        "dispersion",
        "fitted",
    ];
    if MODEL_FIXTURE.contains(&format!("{label} aic ")) {
        keys.extend(["aic", "bic"]);
    }
    for k in keys {
        near_irls(
            &format!("{label} {k}"),
            &floats_of(get(fit, k)),
            &want(label, k),
            k == "p",
        );
    }
    for k in ["iterations", "halvings"] {
        assert_eq!(
            get(fit, k).as_int().map(|i| i as f64),
            want(label, k).first().copied(),
            "{label} {k}"
        );
    }
    let m = fit.as_map().cloned().unwrap_or_default();
    assert!(
        !m.contains_key(if estimated { "z" } else { "t" }),
        "{label}"
    );
    for k in ["sigma", "r2", "adj.r2", "f", "f.df", "f.p"] {
        assert!(!m.contains_key(k), "{label} {k}");
    }
}

const POS: [f64; 8] = [2.1, 3.4, 1.9, 5.6, 4.2, 6.8, 7.5, 3.3];

#[test]
fn fit_glm_is_registered_beside_fit_logistic() {
    let mut i = bund2_interp::Interp::new();
    crate::register_all(&mut i.registry);
    for (w, want) in [("fit.glm", (2, 1)), ("fit.glm.", (1, 0))] {
        let e = i.registry.effect_of(w).unwrap_or_else(|| panic!("{w}"));
        assert_eq!(
            (e.consumes, e.produces, e.opaque),
            (want.0, want.1, false),
            "{w}"
        );
    }
    assert!(crate::biomath::fit::WORDS.contains(&"fit.glm"));
}

/// Criterion 3: Dobson's trial, R's own example (`glm.Rd:335-340`).
#[test]
fn fit_glm_dobson() {
    let counts = [18.0, 17.0, 15.0, 20.0, 10.0, 20.0, 25.0, 13.0, 12.0];
    let d = glm_design(
        &counts,
        vec![
            (
                "foutcome",
                strs(&["1", "2", "3", "1", "2", "3", "1", "2", "3"]),
            ),
            (
                "ftreatment",
                strs(&["1", "1", "1", "2", "2", "2", "3", "3", "3"]),
            ),
        ],
        &["foutcome", "ftreatment"],
        vec![],
    );
    let fit = fit_glm(&d, "poisson", None).unwrap();
    against("glm.dobson", &fit, false);
    assert_eq!(get(&fit, "link").display(), "log");
    assert_eq!(get(&fit, "df.residual").as_int(), Some(4));
    assert_eq!(get(&fit, "df.null").as_int(), Some(8));
}

/// Criterion 3: a rate model, its null deviance the refitted
/// intercept-plus-offset model's.
#[test]
fn fit_glm_rate_with_offset() {
    let y = [2.0, 3.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 15.0];
    let expo = [10.0_f64, 17.0, 26.0, 34.0, 40.0, 43.0, 50.0, 60.0, 75.0];
    let d = glm_design(
        &y,
        vec![("x", xs(9)), ("off", fl(&expo.map(f64::ln)))],
        &["x"],
        vec![("offset", s("off"))],
    );
    against("glm.rate", &fit_glm(&d, "poisson", None).unwrap(), false);
}

/// Criterion 3: proportions with their trials as weights, three links.
#[test]
fn fit_glm_binomial_proportions() {
    let trials = [10.0, 12.0, 15.0, 9.0, 20.0, 11.0];
    let succ = [1.0, 3.0, 6.0, 5.0, 15.0, 10.0];
    let prop: Vec<f64> = succ.iter().zip(&trials).map(|(s, t)| s / t).collect();
    let d = glm_design(
        &prop,
        vec![("x", xs(6)), ("w", fl(&trials))],
        &["x"],
        vec![("weights", s("w"))],
    );
    for link in ["logit", "probit", "cloglog"] {
        let fit = fit_glm(&d, "binomial", Some(link)).unwrap();
        against(&format!("glm.binomial.{link}"), &fit, false);
    }
}

/// Criterion 3: Gamma under inverse and log, the inverse Gaussian under
/// 1/mu^2, the Gaussian under log.
#[test]
fn fit_glm_positive_families() {
    let d = glm_design(&POS, vec![("x", xs(8))], &["x"], vec![]);
    for (label, family, link) in [
        ("glm.gamma.inverse", "gamma", None),
        ("glm.gamma.log", "gamma", Some("log")),
        ("glm.ig", "inverse.gaussian", None),
        ("glm.gaussian.log", "gaussian", Some("log")),
    ] {
        let fit = fit_glm(&d, family, link).unwrap();
        against(label, &fit, true);
    }
}

/// Criterion 3: step halving, under `identity` and under `sqrt`.
#[test]
fn fit_glm_halves_a_step() {
    let y = [1.0, 1.0, 2.0, 0.0, 6.0, 2.0, 0.0, 6.0];
    let d = glm_design(&y, vec![("x", xs(8))], &["x"], vec![]);
    let fit = fit_glm(&d, "poisson", Some("identity")).unwrap();
    against("glm.halving", &fit, false);
    assert_eq!(get(&fit, "iterations").as_int(), Some(7));
    assert_eq!(get(&fit, "halvings").as_int(), Some(1));
    // The fixture's search: η ≤ 0 halved once, every μ inside the range.
    let y = [2.0, 0.0, 0.0, 7.0, 20.0, 20.0];
    assert!(MODEL_FIXTURE.contains("glm.sqrt.y [2 0 0 7 20 20]"));
    let d = glm_design(&y, vec![("x", xs(6))], &["x"], vec![]);
    let fit = fit_glm(&d, "poisson", Some("sqrt")).unwrap();
    against("glm.sqrt", &fit, false);
    assert_eq!(get(&fit, "halvings").as_int(), Some(1));
}

/// Criterion 3: the zero group is answered, its indicator's coefficient
/// below −20; with a tiny exposure, rates numerically 0 are refused.
#[test]
fn fit_glm_zero_counts() {
    let d = glm_design(
        &[3.0, 5.0, 4.0, 0.0, 0.0, 0.0],
        vec![("g", fl(&[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]))],
        &["g"],
        vec![],
    );
    let fit = fit_glm(&d, "poisson", None).unwrap();
    against("glm.zero", &fit, false);
    assert!(floats_of(get(&fit, "coef"))[1] < -20.0);
    let d = glm_design(
        &[3.0, 5.0, 4.0, 0.0],
        vec![("off", fl(&[0.0, 0.0, 0.0, 1e-16_f64.ln()]))],
        &[],
        vec![("offset", s("off"))],
    );
    assert_eq!(
        fit_glm(&d, "poisson", None).unwrap_err(),
        "FIT.GLM: E101 design is outside fit.glm's domain: fitted rate numerically 0 at row 4"
    );
}

/// Criterion 3: the binomial logit on 0/1 data agrees with `fit.logistic`
/// to a relative 10⁻¹² (§S4, K15).
#[test]
fn fit_glm_agrees_with_fit_logistic() {
    const MPG: [f64; 32] = [
        21.0, 21.0, 22.8, 21.4, 18.7, 18.1, 14.3, 24.4, 22.8, 19.2, 17.8, 16.4, 17.3, 15.2, 10.4,
        10.4, 14.7, 32.4, 30.4, 33.9, 21.5, 15.5, 15.2, 13.3, 19.2, 27.3, 26.0, 30.4, 15.8, 19.7,
        15.0, 21.4,
    ];
    const VS: [f64; 32] = [
        0., 0., 1., 1., 0., 1., 0., 1., 1., 1., 1., 0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 0., 0.,
        0., 0., 1., 0., 1., 0., 0., 0., 1.,
    ];
    let d = glm_design(&VS, vec![("mpg", fl(&MPG))], &["mpg"], vec![]);
    let fit = fit_glm(&d, "binomial", None).unwrap();
    let rows: Vec<String> = MPG.iter().map(|x| format!("[ {x:?} ]")).collect();
    let vs: Vec<String> = VS.iter().map(|x| format!("{}", *x as i64)).collect();
    let lr = on(
        &[],
        &format!(
            "[ {} ] matrix [ {} ] fit.logistic",
            rows.join(" "),
            vs.join(" ")
        ),
    )
    .unwrap();
    for (k, l) in [
        ("coef", "coef"),
        ("se", "se"),
        ("z", "z"),
        ("deviance", "deviance"),
        ("aic", "aic"),
    ] {
        let (g, w) = (floats_of(get(&fit, k)), floats_of(get(&lr, l)));
        assert_eq!(g.len(), w.len(), "{k}");
        for (a, b) in g.iter().zip(&w) {
            assert!((a - b).abs() <= 1e-12 * b.abs(), "{k}: {g:?} against {w:?}");
        }
    }
}

/// Criterion 2: a Gaussian identity `fit.glm` gives `fit.lm`'s AIC to 10⁻⁹.
#[test]
fn fit_glm_gaussian_aic_is_fit_lm_aic() {
    let d = dose_design(&["dose", "patient"], vec![("weights", s("w"))]);
    let g = fit_glm(&d, "gaussian", None).unwrap();
    let l = fit_lm(&d);
    let (a, b) = (floats_of(get(&g, "aic"))[0], floats_of(get(&l, "aic"))[0]);
    assert!((a - b).abs() <= 1e-9 * b.abs(), "{a} against {b}");
}

/// Criterion 3: a non-integer Poisson `y` is fitted with no `aic` or `bic`.
#[test]
fn fit_glm_non_integer_poisson_has_no_aic() {
    let d = glm_design(
        &[1.5, 2.0, 3.25, 4.0, 6.5, 7.0],
        vec![("x", xs(6))],
        &["x"],
        vec![],
    );
    let fit = fit_glm(&d, "poisson", None).unwrap();
    let m = fit.as_map().cloned().unwrap_or_default();
    assert!(!m.contains_key("aic") && !m.contains_key("bic"));
    assert!(m.contains_key("deviance") && m.contains_key("null.deviance"));
}

fn glm_e101(op: &str, t: &str) -> String {
    format!("FIT.GLM: E101 {op} is outside fit.glm's domain: {t}")
}

/// Criterion 3: §S8's rows for `fit.glm`, with their texts.
#[test]
fn fit_glm_refusals() {
    let d = glm_design(
        &[1.0, 0.0, 3.0, 2.0, 5.0],
        vec![("x", xs(5))],
        &["x"],
        vec![],
    );
    let o = |kv: Vec<(&str, BundValue)>| map(kv);
    let refuse = |d: &BundValue, opts: BundValue| fails(&[d.clone(), opts], "fit.glm");
    assert_eq!(
        refuse(&d, o(vec![])),
        glm_e101("opts", "family is required")
    );
    assert_eq!(
        refuse(&d, o(vec![("family", s("normal"))])),
        glm_e101("opts", "family is \"normal\", not a family")
    );
    assert_eq!(
        refuse(&d, o(vec![("family", s("binomial")), ("link", s("sqrt"))])),
        glm_e101("opts", "link is \"sqrt\", not a binomial link")
    );
    assert_eq!(
        refuse(&d, o(vec![("family", s("poisson")), ("na", s("omit"))])),
        glm_e101("opts", "the key \"na\", which this word does not take")
    );
    // A response outside its family's range.
    let neg = glm_design(
        &[1.0, -1.0, 3.0, 2.0, 5.0],
        vec![("x", xs(5))],
        &["x"],
        vec![],
    );
    assert_eq!(
        fit_glm(&neg, "poisson", None).unwrap_err(),
        glm_e101("design", "y row 2 is -1.0, below the poisson family's 0")
    );
    assert_eq!(
        fit_glm(&neg, "binomial", None).unwrap_err(),
        glm_e101(
            "design",
            "y row 2 is -1.0, outside the binomial family's [0, 1]"
        )
    );
    assert_eq!(
        fit_glm(&d, "gamma", None).unwrap_err(),
        glm_e101("design", "y row 2 is 0.0, not above the gamma family's 0")
    );
    // Binomial counts and trials.
    let w = |y: &[f64], w: &[f64]| {
        glm_design(
            y,
            vec![("x", xs(5)), ("w", fl(w))],
            &["x"],
            vec![("weights", s("w"))],
        )
    };
    assert_eq!(
        fit_glm(
            &w(&[0.5, 0.5, 0.5, 0.5, 0.5], &[2.0, 2.0, 2.5, 2.0, 2.0]),
            "binomial",
            None
        )
        .unwrap_err(),
        glm_e101("design", "weights row 3 is 2.5, not a count of trials")
    );
    assert_eq!(
        fit_glm(&w(&[0.5, 0.5, 0.25, 0.5, 0.5], &[2.0; 5]), "binomial", None).unwrap_err(),
        glm_e101("design", "y row 3 times its weight is 0.5, not a count")
    );
    // A start with a non-finite η.
    let tiny = glm_design(
        &[1.0, 1e-200, 3.0, 2.0, 5.0],
        vec![("x", xs(5))],
        &["x"],
        vec![],
    );
    assert_eq!(
        fit_glm(&tiny, "inverse.gaussian", None).unwrap_err(),
        glm_e101(
            "design",
            "y row 2 is 1e-200, whose starting η under the 1/mu^2 link is not finite"
        )
    );
    // The first IRLS step out of range.
    let first = glm_design(
        &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        vec![("x", xs(6))],
        &["x"],
        vec![],
    );
    assert!(MODEL_FIXTURE.contains("glm.first.y [0 0 0 1 1 1]"));
    assert_eq!(
        fit_glm(&first, "poisson", Some("identity")).unwrap_err(),
        glm_e101(
            "design",
            "the first IRLS step leaves the poisson family's valid range, with no earlier \
             coefficients to halve towards"
        )
    );
    // Separation, under logit and under probit.
    let sep = glm_design(
        &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        vec![("x", xs(6))],
        &["x"],
        vec![],
    );
    for link in ["logit", "probit"] {
        assert_eq!(
            fit_glm(&sep, "binomial", Some(link)).unwrap_err(),
            glm_e101(
                "design",
                "fitted probabilities numerically 0 or 1: X separates y"
            ),
            "{link}"
        );
    }
    // A singular design: an interaction with an empty combination of levels.
    let f = raw(
        &["y", "a", "b"],
        vec![
            fl(&[1.0, 2.0, 3.0, 2.0, 4.0, 3.0, 5.0]),
            strs(&["p", "q", "p", "q", "p", "q", "p"]),
            strs(&["u", "u", "v", "u", "v", "u", "v"]),
        ],
        vec![("a", strs(&["p", "q"])), ("b", strs(&["u", "v"]))],
    );
    let sing = on(
        &[
            f,
            s("y"),
            list(vec![s("a"), s("b"), strs(&["a", "b"])]),
            map(vec![]),
        ],
        "model.matrix",
    )
    .unwrap();
    assert_eq!(
        fit_glm(&sing, "poisson", None).unwrap_err(),
        "FIT.GLM: E106 the matrix is singular: column \"a.q:b.v\" depends on those before it"
    );
    // A perfect fit under an estimated dispersion.
    let perfect = raw(
        &["y", "x"],
        vec![
            fl(&[3.0, 0.0, 0.0, 0.0, 0.0]),
            fl(&[1.0, 0.0, 0.0, 0.0, 0.0]),
        ],
        vec![],
    );
    let pd = on(
        &[
            perfect,
            s("y"),
            strs(&["x"]),
            map(vec![("intercept", BundValue::boolean(false))]),
        ],
        "model.matrix",
    )
    .unwrap();
    assert_eq!(
        fit_glm(&pd, "gaussian", None).unwrap_err(),
        glm_e101("design", "a perfect fit, its residual variance 0")
    );
    // IRLS past its cap: a Poisson sqrt fit converging linearly towards 0.
    let slow = glm_design(
        &[1.0, 0.0, 0.0, 0.0, 1.0, 8.0],
        vec![("x", xs(6))],
        &["x"],
        vec![],
    );
    assert_eq!(
        fit_glm(&slow, "poisson", Some("sqrt")).unwrap_err(),
        "FIT.GLM: E110 IRLS did not converge within 100 iterations"
    );
    // The design is checked before the options.
    assert_eq!(refuse(&int(1), o(vec![])), glm_e101("design", "1"));
    // The `.` form takes `opts` from the workbench and answers there.
    let good = glm_design(&POS, vec![("x", xs(8))], &["x"], vec![]);
    let mut i = run_on(
        std::slice::from_ref(&good),
        Some(&glm_opts("gamma", None)),
        "fit.glm.",
    )
    .unwrap();
    let wb = i.pull_workbench().unwrap();
    assert_eq!(
        bits(get(&wb, "coef")),
        bits(get(&fit_glm(&good, "gamma", None).unwrap(), "coef"))
    );
}

/// Criterion 8: `fit.glm` on malformed designs and options answers or
/// refuses, every family and link.
#[test]
fn fit_glm_never_aborts() {
    let ok = |e: &str| e.contains(": E1") || e.contains("too shallow");
    let base = glm_design(&POS, vec![("x", xs(8))], &["x"], vec![]);
    let zeros = glm_design(&[0.0; 8], vec![("x", xs(8))], &["x"], vec![]);
    let ones = glm_design(&[1.0; 8], vec![("x", xs(8))], &["x"], vec![]);
    let huge = glm_design(
        &[1e300, 1e-300, 1.0, 2.0, 1e300, 3.0, 1e-300, 4.0],
        vec![("x", xs(8))],
        &["x"],
        vec![],
    );
    let wide = with(&base, "matrix", {
        let rows: Vec<Vec<BundValue>> = (0..8)
            .map(|r| vec![float(1.0), float(f64::from(r) * 1e200)])
            .collect();
        BundValue::matrix(rows)
    });
    let families: [(&str, &[&str]); 5] = [
        (
            "binomial",
            &["logit", "probit", "cloglog", "cauchit", "log"],
        ),
        ("poisson", &["log", "identity", "sqrt"]),
        ("gaussian", &["identity", "log", "inverse"]),
        ("gamma", &["inverse", "log", "identity"]),
        (
            "inverse.gaussian",
            &["1/mu^2", "inverse", "log", "identity"],
        ),
    ];
    for d in [&base, &zeros, &ones, &huge, &wide, &good(), &int(3)] {
        for (family, links) in families {
            for link in links {
                if let Err(e) = fit_glm(d, family, Some(link)) {
                    assert!(ok(&e), "{family} {link}: {e}");
                }
            }
        }
    }
}

// RFC-0018 criterion 4: residuals and influence.

fn dobson_design() -> BundValue {
    glm_design(
        &[18.0, 17.0, 15.0, 20.0, 10.0, 20.0, 25.0, 13.0, 12.0],
        vec![
            (
                "foutcome",
                strs(&["1", "2", "3", "1", "2", "3", "1", "2", "3"]),
            ),
            (
                "ftreatment",
                strs(&["1", "1", "1", "2", "2", "2", "3", "3", "3"]),
            ),
        ],
        &["foutcome", "ftreatment"],
        vec![],
    )
}

fn residuals_of(fit: &BundValue, t: &str) -> Result<BundValue, String> {
    on(&[fit.clone(), map(vec![("type", s(t))])], "model.residuals")
}

#[test]
fn model_residuals_and_influence_against_the_fixture() {
    let gamma = glm_design(&POS, vec![("x", xs(8))], &["x"], vec![]);
    let cases = [
        (
            "glm.dobson",
            fit_glm(&dobson_design(), "poisson", None).unwrap(),
        ),
        ("glm.gamma.inverse", fit_glm(&gamma, "gamma", None).unwrap()),
        (
            "lm.weighted",
            fit_lm(&dose_design(
                &["dose", "patient"],
                vec![("weights", s("w"))],
            )),
        ),
    ];
    for (label, fit) in &cases {
        for t in ["response", "pearson", "deviance", "working", "standardized"] {
            let r = residuals_of(fit, t).unwrap_or_else(|e| panic!("{e}"));
            near_irls(
                &format!("{label} {t}"),
                &floats_of(&r),
                &want(label, &format!("resid.{t}")),
                false,
            );
        }
        // The default is "deviance".
        let d = on(&[fit.clone(), map(vec![])], "model.residuals").unwrap();
        assert_eq!(bits(&d), bits(&residuals_of(fit, "deviance").unwrap()));
        let inf = on(std::slice::from_ref(fit), "model.influence").unwrap();
        for (k, f) in [
            ("hat", "hat"),
            ("std.residuals", "standardized"),
            ("cooks", "cooks"),
        ] {
            near_irls(
                &format!("{label} {k}"),
                &floats_of(get(&inf, k)),
                &want(label, &format!("resid.{f}")),
                false,
            );
        }
    }
    // For an unweighted fit.lm the deviance residual is y − μ.
    let plain = fit_lm(&dose_design(&["dose", "patient"], vec![]));
    let d = floats_of(&residuals_of(&plain, "deviance").unwrap());
    let r = floats_of(&residuals_of(&plain, "response").unwrap());
    near("y − μ", &d, &r);
}

/// Criterion 4: a factor level with one row is refused by
/// `model.influence` and `"standardized"`, and its other types answered.
#[test]
fn a_leverage_of_one_is_refused() {
    let f = raw(
        &["y", "x", "g"],
        vec![
            fl(&[1.0, 2.0, 2.5, 4.0, 5.5, 6.0, 9.0]),
            xs(7),
            strs(&["a", "a", "a", "b", "b", "b", "c"]),
        ],
        vec![("g", strs(&["a", "b", "c"]))],
    );
    let d = on(&[f, s("y"), strs(&["x", "g"]), map(vec![])], "model.matrix").unwrap();
    let fit = fit_lm(&d);
    assert_eq!(
        on(std::slice::from_ref(&fit), "model.influence").unwrap_err(),
        "MODEL.INFLUENCE: E114 the answer overflows a FLOAT: the standardized residual at row 7, \
         whose leverage is 1"
    );
    assert_eq!(
        residuals_of(&fit, "standardized").unwrap_err(),
        "MODEL.RESIDUALS: E114 the answer overflows a FLOAT: the standardized residual at row 7, \
         whose leverage is 1"
    );
    for t in ["response", "pearson", "deviance", "working"] {
        residuals_of(&fit, t).unwrap_or_else(|e| panic!("{t}: {e}"));
    }
    assert_eq!(
        residuals_of(&fit, "raw").unwrap_err(),
        "MODEL.RESIDUALS: E101 opts is outside model.residuals's domain: type is \"raw\""
    );
}

// RFC-0018 criterion 5: prediction.

fn predict_fails(fit: &BundValue, frame: BundValue, opts: Vec<(&str, BundValue)>) -> String {
    fails(&[fit.clone(), frame, map(opts)], "model.predict")
}

fn predict_on(fit: &BundValue, frame: BundValue, opts: Vec<(&str, BundValue)>) -> BundValue {
    on(&[fit.clone(), frame, map(opts)], "model.predict").unwrap_or_else(|e| panic!("{e}"))
}

fn check_predict(
    label: &str,
    fit: &BundValue,
    frame: &BundValue,
    prediction: bool,
    w: Option<&str>,
) {
    for (scale, prefix) in [("link", "link"), ("response", "response")] {
        let mut o = vec![("type", s(scale)), ("interval", s("confidence"))];
        if let Some(w) = w {
            o.push(("weights", s(w)));
        }
        let p = predict_on(fit, frame.clone(), o);
        for (k, f) in [
            ("fit", "fit"),
            ("se", "se"),
            ("lower", "lower"),
            ("upper", "upper"),
        ] {
            near_irls(
                &format!("{label} {prefix} {k}"),
                &floats_of(get(&p, k)),
                &want(label, &format!("predict.{prefix}.{f}")),
                false,
            );
        }
        assert_eq!(get(&p, "dropped").as_int(), Some(0));
    }
    if prediction {
        let mut o = vec![("type", s("link")), ("interval", s("prediction"))];
        if let Some(w) = w {
            o.push(("weights", s(w)));
        }
        let p = predict_on(fit, frame.clone(), o);
        for k in ["lower", "upper"] {
            near_irls(
                &format!("{label} prediction {k}"),
                &floats_of(get(&p, k)),
                &want(label, &format!("predict.pred.{k}")),
                false,
            );
        }
    }
}

#[test]
fn model_predict_against_the_fixture() {
    // Dobson at (1, 1), (2, 3), (3, 2).
    let dobson = fit_glm(&dobson_design(), "poisson", None).unwrap();
    let new = raw(
        &["foutcome", "ftreatment"],
        vec![strs(&["1", "2", "3"]), strs(&["1", "3", "2"])],
        vec![
            ("foutcome", strs(&["1", "2", "3"])),
            ("ftreatment", strs(&["1", "2", "3"])),
        ],
    );
    check_predict("glm.dobson", &dobson, &new, false, None);
    // The Gamma under inverse: every se positive, every lower below upper.
    let gamma = fit_glm(
        &glm_design(&POS, vec![("x", xs(8))], &["x"], vec![]),
        "gamma",
        None,
    )
    .unwrap();
    let gx = raw(&["x"], vec![fl(&[1.5, 4.5, 7.0])], vec![]);
    check_predict("glm.gamma.inverse", &gamma, &gx, false, None);
    let p = predict_on(&gamma, gx, vec![("interval", s("confidence"))]);
    let (lo, hi) = (floats_of(get(&p, "lower")), floats_of(get(&p, "upper")));
    assert!(floats_of(get(&p, "se")).iter().all(|s| *s > 0.0));
    assert!(lo.iter().zip(&hi).all(|(a, b)| a < b));
    // The Gaussian and weighted fits, prediction intervals too, the
    // weighted fit's with and without weights for the new rows.
    let lx = raw(
        &["dose", "patient", "nw"],
        vec![
            fl(&[15.0, 25.0, 5.0]),
            fl(&[4.0, 9.0, 1.0]),
            fl(&[2.0, 1.0, 3.0]),
        ],
        vec![],
    );
    let plain = fit_lm(&dose_design(&["dose", "patient"], vec![]));
    check_predict("lm.plain", &plain, &lx, true, None);
    let weighted = fit_lm(&dose_design(
        &["dose", "patient"],
        vec![("weights", s("w"))],
    ));
    check_predict("lm.weighted", &weighted, &lx, true, None);
    check_predict("lm.weighted.w", &weighted, &lx, true, Some("nw"));
}

/// Criterion 5: the zero group's response-scale interval answers no
/// `upper` key and its `lower`, ε through the clamped `log` inverse.
#[test]
fn model_predict_leaves_an_infinite_end_out() {
    let zero = fit_glm(
        &glm_design(
            &[3.0, 5.0, 4.0, 0.0, 0.0, 0.0],
            vec![("g", fl(&[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]))],
            &["g"],
            vec![],
        ),
        "poisson",
        None,
    )
    .unwrap();
    let p = predict_on(
        &zero,
        raw(&["g"], vec![fl(&[1.0])], vec![]),
        vec![("interval", s("confidence"))],
    );
    let m = p.as_map().cloned().unwrap_or_default();
    assert!(!m.contains_key("upper"));
    assert_eq!(floats_of(get(&p, "lower")), [f64::EPSILON]);
    assert_eq!(f64::EPSILON, 2.220446049250313e-16);
}

fn pe101(op: &str, t: &str) -> String {
    format!("MODEL.PREDICT: E101 {op} is outside model.predict's domain: {t}")
}

/// Criterion 5: one new row of a factor design, its `site` a single level,
/// is answered; an unseen level, a prediction interval on a Poisson fit,
/// and an inverse-link interval across 0 are refused with their texts.
#[test]
fn model_predict_refusals() {
    let d = matrix(&trial(), dose_site(), omit());
    let fit = fit_lm(&d);
    let one = raw(
        &["dose", "site"],
        vec![list(vec![int(20)]), strs(&["south"])],
        vec![("site", strs(&["east", "north", "south"]))],
    );
    let p = predict_on(&fit, one, vec![]);
    assert_eq!(floats_of(get(&p, "fit")).len(), 1);
    let west = raw(
        &["dose", "site"],
        vec![list(vec![int(20)]), strs(&["west"])],
        vec![("site", strs(&["west"]))],
    );
    assert_eq!(
        predict_fails(&fit, west, vec![]),
        pe101(
            "frame",
            "column \"site\" row 1 is \"west\", not one of the fit's levels"
        )
    );
    assert_eq!(
        predict_fails(
            &fit,
            raw(
                &["site"],
                vec![strs(&["east"])],
                vec![("site", strs(&["east"]))]
            ),
            vec![]
        ),
        pe101("frame", "no column \"dose\"")
    );
    let text = raw(
        &["dose", "site"],
        vec![list(vec![int(1)]), strs(&["east"])],
        vec![],
    );
    assert_eq!(
        predict_fails(&fit, text, vec![]),
        pe101(
            "frame",
            "column \"site\" is string, where the fit coded a factor"
        )
    );
    let dobson = fit_glm(&dobson_design(), "poisson", None).unwrap();
    assert_eq!(
        predict_fails(
            &dobson,
            raw(&["x"], vec![fl(&[1.0])], vec![]),
            vec![("interval", s("prediction"))]
        ),
        pe101(
            "opts",
            "interval \"prediction\" needs a gaussian identity fit"
        )
    );
    // A Gamma inverse fit, a new row whose link-scale interval contains 0.
    let gamma = fit_glm(
        &glm_design(&POS, vec![("x", xs(8))], &["x"], vec![]),
        "gamma",
        None,
    )
    .unwrap();
    let far = raw(&["x"], vec![fl(&[60.0])], vec![]);
    let link = predict_on(
        &gamma,
        far.clone(),
        vec![("type", s("link")), ("interval", s("confidence"))],
    );
    let (lo, hi) = (
        floats_of(get(&link, "lower"))[0],
        floats_of(get(&link, "upper"))[0],
    );
    assert!(lo < 0.0 && hi > 0.0, "{lo} {hi}");
    assert_eq!(
        predict_fails(&gamma, far, vec![("interval", s("confidence"))]),
        pe101(
            "frame",
            "row 1's interval crosses 0 under the inverse link; type \"link\" answers it"
        )
    );
    // A fit whose spec codes other columns than its names.
    let mut m = fit.as_map().cloned().unwrap_or_default();
    m.insert("names".into(), strs(&["a", "b", "c", "d", "e", "f"]));
    assert_eq!(
        predict_fails(&BundValue::map(m), raw(&[], vec![], vec![]), vec![]),
        "MODEL.PREDICT: E101 fit is outside model.predict's domain: spec codes column 1 as \
         \"(intercept)\", names has \"a\""
    );
    // A fit whose lengths disagree.
    let mut m = fit.as_map().cloned().unwrap_or_default();
    m.insert("se".into(), fl(&[1.0]));
    assert_eq!(
        predict_fails(&BundValue::map(m), raw(&[], vec![], vec![]), vec![]),
        "MODEL.PREDICT: E101 fit is outside model.predict's domain: se has 1 items, where coef has 6"
    );
    // NONE under "refuse", and under "omit" counted.
    let gaps = raw(
        &["dose", "site"],
        vec![list(vec![int(20), none()]), strs(&["south", "east"])],
        vec![("site", strs(&["east", "north", "south"]))],
    );
    assert_eq!(
        predict_fails(&fit, gaps.clone(), vec![]),
        pe101("frame", "column \"dose\" row 2 is none (na \"refuse\")")
    );
    let p = predict_on(&fit, gaps, vec![("na", s("omit"))]);
    assert_eq!(get(&p, "dropped").as_int(), Some(1));
    assert_eq!(
        predict_fails(&fit, raw(&[], vec![], vec![]), vec![("conf", int(1))]),
        pe101("opts", "conf is 1")
    );
}

/// Criterion 8: the M4 words on malformed fits and frames answer or
/// refuse, every name and form.
#[test]
fn m4_words_never_abort() {
    let ok = |e: &str| e.contains(": E1") || e.contains("too shallow") || e.contains("returned");
    let fit = fit_lm(&dose_design(&["dose", "patient"], vec![]));
    let mut m = fit.as_map().cloned().unwrap_or_default();
    m.insert("vcov".into(), list(vec![fl(&[1.0])]));
    let bad_vcov = BundValue::map(m);
    let mut m = fit.as_map().cloned().unwrap_or_default();
    m.insert("hat".into(), fl(&[1.0; 7]));
    let unit_hat = BundValue::map(m);
    let fits = [
        fit.clone(),
        bad_vcov,
        unit_hat,
        int(1),
        map(vec![]),
        trial(),
    ];
    let others = [
        map(vec![]),
        map(vec![("type", s("link"))]),
        map(vec![("interval", s("confidence"))]),
        dose_frame(),
        trial(),
        int(2),
        none(),
    ];
    for f in &fits {
        for w in ["model.influence", "model.influence."] {
            let r = if w.ends_with('.') {
                run_on(&[], Some(f), w).map(|_| ())
            } else {
                on(std::slice::from_ref(f), w).map(|_| ())
            };
            if let Err(e) = r {
                assert!(ok(&e), "{w}: {e}");
            }
        }
        for a in &others {
            if let Err(e) = on(&[f.clone(), a.clone()], "model.residuals") {
                assert!(ok(&e), "model.residuals: {e}");
            }
            for b in &others {
                if let Err(e) = on(&[f.clone(), a.clone(), b.clone()], "model.predict") {
                    assert!(ok(&e), "model.predict: {e}");
                }
                if let Err(e) =
                    run_on(&[f.clone(), a.clone()], Some(b), "model.predict.").map(|_| ())
                {
                    assert!(ok(&e), "model.predict.: {e}");
                }
            }
        }
    }
}

// RFC-0018 criterion 6: comparison.

fn compare(small: &BundValue, big: &BundValue, opts: BundValue) -> Result<BundValue, String> {
    on(&[small.clone(), big.clone(), opts], "model.compare")
}

fn anova(d: &BundValue, opts: Vec<(&str, BundValue)>) -> Result<BundValue, String> {
    on(&[d.clone(), map(opts)], "model.anova")
}

fn scalar(v: &BundValue, k: &str) -> f64 {
    match get(v, k).unboxed() {
        BundValue::Float(x, _) => *x,
        other => panic!("{k} is {other:?}"),
    }
}

/// A column of a frame.
fn column(f: &BundValue, c: &str) -> Vec<BundValue> {
    get(get(f, "data"), c)
        .as_list()
        .unwrap_or_default()
        .to_vec()
}

/// A numeric column's cells after its first, which is NONE.
fn after_none(f: &BundValue, c: &str) -> Vec<f64> {
    let cells = column(f, c);
    assert_eq!(
        cells.first().map(BundValue::dt),
        Some(bund2_value::NONE),
        "{c}"
    );
    crate::biomath::testkit::floats(&list(cells[1..].to_vec()))
}

fn dobson_outcome_design() -> BundValue {
    glm_design(
        &[18.0, 17.0, 15.0, 20.0, 10.0, 20.0, 25.0, 13.0, 12.0],
        vec![
            (
                "foutcome",
                strs(&["1", "2", "3", "1", "2", "3", "1", "2", "3"]),
            ),
            (
                "ftreatment",
                strs(&["1", "1", "1", "2", "2", "2", "3", "3", "3"]),
            ),
        ],
        &["foutcome"],
        vec![],
    )
}

const Z8: [f64; 8] = [0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0];

fn gamma_design(terms: &[&str], opts: Vec<(&str, BundValue)>) -> BundValue {
    glm_design(&POS, vec![("x", xs(8)), ("z", fl(&Z8))], terms, opts)
}

fn mc_e101(op: &str, t: &str) -> String {
    format!("MODEL.COMPARE: E101 {op} is outside model.compare's domain: {t}")
}

fn ma_e101(op: &str, t: &str) -> String {
    format!("MODEL.ANOVA: E101 {op} is outside model.anova's domain: {t}")
}

fn shown(x: f64) -> String {
    float(x).summary(crate::biomath::operand::WIDTH)
}

/// Criterion 6: Dobson's model without and with `treatment`, LR by
/// default; the treatment adds nothing, so 0, 0 and 1.
#[test]
fn model_compare_dobson_is_lr() {
    let small = fit_glm(&dobson_outcome_design(), "poisson", None).unwrap();
    let big = fit_glm(&dobson_design(), "poisson", None).unwrap();
    let r = compare(&small, &big, map(vec![])).unwrap();
    assert_eq!(get(&r, "test").as_str().as_deref(), Some("LR"));
    assert_eq!(get(&r, "df").as_int(), Some(2));
    assert!(r.as_map().is_some_and(|m| !m.contains_key("df2")));
    for k in ["deviance", "statistic", "p"] {
        near_irls(k, &[scalar(&r, k)], &want("compare.dobson", k), k == "p");
    }
}

/// Criterion 6: two Gamma fits, F by default on the larger model's
/// residual df; LR when asked, scaled by its dispersion.
#[test]
fn model_compare_gamma_is_f() {
    let small = fit_glm(&gamma_design(&[], vec![]), "gamma", None).unwrap();
    let big = fit_glm(&gamma_design(&["x"], vec![]), "gamma", None).unwrap();
    let r = compare(&small, &big, map(vec![])).unwrap();
    assert_eq!(get(&r, "test").as_str().as_deref(), Some("F"));
    assert_eq!(get(&r, "df").as_int(), Some(1));
    assert_eq!(get(&r, "df2").as_int(), Some(6));
    for k in ["deviance", "statistic", "p"] {
        near_irls(k, &[scalar(&r, k)], &want("compare.gamma", k), k == "p");
    }
    let lr = compare(&small, &big, map(vec![("test", s("LR"))])).unwrap();
    assert_eq!(get(&lr, "test").as_str().as_deref(), Some("LR"));
    assert!(lr.as_map().is_some_and(|m| !m.contains_key("df2")));
    let stat = scalar(&lr, "deviance") / scalar(&big, "dispersion");
    assert_eq!(scalar(&lr, "statistic"), stat);
    let p = crate::biomath::dist::Family::ChiSq { df: 1.0 }
        .tails("t", stat)
        .unwrap()
        .1;
    assert_eq!(scalar(&lr, "p"), p);
}

/// A `fit.lm` fit and a `fit.glm` Gaussian identity fit compare, and the
/// F agrees with `model.anova`'s last row and with `fit.lm`'s own t².
#[test]
fn model_compare_takes_fit_lm_and_fit_glm() {
    let small = fit_lm(&dose_design(&["dose"], vec![]));
    let big_d = dose_design(&["dose", "patient"], vec![]);
    let big = fit_lm(&big_d);
    let big_glm = fit_glm(&big_d, "gaussian", None).unwrap();
    let a = compare(&small, &big, map(vec![])).unwrap();
    let b = compare(&small, &big_glm, map(vec![])).unwrap();
    for k in ["deviance", "statistic", "p"] {
        near(k, &[scalar(&a, k)], &[scalar(&b, k)]);
    }
    let f = anova(&big_d, vec![("family", s("gaussian"))]).unwrap();
    near(
        "statistic",
        &[scalar(&a, "statistic")],
        &after_none(&f, "statistic")[1..],
    );
    let t = floats_of(get(&big, "t"))[2];
    near("t²", &[scalar(&a, "statistic")], &[t * t]);
}

/// Criterion 6: `model.anova` of the Dobson design, and without its
/// intercept.
#[test]
fn model_anova_dobson() {
    let f = anova(&dobson_design(), vec![("family", s("poisson"))]).unwrap();
    let terms: Vec<String> = column(&f, "term")
        .iter()
        .filter_map(BundValue::as_str)
        .collect();
    assert_eq!(terms, ["(intercept)", "foutcome", "ftreatment"]);
    assert!(crate::biomath::frame::check::check("t", "frame", &f).is_ok());
    assert_eq!(
        get(&f, "columns").display(),
        strs(&[
            "term",
            "df",
            "deviance",
            "resid.df",
            "resid.deviance",
            "statistic",
            "p"
        ])
        .display()
    );
    assert_eq!(ints_of(&list(column(&f, "resid.df"))), [8, 6, 4]);
    assert_eq!(ints_of(&list(column(&f, "df")[1..].to_vec())), [2, 2]);
    let rd = crate::biomath::testkit::floats(&list(column(&f, "resid.deviance")));
    near_irls(
        "resid.deviance",
        &rd,
        &want("anova.dobson", "resid.deviance"),
        false,
    );
    for k in ["deviance", "statistic", "p"] {
        near_irls(k, &after_none(&f, k), &want("anova.dobson", k), k == "p");
    }
    // The first row's resid.deviance is fit.glm's null deviance.
    let fit = fit_glm(&dobson_design(), "poisson", None).unwrap();
    assert_eq!(rd[0], scalar(&fit, "null.deviance"));
    assert_eq!(rd[2], scalar(&fit, "deviance"));

    let no = dobson_design_without_intercept();
    let f = anova(&no, vec![("family", s("poisson"))]).unwrap();
    let terms: Vec<String> = column(&f, "term")
        .iter()
        .filter_map(BundValue::as_str)
        .collect();
    assert_eq!(terms, ["NULL", "foutcome", "ftreatment"]);
    assert_eq!(ints_of(&list(column(&f, "resid.df"))), [9, 6, 4]);
    assert_eq!(ints_of(&list(column(&f, "df")[1..].to_vec())), [3, 2]);
    let rd = crate::biomath::testkit::floats(&list(column(&f, "resid.deviance")));
    near_irls(
        "resid.deviance",
        &rd,
        &want("anova.dobson0", "resid.deviance"),
        false,
    );
    for k in ["deviance", "statistic", "p"] {
        near_irls(k, &after_none(&f, k), &want("anova.dobson0", k), k == "p");
    }
}

fn dobson_design_without_intercept() -> BundValue {
    glm_design(
        &[18.0, 17.0, 15.0, 20.0, 10.0, 20.0, 25.0, 13.0, 12.0],
        vec![
            (
                "foutcome",
                strs(&["1", "2", "3", "1", "2", "3", "1", "2", "3"]),
            ),
            (
                "ftreatment",
                strs(&["1", "1", "1", "2", "2", "2", "3", "3", "3"]),
            ),
        ],
        &["foutcome", "ftreatment"],
        vec![("intercept", BundValue::boolean(false))],
    )
}

/// Criterion 6: a Gamma design's F on the full model's dispersion and
/// residual df; and a Gaussian identity design by least squares.
#[test]
fn model_anova_gamma_and_gaussian() {
    let f = anova(
        &gamma_design(&["x", "z"], vec![]),
        vec![("family", s("gamma"))],
    )
    .unwrap();
    assert_eq!(ints_of(&list(column(&f, "resid.df"))), [7, 6, 5]);
    let rd = crate::biomath::testkit::floats(&list(column(&f, "resid.deviance")));
    near_irls(
        "resid.deviance",
        &rd,
        &want("anova.gamma", "resid.deviance"),
        false,
    );
    for k in ["deviance", "statistic", "p"] {
        near_irls(k, &after_none(&f, k), &want("anova.gamma", k), k == "p");
    }
    let full = fit_glm(&gamma_design(&["x", "z"], vec![]), "gamma", None).unwrap();
    let st = after_none(&f, "statistic");
    let dv = after_none(&f, "deviance");
    assert_eq!(st[0], dv[0] / scalar(&full, "dispersion"));

    let d = dose_design(&["dose", "patient"], vec![]);
    let f = anova(&d, vec![("family", s("gaussian"))]).unwrap();
    assert_eq!(ints_of(&list(column(&f, "resid.df"))), [6, 5, 4]);
    let rd = crate::biomath::testkit::floats(&list(column(&f, "resid.deviance")));
    near("resid.deviance", &rd, &want("anova.lm", "resid.deviance"));
    for k in ["deviance", "statistic", "p"] {
        near(k, &after_none(&f, k), &want("anova.lm", k));
    }
    let fit = fit_lm(&d);
    assert_eq!(rd[0], scalar(&fit, "null.deviance"));
    assert_eq!(rd[2], scalar(&fit, "deviance"));
}

/// Criterion 6: §S8's rows for the two words, with their texts.
#[test]
fn model_compare_and_anova_refusals() {
    let gx = gamma_design(&["x"], vec![]);
    let big = fit_glm(&gx, "gamma", None).unwrap();
    let null = gamma_design(&[], vec![]);
    let cases = [
        (
            fit_glm(&null, "gaussian", None).unwrap(),
            "family is \"gaussian\", where big's is \"gamma\"".to_string(),
        ),
        (
            fit_glm(&null, "gamma", Some("log")).unwrap(),
            "link is \"log\", where big's is \"inverse\"".to_string(),
        ),
        (
            with(
                &fit_glm(&null, "gamma", None).unwrap(),
                "y",
                fl(&[9.0, 3.4, 1.9, 5.6, 4.2, 6.8, 7.5, 3.3]),
            ),
            format!("y row 1 is {}, where big's is {}", shown(9.0), shown(2.1)),
        ),
        (
            with(
                &fit_glm(&null, "gamma", None).unwrap(),
                "weights",
                fl(&[2.0; 8]),
            ),
            format!(
                "weights row 1 is {}, where big's is {}",
                shown(2.0),
                shown(1.0)
            ),
        ),
        (
            with(
                &fit_glm(&null, "gamma", None).unwrap(),
                "offset",
                fl(&[0.5; 8]),
            ),
            format!(
                "offset row 1 is {}, where big's is {}",
                shown(0.5),
                shown(0.0)
            ),
        ),
        (
            fit_glm(&gamma_design(&["z"], vec![]), "gamma", None).unwrap(),
            "names item 2, \"z\", is not among big's names".to_string(),
        ),
        (
            big.clone(),
            "has 2 columns, not fewer than big's 2".to_string(),
        ),
        (
            with(
                &fit_glm(&null, "gamma", None).unwrap(),
                "df.residual",
                int(6),
            ),
            "df.residual is 6, not above big's 6".to_string(),
        ),
    ];
    for (small, text) in cases {
        assert_eq!(
            compare(&small, &big, map(vec![])).unwrap_err(),
            mc_e101("small", &text)
        );
    }
    let small = fit_glm(&null, "gamma", None).unwrap();
    assert_eq!(
        compare(&small, &with(&big, "dispersion", float(0.0)), map(vec![])).unwrap_err(),
        mc_e101("big", &format!("dispersion is {}, not above 0", shown(0.0)))
    );
    // test: unknown, and F under a fixed dispersion.
    assert_eq!(
        compare(&small, &big, map(vec![("test", s("Chisq"))])).unwrap_err(),
        mc_e101("opts", "test is \"Chisq\"")
    );
    let ps = fit_glm(&dobson_outcome_design(), "poisson", None).unwrap();
    let pb = fit_glm(&dobson_design(), "poisson", None).unwrap();
    let f_poisson = "test \"F\" needs an estimated dispersion, and the poisson family's is 1";
    assert_eq!(
        compare(&ps, &pb, map(vec![("test", s("F"))])).unwrap_err(),
        mc_e101("opts", f_poisson)
    );
    assert_eq!(
        anova(
            &dobson_design(),
            vec![("family", s("poisson")), ("test", s("F"))]
        )
        .unwrap_err(),
        ma_e101("opts", f_poisson)
    );
    // Deepest first: a malformed small before bad opts, small before big.
    assert_eq!(
        compare(&int(1), &int(2), int(3)).unwrap_err(),
        mc_e101("small", "1")
    );
    assert_eq!(
        compare(&small, &int(2), int(3)).unwrap_err(),
        mc_e101("big", "2")
    );
    assert!(
        anova(&int(1), vec![("family", s("nope"))])
            .unwrap_err()
            .starts_with("MODEL.ANOVA: E101 design is outside")
    );
    assert_eq!(
        anova(&dobson_design(), vec![("weights", s("w"))]).unwrap_err(),
        ma_e101("opts", "the key \"weights\", which this word does not take")
    );
    assert_eq!(
        anova(&dobson_design(), vec![]).unwrap_err(),
        ma_e101("opts", "family is required")
    );
    // The null model of no columns at linkinv(0), its deviance not finite.
    let no = gamma_design(&["x"], vec![("intercept", BundValue::boolean(false))]);
    assert_eq!(
        anova(&no, vec![("family", s("gamma"))]).unwrap_err(),
        "MODEL.ANOVA: E114 the answer overflows a FLOAT: the null model's deviance at \
         linkinv(offset)"
    );
    // A row's refusal names the row: x alone steps out of μ > 0 first,
    // where x and z converge.
    let row = glm_design(
        &[0.0, 0.0, 5.0, 5.0, 1.0, 1.0],
        vec![("x", xs(6)), ("z", fl(&[0.0, 1.0, 0.0, 1.0, 0.0, 0.0]))],
        &["x", "z"],
        vec![],
    );
    assert!(fit_glm(&row, "poisson", Some("identity")).is_ok());
    assert_eq!(
        anova(
            &row,
            vec![("family", s("poisson")), ("link", s("identity"))]
        )
        .unwrap_err(),
        ma_e101(
            "design",
            "the first IRLS step leaves the poisson family's valid range, with no earlier \
             coefficients to halve towards, fitting the model through term \"x\""
        )
    );
    // A constant y under Gaussian identity: the perfect-fit refusal, not
    // the constant one, which fit.lm gives. Three rows, so the RSS is
    // exactly 0; at four it is a rounding's, and the table is answered,
    // as fit.lm's perfect-fit test is on an exact 0 (§S3).
    let constant = glm_design(&[1.0; 3], vec![("x", xs(3))], &[], vec![]);
    assert_eq!(
        anova(&constant, vec![("family", s("gaussian"))]).unwrap_err(),
        ma_e101("design", "a perfect fit, its residual variance 0")
    );
    assert!(
        on(&[constant.clone(), map(vec![])], "fit.lm")
            .unwrap_err()
            .contains("constant")
    );
}

#[test]
fn m5_words_never_abort() {
    let ok = |e: &str| e.contains(": E1") || e.contains("too shallow");
    let fit = fit_glm(&dobson_design(), "poisson", None).unwrap();
    let values = [
        fit.clone(),
        with(&fit, "names", int(1)),
        with(&fit, "dispersion", float(-1.0)),
        dobson_design(),
        map(vec![]),
        map(vec![("test", int(1))]),
        map(vec![("family", s("gamma"))]),
        int(2),
        none(),
    ];
    for a in &values {
        for b in &values {
            if let Err(e) = on(&[a.clone(), b.clone()], "model.anova") {
                assert!(ok(&e), "model.anova: {e}");
            }
            if let Err(e) = run_on(std::slice::from_ref(a), Some(b), "model.anova.").map(|_| ()) {
                assert!(ok(&e), "model.anova.: {e}");
            }
            for c in &values {
                if let Err(e) = on(&[a.clone(), b.clone(), c.clone()], "model.compare") {
                    assert!(ok(&e), "model.compare: {e}");
                }
            }
        }
    }
}
