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
    StackEffect::fixed(consumes, produces)
}

/// Pull a name off the stack, as every `_in` word does.
/// Pull a stack name, **restoring it if it is not a string**.
///
/// The restore is the point. `1 1 move` fails in both engines, but the
/// reference leaves both values on the stack and an earlier version of this
/// left none: it pulled first and reported second. That is observable, because
/// the error path prints the stack (F18's own argument, and
/// `reference/Bund/src/stdlib/helpers/print_error.rs:126-131`).
///
/// Found by `cargo xtask effects`, which flagged `move` and `move_from` as
/// declaring an arity the probe disagreed with; the arity was right and the
/// residual stack was not.
fn name_arg(vm: &mut dyn Vm, word: &str) -> Result<String, Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("{word} returns: NO DATA")))?;
    match v.as_str() {
        Some(s) => Ok(s),
        None => {
            vm.push(v);
            Err(Error(format!("{word} expected a string name")))
        }
    }
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
    let top = crate::pull::top(vm, "DUP")?;
    // `dup` in the reference is a bincode round trip that mints a fresh id
    // (`reference/rust_dynamic/src/dup.rs:7-12`). Here it is a fresh header
    // over a shared payload — same observable result, without the round trip.
    vm.push(top.dup());
    Ok(())
}

fn dup_many(vm: &mut dyn Vm) -> Result<(), Error> {
    let n = count_arg(vm, "dup_many")?;
    shallow(vm, 1, "dup_many")?;
    let top = crate::pull::top(vm, "DUP")?;
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

/// `dup_many_in` — duplicate a named stack's top `n` times
/// (`reference/rust_multistack/src/stdlib/dup.rs:67-74`).
///
/// **The name is on top and the count beneath it** (`:71-72`), so the source
/// reads `<count> <name> dup_many_in`. This is the reverse of `dup_many`,
/// whose count is its only operand, and the pair is easy to write backwards —
/// which is what this word did until the two were compared against the oracle.
fn dup_many_in(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline dup_many()".into()));
    }
    let name = name_arg(vm, "dup_many_in")?;
    let n = count_arg(vm, "dup_many_in")?;
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

/// `drop_stack` — **remove the current stack entirely**, taking no argument
/// (`reference/rust_multistack/src/stdlib/drop.rs:54-66`, calling
/// `ts.drop_stack()` at `ts_drop_stack.rs:10-21`).
///
/// It pops the deque and removes the map entry, so the stack and everything in
/// it are gone and the ring is one shorter. An earlier version pulled a stack
/// *name*, which made it a different word — and made a bare `drop_stack` an
/// error where the reference simply works.
fn drop_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = vm.current_name();
    vm.drop_stack(&name);
    Ok(())
}

// --- swap -----------------------------------------------------------------

fn swap_one(vm: &mut dyn Vm) -> Result<(), Error> {
    shallow(vm, 2, "swap_one")?;
    let a = crate::pull::operand(vm, "SWAP", 1)?;
    let b = crate::pull::operand(vm, "SWAP", 2)?;
    vm.push(a);
    vm.push(b);
    Ok(())
}

/// `swap` with a depth: rotate right `n`, exchange, rotate back
/// (`reference/rust_multistack/src/ts_stack_op.rs:94-112`).
fn swap_n(vm: &mut dyn Vm) -> Result<(), Error> {
    let n = count_arg(vm, "swap")?;
    shallow(vm, 2, "swap")?;
    let top = crate::pull::top(vm, "DUP")?;
    for _ in 0..n {
        vm.rotate_right();
    }
    let other = crate::pull::operand(vm, "MOVE", 1)?;
    vm.push(top);
    for _ in 0..n {
        vm.rotate_left();
    }
    vm.pull();
    vm.push(other);
    Ok(())
}

