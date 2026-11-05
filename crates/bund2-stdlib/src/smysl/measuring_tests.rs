//! RFC-0014 M6's criteria: dossiers and measures (§S5), driven through the
//! words, with SMYSL's API, called in §S5's environment, as the second
//! opinion.

use std::collections::BTreeSet;

use bund2_value::BundValue;
use smysl_core::schemas::text as xtext;
use smysl_core::{
    AgentId, Code, Diagnostic, Hlc, KernelType, Record, RelKind, Relation, SourceKind, SourceRef,
    Status, Uid, UnitCoreBuilder, canonical_uid,
};
use smysl_measure::text::{Corpus, texts};
use smysl_measure::{self as ms, NotPersisted, Outcome, Recipe};
use smysl_query::{AskOptions, Env, PlanOptions, ask};
use smysl_text::{Library, LibraryScope};

use super::forming_tests::{Dir, Run, add_text, call, get, handle, items, list, m, raw, s, text};

/// What criterion 10 says is unchanged: the catalog's bytes, and every
/// unit and relation the store holds.
fn held(d: &Dir) -> (u64, BTreeSet<Uid>, BTreeSet<Uid>) {
    let lib = d.open();
    (
        d.catalog_len(),
        lib.store().units().map(|(u, _)| *u).collect(),
        lib.store().relations().map(|r| r.uid()).collect(),
    )
}

/// The scope's name §S5 gives it: the root's file stem.
fn name(d: &Dir) -> String {
    d.0.file_stem().unwrap().to_string_lossy().into_owned()
}

/// The spike's fixture (`tests/smysl_spike.rs`): an entity, an observation
/// that mentions it, and an inferred claim grounded on the observation
/// that mentions it too.
fn naomi(tag: &str) -> (Dir, Uid) {
    let d = Dir::library(tag);
    let entity = xtext::entity("Naomi", "kjv/ruth", None).unwrap();
    let observation = UnitCoreBuilder::new(
        KernelType::Observation,
        "Naomi said: I went out full, and the LORD hath brought me home again empty.",
        Status::Cited,
    )
    .source(SourceRef::new(SourceKind::Doc, "Ruth 1:21"))
    .build()
    .unwrap();
    let claim = UnitCoreBuilder::new(
        KernelType::Claim,
        "Naomi came back to Bethlehem with nothing.",
        Status::Inferred,
    )
    .grounds([canonical_uid(&observation)])
    .build()
    .unwrap();
    let (e, o, c) = (
        canonical_uid(&entity),
        canonical_uid(&observation),
        canonical_uid(&claim),
    );
    let mentions = || RelKind::parse(xtext::MENTIONS).unwrap();
    raw(
        &d,
        &[
            Record::SchemaDecl(xtext::decl()),
            Record::Unit(entity),
            Record::Unit(observation),
            Record::Unit(claim),
            Record::Relation(Relation::new(RelKind::Elaborates, c, o)),
            Record::Relation(Relation::new(mentions(), o, e)),
            Record::Relation(Relation::new(mentions(), c, e)),
        ],
    );
    (d, e)
}

fn uids(v: &BundValue) -> Vec<String> {
    items(v).iter().map(text).collect()
}

fn canon(us: &[Uid]) -> Vec<String> {
    us.iter().map(Uid::canonical).collect()
}

/// Criterion 10's dossier: `smysl.dossier` answers what `ask` gives in
/// §S5's environment, and writes nothing. The direct call's clock is
/// `Hlc::zero`'s, which no field compared depends on.
#[test]
fn a_dossier_is_what_ask_gives() {
    let (d, entity) = naomi("dossier");
    let before = held(&d);
    let (out, warnings) = call(vec![handle(&d), s("What is Naomi?")], "smysl.dossier");
    let answer = out.unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(held(&d), before);

    let lib = d.open();
    let scope = LibraryScope::new(name(&d), &lib);
    let env = Env::new(lib.store(), &scope)
        .with_times(&scope)
        .with_entities(&scope);
    let owner = AgentId::new("tool:bund2").unwrap();
    let direct = ask(
        "What is Naomi?",
        &env,
        &AskOptions::new(owner.clone()).with_ts(Hlc::zero(owner)),
    )
    .unwrap();

    assert_eq!(text(&get(&answer, "subject")), direct.subject);
    assert_eq!(uids(&get(&answer, "entities")), canon(&[entity]));
    assert_eq!(uids(&get(&answer, "entities")), canon(&direct.entities));
    let slots = items(&get(&answer, "slots"));
    assert!(!slots.is_empty());
    assert_eq!(slots.len(), direct.slots.len());
    for (ours, theirs) in slots.iter().zip(&direct.slots) {
        assert_eq!(text(&get(ours, "name")), theirs.name);
        assert_eq!(uids(&get(ours, "units")), canon(&theirs.units));
    }
}

