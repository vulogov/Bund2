//! A word's **specialised arm**, expressed so a code generator can inline it.
//!
//! # Why this exists — RFC-0005 §S6, Q33
//!
//! A compiled word's signature is uniform `fn(&mut dyn Vm)`, because operands
//! travel on the shared VM stack rather than in the call — that is what keeps
//! `return_call` usable (§S8). The consequence is that a word reached through
//! that signature reads its operands off the real stack, so `1 2 +` avoids
//! materialising a `BundValue` only if `+` is **inlined** rather than called.
//!
//! Inlining needs a per-word lowering. Whether one is worth writing is
//! measured by `crates/bund2-bench/benches/fragment.rs` and reported in
//! RFC-0005 §S6 — **and only there**. An earlier version of this comment
//! carried its own copy of the figures, which disagreed with the RFC's table
//! within a day and survived the measurement it quoted being withdrawn.
//!
//! # Why it is not CLIF
//!
//! D9's amendment: the binding constraint is that **no Cranelift type may
//! appear in `bund2-stdlib` or `bund2-api`**. A `LowerFn` carrying a
//! `FunctionBuilder` would pin the stable surface to an exact Cranelift
//! version and pull the optional subsystem into the mandatory crate.
//!
//! So a word publishes *this* — an operation list that names no code generator
//! — and `bund2-jit` lowers it on its own side of the boundary. `bund2-ir` is
//! already a dependency of `bund2-jit`, and it depends on nothing but
//! `bund2-value`.
//!
//! # What a fragment is allowed to be
//!
//! **One arm, never a whole word.** A fragment for `+` cannot restate
//! `numeric_op`, which handles int, float, mixed kinds, LIST append, string
//! concatenation and division by zero. It does not have to: under §S5's
//! guard-and-branch rule the fragment covers the arm its [`Guard`] admits and
//! every other shape branches to the word itself.
//!
//! That is what bounds the divergence risk. A fragment can only be wrong on
//! the arm it claims, and RFC-0005 criterion 16 requires a differential test
//! that runs the arm and the word on the same inputs.

use bund2_value::BundValue;

/// What must hold for a fragment to be entered.
///
/// Deliberately a closed set rather than a predicate: a guard has to be
/// something a code generator can emit as a branch, not a Rust closure it
/// cannot see inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guard {
    /// The top `n` stack values are all unboxed `Int`.
    TopAreInt(u8),
    /// At least `n` values are present, whatever their kind.
    Depth(u8),
}

/// One operation in a fragment.
///
/// The set is small on purpose. It covers the arms measured to be worth
/// inlining and nothing speculative; an operation with no fragment using it is
/// a lowering nobody has tested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Pop an int into the fragment's register file.
    ///
    /// **Numbering is by pop order, reversed.** Each pop shifts the file up
    /// one slot and writes slot 0, so after `k` pops slot 0 holds the value
    /// popped *last* — the deepest — and slot `k - 1` holds what was the top.
    /// For `+` that is invisible, because addition commutes. For `-` it is not:
    /// the word takes the top as its *left* operand (`3 5 -` is `5 - 3`,
    /// `crates/bund2-stdlib/src/math.rs`'s header), so a `-` fragment is
    /// `SubInt { dst: 0, a: 1, b: 0 }`, and the `int_add` shape copied by
    /// analogy would compute the reverse of the word.
    PopInt,
    /// Push a register's int back to the stack.
    PushInt(u8),
    /// Duplicate the top of stack the way `dup` does — **a fresh header over
    /// a shared payload**, which mints a new identity (F13).
    ///
    /// Named `DupTop` and not `CopyTop` for a reason the differential test
    /// found on its first run: a naive `push(clone)` shares the identity, and
    /// the word does not. An arm that copies is not the arm `dup` implements.
    DupTop,
    /// Discard the top of stack.
    DropTop,
    /// `dst = a + b`, wrapping — the reference's `i64` arithmetic (D4).
    AddInt { dst: u8, a: u8, b: u8 },
    /// `dst = a - b`, wrapping. See [`Op::PopInt`] for which register holds
    /// which operand — the order is the whole difficulty. No fragment uses this
    /// yet, which by this enum's own rule makes it a lowering nobody has tested.
    SubInt { dst: u8, a: u8, b: u8 },
}

