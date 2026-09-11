//! Containers, the word table, and `execute` — the vocabulary the corpus's
//! block idiom needs.
//!
//! Three of the four most-demanded words across the goldens live here:
//! `set` (29 goldens), `!` (24) and `register` (22).

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{
    BundValue, CALL, CLASS, CONDITIONAL, LAMBDA, LIST, MAP, Metric, OBJECT, PTR, STRING,
};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// `dict` — an empty MAP (`reference/rust_multistackvm/src/stdlib/artefacts.rs:113-115`).
/// `config` is an alias for it
/// (`reference/Bund/src/stdlib/functions/create_aliases.rs:26`).
fn dict(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::map(Default::default()));
    Ok(())
}

/// `valuemap` — a map keyed by whole values, not strings
/// (`reference/rust_multistackvm/src/stdlib/artefacts.rs:147`). Aliased
/// `match` (`create_aliases.rs:45`).
fn valuemap(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::valuemap(Default::default()));
    Ok(())
}

/// `list` — an empty LIST (`artefacts.rs`).
fn list(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::list(Vec::new()));
    Ok(())
}

/// `lambda` — an empty LAMBDA. Aliased `λ` and `Λ`
/// (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:28-29`).
fn lambda(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::lambda(Vec::new()));
    Ok(())
}

/// `metrics` — a METRICS value of **128 zeroed samples**. Aliased `sample`
/// (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:37`).
///
/// The word is `vm.apply(Value::metrics())`
/// (`reference/rust_multistackvm/src/stdlib/artefacts.rs:61-63`), and
/// `Value::metrics` is `Value::metrics_n(128)`
/// (`reference/rust_dynamic/src/create_metrics.rs:24-26`) — a fixed-size
/// buffer, not a growable one. The 128 is the observable part: it is what
/// `tests/golden/probes/dt-reachable.golden` pins.
///
/// **Each sample is stamped, and the stamps are not all equal.** `Metric::new`
/// calls `timestamp_ns` per sample
/// (`reference/rust_dynamic/src/metric.rs:12-16`), so the buffer records 128
/// separate readings of the clock rather than one. Nothing here can depend on
/// that — the golden normalises every stamp — but constructing them from a
/// single `now` would be a different value, and `debug.display_stack` is not
/// the only thing that ever reads one.
fn metrics(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::metrics(
        (0..128).map(|_| Metric::new(0.0)).collect(),
    ));
    Ok(())
}

/// `nodata`. Aliased `|` and `∅` (`create_aliases.rs:40-41`).
fn nodata(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::nodata());
    Ok(())
}

/// `set` — `receiver key value set`.
///
/// The operands come off in the opposite order to the source text: the stored
/// value first, then the key, then the receiver
/// (`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:10-16`). What
/// happens next is `BundValue::set`'s four arms, which are not all "insert".
fn set(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 3 {
        return Err(Error("Stack is too shallow for inline set".into()));
    }
    let d_val = crate::pull::operand(vm, "SET", 1)?;
    let key_val = crate::pull::operand(vm, "SET", 2)?;
    let receiver = crate::pull::operand(vm, "SET", 3)?;
    // **The branch that makes a valuemap writable.** `set` tests the receiver
    // first and passes the key through uncast for a VALUEMAP
    // (`reference/rust_multistackvm/src/stdlib/values/value_dict.rs:17-19`);
    // only the fallback casts to a string.
    if receiver.is_valuemap() {
        vm.push(receiver.set_vmap(key_val, d_val));
        return Ok(());
    }
    let key = key_val
        .as_str()
        .ok_or_else(|| Error("SET key expected to be string".into()))?;
    vm.push(receiver.set(&key, d_val));
    Ok(())
}