/// A question `ask` cannot answer is SM02 with its `AskError`'s text, and
/// a question that is not a STRING is SM03.
#[test]
fn a_dossier_ask_refuses_is_sm02() {
    let (d, _) = naomi("dossier-refused");
    let err = call(vec![handle(&d), s("What is Zebedee?")], "smysl.dossier")
        .0
        .unwrap_err();
    assert!(
        err.starts_with("SMYSL.DOSSIER: SMYSL refused it: "),
        "{err}"
    );
    let err = call(vec![handle(&d), BundValue::int(7)], "smysl.dossier")
        .0
        .unwrap_err();
    assert_eq!(
        err,
        "SMYSL.DOSSIER: #1 parameter must be STRING, found Integer"
    );
}

/// A text over T1's floor of 10000 tokens, of distinct sentences.
fn body() -> String {
    (1..=900)
        .map(|n| format!("The {n} sheep went over hill number {n} and came back by evening."))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `smysl.measure` of `ids` over `scope`, its answer and its warnings.
fn measured(d: &Dir, ids: &[&str], scope: &str) -> (Result<BundValue, String>, Vec<String>) {
    call(
        vec![
            handle(d),
            list(ids.iter().map(|i| s(i)).collect()),
            s(scope),
        ],
        "smysl.measure",
    )
}

/// The direct calls criterion 10 compares with: §S5's environment, the
/// base `Recipe`, and an `Input` built by §S5's three steps.
fn direct(d: &Dir, ids: &[&str], scope_text: &str) -> Vec<(ms::MeasureResult, Option<Uid>)> {
    let lib: Library = d.open();
    let scope = LibraryScope::new(name(d), &lib);
    let env = Env::new(lib.store(), &scope)
        .with_times(&scope)
        .with_entities(&scope);
    let resolved = ms::resolve(scope_text, &env, &PlanOptions::default()).unwrap();
    let recipe = Recipe::new("", 0, scope_text);
    let corpus = ids
        .iter()
        .any(|id| ms::find(ms::ALL, id, None).is_ok_and(|m| m.needs().contains(ms::Needs::TEXT)))
        .then(|| {
            let found = texts(scope_text, &lib).unwrap();
            Corpus::load(&lib, &found, recipe.level, recipe.analyzer.as_deref()).unwrap()
        });
    let mut input = ms::Input::new(lib.store(), &resolved).with_library(&lib);
    if let Some(c) = &corpus {
        input = input.with_text(c);
    }
    ms::run(ids, &recipe, &input)
        .into_iter()
        .map(|r| {
            let unit = ms::to_unit_on(&r, &[resolved.basis.as_ref()])
                .ok()
                .map(|c| canonical_uid(&c));
            (r, unit)
        })
        .collect()
}

fn value_of(r: &ms::MeasureResult) -> String {
    match &r.outcome {
        Outcome::Value(v) => ms::gist(&r.recipe, v),
        other => panic!("{} did not run: {other:?}", r.recipe.id),
    }
}

/// Criterion 10's measures: T1 over a text scope and A4 over a unit scope
/// give what the direct calls give, `value` is `gist`, `uid` the unit's,
/// and nothing is written.
#[test]
fn measures_are_smysls() {
    let (d, _) = naomi("measure");
    add_text(&d, "flock", &body());
    let before = held(&d);

    for (id, scope) in [
        ("T1", "from texts"),
        (
            "A4",
            "from library\nwhere kind = claim or kind = observation",
        ),
    ] {
        let (out, warnings) = measured(&d, &[id], scope);
        let answer = items(&out.unwrap());
        assert_eq!(answer.len(), 1);
        let got = &answer[0];
        let [(r, unit)] = <[_; 1]>::try_from(direct(&d, &[id], scope)).unwrap();
        assert_eq!(text(&get(got, "id")), id);
        assert_eq!(
            get(got, "ok"),
            BundValue::boolean(true),
            "{id}: {}",
            text(&get(got, "value"))
        );
        assert_eq!(text(&get(got, "value")), value_of(&r));
        assert_eq!(text(&get(got, "uid")), unit.unwrap().canonical());
        assert_eq!(items(&get(got, "diagnostics")).len(), r.diagnostics.len());
        assert_eq!(warnings.len(), r.diagnostics.len(), "{warnings:?}");
    }
    assert_eq!(held(&d), before);
}

/// Criterion 10: a scope long enough that the unit's gist is cut, so the
/// unit's gist and `value` differ, and `value` is the whole of `gist`.
#[test]
fn a_measures_value_is_never_cut() {
    let (d, _) = naomi("measure-long");
    let scope = format!(
        "from library\nwhere {}",
        vec!["kind = claim"; 40].join(" or ")
    );
    let answer = items(&measured(&d, &["A4"], &scope).0.unwrap());
    let value = text(&get(&answer[0], "value"));

    let lib = d.open();
    let sc = LibraryScope::new(name(&d), &lib);
    let env = Env::new(lib.store(), &sc)
        .with_times(&sc)
        .with_entities(&sc);
    let resolved = ms::resolve(&scope, &env, &PlanOptions::default()).unwrap();
    let input = ms::Input::new(lib.store(), &resolved).with_library(&lib);
    let r = ms::run(&["A4"], &Recipe::new("", 0, scope.as_str()), &input).remove(0);
    let core = ms::to_unit_on(&r, &[resolved.basis.as_ref()]).unwrap();
    assert_eq!(value, value_of(&r));
    assert_ne!(core.gist, value);
    assert!(core.gist.ends_with('\u{2026}'), "{}", core.gist);
    assert_eq!(
        text(&get(&answer[0], "uid")),
        canonical_uid(&core).canonical()
    );
}

/// Criterion 10: an unknown id is a result, not a refusal: `ok` false,
/// `value` SMYSL's reason, `SMY-E489` among its diagnostics and reported;
/// the ids beside it still run.
#[test]
fn an_unknown_measure_is_a_result() {
    let (d, _) = naomi("measure-xx");
    let before = held(&d);
    let (out, warnings) = measured(&d, &["XX", "A4"], "from library");
    let answer = items(&out.unwrap());
    assert_eq!(answer.len(), 2);
    let xx = &answer[0];
    assert_eq!(text(&get(xx, "id")), "XX");
    assert_eq!(get(xx, "ok"), BundValue::boolean(false));
    assert_eq!(get(xx, "uid"), BundValue::nodata());
    let ds = items(&get(xx, "diagnostics"));
    assert_eq!(ds.len(), 1);
    assert_eq!(text(&get(&ds[0], "code")), "SMY-E489");
    assert_eq!(text(&get(xx, "value")), text(&get(&ds[0], "message")));
    assert_eq!(get(&answer[1], "ok"), BundValue::boolean(true));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("SMYSL.MEASURE: error: SMY-E489: no measure `XX`"),
        "{warnings:?}"
    );
    assert_eq!(held(&d), before);
}

