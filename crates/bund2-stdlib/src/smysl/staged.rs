//! RFC-0014 Phase 2's staging and commit (§P4, §P6, §P7): what
//! `smysl.ingest` answers, and what `smysl.commit` checks and writes. M12.
//!
//! - **Staging writes nothing** and holds no lock: the model is called with
//!   the library open only to read it (§P6).
//! - **Commit trusts nothing it is given** [R40]: it decodes `records`,
//!   re-applies Phase 1's write checks and rule T, runs rule M and SMYSL's
//!   check again under the locks, and refuses only the errors the batch adds
//!   [R47, R48].
//!
//! Compiled only under `smysl-model`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use bund2_api::Error;
use bund2_value::{BIN, BOOL, BundValue, FLOAT, INTEGER, MAP, NODATA, STRING};
use smysl_check::{CheckOptions, check};
use smysl_core::{
    Diagnostic, GranularityProfile, Hlc, Label, Record, RelKind, Relation, Report, Rung,
    SourcePolicy, SourceRef, Status, Uid, UnitCore, canonical_uid, from_cbor_seq,
    to_cbor_seq,
};
use smysl_graph::Store;
use smysl_graph::labels::label_bindings;
use smysl_ingest::stage::{Attest, prepare_under};
use smysl_ingest::{IngestOptions, IngestPath, Ingestor};
use smysl_provider::{Config, ProviderId, Task};

use super::handle::{self, Handle};
use super::model::{self, Built, refused, sm13};
use super::{Code, Refusal, form, map, sm03, sm05};

// --- opts ----------------------------------------------------------------------------

/// The step a word runs, which decides the keys its `opts` takes (§P4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Ingest,
    Answer,
    #[expect(dead_code, reason = "RFC-0014 M14's smysl.ingest.text takes these opts")]
    Text,
}

impl Step {
    fn keys(self) -> &'static [&'static str] {
        match self {
            Step::Ingest => &[
                "config",
                "offline",
                "model",
                "temperature",
                "path",
                "max_output",
                "granularity",
                "source",
            ],
            Step::Answer => &["config", "offline", "model", "temperature", "max_output"],
            Step::Text => &[
                "config",
                "offline",
                "model",
                "max_output",
                "granularity",
                "level",
                "reextract",
                "digest",
            ],
        }
    }
}

/// A word's `opts`, checked (§P4). A key that is absent is `None`, and is not
/// passed to SMYSL.
#[derive(Debug, Clone, Default)]
pub(crate) struct Opts {
    pub config: Option<PathBuf>,
    pub offline: bool,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub path: Option<IngestPath>,
    pub max_output: Option<usize>,
    pub granularity: Option<String>,
    pub source: Option<SourceRef>,
    pub level: Option<String>,
    pub reextract: bool,
    pub digest: Option<u32>,
}

fn wrong(key: &str, want: &str, v: &BundValue) -> Refusal {
    sm03(format!(
        "opts {key} must be {want}, found {}",
        v.type_name()
    ))
}

fn text(key: &str, v: &BundValue) -> Result<String, Refusal> {
    v.str_ref()
        .filter(|_| v.dt() == STRING)
        .map(str::to_owned)
        .ok_or_else(|| wrong(key, "STRING", v))
}

fn truth(key: &str, v: &BundValue) -> Result<bool, Refusal> {
    match v.unboxed() {
        BundValue::Bool(b, _) if v.dt() == BOOL => Ok(*b),
        _ => Err(wrong(key, "BOOL", v)),
    }
}

fn count(key: &str, v: &BundValue) -> Result<u64, Refusal> {
    match v.as_int().filter(|_| v.dt() == INTEGER) {
        Some(n) => u64::try_from(n).map_err(|_| sm03(format!("opts {key} must be 0 or more"))),
        None => Err(wrong(key, "INT", v)),
    }
}

