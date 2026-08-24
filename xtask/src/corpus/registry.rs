//! Scans `reference/` for the names the reference implementation binds, so
//! corpus usage can be cross-referenced against them.
//!
//! Five distinct namespaces exist, and conflating them produces false
//! "unregistered word" reports. Resolution order is grounded in
//! `reference/rust_multistackvm/src/multistackvm_apply.rs:16-60`:
//!
//! 1. commands (`multistackvm_apply.rs:16`)
//! 2. a `$`-prefixed name forces an internal word (`multistackvm_apply.rs:33`)
//! 3. alias resolution (`multistackvm_apply.rs:39`)
//! 4. lambdas, i.e. words the *program* defined with `register`
//!    (`multistackvm_apply.rs:46`)
//! 5. inline words (`multistackvm_apply.rs:59`)
//!
//! Step 5 itself has two tiers. `i_direct` tries the VM's own inline table
//! (`multistackvm_inline.rs:42`) and, on a miss, falls through to the stack
//! layer's inline table (`multistackvm_inline.rs:52`). That second tier is
//! `reference/rust_multistack`, and it is where the core stack words live —
//! `take` is `rust_multistack/src/stdlib/workbench.rs:81`, `drop` is
//! `rust_multistack/src/stdlib/drop.rs:71`. Scanning only Bund and the VM
//! reports those as unregistered, which is wrong.
//!
//! `register_function` (`rust_multistack/src/ts_functions.rs:6`) is a
//! separate table that `i_direct` never consults, so its names are not
//! reachable as words. It is recorded and kept out of the word set.
//!
//! Methods (`register_method`) are another namespace, reached through object
//! dispatch rather than through `apply`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Site {
    /// Repo-relative, e.g. `reference/rust_multistackvm/src/stdlib/math/add.rs`.
    pub path: String,
    pub line: usize,
    /// The handler function the registration names, e.g.
    /// `stdlib_push_list_stack`. Empty when it could not be parsed. Used to
    /// find the implementation and inspect it — see Q11.
    pub handler: String,
}

impl Site {
    pub fn cite(&self) -> String {
        format!("{}:{}", self.path, self.line)
    }
}

#[derive(Debug, Default)]
pub struct Registry {
    /// `register_inline` — the word table.
    pub inline: BTreeMap<String, Vec<Site>>,
    /// `register_alias` — alias name to target name.
    pub alias: BTreeMap<String, (String, Site)>,
    /// `register_method` — OOP method table, reached by object dispatch.
    pub method: BTreeMap<String, Vec<Site>>,
    /// `register_command` — the `:` / `;` handlers.
    pub command: BTreeMap<String, Vec<Site>>,
    /// `register_class` — built-in classes.
    pub class: BTreeMap<String, Vec<Site>>,
    /// `register_var` — built-in variables.
    pub var: BTreeMap<String, Vec<Site>>,
    /// `register_function` in the stack layer. Recorded for completeness;
    /// `i_direct` never consults this table, so these are not words.
    pub function: BTreeMap<String, Vec<Site>>,
}

