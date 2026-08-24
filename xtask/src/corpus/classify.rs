//! Subsystem attribution and effect classification.
//!
//! Both are derived from the *registration site* of a word, not from its
//! name. A name is a guess; a path is evidence. Every rule below carries the
//! `path:line` that justifies it.

use crate::corpus::registry::{Registry, Site};

/// The implementing subsystem, as `crate/dir`. Derived from the registration
/// site's path: `Bund/src/stdlib/functions/<X>/...` -> `bund/<X>`,
/// `rust_multistackvm/src/stdlib/<X>...` -> `vm/<X>`.
pub fn subsystem(site: &Site) -> String {
    let p = site.path.as_str();
    if let Some(rest) = p.strip_prefix("reference/Bund/src/stdlib/functions/") {
        return format!("bund/{}", head(rest));
    }
    if let Some(rest) = p.strip_prefix("reference/Bund/src/stdlib/") {
        return format!("bund/{}", head(rest));
    }
    if let Some(rest) = p.strip_prefix("reference/rust_multistackvm/src/stdlib/") {
        return format!("vm/{}", head(rest));
    }
    if let Some(rest) = p.strip_prefix("reference/rust_multistackvm/src/") {
        return format!("vm/{}", head(rest));
    }
    // The stack layer. Everything here is stack manipulation, and it is the
    // tier `i_direct` falls through to — multistackvm_inline.rs:52.
    if let Some(rest) = p.strip_prefix("reference/rust_multistack/src/stdlib/") {
        return format!("stack/{}", head(rest));
    }
    if let Some(rest) = p.strip_prefix("reference/rust_multistack/src/") {
        return format!("stack/{}", head(rest));
    }
    p.to_string()
}

/// First path component, with any `.rs` extension dropped.
fn head(rest: &str) -> &str {
    let first = rest.split('/').next().unwrap_or(rest);
    first.strip_suffix(".rs").unwrap_or(first)
}

/// What a word does that a golden must be able to reproduce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Effect {
    /// Stack-to-stack. Reproducible.
    Pure,
    /// Writes stdout. Reproducible; goldens capture stdout.
    Stdout,
    /// Writes stderr through `log`. Not captured by goldens.
    Diagnostic,
    Clock,
    Random,
    /// A fresh nanoid per value.
    OpaqueId,
    Filesystem,
    Network,
    Bus,
    Image,
    Database,
    Process,
    /// Depends on the host: hostname, memory, argv, virtualization.
    Host,
    /// Reads stdin.
    Interactive,
}

impl Effect {
    pub fn hermetic(self) -> bool {
        matches!(self, Effect::Pure | Effect::Stdout | Effect::Diagnostic)
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Effect::Pure => "pure",
            Effect::Stdout => "stdout",
            Effect::Diagnostic => "diagnostic",
            Effect::Clock => "clock",
            Effect::Random => "random",
            Effect::OpaqueId => "opaque-id",
            Effect::Filesystem => "filesystem",
            Effect::Network => "network",
            Effect::Bus => "bus",
            Effect::Image => "image",
            Effect::Database => "database",
            Effect::Process => "process",
            Effect::Host => "host",
            Effect::Interactive => "stdin",
        }
    }
}

/// Per-word overrides, applied before the path rules. Each entry is a word
/// whose effect differs from its file's.
const WORD_OVERRIDES: &[(&str, Effect, &str)] = &[
    // `console.text.randomcolor` / `.rainbow` / `.matrix` pick a colour at
    // random — reference/Bund/src/stdlib/functions/console/spinner.rs:278-279
    // (`random_pleasing_color`), :310.
    (
        "console.text.randomcolor",
        Effect::Random,
        "console/spinner.rs:310",
    ),
    (
        "console.text.rainbow",
        Effect::Random,
        "console/spinner.rs:310",
    ),
    (
        "console.text.matrix",
        Effect::Random,
        "console/spinner.rs:310",
    ),
    (
        "spinner.text.randomcolor",
        Effect::Random,
        "console/spinner.rs:278",
    ),
    // `.id` is a fresh nanoid per value —
    // reference/rust_dynamic/src/id.rs:6, surfaced by
    // reference/Bund/src/stdlib/functions/oop/base_classes.rs:11-18.
    (".id", Effect::OpaqueId, "rust_dynamic/src/id.rs:6"),
    // `.timestamp` reads the clock —
    // reference/Bund/src/stdlib/functions/oop/base_classes.rs:20-27.
    (".timestamp", Effect::Clock, "oop/base_classes.rs:25"),
];

