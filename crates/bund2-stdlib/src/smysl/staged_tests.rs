//! RFC-0014 M12: `smysl.ingest` and `smysl.commit`, criteria 23 and 28 for
//! `smysl.ingest`, 24, 25, and 22's cases that need a word.
//!
//! Every provider is `bund2-script` (§P8). A commit's refusals are tested on
//! staged values built by hand, which is what a program may give it [R40].

#![cfg(feature = "smysl-model-probe")]

use std::collections::BTreeSet;
use std::path::PathBuf;

use bund2_value::BundValue;
use smysl_core::surface::parse_surface;
use smysl_core::{
    KernelType, Label, LabelBinding, Op, Record, RelKind, Relation, Rung, Status, Uid, UnitCore,
    UnitCoreBuilder, canonical_uid, to_cbor_seq,
};
use smysl_provider::ProviderError;
use smysl_text::Library;

use super::forming_tests::{Dir, Run, get, items, m, s, text, uid};
use super::script::{self, Answer, Probing};

// --- the harness ---------------------------------------------------------------

/// A project holding a library at `lib`, its root canonical, with its own
/// sidecar.
pub(super) struct Project {
    pub(super) dir: Dir,
    pub(super) root: PathBuf,
}

impl Project {
    pub(super) fn new(tag: &str) -> Project {
        let dir = Dir::fresh(tag);
        std::fs::create_dir_all(&dir.0).unwrap();
        let root = dir.0.join("lib");
        Library::create(&root).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        Project { dir, root }
    }

    pub(super) fn sidecar(&self, src: &str) {
        let p = self.dir.0.join(".smysl");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("config.hjson"), src).unwrap();
    }

    pub(super) fn root(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }

    pub(super) fn handle(&self) -> BundValue {
        m(&[
            ("type", s("smysl")),
            ("root", s(&self.root())),
            ("agent", s("tool:bund2")),
        ])
    }

    pub(super) fn open(&self) -> Library {
        Library::open(&self.root).unwrap()
    }

    /// Every log's length and the marker's contents.
    pub(super) fn logs(&self) -> Vec<(PathBuf, u64)> {
        let mut out: Vec<(PathBuf, u64)> = smysl_text::shards::list(&self.root)
            .unwrap()
            .into_iter()
            .map(|(_, p)| {
                let n = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                (p, n)
            })
            .collect();
        let marker = self.root.join(smysl_text::library::MARKER);
        out.push((marker.clone(), std::fs::metadata(&marker).map(|m| m.len()).unwrap_or(0)));
        out
    }
}

pub(super) fn endpoint(host: &str, tag: &str) -> String {
    format!("http://{host}/{tag}-{}", std::process::id())
}

/// One `bund2-script` provider `s` at `endpoint`, `content-ingest` routed to
/// it, and `extra` at the top level.
pub(super) fn config(endpoint: &str, extra: &str) -> String {
    format!(
        r#"{{
  providers: {{
    s: {{
      kind: bund2-script
      endpoint: "{endpoint}"
      model: "script-1"
      context_window: 8192
      max_output: 1024
      structured: json-schema
    }}
  }}
  routing: {{ content-ingest: s }}
  {extra}
}}"#
    )
}

pub(super) const SURFACE: &str = "ingest: { path: surface }";

/// Two surface records, as a model on the surface path answers.
pub(super) const ANSWER: &str = "@claim c/pool { status: speculative }\n~ The pool wait rose.\n\n\
                      @claim c/queue { status: speculative }\n~ The queue grew.\n";

pub(super) fn opts(entries: &[(&str, BundValue)]) -> BundValue {
    m(entries)
}

pub(super) fn ingest(r: &mut Run, p: &Project, input: &str, o: BundValue) -> Result<BundValue, String> {
    r.call(vec![p.handle(), s(input), o], "smysl.ingest")
}

