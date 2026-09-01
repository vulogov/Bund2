//! Classes, objects and method dispatch — **RFC-0009**.
//!
//! A class is a MAP tagged `CLASS` carrying `.class_name`, a `.super` list of
//! parent **names**, and one slot per method. Constructing an object copies the
//! class, flips the tag to `OBJECT`, and rebuilds `.super` as a list of
//! constructed parent **objects** — so a class's `.super` holds names and an
//! instance's holds objects, an asymmetry read by the same code and preserved.
//!
//! Dispatch resolves a slot by walking that tree depth-first, first match wins
//! (`reference/rust_multistackvm/src/multistackvm_object.rs:6-24`). A `PTR` in
//! a slot resolves through the method table and is **called**; a `LAMBDA` is
//! evaluated; anything else is pushed (`:57-82`).
//!
//! # The one deviation
//!
//! `<class> !` constructs. In the reference it always errors, because
//! `execute_class` pushes the class *value* while `stdlib_object_inline` wants
//! a *name* (F16). D23 says build from the value; D25 says take `.class_name`
//! from the class itself and fail if it carries none.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, LAMBDA, OBJECT, PTR};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect { consumes, produces }
}

// --- construction -----------------------------------------------------------

/// Depth-first, first match wins — `locate_value_in_object`
/// (`reference/rust_multistackvm/src/multistackvm_object.rs:6-24`).
///
/// Tries the object's own map, then recurses into each element of `.super`.
/// On an instance those elements are objects; on a class they are name
/// strings, which carry no slots, so the same walk serves both.
fn locate(value: &BundValue, name: &str) -> Option<BundValue> {
    if let Some(v) = value.get(name) {
        return Some(v);
    }
    let supers = value.get(".super")?;
    for s in supers.as_list()? {
        if let Some(v) = locate(s, name) {
            return Some(v);
        }
    }
    None
}

/// Build an object from a class value.
///
/// `name` is the class name to stamp. Parents come from the **registry** by
/// name, which is why a class whose parents are unregistered fails here rather
/// than at registration (§S1).
///
/// **Depth is bounded by an explicit budget**, not by the Rust stack. The
/// reference recurses (`bund_object.rs:50`), which is the third depth axis
/// RFC-0003 §S4 names; until the frame loop exists this keeps the recursion
/// shallow enough to report rather than abort, satisfying D37.
fn make_object(vm: &mut dyn Vm, name: &str, class: &BundValue, budget: usize) -> Result<BundValue, Error> {
    if budget == 0 {
        return Err(Error(format!(
            "OBJECT class {name}: hierarchy is deeper than the construction budget"
        )));
    }
    // Copy the class's slots and flip the tag. `set` then rebuilds the value
    // anyway, which is where the instance's identity comes from — not from
    // this copy (`reference/rust_dynamic/src/set.rs:14-27`).
    let mut obj = BundValue::with_dt(
        OBJECT,
        bund2_value::Payload::Map(class.as_map().cloned().unwrap_or_default()),
    );
    obj = obj.set(".class_name", BundValue::str(name));

    // Rebuild `.super`: names in, objects out.
    let mut parents: Vec<BundValue> = Vec::new();
    if let Some(list) = class.get(".super").and_then(|v| v.as_list().map(<[BundValue]>::to_vec)) {
        for p in list {
            let Some(pname) = p.as_str() else { continue };
            let Some(pclass) = vm.class(&pname) else {
                // **F67: name the parent, not the child.** The reference
                // interpolates the class being constructed
                // (`reference/rust_multistackvm/src/stdlib/bund_object.rs:99`),
                // sending the reader to the wrong end of the hierarchy.
                return Err(Error(format!("OBJECT class {pname} not registered")));
            };
            let pobj = make_object(vm, &pname, &pclass, budget - 1)?;
            run_init(vm, &pobj)?;
            parents.push(pobj);
        }
    }
    obj = obj.set(".super", BundValue::list(parents));
    Ok(obj)
}

/// Evaluate an object's `.init`, if it has one that can be run.
///
/// **A PTR naming an unregistered method is silently skipped**, as the
/// reference does — construction succeeds with an uninitialised object rather
/// than failing. Preserved, and stated because it is easy to read as a bug.
fn run_init(vm: &mut dyn Vm, obj: &BundValue) -> Result<(), Error> {
    let Some(init) = obj.get(".init") else {
        return Ok(());
    };
    match init.dt() {
        LAMBDA => {
            let body = init
                .as_lambda()
                .ok_or_else(|| Error::internal("a value tagged LAMBDA carried no body"))?
                .to_vec();
            vm.eval_body(&body)
        }
        PTR => {
            let Some(mname) = init.as_str() else {
                return Ok(());
            };
            match vm.method(&mname) {
                Some(f) => f(vm),
                None => Ok(()),
            }
        }
        _ => Ok(()),
    }
}

