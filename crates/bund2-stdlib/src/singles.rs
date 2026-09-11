//! Seven words that each unblock exactly one golden, and share nothing but
//! that: `?.`, `attribute`, `car`, `cdr`, `compile`, `get,`, `pull`, `var`.
//!
//! Grouped by how much of a golden they unblock rather than by subject,
//! because each is small and none has a family here to join. Where a word has
//! an obvious sibling in the reference it is implemented too — `cdr` beside
//! `car`, `var?` and `var-` beside `var` — since the sibling is the same base
//! function with one flag flipped and leaving it out would be the arbitrary
//! choice.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BundValue, CONDITIONAL, EXIT, LAMBDA, LIST, MAP, OBJECT, PAIR};

/// `push`, spelled `+++`, and `push.` — **append the second operand, as one
/// element, to the first** (`reference/Bund/src/stdlib/functions/values/push.rs:10-53`).
///
/// Both operands go through `conv(LIST)` first (`:32,39`), so a scalar becomes
/// a one-item LIST, and the second LIST is appended whole: `[ 1 2 ] 3 push` is
/// `[3, [1, 2]]`, not `[1, 2, 3]`. Confirmed against the oracle. The `.` form
/// takes the receiver from the workbench and answers there; its guard says
/// "Workbench" for both of its checks (`:17-24`).
fn push_list(vm: &mut dyn Vm) -> Result<(), Error> {
    push_base(vm, crate::wb::Side::Stack, "PUSH")
}

fn push_list_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    push_base(vm, crate::wb::Side::Bench, "PUSH.")
}

fn push_base(vm: &mut dyn Vm, side: crate::wb::Side, prefix: &str) -> Result<(), Error> {
    match side {
        crate::wb::Side::Stack if vm.depth() < 2 => {
            return Err(Error(format!("Stack is too shallow for inline {prefix}")));
        }
        crate::wb::Side::Bench if vm.workbench_depth() < 1 || vm.depth() < 1 => {
            return Err(Error(format!("Workbench is too shallow for inline {prefix}")));
        }
        _ => {}
    }
    let as_list = |v: &BundValue| {
        crate::convert::conv_value(v, LIST)
            .map_err(|e| Error(format!("{prefix} casting of list returned: {}", e.0)))
    };
    let first = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))?;
    let first = as_list(&first)?;
    let second = vm
        .pull()
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #2")))?;
    let second = as_list(&second)?;
    let mut items = first
        .as_list()
        .ok_or_else(|| Error::internal("conv(LIST) answered with something that is not a LIST"))?
        .to_vec();
    items.push(second);
    side.push(vm, BundValue::list(items));
    Ok(())
}

/// `unfold` and `unfold.` — spread a LIST's items onto the side it came from
/// (`reference/Bund/src/stdlib/functions/values/unfold.rs:8-47`). It *casts*
/// rather than converts (`:29-32`), so anything but a LIST is an error.
fn unfold(vm: &mut dyn Vm) -> Result<(), Error> {
    unfold_base(vm, crate::wb::Side::Stack, "UNFOLD")
}

fn unfold_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    unfold_base(vm, crate::wb::Side::Bench, "UNFOLD.")
}

fn unfold_base(vm: &mut dyn Vm, side: crate::wb::Side, prefix: &str) -> Result<(), Error> {
    match side {
        crate::wb::Side::Stack if vm.depth() < 1 => {
            return Err(Error(format!("Stack is too shallow for inline {prefix}")));
        }
        crate::wb::Side::Bench if vm.workbench_depth() < 1 => {
            return Err(Error(format!("Workbench is too shallow for inline {prefix}")));
        }
        _ => {}
    }
    let v = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))?;
    let Some(items) = v.as_list().map(<[BundValue]>::to_vec) else {
        return Err(Error(format!(
            "{prefix} casting of list returned: This Dynamic type is not list"
        )));
    };
    for item in items {
        side.push(vm, item);
    }
    Ok(())
}

/// `tag` — `<value> <key> <text> tag` sets one tag on the value
/// (`reference/rust_multistackvm/src/stdlib/values/value_tag.rs:24-63`).
///
/// The text is pulled and checked first, then the key, then the value, so a
/// failed check leaves what is beneath it on the stack.
fn tag_word(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 3 {
        return Err(Error("Stack is too shallow for inline tag".into()));
    }
    let text = crate::pull::operand(vm, "TAG", 1)?;
    let Some(text) = text.as_str() else {
        return Err(Error(
            "TAG value expected to be string: This Dynamic type is not string".into(),
        ));
    };
    let key = crate::pull::operand(vm, "TAG", 2)?;
    let Some(key) = key.as_str() else {
        return Err(Error(
            "TAG key expected to be string: This Dynamic type is not string".into(),
        ));
    };
    let value = crate::pull::operand(vm, "TAG", 3)?;
    vm.push(value.with_tag(std::rc::Rc::from(key), std::rc::Rc::from(text)));
    Ok(())
}

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// `?.` — run a lambda, and move a value to the workbench if it said `true`
/// (`reference/rust_multistackvm/src/stdlib/stackop/conditional_move.rs:11-79`).
///
/// The order is the surprising part. The lambda comes off first (`:16`) and is
/// **evaluated immediately** (`:20`); its result is then pulled as the
/// condition (`:26`), and the value to move is pulled from *under* that
/// (`:38`). So the lambda is a predicate that runs before anything is
/// inspected, and it may push its own answer from anywhere.
///
/// **If the condition is false the value is dropped, not restored** — the
/// reference pulls it before testing (`:38`) and only the `true` branch pushes
/// it anywhere (`:44-65`). That is easy to read as a bug and is preserved.
///
/// `?move` is the same base with the destination a named stack instead of the
/// workbench; it also pulls a stack name (`:47`).
fn conditional_move(vm: &mut dyn Vm, to_stack: bool, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}()")));
    }
    let lambda_val = crate::pull::operand(vm, prefix, 1)?;
    if lambda_val.dt() != LAMBDA {
        return Err(Error(format!("{prefix} expected to have a LAMBDA at #1")));
    }
    let body = lambda_val.clone();
    vm.eval_lambda(&body)
        .map_err(|e| e.context(format!("{prefix} lambda returns: ")))?;

    let outcome = vm
        .pull()
        .ok_or_else(|| Error(format!("{prefix} returns: NO OUTCOME")))?;
    let BundValue::Bool(cond, _) = *outcome.unboxed() else {
        return Err(Error(format!(
            "{prefix} returns: OUTCOME IS NOT BOOLEAN: This Dynamic type is not bool"
        )));
    };
    let value = vm
        .pull()
        .ok_or_else(|| Error(format!("{prefix} returns: NO VALUE TO MOVE")))?;
    if cond {
        if to_stack {
            let name_val = vm
                .pull()
                .ok_or_else(|| Error(format!("{prefix} returns: NO DATA for stack name")))?;
            let name = name_val
                .as_str()
                .ok_or_else(|| Error(format!("{prefix} returns: not a string")))?;
            vm.push_to(&name, value);
        } else {
            vm.push_workbench(value);
        }
    }
    Ok(())
}

