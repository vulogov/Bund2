//! The bund2 command line runner.
//!
//! **A slice.** It parses a program with `bund2-syntax`, lowers it to a value
//! stream, and hands that to `bund2-interp`'s single evaluator — no REPL, no
//! `--emit`, no subcommands beyond `script --file`, which is what
//! `cargo xtask conform` invokes. RFC-0003's frame loop replaces the
//! evaluator's middle; the front end is now the real one.

use std::process::ExitCode;

use bund2_interp::Interp;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let file = match parse_args(&args) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("bund2: {e}");
            return ExitCode::from(2);
        }
    };
    let src = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("bund2: reading {file}: {e}");
            return ExitCode::from(2);
        }
    };
    match run(&src) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bund2: {e}");
            ExitCode::from(1)
        }
    }
}

/// `script --file <path>`, the shape `conform` and the oracle share.
fn parse_args(args: &[String]) -> Result<String, String> {
    let mut it = args.iter();
    let mut file = None;
    while let Some(a) = it.next() {
        match a.as_str() {
            "script" => {}
            "--file" => file = it.next().cloned(),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    file.ok_or_else(|| "expected: bund2 script --file <path>".to_string())
}

fn run(src: &str) -> Result<(), String> {
    // No `\n` is appended. The reference appends one at four of its five parse
    // sites to satisfy a grammar rule that demands trailing whitespace; S1
    // admits end of input as a terminator instead, so the workaround is gone.
    let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
    let mut vm = Interp::new();
    bund2_stdlib::register_all(&mut vm.registry);
    vm.eval(&stream).map_err(|e| e.0)
}
