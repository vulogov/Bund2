//! Literal control flow and string templating — `if`, `ifthenelse`, `format`.
//!
//! **These are the statically analysable half.** RFC-0003's alternatives
//! section distinguishes them from the `?`-family: `if`, `times`, `while` and
//! `loop` take their branches as lambda *arguments* and can be lowered as
//! control flow, while `?ifthenelse` and `?try` assemble a CONDITIONAL at
//! runtime and dispatch it through a string lookup. Only the first kind is
//! here; the second needs the `CF` registry S7 describes.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LAMBDA};

/// Pull the lambda, then the condition, then run the branch when the condition
/// matches (`reference/rust_multistackvm/src/stdlib/logic/if_fun.rs:12-89`).
///
/// The lambda comes off **first** — it is on top, pushed last — so `cond { … }
/// if` reads in source order even though the pulls are reversed.
///
/// A non-LAMBDA first operand is an error (`:79-81`). The reference has an
/// unreachable `execute` fallback inside the taken branch (`:60-63`), guarded
/// out by the type check above it.
/// **The lambda always comes off the stack.** Only the *condition* moves to
/// the workbench in the `.` form
/// (`reference/rust_multistackvm/src/stdlib/logic/if_fun.rs:27,31-34`) — the
/// rule the whole logic family follows, and the reason `if.` is not "`if` on
/// the workbench" but "`if` whose condition is on the workbench".
fn if_base(vm: &mut dyn Vm, side: crate::wb::Side, want: bool, prefix: &str) -> Result<(), Error> {
    // `:14-25`: the stack form wants two; the workbench form wants one on each.
    if vm.depth() < if side == crate::wb::Side::Stack { 2 } else { 1 } {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    if side == crate::wb::Side::Bench && vm.workbench_depth() < 1 {
        return Err(Error(format!("Workbench is too shallow for inline {prefix}")));
    }
    let lambda_val = crate::pull::operand(vm, prefix, 1)?;
    if lambda_val.dt() != LAMBDA {
        return Err(Error(format!("{prefix}: #1 parameter must be lambda")));
    }
    let cond_val = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #2")))?;
    let cond = cast_bool(&cond_val)
        .ok_or_else(|| Error(format!("{prefix} returns error: can not cast to bool")))?;
    if cond == want {
        let body = lambda_val.clone();
        // Tail position: nothing here runs after the branch, so the loop takes
        // it (§S4a). This is what keeps a self-recursive word through `?true`
        // from adding a Rust frame per iteration.
        vm.tail_lambda(body);
    }
    Ok(())
}

/// `cast_bool`, for the kinds a condition can be.
///
/// A BOOL is itself; the numeric kinds are false at zero. `==` and friends
/// push a BOOL, so that is the path the corpus takes.
pub(crate) fn cast_bool(v: &BundValue) -> Option<bool> {
    match v.unboxed() {
        BundValue::Bool(b, _) => Some(*b),
        BundValue::Int(i, _) => Some(*i != 0),
        BundValue::Float(f, _) => Some(*f != 0.0),
        _ => None,
    }
}

fn if_true(vm: &mut dyn Vm) -> Result<(), Error> {
    if_base(vm, crate::wb::Side::Stack, true, "IF")
}

fn if_true_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    if_base(vm, crate::wb::Side::Bench, true, "IF")
}

fn if_false(vm: &mut dyn Vm) -> Result<(), Error> {
    if_base(vm, crate::wb::Side::Stack, false, "?FALSE")
}

/// `if.false.in_workbench`, spelled `?false.` — **and it does not read the
/// workbench.** F78.
///
/// Its three siblings pass `StackOps::FromWorkBench`; this one passes
/// `FromStack` (`reference/rust_multistackvm/src/stdlib/logic/if_fun.rs:103-105`
/// against `:95-96`). So the word is `?false` with a different error prefix,
/// and a program that puts its condition on the workbench gets
/// `Stack is too shallow`. Reproduced deliberately: `Side::Stack` here is the
/// bug, kept.
fn if_false_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    if_base(vm, crate::wb::Side::Stack, false, "?FALSE.")
}

