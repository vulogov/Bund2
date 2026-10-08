//! The host-touching words: script arguments, the filesystem, the clock, the
//! process title, and `io.graph`.
//!
//! **What `--noio` switches off, and how.** The reference swaps each I/O word
//! for a stub that fails with `bund <GROUP> functions disabled with --noio`,
//! choosing at registration (`reference/Bund/src/stdlib/functions/filesystem/cwd.rs:34-38`
//! and each sibling). [`register`] does the same from [`HostOptions`]. The
//! group named in the message is the reference's, including `fs.rm`'s, which
//! says `FS.CP` because `fs.rm` shares `fs.cp`'s file and stub
//! (`reference/Bund/src/stdlib/functions/filesystem/cp.rs:133-152`).
//! `args`, `sleep.seconds` and `io.graph` have no gate in the reference, and
//! none here.
//!
//! **`bund/system`'s path pair is here for subsystem cohesion, not because it
//! touches the host.** `system.path.split` and `system.path.filename`
//! manipulate a string and read no filesystem, and the reference does not gate
//! them with `--noio`: `init_stdlib` takes the command line and never consults
//! it (`reference/Bund/src/stdlib/functions/system/unixpath.rs`, `init_stdlib`).
//! They live beside `system.setproctitle` and `sleep.seconds` because all of
//! `bund/system` maps to one reference directory.
//!
//! **None of these can print twice the same way on every machine** except
//! `io.graph`, `args` on an empty command line, and the failure paths. The
//! probe `tests/probes/host-words.bund` pins those; the rest are unit-tested.

use std::sync::Mutex;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST, STRING};

use path_absolutize::Absolutize;

use crate::wb::Side;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// How the host words are registered. The reference reads these from its
/// command line (`reference/Bund/src/cmd/mod.rs:139-146`).
#[derive(Debug, Clone, Copy, Default)]
pub struct HostOptions {
    /// `--noio`: register the I/O words as stubs that fail.
    pub noio: bool,
    /// `--nocolor`: `debug.display_hostinfo` draws its table without colour
    /// (`reference/Bund/src/stdlib/functions/debug_fun/debug_display_hostinfo.rs:156-160`).
    pub nocolor: bool,
    /// `--noeval`: `bund.eval` and `use` fail instead of evaluating
    /// (`reference/Bund/src/cmd/mod.rs:142-143`).
    pub noeval: bool,
}

/// `bund.exit` — end the program (D52)
/// (`reference/Bund/src/stdlib/functions/bund/bund_exit.rs:10-31`).
///
/// With an empty stack the code is 0. Otherwise it is the value on top, and a
/// value that is not an INTEGER is logged and taken as 0 (`:22-28`). The code
/// is truncated `as i32` (`:30`). Where the reference calls `process::exit`,
/// this asks the embedder to end the program, and nothing after it runs.
fn bund_exit(vm: &mut dyn Vm) -> Result<(), Error> {
    let code = match vm.pull() {
        None => 0,
        Some(v) => v.as_int().unwrap_or_else(|| {
            // The reference logs this at error level and exits 0 (`:22-28`).
            // A warning, because the program is ending as it asked to.
            vm.report(bund2_api::diag::Diagnostic::warning(
                "Error in casting error code for exit: This Dynamic type is not integer",
            ));
            0
        }),
    };
    vm.request_exit(code as i32);
    Ok(())
}

/// The script's own arguments, the ones after `--`.
///
/// The reference keeps them in a global list that the runner fills before the
/// program starts (`reference/Bund/src/stdlib/functions/bund/bund_args.rs:12-17`,
/// filled at `reference/Bund/src/cmd/bund_script.rs:13-19`). A process runs one
/// script, so a process-wide list is the same shape.
static ARGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Set the script arguments `args` and `args.parse` answer with.
pub fn set_args(args: Vec<String>) {
    match ARGS.lock() {
        Ok(mut a) => *a = args,
        // A poisoned lock only means a panic elsewhere held it; the list is a
        // plain value and still the right thing to overwrite.
        Err(p) => *p.into_inner() = args,
    }
}

fn script_args() -> Result<Vec<String>, Error> {
    ARGS.lock()
        .map(|a| a.clone())
        .map_err(|e| Error(format!("Can not access ARGS: {e}")))
}

/// `args` — push the script arguments as a list of strings
/// (`reference/Bund/src/stdlib/functions/bund/bund_args.rs:20-36`).
fn args_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let a = script_args()?;
    vm.push(BundValue::list(a.into_iter().map(BundValue::str).collect()));
    Ok(())
}

/// `args.parse` — split the script arguments into flags and plain arguments
/// with `argmap` (`bund_args.rs:39-81`).
///
/// Each flag maps to the list of values given for it, and `args` holds the
/// plain arguments. `args` is set **last** (`:78`), so a flag spelled `--args`
/// is overwritten by the plain list.
fn args_parse(vm: &mut dyn Vm) -> Result<(), Error> {
    let a = script_args()?;
    let (plain, flags) = argmap::parse(a.iter());
    let mut out = BundValue::map(Default::default());
    for (k, v) in flags {
        out = out.set(&k, BundValue::list(v.into_iter().map(BundValue::str).collect()));
    }
    out = out.set(
        "args",
        BundValue::list(plain.into_iter().map(BundValue::str).collect()),
    );
    vm.push(out);
    Ok(())
}

/// The reference's depth guard for a word with a stack and a workbench form.
pub(crate) fn guard(vm: &dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    if side.depth(vm) >= 1 {
        return Ok(());
    }
    Err(Error(match side {
        Side::Stack => format!("Stack is too shallow for inline {prefix}"),
        Side::Bench => format!("Workbench is too shallow for inline {prefix}"),
    }))
}

/// `fs.cwd` — push the working directory
/// (`reference/Bund/src/stdlib/functions/filesystem/cwd.rs:10-20`).
fn fs_cwd(vm: &mut dyn Vm) -> Result<(), Error> {
    let cwd = std::env::current_dir().map_err(|e| Error(format!("FS.CWD returned: {e}")))?;
    vm.push(BundValue::str(format!("{}", cwd.display())));
    Ok(())
}

/// Which entries a listing keeps:
/// `reference/Bund/src/stdlib/functions/filesystem/ls.rs:10-15`.
#[derive(Clone, Copy)]
enum Listing {
    Files,
    Directories,
    Both,
}

/// `fs.ls` and its variants — list a directory recursively
/// (`reference/Bund/src/stdlib/functions/filesystem/ls.rs:19-68`).
///
/// The directory comes from the word's side, but **the list always goes to the
/// stack** (`:66`), the `.` forms included. `fs.ls` skips backup files only, so
/// it lists hidden ones; `fs.ls.files` skips both; `fs.ls.dir` skips hidden
/// directories (`:55-59`). A directory that cannot be read is not an error:
/// `walk`'s errors are discarded (`:61`), and whatever was listed is pushed.
/// The order is the walk's, which is the filesystem's.
fn fs_ls(vm: &mut dyn Vm, side: Side, what: Listing, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let d = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #1")))?;
    let dir = d.as_str().ok_or_else(|| {
        Error(format!(
            "Error casting string for target {prefix}: This Dynamic type is not string"
        ))
    })?;
    let mut scan = match what {
        Listing::Files => scan_dir::ScanDir::files(),
        Listing::Directories => scan_dir::ScanDir::dirs(),
        Listing::Both => scan_dir::ScanDir::all(),
    };
    match what {
        Listing::Files => scan.skip_hidden(true).skip_backup(true),
        Listing::Directories => scan.skip_hidden(true),
        Listing::Both => scan.skip_backup(true),
    };
    let mut res: Vec<BundValue> = Vec::new();
    let _ = scan.walk(&dir, |iter| {
        for (entry, _) in iter {
            res.push(BundValue::str(format!("{}", entry.path().display())));
        }
    });
    vm.push(BundValue::list(res));
    Ok(())
}

/// Which of `cp.rs`'s three operations a call is
/// (`reference/Bund/src/stdlib/functions/filesystem/cp.rs`, `FsOperations`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum FsOp {
    Copy,
    Move,
    Remove,
}

/// `fs.cp`, `fs.mv` and `fs.rm` — one base, as the reference has it
/// (`reference/Bund/src/stdlib/functions/filesystem/cp.rs`,
/// `stdlib_bund_file_cp_base`).
///
/// **The source is on top and the target beneath it**, and the target is a
/// *directory* the sources go into: `fs_extra::copy_items` takes a destination
/// directory, so `"d" "a.txt" fs.cp` writes `d/a.txt`, while
/// `"d/b.txt" "a.txt" fs.cp` fails with `No such file or directory`. The
/// options are `CopyOptions::new()`, whose `overwrite` is false, so copying
/// onto a path that already exists is an error and the word is not
/// idempotent: a second `fs.cp` of the same file says `Path "d/a.txt" exists`.
/// A directory source is copied recursively.
///
/// `fs.rm` has no target. The reference substitutes an empty string whose
/// cast cannot fail, so `NO DATA #2` and the target-cast error are
/// unreachable for it, and a path that does not exist is not an error.
///
/// The source is one path or a LIST of them, and the answer is `true`.
fn fs_copy_base(vm: &mut dyn Vm, op: FsOp, prefix: &str) -> Result<(), Error> {
    let need = if op == FsOp::Remove { 1 } else { 2 };
    if vm.depth() < need {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let v = vm.pull().ok_or_else(|| Error(format!("{prefix} NO DATA #1")))?;
    // The target is cast before the source's type is examined, so a bad
    // target wins over a bad source (`cp.rs`, the `target_file_name` match).
    let target = if op == FsOp::Remove {
        String::new()
    } else {
        let t = vm
            .pull()
            .ok_or_else(|| Error(format!("{prefix} NO DATA #2")))?;
        t.as_str().ok_or_else(|| {
            Error(format!(
                "Error casting string for target {prefix}: This Dynamic type is not string"
            ))
        })?
    };
    let not_string =
        || Error(format!("Error casting string for {prefix}: This Dynamic type is not string"));
    let paths: Vec<String> = match v.dt() {
        STRING => vec![v.as_str().ok_or_else(not_string)?],
        LIST => {
            let items = v.as_list().ok_or_else(|| {
                Error(format!(
                    "Error casting list for {prefix}: This Dynamic type is not list"
                ))
            })?;
            let mut out = Vec::with_capacity(items.len());
            for i in items {
                out.push(i.as_str().ok_or_else(not_string)?);
            }
            out
        }
        _ => return Err(Error(format!("Incorrect #1 type for {prefix}"))),
    };
    let options = fs_extra::dir::CopyOptions::new();
    let res = match op {
        FsOp::Copy => fs_extra::copy_items(&paths, &target, &options).map(|_| ()),
        FsOp::Move => fs_extra::move_items(&paths, &target, &options).map(|_| ()),
        FsOp::Remove => fs_extra::remove_items(&paths).map(|_| ()),
    };
    res.map_err(|e| Error(format!("{prefix} returns: {e}")))?;
    vm.push(BundValue::boolean(true));
    Ok(())
}

/// `file.write` and `file.write.` — write a string to a file
/// (`reference/Bund/src/stdlib/functions/filesystem/file_write.rs`,
/// `bund_file_write_base`).
///
/// **The filename comes from the stack in both forms**; only the data moves to
/// the workbench. So `file.write.` guards the stack for the name *and* the
/// workbench for the data, and the filename is the top of the stack:
/// `"text" "name" file.write`. Nothing is pushed — both operands are consumed
/// and the word answers with its effect alone.
///
/// `fs_extra::file::write_all` creates the file or truncates it. Neither
/// operand is converted: an INTEGER is refused by the cast, so
/// `1 2 "x.txt" file.write` fails at `NO CAST #2` having already consumed the
/// name and the data.
///
/// The `#1` messages say `returns NO DATA` and `returns NO CAST` while the
/// `#2` pair say `returns: NO DATA` and `returns: NO CAST` — the colon is in
/// one half and not the other, reproduced.
fn file_write(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    let need = if side == Side::Bench { 1 } else { 2 };
    if vm.depth() < need {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    if side == Side::Bench && vm.workbench_depth() < 1 {
        return Err(Error(format!(
            "Workbench is too shallow for inline {prefix}"
        )));
    }
    let name_v = vm
        .pull()
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))?;
    let name = name_v.as_str().ok_or_else(|| {
        Error(format!(
            "{prefix} returns NO CAST #1: This Dynamic type is not string"
        ))
    })?;
    let data_v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #2")))?;
    let data = data_v.as_str().ok_or_else(|| {
        Error(format!(
            "{prefix} returns: NO CAST #2: This Dynamic type is not string"
        ))
    })?;
    fs_extra::file::write_all(&name, &data)
        .map_err(|e| Error(format!("{prefix} returns: {e}")))?;
    Ok(())
}

