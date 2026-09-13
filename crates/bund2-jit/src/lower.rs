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
use bund2_ir::{Fragment, Op};
use bund2_value::BundValue;

// `MemFlagsData`, not `MemFlags`: `load` takes `Into<MemFlagsData>`, and
// `trusted()` is a constructor on `MemFlagsData` — the two are separate structs
// at 0.135 and only one of them has it.
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlagsData, Signature, types};
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
}

/// A helper's status: `0` success, `1` error parked in [`Ctx::err`].
const OK: i32 = 0;
const FAIL: i32 = 1;

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
    match r {
        Ok(()) => OK,
        Err(e) => {
            // **F96's parity, through D56.** Tier 0 clears a tail request when
            // the native that filed one then fails — `Interp::invoke` does it,
            // and that is the one place Tier 0 calls a native. This adapter
            // calls the `NativeFn` directly and never reaches `invoke`, which
            // is precisely why `Vm::clear_tail_request` is on the public trait:
            // "a caller that runs a native and then answers an error calls this
            // before returning, and Tier 0's next `take_pending` finds nothing
            // to run."
            //
            // Without it a native that filed a body and then failed would leave
            // it pending, and the next `take_pending` would run a body nobody
            // asked for — after `?try` had already dealt with the error. That is
            // F96 reintroduced by the tier rather than inherited.
            //
            // **§S5 assigns this to `status_of`, which does not exist yet.** The
            // clearing belongs there once the boundary has a status helper; it
            // is here in the meantime because the obligation is the adapter's
            // either way, and a gap left for a future helper is still a gap.
            c.vm.clear_tail_request();
            c.err = Some(e);
            FAIL
        }
    }
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
            // A fragment's arm calls no native: its ops are the whole of it.
            natives: &[],
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

        // The register file, as a compile-time renaming. `PopInt` rotates it
        // right and writes slot 0, exactly as `Op::PopInt` documents and
        // `frag::run` performs at run time — here it costs nothing.
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
                    let addr = f.ins().stack_addr(ptr, slot, 0);
                    let call = f.ins().call(pop, &[ctx_val, addr]);
                    let status = f.inst_results(call)[0];
                    let next = f.create_block();
                    f.ins().brif(status, fail, &[], next, &[]);
                    f.switch_to_block(next);
                    emitted_any_call = true;
                    // `stack_load(pointer_type, loaded_type, slot, offset)` —
                    // the generated builder takes both types, and the vendored
                    // `src/` does not carry these signatures at all: they live
                    // in `target/debug/build/cranelift-codegen-*/out/inst_builder.rs`.
                    let v = f.ins().stack_load(ptr, types::I64, slot, 0);
                    if file.is_empty() {
                        return Err(
                            "a fragment popped into a register file it declared as empty".into()
                        );
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
                    let v = f.use_var(src);
                    let call = f.ins().call(push, &[ctx_val, v]);
                    let status = f.inst_results(call)[0];
                    let next = f.create_block();
                    f.ins().brif(status, fail, &[], next, &[]);
                    f.switch_to_block(next);
                    emitted_any_call = true;
                }
                Op::DupTop | Op::DropTop => {
                    let callee = if matches!(op, Op::DupTop) { dup } else { drop_ };
                    let call = f.ins().call(callee, &[ctx_val]);
                    let status = f.inst_results(call)[0];
                    let next = f.create_block();
                    f.ins().brif(status, fail, &[], next, &[]);
                    f.switch_to_block(next);
                    emitted_any_call = true;
                }
                Op::AddInt { dst, a, b } | Op::SubInt { dst, a, b } => {
                    let (Some(&va), Some(&vb)) =
                        (file.get(usize::from(*a)), file.get(usize::from(*b)))
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
pub fn compile_body(calls: usize, last: LastCall) -> Result<CompiledBody, String> {
    if calls == 0 {
        return Err("a body that calls nothing has no call site to exercise".into());
    }

    let mut builder =
        JITBuilder::new(default_libcall_names()).map_err(|e| format!("JIT builder: {e}"))?;
    builder.symbol("jit_call_native", jit_call_native as *const u8);
    let mut module = JITModule::new(builder);

    let frontend = module.target_config();
    let ptr = frontend.pointer_type();
    let conv = module.isa().default_call_conv();

    // The slots, allocated before anything is compiled so the base address the
    // body embeds is final.
    let mut slots: Box<[*const u8]> = vec![std::ptr::null(); calls].into_boxed_slice();
    let slots_base = slots.as_ptr() as i64;

    // The adapter: `(ctx, native) -> status`, under the platform's convention,
    // because it is a Rust function.
    let mut sig_adapter = Signature::new(conv);
    sig_adapter.params.push(AbiParam::new(ptr));
    sig_adapter.params.push(AbiParam::new(ptr));
    sig_adapter.returns.push(AbiParam::new(types::I32));
    let adapter_id = module
        .declare_function("jit_call_native", Linkage::Import, &sig_adapter)
        .map_err(|e| format!("declare jit_call_native: {e}"))?;

    // Thunks and the body are `Tail`, so `return_call_indirect` is legal from
    // any tail position: "body to body, and body to a native's thunk".
    let mut sig_tail = Signature::new(CallConv::Tail);
    sig_tail.params.push(AbiParam::new(ptr));
    sig_tail.returns.push(AbiParam::new(types::I32));

    let mut thunk_ids = Vec::with_capacity(calls);
    for i in 0..calls {
        let id = module
            .declare_function(&format!("bund2_thunk_{i}"), Linkage::Export, &sig_tail)
            .map_err(|e| format!("declare thunk {i}: {e}"))?;
        thunk_ids.push(id);
    }
    let body_id = module
        .declare_function("bund2_body", Linkage::Export, &sig_tail)
        .map_err(|e| format!("declare bund2_body: {e}"))?;

    let mut sig_entry = Signature::new(conv);
    sig_entry.params.push(AbiParam::new(ptr));
    sig_entry.returns.push(AbiParam::new(types::I32));
    let entry_id = module
        .declare_function("bund2_entry", Linkage::Export, &sig_entry)
        .map_err(|e| format!("declare bund2_entry: {e}"))?;

    let mut cg = module.make_context();
    let mut fb_ctx = FunctionBuilderContext::new();

    // --- the thunks: one ordinary call to the adapter, with the index baked in
    for (i, &id) in thunk_ids.iter().enumerate() {
        cg.func.signature = sig_tail.clone();
        {
            let mut f = FunctionBuilder::new(&mut cg.func, &mut fb_ctx);
            let adapter = module.declare_func_in_func(adapter_id, f.func);
            let block = f.create_block();
            f.append_block_params_for_function_params(block);
            f.switch_to_block(block);
            let ctx_val = f.block_params(block)[0];
            let idx = f.ins().iconst(ptr, i as i64);
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

        let width = i32::try_from(ptr.bytes()).map_err(|_| "pointer too wide".to_string())?;
        for i in 0..calls {
            // The slot's offset rides in `load`'s own `Offset32`, rather than an
            // `iadd_imm` before it: one instruction fewer per call, and
            // `iadd_imm` is deprecated at 0.135 in favour of the explicitly
            // sign- or zero-extending forms.
            let off = i32::try_from(i)
                .map_err(|_| "call index does not fit an offset".to_string())?
                .checked_mul(width)
                .ok_or_else(|| "the slot table's offset overflows".to_string())?;
            let callee = f.ins().load(ptr, MemFlagsData::trusted(), base, off);
            let is_last = i + 1 == calls;
            if is_last && last == LastCall::Tail {
                // §S8's claimed tail call. A terminator: nothing follows it,
                // and the thunk's status becomes the body's.
                f.ins().return_call_indirect(tail_sig, callee, &[ctx_val]);
            } else {
                let call = f.ins().call_indirect(tail_sig, callee, &[ctx_val]);
                let status = f.inst_results(call)[0];
                let next = f.create_block();
                f.ins().brif(status, fail, &[], next, &[]);
                f.switch_to_block(next);
            }
        }
        if last == LastCall::Ordinary {
            let ok = f.ins().iconst(types::I32, i64::from(OK));
            f.ins().return_(&[ok]);
        }
        f.switch_to_block(fail);
        let bad = f.ins().iconst(types::I32, i64::from(FAIL));
        f.ins().return_(&[bad]);
        f.seal_all_blocks();
        f.finalize(frontend);
    }
    module
        .define_function(body_id, &mut cg)
        .map_err(|e| format!("define bund2_body: {e}"))?;
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

    Ok(CompiledBody {
        _module: module,
        entry,
        _slots: slots,
        calls,
        last,
    })
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
        let body = compile_body(1, LastCall::Ordinary).expect("the body lowers");
        let mut vm = Interp::new();
        body.run(&mut vm, &[("push7", pushes_seven)]).expect("it ran");
        assert_eq!(vm.depth(), 1, "the native ran exactly once");
        assert_eq!(vm.snapshot().first().and_then(BundValue::as_int), Some(7));
    }

    /// Several calls, in order, each through its own slot and thunk.
    #[test]
    fn a_compiled_body_calls_each_native_in_order() {
        let body = compile_body(3, LastCall::Ordinary).expect("lowers");
        assert_eq!(body.calls(), 3);
        let mut vm = Interp::new();
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
        let body = compile_body(2, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
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
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
        let e = body.run(&mut vm, &[("boom", boom)]).expect_err("the panic is an error");
        assert!(e.is_internal(), "{}", e.0);
        assert!(
            e.0.contains("internal error: native `boom` panicked: boom from a dependency"),
            "{}",
            e.0
        );
        // And the process is still here, which is the other half of the claim.
        let mut after = Interp::new();
        compile_body(1, LastCall::Ordinary)
            .expect("lowers")
            .run(&mut after, &[("push7", pushes_seven)])
            .expect("compiled code still runs after a caught panic");
        assert_eq!(after.depth(), 1);
    }

    /// **Criterion 29's compiled half, in a tail position** — the same claim
    /// through `return_call_indirect`, where the thunk's status becomes the
    /// body's return value directly.
    #[test]
    fn a_panicking_native_in_a_tail_position_matches_tier_zero() {
        let body = compile_body(1, LastCall::Tail).expect("lowers");
        assert_eq!(body.last_call(), LastCall::Tail);
        let mut vm = Interp::new();
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
        let body = compile_body(1, LastCall::Tail).expect("a tail-call body lowers");
        let mut vm = Interp::new();
        body.run(&mut vm, &[("push7", pushes_seven)]).expect("it ran");
        assert_eq!(vm.depth(), 1);
    }

    /// A native index the body was not given is a broken invariant in the
    /// lowering, not a fact about the program — so it is an internal error.
    #[test]
    fn calling_past_the_natives_given_is_an_internal_error() {
        let body = compile_body(2, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
        let e = body
            .run(&mut vm, &[("only_one", pushes_seven)])
            .expect_err("the second index is out of range");
        assert!(e.is_internal(), "{}", e.0);
        assert!(e.0.contains("outside the 1 it was given"), "{}", e.0);
    }

    #[test]
    fn a_body_that_calls_nothing_is_refused() {
        assert!(compile_body(0, LastCall::Ordinary).is_err());
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
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
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
    /// A native that files a request and *succeeds* must still leave the body
    /// to run — the adapter clears on failure only. Without this, an adapter
    /// that cleared unconditionally would pass the test above while silently
    /// discarding every tail request a compiled call ever filed, which is a
    /// worse defect than F96 and in the same place.
    #[test]
    fn a_succeeding_native_keeps_the_tail_request_it_filed() {
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
        body.run(&mut vm, &[("fs", files_then_succeeds)])
            .expect("the native succeeded");

        vm.eval(&[BundValue::int(1)]).expect("runs");
        let stack = vm.snapshot();
        assert_eq!(
            stack.len(),
            2,
            "the filed body ran as well as the 1: {stack:?}"
        );
        assert!(
            stack.iter().any(|v| v.as_int() == Some(99)),
            "the body the native filed must still run: {stack:?}"
        );
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
}