fn q_move_workbench(vm: &mut dyn Vm) -> Result<(), Error> {
    conditional_move(vm, false, "?.")
}

fn q_move_stack(vm: &mut dyn Vm) -> Result<(), Error> {
    conditional_move(vm, true, "?MOVE")
}

/// `attribute` — append to a value's `attr` list
/// (`reference/rust_multistackvm/src/stdlib/values/value_tag.rs:5-25`).
///
/// `attr_add` `dup`s and **regenerates the identity** before pushing
/// (`reference/rust_dynamic/src/attr.rs:19-20`), so the result is a different
/// value from the one that went in — which is why `BundValue::attr_added`
/// exists as its own operation rather than being a field write.
fn attribute(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline attribute".into()));
    }
    let attr = crate::pull::operand(vm, "ATTRIBUTE", 1)?;
    let value = crate::pull::operand(vm, "ATTRIBUTE", 2)?;
    vm.push(value.attr_added(attr));
    Ok(())
}

/// `car` and `cdr` (`reference/rust_dynamic/src/carcdr.rs:72-137,139-...`).
///
/// `car` is the first element; `cdr` is **a LIST of the rest**, so
/// `[ 1 ] cdr` is an empty list and not an error. Both answer `None` for an
/// empty list, which the caller turns into `CAR returned: NO DATA`
/// (`reference/rust_multistackvm/src/stdlib/values/value_carcdr.rs:46`).
///
/// Only the LIST arm is implemented: the reference also handles MATRIX, QUEUE
/// and FIFO, none of which Bund2 constructs.
///
/// **Both forms take their operand and leave their answer on the same side**
/// (`value_carcdr.rs:27-33,39-42`). That is *not* the `.` contract D24
/// describes and not the one `bund/string` uses — see `crate::wb` for the
/// three shapes.
fn car_cdr(vm: &mut dyn Vm, side: crate::wb::Side, want_car: bool, base: &str) -> Result<(), Error> {
    let prefix = &format!("{base}{}", side.dot());
    crate::wb::shallow(vm, side, 1, prefix)?;
    let v = crate::wb::operand(vm, side, prefix)?;
    let items = match v.dt() {
        LIST | PAIR => v.as_list().map(<[BundValue]>::to_vec),
        _ => None,
    };
    let Some(items) = items.filter(|i| !i.is_empty()) else {
        return Err(Error(format!("{prefix} returned: NO DATA")));
    };
    let out = if want_car {
        items.first().cloned().unwrap_or(BundValue::nodata())
    } else {
        BundValue::list(items.get(1..).unwrap_or_default().to_vec())
    };
    side.push(vm, out);
    Ok(())
}

fn car(vm: &mut dyn Vm) -> Result<(), Error> {
    car_cdr(vm, crate::wb::Side::Stack, true, "CAR")
}

fn car_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    car_cdr(vm, crate::wb::Side::Bench, true, "CAR")
}

fn cdr_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    car_cdr(vm, crate::wb::Side::Bench, false, "CDR")
}

fn cdr(vm: &mut dyn Vm) -> Result<(), Error> {
    car_cdr(vm, crate::wb::Side::Stack, false, "CDR")
}

/// `compile` — parse a string and push the token stream as a LIST
/// (`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:20-49`).
///
/// **The oracle's own dump mode**, and the mechanism `cargo xtask parity` is
/// built on: `"<source>" compile debug.display_stack` renders the reference's
/// stream in the text the golden normaliser already handles.
///
/// Two details from the source. The reference appends `\n` before parsing
/// (`:30`) — RFC-0003 §S1 drops that by admitting end of input as a
/// terminator, so Bund2's parser needs no equivalent. And the trailing `EXIT`
/// **breaks the loop** rather than being pushed (`:35-37`), so the LIST holds
/// the program and not the terminator.
fn compile(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline compile".into()));
    }
    let v = crate::pull::operand(vm, "BUND.COMPILE", 1)?;
    let src = v
        .as_str()
        .ok_or_else(|| Error("BUND.COMPILE casting string returns: not a string".into()))?;
    let stream = bund2_syntax::compile(&src)
        .map_err(|e| Error(format!("BUND.COMPILE error in parsing of BUND code: {}", e.render(&src))))?;
    let body: Vec<BundValue> = stream.into_iter().take_while(|v| v.dt() != EXIT).collect();
    vm.push(BundValue::list(body));
    Ok(())
}

/// `get,` — read a key and leave the container in place
/// (`reference/Bund/src/stdlib/functions/values/getsetinplace.rs:16-70`).
///
/// The difference from `get` is the container: `get` consumes it, `get,`
/// pushes it back before the value (`:66-70`), so `dict :k get,` leaves
/// `[dict, value]`. That is what makes a chain of reads possible without
/// re-fetching.
///
/// The tag gate is wide — MAP, INFO, CONFIG, ASSOCIATION, CURRY, MESSAGE,
/// CONDITIONAL and OBJECT (`:51`) — and it is a **tag** test, so a VALUEMAP is
/// excluded even though it is a container.
/// `get,` / `set,` and their `.` siblings — read or write a key **in place**
/// (`reference/Bund/src/stdlib/functions/values/getsetinplace.rs:16-95`).
///
/// A fifth `.` shape, and the most asymmetric one: the **dictionary** follows
/// the side, while the key and the value it sets are always pulled from the
/// current stack (`:54-57,72-79`), and `get` leaves the dictionary on the side
/// but the value it found on the **stack** (`:66-70`).
///
/// So `get.,` reads a MAP that lives on the workbench, takes the key off the
/// stack, and hands the value back on the stack — three different places in one
/// word.
fn getset_inplace(vm: &mut dyn Vm, side: crate::wb::Side, is_set: bool) -> Result<(), Error> {
    let prefix = &format!("{}{},", if is_set { "SET" } else { "GET" }, side.dot());
    // The guards, per `:19-40`: the stack form wants everything on the stack,
    // the workbench form wants one workbench cell plus the operands that still
    // come off the stack.
    match side {
        crate::wb::Side::Stack => {
            let need = if is_set { 3 } else { 2 };
            if vm.depth() < need {
                return Err(Error(format!("Stack is too shallow for inline {prefix}")));
            }
        }
        crate::wb::Side::Bench => {
            if vm.workbench_depth() < 1 {
                return Err(Error(format!(
                    "Workbench is too shallow for inline {prefix}"
                )));
            }
            if vm.depth() < if is_set { 2 } else { 1 } {
                return Err(Error(format!("Stack is too shallow for inline {prefix}")));
            }
        }
    }
    let container = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))?;
    let dt = container.dt();
    // The reference admits eight tags (`:51`); these are the ones Bund2
    // constructs. CURRY, MESSAGE, INFO, CONFIG and ASSOCIATION have no
    // constructor here, so no value can carry them.
    if !matches!(dt, MAP | CONDITIONAL | OBJECT) {
        return Err(Error(format!(
            "{prefix}: unsupported operation for a DICT-based type"
        )));
    }
    if is_set {
        let obj = crate::pull::operand(vm, prefix, 1)?;
        let key_val = vm
            .pull()
            .ok_or_else(|| Error(format!("{prefix} returns NO DATA #2")))?;
        let key = key_val
            .as_str()
            .ok_or_else(|| Error(format!("{prefix} error in GET: not a string")))?;
        side.push(vm, container.set(&key, obj));
    } else {
        let key_val = vm
            .pull()
            .ok_or_else(|| Error(format!("{prefix} returns NO DATA #2")))?;
        let key = key_val
            .as_str()
            .ok_or_else(|| Error(format!("{prefix} error in GET: not a string")))?;
        let val = container
            .get(&key)
            .ok_or_else(|| Error(format!("{prefix}: Key not found: {key}")))?;
        // Dictionary back to the side it came from, value to the stack.
        side.push(vm, container);
        vm.push(val);
    }
    Ok(())
}

