//! The bincode wire representation — RFC-0001's criterion D4, and D20.
//!
//! D20 requires the wire format be **byte-identical** to the reference's, and
//! names serialisation a materialisation point: the lazy identity and stamp
//! are concrete by the time they are written.
//!
//! So the in-memory type and the wire type are different types, and this
//! module is the conversion. That separation is not a nicety — it is what
//! lets `BundValue` be 16 bytes with a lazy identity while the bytes on the
//! wire are exactly what `rust_dynamic::Value` produces.
//!
//! # Every variant must be here, including the ones with no writer
//!
//! bincode encodes an enum variant as its **index**. So the order of [`Val`]
//! is load-bearing, and a variant omitted from the middle shifts the
//! discriminant of every variant after it.
//!
//! F36 and F38 rule that `Val::Token` and four `dt` constants have no writer
//! and should be omitted. **That ruling is about the in-memory
//! representation only.** `Token` sits at index 2 and `Error` at index 3
//! (`reference/rust_dynamic/src/types.rs:69,70`); dropping either would
//! renumber `Bool`, `I64`, `F64` and the fifteen after them, and every value
//! Bund2 wrote would be unreadable by the reference. They are carried here as
//! uninhabitable placeholders.
//!
//! This is a constraint no review of RFC-0001 surfaced and that only shows up
//! when the format is written down.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The reference's `Value`, field for field and in order.
/// `reference/rust_dynamic/src/value.rs:16-25`.
///
/// Field order matters for the same reason variant order does: bincode writes
/// struct fields in declaration order with no names.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WireValue {
    pub id: String,
    pub stamp: f64,
    pub dt: u16,
    pub q: f64,
    pub data: Val,
    pub attr: Vec<WireValue>,
    pub curr: i32,
    pub tags: HashMap<String, String>,
}

/// The reference's `Val`, **in its declaration order**.
/// `reference/rust_dynamic/src/types.rs:66-87`.
///
/// Twenty variants. `Token` and `Error` have no constructor anywhere (F38)
/// and are carried anyway, because their indices hold the numbering for
/// everything below them.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Val {
    Null,
    Exit,
    /// No writer (F38). Present to hold index 2.
    Token(String),
    /// No writer reachable from Bund. Present to hold index 3.
    Error(WireError),
    Bool(bool),
    I64(i64),
    F64(f64),
    List(Vec<WireValue>),
    Matrix(Vec<Vec<WireValue>>),
    Lambda(Vec<WireValue>),
    Queue(Vec<WireValue>),
    Map(HashMap<String, WireValue>),
    ValueMap(Vec<(WireValue, WireValue)>),
    String(String),
    Binary(Vec<u8>),
    Time(u128),
    Metrics(Vec<WireMetric>),
    Operator(WireOperator),
    Json(serde_json::Value),
    Embedding(Vec<f32>),
}

/// `reference/rust_dynamic/src/error.rs`, as the wire sees it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WireError {
    pub code: i32,
    pub message: String,
}

/// `reference/rust_dynamic/src/metric.rs`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WireMetric {
    pub stamp: u128,
    pub data: f64,
}

/// `reference/rust_dynamic/src/types.rs:59-63`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WireOperator {
    pub opcode: i32,
    pub opvalue1: Vec<u8>,
    pub opvalue2: Vec<u8>,
}

// --- the conversion ----------------------------------------------------------

use std::cell::Cell;
use std::rc::Rc;

use super::{
    format_id, mint, now_ms, BundValue, HeapValue, Metric, Payload, StackSym, JSON, NODATA, NONE,
};

/// The tag the reference gives a JSON value it has wrapped as a string for
/// the wire (`reference/rust_dynamic/src/types.rs:40`). No other value carries
/// it.
pub const JSON_WRAPPED: u16 = 25;

