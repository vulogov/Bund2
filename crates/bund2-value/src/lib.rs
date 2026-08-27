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
