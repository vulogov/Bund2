//! What the researching words answer (§S5), as functions of a store opened
//! by the word. None of them appends a record.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use bund2_api::Error;
use bund2_value::{BundValue, LIST, MAP, NODATA, STRING};
use smysl_core::{AgentId, Diagnostic, Hlc, Status, Uid, UnitCore, canonical_uid};
use smysl_graph::{
    EdgeSet, Lineage, RetractionPolicy, SalienceRequest, Store, TraceKind, dependents_trace,
    effective_status, labels_of, salience, trace, trace_via, view_roots,
};
use smysl_measure::text::{Corpus, TextError, texts};
use smysl_measure::{self as ms, MeasureResult, NotPersisted, Outcome, Recipe};
use smysl_pack::{PackRequest, pack};
use smysl_query::{AskOptions, Env, PlanOptions, ask};
use smysl_text::{Library, LibraryScope};

use super::{Refusal, lib, map, sm02, sm03, sm04, sm05};

/// `smysl.get`: the unit as §S2's MAP, with the labels that bind it, or
/// NODATA when the library holds no unit under that uid.
pub fn get(store: &Store, uid: &Uid) -> BundValue {
    match store.get(uid) {
        Some(u) => map::unit(&u.core, &labels_of(store, uid)),
        None => BundValue::nodata(),
    }
}

/// The walks `smysl.trace` takes: SMYSL's four `TraceKind`s, and Bund2's
/// `"dependents"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Walk {
    Trace(TraceKind),
    Dependents,
}

/// A trace kind by name, or SM05.
pub fn walk(kind: &str) -> Result<Walk, Refusal> {
    if kind == "dependents" {
        return Ok(Walk::Dependents);
    }
    TraceKind::parse(kind)
        .map(Walk::Trace)
        .ok_or_else(|| sm05("trace kind", kind))
}

/// `smysl.trace`: the lineage, each node a MAP `{ uid, depth, via }`, in
/// SMYSL's `(depth, uid)` order, the root first at depth 0.
///
/// - `"parents"`, `"grounds"` and `"both"` go through `trace`
///   (`smysl-graph/src/lineage.rs:29-55,149`);
/// - `"rests-on"` goes through `trace_via` over `EdgeSet::premises`
///   (`:222-233`), which is `deps`, `grounds` and `conditions`
///   (`smysl-graph/src/adjacency.rs:165`);
/// - `"dependents"` goes through `dependents_trace` over the same edges
///   (`smysl-graph/src/lineage.rs:389-394`).
pub fn lineage(store: &Store, root: Uid, w: Walk, depth: Option<u32>) -> BundValue {
    let l: Lineage = match w {
        Walk::Trace(TraceKind::Rests) => trace_via(store, root, &EdgeSet::premises(), depth),
        Walk::Trace(k) => trace(store, root, k, depth),
        Walk::Dependents => dependents_trace(store, root, &EdgeSet::premises(), depth),
    };
    BundValue::list(
        l.nodes
            .iter()
            .map(|n| {
                let mut m = BTreeMap::new();
                m.insert("uid".to_owned(), map::uid(&n.uid));
                m.insert("depth".to_owned(), BundValue::int(i64::from(n.depth)));
                m.insert("via".to_owned(), BundValue::str(n.via.as_str()));
                BundValue::map(m)
            })
            .collect(),
    )
}

/// `smysl.status`: the effective status under the strict policy, uid to
/// status, for every unit whose status retraction changed
/// (`effective_status`, `smysl-graph/src/merge/retraction.rs:70-128`).
pub fn status(store: &Store) -> BundValue {
    let eff = effective_status(store, RetractionPolicy::Strict);
    let changed = store
        .units()
        .filter_map(|(uid, unit)| {
            let now: Status = eff.get(uid)?;
            (now != unit.core.status).then(|| (uid.canonical(), BundValue::str(now.as_str())))
        })
        .collect();
    BundValue::map(changed)
}

