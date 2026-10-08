//! `cargo xtask layout` — Phase 0 value-representation measurement.
//!
//! RFC-0001's acceptance criteria are written against a size. This measures
//! it, so the RFC is accepted against a number rather than an argument.
//!
//! `BundValue` does not exist yet, so what is measured here are **candidate**
//! representations, declared below. That is the point: the claim "identity
//! moves to the heap header and the value is 16 bytes" is checkable before a
//! line of the real value is written, and the candidate that wins is the
//! shape RFC-0001 specifies.
//!
//! Nothing here imports `reference/`. The reference's own `Value` is
//! characterised by a *replica* — a struct with the same field types — and
//! labelled as such. Depending on the reference crates would pull them into
//! this workspace's build graph, which the project keeps them out of.
//!
//! Allocation counts come from a counting global allocator, read across a
//! window around each operation. They measure the candidate types, not the
//! reference.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------------------------------------------------------------------
// Counting allocator
// ---------------------------------------------------------------------------

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

pub struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(l.size(), Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(new.saturating_sub(l.size()), Ordering::Relaxed);
        unsafe { System.realloc(p, l, new) }
    }
}

/// Allocations and bytes attributable to `f`.
fn measure<T>(f: impl FnOnce() -> T) -> (usize, usize, T) {
    let a0 = ALLOCS.load(Ordering::Relaxed);
    let b0 = BYTES.load(Ordering::Relaxed);
    let out = f();
    let a = ALLOCS.load(Ordering::Relaxed) - a0;
    let b = BYTES.load(Ordering::Relaxed) - b0;
    (a, b, out)
}

// ---------------------------------------------------------------------------
// A replica of the reference's Value, for comparison only
// ---------------------------------------------------------------------------

/// The reference's `Val` discriminant, abbreviated to its widest arms.
/// `reference/rust_dynamic/src/types.rs:66-87`.
#[allow(dead_code)]
enum RefVal {
    I64(i64),
    F64(f64),
    String(String),
    List(Vec<RefValue>),
    Map(HashMap<String, RefValue>),
    ValueMap(HashMap<RefValue, RefValue>),
    Binary(Vec<u8>),
}

