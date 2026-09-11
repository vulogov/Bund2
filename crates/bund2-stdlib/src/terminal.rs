//! The terminal words: `input`, `input*`, `password`, `bund.prompt`,
//! `io.banner`, and `debug.display_hostinfo`.
//!
//! **What a golden can hold.** The capture runs a program with stdin at
//! end-of-file, so `input` and `input*` only ever see Ctrl-D there. That path
//! is the reference's own: it pushes nothing and runs nothing
//! (`reference/Bund/src/stdlib/functions/io/input.rs:46-51,117-124`). The
//! prompt is a fixed string with fixed colour codes, because the reference
//! never turns yansi off. `io.banner` falls back to an 80-column width when
//! stdout is not a terminal, as it is under capture. The host table depends on
//! the machine, so no golden can hold it.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LAMBDA, STRING};
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

fn editor() -> Result<rustyline::DefaultEditor, Error> {
    rustyline::DefaultEditor::new().map_err(|e| Error(format!("INPUT returns: {e}")))
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
    let mut rl = editor()?;
    let prompt = prompt_of(vm.pull());
    match rl.readline(&prompt) {
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
    let mut rl = editor()?;
    let lambda = vm
        .pull()
        .ok_or_else(|| Error("Error getting INPUT* lambda from stack".into()))?;
    let prompt = prompt_of(vm.pull());
    loop {
        match rl.readline(&prompt) {
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
    use yapp::PasswordReader;
    let m = vm
        .pull()
        .ok_or_else(|| Error("PASSWORD: NO DATA #1".into()))?;
    let msg = m.as_str().ok_or_else(|| {
        Error("PASSWORD error casting message: This Dynamic type is not string".into())
    })?;
    let mut reader = yapp::Yapp::new().with_echo_symbol('.');
    let res = reader
        .read_password_with_prompt(&msg)
        .map_err(|e| Error(format!("PASSWORD returns: {e}")))?;
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
    #[test]
    fn the_host_table_names_bund2_crates() {
        let t = super::hostinfo_table(false);
        assert!(t.contains("bund2-stdlib version"), "{t}");
        assert!(t.contains("Kernel version"), "{t}");
    }
}
