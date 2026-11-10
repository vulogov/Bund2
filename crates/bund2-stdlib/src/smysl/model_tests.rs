//! RFC-0014 M11: the configuration, the registry and its adaptor, the
//! scripted provider, and the spike that confirms §P4's record kinds (Q59)
//! against what SMYSL's three model steps stage.
//!
//! Every provider here is `bund2-script` (§P8); nothing is sent anywhere.

#![cfg(feature = "smysl-model-probe")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use smysl_core::{AgentId, Record, Rung};
use smysl_graph::Store;
use smysl_ingest::answer::Member;
use smysl_ingest::text::{Target, TextIngest};
use smysl_ingest::{IngestOptions, Ingestor, Passes};
use smysl_provider::{ProviderError, ProviderId, Task};
use smysl_text::reading::Level;
use smysl_text::{AddSpec, Caps, Input, Library};

use super::Code;
use super::forming_tests::Dir;
use super::model::{self, Setup};
use super::script::{self, Answer, Probing};

// --- the harness ---------------------------------------------------------------

/// A project directory holding a library at `lib`, its root canonical, so
/// that the sidecar `.smysl/config.hjson` is the project's and no other
/// test's.
struct Project {
    dir: Dir,
    root: PathBuf,
}

impl Project {
    fn new(tag: &str) -> Project {
        let dir = Dir::fresh(tag);
        std::fs::create_dir_all(&dir.0).unwrap();
        let root = dir.0.join("lib");
        Library::create(&root).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        Project { dir, root }
    }

    /// The project directory, canonical.
    fn project(&self) -> PathBuf {
        self.root.parent().unwrap().to_path_buf()
    }

    fn sidecar(&self, src: &str) {
        let p = self.dir.0.join(".smysl");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("config.hjson"), src).unwrap();
    }

    fn setup(&self) -> Setup {
        model::configuration(&self.root, None).unwrap()
    }
}

/// An endpoint no other test arms: local, so the provider reports itself
/// offline-safe, as SMYSL's mappers decide it from the host.
fn endpoint(tag: &str) -> String {
    format!("http://localhost/{tag}-{}", std::process::id())
}

/// A configuration with one `bund2-script` provider `s` at `endpoint`, every
/// task routed to it.
fn config(endpoint: &str, extra: &str) -> String {
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

/// A record's kind, by its variant's name.
fn kind(r: &Record) -> String {
    let d = format!("{r:?}");
    d.split(['(', ' ', '{']).next().unwrap_or_default().to_string()
}

fn kinds(records: &[Record]) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for r in records {
        let k = kind(r);
        if v.last() != Some(&k) {
            v.push(k);
        }
    }
    v
}

fn agent() -> AgentId {
    AgentId::new("tool:bund2-m11").unwrap()
}

// --- the spike (§P4's kinds, Q59) -------------------------------------------------

/// `Ingestor::ingest` through the registry `model::registry` builds over
/// `bund2-script`: the staged records are `Unit`, `LabelBinding`, `Relation`
/// and `Attestation`, in `Staged::records`'s order, and no `SchemaDecl`,
/// since document ingest stages no schema (§P4).
#[test]
fn the_spike_document_ingest_stages_the_kinds_p4_lists() {
    let p = Project::new("spike-ingest");
    let ep = endpoint("spike-ingest");
    p.sidecar(&config(&ep, ""));
    script::arm(
        &ep,
        [Answer::Text(
            r#"{"units":[
                {"type":"claim","label":"c/pool","gist":"the pool wait rose","status":"speculative"},
                {"type":"claim","label":"c/queue","gist":"the queue grew","status":"speculative"}
            ]}"#
            .to_string(),
        )],
    );
    let built = model::registry(&p.setup().config, false);
    assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    let (staged, report) = Ingestor::new(
        &built.registry,
        IngestOptions::at_rung(Rung::Model).with_agent(agent()),
    )
    .ingest(&Store::new(), "The pool wait rose. The queue grew.")
    .unwrap();
    assert_eq!(report.calls, 1, "{:?}", report.diagnostics);
    let records = staged.records();
    let seen = kinds(&records);
    let allowed: BTreeSet<&str> = ["SchemaDecl", "Unit", "LabelBinding", "Relation", "Attestation"]
        .into_iter()
        .collect();
    assert!(
        seen.iter().all(|k| allowed.contains(k.as_str())),
        "{seen:?}"
    );
    assert!(!seen.contains(&"SchemaDecl".to_string()), "{seen:?}");
    assert_eq!(seen.first().map(String::as_str), Some("Unit"), "{seen:?}");
    assert!(seen.contains(&"LabelBinding".to_string()), "{seen:?}");
    assert_eq!(seen.last().map(String::as_str), Some("Attestation"), "{seen:?}");
    // In `Staged::records`'s order: each kind appears in one run.
    let mut distinct = seen.clone();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        seen.iter().collect::<BTreeSet<_>>().len(),
        "{seen:?}"
    );
    assert_eq!(script::calls(&ep).len(), 1);
    assert_eq!(built.failures.total(), 0);
    script::disarm(&ep);
}

