//! Executing a [`Fragment`] — the **first consumer**, and the one that says
//! whether the representation is worth a code generator.
//!
//! # What this is for
//!
//! RFC-0005 §S6's mechanism is that a word publishes a BundIR fragment and
//! `bund2-jit` lowers it. There is no Tier 1 yet, so nothing would exercise a
//! fragment at all — and an unexercised representation is a guess.
//!
//! This runs one directly, on the interpreter's own stack. It is not Tier 1
//! and it is not a JIT: no code is generated, nothing is cached, and the win
//! is only what a specialised arm buys over a generic one. **Its purpose is to
//! make the representation testable and to measure the consumer's half of
//! §S6's ceiling before a lowering is written.**
//!
//! # The rule it must not break
//!
//! A fragment is entered only when its [`Guard`] admits the stack, and the
//! word runs otherwise. So a fragment can be wrong only on the arm it claims,
//! which is what RFC-0005 criterion 16's differential test checks — and what
//! keeps this from being a second implementation of the language.

use bund2_api::{Error, Vm as _};
use bund2_ir::{Fragment, Op};
use bund2_value::BundValue;

use crate::Interp;

/// Run `f` against the interpreter's current stack.
///
/// `Ok(false)` means **the guard declined and the stack was not touched**, so
/// the caller falls through to the word. `Ok(true)` means the fragment ran to
/// completion.
///
/// # Why this returns a `Result` and not a `bool`
///
/// There are two ways a fragment can fail to run, and collapsing them into one
/// `false` is unsound. A guard that declines has consumed nothing, so the word
/// runs next and sees the stack the program built. An **op** that fails has
/// already pulled: `PopInt` consumes before anything can go wrong. Returning
/// `false` there would send the caller to the word with its operands gone —
/// `1 2 +` would run `+` on an empty stack and report a depth error naming a
/// program that was correct.
///
/// A post-guard failure is a broken invariant in Bund2, not a fact about the
/// program being run, so it is [`Error::internal`] — the third of CLAUDE.md's
/// three ways out, and D37's case exactly. [`Fragment::validate`] exists so
/// that a fragment reaching this function cannot fail this way; the arm stays
/// because "cannot happen" is not an explanation.
pub fn run(i: &mut Interp, f: &Fragment) -> Result<bool, Error> {
    // Peek, never pull, until the guard is known to hold.
    //
    // **Nothing on this path allocates.** The guard asks through indexed
    // peeks, and the register file is a fixed array. The first version
    // snapshotted the whole stack to read two values; the second collected the
    // top into a `Vec` and allocated the register file — and measured *slower
    // than the word it stands in for* (RFC-0005 §S6). A consumer that exists to
    // measure what an inlined arm costs cannot carry costs no lowering would.
    if !f.admits_with(i.depth(), |n| i.peek_at(n)) {
        return Ok(false);
    }
    let live = f.registers() as usize;
    if live > Fragment::MAX_REGISTERS as usize {
        return Err(Error::internal(
            "fragment declares more registers than Fragment::MAX_REGISTERS, \
             which Fragment::validate refuses; it reached frag::run unvalidated",
        ));
    }
    let mut regs = [0i64; Fragment::MAX_REGISTERS as usize];
    for op in f.ops() {
        match op {
            Op::PopInt => {
                // The guard established these are unboxed ints. This still
                // asks, because an `Op` list and a `Guard` are separate data;
                // `Fragment::validate` is what ties them, and this is the arm
                // that says so if it ever failed to.
                let Some(v) = i.pull().and_then(|v| v.as_int()) else {
                    return Err(Error::internal(
                        "fragment op PopInt found no unboxed integer after its guard admitted; \
                         the guard and the op list disagree about the arm's domain",
                    ));
                };
                // Within the declared file only, so the numbering is exactly
                // `Op::PopInt`'s: slot 0 is the value popped last.
                if let Some(file) = regs.get_mut(..live) {
                    file.rotate_right(1);
                    if let Some(slot) = file.first_mut() {
                        *slot = v;
                    }
                }
            }
            Op::PushInt(r) => {
                let Some(v) = regs.get(*r as usize).copied() else {
                    return Err(Error::internal(
                        "fragment op PushInt names a register outside the fragment's file",
                    ));
                };
                i.push(BundValue::int(v));
            }
            Op::DupTop => {
                let Some(v) = i.peek() else {
                    return Err(Error::internal(
                        "fragment op DupTop found an empty stack after its guard admitted",
                    ));
                };
                // `.dup()`, not a clone: `dup` gives the copy a fresh header
                // and therefore a fresh identity (F13,
                // `crates/bund2-stdlib/src/stack.rs`, `dup_one`). The
                // differential test caught this the first time it ran.
                i.push(v.dup());
            }
            Op::DropTop => {
                // Silent success here was the sixth review's S3: `drop drop`
                // on a one-deep stack would drop once and report `Ok(true)`,
                // where the words fail "too shallow". `Fragment::new` refuses
                // such a fragment; this arm says so if one ever arrives.
                if i.pull().is_none() {
                    return Err(Error::internal(
                        "fragment op DropTop found an empty stack after its guard admitted",
                    ));
                }
            }
            Op::AddInt { dst, a, b } => {
                let (Some(x), Some(y)) = (
                    regs.get(*a as usize).copied(),
                    regs.get(*b as usize).copied(),
                ) else {
                    return Err(Error::internal(
                        "fragment op AddInt names a register outside the fragment's file",
                    ));
                };
                if let Some(slot) = regs.get_mut(*dst as usize) {
                    *slot = x.wrapping_add(y);
                }
            }
            Op::SubInt { dst, a, b } => {
                let (Some(x), Some(y)) = (
                    regs.get(*a as usize).copied(),
                    regs.get(*b as usize).copied(),
                ) else {
                    return Err(Error::internal(
                        "fragment op SubInt names a register outside the fragment's file",
                    ));
                };
                if let Some(slot) = regs.get_mut(*dst as usize) {
                    *slot = x.wrapping_sub(y);
                }
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_ir::Guard;

    fn add_fragment() -> Fragment {
        Fragment::new(
            Guard::TopAreInt(2),
            vec![
                Op::PopInt,
                Op::PopInt,
                Op::AddInt { dst: 0, a: 0, b: 1 },
                Op::PushInt(0),
            ],
            2,
        )
        .expect("the add arm is well-formed")
    }

    #[test]
    fn the_add_arm_computes_what_the_word_computes() {
        let mut i = Interp::new();
        i.push(BundValue::int(2));
        i.push(BundValue::int(40));
        assert_eq!(run(&mut i, &add_fragment()).ok(), Some(true));
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(42));
        assert_eq!(i.depth(), 0);
    }

    /// **The property the whole design rests on.** A guard that does not admit
    /// must leave the stack exactly as it found it, or the word that runs
    /// instead sees operands the program never put there.
    #[test]
    fn a_declined_guard_does_not_disturb_the_stack() {
        let mut i = Interp::new();
        i.push(BundValue::int(1));
        i.push(BundValue::float(2.0));
        let before = i.snapshot();
        assert_eq!(run(&mut i, &add_fragment()).ok(), Some(false), "float must decline");
        assert_eq!(i.snapshot(), before);

        let mut i = Interp::new();
        i.push(BundValue::int(1));
        let before = i.snapshot();
        assert_eq!(run(&mut i, &add_fragment()).ok(), Some(false), "too shallow must decline");
        assert_eq!(i.snapshot(), before);
    }

    /// A boxed int is not an unboxed `Int`. The guard must decline rather than
    /// silently read through the box, because the fragment's ops do not.
    #[test]
    fn a_boxed_operand_declines() {
        let mut i = Interp::new();
        i.push(BundValue::int(1).promote());
        i.push(BundValue::int(2));
        let before = i.snapshot();
        assert_eq!(run(&mut i, &add_fragment()).ok(), Some(false));
        assert_eq!(i.snapshot(), before);
    }

    /// A fragment whose ops outrun its guard **cannot be built**.
    #[test]
    fn new_refuses_the_fragments_that_could_fail_after_the_guard() {
        assert!(
            Fragment::new(Guard::Depth(2), vec![Op::PopInt, Op::PopInt], 2).is_err(),
            "Depth guard cannot feed PopInt"
        );
        assert!(
            Fragment::new(Guard::TopAreInt(1), vec![Op::PopInt, Op::PopInt], 2).is_err(),
            "pops 2 under a guard checking 1"
        );
        assert!(
            Fragment::new(
                Guard::TopAreInt(2),
                vec![Op::PopInt, Op::PopInt, Op::AddInt { dst: 0, a: 0, b: 7 }],
                2
            )
            .is_err(),
            "register 7 of 2"
        );
        assert!(
            Fragment::new(Guard::Depth(1), vec![Op::DropTop, Op::DropTop], 0).is_err(),
            "drop drop under a one-deep guard"
        );
    }

    /// Run a fragment `new` would refuse, and require an internal error.
    fn assert_internal(bad: Fragment, depth: i64) {
        let mut i = Interp::new();
        for n in 0..depth {
            i.push(BundValue::int(n));
        }
        match run(&mut i, &bad) {
            Err(e) => assert!(e.is_internal(), "must route through the internal path"),
            Ok(v) => panic!("expected an internal error, got Ok({v})"),
        }
    }

    /// A post-guard failure is an **internal error**, never `Ok(false)` and
    /// never silent success. Built with `new_unchecked`, because `new` refuses
    /// every one of these — the executor's arms are the second line, and this
    /// is what they do if the first is ever bypassed.
    #[test]
    fn a_post_guard_failure_is_an_internal_error() {
        assert_internal(
            Fragment::new_unchecked(
                Guard::TopAreInt(2),
                vec![Op::PopInt, Op::PopInt, Op::PushInt(9)],
                2,
            ),
            2,
        );
        // The sixth review's case: the second drop found nothing and the old
        // executor reported success.
        assert_internal(
            Fragment::new_unchecked(Guard::Depth(1), vec![Op::DropTop, Op::DropTop], 0),
            1,
        );
    }
}
