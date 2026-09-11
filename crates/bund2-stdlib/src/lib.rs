//! The standard library: native words, effects, and JIT lowerings.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]

mod pull;
/// What a `.` suffix means — three shapes, none of them the default.
pub(crate) mod wb;

pub mod stack;

pub mod console;

pub mod logic;

pub mod values;

pub mod math;

pub mod control;

pub mod conditional;

pub mod seq;

pub mod oop;

/// The specialised arms Bund2 publishes — RFC-0005 §S6.
pub mod fragments;
pub mod library;
/// `bund/string`'s library group, and every `.`-suffixed sibling.
pub mod library_string;

pub mod check;

pub mod graph;

pub mod singles;

pub mod sysinfo;

pub mod json;

pub mod sort;

pub mod convert;

pub mod report;
// Script arguments, the filesystem, the clock, the process title, `io.graph`.
pub mod host;
// Ids, random integers and strings, and `generator`.
pub mod random;
// `input`, `input*`, `password`, `bund.prompt`, `io.banner`, the host table.
pub mod terminal;

/// Register everything this crate provides.
pub fn register_all(r: &mut bund2_api::Registry) {
    register_all_with(r, &host::HostOptions::default());
}

/// Register every word, with the host words set up as `opts` says: under
/// `--noio` the I/O words are registered as stubs that fail, as the reference
/// does.
pub fn register_all_with(r: &mut bund2_api::Registry, opts: &host::HostOptions) {
    host::register(r, opts);
    random::register(r);
    terminal::register(r, opts);
    stack::register(r);
    console::register(r);
    logic::register(r);
    values::register_words(r);
    math::register(r);
    control::register(r);
    conditional::register(r);
    seq::register(r);
    oop::register(r);
    convert::register_words(r);
    sort::register(r);
    json::register(r);
    sysinfo::register(r);
    singles::register(r);
    graph::register(r);
    library::register(r);
    library_string::register(r);
}

