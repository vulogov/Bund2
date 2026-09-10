//! `cargo xtask depth` — **RFC-0003 criterion 2 and RFC-0009 criterion 3**.
//!
//! Bund2 must not let a deep Bund program abort the process. A stack overflow
//! is not an error a program can catch, does not reach a `Reporter`, and takes
//! the user's state with it — which is exactly what D37 forbids and what
//! RFC-0003's flat frame loop exists to prevent.
//!
//! # Three axes, because they overflow for different reasons
//!
//! RFC-0003 §S4 names all three and addresses only the first.
//!
//! - **call** — a word that calls itself. Today `lambda_eval` applies each body
//!   element through `apply`, which re-enters `lambda_eval`
//!   (`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:13-14`), so
//!   Rust depth tracks Bund depth. The frame loop removes this.
//! - **nesting** — deeply nested brackets. `parse_pair` recurses through
//!   `lambda.rs:11`, `list.rs:11` and `ctx.rs:11`, so this overflows at *parse*
//!   time. RFC-0003 excludes it deliberately; measured here so the exclusion is
//!   a number rather than a claim.
//! - **class** — a deep class chain. `make_bund_object` calls itself per
//!   superclass (`reference/rust_multistackvm/src/stdlib/bund_object.rs:50`),
//!   which RFC-0009 §S3 turns into frames.
//!
//! # This tool is expected to fail today
//!
//! Nothing here is implemented yet. A criterion that cannot fail before the
//! work and pass after it measures nothing, so the tool reports honestly and
//! `--require <axis>` is what a CI gate would use once an axis is meant to
//! hold.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// What became of one run.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// Ran to completion.
    Completed,
    /// Failed the way a Bund program is allowed to fail: a diagnostic, and an
    /// orderly exit. This is a **pass** — the criterion is "does not abort",
    /// not "succeeds".
    BundError,
    /// The process died on a signal. On every platform this project targets
    /// that is what a blown Rust stack looks like.
    Aborted(String),
    /// Still running when the clock ran out.
    TimedOut,
    /// The program could not be built for this axis, because the feature it
    /// exercises is not implemented.
    Unsupported(String),
}

impl Outcome {
    fn passed(&self) -> bool {
        matches!(self, Outcome::Completed | Outcome::BundError)
    }

    fn label(&self) -> String {
        match self {
            Outcome::Completed => "completed".into(),
            Outcome::BundError => "reported a Bund-level error".into(),
            Outcome::Aborted(why) => format!("ABORTED — {why}"),
            Outcome::TimedOut => "TIMED OUT".into(),
            Outcome::Unsupported(why) => format!("unsupported — {why}"),
        }
    }
}

/// The word a run died on, when it died for want of one.
///
/// The report wraps text in a table, so the name can be followed by padding
/// and a border; take the token before `not registered` and trim.
fn missing_word(text: &str) -> Option<String> {
    let at = text.find(" not registered")?;
    let head = &text[..at];
    let word = head.rsplit(|c: char| c.is_whitespace()).next()?;
    let word = word.trim_matches(|c: char| c == '`' || c == '│' || c == '"');
    if word.is_empty() {
        None
    } else {
        Some(word.to_string())
    }
}


/// The level the `loop` axis reached before Tier 0's floor refused to nest
/// further — RFC-0005 criterion 11.
///
/// The counter the program descends is on the stack when the error is
/// reported, so the level is `n` minus it. The stack box also holds the `0`
/// `times` pushed for its iteration, which is why the counter is taken as the
/// largest integer in the box rather than by position. `None` when there is no
/// dump — the run completed, or failed some other way.
fn loop_level(n: usize, text: &str) -> Option<usize> {
    let dump = text.split("[BUND]  Content of the stack").nth(1)?;
    let counter = dump
        .lines()
        .filter_map(|l| {
            l.trim_matches(|c: char| c == '│' || c.is_whitespace())
                .parse::<usize>()
                .ok()
        })
        .max()?;
    n.checked_sub(counter)
}

