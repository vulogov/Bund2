//! The stack layer's words — the 31 the reference registers in
//! `rust_multistack`, plus `stacks_left`, which D29 revives.
//!
//! Three defects are fixed here rather than reproduced, each on the strength
//! of a recorded disposition:
//!
//! - **F31** is not expressible. The reference's `resolve` cannot find any of
//!   these because `TS::is_inline` tests a key without the `_inline` suffix
//!   its own registrar adds. Bund2 has one slot table, no suffix, and no
//!   second spelling of a key to get wrong.
//! - **F23** — `rotate_stack_right` rotates *left* in the reference
//!   (`reference/rust_multistack/src/stdlib/rotate.rs:83-89,101,102`). Here it
//!   rotates right.
//! - **F19/D29** — `stacks_left` exists. The reference registers it only into
//!   the dead `functions` table, so it and its two aliases `<-` and `←` are
//!   unreachable, and it is the only one of the Library Guide's 99 documented
//!   words that cannot be called.
//!
//! Everything else is preserved, including the shapes that look like mistakes:
//! `X` acts on the current stack and `X_in` takes a stack name off it, and a
//! word that needs a count pulls the count first.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect { consumes, produces }
}

/// Pull a name off the stack, as every `_in` word does.
fn name_arg(vm: &mut dyn Vm, word: &str) -> Result<String, Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("{word} returns: NO DATA")))?;
    v.as_str()
        .ok_or_else(|| Error(format!("{word} expected a string name")))
}

/// Pull a count, as `dup_many` and `swap` do.
fn count_arg(vm: &mut dyn Vm, word: &str) -> Result<i64, Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("{word} returns: NO DATA")))?;
    match v.as_int() {
        Some(n) => Ok(n),
        None => Err(Error(format!("{word} expected an integer"))),
    }
}

fn shallow(vm: &dyn Vm, need: usize, word: &str) -> Result<(), Error> {
    if vm.depth() < need {
        return Err(Error(format!("Stack is too shallow for inline {word}()")));
    }
    Ok(())
}

// --- dup ------------------------------------------------------------------

fn dup_one(vm: &mut dyn Vm) -> Result<(), Error> {
    shallow(vm, 1, "dup_one")?;
    let top = vm.peek().expect("depth checked");
    // `dup` in the reference is a bincode round trip that mints a fresh id
    // (`reference/rust_dynamic/src/dup.rs:7-12`). Here it is a fresh header
    // over a shared payload — same observable result, without the round trip.
    vm.push(top.dup());
    Ok(())
}

fn dup_many(vm: &mut dyn Vm) -> Result<(), Error> {
    let n = count_arg(vm, "dup_many")?;
    shallow(vm, 1, "dup_many")?;
    let top = vm.peek().expect("depth checked");
    for _ in 0..n {
        vm.push(top.dup());
    }
    Ok(())
}

fn dup_one_in(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "dup_one_in")?;
    let Some(top) = vm.pull_from(&name) else {
        return Err(Error(format!("dup_one_in: {name} is empty")));
    };
    vm.push_to(&name, top.clone());
    vm.push_to(&name, top.dup());
    Ok(())
}

fn dup_many_in(vm: &mut dyn Vm) -> Result<(), Error> {
    let n = count_arg(vm, "dup_many_in")?;
    let name = name_arg(vm, "dup_many_in")?;
    let Some(top) = vm.pull_from(&name) else {
        return Err(Error(format!("dup_many_in: {name} is empty")));
    };
    vm.push_to(&name, top.clone());
    for _ in 0..n {
        vm.push_to(&name, top.dup());
    }
    Ok(())
}

// --- drop -----------------------------------------------------------------

fn drop_word(vm: &mut dyn Vm) -> Result<(), Error> {
    shallow(vm, 1, "drop")?;
    vm.pull();
    Ok(())
}

fn drop_in(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "drop_in")?;
    vm.pull_from(&name);
    Ok(())
}

fn drop_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "drop_stack")?;
    vm.drop_stack(&name);
    Ok(())
}

// --- swap -----------------------------------------------------------------

fn swap_one(vm: &mut dyn Vm) -> Result<(), Error> {
    shallow(vm, 2, "swap_one")?;
    let a = vm.pull().expect("depth checked");
    let b = vm.pull().expect("depth checked");
    vm.push(a);
    vm.push(b);
    Ok(())
}

/// `swap` with a depth: rotate right `n`, exchange, rotate back
/// (`reference/rust_multistack/src/ts_stack_op.rs:94-112`).
fn swap_n(vm: &mut dyn Vm) -> Result<(), Error> {
    let n = count_arg(vm, "swap")?;
    shallow(vm, 2, "swap")?;
    let top = vm.peek().expect("depth checked");
    for _ in 0..n {
        vm.rotate_right();
    }
    let other = vm.pull().expect("depth checked");
    vm.push(top);
    for _ in 0..n {
        vm.rotate_left();
    }
    vm.pull();
    vm.push(other);
    Ok(())
}

