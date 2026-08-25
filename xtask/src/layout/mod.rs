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
/// `reference/rust_dynamic/src/types.rs:60-90`.
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
    println!("  {name:<34}{size:>5}{align:>7}   {note}");
}

pub fn run(_args: &[String]) -> Result<(), String> {
    println!("# cargo xtask layout\n");
    println!("Phase 0 value-representation measurement. RFC-0001's acceptance");
    println!("criteria are written against these numbers.\n");
    println!("`BundValue` does not exist yet, so these are CANDIDATE shapes");
    println!("declared in xtask/src/layout/mod.rs. Measuring them is what makes");
    println!("the 16-byte claim checkable before the RFC commits to it.\n");

    println!("## size_of, in bytes\n");
    println!(
        "  {:<34}{:>5}{:>7}   {}",
        "representation", "size", "align", "note"
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
    println!();
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
    println!();

    let a = size_of::<CandidateHeader>();
    let b = size_of::<CandidateInline>();
    if a <= 16 {
        println!("  Candidate A is {a} bytes — the scan's conclusion holds.");
    } else {
        println!("  Candidate A is {a} bytes, NOT the 16 the scan predicted.");
        println!("  RFC-0001 cannot claim 16 without changing the candidate.");
    }
    println!(
        "  Carrying identity inline costs {} bytes per value (B - A).\n",
        b - a
    );
    println!("  D4 is still OPEN. NaN-boxing would shrink A further by folding");
    println!("  the tag into unused float bits, at the cost of 51-bit integers.");
    println!("  These figures assume full i64 — D4's default.\n");

    // -----------------------------------------------------------------------
    println!("## allocations per operation\n");
    println!("  Counted with a global allocator across a window around each");
    println!("  operation. Candidate types only — the reference is not linked.\n");
    println!("  {:<44}{:>7}{:>9}", "operation", "allocs", "bytes");

    // Warm any lazily-initialised machinery before measuring.
    let _ = measure(|| {
        Rc::new(Heap {
            id: std::cell::Cell::new(0),
            stamp: std::cell::Cell::new(0.0),
            payload: HeapPayload::Str(String::from("warm")),
        })
    });

    let (n, b, scalar) = measure(|| CandidateHeader::Int(7));
    println!("  {:<44}{n:>7}{b:>9}", "A: construct a scalar");
    let (n, b, _) = measure(|| scalar.clone());
    println!("  {:<44}{n:>7}{b:>9}", "A: clone a scalar");

    let (n, b, heap) = measure(|| {
        CandidateHeader::Heap(Rc::new(Heap {
            id: std::cell::Cell::new(0),
            stamp: std::cell::Cell::new(0.0),
            payload: HeapPayload::List(vec![CandidateHeader::Int(1)]),
        }))
    });
    println!("  {:<44}{n:>7}{b:>9}", "A: construct a 1-element list");
    let (n, b, _) = measure(|| heap.clone());
    println!("  {:<44}{n:>7}{b:>9}", "A: clone a list (Rc bump)");

    let (n, b, inline) = measure(|| CandidateInline {
        id: 0,
        stamp: 0.0,
        body: CandidateInlineBody::Int(7),
    });
    println!("  {:<44}{n:>7}{b:>9}", "B: construct a scalar");
    let (n, b, _) = measure(|| inline.clone());
    println!("  {:<44}{n:>7}{b:>9}", "B: clone a scalar");
    println!();

    println!("  A scalar allocating zero is the property that matters: under");
    println!("  the header design an integer never touches the heap, so the");
    println!("  lazy identity D1 specifies costs nothing until observed.\n");

    println!("  What this does NOT measure: the reference's own allocation");
    println!("  behaviour, since reference/ is deliberately not linked into");
    println!("  this workspace. For that, `cargo xtask bench` times the oracle");
    println!("  end to end instead.\n");

    Ok(())
}
