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
//! stream, and hands that to `bund2-interp`'s single evaluator — no REPL, no
//! `--emit`, no subcommands beyond `script --file`, which is what
//! `cargo xtask conform` invokes. RFC-0003's frame loop replaces the
//! evaluator's middle; the front end is now the real one.

use std::process::ExitCode;

use bund2_api::Vm;
use bund2_interp::Interp;

/// The evaluation thread's stack: Tier 0's share, sized to the main-thread
/// stack Bund2 ran on before (8 MiB), plus the reserve beneath the floor.
/// RFC-0005 §S8; Tier 1's share is added when the tier exists.
const EVAL_STACK: usize = 8 * 1024 * 1024 + bund2_interp::STACK_RESERVE;

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
        .stack_size(EVAL_STACK)
        .spawn(|| {
            bund2_interp::set_stack_region(bund2_interp::stack_marker(), EVAL_STACK);
            run_cli()
        });
    match spawned {
        // A panic is not a path this program has (D37); if one ever reached
        // here, failing is the only honest exit code.
        Ok(handle) => handle.join().unwrap_or(ExitCode::FAILURE),
        Err(e) => {
            eprintln!("bund2: could not start the evaluation thread: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_cli() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
            println!("{name}");
        }
        return ExitCode::SUCCESS;
    }
    // `bund2 effects` — every native's declared arity, for `xtask effects`.
    if args.first().map(String::as_str) == Some("effects") {
        let mut i = Interp::new();
        bund2_stdlib::register_all(&mut i.registry);
        for (name, e) in i.registry.declared_effects() {
            println!("{name}\t{}\t{}", e.consumes, e.produces);
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
            eprintln!("bund2: expected: bund2 infer --file <path>");
            return ExitCode::from(2);
        };
        let Ok(src) = std::fs::read_to_string(&file) else {
            eprintln!("bund2: reading {file}");
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
            eprintln!("bund2: expected: bund2 check --file <path>");
            return ExitCode::from(2);
        };
        let Ok(src) = std::fs::read_to_string(&file) else {
            eprintln!("bund2: reading {file}");
            return ExitCode::from(2);
        };
        return run_check(&src, &file);
    }
    let args = match parse_args(&args) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("bund2: {e}");
            return ExitCode::from(2);
        }
    };
    let src = match std::fs::read_to_string(&args.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("bund2: reading {}: {e}", args.file);
            return ExitCode::from(2);
        }
    };
    bund2_stdlib::host::set_args(args.script_args.clone());
    // Exit 0 either way: a Bund failure is reported, not signalled. Only a
    // failure to *start* — bad arguments, an unreadable file — is an exit code,
    // because there is no program to report against.
    let _ok = run(&src, &args);
    ExitCode::SUCCESS
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
}

/// `script --file <path> [-- <args>…]`, the shape `conform` and the oracle
/// share.
fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut it = args.iter();
    let mut file = None;
    let mut dump_stack = true;
    let mut raw_values = false;
    let mut host = bund2_stdlib::host::HostOptions::default();
    let mut script_args = Vec::new();
    while let Some(a) = it.next() {
        match a.as_str() {
            "script" => {}
            "--file" => file = it.next().cloned(),
            "--dump-stack" => dump_stack = true,
            "--no-dump-stack" => dump_stack = false,
            "--raw-values" | "--debug-values" => raw_values = true,
            "--noio" => host.noio = true,
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
        eprintln!("bund2: {file} does not parse");
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
            println!("  {name:<28} inferred opaque");
            continue;
        }
        match observe(&vm.registry, name, inferred.consumes) {
            None => {
                unobservable += 1;
                println!(
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
                    println!(
                        "  {name:<28} inferred {}->{} (net {want}) but ran net {delta}",
                        inferred.consumes, inferred.produces
                    );
                }
            }
        }
    }
    println!(
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
            eprintln!("{}", e.render(src));
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
        eprintln!(
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
    println!(
        "checked {}: {} finding(s), {} site(s) analysed, {} abandoned",
        file,
        report.findings.len(),
        report.analysed,
        report.abandoned
    );
    if !report.reasons.is_empty() {
        println!("  analysis stopped at:");
        for (why, n) in &report.reasons {
            println!("    {n:>4}  {why}");
        }
        println!(
            "  A finding is only ever about the {} site(s) above that were",
            report.analysed
        );
        println!("  tracked. Nothing is claimed about the rest.");
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
/// Returns whether evaluation succeeded. **A failure is not an error exit**:
/// the reference prints its report and exits 0
/// (`reference/Bund/src/stdlib/helpers/run_snippet.rs:85-90` sets no code), and
/// every golden capturing a failing program pins that. Changing it would be a
/// deviation nobody has asked for.
fn run(src: &str, args: &Args) -> bool {
    let file = args.file.as_str();
    let mut vm = Interp::new();
    bund2_stdlib::register_all_with(&mut vm.registry, &args.host);
    let mut reporter = bund2_stdlib::report::TextReporter::new(args.dump_stack);
    reporter.raw_values = args.raw_values;
    vm.reporter = Box::new(reporter);

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
            return false;
        }
    };
    let lowered = bund2_syntax::lower_with_spans(&terms, src.len());
    match vm.eval_indexed(&lowered.values) {
        Ok(()) => true,
        Err((i, e)) => {
            let mut d = bund2_api::diag::Diagnostic::error(e.0);
            if let Some(span) = lowered.span_of(i) {
                d = d.at(locate(src, file, span));
            }
            vm.report(d);
            false
        }
    }
}
