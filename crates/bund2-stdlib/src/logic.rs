//! The boolean constructors and the six comparison words.
//!
//! `true` and `false` are **words**, not literals. The grammar has no boolean
//! alternative; both are registered inlines that push a `Value::from_bool`
//! (`reference/rust_multistackvm/src/stdlib/artefacts.rs:101-107,137-138`).
//!
//! The six comparisons all funnel through one gate,
//! `stdlib_logic_compare`, whose shape is preserved here in full
//! (`reference/rust_multistackvm/src/stdlib/logic/logic_compare_fun.rs:16-71`).
//! Three of its properties look like mistakes and are not treated as such:
//!
//! - **The operands are reversed.** `value1` is pulled first, so it is the
//!   *top* of the stack, and the comparison computed is `value1 OP value2` —
//!   top against the one below it (`:77-88,143`). `3 5 <` therefore asks
//!   whether `5 < 3` and answers false. Confirmed against the oracle.
//! - **`BOOL` is not comparable.** The gate admits five numeric tags and
//!   `STRING`; every other tag, booleans included, fails operand #1 (`:67-69`).
//!   `true true ==` is an error in this language.
//! - **Strings order by nothing.** A `STRING` operand accepts only `Eq` and
//!   `Ne`; `<` on two strings is an error (`:57-59`), even though the
//!   underlying `Ord` would happily compare them
//!   (`reference/rust_dynamic/src/ord.rs:186-193`).
//!
//! Two behaviours **do** change, each on a recorded disposition — see
//! [`numeric_eq`] for D30's exact equality, and [`numeric_ord`] for the
//! mixed-type ordering defect that is preserved rather than fixed.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, CFLOAT, CINTEGER, FLOAT, INTEGER, STRING, TIME};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// The reference's `Ops`, with `Le`/`Leq` renamed to what they spell.
///
/// The reference calls `<` "Le" and `<=` "Leq"
/// (`reference/rust_multistackvm/src/stdlib/logic/logic_compare_fun.rs:6-13`,
/// registered at `:239,241`). Copying the names would carry a trap forwards
/// for no gain; the registered spellings are what this preserves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Op {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

impl Op {
    /// The word this is registered under, which is also what its error
    /// messages name it.
    fn word(self) -> &'static str {
        match self {
            Op::Eq => "==",
            Op::Ne => "!=",
            Op::Gt => ">",
            Op::Lt => "<",
            Op::Ge => ">=",
            Op::Le => "<=",
        }
    }

    fn is_equality(self) -> bool {
        matches!(self, Op::Eq | Op::Ne)
    }
}

/// A tag the gate accepts as a number.
///
/// This tests the **`dt` tag**, not the payload, because the reference's gate
/// calls `type_of()` and that returns `dt` verbatim
/// (`reference/rust_dynamic/src/value_types.rs:5-7`). In Bund2 the tag and the
/// payload are independent axes, so the distinction is real: the gate decides
/// what may be compared, and the payload decides how.
fn is_numeric_tag(dt: u16) -> bool {
    matches!(dt, INTEGER | FLOAT | CINTEGER | CFLOAT | TIME)
}

