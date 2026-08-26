//! D14's candidate partitions, computed side by side.
//!
//! Method B, chosen by the owner, files `<=`, `and`, `or` and
//! `convert.to_int` as *library*, because the 132-program corpus happens never
//! to invoke them. That is not a bug in B; it is B working as specified, and
//! it is why the partition needs a variant comparison rather than a single
//! number before D14 records anything.
//!
//! Five variants, differing only in how the core set is closed:
//!
//! - **B** — corpus seed, implementation closure, D18 workbench forms.
//! - **B+probes** — B, but the authored probes seed too. They are programs we
//!   run; `debug.display_stack` being library while the whole golden epilogue
//!   is built on it is an artefact of not counting them.
//! - **B'** — B+probes, then *file* completion: a file with any core word is
//!   wholly core. File-level, because that is the granularity D14's closure
//!   method already uses.
//! - **B-double-prime** — B', but file completion applies only in `vm/` and
//!   `stack/`. The Library Guide's own three-layer split puts the standard
//!   library in the Bund runtime, so completing a `bund/` file drags in a
//!   library family while completing a `vm/` file completes a language
//!   feature. This is the variant the numbers favour.
//! - **C** — B+probes, then *subsystem* completion. Coarsest, and it
//!   contradicts Q4's ruling that subsystem grouping is a reporting aid only.
//!   Carried to bound the range.
//!
//! Nothing here resolves D14. It prices the choice.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::corpus::{classify, registry::Registry};

/// How a variant closes the core set after seeding.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Completion {
    /// No completion beyond closure and D18 forms.
    None,
    /// A file with any core word is wholly core.
    File,
    /// A subsystem with any core word is wholly core.
    Subsystem,
    /// File completion, but only inside the two **language** layers.
    ///
    /// The Library Guide's own three-layer split says where the library is:
    /// `rust_multistack` is stack operations, `rust_multistackvm` holds "the
    /// core logic of the BUND language", and the Bund runtime is where "all
    /// standard library functions" live
    /// (`reference/Bund/Documentation/Bund_Library_Guide/Library_introduction.typ:15-19`).
    /// `cargo xtask guide` cross-checked that split against the registration
    /// paths and found 96 agreements and 0 disagreements.
    ///
    /// So completing a file is right in `vm/` and `stack/`, where a partly
    /// used file means a partly used *language feature*, and wrong in
    /// `bund/`, where it means a partly used *library*.
    FileInLanguageLayers,
}

pub struct Variant {
    pub name: &'static str,
    pub note: &'static str,
    pub core: BTreeSet<String>,
}

/// Words a variant is judged on: the ones method B visibly misfiles.
pub const BELLWETHERS: &[(&str, &str)] = &[
    ("<=", "comparison"),
    (">=", "comparison"),
    ("and", "boolean"),
    ("or", "boolean"),
    ("?true", "predicate"),
    ("convert.to_int", "conversion"),
    ("debug.display_stack", "the golden epilogue depends on it"),
    ("math.ln", "arithmetic"),
];

