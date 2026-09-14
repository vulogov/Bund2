//! **Q33's prototype: what would a BundIR fragment actually buy?**
//!
//! RFC-0005 §S6 proposes that a word publish a BundIR fragment so the JIT can
//! inline it, because a value held in a Cranelift `Variable` is invisible to a
//! word called through `fn(&mut dyn Vm)`. §S6 also says the question of
//! *whether that is worth building* is open, and that two or three fragments
//! should be prototyped and measured before the rest are written.
//!
//! # What this measures, and what it cannot
//!
//! There is no Cranelift lowering to benchmark. What can be measured today is
//! the **ceiling** — how much work a perfect lowering of each fragment would
//! remove — and four shapes of one program say it:
//!
//! | shape | what it stands for |
//! |---|---|
//! | `tier0` | today: every word dispatched, every intermediate a `BundValue` |
//! | `inlined` | the fragment run by `frag::run` — **the model, carrying an interpreter's overhead** |
//! | `lowered` | the fragment's own ops written out in Rust — **the ceiling for inlining** |
//! | `promoted` | the arm inlined *and* intermediates in registers — **the ceiling for promotion** |
//!
//! `tier0 → lowered` is the most inlining can buy. `lowered → promoted` is
//! what promotion adds on top. `inlined` against `lowered` is what interpreting
//! a fragment costs rather than compiling it — which is why `inlined` is not
//! the ceiling, and why an earlier version of this file, which reported one
//! hand-optimised column under that name, was wrong in two directions at once:
//! it folded the literal into the arm, which no fragment can express, and it
//! skipped the guard.
//!
//! §S1 says dispatch and work cannot be separated by subtracting benchmarks;
//! here they are not subtracted but constructed, in one harness against one
//! program.
//!
//! **`promoted` is an upper bound, not a prediction.** Real compiled code pays
//! entry and exit, a type guard per specialised region, and a sync at every
//! opaque site (§S5). A fragment cannot beat this number and will not reach it.
//! If the ceiling is close to `tier0`, the answer to Q33 is no and no lowering
//! needs writing to find that out.
//!
//! # The arm
//!
//! `Int + Int → Int`, the smallest useful fragment. It needs no `q`
//! arithmetic: a scalar's `q` is the 100.0 fixpoint, answered by `q` in
//! `crates/bund2-value/src/lib.rs`.
//!
//! And the addition path never writes one — `numeric_op` in
//! `crates/bund2-stdlib/src/math.rs` does not touch `q` on this arm — so the
//! fragment is the addition and nothing else. A float or boxed arm would owe
//! no `q` average either: D32, amended on Q35's answer, says no word averages
//! it.

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use bund2_api::{Error, StackEffect, Vm, WordKind};
use bund2_interp::{Interp, frag};
use bund2_value::BundValue;

/// How many additions in the chain. A hot loop, not a toy.
const N: usize = 1000;

fn interp() -> Interp {
    let mut i = Interp::new();
    bund2_stdlib::register_all(&mut i.registry);
    i
}

