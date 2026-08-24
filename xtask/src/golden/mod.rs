//! `cargo xtask golden` — capture expected final state from the oracle.
//!
//! For every program in `tests/golden/HERMETIC.txt`, run the oracle and record
//! what it leaves behind. The result is the preservation contract Bund2 is
//! measured against by `cargo xtask conform`.
//!
//! Three things make this more than "run it and save stdout".
//!
//! **Every run is captured twice and refused if it differs.** Reproducibility
//! cannot be decided statically — the sweep that produced
//! `tests/golden/UNSTABLE.txt` found 18 of 77 programs varying between runs
//! while every word they invoke is pure or stdout. Capturing twice turns that
//! one-off finding into a standing guarantee: a golden is only written if the
//! oracle agrees with itself.
//!
//! **Output is normalised before comparison**, per the F14 and F15
//! dispositions. Error text embeds a value's `id` and `stamp`
//! (`reference/Bund/src/stdlib/helpers/eval.rs:33`), and dictionary members
//! print in `HashMap` order (`reference/rust_dynamic/src/value.rs:24`).
//! Neither is behaviour the reference defines, so neither can be a contract.
//! Normalising them is what lets 15 of those 18 programs be captured at all.
//!
//! **Final state is read back deliberately.** The oracle dumps the stack only
//! on error, so a capture epilogue is appended to a *copy* of the program —
//! never to the file in `reference/`, which stays read-only.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Appended to a copy of each program so the oracle reveals its final state.
/// The oracle prints the stack only when a program fails, so a clean run would
/// otherwise record nothing but stdout.
pub(crate) const CAPTURE_EPILOGUE: &str = "\ndebug.display_stack\ndebug.display_workbench\n";

/// A capture that could not be trusted, and why.
struct Refusal {
    program: String,
    reason: String,
}

/// Strip ANSI colour. The oracle emits escape sequences even for plain
/// output, and D15 puts colour out of scope, so a golden full of escapes
/// would pin something Bund2 is not going to reproduce.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI ... final byte in @-~
            if chars.peek() == Some(&'[') {
                chars.next();
                for c2 in chars.by_ref() {
                    if ('@'..='~').contains(&c2) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// F14: a value's identity and stamp differ on every run by construction, so
/// they cannot be part of any contract. Replace them with placeholders.
fn normalise_identity(s: &str) -> String {
    // Index by char throughout. An earlier version mixed char indices with
    // byte slicing (`s[i..]`) and panicked on the first box-drawing character
    // the oracle emits — its output is full of them.
    let cs: Vec<char> = s.chars().collect();
    let id_pat: Vec<char> = "id: \"".chars().collect();
    let stamp_pat: Vec<char> = "stamp: ".chars().collect();
    let at = |i: usize, pat: &[char]| cs.len() >= i + pat.len() && cs[i..i + pat.len()] == *pat;

    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < cs.len() {
        if at(i, &id_pat) {
            out.push_str("id: \"<id>\"");
            i += id_pat.len();
            while i < cs.len() && cs[i] != '"' {
                i += 1;
            }
            if i < cs.len() {
                i += 1; // closing quote
            }
            continue;
        }
        if at(i, &stamp_pat) {
            out.push_str("stamp: <stamp>");
            i += stamp_pat.len();
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
                i += 1;
            }
            continue;
        }
        out.push(cs[i]);
        i += 1;
    }
    out
}

/// F15: dictionary members print in `HashMap` order, which is unspecified.
/// Sort them so a golden pins content rather than an order the reference does
/// not define.
fn normalise_dict_order(line: &str) -> String {
    let (Some(open), Some(close)) = (line.find('{'), line.rfind('}')) else {
        return line.to_string();
    };
    if open >= close {
        return line.to_string();
    }
    let body = &line[open + 1..close];
    if !body.contains("::") {
        return line.to_string();
    }
    let mut parts: Vec<&str> = body
        .split("::")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    parts.sort_unstable();
    format!(
        "{}{{ {} }}{}",
        &line[..open],
        parts.join(" :: "),
        &line[close + 1..]
    )
}

/// Sort the members of a Rust `Debug` map.
///
/// Same defect as the Bund display form (F15), different surface. An OBJECT
/// prints through `Debug` as `Map({"k": Value { .. }, ..})`, and those members
/// come out in `HashMap` order — which is why every OOP program in the suite
/// differed between runs even after the display-form sort.
///
/// The split has to be depth- and string-aware: members contain nested
/// `Value { .. }` braces and `String("a, b")` literals, so a naive split on
/// commas would cut them apart.
fn normalise_debug_maps(s: &str) -> String {
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < cs.len() {
        // `Map({` and `tags: {` both introduce an unordered member list.
        let opens_map =
            cs[i] == '{' && (i >= 4 && cs[i - 4..i].iter().collect::<String>() == "Map(");
        let opens_tags =
            cs[i] == '{' && (i >= 6 && cs[i - 6..i].iter().collect::<String>() == "tags: ");
        if !(opens_map || opens_tags) {
            out.push(cs[i]);
            i += 1;
            continue;
        }

        // Find the matching close brace.
        let mut depth = 0usize;
        let mut j = i;
        let mut in_str = false;
        let mut end = None;
        while j < cs.len() {
            let c = cs[j];
            if in_str {
                if c == '\\' {
                    j += 2;
                    continue;
                }
                if c == '"' {
                    in_str = false;
                }
            } else if c == '"' {
                in_str = true;
            } else if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
                if depth == 0 {
                    end = Some(j);
                    break;
                }
            }
            j += 1;
        }
        let Some(end) = end else {
            out.push(cs[i]);
            i += 1;
            continue;
        };

        // Split the interior at top-level commas.
        let inner: String = cs[i + 1..end].iter().collect();
        let ic: Vec<char> = inner.chars().collect();
        let mut parts: Vec<String> = Vec::new();
        let (mut depth2, mut in_str2, mut start) = (0i32, false, 0usize);
        for (k, &c) in ic.iter().enumerate() {
            if in_str2 {
                if c == '"' {
                    in_str2 = false;
                }
                continue;
            }
            match c {
                '"' => in_str2 = true,
                '{' | '[' | '(' => depth2 += 1,
                '}' | ']' | ')' => depth2 -= 1,
                ',' if depth2 == 0 => {
                    parts.push(ic[start..k].iter().collect());
                    start = k + 1;
                }
                _ => {}
            }
        }
        parts.push(ic[start..].iter().collect());

        let mut parts: Vec<String> = parts
            .into_iter()
            .map(|p| normalise_debug_maps(p.trim()))
            .filter(|p| !p.is_empty())
            .collect();
        parts.sort();

        out.push('{');
        out.push_str(&parts.join(", "));
        out.push('}');
        i = end + 1;
    }
    out
}