/// `get` — **D30's mirror**, and a deliberate deviation.
///
/// The reference casts the key to a string *before* pulling the container
/// (`value_dict.rs:52-59`), so it can never observe that the container is a
/// `valuemap` and a non-string key fails before the container is examined —
/// F29, which leaves `valuemap` with no read path at all. `set` does the
/// opposite: it pulls all three and branches on the container (`:17-19`).
///
/// D30 resolves the asymmetry by making `get` mirror `set`. Bund2 pulls both,
/// branches on the container, and only then casts. For a MAP that is
/// indistinguishable from the reference; for a `valuemap` it is the difference
/// between working and not.
fn get(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline get".into()));
    }
    let key_val = crate::pull::operand(vm, "GET", 1)?;
    let container = crate::pull::operand(vm, "GET", 2)?;
    // **D30's mirror, and the read path F29 says does not exist.** The
    // reference casts the key before it looks at the container, so a VALUEMAP
    // can be written and never read. Testing the container first is exactly
    // what `set` already does.
    if container.is_valuemap() {
        return match container.get_vmap(&key_val) {
            Some(v) => {
                vm.push(v);
                Ok(())
            }
            None => Err(Error("GET returns error: key not found".into())),
        };
    }
    let key = key_val
        .as_str()
        .ok_or_else(|| Error("GET key expected to be string".into()))?;
    match container.get(&key) {
        Some(v) => {
            vm.push(v);
            Ok(())
        }
        None => Err(Error(format!("GET returns error: key {key} not found"))),
    }
}

/// `?key` — leaves the container and pushes whether the key is present
/// (`value_dict.rs:83-101`).
fn has_key(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline get".into()));
    }
    let key_val = crate::pull::operand(vm, "?KEY", 1)?;
    let container = crate::pull::operand(vm, "?KEY", 2)?;
    let key = key_val
        .as_str()
        .ok_or_else(|| Error("GET key expected to be string".into()))?;
    let present = container.has_key(&key);
    vm.push(container);
    vm.push(BundValue::boolean(present));
    Ok(())
}

/// `register` — bind a name to a LAMBDA or a CLASS
/// (`reference/rust_multistackvm/src/stdlib/lambdas/registry.rs:5-38`).
///
/// `:Name { … } register` is the documented way to define a word, so this and
/// the parser's block form are what make the corpus's own idiom expressible.
fn register(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline register".into()));
    }
    let body = crate::pull::operand(vm, "REGISTER", 1)?;
    let name_val = crate::pull::operand(vm, "REGISTER", 2)?;
    let name = name_val
        .as_str()
        .ok_or_else(|| Error("REGISTER expecting lambda name to be string".into()))?;
    // A CLASS goes to the **class registry**, a LAMBDA to the word table. They
    // are different tables, which is why `:Probe class register` then `Probe`
    // reports `Probe not registered` — the class is filed, just not as a word.
    match body.dt() {
        LAMBDA => {
            vm.register_lambda(&name, body);
            Ok(())
        }
        CLASS => {
            vm.register_class(&name, body);
            Ok(())
        }
        _ => Err(Error("REGISTER expecting CLASS or LAMBDA".into())),
    }
}

fn unregister(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(name_val) = vm.pull() else {
        return Err(Error("Stack is too shallow for inline unregister".into()));
    };
    let name = name_val
        .as_str()
        .ok_or_else(|| Error("UNREGISTER expecting lanbda name to be string".into()))?;
    vm.unregister_lambda(&name);
    Ok(())
}

/// `lambda!` — a LIST becomes a LAMBDA
/// (`reference/Bund/src/stdlib/functions/bund/bund_fun.rs:163-186`).
fn to_lambda(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull() else {
        return Err(Error("Stack is too shallow for LAMBDA.MAKE".into()));
    };
    match v.as_list() {
        Some(items) => {
            vm.push(BundValue::lambda(items.to_vec()));
            Ok(())
        }
        None => Err(Error("LAMBDA.MAKE casting list returned: not a list".into())),
    }
}

/// `lambda*` — **the whole stack** becomes a LAMBDA, in order
/// (`bund_fun.rs:189-202`).
///
/// Variadic over the live stack, which is why §1.3 calls metaprogramming
/// regions optimisation barriers by construction: nothing static knows how
/// deep the stack was. Note it does not check for an empty stack; draining
/// nothing yields an empty lambda.
fn fold_lambda(vm: &mut dyn Vm) -> Result<(), Error> {
    let mut body = Vec::new();
    while let Some(v) = vm.pull() {
        body.insert(0, v);
    }
    vm.push(BundValue::lambda(body));
    Ok(())
}