/// **D30's exact equality**, and a deviation from the reference in both
/// directions.
///
/// The reference is asymmetric: an `I64` receiver truncates the float it is
/// compared against (`reference/rust_dynamic/src/eq.rs:13`) while an `F64`
/// receiver widens the int (`:24`). So the oracle answers `42 42.5 ==` false
/// and `42.5 42 ==` true — the same two values, opposite answers, depending
/// on which reached the top of the stack. `impl Eq` asserts a symmetry that
/// is not there (`:60-62`).
///
/// D30's amendment resolves this to exact numeric comparison: an integer and
/// a float are equal when they denote the same mathematical value.
///
/// ```text
/// i == f  ⟺  f is finite and integral, f is within i64 range, and f as i64 == i
/// ```
///
/// `42 == 42.0` stays true in both orientations, `42 == 42.5` becomes false in
/// both, and `2^53+1` against `2^53.0` is false in both. The disposition is
/// F33, an original-implementation bug.
///
/// Floats compare by **IEEE** here, which is where the word parts company with
/// the key equality `BundValue` implements for `valuemap`: `NaN == NaN` is
/// false for the word and true for the key, because a key must be reflexive to
/// be findable. Both are correct; they answer different questions.
fn numeric_eq(a: &BundValue, b: &BundValue) -> bool {
    // Through the boxing `push` applies; see `BundValue::unboxed`.
    match (a.unboxed(), b.unboxed()) {
        (BundValue::Int(x, _), BundValue::Int(y, _)) => x == y,
        (BundValue::Float(x, _), BundValue::Float(y, _)) => x == y,
        (BundValue::Int(i, _), BundValue::Float(f, _)) | (BundValue::Float(f, _), BundValue::Int(i, _)) => {
            exact_int_float(*i, *f)
        }
        // Reached only if a value carries a numeric `dt` over a non-numeric
        // payload, which no word can currently produce — nothing sets `dt`
        // independently of the payload. The reference falls back to comparing
        // ids here (`reference/rust_dynamic/src/eq.rs:15,26,53`); Bund2's
        // identity is lazy, so comparing it would mint one and make equality
        // an observation. Content equality is the closest total relation that
        // does not, and D30 already put the two in mirror.
        _ => a == b,
    }
}

/// The one comparison D30's amendment names, spelled out.
///
/// `f as i64` saturates in Rust rather than wrapping, so the range test has to
/// come first or `1e30 as i64` would clamp to `i64::MAX` and compare equal to
/// it. `f.fract() == 0.0` rejects a non-integral float before the cast can
/// truncate it, which is exactly the reference behaviour D30 removes.
fn exact_int_float(i: i64, f: f64) -> bool {
    f.is_finite()
        && f.fract() == 0.0
        && f >= -(2f64.powi(63))
        && f < 2f64.powi(63)
        && (f as i64) == i
}

/// The ordering comparisons, **preserving a defect** recorded as F47.
///
/// `PartialOrd for Value` overrides `lt`, `le`, `gt` and `ge` individually,
/// and every one of them returns `true` when the two payloads are different
/// arms (`reference/rust_dynamic/src/ord.rs:16,24,55,63,94,102,133,141`). So
/// `1 2.0 <` is true and `1 2.0 >` is also true, along with `<=` and `>=`.
/// All four confirmed against the oracle.
///
/// It is preserved rather than fixed because fixing it is a deviation, and an
/// unplanned deviation is a decision. D30 settled equality; it did not reach
/// ordering, and extending it is carried as **D33** for the repository owner.
/// Preserving costs nothing that a later decision cannot undo — the goldens
/// pin the current answers either way.
///
/// Note that `Ord::cmp` is *not* consulted. Rust's `<` calls
/// `PartialOrd::lt`, and these overrides shadow the `partial_cmp` that would
/// otherwise delegate to `cmp` (`reference/rust_dynamic/src/ord.rs:6-8`), so
/// the sane-looking `cmp` at `:167-203` never runs for these words.
fn numeric_ord(op: Op, a: &BundValue, b: &BundValue) -> bool {
    match (a.unboxed(), b.unboxed()) {
        (BundValue::Int(x, _), BundValue::Int(y, _)) => match op {
            Op::Gt => x > y,
            Op::Lt => x < y,
            Op::Ge => x >= y,
            Op::Le => x <= y,
            // `compare` routes equality elsewhere, so this arm is not
            // reached. Answering false is a defensible result for an ordering
            // question that was never asked; aborting is not.
            _ => false,
        },
        (BundValue::Float(x, _), BundValue::Float(y, _)) => match op {
            Op::Gt => x > y,
            Op::Lt => x < y,
            Op::Ge => x >= y,
            Op::Le => x <= y,
            // `compare` routes equality elsewhere, so this arm is not
            // reached. Answering false is a defensible result for an ordering
            // question that was never asked; aborting is not.
            _ => false,
        },
        // F47: mismatched payload arms answer true to every operator.
        _ => true,
    }
}

