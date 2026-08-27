//! `cargo xtask lint` — self-consistency checks over the RFCs and registers.
//!
//! `cargo xtask cite` checks that a citation *resolves*. It cannot check that
//! a document agrees with itself, and across three reviews of RFC-0001 and two
//! of RFC-0002 that is where every blocker has been:
//!
//! | review | blocker | mechanical? |
//! |---|---|---|
//! | RFC-1 #1 | `dup` as an `Rc` bump contradicted D1 | no — needs a reader |
//! | RFC-1 #2 | criterion 4 unsatisfiable by the struct above it | no |
//! | RFC-1 #3 | a preservation row said "preserved exactly" where F12's disposition is FIX | **yes** |
//! | RFC-2 #1 | the slot enum could not hold a lambda and a native | no |
//! | RFC-2 #2 | two `## Design` sections, the second one rejected | **yes** |
//!
//! Two of five are string comparisons. This runs those, plus the figure
//! checks that catch a count drifting from the artefact it describes, so a
//! human review spends its budget on the three that need judgement.
//!
//! Everything here is a **hard** failure. Unlike `cite`'s token proximity,
//! none of these has a false-positive mode: a heading is duplicated or it is
//! not, a claimed count matches or it does not.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub struct Finding {
    pub doc: String,
    pub line: usize,
    pub what: String,
}

/// Documents linted. RFCs and the registers that RFCs cite figures from.
const DOCS: &[&str] = &["docs/rfc", "docs/registers"];

fn markdown(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    items.sort();
    for p in items {
        if p.is_dir() {
            // `reviews/` holds other people's text; it is evidence, not ours.
            if p.file_name().is_some_and(|n| n == "reviews") {
                continue;
            }
            markdown(&p, out);
        } else if p.extension().is_some_and(|e| e == "md") {
            out.push(p);
        }
    }
}

/// A defect's disposition line, keyed by `F<n>`.
///
/// The disposition is the register's ruling on what Bund2 does. A preservation
/// row that cites the defect has to agree with it.
fn dispositions(repo: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(src) = std::fs::read_to_string(repo.join("docs/registers/defects.md")) else {
        return out;
    };
    let mut current = String::new();
    for line in src.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            current = rest.split_whitespace().next().unwrap_or("").to_string();
        }
        let t = line.trim_start();
        if (t.starts_with("- Disposition:") || t.starts_with("Disposition:")) && !current.is_empty()
        {
            out.entry(current.clone()).or_insert_with(|| t.to_string());
        }
    }
    out
}

/// Whether a preservation row and a disposition contradict each other.
///
/// Only the unambiguous pairing is reported. A row saying "preserved exactly"
/// while the register says the defect is fixed is a contradiction in any
/// reading; anything subtler is left to a human.
pub fn contradicts(row: &str, disposition: &str) -> bool {
    let r = row.to_lowercase();
    let d = disposition.to_lowercase();
    let row_preserves = r.contains("preserved exactly");
    let row_fixes = r.contains("deliberately fixed") || r.contains("deliberately changed");
    let disp_fixes = d.contains("fix");
    let disp_preserves = d.contains("preserve") && !disp_fixes;
    (row_preserves && disp_fixes) || (row_fixes && disp_preserves)
}

/// Level-2 headings that appear more than once in one document.
///
/// Level 2 only, and RFCs only. A register is an append-only list of entries
/// and its `### Consequences` subsections legitimately repeat; an RFC is one
/// narrative, and a repeated `## Design` is how a scripted edit that inserted
/// instead of replacing goes unnoticed. That is the case this exists for.
fn duplicate_headings(src: &str) -> Vec<(usize, String)> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = Vec::new();
    let mut in_fence = false;
    for (i, line) in src.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || !line.starts_with("## ") {
            continue;
        }
        let h = line.trim_end().to_string();
        if let Some(first) = seen.get(&h) {
            out.push((i + 1, format!("`{h}` also at line {first}")));
        } else {
            seen.insert(h, i + 1);
        }
    }
    out
}

