//! `cargo xtask guide` — cross-reference the Library Guide against the registry.
//!
//! `reference/Bund/Documentation/Bund_Library_Guide` is Typst source for a
//! book. `docs/research/05-rfc-roadmap.md` §1.5 proposes it as the normative
//! reference against which "100% preserved" is judged, and RFC-0000 declined
//! to adopt it because nobody had read it (Q17). This command is what reading
//! it produced: the guide makes checkable claims, so they are checked here
//! rather than paraphrased into an RFC where they would drift.
//!
//! Five checks:
//!
//! 1. **Every documented word is callable.** A page in `lib/` asserts the word
//!    exists. Resolving each against the registry finds the ones that do not.
//! 2. **Every page is reachable.** The book renders from `index.csv`, so a
//!    directory the index does not name is written and never printed.
//! 3. **The "Defined in" checkbox agrees with the registration site.** The
//!    guide attributes each word to one of three layers; `classify::subsystem`
//!    derives the same thing from the path. Two independent sources for D14's
//!    axis, and disagreements are where one of them is wrong.
//! 4. **`#danger` pages against the derived effect.** The author flagged
//!    hazardous words by hand; the effect audit derives hazard from the
//!    registration path. Comparing them sorts the guide's warnings into two
//!    kinds it does not itself distinguish — touching the world, and losing
//!    your own data — and only the first is an effect a golden cannot survive.
//! 5. **The reverse direction.** Documented words the audit calls hazardous
//!    where the page carries no warning at all.
//!
//! What this cannot check is whether a description is *true* — that still
//! needs a reader, the same limit `cargo xtask cite` has.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::corpus::classify::{self, Effect};
use crate::corpus::registry::{self, Registry};

const GUIDE: &str = "reference/Bund/Documentation/Bund_Library_Guide";

const REGISTRY_ROOTS: &[&str] = &[
    "reference/Bund/src",
    "reference/rust_multistackvm/src",
    "reference/rust_multistack/src",
];

/// One documented word.
struct Page {
    /// The `lib/` directory name, verbatim.
    dir: String,
    /// The word the directory names, after undoing the guide's own escaping.
    word: String,
    /// Layers ticked in the "Defined in" box.
    defined_in: Vec<String>,
    /// The `#danger` text, if the page carries a real one.
    danger: Option<String>,
    /// Whether `index.csv` names this directory.
    indexed: bool,
}

/// Undo the guide's directory-name escaping.
///
/// A directory cannot hold every character a word can, so the guide escapes
/// the trailing comma: `math.smoothing.,` — the workbench form of the `,`
/// variant — is stored as `math.smoothing._comma`. The `.` before it is
/// literal, part of the workbench suffix, not part of the escape. One name
/// also carries a stray leading space, which `index.csv` reproduces exactly,
/// so the book builds and only the word lookup needs it trimmed.
fn word_of(dir: &str) -> String {
    dir.trim().replace("_comma", ",")
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

/// The `#danger[...]` body, unless it is the untouched `_proto` placeholder.
fn danger_of(index_typ: &str) -> Option<String> {
    let start = index_typ.find("#danger[")? + "#danger[".len();
    let rest = &index_typ[start..];
    let end = rest.find("\n]")?;
    let body = rest[..end].trim();
    if body.is_empty() || body.contains("Remove if there is no danger") {
        return None;
    }
    Some(body.split('\n').next().unwrap_or(body).trim().to_string())
}

/// Layers ticked in the "Defined in" checklist, e.g. `- [x] #"rust_multistack"`.
fn defined_in_of(description_typ: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in description_typ.lines() {
        let t = line.trim();
        if !(t.starts_with("- [x]") || t.starts_with("- [X]")) {
            continue;
        }
        if let Some(a) = t.find('"')
            && let Some(b) = t[a + 1..].find('"')
        {
            out.push(t[a + 1..a + 1 + b].to_string());
        }
    }
    out
}

/// The layer a registration path belongs to, in the guide's own vocabulary.
///
/// `classify::subsystem` returns `bund/<x>`, `vm/<x>` or `stack/<x>`; the
/// guide's three checkboxes are exactly those three crates.
fn layer_of_subsystem(sub: &str) -> Option<&'static str> {
    match sub.split('/').next()? {
        "bund" => Some("bund runtime"),
        "vm" => Some("rust_multistackvm"),
        "stack" => Some("rust_multistack"),
        _ => None,
    }
}

