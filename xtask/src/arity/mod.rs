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

/// **Operands only the two-axis probe's second pass offers.** A `.` word
/// often wants different kinds on its two axes — `map.` a LIST on the
/// workbench and a LAMBDA on the stack — which one sentinel type for both
/// cannot give it. The LAMBDA leaves what it found, so what a word that runs
/// it left is the word's own effect around a body that did nothing. Not
/// `{ }`: the reference's grammar refuses an empty lambda.
const BENCH_EXTRA: &[(&str, &str)] = &[("bool", "true"), ("lambda", "{ 1 drop }")];

/// **Operands for the words no pairing of kinds satisfies** — a value of a
/// particular shape rather than of a kind: an expression `srch` parses, a
/// value `encode.base64` made, a condition that ends `while.`. Each is
/// `(word, workbench, stack)`, every entry one value, bottom first; the
/// workbench's are put there with `.`. Found by running each against the
/// oracle; [`probe_sample`] checks every axis it was given was needed.
const SAMPLES: &[(&str, &[&str], &[&str])] = &[
    (".", &[], &["1"]),
    ("?.", &[], &["1", "{ true }"]),
    ("?false.", &[], &["false", "{ 1 drop }"]),
    ("while.", &["false"], &["{ 1 drop }"]),
    ("decode.base64.", &["1 encode.base64"], &[]),
    ("bund.eval.", &["\"1 2 +\""], &[]),
    (
        "string.expressionmatch.",
        &["'contains \"4\"'"],
        &["\"42\""],
    ),
    ("#.", &["1 \"Value\" object"], &["{ 1 drop }"]),
    ("pull.", &["[ \"a\" ]"], &["1"]),
    ("pull.workbench.", &["1", "[ \"a\" ]"], &[]),
    ("io.graph.", &["[ 1.0 2.0 ]"], &[]),
    // A pipe exists once something is sent to it; `send` leaves `true`,
    // which the setup drops so it is not counted as the word's.
    ("recv.", &["\"q\" 1 send drop \"q\""], &[]),
    // `{scratch}` is the probe's scratch directory, where [`probe_sample`]
    // writes `sample.bund` first.
    ("bund.eval-file.", &["\"{scratch}/sample.bund\""], &[]),
];

/// Words the mixed pass probes at once. Each probe is a process, and
/// most of a probe's time is the oracle starting.
const BENCH_WORKERS: usize = 8;

