//! Tier 0: the interpreter. Mandatory on every target.
//!
//! **Scope.** This implements RFC-0002's *dispatch* — the resolution order, the
//! stacks it dispatches over, and the `Vm` receiver `bund2-api` declares. It
//! stops short of RFC-0003's IR and frame loop, which is not written yet. What
//! it makes measurable is RFC-0002's criterion 3, which asks what a dispatch
//! allocates and until now had no VM to be measured in.
//!
//! **Not blocked on D3.** D3 rules what tier `bund.eval`'s output runs at,
//! which is a Tier-1 question; nothing here needs it. `bund.eval` is
//! implemented in `bund2-stdlib` (`singles.rs`), and applies each parsed value
//! through `Vm::apply`, so an eval'd token stream is never a compilation unit.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

mod declined;
/// Executing a word's specialised arm — RFC-0005 §S6's first consumer.
pub mod frag;
pub mod profile;
mod stacks;

/// §D1's safepoint — RFC-0008's first consumer.
pub mod debug;

use std::collections::VecDeque;
use std::rc::Rc;

use bund2_api::{Error, Registry, Resolved, Symbol, Vm};
use bund2_value::BundValue;

/// How an embedder makes the console a word attaches — RFC-0008 §W5.
pub type ConsoleFactory = fn() -> Box<dyn debug::Console>;

/// How an embedder runs a line of source in a VM — RFC-0008 §W2.
pub type Evaluator = fn(&mut dyn Vm, &str) -> Result<(), Error>;

pub use stacks::{Stack, Stacks};

// --- the machine-stack floor: RFC-0005 §S8, F85 ------------------------------

thread_local! {
    /// The machine stack this thread runs on, if whoever started the thread
    /// said so: `(top, size, share)` in bytes. `bund2`'s evaluation thread
    /// declares it; a thread that has not is treated conservatively (see
    /// [`ASSUMED_BUDGET`]).
    ///
    /// **`share` is Tier 1's part of the region, and it is declared rather
    /// than inferred** — RFC-0005 §S8, the ninth review's S3. A region
    /// declared through [`set_stack_region`] has a share of zero and is
    /// Tier 0's alone: a default split would take room from every embedder's
    /// Tier 0 without asking, against D44's "never less".
    static STACK_REGION: std::cell::Cell<Option<(usize, usize, usize)>> =
        const { std::cell::Cell::new(None) };
}

/// Bytes kept below the Tier 0 floor, for the frames between one check and
/// the next. Reporting the error needs none of it: by the time it is
/// reported, the stack has unwound.
pub const STACK_RESERVE: usize = 256 * 1024;

/// How far below its construction point an `Interp` assumes it may recurse,
/// on a thread whose stack nobody declared: half of Rust's default for a
/// spawned thread, 2 MiB, because the constructor is rarely at the very top.
const ASSUMED_BUDGET: usize = 1024 * 1024;

/// Declare the stack this thread runs on — its top, as [`stack_marker`] read
/// in the thread's entry function, and its size in bytes. Every `Interp` built
/// on this thread afterwards takes its floor from it. RFC-0005 §S8.
pub fn set_stack_region(top: usize, size: usize) {
    STACK_REGION.with(|r| r.set(Some((top, size, 0))));
}

/// Declare the stack this thread runs on **and the part of it that is
/// Tier 1's** — RFC-0005 §S8, the ninth review's S3.
///
/// `share` is Tier 1's part, in bytes, and it is *added to* Tier 0's part
/// rather than taken out of it: the region should be sized at Tier 0's part
/// plus the share. That is what makes D44's second requirement hold — a
/// program whose native nesting fits with the tier off still fits with it on —
/// because the Tier 0 floor moves *down* by the share while the Tier 1 floor
/// sits one reserve above the share's bottom.
///
/// **A share is declared, never inferred.** [`set_stack_region`] leaves it at
/// zero, which puts the Tier 1 floor above the thread's own top, so every
/// compiled body declines and compiled code does not run on that thread. An
/// embedder opts in by calling this instead; one that does not keeps its whole
/// region for Tier 0. A default split was rejected because it would take room
/// from every embedder's Tier 0 without asking.
pub fn set_stack_region_with_share(top: usize, size: usize, share: usize) {
    STACK_REGION.with(|r| r.set(Some((top, size, share))));
}

/// The address of a local in this call's frame, as an integer: the stack
/// pointer, near enough, taken in safe Rust. Nothing ever dereferences it.
/// Stacks grow downward on every target Bund2 builds for, so a smaller value
/// is deeper.
#[inline(never)]
pub fn stack_marker() -> usize {
    let marker = 0u8;
    std::ptr::addr_of!(marker) as usize
}

/// The Tier 0 floor for an `Interp` built on the current thread.
fn tier0_floor() -> usize {
    match STACK_REGION.with(std::cell::Cell::get) {
        Some((top, size, _)) => top.saturating_sub(size).saturating_add(STACK_RESERVE),
        None => stack_marker()
            .saturating_sub(ASSUMED_BUDGET)
            .saturating_add(STACK_RESERVE),
    }
}

/// **The Tier 1 floor for an `Interp` built on the current thread** —
/// RFC-0005 §S8, `top − share + STACK_RESERVE`.
///
/// One reserve above the bottom of Tier 1's share, which is a *higher* address
/// than [`tier0_floor`]: the Tier 0 floor sits one reserve above the stack's
/// end. A compiled body whose entry finds the stack pointer below this floor
/// declines, and the body runs interpreted on the heap instead.
///
/// **With no share — every thread that did not call
/// [`set_stack_region_with_share`] — this is `top + STACK_RESERVE`**, above the
/// thread's own top, so no stack pointer is ever above it and every compiled
/// body declines. That is §S8's "compiled code does not run there", reached by
/// arithmetic rather than by a flag, which is why an undeclared thread needs no
/// separate case here: it has no top to speak of, and `stack_marker` stands in
/// for one, giving the same always-decline answer.
fn tier1_floor() -> usize {
    match STACK_REGION.with(std::cell::Cell::get) {
        Some((top, _, share)) => top.saturating_sub(share).saturating_add(STACK_RESERVE),
        // Undeclared: no share, so the floor sits above the construction point
        // and nothing compiled runs, as with a declared region of zero share.
        None => stack_marker().saturating_add(STACK_RESERVE),
    }
}

/// The items a frame's body value carries: a LAMBDA's body, or a LIST's
/// elements for a body assembled at run time (`scoped_call`'s).
fn frame_items(v: &BundValue) -> Option<&[BundValue]> {
    v.as_lambda().or_else(|| v.as_list())
}

/// One body being executed, and how far through it we are.
///
/// **RFC-0003 §S4.** A frame carries the body, an instruction pointer, and an
/// optional exit action that runs when the frame leaves — however it leaves.
/// That last part is what fixes F57: the reference restores a context's stack
/// with a statement placed after three early returns, so a failure skips it.
struct Frame {
    /// The value whose body this frame runs — a LAMBDA, or a LIST for a body
    /// assembled at run time. **Held, not copied (D42)**, so the body's `Rc`,
    /// D35's cache key, is in hand for as long as the frame runs, and a body is
    /// alive whenever anything is executing it.
    body: BundValue,
    ip: usize,
    /// Run when this frame is popped, on success **and** on failure.
    exit: Option<ExitAction>,
    /// **The symbol this frame was pushed for — RFC-0008 §D2.**
    ///
    /// The body and the `ip` are enough to *resume* a body and not enough to
    /// *name* it, so a backtrace read `a lambda at 3`. This is what makes it
    /// `#1  w at 3`.
    ///
    /// `None` for a body that no word names: a `( … )` scratch context, a
    /// LIST assembled at run time, a lambda a native handed to `eval_lambda`.
    /// A backtrace says so rather than inventing a caller — D16's consequence.
    /// Where the body was *written* is another table's (`positions`, RFC-0003
    /// §S5), so a nameless lambda still has a place if it was a literal.
    ///
    /// **A frame is pushed per call, so this is the one field RFC-0008 adds to
    /// a hot structure**, and criterion 12 measures it against RFC-0005
    /// criterion 7's band rather than assuming it free.
    who: Option<Symbol>,
}

/// What a frame does on the way out.
enum ExitAction {
    /// Return to a named stack. `( … )` and `context` both need this.
    ToStack(String),
}

/// The interpreter.
///
/// **It is not `Send`, and that is load-bearing — RFC-0007 §C8 criterion 9.**
/// It holds `Box<dyn Reporter>` and, through every value, `Rc<HeapValue>`.
/// Three designs rest on it: D84 has the host own threads rather than futures,
/// RFC-0008 §D1 has the debuggee inspect itself because a debugger thread may
/// not touch it, and D86 has the bus carry bytes because nothing else can
/// cross. If this type ever became `Send`, those three would silently describe
/// a constraint that had stopped existing.
///
/// So the compiler holds the claim rather than a comment:
///
/// ```compile_fail
/// fn needs_send<T: Send>(_: T) {}
/// needs_send(bund2_interp::Interp::new());
/// ```
///
/// **With a positive control, because `compile_fail` passes for any reason** —
/// a misspelled path or a constructor that moved would make the block above
/// succeed while proving nothing. This one must compile, so the two together
/// say that the path resolves, the constructor exists, and the *only* thing
/// wrong with the first block is the `Send` bound:
///
/// ```
/// fn needs_send<T: Send>(_: T) {}
/// needs_send(7u32);
/// let _ = bund2_interp::Interp::new();
/// ```
///
/// **Doctests, deliberately.** There is no stable way to assert the *absence*
/// of an impl from inside a normal test, and a positive test of the three
/// fields would pass a refactor that replaced them with something else that
/// is also not `Send`.
pub struct Interp {
    /// Contexts opened by `( … )` and not yet closed, each with the stack to
    /// restore. **Separate from the stack-of-stacks on purpose** — the
    /// reference conflates the two and so cannot tell whether a context is
    /// open, which is F60.
    contexts: Vec<(String, String)>,
    /// Where diagnostics go. Silent by default, so a `Vm` built in a test
    /// writes to nobody's terminal; the CLI swaps in a text reporter and a TUI
    /// would swap in its own.
    pub reporter: Box<dyn bund2_api::diag::Reporter>,
    /// The frames that were running when the innermost failure was raised,
    /// with that failure's text — the Bund traceback (D173). Taken at
    /// `run_to`'s error arm before the frames unwind, only when the reporter
    /// wants it, and handed to the fatal report whose reason carries the
    /// text. A `?try` that catches the failure moves it to `caught`.
    failure_trace: Option<FailureTrace>,
    /// **The trace of the failure a `?try` caught** — handed to the report
    /// its `except` block makes of it, matched by the same text test, so a
    /// caught failure is drawn as a traceback too.
    caught: Option<FailureTrace>,
    /// Where a word's line comes from — D112. [`bund2_api::input::NoInput`]
    /// by default, so an `Interp` nobody has given an input never waits on
    /// one; the CLI swaps in the terminal, a debugged session keeps the
    /// program off the stream its commands arrive on, and a test scripts it.
    pub input: Box<dyn bund2_api::input::Input>,
    /// **Lines fed ahead of the input — RFC-0008 §W4.** `debug.feed` writes
    /// here; a read answers from here first and asks [`Interp::input`] only
    /// when it is empty. Held by the interpreter so the rule is one rule for
    /// every input an embedder may install.
    fed: VecDeque<String>,
    /// **How a line of source is run — RFC-0008 §W2.** `None` until an
    /// embedder says: this crate has no parser, for the reason it builds no
    /// child VM for a condition, so the host supplies both. A console line
    /// typed at a stop goes through it.
    pub evaluator: Option<Evaluator>,
    /// **The console a word attaches — RFC-0008 §W5, D113.5.** A script
    /// that arms a breakpoint has no `--debugger` flag and so no console; the
    /// first arming or moving word makes one with this. `None` until an
    /// embedder says, and then those words refuse in words: the console's
    /// commands are parsed and its conditions run by code this crate does not
    /// have.
    pub console_factory: Option<ConsoleFactory>,
    /// **This VM is never to be given a debugger — D115.** Set by a front
    /// end whose program must not stop and wait: a bundle is the one that
    /// does. The arming and moving words then do nothing at all — no console,
    /// no stop, and the tier stays on — where without this they would refuse
    /// for want of a console. The views still print: they read the VM and
    /// wait for nobody.
    pub console_refused: bool,
    /// **No body is offered to the tier any more — D113.6.** Raised by the
    /// first arming or moving word and never lowered. A compiled body has no
    /// safepoint, so a breakpoint inside one would silently not fire; from
    /// here every body is Tier 0. The tier is left installed so its
    /// statistics stay readable, and because it may be *running* when the
    /// word is called.
    tier_off: bool,
    /// **The bodies the tier said never to offer again** — D146, through
    /// `Vm::tier_never`. Asked before
    /// the tier is, and emptied whenever the tier changes.
    declined: declined::Declined,
    /// **Whose body the tier is being offered** — D139. `Some` for the length
    /// of `Tier::enter` and `None` otherwise, so `Vm::tier_frame` records a
    /// frame only where `push_frame` will remove it. The inner value is the
    /// word's name, which only `push_frame` knows.
    tier_entry: Option<Option<Symbol>>,
    /// **Where the compiled body's frame is in `frames`** — D139. Set by
    /// `Vm::tier_frame`, read by `Vm::tier_at`, taken when `enter` returns.
    tier_shadow: Option<usize>,
    /// **The frame stack — RFC-0003 §S4.** Bund call depth lives here, on the
    /// heap, instead of on the Rust stack.
    frames: Vec<Frame>,
    /// A body a native asked the loop to run **after it returns** — §S4a's
    /// request, in its tail-position form. The native sets it and returns; the
    /// loop pushes a frame. Nothing recurses.
    pending_tail: Option<BundValue>,
    /// **Which word the pending body belongs to — §D2.** Set beside
    /// `pending_tail` and cleared with it, so a frame pushed for a word's
    /// lambda knows its name and one pushed for an anonymous body does not.
    ///
    /// Separate from `pending_tail` rather than tupled into it so that
    /// `Option<BundValue>`'s shape, which §S6's mirror and `take_pending` both
    /// read, is unchanged.
    pending_who: Option<Symbol>,
    /// **Where bodies start running, by key — RFC-0005 criterion 20.** `None`
    /// in production, so the cost is one branch per body entry. When `Some`,
    /// every frame pushed for a body records that body's `payload_key`, which
    /// is how a test shows one key reaching every iteration of a loop (D42).
    pub entry_log: Option<Vec<usize>>,
    /// **§D1's safepoint — RFC-0008.** `None` in production, so the cost is
    /// one branch per step, which criterion 11 measures rather than assumes.
    ///
    /// When `Some`, the loops consult it before each value: it decides whether
    /// this is where the host asked to stop, and if so blocks until the host
    /// says what to do. **Boxed**, so the field a run with no debugger pays for
    /// is a pointer-width `None` rather than the whole of [`debug::Debug`].
    debug: Option<Box<debug::Debug>>,
    /// **The effect audit — RFC-0005 criterion 24.** `None` in production,
    /// so the cost is one branch per native call. When `Some`, every native
    /// that declares a fixed effect is held to it while it runs: it may not
    /// start a body, file a tail request or dispatch another word, and when it
    /// returns `Ok` on the stack it started on, that stack's depth must have
    /// moved by exactly what it declares. Each breach is recorded as a
    /// sentence. RFC-0005 §S5 keeps values in registers across such a native,
    /// so every breach is a value the native could not have seen.
    pub effect_audit: Option<Vec<String>>,
    /// **The profiler's record — RFC-0011.** `None` in a plain run, so each
    /// of the three hooks costs one branch; the CLI installs one for
    /// `--profile <base>` and renders it when the program has ended. Boxed
    /// so the branch is on a pointer-sized field.
    pub profile: Option<Box<profile::Profile>>,
    /// `debug.profile` has been told once that this run keeps no profile;
    /// later snapshots are skipped without a word (D190's note).
    profile_unkept_said: bool,
    /// The fixed-effect native running now. Only ever `Some` while the audit
    /// is on, so the checks that read it cost production one branch.
    audit_inside: Option<Symbol>,
    /// The innermost native running now, whatever its effect. Only ever `Some`
    /// while the audit is on. A report at `Error` severity made while one runs
    /// is a breach: RFC-0005 criterion 25's run-time half.
    audit_native: Option<Symbol>,
    /// **Whether anonymous bodies are named for a traceback** — D173. Read
    /// at each top-level stream: the reporter wants frames, or a debugger is
    /// attached. Off on a pipe, so the two fields below cost one branch.
    naming: bool,
    /// The word, conditional or method running now, while `naming`.
    running: Option<Runner>,
    /// `(frame index, runner)` for each body a runner handed to
    /// `eval_lambda`, innermost last — beside the frames rather than in them,
    /// so `Frame` keeps the size RFC-0008 criterion 12 measured.
    runners: Vec<(usize, Runner)>,
    /// **Where each literal body's items were written** — RFC-0003 §S5,
    /// keyed on the body's `payload_key` and holding a `Weak` to it, as
    /// D35's caches do: the entry pins nothing, and one whose `Weak` no
    /// longer upgrades is dead rather than answering for a body at a reused
    /// address. Read only to say where — a trace, `bt`, a stop.
    positions: std::collections::HashMap<usize, Placed>,
    /// The table's size at which dead entries are next swept, so a REPL that
    /// evaluates line after line does not keep every line's bodies' places.
    positions_sweep: usize,
    /// The top-level streams' places, by the key [`Interp::place_stream`]
    /// gave each — a key no body's `payload_key` can be, since its top bit
    /// is set.
    stream_places: std::collections::HashMap<usize, (Rc<str>, Vec<Place>)>,
    /// The key of the top-level stream running now, if it was placed.
    stream_key: Option<usize>,
    /// The key the next top-level stream takes.
    next_stream: Option<usize>,
    /// The index of the top-level value running now.
    top_at: usize,
    /// **The frame depth the running stream's top level sits at** — `0` for
    /// the program's stream, and the depth `use` or `bund.eval-file` was
    /// called at for a string it runs, so a frame below it is not the
    /// stream's and the stream's own place is read instead.
    stream_floor: usize,
    /// How many placed streams are running inside another — added to the
    /// frame depth a stepping command records, so `next` over `use` steps
    /// over the file's top level as it does over a body.
    nest: usize,
    /// The streams a placed one runs inside, outermost first, each as
    /// `(stream_key, top_at, stream_floor)` — what `bt` interleaves with the
    /// frames.
    stream_outer: Vec<(Option<usize>, usize, usize)>,
    /// A placed string's key by its label and text, so a `use` in a loop
    /// places its file once.
    stream_seen: std::collections::HashMap<(Rc<str>, u64), usize>,
    /// **Where each word was registered** — the `register` that bound it,
    /// the last for a name bound twice. RFC-0011 §S8's "where a word came
    /// from", read by §D5's `words` and the profile.
    definitions: std::collections::HashMap<String, bund2_api::diag::Location>,
    /// **The Tier 0 floor — RFC-0005 §S8, F85.** The lowest stack address at
    /// which a native may still re-enter evaluation. Below it,
    /// `Vm::eval_lambda`, `Vm::apply` and `Vm::scoped_call` refuse with
    /// [`Error::stack_exhausted`] instead of nesting until the process aborts.
    stack_floor: usize,
    pub registry: Registry,
    stacks: Stacks,
    /// `apply` tests this in three places, and it does **not** precede the
    /// command check — `is_command` returns at
    /// `reference/rust_multistackvm/src/multistackvm_apply.rs:17`, before the
    /// `autoadd` test at `:19`.
    ///
    /// **Private, because RFC-0005 §S6 mirrors it.** A `pub` field would let
    /// any writer set the mode without reaching the mirror, and compiled code
    /// guarding on the cell would then run the wrong arm — the failure §S6
    /// calls a stale guard. [`Interp::set_autoadd`] is the only writer, and it
    /// writes both, so the pairing is structural rather than remembered.
    autoadd: bool,
    /// The code `bund.exit` asked to end with — D52. Once set, every step
    /// refuses, so whatever is running unwinds, and the top level returns
    /// cleanly instead of reporting.
    exit_code: Option<i32>,
    /// **D55's observation audit.** While the effect audit holds a
    /// fixed-effect native, every read beyond its operands — the whole stack,
    /// the whole workbench, a stack's depth by name — is recorded here as a
    /// sentence. Empty in production, where `audit_inside`
    /// is never set; criterion 28's palette reads it.
    pub observations: std::cell::RefCell<Vec<String>>,
    /// **The cells compiled code reads — RFC-0005 §S6, *Addressing*.**
    ///
    /// Boxed for a **stable address**: §S6 has the JIT embed each cell's
    /// address as an immediate, and a field of `Interp` moves whenever the
    /// `Interp` does. The box does not, so an address handed out at compile
    /// time stays good for this interpreter's life — the same argument
    /// `Registry`'s chunked generation cells rest on (D43).
    ///
    /// One allocation per `Interp`, which is the half of criterion 23 that
    /// concerns cells.
    cells: Box<bund2_api::Cells>,
    /// **Where Tier 1 attaches — RFC-0005's seam.** `None` in a plain
    /// interpreter, so Tier 0 costs one branch per body entry; `bund2-runtime`
    /// installs the implementation, which is what keeps this crate free of
    /// `bund2-jit`. Consulted in [`Interp::push_frame`], the one place a body
    /// starts running (D42). See [`bund2_api::Tier`] for the contract.
    ///
    /// *(Until D145:)* **Taken and replaced while it runs**, because the tier
    /// needs a `&mut dyn Vm` that is this very `Interp`. A re-entrant body
    /// entry while the tier is out finds `None` and is interpreted, which is
    /// correct rather than merely convenient: nothing is lost but a
    /// compilation opportunity.
    ///
    /// **Shared, and left in place, since D145.** The paragraph above is how
    /// it was: the tier was a `Box`, taken out for the length of `enter`. A
    /// body entered beneath a compiled body then found no tier at all, however
    /// hot it was (F194). It is an `Rc` now, cloned for the call, so a nested
    /// entry is offered like any other; §S8's Tier 1 floor, not this field, is
    /// what stops a compiled recursion from running down the machine stack.
    ///
    /// **Private, and installed through [`Interp::install_tier`]**, which
    /// records the registry it was installed for — see `tier_for`.
    tier: Option<std::rc::Rc<dyn bund2_api::Tier>>,
    /// **The registry the tier was installed for** — RFC-0005 §S6, the
    /// twenty-third review's S6.
    ///
    /// Compiled code loads generation cells that belong to a `Registry`, by
    /// address, and the tier's fragment table is keyed by that registry's
    /// registration ids. [`Interp::registry`] is a public field, so an
    /// embedder can assign another registry under an installed tier: the old
    /// cells are freed and the ids mean other natives. `push_frame` compares
    /// this with the registry in place before it offers a body, and a tier
    /// whose registry has gone is removed rather than entered.
    ///
    /// **Checked at the offer, which is enough.** The registry can be replaced
    /// only by whoever holds `&mut Interp`, and evaluation holds that for as
    /// long as it runs; a native and a tier are given `&mut dyn Vm`, which has
    /// no method that replaces it. So a replacement happens between
    /// evaluations, no compiled body is on the stack when it does, and the
    /// next compiled entry would have to pass through here.
    tier_for: Option<bund2_api::RegistryIdentity>,
}

