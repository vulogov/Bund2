//! The terminal words: `input`, `input*`, `password`, `bund.prompt`,
//! `io.banner`, `debug.display_hostinfo`, `debug.display_memstat`,
//! `debug.display_distributed_info`, and the two debug REPLs `debug` and
//! `debug.shell`.
//!
//! **What a golden can hold.** The capture runs a program with stdin at
//! end-of-file, so `input` and `input*` only ever see Ctrl-D there. That path
//! is the reference's own: it pushes nothing and runs nothing
//! (`reference/Bund/src/stdlib/functions/io/input.rs:46-51,117-124`). The
//! prompt is a fixed string with fixed colour codes, because the reference
//! never turns yansi off. `io.banner` falls back to an 80-column width when
//! stdout is not a terminal, as it is under capture. The host table depends on
//! the machine, so no golden can hold it.

use bund2_api::{diag::Diagnostic, Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, EXIT, LAMBDA, NONE, STRING};
use rustyline::error::ReadlineError;

use crate::host::{guard, HostOptions};
use crate::wb::Side;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// The prompt a reader was given, or `"> "` when there was none or it would
/// not cast (`input.rs:30-38`).
fn prompt_of(v: Option<BundValue>) -> String {
    v.and_then(|v| v.as_str()).unwrap_or_else(|| "> ".to_string())
}

/// **The one place this crate reads a line from the terminal** — F158.
///
/// `input`, `input*`, `debug` and `debug.shell` all read through
/// [`Terminal::line`], and [`secret`] is the same thing for `password`. Nothing
/// else in `bund2-stdlib` may touch standard input, and
/// `every_terminal_read_goes_through_the_two_helpers`
/// (`crates/bund2-stdlib/src/lib.rs`) scans the crate to say so.
///
/// # Why there is one place
///
/// Under this crate's **own test build** a read answers end-of-file without
/// touching standard input at all. That is not a convenience. Criterion 24's
/// corpus audit runs every probe that has a golden *in process*, and three of
/// those probes call the words above; with standard input at end-of-file each
/// read returns at once, and with a pipe that stays open each waits forever.
/// A `cargo test` moved to the background is given exactly such a pipe. Four
/// test binaries were found blocked that way, the oldest five hours old, each
/// holding the build lock (F158) — and F141 had recorded the same hazard a
/// month earlier and closed only the path it found.
///
/// The rule that would have prevented it, "give `cargo test` a closed stdin",
/// was true and unenforced, which is the kind that fails. This is enforced:
/// the test build cannot read a terminal, and a new reader that bypasses these
/// helpers fails a test instead of hanging a suite.
///
/// # What a test therefore does and does not exercise
///
/// It exercises each word's **end-of-input arm**, which is the only arm a
/// test or a capture has ever reached — the golden runner gives a program
/// stdin at `/dev/null`. It does not exercise `rustyline` itself. Nothing was
/// lost that a test had: no test could type.
///
/// # Where this is going
///
/// `Vm::report` is the seam a TUI implements for *output* (D36). The same is
/// owed for input, since a word that calls `rustyline` on raw standard input
/// cannot run inside one. That seam is not built; this helper is the
/// consolidation it needs, and where it would plug in.
struct Terminal {
    #[cfg(not(test))]
    rl: rustyline::DefaultEditor,
}

impl Terminal {
    fn open() -> Result<Self, Error> {
        #[cfg(not(test))]
        let terminal = Self {
            rl: rustyline::DefaultEditor::new()
                .map_err(|e| Error(format!("INPUT returns: {e}")))?,
        };
        #[cfg(test)]
        let terminal = Self {};
        Ok(terminal)
    }

    /// One line, with `rustyline`'s own result type so every caller keeps the
    /// match it had: a line, `Interrupted` or `Eof` for the end, anything else
    /// a failure.
    fn line(&mut self, prompt: &str) -> Result<String, ReadlineError> {
        #[cfg(not(test))]
        {
            self.rl.readline(prompt)
        }
        #[cfg(test)]
        {
            let _ = prompt;
            Err(ReadlineError::Eof)
        }
    }
}

