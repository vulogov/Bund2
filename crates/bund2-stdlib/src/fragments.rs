//! The specialised arms Bund2 publishes, and the test that keeps them honest.
//!
//! # RFC-0005 §S6, criterion 16
//!
//! A fragment is one **arm** of a word, entered only when its guard admits the
//! stack. The word itself is the generic branch, so a fragment can be wrong
//! only on the arm it claims — and criterion 16 requires a differential test
//! that runs the arm and the word on the same inputs and asserts they agree.
//!
//! That test is [`mod tests`] below. Its inputs are the arm's **boundaries,
//! chosen by hand** — not generated. An earlier version of this comment said
//! "generated over the arm's domain", which was false, and the hand-picked set
//! it described omitted the one boundary the integer arm has: `i64`
//! wrap-around.
//!
//! # What is here, and why so little
//!
//! Two arms: `Int + Int` and `dup`/`drop`. They are the two measured in
//! `crates/bund2-bench/benches/fragment.rs`, and RFC-0005 §S6 requires that
//! measurement before more are written. An operation in `bund2_ir::Op` with no
//! fragment using it is a lowering nobody has tested.
//!
//! *(2026-10-15: three more — `Int - Int`, `Int < Int` and `swap` — each
//! measured in the same bench before it was published, and each with the
//! differential test below. They were chosen from one program's profile: a
//! recursive `fib` whose bodies are `dup 1 < { … } if` and
//! `dup 1 swap - fib swap 2 swap - fib +`, where `+` was the only site the
//! tier could inline. `swap`'s arm is published under `swap_one`, the native,
//! for the reason `dup`'s is under `dup_one`.)*
//!
//! *(2026-10-15, later — two more, at the owner's direction: `Int * Int` and
//! `Int > Int`. `bund_examples/jit_vs_tier0` measured a loop body of nine
//! `+`/`-` sites running 6× faster compiled, and the same body with one `+`
//! changed to `*` losing most of that, because `*` had no arm and its site
//! was generic. `>` needed no new operation: it is `<`'s `LtInt` with the
//! register file read the other way. Both measured in the same bench before
//! they were published, both with the differential test below.)*
//!
//! *(2026-10-16 — the other four comparisons, at the owner's direction:
//! `<=`, `>=`, `==` and `!=` on two ints. Three operations for four arms:
//! `>=` is `LeInt` with the register file read the other way, as `>` is
//! `LtInt`; `EqInt` and `NeInt` commute. The six comparison words now read as
//! a set. Each measured in the same bench, each with the differential test
//! below.)*
//!
//! **What makes `Int + Int` free of `q` is the guard, not the arithmetic.**
//! `Guard::TopAreInt` admits only *unboxed* scalars, whose `q` is the 100.0
//! every constructor writes, and the word's result is a fresh value at 100.0
//! too. So the arm and the word agree on `q` by construction on this domain —
//! and criterion 16's `q` assertion cannot fail here. Widening the guard to
//! boxed ints (to admit a value carrying an `attr`, say) would owe no `q`
//! arithmetic either: D32, amended on Q35's answer, says no word averages `q`,
//! so the word's result is a fresh 100.0 whatever its operands carried. What it
//! would owe is an operand with another `q` in this test, if one can be built.

use bund2_ir::{Fragment, Guard, Op};

/// `Int + Int → Int`, the arm behind `+`.
pub fn int_add() -> Result<Fragment, String> {
    Fragment::new(
        Guard::TopAreInt(2),
        vec![
            // The reference pulls the top first; addition is commutative so
            // the order does not change the answer, but the *pulls* must still
            // happen in the word's order or a failure mid-arm would leave a
            // different stack.
            Op::PopInt,
            Op::PopInt,
            Op::AddInt { dst: 0, a: 0, b: 1 },
            Op::PushInt(0),
        ],
        2,
    )
}

/// `Int - Int → Int`, the arm behind `-`.
///
/// **The top is the left operand.** `3 5 -` is `5 - 3`: `run` in
/// `crate::math` pulls `x` first and `numeric_op` answers `x - y`. After two
/// pops slot 0 holds the value popped *last* — the one beneath — and slot 1
/// the top, so the subtraction reads `a: 1, b: 0`. `Op::PopInt`'s own
/// documentation warns that the `int_add` shape copied by analogy computes the
/// reverse, and the differential test below is what catches it: unlike `+`,
/// `-` does not commute.
pub fn int_sub() -> Result<Fragment, String> {
    Fragment::new(
        Guard::TopAreInt(2),
        vec![
            Op::PopInt,
            Op::PopInt,
            Op::SubInt { dst: 0, a: 1, b: 0 },
            Op::PushInt(0),
        ],
        2,
    )
}

/// `Int < Int → Bool`, the arm behind `<`.
///
/// The same orientation as `-`: `1 2 <` asks whether 2 is less than 1 (`run`
/// in `crate::logic` pulls `v1` first and `numeric_ord` orders `v1` against
/// `v2`), so the comparison reads `a: 1, b: 0`. The result is a fresh boolean,
/// which is what the word pushes; a boolean is never promoted.
pub fn int_lt() -> Result<Fragment, String> {
    Fragment::new(
        Guard::TopAreInt(2),
        vec![
            Op::PopInt,
            Op::PopInt,
            Op::LtInt { dst: 0, a: 1, b: 0 },
            Op::PushBool(0),
        ],
        2,
    )
}

