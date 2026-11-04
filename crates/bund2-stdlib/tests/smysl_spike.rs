//! **RFC-0014's Q55 spike** — milestone M1, before any `smysl.*` word exists.
//!
//! Q55 asked whether every value §S4–§S7 build can be composed from outside
//! SMYSL's crates. Each test here composes them through SMYSL's public API
//! alone, against a fixture library made in a fresh temporary directory, and
//! runs the operation the words will run: an attested append and its re-open,
//! one query, one dossier, one measure. Nothing here is Bund2 code; a value
//! that could only be reached through a private API would have stopped the
//! milestone, and none had to be.
//!
//! The fixture is the shape of SMYSL's own dossier fixture
//! (`fixtures/dossier/subjects.smy`), cut to three units: an entity, an
//! observation that mentions it, and an inferred claim grounded on the
//! observation that mentions it too.
#![cfg(feature = "smysl")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use smysl_core::schemas::text as xtext;
use smysl_core::{
    AgentId, Attestation, Hlc, KernelType, Op, Record, RelKind, Relation, Rung, SourceKind,
    SourceRef, Status, Uid, UnitCoreBuilder, canonical_uid,
};
use smysl_measure::{Input, Outcome, Recipe};
use smysl_query::dossier::templates;
use smysl_query::{AskOptions, Env, PlanOptions, ask};
use smysl_text::{Library, LibraryScope};

/// The agent every fixture record is attested by.
const AGENT: &str = "tool:bund2-spike";

/// A library root under the system temporary directory, removed on drop, so
/// that nothing is written into the repository tree and a failing assertion
/// still cleans up.
struct Root(PathBuf);

