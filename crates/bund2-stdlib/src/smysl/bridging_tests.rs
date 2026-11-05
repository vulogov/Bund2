//! RFC-0014 M8's criteria: `smysl.kb` (§S7), driven through the word, with
//! the library read through SMYSL's API as the second opinion.

use std::collections::BTreeSet;

use bund2_value::BundValue;
use smysl_core::{KernelType, Record, RelKind, Status, UnitCoreBuilder, canonical_uid};
use smysl_graph::{RetractionPolicy, effective_status, labels_of};

use crate::kb::store::Kb;

use super::forming_tests::{
    Dir, Run, add_text, add_unit, appended, call, citing, get, handle, items, list, m, raw, rel,
    rid, s, text, uid, unit, withdrawal,
};

/// `smysl.kb` over `scope`: the KB's clauses, and the warnings reported.
fn kb_of(d: &Dir, scope: &str) -> (Result<Vec<BundValue>, String>, Vec<String>) {
    let (out, warnings) = call(vec![handle(d), s(scope)], "smysl.kb");
    let clauses = out.map(|kb| Kb::read(&kb, 1).unwrap().clauses().unwrap());
    (clauses, warnings)
}

/// A fact's functor and its arguments, as text.
fn parts(f: &BundValue) -> (String, Vec<BundValue>) {
    let items = items(f);
    (items[0].as_str().unwrap(), items[1..].to_vec())
}

/// The facts of `functor`, each its arguments.
fn of(facts: &[BundValue], functor: &str) -> Vec<Vec<BundValue>> {
    facts
        .iter()
        .map(parts)
        .filter(|(f, _)| f == functor)
        .map(|(_, a)| a)
        .collect()
}

/// A unit MAP of `schema` and `status`, with `grounds` and `deps`.
fn shaped(schema: &str, gist: &str, status: &str, grounds: &[&str], deps: &[&str]) -> BundValue {
    let mut u = vec![
        ("schema", s(schema)),
        ("gist", s(gist)),
        ("status", s(status)),
    ];
    if !grounds.is_empty() {
        u.push(("grounds", list(grounds.iter().map(|g| s(g)).collect())));
    }
    if !deps.is_empty() {
        u.push(("deps", list(deps.iter().map(|g| s(g)).collect())));
    }
    m(&u)
}

/// The fixture: a claim, a finding resting on it, a rebuttal of the claim,
/// a labelled unit, a unit with a dep, and a retraction that unfounds the
/// finding.
struct Fixture {
    d: Dir,
    claim: String,
    finding: String,
    rebuts: smysl_core::Uid,
}

fn fixture(tag: &str) -> Fixture {
    let d = Dir::library(tag);
    let claim = add_unit(
        &d,
        shaped("claim", "The well is dry", "speculative", &[], &[]),
    );
    let finding = add_unit(
        &d,
        shaped(
            "finding",
            "The village must move",
            "inferred",
            &[&claim],
            &[],
        ),
    );
    let rebuttal = add_unit(&d, unit("The well has water"));
    let named = add_unit(&d, unit("A labelled unit"));
    call(vec![handle(&d), s("c/well"), s(&named)], "smysl.label")
        .0
        .unwrap();
    add_unit(
        &d,
        shaped(
            "claim",
            "Depends on the label",
            "speculative",
            &[],
            &[&named],
        ),
    );
    appended(
        &d,
        vec![
            rel("rebuts", &rebuttal, &claim),
            rel("elaborates", &finding, &claim),
        ],
    )
    .unwrap();
    let rebuts = rid(RelKind::Rebuts, &rebuttal, &claim);
    Fixture {
        d,
        claim,
        finding,
        rebuts,
    }
}

