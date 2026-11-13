//! **D249: `vm.spawn`, `vm.join` and `vm.done?`.**
//!
//! Each test runs a program on a thread sized and declared as the CLI's
//! evaluation thread is, and reads the stack it leaves.
//!
//! **A binary of its own, and one test in it at a time,** because the count
//! of live VMs is process-wide: a test that fills it to the limit would make
//! a spawn in a test running beside it fail.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Mutex;

use bund2_api::Vm;
use bund2_runtime::{HostOptions, vms};
use bund2_value::BundValue;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Run `src` in a fresh VM under `opts` and answer its final stack, or the
/// error it stopped with. The caller holds [`ONE_AT_A_TIME`]. A value is not
/// `Send`, so the stack crosses back encoded, as `vm.join`'s does.
fn run_unlocked(opts: HostOptions, src: String) -> Result<Vec<BundValue>, String> {
    let bytes = bund2_runtime::vm_thread("vm-words-test".into())
        .spawn(move || {
            let mut rt = bund2_runtime::vm_on_this_thread(&opts, None);
            rt.eval_str(&src).map_err(|e| e.0)?;
            Ok::<_, String>(
                bund2_value::wire::to_binary(&BundValue::list(rt.interp.snapshot()))
                    .expect("the stack encodes"),
            )
        })
        .expect("the test thread starts")
        .join()
        .expect("the test thread does not panic")?;
    let stack = bund2_value::wire::from_binary(&bytes).expect("the stack decodes");
    Ok(stack.as_list().expect("a LIST").to_vec())
}

fn run_with(opts: HostOptions, src: &str) -> Result<Vec<BundValue>, String> {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    run_unlocked(opts, src.to_string())
}

fn run(src: &str) -> Result<Vec<BundValue>, String> {
    run_with(HostOptions::default(), src)
}

/// The items of the LIST on top of `stack`, as integers.
fn ints_on_top(stack: &[BundValue]) -> Vec<i64> {
    stack
        .last()
        .and_then(|v| v.as_list())
        .expect("a LIST on top")
        .iter()
        .map(|v| v.as_int().expect("an INTEGER"))
        .collect()
}

/// R3, R6: a LAMBDA runs in the child, and `vm.join` answers its final stack.
#[test]
fn a_lambda_runs_in_a_child_and_join_answers_its_stack() {
    let s = run("{ 1 2 + 10 } [ 5 ] vm.spawn vm.join").unwrap();
    assert_eq!(s.len(), 1, "the handle was consumed: {s:?}");
    assert_eq!(ints_on_top(&s), vec![5, 3, 10]);
}