pub(super) fn commit(r: &mut Run, p: &Project, staged: BundValue) -> Result<BundValue, String> {
    r.call(vec![p.handle(), staged], "smysl.commit")
}

/// A staged value built by hand: `records` as SMYSL's CBOR.
fn staged(p: &Project, step: &str, records: &[Record], clean: bool) -> BundValue {
    m(&[
        ("type", s("smysl.staged")),
        ("root", s(&p.root())),
        ("step", s(step)),
        ("granularity", BundValue::nodata()),
        ("records", BundValue::bin(to_cbor_seq(records))),
        ("clean", BundValue::boolean(clean)),
    ])
}

pub(super) fn with(v: &BundValue, key: &str, to: BundValue) -> BundValue {
    let mut map = v.as_map().unwrap().clone();
    map.insert(key.to_string(), to);
    BundValue::map(map)
}

fn claim(gist: &str, status: Status) -> UnitCore {
    UnitCoreBuilder::new(KernelType::Claim, gist, status)
        .build()
        .unwrap()
}

fn grounded(gist: &str, status: Status, grounds: &[Uid]) -> UnitCore {
    UnitCoreBuilder::new(KernelType::Claim, gist, status)
        .grounds(grounds.iter().copied())
        .build()
        .unwrap()
}

/// A uid nothing holds.
fn missing(tag: &str) -> Uid {
    canonical_uid(&claim(&format!("never appended: {tag}"), Status::Speculative))
}

/// Append records straight through SMYSL, as a library a program built.
fn seed(p: &Project, records: &[Record]) {
    let mut lib = p.open();
    lib.append_records(records).unwrap();
}

pub(super) fn uids(v: &BundValue) -> Vec<Uid> {
    items(v).iter().map(|u| uid(&text(u))).collect()
}

// --- criterion 23, for smysl.ingest --------------------------------------------------

/// With `offline`, a provider at a documentation address is SM13 naming
/// `SMY-E301`, before any probe or call; the same at `127.0.0.1` succeeds.
#[test]
fn offline_refuses_a_remote_provider_before_any_probe() {
    let p = Project::new("m12-offline");
    let far = endpoint("192.0.2.1", "m12-offline");
    p.sidecar(&config(&far, SURFACE));
    let mut r = Run::new(false);
    let e = ingest(&mut r, &p, "text", opts(&[("offline", BundValue::boolean(true))])).unwrap_err();
    assert!(e.starts_with("SMYSL.INGEST: the model refused it: SMY-E301"), "{e}");
    assert_eq!(script::probes(&far), 0);
    assert!(script::calls(&far).is_empty());

    let near = endpoint("127.0.0.1", "m12-offline");
    p.sidecar(&config(&near, SURFACE));
    script::arm(&near, [Answer::Text(ANSWER.into())]);
    let v = ingest(&mut r, &p, "text", opts(&[("offline", BundValue::boolean(true))])).unwrap();
    assert_eq!(text(&get(&v, "type")), "smysl.staged");
    script::disarm(&near);
}

// --- criterion 24 ---------------------------------------------------------------

