//! `smysl.query` and the `smysl` conditional (§S6): one `sq/1` query,
//! parsed, planned and run in §S5's environment, and its answer as a MAP.
//!
//! The conditional's runner, [`run_smysl`], computes the answer in full and
//! closes the library before its lambda first runs, so it holds no lock or
//! mutex while the lambda runs, and a lambda that writes to the same library
//! does not change the units still to be visited. It calls `eval_lambda`,
//! so it is named in `REENTERING`.

use std::collections::BTreeMap;
use std::path::Path;

use bund2_api::{Error, Vm};
use bund2_value::{BundValue, LAMBDA, MAP, STRING};
use smysl_core::{Diagnostic, Uid};
use smysl_graph::labels_of;
use smysl_query::{Answer, PlanOptions};
use smysl_text::{Library, LibraryScope};

use super::handle::{self, Handle};
use super::research::{environment, scope_name};
use super::{Code, Refusal, map, sm03, sm04};

/// A query's answer, the units it names, and what was said on the way.
pub struct Queried {
    pub answer: Answer,
    pub warnings: Vec<Diagnostic>,
}

/// Parse, plan and run `text` over `library`, in §S5's environment with
/// `PlanOptions::default()`. A refusal from any step is SM04, carrying
/// every diagnostic. The warnings are the parse's (`W510`,
/// `smysl-query/src/parse.rs:22-31`) and the run's
/// (`smysl-query/src/exec.rs:99-112`), as SMYSL's `q` prints both
/// (`smysl:src/main.rs:6247-6249,10074-10076`).
pub fn query(library: &Library, root: &Path, text: &str) -> Result<Queried, Refusal> {
    let scope = LibraryScope::new(scope_name(root), library);
    let env = environment(library, &scope);
    let opts = PlanOptions::default();
    let parsed = smysl_query::parse(text).map_err(|d| sm04(&[d]))?;
    let plan = smysl_query::plan(&parsed.query, &env, &opts).map_err(|d| sm04(&d))?;
    let run = smysl_query::run(&plan, &env, &opts).map_err(|d| sm04(&d))?;
    let mut warnings = parsed.warnings;
    warnings.extend(run.warnings);
    Ok(Queried {
        answer: run.answer,
        warnings,
    })
}

fn uids(us: &[Uid]) -> BundValue {
    BundValue::list(us.iter().map(map::uid).collect())
}

fn entry(m: &mut BTreeMap<String, BundValue>, k: &str, v: BundValue) {
    m.insert(k.to_owned(), v);
}

/// §S6's MAP for an answer: `kind`, and the fields that kind has. The uids
/// are in the order `run` gives them (`smysl-query/src/exec.rs:69-83`).
pub fn answer(a: &Answer) -> Result<BundValue, Error> {
    let mut m = BTreeMap::new();
    let kind = match a {
        Answer::Scope(us) => {
            entry(&mut m, "units", uids(us));
            "scope"
        }
        Answer::Ranked(hits) => {
            let hits = hits
                .iter()
                .map(|f| {
                    let mut h = BTreeMap::new();
                    entry(&mut h, "uid", map::uid(&f.uid));
                    entry(&mut h, "score", BundValue::float(f.score));
                    BundValue::map(h)
                })
                .collect();
            entry(&mut m, "hits", BundValue::list(hits));
            "ranked"
        }
        Answer::Count(n) => {
            entry(
                &mut m,
                "count",
                BundValue::int(i64::try_from(*n).unwrap_or(i64::MAX)),
            );
            "count"
        }
        Answer::Groups(groups) => {
            let groups = groups
                .iter()
                .map(|(key, us)| {
                    let mut g = BTreeMap::new();
                    entry(&mut g, "key", BundValue::str(key.clone()));
                    entry(&mut g, "units", uids(us));
                    BundValue::map(g)
                })
                .collect();
            entry(&mut m, "groups", BundValue::list(groups));
            "groups"
        }
        Answer::Packed(us) => {
            entry(&mut m, "units", uids(us));
            "packed"
        }
        other => {
            return Err(Error::internal(format!(
                "smysl.query: an answer this build has no arm for, {other:?}; \
                 the SMYSL pin moved without the arm its new variant needs"
            )));
        }
    };
    entry(&mut m, "kind", BundValue::str(kind));
    Ok(BundValue::map(m))
}

/// The units the conditional visits: those `Answer::uids` names
/// (`smysl-query/src/exec.rs:86-95`) that the store holds, each as §S2's
/// MAP, in that order. A uid with no unit, such as a ground a `grounds`
/// closure added (`:414-418`), is not visited.
pub fn visited(library: &Library, a: &Answer) -> Vec<BundValue> {
    let store = library.store();
    a.uids()
        .iter()
        .filter_map(|u| {
            store
                .get(u)
                .map(|unit| map::unit(&unit.core, &labels_of(store, u)))
        })
        .collect()
}

/// A slot of the conditional, or SM03 naming it.
fn slot(c: &BundValue, name: &str, kind: &str, dt: u16) -> Result<BundValue, Refusal> {
    let v = c
        .get(name)
        .ok_or_else(|| sm03(format!("slot {name} is missing")))?;
    if v.dt() != dt {
        return Err(sm03(format!(
            "slot {name} must be {kind}, found {}",
            v.type_name()
        )));
    }
    Ok(v)
}

/// What the runner reads from its slots, before it opens anything.
struct Slots {
    handle: Handle,
    query: String,
    lambda: BundValue,
}

fn slots(c: &BundValue) -> Result<Slots, Refusal> {
    let h = slot(c, "handle", "MAP", MAP)?;
    let handle =
        Handle::from_value(&h, 0).map_err(|_| sm03("slot handle must be a SMYSL handle"))?;
    let query = slot(c, "query", "STRING", STRING)?
        .str_ref()
        .map(str::to_owned)
        .ok_or_else(|| sm03("slot query must be STRING"))?;
    let lambda = slot(c, "lambda", "LAMBDA", LAMBDA)?;
    Ok(Slots {
        handle,
        query,
        lambda,
    })
}

/// Run a `smysl` conditional: the lambda once per unit the answer names
/// that the store holds, with the unit's MAP pushed (§S6).
///
/// - A missing slot, or one of the wrong type, is SM03 naming the slot; a
///   missing `lambda` too, which RFC-0013's `query` would read as empty.
/// - The answer is computed, its warnings reported, and the library closed,
///   before the lambda first runs.
/// - An error inside the lambda is SM10, `lambda returns: <error>`.
pub fn run_smysl(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    const W: &str = "smysl";
    let units = slots(&c).and_then(|s| {
        let (root, library) = handle::open(&s.handle.root)?;
        let q = query(&library, &root, &s.query)?;
        Ok((s.lambda, visited(&library, &q.answer), q.warnings))
    });
    let (lambda, units, warnings) = units.map_err(|r| r.for_word(W))?;
    super::words::warn_each(vm, W, &warnings);
    for unit in units {
        vm.push(unit);
        vm.eval_lambda(&lambda).map_err(|e| {
            Refusal::new(Code::Sm10, format!("lambda returns: {}", e.0)).for_word(W)
        })?;
    }
    Ok(())
}
