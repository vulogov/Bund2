//! `cargo xtask corpus` — evidence gathering over the reference example
//! corpus.
//!
//! Reads `.bund` files and Rust registration sites. Builds nothing, runs
//! nothing, and never writes inside `reference/`.
//!
//! It answers four questions, and only those. It does not resolve a decision:
//! everything it emits is input to `docs/registers/decisions.md`, not a
//! substitute for it.

pub mod classify;
pub mod lex;
pub mod registry;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use classify::{Classifier, Effect};
use lex::Kind;
use registry::Registry;

/// Corpus roots, repo-relative. `.bund` only.
const CORPUS_ROOTS: &[&str] = &["reference/Bund/examples", "reference/Bund/tests"];

/// Crates that register names. All three tiers `i_direct` can reach:
/// the VM's own table, and the stack layer it falls through to
/// (`reference/rust_multistackvm/src/multistackvm_inline.rs:42,52`).
const REGISTRY_ROOTS: &[&str] = &[
    "reference/Bund/src",
    "reference/rust_multistackvm/src",
    "reference/rust_multistack/src",
];

/// Is this subsystem stack / math / logic / lambda / oop — the "basic" set
/// the TASK 3 intrusion test measures against? Everything in the stack layer
/// qualifies by construction: it is stack manipulation and nothing else.
fn is_basic(sub: &str) -> bool {
    sub.starts_with("stack/") || BASIC_SUBSYSTEMS.contains(&sub)
}

/// Subsystems that are stack / math / logic / lambda / oop, for the TASK 3
/// "otherwise-basic" test.
const BASIC_SUBSYSTEMS: &[&str] = &[
    "vm/math",
    "vm/logic",
    "vm/values",
    "vm/artefacts",
    "vm/artefacts_json",
    "vm/stackop",
    "vm/stacks",
    "vm/print",
    "vm/lambdas",
    "vm/vars",
    "vm/ctx",
    "vm/classes",
    "vm/execute",
    "vm/bund_object",
    "vm/convert",
    "bund/oop",
    "bund/conditional",
    "bund/values",
    "bund/math",
];

pub struct Program {
    /// Repo-relative path.
    pub path: String,
    /// Short name: path below the corpus root, without `.bund`.
    pub name: String,
    pub lines: Vec<String>,
    pub lexed: lex::Lexed,
}

impl Program {
    /// 1-indexed, trimmed. Used for the "surrounding line" the report quotes.
    pub fn line(&self, n: usize) -> &str {
        self.lines
            .get(n - 1)
            .map(String::as_str)
            .unwrap_or("")
            .trim()
    }

    /// Word invocations, in order.
    pub fn words(&self) -> impl Iterator<Item = &lex::Token> {
        self.lexed.tokens.iter().filter(|t| t.kind == Kind::Name)
    }

    /// Distinct word invocations.
    pub fn word_set(&self) -> BTreeSet<&str> {
        self.words().map(|t| t.text.as_str()).collect()
    }

    /// Atoms in this file. A name used but not registered is very likely one
    /// of these: the program defined it with `register`.
    pub fn atom_set(&self) -> BTreeSet<&str> {
        self.lexed
            .tokens
            .iter()
            .filter(|t| t.kind == Kind::Atom)
            .map(|t| t.text.as_str())
            .collect()
    }
}

fn collect_bund(dir: &Path, out: &mut Vec<PathBuf>, recurse: bool) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    items.sort();
    for p in items {
        if p.is_dir() {
            if recurse {
                collect_bund(&p, out, true);
            }
        } else if p.extension().is_some_and(|e| e == "bund") {
            out.push(p);
        }
    }
}

fn load_corpus(repo: &Path) -> Vec<Program> {
    let mut programs = Vec::new();
    for (idx, root) in CORPUS_ROOTS.iter().enumerate() {
        let abs = repo.join(root);
        let mut files = Vec::new();
        // examples/**/*.bund is recursive; tests/*.bund is not.
        collect_bund(&abs, &mut files, idx == 0);
        for f in files {
            let Ok(src) = std::fs::read_to_string(&f) else {
                continue;
            };
            let rel = f
                .strip_prefix(repo)
                .unwrap_or(&f)
                .to_string_lossy()
                .replace('\\', "/");
            let name = f
                .strip_prefix(&abs)
                .unwrap_or(&f)
                .to_string_lossy()
                .replace('\\', "/")
                .trim_end_matches(".bund")
                .to_string();
            let name = if idx == 0 {
                name
            } else {
                format!("tests/{name}")
            };
            programs.push(Program {
                path: rel,
                name,
                lines: src.lines().map(str::to_string).collect(),
                lexed: lex::lex(&src),
            });
        }
    }
    programs
}

/// Every occurrence of `word` across the corpus, with its citation and the
/// lambda nesting depth it sits at.
fn occurrences<'a>(programs: &'a [Program], word: &str) -> Vec<(&'a Program, usize, usize)> {
    let mut v = Vec::new();
    for p in programs {
        for t in p.words() {
            if t.text == word {
                v.push((p, t.line, t.lambda_depth));
            }
        }
    }
    v
}

/// How many citations to print per word before summarising. Truncation is
/// always announced — a silent cap reads as "that was all of them".
const CITE_CAP: usize = 12;

pub fn run(_args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let programs = load_corpus(&repo);
    if programs.is_empty() {
        return Err(format!(
            "no .bund files under {:?}. Are the reference submodules checked out?",
            CORPUS_ROOTS
        ));
    }
    let reg = registry::scan(&repo, REGISTRY_ROOTS);

    println!("# cargo xtask corpus\n");
    println!(
        "{} programs ({} examples, {} tests), {} lines.",
        programs.len(),
        programs
            .iter()
            .filter(|p| !p.name.starts_with("tests/"))
            .count(),
        programs
            .iter()
            .filter(|p| p.name.starts_with("tests/"))
            .count(),
        programs.iter().map(|p| p.lines.len()).sum::<usize>(),
    );
    println!(
        "Registry: {} inline words, {} aliases, {} methods, {} commands, {} classes.\n",
        reg.inline.len(),
        reg.alias.len(),
        reg.method.len(),
        reg.command.len(),
        reg.class.len(),
    );

    task1(&programs, &reg);
    task2(&programs, &reg);
    task3(&programs, &reg);
    let unstable = load_unstable(&repo);
    let hermetic = task4(&programs, &reg, &unstable);
    write_hermetic(&repo, &hermetic)?;
    effect_audit(&programs, &reg);
    reachability(&programs, &reg);
    workbench_forms(&repo, &reg);
    lexer_anomalies(&programs);

    Ok(())
}

