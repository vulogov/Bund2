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
// `csv`, the data-file conditional.
pub mod data;
// The world file: `save.model` and `load.model`.
pub mod world;

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
    data::register(r);
    world::register(r, opts);
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
    // Last of all, so the stubs replace `bund.eval`, which `singles`
    // registers above, and `use`.
    if opts.noeval {
        host::register_noeval_stubs(r);
    }
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

    /// One audited run of a native, kept for D55's comparison.
    struct Run {
        ok: bool,
        err: String,
        top: Vec<String>,
        kinds: Vec<u16>,
        wb: Vec<String>,
        /// The values beneath the operands are still the padding put there.
        /// A word that returns nothing and reorders what lies beneath —
        /// `rotate_stack_left` given the current stack's name — changes
        /// nothing else a run shows.
        intact: bool,
        breaches: Vec<String>,
        observed: Vec<String>,
    }

    impl Run {
        /// The same answer: status and error, and on success what it produced
        /// and what it left on the workbench. After a failure only the error
        /// counts, since what a failed native left behind is not its answer.
        fn same(&self, o: &Run) -> bool {
            self.ok == o.ok
                && self.err == o.err
                && (!self.ok || (self.top == o.top && self.wb == o.wb))
        }

        /// For a native whose answer varies by itself, such as a random one:
        /// the same status, and on success the same kinds produced.
        fn same_kind(&self, o: &Run) -> bool {
            self.ok == o.ok && (!self.ok || self.kinds == o.kinds)
        }
    }

    /// Blank what F14 says is not behaviour: ids and stamps, in the reference's
    /// rendering and in a heap value's `Debug`, whose identity counter and
    /// stamp cell differ on every run.
    fn without_ids(s: &str) -> String {
        let number = |c: char| !(c.is_ascii_digit() || matches!(c, '.' | 'e' | '+' | '-'));
        let mut s = blank(s, "id: \"", |c| c == '"');
        for prefix in ["stamp: ", "identity: Cell { value: ", "stamp: Cell { value: "] {
            s = blank(&s, prefix, number);
        }
        s
    }

    /// After each `prefix`, drop characters up to the first that `stop` accepts.
    ///
    /// The prefix is matched **without regard to case**. `string.upper` and
    /// `string.title` handed a lambda case-convert its rendering, id and stamp
    /// included, so `ID: "…"` and `Stamp: …` must be blanked too. ASCII
    /// lowercasing keeps every byte offset, so the search runs on a lowercased
    /// copy and the text kept is the original.
    fn blank(s: &str, prefix: &str, stop: impl Fn(char) -> bool) -> String {
        let lower = s.to_ascii_lowercase();
        let prefix = prefix.to_ascii_lowercase();
        let mut out = String::with_capacity(s.len());
        let mut at = 0;
        while let Some(i) = lower[at..].find(&prefix) {
            let keep = at + i + prefix.len();
            out.push_str(&s[at..keep]);
            let end = s[keep..].find(&stop).map_or(s.len(), |j| keep + j);
            at = end;
        }
        out.push_str(&s[at..]);
        out
    }

    /// Why D55 flagged a native, with the two answers that differed.
    fn differs_why(a: &Run, b: &Run) -> String {
        let show = |r: &Run| -> String {
            let s = if r.ok {
                format!("Ok {:?} {:?}", r.top, r.wb)
            } else {
                format!("Err {}", r.err)
            };
            s.chars().take(120).collect()
        };
        format!(
            "answers differently when the values beneath its operands change: {} / {}",
            show(a),
            show(b)
        )
    }

    /// Run `name` once under the effect audit, with `pad` beneath `ops`.
    fn run_one(
        template: &bund2_api::Registry,
        name: &str,
        produces: u8,
        pad: &[BundValue],
        ops: &[BundValue],
        wb: Option<&BundValue>,
    ) -> Run {
        // A clone of the prepared registry: every word, and the class the
        // OBJECT kind was made from.
        let mut i = Interp::new();
        i.registry = template.clone();
        for v in pad.iter().chain(ops) {
            i.push(v.clone());
        }
        if let Some(v) = wb {
            i.push_workbench(v.clone());
        }
        i.effect_audit = Some(Vec::new());
        let r = i.eval_indexed(&[BundValue::call(name)]);
        let breaches = i.effect_audit.take().unwrap_or_default();
        let observed = i.observations.take();
        let snap = i.snapshot();
        let n = usize::from(produces).min(snap.len());
        let top = &snap[snap.len() - n..];
        // A native that switched the current stack is looking at another
        // stack now; switches are §S5's epoch's business, and the effect audit
        // skips them too.
        let switched = i.current_name() != "main";
        let intact = switched
            || (snap.len() >= pad.len()
                && snap.iter().zip(pad).all(|(got, put)| got.display() == put.display()));
        Run {
            intact,
            ok: r.is_ok(),
            err: r.err().map(|(_, e)| without_ids(&e.0)).unwrap_or_default(),
            top: top.iter().map(|v| without_ids(&v.display())).collect(),
            kinds: top.iter().map(BundValue::dt).collect(),
            wb: i
                .snapshot_workbench()
                .iter()
                .map(|v| without_ids(&v.display()))
                .collect(),
            breaches,
            observed,
        }
    }

    /// **Criterion 28 — D48, F94, D55.** Every fixed-effect native, run under
    /// the effect audit against a palette of fifteen operand kinds, with the top
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
        // `"main"` is the current stack's name, so a word that takes a stack
        // name is tried on the stack promotion would be holding (D55).
        const KINDS: &str = "7 2.5 true \"zz_nofile\" \"A\" \"C1\" \"main\" [ 1 2 ] list dict \
                             { 1 } { drop } nodata `dup :C1 object";
        let mut setup = Interp::new();
        crate::register_all(&mut setup.registry);
        let src = format!("{CLASS}\n{KINDS}");
        setup
            .eval(&bund2_syntax::compile(&src).expect("compiles"))
            .expect("the palette builds");
        let palette = setup.snapshot();
        assert_eq!(palette.len(), 15, "fifteen operand kinds");
        // Kinds that display as their full rendering, id and stamp included:
        // the lambdas and the object. An answer built from one depends on
        // identity and time, which F14 says are not behaviour, and a word that
        // case-converts it respells `id:` in ways no scrubbing keeps up with.
        // D55's comparison skips tuples holding one; the breach, observation
        // and intact checks still run on them.
        let f14_kind: Vec<bool> = palette
            .iter()
            .map(|v| v.display().to_ascii_lowercase().contains("stamp"))
            .collect();
        let template = setup.registry.clone();
        // Natives that act on the host are never run here: the palette's `7`
        // would make `sleep.seconds` wait seven seconds, and its strings would
        // hand `fs.rm` real relative paths. Left unrun, they are not reached,
        // so promotion syncs before them as it does before an embedder's
        // native (D48, dated note 2026-09-11).
        // `password` waits on the terminal for a line nobody will type, and
        // `save.model` would write a world file for each palette string.
        const ACTS_ON_HOST: [&str; 6] = [
            "fs.rm",
            "sleep.seconds",
            "system.setproctitle",
            "system.setproctitle.",
            "password",
            "save.model",
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
        // D55: natives seen reading beyond their operands, with why.
        let mut observers: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        // What lies beneath the operands: the usual padding, and other values
        // for the comparison.
        let pad_a = vec![BundValue::int(7); 3];
        let pad_c = vec![BundValue::str("pad"), BundValue::float(0.5), BundValue::int(-3)];
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
            // D55, decided per native: whether two identical runs ever
            // differed, and the first difference of each kind.
            let (mut nondet, mut det_diff, mut kind_diff): (bool, Option<String>, Option<String>) =
                (false, None, None);
            for t in &tuples {
                for w in &wb {
                    // The operands: `7`s below the varied ones, up to the
                    // declared depth, then the varied kinds on top.
                    let mut ops: Vec<BundValue> = vec![BundValue::int(7); depth - varied];
                    ops.extend(t.iter().map(|&x| palette[x].clone()));
                    let wbv = w.map(|x| palette[x].clone());
                    let run =
                        |pad: &[BundValue]| run_one(&template, name, e.produces, pad, &ops, wbv.as_ref());
                    // D55's differential: padded twice, so a deterministic
                    // native answers alike both times; with nothing beneath
                    // its operands; and with other values beneath.
                    let a1 = run(&pad_a);
                    let a2 = run(&pad_a);
                    let b = run(&[]);
                    let c = run(&pad_c);
                    if a1.ok && a1.breaches.is_empty() {
                        reached.insert(name.clone());
                    }
                    for r in [&a1, &a2, &b, &c] {
                        breaches.extend(r.breaches.iter().cloned());
                        if let Some(o) = r.observed.first() {
                            observers.entry(name.clone()).or_insert_with(|| o.clone());
                        }
                        if r.ok && !r.intact {
                            observers.entry(name.clone()).or_insert_with(|| {
                                "changes the values beneath its operands".to_string()
                            });
                        }
                    }
                    let compare = !t.iter().any(|&x| f14_kind[x]) && !w.is_some_and(|x| f14_kind[x]);
                    if compare {
                        if !a1.same(&a2) {
                            nondet = true;
                        }
                        let other = if a1.same(&b) { &c } else { &b };
                        if det_diff.is_none() && !a1.same(other) {
                            det_diff = Some(differs_why(&a1, other));
                        }
                        let other = if a1.same_kind(&b) { &c } else { &b };
                        if kind_diff.is_none() && !a1.same_kind(other) {
                            kind_diff = Some(differs_why(&a1, other));
                        }
                    }
                }
            }
            // A native that answered two identical runs differently even once
            // is random, and is compared by status and kinds alone everywhere;
            // one lucky match on a single tuple does not make it deterministic.
            if let Some(w) = if nondet { kind_diff } else { det_diff } {
                observers.entry(name.clone()).or_insert(w);
            }
        }
        let shown: Vec<&String> = breaches.iter().take(20).collect();
        assert!(
            breaches.is_empty(),
            "{} breach(es) of a declared effect, first 20:\n{shown:#?}",
            breaches.len()
        );

        // D55: a native seen reading beyond its operands is never crossed.
        let promotable: std::collections::BTreeSet<String> = reached
            .iter()
            .filter(|n| !observers.contains_key(*n))
            .cloned()
            .collect();

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let list = root.join("tests/golden/PROMOTABLE.txt");
        if std::env::var_os("BUND2_UPDATE_PROMOTABLE").is_some() {
            let mut out = String::from(
                "# Natives RFC-0005 §S5's promotion may cross (D48, D55): every\n\
                 # fixed-effect native criterion 28's palette brought to `Ok` with no\n\
                 # breach of its declared effect, and that D55's audit did not see\n\
                 # reading beyond its operands. Anything absent is synced before, as an\n\
                 # embedder's native is. Written by BUND2_UPDATE_PROMOTABLE=1 cargo\n\
                 # test -p bund2-stdlib promotable; never edited by hand.\n",
            );
            out.push_str(&format!(
                "# {} promotable of {} reached, of {} fixed-effect natives; {} reached natives\n\
                 # observe beyond their operands (D55) and are never crossed.\n",
                promotable.len(),
                reached.len(),
                natives.len(),
                reached.len() - promotable.len()
            ));
            for n in &promotable {
                out.push_str(n);
                out.push('\n');
            }
            out.push_str("#\n# Reached, but seen reading beyond their operands (D55), so never crossed:\n");
            for (n, why) in &observers {
                if reached.contains(n) {
                    out.push_str(&format!("# observes: {n} — {why}\n"));
                }
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
        let gained: Vec<&String> = promotable.difference(&listed).collect();
        let lost: Vec<&String> = listed.difference(&promotable).collect();
        // Why each lost native left, when D55 is the reason, so the review of a
        // stale list does not need the list rewritten first.
        let why: Vec<String> = lost
            .iter()
            .filter_map(|n| observers.get(*n).map(|w| format!("{n}: {w}")))
            .collect();
        assert!(
            gained.is_empty() && lost.is_empty(),
            "PROMOTABLE.txt is stale: now reached {gained:?}, no longer reached {lost:?}; \
             of those, D55 observers: {why:#?}"
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

    /// **Criterion 11's path set — RFC-0005 §S8, the twelfth review's S1.**
    /// Every function in this crate whose shipped code calls `Vm::eval_lambda`,
    /// `Vm::apply` or `Vm::scoped_call`. Each re-enters evaluation and spends a
    /// Rust frame per level, so each is a path criterion 11 measures `c_p` and
    /// `δ_p` over. A new re-entering function fails this test until it is
    /// named below, so the set cannot go stale the way a list kept in prose
    /// did, twice. A source scan, cut at each file's test module as criterion
    /// 25's is; a closure counts under the function it is written in. It
    /// matches each call in its method form and its path form (`vm.apply(`,
    /// `Vm::apply(`), reads a function head whatever its qualifiers, and
    /// descends into subdirectories (the thirteenth review's S2). It cannot
    /// see a call made through a function pointer or a macro, nor anything
    /// outside this crate (RFC-0005 assumption 24).
    #[test]
    fn every_reentering_function_is_named() {
        const REENTERING: [&str; 21] = [
            "conditional.rs: run_context",
            "conditional.rs: run_error",
            "conditional.rs: run_ifthenelse",
            "conditional.rs: run_through",
            "conditional.rs: run_tryexcept",
            "control.rs: for_base",
            "control.rs: while_base",
            "data.rs: run_csv",
            "data.rs: run_sqlite",
            "oop.rs: dispatch_method",
            "oop.rs: run_init",
            "seq.rs: loop_base",
            "seq.rs: loop_over_base",
            "seq.rs: map_base",
            "seq.rs: times_base",
            "singles.rs: apply",
            "singles.rs: conditional_move",
            "singles.rs: eval_source",
            "terminal.rs: input_loop",
            // `execute_value` delegates to `execute_reached`, whose worklist
            // (F114) calls this for one value at a time. It holds both calls:
            // a name through `Vm::apply`, a reached lambda through
            // `Vm::eval_lambda` (F113).
            "values.rs: execute_one",
            // `text`, a closure in the registration: it applies a TEXTBUFFER,
            // which pushes, so it cannot recurse.
            "values.rs: register_words",
        ];
        const QUALIFIERS: [&str; 5] = ["pub", "const", "unsafe", "async", "extern"];
        const CALLS: [&str; 6] = [
            ".eval_lambda(",
            ".apply(",
            ".scoped_call(",
            "::eval_lambda(",
            "::apply(",
            "::scoped_call(",
        ];
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = std::collections::BTreeSet::new();
        let mut dirs = vec![src.clone()];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).expect("dir reads") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }
                if path.extension().is_none_or(|x| x != "rs") {
                    continue;
                }
                let file = path
                    .strip_prefix(&src)
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                let text = std::fs::read_to_string(&path).expect("reads");
                let shipped = text.split("#[cfg(test)]\nmod ").next().unwrap_or_default();
                let mut current = String::new();
                for line in shipped.lines() {
                    let t = line.trim_start();
                    if t.starts_with("//") {
                        continue;
                    }
                    // A head is `fn ` preceded only by qualifiers: `pub(super)`,
                    // `const`, `unsafe`, `async`, `extern "C"` and the like.
                    if let Some(at) = t.find("fn ") {
                        let head_ok = t[..at].split_whitespace().all(|w| {
                            QUALIFIERS.iter().any(|q| w.starts_with(q)) || w.starts_with('"')
                        });
                        if head_ok {
                            current = t[at + 3..]
                                .split(['(', '<'])
                                .next()
                                .unwrap_or_default()
                                .to_string();
                        }
                    }
                    if CALLS.iter().any(|m| line.contains(m)) {
                        found.insert(format!("{file}: {current}"));
                    }
                }
            }
        }
        let named: std::collections::BTreeSet<String> =
            REENTERING.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(
            found, named,
            "criterion 11's path set and the functions that re-enter evaluation differ"
        );
    }
}
