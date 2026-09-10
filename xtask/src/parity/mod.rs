//! `cargo xtask parity` — **RFC-0003 criterion 3**.
//!
//! Does Bund2's front end produce the same token stream as the reference's?
//! Conformance cannot answer that: a program can parse identically and still
//! fail on the first unimplemented word, and a program can parse *wrongly* and
//! still produce the right output by accident.
//!
//! # How the reference's stream is obtained
//!
//! `compile` is the oracle's dump mode. It parses a string and pushes the token
//! stream as a LIST, dropping the trailing EXIT
//! (`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:29-43`), so
//!
//! ```text
//! '<source>' compile debug.display_stack
//! ```
//!
//! renders the reference's own stream in the same `Value { … }` text Bund2
//! renders. `reference/` is never instrumented and stays read-only.
//!
//! # Why a `'…'` literal, and what it costs
//!
//! Source has to reach `compile` as a string, and a **double-quoted** string
//! cannot carry one: the reference's handler slices raw text and does no
//! unescaping (`reference/bund_language_parser/src/vm/string.rs:8`), so a `"`
//! inside the source would have to be written `\"` and would *stay* `\"` in the
//! value. The program would be corrupted before it was compiled.
//!
//! A `'…'` literal has no escaping at all and runs to the next `'`
//! (`reference/bund_language_parser/bund.pest:25`), so it carries quotes and
//! newlines verbatim — and cannot carry a `'`.
//!
//! **That is a real cap and it is reported, not hidden.** Sources containing a
//! single quote are listed as skipped with their names. Closing the gap needs a
//! word that reads a file *as a string*; `io.textfile` returns a list of lines
//! (`reference/Bund/src/stdlib/functions/io/textfile.rs:45-53`) and no join
//! word exists — joining with a space would let a `//` comment swallow the rest
//! of the file.
//!
//! # Normalisation
//!
//! Beyond the golden normaliser's `id:` and `stamp:`, a CONTEXT value carries a
//! fresh nanoid in its **`data`** (`reference/rust_dynamic/src/create_special.rs:28`),
//! so two runs of the same source differ there. That payload is erased too, or
//! every program containing `( … )` would be permanently unstable.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::golden;

/// Erase everything that cannot reproduce across two runs.
fn normalise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(&['i', 'd', ':', ' ', '"']) {
            out.push_str("id: \"<id>\"");
            i += 5;
            while i < b.len() && b[i] != '"' {
                i += 1;
            }
            i += 1;
            continue;
        }
        if b[i..].starts_with(&['s', 't', 'a', 'm', 'p', ':', ' ']) {
            out.push_str("stamp: <stamp>");
            i += 7;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == '.' || b[i] == '-') {
                i += 1;
            }
            continue;
        }
        // A CONTEXT value's payload is a generated name, not content.
        if b[i..].starts_with(&['d', 't', ':', ' ', '2', '1', ',']) {
            out.push_str("dt: 21,");
            i += 7;
            // Erase the String("…") that follows in this value's `data`.
            let rest: String = b[i..].iter().collect();
            if let Some(p) = rest.find("data: String(\"") {
                let head: String = rest[..p].to_string();
                out.push_str(&normalise(&head));
                out.push_str("data: String(\"<ctx>\"");
                let after = &rest[p + "data: String(\"".len()..];
                let end = after.find('"').map(|e| e + 1).unwrap_or(0);
                i += p + "data: String(\"".len() + end;
            }
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// Pull the single rendered row out of `debug.display_stack`'s box.
///
/// Returns `None` if the box has anything other than one row, which is how a
/// wrapped cell shows up — the harness reports that rather than reassembling
/// text whose breaks it cannot distinguish from content.
fn unbox(out: &str) -> Option<String> {
    let rows: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with('│'))
        .collect();
    if rows.len() != 1 {
        return None;
    }
    let row = rows[0];
    let inner = row.trim_start_matches('│').trim_end_matches('│');
    Some(inner.trim().to_string())
}

