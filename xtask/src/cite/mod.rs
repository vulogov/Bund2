//! `cargo xtask cite` — verify every `path:line` citation resolves.
//!
//! RFC-0000 made this an acceptance criterion and then had no way to check it.
//! An adversarial review opened all 43 of its citations by hand and found five
//! defects: a line-number off by one, two counts that had drifted from their
//! source, an "every X" that omitted one, and a defect number attributed to
//! the wrong effect. Every one of those is mechanically detectable.
//!
//! Two checks are hard failures, because they are unambiguous:
//!
//! 1. The cited file exists.
//! 2. The cited line exists — a citation past end-of-file is always wrong.
//!
//! A third is advisory only. Where the prose quotes a token near a citation,
//! the token usually appears near the cited line; when it does not, the
//! citation is worth a second look. It is reported and never fails the run,
//! for two reasons found by trying it.
//!
//! It produces false positives that cannot be tuned away. A line citing a
//! range and quoting two symbols has no way to say which symbol belongs to
//! which line, and `ord.rs:19-21` legitimately cites the arm while the prose
//! quotes `partial_cmp`, defined at line 6.
//!
//! More damning: **it would not have caught the defect that motivated it.**
//! The review found `execute.rs:28` should be `:27`; `STRING` appears on 27,
//! which is inside the window around 28, so this check passes it. A heuristic
//! that misses the case it was built for is a warning, not a gate.
//!
//! Checking that a line *means* what the prose says still needs a reader.

use std::collections::BTreeMap;
use std::path::Path;

/// How far from the cited line a corroborating token may appear. A citation
/// naming a function often points at the signature while the token appears in
/// the body a line or two below.
const NEAR: usize = 3;

#[derive(Debug)]
struct Finding {
    doc: String,
    doc_line: usize,
    citation: String,
    problem: String,
    /// Hard findings fail the run; advisory ones are reported only.
    hard: bool,
}

/// Files whose citations are checked.
const ROOTS: &[&str] = &["docs", "tests/golden", "tests/probes", "CLAUDE.md"];

fn markdown_and_text(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    items.sort();
    for p in items {
        if p.is_dir() {
            markdown_and_text(&p, out);
        } else if p
            .extension()
            .is_some_and(|e| e == "md" || e == "txt" || e == "bund")
        {
            out.push(p);
        }
    }
}

/// Every `reference/...:N` or `reference/...:N,M` citation on a line, with the
/// line numbers it names.
fn citations_in(line: &str) -> Vec<(String, Vec<usize>)> {
    // Char indices throughout. Mixing them with byte slicing panics on the
    // first em dash, and these documents are full of them.
    let cs: Vec<char> = line.chars().collect();
    let pat: Vec<char> = "reference/".chars().collect();
    let starts_at = |i: usize| cs.len() >= i + pat.len() && cs[i..i + pat.len()] == pat;

    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if !starts_at(i) {
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i;
        while j < cs.len()
            && (cs[j].is_alphanumeric() || matches!(cs[j], '/' | '.' | '_' | '-' | '+' | '*' | '!'))
        {
            j += 1;
        }
        let path: String = cs[start..j].iter().collect();
        let path = path.trim_end_matches('.').to_string();

        // Optional `:N`, `:N,M`, `:N-M`.
        let mut lines = Vec::new();
        if j < cs.len() && cs[j] == ':' {
            let mut k = j + 1;
            loop {
                let ds = k;
                while k < cs.len() && cs[k].is_ascii_digit() {
                    k += 1;
                }
                if k == ds {
                    break;
                }
                let n: String = cs[ds..k].iter().collect();
                if let Ok(n) = n.parse::<usize>() {
                    lines.push(n);
                }
                if k < cs.len() && matches!(cs[k], ',' | '-') {
                    k += 1;
                    continue;
                }
                break;
            }
            j = k;
        }
        // A glob in prose ("examples/*.bund") is a description, not a
        // citation, and no file will ever match it.
        if path.contains('.') && !path.contains('*') {
            out.push((path, lines));
        }
        i = j.max(start + 1);
    }
    out
}

/// Tokens too common to corroborate anything. A line quoting `match` or
/// `use` is quoting a word name or prose, and the same characters occur on
/// nearly every line of any Rust file.
const UNCORROBORATING: &[&str] = &[
    "match", "use", "value", "let", "fn", "self", "mut", "if", "else", "for", "return", "type",
    "set", "get", "push", "pull", "call", "run", "new",
];

/// A quoted token on the same documentation line, used to corroborate.
fn quoted_tokens(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(a) = rest.find('`') {
        let after = &rest[a + 1..];
        let Some(b) = after.find('`') else { break };
        let tok = &after[..b];
        // Only tokens that could plausibly appear verbatim in source.
        if !tok.is_empty()
            && tok.len() <= 40
            && !tok.contains(' ')
            && !tok.contains('/')
            && tok.chars().any(|c| c.is_alphanumeric())
            && !UNCORROBORATING.contains(&tok.trim_end_matches(['(', ')', ',', '.']))
        {
            out.push(tok.to_string());
        }
        rest = &after[b + 1..];
    }
    out
}

