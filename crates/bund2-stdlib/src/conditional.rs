//! Conditionals — **control flow that is partly data**.
//!
//! `?ifthenelse`, `?try`, `?error`, `context` and `curry` do not branch. Each
//! pushes a **CONDITIONAL value** tagged with a `type` string
//! (`reference/Bund/src/stdlib/functions/conditional/conditional_ifthenelse.rs:8-10`),
//! whose branches are lambdas filed into named slots by `set` afterwards. `!`
//! on that value reads `type`, looks the handler up in the conditional table,
//! and calls it (`reference/rust_multistackvm/src/stdlib/execute_types/execute_conditionals.rs:9-25`).
//!
//! So the shape of a `?try` is not visible until it runs. That is why RFC-0003
//! lowers the `?`-family as a helper call while `if` and `ifthenelse` — which
//! take their branches as arguments — stay statically analysable.
//!
//! **A missing slot defaults to an empty lambda rather than erroring**
//! (`conditional_ifthenelse.rs:15-26`), so a half-populated conditional runs
//! and does nothing. That is preserved.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, CONDITIONAL, LAMBDA, STRING};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// A slot's lambda, or an empty one — the reference's `Err(_) => Value::lambda()`
/// (`conditional_ifthenelse.rs:15-26`).
/// A slot's LAMBDA **value**, for running it — held, not copied, so its `Rc`
/// reaches the entry point (D42). A missing or non-LAMBDA slot is an empty
/// body, as it always was.
fn slot_body(c: &BundValue, key: &str) -> BundValue {
    c.get(key)
        .filter(|v| v.dt() == LAMBDA && v.as_lambda().is_some())
        .unwrap_or_else(|| BundValue::lambda(Vec::new()))
}

/// A slot's items, for `context`, which concatenates three slots into one body
/// assembled per call — a body with no key to keep.
fn slot_items(c: &BundValue, key: &str) -> Vec<BundValue> {
    c.get(key)
        .filter(|v| v.dt() == LAMBDA)
        .and_then(|v| v.as_lambda().map(<[BundValue]>::to_vec))
        .unwrap_or_default()
}

fn new_conditional(ty: &str) -> BundValue {
    BundValue::conditional(Default::default()).set("type", BundValue::str(ty))
}

// --- the words that construct -----------------------------------------------

fn q_ifthenelse(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(new_conditional("ifthenelse"));
    Ok(())
}

fn q_try(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(new_conditional("tryexcept"));
    Ok(())
}

fn q_error(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(new_conditional("error"));
    Ok(())
}

/// `fmt` — a CONDITIONAL that renders a **localised** message template
/// (`reference/Bund/src/stdlib/functions/conditional/conditional_fmt.rs:133-138`).
fn q_fmt(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(new_conditional("fmt"));
    Ok(())
}

/// The machine's locale, or `en-US`
/// (`conditional_fmt.rs:153-156`, and the same fallback at `:14-17`).
fn locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string())
}

/// Look a key up as `{key}.{locale}` first, then as `{key}`
/// (`conditional_fmt.rs:157-161`).
///
/// **This is why `fmt`'s output is machine-dependent.** The corpus demo
/// declares `greeting`, `greeting.en-US` and `greeting.ru-RU`, and which one
/// renders is decided by the host — the golden captured the `en-US` one.
/// Preserved as the reference's behaviour rather than pinned to a fixed
/// locale.
fn localised(c: &BundValue, key: &str) -> Option<BundValue> {
    c.get(&format!("{key}.{}", locale())).or_else(|| c.get(key))
}

