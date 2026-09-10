//! In-process benchmarks. **Q14's answer, made runnable.**
//!
//! # Why this exists rather than `cargo xtask bench`
//!
//! `xtask bench` times `bund2 script --file …` end to end. Measured that way
//! the corpus is 153.9 ms for 57 programs, of which the fastest program —
//! 2.3 ms — is spawn plus registration and nothing else. So roughly 0.4 ms of a
//! 2.7 ms mean run is interpretation, and three consecutive whole-harness runs
//! spread the total across 153.9–162.2 ms. **The run-to-run noise is the same
//! order as the entire quantity of interest**, which is why the corpus cannot
//! carry a performance criterion for RFC-0001 or RFC-0005 and this file has to.
//!
//! # The one rule
//!
//! **Parse and register outside the timed region; time only what is being
//! claimed about.** Every `iter_batched` below builds its `Interp` in the setup
//! closure, so registration — the dominant cost end to end — is excluded from
//! every group except the one that measures registration on purpose.
//!
//! Getting that wrong reproduces the problem this file exists to escape: a
//! "20% faster" that is 80% stdlib registration.
//!
//! # What a JIT would and would not touch
//!
//! Grouped so RFC-0005 can cite the ones its lowering claims to affect:
//!
//! - `dispatch` and `arith` are the tight loops a JIT exists for.
//! - `lambda` is call overhead — RFC-0003's frame loop, and what tail calls
//!   and inline caches act on.
//! - `parse` and `registry` are startup. **A JIT must not move these**, and a
//!   change here is a regression in something else.
//! - `corpus` is a real program end to end, minus the process. It is the
//!   honest headline number: whatever a JIT claims, it has to show up here.

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use bund2_api::Vm as _;
use bund2_interp::Interp;
use bund2_value::BundValue;

/// A registry with the full stdlib, which is what a real run has.
fn interp() -> Interp {
    let mut i = Interp::new();
    bund2_stdlib::register_all(&mut i.registry);
    i
}

/// Parse once, here, so no benchmark below pays for it.
fn compiled(src: &str) -> Vec<BundValue> {
    match bund2_syntax::compile(src) {
        Ok(s) => s,
        // A bench harness may not panic under this workspace's lints, and a
        // benchmark over a program that does not compile would report a
        // number for nothing. Report and measure an empty stream instead —
        // the group will read as ~0 and the message says why.
        Err(e) => {
            eprintln!("bench: source failed to compile, measuring nothing: {e:?}");
            Vec::new()
        }
    }
}

/// Registration and parsing — the fixed cost. A JIT must leave these alone.
fn startup(c: &mut Criterion) {
    let mut g = c.benchmark_group("startup");

    // The single biggest cost in an end-to-end run: 2.3 ms of the 2.7 ms mean
    // is this plus process spawn.
    g.bench_function("registry/register_all", |b| {
        b.iter_batched(
            Interp::new,
            |mut i| {
                bund2_stdlib::register_all(&mut i.registry);
                black_box(i.registry.word_names().len())
            },
            BatchSize::SmallInput,
        );
    });

    let src = include_str!("../programs/mixed.bund");
    g.bench_function("parse/mixed", |b| {
        b.iter(|| black_box(bund2_syntax::compile(black_box(src))));
    });

    g.finish();
}

/// One `Interp` per iteration, built in setup so only `eval` is timed.
fn timed_eval(g: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>, name: &str, src: &str) {
    let stream = compiled(src);
    g.bench_function(name, |b| {
        b.iter_batched(
            interp,
            |mut i| black_box(i.eval(black_box(&stream))).is_ok(),
            BatchSize::SmallInput,
        );
    });
}

/// Word dispatch: the same call, many times. What an inline cache acts on.
///
/// **These decide the study's §2.2 gate**, which says a Cranelift tier is worth
/// building "only if Project A's measurements show that dispatch and boxing are
/// still the bottleneck". Each name ends in the number of *words* the program
/// executes, so the reported time divides straight into nanoseconds per word —
/// the figure the gate turns on. See RFC-0005 §S1.
fn dispatch(c: &mut Criterion) {
    let mut g = c.benchmark_group("dispatch");
    // 3000 words: `1`, `dup`, `drop`.
    timed_eval(&mut g, "dup_drop/w3000", &"1 dup drop ".repeat(1000));
    // 4000 words: `1`, `2`, `+`, `drop`.
    timed_eval(&mut g, "native_call/w4000", &"1 2 + drop ".repeat(1000));
    // **1000 literals and no CALL at all.** This is the non-dispatch path
    // exactly: the eval loop walks the stream and pushes each value, and no
    // word runs. Subtracting it from a program of the same length that *does*
    // call gives an upper bound on what dispatch could cost.
    //
    // It replaces a `nl`-based benchmark that was meant to isolate dispatch
    // and instead measured stdout: `nl` writes a newline, so it timed the
    // terminal at ~450 ns per word. That number appeared in no table, which is
    // the only reason it misled nobody.
    timed_eval(&mut g, "literal_only/w1000", &"1 ".repeat(1000));
    // 2000 words that only push a literal — no dispatch to a native at all.
    timed_eval(&mut g, "literal_push/w2000", &"1 drop ".repeat(1000));
    g.finish();
}

