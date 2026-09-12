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

/// How deeply a value may nest and still be written to the wire — **F118**.
///
/// Bund2's own walk is a heap arena and has no limit, but `bincode`'s derived
/// `Serialize` and `Deserialize` recurse per level **inside the dependency**,
/// and a decode cannot be bounded at all: bincode builds the whole nested
/// `WireValue` before any code here runs. So the bound is on **writing**, and
/// a value too deep to read back is refused before it is stored. The owner
/// chose this over recording the limit, because the alternative is the worse
/// failure: `save.model` succeeding on a value `load.model` then aborts on.
///
/// **256, from the worst case rather than the best.** Decoding aborts at
/// (measured 2026-09-12, nested one-element lists):
///
/// | build | thread | aborts at |
/// |---|---|---|
/// | debug | 2 MiB — an embedder's default | **512** |
/// | debug | 8 MiB | 2,048 |
/// | release | 8 MiB — what `bund2` gives evaluation | 6,000 |
///
/// The deepest nesting anywhere in the corpus is 2, so 256 is 128× what any
/// real program has needed and stays clear of the shallowest abort. D39
/// chooses thresholds this way: far past any real program, far below where it
/// breaks.
pub const MAX_WIRE_DEPTH: usize = 256;

impl WireValue {
    /// A Bund2 value as the reference's `Value`, field for field.
    ///
    /// **Serialising materialises the identity and the stamp** (D20). A value
    /// that has not been asked for either is given both here, as the reference
    /// gave them at construction.
    /// **Built from an arena, not by recursion — F118.**
    ///
    /// Encoding descended per level, so `save.model` on a 3,400-deep value
    /// aborted the process at about 600 bytes of stack a level, which D37
    /// forbids. The walk now runs from a `Vec`: each value is given an index
    /// on the way down, its children are queued, and the `WireValue`s are
    /// assembled bottom-up once every child has one. A child always takes a
    /// higher index than its parent, so assembling in reverse index order
    /// means a parent finds its children already built.
    ///
    /// The bytes are unchanged: this builds the same tree in a different
    /// order, and `wire_fixtures/*.hex` pin it against the reference.
    pub fn from_value(v: &BundValue) -> WireValue {
        let mut nodes: Vec<Node> = Vec::new();
        let mut queue: Vec<BundValue> = vec![v.clone()];
        let mut at = 0usize;
        while at < queue.len() {
            let cur = queue[at].clone();
            at += 1;
            let node = node_of(&cur, &mut queue);
            nodes.push(node);
        }
        // Bottom-up: the last node has no children left to wait for.
        let mut built: Vec<Option<WireValue>> = (0..nodes.len()).map(|_| None).collect();
        for (i, node) in nodes.into_iter().enumerate().rev() {
            let take = |idx: usize, built: &mut Vec<Option<WireValue>>| {
                built[idx].take().unwrap_or_else(WireValue::placeholder)
            };
            let data = match node.shape {
                Shape::Leaf(val) => val,
                Shape::List(ids) => Val::List(ids.into_iter().map(|k| take(k, &mut built)).collect()),
                Shape::Lambda(ids) => {
                    Val::Lambda(ids.into_iter().map(|k| take(k, &mut built)).collect())
                }
                Shape::Map(entries) => Val::Map(
                    entries
                        .into_iter()
                        .map(|(k, idx)| (k, take(idx, &mut built)))
                        .collect(),
                ),
                Shape::ValueMap(pairs) => Val::ValueMap(
                    pairs
                        .into_iter()
                        .map(|(k, x)| (take(k, &mut built), take(x, &mut built)))
                        .collect(),
                ),
            };
            let attr = node
                .attr
                .into_iter()
                .map(|k| take(k, &mut built))
                .collect();
            built[i] = Some(WireValue {
                id: node.id,
                stamp: node.stamp,
                dt: node.dt,
                q: node.q,
                data,
                attr,
                curr: node.curr,
                tags: node.tags,
            });
        }
        built[0].take().unwrap_or_else(WireValue::placeholder)
    }

