//! **RFC-0011's record, checked over the record and not its renderings** —
//! criteria 5, 6 and 8.
//!
//! Each test runs a program through the interpreter with a profile installed,
//! then reads the nodes and edges back. The arithmetic §S2 promises is what
//! is asserted: exclusive times sum to the wall, a word's inclusive time is
//! its outermost activations', and the mirror is empty when the run ends,
//! however it ended.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use bund2_api::Vm;
use bund2_interp::Interp;
use bund2_interp::profile::{Key, Kind, Node, Profile};

/// Run `src` profiled. Answers the interpreter, the record after `finish`,
/// how many mirror entries were open when the run ended, and the run's
/// outcome.
fn profiled(src: &str) -> (Interp, Box<Profile>, usize, Result<(), String>) {
    let mut i = Interp::new();
    bund2_stdlib::register_all(&mut i.registry);
    i.profile = Some(Box::new(Profile::new(Duration::ZERO, Duration::ZERO)));
    let stream = bund2_syntax::compile(src).expect("compiles");
    let outcome = i.eval(&stream).map_err(|e| e.0);
    let mut p = i.profile.take().unwrap();
    let open = p.open_entries();
    p.finish();
    (i, p, open, outcome)
}

fn name(i: &Interp, n: &Node) -> String {
    match &n.key {
        Key::Sym(s, _) => i.registry.interner.name(*s).to_string(),
        Key::Name(s, _) => s.clone(),
    }
}

/// A node by the name the program used, through its spellings too.
fn find<'a>(i: &Interp, p: &'a Profile, what: &str) -> Option<(usize, &'a Node)> {
    p.nodes().iter().enumerate().find(|(_, n)| {
        name(i, n) == what
            || n.spelled
                .iter()
                .any(|s| i.registry.interner.name(*s) == what)
    })
}

/// The edge `a → b` summed over every road it took, or `None`.
fn edge(p: &Profile, a: usize, b: usize) -> Option<(u64, Duration)> {
    let mut found = None;
    for (x, y, _, e) in p.edges() {
        if x == a && y == b {
            let (c, t) = found.unwrap_or((0, Duration::ZERO));
            found = Some((c + e.calls, t + e.time));
        }
    }
    found
}

/// The roads `a → b` took: `(via, calls)` pairs.
fn roads(p: &Profile, a: usize, b: usize) -> Vec<(Option<usize>, u64)> {
    let mut v: Vec<(Option<usize>, u64)> = p
        .edges()
        .filter(|(x, y, _, _)| *x == a && *y == b)
        .map(|(_, _, via, e)| (via, e.calls))
        .collect();
    v.sort();
    v
}

fn sum_exclusive(p: &Profile) -> Duration {
    p.nodes().iter().map(|n| n.exclusive).sum()
}

fn close(a: Duration, b: Duration, within: Duration) -> bool {
    a.abs_diff(b) <= within
}

/// **Criterion 5.** One word of kind `word`, the natives it reached, the
/// edge with its count, and the inclusive order.
#[test]
fn the_graph_is_the_graph() {
    let (i, p, open, outcome) = profiled(":inc { 1 + } register 5 inc inc");
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(open, 0, "the mirror is empty when the run ends");
    let words: Vec<String> = p
        .nodes()
        .iter()
        .filter(|n| n.kind == Kind::Word)
        .map(|n| name(&i, n))
        .collect();
    assert_eq!(words, vec!["inc".to_string()]);
    let (inc_id, inc) = find(&i, &p, "inc").unwrap();
    let (plus_id, plus) = find(&i, &p, "+").unwrap();
    let (reg_id, reg) = find(&i, &p, "register").unwrap();
    assert_eq!(plus.kind, Kind::Native);
    assert_eq!(reg.kind, Kind::Native);
    assert_eq!(inc.calls, 2);
    assert_eq!(plus.calls, 2);
    assert_eq!(edge(&p, inc_id, plus_id).map(|e| e.0), Some(2));
    // `register` was called from the top level, not from `inc`.
    assert_eq!(edge(&p, inc_id, reg_id), None);
    assert_eq!(edge(&p, 0, reg_id).map(|e| e.0), Some(1));
    assert_eq!(edge(&p, 0, inc_id).map(|e| e.0), Some(2));
    // The only natives `inc` reaches are `+`.
    let callees: Vec<usize> = p
        .edges()
        .filter(|(a, _, _, _)| *a == inc_id)
        .map(|(_, b, _, _)| b)
        .collect();
    assert_eq!(callees, vec![plus_id]);
    assert!(inc.inclusive >= plus.inclusive);
    // §S2's identity for a word whose only child is `+`.
    assert!(
        close(
            inc.exclusive + plus.inclusive,
            inc.inclusive,
            Duration::from_micros(1)
        ),
        "{:?} + {:?} != {:?}",
        inc.exclusive,
        plus.inclusive,
        inc.inclusive
    );
    assert!(close(
        sum_exclusive(&p),
        p.wall.unwrap(),
        Duration::from_micros(1)
    ));
}