// --- the words --------------------------------------------------------------

/// `class` — a bare CLASS with an empty `.super`
/// (`reference/rust_multistackvm/src/stdlib/artefacts.rs:69-73`).
///
/// Arity is 0→1: it does **not** consume the name atom. `register` takes the
/// name from beneath the class, which is why the idiom is `:Name class …
/// register` and why `3.14 :Answer dup class` works.
fn class_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let c = BundValue::class(Default::default()).set(".super", BundValue::list(Vec::new()));
    vm.push(c);
    Ok(())
}

/// `object` — construct from a **registered** class, by name.
fn object_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(nv) = vm.pull() else {
        return Err(Error("Stack is too shallow for inline object()".into()));
    };
    let name = nv
        .as_str()
        .ok_or_else(|| Error("OBJECT returns error: This Dynamic type is not string".into()))?;
    let Some(class) = vm.class(&name) else {
        return Err(Error(format!("OBJECT class {name} not registered")));
    };
    let obj = make_object(vm, &name, &class, 512)?;
    vm.push(obj.clone());
    run_init(vm, &obj)?;
    Ok(())
}

/// `<class> !` — **D23 and D25**, completing F16's FIX.
///
/// Builds from the CLASS value on the stack, whatever its provenance, and
/// takes `.class_name` from the value itself. A class carrying none fails
/// here, because an OBJECT without a class name is malformed: `.str` reads it,
/// and so does the object-reuse test.
pub fn construct_from_value(vm: &mut dyn Vm, class: BundValue) -> Result<(), Error> {
    let name = class.get(".class_name").and_then(|v| v.as_str()).ok_or_else(|| {
        Error(
            "OBJECT: the class carries no .class_name, so the object would have none. \
             Set one on the class, as every built-in class does."
                .into(),
        )
    })?;
    let obj = make_object(vm, &name, &class, 512)?;
    vm.push(obj.clone());
    run_init(vm, &obj)
}

/// `!` on an OBJECT — pull a method **name** and dispatch
/// (`reference/rust_multistackvm/src/stdlib/bund_execute/execute_object.rs:8-20`).
pub fn execute_object(vm: &mut dyn Vm, obj: BundValue) -> Result<(), Error> {
    let Some(nv) = vm.pull() else {
        return Err(Error("EXECUTE.OBJECT NO DATA #1".into()));
    };
    let name = nv
        .as_str()
        .ok_or_else(|| Error("EXECUTE.OBJECT casting name returns: not a string".into()))?;
    vm.push(obj);
    dispatch_method(vm, &name)
}

/// `m(name)` — resolve a slot on the object at the top and act on what it
/// holds (`reference/rust_multistackvm/src/multistackvm_object.rs:43-86`).
///
/// **Peeks, not pulls**: the receiver stays for the method to use.
pub fn dispatch_method(vm: &mut dyn Vm, name: &str) -> Result<(), Error> {
    let Some(recv) = vm.peek() else {
        return Err(Error("VM stack is empty".into()));
    };
    if recv.dt() != OBJECT {
        return Err(Error("VM there is no value of type OBJECT in the stack".into()));
    }
    let Some(slot) = locate(&recv, name) else {
        return Err(Error(format!("VM no method {name} has been registered")));
    };
    match slot.dt() {
        PTR => {
            let mname = slot
                .as_str()
                .ok_or_else(|| Error::internal("a PTR slot carried no name"))?;
            match vm.method(&mname) {
                Some(f) => f(vm),
                None => Err(Error(format!("m({name}) returned: method {mname} not registered"))),
            }
        }
        LAMBDA => {
            let body = slot
                .as_lambda()
                .ok_or_else(|| Error::internal("a value tagged LAMBDA carried no body"))?
                .to_vec();
            vm.eval_body(&body)
        }
        _ => {
            vm.push(slot);
            Ok(())
        }
    }
}

fn is_class_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull() else {
        return Err(Error("Stack is too shallow for ?CLASS".into()));
    };
    let name = v.as_str().unwrap_or_default();
    let answer = vm.is_class(&name);
    vm.push(BundValue::Bool(answer));
    Ok(())
}

fn is_object_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull() else {
        return Err(Error("Stack is too shallow for ?OBJECT".into()));
    };
    vm.push(BundValue::Bool(v.dt() == OBJECT));
    Ok(())
}

// --- the base hierarchy -----------------------------------------------------