/// Run one program under a wall clock, and classify how it ended.
///
/// Returns the output beside the outcome. `dump_stack` keeps the error
/// report's stack box, which is where the `loop` axis reads its level from;
/// every other axis runs without it.
fn run(
    bin: &Path,
    repo: &Path,
    src: &str,
    budget: Duration,
    dump_stack: bool,
) -> Result<(Outcome, String), String> {
    let work = repo.join("target/depth");
    std::fs::create_dir_all(&work).map_err(|e| format!("creating {}: {e}", work.display()))?;
    let file = work.join("case.bund");
    std::fs::write(&file, src).map_err(|e| format!("writing {}: {e}", file.display()))?;

    let started = Instant::now();
    let mut cmd = Command::new(bin);
    cmd.arg("script").arg("--file").arg(&file);
    if !dump_stack {
        cmd.arg("--no-dump-stack");
    }
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawning bund2: {e}"))?;

    // Poll rather than `wait`, so a hang is a result instead of a hung xtask.
    loop {
        match child.try_wait().map_err(|e| format!("waiting: {e}"))? {
            Some(status) => {
                let out = child.wait_with_output().map_err(|e| format!("output: {e}"))?;
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    if let Some(sig) = status.signal() {
                        let why = if text.contains("overflowed its stack") {
                            format!("signal {sig}, stack overflow reported")
                        } else {
                            format!("signal {sig}")
                        };
                        return Ok((Outcome::Aborted(why), text));
                    }
                }
                if text.contains("overflowed its stack") {
                    return Ok((Outcome::Aborted("stack overflow reported".into()), text));
                }
                // **A missing word is not a pass.** An axis whose feature is
                // unimplemented fails cleanly for a reason that has nothing to
                // do with depth, and counting that as "does not abort" would
                // make the axis measure nothing — it would pass today and
                // report nothing about the day the feature lands.
                if let Some(w) = missing_word(&text) {
                    return Ok((Outcome::Unsupported(format!("`{w}` is not implemented")), text));
                }
                // An orderly exit. A Bund failure is reported and exits 0
                // (D36), so the diagnostic in the output is what distinguishes
                // the two — not the code.
                if text.contains("│ Error") || text.contains("Error occured") {
                    return Ok((Outcome::BundError, text));
                }
                return Ok((Outcome::Completed, text));
            }
            None => {
                if started.elapsed() > budget {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok((Outcome::TimedOut, String::new()));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

/// A word that calls itself `n` times, counting down on the stack.
fn call_program(n: usize) -> String {
    // `1 swap -`, not `1 -`. **Arithmetic reverses its operands** — the top of
    // the stack is the left side, so `n 1 -` computes `1 - n` and the counter
    // oscillates instead of descending, overflowing at any depth for a reason
    // that has nothing to do with recursion. `swap` puts `n` back on top.
    //
    // `?true` runs the branch when the condition holds, so the recursion stops
    // at zero without needing any word this axis is not about.
    format!(
        ":Down {{ 1 swap - dup 0 != {{ Down }} ?true }} register\n{n} Down\n",
        n = n
    )
}

/// A word that calls itself **through `times`**, `n` levels deep — F85's
/// shape.
///
/// Direct recursion (`call`) runs on RFC-0003's heap frames; this does not.
/// `times` runs its body synchronously, so every level spends Rust stack, and
/// until RFC-0005 §S8's Tier 0 floor it aborted the process. The floor turns
/// that into a Bund-level error, which is a pass here; an abort is not.
fn loop_program(n: usize) -> String {
    format!(
        ":Down {{ 1 swap - dup 0 != {{ 1 {{ drop Down }} times }} ?true }} register\n{n} Down\n",
        n = n
    )
}

/// `n` nested lambdas. Overflows the *parser*, not the evaluator.
fn nesting_program(n: usize) -> String {
    let mut s = String::with_capacity(n * 4 + 8);
    for _ in 0..n {
        s.push_str("{ ");
    }
    s.push('1');
    for _ in 0..n {
        s.push_str(" }");
    }
    s.push('\n');
    s
}

/// A chain of `n` classes, each naming the previous as its parent.
fn class_program(n: usize) -> String {
    let mut s = String::new();
    s.push_str(":C0 class :.class_name \"C0\" set register\n");
    for i in 1..n {
        s.push_str(&format!(
            ":C{i} class :.class_name \"C{i}\" set \".super\" [ :C{prev} ] set register\n",
            i = i,
            prev = i - 1
        ));
    }
    s.push_str(&format!(":C{} object\n", n - 1));
    s
}

pub fn run_cmd(args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let mut depth = 100_000usize;
    let mut budget = 60u64;
    let mut require: Vec<String> = Vec::new();
    let mut features = String::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--depth" => {
                depth = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--depth needs a number")?
            }
            "--timeout" => {
                budget = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--timeout needs seconds")?
            }
            "--require" => require.push(it.next().cloned().ok_or("--require needs an axis")?),
            // **RFC-0005 criterion 11 needs this.** The criterion is that a
            // promoted recursion does not overflow the machine stack, which
            // means running the depth axes against a binary built *with the
            // tier*. Without a passthrough the command in that criterion could
            // not be written, and worse, `bund2_binary` below would silently
            // measure whichever binary happened to be lying in `target/`.
            "--features" => features = it.next().cloned().ok_or("--features needs a list")?,
            other => return Err(format!("unknown argument `{other}`")),
        }
    }

    // **The shared builder, not a private one.** `depth` kept its own
    // `build_bund2` after the other four measuring subcommands moved to
    // `crate::buildcli`, which is the drift that module exists to prevent —
    // and RFC-0005 criterion 2 claimed all five shared it. Release profile,
    // deliberately: a 100,000-deep call in a dev build measures debug-assertion
    // frame sizes, not the tier. `conform` measures dev. The two profiles are
    // a stated split, not an accident.
    let bin = crate::buildcli::bund2(&repo, true, &features)?;
    let budget = Duration::from_secs(budget);

    // Nesting is measured shallower on purpose: a parser that recurses will
    // die long before 100,000, and the number that matters is *where*.
    let cases: Vec<(&str, usize, String, &str)> = vec![
        (
            "call",
            depth,
            call_program(depth),
            "a word that calls itself — RFC-0003 §S4's frame loop",
        ),
        (
            "nesting",
            depth.min(10_000),
            nesting_program(depth.min(10_000)),
            "nested lambdas — parse-time recursion, excluded by §S4",
        ),
        (
            "class",
            depth.min(10_000),
            class_program(depth.min(10_000)),
            "a class chain — RFC-0009 §S3",
        ),
        (
            "loop",
            depth,
            loop_program(depth),
            "a word that calls itself through `times` — F85, RFC-0005 §S8's floor",
        ),
    ];

    println!("# cargo xtask depth\n");
    println!("  Does a deep Bund program abort the process? A stack overflow is");
    println!("  not catchable, never reaches a Reporter, and takes the user's");
    println!("  state with it — D37. Completing and failing cleanly both pass;");
    println!("  only aborting and hanging fail.\n");
    // Which binary this is about — the line `conform` prints too. A depth
    // result with no provenance is F80's failure in a new place.
    println!("  measured: {}\n", crate::buildcli::provenance(true, &features));

    let mut failed: Vec<String> = Vec::new();
    for (axis, n, src, what) in &cases {
        // **The `loop` axis also says how far it got.** Its pass is "reports,
        // does not abort", and that alone would read the same whether the
        // floor fired at level 10 or level 10,000. RFC-0005 criterion 11
        // compares the level with the tier on and off (D44), so the level is
        // printed rather than left in a stack box nobody sees.
        let is_loop = *axis == "loop";
        let (outcome, text) = run(&bin, &repo, src, budget, is_loop)?;
        let mark = if outcome.passed() { "ok  " } else { "FAIL" };
        let level = match (is_loop, &outcome) {
            (true, Outcome::BundError) => match loop_level(*n, &text) {
                Some(l) => format!(" — Tier 0's floor at level {l}"),
                None => " — level not found in the report".to_string(),
            },
            _ => String::new(),
        };
        println!("  {mark}  {axis:<8} depth {n:<7} {}{level}", outcome.label());
        println!("        {what}");
        if !outcome.passed() {
            failed.push((*axis).to_string());
        }
    }
    println!();

    if !failed.is_empty() {
        println!("  Failing axes: {}.\n", failed.join(", "));
        println!("  This is the expected reading until the frame loop lands.");
        println!("  RFC-0003 §S4 replaces the call axis; RFC-0009 §S3 the class");
        println!("  axis; the nesting axis is excluded by §S4 and measured here");
        println!("  so the exclusion is a number rather than a claim.\n");
    }

    // `--require` is the CI gate: name the axes that are meant to hold, and
    // fail only on those. Without it the tool reports and never fails, because
    // a measurement that always fails stops being read.
    let broken: Vec<&String> = require.iter().filter(|a| failed.contains(a)).collect();
    if !broken.is_empty() {
        return Err(format!(
            "required axis/axes not held: {}",
            broken
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for a in &require {
        if !["call", "nesting", "class", "loop"].contains(&a.as_str()) {
            return Err(format!("unknown axis `{a}` — expected call, nesting, class or loop"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The counter must **descend**. With arithmetic's reversed operands
    /// `n 1 -` is `1 - n`, which oscillates and overflows at any depth — a
    /// failure that would have been read as "the call axis aborts at 100".
    #[test]
    fn the_call_program_descends_rather_than_oscillating() {
        let p = call_program(3);
        assert!(p.contains("1 swap -"), "operands must be swapped: {p}");
        assert!(!p.contains("{ 1 - "), "the naive form is the trap: {p}");
        assert!(p.starts_with(":Down"), "registers before calling: {p}");
    }

    #[test]
    fn the_nesting_program_is_balanced() {
        let p = nesting_program(4);
        assert_eq!(p.matches('{').count(), 4);
        assert_eq!(p.matches('}').count(), 4);
        // Non-empty innermost, because `{}` does not parse (F51).
        assert!(p.contains("{ 1 }") || p.contains("1"), "{p}");
    }

    /// Each class names itself, per D25 — a class built from a value that
    /// carries no `.class_name` fails by decision, so a probe that omitted it
    /// would measure the wrong thing.
    #[test]
    fn the_class_program_chains_and_names_itself() {
        let p = class_program(3);
        assert!(p.contains(":C0 class"), "{p}");
        assert!(p.contains("\".super\" [ :C1 ]"), "C2 names C1: {p}");
        assert_eq!(p.matches(".class_name").count(), 3, "every class names itself");
    }

    /// Completing and failing cleanly are both passes; aborting and hanging
    /// are not. The criterion is "does not abort", not "succeeds".
    /// An axis whose feature is missing must not read as a pass.
    #[test]
    fn a_missing_word_is_unsupported_not_a_clean_failure() {
        assert_eq!(
            missing_word("│ Error ┆ class not registered  │").as_deref(),
            Some("class")
        );
        assert_eq!(missing_word("everything worked"), None);
        assert!(!Outcome::Unsupported("x".into()).passed());
    }

    /// The counter is the largest integer in the box, whichever row it is on;
    /// the `0` beside it is `times`'s iteration.
    #[test]
    fn the_loop_level_is_read_from_the_stack_box() {
        let report = "│ Error ┆ machine stack exhausted │\n\
                      [BUND]  Content of the stack\n\
                      ╭───────╮\n│ 89077 │\n├╌╌╌╌╌╌╌┤\n│ 0     │\n╰───────╯\n";
        assert_eq!(loop_level(100_000, report), Some(10_923));
        assert_eq!(loop_level(100_000, "completed, no report"), None);
    }

    #[test]
    fn only_aborting_and_hanging_fail() {
        assert!(Outcome::Completed.passed());
        assert!(Outcome::BundError.passed());
        assert!(!Outcome::Aborted("x".into()).passed());
        assert!(!Outcome::TimedOut.passed());
        assert!(!Outcome::Unsupported("x".into()).passed());
    }
}