/// `Ingestor::answer` over a store holding a dossier: the staged records
/// are `Unit`, `Relation` where the answer has one, and `Attestation`; no
/// `SchemaDecl` (`answer` stages none) and no `Thread` or `View`, which are
/// the dossier's (§P4).
#[test]
fn the_spike_an_answer_stages_the_kinds_p4_lists() {
    use smysl_core::{
        KernelType, SourceKind, SourceRef, Status, UnitCoreBuilder, canonical_uid,
    };
    let p = Project::new("spike-answer");
    let ep = endpoint("spike-answer");
    p.sidecar(&config(&ep, ""));
    let question = UnitCoreBuilder::new(KernelType::Question, "Why did the pool wait rise?", Status::Cited)
        .source(SourceRef::new(SourceKind::Doc, "ask:000000000000"))
        .build()
        .unwrap();
    let passage = UnitCoreBuilder::new(KernelType::Claim, "The pool saturated.", Status::Cited)
        .source(SourceRef::new(SourceKind::Doc, "incident report"))
        .build()
        .unwrap();
    let (q, c) = (canonical_uid(&question), canonical_uid(&passage));
    let mut store = Store::new();
    store
        .append(&[Record::Unit(question.clone()), Record::Unit(passage.clone())])
        .unwrap();
    let members = vec![
        Member {
            role: "question".into(),
            uid: q,
            status: Status::Cited,
            gist: question.gist.clone(),
        },
        Member {
            role: "evidence".into(),
            uid: c,
            status: Status::Cited,
            gist: passage.gist.clone(),
        },
    ];
    script::arm(
        &ep,
        [Answer::Text(format!(
            "@claim a/why {{ status: inferred, grounds: [{c}] }}\n~ The wait rose because the pool saturated.\n",
            c = c.canonical()
        ))],
    );
    let built = model::registry(&p.setup().config, false);
    let report = Ingestor::new(
        &built.registry,
        IngestOptions::at_rung(Rung::Model).with_agent(agent()),
    )
    .answer(&store, &members)
    .unwrap();
    let staged = report.staged.expect("the answer keeps its unit");
    let seen = kinds(&staged.records());
    for k in &seen {
        assert!(
            ["Unit", "LabelBinding", "Relation", "Attestation"].contains(&k.as_str()),
            "{seen:?}"
        );
    }
    assert_eq!(seen.first().map(String::as_str), Some("Unit"), "{seen:?}");
    assert_eq!(staged.units.len(), 1);
    script::disarm(&ep);
}

