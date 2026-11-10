//! Records read back as MAPs, and MAPs written as records (§S2).
//!
//! **Read back.** A unit, a source, a relation, a label binding, a
//! diagnostic and a payload tree each become a MAP, with uids in their full
//! text form `b3:…`. A unit or a relation read back is a view: its `extra`,
//! its source's `extra`, a payload that does not decode and its attestations
//! are left out, and `opaque = true` says that something was.
//!
//! **Written.** A unit, relation or label MAP becomes the record SMYSL
//! builds from it. A MAP's kind is decided by its keys (§S2); a key its kind
//! does not have, a computed key, or a value with no SMYSL form is SM03; an
//! unknown status, relation kind or source kind is SM05; what SMYSL's own
//! builder refuses is SM02, with its text.
//!
//! The functions here do no I/O. Resolving a label in `grounds` or `deps`
//! needs the library, so the caller passes a resolver.

use std::collections::BTreeMap;

use bund2_api::Error;
use bund2_value::{BOOL, BundValue, FLOAT, INTEGER, LIST, MAP, NODATA, STRING};
use smysl_core::diag::{Diagnostic, Subject};
use smysl_core::surface::{HObject, HValue, Spanned, object_to_payload, payload_to_object};
use smysl_core::{
    Date, Label, LabelBinding, LangTag, Mid, RelKind, Relation, SchemaId, SourceKind, SourceRef,
    Span, Status, Uid, UnitCore, UnitCoreBuilder, canonical_uid,
};

use super::{Refusal, sm02, sm03, sm05};

// --- read back ---------------------------------------------------------------

/// A uid as a STRING, its full text form. SMYSL's `Display` writes the
/// short form (`smysl-core/src/ids.rs:139-143`), which `Uid::parse` refuses,
/// so it is never the one given to a program.
pub fn uid(u: &Uid) -> BundValue {
    BundValue::str(u.canonical())
}

fn entry(m: &mut BTreeMap<String, BundValue>, key: &str, v: BundValue) {
    m.insert(key.to_owned(), v);
}

/// A unit as §S2's MAP, with `labels` the labels that bind it.
pub fn unit(core: &UnitCore, labels: &[Label]) -> BundValue {
    let mut m = BTreeMap::new();
    entry(&mut m, "uid", uid(&canonical_uid(core)));
    entry(&mut m, "schema", BundValue::str(core.schema.to_string()));
    entry(&mut m, "gist", BundValue::str(core.gist.clone()));
    if let Some(b) = &core.body {
        entry(&mut m, "body", BundValue::str(b.clone()));
    }
    if let Some(d) = &core.detail {
        entry(&mut m, "detail", BundValue::str(d.clone()));
    }
    entry(&mut m, "status", BundValue::str(core.status.as_str()));
    if !core.grounds.is_empty() {
        entry(
            &mut m,
            "grounds",
            BundValue::list(core.grounds.iter().map(uid).collect()),
        );
    }
    if !core.deps.is_empty() {
        entry(
            &mut m,
            "deps",
            BundValue::list(core.deps.iter().map(uid).collect()),
        );
    }
    let mut opaque = !core.extra.is_empty();
    if let Some(s) = &core.source {
        let (v, hidden) = source(s);
        entry(&mut m, "source", v);
        opaque |= hidden;
    }
    if let Some(l) = &core.lang {
        entry(&mut m, "lang", BundValue::str(l.as_str()));
    }
    if !labels.is_empty() {
        entry(
            &mut m,
            "labels",
            BundValue::list(labels.iter().map(|l| BundValue::str(l.as_str())).collect()),
        );
    }
    if let Some(bytes) = &core.payload {
        match payload(bytes) {
            Some(tree) => entry(&mut m, "payload", tree),
            None => opaque = true,
        }
    }
    if opaque {
        entry(&mut m, "opaque", BundValue::boolean(true));
    }
    BundValue::map(m)
}

/// A source as §S2's MAP, and whether anything of it was left out: its
/// `extra`, or a kind this build cannot name, which reads back as the raw
/// code SMYSL keeps (`smysl-core/src/types/epistemics.rs:180-191,432-439`).
pub fn source(s: &SourceRef) -> (BundValue, bool) {
    let mut m = BTreeMap::new();
    let unknown = s.kind == SourceKind::Unknown;
    let kind = if unknown {
        BundValue::int(i64::from(s.kind_code()))
    } else {
        BundValue::str(s.kind.as_str())
    };
    entry(&mut m, "kind", kind);
    entry(&mut m, "reference", BundValue::str(s.reference.clone()));
    if let Some(d) = s.captured {
        entry(&mut m, "captured", BundValue::str(d.to_string()));
    }
    if let Some(ms) = s.observed {
        // Above `i64::MAX` a program's INT cannot hold it: its decimal
        // digits, which `observed` below writes back from.
        let v = i64::try_from(ms).map_or_else(|_| BundValue::str(ms.to_string()), BundValue::int);
        entry(&mut m, "observed", v);
    }
    if let Some(p) = &s.published {
        entry(&mut m, "published", BundValue::str(p.clone()));
    }
    if let Some((a, b)) = s.span() {
        entry(
            &mut m,
            "span",
            BundValue::list(vec![int_of_u64(a), int_of_u64(b)]),
        );
    }
    if let Some(mid) = s.manifest() {
        entry(&mut m, "manifest", BundValue::str(mid.canonical()));
    }
    (BundValue::map(m), unknown || !s.extra.is_empty())
}

