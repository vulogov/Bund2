//! `smysl.ingest.text` (RFC-0014 §P4, §P6, M14): one text of the library,
//! window by window, through SMYSL's `Ingestor::ingest_text`, which commits
//! each window's batch and journals it as it goes.
//!
//! The word probes the provider before anything else, then marks the root
//! (R42 A), holds it as any writer does, takes SMYSL's locks and re-opens
//! the library under them (§S9), and gives that library to SMYSL for the
//! whole run. Whole or nothing does not hold: a run that stops part way
//! leaves the windows it committed, and its journal resumes it (§P6).

use std::collections::BTreeMap;

use bund2_value::{BundValue, MAP, STRING};
use smysl_core::diag::{Code as Smy, Diagnostic};
use smysl_core::{Mid, Rung};
use smysl_ingest::text::window::leaf_level;
use smysl_ingest::text::{Target, TextIngest, TextReport};
use smysl_ingest::{IngestOptions, Ingestor, Passes};
use smysl_text::Library;
use smysl_text::limits::Caps;
use smysl_text::reading::Level;

use super::handle::{self, Handle};
use super::model::{self, refused, sm13};
use super::staged::{Answered, Opts, hex, int, opt_str, probe, usage};
use super::{Refusal, form, map, sm02, sm03};

const W: &str = "smysl.ingest.text";

/// The two messages SMYSL writes when a window's commit, or the reply
/// edges', fails (`smysl-ingest/src/text/run.rs:544-552,637-640`). The code
/// alone does not tell them apart from a model answer with no units, which
/// is `SMY-E001` too (§P6, U13).
const COMMIT_FAILED: [&str; 2] = [
    "the batch could not be committed: ",
    "the reply edges could not be committed: ",
];

fn commit_failure(d: &Diagnostic) -> bool {
    d.code == Smy::E001 && COMMIT_FAILED.iter().any(|p| d.message.starts_with(p))
}

/// `target` `#k`: an alias STRING, or a MAP `{ manifest: <id> }` (§P4).
pub(crate) fn target(v: &BundValue, k: usize) -> Result<Target, Refusal> {
    if v.dt() == STRING
        && let Some(s) = v.str_ref()
    {
        return Ok(Target::Alias(s.to_owned()));
    }
    let m = v.as_map().filter(|_| v.dt() == MAP).ok_or_else(|| {
        sm03(format!(
            "#{k} parameter must be STRING or MAP, found {}",
            v.type_name()
        ))
    })?;
    if let Some(key) = m.keys().find(|key| key.as_str() != "manifest") {
        return Err(sm03(format!("target has no key {key}")));
    }
    let id = m
        .get("manifest")
        .and_then(|v| v.str_ref().filter(|_| v.dt() == STRING))
        .ok_or_else(|| sm03("target manifest must be STRING"))?;
    let mid = Mid::parse(id).map_err(|_| sm03(format!("manifest {id} is not a manifest id")))?;
    Ok(Target::Manifest(mid))
}

/// `opts.level`, which `Level::new` must take, as SMYSL's flag is a usage
/// error otherwise (`smysl:src/main.rs:10819-10826`).
fn level(l: &str) -> Result<Level, Refusal> {
    Level::new(l).ok_or_else(|| {
        sm03(format!(
            "opts level {l} is not a level: lowercase, digits and -"
        ))
    })
}

/// SMYSL's `default_level` (`smysl:src/main.rs:11108-11151`): the head
/// manifest's first part, its structure, and the most numerous leaf level by
/// `leaf_level`. An alias or manifest the library does not hold is SM13, as
/// `ingest_text` refuses it before egress; a structure that cannot be read,
/// or has no nodes, is SM02 (§P4).
fn default_level(lib: &Library, target: &Target) -> Result<Level, Refusal> {
    let mid = match target {
        Target::Alias(alias) => lib.head(alias).map_err(|e| sm13(format!("{alias}: {e}")))?,
        Target::Manifest(mid) => *mid,
        // `Target` is `#[non_exhaustive]`; `target` makes neither of another.
        _ => return Err(sm03("that target needs an explicit level")),
    };
    let manifest = lib
        .store()
        .manifest(&mid)
        .ok_or_else(|| sm13(format!("no manifest {}", mid.canonical())))?;
    let first = manifest
        .parts
        .first()
        .ok_or_else(|| sm02("that expression has no parts"))?;
    let structure = lib
        .structure(&mid, &first.tid, &Caps::DEFAULT)
        .map_err(|e| sm02(format!("{}: {e}", first.tid.canonical())))?;
    leaf_level(&structure).ok_or_else(|| sm02("that reading has no nodes to window"))
}