/// `pull` — name a run of stack values into a dict
/// (`reference/Bund/src/stdlib/functions/values/pull.rs:10-51`).
///
/// Takes a LIST of names, pulls one value per name, and pushes a MAP. The
/// names are consumed **in list order** while the values come off the stack
/// top-down (`:36-45`), so the first name in the list gets the value that was
/// on top — the reversal again, this time between two sequences rather than
/// two operands.
/// **The `.` form takes the *names list* off the workbench and leaves the MAP
/// there, but pulls the values themselves off the current stack either way**
/// (`reference/Bund/src/stdlib/functions/values/pull.rs:24-25,41-48`). A fourth
/// shape; see `crate::wb`.
fn pull_word(vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    let prefix = &format!("PULL{}", side.dot());
    if side.depth(vm) < 1 {
        return Err(Error(format!(
            "{} is too shallow for inline {prefix}",
            if side == crate::wb::Side::Stack { "Stack" } else { "Workbench" }
        )));
    }
    let names_val = crate::wb::operand(vm, side, prefix)?;
    let dt = names_val.dt();
    if dt != LIST && dt != PAIR {
        return Err(Error(format!(
            "PULL casting of list returned: This is not a LIST/PAIR value but {dt}"
        )));
    }
    let names = names_val
        .as_list()
        .ok_or_else(|| Error("PULL casting of list returned: not a list".into()))?
        .to_vec();
    let mut res = BundValue::map(Default::default());
    for n in names {
        let name = n
            .as_str()
            .ok_or_else(|| Error("PULL error casting name from string".into()))?;
        let value = vm
            .pull()
            .ok_or_else(|| Error("PULL can not pull value from stack".into()))?;
        res = res.set(&name, value);
    }
    side.push(vm, res);
    Ok(())
}

/// `var` — bind a name in the **variable** table
/// (`reference/rust_multistackvm/src/stdlib/vars/registry.rs:4-30`).
///
/// A sixth namespace, and one that name resolution never consults: the only
/// reader is `var?` (`reference/rust_multistackvm/src/stdlib/vars/resolve.rs:12`).
/// So `:dup 1 var` does not shadow `dup`, and a bare `x` after `:x 1 var` is
/// still an unregistered word — the value comes back only through `:x var?`.
fn var(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline var".into()));
    }
    let value = crate::pull::operand(vm, "VAR", 1)?;
    let name_val = crate::pull::operand(vm, "VAR", 2)?;
    let name = name_val
        .as_str()
        .ok_or_else(|| Error("VAR expecting var name to be string".into()))?;
    vm.register_var(&name, value);
    Ok(())
}

fn var_read(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline VAR?".into()));
    }
    let name_val = crate::pull::operand(vm, "VAR?", 1)?;
    let name = name_val
        .as_str()
        .ok_or_else(|| Error("VAR? returns error: not a string".into()))?;
    let v = vm
        .var(&name)
        .ok_or_else(|| Error(format!("VAR? returned: Variable {name} is not registered")))?;
    vm.push(v);
    Ok(())
}

fn var_unregister(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline var-".into()));
    }
    let name_val = crate::pull::operand(vm, "VAR-", 1)?;
    let name = name_val
        .as_str()
        .ok_or_else(|| Error("VAR- expecting var name to be string".into()))?;
    vm.unregister_var(&name);
    Ok(())
}

/// `string.distance.*` — edit distances, as INTEGERs
/// (`reference/Bund/src/stdlib/functions/string/distance.rs:21-95`).
///
/// **F18's shape**: the guard is `< 1` (`:24`) but two operands are pulled
/// (`:44,50`), so `"a" string.distance.levenshtein` reports `NO DATA #2` on
/// the reference. Bund2 declares the probed arity per F18's FIX, so it reports
/// `Stack is too shallow` and leaves the operand in place.
///
/// Operand order is the reference's: #1 off the top is `string1`, #2 below it
/// is `string2`, and the distance is computed `(string1, string2)` (`:73`).
/// Symmetric for these algorithms, so the order is invisible in the answer —
/// stated because it will not be for the asymmetric ones.
fn distance_word(vm: &mut dyn Vm, f: fn(&str, &str) -> usize, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let a = crate::pull::operand(vm, prefix, 1)?;
    let b = crate::pull::operand(vm, prefix, 2)?;
    let (Some(a), Some(b)) = (a.as_str(), b.as_str()) else {
        return Err(Error(format!(
            "{prefix} returned for #1: This Dynamic type is not string"
        )));
    };
    vm.push(BundValue::int(f(&a, &b) as i64));
    Ok(())
}

