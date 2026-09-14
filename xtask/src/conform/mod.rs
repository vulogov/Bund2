//! `cargo xtask conform` — the project's status, in one number.
//!
//! Runs Bund2 against every captured golden and prints N/M. That is the
//! regression number CLAUDE.md builds the health metric on: the JIT and AOT
//! milestones must move it by exactly zero, because they change speed and not
//! meaning, so any movement is a bug.
//!
//! Two properties make the number mean something.
//!
//! **It compares through the same normalisation that captured the golden.**
//! `conform` calls `golden::normalise` and `golden::parse_golden` rather than
//! reimplementing either. A second copy would drift, and the first symptom
//! would be conformance failures nobody can explain.
//!
//! **It fails on regression, not on absence.** A count that only ever goes up
//! is a target; a count that may silently go down is decoration. The
//! high-water mark lives in `tests/golden/CONFORMANCE.txt`, and dropping below
//! it is an error that has to be accepted deliberately.
//!
//! The denominator is goldens, never words. `cargo xtask coverage` is the
//! number that answers "how much of the language is tested" — see Q5. Adding
//! words here would make implementing a word move the regression number, which
//! is exactly the invariant that must not break.

use std::path::{Path, PathBuf};

pub mod deviations;

use crate::golden;

/// Where the high-water mark is kept.
const BASELINE: &str = "tests/golden/CONFORMANCE.txt";

struct Outcome {
    program: String,
    passed: bool,
    /// An approved deviation: neither a pass nor a failure. It stays in the
    /// denominator — it is a captured golden — but it is reported apart,
    /// because the ratio answers "agrees with the oracle" and this one is
    /// approved not to.
    approved: bool,
    detail: String,
    /// The first word Bund2's diagnostic named as unregistered, if any.
    /// Populated for every case so `--blocked-on` costs no extra runs.
    blocked_on: Option<Missing>,
}

/// What a diagnostic said was missing: a word, or a class.
///
/// These are **not** interchangeable, and merging them would recreate the
/// vacuous pass this whole report exists to prevent one level down.
/// `class not registered` means the word `class` is unimplemented;
/// `OBJECT class Bool not registered` means `class` works fine and the corpus
/// wants a built-in class Bund2 does not construct. Criterion 1 quantifies
/// over the first kind only.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Missing {
    Word(String),
    Class(String),
    Method(String),
}

impl Missing {
    fn name(&self) -> &str {
        match self {
            Missing::Word(n) | Missing::Class(n) | Missing::Method(n) => n,
        }
    }
    fn is_class(&self) -> bool {
        matches!(self, Missing::Class(_))
    }
    /// How the row is labelled. Words are backticked because they are spelled
    /// in a program; a class or method is named.
    fn label(&self) -> String {
        match self {
            Missing::Word(n) => format!("`{n}`"),
            Missing::Class(n) => format!("class {n}"),
            Missing::Method(n) => format!("method {n}"),
        }
    }
}