/// The gate, preserving `stdlib_logic_compare`'s structure operand by operand
/// (`reference/rust_multistackvm/src/stdlib/logic/logic_compare_fun.rs:16-71`).
///
/// `v1` is the **top** of the stack. See the module note on operand order.
fn compare(op: Op, v1: &BundValue, v2: &BundValue) -> Result<bool, Error> {
    if is_numeric_tag(v1.dt()) {
        if !is_numeric_tag(v2.dt()) {
            return Err(Error("COMPARE: unsupported operand #2".into()));
        }
        return Ok(match op {
            Op::Eq => numeric_eq(v1, v2),
            Op::Ne => !numeric_eq(v1, v2),
            other => numeric_ord(other, v1, v2),
        });
    }
    if v1.dt() == STRING {
        if v2.dt() != STRING {
            return Err(Error("COMPARE: unsupported operand #2".into()));
        }
        if !op.is_equality() {
            return Err(Error("COMPARE: unsupported operation for string".into()));
        }
        let equal = v1.as_str() == v2.as_str();
        return Ok(if op == Op::Eq { equal } else { !equal });
    }
    Err(Error("COMPARE: unsupported operand #1".into()))
}

/// One comparison word.
///
/// The depth test comes before either pull and names the word, matching
/// `if vm.stack.current_stack_len() < 2 { bail!("Stack is too shallow for
/// inline ==") }`
/// (`reference/rust_multistackvm/src/stdlib/logic/logic_compare_fun.rs:74-76`),
/// and a gate failure is wrapped the same way it is at `:93-95`, giving
/// `== returns error: COMPARE: unsupported operand #1`.
///
/// The reference's `NO DATA #1` / `#2` branches are unreachable — the depth
/// test above them guarantees both pulls succeed — and two of them are
/// mislabelled `<` inside `>=` and `<=` (`:188,215`). Nothing is lost by not
/// reproducing a branch that cannot be entered.
fn run(op: Op, vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(format!(
            "Stack is too shallow for inline {}",
            op.word()
        )));
    }
    let v1 = crate::pull::operand(vm, op.word(), 1)?;
    let v2 = crate::pull::operand(vm, op.word(), 2)?;
    match compare(op, &v1, &v2) {
        Ok(res) => {
            vm.push(BundValue::boolean(res));
            Ok(())
        }
        Err(e) => Err(Error(format!("{} returns error: {}", op.word(), e.0))),
    }
}

fn eq(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Eq, vm)
}
fn ne(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Ne, vm)
}
fn gt(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Gt, vm)
}
fn lt(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Lt, vm)
}
fn ge(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Ge, vm)
}
fn le(vm: &mut dyn Vm) -> Result<(), Error> {
    run(Op::Le, vm)
}

/// `true` and `false`.
///
/// The reference routes these through `vm.apply`, which pushes unless
/// `autoadd` is set, in which case it appends into the value on top instead
/// (`reference/rust_multistackvm/src/multistackvm_apply.rs:88-101`). `autoadd`
/// is list-construction mode and no word in Bund2 sets it yet, so this pushes.
/// When list construction lands, it goes through `apply`, not through here.
fn bool_true(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::boolean(true));
    Ok(())
}

fn bool_false(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::boolean(false));
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.register_native("true", bool_true, eff(0, 1), WordKind::Sync);
    r.register_native("false", bool_false, eff(0, 1), WordKind::Sync);
    r.register_native("==", eq, eff(2, 1), WordKind::Sync);
    r.register_native("!=", ne, eff(2, 1), WordKind::Sync);
    r.register_native(">", gt, eff(2, 1), WordKind::Sync);
    r.register_native("<", lt, eff(2, 1), WordKind::Sync);
    r.register_native(">=", ge, eff(2, 1), WordKind::Sync);
    r.register_native("<=", le, eff(2, 1), WordKind::Sync);
}