/// Directory names `index.csv` renders, verbatim — leading space and all.
fn indexed_dirs(guide: &Path) -> Vec<String> {
    let csv = read(&guide.join("index.csv"));
    csv.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            // The first field, which may be quoted. Names contain no comma
            // except through the `_comma` escape, so a plain split is safe.
            let f = l.split(',').next().unwrap_or(l);
            f.trim_matches('"').to_string()
        })
        .collect()
}

fn load_pages(repo: &Path) -> Result<Vec<Page>, String> {
    let guide = repo.join(GUIDE);
    let lib = guide.join("lib");
    if !lib.is_dir() {
        return Err(format!(
            "no Library Guide at {}. Is the Bund submodule checked out?",
            lib.display()
        ));
    }
    let indexed: BTreeSet<String> = indexed_dirs(&guide).into_iter().collect();

    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&lib)
        .map_err(|e| format!("reading {}: {e}", lib.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();

    Ok(dirs
        .iter()
        .map(|d| {
            let dir = d
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            Page {
                word: word_of(&dir),
                defined_in: defined_in_of(&read(&d.join("description.typ"))),
                danger: danger_of(&read(&d.join("index.typ"))),
                indexed: indexed.contains(&dir),
                dir,
            }
        })
        .collect())
}

/// Programs in the hermetic suite, as raw text, for the danger cross-check.
fn hermetic_sources(repo: &Path) -> Vec<(String, String)> {
    let list = read(&repo.join("tests/golden/HERMETIC.txt"));
    list.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|rel| (rel.to_string(), read(&repo.join(rel))))
        .collect()
}

/// Whether `word` is invoked in `src`, ignoring `//` comments — the guide's
/// hazard names are ordinary English and appear in prose constantly.
fn invokes(src: &str, word: &str) -> bool {
    src.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .any(|code| {
            code.split_whitespace()
                .any(|t| t == word || t.trim_end_matches([';', ')', ']']) == word)
        })
}