impl WireValue {
    /// A Bund2 value as the reference's `Value`, field for field.
    ///
    /// **Serialising materialises the identity and the stamp** (D20). A value
    /// that has not been asked for either is given both here, as the reference
    /// gave them at construction.
    pub fn from_value(v: &BundValue) -> WireValue {
        let (id, _) = v.id_string();
        let (stamp, _) = v.timestamp();
        WireValue {
            id,
            stamp,
            dt: v.dt(),
            q: v.q(),
            data: val_of(v),
            attr: v.attr().iter().map(WireValue::from_value).collect(),
            curr: v.curr(),
            tags: v
                .tags()
                .iter()
                .map(|(k, x)| (k.to_string(), x.to_string()))
                .collect(),
        }
    }

    /// The reference's `Value` as a Bund2 value.
    ///
    /// Every field survives except the **id**. Bund2's identity is a counter
    /// that renders as a nanoid (D1, D5), and a reference id is a random
    /// nanoid, which a counter cannot hold. So a decoded value mints a fresh
    /// identity when one is needed. No golden can see the difference, since
    /// every golden is normalised for ids (F14). The stamp, `q`, `curr`, the
    /// tags and `attr` all carry over, and every value comes back boxed so that
    /// it has somewhere to keep them.
    ///
    /// Seven of the reference's twenty `Val`s have no Bund2 form (`Token`,
    /// `Error`, `Matrix`, `Queue`, `Time`, `Operator`, `Embedding`). A value
    /// holding one is refused with its name, not coerced into something the
    /// reference did not write.
    pub fn into_value(self) -> Result<BundValue, String> {
        let WireValue {
            id: _,
            stamp,
            dt,
            q,
            data,
            attr,
            curr,
            tags,
        } = self;
        let payload = match data {
            Val::Null => match dt {
                NODATA => Payload::Scalar(BundValue::Nodata(StackSym::NONE)),
                NONE => Payload::Scalar(BundValue::None(StackSym::NONE)),
                other => return Err(format!("a NULL value tagged {other} has no Bund2 form")),
            },
            Val::Bool(b) => Payload::Scalar(BundValue::Bool(b, StackSym::NONE)),
            Val::I64(i) => Payload::Scalar(BundValue::Int(i, StackSym::NONE)),
            Val::F64(f) => Payload::Scalar(BundValue::Float(f, StackSym::NONE)),
            Val::Exit => Payload::Exit,
            Val::String(s) => Payload::Str(s),
            Val::Binary(b) => Payload::Bin(b),
            Val::List(items) => Payload::List(decode_all(items)?),
            Val::Lambda(items) => Payload::Lambda(decode_all(items)?),
            Val::Map(m) => Payload::Map(
                m.into_iter()
                    .map(|(k, x)| x.into_value().map(|v| (k, v)))
                    .collect::<Result<_, _>>()?,
            ),
            Val::ValueMap(pairs) => Payload::ValueMap(
                pairs
                    .into_iter()
                    .map(|(k, x)| Ok((k.into_value()?, x.into_value()?)))
                    .collect::<Result<_, String>>()?,
            ),
            Val::Metrics(ms) => Payload::Metrics(
                ms.into_iter()
                    .map(|m| Metric {
                        stamp: m.stamp,
                        data: m.data,
                    })
                    .collect(),
            ),
            Val::Json(j) => Payload::Json(j),
            Val::Token(_) => return Err(no_form("Token")),
            Val::Error(_) => return Err(no_form("Error")),
            Val::Matrix(_) => return Err(no_form("Matrix")),
            Val::Queue(_) => return Err(no_form("Queue")),
            Val::Time(_) => return Err(no_form("Time")),
            Val::Operator(_) => return Err(no_form("Operator")),
            Val::Embedding(_) => return Err(no_form("Embedding")),
        };
        Ok(BundValue::Heap(Rc::new(HeapValue {
            identity: Cell::new(0),
            stamp: Cell::new(stamp),
            dt,
            q,
            curr,
            tags: tags
                .into_iter()
                .map(|(k, x)| (Rc::from(k.as_str()), Rc::from(x.as_str())))
                .collect(),
            attr: decode_all(attr)?,
            payload: Rc::new(payload),
        })))
    }
}