/// `string.expressionmatch` — does a string satisfy an `srch` expression?
/// (`reference/Bund/src/stdlib/functions/string/textexpr_match.rs:11-71`).
///
/// The **expression is pulled second** and the subject first (`:28,32`), then
/// `Expression::new(string1)` is built from `string1` — the *first* pull —
/// and matched against `string2` (`:61,66`). So the expression is on top and
/// the subject beneath it.
fn expression_match(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(
            "Stack is too shallow for inline STRING.EXPRESSIONMATCH".into(),
        ));
    }
    let expr_val = crate::pull::operand(vm, "STRING.EXPRESSIONMATCH", 1)?;
    let subject_val = crate::pull::operand(vm, "STRING.EXPRESSIONMATCH", 2)?;
    let (Some(expr), Some(subject)) = (expr_val.as_str(), subject_val.as_str()) else {
        return Err(Error(
            "STRING.EXPRESSIONMATCH returned for #1: This Dynamic type is not string".into(),
        ));
    };
    let matcher = srch::Expression::new(&expr).map_err(|e| {
        Error(format!(
            "STRING.EXPRESSIONMATCH returned error when creates matcher: {e:?}"
        ))
    })?;
    vm.push(BundValue::boolean(matcher.matches(subject)));
    Ok(())
}

/// `len` — the length of the top value, **peeked**
/// (`reference/rust_multistackvm/src/stdlib/values/value_len.rs:6-19`).
///
/// `Value::len` is a table over tags (`reference/rust_dynamic/src/len.rs:5-107`)
/// with two properties worth stating:
///
/// - **A STRING's length is in bytes, not characters** (`:23`), so a string of
///   multi-byte characters is longer than it looks.
/// - **A scalar falls back to the length of its string form** (`:96-104`), so
///   `42 len` is 2 and `3.14 len` is 4. Not an error, and not 1.
///
/// `NODATA` is 0 (`:95`); anything that will not convert is 1 (`:100`).
fn len(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline len".into()));
    }
    let v = crate::pull::top(vm, "LEN")?;
    let n = match v.dt() {
        bund2_value::NODATA => 0,
        bund2_value::STRING => v.as_str().map_or(0, |s| s.len()),
        LIST | PAIR => v.as_list().map_or(0, <[BundValue]>::len),
        MAP | CONDITIONAL | OBJECT | bund2_value::CLASS => v.as_map().map_or(0, |m| m.len()),
        LAMBDA => v.as_lambda().map_or(0, <[BundValue]>::len),
        bund2_value::VALUEMAP => v.as_valuemap().map_or(0, |m| m.len()),
        bund2_value::JSON => v
            .as_json()
            .map_or(0, |j| j.as_array().map_or(1, |a| a.len())),
        // The scalar fallback: the length of the value's string form.
        _ => v.display().len(),
    };
    vm.push(BundValue::int(n as i64));
    Ok(())
}

/// `++` — merge, or add
/// (`reference/Bund/src/stdlib/functions/values/merge.rs:24-101`).
///
/// One word with two unrelated jobs, chosen by the tags. With a MAP or
/// CONDITIONAL on top:
///
/// - a **LIST** beneath it is merged in **by position**, its elements becoming
///   the keys `"0"`, `"1"`, … (`:57-70`);
/// - a **MAP or CONDITIONAL** beneath it is merged key by key, and the
///   receiver's tag is **restored afterwards** (`:73-85`) because `set` on a
///   CONDITIONAL would otherwise leave it a MAP.
///
/// Anything else — including a MAP with a scalar beneath it — falls through to
/// `numeric_op(Add, …)` (`:89-99`), which is the same addition `+` performs.
/// So `++` on two integers is `+`, and the merge is the special case rather
/// than the rule.
fn merge(vm: &mut dyn Vm) -> Result<(), Error> {
    merge_base(vm, crate::wb::Side::Stack)
}

/// `merge.` / `++.` — D24's contract, for once
/// (`reference/Bund/src/stdlib/functions/values/merge.rs:26-50,17-18`).
///
/// Receiver off the workbench, the thing merged into it off the stack, answer
/// back to the workbench. This is the shape the `.` suffix is *described* as
/// having, and one of the few families that actually has it.
fn merge_wb(vm: &mut dyn Vm) -> Result<(), Error> {
    merge_base(vm, crate::wb::Side::Bench)
}

fn merge_base(vm: &mut dyn Vm, side: crate::wb::Side) -> Result<(), Error> {
    let prefix = &format!("MERGE{}", side.dot());
    match side {
        crate::wb::Side::Stack => {
            if vm.depth() < 2 {
                return Err(Error(format!("Stack is too shallow for inline {prefix}")));
            }
        }
        crate::wb::Side::Bench => {
            if vm.workbench_depth() < 1 {
                return Err(Error(format!("Workbench is too shallow for inline {prefix}")));
            }
            if vm.depth() < 1 {
                return Err(Error(format!("Stack is too shallow for inline {prefix}")));
            }
        }
    }
    let a = side
        .pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))?;
    let b = crate::pull::operand(vm, prefix, 2)?;
    let receiver_dt = a.dt();
    if matches!(receiver_dt, MAP | CONDITIONAL) {
        match b.dt() {
            LIST => {
                let items = b
                    .as_list()
                    .ok_or_else(|| Error("MERGE returns error during LIST casting".into()))?
                    .to_vec();
                let mut out = a;
                for (i, v) in items.into_iter().enumerate() {
                    out = out.set(&i.to_string(), v);
                }
                side.push(vm, out);
                return Ok(());
            }
            MAP | CONDITIONAL => {
                let src = b
                    .as_map()
                    .ok_or_else(|| Error("MERGE returns error during DICT casting".into()))?
                    .clone();
                let mut out = a;
                for (k, v) in src {
                    out = out.set(&k, v);
                }
                // `op_val1.dt = o_type` (`:81`) — `set` rebuilds as a MAP, so
                // a CONDITIONAL receiver has to be re-tagged or it stops being
                // one.
                side.push(vm, out.with_dt_tag(receiver_dt));
                return Ok(());
            }
            _ => {}
        }
    }
    match crate::math::numeric_op(crate::math::Op::Add, &a, &b) {
        Ok(v) => {
            side.push(vm, v);
            Ok(())
        }
        Err(e) => Err(Error(format!(
            "MERGE returns error for default operation: {}",
            e.0
        ))),
    }
}