pub fn run(_args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    let mut docs = Vec::new();
    for root in ROOTS {
        let p = repo.join(root);
        if p.is_dir() {
            markdown_and_text(&p, &mut docs);
        } else if p.is_file() {
            docs.push(p);
        }
    }

    let mut source_cache: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut findings: Vec<Finding> = Vec::new();
    let mut checked = 0usize;
    let mut corroborated = 0usize;

    for doc in &docs {
        // The review file records defects verbatim; checking it would report
        // the very citations it exists to correct.
        if doc.to_string_lossy().contains("/reviews/") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(doc) else {
            continue;
        };
        let doc_rel = doc
            .strip_prefix(&repo)
            .unwrap_or(doc)
            .to_string_lossy()
            .to_string();

        for (n, line) in text.lines().enumerate() {
            let all = citations_in(line);
            // With two citations on a line there is no way to tell which
            // quoted token belongs to which, and pairing them all against all
            // manufactures defects. Existence still gets checked; only
            // corroboration is skipped.
            let single = all.len() == 1;
            for (path, nums) in all {
                checked += 1;
                let abs = repo.join(&path);
                if !abs.is_file() {
                    findings.push(Finding {
                        doc: doc_rel.clone(),
                        doc_line: n + 1,
                        citation: path.clone(),
                        problem: "file does not exist".into(),
                        hard: true,
                    });
                    continue;
                }
                if nums.is_empty() {
                    continue;
                }
                let src = source_cache.entry(path.clone()).or_insert_with(|| {
                    std::fs::read_to_string(&abs)
                        .map(|s| s.lines().map(str::to_string).collect())
                        .unwrap_or_default()
                });
                for num in &nums {
                    if *num == 0 || *num > src.len() {
                        findings.push(Finding {
                            doc: doc_rel.clone(),
                            doc_line: n + 1,
                            citation: format!("{path}:{num}"),
                            problem: format!("line {num} past end of file ({} lines)", src.len()),
                            hard: true,
                        });
                        continue;
                    }
                    // Corroborate with a quoted token where the prose offers one.
                    if !single {
                        continue;
                    }
                    let toks = quoted_tokens(line);
                    if toks.is_empty() {
                        continue;
                    }
                    let lo = num.saturating_sub(NEAR + 1);
                    let hi = (num + NEAR).min(src.len());
                    let window = src[lo..hi].join("\n");
                    let anywhere = src.join("\n");
                    // Only report when the token exists in the file but not
                    // near the cited line: that is a stale line number. A
                    // token absent everywhere is usually prose, not a symbol.
                    if let Some(t) = toks
                        .iter()
                        .find(|t| anywhere.contains(*t) && !window.contains(*t))
                    {
                        findings.push(Finding {
                            doc: doc_rel.clone(),
                            doc_line: n + 1,
                            citation: format!("{path}:{num}"),
                            problem: format!(
                                "`{t}` occurs in the file but not within {NEAR} lines"
                            ),
                            hard: false,
                        });
                    } else {
                        corroborated += 1;
                    }
                }
            }
        }
    }

    println!("# cargo xtask cite\n");
    println!("Every `reference/...:N` citation across docs, registers and the");
    println!("golden lists. Checks that the file exists, the line exists, and —");
    println!("where the prose quotes a token — that the token is near the line");
    println!("it points at. It cannot check that a line means what the prose");
    println!("says; that still needs a reader.\n");

    let (hard, soft): (Vec<&Finding>, Vec<&Finding>) = findings.iter().partition(|f| f.hard);

    println!("  {:<32}{:>6}", "citations checked", checked);
    println!(
        "  {:<32}{:>6}",
        "resolve (file and line)",
        checked - hard.len()
    );
    println!("  {:<32}{:>6}", "corroborated by a token", corroborated);
    println!("  {:<32}{:>6}", "DEFECTS (fail the run)", hard.len());
    println!("  {:<32}{:>6}", "advisories (do not fail)", soft.len());
    println!();

    if !hard.is_empty() {
        println!("## defects\n");
        for f in &hard {
            println!("  {}:{}", f.doc, f.doc_line);
            println!("      {}  —  {}", f.citation, f.problem);
        }
        println!();
    }

    if !soft.is_empty() {
        println!("## advisories\n");
        println!("  Weak signal. A range citation with two quoted symbols cannot");
        println!("  say which belongs to which, and a symbol is often defined far");
        println!("  from the line that uses it. Read them; do not chase them.\n");
        for f in &soft {
            println!("  {}:{}", f.doc, f.doc_line);
            println!("      {}  —  {}", f.citation, f.problem);
        }
        println!();
    }

    if hard.is_empty() {
        println!("  Every citation resolves at the pinned SHAs.\n");
        return Ok(());
    }
    Err(format!("{} citation defect(s)", hard.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_paths_and_line_lists() {
        let c = citations_in("see `reference/a/b.rs:12,34` and reference/c/d.rs");
        assert_eq!(c[0].0, "reference/a/b.rs");
        assert_eq!(c[0].1, vec![12, 34]);
        assert_eq!(c[1].0, "reference/c/d.rs");
        assert!(c[1].1.is_empty());
    }

    #[test]
    fn extracts_ranges() {
        let c = citations_in("`reference/x/y.rs:16-60`");
        assert_eq!(c[0].1, vec![16, 60]);
    }

    #[test]
    fn quoted_tokens_skips_prose_and_paths() {
        let t = quoted_tokens("the `autoadd` flag in `reference/a.rs:19` is `not a symbol`");
        assert!(t.contains(&"autoadd".to_string()));
        assert!(!t.iter().any(|x| x.contains('/')));
        assert!(!t.iter().any(|x| x.contains(' ')));
    }
}