/// **What RFC-0005 §S5's promotion trusts about natives, checked rather than
/// assumed** — criteria 24 and 25.
///
/// Promotion keeps values in registers across a call to a native whose effect
/// is a fixed pair, so two things about every native must hold: a fixed effect
/// never hides a body the native runs, and no native reports at `Error`
/// severity mid-body, where a snapshot would read a short stack (D45). Both
/// were conventions until the eighth review found the first one broken
/// (`execute.`, F87). These tests turn them into properties.
#[cfg(test)]
mod honesty_tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::BundValue;

    /// **Criterion 24.** Run every native with a fixed effect against a stack
    /// *and* a workbench of lambdas, and assert no body starts. `entry_log`
    /// records every frame push (D42), so a native that runs its operand —
    /// through `eval_lambda`, `tail_lambda`, `scoped_call` or `apply` — shows
    /// up there whatever path it took. A word that is honest about running a
    /// body declares `StackEffect::opaque` and is skipped.
    ///
    /// Operands are lambdas only: a string would be a file name to half the
    /// library. So a native that runs a body only when handed something else
    /// is not caught here. `object` was one: it runs a class's `.init` and
    /// needs a registered class name to get there (F91). The corpus audit
    /// below catches what the programs reach, and this one covers the words
    /// they never call.
    #[test]
    fn no_fixed_effect_native_runs_a_body() {
        let mut probe = Interp::new();
        crate::register_all(&mut probe.registry);
        let body = bund2_syntax::compile("{ 1 }")
            .expect("compiles")
            .into_iter()
            .next()
            .expect("one value");
        let mut offenders = Vec::new();
        for (name, e) in probe.registry.declared_effects() {
            if e.opaque {
                continue;
            }
            let mut i = Interp::new();
            crate::register_all(&mut i.registry);
            for _ in 0..usize::from(e.consumes) + 2 {
                i.push(body.clone());
                i.push_workbench(body.clone());
            }
            i.entry_log = Some(Vec::new());
            // Failing is fine — most natives refuse a lambda. Running one is not.
            let _ = i.eval(&[BundValue::call(name.as_str())]);
            if i.entry_log.take().is_some_and(|log| !log.is_empty()) {
                offenders.push(name);
            }
        }
        assert!(
            offenders.is_empty(),
            "declared a fixed effect but ran a body: {offenders:?}"
        );
    }

    /// **Criterion 24, over the corpus.** Every program `conform` runs — each
    /// line of `tests/golden/HERMETIC.txt`, and each probe with a golden — run
    /// in process with [`Interp::effect_audit`] on. A native that declares a
    /// fixed effect may not start a body, file a tail request or dispatch a
    /// word, and its depth change must match its declaration.
    ///
    /// The test above samples operands, and passes only lambdas. This one uses
    /// the operands the programs actually hand each word, which is how it
    /// reaches `object`: that word runs a class's `.init`, and needs a
    /// registered class name to get there (F91). A word the corpus never
    /// calls is not reached, which is the test above's job.
    #[test]
    fn every_fixed_effect_native_keeps_its_effect_over_the_corpus() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let hermetic = std::fs::read_to_string(root.join("tests/golden/HERMETIC.txt"))
            .expect("HERMETIC.txt reads");
        let mut programs: Vec<std::path::PathBuf> = hermetic
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| root.join(l))
            .collect();
        for entry in std::fs::read_dir(root.join("tests/probes")).expect("probes exist") {
            let path = entry.expect("entry").path();
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let golden = root.join("tests/golden/probes").join(format!("{stem}.golden"));
            if path.extension().is_some_and(|x| x == "bund") && golden.exists() {
                programs.push(path);
            }
        }
        programs.sort();
        assert!(programs.len() > 80, "found only {} programs", programs.len());

        let mut breaches = Vec::new();
        for path in &programs {
            let src = std::fs::read_to_string(path).expect("program reads");
            // A program that does not parse runs nothing, so it has nothing to
            // audit; its golden checks the parse error.
            let Ok(values) = bund2_syntax::compile(&src) else {
                continue;
            };
            let mut i = Interp::new();
            crate::register_all(&mut i.registry);
            i.effect_audit = Some(Vec::new());
            // Failing is fine: many programs fail on purpose.
            let _ = i.eval(&values);
            let shown = path.strip_prefix(&root).unwrap_or(path).display().to_string();
            for b in i.effect_audit.take().unwrap_or_default() {
                breaches.push(format!("{shown}: {b}"));
            }
        }
        breaches.sort();
        breaches.dedup();
        assert!(
            breaches.is_empty(),
            "{} breach(es) of a declared effect:\n{}",
            breaches.len(),
            breaches.join("\n")
        );
    }

    /// **Criterion 28 — D48, F94.** Every fixed-effect native, run under the
    /// effect audit against a palette of fourteen operand kinds, with the top
    /// two operands drawn from every pair of kinds (deeper ones and three
    /// padding values are `7`), and — for a workbench form, a name ending in
    /// `.` or `,` — against every kind on the workbench as well.
    ///
    /// Two assertions. **No run breaches its declared effect.** And **the
    /// natives that some run brought to `Ok` with no breach are exactly the
    /// ones `tests/golden/PROMOTABLE.txt` lists.** That list is what RFC-0005
    /// §S5's promotion may cross (D48): a native nothing brought to `Ok` has a
    /// pair nothing has checked, and compiled code syncs before it as it does
    /// before an embedder's. Corpus reach is criterion 24's; this one reaches
    /// the words no program calls. It found `drop_stack` (F94).
    ///
    /// `BUND2_UPDATE_PROMOTABLE=1 cargo test -p bund2-stdlib promotable`
    /// rewrites the list after a deliberate change; the diff is the review.
    #[test]
    fn every_fixed_effect_native_keeps_its_pair_over_the_promotable_palette() {
        const CLASS: &str = ":C1 class :.class_name \"C1\" set register";
        const KINDS: &str = "7 2.5 true \"zz_nofile\" \"A\" \"C1\" [ 1 2 ] list dict \
                             { 1 } { drop } nodata `dup :C1 object";
        let mut setup = Interp::new();
        crate::register_all(&mut setup.registry);
        let src = format!("{CLASS}\n{KINDS}");
        setup
            .eval(&bund2_syntax::compile(&src).expect("compiles"))
            .expect("the palette builds");
        let palette = setup.snapshot();
        assert_eq!(palette.len(), 14, "fourteen operand kinds");
        let template = setup.registry.clone();
        // Natives that act on the host are never run here: the palette's `7`
        // would make `sleep.seconds` wait seven seconds, and its strings would
        // hand `fs.rm` real relative paths. Left unrun, they are not reached,
        // so promotion syncs before them as it does before an embedder's
        // native (D48, dated note 2026-09-11).
        // `password` waits on the terminal for a line nobody will type.
        const ACTS_ON_HOST: [&str; 5] = [
            "fs.rm",
            "sleep.seconds",
            "system.setproctitle",
            "system.setproctitle.",
            "password",
        ];
        let natives: Vec<(String, bund2_api::StackEffect)> = setup
            .registry
            .declared_effects()
            .into_iter()
            .filter(|(n, e)| !e.opaque && !ACTS_ON_HOST.contains(&n.as_str()))
            .collect();

        let k = palette.len();
        let mut breaches = std::collections::BTreeSet::new();
        let mut reached = std::collections::BTreeSet::new();
        for (name, e) in &natives {
            let depth = usize::from(e.consumes);
            let bench = name.ends_with('.') || name.ends_with(',');
            // A workbench form varies its top operand and the workbench; any
            // other varies its top two operands.
            let varied = depth.min(if bench { 1 } else { 2 });
            let tuples: Vec<Vec<usize>> = (0..k.pow(varied as u32))
                .map(|mut n| {
                    (0..varied)
                        .map(|_| {
                            let d = n % k;
                            n /= k;
                            d
                        })
                        .collect()
                })
                .collect();
            let wb: Vec<Option<usize>> = if bench {
                std::iter::once(None).chain((0..k).map(Some)).collect()
            } else {
                vec![None]
            };
            for t in &tuples {
                for w in &wb {
                    // A clone of the prepared registry: every word, and the
                    // class the OBJECT kind was made from.
                    let mut i = Interp::new();
                    i.registry = template.clone();
                    for _ in 0..3 + depth - varied {
                        i.push(BundValue::int(7));
                    }
                    for &x in t {
                        i.push(palette[x].clone());
                    }
                    if let Some(x) = w {
                        i.push_workbench(palette[*x].clone());
                    }
                    i.effect_audit = Some(Vec::new());
                    let r = i.eval(&[BundValue::call(name.as_str())]);
                    let log = i.effect_audit.take().unwrap_or_default();
                    if r.is_ok() && log.is_empty() {
                        reached.insert(name.clone());
                    }
                    breaches.extend(log);
                }
            }
        }
        let shown: Vec<&String> = breaches.iter().take(20).collect();
        assert!(
            breaches.is_empty(),
            "{} breach(es) of a declared effect, first 20:\n{shown:#?}",
            breaches.len()
        );

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let list = root.join("tests/golden/PROMOTABLE.txt");
        if std::env::var_os("BUND2_UPDATE_PROMOTABLE").is_some() {
            let mut out = String::from(
                "# Natives RFC-0005 §S5's promotion may cross (D48): every fixed-effect\n\
                 # native criterion 28's palette brought to `Ok` with no breach of its\n\
                 # declared effect. Anything absent is synced before, as an embedder's\n\
                 # native is. Written by BUND2_UPDATE_PROMOTABLE=1 cargo test -p\n\
                 # bund2-stdlib promotable; never edited by hand.\n",
            );
            out.push_str(&format!(
                "# {} of {} fixed-effect natives reached.\n",
                reached.len(),
                natives.len()
            ));
            for n in &reached {
                out.push_str(n);
                out.push('\n');
            }
            // What the palette never brought to `Ok`, so the list says what it
            // withholds. Comment lines, so the comparison below ignores them.
            out.push_str("#\n# Fixed-effect natives the palette never brought to `Ok`:\n");
            for (n, _) in &natives {
                if !reached.contains(n) {
                    out.push_str(&format!("# not reached: {n}\n"));
                }
            }
            std::fs::write(&list, out).expect("writes the list");
            return;
        }
        let listed: std::collections::BTreeSet<String> = std::fs::read_to_string(&list)
            .expect("tests/golden/PROMOTABLE.txt exists")
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(str::to_string)
            .collect();
        let gained: Vec<&String> = reached.difference(&listed).collect();
        let lost: Vec<&String> = listed.difference(&reached).collect();
        assert!(
            gained.is_empty() && lost.is_empty(),
            "PROMOTABLE.txt is stale: now reached {gained:?}, no longer reached {lost:?}"
        );
    }

    /// **Criterion 25.** No shipped code in this crate reports at `Error`
    /// severity. Natives return errors; the embedder reports them after
    /// evaluation has returned. A native that reported one mid-body would get
    /// a stack snapshot under the default reporter while values were promoted.
    /// A source scan, so it sees paths no run reaches; each file is cut at its
    /// first `#[cfg(test)]` module, which in this crate always runs to the end.
    #[test]
    fn no_native_reports_at_error_severity() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = Vec::new();
        for entry in std::fs::read_dir(&src).expect("src exists") {
            let path = entry.expect("entry").path();
            if path.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("reads");
            let shipped = text.split("#[cfg(test)]\nmod ").next().unwrap_or_default();
            for (n, line) in shipped.lines().enumerate() {
                if line.contains("Diagnostic::error") || line.contains("Severity::Error") {
                    found.push(format!("{}:{}", path.display(), n + 1));
                }
            }
        }
        assert!(found.is_empty(), "reports at Error severity: {found:?}");
    }
}