/// `filename` and `filename.` — make a path absolute
/// (`reference/Bund/src/stdlib/functions/filesystem/filepath.rs`,
/// `bund_filename_base`).
///
/// `path_absolutize` is lexical: a relative path is joined to the working
/// directory and `.`/`..` are resolved in the string, with no filesystem
/// lookup and no symlink resolution. It therefore answers for a path that does
/// not exist, which is what makes it the companion to `file` — whose
/// `file://` URL needs an absolute path — and to `file.write`.
///
/// **Both forms answer on the stack**, the workbench form included, and
/// `filename.` guards the stack it does not read. That guard is F151: the
/// ordinary way to load the workbench consumes the stack, so `filename.`
/// fails whenever it is used as intended. It is reproduced in the reference's
/// order, stack first.
///
/// Where the reference unwraps `to_str`, this is lossy: a path that is not
/// UTF-8 renders with replacement characters rather than aborting (D37).
fn filename_word(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    if side == Side::Bench && vm.workbench_depth() < 1 {
        return Err(Error(format!(
            "Workbench is too shallow for inline {prefix}"
        )));
    }
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA")))?;
    let name = v
        .as_str()
        .ok_or_else(|| Error(format!("{prefix} returns: This Dynamic type is not string")))?;
    let abs = std::path::Path::new(&name)
        .absolutize()
        .map_err(|e| Error(format!("{prefix} returned: {e}")))?;
    vm.push(BundValue::str(abs.to_string_lossy().into_owned()));
    Ok(())
}

/// `fs.is_file` and `fs_is_file.` — is the path a file
/// (`reference/Bund/src/stdlib/functions/filesystem/filesystem.rs`,
/// `bund_filesystem_base`).
///
/// **Both names read the stack**, and the second is spelled with an
/// underscore — F150. The reference's workbench branch is unreachable because
/// its workbench function passes `FromStack`, so there is one function here
/// registered under two names, which is what the reference registers. A
/// directory answers `false`, as does a path that does not exist.
fn fs_is_file(vm: &mut dyn Vm) -> Result<(), Error> {
    const P: &str = "FS.IS_FILE";
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {P}")));
    }
    let v = vm
        .pull()
        .ok_or_else(|| Error(format!("{P} returns: NO DATA")))?;
    let name = v
        .as_str()
        .ok_or_else(|| Error(format!("{P} returns: This Dynamic type is not string")))?;
    vm.push(BundValue::boolean(std::path::Path::new(&name).is_file()));
    Ok(())
}

/// How the bounded scan of a file ended — see [`io_textfile`].
enum Scan {
    /// The reader answered `None`: every line was read.
    AtEof,
    /// A line could not be read, which in practice is invalid UTF-8.
    Unreadable(std::io::Error),
    /// The bound was spent and the reader still had more to say.
    Never,
}

/// `io.textfile` and `io.textfile.` — a file's lines as a LIST of strings
/// (`reference/Bund/src/stdlib/functions/io/textfile.rs`,
/// `string_io_textfile_base`).
///
/// The operand is a **path**, opened with `File::open` — not the `file://` URL
/// that `file` fetches through, so a relative path works here and does not
/// there. Both forms answer on their own side.
///
/// # Three ways the reference goes wrong, and one mechanism for all three
///
/// The reference calls `EasyReader::build_index` and then replays the index.
/// Measured against the oracle, that has three failure modes:
///
/// - **F159 — the last line twice.** A multi-line file ending in a newline
///   answers its last line again with the terminator attached:
///   `one\ntwo\nthree\n` is `[ one two three "three\n" ]`. Deterministic, and
///   **preserved**.
/// - **F160 — it never returns** when the file *begins* with a line
///   terminator. `build_index` is `while let Ok(Some(_)) = next_line()`, the
///   scan does not advance past an empty first line, and the index grows
///   without bound. D39 forbids reproducing a hang; D97 says what to answer.
/// - **F161 — it aborts** with `index out of bounds` when two or more
///   readable lines precede one that is not valid UTF-8. The scan stops at the
///   bad line, leaving the index one entry short, and the replay indexes past
///   it unchecked. D37 forbids reproducing an abort.
///
/// **The scan `build_index` runs is run here first, under a bound.** It is the
/// same loop from the same state — `next_line` on a fresh reader — so how it
/// ends says, before anything unbounded is called, which case this file is:
///
/// | the bounded scan… | so the reference… | and this… |
/// |---|---|---|
/// | spends the bound | never returns (F160) | reports, per D97 |
/// | stops on an unreadable line, ≥ 2 read | aborts (F161) | reports the line |
/// | stops on an unreadable line, 0 or 1 read | answers those lines | answers them |
/// | reaches end-of-file | answers, with F159 | replays the index |
///
/// Only the last row calls `build_index`, and by then the scan that proves it
/// finite has already finished. The replay is in bounds for the same reason:
/// a scan that ended at end-of-file left its last entry ending *at* the file's
/// size, which is the test the reader makes before it indexes.
///
/// **An earlier version drove `next_line` un-indexed and stopped there**, on
/// the argument that it is "the very scan `build_index` records". It is the
/// same scan and *not* the same answer: un-indexed, the extra read at the end
/// is an empty string where the indexed replay gives the duplicated line.
/// Found by diffing twelve file shapes against the oracle, eight of which
/// disagreed.
///
/// A directory answers an **empty list** — `File::open` succeeds on one and
/// the first read fails. An empty file is the crate's own `Empty file`.
fn io_textfile(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))?;
    let name = v.as_str().ok_or_else(|| {
        Error(format!(
            "{prefix} returned for #1: This Dynamic type is not string"
        ))
    })?;
    let open = || std::fs::File::open(&name).map_err(|e| Error(format!("{prefix} returns: {e}")));
    let reader_of = |f: std::fs::File| {
        easy_reader::EasyReader::new(f).map_err(|e| Error(format!("{prefix} returns error: {e}")))
    };

    let file = open()?;
    // Taken before the reader owns the file: D39's bound is on data already
    // in hand. *n* bytes hold at most *n + 1* lines, and F159 adds one.
    let bound = file.metadata().map(|m| m.len()).unwrap_or(0).saturating_add(2);

    // The scan `build_index` would run, bounded.
    let mut probe = reader_of(file)?;
    let mut seen: Vec<String> = Vec::new();
    let mut reads: u64 = 0;
    let ended = loop {
        if reads >= bound {
            break Scan::Never;
        }
        reads += 1;
        match probe.next_line() {
            Ok(Some(line)) => seen.push(line),
            Ok(None) => break Scan::AtEof,
            Err(e) => break Scan::Unreadable(e),
        }
    };

    let lines: Vec<String> = match ended {
        Scan::Never => {
            return Err(Error(format!(
                "{prefix} returns error: the reader did not reach the end of the file"
            )))
        }
        // F161. With two or more lines read the reference's replay indexes
        // past its short index and the process dies; with fewer it never
        // reaches the indexed path and answers what it read.
        Scan::Unreadable(e) if seen.len() >= 2 => {
            return Err(Error(format!("{prefix} returns error: {e}")))
        }
        Scan::Unreadable(_) => seen,
        Scan::AtEof => {
            let mut reader = reader_of(open()?)?;
            // Finite: the bounded scan above is this loop, and it ended.
            let _ = reader.build_index();
            reader.bof();
            let cap = seen.len().saturating_add(2);
            let mut out: Vec<String> = Vec::new();
            loop {
                if out.len() > cap {
                    return Err(Error::internal(format!(
                        "{prefix}: the indexed replay of a file whose scan ended at \
                         end-of-file after {} lines yielded more than {cap}",
                        seen.len()
                    )));
                }
                match reader.next_line() {
                    Ok(Some(line)) => out.push(line),
                    // The reference breaks on `Ok(None)` and on any error.
                    Ok(None) | Err(_) => break,
                }
            }
            out
        }
    };
    side.push(
        vm,
        BundValue::list(lines.into_iter().map(BundValue::str).collect()),
    );
    Ok(())
}

/// `system.ip` and `system.ipv6` — the host's own address
/// (`reference/Bund/src/stdlib/functions/system/ip.rs`).
///
/// **`SYSTEM.IPv6` spells its version lower-case**, alone among the
/// subsystem's prefixes. Reproduced, because the text is what a program sees.
///
/// Neither is gated by `--noio`, though both interrogate the host's network
/// interfaces: `ip.rs`'s `init_stdlib` takes the command line and never reads
/// it. So does `locale.rs`'s. Only `shell.rs` in this subsystem gates.
///
/// **No golden can hold either answer.** An address names the machine, and
/// `system.ipv6` fails outright on a host with no IPv6 -- so even the shape of
/// the answer is not fixed, which is why the test asserts only that it either
/// pushes a string or reports the prefix.
fn system_ip(vm: &mut dyn Vm, v6: bool) -> Result<(), Error> {
    let (addr, prefix) = if v6 {
        (
            local_ip_address::local_ipv6().map(|a| a.to_string()),
            "SYSTEM.IPv6",
        )
    } else {
        (
            local_ip_address::local_ip().map(|a| a.to_string()),
            "SYSTEM.IP",
        )
    };
    let a = addr.map_err(|e| Error(format!("{prefix} returned: {e}")))?;
    vm.push(BundValue::str(a));
    Ok(())
}

