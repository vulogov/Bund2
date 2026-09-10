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
    StackEffect::fixed(consumes, produces)
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
            // **The parent goes on the stack before its `.init` runs, and
            // what comes back off is what gets stored.** The reference pushes
            // it (`bund_object.rs:56`), runs `.init`, then — if the top is
            // still an object of that class — pulls and stores *that*
            // (`:83-88`), falling back to the unpushed value otherwise
            // (`:89-91`).
            //
            // Both halves matter. An `.init` reaches its receiver only
            // through the stack, so without the push it initialises nothing;
            // and the pulled value carries the `stack` tag that `push` writes,
            // which is why `create_class_hierarhy_demo`'s captured parent
            // reads `tags: {"stack": "main"}` where Bund2's read `tags: {}`.
            vm.push(pobj.clone());
            run_init(vm, &pobj)?;
            parents.push(match top_is_object_of_class(vm, &pname) {
                true => vm.pull().unwrap_or(pobj),
                false => pobj,
            });
        }
    }
    obj = obj.set(".super", BundValue::list(parents));
    Ok(obj)
}

/// Is the top of the stack an OBJECT whose `.class_name` is `name`?
///
/// The reference's `if_object_of_class_in_stack`
/// (`reference/rust_multistackvm/src/stdlib/bund_object.rs:6-25`). It peeks,
/// and answers `false` for an empty stack, a non-OBJECT, or a missing or
/// non-string `.class_name` — never an error. That tolerance is the point: it
/// is asked after arbitrary user code has run, and any of those states is a
/// legitimate answer of "no".
fn top_is_object_of_class(vm: &mut dyn Vm, name: &str) -> bool {
    vm.peek().is_some_and(|v| {
        v.dt() == OBJECT && v.get(".class_name").and_then(|c| c.as_str()).as_deref() == Some(name)
    })
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
            let body = init.clone();
            vm.eval_lambda(&body)
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

/// Push the finished object, run its own `.init`, and leave the object on the
/// stack — the tail `stdlib_object_inline` shares with `make_bund_object`'s
/// parent loop (`reference/rust_multistackvm/src/stdlib/bund_object.rs:132-172`).
///
/// **The re-push is not redundant.** An `.init` reaches its receiver through
/// the stack, and most of them *consume* it: `class_constructors_demo.bund`'s
/// is `{ :.class_name get … }`, and `get` pulls its container. So after a
/// typical `.init` the object is gone, `if_object_of_class_in_stack` answers
/// false, and the reference applies the value it kept
/// (`:167-171`) — which is how `:A object` still leaves an object behind. An
/// earlier version of this pushed once and stopped, so every constructor that
/// read a field silently emptied the stack.
///
/// `apply` on an OBJECT is a plain push (`multistackvm_apply.rs:88-99`, the
/// non-`autoadd` arm), so this spells the push rather than routing through an
/// `apply` Bund2 would only use here.
fn push_and_init(vm: &mut dyn Vm, name: &str, obj: BundValue) -> Result<(), Error> {
    vm.push(obj.clone());
    run_init(vm, &obj)?;
    let finished = match top_is_object_of_class(vm, name) {
        true => vm.pull().unwrap_or(obj),
        false => obj,
    };
    vm.push(finished);
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
    push_and_init(vm, &name, obj)
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
    push_and_init(vm, &name, obj)
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
            let body = slot.clone();
            vm.eval_lambda(&body)
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
    vm.push(BundValue::boolean(answer));
    Ok(())
}

/// `?object` — is the top an OBJECT? **Peeks**, unlike `?class`.
///
/// The asymmetry is the reference's, not a slip: `?class` takes a *name* and
/// consumes it (`reference/Bund/src/stdlib/functions/bund/bund_class.rs:15`),
/// while `?object` inspects a *value* and leaves it
/// (`:39`). `create_object.bund`'s closing idiom depends on it —
/// `object ?object { … } if` tests the object it just built and expects it
/// still to be there afterwards, which is why the golden's captured stack
/// holds an object and Bund2's held nothing.
fn is_object_word(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.peek() else {
        return Err(Error("Stack is too shallow for ?OBJECT".into()));
    };
    vm.push(BundValue::boolean(v.dt() == OBJECT));
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
    vm.push(BundValue::float(t));
    Ok(())
}

/// `.format` — render the object's `.template`, peeking
/// (`reference/Bund/src/stdlib/functions/oop/display_class.rs:14-85`).
///
/// Pulls the receiver, then **pushes it back and pushes the rendered text**
/// (`:69-70`), so the effect is 1→2 like `.id`. A non-OBJECT is simply
/// converted to a string (`:72-82`), which is how `display` on a scalar works.
///
/// Keys are filled from the object's own slots, and a key the object does not
/// carry is **pulled from the stack** (`:38-44`) — which is why
/// `3.14 :display :Answer … !` puts `3.14` on the stack first: the template
/// names `{Pi}`, the class has no `Pi`, and the stack supplies it.
///
/// The text goes through `termimad::term_text` (`:65`), whose output wraps to
/// the terminal width — see the note on the dependency in `Cargo.toml`.
fn method_format(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(v) = vm.pull() else {
        return Err(Error("Stack is empty for method '.format'".into()));
    };
    if v.dt() != OBJECT {
        let text = v.display();
        vm.push(v);
        vm.push(BundValue::str(text));
        return Ok(());
    }
    let tpl = v
        .get(".template")
        .and_then(|t| t.as_str())
        .ok_or_else(|| Error("'.format' NO TEMPLATE".into()))?;
    let template = leon::Template::parse(tpl.as_str())
        .map_err(|e| Error(format!("FMT.STR error parsing template: {e}")))?;
    let mut values: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for name in template.keys() {
        if values.contains_key(*name) {
            continue;
        }
        let filled = match v.get(name) {
            Some(a) => a.display(),
            None => vm
                .pull()
                .ok_or_else(|| Error(format!("'.format' can not resolve {name}")))?
                .display(),
        };
        values.insert(name.to_string(), filled);
    }
    let res = template
        .render(&values)
        .map_err(|e| Error(format!("FMT.STR error rendering: {e}")))?;
    vm.push(v);
    vm.push(BundValue::str(format!("{}", termimad::term_text(&res))));
    Ok(())
}

/// `.display` — `.format`, then print
/// (`reference/Bund/src/stdlib/functions/oop/display_class.rs:87-94`).
///
/// It calls `.format` and then `stdlib_print_inline` — `print`, not `println`
/// (`:93`), so no newline is added and the object is left on the stack.
fn method_display(vm: &mut dyn Vm) -> Result<(), Error> {
    method_format(vm)?;
    let out = vm
        .pull()
        .ok_or_else(|| Error("'.display' produced no text".into()))?;
    print!("{}", out.display());
    use std::io::Write;
    let _ = std::io::stdout().flush();
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

    r.register_method(".format", method_format);
    r.register_method(".display", method_display);

    let display = BundValue::class(Default::default())
        .set(".class_name", BundValue::str("Display"))
        .set(".super", BundValue::list(Vec::new()))
        .set("format", BundValue::ptr(".format"))
        .set("display", BundValue::ptr(".display"));
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

// --- the wrapped-value classes ----------------------------------------------
//
// `Value`, `Integer` and `Bool` — a value carried *inside* an object, in the
// slot `.data`, with `wrap`, `unwrap`, `is` and `#` as the access words.
//
// The reference splits these across one file per class
// (`reference/Bund/src/stdlib/functions/oop/value_class.rs`, `int_class.rs`,
// `bool_class.rs`), but they are one mechanism: `Value` owns `.data` and the
// two subclasses differ only in the tag their `.init` converts it to.

/// `set_value_in_object` — `locate`'s mirror, and **not** a plain `set`
/// (`reference/Bund/src/stdlib/functions/oop/value_class.rs:31-52`).
///
/// A subclass instance does not hold `.data` itself; it inherits the slot from
/// the `Value` object sitting in its `.super` list. So writing it means finding
/// which object in the tree owns it, rebuilding that one, and rebuilding the
/// `.super` chain above it — a `set` on the outer object would create a
/// *second* `.data` shadowing the real one, and `unwrap` would then return
/// whichever the depth-first walk reached first.
///
/// **A name owned by nobody is silently not written.** The reference still
/// rebuilds `.super` and returns the object (`:50-51`), so the call reports
/// success and changes nothing. `wrap` is the reason that is survivable: it
/// checks the slot exists *before* calling this (`:95-99`).
fn set_in_object(value: &BundValue, name: &str, n_value: &BundValue, budget: usize) -> BundValue {
    if budget == 0 || value.get(name).is_some() {
        return value.set(name, n_value.clone());
    }
    let Some(supers) = value.get(".super") else {
        return value.clone();
    };
    let Some(items) = supers.as_list() else {
        return value.clone();
    };
    let rebuilt: Vec<BundValue> = items
        .iter()
        .map(|s| set_in_object(s, name, n_value, budget.saturating_sub(1)))
        .collect();
    value.set(".super", BundValue::list(rebuilt))
}

/// Pull an OBJECT, or say which word wanted one and did not get one.
fn pull_object(vm: &mut dyn Vm, prefix: &str) -> Result<BundValue, Error> {
    let Some(v) = vm.pull() else {
        return Err(Error(format!("{prefix}: NO DATA IN #1")));
    };
    if v.dt() != OBJECT {
        return Err(Error(format!("{prefix}: NO OBJECT IN #1")));
    }
    Ok(v)
}

/// `.value_init` — take the value beneath the object and store it in `.data`
/// (`value_class.rs:54-69`).
///
/// **A plain `set`, not `set_in_object`.** This is the object that *owns*
/// `.data`; the slot comes into existence here.
fn method_value_init(vm: &mut dyn Vm) -> Result<(), Error> {
    let Some(obj) = vm.pull() else {
        return Err(Error(
            "Stack is empty for method '.init' of object Value".into(),
        ));
    };
    let Some(data) = vm.pull() else {
        return Err(Error("Object(Value) NO DATA #1".into()));
    };
    vm.push(obj.set(".data", data));
    Ok(())
}

/// The shared body of `.int_init` and `.bool_init`
/// (`int_class.rs:11-29`, `bool_class.rs:11-29`).
///
/// Both run *after* `Value`'s `.init` has stored `.data`, and both do the same
/// thing to it: convert in place to the subclass's tag. That is the entire
/// difference between `Integer` and `Bool` — which is why
/// `"42" :Integer object unwrap` is the integer 42 and not the string.
fn init_converting(vm: &mut dyn Vm, target: u16, class: &str, tag: &str) -> Result<(), Error> {
    let Some(obj) = vm.pull() else {
        return Err(Error(format!("{class}: stack is empty")));
    };
    let Some(data) = locate(&obj, ".data") else {
        return Err(Error(format!("{class}: NO WRAPPED DATA WAS FOUND")));
    };
    let converted = crate::convert::conv_value(&data, target)
        .map_err(|e| Error(format!("{class}: error converting to {tag}: {}", e.0)))?;
    vm.push(set_in_object(&obj, ".data", &converted, 512));
    Ok(())
}

fn method_int_init(vm: &mut dyn Vm) -> Result<(), Error> {
    init_converting(vm, bund2_value::INTEGER, "Integer", "INT")
}

fn method_bool_init(vm: &mut dyn Vm) -> Result<(), Error> {
    init_converting(vm, bund2_value::BOOL, "Bool", "BOOL")
}

fn method_float_init(vm: &mut dyn Vm) -> Result<(), Error> {
    init_converting(vm, bund2_value::FLOAT, "Float", "FLOAT")
}

/// `unwrap` — replace the object with the value it carries
/// (`value_class.rs:108-125`).
fn unwrap_word(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline UNWRAP".into()));
    }
    let obj = pull_object(vm, "UNWRAP")?;
    let Some(v) = locate(&obj, ".data") else {
        return Err(Error("UNWRAP: found no wrapped VALUE".into()));
    };
    vm.push(v);
    Ok(())
}

/// `is` — like `unwrap`, but **leaves the object underneath**
/// (`value_class.rs:127-147`).
///
/// That is the whole distinction the `is_demo` example is built to show: `is`
/// pushes the object and then the value, so a following `if` consumes the value
/// and the object is still there.
fn is_word(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline IS".into()));
    }
    let obj = pull_object(vm, "IS")?;
    let Some(v) = locate(&obj, ".data") else {
        return Err(Error("IS: found no wrapped VALUE".into()));
    };
    vm.push(obj);
    vm.push(v);
    Ok(())
}

