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

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect { consumes, produces }
}

/// Pull the lambda, then the condition, then run the branch when the condition
/// matches (`reference/rust_multistackvm/src/stdlib/logic/if_fun.rs:12-89`).
///
/// The lambda comes off **first** — it is on top, pushed last — so `cond { … }
/// if` reads in source order even though the pulls are reversed.
///
/// A non-LAMBDA first operand is an error (`:79-81`). The reference has an
/// unreachable `execute` fallback inside the taken branch (`:60-63`), guarded
/// out by the type check above it.
fn if_base(vm: &mut dyn Vm, want: bool, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let lambda_val = crate::pull::operand(vm, prefix, 1)?;
    if lambda_val.dt() != LAMBDA {
        return Err(Error(format!("{prefix}: #1 parameter must be lambda")));
    }
    let cond_val = vm
        .pull()
        .ok_or_else(|| Error(format!("{prefix} returns: NO DATA #2")))?;
    let cond = cast_bool(&cond_val)
        .ok_or_else(|| Error(format!("{prefix} returns error: can not cast to bool")))?;
    if cond == want {
        let body = lambda_val
            .as_lambda()
            .ok_or_else(|| Error::internal("a value tagged LAMBDA carried no body"))?
            .to_vec();
        return vm.eval_body(&body);
    }
    Ok(())
}

/// `cast_bool`, for the kinds a condition can be.
///
/// A BOOL is itself; the numeric kinds are false at zero. `==` and friends
/// push a BOOL, so that is the path the corpus takes.
pub(crate) fn cast_bool(v: &BundValue) -> Option<bool> {
    match v.unboxed() {
        BundValue::Bool(b) => Some(*b),
        BundValue::Int(i) => Some(*i != 0),
        BundValue::Float(f) => Some(*f != 0.0),
        _ => None,
    }
}

fn if_true(vm: &mut dyn Vm) -> Result<(), Error> {
    if_base(vm, true, "IF")
}

fn if_false(vm: &mut dyn Vm) -> Result<(), Error> {
    if_base(vm, false, "?FALSE")
}

/// `ifthenelse`, spelled `?true*` — two lambdas and a condition
/// (`reference/rust_multistackvm/src/stdlib/logic/ifthenelse_fun.rs`).
///
/// The *else* branch is on top, because it was pushed last: `cond { then }
/// { else } ?true*`.
fn ifthenelse(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 3 {
        return Err(Error("Stack is too shallow for inline IFTHENELSE".into()));
    }
    let else_val = crate::pull::operand(vm, "IFTHENELSE", 1)?;
    let then_val = crate::pull::operand(vm, "IFTHENELSE", 2)?;
    for v in [&then_val, &else_val] {
        if v.dt() != LAMBDA {
            return Err(Error("IFTHENELSE: parameters must be lambda".into()));
        }
    }
    let cond_val = crate::pull::operand(vm, "IFTHENELSE", 3)?;
    let cond = cast_bool(&cond_val)
        .ok_or_else(|| Error("IFTHENELSE returns error: can not cast to bool".into()))?;
    let chosen = if cond { then_val } else { else_val };
    let body = chosen
        .as_lambda()
        .ok_or_else(|| Error::internal("a value tagged LAMBDA carried no body"))?
        .to_vec();
    vm.eval_body(&body)
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
    let Some(tpl_value) = vm.pull() else {
        return Err(Error("Stack is too shallow for inline format".into()));
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
        values.insert(name.to_string(), to_display(&v));
    }
    let res = template
        .render(&values)
        .map_err(|e| Error(format!("FORMAT error rendering: {e}")))?;
    vm.push(BundValue::str(res));
    Ok(())
}

/// `conv(STRING)` for the kinds a template can carry
/// (`reference/rust_multistackvm/src/stdlib/string/format.rs:31-33`).
fn to_display(v: &BundValue) -> String {
    if let Some(s) = v.as_str() {
        return s;
    }
    match v.unboxed() {
        BundValue::Int(i) => i.to_string(),
        BundValue::Float(f) => format!("{f}"),
        BundValue::Bool(b) => b.to_string(),
        other => other.render(false),
    }
}

pub fn register(r: &mut Registry) {
    r.register_native("if", if_true, eff(2, 0), WordKind::Sync);
    r.register_native("if.false", if_false, eff(2, 0), WordKind::Sync);
    r.register_native("ifthenelse", ifthenelse, eff(3, 0), WordKind::Sync);
    r.register_native("format", format, eff(1, 1), WordKind::Sync);

    // `reference/rust_multistackvm/src/stdlib/create_aliases.rs:9,10,14,15`.
    r.register_alias("?true", "if");
    r.register_alias("?true*", "ifthenelse");
    r.register_alias("?false", "if.false");
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

    /// `testing_ifthenelse.bund`, verbatim. The else branch is on top.
    #[test]
    fn ifthenelse_picks_the_right_branch() {
        let i = run_src("42 42 != {\n  true\n} {\n  false\n} ?true*").expect("runs");
        // `42 42 !=` is false, so the *else* branch runs and pushes `false`.
        let v = i.peek().expect("a value");
        assert_eq!(v.dt(), bund2_value::BOOL);
        assert_eq!(*v.unboxed(), BundValue::Bool(false), "the else branch ran");
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
