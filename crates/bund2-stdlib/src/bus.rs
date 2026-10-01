//! The local bus — `send`, `recv`, and the two predicates.
//!
//! **In scope under D87**, which narrowed D28's deferral of the reference's
//! `bus` directory to its zenoh half. The purpose D87 names is exchanging data
//! between VMs running in one process; zenoh's words stay deferred, and
//! `globals.rs` is still the one path `DEFERRED_PATHS` lists.
//!
//! **The transport is a process-global map of named byte channels**, as the
//! reference's `PIPES` is. Process-global is not incidental: a bus between VMs
//! needs a medium every VM can reach, and a per-VM map would be a bus to
//! oneself (RFC-0007 §C8).
//!
//! **Bytes rather than values, and not by choice.** `Interp` is not `Send` and
//! every heap value carries an `Rc`, so no value can cross a thread boundary
//! (D84, D86). The payload is the wire format, which `bund2_value::wire`
//! already implements against the reference's own `to_binary`/`from_binary`.
//!
//! **Whether a received value keeps the identity it was sent with is still
//! open** — D86 records it as a decision rather than an implementation detail,
//! and this module does not take it: `from_binary` decides, and F109 is where
//! the current answer is written down.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;
use crossbeam_channel::{Receiver, Sender, unbounded};

use crate::wb::Side;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

type Pipe = (Sender<Vec<u8>>, Receiver<Vec<u8>>);

/// Every named channel in this process.
///
/// **`in` and `out` exist before any program runs**, because the reference's
/// `pipes_init` inserts them unconditionally at startup. The difference is
/// observable: `"in" recv` answers `NODATA` where any other untouched name is
/// an error, and RFC-0007's first draft missed them.
fn pipes() -> &'static Mutex<BTreeMap<String, Pipe>> {
    static PIPES: OnceLock<Mutex<BTreeMap<String, Pipe>>> = OnceLock::new();
    PIPES.get_or_init(|| {
        let mut m: BTreeMap<String, Pipe> = BTreeMap::new();
        m.insert("in".to_string(), unbounded());
        m.insert("out".to_string(), unbounded());
        Mutex::new(m)
    })
}

/// A poisoned lock is a broken invariant with no sensible continuation, so it
/// names itself and routes through the diagnostic path — D37's third way out.
fn with_pipes<T>(
    what: &str,
    f: impl FnOnce(&mut BTreeMap<String, Pipe>) -> Result<T, String>,
) -> Result<T, String> {
    match pipes().lock() {
        Ok(mut g) => f(&mut g),
        Err(e) => Err(format!(
            "the bus's channel map is poisoned, so {what} cannot proceed: {e}"
        )),
    }
}

/// The reference's `bus_push`: serialise, then send, creating the channel if
/// this is the first push to that name.
fn push(name: String, v: &BundValue) -> Result<bool, String> {
    let bytes = bund2_value::wire::to_binary(v)
        .map_err(|e| format!("Error enveloping data: {e}"))?;
    with_pipes("a send", |m| {
        let (s, _) = m.entry(name).or_insert_with(unbounded);
        s.send(bytes)
            .map_err(|e| format!("bus::internal::pipe error: {e}"))?;
        Ok(true)
    })
}

/// The reference's `bus_pull` — **two arms, and the difference is the one
/// RFC-0007's first draft got wrong**.
///
/// An **absent** channel is an error. An **existing but empty** one answers
/// `NODATA`. The draft measured a name nothing had created, hit the first arm,
/// and reported it as the second.
fn pull(name: String) -> Result<BundValue, String> {
    with_pipes("a recv", |m| {
        let Some((_, r)) = m.get(&name) else {
            return Err(format!("bus::internal::pipe no pipe: {name}"));
        };
        if r.is_empty() {
            return Ok(BundValue::nodata());
        }
        let bytes = r
            .recv()
            .map_err(|e| format!("bus::internal::pipe {name} can not recv: {e}"))?;
        bund2_value::wire::from_binary(&bytes)
            .map_err(|e| format!("Error converting from binary: {e}"))
    })
}