fn no_form(kind: &str) -> String {
    format!("the value holds a {kind}, which Bund2 has no form for")
}

fn decode_all(items: Vec<WireValue>) -> Result<Vec<BundValue>, String> {
    items.into_iter().map(WireValue::into_value).collect()
}

/// The `Val` a Bund2 value's payload is on the wire.
fn val_of(v: &BundValue) -> Val {
    match v {
        BundValue::Int(i, _) => Val::I64(*i),
        BundValue::Float(f, _) => Val::F64(*f),
        BundValue::Bool(b, _) => Val::Bool(*b),
        BundValue::Nodata(_) | BundValue::None(_) => Val::Null,
        BundValue::Heap(h) => match &*h.payload {
            Payload::Str(s) => Val::String(s.clone()),
            Payload::Bin(b) => Val::Binary(b.clone()),
            Payload::List(items) => Val::List(items.iter().map(WireValue::from_value).collect()),
            Payload::Lambda(items) => {
                Val::Lambda(items.iter().map(WireValue::from_value).collect())
            }
            Payload::Map(m) => Val::Map(
                m.iter()
                    .map(|(k, x)| (k.clone(), WireValue::from_value(x)))
                    .collect(),
            ),
            Payload::ValueMap(m) => Val::ValueMap(
                m.iter()
                    .map(|(k, x)| (WireValue::from_value(k), WireValue::from_value(x)))
                    .collect(),
            ),
            Payload::Exit => Val::Exit,
            Payload::Metrics(ms) => Val::Metrics(
                ms.iter()
                    .map(|m| WireMetric {
                        stamp: m.stamp,
                        data: m.data,
                    })
                    .collect(),
            ),
            Payload::Json(j) => Val::Json(j.clone()),
            Payload::Scalar(inner) => val_of(inner),
        },
    }
}

/// `Value::to_binary` (`reference/rust_dynamic/src/bincode.rs:8-37`).
///
/// A JSON value is not serialised as JSON. bincode cannot read a
/// `serde_json::Value` back, so the reference first converts the value to its
/// text and wraps that as a `JSON_WRAPPED` string, with a fresh id and stamp
/// (`reference/rust_dynamic/src/create_special.rs:241-252`). Only the top
/// level is wrapped; a JSON value nested in a list is written as JSON, which
/// then cannot be read back, in the reference and here alike.
pub fn to_binary(v: &BundValue) -> Result<Vec<u8>, String> {
    let w = if v.dt() == JSON {
        WireValue {
            id: format_id(mint()),
            stamp: now_ms(),
            dt: JSON_WRAPPED,
            q: 100.0,
            data: Val::String(v.display()),
            attr: Vec::new(),
            curr: -1,
            tags: HashMap::new(),
        }
    } else {
        WireValue::from_value(v)
    };
    bincode::serde::encode_to_vec(&w, bincode::config::legacy())
        .map_err(|e| format!("bincode2::serialize() returns {e}"))
}

/// The reference's `from_binary`, its decoder
/// (`reference/rust_dynamic/src/bincode.rs:51-77`).
///
/// A `JSON_WRAPPED` value is parsed back into JSON. Anything else is converted
/// as it was written. A failure is the decoder's own message, as the
/// reference's is. Its text is bincode 2's rather than `bincode2`'s, the one
/// part of the round trip that is not the reference's.
pub fn from_binary(bytes: &[u8]) -> Result<BundValue, String> {
    let (w, _): (WireValue, usize) =
        bincode::serde::decode_from_slice(bytes, bincode::config::legacy())
            .map_err(|e| e.to_string())?;
    if w.dt == JSON_WRAPPED {
        let Val::String(text) = w.data else {
            return Err("This Dynamic type is not string".to_string());
        };
        let j: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        return Ok(BundValue::json(j));
    }
    w.into_value()
}