/// Render one message through `leon`, filling keys from the conditional and
/// then from the stack (`conditional_fmt.rs:9-76`).
///
/// Per key, in order: `{key}.{locale}`, then `{key}` — both **recursively
/// rendered**, because a slot may itself be a template (`:34-42`) — and
/// failing both, one value **pulled from the stack** and converted with
/// `conv(STRING)` (`:45-57`). So `"{answer} {pi}"` with `answer` in the map
/// takes `pi` off the stack.
///
/// A repeated key is filled once (`:26-28`), matching `format`.
fn render_message(vm: &mut dyn Vm, c: &BundValue, msg: &BundValue) -> Result<String, Error> {
    // A non-STRING message is converted and not treated as a template
    // (`:87-95`), which is what stops a number in a slot being parsed as one.
    if msg.dt() != STRING {
        return Ok(msg.display());
    }
    let text = msg
        .as_str()
        .ok_or_else(|| Error("FMT.STR error casting template: not a string".into()))?;
    let template = leon::Template::parse(text.as_str())
        .map_err(|e| Error(format!("FMT.STR error parsing template: {e}")))?;

    let mut values: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for name in template.keys() {
        if values.contains_key(*name) {
            continue;
        }
        let filled = match localised(c, name) {
            Some(inner) => render_message(vm, c, &inner)?,
            None => {
                let v = vm
                    .pull()
                    .ok_or_else(|| Error("FMT.STR: stack is too shallow".into()))?;
                v.display()
            }
        };
        values.insert(name.to_string(), filled);
    }
    template
        .render(&values)
        .map(|r| r.to_string())
        .map_err(|e| Error(format!("FMT.STR error rendering: {e}")))
}

/// `!` on a `fmt` CONDITIONAL — pull a message **name**, render, push the text
/// (`conditional_fmt.rs:140-171`).
fn run_fmt(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for FMT.RUN".into()));
    }
    let name_val = vm
        .pull()
        .ok_or_else(|| Error("FMT.RUN: No context name discovered on the stack".into()))?;
    let name = name_val
        .as_str()
        .ok_or_else(|| Error("FMT.RUN: Error name casting".into()))?;
    let msg = localised(&c, &name).ok_or_else(|| {
        Error(format!(
            "FMT.RUN: getting message with name {name} returns error: key not found"
        ))
    })?;
    let text = render_message(vm, &c, &msg).map_err(|e| {
        Error(format!(
            "FMT.RUN: error converting message with name {name} returns error: {}",
            e.0
        ))
    })?;
    vm.push(BundValue::str(text));
    Ok(())
}

/// `conditional`, aliased `?` — type `through`
/// (`reference/rust_multistackvm/src/stdlib/artefacts.rs:121-125`).
fn conditional(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(new_conditional("through"));
    Ok(())
}

/// `context` and `curry` both take a name first
/// (`conditional_ctx.rs:8-23`, `conditional_curry.rs:8-25`).
fn named(vm: &mut dyn Vm, ty: &str, word: &str) -> Result<(), Error> {
    let Some(name_val) = vm.pull() else {
        return Err(Error(format!("Stack is too shallow for {word}")));
    };
    let name = name_val
        .as_str()
        .ok_or_else(|| Error("CONTEXT: Error name casting".into()))?;
    vm.push(new_conditional(ty).set("name", BundValue::str(name)));
    Ok(())
}

fn context_word(vm: &mut dyn Vm) -> Result<(), Error> {
    named(vm, "context", "context")
}

fn curry_word(vm: &mut dyn Vm) -> Result<(), Error> {
    named(vm, "curry", "curry")
}

/// `raise` — turn a value into a failure
/// (`reference/Bund/src/stdlib/functions/conditional/raise.rs:6-17`).
///
/// A missing or uncastable message becomes a fixed string rather than a
/// different error, which is why both spellings are reproduced.
fn raise(vm: &mut dyn Vm) -> Result<(), Error> {
    let msg = match vm.pull() {
        Some(v) => v
            .as_str()
            .unwrap_or_else(|| "Can not cast a message for raise".to_string()),
        None => "No message for raise".to_string(),
    };
    Err(Error(msg))
}

// --- the handlers -----------------------------------------------------------

/// `through` — run the `run` slot if there is one, and say nothing if there
/// is not (`reference/rust_multistackvm/src/stdlib/execute_types/conditional_through.rs:6-16`).
fn run_through(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    let body = slot_body(&c, "run");
    vm.eval_lambda(&body)
}