// ---------------------------------------------------------------------------
// Workbench-form audit — D18 / Q11
// ---------------------------------------------------------------------------

/// Does the registered handler thread `StackOps`?
///
/// CAUTION — this is **not** a test for "consumes a primary operand", and an
/// earlier version of this audit wrongly presented it as one.
///
/// `StackOps` is the mechanism by which a `.` form is *implemented*
/// (`reference/Bund/src/stdlib/functions/values/push.rs:10`), so "threads
/// StackOps" and "has a `.` form" are near-tautologically correlated. The
/// audit below therefore cannot validate the Q11 criterion; it mostly
/// re-derives which words already have workbench forms.
///
/// What it *is* good for: finding words whose two forms are written as
/// separate functions rather than one parameterised base, which the naive
/// correlation flags as anomalies.
///
/// Deciding Q11 properly needs real stack-effect data — `cargo xtask arity`.
fn handler_uses_stackops(src: &str, handler: &str) -> Option<bool> {
    if handler.is_empty() {
        return None;
    }
    let short = handler.rsplit("::").next().unwrap_or(handler);
    let needle = format!("fn {short}");
    let at = src.find(&needle)?;
    // Body runs to the next item at column 0.
    let rest = &src[at..];
    let end = rest[1..]
        .find("\npub fn ")
        .map(|i| i + 1)
        .or_else(|| rest[1..].find("\nfn ").map(|i| i + 1))
        .unwrap_or(rest.len());
    Some(rest[..end].contains("StackOps"))
}

/// Cross-tabulate "has a `.` form" against "sources an operand", to test the
/// criterion D18/Q11 rests on and to size the gap it implies.
fn workbench_forms(repo: &Path, reg: &Registry) {
    println!("\n---\n\n# Workbench-form audit  [D18 / Q11]\n");
    println!("D18/Q11 fills a missing `W.` only where `W` sources a primary");
    println!("operand; for a pure producer `W .` already says it (`.` is");
    println!("`return`, stack->workbench, create_aliases.rs:4).\n");
    println!("THIS AUDIT DOES NOT DECIDE THAT. `StackOps` is how a `.` form is");
    println!("implemented (push.rs:10), so \"threads StackOps\" and \"has a `.`");
    println!("form\" are near-tautologically correlated — the high agreement");
    println!("below is circular and proves nothing about operand arity. The");
    println!("bucket labelled `neither` is NOT a list of producers: it holds");
    println!("`set`, `get`, `math.sqrt`, `string.upper` and other plain");
    println!("consumers that simply have no workbench variant.\n");
    println!("Applying Q11 needs real stack-effect data: `cargo xtask arity`.");
    println!("What follows is useful only for the two anomaly lists.\n");

    let all: Vec<&str> = reg.word_names();
    let known: BTreeSet<&str> = all.iter().copied().collect();
    let mut cache: BTreeMap<String, String> = BTreeMap::new();

    let (mut consistent_yes, mut consistent_no) = (0usize, 0usize);
    let mut counter: Vec<(&str, String)> = Vec::new();
    let mut gaps: Vec<&str> = Vec::new();
    let mut producers: Vec<&str> = Vec::new();
    let mut undetermined: Vec<&str> = Vec::new();

    for w in &all {
        // Only base names: a `.`/`,` form is not itself a base.
        if w.ends_with('.') || w.ends_with(',') {
            continue;
        }
        let Some(site) = reg.implementing_site(w) else {
            continue;
        };
        let src = cache
            .entry(site.path.clone())
            .or_insert_with(|| std::fs::read_to_string(repo.join(&site.path)).unwrap_or_default());
        let has_dot = known.contains(format!("{w}.").as_str());
        match handler_uses_stackops(src, &site.handler) {
            None => undetermined.push(w),
            Some(uses) => match (has_dot, uses) {
                (true, true) => consistent_yes += 1,
                (false, false) => {
                    consistent_no += 1;
                    producers.push(w);
                }
                (true, false) => counter.push((w, site.cite())),
                (false, true) => gaps.push(w),
            },
        }
    }

    let decided = consistent_yes + consistent_no + counter.len() + gaps.len();
    println!(
        "  {:<44}{:>5}",
        "has `.` form AND uses StackOps", consistent_yes
    );
    println!(
        "  {:<44}{:>5}",
        "has neither (NOT a producer list)", consistent_no
    );
    println!(
        "  {:<44}{:>5}   <-- mostly naming artefacts",
        "no `.` form BUT uses StackOps",
        gaps.len()
    );
    println!(
        "  {:<44}{:>5}   <-- two forms, separate fns",
        "has `.` form BUT no StackOps",
        counter.len()
    );
    println!(
        "  {:<44}{:>5}",
        "handler not resolvable",
        undetermined.len()
    );
    if decided > 0 {
        println!(
            "\n  agreement {}/{} ({:.1}%) — CIRCULAR, proves nothing (see caveat)\n",
            consistent_yes + consistent_no + gaps.len(),
            decided,
            100.0 * (consistent_yes + consistent_no + gaps.len()) as f64 / decided as f64
        );
    }

    println!("## threads StackOps but has no `.` form: {}\n", gaps.len());
    println!("  Mostly naming artefacts: the `if.*` family spells its workbench");
    println!("  form `.in_workbench` rather than with a `.` suffix.\n");
    println!("  {}\n", gaps.join(" "));

    println!(
        "## has a `.` form without threading StackOps: {}\n",
        counter.len()
    );
    if counter.is_empty() {
        println!("  (none)\n");
    }
    for (w, cite) in counter.iter().take(40) {
        println!("  {w:<28} {cite}");
    }
    if counter.len() > 40 {
        println!("  ... {} more", counter.len() - 40);
    }
    println!();

    println!(
        "## has neither a `.` form nor StackOps: {}\n",
        producers.len()
    );
    println!("  NOT a producer list. Partitioning these into producers and");
    println!("  consumers is exactly what Q11 needs and what this audit cannot");
    println!("  do. Listed for reference only.\n");
    println!("  {}\n", producers.join(" "));

    if !undetermined.is_empty() {
        println!("## handler not resolvable: {}\n", undetermined.len());
        println!("  Registered through a path or macro this scan cannot follow;");
        println!("  these need checking by hand before D18 is applied to them.\n");
        println!("  {}\n", undetermined.join(" "));
    }
}

// ---------------------------------------------------------------------------
// Coverage — the second health number (Q5)
// ---------------------------------------------------------------------------