/// **Criterion 6.** A word that calls itself to depth 21 through `if` —
/// `dup 20 >` asks whether 20 is greater than the counter, operands reversed:
/// exclusive times still sum to the wall, its inclusive time is its outermost
/// activation's and not twenty-one of them, and the recursion is counted.
/// `if` files its body as a tail request, so the body's calls are `r`'s own,
/// `via if`, and the recursion is a self edge.
#[test]
fn recursion_adds_up() {
    let (i, p, open, outcome) = profiled(":r { dup 20 > { 1 + r } if } register 0 r");
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(open, 0);
    let wall = p.wall.unwrap();
    assert!(
        close(sum_exclusive(&p), wall, Duration::from_micros(1)),
        "{:?} vs {wall:?}",
        sum_exclusive(&p)
    );
    let (r_id, r) = find(&i, &p, "r").unwrap();
    let (if_id, iff) = find(&i, &p, "if").unwrap();
    let (plus_id, plus) = find(&i, &p, "+").unwrap();
    assert_eq!(r.calls, 21);
    assert_eq!(r.max_depth, 21);
    assert_eq!(plus.calls, 20);
    assert!(r.inclusive <= wall);
    assert!(r.inclusive >= r.exclusive);
    // The body `if` filed runs after `if` returned, as `r`'s own, via `if`:
    // the recursion is a self edge with a count and no time.
    assert_eq!(roads(&p, r_id, r_id), vec![(Some(if_id), 20)]);
    assert_eq!(edge(&p, r_id, r_id), Some((20, Duration::ZERO)));
    assert_eq!(roads(&p, r_id, plus_id), vec![(Some(if_id), 20)]);
    assert_eq!(edge(&p, if_id, r_id), None);
    assert_eq!(edge(&p, 0, r_id).map(|e| e.0), Some(1));
    // `if` itself is a leaf called 21 times, directly from `r`'s body.
    assert_eq!(iff.calls, 21);
    assert_eq!(roads(&p, r_id, if_id), vec![(None, 21)]);
    assert!(iff.inclusive <= r.inclusive);
    // The root's edge to `r` carries the one outermost span.
    let (_, t) = edge(&p, 0, r_id).unwrap();
    assert!(close(t, r.inclusive, Duration::from_micros(1)));
}

/// A word that calls itself directly carries a self edge with a count and no
/// time.
#[test]
fn a_self_edge_carries_the_count_alone() {
    let (i, p, open, outcome) = profiled(":w { 1 + dup 10 < { 7 bund.exit } if w } register 0 w");
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(i.exit_requested(), Some(7), "bund.exit recorded its code");
    assert_eq!(
        open, 0,
        "the exit unwound every frame and the mirror followed"
    );
    let (w_id, w) = find(&i, &p, "w").unwrap();
    assert_eq!(w.calls, 11);
    assert_eq!(w.max_depth, 11);
    assert_eq!(roads(&p, w_id, w_id), vec![(None, 10)]);
    assert_eq!(edge(&p, w_id, w_id), Some((10, Duration::ZERO)));
    assert!(close(
        sum_exclusive(&p),
        p.wall.unwrap(),
        Duration::from_micros(1)
    ));
}