/// `ifthenelse`, spelled `?true*` — two lambdas and a condition
/// (`reference/rust_multistackvm/src/stdlib/logic/ifthenelse_fun.rs`).
///
/// **The branch on top is `then`, not `else`** — so `cond { else } { then }
/// ?true*` in source order. The reference pulls into `then_lambda` first
/// (`ifthenelse_fun.rs:28`) and `else_lambda` second (`:38`), which is the
/// same top-is-first-operand rule the comparisons and arithmetic follow, and
/// it reads backwards for exactly the same reason.
///
/// An earlier version of this word named them the other way round, so
/// `tests/testing_ifthenelse.bund` — `42 42 != { true } { false } ?true*` —
/// answered `false` where the oracle answers `true`. The unit tests did not
/// catch it because they were written from the same wrong reading; the golden
/// did.
fn ifthenelse(vm: &mut dyn Vm) -> Result<(), Error> {
    ifthenelse_base(vm, crate::wb::Side::Stack, "IFTHENELSE")
}

/// `ifthenelse.` — **both lambdas still come off the stack**; only the
/// condition is on the workbench
/// (`reference/rust_multistackvm/src/stdlib/logic/ifthenelse_fun.rs:19-25,47-50`).
fn ifthenelse_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    ifthenelse_base(vm, crate::wb::Side::Bench, "IFTHENELSE")
}

/// `notifthenelse`, spelled `?false*` — **and it does not negate. F97.**
///
/// The reference routes it through the same base as `ifthenelse` with
/// `TypeCond::IfFalse` (`reference/rust_multistackvm/src/stdlib/logic/ifthenelse_fun.rs:92-94`),
/// and that arm reads `if ! cond_bool { else } else { then }` (`:64-70`): the
/// same choice as the `IfTrue` arm, written the other way round. So `true`
/// runs the `then` lambda, exactly as `ifthenelse` does, and the word differs
/// from it only in its error prefix. Confirmed against the oracle, and
/// reproduced, because the goldens capture the oracle.
fn notifthenelse(vm: &mut dyn Vm) -> Result<(), Error> {
    ifthenelse_base(vm, crate::wb::Side::Stack, "NOTIFTHENELSE")
}

/// `notifthenelse.`, spelled `?false*.` — the condition off the workbench, and
/// F97's non-negation with it (`ifthenelse_fun.rs:96-98`).
fn notifthenelse_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    ifthenelse_base(vm, crate::wb::Side::Bench, "NOTIFTHENELSE.")
}

/// `if.stack`, spelled `?stack` — run the lambda when the named stack is the
/// current one (`reference/rust_multistackvm/src/stdlib/logic/if_fun.rs:107-139`).
///
/// The lambda is pulled and type-checked before the name is pulled, so a
/// non-lambda leaves the name on the stack (`:111-131`). The name is compared
/// with the current stack's; any other name runs nothing.
fn if_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline if.stack".into()));
    }
    let lambda_val = crate::pull::operand(vm, "IF.stack", 1)?;
    if lambda_val.dt() != LAMBDA {
        return Err(Error("IF.stack: #1 parameter must be lambda".into()));
    }
    let cond = crate::pull::operand(vm, "IF.stack", 2)?;
    let Some(name) = cond.as_str() else {
        return Err(Error(
            "IF.stack returns error: This Dynamic type is not string".into(),
        ));
    };
    if vm.current_name() == name {
        // Tail position, as `if` is: the loop runs the body.
        vm.tail_lambda(lambda_val);
    }
    Ok(())
}

