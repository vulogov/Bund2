#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
//! The bund2 command line runner.
//!
//! **A slice.** It parses a program with `bund2-syntax`, lowers it to a value
//! stream, and hands that to `bund2-interp`'s single evaluator — no REPL.
//! `script --file` is what `cargo xtask conform` invokes; `build` writes a
//! bundle and `build --inspect` reads one (RFC-0006), and `words`, `effects`,
//! `infer` and `check` serve the xtasks and RFC-0004. RFC-0003's frame loop
//! replaces the evaluator's middle; the front end is now the real one.

use std::process::ExitCode;

mod bundle;
mod debugger;
mod help;
mod profile;
mod trace;
#[macro_use]
mod ui;

/// Which oracle this binary was conformed against — RFC-0006 §B1.
///
/// Baked in at **compile** time, because the artefact must carry it and
/// `bund2 build` may run anywhere. A change to the pinned submodules rebuilds
/// this, which is the intent: a bundle's recorded SHAs then differ from an
/// older one's, and the two are distinguishable rather than silently merged.
const PINNED: &str = include_str!("../../../reference/PINNED.txt");

/// The cargo features this runtime carries, as §B4 requires.
///
/// Compile-time `cfg`, since that is the only thing that knows. An empty
/// string is the default build and is not the same as "unknown".
fn features_built_in() -> String {
    let mut on: Vec<&str> = Vec::new();
    if cfg!(feature = "jit") {
        on.push("jit");
    }
    // `relocation-test` is not recorded: it compiles a test and nothing a
    // bundle carries. As `aot` it was, and a bundle said so of itself (D134).
    // Nor is `async`: this crate has no such feature, since nothing here
    // runs the host (D168).
    // **Asked of the crate that owns the feature.** `grok` is `bund2-stdlib`'s
    // and this crate does not declare it, so a `cfg!` here would always be
    // false and a runtime carrying `string.grok` would record itself as a
    // default build — the very example §B4 gives for why the record exists.
    if bund2_stdlib::GROK_BUILT_IN {
        on.push("grok");
    }
    on.join(",")
}

