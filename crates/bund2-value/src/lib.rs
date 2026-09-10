//! `BundValue`: the runtime value representation. See RFC-0001.
//!
//! This is the first implementation slice. It carries the shape, the identity
//! policy, and the equality/hash core — the parts RFC-0001's criteria D1, D2
//! and D8 test. Rendering (D3), the bincode wire format (D4), the `.id` string
//! format (D5) and the `valuemap` read path (D6) are not here yet.
//!
//! Three things in RFC-0001 are easy to get wrong and are asserted by tests
//! rather than left to prose:
//!
//! - **The value is two words.** 16 bytes, 8-aligned. A scalar never touches
//!   the heap until it acquires something a header holds.
//! - **Clone-equal, dup-unequal** (D13). `Clone` is an `Rc` bump and shares the
//!   identity slot; `dup` allocates a fresh header with a *cleared* slot and
//!   shares the payload. An earlier RFC draft made `dup` a bare `Rc` bump,
//!   which collapses the two.
//! - **Two equalities.** `PartialEq`/`Eq`/`Hash` here are the *key* equality:
//!   total, so `NaN` equals itself and `-0.0` equals `0.0`, because `HashMap`
//!   requires reflexivity. The language's `==` keeps IEEE semantics and is
//!   [`BundValue::eq_ieee`].

// **`clippy::mutable_key_type`, allowed with a reason rather than worked
// around.** `Payload::ValueMap` is `HashMap<BundValue, BundValue>` — Bund's
// VALUEMAP type — so the key type is fixed by the language, not chosen here.
//
// The lint fires because `BundValue` has interior mutability: the lazy
// identity `Cell` (D1) and the sampled stamp `Cell` (D2). Its hazard is a key
// whose hash changes while it sits in a map, and that cannot happen here:
//
// - `Hash` reads `identity()` on one arm only — a composite heap value — and
//   **minting is idempotent**: the first call caches into the `Cell` and every
//   later call returns the same `u64`. A key is hashed on insertion, so it is
//   minted from its first use onward and its hash never moves.
// - Scalars and strings hash by *content* and never consult the `Cell`, which
//   is D30's read path.
// - D41's inline `StackSym` is deliberately excluded from `Hash` and
//   `PartialEq`, so acquiring a stack tag does not move a key's hash either.
//
// Silenced at the crate root rather than per site because all four sites are
// the same type, and a per-site `allow` would repeat this argument four times
// or, more likely, not at all.
#![allow(clippy::mutable_key_type)]
#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

pub mod wire;

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

mod dt {
    //! The reference's type tags, verbatim.
    //! `reference/rust_dynamic/src/types.rs:15-56`.
    //!
    //! 38 of the 42 declared constants are live; `LITERAL`, `LARGE_FLOAT`,
    //! `ASSOCIATION` and `TOKEN` have no writer (F36) and are omitted.
    pub const NONE: u16 = 0;
    pub const BOOL: u16 = 1;
    pub const INTEGER: u16 = 2;
    pub const FLOAT: u16 = 3;
    pub const STRING: u16 = 4;
    pub const CALL: u16 = 6;
    pub const PTR: u16 = 7;
    pub const LIST: u16 = 9;
    pub const MAP: u16 = 11;
    pub const PAIR: u16 = 10;
    /// `TIME` and `CINTEGER` have no constructor in Bund2 yet. They are here
    /// because the comparison gate names them by tag: `stdlib_logic_compare`
    /// admits `INTEGER | FLOAT | CINTEGER | CFLOAT | TIME` as operand types
    /// (`reference/rust_multistackvm/src/stdlib/logic/logic_compare_fun.rs:18,20`),
    /// and that gate reads `type_of()`, which is the `dt` tag verbatim
    /// (`reference/rust_dynamic/src/value_types.rs:5-7`). Writing the gate
    /// with two of its five tags missing would be writing a different gate.
    pub const TIME: u16 = 13;
    pub const CINTEGER: u16 = 14;
    pub const CFLOAT: u16 = 15;
    pub const METRICS: u16 = 16;
    pub const LAMBDA: u16 = 17;
    /// A stack switch. `@name` carries the name; a `( … )` scratch context
    /// carries a generated one (`reference/rust_dynamic/src/create_special.rs:22-33`).
    pub const CONTEXT: u16 = 21;
    pub const TEXTBUFFER: u16 = 22;
    pub const JSON: u16 = 24;
    pub const CONDITIONAL: u16 = 29;
    pub const VALUEMAP: u16 = 30;
    pub const CLASS: u16 = 31;
    pub const OBJECT: u16 = 32;
    /// What the parser emits at end of input; the evaluator breaks on it
    /// (`reference/bundcore/src/bundcore_eval.rs:16-18`).
    pub const EXIT: u16 = 93;
    pub const NODATA: u16 = 97;
}

pub use dt::*;

/// Identity source.
///
/// D1 specifies "a counter plus a VM seed". The seed belongs to the VM, which
/// RFC-0002 defines and which does not exist yet, so this is a process-wide
/// counter for now. It starts at 1 because **zero means unminted** in the
/// identity slot.
fn mint() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// nanoid's alphabet, and its length. `.id` must return a 21-character string
/// over these (D5), because `register_method_id` hands the id straight to
/// `Value::from_string`
/// (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:16`).
const ALPHABET: &[u8; 64] = b"_-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const ID_LEN: usize = 21;

/// Format a minted counter as a nanoid-shaped string.
///
/// D1 chose "a counter plus a VM seed" precisely so the *format* survives
/// while the generation becomes lazy. The counter is spread over all 21
/// positions rather than left-padded, so consecutive ids do not share a
/// 20-character prefix — which would make the goldens' `<id>` normalisation
/// the only thing hiding a very obvious pattern.
fn format_id(n: u64) -> String {
    // Built from `char`s rather than bytes so there is no fallible UTF-8
    // conversion to explain. The alphabet is ASCII by construction and the
    // type system now says so.
    let mut out = String::with_capacity(ID_LEN);
    let mut x = n;
    for _ in 0..ID_LEN {
        out.push(ALPHABET[(x % 64) as usize] as char);
        x /= 64;
    }
    out
}

/// Cut a string to `width` characters, marking that it was cut.
fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_string();
    }
    let kept: String = s.chars().take(width.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// A short name for a `dt`, for summaries. Not the reference's `type_name`,
/// which is title-cased for display
/// (`reference/rust_dynamic/src/value_types.rs:8-14`).
fn tag_name(dt: u16) -> &'static str {
    match dt {
        MAP => "dict",
        CONDITIONAL => "conditional",
        CLASS => "class",
        OBJECT => "object",
        _ => "map",
    }
}

/// What a heap value points at, separately from its header.
///
/// Behind its own `Rc` so `dup` can reset the identity while sharing the
/// payload — identity and payload are shareable on different schedules, which
/// is the whole reason `dup` is not a bare `Rc` bump.
#[derive(Debug)]
pub enum Payload {
    Str(String),
    Bin(Vec<u8>),
    List(Vec<BundValue>),
    Map(BTreeMap<String, BundValue>),
    /// A `HashMap`, not a `BTreeMap`: D30 decided hash-by-content, and a
    /// `BTreeMap` needs a total `Ord` that F12's fix deletes.
    ValueMap(HashMap<BundValue, BundValue>),
    /// The end-of-input marker the parser emits for `EOI`
    /// (`reference/bund_language_parser/src/vm/eoi.rs:8`).
    Exit,
    Lambda(Vec<BundValue>),
    Metrics(Vec<Metric>),
    Json(serde_json::Value),
    /// A boxed scalar. Scalars are inline until they acquire a header, and
    /// `TS::push` tags unconditionally, so anything pushed to a stack is boxed.
    Scalar(BundValue),
}

/// `reference/rust_dynamic/src/metric.rs`, as the value sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Metric {
    pub stamp: u128,
    pub data: f64,
}

impl Metric {
    /// One sample, stamped now.
    ///
    /// **Nanoseconds, not the milliseconds a `Value` carries.** `Metric` uses
    /// `timestamp_ns` (`reference/rust_dynamic/src/metric.rs:13`) where a
    /// `Value`'s `stamp` uses `timestamp_ms`
    /// (`reference/rust_dynamic/src/value.rs:7-9`), and the two fields are both
    /// spelled `stamp` in the debug rendering — so the resolutions differ by a
    /// factor of a million in output that looks uniform.
    pub fn new(data: f64) -> Self {
        Self {
            stamp: now_ns(),
            data,
        }
    }
}

/// A value's tag map.
///
/// **`Rc<str>` on both sides, not `String`.** Every push writes one, and two
/// `String` allocations per push measured **36.5 ns** against **16.3 ns** for
/// two `Rc` clones (`crates/bund2-bench/benches/boxing.rs`) — on a path that
/// runs for every value the interpreter touches. `Rc<str>` orders by content,
/// so a `BTreeMap` keyed on it iterates in the same order a `String`-keyed one
/// did, which the 39 goldens carrying `tags: {"stack": …}` depend on.
pub type Tags = BTreeMap<Rc<str>, Rc<str>>;

/// The header a heap value carries.
#[derive(Debug)]
pub struct HeapValue {
    /// Lazy identity. Zero means unminted; minted on first *need*, where need
    /// is `.id`, equality, ordering, hashing or serialisation (D1).
    identity: Cell<u64>,
    /// Lazy stamp, sampled on first observation (D2). Zero means unsampled.
    stamp: Cell<f64>,
    /// The reference's `dt`. Independent of the payload: `Val::String` carries
    /// `STRING`, `PTR` and `CALL` among others, and they behave differently.
    dt: u16,
    /// A field, not a constant — but no word writes it. Arithmetic does
    /// **not** average it (D32 as amended, Q35): the reference's `+` never
    /// reaches `calc_q`, which has no caller at all. Constructors start at
    /// 100.0, which is why every golden shows it; `Value::none` starts at 0.0.
    q: f64,
    /// The iteration cursor. A plain field, not a `Cell`: a `Cell` inside the
    /// shared `Rc` would give clones a *shared* cursor.
    curr: i32,
    /// Written on every push (`reference/rust_multistack/src/ts_push.rs:25`).
    tags: Tags,
    attr: Vec<BundValue>,
    payload: Rc<Payload>,
}

impl HeapValue {
    fn new(dt: u16, payload: Payload) -> Self {
        Self {
            identity: Cell::new(0),
            stamp: Cell::new(0.0),
            dt,
            q: 100.0,
            curr: -1,
            tags: Tags::new(),
            attr: Vec::new(),
            payload: Rc::new(payload),
        }
    }

    /// The identity, minting it if this is the first need.
    fn identity(&self) -> u64 {
        let id = self.identity.get();
        if id != 0 {
            return id;
        }
        let fresh = mint();
        self.identity.set(fresh);
        fresh
    }
}