/// `smysl.ingest` stages SMYSL's parse of the answer and writes nothing;
/// `smysl.commit` writes it once, attested by the handle's agent as §P7
/// says; a second commit writes nothing.
#[test]
fn ingest_stages_and_commit_writes() {
    let p = Project::new("m12-write");
    let ep = endpoint("localhost", "m12-write");
    p.sidecar(&config(&ep, SURFACE));
    script::arm(&ep, [Answer::Text(ANSWER.into())]);
    let mut r = Run::new(false);
    // An earlier write by the same agent, whose clock the commit's must pass.
    let earlier = r
        .call(vec![p.handle(), super::forming_tests::unit("an earlier claim")], "smysl.unit")
        .unwrap();
    let earlier = uid(&text(&earlier));
    let before = p.logs();
    let v = ingest(&mut r, &p, "The pool wait rose. The queue grew.", opts(&[])).unwrap();
    assert_eq!(p.logs(), before, "staging wrote nothing");

    let parsed: BTreeSet<Uid> = parse_surface(ANSWER)
        .unwrap()
        .records
        .iter()
        .filter_map(|r| match r {
            Record::Unit(u) => Some(canonical_uid(u)),
            _ => None,
        })
        .collect();
    let staged_uids: BTreeSet<Uid> = items(&get(&v, "units"))
        .iter()
        .map(|u| uid(&text(&get(u, "uid"))))
        .collect();
    assert_eq!(staged_uids, parsed);
    assert_eq!(get(&v, "clean"), BundValue::boolean(true));
    assert_eq!(get(&get(&v, "counts"), "failed").as_int(), Some(0));
    assert_eq!(text(&get(&v, "step")), "ingest");
    let recipe = text(&get(&v, "recipe"));
    assert_eq!(recipe.len(), 64);
    assert!(get(&get(&v, "usage"), "input").as_int().is_some());
    assert_eq!(items(&get(&v, "labels")).len(), 2);

    let written = commit(&mut r, &p, v.clone()).unwrap();
    let written: BTreeSet<Uid> = uids(&written).into_iter().collect();
    assert_eq!(written, parsed);
    for u in &written {
        let got = r
            .call(vec![p.handle(), s(&u.canonical())], "smysl.get")
            .unwrap();
        assert_eq!(text(&get(&got, "uid")), u.canonical());
    }
    let lib = p.open();
    let agent = smysl_core::AgentId::new("tool:bund2").unwrap();
    let mut clocks = Vec::new();
    for u in &written {
        let ours: Vec<_> = lib
            .store()
            .attestations_of(u)
            .iter()
            .filter(|a| a.agent == agent)
            .cloned()
            .collect();
        assert_eq!(ours.len(), 1, "attested once");
        assert_eq!(ours[0].op, Op::Imported);
        assert_eq!(ours[0].rung, Rung::Document);
        let hex: String = ours[0]
            .recipe
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hex, recipe);
        clocks.push(ours[0].ts.clone());
    }
    let mut sorted = clocks.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), clocks.len(), "each clock advances");
    let first = lib
        .store()
        .attestations_of(&earlier)
        .iter()
        .find(|a| a.agent == agent)
        .unwrap()
        .ts
        .clone();
    assert!(clocks.iter().all(|c| *c > first), "no clock goes back");

    // A second commit of the same value answers nothing and attests nothing.
    let count = p.open().store().iter().count();
    let again = commit(&mut r, &p, v.clone()).unwrap();
    assert!(items(&again).is_empty());
    assert_eq!(p.open().store().iter().count(), count);
    let lib = p.open();
    for u in &written {
        let ours = lib
            .store()
            .attestations_of(u)
            .iter()
            .filter(|a| a.agent == agent)
            .count();
        assert_eq!(ours, 1, "the second commit attests nothing");
    }
    script::disarm(&ep);
}

/// A staged value saved and loaded commits the same records: its `records`
/// BIN crosses `bund2_value::wire`, the encoding `save.model` and
/// `load.model` use, whole. Not through the words themselves: they sanitise a
/// world's name to a file in the working directory (`world_file`), which in
/// a test is the crate's own.
#[test]
fn a_staged_value_saved_and_loaded_commits_the_same() {
    let p = Project::new("m12-saved");
    let ep = endpoint("localhost", "m12-saved");
    p.sidecar(&config(&ep, SURFACE));
    script::arm(&ep, [Answer::Text(ANSWER.into())]);
    let mut r = Run::new(false);
    let v = ingest(&mut r, &p, "The pool wait rose.", opts(&[])).unwrap();
    let bytes = bund2_value::wire::to_binary(&v).unwrap();
    let back = bund2_value::wire::from_binary(&bytes).unwrap();
    assert_eq!(get(&back, "records").as_bin(), get(&v, "records").as_bin());
    let written = commit(&mut r, &p, back).unwrap();
    assert_eq!(items(&written).len(), 2);
    script::disarm(&ep);
}