/// `ifthenelse` — evaluate `if`, **inspect the stack**, then evaluate a branch
/// (`conditional_ifthenelse.rs:14-50`).
///
/// The inspection between the two evaluations is what S4a's request/resume
/// mechanism exists for: this is not a tail call, so an exit action cannot
/// express it. Until the frame loop lands it re-enters evaluation directly.
fn run_ifthenelse(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    let (if_l, then_l, else_l) = (slot_body(&c, "if"), slot_body(&c, "then"), slot_body(&c, "else"));
    vm.eval_lambda(&if_l)
        .map_err(|e| e.context("IFTHENELSE IF lambda returns: "))?;
    let cond_val = vm
        .pull()
        .ok_or_else(|| Error("IFTHENELSE conditional require condition on the stack".into()))?;
    let cond = crate::control::cast_bool(&cond_val)
        .ok_or_else(|| Error("IFTHENELSE error casting conditional".into()))?;
    if cond {
        vm.eval_lambda(&then_l)
            .map_err(|e| e.context("IFTHENELSE THEN lambda returns: "))
    } else {
        vm.eval_lambda(&else_l)
            .map_err(|e| e.context("IFTHENELSE ELSE lambda returns: "))
    }
}

/// `tryexcept` — run `try`; on failure **reify the error as a CONDITIONAL**,
/// push it, then run `except` and `recovery`
/// (`conditional_tryexcept.rs:13-51`).
///
/// The error becomes a value: type `error`, an `associated` slot carried over,
/// and a `context` slot holding the failure's text. That is the whole exception
/// model — a Rust `Err` caught at the lambda boundary and turned into data.
fn run_tryexcept(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    let try_l = slot_body(&c, "try");
    let except_l = slot_body(&c, "except");
    let recovery_l = slot_body(&c, "recovery");
    let associated = c
        .get("associated")
        .unwrap_or_else(|| BundValue::lambda(Vec::new()));

    if let Err(e) = vm.eval_lambda(&try_l) {
        let err_c = new_conditional("error")
            .set("associated", associated)
            .set("context", BundValue::str(e.0));
        vm.push(err_c);
        vm.eval_lambda(&except_l)
            .map_err(|e| e.context("TRYEXCEPT EXCEPT lambda returns: "))?;
        vm.eval_lambda(&recovery_l)
            .map_err(|e| e.context("TRYEXCEPT RECOVERY lambda returns: "))?;
    }
    Ok(())
}

/// `error` — report, then run `associated` (`conditional_error.rs:15-32`).
fn run_error(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    let msg = c
        .get("context")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    // **Non-critical, so delivered quietly.** The program asked for this to be
    // said and is still running, so it goes through the reporter as a `Notice`
    // — one line on stderr, no table and no stack dump — rather than being
    // printed here. A TUI receives it as a value instead of finding text on a
    // stream it does not own.
    vm.report(bund2_api::diag::Diagnostic::notice(msg));
    let associated = slot_body(&c, "associated");
    vm.eval_lambda(&associated)
        .map_err(|e| e.context("ERROR ASSOCIATED lambda returns: "))
}

/// `context` — run `<n>.pre`, `<n>`, `<n>.post` on a named stack, then come
/// back (`conditional_ctx.rs:25-75`).
///
/// **F57 is fixed here.** The reference restores the previous stack at `:74`,
/// after three `bail!`s that return before reaching it, so a failure inside a
/// context leaves the interpreter on the context's stack — observable wherever
/// `?try` catches. Bund2 restores on both paths.
fn run_context(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    let cond_name = c
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error("CONTEXT.RUN: getting context name returns error".into()))?;
    let prev = vm.current_name();
    let name_val = vm
        .pull()
        .ok_or_else(|| Error("CONTEXT.RUN: No context name discovered on the stack".into()))?;
    let n = name_val
        .as_str()
        .ok_or_else(|| Error("CONTEXT.RUN: Error name casting".into()))?;

    let pre = slot_items(&c, &format!("{n}.pre"));
    let body = slot_items(&c, &n);
    let post = slot_items(&c, &format!("{n}.post"));

    // **F57, structurally.** One scoped call whose frame carries the return as
    // an exit action, so the restore happens on the failure path without a
    // second `to_stack` anyone could forget — which is exactly the shape the
    // reference gets wrong.
    let mut all = pre;
    all.extend(body);
    all.extend(post);
    let _ = prev;
    vm.scoped_call(&cond_name, all)
        .map_err(|e| e.context("CONTEXT lambda returns: "))
}