pub(crate) fn normalise(raw: &str) -> String {
    let stripped = strip_ansi(raw);
    let ident = normalise_identity(&stripped);
    let maps = normalise_debug_maps(&ident);
    maps.lines()
        .map(normalise_dict_order)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A single oracle run: normalised output and exit status.
pub(crate) struct Run {
    pub(crate) output: String,
    pub(crate) status: i32,
}

const RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

pub(crate) fn run_once(exe: &Path, program_file: &Path, cwd: &Path) -> Result<Run, String> {
    use std::io::Read;

    let mut child = std::process::Command::new(exe)
        .arg("script")
        .arg("--file")
        .arg(program_file)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawning oracle: {e}"))?;

    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) => {
                if start.elapsed() > RUN_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("timed out".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => return Err(format!("waiting on oracle: {e}")),
        }
    };

    let mut raw = String::new();
    if let Some(mut o) = child.stdout.take() {
        let _ = o.read_to_string(&mut raw);
    }
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut raw);
    }
    Ok(Run {
        output: normalise(&raw),
        status: status.code().unwrap_or(-1),
    })
}

/// A golden's path, mirroring the program's own path under `reference/Bund/`.
///
/// Flattening with a separator was tried and rejected: `a/b.bund` and
/// `a__b.bund` both collapse to `a__b.golden`, so one program would silently
/// overwrite the other's golden. Mirroring the tree is injective by
/// construction, and the golden sits where the reader expects it.
pub(crate) fn golden_name(program: &str) -> String {
    let stem = program
        .trim_start_matches("reference/Bund/")
        .trim_end_matches(".bund");
    format!("{stem}.golden")
}

pub(crate) fn read_suite(repo: &Path) -> Result<Vec<String>, String> {
    let path = repo.join("tests/golden/HERMETIC.txt");
    let src = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "reading {}: {e}. Run `cargo xtask corpus` first.",
            path.display()
        )
    })?;
    Ok(src
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect())
}

