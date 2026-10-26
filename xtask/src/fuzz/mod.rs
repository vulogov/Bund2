//! `cargo xtask fuzz` — grammar-directed differential fuzzing, D218.
//!
//! The research's §3.8 (`docs/research/04-consolidated-architecture.md`):
//! generate programs from `bund.pest`, run each under every tier, and compare
//! every stack, the workbench and the error. `conform` compares the same
//! things over a fixed corpus; this compares them over programs nobody wrote,
//! which is where a divergence a hand-written test misses is found.
//!
//! Each program runs three ways:
//!
//! - **Tier 0**, the default build;
//! - **Tier 1**, the `jit` build at `--jit-threshold 1`, so every body
//!   compiles on its first evaluation;
//! - **the oracle**, unless `--no-oracle`.
//!
//! Tier 0 against Tier 1 is exact, and any difference is a Bund2 bug. Tier 0
//! against the oracle is by parts (`compare`), and a difference is one of
//! three things for a person to decide: a Bund2 bug, a new defect in the
//! reference, or a deviation already approved. D218 chose both comparisons,
//! and chose to run locally only, since that triage cannot be automated.
//!
//! **What is never a finding.** A program the oracle answers differently on
//! two runs (the second run is taken only when Bund2 disagrees with the
//! first); a program the oracle aborts on, which D37 says Bund2 declines to
//! reproduce; and a program that outlasts the time or memory bound in every
//! engine. Each is counted, so a run that found nothing says how much it
//! compared.
//!
//! **A finding is cut down before it is reported**, a line at a time, for as
//! long as the same kind of finding survives, and written to
//! `target/fuzz/findings/` with the seed that produced it.

mod compare;
mod generate;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::corpus::classify::{Classifier, Effect};

const HELP: &str = "\
cargo xtask fuzz [--seed N] [--count N] [--statements N] [--timeout SECS]
                 [--no-oracle] [--replay FILE]

Generate programs from bund.pest's terms, run each under Tier 0, Tier 1 at
--jit-threshold 1 and the oracle, and report where they part (D218).
Findings are cut down and written to target/fuzz/findings/.
  --seed N        The first program's seed; program i uses N + i. Default:
                  the clock, printed so the run can be repeated.
  --count N       Programs to generate (default 200).
  --statements N  Statements per program, at most (default 12).
  --timeout SECS  Per run (default 10). Memory is bounded at 1.5 GB.
  --no-oracle     Compare the tiers only.
  --replay FILE   Run one program three ways and print what each said.
";

/// What a run may hold before it is stopped. The oracle has no guard of its
/// own, and D171 records eight fuzz runs holding 0.8 to 2.2 GB each beside a
/// runaway that took the host down.
const RSS_CAP_KB: u64 = 1_536 * 1024;

/// A run that ended, with its normalised output.
struct Ran {
    output: String,
    status: i32,
}

/// How a bounded run ended.
enum Ended {
    Ran(Ran),
    OutOfTime,
    OutOfMemory,
}

/// The three engines, as paths and the arguments each takes after `--file`.
struct Engines {
    tier0: PathBuf,
    tier1: PathBuf,
    oracle: Option<PathBuf>,
    timeout: Duration,
    work: PathBuf,
}

/// What a program showed, and what kind of finding it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    /// Tier 0 and Tier 1 disagree.
    Tiers,
    /// Bund2 stopped on its own defect.
    Broke,
    /// One engine failed and the other did not.
    Verdict,
    /// Both ran clean, or both failed, and what they wrote or left differs.
    State,
    /// Both failed at the same point with different messages.
    Message,
}

impl Class {
    fn as_str(self) -> &'static str {
        match self {
            Class::Tiers => "tiers",
            Class::Broke => "broke",
            Class::Verdict => "verdict",
            Class::State => "state",
            Class::Message => "message",
        }
    }
}