/// `.id` — the receiver's identity as a 21-character string
/// (`reference/Bund/src/stdlib/functions/oop/base_classes.rs:11-18`).
///
/// Peeks, and pushes beside the receiver. D1's laziness ends here, which is the
/// consumer that proved identity had to stay answerable rather than be dropped.
fn method_id(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.peek() else {
        return Err(Error("Stack is empty for method '.id'".into()));
    };
    let (s, promoted) = v.id_string();
    if let Some(p) = promoted {
        vm.pull();
        vm.push(p);
    }
    vm.push(BundValue::str(s));
    Ok(())
}

/// `.timestamp` (`base_classes.rs:20-27`). D2's laziness ends here.
fn method_timestamp(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.peek() else {
        return Err(Error("Stack is empty for method '.timestamp'".into()));
    };
    let (t, promoted) = v.timestamp();
    if let Some(p) = promoted {
        vm.pull();
        vm.push(p);
    }
    vm.push(BundValue::Float(t));
    Ok(())
}

/// `.str` — the object's `.data` as a string, or `Object(<class>)`
/// (`base_classes.rs:29-79`).
fn method_str(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.peek() else {
        return Err(Error("Stack is empty for method '.str'".into()));
    };
    let text = match locate(&v, ".data") {
        Some(d) => d.display(),
        None => {
            let cl = v
                .get(".class_name")
                .and_then(|c| c.as_str())
                .unwrap_or_default();
            format!("Object({cl})")
        }
    };
    vm.push(BundValue::str(text));
    Ok(())
}

fn method_println(vm: &mut dyn Vm) -> Result<(), Error> {
    method_str(vm)?;
    let Some(v) = vm.pull() else {
        return Err(Error::internal("`.str` pushed nothing for `.println`"));
    };
    println!("{}", v.display());
    Ok(())
}

fn method_print(vm: &mut dyn Vm) -> Result<(), Error> {
    method_str(vm)?;
    let Some(v) = vm.pull() else {
        return Err(Error::internal("`.str` pushed nothing for `.print`"));
    };
    print!("{}", v.display());
    Ok(())
}

/// The base chain, exactly as the reference builds it: `Object` → `Printable`
/// → `Display` (`base_classes.rs:90-116`).
///
/// Note the slot/method asymmetry it preserves: `Object` maps the slot `.id` to
/// the method `` `.id ``, while `Printable` maps the slot `str` to `` `.str ``.
/// The leading dot belongs to the method's name, not the slot's.
fn register_base(r: &mut Registry) {
    r.register_method(".id", method_id);
    r.register_method(".timestamp", method_timestamp);
    r.register_method(".str", method_str);
    r.register_method(".print", method_print);
    r.register_method(".println", method_println);

    let display = BundValue::class(Default::default())
        .set(".class_name", BundValue::str("Display"))
        .set(".super", BundValue::list(Vec::new()));
    r.register_class("Display", display);

    let printable = BundValue::class(Default::default())
        .set(".class_name", BundValue::str("Printable"))
        .set(".super", BundValue::list(vec![BundValue::str("Display")]))
        .set("str", BundValue::ptr(".str"))
        .set("print", BundValue::ptr(".print"))
        .set("println", BundValue::ptr(".println"));
    r.register_class("Printable", printable);

    let object = BundValue::class(Default::default())
        .set(".class_name", BundValue::str("Object"))
        .set(".super", BundValue::list(vec![BundValue::str("Printable")]))
        .set(".id", BundValue::ptr(".id"))
        .set(".timestamp", BundValue::ptr(".timestamp"));
    r.register_class("Object", object);
}

