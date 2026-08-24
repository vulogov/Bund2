//! `cargo xtask arity` — a first-cut stack-effect table.
//!
//! Two independent passes, reported side by side. Where they disagree, the
//! disagreement is the finding.
//!
//! **Static.** Most words open with a guard of the form
//! `if vm.stack.current_stack_len() < N { bail!("Stack is too shallow ..") }`
//! — 218 such guards across 124 files. That N is the *author's declared*
//! minimum depth. It is cheap, needs no oracle, and covers most of the table.
//!
//! **Probed.** For words that are safe to run, generate a `.bund` program,
//! execute it against the oracle, and read the stack back with
//! `debug.display_stack`, which prints one `Value { .. }` line per entry. The
//! smallest sentinel depth at which the word stops failing with "too shallow"
//! is its consumed count; the resulting depth then gives what it produced.
//!
//! Safety comes from the effect classification in `corpus::classify`: only
//! words that are hermetic and in scope are ever executed, which keeps the
//! probe away from the filesystem, the network, stdin and the clock. Two
//! words are additionally refused by name because they end the process.
//!
//! This is deliberately a *first cut*. A word that wants a LAMBDA will reject
//! an integer sentinel with a type error rather than an arity error, and the
//! probe records that as type-constrained rather than guessing. RFC-0004 gets
//! a table with its uncertainty marked, not a table that looks certain.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::corpus::classify::Classifier;
use crate::corpus::registry;

/// Deepest sentinel stack the probe will build. The declared guards top out
/// at 3 (`current_stack_len() < 3`), so 4 is one past the observed maximum.
const MAX_DEPTH: usize = 4;

/// Words that end the process or otherwise cannot be probed, regardless of
/// what the effect classification says. `exit` is deliberately classified
/// hermetic — its status is deterministic and goldens record it — so it has
/// to be refused here by name.
const NEVER_PROBE: &[&str] = &["exit", "bund.exit"];

/// Sentinel operands, tried in order. A word rejecting an integer on type
/// grounds may accept a string.
const SENTINELS: &[(&str, &str)] = &[
    ("int", "1"),
    ("float", "1.0"),
    ("string", "\"s\""),
    ("list", "[ 1 ]"),
];

#[derive(Debug, Clone)]
struct Row {
    word: String,
    site: String,
    /// From the `current_stack_len() < N` guard, if the handler has one.
    declared_stack: Option<usize>,
    /// From the `workbench.len() < N` guard.
    declared_workbench: Option<usize>,
    /// Probed (consumed, produced) and the sentinel type that worked.
    probed: Option<(usize, usize, &'static str)>,
    note: String,
}

/// Narrow a shared base function's body to the arm that serves this word.
///
/// Words come in stack/workbench pairs served by one base, which guards each
/// arm separately — `push` needs 2 on the stack while `push.` needs 1 on each
/// (`reference/Bund/src/stdlib/functions/values/push.rs:11-25`). Reading the
/// whole body and taking the smallest guard reported `push` as declaring 1
/// when it declares 2, which then looked like a probe disagreement. Splitting
/// on the match arms removes that whole class of false finding.
fn arm_for<'a>(body: &'a str, word: &str) -> &'a str {
    let (mine, theirs) = if word.ends_with('.') {
        ("StackOps::FromWorkBench =>", "StackOps::FromStack =>")
    } else {
        ("StackOps::FromStack =>", "StackOps::FromWorkBench =>")
    };
    let Some(start) = body.find(mine) else {
        return body;
    };
    let rest = &body[start..];
    // The arm ends where the other arm begins, if it follows.
    match rest.find(theirs) {
        Some(end) => &rest[..end],
        None => rest,
    }
}

/// Extract the smallest `N` in `<pattern> < N` guards inside a handler body.
fn declared_min(body: &str, pattern: &str) -> Option<usize> {
    let mut best: Option<usize> = None;
    let mut from = 0;
    while let Some(rel) = body[from..].find(pattern) {
        let at = from + rel + pattern.len();
        let rest = body[at..].trim_start();
        let Some(stripped) = rest.strip_prefix('<') else {
            from = at;
            continue;
        };
        let digits: String = stripped
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if let Ok(n) = digits.parse::<usize>() {
            best = Some(best.map_or(n, |b: usize| b.min(n)));
        }
        from = at;
    }
    best
}

/// The body of `fn <name>`, up to the next item at column 0.
fn handler_body<'a>(src: &'a str, handler: &str) -> Option<&'a str> {
    if handler.is_empty() {
        return None;
    }
    let short = handler.rsplit("::").next().unwrap_or(handler);
    let at = src.find(&format!("fn {short}"))?;
    let rest = &src[at..];
    let end = rest[1..]
        .find("\npub fn ")
        .map(|i| i + 1)
        .or_else(|| rest[1..].find("\nfn ").map(|i| i + 1))
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Follow one level of delegation: many handlers are a single call into a
/// `_base` function that holds the real guard.
fn effective_body<'a>(src: &'a str, handler: &str) -> Option<String> {
    let body = handler_body(src, handler)?;
    let mut out = body.to_string();
    for cand in ["_base", "_inline_base"] {
        if let Some(at) = body.find(cand) {
            let start = body[..at]
                .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
                .map_or(0, |i| i + 1);
            let callee = &body[start..at + cand.len()];
            if let Some(b2) = handler_body(src, callee) {
                out.push('\n');
                out.push_str(b2);
            }
        }
    }
    Some(out)
}