// --- criterion 25 ---------------------------------------------------------------

/// Each is refused, writes nothing, and leaves no lock.
#[test]
fn commit_refuses_what_it_is_not_given_whole() {
    let p = Project::new("m12-refuse");
    let other = Project::new("m12-refuse-other");
    let mut r = Run::new(false);
    let u = claim("a claim", Status::Speculative);
    let good = staged(&p, "ingest", &[Record::Unit(u.clone())], true);
    let before = p.logs();
    let refused = |r: &mut Run, v: BundValue| commit(r, &p, v).unwrap_err();

    let e = refused(&mut r, staged(&other, "ingest", &[Record::Unit(u.clone())], true));
    assert!(e.contains(&format!("staged batch belongs to {}", other.root())), "{e}");
    let e = refused(&mut r, with(&good, "clean", BundValue::boolean(false)));
    assert!(e.ends_with("staged batch did not check clean"), "{e}");
    let e = refused(&mut r, with(&good, "records", s("not a bin")));
    assert!(e.ends_with("records do not decode"), "{e}");
    let bytes = to_cbor_seq(&[Record::Unit(u.clone())]);
    let short = BundValue::bin(bytes[..bytes.len() - 1].to_vec());
    let e = refused(&mut r, with(&good, "records", short));
    assert!(e.ends_with("records do not decode"), "{e}");
    let e = refused(
        &mut r,
        staged(
            &p,
            "ingest",
            &[Record::SchemaDecl(smysl_core::schemas::query::decl())],
            true,
        ),
    );
    assert!(e.contains("which ingest does not make"), "{e}");
    let x = canonical_uid(&u);
    let e = refused(
        &mut r,
        staged(
            &p,
            "ingest",
            &[Record::Unit(u.clone()), Record::Relation(Relation::new(RelKind::Retracts, x, x))],
            true,
        ),
    );
    assert!(e.contains("a retracts relation is written only by smysl.retract"), "{e}");
    let measured = UnitCoreBuilder::new(KernelType::Claim, "measured", Status::Measured)
        .source(smysl_core::SourceRef::new(smysl_core::SourceKind::Doc, "a meter"))
        .build()
        .unwrap();
    let e = refused(&mut r, staged(&p, "ingest", &[Record::Unit(measured)], true));
    assert!(e.contains("status measured cannot be authored"), "{e}");
    let odd = Relation::new(RelKind::Extension("not a kind".into()), x, x);
    let e = refused(
        &mut r,
        staged(&p, "ingest", &[Record::Unit(u.clone()), Record::Relation(odd)], true),
    );
    // SMYSL's decoder refuses a kind `RelKind::parse` refuses, so it never
    // reaches step 5's SM05: it is step 3's refusal.
    assert!(e.ends_with("records do not decode"), "{e}");
    // `clean` set by hand over a batch with an error: a ground nothing holds.
    let bad = grounded("rests on nothing", Status::Speculative, &[missing("clean")]);
    let e = refused(&mut r, staged(&p, "ingest", &[Record::Unit(bad)], true));
    assert!(e.ends_with("staged batch no longer checks clean"), "{e}");

    assert_eq!(p.logs(), before, "nothing was written");
    assert!(
        super::unlock::unlock(&p.root, false).unwrap().is_empty(),
        "no lock was left"
    );
    // An attestation by another agent, placed by hand, is not written.
    let mut lib_attest = smysl_core::Attestation::new(
        x,
        smysl_core::AgentId::new("tool:someone").unwrap(),
        Op::Authored,
        Rung::Computed,
        smysl_core::Hlc::zero(smysl_core::AgentId::new("tool:someone").unwrap()),
    );
    lib_attest.hop = 0;
    let v = staged(&p, "ingest", &[Record::Unit(u.clone()), Record::Attestation(lib_attest)], true);
    commit(&mut r, &p, v).unwrap();
    let agents: Vec<String> = p
        .open()
        .store()
        .attestations_of(&x)
        .iter()
        .map(|a| a.agent.to_string())
        .collect();
    assert_eq!(agents, vec!["tool:bund2".to_string()]);
}

