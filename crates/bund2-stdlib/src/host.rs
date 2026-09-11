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
//! **None of these can print twice the same way on every machine** except
//! `io.graph`, `args` on an empty command line, and the failure paths. The
//! probe `tests/probes/host-words.bund` pins those; the rest are unit-tested.

use std::sync::Mutex;

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LIST, STRING};

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

fn guard(vm: &dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
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

/// `fs.rm` — remove files and directories, recursively
/// (`reference/Bund/src/stdlib/functions/filesystem/cp.rs:29-119`, `Remove`).
///
/// The operand is one path or a list of them. `fs_extra::remove_items` removes
/// each, and a path that does not exist is not an error. The answer is `true`.
fn fs_rm(vm: &mut dyn Vm) -> Result<(), Error> {
    const P: &str = "FS.RM";
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {P}")));
    }
    let v = vm.pull().ok_or_else(|| Error(format!("{P} NO DATA #1")))?;
    let not_string = || Error(format!("Error casting string for {P}: This Dynamic type is not string"));
    let paths: Vec<String> = match v.dt() {
        STRING => vec![v.as_str().ok_or_else(not_string)?],
        LIST => {
            let items = v.as_list().ok_or_else(|| {
                Error(format!("Error casting list for {P}: This Dynamic type is not list"))
            })?;
            let mut out = Vec::with_capacity(items.len());
            for i in items {
                out.push(i.as_str().ok_or_else(not_string)?);
            }
            out
        }
        _ => return Err(Error(format!("Incorrect #1 type for {P}"))),
    };
    fs_extra::remove_items(&paths).map_err(|e| Error(format!("{P} returns: {e}")))?;
    vm.push(BundValue::boolean(true));
    Ok(())
}

/// `sleep.seconds` — wait whole seconds
/// (`reference/Bund/src/stdlib/functions/system/sleep.rs:11-21`).
///
/// The count must be an INTEGER, and it is converted with `as u64` (`:19`), so
/// **a negative count waits for about 585 billion years** — F103, reproduced.
fn sleep_seconds(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = vm
        .pull()
        .ok_or_else(|| Error("SLEEP::SECONDS: NO DATA #1".into()))?;
    let n = v.as_int().ok_or_else(|| {
        Error("SLEEP::SECONDS error casting seconds: This Dynamic type is not integer".into())
    })?;
    spin_sleep::sleep(std::time::Duration::new(n as u64, 0));
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

/// `file` — read a file into a string
/// (`reference/Bund/src/stdlib/functions/filesystem/file.rs:15-62`).
///
/// **D51: read with `std::fs`, where the reference fetches through curl.** The
/// reference builds `file://{path}` and hands it to libcurl
/// (`reference/Bund/src/stdlib/helpers/file_helper.rs:57-59`). Any failure
/// becomes `None` (`:46-50`), reported as `FILE gets no data`
/// (`file.rs:47-48`), and the bytes are decoded lossily (`file_helper.rs:54`).
/// This does the same with a plain read. The answer goes back to the word's
/// side (`file.rs:42-45`).
fn file_word(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    guard(vm, side, prefix)?;
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA")))?;
    let path = v.as_str().ok_or_else(|| {
        Error(format!("{prefix} returns: This Dynamic type is not string"))
    })?;
    let bytes = std::fs::read(&path).map_err(|_| Error(format!("{prefix} gets no data")))?;
    side.push(vm, BundValue::str(String::from_utf8_lossy(&bytes).to_string()));
    Ok(())
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
        stub!("system.setproctitle", "SYSTEM.SETPROCTITLE", eff(1, 0));
        stub!("system.setproctitle.", "SYSTEM.SETPROCTITLE", eff(0, 0));
        stub!("file", "FILE", eff(1, 1));
        stub!("file.", "FILE", eff(0, 0));
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
        r.register_native("fs.rm", fs_rm, eff(1, 1), WordKind::Sync);
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
            |vm| file_word(vm, Side::Stack, "FILE"),
            eff(1, 1),
            WordKind::Sync,
        );
        r.register_native(
            "file.",
            |vm| file_word(vm, Side::Bench, "FILE."),
            eff(0, 0),
            WordKind::Sync,
        );
    }

    // `reference/Bund/src/stdlib/functions/create_aliases.rs:16-19,40`.
    r.register_alias("rm", "fs.rm");
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

    fn scratch(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("bund2-host-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("scratch dir");
        d
    }

    #[test]
    fn noio_replaces_the_io_words_with_the_reference_stubs() {
        let mut i = interp(HostOptions { noio: true });
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

    #[test]
    fn io_graph_wants_floats() {
        let mut i = interp(HostOptions::default());
        let e = run(&mut i, "[ 1 2 ] io.graph").expect_err("ints refused");
        assert!(e.contains("IO.GRAPH casting data element returns"), "{e}");
    }
}