/// `reference/PINNED.txt` compacted to fit the container: each submodule's
/// basename with its SHA's first twelve characters.
fn pinned_summary() -> String {
    PINNED
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let sha = f.next()?;
            // **Only SHA lines.** `PINNED.txt` ends with a human line naming
            // the oracle's version — "bund 0.22.0, built …" — which parsed as
            // a submodule and produced the entry `0.22.0,:bund`. A summary a
            // reader is meant to trust cannot carry a field it misread.
            if sha.len() != 40 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let path = f.next()?;
            let name = path.rsplit('/').next()?;
            Some(format!("{name}:{}", sha.get(..12).unwrap_or(sha)))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

use bund2_api::Vm;
use bund2_interp::Interp;

/// **Run everything on a thread whose stack Bund2 chose — RFC-0005 §S8, F85.**
///
/// The main thread's stack is the operating system's to size, so Bund2 cannot
/// know where it ends. A thread it spawns, it can: the size is the one it asked
/// for, and the top is where the thread's entry function starts. From those
/// two numbers every `Interp` on the thread takes a floor, and recursion that
/// reaches it is reported as a Bund-level error instead of aborting the
/// process. Output, input and the exit code all pass straight through.
///
/// **The OOM guard runs beside it — D171.** A thread samples the host and,
/// when this process is driving it to critical memory pressure, trips the
/// flag the interpreter reads at its next call, where the program fails with
/// an ordinary run-time error. It starts before the program and is stopped
/// when the program's thread returns. `BUND2_OOM_GUARD_GB=0` disables it.
fn main() -> ExitCode {
    // **A panic that escapes is explained, not dumped — D173.** Bund2 does
    // not panic (D37), and a native's dependency that does is caught and
    // reported (D49). Anything that still reaches the top gets `human-panic`'s
    // hook: a report file with the message and the backtrace, and a few lines
    // saying where it is and what to do with it, instead of a Rust backtrace
    // the reader cannot act on. Installed first, so the quiet hook D49 wraps
    // around it finds it, and inert in a debug build or under `RUST_BACKTRACE`,
    // where the plain backtrace is wanted.
    human_panic::setup_panic!(
        human_panic::Metadata::new("bund2", env!("CARGO_PKG_VERSION"))
            .homepage(env!("CARGO_PKG_REPOSITORY"))
            .support("report it with the program that produced it, at the repository's issues")
    );
    let _oom_guard =
        bund2_host::memwatch::Guard::start(bund2_host::memwatch::Policy::from_env(), |t| {
            bund2_api::memguard::trip(bund2_api::memguard::Trip {
                resident: t.resident,
                available: t.available,
                swap_free: t.swap_free,
            })
        });
    let spawned = std::thread::Builder::new()
        .name("bund2".into())
        .stack_size(bund2_runtime::EVAL_STACK)
        .spawn(|| {
            // **A share is declared, never inferred** — RFC-0005 §S8, the
            // ninth review's S3. `declare_region` is the one place that does
            // it, now that RFC-0007 §C8's host spawns threads of its own: two
            // copies of three constants and a two-line call would be two
            // chances to omit the share, which is the failure criterion 18 and
            // `compiled_entries` exist to catch.
            bund2_runtime::declare_region();
            run_cli()
        });
    match spawned {
        // A panic is not a path this program has (D37); if one ever reached
        // here, failing is the only honest exit code.
        Ok(handle) => handle.join().unwrap_or(ExitCode::FAILURE),
        Err(e) => {
            bund2_api::errln!("bund2: could not start the evaluation thread: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_cli() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // **Is this binary a bundle? — RFC-0006 §B1.** Asked of its own image, not
    // of a file: the region is a `static`, so a bundle never has to find its
    // own executable to know what it is.
    //
    // **Before every other arm, including `build`.** A bundle is the program,
    // so all of argv belongs to it (§B3) — a bundle whose program's first
    // argument happened to be `words` must not print the word table.
    match bundle::carried() {
        Ok(Some(carried)) => return run_carried(&carried, args),
        Ok(None) => {}
        Err(damaged) => {
            bund2_api::errln!("bund2: {damaged}");
            return ExitCode::FAILURE;
        }
    }

    // **How loud — D173.** The flags are taken from wherever they stand
    // before `--`, for every command alike; a bundle never reaches here, and
    // reads `BUND2_VERBOSE` in `run_carried`. A level given on the command
    // line also sets the threshold the program's `log.*` words emit at, as the
    // reference's `--debug` count did.
    let args = match ui::take_verbosity(args) {
        Ok((args, level)) => {
            if let Some(level) = level {
                ui::set_level(level);
                if let Err(e) = bund2_stdlib::logging::set_threshold(level.name()) {
                    ui::warning!("{e}");
                }
            }
            args
        }
        Err(e) => {
            ui::error!("{e}");
            return ExitCode::from(2);
        }
    };
    ui::configure_colors(false);

    // **`help`, `--help`, `-h`, `--version` — D173.** Asked before any
    // command parses, so `bund2 build --help` is help and not a build with
    // a bad flag. No arguments at all is a mistake, told as one: the short
    // usage on standard error and exit 2, as any other bad command line.
    match help::asked(&args) {
        Some(help::Asked::Usage) => {
            bund2_api::say!("{}", help::USAGE);
            return ExitCode::SUCCESS;
        }
        Some(help::Asked::Command(text)) => {
            bund2_api::say!("{text}");
            return ExitCode::SUCCESS;
        }
        Some(help::Asked::Version) => {
            bund2_api::say!("{}", help::version(&features_built_in(), &pinned_summary()));
            return ExitCode::SUCCESS;
        }
        Some(help::Asked::Unknown(name)) => {
            ui::error!(
                "`{name}` is not a command. The commands are script, build, check, infer, words and effects."
            );
            return ExitCode::from(2);
        }
        None => {}
    }
    if args.is_empty() {
        ui::error!("expected a command: bund2 script --file <path>, or bund2 --help");
        return ExitCode::from(2);
    }

    // `bund2 build --file <src> --output <path>` — RFC-0006.
    if args.first().map(String::as_str) == Some("build") {
        return build(&args);
    }
    // `bund2 words` — every name a program can call, one per line.
    //
    // Not a language feature: it is the join key `cargo xtask coverage` needs.
    // Coverage used to count in-scope words the *corpus mentioned*, which is a
    // property of the corpus and could not move as words landed — it read
    // 121/497 through forty words arriving in one session. Asking the binary
    // what it registers is the only answer that is about Bund2.
    if args.first().map(String::as_str) == Some("words") {
        let mut i = Interp::new();
        bund2_stdlib::register_all(&mut i.registry);
        for name in i.registry.word_names() {
            bund2_api::sayln!("{name}");
        }
        return ExitCode::SUCCESS;
    }
    // `bund2 effects` — every native's declared arity, for `xtask effects`.
    if args.first().map(String::as_str) == Some("effects") {
        let mut i = Interp::new();
        bund2_stdlib::register_all(&mut i.registry);
        for (name, e) in i.registry.declared_effects() {
            bund2_api::sayln!("{name}\t{}\t{}", e.consumes, e.produces);
        }
        return ExitCode::SUCCESS;
    }
    // `bund2 infer --file <path>` — RFC-0004 §S3 and criterion 6.
    //
    // Runs the program so its `register` calls happen, then for every word it
    // registered prints the **inferred** effect beside the one **observed** by
    // calling the word against a stack of sentinels. The two must agree or the
    // inference must say `opaque`; a wrong number is the failure this exists
    // to catch.
    if args.first().map(String::as_str) == Some("infer") {
        let Some(file) = args.iter().skip_while(|a| *a != "--file").nth(1).cloned() else {
            bund2_api::errln!("bund2: expected: bund2 infer --file <path>");
            return ExitCode::from(2);
        };
        let Ok(src) = std::fs::read_to_string(&file).map(without_shebang) else {
            bund2_api::errln!("bund2: reading {file}");
            return ExitCode::from(2);
        };
        return run_infer(&src, &file);
    }

    // `bund2 check --file <path>` — RFC-0004 §S4.
    //
    // Reports where a program would underflow, and how much of it could not be
    // analysed. It never runs the program and never changes what `script`
    // does; a program it warns about still runs and still fails as it did.
    if args.first().map(String::as_str) == Some("check") {
        let Some(file) = args.iter().skip_while(|a| *a != "--file").nth(1).cloned() else {
            bund2_api::errln!("bund2: expected: bund2 check --file <path>");
            return ExitCode::from(2);
        };
        let Ok(src) = std::fs::read_to_string(&file).map(without_shebang) else {
            bund2_api::errln!("bund2: reading {file}");
            return ExitCode::from(2);
        };
        return run_check(&src, &file);
    }
    // **A command that is not one is told so — D173**, with the list, and a
    // path on its own runs: `bund2 p.bund` is `bund2 script --file p.bund`,
    // the shortcut every interpreter's command line has.
    let args = match args.first().map(String::as_str) {
        Some("script") => args,
        Some(flag) if flag.starts_with('-') => args,
        Some(path) if std::path::Path::new(path).is_file() => {
            let mut run = vec!["script".to_string()];
            run.extend(args.iter().cloned());
            run
        }
        Some(other) => {
            ui::error!(
                "`{other}` is not a command, and not a file. The commands are script, \
                 build, check, infer, words and effects; `bund2 --help` lists them."
            );
            return ExitCode::from(2);
        }
        None => args,
    };
    let args = match parse_args(&args) {
        Ok(f) => f,
        Err(e) => {
            bund2_api::errln!("bund2: {e}");
            return ExitCode::from(2);
        }
    };
    let src = match std::fs::read_to_string(&args.file).map(without_shebang) {
        Ok(s) => s,
        Err(e) => {
            bund2_api::errln!("bund2: reading {}: {e}", args.file);
            return ExitCode::from(2);
        }
    };
    bund2_stdlib::host::set_args(args.script_args.clone());
    // Exit 0 unless the program asked otherwise: a Bund failure is reported,
    // not signalled. Only `bund.exit` (D52), a failure to *start* — bad
    // arguments, an unreadable file — and a program that does not parse
    // (D194) set an exit code.
    match run(&src, &args) {
        // Unix keeps the low 8 bits of `exit`'s argument, and the reference
        // passes its code straight to `process::exit`.
        Some(code) => ExitCode::from((code & 0xff) as u8),
        None => ExitCode::SUCCESS,
    }
}

struct Args {
    file: String,
    /// `--noio`: the I/O words fail instead of touching the host
    /// (`reference/Bund/src/cmd/mod.rs:145-146`).
    host: bund2_stdlib::host::HostOptions,
    /// Everything after `--`, which `args` answers with. The reference takes
    /// them the same way (`reference/Bund/src/cmd/mod.rs:233-234`).
    script_args: Vec<String>,
    /// Whether an error report carries the stack. On by default, as the
    /// reference always dumps; `--no-dump-stack` is for a caller that wants
    /// the reason alone.
    dump_stack: bool,
    /// Show raw `Debug` values in the dump. For a debug session; the default
    /// is a compact summary that fits the terminal.
    raw_values: bool,
    /// `--debugger`: stop before every value and take commands on stdin —
    /// RFC-0008 §D1. **It also declines the tier** (§D6): a compiled body runs
    /// without entering the loop that carries the safepoint, so a debugged
    /// session with a tier would step over exactly the bodies a program spends
    /// its time in.
    debugger: bool,
    /// `--debug-shell`: after a failure has been reported, run `debug.shell`
    /// on the VM the program left — D156. The reference does it on each of
    /// its paths
    /// (`reference/Bund/src/stdlib/helpers/run_snippet.rs:31-33,51-53,78-80,89-91`).
    debug_shell: bool,
    /// The program came out of this executable (RFC-0006) rather than from
    /// `--file`. **A bundle's debugger words do nothing** — D115.
    carried: bool,
    /// `--stats`: report what the tier did, on **stderr** — RFC-0005
    /// criterion 2's statistics flag.
    ///
    /// The criterion asks for it so a `jit` run cannot pass vacuously: without
    /// it the only evidence a tier ran is timing, which cannot tell a compiled
    /// body from a fast interpreted one. F124 is what that gap allowed.
    stats: bool,
    /// `--profile <base>`: write `<base>.profile.md` and `<base>.profile.dot`
    /// when the program has ended — RFC-0011, D185. The operand is required,
    /// so the reference's bare `--profile` is still refused. A profiled run
    /// installs no tier (D186). A bundle reads `BUND2_PROFILE` instead.
    profile: Option<String>,
    /// `--trace <file>`: one line per call entered, indented by depth, into
    /// `<file>` — RFC-0008 §D7, D191. Kept by the profiler's record, so a
    /// traced run installs no tier either. A bundle reads `BUND2_TRACE`.
    trace: Option<String>,
    /// `--jit-threshold <n>`: how many evaluations of one body earn it
    /// compilation — §S7's knob, and what RFC-0005 criterion 2's third run
    /// needs (F125).
    ///
    /// `None` falls back to `BUND2_JIT_THRESHOLD`, then to D74's default of 1024.
    /// The flag wins over the environment because it is the more specific
    /// statement: a command line is about *this* run.
    ///
    /// Accepted, and ignored, in a build without the `jit` feature — there is
    /// no tier to configure, and a uniform `conform` command line must not fail
    /// on the run that has none.
    jit_threshold: Option<u32>,
    /// `--report auto|box|trace`: the reference's table or `ariadne`'s
    /// traceback for a fatal report — D173. `auto` draws the traceback on a
    /// terminal and the table on a pipe.
    report: trace::Mode,
}

impl Args {
    /// Why this run installs no tier though the build may have one: the
    /// debugger (RFC-0008 §D6) or the profiler (D186). `None` when the tier
    /// is offered.
    fn tier_declined(&self) -> Option<&'static str> {
        if self.debugger {
            Some("debugging")
        } else if self.profile.is_some() {
            Some("profiling")
        } else if self.trace.is_some() {
            Some("tracing")
        } else {
            None
        }
    }
}

/// `script --file <path> [-- <args>…]`, the shape `conform` and the oracle
/// share.
fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut it = args.iter();
    let mut file = None;
    let mut dump_stack = true;
    let mut raw_values = false;
    let mut stats = false;
    let mut profile: Option<String> = None;
    let mut trace: Option<String> = None;
    let mut jit_threshold: Option<u32> = None;
    let mut debugger = false;
    let mut debug_shell = false;
    let mut report = trace::Mode::from_env();
    let mut host = bund2_stdlib::host::HostOptions::default();
    let mut script_args = Vec::new();
    while let Some(a) = it.next() {
        // `--flag=value` and `--flag value` alike (D173), as `build` takes
        // them; a bare path is the file, so `bund2 script p.bund` runs it.
        let (name, inline) = match a.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n, Some(v.to_string())),
            _ => (a.as_str(), None),
        };
        let mut operand = |what: &str| -> Result<String, String> {
            inline
                .clone()
                .or_else(|| it.next().cloned())
                .ok_or_else(|| format!("`{name}` needs {what}; see `bund2 help script`"))
        };
        match name {
            "script" => {}
            "--report" => {
                let v = operand("a rendering: auto, box or trace")?;
                report = trace::Mode::parse(&v)?;
            }
            "--file" => file = Some(operand("a path")?),
            "--dump-stack" => dump_stack = true,
            "--no-dump-stack" => dump_stack = false,
            "--raw-values" | "--debug-values" => raw_values = true,
            "--stats" => stats = true,
            // **D185: the operand is required.** A bare `--profile`, the
            // reference's shape, is refused with the operand message; so is
            // one whose "operand" is the next flag, which the shared closure
            // would otherwise take as a path and write two files beside.
            "--profile" => {
                let base = operand("a path for the two files")?;
                if base.is_empty() || base.starts_with('-') {
                    return Err(format!(
                        "`--profile` needs a path for the two files, not `{base}`; see `bund2 help script`"
                    ));
                }
                profile = Some(base);
            }
            // D191: required, as `--profile`'s is, and for the same reason.
            "--trace" => {
                let path = operand("a file for the trace")?;
                if path.is_empty() || path.starts_with('-') {
                    return Err(format!(
                        "`--trace` needs a file for the trace, not `{path}`; see `bund2 help script`"
                    ));
                }
                trace = Some(path);
            }
            "--debugger" => debugger = true,
            "--debug-shell" => debug_shell = true,
            // Refused rather than ignored when it is not a number: this is a
            // command line, which can report. `BUND2_JIT_THRESHOLD` cannot —
            // the runtime reads it in a constructor — so that one is ignored
            // when malformed and `--stats` prints what was actually adopted.
            "--jit-threshold" => {
                let v = operand("a number, as in `--jit-threshold 1`")?;
                jit_threshold = Some(
                    v.parse::<u32>()
                        .map_err(|_| format!("--jit-threshold takes a number, not `{v}`"))?,
                );
            }
            "--noio" => host.noio = true,
            // `reference/Bund/src/cmd/mod.rs:139-140`.
            "--nocolor" => host.nocolor = true,
            // `reference/Bund/src/cmd/mod.rs:142-143`.
            "--noeval" => host.noeval = true,
            "--" => {
                script_args = it.by_ref().cloned().collect();
            }
            // **A bare path ends the flags (D173)**: what follows it is the
            // program's, as `#!/usr/bin/env bund2` passes it and as every
            // interpreter's command line reads it. A `--` right after the
            // path is the separator and not an argument.
            path if !path.starts_with('-') && file.is_none() => {
                file = Some(path.to_string());
                let mut rest = it.by_ref().cloned().peekable();
                if rest.peek().is_some_and(|a| a == "--") {
                    rest.next();
                }
                script_args = rest.collect();
            }
            other => {
                return Err(format!(
                    "unknown argument `{other}`; `bund2 help script` lists the flags"
                ));
            }
        }
    }
    Ok(Args {
        file: file
            .ok_or("expected: bund2 script --file <path>, or bund2 <path>; see `bund2 --help`")?,
        host,
        script_args,
        dump_stack,
        raw_values,
        stats,
        profile,
        trace,
        debugger,
        debug_shell,
        carried: false,
        jit_threshold,
        report,
    })
}

