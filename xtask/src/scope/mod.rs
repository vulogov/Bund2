//! `cargo xtask scope` — compute D14's core/library partition.
//!
//! D14 asks which in-scope words are language core, preserved 100%, and which
//! are library, deferrable and re-implementable out of tree. The owner chose
//! **method B**: core is what the corpus invokes, closed over what those
//! implementations reach into.
//!
//! The partition is computed, not enumerated, so it cannot drift from the
//! evidence. Every step is re-derivable by re-running this command:
//!
//! 1. **Seed** — the distinct words the corpus invokes. Aliases are seeded
//!    together with their targets, since calling an alias calls the target.
//! 2. **Closure** — for each file providing a seed word, the subsystems its
//!    implementation reaches into (`classify::reached_modules`). Every word
//!    registered in a reached subsystem joins the core. This is what D19
//!    established when preserving `display` also preserved `conditional_fmt`.
//! 3. **Workbench forms** — D18 binds each preserved word to its `.` form, so
//!    a core word's `.` sibling is core.
//!
//! What this deliberately does **not** do is complete a file or a subsystem
//! merely because part of it is core. That was the `B'` variant; the owner
//! chose B. The difference is reported below so the cost of the choice is
//! visible rather than assumed.
//!
//! Nothing here resolves D14. It produces the list the decision needs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::corpus::{self, classify, registry};

/// One word's partition verdict, with the reason it landed there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Invoked by the corpus.
    Seed,
    /// In a subsystem a seed word's implementation reaches into.
    Closure(String),
    /// D18's workbench form of a core word.
    WorkbenchForm(String),
    /// Not reached by any of the above.
    Library,
}

impl Verdict {
    fn is_core(&self) -> bool {
        !matches!(self, Verdict::Library)
    }
}

/// The `.`-suffixed sibling of a word, if the name has one.
fn workbench_base(word: &str) -> Option<&str> {
    word.strip_suffix('.').filter(|b| !b.is_empty())
}

