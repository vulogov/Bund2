//! `smysl.kb` (§S7): the units an `sq/1` scope selects, as the facts of a
//! new knowledge base [R7].
//!
//! The scope is resolved as `smysl.query` resolves a query, in §S5's
//! environment (`smysl-measure/src/scope.rs:101-103`), and is the units the
//! answer names (`Answer::uids`, `smysl-query/src/exec.rs:86-98`) that the
//! store holds. Uids, schemas and statuses are STRING atoms, and the KB is
//! built by `Kb::build`, the validation `kb!` uses.

use std::collections::BTreeSet;

use bund2_api::Error;
use bund2_value::BundValue;
use smysl_core::{Status, Uid};
use smysl_graph::{RetractionPolicy, Store, effective_status, labels_of};

use crate::kb::store::Kb;

use super::map;

/// A fact `[ functor arg… ]`: the functor a CALL, as `[ parent tom bob ]`
/// reads, and each argument as given.
fn fact(functor: &str, args: Vec<BundValue>) -> BundValue {
    BundValue::list(
        std::iter::once(BundValue::call(functor))
            .chain(args)
            .collect(),
    )
}

/// The scope: the uids `named` holds that are units of the store, each
/// once, in the order first named. A uid with no unit, such as a ground a
/// `grounds` closure added (`smysl-query/src/exec.rs:414-418`), is left
/// out, so no fact names it as `U` and no `rel` fact ends on it.
pub fn scope(store: &Store, named: &[Uid]) -> Vec<Uid> {
    let mut seen = BTreeSet::new();
    named
        .iter()
        .filter(|u| store.get(u).is_some() && seen.insert(**u))
        .copied()
        .collect()
}

/// §S7's facts for the units of `scope`.
///
/// - `unit`, `gist`, `grounds`, `deps` and `label` for each unit, its
///   grounds and deps held or not; the status is the declared one.
/// - `effective` for each unit whose status retraction changed, under the
///   strict policy over the whole library, as `smysl.status` gives it.
/// - `rel` for each relation with both ends in scope that `is_withdrawn`
///   does not report (`smysl-graph/src/store/mod.rs:1500-1504`).
/// - `rank`, six facts, `unfounded` 0 to `measured` 5
///   (`smysl-core/src/types/epistemics.rs:21-39`).
pub fn facts(store: &Store, scope: &[Uid]) -> Vec<BundValue> {
    let eff = effective_status(store, RetractionPolicy::Strict);
    let mut out = Vec::new();
    for u in scope {
        let Some(unit) = store.get(u) else {
            continue;
        };
        let core = &unit.core;
        out.push(fact(
            "unit",
            vec![
                map::uid(u),
                BundValue::str(core.schema.to_string()),
                BundValue::str(core.status.as_str()),
            ],
        ));
        out.push(fact(
            "gist",
            vec![map::uid(u), BundValue::str(core.gist.clone())],
        ));
        for g in &core.grounds {
            out.push(fact("grounds", vec![map::uid(u), map::uid(g)]));
        }
        for d in &core.deps {
            out.push(fact("deps", vec![map::uid(u), map::uid(d)]));
        }
        for l in labels_of(store, u) {
            out.push(fact("label", vec![BundValue::str(l.as_str()), map::uid(u)]));
        }
        if let Some(now) = eff.get(u).filter(|s| *s != core.status) {
            out.push(fact(
                "effective",
                vec![map::uid(u), BundValue::str(now.as_str())],
            ));
        }
    }
    let held: BTreeSet<&Uid> = scope.iter().collect();
    for r in store.relations() {
        if held.contains(&r.from) && held.contains(&r.to) && !store.is_withdrawn(r) {
            out.push(fact(
                "rel",
                vec![
                    BundValue::str(r.kind.as_str()),
                    map::uid(&r.from),
                    map::uid(&r.to),
                ],
            ));
        }
    }
    for s in Status::ALL {
        out.push(fact(
            "rank",
            vec![
                BundValue::str(s.as_str()),
                BundValue::int(i64::from(s.as_u8())),
            ],
        ));
    }
    out
}

/// The KB of `facts`, through `Kb::build`. Every fact is a CALL functor
/// over STRING and INT atoms, none of them reserved, so a refusal is a
/// defect in Bund2 (D37).
pub fn kb(facts: &[BundValue]) -> Result<BundValue, Error> {
    Kb::build(facts).map(Kb::into_value).map_err(|r| {
        Error::internal(format!(
            "smysl.kb: Kb::build refused a fact built from the library ({}); \
             every fact is a CALL functor over STRING and INT atoms",
            r.text()
        ))
    })
}