    /// A value that cannot occur: every index is filled before it is taken.
    /// Returning one is a wrong answer where aborting is not an answer at all
    /// (CLAUDE.md, D37).
    fn placeholder() -> WireValue {
        WireValue {
            id: String::new(),
            stamp: 0.0,
            dt: NONE,
            q: 0.0,
            data: Val::Null,
            attr: Vec::new(),
            curr: -1,
            tags: HashMap::new(),
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
    /// **Decoded from an arena, not by recursion — F118.**
    ///
    /// The mirror of [`WireValue::from_value`]: a `save` then `load` of a
    /// 12,000-deep value aborted on the way back while the save itself
    /// survived 20,000. The tree is walked iteratively, each node given an
    /// index and its children queued, and the `BundValue`s are built bottom-up
    /// so a parent finds its children already decoded. A child that has no
    /// Bund2 form fails the whole decode with its own message, as before.
    pub fn into_value(self) -> Result<BundValue, String> {
        let mut nodes: Vec<WireNode> = Vec::new();
        let mut queue: Vec<WireValue> = vec![self];
        let mut at = 0usize;
        while at < queue.len() {
            let cur = std::mem::replace(&mut queue[at], WireValue::placeholder());
            at += 1;
            nodes.push(wire_node_of(cur, &mut queue)?);
        }
        let mut built: Vec<Option<BundValue>> = (0..nodes.len()).map(|_| None).collect();
        for (i, node) in nodes.into_iter().enumerate().rev() {
            let take = |idx: usize, built: &mut Vec<Option<BundValue>>| {
                built[idx]
                    .take()
                    .unwrap_or_else(|| BundValue::None(StackSym::NONE))
            };
            let payload = match node.shape {
                WireShape::Leaf(p) => p,
                WireShape::List(ids) => {
                    Payload::List(ids.into_iter().map(|k| take(k, &mut built)).collect())
                }
                WireShape::Lambda(ids) => {
                    Payload::Lambda(ids.into_iter().map(|k| take(k, &mut built)).collect())
                }
                WireShape::Map(entries) => Payload::Map(
                    entries
                        .into_iter()
                        .map(|(k, idx)| (k, take(idx, &mut built)))
                        .collect(),
                ),
                WireShape::ValueMap(pairs) => Payload::ValueMap(
                    pairs
                        .into_iter()
                        .map(|(k, x)| (take(k, &mut built), take(x, &mut built)))
                        .collect(),
                ),
            };
            let attr = node.attr.into_iter().map(|k| take(k, &mut built)).collect();
            built[i] = Some(BundValue::Heap(Rc::new(HeapValue {
                identity: Cell::new(0),
                stamp: Cell::new(node.stamp),
                dt: node.dt,
                q: node.q,
                curr: node.curr,
                tags: node.tags,
                attr,
                payload: Rc::new(payload),
            })));
        }
        Ok(built[0]
            .take()
            .unwrap_or_else(|| BundValue::None(StackSym::NONE)))
    }

    /// The old recursive body, kept for one value's payload: everything that
    /// is not a container decodes without looking at a child.
    fn leaf_payload(dt: u16, data: Val) -> Result<Payload, String> {
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
            // The four container arms never reach here: `wire_node_of` turns
            // them into child indices before this is called.
            Val::List(_) | Val::Lambda(_) | Val::Map(_) | Val::ValueMap(_) => {
                return Err("a container reached the leaf decoder".to_string())
            }
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
        Ok(payload)
    }
}

/// One decoded value's header and the shape of its children — F118's arena,
/// the decode side.
struct WireNode {
    stamp: f64,
    dt: u16,
    q: f64,
    curr: i32,
    tags: super::Tags,
    shape: WireShape,
    attr: Vec<usize>,
}

enum WireShape {
    Leaf(Payload),
    List(Vec<usize>),
    Lambda(Vec<usize>),
    Map(Vec<(String, usize)>),
    ValueMap(Vec<(usize, usize)>),
}

/// Record one `WireValue`, queueing its children — F118.
fn wire_node_of(mut w: WireValue, queue: &mut Vec<WireValue>) -> Result<WireNode, String> {
    // `WireValue` owns a `Drop` since F118, so its fields are taken rather
    // than destructured.
    let (stamp, dt, q, curr) = (w.stamp, w.dt, w.q, w.curr);
    let data = std::mem::replace(&mut w.data, Val::Null);
    let attr = std::mem::take(&mut w.attr);
    let tags = std::mem::take(&mut w.tags);
    let push = |child: WireValue, queue: &mut Vec<WireValue>| {
        queue.push(child);
        queue.len() - 1
    };
    let shape = match data {
        Val::List(items) => {
            WireShape::List(items.into_iter().map(|x| push(x, queue)).collect())
        }
        Val::Lambda(items) => {
            WireShape::Lambda(items.into_iter().map(|x| push(x, queue)).collect())
        }
        Val::Map(m) => WireShape::Map(
            m.into_iter()
                .map(|(k, x)| (k, push(x, queue)))
                .collect(),
        ),
        Val::ValueMap(pairs) => WireShape::ValueMap(
            pairs
                .into_iter()
                .map(|(k, x)| (push(k, queue), push(x, queue)))
                .collect(),
        ),
        other => WireShape::Leaf(WireValue::leaf_payload(dt, other)?),
    };
    Ok(WireNode {
        stamp,
        dt,
        q,
        curr,
        tags: tags
            .into_iter()
            .map(|(k, x)| (Rc::from(k.as_str()), Rc::from(x.as_str())))
            .collect(),
        shape,
        attr: attr.into_iter().map(|a| push(a, queue)).collect(),
    })
}

fn no_form(kind: &str) -> String {
    format!("the value holds a {kind}, which Bund2 has no form for")
}

// `decode_all` went with the recursive decoder: the arena in
// `WireValue::into_value` queues children by index instead (F118).

/// **`Drop` walks the levels on the heap — F118.**
///
/// `WireValue` holds `WireValue`s, in `attr` and inside five `Val` variants,
/// so the derived drop recursed on a decoded value's depth exactly as
/// `HeapValue`'s did before F115. Same fix: take each level's children onto a
/// worklist and free them level by level, each emptied before it is dropped,
/// so the drop the loop triggers finds nothing to descend into.
impl Drop for WireValue {
    fn drop(&mut self) {
        let mut work: Vec<WireValue> = Vec::new();
        take_wire_children(self, &mut work);
        while let Some(mut level) = work.pop() {
            take_wire_children(&mut level, &mut work);
        }
    }
}

/// Move one `WireValue`'s children onto `work`, leaving it empty.
fn take_wire_children(w: &mut WireValue, work: &mut Vec<WireValue>) {
    work.append(&mut w.attr);
    match &mut w.data {
        Val::List(items) | Val::Lambda(items) | Val::Queue(items) => work.append(items),
        Val::Matrix(rows) => {
            for row in std::mem::take(rows) {
                work.extend(row);
            }
        }
        Val::Map(m) => work.extend(std::mem::take(m).into_values()),
        Val::ValueMap(pairs) => {
            for (k, x) in std::mem::take(pairs) {
                work.push(k);
                work.push(x);
            }
        }
        Val::Null
        | Val::Exit
        | Val::Token(_)
        | Val::Error(_)
        | Val::Bool(_)
        | Val::I64(_)
        | Val::F64(_)
        | Val::String(_)
        | Val::Binary(_)
        | Val::Time(_)
        | Val::Metrics(_)
        | Val::Operator(_)
        | Val::Json(_)
        | Val::Embedding(_) => {}
    }
}

/// How deeply a value nests, counted on the heap — F118.
///
/// Stops as soon as the bound is exceeded: the answer only has to be large
/// enough to refuse, and a value can be arbitrarily deep.
fn depth_of(v: &BundValue) -> usize {
    let mut deepest = 0;
    let mut work: Vec<(BundValue, usize)> = vec![(v.clone(), 1)];
    while let Some((cur, d)) = work.pop() {
        deepest = deepest.max(d);
        if deepest > MAX_WIRE_DEPTH {
            return deepest;
        }
        for child in children_of(&cur) {
            work.push((child, d + 1));
        }
    }
    deepest
}

/// A value's members, whatever container holds them.
fn children_of(v: &BundValue) -> Vec<BundValue> {
    let mut out: Vec<BundValue> = v.attr().to_vec();
    if let BundValue::Heap(h) = v.unboxed() {
        match &*h.payload {
            Payload::List(items) | Payload::Lambda(items) => out.extend(items.iter().cloned()),
            Payload::Map(m) => out.extend(m.values().cloned()),
            Payload::ValueMap(m) => {
                for (k, x) in m {
                    out.push(k.clone());
                    out.push(x.clone());
                }
            }
            Payload::Str(_)
            | Payload::Bin(_)
            | Payload::Exit
            | Payload::Metrics(_)
            | Payload::Json(_)
            | Payload::Scalar(_) => {}
        }
    }
    out
}

/// One value's header and the shape of its children — F118's arena.
struct Node {
    id: String,
    stamp: f64,
    dt: u16,
    q: f64,
    curr: i32,
    tags: HashMap<String, String>,
    shape: Shape,
    attr: Vec<usize>,
}

/// What a node's `data` is: either finished, or the indices of the children
/// it is assembled from.
enum Shape {
    Leaf(Val),
    List(Vec<usize>),
    Lambda(Vec<usize>),
    Map(Vec<(String, usize)>),
    ValueMap(Vec<(usize, usize)>),
}

/// Record one value, queueing its children — F118.
///
/// Every child pushed onto `queue` is given the index it will occupy, which is
/// where it lands in the queue, because the queue is walked in order and one
/// node is produced per entry.
fn node_of(v: &BundValue, queue: &mut Vec<BundValue>) -> Node {
    let (id, _) = v.id_string();
    let (stamp, _) = v.timestamp();
    let push = |child: &BundValue, queue: &mut Vec<BundValue>| {
        queue.push(child.clone());
        queue.len() - 1
    };
    // A boxed scalar is the value it boxes, as far as the wire is concerned.
    let inner = v.unboxed();
    let shape = match inner {
        BundValue::Int(i, _) => Shape::Leaf(Val::I64(*i)),
        BundValue::Float(f, _) => Shape::Leaf(Val::F64(*f)),
        BundValue::Bool(b, _) => Shape::Leaf(Val::Bool(*b)),
        BundValue::Nodata(_) | BundValue::None(_) => Shape::Leaf(Val::Null),
        BundValue::Heap(h) => match &*h.payload {
            Payload::Str(s) => Shape::Leaf(Val::String(s.clone())),
            Payload::Bin(b) => Shape::Leaf(Val::Binary(b.clone())),
            Payload::Exit => Shape::Leaf(Val::Exit),
            Payload::Json(j) => Shape::Leaf(Val::Json(j.clone())),
            Payload::Metrics(ms) => Shape::Leaf(Val::Metrics(
                ms.iter()
                    .map(|m| WireMetric {
                        stamp: m.stamp,
                        data: m.data,
                    })
                    .collect(),
            )),
            Payload::List(items) => {
                Shape::List(items.iter().map(|x| push(x, queue)).collect())
            }
            Payload::Lambda(items) => {
                Shape::Lambda(items.iter().map(|x| push(x, queue)).collect())
            }
            Payload::Map(m) => Shape::Map(
                m.iter()
                    .map(|(k, x)| (k.clone(), push(x, queue)))
                    .collect(),
            ),
            Payload::ValueMap(m) => Shape::ValueMap(
                m.iter()
                    .map(|(k, x)| (push(k, queue), push(x, queue)))
                    .collect(),
            ),
            // `unboxed` has already followed every `Scalar`, so a value that
            // is still one carries a container, which cannot happen.
            Payload::Scalar(_) => Shape::Leaf(Val::Null),
        },
    };
    Node {
        id,
        stamp,
        dt: v.dt(),
        q: v.q(),
        curr: v.curr(),
        tags: v
            .tags()
            .iter()
            .map(|(k, x)| (k.to_string(), x.to_string()))
            .collect(),
        shape,
        attr: v.attr().iter().map(|a| push(a, queue)).collect(),
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
    // F118: refuse a value too deep for the codec to read back, before it is
    // written. The check is its own walk, on the heap, so measuring the depth
    // cannot itself overflow.
    let depth = depth_of(v);
    if depth > MAX_WIRE_DEPTH {
        return Err(format!(
            "the value nests {depth} deep, and {MAX_WIRE_DEPTH} is the most the wire format \
             can carry"
        ));
    }
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
    let (mut w, _): (WireValue, usize) =
        bincode::serde::decode_from_slice(bytes, bincode::config::legacy())
            .map_err(|e| e.to_string())?;
    if w.dt == JSON_WRAPPED {
        // Taken, not moved out: `WireValue` has a `Drop` since F118.
        let Val::String(text) = std::mem::replace(&mut w.data, Val::Null) else {
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

    /// **F118's bound.** A value past `MAX_WIRE_DEPTH` is refused where it
    /// would be written, because a decode cannot be bounded — bincode builds
    /// the nested tree before any code here runs. Refusing to write is the
    /// better failure: the alternative is a `save.model` that succeeds on a
    /// value `load.model` then aborts on.
    #[test]
    fn a_value_too_deep_for_the_wire_is_refused_not_written() {
        let mut v = BundValue::list(vec![BundValue::int(1)]);
        for _ in 0..MAX_WIRE_DEPTH {
            v = BundValue::list(vec![v]);
        }
        let e = to_binary(&v).expect_err("past the bound");
        assert!(e.contains(&MAX_WIRE_DEPTH.to_string()), "{e}");
        assert!(e.contains("nests"), "{e}");

        // At the bound it is written, and comes back.
        let mut ok = BundValue::int(7);
        for _ in 0..MAX_WIRE_DEPTH - 1 {
            ok = BundValue::list(vec![ok]);
        }
        let bytes = to_binary(&ok).expect("at the bound");
        let back = from_binary(&bytes).expect("decodes");
        let mut levels = 0;
        let mut cur = back;
        while let Some(items) = cur.as_list().map(<[BundValue]>::to_vec) {
            if items.is_empty() {
                break;
            }
            levels += 1;
            cur = items[0].clone();
        }
        assert_eq!(levels, MAX_WIRE_DEPTH - 1, "every level came back");
        assert_eq!(cur.as_int(), Some(7));
    }

    /// **F118's arena, on the smallest stack that matters.**
    ///
    /// Encoding and decoding walked a value's depth in Rust frames, so
    /// `save.model` aborted at 3,400 levels on the release binary; both build
    /// from a heap arena now. The depth a *dependency* can still take is what
    /// `MAX_WIRE_DEPTH` bounds, and the test above covers that. What this one
    /// covers is that a value at the bound survives a full round trip on a
    /// **2 MiB** thread — an embedder's default, and the worst case the bound
    /// was chosen against.
    #[test]
    fn a_value_at_the_bound_survives_the_wire_on_a_small_thread() {
        let done = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                let mut v = BundValue::int(7);
                for _ in 0..MAX_WIRE_DEPTH - 1 {
                    v = BundValue::list(vec![v]);
                }
                let bytes = to_binary(&v).expect("encodes at the bound");
                assert!(bytes.len() > MAX_WIRE_DEPTH, "every level was written");
                let back = from_binary(&bytes).expect("decodes at the bound");
                let mut levels = 0;
                let mut cur = back;
                while let Some(items) = cur.as_list().map(<[BundValue]>::to_vec) {
                    if items.is_empty() {
                        break;
                    }
                    levels += 1;
                    cur = items[0].clone();
                }
                assert_eq!(levels, MAX_WIRE_DEPTH - 1, "every level came back");
                assert_eq!(cur.as_int(), Some(7));
            })
            .expect("spawns")
            .join();
        assert!(done.is_ok(), "the round trip aborted at the bound");
    }

    fn without_ids(mut w: WireValue) -> WireValue {
        w.id = String::new();
        // Taken, not moved out: `WireValue` has a `Drop` since F118.
        let attr = std::mem::take(&mut w.attr);
        w.attr = attr.into_iter().map(without_ids).collect();
        let data = std::mem::replace(&mut w.data, Val::Null);
        w.data = match data {
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