fn ifthenelse_base(vm: &mut dyn Vm, side: crate::wb::Side, prefix: &str) -> Result<(), Error> {
    if vm.depth() < if side == crate::wb::Side::Stack { 3 } else { 2 } {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    if side == crate::wb::Side::Bench && vm.workbench_depth() < 1 {
        return Err(Error(format!("Workbench is too shallow for inline {prefix}")));
    }
    let then_val = crate::pull::operand(vm, prefix, 1)?;
    let else_val = crate::pull::operand(vm, prefix, 2)?;
    // Numbered from the top, as the reference numbers them (`:33`, `:41`).
    for (v, n) in [(&then_val, 1), (&else_val, 2)] {
        if v.dt() != LAMBDA {
            return Err(Error(format!("{prefix}: #{n} parameter must be lambda")));
        }
    }
    let cond_val = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #3")))?;
    let cond = cast_bool(&cond_val)
        .ok_or_else(|| Error(format!("{prefix} returns error: can not cast to bool")))?;
    let chosen = if cond { then_val } else { else_val };
    let body = chosen.clone();
    vm.tail_lambda(body);
    Ok(())
}

/// `while` — evaluate a lambda while the stack keeps yielding `true`
/// (`reference/rust_multistackvm/src/stdlib/logic/while_fun.rs:5-46`).
///
/// The lambda is pulled once (`:9`); the **condition is pulled afresh on every
/// iteration** (`:13`), so the body has to leave a new one behind or the loop
/// consumes whatever is under it and eventually reports `NO DATA #2` (`:34`).
/// That is the shape of the reference's own loop and not a simplification.
///
/// The condition goes through `cast_bool`, which requires an actual BOOL
/// (`reference/rust_dynamic/src/cast.rs:25-32`) — unlike `not`, which converts
/// first. So `1 { … } while` is `WHILE returns error: This Dynamic type is not
/// bool`, where `1 not` is `false`.
fn while_word(vm: &mut dyn Vm) -> Result<(), Error> {
    while_base(vm, crate::wb::Side::Stack)
}

/// `while.` — lambda off the stack, and the condition re-read from the
/// **workbench** on every iteration
/// (`reference/rust_multistackvm/src/stdlib/logic/while_fun.rs:50-58`).
///
/// So the body must leave a fresh condition *on the workbench*, not on the
/// stack. A body written for `while` will not terminate under `while.`; it will
/// run out of workbench and stop with `NO DATA`.
fn while_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    while_base(vm, crate::wb::Side::Bench)
}

fn while_base(vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    if vm.depth() < if side == crate::wb::Side::Stack { 2 } else { 1 } {
        return Err(Error("Stack is too shallow for inline while()".into()));
    }
    let lambda_val = crate::pull::operand(vm, "WHILE", 1)?;
    if lambda_val.dt() != LAMBDA {
        return Err(Error("WHILE: #1 parameter must be lambda".into()));
    }
    let body = lambda_val.clone();
    // **A loop this long is probably a mistake, and saying so is free.**
    //
    // `while` is the one word whose iteration count the program controls
    // without bound — `loop`, `map` and `times` walk a collection they were
    // handed. A non-terminating `while` cannot be fixed by the interpreter:
    // stopping it would change what the language means, and the reference
    // spins too. But it can be *reported*, which is the difference between a
    // program that appears hung and one that says what it is doing.
    //
    // D36's severity ladder decides the rest. This has not stopped anything,
    // so it is a `Warning`: one line on stderr, no table, no stack dump, and
    // **emitted once** rather than per iteration. The threshold is high enough
    // that no corpus program reaches it — and `conform` folds stderr into the
    // captured output, so if one ever did, the goldens would say so
    // immediately.
    const CHATTER_AT: u64 = 10_000_000;
    let mut turns: u64 = 0;
    loop {
        turns += 1;
        if turns == CHATTER_AT {
            vm.report(bund2_api::diag::Diagnostic::warning(format!(
                "`while` has run {CHATTER_AT} iterations. If its condition never becomes false, nothing will stop it — the body must leave a new condition on the stack each time."
            )));
        }
        // Re-read from the side the word was called on, every iteration
        // (`while_fun.rs:13` and `:58`).
        let cond_val = side
            .pull(vm)
            .ok_or_else(|| Error("WHILE returns: NO DATA #2".into()))?;
        let cond = match cond_val.unboxed() {
            BundValue::Bool(b, _) => *b,
            _ => {
                return Err(Error(
                    "WHILE returns error: This Dynamic type is not bool".into(),
                ));
            }
        };
        if !cond {
            return Ok(());
        }
        vm.eval_lambda(&body)
            .map_err(|e| e.context("WHILE: lambda execution returns error: "))?;
    }
}