/// R47 and R48: a library that holds a dangling ground still takes commits,
/// with one warning counting its errors; a batch that adds a dangling ground
/// is refused, whether the uid is new or the library's own missing one.
#[test]
fn a_dangling_ground_is_the_librarys_and_a_new_one_is_refused() {
    let p = Project::new("m12-dangling");
    let ep = endpoint("localhost", "m12-dangling");
    p.sidecar(&config(&ep, SURFACE));
    let d = missing("library's");
    seed(&p, &[Record::Unit(grounded("rests on D", Status::Speculative, &[d]))]);
    let mut r = Run::new(false);
    script::arm(&ep, [Answer::Text(ANSWER.into())]);
    let v = ingest(&mut r, &p, "text", opts(&[])).unwrap();
    assert_eq!(get(&v, "clean"), BundValue::boolean(true));
    let written = commit(&mut r, &p, v).unwrap();
    assert_eq!(items(&written).len(), 2);
    assert!(
        r.warnings()
            .iter()
            .any(|w| w.contains("the library holds 1 error(s) of its own")),
        "{:?}",
        r.warnings()
    );

    let other = grounded("rests on E", Status::Speculative, &[missing("new")]);
    let e = commit(&mut r, &p, staged(&p, "ingest", &[Record::Unit(other)], true)).unwrap_err();
    assert!(e.ends_with("staged batch no longer checks clean"), "{e}");
    let same = grounded("rests on D too", Status::Speculative, &[d]);
    let e = commit(&mut r, &p, staged(&p, "ingest", &[Record::Unit(same.clone())], true)).unwrap_err();
    assert!(
        e.ends_with(&format!(
            "{} refers to {}, which the library does not hold",
            canonical_uid(&same).canonical(),
            d.canonical()
        )),
        "{e}"
    );
    script::disarm(&ep);
}

/// R48: a second `SMY-E020` on a library unit that already has one is an
/// error the batch adds.
#[test]
fn a_second_error_of_the_same_code_on_the_same_unit_is_added() {
    let p = Project::new("m12-e020");
    let a = claim("an undeclared neighbour", Status::Speculative);
    let x = claim("the staged unit", Status::Speculative);
    // The display form, as prose names a unit (`smysl-core/src/ids.rs`).
    let prefix = |u: &UnitCore| canonical_uid(u).to_string();
    let l = UnitCoreBuilder::new(KernelType::Claim, "a library unit", Status::Speculative)
        .body(format!("It names {} and {} here", prefix(&a), prefix(&x)))
        .build()
        .unwrap();
    seed(&p, &[Record::Unit(a), Record::Unit(l)]);
    let mut r = Run::new(false);
    let e = commit(&mut r, &p, staged(&p, "ingest", &[Record::Unit(x)], true)).unwrap_err();
    assert!(e.ends_with("staged batch no longer checks clean"), "{e}");
    assert!(
        r.warnings().iter().any(|w| w.contains("SMY-E020")),
        "{:?}",
        r.warnings()
    );
}