/// Path rules, longest prefix first. `reference/` prefix implied.
const PATH_RULES: &[(&str, Effect)] = &[
    // -- bund/io is mixed --------------------------------------------------
    // `rasciigraph`, an ASCII plot to stdout — io/graph.rs:7.
    ("Bund/src/stdlib/functions/io/graph.rs", Effect::Stdout),
    // figlet banner to stdout.
    ("Bund/src/stdlib/functions/io/banner.rs", Effect::Stdout),
    // `rustyline` / `yapp` read stdin — io/input.rs:11-12.
    ("Bund/src/stdlib/functions/io/input.rs", Effect::Interactive),
    (
        "Bund/src/stdlib/functions/io/textfile.rs",
        Effect::Filesystem,
    ),
    // -- bund/debug_fun is mixed -------------------------------------------
    (
        "Bund/src/stdlib/functions/debug_fun/debug_display_hostinfo.rs",
        Effect::Host,
    ),
    (
        "Bund/src/stdlib/functions/debug_fun/debug_display_memstats.rs",
        Effect::Host,
    ),
    (
        "Bund/src/stdlib/functions/debug_fun/debug_display_distributed_info.rs",
        Effect::Network,
    ),
    (
        "Bund/src/stdlib/functions/debug_fun/debug_shell.rs",
        Effect::Process,
    ),
    // `log::*` goes to stderr via env_logger — Bund/src/cmd/setloglevel.rs:10.
    (
        "Bund/src/stdlib/functions/debug_fun/debug_trace.rs",
        Effect::Diagnostic,
    ),
    ("Bund/src/stdlib/functions/debug_fun", Effect::Stdout),
    // -- bund/bund is mixed ------------------------------------------------
    (
        "Bund/src/stdlib/functions/bund/bund_load.rs",
        Effect::Filesystem,
    ),
    (
        "Bund/src/stdlib/functions/bund/bund_save.rs",
        Effect::Filesystem,
    ),
    (
        "Bund/src/stdlib/functions/bund/bund_models.rs",
        Effect::Filesystem,
    ),
    (
        "Bund/src/stdlib/functions/bund/bund_world_bootstrap.rs",
        Effect::Filesystem,
    ),
    // `use` fetches its source over HTTP with curl, not from disk —
    // reference/Bund/src/stdlib/helpers/file_helper.rs:42-54. Corrected while
    // resolving D8; it had been classified filesystem.
    (
        "Bund/src/stdlib/functions/bund/bund_use.rs",
        Effect::Network,
    ),
    ("Bund/src/stdlib/functions/bund/bund_args.rs", Effect::Host),
    // `bund.eval` / `bund.eval-file`: eval of a string is pure; the -file
    // variants read the filesystem. bund_eval.rs:118-126.
    ("Bund/src/stdlib/functions/bund/bund_eval.rs", Effect::Pure),
    ("Bund/src/stdlib/functions/bund", Effect::Pure),
    // -- bund/math is mixed ------------------------------------------------
    // `rand_mt` / `fastrand` / `rand_chacha` — math/rand.rs:7-10.
    ("Bund/src/stdlib/functions/math/rand.rs", Effect::Random),
    ("Bund/src/stdlib/functions/math", Effect::Pure),
    // -- bund/console ------------------------------------------------------
    // The spinner is an animation driven off wall-clock — console/spinner.rs:11.
    (
        "Bund/src/stdlib/functions/console/spinner.rs",
        Effect::Stdout,
    ),
    ("Bund/src/stdlib/functions/console", Effect::Stdout),
    // -- wholly effectful subsystems ---------------------------------------
    ("Bund/src/stdlib/functions/filesystem", Effect::Filesystem),
    ("Bund/src/stdlib/functions/bus", Effect::Bus),
    ("Bund/src/stdlib/functions/image", Effect::Image),
    ("Bund/src/stdlib/functions/internaldb", Effect::Database),
    ("Bund/src/stdlib/functions/ai", Effect::Network),
    ("Bund/src/stdlib/functions/sysinfo", Effect::Host),
    // -- bund/system is mixed, and its directory is a poor proxy -----------
    // `display` renders markdown to the terminal with `termimad::print_text`
    // — system/display.rs:11. It is a printing word filed under `system/`,
    // and 2 otherwise-basic programs depend on it.
    (
        "Bund/src/stdlib/functions/system/display.rs",
        Effect::Stdout,
    ),
    // Pure string surgery on a path — system/unixpath.rs has no io imports.
    ("Bund/src/stdlib/functions/system/unixpath.rs", Effect::Pure),
    ("Bund/src/stdlib/functions/system/sleep.rs", Effect::Clock),
    ("Bund/src/stdlib/functions/system/ip.rs", Effect::Network),
    ("Bund/src/stdlib/functions/system/locale.rs", Effect::Host),
    (
        "Bund/src/stdlib/functions/system/proctitle.rs",
        Effect::Process,
    ),
    ("Bund/src/stdlib/functions/system", Effect::Process),
    // `rand::thread_rng` — generators/uniform.rs:49.
    ("Bund/src/stdlib/functions/generators", Effect::Random),
    // -- bund/string is mixed ----------------------------------------------
    // `string.random.*` is `rand::thread_rng` plus `passwords` —
    // string/random.rs:7-8. Found by the Q4 effect audit, not by hand: the
    // subsystem default had these as pure, which wrongly admitted
    // generate_100_random_first_names and generate_25_loorem_ipsum_strings to
    // the golden suite.
    ("Bund/src/stdlib/functions/string/random.rs", Effect::Random),
    // `bund.exit` calls std::process::exit — bund/bund_exit.rs. The exit
    // status is deterministic and goldens record it, so this stays hermetic;
    // it is named here so the classification is deliberate rather than a
    // fall-through.
    ("Bund/src/stdlib/functions/bund/bund_exit.rs", Effect::Pure),
    // -- vm ----------------------------------------------------------------
    ("rust_multistackvm/src/stdlib/time", Effect::Clock),
    ("rust_multistackvm/src/stdlib/print", Effect::Stdout),
];

