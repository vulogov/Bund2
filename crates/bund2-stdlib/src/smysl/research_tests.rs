//! RFC-0014 M5's criteria: retraction and research (§S4, §S5), driven
//! through the words, with SMYSL's API as the second opinion.

use std::collections::BTreeMap;

use bund2_value::BundValue;
use smysl_core::{RelKind, Relation};
use smysl_graph::{
    EdgeSet, Lineage, RetractionPolicy, SalienceRequest, TraceKind, dependents_trace,
    effective_status, salience, trace, trace_via, view_roots,
};

use super::forming_tests::{
    Dir, Run, add_unit, call, get, handle, handle_as, items, list, m, rel, s, text, uid, unit,
};

/// A unit MAP of `status`, grounded on `grounds`.
fn grounded(gist: &str, status: &str, grounds: &[&str]) -> BundValue {
    let mut u = unit(gist).as_map().unwrap().clone();
    u.insert("status".to_owned(), s(status));
    if !grounds.is_empty() {
        u.insert(
            "grounds".to_owned(),
            list(grounds.iter().map(|g| s(g)).collect()),
        );
    }
    BundValue::map(u)
}

fn retract(h: BundValue, target: &str) -> Result<BundValue, String> {
    call(vec![h, s(target)], "smysl.retract").0
}

/// The fixture: a ground, a two-step cascade resting on it, and a unit
/// apart, each written by `tool:bund2`.
struct Cascade {
    d: Dir,
    ground: String,
    first: String,
    second: String,
    apart: String,
}

fn cascade(tag: &str) -> Cascade {
    let d = Dir::library(tag);
    let ground = add_unit(&d, unit("The ground"));
    let first = add_unit(&d, grounded("Rests on the ground", "inferred", &[&ground]));
    let second = add_unit(&d, grounded("Rests on the first", "inferred", &[&first]));
    let apart = add_unit(&d, unit("Rests on nothing"));
    Cascade {
        d,
        ground,
        first,
        second,
        apart,
    }
}

/// Criterion 8: `smysl.status` equals `effective_status` under the strict
/// policy over a retraction and a two-step cascade; the edge runs from the
/// target to itself and carries no attestation; a second retraction of the
/// target writes nothing and answers NODATA, whatever the agent; a unit
/// unfounded only by the cascade is retracted like any other.
#[test]
fn retraction_is_smysls() {
    let c = cascade("retract");
    let r = text(&retract(handle(&c.d), &c.ground).unwrap());
    let lib = c.d.open();
    let edge = lib.store().relation_by_id(&uid(&r)).unwrap().clone();
    assert_eq!(edge.kind, RelKind::Retracts);
    assert_eq!((edge.from, edge.to), (uid(&c.ground), uid(&c.ground)));
    assert!(lib.store().attestations_of(&uid(&r)).is_empty());

    let status = call(vec![handle(&c.d)], "smysl.status").0.unwrap();
    let eff = effective_status(lib.store(), RetractionPolicy::Strict);
    let direct: BTreeMap<String, String> = lib
        .store()
        .units()
        .filter(|(u, unit)| eff.get(u) != Some(unit.core.status))
        .map(|(u, _)| (u.canonical(), eff.get(u).unwrap().as_str().to_owned()))
        .collect();
    let answered: BTreeMap<String, String> = status
        .as_map()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), text(v)))
        .collect();
    assert_eq!(answered, direct);
    assert_eq!(
        answered.keys().cloned().collect::<Vec<_>>().len(),
        3,
        "the ground and both steps of the cascade"
    );
    assert!(!answered.contains_key(&c.apart));

    // Again, by another agent: nothing written, NODATA.
    let len = c.d.catalog_len();
    assert_eq!(
        retract(handle_as(&c.d, "tool:other"), &c.ground).unwrap(),
        BundValue::nodata()
    );
    assert_eq!(c.d.catalog_len(), len);

    // Unfounded only by the cascade: planned and retracted like any other.
    let r2 = text(&retract(handle(&c.d), &c.second).unwrap());
    assert_eq!(
        r2,
        Relation::new(RelKind::Retracts, uid(&c.second), uid(&c.second))
            .uid()
            .canonical()
    );
    let _ = c.first;
}

