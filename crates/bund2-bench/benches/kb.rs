//! **RFC-0013 criterion 16: measured, not gated.** The figures are recorded in
//! `docs/measurements/kb-2026-10-30.md`; nothing here asserts a bound.
//!
//! - `kb/nrev30x10000` — naive reverse of 30 elements, 10,000 times, as one
//!   query: `[ bench L ]` over a rule that runs `nrev` once per item of a
//!   10,000-item list and then fails. The inferences it counts are printed
//!   first, so the time divides into inferences a second.
//! - `kb/join10000` — every answer of `[ j X Z ]` over `[ p X Y ] [ q Y Z ]`,
//!   10,000 facts each, one MAP per answer.
//! - `kb/assert10000` — 10,000 `kb.assert`s, one by one, against the KB
//!   `[ [ f -1 ] ] kb!` builds (a LIST literal cannot be empty).
//! - `kb/nat1000000` — `[ nat _ ]` to 1,000,000 answers through `query`, as
//!   criterion 4 runs it.
//! - `kb/loop20000000` — criterion 5's query, to its cap of 20,000,000. The
//!   arena's two stores are printed first.
//! - **The occurs check's share and the guard poll's** are `kb/nrev30x10000`
//!   A/B across the `kb-unchecked` and `kb-unpolled` features, against a
//!   baseline saved without either (`Cargo.toml` gives the commands). Each
//!   compiles one of them out, so the difference is its whole cost.
//!
//! **Parse and register outside the timed region**, as `interpret.rs` says:
//! the engine groups build their KB once and time only the solving, and the
//! two word groups build their `Interp` in the setup closure.

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use bund2_api::{Error, Vm as _};
use bund2_interp::Interp;
use bund2_stdlib::kb::solve::{Bridge, POLLED, Solver};
use bund2_stdlib::kb::store::Kb;
use bund2_stdlib::kb::unify::CHECKED;
use bund2_value::BundValue;

/// No λ runs in these queries, and W1 is expected from `kb/loop20000000`.
struct Quiet;

impl Bridge for Quiet {
    fn warn(&mut self, _: String) {}
    fn run(&mut self, _: Vec<BundValue>) -> Result<Option<BundValue>, Error> {
        Ok(None)
    }
}

fn interp() -> Interp {
    let mut i = Interp::new();
    bund2_stdlib::register_all(&mut i.registry);
    i
}

/// A bench may not panic under this workspace's lints; a source that does not
/// compile is reported and measured as nothing, as `interpret.rs` does.
fn compiled(src: &str) -> Vec<BundValue> {
    match bund2_syntax::compile(src) {
        Ok(s) => s,
        Err(e) => {
            bund2_api::errln!("bench: source failed to compile, measuring nothing: {e:?}");
            Vec::new()
        }
    }
}

/// What `src` leaves on top, or a report and `None`.
fn top(src: &str) -> Option<BundValue> {
    let mut i = interp();
    if let Err(e) = i.eval(&compiled(src)) {
        bund2_api::errln!("bench: setup failed: {e:?}");
        return None;
    }
    i.pull()
}

/// The KB `clauses kb!` builds, capped at `cap` inferences.
fn kb(clauses: &str, cap: i64) -> Option<Kb> {
    let v = top(&format!("{clauses} kb!"))?;
    Kb::read(&v, 1).ok().map(|k| k.with_inferences(cap))
}

/// Every answer of `goal`, and the inferences they took.
fn solve_all(kb: &Kb, goal: &BundValue) -> (usize, u64) {
    let mut s = match Solver::new(kb, goal) {
        Ok(s) => s,
        Err(e) => {
            bund2_api::errln!("bench: the goal was refused: {e:?}");
            return (0, 0);
        }
    };
    let mut n = 0;
    loop {
        match s.next(&mut Quiet) {
            Ok(Some(a)) => {
                black_box(a);
                n += 1;
            }
            Ok(None) => return (n, s.inferences()),
            Err(e) => {
                bund2_api::errln!("bench: the query stopped: {e:?}");
                return (n, s.inferences());
            }
        }
    }
}

fn ints(n: usize, item: impl Fn(usize) -> String) -> String {
    (0..n).map(item).collect::<Vec<_>>().join(" ")
}

