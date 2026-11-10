//! What a forming word writes (§S4): records and their attestations, under
//! the locks, deciding from the library as re-opened there and from what
//! the same word has already decided.
//!
//! A [`Batch`] holds one word's records until they are all decided, so a
//! refusal anywhere writes nothing: `smysl.append`'s LIST is written under
//! one acquisition of the locks, in order, or not at all.

use std::collections::{BTreeMap, BTreeSet};

use bund2_value::BundValue;
use smysl_core::{
    AgentId, Attestation, Hlc, Label, LabelBinding, Op, Record, RelKind, Relation, Rung, Status,
    Uid, canonical_uid,
};
use smysl_graph::Store;
use smysl_graph::labels::label_bindings;

use super::map::{self, Kind};
use super::{Code, Refusal, sm03};

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

/// The attestation Bund2 writes for a unit or a relation it authors (R15):
/// `Authored`, rung `Document` for a cited unit and `Computed` for every
/// other, hop 0, no parents. A conclusion `smysl.prove` writes adds its
/// grounds as parents ([`Batch::proved`]).
pub(crate) fn attestation(uid: Uid, rung: Rung, agent: &AgentId, ts: Hlc) -> Attestation {
    Attestation::new(uid, agent.clone(), Op::Authored, rung, ts)
}

pub(crate) fn rung(status: Status) -> Rung {
    if status == Status::Cited {
        Rung::Document
    } else {
        Rung::Computed
    }
}

/// One word's records, decided against the library as re-opened under the
/// locks and against what the same word decided before.
pub(crate) struct Batch<'s> {
    store: &'s Store,
    agent: AgentId,
    clock: Clock,
    records: Vec<Record>,
    /// Units this batch writes.
    units: BTreeSet<Uid>,
    /// Uids and rids this batch attests.
    attested: BTreeSet<Uid>,
    /// Relations this batch writes, by rid.
    relations: BTreeMap<Uid, Relation>,
    /// Labels this batch binds.
    labels: BTreeMap<Label, Uid>,
    /// The rids answered that `is_withdrawn` reports, in order.
    withdrawn: Vec<Uid>,
    /// Whether the batch was given a unit whose schema, or a relation whose
    /// kind, begins `x.text/`, written or already held (R29, D247).
    text_schema: bool,
}