/// Field-for-field replica of `reference/rust_dynamic/src/value.rs:15-25`.
/// Not imported — declared, so this crate stays free of reference deps. Same
/// field types means the same layout under the default `repr(Rust)`.
#[allow(dead_code)]
struct RefValue {
    id: String,
    stamp: f64,
    dt: u16,
    q: f64,
    data: RefVal,
    attr: Vec<RefValue>,
    curr: i32,
    tags: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Candidate Bund2 representations
// ---------------------------------------------------------------------------

/// What a heap-allocated value points at. Identity lives here under the
/// header design — which is what the id/stamp scan concluded is possible.
#[allow(dead_code)]
struct Heap {
    /// Lazy identity: zero until something needs it, then minted. Shared
    /// across clones because the `Rc` is, which is what D1 requires, and
    /// `Cell` because equality takes `&self`.
    id: std::cell::Cell<u64>,
    stamp: std::cell::Cell<f64>,
    payload: HeapPayload,
}

#[allow(dead_code)]
enum HeapPayload {
    /// A boxed scalar. RFC-0001's scalar arms are the *unadorned* form; a
    /// scalar that acquires a header — an observed identity, a stamp, a tag,
    /// an attribute — is represented this way. `TS::push` tags
    /// unconditionally (`reference/rust_multistack/src/ts_push.rs:25`), so
    /// every scalar pushed to a stack takes this form, which makes it the
    /// common case rather than an edge one.
    Scalar(CandidateHeader),
    Str(String),
    List(Vec<CandidateHeader>),
    Map(HashMap<String, CandidateHeader>),
}

/// **Candidate A — identity in the heap header.** Scalars carry no identity
/// at all; non-scalars reach it through the `Rc` they already have.
#[derive(Clone)]
#[allow(dead_code)]
enum CandidateHeader {
    Int(i64),
    Float(f64),
    Bool(bool),
    Nodata,
    Heap(Rc<Heap>),
}

/// **Candidate D — A, plus the reference's `dt` type tag.**
///
/// A and B both assume the payload discriminant *is* the type. In the
/// reference it is not: `dt` (`reference/rust_dynamic/src/types.rs:15-56`) and
/// `Val` (`:66-87`) are independent axes. One `Val` arm carries many `dt`
/// tags — `Val::Map` is written with `CLASS`, `CONDITIONAL`, `CONFIG`,
/// `CURRY`, `INFO`, `MAP` and `OBJECT`, and `Val::String` with `CALL`,
/// `CONTEXT`, `JSON_WRAPPED`, `PTR`, `STRING` and `TEXTBUFFER`. `PTR` versus
/// `STRING` is the difference between a name that executes and text that does
/// not, so folding the tag into the payload would lose behaviour.
///
/// The tag is only ever ambiguous for heap types — `NONE`, `BOOL`, `INTEGER`
/// and `FLOAT` are one-to-one with their payloads. So it goes in the heap
/// header beside the identity, not in the value, and the question this
/// measures is whether that keeps the value at 16 bytes.
#[allow(dead_code)]
struct HeapTagged {
    id: std::cell::Cell<u64>,
    stamp: std::cell::Cell<f64>,
    /// The reference's `dt`, verbatim: a `u16` over the constants at
    /// `reference/rust_dynamic/src/types.rs:15-56`.
    dt: u16,
    /// The iteration cursor. `reference/rust_dynamic/src/iter.rs` reads and
    /// writes it for every value type, so it is state, not a constant.
    curr: std::cell::Cell<i32>,
    /// Written on every push — `reference/rust_multistack/src/ts_push.rs:25`
    /// sets the stack tag — so it is never empty on a stack value. That is
    /// what makes `dup` cost more than one allocation.
    tags: std::collections::BTreeMap<String, String>,
    /// Driven by the `attribute` word.
    attr: Vec<CandidateTagged>,
    /// Behind its own `Rc` so `dup` can reset identity while sharing the
    /// payload — see RFC-0001's `dup` section.
    payload: Rc<HeapPayload>,
}

impl HeapTagged {
    /// A stack value: identity unminted, carrying the tag every push writes.
    fn on_stack(payload: HeapPayload) -> Self {
        let mut tags = std::collections::BTreeMap::new();
        tags.insert("stack".to_string(), "main".to_string());
        Self {
            id: std::cell::Cell::new(0),
            stamp: std::cell::Cell::new(0.0),
            dt: 9,
            curr: std::cell::Cell::new(-1),
            tags,
            attr: Vec::new(),
            payload: Rc::new(payload),
        }
    }
}

impl CandidateTagged {
    /// `dup` as RFC-0001 specifies it: a fresh header with a cleared identity,
    /// sharing the payload. Not a bare `Rc` bump — D1's contract is
    /// clone-equal versus dup-unequal.
    fn dup(&self) -> Self {
        match self {
            CandidateTagged::Heap(h) => CandidateTagged::Heap(Rc::new(HeapTagged {
                id: std::cell::Cell::new(0),
                stamp: std::cell::Cell::new(0.0),
                dt: h.dt,
                curr: std::cell::Cell::new(-1),
                tags: h.tags.clone(),
                attr: h.attr.clone(),
                payload: Rc::clone(&h.payload),
            })),
            other => other.clone(),
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
enum CandidateTagged {
    Int(i64),
    Float(f64),
    Bool(bool),
    Nodata,
    Heap(Rc<HeapTagged>),
}

/// **Candidate B — identity inline on every value.** What the representation
/// must look like if `id` and `stamp` cannot move off the value. Carried for
/// contrast: this is the shape the scan rules out.
#[derive(Clone)]
#[allow(dead_code)]
struct CandidateInline {
    id: u64,
    stamp: f64,
    body: CandidateInlineBody,
}

#[derive(Clone)]
#[allow(dead_code)]
enum CandidateInlineBody {
    Int(i64),
    Float(f64),
    Bool(bool),
    Nodata,
    Heap(Rc<Heap>),
}

/// **Candidate C — identity inline, but as a single packed token.** The
/// cheapest inline design: one `u64` covering identity, no separate stamp,
/// stamp derived from it. Shown to bound how much of the cost is the stamp.
#[derive(Clone)]
#[allow(dead_code)]
enum CandidateToken {
    Int(i64, u64),
    Float(f64, u64),
    Bool(bool, u64),
    Nodata(u64),
    Heap(Rc<Heap>, u64),
}

fn row(name: &str, size: usize, align: usize, note: &str) {
    bund2_api::sayln!("  {name:<34}{size:>5}{align:>7}   {note}");
}

pub fn run(_args: &[String]) -> Result<(), String> {
    bund2_api::sayln!("# cargo xtask layout\n");
    bund2_api::sayln!("Phase 0 value-representation measurement. RFC-0001's acceptance");
    bund2_api::sayln!("criteria are written against these numbers.\n");
    bund2_api::sayln!("`BundValue` does not exist yet, so these are CANDIDATE shapes");
    bund2_api::sayln!("declared in xtask/src/layout/mod.rs. Measuring them is what makes");
    bund2_api::sayln!("the 16-byte claim checkable before the RFC commits to it.\n");

    bund2_api::sayln!("## size_of, in bytes\n");
    bund2_api::sayln!(
        "  {:<34}{:>5}{:>7}   note",
        "representation", "size", "align"
    );
    row(
        "reference Value (replica)",
        size_of::<RefValue>(),
        align_of::<RefValue>(),
        "field-for-field, not imported",
    );
    row(
        "A: identity in heap header",
        size_of::<CandidateHeader>(),
        align_of::<CandidateHeader>(),
        "what the id/stamp scan concluded",
    );
    row(
        "B: identity inline (id + stamp)",
        size_of::<CandidateInline>(),
        align_of::<CandidateInline>(),
        "the shape the scan rules out",
    );
    row(
        "C: identity inline, one token",
        size_of::<CandidateToken>(),
        align_of::<CandidateToken>(),
        "cheapest inline; bounds the stamp's cost",
    );
    row(
        "D: A + the reference's dt tag",
        size_of::<CandidateTagged>(),
        align_of::<CandidateTagged>(),
        "dt and Val are independent axes",
    );
    bund2_api::sayln!();
    row(
        "  Rc<Heap>",
        size_of::<Rc<Heap>>(),
        align_of::<Rc<Heap>>(),
        "",
    );
    row(
        "  Heap (pointee)",
        size_of::<Heap>(),
        align_of::<Heap>(),
        "",
    );
    row(
        "  HeapTagged (pointee)",
        size_of::<HeapTagged>(),
        align_of::<HeapTagged>(),
        "carries dt",
    );
    bund2_api::sayln!();

    let a = size_of::<CandidateHeader>();
    let b = size_of::<CandidateInline>();
    if a <= 16 {
        bund2_api::sayln!("  Candidate A is {a} bytes — the scan's conclusion holds.");
    } else {
        bund2_api::sayln!("  Candidate A is {a} bytes, NOT the 16 the scan predicted.");
        bund2_api::sayln!("  RFC-0001 cannot claim 16 without changing the candidate.");
    }
    bund2_api::sayln!(
        "  Carrying identity inline costs {} bytes per value (B - A).\n",
        b - a
    );
    let d = size_of::<CandidateTagged>();
    if d == a {
        bund2_api::sayln!("  Candidate D carries the reference's dt tag and is still {d}");
        bund2_api::sayln!("  bytes: the tag only needs disambiguating for heap types, so");
        bund2_api::sayln!("  it fits in the header beside the identity and costs the value");
        bund2_api::sayln!("  nothing. That is the shape RFC-0001 specifies.\n");
    } else {
        bund2_api::sayln!("  Candidate D is {d} bytes against A's {a} — carrying dt costs");
        bund2_api::sayln!("  {} bytes per value.\n", d - a);
    }
    bund2_api::sayln!("  D4 is RESOLVED: full i64, NaN-boxing not taken. Folding the tag");
    bund2_api::sayln!("  into unused float bits would reach 8 bytes at the cost of 51-bit");
    bund2_api::sayln!("  integers, and the 176 -> 16 win is already banked. These figures");
    bund2_api::sayln!("  are the full-i64 ones the decision rests on.\n");

    // -----------------------------------------------------------------------
    bund2_api::sayln!("## allocations per operation\n");
    bund2_api::sayln!("  Counted with a global allocator across a window around each");
    bund2_api::sayln!("  operation. Candidate types only — the reference is not linked.\n");
    bund2_api::sayln!("  {:<44}{:>7}{:>9}", "operation", "allocs", "bytes");

    // Warm any lazily-initialised machinery before measuring.
    let _ = measure(|| {
        Rc::new(Heap {
            id: std::cell::Cell::new(0),
            stamp: std::cell::Cell::new(0.0),
            payload: HeapPayload::Str(String::from("warm")),
        })
    });

    let (n, b, scalar) = measure(|| CandidateHeader::Int(7));
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "A: construct a scalar");
    let (n, b, _) = measure(|| scalar.clone());
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "A: clone a scalar");

    let (n, b, heap) = measure(|| {
        CandidateHeader::Heap(Rc::new(Heap {
            id: std::cell::Cell::new(0),
            stamp: std::cell::Cell::new(0.0),
            payload: HeapPayload::List(vec![CandidateHeader::Int(1)]),
        }))
    });
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "A: construct a 1-element list");
    let (n, b, _) = measure(|| heap.clone());
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "A: clone a list (Rc bump)");