/// The other place: a line read without echo, for `password`.
///
/// It goes through `yapp` rather than `rustyline`, which is why a fix at the
/// line reader alone would have left it reading. Under the test build it
/// refuses, naming why, rather than answering an empty secret a caller might
/// take for a real one.
fn secret(prompt: &str) -> Result<String, String> {
    #[cfg(not(test))]
    {
        use yapp::PasswordReader;
        yapp::Yapp::new()
            .with_echo_symbol('.')
            .read_password_with_prompt(prompt)
            .map_err(|e| e.to_string())
    }
    #[cfg(test)]
    {
        let _ = prompt;
        Err("no terminal is read under the test build (F158)".to_string())
    }
}

/// `input` — read one line from the terminal (`input.rs:20-57`).
///
/// The prompt is pulled, and the line is pushed with its surrounding space
/// trimmed. Ctrl-C and Ctrl-D push nothing and are not errors, so the word
/// answers zero or one value: its effect is opaque.
fn input(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline INPUT".into()));
    }
    let mut rl = Terminal::open()?;
    let prompt = prompt_of(vm.pull());
    match rl.line(&prompt) {
        Ok(line) => vm.push(BundValue::str(line.trim())),
        Err(ReadlineError::Interrupted | ReadlineError::Eof) => {}
        Err(e) => return Err(Error(format!("INPUT line returns: {e}"))),
    }
    Ok(())
}

/// `input*` — read lines until Ctrl-C or Ctrl-D, running a lambda on each
/// (`input.rs:76-131`). The lambda is on top and the prompt beneath it.
///
/// **The lambda's type is never checked** (F108): the reference writes
/// `! lambda_value.type_of() == LAMBDA` (`:91`), a bitwise NOT of the tag
/// compared with `LAMBDA`, which is always false. So anything is accepted, and
/// a value that is not a lambda fails only when the first line arrives, where
/// the reference's lambda evaluator refuses it:
/// `reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:27-29`.
fn input_loop(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline INPUT".into()));
    }
    let mut rl = Terminal::open()?;
    let lambda = vm
        .pull()
        .ok_or_else(|| Error("Error getting INPUT* lambda from stack".into()))?;
    let prompt = prompt_of(vm.pull());
    loop {
        match rl.line(&prompt) {
            Ok(line) => {
                vm.push(BundValue::str(line.trim()));
                if lambda.dt() != LAMBDA {
                    return Err(Error(
                        "INPUT* returned error from LAMBDA: This is not a lambda".into(),
                    ));
                }
                vm.eval_lambda(&lambda)
                    .map_err(|e| Error(format!("INPUT* returned error from LAMBDA: {}", e.0)))?;
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(e) => return Err(Error(format!("INPUT line returns: {e}"))),
        }
    }
    Ok(())
}

/// `password` — read a line without echoing it (`input.rs:59-74`). Each key
/// shows as a `.`.
fn password(vm: &mut dyn Vm) -> Result<(), Error> {
    let m = vm
        .pull()
        .ok_or_else(|| Error("PASSWORD: NO DATA #1".into()))?;
    let msg = m.as_str().ok_or_else(|| {
        Error("PASSWORD error casting message: This Dynamic type is not string".into())
    })?;
    let res = secret(&msg).map_err(|e| Error(format!("PASSWORD returns: {e}")))?;
    vm.push(BundValue::str(res));
    Ok(())
}

/// `bund.prompt` — push the coloured `[BUND> ` prompt (`input.rs:14-18`).
fn bund_prompt(vm: &mut dyn Vm) -> Result<(), Error> {
    use yansi::Paint;
    let prompt = format!(
        "{}{}{}{}{} {} ",
        Paint::yellow("["),
        Paint::red("B"),
        Paint::blue("U").bold(),
        Paint::white("N"),
        Paint::cyan("D"),
        Paint::green(">").bold()
    );
    vm.push(BundValue::str(prompt));
    Ok(())
}

/// The `[DEBUG> ` prompt both debug REPLs show — the same three paints in
/// `debug_shell.rs` and `debug_debug.rs`.
fn debug_prompt() -> String {
    use yansi::Paint;
    format!(
        "{}{} {} ",
        Paint::yellow("["),
        Paint::red("DEBUG"),
        Paint::white(">").bold()
    )
}