/// `opts` `#k`, for `step`: a MAP whose keys `step` names, each of its type
/// (SM03); `temperature` from 0.0 to 2.0, as SMYSL's CLI checks it; `path`
/// `surface` or `json-ast`, `auto` being SM03 (§P4).
pub(crate) fn opts(v: &BundValue, k: usize, step: Step) -> Result<Opts, Refusal> {
    let m = v.as_map().filter(|_| v.dt() == MAP).ok_or_else(|| {
        sm03(format!(
            "#{k} parameter must be MAP, found {}",
            v.type_name()
        ))
    })?;
    let mut o = Opts::default();
    for (key, val) in m {
        if !step.keys().contains(&key.as_str()) {
            return Err(sm03(format!("opts has no key {key}")));
        }
        match key.as_str() {
            "config" => o.config = Some(PathBuf::from(text(key, val)?)),
            "offline" => o.offline = truth(key, val)?,
            "model" => o.model = Some(text(key, val)?),
            "temperature" => {
                let t = match val.unboxed() {
                    BundValue::Float(f, _) if val.dt() == FLOAT => *f,
                    BundValue::Int(i, _) if val.dt() == INTEGER => *i as f64,
                    _ => return Err(wrong(key, "FLOAT", val)),
                };
                if !(0.0..=2.0).contains(&t) {
                    return Err(sm03(format!("opts temperature {t} is outside 0.0 to 2.0")));
                }
                o.temperature = Some(t as f32);
            }
            "path" => {
                let p = text(key, val)?;
                o.path = Some(IngestPath::parse(&p).ok_or_else(|| {
                    sm03(format!("opts path {p} is not surface or json-ast"))
                })?);
            }
            "max_output" => {
                o.max_output = Some(usize::try_from(count(key, val)?).unwrap_or(usize::MAX));
            }
            "granularity" => o.granularity = Some(text(key, val)?),
            "source" => o.source = Some(map::source_ref(val)?),
            "level" => o.level = Some(text(key, val)?),
            "reextract" => o.reextract = truth(key, val)?,
            "digest" => o.digest = Some(u32::try_from(count(key, val)?).unwrap_or(u32::MAX)),
            _ => return Err(sm03(format!("opts has no key {key}"))),
        }
    }
    Ok(o)
}

// --- the probe (§P6, R41) --------------------------------------------------------------

/// Probe the provider `content-ingest` routes to, before anything is sent:
/// `for_task`, which applies `offline`, then `probe`. A probe that fails, or
/// answers unreachable, is SM13; so is a model the provider's list does not
/// offer, judged as its mapper's own probe judges it: `ollama` (and the
/// scripted provider) by prefix, an empty list being "not installed";
/// `openai`, `gemini` and `deepseek` exactly; any other kind not at all.
pub(crate) fn probe(built: &Built, config: &Config, asked: Option<&str>) -> Result<(), Refusal> {
    let p = built
        .registry
        .for_task(Task::ContentIngest)
        .map_err(|e| refused(&e))?;
    let answer = p.probe().map_err(|e| refused(&e))?;
    if !answer.reachable {
        return Err(sm13(format!(
            "provider {} is unreachable: {}",
            p.id(),
            answer.detail
        )));
    }
    let caps = p.caps();
    let model = asked.unwrap_or(caps.model.as_str());
    let kind = kind_of(config, &p.id());
    let offered = match kind.as_str() {
        "ollama" | "bund2-script" => answer.models.iter().any(|m| m.starts_with(model)),
        "openai" | "gemini" | "deepseek" => answer.models.iter().any(|m| m == model),
        _ => true,
    };
    if !offered {
        return Err(sm13(format!(
            "provider {} does not offer model {model}: {}",
            p.id(),
            answer.detail
        )));
    }
    Ok(())
}

fn kind_of(config: &Config, id: &ProviderId) -> String {
    config
        .providers
        .get(id)
        .map(|p| p.kind.clone())
        .unwrap_or_default()
}

// --- R47 and R48: what a batch adds -----------------------------------------------------

/// Whether SMYSL graded `d` an error. Read by name: this asks what SMYSL's
/// check found, and reports nothing at Bund2's `Error` severity (D36).
fn is_error(d: &Diagnostic) -> bool {
    d.severity.as_str() == "error"
}

/// One error, as the comparison keys it: code, subject and message (R48).
type Key = (String, String, String);

fn errors(report: &Report) -> BTreeMap<Key, Vec<&Diagnostic>> {
    let mut m: BTreeMap<Key, Vec<&Diagnostic>> = BTreeMap::new();
    for d in report
        .diagnostics
        .iter()
        .filter(|d| is_error(d))
    {
        m.entry((
            d.code.as_str().to_string(),
            format!("{:?}", d.subject),
            d.message.clone(),
        ))
        .or_default()
        .push(d);
    }
    m
}