/// `wrap` — store a value into an existing object's `.data`
/// (`value_class.rs:83-106`).
///
/// **The operands are object-on-top, value-beneath** — the opposite of
/// construction, which is why `wrap_unwrap_demo.bund` needs a `swap` first.
///
/// The reference's depth guard and its error strings both say `UNWRAP`
/// (`:85`, `:91`, `:93`); only the two later messages say `WRAP`. Preserved:
/// the text is what a `?try` handler reads.
fn wrap_word(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline UNWRAP".into()));
    }
    let obj = pull_object(vm, "UNWRAP")?;
    // Checked *before* the write, because `set_in_object` cannot report that it
    // found no owner for the slot — it returns the object unchanged.
    if locate(&obj, ".data").is_none() {
        return Err(Error("WRAP: can not detect if OBJECT is wrappable".into()));
    }
    let Some(data) = vm.pull() else {
        return Err(Error("WRAP NO DATA IN #1".into()));
    };
    vm.push(set_in_object(&obj, ".data", &data, 512));
    Ok(())
}

/// `#` — run a lambda over an object's wrapped value
/// (`reference/Bund/src/stdlib/functions/oop/object_execute.rs:11-44`).
///
/// It is `unwrap` followed by `!`, spelled as two `apply` calls on the words
/// themselves (`:36,38`).
///
/// **Both of those calls discard their result** — `let _ = vm.apply(…)` — so a
/// failing `unwrap` does not stop `#`, and the `!` that follows runs against
/// whatever the stack then holds. Reproduced rather than corrected: it is
/// reachable only when the object has no `.data`, and turning it into an error
/// would change what a program sees. The guard above rejects the non-OBJECT
/// case, which is the one that actually occurs.
fn object_execute(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline #".into()));
    }
    let Some(body) = vm.pull() else {
        return Err(Error("# NO DATA IN #1".into()));
    };
    if !matches!(body.dt(), LAMBDA | PTR) {
        return Err(Error("# NO LAMBDA or PTR IN #1".into()));
    }
    let Some(obj) = vm.pull() else {
        return Err(Error("# NO DATA IN #2".into()));
    };
    if obj.dt() != OBJECT {
        return Err(Error("# NO OBJECT IN #2".into()));
    }
    vm.push(obj);
    let _ = unwrap_word(vm);
    vm.push(body);
    let _ = crate::values::execute_top(vm);
    Ok(())
}