/// Why a program was compared to nothing, counted for the summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Skip {
    Bund2Bound,
    OracleBound,
    OracleAborted,
    OracleUnstable,
    Bund2Unstable,
}

impl Skip {
    fn as_str(self) -> &'static str {
        match self {
            Skip::Bund2Bound => "outlasted the bound in both tiers",
            Skip::OracleBound => "outlasted the bound in the oracle",
            Skip::OracleAborted => "the oracle aborted (D37)",
            Skip::OracleUnstable => "the oracle disagreed with itself",
            Skip::Bund2Unstable => "a tier disagreed with itself",
        }
    }
}

/// The outcome of one program.
enum Verdict {
    Agree,
    /// Agreement between the tiers, nothing to compare with the oracle.
    Skipped(Skip),
    Finding(Class, String),
}

pub fn run(args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let mut seed: Option<u64> = None;
    let mut count = 200usize;
    let mut statements = 12usize;
    let mut timeout = 10u64;
    let mut with_oracle = true;
    let mut replay: Option<PathBuf> = None;
    let mut it = args.iter();
    let number = |v: Option<&String>, flag: &str| -> Result<u64, String> {
        v.ok_or_else(|| format!("{flag} needs a number"))?
            .parse::<u64>()
            .map_err(|_| format!("{flag} takes a number"))
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "--seed" => seed = Some(number(it.next(), "--seed")?),
            "--count" => count = number(it.next(), "--count")? as usize,
            "--statements" => statements = (number(it.next(), "--statements")? as usize).max(1),
            "--timeout" => timeout = number(it.next(), "--timeout")?.max(1),
            "--no-oracle" => with_oracle = false,
            "--replay" => {
                replay = Some(PathBuf::from(
                    it.next().ok_or("--replay needs a program file")?,
                ))
            }
            "-h" | "--help" => {
                bund2_api::say!("{HELP}");
                return Ok(());
            }
            other => return Err(format!("unknown argument `{other}`\n\n{HELP}")),
        }
    }

    let engines = build_engines(&repo, with_oracle, Duration::from_secs(timeout))?;

    if let Some(file) = replay {
        return replay_one(&engines, &file);
    }

    let vocab = vocabulary(&repo, &engines.tier0)?;
    let seed = seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_secs())
    });

    bund2_api::sayln!("# cargo xtask fuzz\n");
    bund2_api::sayln!(
        "  seed {seed}, {count} programs of up to {statements} statements, {} words",
        vocab.len()
    );
    bund2_api::sayln!(
        "  comparing: Tier 0 with Tier 1 at --jit-threshold 1{}\n",
        if with_oracle {
            ", and Tier 0 with the oracle"
        } else {
            ""
        }
    );

    // One directory a seed, emptied first, so a rerun's findings are not
    // read beside an earlier run's.
    let findings_dir = repo.join("target/fuzz/findings").join(seed.to_string());
    let _ = std::fs::remove_dir_all(&findings_dir);
    std::fs::create_dir_all(&findings_dir)
        .map_err(|e| format!("creating {}: {e}", findings_dir.display()))?;

    let mut agreed = 0usize;
    let mut skipped: BTreeMap<Skip, usize> = BTreeMap::new();
    // One example per kind and per word the cut-down program ends on, so a
    // bug found a hundred times is reported once with its count.
    let mut found: BTreeMap<(Class, String), (usize, PathBuf)> = BTreeMap::new();
    // The word each unstable program was cut down to, which is most likely
    // the one answering at random.
    let mut unstable: BTreeMap<String, usize> = BTreeMap::new();
    let started = Instant::now();
    let (mut compiled_programs, mut compiled_bodies) = (0usize, 0usize);

    for i in 0..count {
        let program_seed = seed.wrapping_add(i as u64);
        let mut g = generate::Gen::new(program_seed, &vocab);
        let n = 1 + g.rng.below(statements);
        let lines = g.program(n);
        let (verdict, bodies) = judge_counting(&engines, &lines)?;
        if bodies > 0 {
            compiled_programs += 1;
            compiled_bodies += bodies;
        }
        match verdict {
            Verdict::Agree => agreed += 1,
            Verdict::Skipped(s) => {
                *skipped.entry(s).or_default() += 1;
                // An engine that answers one program two ways is running a
                // word the vocabulary should not hold, so the program is
                // kept, uncut, to say which.
                if matches!(s, Skip::OracleUnstable | Skip::Bund2Unstable) {
                    let (cut, _) = cut_down(&engines, lines, |v| match v {
                        Verdict::Skipped(x) if x == s => Some(String::new()),
                        _ => None,
                    })?;
                    if let Some(word) = cut.last().and_then(|l| l.split_whitespace().last()) {
                        *unstable.entry(word.to_string()).or_default() += 1;
                    }
                    let file = findings_dir.join(format!("{i}-unstable.bund"));
                    let body = format!(
                        "// cargo xtask fuzz --seed {program_seed} --count 1, cut down\n// {}\n{}\n",
                        s.as_str(),
                        cut.join("\n")
                    );
                    std::fs::write(&file, body)
                        .map_err(|e| format!("writing {}: {e}", file.display()))?;
                }
            }
            Verdict::Finding(class, _) => {
                let (cut, detail) = cut_down(&engines, lines, |v| match v {
                    Verdict::Finding(c, d) if c == class => Some(d),
                    _ => None,
                })?;
                let detail = detail.unwrap_or_else(|| "did not repeat when run again".into());
                let last = cut
                    .last()
                    .and_then(|l| l.split_whitespace().last())
                    .unwrap_or("")
                    .to_string();
                let key = (class, last);
                if let Some(entry) = found.get_mut(&key) {
                    entry.0 += 1;
                    continue;
                }
                let file = findings_dir.join(format!("{i}-{}.bund", class.as_str()));
                let body = format!(
                    "// cargo xtask fuzz --seed {program_seed} --count 1, cut down\n// {}: {}\n{}\n",
                    class.as_str(),
                    detail.replace('\n', "\n// "),
                    cut.join("\n")
                );
                std::fs::write(&file, body)
                    .map_err(|e| format!("writing {}: {e}", file.display()))?;
                bund2_api::sayln!(
                    "  {:<8} {}",
                    class.as_str(),
                    file.strip_prefix(&repo).unwrap_or(&file).display()
                );
                found.insert(key, (1, file));
            }
        }
    }

    let total_findings: usize = found.values().map(|(n, _)| n).sum();
    bund2_api::sayln!(
        "\n  {count} programs in {:.0} s: {agreed} agreed, {total_findings} found, {} compared to nothing",
        started.elapsed().as_secs_f64(),
        skipped.values().sum::<usize>()
    );
    bund2_api::sayln!(
        "  Tier 1 compiled {compiled_bodies} bodies in {compiled_programs} of {count} programs"
    );
    if compiled_programs == 0 {
        // D141: not a failure of Bund2, and not evidence for the tier either.
        bund2_api::sayln!("    no body compiled, so the tiers were not compared at all");
    }
    for (s, n) in &skipped {
        bund2_api::sayln!("    {n:>5}  {}", s.as_str());
    }
    if !unstable.is_empty() {
        bund2_api::sayln!("\n  answered two ways, by the word each was cut down to:");
        for (w, n) in &unstable {
            bund2_api::sayln!("    {n:>5}  {w}");
        }
    }
    if !found.is_empty() {
        bund2_api::sayln!("\n  findings, one file per kind and last word:");
        for ((class, word), (n, file)) in &found {
            bund2_api::sayln!(
                "    {:<8} {:<24} x{n:<4} {}",
                class.as_str(),
                word,
                file.strip_prefix(&repo).unwrap_or(file).display()
            );
        }
        bund2_api::sayln!("\n  Each is a Bund2 bug, a defect in the reference, or an approved");
        bund2_api::sayln!("  deviation; `--replay <file>` shows all three runs.");
    }
    Ok(())
}

