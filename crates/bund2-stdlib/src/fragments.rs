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

/// The words whose arms this crate publishes, and the fragment for each.
///
/// **`dup_one`, not `dup`.** `dup` is an alias
/// (`crates/bund2-stdlib/src/stack.rs`, `register`), and an alias holds no
/// `Native` and therefore no registration id, so keying the arm by `dup` would
/// put an entry in the table that can never match a callee. The native is
/// `dup_one`. `+` and `drop` are registered under their own names.
const PUBLISHED: [(&str, Build); 3] = [("+", int_add), ("dup_one", dup), ("drop", drop_top)];

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
/// three is ever stubbed, so in a default registry all three are present.
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
        let Some(id) = r.slot(s).and_then(|sl| sl.native.as_ref()).and_then(|n| n.id) else {
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
        r.slot(s).and_then(|sl| sl.native.as_ref()).and_then(|n| n.id)
    }

    /// **RFC-0005 §S6, D43.** The table names three registrations, each the one
    /// `register_all` minted for that word, and no two share an id.
    #[test]
    fn the_published_table_keys_each_arm_by_its_registration() {
        let r = stdlib_registry();
        let table = published(&r).expect("the published fragments are well-formed");
        assert_eq!(table.len(), 3, "§S5: the fragment table holds three ids");

        let ids: std::collections::BTreeSet<_> = table.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), 3, "two arms must not share a registration id");

        for name in ["+", "dup_one", "drop"] {
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
        assert_eq!(id_of(&r, "dup"), None, "`dup` is an alias: no Native, no id");
        let native = id_of(&r, "dup_one").expect("`dup_one` is the native");

        let table = published(&r).expect("well-formed");
        let dup_arm = dup().expect("well-formed");
        let found = table.iter().find(|(_, f)| *f == dup_arm);
        let (id, _) = found.expect("the dup arm is published");
        assert_eq!(*id, native, "keyed by `dup_one`'s registration");
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
