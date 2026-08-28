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