impl Root {
    fn fresh(tag: &str) -> Root {
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "bund2-smysl-spike-{}-{tag}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        Root(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// The scope's name: the root's file stem, as SMYSL's `measure` names it
    /// and RFC-0014 §S5 adopts.
    fn name(&self) -> String {
        self.0
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .expect("a root has a stem")
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The uids the fixture wrote, and the rids of its relations.
struct Written {
    entity: Uid,
    observation: Uid,
    claim: Uid,
    rids: Vec<Uid>,
}

/// Create the fixture library and append its records: three units and three
/// relations, each followed by its own `Record::Attestation`, as §S4's
/// attestation rule says Bund2 will write them (`Op::Authored`, rung
/// `Document` for a cited unit and `Computed` otherwise, hop 0, no parents).
fn fixture(root: &Root) -> Written {
    let agent = AgentId::new(AGENT).expect("the agent matches SMYSL's grammar");

    // `entity` is A-2.4 rule 7's constructor: cited, sourced by the alias
    // (`smysl-core/src/schemas.rs:349-359`).
    let entity = xtext::entity("Naomi", "kjv/ruth", None).expect("an entity");
    let observation = UnitCoreBuilder::new(
        KernelType::Observation,
        "Naomi said: I went out full, and the LORD hath brought me home again empty.",
        Status::Cited,
    )
    .source(SourceRef::new(SourceKind::Doc, "Ruth 1:21"))
    .build()
    .expect("a cited observation with a source");
    let observation_uid = canonical_uid(&observation);
    // `inferred` requires grounds (`smysl-core/src/types/epistemics.rs:78-80`).
    let claim = UnitCoreBuilder::new(
        KernelType::Claim,
        "Naomi came back to Bethlehem with nothing.",
        Status::Inferred,
    )
    .grounds([observation_uid])
    .build()
    .expect("an inferred claim with grounds");

    let entity_uid = canonical_uid(&entity);
    let claim_uid = canonical_uid(&claim);
    let mentions = || RelKind::parse(xtext::MENTIONS).expect("a relation kind");
    let relations = [
        Relation::new(RelKind::Elaborates, claim_uid, observation_uid),
        Relation::new(mentions(), observation_uid, entity_uid),
        Relation::new(mentions(), claim_uid, entity_uid),
    ];

    // One clock per attestation, each after the one before it, as §S4's
    // clock rule asks within one word. Written with `Hlc::new`
    // (`smysl-core/src/types/provenance.rs:33`) at a fixed wall reading and a
    // rising counter, so the fixture is deterministic; the words will take
    // `Hlc::now`, which reads the wall clock (`:55-57`).
    let mut counter = 0u32;
    let mut attest = |uid: Uid, rung: Rung| {
        counter += 1;
        let ts = Hlc::new(1_000, counter, agent.clone());
        Record::Attestation(Attestation::new(uid, agent.clone(), Op::Authored, rung, ts))
    };

    // The `x.text/v1` declaration first, as SMYSL's own text fixtures carry
    // it, so the entity type and `x.text/mentions` are declared.
    let mut records = vec![Record::SchemaDecl(xtext::decl())];
    for (core, rung) in [
        (entity, Rung::Document),
        (observation, Rung::Document),
        (claim, Rung::Computed),
    ] {
        let uid = canonical_uid(&core);
        records.push(Record::Unit(core));
        records.push(attest(uid, rung));
    }
    let mut rids = Vec::new();
    for rel in relations {
        let rid = rel.uid();
        rids.push(rid);
        records.push(Record::Relation(rel));
        records.push(attest(rid, Rung::Computed));
    }

    // `create` makes the marker and the catalogue's directory, then opens
    // (`smysl-text/src/library.rs:288-299`).
    let mut library = Library::create(root.path()).expect("a library is created");
    library
        .append_records(&records)
        .expect("the records are appended");

    Written {
        entity: entity_uid,
        observation: observation_uid,
        claim: claim_uid,
        rids,
    }
}

/// An attested append survives a re-open: each unit, each relation and each
/// attestation the fixture wrote is read back from the logs by `Library::open`,
/// by its `canonical_uid` or its rid.
#[test]
fn an_attested_append_survives_a_reopen() {
    let root = Root::fresh("append");
    let w = fixture(&root);

    let library = Library::open(root.path()).expect("the library re-opens");
    let store = library.store();
    let agent = AgentId::new(AGENT).expect("an agent");
    for uid in [w.entity, w.observation, w.claim] {
        let unit = store.get(&uid).expect("the unit is held after a re-open");
        assert_eq!(canonical_uid(&unit.core), uid);
        let attested = store.attestations_of(&uid);
        assert_eq!(attested.len(), 1, "one attestation per unit");
        let a = attested.iter().next().expect("an attestation");
        assert_eq!(a.agent, agent);
        assert_eq!(a.op, Op::Authored);
        assert_eq!(a.hop, 0);
        assert!(a.parents.is_empty());
    }
    let rung_of = |uid: &Uid| {
        store
            .attestations_of(uid)
            .iter()
            .next()
            .map(|a| a.rung)
            .expect("an attestation")
    };
    assert_eq!(rung_of(&w.observation), Rung::Document);
    assert_eq!(rung_of(&w.claim), Rung::Computed);

    // The claim's grounds are the observation, as written.
    let claim = store.get(&w.claim).expect("the claim");
    assert_eq!(claim.core.grounds, BTreeSet::from([w.observation]));

    // Relations are attested by rid (`smysl-graph/src/store/mod.rs:1460-1470`).
    for rid in &w.rids {
        assert!(store.relation_by_id(rid).is_some(), "the relation is held");
        assert_eq!(
            store.attestations_of(rid).len(),
            1,
            "one attestation per edge"
        );
    }

    // The clocks never go backwards within the agent: every attestation's
    // clock is distinct, so the six advanced one from another.
    let clocks: BTreeSet<Hlc> = [w.entity, w.observation, w.claim]
        .iter()
        .chain(&w.rids)
        .flat_map(|u| store.attestations_of(u).iter().map(|a| a.ts.clone()))
        .collect();
    assert_eq!(clocks.len(), 6);
}

/// Every value §S5–§S7 name composes from outside SMYSL: the scope, the `Env`
/// with both ports, `AskOptions` with its template set and clock, `PlanOptions`,
/// a measure's base `Recipe`, the resolved scope and the `Input` over it with
/// the library. Each is what SMYSL's CLI builds (`smysl:src/main.rs:6305-6315`,
/// `:6451-6456`, `:7361-7366`, `:7438`, `:7448`).
#[test]
fn every_value_the_words_need_composes() {
    let root = Root::fresh("values");
    let _ = fixture(&root);
    let library = Library::open(root.path()).expect("the library opens");
    let agent = AgentId::new(AGENT).expect("an agent");

    let scope = LibraryScope::new(root.name(), &library);
    let env = Env::new(library.store(), &scope)
        .with_times(&scope)
        .with_entities(&scope);
    assert!(env.entities.is_some(), "ask needs the entities port");

    let opts = AskOptions::new(agent.clone())
        .with_templates(templates::SET)
        .with_ts(Hlc::new(0, 0, agent));
    assert_eq!(opts.templates, templates::SET);

    let plan = PlanOptions::default();
    let scope_text = "from library";
    let recipe = Recipe::new("", 0, scope_text);
    assert_eq!(recipe.seed, 0);
    let resolved = smysl_measure::resolve(scope_text, &env, &plan).expect("the scope resolves");
    let input = Input::new(library.store(), &resolved).with_library(&library);
    assert!(input.library.is_some());
    assert_eq!(
        resolved.uids.len(),
        3,
        "the scope is the library's three units"
    );
}

/// One `sq/1` query, parsed, planned and run in §S5's environment
/// (`smysl-query/src/exec.rs:115`), names the claim and, through the
/// traversal, the observation it grounds on.
#[test]
fn a_query_names_the_fixture_units() {
    let root = Root::fresh("query");
    let w = fixture(&root);
    let library = Library::open(root.path()).expect("the library opens");
    let scope = LibraryScope::new(root.name(), &library);
    let env = Env::new(library.store(), &scope)
        .with_times(&scope)
        .with_entities(&scope);
    let opts = PlanOptions::default();

    let text = format!(
        "from store:{}\nwhere kind = claim\nexpand ->grounds",
        root.name()
    );
    let parsed = smysl_query::parse(&text).unwrap_or_else(|d| panic!("{text}: {}", d.message));
    let plan = smysl_query::plan(&parsed.query, &env, &opts)
        .unwrap_or_else(|d| panic!("{text}: {} {}", d[0].code.as_str(), d[0].message));
    let run = smysl_query::run(&plan, &env, &opts)
        .unwrap_or_else(|d| panic!("{text}: {} {}", d[0].code.as_str(), d[0].message));
    let uids: BTreeSet<Uid> = run.answer.uids().into_iter().collect();
    assert_eq!(uids, BTreeSet::from([w.claim, w.observation]));
}

/// One dossier: `ask` resolves the subject through the library's entity port
/// and assembles its slots (`smysl-query/src/dossier/mod.rs:819`). Asserted
/// only as far as is robust: it is a dossier, about the fixture's entity.
#[test]
fn a_dossier_is_asked() {
    let root = Root::fresh("dossier");
    let w = fixture(&root);
    let library = Library::open(root.path()).expect("the library opens");
    let agent = AgentId::new(AGENT).expect("an agent");
    let scope = LibraryScope::new(root.name(), &library);
    let env = Env::new(library.store(), &scope)
        .with_times(&scope)
        .with_entities(&scope);
    let opts = AskOptions::new(agent.clone())
        .with_templates(templates::SET)
        .with_ts(Hlc::new(0, 0, agent));

    let dossier = match ask("What is Naomi?", &env, &opts) {
        Ok(d) => d,
        Err(e) => panic!("ask refused: {e:?}"),
    };
    assert_eq!(dossier.entities, vec![w.entity]);
    assert!(!dossier.slots.is_empty());
    assert!(
        !dossier.records.is_empty(),
        "nothing is saved, but records are built"
    );
}

/// One unit measure: A4, the epistemic profile, needs `UNITS` and no `TEXT`
/// (`smysl-measure/src/graph/epistemic.rs:69-71`), so its `Input` carries no
/// corpus. Over the claim and the observation it counts one `cited` and one
/// `inferred`.
#[test]
fn a_unit_measure_runs() {
    let root = Root::fresh("measure");
    let _ = fixture(&root);
    let library = Library::open(root.path()).expect("the library opens");
    let scope = LibraryScope::new(root.name(), &library);
    let env = Env::new(library.store(), &scope)
        .with_times(&scope)
        .with_entities(&scope);

    let scope_text = "from library\nwhere kind = claim or kind = observation";
    let resolved = smysl_measure::resolve(scope_text, &env, &PlanOptions::default())
        .unwrap_or_else(|d| panic!("{scope_text}: {} {}", d[0].code.as_str(), d[0].message));
    let measure = smysl_measure::find(smysl_measure::ALL, "A4", None).expect("A4 is registered");
    assert!(!measure.needs().contains(smysl_measure::Needs::TEXT));
    let input = Input::new(library.store(), &resolved).with_library(&library);
    let recipe = Recipe::new("", 0, scope_text);

    let mut results = smysl_measure::run(&["A4"], &recipe, &input);
    assert_eq!(results.len(), 1);
    let result = results.remove(0);
    let Outcome::Value(value) = &result.outcome else {
        panic!("A4 did not run: {:?}", result.outcome);
    };
    assert_eq!(value.n.get("units"), Some(&2));
    assert_eq!(
        value.values.get("status:cited"),
        Some(&smysl_measure::Num::Count(1))
    );
    assert_eq!(
        value.values.get("status:inferred"),
        Some(&smysl_measure::Num::Count(1))
    );
    // The gist §S5's `value` is: the id, the headline and the scope.
    let gist = smysl_measure::gist(&result.recipe, value);
    assert!(gist.starts_with("A4"), "{gist}");
}