/// Build both tiers and copy each out of `target/release`, which the second
/// build overwrites.
fn build_engines(repo: &Path, with_oracle: bool, timeout: Duration) -> Result<Engines, String> {
    let dir = repo.join("target/fuzz");
    let work = dir.join("work");
    std::fs::create_dir_all(&work).map_err(|e| format!("creating {}: {e}", work.display()))?;
    let copy = |from: PathBuf, name: &str| -> Result<PathBuf, String> {
        let to = dir.join(name);
        std::fs::copy(&from, &to).map_err(|e| format!("copying {}: {e}", from.display()))?;
        Ok(to)
    };
    let tier1 = copy(crate::buildcli::bund2(repo, true, "jit")?, "bund2-tier1")?;
    let tier0 = copy(crate::buildcli::bund2(repo, true, "")?, "bund2-tier0")?;
    let oracle = if with_oracle {
        let o = repo.join("target/oracle/release/bund");
        if !o.is_file() {
            return Err(
                "no oracle at target/oracle/release/bund. Build it out of tree (CLAUDE.md), \
                 or pass --no-oracle"
                    .into(),
            );
        }
        Some(o)
    } else {
        None
    };
    Ok(Engines {
        tier0,
        tier1,
        oracle,
        timeout,
        work,
    })
}