/// `Value` and its two converting subclasses.
///
/// `Value`'s parent is `Object` (`value_class.rs:76`), so a wrapped value
/// inherits `str`, `println` and the rest of the base chain — that is how
/// `"…{A}…" format` reaches inside one.
fn register_wrapped(r: &mut Registry) {
    r.register_method(".value_init", method_value_init);
    r.register_method(".int_init", method_int_init);
    r.register_method(".bool_init", method_bool_init);
    r.register_method(".float_init", method_float_init);

    let value = BundValue::class(Default::default())
        .set(".class_name", BundValue::str("Value"))
        .set(".super", BundValue::list(vec![BundValue::str("Object")]))
        .set(".init", BundValue::ptr(".value_init"));
    r.register_class("Value", value);

    // `Integer`, `Bool` and `Float` differ *only* in the tag their `.init`
    // converts `.data` to (`int_class.rs:31-40`, `bool_class.rs:31-40`,
    // `float_class.rs:31-40` — three files, one shape).
    //
    // **`List` is deliberately not here.** It is the same shape plus a `push`
    // slot and a `.list_init` that is not a conversion (`list_class.rs:58-69`),
    // so it is a different job rather than a fourth row.
    for (name, init) in [
        ("Integer", ".int_init"),
        ("Bool", ".bool_init"),
        ("Float", ".float_init"),
    ] {
        let c = BundValue::class(Default::default())
            .set(".class_name", BundValue::str(name))
            .set(".super", BundValue::list(vec![BundValue::str("Value")]))
            .set(".init", BundValue::ptr(init));
        r.register_class(name, c);
    }

    r.register_native("unwrap", unwrap_word, eff(1, 1), WordKind::Sync);
    r.register_native("is", is_word, eff(1, 2), WordKind::Sync);
    r.register_native("wrap", wrap_word, eff(2, 1), WordKind::Sync);
    // **Opaque.** `#` ends by executing a lambda whose own effect is unknown,
    // so its consumption is not a constant (RFC-0004 §S6).
    r.register_native("#", object_execute, StackEffect::opaque(2), WordKind::Sync);
}