/// `cargo xtask coverage` — words with a test over words in scope.
///
/// Deliberately *not* folded into `conform`. Conformance is goldens passed
/// over goldens: a regression number over a fixed corpus, which is the only
/// reason CLAUDE.md can require the JIT and AOT milestones to move it by
/// exactly zero and call any movement a bug. Adding words to that denominator
/// would make implementing a word move the number, destroying the invariant.
///
/// So there are two numbers and neither substitutes for the other:
/// conformance answers "did we break observed behaviour", coverage answers
/// "how much of the language is tested at all".
pub fn run_coverage(_args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let programs = load_corpus(&repo);
    if programs.is_empty() {
        return Err(format!(
            "no .bund files under {:?}. Are the reference submodules checked out?",
            CORPUS_ROOTS
        ));
    }
    let reg = registry::scan(&repo, REGISTRY_ROOTS);

    let mut used: BTreeSet<&str> = BTreeSet::new();
    for p in &programs {
        used.extend(p.word_set());
    }

    let all: Vec<&str> = reg.word_names();
    let mut deferred: Vec<&str> = Vec::new();
    let mut in_scope: Vec<&str> = Vec::new();
    for w in &all {
        if Classifier::deferral_of(&reg, w).is_some() {
            deferred.push(w);
        } else {
            in_scope.push(w);
        }
    }
    let covered: Vec<&str> = in_scope
        .iter()
        .copied()
        .filter(|w| used.contains(w))
        .collect();
    let uncovered: Vec<&str> = in_scope
        .iter()
        .copied()
        .filter(|w| !used.contains(w))
        .collect();

    println!("# cargo xtask coverage\n");
    println!("Words with a test, over words in scope. This is NOT conformance.");
    println!("`cargo xtask conform` is goldens passed over goldens — a regression");
    println!("number over a fixed corpus, which is why the JIT and AOT milestones");
    println!("must move it by exactly zero. Coverage is the completeness number.");
    println!("Neither substitutes for the other.\n");

    println!("  registered names            {:>5}", all.len());
    println!("  out of scope by decision    {:>5}", deferred.len());
    println!("  {:-<32}", "");
    println!("  words in scope              {:>5}", in_scope.len());
    println!("  covered by a golden         {:>5}", covered.len());
    println!(
        "  covered by a hand test      {:>5}   (not yet wired — see Q5)",
        0
    );
    println!("  {:-<32}", "");
    let pct = 100.0 * covered.len() as f64 / in_scope.len().max(1) as f64;
    println!(
        "  COVERAGE                {:>5}/{:<5} ({pct:.1}%)\n",
        covered.len(),
        in_scope.len()
    );

    // The uncovered set splits into a cheap half and an expensive one.
    fn base_of(w: &str) -> Option<&str> {
        for suf in [".,", ",", "."] {
            if let Some(b) = w.strip_suffix(suf)
                && !b.is_empty()
            {
                return Some(b);
            }
        }
        None
    }
    let covered_set: BTreeSet<&str> = covered.iter().copied().collect();
    let (variants, orphans): (Vec<&str>, Vec<&str>) = uncovered
        .iter()
        .partition(|w| base_of(w).is_some_and(|b| covered_set.contains(b)));

    println!("## the {} uncovered words\n", uncovered.len());
    println!("  suffix-variant of a covered word  {:>4}", variants.len());
    println!("      A `.` variant differs from its base only by operand source");
    println!("      (StackOps::FromStack vs FromWorkBench —");
    println!("      reference/Bund/src/stdlib/functions/values/push.rs:11-25), so one");
    println!("      mechanical paired test per base covers all of them. Cheapest");
    println!("      coverage available, and D18 preserves them as pairs anyway.");
    println!("      {}\n", variants.join(" "));
    println!("  no covered base                   {:>4}", orphans.len());
    println!("      Genuinely untouched surface. Testing these means probing the");
    println!("      oracle for behaviour, not reading its source.\n");

    let mut by_sub: BTreeMap<String, usize> = BTreeMap::new();
    for w in &orphans {
        let sub = reg
            .implementing_site(w)
            .map(|s| classify::subsystem(&s))
            .unwrap_or_else(|| "?".into());
        *by_sub.entry(sub).or_default() += 1;
    }
    let mut rows: Vec<(String, usize)> = by_sub.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("## uncovered-with-no-covered-base, by subsystem\n");
    for (sub, n) in rows.iter().take(20) {
        println!("  {sub:<24}{n:>4}");
    }
    if rows.len() > 20 {
        println!("  ... {} further subsystems", rows.len() - 20);
    }
    println!();

    probes(&repo, &reg, &covered_set);

    Ok(())
}

/// Authored probes (D21). Structurally checked here so a malformed probe is
/// caught before the oracle is involved; their goldens are counted once
/// `cargo xtask golden` can capture them.
fn probes(repo: &Path, reg: &Registry, covered: &BTreeSet<&str>) {
    let dir = repo.join("tests/probes");
    let mut files = Vec::new();
    collect_bund(&dir, &mut files, false);
    files.sort();

    println!("## probes (D21)\n");
    if files.is_empty() {
        println!("  none authored yet — tests/probes/ is empty.\n");
        return;
    }

    let golden_dir = repo.join("tests/golden/probes");
    let mut captured = 0usize;
    let mut anomalies = 0usize;

    println!(
        "  {} authored. Goldens are captured from the oracle, never written",
        files.len()
    );
    println!("  by hand (D21). Words listed are those each probe invokes.\n");

    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else {
            continue;
        };
        let name = f
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let lexed = lex::lex(&src);
        let words: BTreeSet<&str> = lexed
            .tokens
            .iter()
            .filter(|t| t.kind == Kind::Name)
            .map(|t| t.text.as_str())
            .collect();
        let has_golden = golden_dir.join(format!("{name}.golden")).exists();
        if has_golden {
            captured += 1;
        }
        // A probe naming a word that resolves nowhere is almost always a typo
        // or a word the probe itself defines; flag the former.
        let atoms: BTreeSet<&str> = lexed
            .tokens
            .iter()
            .filter(|t| t.kind == Kind::Atom)
            .map(|t| t.text.as_str())
            .collect();
        let unknown: Vec<&str> = words
            .iter()
            .copied()
            .filter(|w| reg.resolve(w).is_none() && !atoms.contains(w))
            .collect();
        let fresh: Vec<&str> = words
            .iter()
            .copied()
            .filter(|w| !covered.contains(w) && reg.resolve(w).is_some())
            .collect();

        println!(
            "  {:<32} {}",
            name,
            if has_golden {
                "golden captured"
            } else {
                "PENDING — no golden yet"
            }
        );
        if !fresh.is_empty() {
            println!("      newly covered words: {}", fresh.join(" "));
        }
        for a in &lexed.anomalies {
            anomalies += 1;
            println!("      LEX ANOMALY {a}");
        }
        if !unknown.is_empty() {
            anomalies += 1;
            println!("      UNRESOLVED WORDS: {}", unknown.join(" "));
        }
    }

    println!(
        "\n  {captured}/{} captured, {anomalies} structural problem(s).",
        files.len()
    );
    if captured < files.len() {
        println!("  Pending probes contribute nothing to coverage: a probe without");
        println!("  a golden asserts nothing. `cargo xtask golden` is not yet");
        println!("  implemented, so all of them are pending by construction.\n");
    }
}

