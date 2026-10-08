//! `cargo xtask bench` — Phase 0 wall-clock baseline over the corpus.
//!
//! **Not Criterion, deliberately.** The roadmap says "Criterion baseline over
//! the corpus", but Criterion measures a function in-process, with warmup and
//! statistical resampling. What Phase 0 needs is how long the *oracle* takes
//! to run a program end to end — a subprocess, dominated by process start and
//! stdlib registration, and run a handful of times rather than thousands.
//! Criterion would measure the wrong thing precisely. When Bund2 has an
//! in-process interpreter to microbenchmark, Criterion becomes the right tool
//! and belongs in `benches/`, not here.
//!
//! So this times subprocesses and reports the distribution honestly: min,
//! median, and max across N runs, per program and in total. Min is the
//! headline, because it is the least contaminated by scheduling noise.
//!
//! The baseline it writes is what Bund2 is measured against later. Both
//! targets go through the same harness — `--target bund2` times Bund2 instead
//! of the oracle — so the comparison is like for like.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::golden;

/// Runs per program. Small on purpose: each run is a process spawn of a large
/// binary, and the figure that matters is a floor, not a mean.
const DEFAULT_RUNS: usize = 5;

struct Timing {
    program: String,
    runs: Vec<Duration>,
    failed: bool,
}