/// The six comparisons, in the order
/// `reference/rust_multistackvm/src/stdlib/logic/logic_compare_fun.rs:236-241`
/// registers them.
pub const COMPARE_WORDS: &[&str] = &["==", "!=", ">", "<", ">=", "<="];

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_api::Resolved;
    use bund2_interp::Interp;

    fn f(x: f64) -> BundValue {
        BundValue::float(x)
    }
    fn i(x: i64) -> BundValue {
        BundValue::int(x)
    }

    /// `3 5 <` asks whether `5 < 3`. The top operand is on the left of the
    /// comparison, which is the opposite of what the source text reads like.
    #[test]
    fn the_top_of_the_stack_is_the_left_operand() {
        // `3 5 <` — 5 is on top, so v1 = 5.
        assert!(!compare(Op::Lt, &i(5), &i(3)).unwrap(), "5 < 3 is false");
        assert!(compare(Op::Gt, &i(5), &i(3)).unwrap(), "5 > 3 is true");
    }

    /// D30: equal in both orientations, where the reference disagreed with
    /// itself. `42.5 42 ==` answered true in the oracle; F33 says that was a
    /// bug and this is the fix.
    #[test]
    fn exact_equality_is_symmetric() {
        assert!(compare(Op::Eq, &i(42), &f(42.0)).unwrap());
        assert!(compare(Op::Eq, &f(42.0), &i(42)).unwrap());
        assert!(!compare(Op::Eq, &i(42), &f(42.5)).unwrap());
        assert!(
            !compare(Op::Eq, &f(42.5), &i(42)).unwrap(),
            "the reference answered true here; D30 makes it false"
        );
    }

    /// The transitivity D30 was after: truncation made `42 == 42.5` and
    /// `42 == 42.9` both true while `42.5 == 42.9` was false.
    #[test]
    fn exact_equality_is_transitive() {
        assert!(!compare(Op::Eq, &i(42), &f(42.5)).unwrap());
        assert!(!compare(Op::Eq, &i(42), &f(42.9)).unwrap());
    }

    /// Above 2^53 a widening comparison loses the low bit and two distinct
    /// integers collapse onto one float. Exact comparison does not.
    #[test]
    fn exact_equality_holds_past_the_float_mantissa() {
        let big = (1i64 << 53) + 1;
        assert!(!compare(Op::Eq, &i(big), &f(9007199254740992.0)).unwrap());
        assert!(!compare(Op::Eq, &f(9007199254740992.0), &i(big)).unwrap());
        assert!(compare(Op::Eq, &i(1 << 53), &f(9007199254740992.0)).unwrap());
    }

    /// `f as i64` saturates rather than wrapping, so a float far outside i64
    /// range must be rejected before the cast, not after it.
    #[test]
    fn a_float_out_of_range_is_not_equal_to_i64_max() {
        assert!(!compare(Op::Eq, &i(i64::MAX), &f(1e30)).unwrap());
        assert!(!compare(Op::Eq, &i(i64::MIN), &f(-1e30)).unwrap());
    }

    /// IEEE for the word: `NaN` equals nothing, itself included. The key
    /// equality `valuemap` uses answers the opposite, deliberately.
    #[test]
    fn nan_is_equal_to_nothing() {
        assert!(!compare(Op::Eq, &f(f64::NAN), &f(f64::NAN)).unwrap());
        assert!(compare(Op::Ne, &f(f64::NAN), &f(f64::NAN)).unwrap());
        assert!(!compare(Op::Eq, &f(f64::NAN), &i(0)).unwrap());
    }

    /// F47, preserved: mixed payload arms answer true to all four orderings,
    /// including the two that cannot both be true.
    #[test]
    fn mixed_number_kinds_answer_true_to_every_ordering() {
        for op in [Op::Lt, Op::Gt, Op::Le, Op::Ge] {
            assert!(
                compare(op, &f(2.0), &i(1)).unwrap(),
                "F47: {op:?} on mixed kinds is true in the reference"
            );
        }
    }

    /// Strings compare by content, and only for equality.
    #[test]
    fn strings_compare_by_content_but_do_not_order() {
        let a = BundValue::str("a");
        let a2 = BundValue::str("a");
        let b = BundValue::str("b");
        assert!(compare(Op::Eq, &a, &a2).unwrap(), "distinct but equal");
        assert!(!compare(Op::Eq, &a, &b).unwrap());
        assert!(compare(Op::Ne, &a, &b).unwrap());
        assert_eq!(
            compare(Op::Lt, &a, &b).unwrap_err().0,
            "COMPARE: unsupported operation for string"
        );
    }

    /// Booleans are not comparable at all — the gate admits five numeric tags
    /// and `STRING`, and `BOOL` is neither.
    #[test]
    fn booleans_are_not_comparable() {
        assert_eq!(
            compare(Op::Eq, &BundValue::boolean(true), &BundValue::boolean(true))
                .unwrap_err()
                .0,
            "COMPARE: unsupported operand #1"
        );
    }

    /// A number against a string fails on operand #2, not operand #1 — the
    /// gate checks the top first.
    #[test]
    fn a_mismatched_second_operand_is_named_as_such() {
        assert_eq!(
            compare(Op::Eq, &i(1), &BundValue::str("a")).unwrap_err().0,
            "COMPARE: unsupported operand #2"
        );
        assert_eq!(
            compare(Op::Eq, &BundValue::str("a"), &i(1)).unwrap_err().0,
            "COMPARE: unsupported operand #2"
        );
    }

    /// Every comparison word is registered, and under the spelling the
    /// reference registers rather than the name of its `Ops` variant.
    #[test]
    fn all_six_are_registered() {
        let mut i = Interp::new();
        register(&mut i.registry);
        assert_eq!(COMPARE_WORDS.len(), 6);
        for name in COMPARE_WORDS.iter().chain(["true", "false"].iter()) {
            let (s, sigil) = i
                .registry
                .interner
                .lookup_call(name)
                .unwrap_or_else(|| panic!("{name} is not registered"));
            assert_ne!(
                i.registry.resolve(s, sigil),
                Resolved::Unbound,
                "resolve failed for {name}"
            );
        }
    }

    /// The depth test fires before either pull, so a one-deep stack keeps its
    /// value rather than losing it to a half-completed comparison.
    #[test]
    fn a_shallow_stack_is_an_error_that_consumes_nothing() {
        let mut i = Interp::new();
        register(&mut i.registry);
        i.push(BundValue::int(1));
        assert_eq!(
            run(Op::Eq, &mut i).unwrap_err().0,
            "Stack is too shallow for inline =="
        );
        assert_eq!(i.depth(), 1, "the operand is still there");
    }

    /// A gate failure is reported as the word wrapping the gate's own
    /// message, which is the shape the reference's `bail!` produces.
    #[test]
    fn a_gate_failure_names_the_word_and_the_reason() {
        let mut i = Interp::new();
        register(&mut i.registry);
        i.push(BundValue::boolean(true));
        i.push(BundValue::boolean(false));
        assert_eq!(
            run(Op::Eq, &mut i).unwrap_err().0,
            "== returns error: COMPARE: unsupported operand #1"
        );
    }

    /// End to end through the stack, with the operands where a program would
    /// leave them: `3 5 <` is false.
    #[test]
    fn comparison_through_the_stack_pushes_a_bool() {
        let mut i = Interp::new();
        register(&mut i.registry);
        i.push(BundValue::int(3));
        i.push(BundValue::int(5));
        run(Op::Lt, &mut i).unwrap();
        assert_eq!(i.depth(), 1, "two consumed, one produced");
        // `push` boxes it, so the result is a `Heap` carrying a `Scalar`, not
        // an inline `Bool` — which is exactly the boxing `unboxed` looks
        // through, and exactly what a naive match on the outer value misses.
        let top = i.peek().unwrap();
        assert_eq!(top.dt(), bund2_value::BOOL);
        assert_eq!(*top.unboxed(), BundValue::boolean(false), "3 5 < is false");
    }
}