// ---------------------------------------------------------------------------
// TASK 1 — decision evidence
// ---------------------------------------------------------------------------

fn probe(programs: &[Program], title: &str, decision: &str, words: &[(&str, &str)]) {
    println!("## {title}  [{decision}]\n");
    let mut total = 0usize;
    let mut progs_touched: BTreeSet<&str> = BTreeSet::new();
    for (word, cite) in words {
        let occ = occurrences(programs, word);
        total += occ.len();
        for (p, _, _) in &occ {
            progs_touched.insert(p.name.as_str());
        }
        if occ.is_empty() {
            println!("  {word:<20} 0    (registered at {cite})");
            continue;
        }
        let in_lambda = occ.iter().filter(|(_, _, d)| *d > 0).count();
        let progs: BTreeSet<&str> = occ.iter().map(|(p, _, _)| p.name.as_str()).collect();
        println!(
            "  {word:<20} {}    in {} program(s), {in_lambda} inside a lambda body    \
             (registered at {cite})",
            occ.len(),
            progs.len()
        );
        for (p, line, depth) in occ.iter().take(CITE_CAP) {
            let marker = if *depth > 0 {
                format!(" [lambda depth {depth}]")
            } else {
                String::new()
            };
            println!("      {}:{}{marker}  | {}", p.path, line, p.line(*line));
        }
        if occ.len() > CITE_CAP {
            println!(
                "      ... {} further occurrence(s) not listed; the count above is complete",
                occ.len() - CITE_CAP
            );
        }
    }
    println!(
        "\n  total occurrences: {total}, across {} program(s)\n",
        progs_touched.len()
    );
}

fn task1(programs: &[Program], reg: &Registry) {
    println!("---\n\n# TASK 1 — decision evidence\n");

    // D1. `.id` is a method, not an inline word — it is reached by object
    // dispatch. It is therefore invoked as a bare name only when a program
    // sends it to an object.
    let id_words: Vec<(&str, &str)> = classify::ID_WORDS
        .iter()
        .map(|w| {
            let cite = reg
                .method
                .get(*w)
                .or_else(|| reg.inline.get(*w))
                .and_then(|v| v.first())
                .map(|s| s.cite())
                .unwrap_or_else(|| "unregistered".into());
            (*w, Box::leak(cite.into_boxed_str()) as &str)
        })
        .collect();
    probe(programs, "`.id`", "D1", &id_words);
    // `.id` also appears as an *atom* (`:.id`), which is how a class attribute
    // or a method send is written. Count those separately.
    atom_probe(programs, ".id", "D1");

    let ts_words: Vec<(&str, &str)> = classify::TIMESTAMP_WORDS
        .iter()
        .map(|w| {
            let cite = reg
                .method
                .get(*w)
                .or_else(|| reg.inline.get(*w))
                .and_then(|v| v.first())
                .map(|s| s.cite())
                .unwrap_or_else(|| "unregistered".into());
            (*w, Box::leak(cite.into_boxed_str()) as &str)
        })
        .collect();
    probe(programs, "`.timestamp`", "D2", &ts_words);
    atom_probe(programs, ".timestamp", "D2");

    probe(
        programs,
        "runtime code construction",
        "D3",
        classify::EVAL_FAMILY,
    );
    probe(programs, "in-place mutators", "D5", classify::MUTATORS);
    lambda_mutation(programs);
    probe(
        programs,
        "the `*` fold family",
        "D12",
        classify::FOLD_FAMILY,
    );
    probe(
        programs,
        "word-table mutation",
        "closed-world reachability",
        classify::CLOSED_WORLD,
    );
}

/// Occurrences of a name in *atom* position (`:name`), which is a value push,
/// not an invocation.
fn atom_probe(programs: &[Program], name: &str, decision: &str) {
    let mut hits = Vec::new();
    for p in programs {
        for t in &p.lexed.tokens {
            if t.kind == Kind::Atom && t.text == name {
                hits.push((p, t.line));
            }
        }
    }
    println!(
        "  as an atom `:{name}` (a value push, not an invocation)  [{decision}]: {}",
        hits.len()
    );
    for (p, line) in hits.iter().take(CITE_CAP) {
        println!("      {}:{}  | {}", p.path, line, p.line(*line));
    }
    if hits.len() > CITE_CAP {
        println!(
            "      ... {} further occurrence(s) not listed",
            hits.len() - CITE_CAP
        );
    }
    println!();
}

/// D5: a mutator textually adjacent to a closing lambda.
///
/// Read the operand order before reading the count. `set` pulls three values:
/// the stored value first, then the key, then the receiver
/// (`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:10-27`). In
/// `:.init { ... } set` the lambda is therefore the *stored value* and the
/// receiver is the class underneath it. That is a lambda being filed into a
/// dictionary, not a lambda body being rewritten.
///
/// `push` is the same story from the other side: it converts its receiver
/// with `conv(LIST)` before appending
/// (`reference/Bund/src/stdlib/functions/values/push.rs:34`), so `push`
/// applied to a LAMBDA yields a LIST rather than a mutated LAMBDA.
///
/// So this shape is reported as what it is — an adjacency — and the reader is
/// told what it does and does not show.
fn lambda_mutation(programs: &[Program]) {
    let mutators: BTreeSet<&str> = classify::MUTATORS.iter().map(|(w, _)| *w).collect();
    println!("## mutator textually adjacent to a closing lambda  [D5]\n");
    println!("  Shape: `}}` immediately followed by a mutator word.\n");
    println!("  This counts adjacency, not mutation. `set` pulls stored-value,");
    println!("  key, receiver in that order (value_dict.rs:10-27), so in");
    println!("  `:.init {{ ... }} set` the lambda is the value being stored and the");
    println!("  receiver is the class beneath it — the lambda body is untouched.");
    println!("  `push` converts its receiver with conv(LIST) first (push.rs:34).");
    println!("  Neither word writes through to an existing LAMBDA body. A count");
    println!("  below is evidence of lambdas being *stored*, and is not by itself");
    println!("  evidence for or against D5.\n");

    let mut hits = 0usize;
    for p in programs {
        let toks = &p.lexed.tokens;
        for w in toks.windows(2) {
            if w[0].kind == Kind::CloseLambda
                && w[1].kind == Kind::Name
                && mutators.contains(w[1].text.as_str())
            {
                hits += 1;
                let nested = if w[0].depth > 0 {
                    format!(" [nested, bracket depth {}]", w[0].depth)
                } else {
                    String::new()
                };
                println!(
                    "      {}:{}  `}} {}`{nested}  | {}",
                    p.path,
                    w[1].line,
                    w[1].text,
                    p.line(w[1].line)
                );
            }
        }
    }
    if hits == 0 {
        println!("      (none)");
    }
    println!("\n  total: {hits}\n");
}