impl<'s> Batch<'s> {
    pub(crate) fn new(store: &'s Store, agent: &AgentId) -> Batch<'s> {
        Batch {
            store,
            agent: agent.clone(),
            clock: Clock::of(store, agent),
            records: Vec::new(),
            units: BTreeSet::new(),
            attested: BTreeSet::new(),
            relations: BTreeMap::new(),
            labels: BTreeMap::new(),
            withdrawn: Vec::new(),
            text_schema: false,
        }
    }

    /// The records decided, in order, and the withdrawn rids answered.
    ///
    /// A batch given an `x.text` unit or relation, over a library with no
    /// `x.text/v1` declaration, carries SMYSL's declaration first,
    /// unattested, as SMYSL's text ingest stages it (R29, D247).
    pub(crate) fn finish(mut self) -> (Vec<Record>, Vec<Uid>) {
        if self.text_schema && !declares_text(self.store) {
            self.records
                .insert(0, Record::SchemaDecl(smysl_core::schemas::text::decl()));
        }
        (self.records, self.withdrawn)
    }

    /// Every uid a label binds, in the library and in this batch, sorted
    /// and without repeats.
    fn bindings(&self, label: &Label) -> Vec<Uid> {
        let mut uids: BTreeSet<Uid> = label_bindings(self.store, label).into_iter().collect();
        uids.extend(self.labels.get(label).copied());
        uids.into_iter().collect()
    }

    /// The one unit a label in `grounds` or `deps` binds, or SM08 (§S2,
    /// `smysl-graph/src/labels.rs:3-8,20-29`).
    fn resolve(&self, label: &Label) -> Result<Uid, Refusal> {
        match self.bindings(label).as_slice() {
            [one] => Ok(*one),
            many => Err(Refusal::new(
                Code::Sm08,
                format!("label {} binds {} units", label.as_str(), many.len()),
            )),
        }
    }

    fn holds_unit(&self, uid: &Uid) -> bool {
        self.store.contains_uid(uid) || self.units.contains(uid)
    }

    /// Attest `uid` unless this agent already has, in the library or in
    /// this batch (§S4), naming `parents`.
    fn attest(&mut self, uid: Uid, rung: Rung, parents: BTreeSet<Uid>) {
        self.attest_as(uid, |agent, ts| {
            attestation(uid, rung, agent, ts).with_parents(parents)
        });
    }

    /// Attest `uid` with the attestation `make` builds from this agent and
    /// the next clock, unless this agent already has (§S4).
    fn attest_as(&mut self, uid: Uid, make: impl FnOnce(&AgentId, Hlc) -> Attestation) {
        let held = self
            .store
            .attestations_of(&uid)
            .iter()
            .any(|a| a.agent == self.agent);
        if held || !self.attested.insert(uid) {
            return;
        }
        let ts = self.clock.next();
        self.records
            .push(Record::Attestation(make(&self.agent, ts)));
    }

    /// One record MAP, of whichever kind its keys say (§S2), answering its
    /// id: a unit's uid, a relation's rid, and for a label, the uid it binds.
    pub(crate) fn record(&mut self, m: &BTreeMap<String, BundValue>) -> Result<Uid, Refusal> {
        match map::kind(m)? {
            Kind::Unit => self.unit(m),
            Kind::Relation => self.relation(m),
            Kind::Label => {
                let b = map::label_of(m)?;
                self.label(b.label, b.uid)
            }
        }
    }

    /// One unit MAP, with its attestation.
    ///
    /// - Labels in `grounds` or `deps` are resolved against the library and
    ///   the labels this batch has bound (§S2).
    /// - A unit the library or this batch holds is not written again:
    ///   SMYSL's `append` would skip it anyway
    ///   (`smysl-graph/src/store/mod.rs:434-446`).
    /// - The attestation is written only if this agent has not attested that
    ///   uid already, so a re-append adds nothing to the log (§S4).
    pub(crate) fn unit(&mut self, m: &BTreeMap<String, BundValue>) -> Result<Uid, Refusal> {
        self.proved(m, BTreeSet::new())
    }

    /// One unit MAP whose attestation names `parents`: for a conclusion
    /// `smysl.prove` writes, its grounds (R15, §S7); for any other, none.
    pub(crate) fn proved(
        &mut self,
        m: &BTreeMap<String, BundValue>,
        parents: BTreeSet<Uid>,
    ) -> Result<Uid, Refusal> {
        let core = map::unit_core(m, &mut |l| self.resolve(l))?;
        let uid = canonical_uid(&core);
        let status = core.status;
        self.text_schema |= is_text(core.schema.as_str());
        if !self.holds_unit(&uid) {
            self.units.insert(uid);
            self.records.push(Record::Unit(core));
        }
        self.attest(uid, rung(status), parents);
        Ok(uid)
    }

    /// One relation MAP, with its attestation (§S4).
    ///
    /// - `retracts` is written only by `smysl.retract` (R23): SM03.
    /// - An edge whose rid the library or this batch holds is not written
    ///   again when its weight and note are the same, after SMYSL's clamping
    ///   and quantising; with another weight or note it is SM03, since a
    ///   second variant under one rid is a record no read sees.
    /// - An edge `is_withdrawn` reports is answered, and said so by the word.
    pub(crate) fn relation(&mut self, m: &BTreeMap<String, BundValue>) -> Result<Uid, Refusal> {
        let rel = map::relation_of(m)?;
        if rel.kind == RelKind::Retracts {
            return Err(sm03("a retracts relation is written only by smysl.retract"));
        }
        let rid = rel.uid();
        self.text_schema |= is_text(rel.kind.as_str());
        let held = self
            .relations
            .get(&rid)
            .or_else(|| self.store.relation_by_id(&rid));
        match held {
            Some(h) if h.weight == rel.weight && h.note == rel.note => {}
            Some(_) => {
                return Err(sm03(format!(
                    "rid {} is held with another weight or note",
                    rid.canonical()
                )));
            }
            None => {
                self.relations.insert(rid, rel.clone());
                self.records.push(Record::Relation(rel.clone()));
            }
        }
        self.attest(rid, Rung::Computed, BTreeSet::new());
        if self.store.is_withdrawn(&rel) {
            self.withdrawn.push(rid);
        }
        Ok(rid)
    }

    /// Bind `label` to `uid` (§S4).
    ///
    /// - `uid` must name a unit the library or this batch holds: SM03.
    /// - A label the library or this batch binds to another unit is SM08,
    ///   since nothing removes a binding (`smysl-graph/src/labels.rs:3-8`).
    /// - A label already bound to the same unit writes nothing.
    pub(crate) fn label(&mut self, label: Label, uid: Uid) -> Result<Uid, Refusal> {
        if !self.holds_unit(&uid) {
            return Err(sm03(format!(
                "uid {} names no unit in the library",
                uid.canonical()
            )));
        }
        let bound = self.bindings(&label);
        if let Some(other) = bound.iter().find(|u| **u != uid) {
            return Err(Refusal::new(
                Code::Sm08,
                format!(
                    "label {} is bound to {}, not {}",
                    label.as_str(),
                    other.canonical(),
                    uid.canonical()
                ),
            ));
        }
        if bound.is_empty() {
            self.labels.insert(label.clone(), uid);
            self.records
                .push(Record::LabelBinding(LabelBinding::new(label, uid)));
        }
        Ok(uid)
    }
}

/// How model output is attested (RFC-0014 §P7, R39 A): `Imported`, at the
/// rung SMYSL uses for the step, with the run's recipe and family.
#[cfg(feature = "smysl-model")]
#[derive(Debug, Clone, Copy)]
pub(crate) struct Imported {
    pub rung: Rung,
    pub recipe: Option<([u8; 32], [u8; 32])>,
}

#[cfg(feature = "smysl-model")]
impl Batch<'_> {
    fn imported_attestation(how: Imported, uid: Uid, agent: &AgentId, ts: Hlc) -> Attestation {
        let a = Attestation::new(uid, agent.clone(), Op::Imported, how.rung, ts);
        match how.recipe {
            Some((r, f)) => a.with_recipe(r, f),
            None => a,
        }
    }