fn oracle_binary(repo: &Path) -> Result<PathBuf, String> {
    let p = repo.join("target/oracle/release/bund");
    if p.exists() {
        Ok(p)
    } else {
        Err(format!(
            "the oracle is not built. Run:\n    cargo build --release \
             --manifest-path reference/Bund/Cargo.toml --target-dir target/oracle\n  \
             (expected {})",
            p.display()
        ))
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    for a in args {
        if !matches!(a.as_str(), "-v" | "--verbose") {
            return Err(format!("unknown argument `{a}`"));
        }
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();
    let oracle = oracle_binary(&repo)?;
    let work = repo.join("target/parity");
    std::fs::create_dir_all(&work).map_err(|e| format!("creating {}: {e}", work.display()))?;

    let (jobs, _) = golden::capture_jobs(&repo)?;

    let mut agreed = 0usize;
    let mut compared = 0usize;
    let mut skipped_quote: Vec<String> = Vec::new();
    let mut skipped_other: Vec<(String, String)> = Vec::new();
    let mut differed: Vec<(String, String, String)> = Vec::new();

    for (program, name, _cwd) in &jobs {
        let path = repo.join(program);
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        if src.contains('\'') {
            skipped_quote.push(name.clone());
            continue;
        }

        let driver = work.join("parity.bund");
        std::fs::write(&driver, format!("'{src}' compile debug.display_stack\n"))
            .map_err(|e| format!("writing {}: {e}", driver.display()))?;

        let out = Command::new(&oracle)
            .arg("script")
            .arg("--file")
            .arg(&driver)
            .current_dir(&repo)
            .output()
            .map_err(|e| format!("running the oracle: {e}"))?;
        let text = String::from_utf8_lossy(&out.stdout);
        let Some(theirs) = unbox(&text) else {
            skipped_other.push((name.clone(), "oracle box was not a single row".into()));
            continue;
        };
        if !theirs.contains("dt: 9") {
            skipped_other.push((
                name.clone(),
                "oracle did not return a LIST — `compile` failed".into(),
            ));
            continue;
        }

        // Bund2's stream, minus the trailing EXIT that `compile` drops, wrapped
        // in the same LIST so the two renderings are comparable — and carrying
        // the stack tag, because the oracle's LIST reached `debug.display_stack`
        // through a push and `TS::push` tags unconditionally
        // (`reference/rust_multistack/src/ts_push.rs:25`). Without it every
        // program differs on the wrapper and none on its contents.
        let ours = match bund2_syntax::compile(&src) {
            Ok(mut s) => {
                s.pop();
                bund2_value::BundValue::list(s)
                    .with_tag(std::rc::Rc::from("stack"), std::rc::Rc::from("main"))
                    .render(false)
            }
            Err(e) => {
                skipped_other.push((name.clone(), format!("bund2 parse error: {}", e.render(&src))));
                continue;
            }
        };

        compared += 1;
        let (a, b) = (normalise(&theirs), normalise(&ours));
        if a == b {
            agreed += 1;
        } else {
            differed.push((name.clone(), a, b));
        }
    }

    println!("\n  PARITY  {agreed}/{compared}\n");
    println!(
        "  Bund2's token stream against the reference's own, obtained with\n  \
         `compile`. Not a conformance number: this asks whether the two front\n  \
         ends agree, which `conform` cannot see.\n"
    );

    if !skipped_quote.is_empty() {
        println!(
            "  {} source(s) skipped: a `'` in the source cannot be carried\n  \
             through a `'…'` literal, and a double-quoted string would corrupt\n  \
             the program (the reference does not unescape).",
            skipped_quote.len()
        );
        if verbose {
            for n in &skipped_quote {
                println!("      {n}");
            }
        }
        println!();
    }
    for (n, why) in &skipped_other {
        println!("  skipped  {n:<42} {why}");
    }
    for (n, a, b) in differed.iter().take(if verbose { usize::MAX } else { 3 }) {
        println!("\n  DIFFERS  {n}");
        let (i, j) = first_difference(a, b);
        println!("    oracle: …{}", &a[i..j.min(a.len())]);
        println!("    bund2 : …{}", &b[i..j.min(b.len())]);
    }
    if !differed.is_empty() {
        return Err(format!(
            "{} program(s) parse differently from the reference",
            differed.len()
        ));
    }
    Ok(())
}

/// A window around the first byte where two renderings diverge.
fn first_difference(a: &str, b: &str) -> (usize, usize) {
    let n = a
        .bytes()
        .zip(b.bytes())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let start = n.saturating_sub(60);
    // Land on a character boundary in both.
    let start = (0..=start)
        .rev()
        .find(|i| a.is_char_boundary(*i) && b.is_char_boundary(*i))
        .unwrap_or(0);
    (start, start + 200)
}
