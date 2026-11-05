//! RFC-0014 M7's criteria: querying (§S6), driven through `smysl.query`
//! and the `smysl` conditional, with SMYSL's `parse`, `plan` and `run`,
//! called in §S5's environment, as the second opinion.

use std::collections::BTreeSet;

use bund2_api::Vm as _;
use bund2_value::BundValue;
use smysl_core::{Carry, KernelType, Record, Status, Uid, UnitCoreBuilder, canonical_uid};
use smysl_query::{Answer, Env, PlanOptions};
use smysl_text::{LibraryScope, Policy};

use super::forming_tests::{
    Dir, Run, add_text, add_unit, appended, call, get, handle, items, m, raw, rel, s, text, unit,
};

fn name(d: &Dir) -> String {
    d.0.file_stem().unwrap().to_string_lossy().into_owned()
}

/// `run`'s answer for `q`, called directly in §S5's environment.
fn direct(d: &Dir, q: &str) -> Answer {
    let lib = d.open();
    let scope = LibraryScope::new(name(d), &lib);
    let env = Env::new(lib.store(), &scope)
        .with_times(&scope)
        .with_entities(&scope);
    let opts = PlanOptions::default();
    let parsed = smysl_query::parse(q).unwrap();
    let plan = smysl_query::plan(&parsed.query, &env, &opts).unwrap();
    smysl_query::run(&plan, &env, &opts).unwrap().answer
}

fn queried(d: &Dir, q: &str) -> Result<BundValue, String> {
    call(vec![handle(d), s(q)], "smysl.query").0
}

fn uids(v: &BundValue) -> Vec<String> {
    items(v).iter().map(text).collect()
}

/// Every uid a MAP answer names, in its order, as `Answer::uids` does.
fn named(a: &BundValue) -> Vec<String> {
    match text(&get(a, "kind")).as_str() {
        "scope" | "packed" => uids(&get(a, "units")),
        "ranked" => items(&get(a, "hits"))
            .iter()
            .map(|h| text(&get(h, "uid")))
            .collect(),
        "groups" => items(&get(a, "groups"))
            .iter()
            .flat_map(|g| uids(&get(g, "units")))
            .collect(),
        "count" => Vec::new(),
        other => panic!("kind {other}"),
    }
}

/// The units a `smysl` conditional over `q` visits: the lambda leaves each
/// unit's `gist` on the stack.
fn visits(d: &Dir, q: &str) -> Result<Vec<String>, String> {
    let mut r = Run::new(false);
    r.call(
        vec![handle(d), s(q)],
        ":q swap var :h swap var \
         smysl :handle :h var? set :query :q var? set :lambda { :gist get } set !",
    )?;
    Ok(r.i.snapshot().iter().map(text).collect())
}

/// Five units, two of them claims, and no `supersedes` edge.
fn fixture(tag: &str) -> (Dir, Vec<String>) {
    let d = Dir::library(tag);
    let mut us = Vec::new();
    for g in [
        "Naomi went out full",
        "Naomi came back empty",
        "Ruth stayed with Naomi",
        "Boaz owned a field",
        "The harvest was barley",
    ] {
        us.push(add_unit(&d, unit(g)));
    }
    (d, us)
}

/// Criterion 9: one query per `Answer` variant gives §S6's MAP, with the
/// uids `run` gives, in the same order; each keeps its kind here.
#[test]
fn every_answer_shape_is_mapped() {
    let (d, _) = fixture("shapes");
    for (q, kind) in [
        ("from library\nwhere kind = claim", "scope"),
        ("from library\nfind \"Naomi\"", "ranked"),
        ("from library\ncount", "count"),
        ("from library\ngroup by kind", "groups"),
        ("from library\nfind \"Naomi\"\npack 400 tokens", "packed"),
    ] {
        let got = queried(&d, q).unwrap();
        let a = direct(&d, q);
        assert_eq!(text(&get(&got, "kind")), kind, "{q}");
        let theirs: Vec<String> = a.uids().iter().map(Uid::canonical).collect();
        assert_eq!(named(&got), theirs, "{q}");
        match a {
            Answer::Ranked(hits) => {
                let ours = items(&get(&got, "hits"));
                assert!(!hits.is_empty(), "{q}");
                for (o, h) in ours.iter().zip(&hits) {
                    assert_eq!(get(o, "score"), BundValue::float(h.score));
                }
            }
            Answer::Count(n) => {
                assert_eq!(get(&got, "count"), BundValue::int(n as i64));
                assert_eq!(n, 5);
            }
            Answer::Groups(groups) => {
                let keys: Vec<String> = items(&get(&got, "groups"))
                    .iter()
                    .map(|g| text(&get(g, "key")))
                    .collect();
                assert_eq!(keys, groups.iter().map(|g| g.0.clone()).collect::<Vec<_>>());
            }
            Answer::Scope(us) | Answer::Packed(us) => assert!(!us.is_empty(), "{q}"),
            other => panic!("{other:?}"),
        }
    }
}

