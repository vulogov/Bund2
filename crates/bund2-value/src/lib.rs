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

#![deny(unsafe_op_in_unsafe_fn)]

pub mod wire;

use std::cell::Cell;
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
    pub const VALUEMAP: u16 = 30;
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
    let mut out = [ALPHABET[0]; ID_LEN];
    let mut x = n;
    for slot in out.iter_mut() {
        *slot = ALPHABET[(x % 64) as usize];
        x /= 64;
    }
    String::from_utf8(out.to_vec()).expect("alphabet is ASCII")
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
    /// A boxed scalar. Scalars are inline until they acquire a header, and
    /// `TS::push` tags unconditionally, so anything pushed to a stack is boxed.
    Scalar(BundValue),
}

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
    /// Averaged by arithmetic, not a constant — `calc_q` at
    /// `reference/rust_dynamic/src/q.rs:5`. The goldens all show 100.0 because
    /// that is a fixpoint; `Value::none` starts at 0.0.
    q: f64,
    /// The iteration cursor. A plain field, not a `Cell`: a `Cell` inside the
    /// shared `Rc` would give clones a *shared* cursor.
    curr: i32,
    /// Written on every push (`reference/rust_multistack/src/ts_push.rs:25`).
    tags: BTreeMap<String, String>,
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
            tags: BTreeMap::new(),
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
            Payload::Exit => "Exit",
            Payload::Scalar(_) => unreachable!("a boxed scalar renders as its inner value"),
        }
    }
}