/// `display` — render a value through termimad, dispatching on its tag
/// (`reference/Bund/src/stdlib/functions/system/display.rs:15-77`).
///
/// Three arms, and they do genuinely different things:
///
/// - a **CONDITIONAL** of type `fmt` is run and its text printed (`:34-46`);
///   any other conditional type is an error naming `FMT.STR` rather than
///   `DISPLAY` (`:53`), which is F40's shape and is reproduced;
/// - an **OBJECT** is dispatched: the reference pushes the *string* `display`
///   and the object, then applies `!` (`:59-61`), so it is exactly
///   `:display <obj> !` and reaches the `.display` method;
/// - anything else is `conv(STRING)` and printed (`:65-72`).
///
/// It flushes stdout afterwards (`:75`), because `print_text` does not.
fn display(vm: &mut dyn Vm) -> Result<(), Error> {
    use std::io::Write;
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for DISPLAY".into()));
    }
    let v = vm
        .pull()
        .ok_or_else(|| Error("DISPLAY: No value discovered on the stack".into()))?;
    match v.dt() {
        CONDITIONAL => {
            let ty = v
                .get("type")
                .and_then(|t| t.as_str())
                .ok_or_else(|| Error("DISPLAY: getting fmt type returns error".into()))?;
            if ty != "fmt" {
                return Err(Error(format!(
                    "FMT.STR: conditional is of incorrect type: {ty}"
                )));
            }
            let run = vm
                .conditional("fmt")
                .ok_or_else(|| Error("DISPLAY: conditional returns error: no fmt".into()))?;
            run(vm, v).map_err(|e| e.context("DISPLAY: conditional returns error: "))?;
            let out = vm
                .pull()
                .ok_or_else(|| Error("DISPLAY: No value discovered on the stack".into()))?;
            let text = out
                .as_str()
                .ok_or_else(|| Error("DISPLAY: casting out value returns error".into()))?;
            print!("{}", termimad::term_text(&text));
        }
        OBJECT => {
            // The reference pushes the method *name* and the object and then
            // applies `!` (`:59-61`) — the same shape a program writes as
            // `:display <obj> !`.
            vm.push(BundValue::str("display"));
            vm.push(v);
            return crate::values::execute_top(vm);
        }
        _ => print!("{}", termimad::term_text(&v.display())),
    }
    let _ = std::io::stdout().flush();
    Ok(())
}

/// `apply` — evaluate one value as the evaluator would
/// (`reference/Bund/src/stdlib/functions/bund/bund_interpreter.rs:53-64`).
///
/// Pull, then hand it to `VM::apply`
/// (`reference/rust_multistackvm/src/multistackvm_apply.rs:8-104`) — the same
/// function the main loop calls per term. So a CALL is dispatched and anything
/// else is pushed, which is what makes `compile` round-trip: the LIST of terms
/// `compile` produces can be fed back one at a time and mean the same thing.
///
/// `compile_and_apply.bund` is exactly that loop, and it is the reason both
/// words exist — the pair is the language's own eval, reachable from a program.
///
/// The effect is declared 1→0 because what `apply` leaves behind is whatever
/// the applied value does; there is no single answer, and RFC-0004 §S1's
/// `Opaque` is the arm that will eventually say so.
fn apply(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for APPLY".into()));
    }
    let v = crate::pull::operand(vm, "APPLY", 1)?;
    vm.apply(v)
}

/// `alias` — bind a name to another name
/// (`reference/rust_multistackvm/src/stdlib/alias.rs:5-43`).
///
/// The **alias is on top** and the target beneath it (`:9,11`), so the source
/// reads `<target> <alias> alias`. Both are cast to strings; neither has to
/// exist yet, which is D16's open world — a name may be bound before anything
/// answers to it.
///
/// # The cycle, and why it is a Warning and not an error
///
/// `:a :b alias :b :a alias` builds a loop. The reference does not check:
/// `register_alias` unregisters and inserts
/// (`reference/rust_multistackvm/src/multistackvm_alias.rs:5-15`), and
/// resolution then spins. Bund2 cannot spin — `Registry::follow` stops after
/// 64 links — but it resolves to whichever link the guard happened to stop on,
/// which is a silent wrong answer.
///
/// So the binding is **performed**, exactly as the reference performs it, and
/// a `Warning` is reported beside it. D36's ladder puts it there: nothing has
/// stopped, so it earns one line on stderr rather than a table. Refusing the
/// registration would be a deviation the reference does not license; saying
/// nothing would be the silence the guard already provides.
fn alias(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline alias()".into()));
    }
    let alias_val = crate::pull::operand(vm, "ALIAS", 1)?;
    let name_val = crate::pull::operand(vm, "ALIAS", 2)?;
    let Some(alias_name) = alias_val.as_str() else {
        return Err(Error(
            "ALIAS on alias returns: This Dynamic type is not string".into(),
        ));
    };
    let Some(target) = name_val.as_str() else {
        return Err(Error(
            "ALIAS on name returns: This Dynamic type is not string".into(),
        ));
    };
    if vm.register_alias(&alias_name, &target) {
        vm.report(bund2_api::diag::Diagnostic::warning(format!(
            "`{alias_name}` aliases `{target}`, which resolves back to `{alias_name}`. The binding stands, as the reference makes it; but resolving it walks in a circle and stops at an arbitrary link."
        )));
    }
    Ok(())
}

/// `unalias` — remove a binding (`alias.rs:45-70`).
fn unalias(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline unalias()".into()));
    }
    let v = crate::pull::operand(vm, "UNALIAS", 1)?;
    let Some(name) = v.as_str() else {
        return Err(Error(
            "UNALIAS returns: This Dynamic type is not string".into(),
        ));
    };
    vm.unregister_alias(&name);
    Ok(())
}

/// `pair` — two values into a PAIR
/// (`reference/rust_multistackvm/src/stdlib/artefacts.rs:6-22`).
///
/// **F18's clearest case.** The reference guards `< 1` (`:7`) and then pulls
/// twice (`:10,16`), so `1 pair` passes the guard, consumes the `1`, and fails
/// the second pull with `PAIR returns NO DATA #2` — a message no other arity
/// failure produces, on an empty stack.
///
/// F18's disposition is FIX: declare the arity the word consumes, so the guard
/// fires first. `1 pair` reports `Stack is too shallow for inline pair()` and
/// **leaves the `1` where it was**.
fn pair(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline pair()".into()));
    }
    let x = crate::pull::operand(vm, "PAIR", 1)?;
    let y = crate::pull::operand(vm, "PAIR", 2)?;
    vm.push(BundValue::pair(x, y));
    Ok(())
}