/// The reference's `ensure_bus` — **it creates the channel, then reports
/// whether it has data**.
///
/// Asking the question has a side effect, which reads like a bug and is
/// faithful; and the answer is advisory, because the lock is released before
/// the caller acts, so under several VMs a `true` can be followed by `NODATA`
/// (RFC-0007 §C8). `ensure_bus` cannot fail in the reference either — it
/// returns a plain `bool`.
fn ensure(name: String) -> Result<bool, String> {
    with_pipes("a bus.data", |m| {
        let (_, r) = m.entry(name).or_insert_with(unbounded);
        Ok(!r.is_empty())
    })
}

/// A channel name off a stack.
///
/// **`BUS.DATA:` is the prefix even inside `send` and `recv`**, which report a
/// name that will not cast as `BUS.DATA: Error name casting: …` whatever word
/// was called. The quirk is in every one of the reference's six `cast_string`
/// arms, so it is preserved rather than tidied.
fn name_of(v: &BundValue) -> Result<String, Error> {
    v.as_str().ok_or_else(|| {
        Error("BUS.DATA: Error name casting: This Dynamic type is not string".into())
    })
}

/// A bus error reaches the program wrapped: `{prefix} returns error {err}`.
fn wrapped(prefix: &str, e: String) -> Error {
    Error(format!("{prefix} returns error {e}"))
}

/// `send`, `send.`, `send.quick` and `send.quick.`.
///
/// `quick` differs in **one thing**: it drops the bool `bus_push` answers, so
/// `eff(2, 0)` against `send`'s `eff(2, 1)`.
///
/// **The object comes from `side`; the name always comes from the stack**, so
/// `send.` reads its object off the workbench, its name off the stack, and
/// pushes any bool to the stack. The depth guards differ between the two forms
/// for that reason, and the reference writes them out twice.
fn send_base(vm: &mut dyn Vm, side: Side, quick: bool, prefix: &str) -> Result<(), Error> {
    let shallow = || Error(format!("Stack is too shallow for inline {prefix}"));
    match side {
        Side::Stack => {
            if vm.depth() < 2 {
                return Err(shallow());
            }
        }
        Side::Bench => {
            if vm.depth() < 1 {
                return Err(shallow());
            }
            if vm.workbench_depth() < 1 {
                return Err(Error(format!(
                    "Workbench is too shallow for inline {prefix}"
                )));
            }
        }
    }
    let object = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix}: No object for sending was provided")))?;
    let name_val = vm
        .pull()
        .ok_or_else(|| Error(format!("{prefix}: No channel name discovered on the stack")))?;
    let name = name_of(&name_val)?;
    let answered = push(name, &object).map_err(|e| wrapped(prefix, e))?;
    if !quick {
        vm.push(BundValue::boolean(answered));
    }
    Ok(())
}

/// `recv` and `recv.`.
///
/// **`recv.` takes its *name* off the workbench and still pushes the value to
/// the stack** — an asymmetry with `send.`, whose workbench side is the
/// object. So `recv.` consumes nothing from the stack and leaves one value on
/// it, and its only depth guard is the workbench's.
fn recv_base(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    match side {
        Side::Stack => {
            if vm.depth() < 1 {
                return Err(Error(format!("Stack is too shallow for inline {prefix}")));
            }
        }
        Side::Bench => {
            if vm.workbench_depth() < 1 {
                return Err(Error(format!(
                    "Workbench is too shallow for inline {prefix}"
                )));
            }
        }
    }
    let name_val = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix}: No channel name discovered on the stack")))?;
    let name = name_of(&name_val)?;
    let got = pull(name).map_err(|e| wrapped(prefix, e))?;
    vm.push(got);
    Ok(())
}

fn send_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    send_base(vm, Side::Stack, false, "SEND")
}

fn send_bench(vm: &mut dyn Vm) -> Result<(), Error> {
    send_base(vm, Side::Bench, false, "SEND.")
}

