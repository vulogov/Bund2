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
    let code = vm.pull().and_then(|v| v.as_int()).unwrap_or(0);
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
    for name in ["bund.eval", "bund.eval."] {
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

    #[test]
    fn io_graph_wants_floats() {
        let mut i = interp(HostOptions::default());
        let e = run(&mut i, "[ 1 2 ] io.graph").expect_err("ints refused");
        assert!(e.contains("IO.GRAPH casting data element returns"), "{e}");
    }
}