// ---------------------------------------------------------------------------
// TASK 2 — word frequency
// ---------------------------------------------------------------------------

struct Freq {
    counts: BTreeMap<String, usize>,
    programs_using: BTreeMap<String, BTreeSet<String>>,
}

fn frequency(programs: &[Program]) -> Freq {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut programs_using: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for p in programs {
        for t in p.words() {
            *counts.entry(t.text.clone()).or_default() += 1;
            programs_using
                .entry(t.text.clone())
                .or_default()
                .insert(p.name.clone());
        }
    }
    Freq {
        counts,
        programs_using,
    }
}

fn task2(programs: &[Program], reg: &Registry) {
    println!("---\n\n# TASK 2 — word frequency\n");
    let freq = frequency(programs);
    let total: usize = freq.counts.values().sum();

    println!(
        "{} distinct words invoked, {} invocations total.\n",
        freq.counts.len(),
        total
    );

    let mut rows: Vec<(&String, usize)> = freq.counts.iter().map(|(k, v)| (k, *v)).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

    println!("## words used by the corpus\n");
    println!("  {:<28} {:>5} {:>6}  resolves to", "word", "uses", "progs");
    for (word, n) in &rows {
        let progs = freq.programs_using[*word].len();
        let res = reg
            .resolve(word)
            .map(|r| r.label())
            .unwrap_or_else(|| "UNRESOLVED".to_string());
        println!("  {word:<28} {n:>5} {progs:>6}  {res}");
    }
    println!();

    // Registered but never used.
    let used: BTreeSet<&str> = freq.counts.keys().map(String::as_str).collect();
    let mut unused: Vec<&str> = reg
        .word_names()
        .into_iter()
        .filter(|w| !used.contains(w))
        .collect();
    unused.sort_unstable();
    let all_names = reg.word_names();
    println!(
        "## registered but never used: {} of {} distinct registered names\n",
        unused.len(),
        all_names.len()
    );
    let overlap: Vec<&str> = reg
        .alias
        .keys()
        .filter(|a| reg.inline.contains_key(*a))
        .map(String::as_str)
        .collect();
    if !overlap.is_empty() {
        println!(
            "  ({} name(s) are registered both as an inline word and as an alias: {})\n",
            overlap.len(),
            overlap.join(" ")
        );
    }
    let mut by_sub: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for w in &unused {
        let sub = reg
            .implementing_site(w)
            .map(|s| classify::subsystem(&s))
            .unwrap_or_else(|| "?".into());
        by_sub.entry(sub).or_default().push(w);
    }
    for (sub, words) in &by_sub {
        println!("  {:<22} {:>3}  {}", sub, words.len(), words.join(" "));
    }
    println!();

    // Used but unresolved.
    println!("## used but not found in any registration set\n");
    let mut unresolved: Vec<&String> = freq
        .counts
        .keys()
        .filter(|w| reg.resolve(w).is_none())
        .collect();
    unresolved.sort();
    if unresolved.is_empty() {
        println!("  (none)");
    }
    for word in &unresolved {
        let progs = &freq.programs_using[*word];
        // Is it an atom somewhere in a program that uses it? Then the program
        // very likely defined it with `register` — lambdas resolve ahead of
        // inline words (multistackvm_apply.rs:46).
        let locally_defined = programs
            .iter()
            .filter(|p| progs.contains(&p.name))
            .any(|p| p.atom_set().contains(word.as_str()));
        let tag = if locally_defined {
            "defined in-corpus (name appears as an atom in a using program)"
        } else {
            "NOT ACCOUNTED FOR"
        };
        println!(
            "  {:<28} {:>3} uses  {:>2} progs  {tag}",
            word,
            freq.counts[*word],
            progs.len()
        );
        if !locally_defined {
            for (p, line, _) in occurrences(programs, word).iter().take(4) {
                println!("      {}:{}  | {}", p.path, line, p.line(*line));
            }
        }
    }
    println!();

    // Ptr references, which name a word without invoking it.
    let mut ptrs: BTreeMap<&str, usize> = BTreeMap::new();
    for p in programs {
        for t in &p.lexed.tokens {
            if t.kind == Kind::Ptr {
                *ptrs.entry(t.text.as_str()).or_default() += 1;
            }
        }
    }
    println!("## named as PTR but not invoked: {}\n", ptrs.len());
    for (w, n) in &ptrs {
        println!("  `{w:<26} {n:>3}");
    }
    println!();
}

// ---------------------------------------------------------------------------
// TASK 3 — core/library partition evidence
// ---------------------------------------------------------------------------