/// Criterion 11: each kind of fact counted against the library.
#[test]
fn every_kind_of_fact_is_counted_against_the_library() {
    let f = fixture("counts");
    call(vec![handle(&f.d), s(&f.claim)], "smysl.retract")
        .0
        .unwrap();
    let (facts, warnings) = kb_of(&f.d, "from library");
    let facts = facts.unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");

    let lib = f.d.open();
    let store = lib.store();
    let held: BTreeSet<smysl_core::Uid> = store.units().map(|(u, _)| *u).collect();
    let eff = effective_status(store, RetractionPolicy::Strict);
    let us = |u: &smysl_core::Uid| s(&u.canonical());

    // `BundValue` has no order, so each fact's arguments compare as their
    // display, which is total for STRINGs and INTs.
    let shown = |a: Vec<BundValue>| a.iter().map(BundValue::display).collect::<Vec<_>>();
    let expect = |pairs: Vec<Vec<BundValue>>| pairs.into_iter().map(shown).collect::<BTreeSet<_>>();
    let got = |functor: &str| {
        of(&facts, functor)
            .into_iter()
            .map(shown)
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(
        got("unit"),
        expect(
            store
                .units()
                .map(|(u, x)| vec![
                    us(u),
                    s(&x.core.schema.to_string()),
                    s(x.core.status.as_str())
                ])
                .collect()
        )
    );
    assert_eq!(
        got("gist"),
        expect(
            store
                .units()
                .map(|(u, x)| vec![us(u), s(&x.core.gist)])
                .collect()
        )
    );
    assert_eq!(
        got("grounds"),
        expect(
            store
                .units()
                .flat_map(|(u, x)| x.core.grounds.iter().map(move |g| vec![us(u), us(g)]))
                .collect()
        )
    );
    assert_eq!(
        got("deps"),
        expect(
            store
                .units()
                .flat_map(|(u, x)| x.core.deps.iter().map(move |g| vec![us(u), us(g)]))
                .collect()
        )
    );
    assert_eq!(
        got("label"),
        expect(
            held.iter()
                .flat_map(|u| {
                    labels_of(store, u)
                        .into_iter()
                        .map(move |l| vec![s(l.as_str()), us(u)])
                })
                .collect()
        )
    );
    assert_eq!(got("label").len(), 1);
    let effective = got("effective");
    assert_eq!(
        effective,
        expect(
            store
                .units()
                .filter_map(|(u, x)| eff
                    .get(u)
                    .filter(|e| *e != x.core.status)
                    .map(|e| vec![us(u), s(e.as_str())]))
                .collect()
        )
    );
    assert!(effective.contains(&shown(vec![s(&f.finding), s("unfounded")])));
    assert_eq!(
        got("rel"),
        expect(
            store
                .relations()
                .filter(|r| held.contains(&r.from)
                    && held.contains(&r.to)
                    && !store.is_withdrawn(r))
                .map(|r| vec![s(r.kind.as_str()), us(&r.from), us(&r.to)])
                .collect()
        )
    );
    // The retraction's self-edge is a relation with both ends in scope.
    assert!(got("rel").contains(&shown(vec![s("retracts"), s(&f.claim), s(&f.claim)])));
    assert_eq!(
        got("rank"),
        expect(
            Status::ALL
                .iter()
                .map(|x| vec![s(x.as_str()), BundValue::int(i64::from(x.as_u8()))])
                .collect()
        )
    );
    // Nothing else.
    let kinds: BTreeSet<String> = facts.iter().map(|x| parts(x).0).collect();
    assert_eq!(
        kinds,
        [
            "deps",
            "effective",
            "gist",
            "grounds",
            "label",
            "rank",
            "rel",
            "unit"
        ]
        .map(String::from)
        .into()
    );
}

/// The example rule of §S7, asked of a KB through `kb.query`: the uids of
/// the findings it finds.
fn shaky(kb_scope: &str, d: &Dir) -> Vec<String> {
    let mut r = Run::new(false);
    let answers = r
        .call(
            vec![handle(d), s(kb_scope)],
            "smysl.kb \
             [ [ shaky F ] ← [ unit F \"finding\" _ ] [ grounds F C ] [ rel \"rebuts\" _ C ] ] \
             kb.assert [ shaky F ] kb.query",
        )
        .unwrap();
    items(&answers).iter().map(|a| text(&get(a, "F"))).collect()
}

/// Criterion 11: a `rebuts` edge withdrawn through SMYSL's API gives no
/// `rel` fact, and the example rule finds no finding resting on its target
/// through it. Before the withdrawal, it finds one.
#[test]
fn a_withdrawn_edge_gives_no_fact() {
    let f = fixture("withdrawn");
    assert_eq!(
        shaky("from library", &f.d),
        std::slice::from_ref(&f.finding)
    );
    raw(
        &f.d,
        &[Record::Withdrawal(withdrawal(
            f.rebuts,
            "tool:bund2",
            "tool:bund2",
        ))],
    );
    let (facts, _) = kb_of(&f.d, "from library");
    let rebuts = of(&facts.unwrap(), "rel")
        .into_iter()
        .filter(|a| a[0] == s("rebuts"))
        .count();
    assert_eq!(rebuts, 0);
    assert!(shaky("from library", &f.d).is_empty());
}

/// Criterion 11: a superseded unit that the scope matches without a history
/// traversal gives no `unit` fact, and its newest version gives one.
#[test]
fn a_superseded_unit_gives_no_fact() {
    let d = Dir::library("superseded");
    let old = add_unit(&d, unit("The first account"));
    let new = add_unit(&d, unit("The second account"));
    appended(&d, vec![rel("supersedes", &new, &old)]).unwrap();
    let (facts, _) = kb_of(&d, "from library\nwhere kind = claim");
    let units: BTreeSet<String> = of(&facts.unwrap(), "unit")
        .iter()
        .map(|a| text(&a[0]))
        .collect();
    assert!(units.contains(&new));
    assert!(!units.contains(&old));
}

/// Criterion 11: a scope whose answer is a `count` gives only the six
/// `rank` facts.
#[test]
fn a_count_gives_only_the_ranks() {
    let f = fixture("count");
    let facts = kb_of(&f.d, "from library\ncount").0.unwrap();
    assert_eq!(facts.len(), 6);
    assert_eq!(of(&facts, "rank").len(), 6);
}

/// Criterion 11: a scope over a forked supersession reports `SMY-W464` as
/// a warning.
#[test]
fn a_fork_is_a_warning() {
    let d = Dir::library("fork");
    let old = add_unit(&d, unit("The first account"));
    let left = add_unit(&d, unit("One later account"));
    let right = add_unit(&d, unit("Another later account"));
    appended(
        &d,
        vec![
            rel("supersedes", &left, &old),
            rel("supersedes", &right, &old),
        ],
    )
    .unwrap();
    let (facts, warnings) = kb_of(&d, "from library\nfind \"account\"");
    facts.unwrap();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("SMYSL.KB: warning: SMY-W464: "),
        "{warnings:?}"
    );
}

