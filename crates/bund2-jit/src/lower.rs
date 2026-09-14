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
    /// The values a compiled **body** applies, in order.
    ///
    /// A body is a `Vec<BundValue>` — literals, `CALL`s, `CONTEXT`s — and
    /// [`jit_apply`] hands each one to `Vm::apply`, which is Tier 0's own path.
    /// Empty for a fragment arm, which applies no values: its ops are the whole
    /// of it.
    body: &'a [BundValue],
}

/// A helper's status: `0` success, `1` error parked in [`Ctx::err`].
const OK: i32 = 0;
const FAIL: i32 = 1;

/// **Run a filed tail request, after a non-tail call** — §S5's drain helper,
/// on the compiled side.
///
/// `r` is what the call answered. A failure is returned as it is and nothing is
/// drained: Tier 0 discards a failing native's request rather than running it
/// (F96), and `status_of` does the clearing. After a success the request is
/// drained **here, before the next value**, because a compiled call gets
/// control back before the body has run — and because `request_tail` assigns,
/// so a later request would overwrite one left pending and it would never run
/// at all.
///
/// The drain's own failure becomes the call's result, so a body that failed or
/// ended the program reaches `status_of` rather than being reported as the
/// native's success.
fn drained(vm: &mut dyn Vm, r: Result<(), Error>) -> Result<(), Error> {
    r?;
    vm.drain_tail_request()
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
    // **Non-tail: drain before the status.** §S5 — a compiled call gets control
    // back before a filed body has run, so the body must run *here*, before the
    // next value. Sequenced into a local because the drain borrows the context's
    // `Vm` and `status_of` takes the context itself. Then `status_of`, which
    // carries F96's parity (D56) and D52's rule that a recorded exit becomes
    // the error status.
    let r = drained(&mut *c.vm, r);
    status_of(c, r)
}

/// A body's last call **hands the request back rather than draining it** — §S5.
///
/// Same native, same catch, same status protocol; only the drain is absent.
/// The compiled function returns with the request pending and whatever entered
/// the body takes it at once, so a self-recursive word goes back to the frame
/// loop instead of spending a Rust frame per level.
extern "C" fn jit_call_native_tail(c: *mut Ctx<'_>, native: usize) -> i32 {
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
    let outcome = bund2_api::catch_panic(|| f(&mut *c.vm));
    let r = match outcome {
        Ok(r) => r,
        Err(msg) => Err(bund2_api::panicked(&format!("native `{name}`"), &msg)),
    };
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
    // **The drain is a no-op on this path today** and is called anyway: `apply`
    // drains its own request before it returns, so nothing is normally left.
    // Calling it keeps the rule at the call site rather than resting on what
    // `apply` happens to do, which is what §S5 asks of a non-tail call.
    let r = drained(&mut *c.vm, r);
    status_of(c, r)
}