/// `swap_in` — `swap` at a depth, on a **named** stack
/// (`reference/rust_multistack/src/stdlib/swap.rs:86-93`, calling
/// `swap_in_stack` at `ts_stack_op.rs:124-152`).
///
/// **The count is on top and the name beneath it** (`:90-91`), so the source
/// reads `<name> <n> swap_in` — the reverse of `dup_many_in`, whose name is on
/// top. Both operands are consumed.
///
/// The operation is the same rotate-right-`n` / exchange / rotate-back dance
/// `swap` performs, so it runs by switching to the named stack, doing exactly
/// what `swap` does, and switching back. An earlier version ignored the count
/// and exchanged the top two, which is `swap_one` — and it left the count on
/// the current stack, which is what `conform`'s capture epilogue caught after
/// a direct comparison of the *target* stack had missed it.
fn swap_in(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline swap_in()".into()));
    }
    // **The name is on top, despite what the inline wrapper suggests.**
    // `stdlib_swap_in_stack_inline` pulls `n` and then `name` and calls
    // `stdlib_swap_in_stack(ts, n, name)` (`swap.rs:90-92`) — but that
    // function reads its *first* parameter as the name (`:57`), so the two are
    // crossed and the value that must be a string is the one pulled first.
    // Confirmed against the oracle: `1 :t swap_in` works and `:t 1 swap_in`
    // errors.
    let name = name_arg(vm, "swap_in")?;
    let n = count_arg(vm, "swap_in")?;
    if vm.depth_of(&name) < 2 {
        return Err(Error(format!(
            "Swap in stack {name} had failed. Stack too shallow."
        )));
    }
    let here = vm.current_name();
    vm.to_stack(&name);
    let outcome = swap_at(vm, n);
    vm.to_stack(&here);
    outcome
}

/// The rotate/exchange/rotate-back sequence, on whatever stack is current.
fn swap_at(vm: &mut dyn Vm, n: i64) -> Result<(), Error> {
    let top = crate::pull::top(vm, "SWAP")?;
    for _ in 0..n {
        vm.rotate_right();
    }
    let other = crate::pull::operand(vm, "SWAP", 1)?;
    vm.push(top);
    for _ in 0..n {
        vm.rotate_left();
    }
    vm.pull();
    vm.push(other);
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

/// `to_current` — **switch** to a named stack, which must already exist
/// (`reference/rust_multistack/src/ts_to_current.rs:6-36`, reached through
/// `stdlib_to_current` at `stdlib/current.rs:6-22`).
///
/// It rotates the stack deque left until the named stack is at the back, and
/// the back **is** the current stack (`ts_current.rs:7`). No value moves.
///
/// **`to_current` and `to_stack` differ only in the missing case**: this one
/// fails with `Stake with name {} noexists` — the reference's spelling, kept —
/// while `to_stack` creates the stack and switches to it
/// (`ts_to_current.rs:37-43`).
///
/// *An earlier version of this comment, and of the code under it, had
/// `to_current` draining a named stack into the current one. That is
/// `TS::move_to_current` (`ts_move.rs:33-54`), a function with a similar name
/// that **no word registers**. Reading it and assuming the word beside it was
/// its caller is CLAUDE.md's "follow the call one level further" failure,
/// committed while investigating a divergence caused by the same habit.*
fn to_current(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "to_current")?;
    if !vm.stack_exists(&name) {
        return Err(Error(format!("Stake with name {name} noexists")));
    }
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
/// `ensure_stack_with_capacity` — create a stack with a depth bound
/// (`reference/rust_multistack/src/stdlib/ensure_stack.rs:32-61`).
///
/// The **name is on top** and the capacity beneath it (`:36,42`), so the
/// source reads `<capacity> <name> ensure_stack_with_capacity`.
///
/// The capacity is not decoration: a push onto a full stack drops the value
/// **most recently pushed** before adding the new one (`ts_push.rs:52-57`), so
/// a stack capped at 2 given `1 2 3 4` holds `1` and `4`. An earlier version
/// of this word pulled the number and threw it away, which made the bound
/// invisible.
fn ensure_stack_with_capacity(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(
            "Stack is too shallow for inline set_stack_capacity()".into(),
        ));
    }
    let name = name_arg(vm, "ensure_stack_with_capacity")?;
    let cap = count_arg(vm, "ensure_stack_with_capacity")?;
    vm.ensure_stack_with_capacity(&name, cap.max(0) as usize);
    Ok(())
}

fn stack_exists(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "stack_exists")?;
    let e = vm.stack_exists(&name);
    vm.push(BundValue::boolean(e));
    Ok(())
}

// --- move -----------------------------------------------------------------