#[cfg(test)]
mod conversion_tests {
    use super::*;
    use crate::{CALL, FLOAT, INTEGER, LAMBDA, LIST, MAP, STRING};

    fn fixture(hex: &str) -> Vec<u8> {
        let h = hex.trim().as_bytes();
        h.chunks(2)
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).expect("ascii"), 16).expect("hex"))
            .collect()
    }

    /// Seven values the **oracle** saved with `save.model`, taken from the
    /// SQLite world file it wrote (`sqlite3 tst.world "select hex(model) …"`).
    const FIXTURES: [&str; 7] = [
        include_str!("wire_fixtures/m1.hex"),
        include_str!("wire_fixtures/m2.hex"),
        include_str!("wire_fixtures/m3.hex"),
        include_str!("wire_fixtures/m4.hex"),
        include_str!("wire_fixtures/m5.hex"),
        include_str!("wire_fixtures/m6.hex"),
        include_str!("wire_fixtures/m7.hex"),
    ];

    #[test]
    fn decodes_what_the_oracle_saved() {
        let v: Vec<BundValue> = FIXTURES
            .iter()
            .map(|h| from_binary(&fixture(h)).expect("decodes"))
            .collect();
        // `[ 1 2.5 "x" true ]`. Inside a list literal `true` is not evaluated,
        // so the oracle stored it as the word call it is.
        assert_eq!(v[0].dt(), LIST);
        let kinds: Vec<u16> = v[0].as_list().expect("list").iter().map(BundValue::dt).collect();
        assert_eq!(kinds, vec![INTEGER, FLOAT, STRING, CALL]);
        // `dict "a" 1 set`
        assert_eq!(v[1].dt(), MAP);
        assert_eq!(v[1].get("a").and_then(|x| x.as_int()), Some(1));
        // `{ 1 2 + }`
        assert_eq!(v[2].dt(), LAMBDA);
        // `'{"k": [1, 2]}' json`, written as JSON_WRAPPED and read back as JSON
        assert_eq!(v[3].dt(), JSON);
        assert_eq!(v[3].display(), r#"{"k":[1,2]}"#);
        assert_eq!(v[4].as_str().as_deref(), Some("hello"));
        assert_eq!(v[5].as_int(), Some(42));
        assert_eq!(v[6].dt(), NODATA);
    }

    fn without_ids(mut w: WireValue) -> WireValue {
        w.id = String::new();
        w.attr = w.attr.into_iter().map(without_ids).collect();
        w.data = match w.data {
            Val::List(v) => Val::List(v.into_iter().map(without_ids).collect()),
            Val::Lambda(v) => Val::Lambda(v.into_iter().map(without_ids).collect()),
            Val::Map(m) => Val::Map(m.into_iter().map(|(k, x)| (k, without_ids(x))).collect()),
            other => other,
        };
        w
    }

    /// Decode each oracle value and encode it again: every field comes back
    /// as the oracle wrote it except the ids, which a decoded value re-mints.
    #[test]
    fn a_round_trip_keeps_every_field_but_the_id() {
        for h in FIXTURES {
            let (w, _): (WireValue, usize) =
                bincode::serde::decode_from_slice(&fixture(h), bincode::config::legacy())
                    .expect("decodes");
            if w.dt == JSON_WRAPPED {
                continue;
            }
            let back = WireValue::from_value(&w.clone().into_value().expect("converts"));
            assert_eq!(without_ids(back), without_ids(w));
        }
    }

    #[test]
    fn a_bund2_value_survives_the_wire() {
        let v = BundValue::list(vec![
            BundValue::int(7),
            BundValue::str("s"),
            BundValue::map(Default::default()).set("k", BundValue::float(1.5)),
        ]);
        let back = from_binary(&to_binary(&v).expect("encodes")).expect("decodes");
        // A list compares by identity, and a decoded value mints a fresh one,
        // so the check is on what the value holds.
        assert_eq!(back.dt(), v.dt());
        assert_eq!(back.display(), v.display());
    }

    #[test]
    fn a_kind_bund2_lacks_is_refused_by_name() {
        let w = WireValue {
            id: String::new(),
            stamp: 0.0,
            dt: 13,
            q: 100.0,
            data: Val::Time(1),
            attr: Vec::new(),
            curr: -1,
            tags: HashMap::new(),
        };
        let e = w.into_value().expect_err("refused");
        assert!(e.contains("Time"), "{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg()
    -> bincode::config::Configuration<bincode::config::LittleEndian, bincode::config::Fixint> {
        bincode::config::legacy()
    }

    fn encode<T: Serialize>(v: &T) -> Vec<u8> {
        bincode::serde::encode_to_vec(v, cfg()).expect("encodes")
    }

    /// The discriminant bincode actually wrote, read back off the bytes.
    ///
    /// This reads the order **off the type** rather than restating it, so the
    /// test cannot drift from the declaration the way a comment would.
    fn discriminant(v: &Val) -> u32 {
        let bytes = encode(v);
        u32::from_le_bytes(bytes[0..4].try_into().expect("4 bytes"))
    }

    /// bincode encodes a variant as its index, so the declaration order of
    /// [`Val`] *is* the wire format. A variant inserted or removed above
    /// another renumbers it, and every value Bund2 wrote would be unreadable
    /// by the reference.
    ///
    /// This is why `Token` and `Error` are carried despite having no writer:
    /// F38 rules them out of the in-memory type, and they hold indices 2 and
    /// 3 here.
    #[test]
    fn variant_indices_are_the_references() {
        let cases: Vec<(u32, Val)> = vec![
            (0, Val::Null),
            (1, Val::Exit),
            (2, Val::Token(String::new())),
            (
                3,
                Val::Error(WireError {
                    code: 0,
                    message: String::new(),
                }),
            ),
            (4, Val::Bool(false)),
            (5, Val::I64(0)),
            (6, Val::F64(0.0)),
            (7, Val::List(vec![])),
            (8, Val::Matrix(vec![])),
            (9, Val::Lambda(vec![])),
            (10, Val::Queue(vec![])),
            (11, Val::Map(HashMap::new())),
            (12, Val::ValueMap(vec![])),
            (13, Val::String(String::new())),
            (14, Val::Binary(vec![])),
            (15, Val::Time(0)),
            (16, Val::Metrics(vec![])),
            (
                17,
                Val::Operator(WireOperator {
                    opcode: 0,
                    opvalue1: vec![],
                    opvalue2: vec![],
                }),
            ),
            (18, Val::Json(serde_json::Value::Null)),
            (19, Val::Embedding(vec![])),
        ];
        assert_eq!(cases.len(), 20, "the reference declares twenty variants");
        for (want, v) in &cases {
            assert_eq!(
                discriminant(v),
                *want,
                "{v:?} is at the wrong index — the wire format has moved"
            );
        }
    }

    /// Field order is as load-bearing as variant order: bincode writes no
    /// names, so a swap is silent.
    #[test]
    fn id_is_the_first_field() {
        let v = WireValue {
            id: "x".into(),
            stamp: 1.0,
            dt: 2,
            q: 100.0,
            data: Val::I64(7),
            attr: vec![],
            curr: -1,
            tags: HashMap::new(),
        };
        let bytes = encode(&v);
        assert_eq!(&bytes[0..8], &1u64.to_le_bytes(), "id is not first");
        assert_eq!(bytes[8], b'x');
    }

    #[test]
    fn a_value_round_trips() {
        let v = WireValue {
            id: "abc".into(),
            stamp: 2.5,
            dt: 9,
            q: 100.0,
            data: Val::List(vec![]),
            attr: vec![],
            curr: -1,
            tags: HashMap::from([("stack".to_string(), "main".to_string())]),
        };
        let bytes = encode(&v);
        let (back, _): (WireValue, usize) =
            bincode::serde::decode_from_slice(&bytes, cfg()).expect("decodes");
        assert_eq!(v, back);
    }
}
