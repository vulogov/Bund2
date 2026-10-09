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
    if cfg!(feature = "aot") {
        on.push("aot");
    }
    if cfg!(feature = "async") {
        on.push("async");
    }
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
fn main() -> ExitCode {
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
        let Ok(src) = std::fs::read_to_string(&file) else {
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
        let Ok(src) = std::fs::read_to_string(&file) else {
            bund2_api::errln!("bund2: reading {file}");
            return ExitCode::from(2);
        };
        return run_check(&src, &file);
    }
    let args = match parse_args(&args) {
        Ok(f) => f,
        Err(e) => {
            bund2_api::errln!("bund2: {e}");
            return ExitCode::from(2);
        }
    };
    let src = match std::fs::read_to_string(&args.file) {
        Ok(s) => s,
        Err(e) => {
            bund2_api::errln!("bund2: reading {}: {e}", args.file);
            return ExitCode::from(2);
        }
    };
    bund2_stdlib::host::set_args(args.script_args.clone());
    // Exit 0 unless the program asked otherwise: a Bund failure is reported,
    // not signalled. Only `bund.exit` (D52) and a failure to *start* — bad
    // arguments, an unreadable file — set an exit code.
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
}

/// `script --file <path> [-- <args>…]`, the shape `conform` and the oracle
/// share.
fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut it = args.iter();
    let mut file = None;
    let mut dump_stack = true;
    let mut raw_values = false;
    let mut stats = false;
    let mut jit_threshold: Option<u32> = None;
    let mut debugger = false;
    let mut host = bund2_stdlib::host::HostOptions::default();
    let mut script_args = Vec::new();
    while let Some(a) = it.next() {
        match a.as_str() {
            "script" => {}
            "--file" => file = it.next().cloned(),
            "--dump-stack" => dump_stack = true,
            "--no-dump-stack" => dump_stack = false,
            "--raw-values" | "--debug-values" => raw_values = true,
            "--stats" => stats = true,
            "--debugger" => debugger = true,
            // Refused rather than ignored when it is not a number: this is a
            // command line, which can report. `BUND2_JIT_THRESHOLD` cannot —
            // the runtime reads it in a constructor — so that one is ignored
            // when malformed and `--stats` prints what was actually adopted.
            "--jit-threshold" => {
                let v = it
                    .next()
                    .ok_or("--jit-threshold needs a number, as in `--jit-threshold 1`")?;
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
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(Args {
        file: file.ok_or("expected: bund2 script --file <path>")?,
        host,
        script_args,
        dump_stack,
        raw_values,
        stats,
        debugger,
        carried: false,
        jit_threshold,
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
    // it has to run. Its failures are not this command's business.
    let _ = vm.eval(&stream);

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
                    inferred.consumes, inferred.produces
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
                        inferred.consumes, inferred.produces
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
            });
        bund2_api::errln!(
            "Warning: {}:{}:{}: `{}` needs {} value(s); {} can be proven here",
            where_.file.as_deref().unwrap_or(file),
            where_.line,
            where_.column,
            f.word,
            f.needs,
            f.have
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

    let args = Args {
        // The path `bund2 build` was given, so a diagnostic from a bundle
        // names the file a `script` run of the same program would name.
        file: carried.name.clone(),
        host,
        script_args: argv,
        dump_stack: !env_set("BUND2_NO_DUMP_STACK"),
        raw_values: env_set("BUND2_RAW_VALUES"),
        stats: env_set("BUND2_STATS"),
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
        // **Nor by a word in the program — D115.** `debug.step` and the other
        // arming and moving words attach a console in a `script` run (D113.5).
        // In a bundle they do nothing: the program left in a stray
        // `debug.break` would otherwise wait at a prompt wherever it was
        // deployed, for as long as its input stayed open.
        carried: true,
    };
    bund2_stdlib::host::set_args(args.script_args.clone());
    match run(&carried.source, &args) {
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
    /// `--file <src> --output <path>`, with D78's floor.
    Bundle {
        src_path: String,
        out_path: String,
        flags: u8,
    },
}

const BUILD_USAGE: &str =
    "expected: bund2 build --file <path> --output <path> [--noio] [--noeval], \
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
    let mut flags = 0u8;
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
        if file.is_some() || output.is_some() || flags != 0 {
            return Err("`--inspect` reads an artefact and takes no other argument".to_string());
        }
        return Ok(BuildRequest::Inspect(target));
    }
    match (file, output) {
        (Some(src_path), Some(out_path)) => Ok(BuildRequest::Bundle {
            src_path,
            out_path,
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
    let (src_path, out_path, flags) = match build_request(args) {
        // `bund2 build --inspect <artefact>` — D78's readable trailer.
        //
        // **Reads a file, unlike everything else about a bundle.** A runtime
        // asks its own memory what it carries; inspecting asks about *another*
        // artefact, which is the whole point: the person handed a bundle is
        // not the person who built it.
        Ok(BuildRequest::Inspect(target)) => return inspect(&target),
        Ok(BuildRequest::Bundle {
            src_path,
            out_path,
            flags,
        }) => (src_path, out_path, flags),
        Err(why) => {
            bund2_api::errln!("bund2: build: {why}");
            bund2_api::errln!("bund2: {BUILD_USAGE}");
            bund2_api::errln!("bund2: nothing was written");
            return ExitCode::from(2);
        }
    };
    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => {
            bund2_api::errln!("bund2: reading {src_path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    // The parse, discarded on success and reported on failure.
    if let Err(e) = bund2_syntax::parse(&src) {
        let loc = locate(&src, &src_path, e.span);
        bund2_api::errln!("bund2: {src_path}:{}:{}: {}", loc.line, loc.column, e.what);
        bund2_api::errln!("bund2: nothing was written");
        return ExitCode::FAILURE;
    }

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            bund2_api::errln!("bund2: cannot locate this executable to copy: {e}");
            return ExitCode::FAILURE;
        }
    };
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
        && std::fs::canonicalize(&exe).map(|e| e == existing).unwrap_or(false)
    {
        bund2_api::errln!(
                "bund2: --output is this binary ({out_path}). Building over the \
                 executable doing the building would replace the interpreter with \
             the artefact. Nothing was written."
        );
        return ExitCode::FAILURE;
    }
    let mut image = match std::fs::read(&exe) {
        Ok(b) => b,
        Err(e) => {
            bund2_api::errln!("bund2: reading {}: {e}", exe.display());
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = bundle::write_into(
        &mut image,
        &src,
        &src_path,
        flags,
        env!("CARGO_PKG_VERSION"),
        &features_built_in(),
        &pinned_summary(),
    ) {
        bund2_api::errln!("bund2: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = place(&out_path, &image) {
        bund2_api::errln!("bund2: {e}");
        return ExitCode::FAILURE;
    }
    bund2_api::errln!(
        "bund2: wrote {out_path} — {} of {} bytes used",
        src.len(),
        bundle::CAPACITY
    );
    // **Said, not left to be discovered.** The recorded path is what a bundle's
    // diagnostics name (§B3), and it is kept whole only while it fits.
    if src_path.len() > bundle::SOURCE_NAME_BYTES {
        bund2_api::errln!(
            "bund2: the source path is {} bytes and a bundle records the last {}. \
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
/// A symlink at `--output` is still followed, as it was: the destination is
/// the link's target.
fn place(out_path: &str, image: &[u8]) -> Result<(), String> {
    use std::path::{Path, PathBuf};
    let dest: PathBuf = std::fs::canonicalize(out_path).unwrap_or_else(|_| PathBuf::from(out_path));
    let Some(name) = dest.file_name() else {
        return Err(format!("--output {out_path} names no file"));
    };
    let parent = match dest.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let scratch = parent.join(format!(".bund2-build-{}", std::process::id()));
    std::fs::create_dir(&scratch)
        .map_err(|e| format!("writing {out_path}: creating {}: {e}", scratch.display()))?;
    let staged = scratch.join(name);
    let staged_text = staged.to_string_lossy().into_owned();
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
    done.map_err(|e| format!("{e} Nothing was written to {out_path}."))
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
            bund2_api::errln!("bund2: reading {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let seen = match bundle::inspect(&image) {
        Ok(s) => s,
        Err(e) => {
            bund2_api::errln!("bund2: {path} is not a bund2 artefact: {e}");
            return ExitCode::FAILURE;
        }
    };
    bund2_api::sayln!("{path}");
    bund2_api::sayln!("  container      version {}", seen.format);
    if seen.carries_program {
        bund2_api::sayln!(
            "  program        {} of {} bytes, from {}",
            seen.used,
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

/// Returns the code a word asked to exit with (D52), if one did. **A failure
/// is not an error exit**:
/// the reference prints its report and exits 0
/// (`reference/Bund/src/stdlib/helpers/run_snippet.rs:85-90` sets no code), and
/// every golden capturing a failing program pins that. Changing it would be a
/// deviation nobody has asked for.
fn run(src: &str, args: &Args) -> Option<i32> {
    let file = args.file.as_str();
    // **Through `Runtime`, which is what installs the tier — F124.** Building
    // an `Interp` here directly is what made `--features jit` inert: the
    // feature was enabled, the binary differed, and Tier 1 was never reached,
    // so criterion 2 compared one interpreter with itself. `Runtime` registers
    // the same vocabulary with the same host options and adds the tier when the
    // feature is on; everything below reaches the interpreter through it.
    // **§D6: a debugged session installs no tier.** One call, in the one place
    // a tier is installed, which is what §D6 means by needing nothing new.
    let mut rt = if args.debugger {
        bund2_runtime::Runtime::for_debugging(&args.host, args.jit_threshold)
    } else {
        bund2_runtime::Runtime::with_options_and_threshold(&args.host, args.jit_threshold)
    };
    let vm = &mut rt.interp;
    let mut reporter = bund2_stdlib::report::TextReporter::new(args.dump_stack);
    reporter.raw_values = args.raw_values;
    vm.reporter = Box::new(reporter);
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

    // No `\n` is appended. The reference appends one at four of its five parse
    // sites to satisfy a grammar rule that demands trailing whitespace; S1
    // admits end of input as a terminator instead, so the workaround is gone.
    let terms = match bund2_syntax::parse(src) {
        Ok(t) => t,
        Err(e) => {
            vm.report(
                bund2_api::diag::Diagnostic::error(e.what.clone())
                    .at(locate(src, file, e.span)),
            );
            return None;
        }
    };
    let lowered = bund2_syntax::lower_with_spans(&terms, src.len());
    if let Err((i, e)) = vm.eval_indexed(&lowered.values) {
        let mut d = bund2_api::diag::Diagnostic::error(e.0);
        if let Some(span) = lowered.span_of(i) {
            d = d.at(locate(src, file, span));
        }
        vm.report(d);
    }
    let code = vm.exit_requested();

    // **On stderr, deliberately.** Every golden captures stdout, so a stats
    // line there would move `conform` by the whole corpus at once. RFC-0005
    // criterion 2 wants the figure reported, not the programs changed.
    //
    // `None` and `Some(0)` say different things: no tier at all — a build
    // without the feature — against a tier that compiled nothing, which over a
    // corpus is the failure criterion 2 asks this flag to catch.
    if args.stats {
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
                bund2_api::errln!("bund2: tier compiled {bodies} bodies{entered}, inlined {sites} sites");
            }
            (Some(bodies), None, _, _) => bund2_api::errln!("bund2: tier compiled {bodies} bodies{entered}"),
            _ => bund2_api::errln!("bund2: no tier (built without `jit`)"),
        }
    }
    code
}