/// Stack depth, read from a `debug.display_stack` dump.
///
/// Count *lines*, not occurrences. A composite value prints its members
/// inline, so a three-element list renders as one line containing four
/// `Value {` — counting occurrences reported `push` as producing 4 where it
/// produces 1. `debug.display_stack` puts each stack entry on its own line
/// (`reference/Bund/src/stdlib/functions/debug_fun/debug_display_stack.rs`),
/// so one matching line is one entry.
fn count_values(out: &str) -> usize {
    out.lines().filter(|l| l.contains("Value {")).count()
}

fn errored(out: &str) -> bool {
    out.contains("returned error") || out.contains("│ Error")
}

fn too_shallow(out: &str) -> bool {
    out.contains("too shallow") || out.contains("NO DATA")
}

/// How long a single probe may run before it is killed. A word given the
/// wrong operand type can loop — `loop`, `while` and `*loop` all take a
/// lambda — and one hang would stall the whole table.
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Run one probe program against the oracle and return its combined output,
/// or `None` if it had to be killed.
fn run_probe(oracle: &Path, scratch: &Path, program: &str) -> Option<String> {
    use std::io::Read;

    let file = scratch.join("arity_probe.bund");
    std::fs::write(&file, program).ok()?;
    let mut child = std::process::Command::new(oracle)
        .arg("script")
        .arg("--file")
        .arg(&file)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > PROBE_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }

    let mut s = String::new();
    if let Some(mut o) = child.stdout.take() {
        let _ = o.read_to_string(&mut s);
    }
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut s);
    }
    Some(s)
}

