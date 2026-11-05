//! RFC-0014 M4's criteria: the rest of forming (§S4), driven through the
//! words, with SMYSL's API as the second opinion.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bund2_api::Vm as _;
use bund2_api::diag::{Diagnostic, Reporter, Severity};
use bund2_interp::Interp;
use bund2_value::BundValue;
use smysl_core::{AgentId, Hlc, LabelBinding, Record, RelKind, Relation, Uid, Withdrawal};
use smysl_text::Library;

use super::write::LEFT_LOCKED;

// --- the harness ---------------------------------------------------------------

/// A fresh directory under the system's temporary one, removed on drop.
pub(super) struct Dir(pub(super) PathBuf);

impl Dir {
    pub(super) fn fresh(tag: &str) -> Dir {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "bund2-smysl-m4-{}-{tag}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&p);
        Dir(p)
    }

    /// A fresh library, its root canonical.
    pub(super) fn library(tag: &str) -> Dir {
        let d = Dir::fresh(tag);
        Library::create(&d.0).unwrap();
        let canon = std::fs::canonicalize(&d.0).unwrap();
        // The same directory, under its canonical name: `d` must not
        // remove it on the way out.
        std::mem::forget(d);
        Dir(canon)
    }

    pub(super) fn root(&self) -> &str {
        self.0.to_str().unwrap()
    }

    pub(super) fn open(&self) -> Library {
        Library::open(&self.0).unwrap()
    }

    pub(super) fn catalog_len(&self) -> u64 {
        std::fs::metadata(self.0.join(smysl_text::library::CATALOG))
            .map(|m| m.len())
            .unwrap_or(0)
    }