/// **`send.quick`'s prefix is `SEND`, not `SEND.QUICK`** — the reference
/// passes the same string the plain form does.
fn send_quick_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    send_base(vm, Side::Stack, true, "SEND")
}

fn send_quick_bench(vm: &mut dyn Vm) -> Result<(), Error> {
    send_base(vm, Side::Bench, true, "SEND.")
}

fn recv_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    recv_base(vm, Side::Stack, "RECV")
}

fn recv_bench(vm: &mut dyn Vm) -> Result<(), Error> {
    recv_base(vm, Side::Bench, "RECV.")
}

/// `bus.data` — a name off the stack.
fn bus_data(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for BUS.DATA".into()));
    }
    let name_val = vm
        .pull()
        .ok_or_else(|| Error("BUS.DATA: No channel name discovered on the stack".into()))?;
    let name = name_of(&name_val)?;
    let has = ensure(name).map_err(|e| wrapped("BUS.DATA", e))?;
    vm.push(BundValue::boolean(has));
    Ok(())
}

/// `bus.data.current` — **the current stack's own name is the channel**, so a
/// stack and a channel share one namespace.
///
/// The reference bails with `Can not determine current stack name` when
/// `current_stack_name` answers `None`. Bund2's `current_name` answers `main`
/// for an empty stack-of-stacks, so there is no arm to write — D37's first way
/// out, the invariant made structural.
fn bus_data_current(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = vm.current_name();
    let has = ensure(name).map_err(|e| wrapped("BUS.DATA", e))?;
    vm.push(BundValue::boolean(has));
    Ok(())
}

/// `--noio` replaces all eight with one stub, as it does for every other I/O
/// group.
fn disabled(_vm: &mut dyn Vm) -> Result<(), Error> {
    Err(Error("bund BUS functions disabled with --noio".into()))
}

/// The eight names, in the order the reference registers them.
const NAMES: [&str; 8] = [
    "send",
    "send.quick",
    "send.",
    "send.quick.",
    "recv",
    "recv.",
    "bus.data.current",
    "bus.data",
];

