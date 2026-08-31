//! Containers, the word table, and `execute` — the vocabulary the corpus's
//! block idiom needs.
//!
//! Three of the four most-demanded words across the goldens live here:
//! `set` (29 goldens), `!` (24) and `register` (22).

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, CALL, CLASS, CONDITIONAL, LAMBDA, LIST, MAP, OBJECT, PTR, STRING};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect { consumes, produces }
}

/// `dict` — an empty MAP (`reference/rust_multistackvm/src/stdlib/artefacts.rs:113-115`).
/// `config` is an alias for it
/// (`reference/Bund/src/stdlib/functions/create_aliases.rs:26`).
fn dict(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::map(Default::default()));
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

/// `nodata`. Aliased `|` and `∅` (`create_aliases.rs:40-41`).
fn nodata(vm: &mut dyn Vm) -> Result<(), Error> {
    vm.push(BundValue::Nodata);
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
    vm.push(BundValue::Bool(present));
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
    match body.dt() {
        LAMBDA | CLASS => {
            vm.register_lambda(&name, body);
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
    let Some(v) = vm.pull() else {
        return Err(Error("Stack is too shallow for inline MAKE.CALL".into()));
    };
    let name = v
        .as_str()
        .ok_or_else(|| Error("MAKE.CALL casting of string returned: not a string".into()))?;
    vm.push(BundValue::call(name));
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
fn execute(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull() else {
        return Err(Error("Stack is too shallow for inline execute()".into()));
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
            let body = v
                .as_lambda()
                .ok_or_else(|| Error::internal("a LAMBDA value carried no body"))?
                .to_vec();
            vm.eval_body(&body)
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
        CLASS | OBJECT => Err(Error(
            "EXECUTE: the class and object arms need the object model (RFC-0009)".into(),
        )),
        _ => Err(Error("Received value is not of executable type".into())),
    }
}

pub fn register_words(r: &mut Registry) {
    r.register_native("dict", dict, eff(0, 1), WordKind::Sync);
    r.register_native("list", list, eff(0, 1), WordKind::Sync);
    r.register_native("lambda", lambda, eff(0, 1), WordKind::Sync);
    r.register_native("nodata", nodata, eff(0, 1), WordKind::Sync);
    r.register_native("set", set, eff(3, 1), WordKind::Sync);
    r.register_native("get", get, eff(2, 1), WordKind::Sync);
    r.register_native("?key", has_key, eff(2, 2), WordKind::Sync);
    r.register_native("register", register, eff(2, 0), WordKind::Sync);
    r.register_native("unregister", unregister, eff(1, 0), WordKind::Sync);
    r.register_native("lambda!", to_lambda, eff(1, 1), WordKind::Sync);
    r.register_native("lambda*", fold_lambda, eff(0, 1), WordKind::Sync);
    r.register_native("make.call", make_call, eff(1, 1), WordKind::Sync);
    r.register_native("execute", execute, eff(1, 0), WordKind::Sync);

    // The reference's alias table, for the words above
    // (`reference/rust_multistackvm/src/stdlib/create_aliases.rs:5,21,28,29,40,41`
    // and `reference/Bund/src/stdlib/functions/create_aliases.rs:26`).
    r.register_alias("!", "execute");
    r.register_alias("config", "dict");
    r.register_alias("call,", "make.call");
    r.register_alias(",", "set");
    r.register_alias("∈", "set");
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
        let l = BundValue::list(vec![BundValue::Int(1), BundValue::Int(2)]);
        let after = l.set("k", BundValue::Int(9));
        assert_eq!(after.as_list().map(|v| v.len()), Some(1));
    }

    /// D5's citation: `set` on a LAMBDA replaces the body and returns a new
    /// value, leaving the original untouched. That is what makes bodies
    /// write-once, and therefore what D35's cache rests on.
    #[test]
    fn set_on_a_lambda_replaces_the_body_without_mutating_it() {
        let orig = BundValue::lambda(vec![BundValue::Int(1), BundValue::Int(2)]);
        let after = orig.set("k", BundValue::Int(9));
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

    #[test]
    fn every_word_here_resolves() {
        let i = vm();
        for name in [
            "dict", "config", "list", "lambda", "λ", "nodata", "|", "set", ",", "get", "?key",
            "register", "unregister", "lambda!", "lambda*", "make.call", "call,", "execute", "!",
        ] {
            assert!(
                i.registry.interner.lookup_call(name).is_some(),
                "{name} is not registered"
            );
        }
    }
}