/// **Criterion 8.** A failure forty words deep: the record is complete, the
/// failing word is present, and nothing is left open.
#[test]
fn a_failing_program_leaves_a_closed_record() {
    let (i, p, open, outcome) = profiled(":f { dup 40 > { 1 + f } if boom } register 0 f");
    assert!(outcome.is_err());
    assert_eq!(
        open, 0,
        "unwind_to popped forty-one frames and the mirror followed"
    );
    let (_, f) = find(&i, &p, "f").unwrap();
    assert_eq!(f.calls, 41);
    assert_eq!(f.max_depth, 41);
    assert!(close(
        sum_exclusive(&p),
        p.wall.unwrap(),
        Duration::from_micros(1)
    ));
    for n in p.nodes() {
        assert!(
            n.inclusive <= p.wall.unwrap(),
            "{} overshoots the wall",
            name(&i, n)
        );
    }
}

/// `!` on a literal lambda: `execute` is a node, spelled `!`, and what the
/// lambda calls is a sibling of it under the nearest named node, `via !`,
/// because `!` files the body as a tail request. Under `times`, which runs
/// its body through `eval_lambda`, that nearest node is `times`.
#[test]
fn a_bang_is_a_node_and_its_body_hangs_by_the_path_it_took() {
    let (i, p, open, outcome) = profiled(":w { { 1 + } ! } register 0 w 2 { { 1 + } ! } times");
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(open, 0);
    let (exec_id, exec) = find(&i, &p, "!").unwrap();
    assert_eq!(name(&i, exec), "execute");
    assert_eq!(exec.calls, 3);
    let (w_id, _) = find(&i, &p, "w").unwrap();
    let (plus_id, _) = find(&i, &p, "+").unwrap();
    let (times_id, _) = find(&i, &p, "times").unwrap();
    // From `w`: `execute` once, directly, and `+` once as a sibling, via `!`.
    assert_eq!(roads(&p, w_id, exec_id), vec![(None, 1)]);
    assert_eq!(roads(&p, w_id, plus_id), vec![(Some(exec_id), 1)]);
    // Under `times`: `execute` twice, directly, and `+` twice via `!`; the
    // body never hangs under `execute` itself.
    assert_eq!(roads(&p, times_id, exec_id), vec![(None, 2)]);
    assert_eq!(roads(&p, times_id, plus_id), vec![(Some(exec_id), 2)]);
    assert_eq!(edge(&p, exec_id, plus_id), None);
}

/// A method native and a conditional runner are nodes of their own kinds,
/// reached through the `Vm` seam.
#[test]
fn methods_and_runners_are_nodes() {
    let (_, p, open, outcome) = profiled(":str 42 :Value object ! drop ? :run { 1 } set !");
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(open, 0);
    let kinds: Vec<Kind> = p.nodes().iter().map(|n| n.kind).collect();
    assert!(kinds.contains(&Kind::Method), "{kinds:?}");
    assert!(kinds.contains(&Kind::Conditional), "{kinds:?}");
}

/// A word registered twice is one node and is named as redefined.
#[test]
fn a_redefinition_is_named() {
    let (i, p, _, outcome) = profiled(":w { 1 } register w :w { 2 } register w");
    assert!(outcome.is_ok(), "{outcome:?}");
    let (_, w) = find(&i, &p, "w").unwrap();
    assert_eq!(w.calls, 2);
    assert!(p.redefined.contains("w"));
}