/// The value layer under a push. **The study's §2.2 gate turns on this.**
///
/// `Vm::push` does not store the value it is given: it stores
/// `v.with_tag("stack", name)` (`crates/bund2-interp/src/lib.rs`), and
/// `with_tag` boxes the scalar, clones the whole `HeapValue` including its
/// `BTreeMap` of tags, inserts two freshly allocated `String`s, and wraps the
/// result in a new `Rc` (`crates/bund2-value/src/lib.rs`).
///
/// So the question "is dispatch the bottleneck, or is it representation?" is
/// answerable by timing that one call against a whole interpreted word.
fn value(c: &mut Criterion) {
    let mut g = c.benchmark_group("value");
    // **The production case.** `Vm::push` owns the value it tags, and a
    // literal off the stream is a scalar, so boxing it yields a fresh `Rc`
    // with one holder and `with_tag` takes its in-place branch.
    g.bench_function("with_tag/scalar_unique", |b| {
        // Interned once, as `Stack` caches them — measuring `Rc::from` here
        // would measure the allocation the interning exists to remove.
        let k: std::rc::Rc<str> = std::rc::Rc::from("stack");
        let n: std::rc::Rc<str> = std::rc::Rc::from("main");
        b.iter(|| {
            black_box(
                black_box(BundValue::int(42))
                    .with_tag(std::rc::Rc::clone(&k), std::rc::Rc::clone(&n)),
            )
        });
    });
    // The split branch: a heap value someone else still holds. D13 requires a
    // materialise-then-clone here, and this is what that costs.
    g.bench_function("with_tag/heap_shared", |b| {
        let keep = BundValue::list(vec![BundValue::int(1)]);
        let k: std::rc::Rc<str> = std::rc::Rc::from("stack");
        let n: std::rc::Rc<str> = std::rc::Rc::from("main");
        b.iter(|| {
            black_box(
                black_box(&keep)
                    .clone()
                    .with_tag(std::rc::Rc::clone(&k), std::rc::Rc::clone(&n)),
            )
        });
    });
    // Boxing alone, with no tag written. If this is most of `with_tag`, the
    // cost is the *split representation*, not the tagging.
    g.bench_function("promote/scalar", |b| {
        b.iter(|| black_box(black_box(BundValue::int(42)).promote()));
    });
    g.bench_function("clone/scalar", |b| {
        let v = BundValue::int(42);
        b.iter(|| black_box(black_box(&v).clone()));
    });
    // **Balanced, on one `Interp`.** An `iter_batched` version of this read
    // 8.6 µs, which is not a push — it was timing the *drop* of a registry
    // holding 261 words, because teardown of the batch falls inside the
    // measured region. Push and pull cancel, so the stack does not grow and a
    // single long-lived interpreter is correct here.
    g.bench_function("push_pull/balanced", |b| {
        let mut i = interp();
        b.iter(|| {
            i.push(black_box(BundValue::int(42)));
            black_box(i.pull())
        });
    });
    g.finish();
}

/// Arithmetic, the classic JIT target: no allocation, all in the value layer.
fn arith(c: &mut Criterion) {
    let mut g = c.benchmark_group("arith");
    timed_eval(&mut g, "int_add/1000", &format!("0 {}", "1 + ".repeat(1000)));
    timed_eval(&mut g, "float_mul/1000", &format!("1.0 {}", "1.000001 * ".repeat(1000)));
    // `times` runs a body N times through the interpreter's own loop, which is
    // closer to how a real program spends its time than a straight-line stream.
    timed_eval(&mut g, "times_body/1000", "1000 { 1 + } times drop");
    g.finish();
}

/// Call overhead — RFC-0003's frame loop, and what tail calls act on.
fn lambda(c: &mut Criterion) {
    let mut g = c.benchmark_group("lambda");
    timed_eval(
        &mut g,
        "register_and_call/500",
        &format!(":inc {{ 1 + }} register 0 {}drop", "inc ".repeat(500)),
    );
    timed_eval(&mut g, "apply_lambda/500", &"{ 1 } apply drop ".repeat(500));
    g.finish();
}

/// Real programs, end to end minus the process. The honest headline.
///
/// The last two are corpus programs that Bund2 runs clean, included from
/// `reference/` so they cannot drift from the thing being conformed against.
/// **They are read, never written** — the same rule the rest of the tree
/// follows for that directory.
///
/// The point of measuring them rather than only `mixed.bund` is that the
/// synthetic one was written to exercise a JIT's targets, and a program written
/// to be fast to a JIT is not evidence about the corpus.
fn corpus(c: &mut Criterion) {
    let mut g = c.benchmark_group("corpus");
    timed_eval(&mut g, "mixed", include_str!("../programs/mixed.bund"));
    timed_eval(
        &mut g,
        "sequence_generate_2",
        include_str!("../../../reference/Bund/examples/sequence_generate_2.bund"),
    );
    timed_eval(
        &mut g,
        "sorting_numbers_in_list",
        include_str!("../../../reference/Bund/tests/testing_sorting_numbers_in_list.bund"),
    );
    g.finish();
}

/// **Not interpretation.** `display` renders markdown through termimad, which
/// measures the terminal and lays text out; `pull_demo` is the corpus program
/// that reaches it.
///
/// It is kept, in its own group and under its own name, because the number is
/// startling and worth having on record: **9.5 ms against 12 µs** for the two
/// corpus programs beside it — roughly *eight hundred times* a whole ordinary
/// program, for one rendered table. Folded into `corpus` it would have been
/// 99.7% of that group's total and any JIT claim measured there would have been
/// a claim about termimad.
///
/// That is the mistake this file's header warns about, made and caught: the
/// first version of the `corpus` group included this program.
///
/// Note also that every corpus program writes to stdout, so all of them carry
/// some I/O. `mixed.bund` is deliberately silent and is the clean
/// interpretation number.
fn rendering(c: &mut Criterion) {
    let mut g = c.benchmark_group("rendering");
    g.sample_size(20);
    timed_eval(
        &mut g,
        "display_via_termimad/pull_demo",
        include_str!("../../../reference/Bund/examples/code_snippets/pull_demo.bund"),
    );
    g.finish();
}

criterion_group!(benches, startup, value, dispatch, arith, lambda, corpus, rendering);
criterion_main!(benches);