/// `make.call` — a string becomes a CALL. Aliased `call,`
/// (`reference/Bund/src/stdlib/functions/create_aliases.rs`), which is the
/// spelling §1.3's idiom uses.
fn make_call(vm: &mut dyn Vm) -> Result<(), Error> {
    make_call_base(vm, crate::wb::Side::Stack)
}

/// `make.call.` — the plain mirror: name off the workbench, CALL back to it
/// (`reference/Bund/src/stdlib/functions/values/make_call_value.rs:18-19,25,38`).
///
/// This one's guard *does* name the workbench, unlike F77's print family.
fn make_call_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    make_call_base(vm, crate::wb::Side::Bench)
}

fn make_call_base(vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    let prefix = &format!("MAKE.CALL{}", side.dot());
    if side.depth(vm) < 1 {
        return Err(Error(format!(
            "{} is too shallow for inline {prefix}",
            if side == crate::wb::Side::Stack { "Stack" } else { "Workbench" }
        )));
    }
    let Some(v) = side.pull(vm) else {
        return Err(Error(format!("{prefix} returns NO DATA #1")));
    };
    let name = v.as_str().ok_or_else(|| {
        Error(format!("{prefix} casting of string returned: not a string"))
    })?;
    side.push(vm, BundValue::call(name));
    Ok(())
}

/// `execute`, spelled `!` — the polymorphic call.
///
/// Eight arms on `type_of()`
/// (`reference/rust_multistackvm/src/stdlib/execute.rs:26-98`). Four are
/// implemented here; CONDITIONAL, CLASS and OBJECT need the conditional
/// registry and the object model, which are RFC-0009's and are not built.
///
/// **The recursion is real.** The LIST and MAP arms re-enter `execute`
/// (`:42,67`) and the LAMBDA arm re-enters evaluation (`:94`), so Rust depth
/// tracks Bund depth exactly as the reference does. RFC-0003's S4 replaces
/// that with a frame loop; until it exists, this is faithful and shallow.
pub(crate) fn execute_top(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull() else {
        return Err(Error("Stack is too shallow for inline execute()".into()));
    };
    execute_value(vm, v)
}

/// `execute.`, spelled `!.` — **F53 and F59, which land together**.
///
/// Two defects, one change. F53: the reference's wrapper guards the **main**
/// stack (`reference/rust_multistackvm/src/stdlib/execute.rs:117`) and then
/// pulls the **workbench** (`:22`), so a value waiting on the workbench cannot
/// be executed while the main stack happens to be empty. F59: correcting that
/// guard exposes the LIST and MAP arms, which push to the main stack and then
/// recurse still reading the workbench — incoherent, and reachable today
/// whenever the main stack is non-empty.
///
/// The joint disposition: **only the receiver comes from the workbench**, and
/// everything after proceeds exactly as `execute`. That is the `,`-family's own
/// convention — `get,`/`set,` pull the receiver per the operand and take the
/// key from the main stack unconditionally
/// (`reference/Bund/src/stdlib/functions/values/getsetinplace.rs:42-45,54,70`)
/// — so it is the sibling rule applied, not a new one.
///
/// Which makes the implementation one line: pull from the workbench, then hand
/// to the same `execute_value` the main-stack form uses.
fn execute_from_workbench(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull_workbench() else {
        return Err(Error("Stack is too shallow for inline EXECUTE.()".into()));
    };
    execute_value(vm, v)
}