/// A word's specialised arm.
///
/// **The fields are private and the only constructor validates.** A fragment
/// that could fail after its guard admitted must not exist, because by then its
/// operands are gone and nothing can fall back to the word. The previous shape
/// — three `pub` fields and a `validate` a caller could forget — made that a
/// convention; RFC-0005's sixth review (S3) found no non-test path calling it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    guard: Guard,
    ops: Vec<Op>,
    registers: u8,
}

/// What the typed walk in [`Fragment::validate`] knows about one stack slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// An unboxed integer, as `TopAreInt` promises and `PushInt` produces.
    Int,
    /// Present, of no known kind — what `Depth` promises.
    Any,
}

impl Fragment {
    /// The most registers a fragment may declare. The register file is a
    /// fixed array so that entering a fragment allocates nothing.
    pub const MAX_REGISTERS: u8 = 8;

    /// Build a fragment, or say why it cannot exist.
    pub fn new(guard: Guard, ops: Vec<Op>, registers: u8) -> Result<Self, String> {
        let f = Self {
            guard,
            ops,
            registers,
        };
        f.validate()?;
        Ok(f)
    }

    /// Build a fragment **without** validating it — for tests that must hold
    /// an invalid one, to show the executor refuses it. Behind a feature no
    /// shipped crate enables.
    #[cfg(feature = "unchecked")]
    pub fn new_unchecked(guard: Guard, ops: Vec<Op>, registers: u8) -> Self {
        Self {
            guard,
            ops,
            registers,
        }
    }

    pub fn guard(&self) -> Guard {
        self.guard
    }

    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    pub fn registers(&self) -> u8 {
        self.registers
    }

    /// The guard, asked of a stack through `nth` — the value `n` places below
    /// the top — so the stack need not be copied to be asked.
    ///
    /// **The guard's one definition.** [`Fragment::admits`] and
    /// `bund2_interp::frag::run` both answer through this; two copies of a
    /// guard are two guards, and the reviews of this design have found three
    /// duplicated renderers diverge already.
    pub fn admits_with(&self, depth: usize, nth: impl Fn(usize) -> Option<BundValue>) -> bool {
        match self.guard {
            Guard::TopAreInt(n) => {
                let n = n as usize;
                depth >= n && (0..n).all(|k| matches!(nth(k), Some(BundValue::Int(_, _))))
            }
            Guard::Depth(n) => depth >= n as usize,
        }
    }

    /// Does this fragment's guard admit the given stack top?
    ///
    /// `top` is the stack **top-first**, which is the order a guard reasons
    /// about — `TopAreInt(2)` is about the two values `+` would pull.
    pub fn admits(&self, top: &[BundValue]) -> bool {
        self.admits_with(top.len(), |k| top.get(k).cloned())
    }

    /// How deep a stack must be for the guard to be *testable*.
    pub fn needs(&self) -> usize {
        match self.guard {
            Guard::TopAreInt(n) | Guard::Depth(n) => n as usize,
        }
    }