pub fn run(_args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let pages = load_pages(&repo)?;
    let reg: Registry = registry::scan(&repo, REGISTRY_ROOTS);
    let registered: BTreeSet<&str> = reg.word_names().into_iter().collect();

    println!("# cargo xtask guide\n");
    println!("Cross-reference of reference/Bund/Documentation/Bund_Library_Guide");
    println!("against the registry. Closes Q17: the guide is read, and the claims");
    println!("it makes that a machine can check are checked here rather than");
    println!("paraphrased into an RFC.\n");

    // -----------------------------------------------------------------------
    println!("## what the guide is\n");
    let indexed = pages.iter().filter(|p| p.indexed).count();
    println!("  word pages under lib/            {:>5}", pages.len());
    println!("  rendered — named by index.csv    {:>5}", indexed);
    println!("  callable names in the registry   {:>5}", registered.len());
    println!(
        "  documented share of the language {:>5}",
        format!("{}%", pages.len() * 100 / registered.len().max(1))
    );
    println!();
    println!("  It is a reference for part of the standard library, not a");
    println!("  language specification: it documents no grammar, no evaluation");
    println!("  order, and no word outside these pages. Adopting it as the sole");
    println!("  normative source would leave most of the language unspecified.\n");

    // -----------------------------------------------------------------------
    println!("## 1. documented but not callable\n");
    let mut uncallable = Vec::new();
    for p in &pages {
        if !registered.contains(p.word.as_str()) {
            uncallable.push(p);
        }
    }
    if uncallable.is_empty() {
        println!("  none — every documented word resolves.\n");
    } else {
        for p in &uncallable {
            let via = reg
                .resolve(&p.word)
                .map(|r| r.label())
                .unwrap_or_else(|| "unregistered".to_string());
            println!("  {:<24}{}", p.word, via);
        }
        println!();
        println!("  A page asserts the word exists. These do not, so the guide");
        println!("  and the implementation disagree about the language's surface.\n");
    }

    // -----------------------------------------------------------------------
    println!("## 2. written but never rendered\n");
    let orphans: Vec<&Page> = pages.iter().filter(|p| !p.indexed).collect();
    if orphans.is_empty() {
        println!("  none — index.csv names every page.\n");
    } else {
        for p in &orphans {
            println!("  {}", p.dir);
        }
        println!();
        println!("  Library.typ renders by iterating index.csv, so a directory");
        println!("  the index does not name never reaches the book. The text");
        println!("  exists; the reader never sees it.\n");
    }

    // -----------------------------------------------------------------------
    println!("## 3. \"Defined in\" against the registration site\n");
    let mut agree = 0usize;
    let mut disagree: Vec<(String, String, String)> = Vec::new();
    let mut unticked: Vec<&str> = Vec::new();
    let mut by_layer: BTreeMap<&str, usize> = BTreeMap::new();
    for p in &pages {
        if p.defined_in.is_empty() {
            unticked.push(&p.dir);
            continue;
        }
        for l in &p.defined_in {
            *by_layer
                .entry(match l.as_str() {
                    "bund runtime" => "bund runtime",
                    "rust_multistackvm" => "rust_multistackvm",
                    "rust_multistack" => "rust_multistack",
                    _ => "other",
                })
                .or_default() += 1;
        }
        let Some(site) = reg.implementing_site(&p.word) else {
            continue;
        };
        let derived = layer_of_subsystem(&classify::subsystem(&site));
        match derived {
            Some(d) if p.defined_in.iter().any(|l| l == d) => agree += 1,
            Some(d) => disagree.push((p.word.clone(), p.defined_in.join(", "), d.to_string())),
            None => {}
        }
    }
    for (l, n) in &by_layer {
        println!("  guide attributes to {l:<20}{n:>4}");
    }
    println!();
    println!("  agree with the registration path    {agree:>4}");
    println!(
        "  disagree                            {:>4}",
        disagree.len()
    );
    println!(
        "  no box ticked                       {:>4}",
        unticked.len()
    );
    if !disagree.is_empty() {
        println!();
        println!("  {:<24}{:<24}path says", "word", "guide says");
        for (w, g, d) in &disagree {
            println!("  {w:<24}{g:<24}{d}");
        }
    }
    if !unticked.is_empty() {
        println!("\n  unticked: {}", unticked.join(", "));
    }
    println!();
    println!("  This is D14's axis with two independent sources. The guide's");
    println!("  three checkboxes are the same three crates classify::subsystem");
    println!("  derives from the path, so they can be compared directly.\n");

    // -----------------------------------------------------------------------
    println!("## 4. #danger pages against the derived effect\n");
    println!("  The guide flags hazards by hand. The effect audit derives them");
    println!("  from the registration path. Comparing the two sorts the guide's");
    println!("  warnings into two kinds it does not itself distinguish.\n");

    let flagged: Vec<&Page> = pages.iter().filter(|p| p.danger.is_some()).collect();
    let mut external = Vec::new();
    let mut data_loss = Vec::new();
    for p in &flagged {
        match classify::Classifier::effect_of(&reg, &p.word) {
            Some((e, _)) if !e.hermetic() => external.push((p, e)),
            Some((e, _)) => data_loss.push((p, e)),
            None => {}
        }
    }
    println!("  pages carrying a real #danger      {:>4}", flagged.len());
    println!("  derived non-hermetic               {:>4}", external.len());
    println!(
        "  derived hermetic                   {:>4}",
        data_loss.len()
    );
    println!();
    println!("  Derived non-hermetic — the audit independently reached the same");
    println!("  verdict the author did, and none can enter the suite:\n");
    for (p, e) in &external {
        println!(
            "  {:<20}{:<12}{}",
            p.word,
            format!("{e:?}"),
            p.danger.as_deref().unwrap_or("")
        );
    }
    println!();
    println!("  Derived hermetic — the warning is real but it is about losing");
    println!("  your own data, not about touching the world. A golden captures");
    println!("  final state, so a program that drops a value reproduces exactly:\n");
    for (p, e) in &data_loss {
        println!(
            "  {:<20}{:<12}{}",
            p.word,
            format!("{e:?}"),
            p.danger.as_deref().unwrap_or("")
        );
    }
    println!();
    println!("  The split matters because conflating them would strip every");
    println!("  program using `drop` or `clear` out of the suite for no reason.");
    println!("  A first cut of this check did exactly that, reporting five");
    println!("  `drop` uses as effect-audit misses.\n");

    // End to end: none of the externally hazardous words may appear in a
    // captured program, whichever way the hazard was established.
    let suite = hermetic_sources(&repo);
    let mut leaks: Vec<(&str, &str)> = Vec::new();
    for (p, _) in &external {
        for (rel, src) in &suite {
            if invokes(src, &p.word) {
                leaks.push((&p.word, rel));
            }
        }
    }
    if leaks.is_empty() {
        println!(
            "  None of the {} externally hazardous words appears in any of the",
            external.len()
        );
        println!(
            "  {} suite programs. Hand-flagged and derived agree end to end.\n",
            suite.len()
        );
    } else {
        for (w, rel) in &leaks {
            println!("  LEAK {w:<20}{rel}");
        }
        println!("\n  A hand-flagged external hazard reached the suite.\n");
    }

    // The reverse direction: hazards the audit finds and the guide does not
    // warn about. Only over documented words — the guide cannot be faulted
    // for not warning about a word it never documents.
    let mut unwarned: Vec<(&str, Effect)> = Vec::new();
    for p in &pages {
        if p.danger.is_some() {
            continue;
        }
        if let Some((e, _)) = classify::Classifier::effect_of(&reg, &p.word)
            && !e.hermetic()
        {
            unwarned.push((&p.word, e));
        }
    }
    println!("## 5. hazardous but unflagged\n");
    println!("  documented words the audit calls non-hermetic where the page");
    println!("  carries no #danger:      {:>4}\n", unwarned.len());
    for (w, e) in &unwarned {
        println!("  {w:<24}{e:?}");
    }
    if !unwarned.is_empty() {
        println!();
        println!("  Each is a hazard a reader of the guide would not be warned");
        println!("  about. They are also why the derived audit, not the guide,");
        println!("  is what the hermetic filter runs on.");
    }
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The escape is `_comma` alone. Reading it as `._comma` would yield
    /// `math.smoothing,`, which is also a registered name — so getting this
    /// wrong produces a plausible word and a silently misfiled page.
    #[test]
    fn the_comma_escape_is_undone() {
        assert_eq!(word_of("math.smoothing._comma"), "math.smoothing.,");
        assert_eq!(word_of("get_comma"), "get,");
    }

    #[test]
    fn a_stray_leading_space_is_trimmed() {
        assert_eq!(word_of(" debug"), "debug");
    }

    #[test]
    fn a_workbench_form_keeps_its_dot() {
        assert_eq!(word_of("bund.eval."), "bund.eval.");
    }

    #[test]
    fn the_proto_placeholder_is_not_a_danger() {
        let s = "#danger[\nRemove if there is no danger\n]\n";
        assert_eq!(danger_of(s), None);
    }

    #[test]
    fn a_real_danger_is_read() {
        let s = "#danger[\nBUND interpreter will be terminated\n]\n";
        assert_eq!(
            danger_of(s).as_deref(),
            Some("BUND interpreter will be terminated")
        );
    }

    #[test]
    fn only_ticked_layers_are_read() {
        let s = "  - [x] #\"rust_multistack\"\n  - [ ] #\"rust_multistackvm\"\n  - [ ] #\"bund runtime\"\n";
        assert_eq!(defined_in_of(s), vec!["rust_multistack".to_string()]);
    }

    #[test]
    fn subsystems_map_onto_the_guides_three_boxes() {
        assert_eq!(layer_of_subsystem("stack/stdlib"), Some("rust_multistack"));
        assert_eq!(layer_of_subsystem("vm/math"), Some("rust_multistackvm"));
        assert_eq!(layer_of_subsystem("bund/values"), Some("bund runtime"));
    }

    /// The hazard names are ordinary English. `use` appears in the prose of
    /// almost every example, and counting that as an invocation would report
    /// a false leak — which it did, before this filter.
    #[test]
    fn a_word_in_a_comment_is_not_an_invocation() {
        assert!(!invokes("// SHowing how to use string matching\n", "use"));
        assert!(invokes("\"lib.bund\" use\n", "use"));
    }
}