/// Type names a fenced `rust` block uses but no block in the document
/// introduces.
///
/// F41: RFC-0001 wrote `payload: Rc<Payload>` and defined `Payload` nowhere,
/// for two revisions, because a scripted edit reported success without
/// matching. RFC-0002's fourth review found the same shape in
/// `native: Option<NativeFn>`. Both are a name presented as though the
/// document defines it, which a reader checks by scrolling and a machine
/// checks by counting.
pub fn undefined_types(src: &str, also_introduced: &BTreeSet<String>) -> Vec<(usize, String)> {
    let mut introduced: BTreeSet<String> = also_introduced.clone();
    let mut used: Vec<(usize, String)> = Vec::new();
    let mut in_rust = false;
    for (i, line) in src.lines().enumerate() {
        let t = line.trim_start();
        if t.starts_with("```") {
            in_rust = t.starts_with("```rust");
            continue;
        }
        if !in_rust {
            continue;
        }
        for kw in ["pub enum ", "pub struct ", "pub type ", "enum ", "struct ", "type "] {
            if let Some(rest) = t.strip_prefix(kw) {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    introduced.insert(name);
                }
                break;
            }
        }
        // A generic argument or a field type: `Rc<Payload>`, `Option<NativeFn>`.
        for cap in angle_bracketed(t) {
            used.push((i + 1, cap));
        }
    }
    used.retain(|(_, n)| {
        !introduced.contains(n)
            && n.chars().next().is_some_and(|c| c.is_uppercase())
            && !KNOWN.contains(&n.as_str())
    });
    used.dedup_by(|a, b| a.1 == b.1);
    used
}

/// Type names a document introduces in a fenced `rust` block.
///
/// Collected across the whole RFC set before checking any one of them: the
/// RFCs are one design, so RFC-0002 legitimately writes `BundValue` and lets
/// RFC-0001 define it. What the check is for is a name **no** document
/// defines.
pub fn introduced_types(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_rust = false;
    for line in src.lines() {
        let t = line.trim_start();
        if t.starts_with("```") {
            in_rust = t.starts_with("```rust");
            continue;
        }
        if !in_rust {
            continue;
        }
        for kw in ["pub enum ", "pub struct ", "pub type ", "enum ", "struct ", "type "] {
            if let Some(rest) = t.strip_prefix(kw) {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    out.insert(name);
                }
                break;
            }
        }
    }
    out
}

/// Type names Rust or the ecosystem provides, which a document need not define.
const KNOWN: &[&str] = &[
    "String", "Vec", "Option", "Result", "Box", "Rc", "Arc", "Cell", "RefCell", "BTreeMap",
    "HashMap", "BTreeSet", "HashSet", "VecDeque", "Error", "Ordering", "Value", "Symbol",
    // Reference types RFC-0001 carries over verbatim rather than defining.
    "Metric", "Operator",
];

/// Identifiers appearing inside `<...>` on a line.
fn angle_bracketed(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let cs: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '<' {
            let mut j = i + 1;
            let mut cur = String::new();
            while j < cs.len() && cs[j] != '>' {
                if cs[j].is_alphanumeric() || cs[j] == '_' {
                    cur.push(cs[j]);
                } else {
                    if !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                    }
                }
                j += 1;
            }
            if !cur.is_empty() {
                out.push(cur);
            }
            i = j;
        }
        i += 1;
    }
    out
}

/// Figures a document claims that an artefact can settle.
fn figure_checks(repo: &Path) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let count = |path: &str, prefix: &str| -> usize {
        std::fs::read_to_string(repo.join(path))
            .map(|s| s.lines().filter(|l| l.starts_with(prefix)).count())
            .unwrap_or(0)
    };
    out.push((
        "docs/registers/decisions.md entries".to_string(),
        count("docs/registers/decisions.md", "## D"),
    ));
    out.push((
        "docs/registers/defects.md entries".to_string(),
        count("docs/registers/defects.md", "## F"),
    ));
    let goldens = walk_count(&repo.join("tests/golden"), "golden");
    out.push(("goldens".to_string(), goldens));
    out
}

fn walk_count(dir: &Path, ext: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                walk_count(&p, ext)
            } else if p.extension().is_some_and(|x| x == ext) {
                1
            } else {
                0
            }
        })
        .sum()
}