/// R46: a staged label the library binds to another unit is dropped with a
/// warning; the unit is written.
#[test]
fn a_label_the_library_binds_elsewhere_is_dropped() {
    let p = Project::new("m12-label");
    let held = claim("held", Status::Speculative);
    let label = Label::new("c/x").unwrap();
    seed(
        &p,
        &[
            Record::Unit(held.clone()),
            Record::LabelBinding(LabelBinding::new(label.clone(), canonical_uid(&held))),
        ],
    );
    let x = claim("new", Status::Speculative);
    let v = staged(
        &p,
        "ingest",
        &[
            Record::Unit(x.clone()),
            Record::LabelBinding(LabelBinding::new(label.clone(), canonical_uid(&x))),
        ],
        true,
    );
    let mut r = Run::new(false);
    let written = commit(&mut r, &p, v).unwrap();
    assert_eq!(uids(&written), vec![canonical_uid(&x)]);
    assert!(
        r.warnings().iter().any(|w| w.contains("label c/x is bound to")),
        "{:?}",
        r.warnings()
    );
    let lib = p.open();
    assert_eq!(
        smysl_graph::labels::label_bindings(lib.store(), &label),
        vec![canonical_uid(&held)]
    );
}

/// R44 B: a value staged before its ground is retracted commits as staged.
#[test]
fn a_retracted_ground_is_not_seen_at_commit() {
    let p = Project::new("m12-retract");
    let mut r = Run::new(false);
    let g = r
        .call(
            vec![
                p.handle(),
                m(&[
                    ("schema", s("claim")),
                    ("gist", s("a ground")),
                    ("status", s("speculative")),
                ]),
            ],
            "smysl.unit",
        )
        .unwrap();
    let g = uid(&text(&g));
    let x = grounded("rests on g", Status::Speculative, &[g]);
    let v = staged(&p, "ingest", &[Record::Unit(x.clone())], true);
    r.call(vec![p.handle(), s(&g.canonical())], "smysl.retract")
        .unwrap();
    let written = commit(&mut r, &p, v).unwrap();
    assert_eq!(uids(&written), vec![canonical_uid(&x)]);
}

/// Order and provenance (§P4 step 7): a unit rule M lowers carries its label
/// to the new uid, where a library binding to the old one drops it; an edge
/// re-written with another weight is refused once its end has moved.
#[test]
fn rule_m_moves_a_unit_and_step_7_judges_the_moved_uid() {
    let p = Project::new("m12-remap");
    let g = missing("ground");
    let u = grounded("U rests on G", Status::Inferred, &[g]);
    let label = Label::new("c/x").unwrap();
    seed(
        &p,
        &[
            Record::Unit(u.clone()),
            Record::LabelBinding(LabelBinding::new(label.clone(), canonical_uid(&u))),
        ],
    );
    // The unit G, whose status is below U's, is then appended: here it is
    // a unit whose uid is G's only if built the same way.
    let g_unit = claim("never appended: ground", Status::Speculative);
    assert_eq!(canonical_uid(&g_unit), g);
    seed(&p, &[Record::Unit(g_unit)]);
    let v = staged(
        &p,
        "ingest",
        &[
            Record::Unit(u.clone()),
            Record::LabelBinding(LabelBinding::new(label.clone(), canonical_uid(&u))),
        ],
        true,
    );
    let mut r = Run::new(false);
    let written = uids(&commit(&mut r, &p, v).unwrap());
    assert_eq!(written.len(), 1);
    assert_ne!(written[0], canonical_uid(&u), "U was lowered to U'");
    let lib = p.open();
    assert_eq!(
        smysl_graph::labels::label_bindings(lib.store(), &label),
        vec![canonical_uid(&u)],
        "c/x still binds one unit"
    );
    assert!(r.warnings().iter().any(|w| w.contains("label c/x is bound to")));

    // An edge U → V with no weight, where the library holds U' → V with one.
    let lowered = written[0];
    let vv = claim("V", Status::Speculative);
    let v_uid = canonical_uid(&vv);
    seed(
        &p,
        &[
            Record::Unit(vv),
            Record::Relation(Relation::new(RelKind::Elaborates, lowered, v_uid).with_weight(0.5)),
        ],
    );
    let edge = staged(
        &p,
        "ingest",
        &[
            Record::Unit(u.clone()),
            Record::Relation(Relation::new(RelKind::Elaborates, canonical_uid(&u), v_uid)),
        ],
        true,
    );
    let e = commit(&mut r, &p, edge).unwrap_err();
    assert!(e.contains("is held with another weight or note"), "{e}");
}

