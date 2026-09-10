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
use bund2_value::{BundValue, JSON, STRING};

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// `json` — parse a string into a JSON value.
fn json(vm: &mut dyn Vm) -> Result<(), Error> {
    if vm.depth() < 1 {
        return Err(Error("Stack is too shallow for inline json()".into()));
    }
    let v = crate::pull::operand(vm, "JSON", 1)?;
    if v.dt() != STRING {
        return Err(Error(format!(
            "JSON returns error: This Dynamic type is not string: {}",
            v.dt()
        )));
    }
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
fn to_value(j: &serde_json::Value) -> Result<BundValue, Error> {
    use serde_json::Value as J;
    match j {
        J::String(s) => Ok(BundValue::str(s.clone())),
        J::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => Ok(BundValue::int(i)),
            (None, Some(f)) => Ok(BundValue::float(f)),
            _ => Err(Error(
                "This JSON is having a data that is not INT".to_string(),
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

pub fn register(r: &mut Registry) {
    r.register_native("json", json, eff(1, 1), WordKind::Sync);
    r.register_native("json.to_value", json_to_value, eff(1, 1), WordKind::Sync);
}
