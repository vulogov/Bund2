//! **BundIR to CLIF — the first lowering.** RFC-0005 §S6, criterion 16.
//!
//! # What this is, and what it is not
//!
//! This lowers a [`Fragment`]'s ops to machine code and runs them against a
//! real `dyn Vm`. It is the thing criterion 16's **third leg** needs: "the
//! lowered code against `frag::run` over the same boundaries. That leg is
//! required before any lowering ships, and cannot be written before one
//! exists."
//!
//! It is **not §S8's call boundary**, and nothing here should be read as
//! claiming it. §S8 specifies one signature `fn(ctx: i64) -> i32` under
//! `CallConv::Tail`, `Tail` thunks per native, `return_call_indirect` from tail
//! positions, and a C-convention *entry trampoline* because Rust cannot define
//! or call a `Tail` function at all. This module emits a function under the
//! host's **default C convention** so Rust can call it directly, which is the
//! trampoline's job done the short way while there is only one function to
//! call. When §S8's boundary lands, this entry becomes the thing the trampoline
//! wraps.
//!
//! Nor is it inlining, promotion, or the meaning guard. There is no cache, no
//! `Interp` integration, and no compiled body — one fragment, compiled and
//! called, so the representation has a consumer and the third leg can run.
//!
//! # The rule it inherits
//!
//! **A fragment is entered only when its guard admits the stack**, and the word
//! runs otherwise ([`crate::lower::Compiled::run`] asks
//! `Fragment::admits_with`, exactly as `bund2_interp::frag::run` does). So the
//! emitted code may assume admission, and a failure after it is a broken
//! invariant rather than a fact about the program — which is why every helper
//! returns a status and the arm branches out on the first one that is not zero.
//!
//! # Why the register file costs nothing at run time
//!
//! `Op::PopInt`'s numbering — "each pop shifts the file up one slot and writes
//! slot 0" — is **static over a fixed op sequence**, so the shift is a renaming
//! performed *here*, while lowering, over a `Vec<Variable>`. Nothing rotates at
//! run time, and `Op::AddInt` becomes one `iadd` between two SSA values.

use bund2_api::{Error, Vm};
use bund2_ir::{Fragment, Guard, Op};
use bund2_value::BundValue;

// `MemFlagsData`, not `MemFlags`: `load` takes `Into<MemFlagsData>`, and
// `trusted()` is a constructor on `MemFlagsData` — the two are separate structs
// at 0.135 and only one of them has it.
// `Block`, `FuncRef`, `StackSlot`, `Type` and `Value` are named only by the
// fragment emitter's signature — every other use in this file infers them.
// They are `ir::entities` types re-exported from `ir`, except `Type`, which
// `ir` re-exports from `ir::types`.
use cranelift_codegen::ir::{
    AbiParam, Block, FuncRef, InstBuilder, MemFlagsData, Signature, StackSlot, Type, Value, types,
};
// Criterion 17's dominance check: "every inlined region in the emitted code is
// preceded, on every path into it, by a load and compare". Both are public
// modules of `cranelift-codegen` 0.135; `block_dominates` takes two blocks and
// needs no `Layout`, which the `ProgramPoint` form would.
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

/// §S8's per-call context, as far as this step needs it.
///
/// §S8 gives it "the `&mut dyn Vm` as a stored fat pointer, an error slot, and
/// the cells of §S6's *Addressing*". The cells belong to the meaning guard,
/// which does not exist yet, so this holds the first two. Compiled code sees it
/// only as an opaque pointer.
pub struct Ctx<'a> {
    vm: &'a mut dyn Vm,
    /// Where a helper parks an `Err`. §S11: an error travels in the context,
    /// never as an unwind.
    err: Option<Error>,
    /// The natives this body may call, by index — **name and function**.
    ///
    /// The name is not decoration. D49 requires both tiers to word a caught
    /// panic identically, and Tier 0's `Interp::invoke` produces
    /// `native `<name>` panicked: <message>` through `bund2_api::panicked`. An
    /// adapter holding only a function pointer could not reproduce that, and
    /// criterion 29 compares against Tier 0's observable result.
    natives: &'a [(&'a str, bund2_api::NativeFn)],
    /// The values a compiled **body** applies, in order.
    ///
    /// A body is a `Vec<BundValue>` — literals, `CALL`s, `CONTEXT`s — and
    /// [`jit_apply`] hands each one to `Vm::apply`, which is Tier 0's own path.
    /// Empty for a fragment arm, which applies no values: its ops are the whole
    /// of it.
    body: &'a [BundValue],
    /// **The fragments this body inlines, by site index** — RFC-0005 §S6.
    ///
    /// The type guard is asked at run time, against the stack as it stands, so
    /// the table has to travel with the running body rather than being consumed
    /// at compile time. [`jit_admits`] indexes it with the site number the
    /// lowering baked into the call.
    ///
    /// Empty for a body that inlines nothing, which is every body until the
    /// join lands — and for a fragment arm, whose single fragment *is* the
    /// whole of it and needs no table.
    sites: &'a [Fragment],
}

/// A helper's status: `0` success, `1` error parked in [`Ctx::err`].
const OK: i32 = 0;
const FAIL: i32 = 1;

/// **A type guard's verdict — a different protocol from a status, on the same
/// two values.**
///
/// [`jit_admits`] does not report success or failure: it answers whether the
/// fragment's guard admits, and *both* answers are ordinary outcomes. Nothing
/// is parked in the context either way, because declining is not an error —
/// §S6 has the site "branch to the slot call, the generic path", which is
/// always correct and merely slower.
///
/// **The two protocols collide on both values, in opposite directions.**
/// `ADMIT` is `FAIL`'s value and `DECLINE` is `OK`'s. So a `brif` copied from
/// the status pattern — `brif status, fail, next` — would run the inlined ops
/// exactly when the guard refused them, and a status-style error check would
/// read an admission as a failure. Neither mistake changes a type or trips a
/// test that only inspects results, which is why
/// `the_admit_verdict_is_not_a_status` asserts the collision rather than
/// trusting a comment.
const ADMIT: i32 = 1;
const DECLINE: i32 = 0;

/// Everything one generic call site needs — the path a value takes when it is
/// not inlined, and the path every guard falls back to.
struct GenericCall {
    tail_sig: cranelift_codegen::ir::SigRef,
    callee: Value,
    ctx_val: Value,
    fail: Block,
    ptr: Type,
    base: Value,
    drain_off: i32,
    cells_base: Value,
    request_off: i32,
}

/// **Emit one dispatched call and §S5's request check after it.**
///
/// `join` is where control goes when the call is done: `None` for a value that
/// simply continues in the current block, `Some(block)` for the generic arm of
/// an inlined site, which has to rejoin the path the region took.
fn emit_generic_call(f: &mut FunctionBuilder<'_>, c: &GenericCall, join: Option<Block>) {
    let call = f.ins().call_indirect(c.tail_sig, c.callee, &[c.ctx_val]);
    let status = f.inst_results(call)[0];
    let next = f.create_block();
    f.ins().brif(status, c.fail, &[], next, &[]);
    f.switch_to_block(next);

    // **§S5's request check, in the emitted code.** "After every call, compiled
    // code loads it beside the epoch and `autoadd`. If it is set, compiled code
    // calls a drain helper."
    let pending = f
        .ins()
        .uload32(MemFlagsData::trusted(), c.cells_base, c.request_off);
    let drain = f.create_block();
    let after = f.create_block();
    f.ins().brif(pending, drain, &[], after, &[]);

    f.switch_to_block(drain);
    let drain_callee = f
        .ins()
        .load(c.ptr, MemFlagsData::trusted(), c.base, c.drain_off);
    let drain_call = f.ins().call_indirect(c.tail_sig, drain_callee, &[c.ctx_val]);
    let drain_status = f.inst_results(drain_call)[0];
    f.ins().brif(drain_status, c.fail, &[], after, &[]);

    f.switch_to_block(after);
    if let Some(join) = join {
        f.ins().jump(join, &[]);
    }
}

/// Everything a fragment's ops need from the function they are emitted into.
///
/// Grouped rather than passed loose because the list is long and every field is
/// the caller's: [`emit_fragment_ops`] creates no blocks of its own beyond the
/// per-op continuations, and owns none of this.
struct FragmentSite {
    ptr: Type,
    /// Where a pop helper writes the int it produced. One slot is enough: each
    /// pop's value moves into its own `Variable` before the next.
    slot: StackSlot,
    ctx_val: Value,
    /// Where any helper's non-zero status goes. The caller decides what that
    /// block does — return `FAIL` for a whole arm, take the generic path for an
    /// inlined site.
    fail: Block,
    pop: FuncRef,
    push: FuncRef,
    dup: FuncRef,
    drop_: FuncRef,
}

/// **Emit one fragment's ops into the function being built**, and say whether
/// any of them touched the stack.
///
/// Factored out of [`compile`] so an *inlined site* can emit the same ops in
/// the middle of a compiled body (§S6). The two callers differ in what
/// surrounds the ops, not in the ops themselves, which is the point: an arm and
/// an inlined region must compute the same thing or criterion 16's differential
/// is measuring two lowerings rather than one.
///
/// **It stops at the end of the op loop.** The `OK` return, the `fail` block's
/// body, `seal_all_blocks` and `finalize` belong to the caller, because a whole
/// arm *returns* where an inlined site *falls through* to the next value. A
/// version of this that emitted the return would be unusable at a site, and the
/// mistake would show up as unreachable code rather than as a failure.
///
/// **The register file is a compile-time renaming.** `PopInt` rotates the file
/// and writes slot 0, exactly as `Op::PopInt` documents and `frag::run`
/// performs at run time — here it costs nothing, because the "file" is a
/// `Vec<Variable>` the emitter walks rather than storage the code touches.
fn emit_fragment_ops(
    f: &mut FunctionBuilder<'_>,
    fragment: &Fragment,
    site: &FragmentSite,
    promoted: &mut Vec<Variable>,
    keep_result: bool,
) -> Result<bool, String> {
    let (ptr, slot, ctx_val, fail) = (site.ptr, site.slot, site.ctx_val, site.fail);
    let live = usize::from(fragment.registers());
    let mut file: Vec<Variable> = Vec::with_capacity(live);
    for _ in 0..live {
        let v = f.declare_var(types::I64);
        let zero = f.ins().iconst(types::I64, 0);
        f.def_var(v, zero);
        file.push(v);
    }

    let mut emitted_any_call = false;
    for op in fragment.ops() {
        match op {
            Op::PopInt => {
                // **A promoted operand is read, not popped.** `promoted` models
                // the stack *top* at compile time, so taking from its end and
                // falling through to the helper when it is empty is the operand
                // order the stack would have given: what is promoted is above
                // what is not. This is the whole of promotion's saving — the
                // value never reached the stack, so nothing fetches it back.
                let v = if let Some(src) = promoted.pop() {
                    f.use_var(src)
                } else {
                    let addr = f.ins().stack_addr(ptr, slot, 0);
                    let call = f.ins().call(site.pop, &[ctx_val, addr]);
                    let status = f.inst_results(call)[0];
                    let next = f.create_block();
                    f.ins().brif(status, fail, &[], next, &[]);
                    f.switch_to_block(next);
                    emitted_any_call = true;
                    // `stack_load(pointer_type, loaded_type, slot, offset)` —
                    // the generated builder takes both types, and the vendored
                    // `src/` does not carry these signatures at all: they live
                    // in `target/debug/build/cranelift-codegen-*/out/inst_builder.rs`.
                    f.ins().stack_load(ptr, types::I64, slot, 0)
                };
                if file.is_empty() {
                    return Err("a fragment popped into a register file it declared as empty".into());
                }
                file.rotate_right(1);
                let Some(&dst) = file.first() else {
                    return Err("the register file lost its first slot".into());
                };
                f.def_var(dst, v);
            }
            Op::PushInt(r) => {
                let Some(&src) = file.get(usize::from(*r)) else {
                    return Err(format!(
                        "fragment op PushInt names register {r}, outside a file of {live}"
                    ));
                };
                // **Promoted, when the caller has somewhere for it to live.**
                // An inlined site under §S5's residual has no join to satisfy,
                // so the result stays in a `Variable` and the next site reads
                // it as an operand — the chaining `1 2 + 3 +` needs. A
                // standalone arm has no such caller: it returns to Rust, which
                // can only see the stack, so it pushes.
                if keep_result {
                    let held = f.declare_var(types::I64);
                    let v = f.use_var(src);
                    f.def_var(held, v);
                    promoted.push(held);
                } else {
                    let v = f.use_var(src);
                    let call = f.ins().call(site.push, &[ctx_val, v]);
                    let status = f.inst_results(call)[0];
                    let next = f.create_block();
                    f.ins().brif(status, fail, &[], next, &[]);
                    f.switch_to_block(next);
                    emitted_any_call = true;
                }
            }
            Op::DupTop | Op::DropTop => {
                let callee = if matches!(op, Op::DupTop) {
                    site.dup
                } else {
                    site.drop_
                };
                let call = f.ins().call(callee, &[ctx_val]);
                let status = f.inst_results(call)[0];
                let next = f.create_block();
                f.ins().brif(status, fail, &[], next, &[]);
                f.switch_to_block(next);
                emitted_any_call = true;
            }
            Op::AddInt { dst, a, b } | Op::SubInt { dst, a, b } => {
                let (Some(&va), Some(&vb)) = (file.get(usize::from(*a)), file.get(usize::from(*b)))
                else {
                    return Err(format!(
                        "fragment op names registers {a} and {b}, outside a file of {live}"
                    ));
                };
                let x = f.use_var(va);
                let y = f.use_var(vb);
                // Wrapping, as D4's `i64` arithmetic and the word are: a
                // trapping or checked add is where a lowering would part
                // company with `frag::run` at `i64::MAX`.
                let r = if matches!(op, Op::AddInt { .. }) {
                    f.ins().iadd(x, y)
                } else {
                    f.ins().isub(x, y)
                };
                let Some(&slot_dst) = file.get(usize::from(*dst)) else {
                    return Err(format!(
                        "fragment op names destination register {dst}, outside a file of {live}"
                    ));
                };
                f.def_var(slot_dst, r);
            }
        }
    }
    Ok(emitted_any_call)
}

/// **Sync the deepest `n` promoted values back to the stack** — RFC-0005 §S5.
///
/// Promotion holds values in `Variable`s that the stack has never seen. Before
/// anything could observe them they are pushed, deepest first, so the stack is
/// exactly what it would have been had each literal been applied in turn.
///
/// **Through `site.push`, never a bare write.** The helper reaches
/// `Stack::push`, which is what applies D41's stack symbol; a direct write
/// would leave `StackSym::NONE` and render `tags: {}` where the oracle renders
/// `tags: {"stack": "main"}` — criterion 12.
///
/// **A prefix, because the deepest values sync first.** A promoted site
/// consumes the *top* `needs` values; anything below them must already be on
/// the stack, or the site's result would land beneath a value still sitting in
/// a register. Syncing the prefix is what keeps the order right.
///
/// The end this drains is the **bottom** of the modelled stack.
/// [`emit_sync_top_n`] drains the other one, and D68 is why.
fn emit_sync_n(
    f: &mut FunctionBuilder<'_>,
    promoted: &mut Vec<Variable>,
    n: usize,
    site: &FragmentSite,
) -> usize {
    let n = n.min(promoted.len());
    if n == 0 {
        return 0;
    }
    let live: Vec<Variable> = promoted.drain(..n).collect();
    for var in live {
        let v = f.use_var(var);
        let call = f.ins().call(site.push, &[site.ctx_val, v]);
        let status = f.inst_results(call)[0];
        let next = f.create_block();
        f.ins().brif(status, site.fail, &[], next, &[]);
        f.switch_to_block(next);
    }
    n
}

/// **Sync the top `n` promoted values, keeping the deeper ones in registers**
/// — D68's half of the sync, and the mirror of [`emit_sync_n`].
///
/// # Why the other end
///
/// `promoted`'s front is the **deepest** value and its back is the top, so
/// [`emit_sync_n`]'s `drain(..n)` pushes from the bottom up. That is right for
/// an inlined site, which consumes the top `needs` values *from registers* and
/// therefore needs everything beneath them already on the stack.
///
/// D68 asks the opposite question. A crossed call consumes its operands **from
/// the stack**, so those are the values that must be pushed; what may stay in
/// registers is what lies *below* them. Draining the suffix is that.
///
/// # The ordering argument, restated rather than reused
///
/// D68's soundness rests on a property [`emit_sync_n`]'s does not mention:
/// **the promoted values must be the top of the abstract stack when they are
/// finally synced.** Pushing only appends, so a value that ends up beneath
/// something else on the real stack can never be put back above it.
///
/// That is why D68 admits only a callee whose declared effect **produces
/// nothing**. Cross `1 2 3 f` with `f` at `eff(1, 1)` and the real stack ends
/// `[…, r]` while the abstract one is `[…, 1, 2, r]`; syncing then gives
/// `[…, r, 1, 2]` — wrong, silently, with no guard that could catch it. With
/// `produces == 0` the callee leaves nothing behind, so after it returns the
/// held values are the top again and the final sync is sound.
///
/// This helper enforces the *other* half of that invariant: it pushes the top
/// `n`, so what remains held is a contiguous run at the bottom of the abstract
/// stack, in order. Passing an `n` smaller than the callee's `consumes` would
/// leave a held value above something the callee pops, and the final sync would
/// then write it in the wrong place — so callers pass
/// `min(consumes, promoted.len())` and nothing less.
fn emit_sync_top_n(
    f: &mut FunctionBuilder<'_>,
    promoted: &mut Vec<Variable>,
    n: usize,
    site: &FragmentSite,
) -> usize {
    let n = n.min(promoted.len());
    if n == 0 {
        return 0;
    }
    // **From the split point to the end, in order.** `drain` yields
    // front-to-back, and the front of this range is the deepest of the values
    // being pushed — so they reach the stack bottom-first, exactly as
    // `emit_sync_n` pushes the values it drains.
    let at = promoted.len() - n;
    let live: Vec<Variable> = promoted.drain(at..).collect();
    for var in live {
        let v = f.use_var(var);
        let call = f.ins().call(site.push, &[site.ctx_val, v]);
        let status = f.inst_results(call)[0];
        let next = f.create_block();
        f.ins().brif(status, site.fail, &[], next, &[]);
        f.switch_to_block(next);
    }
    n
}

/// **Can this fragment take its operands from registers?**
///
/// Two conditions, and both are about what the ops address rather than what
/// they compute:
///
/// - the guard is [`Guard::TopAreInt`], so a model holding known int literals
///   answers it **statically**. A promoted site asks `jit_admits` nothing: with
///   the operands held back in registers the guard would be interrogating a
///   stack that is missing them, and a `Depth` guard is a claim about the stack
///   itself that promotion cannot discharge;
/// - every op is register-only. `Op::DupTop` and `Op::DropTop` address the
///   stack directly through their helpers, so a fragment using either must see
///   its operands there.
fn takes_registers(fragment: &Fragment) -> bool {
    matches!(fragment.guard(), Guard::TopAreInt(_))
        && fragment.ops().iter().all(|o| {
            matches!(
                o,
                Op::PopInt | Op::PushInt(_) | Op::AddInt { .. } | Op::SubInt { .. }
            )
        })
}

/// **Does this site's fragment admit the stack as it stands?** — RFC-0005 §S6.
///
/// `admits_with` takes a Rust closure over the stack, so compiled code cannot
/// ask the guard itself; it calls here. The standalone fragment lowering has
/// Rust ask the guard *before* entering the arm ("the emitted code may assume
/// admission"), but an inlined site is reached mid-body with no Rust in
/// between, so the question has to be asked from the emitted code — §S6, after
/// the type guard admits and before the first op.
///
/// **It peeks and never pulls**, exactly as `frag::run` does: a declining guard
/// must leave the stack as it found it, or the generic path it falls back to
/// would run against a stack the guard had already eaten.
///
/// A site index the lowering never created is a broken invariant, and the
/// answer is still [`DECLINE`] rather than an error: the generic path is
/// correct for every site, so a lowering bug costs speed instead of meaning.
extern "C" fn jit_admits(c: *mut Ctx<'_>, site: usize) -> i32 {
    // SAFETY: the entry's contract, as every other adapter's.
    let Some(c) = (unsafe { ctx(c) }) else {
        return DECLINE;
    };
    let Some(fragment) = c.sites.get(site) else {
        return DECLINE;
    };
    // Immutable reborrow: the guard reads depth and peeks, and both take
    // `&self`, so nothing here can disturb what it is deciding about.
    let vm = &*c.vm;
    if fragment.admits_with(vm.depth(), |n| vm.peek_at(n)) {
        ADMIT
    } else {
        DECLINE
    }
}

/// **§S5's drain helper, as a callee compiled code reaches** — *A call may
/// leave a body to run*.
///
/// A compiled call gets control back before a filed body has run. If the body
/// went unrun the next value would run first, and since `request_tail` assigns,
/// a later request would overwrite it and it would never run at all. So after
/// every **non-tail** call the body loads §S6's request cell and, when it is
/// set, calls through here.
///
/// **The branch is in the emitted code, not in Rust.** An earlier shape drained
/// unconditionally inside the native adapter, which cost a call on every
/// non-tail call whether or not a request was filed, and left the cell mirrored
/// rather than read. §S6 has compiled code load the cell and branch, which is
/// what this is the callee of.
///
/// The second parameter is unused: the boundary's signature is uniform at
/// `(ctx, index) -> status` (§S8), and a drain has no index.
extern "C" fn jit_drain(c: *mut Ctx<'_>, _unused: usize) -> i32 {
    // SAFETY: the entry's contract, as every other adapter's.
    let Some(c) = (unsafe { ctx(c) }) else {
        return FAIL;
    };
    let r = c.vm.drain_tail_request();
    status_of(c, r)
}

/// **The one function that turns a helper's `Result` into a status** —
/// RFC-0005 §S5, *A call may end the program*.
///
/// Every Rust function that runs Bund code on compiled code's behalf and then
/// returns to it comes through here: the per-native adapter, and — when they
/// exist — the resolving trampoline, §S5's drain helper and the residual path's
/// `apply`. The entry trampoline is not one of them; it converts a body's
/// status rather than making one (assumption 31).
///
/// # Why one function rather than a rule each helper follows
///
/// `bund.exit` records a code and returns `Ok` (D52). Tier 0 stops at its
/// **next step**, because `exit_gate` refuses at the top of `apply_step` and of
/// `Vm::eval_lambda`. Compiled code has no next step: the three cells it reads
/// after a call record no exit, so a helper that reported the native's `Ok`
/// would let the body run on past `exit`. §S5 puts the check in one place so
/// that "a helper added later cannot forget it" — the twelfth review's B1,
/// where the drain helper was exactly such a later addition.
///
/// # What it parks, and why the two arms differ
///
/// The substitution happens **only after `Ok`** (the fourteenth review's B1).
/// After an `Err` the error passes through unchanged, because Tier 0 never
/// replaces one: a native whose body was refused wraps the refusal in its own
/// context, as `map` does with `MAP: lambda execution returns error: …`, and
/// `?try` puts that whole text in its `error` CONDITIONAL's `context` slot,
/// where criterion 30 compares it as text. Substituting after `Err` too would
/// keep only the short form and fail that comparison while every other stated
/// reason still held.
///
/// # The clearing
///
/// It clears the tail request on **every** error it answers, whether it made
/// that error from a recorded exit or is passing a helper's own `Err` through
/// (§S5's third settled edge). Tier 0 does this in `Interp::invoke`, which a
/// compiled call never reaches, so D56 put `Vm::clear_tail_request` on the
/// trait for it. Before this helper existed the clearing sat inlined in each
/// adapter, which §S5 called a gap left for a future helper; this is that
/// helper, and the clearing has moved here.
///
/// **This is §S6's request cell's first reader.** The cell is written beside
/// `pending_tail` by Tier 0's four named writers; `clear_tail_request` is one
/// of them, so clearing through it writes the mirror as well as the truth.
fn status_of(c: &mut Ctx<'_>, r: Result<(), Error>) -> i32 {
    let exited = c.vm.exit_requested();
    match (exited, r) {
        // No exit, and the helper succeeded: the only path that reports
        // success.
        (None, Ok(())) => OK,
        // The helper failed. The error is the program's answer whether or not
        // an exit was also recorded — Tier 0 passes a native's error up
        // unchanged, and under `?try` its text becomes a value.
        (_, Err(e)) => {
            c.vm.clear_tail_request();
            c.err = Some(e);
            FAIL
        }
        // The helper succeeded, but the program asked to end. This is the
        // error Tier 0 would make at its next step, made here because compiled
        // code does not take one.
        (Some(code), Ok(())) => {
            c.vm.clear_tail_request();
            c.err = Some(Error::exited(code));
            FAIL
        }
    }
}

/// Rebuild the context from the pointer compiled code was handed.
///
/// # Safety
///
/// `ctx` must be the pointer [`Compiled::run`] passed to the entry, which
/// borrows a live `Ctx` for the duration of the call and hands out no other
/// reference to it.
unsafe fn ctx<'a>(ctx: *mut Ctx<'a>) -> Option<&'a mut Ctx<'a>> {
    if ctx.is_null() {
        return None;
    }
    // SAFETY: the caller's contract, above.
    Some(unsafe { &mut *ctx })
}

/// `Op::PopInt` — pull an unboxed int into a register.
///
/// Writes the value through `out` and answers a status, rather than returning
/// the value, because a pull can find an empty stack or a value that is not an
/// unboxed int. After the guard admitted, either is a broken invariant.
extern "C" fn jit_pop_int(c: *mut Ctx<'_>, out: *mut i64) -> i32 {
    // SAFETY: `Compiled::run`'s contract.
    let Some(c) = (unsafe { ctx(c) }) else {
        return FAIL;
    };
    if out.is_null() {
        c.err = Some(Error::internal("jit_pop_int was handed no place to write"));
        return FAIL;
    }
    match c.vm.pull().and_then(|v| v.as_int()) {
        Some(v) => {
            // SAFETY: checked non-null above; the lowering points it at a slot
            // of the emitted function's own frame.
            unsafe { out.write(v) };
            OK
        }
        None => {
            c.err = Some(Error::internal(
                "lowered fragment op PopInt found no unboxed integer after its guard admitted; \
                 the guard and the op list disagree about the arm's domain",
            ));
            FAIL
        }
    }
}