/// The words a program may call: every word the reference registers that is
/// in scope, pure or printing, implemented by Bund2, and not left out for a
/// reason of its own (`generate::LEFT_OUT`, `corpus::NO_GOLDEN_CAN_HOLD`).
fn vocabulary(repo: &Path, tier0: &Path) -> Result<Vec<generate::Word>, String> {
    let reg = crate::corpus::registry::scan(repo, crate::corpus::REGISTRY_ROOTS);
    let out = std::process::Command::new(tier0)
        .arg("words")
        .output()
        .map_err(|e| format!("running {} words: {e}", tier0.display()))?;
    let implemented: BTreeSet<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .collect();
    let left_out: BTreeSet<&str> = generate::LEFT_OUT
        .iter()
        .map(|(w, _)| *w)
        .chain(crate::corpus::NO_GOLDEN_CAN_HOLD.iter().map(|(w, _)| *w))
        .collect();
    let arity = generate::read_arity(repo);
    let mut words = Vec::new();
    for w in reg.word_names() {
        if !implemented.contains(w) || left_out.contains(w) {
            continue;
        }
        if Classifier::deferral_of(&reg, w).is_some() {
            continue;
        }
        if !matches!(
            Classifier::effect_of(&reg, w).map(|(e, _)| e),
            Some(Effect::Pure | Effect::Stdout)
        ) {
            continue;
        }
        let (consumes, kind) = arity.get(w).copied().unwrap_or((None, generate::Kind::Any));
        words.push(generate::Word {
            name: w.to_string(),
            consumes,
            kind,
        });
    }
    if words.is_empty() {
        return Err("no word qualified for the vocabulary; is reference/ checked out?".into());
    }
    Ok(words)
}

/// Run one program three ways and say how they compare.
fn judge(engines: &Engines, lines: &[String]) -> Result<Verdict, String> {
    Ok(judge_counting(engines, lines)?.0)
}

/// [`judge`], and how many bodies Tier 1 compiled, from its `--stats` line —
/// D141's lesson: a tier run that compiled nothing compared nothing.
fn judge_counting(engines: &Engines, lines: &[String]) -> Result<(Verdict, usize), String> {
    let mut tier = crate::conform::TierTotal::default();
    let verdict = judge_with(engines, lines, &mut tier)?;
    Ok((verdict, tier.bodies))
}