/// Criterion 18's `Unwritable` case: the mapping gives `uid` NODATA, keeps
/// the diagnostic's own severity among `diagnostics`, and reports one
/// warning. `NoBasis` is mapped the same way, at its own severity.
#[test]
fn an_unwritable_result_is_a_warning() {
    let (d, _) = naomi("measure-unwritable");
    let [(r, _)] = <[_; 1]>::try_from(direct(&d, &["A4"], "from library")).unwrap();
    assert!(r.diagnostics.is_empty());
    for (why, severity) in [
        (
            NotPersisted::Unwritable(
                Diagnostic::new(Code::E489).with_message("a count past a payload integer"),
            ),
            "error",
        ),
        (
            NotPersisted::NoBasis(Diagnostic::new(Code::W483).with_message("no saved scope")),
            "warning",
        ),
    ] {
        let code = match &why {
            NotPersisted::Unwritable(d) | NotPersisted::NoBasis(d) => d.code,
            _ => unreachable!(),
        };
        let m = super::research::measured(&(r.clone(), Err(why))).unwrap();
        assert_eq!(get(&m.answer, "uid"), BundValue::nodata());
        assert_eq!(get(&m.answer, "ok"), BundValue::boolean(true));
        assert_eq!(text(&get(&m.answer, "value")), value_of(&r));
        let ds = items(&get(&m.answer, "diagnostics"));
        assert_eq!(ds.len(), 1);
        assert_eq!(text(&get(&ds[0], "code")), code.as_str());
        assert_eq!(text(&get(&ds[0], "severity")), severity);
        assert_eq!(m.warnings.len(), 1);
        assert_eq!(m.warnings[0].code, code);
        assert_eq!(m.warnings[0].severity.as_str(), severity);
    }
}

