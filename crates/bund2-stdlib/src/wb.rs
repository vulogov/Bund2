//! The workbench half of a word — what a `.` suffix actually means.
//!
//! # There is no single `.` contract, and this module refuses to invent one
//!
//! D24 states the `.` contract as "primary operand from the workbench,
//! secondary from the main stack, result to the workbench". That is *one* of
//! at least three shapes the reference uses, and a word written from the
//! description rather than from its own source lands on the wrong one:
//!
//! | shape | operand 1 | operand 2 | result | example |
//! |---|---|---|---|---|
//! | **D24's** | workbench | stack | workbench | `string.distance.` (`.../string/distance.rs:38-39,90-91`) |
//! | **mixed-in, stack-out** | workbench | stack | **stack** | `string.prefix.` (`.../string/prefix_suffix.rs:32-39,52`) |
//! | **all-workbench** | workbench | **workbench** | workbench | `head.` (`reference/rust_multistackvm/src/stdlib/values/value_carcdr.rs:64-67,80-86`) |
//!
//! F73 records the first two; the third turned up when the suffix variants of
//! `car`/`cdr`/`head`/`tail`/`at` were read. `cargo xtask coverage` calls these
//! "one mechanical paired test per base", and that is exactly the assumption
//! that produces a wrong word.
//!
//! So [`Side`] carries only "which side is this call", and every family names
//! its own operand sources and its own destination at the registration site.
//! Nothing here defaults.

use bund2_api::{Error, Vm};
use bund2_value::BundValue;

/// Which of a word's two forms is running — the reference's `StackOps`
/// (`reference/rust_multistackvm/src/stdlib/values/value_carcdr.rs:14-23`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Stack,
    Bench,
}

impl Side {
    /// Pull from whichever side this call is on.
    pub(crate) fn pull(self, vm: &mut dyn Vm) -> Option<BundValue> {
        match self {
            Side::Stack => vm.pull(),
            Side::Bench => vm.pull_workbench(),
        }
    }

    /// Push to whichever side this call is on.
    pub(crate) fn push(self, vm: &mut dyn Vm, v: BundValue) {
        match self {
            Side::Stack => vm.push(v),
            Side::Bench => vm.push_workbench(v),
        }
    }

    /// How deep this side is.
    pub(crate) fn depth(self, vm: &dyn Vm) -> usize {
        match self {
            Side::Stack => vm.depth(),
            Side::Bench => vm.workbench_depth(),
        }
    }

    /// The `.` that goes on the end of a word's error prefix, and nothing for
    /// the plain form. The reference spells the prefix out per registration
    /// (`value_carcdr.rs:150-171`); this composes it instead, which is the same
    /// string with one place to be wrong rather than thirty.
    pub(crate) fn dot(self) -> &'static str {
        match self {
            Side::Stack => "",
            Side::Bench => ".",
        }
    }
}

/// Guard one side's depth, with the reference's message.
///
/// **The message says "Stack" even for the workbench form.** That is not a slip
/// here: `stdlib_carcdr_base` writes `"Stack is too shallow for inline {}()"`
/// in *both* arms (`value_carcdr.rs:16,21`), and a golden captures the text.
pub(crate) fn shallow(vm: &dyn Vm, side: Side, n: usize, prefix: &str) -> Result<(), Error> {
    if side.depth(vm) < n {
        return Err(Error(format!("Stack is too shallow for inline {prefix}()")));
    }
    Ok(())
}

/// Pull, or report the absence the reference reports.
pub(crate) fn operand(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<BundValue, Error> {
    side.pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returned: NO DATA has been obtained")))
}