/// Criterion 8 and SM09: a unit attested only by `tool:other` is refused
/// when `tool:bund2` retracts it. Criterion 15: a unit written first by
/// `tool:other` and then by `tool:bund2` gains one attestation, and that
/// handle may then retract it; a handle whose agent `smysl.agent` sets to
/// `tool:other` may retract a unit only `tool:other` attested, which pins
/// R21's statement that authority is advisory.
#[test]
fn authority_is_origin_and_advisory() {
    let d = Dir::library("authority");
    let other = text(
        &call(
            vec![handle_as(&d, "tool:other"), unit("Theirs")],
            "smysl.unit",
        )
        .0
        .unwrap(),
    );
    assert_eq!(
        retract(handle(&d), &other).unwrap_err(),
        "SMYSL.RETRACT: retraction refused: origin authority: none of the 1 requesting \
         agent(s) attested this unit"
    );

    let shared = text(
        &call(
            vec![handle_as(&d, "tool:other"), unit("Shared")],
            "smysl.unit",
        )
        .0
        .unwrap(),
    );
    add_unit(&d, unit("Shared"));
    let agents: Vec<String> = d
        .open()
        .store()
        .attestations_of(&uid(&shared))
        .iter()
        .map(|a| a.agent.as_str().to_owned())
        .collect();
    assert_eq!(agents.len(), 2, "{agents:?}");
    retract(handle(&d), &shared).unwrap();

    let mut r = Run::new(false);
    let h = r
        .call(vec![handle(&d), s("tool:other")], "smysl.agent")
        .unwrap();
    let claimed = r.call(vec![h, s(&other)], "smysl.retract").unwrap();
    assert_eq!(
        text(&claimed),
        Relation::new(RelKind::Retracts, uid(&other), uid(&other))
            .uid()
            .canonical()
    );
}

fn nodes(l: &Lineage) -> Vec<(String, i64, String)> {
    l.nodes
        .iter()
        .map(|n| {
            (
                n.uid.canonical(),
                i64::from(n.depth),
                n.via.as_str().to_owned(),
            )
        })
        .collect()
}

fn answered(v: &BundValue) -> Vec<(String, i64, String)> {
    items(v)
        .iter()
        .map(|n| {
            (
                text(&get(n, "uid")),
                get(n, "depth").as_int().unwrap(),
                text(&get(n, "via")),
            )
        })
        .collect()
}

/// Criterion 8: `smysl.trace` with `"rests-on"` equals `trace_via` over
/// `EdgeSet::premises`; so does each other kind its own SMYSL call, at a
/// bound depth and unbounded. An unknown kind is SM05.
#[test]
fn trace_is_smysls_lineage() {
    let d = Dir::library("trace");
    let base = add_unit(&d, unit("A premise"));
    let mid = add_unit(&d, grounded("Rests on the premise", "inferred", &[&base]));
    let top = add_unit(&d, grounded("Rests on the middle", "inferred", &[&mid]));
    let cond = add_unit(&d, unit("A condition"));
    call(
        vec![handle(&d), rel("conditions", &cond, &top)],
        "smysl.rel",
    )
    .0
    .unwrap();
    let lib = d.open();
    let store = lib.store();
    let premises = EdgeSet::premises();
    for depth in [None, Some(1u32)] {
        let dv = depth.map_or_else(BundValue::nodata, |n| BundValue::int(i64::from(n)));
        let run = |root: &str, kind: &str| {
            answered(
                &call(
                    vec![handle(&d), s(root), s(kind), dv.clone()],
                    "smysl.trace",
                )
                .0
                .unwrap(),
            )
        };
        assert_eq!(
            run(&top, "rests-on"),
            nodes(&trace_via(store, uid(&top), &premises, depth))
        );
        for (kind, k) in [
            ("parents", TraceKind::Parents),
            ("grounds", TraceKind::Grounds),
            ("both", TraceKind::Both),
        ] {
            assert_eq!(run(&top, kind), nodes(&trace(store, uid(&top), k, depth)));
        }
        assert_eq!(
            run(&base, "dependents"),
            nodes(&dependents_trace(store, uid(&base), &premises, depth))
        );
    }
    assert!(
        answered(
            &call(
                vec![handle(&d), s(&top), s("rests-on"), BundValue::nodata()],
                "smysl.trace"
            )
            .0
            .unwrap()
        )
        .iter()
        .any(|n| n.0 == cond),
        "rests-on follows conditions"
    );
    assert_eq!(
        call(
            vec![handle(&d), s(&top), s("ancestors"), BundValue::nodata()],
            "smysl.trace"
        )
        .0
        .unwrap_err(),
        "SMYSL.TRACE: unknown trace kind: ancestors"
    );
    assert_eq!(
        call(
            vec![handle(&d), s(&top), s("grounds"), BundValue::int(-1)],
            "smysl.trace"
        )
        .0
        .unwrap_err(),
        "SMYSL.TRACE: depth -1 is negative or too large"
    );
}