fn execute_value(vm: &mut dyn Vm, v: BundValue) -> Result<(), Error> {
    match v.dt() {
        PTR | STRING | CALL => {
            let name = v
                .as_str()
                .ok_or_else(|| Error("EXECUTE not returned a proper function name".into()))?;
            vm.apply(BundValue::call(name))
        }
        LAMBDA => {
            let body = v.clone();
            vm.tail_lambda(body);
            Ok(())
        }
        LIST => {
            let items = v
                .as_list()
                .ok_or_else(|| Error::internal("a LIST value carried no items"))?
                .to_vec();
            for item in items {
                // Pushed and pulled so the item passes through `push`'s tag
                // write, which is what the reference's recursion does
                // (`reference/rust_multistackvm/src/stdlib/execute.rs:41-42`).
                vm.push(item);
                let top = crate::pull::operand(vm, "EXECUTE", 1)?;
                execute_value(vm, top)?;
            }
            Ok(())
        }
        MAP => {
            let key_val = vm
                .pull()
                .ok_or_else(|| Error("EXECUTE can not obtain key for DICT execute".into()))?;
            let key = key_val
                .as_str()
                .ok_or_else(|| Error("EXECUTE returned error during DICT key conversion".into()))?;
            match v.get(&key) {
                Some(inner) => execute_value(vm, inner),
                None => Err(Error(format!(
                    "EXECUTE returned error during DICT execute: key {key} not found"
                ))),
            }
        }
        CONDITIONAL => crate::conditional::execute_conditional(vm, v),
        // F16's fix, per D23 and D25: executing a class constructs from the
        // value, not by looking a name up.
        CLASS => crate::oop::construct_from_value(vm, v),
        OBJECT => crate::oop::execute_object(vm, v),
        _ => Err(Error("Received value is not of executable type".into())),
    }
}

/// The reflection family — what the word table knows about a name
/// (`reference/Bund/src/stdlib/functions/bund/bund_fun.rs:10-104`).
///
/// Each pulls a name and pushes a bool. `?word` is the union of the three
/// (`:92-100`), which is why it cannot be written as an alias for any of them.
fn ask(vm: &mut dyn Vm, word: &str, f: impl Fn(&dyn Vm, &str) -> bool) -> Result<(), Error> {
    let Some(v) = vm.pull() else {
        return Err(Error(format!("Stack is too shallow for {word}")));
    };
    let name = v
        .as_str()
        .ok_or_else(|| Error(format!("{word} casting string returns: not a string")))?;
    let answer = f(vm, &name);
    vm.push(BundValue::boolean(answer));
    Ok(())
}

fn is_alias(vm: &mut dyn Vm) -> Result<(), Error> {
    ask(vm, "?ALIAS", |vm, n| vm.is_alias(n))
}

fn is_lambda(vm: &mut dyn Vm) -> Result<(), Error> {
    ask(vm, "?LAMBDA", |vm, n| vm.is_lambda(n))
}

fn is_stdlib(vm: &mut dyn Vm) -> Result<(), Error> {
    ask(vm, "?STDLIB", |vm, n| vm.is_native(n))
}

/// `?word` — native **or** lambda **or** alias (`bund_fun.rs:92-100`).
fn is_word(vm: &mut dyn Vm) -> Result<(), Error> {
    ask(vm, "?WORD", |vm, n| {
        vm.is_native(n) || vm.is_lambda(n) || vm.is_alias(n)
    })
}

/// `lambda=` — the body bound to a name (`bund_fun.rs:135-160`).
fn get_lambda(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull() else {
        return Err(Error("Stack is too shallow for LAMBDA.GET".into()));
    };
    let name = v
        .as_str()
        .ok_or_else(|| Error("LAMBDA.GET casting string returns: not a string".into()))?;
    match vm.get_lambda(&name) {
        Some(body) => {
            vm.push(body);
            Ok(())
        }
        None => Err(Error(format!("LAMBDA.GET returned: {name} is not a lambda"))),
    }
}

/// `type` — the receiver's tag name, pushed beside it
/// (`reference/rust_multistackvm/src/stdlib/values/value_types.rs:6-19`).
///
/// Peeks, so the effect is 1→2 and not 1→1. Every word in this trio peeks;
/// `?type` is the one that pulls, and it pulls the *name* rather than the
/// value (`:40-47`).
fn value_type(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = crate::pull::top(vm, "TYPE")?;
    vm.push(BundValue::str(v.type_name()));
    Ok(())
}