/// Check inference against a run, for every word a program registers.
///
/// **The observation is the hard half.** A word's depth delta is only
/// meaningful if the word runs, and running it needs operands — so each is
/// called against a stack of integer sentinels deep enough for its inferred
/// floor, and the delta is what the stack lost or gained. A word that fails on
/// integers (it wanted a string, a list, a lambda) is reported as unobservable
/// rather than counted as agreeing: an unrun word is evidence of nothing, and
/// folding it into a pass is how `xtask effects`'s "not in the table" bucket
/// would have gone vacuous.
fn run_infer(src: &str, file: &str) -> ExitCode {
    let mut vm = Interp::new();
    bund2_stdlib::register_all(&mut vm.registry);
    let Ok(stream) = bund2_syntax::compile(src) else {
        bund2_api::errln!("bund2: {file} does not parse");
        return ExitCode::from(2);
    };
    // The program's own `register` calls are what put words in the table, so
    // it has to run. Its failure is not this command's to report as the
    // program's, but it is said (D173): a program that failed before its
    // `register` calls would otherwise show fewer words with no sign why.
    if let Err(e) = vm.eval(&stream) {
        ui::warning!(
            "{file} failed while running, so some words may be missing: {}",
            e.0
        );
    }

    let mut names: Vec<String> = vm.registry.lambda_names();
    names.sort();
    let (mut agree, mut opaque, mut unobservable, mut wrong) = (0, 0, 0, 0);
    for name in &names {
        let Some(body) = vm.registry.lambda_body(name) else {
            continue;
        };
        let inferred = bund2_stdlib::check::infer(&body, &vm.registry, 0);
        if inferred.opaque {
            opaque += 1;
            bund2_api::sayln!("  {name:<28} inferred opaque");
            continue;
        }
        match observe(&vm.registry, name, inferred.consumes) {
            None => {
                unobservable += 1;
                bund2_api::sayln!(
                    "  {name:<28} inferred {}->{}   not observable on integer sentinels",
                    inferred.consumes,
                    inferred.produces
                );
            }
            Some(delta) => {
                let want = i64::from(inferred.produces) - i64::from(inferred.consumes);
                if want == delta {
                    agree += 1;
                } else {
                    wrong += 1;
                    bund2_api::sayln!(
                        "  {name:<28} inferred {}->{} (net {want}) but ran net {delta}",
                        inferred.consumes,
                        inferred.produces
                    );
                }
            }
        }
    }
    bund2_api::sayln!(
        "{file}: {} word(s) — {agree} agree, {opaque} opaque, {unobservable} unobservable, {wrong} WRONG",
        names.len()
    );
    if wrong > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Run one registered word against sentinels and report its depth delta.
fn observe(registry: &bund2_api::Registry, name: &str, floor: u8) -> Option<i64> {
    let mut vm = Interp::new();
    // A fresh VM with the same vocabulary, so the word under test cannot be
    // disturbed by what an earlier one left.
    vm.registry = registry.clone();
    for i in 0..floor.max(1) {
        vm.push(bund2_value::BundValue::int(i64::from(i)));
    }
    let before = vm.depth() as i64;
    let body = registry.lambda_value(name)?;
    vm.eval_lambda(&body).ok()?;
    Some(vm.depth() as i64 - before)
}

/// Analyse a program and report, without running it.
///
/// Exit 0 whether or not anything is found. **`check` does not gate**:
/// RFC-0004's alternatives section rejects failing a build on it, because `!`
/// is `Opaque` and one of the five most-used words in the corpus, so a false
/// positive is certain and a checker that blocks gets turned off.
fn run_check(src: &str, file: &str) -> ExitCode {
    let lowered = match bund2_syntax::parse(src) {
        Ok(terms) => bund2_syntax::lower_with_spans(&terms, src.len()),
        Err(e) => {
            bund2_api::errln!("{}", e.render(src));
            return ExitCode::from(2);
        }
    };
    let mut i = Interp::new();
    bund2_stdlib::register_all(&mut i.registry);
    let report = bund2_stdlib::check::check(&lowered.values, &i.registry, 0);

    for f in &report.findings {
        let where_ = lowered
            .span_of(f.at)
            .map(|sp| locate(src, file, sp))
            .unwrap_or_else(|| bund2_api::diag::Location {
                file: Some(file.to_string()),
                line: 0,
                column: 0,
                excerpt: None,
                span: None,
            });
        bund2_api::errln!(
            "Warning: {}:{}:{}: `{}` needs {} value(s); {} can be proven here{}",
            where_.file.as_deref().unwrap_or(file),
            where_.line,
            where_.column,
            f.word,
            f.needs,
            f.have,
            via_of(f)
        );
    }
    // F62's hazard: not an underflow, so not counted among the findings.
    for (at, what) in &report.hazards {
        let where_ = lowered.span_of(*at).map(|sp| locate(src, file, sp));
        bund2_api::errln!(
            "Warning: {}:{}:{}: {what}",
            file,
            where_.as_ref().map_or(0, |l| l.line),
            where_.as_ref().map_or(0, |l| l.column)
        );
    }

    // **Criterion 5**: say how much was skipped, always — including when
    // nothing was found, which is exactly when a silent report misleads.
    bund2_api::sayln!(
        "checked {}: {} finding(s), {} site(s) analysed, {} abandoned",
        file,
        report.findings.len(),
        report.analysed,
        report.abandoned
    );
    if !report.reasons.is_empty() {
        bund2_api::sayln!("  analysis stopped at:");
        for (why, n) in &report.reasons {
            bund2_api::sayln!("    {n:>4}  {why}");
        }
        bund2_api::sayln!(
            "  A finding is only ever about the {} site(s) above that were",
            report.analysed
        );
        bund2_api::sayln!("  tracked. Nothing is claimed about the rest.");
    }
    ExitCode::SUCCESS
}

/// Where a finding inside a body was reached from — D179. A finding in a
/// lambda `!` runs, or in a method or constructor, is located at the site
/// that ran it, and this names the body so the line is not read as the
/// site's own arity.
fn via_of(f: &bund2_stdlib::check::Finding) -> String {
    match &f.via {
        Some(v) => format!(", inside {v}"),
        None => String::new(),
    }
}

/// What the build's underflow analysis says — D177.
struct Underflow {
    /// One line per finding, `file:line:col: …`, in stream order.
    findings: Vec<String>,
    /// F62's hazards, in the same form. Warnings, never a `--strict` refusal.
    hazards: Vec<String>,
    /// The one-line count, `check`'s own wording.
    summary: String,
    /// Where the analysis stopped, for `--check`'s report.
    detail: Vec<String>,
}

/// `bund2 check`'s analysis over several files as one stream, the includes
/// first — D177. Each finding is located in the file it falls in.
fn underflow_over(files: &[(&str, &str, bund2_syntax::Lowered)]) -> Underflow {
    let mut stream = Vec::new();
    let mut starts = Vec::with_capacity(files.len());
    for (_, _, lowered) in files {
        starts.push(stream.len());
        stream.extend(lowered.values.iter().cloned());
    }
    let mut i = Interp::new();
    bund2_stdlib::register_all(&mut i.registry);
    let report = bund2_stdlib::check::check(&stream, &i.registry, 0);
    // The file whose range holds a site, the last start at or before it, and
    // the site's line and column there.
    let place = |at: usize| {
        let k = starts.iter().rposition(|s| *s <= at).unwrap_or(0);
        let (path, text, lowered) = &files[k];
        let (line, column) = lowered
            .span_of(at - starts[k])
            .map(|sp| {
                let l = locate(text, path, sp);
                (l.line, l.column)
            })
            .unwrap_or((0, 0));
        (*path, line, column)
    };
    let hazards = report
        .hazards
        .iter()
        .map(|(at, what)| {
            let (path, line, column) = place(*at);
            format!("{path}:{line}:{column}: {what}")
        })
        .collect();
    let findings = report
        .findings
        .iter()
        .map(|f| {
            let (path, line, column) = place(f.at);
            format!(
                "{path}:{line}:{column}: `{}` needs {} value(s); {} can be proven here{}",
                f.word,
                f.needs,
                f.have,
                via_of(f)
            )
        })
        .collect();
    let what = if files.len() == 1 {
        files[0].0.to_string()
    } else {
        format!("{} files", files.len())
    };
    let summary = format!(
        "checked {what}: {} finding(s), {} site(s) analysed, {} abandoned",
        report.findings.len(),
        report.analysed,
        report.abandoned
    );
    let mut detail = Vec::new();
    if !report.reasons.is_empty() {
        detail.push("  analysis stopped at:".to_string());
        for (why, n) in &report.reasons {
            detail.push(format!("    {n:>4}  {why}"));
        }
        detail.push(format!(
            "  A finding is only ever about the {} site(s) above that were",
            report.analysed
        ));
        detail.push("  tracked. Nothing is claimed about the rest.".to_string());
    }
    Underflow {
        findings,
        hazards,
        summary,
        detail,
    }
}

/// A program's text with a leading `#!` line blanked — D173, so that
/// `#!/usr/bin/env bund2` at the top of an executable file runs it. The
/// line's bytes go and its newline stays, so every later position is what
/// the file says. The reference took `#!/usr/bin/env` for a word and
/// failed to evaluate it (measured 2026-10-15), so no program that ran
/// there reads differently here.
fn without_shebang(src: String) -> String {
    if !src.starts_with("#!") {
        return src;
    }
    match src.find('\n') {
        Some(end) => src[end..].to_string(),
        None => String::new(),
    }
}

/// Turn a byte offset into a position a programmer can act on.
fn locate(src: &str, file: &str, span: bund2_syntax::Span) -> bund2_api::diag::Location {
    let start = span.start.min(src.len());
    let line = src[..start].bytes().filter(|b| *b == b'\n').count() + 1;
    let line_start = src[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let column = src[line_start..start].chars().count() + 1;
    let excerpt = src[line_start..]
        .split('\n')
        .next()
        .map(str::to_string)
        .filter(|l| !l.trim().is_empty());
    bund2_api::diag::Location {
        file: Some(file.to_string()),
        line,
        column,
        excerpt,
        span: Some((span.start, span.end)),
    }
}

/// Run a program, reporting anything that goes wrong through the reporter.
///
/// Run the program this binary carries — RFC-0006 §B1 and §B3.
///
/// **All of argv reaches the program.** None of the CLI's own flags is read
/// here: a bundle that swallowed `--stats` would shadow one of its program's
/// arguments. The diagnostic flags come from the environment instead
/// (§B3), and so do D78's restrictions.
fn run_carried(carried: &bundle::Carried, argv: Vec<String>) -> ExitCode {
    // **D78: the trailer is a floor and the environment may only tighten it.**
    // An environment variable that could clear a restriction would not be a
    // restriction, and the artefact would still report itself as built with
    // one — the failure RFC-0006 criterion 10 exists to catch. So these are
    // `|=`, never `=`.
    // **Exhaustive on purpose — no `..Default::default()`.** This is the only
    // place D78's floor is applied, and a field added to `HostOptions` without
    // a decision about what a bundle does with it would otherwise default
    // silently. Written out, it is a compile error until someone chooses.
    let host = bund2_stdlib::host::HostOptions {
        noio: carried.flags & bundle::FLAG_NOIO != 0 || env_set("BUND2_NOIO"),
        noeval: carried.flags & bundle::FLAG_NOEVAL != 0 || env_set("BUND2_NOEVAL"),
        nocolor: env_set("BUND2_NOCOLOR"),
    };

    // **`BUND2_VERBOSE` and `BUND2_REPORT`, since the flags are the
    // program's — D173**, as the other `BUND2_*` switches are (§B3).
    if let Some(level) = ui::level_from_env() {
        ui::set_level(level);
        if let Err(e) = bund2_stdlib::logging::set_threshold(level.name()) {
            ui::warning!("{e}");
        }
    }
    let args = Args {
        // The path `bund2 build` was given, so a diagnostic from a bundle
        // names the file a `script` run of the same program would name.
        file: carried.name.clone(),
        host,
        script_args: argv,
        dump_stack: !env_set("BUND2_NO_DUMP_STACK"),
        raw_values: env_set("BUND2_RAW_VALUES"),
        stats: env_set("BUND2_STATS"),
        // **RFC-0011 §S6: a bundle owns all of argv, so the profiler reaches
        // it through the environment, holding the base path** — D185's
        // operand by another road. Resolved against the run's directory.
        profile: std::env::var("BUND2_PROFILE")
            .ok()
            .filter(|s| !s.is_empty()),
        // D191: the trace by the same road.
        trace: std::env::var("BUND2_TRACE").ok().filter(|s| !s.is_empty()),
        jit_threshold: None,
        // **A bundle is never dropped into the debugger, and not by omission.**
        // The exhaustive literal made this a choice rather than a default, as
        // its comment intends. `--debugger` cannot reach here — a bundle owns
        // all of argv (§B3) — so the only route would be a `BUND2_DEBUGGER`
        // switch beside the others, and that would let an environment variable
        // turn a shipped program into one that blocks on stdin. A bundle that
        // hung in a deployment because a variable was exported somewhere is a
        // worse failure than not being able to debug it, and the debuggable
        // form of the same program is `bund2 script --file`.
        debugger: false,
        // Nor into the shell after a failure, for the same reason: D156 left
        // a bundle's answer open, and a shipped program that waits at a
        // prompt because it failed is the hazard the paragraph above names.
        debug_shell: false,
        // **Nor by a word in the program — D115.** `debug.step` and the other
        // arming and moving words attach a console in a `script` run (D113.5).
        // In a bundle they do nothing: the program left in a stray
        // `debug.break` would otherwise wait at a prompt wherever it was
        // deployed, for as long as its input stayed open.
        carried: true,
        report: trace::Mode::from_env(),
    };
    bund2_stdlib::host::set_args(args.script_args.clone());
    match run_with_prelude(&carried.includes, &carried.source, &args) {
        Some(code) => ExitCode::from(u8::try_from(code.rem_euclid(256)).unwrap_or(1)),
        None => ExitCode::SUCCESS,
    }
}

/// `BUND2_*` switches: set and not `0` or empty.
fn env_set(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => !v.is_empty() && v != "0",
        Err(_) => false,
    }
}

/// What `bund2 build` was asked for, once every argument has been understood.
enum BuildRequest {
    /// `--inspect <artefact>`.
    Inspect(String),
    /// `--file <src> --output <path>`, with D78's floor and D176's includes.
    Bundle {
        /// `--include <path>`, each as given, in order.
        includes: Vec<String>,
        src_path: String,
        out_path: String,
        flags: u8,
        /// `--check`: read, parse and analyse every file, write nothing
        /// (D177).
        check_only: bool,
        /// `--strict`: an underflow finding refuses the build (D177).
        strict: bool,
    },
}

const BUILD_USAGE: &str = "expected: bund2 build --file <path> --output <path> [--include <path>]... \
     [--noio] [--noeval] [--strict], or: bund2 build --check --file <path> [--include <path>]..., \
     or: bund2 build --inspect <artefact>";

/// Read `bund2 build`'s arguments, refusing whatever it does not understand.
///
/// **Nothing is skipped.** This once looked each flag up by name and ignored
/// the rest, so `bund2 build --emit=native --features jit …` exited 0 having
/// written a default-feature bundle: a withdrawn mode accepted, and a request
/// for a different runtime answered with this one. A build that does something
/// other than what it was asked and says nothing is the silent failure
/// RFC-0006 §B3a argues against for the restriction flags, and it is the same
/// failure here.
fn build_request(args: &[String]) -> Result<BuildRequest, String> {
    let mut file: Option<String> = None;
    let mut output: Option<String> = None;
    let mut inspect: Option<String> = None;
    let mut includes: Vec<String> = Vec::new();
    let mut flags = 0u8;
    let mut check_only = false;
    let mut strict = false;
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        let mut operand = |what: &str| -> Result<String, String> {
            inline
                .clone()
                .or_else(|| rest.next().cloned())
                .ok_or_else(|| format!("`{name}` needs {what}"))
        };
        let once = |slot: &mut Option<String>, v: String| -> Result<(), String> {
            if slot.is_some() {
                return Err(format!(
                    "`{name}` was given twice. A bundle carries exactly one program \
                     and is written to exactly one path."
                ));
            }
            *slot = Some(v);
            Ok(())
        };
        match name {
            "--file" => {
                let v = operand("a source path")?;
                once(&mut file, v)?;
            }
            "--output" | "-o" => {
                let v = operand("a path to write")?;
                once(&mut output, v)?;
            }
            // **Repeatable, and the order is the order of evaluation** —
            // D176. A prelude the program is written against, carried with
            // it; each file runs in the program's VM before the program, so
            // the whole is what piping the files concatenated into `bund2`
            // means, with each diagnostic naming its own file.
            "--include" => includes.push(operand("a source path")?),
            // **The dry run and the gate — D177.** `--check` stops after the
            // parse and the underflow analysis and writes nothing; `--strict`
            // turns a finding into a refusal, which RFC-0004 §S4 keeps opt-in
            // because `!` is opaque and a false positive is certain.
            "--check" if inline.is_none() => check_only = true,
            "--strict" if inline.is_none() => strict = true,
            "--inspect" => {
                let v = operand("an artefact to read")?;
                once(&mut inspect, v)?;
            }
            "--noio" if inline.is_none() => flags |= bundle::FLAG_NOIO,
            "--noeval" if inline.is_none() => flags |= bundle::FLAG_NOEVAL,
            // The one mode there is, under the name RFC-0006 gives it.
            "--emit" => match operand("a mode")?.as_str() {
                "bundle" => {}
                "native" => {
                    return Err("`--emit=native` was withdrawn and is not built (D83). \
                                `bund2 build` produces a bundle."
                        .to_string());
                }
                other => {
                    return Err(format!(
                        "unknown mode `--emit={other}`. `bund2 build` produces a bundle."
                    ));
                }
            },
            // **The runtime is this binary.** A bundle is a copy of the
            // executable doing the building, so its features are the ones this
            // was compiled with and cannot be chosen here.
            "--features" => {
                let built = features_built_in();
                return Err(format!(
                    "`--features` cannot select a runtime: a bundle is a copy of this \
                     binary, which was built with {}. Build `bund2` itself with the \
                     features the bundle should carry.",
                    if built.is_empty() {
                        "default features".to_string()
                    } else {
                        format!("features: {built}")
                    }
                ));
            }
            _ => return Err(format!("unknown argument `{arg}`")),
        }
    }
    if let Some(target) = inspect {
        if file.is_some()
            || output.is_some()
            || flags != 0
            || !includes.is_empty()
            || check_only
            || strict
        {
            return Err("`--inspect` reads an artefact and takes no other argument".to_string());
        }
        return Ok(BuildRequest::Inspect(target));
    }
    // `--check` needs no `--output`; one given is accepted and not written.
    let output = if check_only {
        Some(output.unwrap_or_default())
    } else {
        output
    };
    match (file, output) {
        (Some(src_path), Some(out_path)) => Ok(BuildRequest::Bundle {
            includes,
            src_path,
            out_path,
            check_only,
            strict,
            flags,
        }),
        (None, _) => Err("no `--file` was given".to_string()),
        (_, None) => Err("no `--output` was given".to_string()),
    }
}