/// R3: a STRING of Bund source runs too.
#[test]
fn a_string_program_runs_in_a_child() {
    let s = run(r#""6 7 *" list vm.spawn vm.join"#).unwrap();
    assert_eq!(ints_on_top(&s), vec![42]);
}

/// R4: the start-up values are on the child's stack, first item lowest.
#[test]
fn the_start_up_values_are_on_the_child_s_stack_in_order() {
    let s = run("{ 0 drop } [ 1 2 3 ] vm.spawn vm.join").unwrap();
    assert_eq!(ints_on_top(&s), vec![1, 2, 3]);
}

/// R5: handles are integers, distinct, and each child answers for itself.
#[test]
fn each_handle_names_its_own_child() {
    let s = run(
        ":a { 0 drop } [ 1 ] vm.spawn var :b { 0 drop } [ 2 ] vm.spawn var \
         :b var? vm.join :a var? vm.join",
    )
    .unwrap();
    assert_eq!(s.len(), 2);
    assert_eq!(ints_on_top(&s[..1]), vec![2]);
    assert_eq!(ints_on_top(&s), vec![1]);
}

/// R6 i: a child that fails makes `vm.join` fail, naming the handle.
#[test]
fn a_failed_child_fails_the_join() {
    let e = run("{ drop } list vm.spawn vm.join").unwrap_err();
    assert!(e.contains("VM.JOIN: VM ") && e.contains(" failed: "), "{e}");
}

/// R5: the first join takes the answer; a second is an error, and so is a
/// handle `vm.spawn` never gave.
#[test]
fn a_handle_is_joined_once() {
    let e =
        run(":h { 0 drop } list vm.spawn var :h var? vm.join drop :h var? vm.join").unwrap_err();
    assert!(e.contains("no VM ") && e.contains("has been joined"), "{e}");
    let e = run("999999 vm.done?").unwrap_err();
    assert!(e.contains("VM.DONE?: no VM 999999"), "{e}");
}

/// `vm.done?` answers without waiting: false while the child waits on the
/// bus, and the parent's `send` reaches it, so the two VMs talk.
#[test]
fn done_does_not_wait_and_a_parent_and_child_talk_over_the_bus() {
    let s = run(r#"
        "vmw.gate" bus.data drop
        :h {
          { "vmw.gate" recv type.of 97 == { 1 + false } { drop 0 sleep true } ifthenelse } for
        } list vm.spawn var
        :h var? vm.done?
        "vmw.gate" 41 send.quick
        :h var? vm.join
        "#)
    .unwrap();
    assert_eq!(s.len(), 2, "{s:?}");
    assert!(
        matches!(s[0].unboxed(), BundValue::Bool(false, _)),
        "the child was still waiting: {s:?}"
    );
    assert_eq!(ints_on_top(&s), vec![42]);
}

/// R7 i: under `--noeval` a STRING program is refused and a LAMBDA is not;
/// R7 A: the child inherits `--noeval`, so it cannot evaluate either.
#[test]
fn noeval_refuses_a_string_program_and_reaches_the_child() {
    let noeval = HostOptions {
        noeval: true,
        ..HostOptions::default()
    };
    let e = run_with(noeval, r#""1" list vm.spawn"#).unwrap_err();
    assert!(e.contains("disabled with --noeval"), "{e}");
    let s = run_with(noeval, "{ 7 } list vm.spawn vm.join").unwrap();
    assert_eq!(ints_on_top(&s), vec![7]);
    let e = run_with(noeval, r#"{ "1 2 +" bund.eval } list vm.spawn vm.join"#).unwrap_err();
    assert!(e.contains("VM.JOIN") && e.contains("--noeval"), "{e}");
}

/// The operands are checked before anything starts.
#[test]
fn spawn_checks_its_operands() {
    let e = run("{ 0 drop } 5 vm.spawn").unwrap_err();
    assert!(e.contains("must be a LIST"), "{e}");
    let e = run("5 list vm.spawn").unwrap_err();
    assert!(e.contains("a LAMBDA or a STRING"), "{e}");
    let e = run(r#""x" vm.join"#).unwrap_err();
    assert!(e.contains("is an INTEGER"), "{e}");
}

/// **Additive:** a program's own `vm.spawn` is the one it calls, because a
/// slot's lambda resolves before its native.
#[test]
fn a_program_s_own_word_of_the_same_name_still_wins() {
    let s = run(":vm.spawn { 99 } register 1 2 vm.spawn").unwrap();
    let ints: Vec<i64> = s.iter().map(|v| v.as_int().unwrap()).collect();
    assert_eq!(ints, vec![1, 2, 99]);
}

/// R8: past the limit `vm.spawn` fails naming it, and the slots come back
/// when the children end. The children wait on a channel, so all of them are
/// live at once. The lock is held across the three runs, so no other test
/// spawns while the count is full.
#[test]
fn spawning_past_the_limit_fails_naming_it() {
    const WAIT: &str = r#"{ { "vmw.hold" recv type.of 97 == { drop false } { drop 0 sleep true } ifthenelse } for }"#;
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let full = run_unlocked(
        HostOptions::default(),
        format!(
            r#""vmw.hold" bus.data drop
            {WAIT} list vm.spawn
            {n} {{ drop {WAIT} list vm.spawn drop }} times"#,
            n = vms::LIMIT - 1
        ),
    )
    .unwrap();
    let first = full
        .last()
        .and_then(|v| v.as_int())
        .expect("the first handle");
    let e = run_unlocked(HostOptions::default(), "{ 0 drop } list vm.spawn".into()).unwrap_err();
    assert!(
        e.contains(&format!(
            "{} VMs started with vm.spawn are running",
            vms::LIMIT
        )),
        "{e}"
    );
    // Release every child and join each, so the count and the table are
    // clear for the next test.
    let s = run_unlocked(
        HostOptions::default(),
        format!(
            r#"{n} {{ drop "vmw.hold" "go" send.quick }} times
            :i {first} var
            {n} {{ drop :i var? vm.join drop :i :i var? 1 + var }} times
            {{ 1 }} list vm.spawn vm.join"#,
            n = vms::LIMIT
        ),
    )
    .unwrap();
    assert_eq!(ints_on_top(&s), vec![1], "a slot is free again");
}