/// `for` — run a lambda, then pull a condition, and repeat while it is true
/// (`reference/rust_multistackvm/src/stdlib/logic/for_fun.rs:5-49`).
///
/// The body runs first and leaves the condition, so it runs at least once. The
/// condition goes through `cast_bool`, which admits only a BOOL
/// (`reference/rust_dynamic/src/cast.rs:25-32`). Nothing bounds the loop but
/// the body, so, as `while` does, it reports once at ten million turns (D39).
fn for_word(vm: &mut dyn Vm) -> Result<(), Error> {
    for_base(vm, "FOR", "Stack is too shallow for inline for")
}

/// `for.` — **and it reads nothing from the workbench.** Both the lambda and
/// the condition come off the stack (`for_fun.rs:51-97`), so the word is `for`
/// with other prefixes: F78's third instance.
fn for_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    for_base(vm, "FOR.", "Stack is too shallow for inline for.()")
}

fn for_base(vm: &mut dyn Vm, prefix: &str, shallow: &str) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error(shallow.into()));
    }
    let body = vm
        .pull()
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #1")))?;
    if body.dt() != LAMBDA {
        return Err(Error(format!("{prefix}: #1 parameter must be lambda")));
    }
    const CHATTER_AT: u64 = 10_000_000;
    let mut turns: u64 = 0;
    loop {
        turns += 1;
        if turns == CHATTER_AT {
            vm.report(bund2_api::diag::Diagnostic::warning(format!(
                "`{}` has run {CHATTER_AT} iterations. If its body never leaves false, nothing will stop it.",
                prefix.to_lowercase()
            )));
        }
        vm.eval_lambda(&body)
            .map_err(|e| e.context(format!("{prefix}: lambda execution returns error: ")))?;
        let cond = vm
            .pull()
            .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #2")))?;
        match cond.unboxed() {
            BundValue::Bool(true, _) => {}
            BundValue::Bool(false, _) => return Ok(()),
            _ => {
                return Err(Error(format!(
                    "{prefix} returns error: This Dynamic type is not bool"
                )));
            }
        }
    }
}

/// `format` — a `leon` template filled from the stack
/// (`reference/rust_multistackvm/src/stdlib/string/format.rs:9-68`).
///
/// The template is pulled first, then **one stack value per distinct key**, in
/// the order the template names them (`:25-49`). A repeated key is filled once
/// (`:26-28`), so `"{A} {A}"` consumes one value.
///
/// `leon` is the reference's own template crate at the same major version. As
/// with `comfy_table`, the goldens capture the rendered text, so using the same
/// renderer is the only way to be byte-exact about its escaping and its errors.
fn format(vm: &mut dyn Vm) -> Result<(), Error> {
    format_base(vm, crate::wb::Side::Stack)
}

/// `format.` — the **template** comes off the workbench, everything else does
/// not (`reference/rust_multistackvm/src/stdlib/string/format.rs:72-119`).
///
/// The values that fill the template are still pulled from the current stack
/// (`:91`) and the rendered string is still pushed to it (`:119`). Only the
/// template moves. Its guard reads the workbench but *says* "Stack is too
/// shallow for inline format." (`:73-75`) — the same misnaming as F77, though
/// here the guard at least tests the side it reads.
fn format_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    format_base(vm, crate::wb::Side::Bench)
}