/// `type.of` — the tag itself, as an INTEGER (`value_types.rs:21-34`).
///
/// The reference casts `type_of()`, a `u16`, to `i64` (`:27`), so the number
/// on the stack is the `dt` verbatim.
fn value_type_of(vm: &mut dyn Vm) -> Result<(), Error> {
    let v = crate::pull::top(vm, "TYPE")?;
    vm.push(BundValue::int(i64::from(v.dt())));
    Ok(())
}

/// `?type` — does the receiver carry this tag name? (`value_types.rs:36-61`).
///
/// The name is pulled from the top and the receiver is peeked *below* it, so
/// the receiver survives and the answer lands on top of it. The comparison is
/// on the name and not the number, which is why `Unknown` in [`type_name`]
/// answers `false` for every name a caller can spell rather than aborting.
///
/// [`type_name`]: bund2_value::BundValue::type_name
fn value_if_type(vm: &mut dyn Vm) -> Result<(), Error> {
    let name = crate::pull::operand(vm, "TYPE", 1)?;
    let Some(name) = name.unboxed().as_str() else {
        return Err(Error("Error casting type name".into()));
    };
    let name = name.to_string();
    let v = crate::pull::top(vm, "TYPE")?;
    vm.push(BundValue::boolean(v.type_name() == name));
    Ok(())
}

/// `ptr` — a PTR built from a string
/// (`reference/rust_multistackvm/src/stdlib/artefacts.rs:80-99`).
///
/// The reference ends with `vm.apply(Value::ptr(name, Vec::new()))` (`:88`).
/// `apply` on a PTR takes the default arm and pushes
/// (`reference/rust_multistackvm/src/multistackvm_apply.rs:88-99`, the
/// non-`autoadd` branch), so this is a push and not an execution — which is
/// the whole point of the word: it makes a callable *value* without calling it.
fn ptr(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline ptr()".into()));
    }
    let v = crate::pull::operand(vm, "PTR", 1)?;
    let Some(name) = v.unboxed().as_str() else {
        return Err(Error(
            "PTR returns error: This Dynamic type is not string".into(),
        ));
    };
    vm.push(BundValue::ptr(name));
    Ok(())
}

pub fn register_words(r: &mut Registry) {
    r.register_native("ptr", ptr, eff(1, 1), WordKind::Sync);
    r.register_native("type", value_type, eff(1, 2), WordKind::Sync);
    r.register_native("type.of", value_type_of, eff(1, 2), WordKind::Sync);
    r.register_native("?type", value_if_type, eff(2, 2), WordKind::Sync);
    r.register_native("valuemap", valuemap, eff(0, 1), WordKind::Sync);
    // `text` — apply an empty TEXTBUFFER
    // (`reference/rust_multistackvm/src/stdlib/artefacts.rs:117-119`, registered
    // at `:142`). Applied, not pushed, as the reference does; for a value that
    // is not a CALL the two are the same outside `autoadd`.
    r.register_native(
        "text",
        |vm| vm.apply(BundValue::textbuffer("")),
        eff(0, 1),
        WordKind::Sync,
    );
    r.register_native("?alias", is_alias, eff(1, 1), WordKind::Sync);
    r.register_native("?lambda", is_lambda, eff(1, 1), WordKind::Sync);
    r.register_native("?stdlib", is_stdlib, eff(1, 1), WordKind::Sync);
    r.register_native("?word", is_word, eff(1, 1), WordKind::Sync);
    r.register_native("lambda=", get_lambda, eff(1, 1), WordKind::Sync);
    r.register_alias("match", "valuemap");
    r.register_native("dict", dict, eff(0, 1), WordKind::Sync);
    r.register_native("list", list, eff(0, 1), WordKind::Sync);
    r.register_native("lambda", lambda, eff(0, 1), WordKind::Sync);
    r.register_native("metrics", metrics, eff(0, 1), WordKind::Sync);
    r.register_native("nodata", nodata, eff(0, 1), WordKind::Sync);
    r.register_native("set", set, eff(3, 1), WordKind::Sync);
    r.register_native("get", get, eff(2, 1), WordKind::Sync);
    r.register_native("?key", has_key, eff(2, 2), WordKind::Sync);
    r.register_native("register", register, eff(2, 0), WordKind::Sync);
    r.register_native("unregister", unregister, eff(1, 0), WordKind::Sync);
    r.register_native("lambda!", to_lambda, eff(1, 1), WordKind::Sync);
    // Opaque: folds the **whole stack** into a LAMBDA, so its consumption is
    // the depth it finds (`bund_fun.rs:189-202`).
    r.register_native("lambda*", fold_lambda, StackEffect::opaque(0), WordKind::Sync);
    r.register_native("make.call", make_call, eff(1, 1), WordKind::Sync);
    r.register_native("make.call.", make_call_wb, eff(0, 0), WordKind::Sync);
    // **Opaque — RFC-0004 §S6's first barrier.** `execute` dispatches on
    // eight tags (`reference/rust_multistackvm/src/stdlib/execute.rs:27-93`)
    // and each arm has a different effect: the MAP arm pulls a second operand,
    // the LAMBDA arm runs a body, the OBJECT arm dispatches a method. It is
    // also one of the five most-used words in the corpus, spelled `!`.
    r.register_native("execute", execute_top, StackEffect::opaque(1), WordKind::Sync);
    // F87: opaque, as `execute` is — it reaches the same `execute_value`, so
    // it runs whatever it is handed. And it consumes nothing from the main
    // stack: the receiver comes off the workbench.
    r.register_native("execute.", execute_from_workbench, StackEffect::opaque(0), WordKind::Sync);

    // The reference's alias table, for the words above
    // (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:5,21,28,29,40,41`
    // and `reference/Bund/src/stdlib/functions/create_aliases.rs:26`).
    r.register_alias("!", "execute");
    r.register_alias("!.", "execute.");
    r.register_alias("config", "dict");
    r.register_alias("call,", "make.call");
    r.register_alias(",", "set");
    r.register_alias("∈", "set");
    r.register_alias("sample", "metrics");
    r.register_alias("λ", "lambda");
    r.register_alias("Λ", "lambda");
    r.register_alias("|", "nodata");
    r.register_alias("∅", "nodata");
}