fn int_of_u64(n: u64) -> BundValue {
    i64::try_from(n).map_or_else(|_| BundValue::str(n.to_string()), BundValue::int)
}

/// A relation as §S2's MAP. No Phase 1 word reads one back from a library;
/// the words a later phase adds will (§S2).
pub fn relation(r: &Relation) -> BundValue {
    let mut m = BTreeMap::new();
    entry(&mut m, "rid", uid(&r.uid()));
    entry(&mut m, "kind", BundValue::str(r.kind.as_str()));
    entry(&mut m, "from", uid(&r.from));
    entry(&mut m, "to", uid(&r.to));
    if let Some(w) = r.weight {
        entry(&mut m, "weight", BundValue::float(f64::from(w)));
    }
    if let Some(n) = &r.note {
        entry(&mut m, "note", uid(n));
    }
    if !r.extra.is_empty() {
        entry(&mut m, "opaque", BundValue::boolean(true));
    }
    BundValue::map(m)
}

/// A label binding as `{ label, uid }`, with `opaque` only when its `extra`
/// is not empty (`smysl-core/src/types/annex.rs:488-493`).
pub fn label(b: &LabelBinding) -> BundValue {
    let mut m = BTreeMap::new();
    entry(&mut m, "label", BundValue::str(b.label.as_str()));
    entry(&mut m, "uid", uid(&b.uid));
    if !b.extra.is_empty() {
        entry(&mut m, "opaque", BundValue::boolean(true));
    }
    BundValue::map(m)
}

/// A diagnostic as §S2's MAP. A subject of a kind added after the pin is
/// `Error::internal` (§S2, D37): only a change of the pin brings one.
pub fn diagnostic(d: &Diagnostic) -> Result<BundValue, Error> {
    let mut m = BTreeMap::new();
    entry(&mut m, "code", BundValue::str(d.code.as_str()));
    entry(&mut m, "severity", BundValue::str(d.severity.as_str()));
    let (kind, value) = match &d.subject {
        Subject::Unit(u) => ("unit", uid(u)),
        Subject::Record(h) => (
            "record",
            BundValue::str(h.iter().map(|b| format!("{b:02x}")).collect::<String>()),
        ),
        Subject::Span(s) => (
            "span",
            BundValue::list(vec![int_of_usize(s.start), int_of_usize(s.end)]),
        ),
        Subject::Store => ("store", BundValue::nodata()),
        other => {
            return Err(Error::internal(format!(
                "smysl: a diagnostic subject this build has no arm for, {other:?}; \
                 the SMYSL pin moved without the arm its new variant needs"
            )));
        }
    };
    let mut subject = BTreeMap::new();
    entry(&mut subject, "kind", BundValue::str(kind));
    entry(&mut subject, "value", value);
    entry(&mut m, "subject", BundValue::map(subject));
    entry(&mut m, "message", BundValue::str(d.message.clone()));
    if let Some(s) = &d.suggestion {
        entry(&mut m, "suggestion", BundValue::str(s.clone()));
    }
    Ok(BundValue::map(m))
}

fn int_of_usize(n: usize) -> BundValue {
    i64::try_from(n).map_or_else(|_| BundValue::str(n.to_string()), BundValue::int)
}

/// A payload as its decoded tree, or `None` when it does not decode.
pub fn payload(bytes: &[u8]) -> Option<BundValue> {
    payload_to_object(bytes).ok().map(|o| object(&o))
}

fn object(o: &HObject) -> BundValue {
    let mut m = BTreeMap::new();
    for (k, v) in o.iter() {
        // A key given twice is the first one's, as `HObject::get` reads it.
        m.entry(k.value.clone()).or_insert_with(|| tree(&v.value));
    }
    BundValue::map(m)
}

fn tree(v: &HValue) -> BundValue {
    match v {
        HValue::Null => BundValue::nodata(),
        HValue::Bool(b) => BundValue::boolean(*b),
        HValue::Int(i) => BundValue::int(*i),
        HValue::Float(f) => BundValue::float(*f),
        HValue::Str(s) => BundValue::str(s.clone()),
        HValue::Array(a) => BundValue::list(a.iter().map(|i| tree(&i.value)).collect()),
        HValue::Object(o) => object(o),
    }
}

// --- written -----------------------------------------------------------------

/// What a record MAP is, by its keys (§S2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Unit,
    Relation,
    Label,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Unit => "unit",
            Kind::Relation => "relation",
            Kind::Label => "label",
        }
    }
}