/// `bund2 build --file <src> --output <path> [--noio] [--noeval]`.
///
/// **Parses before it writes** (RFC-0006 §B3): a syntax error is a build
/// error, so it belongs to whoever built the artefact rather than to whoever
/// ran it. The parse result is then discarded — the artefact parses again at
/// start-up, which is what keeps §B2's preservation exact.
fn build(args: &[String]) -> ExitCode {
    let (includes, src_path, out_path, flags, check_only, strict) = match build_request(args) {
        // `bund2 build --inspect <artefact>` — D78's readable trailer.
        //
        // **Reads a file, unlike everything else about a bundle.** A runtime
        // asks its own memory what it carries; inspecting asks about *another*
        // artefact, which is the whole point: the person handed a bundle is
        // not the person who built it.
        Ok(BuildRequest::Inspect(target)) => return inspect(&target),
        Ok(BuildRequest::Bundle {
            includes,
            src_path,
            out_path,
            flags,
            check_only,
            strict,
        }) => (includes, src_path, out_path, flags, check_only, strict),
        Err(why) => {
            ui::error!("build: {why}");
            ui::error!("{BUILD_USAGE}");
            ui::error!("nothing was written");
            return ExitCode::from(2);
        }
    };
    // **Six steps, drawn on a terminal and silent on a pipe — D173.** Every
    // failure clears the bar before it is reported, so the report is the
    // last thing on the line, and every step says what it did at `info`.
    // Under `--check` the first two are the whole job.
    let bar = ui::progress(
        if check_only { 2 } else { 6 },
        if check_only { "check" } else { "build" },
    );
    let fail = |bar: &indicatif::ProgressBar, code: ExitCode, why: std::fmt::Arguments<'_>| {
        bar.finish_and_clear();
        ui::error!("{why}");
        code
    };

    bar.set_message(format!("reading {src_path}"));
    let src = match std::fs::read_to_string(&src_path).map(without_shebang) {
        Ok(s) => s,
        Err(e) => {
            return fail(
                &bar,
                ExitCode::FAILURE,
                format_args!("reading {src_path}: {e}"),
            );
        }
    };
    ui::info!("read {src_path}: {} bytes", src.len());
    // **The includes, in the order given — D176.** Read like the program,
    // shebang and all, so a file that runs on its own is carried as it is.
    let mut included: Vec<(String, String)> = Vec::with_capacity(includes.len());
    for path in &includes {
        bar.set_message(format!("reading {path}"));
        match std::fs::read_to_string(path).map(without_shebang) {
            Ok(text) => {
                ui::info!("read {path}: {} bytes", text.len());
                included.push((path.clone(), text));
            }
            Err(e) => {
                return fail(&bar, ExitCode::FAILURE, format_args!("reading {path}: {e}"));
            }
        }
    }
    bar.inc(1);

    // The parse, kept for the analysis and reported on failure — for each
    // included file first, then the program, so a build refuses what a run
    // would refuse and names the file.
    bar.set_message("parsing the program");
    let mut parsed = Vec::with_capacity(included.len() + 1);
    for (path, text) in included
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .chain(std::iter::once((src_path.as_str(), src.as_str())))
    {
        match bund2_syntax::parse(text) {
            Ok(terms) => {
                ui::info!("parsed {path}: {} top-level terms", terms.len());
                parsed.push((
                    path,
                    text,
                    bund2_syntax::lower_with_spans(&terms, text.len()),
                ));
            }
            Err(e) => {
                let loc = locate(text, path, e.span);
                bar.finish_and_clear();
                ui::error!("{path}:{}:{}: {}", loc.line, loc.column, e.what);
                ui::error!("nothing was written");
                return ExitCode::FAILURE;
            }
        }
    }
    // **The underflow analysis, over every file as one stream — D177.** The
    // includes run before the program, so a word an include registers is a
    // known effect when the program uses it, and the depth carries across
    // the file boundary exactly as it does at run time. Findings are
    // warnings (RFC-0004 §S4); `--strict` makes them a refusal.
    let underflow = underflow_over(&parsed);
    drop(parsed);
    bar.finish_and_clear();
    for line in underflow.findings.iter().chain(&underflow.hazards) {
        ui::warning!("{line}");
    }
    if check_only {
        bund2_api::sayln!("{}", underflow.summary);
        for line in &underflow.detail {
            bund2_api::sayln!("{line}");
        }
        return if strict && !underflow.findings.is_empty() {
            ui::error!(
                "{} underflow finding(s) under --strict",
                underflow.findings.len()
            );
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        };
    }
    ui::info!("{}", underflow.summary);
    if strict && !underflow.findings.is_empty() {
        ui::error!(
            "{} underflow finding(s) under --strict; nothing was written",
            underflow.findings.len()
        );
        return ExitCode::FAILURE;
    }
    let bar = ui::progress(6, "build");
    bar.inc(2);

    bar.set_message("locating this executable");
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            return fail(
                &bar,
                ExitCode::FAILURE,
                format_args!("cannot locate this executable to copy: {e}"),
            );
        }
    };
    ui::debug!("the runtime to copy is {}", exe.display());
    // **Refuse to overwrite the binary doing the building.**
    //
    // `bund2 build --output $(which bund2)` succeeded, reported success, and
    // turned the interpreter into a bundle: afterwards `bund2 --file x.bund`
    // ran the *embedded* program and ignored the argument, because a bundle
    // gives all of argv to its program. Destructive, silent, and one keystroke
    // away from `-o` on the wrong path.
    //
    // Compared by canonical path, so a symlink or a relative path cannot slip
    // past. **A hard link can** — two names for one file have two canonical
    // paths — and what makes that harmless is `place`, which never writes
    // through an existing name (F181). An output that does not exist yet
    // cannot be the running binary, so absence is not an error.
    if let Ok(existing) = std::fs::canonicalize(&out_path)
        && std::fs::canonicalize(&exe)
            .map(|e| e == existing)
            .unwrap_or(false)
    {
        return fail(
            &bar,
            ExitCode::FAILURE,
            format_args!(
                "--output is this binary ({out_path}). Building over the \
                 executable doing the building would replace the interpreter with \
                 the artefact. Nothing was written."
            ),
        );
    }
    // **Nor over the program it was given.** `--file p.bund --output p.bund`
    // exited 0 and left `p.bund` an executable: the source was read, then
    // replaced by the artefact built from it. The builder can be rebuilt and
    // a source cannot, so the argument for the guard above holds here at
    // least as well. The source was readable a moment ago, so it resolves.
    if let Ok(source) = std::fs::canonicalize(&src_path)
        && destination(&out_path) == source
    {
        return fail(
            &bar,
            ExitCode::FAILURE,
            format_args!(
                "--output is the program being built ({out_path}). The artefact \
                 would replace its own source. Nothing was written."
            ),
        );
    }
    // Nor over a file it includes, for the same reason.
    for (path, _) in &included {
        if let Ok(source) = std::fs::canonicalize(path)
            && destination(&out_path) == source
        {
            return fail(
                &bar,
                ExitCode::FAILURE,
                format_args!(
                    "--output is an included file ({out_path}). The artefact would \
                     replace a file it carries. Nothing was written."
                ),
            );
        }
    }
    ui::trace!(
        "the artefact will be placed at {}",
        destination(&out_path).display()
    );
    bar.inc(1);

    bar.set_message("reading the runtime image");
    let mut image = match std::fs::read(&exe) {
        Ok(b) => b,
        Err(e) => {
            return fail(
                &bar,
                ExitCode::FAILURE,
                format_args!("reading {}: {e}", exe.display()),
            );
        }
    };
    ui::info!("read the runtime: {} bytes", image.len());
    bar.inc(1);

    bar.set_message("writing the program into the image");
    let (version, features, pinned) = (
        env!("CARGO_PKG_VERSION"),
        features_built_in(),
        pinned_summary(),
    );
    ui::debug!(
        "trailer: version {version}, features {}, flags {flags:#04x}, pinned {}",
        if features.is_empty() {
            "default"
        } else {
            features.as_str()
        },
        if pinned.is_empty() {
            "(none)"
        } else {
            pinned.as_str()
        }
    );
    let include_refs: Vec<(&str, &str)> = included
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    if let Err(e) = bundle::write_into(
        &mut image,
        &include_refs,
        &src,
        &src_path,
        flags,
        version,
        &features,
        &pinned,
    ) {
        return fail(&bar, ExitCode::FAILURE, format_args!("{e}"));
    }
    let total: usize = src.len() + included.iter().map(|(_, t)| t.len()).sum::<usize>();
    ui::info!(
        "wrote the program into the image: {total} bytes in {} file(s), compressed into \
         the {} byte region",
        included.len() + 1,
        bundle::CAPACITY
    );
    bar.inc(1);

    bar.set_message(format!("placing {out_path}"));
    if let Err(e) = place(&out_path, &image) {
        return fail(&bar, ExitCode::FAILURE, format_args!("{e}"));
    }
    bar.inc(1);
    bar.finish_and_clear();
    let packed = bundle::inspect(&image).map(|s| s.used).unwrap_or(0);
    ui::error!(
        "wrote {out_path} — {total} bytes{}, {packed} compressed of {} bytes used",
        if included.is_empty() {
            String::new()
        } else {
            format!(" in {} files", included.len() + 1)
        },
        bundle::CAPACITY
    );
    // **Said, not left to be discovered.** The recorded path is what a bundle's
    // diagnostics name (§B3), and it is kept whole only while it fits.
    if src_path.len() > bundle::SOURCE_NAME_BYTES {
        ui::warning!(
            "the source path is {} bytes and a bundle records the last {}. \
             Diagnostics from this artefact name the shortened path.",
            src_path.len(),
            bundle::SOURCE_NAME_BYTES
        );
    }
    ExitCode::SUCCESS
}

