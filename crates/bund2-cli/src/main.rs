//! The bund2 command line runner.
//!
//! **A slice.** It lexes a program with `bund2-syntax`, dispatches each term
//! through `bund2-interp`, and stops there — no REPL, no `--emit`, no
//! subcommands beyond `script --file`, which is what `cargo xtask conform`
//! invokes. RFC-0003 replaces the middle of it with an IR and a frame loop.

use std::process::ExitCode;

use bund2_api::Vm;
use bund2_interp::Interp;
use bund2_syntax::{Term, lex};
use bund2_value::BundValue;

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
    let terms = lex(src).map_err(|e| e.to_string())?;
    let mut vm = Interp::new();
    bund2_stdlib::register_all(&mut vm.registry);

    for t in terms {
        match t {
            Term::Int(i) => vm.push(BundValue::Int(i)),
            Term::Float(f) => vm.push(BundValue::Float(f)),
            Term::Str(s) => vm.push(BundValue::str(s)),
            Term::Name(n) => vm.dispatch_name(&n).map_err(|e| e.0)?,
        }
    }
    Ok(())
}