/// A MAP's kind, decided in §S2's order: `label` first, then `kind`,
/// `from` and `to` together, then `gist`. Anything else is SM03, naming
/// what is missing when it can.
pub fn kind(m: &BTreeMap<String, BundValue>) -> Result<Kind, Refusal> {
    let has = |k: &str| m.contains_key(k);
    if has("label") {
        return Ok(Kind::Label);
    }
    let rel = ["kind", "from", "to"];
    if rel.iter().all(|k| has(k)) {
        return Ok(Kind::Relation);
    }
    if has("gist") {
        return Ok(Kind::Unit);
    }
    if let Some(missing) = rel
        .iter()
        .find(|k| !has(k))
        .filter(|_| rel.iter().any(|k| has(k)))
    {
        return Err(sm03(format!("relation lacks {missing}")));
    }
    if has("schema") {
        return Err(sm03("unit lacks gist"));
    }
    Err(sm03("record is not a unit, relation or label"))
}

/// The keys a computed id or a view's flag would be given under.
const COMPUTED: [&str; 3] = ["uid", "rid", "opaque"];

/// SM03 for any key `allowed` does not name: a computed one as computed,
/// any other as one its kind does not have.
fn only(kind: Kind, m: &BTreeMap<String, BundValue>, allowed: &[&str]) -> Result<(), Refusal> {
    for k in m.keys() {
        if allowed.contains(&k.as_str()) {
            continue;
        }
        if COMPUTED.contains(&k.as_str()) {
            return Err(sm03(format!("{k} is computed, not given")));
        }
        if kind == Kind::Unit && k == "labels" {
            return Err(sm03(
                "labels is read back only: a label is bound with smysl.label or a label MAP",
            ));
        }
        return Err(sm03(format!("{} has no key {k}", kind.name())));
    }
    Ok(())
}

fn found(v: &BundValue) -> &'static str {
    v.type_name()
}

fn text<'a>(key: &str, v: &'a BundValue) -> Result<&'a str, Refusal> {
    v.str_ref()
        .filter(|_| v.dt() == STRING)
        .ok_or_else(|| sm03(format!("{key} must be STRING, found {}", found(v))))
}

fn required<'a>(
    kind: Kind,
    m: &'a BTreeMap<String, BundValue>,
    key: &str,
) -> Result<&'a BundValue, Refusal> {
    m.get(key)
        .ok_or_else(|| sm03(format!("{} lacks {key}", kind.name())))
}

fn parse_uid(key: &str, s: &str) -> Result<Uid, Refusal> {
    Uid::parse(s).map_err(|_| sm03(format!("{key} {s} is not a uid")))
}

/// A unit MAP as the core SMYSL builds from it. `resolve` answers the uid a
/// label in `grounds` or `deps` binds, or SM08 (§S2).
pub fn unit_core(
    m: &BTreeMap<String, BundValue>,
    resolve: &mut dyn FnMut(&Label) -> Result<Uid, Refusal>,
) -> Result<UnitCore, Refusal> {
    const KEYS: [&str; 10] = [
        "schema", "gist", "body", "detail", "status", "grounds", "deps", "source", "lang",
        "payload",
    ];
    only(Kind::Unit, m, &KEYS)?;
    let schema_text = text("schema", required(Kind::Unit, m, "schema")?)?;
    let schema = SchemaId::parse(schema_text)
        .map_err(|e| sm03(format!("schema {schema_text} is refused: {e}")))?;
    let gist = text("gist", required(Kind::Unit, m, "gist")?)?;
    let status = status(text("status", required(Kind::Unit, m, "status")?)?)?;
    let mut b = UnitCoreBuilder::new(schema, gist, status);
    if let Some(v) = m.get("body") {
        b = b.body(text("body", v)?);
    }
    if let Some(v) = m.get("detail") {
        b = b.detail(text("detail", v)?);
    }
    if let Some(v) = m.get("grounds") {
        b = b.grounds(references("grounds", v, resolve)?);
    }
    if let Some(v) = m.get("deps") {
        b = b.deps(references("deps", v, resolve)?);
    }
    if let Some(v) = m.get("source") {
        b = b.source(source_ref(v)?);
    }
    if let Some(v) = m.get("lang") {
        let l = text("lang", v)?;
        b = b.lang(LangTag::new(l).map_err(|e| sm03(format!("lang {l} is refused: {e}")))?);
    }
    if let Some(v) = m.get("payload")
        && let Some(bytes) = payload_bytes(v)?
    {
        b = b.payload(bytes);
    }
    b.build().map_err(sm02)
}

/// A status a program may author. `unfounded` is reached only by
/// retraction, and `measured` only by a measure (R15): both SM03.
fn status(s: &str) -> Result<Status, Refusal> {
    match Status::parse(s) {
        None => Err(sm05("status", s)),
        Some(st @ (Status::Unfounded | Status::Measured)) => {
            Err(sm03(format!("status {} cannot be authored", st.as_str())))
        }
        Some(st) => Ok(st),
    }
}

/// `grounds` or `deps`: a LIST of STRINGs, each a uid when `Uid::parse`
/// accepts it and a label when `Label::new` does. A uid begins `b3:`, so no
/// STRING is both; one that is neither is SM03.
fn references(
    key: &str,
    v: &BundValue,
    resolve: &mut dyn FnMut(&Label) -> Result<Uid, Refusal>,
) -> Result<Vec<Uid>, Refusal> {
    let items = v
        .as_list()
        .filter(|_| v.dt() == LIST)
        .ok_or_else(|| sm03(format!("{key} must be LIST, found {}", found(v))))?;
    items
        .iter()
        .map(|i| {
            let s = text(key, i)?;
            if let Ok(u) = Uid::parse(s) {
                return Ok(u);
            }
            match Label::new(s) {
                Ok(l) => resolve(&l),
                Err(_) => Err(sm03(format!(
                    "{key} entry {s} is neither a uid nor a label"
                ))),
            }
        })
        .collect()
}