pub fn run(args: &[String]) -> Result<(), String> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("cannot locate repository root")?
        .to_path_buf();

    // `--accept <name> --reason <ref>` regenerates one golden deliberately.
    // CLAUDE.md treats tests/golden as read-only otherwise: quietly
    // regenerating a golden because it is easier is not an option, so an
    // existing golden that would change is refused unless it is named here.
    let mut accept: Option<String> = None;
    let mut reason: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--accept" => accept = it.next().cloned(),
            "--reason" => reason = it.next().cloned(),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    if accept.is_some() != reason.is_some() {
        return Err(
            "--accept and --reason must be given together: a regenerated \
                    golden records why it changed"
                .into(),
        );
    }

    let oracle = repo.join("target/oracle/release/bund");
    if !oracle.is_file() {
        return Err(format!(
            "no oracle at {}.\n  Build it out-of-tree so the submodule stays clean:\n    \
             cargo build --release --manifest-path reference/Bund/Cargo.toml \\\n                \
             --target-dir target/oracle",
            oracle.strip_prefix(&repo).unwrap_or(&oracle).display()
        ));
    }

    let suite = read_suite(&repo)?;
    let golden_dir = repo.join("tests/golden");
    let work = repo.join("target/golden-capture");
    std::fs::create_dir_all(&work).map_err(|e| format!("creating {}: {e}", work.display()))?;

    println!("# cargo xtask golden\n");
    println!("Capturing {} programs from the oracle.\n", suite.len());
    println!("Each is run twice and refused if the two runs differ — see");
    println!("tests/golden/UNSTABLE.txt for why that check exists. Output is");
    println!("normalised for F14 (id/stamp in error text) and F15 (dict member");
    println!("order) before comparison; neither is behaviour the reference");
    println!("defines, so neither can be a contract.\n");

    let mut written = 0usize;
    let mut unchanged = 0usize;
    let mut refused: Vec<Refusal> = Vec::new();
    let mut needs_accept: Vec<String> = Vec::new();

    for program in &suite {
        let src_path = repo.join(program);
        let Ok(src) = std::fs::read_to_string(&src_path) else {
            refused.push(Refusal {
                program: program.clone(),
                reason: "could not read source".into(),
            });
            continue;
        };

        // Capture epilogue goes on a copy. `reference/` is never written to.
        let capture_file = work.join("capture.bund");
        if let Err(e) = std::fs::write(&capture_file, format!("{src}{CAPTURE_EPILOGUE}")) {
            return Err(format!("writing capture copy: {e}"));
        }

        // Programs are written to run from the reference root.
        let cwd = repo.join("reference/Bund");
        let first = match run_once(&oracle, &capture_file, &cwd) {
            Ok(r) => r,
            Err(e) => {
                refused.push(Refusal {
                    program: program.clone(),
                    reason: e,
                });
                continue;
            }
        };
        let second = match run_once(&oracle, &capture_file, &cwd) {
            Ok(r) => r,
            Err(e) => {
                refused.push(Refusal {
                    program: program.clone(),
                    reason: e,
                });
                continue;
            }
        };

        if first.output != second.output || first.status != second.status {
            refused.push(Refusal {
                program: program.clone(),
                reason: "two runs disagree after normalisation — not reproducible".into(),
            });
            continue;
        }

        let name = golden_name(program);
        let body = format!(
            "# golden for {program}\n\
             # captured by `cargo xtask golden` from reference/Bund via target/oracle\n\
             # normalised: id and stamp (F14), dict member order (F15), ANSI stripped\n\
             # verified: two oracle runs produced identical output\n\
             {}\
             ## exit\n{}\n## output\n{}\n",
            reason
                .as_ref()
                .filter(|_| accept.as_deref() == Some(name.as_str()))
                .map(|r| format!("# regenerated with --accept, reason: {r}\n"))
                .unwrap_or_default(),
            first.status,
            first.output
        );

        let dest = golden_dir.join(&name);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("creating {}: {e}", parent.display()))?;
        }
        match std::fs::read_to_string(&dest) {
            Ok(existing) if existing == body => {
                unchanged += 1;
            }
            Ok(_) => {
                // An existing golden would change. That is a conformance
                // event, not a capture detail.
                if accept.as_deref() == Some(name.as_str()) {
                    std::fs::write(&dest, &body)
                        .map_err(|e| format!("writing {}: {e}", dest.display()))?;
                    written += 1;
                } else {
                    needs_accept.push(name);
                }
            }
            Err(_) => {
                std::fs::write(&dest, &body)
                    .map_err(|e| format!("writing {}: {e}", dest.display()))?;
                written += 1;
            }
        }
    }

    println!("  {:<34}{:>5}", "written", written);
    println!("  {:<34}{:>5}", "already current", unchanged);
    println!("  {:<34}{:>5}", "refused (not reproducible)", refused.len());
    println!(
        "  {:<34}{:>5}",
        "changed, needs --accept",
        needs_accept.len()
    );
    println!();

    if !refused.is_empty() {
        println!("## refused\n");
        println!("  No golden was written for these. A golden that cannot be");
        println!("  reproduced is worse than none: it fails forever and teaches");
        println!("  the reader to ignore failures.\n");
        for r in &refused {
            println!("  {:<62} {}", r.program, r.reason);
        }
        println!();
    }

    if !needs_accept.is_empty() {
        println!("## existing golden would change\n");
        println!("  These were left alone. If the change is intended, name the");
        println!("  golden and say why:\n");
        println!("    cargo xtask golden --accept <name> --reason <F-number or decision>\n");
        for n in &needs_accept {
            println!("  {n}");
        }
        println!();
    }

    // What the capture cannot see, said plainly rather than implied.
    let named_stack_programs: BTreeSet<&str> = suite
        .iter()
        .filter(|p| {
            std::fs::read_to_string(repo.join(p))
                .map(|s| {
                    s.lines()
                        .filter(|l| !l.trim_start().starts_with("//"))
                        .any(|l| l.split_whitespace().any(|w| w.starts_with('@')))
                })
                .unwrap_or(false)
        })
        .map(String::as_str)
        .collect();
    if !named_stack_programs.is_empty() {
        println!("## limitation: named stacks are not captured\n");
        println!("  The golden README promises every named stack's contents. The");
        println!("  capture epilogue records the current stack and the workbench");
        println!("  (`debug.display_stack`, `debug.display_workbench`), because");
        println!("  the reference exposes no word that enumerates stacks. These");
        println!(
            "  {} suite programs use `@named` stacks, so their goldens pin",
            named_stack_programs.len()
        );
        println!("  less than the README claims:\n");
        for p in &named_stack_programs {
            println!("  {p}");
        }
        println!();
    }

    Ok(())
}