pub fn register(r: &mut Registry) {
    r.register_native("class", class_word, eff(0, 1), WordKind::Sync);
    r.register_native("object", object_word, eff(1, 1), WordKind::Sync);
    r.register_native("?class", is_class_word, eff(1, 1), WordKind::Sync);
    r.register_native("?object", is_object_word, eff(1, 1), WordKind::Sync);
    register_base(r);
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

    /// `create_class_demo.bund`'s idiom: `class` does not consume the atom.
    #[test]
    fn the_class_idiom_registers_and_is_found() {
        let i = run_src(":A dup class :hello \"hi\" set register ?class").expect("runs");
        assert_eq!(*i.peek().unwrap().unboxed(), BundValue::Bool(true));
    }

    /// A class lands in the class registry, **not** the word table — which is
    /// why `:Probe register` then `Probe` reports an unregistered word.
    #[test]
    fn a_class_is_not_a_word() {
        let i = run_src(":A class register").expect("runs");
        assert!(i.is_class("A"));
        assert!(!i.is_lambda("A"));
    }

    #[test]
    fn an_object_carries_its_class_name() {
        let i = run_src(":A class :hello \"hi\" set register :A object").expect("runs");
        let o = i.peek().expect("an object");
        assert_eq!(o.dt(), OBJECT);
        assert_eq!(o.get(".class_name").and_then(|v| v.as_str()).as_deref(), Some("A"));
    }

    /// **`.super` holds names on a class and objects on an instance** — the
    /// asymmetry §S2 preserves.
    #[test]
    fn super_holds_names_on_a_class_and_objects_on_an_instance() {
        let i = run_src(
            ":A class :.class_name \"A\" set register\n\
             :B class \".super\" [ :A ] set register\n\
             :B object",
        )
        .expect("runs");
        let obj = i.peek().expect("an object");
        let sup = obj.get(".super").expect(".super");
        let first = &sup.as_list().expect("a list")[0];
        assert_eq!(first.dt(), OBJECT, "an instance's parents are objects");
        let class = i.class("B").expect("class B");
        let csup = class.get(".super").expect(".super");
        assert!(
            csup.as_list().expect("a list")[0].as_str().is_some(),
            "a class's parents are names"
        );
    }

    /// **Registration does not require the parents** — so flattening at
    /// registration would narrow the language (§S1). Construction is where it
    /// fails, and F67 says it must name the *parent*.
    #[test]
    fn a_class_registers_without_its_parents_and_fails_at_construction() {
        let i = run_src(":B class \".super\" [ :A ] set register").expect("registers");
        assert!(i.is_class("B"));
        let e = err_of(":B class \".super\" [ :A ] set register :B object");
        assert!(e.contains("class A not registered"), "names the parent: {e}");
    }

    /// Dispatch walks `.super` depth-first and calls a PTR through the method
    /// table. `.id` lives on `Object`, two levels above a user class.
    #[test]
    fn dispatch_reaches_an_inherited_method() {
        let i = run_src(
            ":A class \".super\" [ :Object ] set register\n:.id :A object !",
        )
        .expect("runs");
        let id = i.peek().and_then(|v| v.as_str()).expect("an id");
        assert_eq!(id.chars().count(), 21, "a nanoid-shaped string: {id}");
    }

    /// Two instances of one class have different identities.
    #[test]
    fn instances_have_distinct_identities() {
        let i = run_src(
            ":A class \".super\" [ :Object ] set register\n\
             :.id :A object !\n:.id :A object !",
        )
        .expect("runs");
        let s = i.snapshot();
        let a = s.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>();
        assert!(a.len() >= 2, "{a:?}");
        assert_ne!(a[a.len() - 1], a[a.len() - 2], "distinct: {a:?}");
    }

    /// A LAMBDA slot is evaluated rather than called through the method table.
    #[test]
    fn a_lambda_slot_is_evaluated() {
        let i = run_src(":A class :greet { 42 } set register\n:greet :A object !").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(42));
    }

    /// **D23/D25, and F16's fix**: `<class> !` constructs from the value.
    #[test]
    fn executing_a_class_value_constructs() {
        let i = run_src(":A class :.class_name \"A\" set :greet { 7 } set !").expect("runs");
        let o = i.peek().expect("an object");
        assert_eq!(o.dt(), OBJECT);
        assert_eq!(o.get(".class_name").and_then(|v| v.as_str()).as_deref(), Some("A"));
    }

    /// D25: a class with no `.class_name` fails rather than producing an
    /// object without one.
    #[test]
    fn an_anonymous_class_refuses_to_construct() {
        let e = err_of(":A class :greet { 7 } set !");
        assert!(e.contains("no .class_name"), "{e}");
    }

    /// Construction is bounded, so a deep chain reports rather than aborting —
    /// D37. The frame loop (§S3) replaces the bound with real frames.
    #[test]
    fn a_hierarchy_deeper_than_the_budget_reports_rather_than_aborts() {
        let mut src = String::from(":C0 class :.class_name \"C0\" set register\n");
        for n in 1..600 {
            src.push_str(&format!(
                ":C{n} class :.class_name \"C{n}\" set \".super\" [ :C{p} ] set register\n",
                n = n,
                p = n - 1
            ));
        }
        src.push_str(":C599 object\n");
        let e = err_of(&src);
        assert!(e.contains("deeper than the construction budget"), "{e}");
    }

    #[test]
    fn every_word_here_resolves() {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        for n in ["class", "object", "?class", "?object"] {
            assert!(i.registry.interner.lookup_call(n).is_some(), "{n}");
        }
        for c in ["Object", "Printable", "Display"] {
            assert!(i.registry.is_class(c), "{c} is not registered");
        }
        for m in [".id", ".timestamp", ".str", ".print", ".println"] {
            assert!(i.registry.is_method(m), "{m} is not a method");
        }
    }
}