/// `Ingestor::ingest_text` over a library holding one text: it writes into
/// the library as it goes (§P4), and what it adds is units, label bindings,
/// relations and attestations, and SMYSL's own records of the run, which
/// Bund2 does not write: the spike lists them, so §P4's table is checked
/// against SMYSL rather than assumed.
#[test]
fn the_spike_a_text_ingest_writes_the_kinds_p4_lists() {
    let p = Project::new("spike-text");
    let ep = endpoint("spike-text");
    p.sidecar(&config(&ep, ""));
    let text = "the pool wait rose to 1.9 seconds\nthe rollback returned p99 to baseline\n";
    {
        let mut lib = Library::open(&p.root).unwrap();
        lib.add(
            &Input::new(text.as_bytes()),
            &AddSpec::new("txt/1", "incident", "public-domain")
                .with_lang(smysl_core::ids::LangTag::new("en").unwrap()),
            &Caps::DEFAULT,
        )
        .unwrap();
    }
    let answer = |label: &str, gist: &str, quote: &str, node: &str| {
        Answer::Text(format!(
            r#"{{"units":[{{"type":"claim","label":"{label}","gist":"{gist}","status":"cited","quote":"{quote}","node":"{node}"}}]}}"#
        ))
    };
    script::arm(
        &ep,
        [
            answer("c/pool", "the pool wait rose", "the pool wait rose to 1.9 seconds", "L1"),
            answer("c/pool", "the pool wait rose", "the pool wait rose to 1.9 seconds", "L1"),
            answer("c/back", "p99 returned", "the rollback returned p99 to baseline", "L2"),
            answer("c/back", "p99 returned", "the rollback returned p99 to baseline", "L2"),
        ],
    );
    let built = model::registry(&p.setup().config, false);
    let mut lib = Library::open(&p.root).unwrap();
    let before: Vec<Record> = lib.store().iter().cloned().collect();
    let report = {
        let mut t = TextIngest::new(
            &mut lib,
            Target::Alias("incident".into()),
            Level::new("line").unwrap(),
        )
        .with_digest_budget(0);
        Ingestor::new(
            &built.registry,
            IngestOptions::at_rung(Rung::Document)
                .with_passes(Passes::Single)
                .with_agent(agent()),
        )
        .ingest_text(&mut t)
        .unwrap()
    };
    assert!(report.windows >= 1, "{report:?}");
    let after: Vec<Record> = lib.store().iter().cloned().collect();
    let added: BTreeSet<String> = after[before.len()..].iter().map(kind).collect();
    assert!(added.contains("Unit"), "{added:?}");
    for k in &added {
        assert!(
            ["SchemaDecl", "Unit", "LabelBinding", "Relation", "Attestation"]
                .contains(&k.as_str()),
            "a text ingest wrote a kind §P4 does not list: {added:?}"
        );
    }
    script::disarm(&ep);
}

// --- the configuration (§P3, criterion 22's function-level cases) -----------------

/// The sidecar beside the library routes `content-ingest` to its provider.
#[test]
fn the_sidecar_beside_the_library_is_read() {
    let p = Project::new("sidecar");
    let ep = endpoint("sidecar");
    p.sidecar(&config(&ep, ""));
    let s = p.setup();
    assert_eq!(s.base, p.project());
    let built = model::registry(&s.config, false);
    let routed = built.registry.for_task(Task::ContentIngest).unwrap();
    assert_eq!(routed.id(), ProviderId::new("s").unwrap());
    assert_eq!(routed.caps().model, "script-1");
}

/// `opts.config` names another file, read instead, its base its own
/// directory; an absent one is SM13; a relative one SM03.
#[test]
fn a_named_configuration_is_read_instead_and_must_be_absolute_and_present() {
    let p = Project::new("named");
    p.sidecar(&config(&endpoint("named-sidecar"), ""));
    let other = p.dir.0.join("elsewhere");
    std::fs::create_dir_all(&other).unwrap();
    let file = std::fs::canonicalize(&other).unwrap().join("c.hjson");
    std::fs::write(
        &file,
        config(&endpoint("named-other"), "").replace("model: \"script-1\"", "model: \"other\""),
    )
    .unwrap();
    let s = model::configuration(&p.root, Some(&file)).unwrap();
    assert_eq!(s.base, file.parent().unwrap());
    let built = model::registry(&s.config, false);
    assert_eq!(
        built.registry.for_task(Task::ContentIngest).unwrap().caps().model,
        "other"
    );

    let absent = file.with_file_name("absent.hjson");
    let e = model::configuration(&p.root, Some(&absent)).unwrap_err();
    assert_eq!(e.code, Code::Sm13);
    assert!(e.text.starts_with("the model refused it: "), "{}", e.text);
    assert!(e.text.contains("absent.hjson"), "{}", e.text);

    let e = model::configuration(&p.root, Some(Path::new("c.hjson"))).unwrap_err();
    assert_eq!(e.code, Code::Sm03);
}