/// `smysl.get`: the unit's MAP with its labels, NODATA when absent.
#[test]
fn get_answers_the_unit_or_nodata() {
    let d = Dir::library("get");
    let a = add_unit(&d, unit("A"));
    call(vec![handle(&d), s("c/a"), s(&a)], "smysl.label")
        .0
        .unwrap();
    let got = call(vec![handle(&d), s(&a)], "smysl.get").0.unwrap();
    assert_eq!(text(&get(&got, "uid")), a);
    assert_eq!(text(&get(&got, "gist")), "A");
    assert_eq!(
        items(&get(&got, "labels"))
            .iter()
            .map(text)
            .collect::<Vec<_>>(),
        vec!["c/a"]
    );
    let other = Dir::library("get-none");
    assert_eq!(
        call(vec![handle(&other), s(&a)], "smysl.get").0.unwrap(),
        BundValue::nodata()
    );
}

/// 18: `smysl.salience` and `smysl.pack` over a fixture library equal
/// `salience` and `pack` called directly with the same inputs.
#[test]
fn salience_and_pack_are_smysls() {
    let c = cascade("salience");
    let lib = c.d.open();
    let store = lib.store();
    let as_pairs = |v: &BundValue| -> Vec<(String, f64)> {
        items(v)
            .iter()
            .map(|x| {
                let score = match get(x, "score").unboxed() {
                    BundValue::Float(f, _) => *f,
                    other => panic!("score is {other:?}"),
                };
                (text(&get(x, "uid")), score)
            })
            .collect()
    };
    let direct = |req: &SalienceRequest| -> Vec<(String, f64)> {
        let r = salience(store, req);
        r.top(r.scores.len())
            .into_iter()
            .map(|(u, sc)| (u.canonical(), f64::from(sc)))
            .collect()
    };
    let plain = call(vec![handle(&c.d), m(&[])], "smysl.salience")
        .0
        .unwrap();
    assert_eq!(
        as_pairs(&plain),
        direct(&SalienceRequest::default().seeded(view_roots(store)))
    );
    assert_eq!(items(&plain).len(), 4);
    let focused = call(
        vec![handle(&c.d), m(&[("focus", list(vec![s(&c.first)]))])],
        "smysl.salience",
    )
    .0
    .unwrap();
    assert_eq!(
        as_pairs(&focused),
        direct(&SalienceRequest::default().seeded([uid(&c.first)]))
    );

    let sal = salience(store, &SalienceRequest::default().seeded(view_roots(store)));
    for (spec, req) in [
        (
            m(&[("budget", BundValue::int(40))]),
            smysl_pack::PackRequest::budget(40),
        ),
        (
            m(&[
                ("budget", BundValue::int(200)),
                ("focus", list(vec![s(&c.second)])),
            ]),
            smysl_pack::PackRequest::budget(200).focusing([uid(&c.second)]),
        ),
    ] {
        let packed = call(vec![handle(&c.d), spec], "smysl.pack").0.unwrap();
        let direct = smysl_pack::pack(store, &sal, &req).unwrap();
        assert_eq!(
            items(&packed).iter().map(text).collect::<Vec<_>>(),
            direct
                .selection
                .keys()
                .map(|u| u.canonical())
                .collect::<Vec<_>>()
        );
    }
    let missing = Dir::library("pack-missing");
    let err = call(
        vec![
            handle(&missing),
            m(&[
                ("budget", BundValue::int(40)),
                ("focus", list(vec![s(&c.first)])),
            ]),
        ],
        "smysl.pack",
    )
    .0
    .unwrap_err();
    assert!(
        err.starts_with("SMYSL.PACK: SMYSL refused it: SMY-E201"),
        "{err}"
    );
    assert_eq!(
        call(vec![handle(&c.d), m(&[])], "smysl.pack")
            .0
            .unwrap_err(),
        "SMYSL.PACK: spec lacks budget"
    );
}

/// 18's `--noio` case for M5's words.
#[test]
fn under_noio_m5s_words_are_sm06() {
    let h = || {
        m(&[
            ("type", s("smysl")),
            ("root", s("/nowhere")),
            ("agent", s("tool:bund2")),
        ])
    };
    for (operands, word) in [
        (vec![h(), s("b3:x")], "smysl.retract"),
        (vec![h(), s("b3:x")], "smysl.get"),
        (
            vec![h(), s("b3:x"), s("grounds"), BundValue::nodata()],
            "smysl.trace",
        ),
        (vec![h()], "smysl.status"),
        (vec![h(), m(&[])], "smysl.salience"),
        (vec![h(), m(&[])], "smysl.pack"),
    ] {
        let mut r = Run::new(true);
        assert_eq!(
            r.call(operands, word).unwrap_err(),
            super::words::NOIO,
            "{word}"
        );
    }
}