impl Payload {
    /// The reference's `Val` variant name, which the `Debug` rendering emits
    /// and 32 goldens capture. Bund2's own arm names differ — `Str` against
    /// `String` — so the mapping is explicit rather than derived, because a
    /// rename here would silently change captured text.
    fn val_name(&self) -> &'static str {
        match self {
            Payload::Str(_) => "String",
            Payload::Bin(_) => "Binary",
            Payload::List(_) => "List",
            Payload::Map(_) => "Map",
            Payload::ValueMap(_) => "ValueMap",
            Payload::Lambda(_) => "Lambda",
            Payload::Metrics(_) => "Metrics",
            Payload::Json(_) => "Json",
            Payload::Exit => "Exit",
            // A boxed scalar renders as the scalar it boxes, so this name is
            // not normally reached — but naming it is better than refusing to.
            Payload::Scalar(_) => "Scalar",
        }
    }
}

/// A stack name, interned. **D41.**
///
/// `0` means "never pushed". Anything else indexes the thread-local table in
/// [`stack_name`].
///
/// This rides in padding the enum already had: `BundValue` measures 16 bytes
/// with it and measured 16 without, because `Int(i64)` pads 9 bytes to 16
/// either way. A `u16` would also fit; `u32` is chosen for headroom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct StackSym(u32);

impl StackSym {
    /// Never pushed to a stack, so it carries no tag.
    pub const NONE: StackSym = StackSym(0);

    pub fn is_none(self) -> bool {
        self.0 == 0
    }
}

thread_local! {
    /// Symbol -> name. Index 0 is unused and stands for [`StackSym::NONE`].
    ///
    /// Thread-local because `BundValue` is neither `Send` nor `Sync`, so no
    /// lock is involved and no value can cross to a thread whose table would
    /// resolve its symbol differently.
    static STACK_NAMES: RefCell<Vec<Rc<str>>> = RefCell::new(vec![Rc::from("")]);
}

/// Intern a stack name. Cheap and idempotent; the table is tiny because a
/// program names a handful of stacks.
pub fn intern_stack_name(name: &str) -> StackSym {
    STACK_NAMES.with(|t| {
        let mut t = t.borrow_mut();
        if let Some(i) = t.iter().position(|n| &**n == name) {
            return StackSym(i as u32);
        }
        t.push(Rc::from(name));
        StackSym((t.len() - 1) as u32)
    })
}

thread_local! {
    /// The `"stack"` key, interned once. Every tag written names it.
    static STACK_KEY: Rc<str> = Rc::from("stack");
}

/// The interned `"stack"` tag key.
pub fn stack_tag_key() -> Rc<str> {
    STACK_KEY.with(Rc::clone)
}

/// Resolve a symbol back to the name it was interned from.
pub fn stack_name(sym: StackSym) -> Option<Rc<str>> {
    if sym.is_none() {
        return None;
    }
    STACK_NAMES.with(|t| t.borrow().get(sym.0 as usize).cloned())
}

/// The runtime value. **Two words: 16 bytes, 8-aligned.**
///
/// Each scalar variant carries a [`StackSym`] in padding the enum already had
/// — **D41**. It is metadata about where the value has been, never part of
/// what it is: equality, hashing and ordering all ignore it, and every one of
/// those is hand-written here so that omission is deliberate rather than
/// derived.
#[derive(Debug, Clone)]
pub enum BundValue {
    Int(i64, StackSym),
    Float(f64, StackSym),
    Bool(bool, StackSym),
    /// `dt` 97.
    Nodata(StackSym),
    /// `dt` 0. Distinct from `Nodata`: `Val::Null` carries both tags.
    None(StackSym),
    Heap(Rc<HeapValue>),
}

impl BundValue {
    /// The scalar constructors. **Use these, not the variants**: a bare
    /// variant would need the symbol spelled at 125 call sites, and the point
    /// of D41 is that a value acquires its tag by being pushed, not by being
    /// built.
    pub const fn int(v: i64) -> Self {
        BundValue::Int(v, StackSym::NONE)
    }
    pub const fn float(v: f64) -> Self {
        BundValue::Float(v, StackSym::NONE)
    }
    pub const fn boolean(v: bool) -> Self {
        BundValue::Bool(v, StackSym::NONE)
    }
    pub const fn nodata() -> Self {
        BundValue::Nodata(StackSym::NONE)
    }
    pub const fn none() -> Self {
        BundValue::None(StackSym::NONE)
    }

    /// The stack symbol this value carries, if it is an untagged scalar.
    pub fn stack_sym(&self) -> StackSym {
        match self {
            BundValue::Int(_, s)
            | BundValue::Float(_, s)
            | BundValue::Bool(_, s)
            | BundValue::Nodata(s)
            | BundValue::None(s) => *s,
            BundValue::Heap(_) => StackSym::NONE,
        }
    }

    /// The same value, tagged as having been pushed to `sym`.
    ///
    /// For a `Heap` value there is nowhere to put it, so the caller writes the
    /// map instead — see `Vm::push`.
    pub fn with_stack_sym(self, sym: StackSym) -> Self {
        match self {
            BundValue::Int(v, _) => BundValue::Int(v, sym),
            BundValue::Float(v, _) => BundValue::Float(v, sym),
            BundValue::Bool(v, _) => BundValue::Bool(v, sym),
            BundValue::Nodata(_) => BundValue::Nodata(sym),
            BundValue::None(_) => BundValue::None(sym),
            heap => heap,
        }
    }
}

impl BundValue {
    pub fn str(s: impl Into<String>) -> Self {
        Self::heap(STRING, Payload::Str(s.into()))
    }
    pub fn ptr(s: impl Into<String>) -> Self {
        Self::heap(PTR, Payload::Str(s.into()))
    }
    pub fn call(s: impl Into<String>) -> Self {
        Self::heap(CALL, Payload::Str(s.into()))
    }
    pub fn list(v: Vec<BundValue>) -> Self {
        Self::heap(LIST, Payload::List(v))
    }
    pub fn map(m: BTreeMap<String, BundValue>) -> Self {
        Self::heap(MAP, Payload::Map(m))
    }
    /// # Why a key with interior mutability is sound here
    ///
    /// `clippy::mutable_key_type` fires because `BundValue` reaches a
    /// `Cell<u64>`. The lint guards against a key whose hash can change while
    /// it sits in the map. This one cannot: **the identity slot is
    /// write-once.** `HeapValue::identity` mints only when the slot reads
    /// zero and never rewrites it, and both `hash` and `eq` go through that
    /// same accessor — so inserting a value fixes its identity, and every
    /// later hash returns what the first one minted.
    ///
    /// The interior mutability is what makes laziness observable through
    /// `&self` (D1); it is not mutation of the key's value.
    #[allow(clippy::mutable_key_type)]
    pub fn valuemap(m: HashMap<BundValue, BundValue>) -> Self {
        Self::heap(VALUEMAP, Payload::ValueMap(m))
    }
    /// `@name` — a stack switch naming its target.
    pub fn named_context(name: impl Into<String>) -> Self {
        Self::heap(CONTEXT, Payload::Str(name.into()))
    }
    /// The anonymous scratch context `( … )` opens.
    ///
    /// The reference generates a fresh nanoid for the name
    /// (`reference/rust_dynamic/src/create_special.rs:28`) so that no two
    /// contexts collide and none can be reached by `@name`. Bund2 mints from
    /// the same counter the identity uses, so the name is unique per process
    /// and has a nanoid's shape and width.
    pub fn context() -> Self {
        Self::named_context(format_id(mint()))
    }
    /// End of input. Carries no payload — `Val::Exit`
    /// (`reference/rust_dynamic/src/create_special.rs:85-96`).
    pub fn exit() -> Self {
        Self::heap(EXIT, Payload::Exit)
    }
    pub fn lambda(body: Vec<BundValue>) -> Self {
        Self::heap(LAMBDA, Payload::Lambda(body))
    }
    pub fn metrics(m: Vec<Metric>) -> Self {
        Self::heap(METRICS, Payload::Metrics(m))
    }
    pub fn json(j: serde_json::Value) -> Self {
        Self::heap(JSON, Payload::Json(j))
    }

    // The six tags below are the *same* payloads under different `dt`s, which
    // is the independent-axes design paying for itself: no new arm is needed
    // to reach six more of the twenty `dt` values the goldens carry.

    /// `from_pair` builds a list and then assigns `dt`
    /// (`reference/rust_dynamic/src/create.rs:164-167`).
    pub fn pair(a: BundValue, b: BundValue) -> Self {
        Self::heap(PAIR, Payload::List(vec![a, b]))
    }
    pub fn complex_float(re: f64, im: f64) -> Self {
        Self::heap(
            CFLOAT,
            Payload::List(vec![BundValue::float(re), BundValue::float(im)]),
        )
    }
    pub fn textbuffer(s: impl Into<String>) -> Self {
        Self::heap(TEXTBUFFER, Payload::Str(s.into()))
    }
    pub fn conditional(m: BTreeMap<String, BundValue>) -> Self {
        Self::heap(CONDITIONAL, Payload::Map(m))
    }
    pub fn class(m: BTreeMap<String, BundValue>) -> Self {
        Self::heap(CLASS, Payload::Map(m))
    }
    pub fn object(m: BTreeMap<String, BundValue>) -> Self {
        Self::heap(OBJECT, Payload::Map(m))
    }

    /// Build a value with an explicit `dt` over a given payload.
    ///
    /// The independent-axes design in one function: the differential renderer
    /// needs it because a golden says `dt: 10, data: List(…)` and nothing else
    /// distinguishes a `PAIR` from a `LIST`.
    pub fn with_dt(dt: u16, p: Payload) -> Self {
        Self::heap(dt, p)
    }

    /// Set `q`.
    ///
    /// `q` is a field, not the constant two RFC-0001 drafts made it. No word
    /// averages it (D32 as amended, Q35), but `Value::none` — which the
    /// JSON converter returns for a null — starts at **0.0**, not 100.0.
    ///
    /// This existed nowhere until `cargo xtask render` reported four
    /// differences against `q-observable.golden` and `dt-reachable.golden`:
    /// the constructor hardcoded 100.0 and nothing could move it, so the one
    /// case Q18 closed on was unrepresentable.
    pub fn with_q(self, q: f64) -> Self {
        let h = self.into_heap();
        let mut next = (*h).clone();
        next.q = q;
        BundValue::Heap(Rc::new(next))
    }

    /// Set the tags wholesale. Used to reconstruct a captured rendering.
    pub fn with_tags(self, tags: Tags) -> Self {
        let h = self.into_heap();
        let mut next = (*h).clone();
        next.tags = tags;
        BundValue::Heap(Rc::new(next))
    }

    fn heap(dt: u16, p: Payload) -> Self {
        BundValue::Heap(Rc::new(HeapValue::new(dt, p)))
    }