fn task3(programs: &[Program], reg: &Registry) {
    println!("---\n\n# TASK 3 — core/library partition evidence  [D14]\n");
    println!("Grouping is by the subsystem that *implements* the word, taken from");
    println!("its registration path. Aliases are attributed to their target.\n");

    // word -> subsystem
    let mut sub_of: BTreeMap<&str, String> = BTreeMap::new();
    let freq = frequency(programs);
    for word in freq.counts.keys() {
        if let Some(site) = reg.implementing_site(word) {
            sub_of.insert(word.as_str(), classify::subsystem(&site));
        }
    }

    // subsystem -> (words, programs that would break)
    let mut words_in: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut breaks: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for p in programs {
        for w in p.word_set() {
            if let Some(sub) = sub_of.get(w) {
                words_in.entry(sub).or_default().insert(w);
                breaks.entry(sub).or_default().insert(p.name.as_str());
            }
        }
    }

    // For each subsystem, how many programs use it while everything *else*
    // they use is basic. That is the "a core program reaches into this
    // subsystem" signal.
    let mut intrusions: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut intruding_words: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for p in programs {
        let ws = p.word_set();
        let subs: BTreeSet<&str> = ws
            .iter()
            .filter_map(|w| sub_of.get(*w).map(String::as_str))
            .collect();
        for &sub in &subs {
            if is_basic(sub) {
                continue;
            }
            // Everything else this program touches must be basic.
            let rest_all_basic = subs.iter().all(|s| *s == sub || is_basic(s));
            if rest_all_basic {
                intrusions.entry(sub).or_default().insert(p.name.as_str());
                for w in &ws {
                    if sub_of.get(*w).map(String::as_str) == Some(sub) {
                        intruding_words.entry(sub).or_default().insert(w);
                    }
                }
            }
        }
    }

    let mut rows: Vec<(&str, usize, usize, usize)> = breaks
        .iter()
        .map(|(sub, progs)| {
            (
                *sub,
                progs.len(),
                words_in.get(sub).map_or(0, BTreeSet::len),
                intrusions.get(sub).map_or(0, BTreeSet::len),
            )
        })
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

    println!(
        "  {:<22} {:>7} {:>7} {:>10}  {}",
        "subsystem", "breaks", "words", "otherwise", "basic?"
    );
    println!("  {:<22} {:>7} {:>7} {:>10}", "", "progs", "used", "basic");
    for (sub, nbreak, nwords, nintr) in &rows {
        println!(
            "  {:<22} {:>7} {:>7} {:>10}  {}",
            sub,
            nbreak,
            nwords,
            nintr,
            if is_basic(sub) { "basic" } else { "" }
        );
    }
    println!();

    println!("## library-shaped subsystems the corpus leans on from otherwise-basic programs\n");
    println!("  These are the D14 pressure points: a subsystem outside the");
    println!("  stack/math/logic/lambda/oop set that programs reach into while");
    println!("  touching nothing else outside it.\n");
    let mut intr: Vec<(&&str, &BTreeSet<&str>)> = intrusions.iter().collect();
    intr.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    for (sub, progs) in intr {
        let words = intruding_words.get(*sub).cloned().unwrap_or_default();
        println!("  {sub}  — {} program(s)", progs.len());
        println!(
            "      words: {}",
            words.iter().copied().collect::<Vec<_>>().join(" ")
        );
        for p in progs.iter().take(8) {
            println!("      {p}");
        }
        if progs.len() > 8 {
            println!("      ... and {} more", progs.len() - 8);
        }
        println!();
    }

    // The single-word view: which individual words the most programs depend
    // on, annotated with subsystem. This is what a core list would be argued
    // from — reported as counts only.
    println!("## words by number of programs that would break without them\n");
    let mut byprog: Vec<(&String, usize)> = freq
        .programs_using
        .iter()
        .map(|(w, s)| (w, s.len()))
        .collect();
    byprog.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("  {:<28} {:>6}  subsystem", "word", "progs");
    for (w, n) in byprog.iter().take(60) {
        let sub = sub_of
            .get(w.as_str())
            .map(String::as_str)
            .unwrap_or("(in-corpus)");
        println!("  {w:<28} {n:>6}  {sub}");
    }
    println!();
}

// ---------------------------------------------------------------------------
// TASK 4 — hermetic partition
// ---------------------------------------------------------------------------

struct Verdict<'a> {
    program: &'a Program,
    hermetic: bool,
    /// Non-hermetic words, with the effect that disqualifies them.
    blockers: Vec<(String, Effect, String)>,
    /// Words that resolve nowhere, so their effect is unknown.
    unknown: Vec<String>,
    /// Words the owner has put out of scope, with the deciding decision.
    /// Independent of `hermetic`: a colour word is hermetic and out of scope.
    deferred: Vec<(String, &'static str, &'static str)>,
    /// The oracle does not reproduce this program between runs. Empirical —
    /// see `load_unstable`.
    unstable: bool,
}

impl Verdict<'_> {
    /// In the conformance suite: reproducible *and* something Bund2 will
    /// implement.
    fn in_suite(&self) -> bool {
        self.hermetic && self.deferred.is_empty() && !self.unstable
    }
}

fn task4<'a>(
    programs: &'a [Program],
    reg: &Registry,
    unstable: &BTreeSet<String>,
) -> Vec<Verdict<'a>> {
    println!("---\n\n# TASK 4 — hermetic partition\n");
    println!("A program is hermetic when every word it invokes is pure, writes");
    println!("stdout, or writes a diagnostic to stderr. Words the corpus defines");
    println!("itself carry the effects of the words in their bodies, which are");
    println!("counted because the lexer sees the whole file.\n");
    println!("Scope is a second, independent filter. A word can be perfectly");
    println!("hermetic and still be out of scope — colour output is deterministic");
    println!("bytes on stdout that Bund2 will not emit. The conformance suite is");
    println!("the intersection: hermetic AND in scope.\n");

    let mut verdicts = Vec::new();
    for p in programs {
        let mut blockers = Vec::new();
        let mut unknown = Vec::new();
        let mut deferred = Vec::new();
        let atoms = p.atom_set();
        for w in p.word_set() {
            if let Some((d, _)) = Classifier::deferral_of(reg, w) {
                deferred.push((w.to_string(), d.decision, d.reason));
            }
            match Classifier::effect_of(reg, w) {
                Some((e, site)) if !e.hermetic() => {
                    blockers.push((w.to_string(), e, site.cite()));
                }
                Some(_) => {}
                None => {
                    // In-corpus definitions are not blockers: their bodies are
                    // lexed too, so their effects are already accounted for.
                    if !atoms.contains(w) {
                        unknown.push(w.to_string());
                    }
                }
            }
        }
        blockers.sort();
        deferred.sort();
        verdicts.push(Verdict {
            program: p,
            unstable: unstable.contains(&p.path),
            deferred,
            hermetic: blockers.is_empty() && unknown.is_empty(),
            blockers,
            unknown,
        });
    }

    let n_herm = verdicts.iter().filter(|v| v.hermetic).count();
    println!(
        "  hermetic: {} / {}   non-hermetic: {}\n",
        n_herm,
        verdicts.len(),
        verdicts.len() - n_herm
    );

    println!("## non-hermetic, with the word that disqualifies\n");
    for v in verdicts.iter().filter(|v| !v.hermetic) {
        let mut why: Vec<String> = v
            .blockers
            .iter()
            .map(|(w, e, c)| format!("{w} [{}] {c}", e.as_str()))
            .collect();
        if !v.unknown.is_empty() {
            why.push(format!("unresolved: {}", v.unknown.join(" ")));
        }
        println!(
            "  {:<52} {}",
            v.program.name,
            why.first().cloned().unwrap_or_default()
        );
        for extra in why.iter().skip(1) {
            println!("  {:<52} {extra}", "");
        }
    }
    println!();

    // Effect histogram, so the reader can see what dominates.
    let mut hist: BTreeMap<&str, usize> = BTreeMap::new();
    for v in &verdicts {
        for (_, e, _) in &v.blockers {
            *hist.entry(e.as_str()).or_default() += 1;
        }
    }
    println!("## disqualifying effects, by count of (program, word) pairs\n");
    let mut h: Vec<_> = hist.into_iter().collect();
    h.sort_by(|a, b| b.1.cmp(&a.1));
    for (e, n) in h {
        println!("  {e:<14} {n:>4}");
    }
    println!();

    // Scope, the second filter. Named explicitly: a program dropped from the
    // suite by a scope decision must be visible, not absent.
    let scoped_out: Vec<&Verdict> = verdicts.iter().filter(|v| !v.deferred.is_empty()).collect();
    println!("## out of scope by decision\n");
    if scoped_out.is_empty() {
        println!("  (none)\n");
    } else {
        for v in &scoped_out {
            let decision = v.deferred[0].1;
            let reason = v.deferred[0].2;
            let words: Vec<&str> = v.deferred.iter().map(|(w, _, _)| w.as_str()).collect();
            println!(
                "  {:<44} [{decision}] {reason}{}",
                v.program.name,
                if v.hermetic {
                    ""
                } else {
                    "  (also non-hermetic)"
                }
            );
            println!("      {}", words.join(" "));
        }
        println!();
    }

    let unstable_n = verdicts.iter().filter(|v| v.unstable).count();
    println!("## does not reproduce between oracle runs: {unstable_n}\n");
    println!("  Empirical, from tests/golden/UNSTABLE.txt — not derivable here.");
    println!("  Every word these invoke is pure or stdout; the non-determinism");
    println!("  enters through Debug formatting (F14) and map iteration order");
    println!("  (F15). Only running the oracle twice finds them.\n");
    for v in verdicts.iter().filter(|v| v.unstable) {
        println!("  {}", v.program.name);
    }
    println!();

    let n_suite = verdicts.iter().filter(|v| v.in_suite()).count();
    println!(
        "  conformance suite (hermetic AND in scope AND reproducible): {} / {}\n",
        n_suite,
        verdicts.len()
    );

    verdicts
}