fn format_base(vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    let dot = side.dot();
    if side.depth(vm) < 1 {
        return Err(Error(format!("Stack is too shallow for inline format{dot}")));
    }
    let Some(tpl_value) = side.pull(vm) else {
        return Err(Error(format!("FORMAT{dot} returns: NO DATA #1")));
    };
    let str_tpl = tpl_value
        .as_str()
        .ok_or_else(|| Error("FORMAT return error: not a string".into()))?;
    let template = leon::Template::parse(str_tpl.as_str())
        .map_err(|e| Error(format!("FORMAT error parsing template: {e}")))?;

    let mut values: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for name in template.keys() {
        if values.contains_key(*name) {
            continue;
        }
        let Some(v) = vm.pull() else {
            return Err(Error("FORMAT: stack is too shallow".into()));
        };
        // **`conv(STRING)`, which is `display`** — the reference calls
        // `value.conv(STRING)` here (`format.rs:30`), the same conversion
        // `println` uses.
        //
        // This used to be a second, older stringifier living in this module,
        // written before `BundValue::display` existed, and it was wrong in two
        // ways that a template made visible where `println` did not. It
        // rendered a LIST with the raw `Debug` form, so
        // `[ 1 2 ] "L={0}" format` gave `L=Value { id: … }` instead of
        // `L=[ 1 ::  2 :: ]`; and it formatted floats with `{}` rather than
        // `dtoa`, so a `seq` of whole floats printed `100` where the oracle
        // prints `100.0`. Two goldens, one duplicated function.
        values.insert(name.to_string(), v.display());
    }
    let res = template
        .render(&values)
        .map_err(|e| Error(format!("FORMAT error rendering: {e}")))?;
    vm.push(BundValue::str(res));
    Ok(())
}