    /// The string a `Str` payload holds, if this is one.
    ///
    /// Any `dt` — `STRING`, `PTR`, `CALL`, `TEXTBUFFER` all carry `Str`, and
    /// a word that wants a name does not care which.
    pub fn as_str(&self) -> Option<String> {
        match self {
            BundValue::Heap(h) => match &*h.payload {
                Payload::Str(s) => Some(s.clone()),
                Payload::Scalar(inner) => inner.as_str(),
                _ => None,
            },
            _ => None,
        }
    }

    /// The integer this holds, boxed or not.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            BundValue::Int(i, _) => Some(*i),
            BundValue::Heap(h) => match &*h.payload {
                Payload::Scalar(inner) => inner.as_int(),
                _ => None,
            },
            _ => None,
        }
    }

    /// The inline scalar inside, looking through boxing.
    ///
    /// `push` boxes every scalar it touches, because `TS::push` writes a
    /// `stack` tag unconditionally and an inline `Int` has nowhere to keep
    /// one. So a word that pulls its operands never sees `BundValue::Int`; it
    /// sees `Heap { payload: Scalar(Int) }`. Any word that needs to know
    /// *which* scalar kind it has — as the comparisons do, since the
    /// reference's ordering branches on `Val::I64` against `Val::F64` — has to
    /// look through that, and matching the outer value alone silently takes
    /// the wrong arm.
    ///
    /// This is the tag/payload split seen from the other side: [`dt`] answers
    /// what the value is labelled, this answers what it holds.
    ///
    /// [`dt`]: BundValue::dt
    pub fn unboxed(&self) -> &BundValue {
        let mut v = self;
        while let BundValue::Heap(h) = v {
            match &*h.payload {
                Payload::Scalar(inner) => v = inner,
                _ => break,
            }
        }
        v
    }

    /// The map this holds, if it holds one.
    pub fn as_map(&self) -> Option<&BTreeMap<String, BundValue>> {
        match self {
            BundValue::Heap(h) => match &*h.payload {
                Payload::Map(m) => Some(m),
                _ => None,
            },
            _ => None,
        }
    }

    /// The valuemap this holds, if it holds one.
    pub fn as_valuemap(&self) -> Option<&HashMap<BundValue, BundValue>> {
        match self {
            BundValue::Heap(h) => match &*h.payload {
                Payload::ValueMap(m) => Some(m),
                _ => None,
            },
            _ => None,
        }
    }

    /// Re-tag a value, keeping its payload and header.
    ///
    /// **`set` rebuilds a map-like value as a MAP**, which is why `++` has to
    /// put the tag back: merging into a CONDITIONAL would otherwise return a
    /// MAP and stop `!` dispatching it. The reference writes `op_val1.dt =
    /// o_type` directly (`reference/Bund/src/stdlib/functions/values/merge.rs:81`);
    /// this is the same move through the header, since Bund2 has no writable
    /// `dt` field.
    ///
    /// The tag and the payload are independent axes, so this changes only the
    /// first — the pairing it produces is the caller's to justify.
    pub fn with_dt_tag(self, dt: u16) -> Self {
        let h = self.into_heap();
        let mut next = (*h).clone();
        next.dt = dt;
        BundValue::Heap(Rc::new(next))
    }

    /// The `serde_json::Value` this holds, if it holds one.
    ///
    /// Cloned rather than borrowed because the payload sits behind an `Rc` and
    /// `json.to_value` walks it while pushing onto the same VM.
    pub fn as_json(&self) -> Option<serde_json::Value> {
        match self {
            BundValue::Heap(h) => match &*h.payload {
                Payload::Json(j) => Some(j.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// The list this holds, if it holds one.
    pub fn as_list(&self) -> Option<&[BundValue]> {
        match self {
            BundValue::Heap(h) => match &*h.payload {
                Payload::List(v) => Some(v),
                _ => None,
            },
            _ => None,
        }
    }

    /// The sample buffer this holds, if it holds one.
    pub fn as_metrics(&self) -> Option<&[Metric]> {
        match self {
            BundValue::Heap(h) => match &*h.payload {
                Payload::Metrics(m) => Some(m),
                _ => None,
            },
            _ => None,
        }
    }

    /// The lambda body this holds, if it holds one.
    pub fn as_lambda(&self) -> Option<&[BundValue]> {
        match self {
            BundValue::Heap(h) => match &*h.payload {
                Payload::Lambda(v) => Some(v),
                _ => None,
            },
            _ => None,
        }
    }

    /// `Value::set`, arm for arm (`reference/rust_dynamic/src/set.rs:6-36`).
    ///
    /// Four behaviours, and three of them are not "insert into a map":
    ///
    /// - `LIST` **discards the container and the key**, returning a one-element
    ///   list (`:7-9`). That is F42, preserved.
    /// - `LAMBDA` replaces the whole body with one element (`:10-12`), which is
    ///   what D5 relies on for bodies being write-once — the original is
    ///   untouched and a new value comes back.
    /// - The map-like tags insert, preserving the **receiver's** `dt` (`:14-27`),
    ///   so a CONDITIONAL stays a CONDITIONAL. `?try :try { … } set` depends on
    ///   it. The key is trimmed (`:20`).
    /// - Anything else returns the *value*, carrying the receiver's `q`
    ///   (`:29-34`).
    pub fn set(&self, key: &str, value: BundValue) -> BundValue {
        match self.dt() {
            LIST => BundValue::list(vec![value]),
            LAMBDA => BundValue::lambda(vec![value]),
            MAP | CONDITIONAL | CLASS | OBJECT => {
                let mut m = self.as_map().cloned().unwrap_or_default();
                m.insert(key.trim().to_string(), value);
                self.rebuilt(self.dt(), Payload::Map(m))
            }
            _ => value.with_q(self.q()),
        }
    }

    /// `set` for a **`valuemap`**, whose key is a whole value rather than a
    /// string (`reference/rust_dynamic/src/set.rs:38-53`).
    ///
    /// `set` already branches to this when the receiver is a VALUEMAP
    /// (`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:17-19`),
    /// passing the key through uncast. D30 makes `get` mirror it.
    pub fn set_vmap(&self, key: BundValue, value: BundValue) -> BundValue {
        match self {
            BundValue::Heap(h) => match &*h.payload {
                Payload::ValueMap(m) => {
                    let mut next = m.clone();
                    next.insert(key, value);
                    self.rebuilt(self.dt(), Payload::ValueMap(next))
                }
                _ => self.clone(),
            },
            _ => self.clone(),
        }
    }

    /// Read a `valuemap` by whole-value key — **the read path D30 created**.
    ///
    /// F29 records that the reference has none: `get` casts the key to a
    /// string before it ever looks at the container
    /// (`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:52-59`),
    /// so a VALUEMAP can be written and never read. This works because
    /// `BundValue`'s `Hash` mirrors its `Eq` kind by kind, so a freshly built
    /// scalar key hashes into the bucket an equal one was stored under.
    pub fn get_vmap(&self, key: &BundValue) -> Option<BundValue> {
        match self {
            BundValue::Heap(h) => match &*h.payload {
                Payload::ValueMap(m) => m.get(key).cloned(),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether this is a `valuemap`, which decides whether `set` and `get`
    /// take the whole-value path.
    pub fn is_valuemap(&self) -> bool {
        matches!(self, BundValue::Heap(h) if matches!(&*h.payload, Payload::ValueMap(_)))
    }

    /// The **display** form — what `println` writes, as distinct from the
    /// `Debug` rendering a golden captures.
    ///
    /// A list is `"["` then, per element, a space, the element's display form
    /// and `" :: "`, then `"]"`
    /// (`reference/rust_dynamic/src/conv.rs:340-353`). So `[1, "a"]` shows as
    /// `[ 1 ::  a :: ]`, with the doubled space that falls out of every element
    /// carrying both a leading space and a trailing separator.
    /// Can this value convert to a STRING at all?
    ///
    /// **A PAIR cannot**, and that is the tag/payload split once more: it
    /// shares the LIST payload, but `value_list_conversion` admits `LIST` and
    /// `RESULT` only (`reference/rust_dynamic/src/conv.rs:706-711`), so a PAIR
    /// falls through every arm. Confirmed against the oracle —
    /// `1 2 pair println` reports `Can not convert Value from 10`.
    ///
    /// `CINTEGER` and `CFLOAT` are the same story from the other direction:
    /// they carry a numeric payload but `conv` reaches them through neither
    /// the float nor the integer arm, so `1.0 2.0 complex println` reports
    /// `Can not convert Value from 15`. Also confirmed.
    ///
    /// [`display`] answers for everything because it must return a string;
    /// this is what the words that *can* fail consult first.
    ///
    /// [`display`]: BundValue::display
    pub fn displayable(&self) -> bool {
        !matches!(self.dt(), PAIR | CINTEGER | CFLOAT)
    }

    pub fn display(&self) -> String {
        // **Dispatch on the tag before the payload.** `conv` matches the
        // payload arm and then re-checks the tag, sending a `Val::String` to
        // three different conversions depending on whether its `dt` is STRING,
        // CALL or PTR (`reference/rust_dynamic/src/conv.rs:698-703`). A PTR
        // renders `` `(name) `` (`:100`) and a CALL renders `F(name)` (`:84`),
        // where a STRING renders as itself.
        //
        // Reading `as_str` first collapsed all three, so `"dup" ptr println`
        // printed `dup` where the oracle prints `` `(dup) ``. That is the
        // tag/payload split from the rendering side, and it is the third place
        // in this crate where taking the payload arm alone was wrong.
        match self.dt() {
            PTR => return format!("`({})", self.as_str().unwrap_or_default()),
            CALL => return format!("F({})", self.as_str().unwrap_or_default()),
            // NODATA and NONE convert to their *names*, not to nothing
            // (`conv.rs:41-43,60-62`), so `nodata println` prints `NODATA`.
            NODATA => return "NODATA".to_string(),
            NONE => return "NONE".to_string(),
            _ => {}
        }
        if let Some(s) = self.as_str() {
            return s;
        }
        match self.unboxed() {
            BundValue::Int(i, _) => i.to_string(),
            BundValue::Float(f, _) => Self::float_text(*f),
            BundValue::Bool(b, _) => b.to_string(),
            BundValue::Nodata(_) | BundValue::None(_) => String::new(),
            BundValue::Heap(h) => match &*h.payload {
                Payload::List(items) => {
                    let mut out = "[".to_string();
                    for v in items {
                        out.push(' ');
                        out.push_str(&v.display());
                        out.push_str(" :: ");
                    }
                    out.push(']');
                    out
                }
                // A MAP renders `{ k=v ::  k=v :: }` — the same doubled-space
                // shape as a LIST, with `k=` before each element
                // (`reference/rust_dynamic/src/conv.rs:637-649`). A member that
                // will not convert is **skipped**, not reported (`:647`).
                //
                // Found by `pull`, which is the first word to print a MAP: the
                // oracle answers `{ a=3 ::  b=2 :: }` where this fell through
                // to the raw `Debug` form.
                Payload::Map(m) => {
                    let mut out = "{".to_string();
                    for (k, v) in m {
                        out.push(' ');
                        out.push_str(k);
                        out.push('=');
                        out.push_str(&v.display());
                        out.push_str(" :: ");
                    }
                    out.push('}');
                    out
                }
                _ => self.render(false),
            },
        }
    }

    /// `Value::get` for a string key (`reference/rust_dynamic/src/get.rs`).
    pub fn get(&self, key: &str) -> Option<BundValue> {
        self.as_map().and_then(|m| m.get(key.trim()).cloned())
    }

    /// Whether a string key is present.
    pub fn has_key(&self, key: &str) -> bool {
        self.as_map().is_some_and(|m| m.contains_key(key.trim()))
    }

    /// The reference's float→string rendering: `dtoa`, not Rust's `{:?}`
    /// (`reference/rust_dynamic/src/conv.rs:128-130`).
    ///
    /// The two disagree on which shortest form to emit. `2.0 math.sqrt` holds
    /// the same bits in both engines — `F64(1.4142135623730951)`, confirmed
    /// through `debug.display_stack` — but the reference prints
    /// `1.4142135623730952`. Both parse back to that same `f64`, so neither is
    /// wrong; they are different libraries, and a golden captures the bytes.
    ///
    /// **`render` deliberately does not use this.** The `Value { … }` form is
    /// Rust's `Debug`, so `F64(…)` must stay `{:?}` — using `dtoa` there would
    /// break every golden that dumps a stack.
    pub(crate) fn float_text(f: f64) -> String {
        let mut buf = dtoa::Buffer::new();
        buf.format(f).to_string()
    }

    /// A compact, bounded rendering — **what a person reads**.
    ///
    /// [`render`] produces the reference's `Debug` form, which is what the
    /// goldens capture and what a debug session wants: every header field, on
    /// one line, ~150 columns for a single integer. In an error report a stack
    /// of those is unreadable, and a stack of ten overflows any terminal.
    ///
    /// This shows the value and nothing else, truncating with `…` so a row can
    /// never exceed `width`. Containers show their kind and size before their
    /// contents, because on a failure path "a list of 900" is the useful fact
    /// and the 900 elements are not.
    ///
    /// [`render`]: BundValue::render
    pub fn summary(&self, width: usize) -> String {
        let mut out = String::new();
        self.summarise(&mut out, width, 0);
        out
    }

    fn summarise(&self, out: &mut String, width: usize, depth: usize) {
        // Nesting deeper than this is noise in a one-line summary.
        const MAX_DEPTH: usize = 2;
        // Enough elements to recognise the shape, not enough to fill a line.
        const MAX_ITEMS: usize = 4;

        if out.chars().count() >= width {
            return;
        }
        let inner = self.unboxed();
        match inner {
            BundValue::Int(i, _) => out.push_str(&i.to_string()),
            BundValue::Float(f, _) => out.push_str(&format!("{f:?}")),
            BundValue::Bool(b, _) => out.push_str(&b.to_string()),
            BundValue::Nodata(_) => out.push_str("nodata"),
            BundValue::None(_) => out.push_str("none"),
            BundValue::Heap(h) => match &*h.payload {
                Payload::Str(s) => {
                    // `dt` distinguishes what a `Str` payload means: a CALL is
                    // a word, a PTR is a reference to one, a CONTEXT is a
                    // stack. Showing them alike would hide the difference that
                    // matters most on a failure path.
                    match self.dt() {
                        CALL => out.push_str(&truncate(s, width)),
                        PTR => out.push_str(&format!("`{}", truncate(s, width.saturating_sub(1)))),
                        CONTEXT => out.push_str(&format!("@{}", truncate(s, width.saturating_sub(1)))),
                        _ => out.push_str(&format!("\"{}\"", truncate(s, width.saturating_sub(2)))),
                    }
                }
                Payload::Bin(b) => out.push_str(&format!("bin/{}", b.len())),
                Payload::Exit => out.push_str("exit"),
                Payload::Metrics(m) => out.push_str(&format!("metrics/{}", m.len())),
                Payload::Json(_) => out.push_str("json"),
                Payload::Scalar(v) => v.summarise(out, width, depth),
                Payload::Lambda(body) => {
                    // A lambda's body is code. Its length is the useful fact.
                    out.push_str(&format!("lambda/{}", body.len()));
                }
                Payload::List(items) => {
                    out.push_str(&format!("list/{}", items.len()));
                    if depth < MAX_DEPTH && !items.is_empty() {
                        out.push_str(" [");
                        for (n, it) in items.iter().take(MAX_ITEMS).enumerate() {
                            if n > 0 {
                                out.push_str(", ");
                            }
                            it.summarise(out, width, depth + 1);
                        }
                        if items.len() > MAX_ITEMS {
                            out.push_str(", …");
                        }
                        out.push(']');
                    }
                }
                Payload::Map(m) => {
                    out.push_str(&format!("{}/{}", tag_name(self.dt()), m.len()));
                    if depth < MAX_DEPTH && !m.is_empty() {
                        out.push_str(" {");
                        for (n, (k, v)) in m.iter().take(MAX_ITEMS).enumerate() {
                            if n > 0 {
                                out.push_str(", ");
                            }
                            out.push_str(k);
                            out.push_str(": ");
                            v.summarise(out, width, depth + 1);
                        }
                        if m.len() > MAX_ITEMS {
                            out.push_str(", …");
                        }
                        out.push('}');
                    }
                }
                Payload::ValueMap(m) => out.push_str(&format!("valuemap/{}", m.len())),
            },
        }
        // Hard bound: a row can never exceed `width`, whatever the recursion
        // produced.
        if out.chars().count() > width {
            let kept: String = out.chars().take(width.saturating_sub(1)).collect();
            *out = format!("{kept}…");
        }
    }

    /// The `dt` tag.
    pub fn dt(&self) -> u16 {
        match self {
            BundValue::Int(_, _) => INTEGER,
            BundValue::Float(_, _) => FLOAT,
            BundValue::Bool(_, _) => BOOL,
            BundValue::Nodata(_) => NODATA,
            BundValue::None(_) => NONE,
            BundValue::Heap(h) => h.dt,
        }
    }

    /// The reference's name for this value's `dt`
    /// (`reference/rust_dynamic/src/value_types.rs:8-54`).
    ///
    /// This is a **tag** name, not a payload name, so it answers on the `dt`
    /// axis alone: a boxed `Int` reads `Integer` because `promote` carries the
    /// scalar's `dt` onto the header it builds. That is what makes `type` safe
    /// to write without [`unboxed`] — unlike the comparisons, which branch on
    /// the arm and so must look through the box.
    ///
    /// The arms are exactly the tags Bund2 can construct. The reference names
    /// fourteen more, all of them tags Bund2 has no writer for, and its own
    /// last arm is `_ => "Unknown"` (`:52`) — so falling through to the same
    /// answer is the reference's behaviour, not a gap in this table.
    ///
    /// [`unboxed`]: BundValue::unboxed
    pub fn type_name(&self) -> &'static str {
        match self.dt() {
            NONE => "None",
            NODATA => "NODATA",
            BOOL => "Bool",
            INTEGER => "Integer",
            FLOAT => "Float",
            STRING => "String",
            CALL => "Call",
            PTR => "Ptr",
            LIST => "List",
            PAIR => "Pair",
            MAP => "Map",
            TIME => "Time",
            CINTEGER => "ComplexInteger",
            CFLOAT => "ComplexFloat",
            METRICS => "Metrics",
            LAMBDA => "Lambda",
            CONTEXT => "Context",
            TEXTBUFFER => "TextBuffer",
            JSON => "JSON",
            CONDITIONAL => "Conditional",
            VALUEMAP => "ValueMap",
            CLASS => "CLASS",
            OBJECT => "OBJECT",
            EXIT => "Exit",
            _ => "Unknown",
        }
    }

    /// `q`. Rendered from the header for heap values; scalars sit at the
    /// 100.0 fixpoint until something moves them, which requires a header.
    pub fn q(&self) -> f64 {
        match self {
            BundValue::Heap(h) => h.q,
            _ => 100.0,
        }
    }

    /// The identity, minting on first need.
    ///
    /// An **unadorned scalar has nowhere to keep one**, so observing its
    /// identity promotes it — and the promoted value is returned, because the
    /// caller must write it back for observation to be idempotent. That
    /// write-back is the point RFC-0001's earlier drafts missed: `get` returns
    /// a clone, so promoting the clone alone changes nothing.
    /// Whether identity has already been minted, **without minting it**.
    ///
    /// For tests that need to assert a path did *not* materialise identity —
    /// calling [`BundValue::identity`] to find out would mint it and destroy
    /// the thing being measured. A scalar has no header, so it is never
    /// minted.
    pub fn has_identity(&self) -> bool {
        match self {
            BundValue::Heap(h) => h.identity.get() != 0,
            _ => false,
        }
    }

    /// **D35's cache key**: the address of the payload a heap value carries.
    ///
    /// A `dup`'d value shares its original's payload, so the two share a key;
    /// a rebuilt value — `set`, `push` — has a new payload and a new key, which
    /// is D5's write-once property seen from here. `None` for an unboxed
    /// scalar, which has no payload to point at. D42 makes this reachable
    /// where a body starts running: `Vm::eval_lambda` and the frame receive the
    /// value, not a copy of its items.
    pub fn payload_key(&self) -> Option<usize> {
        match self {
            BundValue::Heap(h) => Some(Rc::as_ptr(&h.payload) as *const () as usize),
            _ => None,
        }
    }

    pub fn identity(&self) -> (u64, Option<BundValue>) {
        match self {
            BundValue::Heap(h) => (h.identity(), None),
            scalar => {
                let h = scalar.clone().into_heap();
                let id = h.identity();
                (id, Some(BundValue::Heap(h)))
            }
        }
    }

    /// Box a scalar and hand back the header itself.
    ///
    /// `promote` always yields a `Heap`, but its return type does not say so,
    /// which forced every caller to match and then explain what to do when the
    /// impossible happened. Returning the `Rc` makes the invariant
    /// **structural**: there is no arm left to write, and nothing to panic in.
    fn into_heap(self) -> Rc<HeapValue> {
        match self.promote() {
            BundValue::Heap(h) => h,
            // `promote` returns `Heap` for every input; this arm exists only
            // because the signature cannot express that. Boxing again is a
            // correct answer rather than an abort.
            other => Rc::new(HeapValue::new(other.dt(), Payload::Scalar(other))),
        }
    }

    /// Box a scalar so it can carry a header. A heap value is returned as is.
    pub fn promote(self) -> Self {
        match self {
            BundValue::Heap(_) => self,
            scalar => {
                // **D41 rule 2.** A tagged scalar being boxed must move its
                // symbol into the header's map, or a value that later gains an
                // `attr` would silently lose the stack tag it was pushed with.
                // This is the one rule in D41 whose violation is invisible
                // until a golden disagrees.
                let sym = scalar.stack_sym();
                let dt = scalar.dt();
                let inner = scalar.with_stack_sym(StackSym::NONE);
                let boxed = BundValue::heap(dt, Payload::Scalar(inner));
                match stack_name(sym) {
                    Some(n) => boxed.with_tag(stack_tag_key(), n),
                    None => boxed,
                }
            }
        }
    }

    /// Whether this value carries a header.
    pub fn is_boxed(&self) -> bool {
        matches!(self, BundValue::Heap(_))
    }

    /// `dup`: a fresh header with a **cleared** identity, sharing the payload.
    ///
    /// Not an `Rc` bump. D13's contract is clone-equal versus dup-unequal, and
    /// an `Rc` bump makes `dup` behave exactly like `Clone`. The reference's
    /// `dup` is a bincode round trip that overwrites only `id`
    /// (`reference/rust_dynamic/src/dup.rs:11`), so the copy keeps the
    /// original's stamp, tags, attr, curr and q — this copies all five and
    /// materialises the stamp first, since two unobserved values would
    /// otherwise sample independently and differ.
    pub fn dup(&self) -> Self {
        match self {
            BundValue::Heap(h) => {
                let stamp = h.stamp.get();
                BundValue::Heap(Rc::new(HeapValue {
                    identity: Cell::new(0),
                    stamp: Cell::new(stamp),
                    dt: h.dt,
                    q: h.q,
                    curr: h.curr,
                    tags: h.tags.clone(),
                    attr: h.attr.clone(),
                    payload: Rc::clone(&h.payload),
                }))
            }
            scalar => scalar.clone(),
        }
    }

    /// The language's `==`: IEEE float semantics, so `NaN != NaN`.
    ///
    /// Distinct from [`PartialEq`], which is the *key* equality and must be
    /// reflexive for `HashMap` to be sound.
    pub fn eq_ieee(&self, other: &Self) -> bool {
        match (self, other) {
            (BundValue::Float(a, _), BundValue::Float(b, _)) => a == b,
            (BundValue::Float(a, _), BundValue::Int(b, _)) => int_eq_float(*b, *a),
            (BundValue::Int(a, _), BundValue::Float(b, _)) => int_eq_float(*a, *b),
            _ => self == other,
        }
    }
}

/// Exact equality between an integer and a float (D30's amendment).
///
/// Neither truncating nor widening: both are non-transitive, the first at
/// 42/42.5/42.9 and the second above 2^53. Two values are equal when they
/// denote the same mathematical value.
fn int_eq_float(i: i64, f: f64) -> bool {
    f.is_finite() && f.fract() == 0.0 && f >= -(2f64.powi(63)) && f < 2f64.powi(63) && f as i64 == i
}

/// A float's key form: total, so `Eq` is reflexive.
///
/// All `NaN`s become one bit pattern and `-0.0` becomes `0.0`. Without this a
/// `HashMap<BundValue, _>` is unsound, which is the fault RFC-0001 convicts
/// the reference of and which D30's read path makes reachable.
fn float_key(f: f64) -> u64 {
    if f.is_nan() {
        return f64::NAN.to_bits();
    }
    if f == 0.0 {
        return 0f64.to_bits();
    }
    f.to_bits()
}

impl PartialEq for BundValue {
    fn eq(&self, other: &Self) -> bool {
        use BundValue::*;
        match (self, other) {
            // Six content-compared kinds. `Bool` and the two nullary scalars
            // are content-compared in Bund2 where the reference compares them
            // by identity — equality must not depend on whether an operand
            // happens to be boxed.
            (Int(a, _), Int(b, _)) => a == b,
            (Float(a, _), Float(b, _)) => float_key(*a) == float_key(*b),
            (Bool(a, _), Bool(b, _)) => a == b,
            (Nodata(_), Nodata(_)) | (None(_), None(_)) => true,
            (Int(a, _), Float(b, _)) | (Float(b, _), Int(a, _)) => int_eq_float(*a, *b),
            // A boxed scalar compares as the scalar it boxes, so boxing is
            // invisible here.
            // Destructured in the guard rather than after it, so there is no
            // second match that could fail.
            (Heap(h), other) | (other, Heap(h))
                if matches!(&*h.payload, Payload::Scalar(inner) if inner == other) =>
            {
                true
            }
            (Heap(h), _) | (_, Heap(h)) if matches!(*h.payload, Payload::Scalar(_)) => false,
            // A string payload compares by **content**, whatever its `dt`.
            // `eq.rs` matches on `self.data`, so `Val::String` is
            // content-compared whether it is tagged `STRING`, `PTR` or `CALL`
            // (`reference/rust_dynamic/src/eq.rs:29-36`). A first draft of
            // this file put every heap value in the identity bucket, and a
            // test asserted the resulting miss as correct.
            (Heap(a), Heap(b)) => match (&*a.payload, &*b.payload) {
                (Payload::Str(x), Payload::Str(y)) => x == y,
                // Everything else with a header compares by identity, as the
                // reference does through `eq.rs:53`.
                _ => a.identity() == b.identity(),
            },
            _ => false,
        }
    }
}

impl Eq for BundValue {}

impl Hash for BundValue {
    /// Mirrors `PartialEq` arm for arm, which is what keeps the `Hash`/`Eq`
    /// contract. An integral in-range float hashes as the `i64` it denotes, so
    /// `42` and `42.0` share a bucket.
    fn hash<H: Hasher>(&self, state: &mut H) {
        use BundValue::*;
        match self {
            Int(i, _) => i.hash(state),
            Float(f, _) => {
                if f.fract() == 0.0 && f.is_finite() && *f >= -(2f64.powi(63)) && *f < 2f64.powi(63)
                {
                    (*f as i64).hash(state)
                } else {
                    float_key(*f).hash(state)
                }
            }
            Bool(b, _) => b.hash(state),
            Nodata(_) => 0x4E4F_4441u64.hash(state), // "NODA"
            None(_) => 0u64.hash(state),
            Heap(h) => match &*h.payload {
                Payload::Scalar(inner) => inner.hash(state),
                // Mirrors the content comparison above; without this a
                // string key hashes by identity and never finds its entry,
                // which is the miss D30 exists to fix.
                Payload::Str(x) => x.hash(state),
                _ => h.identity().hash(state),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC-0001 criterion D1.
    #[test]
    fn the_value_is_two_words() {
        assert_eq!(size_of::<BundValue>(), 16);
        assert_eq!(align_of::<BundValue>(), 8);
    }

    /// RFC-0001 criterion D2, and D13's contract. `Clone` shares the identity
    /// slot; `dup` clears it.
    ///
    /// **Only for identity-compared kinds.** RFC-0001's criterion said "every
    /// kind with a heap header to carry an identity", which is the wrong
    /// discriminator: a string has a header *and* compares by content, so
    /// `dup` cannot make it unequal. Confirmed against the oracle —
    /// `"s" dup ==` prints `true`. The criterion is corrected; this test is
    /// what found it.
    #[test]
    fn clone_is_equal_and_dup_is_not_for_identity_compared_kinds() {
        for v in [
            BundValue::list(vec![BundValue::int(1)]),
            BundValue::map(BTreeMap::new()),
        ] {
            assert_eq!(v, v.clone(), "clone-equal failed for dt {}", v.dt());
            assert_ne!(v, v.dup(), "dup-unequal failed for dt {}", v.dt());
        }
    }

    /// `dup` shares the payload and copies the rest of the header. The
    /// reference's `dup` overwrites only `id`.
    #[test]
    fn dup_shares_the_payload_and_keeps_the_dt() {
        let a = BundValue::ptr("x");
        let b = a.dup();
        assert_eq!(a.dt(), b.dt());
        let (BundValue::Heap(ha), BundValue::Heap(hb)) = (&a, &b) else {
            panic!("expected heap values")
        };
        assert!(Rc::ptr_eq(&ha.payload, &hb.payload), "payload not shared");
    }

    /// A bool compares by content, where the reference compares by identity.
    /// The reason is stability under boxing, not the absence of an identity
    /// slot — a boxed scalar has one.
    #[test]
    fn scalars_compare_by_content_boxed_or_not() {
        assert_eq!(BundValue::boolean(true), BundValue::boolean(true));
        assert_eq!(
            BundValue::boolean(true).promote(),
            BundValue::boolean(true),
            "boxing must not change equality"
        );
        assert_eq!(
            BundValue::boolean(true).promote(),
            BundValue::boolean(true).promote()
        );
    }

    /// D30's amendment: exact, and therefore symmetric and transitive.
    /// Truncation fails at 42/42.5/42.9; widening fails above 2^53.
    #[test]
    fn int_float_equality_is_exact_in_both_orientations() {
        let (i, f_eq, f_ne) = (
            BundValue::int(42),
            BundValue::float(42.0),
            BundValue::float(42.5),
        );
        assert_eq!(i, f_eq);
        assert_eq!(f_eq, i);
        assert_ne!(i, f_ne);
        assert_ne!(f_ne, i);

        // 2^53 + 1 is not representable as f64; widening would say equal.
        let big = BundValue::int(9_007_199_254_740_993);
        let near = BundValue::float(9_007_199_254_740_992.0);
        assert_ne!(big, near);
        assert_ne!(near, big);
    }

    /// Transitivity, which is what rules truncation out.
    #[test]
    fn equality_is_transitive_across_int_and_float() {
        let a = BundValue::int(42);
        let b = BundValue::float(42.5);
        let c = BundValue::float(42.9);
        // Truncation would make a == b and a == c while b != c.
        assert!(!(a == b && a == c && b != c));
    }

    /// `Eq` needs reflexivity and `NaN` denies it under IEEE, so the key
    /// equality is total while the word's is not.
    #[test]
    fn nan_is_reflexive_as_a_key_and_not_as_a_word() {
        let nan = BundValue::float(f64::NAN);
        assert_eq!(nan, nan.clone(), "Eq must be reflexive for HashMap");
        assert!(!nan.eq_ieee(&nan), "the word == keeps IEEE semantics");
    }

    #[test]
    fn negative_zero_and_zero_are_one_key() {
        assert_eq!(BundValue::float(-0.0), BundValue::float(0.0));
        assert_eq!(
            hash_of(&BundValue::float(-0.0)),
            hash_of(&BundValue::float(0.0))
        );
    }

    /// D30: equal values must hash alike, or the valuemap lookup misses.
    #[test]
    fn equal_values_hash_alike() {
        assert_eq!(
            hash_of(&BundValue::int(42)),
            hash_of(&BundValue::float(42.0))
        );
        assert_eq!(
            hash_of(&BundValue::boolean(true)),
            hash_of(&BundValue::boolean(true).promote())
        );
    }

    /// The point of D30, and the case that motivated it:
    /// `valuemap "k" 42 set "k" get` must return `42`.
    ///
    /// A first draft of this file compared every heap value by identity, so a
    /// freshly built `"k"` missed — and the test asserted the miss as correct.
    /// The reference content-compares any `Val::String` payload regardless of
    /// `dt` (`reference/rust_dynamic/src/eq.rs:29-36`).
    #[test]
    fn a_valuemap_finds_a_freshly_built_equal_key() {
        let mut m = HashMap::new();
        m.insert(BundValue::str("k"), BundValue::int(42));
        assert_eq!(m.get(&BundValue::str("k")), Some(&BundValue::int(42)));
        m.insert(BundValue::int(7), BundValue::int(1));
        assert_eq!(m.get(&BundValue::int(7)), Some(&BundValue::int(1)));
    }

    /// D30's stated limit: composite keys stay identity-keyed, because `eq`
    /// for a list is identity and changing that reaches past `valuemap`.
    #[test]
    fn a_valuemap_does_not_find_a_freshly_built_composite_key() {
        let mut m = HashMap::new();
        m.insert(BundValue::list(vec![BundValue::int(1)]), BundValue::int(9));
        assert_eq!(m.get(&BundValue::list(vec![BundValue::int(1)])), None);
    }

    /// Where content comparison and D13's contract meet, content wins — and
    /// the reference agrees: `"s" dup ==` prints `true` on the oracle.
    #[test]
    fn dup_of_a_string_compares_equal_because_strings_compare_by_content() {
        let a = BundValue::str("s");
        assert_eq!(a, a.dup(), "content comparison wins over identity for Str");
        assert_ne!(
            BundValue::list(vec![]),
            BundValue::list(vec![]).dup(),
            "and identity still wins for a list"
        );
    }

    /// Observing a scalar's identity promotes it, and the promoted value must
    /// be written back or the next observation mints again.
    #[test]
    fn observing_a_scalar_identity_promotes_and_returns_the_box() {
        let v = BundValue::int(1);
        let (first, promoted) = v.identity();
        let promoted = promoted.expect("a scalar must promote");
        assert!(promoted.is_boxed());
        let (second, none) = promoted.identity();
        assert!(none.is_none(), "a boxed value does not promote again");
        assert_eq!(
            first, second,
            "observation must be idempotent once written back"
        );
    }

    /// The slot is write-once, which is what makes `BundValue` sound as a
    /// `HashMap` key despite its interior mutability — a hash that changed
    /// under the map would corrupt it.
    #[test]
    fn the_identity_slot_is_write_once() {
        let v = BundValue::list(vec![]);
        let (first, _) = v.identity();
        let (second, _) = v.identity();
        assert_eq!(first, second);
        assert_eq!(hash_of(&v), hash_of(&v), "hash must not move once minted");
    }

    /// The lazy slot: nothing is minted until something needs it.
    #[test]
    fn identity_is_not_minted_until_needed() {
        let v = BundValue::list(vec![]);
        let BundValue::Heap(h) = &v else { panic!() };
        assert_eq!(h.identity.get(), 0, "constructing must not mint");
        let _ = v.identity();
        assert_ne!(h.identity.get(), 0, "observing must mint");
    }

    /// `dt` and payload are independent axes: three tags, one payload shape.
    #[test]
    fn dt_is_not_derivable_from_the_payload() {
        assert_eq!(BundValue::str("x").dt(), STRING);
        assert_eq!(BundValue::ptr("x").dt(), PTR);
        assert_eq!(BundValue::call("x").dt(), CALL);
    }

    /// `Val::Null` carries two tags, so Bund2 has two arms.
    #[test]
    fn nodata_and_none_are_distinct() {
        assert_ne!(BundValue::nodata(), BundValue::none());
        assert_eq!(BundValue::nodata().dt(), NODATA);
        assert_eq!(BundValue::none().dt(), NONE);
    }

    fn hash_of(v: &BundValue) -> u64 {
        use std::hash::DefaultHasher;
        let mut h = DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    }
}

// ---------------------------------------------------------------------------
// Rendering (RFC-0001 criterion D3)
// ---------------------------------------------------------------------------

impl BundValue {
    /// The `.timestamp` accessor (D2). Sampled on first observation, like the
    /// identity — and like it, an unadorned scalar has nowhere to keep the
    /// sample, so observing promotes and the caller writes the promotion back.
    ///
    /// **The stamp orders by observation, not construction.** A value built
    /// first and observed second carries the later stamp. That is what
    /// laziness gives up, and criterion D7 asserts it rather than working
    /// around it.
    pub fn timestamp(&self) -> (f64, Option<BundValue>) {
        match self {
            BundValue::Heap(h) => {
                let s = h.stamp.get();
                if s != 0.0 {
                    return (s, None);
                }
                let fresh = now_ms();
                h.stamp.set(fresh);
                (fresh, None)
            }
            scalar => {
                let boxed = scalar.clone().promote();
                let (s, _) = boxed.timestamp();
                (s, Some(boxed))
            }
        }
    }

    /// The `.id` accessor: a 21-character nanoid-shaped string.
    ///
    /// Returns the promoted value alongside it, for the same reason
    /// [`BundValue::identity`] does — an unadorned scalar has nowhere to keep
    /// what it minted, so the caller must write the promotion back.
    pub fn id_string(&self) -> (String, Option<BundValue>) {
        let (n, promoted) = self.identity();
        (format_id(n), promoted)
    }

    /// The reference's `Debug` text for this value.
    ///
    /// `normalised` produces the form the goldens hold — `id: "<id>"` and
    /// `stamp: <stamp>` — which is what criterion D3 compares against. The
    /// goldens are normalised for F14 before capture, so the target is that
    /// text and **not** the reference's raw output; an earlier draft of D3
    /// asked for the raw form, which would also have demanded the `HashMap`
    /// ordering this RFC replaces.
    pub fn render(&self, normalised: bool) -> String {
        let mut out = String::new();
        self.render_into(&mut out, normalised);
        out
    }

    fn render_into(&self, out: &mut String, norm: bool) {
        use std::fmt::Write;
        // A scalar renders through a synthetic header: the reference has no
        // unboxed values, so every rendering is a full `Value { .. }`.
        let (dt, q, curr) = (self.dt(), self.q(), self.curr());
        let id = if norm {
            "<id>".to_string()
        } else {
            self.peek_id()
        };
        let stamp = if norm {
            "<stamp>".to_string()
        } else {
            format!("{:?}", self.render_stamp())
        };
        let _ = write!(
            out,
            "Value {{ id: \"{id}\", stamp: {stamp}, dt: {dt}, q: {q:?}, data: "
        );
        self.render_payload(out, norm);
        let _ = write!(out, ", attr: [");
        for (i, a) in self.attr().iter().enumerate() {
            if i > 0 {
                let _ = write!(out, ", ");
            }
            a.render_into(out, norm);
        }
        let _ = write!(out, "], curr: {curr}, tags: {{");
        for (i, (k, v)) in self.tags().iter().enumerate() {
            if i > 0 {
                let _ = write!(out, ", ");
            }
            let _ = write!(out, "{k:?}: {v:?}");
        }
        let _ = write!(out, "}} }}");
    }

    fn render_payload(&self, out: &mut String, norm: bool) {
        use std::fmt::Write;
        match self {
            BundValue::Int(i, _) => {
                let _ = write!(out, "I64({i})");
            }
            BundValue::Float(f, _) => {
                let _ = write!(out, "F64({f:?})");
            }
            BundValue::Bool(b, _) => {
                let _ = write!(out, "Bool({b})");
            }
            BundValue::Nodata(_) | BundValue::None(_) => out.push_str("Null"),
            BundValue::Heap(h) => match &*h.payload {
                Payload::Scalar(inner) => inner.render_payload(out, norm),
                Payload::Exit => out.push_str("Exit"),
                Payload::Str(x) => {
                    let _ = write!(out, "String({x:?})");
                }
                Payload::Bin(b) => {
                    let _ = write!(out, "Binary({b:?})");
                }
                Payload::Metrics(m) => {
                    let _ = write!(out, "Metrics([");
                    for (i, e) in m.iter().enumerate() {
                        if i > 0 {
                            let _ = write!(out, ", ");
                        }
                        let stamp = if norm {
                            "<stamp>".into()
                        } else {
                            e.stamp.to_string()
                        };
                        let _ = write!(out, "Metric {{ stamp: {stamp}, data: {:?} }}", e.data);
                    }
                    out.push_str("])");
                }
                Payload::Json(j) => {
                    // **`Debug`, not `Display`.** The oracle derives `Debug` on
                    // `Val` (`reference/rust_dynamic/src/types.rs:65`) whose arm
                    // is `Json(serde_json::Value)` (`:85`), so it prints
                    // serde_json's own `Debug` — `Null`, not `null`. `Display`
                    // would serialise the JSON instead, which agrees with the
                    // oracle on no value at all.
                    //
                    // This is only stable because both sides resolve the same
                    // serde_json (1.0.151); its `Debug` is hand-written, not
                    // derived, so a version skew could move the rendering
                    // without moving the payload.
                    let _ = write!(out, "Json({j:?})");
                }
                p @ Payload::Lambda(v) => {
                    let _ = write!(out, "{}([", p.val_name());
                    for (i, e) in v.iter().enumerate() {
                        if i > 0 {
                            let _ = write!(out, ", ");
                        }
                        e.render_into(out, norm);
                    }
                    out.push_str("])");
                }
                p @ Payload::List(v) => {
                    let _ = write!(out, "{}([", p.val_name());
                    for (i, e) in v.iter().enumerate() {
                        if i > 0 {
                            let _ = write!(out, ", ");
                        }
                        e.render_into(out, norm);
                    }
                    out.push_str("])");
                }
                p @ Payload::Map(m) => {
                    let _ = write!(out, "{}({{", p.val_name());
                    for (i, (k, v)) in m.iter().enumerate() {
                        if i > 0 {
                            let _ = write!(out, ", ");
                        }
                        let _ = write!(out, "{k:?}: ");
                        v.render_into(out, norm);
                    }
                    out.push_str("})");
                }
                p @ Payload::ValueMap(m) => {
                    // Ordered by the rendered key. The container hashes, per
                    // D30; determinism comes from the renderer, which is the
                    // distinction an earlier draft collapsed by reaching for
                    // one map type to satisfy both requirements.
                    let _ = write!(out, "{}({{", p.val_name());
                    let mut entries: Vec<(String, &BundValue)> =
                        m.iter().map(|(k, v)| (k.render(true), v)).collect();
                    entries.sort_by(|a, b| a.0.cmp(&b.0));
                    for (i, (k, v)) in entries.iter().enumerate() {
                        if i > 0 {
                            let _ = write!(out, ", ");
                        }
                        out.push_str(k);
                        out.push_str(": ");
                        v.render_into(out, norm);
                    }
                    out.push_str("})");
                }
            },
        }
    }

    /// The id *without* minting, for rendering. Rendering an unminted value
    /// would otherwise mint one, which would make `Debug` an observation and
    /// D2's laziness unobservable in the goldens.
    fn peek_id(&self) -> String {
        match self {
            BundValue::Heap(h) if h.identity.get() != 0 => format_id(h.identity.get()),
            _ => format_id(0),
        }
    }

    /// The stamp for rendering, **materialising it** if it has not been.
    ///
    /// Identity can stay lazy through a render because `format_id(0)` is a
    /// 21-character placeholder — exactly a nanoid's width — so nothing
    /// downstream can tell. The stamp has no such luxury. A real one prints as
    /// `1787954810882.0`, fifteen characters; an unmaterialised `0.0` is
    /// three. The goldens normalise the *text* to `<stamp>`, but the width was
    /// already baked into the box `debug.display_stack` drew around it:
    /// `comfy_table` sizes the column to its content, so a short stamp yields
    /// a border twelve columns narrower than the golden's and the row fails on
    /// a value that normalisation says is equal. Laziness that changes the
    /// output is not laziness, it is a defect.
    ///
    /// Materialising here is D2's own rule — a stamp is taken at first *need*,
    /// and being printed is a need — and it inherits D7's accepted
    /// consequence, that stamps order by observation rather than construction.
    ///
    /// A scalar has nowhere to keep the sample and `render` cannot promote
    /// through `&self`, so it samples without caching. That costs a repeated
    /// `now_ms()` on an unboxed value, which `push` has usually already boxed.
    fn render_stamp(&self) -> f64 {
        match self {
            BundValue::Heap(h) => {
                let s = h.stamp.get();
                if s != 0.0 {
                    return s;
                }
                let fresh = now_ms();
                h.stamp.set(fresh);
                fresh
            }
            _ => now_ms(),
        }
    }

    pub fn curr(&self) -> i32 {
        match self {
            BundValue::Heap(h) => h.curr,
            _ => -1,
        }
    }

    pub fn attr(&self) -> &[BundValue] {
        match self {
            BundValue::Heap(h) => &h.attr,
            _ => &[],
        }
    }

    /// This value's tags. **D41 rule 3.**
    ///
    /// A tagged scalar has no map, so its `"stack"` entry is synthesised from
    /// the inline symbol. Borrowed for a heap value, owned for a scalar — the
    /// render path and the wire format see one shape either way, which is what
    /// keeps the 39 tag-bearing goldens identical.
    pub fn tags(&self) -> std::borrow::Cow<'_, Tags> {
        use std::borrow::Cow;
        if let Some(name) = stack_name(self.stack_sym()) {
            let mut m = Tags::new();
            m.insert(stack_tag_key(), name);
            return Cow::Owned(m);
        }
        match self {
            BundValue::Heap(h) => Cow::Borrowed(&h.tags),
            // A scalar carries no tags. The empty map is thread-local and
            // leaked once rather than a `OnceLock`, because `Tags` holds `Rc`
            // and a `static` would require `Sync` — which `BundValue`
            // deliberately is not. One empty map per thread, for the life of
            // the thread.
            _ => {
                thread_local! {
                    static EMPTY: &'static Tags = Box::leak(Box::new(Tags::new()));
                }
                Cow::Borrowed(EMPTY.with(|e| *e))
            }
        }
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;

    /// Every string here is copied verbatim from
    /// `tests/golden/probes/payload-arms.golden`. This is criterion D3 made
    /// checkable: the target is the normalised text the goldens hold, not the
    /// reference's raw output.
    fn on_main(v: BundValue) -> BundValue {
        // What `TS::push` does to every value it accepts
        // (`reference/rust_multistack/src/ts_push.rs:25`).
        let boxed = v.promote();
        let BundValue::Heap(h) = &boxed else {
            unreachable!()
        };
        let mut tags = h.tags.clone();
        tags.insert(Rc::from("stack"), Rc::from("main"));
        BundValue::Heap(Rc::new(HeapValue {
            identity: Cell::new(h.identity.get()),
            stamp: Cell::new(h.stamp.get()),
            dt: h.dt,
            q: h.q,
            curr: h.curr,
            tags,
            attr: h.attr.clone(),
            payload: Rc::clone(&h.payload),
        }))
    }

    #[test]
    fn scalars_render_as_the_goldens_hold_them() {
        for (v, want) in [
            (
                BundValue::int(42),
                r#"Value { id: "<id>", stamp: <stamp>, dt: 2, q: 100.0, data: I64(42), attr: [], curr: -1, tags: {"stack": "main"} }"#,
            ),
            (
                BundValue::boolean(true),
                r#"Value { id: "<id>", stamp: <stamp>, dt: 1, q: 100.0, data: Bool(true), attr: [], curr: -1, tags: {"stack": "main"} }"#,
            ),
            (
                BundValue::nodata(),
                r#"Value { id: "<id>", stamp: <stamp>, dt: 97, q: 100.0, data: Null, attr: [], curr: -1, tags: {"stack": "main"} }"#,
            ),
        ] {
            assert_eq!(on_main(v).render(true), want);
        }
    }

    #[test]
    fn a_string_and_an_empty_list_render_as_the_goldens_hold_them() {
        assert_eq!(
            on_main(BundValue::str("s")).render(true),
            r#"Value { id: "<id>", stamp: <stamp>, dt: 4, q: 100.0, data: String("s"), attr: [], curr: -1, tags: {"stack": "main"} }"#
        );
        assert_eq!(
            on_main(BundValue::list(vec![])).render(true),
            r#"Value { id: "<id>", stamp: <stamp>, dt: 9, q: 100.0, data: List([]), attr: [], curr: -1, tags: {"stack": "main"} }"#
        );
    }

    /// D5: 21 characters over nanoid's alphabet.
    #[test]
    fn an_id_is_a_21_character_nanoid_shaped_string() {
        let v = BundValue::list(vec![]);
        let (id, _) = v.id_string();
        assert_eq!(id.chars().count(), ID_LEN);
        assert!(
            id.bytes().all(|b| ALPHABET.contains(&b)),
            "id {id} left the alphabet"
        );
    }

    /// Two values minted in one process never collide.
    #[test]
    fn ids_do_not_collide() {
        let ids: std::collections::HashSet<String> = (0..1000)
            .map(|_| BundValue::list(vec![]).id_string().0)
            .collect();
        assert_eq!(ids.len(), 1000);
    }

    /// Rendering must not mint an **identity**, or D2's laziness would be
    /// unobservable in the goldens — every capture would mint everything.
    /// It may stay lazy because `format_id(0)` has a real nanoid's width.
    #[test]
    fn rendering_does_not_mint_an_identity() {
        let v = BundValue::list(vec![]);
        let BundValue::Heap(h) = &v else {
            unreachable!()
        };
        let _ = v.render(false);
        assert_eq!(h.identity.get(), 0, "render must not mint");
    }

    /// The **stamp** is the opposite case: it materialises, because its
    /// rendered width differs from an unmaterialised one's and that width is
    /// baked into the box the goldens captured.
    #[test]
    fn rendering_materialises_the_stamp() {
        let v = BundValue::list(vec![]);
        let BundValue::Heap(h) = &v else {
            unreachable!()
        };
        assert_eq!(h.stamp.get(), 0.0, "lazy until needed");
        let first = v.render(false);
        let stamped = h.stamp.get();
        assert_ne!(stamped, 0.0, "printing is a need");
        assert_eq!(v.render(false), first, "and the sample is taken once");
    }

    /// The reason the two differ: a real stamp is fifteen characters wide and
    /// `0.0` is three, so leaving it lazy would narrow every box drawn round
    /// it. Rendering to a real width is the property conformance rests on.
    #[test]
    fn a_rendered_stamp_has_a_real_stamps_width() {
        let rendered = BundValue::list(vec![]).render(false);
        let stamp = rendered
            .split("stamp: ")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .unwrap();
        assert_eq!(
            stamp.len(),
            format!("{:?}", now_ms()).len(),
            "rendered {stamp} must be as wide as a freshly sampled stamp"
        );
    }

    /// The compact form shows the value and nothing else. The `Debug` form is
    /// ~150 columns for a single integer, which is right for a golden and
    /// wrong for an error report.
    #[test]
    fn a_summary_is_the_value_not_the_header() {
        assert_eq!(BundValue::int(42).summary(80), "42");
        assert_eq!(BundValue::str("hi").summary(80), "\"hi\"");
        assert_eq!(BundValue::boolean(true).summary(80), "true");
        assert_eq!(BundValue::nodata().summary(80), "nodata");
        // The same value's raw rendering is an order of magnitude longer.
        assert!(BundValue::int(42).render(false).len() > 100);
    }

    /// A `Str` payload means different things under different tags, and a
    /// summary that hid the difference would hide the one that matters most
    /// when something has gone wrong.
    #[test]
    fn a_summary_distinguishes_what_a_string_payload_means() {
        assert_eq!(BundValue::call("println").summary(80), "println");
        assert_eq!(BundValue::ptr("dup").summary(80), "`dup");
        assert_eq!(BundValue::named_context("main").summary(80), "@main");
        assert_eq!(BundValue::str("main").summary(80), "\"main\"");
    }

    /// Containers lead with kind and size: on a failure path "a list of 900"
    /// is the useful fact and the 900 elements are not.
    #[test]
    fn a_container_leads_with_its_size() {
        let big = BundValue::list((0..900).map(BundValue::int).collect());
        let s = big.summary(80);
        assert!(s.starts_with("list/900"), "{s}");
        assert!(s.contains('…'), "elided: {s}");
        assert_eq!(BundValue::lambda(vec![BundValue::int(1)]).summary(80), "lambda/1");
    }

    /// **The width is a hard bound.** A row that can overflow a terminal is
    /// the thing this exists to prevent, so the bound holds whatever the
    /// recursion produced.
    #[test]
    fn a_summary_never_exceeds_its_width() {
        let nested = BundValue::list(vec![
            BundValue::str("x".repeat(500)),
            BundValue::list((0..50).map(BundValue::int).collect()),
        ]);
        for w in [8usize, 20, 40, 80] {
            let s = nested.summary(w);
            assert!(s.chars().count() <= w, "width {w}: {} chars", s.chars().count());
        }
        assert!(BundValue::str("y".repeat(300)).summary(30).chars().count() <= 30);
    }

    /// A boxed scalar summarises as the scalar. Everything that reaches a
    /// stack is boxed, so without this every row would read `map/0`.
    #[test]
    fn a_boxed_scalar_summarises_as_its_value() {
        let boxed = BundValue::int(7).promote();
        assert_eq!(boxed.summary(80), "7");
    }

    /// D30's rendering half: the container hashes, the renderer orders.
    #[test]
    fn a_valuemap_renders_in_a_deterministic_order() {
        let mut m = HashMap::new();
        m.insert(BundValue::int(2), BundValue::int(20));
        m.insert(BundValue::int(1), BundValue::int(10));
        m.insert(BundValue::int(3), BundValue::int(30));
        let once = BundValue::valuemap(m.clone()).render(true);
        let twice = BundValue::valuemap(m).render(true);
        assert_eq!(once, twice);
        assert!(once.find("I64(1)").unwrap() < once.find("I64(2)").unwrap());
    }
}

// ---------------------------------------------------------------------------
// The three mutation classes (RFC-0001)
// ---------------------------------------------------------------------------

impl Clone for HeapValue {
    fn clone(&self) -> Self {
        Self {
            identity: Cell::new(self.identity.get()),
            stamp: Cell::new(self.stamp.get()),
            dt: self.dt,
            q: self.q,
            curr: self.curr,
            tags: self.tags.clone(),
            attr: self.attr.clone(),
            payload: Rc::clone(&self.payload),
        }
    }
}

impl BundValue {
    /// **Class 1 — rebuild.** What `set` on a map and `push` do: the result
    /// goes through a constructor, so it carries a fresh identity, a fresh
    /// stamp, an empty `attr`, `curr` at `-1`, empty `tags`, and `q` back at
    /// 100.0 (`reference/rust_dynamic/src/create_map.rs:33-40`). The receiver
    /// is untouched. This is F34, and it is why `Rc::make_mut` alone is the
    /// wrong model: `make_mut` *copies* the header where the reference
    /// discards it.
    pub fn rebuilt(&self, dt: u16, payload: Payload) -> Self {
        let _ = self;
        BundValue::heap(dt, payload)
    }

    /// **Class 2 — regenerate in place.** What `attr_add` does, and it is its
    /// own class: `self.dup().regen_id()` then a push onto the result's `attr`
    /// (`reference/rust_dynamic/src/attr.rs:19-20`), where `regen_id` writes a
    /// fresh id **and** stamp (`reference/rust_dynamic/src/id.rs:6-7`). So it
    /// mints like a rebuild but **preserves** `attr`, `curr` and `tags`.
    ///
    /// Confirmed against the oracle: `1 2 attribute 3 attribute` renders two
    /// entries with tags intact. A two-class partition filed this under
    /// rebuild, which would have emptied the `attr` the word exists to fill.
    pub fn attr_added(&self, value: BundValue) -> Self {
        let h = self.clone().into_heap();
        let mut next = (*h).clone();
        next.identity.set(mint());
        next.stamp.set(now_ms());
        next.attr.push(value);
        BundValue::Heap(Rc::new(next))
    }

    /// **Class 3 — minting-free.** What `set_tag` does
    /// (`reference/rust_dynamic/src/tags.rs:5`), and what `TS::push` runs on
    /// every push. It mutates in place, leaving id and stamp alone.
    ///
    /// Under clone-on-write this is a **split**, and the split
    /// **materialises the identity before copying** — D13 requires one policy
    /// governing both the `Rc` and the identity slot, and an unminted slot
    /// copied as unminted would let the two halves mint different ids and
    /// flip `A == A.clone()` from true to false.
    /// Set one tag, splitting only if the header is actually shared — **D13,
    /// as written**.
    ///
    /// D13's rule is that *a CoW split materialises the identity before it
    /// copies*, because two halves of a split would otherwise mint
    /// independently and `A == A.clone()` would silently become false. The rule
    /// is about a split. A uniquely owned header has no second half, so there
    /// is nothing to protect and nothing to mint.
    ///
    /// **This used to take `&self` and clone unconditionally**, which made the
    /// fast path unreachable by construction: `self.clone().into_heap()` bumps
    /// the count before asking whether anyone else holds it, so the answer was
    /// always "shared". Taking `self` lets a freshly boxed scalar — every
    /// literal an arithmetic word touches — take the in-place branch.
    ///
    /// The earlier comment argued the mint must not depend on a refcount, "or
    /// the identity a value ends up with would depend on how many clones
    /// existed at the time". That is true of the concrete number and false of
    /// every relation built on it: ids are opaque and unstable across runs by
    /// construction (the reference mints a nanoid), no golden pins one — F14
    /// normalises `id` — and the property D13 actually names, that two halves
    /// of a split agree, is preserved exactly because the mint happens on the
    /// branch where a split occurs.
    ///
    /// This is the hottest path in the interpreter: `Vm::push` tags every value
    /// it stores (`crates/bund2-interp/src/lib.rs`).
    pub fn with_tag(self, key: Rc<str>, value: Rc<str>) -> Self {
        let mut h = self.into_heap();
        match Rc::get_mut(&mut h) {
            Some(only) => {
                only.tags.insert(key, value);
            }
            None => {
                // A real split: materialise first, so both halves carry one id.
                let _ = h.identity();
                let mut next = (*h).clone();
                next.tags.insert(key, value);
                h = Rc::new(next);
            }
        }
        BundValue::Heap(h)
    }
}

/// Wall-clock nanoseconds, matching `timestamp_ns`
/// (`reference/rust_dynamic/src/value.rs:11-13`). Used by [`Metric::new`] and
/// by nothing else — a `Value`'s own stamp is milliseconds.
fn now_ns() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Wall-clock milliseconds, matching `timestamp_ms`
/// (`reference/rust_dynamic/src/value.rs:7-9`).
fn now_ms() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod mutation_tests {
    use super::*;

    /// Class 1 empties the header. Class 2 preserves it. A two-class
    /// partition has nowhere to put the second, and an earlier RFC draft
    /// filed it under the first.
    #[test]
    fn rebuild_empties_the_header_and_attr_add_preserves_it() {
        let tagged = BundValue::int(1).with_tag(Rc::from("stack"), Rc::from("main"));
        let with_attr = tagged.attr_added(BundValue::int(2));
        assert_eq!(with_attr.attr().len(), 1);
        assert_eq!(
            with_attr.tags().get("stack").map(|s| &**s),
            Some("main"),
            "attr_add must preserve tags"
        );

        let rebuilt = tagged.rebuilt(MAP, Payload::Map(BTreeMap::new()));
        assert!(rebuilt.tags().is_empty(), "a rebuild empties tags");
        assert!(rebuilt.attr().is_empty());
        assert_eq!(rebuilt.q(), 100.0, "a rebuild resets q");
    }

    /// The oracle case: `1 2 attribute 3 attribute` renders two entries.
    #[test]
    fn attr_add_accumulates() {
        let v = BundValue::int(1)
            .attr_added(BundValue::int(2))
            .attr_added(BundValue::int(3));
        assert_eq!(v.attr().len(), 2);
    }

    /// Class 2 mints; class 3 does not.
    #[test]
    fn attr_add_mints_and_set_tag_does_not() {
        let a = BundValue::list(vec![]).promote();
        let (before, _) = a.identity();
        assert_ne!(a.attr_added(BundValue::int(1)).identity().0, before);
        assert_eq!(a.with_tag(Rc::from("k"), Rc::from("v")).identity().0, before);
    }

    /// D13's condition: the split must not let two halves mint separately.
    /// Without materialising first, `a` and `b` would end up with different
    /// ids and `A == A.clone()` would silently become false.
    #[test]
    fn a_minting_free_split_keeps_both_halves_on_one_identity() {
        let a = BundValue::list(vec![]);
        let b = a.clone();
        assert_eq!(a, b, "clone-equal before the split");
        let split = b.with_tag(Rc::from("stack"), Rc::from("main"));
        assert_eq!(
            a.identity().0,
            split.identity().0,
            "the split must carry the identity across"
        );
    }
}

#[cfg(test)]
mod arm_tests {
    use super::*;

    /// Six more `dt` values reached with no new payload arm — the independent
    /// axes paying for themselves.
    #[test]
    fn one_payload_serves_several_tags() {
        assert_eq!(
            BundValue::pair(BundValue::int(1), BundValue::int(2)).dt(),
            PAIR
        );
        assert_eq!(BundValue::complex_float(1.0, 2.0).dt(), CFLOAT);
        assert_eq!(BundValue::textbuffer("x").dt(), TEXTBUFFER);
        assert_eq!(BundValue::conditional(BTreeMap::new()).dt(), CONDITIONAL);
        assert_eq!(BundValue::class(BTreeMap::new()).dt(), CLASS);
        assert_eq!(BundValue::object(BTreeMap::new()).dt(), OBJECT);
    }

    /// A `PAIR` and a `LIST` share the `List` payload, so they render the same
    /// `data:` and differ only in `dt` — which is exactly the reference's
    /// behaviour and the reason `dt` cannot be derived from the payload.
    #[test]
    fn pair_and_list_differ_only_by_tag() {
        let p = BundValue::pair(BundValue::int(1), BundValue::int(2));
        let l = BundValue::list(vec![BundValue::int(1), BundValue::int(2)]);
        assert_ne!(p.dt(), l.dt());
        let (rp, rl) = (p.render(true), l.render(true));
        assert_eq!(
            rp.replace("dt: 10", "dt: 9"),
            rl,
            "PAIR and LIST must differ only in dt"
        );
    }

    /// D7: sampled on first observation, and stable after.
    #[test]
    fn timestamp_is_sampled_once() {
        let v = BundValue::list(vec![]);
        let (a, _) = v.timestamp();
        let (b, _) = v.timestamp();
        assert_eq!(a, b, "the stamp must not resample");
        assert!(a > 0.0);
    }

    /// D7's deviation, asserted rather than worked around: the stamp orders by
    /// observation, so a value constructed first can carry the later stamp.
    #[test]
    fn stamps_order_by_observation_not_construction() {
        let first_built = BundValue::list(vec![]);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second_built = BundValue::list(vec![]);
        let (second_stamp, _) = second_built.timestamp();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let (first_stamp, _) = first_built.timestamp();
        assert!(
            first_stamp >= second_stamp,
            "the value built first was observed second and must carry the later stamp"
        );
    }

    /// A scalar has nowhere to keep a sample, so observing promotes.
    #[test]
    fn observing_a_scalar_stamp_promotes_it() {
        let (_, promoted) = BundValue::int(1).timestamp();
        assert!(promoted.expect("must promote").is_boxed());
    }
}

#[cfg(test)]
mod q_tests {
    use super::*;

    /// The case Q18 closed on, and the one `cargo xtask render` found
    /// unrepresentable: a value must be able to hold a `q` other than 100.0.
    #[test]
    fn q_can_hold_the_json_null_value() {
        let v = BundValue::none().with_q(0.0);
        assert_eq!(v.q(), 0.0);
        assert!(v.render(true).contains("q: 0.0"));
    }
}