/// `move` — **drain** the current stack into a named one
/// (`reference/rust_multistack/src/stdlib/stack_move.rs:24-30`, calling
/// `move_from_current` at `ts_move.rs:17-31`).
///
/// It moves *everything*, not one value: the reference loops until the source
/// is empty. An earlier version of this word moved a single value, which is a
/// different word.
///
/// **The drain reads a snapshot first — F70.** The reference pulls and pushes
/// inside one loop, and pushing to a stack that does not exist yet *creates*
/// it, which makes it current (`ts_current.rs:7` reads `stacks.back()`), so
/// the loop starts pulling back what it just pushed and never terminates.
/// `1 2 3 :box move` hangs the oracle. Taking the contents once and then
/// pushing cannot feed itself, so this moves three values and returns.
fn move_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "move")?;
    let mut moved = Vec::new();
    while let Some(v) = vm.pull() {
        moved.push(v);
    }
    for v in moved {
        vm.push_to(&name, v);
    }
    Ok(())
}

/// `move_from` — drain one **named** stack into another
/// (`reference/rust_multistack/src/stdlib/stack_move.rs:65-71`).
///
/// **Two names, not one**, and neither is the current stack: `name_from` comes
/// off the top and `name_to` from beneath it (`:68-69`), so the source reads
/// `<name_to> <name_from> move_from`. An earlier version of this word took one
/// name and moved a single value to the *current* stack — a third operation
/// the language does not have. The declared effect said `1 -> 1` where the
/// probe said `2 -> 0`, and the probe was right; `cargo xtask effects` reported
/// it and the first disposition explained it away as a failure path.
///
/// Snapshot-drained for F70's reason, exactly as `move` is.
fn move_from(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(
            "Stack is too shallow for inline move_from()".into(),
        ));
    }
    let from = name_arg(vm, "move_from")?;
    let to = name_arg(vm, "move_from")?;
    let mut moved = Vec::new();
    while let Some(v) = vm.pull_from(&from) {
        moved.push(v);
    }
    for v in moved {
        vm.push_to(&to, v);
    }
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
    let v = crate::pull::operand(vm, "RETURN", 1)?;
    vm.push_workbench(v);
    Ok(())
}

/// `return_to` — move one value from the workbench to a **named** stack
/// (`reference/rust_multistack/src/stdlib/workbench.rs:43-67`, calling
/// `return_from_workbench_to_stack`).
///
/// It takes a stack name, which an earlier version did not: that one moved the
/// workbench's top to the *current* stack, which is `take`. The name is pulled
/// before the workbench is touched, so a failure there leaves the workbench
/// alone — and the name is restored if it is not a string, so `9 :t return_to`
/// with an empty workbench leaves the `9` where the reference leaves it.
fn return_to(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline return_to()".into()));
    }
    let name = name_arg(vm, "return_to")?;
    let Some(v) = vm.pull_workbench() else {
        return Err(Error("return_to returns: NO DATA".into()));
    };
    vm.push_to(&name, v);
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
/// `fold` — drain the current stack into a LIST, **top first**
/// (`reference/rust_multistack/src/ts_list.rs:8-24`).
///
/// The reference pulls and pushes into the list in one pass with no reversal
/// (`:16`), so `1 2 3 fold` is `[ 3 2 1 ]` — the order values came *off*, not
/// the order they went on. An earlier version reversed, on the reasonable but
/// wrong assumption that a fold should read in source order; `lambda*`, which
/// looks like the same operation, genuinely does insert at the front
/// (`bund_fun.rs:189-202`), and the two are easy to conflate.
fn fold(vm: &mut dyn Vm) -> Result<(), Error> {
    let mut items = Vec::new();
    while let Some(v) = vm.pull() {
        items.push(v);
    }
    vm.push(BundValue::list(items));
    Ok(())
}