impl Default for Interp {
    fn default() -> Self {
        Self::new()
    }
}

impl Interp {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
            stacks: Stacks::default(),
            autoadd: false,
            contexts: Vec::new(),
            reporter: Box::new(bund2_api::diag::SilentReporter),
            failure_trace: None,
            caught: None,
            input: Box::new(bund2_api::input::NoInput),
            fed: VecDeque::new(),
            evaluator: None,
            console_factory: None,
            console_refused: false,
            tier_off: false,
            declined: declined::Declined::default(),
            tier_for: None,
            tier_entry: None,
            tier_shadow: None,
            frames: Vec::new(),
            pending_tail: None,
            pending_who: None,
            entry_log: None,
            debug: None,
            effect_audit: None,
            profile: None,
            profile_unkept_said: false,
            audit_inside: None,
            audit_native: None,
            naming: false,
            running: None,
            runners: Vec::new(),
            positions: std::collections::HashMap::new(),
            positions_sweep: Self::POSITIONS_SWEEP,
            stream_places: std::collections::HashMap::new(),
            stream_key: None,
            next_stream: None,
            top_at: 0,
            stream_floor: 0,
            nest: 0,
            stream_outer: Vec::new(),
            stream_seen: std::collections::HashMap::new(),
            definitions: std::collections::HashMap::new(),
            stack_floor: tier0_floor(),
            exit_code: None,
            observations: std::cell::RefCell::new(Vec::new()),
            cells: {
                // §S8's floor cell, set once: the floor is a property of the
                // thread this `Interp` was built on and never moves, so unlike
                // the other three cells it has nothing to drift from.
                let cells = bund2_api::Cells::default();
                cells.set_floor(tier1_floor());
                Box::new(cells)
            },
            tier: None,
        }
    }

    /// **§S8's two floors, as addresses: Tier 0's, then Tier 1's** — RFC-0005
    /// criterion 11 as D147 leaves it, which asks a run to report both.
    ///
    /// Tier 0 refuses to nest a body below the first; a compiled body
    /// declines to start below the second, which is the higher address
    /// wherever a share was declared. On a thread with no share the second is
    /// above the thread's own top, so every compiled entry declines.
    pub fn floors(&self) -> (usize, usize) {
        (self.stack_floor, self.cells.floor())
    }

    /// **The cells compiled code reads** — RFC-0005 §S6. See [`Interp::cells`].
    pub fn cells(&self) -> &bund2_api::Cells {
        &self.cells
    }

    /// Install a tier, **built for the registry this `Interp` holds now**.
    ///
    /// The registry is recorded, and a tier is never entered once another
    /// registry has taken its place: see `tier_for`. A tier built from some
    /// other registry's fragment table is the caller's mistake and nothing
    /// here can see it, which is why `bund2-runtime` builds the table and
    /// installs the tier in one place.
    pub fn install_tier(&mut self, tier: std::rc::Rc<dyn bund2_api::Tier>) {
        self.tier_for = Some(self.registry.identity());
        self.tier = Some(tier);
        // What the last tier declined says nothing about this one.
        self.declined.clear();
    }

    /// The installed tier, to read its statistics.
    pub fn tier(&self) -> Option<&std::rc::Rc<dyn bund2_api::Tier>> {
        self.tier.as_ref()
    }

    /// Take the tier out. Put it back with [`Interp::install_tier`].
    pub fn take_tier(&mut self) -> Option<std::rc::Rc<dyn bund2_api::Tier>> {
        self.tier_for = None;
        self.declined.clear();
        self.tier.take()
    }

    /// The tier a body may be offered to: the installed one, unless the
    /// registry it was installed for has been replaced. Then it is removed,
    /// once, with a warning, and every body from here is Tier 0 — which is
    /// correct, and all that can be said for a tier whose guards read another
    /// registry's cells.
    fn tier_to_offer(&mut self) -> Option<std::rc::Rc<dyn bund2_api::Tier>> {
        let tier = self.tier.as_ref()?;
        if self
            .tier_for
            .as_ref()
            .is_some_and(|id| self.registry.is(id))
        {
            return Some(std::rc::Rc::clone(tier));
        }
        self.tier = None;
        self.tier_for = None;
        self.declined.clear();
        <Self as Vm>::report(
            self,
            bund2_api::diag::Diagnostic::warning(
                "the word table was replaced after the compiled tier was installed for it; \
                 the tier is removed and every body runs interpreted from here",
            ),
        );
        None
    }

    /// The stacks, to read. Nothing outside this crate can move the current
    /// stack except through [`Vm`], which is what keeps §S6's epoch cell true
    /// (RFC-0005 assumption 40).
    pub fn stacks(&self) -> &Stacks {
        &self.stacks
    }

    /// **Set `autoadd` — the only writer**, so the mode and RFC-0005 §S6's
    /// mirror are written together and cannot drift.
    ///
    /// `:` and `;` call this, through `Vm::set_autoadd`, since F84's fix. The
    /// field was made private before they were bound, while its only callers
    /// were tests: the invariant is cheap to make structural while there are
    /// few writers, and expensive afterwards.
    pub fn set_autoadd(&mut self, on: bool) {
        self.autoadd = on;
        self.cells.set_autoadd(on);
    }

    /// Whether `autoadd` is on.
    pub fn autoadd(&self) -> bool {
        self.autoadd
    }

    /// Record, for D55, that the fixed-effect native running under the audit
    /// read beyond its operands, and how. Outside the audit this is a single
    /// branch on `audit_inside`, which production never sets.
    fn note_observation(&self, how: &str) {
        let Some(who) = self.audit_inside else {
            return;
        };
        if let Ok(mut log) = self.observations.try_borrow_mut() {
            log.push(format!("`{}` {how}", self.registry.interner.name(who)));
        }
    }

    /// Refuse to take another step once an exit has been requested (D52).
    ///
    /// The refusal is an error only so that it unwinds: every native between
    /// here and the top level passes it up, including one that catches errors,
    /// because that native's next step is refused too. The top level recognises
    /// it by the request, not by the text, and returns cleanly.
    fn exit_gate(&self) -> Result<(), Error> {
        match self.exit_code {
            // **One constructor, shared with the compiled tier.** RFC-0005
            // §S5's `status_of` makes this same refusal where compiled code has
            // no next step to make it at, and criterion 30 compares the two as
            // text. A second spelling a crate away would drift silently.
            Some(code) => Err(Error::exited(code)),
            None => Ok(()),
        }
    }

    /// Is there room above the Tier 0 floor to re-enter evaluation?
    fn stack_ok(&self) -> bool {
        stack_marker() > self.stack_floor
    }

    /// Call a native, holding it to its declared effect when the audit is on
    /// ([`Interp::effect_audit`]).
    ///
    /// An opaque native is not held to anything, and the words it reaches are
    /// audited on their own. A fixed-effect native is marked as running, so
    /// that a body, a tail request or a dispatch it causes is recorded, and its
    /// depth change is compared with its declaration. The comparison is skipped
    /// when the native fails, or leaves a different stack current: a switch is
    /// §S5's epoch, not a depth.
    fn call_native(&mut self, name: Symbol, n: bund2_api::Native) -> Result<(), Error> {
        // **§D3's breakpoint, for a word that pushes no frame.** A native is
        // a word and has no body, so the frame push that stops a lambda never
        // sees it (Q41). The stop is before the native runs: its operands are
        // still on the stack, which is what a condition is shown. One branch
        // when nothing is attached, as at the frame push.
        if self.debug.is_some()
            && let Some(word) = self.breakpoint_hit(Some(name))
        {
            self.safepoint_for(None, debug::Stop::Breakpoint(word));
            // A line typed at the stop may have asked the program to exit.
            if let Some(code) = self.exit_code {
                return Err(Error::exited(code));
            }
        }
        // **RFC-0011 §S2, hook 1.** Opened after the breakpoint above, so a
        // stop's wall time lies in the caller, and closed whatever the native
        // answered. The native's own frames are closed by the pops inside it.
        let profiled = self.profile.is_some();
        if profiled {
            let depth = self.frames.len();
            let site = self.call_site();
            if let Some(p) = self.profile.as_mut() {
                p.open_native(name, n.kind, depth, site);
                // D191: the trace, named here, where the interner is.
                if p.tracing() {
                    p.trace_call(self.registry.interner.name(name));
                }
            }
        }
        let named = self.naming;
        let outer = if named {
            self.running.replace(Runner::Word(name))
        } else {
            None
        };
        let r = if self.effect_audit.is_none() {
            self.invoke(name, n)
        } else {
            if let Some(outer) = self.audit_inside {
                let callee = self.registry.interner.name(name).to_string();
                self.audit_breach(
                    outer,
                    &format!("declares a fixed effect and dispatched `{callee}`"),
                );
            }
            let outer_native = self.audit_native.replace(name);
            let r = self.call_audited(name, n);
            self.audit_native = outer_native;
            r
        };
        if named {
            self.running = outer;
        }
        if profiled && let Some(p) = self.profile.as_mut() {
            if let Err(e) = &r {
                p.mark_failure(&e.0);
            }
            p.close_leaf();
        }
        r
    }

    /// **RFC-0011 §S2, hook 3's close**: the frames changed depth, so every
    /// entry of the profiler's mirror that belonged to a deeper frame closes.
    /// Called at every site that pops or truncates `frames`.
    fn profile_frames_popped(&mut self) {
        if let Some(p) = self.profile.as_mut() {
            p.close_to(self.frames.len());
        }
    }

    /// [`Interp::call_native`]'s audited half: everything but tracking which
    /// native is innermost.
    fn call_audited(&mut self, name: Symbol, n: bund2_api::Native) -> Result<(), Error> {
        let outer = self.audit_inside.take();
        if n.effect.opaque {
            let r = self.invoke(name, n);
            self.audit_inside = outer;
            return r;
        }
        // **The current stack only.** `StackEffect` has one axis, and RFC-0004
        // §S1 says what it counts: the main stack, with the workbench axis not
        // yet built. It is also the only count §S5 relies on, since promotion
        // holds current-stack values and models the depth after a call from
        // this pair. The workbench delta is reported beside it, as evidence.
        // The interpreter's own reads go to the stacks directly, so they are
        // never taken for the native's (D55).
        let stack = self.stacks.current_name().to_string();
        let (main0, wb0) = (self.depth(), self.stacks.workbench.len());
        self.audit_inside = Some(name);
        let r = self.invoke(name, n);
        self.audit_inside = outer;
        if r.is_ok() && self.stacks.current_name() == stack {
            let (main1, wb1) = (self.depth(), self.stacks.workbench.len());
            let (c, p) = (
                usize::from(n.effect.consumes),
                usize::from(n.effect.produces),
            );
            if main0.checked_sub(c).map(|d| d + p) != Some(main1) {
                self.audit_breach(
                    name,
                    &format!(
                        "declares a fixed effect and declares ({c}, {p}) and moved `{stack}` from {main0} to {main1} and the workbench from {wb0} to {wb1}"
                    ),
                );
            }
        }
        r
    }

    /// Run a native's function: **the one place Tier 0 calls one.**
    ///
    /// - A panic becomes `Error::internal` naming the native (D49), where it
    ///   used to unwind out of the evaluation thread and end the process with
    ///   exit 1 (F95).
    /// - A native that fails leaves no tail request behind (F96). A request it
    ///   filed before failing would otherwise wait in `pending_tail` and run at
    ///   the next `take_pending`, after `?try` had caught the error, as a body
    ///   nobody asked for.
    fn invoke(&mut self, name: Symbol, n: bund2_api::Native) -> Result<(), Error> {
        let r = match bund2_api::catch_panic(|| (n.f)(&mut *self)) {
            Ok(r) => r,
            Err(msg) => Err(bund2_api::panicked(
                &format!("native `{}`", self.registry.interner.name(name)),
                &msg,
            )),
        };
        if r.is_err() {
            self.pending_tail = None;
            // F96's clear reaches the mirror too, or compiled code would drain
            // a body Tier 0 has already discarded.
            self.cells.set_request(false);
        }
        r
    }

    /// Record one breach, while the audit is on: `who` did `what`.
    fn audit_breach(&mut self, who: Symbol, what: &str) {
        let line = format!("`{}` {what}", self.registry.interner.name(who));
        if let Some(log) = self.effect_audit.as_mut() {
            log.push(line);
        }
    }

    /// Dispatch a call.
    ///
    /// The order is the reference's, from
    /// `reference/rust_multistackvm/src/multistackvm_apply.rs:9-62`: command
    /// first and returning immediately, then the `autoadd` branch, then the
    /// sigil deciding whether `lambda` is consulted, then `native`.
    ///
    /// **Takes a `Symbol`, not a string.** That is the whole point of
    /// RFC-0002: the reference allocates thirteen strings and hashes eight
    /// times to dispatch `dup`, and none of it carries information a caller
    /// could not have resolved once.
    pub fn dispatch(&mut self, s: Symbol, sigil: bool) -> Result<(), Error> {
        // The verdict and the slot it was read from, in one walk of the alias
        // chain: the `Lambda` and `Native` arms need the target, and asking
        // `resolve_target` separately walked `dup` to `dup_one` a second time
        // on every call.
        let (resolved, target) = self.registry.resolve_to(s, sigil);
        match resolved {
            Resolved::Command => {
                // `resolve` said the slot holds a command; if the binding has
                // gone in between, that is a broken invariant and not the
                // program's fault, so it is reported rather than asserted.
                let n = self
                    .registry
                    .slot(s)
                    .and_then(|sl| sl.command)
                    .ok_or_else(|| {
                        Error::internal(format!(
                            "`{}` resolved to a command whose binding is absent",
                            self.registry.interner.name(s)
                        ))
                    })?;
                self.call_native(s, n)
            }
            _ if self.autoadd() => {
                // **The CALL is appended *into* the value beneath, leaving one
                // value** (`reference/rust_multistackvm/src/multistackvm_apply.rs:19-22`):
                // `pull`, `val.push(value)`, `push`. The name is what a CALL
                // carries, so this needs the interner — which is why `autoadd`
                // is a dispatch concern and not a resolution one.
                //
                // **F84 was this arm.** It pushed the value beneath back and
                // then pushed the CALL as a *separate* value — two where the
                // reference leaves one — and the test named below pinned that
                // shape rather than catching it.
                let name = self.registry.interner.name(s).to_string();
                let Some(into) = self.pull() else {
                    return Err(Error("Autoadd found no working data on stack".into()));
                };
                self.push(into.push(BundValue::call(name)));
                Ok(())
            }
            Resolved::Lambda => {
                // `lambda_eval` applies each element of the body
                // (`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:13-14`).
                //
                // The reference recurses in Rust here, so its Bund call depth
                // is Rust call depth. Bund2 does not: the body is cloned out,
                // so the registry is not borrowed, and handed to the loop as a
                // tail request below (RFC-0003 §S4's frame loop).
                self.alias_breakpoint(s, target)?;
                if s != target
                    && let Some(p) = self.profile.as_mut()
                {
                    p.spelled(target, profile::Kind::Word, s);
                }
                let body = self
                    .registry
                    .slot(target)
                    .and_then(|slot| slot.lambda.clone())
                    .ok_or_else(|| Error("resolved to a lambda that is not there".into()))?;
                // `lambda_eval` bails on a non-LAMBDA
                // (`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:27-29`).
                // The `register` word guards the type, but the registry API
                // does not, so the check belongs here too.
                if body.as_lambda().is_none() {
                    return Err(Error("This is not a lambda".into()));
                }
                // **The call that used to recurse.** Calling a lambda is a
                // tail position: nothing in `dispatch` runs after the body. So
                // it becomes a request, and the loop pushes a frame — Bund
                // call depth stops being Rust call depth.
                self.request_tail_for(body, Some(target));
                Ok(())
            }
            Resolved::Native => {
                self.alias_breakpoint(s, target)?;
                if s != target
                    && let Some(p) = self.profile.as_mut()
                {
                    p.spelled(target, profile::Kind::Native, s);
                }
                let n = self
                    .registry
                    .slot(target)
                    .and_then(|sl| sl.native)
                    .ok_or_else(|| {
                        Error::internal(format!(
                            "`{}` resolved to a native whose binding is absent",
                            self.registry.interner.name(target)
                        ))
                    })?;
                self.call_native(target, n)
            }
            // The name the alias chain ended at, as the reference's `i`
            // names `real_name`.
            Resolved::Unbound => Err(unregistered(self.registry.interner.name(target))),
        }
    }

    /// **A breakpoint on an alias stops where the alias is called.** A frame
    /// and a native are both known by the word the alias resolves to, so
    /// `break dup` would arm a name nothing is ever called under. Asked only
    /// when the call was through an alias; the word's own name is asked where
    /// it always was, so arming both stops twice.
    fn alias_breakpoint(&mut self, called: Symbol, target: Symbol) -> Result<(), Error> {
        if self.debug.is_none() || called == target {
            return Ok(());
        }
        if let Some(word) = self.breakpoint_hit(Some(called)) {
            self.safepoint_for(None, debug::Stop::Breakpoint(word));
            if let Some(code) = self.exit_code {
                return Err(Error::exited(code));
            }
        }
        Ok(())
    }

    /// Dispatch by name, as `execute` does with a string off the stack.
    ///
    /// **A miss must not intern.** D16 makes a computed name expressible, and
    /// interning every lookup would grow memory without bound on a program
    /// that dispatches a miss in a loop — which the reference, whose tables
    /// are pure reads, does not do.
    pub fn dispatch_name(&mut self, name: &str) -> Result<(), Error> {
        match self.registry.interner.lookup_call(name) {
            Some((s, sigil)) => self.dispatch(s, sigil),
            None => Err(unregistered(name)),
        }
    }

    /// Apply one value, as `VM::apply` does
    /// (`reference/rust_multistackvm/src/multistackvm_apply.rs:8-104`).
    ///
    /// Three arms, and the third is everything else:
    ///
    /// - **CALL** — dispatch. RFC-0002's registry already folds in the command,
    ///   sigil, alias and lambda ordering the reference spells out at `:16-59`.
    /// - **CONTEXT** — switch stacks (`:69-87`). `( … )` opens one this way and
    ///   `endcontext` closes it.
    /// - **anything else** — push (`:99`).
    ///
    /// `autoadd`'s collecting is not implemented, so the reference's branches
    /// at `:19` (the CALL arm) and `:89` (the literal arm) are absent, and so
    /// is the CONTEXT arm's at `:72-73`. When list construction lands it
    /// belongs here, not at the call sites.
    /// Apply one value **synchronously**: whatever it asks for has run by the
    /// time this returns.
    ///
    /// This is what a native gets through the `Vm` trait, because a native that
    /// applies a value and then inspects the stack must see the result. The
    /// evaluation loop uses [`Interp::apply_step`] instead, which leaves the
    /// request for the loop to fulfil — that is the difference between one Rust
    /// frame per Bund call and none.
    pub fn apply(&mut self, v: BundValue) -> Result<(), Error> {
        // A native re-entering evaluation spends Rust stack (RFC-0005 §S8).
        if !self.stack_ok() {
            return Err(Error::stack_exhausted());
        }
        let floor = self.frames.len();
        self.apply_step(v)?;
        // `take_pending` reaches `push_frame`, where a tier may run the body and
        // answer an error. Unwind to the floor this call recorded, as `run_to`'s
        // error arm does, rather than leaving frames above it for the next
        // caller to find.
        if let Err(e) = self.take_pending() {
            self.unwind_to(floor);
            return Err(e);
        }
        self.run_to(floor)?;
        // F112: a synchronous run that ended by requesting an exit returns
        // the refusal, not `Ok`, so the native that asked runs nothing more.
        self.exit_gate()
    }

    /// Apply one value, leaving any requested body for the caller's loop.
    pub fn apply_step(&mut self, v: BundValue) -> Result<(), Error> {
        self.exit_gate()?;
        match v.dt() {
            bund2_value::CALL => {
                // **A body's CALL resolves its name once.** The memo is the
                // symbol this interpreter found last time; a miss — the first
                // call, or a memo another interpreter left — looks the name up
                // and leaves one. An unregistered name leaves none, since a
                // later `register` can bind it. Borrowed, not copied: the copy
                // and the hashed lookup were a fifth of a recursive `fib`.
                if let Some((s, sigil)) = self.registry.interner.from_memo(v.call_memo()) {
                    return self.dispatch(s, sigil);
                }
                let name = v
                    .str_ref()
                    .ok_or_else(|| Error("Empty function name passed for CALL".into()))?;
                if name.is_empty() {
                    return Err(Error("Empty function name passed for CALL".into()));
                }
                match self.registry.interner.lookup_call(name) {
                    Some((s, sigil)) => {
                        v.set_call_memo(self.registry.interner.memo_for(s, sigil));
                        self.dispatch(s, sigil)
                    }
                    None => Err(unregistered(name)),
                }
            }
            bund2_value::CONTEXT => {
                let name = v.as_str().ok_or_else(|| {
                    Error("Can not get the name of context from the CONTEXT value".into())
                })?;
                // **Collecting: pushed, not switched to, and not appended
                // either** (`reference/rust_multistackvm/src/multistackvm_apply.rs:72-73`).
                //
                // The asymmetry is the reference's and is preserved: every
                // other value is *appended* into the collector under
                // `autoadd`, and a CONTEXT is *pushed on top of it*. So the
                // collector stops being the top of the stack and whatever
                // follows appends into the CONTEXT value instead. Surprising,
                // and what the oracle does.
                if self.autoadd {
                    self.push(v);
                    return Ok(());
                }
                // `VM::to_stack` switches *and* pushes the name onto the
                // nesting stack (`reference/rust_multistackvm/src/multistackvm_to_stack.rs:5-19`),
                // which is what `endcontext` later pops. Every switch counts,
                // `@name` included — the difference from the reference is only
                // that this stack starts **empty** rather than holding `main`,
                // so a bare `endcontext` has nothing to pop and F60's guard can
                // finally fire.
                self.push_context(&name);
                self.to_stack(&name);
                Ok(())
            }
            _ => {
                // **Collecting: appended into the value beneath**
                // (`reference/rust_multistackvm/src/multistackvm_apply.rs:89-97`).
                // Literals included — which is the half F84 recorded as
                // absent. `BundValue::push` is the append, and on a
                // non-container it *replaces*, so `5 : 1 ;` leaves `1`.
                if self.autoadd {
                    let Some(into) = self.pull() else {
                        return Err(Error("Autoadd found no working data on stack".into()));
                    };
                    self.push(into.push(v));
                    return Ok(());
                }
                self.push(v);
                Ok(())
            }
        }
    }

    /// **The one evaluator** — RFC-0003 §S6.
    ///
    /// The reference has three copies of this loop, two identical and one that
    /// prints and steps (F52): `reference/bundcore/src/bundcore_eval.rs:7-45`,
    /// `reference/Bund/src/stdlib/helpers/eval.rs:7-41`, and the debugger's at
    /// `reference/Bund/src/stdlib/functions/debug_fun/debug_debug.rs:52-95`.
    /// This is all three, with the difference between them passed in as an
    /// observer rather than duplicated.
    ///
    /// The four arms are the reference's: `NONE` continues, `EXIT` breaks,
    /// `ERROR` bails, everything else applies. The error text on the last arm
    /// is preserved verbatim in *shape*; the interpolated value cannot
    /// reproduce, because `{:?}` embeds `id` and `stamp` (F14).
    pub fn eval(&mut self, stream: &[BundValue]) -> Result<(), Error> {
        self.eval_observed(stream, &mut |_| {}).map_err(|(i, e)| {
            // The reference wraps the failure with the value that caused
            // it (`reference/bundcore/src/bundcore_eval.rs:33`), which is
            // how it says *where* — it has no source positions. Bund2 has
            // spans, so `eval_indexed` hands back the index and the bare
            // reason and lets the caller say where properly. This wrapper
            // survives only for callers that want the reference's text.
            let v = stream.get(i).map(|v| v.render(false)).unwrap_or_default();
            Error(format!(
                "Attempt to evaluate value {v} returned error: {}",
                e.0
            ))
        })
    }

    /// [`Interp::eval`], reporting **which value** failed.
    ///
    /// The index is into the stream the caller lowered, so a caller holding
    /// `Lowered` can turn it into a source position. That is the whole of
    /// RFC-0003 §S5 that a flat `Vec<BundValue>` can support: top-level
    /// positions, not positions inside a lambda body.
    pub fn eval_indexed(&mut self, stream: &[BundValue]) -> Result<(), (usize, Error)> {
        self.eval_observed(stream, &mut |_| {})
    }

    /// [`Interp::eval`] with a per-word observer. The debugger's copy is this
    /// with a printing, stepping observer.
    ///
    /// The stream is the one [`Interp::place_stream`] last placed, if that
    /// placing has not been taken by another stream yet; the outer stream's
    /// key and position are put back when this one ends.
    pub fn eval_observed(
        &mut self,
        stream: &[BundValue],
        observe: &mut dyn FnMut(&BundValue),
    ) -> Result<(), (usize, Error)> {
        let outer = (self.stream_key, self.top_at);
        self.stream_key = self.next_stream.take();
        let r = self.eval_stream(stream, observe);
        (self.stream_key, self.top_at) = outer;
        r
    }

    fn eval_stream(
        &mut self,
        stream: &[BundValue],
        observe: &mut dyn FnMut(&BundValue),
    ) -> Result<(), (usize, Error)> {
        // A trace left by a failure an earlier stream caught is not this
        // stream's (D173).
        self.failure_trace = None;
        self.caught = None;
        self.naming = self.reporter.wants_frames() || self.debug.is_some();
        for (i, v) in stream.iter().enumerate() {
            self.top_at = i;
            match v.dt() {
                bund2_value::NONE => continue,
                bund2_value::EXIT => break,
                _ => {
                    // The top-level half of §D1's safepoint. Both loops need
                    // it: this one runs the stream, `run_to` runs every body
                    // the stream enters, and a debugger that saw only one
                    // would step over whole subtrees or never start.
                    if self.debug.is_some() {
                        // **The top level has no frame to read**, so the value
                        // about to run is handed over. Without it every stop
                        // at the top level renders the same line, which is the
                        // invisible cursor `Console::stopped` exists to avoid.
                        self.safepoint(Some((i, v)));
                    }
                    observe(v);
                    // **D188: a top-level value is a step too**, the root's
                    // own, since this loop and not `run_to` dispatches it.
                    if let Some(p) = self.profile.as_mut() {
                        p.step();
                    }
                    // A step refused because the program asked to exit is not
                    // a failure: the program is over, and there is nothing to
                    // report (D52).
                    if let Err(e) = self.apply_step(v.clone()) {
                        if self.debug.is_some() {
                            self.catch_failure(&e);
                        }
                        if let Some(p) = self.profile.as_mut() {
                            p.mark_failure(&e.0);
                        }
                        if self.exit_code.is_some() {
                            return Ok(());
                        }
                        return Err((i, e));
                    }
                    // A top-level word may have asked for a body to run.
                    if let Err(e) = self.drain_frames() {
                        if let Some(p) = self.profile.as_mut() {
                            p.mark_failure(&e.0);
                        }
                        if self.exit_code.is_some() {
                            return Ok(());
                        }
                        return Err((i, e));
                    }
                }
            }
        }
        Ok(())
    }

    /// Ask the loop to run `body` **after the current native returns**.
    ///
    /// This is §S4a's request mechanism in the shape that covers tail
    /// positions — which is most of them: a lambda call, `if`'s branch, a
    /// `through` conditional's body, `execute` on a LAMBDA. The native sets the
    /// request and returns; the loop pushes a frame; **no Rust frame is added
    /// per Bund call**, which is the whole point.
    ///
    /// Natives that must inspect the stack *between* two evaluations —
    /// `?ifthenelse`, `?try`, `times` — still call [`Vm::eval_lambda`], which
    /// is synchronous. Those add one Rust frame per *native*, not per Bund
    /// call, so depth is bounded by how deeply such natives nest rather than by
    /// how deep the program recurses.
    pub fn request_tail(&mut self, body: BundValue) {
        self.request_tail_for(body, None);
    }

    /// [`Interp::request_tail`], naming the word the body belongs to — §D2.
    ///
    /// Only `dispatch` can say this: by the time `take_pending` runs, the
    /// symbol that resolved to the body is out of scope. So the symbol travels
    /// with the request rather than being recovered from the body, which could
    /// not work anyway — two words may share one lambda.
    pub fn request_tail_for(&mut self, body: BundValue, who: Option<Symbol>) {
        if let Some(breach) = self.audit_inside {
            self.audit_breach(breach, "declares a fixed effect and filed a tail request");
        }
        self.pending_tail = Some(body);
        self.pending_who = who;
        // RFC-0005 §S6's mirror, written with the truth (assumption 33).
        self.cells.set_request(true);
    }

    /// Push a frame for whatever the last native requested, if anything.
    fn take_pending(&mut self) -> Result<(), Error> {
        if let Some(body) = self.pending_tail.take() {
            // Cleared **before** the body runs, not after: the body may file a
            // request of its own, and clearing afterwards would erase it. The
            // symbol is taken in the same breath, for the same reason.
            let who = self.pending_who.take();
            self.cells.set_request(false);
            return self.push_frame(body, None, who);
        }
        Ok(())
    }

    /// Push a frame for `body` — **the one place a body starts running**, so
    /// the one place its key is observed (D42, RFC-0005 criterion 20), and
    /// therefore **the one place Tier 1 is offered it** (RFC-0005's seam).
    ///
    /// Returns `Err` when a tier ran the body and it failed, or when the OOM
    /// guard has tripped; an interpreted body's failure comes later, from
    /// `run_to`.
    ///
    /// **RFC-0005 §S5's hand-back is taken here.** A body compiled for tail
    /// position returns with the body its last value called still filed, so
    /// that call is not made a Rust frame deeper; this takes it at once, and
    /// the next one that hands back, until a body is interpreted or nothing
    /// is filed. Every entry starts a body here, so every entry takes the
    /// hand-back. A request filed before this was called is the caller's,
    /// and is left alone.
    ///
    /// **The compiled body's frame stays, finished, beneath the one it
    /// handed back** — D139: a word that reads the frame list reads Tier 0's,
    /// and Tier 0 keeps a caller's frame until its last callee returns. So
    /// the frames grow on the heap per level as Tier 0's do, and the machine
    /// stack does not; the caller's `run_to` pops them in order.
    fn push_frame(
        &mut self,
        body: BundValue,
        exit: Option<ExitAction>,
        who: Option<Symbol>,
    ) -> Result<(), Error> {
        let theirs = self.pending_tail.is_some();
        let (mut body, mut exit, mut who) = (body, exit, who);
        loop {
            self.push_frame_once(body, exit, who, !theirs)?;
            if theirs {
                return Ok(());
            }
            let Some(next) = self.pending_tail.take() else {
                return Ok(());
            };
            who = self.pending_who.take();
            self.cells.set_request(false);
            (body, exit) = (next, None);
        }
    }

    fn push_frame_once(
        &mut self,
        body: BundValue,
        exit: Option<ExitAction>,
        who: Option<Symbol>,
        may_hand_back: bool,
    ) -> Result<(), Error> {
        // **D171's OOM guard.** Every body starts here, so a program that
        // grows by calling — a word that calls itself forever, a loop pushing
        // a value per round — reaches this within one call of the trip. The
        // body is refused before it has a frame, and the error leaves the way
        // any failing call does: reported with its backtrace and stack, and
        // catchable by `?try`. One relaxed load when nothing is pending.
        if let Some(trip) = bund2_api::memguard::take() {
            return Err(trip.error());
        }
        if let Some(who) = self.audit_inside {
            self.audit_breach(who, "declares a fixed effect and started a body");
        }
        if let (Some(log), Some(k)) = (self.entry_log.as_mut(), body.payload_key()) {
            log.push(k);
        }

        // **The seam.** Offered only for a body with a key and no exit action:
        // a keyless body is an unboxed scalar, which has no items to run, and
        // an exit action must run when the frame leaves (F57) — a tier that
        // ran the body would bypass it and leave the stack unrestored. That
        // second condition is what keeps `scoped_call`'s per-call LIST, which
        // §S3 says never reaches the tier, out of it: the LIST has a key. Both conditions are cheap and neither is a special case:
        // they are the same rule the cache keys on.
        //
        // The tier is **cloned for the call and left in place** — D145. It
        // was taken out and put back, because `enter` needs a `&mut dyn Vm`
        // that is this `Interp`; while it was out a body entered beneath a
        // compiled one saw `None` and was interpreted, however hot (F194).
        // An `Rc` and a `&self` method give `enter` the same `Interp` without
        // hiding the tier from what it calls.
        //
        // **A body the tier declined for good is not offered** — D146. It is
        // asked for before the tier is touched, which is the point: reaching
        // the tier's own answer cost about 120 instructions an entry (F201).
        if exit.is_none()
            && self.tier.is_some()
            && !self.tier_off
            && let Some(key) = body.payload_key()
            && !self.declined.holds(key)
            && let Some(tier) = self.tier_to_offer()
        {
            // **D139: a compiled body has a frame while it runs.** The tier
            // asks for one through `Vm::tier_frame` when it is about to run
            // the body, and it is removed here however `enter` answers — so
            // a body the tier declines after asking is pushed afresh below,
            // and one that failed leaves nothing above the caller's floor.
            // It carries no exit action: a body with one is never offered.
            //
            // **Saved and restored, not cleared** — D145. This entry may be
            // beneath a compiled body that has a frame of its own, and
            // `Vm::tier_at` must go on writing to that one once this returns.
            let depth = self.frames.len();
            let outer_entry = self.tier_entry.replace(who);
            let outer_shadow = self.tier_shadow.take();
            let answered = tier.enter(&body, self);
            // **D173's traceback, taken while the body's frame is still here.**
            // Tier 0 notes a failure in `run_to`, before the frames unwind, so
            // the report names the value that failed inside the body and the
            // innermost message. A compiled body fails with no `run_to` of its
            // own, and its frame (D139, its `ip` kept by `Vm::tier_at`) is
            // removed just below: noted after that, a failure under `map`
            // was reported at `map` with every wrapper, and the tiers'
            // reports differed (found by `cargo xtask fuzz`, D218). A failure
            // already noted deeper is kept: the innermost wins.
            if let Some(Err(e)) = &answered {
                self.note_failure(e);
            }
            let handed_back =
                may_hand_back && matches!(answered, Some(Ok(()))) && self.pending_tail.is_some();
            if self.tier_shadow.take().is_some() && !handed_back {
                self.frames.truncate(depth);
                self.profile_frames_popped();
            }
            self.tier_entry = outer_entry;
            self.tier_shadow = outer_shadow;
            if let Some(outcome) = answered {
                // Compiled code ran it. No frame: the caller's `run_to` finds
                // nothing to do, which is how a body that ran without a frame
                // stays invisible to the loop.
                return outcome;
            }
        }

        // **§D3's breakpoint, before the frame exists — criterion 7.** The
        // frame push is one place, so this is one comparison, and it happens
        // *before* the push so the stop is at the call: frame depth is the
        // caller's and the stack is what it was when the word was reached.
        //
        // Guarded twice: one branch when nothing is attached, and a second
        // when a debugger is attached but watches no word. A debugged session
        // that only steps pays no lookup per call.
        if let Some(name) = self.breakpoint_hit(who) {
            self.safepoint_for(None, debug::Stop::Breakpoint(name));
        }
        // The calling value, read before the callee's frame is on top.
        let site = if self.profile.is_some() {
            self.call_site()
        } else {
            None
        };
        self.frames.push(Frame {
            body,
            ip: 0,
            exit,
            who,
        });
        // **RFC-0011 §S2, hook 3's open.** A named frame opens its word's
        // node; an unnamed one is an anonymous entry.
        if let Some(p) = self.profile.as_mut() {
            p.open_frame(who, self.frames.len(), site);
            if let Some(w) = who
                && p.tracing()
            {
                p.trace_call(self.registry.interner.name(w));
            }
        }
        Ok(())
    }

    /// Pop frames down to `floor`, running every exit action on the way. This
    /// is what the reference cannot do: its restore is a statement after the
    /// early returns.
    fn unwind_to(&mut self, floor: usize) {
        while self.frames.len() > floor {
            if let Some(Frame {
                exit: Some(action), ..
            }) = self.frames.pop()
            {
                self.run_exit(action);
            }
        }
        self.profile_frames_popped();
    }

    /// Run frames until the stack returns to `floor`.
    ///
    /// The loop is flat: a body that calls a body pushes rather than recurses,
    /// so 100,000 Bund calls cost 100,000 heap frames and one Rust frame.
    /// Cheap guard for both watch hooks: attached, and watching something.
    ///
    /// One branch per push when nothing is attached, which is the same price
    /// §D1's safepoint pays per step and what criterion 11 measured.
    fn watch_hit(&self) -> bool {
        self.debug.as_ref().is_some_and(|d| d.watching_stacks())
    }

    /// Whether a frame about to be pushed for `who` should stop — §D3.
    ///
    /// **The condition runs in the host's child VM**, with a snapshot of the
    /// stack the debuggee is on as its operands, so it can read the program's
    /// state and cannot change it. §D3 calls that the requirement rather than
    /// a refinement, because `request_exit` is `get_or_insert`: a condition
    /// that called `bund.exit` in the program's own VM would end the debugged
    /// program and nothing could clear it.
    fn breakpoint_hit(&mut self, who: Option<Symbol>) -> Option<String> {
        // Two cheap guards before any lookup: nothing attached, or attached
        // and watching no word. The call is counted between them, for §D5's
        // `words`, since every call that reaches a debugger passes here.
        let d = self.debug.as_mut()?;
        if let Some(w) = who {
            d.count_call(w);
        }
        if !d.watching_words() {
            return None;
        }
        let name = self.registry.interner.name(who?).to_string();
        let cond = {
            let d = self.debug.as_ref()?;
            match d.condition_for(&name) {
                None => return None,
                Some(None) => None,
                Some(Some(c)) => Some(c.to_string()),
            }
        };
        let holds = match cond {
            None => true,
            Some(src) => {
                let operands = self.snapshot();
                let mut d = self.debug.take()?;
                let holds = d.condition_holds(&src, &operands);
                self.debug = Some(d);
                holds
            }
        };
        if !holds {
            return None;
        }
        // **D191: counted, and stopped on only at its target if it has one.**
        let hit = self.debug.as_mut()?.count_hit(&name)?;
        Some(format!("{name} (hit {hit})"))
    }

    /// **`catch`: a failure is being raised here, its frames still pushed**
    /// — D191. Called on the error paths before they unwind. One stop per
    /// failure: the flag it raises is cleared by the program's next step.
    fn catch_failure(&mut self, e: &Error) {
        if self.exit_code.is_some() || !self.debug.as_ref().is_some_and(|d| d.catches()) {
            return;
        }
        if let Some(d) = self.debug.as_mut() {
            d.caught = true;
        }
        self.safepoint_for(None, debug::Stop::Error(e.0.clone()));
    }

    /// What a stop compares with the last one (D191): the current stack's
    /// name, depth and top values, and the workbench's depth.
    fn seen(&self) -> debug::Seen {
        debug::Seen::of(
            self.stacks.current_name(),
            &self.snapshot(),
            Vm::workbench_depth(self),
        )
    }

    /// Attach a debugger. **§D1**, and the only writer of `debug`.
    ///
    /// The console is the host's; everything it is handed is text this
    /// interpreter rendered, because `Interp` is not `Send` and the host may
    /// not touch a value.
    pub fn attach_debugger(&mut self, console: Box<dyn debug::Console>) {
        self.debug = Some(Box::new(debug::Debug::new(console)));
        if let Some(p) = self.profile.as_mut() {
            p.console_attached = true;
        }
    }

    /// Whether a debugger is attached, and how many safepoints it stopped at.
    ///
    /// A differential needs this: a stepped run that silently detached would
    /// agree with a plain run for the wrong reason.
    pub fn debugger_stops(&self) -> Option<usize> {
        self.debug.as_ref().map(|d| d.stops)
    }

    /// **One safepoint.** Never inlined: the hot loops carry the branch, and
    /// this carries everything else.
    ///
    /// The cell is **taken out for the duration**, which is what lets an
    /// inspecting command run against `self` — the debuggee examining itself,
    /// as §D1 requires and as the reference's own `debug` loop does. Stepping
    /// commands set where to stop next and return; inspecting commands answer
    /// and leave the debuggee stopped, so `bt` then `stack` is two answers at
    /// one safepoint rather than two steps.
    #[inline(never)]
    fn safepoint(&mut self, top: Option<(usize, &BundValue)>) {
        match self.line_break_hit(top) {
            Some(at) => self.safepoint_for(top, debug::Stop::Breakpoint(at)),
            None => self.safepoint_for(top, debug::Stop::Stepping),
        }
    }

    /// **RFC-0008 §D3 by source location.** The value about to run is the
    /// first of a line a line breakpoint names — its predecessor in the same
    /// body, or in the top-level stream, is on another line or there is none
    /// — so a body that reaches the line stops once each time, at the line's
    /// first value. Nothing is looked up unless a line breakpoint is armed.
    fn line_break_hit(&self, top: Option<(usize, &BundValue)>) -> Option<String> {
        let d = self.debug.as_ref()?;
        if !d.watching_lines() {
            return None;
        }
        let (next, prev) = match self.body_frame() {
            Some(f) => (
                self.place_of(&f.body, f.ip)?,
                f.ip.checked_sub(1).and_then(|i| self.place_of(&f.body, i)),
            ),
            None => {
                let (i, _) = top?;
                (
                    self.stream_place(i)?,
                    i.checked_sub(1).and_then(|i| self.stream_place(i)),
                )
            }
        };
        if prev.is_some_and(|p| p.file == next.file && p.line == next.line) {
            return None;
        }
        d.breaks_at(next.file.as_deref()?, next.line)
    }

    /// Where the top-level stream's value at `i` was written, if the
    /// embedder said ([`Interp::place_stream`]).
    fn stream_place(&self, i: usize) -> Option<bund2_api::diag::Location> {
        let (file, at) = self.stream_places.get(&self.stream_key?)?;
        let &(line, column, start, end) = at.get(i)?;
        Some(bund2_api::diag::Location {
            file: Some(file.to_string()),
            line,
            column,
            excerpt: None,
            span: Some((start, end)),
        })
    }

    /// **Where each value of the next top-level stream was written** — the
    /// stream's own spans, beside the bodies' (RFC-0003 §S5), so a line
    /// breakpoint can stop at the top level too. The embedder that lowered
    /// the stream says it before running it.
    pub fn place_stream(&mut self, label: &str, src: &str, spans: &[(usize, usize)]) {
        self.next_stream = Some(self.stream_place_key(label, src, spans));
    }

    /// A key for this stream's places, the one it was given before if this
    /// label and text were placed already.
    fn stream_place_key(&mut self, label: &str, src: &str, spans: &[(usize, usize)]) -> usize {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        src.hash(&mut h);
        let seen = (Rc::from(label), h.finish());
        if let Some(&key) = self.stream_seen.get(&seen) {
            return key;
        }
        let key = STREAM_KEY | self.stream_places.len();
        self.stream_places
            .insert(key, (Rc::clone(&seen.0), places(src, spans)));
        self.stream_seen.insert(seen, key);
        key
    }

    /// The innermost frame, if it is above the running stream's top level.
    fn body_frame(&self) -> Option<&Frame> {
        self.frames
            .last()
            .filter(|_| self.frames.len() > self.stream_floor)
    }

    /// **The value being applied now, as a site** — RFC-0011 §S8: the
    /// innermost frame's body and the index its `ip` has just moved past, or
    /// the placed top-level stream and its index. Cheap: two loads and no
    /// lookup, so a profiled run can take it at every call.
    fn call_site(&self) -> Option<(usize, usize)> {
        match self.body_frame() {
            Some(f) => Some((f.body.payload_key()?, f.ip.saturating_sub(1))),
            None => Some((self.stream_key?, self.top_at)),
        }
    }

    /// Where a site was written, for a body or a placed stream.
    fn place_of_site(&self, (key, i): (usize, usize)) -> Option<bund2_api::diag::Location> {
        let (file, at) = match self.positions.get(&key) {
            Some(p) if p.body.strong_count() > 0 => (&p.file, &p.at),
            Some(_) => return None,
            None => {
                let (file, at) = self.stream_places.get(&key)?;
                (file, at)
            }
        };
        let &(line, column, start, end) = at.get(i)?;
        Some(bund2_api::diag::Location {
            file: Some(file.to_string()),
            line,
            column,
            excerpt: None,
            span: Some((start, end)),
        })
    }

    /// **Label the profile's sites** — before a record is written, so a
    /// renderer, which has no positions, can say `file:line:col`. A body
    /// dropped since its call cannot be placed and stays unlabelled.
    pub fn label_profile_sites(&mut self) {
        let Some(p) = self.profile.as_ref() else {
            return;
        };
        let labels: Vec<((usize, usize), String)> = p
            .site_keys()
            .into_iter()
            .filter_map(|k| self.place_of_site(k).map(|l| (k, l.to_string())))
            .collect();
        if let Some(p) = self.profile.as_mut() {
            p.site_labels.extend(labels);
        }
    }

    /// Where `name` was registered, if the `register` that bound it was in
    /// placed source.
    pub fn defined_at(&self, name: &str) -> Option<&bund2_api::diag::Location> {
        self.definitions.get(name)
    }

    /// One safepoint, with the reason it was reached.
    ///
    /// **A breakpoint or a watchpoint stops whatever the stepping mode says**:
    /// `continue` sets `Mode::Run`, which `should_stop` answers `false` to, and
    /// a breakpoint that `continue` suppressed would be no breakpoint at all.
    #[inline(never)]
    fn safepoint_for(&mut self, top: Option<(usize, &BundValue)>, why: debug::Stop) {
        let Some(mut d) = self.debug.take() else {
            return;
        };
        // **The program took a step, so a failure it stopped at is over**
        // (D191): a later one is a new failure and may stop.
        if matches!(why, debug::Stop::Stepping) {
            d.caught = false;
        }
        // A line typed at a stop is running: its values are not stops.
        if d.busy {
            self.debug = Some(d);
            return;
        }
        let depth = self.frames.len() + self.nest;
        let forced = !matches!(why, debug::Stop::Stepping | debug::Stop::Term(..));
        if forced || d.should_stop(depth) {
            d.stops += 1;
            // **Where, before what.** Rendered here because only the debuggee
            // can read its own frames; the host receives text.
            let at = match &why {
                debug::Stop::Stepping => self.render_position(top),
                // The string's own position, not the frame's: the frame under
                // a `debug.run` is stopped at the word, whichever term is next.
                debug::Stop::Term(i, next) => {
                    format!("debug.run's string at {i}  next: {next}")
                }
                debug::Stop::Breakpoint(w) => {
                    format!("breakpoint: {w}\n{}", self.render_position(top))
                }
                debug::Stop::Watch(s) => {
                    format!("watchpoint: @{s}\n{}", self.render_position(top))
                }
                debug::Stop::WatchWorkbench => {
                    format!("watchpoint: workbench\n{}", self.render_position(top))
                }
                debug::Stop::WatchDepth(s, n) => format!(
                    "watchpoint: @{s} depth > {n}\n{}",
                    self.render_position(top)
                ),
                debug::Stop::Error(reason) => format!(
                    "caught: {reason}\n{}\n(the failure unwinds when the program moves on)",
                    self.render_position(top)
                ),
                debug::Stop::Redefine(name) => {
                    format!("redefining: {name}\n{}", self.render_position(top))
                }
            };
            // **D191: what moved since the last stop**, beneath where.
            let now = self.seen();
            let at = match d.changes(&now) {
                Some(line) => format!("{at}\n{line}"),
                None => at,
            };
            d.stopped(&at);
            loop {
                match d.next_command(self) {
                    // **The host is gone, so the program is not.** A debugger
                    // that dies must not take the debuggee with it, and the
                    // alternative here is a block that never returns.
                    None => {
                        d.detach();
                        break;
                    }
                    Some(cmd) => match d.apply(cmd, depth) {
                        debug::Next::Resume => break,
                        debug::Next::Render(what) => {
                            let text = match what {
                                debug::Render::Backtrace => self.render_backtrace(),
                                debug::Render::Stack => self.render_debug_stack(),
                                debug::Render::Info => d.render_watched(),
                                debug::Render::Words(filter) => {
                                    self.render_words(filter.as_deref(), Some(&d))
                                }
                                debug::Render::Classes => self.render_classes(),
                            };
                            d.answer(&text);
                        }
                        debug::Next::Eval(source) => {
                            // **Put back for the line, so the line can see
                            // what is armed** — `debug.info` typed at a stop
                            // reads it — and marked busy so nothing in the
                            // line is itself a stop.
                            d.busy = true;
                            self.debug = Some(d);
                            let said = self.evaluate_typed(&source);
                            let Some(mut back) = self.debug.take() else {
                                // Nothing detaches a debugger today. If the
                                // line did, there is no host left to ask.
                                return;
                            };
                            back.busy = false;
                            let moved = std::mem::take(&mut back.resume);
                            d = back;
                            if let Some(text) = said {
                                d.answer(&text);
                            }
                            // **A line that exits has ended the program**, and
                            // the request cannot be withdrawn (§D3). Asking for
                            // another command would be asking about a program
                            // that will run nothing more.
                            // Or a moving word was in the line (§W5), which
                            // has said where to stop next.
                            if self.exit_code.is_some() || moved {
                                break;
                            }
                        }
                    },
                }
            }
            // What the next stop compares with: the state the program
            // resumes in, a typed line's changes included.
            d.leave(self.seen());
        }
        self.debug = Some(d);
    }

    /// **Run a line typed at a stop, in this VM — RFC-0008 §W2.**
    ///
    /// Answers what the host should be told, if anything: the line's failure,
    /// or that no evaluator was installed. A line that fails does not move the
    /// program, as in `debug.shell`.
    ///
    /// **The tail request is set aside for the line and put back after it.**
    /// A stop can fall inside a native — a watch fires in the native's own
    /// push — and that native may have filed a body to run when it returns.
    /// The line's first word would otherwise take that body as its own, or a
    /// failing native in the line would discard it (F96's clear). Set aside,
    /// the line can do neither.
    fn evaluate_typed(&mut self, source: &str) -> Option<String> {
        let Some(run) = self.evaluator else {
            return Some("bund2: this host cannot run a line of Bund at a stop\n".to_string());
        };
        let held = self.pending_tail.take();
        let held_who = self.pending_who.take();
        self.cells.set_request(false);
        let out = run(self, source);
        self.pending_tail = held;
        self.pending_who = held_who;
        self.cells.set_request(self.pending_tail.is_some());
        match out {
            Ok(()) => None,
            // The exit's own refusal is not a failure of the line (D52).
            Err(_) if self.exit_code.is_some() => None,
            Err(e) => Some(format!("bund2: {}\n", e.0)),
        }
    }

    /// **The slot table as a view — RFC-0008 §D5.** One line a word: its
    /// name, what is bound in its slot, its generation, and its calls since
    /// the debugger attached.
    ///
    /// **Every binding in the slot, not the one dispatch takes**, so a name
    /// bound twice says so: `swap` reads `alias → swap_one, native` (F20).
    /// Without a filter the program's own lambdas and whatever was called
    /// are shown, since the full table is some six hundred lines; `all`
    /// shows every bound slot, and any other text the names containing it.
    ///
    /// The tier is said once, above the table: under a debugger it is off
    /// (§D6), so a column would say the same thing on every line.
    fn render_words(&self, filter: Option<&str>, d: Option<&debug::Debug>) -> String {
        let calls = |s: Symbol| d.map_or(0, |d| d.calls_to(s));
        let mut out = String::from(match (&self.tier, self.tier_off) {
            (None, _) => "no tier in this build: every body is interpreted\n",
            (Some(_), true) => {
                "the tier is off under the debugger (§D6): every body is interpreted\n"
            }
            (Some(_), false) => "the tier is on: a hot body may run compiled\n",
        });
        if d.is_none() {
            out.push_str("no debugger is attached, so no calls are counted\n");
        }
        let mut rows = Vec::new();
        for (i, name) in self.registry.interner.names().iter().enumerate() {
            let Some((sym, _)) = self.registry.interner.lookup_call(name) else {
                continue;
            };
            if sym.index() != i {
                continue;
            }
            let Some(slot) = self.registry.slot(sym) else {
                continue;
            };
            let mut kinds = Vec::new();
            if let Some(a) = slot.alias {
                kinds.push(format!("alias → {}", self.registry.interner.name(a)));
            }
            if slot.lambda.is_some() {
                kinds.push("lambda".to_string());
            }
            if slot.native.is_some() {
                kinds.push("native".to_string());
            }
            if slot.command.is_some() {
                kinds.push("command".to_string());
            }
            if kinds.is_empty() {
                continue;
            }
            let n = calls(sym);
            let shown = match filter {
                Some("all") => true,
                Some(text) => name.contains(text),
                None => slot.lambda.is_some() || n > 0,
            };
            if shown {
                // RFC-0011 §S8: where a lambda came from, when it was
                // registered in placed source.
                let mut kinds = kinds.join(", ");
                if slot.lambda.is_some()
                    && let Some(at) = self.definitions.get(name)
                {
                    kinds.push_str(&format!(", registered at {at}"));
                }
                rows.push((name.as_str(), kinds, slot.generation(), n));
            }
        }
        if rows.is_empty() {
            out.push_str("no word matches; `words all` lists every slot\n");
            return out;
        }
        rows.sort_by(|a, b| a.0.cmp(b.0));
        let width = rows.iter().map(|r| r.0.chars().count()).max().unwrap_or(0);
        for (name, kinds, generation, n) in rows {
            out.push_str(&format!(
                "  {name:<width$}  {kinds}  gen {generation}  calls {n}\n"
            ));
        }
        out
    }

    /// **The registered classes — RFC-0008 §D5.** Each with the parents its
    /// `.super` names, then its members split as the object system splits
    /// them: a LAMBDA is a method, and so is a PTR, which is how a built-in
    /// class names a native method (`.format`); anything else is a field.
    fn render_classes(&self) -> String {
        let mut names = self.registry.class_names();
        if names.is_empty() {
            return "no class is registered\n".to_string();
        }
        names.sort();
        let list = |v: &[String]| {
            if v.is_empty() {
                "none".to_string()
            } else {
                v.join(" ")
            }
        };
        let mut out = String::new();
        for name in names {
            let Some(class) = self.registry.class(&name) else {
                continue;
            };
            let (mut supers, mut methods, mut fields) = (Vec::new(), Vec::new(), Vec::new());
            if let Some(map) = class.as_map() {
                for (k, v) in map {
                    match k.as_str() {
                        ".super" => {
                            supers = v
                                .as_list()
                                .unwrap_or_default()
                                .iter()
                                .map(|p| p.as_str().unwrap_or_else(|| p.summary(24)))
                                .collect();
                        }
                        ".class_name" => {}
                        _ if matches!(v.dt(), bund2_value::LAMBDA | bund2_value::PTR) => {
                            methods.push(k.clone())
                        }
                        _ => fields.push(k.clone()),
                    }
                }
            }
            out.push_str(&format!(
                "{name}\n  super: {}\n  methods: {}\n  fields: {}\n",
                list(&supers),
                list(&methods),
                list(&fields)
            ));
        }
        out
    }

    /// Every stack by name, with its depth and its top value — §W1's
    /// `debug.stacks`. The current one is marked.
    fn render_stacks(&self) -> String {
        let current = self.stacks.current_name().to_string();
        let mut out = String::new();
        for name in Vm::stack_names(self) {
            let mark = if name == current { '*' } else { ' ' };
            let items = Vm::snapshot_of(self, &name);
            let top = items
                .last()
                .map(|v| v.summary(48))
                .unwrap_or_else(|| "empty".to_string());
            out.push_str(&format!("{mark} @{name}  {}  top: {top}\n", items.len()));
        }
        out.push_str(&format!("  workbench  {}\n", Vm::workbench_depth(self)));
        out
    }

    /// One line saying where the debuggee is stopped — the innermost frame, or
    /// the top-level stream when no frame is running.
    ///
    /// **The value about to run, not the one just run.** `ip` has already been
    /// advanced past a value by the time the next safepoint is reached, so the
    /// frame's `ip` is where execution resumes, and that is what a stepping
    /// host needs to see.
    fn render_position(&self, top: Option<(usize, &BundValue)>) -> String {
        match self.body_frame() {
            Some(f) => {
                let who = f.who.map(|s| self.registry.interner.name(s));
                let by = self.runner_of(self.frames.len() - 1);
                let next = frame_items(&f.body)
                    .and_then(|items| items.get(f.ip))
                    .map(|v| v.summary(48))
                    .unwrap_or_else(|| "the end of the body".to_string());
                // `#0`: the innermost frame is `bt`'s first line and the
                // traceback's (D173), so a stop and the backtrace typed at it
                // name it alike (F217). It was the outermost frame's index.
                format!(
                    "{}  next: {next}{}",
                    debug::frame_line(&f.body, f.ip, 0, who, by.as_deref()),
                    self.place_suffix(&f.body, f.ip)
                )
            }
            // **The index is the stream's, so it is a position a caller
            // holding `Lowered` can turn into a source span** — the whole of
            // RFC-0003 §S5 a flat stream supports, and the same index
            // `eval_indexed` hands back on a failure.
            None => match top {
                Some((i, v)) => {
                    let at = self
                        .stream_place(i)
                        .map(|l| format!("  ({l})"))
                        .unwrap_or_default();
                    format!(
                        "#0  the top-level stream at {i}  next: {}{at}",
                        v.summary(48)
                    )
                }
                None => "#0  the top-level stream".to_string(),
            },
        }
    }

    /// The frame stack, innermost first.
    ///
    /// **What it cannot say yet is the point.** A frame knows the body it runs
    /// and not the symbol it was pushed for, so a line reads `a lambda at 3`;
    /// §D2's field is what makes it a name, and a source position needs the
    /// per-value spans RFC-0003 §S5 describes.
    fn render_backtrace(&self) -> String {
        // **The streams interleave with the frames.** A file `use` ran is a
        // stream whose top level sits at the depth `use` was called at, so
        // its line goes below the frames it entered and above the frame that
        // ran `use` (RFC-0003 §S5).
        let mut levels = vec![(self.stream_key, self.top_at, self.stream_floor)];
        levels.extend(self.stream_outer.iter().rev().copied());
        let mut levels = levels.into_iter().peekable();
        let mut out = String::new();
        let mut n = 0;
        let stream_line = |n: usize, (key, at, _): (Option<usize>, usize, usize)| match key {
            Some(k) => {
                let place = self
                    .place_of_site((k, at))
                    .map(|l| format!("  ({l})"))
                    .unwrap_or_default();
                format!("#{n}  the top-level stream at {at}{place}\n")
            }
            None => format!("#{n}  the top-level stream\n"),
        };
        for (i, f) in self.frames.iter().enumerate().rev() {
            while let Some(l) = levels.next_if(|l| i < l.2) {
                out.push_str(&stream_line(n, l));
                n += 1;
            }
            let who = f.who.map(|s| self.registry.interner.name(s));
            out.push_str(&debug::frame_line(
                &f.body,
                f.ip,
                n,
                who,
                self.runner_of(i).as_deref(),
            ));
            out.push_str(&self.place_suffix(&f.body, f.ip.saturating_sub(1)));
            out.push('\n');
            n += 1;
        }
        // The program's own stream, and any whose floor was the bottom.
        if n == 0 || self.nest > 0 {
            for l in levels {
                out.push_str(&stream_line(n, l));
                n += 1;
            }
        }
        out
    }

    /// The current stack, innermost value last, through `summary` — **D36**,
    /// which bounds what a report may print. The unbounded `Debug` form
    /// belongs to `debug.display_stack` and `--raw-values`.
    fn render_debug_stack(&self) -> String {
        let mut out = format!("@{}\n", self.stacks.current_name());
        for v in self.snapshot() {
            out.push_str("  ");
            out.push_str(&v.summary(72));
            out.push('\n');
        }
        out
    }

    fn run_to(&mut self, floor: usize) -> Result<(), Error> {
        while self.frames.len() > floor {
            // **§D1's safepoint.** One branch per step when nothing is
            // attached; criterion 11 measures what that costs rather than
            // calling it free. The work is behind `#[inline(never)]` so the
            // hot path holds the test and not the body.
            if self.debug.is_some() {
                // The innermost frame knows what is next, so nothing is passed.
                self.safepoint(None);
            }
            let Some(frame) = self.frames.last_mut() else {
                break;
            };
            // Read through the held value (D42). `get`, not an index: a frame
            // whose value carries no body is a broken invariant, not a panic.
            let next = frame_items(&frame.body).map(|items| items.get(frame.ip).cloned());
            let v = match next {
                Some(Some(v)) => {
                    frame.ip += 1;
                    v
                }
                Some(None) => {
                    let done = self.frames.pop();
                    self.profile_frames_popped();
                    if let Some(Frame {
                        exit: Some(action), ..
                    }) = done
                    {
                        self.run_exit(action);
                    }
                    continue;
                }
                None => {
                    self.unwind_to(floor);
                    return Err(Error::internal(
                        "a frame's body is neither a LAMBDA nor a LIST value",
                    ));
                }
            };
            // **D188: one step, counted.** One branch on the off path, the
            // same price as the safepoint above; on the on path one increment
            // and no clock, which is what makes the count the same on every
            // host.
            if let Some(p) = self.profile.as_mut() {
                p.step();
            }
            match self.apply_step(v).and_then(|()| self.take_pending()) {
                // `take_pending` can now fail: it reaches `push_frame`, where a
                // tier may run the body and answer an error. That failure takes
                // the same path an interpreted one does — unwind to the floor,
                // running every exit action — or frames would leak above it.
                Ok(()) => {}
                Err(e) => {
                    // D191: before the frames go, so a `catch` stop sees them.
                    if self.debug.is_some() {
                        self.catch_failure(&e);
                    }
                    self.note_failure(&e);
                    // The word whose body raised, for the report's `ended`
                    // line; a native that answered the error marked itself
                    // first, and the first mark is kept.
                    if let Some(p) = self.profile.as_mut() {
                        p.mark_failure(&e.0);
                    }
                    self.unwind_to(floor);
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn run_exit(&mut self, action: ExitAction) {
        match action {
            ExitAction::ToStack(name) => self.to_stack(&name),
        }
    }

    /// Keep the frames a failure was raised under, before they unwind —
    /// D173. The innermost failure wins: an outer `run_to` sees the trace
    /// already taken and leaves it. Nothing is rendered unless the reporter
    /// asked, so a program catching errors in a loop pays one branch.
    ///
    /// Bounded: the innermost [`Self::TRACE_INNER`] and outermost
    /// [`Self::TRACE_OUTER`] frames, with one line counting the rest, so a
    /// runaway recursion reports in a screen and not in a hundred thousand
    /// lines.
    fn note_failure(&mut self, e: &Error) {
        if self.failure_trace.is_some() || !self.reporter.wants_frames() {
            return;
        }
        self.failure_trace = Some(FailureTrace {
            cause: e.0.clone(),
            frames: self.trace_frames(),
            source: None,
            inner: self.innermost_place(),
        });
    }

    /// Where the item at `i` of `body` was written, if anywhere.
    fn place_of(&self, body: &BundValue, i: usize) -> Option<bund2_api::diag::Location> {
        let p = self.positions.get(&body.payload_key()?)?;
        p.body.upgrade()?;
        let &(line, column, start, end) = p.at.get(i)?;
        Some(bund2_api::diag::Location {
            file: Some(p.file.to_string()),
            line,
            column,
            excerpt: None,
            span: Some((start, end)),
        })
    }

    /// The value each running frame is in the middle of — its `ip` has moved
    /// past the value being applied — placed, innermost first; the first
    /// frame with a place answers.
    fn innermost_place(&self) -> Option<bund2_api::diag::Location> {
        self.frames
            .iter()
            .rev()
            .find_map(|f| self.place_of(&f.body, f.ip.saturating_sub(1)))
    }

    /// `"  (p.bund:3:7)"`, or nothing for a body that was not written.
    fn place_suffix(&self, body: &BundValue, i: usize) -> String {
        self.place_of(body, i)
            .map(|l| format!("  ({l})"))
            .unwrap_or_default()
    }

    /// Dead entries are swept when the table reaches this size, and the mark
    /// then moves to twice what survived.
    const POSITIONS_SWEEP: usize = 4096;

    /// The running frames as the traceback's lines, innermost first.
    fn trace_frames(&self) -> Vec<String> {
        let n = self.frames.len();
        let mut lines = Vec::new();
        for (i, f) in self.frames.iter().enumerate().rev() {
            let depth = n - 1 - i;
            if depth >= Self::TRACE_INNER && i >= Self::TRACE_OUTER {
                if depth == Self::TRACE_INNER {
                    lines.push(format!(
                        "…   {} more frames",
                        n - Self::TRACE_INNER - Self::TRACE_OUTER
                    ));
                }
                continue;
            }
            let who = f.who.map(|s| self.registry.interner.name(s));
            lines.push(
                debug::frame_line(&f.body, f.ip, depth, who, self.runner_of(i).as_deref())
                    + &self.place_suffix(&f.body, f.ip.saturating_sub(1)),
            );
        }
        lines
    }

    /// **The word a program writes to make a conditional of this type** —
    /// `?try` for `tryexcept` — read from the effect each word declares
    /// (D181's `Leaves::Conditional`), so a frame names what the program said
    /// rather than the runner's registry key. A word with a native of its own
    /// is preferred to an alias of it, and then the first by name, so the
    /// answer does not depend on registration order. Asked only while a
    /// trace is rendered.
    fn conditional_maker(&self, ty: &str) -> Option<String> {
        let mut makers: Vec<(bool, &str)> = self
            .registry
            .interner
            .names()
            .iter()
            .filter(|n| {
                self.registry.effect_of(n).is_some_and(
                    |e| matches!(e.leaves, bund2_api::Leaves::Conditional(t) if t == ty),
                )
            })
            .map(|n| {
                let own = self
                    .registry
                    .interner
                    .lookup_call(n)
                    .and_then(|(s, _)| self.registry.slot(s))
                    .is_some_and(|slot| slot.native.is_some() && slot.alias.is_none());
                (!own, n.as_str())
            })
            .collect();
        makers.sort();
        makers.first().map(|(_, n)| (*n).to_string())
    }

    /// What started the body at frame `i`, when a runner handed it to
    /// `eval_lambda` while naming was on.
    fn runner_of(&self, i: usize) -> Option<String> {
        let (_, r) = self.runners.iter().rev().find(|(at, _)| *at == i)?;
        Some(match r {
            Runner::Word(s) => self.registry.interner.name(*s).to_string(),
            Runner::Conditional(ty) => self
                .conditional_maker(ty)
                .unwrap_or_else(|| format!("the {ty} conditional")),
            Runner::Method(m) => format!("method {m}"),
        })
    }

    const TRACE_INNER: usize = 24;
    const TRACE_OUTER: usize = 8;

    /// Drive any frame the top level's last word requested.
    fn drain_frames(&mut self) -> Result<(), Error> {
        self.take_pending()?;
        self.run_to(0)
    }
}

/// **A call to a name nothing binds, in the reference's words — D197.**
///
/// Every unknown call there ends in `VM::i`, which reaches `i_direct` and
/// the stack layer's `get_inline`: `Inline <name> not registered`
/// (`reference/rust_multistack/src/ts_inline.rs:39`), wrapped as
/// `i(<name>) for stack returned: ` (`reference/rust_multistackvm/src/multistackvm_inline.rs:62-64`).
/// A plain call, `$name` and `execute` on a string all arrive there
/// (`multistackvm_apply.rs:39-59`, `multistackvm_call_internal_word.rs:6-8`,
/// `multistackvm_call.rs:7-8`), with the name after alias resolution and
/// without its `$`. The text is the program's to see: `?try` files it in
/// `context`.
fn unregistered(name: &str) -> Error {
    let name = name.strip_prefix('$').unwrap_or(name);
    Error(format!(
        "i({name}) for stack returned: Inline {name} not registered"
    ))
}

/// What handed a body to `eval_lambda` — D173's name for a lambda no word
/// names.
#[derive(Clone)]
enum Runner {
    Word(Symbol),
    Conditional(String),
    Method(String),
}

/// The traceback taken for one failure — D173.
struct FailureTrace {
    /// The innermost failure's own text, before anything wrapped it.
    cause: String,
    /// The running frames, innermost first.
    frames: Vec<String>,
    /// Where in an evaluated string it failed: `(label, text, span)`.
    source: Option<(String, String, (usize, usize))>,
    /// Where inside a body it failed, if that body was written somewhere
    /// (RFC-0003 §S5): the innermost frame with a place for its value.
    inner: Option<bund2_api::diag::Location>,
}

impl FailureTrace {
    /// Where in the evaluated string it failed, as a location naming the
    /// string, and the string as `(name, text)`.
    fn in_source(&self) -> Option<(bund2_api::diag::Location, (String, String))> {
        let (label, src, (start, end)) = self.source.as_ref()?;
        let start = (*start).min(src.len());
        let before = src.get(..start).unwrap_or_default();
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        Some((
            bund2_api::diag::Location {
                file: Some(label.clone()),
                line: before.matches('\n').count() + 1,
                column: before.get(line_start..).unwrap_or_default().chars().count() + 1,
                excerpt: None,
                span: Some((start, (*end).clamp(start, src.len()))),
            },
            (label.clone(), src.clone()),
        ))
    }

    /// **For a fatal report**: the frames, and the place inside — a body's,
    /// or else the evaluated string's, with its text. The location stays the
    /// top-level step the caller placed, which is what the table shows.
    fn attach_fatal(self, d: bund2_api::diag::Diagnostic) -> bund2_api::diag::Diagnostic {
        let in_source = self.in_source();
        let mut d = d.with_frames(self.frames);
        // The traceback leads with the failure's own text; the reason keeps
        // the reference's wrappers for the table that shows it.
        d.cause = Some(self.cause);
        d.inner = self.inner;
        if let Some((at, text)) = in_source {
            let same_file = d.inner.as_ref().is_some_and(|i| i.file == at.file);
            if d.inner.is_none() || same_file {
                d.inner = d.inner.or(Some(at));
            }
            d.source = Some(text);
        }
        d
    }

    fn attach(self, d: bund2_api::diag::Diagnostic) -> bund2_api::diag::Diagnostic {
        let in_source = self.in_source();
        let mut d = d.with_frames(self.frames);
        d.cause = Some(self.cause);
        d.inner = self.inner;
        if let Some((at, text)) = in_source {
            d.location = Some(at);
            d.source = Some(text);
        } else if d.location.is_none() {
            // Caught with no string to point into: the place in the body is
            // the only place there is.
            d.location = d.inner.take();
        }
        d
    }
}

/// One placed value: `(line, column, start, end)`, the first two 1-based,
/// the last two bytes.
type Place = (usize, usize, usize, usize);

/// The top bit, marking a key as a top-level stream's rather than a body's
/// `payload_key`, which is an address and never has it set.
const STREAM_KEY: usize = 1 << (usize::BITS - 1);

/// `(line, column, start, end)` for each span of `src`, line and column
/// 1-based. Line starts are found once, so each place is a search.
fn places(src: &str, spans: &[(usize, usize)]) -> Vec<(usize, usize, usize, usize)> {
    let starts: Vec<usize> = std::iter::once(0)
        .chain(src.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    spans
        .iter()
        .map(|&(start, end)| {
            let start = start.min(src.len());
            let line = starts.partition_point(|&s| s <= start);
            let line_start = starts.get(line.saturating_sub(1)).copied().unwrap_or(0);
            let column = src
                .get(line_start..start)
                .map_or(1, |t| t.chars().count() + 1);
            (line, column, start, end)
        })
        .collect()
}

/// Where a literal body's items were written — RFC-0003 §S5.
struct Placed {
    /// The body, without keeping it alive.
    body: std::rc::Weak<bund2_value::Payload>,
    /// The source's name, shared by every body it holds.
    file: Rc<str>,
    /// One place per item: `(line, column, start, end)`, the first two
    /// 1-based, the last two bytes.
    at: Vec<(usize, usize, usize, usize)>,
}

impl Vm for Interp {
    fn autoadd(&self) -> bool {
        self.autoadd
    }

    /// Delegates to the inherent [`Interp::set_autoadd`], which is the only
    /// writer and which also updates §S6's mirror cell.
    fn set_autoadd(&mut self, on: bool) {
        Interp::set_autoadd(self, on);
    }

    fn push(&mut self, v: BundValue) {
        // **§D4's first hook — criterion 9.** The stack push writes the stack
        // tag (D41), so a `@name`-keyed watch belongs here. Before the push,
        // so the stop shows the stack as it was and the value as "next".
        if self.watch_hit() {
            let name = self.stacks.current_name().to_string();
            let cond = self
                .debug
                .as_ref()
                .and_then(|d| d.watch_condition(&name))
                .map(|c| c.map(str::to_string));
            let stop = match cond {
                None => false,
                Some(None) => true,
                // **D191: the condition sees the value about to land**, on
                // top of the stack it lands on.
                Some(Some(src)) => {
                    let mut operands = self.snapshot();
                    operands.push(v.clone());
                    match self.debug.take() {
                        Some(mut d) => {
                            let holds = d.condition_holds(&src, &operands);
                            self.debug = Some(d);
                            holds
                        }
                        None => false,
                    }
                }
            };
            if stop {
                self.safepoint_for(None, debug::Stop::Watch(name));
            } else if let Some(n) = self.debug.as_ref().and_then(|d| d.watched_depth())
                && self.stacks.current().len() == n
            {
                // **§D4's `watch depth > N`, on the crossing.** Exactly `n`
                // before this push, so a stack that stays deep stops once,
                // and stops again only after coming back down to `n`.
                self.safepoint_for(None, debug::Stop::WatchDepth(name, n));
            }
        }
        self.stacks.current_mut().push(v);
    }

    fn pull(&mut self) -> Option<BundValue> {
        self.stacks.current_mut().pull()
    }

    fn depth(&self) -> usize {
        // Not through `depth_of`: a depth guard reads only as far as its
        // operands, which D55's differential checks, and must not be taken
        // for a read of a stack by name.
        self.stacks.current().len()
    }

    fn peek(&self) -> Option<BundValue> {
        self.stacks.current().peek().cloned()
    }

    fn peek_at(&self, n: usize) -> Option<BundValue> {
        self.stacks.current().peek_at(n).cloned()
    }

    fn clear(&mut self) {
        let name = self.stacks.current_name().to_string();
        self.clear_stack(&name);
    }

    fn snapshot(&self) -> Vec<BundValue> {
        self.note_observation("reads the whole current stack");
        self.stacks.current().contents()
    }

    fn snapshot_workbench(&self) -> Vec<BundValue> {
        self.note_observation("reads the whole workbench");
        self.stacks.workbench.contents()
    }

    fn rotate_left(&mut self) {
        self.stacks.current_mut().rotate_left();
    }

    fn rotate_right(&mut self) {
        self.stacks.current_mut().rotate_right();
    }

    fn current_name(&self) -> String {
        // Not observed: the name is not a value promotion holds, so asking for
        // it reads nothing a compiled caller might be keeping in registers. A
        // word that then *acts* on the current stack by name changes what it
        // answers when the values beneath change, which D55's differential
        // catches (F111 is what it caught).
        self.stacks.current_name().to_string()
    }

    fn to_stack(&mut self, name: &str) {
        self.stacks.to_stack(name, &self.cells);
    }

    fn stack_exists(&self, name: &str) -> bool {
        self.stacks.has(name)
    }

    /// **A stack this creates becomes current — F89.** The reference creates
    /// through `add_named_stack`, which appends to the back of its deque
    /// (`reference/rust_multistack/src/ts_add.rs:14`), and its current stack
    /// *is* the back (`reference/rust_multistack/src/ts_current.rs:7`). A new
    /// name goes in through `Stacks::add_as_current`, as `Stacks::to_stack`
    /// puts one. A name that already exists is left where it is, as the
    /// reference leaves it (`reference/rust_multistack/src/ts_ensure.rs:10-15`).
    /// Every named push creates through here, as the reference's
    /// `push_to_stack` does (`reference/rust_multistack/src/ts_push.rs:46`),
    /// and F70 is the same mechanism seen through `move`.
    fn ensure_stack(&mut self, name: &str) {
        self.stacks.create(name);
        if !self.stacks.in_ring(name) {
            self.stacks.add_as_current(Rc::from(name), &self.cells);
        }
    }

    fn ensure_stack_with_capacity(&mut self, name: &str, cap: usize) {
        self.ensure_stack(name);
        // The reference only records a capacity the first time
        // (`reference/rust_multistack/src/ts_ensure.rs:20-22` inserts into
        // `stack_cap` only when absent), so a second call does not resize.
        if let Some(s) = self.stacks.get_mut(name)
            && s.cap.is_none()
        {
            s.cap = Some(cap);
        }
    }

    fn depth_of(&self, name: &str) -> usize {
        self.note_observation("reads the depth of a stack by name");
        self.stacks.get(name).map(Stack::len).unwrap_or(0)
    }

    /// **In rotation order, not alphabetical.** `order` is the stack-of-stacks
    /// as the reference keeps it, and `save.stacks` writing them in a
    /// different order would reorder a world file's `STACKS` table for no
    /// reason a program could predict.
    fn stack_names(&self) -> Vec<String> {
        self.note_observation("enumerates the stacks");
        self.stacks.ring().map(str::to_string).collect()
    }

    fn snapshot_of(&self, name: &str) -> Vec<BundValue> {
        self.note_observation("reads a stack by name");
        // `contents` is what `snapshot` uses, so the two orders agree.
        self.stacks
            .get(name)
            .map(Stack::contents)
            .unwrap_or_default()
    }

    fn lambda_names(&self) -> Vec<String> {
        self.registry.lambda_names()
    }

    fn alias_pairs(&self) -> Vec<(String, String)> {
        self.registry.alias_pairs()
    }

    fn push_to(&mut self, name: &str, v: BundValue) {
        self.ensure_stack(name);
        if let Some(s) = self.stacks.get_mut(name) {
            s.push(v);
        }
    }

    fn pull_from(&mut self, name: &str) -> Option<BundValue> {
        self.stacks.get_mut(name).and_then(Stack::pull)
    }

    fn clear_stack(&mut self, name: &str) {
        if let Some(s) = self.stacks.get_mut(name) {
            s.items.clear();
        }
    }

    fn drop_stack(&mut self, name: &str) {
        self.stacks.drop_named(name, &self.cells);
    }

    fn rotate_stacks_left(&mut self) {
        self.stacks.rotate_left(&self.cells);
    }

    fn rotate_stacks_right(&mut self) {
        self.stacks.rotate_right(&self.cells);
    }

    fn apply(&mut self, v: BundValue) -> Result<(), Error> {
        Interp::apply(self, v)
    }

    /// Run a body **now**, for a native that must inspect the stack after it.
    ///
    /// Synchronous, but flat inside: it pushes a frame and drives the loop back
    /// down to its own floor, so however deeply the body recurses it costs one
    /// Rust frame — this one. `?ifthenelse` and `?try` need this shape because
    /// they act on what the body left behind; `if` does not, and uses
    /// [`Interp::request_tail`] instead.
    fn tail_lambda(&mut self, lambda: BundValue) {
        if let Some(p) = self.profile.as_mut() {
            p.tail_filed();
        }
        Interp::request_tail(self, lambda);
    }

    /// D56. One of the four places `pending_tail` is written, and the only one
    /// reachable from outside this crate: RFC-0005's adapter calls a native
    /// directly, so it cannot go through [`Interp::invoke`]'s clear (F96).
    fn clear_tail_request(&mut self) {
        self.pending_tail = None;
        // D56's clear is the one a compiled call site reaches, so the mirror
        // matters most here: it is the path that has no `invoke` behind it.
        self.cells.set_request(false);
    }

    /// See [`bund2_api::Vm::drain_tail_request`] for the contract.
    ///
    /// This is `Interp::apply`'s tail, without the value and without the gate:
    /// the floor check, the recorded frame count, `take_pending`, the unwind on
    /// error, and `run_to`. Compiled code has no `apply_step` to precede it,
    /// which is the whole reason it is a separate entry point.
    ///
    /// **Writes no `pending_tail` of its own** — `take_pending` and
    /// `clear_tail_request` do, and both are already in RFC-0005 assumption
    /// 33's named set, so the mirror stays paired without widening it.
    /// `Interp::apply` without the drain: the Tier 0 floor, the step, and
    /// the exit gate. What the value filed is left for the entry to take.
    fn apply_filing(&mut self, v: BundValue) -> Result<(), Error> {
        if !self.stack_ok() {
            return Err(Error::stack_exhausted());
        }
        self.apply_step(v)?;
        self.exit_gate()
    }

    fn drain_tail_request(&mut self) -> Result<(), Error> {
        if self.pending_tail.is_none() {
            return Ok(());
        }
        // Running a body here spends machine stack, as `eval_lambda` does.
        // §S5: a refused drain clears the request before it returns the error,
        // so nothing stale outlives it (F96's rule, at this boundary).
        if !self.stack_ok() {
            Vm::clear_tail_request(self);
            return Err(Error::stack_exhausted());
        }
        let floor = self.frames.len();
        // As `Interp::apply` does: a tier may run the body and answer an error
        // from inside `push_frame`, so unwind to the floor this call recorded
        // rather than leaving frames above it for the next caller.
        if let Err(e) = self.take_pending() {
            self.unwind_to(floor);
            return Err(e);
        }
        self.run_to(floor)
    }

    fn scoped_call(&mut self, stack: &str, body: Vec<BundValue>) -> Result<(), Error> {
        if !self.stack_ok() {
            return Err(Error::stack_exhausted());
        }
        let prev = self.stacks.current_name().to_string();
        let floor = self.frames.len();
        self.to_stack(stack);
        // The exit action is carried by the frame, so the unwinder runs it on a
        // failure just as the loop runs it on success. F57. The body is
        // assembled per call, so it is wrapped as a LIST value: there is no
        // key worth keeping (D42).
        // Carries an exit action, so the tier is never offered it — see
        // `push_frame`. The LIST does have a key, a new one on every call.
        // A `( … )` context is nobody's word, so the frame carries no symbol.
        //
        // **F231.** The frame can be refused before it exists — D171's poll is
        // `push_frame_once`'s first act — and then no exit action runs. Unwind
        // whatever was pushed, and come back by hand if nothing did.
        let pushed = self.push_frame(
            BundValue::list(body),
            Some(ExitAction::ToStack(prev.clone())),
            None,
        );
        if let Err(e) = pushed {
            self.unwind_to(floor);
            if self.stacks.current_name() != prev {
                self.to_stack(&prev);
            }
            return Err(e);
        }
        self.run_to(floor)?;
        // F112, as in `Interp::apply`.
        self.exit_gate()
    }

    fn eval_lambda(&mut self, lambda: &BundValue) -> Result<(), Error> {
        self.exit_gate()?;
        if frame_items(lambda).is_none() {
            return Err(Error::internal(
                "eval_lambda was handed a value that carries no body; every caller checks the LAMBDA tag first",
            ));
        }
        // **F85's fix.** Every native that runs a body synchronously comes
        // through here, and each such level spends Rust stack. Below the floor,
        // refuse rather than nest until the process aborts (RFC-0005 §S8).
        if !self.stack_ok() {
            return Err(Error::stack_exhausted());
        }
        let floor = self.frames.len();
        // If a tier ran the body, no frame was pushed and `run_to` below finds
        // nothing to do — the body having run without a frame is exactly what
        // the seam's contract promises.
        // A lambda a native handed us: the native is on the Rust stack, and the
        // body it is running belongs to no word.
        //
        // **F189: its failure is wrapped like any other.** `push_frame`
        // answers `Err` when a tier ran the body and it failed (or, since
        // D171, when the OOM guard tripped), and that
        // error used to leave through `?`, past the `map_err` below — so a
        // hot body failing under `map` or `?try` lost the wrapper an
        // interpreted one carries, and `?try`'s `context` slot differed by
        // tier.
        // **Named by what started it, while naming (D173).** The runner is
        // taken for the body's run, so a native that compiled code calls
        // directly, past `call_native`, finds none rather than this one.
        let by = if self.naming {
            self.running.take()
        } else {
            None
        };
        if let Some(r) = &by {
            self.runners.push((floor, r.clone()));
        }
        let started = self.push_frame(lambda.clone(), None, None);
        let done = started
            .and_then(|()| self.run_to(floor))
            .and_then(|()| self.exit_gate());
        if by.is_some() {
            self.runners.pop();
            self.running = by;
        }
        // **F112.** `run_to` pops a finished frame without the gate, so a body
        // whose last word is `bund.exit` returns `Ok`, and the native that ran
        // it went on: `map` collected, `input*` read another line. D52 says
        // nothing more runs, so the refusal comes here, at the return to Rust,
        // as it would have at the body's next step.
        done.map_err(|e| e.context("Lambda content evaluation returned error: "))
    }

    fn register_lambda(&mut self, name: &str, body: BundValue) {
        // **D191: `watch redefine`, before the old body is replaced.**
        if self
            .debug
            .as_ref()
            .is_some_and(|d| d.watches_redefinition())
            && (Vm::is_lambda(self, name) || Vm::is_native(self, name))
        {
            self.safepoint_for(None, debug::Stop::Redefine(name.to_string()));
        }
        if self.profile.is_some()
            && (Vm::is_lambda(self, name) || Vm::is_native(self, name))
            && let Some(p) = self.profile.as_mut()
        {
            p.redefined(name);
        }
        // RFC-0011 §S8: where this `register` is, for `words` and the profile.
        if let Some(at) = self.call_site().and_then(|s| self.place_of_site(s)) {
            if let Some(p) = self.profile.as_mut() {
                p.defined.insert(name.to_string(), at.to_string());
            }
            self.definitions.insert(name.to_string(), at);
        }
        self.registry.register_lambda(name, body);
    }

    fn unregister_lambda(&mut self, name: &str) {
        if let Some((s, _)) = self.registry.interner.lookup_call(name) {
            self.registry.unregister_lambda(s);
        }
    }

    fn is_lambda(&self, name: &str) -> bool {
        self.registry
            .interner
            .lookup_call(name)
            .and_then(|(s, _)| self.registry.slot(self.registry.resolve_target(s)))
            .is_some_and(|slot| slot.lambda.is_some())
    }

    fn is_native(&self, name: &str) -> bool {
        self.registry
            .interner
            .lookup_call(name)
            .and_then(|(s, _)| self.registry.slot(self.registry.resolve_target(s)))
            .is_some_and(|slot| slot.native.is_some())
    }

    fn is_alias(&self, name: &str) -> bool {
        self.registry
            .interner
            .lookup_call(name)
            .is_some_and(|(s, _)| self.registry.resolve_target(s) != s)
    }

    /// Forwards to the reporter, which is the point: the field is public and
    /// the CLI replaces it after the `Interp` is built, so the answer is a
    /// property of *now* rather than of construction (RFC-0005 §S5, D71).
    fn wants_stack(&self, severity: bund2_api::diag::Severity) -> bool {
        self.reporter.wants_stack(severity)
    }

    fn tier_frame(&mut self, body: &BundValue) {
        // Outside `enter` nobody would remove it, and a second request inside
        // one entry would bury the first: both are declined.
        let (Some(who), None) = (self.tier_entry, self.tier_shadow) else {
            return;
        };
        self.tier_shadow = Some(self.frames.len());
        self.frames.push(Frame {
            body: body.clone(),
            ip: 0,
            exit: None,
            who,
        });
    }

    fn tier_at(&mut self, ip: usize) {
        if let Some(f) = self.tier_shadow.and_then(|i| self.frames.get_mut(i)) {
            f.ip = ip;
        }
    }

    fn tier_never(&mut self, body: &BundValue) {
        // Only from inside `enter`, which is where a tier is speaking. Said
        // from anywhere else it would be a word keeping a body interpreted,
        // which is harmless and is still not what this is for.
        if self.tier_entry.is_some() {
            self.declined.remember(body);
        }
    }

    fn tier_offered(&self) -> bool {
        !self.tier_off
    }

    fn read_line(
        &mut self,
        ask: &bund2_api::input::Ask<'_>,
    ) -> Result<bund2_api::input::Read, String> {
        // A fed line first — §W4. Not remembered here: the reading word
        // says which lines a history keeps, as for a typed one.
        if let Some(line) = self.fed.pop_front() {
            return Ok(bund2_api::input::Read::Line(line));
        }
        self.input.line(ask)
    }

    fn open_input(&mut self, history: Option<&str>) -> Result<(), String> {
        self.input.open(history)
    }

    fn feed_line(&mut self, line: String) -> bool {
        self.fed.push_back(line);
        true
    }

    fn debugging(&mut self, what: bund2_api::Debugging) -> Result<Option<String>, Error> {
        use bund2_api::Debugging as D;
        let cmd = match what {
            D::Backtrace => return Ok(Some(self.render_backtrace())),
            D::Stacks => return Ok(Some(self.render_stacks())),
            D::Info => {
                return Ok(Some(match self.debug.as_ref() {
                    Some(d) => d.render_watched(),
                    None => "no debugger is attached\n".to_string(),
                }));
            }
            D::Words(filter) => {
                return Ok(Some(
                    self.render_words(filter.as_deref(), self.debug.as_deref()),
                ));
            }
            D::Classes => return Ok(Some(self.render_classes())),
            D::WatchDepth(n) => debug::Command::WatchDepth(n),
            // `debug.break "p.bund:12"` is the line form, as at the console.
            D::Break(w) => match debug::line_spec(&w) {
                Some((file, line)) => debug::Command::BreakLine(file, line),
                None => debug::Command::Break(w),
            },
            D::BreakAt(w, n) => debug::Command::BreakAt(w, n, None),
            D::Until(w) => debug::Command::Until(w),
            D::Catch => debug::Command::Catch,
            D::WatchIf(s, cond) => debug::Command::WatchIf(s, cond),
            D::WatchRedefine => debug::Command::WatchRedefine,
            D::BreakIf(w, cond) => debug::Command::BreakIf(w, cond),
            D::Watch(s) => debug::Command::Watch(s),
            D::WatchWorkbench => debug::Command::WatchWorkbench,
            D::Delete(w) => debug::Command::Delete(w),
            D::Step => debug::Command::Step,
            D::Next => debug::Command::Next,
            D::Finish => debug::Command::Finish,
            D::Continue => debug::Command::Continue,
            // A safepoint offered by a word. Nothing is attached or armed by
            // it: with no debugger this is nothing at all.
            // RFC-0011 §S8: the record so far, rendered by whatever the
            // embedder installed. Not a debugger request: nothing attaches.
            // No record, or none it can render: `Ok(None)` the first time, so
            // the word warns; an empty answer after, so a program that
            // snapshots in a loop warns once and not once a round.
            D::Profile(base) => {
                self.label_profile_sites();
                let writer = self.profile.as_ref().and_then(|p| p.writer.clone());
                let (Some(p), Some(writer)) = (self.profile.as_ref(), writer) else {
                    if self.profile_unkept_said {
                        return Ok(Some(String::new()));
                    }
                    self.profile_unkept_said = true;
                    return Ok(None);
                };
                let (snap, open) = p.snapshot();
                return writer(&snap, &self.registry, &base, open)
                    .map(Some)
                    .map_err(Error);
            }
            D::Term(i, v) => {
                if self.debug.is_some() {
                    self.safepoint_for(None, debug::Stop::Term(i, v.summary(48)));
                }
                return Ok(None);
            }
        };
        // **§W5: a word that arms or moves attaches a debugger if none is.**
        // Quietly, so the run goes on to where the word said to stop.
        if self.debug.is_none() {
            if self.console_refused {
                return Ok(None);
            }
            let Some(make) = self.console_factory else {
                return Err(Error(
                    "no debugger is attached, and this VM was given no console to attach".into(),
                ));
            };
            self.debug = Some(Box::new(debug::Debug::quiet(make())));
            if let Some(p) = self.profile.as_mut() {
                p.console_attached = true;
            }
        }
        // D113.6: from the first such word, no body is compiled or run
        // compiled. A stop inside compiled code cannot happen.
        self.tier_off = true;
        let depth = self.frames.len();
        if let Some(d) = self.debug.as_mut() {
            // A moving word resumes; typed at a stop, the stop's loop has to
            // be told, because the line is run from inside it.
            if matches!(d.apply(cmd, depth), debug::Next::Resume) && d.busy {
                d.resume = true;
            }
        }
        Ok(None)
    }

    fn remember_line(&mut self, history: &str, line: &str) {
        self.input.remember(history, line);
    }

    fn read_secret(&mut self, prompt: &str) -> Result<String, String> {
        if let Some(line) = self.fed.pop_front() {
            return Ok(line);
        }
        self.input.secret(prompt)
    }

    fn note_source_failure(&mut self, label: &str, src: &str, span: (usize, usize), e: &Error) {
        if !self.reporter.wants_frames() {
            return;
        }
        match &mut self.failure_trace {
            // Innermost wins: a body this string called failed first, and the
            // string is where that body was reached from, so it is the place
            // to underline unless an inner string already took it.
            Some(t) if e.0.contains(&t.cause) => {
                if t.source.is_none() {
                    t.source = Some((label.to_string(), src.to_string(), span));
                }
            }
            _ => {
                self.failure_trace = Some(FailureTrace {
                    cause: e.0.clone(),
                    frames: self.trace_frames(),
                    source: Some((label.to_string(), src.to_string(), span)),
                    inner: self.innermost_place(),
                });
            }
        }
    }

    fn note_positions(
        &mut self,
        label: &str,
        src: &str,
        bodies: &[(BundValue, Vec<(usize, usize)>)],
    ) {
        if bodies.is_empty() {
            return;
        }
        if self.positions.len() >= self.positions_sweep {
            self.positions.retain(|_, p| p.body.strong_count() > 0);
            self.positions_sweep = (self.positions.len() * 2).max(Self::POSITIONS_SWEEP);
        }
        let file: Rc<str> = Rc::from(label);
        for (body, spans) in bodies {
            let (Some(key), Some(weak)) = (body.payload_key(), body.payload_weak()) else {
                continue;
            };
            let at = places(src, spans);
            self.positions.insert(
                key,
                Placed {
                    body: weak,
                    file: Rc::clone(&file),
                    at,
                },
            );
        }
    }

    fn place_top(&mut self, label: &str, src: &str, spans: &[(usize, usize)]) -> Option<usize> {
        Some(self.stream_place_key(label, src, spans))
    }

    /// The included stream's value runs as the program's would: its place is
    /// the running stream's for the call, and a debugger may stop before it.
    fn apply_placed(&mut self, stream: Option<usize>, i: usize, v: BundValue) -> Result<(), Error> {
        let Some(key) = stream else {
            return Interp::apply(self, v);
        };
        let outer = (self.stream_key, self.top_at, self.stream_floor);
        self.stream_outer.push(outer);
        (self.stream_key, self.top_at, self.stream_floor) = (Some(key), i, self.frames.len());
        self.nest += 1;
        if self.debug.is_some() {
            self.safepoint(Some((i, &v)));
        }
        let r = Interp::apply(self, v);
        self.nest -= 1;
        self.stream_outer.pop();
        (self.stream_key, self.top_at, self.stream_floor) = outer;
        r
    }

    fn failure_caught(&mut self, e: &Error) {
        self.caught = self.failure_trace.take().filter(|t| e.0.contains(&t.cause));
    }

    fn report(&mut self, d: bund2_api::diag::Diagnostic) {
        // RFC-0005 criterion 25's run-time half: natives return errors, they
        // do not report them, because a mid-body `Error` report would take a
        // snapshot of a stack whose values promotion may be holding.
        if d.severity.is_fatal()
            && let Some(who) = self.audit_native
        {
            self.audit_breach(who, "reported at `Error` severity while it ran");
        }
        // **The traceback, when one was taken for this failure — D173.** The
        // text test is what keeps a trace a `?try` swallowed from being pinned
        // on a later, unrelated error: the reported reason carries the
        // innermost failure's text, wrapped or not, or the trace is not its.
        let d = if d.severity.is_fatal() {
            match self.failure_trace.take() {
                Some(t) if d.reason.contains(&t.cause) => t.attach_fatal(d),
                _ => d,
            }
        } else {
            // A caught failure's report: the `except` block hands on the
            // text `?try` stored, so the same test finds its trace.
            match self.caught.take() {
                Some(t) if d.reason.contains(&t.cause) => t.attach(d),
                other => {
                    self.caught = other;
                    d
                }
            }
        };
        // Collect the stacks only if something will show them, for a
        // diagnostic of this severity (D45).
        let d = if self.reporter.wants_stack(d.severity) {
            // Compact by default; the raw `Debug` form only when a debug
            // session asks for it. A stack of raw renderings is ~150 columns a
            // row and tells the reader nothing they were looking for.
            let w = self.reporter.value_width();
            let raw = self.reporter.wants_raw_values();
            let fmt = |v: &BundValue| if raw { v.render(false) } else { v.summary(w) };
            // Read directly: a report's snapshot is the interpreter's, not the
            // reporting native's, and D45 already governs it (D55).
            let stack = self.stacks.current().contents().iter().map(&fmt).collect();
            let wb = self.stacks.workbench.contents().iter().map(&fmt).collect();
            d.on_stack(self.stacks.current_name().to_string())
                .with_stack(stack)
                .with_workbench(wb)
        } else {
            d.on_stack(self.stacks.current_name().to_string())
        };
        self.reporter.report(&d);
    }

    fn conditional(&self, ty: &str) -> Option<bund2_api::ConditionalFn> {
        self.registry.conditional(ty)
    }

    fn register_class(&mut self, name: &str, class: BundValue) {
        self.registry.register_class(name, class);
    }

    fn class(&self, name: &str) -> Option<BundValue> {
        self.registry.class(name)
    }

    fn is_class(&self, name: &str) -> bool {
        self.registry.is_class(name)
    }

    fn unregister_class(&mut self, name: &str) {
        self.registry.unregister_class(name);
    }

    fn method(&self, name: &str) -> Option<bund2_api::NativeFn> {
        self.registry.method(name)
    }

    /// D49's catch for a method native, and RFC-0011 §S2's hook 2.
    fn call_method(&mut self, name: &str) -> Option<Result<(), Error>> {
        let f = self.registry.method(name)?;
        let profiled = self.profile.is_some();
        if profiled {
            let depth = self.frames.len();
            let site = self.call_site();
            if let Some(p) = self.profile.as_mut() {
                p.open_method(name, depth, site);
                if p.tracing() {
                    p.trace_call(name);
                }
            }
        }
        let named = self.naming;
        let outer = if named {
            self.running.replace(Runner::Method(name.to_string()))
        } else {
            None
        };
        let r = bund2_api::catch_panic(|| f(&mut *self))
            .unwrap_or_else(|msg| Err(bund2_api::panicked(&format!("method `{name}`"), &msg)));
        if named {
            self.running = outer;
        }
        if profiled && let Some(p) = self.profile.as_mut() {
            if let Err(e) = &r {
                p.mark_failure(&e.0);
            }
            p.close_leaf();
        }
        Some(r)
    }

    /// D49's catch for a conditional runner, and RFC-0011 §S2's hook 2.
    fn run_conditional(&mut self, ty: &str, c: BundValue) -> Option<Result<(), Error>> {
        let f = self.registry.conditional(ty)?;
        let profiled = self.profile.is_some();
        if profiled {
            let depth = self.frames.len();
            let site = self.call_site();
            if let Some(p) = self.profile.as_mut() {
                p.open_conditional(ty, depth, site);
                if p.tracing() {
                    p.trace_call(ty);
                }
            }
        }
        let named = self.naming;
        let outer = if named {
            self.running.replace(Runner::Conditional(ty.to_string()))
        } else {
            None
        };
        let r = bund2_api::catch_panic(|| f(&mut *self, c))
            .unwrap_or_else(|msg| Err(bund2_api::panicked(&format!("conditional `{ty}`"), &msg)));
        if named {
            self.running = outer;
        }
        if profiled && let Some(p) = self.profile.as_mut() {
            if let Err(e) = &r {
                p.mark_failure(&e.0);
            }
            p.close_leaf();
        }
        Some(r)
    }

    fn is_method(&self, name: &str) -> bool {
        self.registry.is_method(name)
    }

    fn register_alias(&mut self, alias: &str, target: &str) -> bool {
        self.registry.register_alias(alias, target);
        self.registry.alias_cycles(alias)
    }

    fn unregister_alias(&mut self, alias: &str) {
        self.registry.unregister_alias(alias);
    }

    fn effect_of(&self, name: &str) -> Option<bund2_api::StackEffect> {
        self.registry.effect_of(name)
    }

    fn register_var(&mut self, name: &str, value: BundValue) {
        self.registry.register_var(name, value);
    }

    fn var(&self, name: &str) -> Option<BundValue> {
        self.registry.var(name)
    }

    fn unregister_var(&mut self, name: &str) {
        self.registry.unregister_var(name);
    }

    fn context_depth(&self) -> usize {
        self.contexts.len()
    }

    fn push_context(&mut self, name: &str) {
        // The stack to come back to is the one current *before* the switch.
        let prev = self.stacks.current_name().to_string();
        self.contexts.push((name.to_string(), prev));
    }

    fn pop_context(&mut self) -> Option<String> {
        self.contexts.pop().map(|(_, prev)| prev)
    }

    fn request_exit(&mut self, code: i32) {
        // A native that declares a fixed effect may not end the program: the
        // stop comes at the next step, after the native returned `Ok`, and a
        // compiled caller holding promoted values would not see it
        // (RFC-0005's eleventh review, B1). Only `bund.exit` asks, and it is
        // opaque; the audit makes that a property rather than an observation.
        if let Some(who) = self.audit_inside {
            self.audit_breach(who, "declares a fixed effect and requested an exit");
        }
        // The first request stands; a second, made while unwinding, does not
        // change the code.
        self.exit_code.get_or_insert(code);
    }

    fn exit_requested(&self) -> Option<i32> {
        self.exit_code
    }

    /// See [`bund2_api::Vm::inline_site`] for the contract. This applies
    /// RFC-0005 §S6's three inlining rules, in the order that makes each
    /// refusal cheap, and answers `None` at the first one that bites — which
    /// means "call this name, do not inline it", always correct and merely
    /// slower.
    fn inline_site(&mut self, name: &str) -> Option<bund2_api::InlineSite> {
        let (s, sigil) = self.registry.interner.lookup_call(name)?;
        // **A sigil is not a direct resolution.** `$name` skips the lambda and
        // alias checks entirely (`dispatch`, D16), so what it reaches is not
        // what an unsigiled site would reach, and a fragment inlined here would
        // be guarding the wrong question.
        if sigil {
            return None;
        }
        // **The alias rule.** A name reached through an alias resolves through
        // a *second* slot, whose rewrite would not touch this one's generation,
        // so there would be two generations to guard rather than one.
        if self.registry.resolve_target(s) != s {
            return None;
        }
        let slot = self.registry.slot(s)?;
        // **A command or a lambda in the slot is not a native site.** §S4's
        // chain puts both ahead of the native binding, so what runs here is not
        // the native the fragment specialises.
        if slot.command.is_some() || slot.lambda.is_some() {
            return None;
        }
        // **The registration, not the name** — a command carries no id, and
        // neither does anything but a registered native.
        let registration = slot.native.as_ref()?.id?;
        let generation = slot.generation();
        // **The saturation rule.** `Slot::bump` saturates so a stale inline
        // cache cannot match a wrapped counter; a slot pinned at the maximum
        // keeps that value through every later rewrite, so a guard against it
        // would never fire.
        if generation == u32::MAX {
            return None;
        }
        // Taken last, because it is the only step that may allocate: the cells
        // grow in fixed-size chunks so an address handed out here survives
        // every later registration (D43).
        let cell = self.registry.generation_cell(s).as_ptr() as usize;
        Some(bund2_api::InlineSite {
            registration,
            generation,
            cell,
        })
    }

    /// **RFC-0005 §S6's cells.** `Some`, always: an `Interp` owns them for its
    /// whole life, and their address is stable because they are boxed.
    ///
    /// This exists because a tier reaches the interpreter as `&mut dyn Vm` and
    /// nothing else on this trait would give it the addresses. When the trait
    /// method carried a `None` default, `Interp` inherited it and answered
    /// `None` while holding cells — compiling clean and guarding nothing. The
    /// method is required now, so that cannot recur.
    fn cells(&self) -> Option<&bund2_api::Cells> {
        Some(&self.cells)
    }

    fn get_lambda(&self, name: &str) -> Option<BundValue> {
        self.registry
            .interner
            .lookup_call(name)
            .and_then(|(s, _)| self.registry.slot(self.registry.resolve_target(s)))
            .and_then(|slot| slot.lambda.clone())
    }

    fn push_workbench(&mut self, v: BundValue) {
        // **As it arrives, untagged — F90.** The workbench "does not carry a
        // specific name" (`…/Introduction_the_art_of_stack_operations.typ:72`),
        // and the reference pushes to it with no `set_tag`
        // (`reference/rust_multistack/src/ts_workbench.rs:25-28`). So a value
        // moved off a stack keeps the tag that stack gave it, a fossil rather
        // than a location, and a value made on the spot — a conversion's
        // result, a match's answer — has none and renders `tags: {}`. Tagging
        // here with the current stack gave the second kind a tag the
        // reference never writes.
        // **§D4's *second* hook — criterion 9, checked separately.** The
        // workbench writes no stack tag, as the comment above explains: it has
        // no stack to name. So a `@name`-keyed watch cannot see this path, and
        // `watch workbench` is a different hook rather than the same one — the
        // correction §D4 makes to the first draft's "one place".
        if self.watch_hit() && self.debug.as_ref().is_some_and(|d| d.watches_workbench()) {
            self.safepoint_for(None, debug::Stop::WatchWorkbench);
        }
        self.stacks.workbench.items.push_back(v);
    }

    fn pull_workbench(&mut self) -> Option<BundValue> {
        self.stacks.workbench.pull()
    }

    fn workbench_depth(&self) -> usize {
        self.stacks.workbench.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a `Frame` measures with §D2's symbol in place — **56 bytes, up
    /// from 48**, measured 2026-10-03 on the tree either side of the change.
    ///
    /// `Symbol` is a `u32` with no niche, so `Option<Symbol>` costs eight
    /// bytes rather than four. Making it a `NonZeroU32` would halve that and
    /// would land inside `Frame`'s existing padding; it is not done, because
    /// criterion 12's wall-clock reading did not ask for it and shrinking a
    /// field to fit a measurement nobody needed is the wrong order.
    const FRAME_SIZE: usize = 56;
    use bund2_api::{StackEffect, WordKind};

    fn eff() -> StackEffect {
        StackEffect::fixed(0, 0)
    }
    fn marker(vm: &mut dyn Vm) -> Result<(), Error> {
        vm.push(BundValue::int(7));
        Ok(())
    }

    fn with_native(name: &str) -> (Interp, Symbol) {
        let mut i = Interp::new();
        let s = i
            .registry
            .register_native(name, marker, eff(), WordKind::Sync);
        (i, s)
    }

    #[test]
    fn a_native_dispatches_and_reaches_the_stack() {
        let (mut i, s) = with_native("w");
        i.dispatch(s, false).expect("dispatches");
        assert_eq!(i.depth(), 1);
        assert_eq!(i.pull(), Some(BundValue::int(7)));
    }

    /// RFC-0002's central behaviour, end to end this time: the same name
    /// reaches a lambda or the native depending only on the sigil.
    #[test]
    fn the_sigil_selects_the_native_over_the_lambda() {
        let (mut i, s) = with_native("println");
        // A real lambda now, with an effect that distinguishes it from the
        // native's. An earlier version of this test bound a non-lambda and read
        // the resulting error as proof the lambda arm was reached — which
        // stopped meaning anything once that arm was implemented.
        i.registry
            .register_lambda("println", BundValue::lambda(vec![BundValue::int(1)]));
        // `dispatch` on a lambda now *requests* a frame rather than running the
        // body (§S4), so the caller drives the loop. `apply` is the
        // synchronous door; `dispatch` alone leaves the request pending.
        i.apply(BundValue::call("println"))
            .expect("plain must reach the lambda");
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(1), "the lambda ran");
        i.dispatch(s, true).expect("$name must reach the native");
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(7), "the native ran");
    }

    /// The case D16 forces: a name built at run time, never lexed.
    #[test]
    fn a_runtime_string_dispatches_with_its_sigil() {
        let (mut i, _) = with_native("println");
        i.registry.register_lambda("println", BundValue::int(1));
        i.dispatch_name("$println").expect("reaches the native");
        assert_eq!(i.pull(), Some(BundValue::int(7)));
    }

    /// A miss must not intern — a program dispatching a computed miss in a
    /// loop must not grow the table.
    #[test]
    fn dispatching_a_miss_does_not_grow_the_interner() {
        let (mut i, _) = with_native("known");
        let before = i.registry.interner.len();
        for n in 0..500 {
            assert!(i.dispatch_name(&format!("miss{n}")).is_err());
        }
        assert_eq!(i.registry.interner.len(), before);
    }

    /// Dispatch follows the alias to the target's handler, not the alias's
    /// empty slot.
    #[test]
    fn dispatch_follows_an_alias_chain() {
        let (mut i, _) = with_native("println");
        i.registry.register_alias("b2", "println");
        let a2 = i.registry.register_alias("a2", "b2");
        i.dispatch(a2, false).expect("two links");
        assert_eq!(i.pull(), Some(BundValue::int(7)));
    }

    /// `is_command` returns before `autoadd` is consulted
    /// (`apply.rs:17` against `:19`), so a command runs even with the mode on.
    #[test]
    fn a_command_runs_even_under_autoadd() {
        let mut i = Interp::new();
        let s = i
            .registry
            .register_command("c", marker, eff(), WordKind::Sync);
        i.set_autoadd(true);
        i.dispatch(s, false).expect("commands precede autoadd");
        assert_eq!(i.pull(), Some(BundValue::int(7)));
    }

    /// Under `autoadd` a non-command name is appended to the value beneath
    /// rather than executed (`apply.rs:20-27`).
    /// **RFC-0005 §S8's Tier 0 floor, driven directly.** With the floor set
    /// above the current stack pointer, re-entering evaluation is refused with
    /// the stack-exhausted error — and nothing runs.
    #[test]
    fn below_the_floor_evaluation_is_refused_not_attempted() {
        let here = stack_marker();
        // floor = top - size + reserve = here + 4 * reserve, above us.
        set_stack_region(here + 4 * STACK_RESERVE, STACK_RESERVE);
        let mut i = Interp::new();
        STACK_REGION.with(|r| r.set(None));
        let body = BundValue::lambda(vec![BundValue::int(1)]);
        let e = Vm::eval_lambda(&mut i, &body).expect_err("refused below the floor");
        assert!(e.is_stack_exhausted(), "{}", e.0);
        assert_eq!(i.depth(), 0, "nothing ran");
    }

    /// A declared region puts the floor `size - reserve` below its top.
    #[test]
    fn a_declared_region_sets_the_floor() {
        set_stack_region(10 * 1024 * 1024, 8 * 1024 * 1024);
        let i = Interp::new();
        STACK_REGION.with(|r| r.set(None));
        assert_eq!(i.stack_floor, 2 * 1024 * 1024 + STACK_RESERVE);
    }

    /// **RFC-0005 §S8's two floors are different addresses**, and the Tier 1
    /// floor is the higher one: `top - share + reserve` against Tier 0's
    /// `top - size + reserve`. Mirroring Tier 0's into the cell would admit
    /// compiled frames into Tier 0's part, which D44's "never less" forbids.
    #[test]
    fn a_declared_share_puts_the_tier_one_floor_above_the_tier_zero_floor() {
        const TOP: usize = 20 * 1024 * 1024;
        const SIZE: usize = 16 * 1024 * 1024;
        const SHARE: usize = 8 * 1024 * 1024;
        set_stack_region_with_share(TOP, SIZE, SHARE);
        let i = Interp::new();
        STACK_REGION.with(|r| r.set(None));

        assert_eq!(
            i.stack_floor,
            TOP - SIZE + STACK_RESERVE,
            "Tier 0's floor is one reserve above the stack's end"
        );
        assert_eq!(
            i.cells().floor(),
            TOP - SHARE + STACK_RESERVE,
            "Tier 1's floor is one reserve above the share's bottom"
        );
        assert!(
            i.cells().floor() > i.stack_floor,
            "the Tier 1 floor is the higher address: {} vs {}",
            i.cells().floor(),
            i.stack_floor
        );
    }

    /// **A region declared without a share is Tier 0's alone** — §S8, the
    /// ninth review's S3. The share is zero, so the Tier 1 floor lands one
    /// reserve *above* the thread's own top: no stack pointer is ever above it,
    /// every compiled body declines, and compiled code does not run there.
    ///
    /// This is the property that makes a default split unnecessary, and it is
    /// reached by arithmetic rather than by a flag.
    #[test]
    fn a_region_declared_without_a_share_keeps_compiled_code_out() {
        const TOP: usize = 20 * 1024 * 1024;
        set_stack_region(TOP, 16 * 1024 * 1024);
        let i = Interp::new();
        STACK_REGION.with(|r| r.set(None));

        assert_eq!(
            i.cells().floor(),
            TOP + STACK_RESERVE,
            "no share puts the floor above the region's top"
        );
        assert!(
            i.cells().floor() > TOP,
            "so every stack pointer in the region is beneath it"
        );
    }

    /// An undeclared thread gets no share either, by the same rule: the floor
    /// sits above the construction point, so nothing compiled runs.
    #[test]
    fn an_undeclared_thread_keeps_compiled_code_out() {
        STACK_REGION.with(|r| r.set(None));
        let here = stack_marker();
        let i = Interp::new();
        assert!(
            i.cells().floor() > here,
            "the floor is above the point the Interp was built at: {} vs {here}",
            i.cells().floor()
        );
    }

    /// **Collecting appends the name *into* the value beneath, leaving one
    /// value** (`reference/rust_multistackvm/src/multistackvm_apply.rs:19-22`).
    ///
    /// **This test asserted the opposite until 2026-09-30**, and F84 recorded
    /// that it "pins the divergence rather than catching it": it required the
    /// CALL and the value beneath as *two* entries. The reference leaves one.
    /// Nothing could observe the difference at the time, because nothing bound
    /// `:` or `;`.
    #[test]
    fn autoadd_appends_the_name_into_the_value_beneath() {
        let (mut i, s) = with_native("w");
        i.set_autoadd(true);
        i.push(BundValue::list(vec![BundValue::int(1)]));
        i.dispatch(s, false).expect("autoadd");
        assert_eq!(i.depth(), 1, "one value, not two");
        let top = i.pull().expect("the collector");
        assert_eq!(top.dt(), bund2_value::LIST);
        let items = top.as_list().expect("a list").to_vec();
        assert_eq!(items.len(), 2, "the name joined the collector: {items:?}");
        assert_eq!(items[0], BundValue::int(1));
        assert_eq!(
            items[1].dt(),
            bund2_value::CALL,
            "and it went in as a CALL, not as a string"
        );
    }

    /// **A non-container collector is replaced, not appended to** — `push`'s
    /// `_` arm (`reference/rust_dynamic/src/push.rs:143-149`). Confirmed
    /// against the oracle: `5 : 1 2 ;` leaves `2`.
    #[test]
    fn collecting_onto_a_scalar_replaces_it() {
        let mut i = Interp::new();
        i.set_autoadd(true);
        i.push(BundValue::int(5));
        i.apply_step(BundValue::int(1)).expect("collects");
        i.apply_step(BundValue::int(2)).expect("collects");
        assert_eq!(i.depth(), 1, "still one value");
        assert_eq!(
            i.pull(),
            Some(BundValue::int(2)),
            "each applied value replaces the one before"
        );
    }

    /// **A literal is collected too**, which F84 recorded as absent: "Literals
    /// and CONTEXT values are not collected at all."
    #[test]
    fn collecting_gathers_literals() {
        let mut i = Interp::new();
        i.set_autoadd(true);
        i.push(BundValue::list(vec![BundValue::int(0)]));
        for n in 1..=3 {
            i.apply_step(BundValue::int(n)).expect("collects");
        }
        assert_eq!(i.depth(), 1);
        let items = i
            .pull()
            .and_then(|v| v.as_list().map(<[BundValue]>::to_vec))
            .expect("a list");
        assert_eq!(
            items,
            vec![
                BundValue::int(0),
                BundValue::int(1),
                BundValue::int(2),
                BundValue::int(3)
            ]
        );
    }

    /// **A CONTEXT is pushed, not appended and not switched to** — the
    /// reference's asymmetry (`multistackvm_apply.rs:72-73`), preserved.
    ///
    /// The consequence is what makes it worth a test: the collector stops
    /// being the top of the stack, so a following value appends into the
    /// CONTEXT instead.
    #[test]
    fn collecting_pushes_a_context_instead_of_switching() {
        let mut i = Interp::new();
        let before = Interp::current_name(&i).to_string();
        i.set_autoadd(true);
        i.push(BundValue::list(vec![BundValue::int(0)]));
        i.apply_step(BundValue::named_context("elsewhere"))
            .expect("pushed");
        assert_eq!(
            Interp::current_name(&i),
            before,
            "the stack did not switch, which is the whole asymmetry"
        );
        assert_eq!(i.depth(), 2, "the CONTEXT sits on top of the collector");
        assert_eq!(i.peek().map(|v| v.dt()), Some(bund2_value::CONTEXT));
    }

    /// Every push writes the stack tag, with no type test — and **since D41 a
    /// scalar is no longer boxed to carry it.**
    ///
    /// This test used to assert the opposite: "tagging a scalar boxes it". It
    /// was right, and it was the whole cost — `promote/scalar` measured
    /// 25.1 ns of allocation for a value that only needed four bytes of
    /// padding. The tag is still reported identically, which is what the 39
    /// tag-bearing goldens check; only the allocation is gone.
    #[test]
    fn push_tags_with_the_current_stack() {
        let mut i = Interp::new();
        i.push(BundValue::int(1));
        let v = i.pull().expect("pushed");
        assert_eq!(v.tags().get("stack").map(|s| &**s), Some("main"));
        assert!(
            !v.is_boxed(),
            "D41: a scalar carries its tag inline and must not be boxed"
        );
    }

    /// **D41 rule 2.** Boxing a tagged scalar carries the symbol into the
    /// header's map. Without this a value that gains an `attr` after being
    /// pushed would silently lose its stack tag — the one violation in D41
    /// that no compiler catches.
    #[test]
    fn boxing_a_tagged_scalar_carries_the_tag_into_the_header() {
        let mut i = Interp::new();
        i.to_stack("side");
        i.push(BundValue::int(1));
        let v = i.pull().expect("pushed");
        assert!(!v.is_boxed());
        let boxed = v.promote();
        assert!(boxed.is_boxed());
        assert_eq!(
            boxed.tags().get("stack").map(|s| &**s),
            Some("side"),
            "the symbol must survive promotion into the map"
        );
    }

    #[test]
    fn a_named_stack_becomes_current_and_carries_its_own_tag() {
        let mut i = Interp::new();
        i.to_stack("side");
        assert_eq!(i.stacks.current_name(), "side");
        i.push(BundValue::int(1));
        assert_eq!(
            i.pull().unwrap().tags().get("stack").map(|s| &**s),
            Some("side")
        );
    }

    /// **The workbench tag is a fossil of the source stack, not of the
    /// workbench.** The workbench "does not carry a specific name", so a value
    /// returned there keeps the tag of the stack it came from.
    ///
    /// This is why `Stack::push_as` exists beside `Stack::push`. When each
    /// stack learned its own name — to stop `current_mut` allocating it twice
    /// per push — the workbench acquired one too, and pushing with *its* name
    /// would have quietly erased the fossil. Confirmed against the oracle:
    /// `@foo 1 return take` leaves a value tagged `stack: "foo"`.
    #[test]
    fn the_workbench_keeps_the_tag_of_the_stack_a_value_came_from() {
        let mut i = Interp::new();
        i.to_stack("foo");
        i.push(BundValue::int(1));
        let v = i.pull().expect("pushed");
        i.push_workbench(v);
        let back = i.pull_workbench().expect("returned");
        assert_eq!(
            back.tags().get("stack").map(|s| &**s),
            Some("foo"),
            "the workbench must not retag with its own name"
        );
    }

    /// **D13, both branches.** `with_tag` splits only when the header is
    /// actually shared; a uniquely owned one is mutated in place and needs no
    /// identity, because there is no second half to diverge from.
    ///
    /// The property D13 protects is the *shared* case, and it is asserted in
    /// `bund2-value`. What this test pins is that the fast branch is reachable
    /// at all — before the fix, `with_tag` took `&self` and cloned first, so
    /// the answer to "is anyone else holding this?" was always yes and the
    /// in-place path was dead code.
    #[test]
    fn pushing_a_fresh_scalar_does_not_mint_an_identity() {
        let mut i = Interp::new();
        i.push(BundValue::int(7));
        let v = i.pull().expect("pushed");
        assert!(
            !v.has_identity(),
            "a value that was never shared must not have been minted on push"
        );
        assert_eq!(v.tags().get("stack").map(|s| &**s), Some("main"));
    }

    /// Selecting a stack rotates the stack-of-stacks rather than reassigning
    /// it, which is what the guide describes and what `stacks_left` acts on.
    #[test]
    fn selecting_a_stack_rotates_rather_than_reassigns() {
        let mut i = Interp::new();
        i.to_stack("a");
        i.to_stack("b");
        i.to_stack("main");
        assert_eq!(i.stacks.current_name(), "main");
        assert_eq!(i.stacks.names().count(), 3, "no stack was lost");
    }

    /// **The current stack is held out of the map**, and every way the ring
    /// moves must bring the right stack in and put the old one back. Each
    /// stack holds one marker; after switching, rotating and dropping the
    /// current stack, every survivor still holds its own and the dropped one
    /// is gone.
    #[test]
    fn the_held_current_stack_follows_every_move_of_the_ring() {
        let mut i = Interp::new();
        i.push(BundValue::int(0));
        for (n, name) in [(1, "a"), (2, "b"), (3, "c")] {
            i.to_stack(name);
            i.push(BundValue::int(n));
        }
        let holds = |i: &Interp, name: &str| -> Vec<i64> {
            i.snapshot_of(name)
                .iter()
                .filter_map(|v| v.as_int())
                .collect()
        };
        i.rotate_stacks_left();
        i.rotate_stacks_right();
        i.rotate_stacks_right();
        let cur = i.stacks.current_name().to_string();
        assert_eq!(
            i.peek().and_then(|v| v.as_int()),
            holds(&i, &cur).last().copied()
        );
        i.to_stack("b");
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(2));
        i.push(BundValue::int(2));
        i.drop_stack("b");
        assert!(
            !i.stack_exists("b"),
            "the dropped current stack must not come back"
        );
        for (n, name) in [(0, "main"), (1, "a"), (3, "c")] {
            assert_eq!(holds(&i, name), vec![n], "{name} lost or swapped its value");
        }
        let cur = i.stacks.current_name().to_string();
        assert_eq!(i.depth(), i.depth_of(&cur), "depth reads the current stack");
        assert_eq!(i.stacks.names().count(), 3);
        // Dropping every stack leaves one, empty and current, under a fresh
        // id-shaped name as the reference's `ensure` gives it
        // (`reference/rust_multistack/src/ts_ensure.rs:5-6`), not `main`.
        for name in ["a", "c", "main"] {
            i.drop_stack(name);
        }
        let fresh = i.stacks.current_name().to_string();
        assert_eq!(fresh.chars().count(), 21, "{fresh}");
        assert!(!["a", "c", "main"].contains(&fresh.as_str()));
        assert_eq!(i.stacks.names().count(), 1);
        assert_eq!(i.depth(), 0);
    }

    /// **The CALL memo caches a name's symbol, never its binding.** A body run
    /// once, the word re-registered, and the same body run again calls the
    /// new binding.
    #[test]
    fn a_memoised_call_sees_a_word_registered_again() {
        fn one(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.push(BundValue::int(1));
            Ok(())
        }
        fn two(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.push(BundValue::int(2));
            Ok(())
        }
        let mut i = Interp::new();
        let body = [BundValue::call("w")];
        i.registry.register_native("w", one, eff(), WordKind::Sync);
        i.eval(&body).unwrap();
        assert_ne!(body[0].call_memo(), 0, "the first call left a memo");
        i.registry.register_native("w", two, eff(), WordKind::Sync);
        i.eval(&body).unwrap();
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(2));
    }

    /// **A miss is not memoised**: a name registered after a call failed is
    /// found by the same CALL value the next time.
    #[test]
    fn a_call_that_missed_finds_a_word_registered_later() {
        fn one(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.push(BundValue::int(1));
            Ok(())
        }
        let mut i = Interp::new();
        let body = [BundValue::call("later")];
        let e = i.eval(&body).unwrap_err();
        assert!(e.0.contains("later not registered"), "{}", e.0);
        assert_eq!(body[0].call_memo(), 0);
        i.registry
            .register_native("later", one, eff(), WordKind::Sync);
        i.eval(&body).unwrap();
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(1));
    }

    /// **One interpreter's memo is a miss in another.** The second interns
    /// two names first, so `w` has a different symbol there; reading the
    /// first's memo would dispatch to `pad`, which is bound to nothing.
    #[test]
    fn a_memo_left_by_another_interpreter_is_not_read() {
        fn one(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.push(BundValue::int(1));
            Ok(())
        }
        fn two(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.push(BundValue::int(2));
            Ok(())
        }
        let body = [BundValue::call("w")];
        let mut first = Interp::new();
        first
            .registry
            .register_native("w", one, eff(), WordKind::Sync);
        first.eval(&body).unwrap();
        let mut second = Interp::new();
        second.registry.interner.intern("pad");
        second.registry.interner.intern("pad2");
        second
            .registry
            .register_native("w", two, eff(), WordKind::Sync);
        second.eval(&body).unwrap();
        assert_eq!(second.pull().and_then(|v| v.as_int()), Some(2));
        // And a cloned registry's interner takes its own key.
        let cloned = second.registry.interner.clone();
        assert_eq!(cloned.from_memo(body[0].call_memo()), None);
    }

    /// The stacks are circular buffers, which is why they are `VecDeque`s.
    #[test]
    fn a_stack_rotates() {
        let mut s = Stack::default();
        for n in 1..=3 {
            s.push(BundValue::int(n));
        }
        assert_eq!(s.peek().map(|v| v.dt()), Some(bund2_value::INTEGER));
        s.rotate_left();
        assert_eq!(s.len(), 3, "rotation moves, it does not consume");
    }

    /// **The effect audit's report half — RFC-0005 criterion 25.** A native
    /// that reports at `Error` severity while it runs is a breach whatever its
    /// effect, opaque included. A warning is not.
    #[test]
    fn the_audit_records_an_error_reported_mid_body() {
        fn reports_error(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.report(bund2_api::diag::Diagnostic::error("mid-body"));
            Ok(())
        }
        fn warns(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.report(bund2_api::diag::Diagnostic::warning("fine"));
            Ok(())
        }
        let mut i = Interp::new();
        i.registry
            .register_native("e", reports_error, StackEffect::opaque(0), WordKind::Sync);
        i.registry
            .register_native("w", warns, eff(), WordKind::Sync);
        i.effect_audit = Some(Vec::new());
        let _ = i.eval(&[BundValue::call("w"), BundValue::call("e")]);
        assert_eq!(
            i.effect_audit.take().unwrap_or_default(),
            vec!["`e` reported at `Error` severity while it ran".to_string()]
        );
    }

    /// **D71's seam: `wants_stack` answers for the reporter in place now** —
    /// RFC-0005 §S5, F137.
    ///
    /// A compiled body that crosses a call holds values in registers across it,
    /// and a native reporting a `Warning` or `Notice` there would snapshot a
    /// stack without them. The tier asks this at each body's entry, so the
    /// answer has to be a property of the reporter the `Interp` holds **now**:
    /// the field is public and the CLI replaces it after construction, which is
    /// the reason §S5 says "read at each compiled body's entry" rather than
    /// "fixed when the `Interp` is built".
    ///
    /// Before this existed, no `Vm` method reached the reporter at all, which
    /// is half of what made F137 more than a missing `if`.
    #[test]
    fn wants_stack_answers_for_the_reporter_in_place_now() {
        use bund2_api::diag::Severity;
        let mut i = Interp::new();
        // The default is `SilentReporter`, which shows nothing and so wants
        // nothing collected, at any severity.
        assert!(
            !<Interp as Vm>::wants_stack(&i, Severity::Warning),
            "the default reporter wants no mid-body snapshot"
        );

        i.reporter = Box::new(bund2_api::diag::CollectingReporter {
            wants_stack: true,
            ..Default::default()
        });
        assert!(
            <Interp as Vm>::wants_stack(&i, Severity::Warning),
            "a reporter swapped in after construction must be the one that answers"
        );
        assert!(<Interp as Vm>::wants_stack(&i, Severity::Notice));
    }

    /// **The effect audit's depth half — RFC-0005 criterion 24.** A native
    /// declaring `0 -> 0` that pushes is a breach. With the audit off, nothing
    /// is recorded and the word runs as it always did.
    #[test]
    fn the_audit_records_a_pair_that_miscounts() {
        fn pushes(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.push(BundValue::int(1));
            Ok(())
        }
        let mut off = Interp::new();
        off.registry
            .register_native("p", pushes, eff(), WordKind::Sync);
        assert!(off.eval(&[BundValue::call("p")]).is_ok());
        assert!(off.effect_audit.is_none());

        let mut on = Interp::new();
        on.registry
            .register_native("p", pushes, eff(), WordKind::Sync);
        on.effect_audit = Some(Vec::new());
        assert!(on.eval(&[BundValue::call("p")]).is_ok());
        let log = on.effect_audit.take().unwrap_or_default();
        assert_eq!(log.len(), 1, "{log:?}");
        assert!(
            log.iter()
                .all(|l| l.starts_with("`p` declares a fixed effect and declares (0, 0)")),
            "{log:?}"
        );
    }

    /// **D49 and F95.** A native that panics returns `Error::internal` naming
    /// it, and evaluation goes on being usable: the panic does not unwind out
    /// of `eval`.
    #[test]
    fn a_panicking_native_is_an_internal_error_not_an_unwind() {
        fn boom(_: &mut dyn Vm) -> Result<(), Error> {
            panic!("boom from a dependency");
        }
        let mut i = Interp::new();
        i.registry
            .register_native("boom", boom, eff(), WordKind::Sync);
        let e = i
            .eval(&[BundValue::call("boom")])
            .expect_err("the panic is an error");
        // `eval` wraps what it returns, so the internal error is inside it.
        assert!(
            e.0.contains("internal error: native `boom` panicked: boom from a dependency"),
            "{}",
            e.0
        );
        i.eval(&[BundValue::int(1)])
            .expect("the interpreter still runs");
        assert_eq!(i.depth(), 1);
    }

    /// **RFC-0008 criterion 12's other half: the size, pinned.**
    #[test]
    fn a_frame_is_the_size_it_was_measured_at() {
        // **RFC-0008 criterion 12's other half.** A frame is pushed per call,
        // so §D2's symbol is the one field this RFC adds to a hot structure.
        // The wall-clock cost is measured against RFC-0005 criterion 7's band;
        // this pins the *size*, so a later field cannot arrive unnoticed
        // between two benchmark runs.
        //
        // Printed rather than asserted blind: if this fails, the number in the
        // message is the new size and the question is whether it was meant.
        let n = std::mem::size_of::<Frame>();
        assert_eq!(
            n, FRAME_SIZE,
            "a Frame is now {n} bytes, not {FRAME_SIZE}. If that was deliberate, \
             re-measure criterion 12 and change the constant; a field on this \
             structure costs one allocation slot per Bund call."
        );
    }

    /// **F96.** A native that files a tail request and then fails leaves no
    /// request behind. Before the fix the request waited in `pending_tail`, and
    /// the next evaluation ran the body first — a body nobody asked for, run
    /// after its caller's error had been dealt with.
    #[test]
    fn every_writer_of_the_request_cell_is_named() {
        // **RFC-0005 assumption 33, the seventeenth review's B1.** Every
        // function whose shipped code writes `pending_tail`. RFC-0005's
        // compiled tier mirrors that cell, and a write from anywhere else
        // leaves the mirror behind Tier 0 with nothing to notice. The RFC
        // stated this set wrongly twice, so it is derived here, in the shape
        // criterion 25's scan and criterion 11's path set already use.
        const WRITERS: [&str; 6] = [
            // **RFC-0008 §W2, D113.** A line typed at a debugger stop sets the
            // request aside and puts it back, so the line can neither take
            // nor discard the body a stopped native filed. It writes the
            // mirror at both ends. Reached only from a safepoint.
            "evaluate_typed",
            // D56: the clear for a caller that cannot reach `invoke`.
            "clear_tail_request",
            // F96: clears one a failing native filed.
            "invoke",
            // **`request_tail_for`, not `request_tail`** — RFC-0008 §D2 gave
            // the request a symbol to carry, so the public `request_tail` now
            // delegates and writes nothing itself. This test is what noticed;
            // RFC-0005's assumption 33 is corrected to match.
            "request_tail_for",
            "take_pending",
            // RFC-0005 §S5's hand-back: takes a body a compiled one left
            // filed, as `take_pending` does, and clears the mirror with it.
            "push_frame",
        ];
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
        let text = std::fs::read_to_string(&path).expect("reads");
        let shipped = text.split("#[cfg(test)]\nmod ").next().unwrap_or_default();
        let mut found = std::collections::BTreeSet::new();
        let mut mirrored = std::collections::BTreeSet::new();
        let mut current = String::new();
        for line in shipped.lines() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            if let Some(at) = t.find("fn ") {
                let head_ok = t[..at]
                    .split_whitespace()
                    .all(|w| ["pub", "const", "unsafe", "async", "extern"].contains(&w));
                if head_ok {
                    current = t[at + 3..]
                        .split(['(', '<'])
                        .next()
                        .unwrap_or_default()
                        .to_string();
                }
            }
            // A write, not a read: an assignment, or the `take` that clears it.
            if t.contains("pending_tail =") || t.contains("pending_tail.take()") {
                found.insert(current.clone());
            }
            // **The mirror, RFC-0005 §S6.** Every writer of `pending_tail`
            // must also write the request cell, or Tier 0 and compiled code
            // disagree about whether a body is waiting to run.
            if t.contains("cells.set_request(") {
                mirrored.insert(current.clone());
            }
        }
        let named: std::collections::BTreeSet<String> =
            WRITERS.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(
            found, named,
            "the request cell's writers and RFC-0005 assumption 33 differ"
        );
        // **The pairing, not just the set.** §S6's *Addressing* requires the
        // mirror to be written wherever `pending_tail` is; a writer that
        // updated one and not the other would leave compiled code draining a
        // body Tier 0 had discarded, or missing one it had filed. Asserting
        // the two sets are equal catches a half-write in either direction —
        // including a mirror write in a function that no longer writes the
        // truth.
        assert_eq!(
            mirrored, named,
            "every writer of `pending_tail` must write RFC-0005 §S6's mirror beside it"
        );
    }

    /// **`drain_tail_request` runs a filed body now** — RFC-0005 §S5's drain
    /// helper, Tier 0's half. The compiled tier calls this after a non-tail
    /// call, where Tier 0 would have drained inside `Interp::apply`.
    #[test]
    fn draining_runs_the_filed_body() {
        let mut i = Interp::new();
        i.request_tail(BundValue::lambda(vec![BundValue::int(99)]));
        assert!(i.cells().request(), "filed");

        Vm::drain_tail_request(&mut i).expect("the body ran");
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(99), "it ran now");
        assert!(!i.cells().request(), "and the mirror is clear");
    }

    /// Draining with nothing filed is a no-op, not an error.
    #[test]
    fn draining_nothing_does_nothing() {
        let mut i = Interp::new();
        Vm::drain_tail_request(&mut i).expect("nothing to do");
        assert_eq!(i.depth(), 0);
    }

    /// **A refused drain clears the request** — §S5: "no stale request outlives
    /// an error". With the floor above the stack pointer the drain cannot
    /// re-enter evaluation, so it reports exhaustion and leaves nothing behind
    /// for the next `take_pending` to run.
    #[test]
    fn a_drain_below_the_floor_clears_the_request() {
        let here = stack_marker();
        set_stack_region(here + 4 * STACK_RESERVE, STACK_RESERVE);
        let mut i = Interp::new();
        STACK_REGION.with(|r| r.set(None));

        i.request_tail(BundValue::lambda(vec![BundValue::int(99)]));
        let e = Vm::drain_tail_request(&mut i).expect_err("refused below the floor");
        assert!(e.is_stack_exhausted(), "{}", e.0);
        assert!(!i.cells().request(), "the mirror is clear");
        assert_eq!(i.depth(), 0, "and nothing ran");
    }

    /// **The drain does not consult the exit gate.** RFC-0005 §S5 gives that to
    /// the compiled tier's one status-making function, which substitutes the
    /// refusal after an `Ok`; doing it here as well would substitute twice.
    ///
    /// So a body that records an exit drains successfully, and the refusal is
    /// the caller's to make.
    #[test]
    fn draining_leaves_the_exit_to_the_status_maker() {
        fn exits(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.request_exit(7);
            Ok(())
        }
        let mut i = Interp::new();
        i.registry
            .register_native("ex", exits, StackEffect::opaque(0), WordKind::Sync);
        i.request_tail(BundValue::lambda(vec![BundValue::call("ex")]));

        Vm::drain_tail_request(&mut i).expect("the drain itself does not refuse");
        assert_eq!(i.exit_requested(), Some(7), "the exit is recorded");
    }

    /// **The cells are reachable through `&mut dyn Vm`, which is how a tier
    /// sees them.** RFC-0005 §S6 has the JIT take each cell's address at
    /// compile time, and a tier holds the interpreter only as `&mut dyn Vm`.
    ///
    /// This is written against the trait object on purpose: `Vm::cells` once
    /// carried a `None` default, and `Interp` inherited it while owning cells,
    /// so every guard would have been skipped with the build green. Asking
    /// through `Interp` directly would not have caught it.
    #[test]
    fn a_tier_reaches_the_cells_through_the_trait() {
        let mut i = Interp::new();
        let vm: &mut dyn Vm = &mut i;
        assert!(
            vm.cells().is_some(),
            "an interpreter must offer its cells through the trait a tier holds"
        );
    }

    /// **The `autoadd` mirror cannot drift, because the field is private.**
    #[test]
    fn the_autoadd_mirror_tracks_the_mode() {
        let mut i = Interp::new();
        assert!(!i.cells().autoadd(), "off at construction");
        i.set_autoadd(true);
        assert!(i.autoadd(), "the mode");
        assert!(i.cells().autoadd(), "and the mirror");
        i.set_autoadd(false);
        assert!(!i.cells().autoadd(), "and back");
    }

    /// **The epoch moves on a switch and on nothing else.**
    ///
    /// Pushing and pulling leave it alone — compiled code holding promoted
    /// values cares which stack is current, not how deep it is — and a
    /// `to_stack` to the stack already current is not a switch.
    #[test]
    fn the_epoch_moves_only_when_the_current_stack_changes() {
        let mut i = Interp::new();
        let start = i.cells().epoch.get();

        i.push(BundValue::int(1));
        i.pull();
        assert_eq!(i.cells().epoch.get(), start, "depth is not a switch");

        Vm::to_stack(&mut i, "main");
        assert_eq!(
            i.cells().epoch.get(),
            start,
            "to_stack to the stack already current is not a switch"
        );

        Vm::to_stack(&mut i, "s");
        let after = i.cells().epoch.get();
        assert!(after > start, "a real switch bumps: {start} -> {after}");

        Vm::to_stack(&mut i, "main");
        assert!(i.cells().epoch.get() > after, "and switching back bumps");
    }

    /// **Every path that can move the front of the ring bumps the epoch**, and
    /// the ones that cannot do not.
    ///
    /// `drop_stack` and the rotations used to reach past `Stacks` into `order`
    /// directly, which is why `order` is now sealed: a switch that did not bump
    /// would leave a compiled body guarding on a stale epoch. A rotation of a
    /// one-stack ring moves nothing and must not bump, or every guard in a
    /// compiled body would fire on a program with a single stack.
    #[test]
    fn every_way_the_current_stack_changes_moves_the_epoch() {
        let mut i = Interp::new();

        let one = i.cells().epoch.get();
        i.rotate_stacks_left();
        assert_eq!(i.cells().epoch.get(), one, "a ring of one does not rotate");
        i.rotate_stacks_right();
        assert_eq!(i.cells().epoch.get(), one, "in either direction");

        Vm::to_stack(&mut i, "s");
        let two = i.cells().epoch.get();
        assert!(two > one, "a switch to another stack moves it");
        // The third exact case, which this test did not assert until
        // RFC-0005's twenty-fourth review (B7).
        Vm::to_stack(&mut i, "s");
        assert_eq!(
            i.cells().epoch.get(),
            two,
            "`to_stack` to the stack already current is not a switch"
        );
        i.rotate_stacks_left();
        assert!(i.cells().epoch.get() > two, "a ring of two rotates");

        let before_drop = i.cells().epoch.get();
        i.ensure_stack("keep");
        let after_ensure = i.cells().epoch.get();
        assert!(
            after_ensure > before_drop,
            "a created stack becomes current — F89"
        );

        // Dropping a stack that is not current moves nothing.
        let held = i.current_name();
        let other = if held == "s" { "main" } else { "s" };
        let before = i.cells().epoch.get();
        i.drop_stack(other);
        assert_eq!(
            i.cells().epoch.get(),
            before,
            "dropping a stack that is not current is not a switch"
        );

        // Dropping the current one does.
        let current = i.current_name();
        i.drop_stack(&current);
        assert!(
            i.cells().epoch.get() > before,
            "dropping the current stack switches"
        );
    }

    /// **The request mirror tracks `pending_tail` through a real program**, not
    /// only through direct calls: a native files a request, the loop drains it,
    /// and the mirror is clear at the end.
    #[test]
    fn the_request_mirror_tracks_the_pending_body() {
        fn files(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.tail_lambda(BundValue::lambda(vec![BundValue::int(7)]));
            Ok(())
        }
        let mut i = Interp::new();
        i.registry
            .register_native("f", files, StackEffect::opaque(0), WordKind::Sync);

        assert!(!i.cells().request(), "clear before anything runs");
        i.eval(&[BundValue::call("f")]).expect("runs");
        assert!(
            !i.cells().request(),
            "and clear again once the body has been drained"
        );
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(7), "the body ran");
    }

    /// **A failing native's request is cleared in the mirror too — F96.**
    ///
    /// Tier 0 discards the body; if the mirror kept it, compiled code would
    /// drain a body Tier 0 had already thrown away, which is the half of F96
    /// that only exists once a mirror does.
    #[test]
    fn a_failed_natives_request_clears_the_mirror() {
        fn files_then_fails(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.tail_lambda(BundValue::lambda(vec![BundValue::int(99)]));
            Err(Error("failed after filing".into()))
        }
        let mut i = Interp::new();
        i.registry.register_native(
            "ff",
            files_then_fails,
            StackEffect::opaque(0),
            WordKind::Sync,
        );
        assert!(i.eval(&[BundValue::call("ff")]).is_err());
        assert!(
            !i.cells().request(),
            "F96 clears the mirror as well as the truth"
        );
    }

    /// **`Vm::clear_tail_request` clears both — D56.** This is the path a
    /// compiled call site reaches, the one with no `invoke` behind it.
    #[test]
    fn clearing_through_the_trait_clears_the_mirror() {
        let mut i = Interp::new();
        i.request_tail(BundValue::lambda(vec![BundValue::int(1)]));
        assert!(i.cells().request(), "filed");
        Vm::clear_tail_request(&mut i);
        assert!(!i.cells().request(), "and cleared in the mirror");
        i.eval(&[BundValue::int(5)]).expect("runs");
        assert_eq!(i.depth(), 1, "the discarded body did not run");
    }

    #[test]
    fn a_failed_native_leaves_no_tail_request() {
        fn files_then_fails(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.tail_lambda(BundValue::lambda(vec![BundValue::int(99)]));
            Err(Error("failed after filing".into()))
        }
        let mut i = Interp::new();
        i.registry.register_native(
            "ff",
            files_then_fails,
            StackEffect::opaque(0),
            WordKind::Sync,
        );
        assert!(i.eval(&[BundValue::call("ff")]).is_err());
        i.eval(&[BundValue::int(1)]).expect("runs");
        assert_eq!(i.depth(), 1, "only the 1: the stale body did not run");
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(1));
    }

    /// RFC-0005 criterion 30's audit half (the eleventh review's B1). A
    /// native that declares a fixed effect and asks to end the program is a
    /// breach: compiled code holding promoted values across it would not see
    /// the stop, which comes at Tier 0's next step. Only `bund.exit` asks, and
    /// it is opaque; this makes that a property.
    #[test]
    fn an_exit_requested_under_a_fixed_effect_is_a_breach() {
        fn exits(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.request_exit(3);
            Ok(())
        }
        let mut i = Interp::new();
        i.registry
            .register_native("ex", exits, StackEffect::fixed(0, 0), WordKind::Sync);
        i.effect_audit = Some(Vec::new());
        let _ = i.eval(&[BundValue::call("ex")]);
        let log = i.effect_audit.take().unwrap_or_default();
        assert!(
            log.iter().any(|l| l.contains("requested an exit")),
            "the audit recorded no breach: {log:?}"
        );
        assert_eq!(i.exit_requested(), Some(3), "the exit itself still stands");
    }

    // --- RFC-0005's seam: where Tier 1 attaches --------------------------

    /// A tier that records every body it is offered and answers as told.
    ///
    /// Recording is the point: two of the seam's clauses are about bodies that
    /// must **never** be offered, and that is invisible from outside.
    struct FakeTier {
        /// The `payload_key` of each body offered, in order.
        offered: std::rc::Rc<std::cell::RefCell<Vec<usize>>>,
        answer: Option<Result<(), Error>>,
        /// Decline, and say so for good through `Vm::tier_never`.
        never: bool,
    }

    impl bund2_api::Tier for FakeTier {
        fn enter(&self, body: &BundValue, vm: &mut dyn Vm) -> Option<Result<(), Error>> {
            if let Some(k) = body.payload_key() {
                self.offered.borrow_mut().push(k);
            }
            if self.never {
                vm.tier_never(body);
                return None;
            }
            // A tier that claims the body leaves its effect behind, so the test
            // can tell "ran by the tier" from "interpreted".
            if matches!(self.answer, Some(Ok(()))) {
                vm.push(BundValue::int(99));
            }
            self.answer.clone()
        }
    }

    fn with_tier(
        answer: Option<Result<(), Error>>,
    ) -> (Interp, std::rc::Rc<std::cell::RefCell<Vec<usize>>>) {
        let offered = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut i = Interp::new();
        i.install_tier(std::rc::Rc::new(FakeTier {
            offered: std::rc::Rc::clone(&offered),
            answer,
            never: false,
        }));
        (i, offered)
    }

    /// **RFC-0003 §S5's table pins nothing and forgets the dead.** Places
    /// noted for bodies nobody holds are swept once the table reaches its
    /// mark, and a body still held keeps its place.
    #[test]
    fn positions_for_dropped_bodies_are_swept() {
        let mut i = Interp::new();
        let kept = BundValue::lambda(vec![BundValue::int(1)]);
        i.note_positions("p.bund", "1\n", &[(kept.clone(), vec![(0, 1)])]);
        for _ in 0..Interp::POSITIONS_SWEEP {
            let gone = BundValue::lambda(vec![BundValue::int(2)]);
            i.note_positions("p.bund", "2\n", &[(gone, vec![(0, 1)])]);
        }
        assert!(i.positions.len() < 16, "swept: {}", i.positions.len());
        let at = i.place_of(&kept, 0).expect("still placed");
        assert_eq!((at.line, at.column, at.span), (1, 1, Some((0, 1))));
    }

    /// **A declining tier changes nothing.** This is the clause that keeps the
    /// seam honest for every build that has no tier: `None` must leave Tier 0
    /// exactly as it was.
    #[test]
    fn a_declining_tier_leaves_interpretation_unchanged() {
        // **A registered lambda called by name**, so the body reaches
        // `push_frame` the way a program does: dispatch resolves it,
        // `request_tail` files it, and `drain_frames` takes it. An earlier
        // version wrote `{ 1 } !` against a bare `Interp`, where `!` is not
        // registered — nothing ran, and the test passed its first assertion
        // while proving nothing. `bund2-interp` cannot dev-depend on
        // `bund2-stdlib` to get `!`, since that crate dev-depends on this one.
        let body = BundValue::lambda(vec![BundValue::int(1)]);
        let (mut with, offered) = with_tier(None);
        with.registry.register_lambda("f", body.clone());
        with.eval(&[BundValue::call("f")]).expect("runs");

        let mut without = Interp::new();
        without.registry.register_lambda("f", body);
        without.eval(&[BundValue::call("f")]).expect("runs");

        assert_eq!(
            with.snapshot().len(),
            without.snapshot().len(),
            "a declined body must be interpreted exactly as with no tier"
        );
        assert!(!offered.borrow().is_empty(), "and it was actually offered");
    }

    /// **A tier is not entered once its registry has been replaced** —
    /// RFC-0005 §S6, the twenty-third review's S6.
    ///
    /// The tier here claims every body and leaves a 99, so "was it entered"
    /// is what is on the stack. The registry is replaced by a clone of itself:
    /// the same words, and different generation cells at different addresses,
    /// which is all a compiled guard would need to be reading freed memory.
    #[test]
    fn a_tier_is_removed_when_its_registry_is_replaced() {
        #[derive(Debug)]
        struct Keeping(Rc<std::cell::RefCell<Vec<String>>>);
        impl bund2_api::diag::Reporter for Keeping {
            fn report(&mut self, d: &bund2_api::diag::Diagnostic) {
                self.0.borrow_mut().push(d.reason.clone());
            }
        }
        let (mut i, offered) = with_tier(Some(Ok(())));
        let said = Rc::new(std::cell::RefCell::new(Vec::new()));
        i.reporter = Box::new(Keeping(Rc::clone(&said)));
        let body = BundValue::lambda(vec![BundValue::int(1)]);

        i.eval_lambda(&body).expect("the tier ran it");
        assert_eq!(i.snapshot().last().and_then(BundValue::as_int), Some(99));
        assert!(
            said.borrow().is_empty(),
            "nothing to say while the registry stands"
        );

        i.registry = i.registry.clone();
        i.eval_lambda(&body).expect("interpreted");
        i.eval_lambda(&body).expect("and again");
        let stack: Vec<_> = i.snapshot().iter().filter_map(BundValue::as_int).collect();
        assert_eq!(stack, [99, 1, 1], "the body's own 1 twice: Tier 0 ran it");
        assert_eq!(
            offered.borrow().len(),
            1,
            "offered before the replacement and never after"
        );
        assert!(i.tier().is_none(), "the tier is gone, not merely skipped");
        let said = said.borrow();
        assert_eq!(said.len(), 1, "said once: {said:?}");
        assert!(said[0].contains("word table was replaced"), "{said:?}");
    }

    /// **D146: a body declined for good is offered once.** The tier says
    /// `Vm::tier_never` of every body, and five entries of one body reach it once;
    /// each of the five is interpreted. A second body is its own question.
    #[test]
    fn a_body_declined_for_good_is_offered_once() {
        let offered = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut i = Interp::new();
        i.install_tier(std::rc::Rc::new(FakeTier {
            offered: std::rc::Rc::clone(&offered),
            answer: None,
            never: true,
        }));
        let body = BundValue::lambda(vec![BundValue::int(1)]);
        let other = BundValue::lambda(vec![BundValue::int(2)]);
        for _ in 0..5 {
            i.eval_lambda(&body).expect("interpreted");
        }
        let stack: Vec<_> = i.snapshot().iter().filter_map(BundValue::as_int).collect();
        assert_eq!(stack, [1, 1, 1, 1, 1], "Tier 0 ran every entry");
        assert_eq!(*offered.borrow(), [body.payload_key().expect("a key")]);

        i.eval_lambda(&other).expect("interpreted");
        i.eval_lambda(&other).expect("interpreted");
        assert_eq!(offered.borrow().len(), 2, "another body is offered, once");
    }

    /// **Declining is not declining for good.** A tier that answers `None`
    /// and says nothing more is asked again on every entry, as it was before D146: that is how a body below
    /// the threshold is counted.
    #[test]
    fn a_body_declined_for_now_is_offered_every_time() {
        let (mut i, offered) = with_tier(None);
        let body = BundValue::lambda(vec![BundValue::int(1)]);
        for _ in 0..5 {
            i.eval_lambda(&body).expect("interpreted");
        }
        assert_eq!(offered.borrow().len(), 5);
    }

    /// **What one tier declined is forgotten with it** — D146. The tier is
    /// taken out and installed again, and the body it had declined for good
    /// is offered to it once more.
    #[test]
    fn what_a_tier_declined_is_forgotten_when_the_tier_changes() {
        let offered = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut i = Interp::new();
        i.install_tier(std::rc::Rc::new(FakeTier {
            offered: std::rc::Rc::clone(&offered),
            answer: None,
            never: true,
        }));
        let body = BundValue::lambda(vec![BundValue::int(1)]);
        i.eval_lambda(&body).expect("interpreted");
        i.eval_lambda(&body).expect("interpreted");
        assert_eq!(offered.borrow().len(), 1);

        let tier = i.take_tier().expect("installed");
        i.install_tier(tier);
        i.eval_lambda(&body).expect("interpreted");
        i.eval_lambda(&body).expect("interpreted");
        assert_eq!(
            offered.borrow().len(),
            2,
            "asked again after the change, once"
        );
    }

    /// **Taking a tier out and installing it again is not a replacement**, and
    /// neither is registering a word: the registry is the same registry.
    #[test]
    fn a_tier_survives_registrations_and_reinstalling() {
        let (mut i, offered) = with_tier(Some(Ok(())));
        let body = BundValue::lambda(vec![BundValue::int(1)]);
        i.registry.register_lambda("f", body.clone());
        i.eval_lambda(&body).expect("the tier ran it");
        let tier = i.take_tier().expect("installed");
        i.install_tier(tier);
        i.eval_lambda(&body).expect("the tier ran it again");
        assert_eq!(offered.borrow().len(), 2);
    }

    /// **`Some(Ok(()))` means compiled code ran it**: no frame is pushed, so the
    /// caller's `run_to` finds nothing to do, and the body's own values never
    /// run — the tier's effect is what is left behind.
    #[test]
    fn a_tier_that_runs_the_body_leaves_no_frame_and_the_body_does_not_run() {
        let (mut i, offered) = with_tier(Some(Ok(())));
        let body = BundValue::lambda(vec![BundValue::int(1)]);
        i.eval_lambda(&body).expect("the tier ran it");

        assert_eq!(offered.borrow().len(), 1, "offered once");
        let stack = i.snapshot();
        assert_eq!(stack.len(), 1, "only the tier's own effect: {stack:?}");
        assert_eq!(
            stack[0].as_int(),
            Some(99),
            "the tier's 99, not the body's 1 — the body must not have run"
        );
        assert_eq!(i.frames.len(), 0, "and no frame was left behind");
    }

    /// **`Some(Err)` is the caller's error**, and leaves no frames above the
    /// floor — the same path an interpreted failure takes.
    #[test]
    fn a_tier_failure_propagates_and_leaves_no_frames() {
        let (mut i, _) = with_tier(Some(Err(Error("the tier failed".into()))));
        let body = BundValue::lambda(vec![BundValue::int(1)]);
        let e = i.eval_lambda(&body).expect_err("the tier's error");
        assert!(e.0.contains("the tier failed"), "{}", e.0);
        assert_eq!(i.frames.len(), 0, "no frames above the floor");
    }

    /// **RFC-0005's assumption 49: an `Interp` stays on the thread that built
    /// it.** Both stack floors are fixed at `Interp::new` from that thread's
    /// declared region, so one moved to another thread would compare a stack
    /// pointer with addresses on a different stack. Nothing checks that at
    /// run time, because safe Rust cannot do it: `Interp` holds `Rc`s and is
    /// not `Send`.
    ///
    /// This fails to **compile** if `Interp` becomes `Send`: both impls of
    /// the helper trait would then apply and the call below is ambiguous.
    #[test]
    fn an_interp_cannot_be_sent_to_another_thread() {
        trait AmbiguousIfSend<A> {
            fn check() {}
        }
        impl<T: ?Sized> AmbiguousIfSend<()> for T {}
        impl<T: ?Sized + Send> AmbiguousIfSend<u8> for T {}
        <Interp as AmbiguousIfSend<_>>::check();
    }

    /// **`scoped_call`'s body is never offered.** It carries
    /// `ExitAction::ToStack`, which must run when the frame leaves (F57) — a
    /// tier running it would leave the stack unrestored. The per-call LIST
    /// does have a `payload_key`, a new one each call, so the exit action is
    /// the whole of the exclusion (RFC-0005 §S3's note of 2026-10-12).
    #[test]
    fn a_scoped_call_body_is_never_offered_to_the_tier() {
        let (mut i, offered) = with_tier(Some(Ok(())));
        i.scoped_call("side", vec![BundValue::int(1)])
            .expect("it ran at Tier 0");
        assert!(
            offered.borrow().is_empty(),
            "a body carrying an exit action must never reach the tier"
        );
        assert_eq!(
            i.stacks.current_name(),
            "main",
            "and the exit action restored the stack"
        );
    }

    /// **Criterion 20's precondition, arriving at a real consumer.** A body run
    /// by a loop word reaches the tier under *one* key — which `entry_log`
    /// already showed for Tier 0, and which the tier now sees for itself.
    #[test]
    fn a_body_run_repeatedly_reaches_the_tier_under_one_key() {
        let (mut i, offered) = with_tier(None);
        let body = BundValue::lambda(vec![BundValue::int(1)]);
        for _ in 0..5 {
            i.eval_lambda(&body).expect("runs");
        }
        let seen = offered.borrow();
        assert_eq!(seen.len(), 5, "offered once per entry");
        assert!(
            seen.windows(2).all(|w| w[0] == w[1]),
            "every entry must carry the same key: {seen:?}"
        );
    }
}
