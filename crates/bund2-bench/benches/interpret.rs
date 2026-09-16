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

use bund2_api::{Error, StackEffect, Vm, WordKind};
use bund2_interp::Interp;
use bund2_value::BundValue;

/// A registry with the full stdlib, which is what a real run has — **and,
/// under `--features jit`, a tier installed.**
///
/// # Why this is feature-gated rather than one constructor
///
/// RFC-0005 criterion 7 is an A/B across the `jit` feature, and until
/// 2026-09-14 it could not measure anything: every group here built a bare
/// `Interp`, and **only `bund2_runtime::Runtime` installs a tier**. So
/// `--features jit` changed the binary and changed nothing the benchmarks
/// executed, and the comparison reported noise with a straight face — 26
/// "regressions" in groups the tier cannot reach, including `boxing` and
/// `value/clone/scalar`, against `value/push_pull/balanced` reporting no
/// significant difference in the same run.
///
/// That is **F124's shape one layer down**: the CLI had the same defect, where
/// `--features jit` enabled a feature on code that never ran, so criterion 2
/// had been comparing Tier 0 with itself.
///
/// # The two arms register the same vocabulary
///
/// `register_all` *is* `register_all_with(r, &HostOptions::default())`
/// — `register_all` (`crates/bund2-stdlib/src/lib.rs`).
/// And the runtime registers through the same function with the options it was
/// given, which for a default build are those defaults —
/// `Runtime::with_options` (`crates/bund2-runtime/src/lib.rs`).
///
/// So the feature-off and feature-on runs differ in the tier and in nothing
/// else. If they differed in the word table, the A/B would be comparing two
/// languages rather than two tiers.
#[cfg(feature = "jit")]
fn interp() -> Interp {
    bund2_runtime::Runtime::new().interp
}

/// Without the feature there is no tier to install, and this is the
/// construction every group used before the gate existed.
#[cfg(not(feature = "jit"))]
fn interp() -> Interp {
    let mut i = Interp::new();
    bund2_stdlib::register_all(&mut i.registry);
    i
}

/// **Refuse to measure a tier that is not there** — the guard F124, F127 and
/// F128 each cost a session for want of.
///
/// A benchmark cannot assert, so this reports and the reader sees it beside the
/// numbers. An A/B whose feature-on half never reached compiled code is not a
/// passing criterion 7; it is the absence of a measurement, and it looks
/// exactly like a pass.
#[cfg(feature = "jit")]
fn say_whether_the_tier_is_installed() {
    let rt = bund2_runtime::Runtime::new();
    match rt.compiled_bodies() {
        Some(_) => eprintln!(
            "bench: tier installed (threshold {:?}) — criterion 7's A/B measures it",
            rt.jit_threshold()
        ),
        None => eprintln!(
            "bench: WARNING — `--features jit` is on and no tier is installed. \
             Every group below measures Tier 0 against itself, and any difference \
             is noise. This is F124's shape; do not record the result."
        ),
    }
}