impl Registry {
    /// Every name reachable as a word from source: inline words plus aliases.
    /// Methods and classes are deliberately excluded — they are not resolved
    /// by `apply`.
    pub fn word_names(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.inline.keys().map(String::as_str).collect();
        v.extend(self.alias.keys().map(String::as_str));
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Where a used name resolves, if anywhere. Follows the order in
    /// `multistackvm_apply.rs:16-60`.
    pub fn resolve(&self, name: &str) -> Option<Resolution> {
        if let Some(sites) = self.command.get(name) {
            return Some(Resolution::Command(sites.clone()));
        }
        if let Some((target, site)) = self.alias.get(name) {
            return Some(Resolution::Alias {
                target: target.clone(),
                site: site.clone(),
            });
        }
        if let Some(sites) = self.inline.get(name) {
            return Some(Resolution::Inline(sites.clone()));
        }
        // `$name` forces the internal word — multistackvm_apply.rs:33.
        if let Some(stripped) = name.strip_prefix('$')
            && let Some(sites) = self.inline.get(stripped)
        {
            return Some(Resolution::Internal(sites.clone()));
        }
        if let Some(sites) = self.method.get(name) {
            return Some(Resolution::Method(sites.clone()));
        }
        if let Some(sites) = self.function.get(name) {
            return Some(Resolution::StackFunction(sites.clone()));
        }
        None
    }

    /// The registration site a word's subsystem should be attributed to.
    /// Aliases are attributed to their target, since that is what implements
    /// the behaviour.
    pub fn implementing_site(&self, name: &str) -> Option<Site> {
        match self.resolve(name)? {
            Resolution::Inline(s)
            | Resolution::Internal(s)
            | Resolution::Command(s)
            | Resolution::Method(s)
            | Resolution::StackFunction(s) => s.last().cloned(),
            Resolution::Alias { target, site } => Some(
                self.inline
                    .get(&target)
                    .and_then(|v| v.last())
                    .cloned()
                    .unwrap_or(site),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Resolution {
    Command(Vec<Site>),
    Alias {
        target: String,
        site: Site,
    },
    Inline(Vec<Site>),
    Internal(Vec<Site>),
    Method(Vec<Site>),
    /// In the stack layer's `functions` table only. `i_direct` does not
    /// consult it, so this name is not callable as a word.
    StackFunction(Vec<Site>),
}

impl Resolution {
    pub fn label(&self) -> String {
        // `register_inline` overwrites (multistackvm_inline.rs:6), so where a
        // name has several sites the last one registered is what runs. Say so
        // rather than quietly citing the first.
        fn cite(sites: &[Site]) -> String {
            match sites.len() {
                0 => "?".to_string(),
                1 => sites[0].cite(),
                n => format!(
                    "{} (+{} earlier site(s), last wins)",
                    sites[n - 1].cite(),
                    n - 1
                ),
            }
        }
        match self {
            Resolution::Command(s) => format!("command ({})", cite(s)),
            Resolution::Alias { target, site } => format!("alias -> `{target}` ({})", site.cite()),
            Resolution::Inline(s) => format!("inline ({})", cite(s)),
            Resolution::Internal(s) => format!("$-forced inline ({})", cite(s)),
            Resolution::Method(s) => format!("method ({})", cite(s)),
            Resolution::StackFunction(s) => {
                format!("stack function table, NOT callable as a word ({})", cite(s))
            }
        }
    }
}

/// Find `needle` in `line`, but only where it starts an identifier.
///
/// Without this, `unregister_inline(` matches the `register_inline(` needle
/// and the `{}_inline` format string on
/// `reference/rust_multistack/src/ts_inline.rs:7` gets recorded as a word.
fn find_call(line: &str, needle: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = line[from..].find(needle) {
        let at = from + rel;
        let preceded_by_ident = at > 0
            && line[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
        if !preceded_by_ident {
            return Some(at);
        }
        from = at + needle.len();
    }
    None
}

/// Extract the `n`th double-quoted string starting at `from` in `line`.
fn quoted(line: &str, from: usize, n: usize) -> Option<String> {
    let mut rest = &line[from..];
    let mut found = 0;
    loop {
        let open = rest.find('"')?;
        let after = &rest[open + 1..];
        let close = after.find('"')?;
        if found == n {
            return Some(after[..close].to_string());
        }
        found += 1;
        rest = &after[close + 1..];
    }
}

/// The `n` lines following line index `idx`, joined. Used to reach the
/// arguments of a registration call that wraps.
fn lines_ahead(src: &str, idx: usize, n: usize) -> Option<String> {
    let joined: Vec<&str> = src.lines().skip(idx + 1).take(n).collect();
    if joined.is_empty() {
        None
    } else {
        Some(joined.join(" "))
    }
}

fn scan_file(path: &Path, rel: &str, reg: &mut Registry) {
    let Ok(src) = std::fs::read_to_string(path) else {
        return;
    };
    for (idx, line) in src.lines().enumerate() {
        let lineno = idx + 1;
        // Skip the definitions of the registration functions themselves.
        if line.contains("pub fn register") {
            continue;
        }
        let site = || Site {
            path: rel.to_string(),
            line: lineno,
            handler: String::new(),
        };
        // The handler is the argument after the name: `..("x".to_string(), f)`.
        let handler_after = |from: usize| -> String {
            let rest = &line[from..];
            let Some(comma) = rest.find(',') else {
                return String::new();
            };
            rest[comma + 1..]
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
                .collect::<String>()
                .trim_end_matches(':')
                .to_string()
        };

        for (needle, target) in [
            ("register_inline(", 0u8),
            ("register_method(", 1),
            ("register_command(", 2),
            ("register_class(", 3),
            ("register_var(", 4),
            ("register_function(", 5),
        ] {
            if let Some(at) = find_call(line, needle) {
                // Some calls wrap, with the name on the following line —
                // e.g. reference/Bund/src/stdlib/functions/ai/mod.rs:57-58.
                let name = quoted(line, at, 0)
                    .or_else(|| lines_ahead(&src, idx, 2).and_then(|ahead| quoted(&ahead, 0, 0)));
                if let Some(name) = name {
                    let mut s = site();
                    s.handler = handler_after(at + needle.len());
                    let bucket = match target {
                        0 => &mut reg.inline,
                        1 => &mut reg.method,
                        2 => &mut reg.command,
                        3 => &mut reg.class,
                        4 => &mut reg.var,
                        _ => &mut reg.function,
                    };
                    bucket.entry(name).or_default().push(s);
                }
            }
        }

        if let Some(at) = find_call(line, "register_alias(") {
            let pair = match (quoted(line, at, 0), quoted(line, at, 1)) {
                (Some(a), Some(b)) => Some((a, b)),
                _ => lines_ahead(&src, idx, 3).and_then(|ahead| {
                    match (quoted(&ahead, 0, 0), quoted(&ahead, 0, 1)) {
                        (Some(a), Some(b)) => Some((a, b)),
                        _ => None,
                    }
                }),
            };
            if let Some((alias, to)) = pair {
                reg.alias.entry(alias).or_insert((to, site()));
            }
        }
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    items.sort();
    for p in items {
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// Scan the two crates that register words. `roots` are repo-relative.
pub fn scan(repo: &Path, roots: &[&str]) -> Registry {
    let mut reg = Registry::default();
    for root in roots {
        let abs = repo.join(root);
        let mut files = Vec::new();
        walk(&abs, &mut files);
        for f in files {
            let rel = f
                .strip_prefix(repo)
                .unwrap_or(&f)
                .to_string_lossy()
                .replace('\\', "/");
            scan_file(&f, &rel, &mut reg);
        }
    }
    reg
}