    /// Can this fragment fail after its guard admits? `Err` says how.
    ///
    /// **A typed walk of the ops against what the guard promises**, not a count
    /// of one op kind. The guard promises the top `n` values — as `Int` for
    /// `TopAreInt`, as anything for `Depth` — and each op is checked against
    /// that abstract stack as it is consumed and extended:
    ///
    /// - `PopInt` needs an `Int` there; `DropTop` needs *something*; `DupTop`
    ///   needs something and copies its kind. Every consuming op counts, so
    ///   `drop drop` under `Depth(1)` is refused rather than dropping twice
    ///   and silently doing nothing the second time.
    /// - A register must be **written before it is read**: `PushInt`,
    ///   `AddInt` and `SubInt` may only read registers an earlier `PopInt` or
    ///   arithmetic op wrote. The file starts zeroed, so an unwritten read
    ///   would push `0` and report success.
    /// - Every register named is inside the declared file, the file is at most
    ///   [`Fragment::MAX_REGISTERS`], and no more values are popped into it
    ///   than it holds.
    ///
    /// Written-ness follows `Op::PopInt`'s numbering: each pop shifts the file
    /// up one slot and writes slot 0.
    pub fn validate(&self) -> Result<(), String> {
        let regs = self.registers as usize;
        if self.registers > Self::MAX_REGISTERS {
            return Err(format!(
                "fragment declares {regs} registers; the file holds {}",
                Self::MAX_REGISTERS
            ));
        }
        let pops = self.ops.iter().filter(|o| matches!(o, Op::PopInt)).count();
        if pops > regs {
            return Err(format!(
                "fragment pops {pops} values into {regs} register(s); the earliest would be lost"
            ));
        }
        // Top of the abstract stack is the end of the Vec.
        let (n, kind) = match self.guard {
            Guard::TopAreInt(n) => (n as usize, Kind::Int),
            Guard::Depth(n) => (n as usize, Kind::Any),
        };
        let mut stack = vec![kind; n];
        let mut written = [false; Self::MAX_REGISTERS as usize];
        let inside = |r: u8| (r as usize) < regs;
        for (i, op) in self.ops.iter().enumerate() {
            match op {
                Op::PopInt => match stack.pop() {
                    Some(Kind::Int) => {
                        if let Some(file) = written.get_mut(..regs) {
                            file.rotate_right(1);
                            if let Some(slot) = file.first_mut() {
                                *slot = true;
                            }
                        }
                    }
                    Some(Kind::Any) => {
                        return Err(format!(
                            "op {i} (PopInt) reads an integer the guard does not promise"
                        ));
                    }
                    None => {
                        return Err(format!(
                            "op {i} (PopInt) consumes beyond the {n} value(s) the guard admits"
                        ));
                    }
                },
                Op::DropTop => {
                    if stack.pop().is_none() {
                        return Err(format!(
                            "op {i} (DropTop) consumes beyond the {n} value(s) the guard admits"
                        ));
                    }
                }
                Op::DupTop => {
                    let Some(top) = stack.last().copied() else {
                        return Err(format!("op {i} (DupTop) has nothing the guard admits to copy"));
                    };
                    stack.push(top);
                }
                Op::PushInt(r) => {
                    if !inside(*r) {
                        return Err(format!(
                            "op {i} (PushInt) names register {r}, outside a file of {regs}"
                        ));
                    }
                    if !written[*r as usize] {
                        return Err(format!(
                            "op {i} (PushInt) reads register {r} before anything writes it"
                        ));
                    }
                    stack.push(Kind::Int);
                }
                Op::AddInt { dst, a, b } | Op::SubInt { dst, a, b } => {
                    if let Some(bad) = [*dst, *a, *b].into_iter().find(|r| !inside(*r)) {
                        return Err(format!(
                            "op {i} ({op:?}) names register {bad}, outside a file of {regs}"
                        ));
                    }
                    if let Some(bad) = [*a, *b].into_iter().find(|r| !written[*r as usize]) {
                        return Err(format!(
                            "op {i} ({op:?}) reads register {bad} before anything writes it"
                        ));
                    }
                    written[*dst as usize] = true;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_are_int_rejects_a_boxed_or_wrong_kind_operand() {
        let f = Fragment {
            guard: Guard::TopAreInt(2),
            ops: vec![],
            registers: 0,
        };
        assert!(f.admits(&[BundValue::int(1), BundValue::int(2)]));
        assert!(!f.admits(&[BundValue::int(1), BundValue::float(2.0)]));
        assert!(!f.admits(&[BundValue::int(1)]), "too shallow");
        // A boxed int is *not* an unboxed `Int`, and the guard must say so:
        // the fragment's ops read the scalar arm directly.
        assert!(!f.admits(&[BundValue::int(1).promote(), BundValue::int(2)]));
    }

    /// **The two faults the sixth review found the old check passing.** Both
    /// succeeded silently at run time: a second drop on an empty stack, and a
    /// read of a register nothing wrote.
    #[test]
    fn every_consuming_op_counts_and_registers_are_written_before_read() {
        assert!(
            Fragment::new(Guard::Depth(1), vec![Op::DropTop, Op::DropTop], 0).is_err(),
            "drop drop under a one-deep guard"
        );
        assert!(
            Fragment::new(Guard::TopAreInt(1), vec![Op::PopInt, Op::PushInt(1)], 2).is_err(),
            "register 1 is never written"
        );
        assert!(
            Fragment::new(Guard::Depth(1), vec![Op::DupTop, Op::DropTop, Op::DropTop], 0).is_ok(),
            "dup then two drops consumes exactly what the guard and the dup provide"
        );
        assert!(
            Fragment::new(Guard::Depth(2), vec![Op::PopInt], 1).is_err(),
            "a Depth guard promises no integer"
        );
    }

    #[test]
    fn depth_admits_any_kind() {
        let f = Fragment {
            guard: Guard::Depth(1),
            ops: vec![Op::DupTop],
            registers: 0,
        };
        assert!(f.admits(&[BundValue::str("x")]));
        assert!(!f.admits(&[]));
    }
}