/// A spec MAP's keys, checked against those its word takes.
fn spec_of<'a>(
    v: &'a BundValue,
    k: usize,
    keys: &[&str],
) -> Result<&'a BTreeMap<String, BundValue>, Refusal> {
    let m = v.as_map().filter(|_| v.dt() == MAP).ok_or_else(|| {
        sm03(format!(
            "#{k} parameter must be MAP, found {}",
            v.type_name()
        ))
    })?;
    match m.keys().find(|key| !keys.contains(&key.as_str())) {
        Some(key) => Err(sm03(format!("spec has no key {key}"))),
        None => Ok(m),
    }
}

/// A spec's `focus`: a LIST of uids, each one the library holds or not, as
/// SMYSL's own call decides.
fn focus(m: &BTreeMap<String, BundValue>) -> Result<Option<BTreeSet<Uid>>, Refusal> {
    let Some(v) = m.get("focus") else {
        return Ok(None);
    };
    let items = v
        .as_list()
        .filter(|_| v.dt() == LIST)
        .ok_or_else(|| sm03(format!("spec focus must be LIST, found {}", v.type_name())))?;
    items
        .iter()
        .map(|i| {
            let s = i.str_ref().filter(|_| i.dt() == STRING).ok_or_else(|| {
                sm03(format!(
                    "spec focus entry must be STRING, found {}",
                    i.type_name()
                ))
            })?;
            Uid::parse(s).map_err(|_| sm03(format!("spec focus entry {s} is not a uid")))
        })
        .collect::<Result<BTreeSet<_>, _>>()
        .map(Some)
}

/// The salience request SMYSL's `salience` command builds: the default
/// weights, seeded by the spec's `focus`, or else by the library's view
/// roots, which is plain PageRank when there are none
/// (`smysl:src/main.rs:5845-5884`).
fn request(store: &Store, focus: Option<BTreeSet<Uid>>) -> SalienceRequest {
    SalienceRequest::default().seeded(focus.unwrap_or_else(|| view_roots(store)))
}

/// `smysl.salience`: MAPs `{ uid, score }`, highest first, ties by
/// ascending uid (`SalienceReport::top`). The spec may name a `focus`.
pub fn ranked(store: &Store, spec: &BundValue, k: usize) -> Result<BundValue, Refusal> {
    let m = spec_of(spec, k, &["focus"])?;
    let report = salience(store, &request(store, focus(m)?));
    Ok(BundValue::list(
        report
            .top(report.scores.len())
            .into_iter()
            .map(|(uid, score)| {
                let mut m = BTreeMap::new();
                m.insert("uid".to_owned(), map::uid(&uid));
                m.insert("score".to_owned(), BundValue::float(f64::from(score)));
                BundValue::map(m)
            })
            .collect(),
    ))
}

/// `smysl.pack`: the uids `pack` selects to `spec.budget` tokens
/// (`smysl-pack/src/solve.rs:242`), in uid order, as its selection holds
/// them. The salience is the one SMYSL's `pack` command gives it, seeded by
/// the view roots (`smysl:src/main.rs:5520-5523`); the spec may name a
/// `focus`, which `pack` requires the library to hold. A refusal is SM02.
pub fn packed(store: &Store, spec: &BundValue, k: usize) -> Result<BundValue, Refusal> {
    let m = spec_of(spec, k, &["budget", "focus"])?;
    let b = m.get("budget").ok_or_else(|| sm03("spec lacks budget"))?;
    let budget = b
        .as_int()
        .filter(|_| b.dt() == bund2_value::INTEGER)
        .ok_or_else(|| {
            sm03(format!(
                "spec budget must be INTEGER, found {}",
                b.type_name()
            ))
        })?;
    let budget =
        u64::try_from(budget).map_err(|_| sm03(format!("spec budget {budget} is negative")))?;
    let mut req = PackRequest::budget(budget);
    if let Some(f) = focus(m)? {
        req = req.focusing(f);
    }
    let sal = salience(store, &request(store, None));
    let p = pack(store, &sal, &req).map_err(sm02)?;
    Ok(BundValue::list(p.selection.keys().map(map::uid).collect()))
}

/// A trace's `depth`: a non-negative INT, or NODATA for unbounded.
pub fn depth(v: &BundValue, k: usize) -> Result<Option<u32>, Refusal> {
    if v.dt() == NODATA {
        return Ok(None);
    }
    let d = v
        .as_int()
        .filter(|_| v.dt() == bund2_value::INTEGER)
        .ok_or_else(|| {
            sm03(format!(
                "#{k} parameter must be INTEGER or NODATA, found {}",
                v.type_name()
            ))
        })?;
    u32::try_from(d)
        .map(Some)
        .map_err(|_| sm03(format!("depth {d} is negative or too large")))
}