/// **D188: steps are exact and add up.** A step is one value `run_to`
/// dispatched. The top-level stream of `:inc { 1 + } register 5 inc inc` is
/// six values, each the root's own; `inc`'s body is two, `1` and `+`, so two
/// calls are four steps, all `inc`'s own because `+` dispatches nothing; and
/// the total is the sum. The same program counts the same on every host,
/// which no clock figure in this file can claim.
#[test]
fn steps_are_exact_and_add_up() {
    let (i, p, _, outcome) = profiled(":inc { 1 + } register 5 inc inc");
    assert!(outcome.is_ok(), "{outcome:?}");
    let root = &p.nodes()[0];
    let (inc_id, inc) = find(&i, &p, "inc").unwrap();
    let (plus_id, plus) = find(&i, &p, "+").unwrap();
    assert_eq!(p.steps(), 10);
    assert_eq!(root.steps_inclusive, 10);
    assert_eq!(root.steps, 6);
    assert_eq!(inc.steps, 4);
    assert_eq!(inc.steps_inclusive, 4);
    assert_eq!(plus.steps, 0);
    assert_eq!(plus.steps_inclusive, 0);
    let sum: u64 = p.nodes().iter().map(|n| n.steps).sum();
    assert_eq!(sum, p.steps(), "exclusive steps sum to the total");
    // The edges carry the callee's steps as they carry its time.
    let inc_edge = p
        .edges()
        .find(|(a, b, _, _)| *a == 0 && *b == inc_id)
        .map(|(_, _, _, e)| e.steps);
    assert_eq!(inc_edge, Some(4));
    let plus_edge = p
        .edges()
        .find(|(a, b, _, _)| *a == inc_id && *b == plus_id)
        .map(|(_, _, _, e)| e.steps);
    assert_eq!(plus_edge, Some(0));
    // Nothing failed, so no word is named as failing.
    assert_eq!(p.failing(), None);
}

/// **D188: a recursive word's steps follow the inclusive rule.** `r` recurses
/// to depth 21 through `if`: its inclusive steps are the outermost
/// activation's, which hold every step of the run but the root's own, and
/// its outermost activations number one where its calls number twenty-one.
#[test]
fn a_recursive_word_counts_its_outermost_steps_once() {
    let (i, p, _, outcome) = profiled(":r { dup 20 > { 1 + r } if } register 0 r");
    assert!(outcome.is_ok(), "{outcome:?}");
    let root = &p.nodes()[0];
    let (_, r) = find(&i, &p, "r").unwrap();
    assert_eq!(r.calls, 21);
    assert_eq!(r.outer, 1, "one outermost activation");
    assert_eq!(
        root.steps, 5,
        "`:r`, the lambda, `register`, `0` and `r`: the step that names a word is its caller's"
    );
    assert_eq!(r.steps_inclusive, p.steps() - root.steps);
    // Every step but the root's is `r`'s own: `if` and `+` dispatch nothing,
    // and the bodies `if` files run as `r`'s own via `if`.
    assert_eq!(r.steps, r.steps_inclusive);
    // `min` and `max` are of the one outermost span, so they agree.
    assert_eq!(r.min, r.max);
    assert_eq!(r.max, r.inclusive);
}

/// **D188: the report can say where the run ended.** A native that answers
/// an error is the failing node; so is the word whose body named something
/// that could not be dispatched; a failure `?try` caught and a later one
/// that ended the run leave the later one marked.
#[test]
fn the_failing_word_is_marked() {
    let (i, p, _, outcome) = profiled(":g { + } register g");
    assert!(outcome.is_err());
    let (plus_id, _) = find(&i, &p, "+").unwrap();
    assert_eq!(
        p.failing(),
        Some(plus_id),
        "the native that answered the error"
    );

    let (i, p, _, outcome) = profiled(":f { boom } register f");
    assert!(outcome.is_err());
    let (f_id, _) = find(&i, &p, "f").unwrap();
    assert_eq!(p.failing(), Some(f_id), "the word whose body named `boom`");

    let (i, p, _, outcome) = profiled(":g { + } register { g } ?try :h { boom } register h");
    assert!(outcome.is_err());
    let (h_id, _) = find(&i, &p, "h").unwrap();
    assert_eq!(
        p.failing(),
        Some(h_id),
        "the failure that ended the run, not the one caught"
    );
}