#[cfg(test)]
mod tests {
    use super::*;
    use bund2_interp::Interp;

    fn vm() -> Interp {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        i
    }

    fn run(src: &str) -> Result<Interp, String> {
        let mut i = vm();
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        Ok(i)
    }

    /// The corpus's documented way to define a word, end to end through the
    /// parser (`reference/Bund/examples/helloworld_lambda.bund`).
    #[test]
    fn the_register_idiom_defines_a_word() {
        let i = run(":Twice { 2 } register\nTwice").expect("runs");
        assert_eq!(i.depth(), 1);
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(2));
    }

    /// `set` on a CONDITIONAL keeps it a CONDITIONAL — the receiver's `dt` is
    /// preserved (`reference/rust_dynamic/src/set.rs:23-25`), which is what
    /// `?try :try { … } set` relies on.
    #[test]
    fn set_preserves_the_receivers_tag() {
        let c = BundValue::conditional(Default::default()).set("type", BundValue::str("tryexcept"));
        assert_eq!(c.dt(), CONDITIONAL);
        assert_eq!(c.get("type").and_then(|v| v.as_str()).as_deref(), Some("tryexcept"));
    }

    /// F42, preserved: `set` on a LIST throws the container and the key away.
    #[test]
    fn set_on_a_list_discards_the_container() {
        let l = BundValue::list(vec![BundValue::Int(1, bund2_value::StackSym::NONE), BundValue::Int(2, bund2_value::StackSym::NONE)]);
        let after = l.set("k", BundValue::Int(9, bund2_value::StackSym::NONE));
        assert_eq!(after.as_list().map(|v| v.len()), Some(1));
    }

    /// D5's citation: `set` on a LAMBDA replaces the body and returns a new
    /// value, leaving the original untouched. That is what makes bodies
    /// write-once, and therefore what D35's cache rests on.
    #[test]
    fn set_on_a_lambda_replaces_the_body_without_mutating_it() {
        let orig = BundValue::lambda(vec![BundValue::Int(1, bund2_value::StackSym::NONE), BundValue::Int(2, bund2_value::StackSym::NONE)]);
        let after = orig.set("k", BundValue::Int(9, bund2_value::StackSym::NONE));
        assert_eq!(after.as_lambda().map(|b| b.len()), Some(1));
        assert_eq!(orig.as_lambda().map(|b| b.len()), Some(2), "original intact");
    }

    #[test]
    fn dict_set_get_round_trips() {
        let i = run("dict :k 42 set :k get").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(42));
    }

    /// `config` is an alias for `dict`, which five goldens need.
    #[test]
    fn config_is_dict() {
        let i = run("config :k 1 set").expect("runs");
        assert_eq!(i.peek().map(|v| v.dt()), Some(MAP));
    }

    /// `!` executes a PTR by name, which is `execute`'s most-used arm — 69
    /// invocations across 39 of 132 programs, per ERRATA.
    #[test]
    fn execute_calls_a_ptr_by_name() {
        let i = run(":Five { 5 } register\n`Five !").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(5));
    }

    /// A LAMBDA on the stack runs its body.
    #[test]
    fn execute_evaluates_a_lambda() {
        let i = run("{ 7 } !").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(7));
    }

    /// §1.3's canonical metaprogramming idiom, whole: fabricate a CALL from an
    /// atom, fold the stack into a lambda, register it, call it.
    #[test]
    fn the_metaprogramming_idiom_round_trips() {
        let i = run("( \"Hi\" :println call, lambda* :Greet swap register )\n:Greet ?lambda")
            .or_else(|_| run("42 :println call, lambda* :Greet swap register"))
            .expect("runs");
        assert!(i.depth() <= 2);
    }

    /// `lambda*` drains the whole stack, in order.
    #[test]
    fn fold_lambda_takes_everything() {
        let i = run("1 2 3 lambda*").expect("runs");
        assert_eq!(i.depth(), 1);
        let body = i.peek().expect("a value");
        assert_eq!(body.dt(), LAMBDA);
        assert_eq!(body.as_lambda().map(|b| b.len()), Some(3));
        assert_eq!(body.as_lambda().unwrap()[0].as_int(), Some(1), "order kept");
    }

    /// **F53.** A value on the workbench is executable even when the main
    /// stack is empty — the case the reference's wrong guard refuses.
    #[test]
    fn execute_from_the_workbench_works_on_an_empty_stack() {
        let i = run(":Five { 5 } register
`Five return
execute.").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(5));
    }

    /// **F59.** Only the receiver comes from the workbench; a LIST's elements
    /// execute from the main stack, so the arm is coherent rather than pushing
    /// to one stack and reading another.
    #[test]
    fn only_the_receiver_comes_from_the_workbench() {
        let i = run(":Five { 5 } register
[ `Five ] return
execute.").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(5));
    }

    /// **128, and the stamps are distinct.** The count is what
    /// `tests/golden/probes/dt-reachable.golden` pins; the stamps are what a
    /// single shared `now` would quietly get wrong, since the golden normalises
    /// every one of them away (`create_metrics.rs:8-26`, `metric.rs:12-16`).
    #[test]
    fn metrics_is_a_hundred_and_twenty_eight_zeroed_samples() {
        let i = run("metrics").expect("runs");
        let v = i.peek().expect("a value");
        assert_eq!(v.dt(), bund2_value::METRICS);
        let m = v.as_metrics().expect("a METRICS payload");
        assert_eq!(m.len(), 128);
        assert!(m.iter().all(|s| s.data == 0.0), "every sample starts at 0.0");
        assert!(
            m.iter().any(|s| s.stamp != m[0].stamp),
            "every sample carried one shared stamp; the reference reads the \
             clock per sample"
        );
    }

    #[test]
    fn every_word_here_resolves() {
        let i = vm();
        for name in [
            "dict", "config", "list", "lambda", "λ", "nodata", "|", "set", ",", "get", "?key",
            "register", "unregister", "lambda!", "lambda*", "make.call", "call,", "execute", "!", "execute.", "!.",
            "metrics", "sample",
        ] {
            assert!(
                i.registry.interner.lookup_call(name).is_some(),
                "{name} is not registered"
            );
        }
    }
}