pub fn register(r: &mut Registry) {
    // **Opaque, all four: they run a lambda.** What `if` leaves is whatever
    // its branch leaves, which is a property of the branch and not of `if`.
    // Declaring `2 -> 0` made `bund2 check` report
    // `42 42 != { true } { false } ifthenelse println` — a line that runs,
    // because the branch pushes the value `println` then prints.
    //
    // The floor is real and is kept: `if` genuinely needs two operands before
    // it can start. Only what it leaves is unknown.
    r.register_native("if", if_true, StackEffect::opaque(2), WordKind::Sync);
    r.register_native("if.false", if_false, StackEffect::opaque(2), WordKind::Sync);
    r.register_native("ifthenelse", ifthenelse, StackEffect::opaque(3), WordKind::Sync);
    r.register_native("while", while_word, StackEffect::opaque(2), WordKind::Sync);
    // The `.` siblings: the lambda is still a stack operand, so each consumes
    // one fewer than its base rather than none.
    r.register_native("if.in_workbench", if_true_wb, StackEffect::opaque(1), WordKind::Sync);
    // opaque(2), not opaque(1): F78 means this one really does take both
    // operands off the stack.
    r.register_native("if.false.in_workbench", if_false_wb, StackEffect::opaque(2), WordKind::Sync);
    r.register_native("ifthenelse.", ifthenelse_wb, StackEffect::opaque(2), WordKind::Sync);
    r.register_native("while.", while_wb, StackEffect::opaque(1), WordKind::Sync);
    // Opaque, as `ifthenelse` and `if` are: what they leave is the branch's.
    r.register_native("notifthenelse", notifthenelse, StackEffect::opaque(3), WordKind::Sync);
    r.register_native("notifthenelse.", notifthenelse_wb, StackEffect::opaque(2), WordKind::Sync);
    r.register_native("if.stack", if_stack, StackEffect::opaque(2), WordKind::Sync);
    // `reference/rust_multistackvm/src/stdlib/logic/for_fun.rs:101-102`.
    r.register_native("for", for_word, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("for.", for_wb, StackEffect::opaque(1), WordKind::Sync);
    // Opaque, both forms: each pulls one stack value per distinct placeholder
    // in the template, and the template is not known until run time. `format`
    // said `1 -> 1`, true only of a template with no placeholder (F92).
    r.register_native("format", format, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("format.", format_wb, StackEffect::opaque(0), WordKind::Sync);

    // `reference/rust_multistackvm/src/stdlib/create_aliases.rs:9,10,14,15`.
    r.register_alias("?true", "if");
    r.register_alias("?true*", "ifthenelse");
    r.register_alias("?false", "if.false");
    // `reference/rust_multistackvm/src/stdlib/create_aliases.rs:8,12,13,15`
    r.register_alias("if.", "if.in_workbench");
    r.register_alias("?true.", "if.in_workbench");
    r.register_alias("?false.", "if.false.in_workbench");
    r.register_alias("?true*.", "ifthenelse.");
    // `reference/rust_multistackvm/src/stdlib/create_aliases.rs:11,15,17`.
    r.register_alias("?stack", "if.stack");
    r.register_alias("?false*", "notifthenelse");
    r.register_alias("?false*.", "notifthenelse.");
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

    /// `testing_if.bund`, verbatim from the corpus.
    #[test]
    fn the_corpus_if_program_runs() {
        let i = run_src("42 42 == {\n  true\n} if").expect("runs");
        assert_eq!(i.depth(), 1);
        assert_eq!(i.peek().map(|v| v.dt()), Some(bund2_value::BOOL));
    }

    #[test]
    fn a_false_condition_skips_the_branch() {
        let i = run_src("1 2 == { 99 } if").expect("runs");
        assert_eq!(i.depth(), 0, "nothing ran and nothing is left");
    }

    /// `testing_ifthenelse.bund`, verbatim. **The `then` branch is on top.**
    ///
    /// `42 42 !=` is false, so the *else* branch runs — and the else branch is
    /// the **first** of the two written, `{ true }`, because the reference
    /// pulls the top into `then_lambda` (`ifthenelse_fun.rs:28`). So this
    /// answers `true`, which is what the oracle answers for this program and
    /// what `tests/golden/tests/testing_ifthenelse.golden` holds.
    ///
    /// This test previously asserted `false`, matching an implementation that
    /// had the branches swapped. Both were wrong in the same direction, so the
    /// test could not catch the bug — a reminder that a unit test written
    /// alongside the code it tests is not an oracle. The golden was.
    #[test]
    fn ifthenelse_picks_the_right_branch() {
        let i = run_src("42 42 != {\n  true\n} {\n  false\n} ?true*").expect("runs");
        let v = i.peek().expect("a value");
        assert_eq!(v.dt(), bund2_value::BOOL);
        assert_eq!(*v.unboxed(), BundValue::boolean(true), "the else branch ran");
    }

    #[test]
    fn if_requires_a_lambda() {
        let e = match run_src("true 1 if") {
            Ok(_) => panic!("expected a failure"),
            Err(e) => e,
        };
        assert!(e.contains("IF: #1 parameter must be lambda"), "{e}");
    }

    /// One stack value per distinct key, in template order.
    #[test]
    fn format_fills_from_the_stack() {
        let i = run_src("42 \"n = {A}\" format").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_str()).as_deref(), Some("n = 42"));
    }

    /// A repeated key consumes one value, not two (`format.rs:26-28`).
    #[test]
    fn a_repeated_key_is_filled_once() {
        let i = run_src("7 \"{A} and {A}\" format").expect("runs");
        assert_eq!(
            i.peek().and_then(|v| v.as_str()).as_deref(),
            Some("7 and 7")
        );
        assert_eq!(i.depth(), 1, "only one value was consumed");
    }

    #[test]
    fn every_word_here_resolves() {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        for name in ["if", "?true", "if.false", "?false", "ifthenelse", "?true*", "format"] {
            assert!(
                i.registry.interner.lookup_call(name).is_some(),
                "{name} is not registered"
            );
        }
    }
}
