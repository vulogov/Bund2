//! `json`, `json.to_value`, `json.from_value`.
//!
//! `json` parses a string with `serde_json` and pushes a value tagged `JSON`
//! (`reference/rust_multistackvm/src/stdlib/artefacts_json.rs:7-32`). The
//! payload is a `serde_json::Value` held whole — the reference does not
//! translate it until asked, and neither does this.
//!
//! `json.to_value` is the translation
//! (`reference/rust_dynamic/src/cast_json_to_value.rs:5-90`), and it is the
//! one place in the language where a value is born with `q` at **0.0** rather
//! than 100.0: a JSON `null` becomes `Value::none()`, which is `Value::new()`
//! (`reference/rust_dynamic/src/create_special.rs:19-21`) and starts at zero.
//! `tests/probes/q-observable.bund` exists to pin exactly that.

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::{BOOL, BundValue, FLOAT, INTEGER, JSON, LIST, MAP};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// `json.path` — query a JSON value with a JSONPath expression
/// (`reference/rust_multistackvm/src/stdlib/json/json_path.rs:9-64`), through
/// the reference's own `jsonpath-rust` at the version its lock resolves.
///
/// The JSON value is on top and the path beneath it (`:13,18`). Every match is
/// collected into one JSON array, which is pushed (`:39`).
///
/// **It prints to stdout.** The reference `println!`s the `Debug` form of every
/// matched slice as it collects them (`:37`), and a golden captures that text.
/// So this prints it too, with the same crate's `Debug` (F99).
fn json_path(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 2 {
        return Err(Error("Stack is too shallow for inline json.path".into()));
    }
    let j = vm
        .pull()
        .ok_or_else(|| Error("JSON.PATH returns: NO DATA #1".into()))?;
    if j.dt() != JSON {
        return Err(Error("JSON.PATH: #1 parameter must be JSON".into()));
    }
    let value = j
        .as_json()
        .ok_or_else(|| Error::internal("a JSON value carried no JSON"))?;
    let p = vm
        .pull()
        .ok_or_else(|| Error("JSON.PATH returns: NO DATA #2".into()))?;
    let Some(p) = p.as_str() else {
        return Err(Error(
            "JSON.PATH casting search path returns: This Dynamic type is not string".into(),
        ));
    };
    let path = jsonpath_rust::JsonPath::<serde_json::Value>::try_from(p.as_str())
        .map_err(|e| Error(format!("JSON.PATH compilation of search path returns: {e}")))?;
    let mut found: Vec<serde_json::Value> = Vec::new();
    for s in path.find_slice(&value) {
        match &s {
            jsonpath_rust::JsonPathValue::Slice(v, _) => found.push((*v).clone()),
            _ => continue,
        }
        bund2_api::outln!("{:?}", &s)?;
    }
    vm.push(BundValue::json(serde_json::Value::Array(found)));
    Ok(())
}

/// `json` — parse a string into a JSON value.
fn json(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline json()".into()));
    }
    let v = crate::pull::operand(vm, "JSON", 1)?;
    // `cast_string` asks the payload, not the tag
    // (`reference/rust_dynamic/src/cast.rs:33-40`), so a PTR is taken as its
    // name and fails later, as text that is not JSON.
    let text = v
        .as_str()
        .ok_or_else(|| Error("JSON returns error: This Dynamic type is not string".into()))?;
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(j) => {
            vm.push(BundValue::json(j));
            Ok(())
        }
        Err(e) => Err(Error(format!("JSON convert returns: {e}"))),
    }
}

/// `cast_json_to_value` (`cast_json_to_value.rs:9-90`), arm for arm.
///
/// The order matters: `is_i64` is tried before `is_f64` (`:20,29`), so a JSON
/// `1` is an INTEGER and not a FLOAT. Arrays and objects recurse (`:49,70`).
///
/// **`null` becomes `Value::none()` (`:39`), whose `q` is 0.0.** Every other
/// constructor starts at 100.0, so this is the single observable case for
/// `q`, and it is why `BundValue::with_q` exists at all.
pub(crate) fn to_value(j: &serde_json::Value) -> Result<BundValue, Error> {
    use serde_json::Value as J;
    match j {
        J::String(s) => Ok(BundValue::str(s.clone())),
        // **`is_f64`, not `as_f64`.** An integer above `i64::MAX` is held as
        // a `u64`: `is_i64` is false, `is_f64` is false, and the reference
        // falls past every arm to its last (`:96`). `as_f64` would have
        // widened it, and did -- `'18446744073709551615' json json.to_value`
        // answered a FLOAT where the oracle refuses (F169).
        J::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => Ok(BundValue::int(i)),
            (None, Some(f)) if n.is_f64() => Ok(BundValue::float(f)),
            _ => Err(Error(
                "This JSON is having a data that is not supportable".to_string(),
            )),
        },
        J::Null => Ok(BundValue::none().with_q(0.0)),
        J::Bool(b) => Ok(BundValue::boolean(*b)),
        J::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for v in items {
                out.push(to_value(v).map_err(|e| {
                    Error(format!("JSON returns error during casting: {}", e.0))
                })?);
            }
            Ok(BundValue::list(out))
        }
        J::Object(map) => {
            let mut out = BundValue::map(Default::default());
            for (k, v) in map {
                let inner = to_value(v).map_err(|e| {
                    Error(format!("JSON returns error during casting: {}", e.0))
                })?;
                out = out.set(k, inner);
            }
            Ok(out)
        }
    }
}