/// The errors `batch` holds more often than `library`, compared as multisets
/// of code, subject and message (§P4 step 6, R47, R48).
pub(crate) fn added<'a>(library: &Report, batch: &'a Report) -> Vec<&'a Diagnostic> {
    let before = errors(library);
    let mut out = Vec::new();
    for (key, ds) in errors(batch) {
        let had = before.get(&key).map_or(0, Vec::len);
        out.extend(ds.into_iter().skip(had));
    }
    out
}

/// How many errors the library holds of its own.
fn own_errors(library: &Report) -> usize {
    library
        .diagnostics
        .iter()
        .filter(|d| is_error(d))
        .count()
}

/// The options both checks run under (§P4 step 6): the same labels, and the
/// same granularity.
fn options(labels: &BTreeMap<Label, Uid>, g: Option<&GranularityProfile>) -> CheckOptions {
    let o = CheckOptions::default().with_labels(labels.clone());
    match g {
        Some(g) => o.with_granularity(g.clone()),
        None => o,
    }
}

/// A reference of the batch to a unit neither it nor the library holds
/// (§P4 step 7, R48): a ground, a dep, or a relation's end or note, as the
/// integrity pass reads references. The referrer and the reference.
fn dangling(store: &Store, units: &[UnitCore], relations: &[Relation]) -> Option<(Uid, Uid)> {
    let batch: BTreeSet<Uid> = units.iter().map(canonical_uid).collect();
    let held = |u: &Uid| batch.contains(u) || store.contains_uid(u);
    for u in units {
        let uid = canonical_uid(u);
        if let Some(r) = u.grounds.iter().chain(u.deps.iter()).find(|r| !held(r)) {
            return Some((uid, *r));
        }
    }
    for r in relations {
        let ends = [Some(r.from), Some(r.to), r.note];
        if let Some(x) = ends.into_iter().flatten().find(|x| !held(x)) {
            return Some((r.uid(), x));
        }
    }
    None
}

fn refers(holder: &Uid, r: &Uid) -> Refusal {
    sm03(format!(
        "{} refers to {}, which the library does not hold",
        holder.canonical(),
        r.canonical()
    ))
}

/// The library's own errors, said once (R47).
fn own_warning(n: usize) -> Option<String> {
    (n > 0).then(|| {
        format!("the library holds {n} error(s) of its own, which smysl.check lists")
    })
}

// --- the staged value ------------------------------------------------------------------

/// What a word answers, and the warnings it reports first.
pub(crate) struct Answered {
    pub value: BundValue,
    pub warnings: Vec<String>,
}

fn hex(bytes: &[u8; 32]) -> BundValue {
    BundValue::str(bytes.iter().map(|b| format!("{b:02x}")).collect::<String>())
}

fn unhex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

fn int(n: impl TryInto<i64>) -> BundValue {
    n.try_into().map_or_else(|_| BundValue::int(i64::MAX), BundValue::int)
}

fn opt_str(s: Option<String>) -> BundValue {
    s.map_or_else(BundValue::nodata, BundValue::str)
}

/// The granularity profile a staged value names, or none (§P4).
fn profile(name: Option<&str>) -> Result<Option<GranularityProfile>, Refusal> {
    match name {
        None => Ok(None),
        Some(n) => IngestOptions::default()
            .with_granularity(n)
            .granularity_profile()
            .map(Some)
            .map_err(sm13),
    }
}