    /// A unit a model staged, already checked (§P4): written unless the
    /// library or this batch holds it, and attested as `how` says unless this
    /// agent has attested it already. Answers its uid.
    pub(crate) fn imported_unit(&mut self, core: smysl_core::UnitCore, how: Imported) -> Uid {
        let uid = canonical_uid(&core);
        self.text_schema |= is_text(core.schema.as_str());
        if !self.holds_unit(&uid) {
            self.units.insert(uid);
            self.records.push(Record::Unit(core));
        }
        self.attest_as(uid, |agent, ts| {
            Self::imported_attestation(how, uid, agent, ts)
        });
        uid
    }

    /// A relation a model staged, already checked (§P4): written unless held
    /// with the same weight and note, which step 7 has already required.
    pub(crate) fn imported_relation(&mut self, rel: Relation, how: Imported) -> Uid {
        let rid = rel.uid();
        self.text_schema |= is_text(rel.kind.as_str());
        let held = self.relations.contains_key(&rid) || self.store.relation_by_id(&rid).is_some();
        if !held {
            self.relations.insert(rid, rel.clone());
            self.records.push(Record::Relation(rel.clone()));
        }
        self.attest_as(rid, |agent, ts| {
            Self::imported_attestation(how, rid, agent, ts)
        });
        if self.store.is_withdrawn(&rel) {
            self.withdrawn.push(rid);
        }
        rid
    }

    /// A label a model staged, which step 7 has decided may be written: bound
    /// unless the library or this batch binds it already.
    pub(crate) fn imported_label(&mut self, label: Label, uid: Uid) {
        if self.bindings(&label).is_empty() {
            self.labels.insert(label.clone(), uid);
            self.records
                .push(Record::LabelBinding(LabelBinding::new(label, uid)));
        }
    }