pub fn run(_args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let programs = corpus::load_corpus(&repo);
    if programs.is_empty() {
        return Err("no .bund files found; are the reference submodules checked out?".into());
    }
    let reg = registry::scan(&repo, corpus::REGISTRY_ROOTS);

    // In-scope universe: registered names minus those deferred by decision.
    let in_scope: BTreeSet<String> = reg
        .word_names()
        .into_iter()
        .filter(|w| classify::Classifier::deferral_of(&reg, w).is_none())
        .map(str::to_string)
        .collect();

    // --- step 1: seed -------------------------------------------------------
    let mut verdict: BTreeMap<String, Verdict> = BTreeMap::new();
    let mut invoked: BTreeSet<String> = BTreeSet::new();
    for p in &programs {
        for w in p.word_set() {
            invoked.insert(w.to_string());
        }
    }
    for w in &invoked {
        if in_scope.contains(w) {
            verdict.insert(w.clone(), Verdict::Seed);
        }
        // Calling an alias calls its target, so the target is core too.
        if let Some((target, _)) = reg.alias.get(w.as_str())
            && in_scope.contains(target.as_str())
        {
            verdict.entry(target.clone()).or_insert(Verdict::Seed);
        }
    }
    let seeded = verdict.len();

    // --- step 2: closure ----------------------------------------------------
    // Subsystems reached by the implementation of any seed word.
    let mut reached: BTreeMap<String, String> = BTreeMap::new(); // subsystem -> witness word
    for w in verdict.keys().cloned().collect::<Vec<_>>() {
        let Some(site) = reg.implementing_site(&w) else {
            continue;
        };
        let Ok(src) = std::fs::read_to_string(repo.join(&site.path)) else {
            continue;
        };
        for m in classify::reached_modules(&src, &site.path) {
            reached.entry(m).or_insert_with(|| w.clone());
        }
    }
    for w in &in_scope {
        if verdict.contains_key(w) {
            continue;
        }
        let Some(site) = reg.implementing_site(w) else {
            continue;
        };
        let sub = classify::subsystem(&site);
        if let Some(witness) = reached.get(&sub) {
            verdict.insert(w.clone(), Verdict::Closure(format!("{sub} ({witness})")));
        }
    }
    let after_closure = verdict.len();

    // --- step 3: D18 workbench forms ---------------------------------------
    for w in &in_scope {
        if verdict.contains_key(w) {
            continue;
        }
        if let Some(base) = workbench_base(w)
            && verdict.contains_key(base)
        {
            verdict.insert(w.clone(), Verdict::WorkbenchForm(base.to_string()));
        }
    }
    let after_forms = verdict.len();

    // Everything else is library.
    for w in &in_scope {
        verdict.entry(w.clone()).or_insert(Verdict::Library);
    }

    let core: Vec<&String> = verdict
        .iter()
        .filter(|(_, v)| v.is_core())
        .map(|(k, _)| k)
        .collect();
    let library: Vec<&String> = verdict
        .iter()
        .filter(|(_, v)| !v.is_core())
        .map(|(k, _)| k)
        .collect();

    // ------------------------------------------------------------------
    println!("# cargo xtask scope\n");
    println!("D14's core/library partition, method B: corpus-seeded, closed over");
    println!("implementation reach, plus D18 workbench forms. Computed, not");
    println!("enumerated, so it cannot drift from the evidence.\n");
    println!("This does not resolve D14. It produces the list the decision needs.\n");

    println!("## the partition\n");
    println!("  registered names            {:>5}", reg.word_names().len());
    println!(
        "  out of scope by decision    {:>5}",
        reg.word_names().len() - in_scope.len()
    );
    println!("  ---------------------------------");
    println!("  words in scope              {:>5}", in_scope.len());
    println!();
    println!("  step 1  seeded by the corpus    {:>4}", seeded);
    println!(
        "  step 2  + implementation closure {:>4}   (+{})",
        after_closure,
        after_closure - seeded
    );
    println!(
        "  step 3  + D18 workbench forms    {:>4}   (+{})",
        after_forms,
        after_forms - after_closure
    );
    println!("  ---------------------------------");
    println!(
        "  CORE                        {:>5}   ({:.1}% of in-scope)",
        core.len(),
        core.len() as f64 * 100.0 / in_scope.len().max(1) as f64
    );
    println!("  LIBRARY                     {:>5}", library.len());
    println!();

    // ------------------------------------------------------------------
    // The cost of B over B': subsystems that end up split.
    println!("## where method B splits a subsystem\n");
    println!("  A subsystem with both core and library words. B' would have");
    println!("  completed these; B does not. Each row is a place where a word");
    println!("  sits beside a core sibling and is still library.\n");
    let mut by_sub: BTreeMap<String, (Vec<&String>, Vec<&String>)> = BTreeMap::new();
    for (w, v) in &verdict {
        let Some(site) = reg.implementing_site(w) else {
            continue;
        };
        let e = by_sub.entry(classify::subsystem(&site)).or_default();
        if v.is_core() { e.0.push(w) } else { e.1.push(w) }
    }
    let mut split: Vec<(&String, usize, usize)> = by_sub
        .iter()
        .filter(|(_, (c, l))| !c.is_empty() && !l.is_empty())
        .map(|(s, (c, l))| (s, c.len(), l.len()))
        .collect();
    split.sort_by(|a, b| b.2.cmp(&a.2));
    println!("  {:<26}{:>6}{:>9}", "subsystem", "core", "library");
    for (s, c, l) in split.iter().take(20) {
        println!("  {s:<26}{c:>6}{l:>9}");
    }
    let split_library: usize = split.iter().map(|(_, _, l)| l).sum();
    println!();
    println!(
        "  {} words are library while sharing a subsystem with a core word.",
        split_library
    );
    println!("  That is the B-versus-B' delta, and it is the number to look at");
    println!("  before treating this partition as settled.\n");

    // ------------------------------------------------------------------
    println!("## library words in otherwise-core subsystems, listed\n");
    println!("  The ones most likely to be misfiled. Read these before D14 is");
    println!("  recorded — a word here is one the corpus happens not to reach.\n");
    for (s, _, _) in split.iter().take(8) {
        let (_, lib) = &by_sub[*s];
        let mut names: Vec<&str> = lib.iter().map(|w| w.as_str()).collect();
        names.sort();
        println!("  {s}");
        println!("      {}", names.join(" "));
    }
    println!();

    println!("## core, by how it was reached\n");
    let mut by_reason: BTreeMap<&str, usize> = BTreeMap::new();
    for v in verdict.values().filter(|v| v.is_core()) {
        let k = match v {
            Verdict::Seed => "corpus",
            Verdict::Closure(_) => "closure",
            Verdict::WorkbenchForm(_) => "D18 form",
            Verdict::Library => unreachable!(),
        };
        *by_reason.entry(k).or_default() += 1;
    }
    for (k, n) in &by_reason {
        println!("  {k:<12}{n:>5}");
    }
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_workbench_form_finds_its_base() {
        assert_eq!(workbench_base("format."), Some("format"));
        assert_eq!(workbench_base("format"), None);
    }

    /// A bare `.` is a word in its own right — the alias for `return`. It must
    /// not be read as the workbench form of the empty name.
    #[test]
    fn a_bare_dot_is_not_a_workbench_form() {
        assert_eq!(workbench_base("."), None);
    }

    #[test]
    fn library_is_the_only_non_core_verdict() {
        assert!(!Verdict::Library.is_core());
        assert!(Verdict::Seed.is_core());
        assert!(Verdict::Closure("vm/math".into()).is_core());
        assert!(Verdict::WorkbenchForm("format".into()).is_core());
    }
}