/// `?effect` — a word's declared stack effect, as a MAP. **RFC-0004 §S5.**
///
/// An **addition**: the reference registers no word containing `effect`, so
/// this cannot move `conform` — no golden reaches it — and it is not among the
/// 497 in-scope words, so it cannot move `coverage` either. Worth saying,
/// because a reader watching the health metric would otherwise expect one of
/// them to shift.
///
/// The MAP carries `consumes`, `produces` and `opaque`. The last is the one
/// that matters: for `!`, `apply`, `if` and the rest, the pair is a floor and
/// nothing more, and a caller that reads the numbers without reading `opaque`
/// would draw exactly the conclusion `bund2 check` refuses to draw.
///
/// A name with no declared effect — a lambda, an unbound name — answers
/// `nodata` rather than a zero pair, because "takes nothing" and "cannot say"
/// are different answers and D16 makes the second one common.
fn effect_of_word(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline ?effect".into()));
    }
    let v = crate::pull::operand(vm, "?EFFECT", 1)?;
    let Some(name) = v.as_str() else {
        return Err(Error("?EFFECT expecting a word name to be string".into()));
    };
    match vm.effect_of(&name) {
        Some(e) => {
            let m = BundValue::map(Default::default())
                .set("consumes", BundValue::int(i64::from(e.consumes)))
                .set("produces", BundValue::int(i64::from(e.produces)))
                .set("opaque", BundValue::boolean(e.opaque));
            vm.push(m);
        }
        None => vm.push(BundValue::nodata()),
    }
    Ok(())
}

/// `head`, `tail` and `at` — **F18's other list words**, and the last three of
/// its fourteen that this file adds
/// (`reference/rust_multistackvm/src/stdlib/values/value_carcdr.rs:63-150`).
///
/// Each guards `< 1` and then pulls **two**: the count, then the list (`:64`,
/// and the value pulled before the match). F18's FIX declares the arity they
/// consume, so one operand short reports `Stack is too shallow` and leaves the
/// operand in place.
///
/// The semantics, from `carcdr.rs:5-77`:
///
/// - `head n` — the first `n` (`:11-17`).
/// - `tail n` — the **last** `n`, still in order (`:37`, a `rev().take().rev()`).
/// - `at n` — **one-based**, and out of range is an error rather than nodata
///   (`:57-60`: `n > len || n < 0`, then `raw_list[n-1]`). Note `n == 0`
///   passes the guard and then indexes `[-1]`, which is why Bund2 refuses 0.
/// - A **non-LIST answers itself** for all three (`:26-28`), so `42 1 head` is
///   `42`. Preserved.
fn head_tail_at(vm: &mut dyn Vm, side: crate::wb::Side, which: u8, base: &str) -> Result<(), Error> {
    let prefix = &format!("{base}{}", side.dot());
    // The plain form is declared at the arity it *consumes*, per F18. The `.`
    // form guards **one** workbench cell and then pulls two from it
    // (`value_carcdr.rs:20-22,64-67`) — the reference's own shape, and the
    // one place the two forms genuinely differ.
    crate::wb::shallow(vm, side, if side == crate::wb::Side::Stack { 2 } else { 1 }, prefix)?;
    // **The list is on top and the count beneath it.** `stdlib_carcdr_base`
    // pulls the value before the match (`value_carcdr.rs:27-33`) and each arm
    // then pulls its count (`:64-67`), so the source reads `<n> <list> head`.
    // Written the other way round first, which the oracle rejected.
    let v = crate::wb::operand(vm, side, prefix)?;
    let n = count_arg_named(vm, side, prefix)?;
    if v.dt() != LIST {
        side.push(vm, v);
        return Ok(());
    }
    let items = v
        .as_list()
        .ok_or_else(|| Error(format!("{prefix} returned: NO DATA")))?
        .to_vec();
    let out = match which {
        0 => BundValue::list(items.iter().take(n.max(0) as usize).cloned().collect()),
        1 => {
            let k = (n.max(0) as usize).min(items.len());
            BundValue::list(items[items.len() - k..].to_vec())
        }
        _ => {
            if n < 1 || n as usize > items.len() {
                return Err(Error(format!("{prefix} returned: NO DATA")));
            }
            items
                .get((n - 1) as usize)
                .cloned()
                .ok_or_else(|| Error(format!("{prefix} returned: NO DATA")))?
        }
    };
    side.push(vm, out);
    Ok(())
}

/// The count operand, from **the same side the value came from**
/// (`value_carcdr.rs:64-67`).
fn count_arg_named(vm: &mut dyn Vm, side: crate::wb::Side, prefix: &str) -> Result<i64, Error> {
    let v = crate::wb::operand(vm, side, prefix)?;
    v.as_int().ok_or_else(|| {
        Error(format!(
            "{prefix} returned during index casting: This Dynamic type is not int"
        ))
    })
}

/// `complex` — a CINTEGER from two numbers
/// (`reference/rust_multistackvm/src/stdlib/artefacts.rs:25-50`).
///
/// F18's word with the **wrong guard message**: it says `pair()` (`:27`),
/// because the function was copied from `pair` and the string was not changed.
/// That is F40's shape as well as F18's, and both are reproduced — the message
/// is what a program catching it would see.
fn complex(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline pair()".into()));
    }
    let re = crate::pull::operand(vm, "COMPLEX", 1)?;
    let im = crate::pull::operand(vm, "COMPLEX", 2)?;
    let (Some(re), Some(im)) = (as_f64(&re), as_f64(&im)) else {
        return Err(Error(
            "COMPLEX cast error #1: This Dynamic type is not float".into(),
        ));
    };
    // **Tagged `CFLOAT`, not LIST.** The pair of floats is the payload; the tag
    // is what makes it a complex number, and it is what `println` refuses —
    // `1.0 2.0 complex println` reports `Can not convert Value from 15` in both
    // engines. Pushing a plain LIST printed `[ 2.0 :: 1.0 :: ]` instead.
    vm.push(
        BundValue::list(vec![BundValue::float(re), BundValue::float(im)])
            .with_dt_tag(bund2_value::CFLOAT),
    );
    Ok(())
}

fn as_f64(v: &BundValue) -> Option<f64> {
    match *v.unboxed() {
        BundValue::Float(f, _) => Some(f),
        _ => None,
    }
}

/// `string.regex` and `string.wildcard` — pattern tests, both **F18 words**
/// (`reference/Bund/src/stdlib/functions/string/regex.rs:11-80`,
/// `wildmatch.rs:11-60`).
///
/// **The subject is on top and the pattern beneath it** (`regex.rs:28,31`), so
/// the source reads `<pattern> <subject> string.regex`. That is the opposite
/// of `string.expressionmatch`, whose expression is on top
/// (`textexpr_match.rs:28-32`) — two neighbouring words with the same shape
/// and opposite orders, and writing this one from memory of the other put the
/// pattern in the subject's place.
///
/// `regex` uses `fancy_regex` and `wildcard` uses `wildmatch`, both the
/// reference's own crates, because what counts as a match is their definition
/// and not one this file should invent.
fn pattern_match(vm: &mut dyn Vm, wild: bool, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let subj_val = crate::pull::operand(vm, prefix, 1)?;
    let pat_val = crate::pull::operand(vm, prefix, 2)?;
    let (Some(pat), Some(subj)) = (pat_val.as_str(), subj_val.as_str()) else {
        return Err(Error(format!(
            "{prefix} returned for #1: This Dynamic type is not string"
        )));
    };
    let hit = if wild {
        wildmatch::WildMatch::new(&pat).matches(&subj)
    } else {
        match fancy_regex::Regex::new(&pat) {
            Ok(re) => re.is_match(&subj).unwrap_or(false),
            Err(e) => return Err(Error(format!("{prefix} returned error: {e}"))),
        }
    };
    vm.push(BundValue::boolean(hit));
    Ok(())
}