/// `curry` — **code generation**. Build a lambda of the captured data, the
/// target lambda and a `!`, then register it under a name
/// (`conditional_curry.rs:27-53`).
///
/// The data is pushed in reverse (`:47`), and the trailing `!` is an alias, so
/// every curried word ends in a call site whose inline cache S8 must invalidate
/// on an alias-table generation bump.
fn run_curry(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    let cond_name = c
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error("CONTEXT.RUN: getting curry name returns error".into()))?;
    let data = c
        .get("data")
        .and_then(|v| v.as_list().map(<[BundValue]>::to_vec))
        .unwrap_or_default();
    let lambda_val = c
        .get("lambda")
        .unwrap_or_else(|| BundValue::lambda(Vec::new()));

    let mut body: Vec<BundValue> = data.into_iter().rev().collect();
    body.push(lambda_val);
    body.push(BundValue::call("!"));
    vm.register_lambda(&cond_name, BundValue::lambda(body));
    Ok(())
}

// --- endcontext -------------------------------------------------------------

/// `endcontext` — close the scope `( … )` opened
/// (`reference/rust_multistackvm/src/stdlib/ctx.rs:5-27`).
///
/// Carry the top value out to the workbench if there is one, drop the scratch
/// stack, and return to the stack that was current when the context opened.
///
/// **F60 is fixed here.** The reference guards with
/// `if vm.stacks_stack.len() < 1`, which can never be true: the deque is
/// initialised holding `"main"`
/// (`reference/rust_multistackvm/src/multistackvm.rs:38-39`) and `pop_stacks`
/// refuses to go below one
/// (`reference/rust_multistackvm/src/multistackvm_stacks_stack.rs:10-16`). So a
/// bare `endcontext` passes the guard and drops whatever stack is current —
/// probed: `111 222 333` on `main` leaves `main` empty with `333` on the
/// workbench and no diagnostic.
///
/// Bund2 tracks context depth separately from the stack-of-stacks, so the
/// guard the reference wrote can actually fire. This is a **narrowing**, and
/// the only one in RFC-0003: a program that today destroys a stack silently
/// now gets the reference's own message.
fn endcontext(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.context_depth() == 0 {
        return Err(Error("Context is empty".into()));
    }
    if vm.depth() > 0 {
        let last = crate::pull::operand(vm, "ENDCONTEXT", 1)?;
        vm.push_workbench(last);
    }
    let name = vm.current_name();
    let prev = vm
        .pop_context()
        .ok_or_else(|| Error::internal("context depth was non-zero but no context was open"))?;
    vm.drop_stack(&name);
    vm.to_stack(&prev);
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.register_native("?ifthenelse", q_ifthenelse, eff(0, 1), WordKind::Sync);
    r.register_native("?try", q_try, eff(0, 1), WordKind::Sync);
    r.register_native("?error", q_error, eff(0, 1), WordKind::Sync);
    r.register_native("fmt", q_fmt, eff(0, 1), WordKind::Sync);
    r.register_native("conditional", conditional, eff(0, 1), WordKind::Sync);
    r.register_native("context", context_word, eff(1, 1), WordKind::Sync);
    r.register_native("curry", curry_word, eff(1, 1), WordKind::Sync);
    r.register_native("raise", raise, eff(1, 0), WordKind::Sync);
    r.register_native("endcontext", endcontext, eff(0, 0), WordKind::Sync);
    r.register_alias("?", "conditional");

    // The conditional table. The reference fills this from two crates through
    // a global mutex; here it is registry state.
    r.register_conditional("through", run_through);
    r.register_conditional("ifthenelse", run_ifthenelse);
    r.register_conditional("tryexcept", run_tryexcept);
    r.register_conditional("error", run_error);
    r.register_conditional("context", run_context);
    r.register_conditional("curry", run_curry);
    r.register_conditional("fmt", run_fmt);
}