/// `system.locale` — the host's locale
/// (`reference/Bund/src/stdlib/functions/system/locale.rs`).
///
/// A locale that cannot be determined is an **error**, not an empty string.
fn system_locale(vm: &mut dyn Vm) -> Result<(), Error> {
    let l = sys_locale::get_locale()
        .ok_or_else(|| Error("SYSTEM.LOCALE can not be found".into()))?;
    vm.push(BundValue::str(l));
    Ok(())
}

/// `system.shell` and `system.shell.` — run a command through the shell
/// (`reference/Bund/src/stdlib/functions/system/shell.rs`,
/// `string_system_shell_base`).
///
/// `duct_sh::sh_dangerous` runs the string through `/bin/sh -c`, and the word
/// answers its **standard output** on the side it was called from. The name is
/// the crate's: the command is not quoted or escaped, so whatever the program
/// built is what the shell interprets. The reference calls it exactly this way
/// and `--noio` is the gate it provides, which is reproduced.
///
/// **What `read` does to the output, measured on the oracle.** Internal
/// newlines survive, *all* trailing newlines are trimmed -- `printf 'a\n\n\n'`
/// answers `a` -- and leading and trailing *spaces* are kept, so
/// `printf '  pad  '` answers `  pad  ` and this is not a general trim.
///
/// **A non-zero exit is an error**, and the text is duct's own:
/// `SYSTEM.SHELL returns: command ["/bin/sh", "-c", "exit 3"] exited with code
/// 3`. Standard error is not captured and passes through to the terminal, so
/// `no_such_command_xyz` prints the shell's complaint and then reports code
/// 127.
///
/// This is the well-formed workbench shape -- the guard checks the side it
/// reads -- which F151 records `filename.` getting wrong.
fn system_shell(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))?;
    let cmd = v.as_str().ok_or_else(|| {
        Error(format!(
            "{prefix} returned for #1: This Dynamic type is not string"
        ))
    })?;
    let out = duct_sh::sh_dangerous(cmd)
        .read()
        .map_err(|e| Error(format!("{prefix} returns: {e}")))?;
    side.push(vm, BundValue::str(out));
    Ok(())
}

/// Which of `unixpath.rs`'s two algorithms a call is
/// (`reference/Bund/src/stdlib/functions/system/unixpath.rs`,
/// `UnixPathAlgorithm`).
#[derive(Clone, Copy)]
enum PathOp {
    Split,
    Filename,
}

/// `system.path.split` and `system.path.filename`, with their workbench forms
/// (`reference/Bund/src/stdlib/functions/system/unixpath.rs`,
/// `string_system_path_base`).
///
/// **Both forms answer on their own side**, and each guards the side it reads
/// -- the workbench form checks the workbench, which is the shape F151 says
/// `filename.` gets wrong. So these are the well-formed pair and `filename.`
/// is the defective one, in two files that otherwise look alike.
///
/// `split` answers the path's components as a LIST of strings, **with the root
/// as a component of its own**: `/a/b` gives `[ / a b ]` and `a/b` gives
/// `[ a b ]`. Repeated and trailing separators collapse, `.` is dropped except
/// where it is the first component, and `..` is kept. The empty path answers
/// an empty list rather than failing. All measured against the oracle.
///
/// `filename` is the last component, and it **fails where there is none**:
/// `/`, the empty path, `.`, `..`, and anything whose last component is `..`.
/// That message names the path and *not the word*, so the two forms cannot be
/// told apart by their error -- the one place in this pair where the prefix is
/// not used.
///
/// `unix_path` rather than `std::path` is the reference's own choice, at the
/// version its lock resolves. On a Unix host the two agree, measured; the
/// crate is what keeps a component split meaning the same thing on a host
/// whose separator is not `/`, and no decision in the registers says which
/// hosts those are.
///
/// Where the reference unwraps `to_str`, this is lossy (D37). A component of a
/// path that arrived as a Bund STRING is already UTF-8, so the difference is
/// unreachable from the language.
fn system_path(vm: &mut dyn Vm, side: Side, op: PathOp, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))?;
    let name = v.as_str().ok_or_else(|| {
        Error(format!(
            "{prefix} returned for #1: This Dynamic type is not string"
        ))
    })?;
    let path = unix_path::Path::new(name.as_str());
    match op {
        PathOp::Split => {
            let parts: Vec<BundValue> = path
                .iter()
                .map(|p| BundValue::str(p.to_string_lossy().into_owned()))
                .collect();
            side.push(vm, BundValue::list(parts));
        }
        PathOp::Filename => {
            let last = path
                .file_name()
                .ok_or_else(|| Error(format!("Error getting filename for: {name}")))?;
            side.push(vm, BundValue::str(last.to_string_lossy().into_owned()));
        }
    }
    Ok(())
}

/// `time.now` — the present moment as a TIME value
/// (`reference/rust_multistackvm/src/stdlib/time/timestamp.rs`,
/// `stdlib_time_now`).
///
/// Nanoseconds since the Unix epoch, as `Value::now` holds them. A clock set
/// before the epoch is reported rather than read as zero: a wrong time that
/// looks like a time is worse than none.
///
/// No golden can hold the answer; only its type can be asked.
fn time_now(vm: &mut dyn Vm) -> Result<(), Error> {
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error(format!("TIME.NOW returns: {e}")))?
        .as_nanos();
    vm.push(BundValue::time(at));
    Ok(())
}

/// `time.timestamp` — a TIME value from an integer (`stdlib_time_make_timestamp`).
///
/// **The integer is cast `as u128`, so a negative one wraps** rather than
/// failing: `-1 time.timestamp` is the largest `u128`. Reproduced, with the
/// cast written the way the reference writes it.
///
/// Its shallow-stack message is lower-case — `inline time.timestamp` — where
/// its other two say `TIME.TIMESTAMP`, and one of those says `return error`
/// without the `s`. All three as the reference has them.
///
/// **What the answer is good for is very little** — see `Payload::Time`. It
/// can be compared and asked its type; it cannot be printed, converted or
/// added to.
fn time_timestamp(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error(
            "Stack is too shallow for inline time.timestamp".into(),
        ));
    }
    let v = vm
        .pull()
        .ok_or_else(|| Error("TIME.TIMESTAMP returns: NO DATA #1".into()))?;
    let stamp = v.as_int().ok_or_else(|| {
        Error("TIME.TIMESTAMP return error: This Dynamic type is not integer".into())
    })?;
    #[expect(clippy::cast_sign_loss, reason = "the reference's own `stamp as u128`")]
    vm.push(BundValue::time(stamp as u128));
    Ok(())
}

/// `sleep.seconds` — wait whole seconds
/// (`reference/Bund/src/stdlib/functions/system/sleep.rs:11-21`).
///
/// The count must be an INTEGER, and the reference converts it with `as u64`
/// (`:19`), so a negative count becomes one near `u64::MAX`.
///
/// **A count the clock cannot hold is refused** — F103 as corrected. That
/// count does not wait: adding it to the present overflows and the reference
/// aborts, `overflow when adding duration to instant`. Bund2 took the same
/// panic inside this word and reported it as its own internal error, which
/// D37 forbids. The test is the one the sleep would fail.
fn sleep_seconds(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error("SLEEP::SECONDS: NO DATA #1".into()))?;
    let n = v.as_int().ok_or_else(|| {
        Error("SLEEP::SECONDS error casting seconds: This Dynamic type is not integer".into())
    })?;
    let wait = std::time::Duration::new(n as u64, 0);
    if std::time::Instant::now().checked_add(wait).is_none() {
        return Err(Error(format!(
            "SLEEP::SECONDS: the clock cannot wait {n} seconds"
        )));
    }
    spin_sleep::sleep(wait);
    Ok(())
}

/// `system.setproctitle` — set the title the operating system shows for the
/// process (`reference/Bund/src/stdlib/functions/system/proctitle.rs:10-43`).
/// It pushes nothing.
fn setproctitle(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))?;
    let title = v.as_str().ok_or_else(|| {
        Error(format!("{prefix} returned for #1: This Dynamic type is not string"))
    })?;
    proctitle::set_title(title);
    Ok(())
}

/// Decode `%xx` escapes as curl does in a `file://` path. A `%` not followed
/// by two hex digits is kept as it is.
fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        match (b.get(i), b.get(i + 1).copied().and_then(hex), b.get(i + 2).copied().and_then(hex)) {
            (Some(b'%'), Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (Some(c), _, _) => {
                out.push(*c);
                i += 1;
            }
            (None, _, _) => break,
        }
    }
    String::from_utf8(out).ok()
}

/// Fetch a URI's contents as text — the reference's `get_file_from_uri`
/// (`reference/Bund/src/stdlib/helpers/file_helper.rs:32-55`), with the
/// schemes D54 admits.
///
/// The reference hands every string to libcurl. That is why a bare path never
/// loads there: curl reads `lib.bund` as a host name, and `/abs/lib.bund` as a
/// malformed URL, and only `file:///abs/lib.bund` reads a file. Confirmed
/// against the oracle on 2026-09-11. So:
///
/// - **`file://`** takes an absolute path, optionally after the host
///   `localhost`, and decodes `%xx` as curl does. `file://relative/x` names a
///   host and fails, as it does under curl.
/// - **`http://`** is fetched by `ureq` with curl's defaults as the reference
///   leaves them: no redirect is followed, a 404's body is still the answer,
///   the body has no size limit, and the user agent is `ZBUS` (`:43`).
/// - **Anything else fails**, `https://` included until it is decided (D54).
///
/// Any failure is `None`, and the bytes are decoded lossily (`:46-54`).
pub(crate) fn fetch_uri(uri: &str) -> Option<String> {
    let bytes = if let Some(rest) = uri.strip_prefix("file://") {
        let path = rest.strip_prefix("localhost").unwrap_or(rest);
        if !path.starts_with('/') {
            return None;
        }
        std::fs::read(percent_decode(path)?).ok()?
    } else if uri.starts_with("http://") {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .max_redirects(0)
            .max_redirects_will_error(false)
            .http_status_as_error(false)
            .user_agent("ZBUS")
            .build()
            .into();
        let mut resp = agent.get(uri).call().ok()?;
        resp.body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_to_vec()
            .ok()?
    } else {
        return None;
    };
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// `file` and `url` — read a file, or fetch a URL, into a string
/// (`reference/Bund/src/stdlib/functions/filesystem/file.rs:15-78`).
///
/// `file` builds `file://{path}` and fetches it
/// (`reference/Bund/src/stdlib/helpers/file_helper.rs:57-59`). So **a relative
/// path fails**, as `file://relative/x` does under curl. Confirmed against the
/// oracle on 2026-09-11: `"tests/…/scores.csv" file` fails, and the same file
/// named absolutely reads. `url` fetches its operand as given. Any failure is
/// `{prefix} gets no data` (`file.rs:47-48`), and the answer goes back to the
/// word's side (`:42-45`).
fn fetch_word(vm: &mut dyn Vm, side: Side, prefix: &str, as_file: bool) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA")))?;
    let name = v.as_str().ok_or_else(|| {
        Error(format!("{prefix} returns: This Dynamic type is not string"))
    })?;
    let uri = if as_file { format!("file://{name}") } else { name };
    let text = fetch_uri(&uri).ok_or_else(|| Error(format!("{prefix} gets no data")))?;
    side.push(vm, BundValue::str(text));
    Ok(())
}