fn judge_with(
    engines: &Engines,
    lines: &[String],
    tier: &mut crate::conform::TierTotal,
) -> Result<Verdict, String> {
    let src = format!("{}\n", lines.join("\n"));
    let program = format!("{src}{}", crate::golden::capture_epilogue(&src));
    let file = engines.work.join("case.bund");
    std::fs::write(&file, &program).map_err(|e| format!("writing {}: {e}", file.display()))?;

    let bund2_args = |tier1: bool| {
        let mut a = vec!["--report", "trace", "--raw-values"];
        if tier1 {
            a.extend(["--jit-threshold", "1", "--stats"]);
        }
        a
    };
    let t0 = bounded(engines, &engines.tier0, &bund2_args(false))?;
    let t1 = match bounded(engines, &engines.tier1, &bund2_args(true))? {
        Ended::Ran(mut r) => {
            tier.take(&mut r.output);
            Ended::Ran(r)
        }
        other => other,
    };

    let t0 = match (t0, t1) {
        (Ended::Ran(a), Ended::Ran(b)) => {
            if a.status != b.status || a.output != b.output {
                // A word the classification missed may answer at random;
                // then a tier disagrees with itself too, and it is that.
                // Twice each, since an order drawn from a hash agrees with
                // itself often (F74 agreed five runs of six).
                for _ in 0..2 {
                    for (tier1, first) in [(false, &a), (true, &b)] {
                        let exe = if tier1 {
                            &engines.tier1
                        } else {
                            &engines.tier0
                        };
                        if let Ended::Ran(mut again) = bounded(engines, exe, &bund2_args(tier1))? {
                            if tier1 {
                                crate::conform::TierTotal::default().take(&mut again.output);
                            }
                            if again.status != first.status || again.output != first.output {
                                return Ok(Verdict::Skipped(Skip::Bund2Unstable));
                            }
                        }
                    }
                }
                return Ok(Verdict::Finding(
                    Class::Tiers,
                    format!(
                        "Tier 0 exit {}, Tier 1 exit {}; {}",
                        a.status,
                        b.status,
                        compare::first_difference(&a.output, &b.output)
                            .replace("bund2: ", "tier0: ")
                            .replace("oracle:", "tier1: ")
                    ),
                ));
            }
            a
        }
        (Ended::Ran(_), _) | (_, Ended::Ran(_)) => {
            return Ok(Verdict::Finding(
                Class::Tiers,
                "one tier ran out of time or memory and the other finished".into(),
            ));
        }
        _ => return Ok(Verdict::Skipped(Skip::Bund2Bound)),
    };

    if let Some(why) = compare::bund2_broke(t0.status, &t0.output) {
        // A native's dependency that panics in Bund2 usually panics in the
        // oracle too, which makes it the reference's defect that Bund2
        // survived (D37) — still reported, since Bund2 blames itself.
        let oracle_too = match &engines.oracle {
            Some(o) => matches!(
                bounded(engines, o, &[])?,
                Ended::Ran(r) if compare::oracle_aborted(r.status, &r.output)
            ),
            None => false,
        };
        return Ok(Verdict::Finding(
            Class::Broke,
            format!(
                "Bund2 stopped on {why}{}",
                if oracle_too {
                    "; the oracle aborts on it too"
                } else {
                    ""
                }
            ),
        ));
    }

    let Some(oracle) = &engines.oracle else {
        return Ok(Verdict::Agree);
    };
    let o = match bounded(engines, oracle, &[])? {
        Ended::Ran(r) => Ran {
            output: compare::without_log_lines(&r.output),
            status: r.status,
        },
        _ => return Ok(Verdict::Skipped(Skip::OracleBound)),
    };
    if compare::oracle_aborted(o.status, &o.output) {
        return Ok(Verdict::Skipped(Skip::OracleAborted));
    }
    let Some((class, detail)) = against_oracle(&t0, &o) else {
        return Ok(Verdict::Agree);
    };
    // Only now is the oracle asked twice: a program it answers differently
    // on two runs pins nothing, as a golden it disagrees with itself on
    // would not be captured.
    match bounded(engines, oracle, &[])? {
        Ended::Ran(again)
            if compare::without_log_lines(&again.output) == o.output
                && again.status == o.status =>
        {
            Ok(Verdict::Finding(class, detail))
        }
        _ => Ok(Verdict::Skipped(Skip::OracleUnstable)),
    }
}

