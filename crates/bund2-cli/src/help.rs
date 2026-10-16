//! `bund2 help`, `--help`, `-h` and `--version` — D173.
//!
//! The text is written here, by hand, because the command line is parsed
//! by hand: there is no declarative table to generate it from, and a help
//! that drifts from the parser is worse than none. Every flag named below
//! is matched in `main.rs`'s `parse_args`, `build_request` or `ui`; a flag
//! added to one without the other is what the process-level tests in
//! `tests/cli_ux.rs` exist to catch for the common ones.

/// The whole of it: every command, flag and environment switch.
pub const USAGE: &str = "\
bund2 - the Bund language, concatenative, OOP, metaprogramming tool

USAGE
  bund2 [GLOBAL] script --file <path> [FLAGS] [-- <args>…]
  bund2 [GLOBAL] [FLAGS] <path> [<args>…]             the flags first, then
                                                      the path, then the
                                                      program's arguments
  bund2 [GLOBAL] build --file <path> --output <path> [--noio] [--noeval]
  bund2 [GLOBAL] build --inspect <artefact>
  bund2 [GLOBAL] check --file <path>
  bund2 [GLOBAL] infer --file <path>
  bund2 words | effects
  bund2 help [<command>] | --help | -h
  bund2 --version | -V

COMMANDS
  script    Run a program. Everything after `--` is the program's `args`.
  build     Write the program into a copy of this binary: a bundle that runs
            it, restricted by --noio and --noeval (RFC-0006).
  check     Report where a program would underflow, without running it.
  infer     Run a program, then compare each registered word's inferred
            stack effect with the one observed.
  words     Every name a program can call, one per line.
  effects   Every native's declared stack effect, as name, consumes, produces.

GLOBAL FLAGS (anywhere before `--`)
  -v, -vv, -vvv       Say more: info, debug, trace. Also raises what the
                      program's log.* words may say (log.info under -v).
  --verbose=<level>   One of error, warning, info, debug, trace.
  -q, --quiet         Errors only: warnings are dropped, notices are kept.
  -h, --help          This text, or `help <command>` for one command.
  -V, --version       The version, the features built in, the oracle pinned.