/// Crates and std paths whose presence in a file's `use` lines implies an
/// effect. Used to audit [`PATH_RULES`] against what the code actually pulls
/// in, so the classification is checkable rather than asserted.
///
/// These are indicators, not verdicts: a file may import `rand` and use it in
/// one function out of ten, and a path rule that disagrees may still be
/// right. The audit reports; a human decides.
pub const EFFECT_MARKERS: &[(&str, Effect)] = &[
    ("std::fs", Effect::Filesystem),
    ("walkdir", Effect::Filesystem),
    ("glob", Effect::Filesystem),
    ("reqwest", Effect::Network),
    ("ureq", Effect::Network),
    ("std::net", Effect::Network),
    ("rusqlite", Effect::Database),
    ("prql", Effect::Database),
    ("zenoh", Effect::Bus),
    ("rand", Effect::Random),
    ("fastrand", Effect::Random),
    ("image::", Effect::Image),
    ("sysinfo", Effect::Host),
    ("std::env", Effect::Host),
    ("std::process", Effect::Process),
    ("proctitle", Effect::Process),
    ("std::time", Effect::Clock),
    ("chrono", Effect::Clock),
    ("spin_sleep", Effect::Clock),
    ("rustyline", Effect::Interactive),
    ("yapp", Effect::Interactive),
    // Presentation crates. These write stdout and nothing worse, so they are
    // hermetic; they are listed so the audit does not read their absence as
    // evidence of purity.
    ("termimad", Effect::Stdout),
    ("spinoff", Effect::Stdout),
    ("rusty_termcolor", Effect::Stdout),
    ("yansi", Effect::Stdout),
    ("rasciigraph", Effect::Stdout),
];

/// Effects implied by a source file's `use` lines, each with the line that
/// implied it.
pub fn implied_effects(src: &str) -> Vec<(Effect, String)> {
    let mut out: Vec<(Effect, String)> = Vec::new();
    for line in src.lines() {
        let t = line.trim();
        if !t.starts_with("use ") {
            continue;
        }
        for (marker, effect) in EFFECT_MARKERS {
            if t.contains(marker) && !out.iter().any(|(e, _)| e == effect) {
                out.push((*effect, t.to_string()));
            }
        }
    }
    out
}

/// Cross-crate module-path prefixes. `crate::` is handled separately because
/// what it resolves to depends on which crate the file belongs to.
const REACH_PREFIXES: &[(&str, &str)] = &[
    ("rust_multistackvm::stdlib::", "vm/"),
    ("rust_multistack::stdlib::", "stack/"),
];