/// Write a finished artefact beside `out_path`, then move it into place.
///
/// **Never in place — F181.** `std::fs::write` truncates and rewrites
/// whatever inode the name already has. With `--output` a hard link to the
/// building binary that inode is the builder's: the guard in `build` compares
/// canonical paths, two names for one file have two, and the build reported
/// success having destroyed the interpreter. A rename replaces the *name*, so
/// every other name for the old file, and any process running it, keeps the
/// old bytes.
///
/// **And nothing is left at `--output` unless all of it worked.** The file is
/// written, made executable and signed under a scratch directory; a failure
/// at any step removes that directory and leaves `--output` as it was. A
/// failed signing used to leave a file there that the system kills.
///
/// The scratch directory is beside the destination, so the rename stays on
/// one filesystem, and the file keeps its final name inside it, because an
/// ad-hoc signature takes its identifier from the name it is signed under.
///
/// A symlink at `--output` is followed, as an in-place write followed it: the
/// destination is where the link leads ([`destination`]), whether or not
/// anything is there yet.
///
/// **What this costs, stated.** The destination's *directory* has to be
/// writable, where an in-place write needed only the file. The file that was
/// there is replaced, so its mode and ownership are not kept. And a build
/// killed between the write and the rename leaves its scratch directory
/// behind; the name carries the process id, and one left by an earlier
/// process of the same id is cleared before this one starts.
fn place(out_path: &str, image: &[u8]) -> Result<(), String> {
    use std::path::Path;
    let nothing = |e: String| format!("{e} Nothing was written to {out_path}.");
    let dest = destination(out_path);
    let Some(name) = dest.file_name() else {
        return Err(nothing(format!("--output {out_path} names no file.")));
    };
    let parent = match dest.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let scratch = parent.join(format!(".bund2-build-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir(&scratch).map_err(|e| {
        nothing(format!(
            "writing {out_path}: creating {}: {e}. The artefact is assembled beside its \
             destination, so that directory has to be writable.",
            scratch.display()
        ))
    })?;
    let staged = scratch.join(name);
    let staged_text = staged.to_string_lossy().into_owned();
    ui::debug!(
        "staging the artefact at {staged_text}, then renaming it to {}",
        dest.display()
    );
    let done = std::fs::write(&staged, image)
        .map_err(|e| format!("writing {out_path}: {e}"))
        .and_then(|()| {
            make_executable(&staged_text)
                .map_err(|e| format!("{out_path} could not be made executable: {e}"))
        })
        .and_then(|()| reseal(&staged_text, out_path))
        .and_then(|()| {
            std::fs::rename(&staged, &dest)
                .map_err(|e| format!("moving the artefact to {out_path}: {e}"))
        });
    // Whether or not it worked: on success the directory is empty, and on
    // failure what is in it is an artefact that must not be found.
    let _ = std::fs::remove_dir_all(&scratch);
    done.map_err(nothing)
}