/// `use` — fetch Bund source and evaluate it in this VM
/// (`reference/Bund/src/stdlib/functions/bund/bund_use.rs:9-49`).
///
/// The operand is a URI for [`fetch_uri`], so a library is named
/// `file:///abs/lib.bund`, or built from `cwd`. The text is evaluated as
/// `bund.eval` evaluates a string (`:33`), so what it registers stays
/// registered, and its top level runs once (D3).
fn use_word(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA")))?;
    let addr = v.as_str().ok_or_else(|| {
        Error(format!("{prefix} returns: This Dynamic type is not string"))
    })?;
    let src = fetch_uri(&addr).ok_or_else(|| Error(format!("{prefix} can not get from {addr}")))?;
    crate::singles::eval_source(vm, &src)
}

/// `--noeval`: the evaluating words become stubs that fail, as the reference
/// registers them (`reference/Bund/src/stdlib/functions/bund/bund_eval.rs:117-121`,
/// `bund_use.rs:74-76`). Applied after every other registration, so the stubs
/// replace the real words, and `!!` follows its target.
pub fn register_noeval_stubs(r: &mut Registry) {
    // **All four together, as the reference stubs them** — `bund.eval-file`
    // reads a file to *run* it, so it is the evaluating flag that disables it
    // and not `--noio` (`reference/Bund/src/stdlib/functions/bund/bund_eval.rs`).
    for name in ["bund.eval", "bund.eval.", "bund.eval-file", "bund.eval-file."] {
        r.register_native(
            name,
            |_vm| Err(Error("bund EVAL functions disabled with --noeval".into())),
            StackEffect::opaque(0),
            WordKind::Sync,
        );
    }
    for name in ["use", "use."] {
        r.register_native(
            name,
            |_vm| Err(Error("bund USE functions disabled with --noeval".into())),
            StackEffect::opaque(0),
            WordKind::Sync,
        );
    }
}

/// `io.graph` — draw a list of floats as a text chart with `rasciigraph`
/// (`reference/Bund/src/stdlib/functions/io/graph.rs:10-60`).
///
/// Every element must already be a FLOAT: it is `cast_float`ed, not converted
/// (`:35`), so `[ 1 2 ]` is refused. The chart always goes to the stack
/// (`:48`), the `.` form included.
///
/// **An empty list, or one of NaNs, panics inside `rasciigraph`** (F104). The
/// reference crashes; here D49's guard turns the panic into a reported
/// internal error. No chart is drawn, since the reference never draws one.
fn io_graph(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #1")))?;
    let items = v.as_list().ok_or_else(|| {
        Error(format!(
            "{prefix} casting data returns: This is not a LIST/PAIR value but {}",
            v.dt()
        ))
    })?;
    let mut series: Vec<f64> = Vec::with_capacity(items.len());
    for i in items {
        match *i.unboxed() {
            BundValue::Float(f, _) => series.push(f),
            _ => {
                return Err(Error(format!(
                    "{prefix} casting data element returns: This Dynamic type is not float: {}",
                    i.dt()
                )))
            }
        }
    }
    vm.push(BundValue::str(rasciigraph::plot(
        series,
        rasciigraph::Config::default(),
    )));
    Ok(())
}