    pub(super) fn no_lock(&self) -> bool {
        super::unlock::unlock(&self.0, false).unwrap().is_empty()
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Keeps every warning, so a test can assert what was said.
struct Kept(Rc<RefCell<Vec<String>>>);

impl Reporter for Kept {
    fn report(&mut self, d: &Diagnostic) {
        if d.severity == Severity::Warning {
            self.0.borrow_mut().push(d.reason.clone());
        }
    }
}

/// One VM, its warnings kept.
pub(super) struct Run {
    i: Interp,
    said: Rc<RefCell<Vec<String>>>,
}

impl Run {
    pub(super) fn new(noio: bool) -> Run {
        let said = Rc::new(RefCell::new(Vec::new()));
        let mut i = Interp::new();
        i.reporter = Box::new(Kept(Rc::clone(&said)));
        crate::register_all_with(
            &mut i.registry,
            &crate::host::HostOptions {
                noio,
                ..Default::default()
            },
        );
        Run { i, said }
    }

    /// Push `operands`, run `word`, and answer the top of the stack (NODATA
    /// when the word leaves nothing) or the word's own error text.
    pub(super) fn call(
        &mut self,
        operands: Vec<BundValue>,
        word: &str,
    ) -> Result<BundValue, String> {
        let before = self.i.snapshot().len();
        for v in operands {
            self.i.push(v);
        }
        let code = bund2_syntax::compile(word).map_err(|e| format!("{e:?}"))?;
        self.i.eval(&code).map_err(|e| {
            e.0.rsplit_once(" returned error: ")
                .map_or(e.0.clone(), |(_, m)| m.to_string())
        })?;
        let now = self.i.snapshot();
        if now.len() > before {
            Ok(now.last().cloned().unwrap())
        } else {
            Ok(BundValue::nodata())
        }
    }

    pub(super) fn warnings(&self) -> Vec<String> {
        self.said.borrow().clone()
    }
}

/// `word` over `operands`, in a fresh VM: its answer and its warnings.
pub(super) fn call(
    operands: Vec<BundValue>,
    word: &str,
) -> (Result<BundValue, String>, Vec<String>) {
    let mut r = Run::new(false);
    let out = r.call(operands, word);
    (out, r.warnings())
}

pub(super) fn s(v: &str) -> BundValue {
    BundValue::str(v)
}

pub(super) fn m(entries: &[(&str, BundValue)]) -> BundValue {
    BundValue::map(
        entries
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
    )
}

pub(super) fn list(items: Vec<BundValue>) -> BundValue {
    BundValue::list(items)
}

pub(super) fn handle(d: &Dir) -> BundValue {
    handle_as(d, "tool:bund2")
}

pub(super) fn handle_as(d: &Dir, agent: &str) -> BundValue {
    m(&[
        ("type", s("smysl")),
        ("root", s(d.root())),
        ("agent", s(agent)),
    ])
}

pub(super) fn unit(gist: &str) -> BundValue {
    m(&[
        ("schema", s("claim")),
        ("gist", s(gist)),
        ("status", s("speculative")),
    ])
}

pub(super) fn rel(kind: &str, from: &str, to: &str) -> BundValue {
    m(&[("kind", s(kind)), ("from", s(from)), ("to", s(to))])
}

pub(super) fn text(v: &BundValue) -> String {
    v.as_str().unwrap()
}

pub(super) fn get(v: &BundValue, k: &str) -> BundValue {
    v.as_map().unwrap()[k].clone()
}

pub(super) fn items(v: &BundValue) -> Vec<BundValue> {
    v.as_list().unwrap().to_vec()
}

pub(super) fn uid(v: &str) -> Uid {
    Uid::parse(v).unwrap()
}

/// `smysl.unit` of `u` as `tool:bund2`, answering its uid.
pub(super) fn add_unit(d: &Dir, u: BundValue) -> String {
    text(&call(vec![handle(d), u], "smysl.unit").0.unwrap())
}

/// The ids `smysl.append` answers, as a set of STRINGs.
pub(super) fn appended(d: &Dir, maps: Vec<BundValue>) -> Result<BTreeSet<String>, String> {
    call(vec![handle(d), list(maps)], "smysl.append")
        .0
        .map(|v| items(&v).iter().map(text).collect())
}

fn parsed(doc: &str) -> (Result<Vec<BundValue>, String>, Vec<String>) {
    let (out, warnings) = call(vec![s(doc)], "smysl.parse");
    (out.map(|v| items(&v)), warnings)
}

fn written(d: &Dir) -> (String, Vec<String>) {
    let (out, warnings) = call(vec![handle(d)], "smysl.write");
    (text(&out.unwrap()), warnings)
}

/// Append `records` through SMYSL's API, under no lock: the fixture's own
/// writer, before any word runs.
pub(super) fn raw(d: &Dir, records: &[Record]) {
    d.open().append_records(records).unwrap();
}

pub(super) fn agent(a: &str) -> AgentId {
    AgentId::new(a).unwrap()
}

/// A withdrawal of `rid` by `by`, its clock `clock`'s.
pub(super) fn withdrawal(rid: Uid, by: &str, clock: &str) -> Withdrawal {
    let c = agent(clock);
    Withdrawal::new(rid, agent(by), Hlc::now(&Hlc::zero(c.clone()), &c))
}

pub(super) fn rid(kind: RelKind, from: &str, to: &str) -> Uid {
    Relation::new(kind, uid(from), uid(to)).uid()
}

// --- criterion 5: the surface round-trips through a library ----------------------

const STATUSES: &str = r#"@claim c/maybe { status: speculative }
~ The well may be dry.

@observation o/bucket { status: cited, source: { kind: doc, ref: "Log 1" } }
~ The bucket came up empty.

@claim c/dry { status: inferred, grounds: [o/bucket] }
~ The well is dry.

@claim c/nobody { status: derived, grounds: [o/bucket], n: 3, tags: ["dry", "well"], ratio: 0.5 }
~ Nobody drew water today.
"#;

const KERNEL_ONE: &str = r#"@claim c/a { status: speculative }
~ The river rose.

@claim c/b { status: speculative }
~ The bridge closed.

@claim c/c { status: speculative }
~ The market moved upstream.

@claim c/d { status: speculative }
~ Traders took the ford.

@rel c/b --elaborates--> c/a
@rel c/c --contrasts--> c/d
@rel c/d --concedes--> c/c
@rel c/a --causes--> c/b
@rel c/b --enables--> c/c
@rel c/d --exemplifies--> c/c
@rel c/a --conditions--> c/d { weight: 0.25 }
"#;

const KERNEL_TWO: &str = r#"@schema x.text/v1 { version: 1, types: ["x.text/entity"], relations: [x.text/mentions] }

@question q/why { status: speculative }
~ Why did the bridge close?

@claim c/flood { status: speculative }
~ The flood closed the bridge.

@claim c/repair { status: speculative }
~ A repair closed the bridge.

@claim c/repair2 { status: speculative }
~ A scheduled repair closed the bridge.

@definition d/ford { status: cited, source: { kind: doc, ref: "Gazetteer" } }
~ A ford is a shallow place to cross a river.

@rel c/flood --sequences--> c/repair
@rel c/flood --answers--> q/why
@rel c/repair --rebuts--> c/flood { weight: 0.75 }
@rel d/ford --warrant--> c/flood
@rel d/ford --backs--> c/repair
@rel c/repair2 --supersedes--> c/repair
@rel c/flood --x.text/mentions--> d/ford
"#;

const HEADED: &str = r#"@doc smysl/0.1 {
  id: v/bridge
  intent: incident-brief
  lang: en
  requires: ["smysl.kernel/0.1"]
  granularity: { profile: default }
  roots: [f/closed]
}

@claim c/water { status: speculative }
~ Water reached the deck.

@finding f/closed { status: inferred, grounds: [c/water] }
~ The bridge was closed for safety.

@thread t/brief { schema: brief, owner: "human:ann" }
~ The bridge closed because water reached the deck.
  bottom-line → f/closed
  support → c/water
"#;

const DEPS: &str = r#"@definition d/deck { status: cited, source: { kind: doc, ref: "Bridge plan" }, lang: en }
~ The deck is the surface the traffic crosses.

@claim c/deck-wet { status: speculative, deps: [d/deck] }
~ The deck was wet.

Rain fell all night, and the gutters were blocked.
--
The night watch logged it twice.

@rel c/deck-wet --elaborates--> d/deck { weight: 1 }
"#;

/// Criterion 5: each document, parsed and appended into a fresh library,
/// written, then parsed and appended into a second, answers the same ids as
/// a set; no unit MAP has `labels`, and each label appears once.
#[test]
fn the_surface_round_trips_through_a_library() {
    for (n, doc) in [STATUSES, KERNEL_ONE, KERNEL_TWO, HEADED, DEPS]
        .iter()
        .enumerate()
    {
        let (maps, warnings) = parsed(doc);
        let maps = maps.unwrap_or_else(|e| panic!("document {n}: {e}"));
        let mut labels = BTreeSet::new();
        for v in &maps {
            assert!(!v.has_key("labels"), "document {n}: {}", v.display());
            if v.has_key("label") {
                assert!(labels.insert(text(&get(v, "label"))), "document {n}");
            }
        }
        let first = Dir::library("rt1");
        let a = appended(&first, maps).unwrap_or_else(|e| panic!("document {n}: {e}"));
        let (surface, w) = written(&first);
        assert!(w.is_empty(), "document {n}: {w:?}");
        let (again, _) = parsed(&surface);
        let second = Dir::library("rt2");
        let b = appended(&second, again.unwrap()).unwrap();
        assert_eq!(a, b, "document {n}:\n{surface}");

        // The skipped kinds draw §S4's warnings.
        match n {
            2 => assert_eq!(
                warnings,
                vec![
                    "SMYSL.PARSE: 1 schemadecl record(s) skipped: they have no unit, relation or label MAP"
                ]
            ),
            3 => assert_eq!(
                warnings,
                vec![
                    "SMYSL.PARSE: 1 thread record(s) skipped: they have no unit, relation or label MAP",
                    "SMYSL.PARSE: 1 view record(s) skipped: they have no unit, relation or label MAP",
                ]
            ),
            _ => assert!(warnings.is_empty(), "document {n}: {warnings:?}"),
        }
    }
}

/// Criterion 5: a `retracts` relation is refused by `smysl.append` with
/// SM03, and the library is unchanged [R23].
#[test]
fn a_document_holding_a_retraction_is_refused_whole() {
    let (maps, _) = parsed(
        "@claim c/gone { status: speculative }\n~ Withdrawn.\n\n@rel c/gone --retracts--> c/gone\n",
    );
    let d = Dir::library("retracts");
    let before = d.catalog_len();
    assert_eq!(
        appended(&d, maps.unwrap()).unwrap_err(),
        "SMYSL.APPEND: a retracts relation is written only by smysl.retract"
    );
    assert_eq!(d.catalog_len(), before);
    assert!(d.open().store().is_empty());
}

/// Criterion 5: a malformed record, a `grounds` naming a label never bound,
/// and a relation naming one, are each SM02 carrying SMYSL's diagnostics,
/// spans included.
#[test]
fn a_document_smysl_could_not_parse_whole_is_refused() {
    for (doc, code) in [
        ("@claim c/bad { status: nonsense }\n~ Bad.\n", "SMY-E"),
        (
            "@claim c/a { status: inferred, grounds: [o/missing] }\n~ Rests on nothing.\n",
            "SMY-E060",
        ),
        (
            "@claim c/a { status: speculative }\n~ A.\n\n@rel c/a --causes--> c/missing\n",
            "SMY-E060",
        ),
    ] {
        let err = parsed(doc).0.unwrap_err();
        assert!(
            err.starts_with("SMYSL.PARSE: SMYSL refused it: error: "),
            "{err}"
        );
        assert!(err.contains(code), "{err}");
        assert!(err.contains("(at "), "{err}");
    }
    // The malformed record is skipped by recovery, and said.
    let err = parsed("@claim c/bad { status: nonsense }\n~ Bad.\n")
        .0
        .unwrap_err();
    assert!(err.ends_with("records skipped"), "{err}");
}

// --- criterion 7: checks report --------------------------------------------------

/// Criterion 7: a `derived` unit over a `speculative` ground is `SMY-E030`
/// at `error`; a library of well-formed units is clean.
#[test]
fn check_reports_rule_m_and_a_clean_library_has_no_error() {
    let d = Dir::library("check");
    let ground = add_unit(&d, unit("A guess"));
    let mut over = unit("A derivation").as_map().unwrap().clone();
    over.insert("status".to_owned(), s("derived"));
    over.insert("grounds".to_owned(), list(vec![s(&ground)]));
    add_unit(&d, BundValue::map(over));
    let diags = items(&call(vec![handle(&d)], "smysl.check").0.unwrap());
    let e030: Vec<_> = diags
        .iter()
        .filter(|x| text(&get(x, "code")) == "SMY-E030")
        .collect();
    assert_eq!(e030.len(), 1, "{diags:?}");
    assert_eq!(text(&get(e030[0], "severity")), "error");

    let clean = Dir::library("clean");
    add_unit(&clean, unit("One well-formed claim"));
    let diags = items(&call(vec![handle(&clean)], "smysl.check").0.unwrap());
    assert!(
        diags.iter().all(|x| text(&get(x, "severity")) != "error"),
        "{diags:?}"
    );
}

// --- criterion 15: locks are SMYSL's ---------------------------------------------

/// `smysl.text.add` of `body` under `alias`, answering its MAP.
fn add_text(d: &Dir, alias: &str, body: &str) -> BundValue {
    let spec = m(&[
        ("text", s(body)),
        ("reader", s("txt/1")),
        ("alias", s(alias)),
        ("licence", s("CC0-1.0")),
        ("lang", s("en")),
    ]);
    call(vec![handle(d), spec], "smysl.text.add").0.unwrap()
}

/// A unit citing the first part of the text `added` answered.
fn citing(gist: &str, added: &BundValue) -> BundValue {
    let part = text(&items(&get(added, "parts"))[0]);
    m(&[
        ("schema", s("observation")),
        ("gist", s(gist)),
        ("status", s("cited")),
        (
            "source",
            m(&[
                ("kind", s("doc")),
                ("reference", s(&part)),
                ("span", list(vec![BundValue::int(0), BundValue::int(5)])),
                ("manifest", get(added, "mid")),
            ]),
        ),
    ])
}

fn zeta_log(d: &Dir) -> PathBuf {
    d.0.join(smysl_text::shards::log_path("zeta"))
}

/// Criterion 15, a failed append: `catalog`'s log grows and `zeta`'s open
/// fails, so the word answers SM02 with SMYSL's `Io` text, no code, and
/// the "left locked" suffix, and every lock stays.
#[test]
fn a_failed_append_after_a_written_shard_leaves_the_locks() {
    use std::os::unix::fs::PermissionsExt as _;
    let d = Dir::library("failed");
    let added = add_text(&d, "zeta", "Hello there. The well is dry.\n");
    appended(&d, vec![citing("Hello", &added)]).unwrap();
    let log = zeta_log(&d);
    assert!(log.is_file(), "zeta has a log");
    std::fs::set_permissions(&log, std::fs::Permissions::from_mode(0o444)).unwrap();
    assert!(
        std::fs::OpenOptions::new().append(true).open(&log).is_err(),
        "this environment ignores file permissions (a root user?), so a read-only \
         log does not refuse an append and the failed-append case cannot be built"
    );
    let catalog = d.catalog_len();

    let err = appended(
        &d,
        vec![unit("Routed to the catalog"), citing("there", &added)],
    )
    .unwrap_err();
    std::fs::set_permissions(&log, std::fs::Permissions::from_mode(0o644)).unwrap();
    let head = "SMYSL.APPEND: SMYSL refused it: ";
    assert!(err.starts_with(head), "{err}");
    assert!(!err[head.len()..].starts_with("SMY-"), "{err}");
    assert!(err.ends_with(LEFT_LOCKED), "{err}");
    assert!(d.catalog_len() > catalog, "the catalog was written");
    let found = super::unlock::unlock(&d.0, false).unwrap();
    assert!(
        found.iter().any(|f| f.path == d.0.join("lock")),
        "{found:?}"
    );
    assert!(
        found.iter().any(|f| f.path.ends_with("catalog.lock")),
        "{found:?}"
    );
    assert!(
        found.iter().any(|f| f.path.ends_with("zeta.lock")),
        "{found:?}"
    );
}

/// Criterion 15: over a library whose `zeta` log holds a torn tail, a
/// `smysl.text.add` SMYSL refuses for `NoParts` is SM02 without the suffix,
/// leaves no lock, and the next write succeeds.
#[test]
fn a_refusal_over_a_torn_library_leaves_no_lock() {
    let d = Dir::library("torn");
    let added = add_text(&d, "zeta", "Hello there.\n");
    appended(&d, vec![citing("Hello", &added)]).unwrap();
    // A torn tail: half of a whole record's bytes.
    let core = smysl_core::UnitCoreBuilder::new(
        smysl_core::KernelType::Claim,
        "a record cut in half by a crash",
        smysl_core::Status::Speculative,
    )
    .build()
    .unwrap();
    let bytes = smysl_core::to_cbor(&Record::Unit(core));
    {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(zeta_log(&d))
            .unwrap();
        f.write_all(&bytes[..bytes.len() / 2]).unwrap();
    }
    let spec = m(&[
        ("text", s("")),
        ("reader", s("txt/1")),
        ("alias", s("empty")),
        ("licence", s("CC0-1.0")),
        ("lang", s("en")),
    ]);
    let err = call(vec![handle(&d), spec], "smysl.text.add")
        .0
        .unwrap_err();
    assert!(
        err.starts_with("SMYSL.TEXT.ADD: SMYSL refused it: "),
        "{err}"
    );
    assert!(!err.contains("left locked"), "{err}");
    assert!(d.no_lock());
    add_unit(&d, unit("The next write"));
}

/// Criterion 15: a batch `check_loggable` refuses, and an `add` SMYSL
/// refuses for want of a language, each through the guarded write
/// directly, leave no lock, and the next write succeeds.
#[test]
fn a_failure_that_writes_nothing_leaves_no_lock() {
    let d = Dir::library("nothing");
    let part = Record::PartText(smysl_core::PartText::new(b"a part text".to_vec()));
    let r = super::write::write("smysl.append", &d.0, |w| w.append(&[part]));
    let r = r.unwrap_err();
    assert_eq!(r.code, super::Code::Sm02);
    assert!(!r.text.contains("left locked"), "{}", r.text);
    assert!(d.no_lock());

    let r = super::write::write("smysl.text.add", &d.0, |w| {
        w.call(|lib| {
            lib.add(
                &smysl_text::Input::new(b"Hello there.\n"),
                &smysl_text::AddSpec::new("txt/1", "nolang", "CC0-1.0"),
                &smysl_text::Caps::default(),
            )
        })
    });
    let r = r.unwrap_err();
    assert_eq!(r.code, super::Code::Sm02);
    assert!(r.text.contains("lang"), "{}", r.text);
    assert!(d.no_lock());
    add_unit(&d, unit("The next write"));
}

/// Criterion 15, SM02's text: a licence that forbids `carry: "text"` is
/// `SMY-E402`, once, with no suffix.
#[test]
fn a_licence_refusal_is_sm02_with_its_code_once() {
    let d = Dir::library("e402");
    let spec = m(&[
        ("text", s("Hello there.\n")),
        ("reader", s("txt/1")),
        ("alias", s("closed")),
        ("licence", s("LicenseRef-Proprietary")),
        ("lang", s("en")),
        ("carry", s("text")),
    ]);
    let err = call(vec![handle(&d), spec], "smysl.text.add")
        .0
        .unwrap_err();
    assert_eq!(
        err,
        "SMYSL.TEXT.ADD: SMYSL refused it: SMY-E402: licence `LicenseRef-Proprietary` \
         does not permit carrying the text"
    );
    assert!(d.no_lock());
}

/// Criterion 15, the re-open: with the probe armed to `pause`, a
/// `smysl.append` of two new units stops after its first open; another
/// writer appends the first of them with an attestation by the same agent,
/// under `acquire_all`. The write then appends neither that unit nor a
/// second attestation of it, and attests the second later than the other
/// writer's clock.
#[cfg(feature = "smysl-panic-probe")]
#[test]
fn a_write_decides_from_the_library_as_reopened_under_the_locks() {
    use super::probe::{Action, arm};
    use smysl_core::{Attestation, Op, Rung};
    let d = Dir::library("reopen");
    let flags = Dir::fresh("reopen-flag");
    std::fs::create_dir_all(&flags.0).unwrap();
    let flag = flags.0.join("arrived");
    arm(&d.0, Action::Pause(flag.clone()));

    let (first, second) = (unit("First of two"), unit("Second of two"));
    let first_core =
        super::map::unit_core(first.as_map().unwrap(), &mut |_| panic!("no labels")).unwrap();
    let first_uid = smysl_core::canonical_uid(&first_core);
    let root = d.root().to_owned();
    let writer = std::thread::spawn(move || {
        let h = m(&[
            ("type", s("smysl")),
            ("root", s(&root)),
            ("agent", s("tool:bund2")),
        ]);
        call(
            vec![h, list(vec![unit("First of two"), unit("Second of two")])],
            "smysl.append",
        )
        .0
        .map(|v| items(&v).iter().map(text).collect::<Vec<_>>())
    });
    let start = std::time::Instant::now();
    while !flag.exists() {
        assert!(start.elapsed().as_secs() < 30, "the write never paused");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let a = agent("tool:bund2");
    let other_ts = Hlc::now(&Hlc::zero(a.clone()), &a);
    {
        let mut lib = d.open();
        let _locks =
            smysl_text::lock::acquire_all(&d.0, &lib.shards(), "the other writer").unwrap();
        lib.append_records(&[
            Record::Unit(first_core),
            Record::Attestation(Attestation::new(
                first_uid,
                a.clone(),
                Op::Authored,
                Rung::Computed,
                other_ts.clone(),
            )),
        ])
        .unwrap();
    }
    std::fs::remove_file(&flag).unwrap();
    let ids = writer.join().unwrap().unwrap();
    assert_eq!(ids[0], first_uid.canonical());
    drop(second);

    let lib = d.open();
    assert_eq!(lib.store().duplicate_records(), 0);
    let mine: Vec<_> = lib
        .store()
        .attestations_of(&first_uid)
        .iter()
        .filter(|x| x.agent == a)
        .cloned()
        .collect();
    assert_eq!(mine.len(), 1, "no second attestation of the first unit");
    assert_eq!(mine[0].ts, other_ts);
    let units = lib
        .store()
        .iter()
        .filter(|r| matches!(r, Record::Unit(u) if smysl_core::canonical_uid(u) == first_uid))
        .count();
    assert_eq!(units, 1, "the first unit is not appended again");
    let second_ts = &lib
        .store()
        .attestations_of(&uid(&ids[1]))
        .iter()
        .next()
        .unwrap()
        .ts;
    assert!(*second_ts > other_ts);
}

// --- criterion 18: what the other criteria do not reach --------------------------

/// 18: `smysl.rel` and `smysl.label`, each alone, write what the same
/// record written through `smysl.append` writes.
#[test]
fn rel_and_label_write_what_append_writes() {
    let alone = Dir::library("alone");
    let batch = Dir::library("batch");
    let a = add_unit(&alone, unit("A"));
    let b = add_unit(&alone, unit("B"));
    let r = text(
        &call(vec![handle(&alone), rel("causes", &a, &b)], "smysl.rel")
            .0
            .unwrap(),
    );
    assert_eq!(
        call(vec![handle(&alone), s("c/a"), s(&a)], "smysl.label")
            .0
            .unwrap(),
        BundValue::nodata()
    );
    let ids = appended(
        &batch,
        vec![
            unit("A"),
            unit("B"),
            rel("causes", &a, &b),
            m(&[("label", s("c/a")), ("uid", s(&a))]),
        ],
    )
    .unwrap();
    assert_eq!(ids, [a.clone(), b, r].into_iter().collect());
    // Record for record, attestation clocks aside.
    let strip = |d: &Dir| -> Vec<Record> {
        d.open()
            .store()
            .iter()
            .filter(|r| !matches!(r, Record::Attestation(_)))
            .cloned()
            .collect()
    };
    assert_eq!(strip(&alone), strip(&batch));
    let attested = |d: &Dir| {
        d.open()
            .store()
            .iter()
            .filter(|r| matches!(r, Record::Attestation(_)))
            .count()
    };
    assert_eq!(attested(&alone), 3);
    assert_eq!(attested(&batch), 3);
}

/// 18: `smysl.rel` given a `retracts` relation is SM03 [R23]; `smysl.label`
/// of a label bound to another unit is SM08 and writes nothing, and of one
/// bound to the same unit writes nothing; a LIST binding one label to two
/// units is SM08 and writes nothing.
#[test]
fn label_and_rel_refuse_what_cannot_be_undone() {
    let d = Dir::library("bind");
    let a = add_unit(&d, unit("A"));
    let b = add_unit(&d, unit("B"));
    assert_eq!(
        call(vec![handle(&d), rel("retracts", &a, &a)], "smysl.rel")
            .0
            .unwrap_err(),
        "SMYSL.REL: a retracts relation is written only by smysl.retract"
    );
    call(vec![handle(&d), s("c/x"), s(&a)], "smysl.label")
        .0
        .unwrap();
    let len = d.catalog_len();
    assert_eq!(
        call(vec![handle(&d), s("c/x"), s(&b)], "smysl.label")
            .0
            .unwrap_err(),
        format!("SMYSL.LABEL: label c/x is bound to {a}, not {b}")
    );
    call(vec![handle(&d), s("c/x"), s(&a)], "smysl.label")
        .0
        .unwrap();
    assert_eq!(d.catalog_len(), len, "nothing written either time");

    let fresh = Dir::library("bind-list");
    let err = appended(
        &fresh,
        vec![
            unit("A"),
            unit("B"),
            m(&[("label", s("c/y")), ("uid", s(&a))]),
            m(&[("label", s("c/y")), ("uid", s(&b))]),
        ],
    )
    .unwrap_err();
    assert_eq!(
        err,
        format!("SMYSL.APPEND: label c/y is bound to {a}, not {b}")
    );
    assert!(fresh.open().store().is_empty());
}

/// 18: a label MAP in `smysl.append` writes the `LabelBinding` `smysl.label`
/// writes; a unit MAP with `labels` is SM03; a `grounds` entry naming an
/// unbound label is SM08 with `0`; `smysl.parse` of a labelled unit gives a
/// label MAP whose `uid` is the uid `smysl.append` answers for it.
#[test]
fn labels_travel_as_label_maps() {
    let d = Dir::library("labels");
    let a = add_unit(&d, unit("A"));
    appended(&d, vec![m(&[("label", s("c/a")), ("uid", s(&a))])]).unwrap();
    let bindings: Vec<LabelBinding> = d
        .open()
        .store()
        .iter()
        .filter_map(|r| match r {
            Record::LabelBinding(b) => Some(b.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        bindings,
        vec![LabelBinding::new(
            smysl_core::Label::new("c/a").unwrap(),
            uid(&a)
        )]
    );

    let mut labelled = unit("L").as_map().unwrap().clone();
    labelled.insert("labels".to_owned(), list(vec![s("c/l")]));
    assert_eq!(
        appended(&d, vec![BundValue::map(labelled)]).unwrap_err(),
        "SMYSL.APPEND: labels is read back only: a label is bound with smysl.label or a label MAP"
    );
    let mut grounded = unit("G").as_map().unwrap().clone();
    grounded.insert("grounds".to_owned(), list(vec![s("c/nowhere")]));
    assert_eq!(
        appended(&d, vec![BundValue::map(grounded)]).unwrap_err(),
        "SMYSL.APPEND: label c/nowhere binds 0 units"
    );

    let (maps, _) = parsed("@claim c/named { status: speculative }\n~ Named.\n");
    let maps = maps.unwrap();
    let fresh = Dir::library("labels-parse");
    let ids = call(vec![handle(&fresh), list(maps.clone())], "smysl.append")
        .0
        .unwrap();
    let label = maps.iter().find(|v| v.has_key("label")).unwrap();
    assert_eq!(text(&get(label, "uid")), text(&items(&ids)[0]));
}

/// 18: a LIST's MAPs are decided by §S2's rule.
#[test]
fn append_decides_each_maps_kind() {
    let d = Dir::library("kinds");
    let a = add_unit(&d, unit("A"));
    let refused = |v: BundValue| appended(&d, vec![v]).unwrap_err();
    assert_eq!(
        refused(m(&[("colour", s("red"))])),
        "SMYSL.APPEND: record is not a unit, relation or label"
    );
    assert_eq!(
        refused(m(&[("kind", s("causes")), ("from", s(&a))])),
        "SMYSL.APPEND: relation lacks to"
    );
    let mut odd = unit("Odd").as_map().unwrap().clone();
    odd.insert("colour".to_owned(), s("red"));
    assert_eq!(
        refused(BundValue::map(odd)),
        "SMYSL.APPEND: unit has no key colour"
    );
    let missing = smysl_core::canonical_uid(
        &smysl_core::UnitCoreBuilder::new(
            smysl_core::KernelType::Claim,
            "never written",
            smysl_core::Status::Speculative,
        )
        .build()
        .unwrap(),
    );
    assert_eq!(
        refused(m(&[("label", s("c/m")), ("uid", s(&missing.canonical()))])),
        format!(
            "SMYSL.APPEND: uid {} names no unit in the library",
            missing.canonical()
        )
    );
    let mut bad = unit("Bad").as_map().unwrap().clone();
    bad.insert("grounds".to_owned(), list(vec![s("Not A Ref")]));
    assert_eq!(
        refused(BundValue::map(bad)),
        "SMYSL.APPEND: grounds entry Not A Ref is neither a uid nor a label"
    );
    assert_eq!(
        call(vec![handle(&d), rel("causes", &a, &a)], "smysl.unit")
            .0
            .unwrap_err(),
        "SMYSL.UNIT: #1 parameter is a relation MAP, not a unit"
    );
    assert_eq!(
        appended(&d, vec![s("a string")]).unwrap_err(),
        "SMYSL.APPEND: list item 0 must be MAP, found String"
    );
}

const SUPERSEDED: &str = "@claim c/old { status: speculative }\n~ Old.\n\n\
@claim c/new { status: speculative }\n~ New.\n\n\
@rel c/new --supersedes--> c/old\n";

/// 18: a document holding a `supersedes` relation and a withdrawal of it
/// answers the relation, leaves nothing out, and reports one `SMY-W056`
/// per distinct withdrawal.
#[test]
fn a_withdrawn_lifecycle_edge_is_answered_and_warned_of() {
    let by = |who: &str, at: u64| {
        format!("\n@withdraw c/new --supersedes--> c/old {{ agent: {who}, ts: [{at}, 0] }}\n")
    };
    let w056 = |warnings: &[String]| {
        warnings
            .iter()
            .filter(|w| w.contains("SMY-W056"))
            .cloned()
            .collect::<Vec<_>>()
    };
    let (maps, warnings) = parsed(&format!("{SUPERSEDED}{}", by("human:ann", 1726500000000)));
    let maps = maps.unwrap();
    assert!(
        maps.iter()
            .any(|v| v.has_key("kind") && text(&get(v, "kind")) == "supersedes")
    );
    assert!(
        !warnings.iter().any(|w| w.contains("withdraws left out")),
        "{warnings:?}"
    );
    let one = w056(&warnings);
    assert_eq!(one.len(), 1, "{warnings:?}");
    assert!(
        one[0].contains("human:ann") && one[0].contains("supersedes"),
        "{one:?}"
    );

    let (_, warnings) = parsed(&format!(
        "{SUPERSEDED}{}{}",
        by("human:ann", 1726500000000),
        by("human:bob", 1726500000000)
    ));
    assert_eq!(w056(&warnings).len(), 2, "{warnings:?}");
    let (_, warnings) = parsed(&format!(
        "{SUPERSEDED}{}{}",
        by("human:ann", 1726500000000),
        by("human:ann", 1726500000000)
    ));
    assert_eq!(w056(&warnings).len(), 1, "{warnings:?}");
}

/// 18: `smysl.parse` of a document with a comment and an authored salience
/// reports one warning for each, and answers the same LIST as without.
#[test]
fn comments_and_salience_are_dropped_and_said() {
    let plain = "@claim c/a { status: speculative }\n~ A claim.\n";
    let rich =
        "# a reviewer's note\n@claim c/a { status: speculative, salience: 0.75 }\n~ A claim.\n";
    let (a, wa) = parsed(plain);
    let (b, wb) = parsed(rich);
    assert!(wa.is_empty(), "{wa:?}");
    assert_eq!(
        wb,
        vec![
            "SMYSL.PARSE: 1 comment line(s) dropped: a comment is not a record",
            "SMYSL.PARSE: the authored salience of 1 unit(s) dropped: a unit MAP has no salience",
        ]
    );
    let shown = |v: Vec<BundValue>| v.iter().map(BundValue::display).collect::<Vec<_>>();
    assert_eq!(shown(a.unwrap()), shown(b.unwrap()));
}

/// 18: `smysl.write` leaves out a unit whose payload has a `status` key, a
/// unit whose source has an unknown kind, and the label of the first, and
/// says so, counting 2 units and 1 label and nothing else; its text,
/// parsed and appended, holds every other uid, referring to the omitted
/// unit by uid.
#[test]
fn write_leaves_out_what_has_no_surface_form_and_says_so() {
    let d = Dir::library("omit");
    let mut odd = unit("Odd").as_map().unwrap().clone();
    odd.insert("payload".to_owned(), m(&[("status", s("x"))]));
    let odd = add_unit(&d, BundValue::map(odd));
    call(vec![handle(&d), s("c/odd"), s(&odd)], "smysl.label")
        .0
        .unwrap();
    let pointer = add_unit(&d, unit("Points at the odd one"));
    let r = text(
        &call(
            vec![handle(&d), rel("elaborates", &pointer, &odd)],
            "smysl.rel",
        )
        .0
        .unwrap(),
    );
    let mut grounded = unit("Grounded on the odd one").as_map().unwrap().clone();
    grounded.insert("status".to_owned(), s("inferred"));
    grounded.insert("grounds".to_owned(), list(vec![s(&odd)]));
    let grounded = add_unit(&d, BundValue::map(grounded));
    let unknown = smysl_core::UnitCoreBuilder::new(
        smysl_core::KernelType::Claim,
        "From a source this build cannot name",
        smysl_core::Status::Cited,
    )
    .source(
        smysl_core::SourceRef::new(smysl_core::SourceKind::Doc, "r")
            .with_unknown_kind(77)
            .unwrap(),
    )
    .build()
    .unwrap();
    let unknown_uid = smysl_core::canonical_uid(&unknown);
    raw(&d, &[Record::Unit(unknown)]);

    let (surface, warnings) = written(&d);
    assert_eq!(
        warnings,
        vec![
            "SMYSL.WRITE: 1 labelbinding record(s) have no surface form and were left out",
            "SMYSL.WRITE: 2 unit record(s) have no surface form and were left out",
        ]
    );
    let (maps, _) = parsed(&surface);
    let fresh = Dir::library("omit-again");
    let ids = appended(&fresh, maps.unwrap()).unwrap();
    assert!(ids.contains(&pointer) && ids.contains(&grounded) && ids.contains(&r));
    assert!(!ids.contains(&odd) && !ids.contains(&unknown_uid.canonical()));
    assert!(surface.contains(&odd), "the omitted unit is spelled by uid");
}

/// 18: a label bound to two units is written on neither, and named; the
/// text answers the same uids. A library written by `smysl.unit`,
/// `smysl.rel` and `smysl.label` alone draws no warning.
#[test]
fn write_names_a_label_bound_twice_and_is_quiet_otherwise() {
    let d = Dir::library("shared");
    let a = add_unit(&d, unit("A"));
    let b = add_unit(&d, unit("B"));
    let l = smysl_core::Label::new("c/both").unwrap();
    raw(
        &d,
        &[
            Record::LabelBinding(LabelBinding::new(l.clone(), uid(&a))),
            Record::LabelBinding(LabelBinding::new(l, uid(&b))),
        ],
    );
    let (surface, warnings) = written(&d);
    assert_eq!(
        warnings,
        vec![
            "SMYSL.WRITE: 2 labelbinding record(s) have no surface form and were left out",
            "SMYSL.WRITE: label c/both binds 2 units, and is written on none of them",
        ]
    );
    assert!(!surface.contains("c/both"), "{surface}");
    let fresh = Dir::library("shared-again");
    let ids = appended(&fresh, parsed(&surface).0.unwrap()).unwrap();
    assert_eq!(ids, [a, b].into_iter().collect());

    let quiet = Dir::library("quiet");
    let a = add_unit(&quiet, unit("A"));
    let b = add_unit(&quiet, unit("B"));
    call(vec![handle(&quiet), rel("causes", &a, &b)], "smysl.rel")
        .0
        .unwrap();
    call(vec![handle(&quiet), s("c/a"), s(&a)], "smysl.label")
        .0
        .unwrap();
    call(vec![handle(&quiet), s("c/b"), s(&b)], "smysl.label")
        .0
        .unwrap();
    let (_, warnings) = written(&quiet);
    assert!(warnings.is_empty(), "{warnings:?}");
}

/// 18: `smysl.rel` of a new edge whose rid a withdrawal already names
/// answers its rid and warns.
#[test]
fn a_new_edge_already_withdrawn_is_answered_and_warned_of() {
    let d = Dir::library("pre-withdrawn");
    let a = add_unit(&d, unit("A"));
    let b = add_unit(&d, unit("B"));
    let r = rid(RelKind::Rebuts, &a, &b);
    raw(
        &d,
        &[Record::Withdrawal(withdrawal(r, "human:rev", "human:rev"))],
    );
    let (out, warnings) = call(vec![handle(&d), rel("rebuts", &a, &b)], "smysl.rel");
    assert_eq!(text(&out.unwrap()), r.canonical());
    assert_eq!(
        warnings,
        vec![format!(
            "SMYSL.REL: edge {} is withdrawn: nothing that interprets the graph follows it",
            r.canonical()
        )]
    );
}

/// 18: a withdrawn relation and a text: the relation is written with its
/// withdrawal, the manifest when it has a surface form; parsed back, the
/// edge is left out and counted. A withdrawal whose clock is another
/// agent's has no surface form: counted, and its edge named, unless a
/// written withdrawal still withdraws it.
#[test]
fn write_carries_withdrawals_and_names_an_edge_that_would_read_back_live() {
    let d = Dir::library("withdrawn");
    let a = add_unit(&d, unit("A"));
    let b = add_unit(&d, unit("B"));
    let r = rid(RelKind::Rebuts, &a, &b);
    call(vec![handle(&d), rel("rebuts", &a, &b)], "smysl.rel")
        .0
        .unwrap();
    raw(
        &d,
        &[Record::Withdrawal(withdrawal(r, "human:rev", "human:rev"))],
    );
    add_text(&d, "notes", "A note on the river.\n");
    let manifest = d
        .open()
        .store()
        .iter()
        .find(|x| matches!(x, Record::Manifest(_)))
        .cloned()
        .unwrap();
    let admitted = smysl_core::surface::has_surface_form(
        &manifest,
        &smysl_core::surface::WriteContext::default(),
        None,
    );
    let (surface, warnings) = written(&d);
    assert!(surface.contains("@withdraw"), "{surface}");
    let counted = warnings
        .iter()
        .any(|w| w.contains("manifest record(s) have no surface form"));
    assert_eq!(counted, !admitted, "{warnings:?}");
    let (maps, warnings) = parsed(&surface);
    assert!(
        !maps
            .unwrap()
            .iter()
            .any(|v| v.has_key("kind") && text(&get(v, "kind")) == "rebuts")
    );
    assert!(
        warnings
            .iter()
            .any(|w| w == "SMYSL.PARSE: 1 relation(s) the document withdraws left out"),
        "{warnings:?}"
    );

    let other = Dir::library("withdrawn-other");
    let a = add_unit(&other, unit("A"));
    let b = add_unit(&other, unit("B"));
    let r = rid(RelKind::Rebuts, &a, &b);
    call(vec![handle(&other), rel("rebuts", &a, &b)], "smysl.rel")
        .0
        .unwrap();
    raw(
        &other,
        &[Record::Withdrawal(withdrawal(
            r,
            "human:rev",
            "human:other",
        ))],
    );
    let (_, warnings) = written(&other);
    assert_eq!(
        warnings,
        vec![format!(
            "SMYSL.WRITE: 1 withdrawal record(s) have no surface form and were left out, \
             so these withdrawn edges read back live: {}",
            r.canonical()
        )]
    );
    raw(
        &other,
        &[Record::Withdrawal(withdrawal(
            r,
            "human:rev2",
            "human:rev2",
        ))],
    );
    let (_, warnings) = written(&other);
    assert_eq!(
        warnings,
        vec!["SMYSL.WRITE: 1 withdrawal record(s) have no surface form and were left out"]
    );
}

/// 18: `payload: {}` gives the uid of the same unit without `payload`.
#[test]
fn an_empty_payload_is_the_same_unit() {
    let d = Dir::library("empty-payload");
    let mut with = unit("A").as_map().unwrap().clone();
    with.insert("payload".to_owned(), BundValue::map(BTreeMap::new()));
    assert_eq!(add_unit(&d, BundValue::map(with)), add_unit(&d, unit("A")));
}

/// 18: re-writing an edge the library holds: the same weight writes
/// nothing and answers the rid; another weight, or another note, is SM03
/// and the library is unchanged; a withdrawn edge with the same weight is
/// answered, warned of, and stays withdrawn, through `smysl.rel` and
/// through `smysl.append`.
#[test]
fn an_edge_is_held_with_one_weight_and_note() {
    let d = Dir::library("rewrite");
    let a = add_unit(&d, unit("A"));
    let b = add_unit(&d, unit("B"));
    let n = add_unit(&d, unit("A note"));
    let weighted = |w: f64| {
        let mut x = rel("rebuts", &a, &b).as_map().unwrap().clone();
        x.insert("weight".to_owned(), BundValue::float(w));
        BundValue::map(x)
    };
    let r = text(
        &call(vec![handle(&d), weighted(0.5)], "smysl.rel")
            .0
            .unwrap(),
    );
    let len = d.catalog_len();
    assert_eq!(
        text(
            &call(vec![handle(&d), weighted(0.5)], "smysl.rel")
                .0
                .unwrap()
        ),
        r
    );
    assert_eq!(d.catalog_len(), len);
    let held = format!("SMYSL.REL: rid {r} is held with another weight or note");
    assert_eq!(
        call(vec![handle(&d), weighted(0.75)], "smysl.rel")
            .0
            .unwrap_err(),
        held
    );
    let mut noted = weighted(0.5).as_map().unwrap().clone();
    noted.insert("note".to_owned(), s(&n));
    assert_eq!(
        call(vec![handle(&d), BundValue::map(noted)], "smysl.rel")
            .0
            .unwrap_err(),
        held
    );
    assert_eq!(d.catalog_len(), len);

    raw(
        &d,
        &[Record::Withdrawal(withdrawal(
            uid(&r),
            "human:rev",
            "human:rev",
        ))],
    );
    let warned = vec![format!(
        "SMYSL.REL: edge {r} is withdrawn: nothing that interprets the graph follows it"
    )];
    let (out, warnings) = call(vec![handle(&d), weighted(0.5)], "smysl.rel");
    assert_eq!(text(&out.unwrap()), r);
    assert_eq!(warnings, warned);
    let lib = d.open();
    let held = lib.store().relation_by_id(&uid(&r)).unwrap().clone();
    assert!(lib.store().is_withdrawn(&held));
    let (_, warnings) = call(vec![handle(&d), list(vec![weighted(0.5)])], "smysl.append");
    assert_eq!(
        warnings,
        vec![warned[0].replace("SMYSL.REL", "SMYSL.APPEND")]
    );
}

/// 18: `smysl.text.add`'s `lossy` and `redacted` equal what `Library::add`
/// answers for the same text in a twin library, including a text with a
/// part the library has redacted; the sidecar is current afterwards.
#[test]
fn text_add_reports_what_library_add_reports() {
    const BODY: &str = "# A heading\n\nA paragraph about the river.\n\nAnother about the ford.\n";
    let words = Dir::library("text-words");
    let twin = Dir::library("text-twin");
    let spec = |alias: &str| {
        m(&[
            ("text", s(BODY)),
            ("reader", s("md/1")),
            ("alias", s(alias)),
            ("licence", s("CC0-1.0")),
            ("lang", s("en")),
        ])
    };
    let api = |alias: &str| {
        smysl_text::AddSpec::new("md/1", alias, "CC0-1.0")
            .with_lang(smysl_text::LangTag::new("en").unwrap())
    };
    let first = call(vec![handle(&words), spec("first")], "smysl.text.add")
        .0
        .unwrap();
    let mut lib = twin.open();
    let direct = lib
        .add(
            &smysl_text::Input::new(BODY.as_bytes()),
            &api("first"),
            &smysl_text::Caps::default(),
        )
        .unwrap();
    assert_eq!(text(&get(&first, "mid")), direct.mid.canonical());
    assert_eq!(get(&first, "lossy"), BundValue::boolean(direct.lossy));
    assert_eq!(get(&first, "redacted"), BundValue::int(0));

    // Redact the first part in each, then add the text again.
    let rev = agent("human:rev");
    for d in [&words, &twin] {
        let mut lib = d.open();
        lib.redact(
            &direct.parts[0],
            &rev,
            Hlc::now(&Hlc::zero(rev.clone()), &rev),
            None,
        )
        .unwrap();
    }
    let again = call(vec![handle(&words), spec("second")], "smysl.text.add")
        .0
        .unwrap();
    let mut lib = twin.open();
    let direct = lib
        .add(
            &smysl_text::Input::new(BODY.as_bytes()),
            &api("second"),
            &smysl_text::Caps::default(),
        )
        .unwrap();
    assert_ne!(direct.objects_redacted, 0);
    assert_eq!(
        get(&again, "redacted"),
        BundValue::int(direct.objects_redacted as i64)
    );
    assert_eq!(get(&again, "lossy"), BundValue::boolean(direct.lossy));

    let lib = words.open();
    let bytes = std::fs::read(smysl_text::index::Index::path(&words.0)).unwrap();
    let ix = smysl_text::index::Index::from_bytes(&bytes).unwrap();
    let (len, hash) = lib.stamp();
    assert!(ix.matches(len, &hash), "the sidecar is current");

    // `smysl.texts` names each alias with its head.
    let texts = call(vec![handle(&words)], "smysl.texts").0.unwrap();
    assert_eq!(
        items(&get(&texts, "second"))
            .iter()
            .map(text)
            .collect::<Vec<_>>(),
        vec![text(&get(&again, "mid"))]
    );
}

// --- criterion 14: refusals ------------------------------------------------------

/// 14: a `path` to a sparse file one byte over `Caps::default().input_bytes`
/// is SM02 with `SMY-E440`, without the CLI's flag, and reads nothing; a
/// spec with no `lang` is SM03; a relative root given to `smysl.texts` is
/// SM00 and touches no file.
#[test]
fn text_add_and_texts_refuse_as_section_s8_says() {
    let d = Dir::library("caps");
    let input = Dir::fresh("caps-input");
    std::fs::create_dir_all(&input.0).unwrap();
    let big = input.0.join("big.txt");
    let cap = smysl_text::Caps::default().input_bytes;
    std::fs::File::create(&big)
        .unwrap()
        .set_len(cap + 1)
        .unwrap();
    let spec = |extra: &[(&str, BundValue)]| {
        let mut x = m(&[
            ("reader", s("txt/1")),
            ("alias", s("big")),
            ("licence", s("CC0-1.0")),
            ("lang", s("en")),
        ])
        .as_map()
        .unwrap()
        .clone();
        for (k, v) in extra {
            x.insert((*k).to_owned(), v.clone());
        }
        BundValue::map(x)
    };
    assert_eq!(
        call(
            vec![handle(&d), spec(&[("path", s(big.to_str().unwrap()))])],
            "smysl.text.add"
        )
        .0
        .unwrap_err(),
        format!(
            "SMYSL.TEXT.ADD: SMYSL refused it: SMY-E440: input_bytes limit {cap} exceeded by {}",
            cap + 1
        )
    );
    assert!(d.no_lock());

    let mut no_lang = spec(&[("text", s("Hello.\n"))]).as_map().unwrap().clone();
    no_lang.remove("lang");
    assert_eq!(
        call(vec![handle(&d), BundValue::map(no_lang)], "smysl.text.add")
            .0
            .unwrap_err(),
        "SMYSL.TEXT.ADD: spec lacks lang"
    );
    assert_eq!(
        call(
            vec![
                handle(&d),
                spec(&[("text", s("Hello.\n")), ("reader", s("pdf/1"))])
            ],
            "smysl.text.add"
        )
        .0
        .unwrap_err(),
        "SMYSL.TEXT.ADD: unknown reader: pdf/1"
    );

    let relative = m(&[
        ("type", s("smysl")),
        ("root", s("rel/lib")),
        ("agent", s("tool:bund2")),
    ]);
    assert_eq!(
        call(vec![relative], "smysl.texts").0.unwrap_err(),
        "SMYSL.TEXTS: #1 parameter must be a SMYSL handle"
    );
    assert!(!Path::new("rel").exists());
}

// --- criterion 18's `--noio` case, for M4's words --------------------------------

#[test]
fn under_noio_m4s_words_are_sm06_and_parse_still_works() {
    let h = || {
        m(&[
            ("type", s("smysl")),
            ("root", s("/nowhere")),
            ("agent", s("tool:bund2")),
        ])
    };
    for (operands, word) in [
        (vec![h(), m(&[])], "smysl.rel"),
        (vec![h(), s("c/a"), s("b3:x")], "smysl.label"),
        (vec![h(), list(vec![])], "smysl.append"),
        (vec![h(), m(&[])], "smysl.text.add"),
        (vec![h()], "smysl.texts"),
        (vec![h()], "smysl.write"),
        (vec![h()], "smysl.check"),
    ] {
        let mut r = Run::new(true);
        assert_eq!(
            r.call(operands, word).unwrap_err(),
            super::words::NOIO,
            "{word}"
        );
    }
    let mut r = Run::new(true);
    let out = r
        .call(
            vec![s("@claim c/a { status: speculative }\n~ A.\n")],
            "smysl.parse",
        )
        .unwrap();
    assert_eq!(items(&out).len(), 2);
}