/// Where `--output` leads: the path itself, with every symlink followed.
///
/// `canonicalize` answers this for a path that exists. **A link to a path
/// that does not exist yet is the case it cannot** — it fails, and falling
/// back to the path as given made the rename replace the *link* where an
/// in-place write would have created its target. So a dangling link is
/// followed by hand. The loop is bounded (D39): a chain longer than the bound,
/// or a cycle, is left where the walk stopped, and the rename then replaces
/// that link rather than spinning.
fn destination(out_path: &str) -> std::path::PathBuf {
    use std::path::PathBuf;
    const HOPS: usize = 40;
    let mut at = PathBuf::from(out_path);
    for _ in 0..HOPS {
        if let Ok(whole) = std::fs::canonicalize(&at) {
            return whole;
        }
        let Ok(target) = std::fs::read_link(&at) else {
            break;
        };
        at = match at.parent() {
            Some(dir) if target.is_relative() => dir.join(target),
            _ => target,
        };
    }
    // Nothing is there yet. Resolve the directory, so the comparison in
    // `build` sees the same spelling `canonicalize` gives an existing file.
    match (at.parent(), at.file_name()) {
        (Some(dir), Some(name)) if !dir.as_os_str().is_empty() => std::fs::canonicalize(dir)
            .map(|d| d.join(name))
            .unwrap_or(at),
        _ => at,
    }
}

