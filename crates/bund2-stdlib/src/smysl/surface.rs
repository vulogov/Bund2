//! The surface syntax (§S4): `smysl.parse`'s reading of a document into
//! §S2's MAPs, and `smysl.write`'s choice of what a library's text holds.
//!
//! Both are functions of their input and do no I/O, and each answers its
//! warnings beside its result, for the word to report (D36).

use std::collections::{BTreeMap, BTreeSet};

use bund2_value::BundValue;
use smysl_core::diag::{Code as DCode, Diagnostic, Severity, Subject};
use smysl_core::surface::{WriteContext, has_surface_form, parse_surface, unit_has_surface_form};
use smysl_core::{Label, Record, Relation, Uid, Withdrawal};
use smysl_graph::Store;

use super::{Refusal, map, sm02};

/// What a parse answers: the MAPs, or the refusal, and the warnings either
/// way, a refused document's included (§S4).
pub struct Parsed {
    pub answer: Result<Vec<BundValue>, Refusal>,
    pub warnings: Vec<String>,
}

fn without(v: BundValue, key: &str) -> BundValue {
    match v.as_map() {
        Some(m) => {
            let mut m = m.clone();
            m.remove(key);
            BundValue::map(m)
        }
        None => v,
    }
}

/// `smysl.parse`: the document's units, relations and labels as §S2's MAPs,
/// without the units' and relations' ids.
///
/// - **A document SMYSL could not parse whole is refused** with SM02: any
///   diagnostic of error severity, which is SMYSL's own `has_errors`
///   (`smysl-core/src/surface/parse.rs:121-123`), or any recovered record
///   (`:66-67`). A `ParseError` from the `@doc` header is SM02 too.
/// - Labels are read from the `LabelBinding` records only
///   (`:2596-2606`), each a label MAP.
/// - Comments and authored salience have no §S2 form, and are counted.
/// - A relation the document withdraws is left out, by `is_withdrawn`'s
///   test over the document's records: not `supersedes` or `retracts`, and
///   named by a withdrawal (`smysl-graph/src/store/mod.rs:1500-1504`).
///   A withdrawal of a lifecycle edge withdraws nothing, and draws
///   `SMY-W056`'s warning once per distinct withdrawal, as SMYSL's `check`
///   words it (`smysl-check/src/passes/integrity.rs:40-59`).
/// - Every other record has no §S2 form, and is counted by kind.
pub fn parse(text: &str) -> Parsed {
    let out = match parse_surface(text) {
        Ok(out) => out,
        Err(e) => {
            return Parsed {
                answer: Err(sm02(e)),
                warnings: Vec::new(),
            };
        }
    };
    let mut warnings: Vec<String> = out
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warn)
        .map(ToString::to_string)
        .collect();
    if out.has_errors() || out.recovered > 0 {
        let mut parts: Vec<String> = out
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(ToString::to_string)
            .collect();
        if out.recovered > 0 {
            parts.push(format!("{} records skipped", out.recovered));
        }
        return Parsed {
            answer: Err(sm02(parts.join("; "))),
            warnings,
        };
    }
    if out.comments > 0 {
        warnings.push(format!(
            "{} comment line(s) dropped: a comment is not a record",
            out.comments
        ));
    }
    if !out.salience.is_empty() {
        warnings.push(format!(
            "the authored salience of {} unit(s) dropped: a unit MAP has no salience",
            out.salience.len()
        ));
    }

    let mut declared: BTreeMap<Uid, &Relation> = BTreeMap::new();
    let mut withdrawals: BTreeSet<&Withdrawal> = BTreeSet::new();
    for r in &out.records {
        match r {
            Record::Relation(rel) => {
                declared.entry(rel.uid()).or_insert(rel);
            }
            Record::Withdrawal(w) => {
                withdrawals.insert(w);
            }
            _ => {}
        }
    }
    let withdrawn: BTreeSet<Uid> = withdrawals.iter().map(|w| w.relation).collect();
    for w in &withdrawals {
        if let Some(rel) = declared.get(&w.relation).filter(|r| r.kind.is_lifecycle()) {
            let d = Diagnostic::new(DCode::W056)
                .with_subject(Subject::Unit(rel.to))
                .with_message(format!(
                    "{} withdrew a `{}` edge; it cannot be withdrawn and still applies",
                    w.agent, rel.kind
                ));
            warnings.push(d.to_string());
        }
    }

    let mut maps = Vec::new();
    let mut left_out = 0usize;
    let mut skipped: BTreeMap<&'static str, usize> = BTreeMap::new();
    for r in &out.records {
        match r {
            Record::Unit(core) => maps.push(without(map::unit(core, &[]), "uid")),
            Record::Relation(rel) => {
                if !rel.kind.is_lifecycle() && withdrawn.contains(&rel.uid()) {
                    left_out += 1;
                } else {
                    maps.push(without(map::relation(rel), "rid"));
                }
            }
            Record::LabelBinding(b) => maps.push(map::label(b)),
            other => *skipped.entry(other.type_name()).or_insert(0) += 1,
        }
    }
    if left_out > 0 {
        warnings.push(format!(
            "{left_out} relation(s) the document withdraws left out"
        ));
    }
    for (kind, n) in &skipped {
        warnings.push(format!(
            "{n} {kind} record(s) skipped: they have no unit, relation or label MAP"
        ));
    }
    Parsed {
        answer: Ok(maps),
        warnings,
    }
}