/// `Int * Int → Int`, the arm behind `*`.
///
/// Multiplication commutes, so the register order is invisible, as it is for
/// `+`; the pops still happen in the word's order. The word wraps
/// (`Op::int`'s `wrapping_mul` in `crate::math`), and so does `Op::MulInt`,
/// which is the boundary the differential test squares over.
pub fn int_mul() -> Result<Fragment, String> {
    Fragment::new(
        Guard::TopAreInt(2),
        vec![
            Op::PopInt,
            Op::PopInt,
            Op::MulInt { dst: 0, a: 0, b: 1 },
            Op::PushInt(0),
        ],
        2,
    )
}

/// `Int > Int → Bool`, the arm behind `>`.
///
/// **No new operation**: `run` in `crate::logic` pulls `v1` first and
/// `numeric_ord` asks `v1 > v2`, so `1 2 >` asks whether 2 is greater than 1
/// — the top against the value beneath, as `<` does. `top > below` is
/// `below < top`, and after two pops the value beneath is in slot 0 and the
/// top in slot 1, so this is `<`'s `LtInt` with `a` and `b` exchanged:
/// `a: 0, b: 1`. The differential test's squared set holds every pair in
/// both orders, which is where the exchange would show if it were wrong.
pub fn int_gt() -> Result<Fragment, String> {
    Fragment::new(
        Guard::TopAreInt(2),
        vec![
            Op::PopInt,
            Op::PopInt,
            Op::LtInt { dst: 0, a: 0, b: 1 },
            Op::PushBool(0),
        ],
        2,
    )
}

/// `Int <= Int → Bool`, the arm behind `<=`. Read as `<`'s: `1 2 <=` asks
/// whether 2 is at most 1 (`numeric_ord`'s `Op::Le` is `o != Greater` with
/// the top as `v1`), so `a: 1, b: 0`.
pub fn int_le() -> Result<Fragment, String> {
    Fragment::new(
        Guard::TopAreInt(2),
        vec![
            Op::PopInt,
            Op::PopInt,
            Op::LeInt { dst: 0, a: 1, b: 0 },
            Op::PushBool(0),
        ],
        2,
    )
}

/// `Int >= Int → Bool`, the arm behind `>=`: `top >= below` is
/// `below <= top`, so this is `<=`'s `LeInt` with `a` and `b` exchanged, as
/// `>`'s is `<`'s.
pub fn int_ge() -> Result<Fragment, String> {
    Fragment::new(
        Guard::TopAreInt(2),
        vec![
            Op::PopInt,
            Op::PopInt,
            Op::LeInt { dst: 0, a: 0, b: 1 },
            Op::PushBool(0),
        ],
        2,
    )
}

/// `Int == Int → Bool`, the arm behind `==`. On two unboxed ints the word is
/// `i64` equality (`numeric_eq`'s `Int, Int` arm in `crate::logic`); the
/// mixed arm D30 made exact is one the guard never admits. Equality commutes,
/// so the register order is invisible.
pub fn int_eq() -> Result<Fragment, String> {
    Fragment::new(
        Guard::TopAreInt(2),
        vec![
            Op::PopInt,
            Op::PopInt,
            Op::EqInt { dst: 0, a: 0, b: 1 },
            Op::PushBool(0),
        ],
        2,
    )
}

/// `Int != Int → Bool`, the arm behind `!=`: `==`'s complement, as one
/// operation.
pub fn int_ne() -> Result<Fragment, String> {
    Fragment::new(
        Guard::TopAreInt(2),
        vec![
            Op::PopInt,
            Op::PopInt,
            Op::NeInt { dst: 0, a: 0, b: 1 },
            Op::PushBool(0),
        ],
        2,
    )
}

/// `swap` — exchange the top two, whatever they are. Depth is the only guard,
/// because `swap_one` inspects neither value.
pub fn swap() -> Result<Fragment, String> {
    Fragment::new(Guard::Depth(2), vec![Op::SwapTop], 0)
}

/// `dup` — copy the top, whatever it is. No guard beyond depth, because the
/// word does not inspect the value either.
pub fn dup() -> Result<Fragment, String> {
    Fragment::new(Guard::Depth(1), vec![Op::DupTop], 0)
}

/// `drop` — discard the top.
pub fn drop_top() -> Result<Fragment, String> {
    Fragment::new(Guard::Depth(1), vec![Op::DropTop], 0)
}

/// The words whose arms this crate publishes, and the fragment for each.
///
/// **`dup_one`, not `dup`; `swap_one`, not `swap`.** `dup` and `swap` are
/// aliases (`crates/bund2-stdlib/src/stack.rs`, `register`), and an alias
/// holds no `Native` and therefore no registration id, so keying an arm by the
/// alias would put an entry in the table that can never match a callee. The
/// natives are `dup_one` and `swap_one`. `+`, `-`, `*`, the six comparisons
/// and `drop` are registered under their own names.
const PUBLISHED: [(&str, Build); 12] = [
    ("+", int_add),
    ("-", int_sub),
    ("*", int_mul),
    ("<", int_lt),
    (">", int_gt),
    ("<=", int_le),
    (">=", int_ge),
    ("==", int_eq),
    ("!=", int_ne),
    ("dup_one", dup),
    ("swap_one", swap),
    ("drop", drop_top),
];

