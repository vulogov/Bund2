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

pub fn run(args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let mut target = "oracle".to_string();
    let mut runs = DEFAULT_RUNS;
    let mut write = false;
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
            let p = repo.join("target/release/bund2");
            if !p.is_file() {
                return Err(format!(
                    "no bund2 at {}. Build it with:\n    cargo build --release -p bund2-cli",
                    p.strip_prefix(&repo).unwrap_or(&p).display()
                ));
            }
            p
        }
        other => return Err(format!("unknown --target `{other}`; use oracle or bund2")),
    };

    let suite = golden::read_suite(&repo)?;
    let cwd = repo.join("reference/Bund");

    println!("# cargo xtask bench\n");
    println!(
        "Wall-clock baseline over the {} suite programs, {runs} runs each,",
        suite.len()
    );
    println!("target `{target}`.\n");
    println!("Not Criterion: this times a subprocess end to end, where cost is");
    println!("dominated by process start and stdlib registration. Criterion");
    println!("measures a function in-process and would measure the wrong thing");
    println!("precisely. It becomes the right tool when Bund2 has an in-process");
    println!("interpreter to microbenchmark.\n");
    println!("Min is the headline — it is the least contaminated by scheduling");
    println!("noise. Median and max are shown so the spread is visible.\n");

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

    println!("## totals\n");
    println!("  programs timed          {:>9}", ok.len());
    println!("  failed to spawn         {:>9}", timings.len() - ok.len());
    println!("  sum of per-program min  {:>9.1} ms", ms(total_min));
    println!("  sum of per-program med  {:>9.1} ms", ms(total_median));
    if !ok.is_empty() {
        println!(
            "  mean per program (min)  {:>9.1} ms",
            ms(total_min) / ok.len() as f64
        );
    }
    println!();

    let mut slowest: Vec<&Timing> = ok.clone();
    slowest.sort_by(|a, b| b.min().cmp(&a.min()));
    println!("## slowest 15 by min\n");
    println!(
        "  {:<58}{:>9}{:>9}{:>9}",
        "program", "min ms", "med ms", "max ms"
    );
    for t in slowest.iter().take(15) {
        println!(
            "  {:<58}{:>9.1}{:>9.1}{:>9.1}",
            t.program.trim_start_matches("reference/Bund/"),
            ms(t.min()),
            ms(t.median()),
            ms(t.max())
        );
    }
    println!();

    // A floor worth knowing: how much of each run is just starting the binary.
    println!("## interpretation\n");
    if let Some(fastest) = ok.iter().map(|t| t.min()).min() {
        println!(
            "  The fastest program takes {:.1} ms. That is close to the floor for",
            ms(fastest)
        );
        println!("  spawning this binary and registering its stdlib, so most of the");
        println!("  per-program figure is fixed cost, not interpretation. Comparing");
        println!("  Bund2 against this baseline measures both together — worth");
        println!("  separating before drawing conclusions about the interpreter.\n");
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
        println!("wrote docs/bench-baseline.md");
    } else {
        println!("  Nothing written. Pass --write to record docs/bench-baseline.md.\n");
    }

    Ok(())
}
