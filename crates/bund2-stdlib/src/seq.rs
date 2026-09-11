//! Sequence generation and counted iteration — `seq` and `times`.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LAMBDA, LIST, PAIR};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// `conf_get` — a key from a config dict, or a default
/// (`reference/Bund/src/stdlib/helpers/conf.rs:5-22`).
///
/// A missing key, and a receiver that is not a dict at all, both give the
/// default. The reference logs the second case and carries on; there is no
/// error path.
fn conf_get(conf: &BundValue, key: &str) -> Option<BundValue> {
    conf.get(key)
}

fn conf_f64(conf: &BundValue, key: &str, default: f64) -> f64 {
    match conf_get(conf, key) {
        Some(v) => match v.unboxed() {
            BundValue::Float(f, _) => *f,
            BundValue::Int(i, _) => *i as f64,
            _ => default,
        },
        None => default,
    }
}

fn conf_i64(conf: &BundValue, key: &str, default: i64) -> i64 {
    match conf_get(conf, key) {
        Some(v) => match v.unboxed() {
            BundValue::Int(i, _) => *i,
            BundValue::Float(f, _) => *f as i64,
            _ => default,
        },
        None => default,
    }
}

/// `seq` — a LIST of floats described by a config dict
/// (`reference/Bund/src/stdlib/functions/math/seq.rs:53-70`).
///
/// Four keys, each with a default the reference hardcodes: `type`
/// (`"seq.ascending"`), `X` (`0.0`), `Step` (`1.0`) and `N` (`128`). The
/// elements are **always floats**, whatever the config holds — `seq_ascending`
/// and its siblings push `Value::from_float` unconditionally (`:23-25`).
///
/// Confirmed against the oracle:
///
/// ```text
/// config :N 5 set :X 1.0 set :Step 2.0 set seq   ->  [1.0, 3.0, 5.0, 7.0, 9.0]
/// config :N 3 set :type "seq.descending" set …   ->  [10.0, 7.5, 5.0]
/// config :N 3 set :type "single" set :X 7.0 set  ->  [7.0, 7.0, 7.0]
/// config :N 3 set                                ->  [0.0, 1.0, 2.0]
/// ```
fn seq(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(conf) = vm.pull() else {
        return Err(Error("SEQ_OP returns: NO DATA #1".into()));
    };
    let kind = conf_get(&conf, "type")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| "seq.ascending".to_string());
    let x = conf_f64(&conf, "X", 0.0);
    let step = conf_f64(&conf, "Step", 1.0);
    let n = conf_i64(&conf, "N", 128).max(0) as usize;

    let items: Vec<BundValue> = match kind.as_str() {
        "seq.ascending" => (0..n)
            .map(|i| BundValue::float(x + step * i as f64))
            .collect(),
        "seq.descending" => (0..n)
            .map(|i| BundValue::float(x - step * i as f64))
            .collect(),
        "single" => (0..n).map(|_| BundValue::float(x)).collect(),
        other => return Err(Error(format!("Unknown SEQ type: {other}"))),
    };
    vm.push(BundValue::list(items));
    Ok(())
}

/// `times` — run a lambda `n` times, **pushing the counter each round**
/// (`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:7-45`).
///
/// The counter runs `0..n`, so the body sees `0` first and `n-1` last, and a
/// body that does not consume it leaves `n` values behind. A non-positive `n`
/// runs the body zero times.
///
/// The lambda is on top — pushed last — so `n { … } times` reads in order.
fn times(vm: &mut dyn Vm) -> Result<(), Error> {
    times_base(vm, crate::wb::Side::Stack)
}

/// `times.` — the **count** is on the workbench, the lambda still on the stack
/// (`reference/rust_multistackvm/src/stdlib/logic/times_fun.rs:49-56`).
///
/// The counter it pushes each iteration still goes to the stack (`:61`), so a
/// body written for `times` works unchanged; only the count moved.
fn times_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    times_base(vm, crate::wb::Side::Bench)
}