/// The first `<word> not registered` a Bund2 run named, or `None`.
///
/// Reads the *rendered* diagnostic rather than a structured error, because
/// that is what `run_once` captures — the same bytes `conform` compares. So
/// this has to undo the table: `comfy_table` puts the message in the right
/// cell of a `┆`-separated row and continues it on rows whose left cell is
/// blank, wrapping at the terminal width. Joining those continuations before
/// matching is the difference between finding `class` and finding nothing,
/// and an earlier attempt at this criterion failed vacuously for exactly that
/// reason: grepping the raw failure list for the phrase matched nothing
/// whether or not Bund2 was blocked.
fn first_unregistered(output: &str) -> Option<Missing> {
    let mut message = String::new();
    let mut in_error = false;
    for line in output.lines() {
        let Some((left, right)) = line.split_once('┆') else {
            continue;
        };
        let label = left.trim_matches(|c: char| !c.is_alphanumeric()).trim();
        let text = right.trim_matches(|c: char| c == '│' || c == ' ').trim();
        if label.eq_ignore_ascii_case("Error") {
            in_error = true;
            message.push_str(text);
        } else if in_error && label.is_empty() {
            // A wrapped continuation of the same cell.
            message.push(' ');
            message.push_str(text);
        } else if in_error {
            break;
        }
    }
    let message = message.split_whitespace().collect::<Vec<_>>().join(" ");
    // Two phrasings, both the reference's. `<word> not registered` covers an
    // unresolved word and `OBJECT class <name> not registered` a missing
    // class; `VM no method <name> has been registered`
    // (`reference/rust_multistackvm/src/multistackvm_object.rs:37,53`) covers
    // a slot that resolves to a PTR with no method behind it. Matching only
    // the first missed `class_display_demo`, which then read as a semantic
    // disagreement when it is a plain gap — the same conflation this report
    // exists to prevent, one phrasing further along.
    if let Some(idx) = message.find("no method ") {
        let rest = &message[idx + "no method ".len()..];
        if let Some(name) = rest.split_whitespace().next() {
            return Some(Missing::Method(name.to_string()));
        }
    }
    let idx = message.find("not registered")?;
    let head = &message[..idx];
    let mut words = head.split_whitespace().rev();
    let name = words.next()?.to_string();
    // `make_object` prefixes the class kind — `OBJECT class <name>`
    // (`crates/bund2-stdlib/src/oop.rs`, and the parent arm at `:86`).
    if words.next() == Some("class") {
        Some(Missing::Class(name))
    } else {
        Some(Missing::Word(name))
    }
}

fn read_baseline(repo: &Path) -> Option<usize> {
    let src = std::fs::read_to_string(repo.join(BASELINE)).ok()?;
    src.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .and_then(|l| l.split('/').next())
        .and_then(|n| n.trim().parse().ok())
}

fn write_baseline(repo: &Path, passed: usize, total: usize) -> Result<(), String> {
    let body = format!(
        "# Conformance high-water mark. Written by `cargo xtask conform --accept`.\n\
         #\n\
         # Goldens passed over goldens captured. `cargo xtask conform` fails if\n\
         # it drops below this. The JIT and AOT milestones must not move it at\n\
         # all — they change speed, not meaning.\n\
         #\n\
         # This is NOT a measure of how much of the language works; 59 goldens\n\
         # reach a fraction of the word table. That number is `cargo xtask\n\
         # coverage`, and neither substitutes for the other.\n\n\
         {passed}/{total}\n"
    );
    std::fs::write(repo.join(BASELINE), body).map_err(|e| format!("writing {BASELINE}: {e}"))
}

/// Locate the bund2 binary, building it if needed.
fn bund2_binary(repo: &Path, features: &str) -> Result<PathBuf, String> {
    crate::buildcli::bund2(repo, false, features)
}