/// `smysl.ingest` (§P4): extract units from `input` with `Ingestor::ingest`
/// over the library's store, and answer the staged value. Writes nothing.
pub(crate) fn ingest(h: &Handle, input: &str, o: &Opts) -> Result<Answered, Refusal> {
    let (root, library) = handle::open(&h.root)?;
    let setup = model::configuration(&root, o.config.as_deref())?;
    let prompt = model::prompt(&setup)?;
    let built = model::registry(&setup.config, o.offline);
    let mut warnings: Vec<String> = built.warnings.clone();
    probe(&built, &setup.config, o.model.as_deref())?;

    let mut io = IngestOptions::default().with_agent(h.agent.clone());
    if let Some(m) = &o.model {
        io = io.with_model(m.clone());
    }
    if let Some(t) = o.temperature.or(setup.config.ingest_temperature) {
        io = io.with_temperature(t);
    }
    let path = o.path.or_else(|| {
        setup
            .config
            .ingest_path
            .as_deref()
            .and_then(IngestPath::parse)
    });
    if let Some(p) = path {
        io = io.with_path(p);
    }
    if let Some(n) = o.max_output {
        io = io.with_max_output(n);
    }
    if let Some(g) = &o.granularity {
        io = io.with_granularity(g.clone());
    }
    if let Some(s) = &o.source {
        io = io.with_source(s.clone(), SourcePolicy::FillMissing);
    }
    if let Some(p) = prompt {
        io = io.with_prompt(p);
    }

    let store = library.store();
    let (staged, report) = Ingestor::new(&built.registry, io)
        .ingest(store, input)
        .map_err(|e| refused(&e))?;

    // `clean`, judged as step 6 and step 7 judge a commit (R47, R48).
    let g = profile(o.granularity.as_deref())?;
    let library_report = check(store, options(&staged.labels, g.as_ref()));
    let added = added(&library_report, &staged.report);
    let dangles = dangling(store, &staged.units, &staged.relations);
    let failed = built.failures.total();
    let clean = added.is_empty() && dangles.is_none() && failed == 0;
    warnings.extend(own_warning(own_errors(&library_report)));
    if let Some((holder, r)) = &dangles {
        warnings.push(refers(holder, r).text);
    }
    if failed > 0 {
        warnings.push(format!(
            "{failed} model call(s) failed, so the staged value is not clean; staging again may succeed"
        ));
    }
    for d in report
        .diagnostics
        .iter()
        .filter(|d| !is_error(d))
    {
        warnings.push(d.to_string());
    }

    let mut m: BTreeMap<String, BundValue> = BTreeMap::new();
    let mut put = |k: &str, v: BundValue| {
        m.insert(k.to_string(), v);
    };
    put("type", BundValue::str("smysl.staged"));
    put("root", BundValue::str(root.to_string_lossy().into_owned()));
    put("step", BundValue::str("ingest"));
    put("granularity", opt_str(o.granularity.clone()));
    put("records", BundValue::bin(to_cbor_seq(&staged.records())));
    let mut labels_of: BTreeMap<Uid, Vec<Label>> = BTreeMap::new();
    for (l, u) in &staged.labels {
        labels_of.entry(*u).or_default().push(l.clone());
    }
    put(
        "units",
        BundValue::list(
            staged
                .units
                .iter()
                .map(|u| {
                    let ls = labels_of.get(&canonical_uid(u)).cloned().unwrap_or_default();
                    map::unit(u, &ls)
                })
                .collect(),
        ),
    );
    put(
        "relations",
        BundValue::list(staged.relations.iter().map(map::relation).collect()),
    );
    put(
        "labels",
        BundValue::list(
            staged
                .labels
                .iter()
                .map(|(l, u)| map::label(&smysl_core::LabelBinding::new(l.clone(), *u)))
                .collect(),
        ),
    );
    put("clean", BundValue::boolean(clean));
    let diagnostics = report
        .diagnostics
        .iter()
        .chain(staged.report.diagnostics.iter())
        .map(map::diagnostic)
        .collect::<Result<Vec<_>, Error>>()
        .map_err(|e| Refusal::new(Code::Sm02, e.0))?;
    put("diagnostics", BundValue::list(diagnostics));
    put("provider", opt_str(report.provider.map(|p| p.to_string())));
    put("model", opt_str(report.model.clone()));
    put("recipe", report.recipe.as_ref().map_or_else(BundValue::nodata, hex));
    put("family", report.family.as_ref().map_or_else(BundValue::nodata, hex));
    let mut usage = BTreeMap::new();
    usage.insert("input".to_string(), int(report.usage.input_tokens));
    usage.insert("output".to_string(), int(report.usage.output_tokens));
    usage.insert(
        "estimated".to_string(),
        BundValue::boolean(report.usage.estimated),
    );
    usage.insert("retries".to_string(), int(report.usage.retries));
    put("usage", BundValue::map(usage));
    let mut counts = BTreeMap::new();
    counts.insert("chunks".to_string(), int(report.chunks));
    counts.insert("calls".to_string(), int(report.calls));
    counts.insert("degraded".to_string(), int(report.degraded));
    counts.insert("repaired".to_string(), int(report.repaired));
    counts.insert("failed".to_string(), int(failed));
    put("counts", BundValue::map(counts));
    Ok(Answered {
        value: BundValue::map(m),
        warnings,
    })
}