pub fn register(r: &mut Registry, opts: &HostOptions) {
    // Ungated in the reference: `bund_args.rs:99-100`, `sleep.rs:32`,
    // `io/graph.rs:80-81`.
    r.register_native("args", args_word, eff(0, 1), WordKind::Sync);
    r.register_native("args.parse", args_parse, eff(0, 1), WordKind::Sync);
    r.register_native("sleep.seconds", sleep_seconds, eff(1, 0), WordKind::Sync);
    // `reference/Bund/src/stdlib/functions/bund/bund_exit.rs:41`, aliased at
    // `reference/Bund/src/stdlib/functions/create_aliases.rs:31`. Opaque: it
    // takes a value only when there is one.
    r.register_native("bund.exit", bund_exit, StackEffect::opaque(0), WordKind::Sync);
    r.register_alias("exit", "bund.exit");
    r.register_native(
        "io.graph",
        |vm| io_graph(vm, Side::Stack, "IO.GRAPH"),
        eff(1, 1),
        WordKind::Sync,
    );
    r.register_native(
        "io.graph.",
        |vm| io_graph(vm, Side::Bench, "IO.GRAPH."),
        eff(0, 1),
        WordKind::Sync,
    );

    // `vm/time`: registered by the VM crate itself, with no gate at all.
    r.register_native("time.now", time_now, eff(0, 1), WordKind::Sync);
    r.register_native("time.timestamp", time_timestamp, eff(1, 1), WordKind::Sync);
    crate::wb::bench!(r, "time.timestamp", time_timestamp);

    // Ungated, as the reference leaves them: `ip.rs` and `locale.rs` take the
    // command line and never read it.
    r.register_native("system.ip", |vm| system_ip(vm, false), eff(0, 1), WordKind::Sync);
    r.register_native("system.ipv6", |vm| system_ip(vm, true), eff(0, 1), WordKind::Sync);
    r.register_native("system.locale", system_locale, eff(0, 1), WordKind::Sync);

    // Ungated, because the reference does not gate them: pure string work.
    r.register_native(
        "system.path.split",
        |vm| system_path(vm, Side::Stack, PathOp::Split, "SYSTEM.PATH.SPLIT"),
        eff(1, 1),
        WordKind::Sync,
    );
    r.register_native(
        "system.path.split.",
        |vm| system_path(vm, Side::Bench, PathOp::Split, "SYSTEM.PATH.SPLIT."),
        eff(0, 0),
        WordKind::Sync,
    );
    r.register_native(
        "system.path.filename",
        |vm| system_path(vm, Side::Stack, PathOp::Filename, "SYSTEM.PATH.FILENAME"),
        eff(1, 1),
        WordKind::Sync,
    );
    r.register_native(
        "system.path.filename.",
        |vm| system_path(vm, Side::Bench, PathOp::Filename, "SYSTEM.PATH.FILENAME."),
        eff(0, 0),
        WordKind::Sync,
    );

    // The gated words, as real words or as the reference's stubs.
    macro_rules! stub {
        ($name:literal, $group:literal, $e:expr) => {
            r.register_native(
                $name,
                |_vm| {
                    Err(Error(
                        concat!("bund ", $group, " functions disabled with --noio").into(),
                    ))
                },
                $e,
                WordKind::Sync,
            );
        };
    }
    if opts.noio {
        stub!("fs.cwd", "FS.CWD", eff(0, 1));
        stub!("fs.ls", "FS.LS", eff(1, 1));
        stub!("fs.ls.", "FS.LS", eff(0, 1));
        stub!("fs.ls.dir", "FS.LS", eff(1, 1));
        stub!("fs.ls.dir.", "FS.LS", eff(0, 1));
        stub!("fs.ls.files", "FS.LS", eff(1, 1));
        stub!("fs.ls.files.", "FS.LS", eff(0, 1));
        stub!("fs.rm", "FS.CP", eff(1, 1));
        stub!("fs.cp", "FS.CP", eff(2, 1));
        stub!("fs.mv", "FS.CP", eff(2, 1));
        stub!("file.write", "FILE.WRITE", eff(2, 0));
        stub!("file.write.", "FILE.WRITE", eff(1, 0));
        stub!("filename", "FILEPATH", eff(1, 1));
        stub!("filename.", "FILEPATH", eff(0, 1));
        stub!("fs.is_file", "FILESYSTEM", eff(1, 1));
        // F150: this spelling exists in the `--noio` build alone, and
        // `fs_is_file.` exists only in the other one.
        stub!("fs.is_file.", "FILESYSTEM", eff(0, 1));
        // F152: `stdin` and `stdin.` are registered here and nowhere else.
        stub!("stdin", "FILE", eff(1, 1));
        stub!("stdin.", "FILE", eff(0, 0));
        stub!("io.textfile", "IO.TEXTFILE", eff(1, 1));
        stub!("io.textfile.", "IO.TEXTFILE", eff(0, 0));
        stub!("system.shell", "SYSTEM.SHELL", eff(1, 1));
        stub!("system.shell.", "SYSTEM.SHELL", eff(0, 0));
        stub!("system.setproctitle", "SYSTEM.SETPROCTITLE", eff(1, 0));
        stub!("system.setproctitle.", "SYSTEM.SETPROCTITLE", eff(0, 0));
        stub!("file", "FILE", eff(1, 1));
        stub!("file.", "FILE", eff(0, 0));
        stub!("url", "FILE", eff(1, 1));
        stub!("url.", "FILE", eff(0, 0));
    } else {
        r.register_native("fs.cwd", fs_cwd, eff(0, 1), WordKind::Sync);
        r.register_native(
            "fs.ls",
            |vm| fs_ls(vm, Side::Stack, Listing::Both, "FS.LS"),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "fs.ls.",
            |vm| fs_ls(vm, Side::Bench, Listing::Both, "FS.LS."),
            eff(0, 1),
            WordKind::Sync,
        );
        r.register_native(
            "fs.ls.dir",
            |vm| fs_ls(vm, Side::Stack, Listing::Directories, "FS.LS.DIR"),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "fs.ls.dir.",
            |vm| fs_ls(vm, Side::Bench, Listing::Directories, "FS.LS.DIR."),
            eff(0, 1),
            WordKind::Sync,
        );
        r.register_native(
            "fs.ls.files",
            |vm| fs_ls(vm, Side::Stack, Listing::Files, "FS.LS.FILES"),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "fs.ls.files.",
            |vm| fs_ls(vm, Side::Bench, Listing::Files, "FS.LS.FILES."),
            eff(0, 1),
            WordKind::Sync,
        );
        r.register_native(
            "fs.rm",
            |vm| fs_copy_base(vm, FsOp::Remove, "FS.RM"),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "fs.cp",
            |vm| fs_copy_base(vm, FsOp::Copy, "FS.CP"),
            eff(2, 1),
            WordKind::Sync,
        );
        r.register_native(
            "fs.mv",
            |vm| fs_copy_base(vm, FsOp::Move, "FS.MV"),
            eff(2, 1),
            WordKind::Sync,
        );
        r.register_native(
            "file.write",
            |vm| file_write(vm, Side::Stack, "FILE.WRITE"),
            eff(2, 0),
            WordKind::Sync,
        );
        r.register_native(
            "file.write.",
            |vm| file_write(vm, Side::Bench, "FILE.WRITE."),
            eff(1, 0),
            WordKind::Sync,
        );
        r.register_native(
            "filename",
            |vm| filename_word(vm, Side::Stack, "FILENAME"),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "filename.",
            |vm| filename_word(vm, Side::Bench, "FILENAME."),
            eff(0, 1),
            WordKind::Sync,
        );
        r.register_native("fs.is_file", fs_is_file, eff(1, 1), WordKind::Sync);
        // F150: the underscore is the reference's registration, and the only
        // spelling of the workbench form that a default build binds.
        r.register_native("fs_is_file.", fs_is_file, eff(1, 1), WordKind::Sync);
        r.register_native(
            "io.textfile",
            |vm| io_textfile(vm, Side::Stack, "IO.TEXTFILE"),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "io.textfile.",
            |vm| io_textfile(vm, Side::Bench, "IO.TEXTFILE."),
            eff(0, 0),
            WordKind::Sync,
        );
        r.register_native(
            "system.shell",
            |vm| system_shell(vm, Side::Stack, "SYSTEM.SHELL"),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "system.shell.",
            |vm| system_shell(vm, Side::Bench, "SYSTEM.SHELL."),
            eff(0, 0),
            WordKind::Sync,
        );
        r.register_native(
            "system.setproctitle",
            |vm| setproctitle(vm, Side::Stack, "SYSTEM.SETPROCTITLE"),
            eff(1, 0),
            WordKind::Sync,
        );
        r.register_native(
            "system.setproctitle.",
            |vm| setproctitle(vm, Side::Bench, "SYSTEM.SETPROCTITLE."),
            eff(0, 0),
            WordKind::Sync,
        );
        r.register_native(
            "file",
            |vm| fetch_word(vm, Side::Stack, "FILE", true),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "file.",
            |vm| fetch_word(vm, Side::Bench, "FILE.", true),
            eff(0, 0),
            WordKind::Sync,
        );
        // `file.rs:102-103`.
        r.register_native(
            "url",
            |vm| fetch_word(vm, Side::Stack, "URL", false),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "url.",
            |vm| fetch_word(vm, Side::Bench, "URL.", false),
            eff(0, 0),
            WordKind::Sync,
        );
    }

    // `reference/Bund/src/stdlib/functions/bund/bund_use.rs:78-79`. Opaque:
    // the source can do anything. `--noeval` replaces both, afterwards.
    r.register_native(
        "use",
        |vm| use_word(vm, Side::Stack, "USE"),
        StackEffect::opaque(1),
        WordKind::Sync,
    );
    r.register_native(
        "use.",
        |vm| use_word(vm, Side::Bench, "USE."),
        StackEffect::opaque(0),
        WordKind::Sync,
    );

    // `reference/Bund/src/stdlib/functions/create_aliases.rs:16-19,40`.
    r.register_alias("rm", "fs.rm");
    r.register_alias("cp", "fs.cp");
    r.register_alias("mv", "fs.mv");
    // `create_aliases.rs` again. **Missed when `system.shell` landed** -- the
    // words were implemented and their two aliases were not, though `cp` and
    // `mv` had been added beside `fs.cp` and `fs.mv` two commits earlier.
    // Found by diffing the alias tables, not by a test: nothing in the suite
    // asks whether an alias the reference binds is bound here.
    r.register_alias("sh", "system.shell");
    r.register_alias("sh.", "system.shell.");
    r.register_alias("ls", "fs.ls");
    r.register_alias("ls.", "fs.ls.");
    r.register_alias("cwd", "fs.cwd");
    r.register_alias("sleep", "sleep.seconds");
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_interp::Interp;

    fn interp(opts: HostOptions) -> Interp {
        let mut i = Interp::new();
        crate::register_all_with(&mut i.registry, &opts);
        i
    }

    fn run(i: &mut Interp, src: &str) -> Result<(), String> {
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)
    }

    /// `BundValue` has no boolean accessor -- D1 keeps truthiness a word's
    /// business rather than the value's -- so a test that wants the BOOL it
    /// was handed reads the variant.
    fn truth(v: &BundValue) -> Option<bool> {
        match v {
            BundValue::Bool(b, _) => Some(*b),
            _ => None,
        }
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("bund2-host-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("scratch dir");
        d
    }

    #[test]
    fn noio_replaces_the_io_words_with_the_reference_stubs() {
        let mut i = interp(HostOptions {
            noio: true,
            ..HostOptions::default()
        });
        let e = run(&mut i, "fs.cwd").expect_err("stubbed");
        assert!(e.contains("bund FS.CWD functions disabled with --noio"), "{e}");
        let e = run(&mut i, "\"x\" rm").expect_err("stubbed");
        assert!(e.contains("bund FS.CP functions disabled with --noio"), "{e}");
        // Ungated words still run.
        run(&mut i, "0 sleep").expect("sleep is not gated");
    }

    #[test]
    fn file_reads_and_fs_ls_lists_and_fs_rm_removes() {
        let d = scratch("files");
        let f = d.join("a.txt");
        std::fs::write(&f, "hello").expect("write");
        let mut i = interp(HostOptions::default());

        run(&mut i, &format!("\"{}\" file", f.display())).expect("file");
        assert_eq!(i.peek().and_then(|v| v.as_str()).as_deref(), Some("hello"));

        run(&mut i, &format!("\"{}\" ls", d.display())).expect("ls");
        let listed = i.peek().and_then(|v| v.as_list().map(<[BundValue]>::to_vec));
        assert_eq!(listed.map(|l| l.len()), Some(1));

        run(&mut i, &format!("\"{}\" rm", f.display())).expect("rm");
        assert!(!f.exists());
        let e = run(&mut i, &format!("\"{}\" file", f.display())).expect_err("gone");
        assert!(e.contains("FILE gets no data"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// The messages a golden cannot reach, because an error ends a program and
    /// a capture records one. Every text here was measured against the oracle
    /// on 2026-10-04.
    ///
    /// **The colon moves between the halves.** `#1` says `returns NO DATA` and
    /// `returns NO CAST`; `#2` says `returns: NO DATA` and `returns: NO CAST`.
    #[test]
    fn file_write_writes_and_reports_the_asymmetric_messages() {
        let d = scratch("write");
        let f = d.join("w.txt");
        let mut i = interp(HostOptions::default());
        run(&mut i, &format!("\"data\" \"{}\" file.write", f.display())).expect("write");
        assert_eq!(std::fs::read_to_string(&f).ok().as_deref(), Some("data"));
        assert_eq!(i.depth(), 0, "both operands consumed and nothing pushed");

        // Truncation, not append: the shorter second write wins outright.
        run(&mut i, &format!("\"ab\" \"{}\" file.write", f.display())).expect("rewrite");
        assert_eq!(std::fs::read_to_string(&f).ok().as_deref(), Some("ab"));

        // The data comes from the workbench and the name from the stack, so
        // `.` -- which empties the stack -- is not enough on its own.
        run(&mut i, &format!("\"wb\" . \"{}\" file.write.", f.display())).expect("wb write");
        assert_eq!(std::fs::read_to_string(&f).ok().as_deref(), Some("wb"));

        for (src, want) in [
            (
                "\"only\" file.write".to_string(),
                "Stack is too shallow for inline FILE.WRITE".to_string(),
            ),
            (
                "\"x\" . file.write.".to_string(),
                "Stack is too shallow for inline FILE.WRITE.".to_string(),
            ),
            (
                "\"name\" file.write.".to_string(),
                "Workbench is too shallow for inline FILE.WRITE.".to_string(),
            ),
            (
                "\"d\" 42 file.write".to_string(),
                "FILE.WRITE returns NO CAST #1: This Dynamic type is not string".to_string(),
            ),
            (
                "1 2 \"x.txt\" file.write".to_string(),
                "FILE.WRITE returns: NO CAST #2: This Dynamic type is not string".to_string(),
            ),
            (
                format!("\"d\" \"{}\" file.write", d.join("no/such/dir/x").display()),
                "FILE.WRITE returns: No such file or directory (os error 2)".to_string(),
            ),
        ] {
            let mut i = interp(HostOptions::default());
            let e = run(&mut i, &src).expect_err(&src);
            // The evaluator wraps a word's error with the value it was
            // evaluating, so the word's own text is the tail.
            assert!(e.ends_with(&want), "{src}:\n  got {e}\n want ...{want}");
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    /// `filename` is lexical: it joins a relative path to the working
    /// directory and resolves `.` and `..` in the string, with no filesystem
    /// lookup, so it answers for a path that does not exist.
    ///
    /// **F151 is asserted, not worked around**: `filename.` fails on an empty
    /// stack however loaded the workbench is, and the fourth case below is the
    /// only way to call it.
    #[test]
    fn filename_absolutizes_lexically_and_filename_dot_guards_the_wrong_side() {
        let mut i = interp(HostOptions::default());
        run(&mut i, "\"no/such/../file.txt\" filename").expect("absolutize");
        let answered = i.peek().and_then(|v| v.as_str()).expect("a path");
        let want = std::env::current_dir()
            .expect("cwd")
            .join("no/file.txt");
        assert_eq!(answered, format!("{}", want.display()));
        assert!(!std::path::Path::new(&answered).exists(), "no lookup happened");

        for (src, want) in [
            ("filename", "Stack is too shallow for inline FILENAME"),
            ("42 filename", "FILENAME returns: This Dynamic type is not string"),
            // F151: `.` moved the operand and emptied the stack, which is the
            // ordinary way to use a workbench form and the one that fails.
            ("\"x\" . filename.", "Stack is too shallow for inline FILENAME."),
            ("0 filename.", "Workbench is too shallow for inline FILENAME."),
        ] {
            let mut i = interp(HostOptions::default());
            let e = run(&mut i, src).expect_err(src);
            assert!(e.ends_with(want), "{src}:\n  got {e}\n want ...{want}");
        }

        // Reachable only with an unrelated value left on the stack, and the
        // answer lands on the stack rather than the workbench.
        let mut i = interp(HostOptions::default());
        run(&mut i, "\"rel.txt\" . 0 filename.").expect("the F151 shape");
        assert_eq!(i.workbench_depth(), 0, "the operand was consumed");
        assert_eq!(i.depth(), 2, "the 0 and the path, both on the stack");
    }

    /// `fs.cp` and `fs.mv` copy *into a directory*, do not overwrite, and
    /// answer `true`. F150's second name is checked here too: `fs_is_file.`
    /// reads the stack, because the reference's workbench function passes
    /// `FromStack`.
    #[test]
    fn fs_cp_and_mv_move_into_a_directory_and_fs_is_file_has_two_stack_names() {
        let d = scratch("cp");
        let from = d.join("a.txt");
        let into = d.join("into");
        std::fs::write(&from, "x").expect("seed");
        std::fs::create_dir_all(&into).expect("target dir");
        let mut i = interp(HostOptions::default());

        run(&mut i, &format!("\"{}\" \"{}\" fs.cp", into.display(), from.display()))
            .expect("copy");
        assert!(into.join("a.txt").is_file() && from.is_file(), "copied, not moved");
        assert_eq!(i.pull().as_ref().and_then(truth), Some(true));

        // `overwrite` is false, so the same copy twice is an error.
        let e = run(&mut i, &format!("\"{}\" \"{}\" fs.cp", into.display(), from.display()))
            .expect_err("exists");
        assert!(
            e.contains("FS.CP returns: Path \"") && e.ends_with("\" exists"),
            "{e}"
        );

        let moved = d.join("b.txt");
        std::fs::write(&moved, "y").expect("seed");
        run(&mut i, &format!("\"{}\" \"{}\" fs.mv", into.display(), moved.display()))
            .expect("move");
        assert!(into.join("b.txt").is_file() && !moved.exists(), "the source is gone");

        // Both names read the stack; a directory and a missing path are false.
        for (src, want) in [
            (format!("\"{}\" fs.is_file", from.display()), true),
            (format!("\"{}\" fs_is_file.", from.display()), true),
            (format!("\"{}\" fs.is_file", d.display()), false),
            (format!("\"{}\" fs.is_file", d.join("gone").display()), false),
        ] {
            let mut i = interp(HostOptions::default());
            run(&mut i, &src).expect(&src);
            assert_eq!(i.pull().as_ref().and_then(truth), Some(want), "{src}");
        }

        for (src, want) in [
            (
                "\"only\" fs.cp".to_string(),
                "Stack is too shallow for inline FS.CP".to_string(),
            ),
            (
                "\"only\" fs.mv".to_string(),
                "Stack is too shallow for inline FS.MV".to_string(),
            ),
            (
                "42 \"src\" fs.cp".to_string(),
                "Error casting string for target FS.CP: This Dynamic type is not string"
                    .to_string(),
            ),
            (
                "\"dir\" 42 fs.cp".to_string(),
                "Incorrect #1 type for FS.CP".to_string(),
            ),
            (
                "\"dir\" [ 42 ] fs.cp".to_string(),
                "Error casting string for FS.CP: This Dynamic type is not string".to_string(),
            ),
            (
                "fs.is_file".to_string(),
                "Stack is too shallow for inline FS.IS_FILE".to_string(),
            ),
            (
                "42 fs.is_file".to_string(),
                "FS.IS_FILE returns: This Dynamic type is not string".to_string(),
            ),
        ] {
            let mut i = interp(HostOptions::default());
            let e = run(&mut i, &src).expect_err(&src);
            // The evaluator wraps a word's error with the value it was
            // evaluating, so the word's own text is the tail.
            assert!(e.ends_with(&want), "{src}:\n  got {e}\n want ...{want}");
        }

        // The source must exist, and the message names it.
        let mut i = interp(HostOptions::default());
        let e = run(
            &mut i,
            &format!("\"{}\" \"{}\" fs.cp", into.display(), d.join("gone").display()),
        )
        .expect_err("missing source");
        assert!(
            e.ends_with("does not exist or you don't have access!")
                && e.contains("FS.CP returns: Path \""),
            "{e}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// `system.path.*`'s failures and its guard sides.
    ///
    /// **`filename` fails where the path has no last component** -- five
    /// shapes, and the message names the path rather than the word, so the
    /// stack and workbench forms are indistinguishable by it. An error ends a
    /// program, so these cannot live in the probe.
    ///
    /// **The guard is on the side the word reads**, which is what F151 says
    /// `filename.` gets wrong in the sibling file: `"x" . system.path.split.`
    /// succeeds with an empty stack, where `"x" . filename.` does not.
    #[test]
    fn system_path_guards_the_side_it_reads_and_names_the_path_when_there_is_no_filename() {
        // The well-formed shape: the stack is empty and the word does not care.
        let mut i = interp(HostOptions::default());
        run(&mut i, "\"/a/b\" . system.path.split.").expect("reads the workbench alone");
        assert_eq!(i.depth(), 0, "nothing reached the stack");
        assert_eq!(i.workbench_depth(), 1, "the list is on the workbench");
        run(&mut i, "\"/q/r.txt\" . system.path.filename.").expect("same for filename.");

        for (src, want) in [
            (
                "system.path.split",
                "Stack is too shallow for inline SYSTEM.PATH.SPLIT",
            ),
            (
                "system.path.split.",
                "Workbench is too shallow for inline SYSTEM.PATH.SPLIT.",
            ),
            (
                "system.path.filename",
                "Stack is too shallow for inline SYSTEM.PATH.FILENAME",
            ),
            (
                "system.path.filename.",
                "Workbench is too shallow for inline SYSTEM.PATH.FILENAME.",
            ),
            (
                "42 system.path.split",
                "SYSTEM.PATH.SPLIT returned for #1: This Dynamic type is not string",
            ),
            (
                "42 system.path.filename",
                "SYSTEM.PATH.FILENAME returned for #1: This Dynamic type is not string",
            ),
            // The five paths with no last component. The word is not named.
            ("\"/\" system.path.filename", "Error getting filename for: /"),
            ("\"\" system.path.filename", "Error getting filename for: "),
            ("\".\" system.path.filename", "Error getting filename for: ."),
            ("\"..\" system.path.filename", "Error getting filename for: .."),
            (
                "\"a/..\" system.path.filename",
                "Error getting filename for: a/..",
            ),
            // ...and the workbench form says exactly the same thing.
            (
                "\"/\" . system.path.filename.",
                "Error getting filename for: /",
            ),
        ] {
            let mut i = interp(HostOptions::default());
            let e = run(&mut i, src).expect_err(src);
            assert!(e.ends_with(want), "{src}:\n  got {e}\n want ...{want}");
        }
    }

    /// The reference does not gate the path pair with `--noio`: its
    /// `init_stdlib` takes the command line and never reads it. So these four
    /// are the only `bund/system` words that still work under the flag.
    #[test]
    fn the_path_pair_is_not_gated_by_noio() {
        let mut i = interp(HostOptions {
            noio: true,
            ..HostOptions::default()
        });
        run(&mut i, "\"/a/b.txt\" system.path.filename").expect("ungated");
        assert_eq!(i.peek().and_then(|v| v.as_str()).as_deref(), Some("b.txt"));
        run(&mut i, "\"/a/b\" system.path.split").expect("ungated");
        // The gated sibling in the same subsystem still refuses.
        let e = run(&mut i, "\"t\" system.setproctitle").expect_err("gated");
        assert!(
            e.contains("bund SYSTEM.SETPROCTITLE functions disabled with --noio"),
            "{e}"
        );
    }

    /// `io.textfile` across the reference's three failure modes — F159, F160
    /// and F161 — which is the whole reason the word is not a ten-line loop.
    ///
    /// **F159 is preserved**: a multi-line file ending in a newline answers its
    /// last line twice, the copy keeping its terminator.
    ///
    /// **F160 and F161 are corrected**, and neither has an oracle answer to
    /// compare against — the reference never returns on the first and aborts
    /// on the second. So these assertions are the only thing that says Bund2
    /// returns at all, and they are why the test exists: a regression here
    /// would be a hang or an abort, which no golden can record.
    #[test]
    fn io_textfile_keeps_the_duplicate_and_reports_where_the_reference_cannot_answer() {
        let d = scratch("textfile");
        let write = |name: &str, bytes: &[u8]| {
            let p = d.join(name);
            std::fs::write(&p, bytes).expect("seed");
            p
        };
        let lines = |p: &std::path::Path| -> Vec<String> {
            let mut i = interp(HostOptions::default());
            run(&mut i, &format!("\"{}\" io.textfile", p.display())).expect("reads");
            i.pull()
                .expect("a list")
                .as_list()
                .expect("a LIST")
                .iter()
                .map(|v| v.as_str().unwrap_or_default())
                .collect()
        };

        // No final newline: the lines, and nothing else.
        assert_eq!(lines(&write("a", b"one\ntwo\nthree")), ["one", "two", "three"]);
        // F159: one byte more, one element more -- the last line with its
        // terminator.
        assert_eq!(
            lines(&write("b", b"one\ntwo\nthree\n")),
            ["one", "two", "three", "three\n"]
        );
        // CRLF: stripped from the lines, kept on the duplicate.
        assert_eq!(
            lines(&write("c", b"one\r\ntwo\r\n")),
            ["one", "two", "two\r\n"]
        );
        // A single line starts at offset zero, never takes the indexed path,
        // and so its extra element is empty rather than a duplicate.
        assert_eq!(lines(&write("e", b"solo\n")), ["solo", ""]);
        // Blank lines in the middle and at the end are data.
        assert_eq!(lines(&write("f", b"x\n\n\ny")), ["x", "", "", "y"]);
        // A directory opens and reads nothing.
        assert!(lines(&d).is_empty());

        let failure = |p: &std::path::Path| -> String {
            let mut i = interp(HostOptions::default());
            run(&mut i, &format!("\"{}\" io.textfile", p.display())).expect_err("fails")
        };

        // F160: the reference never returns on a leading line terminator --
        // one is enough, and what follows is irrelevant.
        for (name, bytes) in [
            ("h1", &b"\n"[..]),
            ("h2", &b"\n\n"[..]),
            ("h3", &b"\nlead\n"[..]),
            ("h4", &b"\r\nx\n"[..]),
        ] {
            let e = failure(&write(name, bytes));
            assert!(
                e.ends_with("IO.TEXTFILE returns error: the reader did not reach the end of the file"),
                "{name}: {e}"
            );
        }
        // ...and a leading *space* is not a leading terminator.
        assert_eq!(lines(&write("h5", b" \nx")), [" ", "x"]);

        // F161: two readable lines and then invalid UTF-8 aborts the
        // reference. Here it reports the line, in the crate's own words.
        let e = failure(&write("u2", b"a\nb\n\xff\n"));
        assert!(
            e.contains("IO.TEXTFILE returns error: The line starting at byte: 4")
                && e.contains("is not valid UTF-8"),
            "{e}"
        );
        // With one readable line the reference answers it and drops the rest,
        // silently. Preserved, truncation included.
        assert_eq!(lines(&write("u1", b"a\n\xff\n")), ["a"]);
        // ...and with none, an empty list.
        assert!(lines(&write("u0", b"\xff\xfe\n")).is_empty());

        // An empty file is the crate's own error, and a missing one the OS's.
        let e = failure(&write("z", b""));
        assert!(e.ends_with("IO.TEXTFILE returns error: Empty file"), "{e}");
        let e = failure(&d.join("no-such-file"));
        assert!(
            e.ends_with("IO.TEXTFILE returns: No such file or directory (os error 2)"),
            "{e}"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    /// `io.textfile`'s guards and its `--noio` gate.
    #[test]
    fn io_textfile_guards_the_side_it_reads_and_is_gated() {
        for (src, want) in [
            ("io.textfile", "Stack is too shallow for inline IO.TEXTFILE"),
            (
                "io.textfile.",
                "Workbench is too shallow for inline IO.TEXTFILE.",
            ),
            (
                "42 io.textfile",
                "IO.TEXTFILE returned for #1: This Dynamic type is not string",
            ),
        ] {
            let mut i = interp(HostOptions::default());
            let e = run(&mut i, src).expect_err(src);
            assert!(e.ends_with(want), "{src}:\n  got {e}\n want ...{want}");
        }
        let mut n = interp(HostOptions {
            noio: true,
            ..HostOptions::default()
        });
        for src in ["\"x\" io.textfile", "io.textfile."] {
            let e = run(&mut n, src).expect_err(src);
            assert!(
                e.ends_with("bund IO.TEXTFILE functions disabled with --noio"),
                "{src}: {e}"
            );
        }
    }

    /// `system.shell`'s failures and the `--noio` gate -- the one word in this
    /// subsystem the reference does gate.
    ///
    /// **A non-zero exit reports duct's own text**, naming the argv it ran.
    /// That string is why `duct_sh` is a dependency rather than a
    /// `std::process::Command` by hand: nothing else produces it.
    #[test]
    fn system_shell_runs_a_command_and_reports_a_non_zero_exit() {
        let mut i = interp(HostOptions::default());
        run(&mut i, "\"echo hi\" system.shell").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_str()).as_deref(), Some("hi"));

        for (src, want) in [
            (
                "system.shell",
                "Stack is too shallow for inline SYSTEM.SHELL",
            ),
            (
                "system.shell.",
                "Workbench is too shallow for inline SYSTEM.SHELL.",
            ),
            (
                "42 system.shell",
                "SYSTEM.SHELL returned for #1: This Dynamic type is not string",
            ),
            (
                "\"exit 3\" system.shell",
                "SYSTEM.SHELL returns: command [\"/bin/sh\", \"-c\", \"exit 3\"] exited with code 3",
            ),
        ] {
            let mut i = interp(HostOptions::default());
            let e = run(&mut i, src).expect_err(src);
            assert!(e.ends_with(want), "{src}:\n  got {e}\n want ...{want}");
        }

        // The gate, and its siblings' lack of one.
        let mut n = interp(HostOptions {
            noio: true,
            ..HostOptions::default()
        });
        for name in ["\"echo hi\" system.shell", "system.shell."] {
            let e = run(&mut n, name).expect_err(name);
            assert!(
                e.ends_with("bund SYSTEM.SHELL functions disabled with --noio"),
                "{name}: {e}"
            );
        }
    }

    /// The three that name the machine. None is gated by `--noio` -- `ip.rs`
    /// and `locale.rs` take the command line and never read it -- and no
    /// golden can hold any of their answers.
    ///
    /// **`system.ipv6` is allowed to fail**, and that is the point of the
    /// assertion's shape: a host without IPv6 reports rather than pushing, so
    /// the test asserts the *disjunction* -- a string, or an error naming the
    /// word's prefix -- rather than an address. Pinning a success here would
    /// be a test that passes on this machine and on no other.
    #[test]
    fn the_host_facts_either_answer_a_string_or_report_their_own_prefix() {
        for (src, prefix) in [
            ("system.ip", "SYSTEM.IP returned: "),
            ("system.ipv6", "SYSTEM.IPv6 returned: "),
            ("system.locale", "SYSTEM.LOCALE can not be found"),
        ] {
            let mut i = interp(HostOptions::default());
            match run(&mut i, src) {
                Ok(()) => {
                    let s = i.peek().and_then(|v| v.as_str()).expect("a string");
                    assert!(!s.is_empty(), "{src} answered an empty string");
                }
                Err(e) => assert!(e.contains(prefix), "{src} failed with the wrong text: {e}"),
            }
            // Ungated: the same call under --noio must behave the same way.
            let mut n = interp(HostOptions {
                noio: true,
                ..HostOptions::default()
            });
            let e = run(&mut n, src).err().unwrap_or_default();
            assert!(
                !e.contains("disabled with --noio"),
                "{src} is not gated in the reference: {e}"
            );
        }
    }

    /// **D119: `--noio` reaches `csv` and `sqlite`, where the reference's does
    /// not.** Both open a file the program names. The last program is the
    /// reason the handlers are stubbed and not only the words: `conditional`
    /// makes a CONDITIONAL of any type, so the read is reachable without
    /// calling `csv` at all.
    #[test]
    fn noio_reaches_the_two_words_that_read_a_data_file() {
        let mut n = interp(HostOptions {
            noio: true,
            ..HostOptions::default()
        });
        for (src, group) in [
            ("\"t.csv\" csv", "CSV"),
            ("\"t.csv\" csv.", "CSV"),
            ("\"t.db\" sqlite", "SQLITE"),
            (
                "conditional :type \"csv\" set :name \"t.csv\" set :lambda { drop } set !",
                "CSV",
            ),
            (
                "conditional :type \"sqlite\" set :name \"t.db\" set :lambda { drop } set !",
                "SQLITE",
            ),
        ] {
            let e = run(&mut n, src).expect_err(src);
            let want = format!("bund {group} functions disabled with --noio");
            assert!(e.ends_with(&want), "`{src}` gave: {e}");
        }
        // Without the flag neither is a stub: a missing file is what is wrong.
        let mut d = interp(HostOptions::default());
        for src in ["\"no-such-file.csv\" csv", "\"no-such-file.db\" sqlite"] {
            let e = run(&mut d, src).expect_err(src);
            assert!(!e.contains("disabled with --noio"), "`{src}` gave: {e}");
        }
    }

    /// F150 and F152: three names the reference registers in the `--noio`
    /// build alone. The default build binds none of them, and `--noio` binds
    /// each as the stub with the reference's own group name — so `fs.is_file.`
    /// is a word only where it refuses to run, and `stdin` has no
    /// implementation anywhere.
    #[test]
    fn three_names_exist_in_the_noio_build_alone() {
        let mut d = interp(HostOptions::default());
        for name in ["fs.is_file.", "stdin", "stdin."] {
            let e = run(&mut d, name).expect_err(name);
            assert!(
                !e.contains("disabled with --noio"),
                "{name} must not be bound in the default build: {e}"
            );
        }
        let mut n = interp(HostOptions {
            noio: true,
            ..HostOptions::default()
        });
        for (name, group) in [
            ("fs.is_file.", "FILESYSTEM"),
            ("stdin", "FILE"),
            ("stdin.", "FILE"),
            ("fs.is_file", "FILESYSTEM"),
            ("file.write", "FILE.WRITE"),
            ("filename", "FILEPATH"),
            ("fs.cp", "FS.CP"),
            ("fs.mv", "FS.CP"),
        ] {
            let e = run(&mut n, name).expect_err(name);
            let want = format!("bund {group} functions disabled with --noio");
            assert!(e.ends_with(&want), "{name}:\n  got {e}\n want ...{want}");
        }
        // And `fs_is_file.` is the other side of F150: bound here, not there.
        let e = run(&mut n, "fs_is_file.").expect_err("not in the noio build");
        assert!(!e.contains("disabled with --noio"), "{e}");
    }

    #[test]
    fn args_parse_splits_flags_from_plain_arguments() {
        set_args(vec!["--name".into(), "x".into(), "plain".into()]);
        let mut i = interp(HostOptions::default());
        run(&mut i, "args.parse").expect("parse");
        let m = i.peek().expect("a map");
        assert_eq!(m.get("name").map(|v| v.display()).as_deref(), Some("[ x :: ]"));
        assert_eq!(m.get("args").map(|v| v.display()).as_deref(), Some("[ plain :: ]"));
        set_args(Vec::new());
    }

    /// D54: what a URI may be, by curl's rules for `file://`.
    #[test]
    fn fetch_takes_file_urls_by_curls_rules() {
        let d = scratch("fetch");
        let f = d.join("a b.txt");
        std::fs::write(&f, "hi").expect("write");
        let abs = f.display().to_string();
        let encoded = abs.replace(' ', "%20");
        assert_eq!(fetch_uri(&format!("file://{encoded}")).as_deref(), Some("hi"));
        assert_eq!(fetch_uri(&format!("file://localhost{encoded}")).as_deref(), Some("hi"));
        assert_eq!(fetch_uri(&abs), None, "a bare path is not a URL");
        assert_eq!(fetch_uri("file://relative/x"), None, "a relative file URL names a host");
        assert_eq!(fetch_uri("https://example.com/"), None, "https is deferred");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn use_evaluates_a_library_and_file_refuses_a_relative_path() {
        let d = scratch("use");
        let lib = d.join("lib.bund");
        std::fs::write(&lib, ":lib.x { 42 } register").expect("write");
        let mut i = interp(HostOptions::default());
        run(&mut i, &format!("\"file://{}\" use lib.x", lib.display())).expect("use");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(42));
        let e = run(&mut i, "\"relative.txt\" file").expect_err("relative");
        assert!(e.contains("FILE gets no data"), "{e}");
        let e = run(&mut i, "\"lib.bund\" use").expect_err("bare");
        assert!(e.contains("USE can not get from lib.bund"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn noeval_stubs_the_evaluating_words() {
        let mut i = interp(HostOptions {
            noeval: true,
            ..HostOptions::default()
        });
        for src in ["\"1\" bund.eval", "\"1\" !!", "\"file:///x\" use"] {
            let e = run(&mut i, src).expect_err("stubbed");
            assert!(e.contains("functions disabled with --noeval"), "{src}: {e}");
        }
    }

    /// D52: nothing runs after `exit`, inside a lambda or out of one, and the
    /// first code stands.
    #[test]
    fn exit_stops_the_program_and_keeps_its_code() {
        use bund2_api::Vm as _;
        let mut i = interp(HostOptions::default());
        run(&mut i, ":f { 1 7 exit 2 } register 0 f 3").expect("an exit is not a failure");
        assert_eq!(i.exit_requested(), Some(7));
        let left: Vec<i64> = i.snapshot().iter().filter_map(|v| v.as_int()).collect();
        assert_eq!(left, vec![0, 1], "nothing after the exit ran");

        let mut j = interp(HostOptions::default());
        run(&mut j, "exit").expect("runs");
        assert_eq!(j.exit_requested(), Some(0), "an empty stack exits 0");
    }

    /// **F103, F178.** A count the clock cannot hold is refused before the
    /// sleep that would panic on it. The reference aborts on both of these.
    #[test]
    fn sleep_refuses_a_count_the_clock_cannot_hold() {
        for src in ["-3 sleep.seconds", "-1 sleep", "9223372036854775807 sleep.seconds"] {
            let mut i = interp(HostOptions::default());
            let e = run(&mut i, src).expect_err("refused");
            assert!(e.contains("SLEEP::SECONDS: the clock cannot wait"), "{src}: {e}");
            assert!(!e.contains("internal error"), "{src}: {e}");
        }
        let mut i = interp(HostOptions::default());
        run(&mut i, "0 sleep.seconds").expect("a wait of nothing is a wait");
    }

    /// **F178.** An exit code that is not an integer is taken as 0, and said
    /// so — the reference logs the same sentence at error level.
    #[test]
    fn exit_with_a_code_that_is_not_an_integer_exits_zero() {
        use bund2_api::Vm as _;
        let mut i = interp(HostOptions::default());
        run(&mut i, "\"s\" exit").expect("an exit is not a failure");
        assert_eq!(i.exit_requested(), Some(0));
    }

    /// F112: a body a native runs synchronously, ending in `exit`, stops the
    /// native too. `map` used to see `Ok`, collect, and go on: `[1]` came back
    /// as a LIST, and `[1 2]` pushed its second item before the refusal.
    #[test]
    fn an_exit_ending_a_synchronous_body_stops_the_native_that_ran_it() {
        use bund2_api::Vm as _;
        for src in ["[ 1 ] { 7 exit } map", "[ 1 2 ] { 7 exit } map"] {
            let mut i = interp(HostOptions::default());
            run(&mut i, src).expect("an exit is not a failure");
            assert_eq!(i.exit_requested(), Some(7), "{src}");
            let left: Vec<Option<i64>> = i.snapshot().iter().map(BundValue::as_int).collect();
            assert_eq!(left, vec![Some(1)], "{src}: `map` stopped at the first item");
        }
        // The control: `exit` not last already stopped at the next step.
        let mut k = interp(HostOptions::default());
        run(&mut k, "1 [ 1 ] { 7 exit 99 } map").expect("runs");
        let left: Vec<Option<i64>> = k.snapshot().iter().map(BundValue::as_int).collect();
        assert_eq!(left, vec![Some(1), Some(1)]);
    }

    /// RFC-0005's fourteenth review, B1 and S1. After an exit `?try` still
    /// pushes its `error` CONDITIONAL, and the `context` slot keeps the error
    /// its `try` body returned, wrapped by every native between the refusal
    /// and `?try`. Tier 0 never replaces that error, so compiled code must
    /// not either. The `bund.eval` and `context` rows pin F112's gates in
    /// `Interp::apply` and `Vm::scoped_call`: without them, those natives
    /// returned `Ok` and the refusal came later, without their wrapper.
    #[test]
    fn try_keeps_the_error_its_body_returned_after_an_exit() {
        use bund2_api::Vm as _;
        let cases = [
            ("{ 7 exit }", None),
            ("{ [ 1 ] { 7 exit } map }", Some("MAP: lambda execution returns error: ")),
            ("{ 1 { 7 exit } times }", Some("TIMES: lambda execution returns error: ")),
            ("{ \"7 exit\" bund.eval }", Some("Attempt to evaluate value")),
            (
                "{ :scratch context :run { 7 exit } set :run swap ! }",
                Some("CONTEXT lambda returns: "),
            ),
        ];
        for (body, wrapper) in cases {
            let src = format!(
                "?try :try {body} set :except {{ \"EXCEPT\" println }} set \
                 :recovery {{ \"RECOVERY\" println }} set !"
            );
            let mut i = interp(HostOptions::default());
            run(&mut i, &src).expect("an exit is not a failure");
            assert_eq!(i.exit_requested(), Some(7), "{body}");
            let top = i.peek().expect("`?try` left its CONDITIONAL");
            let ctx = top.get("context").and_then(|v| v.as_str()).unwrap_or_default();
            assert!(
                ctx.ends_with("the program asked to exit with code 7"),
                "{body}: {ctx}"
            );
            if let Some(w) = wrapper {
                assert!(ctx.contains(w), "{body}: the native's wrapper is kept: {ctx}");
            }
        }
    }

    /// RFC-0005 criterion 30's drained-body and residual-path cases, Tier 0's
    /// half. `!` on a lambda files the body for the loop to drain rather than
    /// running it, so the exit happens one level further out than in the
    /// `?try` rows above. `map`'s wrapper must still reach the value `?try`
    /// leaves. The second body switches stacks first, which is what puts what
    /// follows on RFC-0005's residual path.
    #[test]
    fn a_drained_body_that_exits_through_a_native_keeps_its_error() {
        use bund2_api::Vm as _;
        for body in [
            "{ { [ 1 ] { 7 exit } map } ! }",
            "{ :scratch to_stack { [ 1 ] { 7 exit } map } ! }",
        ] {
            let src = format!(
                "?try :try {body} set :except {{ \"EXCEPT\" println }} set \
                 :recovery {{ \"RECOVERY\" println }} set !"
            );
            let mut i = interp(HostOptions::default());
            run(&mut i, &src).expect("an exit is not a failure");
            assert_eq!(i.exit_requested(), Some(7), "{body}");
            let top = i.peek().expect("`?try` left its CONDITIONAL");
            let ctx = top.get("context").and_then(|v| v.as_str()).unwrap_or_default();
            assert!(
                ctx.contains("MAP: lambda execution returns error: "),
                "{body}: the native's wrapper is kept: {ctx}"
            );
            assert!(
                ctx.ends_with("the program asked to exit with code 7"),
                "{body}: {ctx}"
            );
        }
    }

    /// F113, caught under `?try`: `!` on a LIST runs `{ 10 }` at once, then
    /// fails on `5`, which is not executable. What the lambda left survives
    /// the failure, as it does in the reference, and `?try`'s CONDITIONAL
    /// comes to rest above it.
    ///
    /// Until F113 this program was RFC-0005 criterion 26's, as the one place
    /// a `bund2-stdlib` native filed a tail request and then failed. The LIST
    /// arm no longer files, so F96's case is an embedder's again and the
    /// criterion cites `a_failed_native_leaves_no_tail_request`
    /// (`crates/bund2-interp/src/lib.rs`).
    #[test]
    fn a_list_execute_that_fails_keeps_what_earlier_items_left() {
        use bund2_api::Vm as _;
        let src = "?try :try { [ { 10 } 5 ] ! } set :except { \"EXCEPT\" println } set \
                   :recovery { \"RECOVERY\" println } set !";
        let mut i = interp(HostOptions::default());
        run(&mut i, src).expect("the error is caught");
        assert_eq!(i.exit_requested(), None, "nothing asked to exit");
        assert_eq!(i.depth(), 2, "`10` beneath `?try`'s CONDITIONAL");
        let top = i.peek().expect("`?try` left its CONDITIONAL");
        let ctx = top.get("context").and_then(|v| v.as_str()).unwrap_or_default();
        assert!(
            ctx.contains("Received value is not of executable type"),
            "{ctx}"
        );
        let left: Vec<Option<i64>> = i.snapshot().iter().map(BundValue::as_int).collect();
        assert_eq!(left.first().copied(), Some(Some(10)), "`{{ 10 }}` had run");
    }

    /// **F116**, where it bites: the source `bund.eval` parses is a value the
    /// program built, so its depth is the program's choice. A 16,000-deep
    /// literal aborted the process; now the parser refuses and the refusal
    /// arrives as an ordinary Bund error, catchable by `?try`.
    #[test]
    fn evaluating_a_too_deeply_nested_string_reports_instead_of_aborting() {
        let n = bund2_syntax::MAX_NESTING + 1;
        let src = format!("\"{}1{}\" bund.eval", "[ ".repeat(n), " ]".repeat(n));
        let mut i = interp(HostOptions::default());
        let e = run(&mut i, &src).expect_err("refused, not fatal");
        assert!(e.contains("nesting deeper than"), "{e}");
    }

    #[test]
    fn io_graph_wants_floats() {
        let mut i = interp(HostOptions::default());
        let e = run(&mut i, "[ 1 2 ] io.graph").expect_err("ints refused");
        assert!(e.contains("IO.GRAPH casting data element returns"), "{e}");
    }

    /// What a TIME value refuses, each of which ends a program and so cannot
    /// sit in the probe. Every message measured on the oracle.
    ///
    /// The reference names `Val::Time` in equality and ordering and nowhere
    /// else, so printing and converting reach `conv`'s final arm and arithmetic
    /// reaches `numeric_op`'s -- which says `X` when the TIME is on top and
    /// lets the number's own arm say `Y` when it is underneath.
    #[test]
    fn a_time_value_refuses_what_the_reference_refuses() {
        for (src, want) in [
            (
                "1 time.timestamp println",
                "PRINTLN returns: Can not convert Value from 13",
            ),
            (
                "1 time.timestamp convert.to_string",
                "CONVERT.TO_STRING returned error: Can not convert Value from 13",
            ),
            (
                "1 time.timestamp convert.to_int",
                "CONVERT.TO_INTEGER returned error: Can not convert Value from 13",
            ),
            (
                "1 time.timestamp 2 time.timestamp +",
                "ADD returns error: Incompartible X argument for the math operations: 13",
            ),
            (
                "5 1 time.timestamp +",
                "ADD returns error: Incompartible X argument for the math operations: 13",
            ),
            (
                "1 time.timestamp 5 +",
                "ADD returns error: Incompartible Y argument for the math operations",
            ),
            (
                "[ 5 ] 1 time.timestamp -",
                "SUB returns error: Incompartible operation for the list",
            ),
            (
                "time.timestamp",
                "Stack is too shallow for inline time.timestamp",
            ),
        ] {
            let mut i = interp(HostOptions::default());
            let e = run(&mut i, src).expect_err(src);
            assert!(e.ends_with(want), "{src}: {e}");
        }
    }

    /// `-1 time.timestamp` wraps, because the reference casts `as u128`.
    #[test]
    fn a_negative_timestamp_wraps_as_the_reference_casts_it() {
        let mut i = interp(HostOptions::default());
        run(&mut i, "-1 time.timestamp").expect("made");
        assert_eq!(i.peek().and_then(|v| v.as_time()), Some(u128::MAX));
    }

    /// The conversions `display` can render and `conv` cannot -- F166.
    ///
    /// `convert.to_string` took `display`'s answer for every source, so a PAIR
    /// came back as `[ 2 ::  1 :: ]` where the oracle refuses it.
    #[test]
    fn to_string_refuses_what_conv_has_no_arm_for() {
        for (src, dt) in [("1 2 pair", 10), ("1.0 2.0 complex", 15)] {
            let mut i = interp(HostOptions::default());
            let e = run(&mut i, &format!("{src} convert.to_string")).expect_err(src);
            let want =
                format!("CONVERT.TO_STRING returned error: Can not convert Value from {dt}");
            assert!(e.ends_with(&want), "{src}: {e}");
        }
    }
}
