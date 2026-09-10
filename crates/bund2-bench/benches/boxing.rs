//! **Q29's experiment: what does the second allocation cost?**
//!
//! Boxing a scalar measures 26.0 ns (`interpret.rs`, `value/promote/scalar`)
//! and costs two allocations, because `HeapValue` holds
//! `payload: Rc<Payload>` — a second indirection that exists so `dup` can
//! share a payload, and that **D35 depends on**, since the compiled-code cache
//! keys on the payload pointer.
//!
//! A *scalar* payload is never worth sharing, so option C of RFC-0001's Q25
//! amendment proposes storing it inline in the header. The arithmetic in that
//! recommendation — "halving 26 ns" — was an estimate. This measures it.
//!
//! Both shapes replicate `HeapValue`'s field set exactly
//! (`crates/bund2-value/src/lib.rs`), so the difference is the second
//! allocation and nothing else. A stand-in payload enum is used rather than
//! the real one because the real one is private; its scalar arm is the same
//! shape.

use criterion::{Criterion, criterion_group, criterion_main};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::hint::black_box;
use std::rc::Rc;

/// Stand-in for `Payload`. Only the scalar arm matters here; the others are
/// present so the enum's size and discriminant behave like the real one.
#[derive(Debug)]
#[allow(dead_code)]
enum Payload {
    Scalar(i64),
    Str(String),
    List(Vec<i64>),
}

/// Today: the payload behind its own `Rc`. Two allocations.
///
/// **Every field is dead and every field is the point.** These structs
/// replicate `HeapValue`'s field set exactly so that the only difference
/// between the two benchmarks is the second allocation; removing an unread
/// field would change the size being allocated and quietly invalidate the
/// comparison.
#[derive(Debug)]
#[allow(dead_code)]
struct Shared {
    identity: Cell<u64>,
    stamp: Cell<f64>,
    dt: u16,
    q: f64,
    curr: i32,
    tags: BTreeMap<String, String>,
    attr: Vec<i64>,
    payload: Rc<Payload>,
}

/// Option C: the payload inline. One allocation. Fields dead for the same
/// reason as `Shared` above.
#[derive(Debug)]
#[allow(dead_code)]
struct Inline {
    identity: Cell<u64>,
    stamp: Cell<f64>,
    dt: u16,
    q: f64,
    curr: i32,
    tags: BTreeMap<String, String>,
    attr: Vec<i64>,
    payload: Payload,
}

fn boxing(c: &mut Criterion) {
    let mut g = c.benchmark_group("boxing");

    g.bench_function("two_allocations/payload_behind_rc", |b| {
        b.iter(|| {
            black_box(Rc::new(Shared {
                identity: Cell::new(0),
                stamp: Cell::new(0.0),
                dt: black_box(2),
                q: 100.0,
                curr: -1,
                tags: BTreeMap::new(),
                attr: Vec::new(),
                payload: Rc::new(Payload::Scalar(black_box(42))),
            }))
        });
    });

    g.bench_function("one_allocation/payload_inline", |b| {
        b.iter(|| {
            black_box(Rc::new(Inline {
                identity: Cell::new(0),
                stamp: Cell::new(0.0),
                dt: black_box(2),
                q: 100.0,
                curr: -1,
                tags: BTreeMap::new(),
                attr: Vec::new(),
                payload: Payload::Scalar(black_box(42)),
            }))
        });
    });

    // The tag insert that sits on top of either, for scale: two `String`
    // allocations and a `BTreeMap` insert.
    g.bench_function("tag_insert/two_strings", |b| {
        b.iter(|| {
            let mut m: BTreeMap<String, String> = BTreeMap::new();
            m.insert(black_box("stack").to_string(), black_box("main").to_string());
            black_box(m)
        });
    });

    // The same insert with interning — option 2 of the amendment — for the
    // combined estimate.
    g.bench_function("tag_insert/interned", |b| {
        let k: Rc<str> = Rc::from("stack");
        let v: Rc<str> = Rc::from("main");
        b.iter(|| {
            let mut m: BTreeMap<Rc<str>, Rc<str>> = BTreeMap::new();
            m.insert(Rc::clone(black_box(&k)), Rc::clone(black_box(&v)));
            black_box(m)
        });
    });

    g.finish();
}

criterion_group!(benches, boxing);
criterion_main!(benches);
