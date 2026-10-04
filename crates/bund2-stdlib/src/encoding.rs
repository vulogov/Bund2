//! **`encode.base64`, `decode.base64` and `pull.workbench`.**
//!
//! **base64 carries a whole value, not a string.** `encode.base64` serialises
//! its operand through the wire format and base64s *that*, so `42
//! encode.base64 decode.base64` gives the integer back rather than the text
//! `"42"`. The reference's own `to_binary`/`from_binary` pair, which is why
//! F109's decode rule reaches here too: the value comes back with its stamp
//! and a fresh identity (D88).

use bund2_api::{Error, Registry, StackEffect, Vm, WordKind};
use bund2_value::BundValue;

use crate::wb::Side;

fn eff(consumes: u8, produces: u8) -> StackEffect {
    StackEffect::fixed(consumes, produces)
}

/// Guard, then take one operand from whichever side.
fn one(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<BundValue, Error> {
    if side.depth(vm) < 1 {
        return Err(match side {
            Side::Stack => Error(format!("Stack is too shallow for inline {prefix}")),
            Side::Bench => Error(format!("Workbench is too shallow for inline {prefix}")),
        });
    }
    side.pull(vm)
        .ok_or_else(|| Error(format!("{prefix} returns NO DATA #1")))
}

fn encode_base64(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    use base64ct::Encoding as _;
    let v = one(vm, side, prefix)?;
    let data = bund2_value::wire::to_binary(&v)
        .map_err(|e| Error(format!("{prefix} wrapping object returned: {e}")))?;
    let encoded = base64ct::Base64::encode_string(&data);
    side.push(vm, BundValue::str(encoded));
    Ok(())
}

fn decode_base64(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    use base64ct::Encoding as _;
    let v = one(vm, side, prefix)?;
    let text = v.as_str().ok_or_else(|| {
        Error(format!(
            "{prefix} casting object returned: This Dynamic type is not string"
        ))
    })?;
    // **The reference's own codec, and its own error text.** The message
    // interpolates the decoder's `Debug`, so the crate that decodes *is* the
    // text — F147's lesson, applied before it could bite again.
    let bytes = base64ct::Base64::decode_vec(&text)
        .map_err(|e| Error(format!("{prefix} decoding object returned: {e}")))?;
    let value = bund2_value::wire::from_binary(&bytes)
        .map_err(|e| Error(format!("{prefix} unwrapping object returned: {e}")))?;
    side.push(vm, value);
    Ok(())
}

// **`unique` and `unique.` are not here, and that is a decision rather than an
// omission.** The reference deduplicates by asking
// `algos::cs::search::fibonacci::search` whether each element is already in
// the accumulator, and that function requires `Ord`. `BundValue` has no `Ord`
// impl: F12 records that the reference's own ordering fallback is inconsistent
// with its `PartialOrd` and unreachable, and D1 makes non-scalar comparison
// identity-based. Giving `BundValue` an `Ord` to satisfy a crate bound would
// settle that by accident. F149 records the word's measured behaviour — it
// errors on any list that is not ascending — and what implementing it needs.

/// `pull.workbench` — a dict built by naming values off the workbench.
///
/// **The names come from the given side; the values always come from the
/// workbench.** So `pull.workbench` takes its name list off the stack and
/// still drains the workbench, and `pull.workbench.` takes the list off the
/// workbench and then drains what is left of it.
fn pull_workbench(vm: &mut dyn Vm, side: Side, prefix: &str) -> Result<(), Error> {
    let v = one(vm, side, prefix)?;
    let names = v.as_list().map(<[BundValue]>::to_vec).ok_or_else(|| {
        Error(format!(
            "{prefix} casting of list returned: This Dynamic type is not list"
        ))
    })?;
    let mut dict = BundValue::map(Default::default());
    for n in names {
        let name = n.as_str().ok_or_else(|| {
            Error(format!(
                "{prefix} error casting name from string: This Dynamic type is not string"
            ))
        })?;
        let value = vm
            .pull_workbench()
            .ok_or_else(|| Error(format!("{prefix} can not pull value from stack")))?;
        dict = dict.set(&name, value);
    }
    side.push(vm, dict);
    Ok(())
}

pub fn register(r: &mut Registry) {
    fn e_s(vm: &mut dyn Vm) -> Result<(), Error> {
        encode_base64(vm, Side::Stack, "ENCODE.BASE64")
    }
    fn e_w(vm: &mut dyn Vm) -> Result<(), Error> {
        encode_base64(vm, Side::Bench, "ENCODE.BASE64.")
    }
    fn d_s(vm: &mut dyn Vm) -> Result<(), Error> {
        decode_base64(vm, Side::Stack, "DECODE.BASE64")
    }
    fn d_w(vm: &mut dyn Vm) -> Result<(), Error> {
        decode_base64(vm, Side::Bench, "DECODE.BASE64.")
    }
    fn p_s(vm: &mut dyn Vm) -> Result<(), Error> {
        pull_workbench(vm, Side::Stack, "PULL.WORKBENCH")
    }
    fn p_w(vm: &mut dyn Vm) -> Result<(), Error> {
        pull_workbench(vm, Side::Bench, "PULL.WORKBENCH.")
    }

    r.register_native("encode.base64", e_s, eff(1, 1), WordKind::Sync);
    r.register_native("decode.base64", d_s, eff(1, 1), WordKind::Sync);
    // The `.` forms take their operand off the workbench and leave their
    // answer there, so the stack is untouched.
    r.register_native("encode.base64.", e_w, eff(0, 0), WordKind::Sync);
    r.register_native("decode.base64.", d_w, eff(0, 0), WordKind::Sync);
    // **Opaque, both forms.** One workbench value is drained per name in the
    // list, so what either consumes is not a constant — the same reason
    // `pull` is opaque (F92).
    r.register_native("pull.workbench", p_s, StackEffect::opaque(1), WordKind::Sync);
    r.register_native("pull.workbench.", p_w, StackEffect::opaque(0), WordKind::Sync);
}