#[cfg(not(feature = "jit"))]
fn say_whether_the_tier_is_installed() {
    eprintln!("bench: no tier (built without `jit`) — this is criterion 7's baseline half");
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
    // First group in `criterion_group!`, so this prints above every number in
    // the run. It says whether the half being measured has a tier at all —
    // without which criterion 7's A/B compares Tier 0 with itself and reports
    // noise as a result.
    say_whether_the_tier_is_installed();

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
    // **F135's discriminator lived here and is gone, 2026-09-15.** Three
    // byte-identical copies of the row above — `scalar`, `_adjacent` and
    // `_late` — separated within-process variance from cross-process variance.
    // They did their job: the adjacent pair agreed to 0.6% while the same id
    // moved between processes, which excluded everything fixed at process start
    // and pointed at the host. F135 resolved as a measurement protocol, and the
    // copies were deleted as that entry said they would be.
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

/// **One `Interp` for the whole benchmark, and the program is a registered
/// word** — the warm counterpart to [`timed_eval`].
///
/// `timed_eval` passes [`interp`] as `iter_batched`'s setup, so every iteration
/// starts with an empty cache and a body that crosses §S7's threshold within one
/// eval **recompiles on every iteration**. That is a real cost, but it is a
/// cold-start cost: no session recompiles a body on each call. F130 measured
/// what it costs — one compilation of order 125–141 µs — and criterion 7's
/// `arith` rows were dominated by it.
///
/// Here the program is `register`ed once and entered repeatedly on one warmed
/// interpreter, so the timed region is steady-state execution.
///
/// **The trailing `clear` is load-bearing.** `0 1 + 1 + …` leaves a value per
/// eval and `1000 { 1 + } times drop` leaves 999, so on a long-lived `Interp`
/// the stack would grow without bound and this would become an allocation
/// benchmark. The balance is asserted rather than assumed.
fn warm_eval(
    g: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    body: &str,
) {
    let setup = compiled(&format!(":w {{ {body} clear }} register"));
    let call = compiled("w");
    g.bench_function(name, |b| {
        let mut i = interp();
        if i.eval(&setup).is_err() {
            eprintln!("bench: warm_eval setup failed for {name}; measuring nothing");
        }
        // Past §S7's threshold, outside the timed region: every timed entry
        // finds compiled code and none pays for compilation.
        for _ in 0..(bund2_runtime_threshold() + 8) {
            let _ = i.eval(&call);
        }
        debug_assert_eq!(i.depth(), 0, "the body must be stack-balanced");
        b.iter(|| black_box(i.eval(black_box(&call))).is_ok());
    });
}

/// Arithmetic, the classic JIT target: no allocation, all in the value layer.
///
/// **Restated 2026-09-15 to measure what a session does.** Each program now runs
/// through [`warm_eval`] — registered once, entered repeatedly on one warm
/// interpreter — beside the original cold row it replaces. The `/cold` rows are
/// kept, not deleted: they are the only thing that measures first-entry cost,
/// which a short-lived process really pays, and keeping them is what stops the
/// restatement from being a failure redefined away. See RFC-0005 criterion 7
/// and D69.
///
/// **What each warm row turned out to measure**, by `bund2 --stats`:
///
/// - `int_add` — 1000 sites inlined, 1001 values promoted, so Cranelift folds
///   the literal chain to a constant. It reads −97.9%, and that is **folding,
///   not arithmetic**: the row cannot speak for arithmetic throughput, and a
///   non-foldable shape is owed before it can.
/// - `float_mul` — **0 sites, 0 promoted**. The body compiles, pays the
///   boundary on every value through `jit_apply` and `Vm::apply`, and is repaid
///   nothing: +22.7% to +26.4%, the live half of criterion 7's failure (F133).
/// - `times_body` — straddles zero, which is what F130 predicted once the
///   per-iteration compilation left the timed region.
fn arith(c: &mut Criterion) {
    let mut g = c.benchmark_group("arith");

    let int_add = format!("0 {}", "1 + ".repeat(1000));
    let float_mul = format!("1.0 {}", "1.000001 * ".repeat(1000));
    // `times` runs a body N times through the interpreter's own loop, which is
    // closer to how a real program spends its time than a straight-line stream.
    let times_body = "1000 { 1 + } times drop";

    warm_eval(&mut g, "int_add/1000", &int_add);
    warm_eval(&mut g, "float_mul/1000", &float_mul);
    warm_eval(&mut g, "times_body/1000", times_body);

    timed_eval(&mut g, "int_add/1000/cold", &int_add);
    timed_eval(&mut g, "float_mul/1000/cold", &float_mul);
    timed_eval(&mut g, "times_body/1000/cold", times_body);

    g.finish();
}

/// **Entry into an already-compiled body, with compilation outside the timed
/// region** — F130.
///
/// # Why the `arith/times_body` row could not answer this
///
/// [`timed_eval`] passes [`interp`] as `iter_batched`'s *setup*, so **every
/// iteration builds a fresh `Interp` with an empty cache**. One eval of
/// `1000 { 1 + } times drop` crosses §S7's threshold of 64 within itself, so
/// each iteration pays 64 interpreted entries, **one full Cranelift
/// compilation**, and ~936 compiled entries. Its +136% is the sum of those
/// three, and F130's first diagnosis — three hash probes per entry — was an
/// arithmetic fitted to that sum rather than derived from it. Removing two of
/// those probes measured neutral, which is how the diagnosis was falsified.
///
/// # What this group does instead
///
/// **One `Interp` for the whole benchmark**, as `value/push_pull/balanced`
/// does and for the same reason: `iter_batched`'s teardown falls inside the
/// measured region, and here it would drop a registry of 261 words per
/// iteration.
///
/// **The body is a registered word, not a literal.** Three copies of the same
/// source text are three different bodies — `BundValue::lambda` allocates a
/// fresh `Rc` per occurrence and D35 keys the cache on the payload pointer, so
/// re-evaluating a *stream* recompiles while re-entering a *word* hits. Checked
/// with `--stats`: three top-level copies compile 3 bodies, one registered word
/// entered three times compiles 1.
///
/// **It is warmed past the threshold before timing starts**, so every timed
/// entry finds compiled code and none of them pays for compilation.
///
/// **It is stack-balanced.** `1000 { 1 + } times drop` leaves 999 values per
/// eval — 2997 after three — which on a long-lived `Interp` would grow the
/// stack until this timed allocation rather than entry. The trailing `clear`
/// is what makes a single interpreter safe here, and the balance is asserted
/// below rather than assumed.
fn entry(c: &mut Criterion) {
    let mut g = c.benchmark_group("entry");

    // The word under test, and the call that enters it.
    let setup = compiled(":w { 1000 { 1 + } times drop clear } register");
    let call = compiled("w");

    g.bench_function("compiled_body/e1", |b| {
        let mut i = interp();
        if i.eval(&setup).is_err() {
            eprintln!("bench: entry setup failed; measuring nothing");
        }
        // **Warm past §S7's threshold, outside the timed region.** After this
        // the body is compiled and filed, so every `iter` below enters
        // compiled code and pays no compilation.
        for _ in 0..(bund2_runtime_threshold() + 8) {
            let _ = i.eval(&call);
        }
        // The balance this group rests on: if the body leaked values, a
        // long-lived `Interp` would turn this into an allocation benchmark.
        debug_assert_eq!(i.depth(), 0, "the body must be stack-balanced");
        b.iter(|| black_box(i.eval(black_box(&call))).is_ok());
    });

    g.finish();
}

/// **Compilation, priced on its own** — F130's one remaining suspect.
///
/// The `entry` group shows that entering an already-compiled body costs nothing
/// measurable, while `arith/times_body` regresses ~+121% and recompiles once
/// per Criterion iteration. What is left to price is the compilation, and it is
/// priced here without a second harness: the warming runs in `iter_batched`'s
/// **setup**, which is not timed, so the timed region is one `eval` either side
/// of §S7's threshold.
///
/// - `crossing_entry` — the entry that reaches the threshold. It compiles.
/// - `ordinary_entry` — the entry just before it. Same work, no compilation.
/// - `compiled_entry` — the steady state, well past the threshold.
///
/// `crossing_entry − ordinary_entry` **is** the compilation, measured through
/// the shipped path. The alternative — calling `Compiler::compile_word` from
/// here — would mean a new dependency and rebuilding §S6's fragment table by
/// hand, and F124, F127, F128 and F129 were every one of them a fixture that
/// looked like the real path and was not.
///
/// The body is `1 2 + drop`: one frame per call and no inner loop, so entries
/// equal calls and the threshold arithmetic below is exact.
fn compile(c: &mut Criterion) {
    let t = bund2_runtime_threshold();
    // `t - 2` must exist. A threshold of 1 compiles on the first entry and
    // leaves no "one before it" to compare against.
    if t < 3 {
        eprintln!("bench: threshold {t} is too low to isolate the crossing entry; skipping `compile`");
        return;
    }

    let mut g = c.benchmark_group("compile");
    let setup = compiled(":c { 1 2 + drop } register");
    let call = compiled("c");

    // An interpreter that has entered `c` exactly `entries` times.
    let warmed = |entries: u32| {
        let mut i = interp();
        if i.eval(&setup).is_err() {
            eprintln!("bench: compile setup failed; measuring nothing");
        }
        for _ in 0..entries {
            let _ = i.eval(&call);
        }
        i
    };

    // **Pre-flight, printed rather than assumed.** Two arms that do the same
    // thing report a difference of zero, which looks exactly like a result.
    // This says whether the crossing entry compiles and the ordinary one does
    // not — the whole premise of the subtraction.
    let mut probe = warmed(t - 1);
    let before = compiled_bodies(&probe);
    let _ = probe.eval(&call);
    let crossing = compiled_bodies(&probe);
    let mut earlier = warmed(t - 2);
    let _ = earlier.eval(&call);
    let ordinary = compiled_bodies(&earlier);
    eprintln!(
        "bench: crossing entry {before} -> {crossing} bodies (want 0 -> 1), ordinary entry -> {ordinary} (want 0)"
    );

    g.bench_function("crossing_entry", |b| {
        b.iter_batched(
            || warmed(t - 1),
            |mut i| black_box(i.eval(black_box(&call))).is_ok(),
            BatchSize::SmallInput,
        );
    });
    g.bench_function("ordinary_entry", |b| {
        b.iter_batched(
            || warmed(t - 2),
            |mut i| black_box(i.eval(black_box(&call))).is_ok(),
            BatchSize::SmallInput,
        );
    });
    g.bench_function("compiled_entry", |b| {
        b.iter_batched(
            || warmed(t + 8),
            |mut i| black_box(i.eval(black_box(&call))).is_ok(),
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

/// **Cold entry, and the reason F132 was withdrawn.**
///
/// This group was written to find a crossover: the body size beneath which
/// compiling does not repay. It found none — compiled lost at every size, worse
/// with length, up to +41% at 64 values — and that reading was **an artifact of
/// this group's own shape**, which is why the group is kept rather than deleted.
///
/// `interp` is `iter_batched`'s setup, and it builds a whole `Runtime`:
/// `register_all`, a fresh `Interp`, and with the feature a fresh `JITModule`.
/// Setup is untimed but its effects are not, so every timed entry runs against
/// cold caches and, on the tier side, compiled code emitted microseconds earlier
/// that no instruction cache has seen. One entry of the two-value body reads
/// ~9.7 µs here and ~66.9 ns in `hot_body` — a pedestal **147×** the difference
/// being measured.
///
/// So what this measures is **the first entry into a cold interpreter**, which
/// is a real cost for a short-lived process and is not what F132 claimed. For
/// the steady state — criterion 10's shape, and the tier's actual behaviour —
/// see `hot_body`, where the same bodies read 1.32× to 2.40× *faster* compiled
/// and the slope runs the other way.
///
/// Each size builds a body of `1 drop` pairs, so the value count is exact and
/// the body is stack-balanced. `interpreted/vN` is warmed to one entry below the
/// threshold; `compiled/vN` past it.
fn size_crossover(c: &mut Criterion) {
    let t = bund2_runtime_threshold();
    if t < 3 {
        eprintln!("bench: threshold {t} is too low to warm an interpreted arm; skipping");
        return;
    }

    let mut g = c.benchmark_group("size_crossover");

    for pairs in [1usize, 2, 4, 8, 16, 32] {
        // `1 drop` is two values, so the body holds exactly `pairs * 2`.
        let values = pairs * 2;
        let setup = compiled(&format!(":w {{ {}}} register", "1 drop ".repeat(pairs)));
        let call = compiled("w");

        let warmed = |entries: u32| {
            let mut i = interp();
            if i.eval(&setup).is_err() {
                eprintln!("bench: size_crossover setup failed at v{values}; measuring nothing");
            }
            for _ in 0..entries {
                let _ = i.eval(&call);
            }
            i
        };

        // Say whether the compiled arm actually compiled, per size. A silent
        // failure here would report the two arms as equal and read as a
        // crossover of 1 — F129's shape.
        let hot = warmed(t + 8);
        eprintln!("bench: v{values} compiled bodies {}", compiled_bodies(&hot));

        g.bench_function(format!("interpreted/v{values}"), |b| {
            b.iter_batched(
                || warmed(t - 2),
                |mut i| black_box(i.eval(black_box(&call))).is_ok(),
                BatchSize::SmallInput,
            );
        });
        g.bench_function(format!("compiled/v{values}"), |b| {
            b.iter_batched(
                || warmed(t + 8),
                |mut i| black_box(i.eval(black_box(&call))).is_ok(),
                BatchSize::SmallInput,
            );
        });
    }

    g.finish();
}

/// **The same question with a long-lived `Interp`** — the control that
/// reconciles F132 with criterion 10.
///
/// `compile` and `size_crossover` rebuild the `Interp` in `iter_batched`'s
/// setup, so every timed iteration enters a **freshly built `JITModule`** whose
/// code no instruction cache has seen, while Tier 0's interpreter loop is the
/// same hot code on every iteration. Criterion 10 measures the opposite: one
/// process, one registered word called 10⁶ times in a `for` loop, where
/// compiled code is hot — and it reads a speedup where those two read a loss.
/// They disagree in sign, so at most one of them is a property of the tier.
///
/// Here one `Interp` is warmed past the threshold once and then entered
/// repeatedly, which is criterion 10's shape. With the feature off the same
/// fixture interprets, so criterion 7's A/B — `--save-baseline off` then
/// `--baseline off` — reads compiled against interpreted at each body size.
///
/// # What six lengths bought that three could not
///
/// Timing against body length separates the two costs a compiled entry pays:
/// the **slope** is per value, the **intercept** is per entry. Measured
/// 2026-09-15 across 2, 4, 8, 16, 32 and 64 values:
///
/// | | per value | per entry | R² |
/// |---|---|---|---|
/// | Tier 0 | 18.08 ns | 34.78 ns | 0.99996 |
/// | with the tier | 7.26 ns | 34.21 ns | 0.99997 |
///
/// **The per-entry costs cancel** — a difference of −0.57 ns against a 34 ns
/// intercept — because that intercept is `Interp::eval`'s own overhead, paid
/// with no tier installed. The boundary is *entirely* per value, at 7.26 ns.
/// An earlier three-point fit of this same group reported "~7 ns of fixed entry
/// cost"; that was the residual of too few points, and RFC-0005 criterion 10
/// carries its withdrawal.
fn hot_body(c: &mut Criterion) {
    let t = bund2_runtime_threshold();
    let mut g = c.benchmark_group("hot_body");

    // **Six lengths, to separate the two costs rather than fit one number.**
    // A compiled entry pays a per-*entry* cost — the trampoline, context setup,
    // the cache lookup that found the code — and a per-*value* cost: a slot
    // load, a `call_indirect`, and the request-cell load after it, plus a
    // second indirect call to `jit_admits` at an inlined site. Timing against
    // body length separates them: the **slope** is per value, the **intercept**
    // is per entry. Three points fit a line but cannot show it is one; six can.
    //
    // `hot_body` previously ran 2, 8 and 64, and the ~11 ns per value against
    // ~7 ns fixed quoted in F130 and criterion 10 was fitted from those three.
    // That fit assumed linearity rather than testing it.
    for pairs in [1usize, 2, 4, 8, 16, 32] {
        let values = pairs * 2;
        let setup = compiled(&format!(":w {{ {}}} register", "1 drop ".repeat(pairs)));
        let call = compiled("w");

        // **Said once, here, and not inside the routine.** Criterion invokes the
        // routine closure many times over warm-up and measurement, so an
        // `eprintln!` in there prints thousands of lines and buries the numbers
        // it was added to qualify.
        {
            let mut probe = interp();
            if probe.eval(&setup).is_err() {
                eprintln!("bench: hot_body setup failed at v{values}; measuring nothing");
            }
            for _ in 0..(t + 8) {
                let _ = probe.eval(&call);
            }
            // **Exactly one body, at every length.** The count is the premise of
            // the fit: a program whose driver lambda also compiles would fold a
            // second entry cost into the intercept. `:w { 1 } register` with
            // `100 { w drop } times` reports 2 for that reason, which is why the
            // call here is a bare `w` and why this is printed per size rather
            // than assumed once.
            let bodies = compiled_bodies(&probe);
            // **The warning is for the tier's half only.** With the feature off
            // there is no tier and zero bodies compile *by construction*, so
            // flagging that would cry wolf on every baseline run — which the
            // first version of this line did.
            let want = if cfg!(feature = "jit") { 1 } else { 0 };
            eprintln!(
                "bench: hot_body v{values} compiled bodies {bodies}{}",
                if bodies == want {
                    ""
                } else {
                    "  <-- unexpected, the fit's premise fails here"
                }
            );
        }

        g.bench_function(format!("v{values}"), |b| {
            let mut i = interp();
            if i.eval(&setup).is_err() {
                eprintln!("bench: hot_body setup failed at v{values}; measuring nothing");
            }
            // Warmed once, outside the timed region, so every entry below finds
            // compiled code and none pays for compilation.
            for _ in 0..(t + 8) {
                let _ = i.eval(&call);
            }
            debug_assert_eq!(i.depth(), 0, "the body must be stack-balanced");
            b.iter(|| black_box(i.eval(black_box(&call))).is_ok());
        });
    }

    g.finish();
}

/// **Entry cost, anchored rather than extrapolated** — criterion 10, F130.
///
/// `hot_body`'s six-length sweep gives the per-entry cost as a regression
/// *intercept*, and that intercept is an extrapolation to zero values with
/// nothing near zero to hold it down: its closest measured point is two values
/// at ~59 ns, and the fit pivots on the 64-value point at the far end. Two
/// clean Tier 0 blocks taken six minutes apart, agreeing row-by-row to within
/// 2%, produced intercepts of **35.92 ns and 27.29 ns** — the whole quantity in
/// dispute, from noise at the short end. The pre-restart "the intercepts
/// cancel" and the later "+6.28 ns per entry" were both readings of that.
///
/// This group anchors it. Bodies are **int literals**, one to sixteen, so the
/// shortest is a single value rather than two — and literals are what F133's
/// rule admits: a body with no inlinable site and nothing to promote is refused
/// by the tier, so a body of `clear` calls (the obvious balanced choice) would
/// measure Tier 0 in both arms.
///
/// **`eval_empty` is the anchor proper**: `eval` of an empty stream, which is
/// the harness's own fixed cost with no dispatch and no frame push at all. The
/// entry cost is then `call/v1 − eval_empty − (one value)`, read off measured
/// points instead of a line's y-intercept.
///
/// **The call is `w clear`**, so the stack is balanced at every length. `clear`
/// runs in the stream, at Tier 0, in both arms — a constant that cancels in the
/// difference.
fn entry_anchored(c: &mut Criterion) {
    let t = bund2_runtime_threshold();
    let mut g = c.benchmark_group("entry_anchored");

    let empty: Vec<BundValue> = Vec::new();
    g.bench_function("eval_empty", |b| {
        let mut i = interp();
        b.iter(|| black_box(i.eval(black_box(&empty))).is_ok());
    });

    for n in [1usize, 2, 3, 4, 6, 8, 12, 16] {
        let setup = compiled(&format!(":w {{ {}}} register", "1 ".repeat(n)));
        let call = compiled("w clear");

        {
            let mut probe = interp();
            if probe.eval(&setup).is_err() {
                eprintln!("bench: entry_anchored setup failed at v{n}; measuring nothing");
            }
            for _ in 0..(t + 8) {
                let _ = probe.eval(&call);
            }
            let bodies = compiled_bodies(&probe);
            let want = if cfg!(feature = "jit") { 1 } else { 0 };
            eprintln!(
                "bench: entry_anchored v{n} compiled bodies {bodies}{}",
                if bodies == want {
                    ""
                } else {
                    "  <-- unexpected; F133 may have refused this body"
                }
            );
        }

        g.bench_function(format!("call/v{n}"), |b| {
            let mut i = interp();
            if i.eval(&setup).is_err() {
                eprintln!("bench: entry_anchored setup failed at v{n}; measuring nothing");
            }
            for _ in 0..(t + 8) {
                let _ = i.eval(&call);
            }
            debug_assert_eq!(i.depth(), 0, "the body must be stack-balanced");
            b.iter(|| black_box(i.eval(black_box(&call))).is_ok());
        });
    }

    g.finish();
}

/// **Where the per-value cost goes: the three regimes a value can take.**
///
/// Criterion 10's shortfall is a per-value cost of ~7 ns, and the emitter says
/// what a value pays: a slot load, a `call_indirect`, the request-cell load
/// after it (§S5, *A call may leave a body to run*), and at an inlined site a
/// **second** indirect call to `jit_admits` through its own guard slot.
///
/// **Those four are not separable by a benchmark.** The first three are emitted
/// together for every value that is not a site, so no program exercises one
/// without the others; splitting them needs emitter variants with pieces
/// disabled, or instruction-level profiling. What a fixture *can* separate is
/// the three regimes a value falls into, which is where the 7 ns actually goes:
///
/// - `literal/vN` — N int literals. **0 sites, N promoted.** Each becomes an
///   `iconst` in a `Variable` (D66) and is synced before return, so this is
///   promotion's cost with no call at all.
/// - `generic/vN` — one literal then N `clear` calls. **0 sites, 1 promoted.**
///   `clear` publishes no fragment, so every call takes the generic path: slot,
///   `call_indirect`, request-cell load.
/// - `inlined/vN` — one literal then N `dup_one` calls. **N sites, 1 promoted.**
///
/// **`dup_one`, not `dup`.** §S6's table publishes `("dup_one", dup)`, and
/// `dup` is an alias holding no `Native` and therefore no registration id — so a
/// body of `dup` calls inlines **nothing**, checked with `--stats` before this
/// was written. That distinction would have measured the generic regime twice
/// while looking correct.
///
/// The leading literal exists because **F133 refuses a body with no site and
/// nothing to promote**: a body of bare `clear` calls compiles 0 bodies. It is
/// one value at every length, so it lands in the intercept and leaves the slope
/// clean.
///
/// The call is `w clear`, balanced at every length in every family, with
/// `clear` running at Tier 0 in both arms as a constant that cancels.
fn regimes(c: &mut Criterion) {
    let t = bund2_runtime_threshold();
    let mut g = c.benchmark_group("regimes");
    let call = compiled("w clear");

    let families: [(&str, &dyn Fn(usize) -> String); 3] = [
        ("literal", &|n: usize| "1 ".repeat(n)),
        ("generic", &|n: usize| format!("1 {}", "clear ".repeat(n))),
        ("inlined", &|n: usize| format!("1 {}", "dup_one ".repeat(n))),
    ];

    for (name, body_of) in families {
        for n in [1usize, 2, 4, 8, 12, 16] {
            let setup = compiled(&format!(":w {{ {}}} register", body_of(n)));

            {
                let mut probe = interp();
                if probe.eval(&setup).is_err() {
                    eprintln!("bench: regimes {name}/v{n} setup failed; measuring nothing");
                }
                for _ in 0..(t + 8) {
                    let _ = probe.eval(&call);
                }
                eprintln!(
                    "bench: regimes {name}/v{n} bodies {}",
                    compiled_bodies(&probe)
                );
            }

            g.bench_function(format!("{name}/v{n}"), |b| {
                let mut i = interp();
                if i.eval(&setup).is_err() {
                    eprintln!("bench: regimes {name}/v{n} setup failed; measuring nothing");
                }
                for _ in 0..(t + 8) {
                    let _ = i.eval(&call);
                }
                debug_assert_eq!(i.depth(), 0, "the body must be stack-balanced");
                b.iter(|| black_box(i.eval(black_box(&call))).is_ok());
            });
        }
    }

    g.finish();
}

/// **F130's compile time, re-taken warm.**
///
/// The `compile` group measures a compilation through the tier, but it rebuilds
/// the `Interp` per iteration, so its ~128 µs carries the cold-start pedestal
/// F132 was withdrawn over. The difference of two identically-cold arms should
/// largely cancel — and the no-tier control says the gap appears only when a
/// compilation happens — but the *magnitude* was never trustworthy, and F130
/// says so.
///
/// This times `Compiler::compile_word` directly, with the compiler **reused
/// across iterations**, so the module, its tables and the allocator are warm and
/// nothing per-iteration is built outside the timed region. The fragment table
/// comes from `bund2_stdlib::fragments::published`, which is the call
/// `bund2_runtime`'s tier makes — not a table assembled here, which is how
/// F124, F127, F128 and F129 each went wrong.
///
/// `iter_custom` bounds a batch to 256 compilations: one `JITModule` accumulates
/// a function per compile, and code memory is never reclaimed (§S4), so an
/// unbounded run would measure an ever-growing module. Batch setup and eight
/// warm-up compilations sit outside the timed region — the first compilation
/// into a fresh module pays for pages and tables the rest reuse, which is not
/// what F130 asks about.
///
/// The body is `1 2 + drop`, the same four values the `compile` group compiles,
/// so the two figures answer the same question and can be compared.
#[cfg(feature = "jit")]
fn compile_warm(c: &mut Criterion) {
    use bund2_jit::lower::{Compiler, LastCall};
    use std::time::{Duration, Instant};

    let mut g = c.benchmark_group("compile_warm");
    let body = compiled("1 2 + drop");

    g.bench_function("1_2_add_drop", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            // **The batch depth is a knob because it is a suspect.** Every
            // compilation in a batch adds a function to the *same* `JITModule`,
            // so if finalisation cost grows with what the module already holds,
            // a deep batch reports more than one compilation costs. Sweeping
            // this says whether the number is the compiler's or the harness's.
            let depth: u64 = std::env::var("BUND2_BENCH_COMPILE_BATCH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(256);
            let mut done = 0u64;
            while done < iters {
                let batch = (iters - done).min(depth.max(1));

                // Untimed: the interpreter, the published table and the
                // compiler the batch will reuse.
                let mut vm = bund2_runtime::Runtime::new().interp;
                let table = bund2_stdlib::fragments::published(&vm.registry).unwrap_or_default();
                let Ok(mut comp) = Compiler::new(table) else {
                    eprintln!("bench: no compiler could be built; measuring nothing");
                    return total;
                };
                let cells = vm.cells().base();

                for _ in 0..8 {
                    let _ = comp.compile_word(&body, LastCall::Ordinary, cells, &mut vm);
                }

                let t0 = Instant::now();
                for _ in 0..batch {
                    let _ = black_box(comp.compile_word(
                        black_box(&body),
                        LastCall::Ordinary,
                        cells,
                        &mut vm,
                    ));
                }
                total += t0.elapsed();
                done += batch;
            }
            total
        });
    });

    g.finish();
}

/// Without the feature there is no compiler to time.
#[cfg(not(feature = "jit"))]
fn compile_warm(_: &mut Criterion) {}

/// How many bodies this interpreter's tier has compiled; 0 where there is no
/// tier, which is what the feature-off half of the A/B sees.
#[cfg(feature = "jit")]
fn compiled_bodies(i: &Interp) -> usize {
    i.tier
        .as_ref()
        .and_then(|t| bund2_api::Tier::compiled_bodies(t.as_ref()))
        .unwrap_or(0)
}

#[cfg(not(feature = "jit"))]
fn compiled_bodies(_: &Interp) -> usize {
    0
}

/// §S7's default threshold, so the warm-up above outlasts it without this file
/// hard-coding a number that lives in `bund2-jit`.
///
/// Without the feature there is no tier and no threshold; the warm-up is then
/// merely a few extra interpreted entries, which is harmless and keeps the two
/// halves of the A/B doing the same work.
#[cfg(feature = "jit")]
fn bund2_runtime_threshold() -> u32 {
    bund2_runtime::Runtime::new().jit_threshold().unwrap_or(64)
}

#[cfg(not(feature = "jit"))]
fn bund2_runtime_threshold() -> u32 {
    64
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

/// An empty native body: a word's own work, set to exactly zero.
///
/// Not a language word and not registered by `bund2-stdlib`. It exists so the
/// two arms below can differ in their dispatch and in nothing else.
fn nop(_: &mut dyn Vm) -> Result<(), Error> {
    Ok(())
}

/// **Dispatch, isolated — §S1's gate, constructed rather than subtracted.**
///
/// The `dispatch` group above can only *bound* dispatch, because it differences
/// two programs that differ in their work as well as in their dispatch: `1
/// drop` minus a bare literal leaves a dispatch and a `VecDeque::pop_back`
/// together, which is why §S1 has to say "at most". These two arms run the
/// **same empty native body** 1000 times and differ in nothing else:
///
/// - `resolved/w1000` — a stream of 1000 calls to it. The eval loop walks the
///   stream, resolves each name through the registry, and dispatches.
/// - `direct/w1000` — the same `NativeFn`, called 1000 times through its
///   pointer. The body, and nothing around it.
///
/// So `resolved − direct` is the eval loop step plus name resolution plus
/// dispatch, with the word's work held at zero — the quantity §S1 could
/// previously only bound from above. `direct` is the floor no lowering can pass
/// while still calling the native at all.
///
/// Registration happens in the setup closure, outside the timed region, as this
/// file's one rule requires.
fn dispatch_isolated(c: &mut Criterion) {
    let mut g = c.benchmark_group("dispatch_isolated");
    // 1000 words, every one of them the nop.
    let stream = compiled(&"benchnop ".repeat(1000));
    let with_nop = || {
        let mut i = interp();
        i.registry
            .register_native("benchnop", nop, StackEffect::fixed(0, 0), WordKind::Sync);
        i
    };
    g.bench_function("resolved/w1000", |b| {
        b.iter_batched(
            with_nop,
            |mut i| black_box(i.eval(black_box(&stream))).is_ok(),
            BatchSize::SmallInput,
        );
    });
    g.bench_function("direct/w1000", |b| {
        b.iter_batched(
            with_nop,
            |mut i| {
                for _ in 0..1000 {
                    let _ = black_box(nop(&mut i));
                }
                black_box(i.depth())
            },
            BatchSize::SmallInput,
        );
    });
    g.finish();
}

criterion_group!(
    benches,
    startup,
    value,
    dispatch,
    dispatch_isolated,
    arith,
    entry,
    compile,
    compile_warm,
    size_crossover,
    hot_body,
    entry_anchored,
    regimes,
    lambda,
    corpus,
    rendering
);
criterion_main!(benches);