/// Re-sign the artefact, where the platform requires it — **measured, 2026-09-30.**
///
/// On macOS a binary's signature covers the bytes `bund2 build` just wrote, so
/// the artefact is **killed with SIGKILL** until it is signed again: not a
/// validation warning, a refusal to execute. An ad-hoc re-sign restores both —
/// it runs, and `codesign -v` reports no error, which is what makes a later
/// Developer ID signature and notarisation possible at all.
///
/// `/usr/bin/codesign` is a base-system binary rather than a toolchain
/// install, which is why this is not the `cc` dependency D10 forbids below
/// `bund2 build`. **The boundary is the repository owner's to confirm**, and
/// until it is this reports plainly rather than silently producing an artefact
/// that cannot run.
///
/// Nothing to do on platforms with no whole-file signature: ELF and PE carry
/// none by default, so the bytes just written are the bytes that run.
#[cfg(target_os = "macos")]
fn reseal(path: &str, shown: &str) -> Result<(), String> {
    ui::debug!("signing {shown} ad hoc: /usr/bin/codesign -f -s - {path}");
    let out = std::process::Command::new("/usr/bin/codesign")
        .args(["-f", "-s", "-", path])
        .output()
        .map_err(|e| {
            format!(
                "{shown} could not be signed: /usr/bin/codesign did not run \
                 ({e}). On macOS an unsigned edit to a binary is killed rather than run, so \
                 the artefact would not execute."
            )
        })?;
    if out.status.success() {
        return Ok(());
    }
    Err(format!(
        "codesign refused {shown}: {}. The artefact would be killed \
         rather than run.",
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}

#[cfg(not(target_os = "macos"))]
fn reseal(_path: &str, _shown: &str) -> Result<(), String> {
    Ok(())
}

/// Print what an artefact says about itself.
fn inspect(path: &str) -> ExitCode {
    let image = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            ui::error!("reading {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let seen = match bundle::inspect(&image) {
        Ok(s) => s,
        Err(e) => {
            ui::error!("{path} is not a bund2 artefact: {e}");
            return ExitCode::FAILURE;
        }
    };
    bund2_api::sayln!("{path}");
    bund2_api::sayln!("  container      version {}", seen.format);
    if seen.carries_program {
        bund2_api::sayln!(
            "  program        {} bytes, {} of {} bytes in the region, from {}",
            seen.raw,
            if seen.format == bundle::FORMAT {
                format!("{} compressed", seen.used)
            } else {
                format!("{} plain", seen.used)
            },
            seen.capacity,
            if seen.source_name.is_empty() {
                "an unrecorded path"
            } else {
                seen.source_name.as_str()
            }
        );
    } else {
        bund2_api::sayln!("  program        none — this is the plain interpreter");
    }
    // **The includes, in the order they run — D176.** Every file but the
    // last is one; the last is the program the line above described.
    if seen.files.len() > 1 {
        let listed: Vec<String> = seen.files[..seen.files.len() - 1]
            .iter()
            .map(|(name, n)| format!("{name} ({n} bytes)"))
            .collect();
        bund2_api::sayln!("  includes       {}", listed.join(", "));
    }
    // **Named even when empty.** "restrictions none" and a missing line say
    // different things to someone deciding whether to trust an artefact, and
    // D78 exists because a restriction nobody can see is one nobody can rely
    // on.
    let mut restrictions: Vec<&str> = Vec::new();
    if seen.noio {
        restrictions.push("--noio");
    }
    if seen.noeval {
        restrictions.push("--noeval");
    }
    // **Never "none" for a bit this bund2 cannot name.** It does not know
    // what the bit forbids, so it says that, and that it will not run it.
    let unknown = format!(
        "unknown (flag bits {:#04x}; this bund2 does not define them, and the artefact's own runtime decides)",
        seen.unknown_flags
    );
    if seen.unknown_flags != 0 {
        restrictions.push(&unknown);
    }
    bund2_api::sayln!(
        "  restrictions   {}",
        if restrictions.is_empty() {
            "none".to_string()
        } else {
            restrictions.join(" ")
        }
    );
    // **A runtime nobody built from has no record, and says so.** The fields
    // are written by `bund2 build`, so in the plain interpreter they are
    // empty — and an empty feature field read as "default features" described
    // a `jit` build of `bund2` as one without.
    if seen.carries_program {
        bund2_api::sayln!(
            "  built by       bund2 {}{}",
            if seen.bund2_version.is_empty() {
                "(unrecorded)"
            } else {
                seen.bund2_version.as_str()
            },
            if seen.features.is_empty() {
                ", default features".to_string()
            } else {
                format!(", features: {}", seen.features)
            }
        );
    } else {
        bund2_api::sayln!("  built by       (unrecorded — nothing was built from this runtime)");
    }
    bund2_api::sayln!(
        "  conformed to   {}",
        if seen.pinned.is_empty() {
            "(unrecorded)"
        } else {
            seen.pinned.as_str()
        }
    );
    // The restrictions are a floor; run time may add. Saying so here stops a
    // reader treating the line above as the whole truth (D78).
    if seen.carries_program {
        bund2_api::sayln!("\n  Restrictions are a floor: BUND2_NOIO and BUND2_NOEVAL may add,");
        bund2_api::sayln!("  never remove. What a bundle does not restrict, it permits.");
    }
    ExitCode::SUCCESS
}

/// Give the artefact the mode this binary has, so it can be run.
#[cfg(unix)]
fn make_executable(path: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(path)
        .map_err(|e| format!("reading the mode of {path}: {e}"))?
        .permissions();
    perm.set_mode(perm.mode() | 0o111);
    std::fs::set_permissions(path, perm).map_err(|e| format!("setting the mode of {path}: {e}"))
}

#[cfg(not(unix))]
fn make_executable(_path: &str) -> Result<(), String> {
    Ok(())
}

/// **`--stats`: what the tier did and where §S8's floors are.**
///
/// **On stderr, deliberately.** Every golden captures stdout, so a stats
/// line there would move `conform` by the whole corpus at once. RFC-0005
/// criterion 2 wants the figure reported, not the programs changed.
///
/// `None` and `Some(0)` say different things: no tier at all — a build
/// without the feature — against a tier that compiled nothing, which over a
/// corpus is the failure criterion 2 asks this flag to catch.
///
/// **Printed for a program that did not parse, too**, since 2026-10-12. The
/// floors belong to the process and not to the program, and RFC-0005's
/// criterion 11 has every run report them: `cargo xtask depth`'s `nesting`
/// axis ends at the parser's refusal and printed nothing.
/// `declined` names why no tier was installed when the build has one —
/// `--debugger` (RFC-0008 §D6) or `--profile` (D186) — so the no-tier line
/// says that rather than "built without `jit`", which would be false.
fn print_stats(rt: &bund2_runtime::Runtime, declined: Option<&str>) {
    // **How many of those bodies actually ran, when the tier can say.**
    // Every figure below counts a decision the *compiler* made. None of
    // them counts an execution, so a tier that compiles a corpus and then
    // declines every entry — §S8's Tier 1 floor on a thread with no share
    // is the way that happens — prints exactly what a working tier prints.
    // Four fixtures were found in that state before this figure existed.
    let entered = match rt.compiled_entries() {
        Some(n) => format!(" ({n} entered)"),
        None => String::new(),
    };
    match (
        rt.compiled_bodies(),
        rt.inlined_sites(),
        rt.promoted_values(),
        rt.compiled_values(),
    ) {
        // **All three figures, because each without the others says
        // little.** A body that compiled but inlined nothing still applies
        // every value through `Vm::apply`; a body that inlined but promoted
        // nothing still pushes and pops every literal. A timing of either
        // answers a different question from the one criterion 10 asks.
        // **The threshold is part of the figure, not context for it.** The
        // same count at 64 and at 1 says two different things — at 64 most
        // corpus programs compile nothing — and a threshold that arrived
        // from `BUND2_JIT_THRESHOLD`, or was dropped because that variable
        // was malformed, is invisible otherwise (F125).
        // **The fourth figure is the denominator, and it reports a cost**
        // (F136). The three above say what a body gained; none says what the
        // rest of it paid. A value that is neither a site nor a promoted
        // literal takes the generic path, which is **5.13 ns dearer than
        // Tier 0** (criterion 10), so `values − sites − promoted` is what
        // compiling that body cost. F133's rule can ask whether a body gains
        // anything; asking whether it gains *enough* needs this.
        (Some(bodies), Some(sites), Some(promoted), Some(values)) => {
            let at = match rt.jit_threshold() {
                Some(n) => format!(" at threshold {n}"),
                None => String::new(),
            };
            let generic = values.saturating_sub(sites + promoted);
            // **The fifth figure, when the tier can answer it** (D68).
            // Without it a corpus sweep sees how many values were generic
            // but not whether promotion crossed any of those calls, which
            // is the difference between "crossing does not pay" and
            // "crossing never happened".
            let crossed = match rt.crossed_calls() {
                Some(n) => format!(", {n} crossed"),
                None => String::new(),
            };
            bund2_api::errln!(
                "bund2: tier compiled {bodies} bodies{entered}, inlined {sites} sites, \
                 promoted {promoted} values, {generic} generic of {values}{crossed}{at}"
            );
        }
        (Some(bodies), Some(sites), Some(promoted), None) => {
            let at = match rt.jit_threshold() {
                Some(n) => format!(" at threshold {n}"),
                None => String::new(),
            };
            bund2_api::errln!(
                "bund2: tier compiled {bodies} bodies{entered}, inlined {sites} sites, \
                 promoted {promoted} values{at}"
            );
        }
        (Some(bodies), Some(sites), None, _) => {
            bund2_api::errln!(
                "bund2: tier compiled {bodies} bodies{entered}, inlined {sites} sites"
            );
        }
        (Some(bodies), None, _, _) => {
            bund2_api::errln!("bund2: tier compiled {bodies} bodies{entered}")
        }
        _ => match declined {
            Some(why) => bund2_api::errln!("bund2: no tier (declined: {why})"),
            None => bund2_api::errln!("bund2: no tier (built without `jit`)"),
        },
    }
    // **§S8's floors, and what the upper one has done** — RFC-0005
    // criterion 11 as D147 leaves it. A line of its own, because the
    // addresses differ from run to run and the line above is compared
    // between runs by tests and by `conform`'s sweep.
    let (tier0, tier1) = rt.interp.floors();
    let gap = match tier1.checked_sub(tier0) {
        Some(n) => format!("{n} bytes above"),
        None => format!("{} bytes below", tier0.saturating_sub(tier1)),
    };
    let declined = match rt.floor_declines() {
        Some((0, _)) => "; no entry declined at the tier 1 floor".to_string(),
        Some((n, Some(depth))) => format!(
            "; {n} entries declined at the tier 1 floor, the first with {depth} \
             compiled bodies running"
        ),
        Some((n, None)) => format!("; {n} entries declined at the tier 1 floor"),
        None => String::new(),
    };
    bund2_api::errln!(
        "bund2: floors: tier 0 at {tier0:#x}, tier 1 at {tier1:#x} ({gap}){declined}"
    );
}

/// `--debug-shell` — D156: a failure has been reported, and the session is
/// given `debug.shell` on the VM as the program left it, stacks and words.
///
/// **By the word's name and not by a call**, so the shell is whatever the
/// registry binds: under `--noio` and `--noeval` it is what those make of
/// the word, and the flag is no way round either.
///
/// **Under `--debugger` there is no shell, and the session is told so** —
/// D162. A debugged program is given no input (D112), since standard input
/// is the session's, so a shell there would read the end at once and return.
/// The reference does run its shell after a `--debugger` failure; Bund2's
/// console was already on this VM when the failure was reported, which is
/// what the shell is for. One `Notice` says no shell follows, so a user who
/// passed both flags does not take the second for ignored.
///
/// A failure of the shell itself is reported and is not a second shell.
fn shell_after_a_failure(vm: &mut bund2_interp::Interp, args: &Args) {
    if !args.debug_shell {
        return;
    }
    if args.debugger {
        vm.report(bund2_api::diag::Diagnostic::notice(
            "--debug-shell: no shell follows a --debugger session; \
             its console was on this VM when the failure was reported",
        ));
        return;
    }
    let shell = match bund2_syntax::parse("debug.shell") {
        Ok(terms) => bund2_syntax::lower_with_spans(&terms, 0),
        Err(e) => {
            vm.report(bund2_api::diag::Diagnostic::warning(format!(
                "--debug-shell: {}",
                e.what
            )));
            return;
        }
    };
    if let Err((_, e)) = vm.eval_indexed(&shell.values) {
        vm.report(bund2_api::diag::Diagnostic::warning(format!(
            "--debug-shell: {}",
            e.0
        )));
    }
}

/// Returns the code a word asked to exit with (D52), if one did. **A failure
/// is not an error exit**:
/// the reference prints its report and exits 0
/// (`reference/Bund/src/stdlib/helpers/run_snippet.rs:85-90` sets no code), and
/// every golden capturing a failing program pins that. Changing it would be a
/// deviation nobody has asked for.
/// **A program that does not parse exits 1 — D194.** Nothing of it ran, so
/// no exit status of the program's own is being overridden; a runtime failure
/// still exits 0, as the reference's does, since the goldens record it.
const PARSE_FAILED: i32 = 1;

fn run(src: &str, args: &Args) -> Option<i32> {
    run_with_prelude(&[], src, args)
}

/// [`run`], with the files a bundle's `--include` named evaluated first, in
/// order, in the same VM — D176. A failure in one is reported against that
/// file and the program does not run, exactly as a failure partway through
/// one concatenated stream would leave the rest unrun.
fn run_with_prelude(prelude: &[(String, String)], src: &str, args: &Args) -> Option<i32> {
    let file = args.file.as_str();
    // **Through `Runtime`, which is what installs the tier — F124.** Building
    // an `Interp` here directly is what made `--features jit` inert: the
    // feature was enabled, the binary differed, and Tier 1 was never reached,
    // so criterion 2 compared one interpreter with itself. `Runtime` registers
    // the same vocabulary with the same host options and adds the tier when the
    // feature is on; everything below reaches the interpreter through it.
    // **§D6: a debugged session installs no tier.** One call, in the one place
    // a tier is installed, which is what §D6 means by needing nothing new.
    // **D186: a profiled run installs no tier either**, through the same
    // constructor and for the same shape of reason (RFC-0011 §S3).
    let mut rt = if args.tier_declined().is_some() {
        bund2_runtime::Runtime::for_debugging(&args.host, args.jit_threshold)
    } else {
        bund2_runtime::Runtime::with_options_and_threshold(&args.host, args.jit_threshold)
    };
    let startup_defect = rt.take_startup_defect();
    let vm = &mut rt.interp;
    // **RFC-0011: the record, installed before anything runs.** The clock's
    // cost is measured first so the report can print it.
    if args.profile.is_some() || args.trace.is_some() {
        let clock = profile::calibrate();
        let mut record = bund2_interp::profile::Profile::new(clock.cost, clock.resolution);
        // `debug.profile` renders mid-run with the same two renderers, so
        // the record carries them; the header says the run had not ended.
        let program = args.file.clone();
        let jit_threshold_given = args.jit_threshold.is_some();
        record.writer = Some(std::rc::Rc::new(move |p, reg, base, open| {
            let meta = profile::Meta {
                program: &program,
                jit_threshold_given,
                ended: profile::Ended::Running(open),
            };
            let dot = profile::render_dot(p, reg);
            let md = profile::render_markdown(p, reg, &meta);
            profile::write_both(base, &dot, &md).map(|(dot, md)| format!("{md} and {dot}"))
        }));
        // **D191: the trace, opened before anything runs.** A file that
        // cannot be made ends the run before it starts, as a bad operand
        // would: a trace asked for and silently absent is worse.
        if let Some(path) = args.trace.as_deref() {
            let file = match std::fs::File::create(path) {
                Ok(f) => f,
                Err(e) => {
                    ui::error!("--trace: cannot write {path}: {e}");
                    return Some(1);
                }
            };
            let out = std::rc::Rc::new(std::cell::RefCell::new(std::io::BufWriter::new(file)));
            {
                use std::io::Write as _;
                let mut w = out.borrow_mut();
                let _ = writeln!(
                    w,
                    "# bund2 trace of {}: one line per call entered, natives included, indented by the calls open around it; `!` marks where a failure was raised",
                    args.file
                );
            }
            record.trace_to(out);
        }
        vm.profile = Some(Box::new(record));
    }
    ui::configure_colors(args.host.nocolor);
    let mut reporter = bund2_stdlib::report::TextReporter::new(args.dump_stack);
    reporter.raw_values = args.raw_values;
    reporter.quiet = ui::level() < ui::Level::Warning;
    // **D173: the table, or the traceback in front of it.** One reporter on
    // the seam either way; what differs is the drawing.
    let traces = args.report.traces();
    ui::debug!(
        "reporting {} ({})",
        if traces {
            "tracebacks"
        } else {
            "the reference's table"
        },
        match args.report {
            trace::Mode::Auto => "auto: standard output is a terminal, or is not",
            trace::Mode::Box => "--report box",
            trace::Mode::Trace => "--report trace",
        }
    );
    vm.reporter =
        Box::new(trace::TraceReporter::new(reporter, src, file, traces).with_sources(prelude));
    ui::info!("running {file}, {} bytes", src.len());
    // Found while the runtime was assembled, before this reporter existed.
    if let Some(e) = startup_defect {
        bund2_api::Vm::report(vm, bund2_api::diag::Diagnostic::warning(e.0));
    }
    // **Where the program's lines come from — D112.** The terminal, except
    // under `--debugger`: the session's own commands arrive on standard input,
    // and a program reading the same stream took them as its lines (F165). A
    // debugged program is given no input, which is what a capture gives it —
    // every read is the end at once — so stepping a program that reads
    // changes nothing about what it does.
    vm.input = if args.debugger {
        Box::new(bund2_api::input::NoInput)
    } else {
        Box::new(bund2_stdlib::terminal::Terminal::new())
    };
    // **Attached after the reporter, before any evaluation.** The debuggee
    // renders its own answers through the same `Interp` the reporter is on, so
    // a session's text and its diagnostics come from one place.
    // **Before either console can exist — F180.** A breakpoint's condition is
    // evaluated in a child VM, and it is given these so it is no freer than
    // the program that armed it.
    debugger::restrict(args.host);
    if args.debugger {
        vm.attach_debugger(Box::new(debugger::Stdio::new()));
    } else if args.carried {
        // **D115: a bundle is given no console and says so**, so the words
        // that would attach one do nothing rather than refuse.
        vm.console_refused = true;
    } else {
        // **§W5: a word may ask for a debugger the flag did not.** The
        // console it gets reads through the program's own input. Not under
        // `--debugger`, where one is attached already and owns standard input.
        vm.console_factory = Some(debugger::OverInput::boxed);
    }

    // **The prelude first — D176.** Each included file is parsed and run as
    // the program is below, against its own name, and the first failure ends
    // the run before the program starts.
    for (name, text) in prelude {
        ui::info!("running the included {name}, {} bytes", text.len());
        let terms = match bund2_syntax::parse(text) {
            Ok(t) => t,
            Err(e) => {
                vm.report(
                    bund2_api::diag::Diagnostic::error(e.what.clone())
                        .at(locate(text, name, e.span)),
                );
                shell_after_a_failure(vm, args);
                if args.stats {
                    print_stats(&rt, args.tier_declined());
                }
                let ended = profile::Ended::ParseError(e.what.clone());
                return finish_profile(&mut rt, args, ended, Some(PARSE_FAILED));
            }
        };
        let lowered = bund2_syntax::lower_with_spans(&terms, text.len());
        if let Err((i, e)) = vm.eval_indexed(&lowered.values) {
            ui::trace!("value {i} of the included {name} failed: {}", e.0);
            let mut d = bund2_api::diag::Diagnostic::error(e.0);
            if let Some(span) = lowered.span_of(i) {
                d = d.at(locate(text, name, span));
            }
            let ended = profile::Ended::Error(ended_text(&d));
            vm.report(d);
            shell_after_a_failure(vm, args);
            let code = vm.exit_requested();
            if args.stats {
                print_stats(&rt, args.tier_declined());
            }
            return finish_profile(&mut rt, args, ended, code);
        }
    }

    // No `\n` is appended. The reference appends one at four of its five parse
    // sites to satisfy a grammar rule that demands trailing whitespace; S1
    // admits end of input as a terminator instead, so the workaround is gone.
    let terms = match bund2_syntax::parse(src) {
        Ok(t) => t,
        Err(e) => {
            vm.report(
                bund2_api::diag::Diagnostic::error(e.what.clone()).at(locate(src, file, e.span)),
            );
            shell_after_a_failure(vm, args);
            if args.stats {
                print_stats(&rt, args.tier_declined());
            }
            let ended = profile::Ended::ParseError(e.what.clone());
            return finish_profile(&mut rt, args, ended, Some(PARSE_FAILED));
        }
    };
    let lowered = bund2_syntax::lower_with_spans(&terms, src.len());
    ui::debug!(
        "parsed {} top-level terms, lowered to {} values",
        terms.len(),
        lowered.values.len()
    );
    let mut ended = None;
    if let Err((i, e)) = vm.eval_indexed(&lowered.values) {
        ui::trace!("value {i} of the top-level stream failed: {}", e.0);
        let mut d = bund2_api::diag::Diagnostic::error(e.0);
        if let Some(span) = lowered.span_of(i) {
            d = d.at(locate(src, file, span));
        }
        ended = Some(profile::Ended::Error(ended_text(&d)));
        vm.report(d);
        shell_after_a_failure(vm, args);
    }
    let code = vm.exit_requested();
    let ended = match (ended, code) {
        (Some(e), _) => e,
        (None, Some(c)) => profile::Ended::Exit(c),
        (None, None) => profile::Ended::Normally,
    };

    if args.stats {
        print_stats(&rt, args.tier_declined());
    }
    finish_profile(&mut rt, args, ended, code)
}

/// A diagnostic's reason with its location, for the profile's `ended`
/// cell: the same two things the reporter shows, on one line.
fn ended_text(d: &bund2_api::diag::Diagnostic) -> String {
    match &d.location {
        Some(loc) => format!("{} at {loc}", d.reason),
        None => d.reason.clone(),
    }
}

/// **RFC-0011 §S6, D187: render the record and write the two files.**
///
/// Nothing is said at the default level on success: writing the files is the
/// command line's own work, so it speaks in D173's second voice, one `info`
/// line. A failure to write is an `error` at every level. The exit code is
/// the program's, except when the program itself succeeded and set no code
/// and its profile could not be written: then it is 1, the code `build`
/// answers when it cannot write its output, so one number has one meaning.
fn finish_profile(
    rt: &mut bund2_runtime::Runtime,
    args: &Args,
    ended: profile::Ended,
    code: Option<i32>,
) -> Option<i32> {
    let Some(mut record) = rt.interp.profile.take() else {
        return code;
    };
    // D191: the trace first, whether or not a report follows. A trace that
    // lost lines says so and the run's code is 1, as for a report that
    // could not be written.
    let failed = matches!(
        ended,
        profile::Ended::Error(_) | profile::Ended::ParseError(_)
    );
    let code = match record.finish_trace() {
        Ok(()) => code,
        Err(e) => {
            ui::error!(
                "--trace: {}: {e}; the trace is incomplete",
                args.trace.as_deref().unwrap_or("")
            );
            if failed { code } else { code.or(Some(1)) }
        }
    };
    let Some(base) = args.profile.as_deref() else {
        return code;
    };
    record.finish();
    let meta = profile::Meta {
        program: &args.file,
        jit_threshold_given: args.jit_threshold.is_some(),
        ended,
    };
    let dot = profile::render_dot(&record, &rt.interp.registry);
    let md = profile::render_markdown(&record, &rt.interp.registry, &meta);
    match profile::write_both(base, &dot, &md) {
        Ok((dot_path, md_path)) => {
            ui::info!("wrote {md_path} and {dot_path}");
            code
        }
        Err(e) => {
            ui::error!("--profile: {e}");
            if !failed && code.is_none() {
                Some(1)
            } else {
                code
            }
        }
    }
}