/// [`jit_apply`] in tail position: no drain, for §S5's reason.
extern "C" fn jit_apply_tail(c: *mut Ctx<'_>, index: usize) -> i32 {
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
    status_of(c, r)
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
            body: &[],
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

    /// The **tail-position** adapter, which does not drain.
    ///
    /// §S5: "A body's last call does not drain. The compiled function returns
    /// with the request pending, and each entry takes it at once." Draining
    /// here instead would spend a Rust frame per level, which is what
    /// RFC-0003's frame loop exists to avoid — so the position has to reach the
    /// adapter, and it does by the last thunk importing this symbol rather than
    /// the one above.
    fn tail_symbol(self) -> (&'static str, *const u8) {
        match self {
            Adapter::Native => ("jit_call_native_tail", jit_call_native_tail as *const u8),
            Adapter::Apply => ("jit_apply_tail", jit_apply_tail as *const u8),
        }
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
    module: JITModule,
    words: Vec<Word>,
}

impl Compiler {
    /// A compiler with an empty module, ready to take bodies.
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            module: new_module(Adapter::Apply)?,
            words: Vec::new(),
        })
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
    pub fn compile_word(&mut self, values: usize, last: LastCall) -> Result<WordHandle, String> {
        if values == 0 {
            return Err("a word with an empty body has nothing to lower".into());
        }
        let seq = self.words.len();
        let (entry, slots) = emit_into(&mut self.module, seq, values, last, Adapter::Apply)?;
        self.words.push(Word {
            entry,
            _slots: slots,
            values,
            last,
        });
        Ok(WordHandle(seq))
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
        let mut c = Ctx {
            vm,
            err: None,
            natives: &[],
            body,
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
    emit_body(calls, last, Adapter::Native)
}

/// The shared emitter behind [`compile_body`] and [`Compiler::compile_word`].
///
/// Everything below the adapter is the same for both — §S8's `Tail` thunks, the
/// slot table §S6 addresses, the entry trampoline, the status protocol — so the
/// two lowerings share one emitter and cannot drift apart in the parts criteria
/// 4 and 29 are about.
fn emit_body(calls: usize, last: LastCall, adapter: Adapter) -> Result<CompiledBody, String> {
    let mut module = new_module(adapter)?;
    let (entry, slots) = emit_into(&mut module, 0, calls, last, adapter)?;
    Ok(CompiledBody {
        _module: module,
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
    let (tail_name, tail_ptr) = adapter.tail_symbol();
    let mut builder =
        JITBuilder::new(default_libcall_names()).map_err(|e| format!("JIT builder: {e}"))?;
    builder.symbol(name, ptr);
    // Both, because a body's thunks reach two adapters: the last one under
    // `LastCall::Tail` hands the request back rather than draining it (§S5).
    builder.symbol(tail_name, tail_ptr);
    Ok(JITModule::new(builder))
}

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
    module: &mut JITModule,
    seq: usize,
    calls: usize,
    last: LastCall,
    adapter: Adapter,
) -> Result<(Entry, Box<[*const u8]>), String> {
    let adapter_name = adapter.symbol().0;
    let tail_adapter_name = adapter.tail_symbol().0;
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
        .declare_function(adapter_name, Linkage::Import, &sig_adapter)
        .map_err(|e| format!("declare {adapter_name}: {e}"))?;
    // **The tail adapter**, for a last call under `LastCall::Tail`: same
    // signature, and it hands a filed request back instead of draining it
    // (§S5). Declared always; imported only by the thunk that needs it, and an
    // import nothing calls costs nothing.
    let tail_adapter_id = module
        .declare_function(tail_adapter_name, Linkage::Import, &sig_adapter)
        .map_err(|e| format!("declare {tail_adapter_name}: {e}"))?;

    // Thunks and the body are `Tail`, so `return_call_indirect` is legal from
    // any tail position: "body to body, and body to a native's thunk".
    let mut sig_tail = Signature::new(CallConv::Tail);
    sig_tail.params.push(AbiParam::new(ptr));
    sig_tail.returns.push(AbiParam::new(types::I32));

    let mut thunk_ids = Vec::with_capacity(calls);
    for i in 0..calls {
        let id = module
            .declare_function(&format!("bund2_thunk_{seq}_{i}"), Linkage::Export, &sig_tail)
            .map_err(|e| format!("declare thunk {i}: {e}"))?;
        thunk_ids.push(id);
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

    // --- the thunks: one ordinary call to the adapter, with the index baked in
    for (i, &id) in thunk_ids.iter().enumerate() {
        cg.func.signature = sig_tail.clone();
        {
            let mut f = FunctionBuilder::new(&mut cg.func, &mut fb_ctx);
            // **The position reaches the adapter here.** Only the body's last
            // call is in tail position, and only when `last` says so; every
            // other call drains. The adapter cannot see where it was called
            // from, so the thunk decides by importing one symbol or the other.
            let is_last = i + 1 == calls;
            let which = if is_last && last == LastCall::Tail {
                tail_adapter_id
            } else {
                adapter_id
            };
            let adapter = module.declare_func_in_func(which, f.func);
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

    Ok((entry, slots))
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
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
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
        let body = compile_body(2, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
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
        let body = compile_body(1, LastCall::Tail).expect("lowers");
        let mut vm = Interp::new();
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

        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
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
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");

        let here = bund2_interp::stack_marker();
        // A floor above us, as `bund2-interp`'s own floor test builds it.
        bund2_interp::set_stack_region(
            here + 4 * bund2_interp::STACK_RESERVE,
            bund2_interp::STACK_RESERVE,
        );
        let mut vm = Interp::new();
        bund2_interp::set_stack_region(here, 8 * 1024 * 1024);

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
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
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
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
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
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
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
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
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
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
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
        let body = compile_body(1, LastCall::Ordinary).expect("lowers");
        let mut vm = Interp::new();
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

    /// `{ 1 2 + }` — a real word's body, as the parser would leave it.
    fn add_body() -> Vec<BundValue> {
        vec![BundValue::int(1), BundValue::int(2), BundValue::call("+")]
    }

    /// **The differential that makes the claim checkable.** The same body, run
    /// through Tier 0 and through the compiled word, must leave the same stack.
    fn assert_matches_tier0(body: &[BundValue], label: &str) {
        let mut tier0 = with_stdlib();
        let by_tier0 = tier0.eval(body);

        let mut c = Compiler::new().expect("a compiler");
        let word = c.compile_word(body.len(), LastCall::Ordinary).expect("lowers");
        let mut compiled = with_stdlib();
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

    /// A compiled body that is a real word: literals pushed, a native called.
    #[test]
    fn a_compiled_word_runs_a_real_body() {
        let mut c = Compiler::new().expect("a compiler");
        let word = c.compile_word(3, LastCall::Ordinary).expect("lowers");
        assert_eq!(c.values(word), Some(3));
        let mut vm = with_stdlib();
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

        let mut c = Compiler::new().expect("a compiler");
        let word = c.compile_word(body.len(), LastCall::Ordinary).expect("lowers");
        let mut compiled = with_stdlib();
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

        let mut c = Compiler::new().expect("a compiler");
        let word = c.compile_word(2, LastCall::Ordinary).expect("lowers");
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
        let mut c = Compiler::new().expect("a compiler");
        let word = c.compile_word(2, LastCall::Ordinary).expect("lowers");
        let mut vm = with_stdlib();
        // `+` on an empty stack fails; the `99` after it must not run.
        let body = vec![BundValue::call("+"), BundValue::int(99)];
        let e = c.run(word, &mut vm, &body).expect_err("the call failed");
        assert!(!e.0.is_empty());
        assert_eq!(vm.depth(), 0, "the value after the failure must not run");
    }

    /// The tail-position variant runs the same body the same way.
    #[test]
    fn a_compiled_word_in_tail_position_runs_the_same_body() {
        let mut c = Compiler::new().expect("a compiler");
        let word = c.compile_word(3, LastCall::Tail).expect("lowers");
        assert_eq!(c.last_call(word), Some(LastCall::Tail));
        let mut vm = with_stdlib();
        c.run(word, &mut vm, &add_body()).expect("it ran");
        assert_eq!(vm.snapshot().first().and_then(BundValue::as_int), Some(3));
    }

    /// A body of a different length than the word was built for is a broken
    /// invariant in the caller, not a fact about the program.
    #[test]
    fn a_word_handed_the_wrong_body_length_is_an_internal_error() {
        let mut c = Compiler::new().expect("a compiler");
        let word = c.compile_word(3, LastCall::Ordinary).expect("lowers");
        let mut vm = with_stdlib();
        let e = c
            .run(word, &mut vm, &[BundValue::int(1)])
            .expect_err("length mismatch");
        assert!(e.is_internal(), "{}", e.0);
    }

    #[test]
    fn a_word_with_an_empty_body_is_refused() {
        let mut c = Compiler::new().expect("a compiler");
        assert!(c.compile_word(0, LastCall::Ordinary).is_err());
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
        let mut c = Compiler::new().expect("a compiler");
        let sum = c.compile_word(3, LastCall::Ordinary).expect("the first lowers");
        let lone = c.compile_word(1, LastCall::Ordinary).expect("the second lowers");
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
}