/// Stack consumed and produced, workbench consumed and produced, and the
/// sentinel types offered on the workbench and on the stack.
type Bench2 = (usize, usize, usize, usize, &'static str, &'static str);

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
    /// **Both axes, for a word that touches the workbench** — RFC-0004 §S1's
    /// second axis: (stack consumed, stack produced, workbench consumed,
    /// workbench produced) and the sentinel types that worked, the
    /// workbench's then the stack's. Probed with the workbench seeded, which
    /// [`probe_word`] never does.
    probed_bench: Option<Bench2>,
    /// Adding operands did not change what the word left behind, so `probed`'s
    /// `consumed` is a **floor** and not an arity. See [`probe_word`] for what
    /// this does and does not prove.
    depth_insensitive: bool,
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
fn effective_body(src: &str, handler: &str) -> Option<String> {
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

/// A failure, the program's or the parser's. The parser's is not a table:
/// the reference prints `Error occured` and runs nothing, so a probe that
/// did not parse would otherwise read as a word that left nothing.
fn errored(out: &str) -> bool {
    out.contains("returned error") || out.contains("│ Error") || out.contains("Error occured")
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

    // **Per-process, because two concurrent runs otherwise clobber each other
    // silently.** A shared `arity_probe.bund` lets one run's oracle read
    // another run's program, and the result is not an error — it is a
    // plausible wrong number. Observed: `!=` reported `probed 0->3`, which is
    // impossible for a word probed against an empty stack, alongside a probed
    // count that moved by 14 between runs of an otherwise deterministic tool.
    // Per thread too, for the mixed pass's workers.
    let thread = format!("{:?}", std::thread::current().id());
    let thread: String = thread.chars().filter(char::is_ascii_digit).collect();
    let file = scratch.join(format!("arity_probe_{}_{thread}.bund", std::process::id()));
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
) -> (Option<(usize, usize, &'static str)>, bool, String) {
    let mut last_note = String::from("no sentinel type accepted");
    for (tyname, lit) in SENTINELS {
        for k in 0..=MAX_DEPTH {
            let sentinels = vec![*lit; k].join(" ");
            let program = format!("{sentinels}\n{word}\ndebug.display_stack\n");
            let Some(out) = run_probe(oracle, scratch, &program) else {
                return (None, false, format!("timed out at depth {k} ({tyname})"));
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

            // **Is `k` an arity, or just a minimum?** The loop above only
            // proves the word refused every shallower stack. A whole-stack
            // fold refuses them too — `stdlib_math_op_multiple_inline` pulls
            // until the stack yields NODATA
            // (`reference/rust_multistackvm/src/stdlib/math/math_op.rs:38-52`)
            // — and then reports the same `k` as a binary word does. D12 names
            // that family, and before this check the table recorded `**`,
            // `*+`, `*-` and `*/` as consuming 2, which is what a two-deep
            // sentinel stack happens to hold.
            //
            // Two extra operands tell them apart: a fixed-arity word leaves
            // two more behind. Confirmed against the oracle — at depth 4, `+`
            // and `++` leave 3 while `**`, `*+`, `*-` and `*/` leave 1.
            //
            // **What this flag means is "the residual did not move", and that
            // is weaker than "variadic".** A word that switches stacks reads
            // its residual on a different stack, and a word that empties the
            // stack reads 0 either way; both are depth-insensitive without
            // consuming a variable number of operands. `ensure_stack` is the
            // worked example — flagged here, but 2 operands in leaves 2 and 6
            // leaves 6. The flag is therefore a *superset*, and it is reported
            // as one rather than being narrowed by a guess.
            let deeper = k + 2;
            let probe = format!(
                "{}\n{word}\ndebug.display_stack\n",
                vec![*lit; deeper].join(" ")
            );
            let depth_insensitive = match run_probe(oracle, scratch, &probe) {
                Some(out2) if !errored(&out2) => count_values(&out2) == produced,
                // Deeper stack errors or times out: no evidence of folding, so
                // report the fixed reading rather than guess.
                _ => false,
            };
            return (
                Some((consumed, produced, tyname)),
                depth_insensitive,
                String::new(),
            );
        }
    }
    (None, false, last_note)
}

/// Deepest sentinel workbench the two-axis probe builds. D24's `.` contract
/// takes one operand from the workbench, and no guard asks for more than one.
const MAX_BENCH: usize = 2;

/// **The two-axis probe — RFC-0004 §S1.** [`probe_word`] runs a word on a
/// stack alone, so a `.` word, which takes its primary operand from the
/// workbench (D24), refuses every depth it is offered. This seeds the
/// workbench with `<sentinel> .` — `.` is an alias of `return`, which moves
/// the stack's top to the workbench
/// (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:4`) — and
/// reads both stacks back, the workbench after a marker line.
///
/// Pairs are tried smallest total first, and the workbench before the stack
/// within a total, so the first pair the word accepts is its least demand on
/// both. What it left on each is then what it produced there, by the same
/// argument [`probe_word`] makes for one axis: every operand offered at the
/// least accepted pair was taken. F73 is why this is measured per word: the
/// `.` family does not agree with itself about where the answer goes.
fn probe_bench(oracle: &Path, scratch: &Path, word: &str) -> Option<Bench2> {
    for (tyname, lit) in SENTINELS {
        match bench_pairs(oracle, scratch, word, (tyname, lit), (tyname, lit), 0) {
            Tried::Accepted(found) => return Some(found),
            Tried::Refused => continue,
            Tried::TimedOut => return None,
        }
    }
    None
}

/// **The second pass: a different kind on each axis.** Run only for a word
/// [`probe_bench`] could not measure. Every pairing of the sentinels and
/// [`BENCH_EXTRA`] is tried, workbench kind first, except the uniform ones
/// the first pass tried; the workbench is offered at least one value, since
/// a word that needed none was measured there. The first pairing the word
/// accepts is reported with its two kinds, so `docs/arity.md` says what the
/// word was given. A pairing that times out — an empty body run forever by
/// `loop.` — is passed over rather than ending the word's probe.
fn probe_bench_mixed(oracle: &Path, scratch: &Path, word: &str) -> Option<Bench2> {
    let kinds: Vec<&(&'static str, &'static str)> = SENTINELS.iter().chain(BENCH_EXTRA).collect();
    for &&(bt, bl) in &kinds {
        for &&(st, sl) in &kinds {
            if bt == st && SENTINELS.iter().any(|(t, _)| *t == bt) {
                continue;
            }
            if let Tried::Accepted(found) =
                bench_pairs(oracle, scratch, word, (&bt, &bl), (&st, &sl), 1)
            {
                return Some(found);
            }
        }
    }
    None
}

/// **A word run on its [`SAMPLES`] operands.** Accepted only if it ran,
/// and if dropping the bottom value of either axis it was given makes it
/// fail — so every value offered was one it needed, and what it left is
/// what it produced.
fn probe_sample(oracle: &Path, scratch: &Path, word: &str) -> Option<Bench2> {
    const MARK: &str = "--arity-workbench--";
    let &(_, bench, stack) = SAMPLES.iter().find(|(w, _, _)| *w == word)?;
    std::fs::write(scratch.join("sample.bund"), "1 2 +\n").ok()?;
    let dir = scratch.display().to_string();
    let program = |bench: &[&str], stack: &[&str]| {
        let bench: Vec<String> = bench
            .iter()
            .map(|v| format!("{} .", v.replace("{scratch}", &dir)))
            .collect();
        format!(
            "{}\n{}\n{word}\ndebug.display_stack\n\"{MARK}\" println\ndebug.display_workbench\n",
            bench.join(" "),
            stack.join(" ")
        )
    };
    let out = run_probe(oracle, scratch, &program(bench, stack))?;
    if errored(&out) {
        return None;
    }
    let (on_stack, on_bench) = out.split_once(MARK)?;
    let fewer_bench = bench.get(1..).map(|b| program(b, stack));
    let fewer_stack = stack.get(1..).map(|k| program(bench, k));
    for fewer in [fewer_bench, fewer_stack].into_iter().flatten() {
        if !errored(&run_probe(oracle, scratch, &fewer)?) {
            return None;
        }
    }
    Some((
        stack.len(),
        count_values(on_stack),
        bench.len(),
        count_values(on_bench),
        "sample",
        "sample",
    ))
}

enum Tried {
    Accepted(Bench2),
    Refused,
    TimedOut,
}

/// One pairing of kinds at every pair of depths, smallest total first and
/// the workbench before the stack within a total — `min_bench` values on the
/// workbench at least. Refused on the first objection that is not depth.
fn bench_pairs(
    oracle: &Path,
    scratch: &Path,
    word: &str,
    (bt, bl): (&&'static str, &&str),
    (st, sl): (&&'static str, &&str),
    min_bench: usize,
) -> Tried {
    const MARK: &str = "--arity-workbench--";
    let mut pairs: Vec<(usize, usize)> = (min_bench..=MAX_BENCH)
        .flat_map(|b| (0..=MAX_DEPTH).map(move |k| (b, k)))
        .collect();
    pairs.sort_by_key(|&(b, k)| (b + k, std::cmp::Reverse(b)));
    for &(b, k) in &pairs {
        let bench = vec![format!("{bl} ."); b].join(" ");
        let stack = vec![*sl; k].join(" ");
        let program = format!(
            "{bench}\n{stack}\n{word}\ndebug.display_stack\n\"{MARK}\" println\ndebug.display_workbench\n"
        );
        let Some(out) = run_probe(oracle, scratch, &program) else {
            return Tried::TimedOut;
        };
        if errored(&out) {
            if too_shallow(&out) {
                continue;
            }
            return Tried::Refused;
        }
        let Some((on_stack, on_bench)) = out.split_once(MARK) else {
            return Tried::Refused;
        };
        return Tried::Accepted((k, count_values(on_stack), b, count_values(on_bench), bt, st));
    }
    Tried::Refused
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

    bund2_api::sayln!("# cargo xtask arity\n");
    bund2_api::sayln!("A first-cut stack-effect table. Two passes, reported side by side.\n");
    bund2_api::sayln!("  declared  the word's own `current_stack_len() < N` guard — what");
    bund2_api::sayln!("            its author says it needs. Static, no oracle.");
    bund2_api::sayln!("  probed    consumed/produced observed by running the word against");
    bund2_api::sayln!("            the oracle with sentinel operands.\n");
    if !have_oracle {
        if static_only {
            bund2_api::sayln!("  Probing skipped: --static-only.\n");
        } else {
            bund2_api::sayln!("  Probing skipped: no oracle at target/oracle/release/bund.");
            bund2_api::sayln!("  Build it with:");
            bund2_api::sayln!(
                "    cargo build --release --manifest-path reference/Bund/Cargo.toml \\"
            );
            bund2_api::sayln!("                --target-dir target/oracle\n");
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

        // A `.` word, or one whose own guard reads the workbench, is probed
        // on both axes too.
        let probed_bench = if have_oracle && safe && (word.ends_with('.') || dw.is_some()) {
            probe_bench(&oracle, &scratch, word)
        } else {
            None
        };
        let (probed, depth_insensitive, note) = if have_oracle && safe {
            probe_word(&oracle, &scratch, word)
        } else if !safe {
            (
                None,
                false,
                "not probed: effectful, out of scope, or terminates".into(),
            )
        } else {
            (None, false, String::new())
        };

        rows.push(Row {
            word: word.to_string(),
            site: site.cite(),
            declared_stack: ds,
            declared_workbench: dw,
            probed,
            probed_bench,
            depth_insensitive,
            note,
        });
    }

    // The mixed pass, for the `.` words the uniform one could not measure.
    if have_oracle {
        let todo: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                r.probed_bench.is_none()
                    && r.word.ends_with('.')
                    && !r.note.starts_with("not probed")
            })
            .map(|(i, _)| i)
            .collect();
        let next = std::sync::atomic::AtomicUsize::new(0);
        let found: std::sync::Mutex<Vec<(usize, Bench2)>> = std::sync::Mutex::new(Vec::new());
        std::thread::scope(|sc| {
            for _ in 0..BENCH_WORKERS {
                sc.spawn(|| {
                    while let Some(&i) =
                        todo.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                    {
                        let Some(word) = rows.get(i).map(|r| r.word.clone()) else {
                            continue;
                        };
                        let measured = probe_bench_mixed(&oracle, &scratch, &word)
                            .or_else(|| probe_sample(&oracle, &scratch, &word));
                        if let Some(b) = measured
                            && let Ok(mut f) = found.lock()
                        {
                            f.push((i, b));
                        }
                    }
                });
            }
        });
        for (i, b) in found.into_inner().unwrap_or_default() {
            if let Some(r) = rows.get_mut(i) {
                r.probed_bench = Some(b);
            }
        }
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
    let with_bench = rows.iter().filter(|r| r.probed_bench.is_some()).count();

    // Disagreement: declared says it needs N, the probe succeeded shallower.
    let mut disagree: Vec<&Row> = Vec::new();
    for r in rows {
        // Excluded: for these the declared guard is a minimum and the probed
        // count is the same minimum measured a different way, so a
        // "disagreement" would be an artefact of comparing a floor to a floor.
        if let (Some(d), Some((c, _, _))) = (r.declared_stack, r.probed)
            && d != c
            && !r.depth_insensitive
        {
            disagree.push(r);
        }
    }

    bund2_api::sayln!("  {:<34}{:>5}", "registered names", rows.len());
    bund2_api::sayln!(
        "  {:<34}{:>5}",
        "with a declared depth guard",
        with_declared
    );
    bund2_api::sayln!("  {:<34}{:>5}", "probed successfully", with_probe);
    bund2_api::sayln!("  {:<34}{:>5}", "probed on both axes", with_bench);
    bund2_api::sayln!(
        "  {:<34}{:>5}",
        "not probed (unsafe to run)",
        skipped_unsafe
    );
    bund2_api::sayln!(
        "  {:<34}{:>5}",
        "declared and probed disagree",
        disagree.len()
    );
    let insensitive: Vec<&Row> = rows.iter().filter(|r| r.depth_insensitive).collect();
    bund2_api::sayln!(
        "  {:<34}{:>5}",
        "depth-insensitive residual",
        insensitive.len()
    );
    bund2_api::sayln!();

    if !insensitive.is_empty() {
        bund2_api::sayln!("## depth-insensitive residual\n");
        bund2_api::sayln!("  Two extra operands changed nothing about what these left");
        bund2_api::sayln!("  behind, so the probed `consumed` is a **floor** and the");
        bund2_api::sayln!("  table writes it `N+`. Reading it as an arity is what this");
        bund2_api::sayln!("  column exists to stop: a two-deep sentinel stack makes a");
        bund2_api::sayln!("  whole-stack fold look binary, which is what the table said");
        bund2_api::sayln!("  about `**`, `*+`, `*-` and `*/` before this check.\n");
        bund2_api::sayln!("  **It does not prove variadicity.** It is a superset. A word");
        bund2_api::sayln!("  that switches stacks measures its residual on a *different*");
        bund2_api::sayln!("  stack and lands here too — `ensure_stack` does, probed with");
        bund2_api::sayln!("  a string sentinel, and its depth is genuinely unchanged");
        bund2_api::sayln!("  (verified against the oracle: 2 in, 2 out; 6 in, 6 out).");
        bund2_api::sayln!("  Same for words that empty the stack outright. Which cause");
        bund2_api::sayln!("  applies is a source question, not a probe question.\n");
        bund2_api::sayln!("  D12's four are confirmed by source: `stdlib_math_op_multiple_inline`");
        bund2_api::sayln!("  pulls until the stack yields NODATA");
        bund2_api::sayln!("  (`reference/rust_multistackvm/src/stdlib/math/math_op.rs:38-52`),");
        bund2_api::sayln!("  and at depth 4 they leave 1 where `+` and `++` leave 3.\n");
        bund2_api::sayln!("  {:<28}{:>10}{:>10}", "word", "floor", "produced");
        for r in &insensitive {
            let (floor, produced) = r.probed.map(|(c, p, _)| (c, p)).unwrap_or((0, 0));
            bund2_api::sayln!("  {:<28}{:>9}+{:>10}", r.word, floor, produced);
        }
        bund2_api::sayln!();
    }

    if !disagree.is_empty() {
        bund2_api::sayln!("## declared depth differs from probed consumption\n");
        bund2_api::sayln!("  The guard is a minimum, not an arity: a word may guard for 1");
        bund2_api::sayln!("  and consume 2, or guard for 2 and succeed on a shallower");
        bund2_api::sayln!("  stack because the sentinel took a different branch. Each of");
        bund2_api::sayln!("  these needs reading before RFC-0004 trusts either number.\n");
        // `disagree` is built from rows that have both numbers, so these are
        // present; defaulting rather than unwrapping keeps the report from
        // aborting if that filter ever changes.
        for r in disagree.iter().take(40) {
            let (c, p, t) = r.probed.unwrap_or((0, 0, ""));
            let declared = r.declared_stack.unwrap_or(0);
            bund2_api::sayln!(
                "  {:<26} declared {declared}  probed {c}->{p} ({t})   {}",
                r.word,
                r.site
            );
        }
        if disagree.len() > 40 {
            bund2_api::sayln!("  ... {} more", disagree.len() - 40);
        }
        bund2_api::sayln!();
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
    s.push_str("`wb in`/`wb out` and the stack pair beside them, `in`/`out`, are the\n");
    s.push_str("two-axis probe (RFC-0004 §S1): the word run with the workbench seeded\n");
    s.push_str("too, at the least pair of depths it accepts.\n\n");
    s.push_str("| word | declared | wb | consumed | produced | via | in | out | wb in | wb out | site | note |\n");
    s.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for r in rows {
        let (c, p, t) = match r.probed {
            // A floor, not an arity, so it is written `2+` rather than `2`.
            Some((c, p, t)) if r.depth_insensitive => {
                (format!("{c}+"), p.to_string(), t.to_string())
            }
            Some((c, p, t)) => (c.to_string(), p.to_string(), t.to_string()),
            None => (String::new(), String::new(), String::new()),
        };
        let two = |f: fn(&Bench2) -> usize| {
            r.probed_bench
                .as_ref()
                .map(|b| f(b).to_string())
                .unwrap_or_default()
        };
        s.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.word,
            r.declared_stack.map(|n| n.to_string()).unwrap_or_default(),
            r.declared_workbench
                .map(|n| n.to_string())
                .unwrap_or_default(),
            c,
            p,
            t,
            two(|b| b.0),
            two(|b| b.1),
            two(|b| b.2),
            two(|b| b.3),
            r.site,
            match r.probed_bench {
                Some((.., "sample", _)) => {
                    let sep = if r.note.is_empty() { "" } else { "; " };
                    format!("{}{sep}both axes: the operands `SAMPLES` gives it", r.note)
                }
                Some((.., bt, st)) if bt != st => {
                    let sep = if r.note.is_empty() { "" } else { "; " };
                    format!("{}{sep}both axes: workbench {bt}, stack {st}", r.note)
                }
                _ => r.note.clone(),
            },
        ));
    }
    std::fs::write(&path, s).map_err(|e| format!("writing {}: {e}", path.display()))?;
    bund2_api::sayln!(
        "wrote {}",
        path.strip_prefix(repo).unwrap_or(&path).display()
    );
    Ok(())
}
