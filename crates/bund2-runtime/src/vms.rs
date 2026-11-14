//! **D249: a Bund program starting VMs** — `vm.spawn`, `vm.join` and
//! `vm.done?`.
//!
//! RFC-0007 §C6 left open whether a Bund program creates tasks or an embedder
//! does, and followed the reference's silence: no spawn word. D249 adds the
//! words, **additively**. `bund2_async::host::Host` is unchanged and keeps its
//! own count, the bus words are unchanged, and a program that defines a `vm.`
//! word of its own keeps it, because a slot's lambda resolves before its native.
//!
//! **Why here and not in `bund2-stdlib`.** A word that starts a VM builds a
//! [`Runtime`](crate::Runtime), and `bund2-stdlib` sits below this crate. So the words are
//! registered by [`Runtime`](crate::Runtime)'s `build`, after the vocabulary, as `test.panic`
//! is. The standard library's palette audit never sees them, which is right:
//! they act on the host, and no promotion may cross them.
//!
//! **What crosses a thread is bytes** (D84, D86). `Interp` is not `Send`, so a
//! child is built on its own thread from the program, the start-up values and
//! the parent's options; the program and values go as the bus's wire format,
//! and the child's final stack comes back the same way.
//!
//! **What a child inherits** (D249 R7): the parent's `--noio`, `--noeval` and
//! `--nocolor`. A native is a plain `fn`, so it cannot ask its VM for them;
//! `vm.spawn` is registered as one of eight instances, one per combination,
//! each carrying its options as constants.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

use bund2_api::{Error, NativeFn, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LAMBDA, LIST, STRING};

use crate::HostOptions;

/// **How many VMs started from Bund may exist at once** — D85's figure, a
/// count of its own (D249 R8), so an embedder's `Host` is not affected.
pub const LIMIT: usize = 64;

/// How a child ended: its final stack, encoded as a LIST, or why it stopped.
type Outcome = Result<Vec<u8>, String>;

/// A child that has not been joined.
struct Child {
    thread: JoinHandle<()>,
    /// Written by the child's thread as its last act. Empty after the thread
    /// has ended only when the thread panicked.
    outcome: Arc<Mutex<Option<Outcome>>>,
}

#[derive(Default)]
struct Table {
    /// The last handle given out. Handles start at 1 and are never reused.
    last: i64,
    children: BTreeMap<i64, Child>,
}

fn table() -> &'static Mutex<Table> {
    static TABLE: OnceLock<Mutex<Table>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(Table::default()))
}

/// Children whose threads have not ended. Separate from the table, because a
/// child that has ended but not been joined holds no word table (D85 bounds
/// memory, not handles).
static LIVE: AtomicUsize = AtomicUsize::new(0);

/// A poisoned lock means a thread panicked while holding it: a broken
/// invariant with no sensible continuation (D37).
fn with_table<T>(f: impl FnOnce(&mut Table) -> Result<T, Error>) -> Result<T, Error> {
    let mut g = table().lock().map_err(|_| {
        Error::internal(
            "the table of VMs started with vm.spawn is poisoned: a thread panicked while \
             holding it",
        )
    })?;
    f(&mut g)
}

/// **How many VMs started with `vm.spawn` have not been joined** — running or
/// finished. The CLI warns with it when a program ends (D249 R9).
pub fn unjoined() -> usize {
    table().lock().map(|t| t.children.len()).unwrap_or(0)
}

/// Gives a slot back when the child's thread ends, however it ends — or when
/// the thread never starts, since the closure that owns it is then dropped.
struct Slot;

impl Drop for Slot {
    fn drop(&mut self) {
        LIVE.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Take a slot if one is free. Counted before the thread starts, so two
/// spawns cannot both pass the check before either has counted.
fn take_slot() -> Option<Slot> {
    LIVE.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
        (n < LIMIT).then_some(n + 1)
    })
    .ok()
    .map(|_| Slot)
}

/// What the child runs, in a form that may cross a thread.
enum Program {
    Source(String),
    /// A LAMBDA, encoded.
    Lambda(Vec<u8>),
}