/// Tier 0 against the oracle, by parts.
fn against_oracle(t0: &Ran, o: &Ran) -> Option<(Class, String)> {
    let b = compare::bund2_parts(&t0.output);
    let r = compare::oracle_parts(&o.output);
    // F60 and F72: where Bund2 refuses to close a context never opened, the
    // reference drops a stack and runs on; only the output before compares.
    if let Some(be) = &b.error
        && compare::ruled_stop(be)
    {
        // The reference runs on past this point, so it may print more.
        if !r.before.starts_with(&b.before) {
            return Some((
                Class::State,
                format!(
                    "before any error, {}",
                    compare::first_difference(&b.before, &r.before)
                ),
            ));
        }
        return None;
    }
    match (&b.error, &r.error) {
        (Some(e), None) => {
            return Some((
                Class::Verdict,
                format!("Bund2 failed and the oracle did not: {e}"),
            ));
        }
        (None, Some(e)) => {
            return Some((
                Class::Verdict,
                format!("the oracle failed and Bund2 did not: {e}"),
            ));
        }
        _ => {}
    }
    if b.before != r.before {
        return Some((
            Class::State,
            format!(
                "before any error, {}",
                compare::first_difference(&b.before, &r.before)
            ),
        ));
    }
    // F18's guards leave what the reference's second pull spent, so the dump
    // differs with the message, by ruling.
    if let (Some(be), Some(re)) = (&b.error, &r.error)
        && compare::f18_guard(be, re)
    {
        return None;
    }
    if b.after != r.after {
        return Some((
            Class::State,
            format!(
                "in the dump, {}",
                compare::first_difference(&b.after, &r.after)
            ),
        ));
    }
    if let (Some(be), Some(re)) = (&b.error, &r.error)
        && be != re
        && compare::approved_message(be, re).is_some()
    {
        return None;
    }
    if b.error != r.error {
        return Some((
            Class::Message,
            format!(
                "bund2:  {}\n oracle: {}",
                b.error.unwrap_or_default(),
                r.error.unwrap_or_default()
            ),
        ));
    }
    None
}

/// Cut a program down a line at a time, for as long as `keep` still sees
/// what it was cut for, and return what it saw last. Bounded, since every
/// attempt runs every engine. `None` when the whole program no longer shows
/// it, which an engine answering at random can do.
fn cut_down(
    engines: &Engines,
    mut lines: Vec<String>,
    keep: impl Fn(Verdict) -> Option<String>,
) -> Result<(Vec<String>, Option<String>), String> {
    let Some(mut detail) = keep(judge(engines, &lines)?) else {
        return Ok((lines, None));
    };
    let mut budget = 120usize;
    let mut changed = true;
    while changed && budget > 0 {
        changed = false;
        let mut i = lines.len();
        while i > 0 && budget > 0 && lines.len() > 1 {
            i -= 1;
            let mut fewer = lines.clone();
            fewer.remove(i);
            budget -= 1;
            if let Some(d) = keep(judge(engines, &fewer)?) {
                lines = fewer;
                detail = d;
                changed = true;
            }
        }
    }
    Ok((lines, Some(detail)))
}