/// A distance that answers a FLOAT rather than an INTEGER
/// (`reference/Bund/src/stdlib/functions/string/distance.rs:85-86`).
fn distance_f(vm: &mut dyn Vm, f: fn(&str, &str) -> f64, prefix: &str) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let a = crate::pull::operand(vm, prefix, 1)?;
    let b = crate::pull::operand(vm, prefix, 2)?;
    let (Some(a), Some(b)) = (a.as_str(), b.as_str()) else {
        return Err(Error(format!(
            "{prefix} returned for #1: This Dynamic type is not string"
        )));
    };
    vm.push(BundValue::float(f(&a, &b)));
    Ok(())
}

/// `string.distance.hamming` — the one that can **fail**
/// (`distance.rs:75-83`).
///
/// `distance::hamming` returns a `Result` because the metric is undefined for
/// strings of unequal length, and the reference reports
/// `returned hamming error: {err:?}` rather than answering. That refusal is
/// the word's contract, not an implementation detail.
fn hamming(vm: &mut dyn Vm) -> Result<(), Error> {
    let prefix = "STRING.DISTANCE.HAMMING";
    if vm.depth() < 2 {
        return Err(Error(format!("Stack is too shallow for inline {prefix}")));
    }
    let a = crate::pull::operand(vm, prefix, 1)?;
    let b = crate::pull::operand(vm, prefix, 2)?;
    let (Some(a), Some(b)) = (a.as_str(), b.as_str()) else {
        return Err(Error(format!(
            "{prefix} returned for #1: This Dynamic type is not string"
        )));
    };
    match distance::hamming(&a, &b) {
        Ok(d) => {
            vm.push(BundValue::int(d as i64));
            Ok(())
        }
        Err(e) => Err(Error(format!("{prefix} returned hamming error: {e:?}"))),
    }
}