/// Paths written, for the caller's benefit in tests.
#[allow(dead_code)]
pub fn golden_path_for(repo: &Path, program: &str) -> PathBuf {
    repo.join("tests/golden").join(golden_name(program))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ansi_escapes() {
        assert_eq!(strip_ansi("\u{1b}[33m[\u{1b}[0mBUND"), "[BUND");
    }

    #[test]
    fn normalises_id_and_stamp() {
        let s = "Value { id: \"aVKGewYRnkMPGLGTGJLfh\", stamp: 1787605792237.0, dt: 2 }";
        assert_eq!(
            normalise_identity(s),
            "Value { id: \"<id>\", stamp: <stamp>, dt: 2 }"
        );
    }

    #[test]
    fn survives_non_ascii_output() {
        // The oracle draws tables with box characters; an earlier version
        // panicked slicing a str by char index.
        let s = "╭───╮\n│ Value { id: \"abc\", stamp: 12.0 } │\n╰───╯";
        let out = normalise_identity(s);
        assert!(out.contains("id: \"<id>\""));
        assert!(out.contains("stamp: <stamp>"));
        assert!(out.contains('╭'));
    }

    #[test]
    fn sorts_dict_members() {
        // The two orders reference/Bund/examples/configuration_create.bund
        // alternates between must normalise to one string.
        let a = "{ type=simple ::  N=100 ::  X=0.0 ::  Step=0.1 :: }";
        let b = "{ X=0.0 ::  N=100 ::  type=simple ::  Step=0.1 :: }";
        assert_eq!(normalise_dict_order(a), normalise_dict_order(b));
    }

    #[test]
    fn sorts_debug_map_members() {
        // The two orders create_object.bund alternates between.
        let a = r#"data: Map({".super": Value { a: 1 }, "hello": Value { b: 2 }})"#;
        let b = r#"data: Map({"hello": Value { b: 2 }, ".super": Value { a: 1 }})"#;
        assert_eq!(normalise_debug_maps(a), normalise_debug_maps(b));
    }

    #[test]
    fn debug_map_split_respects_strings_and_nesting() {
        // A comma inside a string literal must not split a member.
        let s = r#"tags: {"k": String("a, b"), "j": String("c")}"#;
        let out = normalise_debug_maps(s);
        assert!(out.contains(r#"String("a, b")"#), "{out}");
    }

    #[test]
    fn leaves_non_dict_braces_alone() {
        let s = "no members here { }";
        assert_eq!(normalise_dict_order(s), s);
    }

    #[test]
    fn golden_paths_mirror_the_program_and_never_collide() {
        assert_eq!(
            golden_name("reference/Bund/examples/code_snippets/x.bund"),
            "examples/code_snippets/x.golden"
        );
        // The flattening this replaced mapped both of these to one file.
        assert_ne!(
            golden_name("reference/Bund/examples/a/b.bund"),
            golden_name("reference/Bund/examples/a__b.bund")
        );
    }
}

/// Split a golden into its recorded exit status and output.
///
/// Kept beside the writer so the two cannot drift: `conform` reads goldens
/// through this, and any change to the format is a change to one file.
pub(crate) fn parse_golden(body: &str) -> Option<(i32, String)> {
    let out_at = body.find("\n## output\n")?;
    let exit_at = body.find("\n## exit\n")?;
    let status: i32 = body[exit_at + "\n## exit\n".len()..out_at]
        .trim()
        .parse()
        .ok()?;
    let output = body[out_at + "\n## output\n".len()..]
        .strip_suffix('\n')
        .unwrap_or(&body[out_at + "\n## output\n".len()..])
        .to_string();
    Some((status, output))
}