/// Criterion 9's second fixture: with a `supersedes` edge, the ranked query
/// answers `"scope"` and the packed one `"packed"`, as `run` gives them.
#[test]
fn a_supersession_turns_a_ranking_into_a_scope() {
    let (d, us) = fixture("supersedes");
    appended(&d, vec![rel("supersedes", &us[1], &us[0])]).unwrap();
    for (q, kind) in [
        ("from library\nfind \"Naomi\"", "scope"),
        ("from library\nfind \"Naomi\"\npack 400 tokens", "packed"),
    ] {
        let got = queried(&d, q).unwrap();
        assert_eq!(text(&get(&got, "kind")), kind, "{q}");
        let theirs: Vec<String> = direct(&d, q).uids().iter().map(Uid::canonical).collect();
        assert_eq!(named(&got), theirs, "{q}");
    }
}

/// Criterion 9: a `grounds` closure from a unit with a ground the library
/// does not hold. `smysl.query`'s `units` include the ground; the
/// conditional does not visit it.
#[test]
fn an_unheld_ground_is_answered_and_not_visited() {
    let d = Dir::library("unheld");
    let absent = UnitCoreBuilder::new(KernelType::Claim, "Never written", Status::Speculative)
        .build()
        .unwrap();
    let claim = UnitCoreBuilder::new(KernelType::Claim, "Rests on it", Status::Inferred)
        .grounds([canonical_uid(&absent)])
        .build()
        .unwrap();
    let c = canonical_uid(&claim);
    raw(&d, &[Record::Unit(claim)]);
    let q = "from library\nwith grounds";
    let got = queried(&d, q).unwrap();
    let units: BTreeSet<String> = named(&got).into_iter().collect();
    assert_eq!(
        units,
        BTreeSet::from([c.canonical(), canonical_uid(&absent).canonical()])
    );
    assert_eq!(visits(&d, q).unwrap(), ["Rests on it"]);
}

/// Criterion 9: a `count` answer runs the lambda zero times; a query `plan`
/// refuses is SM04, carrying every diagnostic.
#[test]
fn count_visits_nothing_and_a_refused_plan_is_sm04() {
    let (d, _) = fixture("count");
    assert!(visits(&d, "from library\ncount").unwrap().is_empty());
    let all = visits(&d, "from library").unwrap();
    assert_eq!(all.len(), 5);

    // No semantic port is bound, so `~sem` is refused by `plan`.
    let q = "from library\nfind \"Naomi\" ~sem";
    let lib = d.open();
    let scope = LibraryScope::new(name(&d), &lib);
    let env = Env::new(lib.store(), &scope)
        .with_times(&scope)
        .with_entities(&scope);
    let parsed = smysl_query::parse(q).unwrap();
    let diags = smysl_query::plan(&parsed.query, &env, &PlanOptions::default()).unwrap_err();
    let each: Vec<String> = diags
        .iter()
        .map(|d| format!("{} {}", d.code.as_str(), d.message))
        .collect();
    assert_eq!(
        queried(&d, q).unwrap_err(),
        format!("SMYSL.QUERY: the query was refused: {}", each.join("; "))
    );
    // A query `parse` refuses is SM04 too, through the conditional as well.
    let err = visits(&d, "from library\nwhere").unwrap_err();
    assert!(
        err.starts_with("SMYSL: the query was refused: SMY-E"),
        "{err}"
    );
}

