//! RFC-0014 M13: `smysl.answer`, and `smysl.commit` of what it stages:
//! criterion 26, 25's answer cases, and 22's, 23's and 28's for
//! `smysl.answer`.
//!
//! Every provider is `bund2-script` (§P8). The script's answers quote the
//! dossier's uids in full, as SMYSL's answer template asks; the question's
//! uid does not depend on the dossier's clock, so the test assembles the
//! same dossier with the function the word uses to learn it.

#![cfg(feature = "smysl-model-probe")]

use bund2_value::BundValue;
use smysl_core::schemas::text as xtext;
use smysl_core::{
    AgentId, Code, KernelType, Op, Record, RelKind, Relation, Rung, SourceKind, SourceRef, Status,
    Uid, UnitCoreBuilder, canonical_uid, from_cbor_seq, to_cbor_seq,
};
use smysl_provider::ProviderError;

use super::forming_tests::{Run, get, items, s, text};
use super::script::{self, Answer, Probing};
use super::staged_tests::{
    ANSWER, Project, SURFACE, commit, config, endpoint, ingest, opts, uids, with,
};

// --- the fixture ---------------------------------------------------------------------

const QUESTION: &str = "What is Naomi?";

/// The library's units the dossier finds: Naomi's entity, a `cited`
/// observation, an `inferred` claim over it, and a `speculative` guess;
/// the claim and the guess fill the `characteristic` slot.
struct Naomi {
    p: Project,
    ep: String,
    entity: Uid,
    guess: Uid,
}

/// A library that checks clean, holding Naomi, with its sidecar routing
/// `content-ingest` to a script at `127.0.0.1`.
fn naomi(tag: &str) -> Naomi {
    let p = Project::new(tag);
    let ep = endpoint("127.0.0.1", tag);
    p.sidecar(&config(&ep, SURFACE));
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
    let guess = UnitCoreBuilder::new(
        KernelType::Claim,
        "Naomi may have sold her field before she left.",
        Status::Speculative,
    )
    .build()
    .unwrap();
    let (e, o, c, g) = (
        canonical_uid(&entity),
        canonical_uid(&observation),
        canonical_uid(&claim),
        canonical_uid(&guess),
    );
    let mentions = || RelKind::parse(xtext::MENTIONS).unwrap();
    p.open()
        .append_records(&[
            Record::SchemaDecl(xtext::decl()),
            Record::Unit(entity),
            Record::Unit(observation),
            Record::Unit(claim),
            Record::Unit(guess),
            Record::Relation(Relation::new(RelKind::Elaborates, c, o)),
            Record::Relation(Relation::new(mentions(), o, e)),
            Record::Relation(Relation::new(mentions(), c, e)),
            Record::Relation(Relation::new(mentions(), g, e)),
        ])
        .unwrap();
    let errors = smysl_check::check(p.open().store(), smysl_check::CheckOptions::default())
        .diagnostics
        .into_iter()
        .filter(|d| d.severity.as_str() == "error")
        .count();
    assert_eq!(errors, 0, "the fixture checks clean");
    Naomi {
        p,
        ep,
        entity: e,
        guess: g,
    }
}

impl Naomi {
    /// The question unit's uid, from the dossier the word assembles.
    fn question(&self, q: &str) -> Uid {
        let lib = self.p.open();
        let agent = AgentId::new("tool:bund2").unwrap();
        super::research::assemble(&lib, &self.p.root, &agent, q)
            .unwrap()
            .question
    }

    fn answer(&self, r: &mut Run, q: &str, o: BundValue) -> Result<BundValue, String> {
        r.call(vec![self.p.handle(), s(q), o], "smysl.answer")
    }
}

/// One answer record, as SMYSL's template shapes it.
fn record(label: &str, status: &str, grounds: &[Uid], gist: &str) -> String {
    let grounds: Vec<String> = grounds.iter().map(Uid::canonical).collect();
    format!(
        "@claim a/{label} {{ status: {status}, grounds: [{}] }}\n~ {gist}\n",
        grounds.join(", ")
    )
}

fn records_of(v: &BundValue) -> Vec<Record> {
    let bytes = get(v, "records").as_bin().unwrap().to_vec();
    from_cbor_seq(&bytes).unwrap().0
}