fn times_base(vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    if vm.depth() < if side == crate::wb::Side::Stack { 2 } else { 1 } {
        return Err(Error("Stack is too shallow for inline times".into()));
    }
    let lambda_val = crate::pull::operand(vm, "TIMES", 1)?;
    if lambda_val.dt() != LAMBDA {
        return Err(Error("TIMES: #1 parameter must be lambda".into()));
    }
    let n_val = side
        .pull(vm)
        .ok_or_else(|| Error("TIMES returns: NO DATA #2".into()))?;
    let n = n_val
        .as_int()
        .ok_or_else(|| Error("TIMES returns error: operand is not an integer".into()))?;
    let body = lambda_val.clone();
    for v in 0..n {
        vm.push(BundValue::int(v));
        vm.eval_lambda(&body)
            .map_err(|e| e.context("TIMES: lambda execution returns error: "))?;
    }
    Ok(())
}

/// `loop` — evaluate a lambda once per element of a list, pushing the element
/// (`reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:6-48`).
///
/// The same operand shape as `times`: the lambda is on top (`:10`) and the
/// thing being iterated is below it (`:13`). The element is pushed *before*
/// each evaluation (`:22`), so the body reads it off the stack exactly as
/// `times` reads its counter.
///
/// **It accepts a PAIR as well as a LIST**, because it goes through
/// `cast_list`, which admits both (`reference/rust_dynamic/src/cast.rs:58`).
/// That is worth spelling out: a PAIR is not a two-element list in the tag
/// system — it has its own `dt` — and iterating one is reachable from Bund.
///
/// Nothing is pushed after the loop: the list is consumed and only whatever
/// the body left remains.
fn loop_word(vm: &mut dyn Vm) -> Result<(), Error> {
    loop_base(vm, crate::wb::Side::Stack)
}

/// `loop.` — the **list** is on the workbench, the lambda on the stack
/// (`reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:6-16,107`).
///
/// Note this is `stdlib_logic_loop_base`, which walks a LIST. The word that
/// walks the current stack until NODATA is `*loop`, a different registration
/// (`:49,111`) — and `*loop.` is F78's second instance: it passes `FromStack`
/// like its sibling does not.
fn loop_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    loop_base(vm, crate::wb::Side::Bench)
}

/// `*loop` — run a lambda over the current stack until it is empty or reaches
/// a NODATA, which it consumes
/// (`reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:51-101`).
///
/// The lambda runs with the stack as it finds it; it is the lambda's job to
/// consume. One that consumes nothing spins in the reference, and Bund2 cannot
/// stop it without changing what the program means. So, as `while` does, it
/// reports once at ten million turns (D39).
fn loop_over(vm: &mut dyn Vm) -> Result<(), Error> {
    loop_over_base(vm, "*LOOP")
}

/// `*loop.` — **the lambda still comes off the stack.** F78: the reference's
/// `stdlib_logic_loop_over_workbench` passes `StackOps::FromStack`
/// (`loop_fun.rs:115-117`), so the word is `*loop` with a different prefix.
fn loop_over_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    loop_over_base(vm, "*LOOP.")
}

fn loop_over_base(vm: &mut dyn Vm, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}()")));
    }
    let lambda_val = vm
        .pull()
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #1")))?;
    if lambda_val.dt() != LAMBDA {
        return Err(Error(format!("{prefix}: #1 parameter must be lambda")));
    }
    const CHATTER_AT: u64 = 10_000_000;
    let mut turns: u64 = 0;
    loop {
        match vm.peek() {
            None => return Ok(()),
            Some(v) if v.dt() == bund2_value::NODATA => {
                vm.pull();
                return Ok(());
            }
            Some(_) => {}
        }
        turns += 1;
        if turns == CHATTER_AT {
            vm.report(bund2_api::diag::Diagnostic::warning(format!(
                "`{}` has run {CHATTER_AT} iterations. It stops when the stack is empty or reaches NODATA, so its lambda must consume what it is handed.",
                prefix.to_lowercase()
            )));
        }
        vm.eval_lambda(&lambda_val)
            .map_err(|e| e.context(format!("{prefix}: lambda execution returns error: ")))?;
    }
}