/// With no file, the configuration is `Config::local_default()`, and its
/// registry routes to the one local Ollama, built only where the build has
/// that mapper. Nothing is called.
#[test]
fn with_no_file_the_configuration_is_the_local_default() {
    let p = Project::new("default");
    let s = p.setup();
    assert_eq!(s.config, smysl_provider::Config::local_default());
    let built = model::registry(&s.config, false);
    let ollama = ProviderId::new("ollama").unwrap();
    assert_eq!(built.registry.routing().get(&Task::ContentIngest), Some(&ollama));
    if cfg!(feature = "smysl-ollama") {
        assert!(built.warnings.is_empty(), "{:?}", built.warnings);
    } else {
        // The mapper is not compiled in: skipped with a warning naming it.
        assert_eq!(built.warnings.len(), 1, "{:?}", built.warnings);
        assert!(built.warnings[0].contains("ollama"), "{:?}", built.warnings);
        assert!(built.registry.get(&ollama).is_none());
    }
}

/// A provider field named `api_key` is SM13 with SMYSL's text; a kind this
/// build lacks is a warning naming it, and the other providers are kept.
#[test]
fn a_key_in_the_file_is_refused_and_an_unknown_kind_is_skipped() {
    let p = Project::new("refusals");
    let ep = endpoint("refusals");
    p.sidecar(&config(&ep, "").replace(
        "structured: json-schema",
        "structured: json-schema\n      api_key: \"sk-not-a-key\"",
    ));
    let e = model::configuration(&p.root, None).unwrap_err();
    assert_eq!(e.code, Code::Sm13);
    assert!(
        e.text.contains("provider s has a `api_key` field"),
        "{}",
        e.text
    );

    p.sidecar(&config(&ep, "").replace(
        "  routing:",
        "    far: { kind: martian, endpoint: \"http://localhost/x\", model: \"m\" }\n  }\n  routing:",
    ).replacen("    }\n  }\n", "    }\n", 1));
    let s = p.setup();
    let built = model::registry(&s.config, false);
    assert_eq!(built.warnings.len(), 1, "{:?}", built.warnings);
    assert!(built.warnings[0].contains("far"), "{:?}", built.warnings);
    assert!(built.warnings[0].contains("martian"), "{:?}", built.warnings);
    assert!(built.registry.for_task(Task::ContentIngest).is_ok());
}

/// `ingest.prompt` is read relative to the project under the sidecar, and
/// to the file's own directory under a named configuration; a missing
/// override, or one whose `system_file` names an absent file, is SM13.
#[test]
fn the_prompt_override_is_read_relative_to_its_configuration() {
    let p = Project::new("prompt");
    let ep = endpoint("prompt");
    let with_prompt = config(&ep, "ingest: { prompt: \"prompts/p.hjson\" }");
    p.sidecar(&with_prompt);
    let s = p.setup();
    let e = model::prompt(&s).unwrap_err();
    assert_eq!(e.code, Code::Sm13, "an absent override");

    let prompts = p.project().join("prompts");
    std::fs::create_dir_all(&prompts).unwrap();
    std::fs::write(
        prompts.join("p.hjson"),
        "{ id: bund2.m11, version: 1, system: \"sidecar system\" }",
    )
    .unwrap();
    let got = model::prompt(&s).unwrap().unwrap();
    assert_eq!(got.system.as_deref(), Some("sidecar system"));

    std::fs::write(
        prompts.join("p.hjson"),
        "{ id: bund2.m11, version: 1, system_file: \"absent.txt\" }",
    )
    .unwrap();
    let e = model::prompt(&s).unwrap_err();
    assert_eq!(e.code, Code::Sm13, "{}", e.text);

    let other = std::fs::canonicalize(&p.dir.0).unwrap().join("o");
    std::fs::create_dir_all(other.join("prompts")).unwrap();
    std::fs::write(other.join("c.hjson"), &with_prompt).unwrap();
    std::fs::write(
        other.join("prompts").join("p.hjson"),
        "{ id: bund2.m11, version: 1, system: \"named system\" }",
    )
    .unwrap();
    let s = model::configuration(&p.root, Some(&other.join("c.hjson"))).unwrap();
    let got = model::prompt(&s).unwrap().unwrap();
    assert_eq!(got.system.as_deref(), Some("named system"));
}