/// The scope's name: the root's file stem, as SMYSL's `measure` names it,
/// or the root whole when it has none (`smysl:src/main.rs:7355-7358`).
pub(crate) fn scope_name(root: &Path) -> String {
    root.file_stem().map_or_else(
        || root.to_string_lossy().into_owned(),
        |s| s.to_string_lossy().into_owned(),
    )
}

/// §S5's `Env`: the library's store, with its times and entities ports
/// over `scope`, and no semantic port (`smysl:src/main.rs:6313-6315`).
pub(crate) fn environment<'a>(library: &'a Library, scope: &'a LibraryScope<'a>) -> Env<'a> {
    Env::new(library.store(), scope)
        .with_times(scope)
        .with_entities(scope)
}

/// What a word answers, and the diagnostics it reports as warnings (§S8).
pub struct Reported {
    pub answer: BundValue,
    pub warnings: Vec<Diagnostic>,
}

/// `smysl.dossier`: the dossier `ask` assembles (`smysl-query/src/dossier/mod.rs:819`)
/// as `{ subject, entities, slots }`, each slot `{ name, units }`, with the
/// slots' warnings (`:333-334`). The owner is `agent`, and the clock the
/// wall clock's, advanced from `Hlc::zero`
/// (`smysl-core/src/types/provenance.rs:42-57`). Its `records` are not
/// appended [R17]. An `AskError` is SM02, as its `Display` writes it
/// (`smysl-query/src/dossier/mod.rs:219-250`).
pub fn dossier(
    library: &Library,
    root: &Path,
    agent: &AgentId,
    question: &str,
) -> Result<Reported, Refusal> {
    let scope = LibraryScope::new(scope_name(root), library);
    let env = environment(library, &scope);
    let opts = AskOptions::new(agent.clone()).with_ts(Hlc::now(&Hlc::zero(agent.clone()), agent));
    let d = ask(question, &env, &opts).map_err(sm02)?;
    let uids = |us: &[Uid]| BundValue::list(us.iter().map(map::uid).collect());
    let slots = d
        .slots
        .iter()
        .map(|slot| {
            let mut m = BTreeMap::new();
            m.insert("name".to_owned(), BundValue::str(slot.name));
            m.insert("units".to_owned(), uids(&slot.units));
            BundValue::map(m)
        })
        .collect();
    let mut m = BTreeMap::new();
    m.insert("subject".to_owned(), BundValue::str(d.subject.clone()));
    m.insert("entities".to_owned(), uids(&d.entities));
    m.insert("slots".to_owned(), BundValue::list(slots));
    Ok(Reported {
        answer: BundValue::map(m),
        warnings: d.warnings,
    })
}

/// A measure's ids: a LIST of STRINGs.
pub fn ids(v: &BundValue, k: usize) -> Result<Vec<String>, Refusal> {
    let items = v.as_list().filter(|_| v.dt() == LIST).ok_or_else(|| {
        sm03(format!(
            "#{k} parameter must be LIST, found {}",
            v.type_name()
        ))
    })?;
    items
        .iter()
        .map(|i| {
            i.str_ref()
                .filter(|_| i.dt() == STRING)
                .map(str::to_owned)
                .ok_or_else(|| {
                    sm03(format!(
                        "measure id must be STRING, found {}",
                        i.type_name()
                    ))
                })
        })
        .collect()
}

/// A measure that ran or did not, and the unit it would be.
pub type Ran = (MeasureResult, Result<UnitCore, NotPersisted>);