pub fn register(r: &mut Registry, opts: &crate::host::HostOptions) {
    if opts.noio {
        for n in NAMES {
            r.register_native(n, disabled, eff(0, 0), WordKind::Sync);
        }
        return;
    }
    r.register_native("send", send_stack, eff(2, 1), WordKind::Sync);
    r.register_native("send.quick", send_quick_stack, eff(2, 0), WordKind::Sync);
    // The workbench forms take one value off the stack — the name — and their
    // object from the other side, which the effect cannot express.
    r.register_native("send.", send_bench, eff(1, 1), WordKind::Sync);
    r.register_native("send.quick.", send_quick_bench, eff(1, 0), WordKind::Sync);
    r.register_native("recv", recv_stack, eff(1, 1), WordKind::Sync);
    r.register_native("recv.", recv_bench, eff(0, 1), WordKind::Sync);
    r.register_native(
        "bus.data.current",
        bus_data_current,
        eff(0, 1),
        WordKind::Sync,
    );
    r.register_native("bus.data", bus_data, eff(1, 1), WordKind::Sync);
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use bund2_api::Vm as _;
    use bund2_interp::Interp;
    use bund2_value::BundValue;

    /// Channels are **process-global**, so two tests sharing a name would
    /// share a queue. Every name below is unique to its test, prefixed `t_`.
    ///
    /// **That is not only about the tests in this module.** The promotable
    /// audit in `lib.rs` runs every fixed-effect native over its operand
    /// palette, and `send` is now one of them — so by the time any test here
    /// runs, channels may exist under the palette's strings, `"main"` among
    /// them. `"main"` is also what `bus.data.current` names on the default
    /// stack, which is why the test for it switches to a `t_`-named stack
    /// first rather than measuring the one it starts on.
    /// The interpreter wraps a word's error in `Attempt to evaluate value …
    /// returned error: <text>`, so a test asserts the tail.
    fn failed_with(src: &str, want: &str) {
        let e = run(src).expect_err(src);
        assert!(
            e.ends_with(&format!("returned error: {want}")) || e == want,
            "{src}: {e}"
        );
    }

    fn run(src: &str) -> Result<Vec<BundValue>, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let ir = bund2_syntax::compile(src).map_err(|e| format!("{e:?}"))?;
        i.eval(&ir).map_err(|e| e.0)?;
        Ok(i.snapshot())
    }

    /// A bool off the stack, looking through boxing as the dumps do.
    fn truth(v: &BundValue) -> Option<bool> {
        match v.unboxed() {
            BundValue::Bool(b, _) => Some(*b),
            _ => None,
        }
    }

    /// A value sent is the value received — through the wire format, because
    /// nothing else can cross a thread.
    #[test]
    fn a_value_survives_the_round_trip() {
        let out = run(r#""t_round" [ 1 2.5 "x" ] send.quick "t_round" recv"#).expect("runs");
        assert_eq!(out.len(), 1, "one value back");
        assert_eq!(out[0].summary(80), "list/3 [1, 2.5, \"x\"]", "{:?}", out[0]);
    }

    /// **`send` answers a bool, `send.quick` answers nothing.** That is the
    /// whole difference between them.
    #[test]
    fn send_answers_a_bool_and_quick_send_answers_nothing() {
        let out = run(r#""t_bool" 1 send"#).expect("runs");
        assert_eq!(out.len(), 1);
        assert_eq!(truth(&out[0]), Some(true));
        let out = run(r#""t_quick" 1 send.quick"#).expect("runs");
        assert!(out.is_empty(), "quick leaves nothing: {out:?}");
    }

    /// **The two arms of `bus_pull`.** An existing-but-empty channel answers
    /// `NODATA`; an absent one is an error. RFC-0007's first draft reported
    /// the first behaviour having measured the second.
    #[test]
    fn an_empty_channel_is_nodata_and_an_absent_one_is_an_error() {
        // `in` exists before any program runs, so it is empty rather than
        // absent — which is the only way to reach the NODATA arm without
        // creating a channel first.
        let out = run(r#""in" recv"#).expect("runs");
        assert_eq!(out.len(), 1);
        assert!(out[0].dt() == bund2_value::NODATA, "{:?}", out[0]);

        failed_with(
            r#""t_absent" recv"#,
            "RECV returns error bus::internal::pipe no pipe: t_absent",
        );
    }

    /// `out` is pre-created too, and nothing else is.
    #[test]
    fn in_and_out_are_the_only_channels_a_program_starts_with() {
        assert!(run(r#""out" recv"#).is_ok());
        assert!(run(r#""t_unused" recv"#).is_err());
    }

    /// **`bus.data` creates the channel it was asked about.** So the second
    /// call cannot error where the first did, and a `recv` after a `false`
    /// answers `NODATA` rather than failing.
    #[test]
    fn asking_whether_a_channel_has_data_creates_it() {
        let out = run(r#""t_ensure" bus.data "t_ensure" recv"#).expect("runs");
        assert_eq!(out.len(), 2);
        assert_eq!(truth(&out[0]), Some(false), "nothing in it yet");
        assert!(out[1].dt() == bund2_value::NODATA, "and so NODATA, not an error");
    }

    /// `bus.data.current` names the channel after the current stack, so the
    /// two share one namespace.
    #[test]
    fn the_current_stacks_name_is_a_channel_name() {
        // `@t_cur`, not the default stack: see the note on `run` — the
        // promotable audit may have left a value in a channel called `main`.
        let out = run("@t_cur 1 drop bus.data.current").expect("runs");
        assert_eq!(out.last().and_then(truth), Some(false));
        // Sending to a channel of that name is what the predicate then sees.
        let out = run(r#"@t_cur2 "t_cur2" 5 send.quick bus.data.current"#).expect("runs");
        assert_eq!(out.last().and_then(truth), Some(true));
    }

    /// **`recv.` takes its name off the workbench and pushes to the stack** —
    /// the asymmetry with `send.`, whose workbench side is the object.
    #[test]
    fn the_workbench_forms_take_different_operands_from_the_workbench() {
        let out = run(r#""t_wbs" 9 { true } ?. send. "t_wbs" recv"#).expect("runs");
        // The bool `send.` pushed, then the value `recv` brought back.
        assert_eq!(out.len(), 2);
        assert_eq!(truth(&out[0]), Some(true));
        assert_eq!(out[1].as_int(), Some(9));

        let out = run(r#""t_wbr" 5 send.quick "t_wbr" { true } ?. recv."#).expect("runs");
        assert_eq!(out.len(), 1, "the name left the workbench, the value came to the stack");
        assert_eq!(out[0].as_int(), Some(5));
    }

    /// Every depth guard, with the reference's exact words — including that
    /// `send.quick`'s prefix is `SEND`, not `SEND.QUICK`.
    #[test]
    fn the_depth_guards_name_the_word_the_reference_names() {
        for (src, want) in [
            ("1 send", "Stack is too shallow for inline SEND"),
            ("1 send.quick", "Stack is too shallow for inline SEND"),
            ("recv", "Stack is too shallow for inline RECV"),
            ("recv.", "Workbench is too shallow for inline RECV."),
            (r#""c" send."#, "Workbench is too shallow for inline SEND."),
            ("bus.data", "Stack is too shallow for BUS.DATA"),
        ] {
            failed_with(src, want);
        }
    }

    /// **`BUS.DATA:` prefixes a name-casting failure whatever word was
    /// called.** The reference writes that string in all six arms.
    #[test]
    fn a_name_that_will_not_cast_is_always_a_bus_data_error() {
        const WANT: &str = "BUS.DATA: Error name casting: This Dynamic type is not string";
        for src in ["7 7 send", "7 recv", "7 bus.data"] {
            failed_with(src, WANT);
        }
    }

    /// A channel is a queue: first in, first out.
    #[test]
    fn a_channel_keeps_order() {
        let out = run(r#""t_fifo" 1 send.quick "t_fifo" 2 send.quick "t_fifo" recv "t_fifo" recv"#)
            .expect("runs");
        assert_eq!(
            out.iter().filter_map(BundValue::as_int).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    /// **Under `--noio` all eight fail with one message**, and the stub is
    /// registered for every name — so none of them can reach a channel.
    #[test]
    fn noio_replaces_every_one_of_the_eight() {
        let opts = crate::host::HostOptions {
            noio: true,
            ..Default::default()
        };
        let mut i = Interp::new();
        crate::register_all_with(&mut i.registry, &opts);
        for name in super::NAMES {
            let ir = bund2_syntax::compile(name).expect("compiles");
            let e = i.eval(&ir).expect_err(name).0;
            assert!(
                e.ends_with("returned error: bund BUS functions disabled with --noio"),
                "{name}: {e}"
            );
        }
    }

    /// **The stamp crosses, the identity does not** — F109's shape, now
    /// reachable through the bus. No word exposes a value's id, and
    /// `debug.display_stack` is normalised for it (F14), so no program can
    /// tell; the open half of D86 is which of those two the bus should
    /// guarantee.
    #[test]
    fn a_received_value_keeps_its_stamp() {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let ir = bund2_syntax::compile(r#""t_stamp" 42 send.quick"#).expect("compiles");
        i.eval(&ir).expect("sends");
        let ir = bund2_syntax::compile(r#""t_stamp" recv"#).expect("compiles");
        i.eval(&ir).expect("receives");
        let got = i.snapshot();
        assert_eq!(got.len(), 1);
        // A stamp exists and is not the moment of the `recv`: it was taken
        // when the value was first observed, before the send (D2).
        assert!(got[0].timestamp().0 > 0.0, "{:?}", got[0]);
    }
}
