//! What a forming word writes (§S4): a record and its attestation, under
//! the locks, deciding from the library as re-opened there.

use std::collections::BTreeMap;

use bund2_value::BundValue;
use smysl_core::{
    AgentId, Attestation, Hlc, Label, Op, Record, Rung, Status, Uid, UnitCore, canonical_uid,
};
use smysl_graph::Store;
use smysl_graph::labels::{LabelError, resolve_label};

use super::map;
use super::write::Writer;
use super::{Code, Refusal};

/// SM08 for a label in `grounds` or `deps` bound to several units, or to
/// none (`LabelError::Unbound`, `smysl-graph/src/labels.rs:20-29`).
pub(crate) fn resolve(store: &Store, label: &Label) -> Result<Uid, Refusal> {
    resolve_label(store, label).map_err(|e| {
        let n = match &e {
            LabelError::Unbound => 0,
            LabelError::Ambiguous(uids) => uids.len(),
            // A variant added after the pin names no count; SMYSL's text
            // still says why.
            _ => return Refusal::new(Code::Sm08, format!("label {} {e}", label.as_str())),
        };
        Refusal::new(
            Code::Sm08,
            format!("label {} binds {n} units", label.as_str()),
        )
    })
}

/// The clock an agent's next attestation takes (§S4): `Hlc::now` from the
/// newest of this agent's clocks in the library as re-opened under the
/// locks, or from `Hlc::zero`; within one word, each from the one before.
pub(crate) struct Clock {
    agent: AgentId,
    prev: Hlc,
}

impl Clock {
    pub(crate) fn of(store: &Store, agent: &AgentId) -> Clock {
        let prev = store
            .iter()
            .filter_map(|r| match r {
                Record::Attestation(a) if &a.agent == agent => Some(&a.ts),
                _ => None,
            })
            .max()
            .cloned()
            .unwrap_or_else(|| Hlc::zero(agent.clone()));
        Clock {
            agent: agent.clone(),
            prev,
        }
    }

    pub(crate) fn next(&mut self) -> Hlc {
        let now = Hlc::now(&self.prev, &self.agent);
        self.prev = now.clone();
        now
    }
}

/// Whether `agent` has attested `uid` in `store` already.
pub(crate) fn attested(store: &Store, uid: &Uid, agent: &AgentId) -> bool {
    store.attestations_of(uid).iter().any(|a| &a.agent == agent)
}

/// The attestation Bund2 writes for a unit it authors (R15): `Authored`,
/// rung `Document` for a cited unit and `Computed` for every other, hop 0,
/// no parents.
pub(crate) fn attestation(uid: Uid, status: Status, agent: &AgentId, ts: Hlc) -> Attestation {
    let rung = if status == Status::Cited {
        Rung::Document
    } else {
        Rung::Computed
    };
    Attestation::new(uid, agent.clone(), Op::Authored, rung, ts)
}

/// Write one unit MAP under the locks, with its attestation, and answer its
/// uid.
///
/// - Labels in `grounds` or `deps` are resolved against the library as
///   re-opened under the locks (§S2).
/// - A unit the library holds is not written again: SMYSL's `append` would
///   skip it anyway (`smysl-graph/src/store/mod.rs:434-446`).
/// - The attestation is written only if this agent has not attested that
///   uid already, so a re-append adds nothing to the log (§S4).
pub(crate) fn unit(
    w: &mut Writer,
    agent: &AgentId,
    m: &BTreeMap<String, BundValue>,
) -> Result<Uid, Refusal> {
    let store = w.library.store();
    let core: UnitCore = map::unit_core(m, &mut |l| resolve(store, l))?;
    let uid = canonical_uid(&core);
    let mut records = Vec::new();
    let status = core.status;
    if !store.contains_uid(&uid) {
        records.push(Record::Unit(core));
    }
    if !attested(store, &uid, agent) {
        let ts = Clock::of(store, agent).next();
        records.push(Record::Attestation(attestation(uid, status, agent, ts)));
    }
    w.append(&records)?;
    Ok(uid)
}