/// `Op::PushInt` — push a register's value as a fresh value at `q` 100.0.
///
/// `BundValue::int` is what `frag::run` pushes, so the arm and the model agree
/// on the tag and on `q` by construction (§S6, constraint 2).
extern "C" fn jit_push_int(c: *mut Ctx<'_>, v: i64) -> i32 {
    // SAFETY: `Compiled::run`'s contract.
    let Some(c) = (unsafe { ctx(c) }) else {
        return FAIL;
    };
    c.vm.push(BundValue::int(v));
    OK
}

/// `Op::DupTop` — **`.dup()`, not a clone.**
///
/// `dup` gives the copy a fresh header and therefore a fresh identity (F13),
/// and the differential test asserts that directly. A `clone` would share the
/// `Rc` and the identity with it, which is not the arm `dup` implements.
extern "C" fn jit_dup_top(c: *mut Ctx<'_>) -> i32 {
    // SAFETY: `Compiled::run`'s contract.
    let Some(c) = (unsafe { ctx(c) }) else {
        return FAIL;
    };
    match c.vm.peek() {
        Some(v) => {
            let copy = v.dup();
            c.vm.push(copy);
            OK
        }
        None => {
            c.err = Some(Error::internal(
                "lowered fragment op DupTop found an empty stack after its guard admitted",
            ));
            FAIL
        }
    }
}

/// `Op::DropTop` — discard the top, and fail rather than succeed silently.
///
/// Silent success here was the sixth review's S3: `drop drop` on a one-deep
/// stack would drop once and report success, where the words fail "too
/// shallow".
extern "C" fn jit_drop_top(c: *mut Ctx<'_>) -> i32 {
    // SAFETY: `Compiled::run`'s contract.
    let Some(c) = (unsafe { ctx(c) }) else {
        return FAIL;
    };
    if c.vm.pull().is_none() {
        c.err = Some(Error::internal(
            "lowered fragment op DropTop found an empty stack after its guard admitted",
        ));
        return FAIL;
    }
    OK
}

/// **§S8's second piece: the per-native adapter.**
///
/// §S8: "A per-native adapter in Rust: `extern "C" fn(ctx: *mut Ctx, native:
/// usize) -> i32`. It rebuilds `&mut dyn Vm` from the context and calls the
/// native's `NativeFn` through `bund2_api::catch_panic` (D49). It parks an `Err`
/// in the error slot and returns the status. No panic unwinds out of it, so none
/// reaches a compiled frame."
///
/// **The wording of a caught panic is Tier 0's, deliberately.** D49 says "every
/// native call catches a panic and returns `Error::internal` naming what was
/// running", and that both tiers agree. Tier 0's `Interp::invoke` builds the
/// message as `panicked(&format!("native `{}`", name), &msg)`; this does the
/// same, with the same helper, so criterion 29 can compare the two results
/// rather than two spellings of one failure.
///
/// `native` indexes [`Ctx::natives`]. An index outside it is a broken invariant
/// in the lowering, not a fact about the program, so it is `Error::internal`.
extern "C" fn jit_call_native(c: *mut Ctx<'_>, native: usize) -> i32 {
    // SAFETY: `CompiledBody::run`'s contract.
    let Some(c) = (unsafe { ctx(c) }) else {
        return FAIL;
    };
    let Some(&(name, f)) = c.natives.get(native) else {
        c.err = Some(Error::internal(format!(
            "a compiled body called native index {native}, outside the {} it was given",
            c.natives.len()
        )));
        return FAIL;
    };
    // The catch is here, at the call, which is where Tier 0's is.
    let outcome = bund2_api::catch_panic(|| f(&mut *c.vm));
    let r = match outcome {
        Ok(r) => r,
        Err(msg) => Err(bund2_api::panicked(&format!("native `{name}`"), &msg)),
    };
    // **The drain is no longer here.** Compiled code loads §S6's request cell
    // after a non-tail call and calls [`jit_drain`] when it is set, so this
    // adapter does one thing: run the native and answer a status. `status_of`
    // carries F96's parity (D56) and D52's rule that a recorded exit becomes
    // the error status.
    status_of(c, r)
}


/// **Apply one of the body's values — the first lowering of a real Bund word.**
///
/// `index` names a value in [`Ctx::body`], and this hands it to `Vm::apply`,
/// which is `Interp::apply`: the Tier 0 floor check, `apply_step`,
/// `take_pending`, `run_to` and the exit gate. So a compiled body reproduces
/// Tier 0 **exactly** rather than approximately — and it has to, because the
/// semantics a body must honour are not all expressible in compiled code yet:
///
/// - **Under `autoadd` a `CALL` is not a call** (§S4, *Step 3*). The flag is
///   VM-wide, toggled by `:` and `;` as *commands*, and it changes what happens
///   to *every* value applied: a literal and a `CALL` are both appended into the
///   value beneath, and a `CONTEXT` is pushed rather than switching stacks. §S4
///   requires a compiled body to guard on it at entry and re-read it after every
///   call; that guard needs §S6's `autoadd` cell, which does not exist, and
///   `autoadd` is not readable through the `Vm` trait at all. Going through
///   `apply` honours the mode by construction instead.
/// - **A `CONTEXT` switches stacks and pushes onto the nesting stack**
///   (`apply_step`'s CONTEXT arm), and **`bund.exit` gates every step** (F112).
/// - **A `CALL` to a lambda files a tail request** that `apply` then drains, so
///   Bund depth stays on the heap as RFC-0003's frame loop requires.
///
/// **This is a shape, not a speedup**, and the RFC should not be read as
/// claiming otherwise: it wraps an entry trampoline, a slot table and a status
/// protocol around work the interpreter already does. What it buys is the
/// structure §S6's fragments are inlined *into*; criterion 10 measures the tier
/// as shipped, not this.
extern "C" fn jit_apply(c: *mut Ctx<'_>, index: usize) -> i32 {
    // SAFETY: `CompiledWord::run`'s contract.
    let Some(c) = (unsafe { ctx(c) }) else {
        return FAIL;
    };
    let Some(v) = c.body.get(index).cloned() else {
        c.err = Some(Error::internal(format!(
            "a compiled body applied value {index}, outside the {} it was given",
            c.body.len()
        )));
        return FAIL;
    };
    let r = c.vm.apply(v);
    // Same helper, same two obligations. `Vm::apply` is `Interp::apply`, which
    // already consults the exit gate after `run_to` (F112), so a value that
    // ended the program arrives here as an `Err` and passes through unchanged.
    // The substitution matters for the arm where `apply` answers `Ok` with an
    // exit recorded — and the clearing for a `CALL` to a lambda, which files
    // through `request_tail` where `run_to`'s error arm unwinds frames only.
    //
    // **The drain is in the emitted code**, after this returns: `Vm::apply`
    // drains its own request before it returns, so the cell is normally clear
    // and the branch is not taken. Keeping the load at the call site rather
    // than resting on what `apply` happens to do is what §S5 asks of a non-tail
    // call.
    status_of(c, r)
}

/// **§S5's residual path: apply the rest of the body, one value at a time.**
///
/// The residual is *not* "take the generic call and rejoin the fast path". §S5:
/// it "applies the rest of the body's values one at a time through the
/// runtime's `apply`, exactly as Tier 0 would". It never comes back, and that
/// is what makes it affordable — a guard that refuses leaves the fast path for
/// good, so the fast path owes no merge and may keep values promoted across
/// the site that failed.
///
/// **Still guard-and-branch, not guard-and-bail.** Control never leaves
/// compiled code: this is a runtime helper called from the emitted body, like
/// [`jit_apply`], and the body returns its status. There is no OSR and no
/// frame handed back to Tier 0.
///
/// `index` is assumption 38's **resume index** — the position in the source
/// body at which interpretation resumes. The caller has already synced every
/// promoted value, so the stack here is what Tier 0 would have built.
///
/// **It stops at the first failure**, as a body does: the values after a
/// failing one must not run, which is the property
/// `a_compiled_word_stops_at_the_first_failure` pins for the fast path.
extern "C" fn jit_residual(c: *mut Ctx<'_>, index: usize) -> i32 {
    // SAFETY: `CompiledWord::run`'s contract, as `jit_apply`'s.
    let Some(c) = (unsafe { ctx(c) }) else {
        return FAIL;
    };
    if index > c.body.len() {
        c.err = Some(Error::internal(format!(
            "a compiled body resumed at value {index}, outside the {} it was given",
            c.body.len()
        )));
        return FAIL;
    }
    // Cloned up front: `c.vm` is borrowed mutably by `apply`, so the slice
    // cannot stay borrowed from `c` across the loop.
    let rest: Vec<BundValue> = c.body[index..].to_vec();
    for v in rest {
        let r = c.vm.apply(v);
        // **Every value goes through `status_of`**, not just the last. It is
        // §S5's one status-maker: it substitutes `Error::exited` after an `Ok`
        // with an exit recorded, passes an `Err` through unchanged, and clears
        // a tail request on every error. A loop that checked only `is_err`
        // would run the value after `bund.exit`.
        let status = status_of(c, r);
        if status != OK {
            return status;
        }
    }
    OK
}

/// The **entry trampoline's** type: an opaque context in, a status out.
///
/// §S8's fourth piece. Rust enters compiled code only through this, because it
/// can neither define nor call a `CallConv::Tail` function: "0.135.0's
/// `CallConv` has no Rust ABI". So the trampoline carries the platform's own
/// convention and the arm behind it carries `Tail`.
type Entry = unsafe extern "C" fn(*mut Ctx<'_>) -> i32;

/// One compiled fragment, and the module whose memory holds its code.
///
/// The `JITModule` is kept because dropping it frees the code; the entry
/// pointer would dangle.
pub struct Compiled {
    /// Held for its code memory. Never read directly.
    _module: JITModule,
    entry: Entry,
    fragment: Fragment,
    arm_conv: CallConv,
    entry_conv: CallConv,
}

impl Compiled {
    /// The convention the **arm** was emitted under — `CallConv::Tail` wherever
    /// the target supports tail calls.
    ///
    /// Exposed so §S8's boundary shape is *checkable* rather than described. A
    /// signature is not observable in the finished code, so the lowering
    /// records what it chose and a test reads it back.
    pub fn arm_call_conv(&self) -> CallConv {
        self.arm_conv
    }

    /// The convention the **entry trampoline** was emitted under: the
    /// platform's own, which is what makes it callable from Rust.
    pub fn entry_call_conv(&self) -> CallConv {
        self.entry_conv
    }
}

impl Compiled {
    /// Run the arm if its guard admits, exactly as `frag::run` decides it.
    ///
    /// `Ok(false)` means **the guard declined and the stack was not touched**,
    /// so the caller falls through to the word. `Ok(true)` means the arm ran to
    /// completion. An `Err` is a broken invariant reported through the context,
    /// never an unwind (§S11).
    pub fn run(&self, vm: &mut dyn Vm) -> Result<bool, Error> {
        let depth = vm.depth();
        if !self
            .fragment
            .admits_with(depth, |n| vm.peek_at(n))
        {
            return Ok(false);
        }
        let mut c = Ctx {
            vm,
            err: None,
            // A fragment's arm calls no native and applies no value: its ops
            // are the whole of it.
            natives: &[],
            body: &[],
            // Its guard is asked by Rust before entry, so the arm needs no
            // site table of its own.
            sites: &[],
        };
        // SAFETY: the entry is the address `finalize_definitions` published for
        // a function this module emitted under the host's own calling
        // convention, and `&mut c` is live for the whole call. The emitted code
        // treats the pointer as opaque and hands it back to the helpers above.
        let status = unsafe { (self.entry)(&raw mut c) };
        match c.err {
            Some(e) => Err(e),
            None if status == OK => Ok(true),
            None => Err(Error::internal(
                "a lowered fragment returned a failing status with no error in the context",
            )),
        }
    }
}

/// Lower one fragment to machine code.
///
/// Returns the error rather than panicking: a fragment that cannot be lowered
/// is a defect in Bund2 or an op this module has not learned, and shipped code
/// may not `expect` (D37).
pub fn compile(fragment: &Fragment) -> Result<Compiled, String> {
    let mut builder =
        JITBuilder::new(default_libcall_names()).map_err(|e| format!("JIT builder: {e}"))?;
    builder.symbol("jit_pop_int", jit_pop_int as *const u8);
    builder.symbol("jit_push_int", jit_push_int as *const u8);
    builder.symbol("jit_dup_top", jit_dup_top as *const u8);
    builder.symbol("jit_drop_top", jit_drop_top as *const u8);
    let mut module = JITModule::new(builder);

    // Captured before the function builder borrows the context: `finalize`
    // wants the frontend config, and `module` is not reachable while the
    // builder holds `ctx_codegen.func`.
    let frontend = module.target_config();
    let ptr = frontend.pointer_type();
    let conv = module.isa().default_call_conv();

    // The four helper signatures, declared as imports.
    let mut sig_pop = Signature::new(conv);
    sig_pop.params.push(AbiParam::new(ptr));
    sig_pop.params.push(AbiParam::new(ptr));
    sig_pop.returns.push(AbiParam::new(types::I32));

    let mut sig_push = Signature::new(conv);
    sig_push.params.push(AbiParam::new(ptr));
    sig_push.params.push(AbiParam::new(types::I64));
    sig_push.returns.push(AbiParam::new(types::I32));

    let mut sig_bare = Signature::new(conv);
    sig_bare.params.push(AbiParam::new(ptr));
    sig_bare.returns.push(AbiParam::new(types::I32));

    let pop_id = module
        .declare_function("jit_pop_int", Linkage::Import, &sig_pop)
        .map_err(|e| format!("declare jit_pop_int: {e}"))?;
    let push_id = module
        .declare_function("jit_push_int", Linkage::Import, &sig_push)
        .map_err(|e| format!("declare jit_push_int: {e}"))?;
    let dup_id = module
        .declare_function("jit_dup_top", Linkage::Import, &sig_bare)
        .map_err(|e| format!("declare jit_dup_top: {e}"))?;
    let drop_id = module
        .declare_function("jit_drop_top", Linkage::Import, &sig_bare)
        .map_err(|e| format!("declare jit_drop_top: {e}"))?;

    // **§S8's first piece: one JIT signature, `fn(ctx) -> i32` under
    // `CallConv::Tail`.** A `Tail` arm is what makes `return_call_indirect`
    // legal from a tail position later; the verifier's `typecheck_tail_call`
    // requires the callee's convention to support tail calls *and* to match the
    // caller's, so every target a compiled body can reach has to be `Tail`.
    //
    // **On the degradation §S8 asks for: at this pinned version it has no
    // trigger, so there is no branch here.** §S8 says tail calls are supported
    // "on x86-64, aarch64 and riscv64; s390x historically lacked it, so the
    // lowering must degrade to an ordinary call there rather than assume it".
    // At `cranelift-codegen` 0.135.0 that history has moved on: `return_call`
    // and `return_call_indirect` have lowering rules in **every** backend,
    // s390x included (`src/isa/s390x/lower.isle`, "Rules for `return_call` and
    // `return_call_indirect`", and the same section in `aarch64`, `x64` and
    // `riscv64`). There is also no per-target predicate to ask:
    // `supports_tail_calls` is a property of the *convention* and answers
    // `true` for `CallConv::Tail` and nothing else, on every target.
    //
    // An earlier draft of this function branched on
    // `CallConv::Tail.supports_tail_calls()`, which is a constant `true` — a
    // check that reads like a capability test and tests nothing. Emitting
    // `Tail` unconditionally and saying why is honest; a branch no target can
    // take would be worse than none, because it would look covered.
    let arm_conv = CallConv::Tail;
    let mut sig_arm = Signature::new(arm_conv);
    sig_arm.params.push(AbiParam::new(ptr));
    sig_arm.returns.push(AbiParam::new(types::I32));
    let arm_id = module
        .declare_function("bund2_arm", Linkage::Export, &sig_arm)
        .map_err(|e| format!("declare bund2_arm: {e}"))?;

    // **§S8's fourth piece: the entry trampoline**, under the platform's own
    // convention, because Rust cannot call a `Tail` function at all.
    let mut sig_entry = Signature::new(conv);
    sig_entry.params.push(AbiParam::new(ptr));
    sig_entry.returns.push(AbiParam::new(types::I32));
    let entry_id = module
        .declare_function("bund2_entry", Linkage::Export, &sig_entry)
        .map_err(|e| format!("declare bund2_entry: {e}"))?;

    let mut ctx_codegen = module.make_context();
    ctx_codegen.func.signature = sig_arm;
    let mut fb_ctx = FunctionBuilderContext::new();
    {
        let mut f = FunctionBuilder::new(&mut ctx_codegen.func, &mut fb_ctx);

        let pop = module.declare_func_in_func(pop_id, f.func);
        let push = module.declare_func_in_func(push_id, f.func);
        let dup = module.declare_func_in_func(dup_id, f.func);
        let drop_ = module.declare_func_in_func(drop_id, f.func);

        let entry = f.create_block();
        let fail = f.create_block();
        f.append_block_params_for_function_params(entry);
        f.switch_to_block(entry);

        let ctx_val = f.block_params(entry)[0];

        // A slot for a helper to write a popped int into. One is enough: each
        // pop's value is moved into its own `Variable` before the next.
        let slot = f.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            8,
            3,
        ));

        let emitted_any_call = emit_fragment_ops(
            &mut f,
            fragment,
            &FragmentSite {
                ptr,
                slot,
                ctx_val,
                fail,
                pop,
                push,
                dup,
                drop_,
            },
            // **A standalone arm promotes nothing.** It is entered from Rust
            // with the operands already on the stack (`Compiled::run` asks
            // `admits_with` first), so every operand comes from the helper —
            // and its result must reach the stack, because Rust is what reads
            // it next and a `Variable` dies with the function.
            &mut Vec::new(),
            false,
        )?;

        let ok = f.ins().iconst(types::I32, i64::from(OK));
        f.ins().return_(&[ok]);

        f.switch_to_block(fail);
        let bad = f.ins().iconst(types::I32, i64::from(FAIL));
        f.ins().return_(&[bad]);
        f.seal_all_blocks();
        f.finalize(frontend);

        // An arm with no call at all would leave `fail` unreachable, which is
        // fine, but it would also mean nothing touched the stack — not a
        // fragment this module should have been handed.
        if !emitted_any_call {
            return Err("a fragment with no stack traffic has nothing to lower".into());
        }
    }

    module
        .define_function(arm_id, &mut ctx_codegen)
        .map_err(|e| format!("define bund2_arm: {e}"))?;
    module.clear_context(&mut ctx_codegen);

    // The trampoline: take the context, call the arm, hand its status back.
    //
    // **This is a direct call, and it is the one criterion 4 allows.** That
    // criterion requires the module's relocations to contain no entry targeting
    // a function, with exactly two exceptions "both from §S8's call boundary: a
    // native's `Tail` thunk calling its Rust adapter, and the entry trampoline
    // calling the body it enters". This is the second of those. Calls *between*
    // compiled bodies stay indirect, through a slot.
    //
    // A plain `call` may cross conventions: the verifier has no
    // `typecheck_call`, and only `typecheck_tail_call` demands that caller and
    // callee agree. That is precisely why the trampoline can be the seam.
    ctx_codegen.func.signature = sig_entry;
    {
        let mut f = FunctionBuilder::new(&mut ctx_codegen.func, &mut fb_ctx);
        let arm = module.declare_func_in_func(arm_id, f.func);
        let block = f.create_block();
        f.append_block_params_for_function_params(block);
        f.switch_to_block(block);
        let ctx_val = f.block_params(block)[0];
        let call = f.ins().call(arm, &[ctx_val]);
        let status = f.inst_results(call)[0];
        f.ins().return_(&[status]);
        f.seal_all_blocks();
        f.finalize(frontend);
    }
    module
        .define_function(entry_id, &mut ctx_codegen)
        .map_err(|e| format!("define bund2_entry: {e}"))?;
    module.clear_context(&mut ctx_codegen);

    module
        .finalize_definitions()
        .map_err(|e| format!("finalize: {e}"))?;

    // **The trampoline's address, never the arm's.** Transmuting the arm would
    // have Rust calling a `Tail` function directly, which is the one thing §S8
    // establishes is impossible.
    let addr = module.get_finalized_function(entry_id);
    if addr.is_null() {
        return Err("the finalized entry trampoline has no address".into());
    }
    // SAFETY: `addr` is the entry of a function this module just emitted with
    // the signature `sig_entry` — one pointer-sized parameter, one `i32`
    // result, under the host's default calling convention — which is exactly
    // `Entry`. The arm it calls carries `Tail` and is never called from Rust.
    let entry: Entry = unsafe { std::mem::transmute::<*const u8, Entry>(addr) };

    Ok(Compiled {
        _module: module,
        entry,
        fragment: fragment.clone(),
        arm_conv,
        entry_conv: conv,
    })
}

/// Where a body's last call sits — §S8 claims tail position for it alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastCall {
    /// An ordinary call, then a return. Costs a frame.
    Ordinary,
    /// `return_call_indirect`: §S8's claimed tail call, which "saves a frame
    /// per body call, and matters most for self-recursive words".
    Tail,
}

/// One compiled body that calls natives, and the code memory behind it.
pub struct CompiledBody {
    /// Held for its code memory. Never read directly.
    _module: JITModule,
    entry: Entry,
    /// **The call slots — the indirection criterion 4 requires.**
    ///
    /// Boxed and allocated *before* the body is compiled, so its base address
    /// is stable and can be embedded as an immediate (§S6, *Addressing*: "the
    /// JIT embeds each cell's address as an immediate"). Filled from
    /// `get_finalized_function` once the thunks exist. A body loads its slot at
    /// run time and calls through it, so no call between compiled code names a
    /// `FuncId` — which is what criterion 4 is about.
    ///
    /// **Underscored because nothing in Rust reads it, and that is the point.**
    /// The reader is the emitted code, through an address taken before this box
    /// was filled. It is held here only to keep the allocation alive: drop it
    /// and the body loads freed memory. Same reason as `_module`, one field up.
    _slots: Box<[*const u8]>,
    calls: usize,
    last: LastCall,
}

impl CompiledBody {
    /// Run the body, calling `natives` by index in order.
    ///
    /// An `Err` is the failure the adapter parked, never an unwind (§S11), and
    /// for a panic it is worded exactly as Tier 0 words it (D49).
    pub fn run(
        &self,
        vm: &mut dyn Vm,
        natives: &[(&str, bund2_api::NativeFn)],
    ) -> Result<(), Error> {
        let mut c = Ctx {
            vm,
            err: None,
            natives,
            body: &[],
            sites: &[],
        };
        // SAFETY: the entry is the trampoline's published address, under the
        // platform's convention, and `&mut c` is live for the whole call. The
        // emitted code treats the pointer as opaque and hands it to the adapter.
        let status = unsafe { (self.entry)(&raw mut c) };
        match c.err {
            Some(e) => Err(e),
            None if status == OK => Ok(()),
            None => Err(Error::internal(
                "a compiled body returned a failing status with no error in the context",
            )),
        }
    }

    /// How many calls the body makes.
    pub fn calls(&self) -> usize {
        self.calls
    }

    /// Where the last one sits.
    pub fn last_call(&self) -> LastCall {
        self.last
    }
}

/// Which Rust adapter a body's thunks call.
///
/// The thunk-and-slot machinery is the same either way — §S8's third piece with
/// §S6's slot-table indirection — and only the adapter behind it differs, so the
/// two lowerings share one emitter rather than drifting apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Adapter {
    /// [`jit_call_native`]: call the native at that index.
    Native,
    /// [`jit_apply`]: apply the body's value at that index, through
    /// `Vm::apply`.
    Apply,
}

impl Adapter {
    /// The symbol the thunks import, and the function behind it.
    ///
    /// This is the **non-tail** adapter: it drains a filed request before it
    /// returns, because compiled code that went on would run the body after the
    /// next value (§S5).
    fn symbol(self) -> (&'static str, *const u8) {
        match self {
            Adapter::Native => ("jit_call_native", jit_call_native as *const u8),
            Adapter::Apply => ("jit_apply", jit_apply as *const u8),
        }
    }

    /// **§S5's drain helper, the same for either adapter.**
    ///
    /// The drain does not depend on what the call was: it runs whatever body
    /// the call left filed. So one symbol serves both lowerings, and the
    /// *position* is decided where it belongs — in the emitted code, which
    /// loads the request cell after a non-tail call and not after a tail one.
    fn drain_symbol() -> (&'static str, *const u8) {
        ("jit_drain", jit_drain as *const u8)
    }

    /// **§S6's type guard**, the same for either adapter: it asks whether a
    /// site's fragment admits the stack, and that question does not depend on
    /// what the surrounding body is doing.
    ///
    /// Reached through a slot like every other callee, so criterion 4's rule
    /// holds: the only relocations stay "a thunk calling its adapter" and "the
    /// trampoline calling the body". Each inlined site gets its own thunk,
    /// baking its site index, exactly as each call gets one baking its index.
    fn admits_symbol() -> (&'static str, *const u8) {
        ("jit_admits", jit_admits as *const u8)
    }
}

/// **Where a compiled word lives: a handle into its [`Compiler`].**
///
/// Deliberately *not* a pointer to code. A compiled word's entry is only valid
/// while the module that emitted it is alive, so a self-contained value holding
/// that pointer would be a dangling reference waiting for its module to be
/// dropped — and nothing in the type system would say so.
///
/// **What a handle does and does not guarantee.** It is an index, so it can
/// only ever reach a word of the compiler it is used against: never freed code,
/// never another module's address, and out of range it answers `None`. It is
/// *not* an identity — every compiler numbers from zero, so a handle from one
/// compiler used against another names that other compiler's word instead.
/// Nothing here mixes them: the cache that stores handles lives in the same
/// `JitTier` as the compiler that issued them, which is the structural reason
/// the question does not arise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordHandle(usize);

