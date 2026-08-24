//! Project tooling. Run with `cargo xtask <command>`.
//!
//! Everything lives here rather than in shell scripts so it is cross-platform,
//! cargo-native, and type-checked by CI like the rest of the workspace.

const HELP: &str = "\
cargo xtask <command>

Oracle and conformance
  golden        Build reference/Bund and record expected final state for the
                hermetic example corpus into tests/golden/
  conform       Run Bund2 against the goldens; print N/M and fail on regression
  unblock       For each unimplemented word, count hermetic examples it alone
                gates; sort descending. This is the M6 work queue.

Evidence
  corpus        Scan the example corpus for uses of .id, .timestamp, bund.eval,
                load.lambdas, register, and post-construction LAMBDA mutation.
                Resolves decisions D1, D2, D3, D5, D12 from data.
  layout        Print size_of::<Value>() and allocation counts per operation
  arity         Probe every registered word against instrumented stacks and
                emit a first-cut stack-effect table (unblocks RFC-0004)
  bench         Criterion baseline over the corpus (Phase 0)
";

fn main() -> std::process::ExitCode {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    match cmd.as_str() {
        "golden" | "conform" | "unblock" | "corpus" | "layout" | "arity" | "bench" => {
            eprintln!("xtask: `{cmd}` is not implemented yet");
            std::process::ExitCode::from(70)
        }
        "" | "-h" | "--help" | "help" => {
            print!("{HELP}");
            std::process::ExitCode::SUCCESS
        }
        other => {
            eprintln!("xtask: unknown command `{other}`\n");
            print!("{HELP}");
            std::process::ExitCode::FAILURE
        }
    }
}