/// How an arm is built — each constructor validates, so each can refuse.
type Build = fn() -> Result<Fragment, String>;

/// **The `(registration id, Fragment)` table — RFC-0005 §S6, D43.**
///
/// §S6 puts the association here: "`bund2-stdlib` publishes
/// `(registration id, Fragment)` pairs from `crate::fragments`, and
/// `bund2-jit` — which may depend on `bund2-stdlib` (D9 amended) — reads them
/// when it compiles a site." `bund2-api` carries no `Fragment` type, so the id
/// is the only thing crossing that boundary, and an external package still
/// cannot publish an arm.
///
/// # Take this at registration time
///
/// The ids are whatever the registry's slots hold **now**, so this must be
/// called once the vocabulary is registered and the result kept. §S6: the
/// table is "built per registry, at registration time, never as a static".
///
/// **A later re-registration is supposed to stop matching.** `register` is a
/// word, so a program can rebind `+`; F32's replay mints a *fresh* id, the
/// entry taken earlier no longer matches the slot, and a consumer declines to
/// inline the arm. That is the meaning guard working rather than a stale table
/// — and it is why the fragment is keyed by the registration and not by the
/// name.
///
/// # Why a `Result`
///
/// Each constructor validates, and a malformed fragment is a defect in Bund2
/// rather than a fact about the program being run. Shipped code may not
/// `expect` (D37), so the error travels to the caller, which is the consumer
/// best placed to report it.
///
/// A word this crate did not register — under `--noio` or `--noeval`, say, or
/// in a registry built by hand — is simply absent from the table. None of the
/// twelve is ever stubbed, so in a default registry all twelve are present.
pub fn published(
    r: &bund2_api::Registry,
) -> Result<Vec<(bund2_api::RegistrationId, Fragment)>, String> {
    let mut out = Vec::with_capacity(PUBLISHED.len());
    for (name, build) in PUBLISHED {
        // `Interner::lookup_call` rather than `intern`: this takes
        // `&Registry`, and the crate's rule is that looking up a miss must not
        // retain a slot. None of the three names carries a `$`, so the sigil
        // half is moot. `Registry` has no by-name symbol lookup of its own —
        // its `&self` accessors answer with a *binding* — so this goes through
        // the public `interner`, as `bund2-interp`'s `dispatch_name` does.
        let Some((s, _)) = r.interner.lookup_call(name) else {
            continue;
        };
        let Some(id) = r
            .slot(s)
            .and_then(|sl| sl.native.as_ref())
            .and_then(|n| n.id)
        else {
            continue;
        };
        out.push((id, build()?));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_api::Vm as _;
    use bund2_interp::{Interp, frag};
    use bund2_value::BundValue;

    /// Blank out the two fields that differ per value and per run — F14's
    /// normalisation, borrowed so the comparison is about the rest.
    ///
    /// **This hides identity**, which is why F13's fresh-identity property is
    /// asserted separately in `dup_mints_a_fresh_identity_in_the_arm_and_the_word`
    /// rather than left to the render comparison.
    fn norm(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut rest = s;
        while let Some(i) = rest.find("id: \"") {
            out.push_str(&rest[..i + 5]);
            rest = &rest[i + 5..];
            let end = rest.find('"').unwrap_or(0);
            out.push_str("<id>");
            rest = &rest[end..];
        }
        out.push_str(rest);
        let mut out2 = String::with_capacity(out.len());
        let mut rest = out.as_str();
        while let Some(i) = rest.find("stamp: ") {
            out2.push_str(&rest[..i + 7]);
            rest = &rest[i + 7..];
            let end = rest.find(&[',', ' '][..]).unwrap_or(0);
            out2.push_str("<stamp>");
            rest = &rest[end..];
        }
        out2.push_str(rest);
        out2
    }

    fn interp() -> Interp {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i
    }

    /// Push `operands` bottom-first, run `word` by name, return the stack.
    ///
    /// **The same values the fragment receives, not the same source text.** An
    /// earlier version compiled `format!("{a} {b} +")`, which sends the word's
    /// operands through the lexer and the fragment's around it. At `i64::MIN`
    /// that compares the lexer's reading of `-9223372036854775808` against a
    /// constructed value, and a disagreement there is about literals, not about
    /// the arm.
    fn via_word(word: &str, operands: &[BundValue]) -> Vec<BundValue> {
        let mut i = interp();
        for v in operands {
            i.push(v.clone());
        }
        let stream = bund2_syntax::compile(word).expect("compiles");
        i.eval(&stream).expect("runs");
        i.snapshot()
    }

    /// Push `operands` bottom-first, run the fragment, return the stack — or
    /// `None` if the guard declined. An internal error fails the test: it means
    /// the fragment is malformed, which `validate` should have refused first.
    fn via_fragment(f: &Fragment, operands: &[BundValue]) -> Option<Vec<BundValue>> {
        assert!(
            f.validate().is_ok(),
            "a fragment under test must be well-formed"
        );
        let mut i = interp();
        for v in operands {
            i.push(v.clone());
        }
        match frag::run(&mut i, f) {
            Ok(true) => Some(i.snapshot()),
            Ok(false) => None,
            Err(e) => panic!("fragment raised an internal error: {}", e.0),
        }
    }

    /// The integer arm's boundaries, chosen by hand.
    ///
    /// The last four are what an earlier version lacked. `Op::AddInt` wraps,
    /// as the word's `Op::int` does (`crates/bund2-stdlib/src/math.rs`, the
    /// `wrapping_add` arm), and wrap-around is exactly where a lowering using a
    /// trapping or checked `iadd` would part company with the word.
    const INT_BOUNDARY: [i64; 15] = [
        0,
        1,
        -1,
        2,
        -2,
        7,
        42,
        -42,
        1000,
        i32::MAX as i64,
        i32::MIN as i64,
        i64::MAX,
        i64::MIN,
        i64::MAX - 1,
        i64::MIN + 1,
    ];

    /// **Criterion 16 for `Int + Int`**, over the boundary set.
    #[test]
    fn the_int_add_arm_agrees_with_the_word() {
        for &a in &INT_BOUNDARY {
            for &b in &INT_BOUNDARY {
                let ops = [BundValue::int(a), BundValue::int(b)];
                let word = via_word("+", &ops);
                let frag = via_fragment(&int_add().expect("well-formed"), &ops)
                    .unwrap_or_else(|| panic!("guard declined {a} + {b}"));
                assert_eq!(word.len(), frag.len(), "{a} + {b}: different stack depth");
                for (w, f) in word.iter().zip(frag.iter()) {
                    assert_eq!(w.as_int(), f.as_int(), "{a} + {b}: different value");
                    assert_eq!(w.dt(), f.dt(), "{a} + {b}: different dt");
                    // Cannot fail on this domain — see the module doc. Kept so
                    // that widening the guard makes it able to.
                    assert!(
                        (w.q() - f.q()).abs() < f64::EPSILON,
                        "{a} + {b}: q diverged, {} against {}",
                        w.q(),
                        f.q()
                    );
                    assert_eq!(
                        w.tags().get("stack").map(|s| &**s),
                        f.tags().get("stack").map(|s| &**s),
                        "{a} + {b}: stack tag diverged — D41"
                    );
                }
            }
        }
    }

    /// Wrap-around asserted **by value**, not only by agreement: a harness
    /// fault that broke both paths identically would pass the test above.
    #[test]
    fn the_int_add_arm_wraps() {
        let out = via_fragment(
            &int_add().expect("well-formed"),
            &[BundValue::int(i64::MAX), BundValue::int(1)],
        )
        .expect("admitted");
        assert_eq!(out.last().and_then(|v| v.as_int()), Some(i64::MIN));
    }

    /// The arm must **decline** everything outside its guard, and the word
    /// must still be the one that answers.
    #[test]
    fn the_int_add_arm_declines_what_it_does_not_claim() {
        let outside = [
            vec![BundValue::int(1), BundValue::float(2.0)],
            vec![BundValue::float(1.0), BundValue::int(2)],
            vec![BundValue::str("a"), BundValue::int(2)],
            vec![BundValue::int(1).promote(), BundValue::int(2)],
            vec![BundValue::int(1)],
            vec![],
        ];
        for ops in outside {
            assert!(
                via_fragment(&int_add().expect("well-formed"), &ops).is_none(),
                "arm claimed {ops:?}, which is outside TopAreInt(2)"
            );
        }
    }

    /// Compare one value from the word's stack against one from the arm's on
    /// everything criterion 16 names but identity.
    fn assert_same(w: &BundValue, f: &BundValue, label: &str) {
        assert_eq!(w.dt(), f.dt(), "{label}: different dt");
        assert_eq!(w.as_int(), f.as_int(), "{label}: different value");
        assert!(
            (w.q() - f.q()).abs() < f64::EPSILON,
            "{label}: q diverged, {} against {}",
            w.q(),
            f.q()
        );
        assert_eq!(
            w.tags().get("stack").map(|s| &**s),
            f.tags().get("stack").map(|s| &**s),
            "{label}: stack tag diverged — D41"
        );
        assert_eq!(
            norm(&w.render(false)),
            norm(&f.render(false)),
            "{label}: render"
        );
    }

    /// **Criterion 16 for `Int - Int`**, over the boundary set, **in both
    /// orders by construction** — the set is squared, so every pair appears
    /// reversed too, and subtraction does not forgive a reversed register file
    /// the way addition does.
    #[test]
    fn the_int_sub_arm_agrees_with_the_word() {
        for &a in &INT_BOUNDARY {
            for &b in &INT_BOUNDARY {
                let ops = [BundValue::int(a), BundValue::int(b)];
                let word = via_word("-", &ops);
                let frag = via_fragment(&int_sub().expect("well-formed"), &ops)
                    .unwrap_or_else(|| panic!("guard declined {a} {b} -"));
                assert_eq!(word.len(), frag.len(), "{a} {b} -: different stack depth");
                for (w, f) in word.iter().zip(frag.iter()) {
                    assert_same(w, f, &format!("{a} {b} -"));
                }
            }
        }
    }

    /// **The orientation and the wrap, asserted by value.** `3 5 -` is `2`
    /// and not `-2`; `i64::MIN 1 -` is `1 - i64::MIN`, which wraps to
    /// `i64::MIN + 1`... read the other way: the top is the left operand, so
    /// `1 i64::MIN -` is `i64::MIN - 1` and wraps to `i64::MAX`.
    #[test]
    fn the_int_sub_arm_takes_the_top_as_its_left_operand_and_wraps() {
        let arm = int_sub().expect("well-formed");
        let out = via_fragment(&arm, &[BundValue::int(3), BundValue::int(5)]).expect("admitted");
        assert_eq!(
            out.last().and_then(|v| v.as_int()),
            Some(2),
            "`3 5 -` is 5 - 3"
        );
        let out =
            via_fragment(&arm, &[BundValue::int(1), BundValue::int(i64::MIN)]).expect("admitted");
        assert_eq!(
            out.last().and_then(|v| v.as_int()),
            Some(i64::MAX),
            "`1 i64::MIN -` is i64::MIN - 1, wrapped"
        );
    }

    /// **Criterion 16 for `Int < Int`**, over the same squared set, which
    /// covers `a < b`, `a > b` and `a == b` at every boundary.
    #[test]
    fn the_int_lt_arm_agrees_with_the_word() {
        for &a in &INT_BOUNDARY {
            for &b in &INT_BOUNDARY {
                let ops = [BundValue::int(a), BundValue::int(b)];
                let word = via_word("<", &ops);
                let frag = via_fragment(&int_lt().expect("well-formed"), &ops)
                    .unwrap_or_else(|| panic!("guard declined {a} {b} <"));
                assert_eq!(word.len(), frag.len(), "{a} {b} <: different stack depth");
                for (w, f) in word.iter().zip(frag.iter()) {
                    assert_same(w, f, &format!("{a} {b} <"));
                }
            }
        }
    }

    /// **The orientation, asserted by value**: `1 2 <` asks whether 2 is less
    /// than 1, and the oracle answers `false`.
    #[test]
    fn the_int_lt_arm_compares_the_top_against_the_value_beneath() {
        let arm = int_lt().expect("well-formed");
        let out = via_fragment(&arm, &[BundValue::int(1), BundValue::int(2)]).expect("admitted");
        assert!(
            matches!(out.last(), Some(BundValue::Bool(false, _))),
            "`1 2 <`"
        );
        let out = via_fragment(&arm, &[BundValue::int(2), BundValue::int(1)]).expect("admitted");
        assert!(
            matches!(out.last(), Some(BundValue::Bool(true, _))),
            "`2 1 <`"
        );
        let out = via_fragment(&arm, &[BundValue::int(2), BundValue::int(2)]).expect("admitted");
        assert!(
            matches!(out.last(), Some(BundValue::Bool(false, _))),
            "`2 2 <`"
        );
    }

    /// **Criterion 16 for `Int * Int`**, over the squared boundary set, which
    /// holds the wrap at every corner: `i64::MAX * 2`, `i64::MIN * -1`,
    /// `i64::MAX * i64::MAX`.
    #[test]
    fn the_int_mul_arm_agrees_with_the_word() {
        for &a in &INT_BOUNDARY {
            for &b in &INT_BOUNDARY {
                let ops = [BundValue::int(a), BundValue::int(b)];
                let word = via_word("*", &ops);
                let frag = via_fragment(&int_mul().expect("well-formed"), &ops)
                    .unwrap_or_else(|| panic!("guard declined {a} {b} *"));
                assert_eq!(word.len(), frag.len(), "{a} {b} *: different stack depth");
                for (w, f) in word.iter().zip(frag.iter()) {
                    assert_same(w, f, &format!("{a} {b} *"));
                }
            }
        }
    }

    /// **The wrap, asserted by value**: `i64::MAX 2 *` is `-2`, and
    /// `i64::MIN -1 *` is `i64::MIN` — the two products a checked or trapping
    /// `imul` would refuse.
    #[test]
    fn the_int_mul_arm_wraps() {
        let arm = int_mul().expect("well-formed");
        let out =
            via_fragment(&arm, &[BundValue::int(i64::MAX), BundValue::int(2)]).expect("admitted");
        assert_eq!(out.last().and_then(|v| v.as_int()), Some(-2));
        let out =
            via_fragment(&arm, &[BundValue::int(i64::MIN), BundValue::int(-1)]).expect("admitted");
        assert_eq!(out.last().and_then(|v| v.as_int()), Some(i64::MIN));
        let out = via_fragment(&arm, &[BundValue::int(3), BundValue::int(5)]).expect("admitted");
        assert_eq!(out.last().and_then(|v| v.as_int()), Some(15));
    }

    /// **Criterion 16 for `Int > Int`**, over the squared set, so every pair
    /// appears in both orders — which is where an exchanged register file
    /// would show, since `>` is `<` with `a` and `b` swapped.
    #[test]
    fn the_int_gt_arm_agrees_with_the_word() {
        for &a in &INT_BOUNDARY {
            for &b in &INT_BOUNDARY {
                let ops = [BundValue::int(a), BundValue::int(b)];
                let word = via_word(">", &ops);
                let frag = via_fragment(&int_gt().expect("well-formed"), &ops)
                    .unwrap_or_else(|| panic!("guard declined {a} {b} >"));
                assert_eq!(word.len(), frag.len(), "{a} {b} >: different stack depth");
                for (w, f) in word.iter().zip(frag.iter()) {
                    assert_same(w, f, &format!("{a} {b} >"));
                }
            }
        }
    }

    /// **The orientation, asserted by value**: `1 2 >` asks whether 2 is
    /// greater than 1, and the oracle answers `true`.
    #[test]
    fn the_int_gt_arm_compares_the_top_against_the_value_beneath() {
        let arm = int_gt().expect("well-formed");
        let out = via_fragment(&arm, &[BundValue::int(1), BundValue::int(2)]).expect("admitted");
        assert!(
            matches!(out.last(), Some(BundValue::Bool(true, _))),
            "`1 2 >`"
        );
        let out = via_fragment(&arm, &[BundValue::int(2), BundValue::int(1)]).expect("admitted");
        assert!(
            matches!(out.last(), Some(BundValue::Bool(false, _))),
            "`2 1 >`"
        );
        let out = via_fragment(&arm, &[BundValue::int(2), BundValue::int(2)]).expect("admitted");
        assert!(
            matches!(out.last(), Some(BundValue::Bool(false, _))),
            "`2 2 >`"
        );
    }

    /// **Criterion 16 for `<=`, `>=`, `==` and `!=`** on ints, over the
    /// squared set, which holds `a < b`, `a == b` and `a > b` at every
    /// boundary and so every answer each word can give.
    #[test]
    fn the_int_le_ge_eq_and_ne_arms_agree_with_their_words() {
        for (word, arm) in [
            ("<=", int_le()),
            (">=", int_ge()),
            ("==", int_eq()),
            ("!=", int_ne()),
        ] {
            let arm = arm.expect("well-formed");
            for &a in &INT_BOUNDARY {
                for &b in &INT_BOUNDARY {
                    let ops = [BundValue::int(a), BundValue::int(b)];
                    let by_word = via_word(word, &ops);
                    let by_frag = via_fragment(&arm, &ops)
                        .unwrap_or_else(|| panic!("guard declined {a} {b} {word}"));
                    assert_eq!(
                        by_word.len(),
                        by_frag.len(),
                        "{a} {b} {word}: different stack depth"
                    );
                    for (w, f) in by_word.iter().zip(by_frag.iter()) {
                        assert_same(w, f, &format!("{a} {b} {word}"));
                    }
                }
            }
        }
    }

    /// **The orientation and the answers, asserted by value.** `1 2 <=` asks
    /// whether 2 is at most 1, and the oracle answers `false`; `1 2 >=` asks
    /// whether 2 is at least 1, `true`; both hold at equality. `==` and `!=`
    /// answer opposite ways on the same pair, and `i64::MIN` against itself
    /// is equal.
    #[test]
    fn the_int_le_ge_eq_and_ne_arms_answer_as_the_oracle_does() {
        let answer = |arm: &Fragment, a: i64, b: i64| -> bool {
            let out = via_fragment(arm, &[BundValue::int(a), BundValue::int(b)]).expect("admitted");
            match out.last() {
                Some(BundValue::Bool(v, _)) => *v,
                other => panic!("{a} {b}: left {other:?}, not a boolean"),
            }
        };
        let le = int_le().expect("well-formed");
        assert!(!answer(&le, 1, 2), "`1 2 <=`");
        assert!(answer(&le, 2, 1), "`2 1 <=`");
        assert!(answer(&le, 2, 2), "`2 2 <=`");
        let ge = int_ge().expect("well-formed");
        assert!(answer(&ge, 1, 2), "`1 2 >=`");
        assert!(!answer(&ge, 2, 1), "`2 1 >=`");
        assert!(answer(&ge, 2, 2), "`2 2 >=`");
        let eq = int_eq().expect("well-formed");
        assert!(answer(&eq, 2, 2), "`2 2 ==`");
        assert!(!answer(&eq, 1, 2), "`1 2 ==`");
        assert!(answer(&eq, i64::MIN, i64::MIN), "`MIN MIN ==`");
        assert!(!answer(&eq, i64::MIN, i64::MAX), "`MIN MAX ==`");
        let ne = int_ne().expect("well-formed");
        assert!(!answer(&ne, 2, 2), "`2 2 !=`");
        assert!(answer(&ne, 1, 2), "`1 2 !=`");
        assert!(answer(&ne, i64::MIN, i64::MAX), "`MIN MAX !=`");
    }

    /// The eight two-operand integer arms must **decline** everything outside
    /// `TopAreInt(2)`.
    #[test]
    fn the_two_operand_int_arms_decline_what_they_do_not_claim() {
        let outside = [
            vec![BundValue::int(1), BundValue::float(2.0)],
            vec![BundValue::float(1.0), BundValue::int(2)],
            vec![BundValue::str("a"), BundValue::int(2)],
            vec![BundValue::int(1).promote(), BundValue::int(2)],
            vec![BundValue::int(1)],
            vec![],
        ];
        for ops in outside {
            for (name, arm) in [
                ("-", int_sub()),
                ("*", int_mul()),
                ("<", int_lt()),
                (">", int_gt()),
                ("<=", int_le()),
                (">=", int_ge()),
                ("==", int_eq()),
                ("!=", int_ne()),
            ] {
                assert!(
                    via_fragment(&arm.expect("well-formed"), &ops).is_none(),
                    "{name}'s arm claimed {ops:?}, which is outside TopAreInt(2)"
                );
            }
        }
    }

    /// **Criterion 16 for `swap`**, over every ordered pair of the four
    /// shapes, and the moved-not-copied property: `swap` copies nothing, so a
    /// heap value keeps the payload it had, under the arm as under the word.
    /// The render comparison cannot see that, and `identity()` mints an id
    /// for an unboxed scalar on every call, so the property is asserted on
    /// the **payload key** (D35) of the two heap shapes.
    #[test]
    fn the_swap_arm_agrees_with_its_word_and_moves_rather_than_copies() {
        let arm = swap().expect("well-formed");
        for a in shapes() {
            for b in shapes() {
                let ops = [a.clone(), b.clone()];
                let by_word = via_word("swap", &ops);
                let by_frag =
                    via_fragment(&arm, &ops).unwrap_or_else(|| panic!("swap declined {a:?} {b:?}"));
                assert_eq!(by_word.len(), 2, "{a:?} {b:?} swap: depth");
                assert_eq!(by_frag.len(), 2, "{a:?} {b:?} swap: depth");
                for (w, g) in by_word.iter().zip(by_frag.iter()) {
                    assert_eq!(w.dt(), g.dt(), "{a:?} {b:?} swap: dt");
                    assert_eq!(
                        norm(&w.render(false)),
                        norm(&g.render(false)),
                        "{a:?} {b:?} swap"
                    );
                }
                for (label, stack) in [("word", &by_word), ("arm", &by_frag)] {
                    assert_eq!(
                        stack[0].payload_key(),
                        b.payload_key(),
                        "{label}: the former top keeps its payload"
                    );
                    assert_eq!(
                        stack[1].payload_key(),
                        a.payload_key(),
                        "{label}: the former second keeps its payload"
                    );
                }
            }
        }
        assert!(
            shapes()
                .iter()
                .filter(|v| v.payload_key().is_some())
                .count()
                >= 2,
            "the property above must be asked of at least two heap shapes"
        );
        assert!(
            via_fragment(&arm, &[BundValue::int(1)]).is_none(),
            "one value is outside Depth(2)"
        );
    }

    /// One value of each payload shape `dup` and `drop` will see.
    fn shapes() -> [BundValue; 4] {
        [
            BundValue::int(1),
            BundValue::float(2.5),
            BundValue::str("s"),
            BundValue::list(vec![BundValue::int(1), BundValue::int(2)]),
        ]
    }

    /// **Criterion 16 for `dup` and `drop`**, everything but identity.
    ///
    /// **`"dup"` here is deliberate and differs from [`PUBLISHED`]'s
    /// `"dup_one"`.** This test dispatches *by name*, and dispatch resolves the
    /// alias, so `"dup"` reaches the same native and is the spelling a program
    /// writes. The published table keys by *registration id*, which an alias
    /// does not have. Both are right for what they do; making them agree would
    /// break one of them.
    #[test]
    fn the_dup_and_drop_arms_agree_with_their_words() {
        for v in shapes() {
            for (word, f) in [
                ("dup", dup().expect("well-formed")),
                ("drop", drop_top().expect("well-formed")),
            ] {
                let by_word = via_word(word, std::slice::from_ref(&v));
                let by_frag = via_fragment(&f, std::slice::from_ref(&v))
                    .unwrap_or_else(|| panic!("{word} declined {v:?}"));
                assert_eq!(by_word.len(), by_frag.len(), "{v:?} {word}: depth");
                for (w, g) in by_word.iter().zip(by_frag.iter()) {
                    assert_eq!(w.dt(), g.dt(), "{v:?} {word}: dt");
                    assert_eq!(
                        norm(&w.render(false)),
                        norm(&g.render(false)),
                        "{v:?} {word}"
                    );
                }
            }
        }
    }

    /// **F13, asserted directly.** `dup` gives the copy a fresh header, so the
    /// two values it leaves have *different* identities; `push(clone)` would
    /// share one Rc and therefore one identity.
    ///
    /// The render comparison above cannot see this: identities differ from run
    /// to run, so `norm` blanks them, and a shared identity and two distinct
    /// ones normalise to the same text. The version of this test criterion
    /// 16's first review examined relied on the render alone.
    #[test]
    fn dup_mints_a_fresh_identity_in_the_arm_and_the_word() {
        for v in shapes() {
            let by_word = via_word("dup", std::slice::from_ref(&v));
            let by_frag = via_fragment(&dup().expect("well-formed"), std::slice::from_ref(&v))
                .expect("admitted");
            for (label, stack) in [("word", by_word), ("arm", by_frag)] {
                assert_eq!(
                    stack.len(),
                    2,
                    "{label}: dup of {v:?} left {} values",
                    stack.len()
                );
                let (below, _) = stack[0].identity();
                let (top, _) = stack[1].identity();
                assert_ne!(
                    below, top,
                    "{label}: dup of {v:?} shared one identity — that is a clone, not F13's dup"
                );
            }
        }
    }

    fn noop(_: &mut dyn bund2_api::Vm) -> Result<(), bund2_api::Error> {
        Ok(())
    }

    fn stdlib_registry() -> bund2_api::Registry {
        let mut r = bund2_api::Registry::new();
        crate::register_all(&mut r);
        r
    }

    fn id_of(r: &bund2_api::Registry, name: &str) -> Option<bund2_api::RegistrationId> {
        let (s, _) = r.interner.lookup_call(name)?;
        r.slot(s)
            .and_then(|sl| sl.native.as_ref())
            .and_then(|n| n.id)
    }

    /// **RFC-0005 §S6, D43.** The table names twelve registrations, each the
    /// one `register_all` minted for that word, and no two share an id.
    #[test]
    fn the_published_table_keys_each_arm_by_its_registration() {
        let r = stdlib_registry();
        let table = published(&r).expect("the published fragments are well-formed");
        assert_eq!(table.len(), 12, "§S5: the fragment table holds twelve ids");

        let ids: std::collections::BTreeSet<_> = table.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), 12, "two arms must not share a registration id");

        for name in [
            "+", "-", "*", "<", ">", "<=", ">=", "==", "!=", "dup_one", "swap_one", "drop",
        ] {
            let id = id_of(&r, name).unwrap_or_else(|| panic!("{name} is registered as a native"));
            assert!(ids.contains(&id), "{name}'s registration is in the table");
        }
    }

    /// **The alias trap, pinned.** `dup` is an alias, so it holds no `Native`
    /// and no registration id; the arm belongs to `dup_one`. Keying by `dup`
    /// would put an entry in the table that no callee could ever match, and
    /// nothing else in the build would complain.
    #[test]
    fn the_dup_arm_is_keyed_by_the_native_and_not_by_its_alias() {
        let r = stdlib_registry();
        assert_eq!(
            id_of(&r, "dup"),
            None,
            "`dup` is an alias: no Native, no id"
        );
        let native = id_of(&r, "dup_one").expect("`dup_one` is the native");

        let table = published(&r).expect("well-formed");
        let dup_arm = dup().expect("well-formed");
        let found = table.iter().find(|(_, f)| *f == dup_arm);
        let (id, _) = found.expect("the dup arm is published");
        assert_eq!(*id, native, "keyed by `dup_one`'s registration");
    }

    /// The same trap for `swap`, with a twist F20 records: `swap`'s slot
    /// holds a native of its own (`swap_n`, the depth-taking form) **and** an
    /// alias to `swap_one`, and the alias wins because the chain consults it
    /// first. So `swap` has a registration id, and it is the wrong one — the
    /// arm must be keyed by `swap_one`'s, which is the word a program's `swap`
    /// reaches.
    #[test]
    fn the_swap_arm_is_keyed_by_the_native_the_alias_reaches() {
        let r = stdlib_registry();
        let shadowed = id_of(&r, "swap").expect("`swap`'s slot holds `swap_n` (F20)");
        let native = id_of(&r, "swap_one").expect("`swap_one` is the native");
        assert_ne!(shadowed, native, "two registrations, two ids");
        let (s, _) = r.interner.lookup_call("swap").expect("interned");
        let (t, _) = r.interner.lookup_call("swap_one").expect("interned");
        assert_eq!(
            r.resolve_target(s),
            t,
            "`swap` resolves through its alias to `swap_one`"
        );

        let table = published(&r).expect("well-formed");
        let swap_arm = swap().expect("well-formed");
        let found = table.iter().find(|(_, f)| *f == swap_arm);
        let (id, _) = found.expect("the swap arm is published");
        assert_eq!(*id, native, "keyed by `swap_one`'s registration");
        assert!(
            !table.iter().any(|(id, _)| *id == shadowed),
            "the shadowed `swap_n` registration is not in the table"
        );
    }

    /// **Why the key is the registration and not the name.** `register` is a
    /// word, so a program can rebind `+`. F32's replay mints a *fresh* id, so a
    /// table taken earlier stops matching the slot and a consumer declines to
    /// inline the arm. That is the meaning guard working — the stale entry is
    /// the mechanism, not a leak.
    #[test]
    fn a_rebound_word_no_longer_matches_the_table_taken_before_it() {
        let mut r = stdlib_registry();
        let table = published(&r).expect("well-formed");
        let before = id_of(&r, "+").expect("`+` is a native");

        r.register_native(
            "+",
            noop,
            bund2_api::StackEffect::fixed(2, 1),
            bund2_api::WordKind::Sync,
        );
        let after = id_of(&r, "+").expect("still a native");

        assert_ne!(before, after, "a re-registration mints a fresh id (F32)");
        assert!(
            table.iter().any(|(id, _)| *id == before),
            "the table still holds the registration it was taken from"
        );
        assert!(
            !table.iter().any(|(id, _)| *id == after),
            "and does not match the new one, so the arm is not inlined"
        );
    }

    /// A registry this crate never registered publishes nothing — the table is
    /// built from what is there, not from a static list of names.
    #[test]
    fn an_empty_registry_publishes_no_arms() {
        let r = bund2_api::Registry::new();
        assert!(published(&r).expect("well-formed").is_empty());
    }
}