/// One compiled word's code, as its [`Compiler`] records it.
struct Word {
    entry: Entry,
    /// The call slots. Held to keep the allocation alive, never read from Rust
    /// — see [`CompiledBody::_slots`], which explains the same field at length.
    _slots: Box<[*const u8]>,
    values: usize,
    last: LastCall,
    /// **The fragments this word inlined, by site index** — §S6.
    ///
    /// Read at run time by [`jit_admits`], through [`Ctx::sites`]: the type
    /// guard is a question about the stack as it stands, so the fragment has to
    /// outlive compilation. Empty for a word that inlined nothing.
    sites: Vec<Fragment>,
    /// **The literals this word baked, by body index** — §S5's promotion.
    ///
    /// A [`Plan::Literal`] becomes an `iconst`, so its value is fixed when the
    /// word is compiled rather than fetched from [`Ctx::body`] the way
    /// [`jit_apply`] fetches every other value. That is sound because a
    /// compiled word is only ever run against the body it was compiled from:
    /// `Tiering`'s cache keys on `BundValue::payload_key` — the payload's `Rc`
    /// pointer, D35 — and a lambda's payload *is* its body, so a cache hit
    /// names the same allocation.
    ///
    /// **Recorded anyway, and checked in [`Compiler::run`].** That soundness
    /// rests on how a *different crate* keys a cache, and nothing here would
    /// notice if it changed: the promoted values would simply be the wrong
    /// ones, silently. Keeping the pairs turns "true because of somebody
    /// else's invariant" into one this crate states and enforces.
    literals: Vec<(usize, i64)>,
    /// **Assumption 38's resume index, per inlined site** — §S5's residual.
    ///
    /// Site index paired with the position in the source body at which that
    /// site's residual path resumes interpretation. The index is the site's
    /// *own* position, because a guard refuses before its value has run, so the
    /// value is still owed and the residual re-applies it through `Vm::apply`.
    ///
    /// Criterion 21 asserts this table directly rather than through the stacks
    /// it produces: "stacks alone would pass a lowering that resumed at the
    /// wrong index on a seventh program".
    resumes: Vec<(usize, usize)>,
    /// **D68's classification, per generic call** — body index, and whether
    /// promotion may cross it.
    ///
    /// Criterion 27 requires "the lowering's side table must record the call as
    /// synced, not crossed", and criterion 22's last bullet asks the same of a
    /// body that promotes. Neither could be written, because no field carried
    /// the distinction: `sites`, `literals` and `resumes` describe what was
    /// inlined and promoted, never what a *call* was permitted.
    ///
    /// **A permission, not a record of what was emitted.** Every call is synced
    /// before today — D68's emitter half is unbuilt — so a `true` here means
    /// "the four gates hold", not "the sync was skipped". When the emitter
    /// learns to cross, this is the table it reads and the criteria assert.
    crossings: Vec<(usize, bool)>,
    /// **Pushes emitted *ahead of a call*, to sync promoted values** — D68's
    /// witness.
    ///
    /// Every crossing test is a differential, and a differential passes just as
    /// well if the emitter synced everything and crossed nothing: syncing early
    /// is always *correct*, merely slower. [`Word::crossings`] records what the
    /// planner decided; nothing recorded what the emitter did.
    ///
    /// This does. A held value is exactly a push that did not happen before the
    /// call, so compiling one body twice — once with the crossable table, once
    /// without — and comparing this count witnesses the crossing in the emitted
    /// code rather than in the plan.
    ///
    /// # Ahead of a call, and nothing else
    ///
    /// **The body's final sync is excluded**, and so are the two spill blocks.
    /// A body that promotes ends with the final sync whether or not it crossed
    /// anything, so counting it adds the same constant to both arms; the spills
    /// sit on failure edges and do not run when the body succeeds. The first
    /// version counted everything and read **2 and 2** for `1 2 nl` — the
    /// synced form pushing both literals before the call, the crossed form
    /// pushing both at the end. Identical totals, opposite code.
    syncs: usize,
}

/// **What the lowering decided about one value of a body** — §S6's join.
///
/// Every value is either applied through `Vm::apply` at run time, exactly as
/// before, or inlined behind three guards. A value is only ever inlined when
/// all of §S6's rules hold, and the generic path stays emitted beside it: a
/// guard that refuses falls through to the very call the value would otherwise
/// have made, so nothing is lost but speed.
#[derive(Debug, Clone)]
enum Plan {
    /// Dispatched: load the slot, call through it, then §S5's request check.
    ///
    /// **`cross` is D68's verdict, and the operand count it turns on.**
    ///
    /// `Some(consumes)` when all four of D68's gates hold: promotion may keep
    /// values in registers across this call, and `consumes` of them — the top
    /// ones — must be pushed first, because the callee pops its operands from
    /// the real stack. `None` when any gate refuses, and then every promoted
    /// value is synced as before.
    ///
    /// **The count is part of the verdict, not a separate lookup.** The
    /// emitter has no `Vm` to ask, and asking again at emit time could answer
    /// differently from the plan — a rebind between the two would make the
    /// emitted sync disagree with the classification that licensed it.
    Call { cross: Option<u8> },
    /// **Promoted: an int literal that never reaches the stack.**
    ///
    /// `Interp::apply_step` sends `CALL` to `dispatch_name` and `CONTEXT` to
    /// the context switch; **every other kind falls to a default arm that is
    /// `self.push(v)` and nothing else**. `autoadd` lives inside
    /// `dispatch_name`, so it cannot reach a literal — which is what makes an
    /// int literal promotable with no meaning guard of its own.
    ///
    /// The value becomes an `iconst` in a `Variable` and is synced to the stack
    /// before the first thing that could observe it (*A sync precedes every
    /// call*, in the body emitter).
    Literal(i64),
    /// Inlined, guarded. `site` indexes the word's fragment table.
    Inline {
        site: usize,
        /// The slot's generation when the fragment was inlined against it. The
        /// emitted guard compares the cell against this and takes the generic
        /// path when they differ (§S6, *Inlining freezes a name*).
        generation: u32,
        /// The address of that name's generation cell, embedded as an
        /// immediate (§S6, *Addressing*).
        cell: i64,
    },
}

/// **One `JITModule`, and every word compiled into it.**
///
/// *One compiler per `Interp`*, which is what makes it one module per `Interp`:
/// `JitTier` owns a `Compiler`, `Interp` owns the tier, so an interpreter's
/// compiled code is emitted into a single module that lives exactly as long as
/// the interpreter does. Two `Interp`s share no code and no cache — criterion
/// 23 — and neither can be served the other's pointers, because a
/// [`WordHandle`] is only meaningful to the compiler that issued it.
///
/// **Why the module is shared rather than one per body.** A module is a code
/// allocator: each one reserves and finalises its own memory, and none of it is
/// ever reclaimed (§S4, and `free_memory` is an `unsafe fn` taking `self` that
/// nothing here is in a position to call). A module per compiled body would
/// multiply that fixed cost by the compiled-function cap. Cranelift supports
/// the sharing directly: `finalize_definitions` takes the pending list rather
/// than reprocessing it, so calling it once per compilation finalises only the
/// functions that compilation defined and leaves every earlier body's code
/// untouched and still executable.
pub struct Compiler {
    emitter: Emitter,
    words: Vec<Word>,
    /// **The fragment table — `(registration id, Fragment)` pairs** §S6 has
    /// `bund2-stdlib` publish and this crate read.
    ///
    /// **Handed in rather than fetched**, because `fragments::published` takes
    /// a `&Registry` and the tier holds only a `&mut dyn Vm` — §S6 designs the
    /// `Vm` trait to expose no registry, and a `Vm` method returning fragments
    /// would put `Fragment` into `bund2-api`, which D9 forbids. `bund2-runtime`
    /// builds the table, since it constructs the `Interp` and has its registry
    /// in hand, and passes it down.
    ///
    /// Empty is a valid table: every site then takes the generic path.
    table: Vec<(bund2_api::RegistrationId, Fragment)>,
    /// **The registrations promotion may cross — D47 and D48, as a table.**
    ///
    /// Handed in for the same reason `table` is: membership comes from
    /// `bund2_stdlib::promotable::crossable`, which takes a `&Registry` that a
    /// tier does not hold. Keyed by registration id rather than name, because
    /// a different native registered under one of those names has a different
    /// id — which is the hazard D47 exists for, `Registry::register_native`
    /// being public.
    ///
    /// **Empty is the safe default and today's behaviour**: no call is
    /// classified crossable, so promotion stops at every one of them.
    crossable: std::collections::BTreeSet<bund2_api::RegistrationId>,
}

/// **A module and the thunks already emitted into it.**
///
/// They travel together because a thunk's `FuncId` is only meaningful for the
/// module that declared it, and because the cache's whole correctness argument
/// is per module: an index is resolved against the running context, so a thunk
/// is shareable by every body *in this module* and by none outside it.
struct Emitter {
    module: JITModule,
    /// **The thunks this module has already emitted, by index** — F130.
    ///
    /// A thunk bakes an index and nothing else, and the index is resolved
    /// **against the running context**: `jit_call_native` takes
    /// `Ctx::natives[i]`, `jit_admits` takes `Ctx::sites[k]`, and the drain
    /// thunk bakes nothing at all. So a thunk baking `2` means "the third
    /// entry of whatever body is running" and is correct for *every* body —
    /// two bodies that each have a third call want the identical four
    /// instructions.
    ///
    /// Emitting one per site per body was **50.8% of a compilation**, measured
    /// by sampling profile: each four-instruction function paid Cranelift's
    /// whole per-function pipeline, its own `FunctionBuilder`, legalisation and
    /// regalloc2 run. Keeping them here means a body pays only for indices no
    /// earlier body reached.
    ///
    /// §S8 asks for no more than this: its third piece is "a JIT-emitted `Tail`
    /// thunk **for each native a call slot can hold**" — per callee kind, not
    /// per call site.
    thunks: Thunks,
}

/// The per-module thunk cache — see [`Emitter::thunks`].
///
/// Three kinds, because three adapters: a call thunk per call index, one drain
/// thunk, and a guard thunk per site index. Each vector is dense and grows only
/// when a body needs an index no earlier body did.
#[derive(Default)]
struct Thunks {
    calls: Vec<cranelift_module::FuncId>,
    drain: Option<cranelift_module::FuncId>,
    admits: Vec<cranelift_module::FuncId>,
}

impl Compiler {
    /// A compiler with an empty module, ready to take bodies.
    ///
    /// `table` is §S6's published fragments. Pass an empty one to compile with
    /// no inlining at all, which is what a `Vm` that publishes nothing gets.
    pub fn new(table: Vec<(bund2_api::RegistrationId, Fragment)>) -> Result<Self, String> {
        Self::with_crossable(table, std::collections::BTreeSet::new())
    }

    /// A compiler that also knows which callees D47 and D48 permit crossing.
    ///
    /// [`Compiler::new`] is this with an empty set, which classifies no call as
    /// crossable — the conservative answer, and what every caller got before
    /// D68. Only `bund2-runtime` has the registry the set is built from.
    pub fn with_crossable(
        table: Vec<(bund2_api::RegistrationId, Fragment)>,
        crossable: std::collections::BTreeSet<bund2_api::RegistrationId>,
    ) -> Result<Self, String> {
        Ok(Self {
            emitter: Emitter {
                module: new_module(Adapter::Apply)?,
                thunks: Thunks::default(),
            },
            words: Vec::new(),
            table,
            crossable,
        })
    }

    /// **How many thunks this module holds** — F130's cache, as a figure a
    /// test can assert on.
    ///
    /// Call thunks plus guard thunks plus the drain thunk. It counts what has
    /// been *emitted*, not what bodies asked for, so it stops growing once a
    /// body's indices have all been seen — which is the whole claim the cache
    /// makes and the only way to check it from outside.
    pub fn thunk_count(&self) -> usize {
        let t = &self.emitter.thunks;
        t.calls.len() + t.admits.len() + usize::from(t.drain.is_some())
    }

    /// **D68's four gates, asked of one callee.**
    ///
    /// Promotion may cross a call only when every one holds:
    ///
    /// - **D46** — the name resolves to a **native**, not a lambda. A lambda
    ///   has no declared effect, and its own callee can be rebound underneath
    ///   it by a program the pre-call check cannot see.
    /// - **D47** — `bund2-stdlib` registered it. An embedder's native carries a
    ///   declared effect nothing has audited, and `register_native` is public.
    /// - **D48** — it is on `PROMOTABLE.txt`, which is what
    ///   [`crate::Compiler::with_crossable`] is handed. D47 and D48 are
    ///   answered together, because only `bund2-stdlib` mints ids for those
    ///   names.
    /// - **D68** — its declared effect **produces nothing**, and is not
    ///   `opaque`. A callee that leaves a result puts it above the promoted
    ///   values permanently, so the final sync would write them beneath it —
    ///   "wrong silently, and no guard catches it".
    ///
    /// A value that is not a `CALL` is not a callee at all. A `CONTEXT`
    /// literal reaches the generic path too and is never crossed: §S5 makes it
    /// a static barrier, and it does not resolve to a native.
    /// `Some(consumes)` when every gate holds, and the count the emitter needs
    /// to push before the call.
    fn crossable_callee(&self, v: &BundValue, vm: &mut dyn Vm) -> Option<u8> {
        if v.dt() != bund2_value::CALL {
            return None;
        }
        let name = v.as_str()?;
        // D46: a lambda is never crossed, whatever its inferred effect.
        if vm.is_lambda(&name) || !vm.is_native(&name) {
            return None;
        }
        // D68: produces nothing, and the pair means what it says.
        let effect = vm.effect_of(&name)?;
        if effect.opaque || effect.produces != 0 {
            return None;
        }
        // D47 and D48: the registration is one the audit certified.
        let site = vm.inline_site(&name)?;
        if !self.crossable.contains(&site.registration) {
            return None;
        }
        Some(effect.consumes)
    }

    /// **Compile a Bund word's body — the first lowering of a real word.**
    ///
    /// `values` is the body's length. Each value gets a `Tail` thunk that calls
    /// [`jit_apply`] with its index, and the body calls each thunk indirectly
    /// through its slot, exactly as the native lowering does — so criterion 4's
    /// property holds here too: no call between compiled functions names a
    /// `FuncId`.
    ///
    /// See [`jit_apply`] for why every value goes through `Vm::apply` rather
    /// than being lowered directly, and for what that does and does not buy.
    ///
    /// **Never call this while compiled code is running.** Emitting into the
    /// module finalises it, and the seam guarantees the quiet moment: `Interp`
    /// takes the tier out for the duration of a compiled body, so nothing
    /// reaches a compiler that is already inside one of its own words.
    /// `cells` is [`bund2_api::Cells::base`] for the `Interp` this compiler
    /// serves — §S6's addressing. The emitted body loads the request cell
    /// through it after every non-tail call, so a body compiled against one
    /// interpreter's cells must never be run by another; the per-`Interp`
    /// compiler is what guarantees that (criterion 23).
    pub fn compile_word(
        &mut self,
        body: &[BundValue],
        last: LastCall,
        cells: usize,
        vm: &mut dyn Vm,
    ) -> Result<WordHandle, String> {
        let values = body.len();
        if values == 0 {
            return Err("a word with an empty body has nothing to lower".into());
        }
        let cells =
            i64::try_from(cells).map_err(|_| "the cells' address does not fit".to_string())?;
        let seq = self.words.len();
        let (plan, sites) = self.plan_body(body, vm);
        let (entry, slots, resumes, syncs) =
            emit_into(
                &mut self.emitter,
                seq,
                &plan,
                &sites,
                last,
                Adapter::Apply,
                cells,
            )?;
        // The constants the emitted code baked, paired with the body positions
        // they came from. `Compiler::run` compares them against the body it is
        // handed — see [`Word::literals`].
        let literals = plan
            .iter()
            .enumerate()
            .filter_map(|(i, p)| match p {
                Plan::Literal(n) => Some((i, *n)),
                _ => None,
            })
            .collect();
        // **D68's classification, kept per body index.** Only generic calls
        // carry it: an inlined site is not a call the promoted set crosses, and
        // a promoted literal is not a call at all.
        let crossings = plan
            .iter()
            .enumerate()
            .filter_map(|(i, p)| match p {
                Plan::Call { cross } => Some((i, cross.is_some())),
                _ => None,
            })
            .collect();
        self.words.push(Word {
            entry,
            _slots: slots,
            values,
            last,
            sites,
            literals,
            resumes,
            crossings,
            syncs,
        });
        Ok(WordHandle(seq))
    }

    /// **Plan a body's values — §S6's join, at compile time.**
    ///
    /// Each value becomes either [`Plan::Call`], dispatched through its slot as
    /// every value is today, or [`Plan::Inline`] behind three guards. A value
    /// is inlined only when every one of §S6's rules holds, and each rule that
    /// does not simply yields a call:
    ///
    /// - the value is a `CALL` naming a word — a literal or a `CONTEXT` is not
    ///   a site at all;
    /// - [`Vm::inline_site`] answers, which is where §S6's three rules live:
    ///   a direct resolution, an unsaturated slot, and a registration id;
    /// - this compiler's table publishes a fragment for **that registration**,
    ///   not merely for that name.
    ///
    /// The fragments collected here travel with the word, because the type
    /// guard is asked at run time against the stack as it stands.
    fn plan_body(&self, body: &[BundValue], vm: &mut dyn Vm) -> (Vec<Plan>, Vec<Fragment>) {
        let mut plan = Vec::with_capacity(body.len());
        let mut sites = Vec::new();
        for v in body {
            let inlined = (v.dt() == bund2_value::CALL)
                .then(|| v.as_str())
                .flatten()
                .and_then(|name| vm.inline_site(&name))
                .and_then(|s| {
                    let fragment = self
                        .table
                        .iter()
                        .find(|(id, _)| *id == s.registration)
                        .map(|(_, f)| f.clone())?;
                    let cell = i64::try_from(s.cell).ok()?;
                    Some((fragment, s.generation, cell))
                });
            match inlined {
                Some((fragment, generation, cell)) => {
                    let site = sites.len();
                    sites.push(fragment);
                    plan.push(Plan::Inline {
                        site,
                        generation,
                        cell,
                    });
                }
                // **Not a site — but possibly a promotion.** An int literal is
                // an unconditional push (`Interp::apply_step`'s default arm),
                // so it can live in a register until something could observe
                // it. Every other kind, `CONTEXT` included, stays a call: a
                // CONTEXT literal switches stacks with no call at all, which
                // §S5 makes a static barrier, and routing it through the slot
                // is how the sync lands before the switch.
                None => match (v.dt() == bund2_value::INTEGER)
                    .then(|| v.as_int())
                    .flatten()
                {
                    Some(n) => plan.push(Plan::Literal(n)),
                    None => plan.push(Plan::Call {
                        cross: self.crossable_callee(v, vm),
                    }),
                },
            }
        }
        (plan, sites)
    }

    /// **Would compiling this body buy anything at all?** — F133.
    ///
    /// A compiled body runs every value that is *not* an inlined site through
    /// [`jit_apply`], which calls `Vm::apply` — Tier 0's own path — and adds the
    /// boundary around it: an entry trampoline, a slot load and an indirect call
    /// per value, plus the request-cell load after each. Inlining and promotion
    /// are what repay that. A body with neither is strictly slower compiled than
    /// interpreted, measured at **+24%** on `arith/float_mul/1000`, whose
    /// `--stats` read 0 sites inlined and 0 values promoted.
    ///
    /// **The answer is [`plan_body`](Self::plan_body)'s own, not a second
    /// opinion.** The planning that decides what gets emitted decides whether to
    /// emit at all, so the rule cannot drift from the lowering it guards — which
    /// is how F127 went wrong, with a fixture that resembled the real path.
    ///
    /// Whether to *act* on this is §S7's policy and belongs to the tier, not
    /// here: `bund2-jit` stays the mechanism.
    ///
    /// # The test is a site, not any gain at all — F136
    ///
    /// This first answered "does the body gain **anything**": a site *or* a
    /// promoted literal. That refused the body F133 found (0 sites, 0 promoted,
    /// +24%) and admitted one that still loses — a body of one literal and
    /// twenty generic calls has a promoted value, so it compiled, then paid
    /// 20 × 5.13 ns to save 1.13. Two of the corpus's nine compiled bodies are
    /// exactly that shape, at −4.0 ns an entry.
    ///
    /// **An inlined site is the only gain that pays for compiling.** Criterion
    /// 10's decomposition: a site saves **24.19 ns** per entry, a promoted
    /// literal **1.13 ns**, and a generic value **costs 5.13 ns**. Set beside
    /// F130's compile cost of **125–141 µs**, a body whose whole gain is
    /// promotion needs on the order of forty thousand entries to repay being
    /// compiled; one with a site repays it in a few thousand. So a body with no
    /// site is refused whatever it promotes.
    ///
    /// **This refuses pure-literal bodies too**, which do gain 1.13 ns a value.
    /// That is deliberate, on the compile-cost argument above, and it is the
    /// rule the repository owner chose over a ratio test — which on the corpus
    /// refused the same bodies at every fraction from 1/8 to 1/4 and began
    /// refusing winners at 1/3.
    pub fn would_gain(&self, body: &[BundValue], vm: &mut dyn Vm) -> bool {
        let (_plan, sites) = self.plan_body(body, vm);
        !sites.is_empty()
    }

    /// Apply the body's values in order, through Tier 0's own `apply`.
    ///
    /// `body` must be the same length the word was compiled for; a shorter one
    /// is a broken invariant and reported as such rather than silently short.
    /// A handle this compiler never issued is the same kind of invariant, and
    /// is reported rather than indexed with.
    pub fn run(&self, word: WordHandle, vm: &mut dyn Vm, body: &[BundValue]) -> Result<(), Error> {
        let Some(w) = self.words.get(word.0) else {
            return Err(Error::internal(
                "a compiled word was run against a compiler that never issued its handle",
            ));
        };
        if body.len() != w.values {
            return Err(Error::internal(format!(
                "a compiled word was built for {} values and handed {}",
                w.values,
                body.len()
            )));
        }
        // **The promoted literals must be the ones that were baked.** Every
        // other value is fetched from `Ctx::body` at run time by [`jit_apply`],
        // so a compiled word is otherwise indifferent to which body of the
        // right length it is given; a [`Plan::Literal`] is not, because its
        // value is an `iconst`. The cache makes this true already — it keys on
        // the payload's `Rc` pointer, so a hit is the same body — but that is
        // another crate's invariant, and a body that disagreed would otherwise
        // push the wrong numbers with nothing to say so.
        for &(i, n) in &w.literals {
            if body.get(i).and_then(BundValue::as_int) != Some(n) {
                return Err(Error::internal(format!(
                    "a compiled word baked the literal {n} at body position {i} and was handed a \
                     body holding {} there",
                    body.get(i).map_or("nothing".to_string(), |v| v.summary(40))
                )));
            }
        }
        let mut c = Ctx {
            vm,
            err: None,
            natives: &[],
            body,
            // The word's own fragments, so `jit_admits` can ask each site's
            // guard against the stack as it stands.
            sites: &w.sites,
        };
        // SAFETY: as `CompiledBody::run`'s — the trampoline's published address
        // under the platform's convention, with `&mut c` live for the call. The
        // code is alive because `self` owns the module that emitted it.
        let status = unsafe { (w.entry)(&raw mut c) };
        match c.err {
            Some(e) => Err(e),
            None if status == OK => Ok(()),
            None => Err(Error::internal(
                "a compiled word returned a failing status with no error in the context",
            )),
        }
    }

    /// How many values the word applies.
    pub fn values(&self, word: WordHandle) -> Option<usize> {
        self.words.get(word.0).map(|w| w.values)
    }

    /// Where the last one sits.
    pub fn last_call(&self, word: WordHandle) -> Option<LastCall> {
        self.words.get(word.0).map(|w| w.last)
    }

    /// How many words this compiler has emitted.
    pub fn compiled_words(&self) -> usize {
        self.words.len()
    }

    /// **How many of a word's values were inlined** — §S6's join, as a number a
    /// test can read.
    ///
    /// Without this, "did it inline?" could only be inferred from timing, which
    /// is precisely the evidence that cannot distinguish an inlined arm from a
    /// fast generic call. `None` for a handle this compiler never issued.
    pub fn inlined_sites(&self, word: WordHandle) -> Option<usize> {
        self.words.get(word.0).map(|w| w.sites.len())
    }

    /// **Inlined sites across every word this compiler emitted.**
    ///
    /// What `bund2 --stats` reports beside the body count, and the figure that
    /// makes a timing attributable: a compiled body that inlined nothing is
    /// still routing every value through `Vm::apply`, which is the shape
    /// criterion 10's dated note excludes from its A/B. Without this, a
    /// measurement cannot tell "the lowering does not pay" from "the lowering
    /// did not happen".
    pub fn inlined_total(&self) -> usize {
        self.words.iter().map(|w| w.sites.len()).sum()
    }

    /// **How many values this word promoted** — §S5, and the figure that makes
    /// promotion's firing checkable.
    ///
    /// A body whose literals all went to registers and a body that promoted
    /// nothing produce the *same answers*, because the unpromoted path is the
    /// old one and is correct. So a green differential is no evidence that
    /// promotion ran at all. This counts what [`Plan::Literal`] planned, which
    /// is exactly what the emitter turned into an `iconst`.
    pub fn promoted_values(&self, word: WordHandle) -> Option<usize> {
        self.words.get(word.0).map(|w| w.literals.len())
    }

    /// Promoted values across every word this compiler emitted.
    pub fn promoted_total(&self) -> usize {
        self.words.iter().map(|w| w.literals.len()).sum()
    }

    /// **Every generic call this word planned, and whether D68 permits crossing
    /// it** — body index, then the verdict.
    ///
    /// The table criterion 27 and criterion 22's last bullet ask for. `None`
    /// for a word this compiler never issued.
    ///
    /// **A permission, not a report of emitted code.** The emitter syncs before
    /// every call until D68's second half is built, so a `true` means the four
    /// gates held, not that the sync was skipped.
    pub fn crossings(&self, word: WordHandle) -> Option<&[(usize, bool)]> {
        self.words.get(word.0).map(|w| w.crossings.as_slice())
    }