/// The child's thread: build a VM, push the start-up values, run the program,
/// and give back the final stack.
fn run_child(opts: &HostOptions, program: Program, args: &[u8]) -> Outcome {
    let mut rt = crate::vm_on_this_thread(opts, None);
    let args = bund2_value::wire::from_binary(args)
        .map_err(|e| format!("its start-up values could not be decoded: {e}"))?;
    for v in args.as_list().unwrap_or_default() {
        rt.interp.push(v.clone());
    }
    let ran = match program {
        Program::Source(src) => rt.eval_str(&src),
        Program::Lambda(bytes) => match bund2_value::wire::from_binary(&bytes) {
            Ok(lambda) => {
                if let Some(e) = rt.take_startup_defect() {
                    rt.interp.report(bund2_api::diag::Diagnostic::warning(e.0));
                }
                rt.interp.eval_lambda(&lambda)
            }
            Err(e) => Err(Error(format!("its program could not be decoded: {e}"))),
        },
    };
    // `bund.exit` ends a program as it asked to, which is not a failure: the
    // evaluator answers an error only so that nothing after it runs.
    if let Err(e) = ran
        && rt.interp.exit_requested().is_none()
    {
        return Err(e.0);
    }
    bund2_value::wire::to_binary(&BundValue::list(rt.interp.snapshot()))
        .map_err(|e| format!("its final stack could not be encoded: {e}"))
}

/// The handle on top of the stack.
fn handle(vm: &mut dyn Vm, word: &str) -> Result<i64, Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("Stack is too shallow for inline {word}")))?;
    v.as_int().ok_or_else(|| {
        Error(format!(
            "{word}: a VM's handle is an INTEGER, not {}",
            v.summary(80)
        ))
    })
}

fn unknown(word: &str, id: i64) -> Error {
    Error(format!(
        "{word}: no VM {id}: vm.spawn never answered it, or it has been joined"
    ))
}

/// `vm.spawn` — `program values vm.spawn` answers a handle.
///
/// `program` is a LAMBDA or a STRING of Bund source (D249 R3); `values` is a
/// LIST whose items the child finds on its stack, first item lowest (R4). The
/// child starts with the standard vocabulary only: a LAMBDA that calls the
/// parent's own words fails in the child.
fn spawn<const NOIO: bool, const NOEVAL: bool, const NOCOLOR: bool>(
    vm: &mut dyn Vm,
) -> Result<(), Error> {
    const WORD: &str = "VM.SPAWN";
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline vm.spawn".into()));
    }
    let args = vm
        .pull()
        .ok_or_else(|| Error(format!("{WORD}: no start-up values")))?;
    let program = vm
        .pull()
        .ok_or_else(|| Error(format!("{WORD}: no program")))?;
    if args.dt() != LIST {
        return Err(Error(format!(
            "{WORD}: the start-up values must be a LIST, not {}",
            args.summary(80)
        )));
    }
    let program = match program.dt() {
        LAMBDA => Program::Lambda(
            bund2_value::wire::to_binary(&program)
                .map_err(|e| Error(format!("{WORD}: the program could not be encoded: {e}")))?,
        ),
        // **A STRING is code the program did not contain**, which is what
        // `--noeval` refuses (D249 R7 i). A LAMBDA was parsed where it was
        // written, as any lambda the program runs.
        STRING if NOEVAL => {
            return Err(Error(format!(
                "{WORD}: a STRING program is evaluation, disabled with --noeval; a LAMBDA \
                 program is not"
            )));
        }
        STRING => Program::Source(program.as_str().unwrap_or_default()),
        _ => {
            return Err(Error(format!(
                "{WORD}: the program must be a LAMBDA or a STRING, not {}",
                program.summary(80)
            )));
        }
    };
    let args = bund2_value::wire::to_binary(&args).map_err(|e| {
        Error(format!(
            "{WORD}: the start-up values could not be encoded: {e}"
        ))
    })?;
    let slot = take_slot().ok_or_else(|| {
        Error(format!(
            "{WORD}: {LIMIT} VMs started with vm.spawn are running, which is the limit (D85); \
             join one first"
        ))
    })?;
    with_table(|t| {
        let id = t.last + 1;
        let outcome = Arc::new(Mutex::new(None));
        let written = Arc::clone(&outcome);
        let opts = HostOptions {
            noio: NOIO,
            noeval: NOEVAL,
            nocolor: NOCOLOR,
        };
        let thread = crate::vm_thread(format!("bund2-vm.spawn-{id}"))
            .spawn(move || {
                // Dropped last, so the slot is given back after the child's
                // word table is freed.
                let _slot = slot;
                let ended = run_child(&opts, program, &args);
                if let Ok(mut g) = written.lock() {
                    *g = Some(ended);
                }
            })
            .map_err(|e| Error(format!("{WORD}: could not start a thread: {e}")))?;
        t.last = id;
        t.children.insert(id, Child { thread, outcome });
        vm.push(BundValue::int(id));
        Ok(())
    })
}