pub fn register(r: &mut Registry) {
    r.register_native("class", class_word, eff(0, 1), WordKind::Sync);
    r.register_native("object", object_word, eff(1, 1), WordKind::Sync);
    r.register_native("?class", is_class_word, eff(1, 1), WordKind::Sync);
    // **1 -> 2, because `?object` peeks.** It leaves the receiver and pushes
    // the answer beside it (`bund_class.rs:39`), unlike `?class`, which
    // consumes its name. The declaration said 1 -> 1 until
    // `cargo xtask effects` compared it against the probed table: the peek was
    // fixed earlier and the effect beside it was not.
    r.register_native("?object", is_object_word, eff(1, 2), WordKind::Sync);
    register_base(r);
    register_wrapped(r);
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
        assert_eq!(*i.peek().unwrap().unboxed(), BundValue::boolean(true));
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

    /// **Criterion 5**: the method table is registry state, not a global.
    ///
    /// The reference keeps `methods_fun` on the VM
    /// (`reference/rust_multistackvm/src/multistackvm_methods.rs:8`), and Bund2
    /// keeps it on `Registry`. The structural half — no `static`, no
    /// `thread_local` — is visible by reading; this is the behavioural half,
    /// and it is the one a future refactor to a global would break silently.
    ///
    /// Two `Interp`s in one process bind the *same* method name to different
    /// natives, and each answers with its own. Deliberately using a name the
    /// base hierarchy already owns, `.id`: if anything were shared, the second
    /// registration would be visible from the first VM.
    #[test]
    fn two_interps_bind_one_method_name_differently() {
        fn pushes_one(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.push(BundValue::int(1));
            Ok(())
        }
        fn pushes_two(vm: &mut dyn Vm) -> Result<(), Error> {
            vm.push(BundValue::int(2));
            Ok(())
        }

        let mut a = Interp::new();
        crate::register_all(&mut a.registry);
        let mut b = Interp::new();
        crate::register_all(&mut b.registry);
        a.registry.register_method(".id", pushes_one);
        b.registry.register_method(".id", pushes_two);

        let src = ":A class \".super\" [ :Object ] set register\n:.id :A object !";
        for (vm, want) in [(&mut a, 1i64), (&mut b, 2i64)] {
            let stream = bund2_syntax::compile(src).expect("compiles");
            vm.eval(&stream).expect("runs");
            assert_eq!(vm.peek().and_then(|v| v.as_int()), Some(want));
        }
    }

    // --- the wrapped-value classes -----------------------------------------

    fn top_display(src: &str) -> String {
        let i = run_src(src).expect("runs");
        i.peek().map(|v| v.display()).expect("a value")
    }

    /// **The subclass `.init` converts, and that is its whole job.** All four
    /// rows confirmed against the oracle (`int_class.rs:18`,
    /// `bool_class.rs:18`, `float_class.rs:18`).
    #[test]
    fn a_subclass_init_converts_the_wrapped_value() {
        assert_eq!(top_display("\"42\" :Integer object unwrap"), "42");
        assert_eq!(top_display("1 :Bool object unwrap"), "true");
        assert_eq!(top_display("0 :Bool object unwrap"), "false");
        assert_eq!(top_display("\"3.5\" :Float object unwrap"), "3.5");
    }

    /// `Value` itself does not convert — it stores what it was given.
    #[test]
    fn the_value_class_stores_without_converting() {
        let i = run_src("42 :Value object unwrap").expect("runs");
        assert_eq!(i.peek().and_then(|v| v.as_int()), Some(42));
        assert_eq!(top_display("\"42\" :Value object unwrap"), "42");
        let i = run_src("\"42\" :Value object unwrap").expect("runs");
        assert_eq!(
            i.peek().map(|v| v.dt()),
            Some(bund2_value::STRING),
            "Value must not convert; only Integer does"
        );
    }

    /// **`is` leaves the object, `unwrap` consumes it.** That is the entire
    /// difference between them (`value_class.rs:141-142` against `:121`), and
    /// it is what `is_demo.bund` exists to show.
    #[test]
    fn is_leaves_the_object_beneath_the_value() {
        let i = run_src("true :Bool object is").expect("runs");
        assert_eq!(i.depth(), 2, "`is` pushes the object and the value");
        let mut i = i;
        assert_eq!(i.pull().map(|v| v.display()).as_deref(), Some("true"));
        assert_eq!(i.pull().map(|v| v.dt()), Some(OBJECT));

        let i = run_src("true :Bool object unwrap").expect("runs");
        assert_eq!(i.depth(), 1, "`unwrap` consumes the object");
    }

    /// `wrap` writes through to whichever object in the tree owns `.data` — a
    /// plain `set` on the outer object would shadow it, and `unwrap` would then
    /// read the shadow. `9` must come back, not `7`.
    #[test]
    fn wrap_writes_the_inherited_slot_rather_than_shadowing_it() {
        assert_eq!(top_display("7 :Integer object 9 swap wrap unwrap"), "9");
    }

    /// `#` is `unwrap` then `!` (`object_execute.rs:35-38`).
    #[test]
    fn hash_runs_a_lambda_over_the_wrapped_value() {
        assert_eq!(top_display("5 :Integer object { 2 * }  #"), "10");
    }

    /// Every one of these is reported rather than asserted, and each message is
    /// the reference's — including `wrap`'s guard, which says `UNWRAP`
    /// (`value_class.rs:85`) because the reference's does.
    #[test]
    fn the_wrapped_value_words_report_their_misuse() {
        for (src, want) in [
            ("unwrap", "Stack is too shallow for inline UNWRAP"),
            ("is", "Stack is too shallow for inline IS"),
            ("wrap", "Stack is too shallow for inline UNWRAP"),
            ("1 2 wrap", "UNWRAP: NO OBJECT IN #1"),
            ("42 unwrap", "UNWRAP: NO OBJECT IN #1"),
            ("42 is", "IS: NO OBJECT IN #1"),
            ("1 2 #", "# NO LAMBDA or PTR IN #1"),
            ("1 { 1 } #", "# NO OBJECT IN #2"),
            ("#", "Stack is too shallow for inline #"),
        ] {
            match run_src(src) {
                Ok(_) => panic!("{src} was expected to fail"),
                Err(e) => assert!(e.contains(want), "{src}: wanted {want:?}, got {e:?}"),
            }
        }
    }

    /// A conversion that cannot succeed is reported with the class's own
    /// prefix, not the conversion table's (`int_class.rs:23`).
    #[test]
    fn a_failing_subclass_init_names_the_class() {
        match run_src("nodata :Integer object") {
            Ok(_) => panic!("expected a failure"),
            Err(e) => assert!(
                e.contains("Integer: error converting to INT"),
                "got {e:?}"
            ),
        }
    }

    #[test]
    fn the_wrapped_value_classes_and_words_are_registered() {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        for c in ["Value", "Integer", "Bool", "Float"] {
            assert!(i.registry.class(c).is_some(), "class {c} is not registered");
        }
        for w in ["wrap", "unwrap", "is", "#"] {
            assert!(
                i.registry.interner.lookup_call(w).is_some(),
                "{w} is not registered"
            );
        }
    }
}