    /// A dossier's unit, already checked (§P4): written unless the library
    /// or this batch holds it, and attested as R15 attests Bund2's own
    /// (§P7, R50). Answers its uid.
    pub(crate) fn authored_unit(&mut self, core: smysl_core::UnitCore) -> Uid {
        let uid = canonical_uid(&core);
        let status = core.status;
        self.text_schema |= is_text(core.schema.as_str());
        if !self.holds_unit(&uid) {
            self.units.insert(uid);
            self.records.push(Record::Unit(core));
        }
        self.attest(uid, rung(status), BTreeSet::new());
        uid
    }

    /// A dossier's relation, already checked (§P4), attested as R15 attests
    /// Bund2's own (§P7, R50).
    pub(crate) fn authored_relation(&mut self, rel: Relation) -> Uid {
        let rid = rel.uid();
        self.text_schema |= is_text(rel.kind.as_str());
        let held = self.relations.contains_key(&rid) || self.store.relation_by_id(&rid).is_some();
        if !held {
            self.relations.insert(rid, rel.clone());
            self.records.push(Record::Relation(rel.clone()));
        }
        self.attest(rid, Rung::Computed, BTreeSet::new());
        if self.store.is_withdrawn(&rel) {
            self.withdrawn.push(rid);
        }
        rid
    }

    /// A record written as it is, unattested: an answer's `x.query/v1`
    /// declaration, `Thread` and `View`, as SMYSL's `ask --save` appends
    /// them (§P4, R37).
    pub(crate) fn unattested(&mut self, record: Record) {
        self.records.push(record);
    }

    /// Whether this batch writes `uid` as a new unit.
    pub(crate) fn writes(&self, uid: &Uid) -> bool {
        self.units.contains(uid)
    }
}

/// Whether a schema or a relation kind is the `x.text` schema's: SMYSL's
/// ingest test (`smysl-ingest/src/text/run.rs:734-740`).
fn is_text(name: &str) -> bool {
    name.starts_with("x.text/")
}

/// Whether the library holds the `x.text/v1` declaration, at any revision
/// (`smysl-ingest/src/text/run.rs:743-748`).
fn declares_text(store: &Store) -> bool {
    store.iter().any(
        |r| matches!(r, Record::SchemaDecl(d) if d.id.as_str() == smysl_core::schemas::text::ID),
    )
}

/// What `smysl.retract` decided: nothing to do, or the self-edge to write.
pub(crate) enum Retraction {
    /// The target was retracted directly already: nothing is written.
    Already,
    /// The `retracts` edge from the target to itself, unattested.
    Edge(Relation),
}

/// Retract `target` as SMYSL's own `retract` does
/// (`smysl:src/main.rs:4484-4505`), step for step, against the library as
/// re-opened under the locks:
///
/// 1. retracted directly already, by `effective_status`'s strict test,
///    which reads only `retracts` edges
///    (`smysl-graph/src/merge/retraction.rs:42-45,83-88`): nothing to do,
///    checked before authority as SMYSL checks it;
/// 2. planned with `plan_retraction`, the handle's agent, `Strict` and
///    `Origin` (`:181-191`);
/// 3. a refused plan is SM09, carrying SMYSL's reason;
/// 4. otherwise the self-edge, **unattested**, as SMYSL writes it (`:218`).
pub(crate) fn retract(store: &Store, agent: &AgentId, target: Uid) -> Result<Retraction, Refusal> {
    use smysl_graph::{RetractionAuthority, RetractionPolicy, effective_status, plan_retraction};
    if effective_status(store, RetractionPolicy::Strict).is_retracted(&target) {
        return Ok(Retraction::Already);
    }
    let plan = plan_retraction(
        store,
        target,
        std::slice::from_ref(agent),
        RetractionPolicy::Strict,
        RetractionAuthority::Origin,
    );
    if !plan.authorised {
        return Err(Refusal::new(
            Code::Sm09,
            format!(
                "retraction refused: {}",
                plan.refusal.as_deref().unwrap_or("refused")
            ),
        ));
    }
    Ok(Retraction::Edge(Relation::new(
        RelKind::Retracts,
        target,
        target,
    )))
}