/// `vm.join` — `handle vm.join` waits for the child and answers its final
/// stack as a LIST, bottom first (D249 R6 A). A child that failed makes
/// `vm.join` fail with its error (R6 i). Any VM may join any handle; the first
/// join takes the answer.
fn join(vm: &mut dyn Vm) -> Result<(), Error> {
    const WORD: &str = "VM.JOIN";
    let id = handle(vm, "vm.join")?;
    // Taken out under the lock and waited for outside it, so a slow child
    // does not hold up every other spawn, join and question.
    let child = with_table(|t| t.children.remove(&id).ok_or_else(|| unknown(WORD, id)))?;
    // A thread that panicked wrote no outcome, which the match below names;
    // what `join` says about the panic adds nothing to that.
    let _ = child.thread.join();
    let outcome = child.outcome.lock().ok().and_then(|mut g| g.take());
    match outcome {
        Some(Ok(bytes)) => {
            let stack = bund2_value::wire::from_binary(&bytes)
                .map_err(|e| Error(format!("{WORD}: VM {id}'s final stack: {e}")))?;
            vm.push(stack);
            Ok(())
        }
        Some(Err(e)) => Err(Error(format!("{WORD}: VM {id} failed: {e}"))),
        None => Err(Error::internal(format!(
            "VM {id}'s thread ended by panicking, so it has no answer; Bund2 does not panic, \
             so this is a defect in Bund2 or in a native it links"
        ))),
    }
}

/// `vm.done?` — `handle vm.done?` answers whether `vm.join` would answer at
/// once, without waiting.
fn done(vm: &mut dyn Vm) -> Result<(), Error> {
    let id = handle(vm, "vm.done?")?;
    let finished = with_table(|t| {
        t.children
            .get(&id)
            .map(|c| c.thread.is_finished())
            .ok_or_else(|| unknown("VM.DONE?", id))
    })?;
    vm.push(BundValue::boolean(finished));
    Ok(())
}

/// The `vm.spawn` that passes on exactly these options.
fn spawn_for(opts: &HostOptions) -> NativeFn {
    match (opts.noio, opts.noeval, opts.nocolor) {
        (false, false, false) => spawn::<false, false, false>,
        (false, false, true) => spawn::<false, false, true>,
        (false, true, false) => spawn::<false, true, false>,
        (false, true, true) => spawn::<false, true, true>,
        (true, false, false) => spawn::<true, false, false>,
        (true, false, true) => spawn::<true, false, true>,
        (true, true, false) => spawn::<true, true, false>,
        (true, true, true) => spawn::<true, true, true>,
    }
}

/// Register the three words, for a VM built with `opts`. Public so
/// `bund2 words` lists them, as D249 says it does.
pub fn register(r: &mut Registry, opts: &HostOptions) {
    r.register_native(
        "vm.spawn",
        spawn_for(opts),
        StackEffect::fixed(2, 1),
        WordKind::Sync,
    );
    r.register_native("vm.join", join, StackEffect::fixed(1, 1), WordKind::Sync);
    r.register_native("vm.done?", done, StackEffect::fixed(1, 1), WordKind::Sync);
}