fn loop_base(vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    if vm.depth() < if side == crate::wb::Side::Stack { 2 } else { 1 } {
        return Err(Error("Stack is too shallow for inline LOOP".into()));
    }
    let lambda_val = crate::pull::operand(vm, "LOOP", 1)?;
    if lambda_val.dt() != LAMBDA {
        return Err(Error("LOOP: #1 parameter must be lambda".into()));
    }
    let seq_val = side
        .pull(vm)
        .ok_or_else(|| Error("LOOP returns: NO DATA #2".into()))?;
    let dt = seq_val.dt();
    if dt != LIST && dt != PAIR {
        return Err(Error(format!(
            "LOOP returns error: This is not a LIST/PAIR value but {dt}"
        )));
    }
    let items = seq_val
        .as_list()
        .ok_or_else(|| Error("LOOP returns error: This Dynamic type is not list".into()))?
        .to_vec();
    let body = lambda_val.clone();
    for v in items {
        vm.push(v);
        vm.eval_lambda(&body)
            .map_err(|e| e.context("LOOP: lambda execution returns error: "))?;
    }
    Ok(())
}

/// `map` — evaluate a lambda per element and collect what it leaves
/// (`reference/rust_multistackvm/src/stdlib/logic/map_fun.rs:6-107`).
///
/// Same operand shape as `loop`, and the same push-then-evaluate step — the
/// difference is that `map` **pulls the result back** after each evaluation
/// (`:29`) and collects it, then pushes one list (`:43`). A body that leaves
/// nothing is `MAP can not obtain MAP outcome from stack` (`:34`), not a
/// silently short list.
///
/// **It does not accept a PAIR, where `loop` does.** `map` branches on
/// `type_of()` and only admits `LIST` and `MATRIX` (`:21,51`), while `loop`
/// goes through `cast_list`, which admits `LIST | PAIR`
/// (`reference/rust_dynamic/src/cast.rs:58`). Two neighbouring words over the
/// same shape with different type gates; both are preserved as written.
///
/// The MATRIX arm is not implemented because Bund2 constructs no MATRIX. It
/// reports the reference's own text for an unsupported container, which names
/// the type — the same `type_name()` table `type` uses.
fn map_word(vm: &mut dyn Vm) -> Result<(), Error> {
    map_base(vm, crate::wb::Side::Stack)
}

/// `map.` — list off the workbench, lambda off the stack, **result back to the
/// workbench** (`reference/rust_multistackvm/src/stdlib/logic/map_fun.rs:14-15,40-41`).
///
/// The per-item values the body sees are still pushed to and pulled from the
/// stack (`:22,25`); only the collection and the answer live on the workbench.
fn map_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    map_base(vm, crate::wb::Side::Bench)
}