/// `json.to_value` (`reference/rust_multistackvm/src/stdlib/json/conversion.rs:31-53`).
fn json_to_value(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error(
            "Stack is too shallow for inline json.to_value".into(),
        ));
    }
    let v = crate::pull::operand(vm, "JSON.TO_VALUE", 1)?;
    if v.dt() != JSON {
        return Err(Error("JSON.TO_VALUE: #1 parameter must be JSON".into()));
    }
    let Some(j) = v.as_json() else {
        return Err(Error::internal("a value tagged JSON carried no JSON"));
    };
    let out = to_value(&j).map_err(|e| Error(format!("Error casting from JSON value: {}", e.0)))?;
    vm.push(out);
    Ok(())
}

/// **`cast_value_to_json`'s accepted set, which is not every value.**
///
/// INTEGER, FLOAT, BOOL, NONE, LIST and MAP, the containers recursively —
/// and **a STRING is refused** (`reference/rust_dynamic/src/cast_value_to_json.rs`),
/// which is the thing no reading of `json.from_value` would predict. Measured:
/// `"x" json.from_value` answers `This Dynamic type is not supported for
/// JSON: 4`.
///
/// Deliberately not `BundValue::as_json_operand`, which is `push`'s
/// conversion and *does* take a string. The two were briefly merged on a "one
/// function, one answer" argument; this word is the counterexample — they are
/// two questions with two answers, and the merge made `"x" json.from_value`
/// succeed where the oracle fails.
fn value_to_json(v: &BundValue) -> Result<serde_json::Value, Error> {
    let fail = || {
        Error(format!(
            "This Dynamic type is not supported for JSON: {}",
            v.dt()
        ))
    };
    match v.dt() {
        INTEGER => v
            .as_int()
            .map(serde_json::Value::from)
            .ok_or_else(|| Error("Error casting INTEGER: This Dynamic type is not integer".into())),
        FLOAT => match v.unboxed() {
            BundValue::Float(f, _) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .ok_or_else(|| Error("Error casting FLOAT: not representable in JSON".into())),
            _ => Err(Error("Error casting FLOAT: This Dynamic type is not float".into())),
        },
        BOOL => match v.unboxed() {
            BundValue::Bool(b, _) => Ok(serde_json::Value::Bool(*b)),
            _ => Err(Error("Error casting BOOL: This Dynamic type is not bool".into())),
        },
        bund2_value::NONE => Ok(serde_json::Value::Null),
        LIST => {
            let mut out = Vec::new();
            for item in v.as_list().unwrap_or(&[]) {
                out.push(
                    value_to_json(item)
                        .map_err(|e| Error(format!("Error casting LIST: {}", e.0)))?,
                );
            }
            Ok(serde_json::Value::Array(out))
        }
        MAP => {
            let mut out = serde_json::Map::new();
            for (k, item) in v.as_map().map(|m| m.iter().collect::<Vec<_>>()).unwrap_or_default() {
                out.insert(
                    k.clone(),
                    value_to_json(item)
                        .map_err(|e| Error(format!("Error casting DICT: {}", e.0)))?,
                );
            }
            Ok(serde_json::Value::Object(out))
        }
        _ => Err(fail()),
    }
}

/// `json.from_value` — the inverse of `json.to_value`.
///
/// **A JSON on the stack is refused rather than passed through.** The
/// reference checks `is_type(JSON)` first and bails with `Stack already having
/// a JSON value`, so wrapping twice is an error and not a no-op
/// (`reference/rust_multistackvm/src/stdlib/json/conversion.rs`).
fn json_from_value(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error(
            "Stack is too shallow for inline json.from_value".into(),
        ));
    }
    let v = crate::pull::operand(vm, "JSON.FROM_VALUE", 1)?;
    if v.dt() == JSON {
        return Err(Error("Stack already having a JSON value".into()));
    }
    let j = value_to_json(&v)
        .map_err(|e| Error(format!("Error casting JSON value: {}", e.0)))?;
    vm.push(BundValue::json(j));
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.register_native("json.from_value", json_from_value, eff(1, 1), WordKind::Sync);
    crate::wb::bench!(r, "json.from_value", json_from_value);
    r.register_native("json", json, eff(1, 1), WordKind::Sync);
    crate::wb::bench!(r, "json", json);
    r.register_native("json.to_value", json_to_value, eff(1, 1), WordKind::Sync);
    crate::wb::bench!(r, "json.to_value", json_to_value);
    // `reference/rust_multistackvm/src/stdlib/json/json_path.rs`, `init_stdlib`.
    r.register_native("json.path", json_path, eff(2, 1), WordKind::Sync);
    crate::wb::bench!(r, "json.path", json_path, 2, 0);
}

#[cfg(test)]
mod tests {
    use bund2_api::Vm as _;
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

    #[test]
    fn json_path_wants_json_on_top() {
        let e = err_of("'$.a' 1 json.path");
        assert!(e.contains("JSON.PATH: #1 parameter must be JSON"), "{e}");
    }

    #[test]
    fn a_path_that_does_not_compile_is_reported() {
        let e = err_of("'$[' '{}' json json.path");
        assert!(e.contains("JSON.PATH compilation of search path returns"), "{e}");
    }

    /// JSON prints as compact `serde_json` text (`conv.rs:677`).
    #[test]
    fn json_prints_compact() {
        let i = run_src("'$.a[*]' '{\"a\": [1, 2]}' json json.path").expect("runs");
        assert_eq!(i.peek().map(|v| v.display()).as_deref(), Some("[1,2]"));
    }
}