/// `debug.shell` — a REPL on the running VM
/// (`reference/Bund/src/stdlib/functions/debug_fun/debug_shell.rs`,
/// `debug_shell`).
///
/// Each line is evaluated against the *live* VM, so the session sees and
/// changes the stacks the program built. A line that fails does not end the
/// loop.
///
/// **Under capture it is a no-op**, which is the only reason a golden can hold
/// it: the runner gives a program stdin at `/dev/null`, so the first read is
/// end-of-file and the word returns having done nothing. Measured on the
/// oracle -- `debug.shell "after" println` prints `after`.
///
/// **Opaque, and that is what keeps it out of D55's palette.** It evaluates
/// whatever is typed, so it has no effect to declare; the audit skips opaque
/// natives before running anything, so F141's hang hazard is answered by the
/// type rather than by a list the next session has to remember.
///
/// Two departures from the reference, both deliberate. It **reports** a failing
/// line rather than printing it, because D36 says a word does not write to
/// stderr itself; the severity is `Warning` because the program has not
/// stopped. And a readline error that is neither Ctrl-C nor Ctrl-D ends the
/// word instead of looping -- the reference prints and continues, which spins
/// if the condition persists, and `input*` here already chose to error out.
fn debug_shell(vm: &mut dyn Vm) -> Result<(), Error> {
    let mut rl = Terminal::open()?;
    let prompt = debug_prompt();
    loop {
        match rl.line(&prompt) {
            Ok(line) => {
                if let Err(e) = crate::singles::eval_source(vm, &line) {
                    vm.report(Diagnostic::warning(e.0));
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(e) => return Err(Error(format!("INPUT line returns: {e}"))),
        }
    }
    Ok(())
}

/// One value's three-row table, as `debug` shows it before applying it
/// (`debug_debug.rs`, `bund_debug_print_word`).
///
/// The rows are the type name, the value converted to STRING, and the
/// `Debug` form. A conversion that fails shows `NONE`, as the reference's
/// `Value::none()` does.
///
/// **The third row is `render`, not Rust's `{:?}`.** `BundValue` is not the
/// reference's `Value`, so its derived `Debug` prints
/// `Int(1, StackSym(0))` where the oracle prints
/// `Value { id: …, dt: 2, …, data: I64(1), … }`. `render` is the emulation
/// `debug.display_stack`'s goldens are built on, and this row is the same
/// claim, so it uses the same function. Found by diffing against the oracle,
/// not by reading.
fn debug_print_word(v: &BundValue) {
    use comfy_table::modifiers::UTF8_ROUND_CORNERS;
    use comfy_table::presets::UTF8_FULL;
    use comfy_table::{ContentArrangement, Table};
    let shown = crate::convert::conv_value(v, STRING)
        .map(|s| s.display())
        .unwrap_or_else(|_| BundValue::none().display());
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .add_row(vec!["Value type", v.type_name()])
        .add_row(vec!["Value", &shown])
        .add_row(vec!["Debug", &v.render(false)]);
    println!("{table}");
}

/// `debug` — step a snippet, one top-level value at a time
/// (`debug_debug.rs`, `debug_debug` and `bund_debugger`).
///
/// The operand is a STRING of Bund source. It is parsed, and for each value:
/// the table above is printed, the value is **applied to the live VM**, and
/// then lines are read and evaluated until a blank one or end-of-file moves on
/// to the next value. So the pause is *after* each value, and the stack a
/// session inspects is the one that value left.
///
/// **The `EXIT` arm is load-bearing and nearly went missing.** A compiled
/// stream ends with an `EXIT` value (dt 93), so without the break this word
/// prints a fourth table for it, applies it, and leaves it on the stack --
/// which is what the first version did, and what diffing against the oracle
/// showed. The reference breaks on `EXIT`, and so does this.
///
/// The `ERROR` arm is the one with no representable input: Bund2's value model
/// has no such tag, so "bail on an error posted on the stack" is absent by
/// construction rather than written as an arm that cannot run -- D37's first
/// preference. A `NONE` value is skipped, as in the reference.
///
/// Opaque, for the same reason `debug.shell` is.
fn debug_word(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline DEBUG".into()));
    }
    // The reference guards the depth and *then* pulls, with a second message
    // for a pull that cannot fail after the guard. Both are reproduced.
    let v = vm
        .pull()
        .ok_or_else(|| Error("Stack is too shallow for debug()".into()))?;
    let snippet = v.as_str().ok_or_else(|| {
        Error("Casting debug snippet returns: This Dynamic type is not string".into())
    })?;
    // `\n` appended, as `bund_debugger` does before parsing.
    let source = format!("{snippet}\n");
    let words = bund2_syntax::compile(&source).map_err(|e| Error(e.render(&source)))?;
    let mut rl = Terminal::open()?;
    let prompt = debug_prompt();
    for word in words {
        if word.dt() == NONE {
            continue;
        }
        if word.dt() == EXIT {
            break;
        }
        debug_print_word(&word);
        vm.apply(word.clone())
            .map_err(|e| {
                Error(format!(
                    "Attempt to evaluate value {} returned error: {}",
                    word.render(false),
                    e.0
                ))
            })?;
        loop {
            match rl.line(&prompt) {
                Ok(line) if line.is_empty() => break,
                Ok(line) => {
                    if let Err(e) = crate::singles::eval_source(vm, &line) {
                        vm.report(Diagnostic::warning(e.0));
                    }
                }
                Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
                Err(e) => return Err(Error(format!("INPUT line returns: {e}"))),
            }
        }
    }
    Ok(())
}

/// `debug.display_memstat` — this process's own memory use
/// (`reference/Bund/src/stdlib/functions/debug_fun/debug_display_memstats.rs`).
///
/// A `--nocolor` pair like `debug.display_hostinfo`, and the two differ only in
/// the cell colours. **No golden can hold the table**: the figures are this
/// process's, and two oracle runs a second apart reported 28.51 MB and
/// 28.56 MB. They are also textually identical under capture, because neither
/// comfy_table nor yansi colours a pipe -- so the `--nocolor` distinction is
/// unobservable there, which is F66's shape.
///
/// `memory_stats` answering `None` prints the reference's one-line fallback
/// rather than failing.
fn debug_display_memstat(vm: &mut dyn Vm) -> Result<(), Error> {
    let _ = vm;
    println!("{}", memstat_table());
    Ok(())
}

/// The table `debug.display_memstat` prints, built separately so a test can
/// read it without capturing stdout -- `hostinfo_table` is factored the same
/// way and for the same reason.
fn memstat_table() -> String {
    use comfy_table::modifiers::UTF8_ROUND_CORNERS;
    use comfy_table::presets::UTF8_FULL;
    use comfy_table::{ContentArrangement, Table};
    let Some(usage) = memory_stats::memory_stats() else {
        return "Couldn't get the current memory usage.".to_string();
    };
    use humansize::{format_size, DECIMAL};
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .add_row(vec![
            "Current physical memory usage",
            &format_size(usage.physical_mem, DECIMAL),
        ])
        .add_row(vec![
            "Current virtual memory usage",
            &format_size(usage.virtual_mem, DECIMAL),
        ]);
    format!("{table}")
}

/// `debug.display_distributed_info` — the bus node's identity
/// (`reference/Bund/src/stdlib/functions/debug_fun/debug_display_distributed_info.rs`).
///
/// **Its first act is to require `--distributed`**, and failing that it logs
/// one line and returns `Ok`. Bund2 has no `--distributed` and links no zenoh
/// session, so that is the word's whole reachable behaviour here: the table it
/// would otherwise draw needs a bus Bund2 does not have, which RFC-0007 and
/// D92 govern rather than this word.
///
/// The reference logs at ERROR and keeps going; this reports at `Warning`,
/// since the program has not stopped (D36), and the line is therefore the
/// reporter's rather than a timestamped `log::error!`. No golden can hold
/// either: the oracle's carries a wall clock, and stderr is concatenated into
/// the captured output -- the same reason `log.error` is permanently
/// unreachable.
fn debug_display_distributed_info(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.report(Diagnostic::warning(
        "BUND must be in distributed mode. You shall pass --distributed to CLI",
    ));
    Ok(())
}

/// `io.banner` — draw a value as large text in cfonts' tiny font
/// (`reference/Bund/src/stdlib/functions/io/banner.rs:11-60`).
///
/// The value is converted to STRING first (`:32`), so any value draws as it
/// prints. The banner always goes to the stack (`:43`), the `.` form included.
fn io_banner(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #1")))?;
    let s = crate::convert::conv_value(&v, STRING)
        .map_err(|e| Error(format!("{prefix} returns: NO CONVERSION #1: {}", e.0)))?;
    let text = s.as_str().ok_or_else(|| {
        Error(format!(
            "{prefix} returns: NO CASTING #1: This Dynamic type is not string"
        ))
    })?;
    let out = cfonts::render(cfonts::Options {
        text,
        font: cfonts::Fonts::FontTiny,
        ..cfonts::Options::default()
    })
    .text;
    vm.push(BundValue::str(out));
    Ok(())
}

/// The table `debug.display_hostinfo` prints
/// (`reference/Bund/src/stdlib/functions/debug_fun/debug_display_hostinfo.rs:12-136`).
///
/// **D53: the six version rows name Bund2's crates.** The reference lists its
/// own internal crates there (`:39-56`). Bund2's crates share one workspace
/// version, which is this crate's. The host rows are the reference's, read
/// through the same `sys_metrics`. A value that cannot be read shows as
/// `Unknown` (`:23-34`). Bund2 has no distributed mode, so that row is always
/// `false`.
fn hostinfo_table(color: bool) -> String {
    use comfy_table::modifiers::UTF8_ROUND_CORNERS;
    use comfy_table::presets::UTF8_FULL;
    use comfy_table::{Cell, Color, ContentArrangement, Table};
    let or_unknown = |r: Result<String, _>| r.unwrap_or_else(|_: std::io::Error| "Unknown".to_string());
    let version = env!("CARGO_PKG_VERSION").to_string();
    let rows: Vec<(&str, String, Color)> = vec![
        ("bund2-value version", version.clone(), Color::Green),
        ("bund2-api version", version.clone(), Color::Green),
        ("bund2-syntax version", version.clone(), Color::Green),
        ("bund2-ir version", version.clone(), Color::Green),
        ("bund2-interp version", version.clone(), Color::Green),
        ("bund2-stdlib version", version, Color::Green),
        ("Distributed mode", "false".to_string(), Color::Blue),
        ("Hostname", or_unknown(sys_metrics::host::get_hostname()), Color::Blue),
        ("OS version", or_unknown(sys_metrics::host::get_os_version()), Color::Blue),
        ("Virtualization", crate::sysinfo::virtualization(), Color::Blue),
        (
            "Kernel version",
            or_unknown(sys_metrics::host::get_kernel_version()),
            Color::Blue,
        ),
    ];
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    for (k, v, c) in rows {
        if color {
            table.add_row(vec![Cell::new(k).fg(c), Cell::new(v).fg(Color::White)]);
        } else {
            table.add_row(vec![Cell::new(k), Cell::new(v)]);
        }
    }
    table.to_string()
}

pub fn register(r: &mut Registry, opts: &HostOptions) {
    // `reference/Bund/src/stdlib/functions/io/input.rs:142-145`. `input` and
    // `input*` are opaque: one answers zero or one value, the other runs a
    // body per line.
    r.register_native("input", input, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("input*", input_loop, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("password", password, eff(1, 1), WordKind::Sync);
    r.register_native("bund.prompt", bund_prompt, eff(0, 1), WordKind::Sync);
    // `reference/Bund/src/stdlib/functions/io/banner.rs:82-88`.
    if opts.noio {
        for name in ["io.banner", "io.banner."] {
            r.register_native(
                name,
                |_vm| Err(Error("bund IO.BANNER functions disabled with --noio".into())),
                if name == "io.banner" { eff(1, 1) } else { eff(0, 1) },
                WordKind::Sync,
            );
        }
    } else {
        r.register_native(
            "io.banner",
            |vm| io_banner(vm, Side::Stack, "IO.BANNER"),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "io.banner.",
            |vm| io_banner(vm, Side::Bench, "IO.BANNER."),
            eff(0, 1),
            WordKind::Sync,
        );
    }
    // `bund/debug_fun`'s remaining four. `debug` and `debug.shell` are opaque
    // because they evaluate whatever is typed -- which also keeps them out of
    // D55's palette, where a word that reads stdin is F141's hazard.
    r.register_native("debug", debug_word, StackEffect::opaque(1), WordKind::Sync);
    r.register_native(
        "debug.shell",
        debug_shell,
        StackEffect::opaque(0),
        WordKind::Sync,
    );
    r.register_native(
        "debug.display_memstat",
        debug_display_memstat,
        eff(0, 0),
        WordKind::Sync,
    );
    r.register_native(
        "debug.display_distributed_info",
        debug_display_distributed_info,
        eff(0, 0),
        WordKind::Sync,
    );

    // `debug_display_hostinfo.rs:156-160`: `--nocolor` picks the plain table.
    if opts.nocolor {
        r.register_native(
            "debug.display_hostinfo",
            |_vm| {
                println!("{}", hostinfo_table(false));
                Ok(())
            },
            eff(0, 0),
            WordKind::Sync,
        );
    } else {
        r.register_native(
            "debug.display_hostinfo",
            |_vm| {
                println!("{}", hostinfo_table(true));
                Ok(())
            },
            eff(0, 0),
            WordKind::Sync,
        );
    }
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm;
    use bund2_interp::Interp;

    fn interp() -> Interp {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i
    }

    fn run(i: &mut Interp, src: &str) -> Result<(), String> {
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)
    }

    #[test]
    fn the_host_table_names_bund2_crates() {
        let t = super::hostinfo_table(false);
        assert!(t.contains("bund2-stdlib version"), "{t}");
        assert!(t.contains("Kernel version"), "{t}");
    }

    /// `debug`'s guards. An error ends a program, so the probe cannot hold
    /// these; the texts are the oracle's, measured 2026-10-05.
    ///
    /// The second message is the reference's unreachable one -- it guards the
    /// depth and then writes a different text for a pull that cannot fail
    /// after the guard. It is reproduced where the reference puts it.
    #[test]
    fn debug_guards_its_snippet() {
        for (src, want) in [
            ("debug", "Stack is too shallow for inline DEBUG"),
            (
                "42 debug",
                "Casting debug snippet returns: This Dynamic type is not string",
            ),
        ] {
            let mut i = interp();
            let e = run(&mut i, src).expect_err(src);
            assert!(e.ends_with(want), "{src}:\n  got {e}\n want ...{want}");
        }
    }

    /// `debug` applies each value it steps over, and stops at the `EXIT` that
    /// ends every compiled stream rather than applying it.
    ///
    /// Without the break the word leaves the `EXIT` on the stack; with it the
    /// stack holds the snippet's own answer and nothing else. Stdin is at
    /// end-of-file under `cargo test` as it is under capture, so the inner
    /// read returns at once.
    #[test]
    fn debug_applies_each_value_and_stops_at_exit() {
        let mut i = interp();
        run(&mut i, "\"1 2 +\" debug").expect("steps the snippet");
        assert_eq!(i.depth(), 1, "one value, not two: the EXIT was not applied");
        assert_eq!(i.pull().and_then(|v| v.as_int()), Some(3));
    }

    /// The two siblings no golden can hold: one prints a figure that moves,
    /// the other reports a line the oracle timestamps.
    ///
    /// `debug.display_distributed_info` requires `--distributed`, which Bund2
    /// has not got, so reporting that is its whole reachable behaviour -- the
    /// table it would otherwise draw needs a bus RFC-0007 governs.
    #[test]
    fn memstat_prints_and_distributed_info_reports_that_it_is_not_distributed() {
        let mut i = interp();
        run(&mut i, "debug.display_memstat").expect("prints a table");
        assert_eq!(i.depth(), 0, "it answers on stdout, not the stack");

        let t = super::memstat_table();
        assert!(t.contains("Current physical memory usage"), "{t}");
        assert!(t.contains("Current virtual memory usage"), "{t}");

        let mut i = interp();
        run(&mut i, "debug.display_distributed_info").expect("reports and continues");
        assert_eq!(i.depth(), 0);
    }
}