/// Run `ids` over the scope `text` names, in §S5's environment, as SMYSL's
/// `measure` does (`smysl:src/main.rs:7361-7438`) with its base recipe
/// (`:7448`), and each result's unit, grounded on the scope's basis
/// (`:7478-7502`). Nothing is appended [R17].
///
/// - The scope is resolved with `scope::resolve`
///   (`smysl-measure/src/scope.rs:100`); a refusal is SM04.
/// - When an id needs `TEXT`, the texts the scope names are found with
///   `texts`, whose refusal, `SMY-E461` among them, is SM04 too, since it
///   is the scope's (`smysl-measure/src/text/mod.rs:160-189`), and read
///   with `Corpus::load` at the recipe's level and analyzer (`:435-440`),
///   whose refusal is SM02 as `TextError` displays it (`:129-136`).
pub fn measure(
    library: &Library,
    root: &Path,
    ids: &[String],
    text: &str,
) -> Result<Vec<Ran>, Refusal> {
    let scope = LibraryScope::new(scope_name(root), library);
    let env = environment(library, &scope);
    let resolved = ms::resolve(text, &env, &PlanOptions::default()).map_err(|d| sm04(&d))?;
    let recipe = Recipe::new("", 0, text);
    let reads_text = ids
        .iter()
        .any(|id| ms::find(ms::ALL, id, None).is_ok_and(|m| m.needs().contains(ms::Needs::TEXT)));
    let corpus = if reads_text {
        let found = texts(text, library).map_err(|d| sm04(&[d]))?;
        Some(
            Corpus::load(library, &found, recipe.level, recipe.analyzer.as_deref()).map_err(
                |e| match e {
                    TextError::Library(e) => lib(e),
                    refused => sm02(refused),
                },
            )?,
        )
    } else {
        None
    };
    let mut input = ms::Input::new(library.store(), &resolved).with_library(library);
    if let Some(c) = &corpus {
        input = input.with_text(c);
    }
    let named: Vec<&str> = ids.iter().map(String::as_str).collect();
    Ok(ms::run(&named, &recipe, &input)
        .into_iter()
        .map(|r| {
            let unit = ms::to_unit_on(&r, &[resolved.basis.as_ref()]);
            (r, unit)
        })
        .collect())
}

/// §S5's MAP for one measure, `{ id, ok, value, uid, diagnostics }`, and the
/// diagnostics to report, each a warning (§S8).
///
/// - Ran: `value` is `gist` of the recipe and the value
///   (`smysl-measure/src/outcome.rs:268-292`), never cut, and `uid` the
///   `canonical_uid` of its unit. A result with no unit, `Unwritable` or
///   `NoBasis` (`smysl-measure/src/lib.rs:408-417`), has `uid` NODATA, and
///   the diagnostic `to_unit_on` gave is added to its `diagnostics`, with
///   its own severity, and reported.
/// - Not run: `ok` is false, `value` SMYSL's reason, and its code is among
///   the result's own `diagnostics` (`:266-274`, `:353-359`).
pub fn measured(ran: &Ran) -> Result<Reported, Error> {
    let (r, unit) = ran;
    let mut diagnostics = r.diagnostics.clone();
    let (ok, value, uid) = match (&r.outcome, unit) {
        (Outcome::Value(v), Ok(core)) => {
            (true, ms::gist(&r.recipe, v), map::uid(&canonical_uid(core)))
        }
        (Outcome::Value(v), Err(NotPersisted::Unwritable(d) | NotPersisted::NoBasis(d))) => {
            diagnostics.push(d.clone());
            (true, ms::gist(&r.recipe, v), BundValue::nodata())
        }
        (Outcome::NotRun { why, .. }, _) => (false, why.clone(), BundValue::nodata()),
        (outcome, unit) => {
            return Err(Error::internal(format!(
                "smysl.measure: a result this build has no arm for, {outcome:?} with {unit:?}; \
                 the SMYSL pin moved without the arm its new variant needs"
            )));
        }
    };
    let mut m = BTreeMap::new();
    m.insert("id".to_owned(), BundValue::str(r.recipe.id.clone()));
    m.insert("ok".to_owned(), BundValue::boolean(ok));
    m.insert("value".to_owned(), BundValue::str(value));
    m.insert("uid".to_owned(), uid);
    m.insert(
        "diagnostics".to_owned(),
        BundValue::list(
            diagnostics
                .iter()
                .map(map::diagnostic)
                .collect::<Result<_, _>>()?,
        ),
    );
    Ok(Reported {
        answer: BundValue::map(m),
        warnings: diagnostics,
    })
}