// --- criterion 28, for smysl.ingest -----------------------------------------------

/// The probe refuses before any call: a failed probe, an unreachable one, a
/// model it does not offer, and an empty list.
#[test]
fn the_probe_refuses_before_any_call() {
    let p = Project::new("m12-probe");
    let ep = endpoint("localhost", "m12-probe");
    p.sidecar(&config(&ep, SURFACE));
    let mut r = Run::new(false);
    for (probing, want) in [
        (Probing::Fail(ProviderError::Unreachable), "SMY-E300"),
        (Probing::Unreachable, "is unreachable"),
        (Probing::Models(vec!["other".into()]), "does not offer model script-1"),
        (Probing::Models(Vec::new()), "does not offer model script-1"),
    ] {
        script::arm(&ep, [Answer::Text(ANSWER.into())]);
        script::probing(&ep, probing);
        let e = ingest(&mut r, &p, "text", opts(&[])).unwrap_err();
        assert!(e.starts_with("SMYSL.INGEST: the model refused it: "), "{e}");
        assert!(e.contains(want), "{e}");
        assert!(script::calls(&ep).is_empty(), "no completion before the probe");
    }
    // `opts.model` is the model judged.
    script::arm(&ep, [Answer::Text(ANSWER.into())]);
    script::probing(&ep, Probing::Models(vec!["script-1".into()]));
    let e = ingest(&mut r, &p, "text", opts(&[("model", s("absent"))])).unwrap_err();
    assert!(e.contains("does not offer model absent"), "{e}");
    script::disarm(&ep);
}

/// A failed call makes the staged value unclean, counts it, and keeps
/// SMYSL's `SMY-W304`; the commit refuses it.
#[test]
fn a_failed_call_stages_an_unclean_value() {
    let p = Project::new("m12-failed");
    let ep = endpoint("localhost", "m12-failed");
    p.sidecar(&config(&ep, SURFACE));
    script::arm(
        &ep,
        std::iter::repeat_n(Answer::Fail(ProviderError::Unauthorized), 8),
    );
    let mut r = Run::new(false);
    let v = ingest(&mut r, &p, "The pool wait rose.", opts(&[])).unwrap();
    assert_eq!(get(&v, "clean"), BundValue::boolean(false));
    assert!(get(&get(&v, "counts"), "failed").as_int().unwrap() >= 1);
    assert!(
        items(&get(&v, "diagnostics"))
            .iter()
            .any(|d| text(&get(d, "code")) == "SMY-W304")
    );
    assert!(r.warnings().iter().any(|w| w.contains("model call(s) failed")));
    let e = commit(&mut r, &p, v).unwrap_err();
    assert!(e.ends_with("staged batch did not check clean"), "{e}");
    script::disarm(&ep);
}

// --- criterion 22, the cases that need a word -------------------------------------

