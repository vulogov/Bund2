//! Pulling operands without asserting.
//!
//! Every word here guards its depth before pulling, so the pulls that follow
//! "cannot" fail. Saying that with `expect` puts a panic on an unreachable
//! path — and an interpreter that aborts takes the user's program state with
//! it and explains nothing.
//!
//! The reference does not assert either. It writes the guard *and* a real
//! failure arm, `SET returns: NO DATA #1`
//! (`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:30-42`), for
//! exactly the same pulls. Reproducing that arm is both safer and more
//! faithful than asserting it away.

use bund2_api::{Error, Vm};
use bund2_value::BundValue;

/// Pull one operand, or report that the stack was not as deep as the guard
/// believed. `n` is 1-based and counts from the top, as the reference's
/// `NO DATA #n` does.
pub(crate) fn operand(vm: &mut dyn Vm, word: &str, n: usize) -> Result<BundValue, Error> {
    vm.pull()
        .ok_or_else(|| Error(format!("{word} returns: NO DATA #{n}")))
}

/// Pull from a named stack, same contract.
pub(crate) fn operand_from(
    vm: &mut dyn Vm,
    stack: &str,
    word: &str,
    n: usize,
) -> Result<BundValue, Error> {
    vm.pull_from(stack)
        .ok_or_else(|| Error(format!("{word} returns: NO DATA #{n}")))
}

/// The top without consuming it, or the same report.
pub(crate) fn top(vm: &mut dyn Vm, word: &str) -> Result<BundValue, Error> {
    vm.peek()
        .ok_or_else(|| Error(format!("{word} returns: NO DATA #1")))
}