/// A source MAP as `SourceRef`. Its kind must be one SMYSL knows (SM05);
/// a `span` needs its `manifest`, and is built by `SourceRef::in_part`,
/// which refuses an empty or reversed one (SM03).
pub(crate) fn source_ref(v: &BundValue) -> Result<SourceRef, Refusal> {
    const KEYS: [&str; 7] = [
        "kind",
        "reference",
        "captured",
        "observed",
        "published",
        "span",
        "manifest",
    ];
    let m = v
        .as_map()
        .filter(|_| v.dt() == MAP)
        .ok_or_else(|| sm03(format!("source must be MAP, found {}", found(v))))?;
    for k in m.keys() {
        if !KEYS.contains(&k.as_str()) {
            return Err(sm03(format!("source has no key {k}")));
        }
    }
    let lacks = |k: &str| sm03(format!("source lacks {k}"));
    let kind_v = m.get("kind").ok_or_else(|| lacks("kind"))?;
    let kind = match kind_v.str_ref().filter(|_| kind_v.dt() == STRING) {
        Some(k) => SourceKind::parse(k).ok_or_else(|| sm05("source kind", k))?,
        // The INT a source of an unknown kind reads back with names a kind
        // this build cannot write (§S2).
        None => return Err(sm05("source kind", &kind_v.display())),
    };
    let reference = text(
        "reference",
        m.get("reference").ok_or_else(|| lacks("reference"))?,
    )?;
    let mut s = SourceRef::new(kind, reference);
    if let Some(v) = m.get("captured") {
        let d = text("captured", v)?;
        s = s.captured_on(
            Date::parse(d).map_err(|e| sm03(format!("captured {d} is refused: {e}")))?,
        );
    }
    if let Some(v) = m.get("observed") {
        s = s.observed_at(observed(v)?);
    }
    if let Some(v) = m.get("published") {
        let p = text("published", v)?;
        let when = smysl_core::edtf::parse(p)
            .map_err(|e| sm03(format!("published {p} is refused: {e}")))?;
        s = s.published_at(&when);
    }
    match (m.get("span"), m.get("manifest")) {
        (None, None) => Ok(s),
        (Some(_), None) => Err(sm03("source span needs its manifest")),
        (None, Some(_)) => Err(sm03("source manifest needs its span")),
        (Some(span), Some(mid)) => {
            let (a, b) = span_of(span)?;
            let mid_text = text("manifest", mid)?;
            let mid = Mid::parse(mid_text)
                .map_err(|_| sm03(format!("manifest {mid_text} is not a manifest id")))?;
            s.in_part(a, b, mid)
                .ok_or_else(|| sm03(format!("source span [ {a} {b} ] is empty or reversed")))
        }
    }
}

/// `observed`: an INT of milliseconds, or, above `i64::MAX`, the STRING of
/// its decimal digits it reads back as. Any other STRING is SM03 (§S2).
fn observed(v: &BundValue) -> Result<u64, Refusal> {
    if v.dt() == INTEGER {
        let i = v.as_int().unwrap_or(-1);
        return u64::try_from(i).map_err(|_| sm03(format!("observed {i} is negative")));
    }
    let s = text("observed", v)?;
    s.parse::<u64>()
        .ok()
        .filter(|n| i64::try_from(*n).is_err())
        .ok_or_else(|| {
            sm03(format!(
                "observed {s} is not a number of milliseconds above an INT's range"
            ))
        })
}

fn span_of(v: &BundValue) -> Result<(u64, u64), Refusal> {
    let bad = || sm03("source span must be a LIST of two INTs");
    let items = v.as_list().filter(|_| v.dt() == LIST).ok_or_else(bad)?;
    let [a, b] = items else {
        return Err(bad());
    };
    let n = |x: &BundValue| {
        x.as_int()
            .filter(|_| x.dt() == INTEGER)
            .and_then(|i| u64::try_from(i).ok())
            .ok_or_else(bad)
    };
    Ok((n(a)?, n(b)?))
}

/// `payload`: a MAP, encoded as SMYSL encodes an object. An empty MAP is no
/// payload, as SMYSL encodes none for an empty object, so that one core
/// has one encoding (`smysl-core/src/surface/payload.rs:30-36`).
fn payload_bytes(v: &BundValue) -> Result<Option<Vec<u8>>, Refusal> {
    if v.dt() != MAP {
        return Err(sm03(format!("payload must be MAP, found {}", found(v))));
    }
    let HValue::Object(o) = hvalue("payload", v)? else {
        return Err(sm03(format!("payload must be MAP, found {}", found(v))));
    };
    Ok(object_to_payload(&o))
}

fn spanned<T>(value: T) -> Spanned<T> {
    Spanned::new(value, Span::new(0, 0))
}