pub fn register(r: &mut Registry) {
    r.register_native("pair", pair, eff(2, 1), WordKind::Sync);
    // **F18's fourteen, declared at the probed arity.** Each guards `< 1` in
    // the reference and pulls two, so `1 head` there reports `NO DATA #2` on
    // an emptied stack; here the guard fires first and the operand survives.
    use crate::wb::Side;
    r.register_native("head", |vm| head_tail_at(vm, Side::Stack, 0, "HEAD"), eff(2, 1), WordKind::Sync);
    r.register_native("tail", |vm| head_tail_at(vm, Side::Stack, 1, "TAIL"), eff(2, 1), WordKind::Sync);
    r.register_native("at", |vm| head_tail_at(vm, Side::Stack, 2, "AT"), eff(2, 1), WordKind::Sync);
    // The `.` siblings take **both** operands off the workbench and leave the
    // answer there, so they consume nothing from the stack and produce nothing
    // on it — an effect the type cannot express, which is RFC-0004 §S1's
    // missing second axis showing through.
    r.register_native("head.", |vm| head_tail_at(vm, Side::Bench, 0, "HEAD"), eff(0, 0), WordKind::Sync);
    r.register_native("tail.", |vm| head_tail_at(vm, Side::Bench, 1, "TAIL"), eff(0, 0), WordKind::Sync);
    r.register_native("at.", |vm| head_tail_at(vm, Side::Bench, 2, "AT"), eff(0, 0), WordKind::Sync);
    r.register_native("car.", car_wb, eff(0, 0), WordKind::Sync);
    r.register_native("cdr.", cdr_wb, eff(0, 0), WordKind::Sync);
    r.register_native("complex", complex, eff(2, 1), WordKind::Sync);
    r.register_native(
        "string.regex",
        |vm| pattern_match(vm, false, "STRING.REGEX"),
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.wildcard",
        |vm| pattern_match(vm, true, "STRING.WILDCARD"),
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native("?effect", effect_of_word, eff(1, 1), WordKind::Sync);
    r.register_native("alias", alias, eff(2, 0), WordKind::Sync);
    r.register_native("unalias", unalias, eff(1, 0), WordKind::Sync);
    // Opaque: `apply` runs whatever it is handed, so what it leaves is
    // whatever that does. `tests/golden/EFFECTS.txt` records the same thing
    // for the probed column.
    r.register_native("apply", apply, StackEffect::opaque(1), WordKind::Sync);
    // **Opaque — F91.** A `fmt` CONDITIONAL is rendered by its runner, which
    // pulls a value per placeholder, and an OBJECT is dispatched as
    // `:display <obj> !`, which runs its `.display` method.
    r.register_native("display", display, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("len", len, eff(1, 2), WordKind::Sync);
    r.register_native("++", merge, eff(2, 1), WordKind::Sync);
    r.register_native("merge", merge, eff(2, 1), WordKind::Sync);
    // D24's contract: receiver off the workbench, operand off the stack,
    // answer back to the workbench.
    r.register_native("merge.", merge_wb, eff(1, 0), WordKind::Sync);
    r.register_alias("++.", "merge.");
    // `reference/Bund/src/stdlib/functions/values/push.rs:74-75` and the
    // aliases at `reference/Bund/src/stdlib/functions/create_aliases.rs:34-35`.
    // The `.` form takes its receiver off the workbench and answers there, so
    // it takes one value from the stack and leaves none.
    r.register_native("push", push_list, eff(2, 1), WordKind::Sync);
    r.register_native("push.", push_list_wb, eff(1, 0), WordKind::Sync);
    r.register_alias("+++", "push");
    r.register_alias("+++.", "push.");
    // `reference/Bund/src/stdlib/functions/values/unfold.rs:61-62`. Opaque:
    // it leaves as many values as the list holds. The `.` form spreads onto
    // the workbench and touches the stack not at all.
    r.register_native("unfold", unfold, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("unfold.", unfold_wb, eff(0, 0), WordKind::Sync);
    // `reference/rust_multistackvm/src/stdlib/values/value_tag.rs:72`.
    r.register_native("tag", tag_word, eff(3, 1), WordKind::Sync);
    r.register_native(
        "string.distance.levenshtein",
        |vm| distance_word(vm, distance::levenshtein, "STRING.DISTANCE.LEVENSHTEIN"),
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.distance",
        |vm| distance_word(vm, distance::levenshtein, "STRING.DISTANCE"),
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.distance.dameraulevenshtein",
        |vm| {
            distance_word(
                vm,
                distance::damerau_levenshtein,
                "STRING.DISTANCE.DAMERAULEVENSHTEIN",
            )
        },
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.expressionmatch",
        expression_match,
        eff(2, 1),
        WordKind::Sync,
    );
    // The three distance variants F18 also names. `sift3` and `jarowinkler`
    // answer a **FLOAT** where the edit distances answer an INTEGER
    // (`distance.rs:85-86`), and `hamming` is the one that *fails* on unequal
    // lengths rather than answering.
    r.register_native(
        "string.distance.sift3",
        |vm| distance_f(vm, |a, b| f64::from(distance::sift3(a, b)), "STRING.DISTANCE.SIFT3"),
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native(
        "string.distance.jarowinkler",
        |vm| {
            distance_f(
                vm,
                |a, b| f64::from(natural::distance::jaro_winkler_distance(a, b)),
                "STRING.DISTANCE.JAROWINKLER",
            )
        },
        eff(2, 1),
        WordKind::Sync,
    );
    r.register_native("string.distance.hamming", hamming, eff(2, 1), WordKind::Sync);
    // Opaque: the lambda runs first and its result becomes the condition, so
    // it may push from anywhere (`conditional_move.rs:16-45`).
    r.register_native("?.", q_move_workbench, StackEffect::opaque(2), WordKind::Sync);
    r.register_native("?move", q_move_stack, StackEffect::opaque(3), WordKind::Sync);
    r.register_native("attribute", attribute, eff(2, 1), WordKind::Sync);
    r.register_native("car", car, eff(1, 1), WordKind::Sync);
    r.register_native("cdr", cdr, eff(1, 1), WordKind::Sync);
    r.register_native("compile", compile, eff(1, 1), WordKind::Sync);
    r.register_native("get,", |vm| getset_inplace(vm, Side::Stack, false), eff(2, 2), WordKind::Sync);
    r.register_native("set,", |vm| getset_inplace(vm, Side::Stack, true), eff(3, 1), WordKind::Sync);
    // The `.` forms keep the dictionary on the workbench: `get.,` takes the
    // key off the stack and leaves the value there (1 -> 1); `set.,` takes the
    // value and the key and leaves nothing (2 -> 0).
    r.register_native("get.,", |vm| getset_inplace(vm, Side::Bench, false), eff(1, 1), WordKind::Sync);
    r.register_native("set.,", |vm| getset_inplace(vm, Side::Bench, true), eff(2, 0), WordKind::Sync);
    // Opaque, both forms: one stack value is pulled per name in the list, so
    // neither consumes a fixed number. `pull` said `1 -> 1`, true only of an
    // empty list (F92). `pull.` takes the names off the workbench, values off
    // the stack, and puts the MAP back on the workbench.
    r.register_native("pull", |vm| pull_word(vm, Side::Stack), StackEffect::opaque(1), WordKind::Sync);
    r.register_native("pull.", |vm| pull_word(vm, Side::Bench), StackEffect::opaque(0), WordKind::Sync);
    r.register_native("var", var, eff(2, 0), WordKind::Sync);
    r.register_native("var?", var_read, eff(1, 1), WordKind::Sync);
    r.register_native("var-", var_unregister, eff(1, 0), WordKind::Sync);
}

#[cfg(test)]
mod f18_tests {
    use super::*;
    use bund2_interp::Interp;

    fn run(src: &str) -> Result<Interp, String> {
        let mut i = Interp::new();
        crate::register_all(&mut i.registry);
        let stream = bund2_syntax::compile(src).map_err(|e| e.render(src))?;
        i.eval(&stream).map_err(|e| e.0)?;
        Ok(i)
    }

    /// **RFC-0004 criterion 3**, for the words of F18's fourteen that Bund2
    /// implements.
    ///
    /// F18's disposition: declare the arity the word *consumes*, so the guard
    /// fires before the first pull. The reference guards `< 1` and then pulls
    /// two, so one operand short it reports `NO DATA #2` on a stack it has
    /// already emptied. Here it must report `Stack is too shallow` **and leave
    /// the operand where it was** — the residual stack is the half F18's
    /// disposition calls out as observable, because the error path prints it.
    ///
    /// The list is F18's own, not a recomputation: a criterion keyed to a
    /// regenerated table is keyed to a moving number.
    #[test]
    fn f18_words_guard_before_pulling() {
        let cases: &[(&str, &str)] = &[
            ("1 pair", "pair()"),
            ("[ 1 ] head", "HEAD()"),
            ("[ 1 ] tail", "TAIL()"),
            ("[ 1 ] at", "AT()"),
            ("1.0 complex", "pair()"),
            ("\"a\" string.distance", "STRING.DISTANCE"),
            ("\"a\" string.distance.levenshtein", "STRING.DISTANCE.LEVENSHTEIN"),
            ("\"a\" string.distance.dameraulevenshtein", "STRING.DISTANCE.DAMERAULEVENSHTEIN"),
            ("\"a\" string.distance.hamming", "STRING.DISTANCE.HAMMING"),
            ("\"a\" string.distance.jarowinkler", "STRING.DISTANCE.JAROWINKLER"),
            ("\"a\" string.distance.sift3", "STRING.DISTANCE.SIFT3"),
            ("\"a\" string.regex", "STRING.REGEX"),
            ("\"a\" string.wildcard", "STRING.WILDCARD"),
        ];
        for (src, want) in cases {
            match run(src) {
                Ok(_) => panic!("{src} was expected to fail"),
                Err(e) => {
                    assert!(
                        e.contains("too shallow"),
                        "{src}: guard did not fire first: {e}"
                    );
                    assert!(e.contains(want), "{src}: wrong word named: {e}");
                }
            }
        }
    }

    /// The other half, and the one a message alone would miss: the operand is
    /// still there. `pair` is F18's worked example — the reference leaves an
    /// empty stack here.
    #[test]
    fn f18_words_leave_the_operand() {
        for src in ["1 pair", "[ 1 ] head", "\"a\" string.regex"] {
            let mut i = Interp::new();
            crate::register_all(&mut i.registry);
            let stream = bund2_syntax::compile(src).expect("compiles");
            assert!(i.eval(&stream).is_err(), "{src} should fail");
            assert_eq!(i.depth(), 1, "{src}: the operand was consumed");
        }
    }
}