const NREV: &str = "[ app nil L L ] \
     [ [ app [ H | T ] L [ H | R ] ] ← [ app T L R ] ] \
     [ nrev nil nil ] \
     [ [ nrev [ H | T ] R ] ← [ nrev T RT ] [ app RT [ H ] R ] ]";

fn engine(c: &mut Criterion) {
    bund2_api::errln!(
        "kb bench: occurs check {}, guard poll {}",
        if CHECKED { "on" } else { "COMPILED OUT" },
        if POLLED { "on" } else { "COMPILED OUT" }
    );
    let mut g = c.benchmark_group("kb");
    g.sample_size(10);

    let driver = format!(
        "[ {NREV} [ [ bench L ] ← [ in _ [ {} ] ] [ nrev L _ ] fail ] ]",
        ints(10_000, |_| "0".into())
    );
    let goal = top(&format!(
        "[ bench [ {} ] ]",
        ints(30, |i| (i + 1).to_string())
    ));
    if let (Some(kb), Some(goal)) = (kb(&driver, 0), goal) {
        let (_, inferences) = solve_all(&kb, &goal);
        bund2_api::errln!("kb/nrev30x10000: {inferences} inferences a run");
        g.bench_function("nrev30x10000", |b| {
            b.iter(|| solve_all(&kb, black_box(&goal)))
        });
    }

    let facts = format!(
        "[ {} {} [ [ j X Z ] ← [ p X Y ] [ q Y Z ] ] ]",
        ints(10_000, |i| format!("[ p {i} {i} ]")),
        ints(10_000, |i| format!("[ q {i} {} ]", i + 1))
    );
    if let (Some(kb), Some(goal)) = (kb(&facts, 0), top("[ j X Z ]")) {
        let (answers, inferences) = solve_all(&kb, &goal);
        bund2_api::errln!("kb/join10000: {answers} answers, {inferences} inferences a run");
        g.bench_function("join10000", |b| b.iter(|| solve_all(&kb, black_box(&goal))));
    }

    if let (Some(kb), Some(goal)) = (
        kb("[ [ [ loop ] ← [ loop ] ] ]", 20_000_000),
        top("[ loop ]"),
    ) {
        if let Ok(mut s) = Solver::new(&kb, &goal) {
            let stopped = s.next(&mut Quiet).is_err();
            let (main, tpl) = s.arena_cells();
            bund2_api::errln!(
                "kb/loop20000000: stopped by the cap {stopped}, {} inferences; \
                 arena: {main} main cells, {tpl} template cells",
                s.inferences()
            );
        }
        // E30 is the expected end here, so it is not reported each run.
        g.bench_function("loop20000000", |b| {
            b.iter(|| {
                Solver::new(&kb, black_box(&goal))
                    .map(|mut s| s.next(&mut Quiet).is_err())
                    .is_ok()
            })
        });
    }
    g.finish();
}

/// Run `stream` once before it is timed, so a refusal is reported rather than
/// measured as a fast failure.
fn checked(name: &str, stream: &[BundValue]) {
    let mut i = interp();
    match i.eval(stream) {
        Ok(()) => bund2_api::errln!("kb/{name}: ran, leaving {} value(s)", i.depth()),
        Err(e) => bund2_api::errln!("kb/{name}: FAILED, and the time below is a failure's: {e:?}"),
    }
}

fn words(c: &mut Criterion) {
    let mut g = c.benchmark_group("kb");
    g.sample_size(10);

    let asserts = compiled(&format!(
        "[ [ f -1 ] ] kb! {}",
        ints(10_000, |i| format!("[ f {i} ] kb.assert"))
    ));
    checked("assert10000", &asserts);
    g.bench_function("assert10000", |b| {
        b.iter_batched(
            interp,
            |mut i| black_box(i.eval(black_box(&asserts))).is_ok(),
            BatchSize::SmallInput,
        );
    });

    let nat = compiled(
        ":nats [ [ nat z ] [ [ nat [ s N ] ] ← [ nat N ] ] ] kb! var \
         query :kb :nats var? set :goal [ nat _ ] set \
         :lambda { drop } set :limit 1000000 set !",
    );
    checked("nat1000000", &nat);
    g.bench_function("nat1000000", |b| {
        b.iter_batched(
            interp,
            |mut i| black_box(i.eval(black_box(&nat))).is_ok(),
            BatchSize::SmallInput,
        );
    });
    g.finish();
}

criterion_group!(benches, engine, words);
criterion_main!(benches);