/// `--replay`: one program, three runs, printed whole.
fn replay_one(engines: &Engines, file: &Path) -> Result<(), String> {
    let src =
        std::fs::read_to_string(file).map_err(|e| format!("reading {}: {e}", file.display()))?;
    let lines: Vec<String> = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .map(str::to_string)
        .collect();
    let verdict = judge(engines, &lines)?;
    for (name, exe, args) in [
        (
            "tier 0",
            Some(&engines.tier0),
            vec!["--report", "trace", "--raw-values"],
        ),
        (
            "tier 1",
            Some(&engines.tier1),
            vec!["--report", "trace", "--raw-values", "--jit-threshold", "1"],
        ),
        ("oracle", engines.oracle.as_ref(), vec![]),
    ] {
        let Some(exe) = exe else { continue };
        bund2_api::sayln!("== {name}");
        match bounded(engines, exe, &args)? {
            Ended::Ran(r) => bund2_api::sayln!("{}\n-- exit {}", r.output, r.status),
            Ended::OutOfTime => bund2_api::sayln!("-- ran out of time"),
            Ended::OutOfMemory => bund2_api::sayln!("-- ran out of memory"),
        }
    }
    match verdict {
        Verdict::Agree => bund2_api::sayln!("\n== verdict: agree"),
        Verdict::Skipped(s) => {
            bund2_api::sayln!("\n== verdict: compared to nothing, {}", s.as_str())
        }
        Verdict::Finding(c, d) => bund2_api::sayln!("\n== verdict: {}\n{d}", c.as_str()),
    }
    Ok(())
}

/// Run `case.bund` under one engine, within the time and memory bound.
///
/// The output is normalised as a golden's is (F14, F15, ANSI), and a
/// context's stack name as an id is (`compare::stack_ids`); the host
/// is pinned as a capture pins it, so a difference is not the host's.
///
/// **The path is absolute.** The oracle reads `--file` as a `file://` URL
/// through curl, which rejects a relative one and runs nothing — so given
/// `case.bund` every program "succeeded" in the oracle and failed in Bund2.
fn bounded(engines: &Engines, exe: &Path, extra: &[&str]) -> Result<Ended, String> {
    use std::io::Read;
    let mut child = std::process::Command::new(exe)
        .arg("script")
        .arg("--file")
        .arg(engines.work.join("case.bund"))
        .args(extra)
        .envs(crate::golden::PINNED_HOST)
        .current_dir(&engines.work)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawning {}: {e}", exe.display()))?;
    let mut out_pipe = child.stdout.take().ok_or("no stdout pipe")?;
    let mut err_pipe = child.stderr.take().ok_or("no stderr pipe")?;
    // Drained while the child runs, or a program writing more than a pipe
    // holds would block and read as a timeout (F43).
    let out_h = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = out_pipe.read_to_end(&mut b);
        b
    });
    let err_h = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = err_pipe.read_to_end(&mut b);
        b
    });
    let start = Instant::now();
    let mut polls = 0u32;
    let ended = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) => {}
            Err(e) => return Err(format!("waiting on {}: {e}", exe.display())),
        }
        if start.elapsed() > engines.timeout {
            break None;
        }
        polls += 1;
        // Most programs end within milliseconds, so memory is asked about
        // only once a run has lasted, and then every tenth of a second.
        if polls.is_multiple_of(20)
            && start.elapsed() > Duration::from_millis(200)
            && over_memory(child.id())
        {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out_h.join();
            let _ = err_h.join();
            return Ok(Ended::OutOfMemory);
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let Some(status) = ended else {
        let _ = child.kill();
        let _ = child.wait();
        let _ = out_h.join();
        let _ = err_h.join();
        return Ok(Ended::OutOfTime);
    };
    let mut raw = String::from_utf8_lossy(&out_h.join().unwrap_or_default()).into_owned();
    raw.push_str(&String::from_utf8_lossy(&err_h.join().unwrap_or_default()));
    Ok(Ended::Ran(Ran {
        output: compare::stack_ids(&crate::golden::normalise(&raw)),
        status: status.code().unwrap_or(-1),
    }))
}

/// Whether a process holds more than [`RSS_CAP_KB`] resident, asked of `ps`,
/// which macOS and Linux both have; where it cannot answer, the time bound
/// is the only one.
fn over_memory(pid: u32) -> bool {
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<u64>()
                .ok()
        })
        .is_some_and(|kb| kb > RSS_CAP_KB)
}