ENVIRONMENT
  BUND2_VERBOSE       A level, for a bundle (its command line is its program's).
  BUND2_REPORT        auto | box | trace: how a fatal error is drawn.
  BUND2_NOCOLOR, NO_COLOR
                      No colour on a terminal.
  BUND_LOG_LEVEL      The log.* threshold, as the reference reads it.
  BUND2_JIT_THRESHOLD How many runs of a body earn compilation (jit builds).
  BUND2_OOM_GUARD_GB  The memory guard's floor in GB; 0 disables it (D171).
  BUND2_OOM_GUARD_SUSTAINED
                      Samples of critical pressure before the guard trips.
  BUND2_NOIO, BUND2_NOEVAL, BUND2_NO_DUMP_STACK, BUND2_RAW_VALUES, BUND2_STATS
                      A bundle's switches, each the flag of the same name.

Run `bund2 help script` or `bund2 help build` for a command's flags.
";

pub const SCRIPT: &str = "\
bund2 script — run a program

USAGE
  bund2 [GLOBAL] script --file <path> [FLAGS] [-- <args>…]
  bund2 [GLOBAL] [script] [FLAGS] <path> [<args>…]

A flag that takes a value takes it either way: `--file p.bund` or
`--file=p.bund`. A bare path ends the flags: everything after it is the
program's `args`, which is how an executable file whose first line is
`#!/usr/bin/env bund2` receives what it was run with. That first line is
skipped; the file's line numbers are unchanged.

FLAGS
  --file <path>         The program. With this form the flags may follow.
  --report <mode>       How a fatal error is drawn: auto (the traceback on a
                        terminal, the reference's table on a pipe), box, or
                        trace. BUND2_REPORT sets the default.
  --dump-stack          Show the stack under a fatal error (the default).
  --no-dump-stack       The reason and the position alone.
  --raw-values          Raw value renderings in the dump, as a debug session
                        wants them; the default is a summary that fits.
  --stats               What the tier did, on standard error (jit builds).
  --jit-threshold <n>   Runs of a body before it is compiled (jit builds).
  --debugger            Stop before every value and take commands on stdin.
  --debug-shell         After a failure, open debug.shell on the VM as the
                        program left it.
  --noio                The I/O words fail instead of touching the host.
  --noeval              The evaluating words fail (bund.eval, use, …).
  --nocolor             No colour, as the reference's flag.
  --                    Everything after it is the program's `args`.

A failing program is reported and exits 0, as the reference's does; only
bund.exit, and a failure to start, set an exit code.
";

pub const BUILD: &str = "\
bund2 build — write a program into a copy of this binary

USAGE
  bund2 [GLOBAL] build --file <path> --output <path> [--include <path>]...
                       [--noio] [--noeval]
  bund2 [GLOBAL] build --inspect <artefact>

FLAGS
  --file <path>         The program to carry.
  --output, -o <path>   Where the bundle goes. Never this binary, never the
                        program's own file, never an included one.
  --include <path>      A file to carry ahead of the program and run before
                        it, in the program's VM; repeat for several, in the
                        order they should run. Words a file registers are
                        there when the program starts. A failure in one
                        names that file, and the program does not run.
  --noio, --noeval      Restrictions recorded in the bundle: a floor the
                        environment may tighten (BUND2_NOIO, BUND2_NOEVAL)
                        and never lift.
  --emit bundle         The one mode there is.
  --inspect <artefact>  Print what a bundle says about itself: its program,
                        its restrictions, the bund2 that built it, and the
                        oracle it conformed to.

The bundle is this binary, so it carries this binary's features; a
different runtime needs a different bund2. The program and its includes
are stored deflated in a 1 MiB region, which holds two to three MiB of
Bund text; a program whose compressed form does not fit is refused with
both sizes named. `--include` carries a prelude; `use` inside a program
still fetches at run time (D76). Progress is drawn on a terminal
and the steps are said at -v, -vv and -vvv.
";

pub const CHECK: &str = "\
bund2 check — report where a program would underflow

USAGE
  bund2 [GLOBAL] check --file <path>

Never runs the program. Prints each finding as a warning, then how many
sites were analysed and how many abandoned, and why. Exit 0 either way:
`!` is opaque and a false positive is certain, so check does not gate.
";

pub const INFER: &str = "\
bund2 infer — compare inferred and observed stack effects

USAGE
  bund2 [GLOBAL] infer --file <path>

Runs the program so its `register` calls happen, then for every word it
registered prints the inferred effect beside the one observed by calling
the word on integer sentinels. Exit 1 when any pair disagrees.
";

/// The text for one command, or `None` for a name that is not one.
pub fn for_command(name: &str) -> Option<&'static str> {
    match name {
        "script" => Some(SCRIPT),
        "build" => Some(BUILD),
        "check" => Some(CHECK),
        "infer" => Some(INFER),
        "words" => Some("bund2 words — every name a program can call, one per line.\n"),
        "effects" => Some(
            "bund2 effects — every native's declared stack effect: name, consumes, produces.\n",
        ),
        _ => None,
    }
}

/// `bund2 --version`: the version, the features, and the oracle pinned.
pub fn version(features: &str, pinned: &str) -> String {
    let mut out = format!("bund2 {}", env!("CARGO_PKG_VERSION"));
    if features.is_empty() {
        out.push_str(" (default features)");
    } else {
        out.push_str(&format!(" (features: {features})"));
    }
    if !pinned.is_empty() {
        out.push_str(&format!("\nconformed to {pinned}"));
    }
    out.push('\n');
    out
}

/// What a command line asked for, if it asked for text and not work.
#[derive(Debug, PartialEq, Eq)]
pub enum Asked {
    Usage,
    Command(&'static str),
    Version,
    /// `help <name>` for a name that is not a command.
    Unknown(String),
}

/// Whether `args` asks for help or the version: `help [<command>]`,
/// `-h`/`--help` anywhere before `--` (for the command named first, when
/// one is), `-V`/`--version`. `None` means there is work to do.
pub fn asked(args: &[String]) -> Option<Asked> {
    let first = args.first().map(String::as_str);
    if first == Some("help") {
        return Some(match args.get(1).map(String::as_str) {
            None => Asked::Usage,
            Some(name) => for_command(name)
                .map(Asked::Command)
                .unwrap_or_else(|| Asked::Unknown(name.to_string())),
        });
    }
    let before_dashes = args.iter().take_while(|a| *a != "--");
    let mut wants_help = false;
    for a in before_dashes {
        match a.as_str() {
            "-h" | "--help" => wants_help = true,
            "-V" | "--version" => return Some(Asked::Version),
            _ => {}
        }
    }
    if wants_help {
        return Some(match first.and_then(for_command) {
            Some(text) => Asked::Command(text),
            None => Asked::Usage,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(args: &[&str]) -> Option<Asked> {
        asked(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn help_is_asked_in_every_spelling_and_for_one_command() {
        assert_eq!(ask(&["help"]), Some(Asked::Usage));
        assert_eq!(ask(&["--help"]), Some(Asked::Usage));
        assert_eq!(ask(&["-h"]), Some(Asked::Usage));
        assert_eq!(ask(&["help", "build"]), Some(Asked::Command(BUILD)));
        assert_eq!(ask(&["build", "--help"]), Some(Asked::Command(BUILD)));
        assert_eq!(
            ask(&["script", "--file", "p", "-h"]),
            Some(Asked::Command(SCRIPT))
        );
        assert_eq!(ask(&["help", "nope"]), Some(Asked::Unknown("nope".into())));
        assert_eq!(ask(&["--version"]), Some(Asked::Version));
        assert_eq!(ask(&["script", "-V"]), Some(Asked::Version));
        // After `--` the flags are the program's.
        assert_eq!(ask(&["script", "--file", "p", "--", "--help"]), None);
        assert_eq!(ask(&["script", "--file", "p"]), None);
    }

    /// Every flag the parsers match is in the text, and nothing the text
    /// names is unknown to them: the two are kept by hand.
    #[test]
    fn the_text_names_the_flags_the_parsers_take() {
        for flag in [
            "--file",
            "--report",
            "--dump-stack",
            "--no-dump-stack",
            "--raw-values",
            "--stats",
            "--jit-threshold",
            "--debugger",
            "--debug-shell",
            "--noio",
            "--noeval",
            "--nocolor",
        ] {
            assert!(SCRIPT.contains(flag), "script help lacks {flag}");
        }
        for flag in [
            "--file",
            "--output",
            "--inspect",
            "--noio",
            "--noeval",
            "--emit",
        ] {
            assert!(BUILD.contains(flag), "build help lacks {flag}");
        }
        for flag in ["-v", "--verbose=", "-q", "--help", "--version"] {
            assert!(USAGE.contains(flag), "usage lacks {flag}");
        }
        assert!(version("", "").starts_with("bund2 0."));
        assert!(version("jit", "Bund:abc").contains("features: jit"));
    }
}