// --- the commit (§P4, R40) -------------------------------------------------------------

/// What `smysl.commit` reports, whether it wrote or refused: its warnings,
/// the added errors among them.
pub(crate) struct Committed {
    pub result: Result<BundValue, Refusal>,
    pub warnings: Vec<String>,
}

fn field<'a>(m: &'a BTreeMap<String, BundValue>, key: &str) -> Result<&'a BundValue, Refusal> {
    m.get(key)
        .ok_or_else(|| sm03(format!("staged value lacks {key}")))
}

/// The record kinds a step makes (§P4, Q59).
fn kind_made(step: Step, r: &Record) -> bool {
    match r {
        Record::Unit(_)
        | Record::LabelBinding(_)
        | Record::Relation(_)
        | Record::Attestation(_) => true,
        Record::Thread(_) | Record::View(_) => step == Step::Answer,
        Record::SchemaDecl(d) => {
            step == Step::Answer && d.id.as_str() == smysl_core::schemas::query::ID
        }
        _ => false,
    }
}

fn kind_name(r: &Record) -> String {
    format!("{r:?}")
        .split(['(', ' ', '{'])
        .next()
        .unwrap_or_default()
        .to_string()
}

/// `smysl.commit` (§P4): steps 1–4 before the locks, 5–9 under them.
pub(crate) fn commit(h: &Handle, staged: &BundValue) -> Committed {
    let mut warnings = Vec::new();
    let result = committed(h, staged, &mut warnings);
    Committed { result, warnings }
}