/// Segments appearing immediately after `prefix`, including the members of a
/// `{a, b}` brace group.
fn segments_after(src: &str, prefix: &str) -> Vec<String> {
    fn ident(s: &str) -> String {
        s.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }

    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = src[from..].find(prefix) {
        let at = from + rel + prefix.len();
        let rest = &src[at..];
        if let Some(stripped) = rest.strip_prefix('{') {
            // `use ...::{a, b::c, d};`
            let group = stripped.split('}').next().unwrap_or("");
            for member in group.split(',') {
                let seg = ident(member.trim());
                if !seg.is_empty() {
                    out.push(seg);
                }
            }
        } else {
            let seg = ident(rest);
            if !seg.is_empty() {
                out.push(seg);
            }
        }
        from = at.max(from + 1);
    }
    out
}

/// The stdlib subsystems a source file's implementation reaches into.
///
/// Scans both `use` lines and inline fully-qualified paths, because the
/// reference does both: `display` imports its dependency
/// (`reference/Bund/src/stdlib/functions/system/display.rs:6`) while
/// `.display` calls `rust_multistackvm::stdlib::print::stdlib_print_inline`
/// fully qualified in the body
/// (`reference/Bund/src/stdlib/functions/oop/display_class.rs:93`). A
/// `use`-only scan misses the second.
///
/// `site_path` decides what bare `crate::` means: inside `Bund` it is
/// `Bund/src/stdlib/`, inside the VM crate it is that crate's own stdlib.
/// Resolving it the same way for both would report `use crate::stdlib::BUND`
/// — the Bund crate's global mutex — as a VM subsystem.
pub fn reached_modules(src: &str, site_path: &str) -> std::collections::BTreeSet<String> {
    /// Module names are snake_case; a capitalised segment is a type or a
    /// static (`BUND`, `Mutex`), not a subsystem.
    fn is_module(seg: &str) -> bool {
        seg.chars().next().is_some_and(|c| c.is_lowercase())
    }

    let mut out = std::collections::BTreeSet::new();

    for (prefix, subsystem_prefix) in REACH_PREFIXES {
        for seg in segments_after(src, prefix) {
            if is_module(&seg) {
                out.insert(format!("{subsystem_prefix}{seg}"));
            }
        }
    }

    let in_bund = site_path.starts_with("reference/Bund/");
    if in_bund {
        for seg in segments_after(src, "crate::stdlib::functions::") {
            if is_module(&seg) {
                out.insert(format!("bund/{seg}"));
            }
        }
        for seg in segments_after(src, "crate::stdlib::helpers::") {
            if is_module(&seg) {
                out.insert(format!("bund/helpers/{seg}"));
            }
        }
    } else {
        // Inside the VM or stack crate, `crate::stdlib::X` is that crate's
        // own subsystem X.
        let own = if site_path.starts_with("reference/rust_multistackvm/") {
            "vm/"
        } else {
            "stack/"
        };
        for seg in segments_after(src, "crate::stdlib::") {
            if is_module(&seg) {
                out.insert(format!("{own}{seg}"));
            }
        }
    }

    out
}

/// A word the owner has put out of scope for Bund2, with the decision that
/// did it. Scope is a different axis from [`Effect`]: a colour-writing word
/// is perfectly hermetic — deterministic bytes on stdout — and still not
/// something Bund2 implements. Conflating the two would misreport both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deferral {
    pub decision: &'static str,
    pub reason: &'static str,
}

/// Registration paths that are out of scope, longest prefix first.
/// `reference/` prefix implied.
const DEFERRED_PATHS: &[(&str, Deferral)] = &[
    // D15: only basic console output is in scope — print, println, nl, space
    // (rust_multistackvm/src/stdlib/print.rs:63-68). Every word under
    // console/ is presentation: spinners (`spinoff`, console/spinner.rs:11),
    // colour (`rusty_termcolor`, console/spinner.rs:12), the typewriter
    // animation (console/terminal.rs:35), and terminal control.
    (
        "Bund/src/stdlib/functions/console",
        Deferral {
            decision: "D15",
            reason: "console presentation: spinners, animation, colour",
        },
    ),
    // D26: the embedded database layer is not a mandatory layer. Deferred
    // pending a decision on what replaces it.
    (
        "Bund/src/stdlib/functions/internaldb",
        Deferral {
            decision: "D26",
            reason: "embedded database layer, deferred",
        },
    ),
];

