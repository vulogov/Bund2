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

/// Whether this build carries `string.grok` — D40's feature gate.
///
/// A constant here because the feature is this crate's: a crate that depends
/// on this one cannot ask `cfg!(feature = "grok")` and get a true answer.
pub const GROK_BUILT_IN: bool = cfg!(feature = "grok");

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
/// The natives promotion may cross, as registration ids — D48's list read as a
/// table, beside §S6's fragments. D68 needs both: which calls may be inlined,
/// and which may be crossed.
pub mod promotable;
pub mod library;
/// `bund/string`'s library group, and every `.`-suffixed sibling.
pub mod library_string;
/// The Porter stemmer `string.tokenize.stemmed` needs, written out rather than
/// brought in -- see the module's own note for why, and for what verifies it.
mod stem;

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
// The local bus: `send`, `recv`, `bus.data` (D87).
pub mod bus;
// The program's own logging: the five `log.*` words (D90).
pub mod logging;
// `math.normalize`, `math.smoothing`, `seq.asc`/`seq.desc`, and the series
// reader they share.
pub mod series;
// base64, `unique` and `pull.workbench`.
pub mod encoding;

/// Run a line of Bund source in `vm`, value by value, as `bund.eval` does
/// (`reference/Bund/src/stdlib/helpers/eval.rs:7-37`).
///
/// Public for the embedder: an interpreter has no parser of its own, so the
/// line a debugger's console is handed at a stop is run through this
/// (RFC-0008 §W2).
///
/// **The failure is the word's own, without `bund.eval`'s frame around it.**
/// `eval_source` prefixes the reference's `Attempt to evaluate value …`, which
/// carries the raw rendering of the value; that is `bund.eval`'s contract and
/// noise at a prompt where the person has just typed the line.
pub fn eval_line(vm: &mut dyn bund2_api::Vm, src: &str) -> Result<(), bund2_api::Error> {
    let stream = bund2_syntax::compile(src).map_err(|e| bund2_api::Error(e.render(src)))?;
    for word in stream {
        if word.dt() == bund2_value::NONE {
            continue;
        }
        if word.dt() == bund2_value::EXIT {
            break;
        }
        vm.apply(word)?;
    }
    Ok(())
}

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
    data::register(r, opts);
    world::register(r, opts);
    bus::register(r, opts);
    logging::register(r);
    series::register(r);
    encoding::register(r);
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

    /// **F158: this crate reads the terminal in two functions and in no
    /// other.** `Terminal::line` and `secret`, both in `terminal.rs`, and both
    /// answering without touching standard input under this test build.
    ///
    /// This is the enforcement the earlier rule lacked. "Give `cargo test` a
    /// closed stdin" was true, written down after F141, and not checked by
    /// anything — and four test binaries were later found blocked on exactly
    /// the read it warned about. A reader added here that bypasses the two
    /// helpers would reintroduce that hang, silently, for whoever next runs
    /// the suite with a live stdin; this test fails first.
    ///
    /// A source scan, cut at each file's test module as its siblings are.
    /// Three spellings are looked for, which between them are every way this
    /// crate has ever reached standard input: `rustyline`'s read, `yapp`'s,
    /// and the standard library's handle.
    #[test]
    fn every_terminal_read_goes_through_the_two_helpers() {
        const READERS: [&str; 2] = ["terminal.rs: line", "terminal.rs: secret"];
        const READS_AT: [&str; 3] = [".readline(", "read_password", "stdin()"];
        const QUALIFIERS: [&str; 5] = ["pub", "const", "unsafe", "async", "extern"];
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
                    if READS_AT.iter().any(|m| line.contains(m)) {
                        found.insert(format!("{file}: {current}"));
                    }
                }
            }
        }
        let want: std::collections::BTreeSet<String> =
            READERS.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(
            found, want,
            "F158: standard input may be read only in `Terminal::line` and `secret`"
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
        // `password` and `bund.prompt` wait on the terminal for a line nobody
        // will type, and `save.model` would write a world file for each palette
        // string.
        //
        // **`bund.prompt` was missing from this list until F141**, and the cost
        // was not a failure but a *hang*: with stdin on a pipe or tty that
        // never reaches EOF, the read blocks and the whole crate's suite stops
        // with it. Two test binaries were found still resident five days after
        // the runs that started them, and a background run chained to a commit
        // never reached the commit. A hang leaves no failing test — only a run
        // that never ends, which in the background is indistinguishable from
        // one still going.
        //
        // `input` and `input*` read the terminal too and are not here: both are
        // `StackEffect::opaque`, which this audit skips before it runs
        // anything.
        //
        // **The four writers were found by the files they left behind.** The
        // palette's strings are filenames to `file.write`, so the first run
        // after it was implemented wrote `A`, `C1`, `dup`, `main` and
        // `zz_nofile` into the crate directory -- in the repository working
        // tree, as untracked files. The visible symptom was not those files
        // but the audit's own answer: `csv` and `sqlite` turned up as `now
        // reached`, because a palette string had become the name of a file
        // that existed and they could open it. A host write does not only act
        // on the host, it changes what the rest of the audit measures.
        //
        // `fs.is_file`, `fs.ls` and `filename` stay: the first two read the
        // working directory and answer `Ok` whatever is in it, and `filename`
        // is lexical and touches no filesystem at all.
        // **`system.shell` is the sharpest case, and it was not caught by a
        // symptom.** It hands its operand to `/bin/sh -c`, so leaving it here
        // unlisted means the audit executes fifteen palette strings as shell
        // commands on every run of `cargo test`. It never appeared as `now
        // reached` -- every palette string fails as a command, so the native
        // returns `Err` and is never certified -- and that is luck rather than
        // safety: the spawn had already happened by then, and a palette string
        // that happened to name a real command would have run it. Listed for
        // what it does, not for what it managed to do.
        const ACTS_ON_HOST: [&str; 13] = [
            "system.shell",
            "system.shell.",
            "fs.rm",
            "fs.cp",
            "fs.mv",
            "file.write",
            "file.write.",
            "sleep.seconds",
            "system.setproctitle",
            "system.setproctitle.",
            "password",
            "bund.prompt",
            "save.model",
        ];
        // **Natives a feature gate may or may not have registered — F123.**
        //
        // `PROMOTABLE.txt` is one file and there are two builds. `string.grok`
        // and `string.grok.` exist only under `--features grok`, which D10 and
        // D40 keep off by default, so a list regenerated without the feature
        // fails `--all-features` with `now reached ["string.grok",
        // "string.grok."]`, and a list regenerated with it fails the default
        // build from the other side. No content of the file satisfies both.
        //
        // **Excluded at the source rather than subtracted afterwards.**
        // Filtering here means the audit never *claims* to have checked them;
        // subtracting after the run would measure them under `--all-features`
        // and then discard the result, which is a measurement taken and thrown
        // away. Unreached is also the conservative answer: promotion syncs
        // before an unlisted native exactly as it does before an embedder's, so
        // the two words cost speed and never meaning.
        //
        // Kept by hand, as `ACTS_ON_HOST` is, and for the same reason: a new
        // feature-gated native runs for real until it is named here. `grok` is
        // the only feature this crate has today, and it binds exactly these
        // two.
        const FEATURE_GATED: [&str; 2] = ["string.grok", "string.grok."];
        let natives: Vec<(String, bund2_api::StackEffect)> = setup
            .registry
            .declared_effects()
            .into_iter()
            .filter(|(n, e)| {
                !e.opaque
                    && !ACTS_ON_HOST.contains(&n.as_str())
                    && !FEATURE_GATED.contains(&n.as_str())
            })
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
                    // How many more identical runs F163's recheck makes.
                    // Sixteen puts a two-outcome coin at 1 in 131,072 and
                    // `string.random.word` far beyond counting.
                    const RECHECKS: usize = 16;
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
                            // **F163: ask again before calling it an
                            // observer.** `a1` and `a2` are the only evidence
                            // so far that this native is deterministic, and
                            // for a native with *no operands* there is one
                            // tuple, so that is one comparison in the whole
                            // audit. `string.random.word` draws `et` 4.8% of
                            // the time; two draws coincide in 0.95% of runs,
                            // the native then looks deterministic, and the
                            // difference below is read as observation --
                            // failing this test about once in 105 for a
                            // reason that is not in the code under test.
                            //
                            // So the identical run is repeated here, where it
                            // is cheap: only a native about to be flagged
                            // pays, and a real observer agrees with itself
                            // every time and is flagged exactly as before.
                            if !nondet && (0..RECHECKS).any(|_| !a1.same(&run(&pad_a))) {
                                nondet = true;
                            }
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
                 # test -p bund2-stdlib promotable; never edited by hand.\n\
                 #\n\
                 # **One file, two builds — F123.** A native behind a Cargo feature is\n\
                 # registered in one build and not the other, so no content here could\n\
                 # be true for both: listed, the default build reports it `no longer\n\
                 # reached`; absent, `--all-features` reports it `now reached`. Such\n\
                 # natives are excluded from the audit in both builds (`FEATURE_GATED`,\n\
                 # crates/bund2-stdlib/src/lib.rs) and named below. Unlisted is the\n\
                 # conservative answer: promotion syncs before them as before an\n\
                 # embedder's native, which costs speed and never meaning.\n",
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
            // **The feature-gated names the header promises** — F123. Written
            // whichever build regenerates the file, because the exclusion is a
            // property of the audit rather than of the build that ran it: a
            // reader must be able to see what was withheld without opening
            // `lib.rs`.
            out.push_str(
                "#\n# Excluded in both builds because a Cargo feature decides whether they\n\
                 # exist at all (F123); never crossed by promotion:\n",
            );
            for n in FEATURE_GATED {
                out.push_str(&format!("# feature-gated: {n}\n"));
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
                // **Comments are skipped, as D71's sibling scan already does.**
                // The claim is about code; a doc comment *explaining* why a
                // native does not report at `Error` severity names the thing
                // it is promising not to do, and the first such comment —
                // `logging.rs`'s, on D90's `log.error` mapping to `Warning` —
                // was reported as a violation. A comment cannot report
                // anything, so this is strictly more precise rather than
                // weaker.
                let t = line.trim_start();
                if t.starts_with("//") {
                    continue;
                }
                if t.contains("Diagnostic::error") || t.contains("Severity::Error") {
                    found.push(format!("{}:{}", path.display(), n + 1));
                }
            }
        }
        assert!(found.is_empty(), "reports at Error severity: {found:?}");
    }

    /// **D71's set: every native that reports a `Warning` or `Notice`
    /// mid-body** — F137, RFC-0005 §S5's reporter rule.
    ///
    /// A crossed call holds values in registers across it, and
    /// `Interp::report` snapshots the stack when the reporter wants one for
    /// that severity — so a native reporting mid-body while a crossing is live
    /// would show a **short stack**, which is Q34's shape. D71 keeps such a
    /// native out of the crossable table rather than reading the reporter, and
    /// this is what stops that table going stale: shipped code that gains a
    /// sixth report site fails here until it is named, and whoever names it has
    /// to decide whether it is reachable as a crossed call.
    ///
    /// **Being on this list is not the same as being excluded.** Four of the
    /// five are already unreachable as a crossed call — `while`, `for` and
    /// `*loop` are `StackEffect::opaque`, and `run_error` is a conditional arm
    /// run by `!`, which is opaque too. Only `alias` needs
    /// `promotable::REPORTS_MID_BODY`, and
    /// `a_native_that_reports_mid_body_is_not_crossable` asserts that end.
    ///
    /// A source scan, cut at each file's test module as criterion 25's is, and
    /// descending into subdirectories as criterion 11's does.
    #[test]
    fn every_native_reporting_mid_body_is_named() {
        const REPORTS: [&str; 10] = [
            // A notice, when a TRY block left an error the EXCEPT arm runs.
            "conditional.rs: run_error",
            // A warning, when the exit code is not an integer and 0 is taken
            // (F178). `StackEffect::opaque`, so no crossing reaches it.
            "host.rs: bund_exit",
            "control.rs: for_base",
            "control.rs: while_base",
            "seq.rs: loop_over_base",
            // The one that matters: `eff(2, 0)`, certified by the palette, and
            // refused a crossing only by D71's gate.
            // D90: the five `log.*` words all construct their diagnostic here.
            "logging.rs: diagnostic",
            "singles.rs: alias",
            // The two debug REPLs report a failing line and keep going -- the
            // reference prints it and keeps going, and D36 says a word reports
            // rather than printing. `Warning`, because the session has not
            // stopped.
            "terminal.rs: debug_shell",
            "terminal.rs: debug_word",
            // Its whole reachable behaviour: Bund2 has no `--distributed`, so the
            // word reports that and returns, as the reference logs and returns.
            "terminal.rs: debug_display_distributed_info",
        ];
        const QUALIFIERS: [&str; 5] = ["pub", "const", "unsafe", "async", "extern"];
        const REPORTS_AT: [&str; 2] = ["Diagnostic::warning", "Diagnostic::notice"];
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
                    if REPORTS_AT.iter().any(|m| line.contains(m)) {
                        found.insert(format!("{file}: {current}"));
                    }
                }
            }
        }
        let named: std::collections::BTreeSet<String> =
            REPORTS.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(
            found, named,
            "D71's mid-body reporting set and what shipped code actually reports differ"
        );
    }

    /// **D73's set: every native that changes which stack is current** — F140.
    ///
    /// Promotion syncs by *pushing*, and a push goes to whatever stack is
    /// current at that moment. A callee that changes the current stack while
    /// values are held in registers therefore moves them — the body's final
    /// sync lands them on the stack in force after the call, where Tier 0 put
    /// them before it. F140 measured that: `:w { 1 2 + stacks_left }` left one
    /// value on the wrong stack at the shipped threshold.
    ///
    /// `promotable::SWITCHES_STACK` keeps these out of the crossable table, and
    /// this is what stops that list going stale: shipped code that gains a
    /// ninth switcher fails here until someone names it and decides whether it
    /// is reachable as a crossed call.
    ///
    /// A source scan, cut at each file's test module as criterion 25's is, and
    /// descending into subdirectories as criterion 11's does.
    #[test]
    fn every_native_that_changes_the_current_stack_is_named() {
        const SWITCHES: [&str; 8] = [
            // Restores the stack it came from, but a scan cannot tell that and
            // the gate does not lean on it.
            "conditional.rs: endcontext",
            "stack.rs: rotate_stack_left",
            "stack.rs: rotate_stack_right",
            // The two that F140 is actually about: `eff(0, 0)`, so a crossing
            // syncs nothing before them.
            "stack.rs: stacks_left",
            "stack.rs: stacks_right",
            "stack.rs: swap_in",
            "stack.rs: to_current",
            "stack.rs: to_stack",
        ];
        const QUALIFIERS: [&str; 5] = ["pub", "const", "unsafe", "async", "extern"];
        const SWITCHERS: [&str; 3] = [
            ".to_stack(",
            ".rotate_stacks_left(",
            ".rotate_stacks_right(",
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
                    if SWITCHERS.iter().any(|m| line.contains(m)) {
                        found.insert(format!("{file}: {current}"));
                    }
                }
            }
        }
        let named: std::collections::BTreeSet<String> =
            SWITCHES.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(
            found, named,
            "D73's stack-switching set and what shipped code actually switches differ"
        );
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
        const REENTERING: [&str; 28] = [
            // RFC-0008 §W2: a line typed at a debugger stop, applied value by
            // value. Called from a safepoint and never from a native.
            "lib.rs: eval_line",
            "conditional.rs: run_context",
            "conditional.rs: run_error",
            "conditional.rs: run_ifthenelse",
            "conditional.rs: run_through",
            "conditional.rs: run_tryexcept",
            "control.rs: for_base",
            "control.rs: while_base",
            "data.rs: run_csv",
            "data.rs: run_sqlite",
            // `True` and `False` apply a CALL to `object`, as the reference
            // does, so a rebound `object` reaches them — which is exactly why
            // it re-enters rather than calling `object_word` directly.
            "oop.rs: bool_object",
            "oop.rs: dispatch_method",
            // `List`, `Floats` and `Intervals` apply a CALL to `object`, as
            // `bool_object` does and for its reason: a rebound `object` must
            // reach them too (RFC-0010 S1).
            "oop.rs: empty_of",
            "oop.rs: run_init",
            "seq.rs: loop_base",
            "seq.rs: loop_over_base",
            "seq.rs: map_base",
            "seq.rs: times_base",
            "singles.rs: apply",
            "singles.rs: conditional_move",
            // `do` runs its lambda in a loop until the stack empties, and
            // `resolve` applies a PTR to whatever it found.
            "singles.rs: do_base",
            "singles.rs: eval_source",
            "singles.rs: resolve_word",
            // `debug.run` is `bund.eval` with a safepoint offered before each
            // term (D113.5).
            "terminal.rs: debug_run",
            // `debug` applies each value it steps over, so a stepped word can do
            // anything the language can -- which is the point of the word.
            "terminal.rs: debug_word",
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