/// A Bund value as a payload tree, or SM03 naming its key.
fn hvalue(path: &str, v: &BundValue) -> Result<HValue, Refusal> {
    match v.dt() {
        NODATA => Ok(HValue::Null),
        BOOL => match v.unboxed() {
            BundValue::Bool(b, _) => Ok(HValue::Bool(*b)),
            _ => Err(no_form(path, v)),
        },
        INTEGER => v.as_int().map(HValue::Int).ok_or_else(|| no_form(path, v)),
        FLOAT => match v.unboxed() {
            BundValue::Float(f, _) => Ok(HValue::Float(*f)),
            _ => Err(no_form(path, v)),
        },
        STRING => v
            .str_ref()
            .map(|s| HValue::Str(s.to_owned()))
            .ok_or_else(|| no_form(path, v)),
        LIST => {
            let items = v.as_list().ok_or_else(|| no_form(path, v))?;
            items
                .iter()
                .enumerate()
                .map(|(i, x)| hvalue(&format!("{path}[{i}]"), x).map(spanned))
                .collect::<Result<Vec<_>, _>>()
                .map(HValue::Array)
        }
        MAP => {
            let m = v.as_map().ok_or_else(|| no_form(path, v))?;
            let mut o = HObject::new();
            for (k, x) in m {
                o.insert(
                    spanned(k.clone()),
                    spanned(hvalue(&format!("{path}.{k}"), x)?),
                );
            }
            Ok(HValue::Object(o))
        }
        _ => Err(no_form(path, v)),
    }
}

fn no_form(path: &str, v: &BundValue) -> Refusal {
    sm03(format!("{path} has no SMYSL form: {}", found(v)))
}

/// A relation MAP as the relation SMYSL builds from it. Its kind must be
/// one `RelKind::parse` accepts, kernel or extension (SM05, R24). Whether
/// its word takes a `retracts` relation is the word's to say (R23).
pub fn relation_of(m: &BTreeMap<String, BundValue>) -> Result<Relation, Refusal> {
    const KEYS: [&str; 5] = ["kind", "from", "to", "weight", "note"];
    only(Kind::Relation, m, &KEYS)?;
    let k = text("kind", required(Kind::Relation, m, "kind")?)?;
    let kind = RelKind::parse(k).map_err(|_| sm05("relation kind", k))?;
    let from = parse_uid("from", text("from", required(Kind::Relation, m, "from")?)?)?;
    let to = parse_uid("to", text("to", required(Kind::Relation, m, "to")?)?)?;
    let mut r = Relation::new(kind, from, to);
    if let Some(w) = m.get("weight") {
        let f = match w.unboxed() {
            BundValue::Float(f, _) => *f,
            _ if w.dt() == INTEGER => w.as_int().map_or(0.0, |i| i as f64),
            _ => {
                return Err(sm03(format!("weight must be FLOAT, found {}", found(w))));
            }
        };
        // SMYSL clamps to [0, 1] and quantises (§S2).
        r = r.with_weight(f as f32);
    }
    if let Some(n) = m.get("note") {
        r = r.with_note(parse_uid("note", text("note", n)?)?);
    }
    Ok(r)
}

/// A label MAP `{ label, uid }` as the binding SMYSL records. Whether the
/// uid names a unit is the library's to say, and the word's to check (§S2).
pub fn label_of(m: &BTreeMap<String, BundValue>) -> Result<LabelBinding, Refusal> {
    only(Kind::Label, m, &["label", "uid"])?;
    let l = text("label", required(Kind::Label, m, "label")?)?;
    let label = Label::new(l).map_err(|e| sm03(format!("label {l} is refused: {e}")))?;
    let u = parse_uid("uid", text("uid", required(Kind::Label, m, "uid")?)?)?;
    Ok(LabelBinding::new(label, u))
}

#[cfg(test)]
mod tests {
    use smysl_core::{KernelType, SourceKind};

    use super::super::Code;
    use super::*;

    fn s(v: &str) -> BundValue {
        BundValue::str(v)
    }

    fn map(entries: &[(&str, BundValue)]) -> BTreeMap<String, BundValue> {
        entries
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect()
    }

    fn no_labels(l: &Label) -> Result<Uid, Refusal> {
        Err(Refusal::new(
            Code::Sm08,
            format!("label {} binds 0 units", l.as_str()),
        ))
    }

    fn core(entries: &[(&str, BundValue)]) -> Result<UnitCore, Refusal> {
        unit_core(&map(entries), &mut no_labels)
    }

    fn base() -> Vec<(&'static str, BundValue)> {
        vec![
            ("schema", s("claim")),
            ("gist", s("The well is dry")),
            ("status", s("speculative")),
        ]
    }