fn with_records(v: &BundValue, records: &[Record]) -> BundValue {
    with(v, "records", BundValue::bin(to_cbor_seq(records)))
}

fn codes(v: &BundValue) -> Vec<String> {
    items(&get(v, "diagnostics"))
        .iter()
        .map(|d| text(&get(d, "code")))
        .collect()
}

fn count(v: &BundValue, k: &str) -> i64 {
    get(&get(v, "counts"), k).as_int().unwrap()
}

fn declares_query(records: &[Record]) -> bool {
    records.iter().any(
        |r| matches!(r, Record::SchemaDecl(d) if d.id.as_str() == smysl_core::schemas::query::ID),
    )
}

fn unlocked(p: &Project) -> bool {
    super::unlock::unlock(&p.root, false).unwrap().is_empty()
}

// --- criterion 26 ---------------------------------------------------------------------

/// The answer keeps what grounds in the dossier, lowers what rests on the
/// `speculative` member, and is written with its dossier: the answer's
/// units `Imported` at `Model`, the dossier's `Authored` (§P7, R50).
#[test]
fn an_answer_is_grounded_on_its_dossier_and_written_with_it() {
    let n = naomi("m13-answer");
    let q = n.question(QUESTION);
    let reply = [
        record("back-empty", "inferred", &[q], "Naomi came back with nothing."),
        record("outside", "inferred", &[n.entity], "Naomi is an entity."),
        record("sold", "inferred", &[n.guess], "Naomi sold her field."),
    ]
    .join("\n");
    script::arm(&n.ep, [Answer::Text(reply)]);
    let mut r = Run::new(false);
    let before = n.p.logs();
    let v = n.answer(&mut r, QUESTION, opts(&[])).unwrap();
    assert_eq!(n.p.logs(), before, "staging wrote nothing");
    assert_eq!(text(&get(&v, "step")), "answer");
    assert_eq!(get(&v, "granularity"), BundValue::nodata());
    assert_eq!(get(&v, "clean"), BundValue::boolean(true), "{:?}", codes(&v));
    assert_eq!(count(&v, "proposed"), 3);
    assert_eq!(count(&v, "dropped"), 1);
    assert_eq!(count(&v, "lowered"), 1);
    assert_eq!(count(&v, "capped"), 0);
    let cs = codes(&v);
    assert!(cs.iter().any(|c| c == "SMY-W478"), "{cs:?}");
    assert!(cs.iter().any(|c| c == "SMY-W036"), "{cs:?}");
    let staged = records_of(&v);
    assert!(declares_query(&staged), "the library did not declare x.query/v1");
    assert!(staged.iter().any(|r| matches!(r, Record::Thread(_))));
    assert!(staged.iter().any(|r| matches!(r, Record::View(_))));

    let written = uids(&commit(&mut r, &n.p, v.clone()).unwrap());
    assert!(written.contains(&q), "the question is written");
    let lib = n.p.open();
    let store = lib.store();
    let agent = AgentId::new("tool:bund2").unwrap();
    let ours = |u: &Uid| {
        store
            .attestations_of(u)
            .iter()
            .filter(|a| a.agent == agent)
            .cloned()
            .collect::<Vec<_>>()
    };
    let root_set: std::collections::BTreeSet<Uid> = staged
        .iter()
        .filter_map(|r| match r {
            Record::View(view) => Some(view.roots.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    let mut answers = 0;
    for u in &written {
        let a = ours(u);
        assert_eq!(a.len(), 1, "{u} attested once");
        if root_set.contains(u) {
            assert_eq!(a[0].op, Op::Authored, "the dossier's {u}");
            assert_eq!(a[0].rung, Rung::Document);
        } else {
            answers += 1;
            assert_eq!(a[0].op, Op::Imported, "the answer's {u}");
            assert_eq!(a[0].rung, Rung::Model);
            let unit = store.get(u).unwrap();
            for g in &unit.core.grounds {
                assert!(store.contains_uid(g), "{u} grounds on {g}, held");
            }
        }
    }
    assert_eq!(answers, 2);
    assert!(store.threads().count() >= 1 && store.views().count() >= 1);

    // The library now declares x.query/v1: a second answer does not again.
    script::arm(
        &n.ep,
        [Answer::Text(record("again", "inferred", &[q], "Again."))],
    );
    let again = n.answer(&mut r, QUESTION, opts(&[])).unwrap();
    assert!(!declares_query(&records_of(&again)));
    script::disarm(&n.ep);
}

/// R45: a `derived` answer is staged at `inferred`, its `SMY-E033` among the
/// diagnostics and `clean` still true; once written, a later ingest commits
/// and the library checks with no `SMY-E033`.
#[test]
fn a_derived_answer_is_capped_at_the_model_ceiling() {
    let n = naomi("m13-capped");
    let q = n.question(QUESTION);
    script::arm(
        &n.ep,
        [
            Answer::Text(record("derived", "derived", &[q], "She came back empty.")),
            Answer::Text(ANSWER.into()),
        ],
    );
    let mut r = Run::new(false);
    let v = n.answer(&mut r, QUESTION, opts(&[])).unwrap();
    assert_eq!(get(&v, "clean"), BundValue::boolean(true), "{:?}", codes(&v));
    assert_eq!(count(&v, "capped"), 1);
    assert!(codes(&v).iter().any(|c| c == "SMY-E033"));
    assert!(r.warnings().iter().any(|w| w.contains("SMY-E033")));
    let capped: Vec<_> = items(&get(&v, "units"))
        .iter()
        .filter(|u| text(&get(u, "gist")) == "She came back empty.")
        .map(|u| text(&get(u, "status")))
        .collect();
    assert_eq!(capped, ["inferred"]);
    commit(&mut r, &n.p, v).unwrap();

    let staged = ingest(&mut r, &n.p, "The pool wait rose.", opts(&[])).unwrap();
    commit(&mut r, &n.p, staged).unwrap();
    let report = smysl_check::check(n.p.open().store(), smysl_check::CheckOptions::default());
    assert!(
        !report.diagnostics.iter().any(|d| d.code == Code::E033),
        "{:?}",
        report.diagnostics
    );
    script::disarm(&n.ep);
}

/// A question longer than the library's profile allows a gist stages
/// `clean` false, with `SMY-E022` on the question unit, and is refused.
#[test]
fn a_question_too_long_for_the_profile_is_not_clean() {
    let n = naomi("m13-long");
    let long = "What is Naomi, the widow of Elimelech of Bethlehem-judah, who went out \
                full in the days when the judges ruled, and whom the LORD brought home \
                again empty, with Ruth the Moabitess her daughter in law, at the \
                beginning of the barley harvest?";
    let q = n.question(long);
    script::arm(
        &n.ep,
        [Answer::Text(record("long", "inferred", &[q], "She came back."))],
    );
    let mut r = Run::new(false);
    let v = n.answer(&mut r, long, opts(&[])).unwrap();
    assert_eq!(get(&v, "clean"), BundValue::boolean(false));
    let e022: Vec<_> = items(&get(&v, "diagnostics"))
        .into_iter()
        .filter(|d| text(&get(d, "code")) == "SMY-E022")
        .collect();
    assert!(!e022.is_empty());
    let before = n.p.logs();
    let e = commit(&mut r, &n.p, v).unwrap_err();
    assert!(e.ends_with("staged batch did not check clean"), "{e}");
    assert_eq!(n.p.logs(), before);
    script::disarm(&n.ep);
}

/// An answer that does not parse stages nothing: empty records, `clean`
/// false, refused at commit, and the library gains no dossier.
#[test]
fn an_answer_that_does_not_parse_stages_nothing() {
    let n = naomi("m13-unparsed");
    script::arm(&n.ep, [Answer::Text("this is { not surface".into())]);
    let mut r = Run::new(false);
    let v = n.answer(&mut r, QUESTION, opts(&[])).unwrap();
    assert!(records_of(&v).is_empty());
    for k in ["units", "relations", "labels"] {
        assert!(items(&get(&v, k)).is_empty(), "{k}");
    }
    assert_eq!(get(&v, "clean"), BundValue::boolean(false));
    assert!(codes(&v).iter().any(|c| c == "SMY-E001"));
    let before = n.p.logs();
    let e = commit(&mut r, &n.p, v).unwrap_err();
    assert!(e.ends_with("staged batch did not check clean"), "{e}");
    assert_eq!(n.p.logs(), before);
    assert_eq!(n.p.open().store().views().count(), 0, "no dossier");
    script::disarm(&n.ep);
}

// --- criterion 25, an answer's cases ------------------------------------------------------

/// Each edit is refused, writes nothing, and leaves no lock.
#[test]
fn commit_refuses_an_edited_answer() {
    let n = naomi("m13-refuse");
    let q = n.question(QUESTION);
    script::arm(
        &n.ep,
        [Answer::Text(record("back-empty", "inferred", &[q], "She came back."))],
    );
    let mut r = Run::new(false);
    let v = n.answer(&mut r, QUESTION, opts(&[])).unwrap();
    let records = records_of(&v);
    let before = n.p.logs();
    let refused = |r: &mut Run, edited: Vec<Record>| {
        let e = commit(r, &n.p, with_records(&v, &edited)).unwrap_err();
        assert_eq!(n.p.logs(), before, "nothing was written: {e}");
        assert!(unlocked(&n.p), "no lock was left: {e}");
        e
    };

    // An answer unit edited to `derived` (R45).
    let edited: Vec<Record> = records
        .iter()
        .map(|rec| match rec {
            Record::Unit(u) if u.gist == "She came back." => {
                let mut b = UnitCoreBuilder::new(u.schema.clone(), &u.gist, Status::Derived)
                    .grounds(u.grounds.iter().copied());
                if let Some(s) = &u.source {
                    b = b.source(s.clone());
                }
                Record::Unit(b.build().unwrap())
            }
            other => other.clone(),
        })
        .collect();
    let e = refused(&mut r, edited);
    assert!(e.contains("exceeds the model ceiling"), "{e}");

    // A `Thread` or a `View` edited to name a uid nothing holds.
    let nowhere = canonical_uid(
        &UnitCoreBuilder::new(KernelType::Claim, "never held", Status::Speculative)
            .build()
            .unwrap(),
    );
    let edited: Vec<Record> = records
        .iter()
        .map(|rec| match rec {
            Record::Thread(t) => {
                let mut t = t.clone();
                t.steps[0].unit = nowhere;
                Record::Thread(t)
            }
            other => other.clone(),
        })
        .collect();
    let e = refused(&mut r, edited);
    assert!(e.contains("thread dossier/"), "{e}");
    assert!(e.contains(&format!("names {}, which the library does not hold", nowhere.canonical())), "{e}");
    let edited: Vec<Record> = records
        .iter()
        .map(|rec| match rec {
            Record::View(view) => {
                let mut view = view.clone();
                view.roots.insert(nowhere);
                Record::View(view)
            }
            other => other.clone(),
        })
        .collect();
    let e = refused(&mut r, edited);
    assert!(e.contains("view dossier/"), "{e}");

    // A `View` whose granularity was edited.
    let edited: Vec<Record> = records
        .iter()
        .map(|rec| match rec {
            Record::View(view) => {
                let mut view = view.clone();
                view.granularity.l0_max += 1;
                Record::View(view)
            }
            other => other.clone(),
        })
        .collect();
    let e = refused(&mut r, edited);
    assert!(e.contains("is not bounded as a dossier's view is"), "{e}");

    // A dossier unit rule M moves: the question, grounded by hand on the
    // `speculative` guess, and named so by the `Thread` and the `View`.
    let mut moved = None;
    let edited: Vec<Record> = records
        .iter()
        .filter_map(|rec| match rec {
            Record::Unit(u) if canonical_uid(u) == q => {
                let mut u = u.clone();
                u.grounds.insert(n.guess);
                moved = Some(canonical_uid(&u));
                Some(Record::Unit(u))
            }
            Record::Unit(u) if u.gist == "She came back." => None,
            Record::LabelBinding(_) | Record::Attestation(_) => None,
            other => Some(other.clone()),
        })
        .collect();
    let moved = moved.unwrap();
    let edited: Vec<Record> = edited
        .into_iter()
        .map(|rec| match rec {
            Record::Thread(mut t) => {
                for s in &mut t.steps {
                    if s.unit == q {
                        s.unit = moved;
                    }
                }
                Record::Thread(t)
            }
            Record::View(mut view) => {
                view.roots.remove(&q);
                view.roots.insert(moved);
                Record::View(view)
            }
            other => other,
        })
        .collect();
    let e = refused(&mut r, edited);
    assert!(e.ends_with("staged dossier changed under the library"), "{e}");

    // Committed whole, then a second answer with `step` edited to "ingest",
    // over a library that now declares x.query/v1: step 4 names its Thread.
    commit(&mut r, &n.p, v).unwrap();
    script::arm(
        &n.ep,
        [Answer::Text(record("again", "inferred", &[q], "Again."))],
    );
    let again = n.answer(&mut r, QUESTION, opts(&[])).unwrap();
    let e = commit(&mut r, &n.p, with(&again, "step", s("ingest"))).unwrap_err();
    assert!(e.contains("is a Thread, which ingest does not make"), "{e}");
    script::disarm(&n.ep);
}

// --- criteria 22, 23 and 28, for smysl.answer ---------------------------------------------

/// `granularity` is not `smysl.answer`'s, SM03.
#[test]
fn an_answer_takes_no_granularity() {
    let n = naomi("m13-opts");
    let mut r = Run::new(false);
    let e = n
        .answer(&mut r, QUESTION, opts(&[("granularity", s("fine"))]))
        .unwrap_err();
    assert!(e.starts_with("SMYSL.ANSWER: "), "{e}");
    assert!(e.contains("opts has no key granularity"), "{e}");
}

/// With `offline`, a provider at a documentation address is SM13 naming
/// `SMY-E301`, before any probe or call.
#[test]
fn offline_refuses_a_remote_provider_for_an_answer() {
    let n = naomi("m13-offline");
    let far = endpoint("192.0.2.1", "m13-offline");
    n.p.sidecar(&config(&far, SURFACE));
    let mut r = Run::new(false);
    let e = n
        .answer(&mut r, QUESTION, opts(&[("offline", BundValue::boolean(true))]))
        .unwrap_err();
    assert!(e.starts_with("SMYSL.ANSWER: the model refused it: SMY-E301"), "{e}");
    assert_eq!(script::probes(&far), 0);
    assert!(script::calls(&far).is_empty());
}

/// A probe that refuses makes the word SM13 before any completion; an error
/// the model's call returns is SM13 with its `Display`, its code where it
/// has one (§P6).
#[test]
fn the_models_errors_are_sm13_for_an_answer() {
    let n = naomi("m13-errors");
    let mut r = Run::new(false);
    script::arm(&n.ep, [Answer::Text(String::new())]);
    script::probing(&n.ep, Probing::Unreachable);
    let e = n.answer(&mut r, QUESTION, opts(&[])).unwrap_err();
    assert!(e.contains("is unreachable"), "{e}");
    assert!(script::calls(&n.ep).is_empty());

    for err in [
        ProviderError::Unreachable,
        ProviderError::Unauthorized,
        ProviderError::Truncated {
            limit: Some(64),
            used: Some(64),
        },
        ProviderError::Config("the script says no".into()),
    ] {
        let shown = err.to_string();
        let code = err.code().map(|c| c.as_str().to_string());
        script::arm(&n.ep, std::iter::repeat_n(Answer::Fail(err), 8));
        let e = n.answer(&mut r, QUESTION, opts(&[])).unwrap_err();
        assert!(e.starts_with("SMYSL.ANSWER: the model refused it: "), "{e}");
        assert!(e.contains(&shown), "{e}");
        if let Some(code) = code {
            assert!(e.contains(&code), "{e}");
        }
    }
    script::disarm(&n.ep);
}

/// A script that panics makes the word `Error::internal` (D49), and
/// `smysl.answer`, which holds no lock, leaves none.
#[test]
fn a_panicking_model_is_an_internal_error_for_an_answer() {
    let n = naomi("m13-panic");
    script::arm(&n.ep, [Answer::Panic]);
    let mut r = Run::new(false);
    let e = n.answer(&mut r, QUESTION, opts(&[])).unwrap_err();
    assert!(e.starts_with("internal error: native `smysl.answer` panicked: "), "{e}");
    assert!(unlocked(&n.p));
    script::disarm(&n.ep);
}