    // Candidate D is the shape RFC-0001 specifies, so its rows are the ones
    // the acceptance criteria are written against. A and B remain for contrast.
    let (n, b, tagged) = measure(|| CandidateTagged::Int(7));
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "D: construct a scalar");
    let (n, b, _) = measure(|| tagged.clone());
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "D: clone a scalar");

    let (n, b, dheap) = measure(|| {
        CandidateTagged::Heap(Rc::new(HeapTagged::on_stack(HeapPayload::List(vec![
            CandidateHeader::Int(1),
        ]))))
    });
    bund2_api::sayln!(
        "  {:<44}{n:>7}{b:>9}",
        "D: construct a 1-element list on a stack"
    );
    let (n, b, _) = measure(|| dheap.clone());
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "D: clone a list (Rc bump)");
    let (n, b, _) = measure(|| dheap.dup());
    bund2_api::sayln!(
        "  {:<44}{n:>7}{b:>9}",
        "D: dup a list (fresh header, shared payload)"
    );
    let (n, b, _) = measure(|| scalar.clone());
    bund2_api::sayln!(
        "  {:<44}{n:>7}{b:>9}",
        "D: dup an unadorned scalar (no header)"
    );

    // The case that actually occurs: `push` tags unconditionally, so a scalar
    // on a stack is a boxed scalar. Criterion 2's "unadorned" qualifier is
    // load-bearing precisely because of this row.
    let (n, b, boxed) = measure(|| {
        CandidateTagged::Heap(Rc::new(HeapTagged::on_stack(HeapPayload::Scalar(
            CandidateHeader::Int(7),
        ))))
    });
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "D: box a scalar (what push forces)");
    let (n, b, _) = measure(|| boxed.clone());
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "D: clone a boxed scalar (Rc bump)");
    let (n, b, _) = measure(|| boxed.dup());
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "D: dup a boxed scalar");

    let (n, b, inline) = measure(|| CandidateInline {
        id: 0,
        stamp: 0.0,
        body: CandidateInlineBody::Int(7),
    });
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "B: construct a scalar");
    let (n, b, _) = measure(|| inline.clone());
    bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "B: clone a scalar");
    bund2_api::sayln!();

    bund2_api::sayln!("  An UNADORNED scalar allocating zero is the property that");
    bund2_api::sayln!("  matters, and the qualifier is the point: `push` writes a stack");
    bund2_api::sayln!("  tag with no type test, so a scalar on a stack is boxed and the");
    bund2_api::sayln!("  `box a scalar` row above is what a pushed integer costs.");
    bund2_api::sayln!("  RFC-0001 criterion 2 is written against the unadorned row and");
    bund2_api::sayln!("  says so; criterion 3 is written against the boxed ones.\n");
    bund2_api::sayln!("  Historical note: under");
    bund2_api::sayln!("  the header design an integer never touches the heap, so the");
    bund2_api::sayln!("  lazy identity D1 specifies costs nothing until observed.\n");

    // -----------------------------------------------------------------------
    // RFC-0002 criterion 3. Until `bund2-interp` existed there was no VM to
    // dispatch in, and the criterion was listed with no tool that could run
    // it — which RFC-0002's second review recorded.
    bund2_api::sayln!("## allocations per dispatch  (RFC-0002 criterion 3)\n");
    {
        use bund2_api::{StackEffect, Vm, WordKind};
        use bund2_interp::Interp;
        use bund2_value::BundValue;

        fn nop(_: &mut dyn Vm) -> Result<(), bund2_api::Error> {
            Ok(())
        }
        fn pusher(vm: &mut dyn Vm) -> Result<(), bund2_api::Error> {
            vm.push(BundValue::int(1));
            Ok(())
        }
        let eff = StackEffect::fixed(0, 0);

        let mut i = Interp::new();
        let plain = i.registry.register_native("w", nop, eff, WordKind::Sync);
        i.registry.register_alias("b", "w");
        let aliased = i.registry.register_alias("a", "b");
        let pushes = i.registry.register_native("p", pusher, eff, WordKind::Sync);

        // Warm anything lazily initialised before measuring.
        let _ = measure(|| i.dispatch(plain, false));

        let (n, b, _) = measure(|| i.dispatch(plain, false));
        bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "dispatch a native by Symbol");
        let (n, b, _) = measure(|| i.dispatch(aliased, false));
        bund2_api::sayln!("  {:<44}{n:>7}{b:>9}", "dispatch through a two-link alias");
        let (n, b, _) = measure(|| i.dispatch(pushes, false));
        bund2_api::sayln!(
            "  {:<44}{n:>7}{b:>9}",
            "dispatch a word that pushes a scalar"
        );
        let (n, b, _) = measure(|| i.dispatch_name("w"));
        bund2_api::sayln!(
            "  {:<44}{n:>7}{b:>9}",
            "dispatch by name (lookup, no intern)"
        );
        bund2_api::sayln!();
        bund2_api::sayln!("  The reference allocates **thirteen** strings and hashes");
        bund2_api::sayln!("  eight times to dispatch `dup`, itemised per call site in");
        bund2_api::sayln!("  RFC-0002's Motivation. Dispatch by Symbol is the whole");
        bund2_api::sayln!("  point, and the alias row shows the chain costs nothing");
        bund2_api::sayln!("  extra because a link is an index.\n");
        bund2_api::sayln!("  The pushing row is not zero, and that is RFC-0001's");
        bund2_api::sayln!("  boxing cost, not a dispatch cost: `TS::push` tags every");
        bund2_api::sayln!("  value with no type test, so a scalar reaching a stack is");
        bund2_api::sayln!("  boxed. RFC-0003 may move the tag into the stack slot.\n");
    }

    bund2_api::sayln!("  What this does NOT measure: the reference's own allocation");
    bund2_api::sayln!("  behaviour, since reference/ is deliberately not linked into");
    bund2_api::sayln!("  this workspace. For that, `cargo xtask bench` times the oracle");
    bund2_api::sayln!("  end to end instead.\n");

    Ok(())
}