/// The runtime value. **Two words: 16 bytes, 8-aligned.**
#[derive(Debug, Clone)]
pub enum BundValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    /// `dt` 97.
    Nodata,
    /// `dt` 0. Distinct from `Nodata`: `Val::Null` carries both tags.
    None,
    Heap(Rc<HeapValue>),
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

    fn heap(dt: u16, p: Payload) -> Self {
        BundValue::Heap(Rc::new(HeapValue::new(dt, p)))
    }

    /// The `dt` tag.
    pub fn dt(&self) -> u16 {
        match self {
            BundValue::Int(_) => INTEGER,
            BundValue::Float(_) => FLOAT,
            BundValue::Bool(_) => BOOL,
            BundValue::Nodata => NODATA,
            BundValue::None => NONE,
            BundValue::Heap(h) => h.dt,
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
    pub fn identity(&self) -> (u64, Option<BundValue>) {
        match self {
            BundValue::Heap(h) => (h.identity(), None),
            scalar => {
                let boxed = scalar.clone().promote();
                let BundValue::Heap(h) = &boxed else {
                    unreachable!("promote always yields a heap value")
                };
                (h.identity(), Some(boxed))
            }
        }
    }

    /// Box a scalar so it can carry a header. A heap value is returned as is.
    pub fn promote(self) -> Self {
        match self {
            BundValue::Heap(_) => self,
            scalar => {
                let dt = scalar.dt();
                BundValue::heap(dt, Payload::Scalar(scalar))
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
            (BundValue::Float(a), BundValue::Float(b)) => a == b,
            (BundValue::Float(a), BundValue::Int(b)) => int_eq_float(*b, *a),
            (BundValue::Int(a), BundValue::Float(b)) => int_eq_float(*a, *b),
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
            (Int(a), Int(b)) => a == b,
            (Float(a), Float(b)) => float_key(*a) == float_key(*b),
            (Bool(a), Bool(b)) => a == b,
            (Nodata, Nodata) | (None, None) => true,
            (Int(a), Float(b)) | (Float(b), Int(a)) => int_eq_float(*a, *b),
            // A boxed scalar compares as the scalar it boxes, so boxing is
            // invisible here.
            (Heap(h), other) | (other, Heap(h)) if matches!(*h.payload, Payload::Scalar(_)) => {
                let Payload::Scalar(inner) = &*h.payload else {
                    unreachable!()
                };
                inner == other
            }
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
            Int(i) => i.hash(state),
            Float(f) => {
                if f.fract() == 0.0 && f.is_finite() && *f >= -(2f64.powi(63)) && *f < 2f64.powi(63)
                {
                    (*f as i64).hash(state)
                } else {
                    float_key(*f).hash(state)
                }
            }
            Bool(b) => b.hash(state),
            Nodata => 0x4E4F_4441u64.hash(state), // "NODA"
            None => 0u64.hash(state),
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
            BundValue::list(vec![BundValue::Int(1)]),
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
        assert_eq!(BundValue::Bool(true), BundValue::Bool(true));
        assert_eq!(
            BundValue::Bool(true).promote(),
            BundValue::Bool(true),
            "boxing must not change equality"
        );
        assert_eq!(
            BundValue::Bool(true).promote(),
            BundValue::Bool(true).promote()
        );
    }

    /// D30's amendment: exact, and therefore symmetric and transitive.
    /// Truncation fails at 42/42.5/42.9; widening fails above 2^53.
    #[test]
    fn int_float_equality_is_exact_in_both_orientations() {
        let (i, f_eq, f_ne) = (
            BundValue::Int(42),
            BundValue::Float(42.0),
            BundValue::Float(42.5),
        );
        assert_eq!(i, f_eq);
        assert_eq!(f_eq, i);
        assert_ne!(i, f_ne);
        assert_ne!(f_ne, i);

        // 2^53 + 1 is not representable as f64; widening would say equal.
        let big = BundValue::Int(9_007_199_254_740_993);
        let near = BundValue::Float(9_007_199_254_740_992.0);
        assert_ne!(big, near);
        assert_ne!(near, big);
    }

    /// Transitivity, which is what rules truncation out.
    #[test]
    fn equality_is_transitive_across_int_and_float() {
        let a = BundValue::Int(42);
        let b = BundValue::Float(42.5);
        let c = BundValue::Float(42.9);
        // Truncation would make a == b and a == c while b != c.
        assert!(!(a == b && a == c && b != c));
    }

    /// `Eq` needs reflexivity and `NaN` denies it under IEEE, so the key
    /// equality is total while the word's is not.
    #[test]
    fn nan_is_reflexive_as_a_key_and_not_as_a_word() {
        let nan = BundValue::Float(f64::NAN);
        assert_eq!(nan, nan.clone(), "Eq must be reflexive for HashMap");
        assert!(!nan.eq_ieee(&nan), "the word == keeps IEEE semantics");
    }

    #[test]
    fn negative_zero_and_zero_are_one_key() {
        assert_eq!(BundValue::Float(-0.0), BundValue::Float(0.0));
        assert_eq!(
            hash_of(&BundValue::Float(-0.0)),
            hash_of(&BundValue::Float(0.0))
        );
    }

    /// D30: equal values must hash alike, or the valuemap lookup misses.
    #[test]
    fn equal_values_hash_alike() {
        assert_eq!(
            hash_of(&BundValue::Int(42)),
            hash_of(&BundValue::Float(42.0))
        );
        assert_eq!(
            hash_of(&BundValue::Bool(true)),
            hash_of(&BundValue::Bool(true).promote())
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
        m.insert(BundValue::str("k"), BundValue::Int(42));
        assert_eq!(m.get(&BundValue::str("k")), Some(&BundValue::Int(42)));
        m.insert(BundValue::Int(7), BundValue::Int(1));
        assert_eq!(m.get(&BundValue::Int(7)), Some(&BundValue::Int(1)));
    }

    /// D30's stated limit: composite keys stay identity-keyed, because `eq`
    /// for a list is identity and changing that reaches past `valuemap`.
    #[test]
    fn a_valuemap_does_not_find_a_freshly_built_composite_key() {
        let mut m = HashMap::new();
        m.insert(BundValue::list(vec![BundValue::Int(1)]), BundValue::Int(9));
        assert_eq!(m.get(&BundValue::list(vec![BundValue::Int(1)])), None);
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
        let v = BundValue::Int(1);
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
        assert_ne!(BundValue::Nodata, BundValue::None);
        assert_eq!(BundValue::Nodata.dt(), NODATA);
        assert_eq!(BundValue::None.dt(), NONE);
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
            format!("{:?}", self.peek_stamp())
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
            BundValue::Int(i) => {
                let _ = write!(out, "I64({i})");
            }
            BundValue::Float(f) => {
                let _ = write!(out, "F64({f:?})");
            }
            BundValue::Bool(b) => {
                let _ = write!(out, "Bool({b})");
            }
            BundValue::Nodata | BundValue::None => out.push_str("Null"),
            BundValue::Heap(h) => match &*h.payload {
                Payload::Scalar(inner) => inner.render_payload(out, norm),
                Payload::Exit => out.push_str("Exit"),
                Payload::Str(x) => {
                    let _ = write!(out, "String({x:?})");
                }
                Payload::Bin(b) => {
                    let _ = write!(out, "Binary({b:?})");
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

    fn peek_stamp(&self) -> f64 {
        match self {
            BundValue::Heap(h) => h.stamp.get(),
            _ => 0.0,
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

    pub fn tags(&self) -> &BTreeMap<String, String> {
        static EMPTY: std::sync::OnceLock<BTreeMap<String, String>> = std::sync::OnceLock::new();
        match self {
            BundValue::Heap(h) => &h.tags,
            _ => EMPTY.get_or_init(BTreeMap::new),
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
        tags.insert("stack".into(), "main".into());
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
                BundValue::Int(42),
                r#"Value { id: "<id>", stamp: <stamp>, dt: 2, q: 100.0, data: I64(42), attr: [], curr: -1, tags: {"stack": "main"} }"#,
            ),
            (
                BundValue::Bool(true),
                r#"Value { id: "<id>", stamp: <stamp>, dt: 1, q: 100.0, data: Bool(true), attr: [], curr: -1, tags: {"stack": "main"} }"#,
            ),
            (
                BundValue::Nodata,
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

    /// Rendering must not be an observation, or D2's laziness would be
    /// unobservable in the goldens — every capture would mint everything.
    #[test]
    fn rendering_does_not_mint() {
        let v = BundValue::list(vec![]);
        let BundValue::Heap(h) = &v else {
            unreachable!()
        };
        let _ = v.render(false);
        assert_eq!(h.identity.get(), 0, "render must not mint");
    }

    /// D30's rendering half: the container hashes, the renderer orders.
    #[test]
    fn a_valuemap_renders_in_a_deterministic_order() {
        let mut m = HashMap::new();
        m.insert(BundValue::Int(2), BundValue::Int(20));
        m.insert(BundValue::Int(1), BundValue::Int(10));
        m.insert(BundValue::Int(3), BundValue::Int(30));
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
        let base = self.clone().promote();
        let BundValue::Heap(h) = &base else {
            unreachable!()
        };
        let mut next = (**h).clone();
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
    pub fn with_tag(&self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let base = self.clone().promote();
        let BundValue::Heap(h) = &base else {
            unreachable!()
        };
        // Materialise before the split, whether or not the split happens: the
        // rule must not depend on a refcount, or the identity a value ends up
        // with would depend on how many clones existed at the time.
        let _ = h.identity();
        let mut next = (**h).clone();
        next.tags.insert(key.into(), value.into());
        BundValue::Heap(Rc::new(next))
    }
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
        let tagged = BundValue::Int(1).with_tag("stack", "main");
        let with_attr = tagged.attr_added(BundValue::Int(2));
        assert_eq!(with_attr.attr().len(), 1);
        assert_eq!(
            with_attr.tags().get("stack").map(String::as_str),
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
        let v = BundValue::Int(1)
            .attr_added(BundValue::Int(2))
            .attr_added(BundValue::Int(3));
        assert_eq!(v.attr().len(), 2);
    }

    /// Class 2 mints; class 3 does not.
    #[test]
    fn attr_add_mints_and_set_tag_does_not() {
        let a = BundValue::list(vec![]).promote();
        let (before, _) = a.identity();
        assert_ne!(a.attr_added(BundValue::Int(1)).identity().0, before);
        assert_eq!(a.with_tag("k", "v").identity().0, before);
    }

    /// D13's condition: the split must not let two halves mint separately.
    /// Without materialising first, `a` and `b` would end up with different
    /// ids and `A == A.clone()` would silently become false.
    #[test]
    fn a_minting_free_split_keeps_both_halves_on_one_identity() {
        let a = BundValue::list(vec![]);
        let b = a.clone();
        assert_eq!(a, b, "clone-equal before the split");
        let split = b.with_tag("stack", "main");
        assert_eq!(
            a.identity().0,
            split.identity().0,
            "the split must carry the identity across"
        );
    }
}