/// `smysl.ingest.text` (§P4, §P6): probe, then mark, hold and lock the
/// library, run `ingest_text` over it, and answer its report.
pub(crate) fn ingest_text(h: &Handle, target: Target, o: &Opts) -> Result<Answered, Refusal> {
    let asked = o.level.as_deref().map(level).transpose()?;
    let (root, library) = handle::open(&h.root)?;
    drop(library);
    let setup = model::configuration(&root, o.config.as_deref())?;
    let prompt = model::prompt(&setup)?;
    let built = model::registry(&setup.config, o.offline);
    let mut warnings: Vec<String> = built.warnings.clone();
    // Before the mark, the hold and the locks, and even when every window
    // is journalled (§P6, R41).
    probe(&built, &setup.config, o.model.as_deref())?;

    let report = super::write::write_marked(W, &root, |w| {
        let level = match asked {
            Some(l) => l,
            None => default_level(&w.library, &target)?,
        };
        // SMYSL's own attestation for the run, at §S4's clock for the agent
        // at its start (§P7, R39).
        let now = form::Clock::of(w.library.store(), &h.agent).next();
        let mut io = IngestOptions::at_rung(Rung::Document)
            .with_agent(h.agent.clone())
            .with_now(now)
            .with_passes(Passes::TEXT);
        if let Some(m) = &o.model {
            io = io.with_model(m.clone());
        }
        if let Some(n) = o.max_output {
            io = io.with_max_output(n);
        }
        if let Some(g) = &o.granularity {
            io = io.with_granularity(g.clone());
        }
        if let Some(p) = prompt {
            io = io.with_prompt(p);
        }
        let ingestor = Ingestor::new(&built.registry, io);
        let report = w
            .within(|lib| {
                let mut t = TextIngest::new(lib, target, level);
                t.reextract = o.reextract;
                if let Some(d) = o.digest {
                    t.digest_budget = d;
                }
                ingestor.ingest_text(&mut t)
            })?
            .map_err(|e| refused(&e))?;
        // R25 over a commit SMYSL reports as a diagnostic (§P6).
        if let Some(d) = report.diagnostics.iter().find(|d| commit_failure(d)) {
            return Err(w.failed(d.message.clone()));
        }
        Ok(report)
    })?;

    let failed = built.failures.total();
    if failed > 0 {
        warnings.push(format!(
            "{failed} model call(s) failed, and SMYSL committed and journalled their windows as \
             prose; reextract extracts the text again, every window of it"
        ));
    }
    warnings.extend(report.diagnostics.iter().map(ToString::to_string));
    let value = value(&report, failed)?;
    Ok(Answered { value, warnings })
}

/// The report MAP (§P6).
fn value(r: &TextReport, failed: u64) -> Result<BundValue, Refusal> {
    let mut m: BTreeMap<String, BundValue> = BTreeMap::new();
    let mut put = |k: &str, v: BundValue| {
        m.insert(k.to_string(), v);
    };
    put("alias", BundValue::str(r.alias.clone()));
    put("manifest", opt_str(r.mid.map(|m| m.canonical())));
    put("lang", BundValue::str(r.lang.clone()));
    put("path", opt_str(r.path.map(|p| p.as_str().to_string())));
    for (k, n) in [
        ("windows", r.windows),
        ("skipped", r.skipped),
        ("calls", r.calls),
        ("passes", r.passes),
        ("units", r.units),
        ("entities", r.entities),
        ("replies", r.replies),
        ("spanned", r.spanned),
        ("spanless", r.spanless),
        ("degraded", r.degraded),
        ("repaired", r.repaired),
        ("retries", r.retries),
        ("pivoted", r.pivoted),
        ("pivot_missing", r.pivot_missing),
        ("pivot_calls", r.pivot_calls),
    ] {
        put(k, int(n));
    }
    put("failed", int(failed));
    put("usage", usage(&r.usage));
    put("recipe", r.recipe.as_ref().map_or_else(BundValue::nodata, hex));
    put("family", r.family.as_ref().map_or_else(BundValue::nodata, hex));
    let diagnostics = r
        .diagnostics
        .iter()
        .map(map::diagnostic)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| sm02(e.0))?;
    put("diagnostics", BundValue::list(diagnostics));
    Ok(BundValue::map(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two commit messages, and not a model answer with no units, which
    /// is `SMY-E001` too.
    #[test]
    fn only_smysls_commit_messages_are_commit_failures() {
        let d = |m: &str| Diagnostic::new(Smy::E001).with_message(m.to_string());
        assert!(commit_failure(&d("the batch could not be committed: disk full")));
        assert!(commit_failure(&d(
            "the reply edges could not be committed: disk full"
        )));
        assert!(!commit_failure(&d("the model answered no units")));
        assert!(!commit_failure(
            &Diagnostic::new(Smy::W304)
                .with_message("the batch could not be committed: x".to_string())
        ));
    }
}
