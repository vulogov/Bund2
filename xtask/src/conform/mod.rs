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

/// `bund2 build --file <prepared> --output <artefact>` — RFC-0006 criterion 2.
///
/// **Through the real builder**, not by writing the container from here. A
/// second implementation of the format in `xtask` would be a second thing to
/// keep right, and it would stop this criterion from testing the builder at
/// all — which is half of what it is for.
fn build_bundle(bund2: &Path, program: &Path, artefact: &Path) -> Result<(), String> {
    let out = std::process::Command::new(bund2)
        .arg("build")
        .arg("--file")
        .arg(program)
        .arg("--output")
        .arg(artefact)
        .output()
        .map_err(|e| format!("spawning `bund2 build`: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    Err(format!(
        "`bund2 build` refused this program: {}",
        String::from_utf8_lossy(&out.stderr).trim()
    ))
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
    let mut threshold_args: Vec<String> = match jit_threshold {
        Some(n) => vec!["--jit-threshold".to_string(), n.to_string()],
        None => Vec::new(),
    };
    // **RFC-0005 criterion 2's total — D141.** The criterion has always said
    // "`conform` prints the total beside its `measured:` line. A `jit` run
    // that compiles no body over the corpus fails", and until the
    // twenty-third review this file asked the binary for nothing and failed
    // on nothing. A `jit` run now asks every program for its `--stats` line,
    // takes it back out of the output before the comparison, and sums it.
    let with_tier = features.split(',').any(|f| f.trim() == "jit");
    if with_tier {
        threshold_args.push("--stats".to_string());
    }
    let mut tier = TierTotal::default();
    let args = args.as_slice();

    let accept = args.iter().any(|a| a == "--accept");
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let parse_only = args.iter().any(|a| a == "--parse-only");
    let blocked_on = args.iter().any(|a| a == "--blocked-on");
    // **RFC-0006 criterion 2.** Runs every case as a *bundle* instead of as
    // source: `bund2 build` writes an artefact from the same prepared program,
    // and the artefact is executed with no arguments. Everything downstream —
    // normalisation, the per-golden comparison, the deviations, the CEILING —
    // is the code the source run uses, so a difference in the numbers is a
    // difference in meaning and not in how it was measured.
    let bundles = args.iter().any(|a| a == "--bundles");
    // `--accept-deviation <golden> --reason <ref>` records that Bund2 is
    // approved to disagree with a golden, and what it must produce instead.
    let mut accept_deviation: Option<String> = None;
    let mut reason: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--accept-deviation" => accept_deviation = it.next().cloned(),
            "--reason" => reason = it.next().cloned(),
            "--accept" | "-v" | "--verbose" | "--parse-only" | "--blocked-on"
            | "--bundles" => {}
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

        // **The one difference between the two modes.** A bundle carries the
        // program, so it is executed with no arguments and the threshold
        // reaches it through the environment (§B3); a source run passes both
        // on the command line.
        let executed = if bundles {
            let artefact = work.join("case.bundle");
            let _ = std::fs::remove_file(&artefact);
            match build_bundle(&bund2, &case_file, &artefact) {
                Ok(()) => {
                    let mut env: Vec<(String, String)> = Vec::new();
                    if let Some(n) = jit_threshold {
                        env.push(("BUND2_JIT_THRESHOLD".into(), n.to_string()));
                    }
                    if with_tier {
                        env.push(("BUND2_STATS".into(), "1".into()));
                    }
                    golden::run_artifact_once(&artefact, cwd, &env)
                }
                Err(e) => Err(e),
            }
        } else {
            golden::run_once(&bund2, &case_file, cwd, &threshold_args)
        };

        match executed {
            Ok(mut got) => {
                if with_tier {
                    tier.take(&mut got.output);
                }
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
        bund2_api::sayln!("\n  recorded deviation  {golden}  ({why})");
        bund2_api::sayln!("  Bund2's current output is now what that golden must keep");
        bund2_api::sayln!("  producing; a later change to it fails as a drift.\n");
    } else if accept_deviation.is_some() && newly_recorded.is_none() {
        return Err(format!(
            "no golden named `{}` was run, so nothing was recorded",
            accept_deviation.unwrap_or_default()
        ));
    }

    bund2_api::sayln!("# cargo xtask conform\n");
    // **Name the binary this number is about.** RFC-0005 recorded "criterion 2
    // has run: 73/86 with the feature on" from a run that had silently rebuilt
    // without the feature. A report that does not say what it measured invites
    // exactly that.
    bund2_api::sayln!(
        "\n  measured: {}",
        crate::buildcli::provenance_with(false, &features, jit_threshold)
    );
    // **Say which artefact answered — RFC-0006 criterion 2.** A bundle run and
    // a source run print the same shape of report, and a reader comparing two
    // numbers has no other way to tell which is which. F124's lesson: a report
    // that does not say what it measured invites the wrong conclusion.
    if bundles {
        bund2_api::sayln!("  as: `bund2 build` artefacts, executed with no arguments");
    }
    if with_tier {
        bund2_api::sayln!(
            "  tier: compiled {} bodies in {} of {} programs, {} compiled entries",
            tier.bodies, tier.programs, tier.reported, tier.entered
        );
        if tier.bodies == 0 && jit_threshold != Some(1) {
            // D141: not a failure at this threshold, and not evidence either.
            bund2_api::sayln!("        no body compiled at this threshold, so this run is Tier 0");
            bund2_api::sayln!("        against Tier 0; `--jit-threshold 1` is the run that tests the tier");
        }
    }
    if approved_hits.is_empty() {
        bund2_api::sayln!("  CONFORMANCE  {passed}/{total}\n");
    } else {
        bund2_api::sayln!(
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
    bund2_api::sayln!("  CEILING      {ceiling}/{total}   ({remaining} golden(s) still to reach it)\n");
    bund2_api::sayln!(
        "  Denominator is every captured golden: {} suite programs plus {probe_count}",
        total.saturating_sub(probe_count)
    );
    bund2_api::sayln!("  authored probes (D21). Both are captured from the oracle, so a");
    bund2_api::sayln!("  probe failing is a preservation failure like any other.\n");
    if !approved_hits.is_empty() {
        bund2_api::sayln!("  The ceiling is below the denominator because an approved");
        bund2_api::sayln!("  deviation is a **decision**, not a gap — an oracle defect Bund2");
        bund2_api::sayln!("  declines to reproduce, or text no second machine can produce.");
        bund2_api::sayln!("  No amount of implementation moves one. By the reason each was");
        bund2_api::sayln!("  recorded under:\n");
        // Grouped from the register rather than described in prose, so the
        // breakdown cannot drift from what is actually recorded. An earlier
        // version said "three and three" and was wrong about both.
        let mut by_reason: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        for (_, why) in &approved_hits {
            *by_reason.entry(why.as_str()).or_default() += 1;
        }
        for (why, n) in &by_reason {
            bund2_api::sayln!("      {why:<20}{n}");
        }
        bund2_api::sayln!();
        bund2_api::sayln!("  The denominator stays the full set: shrinking it would make the");
        bund2_api::sayln!("  ratio flatter by measuring against less.\n");
    }

    if !approved_hits.is_empty() {
        bund2_api::sayln!(
            "  {} golden(s) Bund2 is **approved** to disagree with — F48. Each is\n               counted apart from the ratio, not folded into it, because a\n               conformance number that silently absorbs deviations stops being a\n               regression number.\n",
            approved_hits.len()
        );
        for (g, why) in &approved_hits {
            bund2_api::sayln!("      {g:<44} {why}");
        }
        bund2_api::sayln!();
    }

    if !drifted.is_empty() {
        bund2_api::sayln!(
            "  {} approved deviation(s) DRIFTED: the deviation is still approved,\n               but Bund2 no longer produces what was recorded for it. That is a\n               regression inside a deviation, which a bare exclusion would hide.\n",
            drifted.len()
        );
        for (g, why) in &drifted {
            bund2_api::sayln!("      {g:<44} {why}");
        }
        bund2_api::sayln!();
    }

    if uncaptured > 0 {
        bund2_api::sayln!("  {uncaptured} program(s) have no golden and are excluded from");
        bund2_api::sayln!("  the denominator — `cargo xtask golden` refused them as not");
        bund2_api::sayln!("  reproducible. There is nothing there to conform to.\n");
    }

    if not_implemented == total && total > 0 {
        bund2_api::sayln!("  Bund2 is a scaffold: `bund2` exits 70 with \"not yet implemented\"");
        bund2_api::sayln!("  (crates/bund2-cli/src/main.rs). 0/{total} is the correct reading,");
        bund2_api::sayln!("  and moving it is the work.\n");
    } else if verbose || (passed < total && not_implemented < total) {
        bund2_api::sayln!("## failing\n");
        for o in outcomes.iter().filter(|o| !o.passed && !o.approved).take(40) {
            bund2_api::sayln!("  {:<62} {}", o.program, o.detail);
        }
        let failing = outcomes.iter().filter(|o| !o.passed && !o.approved).count();
        if failing > 40 {
            bund2_api::sayln!("  ... {} more", failing - 40);
        }
        bund2_api::sayln!();
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

        bund2_api::sayln!("## blocked-on\n");
        bund2_api::sayln!("  For each failing golden, the first thing Bund2's diagnostic");
        bund2_api::sayln!("  named as unregistered — a word, or a class. This is what");
        bund2_api::sayln!("  RFC-0009 criterion 1 is decided by, and what makes it");
        bund2_api::sayln!("  decidable at all: the failure list above says only `output");
        bund2_api::sayln!("  differs`, which names nothing and so cannot distinguish a");
        bund2_api::sayln!("  missing word from a wrong answer.\n");

        if by_word.is_empty() {
            bund2_api::sayln!("  No failing golden is blocked on an unregistered word.\n");
        } else {
            let mut ranked: Vec<(&&Missing, &Vec<&str>)> = by_word.iter().collect();
            ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
            let label = Missing::label;
            bund2_api::sayln!("  {:<30}{:>9}", "missing", "goldens");
            for (w, progs) in &ranked {
                bund2_api::sayln!("  {:<30}{:>9}", label(w), progs.len());
            }
            bund2_api::sayln!();
            if verbose {
                for (w, progs) in &ranked {
                    bund2_api::sayln!("  {}", label(w));
                    for p in progs.iter() {
                        bund2_api::sayln!("      {p}");
                    }
                }
                bund2_api::sayln!();
            } else {
                bund2_api::sayln!("  Pass -v to list the goldens under each entry.\n");
            }

            // Criterion 1's actual question, asked directly rather than left
            // for a reader to scan the table for.
            let blocking: Vec<&Missing> = ranked
                .iter()
                .map(|(m, _)| **m)
                .filter(|m| matches!(m, Missing::Word(w) if w == "class" || w == "object"))
                .collect();
            if blocking.is_empty() {
                bund2_api::sayln!("  RFC-0009 criterion 1: no golden is blocked on `class` or");
                bund2_api::sayln!("  `object`. Both are registered.\n");
            } else {
                bund2_api::sayln!("  RFC-0009 criterion 1 NOT met — still blocked on:");
                for m in blocking {
                    bund2_api::sayln!("      `{}`", m.name());
                }
                bund2_api::sayln!();
            }

            let classes: usize = ranked
                .iter()
                .filter(|(m, _)| m.is_class())
                .map(|(_, p)| p.len())
                .sum();
            if classes > 0 {
                bund2_api::sayln!("  {classes} golden(s) want a built-in **class** Bund2 does not");
                bund2_api::sayln!("  register. That is not `class` missing — it works — it is the");
                bund2_api::sayln!("  oracle's per-type class hierarchy");
                bund2_api::sayln!("  (`reference/Bund/src/stdlib/functions/oop/int_class.rs:39` and");
                bund2_api::sayln!("  its siblings), which RFC-0009 does not scope. Listed here so");
                bund2_api::sayln!("  the two are never read as one blocker.\n");
            }
        }

        bund2_api::sayln!(
            "  {} failing golden(s) name no unregistered word — they reach the",
            other.len()
        );
        bund2_api::sayln!("  end and disagree, or fail for another reason. Implementing a");
        bund2_api::sayln!("  word will not move them.\n");
    }

    bund2_api::sayln!("  Conformance counts goldens, never words. `cargo xtask coverage`");
    bund2_api::sayln!("  answers how much of the language is tested at all; this number");
    bund2_api::sayln!("  answers whether what was captured still holds.\n");

    // **A threshold-1 run that compiled nothing fails — D141.** Scoped to
    // that run by the owner's ruling: at the shipped threshold the corpus
    // compiles nothing by design (F139), so the rule as criterion 2 first
    // wrote it would fail a run that is behaving as intended. `reported == 0`
    // fails too: a `jit` run in which no program printed its line has shown
    // nothing about the tier, and the usual cause is a binary built without
    // the feature.
    let tier_failure = if with_tier && jit_threshold == Some(1) && tier.bodies == 0 {
        Some(format!(
            "NO TIER: a `jit` run at threshold 1 compiled no body over {} programs \
             ({} printed a `--stats` line). RFC-0005 criterion 2 counts that as a failure: \
             the run compared Tier 0 with itself.",
            total, tier.reported
        ))
    } else if with_tier && tier.reported == 0 {
        Some(format!(
            "NO TIER: `--features {features}` was asked for and none of {total} programs \
             printed a `--stats` line, so nothing here says a tier was installed."
        ))
    } else {
        None
    };

    // Regression check.
    let baseline = read_baseline(&repo);
    let verdict = match baseline {
        Some(prev) if passed < prev => {
            if accept {
                write_baseline(&repo, passed, total)?;
                bund2_api::sayln!("  baseline lowered {prev} -> {passed} by --accept");
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
                bund2_api::sayln!("  baseline raised {prev} -> {passed}");
            } else {
                bund2_api::sayln!(
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
                bund2_api::sayln!("  baseline refreshed at {passed}/{total}");
            }
            Ok(())
        }
        None => {
            if accept {
                write_baseline(&repo, passed, total)?;
                bund2_api::sayln!("  baseline recorded at {passed}/{total}");
            } else {
                bund2_api::sayln!("  no baseline recorded yet; set one with `cargo xtask conform --accept`");
            }
            Ok(())
        }
    };
    verdict?;
    match tier_failure {
        Some(why) => Err(why),
        None => Ok(()),
    }
}

/// What the tier did over a whole `conform` run — RFC-0005 criterion 2.
#[derive(Default)]
struct TierTotal {
    /// Bodies compiled, summed over every program.
    bodies: usize,
    /// Entries that ran compiled code, summed the same way.
    entered: usize,
    /// Programs that compiled at least one body.
    programs: usize,
    /// Programs that printed a `--stats` line at all.
    reported: usize,
}

impl TierTotal {
    /// Take the `--stats` line out of `output` and add it to the total.
    ///
    /// The line goes to stderr, which the runner appends to stdout before it
    /// normalises, so it arrives here as one line of the text about to be
    /// compared with a golden. It is removed whether or not it parses: a
    /// golden never holds one.
    fn take(&mut self, output: &mut String) {
        const MARK: &str = "bund2: tier compiled ";
        // **By line, rejoined as `normalise` joins them** — with `\n` between
        // and none after the last. Rebuilding with a newline after every line
        // made all 145 goldens differ by one trailing byte.
        let mut kept: Vec<&str> = Vec::new();
        for line in output.lines() {
            // **Found anywhere in the line, not only at its start.** A
            // program whose last output has no newline leaves stdout ending
            // mid-line, and stderr is appended straight after it.
            let Some(at) = line.find(MARK) else {
                kept.push(line);
                continue;
            };
            if at > 0 {
                kept.push(&line[..at]);
            }
            let rest = &line[at + MARK.len()..];
            self.reported += 1;
            let bodies = leading_number(rest).unwrap_or(0);
            let entered = rest
                .split_once('(')
                .and_then(|(_, after)| leading_number(after))
                .unwrap_or(0);
            self.bodies = self.bodies.saturating_add(bodies);
            self.entered = self.entered.saturating_add(entered);
            if bodies > 0 {
                self.programs += 1;
            }
        }
        let kept = kept.join("\n");
        *output = kept;
    }
}

/// The decimal number `s` starts with, if it starts with one.
fn leading_number(s: &str) -> Option<usize> {
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
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
    bund2_api::sayln!("\n  PARSE-REACH  {ok}/{total}\n");
    bund2_api::sayln!(
        "  How many golden sources the front end accepts. Not a conformance\n           number: a program can parse and still fail on the first word that is\n           not implemented. `cargo xtask conform` answers that one.\n"
    );
    if !failures.is_empty() {
        let show = if verbose { failures.len() } else { 12 };
        for (name, why) in failures.iter().take(show) {
            bund2_api::sayln!("  {name:<44} {why}");
        }
        if failures.len() > show {
            bund2_api::sayln!("  ... {} more", failures.len() - show);
        }
    }
    Ok(())
}