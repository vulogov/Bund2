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

/// `dup` — copy the top, whatever it is. No guard beyond depth, because the
/// word does not inspect the value either.
pub fn dup() -> Result<Fragment, String> {
    Fragment::new(Guard::Depth(1), vec![Op::DupTop], 0)
}

/// `drop` — discard the top.
pub fn drop_top() -> Result<Fragment, String> {
    Fragment::new(Guard::Depth(1), vec![Op::DropTop], 0)
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
        assert!(f.validate().is_ok(), "a fragment under test must be well-formed");
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
        let out = via_fragment(&int_add().expect("well-formed"), &[BundValue::int(i64::MAX), BundValue::int(1)])
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
    #[test]
    fn the_dup_and_drop_arms_agree_with_their_words() {
        for v in shapes() {
            for (word, f) in [("dup", dup().expect("well-formed")), ("drop", drop_top().expect("well-formed"))] {
                let by_word = via_word(word, std::slice::from_ref(&v));
                let by_frag = via_fragment(&f, std::slice::from_ref(&v))
                    .unwrap_or_else(|| panic!("{word} declined {v:?}"));
                assert_eq!(by_word.len(), by_frag.len(), "{v:?} {word}: depth");
                for (w, g) in by_word.iter().zip(by_frag.iter()) {
                    assert_eq!(w.dt(), g.dt(), "{v:?} {word}: dt");
                    assert_eq!(norm(&w.render(false)), norm(&g.render(false)), "{v:?} {word}");
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
            let by_frag = via_fragment(&dup().expect("well-formed"), std::slice::from_ref(&v)).expect("admitted");
            for (label, stack) in [("word", by_word), ("arm", by_frag)] {
                assert_eq!(stack.len(), 2, "{label}: dup of {v:?} left {} values", stack.len());
                let (below, _) = stack[0].identity();
                let (top, _) = stack[1].identity();
                assert_ne!(
                    below, top,
                    "{label}: dup of {v:?} shared one identity — that is a clone, not F13's dup"
                );
            }
        }
    }
}