/// Probe one word: find the smallest depth at which it stops complaining
/// about depth, then read what it left behind.
fn probe_word(
    oracle: &Path,
    scratch: &Path,
    word: &str,
) -> (Option<(usize, usize, &'static str)>, String) {
    let mut last_note = String::from("no sentinel type accepted");
    for (tyname, lit) in SENTINELS {
        for k in 0..=MAX_DEPTH {
            let sentinels = vec![*lit; k].join(" ");
            let program = format!("{sentinels}\n{word}\ndebug.display_stack\n");
            let Some(out) = run_probe(oracle, scratch, &program) else {
                return (None, format!("timed out at depth {k} ({tyname})"));
            };
            if errored(&out) {
                if too_shallow(&out) {
                    continue; // needs a deeper stack
                }
                last_note = format!("type-constrained: {tyname} rejected");
                break; // a different objection — try the next sentinel type
            }
            // Every shallower depth was refused as too shallow — the loop
            // only reaches here after `continue`ing through them — so the
            // word needs exactly `k` operands, and consumes them all.
            // Starting depth `k` minus `k` consumed leaves nothing, so
            // whatever remains is precisely what it produced.
            let consumed = k;
            let produced = count_values(&out);
            return (Some((consumed, produced, tyname)), String::new());
        }
    }
    (None, last_note)
}

pub fn run(args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();
    let static_only = args.iter().any(|a| a == "--static-only");

    let reg = registry::scan(
        &repo,
        &[
            "reference/Bund/src",
            "reference/rust_multistackvm/src",
            "reference/rust_multistack/src",
        ],
    );

    let oracle = repo.join("target/oracle/release/bund");
    let have_oracle = !static_only && oracle.is_file();

    let scratch = repo.join("target/arity");
    std::fs::create_dir_all(&scratch)
        .map_err(|e| format!("creating {}: {e}", scratch.display()))?;

    println!("# cargo xtask arity\n");
    println!("A first-cut stack-effect table. Two passes, reported side by side.\n");
    println!("  declared  the word's own `current_stack_len() < N` guard — what");
    println!("            its author says it needs. Static, no oracle.");
    println!("  probed    consumed/produced observed by running the word against");
    println!("            the oracle with sentinel operands.\n");
    if !have_oracle {
        if static_only {
            println!("  Probing skipped: --static-only.\n");
        } else {
            println!("  Probing skipped: no oracle at target/oracle/release/bund.");
            println!("  Build it with:");
            println!("    cargo build --release --manifest-path reference/Bund/Cargo.toml \\");
            println!("                --target-dir target/oracle\n");
        }
    }

    let mut rows: Vec<Row> = Vec::new();
    let mut src_cache: BTreeMap<String, String> = BTreeMap::new();
    let never: BTreeSet<&str> = NEVER_PROBE.iter().copied().collect();
    let mut skipped_unsafe = 0usize;

    for word in reg.word_names() {
        let Some(site) = reg.implementing_site(word) else {
            continue;
        };
        let src = src_cache
            .entry(site.path.clone())
            .or_insert_with(|| std::fs::read_to_string(repo.join(&site.path)).unwrap_or_default());
        let body = effective_body(src, &site.handler);
        let (ds, dw) = match &body {
            Some(b) => {
                let arm = arm_for(b, word);
                (
                    declared_min(arm, "current_stack_len()"),
                    declared_min(arm, "workbench.len()"),
                )
            }
            None => (None, None),
        };

        // Probe only what is safe to execute.
        let effect_ok = Classifier::effect_of(&reg, word).is_some_and(|(e, _)| e.hermetic());
        let scope_ok = Classifier::deferral_of(&reg, word).is_none();
        let safe = effect_ok && scope_ok && !never.contains(word);
        if !safe {
            skipped_unsafe += 1;
        }

        let (probed, note) = if have_oracle && safe {
            probe_word(&oracle, &scratch, word)
        } else if !safe {
            (
                None,
                "not probed: effectful, out of scope, or terminates".into(),
            )
        } else {
            (None, String::new())
        };

        rows.push(Row {
            word: word.to_string(),
            site: site.cite(),
            declared_stack: ds,
            declared_workbench: dw,
            probed,
            note,
        });
    }

    report(&repo, &rows, have_oracle, skipped_unsafe)
}

fn report(
    repo: &Path,
    rows: &[Row],
    probed_ran: bool,
    skipped_unsafe: usize,
) -> Result<(), String> {
    let with_declared = rows.iter().filter(|r| r.declared_stack.is_some()).count();
    let with_probe = rows.iter().filter(|r| r.probed.is_some()).count();

    // Disagreement: declared says it needs N, the probe succeeded shallower.
    let mut disagree: Vec<&Row> = Vec::new();
    for r in rows {
        if let (Some(d), Some((c, _, _))) = (r.declared_stack, r.probed)
            && d != c
        {
            disagree.push(r);
        }
    }

    println!("  {:<34}{:>5}", "registered names", rows.len());
    println!(
        "  {:<34}{:>5}",
        "with a declared depth guard", with_declared
    );
    println!("  {:<34}{:>5}", "probed successfully", with_probe);
    println!(
        "  {:<34}{:>5}",
        "not probed (unsafe to run)", skipped_unsafe
    );
    println!(
        "  {:<34}{:>5}",
        "declared and probed disagree",
        disagree.len()
    );
    println!();

    if !disagree.is_empty() {
        println!("## declared depth differs from probed consumption\n");
        println!("  The guard is a minimum, not an arity: a word may guard for 1");
        println!("  and consume 2, or guard for 2 and succeed on a shallower");
        println!("  stack because the sentinel took a different branch. Each of");
        println!("  these needs reading before RFC-0004 trusts either number.\n");
        for r in disagree.iter().take(40) {
            let (c, p, t) = r.probed.unwrap();
            println!(
                "  {:<26} declared {}  probed {c}->{p} ({t})   {}",
                r.word,
                r.declared_stack.unwrap(),
                r.site
            );
        }
        if disagree.len() > 40 {
            println!("  ... {} more", disagree.len() - 40);
        }
        println!();
    }

    let path = repo.join("docs/arity.md");
    let mut s = String::new();
    s.push_str("# Stack-effect table (generated)\n\n");
    s.push_str("Generated by `cargo xtask arity`. Do not edit — re-run the command.\n\n");
    s.push_str("`declared` is the word's own `current_stack_len() < N` guard: the\n");
    s.push_str("minimum depth its author requires. `wb` is the matching\n");
    s.push_str("`workbench.len() < N` guard. `consumed`/`produced` are observed by\n");
    s.push_str("running the word against the oracle with sentinel operands.\n\n");
    s.push_str("A blank `consumed` means the word was not probed — either it is\n");
    s.push_str("effectful, out of scope, or it rejected every sentinel type on type\n");
    s.push_str("grounds. Those are listed with a reason rather than guessed at.\n\n");
    if !probed_ran {
        s.push_str("**This run was static only — no oracle was available.**\n\n");
    }
    s.push_str("| word | declared | wb | consumed | produced | via | site | note |\n");
    s.push_str("|---|---|---|---|---|---|---|---|\n");
    for r in rows {
        let (c, p, t) = match r.probed {
            Some((c, p, t)) => (c.to_string(), p.to_string(), t.to_string()),
            None => (String::new(), String::new(), String::new()),
        };
        s.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} | {} | {} |\n",
            r.word,
            r.declared_stack.map(|n| n.to_string()).unwrap_or_default(),
            r.declared_workbench
                .map(|n| n.to_string())
                .unwrap_or_default(),
            c,
            p,
            t,
            r.site,
            r.note,
        ));
    }
    std::fs::write(&path, s).map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!(
        "wrote {}",
        path.strip_prefix(repo).unwrap_or(&path).display()
    );
    Ok(())
}