fn committed(
    h: &Handle,
    staged: &BundValue,
    warnings: &mut Vec<String>,
) -> Result<BundValue, Refusal> {
    let m = staged
        .as_map()
        .filter(|_| staged.dt() == MAP)
        .filter(|m| m.get("type").and_then(BundValue::str_ref) == Some("smysl.staged"))
        .ok_or_else(|| sm03("#1 parameter is not a staged value"))?;

    // 1. The root.
    let canon = handle::canonical(&h.root)?;
    let root = field(m, "root")?;
    if root.str_ref() != Some(canon.to_string_lossy().as_ref()) {
        return Err(sm03(format!(
            "staged batch belongs to {}",
            root.str_ref().unwrap_or_default()
        )));
    }
    // 2. `clean`.
    let clean = field(m, "clean")?;
    if !matches!(clean.unboxed(), BundValue::Bool(true, _)) {
        return Err(sm03("staged batch did not check clean"));
    }
    // 3. `records` decodes whole.
    let undecodable = || sm03("records do not decode");
    let bytes = field(m, "records")?
        .as_bin()
        .filter(|_| field(m, "records").map(BundValue::dt) == Ok(BIN))
        .ok_or_else(undecodable)?;
    let (records, used) = from_cbor_seq(bytes).map_err(|_| undecodable())?;
    if used != bytes.len() {
        return Err(undecodable());
    }
    // 4. The kinds the step makes.
    let step = match field(m, "step")?.str_ref() {
        Some("ingest") => Step::Ingest,
        Some("answer") => Step::Answer,
        _ => return Err(sm03("staged step is neither ingest nor answer")),
    };
    if let Some((i, r)) = records.iter().enumerate().find(|(_, r)| !kind_made(step, r)) {
        return Err(sm03(format!(
            "record {i} is a {}, which {} does not make",
            kind_name(r),
            if step == Step::Ingest { "ingest" } else { "answer" }
        )));
    }
    if step == Step::Answer {
        return Err(sm03("staged step answer is not committed by this build"));
    }
    let granularity = match field(m, "granularity")? {
        g if g.dt() == NODATA => None,
        g => Some(g.str_ref().map(str::to_owned).ok_or_else(|| {
            sm03(format!("staged granularity must be STRING, found {}", g.type_name()))
        })?),
    };
    let g = profile(granularity.as_deref())?;
    let recipe = match (m.get("recipe").and_then(BundValue::str_ref), m.get("family").and_then(BundValue::str_ref)) {
        (Some(r), Some(f)) => Some((
            unhex(r).ok_or_else(|| sm03("staged recipe is not a hash"))?,
            unhex(f).ok_or_else(|| sm03("staged family is not a hash"))?,
        )),
        _ => None,
    };
    let rung = Rung::Document;

    super::write::write("smysl.commit", &canon, |w| {
        let store = w.library.store();

        // 5. Phase 1's write checks that do not depend on a uid, and rule T;
        //    the first label pass; staged attestations dropped.
        let mut units: Vec<UnitCore> = Vec::new();
        let mut relations: Vec<Relation> = Vec::new();
        let mut labels: BTreeMap<Label, Uid> = BTreeMap::new();
        for r in &records {
            match r {
                Record::Unit(u) => {
                    if matches!(u.status, Status::Unfounded | Status::Measured) {
                        return Err(sm03(format!(
                            "status {} cannot be authored",
                            u.status.as_str()
                        )));
                    }
                    if u.status > smysl_ingest::ceiling::ceiling(rung) {
                        return Err(sm03(format!(
                            "{} exceeds the {rung} ceiling",
                            canonical_uid(u).canonical()
                        )));
                    }
                    units.push(u.clone());
                }
                Record::Relation(rel) => {
                    if RelKind::parse(rel.kind.as_str()).is_err() {
                        return Err(sm05("relation kind", rel.kind.as_str()));
                    }
                    if rel.kind == RelKind::Retracts {
                        return Err(sm03("a retracts relation is written only by smysl.retract"));
                    }
                    relations.push(rel.clone());
                }
                Record::LabelBinding(b) => {
                    match label_bindings(store, &b.label)
                        .into_iter()
                        .find(|u| *u != b.uid)
                    {
                        Some(other) => warnings.push(dropped(&b.label, &other, &b.uid)),
                        None => {
                            labels.insert(b.label.clone(), b.uid);
                        }
                    }
                }
                _ => {}
            }
        }

        // 6. Rule M and SMYSL's checks, again; the library alone under the
        //    same options; only what the batch adds refuses.
        let attest = Attest::new(h.agent.clone(), rung, Hlc::zero(h.agent.clone()));
        let prepared = prepare_under(store, units, relations, labels, Vec::new(), &attest, g.clone());
        let library_report = check(store, options(&prepared.labels, g.as_ref()));
        warnings.extend(own_warning(own_errors(&library_report)));
        let added = added(&library_report, &prepared.report);
        if !added.is_empty() {
            warnings.extend(added.iter().map(|d| d.to_string()));
            return Err(sm03("staged batch no longer checks clean"));
        }

        // 7. The checks that depend on a uid, after the remap.
        if let Some((holder, r)) = dangling(store, &prepared.units, &prepared.relations) {
            return Err(refers(&holder, &r));
        }
        for rel in &prepared.relations {
            if let Some(held) = store.relation_by_id(&rel.uid())
                && (held.weight != rel.weight || held.note != rel.note)
            {
                return Err(sm03(format!(
                    "rid {} is held with another weight or note",
                    rel.uid().canonical()
                )));
            }
        }
        let mut kept: Vec<(Label, Uid)> = Vec::new();
        for (l, u) in &prepared.labels {
            match label_bindings(store, l).into_iter().find(|x| x != u) {
                Some(other) => warnings.push(dropped(l, &other, u)),
                None => kept.push((l.clone(), *u)),
            }
        }

        // 8 and 9. Written through Phase 1's batch, which adds D247's
        //    declaration and §P7's attestations, in one `append_records`.
        let how = form::Imported { rung, recipe };
        let (records, withdrawn, written) = {
            let mut b = form::Batch::new(store, &h.agent);
            let mut written = Vec::new();
            for u in &prepared.units {
                let uid = b.imported_unit(u.clone(), how);
                if b.writes(&uid) && !written.contains(&uid) {
                    written.push(uid);
                }
            }
            for (l, u) in kept {
                b.imported_label(l, u);
            }
            for rel in &prepared.relations {
                b.imported_relation(rel.clone(), how);
            }
            let (records, withdrawn) = b.finish();
            (records, withdrawn, written)
        };
        for rid in &withdrawn {
            warnings.push(format!(
                "edge {} is withdrawn: nothing that interprets the graph follows it",
                rid.canonical()
            ));
        }
        w.append(&records)?;
        Ok(BundValue::list(written.iter().map(map::uid).collect()))
    })
}

/// R46's warning: a staged label the library binds to another unit.
fn dropped(label: &Label, other: &Uid, uid: &Uid) -> String {
    format!(
        "label {} is bound to {} in the library; {} is written without it",
        label.as_str(),
        other.canonical(),
        uid.canonical()
    )
}
