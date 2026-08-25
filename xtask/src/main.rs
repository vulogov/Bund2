//! Project tooling. Run with `cargo xtask <command>`.
//!
//! Everything lives here rather than in shell scripts so it is cross-platform,
//! cargo-native, and type-checked by CI like the rest of the workspace.

const HELP: &str = "\
cargo xtask <command>

Health. Two numbers, and neither substitutes for the other (Q5).
  conform       Regression: goldens passed over goldens. Prints N/M and fails
                if it drops below the mark in tests/golden/CONFORMANCE.txt.
                A fixed corpus, which is why the JIT and AOT milestones must
                move it by exactly zero. Never add words to this denominator —
                that would make implementing a word move the number and
                destroy the invariant. --accept records a new mark.
  coverage      Completeness: words with a test over words in scope. 77 goldens
                cover 140 of the 586 in-scope names, so `conform` can read
                100% with three quarters of the language untested. This is the
                number that says so.

Oracle
  golden        Capture expected final state for the suite in
                tests/golden/HERMETIC.txt into tests/golden/*.golden. Each
                program is run twice and refused if the runs differ; output is
                normalised for F14 (id/stamp) and F15 (dict order) first.
                Needs the oracle built out-of-tree. An existing golden that
                would change is left alone unless named:
                  --accept <name> --reason <F-number or decision>
  unblock       NEEDS REDESIGN (Q15). Ranks unimplemented words by the hermetic
                examples each alone gates — which can only ever see the 140
                in-scope words the goldens touch. As specified it would report
                an empty work queue with 446 words unimplemented. Rank against
                the coverage denominator instead.

Evidence
  corpus        Scan the example corpus for uses of .id, .timestamp, bund.eval,
                load.lambdas, register, and post-construction LAMBDA mutation;
                cross-reference every invoked word against the reference
                registration sets; group words by implementing subsystem; and
                write the hermetic program list to tests/golden/HERMETIC.txt.
                Reads .bund files only — it builds nothing. Gathers evidence
                for D1, D2, D3, D5, D12 and D14; it resolves none of them.
  layout        Phase 0: size_of and allocations per operation for CANDIDATE
                value representations, since BundValue does not exist yet.
                Makes RFC-0001's 16-byte claim checkable before it is written.
  arity         First-cut stack-effect table, written to docs/arity.md. Two
                passes: the word's own `current_stack_len() < N` guard (static),
                and consumed/produced observed by running it against the oracle.
                Only hermetic, in-scope words are ever executed. --static-only
                skips the oracle. Unblocks RFC-0004 and D24.
  bench         Phase 0: wall-clock baseline over the suite. Not Criterion —
                this times a whole subprocess, where cost is dominated by
                process start; Criterion measures in-process and would measure
                the wrong thing precisely. --target oracle|bund2, --runs N,
                --write to record docs/bench-baseline.md.
";

mod arity;
mod bench;
mod conform;
mod corpus;
mod golden;
mod layout;

/// Counting allocator, so `layout` can report allocations per operation.
#[global_allocator]
static ALLOC: layout::Counting = layout::Counting;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let cmd = std::env::args().nth(1).unwrap_or_default();
    match cmd.as_str() {
        "corpus" => match corpus::run(&args) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("xtask corpus: {err}");
                std::process::ExitCode::FAILURE
            }
        },
        "coverage" => match corpus::run_coverage(&args) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("xtask coverage: {err}");
                std::process::ExitCode::FAILURE
            }
        },
        "arity" => match arity::run(&args) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("xtask arity: {err}");
                std::process::ExitCode::FAILURE
            }
        },
        "golden" => match golden::run(&args) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("xtask golden: {err}");
                std::process::ExitCode::FAILURE
            }
        },
        "conform" => match conform::run(&args) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("xtask conform: {err}");
                std::process::ExitCode::FAILURE
            }
        },
        "layout" => match layout::run(&args) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("xtask layout: {err}");
                std::process::ExitCode::FAILURE
            }
        },
        "bench" => match bench::run(&args) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("xtask bench: {err}");
                std::process::ExitCode::FAILURE
            }
        },
        "unblock" => {
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