fn fragment(c: &mut Criterion) {
    let mut g = c.benchmark_group("fragment");

    // --- tier0: `0 1 + 1 + …`, every word dispatched -------------------
    let src = format!("0 {}", "1 + ".repeat(N));
    let stream = bund2_syntax::compile(&src).unwrap_or_default();
    g.bench_function("int_add/tier0", |b| {
        b.iter_batched(
            interp,
            |mut i| black_box(i.eval(black_box(&stream))).is_ok(),
            BatchSize::SmallInput,
        );
    });

    // --- inlined: the fragment, as `frag::run` executes it --------------
    //
    // The model, run as the model runs: guard through indexed peeks, then an
    // op loop with a match per op. It carries an interpreter's overhead that a
    // lowering would not, so it is **not** the inlining ceiling — `lowered`
    // below is. What it answers is whether interpreting a fragment is already
    // a win over dispatching the word, and how far short of `lowered` it falls.
    //
    // **This runs the real fragment.** An earlier version of this column
    // hand-wrote `pull(); push(int(a + 1))` — one pull, one push, with the
    // literal `1` folded into the arm as a constant. That is a shape the
    // representation cannot express: `Op` has no immediate operand, so a
    // fragment *cannot* fold a constant, and `fragments::int_add()` is
    // `PopInt, PopInt, AddInt, PushInt` behind `Guard::TopAreInt(2)`. It also
    // skipped the guard entirely. The column reported 16.3 ns for a program
    // that was not the fragment and could not be.
    //
    // So: push the literal the `tier0` program pushes, then enter the fragment
    // through `frag::run` — guard, peeks and all.
    let f_add = bund2_stdlib::fragments::int_add();
    assert!(f_add.is_ok(), "the measured fragment must be well-formed: {f_add:?}");
    let Ok(f_add) = f_add else { return };
    g.bench_function("int_add/inlined", |b| {
        b.iter_batched(
            || {
                let mut i = interp();
                i.push(BundValue::int(0));
                i
            },
            |mut i| {
                for _ in 0..N {
                    i.push(BundValue::int(1));
                    let _ = black_box(frag::run(&mut i, black_box(&f_add)));
                }
                black_box(i.pull())
            },
            BatchSize::SmallInput,
        );
    });

    // --- lowered: the fragment's own ops, and nothing else --------------
    //
    // **The inlining ceiling.** Exactly what `fragments::int_add()` says — the
    // literal pushed, `TopAreInt(2)` asked through indexed peeks, two pulls, a
    // wrapping add, one push — with none of `frag::run`'s interpretation: no op
    // loop, no match per op. It is what a perfect lowering of *this* fragment
    // would execute, minus the entry, exit and helper calls compiled code pays,
    // and it contains nothing a fragment cannot express. The literal is pushed
    // and pulled, not folded.
    g.bench_function("int_add/lowered", |b| {
        b.iter_batched(
            || {
                let mut i = interp();
                i.push(BundValue::int(0));
                i
            },
            |mut i| {
                for _ in 0..N {
                    i.push(BundValue::int(1));
                    let admitted = matches!(i.peek_at(0), Some(BundValue::Int(_, _)))
                        && matches!(i.peek_at(1), Some(BundValue::Int(_, _)));
                    if admitted {
                        let top = i.pull().and_then(|v| v.as_int()).unwrap_or(0);
                        let below = i.pull().and_then(|v| v.as_int()).unwrap_or(0);
                        i.push(BundValue::int(below.wrapping_add(top)));
                    }
                }
                black_box(i.pull())
            },
            BatchSize::SmallInput,
        );
    });

    // --- promoted: the §S6 ideal ---------------------------------------
    //
    // The chain's intermediates live in a register. One guard on entry, one
    // materialisation on exit. This is what stack-slot promotion means, and
    // it is the ceiling: real code adds entry/exit, the guard branch, and a
    // sync wherever the region ends.
    g.bench_function("int_add/promoted", |b| {
        b.iter_batched(
            || {
                let mut i = interp();
                i.push(BundValue::int(0));
                i
            },
            |mut i| {
                let mut acc = i.pull().and_then(|v| v.as_int()).unwrap_or(0);
                for _ in 0..N {
                    acc += black_box(1);
                }
                i.push(BundValue::int(acc));
                black_box(i.pull())
            },
            BatchSize::SmallInput,
        );
    });

    // --- a second arm: `dup drop`, pure stack traffic ------------------
    //
    // Chosen because it is where promotion should look best: promoted, a
    // `dup` is a register copy whose result is immediately dead, so the pair
    // compiles to nothing at all. If promotion's advantage does not show here
    // it does not exist anywhere.
    let src2 = format!("1 {}", "dup drop ".repeat(N));
    let stream2 = bund2_syntax::compile(&src2).unwrap_or_default();
    g.bench_function("dup_drop/tier0", |b| {
        b.iter_batched(
            interp,
            |mut i| black_box(i.eval(black_box(&stream2))).is_ok(),
            BatchSize::SmallInput,
        );
    });
    let (f_dup, f_drop) = (
        bund2_stdlib::fragments::dup(),
        bund2_stdlib::fragments::drop_top(),
    );
    assert!(f_dup.is_ok() && f_drop.is_ok(), "{f_dup:?} {f_drop:?}");
    let (Ok(f_dup), Ok(f_drop)) = (f_dup, f_drop) else { return };
    g.bench_function("dup_drop/inlined", |b| {
        b.iter_batched(
            || {
                let mut i = interp();
                i.push(BundValue::int(1));
                i
            },
            |mut i| {
                for _ in 0..N {
                    // Both arms through `frag::run`, guards included — the same
                    // correction as `int_add/inlined` above. The hand-written
                    // version also used `.dup()` rather than a clone, because
                    // `dup` mints a fresh identity (F13) and a clone does not;
                    // that fix is now inside `Op::DupTop` where the differential
                    // test can see it.
                    let _ = black_box(frag::run(&mut i, black_box(&f_dup)));
                    let _ = black_box(frag::run(&mut i, black_box(&f_drop)));
                }
                black_box(i.pull())
            },
            BatchSize::SmallInput,
        );
    });
    // The same ceiling for `dup` then `drop`: each guard, then each arm.
    g.bench_function("dup_drop/lowered", |b| {
        b.iter_batched(
            || {
                let mut i = interp();
                i.push(BundValue::int(1));
                i
            },
            |mut i| {
                for _ in 0..N {
                    if let Some(top) = i.peek() {
                        i.push(top.dup());
                    }
                    if i.depth() >= 1 {
                        let _ = i.pull();
                    }
                }
                black_box(i.pull())
            },
            BatchSize::SmallInput,
        );
    });
    g.bench_function("dup_drop/promoted", |b| {
        b.iter_batched(
            || {
                let mut i = interp();
                i.push(BundValue::int(1));
                i
            },
            |mut i| {
                // Promoted, the pair is a copy that dies immediately. The
                // black_box keeps the loop from vanishing entirely, which is
                // the honest way to say "this compiles to nothing".
                let v = i.pull().and_then(|v| v.as_int()).unwrap_or(0);
                for _ in 0..N {
                    black_box(v);
                }
                i.push(BundValue::int(v));
                black_box(i.pull())
            },
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

// --- criterion 9: is the sync worth its cost? -----------------------------
//
// §S5's second rule requires promoted values to be written back — each to the
// stack it was taken from — before an opaque call. RFC-0005 criterion 9 says
// plainly that this is **not established**: "For a short straight-line run the
// sync may cost more than the promotion saved", and "If no length passes,
// §S5's second rule is wrong."
//
// The criterion: a body with an opaque site in the middle against the same
// body interpreted, at straight-line lengths of 1, 4, 16 and 64 words before
// the site, with the compiled form no more than **5% slower** at each. The
// shortest passing length is the crossover, and promotion is skipped beneath
// it.
//
// # What the `promoted` column is, and is not
//
// There is no lowering that promotes, so this column is **hand-written Rust
// standing for one** — the same construction `fragment/int_add/promoted` uses,
// plus the sync the rule requires. It is a *ceiling*: real compiled code pays
// an entry, an exit, a guard per site and a helper call per sync, none of
// which is here. So a length that fails this band cannot pass in a real
// lowering, while one that passes here may still fail there. The criterion is
// therefore usable as a **veto** and not as a certificate, and that asymmetry
// is the point of running it before building the residual path.
//
// # The opaque site is synthetic, and deliberately
//
// Every opaque native the stdlib registers is expensive or destructive:
// `for`, `while.`, `times.`, `loop.`, `map.` and `apply` evaluate a lambda,
// `object` runs a class's `.init`, `format` parses a `leon::Template`, and
// `drop_stack` destroys the current stack. Any of them would cost microseconds
// and swallow the sync entirely — the benchmark would be measuring `format`,
// not §S5's rule. So this file registers a no-op native with an `Opaque`
// effect. It is not a word any program calls, and naming it `bench.opaque`
// says so.
//
// # The reporter is the CLI's, as the criterion requires
//
// "It runs under the CLI's default reporter, `TextReporter` with its stack
// dump on, which is the configuration `bund2 script` uses and which permits
// promotion across calls under D45." `Interp::new` installs a `SilentReporter`,
// which is a *different* configuration — under a reporter that wants mid-body
// snapshots, §S5 keeps nothing promoted across a call at all. Nothing here
// raises a diagnostic, so the reporter prints nothing; it is installed because
// the criterion names it.

/// The opaque site. It does nothing: the criterion is about what surrounds it.
fn bench_opaque(_: &mut dyn Vm) -> Result<(), Error> {
    Ok(())
}

/// The interpreter these benchmarks measure: the full stdlib, the synthetic
/// opaque site, and the CLI's reporter.
fn interp_with_site() -> Interp {
    let mut i = Interp::new();
    bund2_stdlib::register_all(&mut i.registry);
    i.registry.register_native(
        "bench.opaque",
        bench_opaque,
        StackEffect::opaque(0),
        WordKind::Sync,
    );
    i.reporter = Box::new(bund2_stdlib::report::TextReporter::new(true));
    i
}

/// How many times each length's body is repeated inside one timed iteration.
///
/// **Not decoration: without it this group cannot decide its own criterion.**
/// Every `iter_batched` here builds an `Interp`, and `startup/registry` puts
/// `register_all` at **5.08 µs**. The existing `fragment` group hides that
/// under `N = 1000` operations, where the signal is 84% of the span. This group
/// measures lengths of 1 to 64, so a single body is at most a few microseconds
/// against a ~9 µs pedestal — and the pedestal was measured wobbling **18.6%**
/// across three runs, against criterion 9's **5%** band. A first version
/// reported a crossover at length 1 that was entirely that artefact.
///
/// At `R = 1000` the signal is 84% of the span at length 1, the worst case, and
/// 96–100% at the rest. The comparison the criterion asks for is unchanged: the
/// same body, the same site, at the same four lengths.
const R: usize = 1000;

fn sync(c: &mut Criterion) {
    let mut g = c.benchmark_group("sync");

    for len in [1usize, 4, 16, 64] {
        // `1 +` repeated, then the opaque call. Both columns re-seed the
        // accumulator per repeat and clear the stack after it, so the two do
        // the same bookkeeping and only the straight-line run and the sync
        // differ.
        let src = format!("{}bench.opaque", "1 + ".repeat(len));
        let stream = bund2_syntax::compile(&src).unwrap_or_default();

        g.bench_with_input(BenchmarkId::new("tier0", len), &stream, |b, stream| {
            b.iter_batched(
                interp_with_site,
                |mut i| {
                    for _ in 0..R {
                        i.push(BundValue::int(0));
                        let ok = black_box(i.eval(black_box(stream))).is_ok();
                        black_box(ok);
                        i.clear();
                    }
                },
                BatchSize::SmallInput,
            );
        });

        g.bench_with_input(BenchmarkId::new("promoted", len), &len, |b, &len| {
            b.iter_batched(
                interp_with_site,
                |mut i| {
                    for _ in 0..R {
                        i.push(BundValue::int(0));

                        // **Promote.** The value leaves the stack and its
                        // origin is remembered — §S5 syncs "to the stack it was
                        // taken from, recorded when the value was promoted, not
                        // the stack current now". D41 put that origin inside
                        // the value.
                        let v = i.pull().unwrap_or_else(|| BundValue::int(0));
                        let origin = v.stack_sym();
                        let mut acc = v.as_int().unwrap_or(0);

                        // The straight-line run, with the literal pushes
                        // elided — which is what promotion buys.
                        for _ in 0..len {
                            acc = acc.wrapping_add(black_box(1));
                        }

                        // **Sync, before the opaque call.** Through
                        // `push`/`push_to` rather than a bare write, so the
                        // value carries its D41 stack symbol: a bare push would
                        // leave `StackSym::NONE` and render `tags: {}` where
                        // the oracle renders `tags: {"stack": "main"}`
                        // (criterion 12).
                        let out = BundValue::int(acc);
                        match bund2_value::stack_name(origin) {
                            Some(name) => i.push_to(&name, out),
                            None => i.push(out),
                        }

                        // The same dispatch both columns make, so the
                        // difference measured is the straight-line run and the
                        // sync alone.
                        let ok = black_box(i.apply(BundValue::call("bench.opaque"))).is_ok();
                        black_box(ok);
                        i.clear();
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    g.finish();
}

criterion_group!(benches, fragment, sync);
criterion_main!(benches);