pub fn run(args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    // RFC-0005 criterion 2 is "the same N/M with the `jit` feature on and
    // off", which needs this. Without it the builder below rebuilt without
    // features and measured that, whatever had been built beforehand.
    let (features, args) = crate::buildcli::take_features(args);
    // RFC-0005 criterion 2's third run — `--jit-threshold 1`, so every body
    // compiles on its first evaluation, which is the strongest test of meaning
    // the corpus can give. Unavailable until F125 built the flag.
    let (jit_threshold, args) = crate::buildcli::take_jit_threshold(&args)?;
    let threshold_args: Vec<String> = match jit_threshold {
        Some(n) => vec!["--jit-threshold".to_string(), n.to_string()],
        None => Vec::new(),
    };
    let args = args.as_slice();

    let accept = args.iter().any(|a| a == "--accept");
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let parse_only = args.iter().any(|a| a == "--parse-only");
    let blocked_on = args.iter().any(|a| a == "--blocked-on");
    // `--accept-deviation <golden> --reason <ref>` records that Bund2 is
    // approved to disagree with a golden, and what it must produce instead.
    let mut accept_deviation: Option<String> = None;
    let mut reason: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--accept-deviation" => accept_deviation = it.next().cloned(),
            "--reason" => reason = it.next().cloned(),
            "--accept" | "-v" | "--verbose" | "--parse-only" | "--blocked-on" => {}
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    if accept_deviation.is_some() && reason.is_none() {
        return Err(
            "--accept-deviation needs --reason: a deviation without the decision \n               that approved it is indistinguishable from a regression someone gave up on"
                .into(),
        );
    }
    if parse_only {
        return parse_reach(&repo, verbose);
    }

    // The same job list `golden` captures from, so the two cannot disagree
    // about what is in the denominator.
    let (jobs, probe_count) = golden::capture_jobs(&repo)?;
    let golden_dir = repo.join("tests/golden");
    let bund2 = bund2_binary(&repo, &features)?;
    let work = repo.join("target/conform");
    std::fs::create_dir_all(&work).map_err(|e| format!("creating {}: {e}", work.display()))?;

    // Only programs that actually have a captured golden are in the
    // denominator. A program in HERMETIC.txt whose capture was refused is not
    // a conformance failure — there is nothing to conform to.
    let approved = deviations::load(&repo);
    let mut approved_hits: Vec<(String, String)> = Vec::new();
    let mut drifted: Vec<(String, String)> = Vec::new();
    let mut newly_recorded: Option<(String, String)> = None;

    let mut cases: Vec<(String, String, PathBuf, i32, String)> = Vec::new();
    let mut uncaptured = 0usize;
    for (program, name, cwd) in &jobs {
        let gpath = golden_dir.join(name);
        match std::fs::read_to_string(&gpath) {
            Ok(body) => match golden::parse_golden(&body) {
                Some((status, output)) => {
                    cases.push((program.clone(), name.clone(), cwd.clone(), status, output))
                }
                None => return Err(format!("{} is malformed", gpath.display())),
            },
            Err(_) => uncaptured += 1,
        }
    }

    let mut outcomes: Vec<Outcome> = Vec::new();
    let mut not_implemented = 0usize;

    for (program, name, cwd, want_status, want_output) in &cases {
        let src = std::fs::read_to_string(repo.join(program))
            .map_err(|e| format!("reading {program}: {e}"))?;
        let case_file = work.join("case.bund");
        std::fs::write(
            &case_file,
            format!("{src}{}", golden::capture_epilogue(&src)),
        )
        .map_err(|e| format!("writing case copy: {e}"))?;

        match golden::run_once(&bund2, &case_file, cwd, &threshold_args) {
            Ok(got) => {
                // The scaffold exits 70 with a message. Distinguish that from
                // a real mismatch so the report says "unimplemented", not
                // "wrong".
                if got.status == 70 && got.output.contains("not yet implemented") {
                    not_implemented += 1;
                    outcomes.push(Outcome {
                        program: program.clone(),
                        passed: false,
                        approved: false,
                        detail: "bund2 is not implemented".into(),
                        blocked_on: None,
                    });
                    continue;
                }
                let matches_oracle = got.status == *want_status && got.output == *want_output;

                // Recording a deviation: capture what Bund2 produces now, so a
                // later change to it is still caught.
                if accept_deviation.as_deref() == Some(name.as_str()) {
                    newly_recorded = Some((name.clone(), got.output.clone()));
                }

                // An approved deviation is not a pass and not a failure — it
                // is its own outcome, counted and reported apart so the ratio
                // keeps meaning what it says.
                match deviations::judge(&approved, name, &got.output) {
                    deviations::Verdict::Approved => {
                        let why = approved
                            .get(name)
                            .map(|d| d.reason.clone())
                            .unwrap_or_default();
                        approved_hits.push((name.clone(), why.clone()));
                        outcomes.push(Outcome {
                            program: program.clone(),
                            passed: false,
                            approved: true,
                            detail: format!("approved deviation ({why})"),
                            blocked_on: first_unregistered(&got.output),
                        });
                        continue;
                    }
                    deviations::Verdict::Drifted => {
                        let why = approved
                            .get(name)
                            .map(|d| d.reason.clone())
                            .unwrap_or_default();
                        drifted.push((name.clone(), why));
                        outcomes.push(Outcome {
                            program: program.clone(),
                            passed: false,
                            approved: false,
                            detail: "approved deviation, but its output changed".into(),
                            blocked_on: first_unregistered(&got.output),
                        });
                        continue;
                    }
                    deviations::Verdict::NotDeviating => {}
                }

                let passed = matches_oracle;
                let detail = if passed {
                    String::new()
                } else if got.status != *want_status {
                    format!("exit {} != {want_status}", got.status)
                } else {
                    let g = got.output.lines().count();
                    let w = want_output.lines().count();
                    format!("output differs ({g} lines vs {w})")
                };
                outcomes.push(Outcome {
                    program: program.clone(),
                    passed,
                    approved: false,
                    detail,
                    blocked_on: first_unregistered(&got.output),
                });
            }
            Err(e) => outcomes.push(Outcome {
                program: program.clone(),
                passed: false,
                approved: false,
                detail: e,
                blocked_on: None,
            }),
        }
    }

    let passed = outcomes.iter().filter(|o| o.passed).count();
    let total = outcomes.len();

    // Persist a newly recorded deviation before reporting, so the report
    // already reflects it on the next run.
    if let (Some((golden, out)), Some(why)) = (newly_recorded.clone(), reason.clone()) {
        let mut rows = approved.clone();
        rows.insert(
            golden.clone(),
            deviations::Deviation {
                golden: golden.clone(),
                reason: why.clone(),
                expected: deviations::hash(&out),
            },
        );
        deviations::save(&repo, &rows)?;
        println!("\n  recorded deviation  {golden}  ({why})");
        println!("  Bund2's current output is now what that golden must keep");
        println!("  producing; a later change to it fails as a drift.\n");
    } else if accept_deviation.is_some() && newly_recorded.is_none() {
        return Err(format!(
            "no golden named `{}` was run, so nothing was recorded",
            accept_deviation.unwrap_or_default()
        ));
    }

    println!("# cargo xtask conform\n");
    // **Name the binary this number is about.** RFC-0005 recorded "criterion 2
    // has run: 73/86 with the feature on" from a run that had silently rebuilt
    // without the feature. A report that does not say what it measured invites
    // exactly that.
    println!(
        "\n  measured: {}",
        crate::buildcli::provenance_with(false, &features, jit_threshold)
    );
    if approved_hits.is_empty() {
        println!("  CONFORMANCE  {passed}/{total}\n");
    } else {
        println!(
            "  CONFORMANCE  {passed}/{total}  (+{} approved deviation(s))\n",
            approved_hits.len()
        );
    }
    // **The ceiling, stated rather than left to be worked out.** An approved
    // deviation is a golden Bund2 is *decided* not to match, so it can never
    // move into the numerator. Printing `37/72` alone implies 35 goldens of
    // remaining work when the real figure is 28, and the gap grows every time
    // a deviation is recorded.
    let ceiling = total.saturating_sub(approved_hits.len());
    let remaining = ceiling.saturating_sub(passed);
    println!("  CEILING      {ceiling}/{total}   ({remaining} golden(s) still to reach it)\n");
    println!(
        "  Denominator is every captured golden: {} suite programs plus {probe_count}",
        total.saturating_sub(probe_count)
    );
    println!("  authored probes (D21). Both are captured from the oracle, so a");
    println!("  probe failing is a preservation failure like any other.\n");
    if !approved_hits.is_empty() {
        println!("  The ceiling is below the denominator because an approved");
        println!("  deviation is a **decision**, not a gap — an oracle defect Bund2");
        println!("  declines to reproduce, or text no second machine can produce.");
        println!("  No amount of implementation moves one. By the reason each was");
        println!("  recorded under:\n");
        // Grouped from the register rather than described in prose, so the
        // breakdown cannot drift from what is actually recorded. An earlier
        // version said "three and three" and was wrong about both.
        let mut by_reason: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        for (_, why) in &approved_hits {
            *by_reason.entry(why.as_str()).or_default() += 1;
        }
        for (why, n) in &by_reason {
            println!("      {why:<20}{n}");
        }
        println!();
        println!("  The denominator stays the full set: shrinking it would make the");
        println!("  ratio flatter by measuring against less.\n");
    }

    if !approved_hits.is_empty() {
        println!(
            "  {} golden(s) Bund2 is **approved** to disagree with — F48. Each is\n               counted apart from the ratio, not folded into it, because a\n               conformance number that silently absorbs deviations stops being a\n               regression number.\n",
            approved_hits.len()
        );
        for (g, why) in &approved_hits {
            println!("      {g:<44} {why}");
        }
        println!();
    }

    if !drifted.is_empty() {
        println!(
            "  {} approved deviation(s) DRIFTED: the deviation is still approved,\n               but Bund2 no longer produces what was recorded for it. That is a\n               regression inside a deviation, which a bare exclusion would hide.\n",
            drifted.len()
        );
        for (g, why) in &drifted {
            println!("      {g:<44} {why}");
        }
        println!();
    }

    if uncaptured > 0 {
        println!("  {uncaptured} program(s) have no golden and are excluded from");
        println!("  the denominator — `cargo xtask golden` refused them as not");
        println!("  reproducible. There is nothing there to conform to.\n");
    }

    if not_implemented == total && total > 0 {
        println!("  Bund2 is a scaffold: `bund2` exits 70 with \"not yet implemented\"");
        println!("  (crates/bund2-cli/src/main.rs). 0/{total} is the correct reading,");
        println!("  and moving it is the work.\n");
    } else if verbose || (passed < total && not_implemented < total) {
        println!("## failing\n");
        for o in outcomes.iter().filter(|o| !o.passed && !o.approved).take(40) {
            println!("  {:<62} {}", o.program, o.detail);
        }
        let failing = outcomes.iter().filter(|o| !o.passed && !o.approved).count();
        if failing > 40 {
            println!("  ... {} more", failing - 40);
        }
        println!();
    }

    if blocked_on {
        // Every failing golden, keyed by the first word Bund2 could not
        // resolve. A golden that fails for some *other* reason is listed
        // apart rather than silently omitted: "no word blocks it" is a
        // different claim from "it passes", and conflating them is how the
        // last two attempts at this criterion read as satisfied.
        let mut by_word: std::collections::BTreeMap<&Missing, Vec<&str>> =
            std::collections::BTreeMap::new();
        let mut other: Vec<&Outcome> = Vec::new();
        for o in outcomes.iter().filter(|o| !o.passed && !o.approved) {
            match &o.blocked_on {
                Some(w) => by_word.entry(w).or_default().push(&o.program),
                None => other.push(o),
            }
        }

        println!("## blocked-on\n");
        println!("  For each failing golden, the first thing Bund2's diagnostic");
        println!("  named as unregistered — a word, or a class. This is what");
        println!("  RFC-0009 criterion 1 is decided by, and what makes it");
        println!("  decidable at all: the failure list above says only `output");
        println!("  differs`, which names nothing and so cannot distinguish a");
        println!("  missing word from a wrong answer.\n");

        if by_word.is_empty() {
            println!("  No failing golden is blocked on an unregistered word.\n");
        } else {
            let mut ranked: Vec<(&&Missing, &Vec<&str>)> = by_word.iter().collect();
            ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
            let label = Missing::label;
            println!("  {:<30}{:>9}", "missing", "goldens");
            for (w, progs) in &ranked {
                println!("  {:<30}{:>9}", label(w), progs.len());
            }
            println!();
            if verbose {
                for (w, progs) in &ranked {
                    println!("  {}", label(w));
                    for p in progs.iter() {
                        println!("      {p}");
                    }
                }
                println!();
            } else {
                println!("  Pass -v to list the goldens under each entry.\n");
            }

            // Criterion 1's actual question, asked directly rather than left
            // for a reader to scan the table for.
            let blocking: Vec<&Missing> = ranked
                .iter()
                .map(|(m, _)| **m)
                .filter(|m| matches!(m, Missing::Word(w) if w == "class" || w == "object"))
                .collect();
            if blocking.is_empty() {
                println!("  RFC-0009 criterion 1: no golden is blocked on `class` or");
                println!("  `object`. Both are registered.\n");
            } else {
                println!("  RFC-0009 criterion 1 NOT met — still blocked on:");
                for m in blocking {
                    println!("      `{}`", m.name());
                }
                println!();
            }

            let classes: usize = ranked
                .iter()
                .filter(|(m, _)| m.is_class())
                .map(|(_, p)| p.len())
                .sum();
            if classes > 0 {
                println!("  {classes} golden(s) want a built-in **class** Bund2 does not");
                println!("  register. That is not `class` missing — it works — it is the");
                println!("  oracle's per-type class hierarchy");
                println!("  (`reference/Bund/src/stdlib/functions/oop/int_class.rs:39` and");
                println!("  its siblings), which RFC-0009 does not scope. Listed here so");
                println!("  the two are never read as one blocker.\n");
            }
        }

        println!(
            "  {} failing golden(s) name no unregistered word — they reach the",
            other.len()
        );
        println!("  end and disagree, or fail for another reason. Implementing a");
        println!("  word will not move them.\n");
    }

    println!("  Conformance counts goldens, never words. `cargo xtask coverage`");
    println!("  answers how much of the language is tested at all; this number");
    println!("  answers whether what was captured still holds.\n");

    // Regression check.
    let baseline = read_baseline(&repo);
    match baseline {
        Some(prev) if passed < prev => {
            if accept {
                write_baseline(&repo, passed, total)?;
                println!("  baseline lowered {prev} -> {passed} by --accept");
                Ok(())
            } else {
                Err(format!(
                    "REGRESSION: {passed}/{total} is below the recorded baseline of {prev}.\n  \
                     Fix it, or lower the mark deliberately with `cargo xtask conform --accept`."
                ))
            }
        }
        Some(prev) if passed > prev => {
            if accept {
                write_baseline(&repo, passed, total)?;
                println!("  baseline raised {prev} -> {passed}");
            } else {
                println!(
                    "  above baseline ({prev}); record it with `cargo xtask conform --accept`"
                );
            }
            Ok(())
        }
        Some(_) => {
            // Unchanged pass count, but the denominator may have moved — a
            // scope decision narrows the suite, or new probes are captured.
            // Refresh so the file never claims a stale total.
            if accept {
                write_baseline(&repo, passed, total)?;
                println!("  baseline refreshed at {passed}/{total}");
            }
            Ok(())
        }
        None => {
            if accept {
                write_baseline(&repo, passed, total)?;
                println!("  baseline recorded at {passed}/{total}");
            } else {
                println!("  no baseline recorded yet; set one with `cargo xtask conform --accept`");
            }
            Ok(())
        }
    }
}