fn map_base(vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    if vm.depth() < if side == crate::wb::Side::Stack { 2 } else { 1 } {
        return Err(Error("Stack is too shallow for inline MAP".into()));
    }
    let lambda_val = crate::pull::operand(vm, "MAP", 1)?;
    if lambda_val.dt() != LAMBDA {
        return Err(Error("MAP: #1 parameter must be lambda".into()));
    }
    let seq_val = side
        .pull(vm)
        .ok_or_else(|| Error("MAP returns: NO DATA #2".into()))?;
    if seq_val.dt() != LIST {
        return Err(Error(format!(
            "MAP: can not run map over {}",
            seq_val.type_name()
        )));
    }
    let items = seq_val
        .as_list()
        .ok_or_else(|| Error("MAP returns error: This Dynamic type is not list".into()))?
        .to_vec();
    let body = lambda_val.clone();
    let mut out: Vec<BundValue> = Vec::with_capacity(items.len());
    for v in items {
        vm.push(v);
        vm.eval_lambda(&body)
            .map_err(|e| e.context("MAP: lambda execution returns error: "))?;
        let outcome = vm
            .pull()
            .ok_or_else(|| Error("MAP can not obtain MAP outcome from stack".into()))?;
        out.push(outcome);
    }
    side.push(vm, BundValue::list(out));
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.register_native("seq", seq, eff(1, 1), WordKind::Sync);
    // Opaque for the same reason as `if`: each evaluates a lambda per
    // element, so what is left is the body's business. The floors stand.
    r.register_native("times", times, StackEffect::opaque(2), WordKind::Sync);
    r.register_native("loop", loop_word, StackEffect::opaque(2), WordKind::Sync);
    // `reference/rust_multistackvm/src/stdlib/logic/loop_fun.rs:122-123`.
    // Opaque: each runs a body until the stack is empty or a NODATA.
    r.register_native("*loop", loop_over, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("*loop.", loop_over_wb, StackEffect::opaque(1), WordKind::Sync);
    // Each `.` sibling keeps the lambda as a stack operand and moves only its
    // other one, so each consumes one from the stack rather than none.
    r.register_native("times.", times_wb, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("loop.", loop_wb, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("map", map_word, StackEffect::opaque(2), WordKind::Sync);
    r.register_native("map.", map_wb, StackEffect::opaque(1), WordKind::Sync);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_interp::Interp;

    fn run_src(src: &str) -> Result<Interp, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        Ok(i)
    }

    fn floats(src: &str) -> Vec<f64> {
        let i = run_src(src).expect("runs");
        let v = i.peek().expect("a value");
        v.as_list()
            .expect("a list")
            .iter()
            .filter_map(|e| match e.unboxed() {
                BundValue::Float(f, _) => Some(*f),
                _ => None,
            })
            .collect()
    }

    /// All four confirmed against the oracle.
    #[test]
    fn seq_generates_what_the_oracle_generates() {
        assert_eq!(
            floats("config :N 5 set :X 1.0 set :Step 2.0 set seq"),
            vec![1.0, 3.0, 5.0, 7.0, 9.0]
        );
        assert_eq!(
            floats("config :N 3 set :type \"seq.descending\" set :X 10.0 set :Step 2.5 set seq"),
            vec![10.0, 7.5, 5.0]
        );
        assert_eq!(
            floats("config :N 3 set :type \"single\" set :X 7.0 set seq"),
            vec![7.0, 7.0, 7.0]
        );
    }

    /// The defaults are the reference's hardcoded ones: X 0.0, Step 1.0,
    /// type ascending. (N defaults to 128 and is not exercised here.)
    #[test]
    fn seq_defaults_every_key() {
        assert_eq!(floats("config :N 3 set seq"), vec![0.0, 1.0, 2.0]);
    }

    /// Elements are floats even when the config holds integers.
    #[test]
    fn seq_always_yields_floats() {
        let i = run_src("config :N 2 set :X 1 set :Step 1 set seq").expect("runs");
        let v = i.peek().expect("a value");
        for e in v.as_list().expect("a list") {
            assert_eq!(e.dt(), bund2_value::FLOAT, "{:?}", e.summary(40));
        }
    }

    #[test]
    fn an_unknown_seq_type_is_named() {
        let e = match run_src("config :type \"nope\" set seq") {
            Ok(_) => panic!("expected a failure"),
            Err(e) => e,
        };
        assert!(e.contains("Unknown SEQ type: nope"), "{e}");
    }

    /// The counter is pushed each round, so a body that ignores it leaves `n`
    /// values behind.
    /// The body is `0 drop`, not `{ }`: an empty lambda does not parse (F51),
    /// which this test discovered by trying.
    /// `loop` pushes each element before evaluating, so the body reads it off
    /// the stack exactly as `times` reads its counter. Confirmed against the
    /// oracle for a list of integers and a list of strings.
    #[test]
    fn loop_pushes_each_element() {
        let i = run_src("[ 1 2 3 ] { drop } loop").expect("runs");
        assert_eq!(i.depth(), 0, "each element consumed by the body");
        let i = run_src("[ 1 2 3 ] { } loop");
        // An empty lambda is F51's parse error, so the body must do something.
        assert!(i.is_err(), "an empty block is a parse error");
    }

    /// All three error texts confirmed against the oracle's *inner* reason —
    /// the reference wraps them in `Attempt to evaluate value … returned
    /// error:`, which D36 drops in favour of a real Bund location.
    #[test]
    fn loop_reports_the_references_reasons() {
        for (src, want) in [
            (
                "\"nope\" { drop } loop",
                "LOOP returns error: This is not a LIST/PAIR value but 4",
            ),
            ("[ 1 ] 5 loop", "LOOP: #1 parameter must be lambda"),
            ("{ drop } loop", "Stack is too shallow for inline LOOP"),
        ] {
            match run_src(src) {
                Ok(_) => panic!("{src} was expected to fail"),
                Err(e) => assert!(e.contains(want), "{src}: got {e}"),
            }
        }
    }

    #[test]
    fn times_pushes_the_counter() {
        let i = run_src("3 { 0 drop } times").expect("runs");
        assert_eq!(i.depth(), 3);
        assert_eq!(i.snapshot()[0].as_int(), Some(0), "counts from zero");
        assert_eq!(i.snapshot()[2].as_int(), Some(2), "and stops at n-1");
    }

    #[test]
    fn times_runs_the_body() {
        let i = run_src("3 { drop 1 } times").expect("runs");
        assert_eq!(i.depth(), 3);
        assert!(i.snapshot().iter().all(|v| v.as_int() == Some(1)));
    }

    #[test]
    fn times_with_a_non_positive_count_runs_nothing() {
        assert_eq!(run_src("0 { 9 } times").expect("runs").depth(), 0);
        assert_eq!(run_src("-2 { 9 } times").expect("runs").depth(), 0);
    }

    #[test]
    fn times_requires_a_lambda() {
        let e = match run_src("3 4 times") {
            Ok(_) => panic!("expected a failure"),
            Err(e) => e,
        };
        assert!(e.contains("TIMES: #1 parameter must be lambda"), "{e}");
    }
}

#[cfg(test)]
mod entry_key_tests {
    use bund2_interp::Interp;

    fn entries(src: &str) -> Vec<usize> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i.entry_log = Some(Vec::new());
        let stream = bund2_syntax::compile(src).expect("compiles");
        i.eval(&stream).expect("runs");
        i.entry_log.take().unwrap_or_default()
    }

    /// **RFC-0005 criterion 20's precondition, and D42's point.** A body run
    /// by `times` reaches its entry point under **one** key on every
    /// iteration. Before D42 `times` copied the body out of its `Rc` once per
    /// call and ran a slice, so there was no key to observe at all.
    #[test]
    fn times_enters_one_body_under_one_key() {
        let log = entries("100 { drop } times");
        assert_eq!(log.len(), 100, "one entry per iteration");
        assert!(
            log.iter().all(|k| Some(k) == log.first()),
            "every iteration entered the same body"
        );
    }

    /// A registered lambda called twice enters under one key both times — the
    /// named-lambda path always held its `Rc`, and still does.
    #[test]
    fn a_named_lambda_enters_under_one_key() {
        let log = entries(":f { 1 drop } register f f");
        assert_eq!(log.len(), 2);
        assert_eq!(log.first(), log.get(1));
    }
}

#[cfg(test)]
mod stack_floor_tests {
    use bund2_interp::Interp;

    /// **F85, fixed.** Recursion through `times` spends a Rust frame per
    /// level and used to abort the process on the machine stack, as the
    /// reference still does. It now reports a Bund-level error — and the error
    /// was not re-wrapped at every level on the way out, or it would be
    /// megabytes long.
    #[test]
    fn recursion_through_times_reports_instead_of_aborting() {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream =
            bund2_syntax::compile(":f { 1 { drop f } times } register f").expect("compiles");
        let e = i.eval(&stream).expect_err("must report, not abort");
        assert!(
            e.0.contains("machine stack exhausted"),
            "{}",
            &e.0[..e.0.len().min(300)]
        );
        assert!(
            e.0.len() < 4096,
            "the error was re-wrapped at every level: {} bytes",
            e.0.len()
        );
    }
}