    /// How many of this word's calls D68 permits crossing. `None` for a word
    /// this compiler never issued.
    pub fn crossable_calls(&self, word: WordHandle) -> Option<usize> {
        self.words
            .get(word.0)
            .map(|w| w.crossings.iter().filter(|(_, c)| *c).count())
    }

    /// **Pushes this word emits ahead of a call, to sync promoted values** —
    /// D68's witness. `None` for a word this compiler never issued.
    ///
    /// A held value is a push that did not happen before the call, so compiling
    /// one body twice — with the crossable table and without — and comparing
    /// this count witnesses the crossing in the emitted code rather than in the
    /// plan. The body's final sync is excluded: it runs either way, and
    /// counting it hides the difference.
    pub fn syncs(&self, word: WordHandle) -> Option<usize> {
        self.words.get(word.0).map(|w| w.syncs)
    }

    /// Crossable calls across every word this compiler emitted.
    pub fn crossable_total(&self) -> usize {
        self.words
            .iter()
            .map(|w| w.crossings.iter().filter(|(_, c)| *c).count())
            .sum()
    }

    /// **Values across every word this compiler emitted** — the denominator the
    /// other three figures were missing (F136).
    ///
    /// `inlined_total` and `promoted_total` say what a body *gained*. Neither
    /// says what it *paid*, because a value that is neither a site nor a
    /// promoted literal takes the generic path — slot load, `call_indirect`,
    /// the request-cell load — and that path is **5.13 ns dearer than Tier 0**
    /// (criterion 10, *The per-value cost decomposed*). The count of those
    /// values is `values_total − inlined_total − promoted_total`, and without
    /// this figure it cannot be had: F133's rule can see that a body gains
    /// something, but not whether the gain outweighs what the rest of the body
    /// costs.
    pub fn values_total(&self) -> usize {
        self.words.iter().map(|w| w.values).sum()
    }

    /// **Where site `site`'s residual resumes in the source body** —
    /// assumption 38, asserted by criterion 21.
    ///
    /// `None` for a word this compiler never issued, or a site it never
    /// inlined. The criterion wants the table itself, not its consequences.
    pub fn resume_index(&self, word: WordHandle, site: usize) -> Option<usize> {
        self.words
            .get(word.0)?
            .resumes
            .iter()
            .find(|(s, _)| *s == site)
            .map(|(_, at)| *at)
    }

    /// Every resume index this word carries, by site — the whole side table.
    pub fn resume_table(&self, word: WordHandle) -> Option<&[(usize, usize)]> {
        self.words.get(word.0).map(|w| w.resumes.as_slice())
    }
}

/// **Compile a body that calls `calls` natives in order — §S8's pieces 2 and 3
/// wired to a call site.**
///
/// Emits, per native, a `Tail` **thunk** (§S8's third piece) that makes an
/// ordinary call to the Rust adapter; a `Tail` **body** that loads each thunk's
/// address from its slot and calls through it; and the C-convention **entry
/// trampoline** Rust enters. Every call from the body is `call_indirect`, or
/// `return_call_indirect` for the last when `last` is [`LastCall::Tail`].
///
/// **What this is not.** There is no cache, no `Interp` integration, no
/// promotion and no meaning guard: the body is a fixed sequence of calls, not a
/// compiled Bund word. Its purpose is to give the adapter and the thunks a call
/// site, so criterion 29 can run and §S8's boundary is exercised end to end.
pub fn compile_body(calls: usize, last: LastCall, cells: usize) -> Result<CompiledBody, String> {
    if calls == 0 {
        return Err("a body that calls nothing has no call site to exercise".into());
    }
    let cells = i64::try_from(cells).map_err(|_| "the cells' address does not fit".to_string())?;
    emit_body(calls, last, Adapter::Native, cells)
}

/// The shared emitter behind [`compile_body`] and [`Compiler::compile_word`].
///
/// Everything below the adapter is the same for both — §S8's `Tail` thunks, the
/// slot table §S6 addresses, the entry trampoline, the status protocol — so the
/// two lowerings share one emitter and cannot drift apart in the parts criteria
/// 4 and 29 are about.
fn emit_body(
    calls: usize,
    last: LastCall,
    adapter: Adapter,
    cells: i64,
) -> Result<CompiledBody, String> {
    let mut emitter = Emitter {
        module: new_module(adapter)?,
        // This path builds a module per body and throws it away, so the cache
        // has nothing to share with: a fresh one, used once.
        thunks: Thunks::default(),
    };
    // The native-calling lowering inlines nothing: its "values" are natives to
    // call, not a Bund body to plan over. Every site is generic.
    // **Never crossable.** This path lowers a body of `calls` generic values
    // with no `Vm` to classify them against — `compile_body` builds a shape,
    // not a program — so D68's gates cannot be asked and the conservative
    // answer is the only sound one.
    let plan = vec![Plan::Call { cross: None }; calls];
    // A body compiled this way inlines nothing — it is given no fragments — so
    // it has no site whose residual could resume, and the table is empty.
    let (entry, slots, _resumes, _syncs) =
        emit_into(
            &mut emitter,
            0,
            &plan,
            &[],
            last,
            adapter,
            cells,
        )?;
    Ok(CompiledBody {
        _module: emitter.module,
        entry,
        _slots: slots,
        calls,
        last,
    })
}

/// A fresh module with `adapter`'s symbol bound.
///
/// The symbol must be bound on the *builder*, before the module exists, which
/// is why an adapter is chosen once per module rather than once per body: every
/// body emitted into a module shares its adapter.
fn new_module(adapter: Adapter) -> Result<JITModule, String> {
    let (name, ptr) = adapter.symbol();
    let (drain_name, drain_ptr) = Adapter::drain_symbol();
    let mut builder =
        JITBuilder::new(default_libcall_names()).map_err(|e| format!("JIT builder: {e}"))?;
    builder.symbol(name, ptr);
    // §S5's drain helper, which the body reaches through its own slot when the
    // request cell is set.
    builder.symbol(drain_name, drain_ptr);
    // **§S6's type guard and the four fragment helpers.** A body that inlines
    // an arm reaches all five: `jit_admits` to ask whether the guard admits,
    // and the stack helpers the arm's ops call. Bound unconditionally — a
    // symbol nothing imports costs nothing, and binding them per-body would
    // mean deciding at module construction what a later body may inline.
    let (admits_name, admits_ptr) = Adapter::admits_symbol();
    builder.symbol(admits_name, admits_ptr);
    builder.symbol("jit_pop_int", jit_pop_int as *const u8);
    builder.symbol("jit_push_int", jit_push_int as *const u8);
    builder.symbol("jit_dup_top", jit_dup_top as *const u8);
    builder.symbol("jit_drop_top", jit_drop_top as *const u8);
    // §S5's residual path. Bound unconditionally for the same reason the five
    // above are: a symbol nothing imports costs nothing.
    builder.symbol("jit_residual", jit_residual as *const u8);
    Ok(JITModule::new(builder))
}

/// What [`emit_into`] hands back: the entry trampoline, the slot table the
/// emitted code reads, and assumption 38's resume index per inlined site.
///
/// A named type because the triple is wide enough that clippy's
/// `type_complexity` is right about it, and because the third member is a side
/// table criterion 21 asserts rather than an implementation detail.
type Emitted = (Entry, Box<[*const u8]>, Vec<(usize, usize)>, usize);