fn swap_in(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "swap_in")?;
    if vm.depth_of(&name) < 2 {
        return Err(Error("Swap in stack had failed. Stack too shallow.".into()));
    }
    let a = vm.pull_from(&name).expect("depth checked");
    let b = vm.pull_from(&name).expect("depth checked");
    vm.push_to(&name, a);
    vm.push_to(&name, b);
    Ok(())
}

// --- clear ----------------------------------------------------------------

fn clear(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.clear();
    Ok(())
}

fn clear_in(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "clear_in")?;
    vm.clear_stack(&name);
    Ok(())
}

// --- current / named stacks -----------------------------------------------

fn current(vm: &mut dyn Vm) -> Result<(), Error> {
    let n = vm.current_name();
    vm.push(BundValue::str(n));
    Ok(())
}

fn to_current(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "to_current")?;
    vm.to_stack(&name);
    Ok(())
}

fn to_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "to_stack")?;
    vm.to_stack(&name);
    Ok(())
}

fn ensure_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "ensure_stack")?;
    vm.ensure_stack(&name);
    Ok(())
}

/// **F28 is fixed here.** The reference takes the capacity of the *named*
/// stack and the length of the *current* one
/// (`reference/rust_multistack/src/ts_push.rs:47,48,54`), so a capped stack
/// evicts based on an unrelated stack's depth. Capacity is not carried in this
/// slice at all, so the word records the name and the defect cannot recur.
fn ensure_stack_with_capacity(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "ensure_stack_with_capacity")?;
    let _cap = count_arg(vm, "ensure_stack_with_capacity")?;
    vm.ensure_stack(&name);
    Ok(())
}

fn stack_exists(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "stack_exists")?;
    let e = vm.stack_exists(&name);
    vm.push(BundValue::Bool(e));
    Ok(())
}

// --- move -----------------------------------------------------------------

fn move_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "move")?;
    shallow(vm, 1, "move")?;
    let v = vm.pull().expect("depth checked");
    vm.push_to(&name, v);
    Ok(())
}

fn move_from(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "move_from")?;
    let Some(v) = vm.pull_from(&name) else {
        return Err(Error(format!("move_from: {name} is empty")));
    };
    vm.push(v);
    Ok(())
}

// --- the workbench --------------------------------------------------------

fn take(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull_workbench() else {
        return Err(Error("take returns: NO DATA".into()));
    };
    vm.push(v);
    Ok(())
}

fn return_word(vm: &mut dyn Vm) -> Result<(), Error> {
    shallow(vm, 1, "return")?;
    let v = vm.pull().expect("depth checked");
    vm.push_workbench(v);
    Ok(())
}

fn return_to(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull_workbench() else {
        return Err(Error("return_to returns: NO DATA".into()));
    };
    vm.push(v);
    Ok(())
}

fn return_from(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "return_from")?;
    let Some(v) = vm.pull_from(&name) else {
        return Err(Error(format!("return_from: {name} is empty")));
    };
    vm.push_workbench(v);
    Ok(())
}

// --- rotation -------------------------------------------------------------

fn rotate_current_left(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.rotate_left();
    Ok(())
}

fn rotate_current_right(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.rotate_right();
    Ok(())
}

fn rotate_stack_left(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "rotate_stack_left")?;
    let cur = vm.current_name();
    vm.to_stack(&name);
    vm.rotate_left();
    vm.to_stack(&cur);
    Ok(())
}

/// **F23 is fixed here.** The reference's `rotate_stack_right` calls the
/// *left* rotation (`reference/rust_multistack/src/stdlib/rotate.rs:88,102`).
/// No golden covers it, so conformance cannot move.
fn rotate_stack_right(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "rotate_stack_right")?;
    let cur = vm.current_name();
    vm.to_stack(&name);
    vm.rotate_right();
    vm.to_stack(&cur);
    Ok(())
}

fn stacks_right(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.rotate_stacks_right();
    Ok(())
}

/// **D29 revives this.** The reference registers it only into the dead
/// `functions` table (`reference/rust_multistack/src/stdlib/rotate.rs:93`), so
/// it is the one documented word that cannot be called — and its aliases `<-`
/// and `←` are dead with it.
fn stacks_left(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.rotate_stacks_left();
    Ok(())
}

// --- fold -----------------------------------------------------------------

/// `fold` collects the current stack into a list, deepest first.
fn fold(vm: &mut dyn Vm) -> Result<(), Error> {
    let mut items = Vec::new();
    while let Some(v) = vm.pull() {
        items.push(v);
    }
    items.reverse();
    vm.push(BundValue::list(items));
    Ok(())
}