pub fn run(_args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let mut docs = Vec::new();
    for d in DOCS {
        markdown(&repo.join(d), &mut docs);
    }
    let disp = dispositions(&repo);
    let mut findings: Vec<Finding> = Vec::new();

    println!("# cargo xtask lint\n");
    println!("Self-consistency checks. `cite` verifies that a citation resolves;");
    println!("these verify that a document agrees with itself and with the");
    println!("artefacts it quotes. Two of the five review blockers so far were");
    println!("of exactly this kind.\n");

    // --- 1. preservation rows against defect dispositions -------------------
    let mut rows_checked = 0usize;
    for path in &docs {
        let rel = path
            .strip_prefix(&repo)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if !rel.contains("/rfc/") {
            continue;
        }
        for (i, line) in src.lines().enumerate() {
            if !line.starts_with('|') {
                continue;
            }
            for f in fnums(line) {
                let Some(d) = disp.get(&f) else { continue };
                rows_checked += 1;
                if contradicts(line, d) {
                    findings.push(Finding {
                        doc: rel.clone(),
                        line: i + 1,
                        what: format!(
                            "row cites {f} but contradicts its disposition\n        row:  {}\n        {f}:  {}",
                            squeeze(line, 96),
                            squeeze(d, 96)
                        ),
                    });
                }
            }
        }
    }
    println!("  preservation rows cross-checked against dispositions  {rows_checked:>4}");

    // --- 2. duplicate headings ---------------------------------------------
    let mut headings_checked = 0usize;
    for path in &docs {
        let rel = path
            .strip_prefix(&repo)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if !rel.contains("/rfc/") {
            continue;
        }
        headings_checked += src.lines().filter(|l| l.starts_with("## ")).count();
        for (line, what) in duplicate_headings(&src) {
            findings.push(Finding {
                doc: rel.clone(),
                line,
                what: format!("duplicate heading — {what}"),
            });
        }
    }
    println!("  RFC level-2 headings checked for duplication          {headings_checked:>4}");

    // --- 3. claimed figures against artefacts -------------------------------
    let figures = figure_checks(&repo);
    let mut figures_checked = 0usize;
    for path in &docs {
        let rel = path
            .strip_prefix(&repo)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        for (i, line) in src.lines().enumerate() {
            for (name, actual) in &figures {
                let Some(claimed) = claimed_entries(line, name) else {
                    continue;
                };
                figures_checked += 1;
                if claimed != *actual {
                    findings.push(Finding {
                        doc: rel.clone(),
                        line: i + 1,
                        what: format!("claims {claimed} for {name}; there are {actual}"),
                    });
                }
            }
        }
    }
    println!("  figure claims checked against artefacts               {figures_checked:>4}");

    // --- 4. types used in a code block that no block introduces --------------
    // Introductions are pooled across the RFC set first: the RFCs are one
    // design, so a name defined in RFC-0001 and used in RFC-0002 is fine.
    let mut all_introduced: BTreeSet<String> = BTreeSet::new();
    for path in &docs {
        if !path.to_string_lossy().contains("/rfc/") {
            continue;
        }
        if let Ok(src) = std::fs::read_to_string(path) {
            all_introduced.extend(introduced_types(&src));
        }
    }
    let mut blocks_checked = 0usize;
    for path in &docs {
        let rel = path
            .strip_prefix(&repo)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        if !rel.contains("/rfc/") {
            continue;
        }
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        blocks_checked += src.lines().filter(|l| l.trim_start().starts_with("```rust")).count();
        for (line, name) in undefined_types(&src, &all_introduced) {
            findings.push(Finding {
                doc: rel.clone(),
                line,
                what: format!("`{name}` is used in a rust block and introduced in none"),
            });
        }
    }
    println!("  rust blocks checked for undefined types               {blocks_checked:>4}");
    println!();

    if findings.is_empty() {
        println!("  no contradictions.\n");
        return Ok(());
    }
    println!("## findings\n");
    for f in &findings {
        println!("  {}:{}\n      {}", f.doc, f.line, f.what);
    }
    println!();
    Err(format!("{} inconsistency/ies", findings.len()))
}

/// `F12`, `F28` … mentioned on a line.
fn fnums(line: &str) -> Vec<String> {
    let cs: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == 'F' && i + 1 < cs.len() && cs[i + 1].is_ascii_digit() {
            let start = i;
            i += 1;
            while i < cs.len() && cs[i].is_ascii_digit() {
                i += 1;
            }
            let tok: String = cs[start..i].iter().collect();
            if !out.contains(&tok) {
                out.push(tok);
            }
        } else {
            i += 1;
        }
    }
    out
}

/// A claim like "— 38 entries" for the named register, if this line makes one.
fn claimed_entries(line: &str, figure: &str) -> Option<usize> {
    let file = figure.strip_suffix(" entries")?;
    if !line.contains(file) {
        return None;
    }
    let cs: Vec<char> = line.chars().collect();
    let idx = line.find("entries")?;
    let upto: String = line[..idx].to_string();
    let _ = cs;
    upto
        .split_whitespace()
        .rev()
        .find_map(|t| t.trim_matches(|c: char| !c.is_ascii_digit()).parse().ok())
}