/// The runner's own refusals: a missing or mistyped slot is SM03 naming it,
/// a missing `lambda` among them; an error inside `lambda` is SM10.
#[test]
fn the_runners_refusals() {
    let (d, _) = fixture("runner");
    let run = |slots: &str| {
        let mut r = Run::new(false);
        r.call(vec![handle(&d)], &format!(":h swap var smysl {slots} !"))
    };
    let q = ":query \"from library\" set";
    assert_eq!(
        run(&format!("{q} :lambda {{ drop }} set")).unwrap_err(),
        "SMYSL: slot handle is missing"
    );
    assert_eq!(
        run(&format!(":handle 7 set {q} :lambda {{ drop }} set")).unwrap_err(),
        "SMYSL: slot handle must be MAP, found Integer"
    );
    assert_eq!(
        run(":handle :h var? set :lambda { drop } set").unwrap_err(),
        "SMYSL: slot query is missing"
    );
    assert_eq!(
        run(&format!(":handle :h var? set {q}")).unwrap_err(),
        "SMYSL: slot lambda is missing"
    );
    // The whole error: `Run::call` keeps only the innermost text.
    let mut r = Run::new(false);
    r.call(vec![handle(&d)], ":h swap var").unwrap();
    let src = format!("smysl :handle :h var? set {q} :lambda {{ drop drop }} set !");
    let e =
        r.i.eval(&bund2_syntax::compile(&src).unwrap())
            .unwrap_err()
            .0;
    assert!(e.contains("SMYSL: lambda returns: "), "{e}");
    assert!(e.contains("drop"), "{e}");
    // A slot holding a MAP that is no handle.
    let mut r = Run::new(false);
    let err = r
        .call(
            vec![m(&[("type", s("x"))])],
            &format!(":h swap var smysl :handle :h var? set {q} :lambda {{ drop }} set !"),
        )
        .unwrap_err();
    assert_eq!(err, "SMYSL: slot handle must be a SMYSL handle");
}

/// The answer is computed before the lambda runs: a lambda that writes a
/// unit to the same library does not add it to the units still visited,
/// and it can write, because no lock is held while it runs.
#[test]
fn a_lambda_may_write_to_the_library_it_visits() {
    let (d, _) = fixture("writes");
    let mut r = Run::new(false);
    r.call(vec![], ":n 0 var").unwrap();
    r.call(
        vec![handle(&d), unit("Written by the lambda")],
        ":u swap var :h swap var \
         smysl :handle :h var? set :query \"from library\" set \
         :lambda { drop :h var? :u var? smysl.unit drop :n var? 1 + :n swap var } set !",
    )
    .unwrap();
    assert_eq!(r.call(vec![], ":n var?").unwrap(), BundValue::int(5));
    assert_eq!(d.open().store().units().count(), 6);
}

const PLAIN: &str = "A plain text.\n\nIn two paragraphs.";
const MARKED: &str = "# A heading\n\nA marked text.\n";