// --- the adaptor and the script (§P3, §P6, §P8) ------------------------------------

/// The adaptor counts the calls that failed, by kind, and not a probe.
#[test]
fn the_adaptor_counts_failed_calls_by_kind() {
    let p = Project::new("adaptor");
    let ep = endpoint("adaptor");
    p.sidecar(&config(&ep, ""));
    script::arm(
        &ep,
        [
            Answer::Fail(ProviderError::Unreachable),
            Answer::Fail(ProviderError::Unreachable),
            Answer::Fail(ProviderError::Unauthorized),
            Answer::Text("ok".into()),
        ],
    );
    script::probing(&ep, Probing::Fail(ProviderError::Unreachable));
    let built = model::registry(&p.setup().config, false);
    let prov = built.registry.for_task(Task::ContentIngest).unwrap();
    let req = smysl_provider::Request::new("", "hello");
    for _ in 0..3 {
        assert!(prov.complete(&req).is_err());
    }
    assert_eq!(prov.complete(&req).unwrap().text, "ok");
    assert!(prov.probe().is_err());
    assert_eq!(built.failures.total(), 3);
    let by = built.failures.by_kind();
    assert_eq!(by.get("unreachable"), Some(&2));
    assert_eq!(by.get("unauthorized"), Some(&1));
    assert_eq!(script::probes(&ep), 1);
    assert_eq!(script::calls(&ep).len(), 4);
    script::disarm(&ep);
}

/// The script reports itself local or not from its endpoint's host, as
/// SMYSL's mappers do, so `offline` refuses a hosted one before any call.
#[test]
fn the_script_is_local_or_hosted_by_its_endpoint() {
    let p = Project::new("offline");
    let hosted = format!("https://script.example/{}", std::process::id());
    p.sidecar(&config(&hosted, ""));
    let built = model::registry(&p.setup().config, true);
    let e = match built.registry.for_task(Task::ContentIngest) {
        Ok(_) => panic!("offline reached a hosted provider"),
        Err(e) => e,
    };
    assert_eq!(e, ProviderError::OfflineViolation);
    let r = model::refused(&e);
    assert_eq!(r.code, Code::Sm13);
    assert!(r.text.starts_with("the model refused it: SMY-E301 "), "{}", r.text);
    assert!(script::calls(&hosted).is_empty());
}

/// The script's probe answers what the test set, and its actions run on the
/// n-th call.
#[test]
fn the_script_probes_as_armed_and_runs_an_action_on_a_call() {
    let p = Project::new("probe");
    let ep = endpoint("probe");
    p.sidecar(&config(&ep, ""));
    script::arm(&ep, [Answer::Text("a".into()), Answer::Text("b".into())]);
    let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = ran.clone();
    script::on_call(
        &ep,
        2,
        Box::new(move || flag.store(true, std::sync::atomic::Ordering::SeqCst)),
    );
    let built = model::registry(&p.setup().config, false);
    let prov = built.registry.for_task(Task::ContentIngest).unwrap();
    assert_eq!(prov.probe().unwrap().models, vec!["script-1".to_string()]);
    script::probing(&ep, Probing::Models(Vec::new()));
    assert!(prov.probe().unwrap().models.is_empty());
    script::probing(&ep, Probing::Unreachable);
    assert!(!prov.probe().unwrap().reachable);
    let req = smysl_provider::Request::new("", "hello").with_system("sys");
    prov.complete(&req).unwrap();
    assert!(!ran.load(std::sync::atomic::Ordering::SeqCst));
    prov.complete(&req).unwrap();
    assert!(ran.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(script::calls(&ep)[0].system, "sys");
    script::disarm(&ep);
}