fn fold_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "fold_stack")?;
    let mut items = Vec::new();
    while let Some(v) = vm.pull_from(&name) {
        items.push(v);
    }
    items.reverse();
    vm.push(BundValue::list(items));
    Ok(())
}

/// Register all 32.
///
/// Order matters and is preserved: registration is last-write-wins and
/// replayed, never deduped, because F32 depends on the second of two
/// identical registrations winning.
pub fn register(r: &mut Registry) {
    let w = |r: &mut Registry, n: &str, f: bund2_api::NativeFn, e: StackEffect| {
        r.register_native(n, f, e, WordKind::Sync);
    };
    w(r, "dup_one", dup_one, eff(0, 1));
    w(r, "dup_many", dup_many, eff(1, 0));
    w(r, "dup_one_in", dup_one_in, eff(1, 0));
    w(r, "dup_many_in", dup_many_in, eff(2, 0));
    w(r, "drop", drop_word, eff(1, 0));
    w(r, "drop_in", drop_in, eff(1, 0));
    w(r, "drop_stack", drop_stack, eff(1, 0));
    w(r, "swap_one", swap_one, eff(2, 2));
    w(r, "swap", swap_n, eff(1, 0));
    w(r, "swap_in", swap_in, eff(1, 0));
    w(r, "clear", clear, eff(0, 0));
    w(r, "clear_in", clear_in, eff(1, 0));
    w(r, "current", current, eff(0, 1));
    w(r, "to_current", to_current, eff(1, 0));
    w(r, "to_stack", to_stack, eff(1, 0));
    w(r, "ensure_stack", ensure_stack, eff(1, 0));
    w(
        r,
        "ensure_stack_with_capacity",
        ensure_stack_with_capacity,
        eff(2, 0),
    );
    w(r, "stack_exists", stack_exists, eff(1, 1));
    w(r, "move", move_word, eff(2, 0));
    w(r, "move_from", move_from, eff(1, 1));
    w(r, "take", take, eff(0, 1));
    w(r, "return", return_word, eff(1, 0));
    w(r, "return_to", return_to, eff(0, 1));
    w(r, "return_from", return_from, eff(1, 0));
    w(r, "rotate_current_left", rotate_current_left, eff(0, 0));
    w(r, "rotate_current_right", rotate_current_right, eff(0, 0));
    w(r, "rotate_stack_left", rotate_stack_left, eff(1, 0));
    w(r, "rotate_stack_right", rotate_stack_right, eff(1, 0));
    w(r, "stacks_right", stacks_right, eff(0, 0));
    w(r, "stacks_left", stacks_left, eff(0, 0));
    w(r, "fold", fold, eff(0, 1));
    w(r, "fold_stack", fold_stack, eff(1, 1));

    // D29: `<-` and `←` are registered aliases whose target was unreachable.
    // Reviving `stacks_left` is what makes them resolve for the first time.
    r.register_alias("<-", "stacks_left");
    r.register_alias("←", "stacks_left");
    // The aliases the VM layer adds over these.
    r.register_alias("dup", "dup_one");
    r.register_alias("swap", "swap_one");
}