/// The word reports each warning the mapping returns, one `Warning` per
/// diagnostic, whatever its severity (§S8, D36).
#[test]
fn each_diagnostic_is_one_warning() {
    let mut r = Run::new(false);
    let ds = [
        Diagnostic::new(Code::E489).with_message("an error"),
        Diagnostic::new(Code::W483).with_message("a warning"),
    ];
    let warned = ds.iter().map(ToString::to_string).collect::<Vec<_>>();
    super::words::warn_each(&mut r.i, "smysl.measure", &ds);
    assert_eq!(
        r.warnings(),
        warned
            .iter()
            .map(|w| format!("SMYSL.MEASURE: {w}"))
            .collect::<Vec<_>>()
    );
}

/// SM04: a scope `sq/1` refuses, and a text scope that filters, which
/// `texts` refuses with `SMY-E461`, carrying SMYSL's code and message.
#[test]
fn a_refused_scope_is_sm04() {
    let (d, _) = naomi("measure-sm04");
    let err = measured(&d, &["A4"], "from nowhere at all").0.unwrap_err();
    assert!(
        err.starts_with("SMYSL.MEASURE: the query was refused: SMY-E"),
        "{err}"
    );
    add_text(&d, "flock", &body());
    let err = measured(&d, &["T1"], "from texts\nwhere kind = claim")
        .0
        .unwrap_err();
    assert!(
        err.starts_with("SMYSL.MEASURE: the query was refused: SMY-E461 a text measure reads only"),
        "{err}"
    );
}

/// SM03 for the ids and the scope.
#[test]
fn measure_operands_are_checked() {
    let (d, _) = naomi("measure-sm03");
    let err = call(
        vec![handle(&d), s("A4"), s("from library")],
        "smysl.measure",
    )
    .0
    .unwrap_err();
    assert_eq!(
        err,
        "SMYSL.MEASURE: #2 parameter must be LIST, found String"
    );
    let err = call(
        vec![handle(&d), list(vec![BundValue::int(4)]), s("from library")],
        "smysl.measure",
    )
    .0
    .unwrap_err();
    assert_eq!(
        err,
        "SMYSL.MEASURE: measure id must be STRING, found Integer"
    );
    let err = call(
        vec![handle(&d), list(vec![]), BundValue::int(4)],
        "smysl.measure",
    )
    .0
    .unwrap_err();
    assert_eq!(
        err,
        "SMYSL.MEASURE: #1 parameter must be STRING, found Integer"
    );
}

/// 18's `--noio` case for M6's words.
#[test]
fn under_noio_m6s_words_are_sm06() {
    let h = || {
        m(&[
            ("type", s("smysl")),
            ("root", s("/nowhere")),
            ("agent", s("tool:bund2")),
        ])
    };
    for (operands, word) in [
        (vec![h(), s("What is Naomi?")], "smysl.dossier"),
        (
            vec![h(), list(vec![s("A4")]), s("from library")],
            "smysl.measure",
        ),
    ] {
        let mut r = Run::new(true);
        assert_eq!(
            r.call(operands, word).unwrap_err(),
            super::words::NOIO,
            "{word}"
        );
    }
}