impl Timing {
    fn min(&self) -> Duration {
        self.runs.iter().copied().min().unwrap_or_default()
    }
    fn median(&self) -> Duration {
        if self.runs.is_empty() {
            return Duration::ZERO;
        }
        let mut v = self.runs.clone();
        v.sort();
        v[v.len() / 2]
    }
    fn max(&self) -> Duration {
        self.runs.iter().copied().max().unwrap_or_default()
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Time one run, discarding output. Failure is recorded, not fatal: a program
/// that errors still has a meaningful cost, and excluding it would flatter the
/// total.
fn time_once(exe: &Path, program: &Path, cwd: &Path) -> Option<Duration> {
    let start = Instant::now();
    let out = std::process::Command::new(exe)
        .arg("script")
        .arg("--file")
        .arg(program)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match out {
        Ok(_) => Some(start.elapsed()),
        Err(_) => None,
    }
}

/// Class-chain depths sampled by `--oop`. Only the endpoints matter: the cost
/// per hierarchy level is read as a *slope* between two depths, which is what
/// cancels the fixed cost the module header warns about. 128 is the top
/// because `make_object`'s budget is 512 (`crates/bund2-stdlib/src/oop.rs`)
/// and a chain at the budget reports rather than constructs — which is §S3
/// and D37 working, but measures nothing.
const OOP_DEPTHS: [usize; 3] = [1, 32, 128];

/// Dispatches per dispatch-mode program, and constructions per construct-mode
/// program. They differ because a construction at depth 128 costs roughly
/// twenty times a dispatch, and equal counts would make the construction
/// program dominate the wall clock without buying any precision.
const OOP_DISPATCHES: usize = 20_000;
const OOP_CONSTRUCTIONS: usize = 2_000;

/// One side of the comparison: a display label, a program generator taking
/// `(depth, iterations)`, and how many iterations that side runs.
type OopMode = (&'static str, fn(usize, usize) -> String, usize);

/// A class chain `C0 <- C1 <- … <- C(depth-1)`, rooted at `Object` so `.id`
/// resolves — it lives on the base class, `depth` levels up.
fn oop_chain(depth: usize) -> String {
    let mut s = String::from(":C0 class \".super\" [ :Object ] set register\n");
    for i in 1..depth {
        s.push_str(&format!(
            ":C{i} class \".super\" [ :C{} ] set register\n",
            i - 1
        ));
    }
    s
}

/// Construct once, then dispatch `n` times against that one object.
///
/// `:.id` pushes the method *name* — a STRING, not a PTR — and `!` on an
/// OBJECT takes the name from below the object (§S6), so the body is
/// `:.id swap ! drop`: push name, put it under the receiver, dispatch, discard
/// the answer. The receiver survives because `.id` peeks (§S4).
///
/// Written out rather than looped with `times`, because a `times` body runs
/// against a scoped stack — which is why the corpus's own `times` test carries
/// its accumulator on the workbench
/// (`reference/Bund/tests/test_times_loop.bund:1-4`) — and the receiver would
/// not be visible inside it.
fn oop_dispatch_program(depth: usize, n: usize) -> String {
    let mut s = oop_chain(depth);
    s.push_str(&format!(":C{} object\n", depth - 1));
    for _ in 0..n {
        s.push_str(":.id swap ! drop\n");
    }
    s.push_str("drop\n");
    s
}

/// Construct `n` objects from the deepest class, discarding each.
fn oop_construct_program(depth: usize, n: usize) -> String {
    let mut s = oop_chain(depth);
    let line = format!(":C{} object drop\n", depth - 1);
    for _ in 0..n {
        s.push_str(&line);
    }
    s
}

/// `cargo xtask bench --oop` — what does a hierarchy level cost, on each side?
///
/// This exists to decide RFC-0009 criterion 2. §S1 adds a flattened method
/// table so that dispatch stops walking `.super`; §S1a then has to rebuild or
/// invalidate that table per object, which lands on construction. Whether that
/// is a win is a measurement, and this is it.
///
/// **Read the slope, not the level.** Every figure here includes process
/// start, stdlib registration, and parsing tens of thousands of lines — the
/// fixed cost this module's header warns about. Subtracting the depth-1
/// program from the depth-128 one cancels all of it, because the two differ
/// only in how deep the chain is. What survives is the per-level cost, which
/// is the only quantity the criterion turns on.
fn run_oop(exe: &Path, cwd: &Path, runs: usize, target: &str) -> Result<(), String> {
    let dir = std::env::temp_dir().join("bund2-xtask-bench-oop");
    std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;

    bund2_api::sayln!("# cargo xtask bench --oop\n");
    bund2_api::sayln!("What does one class-hierarchy level cost — on dispatch, and on");
    bund2_api::sayln!("construction? Target `{target}`, {runs} runs each, min reported.\n");
    bund2_api::sayln!("RFC-0009 §S1 flattens the method table so dispatch stops walking");
    bund2_api::sayln!("`.super`; §S1a then has to guard or rebuild that table per object,");
    bund2_api::sayln!("which lands on construction. Criterion 2 turns on which side is");
    bund2_api::sayln!("actually paying, so this measures both.\n");
    bund2_api::sayln!("Read the *slope*. Each figure below includes process start, stdlib");
    bund2_api::sayln!("registration and parsing tens of thousands of lines. The depth-1");
    bund2_api::sayln!("and depth-128 programs differ only in chain depth, so subtracting");
    bund2_api::sayln!("them cancels every one of those, and the per-level cost is what is");
    bund2_api::sayln!("left.\n");

    let modes: [OopMode; 2] = [
        ("dispatch", oop_dispatch_program, OOP_DISPATCHES),
        ("construct", oop_construct_program, OOP_CONSTRUCTIONS),
    ];

    bund2_api::sayln!("  {:<12}{:>8}{:>12}{:>12}", "mode", "depth", "iters", "min ms");
    let mut floor: [Option<f64>; 2] = [None, None];
    let mut top: [Option<f64>; 2] = [None, None];
    for (mi, (label, make, iters)) in modes.iter().enumerate() {
        for &depth in &OOP_DEPTHS {
            let path = dir.join(format!("{label}-{depth}.bund"));
            std::fs::write(&path, make(depth, *iters))
                .map_err(|e| format!("writing {}: {e}", path.display()))?;
            let mut samples = Vec::with_capacity(runs);
            for _ in 0..runs {
                match time_once(exe, &path, cwd) {
                    Some(d) => samples.push(d),
                    None => return Err(format!("could not spawn {}", exe.display())),
                }
            }
            let Some(min) = samples.iter().copied().min() else {
                return Err("no samples".into());
            };
            bund2_api::sayln!("  {label:<12}{depth:>8}{:>12}{:>12.1}", iters, ms(min));
            if depth == OOP_DEPTHS[0] {
                floor[mi] = Some(ms(min));
            }
            if depth == OOP_DEPTHS[OOP_DEPTHS.len() - 1] {
                top[mi] = Some(ms(min));
            }
        }
    }
    bund2_api::sayln!();

    let levels = (OOP_DEPTHS[OOP_DEPTHS.len() - 1] - OOP_DEPTHS[0]) as f64;
    bund2_api::sayln!("## per level\n");
    let mut per_level = [0.0f64; 2];
    for (mi, (label, _, iters)) in modes.iter().enumerate() {
        let (Some(lo), Some(hi)) = (floor[mi], top[mi]) else {
            return Err("missing endpoint measurement".into());
        };
        // ms over the whole program → ns per iteration per level.
        let ns = (hi - lo) * 1.0e6 / (*iters as f64) / levels;
        per_level[mi] = ns;
        bund2_api::sayln!("  {label:<12}{ns:>9.0} ns  per hierarchy level, per operation");
    }
    bund2_api::sayln!();

    bund2_api::sayln!("## reading\n");
    if per_level[0] <= 0.0 || per_level[1] <= 0.0 {
        bund2_api::sayln!("  A slope came out at or below zero, which means the fixed cost");
        bund2_api::sayln!("  swamped the signal on this machine. Raise --runs and re-read");
        bund2_api::sayln!("  before drawing any conclusion; do not report this as flat.\n");
        return Ok(());
    }
    let ratio = per_level[1] / per_level[0];
    bund2_api::sayln!("  Construction pays {ratio:.0}x what dispatch pays for the same");
    bund2_api::sayln!("  hierarchy level. §S1's flattened table removes the cheaper of");
    bund2_api::sayln!("  the two, and §S1a's per-object guard adds to the dearer one.");
    bund2_api::sayln!("  RFC-0009 criterion 2 records this and what follows from it.\n");
    Ok(())
}

pub fn run(args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let (features, args) = crate::buildcli::take_features(args);
    let args = args.as_slice();

    let mut target = "oracle".to_string();
    let mut runs = DEFAULT_RUNS;
    let mut write = false;
    let mut oop = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--target" => {
                target = it
                    .next()
                    .cloned()
                    .ok_or("--target needs a value: oracle or bund2")?;
            }
            "--runs" => {
                runs = it
                    .next()
                    .and_then(|n| n.parse().ok())
                    .ok_or("--runs needs a number")?;
            }
            "--write" => write = true,
            "--oop" => oop = true,
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    if runs == 0 {
        return Err("--runs must be at least 1".into());
    }

    let exe: PathBuf = match target.as_str() {
        "oracle" => {
            let p = repo.join("target/oracle/release/bund");
            if !p.is_file() {
                return Err(format!(
                    "no oracle at {}.\n  Build it out-of-tree so the submodule stays clean:\n    \
                     cargo build --release --manifest-path reference/Bund/Cargo.toml \\\n                \
                     --target-dir target/oracle",
                    p.strip_prefix(&repo).unwrap_or(&p).display()
                ));
            }
            p
        }
        "bund2" => {
            // **Build it, do not merely find it.** A stale `target/release/bund2`
            // times a Bund2 that no longer exists and says nothing about it:
            // this harness once reported numbers for a binary five days and
            // sixty-four words out of date, and the output was indistinguishable
            // from a current run. `xtask effects` already builds `bund2-cli`
            // before asking it anything, for the same reason — a measurement
            // whose subject is unidentified is worse than no measurement,
            // because it will be quoted.
            crate::buildcli::bund2(&repo, true, &features)?
        }
        other => return Err(format!("unknown --target `{other}`; use oracle or bund2")),
    };

    let cwd = repo.join("reference/Bund");

    if oop {
        return run_oop(&exe, &cwd, runs, &target);
    }

    let suite = golden::read_suite(&repo)?;

    bund2_api::sayln!("# cargo xtask bench\n");
    bund2_api::sayln!(
        "Wall-clock baseline over the {} suite programs, {runs} runs each,",
        suite.len()
    );
    bund2_api::sayln!("target `{target}`.\n");
    bund2_api::sayln!("Not Criterion: this times a subprocess end to end, where cost is");
    bund2_api::sayln!("dominated by process start and stdlib registration. Criterion");
    bund2_api::sayln!("measures a function in-process and would measure the wrong thing");
    bund2_api::sayln!("precisely. It becomes the right tool when Bund2 has an in-process");
    bund2_api::sayln!("interpreter to microbenchmark.\n");
    bund2_api::sayln!("Min is the headline — it is the least contaminated by scheduling");
    bund2_api::sayln!("noise. Median and max are shown so the spread is visible.\n");

    let mut timings: Vec<Timing> = Vec::new();
    for program in &suite {
        let path = repo.join(program);
        let mut samples = Vec::with_capacity(runs);
        let mut failed = false;
        for _ in 0..runs {
            match time_once(&exe, &path, &cwd) {
                Some(d) => samples.push(d),
                None => {
                    failed = true;
                    break;
                }
            }
        }
        timings.push(Timing {
            program: program.clone(),
            runs: samples,
            failed,
        });
    }

    let ok: Vec<&Timing> = timings.iter().filter(|t| !t.failed).collect();
    let total_min: Duration = ok.iter().map(|t| t.min()).sum();
    let total_median: Duration = ok.iter().map(|t| t.median()).sum();

    bund2_api::sayln!("## totals\n");
    bund2_api::sayln!("  programs timed          {:>9}", ok.len());
    bund2_api::sayln!("  failed to spawn         {:>9}", timings.len() - ok.len());
    bund2_api::sayln!("  sum of per-program min  {:>9.1} ms", ms(total_min));
    bund2_api::sayln!("  sum of per-program med  {:>9.1} ms", ms(total_median));
    if !ok.is_empty() {
        bund2_api::sayln!(
            "  mean per program (min)  {:>9.1} ms",
            ms(total_min) / ok.len() as f64
        );
    }
    bund2_api::sayln!();

    let mut slowest: Vec<&Timing> = ok.clone();
    slowest.sort_by_key(|r| std::cmp::Reverse(r.min()));
    bund2_api::sayln!("## slowest 15 by min\n");
    bund2_api::sayln!(
        "  {:<58}{:>9}{:>9}{:>9}",
        "program", "min ms", "med ms", "max ms"
    );
    for t in slowest.iter().take(15) {
        bund2_api::sayln!(
            "  {:<58}{:>9.1}{:>9.1}{:>9.1}",
            t.program.trim_start_matches("reference/Bund/"),
            ms(t.min()),
            ms(t.median()),
            ms(t.max())
        );
    }
    bund2_api::sayln!();

    // A floor worth knowing: how much of each run is just starting the binary.
    bund2_api::sayln!("## interpretation\n");
    if let Some(fastest) = ok.iter().map(|t| t.min()).min() {
        bund2_api::sayln!(
            "  The fastest program takes {:.1} ms. That is close to the floor for",
            ms(fastest)
        );
        bund2_api::sayln!("  spawning this binary and registering its stdlib, so most of the");
        bund2_api::sayln!("  per-program figure is fixed cost, not interpretation. Comparing");
        bund2_api::sayln!("  Bund2 against this baseline measures both together — worth");
        bund2_api::sayln!("  separating before drawing conclusions about the interpreter.\n");
    }

    if write {
        let path = repo.join("docs/bench-baseline.md");
        let mut s = String::new();
        s.push_str("# Wall-clock baseline (generated)\n\n");
        s.push_str("Generated by `cargo xtask bench --write`. Do not edit.\n\n");
        s.push_str(&format!(
            "Target `{target}`, {runs} runs per program, {} programs.\n\n",
            ok.len()
        ));
        s.push_str("Figures are wall-clock for a whole subprocess: process start,\n");
        s.push_str("stdlib registration, parse and run. They are not interpreter\n");
        s.push_str("microbenchmarks and should not be read as such.\n\n");
        s.push_str("| program | min ms | median ms | max ms |\n|---|---|---|---|\n");
        let mut sorted: Vec<&Timing> = ok.clone();
        sorted.sort_by(|a, b| a.program.cmp(&b.program));
        for t in sorted {
            s.push_str(&format!(
                "| `{}` | {:.1} | {:.1} | {:.1} |\n",
                t.program.trim_start_matches("reference/Bund/"),
                ms(t.min()),
                ms(t.median()),
                ms(t.max())
            ));
        }
        s.push_str(&format!(
            "\nSum of per-program minima: {:.1} ms.\n",
            ms(total_min)
        ));
        std::fs::write(&path, s).map_err(|e| format!("writing {}: {e}", path.display()))?;
        bund2_api::sayln!("wrote docs/bench-baseline.md");
    } else {
        bund2_api::sayln!("  Nothing written. Pass --write to record docs/bench-baseline.md.\n");
    }

    Ok(())
}