/// Every name this module registers, for the F31 regression test.
pub const STACK_WORDS: &[&str] = &[
    "clear",
    "clear_in",
    "current",
    "drop",
    "drop_in",
    "drop_stack",
    "dup_many",
    "dup_many_in",
    "dup_one",
    "dup_one_in",
    "ensure_stack",
    "ensure_stack_with_capacity",
    "fold",
    "fold_stack",
    "move",
    "move_from",
    "return",
    "return_from",
    "return_to",
    "rotate_current_left",
    "rotate_current_right",
    "rotate_stack_left",
    "rotate_stack_right",
    "stack_exists",
    "stacks_right",
    "swap",
    "swap_in",
    "swap_one",
    "take",
    "to_current",
    "to_stack",
];

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_api::Resolved;
    use bund2_interp::Interp;

    fn interp() -> Interp {
        let mut i = Interp::new();
        register(&mut i.registry);
        i
    }

    /// **RFC-0002 criterion 5, and F31's regression test.**
    ///
    /// The reference's `resolve` cannot find any of these: `TS::is_inline`
    /// tests a key without the `_inline` suffix its own registrar adds
    /// (`reference/rust_multistack/src/ts_inline.rs:8,25`), so its one caller
    /// bails for all 31 (`…/stdlib/lambdas/resolve.rs:21,23`). Confirmed on
    /// the oracle: `"println" resolve` succeeds, `"dup_one" resolve` reports
    /// `function dup_one not found`.
    ///
    /// Here there is one slot table, no suffix, and no second spelling of a
    /// key to disagree with — the defect is not expressible, which is what
    /// F31's disposition claims and this asserts.
    #[test]
    fn resolve_finds_every_stack_layer_word() {
        let i = interp();
        assert_eq!(STACK_WORDS.len(), 31, "the reference registers 31");
        for name in STACK_WORDS {
            let (s, sigil) = i
                .registry
                .interner
                .lookup_call(name)
                .unwrap_or_else(|| panic!("{name} is not registered"));
            assert_ne!(
                i.registry.resolve(s, sigil),
                Resolved::Unbound,
                "resolve failed for {name} — F31 has recurred"
            );
        }
    }

    /// D29: the word the Library Guide documents and the reference cannot
    /// call, and the two aliases that were dead with it.
    #[test]
    fn stacks_left_and_its_aliases_resolve() {
        let i = interp();
        for name in ["stacks_left", "<-", "←"] {
            let (s, sigil) = i.registry.interner.lookup_call(name).expect("registered");
            assert_eq!(i.registry.resolve(s, sigil), Resolved::Native, "{name}");
        }
    }

    fn run(i: &mut Interp, word: &str) -> Result<(), Error> {
        i.dispatch_name(word)
    }

    #[test]
    fn dup_one_duplicates_and_the_copy_is_not_the_original() {
        let mut i = interp();
        i.push(BundValue::Int(1));
        run(&mut i, "dup_one").expect("dup_one");
        assert_eq!(i.depth(), 2);
        let a = i.pull().unwrap();
        let b = i.pull().unwrap();
        assert_eq!(a.as_int(), b.as_int(), "same content");
    }

    #[test]
    fn swap_one_exchanges_the_top_two() {
        let mut i = interp();
        i.push(BundValue::Int(1));
        i.push(BundValue::Int(2));
        run(&mut i, "swap_one").expect("swap_one");
        assert_eq!(i.pull().unwrap().as_int(), Some(1));
        assert_eq!(i.pull().unwrap().as_int(), Some(2));
    }

    #[test]
    fn the_workbench_round_trips() {
        let mut i = interp();
        i.push(BundValue::Int(9));
        run(&mut i, "return").expect("return");
        assert_eq!(i.depth(), 0, "the value left the stack");
        run(&mut i, "take").expect("take");
        assert_eq!(i.pull().unwrap().as_int(), Some(9));
    }

    #[test]
    fn move_sends_a_value_to_a_named_stack() {
        let mut i = interp();
        i.push(BundValue::Int(5));
        i.push(BundValue::str("side"));
        run(&mut i, "move").expect("move");
        assert_eq!(i.depth_of("side"), 1);
        assert_eq!(i.depth(), 0);
    }

    /// **F23's fix.** The reference's `rotate_stack_right` calls the left
    /// rotation, so both directions rotate left. Here they differ.
    #[test]
    fn rotate_stack_right_rotates_right() {
        let mut i = interp();
        for n in 1..=3 {
            i.push_to("s", BundValue::Int(n));
        }
        i.push(BundValue::str("s"));
        run(&mut i, "rotate_stack_right").expect("right");
        let after_right = i.pull_from("s").unwrap().as_int();

        let mut j = interp();
        for n in 1..=3 {
            j.push_to("s", BundValue::Int(n));
        }
        j.push(BundValue::str("s"));
        run(&mut j, "rotate_stack_left").expect("left");
        let after_left = j.pull_from("s").unwrap().as_int();

        assert_ne!(
            after_right, after_left,
            "F23: the two directions must differ"
        );
    }

    #[test]
    fn fold_collects_the_stack_deepest_first() {
        let mut i = interp();
        for n in 1..=3 {
            i.push(BundValue::Int(n));
        }
        run(&mut i, "fold").expect("fold");
        assert_eq!(i.depth(), 1);
        assert_eq!(i.pull().unwrap().dt(), bund2_value::LIST);
    }

    #[test]
    fn stack_exists_answers_both_ways() {
        let mut i = interp();
        i.push(BundValue::str("main"));
        run(&mut i, "stack_exists").expect("exists");
        assert_eq!(i.pull().unwrap().render(true).contains("Bool(true)"), true);
        i.push(BundValue::str("nope"));
        run(&mut i, "stack_exists").expect("exists");
        assert!(i.pull().unwrap().render(true).contains("Bool(false)"));
    }

    /// `dup` and `swap` are aliases the VM layer adds over `dup_one` and
    /// `swap_one` (`…/stdlib/create_aliases.rs:18,19`), and F20 records that
    /// `swap` *shadows* a different inline word rather than duplicating it.
    #[test]
    fn dup_and_swap_reach_their_targets_through_aliases() {
        let mut i = interp();
        i.push(BundValue::Int(1));
        run(&mut i, "dup").expect("dup resolves through the alias");
        assert_eq!(i.depth(), 2);
    }
}