/// Programs the oracle does not reproduce between runs, from
/// `tests/golden/UNSTABLE.txt`.
///
/// This is empirical input a static pass cannot derive. Every word these
/// programs invoke is pure or writes stdout; the non-determinism enters
/// through `Debug` formatting (F14) and `HashMap` iteration order (F15),
/// neither of which is a word effect. Without this list `HERMETIC.txt` would
/// name 18 programs that cannot pass their own golden.
fn load_unstable(repo: &Path) -> BTreeSet<String> {
    let path = repo.join("tests/golden/UNSTABLE.txt");
    let Ok(src) = std::fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    src.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

fn write_hermetic(repo: &Path, verdicts: &[Verdict]) -> Result<(), String> {
    let dir = repo.join("tests/golden");
    std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    let path = dir.join("HERMETIC.txt");

    let mut s = String::new();
    s.push_str("# Hermetic conformance suite.\n");
    s.push_str("#\n");
    s.push_str("# One program per line, repo-relative. Generated by `cargo xtask corpus`.\n");
    s.push_str("#\n");
    s.push_str("# A program is listed only if it passes all three filters below.\n");
    s.push_str("#\n");
    s.push_str("# 1. Hermetic: every word it invokes is pure, writes stdout, or writes a\n");
    s.push_str("#    diagnostic to stderr — no network, filesystem, image, database, bus,\n");
    s.push_str("#    clock, randomness, or host state.\n");
    s.push_str("# 2. In scope: no word it invokes has been deferred by a decision in\n");
    s.push_str("#    docs/registers/decisions.md.\n");
    s.push_str("# 3. Reproducible: the oracle produces identical output on two runs.\n");
    s.push_str("#    Empirical, and necessarily so — the non-determinism enters through\n");
    s.push_str("#    Debug formatting and map iteration, neither of which is a word\n");
    s.push_str("#    effect, so no static analysis of the word table can derive it.\n");
    s.push_str("#    See tests/golden/UNSTABLE.txt.\n");
    s.push_str("#\n");
    s.push_str("# Filters 1 and 2 are derived from each word's registration site in\n");
    s.push_str("# reference/; see xtask/src/corpus/classify.rs for the rules and their\n");
    s.push_str("# citations.\n");
    s.push_str("#\n");

    // Spell the funnel out. Reporting only the final count invites reading a
    // later "refused 0" as "everything hermetic was captured", when in fact
    // most of the reduction happened here, upstream of any capture.
    let total = verdicts.len();
    let herm = verdicts.iter().filter(|v| v.hermetic).count();
    let scope_out = verdicts
        .iter()
        .filter(|v| v.hermetic && !v.deferred.is_empty())
        .count();
    let unstable_out = verdicts
        .iter()
        .filter(|v| v.hermetic && v.deferred.is_empty() && v.unstable)
        .count();
    let suite = verdicts.iter().filter(|v| v.in_suite()).count();
    s.push_str("# How the suite narrows, in order:\n");
    s.push_str("#\n");
    s.push_str(&format!("#   {total:>4}  programs in the corpus\n"));
    s.push_str(&format!(
        "#   {:>4}  non-hermetic, dropped by filter 1\n",
        total - herm
    ));
    s.push_str(&format!("#   {herm:>4}  hermetic\n"));
    s.push_str(&format!(
        "#   {scope_out:>4}  hermetic but out of scope, dropped by filter 2\n"
    ));
    s.push_str(&format!(
        "#   {unstable_out:>4}  hermetic and in scope but not reproducible, dropped by filter 3\n"
    ));
    s.push_str(&format!("#   {suite:>4}  in this file\n"));
    s.push_str("#\n");

    // Name every program a scope decision removed. A program that simply
    // vanishes from this list reads as "not hermetic", which would be wrong.
    let scoped_out: Vec<&Verdict> = verdicts
        .iter()
        .filter(|v| v.hermetic && !v.deferred.is_empty())
        .collect();
    if !scoped_out.is_empty() {
        s.push_str("#\n");
        s.push_str("# Hermetic but excluded by a scope decision:\n");
        for v in &scoped_out {
            s.push_str(&format!(
                "#   {}  [{}] {}\n",
                v.program.path, v.deferred[0].1, v.deferred[0].2
            ));
        }
    }
    s.push('\n');

    for v in verdicts.iter().filter(|v| v.in_suite()) {
        s.push_str(&v.program.path);
        s.push('\n');
    }

    std::fs::write(&path, s).map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!(
        "wrote {}",
        path.strip_prefix(repo).unwrap_or(&path).display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Effect audit — Q4
// ---------------------------------------------------------------------------

/// Cross-check every used word's assigned effect against the crates its
/// registering file imports.
///
/// The effect table in `classify.rs` keys off registration *paths*, which is a
/// proxy: a directory says who wrote a word, not what it does. `display` is
/// the known case — a terminal renderer filed under `system/`
/// (`reference/Bund/src/stdlib/functions/system/display.rs:88`). This audit
/// looks for the ones nobody spotted by hand.
///
/// Import markers are indicators, not verdicts. A file may import `rand` and
/// use it in one function of ten. The audit reports; a human decides.
fn effect_audit(programs: &[Program], reg: &Registry) {
    println!("\n---\n\n# Effect audit  [Q4]\n");
    println!("Assigned effect vs. what the registering file imports. Import");
    println!("markers are indicators, not verdicts — this is a review list.\n");

    let mut used: BTreeSet<&str> = BTreeSet::new();
    for p in programs {
        used.extend(p.word_set());
    }

    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut cache: BTreeMap<String, Vec<(Effect, String)>> = BTreeMap::new();
    let mut under: Vec<(String, Effect, Effect, String, String)> = Vec::new();
    let mut over: Vec<(String, Effect, String)> = Vec::new();

    for word in &used {
        let Some((assigned, site)) = Classifier::effect_of(reg, word) else {
            continue;
        };
        let implied = cache.entry(site.path.clone()).or_insert_with(|| {
            std::fs::read_to_string(repo.join(&site.path))
                .map(|s| classify::implied_effects(&s))
                .unwrap_or_default()
        });

        // Dangerous direction: we call it hermetic, the imports do not.
        if assigned.hermetic()
            && let Some((e, line)) = implied.iter().find(|(e, _)| !e.hermetic())
        {
            under.push((word.to_string(), assigned, *e, site.cite(), line.clone()));
        }
        // Costly direction: we call it effectful, no import supports it.
        if !assigned.hermetic() && !implied.iter().any(|(e, _)| *e == assigned) {
            over.push((word.to_string(), assigned, site.cite()));
        }
    }

    println!(
        "## classified hermetic, imports suggest otherwise: {}\n",
        under.len()
    );
    println!("  These are the ones that matter: a false hermetic verdict puts a");
    println!("  program into the golden suite that cannot reproduce.\n");
    if under.is_empty() {
        println!("  (none)\n");
    }
    for (w, a, i, cite, line) in &under {
        println!("  {:<26} {} -> imports imply {}", w, a.as_str(), i.as_str());
        println!("      {cite}");
        println!("      {line}");
    }

    println!(
        "\n## classified effectful, no import supports it: {}\n",
        over.len()
    );
    println!("  Lower stakes — these cost coverage rather than correctness. A");
    println!("  word may still be effectful through a helper in another file.\n");
    if over.is_empty() {
        println!("  (none)");
    }
    for (w, a, cite) in &over {
        println!("  {:<26} {:<12} {cite}", w, a.as_str());
    }
    println!();
}

// ---------------------------------------------------------------------------
// Implementation reachability — Q4, option D
// ---------------------------------------------------------------------------

/// What each corpus-used word's implementation reaches into, beyond its own
/// subsystem.
///
/// D14 is resolved per word (D17, D19), and a per-word ruling silently commits
/// to whatever that word's implementation calls. D19 is the worked example:
/// preserving `display` also preserves `conditional_fmt::conditional_run`
/// (`reference/Bund/src/stdlib/functions/system/display.rs:36`), which is the
/// `fmt` word's machinery. That was found by reading the file. This section
/// finds the rest.
///
/// Subsystem grouping in TASK 3 is left as it is — it answers "which module
/// could we not ship", a different question from "what does this ruling drag
/// in". Both are reported; neither is bent into the other.
fn reachability(programs: &[Program], reg: &Registry) {
    println!("\n---\n\n# Implementation reachability  [Q4]\n");
    println!("For each corpus-used word, what its implementing file references");
    println!("in other stdlib subsystems. A per-word D14 ruling commits to");
    println!("everything listed against it.\n");

    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    let mut used: BTreeSet<&str> = BTreeSet::new();
    for p in programs {
        used.extend(p.word_set());
    }

    // Many words share a registration file; report per file, listing the
    // corpus-used words it provides.
    let mut by_file: BTreeMap<String, (String, BTreeSet<&str>)> = BTreeMap::new();
    for word in &used {
        let Some(site) = reg.implementing_site(word) else {
            continue;
        };
        let sub = classify::subsystem(&site);
        by_file
            .entry(site.path.clone())
            .or_insert_with(|| (sub, BTreeSet::new()))
            .1
            .insert(word);
    }

    let mut crossing = 0usize;
    let mut self_contained = 0usize;
    let mut rows: Vec<(String, String, BTreeSet<&str>, Vec<String>)> = Vec::new();

    for (path, (sub, words)) in &by_file {
        let Ok(src) = std::fs::read_to_string(repo.join(path)) else {
            continue;
        };
        // A word reaching its own subsystem is not a dependency worth
        // reporting; only cross-subsystem edges commit you to something new.
        let reached: Vec<String> = classify::reached_modules(&src, path)
            .into_iter()
            .filter(|r| r != sub && !r.starts_with(&format!("{sub}/")))
            .collect();
        if reached.is_empty() {
            self_contained += 1;
        } else {
            crossing += 1;
            rows.push((path.clone(), sub.clone(), words.clone(), reached));
        }
    }

    println!(
        "  {} registration files provide the {} words the corpus uses.",
        by_file.len(),
        used.len()
    );
    println!("  {self_contained} are self-contained; {crossing} reach another subsystem.\n");

    rows.sort_by(|a, b| b.3.len().cmp(&a.3.len()).then(a.0.cmp(&b.0)));
    for (path, sub, words, reached) in &rows {
        println!("  {path}");
        println!("      subsystem: {sub}   reaches: {}", reached.join(" "));
        println!(
            "      corpus-used words here: {}",
            words.iter().copied().collect::<Vec<_>>().join(" ")
        );
    }
    println!();
}

fn lexer_anomalies(programs: &[Program]) {
    let total: usize = programs.iter().map(|p| p.lexed.anomalies.len()).sum();
    println!("\n---\n\n# Lexer anomalies: {total}\n");
    if total == 0 {
        println!("  (none — every token matched a rule in bund.pest)");
        return;
    }
    println!("  Places where the corpus does not match bund.pest as written.");
    println!("  These are reported, not corrected.\n");
    for p in programs {
        for a in &p.lexed.anomalies {
            println!("  {}:{}  {}", p.path, a.line, a.what);
        }
    }
}
