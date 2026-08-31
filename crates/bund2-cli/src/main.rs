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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
    // Exit 0 either way: a Bund failure is reported, not signalled. Only a
    // failure to *start* — bad arguments, an unreadable file — is an exit code,
    // because there is no program to report against.
    let _ok = run(&src, &args.file, args.dump_stack);
    ExitCode::SUCCESS
}

struct Args {
    file: String,
    /// Whether an error report carries the stack. On by default, as the
    /// reference always dumps; `--no-dump-stack` is for a caller that wants
    /// the reason alone.
    dump_stack: bool,
}

/// `script --file <path>`, the shape `conform` and the oracle share.
fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut it = args.iter();
    let mut file = None;
    let mut dump_stack = true;
    while let Some(a) = it.next() {
        match a.as_str() {
            "script" => {}
            "--file" => file = it.next().cloned(),
            "--dump-stack" => dump_stack = true,
            "--no-dump-stack" => dump_stack = false,
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(Args {
        file: file.ok_or("expected: bund2 script --file <path>")?,
        dump_stack,
    })
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
fn run(src: &str, file: &str, dump_stack: bool) -> bool {
    let mut vm = Interp::new();
    bund2_stdlib::register_all(&mut vm.registry);
    vm.reporter = Box::new(bund2_stdlib::report::TextReporter::new(dump_stack));

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