/// Criterion 6: forming persists. A library made, a text added with each
/// enabled reader, units and relations appended; then, in a fresh VM,
/// `smysl.texts` names each alias with the `mid` it was added under, every
/// unit is found by `smysl.get` and by `smysl.query`, and each manifest
/// records `carry` none and SMYSL's default part policy.
#[test]
fn forming_persists() {
    let d = Dir::library("persists");
    let plain = add_text(&d, "plain", PLAIN);
    let marked = {
        let spec = m(&[
            ("text", s(MARKED)),
            ("reader", s("md/1")),
            ("alias", s("marked")),
            ("licence", s("CC0-1.0")),
            ("lang", s("en")),
        ]);
        call(vec![handle(&d), spec], "smysl.text.add").0.unwrap()
    };
    let a = add_unit(&d, unit("The first unit"));
    let b = add_unit(&d, unit("The second unit"));
    let more = appended(&d, vec![unit("The third unit"), rel("elaborates", &b, &a)]).unwrap();

    let mut r = Run::new(false);
    let texts = r.call(vec![handle(&d)], "smysl.texts").unwrap();
    for (alias, added) in [("plain", &plain), ("marked", &marked)] {
        assert_eq!(
            uids(&get(&texts, alias)),
            [text(&get(added, "mid"))],
            "{alias}"
        );
    }
    let all = r
        .call(vec![handle(&d), s("from library")], "smysl.query")
        .unwrap();
    let found: BTreeSet<String> = named(&all).into_iter().collect();
    let lib = d.open();
    let units: Vec<String> = lib.store().units().map(|(u, _)| u.canonical()).collect();
    assert!(units.contains(&a) && units.contains(&b));
    assert!(more.iter().any(|u| units.contains(u)));
    for u in &units {
        let got = r.call(vec![handle(&d), s(u)], "smysl.get").unwrap();
        assert_ne!(got, BundValue::nodata(), "{u}");
        assert!(found.contains(u), "{u} is not in the query's scope");
    }
    // SMYSL's default part policy is the reader's top level with
    // `Policy::default()`'s sizes (`smysl-text/src/library.rs:1176-1179`), so
    // the same text added through SMYSL's API to a twin library, with no
    // policy named, is the second opinion.
    let twin = Dir::library("persists-twin");
    for (added, reader, body) in [(&plain, "txt/1", PLAIN), (&marked, "md/1", MARKED)] {
        let mid = smysl_core::Mid::parse(&text(&get(added, "mid"))).unwrap();
        let manifest = lib.store().manifest(&mid).unwrap();
        assert_eq!(manifest.carry, Carry::None);
        let alias = format!("twin-{}", &reader[..2]);
        let spec = smysl_text::AddSpec::new(reader, alias.as_str(), "CC0-1.0")
            .with_lang(smysl_text::LangTag::new("en").unwrap());
        let theirs = twin
            .open()
            .add(
                &smysl_text::Input::new(body.as_bytes()),
                &spec,
                &smysl_text::Caps::default(),
            )
            .unwrap();
        let tl = twin.open();
        let expected = tl.store().manifest(&theirs.mid).unwrap();
        assert_eq!(manifest.part_policy, expected.part_policy, "{reader}");
        let policy = Policy::parse(&manifest.part_policy).unwrap();
        assert_eq!(policy.target_min, Policy::default().target_min);
        assert_eq!(policy.target_max, Policy::default().target_max);
    }
}

/// Criterion 18: `bund2 check` reads a program using the `smysl`
/// conditional with no unknown-effect diagnostic, follows its lambda over
/// the MAP pushed, and reads a missing `lambda` as empty.
#[test]
fn check_follows_the_conditional() {
    let r = Run::new(false);
    let check =
        |src: &str| crate::check::check(&bund2_syntax::compile(src).unwrap(), &r.i.registry, 0);
    let h = "1 set";
    let report = check(&format!(
        "smysl :handle {h} :query \"from library\" set :lambda {{ drop }} set ! 1 2 +"
    ));
    assert!(report.findings.is_empty(), "{report:?}");
    assert_eq!(report.abandoned, 0, "{report:?}");
    let report = check(&format!(
        "smysl :handle {h} :query \"from library\" set :lambda {{ drop drop }} set !"
    ));
    assert_eq!(report.findings.len(), 1, "the second drop: {report:?}");
    // A missing `lambda` is read as empty, as for `query`: the MAP each unit
    // pushes stays, so the depth after depends on the answer, and the
    // checker abandons the program there rather than reporting a word.
    let report = check(&format!("smysl :handle {h} :query \"q\" set ! 1 2 +"));
    assert!(report.findings.is_empty(), "{report:?}");
    assert_eq!(report.abandoned, 1, "{report:?}");
    assert!(
        report.reasons[0]
            .0
            .starts_with("`smysl` with a body that leaves +1")
    );
    let report = check("1 smysl.query");
    assert_eq!(report.findings.len(), 1, "{report:?}");
    assert_eq!(report.findings[0].word, "smysl.query");
}

/// 18's `--noio` case for `smysl.query`, the constructor, and a
/// CONDITIONAL of type `smysl` built with `conditional` and run, which
/// reads nothing.
#[test]
fn under_noio_the_query_the_constructor_and_the_handler_are_sm06() {
    let h = m(&[
        ("type", s("smysl")),
        ("root", s("/nowhere")),
        ("agent", s("tool:bund2")),
    ]);
    let mut r = Run::new(true);
    assert_eq!(
        r.call(vec![h, s("from library")], "smysl.query")
            .unwrap_err(),
        super::words::NOIO
    );
    assert_eq!(r.call(vec![], "smysl").unwrap_err(), super::words::NOIO);
    assert_eq!(
        r.call(vec![], "conditional :type \"smysl\" set !")
            .unwrap_err(),
        super::words::NOIO
    );
}