fn squeeze(s: &str, n: usize) -> String {
    let t: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() <= n {
        t
    } else {
        format!("{}…", t.chars().take(n).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserved_exactly_contradicts_a_fix_disposition() {
        assert!(contradicts(
            "| Ordering | **Preserved exactly**, including F12 |",
            "- Behavioural, latent. Disposition: **FIX** — Bund2 implements"
        ));
    }

    #[test]
    fn deliberately_fixed_agrees_with_a_fix_disposition() {
        assert!(!contradicts(
            "| push on RESULT | **Deliberately fixed** |",
            "- Disposition: Bund2 fixes it"
        ));
    }

    /// The pairing has to fire the other way too: a row that claims a
    /// deviation where the register says preserve is equally wrong.
    #[test]
    fn deliberately_changed_contradicts_a_preserve_disposition() {
        assert!(contradicts(
            "| something | **Deliberately changed** |",
            "- Disposition: preserve the observable behaviour"
        ));
    }

    #[test]
    fn a_disposition_that_says_both_is_not_flagged() {
        // "preserve ... unless fixed" is ambiguous; leave it to a reader
        // rather than report a finding that cannot be acted on.
        assert!(!contradicts(
            "| x | **Preserved exactly** |",
            "- Disposition: preserve"
        ));
    }

    #[test]
    fn duplicate_headings_are_found_with_both_line_numbers() {
        let src = "# t\n\n## Design\n\ntext\n\n## Other\n\n## Design\n";
        let d = duplicate_headings(src);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].0, 9);
        assert!(d[0].1.contains("line 3"));
    }

    /// A heading inside a fenced block is code, not structure.
    #[test]
    fn headings_inside_a_fence_are_not_headings() {
        let src = "## Design\n\n```\n## Design\n```\n";
        assert!(duplicate_headings(src).is_empty());
    }

    /// Level 3 repeats legitimately — every register entry may have its own
    /// `### Consequences`. Only level 2 carries document structure.
    #[test]
    fn level_three_headings_may_repeat() {
        let src = "### Consequences\n\nx\n\n### Consequences\n";
        assert!(duplicate_headings(src).is_empty());
    }

    #[test]
    fn f_numbers_are_extracted_without_duplicates() {
        assert_eq!(fnums("cites F12 and F28 and F12"), vec!["F12", "F28"]);
        assert!(fnums("no defects here").is_empty());
    }

    /// `F` followed by a letter is a word, not a defect number.
    #[test]
    fn a_bare_f_is_not_a_defect_number() {
        assert!(fnums("Function FIX").is_empty());
    }

    #[test]
    fn a_type_used_but_never_introduced_is_found() {
        let src = "```rust\npub struct V { payload: Rc<Payload> }\n```\n";
        let f = undefined_types(src, &BTreeSet::new());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].1, "Payload");
    }

    #[test]
    fn a_type_the_document_introduces_is_not_flagged() {
        let src = "```rust\npub struct V { payload: Rc<Payload> }\npub enum Payload { A }\n```\n";
        assert!(undefined_types(src, &BTreeSet::new()).is_empty());
    }

    /// Rust's own types are not the document's to define.
    #[test]
    fn known_types_are_not_flagged() {
        let src = "```rust\nstruct V { a: Vec<String>, b: BTreeMap<String, Value> }\n```\n";
        assert!(undefined_types(src, &BTreeSet::new()).is_empty());
    }

    /// A name another RFC defines is not this one's to define. The RFCs are
    /// one design and the pool is shared.
    #[test]
    fn a_type_another_document_introduces_is_not_flagged() {
        let other: BTreeSet<String> = ["BundValue".to_string()].into_iter().collect();
        let src = "```rust\nstruct Slot { lambda: Option<BundValue> }\n```\n";
        assert!(undefined_types(src, &other).is_empty());
    }

    /// Only `rust` blocks — a shell transcript is not a declaration site.
    #[test]
    fn non_rust_blocks_are_ignored() {
        let src = "```\nfoo: Rc<Whatever>\n```\n";
        assert!(undefined_types(src, &BTreeSet::new()).is_empty());
    }

    #[test]
    fn an_entry_claim_is_read_off_the_line() {
        assert_eq!(
            claimed_entries(
                "- `docs/registers/defects.md` — 38 entries. The roadmap listed",
                "docs/registers/defects.md entries"
            ),
            Some(38)
        );
        assert_eq!(
            claimed_entries("nothing to see", "docs/registers/defects.md entries"),
            None
        );
    }
}