/// Compute one partition.
///
/// `seed` is already filtered to in-scope names. Closure and D18 are applied
/// in every variant; `completion` is what distinguishes them.
pub fn partition(
    repo: &Path,
    reg: &Registry,
    in_scope: &BTreeSet<String>,
    seed: &BTreeSet<String>,
    completion: Completion,
) -> BTreeSet<String> {
    let mut core: BTreeSet<String> = seed.clone();

    // Closure: every word in a subsystem a seed word's implementation reaches.
    let mut reached: BTreeSet<String> = BTreeSet::new();
    for w in seed {
        let Some(site) = reg.implementing_site(w) else {
            continue;
        };
        let Ok(src) = std::fs::read_to_string(repo.join(&site.path)) else {
            continue;
        };
        reached.extend(classify::reached_modules(&src, &site.path));
    }
    for w in in_scope {
        if core.contains(w) {
            continue;
        }
        if let Some(site) = reg.implementing_site(w)
            && reached.contains(&classify::subsystem(&site))
        {
            core.insert(w.clone());
        }
    }

    // Completion, if the variant asks for it.
    if completion != Completion::None {
        let key = |w: &str| -> Option<String> {
            let site = reg.implementing_site(w)?;
            Some(match completion {
                Completion::File => site.path.clone(),
                Completion::Subsystem => classify::subsystem(&site),
                Completion::FileInLanguageLayers => {
                    let sub = classify::subsystem(&site);
                    if sub.starts_with("vm/") || sub.starts_with("stack/") {
                        site.path.clone()
                    } else {
                        // A key nothing else can share, so `bund/` files are
                        // never completed.
                        format!("\0{w}")
                    }
                }
                Completion::None => unreachable!(),
            })
        };
        let live: BTreeSet<String> = core.iter().filter_map(|w| key(w)).collect();
        for w in in_scope {
            if !core.contains(w)
                && let Some(k) = key(w)
                && live.contains(&k)
            {
                core.insert(w.clone());
            }
        }
    }

    // D18: a core word's workbench form is core. Applied last so it picks up
    // whatever completion added.
    let snapshot: Vec<String> = core.iter().cloned().collect();
    for w in in_scope {
        if core.contains(w) {
            continue;
        }
        if let Some(base) = w.strip_suffix('.').filter(|b| !b.is_empty())
            && snapshot.iter().any(|c| c == base)
        {
            core.insert(w.clone());
        }
    }

    core
}

/// Render the comparison the decision needs.
pub fn report(variants: &[Variant], in_scope: &BTreeSet<String>) {
    println!("## the five partitions\n");
    println!(
        "  {:<12}{:>7}{:>10}{:>9}   {}",
        "variant", "core", "library", "core %", "closes by"
    );
    for v in variants {
        println!(
            "  {:<12}{:>7}{:>10}{:>8.1}%   {}",
            v.name,
            v.core.len(),
            in_scope.len() - v.core.len(),
            v.core.len() as f64 * 100.0 / in_scope.len().max(1) as f64,
            v.note
        );
    }
    println!();

    println!("## do they file the bellwether words as core?\n");
    println!("  Words method B visibly misfiles. A variant that leaves these in");
    println!("  library is one that calls a comparison operator a library word.\n");
    let mut head = format!("  {:<24}", "word");
    for v in variants {
        head.push_str(&format!("{:>12}", v.name));
    }
    println!("{head}   why it matters");
    for (w, why) in BELLWETHERS {
        let mut row = format!("  {w:<24}");
        for v in variants {
            row.push_str(&format!("{:>12}", if v.core.contains(*w) { "core" } else { "—" }));
        }
        println!("{row}   {why}");
    }
    println!();
}

/// Words that are core under `wide` but library under `narrow`.
pub fn delta<'a>(narrow: &'a BTreeSet<String>, wide: &'a BTreeSet<String>) -> Vec<&'a String> {
    wide.difference(narrow).collect()
}

/// Group a word list by implementing subsystem, for readable output.
pub fn by_subsystem(reg: &Registry, words: &[&String]) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for w in words {
        let sub = reg
            .implementing_site(w)
            .map(|s| classify::subsystem(&s))
            .unwrap_or_else(|| "?".to_string());
        out.entry(sub).or_default().push((*w).clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_variants_are_distinct() {
        assert_ne!(Completion::None, Completion::File);
        assert_ne!(Completion::File, Completion::Subsystem);
        assert_ne!(Completion::File, Completion::FileInLanguageLayers);
    }

    #[test]
    fn the_bellwethers_are_the_words_b_misfiles() {
        // If this list ever empties, method B stopped needing a variant
        // comparison — which would itself be worth noticing.
        assert!(BELLWETHERS.iter().any(|(w, _)| *w == "<="));
        assert!(BELLWETHERS.iter().any(|(w, _)| *w == "convert.to_int"));
    }

    #[test]
    fn delta_is_what_the_wider_variant_adds() {
        let narrow: BTreeSet<String> = ["a".to_string()].into_iter().collect();
        let wide: BTreeSet<String> = ["a".to_string(), "b".to_string()].into_iter().collect();
        assert_eq!(delta(&narrow, &wide), vec![&"b".to_string()]);
    }
}