    fn with(extra: &[(&'static str, BundValue)]) -> Vec<(&'static str, BundValue)> {
        let mut v = base();
        v.extend(extra.iter().cloned());
        v
    }

    fn refused(r: Result<UnitCore, Refusal>) -> (Code, String) {
        let r = r.unwrap_err();
        (r.code, r.text)
    }

    fn get(v: &BundValue, k: &str) -> BundValue {
        v.as_map().unwrap()[k].clone()
    }

    #[test]
    fn a_unit_reads_back_as_its_map() {
        let ground = UnitCoreBuilder::new(KernelType::Claim, "ground", Status::Speculative)
            .build()
            .unwrap();
        let g = canonical_uid(&ground);
        let mut obj = HObject::new();
        obj.insert(spanned("n".to_owned()), spanned(HValue::Int(3)));
        let c = UnitCoreBuilder::new(KernelType::Claim, "The well is dry", Status::Inferred)
            .body("It has not rained.")
            .grounds([g])
            .source(SourceRef::new(SourceKind::Doc, "Ruth 1:21").observed_at(5))
            .payload(object_to_payload(&obj).unwrap())
            .build()
            .unwrap();
        let v = unit(&c, &[Label::new("d/well").unwrap()]);
        assert_eq!(get(&v, "uid"), s(&canonical_uid(&c).canonical()));
        assert_eq!(get(&v, "schema"), s("claim"));
        assert_eq!(get(&v, "status"), s("inferred"));
        assert_eq!(get(&v, "body"), s("It has not rained."));
        assert_eq!(
            get(&v, "grounds").display(),
            BundValue::list(vec![s(&g.canonical())]).display()
        );
        assert_eq!(
            get(&v, "labels").display(),
            BundValue::list(vec![s("d/well")]).display()
        );
        assert_eq!(get(&get(&v, "payload"), "n"), BundValue::int(3));
        let src = get(&v, "source");
        assert_eq!(get(&src, "kind"), s("doc"));
        assert_eq!(get(&src, "observed"), BundValue::int(5));
        assert!(!v.has_key("opaque"));
        assert!(!v.has_key("detail"));
    }

    #[test]
    fn a_unit_map_writes_the_core_smysl_builds() {
        let ground = UnitCoreBuilder::new(KernelType::Claim, "ground", Status::Speculative)
            .build()
            .unwrap();
        let g = canonical_uid(&ground);
        let written = core(&with(&[
            ("status", s("inferred")),
            ("grounds", BundValue::list(vec![s(&g.canonical())])),
            ("payload", BundValue::map(map(&[("n", BundValue::int(3))]))),
        ]))
        .unwrap();
        let mut obj = HObject::new();
        obj.insert(spanned("n".to_owned()), spanned(HValue::Int(3)));
        let direct = UnitCoreBuilder::new(KernelType::Claim, "The well is dry", Status::Inferred)
            .grounds([g])
            .payload(object_to_payload(&obj).unwrap())
            .build()
            .unwrap();
        assert_eq!(canonical_uid(&written), canonical_uid(&direct));
    }

    /// 18: `payload: {}` gives the uid of the same MAP without `payload`,
    /// and reads back without the key.
    #[test]
    fn an_empty_payload_is_no_payload() {
        let empty = core(&with(&[("payload", BundValue::map(BTreeMap::new()))])).unwrap();
        let none = core(&base()).unwrap();
        assert_eq!(canonical_uid(&empty), canonical_uid(&none));
        assert!(!unit(&empty, &[]).has_key("payload"));
    }

    #[test]
    fn a_label_in_grounds_goes_to_the_resolver() {
        let ground = UnitCoreBuilder::new(KernelType::Claim, "ground", Status::Speculative)
            .build()
            .unwrap();
        let g = canonical_uid(&ground);
        let mut resolve = |l: &Label| {
            assert_eq!(l.as_str(), "d/ground");
            Ok(g)
        };
        let c = unit_core(
            &map(&with(&[
                ("status", s("inferred")),
                ("grounds", BundValue::list(vec![s("d/ground")])),
            ])),
            &mut resolve,
        )
        .unwrap();
        assert!(c.grounds.contains(&g));
        let (code, text) = refused(core(&with(&[(
            "grounds",
            BundValue::list(vec![s("d/nowhere")]),
        )])));
        assert_eq!(
            (code, text.as_str()),
            (Code::Sm08, "label d/nowhere binds 0 units")
        );
    }

    #[test]
    fn what_a_unit_map_may_not_hold_is_sm03() {
        let sm03 = |r| {
            let (code, text) = refused(r);
            assert_eq!(code, Code::Sm03, "{text}");
            text
        };
        assert_eq!(
            sm03(core(&with(&[("uid", s("b3:x"))]))),
            "uid is computed, not given"
        );
        assert_eq!(
            sm03(core(&with(&[("opaque", BundValue::boolean(true))]))),
            "opaque is computed, not given"
        );
        assert_eq!(
            sm03(core(&with(&[("labels", BundValue::list(vec![]))]))),
            "labels is read back only: a label is bound with smysl.label or a label MAP"
        );
        assert_eq!(
            sm03(core(&with(&[("colour", s("red"))]))),
            "unit has no key colour"
        );
        assert_eq!(
            sm03(core(&with(&[(
                "grounds",
                BundValue::list(vec![s("Not a ref")])
            )]))),
            "grounds entry Not a ref is neither a uid nor a label"
        );
        // 14: a non-MAP payload.
        assert_eq!(
            sm03(core(&with(&[("payload", BundValue::int(1))]))),
            "payload must be MAP, found Integer"
        );
        assert_eq!(
            sm03(core(&with(&[(
                "payload",
                BundValue::map(map(&[("f", BundValue::call("x"))]))
            )]))),
            "payload.f has no SMYSL form: Call"
        );
        assert_eq!(
            sm03(core(&with(&[("status", s("unfounded"))]))),
            "status unfounded cannot be authored"
        );
        assert_eq!(
            sm03(core(&with(&[("status", s("measured"))]))),
            "status measured cannot be authored"
        );
        assert_eq!(sm03(core(&[("gist", s("g"))])), "unit lacks schema");
        assert_eq!(
            sm03(core(&with(&[("gist", BundValue::int(1))]))),
            "gist must be STRING, found Integer"
        );
    }

    #[test]
    fn what_smysl_refuses_is_sm02_and_unknown_names_are_sm05() {
        // 14: a gist of spaces, refused as empty after `trim`
        // (`smysl-core/src/types/unit.rs:146-149`).
        let (code, text) = refused(core(&with(&[("gist", s("   "))])));
        assert_eq!(code, Code::Sm02);
        assert!(text.starts_with("SMYSL refused it: "), "{text}");
        // 14: `body: ""` with a `detail` is "detail without a body".
        let (code, _) = refused(core(&with(&[("body", s("")), ("detail", s("d"))])));
        assert_eq!(code, Code::Sm02);
        assert_eq!(
            refused(core(&with(&[("status", s("certain"))]))),
            (Code::Sm05, "unknown status: certain".to_owned())
        );
        let src = |m: BTreeMap<String, BundValue>| core(&with(&[("source", BundValue::map(m))]));
        assert_eq!(
            refused(src(map(&[("kind", s("pigeon")), ("reference", s("r"))]))),
            (Code::Sm05, "unknown source kind: pigeon".to_owned())
        );
        assert_eq!(
            refused(src(map(&[
                ("kind", BundValue::int(77)),
                ("reference", s("r"))
            ]))),
            (Code::Sm05, "unknown source kind: 77".to_owned())
        );
    }

    /// 14: an empty span, and a reversed one, are SM03.
    #[test]
    fn a_span_needs_its_manifest_and_must_not_be_empty() {
        let mid = Mid::from_bytes([7; 32]).canonical();
        let src = |span: Vec<i64>, with_mid: bool| {
            let mut m = map(&[
                ("kind", s("doc")),
                ("reference", s("r")),
                (
                    "span",
                    BundValue::list(span.into_iter().map(BundValue::int).collect()),
                ),
            ]);
            if with_mid {
                m.insert("manifest".to_owned(), s(&mid));
            }
            core(&with(&[("source", BundValue::map(m))]))
        };
        assert_eq!(
            refused(src(vec![4, 4], true)),
            (
                Code::Sm03,
                "source span [ 4 4 ] is empty or reversed".to_owned()
            )
        );
        assert_eq!(
            refused(src(vec![5, 4], true)),
            (
                Code::Sm03,
                "source span [ 5 4 ] is empty or reversed".to_owned()
            )
        );
        assert_eq!(
            refused(src(vec![1, 4], false)),
            (Code::Sm03, "source span needs its manifest".to_owned())
        );
        let ok = src(vec![1, 4], true).unwrap();
        let back = get(&get(&unit(&ok, &[]), "source"), "span");
        assert_eq!(
            back.display(),
            BundValue::list(vec![BundValue::int(1), BundValue::int(4)]).display()
        );
    }

    /// 18: `observed` above `i64::MAX`, written from its STRING form, reads
    /// back as that STRING.
    #[test]
    fn observed_above_an_ints_range_round_trips_as_a_string() {
        let big = (i64::MAX as u64 + 5).to_string();
        let src = |o: BundValue| {
            core(&with(&[(
                "source",
                BundValue::map(map(&[
                    ("kind", s("metric")),
                    ("reference", s("r")),
                    ("observed", o),
                ])),
            )]))
        };
        let c = src(s(&big)).unwrap();
        assert_eq!(
            c.source.as_ref().unwrap().observed,
            Some(i64::MAX as u64 + 5)
        );
        assert_eq!(get(&get(&unit(&c, &[]), "source"), "observed"), s(&big));
        assert_eq!(refused(src(s("12"))).0, Code::Sm03);
        assert_eq!(refused(src(BundValue::int(-1))).0, Code::Sm03);
    }

    /// 18: a source of a kind this build cannot name reads back with
    /// `kind` as that INT and the unit `opaque`.
    #[test]
    fn an_unknown_source_kind_reads_back_as_its_code() {
        let src = SourceRef::new(SourceKind::Doc, "r")
            .with_unknown_kind(77)
            .unwrap();
        let c = UnitCoreBuilder::new(KernelType::Claim, "g", Status::Cited)
            .source(src)
            .build()
            .unwrap();
        let v = unit(&c, &[]);
        assert_eq!(get(&get(&v, "source"), "kind"), BundValue::int(77));
        assert_eq!(get(&v, "opaque"), BundValue::boolean(true));
    }

    #[test]
    fn a_payload_that_does_not_decode_is_left_out_and_said() {
        let c = UnitCoreBuilder::new(KernelType::Claim, "g", Status::Speculative)
            .payload(vec![0xff, 0x00])
            .build()
            .unwrap();
        let v = unit(&c, &[]);
        assert!(!v.has_key("payload"));
        assert_eq!(get(&v, "opaque"), BundValue::boolean(true));
    }

    /// 18: an extension relation kind reads back as its text, and a relation
    /// with a non-empty `extra` is `opaque`.
    #[test]
    fn relations_read_back_and_write() {
        let a = canonical_uid(
            &UnitCoreBuilder::new(KernelType::Claim, "a", Status::Speculative)
                .build()
                .unwrap(),
        );
        let b = canonical_uid(
            &UnitCoreBuilder::new(KernelType::Claim, "b", Status::Speculative)
                .build()
                .unwrap(),
        );
        let ext = Relation::new(RelKind::parse("x.text/mentions").unwrap(), a, b);
        let v = relation(&ext);
        assert_eq!(get(&v, "kind"), s("x.text/mentions"));
        assert_eq!(get(&v, "rid"), s(&ext.uid().canonical()));
        assert!(!v.has_key("opaque"));

        let mut odd = Relation::new(RelKind::Rebuts, a, b);
        odd.extra.insert(99, vec![1]);
        assert_eq!(get(&relation(&odd), "opaque"), BundValue::boolean(true));

        let written = relation_of(&map(&[
            ("kind", s("rebuts")),
            ("from", s(&a.canonical())),
            ("to", s(&b.canonical())),
            ("weight", BundValue::float(0.5)),
        ]))
        .unwrap();
        assert_eq!(written.uid(), Relation::new(RelKind::Rebuts, a, b).uid());
        assert_eq!(written.weight, Some(0.5));
        let rel = |entries: &[(&str, BundValue)]| relation_of(&map(entries)).unwrap_err();
        assert_eq!(
            rel(&[
                ("kind", s("hates")),
                ("from", s(&a.canonical())),
                ("to", s(&b.canonical()))
            ]),
            sm05("relation kind", "hates")
        );
        assert_eq!(
            rel(&[
                ("kind", s("rebuts")),
                ("from", s(&a.canonical())),
                ("to", s(&b.canonical())),
                ("rid", s("x")),
            ]),
            sm03("rid is computed, not given")
        );
    }

    #[test]
    fn labels_read_back_and_write() {
        let u = canonical_uid(
            &UnitCoreBuilder::new(KernelType::Claim, "a", Status::Speculative)
                .build()
                .unwrap(),
        );
        let b = LabelBinding::new(Label::new("d/a").unwrap(), u);
        let v = label(&b);
        assert_eq!(get(&v, "label"), s("d/a"));
        assert_eq!(get(&v, "uid"), s(&u.canonical()));
        assert!(!v.has_key("opaque"));
        assert_eq!(label_of(v.as_map().unwrap()).unwrap(), b);
        assert_eq!(
            label_of(&map(&[
                ("label", s("d/a")),
                ("uid", s(&u.canonical())),
                ("gist", s("g"))
            ]))
            .unwrap_err(),
            sm03("label has no key gist")
        );
    }

    /// §S2's order, and step 4 naming what is missing.
    #[test]
    fn a_maps_kind_is_decided_by_its_keys() {
        assert_eq!(
            kind(&map(&[("label", s("d/a")), ("gist", s("g"))])),
            Ok(Kind::Label)
        );
        assert_eq!(
            kind(&map(&[
                ("kind", s("x")),
                ("from", s("a")),
                ("to", s("b")),
                ("gist", s("g"))
            ])),
            Ok(Kind::Relation)
        );
        assert_eq!(kind(&map(&[("gist", s("g"))])), Ok(Kind::Unit));
        assert_eq!(
            kind(&map(&[("kind", s("x")), ("from", s("a"))])),
            Err(sm03("relation lacks to"))
        );
        assert_eq!(
            kind(&map(&[("schema", s("claim"))])),
            Err(sm03("unit lacks gist"))
        );
        assert_eq!(
            kind(&map(&[("colour", s("red"))])),
            Err(sm03("record is not a unit, relation or label"))
        );
    }

    #[test]
    fn diagnostics_read_back_with_their_subject() {
        use smysl_core::diag::Code as DCode;
        let u = canonical_uid(
            &UnitCoreBuilder::new(KernelType::Claim, "a", Status::Speculative)
                .build()
                .unwrap(),
        );
        let mut d = Diagnostic::new(DCode::E060).with_subject(Subject::Unit(u));
        d.message = "referenced but not present".to_owned();
        let v = diagnostic(&d).unwrap();
        assert_eq!(get(&v, "code"), s("SMY-E060"));
        assert_eq!(get(&v, "severity"), s("error"));
        assert_eq!(get(&v, "message"), s("referenced but not present"));
        let subject = get(&v, "subject");
        assert_eq!(get(&subject, "kind"), s("unit"));
        assert_eq!(get(&subject, "value"), s(&u.canonical()));

        let store = diagnostic(&Diagnostic::new(DCode::E060).with_subject(Subject::Store)).unwrap();
        assert_eq!(get(&get(&store, "subject"), "value"), BundValue::nodata());
        let span =
            diagnostic(&Diagnostic::new(DCode::E060).with_subject(Subject::Span(Span::new(2, 9))))
                .unwrap();
        assert_eq!(
            get(&get(&span, "subject"), "value").display(),
            BundValue::list(vec![BundValue::int(2), BundValue::int(9)]).display()
        );
    }
}