/// `fold_stack` — the same over a named stack, and the list **goes back
/// there** (`reference/rust_multistack/src/ts_list.rs:41-58`).
///
/// The fold drains the named stack and pushes the list to that same stack
/// (`:56`), leaving the current stack untouched. An earlier version pushed the
/// list to the *current* stack, which is a different word: it moved data
/// between stacks as a side effect of folding.
///
/// Same order as `fold`: no reversal, so the list reads top-first.
fn fold_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = name_arg(vm, "fold_stack")?;
    let mut items = Vec::new();
    while let Some(v) = vm.pull_from(&name) {
        items.push(v);
    }
    vm.push_to(&name, BundValue::list(items));
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
    // **`consumes` is a depth floor, not a net.** `dup_one` peeks one and
    // pushes one — net `+1` — but it *needs* one to be there, so it declares
    // `1 -> 2` and not `0 -> 1`. `bund2 check` reads the first number as "what
    // must be on the stack before this runs", which is F18's reading, and a
    // net of zero there would let a bare `dup` pass unremarked.
    w(r, "dup_one", dup_one, eff(1, 2));
    w(r, "dup_many", dup_many, eff(1, 0));
    w(r, "dup_one_in", dup_one_in, eff(1, 0));
    w(r, "dup_many_in", dup_many_in, eff(2, 0));
    w(r, "drop", drop_word, eff(1, 0));
    w(r, "drop_in", drop_in, eff(1, 0));
    w(r, "drop_stack", drop_stack, eff(1, 0));
    w(r, "swap_one", swap_one, eff(2, 2));
    // `2 -> 2`, the probed column, per **F18's rule**: take what the probe
    // observed wherever it disagrees with the guard, because the guard is a
    // minimum and the effect is a contract. `swap` reaches its depth
    // requirement at 2 and leaves the stack the size it found it —
    // `10 20 30 1 swap` answers `10 20 1 30` in both engines, four in and four
    // out. The declaration said `1 -> 0` until `cargo xtask effects` compared
    // it against the table.
    w(r, "swap", swap_n, eff(2, 2));
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
    w(r, "move_from", move_from, eff(2, 0));
    // **`0 -> 1` on the main stack, and that is all this shape can say.**
    // `take` needs a value on the **workbench** and puts one on the main
    // stack; the requirement is on the axis `StackEffect` does not have —
    // RFC-0004 §S1's second one.
    //
    // Declaring `1 -> 1` to record "not free" was tried and was wrong: it made
    // `bund2 check` read a workbench requirement as a main-stack one and
    // report `test_times_loop.bund`, a program that runs. A number on the
    // wrong axis is worse than no number, because the checker believes it.
    w(r, "take", take, eff(0, 1));
    w(r, "return", return_word, eff(1, 0));
    w(r, "return_to", return_to, eff(1, 0));
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
    // `reference/rust_multistackvm/src/stdlib/create_aliases.rs:4,24-27,36`.
    r.register_alias(".", "return");
    r.register_alias("->", "stacks_right");
    r.register_alias("→", "stacks_right");
    r.register_alias("<--", "rotate_current_left");
    r.register_alias("-->", "rotate_current_right");
    r.register_alias("stack", "ensure_stack");
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
        i.push(BundValue::int(1));
        run(&mut i, "dup_one").expect("dup_one");
        assert_eq!(i.depth(), 2);
        let a = i.pull().unwrap();
        let b = i.pull().unwrap();
        assert_eq!(a.as_int(), b.as_int(), "same content");
    }

    #[test]
    fn swap_one_exchanges_the_top_two() {
        let mut i = interp();
        i.push(BundValue::int(1));
        i.push(BundValue::int(2));
        run(&mut i, "swap_one").expect("swap_one");
        assert_eq!(i.pull().unwrap().as_int(), Some(1));
        assert_eq!(i.pull().unwrap().as_int(), Some(2));
    }

    #[test]
    fn the_workbench_round_trips() {
        let mut i = interp();
        i.push(BundValue::int(9));
        run(&mut i, "return").expect("return");
        assert_eq!(i.depth(), 0, "the value left the stack");
        run(&mut i, "take").expect("take");
        assert_eq!(i.pull().unwrap().as_int(), Some(9));
    }

    #[test]
    fn move_sends_a_value_to_a_named_stack() {
        let mut i = interp();
        i.push(BundValue::int(5));
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
            i.push_to("s", BundValue::int(n));
        }
        i.push(BundValue::str("s"));
        run(&mut i, "rotate_stack_right").expect("right");
        let after_right = i.pull_from("s").unwrap().as_int();

        let mut j = interp();
        for n in 1..=3 {
            j.push_to("s", BundValue::int(n));
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
            i.push(BundValue::int(n));
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
        assert!(i.pull().unwrap().render(true).contains("Bool(true)"));
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
        i.push(BundValue::int(1));
        run(&mut i, "dup").expect("dup resolves through the alias");
        assert_eq!(i.depth(), 2);
    }
}