/// What `smysl.write` answers: the text, and its warnings.
pub struct Written {
    pub text: String,
    pub warnings: Vec<String>,
}

/// The labels the writer's context spells (§S4): for each uid, the first
/// label in canonical order among those that bind that uid alone, and only
/// when its unit has a surface form (`unit_has_surface_form`,
/// `smysl-core/src/surface/write.rs:267-295`). Answers the context and the
/// labels bound to several units.
fn context(store: &Store) -> (WriteContext, BTreeMap<Label, BTreeSet<Uid>>) {
    let mut by_label: BTreeMap<Label, BTreeSet<Uid>> = BTreeMap::new();
    for r in store.iter() {
        if let Record::LabelBinding(b) = r {
            by_label.entry(b.label.clone()).or_default().insert(b.uid);
        }
    }
    let mut spelled: BTreeMap<Label, Uid> = BTreeMap::new();
    for (label, uids) in &by_label {
        if let [uid] = uids.iter().collect::<Vec<_>>().as_slice()
            && store
                .get(uid)
                .is_some_and(|u| unit_has_surface_form(&u.core))
        {
            spelled.insert(label.clone(), **uid);
        }
    }
    let shared = by_label
        .into_iter()
        .filter(|(_, uids)| uids.len() > 1)
        .collect();
    // `from_labels` keeps the first label in canonical order for each uid
    // (`smysl-core/src/surface/write.rs:51-61`).
    (WriteContext::from_labels(&spelled), shared)
}

/// `smysl.write`: the surface syntax of the library's units, relations and
/// labels, as `write_surface` gives it.
///
/// - **Every record `Store::iter` holds is passed** but attestations, which
///   have no surface form and which every Bund2 write makes, and views, so
///   the text has no `@doc` header.
/// - **Relations are passed one per rid**, as `Store::relations` resolves
///   them, each in the place of its rid's first record; a later variant
///   under the same rid is counted.
/// - **What the writer leaves out is counted** by SMYSL's own
///   `has_surface_form` (`smysl-core/src/surface/write.rs:205-221`), one
///   warning per record kind; a label bound to several units is named; an
///   `extra` the writer drops, and a second variant of an edge, are counted
///   in one more.
pub fn write(store: &Store) -> Written {
    let (ctx, shared) = context(store);
    let mut records: Vec<Record> = Vec::new();
    let mut seen: BTreeSet<Uid> = BTreeSet::new();
    let mut omitted: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut lost = 0usize;
    for r in store.iter() {
        match r {
            Record::Attestation(_) => {}
            Record::Relation(rel) => {
                let rid = rel.uid();
                let Some(resolved) = store.relation_by_id(&rid) else {
                    continue;
                };
                if seen.insert(rid) {
                    let mut resolved = resolved.clone();
                    resolved.attestations.clear();
                    if !resolved.extra.is_empty() {
                        lost += 1;
                    }
                    records.push(Record::Relation(resolved));
                } else if rel.weight != resolved.weight
                    || rel.note != resolved.note
                    || rel.extra != resolved.extra
                {
                    lost += 1;
                }
            }
            other => {
                if !has_surface_form(other, &ctx, None) {
                    *omitted.entry(other.type_name()).or_insert(0) += 1;
                } else if let Record::LabelBinding(b) = other
                    && !b.extra.is_empty()
                {
                    lost += 1;
                }
                if !matches!(other, Record::View(_)) {
                    records.push(other.clone());
                }
            }
        }
    }
    let text = smysl_core::surface::write_surface(None, &records, &ctx);

    let mut warnings = Vec::new();
    for (kind, n) in &omitted {
        let mut w = format!("{n} {kind} record(s) have no surface form and were left out");
        if *kind == "withdrawal" {
            let live = reads_back_live(store, &ctx);
            if !live.is_empty() {
                w.push_str(&format!(
                    ", so these withdrawn edges read back live: {}",
                    live.join(", ")
                ));
            }
        }
        warnings.push(w);
    }
    for (label, uids) in &shared {
        warnings.push(format!(
            "label {} binds {} units, and is written on none of them",
            label.as_str(),
            uids.len()
        ));
    }
    if lost > 0 {
        warnings.push(format!(
            "{lost} relation(s) or label binding(s) written without their extra, \
             or second variants of an edge left out"
        ));
    }
    Written { text, warnings }
}

/// The rids of held edges `is_withdrawn` reports, no withdrawal of which
/// has a surface form: the text writes the edge and none of its
/// withdrawals, so it reads back live (§S4).
fn reads_back_live(store: &Store, ctx: &WriteContext) -> Vec<String> {
    store
        .relations()
        .filter(|rel| store.is_withdrawn(rel))
        .filter(|rel| {
            let rid = rel.uid();
            !store.withdrawals().any(|w| {
                w.relation == rid && has_surface_form(&Record::Withdrawal(w.clone()), ctx, None)
            })
        })
        .map(|rel| rel.uid().canonical())
        .collect()
}