pub struct Classifier;

impl Classifier {
    /// Effect of a word, or `None` if it resolves nowhere.
    pub fn effect_of(reg: &Registry, word: &str) -> Option<(Effect, Site)> {
        let site = reg.implementing_site(word)?;
        if let Some((_, e, _)) = WORD_OVERRIDES.iter().find(|(w, _, _)| *w == word) {
            return Some((*e, site));
        }
        let path = site
            .path
            .strip_prefix("reference/")
            .unwrap_or(&site.path)
            .to_string();
        for (prefix, effect) in PATH_RULES {
            if path.starts_with(prefix) {
                return Some((*effect, site));
            }
        }
        Some((Effect::Pure, site))
    }

    /// Whether a word is out of scope, and under which decision.
    pub fn deferral_of(reg: &Registry, word: &str) -> Option<(Deferral, Site)> {
        let site = reg.implementing_site(word)?;
        let path = site.path.strip_prefix("reference/").unwrap_or(&site.path);
        DEFERRED_PATHS
            .iter()
            .find(|(prefix, _)| path.starts_with(prefix))
            .map(|(_, d)| (*d, site.clone()))
    }
}

/// The `*` fold family of D12: whole-stack variadic words. Membership is by
/// registration site, listed here with the citation for each.
pub const FOLD_FAMILY: &[(&str, &str)] = &[
    (
        "*+",
        "reference/rust_multistackvm/src/stdlib/math/add.rs:25",
    ),
    (
        "*+.",
        "reference/rust_multistackvm/src/stdlib/math/add.rs:26",
    ),
    (
        "*-",
        "reference/rust_multistackvm/src/stdlib/math/sub.rs:25",
    ),
    (
        "*-.",
        "reference/rust_multistackvm/src/stdlib/math/sub.rs:26",
    ),
    (
        "**",
        "reference/rust_multistackvm/src/stdlib/math/mul.rs:25",
    ),
    (
        "**.",
        "reference/rust_multistackvm/src/stdlib/math/mul.rs:26",
    ),
    (
        "*/",
        "reference/rust_multistackvm/src/stdlib/math/div.rs:25",
    ),
    (
        "*/.",
        "reference/rust_multistackvm/src/stdlib/math/div.rs:26",
    ),
    (
        "*loop",
        "reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:122",
    ),
    (
        "*loop.",
        "reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:123",
    ),
    // Unicode spellings of the sum fold. Probing only the ASCII names would
    // miss a program that writes `Σ`.
    (
        "Σ",
        "reference/rust_multistackvm/src/stdlib/create_aliases.rs:37",
    ),
    (
        "Σ.",
        "reference/rust_multistackvm/src/stdlib/create_aliases.rs:38",
    ),
    (
        "lambda*",
        "reference/Bund/src/stdlib/functions/bund/bund_fun.rs:219",
    ),
    (
        "input*",
        "reference/Bund/src/stdlib/functions/io/input.rs:144",
    ),
    (
        "global*",
        "reference/Bund/src/stdlib/functions/bus/globals.rs:110",
    ),
    (
        "generator.sample*",
        "reference/Bund/src/stdlib/functions/generators/mod.rs:70",
    ),
];

/// Words that make the word table mutable at run time, and so decide whether
/// the world can be closed at compile time.
pub const CLOSED_WORLD: &[(&str, &str)] = &[
    (
        "register",
        "reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:88",
    ),
    (
        "unregister",
        "reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:89",
    ),
    (
        "alias",
        "reference/rust_multistackvm/src/stdlib/alias.rs:72",
    ),
    (
        "unalias",
        "reference/rust_multistackvm/src/stdlib/alias.rs:73",
    ),
];