/// `!` on a CONDITIONAL — read `type`, look it up, call it
/// (`reference/rust_multistackvm/src/stdlib/execute_types/execute_conditionals.rs:9-25`).
pub fn execute_conditional(vm: &mut dyn Vm, c: BundValue) -> Result<(), Error> {
    debug_assert_eq!(c.dt(), CONDITIONAL);
    let ty = c
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error("EXECUTE:CONDITIONAL can not detect conditional type".into()))?;
    match vm.conditional(&ty) {
        Some(f) => f(vm, c),
        None => Err(Error(format!(
            "EXECUTE:CONDITIONAL conditionals handler does not exist: {ty}"
        ))),
    }
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

    fn err_of(src: &str) -> String {
        match run_src(src) {
            Ok(_) => panic!("{src} was expected to fail"),
            Err(e) => e,
        }
    }

    /// `testing_tryexcept.bund`, verbatim from the corpus. `raise` fails inside
    /// `try`, the error is reified, `except` drops it and pushes `true`.
    #[test]
    fn the_corpus_tryexcept_program_runs() {
        let i = run_src(
            "?try\n  :try {\n    \"boom\" raise\n  } set\n  :except {\n    drop true\n  } set\n!",
        )
        .expect("runs");
        assert_eq!(i.depth(), 1, "the except branch left one value");
        assert_eq!(*i.peek().unwrap().unboxed(), BundValue::boolean(true));
    }

    /// A `try` that succeeds runs neither `except` nor `recovery`.
    #[test]
    fn a_successful_try_skips_except() {
        let i = run_src("?try :try { 1 } set :except { 99 } set !").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(1));
    }

    /// The caught error arrives as a CONDITIONAL carrying its text.
    ///
    /// The text is **wrapped**, not bare: `lambda_eval` prefixes each element's
    /// failure with `Lambda content evaluation returned error:`
    /// (`reference/rust_multistackvm/src/multistackvm_lambda_eval.rs:17`), so
    /// `err.ctx` encodes the nesting a failure passed through. That is Q21 —
    /// RFC-0003's frame loop replaces the nesting and has not said how the
    /// string is rebuilt — so this asserts the message survives, not its exact
    /// assembly.
    #[test]
    fn a_caught_error_is_reified_as_a_value() {
        let i = run_src("?try :try { \"boom\" raise } set :except { 0 } set !").expect("runs");
        let c = i
            .snapshot()
            .into_iter()
            .find(|v| v.dt() == CONDITIONAL)
            .expect("the error conditional was pushed");
        assert_eq!(c.get("type").and_then(|t| t.as_str()).as_deref(), Some("error"));
        let ctx = c.get("context").and_then(|t| t.as_str()).unwrap_or_default();
        assert!(ctx.contains("boom"), "the message survives: {ctx}");
    }

    /// `?ifthenelse` evaluates its condition slot, inspects, then branches.
    #[test]
    fn ifthenelse_as_a_conditional_value() {
        let i = run_src("?ifthenelse :if { true } set :then { 1 } set :else { 2 } set !")
            .expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(1));
        let j = run_src("?ifthenelse :if { false } set :then { 1 } set :else { 2 } set !")
            .expect("runs");
        assert_eq!(j.peek().and_then(|v| v.as_int()), Some(2));
    }

    /// A missing slot is an empty lambda, not an error.
    #[test]
    fn a_missing_slot_runs_nothing() {
        let i = run_src("?ifthenelse :if { true } set !").expect("runs");
        assert_eq!(i.depth(), 0);
    }

    /// `( … )` now runs end to end, which needs `endcontext`.
    #[test]
    fn a_context_opens_and_closes() {
        let i = run_src("( 7 ) take").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(7), "carried out");
    }

    /// **F60.** A bare `endcontext` fails instead of destroying the stack.
    #[test]
    fn a_bare_endcontext_refuses() {
        let e = err_of("111 222 333 endcontext");
        assert!(e.contains("Context is empty"), "{e}");
    }

    /// **F57.** A failure inside a context leaves the interpreter where it
    /// started, because the restore runs on both paths.
    #[test]
    fn a_failing_context_still_restores_the_stack() {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let before = i.current_name();
        // `:scratch context` names the stack to run on; `:run { … } set` files
        // the body; `:run swap !` leaves the slot name under the conditional,
        // which is the order the handler pulls them in.
        let src = ":scratch context :run { \"boom\" raise } set :run swap !";
        let stream = bund2_syntax::compile(src).expect("parses");
        let _ = i.eval(&stream);
        assert_eq!(i.current_name(), before, "restored despite the failure");
    }

    #[test]
    fn every_word_here_resolves() {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        for name in [
            "?ifthenelse", "?try", "?error", "conditional", "?", "context", "curry", "raise",
            "endcontext",
        ] {
            assert!(
                i.registry.interner.lookup_call(name).is_some(),
                "{name} is not registered"
            );
        }
        for ty in ["through", "ifthenelse", "tryexcept", "error", "context", "curry"] {
            assert!(i.registry.conditional(ty).is_some(), "{ty} has no handler");
        }
    }
}