/// `opts` refusals, SM03; a granularity SMYSL refuses, SM13; and the
/// configuration's `ingest.temperature` and `ingest.path` give the recipe
/// the same run given both in `opts` gives.
#[test]
fn opts_are_checked_and_the_configuration_fills_what_they_leave_out() {
    let p = Project::new("m12-opts");
    let ep = endpoint("localhost", "m12-opts");
    p.sidecar(&config(&ep, ""));
    let mut r = Run::new(false);
    for (o, want) in [
        (opts(&[("colour", s("blue"))]), "opts has no key colour"),
        (opts(&[("offline", s("yes"))]), "opts offline must be BOOL"),
        (opts(&[("temperature", BundValue::float(2.5))]), "outside 0.0 to 2.0"),
        (opts(&[("path", s("auto"))]), "opts path auto is not surface or json-ast"),
        (opts(&[("level", s("line"))]), "opts has no key level"),
        (opts(&[("config", s("relative.hjson"))]), "is not an absolute path"),
    ] {
        let e = ingest(&mut r, &p, "text", o).unwrap_err();
        assert!(e.starts_with("SMYSL.INGEST: "), "{e}");
        assert!(e.contains(want), "{e}");
    }
    script::arm(&ep, [Answer::Text(ANSWER.into())]);
    let e = ingest(&mut r, &p, "text", opts(&[("granularity", s("bogus"))])).unwrap_err();
    assert!(e.starts_with("SMYSL.INGEST: the model refused it: "), "{e}");

    script::arm(&ep, [Answer::Text(ANSWER.into())]);
    let given = ingest(
        &mut r,
        &p,
        "The pool wait rose.",
        opts(&[
            ("temperature", BundValue::float(0.0)),
            ("path", s("surface")),
        ]),
    )
    .unwrap();
    p.sidecar(&config(
        &ep,
        "ingest: { path: surface, temperature: 0.0 }",
    ));
    script::arm(&ep, [Answer::Text(ANSWER.into())]);
    let filled = ingest(&mut r, &p, "The pool wait rose.", opts(&[])).unwrap();
    assert_eq!(text(&get(&filled, "recipe")), text(&get(&given, "recipe")));
    script::disarm(&ep);
}

/// The prompt override's system text is what the model receives.
#[test]
fn the_prompt_override_reaches_the_model() {
    let p = Project::new("m12-prompt");
    let ep = endpoint("localhost", "m12-prompt");
    p.sidecar(&config(&ep, "ingest: { path: surface, prompt: \"prompts/p.hjson\" }"));
    let prompts = p.root.parent().unwrap().join("prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    std::fs::write(
        prompts.join("p.hjson"),
        "{ id: bund2.m12, version: 1, system: \"the system text the override sets\" }",
    )
    .unwrap();
    script::arm(&ep, [Answer::Text(ANSWER.into())]);
    let mut r = Run::new(false);
    ingest(&mut r, &p, "The pool wait rose.", opts(&[])).unwrap();
    assert_eq!(
        script::calls(&ep)[0].system,
        "the system text the override sets"
    );
    script::disarm(&ep);
}

/// Under `--noio`, the model words are SM06.
#[test]
fn noio_stubs_the_model_words() {
    let p = Project::new("m12-noio");
    let mut r = Run::new(true);
    for (word, operands) in [
        ("smysl.ingest", vec![p.handle(), s("text"), opts(&[])]),
        ("smysl.commit", vec![p.handle(), opts(&[])]),
        ("smysl.answer", vec![p.handle(), s("why?"), opts(&[])]),
        ("smysl.ingest.text", vec![p.handle(), s("t"), opts(&[])]),
    ] {
        let e = r.call(operands, word).unwrap_err();
        assert!(e.contains(super::words::NOIO), "{word}: {e}");
    }
}

/// A script that panics makes the word `Error::internal` (D49), and
/// `smysl.ingest`, which holds no lock, leaves none.
#[test]
fn a_panicking_model_is_an_internal_error_and_leaves_no_lock() {
    let p = Project::new("m12-panic");
    let ep = endpoint("localhost", "m12-panic");
    p.sidecar(&config(&ep, SURFACE));
    script::arm(&ep, [Answer::Panic]);
    let mut r = Run::new(false);
    let e = ingest(&mut r, &p, "text", opts(&[])).unwrap_err();
    assert!(e.starts_with("internal error: native `smysl.ingest` panicked: "), "{e}");
    assert!(e.contains("bund2-script: the armed panic"), "{e}");
    assert!(super::unlock::unlock(&p.root, false).unwrap().is_empty());
    script::disarm(&ep);
}