/// Criterion 11: a `grounds` closure from a unit with a ground the library
/// does not hold gives a `grounds` fact for it, and no `unit` fact.
#[test]
fn an_unheld_ground_gives_no_unit_fact() {
    let d = Dir::library("unheld");
    let absent = UnitCoreBuilder::new(KernelType::Claim, "Never written", Status::Speculative)
        .build()
        .unwrap();
    let claim = UnitCoreBuilder::new(KernelType::Claim, "Rests on it", Status::Inferred)
        .grounds([canonical_uid(&absent)])
        .build()
        .unwrap();
    let c = canonical_uid(&claim).canonical();
    let a = canonical_uid(&absent).canonical();
    raw(&d, &[Record::Unit(claim)]);
    let facts = kb_of(&d, "from library\nwith grounds").0.unwrap();
    let units: Vec<String> = of(&facts, "unit").iter().map(|x| text(&x[0])).collect();
    assert_eq!(units, std::slice::from_ref(&c));
    assert_eq!(of(&facts, "grounds"), [vec![s(&c), s(&a)]]);
}

/// M8's first step: what a `from texts` scope's answer names. In a query,
/// `texts` resolves to every unit of the library, as `library` does
/// (`smysl-text/src/scope.rs:178-180,238,341-343`): units, whether or not
/// they cite a text, and never a part or a manifest. So §S7's rule for a
/// uid with no unit has nothing to leave out of it, and its facts are the
/// library's units'.
#[test]
fn a_texts_scope_names_the_librarys_units() {
    let d = Dir::library("texts");
    let added = add_text(&d, "flock", "One sheep.\n\nTwo sheep.");
    let cites = add_unit(&d, citing("Cites the flock", &added));
    let apart = add_unit(&d, unit("Cites nothing"));
    let answer = call(vec![handle(&d), s("from texts")], "smysl.query")
        .0
        .unwrap();
    let named: BTreeSet<String> = items(&get(&answer, "units")).iter().map(text).collect();
    assert_eq!(named, BTreeSet::from([cites.clone(), apart.clone()]));
    let lib = d.open();
    assert!(named.iter().all(|u| lib.store().get(&uid(u)).is_some()));
    let facts = kb_of(&d, "from texts").0.unwrap();
    let units: BTreeSet<String> = of(&facts, "unit").iter().map(|x| text(&x[0])).collect();
    assert_eq!(units, named);
}

/// SM04 for a scope `plan` refuses, and SM03 for a scope that is not a
/// STRING.
#[test]
fn a_refused_scope_is_sm04() {
    let d = Dir::library("sm04");
    let err = kb_of(&d, "from library\nfind \"x\" ~sem").0.unwrap_err();
    assert!(
        err.starts_with("SMYSL.KB: the query was refused: SMY-"),
        "{err}"
    );
    let err = call(vec![handle(&d), BundValue::int(1)], "smysl.kb")
        .0
        .unwrap_err();
    assert_eq!(err, "SMYSL.KB: #1 parameter must be STRING, found Integer");
}

/// 18's `--noio` case for `smysl.kb`.
#[test]
fn under_noio_the_bridge_is_sm06() {
    let h = m(&[
        ("type", s("smysl")),
        ("root", s("/nowhere")),
        ("agent", s("tool:bund2")),
    ]);
    let mut r = Run::new(true);
    assert_eq!(
        r.call(vec![h, s("from library")], "smysl.kb").unwrap_err(),
        super::words::NOIO
    );
}