/// **Emit one body into an existing module**, returning its entry and the slot
/// table the emitted code reads.
///
/// `seq` distinguishes this body's functions from every other body's in the
/// same module. Cranelift's `declare_function` *merges* a duplicate name into
/// the existing `FuncId` rather than failing, so without the suffix a second
/// body would silently redefine the first's — the failure would be a wrong
/// answer, not an error. The adapter import is the one name deliberately left
/// unsuffixed: merging is exactly right for it, since every body imports the
/// same Rust function.
fn emit_into(
    e: &mut Emitter,
    seq: usize,
    plan: &[Plan],
    sites: &[Fragment],
    last: LastCall,
    adapter: Adapter,
    cells: i64,
) -> Result<Emitted, String> {
    // Destructured once, so the body below reads as it did when the module and
    // the cache were separate parameters — and so this function stays inside
    // clippy's argument limit, which bundling them is the honest way to meet.
    let Emitter { module, thunks } = e;
    // One value, one call slot — whether or not the value is inlined, because
    // an inlined site keeps its generic path beside it and reaches it through
    // that slot when a guard refuses.
    let calls = plan.len();
    let inlined = plan
        .iter()
        .filter(|p| matches!(p, Plan::Inline { .. }))
        .count();
    let adapter_name = adapter.symbol().0;
    let drain_adapter_name = Adapter::drain_symbol().0;
    // **A body must know where its cells are.** §S6 has the lowering embed the
    // address as an immediate; without one the emitted load would read address
    // zero. Refused rather than emitted, because the failure would be a fault
    // in machine code rather than an error a caller can act on.
    if cells == 0 {
        return Err("a body cannot be lowered without the address of its cells".into());
    }
    let frontend = module.target_config();
    let ptr = frontend.pointer_type();
    let conv = module.isa().default_call_conv();

    // The slots, allocated before anything is compiled so the base address the
    // body embeds is final.
    // **`calls + 1`**: one slot per call, and one more for §S5's drain thunk.
    // The body reaches the drain through a slot like any other callee, so
    // criterion 4's rule holds unchanged — no call between compiled functions
    // names a `FuncId`, and the only relocations stay "a thunk calling its
    // adapter" and "the trampoline calling the body".
    //
    // **And one per inlined site**, for §S6's type guard: an inlined site asks
    // `jit_admits` through a slot for the same reason a call reaches its
    // adapter through one.
    let mut slots: Box<[*const u8]> =
        vec![std::ptr::null(); calls + 1 + inlined].into_boxed_slice();
    let slots_base = slots.as_ptr() as i64;
    let drain_slot = calls;
    let admits_slot_base = calls + 1;

    // The adapter: `(ctx, native) -> status`, under the platform's convention,
    // because it is a Rust function.
    let mut sig_adapter = Signature::new(conv);
    sig_adapter.params.push(AbiParam::new(ptr));
    sig_adapter.params.push(AbiParam::new(ptr));
    sig_adapter.returns.push(AbiParam::new(types::I32));
    let adapter_id = module
        .declare_function(adapter_name, Linkage::Import, &sig_adapter)
        .map_err(|e| format!("declare {adapter_name}: {e}"))?;
    // **§S5's drain helper**, same signature, reached through its own thunk
    // when the request cell says a body is waiting.
    let drain_adapter_id = module
        .declare_function(drain_adapter_name, Linkage::Import, &sig_adapter)
        .map_err(|e| format!("declare {drain_adapter_name}: {e}"))?;
    // **§S6's type guard**, same `(ctx, index) -> status` shape: the index is
    // the site, and the answer is a verdict rather than a status (see `ADMIT`).
    let admits_adapter_name = Adapter::admits_symbol().0;
    let admits_adapter_id = module
        .declare_function(admits_adapter_name, Linkage::Import, &sig_adapter)
        .map_err(|e| format!("declare {admits_adapter_name}: {e}"))?;

    // The four stack helpers an inlined arm's ops call. Declared whether or not
    // this body inlines: an import nothing references costs nothing, and
    // deciding here would mean knowing at module level what a later body wants.
    let mut sig_pop = Signature::new(conv);
    sig_pop.params.push(AbiParam::new(ptr));
    sig_pop.params.push(AbiParam::new(ptr));
    sig_pop.returns.push(AbiParam::new(types::I32));
    let mut sig_push = Signature::new(conv);
    sig_push.params.push(AbiParam::new(ptr));
    sig_push.params.push(AbiParam::new(types::I64));
    sig_push.returns.push(AbiParam::new(types::I32));
    let mut sig_bare = Signature::new(conv);
    sig_bare.params.push(AbiParam::new(ptr));
    sig_bare.returns.push(AbiParam::new(types::I32));
    // §S5's residual: `(ctx, resume index) -> status`, the same shape the
    // apply adapter takes, because it answers the same kind of question.
    let mut sig_residual = Signature::new(conv);
    sig_residual.params.push(AbiParam::new(ptr));
    sig_residual.params.push(AbiParam::new(ptr));
    sig_residual.returns.push(AbiParam::new(types::I32));
    let residual_id = module
        .declare_function("jit_residual", Linkage::Import, &sig_residual)
        .map_err(|e| format!("declare jit_residual: {e}"))?;
    let pop_id = module
        .declare_function("jit_pop_int", Linkage::Import, &sig_pop)
        .map_err(|e| format!("declare jit_pop_int: {e}"))?;
    let push_id = module
        .declare_function("jit_push_int", Linkage::Import, &sig_push)
        .map_err(|e| format!("declare jit_push_int: {e}"))?;
    let dup_id = module
        .declare_function("jit_dup_top", Linkage::Import, &sig_bare)
        .map_err(|e| format!("declare jit_dup_top: {e}"))?;
    let drop_id = module
        .declare_function("jit_drop_top", Linkage::Import, &sig_bare)
        .map_err(|e| format!("declare jit_drop_top: {e}"))?;

    // Thunks and the body are `Tail`, so `return_call_indirect` is legal from
    // any tail position: "body to body, and body to a native's thunk".
    let mut sig_tail = Signature::new(CallConv::Tail);
    sig_tail.params.push(AbiParam::new(ptr));
    sig_tail.returns.push(AbiParam::new(types::I32));

    // **One thunk per call, then the drain thunk, then one per inlined site**
    // — the slot table's shape, and the cache's keys (F130).
    //
    // A thunk bakes an index and nothing else, and the index is resolved
    // against the **running** context: `jit_call_native` takes
    // `Ctx::natives[i]` and `jit_admits` takes `Ctx::sites[k]`. So a thunk an
    // earlier body caused to be emitted is correct for this one, and the names
    // carry no `seq`. `fresh` records which had to be emitted now; only those
    // are defined below, and a body pays only for indices no earlier body
    // reached.
    let mut thunk_ids = Vec::with_capacity(calls + 1 + inlined);
    let mut fresh = Vec::with_capacity(calls + 1 + inlined);
    for i in 0..calls {
        if let Some(&id) = thunks.calls.get(i) {
            thunk_ids.push(id);
            fresh.push(false);
        } else {
            let id = module
                .declare_function(&format!("bund2_thunk_{i}"), Linkage::Export, &sig_tail)
                .map_err(|e| format!("declare thunk {i}: {e}"))?;
            thunks.calls.push(id);
            thunk_ids.push(id);
            fresh.push(true);
        }
    }
    if let Some(id) = thunks.drain {
        thunk_ids.push(id);
        fresh.push(false);
    } else {
        let id = module
            .declare_function("bund2_drain", Linkage::Export, &sig_tail)
            .map_err(|e| format!("declare bund2_drain: {e}"))?;
        thunks.drain = Some(id);
        thunk_ids.push(id);
        fresh.push(true);
    }
    for k in 0..inlined {
        if let Some(&id) = thunks.admits.get(k) {
            thunk_ids.push(id);
            fresh.push(false);
        } else {
            let id = module
                .declare_function(&format!("bund2_admits_{k}"), Linkage::Export, &sig_tail)
                .map_err(|e| format!("declare bund2_admits_{k}: {e}"))?;
            thunks.admits.push(id);
            thunk_ids.push(id);
            fresh.push(true);
        }
    }
    let body_id = module
        .declare_function(&format!("bund2_body_{seq}"), Linkage::Export, &sig_tail)
        .map_err(|e| format!("declare bund2_body_{seq}: {e}"))?;

    let mut sig_entry = Signature::new(conv);
    sig_entry.params.push(AbiParam::new(ptr));
    sig_entry.returns.push(AbiParam::new(types::I32));
    let entry_id = module
        .declare_function(&format!("bund2_entry_{seq}"), Linkage::Export, &sig_entry)
        .map_err(|e| format!("declare bund2_entry_{seq}: {e}"))?;

    let mut cg = module.make_context();
    let mut fb_ctx = FunctionBuilderContext::new();
    // **Pushes on the success path only** — D68's witness, see [`Word::syncs`].
    // The two spill sites (a crossed call's failure edge, and the residual's)
    // are excluded deliberately: they do not run when the body succeeds, and
    // counting them would make a crossed body's total *rise* as its
    // success-path pushes fall, inverting the very comparison this exists for.
    //
    // Declared out here and assigned inside, as `side_table` and `resume_table`
    // are: the body is built in a block that borrows the function, and the
    // return needs the figure after that borrow ends.
    let syncs: usize;
    // Filled by the body emitter, read by the dominance check after it.
    let side_table: Vec<(Block, Block)>;
    // Assumption 38's map, escaping the builder's block the way `side_table`
    // does: site index, and the source-body index its residual resumes at.
    let resume_table: Vec<(usize, usize)>;

    // --- the thunks: one ordinary call to the adapter, with the index baked in
    for (i, &id) in thunk_ids.iter().enumerate() {
        // **Already in this module, from an earlier body** (F130). Its code is
        // correct here because the index it bakes is resolved against the
        // running context, not against the body that caused it to be emitted.
        // Re-defining it would also be an error: a `FuncId` may be defined
        // once.
        if !fresh.get(i).copied().unwrap_or(true) {
            continue;
        }
        cg.func.signature = sig_tail.clone();
        {
            let mut f = FunctionBuilder::new(&mut cg.func, &mut fb_ctx);
            // **One adapter for every call.** Position is not the adapter's
            // business: the emitted body decides whether to drain, by loading
            // the request cell after a non-tail call and not after a tail one.
            // The last thunk in this vector is the drain thunk, which imports
            // the drain adapter instead.
            // Three kinds now: the call thunks, the one drain thunk, and a
            // guard thunk per inlined site. The index each bakes differs too —
            // a call thunk's is its call, a guard thunk's is its *site*.
            let (which, baked) = if i < calls {
                (adapter_id, i)
            } else if i == drain_slot {
                (drain_adapter_id, 0)
            } else {
                (admits_adapter_id, i - admits_slot_base)
            };
            let adapter = module.declare_func_in_func(which, f.func);
            let block = f.create_block();
            f.append_block_params_for_function_params(block);
            f.switch_to_block(block);
            let ctx_val = f.block_params(block)[0];
            let idx = f.ins().iconst(ptr, baked as i64);
            // The one relocation criterion 4 allows on this side: "a native's
            // `Tail` thunk calling its Rust adapter".
            let call = f.ins().call(adapter, &[ctx_val, idx]);
            let status = f.inst_results(call)[0];
            f.ins().return_(&[status]);
            f.seal_all_blocks();
            f.finalize(frontend);
        }
        module
            .define_function(id, &mut cg)
            .map_err(|e| format!("define thunk {i}: {e}"))?;
        module.clear_context(&mut cg);
    }

    // --- the body: load each slot, call through it
    cg.func.signature = sig_tail.clone();
    {
        let mut f = FunctionBuilder::new(&mut cg.func, &mut fb_ctx);
        let tail_sig = f.import_signature(sig_tail.clone());
        let block = f.create_block();
        let fail = f.create_block();
        f.append_block_params_for_function_params(block);
        f.switch_to_block(block);
        let ctx_val = f.block_params(block)[0];
        let base = f.ins().iconst(ptr, slots_base);
        // **§S6's cells, addressed as an immediate.** The allocation outlives
        // every compiled function — both die with the `Interp` — which is what
        // makes embedding its address safe.
        let cells_base = f.ins().iconst(ptr, cells);
        let request_off = i32::try_from(bund2_api::Cells::request_offset())
            .map_err(|_| "the request cell's offset does not fit".to_string())?;
        let autoadd_off = i32::try_from(bund2_api::Cells::autoadd_offset())
            .map_err(|_| "the autoadd cell's offset does not fit".to_string())?;

        // What an inlined arm's ops need: the four helpers, and one slot for a
        // popped int. Created whether or not this body inlines — an unused
        // import references nothing and an unused slot is eight bytes.
        let pop = module.declare_func_in_func(pop_id, f.func);
        let push = module.declare_func_in_func(push_id, f.func);
        let dup = module.declare_func_in_func(dup_id, f.func);
        let drop_ = module.declare_func_in_func(drop_id, f.func);
        // §S5's residual path, reached when an inlined site's meaning guard
        // refuses. Imported whether or not this body inlines, for the same
        // reason the four above are.
        let residual = module.declare_func_in_func(residual_id, f.func);
        let pop_slot = f.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            8,
            3,
        ));

        // Criterion 17's side table: for each inlined region, the block its
        // guard was decided in and the block its ops begin in.
        let mut regions: Vec<(Block, Block)> = Vec::with_capacity(inlined);

        // **Assumption 38's side table**: for each inlined site, the index in
        // the source body at which its residual path resumes. Criterion 21
        // asserts this directly rather than through its effects, because
        // "stacks alone would pass a lowering that resumed at the wrong index".
        let mut resumes: Vec<(usize, usize)> = Vec::with_capacity(inlined);

        // The helpers an arm or a sync reaches, gathered once. Hoisted out of
        // the loop because a *sync* needs them at points where no site is being
        // emitted at all — before a call, before a tail call, before the return.
        let helpers = FragmentSite {
            ptr,
            slot: pop_slot,
            ctx_val,
            fail,
            pop,
            push,
            dup,
            drop_,
        };

        // **§S5's promotion, at compile time.** The values an int literal would
        // have pushed, held in `Variable`s instead. The end of this vector is
        // the top of the modelled stack.
        //
        // **A sync precedes every call**, so the model is empty at every edge
        // that can branch to `fail` — which makes the error paths correct by
        // construction rather than by enumerating them. It is also what keeps
        // "the stack a value is synced to" and "the stack it came from" the
        // same stack (criterion 21): only a `CALL` or a `CONTEXT` literal can
        // move the current stack, both are [`Plan::Call`], and both sync first.
        let mut promoted: Vec<Variable> = Vec::new();

        let mut syncs_here = 0usize;

        let width = i32::try_from(ptr.bytes()).map_err(|_| "pointer too wide".to_string())?;
        let drain_off = i32::try_from(drain_slot)
            .map_err(|_| "the drain slot index does not fit an offset".to_string())?
            .checked_mul(width)
            .ok_or_else(|| "the drain slot's offset overflows".to_string())?;
        for i in 0..calls {
            // The slot's offset rides in `load`'s own `Offset32`, rather than an
            // `iadd_imm` before it: one instruction fewer per call, and
            // `iadd_imm` is deprecated at 0.135 in favour of the explicitly
            // sign- or zero-extending forms.
            let off = i32::try_from(i)
                .map_err(|_| "call index does not fit an offset".to_string())?
                .checked_mul(width)
                .ok_or_else(|| "the slot table's offset overflows".to_string())?;
            let is_last = i + 1 == calls;
            let tail_here = is_last && last == LastCall::Tail;

            // **A promoted literal emits nothing but an `iconst`.** No slot
            // load, no indirect call, no `Vm::apply` — the saving promotion
            // exists for. Its slot and thunk are still allocated and filled;
            // they are simply never read, which costs eight bytes and keeps
            // every other index in this loop meaning what it meant.
            //
            // A literal in tail position is not promoted: the body must end in
            // `return_call_indirect`, and an `iconst` is not a terminator.
            if let Some(Plan::Literal(n)) = plan.get(i)
                && !tail_here
            {
                let var = f.declare_var(types::I64);
                let v = f.ins().iconst(types::I64, *n);
                f.def_var(var, v);
                promoted.push(var);
                continue;
            }

            // **A tail-position value is never inlined.** `return_call_indirect`
            // is a terminator and an inlined region falls through to the next
            // value; the two cannot occupy the same position. Such a value takes
            // the generic path, and its guard thunk is emitted but never called.
            let inline_here = match plan.get(i) {
                Some(Plan::Inline {
                    site,
                    generation,
                    cell,
                }) if !tail_here => Some((*site, *generation, *cell)),
                _ => None,
            };

            let Some((site, generation, cell)) = inline_here else {
                // **D68: may promotion cross this call?** `Some(consumes)` when
                // all four gates held at plan time. A tail call never crosses:
                // it is a terminator, nothing follows it in this body, and the
                // held values would have no later sync to reach the stack by.
                let cross = match plan.get(i) {
                    Some(Plan::Call { cross }) if !tail_here => *cross,
                    _ => None,
                };

                match cross {
                    // **The sync, before the call that could observe it.** A
                    // slot call reaches `Vm::apply`, which can run arbitrary
                    // code: read the stack, switch it, report. Every promoted
                    // value goes back first, deepest last, so what the callee
                    // sees is what Tier 0 would have left it. This is the point
                    // §S5 calls an opaque site, and a `CONTEXT` literal reaches
                    // it too — which is what makes the barrier static.
                    None => {
                        syncs_here += emit_sync_n(&mut f, &mut promoted, usize::MAX, &helpers);
                        let callee = f.ins().load(ptr, MemFlagsData::trusted(), base, off);
                        if tail_here {
                            // §S8's claimed tail call. A terminator: nothing
                            // follows it, and the thunk's status becomes the
                            // body's.
                            f.ins().return_call_indirect(tail_sig, callee, &[ctx_val]);
                        } else {
                            emit_generic_call(
                                &mut f,
                                &GenericCall {
                                    tail_sig,
                                    callee,
                                    ctx_val,
                                    fail,
                                    ptr,
                                    base,
                                    drain_off,
                                    cells_base,
                                    request_off,
                                },
                                None,
                            );
                        }
                    }
                    // **D68's crossing.** The callee pops `consumes` operands
                    // from the real stack, so those are pushed; what lies below
                    // them stays in registers. `produces == 0` is what makes
                    // that sound — the callee leaves nothing above the held
                    // values, so they are the top of the abstract stack again
                    // when the body's last sync reaches them.
                    Some(consumes) => {
                        syncs_here +=
                            emit_sync_top_n(&mut f, &mut promoted, usize::from(consumes), &helpers);

                        // **The values that survive the call**, and the reason
                        // this needs a block of its own.
                        let held = promoted.clone();
                        let callee = f.ins().load(ptr, MemFlagsData::trusted(), base, off);

                        // **§S5's invariant dies here, and is replaced rather
                        // than dropped.** "A sync precedes every call" made the
                        // promoted model empty at every edge to `fail`, so the
                        // error paths were correct by construction. A crossed
                        // call reaches `fail` with values still in registers,
                        // and `fail` is `iconst FAIL; return_` with no block
                        // parameters — it cannot know what any caller held. So
                        // each crossed call gets its own spill: sync the held
                        // values, then fail. Without it a failing callee would
                        // report a stack missing everything still in a
                        // register, which is criterion 22's first bullet.
                        let spill = if held.is_empty() {
                            fail
                        } else {
                            f.create_block()
                        };

                        emit_generic_call(
                            &mut f,
                            &GenericCall {
                                tail_sig,
                                callee,
                                ctx_val,
                                fail: spill,
                                ptr,
                                base,
                                drain_off,
                                cells_base,
                                request_off,
                            },
                            None,
                        );

                        if !held.is_empty() {
                            // The call's success path, to return to once the
                            // spill block is filled.
                            let carry_on = f
                                .current_block()
                                .ok_or_else(|| "the crossed call left no block".to_string())?;

                            f.switch_to_block(spill);
                            // Deepest first, as everywhere: these are what the
                            // program still owes the stack. A push that itself
                            // fails goes straight to `fail`, which is what
                            // `helpers` carries.
                            let mut owed = held;
                            emit_sync_n(&mut f, &mut owed, usize::MAX, &helpers);
                            f.ins().jump(fail, &[]);

                            f.switch_to_block(carry_on);
                        }
                    }
                }
                continue;
            };

            let Some(fragment) = sites.get(site) else {
                return Err(format!(
                    "the lowering planned site {site}, outside the {} fragments it was given",
                    sites.len()
                ));
            };

            // **Is this site's type guard already answered?** It is when every
            // operand the fragment needs is a promoted int literal and the ops
            // address registers rather than the stack. Then `jit_admits` is not
            // merely redundant, it is *wrong to call*: with the operands held
            // back in `Variable`s it would interrogate a stack that is missing
            // them and decline a site that must admit.
            let from_registers = takes_registers(fragment) && promoted.len() >= fragment.needs();

            if from_registers {
                // **Sync the excess first.** A promoted site consumes the top
                // `needs` values; anything promoted below them must already be
                // on the stack, or the result would be pushed *beneath* a value
                // still sitting in a register. Nothing guards stack order, so
                // this is arithmetic rather than a check.
                let excess = promoted.len() - fragment.needs();
                syncs_here += emit_sync_n(&mut f, &mut promoted, excess, &helpers);
            } else {
                // Not promotable here: the operands belong on the stack, which
                // is where the type guard will look for them.
                syncs_here += emit_sync_n(&mut f, &mut promoted, usize::MAX, &helpers);
            }

            // **§S6's inlined site: three guards, then the arm.** Each guard
            // that refuses branches to the generic path — the very call this
            // value would otherwise have made, emitted below — so a refusal
            // costs nothing but the speed it was trying to buy. There is no
            // bail and no OSR: control never leaves compiled code.
            //
            // **A promoted site asks two rather than three.** The type guard is
            // discharged at compile time, above; the *meaning* guards are not,
            // because a name can be rebound however its operands arrive.
            let generic = f.create_block();
            let join = f.create_block();
            if !from_registers {
                let admits_off = i32::try_from(admits_slot_base + site)
                    .map_err(|_| "the guard slot index does not fit an offset".to_string())?
                    .checked_mul(width)
                    .ok_or_else(|| "the guard slot's offset overflows".to_string())?;
                let guard_callee = f.ins().load(ptr, MemFlagsData::trusted(), base, admits_off);
                let verdict_call = f.ins().call_indirect(tail_sig, guard_callee, &[ctx_val]);
                let verdict = f.inst_results(verdict_call)[0];
                let meaning = f.create_block();
                // **On the verdict, not on a status.** `ADMIT` is `FAIL`'s
                // value: an is-error test here would invert the guard.
                f.ins().brif(verdict, meaning, &[], generic, &[]);
                f.switch_to_block(meaning);
            }

            // The block the guard is decided in — criterion 17's dominance is
            // asserted from here to the region's first block.
            let guard_block = f
                .current_block()
                .ok_or_else(|| "the guard was emitted outside a block".to_string())?;

            // **Guard two: the slot still holds the binding we inlined
            // against.** §S6, *Inlining freezes a name* — an inlined fragment
            // goes through no slot, so a redefinition would otherwise be
            // invisible to it.
            let cell_addr = f.ins().iconst(ptr, cell);
            let now = f.ins().uload32(MemFlagsData::trusted(), cell_addr, 0);
            // **`_u`, because the generation is unsigned.** `uload32` brings
            // the cell in zero-extended, and `Slot::bump` saturates at
            // `u32::MAX` rather than wrapping — so a generation above
            // `i32::MAX` is reachable, and the sign-extending form would
            // compare it against a negative immediate and never match. The
            // bare `icmp_imm` is deprecated at 0.135 for exactly this
            // ambiguity, as `iadd_imm` was.
            let unchanged = f
                .ins()
                .icmp_imm_u(IntCC::Equal, now, i64::from(generation));
            let mode = f.create_block();
            f.ins().brif(unchanged, mode, &[], generic, &[]);

            // **Guard three: `autoadd` is clear.** Under it a `CALL` is not a
            // call at all (§S4's step 3), so an inlined arm would be the wrong
            // thing entirely rather than merely stale.
            f.switch_to_block(mode);
            let aa = f
                .ins()
                .uload32(MemFlagsData::trusted(), cells_base, autoadd_off);
            let region = f.create_block();
            f.ins().brif(aa, generic, &[], region, &[]);

            // The arm itself. **No request check follows it**: a fragment calls
            // only the four stack helpers, none of which files a tail request,
            // so §S5's after-every-call load belongs to the generic path alone.
            f.switch_to_block(region);
            // **The region keeps its result in a register.** §S5's residual
            // never rejoins the fast path, so there is no join at which two
            // register models could meet — which is exactly what forced the
            // earlier stage to sync every site's result. Without that merge the
            // arm's trailing `PushInt` defines a `Variable` instead of calling
            // the push helper, and `1 2 + 3 +` chains: the sum stays promoted
            // and becomes the next site's operand.
            let snapshot = promoted.clone();
            emit_fragment_ops(&mut f, fragment, &helpers, &mut promoted, true)?;
            f.ins().jump(join, &[]);
            regions.push((guard_block, region));

            // **§S5's residual path.** A refusing guard does not take the
            // generic call and rejoin: it syncs every promoted value and then
            // applies the rest of the body through `Vm::apply`, "exactly as
            // Tier 0 would", and returns that status. Control never leaves
            // compiled code — `jit_residual` is a runtime helper like any
            // other — so this is still guard-and-branch and needs no OSR.
            //
            // **The resume index is `i`, not `i + 1`.** The guard refused
            // *before* this value ran, so the value itself is still owed. The
            // residual re-applies it through the runtime, which is precisely
            // the generic path this block used to emit inline.
            f.switch_to_block(generic);
            let mut in_residual = snapshot;
            emit_sync_n(&mut f, &mut in_residual, usize::MAX, &helpers);
            let resume = f.ins().iconst(ptr, i as i64);
            let residual_call = f.ins().call(residual, &[ctx_val, resume]);
            let residual_status = f.inst_results(residual_call)[0];
            f.ins().return_(&[residual_status]);
            resumes.push((site, i));

            f.switch_to_block(join);
        }
        if last == LastCall::Ordinary {
            // **The last sync.** A body may end in a literal — `{ 1 2 }` is
            // two promotions and no call — and those values have to be on the
            // stack before the body returns, or the word would leave nothing
            // where Tier 0 leaves two.
            // **Not counted.** `syncs` is a count of pushes emitted *ahead of a
            // call*, which is the only place D68 changes anything. Every body
            // that promotes ends with this one, crossed or not, so counting it
            // would add the same constant to both arms and hide the difference
            // the figure exists to show — which is exactly what the first
            // version did: `1 2 nl` read two pushes either way, because the
            // synced form pushed both before the call and the crossed form
            // pushed both here.
            emit_sync_n(&mut f, &mut promoted, usize::MAX, &helpers);
            let ok = f.ins().iconst(types::I32, i64::from(OK));
            f.ins().return_(&[ok]);
        }
        f.switch_to_block(fail);
        let bad = f.ins().iconst(types::I32, i64::from(FAIL));
        f.ins().return_(&[bad]);
        f.seal_all_blocks();
        f.finalize(frontend);
        side_table = regions;
        resume_table = resumes;
        syncs = syncs_here;
    }

    // **Criterion 17, as a refusal rather than an assertion.** "Every inlined
    // region in the emitted code is preceded, on every path into it, by a load
    // and compare" — a dominance property. Checking it here, before the
    // function is defined, makes a lowering that could skip a guard *fail to
    // compile* rather than ship and be caught by a test; and it cannot panic,
    // which a `debug_assert` in emitted-code territory would.
    if !side_table.is_empty() {
        let cfg = ControlFlowGraph::with_function(&cg.func);
        let mut domtree = DominatorTree::new();
        domtree.compute(&cg.func, &cfg);
        for (guard, region) in &side_table {
            if !domtree.block_dominates(*guard, *region) {
                return Err(format!(
                    "an inlined region at {region} is reachable by a path that does not pass \
                     its guard at {guard}: RFC-0005 criterion 17 requires the guard to \
                     dominate every path into the region"
                ));
            }
        }
    }

    module
        .define_function(body_id, &mut cg)
        .map_err(|e| format!("define bund2_body_{seq}: {e}"))?;
    module.clear_context(&mut cg);

    // --- the entry trampoline
    cg.func.signature = sig_entry;
    {
        let mut f = FunctionBuilder::new(&mut cg.func, &mut fb_ctx);
        let body = module.declare_func_in_func(body_id, f.func);
        let block = f.create_block();
        f.append_block_params_for_function_params(block);
        f.switch_to_block(block);
        let ctx_val = f.block_params(block)[0];
        let call = f.ins().call(body, &[ctx_val]);
        let status = f.inst_results(call)[0];
        f.ins().return_(&[status]);
        f.seal_all_blocks();
        f.finalize(frontend);
    }
    module
        .define_function(entry_id, &mut cg)
        .map_err(|e| format!("define bund2_entry: {e}"))?;
    module.clear_context(&mut cg);

    module
        .finalize_definitions()
        .map_err(|e| format!("finalize: {e}"))?;

    // Now the thunks have addresses: fill the slots the body already reads.
    // `thunk_ids` ends with the drain thunk, and `slots` has the matching extra
    // entry, so one loop fills both.
    for (i, &id) in thunk_ids.iter().enumerate() {
        let addr = module.get_finalized_function(id);
        if addr.is_null() {
            return Err(format!("thunk {i} has no address"));
        }
        if let Some(s) = slots.get_mut(i) {
            *s = addr;
        }
    }

    let addr = module.get_finalized_function(entry_id);
    if addr.is_null() {
        return Err("the finalized entry trampoline has no address".into());
    }
    // SAFETY: the trampoline was emitted with `sig_entry` — one pointer
    // parameter, one `i32` result, the platform's convention — which is `Entry`.
    let entry: Entry = unsafe { std::mem::transmute::<*const u8, Entry>(addr) };

    Ok((entry, slots, resume_table, syncs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_interp::{Interp, frag};
    use bund2_ir::{Guard, Op};

    /// **Criterion 16's third leg.** The two legs already in
    /// `crates/bund2-stdlib/src/fragments.rs` compare the *model* — the arm as
    /// `frag::run` executes it — against the word. This one compares the
    /// **lowered code** against that model, over the same boundaries, which the
    /// criterion requires "before any lowering ships" and could not be written
    /// until a lowering existed.
    ///
    /// The comparison is `fragments.rs`'s: `dt`, and a render with identity and
    /// stamp blanked. The render carries the value, `dt`, `q` and D41's stack
    /// tag in one string, so it covers everything the criterion names except
    /// identity — which is asserted separately for `dup`, because `norm` is
    /// exactly what hides it.
    fn norm(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut rest = s;
        loop {
            let Some(i) = rest.find("id: \"") else {
                out.push_str(rest);
                return out;
            };
            let after = i + 5;
            out.push_str(&rest[..after]);
            let Some(end) = rest[after..].find('"') else {
                out.push_str(&rest[after..]);
                return out;
            };
            out.push_str("<id>");
            rest = &rest[after + end..];
            // And the stamp, which moves per run.
            if let Some(j) = rest.find("stamp: ") {
                let head = j + 7;
                out.push_str(&rest[..head]);
                let tail = rest[head..]
                    .find(',')
                    .map_or(rest.len(), |k| head + k);
                out.push_str("<stamp>");
                rest = &rest[tail..];
            }
        }
    }

    fn interp_with(start: &[BundValue]) -> Interp {
        let mut i = Interp::new();
        for v in start {
            i.push(v.clone());
        }
        i
    }

    /// One run's outcome: what the call answered, and the stack it left.
    type Outcome = (Result<bool, Error>, Vec<BundValue>);

    /// Run the same fragment over the same starting stack both ways.
    fn both(f: &Fragment, start: &[BundValue]) -> (Outcome, Outcome) {
        let mut model = interp_with(start);
        let by_model = frag::run(&mut model, f);

        let compiled = compile(f).expect("the arm lowers");
        let mut lowered = interp_with(start);
        let by_lowered = compiled.run(&mut lowered);

        (
            (by_model, model.snapshot()),
            (by_lowered, lowered.snapshot()),
        )
    }

    fn assert_agree(f: &Fragment, start: &[BundValue], label: &str) {
        let ((rm, sm), (rl, sl)) = both(f, start);
        assert_eq!(
            rm.is_ok(),
            rl.is_ok(),
            "{label}: one path failed and the other did not"
        );
        assert_eq!(
            rm.unwrap_or(false),
            rl.unwrap_or(false),
            "{label}: the guard decided differently"
        );
        assert_eq!(sm.len(), sl.len(), "{label}: depth");
        for (m, l) in sm.iter().zip(sl.iter()) {
            assert_eq!(m.dt(), l.dt(), "{label}: dt");
            assert_eq!(
                norm(&m.render(false)),
                norm(&l.render(false)),
                "{label}: value, q or stack tag"
            );
        }
    }

    fn add_arm() -> Fragment {
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
        .expect("well-formed")
    }

    fn dup_arm() -> Fragment {
        Fragment::new(Guard::Depth(1), vec![Op::DupTop], 0).expect("well-formed")
    }

    fn drop_arm() -> Fragment {
        Fragment::new(Guard::Depth(1), vec![Op::DropTop], 0).expect("well-formed")
    }

    /// **The arm's boundaries, chosen by hand** — criterion 16 is explicit that
    /// they are not generated, and names `i64::MAX`, `i64::MIN` and their
    /// neighbours.
    const EDGES: [i64; 8] = [0, 1, -1, 2, i64::MAX, i64::MAX - 1, i64::MIN, i64::MIN + 1];

    #[test]
    fn the_lowered_int_add_agrees_with_frag_run_at_every_boundary() {
        let f = add_arm();
        for a in EDGES {
            for b in EDGES {
                // Both orders: `PopInt`'s numbering is the whole difficulty,
                // and addition commuting is what hides a reversed file here.
                // `SubInt` would not forgive it.
                assert_agree(
                    &f,
                    &[BundValue::int(a), BundValue::int(b)],
                    &format!("{a} + {b}"),
                );
            }
        }
    }

    /// **The wrap pin.** Criterion 16: "a separate assertion pins
    /// `i64::MAX 1 +` to `i64::MIN` — wrap-around is where a lowering using a
    /// trapping or checked add would part company with the word."
    #[test]
    fn the_lowered_add_wraps_at_i64_max_as_the_word_does() {
        let f = add_arm();
        let start = [BundValue::int(i64::MAX), BundValue::int(1)];
        assert_agree(&f, &start, "i64::MAX 1 +");

        let compiled = compile(&f).expect("lowers");
        let mut vm = interp_with(&start);
        assert_eq!(compiled.run(&mut vm), Ok(true), "the arm ran");
        assert_eq!(
            vm.snapshot().first().and_then(BundValue::as_int),
            Some(i64::MIN),
            "the lowered add must wrap, not trap"
        );
    }

    #[test]
    fn the_lowered_dup_and_drop_agree_with_frag_run() {
        let shapes = [
            BundValue::int(7),
            BundValue::float(2.5),
            BundValue::str("s"),
            BundValue::list(vec![BundValue::int(1), BundValue::int(2)]),
        ];
        for v in shapes {
            assert_agree(&dup_arm(), std::slice::from_ref(&v), "dup");
            assert_agree(&drop_arm(), std::slice::from_ref(&v), "drop");
        }
    }

    /// **F13, asserted directly**, because `norm` blanks the identities the
    /// render would otherwise show. `Op::DupTop` is `.dup()` and not a clone,
    /// so the two values left behind have *different* identities. A lowering
    /// that pushed a clone would pass the render comparison above and fail
    /// here — which is how the model's own version of this test found the bug.
    #[test]
    fn the_lowered_dup_mints_a_fresh_identity() {
        let compiled = compile(&dup_arm()).expect("lowers");
        for v in [
            BundValue::int(7),
            BundValue::list(vec![BundValue::int(1)]),
        ] {
            let mut vm = interp_with(std::slice::from_ref(&v));
            assert_eq!(compiled.run(&mut vm), Ok(true));
            let stack = vm.snapshot();
            assert_eq!(stack.len(), 2, "dup left {} values", stack.len());
            let (below, _) = stack[0].identity();
            let (top, _) = stack[1].identity();
            assert_ne!(
                below, top,
                "the lowered dup shared one identity — that is a clone, not F13's dup"
            );
        }
    }

    /// **§S8's boundary shape, pieces 1 and 4.** The arm carries `Tail` and the
    /// entry carries the platform's own convention — which is the whole reason
    /// the trampoline exists, since Rust can neither define nor call a `Tail`
    /// function.
    ///
    /// A signature is not observable in finished machine code, so the lowering
    /// records what it chose and this reads it back. It would fail for a
    /// lowering that emitted the arm under the default convention and quietly
    /// let Rust call it, which is what this step replaced.
    #[test]
    fn the_arm_is_tail_and_the_entry_is_the_platform_convention() {
        let compiled = compile(&add_arm()).expect("lowers");

        // The arm: `Tail`, which is the only convention the verifier's
        // `typecheck_tail_call` will accept for a `return_call_indirect` target.
        assert_eq!(
            compiled.arm_call_conv(),
            CallConv::Tail,
            "the arm a compiled body will tail-call must be Tail"
        );
        assert!(
            compiled.arm_call_conv().supports_tail_calls(),
            "and must therefore support tail calls"
        );

        // The entry: **not** `Tail`, which is the whole reason it exists. Rust
        // can neither define nor call a `Tail` function, so asserting the
        // trampoline is anything else is asserting the seam is real. Compared
        // against the convention the module itself chose rather than a triple
        // re-derived here, which would only restate `default_call_conv`.
        assert_ne!(
            compiled.entry_call_conv(),
            CallConv::Tail,
            "a Tail trampoline could not be called from Rust at all"
        );
        assert!(
            !compiled.entry_call_conv().supports_tail_calls(),
            "the entry is the platform's own convention, which is not a tail-call one"
        );
    }

    /// **The trampoline is what runs, and it still agrees with the model.**
    ///
    /// Pieces 1 and 4 changed how the code is entered — a `Tail` arm behind a
    /// C-convention trampoline, where before Rust called the arm directly. If
    /// that seam were wrong the arm would not run at all, or would run with the
    /// context in the wrong register. The third leg above is the real check;
    /// this one says so in one place.
    #[test]
    fn the_boundary_did_not_change_what_the_arm_computes() {
        assert_agree(
            &add_arm(),
            &[BundValue::int(2), BundValue::int(3)],
            "through the trampoline",
        );
    }

    // --- §S8's pieces 2 and 3: the adapter, the thunks, and a call site ----

    /// **Lower a native-calling body against `vm`'s cells** — §S6's addressing.
    ///
    /// The emitted body loads the request cell through an address embedded at
    /// compile time, so the `Interp` it will run against has to exist *first*.
    /// Every test below therefore builds its interpreter before it compiles,
    /// which is the order a tier uses too: `JitTier::enter` has the `Vm` in
    /// hand and takes the address from it.
    fn lowered(vm: &Interp, calls: usize, last: LastCall) -> CompiledBody {
        compile_body(calls, last, vm.cells().base()).expect("lowers")
    }

    fn pushes_seven(vm: &mut dyn Vm) -> Result<(), Error> {
        vm.push(BundValue::int(7));
        Ok(())
    }

    fn fails(_: &mut dyn Vm) -> Result<(), Error> {
        Err(Error("deliberate failure".to_string()))
    }

    /// The same body, and the same message, as Tier 0's own test uses.
    fn boom(_: &mut dyn Vm) -> Result<(), Error> {
        panic!("boom from a dependency");
    }

    /// **A compiled body reaches a native through its `Tail` thunk.**
    ///
    /// End to end: Rust calls the C-convention trampoline, which calls the
    /// `Tail` body, which loads the thunk's address from its slot and calls
    /// through it, and the thunk calls the Rust adapter, which runs the native.
    /// Every one of §S8's four pieces is on that path.
    #[test]
    fn a_compiled_body_calls_a_native_through_its_thunk() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Ordinary);
        body.run(&mut vm, &[("push7", pushes_seven)]).expect("it ran");
        assert_eq!(vm.depth(), 1, "the native ran exactly once");
        assert_eq!(vm.snapshot().first().and_then(BundValue::as_int), Some(7));
    }

    /// Several calls, in order, each through its own slot and thunk.
    #[test]
    fn a_compiled_body_calls_each_native_in_order() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 3, LastCall::Ordinary);
        assert_eq!(body.calls(), 3);
        body.run(
            &mut vm,
            &[("a", pushes_seven), ("b", pushes_seven), ("c", pushes_seven)],
        )
        .expect("it ran");
        assert_eq!(vm.depth(), 3, "one push per call");
    }

    /// **§S11: the error travels in the context, never as an unwind.** And the
    /// body stops at the first failure rather than running the rest.
    #[test]
    fn a_failing_native_stops_the_body_and_reaches_rust_through_the_context() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 2, LastCall::Ordinary);
        let e = body
            .run(&mut vm, &[("fails", fails), ("push7", pushes_seven)])
            .expect_err("the first native failed");
        assert_eq!(e.0, "deliberate failure");
        assert_eq!(vm.depth(), 0, "the second native must not have run");
    }

    /// **Criterion 29's compiled half, in a non-tail position.**
    ///
    /// "Register a native that panics. Call it from a compiled body in a
    /// non-tail and a tail position, and assert the same observable result
    /// Tier 0 gives: `Error::internal` naming the native […] with nothing on
    /// stderr and no abort."
    ///
    /// The text is compared against Tier 0's, not merely checked for being an
    /// internal error: D49 requires both tiers to word one failure the same
    /// way, and the adapter reaches `bund2_api::panicked` exactly as
    /// `Interp::invoke` does.
    #[test]
    fn a_panicking_native_in_a_non_tail_position_matches_tier_zero() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Ordinary);
        let e = body.run(&mut vm, &[("boom", boom)]).expect_err("the panic is an error");
        assert!(e.is_internal(), "{}", e.0);
        assert!(
            e.0.contains("internal error: native `boom` panicked: boom from a dependency"),
            "{}",
            e.0
        );
        // And the process is still here, which is the other half of the claim.
        let mut after = Interp::new();
        lowered(&after, 1, LastCall::Ordinary)
            .run(&mut after, &[("push7", pushes_seven)])
            .expect("compiled code still runs after a caught panic");
        assert_eq!(after.depth(), 1);
    }

    /// **Criterion 29's compiled half, in a tail position** — the same claim
    /// through `return_call_indirect`, where the thunk's status becomes the
    /// body's return value directly.
    #[test]
    fn a_panicking_native_in_a_tail_position_matches_tier_zero() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Tail);
        assert_eq!(body.last_call(), LastCall::Tail);
        let e = body.run(&mut vm, &[("boom", boom)]).expect_err("the panic is an error");
        assert!(
            e.0.contains("internal error: native `boom` panicked: boom from a dependency"),
            "{}",
            e.0
        );
    }

    /// **§S8's claimed tail call, exercised.** A single call in tail position is
    /// also the shape where the body's failure block has no predecessor at all,
    /// so this is what puts the verifier's opinion of an unreachable block on
    /// the record rather than leaving it to the builder's assertions.
    #[test]
    fn a_tail_call_body_runs_and_leaves_no_unreachable_block_behind() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Tail);
        body.run(&mut vm, &[("push7", pushes_seven)]).expect("it ran");
        assert_eq!(vm.depth(), 1);
    }

    /// A native index the body was not given is a broken invariant in the
    /// lowering, not a fact about the program — so it is an internal error.
    #[test]
    fn calling_past_the_natives_given_is_an_internal_error() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 2, LastCall::Ordinary);
        let e = body
            .run(&mut vm, &[("only_one", pushes_seven)])
            .expect_err("the second index is out of range");
        assert!(e.is_internal(), "{}", e.0);
        assert!(e.0.contains("outside the 1 it was given"), "{}", e.0);
    }

    #[test]
    fn a_body_that_calls_nothing_is_refused() {
        // The arity check fires before the address is used, so any live
        // interpreter's cells serve.
        let vm = Interp::new();
        assert!(compile_body(0, LastCall::Ordinary, vm.cells().base()).is_err());
    }

    /// **A body cannot be lowered without the address of its cells.** §S6 has
    /// the emitted code load the request cell through an embedded immediate; a
    /// zero base would make that a read of address zero, so the lowering
    /// refuses rather than emitting a fault into machine code.
    #[test]
    fn a_body_without_cells_is_refused() {
        assert!(compile_body(1, LastCall::Ordinary, 0).is_err());
    }

    /// **The cell is loaded by the emitted code, not asked of the `Vm`** — §S6,
    /// *Addressing*: "At compile time the JIT embeds each cell's address as an
    /// immediate."
    ///
    /// This is the test that separates the two designs. Every other drain test
    /// here would pass just as well against a lowering that called into Rust
    /// and asked `Vm::drain_tail_request` unconditionally, because they observe
    /// only the outcome. This one compiles against **one** interpreter's cells
    /// and then runs the body against a **different** interpreter: the address
    /// is baked in, so the body reads the first `Interp`'s request cell and
    /// finds it clear, and the second interpreter's filed body is left pending.
    ///
    /// That is not a behaviour to rely on — it is why criterion 23 forbids
    /// running a body compiled for one `Interp` under another, and why the
    /// compiler is per-`Interp`. It is asserted here because it is the only
    /// cheap evidence that the load is in the machine code at all.
    #[test]
    fn the_request_cell_is_read_from_the_address_compiled_in() {
        let owner = Interp::new();
        let body = lowered(&owner, 1, LastCall::Ordinary);

        // A different interpreter, with its own cells at a different address.
        let mut other = Interp::new();
        assert_ne!(
            owner.cells().base(),
            other.cells().base(),
            "two Interps must not share one allocation"
        );

        body.run(&mut other, &[("fs", files_then_succeeds)])
            .expect("the native succeeded");

        // The body consulted `owner`'s cell, which nothing set, so it did not
        // drain — and `other`'s request is still pending.
        assert!(
            other.cells().request(),
            "the filed request is untouched, because the body read another \
             Interp's cell: the address is compiled in"
        );
        assert_eq!(other.depth(), 0, "and the body has not run");
    }

    /// Files a body for the loop to run, then fails — F96's shape exactly, and
    /// the same native Tier 0's `a_failed_native_leaves_no_tail_request` uses.
    fn files_then_fails(vm: &mut dyn Vm) -> Result<(), Error> {
        vm.tail_lambda(BundValue::lambda(vec![BundValue::int(99)]));
        Err(Error("failed after filing".to_string()))
    }

    /// Files a body and succeeds. The **positive control** for the test below.
    fn files_then_succeeds(vm: &mut dyn Vm) -> Result<(), Error> {
        vm.tail_lambda(BundValue::lambda(vec![BundValue::int(99)]));
        Ok(())
    }

    /// **F96's parity, closed for compiled code.**
    ///
    /// Tier 0 clears a tail request when the native that filed one then fails —
    /// `Interp::invoke` does it, and that is the one place Tier 0 calls a
    /// native. The adapter calls a `NativeFn` directly and never reaches
    /// `invoke`, so without `Vm::clear_tail_request` (D56) the stale body would
    /// wait in `pending_tail` and the next `take_pending` would run it: a body
    /// nobody asked for, after the error had already been dealt with.
    ///
    /// The assertion is that the body did **not** run. Evaluating a `1` through
    /// the same `Interp` afterwards is what gives `take_pending` its chance;
    /// depth 1 rather than 2 is the proof, exactly as Tier 0's test reads it.
    #[test]
    fn a_failing_native_leaves_no_tail_request_behind_compiled_code() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Ordinary);
        let e = body
            .run(&mut vm, &[("ff", files_then_fails)])
            .expect_err("the native failed");
        assert_eq!(e.0, "failed after filing");

        // The loop's next chance to run a pending body.
        vm.eval(&[BundValue::int(1)]).expect("the interpreter still runs");
        assert_eq!(
            vm.depth(),
            1,
            "only the 1: the stale body must not have run (F96)"
        );
        assert_eq!(vm.pull().and_then(|v| v.as_int()), Some(1));
    }

    /// **The positive control, so the clearing above is specific rather than
    /// blanket.**
    ///
    /// A native that files a request and *succeeds* must have its body run —
    /// the clearing is for failure only. Without this, a helper that cleared
    /// unconditionally would pass the test above while silently discarding
    /// every tail request a compiled call ever filed, which is a worse defect
    /// than F96 and in the same place.
    ///
    /// **Amended when §S5's drain landed, because it had started passing for a
    /// different reason than it was written for.** As written, it ran a further
    /// `eval` and checked the `99` was on the stack — which was evidence that
    /// Tier 0's *next* `take_pending` found the request. A non-tail call now
    /// drains, so the `99` is pushed during `run`, and the later `eval` proved
    /// nothing. The assertion is now made where the work happens.
    #[test]
    fn a_succeeding_native_keeps_the_tail_request_it_filed() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Ordinary);
        body.run(&mut vm, &[("fs", files_then_succeeds)])
            .expect("the native succeeded");

        let stack = vm.snapshot();
        assert_eq!(stack.len(), 1, "the filed body ran at the drain: {stack:?}");
        assert_eq!(
            stack[0].as_int(),
            Some(99),
            "the body the native filed must run, not be discarded: {stack:?}"
        );
        assert!(!vm.cells().request(), "and nothing is left pending");
    }

    // --- §S6's join: a body that inlines a published arm --------------------

    /// An `Interp` with the vocabulary, and §S6's fragment table taken from it.
    ///
    /// The table is keyed by the registration ids `register_all` just minted,
    /// so it has to be taken from *this* registry — a table from another
    /// `Interp` would match nothing, which is D43's point.
    fn with_fragments() -> (Interp, Vec<(bund2_api::RegistrationId, Fragment)>) {
        let vm = with_stdlib();
        let table = bund2_stdlib::fragments::published(&vm.registry).expect("well-formed");
        (vm, table)
    }

    /// **The join, end to end: a real body inlines a real arm, and agrees with
    /// Tier 0.**
    ///
    /// `{ 1 2 + }` has one inlinable value. The two literals are not `CALL`s
    /// and so are not sites at all; `+` resolves directly to a native whose
    /// registration the table publishes, so it becomes an inlined region behind
    /// three guards. The count is asserted rather than inferred, because timing
    /// cannot tell an inlined arm from a quick call.
    ///
    /// The differential is the one that matters: whatever the lowering did, the
    /// stack it leaves must be the stack `Interp::eval` leaves.
    #[test]
    fn a_body_inlines_a_published_arm_and_agrees_with_tier_zero() {
        let (mut vm, table) = with_fragments();
        let mut c = Compiler::new(table).expect("a compiler");
        let body = add_body();
        let cells = vm.cells().base();
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");

        assert_eq!(
            c.inlined_sites(word),
            Some(1),
            "`+` publishes an arm and resolves directly, so it is a site; the \
             two literals are not `CALL`s and never are"
        );

        c.run(word, &mut vm, &body).expect("the compiled word ran");

        let mut tier0 = with_stdlib();
        tier0.eval(&body).expect("Tier 0 runs the same body");

        let (a, b) = (tier0.snapshot(), vm.snapshot());
        assert_eq!(a.len(), b.len(), "depth: {a:?} vs {b:?}");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.as_int(), y.as_int(), "value: {a:?} vs {b:?}");
            assert_eq!(x.dt(), y.dt(), "dt");
        }
        assert_eq!(b.first().and_then(BundValue::as_int), Some(3));
    }

    /// **An empty table inlines nothing**, and the same body still computes the
    /// same answer through the generic path. Without this, the test above could
    /// pass against a lowering that ignored the table and always inlined.
    #[test]
    fn without_a_table_the_same_body_takes_the_generic_path() {
        let mut vm = with_stdlib();
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let body = add_body();
        let cells = vm.cells().base();
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");

        assert_eq!(c.inlined_sites(word), Some(0), "nothing published, nothing inlined");
        c.run(word, &mut vm, &body).expect("it ran");
        assert_eq!(vm.snapshot().first().and_then(BundValue::as_int), Some(3));
    }

    /// **`dup` is an alias, so §S6's direct-resolution rule refuses it.**
    ///
    /// The arm is published for `dup_one`, the native; `dup` reaches it through
    /// a second slot whose rewrite would not touch the first's generation, so
    /// there would be two generations to guard rather than one. The site is
    /// called, not inlined — and that is the rule working, not a gap.
    #[test]
    fn an_aliased_name_is_called_rather_than_inlined() {
        let (mut vm, table) = with_fragments();
        let mut c = Compiler::new(table).expect("a compiler");
        let cells = vm.cells().base();

        let aliased = vec![BundValue::int(7), BundValue::call("dup")];
        let word = c
            .compile_word(&aliased, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");
        assert_eq!(
            c.inlined_sites(word),
            Some(0),
            "`dup` is an alias for `dup_one`, so it resolves through a second slot"
        );

        let direct = vec![BundValue::int(7), BundValue::call("dup_one")];
        let word = c
            .compile_word(&direct, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");
        assert_eq!(
            c.inlined_sites(word),
            Some(1),
            "the native under its own name is a direct resolution"
        );
    }

    // --- §S6's type guard: the verdict, and its polarity --------------------

    /// A context over `vm` with `sites` as its fragment table, for calling
    /// [`jit_admits`] directly. The guard is the one thing a site asks before
    /// anything is emitted, so it is tested on its own before any site exists.
    fn admits_for(vm: &mut dyn Vm, sites: &[Fragment], site: usize) -> i32 {
        let mut c = Ctx {
            vm,
            err: None,
            natives: &[],
            body: &[],
            sites,
        };
        jit_admits(&raw mut c, site)
    }

    /// **The verdict's polarity, asserted from the specification rather than
    /// from the code.**
    ///
    /// Every other helper answers a *status*: [`OK`] is `0` and means the call
    /// succeeded, [`FAIL`] is `1` and means an error is parked. [`jit_admits`]
    /// answers a *verdict*: [`ADMIT`] is `1`, [`DECLINE`] is `0`, and neither
    /// is a failure.
    ///
    /// **The two protocols collide on both values, in opposite directions**, so
    /// there is no value a confused branch could land on and still be right:
    /// `ADMIT == FAIL`, so a status-style error check reads an admission as a
    /// failure; `DECLINE == OK`, so `brif status, fail, next` copied from the
    /// call pattern runs the inlined ops exactly when the guard refused them.
    /// Both mistakes typecheck, and both leave every result-inspecting test
    /// green. This is the test that does not.
    #[test]
    fn the_admit_verdict_is_not_a_status() {
        assert_eq!(OK, 0, "a status: zero means success");
        assert_eq!(FAIL, 1, "a status: one means an error is parked");
        assert_eq!(ADMIT, 1, "a verdict: one means the guard admits");
        assert_eq!(DECLINE, 0, "a verdict: zero means take the generic path");

        assert_eq!(
            ADMIT, FAIL,
            "the collision is real: an emitted branch must test the verdict \
             for ADMIT, never reuse a status's is-error test"
        );
        assert_eq!(
            DECLINE, OK,
            "and the other way: a decline is not a success, so a status's \
             is-ok test would inline against a guard that refused"
        );
    }

    /// The guard admits when it holds, and declines when it does not — the
    /// behavioural half of the polarity, against `Int + Int`'s real fragment.
    #[test]
    fn the_guard_admits_only_what_the_fragment_promises() {
        let sites = [add_arm()];

        let mut vm = Interp::new();
        vm.push(BundValue::int(1));
        vm.push(BundValue::int(2));
        assert_eq!(
            admits_for(&mut vm, &sites, 0),
            ADMIT,
            "two unboxed ints are exactly what TopAreInt(2) promises"
        );

        let mut shallow = Interp::new();
        shallow.push(BundValue::int(1));
        assert_eq!(
            admits_for(&mut shallow, &sites, 0),
            DECLINE,
            "one value is too few"
        );

        let mut wrong = Interp::new();
        wrong.push(BundValue::int(1));
        wrong.push(BundValue::str("s"));
        assert_eq!(
            admits_for(&mut wrong, &sites, 0),
            DECLINE,
            "a string on top is not an unboxed int"
        );
    }

    /// **A declining guard leaves the stack as it found it.** `frag::run` peeks
    /// and never pulls until the guard holds, and the emitted site must too:
    /// the generic path it falls back to reads the same operands, and a guard
    /// that had eaten them would leave the word running against a short stack.
    #[test]
    fn asking_the_guard_does_not_disturb_the_stack() {
        let sites = [add_arm()];
        let mut vm = Interp::new();
        vm.push(BundValue::int(1));
        vm.push(BundValue::str("s"));

        let before = vm.snapshot();
        assert_eq!(admits_for(&mut vm, &sites, 0), DECLINE);
        let after = vm.snapshot();

        assert_eq!(before.len(), after.len(), "depth unchanged: {after:?}");
        for (a, b) in before.iter().zip(after.iter()) {
            assert_eq!(a.render(false), b.render(false), "value unchanged");
        }
    }

    /// A site index the lowering never created declines rather than failing.
    /// The generic path is correct for every site, so a lowering bug costs
    /// speed and not meaning — and nothing is parked in the context, because a
    /// decline is not an error.
    #[test]
    fn an_unknown_site_declines_rather_than_erroring() {
        let mut vm = Interp::new();
        vm.push(BundValue::int(1));
        vm.push(BundValue::int(2));
        assert_eq!(
            admits_for(&mut vm, &[], 0),
            DECLINE,
            "no table at all: decline"
        );
        assert_eq!(
            admits_for(&mut vm, &[add_arm()], 7),
            DECLINE,
            "past the end of the table: decline"
        );
    }

    // --- §S5's drain helper: a call may leave a body to run -----------------

    /// **Criterion 26's shape, at the adapter.** A non-tail call must run a
    /// filed body *before the next value*: Tier 0's `{ 10 } ! 20` leaves `10`
    /// beneath `20`, and a compiled body that went on would leave them the
    /// other way round.
    ///
    /// The body here files `99` and the second call pushes `7`, so the order on
    /// the stack is what says whether the drain happened at the right moment.
    #[test]
    fn a_non_tail_call_drains_before_the_next_value() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 2, LastCall::Ordinary);
        body.run(&mut vm, &[("fs", files_then_succeeds), ("p7", pushes_seven)])
            .expect("it ran");

        let stack = vm.snapshot();
        assert_eq!(stack.len(), 2, "{stack:?}");
        assert_eq!(
            stack[0].as_int(),
            Some(99),
            "the filed body ran before the next call: {stack:?}"
        );
        assert_eq!(stack[1].as_int(), Some(7), "{stack:?}");
        assert!(!vm.cells().request(), "and nothing is left pending");
    }

    /// **A body's last call hands the request back rather than draining it** —
    /// §S5. Draining in tail position would spend a Rust frame per level, which
    /// is what RFC-0003's frame loop exists to prevent.
    ///
    /// So after a tail call the request is still pending when compiled code
    /// returns, and whatever entered the body takes it. Here that is the test
    /// itself: the body leaves nothing, and the next evaluation runs it.
    #[test]
    fn a_tail_call_hands_the_request_back_instead_of_draining() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Tail);
        body.run(&mut vm, &[("fs", files_then_succeeds)])
            .expect("it ran");

        assert_eq!(vm.depth(), 0, "the tail call did not drain");
        assert!(
            vm.cells().request(),
            "the request is handed back, still pending in the mirror"
        );

        // The entry takes it at once, as Tier 0's loop does.
        vm.eval(&[BundValue::int(1)]).expect("runs");
        let stack = vm.snapshot();
        assert!(
            stack.iter().any(|v| v.as_int() == Some(99)),
            "the handed-back body ran: {stack:?}"
        );
    }

    /// **A request filed inside a drained body is drained before the drain
    /// returns** — the tenth review's S3, through criterion 26.
    ///
    /// The frame loop is flat, so the body the drain starts runs its own filed
    /// body in the same `run_to`. Nothing may be left pending when the adapter
    /// returns.
    #[test]
    fn a_request_filed_inside_a_drained_body_is_drained_too() {
        /// Reached by name from the drained body, and files a body of its own.
        fn inner(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.tail_lambda(BundValue::lambda(vec![BundValue::int(42)]));
            Ok(())
        }
        /// The native the compiled call reaches: it files a body that pushes
        /// `7` and then calls `inner`, so the drain starts a body that files.
        fn files_outer(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.tail_lambda(BundValue::lambda(vec![
                BundValue::int(7),
                BundValue::call("inner"),
            ]));
            Ok(())
        }

        let mut vm = Interp::new();
        vm.registry.register_native(
            "inner",
            inner,
            bund2_api::StackEffect::opaque(0),
            bund2_api::WordKind::Sync,
        );

        let body = lowered(&vm, 1, LastCall::Ordinary);
        body.run(&mut vm, &[("fo", files_outer)]).expect("it ran");

        let stack = vm.snapshot();
        assert!(
            stack.iter().any(|v| v.as_int() == Some(7)),
            "the drained body ran: {stack:?}"
        );
        assert!(
            stack.iter().any(|v| v.as_int() == Some(42)),
            "and the body it filed ran too, before the drain returned: {stack:?}"
        );
        assert!(!vm.cells().request(), "nothing left pending");
    }

    /// **A refused drain clears the request** — §S5's second settled edge. With
    /// the Tier 0 floor above the stack pointer, the drain cannot re-enter
    /// evaluation, so it answers the exhaustion error and leaves nothing
    /// pending: "no stale request outlives an error".
    #[test]
    fn a_drain_refused_below_the_floor_clears_the_request() {
        let here = bund2_interp::stack_marker();
        // A floor above us, as `bund2-interp`'s own floor test builds it. The
        // `Interp` must be built **while the raised region is in force**, since
        // that is where its floor comes from — so the compile follows it rather
        // than preceding it, which is the opposite order from every other test
        // here and the whole point of this one.
        bund2_interp::set_stack_region(
            here + 4 * bund2_interp::STACK_RESERVE,
            bund2_interp::STACK_RESERVE,
        );
        let mut vm = Interp::new();
        bund2_interp::set_stack_region(here, 8 * 1024 * 1024);

        let body = lowered(&vm, 1, LastCall::Ordinary);

        let e = body
            .run(&mut vm, &[("fs", files_then_succeeds)])
            .expect_err("the drain was refused");
        assert!(e.is_stack_exhausted(), "{}", e.0);
        assert!(
            !vm.cells().request(),
            "the refused drain cleared the request rather than leaving it stale"
        );
    }

    /// **Draining when nothing was filed does nothing**, and reports success.
    /// Without this the four tests above would pass against a drain that always
    /// ran something or always failed.
    #[test]
    fn a_call_that_files_nothing_drains_nothing() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Ordinary);
        body.run(&mut vm, &[("p7", pushes_seven)]).expect("it ran");
        assert_eq!(vm.depth(), 1, "only what the native pushed");
        assert!(!vm.cells().request());
    }

    // --- `status_of`: D52's exit, and §S5's one status-maker ----------------

    /// A native that records an exit and returns `Ok`, as `bund.exit` does
    /// (D52): "it records a code through `Vm::request_exit` and returns `Ok`;
    /// nothing ends at that moment".
    fn exits_then_succeeds(vm: &mut dyn Vm) -> Result<(), Error> {
        vm.request_exit(7);
        Ok(())
    }

    /// A native that records an exit and then fails on its own account.
    fn exits_then_fails(vm: &mut dyn Vm) -> Result<(), Error> {
        vm.request_exit(7);
        Err(Error("the native's own error".into()))
    }

    /// **D52 through the compiled boundary.** A native that asks to end the
    /// program returns `Ok`, and Tier 0 stops at its *next step* — which
    /// compiled code never takes. Without `status_of` the adapter would report
    /// success and the body would run on past `exit`.
    ///
    /// The error is the one Tier 0 would make, from the one constructor both
    /// tiers use, so criterion 30 can compare the texts rather than two
    /// spellings of one refusal.
    #[test]
    fn a_recorded_exit_becomes_the_error_status() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Ordinary);
        let e = body
            .run(&mut vm, &[("ex", exits_then_succeeds)])
            .expect_err("an exit stops the body");

        assert!(e.is_exited(), "{}", e.0);
        assert_eq!(e, Error::exited(7), "the text Tier 0 would make");
        assert_eq!(vm.exit_requested(), Some(7), "and the code is recorded");
    }

    /// **The substitution is for `Ok` alone** — the fourteenth review's B1.
    ///
    /// Tier 0 never replaces a native's error: it wraps it, and under `?try`
    /// that whole text becomes a value on the stack, which criterion 30
    /// compares as text. A `status_of` that substituted after `Err` too would
    /// keep only the short refusal and fail that comparison.
    #[test]
    fn an_error_is_not_replaced_by_the_refusal() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Ordinary);
        let e = body
            .run(&mut vm, &[("exf", exits_then_fails)])
            .expect_err("the native failed");

        assert_eq!(e.0, "the native's own error", "passed through unchanged");
        assert!(!e.is_exited(), "not replaced by the refusal: {}", e.0);
        assert_eq!(vm.exit_requested(), Some(7), "the exit is still recorded");
    }

    /// **Every error `status_of` answers clears the request** — §S5's third
    /// settled edge. A native that files a body and then ends the program
    /// leaves nothing for Tier 0's next `take_pending` to run.
    #[test]
    fn an_exit_clears_a_request_the_native_filed() {
        fn files_then_exits(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.tail_lambda(BundValue::lambda(vec![BundValue::int(99)]));
            vm.request_exit(7);
            Ok(())
        }
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Ordinary);
        let e = body
            .run(&mut vm, &[("fx", files_then_exits)])
            .expect_err("the exit stops it");
        assert!(e.is_exited(), "{}", e.0);
        assert!(
            !vm.cells().request(),
            "§S6's mirror is clear as well as Tier 0's pending_tail"
        );
    }

    /// **The mirror tracks the truth through the compiled boundary.** A
    /// failing native's request is cleared in both, which is F96's parity and
    /// §S6's pairing seen from the compiled side.
    #[test]
    fn a_failing_natives_request_clears_the_mirror_through_the_adapter() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Ordinary);
        body.run(&mut vm, &[("ff", files_then_fails)])
            .expect_err("the native failed");
        assert!(
            !vm.cells().request(),
            "the request cell is clear after the adapter answered an error"
        );
    }

    /// **The positive control for `status_of` itself.** A succeeding native
    /// with no exit recorded must report success, and the body it filed must
    /// run — otherwise the four tests above would pass against a helper that
    /// failed everything.
    ///
    /// **Amended when §S5's drain landed.** This asserted the request was
    /// *still pending* afterwards, which was right while the adapter only
    /// cleared on failure. A non-tail call now drains, so the body has already
    /// run by the time `run` returns and the cell is correctly clear. The
    /// control's point is unchanged — a succeeding native's body is not
    /// discarded — but the evidence moved from "still pending" to "ran".
    #[test]
    fn status_of_reports_success_when_nothing_ended_or_failed() {
        let mut vm = Interp::new();
        let body = lowered(&vm, 1, LastCall::Ordinary);
        body.run(&mut vm, &[("fs", files_then_succeeds)])
            .expect("no exit, no error");
        assert!(
            vm.snapshot().iter().any(|v| v.as_int() == Some(99)),
            "the filed body ran, at the drain: {:?}",
            vm.snapshot()
        );
        assert!(
            !vm.cells().request(),
            "and nothing is left pending once it has run"
        );
    }

    // --- the first lowering of a real Bund word -----------------------------

    /// An `Interp` with the real vocabulary, so `+` means `+`.
    fn with_stdlib() -> Interp {
        let mut i = Interp::new();
        bund2_stdlib::register_all(&mut i.registry);
        i
    }

    /// Compile a word against `vm`'s cells — the `compile_word` counterpart of
    /// [`lowered`], and the same ordering rule: the interpreter first.
    /// Compile a word of `values` generic values against `vm`'s cells.
    ///
    /// The body is `values` literals, which inline nothing: a literal is not a
    /// `CALL` and so never a site. Tests that want inlining build a real body
    /// and call [`Compiler::compile_word`] directly.
    ///
    /// **It takes the body rather than inventing one.** An earlier version
    /// compiled `0 1 2` and let callers run some *other* body of the same
    /// length through the result, which worked while every value was fetched
    /// from `Ctx::body` at run time. §S5's promotion bakes an int literal into
    /// an `iconst`, so a word is now tied to the body it was planned from —
    /// which is what the cache guarantees for every real caller (D35's
    /// payload-pointer key) and what [`Compiler::run`] now checks.
    fn word_in(c: &mut Compiler, vm: &mut Interp, body: &[BundValue], last: LastCall) -> WordHandle {
        let cells = vm.cells().base();
        c.compile_word(body, last, cells, vm).expect("lowers")
    }

    /// `values` distinct int literals — the generic body `word_in` used to
    /// invent, now named so a caller runs the same one it compiled.
    fn literal_body(values: usize) -> Vec<BundValue> {
        (0..values).map(|n| BundValue::int(n as i64)).collect()
    }

    /// `{ 1 2 + }` — a real word's body, as the parser would leave it.
    fn add_body() -> Vec<BundValue> {
        vec![BundValue::int(1), BundValue::int(2), BundValue::call("+")]
    }

    /// **The differential that makes the claim checkable.** The same body, run
    /// through Tier 0 and through the compiled word, must leave the same stack.
    fn assert_matches_tier0(body: &[BundValue], label: &str) {
        let mut tier0 = with_stdlib();
        let by_tier0 = tier0.eval(body);

        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let mut compiled = with_stdlib();
        let word = word_in(&mut c, &mut compiled, body, LastCall::Ordinary);
        let by_compiled = c.run(word, &mut compiled, body);

        assert_eq!(
            by_tier0.is_ok(),
            by_compiled.is_ok(),
            "{label}: one path failed and the other did not: {by_tier0:?} vs {by_compiled:?}"
        );
        let (a, b) = (tier0.snapshot(), compiled.snapshot());
        assert_eq!(a.len(), b.len(), "{label}: depth");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.dt(), y.dt(), "{label}: dt");
            assert_eq!(
                norm(&x.render(false)),
                norm(&y.render(false)),
                "{label}: value, q or stack tag"
            );
        }
    }

    /// **Criterion 30's compiled half: a body that ends the program** — D52,
    /// and §S5's *A call may end the program*.
    ///
    /// The criterion asks for compiled bodies calling `bund.exit` "directly,
    /// through the alias `exit`, and in tail position, each followed by a call
    /// and by an inlined `+`", with output, exit code and final stacks matching
    /// Tier 0's.
    ///
    /// **Why these compile the body outright instead of warming it through the
    /// tier.** A straight-line body containing `bund.exit` can never be warmed
    /// into compiled form: §S7's threshold compiles on an entry and runs that
    /// entry interpreted, and the exit ends the program before a second entry
    /// exists. So the threshold is not the instrument here, and these compile
    /// directly, as the status-protocol tests beside them do. The cases the
    /// twelfth review's B1 added — a *cold callee* under a compiled caller —
    /// are the ones the threshold can express, and they are separate from
    /// these.
    ///
    /// `inlining` decides whether §S6's published fragments reach the compiler,
    /// which is what makes the "inlined `+`" rows inline rather than call. It
    /// is asserted rather than assumed: with the table the `+` must be a site,
    /// and without it none.
    fn assert_exit_matches_tier0(
        body: &[BundValue],
        last: LastCall,
        inlining: bool,
        label: &str,
    ) {
        let mut tier0 = with_stdlib();
        let by_tier0 = tier0.eval(body);

        let (mut compiled, published) = with_fragments();
        let table = if inlining { published } else { Vec::new() };
        let mut c = Compiler::new(table).expect("a compiler");
        let word = word_in(&mut c, &mut compiled, body, last);
        assert_eq!(
            c.inlined_sites(word).unwrap_or(0) > 0,
            inlining,
            "{label}: the fixture must inline exactly when it was asked to, or \
             the row measures the other configuration"
        );
        let by_compiled = c.run(word, &mut compiled, body);

        // **The two tiers signal an exit differently, and the criterion is
        // worded around it.** Tier 0 records the request and stops, returning
        // `Ok`; a compiled body returns `Err(Error::exited(code))`, which is
        // §S5's status protocol and what `a_recorded_exit_becomes_the_error_
        // status` pins. Neither is observable to a program: the embedder reads
        // `exit_requested`, which is why this criterion asks for "output, exit
        // code and final stacks" and not for the same `Result`. Comparing the
        // `Result`s would fail every row here while nothing was wrong.
        assert!(
            by_tier0.is_ok(),
            "{label}: Tier 0 records an exit and returns Ok: {by_tier0:?}"
        );
        let b = by_compiled.expect_err("the compiled body reports the exit as a status");
        assert!(
            b.is_exited(),
            "{label}: the compiled error must be an exit, not another failure: {}",
            b.0
        );

        // The exit code is the half both tiers do share, and it is the one a
        // caller acts on.
        assert_eq!(
            tier0.exit_requested(),
            compiled.exit_requested(),
            "{label}: the recorded exit code"
        );
        assert_eq!(
            compiled.exit_requested(),
            Some(7),
            "{label}: and it is the code the body asked for"
        );

        let (x, y) = (tier0.snapshot(), compiled.snapshot());
        assert_eq!(x.len(), y.len(), "{label}: depth after the exit");
        for (m, n) in x.iter().zip(y.iter()) {
            assert_eq!(m.dt(), n.dt(), "{label}: dt");
            assert_eq!(
                norm(&m.render(false)),
                norm(&n.render(false)),
                "{label}: value, q or stack tag"
            );
        }
    }

    /// **Criterion 30, the direct and aliased rows.**
    ///
    /// `bund.exit` by its own name and through `exit`, each followed by a call
    /// and by an inlined `+`. What follows the exit must not run, in either
    /// tier — the stack comparison is what says so, since a `+` that ran would
    /// leave a sum and a `clear` that ran would leave nothing.
    ///
    /// It fails for "an adapter that returns success after a recorded exit",
    /// which is this criterion's own stated failure mode: the body would carry
    /// on into the value after the exit and the two tiers would part.
    #[test]
    fn a_compiled_exit_matches_tier_zero() {
        let code = || BundValue::int(7);
        for (name, call) in [("bund.exit", "bund.exit"), ("the alias `exit`", "exit")] {
            assert_exit_matches_tier0(
                &[code(), BundValue::call(call), BundValue::call("clear")],
                LastCall::Ordinary,
                false,
                &format!("{name}, followed by a call"),
            );
            assert_exit_matches_tier0(
                &[
                    code(),
                    BundValue::call(call),
                    BundValue::int(1),
                    BundValue::int(2),
                    BundValue::call("+"),
                ],
                LastCall::Ordinary,
                true,
                &format!("{name}, followed by an inlined `+`"),
            );
        }
    }

    /// **Criterion 30, the tail-position rows.**
    ///
    /// **A reading, recorded rather than silently chosen.** The criterion says
    /// the exit is "in tail position, each followed by a call and by an inlined
    /// `+`" — but a value in tail position is the body's last, so nothing can
    /// follow it. Taken as: the exit is last under `LastCall::Tail`, and the
    /// call and the inlined `+` *precede* it, which is the only arrangement
    /// that keeps both halves of the sentence.
    ///
    /// Tail position is what §S8 lowers as `return_call_indirect`, so this is
    /// the row where an exit is recorded by a callee whose frame has replaced
    /// the body's own.
    #[test]
    fn a_compiled_exit_in_tail_position_matches_tier_zero() {
        assert_exit_matches_tier0(
            &[
                BundValue::int(1),
                BundValue::call("clear"),
                BundValue::int(7),
                BundValue::call("bund.exit"),
            ],
            LastCall::Tail,
            false,
            "a call, then `bund.exit` in tail position",
        );
        assert_exit_matches_tier0(
            &[
                BundValue::int(1),
                BundValue::int(2),
                BundValue::call("+"),
                BundValue::call("clear"),
                BundValue::int(7),
                BundValue::call("exit"),
            ],
            LastCall::Tail,
            true,
            "an inlined `+`, then the alias `exit` in tail position",
        );
    }

    /// **Criterion 30, the twelfth review's third B1 case: a body on the
    /// residual path whose last value is `exit`.**
    ///
    /// §S5's residual runs when an inlined site's guard declines: it syncs every
    /// promoted value and then applies the rest of the body one value at a time
    /// through `Vm::apply`, "exactly as Tier 0 would", and never rejoins. So an
    /// `exit` after a declined site is recorded by the *residual*, not by
    /// compiled code, and the status has to come back out through it.
    ///
    /// **How the guard is made to decline without failing.** `+` publishes an
    /// arm whose guard admits two `Int`s. Handed an `Int` and a string it
    /// declines — and the generic `+` beneath it *succeeds*, because F64's
    /// pass-through family joins them. That is what makes this row possible: a
    /// site that declines and a call that does not fail, so the body reaches
    /// the `exit` after it. A guard that declined into a failure would test
    /// criterion 19's path instead.
    ///
    /// The precondition is that the site exists at all — `inlined_sites > 0` —
    /// which `assert_exit_matches_tier0` asserts for every inlining row. The
    /// guard declining at run time is what routes it to the residual.
    #[test]
    fn an_exit_on_the_residual_path_matches_tier_zero() {
        assert_exit_matches_tier0(
            &[
                BundValue::int(1),
                BundValue::str("s"),
                BundValue::call("+"),
                BundValue::int(7),
                BundValue::call("exit"),
            ],
            LastCall::Ordinary,
            true,
            "an exit after a declined inline site, on the residual path",
        );
    }

    /// A compiled body that is a real word: literals pushed, a native called.
    #[test]
    fn a_compiled_word_runs_a_real_body() {
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let mut vm = with_stdlib();
        let word = word_in(&mut c, &mut vm, &add_body(), LastCall::Ordinary);
        assert_eq!(c.values(word), Some(3));
        c.run(word, &mut vm, &add_body()).expect("it ran");
        assert_eq!(vm.depth(), 1, "1 and 2 consumed, the sum left");
        assert_eq!(vm.snapshot().first().and_then(BundValue::as_int), Some(3));
    }

    /// The differential over several shapes a body can take.
    #[test]
    fn a_compiled_word_matches_tier_zero() {
        assert_matches_tier0(&add_body(), "1 2 +");
        assert_matches_tier0(&[BundValue::int(7)], "a lone literal");
        assert_matches_tier0(
            &[BundValue::int(4), BundValue::int(4), BundValue::call("+"),
              BundValue::int(2), BundValue::call("*")],
            "4 4 + 2 *",
        );
        assert_matches_tier0(
            &[BundValue::str("s"), BundValue::call("dup")],
            "a string and dup",
        );
    }

    /// **§S4's step 3, honoured because `apply` honours it.** Under `autoadd` a
    /// CALL is *not* a call: it is appended into the value beneath. A compiled
    /// body that lowered calls directly would need §S6's `autoadd` cell and a
    /// residual path to get this right; going through `Vm::apply` gets it for
    /// free, and this test is what says so rather than the comment above it.
    #[test]
    fn a_compiled_word_honours_autoadd_because_apply_does() {
        let body = add_body();

        let mut tier0 = with_stdlib();
        tier0.set_autoadd(true);
        let by_tier0 = tier0.eval(&body);

        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let mut compiled = with_stdlib();
        let word = word_in(&mut c, &mut compiled, &body, LastCall::Ordinary);
        compiled.set_autoadd(true);
        let by_compiled = c.run(word, &mut compiled, &body);

        assert_eq!(by_tier0.is_ok(), by_compiled.is_ok(), "autoadd: outcome");
        let (a, b) = (tier0.snapshot(), compiled.snapshot());
        assert_eq!(a.len(), b.len(), "autoadd: depth — a CALL was appended, not run");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(norm(&x.render(false)), norm(&y.render(false)), "autoadd");
        }
    }

    /// A `CALL` to a lambda files a tail request, which `apply` drains — so
    /// Bund depth stays on the heap and the body runs before the next value.
    #[test]
    fn a_compiled_word_runs_a_lambda_call_in_order() {
        let mut vm = with_stdlib();
        vm.registry
            .register_lambda("ten", BundValue::lambda(vec![BundValue::int(10)]));
        let body = vec![BundValue::call("ten"), BundValue::int(20)];

        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let word = word_in(&mut c, &mut vm, &body, LastCall::Ordinary);
        c.run(word, &mut vm, &body).expect("it ran");

        let stack = vm.snapshot();
        assert_eq!(stack.len(), 2, "{stack:?}");
        assert_eq!(stack[0].as_int(), Some(10), "the body ran before the 20");
        assert_eq!(stack[1].as_int(), Some(20));
    }

    /// A failing value stops the body, and the error arrives through the
    /// context rather than as an unwind (§S11).
    #[test]
    fn a_compiled_word_stops_at_the_first_failure() {
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let mut vm = with_stdlib();
        // `+` on an empty stack fails; the `99` after it must not run.
        let body = vec![BundValue::call("+"), BundValue::int(99)];
        let word = word_in(&mut c, &mut vm, &body, LastCall::Ordinary);
        let e = c.run(word, &mut vm, &body).expect_err("the call failed");
        assert!(!e.0.is_empty());
        assert_eq!(vm.depth(), 0, "the value after the failure must not run");
    }

    /// **F130's thunk cache: a second body reuses the first's thunks, and both
    /// still run.**
    ///
    /// A thunk bakes an index resolved against the **running** context —
    /// `jit_call_native` takes `Ctx::natives[i]`, `jit_admits` takes
    /// `Ctx::sites[k]` — so one emitted while compiling body A is correct when
    /// body B's slot table points at it. That is the claim the cache rests on,
    /// and the thing that would be silently wrong if it were false: B would
    /// call A's natives, with no guard anywhere to catch it.
    ///
    /// So this asserts both halves. **Reuse**: compiling a second body of the
    /// same shape emits no new thunk. **Correctness**: both bodies still give
    /// the right answer afterwards, and they are deliberately *different*
    /// programs over different natives, so a body running the other's table
    /// would produce the wrong number rather than the right one by luck.
    #[test]
    fn a_second_body_reuses_the_first_bodys_thunks() {
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let mut vm = with_stdlib();

        // `1 2 +` -> 3, two literals and a call.
        let first = add_body();
        let w1 = word_in(&mut c, &mut vm, &first, LastCall::Ordinary);
        let after_first = c.thunk_count();
        assert!(
            after_first > 0,
            "the first body must emit thunks, or this tests nothing"
        );

        // A different program with the same number of calls: `10 4 -`, which
        // Tier 0 answers **-6** — this native subtracts the top from the one
        // beneath it, checked against `bund2 script` rather than assumed, after
        // the first draft of this test asserted 6 and was wrong about the
        // order. Same slot shape, so every thunk index it needs is cached.
        let second = vec![
            BundValue::int(10),
            BundValue::int(4),
            BundValue::call("-"),
        ];
        let w2 = word_in(&mut c, &mut vm, &second, LastCall::Ordinary);
        assert_eq!(
            c.thunk_count(),
            after_first,
            "the second body needed no index the first had not reached, so it \
             must have emitted no thunk of its own"
        );

        // Both still run, and each over its own natives: if the shared thunk
        // resolved against the body it was emitted for rather than the body
        // running, the second would answer 3 and not 6.
        c.run(w1, &mut vm, &first).expect("the first body ran");
        assert_eq!(
            vm.snapshot().first().and_then(BundValue::as_int),
            Some(3),
            "the first body"
        );
        vm.clear();
        c.run(w2, &mut vm, &second).expect("the second body ran");
        assert_eq!(
            vm.snapshot().first().and_then(BundValue::as_int),
            Some(-6),
            "the second body, through thunks the first emitted — and had it \
             reached the *first* body's natives instead, `+` would answer 14"
        );
    }

    /// The tail-position variant runs the same body the same way.
    #[test]
    fn a_compiled_word_in_tail_position_runs_the_same_body() {
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let mut vm = with_stdlib();
        let word = word_in(&mut c, &mut vm, &add_body(), LastCall::Tail);
        assert_eq!(c.last_call(word), Some(LastCall::Tail));
        c.run(word, &mut vm, &add_body()).expect("it ran");
        assert_eq!(vm.snapshot().first().and_then(BundValue::as_int), Some(3));
    }

    /// A body of a different length than the word was built for is a broken
    /// invariant in the caller, not a fact about the program.
    #[test]
    fn a_word_handed_the_wrong_body_length_is_an_internal_error() {
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let mut vm = with_stdlib();
        let word = word_in(&mut c, &mut vm, &literal_body(3), LastCall::Ordinary);
        let e = c
            .run(word, &mut vm, &[BundValue::int(1)])
            .expect_err("length mismatch");
        assert!(e.is_internal(), "{}", e.0);
    }

    #[test]
    fn a_word_with_an_empty_body_is_refused() {
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let mut vm = Interp::new();
        let cells = vm.cells().base();
        assert!(
            c.compile_word(&[], LastCall::Ordinary, cells, &mut vm)
                .is_err()
        );
    }

    /// **Two words in one module stay distinct.** `declare_function` *merges* a
    /// duplicate name into the existing `FuncId` rather than failing, so
    /// unsuffixed function names would let a second word silently redefine the
    /// first's body — and the symptom would be a wrong answer, not an error.
    ///
    /// It also pins the property the shared module rests on: finalising a later
    /// definition must leave code already published alone, which is why the
    /// first word is run again *after* the second is emitted.
    #[test]
    fn two_words_compiled_into_one_module_stay_distinct() {
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let mut host = with_stdlib();
        let sum = word_in(&mut c, &mut host, &add_body(), LastCall::Ordinary);
        let lone = word_in(&mut c, &mut host, &[BundValue::int(7)], LastCall::Ordinary);
        assert_eq!(c.compiled_words(), 2);
        assert_ne!(sum, lone, "distinct words get distinct handles");

        let mut vm = with_stdlib();
        c.run(sum, &mut vm, &add_body()).expect("the first ran");
        assert_eq!(vm.snapshot().first().and_then(BundValue::as_int), Some(3));

        let mut vm = with_stdlib();
        c.run(lone, &mut vm, &[BundValue::int(7)])
            .expect("the second ran");
        assert_eq!(vm.snapshot().first().and_then(BundValue::as_int), Some(7));

        let mut vm = with_stdlib();
        c.run(sum, &mut vm, &add_body())
            .expect("and the first still runs");
        assert_eq!(vm.snapshot().first().and_then(BundValue::as_int), Some(3));
    }

    /// **`norm` does what the comparison needs, and no more.**
    ///
    /// Every test above rests on this function, so it is asserted rather than
    /// assumed: a `norm` that collapsed distinct values into equal strings
    /// would make the whole third leg pass while checking nothing. Two
    /// independently built `int(1)`s carry different identities and stamps and
    /// must normalise **equal**; `int(1)` and `int(2)` must not.
    #[test]
    fn norm_hides_identity_and_stamp_and_nothing_else() {
        let a = BundValue::int(1).render(false);
        let b = BundValue::int(1).render(false);
        let c = BundValue::int(2).render(false);
        assert_eq!(
            norm(&a),
            norm(&b),
            "two renders of the same value must normalise equal — identity and stamp differ per value"
        );
        assert_ne!(
            norm(&a),
            norm(&c),
            "norm must not collapse different values, or every comparison above is vacuous"
        );
    }

    /// **Mutation check: the comparison can fail.**
    ///
    /// The repo's own habit (criterion 28 is "mutation-checked") is to show that
    /// a test would catch the defect it exists for. Here the model adds and the
    /// lowered arm subtracts — `10 3` gives 13 one way and 7 the other — so the
    /// renders must differ. If they did not, the five tests above would be
    /// agreeing about nothing.
    #[test]
    fn the_comparison_catches_an_arm_that_computes_the_wrong_thing() {
        let start = [BundValue::int(10), BundValue::int(3)];

        let mut model = interp_with(&start);
        let by_model = frag::run(&mut model, &add_arm());
        assert_eq!(by_model, Ok(true), "the model ran");

        let wrong = Fragment::new(
            Guard::TopAreInt(2),
            vec![
                Op::PopInt,
                Op::PopInt,
                Op::SubInt { dst: 0, a: 0, b: 1 },
                Op::PushInt(0),
            ],
            2,
        )
        .expect("well-formed");
        let compiled = compile(&wrong).expect("lowers");
        let mut lowered = interp_with(&start);
        assert_eq!(compiled.run(&mut lowered), Ok(true), "the wrong arm ran");

        let (sm, sl) = (model.snapshot(), lowered.snapshot());
        assert_eq!(sm.len(), 1);
        assert_eq!(sl.len(), 1);
        assert_eq!(sm[0].as_int(), Some(13), "the model added");
        assert_eq!(sl[0].as_int(), Some(7), "the wrong arm subtracted");
        assert_ne!(
            norm(&sm[0].render(false)),
            norm(&sl[0].render(false)),
            "the comparison the third leg uses must be able to fail"
        );
    }

    /// **The rule the lowering inherits.** A fragment is entered only when its
    /// guard admits, and the word runs otherwise. A declined guard must consume
    /// nothing: `Ok(false)`, and the stack exactly as the program built it.
    #[test]
    fn a_declined_guard_touches_neither_stack() {
        let f = add_arm();
        // A float on top: `TopAreInt(2)` does not admit it.
        let start = [BundValue::int(1), BundValue::float(2.5)];
        assert_agree(&f, &start, "declined");

        let compiled = compile(&f).expect("lowers");
        let mut vm = interp_with(&start);
        assert_eq!(compiled.run(&mut vm), Ok(false), "the guard declined");
        assert_eq!(vm.depth(), 2, "a declined guard consumed nothing");

        // And too shallow, which is the other way the guard declines.
        let mut shallow = interp_with(&[BundValue::int(1)]);
        assert_eq!(compiled.run(&mut shallow), Ok(false));
        assert_eq!(shallow.depth(), 1);
    }

    // --- §S5's promotion ----------------------------------------------------

    /// **Promotion fires at all.**
    ///
    /// Every other test here would pass unchanged if `Plan::Literal` were never
    /// planned, because the unpromoted path is the old one and is correct. So a
    /// green differential is not evidence that promotion ran. This asserts the
    /// count directly, which is the only thing that separates "the lowering
    /// does not pay" from "the lowering did not happen".
    #[test]
    fn a_compiled_word_promotes_its_int_literals() {
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let mut vm = with_stdlib();
        let word = word_in(&mut c, &mut vm, &add_body(), LastCall::Ordinary);
        assert_eq!(
            c.promoted_values(word),
            Some(2),
            "`1 2 +` promotes both literals; the `+` is a call"
        );
        assert_eq!(c.promoted_total(), 2);
        // F136's denominator: three values, two of them promoted, so one took
        // the generic path and cost more compiled than interpreted.
        assert_eq!(c.values_total(), 3, "`1`, `2` and the call");

        // A CONTEXT literal is §S5's static barrier and never a promotion, and
        // a string is not an int.
        let mixed = vec![
            BundValue::str("s"),
            BundValue::named_context("other"),
            BundValue::int(5),
        ];
        let w2 = word_in(&mut c, &mut vm, &mixed, LastCall::Ordinary);
        assert_eq!(
            c.promoted_values(w2),
            Some(1),
            "only the int; a CONTEXT switches stacks with no call and must sync before it"
        );
    }

    /// **The excess sync.** Three promoted values meet a site needing two. The
    /// deepest must be pushed *before* the arm runs, or the sum lands beneath
    /// a value still sitting in a register — a silent misordering that no guard
    /// would catch.
    #[test]
    fn a_promoted_value_below_the_operands_is_synced_first() {
        assert_matches_tier0(
            &[
                BundValue::int(1),
                BundValue::int(2),
                BundValue::int(3),
                BundValue::call("+"),
            ],
            "1 2 3 +",
        );
        assert_matches_tier0(
            &[
                BundValue::int(1),
                BundValue::int(2),
                BundValue::int(3),
                BundValue::int(4),
                BundValue::call("+"),
                BundValue::call("+"),
            ],
            "1 2 3 4 + +",
        );
    }

    /// **The final sync.** A body that ends in a literal has values in
    /// registers when it returns, and they have to reach the stack first.
    #[test]
    fn a_body_ending_in_a_literal_syncs_before_it_returns() {
        assert_matches_tier0(&[BundValue::int(1), BundValue::int(2)], "1 2");
        assert_matches_tier0(
            &[BundValue::int(1), BundValue::int(2), BundValue::call("+"), BundValue::int(9)],
            "1 2 + 9",
        );
    }

    /// **Chaining: a site's result stays in a register and feeds the next.**
    ///
    /// `1 2 + 3 +` is two sites. Under the earlier stage each one pushed its
    /// sum and the next popped it straight back; with §S5's residual there is
    /// no join to merge at, so the first `+` leaves its result in a `Variable`
    /// and the second reads it as an operand. The whole body should touch the
    /// stack once, at the final sync.
    ///
    /// The differential is what makes it checkable — a chained lowering and an
    /// unchained one must leave the same stack, so this asserts agreement with
    /// Tier 0 and the site count, not the instruction sequence.
    #[test]
    fn a_sites_result_feeds_the_next_site_from_a_register() {
        let body = vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("+"),
            BundValue::int(3),
            BundValue::call("+"),
        ];

        let (mut vm, table) = with_fragments();
        let mut c = Compiler::new(table).expect("a compiler");
        let cells = vm.cells().base();
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");

        assert_eq!(c.inlined_sites(word), Some(2), "both `+`s are sites");
        assert_eq!(
            c.promoted_values(word),
            Some(3),
            "three int literals; the two `+`s are calls, not literals"
        );

        c.run(word, &mut vm, &body).expect("the compiled word ran");

        let mut tier0 = with_stdlib();
        tier0.eval(&body).expect("Tier 0 runs the same body");

        let (a, b) = (tier0.snapshot(), vm.snapshot());
        assert_eq!(a.len(), b.len(), "depth: {a:?} vs {b:?}");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(
                norm(&x.render(false)),
                norm(&y.render(false)),
                "value or stack tag: {a:?} vs {b:?}"
            );
        }
        assert_eq!(b.first().and_then(BundValue::as_int), Some(6), "1 + 2 + 3");
    }

    /// **Assumption 38's side table, asserted directly** — criterion 21.
    ///
    /// The criterion is explicit that stacks alone would pass a lowering that
    /// resumed at the wrong index, so the map itself is the assertion. The
    /// resume index for a site is the site's **own** body position: a guard
    /// refuses before its value has run, so the value is still owed and the
    /// residual re-applies it.
    #[test]
    fn every_inlined_site_records_where_its_residual_resumes() {
        let body = vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("+"),
            BundValue::int(3),
            BundValue::call("+"),
        ];

        let (mut vm, table) = with_fragments();
        let mut c = Compiler::new(table).expect("a compiler");
        let cells = vm.cells().base();
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");

        let side = c.resume_table(word).expect("the word exists");
        assert_eq!(side.len(), 2, "one entry per inlined site: {side:?}");
        assert_eq!(
            c.resume_index(word, 0),
            Some(2),
            "site 0 is the `+` at body index 2"
        );
        assert_eq!(
            c.resume_index(word, 1),
            Some(4),
            "site 1 is the `+` at body index 4"
        );
        assert_eq!(
            c.resume_index(word, 2),
            None,
            "a site this word never inlined has no entry"
        );

        // Every recorded index is a real position in the source body —
        // assumption 38's "the index is always a real position".
        for &(site, at) in side {
            assert!(at < body.len(), "site {site} resumes past the body at {at}");
        }
    }

    /// **`autoadd` sends an inlined site to the residual.** Guard three: under
    /// `autoadd` a `CALL` is appended into the value beneath rather than run
    /// (§S4's step 3), so the arm would be the wrong thing entirely. The
    /// residual syncs and resumes, and the result must match Tier 0's.
    #[test]
    fn autoadd_sends_an_inlined_site_to_the_residual() {
        let body = add_body();

        let mut tier0 = with_stdlib();
        tier0.set_autoadd(true);
        let by_tier0 = tier0.eval(&body);

        let (mut compiled, table) = with_fragments();
        let mut c = Compiler::new(table).expect("a compiler");
        let cells = compiled.cells().base();
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut compiled)
            .expect("lowers");
        assert_eq!(c.inlined_sites(word), Some(1), "`+` is a site");

        // Set *after* compiling: the guard is emitted against the state at
        // compile time and reads the cell at run time.
        compiled.set_autoadd(true);
        let by_compiled = c.run(word, &mut compiled, &body);

        assert_eq!(by_tier0.is_ok(), by_compiled.is_ok(), "autoadd: outcome");
        let (a, b) = (tier0.snapshot(), compiled.snapshot());
        assert_eq!(a.len(), b.len(), "autoadd: depth {a:?} vs {b:?}");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(
                norm(&x.render(false)),
                norm(&y.render(false)),
                "autoadd: {a:?} vs {b:?}"
            );
        }
    }

    /// **The residual.** A rebound `+` fails the generation guard, so control
    /// reaches §S5's residual path with the operands still in registers — and
    /// the residual must sync them before it resumes.
    ///
    /// The rebinding is registered *after* the word is compiled, which is the
    /// only ordering that leaves the compiled generation stale (§S6, *Inlining
    /// freezes a name*).
    ///
    /// **It compiles with the fragment table.** An earlier version of this test
    /// used `Compiler::new(Vec::new())`, so `+` was never a site, no generation
    /// guard was emitted, and the residual it names was unreachable — the test
    /// asserted a path it could not take. `inlined_sites` is asserted here so
    /// that cannot recur silently.
    #[test]
    fn a_rebound_name_syncs_the_operands_it_promoted() {
        let body = add_body();

        let mut tier0 = with_stdlib();
        let (mut compiled, table) = with_fragments();

        let mut c = Compiler::new(table).expect("a compiler");
        let cells = compiled.cells().base();
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut compiled)
            .expect("lowers");
        assert_eq!(
            c.inlined_sites(word),
            Some(1),
            "`+` must be a site, or the generation guard this test needs is never emitted"
        );
        assert_eq!(c.promoted_values(word), Some(2), "both literals promoted");

        // `+` now drops instead of adding. Tier 0 sees the same rebinding, so
        // the two must still agree — which they can only do if the residual
        // put `1` and `2` back on the stack.
        let rebound = BundValue::lambda(vec![BundValue::call("drop")]);
        tier0.registry.register_lambda("+", rebound.clone());
        compiled.registry.register_lambda("+", rebound);

        let by_tier0 = tier0.eval(&body);
        let by_compiled = c.run(word, &mut compiled, &body);
        assert_eq!(
            by_tier0.is_ok(),
            by_compiled.is_ok(),
            "{by_tier0:?} vs {by_compiled:?}"
        );

        let (a, b) = (tier0.snapshot(), compiled.snapshot());
        assert_eq!(a.len(), b.len(), "depth after the residual");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(
                norm(&x.render(false)),
                norm(&y.render(false)),
                "value or stack tag after the residual"
            );
        }
    }

    /// **The differential for a body that actually crosses a call** — D68.
    ///
    /// [`assert_matches_tier0`] builds its compiler with
    /// `Compiler::new(Vec::new())`, which after D68's first step means an empty
    /// crossable table: nothing is classified, nothing is crossed, and a
    /// crossing test driven through it would pass whether or not the emitter
    /// ever held a value across a call. That is F127's shape, so this supplies
    /// the real table — and asserts a crossing happened before comparing
    /// anything.
    ///
    /// **No fragment table.** Inlining is a different mechanism with its own
    /// criteria; leaving it out keeps a failure here attributable to the
    /// crossing rather than to a site.
    fn assert_crossing_matches_tier0(body: &[BundValue], crossings: usize, label: &str) {
        let mut tier0 = with_stdlib();
        let by_tier0 = tier0.eval(body);

        let mut compiled = with_stdlib();
        let crossable = bund2_stdlib::promotable::crossable(&compiled.registry);
        let mut c = Compiler::with_crossable(Vec::new(), crossable).expect("a compiler");
        let cells = compiled.cells().base();
        let word = c
            .compile_word(body, LastCall::Ordinary, cells, &mut compiled)
            .expect("lowers");

        assert_eq!(
            c.crossable_calls(word),
            Some(crossings),
            "{label}: the body must cross what it claims, or this asserts nothing: {:?}",
            c.crossings(word)
        );

        let by_compiled = c.run(word, &mut compiled, body);
        assert_eq!(
            by_tier0.is_ok(),
            by_compiled.is_ok(),
            "{label}: one path failed and the other did not: {by_tier0:?} vs {by_compiled:?}"
        );
        let (a, b) = (tier0.snapshot(), compiled.snapshot());
        assert_eq!(a.len(), b.len(), "{label}: depth");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.dt(), y.dt(), "{label}: dt");
            assert_eq!(
                norm(&x.render(false)),
                norm(&y.render(false)),
                "{label}: value, q or stack tag"
            );
        }
    }

    /// **The emitter really holds a value across a crossed call.**
    ///
    /// Every other crossing test is a differential, and a differential passes
    /// just as well if the emitter quietly synced everything and crossed
    /// nothing — the stacks would still match, because syncing early is always
    /// *correct*, merely slower. `crossable_calls` proves the **plan** said
    /// crossable; it says nothing about the code.
    ///
    /// So this asserts the shape instead, which is the by-construction style
    /// criterion 17 uses for dominance: compile the same body twice, once with
    /// the crossable table and once without, and require the crossed form to
    /// emit **strictly fewer pushes before its call**. The push helper is
    /// `jit_push_int`, and a held value is precisely a push that did not
    /// happen there.
    ///
    /// **`1 2 nl` is the clearest case**: `nl` consumes nothing, so a crossing
    /// emits *no* push before the call and the synced form emits two.
    #[test]
    fn a_crossed_call_emits_fewer_pushes_than_a_synced_one() {
        let body = [
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("nl"),
        ];

        // The synced form: an empty crossable table, so D68 classifies nothing
        // and every promoted value is pushed before the call.
        let mut synced_vm = with_stdlib();
        let mut synced = Compiler::new(Vec::new()).expect("a compiler");
        let cells = synced_vm.cells().base();
        let synced_word = synced
            .compile_word(&body, LastCall::Ordinary, cells, &mut synced_vm)
            .expect("lowers");
        assert_eq!(
            synced.crossable_calls(synced_word),
            Some(0),
            "the control must cross nothing, or it is not a control"
        );

        // The crossed form: the real table, so `nl` is classified and held
        // values survive the call.
        let mut crossed_vm = with_stdlib();
        let table = bund2_stdlib::promotable::crossable(&crossed_vm.registry);
        let mut crossed = Compiler::with_crossable(Vec::new(), table).expect("a compiler");
        let cells = crossed_vm.cells().base();
        let crossed_word = crossed
            .compile_word(&body, LastCall::Ordinary, cells, &mut crossed_vm)
            .expect("lowers");
        assert_eq!(
            crossed.crossable_calls(crossed_word),
            Some(1),
            "`nl` must be classified crossable, or this compares two controls"
        );

        // **The witness.** `nl` consumes nothing, so the crossed form pushes
        // nothing ahead of the call while the synced form pushes both literals.
        // Both reach the stack by the body's final sync, which is why that one
        // is not counted: it runs either way and would mask the difference.
        let synced_pushes = synced.syncs(synced_word).expect("the word was issued");
        let crossed_pushes = crossed.syncs(crossed_word).expect("the word was issued");
        assert_eq!(
            synced_pushes, 2,
            "the synced form pushes both literals before the call"
        );
        assert_eq!(
            crossed_pushes, 0,
            "`nl` consumes nothing, so a crossing pushes nothing ahead of it"
        );

        // Both must still agree with Tier 0 — the shape assertion is in
        // addition to the differential, never instead of it.
        let mut tier0 = with_stdlib();
        tier0.eval(&body).expect("Tier 0 runs it");
        synced
            .run(synced_word, &mut synced_vm, &body)
            .expect("the synced form runs");
        crossed
            .run(crossed_word, &mut crossed_vm, &body)
            .expect("the crossed form runs");
        assert_eq!(
            tier0.snapshot().len(),
            crossed_vm.snapshot().len(),
            "the crossed form must leave Tier 0's stack"
        );
        assert_eq!(
            synced_vm.snapshot().len(),
            crossed_vm.snapshot().len(),
            "and both forms must agree with each other"
        );
    }

    /// **D68, the crossing itself: a callee that consumes nothing.**
    ///
    /// `nl` is `eff(0, 0)`, on `PROMOTABLE.txt`, `bund2-stdlib`'s, and
    /// unpublished by §S6 — so it is a generic call that all four gates admit,
    /// and it consumes nothing. Both literals stay in registers across it and
    /// reach the stack by the body's last sync.
    ///
    /// This is the strongest form of the crossing: `emit_sync_top_n` is asked
    /// for zero, so nothing at all is pushed before the call.
    #[test]
    fn a_crossed_call_that_consumes_nothing_keeps_every_value_held() {
        assert_crossing_matches_tier0(
            &[
                BundValue::int(1),
                BundValue::int(2),
                BundValue::call("nl"),
            ],
            1,
            "1 2 nl",
        );
    }

    /// **D68, with operands: the top is pushed, the rest stays held.**
    ///
    /// `println` is `eff(1, 0)`, so it pops one value from the real stack.
    /// `emit_sync_top_n` pushes the top promoted value — the `2` — and keeps
    /// the `1` in a register; the callee consumes the `2`, and the final sync
    /// puts the `1` back. Tier 0 leaves the same single value.
    ///
    /// **This is the ordering property `emit_sync_top_n` exists for.** Syncing
    /// the wrong end would push the `1`, leave the `2` held, and `println`
    /// would print the wrong value while the stacks still matched in depth.
    #[test]
    fn a_crossed_call_pushes_its_operands_and_holds_the_rest() {
        assert_crossing_matches_tier0(
            &[
                BundValue::int(1),
                BundValue::int(2),
                BundValue::call("println"),
            ],
            1,
            "1 2 println",
        );
    }

    /// **The spill: a crossed call that fails must still report what it held.**
    ///
    /// `1 2 println` crosses and holds the `1`. Following it with a call that
    /// fails — `+` on a stack holding one value — reaches `fail` with the `1`
    /// still in a register. Without the spill block the body would report a
    /// stack missing it, where Tier 0 reports it present.
    ///
    /// The bodies must agree on the failure *and* on the stack the program is
    /// left with, which is criterion 22's first bullet in miniature.
    #[test]
    fn a_crossed_call_that_fails_spills_what_it_held() {
        assert_crossing_matches_tier0(
            &[
                BundValue::int(1),
                BundValue::int(2),
                BundValue::call("println"),
                BundValue::call("+"),
            ],
            1,
            "1 2 println +",
        );
    }

    /// **Criterion 27 — promotion does not cross a native `bund2-stdlib` did
    /// not register (D47).**
    ///
    /// "Register a native declaring `eff(1, 1)` that replaces its operand with
    /// the current depth. Its pair is honest, and it still observes beyond its
    /// operand. Compile a body that promotes, calls it, and continues. The
    /// result must match Tier 0's, and the lowering's side table must record
    /// the call as **synced, not crossed**."
    ///
    /// # Why this could not be written until now
    ///
    /// The second half needs a synced-versus-crossed record, and there was
    /// none: `Word` carried no field distinguishing them, because D66/D67 sync
    /// before every call and nothing was ever crossed. D68 built the crossing
    /// and [`Word::crossings`] records the verdict per call, so the criterion's
    /// own words can finally be asserted rather than approximated.
    ///
    /// # What refuses this native, precisely
    ///
    /// `register_native` mints a `RegistrationId` for **any** caller — an
    /// embedder's native is registered exactly as `bund2-stdlib`'s is. What
    /// separates them is `PROMOTABLE.txt`: the audit lists only the natives
    /// criterion 28's palette brought to `Ok`, a test's native is not among
    /// them, and `crossable_callee`'s membership check refuses it. That is D47
    /// and D48 doing their work through one table, which is why the entry is
    /// keyed by registration and not by name.
    #[test]
    fn promotion_does_not_cross_a_native_the_stdlib_did_not_register() {
        /// `eff(1, 1)`: an honest pair, and an observer all the same.
        fn depth_of(vm: &mut dyn Vm) -> Result<(), Error> {
            let _operand = vm.pull();
            let seen = vm.depth() as i64;
            vm.push(BundValue::int(seen));
            Ok(())
        }

        let mut vm = with_stdlib();
        vm.registry.register_native(
            "embedders",
            depth_of,
            bund2_api::StackEffect::fixed(1, 1),
            bund2_api::WordKind::Sync,
        );

        // The real crossable table, so the refusal below is D47/D48's and not
        // an empty table refusing everything — which would make this pass for
        // the wrong reason, as F127's fixtures did.
        let crossable = bund2_stdlib::promotable::crossable(&vm.registry);
        assert!(
            !crossable.is_empty(),
            "an empty table would refuse every call and prove nothing"
        );

        // `1 2 nl` crosses — `nl` is certified — and `3 embedders` must not.
        // Both calls in one body, so the table distinguishes them rather than
        // refusing wholesale.
        let body = vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("nl"),
            BundValue::int(3),
            BundValue::call("embedders"),
        ];

        let mut c = Compiler::with_crossable(Vec::new(), crossable).expect("a compiler");
        let cells = vm.cells().base();
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");

        let crossings = c.crossings(word).expect("the word was issued");
        let verdict = |i: usize| crossings.iter().find(|(at, _)| *at == i).map(|(_, x)| *x);
        assert_eq!(
            verdict(2),
            Some(true),
            "`nl` is certified, so the table admits it: {crossings:?}"
        );
        assert_eq!(
            verdict(4),
            Some(false),
            "an embedder's native is recorded synced, not crossed: {crossings:?}"
        );

        // And the result matches Tier 0's, which is the criterion's first half.
        c.run(word, &mut vm, &body).expect("the compiled word ran");
        let got = vm.snapshot();

        let mut tier0 = with_stdlib();
        tier0.registry.register_native(
            "embedders",
            depth_of,
            bund2_api::StackEffect::fixed(1, 1),
            bund2_api::WordKind::Sync,
        );
        tier0.eval(&body).expect("Tier 0 runs the same body");
        let want = tier0.snapshot();

        assert_eq!(want.len(), got.len(), "depth: {want:?} against {got:?}");
        for (x, y) in want.iter().zip(got.iter()) {
            assert_eq!(x.as_int(), y.as_int(), "{want:?} against {got:?}");
        }
    }

    /// **D68's classification, gate by gate.**
    ///
    /// The four gates are answered at plan time and recorded per call. The
    /// emitter still syncs before every one of them — D68's second half is
    /// unbuilt — so these assert a *permission*, which is exactly what
    /// criterion 27 and criterion 22's last bullet ask the side table for.
    ///
    /// **A classifier that answered `false` everywhere would pass no test but
    /// this one**, so each gate is given a case that must be refused *and* the
    /// crossable case that must be admitted. Without the positive, the whole
    /// table could be dead and every assertion would still hold.
    #[test]
    fn d68_classifies_a_call_by_its_four_gates() {
        let (mut vm, table) = with_fragments();
        let crossable = bund2_stdlib::promotable::crossable(&vm.registry);
        assert!(
            !crossable.is_empty(),
            "an empty table would make every refusal below vacuous"
        );

        // A lambda, for D46. Registered after the table is taken, which is
        // also the case D46's second program describes.
        vm.registry
            .register_lambda("lam", BundValue::lambda(vec![BundValue::call("drop")]));

        let mut c = Compiler::with_crossable(table, crossable).expect("a compiler");
        let cells = vm.cells().base();

        // `nl` is on `PROMOTABLE.txt`, is `bund2-stdlib`'s, resolves to a
        // native, and declares `eff(0, 0)` — produces nothing. All four gates
        // hold, so it is the one call here promotion may cross.
        //
        // **`nl` and not `drop`, though `drop` passes every gate too.** §S6
        // publishes a fragment for `drop`, so it plans as an *inlined site* and
        // never becomes a generic call — it would carry no verdict to assert,
        // which is what the first version of this test got wrong. The three
        // published names are `+`, `dup_one` and `drop`; a positive case has to
        // come from outside them.
        //
        // `+` is on the list too and is a native, but declares `eff(2, 1)`: it
        // leaves a result, which would sit above the promoted values
        // permanently and make the final sync write them beneath it. Here it
        // inlines, so its refusal is asserted separately in
        // `d68_refuses_a_native_that_produces_a_value`, where no fragment table
        // is given and it takes the generic path.
        //
        // `lam` resolves to a lambda, so D46 refuses it whatever it infers.
        let body = vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("+"),
            BundValue::call("nl"),
            BundValue::call("lam"),
        ];
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");

        let crossings = c.crossings(word).expect("the word was issued");
        let verdict = |i: usize| crossings.iter().find(|(at, _)| *at == i).map(|(_, x)| *x);

        // Index 2 is `+`. It plans as an inlined site, not a generic call, so
        // it carries no crossing at all — an inlined site is not a call the
        // promoted set crosses.
        assert_eq!(verdict(2), None, "`+` inlines; it is not a generic call");
        assert_eq!(verdict(3), Some(true), "`nl`: all four gates hold");
        assert_eq!(verdict(4), Some(false), "`lam` is a lambda — D46 refuses");
        assert_eq!(
            c.crossable_calls(word),
            Some(1),
            "exactly one call is crossable: {crossings:?}"
        );
    }

    /// **D68 refuses a producing native even when the audit certified it.**
    ///
    /// `+` is on `PROMOTABLE.txt` — criterion 28's palette brought it to `Ok` —
    /// so D47 and D48 hold. D68 is the gate that stops it: `eff(2, 1)` leaves a
    /// result above the promoted values, and the final sync would then write
    /// them beneath it, which D68 calls "wrong silently, and no guard catches
    /// it".
    ///
    /// The body denies `+` its fragment so it plans as a generic call rather
    /// than an inlined site, which is the only way to see the verdict.
    #[test]
    fn d68_refuses_a_native_that_produces_a_value() {
        let vm_and = with_fragments();
        let mut vm = vm_and.0;
        let crossable = bund2_stdlib::promotable::crossable(&vm.registry);
        // No fragment table: `+` cannot inline, so it takes the generic path
        // and its classification becomes visible.
        let mut c = Compiler::with_crossable(Vec::new(), crossable).expect("a compiler");
        let cells = vm.cells().base();

        let body = vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("+"),
        ];
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");

        assert_eq!(
            c.crossable_calls(word),
            Some(0),
            "`+` produces a value, so D68 refuses it: {:?}",
            c.crossings(word)
        );
    }

    /// **With no table, nothing is crossable** — `Compiler::new`'s behaviour,
    /// and every caller's before D68.
    ///
    /// This is the fallback that keeps the change inert until the emitter
    /// learns to cross: a compiler built the old way classifies no call, so the
    /// side table records `false` for all of them.
    #[test]
    fn without_the_table_no_call_is_crossable() {
        let (mut vm, _) = with_fragments();
        let mut c = Compiler::new(Vec::new()).expect("a compiler");
        let cells = vm.cells().base();
        let body = vec![BundValue::int(1), BundValue::call("nl")];
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");

        assert_eq!(
            c.crossable_calls(word),
            Some(0),
            "`nl` passes D46 and D68 but is not in an empty D48 table"
        );
        assert_eq!(c.crossable_total(), 0, "and the total agrees");
    }

    /// **Criterion 14 — a word that reads beyond its arity is a promotion
    /// barrier.**
    ///
    /// The criterion: "compile a body that promotes, then calls a word which
    /// inspects the stack beyond its declared arity, and assert the observed
    /// depth and contents match Tier 0's."
    ///
    /// D55's audit *identifies* such natives — `Interp::note_observation`
    /// records a native that reads the whole stack, the whole workbench or a
    /// stack's depth by name, and criterion 28 keeps them off
    /// `PROMOTABLE.txt`. **Nothing asserted the barrier itself**, which is a
    /// different claim: that a compiled body has actually synced before one
    /// runs. This is that assertion.
    ///
    /// # Why the native must read *beneath* its operand
    ///
    /// A native declaring `eff(1, 1)` is entitled to its own operand. The
    /// interesting one looks *past* it: `beneath` pops its operand, then reports
    /// the depth still under it. A body that promoted `1` and `2` and failed to
    /// sync would leave those in registers, so the native would see a shallower
    /// stack than Tier 0 gave it — and the two arms would disagree on a value
    /// neither the type guard nor the effect declaration would catch.
    ///
    /// The differential is the assertion. `a_synced_value_keeps_its_stack_tag`
    /// above proves the sync *happens*; this proves it happens **before a
    /// native that would notice**, which is the barrier D55 exists to place.
    #[test]
    fn a_native_reading_beneath_its_operand_sees_tier_zeros_stack() {
        /// `eff(1, 1)`: consumes its operand, answers the depth beneath it.
        ///
        /// Honest about its pair and still an observer — exactly the shape
        /// criterion 14 names, and the one D55 flags for `PROMOTABLE.txt`.
        fn beneath(vm: &mut dyn Vm) -> Result<(), Error> {
            let _operand = vm.pull();
            let under = vm.depth() as i64;
            vm.push(BundValue::int(under));
            Ok(())
        }

        let (mut vm, table) = with_fragments();
        vm.registry.register_native(
            "beneath",
            beneath,
            bund2_api::StackEffect::fixed(1, 1),
            bund2_api::WordKind::Sync,
        );

        // `1 2 +` promotes both literals and inlines the site; `3` promotes
        // too. `beneath` then consumes the `3` and reports what is under it —
        // which is the sum, and only if the sum was synced first.
        let body = vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("+"),
            BundValue::int(3),
            BundValue::call("beneath"),
        ];

        let mut c = Compiler::new(table).expect("a compiler");
        let cells = vm.cells().base();
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");
        // Asserted, not assumed: a body that inlined and promoted nothing would
        // make this test a differential of Tier 0 against itself — F127's shape.
        assert_eq!(c.inlined_sites(word), Some(1), "`+` is a site");
        assert_eq!(
            c.promoted_values(word),
            Some(3),
            "`1`, `2` and `3` are literals"
        );

        c.run(word, &mut vm, &body).expect("the compiled word ran");
        let got = vm.snapshot();

        let mut tier0 = with_stdlib();
        tier0.registry.register_native(
            "beneath",
            beneath,
            bund2_api::StackEffect::fixed(1, 1),
            bund2_api::WordKind::Sync,
        );
        tier0.eval(&body).expect("Tier 0 runs the same body");
        let want = tier0.snapshot();

        assert_eq!(want.len(), got.len(), "depth: {want:?} against {got:?}");
        for (x, y) in want.iter().zip(got.iter()) {
            assert_eq!(x.as_int(), y.as_int(), "{want:?} against {got:?}");
        }

        // And the observation itself: the native must have seen the synced sum
        // beneath its operand, not an empty stack. Tier 0 answers 1, so the
        // compiled arm must too — that equality is the barrier.
        assert_eq!(
            got.last().and_then(|v| v.as_int()),
            Some(1),
            "`beneath` saw {got:?}; a missing sync would read 0"
        );
    }

    /// **Criterion 12 — a synced value keeps its stack tag.**
    ///
    /// §S5 writes promoted values back to the real stack, and **D41 put the
    /// stack tag inside the value for scalars**. A sync that pushed a bare
    /// `BundValue` would leave `StackSym::NONE`, and the value would render
    /// `tags: {}` where the oracle renders `tags: {"stack": "main"}`. 42 of
    /// 113 goldens carry exactly that text, so the failure would be loud —
    /// but only once a golden exercises a compiled body with an opaque site
    /// in it, and none does. That is why this is a test rather than a golden.
    ///
    /// **The tag is asserted directly, not through `render`.** `norm` blanks
    /// `id` and `stamp` and leaves the rest as one string, so a differential
    /// against Tier 0 would pass on two values that were *both* untagged.
    /// `BundValue::tags` answers the question the criterion actually asks.
    ///
    /// The body promotes two literals, inlines `+`, and then calls a native —
    /// which is what forces the sync, since a sync precedes every call. The
    /// value the native leaves and the values it saw must all carry the tag.
    #[test]
    fn a_synced_value_keeps_its_stack_tag() {
        /// Sees the stack the sync built, and leaves a value of its own.
        fn observes(vm: &mut dyn Vm) -> Result<(), Error> {
            let seen = vm.depth() as i64;
            vm.push(BundValue::int(seen));
            Ok(())
        }

        let (mut vm, table) = with_fragments();
        vm.registry
            .register_native("obs", observes, bund2_api::StackEffect::fixed(0, 1), bund2_api::WordKind::Sync);

        // `1 2 +` promotes both literals and inlines the site; `obs` is the
        // call the promoted result must be synced before.
        let body = vec![
            BundValue::int(1),
            BundValue::int(2),
            BundValue::call("+"),
            BundValue::call("obs"),
        ];

        let mut c = Compiler::new(table).expect("a compiler");
        let cells = vm.cells().base();
        let word = c
            .compile_word(&body, LastCall::Ordinary, cells, &mut vm)
            .expect("lowers");
        assert_eq!(c.inlined_sites(word), Some(1), "`+` is a site");
        assert_eq!(c.promoted_values(word), Some(2), "both literals promoted");

        c.run(word, &mut vm, &body).expect("the compiled word ran");

        let stack = vm.snapshot();
        assert_eq!(stack.len(), 2, "the sum and what `obs` left: {stack:?}");
        assert_eq!(stack[0].as_int(), Some(3), "1 + 2, synced before the call");

        // **The sum was synced by compiled code.** It must carry the current
        // stack's name, exactly as a value Tier 0 pushed would.
        assert_eq!(
            stack[0].tags().get("stack").map(|s| &**s),
            Some("main"),
            "a synced value rendered `tags: {{}}`, which is D41's tag lost: {:?}",
            stack[0]
        );

        // And the native saw a stack of depth one — the sync happened *before*
        // the call, not after it.
        assert_eq!(
            stack[1].as_int(),
            Some(1),
            "the sync must precede the call, or `obs` saw an empty stack"
        );

        // The differential, as the rest of this module does it: Tier 0's
        // answer, tags included.
        let mut tier0 = with_stdlib();
        tier0
            .registry
            .register_native("obs", observes, bund2_api::StackEffect::fixed(0, 1), bund2_api::WordKind::Sync);
        tier0.eval(&body).expect("Tier 0 runs the same body");
        let want = tier0.snapshot();
        assert_eq!(want.len(), stack.len(), "depth");
        for (x, y) in want.iter().zip(stack.iter()) {
            assert_eq!(
                norm(&x.render(false)),
                norm(&y.render(false)),
                "value or stack tag: {want:?} vs {stack:?}"
            );
        }
    }
}