/// D3: words that turn data into code at run time.
pub const EVAL_FAMILY: &[(&str, &str)] = &[
    (
        "bund.eval",
        "reference/Bund/src/stdlib/functions/bund/bund_eval.rs:124",
    ),
    (
        "bund.eval.",
        "reference/Bund/src/stdlib/functions/bund/bund_eval.rs:124",
    ),
    (
        "bund.eval-file",
        "reference/Bund/src/stdlib/functions/bund/bund_eval.rs:126",
    ),
    (
        "bund.eval-file.",
        "reference/Bund/src/stdlib/functions/bund/bund_eval.rs:126",
    ),
    (
        "!!",
        "reference/Bund/src/stdlib/functions/create_aliases.rs:13",
    ),
    (
        "compile",
        "reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:76",
    ),
    (
        "apply",
        "reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:76",
    ),
    (
        "load.script",
        "reference/Bund/src/stdlib/functions/bund/bund_world_bootstrap.rs:150",
    ),
    (
        "load.lambdas",
        "reference/Bund/src/stdlib/functions/bund/bund_load.rs:201",
    ),
    (
        "save.lambdas",
        "reference/Bund/src/stdlib/functions/bund/bund_save.rs:149",
    ),
    (
        "execute",
        "reference/rust_multistackvm/src/stdlib/execute.rs:124",
    ),
    (
        "execute.",
        "reference/rust_multistackvm/src/stdlib/execute.rs:125",
    ),
    // `!` and `!.` are the spellings the corpus actually uses. Probing only
    // `execute` reports zero and reads as "no dynamic dispatch", which is the
    // opposite of the truth.
    (
        "!",
        "reference/rust_multistackvm/src/stdlib/create_aliases.rs:5",
    ),
    (
        "!.",
        "reference/rust_multistackvm/src/stdlib/create_aliases.rs:6",
    ),
];

/// D5: words that write into an existing value in place.
pub const MUTATORS: &[(&str, &str)] = &[
    (
        "set",
        "reference/rust_multistackvm/src/stdlib/values/value_dict.rs:120",
    ),
    (
        "push",
        "reference/Bund/src/stdlib/functions/values/push.rs:74",
    ),
    (
        "push.",
        "reference/Bund/src/stdlib/functions/values/push.rs:75",
    ),
    (
        "+++",
        "reference/Bund/src/stdlib/functions/create_aliases.rs:34",
    ),
    (
        "+++.",
        "reference/Bund/src/stdlib/functions/create_aliases.rs:35",
    ),
];

/// D1 / D2.
pub const ID_WORDS: &[&str] = &[".id"];
pub const TIMESTAMP_WORDS: &[&str] = &[".timestamp", "time.timestamp"];

#[cfg(test)]
mod tests {
    use super::*;

    const BUND: &str = "reference/Bund/src/stdlib/functions/system/display.rs";
    const VM: &str = "reference/rust_multistackvm/src/stdlib/print.rs";

    #[test]
    fn finds_a_use_line_dependency() {
        // reference/Bund/src/stdlib/functions/system/display.rs:6 — the
        // dependency behind D19.
        let src = "use crate::stdlib::functions::conditional::conditional_fmt;\n";
        assert!(reached_modules(src, BUND).contains("bund/conditional"));
    }

    #[test]
    fn finds_an_inline_qualified_call() {
        // reference/Bund/src/stdlib/functions/oop/display_class.rs:93 calls
        // this fully qualified, with no `use`. A use-only scan misses it.
        let src = "    rust_multistackvm::stdlib::print::stdlib_print_inline(vm)\n";
        assert!(reached_modules(src, BUND).contains("vm/print"));
    }

    #[test]
    fn expands_brace_groups() {
        let src = "use crate::stdlib::functions::{oop, conditional};\n";
        let r = reached_modules(src, BUND);
        assert!(r.contains("bund/oop"));
        assert!(r.contains("bund/conditional"));
    }

    #[test]
    fn crate_stdlib_is_resolved_per_crate() {
        // `use crate::stdlib::BUND` in the Bund crate is the global mutex,
        // not a VM subsystem. Capitalised segments are never modules.
        let src = "use crate::stdlib::BUND;\nuse crate::stdlib::Mutex;\n";
        assert!(reached_modules(src, BUND).is_empty());
        // The same text inside the VM crate still yields nothing, for the
        // same capitalisation reason.
        assert!(reached_modules(src, VM).is_empty());
        // But a lowercase segment inside the VM crate is a real subsystem.
        assert!(reached_modules("use crate::stdlib::print;\n", VM).contains("vm/print"));
    }

    #[test]
    fn helpers_are_namespaced_separately() {
        let src = "use crate::stdlib::helpers::zenoh;\n";
        assert!(reached_modules(src, BUND).contains("bund/helpers/zenoh"));
    }

    #[test]
    fn segments_after_does_not_loop_forever() {
        // A prefix with nothing parseable after it must still terminate.
        let out = segments_after("crate::stdlib::functions::", "crate::stdlib::functions::");
        assert!(out.is_empty());
    }
}
