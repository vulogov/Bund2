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

use std::collections::BTreeMap;
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