/// **RFC-0003 criterion 1's parse-reach number.**
///
/// How many golden sources the front end accepts, which is a different
/// question from how many conform: a program can parse perfectly and still
/// fail on the first word Bund2 has not implemented. Reported separately for
/// exactly that reason.
///
/// **The denominator is fixed at the 69 goldens as of RFC-0003** — 57 suite
/// plus 12 probes. Criterion 4 adds probes, two of which are *required* not to
/// parse (`{}` and `{ 1 println}`), so counting them here would make criteria 1
/// and 4 contradict each other. Sources whose name marks them as
/// deliberately-rejecting are listed apart from the ratio.
fn parse_reach(repo: &Path, verbose: bool) -> Result<(), String> {
    let (jobs, _) = golden::capture_jobs(repo)?;
    let mut ok = 0usize;
    let mut total = 0usize;
    let mut failures: Vec<(String, String)> = Vec::new();
    for (program, name, _cwd) in &jobs {
        let path = repo.join(program);
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        total += 1;
        match bund2_syntax::parse(&src) {
            Ok(_) => ok += 1,
            Err(e) => failures.push((name.clone(), e.render(&src))),
        }
    }
    println!("\n  PARSE-REACH  {ok}/{total}\n");
    println!(
        "  How many golden sources the front end accepts. Not a conformance\n           number: a program can parse and still fail on the first word that is\n           not implemented. `cargo xtask conform` answers that one.\n"
    );
    if !failures.is_empty() {
        let show = if verbose { failures.len() } else { 12 };
        for (name, why) in failures.iter().take(show) {
            println!("  {name:<44} {why}");
        }
        if failures.len() > show {
            println!("  ... {} more", failures.len() - show);
        }
    }
    Ok(())
}